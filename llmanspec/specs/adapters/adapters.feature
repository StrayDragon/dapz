# language: en
# capability: adapters
# purpose: 适配器发现：扩展名查找与 resolve_tool 包管理器默认路径。
# scope: src/adapters.rs, scripts/check-env.sh, tests/

Feature: adapters

  @req:r1 @human
  Scenario: 扩展名多后端
    - 系统 MUST 按文件扩展名解析 AdapterInfo（py→debugpy；c/cpp/rs→lldb）。

  @req:r2 @human
  Scenario: resolve_tool
    - resolve_tool MUST 按 PATH → ~/.local/bin → cargo bin → go bin → npm/bun 默认目录顺序查找可执行文件。

  @req:r3 @human
  Scenario: debugpy 发现
    - resolve_python_debug_adapter MUST 在 resolve_tool(debugpy-adapter) 之后尝试 uv tools 默认布局与 python -m debugpy.adapter。

  @req:r4 @human
  Scenario: 可测上下文
    - DiscoveryContext MUST 支持注入 HOME/XDG/CARGO/GOPATH/PATH 以便单测不依赖进程全局环境。

  @req:r56 @human
  Scenario: lldb 发现
    - 系统 MUST 提供 resolve_lldb_debug_adapter，按 lldb-dap 再 lldb-vscode 顺序经 resolve_tool 查找。

  @req:r63 @human
  Scenario: lldb e2e harness
    - 系统 MUST 提供可选的 lldb-dap e2e（ignored 测试 + harness）：在发现 lldb-dap/lldb-vscode 时可编译 C fixture 并完成 launch→stack→disconnect；缺失适配器时 MUST SKIP 且 MUST NOT 使 debugpy 门禁失败（除非 DAPZ_REQUIRE_LLDB_E2E=1）。

  @req:r1 @human
  Scenario: lookup-rs
    - MUST hold: Given 扩展名 rs; When lookup_by_extension; Then 返回 language=rust 且 backend 含 lldb.
    Given 扩展名 rs
    When lookup_by_extension
    Then 返回 language=rust 且 backend 含 lldb

  @req:r2 @human
  Scenario: user-bin-without-path
    - MUST hold: Given PATH 为空且 ~/.local/bin/mytool 可执行; When resolve_tool_in; Then 返回该路径.
    Given PATH 为空且 ~/.local/bin/mytool 可执行
    When resolve_tool_in
    Then 返回该路径

  @req:r3 @human
  Scenario: uv-tools-layout
    - MUST hold: Given 仅存在 XDG uv/tools/debugpy/bin/debugpy-adapter; When resolve_python_debug_adapter_in; Then 返回该 adapter 路径.
    Given 仅存在 XDG uv/tools/debugpy/bin/debugpy-adapter
    When resolve_python_debug_adapter_in
    Then 返回该 adapter 路径

  @req:r4 @human
  Scenario: mock-home
    - MUST hold: Given 临时目录作为 HOME; When 单元测试; Then 不修改真实环境变量即可断言发现结果.
    Given 临时目录作为 HOME
    When 单元测试
    Then 不修改真实环境变量即可断言发现结果

  @req:r56 @human
  Scenario: prefer-lldb-dap
    - MUST hold: Given PATH 同时有 lldb-dap 与 lldb-vscode; When resolve_lldb_debug_adapter_in; Then 返回 lldb-dap.
    Given PATH 同时有 lldb-dap 与 lldb-vscode
    When resolve_lldb_debug_adapter_in
    Then 返回 lldb-dap

  @req:r63 @human
  Scenario: lldb-e2e-run
    - MUST hold: Given 本机有 lldb-dap 与 cc; When 运行 harness lldb 段; Then ignored 测试通过.
    Given 本机有 lldb-dap 与 cc
    When 运行 harness lldb 段
    Then ignored 测试通过

  @req:r63 @human
  Scenario: lldb-e2e-skip
    - MUST hold: Given 无 lldb 适配器; When 运行 harness; Then 打印 SKIP 且退出码仍成功.
    Given 无 lldb 适配器
    When 运行 harness
    Then 打印 SKIP 且退出码仍成功
