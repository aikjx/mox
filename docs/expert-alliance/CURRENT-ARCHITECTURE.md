---
title: 专家联盟当前实现架构（唯一权威）
version: V1.0
authority: 🟢权威
doc_id: EA-DOC-CURRENT
last_updated: 2026-09-22
source_of_truth: 代码事实（与 2026-09-22 代码核对一致）
---

# 专家联盟当前实现架构

> **本文档是专家联盟"当前实现态"的唯一权威架构描述。**
> v1/v2/v3 系列文档均为设计目标态，与代码可能存在差异，以本文为准。

---

## 一、物理代码分布

### 1.1 Crate 拓扑（15 crates）

```
platform/domains/alliance/
├── api/                          # DTO 定义
│   └── mox-alliance-api/
├── core/                         # 纯算法与业务核心
│   ├── mox-alliance-core/        # 纯算法：DAG拓扑 + 6种融合策略
│   ├── mox-alliance-scheduler-core/  # 调度器业务逻辑
│   ├── mox-alliance-executor-core/   # 执行器业务逻辑
│   ├── mox-alliance-config-core/     # 10大领域专家配置 + LLM路由
│   └── mox-alliance-boot-config/    # Nacos/命名/启动配置
├── proto/                        # 契约层：DTO + trait 抽象（协议先行）
│   ├── mox-alliance-common-proto/
│   ├── mox-alliance-scheduler-proto/
│   ├── mox-alliance-executor-proto/
│   └── mox-alliance-registry-proto/  # 注册中心契约（2026-09 归一化落地）
├── sdk/                          # 客户端SDK
│   ├── mox-alliance-sdk/
│   └── mox-alliance-http-sdk/
└── svc/                          # 可独立部署的服务
    ├── mox-alliance-scheduler-svc/  # 任务调度服务（:3100）
    ├── mox-alliance-registry-svc/   # 专家注册中心（:3400，实现 registry-proto 契约）
    └── mox-alliance-executor-svc/   # DAG执行服务（:3200）
```

### 1.2 网关内联模块（gateway/src/alliance/）

| 模块 | 行数 | 职责 |
|------|------|------|
| experts_collaboration.rs | 1943 | 协作编排核心：多轮辩论、任务分解、结果融合 |
| experts_orchestration.rs | 1090 | 编排逻辑：流程控制、条件分支、循环 |
| experts_dispatcher.rs | 1066 | 任务分发：专家路由、调用转发、审计记录 |
| experts_graph.rs | 979 | 图谱关联：专家关系网络、能力图谱查询 |
| experts_registry.rs | 875 | 专家注册：CRUD、健康状态、能力声明 |
| experts_common.rs | 864 | 共享状态：ExpertsSharedState、工具函数 |
| experts_session.rs | 848 | 会话管理：对话上下文、多轮历史 |
| experts_db.rs | 676 | 持久化：SQLite存储 + JSON文件备份 |
| experts_ext.rs | 311 | 扩展点：专家预约、预订管理 |

---

## 二、服务拓扑与端口

### 2.1 运行时进程

| 进程 | 端口 | 职责 | 部署方式 |
|------|------|------|---------|
| **platform-gateway-svc** | 3080 | 统一入口：REST/WS/内联专家逻辑 | Deployment |
| **alliance-scheduler-svc** | 3100 | 任务调度、专家匹配、计划生成 | Deployment |
| **alliance-executor-svc** | 3200 | DAG执行、节点调度、进度推送 | Deployment |
| 模块化网关 | 3080 | （与 platform-gateway 同一进程） | — |

### 2.2 进程间调用

```
用户/前端 → gateway:3080
              ↓ 进程内调用
         alliance/ 模块（专家注册/会话/协作/图谱）
              ↓ HTTP
         scheduler-svc:3100（匹配/计划/排队）
              ↓ HTTP
         executor-svc:3200（DAG执行/节点调度）
              ↓
         底层微服务（AI/图谱/搜索等）
```

