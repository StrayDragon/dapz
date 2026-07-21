//! TOON (Token-Oriented Object Notation) for DAP JSON values.
//!
//! Encodes arbitrary [`serde_json::Value`] (typically a compressed DAP
//! response/event body) via the official [`toon_format`] crate.
//!
//! See <https://github.com/toon-format/toon> for the TOON specification.

use serde_json::Value;
use toon_format::encode_default;

use crate::error::DapzError;

/// Convert a JSON [`Value`] to TOON text using `toon-format`.
pub fn value_to_toon(value: &Value) -> Result<String, DapzError> {
    encode_default(value).map_err(|e| DapzError::Protocol(format!("TOON encode failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_toon_simple_object() {
        let v = json!({"threadId": 1, "reason": "breakpoint"});
        let s = value_to_toon(&v).unwrap();
        assert!(s.contains("threadId: 1"));
        assert!(s.contains("reason: breakpoint"));
    }

    #[test]
    fn test_toon_tabular_scopes() {
        let v = json!({
            "scopes": [
                {"name": "Locals", "variablesReference": 100, "expensive": false},
                {"name": "Globals", "variablesReference": 200, "expensive": true}
            ]
        });
        let s = value_to_toon(&v).unwrap();
        assert!(s.contains("scopes[2]{") || s.contains("items[2]{"));
        assert!(s.contains("Locals"));
        assert!(s.contains("Globals"));
    }

    #[test]
    fn test_toon_empty_array() {
        let v = json!({"variables": []});
        let s = value_to_toon(&v).unwrap();
        assert!(s.contains("variables[0]:") || s.contains("items[0]:") || s.contains("[]"));
    }

    #[test]
    fn test_toon_escapes_comma() {
        let v = json!({"message": "a, b"});
        let s = value_to_toon(&v).unwrap();
        assert!(s.contains("\"a, b\"") || s.contains("a, b") || s.contains("a\\, b"));
    }
}
