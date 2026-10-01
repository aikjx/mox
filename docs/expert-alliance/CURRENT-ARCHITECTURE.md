---
title: 专家联盟当前实现架构（唯一权威）
version: V1.1
authority: 🟢权威
doc_id: EA-DOC-CURRENT
last_updated: 2026-09-24
source_of_truth: 代码事实（2026-09-24 已逐条重核并就地修正：§1 物理代码分布、§3.6 三层架构、§3.7 计划生成器、§4 数据流、§5 存储、§6.1 对外接口数、§6.2 服务间调用、§7 内置专家——各节内嵌「⚠️ V1.1 核对补记」；§2 端口以 docs/api/PORT-REGISTRY.md 为权威并过 verify-ports.py；§8/§9 本轮未重核，沿用 V1.0，未重核即不声称已核）
---

# 专家联盟当前实现架构

> **本文档是专家联盟"当前实现态"的唯一权威架构描述。**
> v1/v2/v3 系列文档均为设计目标态，与代码可能存在差异，以本文为准。

---

## 一、物理代码分布

### 1.1 Crate 拓扑（16 crates）

```
platform/domains/alliance/
├── api/                          # DTO 定义 —— 该目录本身即 crate mox-alliance-api（Cargo.toml 在 api/ 下，不存在同名子目录）
├── core/                         # 纯算法与业务核心
│   ├── mox-alliance-core/        # 纯算法：DAG拓扑 + 6种融合策略
│   ├── mox-alliance-scheduler-core/  # 调度器业务逻辑
│   ├── mox-alliance-executor-core/   # 执行器业务逻辑
│   ├── mox-alliance-config-core/     # 10大领域专家配置 + LLM路由
│   ├── mox-alliance-boot-config/    # Nacos/命名/启动配置
│   └── mox-alliance-registry-core/  # 注册核心：分级心跳聚合 node→rack→cell 10:1:1
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
| experts_collaboration.rs | 2227 | 协作编排核心：多轮辩论（run_debate，rounds 为真循环 :496）、任务分解、结果融合 |
| experts_orchestration.rs | 1236 | 编排：topological_sort（:39-92，Kahn + 真环检测）、按 task_type 选固定步骤表（:99-140）、simulate_step_execution 预演文案（:221-274）、进程内 plan/history 台账 |
| experts_dispatcher.rs | 1173 | 任务分发：专家路由、调用转发、审计记录 |
| experts_graph.rs | 1099 | 图谱关联：专家关系网络、能力图谱查询 |
| experts_common.rs | 1014 | 共享状态：ExpertsSharedState（:456 起）、工具函数 |
| experts_registry.rs | 1009 | 专家注册：CRUD、可用性状态（availability.status 取 online/busy/offline/away，是登记值不是探活结果）、能力声明（skills / capabilities） |
| experts_session.rs | 959 | 会话管理：对话上下文、多轮历史 |
| experts_db.rs | 721 | 持久化：SQLite 存储（JSON 仅作一次性导入的历史输入，导入后改名归档） |
| experts_ext.rs | 366 | 扩展点：专家预约、预订管理 |
| registry_client.rs | 92 | 远程注册中心客户端：registry-svc 的 HTTP 调用面，health() 即 GET {base}/health（:31-38），探的是那个服务而非单个专家 |
| mod.rs | 28 | 模块声明 |

「行数」为 2026-09-24 实测（wc -l platform/gateway/mox-platform-gateway-svc/src/alliance/\*.rs，11 个文件合计 9924 行）。**此列无任何门禁或测试校验，跨版本必然漂移**；需要精确值时按上面的命令复算，不要引用本表数字做结论。

> ⚠️ **V1.1 核对补记（2026-09-24）**：本节此前的 9 行「行数」全部过期（如 experts_orchestration.rs 记 1090 实为 1236、experts_collaboration.rs 记 1943 实为 2227），且漏登记 registry_client.rs 与 mod.rs；两处职责与代码不符——
> ① experts_orchestration.rs 原写「流程控制、条件分支、循环」：**代码里不存在条件分支与循环执行**（无 step_type 分支、无重试回路；唯一的 while let 是 :69 的 Kahn 出队）。看起来像"分支/重试"的 retry_on_failure、retry_count 出现在 :864 与 :917，属于 GET orchestration/plugins 那个**硬编码 vec![...]**（:840-922）里的 config_schema 字符串，不是任何行为——该端点因此在前端台账里定性为 rejected。
> ② experts_db.rs 原写「SQLite存储 + JSON文件备份」：**全文件零 fs::write**，JSON 是被导入后 std::fs::rename 成 .json.migrated-时间戳 让路的旧输入（:588-591），不存在"备份写出"这条能力。
> ③ experts_registry.rs 原写「健康状态」：该文件里零 health 命中，专家侧只有 availability.status（online/busy/offline/away，注册时写入的登记值）、metrics、enabled、verification_status。健康这件事**确实存在于契约层与调度器侧**（common-proto types.rs:464 ExpertHealth；scheduler-core registry.rs:65 的 update_expert_health 由 synchronizer.rs:573 写入；matcher.rs:166 折成 is_healthy ? 1.0 : 0.3，再按 :122 以 0.15 权重参与评分），缺的是**网关的专家记录不带它**；网关侧唯一的探活是 registry_client.rs 对 registry-svc 进程的 GET {base}/health（:31-38），探的是服务不是某个专家。所以引用"健康"必须写明是哪一层——这与 engine_status 恒为字面量 running 同族：**状态字段不等于探活**，界面文案不得把登记状态写成"在线检测通过"。
> 另：§1.1 原画了 api/mox-alliance-api/ 一层，实际 api 目录本身就是该 crate。本节其余小节沿用 V1.0 的核对结果，未在本轮重核。

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

支持**7**种模式：Sequential / Parallel / Debate / Hierarchical / Iterative / Voting / Dynamic（AllianceMode 枚举 7 个变体，planner.rs:81-92 七条 match 分支全覆盖）

已有设计：0匹配兜底、匹配分映射为融合权重

改进方向：Voting与Parallel结构相同需区分、缺少条件分支节点

> ⚠️ **V1.1 核对补记**：本节原记「支持6种模式」并把「缺少Dynamic模式」列为改进方向，均已过期——Dynamic 已实现（planner.rs:88 分支 + :338 `decide_dynamic_mode`）。同一条"缺少条件分支节点"仍然成立，且与 §1.2 原表格宣称 experts_orchestration.rs 支持「条件分支」自相矛盾：以代码为准，**两处都没有条件分支执行**，网关侧那 5 个 orchestrate/plan 端点做的是固定步骤表 + Kahn 拓扑 + 预演文案。

---

## 四、数据流

### 4.1 任务创建到完成

**默认形态是全本地**（未配置远程 URL，或 `MOX_ALLIANCE_REMOTE_MODE=off`）——整条链路不跨进程：

```
1. POST /api/alliance/tasks            网关内 SDK 的 create_task（alliance.rs:565）
2. 任务写入 SDK 内嵌的 scheduler-core InMemoryTaskRepository（alliance.rs:41-42, 180-188）
3. 专家匹配 RuleBasedExpertMatcher     同进程
4. 协作计划 DAG     scheduler-core 的 SimplePlanGenerator 在规划期完成（alliance.rs:506）
5. 节点推进与融合   同一进程内
6. /dag、/logs/stream 读的就是这份进程内状态（进程内 ⇒ 重启即失，见 §5）
```

**只有显式接入远程时**（配了 URL 且 `MOX_ALLIANCE_REMOTE_MODE=auto`，`alliance_remote.rs:35`、`:101`）才走跨服务链路：网关 → scheduler-svc:3100（匹配 + 生成计划）→ ExecutorBridge → executor-svc:3200（DAG 引擎逐节点执行、fusion 融合）→ 回传网关。

> ⚠️ **V1.1 核对补记**：本节原为唯一一条 11 步链路，把「网关创建会话记录 → 转发 scheduler → executor 融合」当成默认路径，且第 1 步写作 POST /api/experts/tasks——网关路由表里没有这一行，真实是 /api/alliance/tasks（`actuator.rs:519`）。两条路径的差别不是细节：**本地形态的任务状态是进程内结构，重启即失；远程形态才谈得上 executor 的 SQLite 落盘与重启恢复**。界面文案与运维判断都必须先看 `MOX_ALLIANCE_REMOTE_MODE` 的取值，否则会把"重启后任务全没了"误读成后端丢数据。

### 4.2 专家匹配流程

```
任务描述 → 中文分词（matching::tokenize）
         → 领域推断（infer_domains）
         → 专家文本重叠计算（description_overlap）
         → 模块化权重评分（ModularWeightMatcher）
         → 健康度参与加权（非过滤）
         → Top N 专家输出
