#!/usr/bin/env bash
# Regenerate compression benchmark artifacts from fixtures.
# - docs/src/benchmarks.md  (full report)
# - README.md               (summary between BENCH-SUMMARY markers)
set -euo pipefail

cd "$(dirname "$0")/.."

cargo run --example bench-report

echo "Generated docs/src/benchmarks.md and synced README BENCH-SUMMARY block"
