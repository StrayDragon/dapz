//! Tier-1 passthrough regression: Proxy/interceptors must not mutate
//! non-whitelist DAP responses (e.g. `modules`, `setVariable`).

use dapz::codec::json_rpc::DapMessage;
use dapz::interceptors::Interceptor;
use dapz::interceptors::InterceptorChain;
use dapz::interceptors::capping::CappingInterceptor;
use dapz::interceptors::evaluate::EvaluateCompressor;
use dapz::interceptors::output::OutputCompressor;
use dapz::interceptors::scopes::ScopesCompressor;
use dapz::interceptors::stacktrace::StackTraceCompressor;
use dapz::interceptors::variables::VariablesCompressor;
use dapz::proxy::Direction;
use dapz::{CappingConfig, Config, OutputFormat};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

fn build_chain() -> InterceptorChain {
    let config = Arc::new(RwLock::new(Config {
        backend_cmd: "test".into(),
        capping: CappingConfig {
            max_frames: 5,
            max_variables: 5,
            max_output_length: 100,
            max_evaluate_length: 500,
            max_value_length: 120,
        },
        enable_output_compress: true,
        enable_variables_compress: true,
        enable_stacktrace_compress: true,
        enable_evaluate_compress: true,
        enable_scopes_compress: true,
        output_format: OutputFormat::Json,
        log_level: "info".into(),
    }));

    let interceptors: Vec<Box<dyn Interceptor>> = vec![
        Box::new(CappingInterceptor::new(5, 5, 100)),
        Box::new(OutputCompressor),
        Box::new(EvaluateCompressor::new(500)),
        Box::new(VariablesCompressor::new(120)),
        Box::new(StackTraceCompressor),
        Box::new(ScopesCompressor),
    ];
    InterceptorChain::new(interceptors, config)
}

fn response(command: &str, body: serde_json::Value) -> DapMessage {
    DapMessage {
        seq: 2,
        msg_type: "response".into(),
        command: Some(command.into()),
        event: None,
        request_seq: Some(1),
        success: Some(true),
        body: Some(body),
        arguments: None,
    }
}

#[tokio::test]
async fn test_tier1_modules_passthrough_unchanged() {
    let original = response(
        "modules",
        json!({
            "modules": [{
                "id": 1,
                "name": "main",
                "path": "/tmp/main.py",
                "isOptimized": false,
                "extraNoiseField": "keep-me"
            }],
            "totalModules": 1
        }),
    );
    let before = serde_json::to_value(&original).unwrap();
    let chain = build_chain();
    let out = chain
        .process(original, Direction::ServerToClient)
        .await
        .unwrap()
        .unwrap();
    let after = serde_json::to_value(&out).unwrap();
    assert_eq!(before, after, "modules response must be untouched (Tier-1)");
}

#[tokio::test]
async fn test_tier1_set_variable_passthrough_unchanged() {
    let original = response(
        "setVariable",
        json!({
            "value": "42",
            "type": "int",
            "variablesReference": 0,
            "memoryReference": "0xdeadbeef"
        }),
    );
    let before = serde_json::to_value(&original).unwrap();
    let chain = build_chain();
    let out = chain
        .process(original, Direction::ServerToClient)
        .await
        .unwrap()
        .unwrap();
    let after = serde_json::to_value(&out).unwrap();
    assert_eq!(before, after, "setVariable must be untouched (Tier-1)");
}

#[tokio::test]
async fn test_tier0_variables_still_compressed() {
    let original = response(
        "variables",
        json!({
            "variables": [{
                "name": "x",
                "value": "hello",
                "type": "str",
                "variablesReference": 0,
                "memoryReference": "0x1"
            }]
        }),
    );
    let chain = build_chain();
    let out = chain
        .process(original, Direction::ServerToClient)
        .await
        .unwrap()
        .unwrap();
    let var = &out.body.as_ref().unwrap()["variables"][0];
    // Type prefix + prune memoryReference
    assert!(var.get("memoryReference").is_none());
    assert!(var["name"].as_str().unwrap().contains("str") || var["name"] == "x");
}
