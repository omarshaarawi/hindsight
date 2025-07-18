use crossbeam_channel::{bounded, Sender};
use rusqlite::Connection;
use std::thread;
use skim::prelude::*;

use crate::db::{Database, HistoryRecord};
use crate::item::HistoryItem;

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
        let (query, params): (&str, Vec<&dyn rusqlite::ToSql>) = match mode {
            "session" => (
                "SELECT DISTINCT command, MAX(start_ts) as start_ts, MAX(duration) as duration 
                 FROM history WHERE session = ?1 
                 GROUP BY command ORDER BY start_ts DESC LIMIT ?2",
                vec![&session, &limit]
            ),
            "cwd" => (
                "SELECT DISTINCT command, MAX(start_ts) as start_ts, MAX(duration) as duration 
                 FROM history WHERE cwd = ?1 
                 GROUP BY command ORDER BY start_ts DESC LIMIT ?2",
                vec![&cwd, &limit]
            ),
            _ => (
                "SELECT DISTINCT command, MAX(start_ts) as start_ts, MAX(duration) as duration 
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
        
        Ok(())
    }
    
    pub fn into_receiver(self) -> SkimItemReceiver {
        self.receiver
    }
}