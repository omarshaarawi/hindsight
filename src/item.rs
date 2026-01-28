use skim::prelude::*;
use std::borrow::Cow;
use crate::db::{HistoryRecord, SavedCommand};
use chrono::Utc;

fn format_duration(seconds: i64) -> String {
    if seconds <= 0 {
        "0s".to_string()
    } else if seconds < 60 {
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
    if timestamp == 0 {
        return "unknown".to_string();
    }
    let diff = Utc::now().timestamp() - timestamp;
    if diff < 0 {
        return "just now".to_string();
    }
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
        let duration_secs = self.record.duration;
        let duration_str = if duration_secs < 60 {
            format!("{}s", duration_secs)
        } else {
            format!("{}m {}s", duration_secs / 60, duration_secs % 60)
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

pub struct SavedCommandItem {
    pub command: SavedCommand,
}

impl SkimItem for SavedCommandItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.command.command)
    }

    fn display<'a>(&'a self, _context: DisplayContext<'a>) -> AnsiString<'a> {
        let tags_str = if self.command.tags.is_empty() {
            String::new()
        } else {
            format!("[{}] ", self.command.tags.join(", "))
        };

        let desc_str = self.command.description
            .as_ref()
            .map(|d| format!(" - {}", d))
            .unwrap_or_default();

        let display_str = format!("{}{}{}", tags_str, self.command.command, desc_str);
        AnsiString::new_string(display_str, vec![])
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let created = chrono::DateTime::from_timestamp(self.command.created_at, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let tags_str = if self.command.tags.is_empty() {
            "None".to_string()
        } else {
            self.command.tags.join(", ")
        };

        let desc_str = self.command.description
            .as_ref()
            .map(|d| d.as_str())
            .unwrap_or("None");

        let preview = format!(
            "Command: {}\nTags: {}\nDescription: {}\nCreated: {}",
            self.command.command, tags_str, desc_str, created
        );

        ItemPreview::Text(preview)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(5), "5s");
        assert_eq!(format_duration(59), "59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60), "1m");
        assert_eq!(format_duration(90), "1m");
        assert_eq!(format_duration(3599), "59m");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3600), "1h");
        assert_eq!(format_duration(7200), "2h");
        assert_eq!(format_duration(86399), "23h");
    }

    #[test]
    fn test_format_duration_days() {
        assert_eq!(format_duration(86400), "1d");
        assert_eq!(format_duration(172800), "2d");
    }

    #[test]
    fn test_format_duration_negative() {
        assert_eq!(format_duration(-5), "0s");
        assert_eq!(format_duration(-100), "0s");
    }

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(format_duration(0), "0s");
    }

    #[test]
    fn test_format_age_zero_timestamp() {
        assert_eq!(format_age(0), "unknown");
    }

    #[test]
    fn test_format_age_future_timestamp() {
        let future = Utc::now().timestamp() + 1000;
        assert_eq!(format_age(future), "just now");
    }
}