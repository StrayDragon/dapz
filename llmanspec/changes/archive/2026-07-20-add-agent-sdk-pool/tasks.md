# Tasks: add-agent-sdk-pool

- [x] 1. Add `src/agent_sdk/pool.rs` with session-key `AgentPool` / `AgentPoolBuilder`
- [x] 2. Export from `agent_sdk/mod.rs`; unit tests with mock `AgentHandle` insert
- [x] 3. `cargo test --features agent-sdk pool -- --nocapture`
- [x] 4. Delta specs for agent-sdk AgentPool requirements
- [x] 5. `llman sdd validate add-agent-sdk-pool --strict --no-interactive`
- [x] 6. `just qa`
- [x] 7. Update `_PLAN.md` §9 P1 to done after archive