```

> ⚠️ **V1.1 核对补记**：第五步原写「健康状态过滤」，实为**加权项**：matcher.rs:166 `is_healthy ? 1.0 : 0.3` 折成 health_score，再按 :122 以 **0.15 权重**进入总分。也就是说不健康的专家仍会被选中，只是排名靠后——写成"过滤"会让人以为不健康者已被排除，这与熔断器无写侧（circuit_breakers 恒 `[]`）合起来看尤其容易误判。

---

## 五、存储与持久化

| 数据类型 | 存储介质 | 位置 | 说明 |
|---------|---------|------|------|
| 专家注册表 | SQLite | 网关 experts_db.rs（表 experts） | 专家 CRUD、可用性状态；启动 load_registry、变更 save_registry |
| 协作任务 | SQLite + JSON | scheduler-core/storage | 任务状态、节点记录 |
| 执行状态 + 融合结果 | SQLite（WAL）或 JSON | executor-svc/state_sink.rs → `data/alliance_tasks.db` / `.json` | 任务级 + 节点级增量落盘；融合输出以保留行 `__fusion_output__` 持久化，进程重启后 `/result`、`/fusion-result` 仍可读回 |
| 专家会话 | 内存 + SQLite | 网关表 sessions / session_messages | load_sessions / save_sessions（`experts_common.rs:626-630`） |
| 知识图谱关联 | **内存 + SQLite** | 网关表 graph_nodes / graph_edges / graph_meta | save_graph 定义于 `experts_common.rs:637-638`、`:499` 有调用点。**本节此前记为「进程内结构」，与代码不符** |
| 专家收藏 favorites | 进程内 | `experts_common.rs`（HashSet） | experts_db.rs 内零命中：**重启即失** |
| 编排计划 plans | 进程内 | `experts_common.rs:468`（`Arc<Mutex<HashMap<String, CollaborationPlan>>>`） | experts_db.rs 无 plans 表：**重启即失**，且无任何 GET 列表端点，计划内容只在 POST 响应里可见 |
| 编排历史 orchestration_history | 进程内 | `experts_common.rs:470`（`Arc<Mutex<Vec<OrchestrationRecord>>>`） | experts_db.rs 无对应表：重启后 GET orchestration/stats 归零、history 返回空 records |

**「进程内」那四行的共同后果**：这些端点返回空列表只证明「本进程内没有」，不证明「从未发生」。界面文案不得把 orchestration/history 的空结果写成"尚无编排记录"这类历史断言——同族的先例是 circuit_breakers 恒为 `[]`（无写侧）与 engine_status 恒为字面量 running。

**注意**：当前无外部 Redis/PostgreSQL/pgvector 依赖，全部嵌入式存储。

**生产部署约定**：调度器与执行器共用环境变量 `MOX_ALLIANCE_STORAGE_MODE`（同一任务真源）。设为 `sqlite` 时执行器具备完整持久化 + 重启恢复（未完成任务重注入、running 节点标记 interrupted、融合结果跨重启可读回）；默认 `file` 仅任务级快照、无恢复能力；`memory` 为纯内存。生产环境应显式设置 `MOX_ALLIANCE_STORAGE_MODE=sqlite`。

---

## 六、API 接口

### 6.1 网关对外（:3080）

| 前缀 | 接口数 | 说明 |
|------|--------|------|
| `/api/experts/*` | 43 | 专家CRUD、会话、协作、查询（2026-09-24 按 actuator.rs ROUTES 的**去重路径**计，忽略动词） |
| `/api/alliance/*` | 20 | 联盟管理、配置、状态（同上口径） |
| `/ws/v1/*` | **0** | 见下方补记：网关侧没有 WebSocket 端点 |

> ⚠️ **V1.1 核对补记（/ws/v1）**：本节原记「/ws/v1/* 2 个 WebSocket 进度推送」。实测网关全 crate 内 `WebSocketUpgrade` **零命中**，actuator.rs ROUTES 里也没有任何 `/ws` 行；仓库里真正实现 WS 的是 system-core/server.rs、orchestrator-svc 的 governance/hitl handler 与 voice-operator-svc，都与联盟无关。连带一处同源错误：`routes.rs:97` 的 DomainDescriptor 把 `/ws/v1` 记为 status=ready 且注明「子域能力，已在 Alliance 域实现」——**联盟域没有实现它**，该描述会随 `scripts/doc/gen-api-registry.py` 流入 `docs/API-REGISTRY.md`。联盟侧的实时性只有 SSE 日志流 `GET /api/alliance/tasks/:id/logs/stream`（见 `DAG-VISUALIZATION-DESIGN.md` 文末对账）。
>
> 计数口径提醒：本表的 43 / 20 是**去重路径**；前端模块台账里的 **74** 是两域 status=ready 的 **(path, method) 行数**（一行一个动词），两者不可互换引用。

### 6.2 调度器内部（:3100）

| 路径 | 方法 | 说明 |
|------|------|------|
| `/health` | GET | 服务探活（进程级，非专家级） |
| `/metrics` | GET | 运行指标 |
| `/leadership` | GET | 多活租约选主状态（`MOX_ALLIANCE_HA_MODE=on` 时才有意义） |
| `/tasks` | POST + GET | 创建任务 / 列出任务 |
| `/tasks/:task_id` | GET + POST | 任务详情 / 任务动作（pause·resume·cancel·retry 走 `handle_task_action`） |
| `/experts/search` | POST | 专家搜索 |

以上即 `scheduler-svc/src/routes.rs:23-32` 注册的全部 6 条路径、9 个动词面。

> ⚠️ **V1.1 核对补记**：本节原表把 `/tasks/:task_id/nodes` 与 `/tasks/:task_id/result` 列为调度器端点，**这两条已在 2026-09 的边界归一化中删除**（routes.rs:28-31 的注释写明理由：调度器只管排队/计划/匹配/执行器桥接，执行状态·节点·融合结果的读路径由网关 :3080 直连执行器 :3200，此前的 HTTP 读代理既与网关重复、又含逐请求 `.expect()` 恐慌点）；同时漏记了实际存在的 `/metrics`、`/leadership` 两条路径和 `/tasks/:task_id` 的 POST 动词。这一处对前端的直接影响：**要拿节点级与融合结果，唯一入口是网关的 `/api/alliance/tasks/:id/...` 一族**，按本表旧值去调调度器 :3100 只会得到 404。

---

### 3.6 专家模型三层架构

| 层 | 位置 | 模型 | 职责 |
|---|------|------|------|
| **配置层** | config-core examples/domain_experts.rs（11 个专家，见 §7.1）+ boot-config experts.rs | 领域专家默认配置 | 种子数据、LLM路由、模块配置 |
| **契约层** | common-proto src/types.rs | Expert(:517) / ExpertStatus(:451) / ExpertHealth(:464) / ExpertModuleConfig(:1161) | 跨服务 gRPC 契约、序列化 |
| **域模型层** | gateway experts_common.rs | ExpertDescriptor | 网关完整域模型：UI展示 + 业务管理 + SQLite持久化 |

**数据流**：config-core种子 → gateway SQLite持久化 → common-proto契约 → scheduler匹配

> ⚠️ **V1.1 核对补记**：本节原写「契约层 = common-proto types.rs 的 Expert / **ExpertCapability** / ExpertStatus」，其中 ExpertCapability **不在契约层**，它是网关域模型里的类型（gateway experts_common.rs:36）；契约层实际有的是 ExpertStatus / ExpertHealth / Expert / ExpertModuleConfig。原表还把配置层路径写成 config-core domain_experts.rs（缺 examples/ 一级，且该 crate 的注释自称「10 大专家」而 vec! 实为 11 条）。这一处的代价很实在：把域模型类型当契约类型引用，跨服务编译能过、语义却对不上。

---

## 七、内置专家（两套，别混）

### 7.1 调度器侧模块目录：11 个 expert-<domain>

来源：`mox-alliance-config-core/src/examples/domain_experts.rs:108-132` 的 build_domain_experts()，返回 **11** 条 ExpertModuleConfig（新增的代码引擎专家被注释编号为「1b」，所以注释序号只到 10）；由 scheduler-svc 的 `server.rs:296` 作为 builtin_modules 装载。

| 注释序号 | id | 领域（源码注释原文） | 注释里的模型指向 |
|---|---|---|---|
| 1 | expert-code | 代码编程专家 | DeepSeek Coder 最强 |
| 1b | expert-code-engine | 自研 AI 代码引擎专家 | mox-codeengine-core 全链路 |
| 2 | expert-math | 数学推理专家 | GPT-4o 严格推理 |
| 3 | expert-medical | 医学咨询专家 | Claude Opus（医学知识丰富） |
| 4 | expert-law | 法律咨询专家 | 法律专业模型 |
| 5 | expert-finance | 金融分析专家 | 金融领域模型 |
| 6 | expert-creative | 创意写作专家 | Claude Opus 长文本 |
| 7 | expert-vision | 图像理解专家 | GPT-4o 视觉 |
| 8 | expert-translation | 翻译专家 | 多语言模型 |
| 9 | expert-research | 学术研究专家 | 深度研究模式 |
| 10 | expert-arch | 架构设计专家 | 混合专家 |

模型名在这张表里只作「注释原文」引用，不作为字段结论——每个专家真正下发给 LLM 的模型取自各自 build_*_expert() 的配置字段，本节未逐项核对。

### 7.2 网关运行态种子：10 位具名专家 exp-*-001

来源：网关 `experts_common.rs:645-660` 的 seed_builtin_experts()，在 `:489` 于注册表为空时播种。**`GET /api/experts` 列表与专家排行榜读到的就是这一套**，与 7.1 的 id 零交集：

exp-architecture-001 架构师·玄枢 ｜ exp-ai-001 AI算法·灵玑 ｜ exp-data-001 数据工程·衡宇 ｜ exp-security-001 安全专家·镇岳 ｜ exp-cloud-001 云原生·凌霄 ｜ exp-product-001 产品战略·明鉴 ｜ exp-frontend-001 前端工程·织锦 ｜ exp-math-001 数学建模·璇玑 ｜ exp-finance-001 金融量化·泉通 ｜ exp-enterprise-001 企业架构·鼎元

> ⚠️ **V1.1 核对补记**：本节原表 10 行中有 **7 行**（expert-data / expert-ai / expert-security / expert-flow / expert-governance / expert-fusion / expert-alliance）在 7.1 与 7.2 两处真实来源里都不存在——那张表是把 7.1 的命名前缀（expert-）和 7.2 的领域名（数据 / AI / 安全 / 流程 / 治理 / 融合 / 联盟）拼出来的第三套，且「主模型」列除前三行外无出处。按「一个事实一个权威源」：谈**模块目录**用 7.1 的坐标，谈**界面上能看到的专家**用 7.2 的坐标。

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
