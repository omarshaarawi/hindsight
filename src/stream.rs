use crossbeam_channel::{bounded, Sender};
use rusqlite::Connection;
use std::thread;
use skim::prelude::*;

use crate::db::{Database, HistoryRecord, SavedCommand};
use crate::item::{HistoryItem, SavedCommandItem};

pub struct StreamingSearch {
    receiver: SkimItemReceiver,
    _handle: thread::JoinHandle<()>,
}

impl StreamingSearch {
    pub fn new(
        mode: String,
        limit: u32,
        session: String,
        cwd: String,
    ) -> Self {
        let (sender, receiver): (Sender<Arc<dyn SkimItem>>, SkimItemReceiver) = bounded(1000);
        
        let handle = thread::spawn(move || {
            let _ = Self::stream_results(&mode, limit, &session, &cwd, sender);
        });
        
        StreamingSearch {
            receiver,
            _handle: handle,
        }
    }
    
    fn stream_results(
        mode: &str,
        limit: u32,
        session: &str,
        cwd: &str,
        sender: Sender<Arc<dyn SkimItem>>,
    ) -> rusqlite::Result<()> {
        let db_path = Database::db_path()?;
        let conn = Connection::open(db_path)?;

        conn.execute_batch(
            "PRAGMA query_only = ON;
             PRAGMA cache_size = -32000;
             PRAGMA mmap_size = 268435456;"
        )?;

        if mode == "saved" {
            let mut stmt = conn.prepare_cached(
                "SELECT sc.id, sc.command, sc.description, sc.created_at, GROUP_CONCAT(t.name) as tags
                 FROM saved_commands sc
                 LEFT JOIN command_tags ct ON sc.id = ct.command_id
                 LEFT JOIN tags t ON ct.tag_id = t.id
                 GROUP BY sc.id
                 ORDER BY sc.created_at DESC
                 LIMIT ?1"
            )?;
            let mut rows = stmt.query([limit])?;

            while let Some(row) = rows.next()? {
                let tags_str: Option<String> = row.get(4)?;
                let tags: Vec<String> = tags_str
                    .map(|s| s.split(',').map(|t| t.to_string()).collect())
                    .unwrap_or_default();

                let saved_cmd = SavedCommand {
                    id: row.get(0)?,
                    command: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    tags,
                };

                let item = Arc::new(SavedCommandItem { command: saved_cmd }) as Arc<dyn SkimItem>;
                if sender.send(item).is_err() {
                    break;
                }
            }
        } else {
            let (query, params): (&str, Vec<&dyn rusqlite::ToSql>) = match mode {
                "session" => (
                    "SELECT command, MAX(start_ts) as start_ts, MAX(duration) as duration
                     FROM history WHERE session = ?1
                     GROUP BY command ORDER BY start_ts DESC LIMIT ?2",
                    vec![&session, &limit]
                ),
                "cwd" => (
                    "SELECT command, MAX(start_ts) as start_ts, MAX(duration) as duration
                     FROM history WHERE cwd = ?1
                     GROUP BY command ORDER BY start_ts DESC LIMIT ?2",
                    vec![&cwd, &limit]
                ),
                _ => (
                    "SELECT command, MAX(start_ts) as start_ts, MAX(duration) as duration
                     FROM history
                     GROUP BY command ORDER BY start_ts DESC LIMIT ?1",
                    vec![&limit]
                ),
            };

            let mut stmt = conn.prepare_cached(query)?;
            let mut rows = stmt.query(params.as_slice())?;

            while let Some(row) = rows.next()? {
                let record = HistoryRecord {
                    command: row.get(0)?,
                    timestamp: row.get(1)?,
                    duration: row.get(2)?,
                };

                let item = Arc::new(HistoryItem { record }) as Arc<dyn SkimItem>;
                if sender.send(item).is_err() {
                    break;
                }
            }
        }

        Ok(())
    }
    
    pub fn into_receiver(self) -> SkimItemReceiver {
        self.receiver
    }
}