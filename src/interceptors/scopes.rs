//! Scopes response compressor — compress DAP `scopes` responses.
//!
//! Keeps fields agents need to walk variables (`name`, `variablesReference`,
//! `expensive`, optional `presentationHint`) and drops source location noise.

use crate::codec::json_rpc::DapMessage;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
use crate::proxy::Direction;

/// Compressor for DAP `scopes` responses.
pub struct ScopesCompressor;

#[async_trait::async_trait]
impl Interceptor for ScopesCompressor {
    fn name(&self) -> &str {
        "scopes_compressor"
    }

    fn applies_to(&self, msg: &DapMessage, direction: Direction) -> bool {
        direction == Direction::ServerToClient
            && msg.msg_type == "response"
            && msg.command.as_deref() == Some("scopes")
    }

    async fn intercept(
        &self,
        mut msg: DapMessage,
        _direction: Direction,
    ) -> Result<Option<DapMessage>, DapzError> {
        if let Some(ref mut body) = msg.body
            && let Some(scopes) = body.get_mut("scopes").and_then(|v| v.as_array_mut())
        {
            for scope in scopes.iter_mut() {
                if let Some(obj) = scope.as_object_mut() {
                    obj.remove("source");
                    obj.remove("line");
                    obj.remove("column");
                    obj.remove("endLine");
                    obj.remove("endColumn");
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

    fn scopes_response() -> DapMessage {
        DapMessage {
            seq: 2,
            msg_type: "response".into(),
            command: Some("scopes".into()),
            event: None,
            request_seq: Some(1),
            success: Some(true),
            body: Some(json!({
                "scopes": [{
                    "name": "Locals",
                    "variablesReference": 100,
                    "expensive": false,
                    "source": {"path": "/tmp/a.py"},
                    "line": 10,
                    "column": 0,
                    "endLine": 20,
                    "endColumn": 1,
                    "presentationHint": "locals"
                }]
            })),
            arguments: None,
        }
    }

    #[tokio::test]
    async fn test_scopes_strips_location_fields() {
        let compressor = ScopesCompressor;
        let msg = scopes_response();
        assert!(compressor.applies_to(&msg, Direction::ServerToClient));
        let out = compressor
            .intercept(msg, Direction::ServerToClient)
            .await
            .unwrap()
            .unwrap();
        let scope = &out.body.as_ref().unwrap()["scopes"][0];
        assert_eq!(scope["name"], "Locals");
        assert_eq!(scope["variablesReference"], 100);
        assert!(scope.get("source").is_none());
        assert!(scope.get("line").is_none());
        assert_eq!(scope["presentationHint"], "locals");
    }

    #[tokio::test]
    async fn test_scopes_does_not_apply_to_variables() {
        let compressor = ScopesCompressor;
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
        assert!(!compressor.applies_to(&msg, Direction::ServerToClient));
    }
}
