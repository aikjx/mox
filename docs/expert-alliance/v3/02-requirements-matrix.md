# 02 - 架构需求矩阵

> 版本：v3.0 | 日期：2026-08-26
>
> 前置：[01-架构优化分析](./01-architecture-optimization.md)

---

## 一、功能需求矩阵

| ID | 需求 | 优先级 | 对应服务 | 验收标准 | 依赖 |
|----|------|--------|----------|----------|------|
| FR-001 | 自然语言创建协作任务 | P0 | scheduler | 输入描述→返回task_id+计划预览，<500ms | 专家匹配 |
| FR-002 | 任务取消 | P0 | scheduler+executor | 取消后所有节点优雅终止，<2s | 执行引擎 |
| FR-003 | 任务状态查询 | P0 | scheduler | 返回状态/进度/当前节点，<100ms | - |
| FR-004 | 任务列表(分页/筛选/排序) | P0 | scheduler | 支持按状态/时间/类型筛选，<200ms | PostgreSQL |
| FR-005 | 任务执行详情(DAG+节点) | P0 | executor | 返回DAG图+每个节点状态/结果/思考，<200ms | - |
| FR-006 | 任务结果获取 | P0 | fusion+scheduler | 返回融合结果+各专家原始结果，<100ms | 融合引擎 |
| FR-007 | 失败节点重试 | P1 | executor | 指定节点重试，指数退避，最多3次 | - |
| FR-008 | 人工干预(暂停/恢复/改计划/指定专家/跳过) | P1 | scheduler+executor | 5种干预操作全部可用，<1s响应 | - |
| FR-009 | 任务结果导出(JSON/PDF/Markdown) | P2 | scheduler | 3种格式导出，文件可下载 | MinIO |
| FR-010 | 任务模板保存/复用 | P2 | scheduler | 保存任务配置为模板，一键复用 | - |
| FR-011 | 专家注册(JSON/YAML/Proto) | P0 | registry | 提交定义→验证→注册，<1s | 工具验证 |
| FR-012 | 专家定义验证(工具存在/能力完整/命名冲突) | P0 | registry | 3项验证全部执行，错误信息明确 | gRPC反射 |
| FR-013 | 专家更新/版本管理(semver/灰度) | P1 | registry | 支持版本号+灰度比例发布 | - |
| FR-014 | 专家注销 | P1 | registry | 注销后不接受新任务，进行中任务继续 | - |
| FR-015 | 专家搜索(领域/能力/名称/状态) | P0 | registry | 4种筛选+全文搜索，<200ms | PostgreSQL |
| FR-016 | 专家匹配(任务描述→Top N) | P0 | scheduler | 图谱推理+综合评分，<80ms | 图存储 |
| FR-017 | 专家健康检查(心跳/成功率/延迟/错误率) | P0 | registry | 4项指标实时统计，不健康自动降级 | - |
| FR-018 | 工具自动注册(gRPC反射) | P1 | registry | 扫描所有gRPC服务→生成Tool描述，<5s | gRPC反射 |
| FR-019 | 协作计划自动生成(DAG) | P0 | scheduler | 6种模式+依赖分析+拓扑排序，<150ms | 专家匹配 |
| FR-020 | 计划验证(无环/可达/输入完整) | P0 | scheduler | 3项验证，有环/不可达报错 | - |
| FR-021 | DAG执行引擎(拓扑调度/并行/依赖/状态) | P0 | executor | 支持并行/串行/条件/循环，<20ms调度延迟 | - |
| FR-022 | 节点执行(调用agent+超时/重试/替代/降级) | P0 | executor | 4种容错机制，节点失败自动处理 | agent |
| FR-023 | 进度实时推送(WebSocket+NATS) | P0 | executor | 状态变更<1s推送到前端 | WebSocket |
| FR-024 | Agent ReAct循环(理解→规划→执行→观察→审核) | P0 | agent | 5步循环，最多3轮迭代 | 工具调用 |
| FR-025 | 工具调用器(gRPC+超时/重试/熔断) | P0 | agent | 3种容错，参数自动映射 | 底层服务 |
| FR-026 | AI推理对接(Python sidecar+UDS+流式) | P0 | agent | 流式输出<500ms首包，Token级推送 | sidecar |
| FR-027 | 知识检索(图谱/语义/向量/全文) | P1 | agent | 4种检索方式，<300ms | 搜索服务 |
| FR-028 | 结果融合(6种策略) | P0 | fusion | 6种策略全部可用，<500ms | - |
| FR-029 | 融合结果质量评估(完整/一致/准确) | P1 | fusion | 3维度评分，<200ms | - |
| FR-030 | 迭代精炼(生成→审核→重做) | P1 | fusion | 最多3轮，质量达标自动停止 | agent |
| FR-031 | 工作记忆(任务级上下文/中间结果) | P0 | memory | 任务内共享，任务结束归档，<10ms | Redis |
| FR-032 | 会话记忆(用户级偏好/历史) | P1 | memory | TTL 24h，自动过期，<10ms | Redis |
| FR-033 | 长期记忆(历史任务/统计) | P1 | memory | 永久保存，可查询，<100ms | PostgreSQL |
| FR-034 | 案例自动提升(评分≥4→案例) | P0 | memory | 自动判断+写入图谱，异步不阻塞 | 图存储 |
| FR-035 | 案例检索(相似任务→历史案例) | P0 | memory+scheduler | 向量+图谱混合检索，<200ms | pgvector |
| FR-036 | 图谱学习(任务完成→更新边权重) | P1 | memory | 异步批量更新，不阻塞主流程 | 图存储 |
| FR-037 | REST API(/api/v1/expert/*) | P0 | gateway-http | 20+接口，OpenAPI文档 | 转码 |
| FR-038 | gRPC(内部服务间) | P0 | gateway-grpc | 5个服务gRPC接口，Proto契约 | - |
| FR-039 | JSON-RPC 2.0(/rpc) | P1 | gateway-http | 标准JSON-RPC，批量请求支持 | 转码 |
| FR-040 | MCP协议(/mcp，tools/list+tools/call) | P1 | gateway-http | Claude Desktop可连接调用 | 工具注册 |
| FR-041 | WebSocket(/ws，进度+流式输出) | P0 | gateway-http | 实时推送，断线重连，<1s延迟 | - |
| FR-042 | JSON-RPC→gRPC自动转码 | P1 | gateway-http | 基于Proto反射，零手写代码 | prost-reflect |

---

## 二、非功能需求矩阵

| ID | 需求 | 指标 | 优先级 | 验证方式 |
|----|------|------|--------|----------|
| NFR-001 | 服务可用性 | 99.95% | P0 | 连续运行30天统计 |
| NFR-002 | 任务创建响应 | P99<500ms | P0 | 压测1000请求 |
| NFR-003 | 专家匹配延迟 | P99<80ms | P0 | 压测1000请求 |
| NFR-004 | 计划生成延迟 | P99<150ms | P0 | 压测1000请求 |
| NFR-005 | 节点执行(不含AI) | P99<2s | P0 | 压测100节点 |
| NFR-006 | AI流式首包 | <500ms | P0 | 实测100次 |
| NFR-007 | 进度推送延迟 | <1s | P0 | 实测100次 |
| NFR-008 | 并发任务数 | ≥200 | P0 | 压测稳定运行10min |
| NFR-009 | 单任务最大节点数 | ≤20 | P1 | 配置限制 |
| NFR-010 | 单任务最大专家数 | ≤10 | P1 | 配置限制 |
| NFR-011 | 数据持久性 | 11个9 | P0 | 备份恢复测试 |
| NFR-012 | RPO | <1min | P0 | 故障注入测试 |
| NFR-013 | RTO | <15min | P0 | 故障恢复演练 |
| NFR-014 | 故障恢复(无状态) | <30s | P1 | Pod重启测试 |
| NFR-015 | 水平扩展 | 线性(70%效率) | P1 | 1→3→5副本压测 |
| NFR-016 | 认证 | JWT+OIDC+MFA | P0 | 安全测试 |
| NFR-017 | 授权 | RBAC+ABAC+数据权限 | P0 | 权限矩阵测试 |
| NFR-018 | 多租户隔离 | L1逻辑(默认)+L2/L3可升级 | P0 | 跨租户数据泄露测试 |
| NFR-019 | 传输加密 | TLS1.3+mTLS | P0 | 安全扫描 |
| NFR-020 | 存储加密 | 字段加密(AES-256/SM4) | P0 | 安全扫描 |
| NFR-021 | 密码哈希 | Argon2id | P0 | 安全审计 |
| NFR-022 | 审计日志 | 不可篡改(哈希链+WORM) | P0 | 篡改测试 |
| NFR-023 | 敏感数据脱敏 | 手机号/身份证/邮箱/Token | P0 | 日志检查 |
| NFR-024 | 合规 | 等保三级 | P1 | 第三方测评 |
| NFR-025 | 结构化日志 | JSON+Loki聚合 | P0 | 日志检索测试 |
| NFR-026 | 指标采集 | Prometheus+RED+USE+业务 | P0 | Grafana验证 |
| NFR-027 | 链路追踪 | OTel+Jaeger全链路 | P0 | Trace查询测试 |
| NFR-028 | 告警 | P0-P3四级+多渠道 | P0 | 告警触发测试 |
| NFR-029 | 限流 | 租户/用户/接口三级 | P0 | 限流测试 |
| NFR-030 | 熔断 | 三态(Closed/Open/HalfOpen) | P0 | 故障注入测试 |
| NFR-031 | 降级 | 5种降级策略 | P1 | 降级测试 |
| NFR-032 | 重试 | 指数退避+抖动≤3次 | P0 | 重试测试 |
| NFR-033 | 超时 | 多层级(网关/服务/DB/AI) | P0 | 超时测试 |
| NFR-034 | 舱壁 | 资源隔离(连接池/信号量) | P1 | 资源隔离测试 |
| NFR-035 | 死信队列 | NATS DLQ+人工处理 | P1 | DLQ测试 |
| NFR-036 | 专家故障切换 | 自动替代专家/降级 | P0 | 专家下线测试 |
| NFR-037 | 构建时间 | <5min(全量) | P2 | CI统计 |
| NFR-038 | 部署时间 | <2min(单服务) | P2 | CD统计 |
| NFR-039 | 回滚时间 | <1min | P2 | 回滚测试 |
| NFR-040 | 代码覆盖率 | ≥80% | P2 | CI统计 |

---

## 三、服务-需求映射矩阵

| 服务 | P0需求数 | P1需求数 | P2需求数 | 核心需求 |
|------|----------|----------|----------|----------|
| gateway-http | 8 | 4 | 0 | REST/WS/认证/限流/转码 |
| gateway-grpc | 3 | 0 | 0 | gRPC路由/负载均衡 |
| alliance-scheduler | 12 | 4 | 2 | 任务管理/专家匹配/计划生成/案例检索 |
| alliance-executor | 8 | 2 | 0 | DAG执行/节点调度/进度推送/人工干预 |
| alliance-fusion | 2 | 2 | 0 | 结果融合/质量评估/迭代精炼 |
| expert-registry | 6 | 3 | 0 | 专家CRUD/验证/健康检查/工具发现 |
| expert-agent | 4 | 2 | 0 | ReAct循环/工具调用/AI推理/知识检索 |
| expert-memory | 3 | 3 | 0 | 工作/会话/长期记忆/案例/图谱学习 |

---

## 四、依赖关系矩阵

| 服务 | 依赖的服务 | 被依赖的服务 | 数据依赖 |
|------|-----------|-------------|----------|
| gateway-http | scheduler/executor/fusion/registry/agent/memory | (对外) | Redis(限流/会话) |
| gateway-grpc | 所有内部服务 | 所有内部服务 | - |
| alliance-scheduler | registry/agent/memory/gateway-grpc | gateway-http/executor/fusion | PostgreSQL(tasks)/Redis(缓存)/图存储(匹配) |
| alliance-executor | agent/scheduler/memory/gateway-grpc | gateway-http/scheduler | PostgreSQL(task_nodes)/Redis(状态)/NATS(事件) |
| alliance-fusion | agent/memory/scheduler/gateway-grpc | gateway-http/scheduler | Redis(中间结果) |
| expert-registry | gateway-grpc | scheduler/gateway-http | PostgreSQL(experts)/Redis(缓存) |
| expert-agent | 所有底层gRPC服务/memory/gateway-grpc | executor/fusion | Redis(会话)/ai-inference-sidecar |
| expert-memory | 图存储/PostgreSQL/Redis/gateway-grpc | scheduler/executor/fusion/agent | PostgreSQL(案例)/Redis(记忆)/图存储(图谱) |

---

## 五、里程碑-需求映射

| 里程碑 | 时间 | P0完成率 | P1完成率 | 关键交付 |
|--------|------|----------|----------|----------|
| M1 | W22末 | 30% | 0% | 共享库+Proto+数据库迁移 |
| M2 | W24末 | 50% | 10% | 注册中心+Agent运行时+记忆服务 |
| M3 | W26末 | 70% | 30% | 调度器+执行器+基础融合 |
| M4 | W28末 | 90% | 50% | 端到端跑通+多协议网关 |
| M5 | W30末 | 100% | 80% | 安全加固+可观测性 |
| M6 | W32末 | 100% | 100% | 5个场景验证+性能压测达标 |

---

*下一篇：[03-全维业务流程图](./03-business-flow-diagrams.md)*
