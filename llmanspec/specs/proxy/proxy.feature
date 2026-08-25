# language: en
# capability: proxy
# purpose: 代理核心：DAP 会话转发、拦截链与 CLI 子命令。
# scope: src/proxy.rs, src/main.rs, tests/

Feature: proxy

  @req:r12 @human
  Scenario: 透明代理
    - 系统 MUST 提供 Proxy 在 Agent 与 DAP adapter 之间透明转发请求与响应。

  @req:r20 @human
  Scenario: 拦截链
    - Proxy MUST 对 Server→Client 消息应用 InterceptorChain（capping + compressors）；Client→Server MUST 原样转发。

  @req:r28 @human
  Scenario: CLI 子命令
    - CLI MUST 提供 dapz proxy（及别名 p）与 dapz mcp 子命令，且 proxy 子命令 MUST 暴露 metrics 与 transport 旋钮。

  @req:r35 @human
  Scenario: 配置驱动输出
    - 系统 MUST 使 Proxy 按 OutputFormat 分流：passthrough MUST 原样转发且 MUST NOT 进入拦截链；json MUST 经 InterceptorChain 后输出压缩 DAP JSON 帧；toon MUST 经 InterceptorChain 后将 body 编码为 {format:toon,text} 包装（保留 DAP 信封字段）。编码失败 MUST fail-open 原始帧。

  @req:r60 @human
  Scenario: Proxy CLI 旋钮
    - 系统 MUST 使 dapz proxy 支持 --metrics 与 --transport（stdio 或 tcp://）；--transport 默认 stdio；--output 默认 MUST 为 toon（与 MCP/SDK 一致），IDE 可显式选用 json。

  @req:r61 @human
  Scenario: Proxy 默认 TOON
    - 系统 MUST 使 dapz proxy 的默认 OutputFormat 为 toon；用户 MUST 仍可通过 --output json 或 passthrough 覆盖。

  @req:r12 @human
  Scenario: proxy-start
    - MUST hold: Given 配置 backend 命令; When 启动 Proxy; Then 建立与 adapter 的会话.
    Given 配置 backend 命令
    When 启动 Proxy
    Then 建立与 adapter 的会话

  @req:r20 @human
  Scenario: server-to-client-only
    - MUST hold: Given Client→Server 请求与 Server→Client 响应; When 经 Proxy; Then 仅后者进入拦截链.
    Given Client→Server 请求与 Server→Client 响应
    When 经 Proxy
    Then 仅后者进入拦截链

  @req:r28 @human
  Scenario: help-subcommands
    - MUST hold: Given 运行 dapz --help; When 查看子命令; Then 列出 proxy 与 mcp.
    Given 运行 dapz --help
    When 查看子命令
    Then 列出 proxy 与 mcp

  @req:r35 @human
  Scenario: passthrough-raw
    - MUST hold: Given output_format=passthrough; When 转发 Server→Client output 事件; Then 输出字节与输入一致且未经压缩.
    Given output_format=passthrough
    When 转发 Server→Client output 事件
    Then 输出字节与输入一致且未经压缩

  @req:r35 @human
  Scenario: json-compressed
    - MUST hold: Given output_format=json 且启用 output 压缩; When 转发含 ANSI 的 output 事件; Then 仍为 DAP JSON 帧且 body 已压缩.
    Given output_format=json 且启用 output 压缩
    When 转发含 ANSI 的 output 事件
    Then 仍为 DAP JSON 帧且 body 已压缩

  @req:r35 @human
  Scenario: toon-wrapped
    - MUST hold: Given output_format=toon; When 转发可压缩 response/event; Then body.format=toon 且 body.text 为非空 TOON.
    Given output_format=toon
    When 转发可压缩 response/event
    Then body.format=toon 且 body.text 为非空 TOON

  @req:r60 @human
  Scenario: proxy-no-ws-help
    - MUST hold: Given 运行 dapz proxy --help; When 查看 --transport 说明; Then 不承诺 ws:// 支持.
    Given 运行 dapz proxy --help
    When 查看 --transport 说明
    Then 不承诺 ws:// 支持

  @req:r61 @human
  Scenario: proxy-default-toon
    - MUST hold: Given 未传 --output; When 解析 ProxyArgs / Config 缺省; Then output_format 为 toon.
    Given 未传 --output
    When 解析 ProxyArgs / Config 缺省
    Then output_format 为 toon
