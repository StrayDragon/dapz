# language: zh-CN
# capability: adapters
# purpose: 适配器发现：扩展名查找与 resolve_tool 包管理器默认路径。
# scope: src/adapters.rs, scripts/check-env.sh, tests/

功能: adapters

  @req:r1
  规则: 扩展名多后端
    系统 MUST 按文件扩展名解析 AdapterInfo（py→debugpy；c/cpp/rs→lldb）。

    @skip
    场景: lookup-rs
      假如 扩展名 rs
      当 lookup_by_extension
      那么 返回 language=rust 且 backend 含 lldb

  @req:r2
  规则: resolve_tool
    resolve_tool MUST 按 PATH → ~/.local/bin → cargo bin → go bin → npm/bun 默认目录顺序查找可执行文件。

    @skip
    场景: user-bin-without-path
      假如 PATH 为空且 ~/.local/bin/mytool 可执行
      当 resolve_tool_in
      那么 返回该路径

  @req:r3
  规则: debugpy 发现
    resolve_python_debug_adapter MUST 在 resolve_tool(debugpy-adapter) 之后尝试 uv tools 默认布局与 python -m debugpy.adapter。

    @skip
    场景: uv-tools-layout
      假如 仅存在 XDG uv/tools/debugpy/bin/debugpy-adapter
      当 resolve_python_debug_adapter_in
      那么 返回该 adapter 路径

  @req:r4
  规则: 可测上下文
    DiscoveryContext MUST 支持注入 HOME/XDG/CARGO/GOPATH/PATH 以便单测不依赖进程全局环境。

    @skip
    场景: mock-home
      假如 临时目录作为 HOME
      当 单元测试
      那么 不修改真实环境变量即可断言发现结果

  @req:r56
  规则: lldb 发现
    系统 MUST 提供 resolve_lldb_debug_adapter，按 lldb-dap 再 lldb-vscode 顺序经 resolve_tool 查找。

    @skip
    场景: prefer-lldb-dap
      假如 PATH 同时有 lldb-dap 与 lldb-vscode
      当 resolve_lldb_debug_adapter_in
      那么 返回 lldb-dap

  @req:r63
  规则: lldb e2e harness
    系统 MUST 提供可选的 lldb-dap e2e（ignored 测试 + harness）：在发现 lldb-dap/lldb-vscode 时可编译 C fixture 并完成 launch→stack→disconnect；缺失适配器时 MUST SKIP 且 MUST NOT 使 debugpy 门禁失败（除非 DAPZ_REQUIRE_LLDB_E2E=1）。

    @skip
    场景: lldb-e2e-run
      假如 本机有 lldb-dap 与 cc
      当 运行 harness lldb 段
      那么 ignored 测试通过

    @skip
    场景: lldb-e2e-skip
      假如 无 lldb 适配器
      当 运行 harness
      那么 打印 SKIP 且退出码仍成功
