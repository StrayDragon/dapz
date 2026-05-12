//! StackTrace response compressor — compress DAP `stackTrace` responses.
//!
//! Stack traces can be very deep (100+ frames), consuming significant tokens.
//!
//! ## Compression Strategies
//!
//! 1. **Path abbreviation**: shorten source paths to just filename
//! 2. **Parameter removal**: strip function arguments from frame names
//! 3. **Module prefix trimming**: remove common module prefixes

use crate::codec::json_rpc::DapMessage;
use crate::error::DapzError;
use crate::interceptors::Interceptor;
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
            for frame in frames.iter_mut() {
                if let Some(source) = frame.get_mut("source")
                    && let Some(path) = source.get("path").and_then(|v| v.as_str())
                    && let Ok(Some(filename)) = filename_from_path(path)
                {
                    source["path"] = serde_json::Value::String(shorten_path(path, &filename));
                }

                if let Some(name) = frame.get("name").and_then(|v| v.as_str()) {
                    let trimmed = trim_function_name(name);
                    frame["name"] = serde_json::Value::String(trimmed);
                }
            }
        }
        Ok(Some(msg))
    }
}

/// Extract filename from a path string.
fn filename_from_path(path: &str) -> Result<Option<String>, std::convert::Infallible> {
    if let Some(name) = path.rsplit('/').next() {
        Ok(Some(name.to_string()))
    } else if let Some(name) = path.rsplit('\\').next() {
        Ok(Some(name.to_string()))
    } else {
        Ok(Some(path.to_string()))
    }
}

/// Shorten a path to `dir/filename` or `.../dir/filename`.
fn shorten_path(path: &str, filename: &str) -> String {
    // If the path has a parent directory, try dir/filename
    let parent = path
        .trim_end_matches(filename)
        .trim_end_matches('/')
        .trim_end_matches('\\');

    if parent.is_empty() {
        return filename.to_string();
    }

    // Get the last directory component
    if let Some(dir) = parent.rsplit('/').next() {
        format!("{dir}/{filename}")
    } else if let Some(dir) = parent.rsplit('\\').next() {
        format!("{dir}/{filename}")
    } else {
        format!("{parent}/{filename}")
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

    #[test]
    fn test_shorten_path() {
        let path = "/home/user/project/src/main.rs";
        let filename = "main.rs";
        assert_eq!(shorten_path(path, filename), "src/main.rs");
    }

    #[test]
    fn test_trim_function_name() {
        assert_eq!(trim_function_name("my_func(a, b)"), "my_func(...)");
        assert_eq!(trim_function_name("simple"), "simple");
    }

    #[test]
    fn test_filename_from_path_unix() {
        let (path, filename) = (
            "/home/user/src/main.rs",
            filename_from_path("/home/user/src/main.rs")
                .unwrap()
                .unwrap(),
        );
        assert_eq!(filename, "main.rs");
        assert_eq!(shorten_path(path, &filename), "src/main.rs");
    }
}
