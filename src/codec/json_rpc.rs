//! JSON-RPC 2.0 frame parsing and serialization for DAP protocol.
//!
//! DAP uses the same `Content-Length` framing as LSP:
//!
//! ```text
//! Content-Length: 123\r\n
//! \r\n
//! {"seq":1,"type":"request","command":"launch",...}
//! ```
//!
//! ## Frame Format
//!
//! [MermaidChart:../docs/mmd/json-rpc-frame.mmd]

use crate::error::DapzError;

/// A parsed LSP-style frame with headers and body.
#[derive(Debug)]
pub struct Frame {
    /// Raw header bytes (including trailing \r\n).
    pub header: Vec<u8>,
    /// The JSON body bytes.
    pub body: Vec<u8>,
}

/// Parse a `Content-Length` framed message from raw bytes.
///
/// Returns `Ok(Some((frame, consumed_bytes)))` on success,
/// `Ok(None)` if more data is needed, or `Err` on parse error.
pub fn parse_frame(raw: &[u8]) -> Result<Option<(Frame, usize)>, DapzError> {
    let raw_str = std::str::from_utf8(raw)
        .map_err(|_| DapzError::Protocol("non-UTF8 in frame header".into()))?;

    // Find the double CRLF (or just \n\n) that separates headers from body
    let header_end = raw_str
        .find("\r\n\r\n")
        .or_else(|| raw_str.find("\n\n"))
        .ok_or(DapzError::Protocol("missing header terminator".into()))?;

    let header_part = &raw_str[..header_end];
    // The body starts after the blank line
    let body_start = if raw_str[header_end..].starts_with("\r\n\r\n") {
        header_end + 4
    } else {
        header_end + 2
    };

    // Parse Content-Length
    let content_length = parse_content_length(header_part)?;

    // Check if we have enough data
    let total_len = body_start + content_length as usize;
    if raw.len() < total_len {
        return Ok(None); // Need more data
    }

    let body = raw[body_start..total_len].to_vec();
    let header = raw[..body_start].to_vec();

    Ok(Some((Frame { header, body }, total_len)))
}

/// Serialize a JSON value into a Content-Length framed message.
pub fn serialize_frame(value: &serde_json::Value) -> Result<Vec<u8>, DapzError> {
    let body = serde_json::to_string(value)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut bytes = header.into_bytes();
    bytes.extend_from_slice(body.as_bytes());
    Ok(bytes)
}

/// Parse Content-Length from header text.
pub fn parse_content_length(header: &str) -> Result<u64, DapzError> {
    for line in header.lines() {
        let line = line.trim();
        if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
            let value = value.trim();
            return value
                .parse::<u64>()
                .map_err(|_| DapzError::Protocol(format!("invalid Content-Length: {value}")));
        }
    }
    Err(DapzError::Protocol("missing Content-Length header".into()))
}

/// Represents a parsed DAP protocol message.
///
/// DAP messages have `type`, `seq`, and type-specific fields.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DapMessage {
    /// Sequence number (unique per session).
    pub seq: i64,
    /// Message type: "request", "response", or "event".
    #[serde(rename = "type")]
    pub msg_type: String,
    /// For requests: the DAP command name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// For events: the event name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// For responses: the request sequence this responds to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_seq: Option<i64>,
    /// For responses: success flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    /// For responses: command being responded to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    /// For events: event body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

impl DapMessage {
    /// Parse a DAP message from a JSON-RPC framed byte slice.
    pub fn from_frame(bytes: &[u8]) -> Result<Self, DapzError> {
        let (frame, _) =
            parse_frame(bytes)?.ok_or_else(|| DapzError::Protocol("incomplete frame".into()))?;
        let msg: DapMessage = serde_json::from_slice(&frame.body)?;
        Ok(msg)
    }

    /// Serialize this message to framed bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, DapzError> {
        let value = serde_json::to_value(self)?;
        serialize_frame(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_length() {
        let header = "Content-Length: 42\r\n";
        assert_eq!(parse_content_length(header).unwrap(), 42);
    }

    #[test]
    fn test_missing_content_length() {
        assert!(parse_content_length("").is_err());
    }

    #[test]
    fn test_invalid_content_length_value() {
        let header = "Content-Length: abc\r\n";
        assert!(parse_content_length(header).is_err());
    }

    #[test]
    fn test_roundtrip() {
        let msg = DapMessage {
            seq: 1,
            msg_type: "request".into(),
            command: Some("initialize".into()),
            event: None,
            request_seq: None,
            success: None,
            body: None,
            arguments: None,
        };
        let bytes = msg.to_bytes().unwrap();
        let parsed = DapMessage::from_frame(&bytes).unwrap();
        assert_eq!(parsed.seq, 1);
        assert_eq!(parsed.msg_type, "request");
        assert_eq!(parsed.command.as_deref(), Some("initialize"));
    }

    #[test]
    fn test_event_roundtrip() {
        let msg = DapMessage {
            seq: 2,
            msg_type: "event".into(),
            command: None,
            event: Some("output".into()),
            request_seq: None,
            success: None,
            body: Some(serde_json::json!({
                "category": "stdout",
                "output": "hello world",
            })),
            arguments: None,
        };
        let bytes = msg.to_bytes().unwrap();
        let parsed = DapMessage::from_frame(&bytes).unwrap();
        assert_eq!(parsed.event.as_deref(), Some("output"));
        assert!(parsed.body.is_some());
        assert_eq!(parsed.body.unwrap()["output"], "hello world");
    }
}
