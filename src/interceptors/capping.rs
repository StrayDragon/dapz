//! Capping interceptor — truncate large DAP responses before compression.
//!
//! Limits stack frames, variables, and output event text length
//! based on [`CappingConfig`](crate::config::CappingConfig).

use crate::codec::json_rpc::DapMessage;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
use crate::proxy::Direction;

/// Interceptor that truncates oversized DAP responses.
///
/// Operates on:
/// - `stackTrace` responses: limits number of frames
/// - `variables` responses: limits number of variables
/// - `output` events: limits text length
pub struct CappingInterceptor {
    max_frames: usize,
    max_variables: usize,
    max_output_length: usize,
}

impl CappingInterceptor {
    /// Create a new capping interceptor with the given limits.
    /// A value of `0` means unlimited.
    pub fn new(max_frames: usize, max_variables: usize, max_output_length: usize) -> Self {
        Self {
            max_frames,
            max_variables,
            max_output_length,
        }
    }
}

#[async_trait::async_trait]
impl Interceptor for CappingInterceptor {
    fn name(&self) -> &str {
        "capping"
    }

    fn applies_to(&self, msg: &DapMessage, direction: Direction) -> bool {
        if direction != Direction::ServerToClient {
            return false;
        }
        match msg.msg_type.as_str() {
            "response" => {
                matches!(
                    msg.command.as_deref(),
                    Some("stackTrace") | Some("variables")
                )
            }
            "event" => msg.event.as_deref() == Some("output"),
            _ => false,
        }
    }

    async fn intercept(
        &self,
        mut msg: DapMessage,
        _direction: Direction,
    ) -> Result<Option<DapMessage>, DapzError> {
        match msg.msg_type.as_str() {
            "response" => {
                if let Some(ref mut body) = msg.body {
                    match msg.command.as_deref() {
                        Some("stackTrace") if self.max_frames > 0 => {
                            if let Some(frames) =
                                body.get_mut("stackFrames").and_then(|v| v.as_array_mut())
                            {
                                frames.truncate(self.max_frames);
                            }
                        }
                        Some("variables") if self.max_variables > 0 => {
                            if let Some(vars) =
                                body.get_mut("variables").and_then(|v| v.as_array_mut())
                            {
                                vars.truncate(self.max_variables);
                            }
                        }
                        _ => {}
                    }
                }
            }
            "event" => {
                if self.max_output_length > 0
                    && let Some(ref mut args) = msg.arguments
                    && let Some(output) = args.get_mut("output").and_then(|v| v.as_str())
                {
                    let truncated = truncate_text(output, self.max_output_length);
                    args["output"] = serde_json::Value::String(truncated);
                }
            }
            _ => {}
        }
        Ok(Some(msg))
    }
}

/// Truncate text to at most `max_len` characters, adding "..." if truncated.
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        let mut s: String = text.chars().take(max_len).collect();
        s.push_str("...");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_text_short() {
        assert_eq!(truncate_text("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_text_long() {
        let result = truncate_text("hello world this is long", 10);
        assert!(result.starts_with("hello wor"));
        assert!(result.ends_with("..."));
    }

    #[tokio::test]
    async fn test_capping_frames() {
        let interceptor = CappingInterceptor::new(2, 0, 0);
        let frames: Vec<serde_json::Value> = (0..5)
            .map(|i| serde_json::json!({"id": i, "name": format!("frame_{i}")}))
            .collect();
        let msg = DapMessage {
            seq: 1,
            msg_type: "response".into(),
            command: Some("stackTrace".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: Some(serde_json::json!({"stackFrames": frames})),
            arguments: None,
        };

        let result = interceptor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let remaining = body["stackFrames"].as_array().unwrap();
        assert_eq!(remaining.len(), 2);
    }
}
