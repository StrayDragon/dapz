set shell := ["bash", "-euo", "pipefail", "-c"]

_default:
    @just --list

# Install prek hooks and build the project.
setup:
    prek install

# Run cargo fmt (write).
fmt:
    cargo fmt

# Run cargo clippy with strict lints.
lint:
    cargo clippy

# Run cargo test.
test:
    cargo test

# Run all checks (qa = fmt-check + lint + test).
qa: fmt-check lint test
    @echo "All checks passed!"

alias check := qa
alias ci := qa

# fmt-check only
fmt-check:
    cargo fmt -- --check

# Run integration tests with debugpy (requires debugpy-adapter on PATH).
test-integration:
    cargo test --test debugpy_integration -- --ignored --test-threads=1

# Run Criterion throughput benchmarks.
bench:
    cargo bench

# Run compression ratio report (fixture-based benchmark).
bench-report:
    cargo run --example bench-report

# Run compression demo with sample data (quick verification).
compress-demo:
    cargo run --example compress-demo

# Regenerate docs/src/benchmarks.md from fixtures.
gen-bench:
    bash scripts/run-bench.sh

# Run full benchmark suite + generate report.
bench-all: bench gen-bench
