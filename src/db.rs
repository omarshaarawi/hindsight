use rusqlite::{Connection, Result};
use directories::ProjectDirs;
use std::path::PathBuf;
use chrono;

pub struct Database {
    _conn: Connection,
}

pub struct HistoryRecord {
    pub command: String,
    #[allow(dead_code)]
    pub timestamp: i64,
    #[allow(dead_code)]
    pub duration: i64,
}

pub struct SavedCommand {
    pub id: i64,
    pub command: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub tags: Vec<String>,
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

        // Create saved commands tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS saved_commands (
                id          INTEGER PRIMARY KEY,
                command     TEXT NOT NULL UNIQUE,
                description TEXT,
                created_at  INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tags (
                id   INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS command_tags (
                command_id INTEGER NOT NULL,
                tag_id     INTEGER NOT NULL,
                PRIMARY KEY (command_id, tag_id),
                FOREIGN KEY (command_id) REFERENCES saved_commands(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_command_tags_tag ON command_tags(tag_id)", [])?;

        Ok(Self { _conn: conn })
    }

    pub fn db_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "shaarawi", "hindsight")
            .ok_or_else(|| rusqlite::Error::InvalidPath("Could not find data directory".into()))?;
        let data_dir = proj_dirs.data_dir();
        std::fs::create_dir_all(data_dir)
            .map_err(|_| rusqlite::Error::InvalidPath("Could not create data directory".into()))?;
        Ok(data_dir.join("history.sqlite3"))
    }

    pub fn save_command(&self, command: &str, description: Option<&str>, tags: Vec<String>) -> Result<i64> {
        let created_at = chrono::Utc::now().timestamp();

        self._conn.execute(
            "INSERT INTO saved_commands (command, description, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(command) DO UPDATE SET description = ?2, created_at = ?3",
            rusqlite::params![command, description, created_at],
        )?;

        let command_id = self._conn.last_insert_rowid();

        // Clear existing tags
        self._conn.execute(
            "DELETE FROM command_tags WHERE command_id = ?1",
            rusqlite::params![command_id],
        )?;

        // Insert new tags
        for tag in tags {
            self._conn.execute(
                "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
                rusqlite::params![&tag],
            )?;

            let tag_id: i64 = self._conn.query_row(
                "SELECT id FROM tags WHERE name = ?1",
                rusqlite::params![&tag],
                |row| row.get(0),
            )?;

            self._conn.execute(
                "INSERT INTO command_tags (command_id, tag_id) VALUES (?1, ?2)",
                rusqlite::params![command_id, tag_id],
            )?;
        }

        Ok(command_id)
    }

    pub fn delete_saved_command(&self, id: i64) -> Result<()> {
        self._conn.execute(
            "DELETE FROM saved_commands WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    pub fn get_saved_commands(&self, tag_filter: Option<Vec<String>>) -> Result<Vec<SavedCommand>> {
        let mut commands = Vec::new();

        let (query, has_filter) = if let Some(ref tags) = tag_filter {
            if tags.is_empty() {
                ("SELECT id, command, description, created_at FROM saved_commands ORDER BY created_at DESC".to_string(), false)
            } else {
                (format!(
                    "SELECT DISTINCT sc.id, sc.command, sc.description, sc.created_at
                     FROM saved_commands sc
                     JOIN command_tags ct ON sc.id = ct.command_id
                     JOIN tags t ON ct.tag_id = t.id
                     WHERE t.name IN ({})
                     ORDER BY sc.created_at DESC",
                    tags.iter().map(|_| "?").collect::<Vec<_>>().join(",")
                ), true)
            }
        } else {
            ("SELECT id, command, description, created_at FROM saved_commands ORDER BY created_at DESC".to_string(), false)
        };

        let mut stmt = self._conn.prepare(&query)?;

        if has_filter {
            if let Some(tags) = tag_filter {
                let params: Vec<&dyn rusqlite::ToSql> = tags.iter().map(|t| t as &dyn rusqlite::ToSql).collect();
                let rows = stmt.query_map(params.as_slice(), |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })?;

                for row in rows {
                    let (id, command, description, created_at): (i64, String, Option<String>, i64) = row?;

                    let tags: Vec<String> = self._conn
                        .prepare("SELECT t.name FROM tags t JOIN command_tags ct ON t.id = ct.tag_id WHERE ct.command_id = ?1")?
                        .query_map([id], |row| row.get(0))?
                        .collect::<Result<Vec<String>>>()?;

                    commands.push(SavedCommand {
                        id,
                        command,
                        description,
                        created_at,
                        tags,
                    });
                }
            }
        } else {
            let rows = stmt.query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?;

            for row in rows {
                let (id, command, description, created_at): (i64, String, Option<String>, i64) = row?;

                let tags: Vec<String> = self._conn
                    .prepare("SELECT t.name FROM tags t JOIN command_tags ct ON t.id = ct.tag_id WHERE ct.command_id = ?1")?
                    .query_map([id], |row| row.get(0))?
                    .collect::<Result<Vec<String>>>()?;

                commands.push(SavedCommand {
                    id,
                    command,
                    description,
                    created_at,
                    tags,
                });
            }
        }

        Ok(commands)
    }
}