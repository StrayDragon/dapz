//! Variables response compressor — compress DAP `variables` responses.
//!
//! The `variables` request returns the children of a variable reference.
//! Deeply nested objects can result in very large responses with many
//! fields that are irrelevant to LLMs.
//!
//! ## Compression Strategies
//!
//! 1. **Type prefix**: prepend type before variable name (e.g. `String: name`)
//!    then remove the `type` field — saves one key-value pair per variable.
//! 2. **Array summarization**: large arrays → `Array[0..N]` instead of full content.
//! 3. **Value truncation**: cap string values at `max_value_length`.
//! 4. **Field pruning**: drop `memoryReference`, `declarationLocationReference`,
//!    `valueLocationReference` — LLM cannot use these.
//! 5. **Preserved**: `evaluateName`, `variablesReference`, `namedVariables`, `indexedVariables`.

use crate::codec::json_rpc::DapMessage;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
use crate::proxy::Direction;

/// Compressor for DAP `variables` responses.
///
/// A value of `0` for `max_value_length` means unlimited (no truncation).
pub struct VariablesCompressor {
    max_value_length: usize,
}

impl VariablesCompressor {
    /// Create a new variables compressor.
    ///
    /// `max_value_length` — max chars of each variable's `value` (0 = unlimited).
    pub fn new(max_value_length: usize) -> Self {
        Self { max_value_length }
    }
}

#[async_trait::async_trait]
impl Interceptor for VariablesCompressor {
    fn name(&self) -> &str {
        "variables_compressor"
    }

    fn applies_to(&self, msg: &DapMessage, direction: Direction) -> bool {
        direction == Direction::ServerToClient
            && msg.msg_type == "response"
            && msg.command.as_deref() == Some("variables")
    }

