## Design

| | debugpy e2e | lldb e2e |
|--|-------------|----------|
| Adapter | required for local harness | **optional** |
| Fixture | Python script | C source + `cc -g` binary |
| Launch | `launch_program(program=script)` | same API with `program=binary` |
| CI | SKIP if no debugpy | SKIP if no lldb-dap |

Harness: after debugpy block, if `DAPZ_LLDB_BACKEND` set → run ignored lldb test；else SKIP（不 fail，除非 `DAPZ_REQUIRE_LLDB_E2E=1`）。
