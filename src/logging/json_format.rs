//! Custom JSON formatter that extracts [TAG] prefixes from log messages.
//!
//! This module provides a custom tracing-subscriber layer that formats log
//! entries as JSON Lines with automatic tag extraction.

use serde_json::json;
use tracing::{
    field::Visit,
    Event, Level, Subscriber,
};
use tracing_subscriber::{
    fmt::{
        format::{FmtSpan, FormatEvent, FormatFields, Writer},
        FmtContext, FormattedFields,
    },
    registry::LookupSpan,
};

/// Extracts a tag from a message that starts with `[TAG]`.
///
/// # Examples
/// - `"[ZONE LOADER] Loading zone 1"` -> `Some("ZONE LOADER")`
/// - `"[VFS] File not found"` -> `Some("VFS")`
/// - `"No tag here"` -> `None`
pub fn extract_tag(message: &str) -> Option<&str> {
    if !message.starts_with('[') {
        return None;
    }
    
    let end = message.find(']')?;
    let tag = &message[1..end];
    
    // Skip empty tags
    if tag.trim().is_empty() {
        return None;
    }
    
    Some(tag)
}

/// Removes the [TAG] prefix from a message and returns the cleaned message.
pub fn remove_tag_prefix(message: &str) -> &str {
    if let Some(end) = message.find(']') {
        let rest = &message[end + 1..];
        rest.trim_start()
    } else {
        message
    }
}

/// A custom JSON format that extracts tags from log messages.
///
/// This formatter produces JSON Lines output where each line is a valid JSON object:
/// ```json
/// {"ts":"2026-03-02T11:24:11.123-06:00","level":"INFO","tag":"ZONE LOADER","msg":"Loading zone 1...","kvs":{}}
/// ```
#[derive(Debug)]
pub struct TagExtractingJsonFormat {
    span_events: FmtSpan,
    ansi: bool,
    display_target: bool,
    display_filename: bool,
    display_line_number: bool,
}

impl Default for TagExtractingJsonFormat {
    fn default() -> Self {
        Self {
            span_events: FmtSpan::NONE,
            ansi: false,
            display_target: true,
            display_filename: false,
            display_line_number: false,
        }
    }
}

impl TagExtractingJsonFormat {
    /// Create a new JSON format with tag extraction.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable ANSI terminal colors.
    pub fn with_ansi(mut self, ansi: bool) -> Self {
        self.ansi = ansi;
        self
    }

    /// Enable or disable the target (module path) in the output.
    pub fn with_target(mut self, display_target: bool) -> Self {
        self.display_target = display_target;
        self
    }

    /// Enable or disable the filename in the output.
    pub fn with_file(mut self, display_filename: bool) -> Self {
        self.display_filename = display_filename;
        self
    }

    /// Enable or disable the line number in the output.
    pub fn with_line_number(mut self, display_line_number: bool) -> Self {
        self.display_line_number = display_line_number;
        self
    }

    /// Configure which span events to include.
    pub fn with_span_events(mut self, span_events: FmtSpan) -> Self {
        self.span_events = span_events;
        self
    }
}