    async fn intercept(
        &self,
        mut msg: DapMessage,
        _direction: Direction,
    ) -> Result<Option<DapMessage>, DapzError> {
        if let Some(ref mut body) = msg.body
            && let Some(vars) = body.get_mut("variables").and_then(|v| v.as_array_mut())
        {
            for var in vars.iter_mut() {
                // 1. Type prefix: "type: name" → saves one field per variable
                if let (Some(type_name), Some(name)) = (
                    var.get("type").and_then(|v| v.as_str()),
                    var.get("name").and_then(|v| v.as_str()),
                ) && !type_name.is_empty()
                    && !name.starts_with(&format!("{type_name}: "))
                {
                    let prefixed = format!("{type_name}: {name}");
                    var["name"] = serde_json::Value::String(prefixed);
                    var.as_object_mut().map(|obj| obj.remove("type"));
                }

                // 2. Array summarization
                let is_large_array = var
                    .get("indexedVariables")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    > 0;
                if is_large_array
                    && let Some(value) = var.get("value").and_then(|v| v.as_str())
                    && value.starts_with('[')
                {
                    let count = var["indexedVariables"].as_u64().unwrap_or(0);
                    var["value"] = serde_json::Value::String(format!("Array[0..{})", count));
                    continue; // skip value truncation for summarized arrays
                }

                // 3. Value truncation
                if self.max_value_length > 0
                    && let Some(value) = var.get("value").and_then(|v| v.as_str())
                    && value.len() > self.max_value_length
                {
                    let truncated: String = value.chars().take(self.max_value_length).collect();
                    var["value"] = serde_json::Value::String(format!("{truncated}..."));
                }

                // 4. Field pruning (noise fields that LLM can't use)
                if let Some(obj) = var.as_object_mut() {
                    obj.remove("memoryReference");
                    obj.remove("declarationLocationReference");
                    obj.remove("valueLocationReference");
                }
            }
        }
        Ok(Some(msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vars_response(vars: serde_json::Value) -> DapMessage {
        DapMessage {
            seq: 1,
            msg_type: "response".into(),
            command: Some("variables".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: Some(serde_json::json!({ "variables": vars })),
            arguments: None,
        }
    }

    #[tokio::test]
    async fn test_variables_type_prefix() {
        let compressor = VariablesCompressor::new(120);
        let msg = make_vars_response(serde_json::json!([
            {"name": "x", "value": "42", "type": "int", "variablesReference": 0},
            {"name": "msg", "value": "\"hello\"", "type": "String", "variablesReference": 0},
        ]));
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let vars = body["variables"].as_array().unwrap();
        assert_eq!(vars[0]["name"], "int: x");
        assert!(!vars[0].as_object().unwrap().contains_key("type"));
        assert_eq!(vars[1]["name"], "String: msg");
        assert!(!vars[1].as_object().unwrap().contains_key("type"));
    }

    #[tokio::test]
    async fn test_variables_type_prefix_empty_type_skipped() {
        let compressor = VariablesCompressor::new(120);
        let msg = make_vars_response(serde_json::json!([
            {"name": "x", "value": "42", "type": "", "variablesReference": 0},
        ]));
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let vars = body["variables"].as_array().unwrap();
        assert_eq!(vars[0]["name"], "x"); // unchanged
    }

    #[tokio::test]
    async fn test_variables_array_summarization() {
        let compressor = VariablesCompressor::new(120);
        let msg = make_vars_response(serde_json::json!([
            {
                "name": "items",
                "value": "[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]",
                "type": "int[]",
                "variablesReference": 10,
                "indexedVariables": 10,
                "namedVariables": 0,
            },
        ]));
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let vars = body["variables"].as_array().unwrap();
        assert_eq!(vars[0]["value"], "Array[0..10)");
    }

    #[tokio::test]
    async fn test_variables_value_truncation() {
        let compressor = VariablesCompressor::new(10);
        let long_str = "a".repeat(100);
        let msg = make_vars_response(serde_json::json!([
            {"name": "x", "value": &long_str, "type": "string", "variablesReference": 0},
        ]));
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let value = body["variables"][0]["value"].as_str().unwrap();
        assert_eq!(value.len(), 13); // 10 + "..."
        assert!(value.ends_with("..."));
    }

    #[tokio::test]
    async fn test_variables_no_truncation_when_zero() {
        let compressor = VariablesCompressor::new(0);
        let long_str = "a".repeat(1000);
        let msg = make_vars_response(serde_json::json!([
            {"name": "x", "value": &long_str, "type": "string", "variablesReference": 0},
        ]));
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        assert_eq!(body["variables"][0]["value"].as_str().unwrap().len(), 1000);
    }

    #[tokio::test]
    async fn test_variables_field_pruning() {
        let compressor = VariablesCompressor::new(120);
        let msg = make_vars_response(serde_json::json!([
            {
                "name": "x",
                "value": "42",
                "type": "int",
                "variablesReference": 0,
                "evaluateName": "x",
                "memoryReference": "0x7fff00000010",
                "declarationLocationReference": 99,
                "valueLocationReference": 100,
            },
        ]));
        let result = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let body = result.body.unwrap();
        let var = &body["variables"][0];
        let obj = var.as_object().unwrap();
        assert!(!obj.contains_key("memoryReference"));
        assert!(!obj.contains_key("declarationLocationReference"));
        assert!(!obj.contains_key("valueLocationReference"));
        // preserve essential
        assert!(obj.contains_key("evaluateName"));
        assert!(obj.contains_key("variablesReference"));
    }

    #[tokio::test]
    async fn test_applies_to_variables_response() {
        let compressor = VariablesCompressor::new(120);
        let msg = DapMessage {
            seq: 1,
            msg_type: "response".into(),
            command: Some("variables".into()),
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
        let compressor = VariablesCompressor::new(120);
        for cmd in &["stackTrace", "scopes", "evaluate"] {
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
            assert!(!compressor.applies_to(&msg, Direction::ServerToClient));
        }
    }
}
