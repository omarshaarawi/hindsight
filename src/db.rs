use rusqlite::{Connection, Result};
use directories::ProjectDirs;
use std::path::PathBuf;
use std::io::{BufRead, BufReader};
use std::fs::File;
use chrono;

pub struct Database {
    _conn: Connection,
}

pub struct HistoryRecord {
    pub command: String,
    pub timestamp: i64,
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
        Self::with_connection(Connection::open(db_path)?)
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        Self::with_connection(Connection::open_in_memory()?)
    }

    fn with_connection(conn: Connection) -> Result<Self> {
        
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

    pub fn import_zsh_history(&self, path: &PathBuf) -> Result<ImportStats> {
        let file = File::open(path)
            .map_err(|e| rusqlite::Error::InvalidPath(e.to_string().into()))?;
        let reader = BufReader::new(file);

        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let import_session = format!("import-{}", chrono::Utc::now().timestamp());

        let mut imported = 0u64;
        let mut skipped = 0u64;
        let mut current_cmd = String::new();
        let mut current_ts: Option<i64> = None;

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            if line.is_empty() {
                continue;
            }

            if Self::is_extended_format(&line) {
                if !current_cmd.is_empty() {
                    match self.insert_history_record(&current_cmd.trim(), current_ts, &hostname, &import_session) {
                        Ok(true) => imported += 1,
                        Ok(false) => skipped += 1,
                        Err(_) => skipped += 1,
                    }
                    current_cmd.clear();
                }

                let parts: Vec<&str> = line.splitn(2, ';').collect();
                if parts.len() == 2 {
                    let meta = parts[0];
                    let cmd = parts[1];

                    current_ts = meta
                        .strip_prefix(": ")
                        .and_then(|s| s.split(':').next())
                        .and_then(|s| s.trim().parse::<i64>().ok());

                    if Self::is_line_continuation(cmd) {
                        current_cmd = cmd.to_string();
                        current_cmd.push('\n');
                    } else {
                        match self.insert_history_record(cmd.trim(), current_ts, &hostname, &import_session) {
                            Ok(true) => imported += 1,
                            Ok(false) => skipped += 1,
                            Err(_) => skipped += 1,
                        }
                        current_ts = None;
                    }
                }
            } else if !current_cmd.is_empty() {
                current_cmd.push_str(&line);
                if Self::is_line_continuation(&line) {
                    current_cmd.push('\n');
                } else {
                    match self.insert_history_record(&current_cmd.trim(), current_ts, &hostname, &import_session) {
                        Ok(true) => imported += 1,
                        Ok(false) => skipped += 1,
                        Err(_) => skipped += 1,
                    }
                    current_cmd.clear();
                    current_ts = None;
                }
            } else {
                match self.insert_history_record(&line, None, &hostname, &import_session) {
                    Ok(true) => imported += 1,
                    Ok(false) => skipped += 1,
                    Err(_) => skipped += 1,
                }
            }
        }

        if !current_cmd.is_empty() {
            match self.insert_history_record(&current_cmd.trim(), current_ts, &hostname, &import_session) {
                Ok(true) => imported += 1,
                Ok(false) => skipped += 1,
                Err(_) => skipped += 1,
            }
        }

        Ok(ImportStats { imported, skipped })
    }

    fn is_extended_format(line: &str) -> bool {
        if !line.starts_with(": ") {
            return false;
        }
        let Some(rest) = line.strip_prefix(": ") else {
            return false;
        };
        let Some(semicolon_pos) = rest.find(';') else {
            return false;
        };
        let meta = &rest[..semicolon_pos];
        let parts: Vec<&str> = meta.split(':').collect();
        if parts.len() != 2 {
            return false;
        }
        parts[0].trim().parse::<i64>().is_ok() && parts[1].trim().parse::<i64>().is_ok()
    }

    fn is_line_continuation(line: &str) -> bool {
        let trailing_backslashes = line.chars().rev().take_while(|&c| c == '\\').count();
        trailing_backslashes % 2 == 1
    }

    fn insert_history_record(&self, command: &str, timestamp: Option<i64>, hostname: &str, session: &str) -> Result<bool> {
        if command.is_empty() {
            return Ok(false);
        }

        let ts = timestamp.unwrap_or_else(|| chrono::Utc::now().timestamp());

        let exists: bool = self._conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM history WHERE command = ?1 AND start_ts = ?2)",
            rusqlite::params![command, ts],
            |row| row.get(0),
        )?;

        if exists {
            return Ok(false);
        }

        self._conn.execute(
            "INSERT INTO history (command, exit_code, cwd, hostname, session, start_ts, duration) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![command, 0i32, Option::<String>::None, hostname, session, ts, 0i64],
        )?;

        Ok(true)
    }
}

