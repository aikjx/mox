# MOX 模块化模式智能分析（MODULARITY）

> 数据基准日：2026-09-07（注册表 223 条 · 43 域 · ready 12/beta 1/stub 30）
> 配套：`docs/API-REGISTRY.md`（接口↔实现）、`docs/ROADMAP-DOMAINS.md`（域排产）、
> `docs/architecture/BUSINESS-FLOWS.md`（业务处理流程）。本文档回答"模块化模式是否最优、如何全维调整"。

## 1. 现状模块化模式：六维盘点

| 维度 | 现状 | 具体形态 |
| --- | --- | --- |
| **Crate 层** | 6 层分层 | `foundation`（横切）→ 域内 `api`（契约）→ `proto`（gRPC）→ `core`（纯计算）→ `svc`（服务）→ `gateway`（唯一入口）；workspace 143 crates |
| **域切分** | 12 域驱动 | kg/ai/flow/data/cloud/voice/market/alliance/kb/base/project/platform（按业务能力切分） |
| **接口面** | 43 域描述符 | 前缀 + 层（L0-L10）+ 状态（ready/beta/stub）；223 条路由注册表（actuator 单一权威源） |
| **进程面** | 模块化单体 + 可拆分 | 网关 :8080 进程内装配 21 个路由单元；独立进程 KB:8104 / Scheduler:3100 / Executor:3200 / 编排器:3001；用户服务 :8012/:8000/:3020 |
| **状态面** | 注册中心单例 | `ModuleStates` 统一构造一次注入（防数据分裂）；跨进程走环境变量 + HTTP |
| **治理面** | 声明↔实现门禁 | gen-api-registry 自动生成；端口注册表 + verify-ports；一键启停；AGENTS.md |

## 2. 业界最优模块化模式对照（评估框架）

| 模式 | 核心主张 | 适用场景 | 代价 |
| --- | --- | --- | --- |
| 模块化单体（Modular Monolith） | 单进程内按域强边界组织，依赖单向、接口显式；**可渐进拆分为服务** | 中大型内部平台（MOX 现状） | 部署仍是单体，需纪律维护边界 |
| 微服务 | 独立部署 + 独立伸缩 | 组织规模 >2 个团队、强独立伸缩需求 | 分布式复杂性（网络/一致性/运维） |
| 清洁架构（Clean/Hexagonal） | 领域层独立于框架/IO，适配器在边缘 | 领域复杂、长期演进 | 抽象层多，上手门槛高 |
| DDD 限界上下文 | 按业务能力划界，防"大泥球" | 业务语义复杂 | 需领域专家 |
| 插件化架构 | 核心 + 可插拔扩展点 | 生态扩展 | 契约管理成本 |

**共识结论（业界主流演进路径）**：**先模块化单体 → 边界稳定后按需渐进拆微服务**。
一上来就微服务是反模式；模块化单体保留了单体的简单性与服务的演进性。

## 3. MOX 模式评估：智能评分（1-5）

| 准则 | 得分 | 依据 |
| --- | --- | --- |
| 内聚（域边界） | 4.5 | 12 域按业务能力切分；43 域描述符 = 插件式声明；新域三步接入（模块+描述符+登记） |
| 解耦（依赖单向） | 4.5 | 6 层单向依赖（foundation→api→proto→core→svc→gateway）；网关唯一入口；路由优先级显式（具体域先于 proxy 兜底） |
| 可演进（渐进拆分） | 4.5 | 进程面已分层：网关可把任何域路由单元迁出为独立进程（kb/scheduler/executor 已示范）；联盟 remote/off 模式切换即拆分预演 |
| 可替换（后端抽象） | 4.0 | 上游可覆盖（MOX_VOICE_UPSTREAM_URL/PRIMIFLOW_URL/ORCHESTRATOR_URL）；任务仓储可插拔（file/memory）；执行器 mock/expert 模式 |
| 可观测（可诊断） | 3.5→4.0 | 中间件 trace/指标/日志 + API 启停；**本轮新增 x-request-id 全链路追踪头** |
| 一致性（单一事实源） | 4.5 | actuator ROUTES 单一权威源 → 注册表自动生成 → 端口门禁；文档与实现由生成器闭环 |
| 声明诚实度（不虚标） | 4.5 | 43 域状态如实（stub 不装 ready）；本轮修正"联盟任务仓储"过时描述 |

**总分 ≈ 4.3/5**：在 Rust 生态企业级 AI 平台中属一流模块化实践（对照前轮框架评估：
mox 的模块化结构优于 LangChain/CrewAI 的库模式，与其"平台级"定位一致）。

## 4. 全维调整清单（本轮已落地 ✅ / 建议 ⏳）

| # | 维度 | 调整 | 状态 |
| --- | --- | --- | --- |
| 1 | 可观测 | `x-request-id` 全链路追踪：透传客户端 ID 或生成 UUID，注入响应头 + 日志关联（rid=） | ✅ 已落地 |
| 2 | 声明诚实 | 联盟任务仓储描述修正：InMemoryTaskRepository → 可插拔（MOX_ALLIANCE_STORAGE_MODE=file 默认快照持久化，data/alliance_tasks.json） | ✅ 已落地 |
| 3 | 域粒度 | 43 域描述符加 group 字段，归并为 9 能力组（platform/knowledge/ai/orchestration/storage/data/media/commerce/streaming），/api/v1/domains 输出带 group | ✅ 已落地 |
| 4 | 跨进程追踪 | proxy 反代透传 `x-request-id` 到 :3001/:8000（middleware 写回请求头，全量转发天然携带） | ✅ 已落地 |
| 5 | 配置统一 | 环境变量入口收敛到 platform_config.json 单一登记（当前已登记 9 服务） | ⏳ 建议 |
| 6 | 测试门禁 | 域归并集成测试 7 条（5 单元 + 2 HTTP 集成）进 CI，验证域数量/能力组/映射/端点输出 | ✅ 已落地 |
| 7 | 契约版本化 | proto 层 gRPC 契约语义化版本（当前内部未公开承诺） | ⏳ 建议 |

## 5. 演进路径（3 步，保持"模块化单体优先"）

1. **近期**：域归并分组（audio/storage 能力组）+ 跨进程 x-request-id 透传 + 集成测试门禁
2. **中期**：专家/联盟域按负载拆独立进程（进程面已预留：env 切换 + HTTP 桥接）；proto 契约版本化
3. **远期**：对第三方开放 gRPC 客户端 SDK（Python/TS），平台化对外（此时才需要完整微服务化）

> 原则：**拆分是演进而非目标**。任何拆分必须由真实负载/团队边界驱动，不因"听起来企业级"而拆。
