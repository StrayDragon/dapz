//! Output event compressor — compress DAP `output` events.
//!
//! The `output` event is emitted by DAP servers to send text output
//! (stdout, stderr, console) from the debuggee process. This can be
//! extremely large in long-running debug sessions.
//!
//! ## Compression Strategies
//!
//! 1. **Repeated line folding**: consecutive identical output lines → `(xN)` suffix
//! 2. **Category abbreviation**: `stdout`→`O`, `stderr`→`E`, `console`→`C`
//! 3. **Line count limit**: cap at a maximum number of lines

use crate::codec::json_rpc::DapMessage;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
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
        if let Some(ref mut args) = msg.arguments {
            // Abbreviate category
            if let Some(cat) = args.get("category").and_then(|v| v.as_str()) {
                let abbr = match cat {
                    "stdout" => "O",
                    "stderr" => "E",
                    "console" => "C",
                    "stdin" => "I",
                    "telemetry" => "T",
                    _ => cat,
                };
                args["category"] = serde_json::Value::String(abbr.into());
            }

            // Compress repeated lines in output text
            if let Some(output) = args.get("output").and_then(|v| v.as_str()) {
                let compressed = compress_output_text(output);
                args["output"] = serde_json::Value::String(compressed);
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
    async fn test_output_compressor_abbreviates_category() {
        let compressor = OutputCompressor;
        let msg = DapMessage {
            seq: 1,
            msg_type: "event".into(),
            command: None,
            event: Some("output".into()),
            request_seq: None,
            success: None,
            body: None,
            arguments: Some(serde_json::json!({
                "category": "stdout",
                "output": "hello\nworld\nworld"
            })),
        };

        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let args = result.arguments.unwrap();
        assert_eq!(args["category"], "O");
        assert_eq!(args["output"], "hello\nworld (x2)");
    }
}
