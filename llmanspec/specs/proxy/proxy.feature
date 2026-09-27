# language: zh-CN
# capability: proxy
# purpose: 代理核心：DAP 会话转发、拦截链与 CLI 子命令。
# scope: src/proxy.rs, src/main.rs, tests/

功能: proxy

  @req:r12
  规则: 透明代理
    系统 MUST 提供 Proxy 在 Agent 与 DAP adapter 之间透明转发请求与响应。

    场景: proxy-start
      假如 配置 backend 命令
      当 启动 Proxy
      那么 建立与 adapter 的会话

  @req:r20
  规则: 拦截链
    Proxy MUST 对 Server→Client 消息应用 InterceptorChain（capping + compressors）；Client→Server MUST 原样转发。

    场景: server-to-client-only
      假如 Client→Server 请求与 Server→Client 响应
      当 经 Proxy
      那么 仅后者进入拦截链

  @req:r28
  规则: CLI 子命令
    CLI MUST 提供 dapz proxy（及别名 p）与 dapz mcp 子命令，且 proxy 子命令 MUST 暴露 metrics 与 transport 旋钮。

    场景: help-subcommands
      假如 运行 dapz --help
      当 查看子命令
      那么 列出 proxy 与 mcp

  @req:r35
  规则: 配置驱动输出
    系统 MUST 使 Proxy 按 OutputFormat 分流：passthrough MUST 原样转发且 MUST NOT 进入拦截链；json MUST 经 InterceptorChain 后输出压缩 DAP JSON 帧；toon MUST 经 InterceptorChain 后将 body 编码为 {format:toon,text} 包装（保留 DAP 信封字段）。编码失败 MUST fail-open 原始帧。

    场景: passthrough-raw
      假如 output_format=passthrough
      当 转发 Server→Client output 事件
      那么 输出字节与输入一致且未经压缩

    场景: json-compressed
      假如 output_format=json 且启用 output 压缩
      当 转发含 ANSI 的 output 事件
      那么 仍为 DAP JSON 帧且 body 已压缩

    场景: toon-wrapped
      假如 output_format=toon
      当 转发可压缩 response/event
      那么 body.format=toon 且 body.text 为非空 TOON

  @req:r60
  规则: Proxy CLI 旋钮
    系统 MUST 使 dapz proxy 支持 --metrics 与 --transport（stdio 或 tcp://）；--transport 默认 stdio；--output 默认 MUST 为 toon（与 MCP/SDK 一致），IDE 可显式选用 json。

    场景: proxy-no-ws-help
      假如 运行 dapz proxy --help
      当 查看 --transport 说明
      那么 不承诺 ws:// 支持

  @req:r61
  规则: Proxy 默认 TOON
    系统 MUST 使 dapz proxy 的默认 OutputFormat 为 toon；用户 MUST 仍可通过 --output json 或 passthrough 覆盖。

    场景: proxy-default-toon
      假如 未传 --output
      当 解析 ProxyArgs / Config 缺省
      那么 output_format 为 toon
