//! Compression benchmarks for dapz interceptors.
//!
//! Uses Criterion for throughput measurement.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_output_compression(c: &mut Criterion) {
    use dapz::codec::json_rpc::DapMessage;
    use dapz::interceptors::Interceptor;
    use dapz::interceptors::output::OutputCompressor;
    use dapz::proxy::Direction;

    let rt = tokio::runtime::Runtime::new().unwrap();
    let compressor = OutputCompressor;
    let msg = DapMessage {
        seq: 1,
        msg_type: "event".into(),
        command: None,
        event: Some("output".into()),
        request_seq: None,
        success: None,
        body: Some(serde_json::json!({
            "category": "stdout",
            "output": "line1\nline2\nline2\nline3\nline3\nline3\nline4\n",
        })),
        arguments: None,
    };

    c.bench_function("output_compress", |b| {
        b.iter(|| {
            let msg = black_box(msg.clone());
            let _ = rt.block_on(compressor.intercept(msg, Direction::ServerToClient));
        })
    });
}

criterion_group!(benches, bench_output_compression);
criterion_main!(benches);
