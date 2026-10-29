use crate::logging::{Level, LogSink};

/// Writes one JSON object per line to stdout, for a tool or an agent to parse
/// rather than scrape: `{"level":"warn","category":"engine","message":"…"}`.
pub struct JsonSink;

impl JsonSink {
    /// The line `log` writes, without its newline.
    pub fn format(level: Level, category: &str, message: &str) -> String {
        format!(
            "{{\"level\":\"{}\",\"category\":\"{}\",\"message\":\"{}\"}}",
            level.as_str().to_ascii_lowercase(),
            escape(category),
            escape(message)
        )
    }
}

impl LogSink for JsonSink {
    fn log(&self, level: Level, category: &str, message: &str) {
        println!("{}", Self::format(level, category, message));
    }
}

/// Escapes a string for a JSON string literal: quotes, backslashes, and
/// every control character.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => escaped.push_str(&format!("\\u{:04x}", c as u32)),
            c => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests;
