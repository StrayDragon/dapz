# language: zh-CN
# capability: interceptors
# purpose: 拦截器：capping 与 DAP 压缩器（output/stack/vars/evaluate/scopes/exceptionInfo）。
# scope: src/interceptors/, tests/

功能: interceptors

  @req:r9
  规则: Capping
    系统 MUST 提供 CappingInterceptor 限制 output/stackTrace/variables 条目规模。
  @req:r17
  规则: OutputCompressor
    系统 MUST 压缩 output 事件（重复行折叠、类别缩写等）。
  @req:r25
  规则: StackTraceCompressor
    系统 MUST 压缩 stackTrace 响应（路径缩写、合成帧过滤等）。
  @req:r32
  规则: VariablesCompressor
    系统 MUST 压缩 variables 响应（长值截断、类型前缀等）。
  @req:r5
  规则: EvaluateCompressor
    系统 MUST 压缩 evaluate 响应（结果截断、去 memoryReference 等）。
  @req:r6
  规则: ScopesCompressor
    系统 MUST 压缩 scopes 响应（去掉位置噪声并保留 variablesReference）。
  @req:r47
  规则: ExceptionInfoCompressor
    系统 MUST 提供 ExceptionInfoCompressor，压缩 exceptionInfo 响应中的过长 details/堆栈噪声。
  @req:r59
  规则: Scopes Exception 基准
    系统 MUST 为 ScopesCompressor 与 ExceptionInfoCompressor 提供 fixtures/bench 场景，并纳入 bench-report 汇总。
  @req:r66
  规则: 压缩契约版本
    系统 MUST 公布压缩契约标识 dapz-compress/1；改变删除字段集合时 MUST 递增契约版本并更新文档，避免适配器升级时静默改变 Agent 可见字段。
  @req:r9
  规则: cap-frames
    MUST hold: Given 超长 stackTrace; When 经 CappingInterceptor; Then 帧数不超过配置上限.
  @req:r17
  规则: fold-output
    MUST hold: Given 重复 stdout 行; When 经 OutputCompressor; Then 折叠为带计数的紧凑形式.
  @req:r25
  规则: abbrev-path
    MUST hold: Given 深路径 stack frame; When 经 StackTraceCompressor; Then 路径缩短且可定位.
  @req:r32
  规则: truncate-value
    MUST hold: Given 超长变量值; When 经 VariablesCompressor; Then 截断到 max_value_length.
  @req:r5
  规则: truncate-eval
    MUST hold: Given 超长 evaluate result; When 经 EvaluateCompressor; Then 截断到 max_evaluate_length.
  @req:r6
  规则: scopes-keep-ref
    MUST hold: Given 含 source/line 的 scopes; When 经 ScopesCompressor; Then 保留 variablesReference 且去掉位置噪声.
  @req:r47
  规则: truncate-details
    MUST hold: Given 超长 exceptionInfo details; When 经 ExceptionInfoCompressor; Then details 被截断且保留 exceptionId/breakMode.
  @req:r59
  规则: bench-scopes-exception
    MUST hold: Given 存在 scopes 与 exception fixtures; When 运行 bench-report; Then 报告含二者场景行.
  @req:r66
  规则: contract-const
    MUST hold: Given 查阅 interceptors 模块; When 读取契约常量; Then 值为 dapz-compress/1.
