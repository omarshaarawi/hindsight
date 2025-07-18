use rusqlite::{Connection, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

pub struct HistoryRecord {
    pub command: String,
    #[allow(dead_code)]
    pub timestamp: i64,
    #[allow(dead_code)]
    pub duration: i64,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::db_path()?;
        let conn = Connection::open(db_path)?;
        
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA mmap_size = 268435456;
             PRAGMA temp_store = MEMORY;",
        )?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id         INTEGER PRIMARY KEY,
                command    TEXT NOT NULL,
                exit_code  INTEGER,
                cwd        TEXT,
                hostname   TEXT,
                session    TEXT,
                start_ts   INTEGER,
                duration   INTEGER
            )",
            [],
        )?;
        
        conn.execute("CREATE INDEX IF NOT EXISTS idx_history_session ON history(session)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_history_cwd ON history(cwd)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_history_start_ts ON history(start_ts DESC)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_history_command ON history(command)", [])?;
        
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS history_fts USING fts5(
                command, 
                content='history', 
                content_rowid='rowid'
            )",
            [],
        )?;
        
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS history_fts_insert AFTER INSERT ON history BEGIN
                INSERT INTO history_fts(rowid, command) VALUES (new.rowid, new.command);
            END",
            [],
        )?;
        
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS history_fts_delete AFTER DELETE ON history BEGIN
                DELETE FROM history_fts WHERE rowid = old.rowid;
            END",
            [],
        )?;
        
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS history_fts_update AFTER UPDATE ON history BEGIN
                UPDATE history_fts SET command = new.command WHERE rowid = new.rowid;
            END",
            [],
        )?;
        
        Ok(Self { conn })
    }

    pub fn db_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "shaarawi", "hindsight")
            .ok_or_else(|| rusqlite::Error::InvalidPath("Could not find data directory".into()))?;
        let data_dir = proj_dirs.data_dir();
        std::fs::create_dir_all(data_dir)
            .map_err(|_| rusqlite::Error::InvalidPath("Could not create data directory".into()))?;
        Ok(data_dir.join("history.sqlite3"))
    }

    pub fn search(&self, mode: &str, limit: u32, current_session: &str, current_cwd: &str) -> Result<Vec<HistoryRecord>> {
        let (query, params): (&str, Vec<&dyn rusqlite::ToSql>) = match mode {
            "session" => (
                "SELECT DISTINCT command, MAX(start_ts) as start_ts, MAX(duration) as duration 
                 FROM history WHERE session = ?1 
                 GROUP BY command ORDER BY start_ts DESC LIMIT ?2",
                vec![&current_session, &limit]
            ),
            "cwd" => (
                "SELECT DISTINCT command, MAX(start_ts) as start_ts, MAX(duration) as duration 
                 FROM history WHERE cwd = ?1 
                 GROUP BY command ORDER BY start_ts DESC LIMIT ?2",
                vec![&current_cwd, &limit]
            ),
            _ => (
                "SELECT DISTINCT command, MAX(start_ts) as start_ts, MAX(duration) as duration 
                 FROM history 
                 GROUP BY command ORDER BY start_ts DESC LIMIT ?1",
                vec![&limit]
            ),
        };

        let mut stmt = self.conn.prepare_cached(query)?;
        let records = stmt.query_map(params.as_slice(), |row| {
            Ok(HistoryRecord {
                command: row.get(0)?,
                timestamp: row.get(1)?,
                duration: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
        
        Ok(records)
    }
}