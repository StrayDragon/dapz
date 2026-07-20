#!/usr/bin/env bash
# Check local environment for dapz harness (required vs optional).
#
# Discovers debugpy via PATH + common package-manager locations (uv tool,
# ~/.local/bin, cargo/go bins) so Cursor/agent shells without those dirs still pass.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Common user-bin paths (uv tool / pipx / cargo) — prepend if missing.
export PATH="${HOME}/.local/bin:${CARGO_HOME:-$HOME/.cargo}/bin:${PATH:-}"

fail=0
warn=0

ok() { echo "PASS  $*"; }
bad() { echo "FAIL  $*"; fail=1; }
wrn() { echo "WARN  $*"; warn=1; }

echo "== dapz check-env =="

if command -v rustc >/dev/null && command -v cargo >/dev/null; then
  ok "rustc $(rustc --version | awk '{print $2}') / cargo"
else
  bad "rustc/cargo missing"
fi

if [[ -f rust-toolchain.toml ]]; then
  ok "rust-toolchain.toml present"
else
  wrn "rust-toolchain.toml missing"
fi

if command -v python3 >/dev/null; then
  ok "python3 $(python3 --version 2>&1 | awk '{print $2}')"
else
  bad "python3 missing"
fi

# Resolve debugpy adapter (same search order as src/adapters.rs).
resolve_debugpy() {
  if command -v debugpy-adapter >/dev/null 2>&1; then
    command -v debugpy-adapter
    return 0
  fi
  local candidates=(
    "${HOME}/.local/bin/debugpy-adapter"
    "${CARGO_HOME:-$HOME/.cargo}/bin/debugpy-adapter"
    "${XDG_DATA_HOME:-$HOME/.local/share}/uv/tools/debugpy/bin/debugpy-adapter"
  )
  local c
  for c in "${candidates[@]}"; do
    if [[ -x "$c" ]]; then
      echo "$c"
      return 0
    fi
  done
  local uv_py="${XDG_DATA_HOME:-$HOME/.local/share}/uv/tools/debugpy/bin/python3"
  if [[ -x "$uv_py" ]] && "$uv_py" -c "import debugpy" 2>/dev/null; then
    echo "$uv_py -m debugpy.adapter"
    return 0
  fi
  if python3 -c "import debugpy" 2>/dev/null; then
    echo "python3 -m debugpy.adapter"
    return 0
  fi
  return 1
}

if DAPZ_DEBUGPY_BACKEND="$(resolve_debugpy)"; then
  ok "debugpy adapter: $DAPZ_DEBUGPY_BACKEND"
  export DAPZ_DEBUGPY_BACKEND
else
  bad "debugpy adapter not found — try: uv tool install debugpy   (or pip install debugpy)"
fi

# Acceptance: default layouts still resolve with a stripped PATH (tip §验收).
if [[ -n "${DAPZ_DEBUGPY_BACKEND:-}" ]]; then
  CLEAN_BACKEND="$(
    env -i \
      HOME="$HOME" \
      XDG_DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}" \
      CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" \
      PATH="/usr/bin:/bin" \
      bash -c '
        resolve_debugpy() {
          local candidates=(
            "${HOME}/.local/bin/debugpy-adapter"
            "${CARGO_HOME}/bin/debugpy-adapter"
            "${XDG_DATA_HOME}/uv/tools/debugpy/bin/debugpy-adapter"
          )
          local c
          for c in "${candidates[@]}"; do
            if [[ -x "$c" ]]; then echo "$c"; return 0; fi
          done
          local uv_py="${XDG_DATA_HOME}/uv/tools/debugpy/bin/python3"
          if [[ -x "$uv_py" ]] && "$uv_py" -c "import debugpy" 2>/dev/null; then
            echo "$uv_py -m debugpy.adapter"; return 0
          fi
          return 1
        }
        resolve_debugpy
      '
  )" || CLEAN_BACKEND=""
  if [[ -n "$CLEAN_BACKEND" ]]; then
    ok "clean-PATH discovery: $CLEAN_BACKEND"
  else
    wrn "clean-PATH discovery missed (adapter only on extended PATH?)"
  fi
fi

if cargo build --features mcp,agent-sdk -q; then
  ok "cargo build --features mcp,agent-sdk"
else
  bad "cargo build --features mcp,agent-sdk failed"
fi

if command -v lldb-vscode >/dev/null || command -v lldb-dap >/dev/null \
  || [[ -x "${CARGO_HOME:-$HOME/.cargo}/bin/lldb-dap" ]]; then
  ok "lldb adapter present (optional)"
else
  wrn "lldb-vscode/lldb-dap not found (optional)"
fi

echo
if [[ "$fail" -ne 0 ]]; then
  echo "check-env: FAILED (required checks)"
  exit 1
fi
echo "check-env: OK (${warn} warnings)"
echo "DAPZ_DEBUGPY_BACKEND=${DAPZ_DEBUGPY_BACKEND:-}"
exit 0
