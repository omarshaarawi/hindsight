use skim::prelude::*;
use std::borrow::Cow;
use crate::db::HistoryRecord;

pub struct HistoryItem {
    pub record: HistoryRecord,
}

impl SkimItem for HistoryItem {
    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.record.command)
    }
    
    fn display<'a>(&'a self, _context: DisplayContext<'a>) -> AnsiString<'a> {
        AnsiString::new_string(self.record.command.clone(), vec![])
    }
    
    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let duration_ms = self.record.duration;
        let duration_str = if duration_ms < 1000 {
            format!("{}ms", duration_ms)
        } else {
            format!("{:.1}s", duration_ms as f64 / 1000.0)
        };
        
        let timestamp = chrono::DateTime::from_timestamp(self.record.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        let preview = format!(
            "Command: {}\nExecuted: {}\nDuration: {}",
            self.record.command, timestamp, duration_str
        );
        
        ItemPreview::Text(preview)
    }
}