impl<S, N> FormatEvent<S, N> for TagExtractingJsonFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let metadata = event.metadata();
        
        // Get timestamp in ISO 8601 format with timezone
        let timestamp = chrono::Local::now().to_rfc3339();
        
        // Get log level
        let level = match *metadata.level() {
            Level::ERROR => "ERROR",
            Level::WARN => "WARN",
            Level::INFO => "INFO",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "TRACE",
        };

        // Collect the message and key-value pairs
        let mut visitor = JsonVisitor::new();
        event.record(&mut visitor);
        
        // Extract tag from message if present
        let (tag, message) = if let Some(msg) = &visitor.message {
            if let Some(tag_str) = extract_tag(msg) {
                (Some(tag_str.to_string()), remove_tag_prefix(msg).to_string())
            } else {
                (None, msg.clone())
            }
        } else {
            (None, String::new())
        };

        // Build the JSON object
        let mut json_obj = serde_json::Map::new();
        json_obj.insert("ts".to_string(), json!(timestamp));
        json_obj.insert("level".to_string(), json!(level));
        json_obj.insert("tag".to_string(), json!(tag));
        json_obj.insert("msg".to_string(), json!(message));
        
        // Add target (module path) if enabled
        if self.display_target {
            json_obj.insert("target".to_string(), json!(metadata.target()));
        }
        
        // Add filename if enabled
        if self.display_filename {
            if let Some(filename) = metadata.file() {
                json_obj.insert("file".to_string(), json!(filename));
            }
        }
        
        // Add line number if enabled
        if self.display_line_number {
            if let Some(line) = metadata.line() {
                json_obj.insert("line".to_string(), json!(line));
            }
        }
        
        // Add span context if available
        if let Some(span) = ctx.lookup_current() {
            let span_name = span.name();
            let mut span_obj = serde_json::Map::new();
            span_obj.insert("name".to_string(), json!(span_name));
            
            // Add span fields if available
            if let Some(fields) = span.extensions().get::<FormattedFields<N>>() {
                if !fields.fields.is_empty() {
                    span_obj.insert("fields".to_string(), json!(fields.fields.as_str()));
                }
            }
            
            json_obj.insert("span".to_string(), json!(span_obj));
        } else {
            json_obj.insert("span".to_string(), json!(serde_json::Value::Null));
        }
        
        // Add any additional key-value pairs
        if !visitor.kvs.is_empty() {
            json_obj.insert("kvs".to_string(), json!(visitor.kvs));
        } else {
            json_obj.insert("kvs".to_string(), json!({}));
        }

        // Write the JSON line
        let json_str = serde_json::to_string(&json_obj)
            .map_err(|_| std::fmt::Error)?;
        
        writer.write_str(&json_str)?;
        writer.write_str("\n")?;
        
        Ok(())
    }
}

/// Visitor to collect fields from a tracing event.
struct JsonVisitor {
    message: Option<String>,
    kvs: serde_json::Map<String, serde_json::Value>,
}

impl JsonVisitor {
    fn new() -> Self {
        Self {
            message: None,
            kvs: serde_json::Map::new(),
        }
    }
}

impl Visit for JsonVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.kvs.insert(field.name().to_string(), json!(value));
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        } else {
            self.kvs.insert(field.name().to_string(), json!(format!("{:?}", value)));
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.kvs.insert(field.name().to_string(), json!(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.kvs.insert(field.name().to_string(), json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.kvs.insert(field.name().to_string(), json!(value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.kvs.insert(field.name().to_string(), json!(value));
    }

    fn record_error(&mut self, field: &tracing::field::Field, value: &(dyn std::error::Error + 'static)) {
        self.kvs.insert(field.name().to_string(), json!(value.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tag() {
        assert_eq!(
            extract_tag("[ZONE LOADER] Loading zone 1"),
            Some("ZONE LOADER")
        );
        assert_eq!(extract_tag("[VFS] File not found"), Some("VFS"));
        assert_eq!(extract_tag("[MEMORY] Failed to allocate"), Some("MEMORY"));
        assert_eq!(extract_tag("No tag here"), None);
        assert_eq!(extract_tag(""), None);
        assert_eq!(extract_tag("[] Empty tag"), None);
        assert_eq!(extract_tag("[   ] Whitespace tag"), None);
    }

    #[test]
    fn test_remove_tag_prefix() {
        assert_eq!(
            remove_tag_prefix("[ZONE LOADER] Loading zone 1"),
            "Loading zone 1"
        );
        assert_eq!(
            remove_tag_prefix("[VFS]File not found"),
            "File not found"
        );
        assert_eq!(remove_tag_prefix("No tag here"), "No tag here");
    }
}
