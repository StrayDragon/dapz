//! dapz Compression Demo
//!
//! Quick verification of all interceptors with sample data.
//! Measures token and byte savings for each interceptor.
//!
//! Run: cargo run --example compress-demo

use tiktoken_rs::cl100k_base;

use dapz::codec::json_rpc::DapMessage;
use dapz::interceptors::Interceptor;
use dapz::interceptors::capping::CappingInterceptor;
use dapz::interceptors::evaluate::EvaluateCompressor;
use dapz::interceptors::output::OutputCompressor;
use dapz::interceptors::stacktrace::StackTraceCompressor;
use dapz::interceptors::variables::VariablesCompressor;
use dapz::proxy::Direction;

fn count_tokens(bpe: &tiktoken_rs::CoreBPE, msg: &DapMessage) -> usize {
    let json = serde_json::to_string(msg).unwrap();
    bpe.encode_with_special_tokens(&json).len()
}

fn demo_interceptor(
    bpe: &tiktoken_rs::CoreBPE,
    name: &str,
    interceptor: &dyn Interceptor,
    msg: DapMessage,
) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let orig_tokens = count_tokens(bpe, &msg);
    let orig_bytes = serde_json::to_string(&msg).unwrap().len();

    let result = rt
        .block_on(interceptor.intercept(msg, Direction::ServerToClient))
        .unwrap()
        .unwrap();

    let comp_tokens = count_tokens(bpe, &result);
    let comp_bytes = serde_json::to_string(&result).unwrap().len();

    let token_pct = if orig_tokens > 0 {
        (orig_tokens as f64 - comp_tokens as f64) / orig_tokens as f64 * 100.0
    } else {
        0.0
    };
    let byte_pct = if orig_bytes > 0 {
        (orig_bytes as f64 - comp_bytes as f64) / orig_bytes as f64 * 100.0
    } else {
        0.0
    };

    let token_pct_s = format!("{:.1}%", token_pct);
    let byte_pct_s = format!("{:.1}%", byte_pct);
    println!(
        "| {name:<25} | {orig_tokens:<10} | {comp_tokens:<10} | {token_pct_s:>7} | {orig_bytes:<10} | {comp_bytes:<10} | {byte_pct_s:>7} |"
    );
}

fn main() {
    let bpe = cl100k_base().expect("Failed to initialize tiktoken");

    println!("# dapz Compression Demo\n");
    println!("| Compressor | Raw (T) | Compact (T) | T Δ% | Raw (B) | Compact (B) | B Δ% |");
    println!("|------------|---------|-------------|------|---------|-------------|------|");

    // OutputCompressor
    demo_interceptor(
        &bpe,
        "OutputCompressor",
        &OutputCompressor,
        DapMessage {
            seq: 1,
            msg_type: "event".into(),
            command: None,
            event: Some("output".into()),
            request_seq: None,
            success: None,
            body: Some(serde_json::json!({
                "category": "stdout",
                "output": "\x1b[32mPASS\x1b[0m: test_foo\n\x1b[32mPASS\x1b[0m: test_bar\n\x1b[31mFAIL\x1b[0m: test_baz\n\x1b[32mPASS\x1b[0m: test_foo\n\x1b[32mPASS\x1b[0m: test_bar\n",
                "source": {
                    "path": "/home/user/project/src/main.rs",
                    "name": "main.rs",
                    "checksums": [{"algorithm": "md5", "checksum": "abc"}],
                },
                "line": 42,
                "column": 10,
            })),
            arguments: None,
        },
    );

    // EvaluateCompressor
    demo_interceptor(
        &bpe,
        "EvaluateCompressor",
        &EvaluateCompressor::new(500),
        DapMessage {
            seq: 10,
            msg_type: "response".into(),
            command: Some("evaluate".into()),
            event: None,
            request_seq: Some(9),
            success: Some(true),
            body: Some(serde_json::json!({
                "result": "{\n  \"name\": \"John\",\n  \"age\": 30,\n  \"email\": \"john@example.com\"\n}",
                "type": "dict",
                "variablesReference": 42,
                "namedVariables": 3,
                "memoryReference": "0x7fff00000010",
            })),
            arguments: None,
        },
    );

    // VariablesCompressor
    demo_interceptor(
        &bpe,
        "VariablesCompressor",
        &VariablesCompressor::new(120),
        DapMessage {
            seq: 20,
            msg_type: "response".into(),
            command: Some("variables".into()),
            event: None,
            request_seq: Some(19),
            success: Some(true),
            body: Some(serde_json::json!({
                "variables": [
                    {"name": "x", "value": "42", "type": "int", "variablesReference": 0},
                    {"name": "msg", "value": "\"hello world this is a test string\"", "type": "String", "variablesReference": 0},
                    {"name": "items", "value": "[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]", "type": "int[]", "variablesReference": 10, "indexedVariables": 10, "namedVariables": 0},
                    {"name": "data", "value": "a very long string that should definitely be truncated by the compressor at 120 characters for token savings purposes", "type": "String", "variablesReference": 0, "memoryReference": "0x7fff00000020"},
                ]
            })),
            arguments: None,
        },
    );

    // StackTraceCompressor
    demo_interceptor(
        &bpe,
        "StackTraceCompressor",
        &StackTraceCompressor,
        DapMessage {
            seq: 30,
            msg_type: "response".into(),
            command: Some("stackTrace".into()),
            event: None,
            request_seq: Some(29),
            success: Some(true),
            body: Some(serde_json::json!({
                "stackFrames": [
                    {"id": 1, "name": "main", "source": {"path": "/home/user/project/src/main.rs", "name": "main.rs"}, "line": 42, "column": 5, "instructionPointerReference": "0xaaa0", "moduleId": 100},
                    {"id": 2, "name": "handle(data: Vec<u8>, ctx: &Context)", "source": {"path": "/home/user/project/src/main.rs", "name": "main.rs"}, "line": 38, "column": 9, "instructionPointerReference": "0xaaa1", "moduleId": 100},
                    {"id": 3, "name": "parse(buf: &[u8]) -> Result<Packet>", "source": {"path": "/home/user/project/src/parser.rs", "name": "parser.rs"}, "line": 100, "column": 4, "instructionPointerReference": "0xaaa2", "moduleId": 100},
                    {"id": 4, "name": "label_frame", "presentationHint": "label", "source": {"path": "/usr/lib/libc.so.6", "name": "libc.so.6"}, "line": 0, "column": 0},
                ]
            })),
            arguments: None,
        },
    );

    // CappingInterceptor (max 3 frames, 3 variables, 200 chars output)
    demo_interceptor(
        &bpe,
        "CappingInterceptor",
        &CappingInterceptor::new(3, 3, 200),
        DapMessage {
            seq: 40,
            msg_type: "response".into(),
            command: Some("stackTrace".into()),
            event: None,
            request_seq: Some(39),
            success: Some(true),
            body: Some(serde_json::json!({
                "stackFrames": [
                    {"id": 1, "name": "a", "line": 1, "column": 1},
                    {"id": 2, "name": "b", "line": 2, "column": 1},
                    {"id": 3, "name": "c", "line": 3, "column": 1},
                    {"id": 4, "name": "d", "line": 4, "column": 1},
                    {"id": 5, "name": "e", "line": 5, "column": 1},
                ]
            })),
            arguments: None,
        },
    );
}