pub struct ImportStats {
    pub imported: u64,
    pub skipped: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn get_all_commands(db: &Database) -> Vec<String> {
        let mut stmt = db._conn.prepare("SELECT command FROM history ORDER BY start_ts").unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    #[test]
    fn test_import_simple_format() {
        let db = Database::in_memory().unwrap();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "echo hello").unwrap();
        writeln!(file, "ls -la").unwrap();
        writeln!(file, "cd /tmp").unwrap();

        let stats = db.import_zsh_history(&file.path().to_path_buf()).unwrap();

        assert_eq!(stats.imported, 3);
        assert_eq!(stats.skipped, 0);
        let mut cmds = get_all_commands(&db);
        cmds.sort();
        assert_eq!(cmds, vec!["cd /tmp", "echo hello", "ls -la"]);
    }

    #[test]
    fn test_import_extended_format() {
        let db = Database::in_memory().unwrap();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, ": 1706384400:0;echo hello").unwrap();
        writeln!(file, ": 1706384401:5;sleep 5").unwrap();

        let stats = db.import_zsh_history(&file.path().to_path_buf()).unwrap();

        assert_eq!(stats.imported, 2);
        let cmds = get_all_commands(&db);
        assert_eq!(cmds, vec!["echo hello", "sleep 5"]);
    }

    #[test]
    fn test_import_multiline_command() {
        let db = Database::in_memory().unwrap();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, ": 1706384400:0;echo \"hello \\").unwrap();
        writeln!(file, "world\"").unwrap();

        let stats = db.import_zsh_history(&file.path().to_path_buf()).unwrap();

        assert_eq!(stats.imported, 1);
        let cmds = get_all_commands(&db);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], "echo \"hello \\\nworld\"");
    }

    #[test]
    fn test_import_escaped_backslash_not_continuation() {
        let db = Database::in_memory().unwrap();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, ": 1706384400:0;echo test\\\\").unwrap();
        writeln!(file, ": 1706384401:0;echo next").unwrap();

        let stats = db.import_zsh_history(&file.path().to_path_buf()).unwrap();

        assert_eq!(stats.imported, 2);
        let cmds = get_all_commands(&db);
        assert_eq!(cmds, vec!["echo test\\\\", "echo next"]);
    }

    #[test]
    fn test_import_command_with_semicolon() {
        let db = Database::in_memory().unwrap();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, ": 1706384400:0;echo hello; echo world").unwrap();

        let stats = db.import_zsh_history(&file.path().to_path_buf()).unwrap();

        assert_eq!(stats.imported, 1);
        let cmds = get_all_commands(&db);
        assert_eq!(cmds, vec!["echo hello; echo world"]);
    }

    #[test]
    fn test_import_duplicates_skipped() {
        let db = Database::in_memory().unwrap();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, ": 1706384400:0;echo hello").unwrap();
        writeln!(file, ": 1706384400:0;echo hello").unwrap();

        let stats = db.import_zsh_history(&file.path().to_path_buf()).unwrap();

        assert_eq!(stats.imported, 1);
        assert_eq!(stats.skipped, 1);
    }

    #[test]
    fn test_import_empty_lines_ignored() {
        let db = Database::in_memory().unwrap();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "echo hello").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "echo world").unwrap();

        let stats = db.import_zsh_history(&file.path().to_path_buf()).unwrap();

        assert_eq!(stats.imported, 2);
        let cmds = get_all_commands(&db);
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn test_import_triple_backslash_is_continuation() {
        let db = Database::in_memory().unwrap();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, ": 1706384400:0;echo test\\\\\\").unwrap();
        writeln!(file, "continued").unwrap();

        let stats = db.import_zsh_history(&file.path().to_path_buf()).unwrap();

        assert_eq!(stats.imported, 1);
        let cmds = get_all_commands(&db);
        assert_eq!(cmds[0], "echo test\\\\\\\ncontinued");
    }

    #[test]
    fn test_import_colon_command_not_extended() {
        let db = Database::in_memory().unwrap();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, ": this is a comment;not metadata").unwrap();

        let stats = db.import_zsh_history(&file.path().to_path_buf()).unwrap();

        assert_eq!(stats.imported, 1);
        let cmds = get_all_commands(&db);
        assert_eq!(cmds[0], ": this is a comment;not metadata");
    }

    #[test]
    fn test_import_multiline_three_lines() {
        let db = Database::in_memory().unwrap();
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, ": 1706384400:0;for i in 1 2 3; do \\").unwrap();
        writeln!(file, "  echo $i \\").unwrap();
        writeln!(file, "done").unwrap();

        let stats = db.import_zsh_history(&file.path().to_path_buf()).unwrap();

        assert_eq!(stats.imported, 1);
        let cmds = get_all_commands(&db);
        assert_eq!(cmds[0], "for i in 1 2 3; do \\\n  echo $i \\\ndone");
    }
}