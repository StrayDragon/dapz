# dapz 编码约定

> 项目通用编码规范定义在项目根目录的 [AGENTS.md](../../AGENTS.md)（SSOT）。

---

## 项目结构

```
dapz/
├── Cargo.toml           # single crate, feature flags
├── src/
│   ├── lib.rs           # 公共 API + 模块声明
│   ├── main.rs          # CLI 入口 (feature = "cli")
│   ├── proxy.rs         # Proxy 核心
│   ├── interceptors/    # 所有拦截器实现
│   │   ├── mod.rs       # Interceptor trait + chain
│   │   ├── capping.rs   # CappingInterceptor
│   │   ├── output.rs    # OutputCompressor
│   │   ├── variables.rs # VariablesCompressor
│   │   └── stacktrace.rs# StackTraceCompressor
│   ├── codec/           # 编解码层（JSON-RPC）
│   │   ├── mod.rs
│   │   └── json_rpc.rs  # JSON-RPC 2.0 framing
│   ├── transport/       # 传输层实现
│   │   ├── mod.rs       # Transport trait
│   │   ├── stdio.rs     # StdioTransport
│   │   ├── tcp.rs       # TcpTransport
│   │   └── mock.rs      # MockTransport
│   ├── config.rs        # Config + Builder
│   └── error.rs         # DapzError enum
├── examples/
├── tests/
└── benches/
```

## 拦截器约定

```rust
/// DAP 消息拦截器。
#[async_trait]
pub trait Interceptor: Send + Sync {
    /// 拦截器唯一名称
    fn name(&self) -> &str;

    /// 是否处理该消息
    fn applies_to(&self, msg: &DapMessage, direction: Direction) -> bool;

    /// 转换消息，Ok(None) = 丢弃
    async fn intercept(
        &self,
        msg: DapMessage,
        direction: Direction,
    ) -> Result<Option<DapMessage>, DapzError>;
}
```

## DAP 消息格式

DAP 基于 JSON-RPC 2.0，使用 `Content-Length` 头帧格式（与 LSP 相同）。

```
Content-Length: 123\r\n
\r\n
{"seq":1,"type":"request","command":"launch",...}
```

## 错误处理

- 所有公共 API 返回 `Result<T, DapzError>`
- 压缩失败降级：WARN 日志 + 透明转发原始消息
- 不 panic

## 公共 API 设计

```rust
// ✅ 好: 清晰的构建器模式
let config = Config::builder()
    .backend_cmd("lldb-vscode")
    .build();

let transport = StdioTransport::spawn("lldb-vscode", &[])?;
let proxy = Proxy::new(Arc::new(RwLock::new(config)), Box::new(transport), chain);
proxy.start().await?;
```

## 压缩失败降级

```rust
match self.interceptor_chain.process(msg, direction).await {
    Ok(Some(compressed)) => send(compressed),
    Ok(None) => { /* 消息被丢弃 */ }
    Err(e) => {
        warn!("Compression failed: {}, forwarding original", e);
        send_original(raw);
    }
}
```

## DAP 兼容性约定

- 非目标消息必须透明转发
- 不得修改 DAP 初始化握手（initialize/launch/configurationDone）
- 不得修改 server/client capabilities
