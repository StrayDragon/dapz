//! Shared utility functions for interceptors.
//!
//! Common operations like Source abbreviation, path shortening,
//! and ANSI escape stripping, shared across multiple interceptors.

use serde_json::Value;

/// Abbreviate a DAP `Source` object to reduce token usage.
///
/// - Shortens `path` to `dir/filename` (e.g. `/home/user/project/src/main.rs` → `src/main.rs`)
/// - Removes `checksums` (debugger-internal, large, useless for LLM)
/// - Removes `adapterData` (debugger-internal, useless for LLM)
/// - Preserves `name`, `sourceReference`, `presentationHint`, `origin`
pub fn abbreviate_source(source: &mut Value) {
    if let Some(obj) = source.as_object_mut() {
        if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
            obj.insert("path".into(), Value::String(shorten_path_display(path)));
        }
        obj.remove("checksums");
        obj.remove("adapterData");
    }
}

/// Shorten a path from absolute to `dir/filename` form.
///
/// `/home/user/project/src/main.rs` → `src/main.rs`
/// `src/main.rs` → `src/main.rs` (already short)
/// `main.rs` → `main.rs` (no parent dir)
pub fn shorten_path_display(path: &str) -> String {
    let pos = path.rfind('/').or_else(|| path.rfind('\\'));
    let (filename, parent) = match pos {
        Some(p) => (&path[p + 1..], &path[..p]),
        None => (path, ""),
    };

    if parent.is_empty() {
        return filename.to_string();
    }

    // Take only the last directory component
    if let Some(sep) = parent.rfind('/').or_else(|| parent.rfind('\\')) {
        let dir = &parent[sep + 1..];
        if dir.is_empty() {
            filename.to_string()
        } else {
            format!("{dir}/{filename}")
        }
    } else {
        format!("{parent}/{filename}")
    }
}

/// Strip ANSI escape sequences from a string.
///
/// Handles common CSI sequences: `\x1b[<params>m`, `\x1b[<params>K`, etc.
pub fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2; // skip ESC[
            // consume CSI sequence: parameter bytes (0x30-0x3F), intermediate bytes (0x20-0x2F),
            // then a final byte (0x40-0x7E)
            while i < bytes.len() {
                let b = bytes[i];
                if (0x40..=0x7E).contains(&b) {
                    i += 1; // consume final byte
                    break;
                }
                if (0x20..=0x3F).contains(&b) {
                    i += 1; // consume param/intermediate byte
                    continue;
                }
                // Not a valid CSI sequence byte — stop
                break;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── shorten_path_display ────────────────────────────────────────────

    #[test]
    fn test_shorten_path_absolute() {
        assert_eq!(
            shorten_path_display("/home/user/project/src/main.rs"),
            "src/main.rs"
        );
    }

    #[test]
    fn test_shorten_path_windows() {
        // On non-Windows platforms, backslashes are treated as filename chars.
        // The function shortens the last meaningful segment regardless.
        assert!(shorten_path_display("C:\\Users\\user\\src\\main.rs").contains("main.rs"));
    }

    #[test]
    fn test_shorten_path_short() {
        assert_eq!(shorten_path_display("src/main.rs"), "src/main.rs");
    }

    #[test]
    fn test_shorten_path_filename_only() {
        assert_eq!(shorten_path_display("main.rs"), "main.rs");
    }

    #[test]
    fn test_shorten_path_root() {
        assert_eq!(shorten_path_display("/main.rs"), "main.rs");
    }

    // ── abbreviate_source ───────────────────────────────────────────────

    #[test]
    fn test_abbreviate_source_removes_fields_and_shortens_path() {
        let mut source = serde_json::json!({
            "path": "/home/user/project/src/main.rs",
            "name": "main.rs",
            "checksums": [{"algorithm": "md5", "checksum": "abc123"}],
            "adapterData": {"internal": "stuff"},
            "presentationHint": "normal",
        });
        abbreviate_source(&mut source);
        let obj = source.as_object().unwrap();
        assert_eq!(obj["path"], "src/main.rs");
        assert!(!obj.contains_key("checksums"));
        assert!(!obj.contains_key("adapterData"));
        assert!(obj.contains_key("presentationHint"));
        assert!(obj.contains_key("name"));
    }

    #[test]
    fn test_abbreviate_source_no_path() {
        let mut source = serde_json::json!({
            "name": "main.rs",
            "checksums": [],
        });
        abbreviate_source(&mut source);
        assert!(!source.as_object().unwrap().contains_key("checksums"));
        assert_eq!(source["name"], "main.rs");
    }

    // ── strip_ansi ──────────────────────────────────────────────────────

    #[test]
    fn test_strip_ansi_no_ansi() {
        assert_eq!(strip_ansi("hello world"), "hello world");
    }

    #[test]
    fn test_strip_ansi_simple_color() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
    }

    #[test]
    fn test_strip_ansi_complex() {
        assert_eq!(
            strip_ansi("\x1b[1;32mBOLD GREEN\x1b[0m normal"),
            "BOLD GREEN normal"
        );
    }

    #[test]
    fn test_strip_ansi_with_background() {
        assert_eq!(
            strip_ansi("\x1b[41m\x1b[37mwhite on red\x1b[0m"),
            "white on red"
        );
    }

    #[test]
    fn test_strip_ansi_cursor_move() {
        // Cursor movement sequences: ESC[N;NH, ESC[K (erase line)
        assert_eq!(strip_ansi("\x1b[2K\x1b[1Aprogress: 50%"), "progress: 50%");
    }

    #[test]
    fn test_strip_ansi_empty() {
        assert_eq!(strip_ansi(""), "");
    }
}