**注意**：当前服务间通信为 HTTP 短调用，非 gRPC 长连接。gRPC :50051 端口在 framework 层保留但专家联盟未使用。

---

## 三、核心组件职责

### 3.1 调度器（scheduler-svc:3100）

**入口路由**：
- `POST /tasks` — 创建协作任务
- `GET /tasks` — 列出任务
- `GET /tasks/:task_id` — 获取任务详情
- `POST /tasks/:task_id` — 任务动作（取消/暂停/恢复/完成）
- `POST /experts/search` — 专家搜索
- `GET /health` · `GET /metrics` — 存活与联盟指标快照
- `GET /leadership` — 多活视角：`ha_enabled=false` 时本副本即唯一执行者；开启后返回 holder/是否 leader/租约任期与到期时刻

> 执行状态/节点/融合结果的读代理端点已移除（2026-09 边界归一化）：读路径由网关(:3080)直连执行器(:3200)。

**内部模块**（mox-alliance-scheduler-core）：
| 模块 | 职责 | 生产状态 |
|------|------|---------|
| `modular_matcher` | 模块化权重专家匹配 | ✅ 生产主路径 |
| `matcher` | 规则匹配（fallback） | ✅ 备用 |
| `matching` | 中文分词与领域推断 | ✅ 被两个matcher依赖 |
| `planner` | 协作计划DAG生成 | ✅ |
| `scheduler` | 任务排队与状态机 | ✅ |
| `llm_router` | 多Provider智能路由 | ✅ |
| `executor_bridge` | 执行器HTTP桥接 | ✅ |
| `registry` | 专家注册桥接trait | ✅ |
| `synchronizer` | 专家数据同步 | ✅ |
| `storage` | 任务持久化：memory/file(单写者)/SQLite；SQLite 侧同库存放租约表 | ✅ |
| `leadership` | 租约选主 + fencing 任期（`LeaseStore`/`LeaderElector`/`SqliteLeaseStore`） | ✅ 仅 `MOX_ALLIANCE_HA_MODE=on` 生效 |
| `metrics` | 全联盟共享计数器（`/metrics` 暴露） | ✅ |

### 3.2 执行器（executor-svc:3200）

**内部模块**（mox-alliance-executor-core）：
| 模块 | 职责 |
|------|------|
| `dag_engine` | DAG执行引擎：拓扑调度、并行执行、依赖管理 |
| `expert_executor` | 专家节点执行器：调用AI专家服务 |
| `fusion` | 结果融合适配层：节点结果→core引擎→FusionOutput |
| `mock_executor` | Mock执行器（测试用） |

### 3.3 网关内联层（gateway:3080）

**职责边界**：用户对话与会话层
- 接收 HTTP/WebSocket 请求
- 管理专家会话与上下文
- 专家注册 CRUD 与 SQLite 持久化
- 协作编排的内联实现（与 scheduler-svc 互补）
- 图谱关联查询（内存中）

**共享状态**：`ExpertsSharedState` 统一持有，避免多模块各建一份。

---

### 3.7 协作计划生成器

实现：SimplePlanGenerator（scheduler-core/planner.rs）

支持6种模式：Parallel/Sequential/Voting/Hierarchical/Debate/Iterative

已有设计：0匹配兜底、匹配分映射为融合权重

改进方向：Voting与Parallel结构相同需区分、缺少Dynamic模式、缺少条件分支节点

---

## 四、数据流

### 4.1 任务创建到完成

```
1. POST /api/experts/tasks（网关）
2. 网关创建会话记录 → experts_session
3. 转发到 scheduler-svc:3100 /tasks
4. scheduler 匹配专家（ModularWeightMatcher）
5. scheduler 生成协作计划 DAG（SimplePlanGenerator）
6. scheduler 通过 ExecutorBridge 调 executor-svc:3200
7. executor DAG引擎调度各节点
8. 每个节点调用对应专家（ExpertNodeExecutor）
9. 节点完成 → executor 状态更新 → 进度推送
10. 全部节点完成 → executor 调 fusion 引擎
11. 融合结果返回 scheduler → 网关 → 用户
```

