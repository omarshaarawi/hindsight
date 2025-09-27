use skim::prelude::*;
use std::borrow::Cow;
use crate::db::HistoryRecord;
use chrono::Utc;

fn format_duration(ms: i64) -> String {
    let seconds = ms / 1000;
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86400 {
        format!("{}h", seconds / 3600)
    } else {
        format!("{}d", seconds / 86400)
    }
}

fn format_age(timestamp: i64) -> String {
    let diff = Utc::now().timestamp() - timestamp;
    let unit = if diff < 60 {
        format!("{}s", diff)
    } else if diff < 3600 {
        format!("{}m", diff / 60)
    } else if diff < 86400 {
        format!("{}h", diff / 3600)
    } else {
        format!("{}d", diff / 86400)
    };
    format!("{} ago", unit)
}

pub struct HistoryItem {
    pub record: HistoryRecord,
}

impl SkimItem for HistoryItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.record.command)
    }
    
    fn display<'a>(&'a self, _context: DisplayContext<'a>) -> AnsiString<'a> {
        let duration = format_duration(self.record.duration);
        let age = format_age(self.record.timestamp);
        let display_str = format!("{:<6} {:<10}\t{}", duration, age, self.record.command);
        AnsiString::new_string(display_str, vec![])
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