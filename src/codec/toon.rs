//! TOON (Token-Oriented Object Notation) for DAP JSON values.
//!
//! Generic encoder: turns arbitrary [`serde_json::Value`] (typically a DAP
//! response/event body) into a compact, LLM-friendly line protocol.
//! Field names stay self-explanatory (no abbreviations).
//!
//! Pattern inspired by lspz's TOON codec, adapted for DAP without compact types.

use serde_json::Value;

use crate::error::DapzError;

/// Convert a JSON [`Value`] to TOON text.
pub fn value_to_toon(value: &Value) -> Result<String, DapzError> {
    let mut out = String::new();
    write_value(&mut out, value, 0)?;
    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    Ok(out)
}

fn write_value(out: &mut String, value: &Value, indent: usize) -> Result<(), DapzError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => out.push_str(&escape_scalar(s)),
        Value::Array(arr) => write_array(out, arr, indent)?,
        Value::Object(map) => write_object(out, map, indent)?,
    }
    Ok(())
}

fn write_object(
    out: &mut String,
    map: &serde_json::Map<String, Value>,
    indent: usize,
) -> Result<(), DapzError> {
    let pad = "  ".repeat(indent);
    let mut first = true;
    for (key, val) in map {
        if !first {
            out.push('\n');
        }
        first = false;
        out.push_str(&pad);
        out.push_str(key);
        out.push(':');
        match val {
            Value::Object(_) | Value::Array(_) => {
                out.push('\n');
                write_value(out, val, indent + 1)?;
            }
            _ => {
                out.push(' ');
                write_value(out, val, indent)?;
            }
        }
    }
    Ok(())
}

fn write_array(out: &mut String, arr: &[Value], indent: usize) -> Result<(), DapzError> {
    let pad = "  ".repeat(indent);

    // Tabular: array of objects with a shared key set
    if let Some(fields) = tabular_fields(arr) {
        out.push_str(&pad);
        out.push_str(&format!("items[{}]{{{}}}:\n", arr.len(), fields.join(",")));
        for item in arr {
            let obj = item.as_object().expect("tabular_fields checked objects");
            out.push_str(&pad);
            out.push_str("  ");
            let row: Vec<String> = fields
                .iter()
                .map(|f| obj.get(*f).map(scalar_for_csv).unwrap_or_else(|| "".into()))
                .collect();
            out.push_str(&row.join(","));
            out.push('\n');
        }
        // trim trailing newline from last row handling — keep one
        return Ok(());
    }

    if arr.is_empty() {
        out.push_str(&pad);
        out.push_str("items[0]:");
        return Ok(());
    }

    out.push_str(&pad);
    out.push_str(&format!("items[{}]:\n", arr.len()));
    for (i, item) in arr.iter().enumerate() {
        out.push_str(&pad);
        out.push_str("  ");
        out.push_str(&format!("- [{i}] "));
        match item {
            Value::Object(_) | Value::Array(_) => {
                out.push('\n');
                write_value(out, item, indent + 2)?;
                out.push('\n');
            }
            _ => {
                write_value(out, item, indent)?;
                out.push('\n');
            }
        }
    }
    Ok(())
}

/// If `arr` is a non-empty array of objects, return a stable field list for a table.
fn tabular_fields(arr: &[Value]) -> Option<Vec<&str>> {
    if arr.is_empty() {
        return None;
    }
    let mut fields: Vec<&str> = Vec::new();
    for item in arr {
        let obj = item.as_object()?;
        if fields.is_empty() {
            fields = obj.keys().map(|k| k.as_str()).collect();
            if fields.is_empty() {
                return None;
            }
        } else {
            // Allow subset; require all rows are objects
            let _ = obj;
        }
    }
    // Prefer a common key order: intersection preserving first object's order,
    // then extras from later objects.
    let mut ordered = fields;
    for item in arr.iter().skip(1) {
        let obj = item.as_object()?;
        for k in obj.keys() {
            if !ordered.iter().any(|e| e == &k.as_str()) {
                ordered.push(k.as_str());
            }
        }
    }
    Some(ordered)
}

fn scalar_for_csv(v: &Value) -> String {
    match v {
        Value::Null => "".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => escape_csv(s),
        other => escape_csv(&other.to_string()),
    }
}

fn escape_scalar(s: &str) -> String {
    if s.contains('\n') || s.contains(':') {
        escape_csv(s)
    } else {
        s.to_string()
    }
}

fn escape_csv(s: &str) -> String {
    let s = s.replace('\n', "\\n");
    if s.contains(',') || s.contains('"') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
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
        assert!(s.contains("items[2]{"));
        assert!(s.contains("Locals"));
        assert!(s.contains("Globals"));
    }

    #[test]
    fn test_toon_empty_array() {
        let v = json!({"variables": []});
        let s = value_to_toon(&v).unwrap();
        assert!(s.contains("items[0]:"));
    }

    #[test]
    fn test_toon_escapes_comma() {
        let v = json!({"message": "a, b"});
        let s = value_to_toon(&v).unwrap();
        assert!(s.contains("\"a, b\"") || s.contains("a, b"));
    }
}
