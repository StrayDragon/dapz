//! StackTrace response compressor — compress DAP `stackTrace` responses.
//!
//! Stack traces can be very deep (100+ frames), consuming significant tokens.
//!
//! ## Compression Strategies
//!
//! 1. **Synthetic frame filtering**: remove frames with `presentationHint: "label"`
//!    or `"subtle"` — these are synthetic frames with no meaningful source location.
//! 2. **Source deduplication**: consecutive frames from the same file get an empty
//!    `path` (repeating the same path wastes tokens).
//! 3. **Path abbreviation**: shorten source paths to `dir/filename`.
//! 4. **Function name trimming**: strip parameters from function names.
//! 5. **Field pruning**: remove `instructionPointerReference` and `moduleId`
//!    (raw addresses and module IDs are noise for LLM).

use crate::codec::json_rpc::DapMessage;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
use crate::interceptors::utils::shorten_path_display;
use crate::proxy::Direction;

/// Compressor for DAP `stackTrace` responses.
pub struct StackTraceCompressor;

#[async_trait::async_trait]
impl Interceptor for StackTraceCompressor {
    fn name(&self) -> &str {
        "stacktrace_compressor"
    }

    fn applies_to(&self, msg: &DapMessage, direction: Direction) -> bool {
        direction == Direction::ServerToClient
            && msg.msg_type == "response"
            && msg.command.as_deref() == Some("stackTrace")
    }

    async fn intercept(
        &self,
        mut msg: DapMessage,
        _direction: Direction,
    ) -> Result<Option<DapMessage>, DapzError> {
        if let Some(ref mut body) = msg.body
            && let Some(frames) = body.get_mut("stackFrames").and_then(|v| v.as_array_mut())
        {
            // 1. Filter out synthetic frames (label/subtle presentation hint)
            frames.retain(|frame| {
                let hint = frame.get("presentationHint").and_then(|v| v.as_str());
                !matches!(hint, Some("label") | Some("subtle"))
            });

            // Track previous frame's source path for dedup
            let mut prev_source_path: Option<String> = None;

            for frame in frames.iter_mut() {
                // 2. Source path abbreviation + dedup
                if let Some(src_obj) = frame.get_mut("source").and_then(|v| v.as_object_mut()) {
                    let path_owned = src_obj
                        .get("path")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    if let Some(ref path) = path_owned {
                        let is_dup = prev_source_path.as_deref() == Some(path.as_str());
                        if is_dup {
                            src_obj.insert("path".into(), serde_json::Value::String(String::new()));
                        } else {
                            let shortened = shorten_path_display(path);
                            src_obj.insert("path".into(), serde_json::Value::String(shortened));
                        }
                        prev_source_path = Some(path.clone());
                    }
                }

                // 3. Trim function name — strip parameters
                if let Some(name) = frame.get("name").and_then(|v| v.as_str()) {
                    let trimmed = trim_function_name(name);
                    frame["name"] = serde_json::Value::String(trimmed);
                }

                // 4. Prune noise fields
                if let Some(obj) = frame.as_object_mut() {
                    obj.remove("instructionPointerReference");
                    obj.remove("moduleId");
                }
            }
        }
        Ok(Some(msg))
    }
}

/// Trim function name: keep only the function/method name, drop parameters.
/// `foo(a: i32, b: String)` → `foo(...)`
fn trim_function_name(name: &str) -> String {
    if let Some(paren) = name.find('(') {
        let base = &name[..paren].trim();
        if !base.is_empty() {
            return format!("{base}(...)");
        }
    }
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_frame(name: &str, path: &str, hint: Option<&str>) -> serde_json::Value {
        let mut frame = serde_json::json!({
            "id": 0,
            "name": name,
            "source": {
                "path": path,
                "name": path.rsplit('/').next().unwrap_or(path),
            },
            "line": 1,
            "column": 1,
            "instructionPointerReference": "0x7ffff7a3d8af",
            "moduleId": 42,
        });
        if let Some(h) = hint {
            frame["presentationHint"] = serde_json::Value::String(h.into());
        }
        frame
    }

    #[tokio::test]
    async fn test_stacktrace_path_abbreviation() {
        let compressor = StackTraceCompressor;
        let frames =
            serde_json::json!([make_frame("main", "/home/user/project/src/main.rs", None)]);
        let msg = DapMessage {
            seq: 1,
            msg_type: "response".into(),
            command: Some("stackTrace".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: Some(serde_json::json!({ "stackFrames": frames })),
            arguments: None,
        };
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let frame = &body["stackFrames"][0];
        assert_eq!(frame["source"]["path"], "src/main.rs");
    }

    #[tokio::test]
    async fn test_stacktrace_filters_label_frames() {
        let compressor = StackTraceCompressor;
        let frames = serde_json::json!([
            make_frame("real_func", "/home/user/src/main.rs", None),
            make_frame("label_frame", "/home/user/src/main.rs", Some("label")),
            make_frame("subtle_frame", "/home/user/src/main.rs", Some("subtle")),
        ]);
        let msg = DapMessage {
            seq: 1,
            msg_type: "response".into(),
            command: Some("stackTrace".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: Some(serde_json::json!({ "stackFrames": frames })),
            arguments: None,
        };
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let remaining = body["stackFrames"].as_array().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0]["name"], "real_func");
    }

    #[tokio::test]
    async fn test_stacktrace_source_dedup() {
        let compressor = StackTraceCompressor;
        let frames = serde_json::json!([
            make_frame("func_a", "/home/user/src/main.rs", None),
            make_frame("func_b", "/home/user/src/main.rs", None),
            make_frame("func_c", "/home/user/src/other.rs", None),
        ]);
        let msg = DapMessage {
            seq: 1,
            msg_type: "response".into(),
            command: Some("stackTrace".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: Some(serde_json::json!({ "stackFrames": frames })),
            arguments: None,
        };
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let remaining = body["stackFrames"].as_array().unwrap();
        // First frame: shortened path
        assert_eq!(remaining[0]["source"]["path"], "src/main.rs");
        // Second frame (same file): empty path (dedup marker)
        assert_eq!(remaining[1]["source"]["path"], "");
        // Third frame (different file): new path
        assert_eq!(remaining[2]["source"]["path"], "src/other.rs");
    }

    #[tokio::test]
    async fn test_stacktrace_prunes_noise_fields() {
        let compressor = StackTraceCompressor;
        let frames = serde_json::json!([make_frame("main", "/home/user/main.rs", None)]);
        let msg = DapMessage {
            seq: 1,
            msg_type: "response".into(),
            command: Some("stackTrace".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: Some(serde_json::json!({ "stackFrames": frames })),
            arguments: None,
        };
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let frame = &body["stackFrames"][0];
        let obj = frame.as_object().unwrap();
        assert!(!obj.contains_key("instructionPointerReference"));
        assert!(!obj.contains_key("moduleId"));
    }

    #[test]
    fn test_trim_function_name_with_params() {
        assert_eq!(trim_function_name("my_func(a, b)"), "my_func(...)");
        assert_eq!(trim_function_name("simple"), "simple");
        assert_eq!(trim_function_name(""), "");
    }

    #[tokio::test]
    async fn test_applies_to_stacktrace_response() {
        let compressor = StackTraceCompressor;
        let msg = DapMessage {
            seq: 1,
            msg_type: "response".into(),
            command: Some("stackTrace".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: None,
            arguments: None,
        };
        assert!(compressor.applies_to(&msg, Direction::ServerToClient));
        assert!(!compressor.applies_to(&msg, Direction::ClientToServer));
    }
}