### 4.2 专家匹配流程

```
任务描述 → 中文分词（matching::tokenize）
         → 领域推断（infer_domains）
         → 专家文本重叠计算（description_overlap）
         → 模块化权重评分（ModularWeightMatcher）
         → 健康状态过滤
         → Top N 专家输出
```

---

## 五、存储与持久化

| 数据类型 | 存储介质 | 位置 | 说明 |
|---------|---------|------|------|
| 专家注册表 | SQLite | 网关 experts_db.rs | 专家CRUD、健康状态 |
| 协作任务 | SQLite + JSON | scheduler-core/storage | 任务状态、节点记录 |
| 执行状态 + 融合结果 | SQLite（WAL）或 JSON | executor-svc/state_sink.rs → `data/alliance_tasks.db` / `.json` | 任务级 + 节点级增量落盘；融合输出以保留行 `__fusion_output__` 持久化，进程重启后 `/result`、`/fusion-result` 仍可读回 |
| 专家会话 | 内存 + SQLite备份 | 网关 experts_session.rs | 对话上下文 |
| 知识图谱关联 | 进程内结构 | 网关 experts_graph.rs | 专家-领域-能力关系 |
| 协作记忆 | 进程内 | 网关 experts_common.rs | 工作记忆、案例 |

**注意**：当前无外部 Redis/PostgreSQL/pgvector 依赖，全部嵌入式存储。

**生产部署约定**：调度器与执行器共用环境变量 `MOX_ALLIANCE_STORAGE_MODE`（同一任务真源）。设为 `sqlite` 时执行器具备完整持久化 + 重启恢复（未完成任务重注入、running 节点标记 interrupted、融合结果跨重启可读回）；默认 `file` 仅任务级快照、无恢复能力；`memory` 为纯内存。生产环境应显式设置 `MOX_ALLIANCE_STORAGE_MODE=sqlite`。

---

## 六、API 接口

### 6.1 网关对外（:3080）

| 前缀 | 接口数 | 说明 |
|------|--------|------|
| `/api/experts/*` | ~49 | 专家CRUD、会话、协作、查询 |
| `/api/alliance/*` | ~20 | 联盟管理、配置、状态 |
| `/ws/v1/*` | 2 | WebSocket进度推送 |

### 6.2 调度器内部（:3100）

| 路径 | 方法 | 说明 |
|------|------|------|
| `/tasks` | POST/GET | 创建/列出任务 |
| `/tasks/:task_id` | GET | 任务详情 |
| `/tasks/:task_id/nodes` | GET | 节点列表 |
| `/tasks/:task_id/result` | GET | 融合结果 |
| `/experts/search` | POST | 专家搜索 |
| `/health` | GET | 健康检查 |

---

### 3.6 专家模型三层架构

| 层 | 位置 | 模型 | 职责 |
|---|------|------|------|
| **配置层** | config-core domain_experts.rs | 10大领域专家默认配置 | 种子数据、LLM路由、模块配置 |
| **契约层** | common-proto 	ypes.rs | Expert / ExpertCapability / ExpertStatus | 跨服务gRPC契约、序列化 |
| **域模型层** | gateway experts_common.rs | ExpertDescriptor | 网关完整域模型：UI展示 + 业务管理 + SQLite持久化 |

**数据流**：config-core种子 → gateway SQLite持久化 → common-proto契约 → scheduler匹配

---

## 七、内置专家（10大领域）

| 专家ID | 领域 | 主模型 |
|--------|------|--------|
| expert-code | 代码编程 | DeepSeek Coder |
| expert-math | 数学推理 | GPT-4o |
| expert-medical | 医学咨询 | Claude Opus |
| expert-data | 数据分析 | GPT-4o |
| expert-ai | AI推理 | Claude 3.5 Sonnet |
| expert-security | 安全审计 | GPT-4o |
| expert-flow | 流程自动化 | GPT-4o |
| expert-governance | 数据治理 | GPT-4o |
| expert-fusion | 知识融合 | Claude 3.5 Sonnet |
| expert-alliance | 联盟协调 | GPT-4o |

