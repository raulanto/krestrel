use std::ops::Range;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentTypeCategory {
    Json,
    Xml,
    Html,
    Text,
    Image,
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonParseError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl std::fmt::Display for JsonParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "JSON inválido en línea {}, columna {}: {}",
            self.line, self.column, self.message
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxToken {
    Key,
    String,
    Number,
    Boolean,
    Null,
    Punctuation,
    Tag,
    AttributeName,
    AttributeValue,
    Comment,
    Plain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightRange {
    pub range: Range<usize>,
    pub token: SyntaxToken,
}

/// Formats a duration nicely (e.g. "842 ms", "1.24 s", "< 1 ms")
pub fn format_duration(duration: Duration) -> String {
    let millis = duration.as_secs_f64() * 1000.0;
    if millis < 1.0 && duration.as_micros() > 0 {
        format!("{:.0} µs", duration.as_micros())
    } else if millis < 1.0 {
        "< 1 ms".to_string()
    } else if millis >= 1000.0 {
        format!("{:.2} s", millis / 1000.0)
    } else {
        format!("{:.0} ms", millis)
    }
}

/// Formats byte size into human-readable string (e.g. "450 B", "12.4 KB", "3.2 MB")
pub fn format_byte_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        let kb = bytes as f64 / 1024.0;
        format!("{:.1} KB", kb)
    } else if bytes < 1024 * 1024 * 1024 {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        format!("{:.2} MB", mb)
    } else {
        let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        format!("{:.2} GB", gb)
    }
}

/// Categorizes content type based on Content-Type header and body magic bytes
pub fn detect_content_type(content_type_header: Option<&str>, body: &[u8]) -> ContentTypeCategory {
    if let Some(ct) = content_type_header {
        let ct_lower = ct.to_lowercase();
        if ct_lower.contains("json") || ct_lower.contains("+json") {
            return ContentTypeCategory::Json;
        }
        if ct_lower.contains("xml") || ct_lower.contains("+xml") {
            return ContentTypeCategory::Xml;
        }
        if ct_lower.contains("html") {
            return ContentTypeCategory::Html;
        }
        if ct_lower.contains("image/") {
            return ContentTypeCategory::Image;
        }
        if ct_lower.contains("text/") || ct_lower.contains("javascript") || ct_lower.contains("css")
        {
            return ContentTypeCategory::Text;
        }
    }

    // Sniff magic bytes if Content-Type was missing or generic octet-stream
    if is_image_bytes(body) {
        return ContentTypeCategory::Image;
    }

    // Check if trimmed body looks like JSON
    let trimmed = trim_ascii_whitespace(body);
    if (trimmed.starts_with(b"{") && trimmed.ends_with(b"}"))
        || (trimmed.starts_with(b"[") && trimmed.ends_with(b"]"))
    {
        return ContentTypeCategory::Json;
    }

    // Check if trimmed body looks like XML/HTML
    if trimmed.starts_with(b"<") && trimmed.ends_with(b">") {
        if trimmed.starts_with(b"<!DOCTYPE html") || trimmed.starts_with(b"<html") {
            return ContentTypeCategory::Html;
        }
        return ContentTypeCategory::Xml;
    }

    // Check for null bytes / binary content
    if body.iter().take(1024).any(|&b| b == 0) {
        ContentTypeCategory::Binary
    } else {
        ContentTypeCategory::Text
    }
}

fn trim_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    let mut end = bytes.len();
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    &bytes[start..end]
}

/// Checks if bytes start with common image signatures (PNG, JPEG, GIF, WebP, SVG, ICO)
pub fn is_image_bytes(body: &[u8]) -> bool {
    if body.len() < 4 {
        return false;
    }
    // PNG: 89 50 4E 47
    if body.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return true;
    }
    // JPEG: FF D8 FF
    if body.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return true;
    }
    // GIF: "GIF87a" or "GIF89a"
    if body.starts_with(b"GIF87a") || body.starts_with(b"GIF89a") {
        return true;
    }
    // WebP: "RIFF....WEBP"
    if body.len() >= 12 && body.starts_with(b"RIFF") && &body[8..12] == b"WEBP" {
        return true;
    }
    // SVG: "<svg"
    let trimmed = trim_ascii_whitespace(body);
    if trimmed.starts_with(b"<svg")
        || trimmed.starts_with(b"<?xml") && trimmed.windows(4).any(|w| w == b"<svg")
    {
        return true;
    }
    // ICO: 00 00 01 00
    if body.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
        return true;
    }
    false
}

