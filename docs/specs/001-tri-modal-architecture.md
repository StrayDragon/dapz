# dapz 三模态架构规格

**版本**: v0.0.1
**状态**: 规划
**最后更新**: 2026-05-12

## 概述

dapz 采用三模态架构设计，支持作为库、DAP 代理、MCP 服务器三种产品形态。

## 架构总览

```mermaid
graph TB
    subgraph Core["dapz crate (feature flags)"]
        PROXY["Proxy Core<br/>生命周期管理<br/>消息路由"]
        INTERCEPTOR["Interceptor Chain<br/>转换/压缩/过滤"]
        CODEC["Codec Layer<br/>JSON-RPC 编解码"]
        TRANSPORT["Transport Trait<br/>抽象 I/O 通道"]
        MCPMOD["MCP Server<br/>(feature = mcp)"]
        AGENT["Agent SDK<br/>(feature = agent-sdk)"]
        CONFIG["Config<br/>Builder + Env"]
        ERROR["Error Types<br/>thiserror"]
    end

    subgraph Delivery["交付形态"]
        LIB["Library Mode<br/>use dapz::Proxy"]
        CLIBIN["Proxy Mode<br/>dapz --backend debug-adapter"]
        MCP["MCP Mode<br/>dapz mcp"]
    end

    PROXY --> INTERCEPTOR
    PROXY --> CODEC
    PROXY --> TRANSPORT
    PROXY --> CONFIG
    PROXY --> ERROR
    INTERCEPTOR --> CODEC
    CODEC --> ERROR
    MCPMOD --> PROXY
    AGENT --> MCPMOD
    LIB --> PROXY

    classDef core fill:#4A90E2,stroke:#2E5C8A,stroke-width:2px,color:#fff
    classDef delivery fill:#7ED321,stroke:#5BA01A,stroke-width:2px,color:#fff
    class Core core
    class LIB,CLIBIN,MCP delivery
```

## 拦截器链架构

```mermaid
flowchart TB
    subgraph Incoming["Server → Client Messages"]
        MSG["DAP Message (JSON-RPC 2.0)"]
    end

    subgraph Chain["Interceptor Chain (4 interceptors, config-gated)"]
        direction TB
        C["CappingInterceptor<br/>截断大返回"]
        O["OutputCompressor<br/>重复行折叠"]
        V["VariablesCompressor<br/>长值截断"]
        S["StackTraceCompressor<br/>路径缩写"]
    end

    subgraph ErrorPath["Error Handling (Fail-Open)"]
        ERR["Compression Failed"]
        FALLBACK["Transparent Forward"]
        LOG["Log WARN + Continue"]
        ERR --> FALLBACK
        ERR --> LOG
    end

    subgraph Outgoing["→ AI Agent"]
        COMPRESSED["Compressed"]
        RAW["Original Message"]
    end

    MSG --> C --> O --> V --> S
    S -->|Success| COMPRESSED
    S -->|Failure| ERR
    FALLBACK --> RAW

    classDef incoming fill:#7ED321,stroke:#5BA01A,stroke-width:2px,color:#fff
    classDef chain fill:#4A90E2,stroke:#2E5C8A,stroke-width:2px,color:#fff
    classDef error fill:#F5A623,stroke:#D4880F,stroke-width:2px,color:#fff
    classDef outgoing fill:#9013FE,stroke:#6A0DAD,stroke-width:2px,color:#fff

    class MSG incoming
    class C,O,V,S chain
    class ERR,FALLBACK,LOG error
    class COMPRESSED,RAW outgoing
```

## Proxy 状态机

```mermaid
stateDiagram-v2
    [*] --> Created: Proxy::new(config)
    Created --> Initializing: start()
    Initializing --> Ready: configurationDone received
    Ready --> ShuttingDown: disconnect received
    ShuttingDown --> Exited: exit
    Exited --> [*]

    state Ready {
        [*] --> Idle
        Idle --> Forwarding: Client→Server msg
        Forwarding --> Idle
        Idle --> Intercepting: Server→Client msg
        Intercepting --> Compressing: matched interceptor
        Intercepting --> Forwarding: no match
        Compressing --> Idle
        Compressing --> Forwarding: compression failed
    }
```

## DAP 握手流程

```mermaid
sequenceDiagram
    participant Agent as AI Agent (DAP Client)
    participant Proxy as dapz Proxy
    participant Server as DAP Server

    rect rgb(240, 248, 255)
        Note over Agent,Server: Phase 1: Initialize
        Agent->>Proxy: initialize request
        Proxy->>Server: forward (unmodified)
        Server-->>Proxy: initialize response (capabilities)
        Proxy-->>Agent: forward (unmodified)
    end

    rect rgb(240, 255, 240)
        Note over Agent,Server: Phase 2: Launch/Attach
        Agent->>Proxy: launch / attach request
        Proxy->>Server: forward
        Server-->>Proxy: response
        Proxy-->>Agent: forward
    end

    rect rgb(255, 248, 220)
        Note over Agent,Server: Phase 3: Configuration
        Agent->>Proxy: configurationDone request
        Proxy->>Server: forward
        Server-->>Proxy: response
        Proxy-->>Agent: forward
    end

    rect rgb(255, 240, 245)
        Note over Agent,Server: Phase 4: Debug Session
        Server-->>Proxy: output event (raw)
        Note over Proxy: Interceptor: compress output
        Proxy-->>Agent: output (compressed)

        Server-->>Proxy: stopped event
        Proxy-->>Agent: stopped (passthrough)

        Agent->>Proxy: stackTrace request
        Proxy->>Server: forward
        Server-->>Proxy: stackTrace response (raw)
        Note over Proxy: Interceptor: compress stacktrace
        Proxy-->>Agent: stackTrace (compressed)
    end
```

## Crate 结构

```
dapz/
├── Cargo.toml           # single crate, feature flags
├── src/
│   ├── lib.rs           # 公共 API + 模块声明
│   ├── main.rs          # CLI 入口 (feature = "cli")
│   ├── proxy.rs         # Proxy 核心状态机 + 消息循环
│   ├── interceptors/    # 所有拦截器实现
│   ├── codec/           # 编解码层（JSON-RPC）
│   ├── transport/       # 传输层实现（stdio, TCP, mock）
│   ├── config.rs        # Config + Builder
│   └── error.rs         # DapzError enum
├── scripts/             # 开发脚本
├── docs/                # 文档
├── benches/             # 基准测试
└── examples/            # 示例代码
```

## 兼容性矩阵

| 功能 | Library | Proxy | MCP |
|------|----------|-------|-----|
| Output 压缩 | ✅ | ✅ | ✅ |
| Variables 压缩 | ✅ | ✅ | ✅ |
| StackTrace 压缩 | ✅ | ✅ | ✅ |
| 自定义传输 | ✅ | ✅ (TCP/WS) | ❌ |
| 运行时配置 | ✅ | ✅ | ⚠️ |
| 多 server | ✅ | ⚠️ | ✅ |
| 零额外开销 | ✅ | ❌ (进程间) | ❌ (MCP 协议) |


## 参考文档

- [ROADMAP.md](../../ROADMAP.md) — 项目路线图
- [AGENTS.md](../../AGENTS.md) — 项目规范 SSOT
- [DAP Specification](https://microsoft.github.io/debug-adapter-protocol/specification)
