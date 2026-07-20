//! ExceptionInfo response compressor — DAP `exceptionInfo` responses.
//!
//! Unlike LSP diagnostics merging, this targets long exception `details`
//! / stack text that burns tokens for Agents using debugpy (and peers).

use crate::codec::json_rpc::DapMessage;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
use crate::proxy::Direction;

/// Compressor for DAP `exceptionInfo` responses.
pub struct ExceptionInfoCompressor {
    max_details_length: usize,
}

impl ExceptionInfoCompressor {
    /// Create a compressor; `0` means no truncation.
    pub fn new(max_details_length: usize) -> Self {
        Self { max_details_length }
    }
}

impl Default for ExceptionInfoCompressor {
    fn default() -> Self {
        Self::new(800)
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    let truncated: String = s.chars().take(max).collect();
    format!("{truncated}...")
}

#[async_trait::async_trait]
impl Interceptor for ExceptionInfoCompressor {
    fn name(&self) -> &str {
        "exception_info_compressor"
    }

    fn applies_to(&self, msg: &DapMessage, direction: Direction) -> bool {
        direction == Direction::ServerToClient
            && msg.msg_type == "response"
            && msg.command.as_deref() == Some("exceptionInfo")
    }

    async fn intercept(
        &self,
        mut msg: DapMessage,
        _direction: Direction,
    ) -> Result<Option<DapMessage>, DapzError> {
        if self.max_details_length == 0 {
            return Ok(Some(msg));
        }
        if let Some(body) = msg.body.as_mut() {
            // Top-level description
            if let Some(desc) = body.get("description").and_then(|v| v.as_str())
                && desc.chars().count() > self.max_details_length
            {
                body["description"] =
                    serde_json::Value::String(truncate_chars(desc, self.max_details_length));
            }
            // Nested details.message / stackTrace
            if let Some(details) = body.get_mut("details").and_then(|d| d.as_object_mut()) {
                for key in ["message", "stackTrace", "typeName"] {
                    if let Some(s) = details.get(key).and_then(|v| v.as_str())
                        && s.chars().count() > self.max_details_length
                    {
                        details.insert(
                            key.into(),
                            serde_json::Value::String(truncate_chars(s, self.max_details_length)),
                        );
                    }
                }
            }
        }
        Ok(Some(msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_exception_info_truncates_details() {
        let long = "E".repeat(2000);
        let msg = DapMessage {
            seq: 2,
            msg_type: "response".into(),
            command: Some("exceptionInfo".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: Some(json!({
                "exceptionId": "ZeroDivisionError",
                "description": long.clone(),
                "breakMode": "unhandled",
                "details": {
                    "message": long.clone(),
                    "typeName": "ZeroDivisionError"
                }
            })),
            arguments: None,
        };
        let out = ExceptionInfoCompressor::new(100)
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let desc = out.body.as_ref().unwrap()["description"].as_str().unwrap();
        assert!(desc.chars().count() <= 103); // truncate may add ellipsis
        assert_eq!(
            out.body.as_ref().unwrap()["exceptionId"].as_str().unwrap(),
            "ZeroDivisionError"
        );
    }
}
