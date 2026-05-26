//! Formatting utilities

/// Format bytes to human-readable size
pub fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    }
}

/// Format number with thousands separator
pub fn format_number(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

/// Truncate string to max length with ellipsis
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Format Kafka offset
pub fn format_offset(offset: i64) -> String {
    if offset < 0 {
        "Latest".to_string()
    } else {
        format_number(offset)
    }
}

/// Check if string looks like JSON
pub fn is_json_like(s: &str) -> bool {
    s.trim().starts_with('{') || s.trim().starts_with('[')
}

/// Pretty print JSON if valid
pub fn pretty_json(s: &str) -> Option<String> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(s) {
        serde_json::to_string_pretty(&json).ok()
    } else {
        None
    }
}