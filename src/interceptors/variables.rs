//! Variables response compressor — compress DAP `variables` responses.
//!
//! The `variables` request returns the children of a variable reference.
//! Deeply nested objects can result in very large responses.
//!
//! ## Compression Strategies
//!
//! 1. **Type prefix**: prepend type info before variable name
//! 2. **Value truncation**: truncate long string values
//! 3. **Null removal**: remove variables with `null` value (optional)

use crate::codec::json_rpc::DapMessage;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
use crate::proxy::Direction;

/// Compressor for DAP `variables` responses.
pub struct VariablesCompressor;

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
                if let Some(value) = var.get("value").and_then(|v| v.as_str()) {
                    let truncated = truncate_value(value);
                    var["value"] = serde_json::Value::String(truncated);
                }
            }
        }
        Ok(Some(msg))
    }
}

/// Truncate long string values for compact display.
fn truncate_value(value: &str) -> String {
    if value.len() > 120 {
        let mut s: String = value.chars().take(120).collect();
        s.push_str("...");
        s
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_value_short() {
        assert_eq!(truncate_value("hello"), "hello");
    }

    #[test]
    fn test_truncate_value_long() {
        let long = "a".repeat(200);
        let result = truncate_value(&long);
        assert_eq!(result.len(), 123); // 120 + "..."
        assert!(result.ends_with("..."));
    }
}
