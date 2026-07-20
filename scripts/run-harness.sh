#!/usr/bin/env bash
# Full harness: env → qa → optional debugpy e2e.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Ensure uv/pipx/cargo user bins are visible in agent/CI shells.
export PATH="${HOME}/.local/bin:${CARGO_HOME:-$HOME/.cargo}/bin:${PATH:-}"

REQUIRE_E2E="${DAPZ_REQUIRE_E2E:-1}"
SKIP_E2E=0

echo "== dapz harness =="

ENV_OUT="$(mktemp)"
if bash scripts/check-env.sh | tee "$ENV_OUT"; then
  # Pick up discovered backend for e2e
  if grep -q '^DAPZ_DEBUGPY_BACKEND=' "$ENV_OUT"; then
    # shellcheck disable=SC1090
    eval "$(grep '^DAPZ_DEBUGPY_BACKEND=' "$ENV_OUT" | tail -1)"
    export DAPZ_DEBUGPY_BACKEND
  fi
else
  if [[ "${CI:-}" == "true" || "${DAPZ_CI_SKIP_ENV:-}" == "1" ]]; then
    echo "WARN  check-env failed in CI — continuing unit qa only"
    SKIP_E2E=1
  else
    rm -f "$ENV_OUT"
    exit 1
  fi
fi
rm -f "$ENV_OUT"

echo
echo "-- fmt-check --"
cargo fmt -- --check

echo
echo "-- clippy --"
cargo clippy --all-features -- -D warnings

echo
echo "-- test --"
cargo test --all-features

echo
echo "-- e2e (ignored debugpy) --"
if [[ "$SKIP_E2E" -eq 1 ]]; then
  echo "SKIP  agent_debug_loop / debugpy_integration (env)"
elif [[ -n "${DAPZ_DEBUGPY_BACKEND:-}" ]]; then
  export DAPZ_DEBUGPY_BACKEND
  cargo test --all-features --test agent_debug_loop -- --ignored --test-threads=1
  cargo test --all-features --test debugpy_integration -- --ignored --test-threads=1 || true
else
  if [[ "$REQUIRE_E2E" == "1" ]]; then
    echo "FAIL  debugpy missing and DAPZ_REQUIRE_E2E=1"
    exit 1
  fi
  echo "SKIP  debugpy e2e"
fi

echo
echo "harness: PASS"
