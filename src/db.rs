use rusqlite::{params, Connection, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

pub struct HistoryRecord {
    pub command: String,
    pub timestamp: i64,
    pub duration: i64,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::db_path()?;
        if !db_path.exists() {
            return Err(rusqlite::Error::InvalidPath("History database not found".into()));
        }
        let conn = Connection::open(db_path)?;
        Ok(Self { conn })
    }

    fn db_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "shaarawi", "zhistory")
            .ok_or_else(|| rusqlite::Error::InvalidPath("Could not find data directory".into()))?;
        let data_dir = proj_dirs.data_dir();
        Ok(data_dir.join("history.sqlite3"))
    }

    pub fn search(&self, mode: &str, limit: u32, current_session: &str, current_cwd: &str) -> Result<Vec<HistoryRecord>> {
        let query = match mode {
            "session" => "SELECT command, start_ts, duration FROM history WHERE session = ?1 ORDER BY start_ts DESC LIMIT ?2",
            "cwd" => "SELECT command, start_ts, duration FROM history WHERE cwd = ?1 ORDER BY start_ts DESC LIMIT ?2",
            _ => "SELECT command, start_ts, duration FROM history ORDER BY start_ts DESC LIMIT ?1",
        };

        let mut stmt = self.conn.prepare(query)?;
        let mut records = Vec::new();
        
        match mode {
            "session" => {
                let mut rows = stmt.query(params![current_session, limit])?;
                while let Some(row) = rows.next()? {
                    records.push(HistoryRecord {
                        command: row.get(0)?,
                        timestamp: row.get(1)?,
                        duration: row.get(2)?,
                    });
                }
            }
            "cwd" => {
                let mut rows = stmt.query(params![current_cwd, limit])?;
                while let Some(row) = rows.next()? {
                    records.push(HistoryRecord {
                        command: row.get(0)?,
                        timestamp: row.get(1)?,
                        duration: row.get(2)?,
                    });
                }
            }
            _ => {
                let mut rows = stmt.query(params![limit])?;
                while let Some(row) = rows.next()? {
                    records.push(HistoryRecord {
                        command: row.get(0)?,
                        timestamp: row.get(1)?,
                        duration: row.get(2)?,
                    });
                }
            }
        }
        
        Ok(records)
    }
}