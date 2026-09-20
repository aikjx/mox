---
title: 专家联盟当前实现架构（唯一权威）
version: V1.0
authority: 🟢权威
doc_id: EA-DOC-CURRENT
last_updated: 2026-09-19
source_of_truth: 代码事实（与 2026-09-19 代码核对一致）
---

# 专家联盟当前实现架构

> **本文档是专家联盟"当前实现态"的唯一权威架构描述。**
> v1/v2/v3 系列文档均为设计目标态，与代码可能存在差异，以本文为准。

---

## 一、物理代码分布

### 1.1 Crate 拓扑（13 crates）

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
├── proto/                        # gRPC契约与trait抽象
│   ├── mox-alliance-common-proto/
│   ├── mox-alliance-scheduler-proto/
│   └── mox-alliance-executor-proto/
├── sdk/                          # 客户端SDK
│   ├── mox-alliance-sdk/
│   └── mox-alliance-http-sdk/
└── svc/                          # 可独立部署的服务
    ├── mox-alliance-scheduler-svc/  # 任务调度服务（:3100）
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
- `GET /tasks/:task_id/nodes` — 代理查询节点
- `GET /tasks/:task_id/result` — 代理查询结果
- `POST /experts/search` — 专家搜索

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
| `storage` | 任务持久化（SQLite/JSON） | ✅ |

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
| 专家会话 | 内存 + SQLite备份 | 网关 experts_session.rs | 对话上下文 |
| 知识图谱关联 | 进程内结构 | 网关 experts_graph.rs | 专家-领域-能力关系 |
| 协作记忆 | 进程内 | 网关 experts_common.rs | 工作记忆、案例 |

**注意**：当前无外部 Redis/PostgreSQL/pgvector 依赖，全部嵌入式存储。

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
| 服务数 | 2 svc + 网关内联 | 7 独立服务 + sidecar | P2 |
| 存储 | SQLite + JSON | PostgreSQL + Redis + pgvector | P1 |
| 服务间通信 | HTTP | gRPC :50051 | P3 |
| fusion | executor-core 适配层 | 独立 fusion-svc | P2 |
| memory | 网关内联 session/db | 独立 memory-svc | P2 |
| registry | 网关内联 experts_registry | 独立 registry-svc | P1 |
| 协议 | REST + WS | + JSON-RPC + MCP | P3 |

---

## 九、关键设计决策

1. **务实优先**：先用 2 svc + 网关内联跑通全链路，再逐步拆分独立服务
2. **纯算法与IO分离**：mox-alliance-core 纯函数无IO，core层业务逻辑无状态
3. **共享状态单点**：ExpertsSharedState 在网关统一构造，避免数据分裂
4. **桥接模式**：scheduler 通过 ExecutorBridge trait 调用 executor，可替换实现
5. **模块化专家配置**：10大专家各自独立 LLM 配置，未配置回退全局默认

---

*相关文档：[v3 架构优化设计](v3/README.md)（演进路线） | [专家注册表协议](expert-registry-and-protocol.md) | [知识图谱Schema](knowledge-graph-schema.md)*