/// Decodes bytes to string, stripping BOM and normalizing line endings for display only.
/// Returns (DecodedString, is_lossy).
pub fn decode_body(body: &[u8], _content_type: Option<&str>) -> (String, bool) {
    let mut raw_bytes = body;

    // Check and strip UTF-8 BOM if present (EF BB BF)
    if raw_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        raw_bytes = &raw_bytes[3..];
    }

    match std::str::from_utf8(raw_bytes) {
        Ok(valid_str) => {
            let normalized = normalize_newlines(valid_str);
            (normalized, false)
        }
        Err(_) => {
            // Lossy fallback
            let lossy = String::from_utf8_lossy(raw_bytes);
            let normalized = normalize_newlines(&lossy);
            (normalized, true)
        }
    }
}

fn normalize_newlines(s: &str) -> String {
    if s.contains("\r\n") {
        s.replace("\r\n", "\n")
    } else {
        s.to_string()
    }
}

/// Formats JSON with 2 spaces indentation without losing precision on numbers or keys
pub fn format_pretty_json(raw_json: &str) -> Result<String, JsonParseError> {
    let trimmed = raw_json.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    // Try parsing with serde_json::Value
    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(val) => serde_json::to_string_pretty(&val).map_err(|e| JsonParseError {
            line: e.line(),
            column: e.column(),
            message: e.to_string(),
        }),
        Err(e) => Err(JsonParseError {
            line: e.line(),
            column: e.column(),
            message: e.to_string(),
        }),
    }
}

/// Formats XML with indentation if possible, otherwise returns as-is
pub fn format_pretty_xml(raw_xml: &str) -> String {
    let trimmed = raw_xml.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // Simple pure indentation helper for XML tags
    let mut out = String::with_capacity(trimmed.len() + 128);
    let mut indent: usize = 0;
    let mut chars = trimmed.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            tag.push('<');
            let mut is_closing = false;
            let mut is_self_closing = false;
            let mut is_decl = false;

            if chars.peek() == Some(&'/') {
                is_closing = true;
                tag.push(chars.next().unwrap());
            } else if chars.peek() == Some(&'?') || chars.peek() == Some(&'!') {
                is_decl = true;
            }

            for tc in chars.by_ref() {
                tag.push(tc);
                if tc == '>' {
                    break;
                }
            }

            if tag.ends_with("/>") {
                is_self_closing = true;
            }

            if is_closing && indent > 0 {
                indent -= 1;
            }

            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&"  ".repeat(indent));
            out.push_str(&tag);

            if !is_closing && !is_self_closing && !is_decl {
                indent += 1;
            }
        } else if c == '\n' || c == '\r' {
            // ignore raw newlines to reformat cleanly
        } else {
            // Text content between tags
            let mut text = String::new();
            text.push(c);
            while let Some(&next_c) = chars.peek() {
                if next_c == '<' {
                    break;
                }
                text.push(chars.next().unwrap());
            }
            let trimmed_text = text.trim();
            if !trimmed_text.is_empty() {
                out.push_str(trimmed_text);
            }
        }
    }

    if out.is_empty() {
        trimmed.to_string()
    } else {
        out
    }
}