权威定义：`mox-alliance-config-core/src/examples/domain_experts.rs`

---

## 八、与 v3 设计目标态的差距

| 维度 | 当前实现 | v3 目标态 | 演进优先级 |
|------|---------|----------|-----------|
| 服务数 | 3 svc（scheduler/registry/executor）+ 网关内联 | 7 独立服务 + sidecar | P2 |
| 存储 | SQLite 任务库（P1 起多副本安全：WAL + busy_timeout + 读直查库；租约表与任务表同库同权威源）+ JSON 文件仓库（全量快照单写者，开 HA 直接拒绝启动） | PostgreSQL + Redis + pgvector | P1 |
| 服务间通信 | HTTP | gRPC :50051 | P3 |
| fusion | executor-core 适配层 | 独立 fusion-svc | P2 |
| memory | 网关内联 session/db | 独立 memory-svc | P2 |
| registry | ✅ 已落地：独立 registry-svc(:3400) + proto 契约层 `mox-alliance-registry-proto`（2026-09 归一化）；P1 追加 node→rack→cell 10:1:1 分级心跳聚合（入流降 100 倍） | 独立 registry-svc | 已完成 |
| 传输加密 | ✅ 已落地：一键开关 `MOX_API_CRYPTO=sm4`，接口 data gzip+SM4-GCM 全链路归一化（网关/调度/执行/注册/桥/SDK，2026-09 证明 6/6，见 [API-CRYPTO-TRANSPORT](../api/API-CRYPTO-TRANSPORT.md)） | 生产密钥注入 + 前端协商 | 已完成(P0) |
| 海量规模编排 | ✅ 部分落地：registry 10:1:1 分级心跳聚合（`aggregation.rs` + `POST /api/registry/aggregated-heartbeat`）；调度器多活（`scheduler-core/src/leadership.rs` 租约选主 + fencing 任期、`SqliteLeaseStore`、leader 专属执行器对账与孤儿接管、`GET /leadership`，`MOX_ALLIANCE_HA_MODE=on` 显式开启，默认关闭＝单副本行为不变）。Cell 分层与分片感知方案见 [十万级规模方案](../architecture/microservices/07-massive-scale-100k-nodes.md) | 跨机仲裁后端（换 PG/etcd 类 `LeaseStore` 实现）+ ShardFanout 分片感知 DAG | P1→P2 |
| 协议 | REST + WS | + JSON-RPC + MCP | P3 |

---

## 九、关键设计决策

1. **务实优先**：先用 2 svc + 网关内联跑通全链路，再逐步拆分独立服务
2. **纯算法与IO分离**：mox-alliance-core 纯函数无IO，core层业务逻辑无状态
3. **共享状态单点**：ExpertsSharedState 在网关统一构造，避免数据分裂
4. **桥接模式**：scheduler 通过 ExecutorBridge trait 调用 executor，可替换实现
5. **模块化专家配置**：10大专家各自独立 LLM 配置，未配置回退全局默认
6. **周期职责单点、请求路径多活**：调度器多活不加分布式锁，而是区分两类工作——请求路径（建任务/查任务）无状态可任意 LB；扫全表并逐条求证执行器的对账/接管是唯一的单点职责，用租约选主限定在 leader，且写成幂等（租约只保证互斥，不保证 exactly-once）。租约表与任务表同库，避免"自认 leader 却写着别人的状态表"

---

*相关文档：[v3 架构优化设计](v3/README.md)（演进路线） | [专家注册表协议](expert-registry-and-protocol.md) | [知识图谱Schema](knowledge-graph-schema.md) | [接口传输加密一键开关](../api/API-CRYPTO-TRANSPORT.md)*
