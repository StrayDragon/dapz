//! Evaluate response compressor — compress DAP `evaluate` responses.
//!
//! The `evaluate` request evaluates an expression in the debuggee context.
//! Results can contain very long strings (HTML/JSON dumps, stack traces, etc.)
//! and metadata fields irrelevant to LLMs.
//!
//! ## Compression Strategies
//!
//! 1. **Result truncation**: cap result strings at `max_result_length`
//! 2. **Field pruning**: remove `memoryReference` (LLM cannot use raw memory addresses)
//! 3. **Preserved**: `type`, `variablesReference`, `namedVariables`, `indexedVariables`,
//!    `presentationHint` — all essential for the LLM to understand the result structure

use crate::codec::json_rpc::DapMessage;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
use crate::proxy::Direction;

/// Compressor for DAP `evaluate` responses.
///
/// A value of `0` for `max_result_length` means unlimited (no truncation).
pub struct EvaluateCompressor {
    max_result_length: usize,
}

impl EvaluateCompressor {
    /// Create a new evaluate compressor.
    ///
    /// `max_result_length` — max chars of the `result` string (0 = unlimited).
    pub fn new(max_result_length: usize) -> Self {
        Self { max_result_length }
    }
}

#[async_trait::async_trait]
impl Interceptor for EvaluateCompressor {
    fn name(&self) -> &str {
        "evaluate_compressor"
    }

    fn applies_to(&self, msg: &DapMessage, direction: Direction) -> bool {
        direction == Direction::ServerToClient
            && msg.msg_type == "response"
            && msg.command.as_deref() == Some("evaluate")
    }

    async fn intercept(
        &self,
        mut msg: DapMessage,
        _direction: Direction,
    ) -> Result<Option<DapMessage>, DapzError> {
        if let Some(ref mut body) = msg.body {
            // Truncate result string beyond the configured limit
            if self.max_result_length > 0
                && let Some(result) = body
                    .get("result")
                    .and_then(|v| v.as_str())
                    .filter(|r| r.len() > self.max_result_length)
            {
                let truncated: String = result.chars().take(self.max_result_length).collect();
                body["result"] = serde_json::Value::String(format!("{truncated}..."));
            }

            // Remove memoryReference — raw addresses are noise for LLM
            if let Some(obj) = body.as_object_mut() {
                obj.remove("memoryReference");
            }

            // Preserved (not removed):
            // - type: essential to understand the result kind
            // - variablesReference: needed to fetch children
            // - namedVariables / indexedVariables: structure hints
            // - presentationHint: small, useful for UI context
        }
        Ok(Some(msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_eval_response(result: serde_json::Value) -> DapMessage {
        DapMessage {
            seq: 1,
            msg_type: "response".into(),
            command: Some("evaluate".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: Some(result),
            arguments: None,
        }
    }

    #[tokio::test]
    async fn test_evaluate_no_truncation_when_zero() {
        let compressor = EvaluateCompressor::new(0);
        let msg = make_eval_response(serde_json::json!({
            "result": "a long string that stays intact",
            "type": "string",
            "variablesReference": 0,
        }));
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        assert_eq!(body["result"], "a long string that stays intact");
    }

    #[tokio::test]
    async fn test_evaluate_truncates_long_result() {
        let compressor = EvaluateCompressor::new(10);
        let long_str = "a".repeat(100);
        let msg = make_eval_response(serde_json::json!({
            "result": &long_str,
            "type": "string",
            "variablesReference": 0,
        }));
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let result_str = body["result"].as_str().unwrap();
        assert_eq!(result_str.len(), 13); // 10 + "..."
        assert!(result_str.ends_with("..."));
        assert!(result_str.starts_with("aaaaaaaaaa"));
    }

    #[tokio::test]
    async fn test_evaluate_short_result_not_truncated() {
        let compressor = EvaluateCompressor::new(500);
        let msg = make_eval_response(serde_json::json!({
            "result": "42",
            "type": "int",
            "variablesReference": 0,
        }));
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        assert_eq!(body["result"], "42");
    }

    #[tokio::test]
    async fn test_evaluate_removes_memory_reference() {
        let compressor = EvaluateCompressor::new(500);
        let msg = make_eval_response(serde_json::json!({
            "result": "x",
            "type": "int",
            "variablesReference": 0,
            "memoryReference": "0x7fff00000010"
        }));
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        assert!(!body.as_object().unwrap().contains_key("memoryReference"));
    }

    #[tokio::test]
    async fn test_evaluate_preserves_essential_fields() {
        let compressor = EvaluateCompressor::new(500);
        let msg = make_eval_response(serde_json::json!({
            "result": "obj",
            "type": "object",
            "variablesReference": 42,
            "namedVariables": 3,
            "indexedVariables": 5,
            "presentationHint": {"kind": "data"}
        }));
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        assert_eq!(body["type"], "object");
        assert_eq!(body["variablesReference"], 42);
        assert_eq!(body["namedVariables"], 3);
        assert_eq!(body["indexedVariables"], 5);
        assert!(body.get("presentationHint").is_some());
    }

    #[tokio::test]
    async fn test_applies_to_evaluate_response() {
        let compressor = EvaluateCompressor::new(500);
        let msg = DapMessage {
            seq: 1,
            msg_type: "response".into(),
            command: Some("evaluate".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: None,
            arguments: None,
        };
        assert!(compressor.applies_to(&msg, Direction::ServerToClient));
        assert!(!compressor.applies_to(&msg, Direction::ClientToServer));
    }

    #[tokio::test]
    async fn test_does_not_apply_to_other_commands() {
        let compressor = EvaluateCompressor::new(500);
        for cmd in &["variables", "stackTrace", "scopes", "setVariable"] {
            let msg = DapMessage {
                seq: 1,
                msg_type: "response".into(),
                command: Some(cmd.to_string()),
                event: None,
                request_seq: Some(1),
                success: Some(true),
                body: None,
                arguments: None,
            };
            assert!(
                !compressor.applies_to(&msg, Direction::ServerToClient),
                "should not apply to {cmd}"
            );
        }
    }
}
