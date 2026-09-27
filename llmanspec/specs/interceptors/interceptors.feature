# language: zh-CN
# capability: interceptors
# purpose: 拦截器：capping 与 DAP 压缩器（output/stack/vars/evaluate/scopes/exceptionInfo）。
# scope: src/interceptors/, tests/

功能: interceptors

  @req:r9
  规则: Capping
    系统 MUST 提供 CappingInterceptor 限制 output/stackTrace/variables 条目规模。

    @skip
    场景: cap-frames
      假如 超长 stackTrace
      当 经 CappingInterceptor
      那么 帧数不超过配置上限

  @req:r17
  规则: OutputCompressor
    系统 MUST 压缩 output 事件（重复行折叠、类别缩写等）。

    @skip
    场景: fold-output
      假如 重复 stdout 行
      当 经 OutputCompressor
      那么 折叠为带计数的紧凑形式

  @req:r25
  规则: StackTraceCompressor
    系统 MUST 压缩 stackTrace 响应（路径缩写、合成帧过滤等）。

    @skip
    场景: abbrev-path
      假如 深路径 stack frame
      当 经 StackTraceCompressor
      那么 路径缩短且可定位

  @req:r32
  规则: VariablesCompressor
    系统 MUST 压缩 variables 响应（长值截断、类型前缀等）。

    @skip
    场景: truncate-value
      假如 超长变量值
      当 经 VariablesCompressor
      那么 截断到 max_value_length

  @req:r5
  规则: EvaluateCompressor
    系统 MUST 压缩 evaluate 响应（结果截断、去 memoryReference 等）。

    @skip
    场景: truncate-eval
      假如 超长 evaluate result
      当 经 EvaluateCompressor
      那么 截断到 max_evaluate_length

  @req:r6
  规则: ScopesCompressor
    系统 MUST 压缩 scopes 响应（去掉位置噪声并保留 variablesReference）。

    @skip
    场景: scopes-keep-ref
      假如 含 source/line 的 scopes
      当 经 ScopesCompressor
      那么 保留 variablesReference 且去掉位置噪声

  @req:r47
  规则: ExceptionInfoCompressor
    系统 MUST 提供 ExceptionInfoCompressor，压缩 exceptionInfo 响应中的过长 details/堆栈噪声。

    @skip
    场景: truncate-details
      假如 超长 exceptionInfo details
      当 经 ExceptionInfoCompressor
      那么 details 被截断且保留 exceptionId/breakMode

  @req:r59
  规则: Scopes Exception 基准
    系统 MUST 为 ScopesCompressor 与 ExceptionInfoCompressor 提供 fixtures/bench 场景，并纳入 bench-report 汇总。

    @skip
    场景: bench-scopes-exception
      假如 存在 scopes 与 exception fixtures
      当 运行 bench-report
      那么 报告含二者场景行

  @req:r66
  规则: 压缩契约版本
    系统 MUST 公布压缩契约标识 dapz-compress/1；改变删除字段集合时 MUST 递增契约版本并更新文档，避免适配器升级时静默改变 Agent 可见字段。

    @skip
    场景: contract-const
      假如 查阅 interceptors 模块
      当 读取契约常量
      那么 值为 dapz-compress/1
