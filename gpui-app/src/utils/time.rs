//! Time utilities

use chrono::{DateTime, Utc};

/// Format timestamp to human-readable string
pub fn format_timestamp(timestamp_ms: i64) -> String {
    if timestamp_ms == 0 {
        return "N/A".to_string();
    }

    DateTime::from_timestamp_millis(timestamp_ms)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Invalid".to_string())
}

/// Format duration in milliseconds to human-readable string
pub fn format_duration_ms(ms: i64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else if ms < 3_600_000 {
        let seconds = ms / 1000;
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{}m {}s", minutes, secs)
    } else {
        let seconds = ms / 1000;
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    }
}

/// Get current timestamp in milliseconds
pub fn current_timestamp_ms() -> i64 {
    Utc::now().timestamp_millis()
}

/// Parse ISO datetime string to timestamp
pub fn parse_iso_to_timestamp(iso_str: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(iso_str)
        .map(|dt| dt.timestamp_millis())
        .ok()
}