//! Output event compressor — compress DAP `output` events.
//!
//! The `output` event is emitted by DAP servers to send text output
//! (stdout, stderr, console) from the debuggee process. This can be
//! extremely large in long-running debug sessions.
//!
//! ## Compression Strategies
//!
//! 1. **ANSI stripping**: remove color/style escape sequences (pure noise for LLMs)
//! 2. **Repeated line folding**: consecutive identical output lines → `(xN)` suffix
//! 3. **Category abbreviation**: `stdout`→`O`, `stderr`→`E`, `console`→`C`
//! 4. **Source abbreviation**: shorten source paths
//! 5. **Noise field removal**: drop `data`, `line`, `column` (not useful for LLM)

use crate::codec::json_rpc::DapMessage;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
use crate::interceptors::utils::{abbreviate_source, strip_ansi};
use crate::proxy::Direction;

/// Compressor for DAP `output` events.
pub struct OutputCompressor;

#[async_trait::async_trait]
impl Interceptor for OutputCompressor {
    fn name(&self) -> &str {
        "output_compressor"
    }

    fn applies_to(&self, msg: &DapMessage, direction: Direction) -> bool {
        direction == Direction::ServerToClient
            && msg.msg_type == "event"
            && msg.event.as_deref() == Some("output")
    }

    async fn intercept(
        &self,
        mut msg: DapMessage,
        _direction: Direction,
    ) -> Result<Option<DapMessage>, DapzError> {
        if let Some(ref mut body) = msg.body {
            // 1. Strip ANSI escape sequences from output text
            if let Some(output) = body.get("output").and_then(|v| v.as_str()) {
                let cleaned = strip_ansi(output);
                body["output"] = serde_json::Value::String(cleaned);
            }

            // 2. Abbreviate category
            if let Some(cat) = body.get("category").and_then(|v| v.as_str()) {
                let abbr = match cat {
                    "stdout" => "O",
                    "stderr" => "E",
                    "console" => "C",
                    "stdin" => "I",
                    "telemetry" => "T",
                    _ => cat,
                };
                body["category"] = serde_json::Value::String(abbr.into());
            }

            // 3. Compress repeated lines in output text
            if let Some(output) = body.get("output").and_then(|v| v.as_str()) {
                let compressed = compress_output_text(output);
                body["output"] = serde_json::Value::String(compressed);
            }

            // 4. Abbreviate source if present
            if let Some(source) = body.get_mut("source") {
                abbreviate_source(source);
            }

            // 5. Remove noise fields (not useful for LLM debug flow)
            if let Some(obj) = body.as_object_mut() {
                obj.remove("data");
                obj.remove("line");
                obj.remove("column");
            }
        }
        Ok(Some(msg))
    }
}

/// Compress output text by folding consecutive repeated lines.
///
/// Input:  "line1\nline2\nline2\nline2\nline3"
/// Output: "line1\nline2 (x2)\nline3"
fn compress_output_text(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut count = 1;
    let mut prev = lines[0];

    for line in &lines[1..] {
        if *line == prev {
            count += 1;
        } else {
            append_line(&mut result, prev, count);
            prev = line;
            count = 1;
        }
    }
    append_line(&mut result, prev, count);

    result
}

fn append_line(result: &mut String, line: &str, count: usize) {
    if !result.is_empty() {
        result.push('\n');
    }
    result.push_str(line);
    if count > 1 {
        result.push_str(&format!(" (x{})", count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_output_text_no_repeats() {
        let input = "line1\nline2\nline3";
        let output = compress_output_text(input);
        assert_eq!(output, "line1\nline2\nline3");
    }

    #[test]
    fn test_compress_output_text_with_repeats() {
        let input = "a\nb\nb\nb\nc";
        let output = compress_output_text(input);
        assert_eq!(output, "a\nb (x3)\nc");
    }

    #[test]
    fn test_compress_output_text_empty() {
        assert_eq!(compress_output_text(""), "");
    }

    #[tokio::test]
    async fn test_output_compressor_abbreviates_category_and_compresses() {
        let compressor = OutputCompressor;
        let msg = DapMessage {
            seq: 1,
            msg_type: "event".into(),
            command: None,
            event: Some("output".into()),
            request_seq: None,
            success: None,
            body: Some(serde_json::json!({
                "category": "stdout",
                "output": "hello\nworld\nworld"
            })),
            arguments: None,
        };

        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        assert_eq!(body["category"], "O");
        assert_eq!(body["output"], "hello\nworld (x2)");
    }

    #[tokio::test]
    async fn test_output_compressor_strips_ansi() {
        let compressor = OutputCompressor;
        let msg = DapMessage {
            seq: 1,
            msg_type: "event".into(),
            command: None,
            event: Some("output".into()),
            request_seq: None,
            success: None,
            body: Some(serde_json::json!({
                "category": "stdout",
                "output": "\x1b[31mERROR\x1b[0m: something broke\n\x1b[33mWARN\x1b[0m: caution"
            })),
            arguments: None,
        };

        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        assert_eq!(body["output"], "ERROR: something broke\nWARN: caution");
    }

    #[tokio::test]
    async fn test_output_compressor_abbreviates_source() {
        let compressor = OutputCompressor;
        let msg = DapMessage {
            seq: 1,
            msg_type: "event".into(),
            command: None,
            event: Some("output".into()),
            request_seq: None,
            success: None,
            body: Some(serde_json::json!({
                "category": "stdout",
                "output": "hello",
                "source": {
                    "path": "/home/user/project/src/main.rs",
                    "name": "main.rs",
                    "checksums": [{"algorithm": "md5", "checksum": "abc"}],
                }
            })),
            arguments: None,
        };

        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let source = body.get("source").unwrap();
        assert_eq!(source["path"], "src/main.rs");
        assert!(!source.as_object().unwrap().contains_key("checksums"));
    }

    #[tokio::test]
    async fn test_output_compressor_removes_noise_fields() {
        let compressor = OutputCompressor;
        let msg = DapMessage {
            seq: 1,
            msg_type: "event".into(),
            command: None,
            event: Some("output".into()),
            request_seq: None,
            success: None,
            body: Some(serde_json::json!({
                "category": "stdout",
                "output": "hello",
                "data": {"extra": "info"},
                "line": 42,
                "column": 10,
            })),
            arguments: None,
        };

        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let obj = body.as_object().unwrap();
        assert!(!obj.contains_key("data"));
        assert!(!obj.contains_key("line"));
        assert!(!obj.contains_key("column"));
        assert!(obj.contains_key("output"));
        assert!(obj.contains_key("category"));
    }

    #[tokio::test]
    async fn test_output_does_not_apply_to_non_output() {
        let compressor = OutputCompressor;
        let msg = DapMessage {
            seq: 1,
            msg_type: "event".into(),
            command: None,
            event: Some("stopped".into()),
            request_seq: None,
            success: None,
            body: None,
            arguments: None,
        };
        assert!(!compressor.applies_to(&msg, Direction::ServerToClient));
    }
}
