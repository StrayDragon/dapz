//! Compression benchmarks for dapz interceptors.
//!
//! Uses Criterion for throughput measurement. Loads fixture data
//! from `fixtures/bench/` to provide realistic scenarios.
//!
//! Run: cargo bench

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use dapz::codec::json_rpc::DapMessage;
use dapz::interceptors::Interceptor;
use dapz::interceptors::capping::CappingInterceptor;
use dapz::interceptors::evaluate::EvaluateCompressor;
use dapz::interceptors::output::OutputCompressor;
use dapz::interceptors::stacktrace::StackTraceCompressor;
use dapz::interceptors::variables::VariablesCompressor;
use dapz::proxy::Direction;

fn load_fixtures(path: &str) -> Vec<(String, DapMessage)> {
    let content = std::fs::read_to_string(path).expect("fixture file");
    let cases: Vec<serde_json::Value> = serde_json::from_str(&content).expect("valid JSON");
    cases
        .into_iter()
        .map(|c| {
            let name = c["name"].as_str().unwrap().to_string();
            let msg: DapMessage =
                serde_json::from_value(c["message"].clone()).expect("valid DapMessage");
            (name, msg)
        })
        .collect()
}

fn bench_output_compression(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let compressor = OutputCompressor;
    let cases = load_fixtures("fixtures/bench/output.json");

    let mut group = c.benchmark_group("output_compressor");
    for (name, msg) in &cases {
        group.bench_with_input(name, msg, |b, msg| {
            b.iter(|| {
                let msg = black_box(msg.clone());
                let _ = rt.block_on(compressor.intercept(msg, Direction::ServerToClient));
            })
        });
    }
    group.finish();
}

fn bench_evaluate_compression(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let compressor = EvaluateCompressor::new(500);
    let cases = load_fixtures("fixtures/bench/evaluate.json");

    let mut group = c.benchmark_group("evaluate_compressor");
    for (name, msg) in &cases {
        group.bench_with_input(name, msg, |b, msg| {
            b.iter(|| {
                let msg = black_box(msg.clone());
                let _ = rt.block_on(compressor.intercept(msg, Direction::ServerToClient));
            })
        });
    }
    group.finish();
}

fn bench_variables_compression(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let compressor = VariablesCompressor::new(120);
    let cases = load_fixtures("fixtures/bench/variables.json");

    let mut group = c.benchmark_group("variables_compressor");
    for (name, msg) in &cases {
        group.bench_with_input(name, msg, |b, msg| {
            b.iter(|| {
                let msg = black_box(msg.clone());
                let _ = rt.block_on(compressor.intercept(msg, Direction::ServerToClient));
            })
        });
    }
    group.finish();
}

fn bench_stacktrace_compression(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let compressor = StackTraceCompressor;
    let cases = load_fixtures("fixtures/bench/stacktrace.json");

    let mut group = c.benchmark_group("stacktrace_compressor");
    for (name, msg) in &cases {
        group.bench_with_input(name, msg, |b, msg| {
            b.iter(|| {
                let msg = black_box(msg.clone());
                let _ = rt.block_on(compressor.intercept(msg, Direction::ServerToClient));
            })
        });
    }
    group.finish();
}

fn bench_capping(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let compressor = CappingInterceptor::new(5, 5, 500);
    let cases = load_fixtures("fixtures/bench/capping.json");

    let mut group = c.benchmark_group("capping_interceptor");
    for (name, msg) in &cases {
        group.bench_with_input(name, msg, |b, msg| {
            b.iter(|| {
                let msg = black_box(msg.clone());
                let _ = rt.block_on(compressor.intercept(msg, Direction::ServerToClient));
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_output_compression,
    bench_evaluate_compression,
    bench_variables_compression,
    bench_stacktrace_compression,
    bench_capping,
);
criterion_main!(benches);