/// Tokenizes JSON string into highlight ranges for syntax coloring
pub fn highlight_json(text: &str) -> Vec<HighlightRange> {
    let mut highlights = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        match bytes[i] {
            b'"' => {
                let start = i;
                i += 1;
                let mut escaped = false;
                while i < len {
                    if escaped {
                        escaped = false;
                    } else if bytes[i] == b'\\' {
                        escaped = true;
                    } else if bytes[i] == b'"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                let end = i;

                // Check if this string is an object key by peeking ahead for ':'
                let mut peek = end;
                while peek < len && bytes[peek].is_ascii_whitespace() {
                    peek += 1;
                }
                let is_key = peek < len && bytes[peek] == b':';

                highlights.push(HighlightRange {
                    range: start..end,
                    token: if is_key {
                        SyntaxToken::Key
                    } else {
                        SyntaxToken::String
                    },
                });
            }
            b't' if bytes[i..].starts_with(b"true") => {
                highlights.push(HighlightRange {
                    range: i..i + 4,
                    token: SyntaxToken::Boolean,
                });
                i += 4;
            }
            b'f' if bytes[i..].starts_with(b"false") => {
                highlights.push(HighlightRange {
                    range: i..i + 5,
                    token: SyntaxToken::Boolean,
                });
                i += 5;
            }
            b'n' if bytes[i..].starts_with(b"null") => {
                highlights.push(HighlightRange {
                    range: i..i + 4,
                    token: SyntaxToken::Null,
                });
                i += 4;
            }
            b'-' | b'0'..=b'9' => {
                let start = i;
                i += 1;
                while i < len
                    && (bytes[i].is_ascii_digit()
                        || bytes[i] == b'.'
                        || bytes[i] == b'e'
                        || bytes[i] == b'E'
                        || bytes[i] == b'+'
                        || bytes[i] == b'-')
                {
                    i += 1;
                }
                highlights.push(HighlightRange {
                    range: start..i,
                    token: SyntaxToken::Number,
                });
            }
            b'{' | b'}' | b'[' | b']' | b':' | b',' => {
                highlights.push(HighlightRange {
                    range: i..i + 1,
                    token: SyntaxToken::Punctuation,
                });
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    highlights
}

/// Finds all match ranges of `query` in `text`
pub fn search_matches(text: &str, query: &str, case_sensitive: bool) -> Vec<Range<usize>> {
    if query.is_empty() || text.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();

    if case_sensitive {
        let mut start = 0;
        while let Some(idx) = text[start..].find(query) {
            let actual_start = start + idx;
            let actual_end = actual_start + query.len();
            matches.push(actual_start..actual_end);
            start = actual_end;
        }
    } else {
        let text_lower = text.to_lowercase();
        let query_lower = query.to_lowercase();
        let mut start = 0;
        while let Some(idx) = text_lower[start..].find(&query_lower) {
            let actual_start = start + idx;
            let actual_end = actual_start + query.len();
            matches.push(actual_start..actual_end);
            start = actual_end;
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_millis(842)), "842 ms");
        assert_eq!(format_duration(Duration::from_millis(1240)), "1.24 s");
        assert_eq!(format_duration(Duration::from_micros(450)), "450 µs");
        assert_eq!(format_duration(Duration::from_nanos(0)), "< 1 ms");
    }

    #[test]
    fn test_format_byte_size() {
        assert_eq!(format_byte_size(0), "0 B");
        assert_eq!(format_byte_size(500), "500 B");
        assert_eq!(format_byte_size(1024), "1.0 KB");
        assert_eq!(format_byte_size(15 * 1024), "15.0 KB");
        assert_eq!(format_byte_size(2 * 1024 * 1024), "2.00 MB");
    }

    #[test]
    fn test_detect_content_type() {
        assert_eq!(
            detect_content_type(Some("application/json; charset=utf-8"), b"{}"),
            ContentTypeCategory::Json
        );
        assert_eq!(
            detect_content_type(Some("text/xml"), b"<xml/>"),
            ContentTypeCategory::Xml
        );
        assert_eq!(
            detect_content_type(Some("image/png"), &[0x89, 0x50, 0x4E, 0x47]),
            ContentTypeCategory::Image
        );
        assert_eq!(
            detect_content_type(None, b"{\"a\": 1}"),
            ContentTypeCategory::Json
        );
    }

    #[test]
    fn test_decode_body_utf8_and_bom() {
        let with_bom = vec![0xEF, 0xBB, 0xBF, b'H', b'o', b'l', b'a'];
        let (decoded, is_lossy) = decode_body(&with_bom, None);
        assert_eq!(decoded, "Hola");
        assert!(!is_lossy);

        let crlf = b"Linea 1\r\nLinea 2";
        let (decoded_crlf, _) = decode_body(crlf, None);
        assert_eq!(decoded_crlf, "Linea 1\nLinea 2");
    }

    #[test]
    fn test_format_pretty_json_valid_and_invalid() {
        let raw = r#"{"name":"Kestrel","active":true,"count":10000000000000000000}"#;
        let formatted = format_pretty_json(raw).expect("Valid JSON");
        assert!(formatted.contains("  \"name\": \"Kestrel\""));
        assert!(formatted.contains("10000000000000000000"));

        let invalid = r#"{"name": "broken",}"#;
        let err = format_pretty_json(invalid).unwrap_err();
        assert!(err.line >= 1);
    }

    #[test]
    fn test_highlight_json() {
        let json = r#"{"key": "value", "num": 42, "b": true, "n": null}"#;
        let tokens = highlight_json(json);
        assert!(!tokens.is_empty());
        assert!(tokens.iter().any(|t| t.token == SyntaxToken::Key));
        assert!(tokens.iter().any(|t| t.token == SyntaxToken::String));
        assert!(tokens.iter().any(|t| t.token == SyntaxToken::Number));
        assert!(tokens.iter().any(|t| t.token == SyntaxToken::Boolean));
        assert!(tokens.iter().any(|t| t.token == SyntaxToken::Null));
    }

    #[test]
    fn test_search_matches() {
        let text = "El rápido zorro marrón salta sobre el perro rápido.";
        let matches = search_matches(text, "rápido", false);
        assert_eq!(matches.len(), 2);
    }
}
