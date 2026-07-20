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
    cargo clippy --all-features -- -D warnings

# Run cargo test.
test:
    cargo test --all-features

# Run all checks (qa = fmt-check + lint + test + doc-check).
qa: fmt-check lint test doc-check
    @echo "All checks passed!"

alias check := qa
alias ci := qa

# Full verification (qa + SDD validate + prek).
verify:
    bash scripts/verify-all.sh

# fmt-check only
fmt-check:
    cargo fmt -- --check

# Build API docs and open in browser.
doc:
    cargo doc --no-deps --all-features --open

# Check API docs build without errors.
doc-check:
    cargo doc --no-deps --all-features

# Run doc tests.
doc-test:
    cargo test --doc --all-features

# Run integration tests with debugpy (requires debugpy-adapter on PATH).
test-integration:
    cargo test --test debugpy_integration -- --ignored --test-threads=1

# Environment check for harness (rustc, python, debugpy, mcp build).
harness-env:
    bash scripts/check-env.sh

# Full local gate: env + fmt + clippy + test + ignored e2e (debugpy).
harness:
    bash scripts/run-harness.sh

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
