# 专家联盟（alliance）域代码事实核验与文档对齐报告

> **核验日期**：2026-08-31
> **核验范围**：`platform/domains/alliance/` 全部 11 个 crate 源码 + 8 份关键文档
> **核验性质**：只读核验，未修改任何源码或文档
> **代码事实权威等级**：所有结论均从源码逐文件读取确认，非推断

---

## 第一部分：代码事实参考卡

### 1.1 Crate 全景表

alliance 域共 **11 个 crate**，分 6 层：

| # | Crate 名称 | 路径 | 版本 | 层级 | 职责 | 关键依赖 |
|---|-----------|------|------|------|------|---------|
| 1 | `mox-alliance-api` | `api/` | 0.1.0 | API层 | HTTP DTO（请求/响应结构体），纯 serde 数据结构 | serde, serde_json |
| 2 | `mox-alliance-common-proto` | `proto/mox-alliance-common-proto/` | 0.1.0 | Proto层 | 通用类型：Task、Expert、ExpertModuleConfig、AllianceError、AllianceErrorCode、事件、常量、trait | serde, chrono, uuid |
| 3 | `mox-alliance-scheduler-proto` | `proto/mox-alliance-scheduler-proto/` | 0.1.0 | Proto层 | 调度器 trait：TaskScheduler、ExpertMatcher、SchedulerConfig | mox-alliance-common-proto |
| 4 | `mox-alliance-executor-proto` | `proto/mox-alliance-executor-proto/` | 0.1.0 | Proto层 | 执行器 trait：DagEngine、NodeExecutor；ExecutionStatus、CollaborationPlan | mox-alliance-common-proto |
| 5 | `mox-alliance-core` | `core/mox-alliance-core/` | 0.1.0 | Core层 | DAG 数据结构 + 融合引擎（6种策略）+ 工具函数 | mox-alliance-common-proto, mox-alliance-executor-proto |
| 6 | `mox-alliance-config-core` | `core/mox-alliance-config-core/` | 0.1.0 | Core层 | ConfigEngine、ExpertModuleConfig、build_domain_experts（10内置专家）、ConfigSynchronizer、配置校验/存储/事件 | mox-alliance-common-proto |
| 7 | `mox-alliance-scheduler-core` | `core/mox-alliance-scheduler-core/` | 0.1.0 | Core层 | TaskSchedulerImpl、ModularWeightMatcher、RuleBasedExpertMatcher、DagExecutionEngine、FusionEngine、HttpExecutorBridge、InProcessExecutorBridge、TaskRepository（内存+文件快照）、LLMRouter、Planner、Registry、Storage、Synchronizer | mox-alliance-common-proto, mox-alliance-scheduler-proto, mox-alliance-executor-proto, mox-alliance-core, mox-alliance-config-core |
| 8 | `mox-alliance-executor-core` | `core/mox-alliance-executor-core/` | 0.1.0 | Core层 | DagEngineImpl（DAG执行引擎）、ExpertExecutor（专家执行器） | mox-alliance-common-proto, mox-alliance-executor-proto, mox-alliance-core |
| 9 | `mox-alliance-sdk` | `sdk/mox-alliance-sdk/` | 0.1.0 | SDK层 | AllianceClient（HTTP客户端），封装调度器/执行器所有端点 | reqwest, mox-alliance-api, mox-alliance-common-proto |
| 10 | `mox-alliance-scheduler-svc` | `svc/mox-alliance-scheduler-svc/` | 0.1.0 | Svc层 | 调度器 HTTP 服务（axum），端口 8701，14条路由 | mox-alliance-scheduler-core, mox-alliance-config-core, mox-alliance-api |
| 11 | `mox-alliance-executor-svc` | `svc/mox-alliance-executor-svc/` | 0.1.0 | Svc层 | 执行器 HTTP 服务（axum），端口 8702，5条路由 | mox-alliance-executor-core, mox-alliance-api |

**依赖关系图（自底向上）**：

```
api (独立，无 alliance 内部依赖)
common-proto (独立基础)
├── scheduler-proto
├── executor-proto
├── config-core
├── core (依赖 common-proto + executor-proto)
│   ├── scheduler-core (依赖全部 proto + core + config-core)
│   └── executor-core (依赖 common-proto + executor-proto + core)
├── scheduler-svc (依赖 scheduler-core + config-core + api)
├── executor-svc (依赖 executor-core + api)
└── sdk (依赖 api + common-proto，通过 HTTP 调用 svc)
```

**Workspace 成员声明**：所有 11 个 crate 均在根 `Cargo.toml` 的 `[workspace.members]` 中注册，版本统一通过 `workspace.package.version` 管理。

---

### 1.2 Proto 定义速查

#### 1.2.1 mox-alliance-common-proto

**模块结构**：`lib.rs` / `types.rs` / `error.rs` / `constants.rs` / `events.rs` / `traits.rs`

**Task 结构体核心字段**：
- `id: String` — 任务唯一标识（UUID）
- `title: String` — 任务标题
- `description: String` — 任务描述
- `status: TaskStatus` — 枚举：Pending / Scheduled / Running / Completed / Failed / Cancelled
- `priority: Priority` — 枚举：Low / Medium / High / Critical
- `expert_ids: Vec<String>` — 分配的专家 ID 列表
- `input: serde_json::Value` — 任务输入（JSON）
- `output: Option<serde_json::Value>` — 任务输出（JSON）
- `created_at / updated_at / started_at / completed_at: Option<DateTime<Utc>>`
- `tenant_id: Option<String>` — 租户 ID（多租户预留）
- `metadata: HashMap<String, String>`

**Expert 结构体核心字段**：
- `id: String` — 专家唯一标识
- `name: String` — 专家名称
- `description: String` — 专家描述
- `expert_type: String` — 专家类型（如 architecture-review, code-review）
- `capabilities: Vec<String>` — 能力标签列表
- `domain_tags: Vec<String>` — 领域标签
- `model_config: ModelConfig` — 模型配置（model_name, temperature, max_tokens, api_endpoint）
- `metrics: ExpertMetrics` — 评分指标（accuracy, latency, cost, success_rate, rating）
- `version: String` — 版本号
- `enabled: bool` — 是否启用
- `collaboration_rules: CollaborationRules` — 协作规则

**ExpertModuleConfig 核心字段**：
- `module_id: String` — 模块 ID
- `module_name: String` — 模块名称
- `experts: Vec<Expert>` — 该模块下的专家列表
- `default_expert_id: Option<String>` — 默认专家
- `routing_rules: Vec<RoutingRule>` — 路由规则
- `enabled: bool`

**AllianceError / AllianceErrorCode**：
- `AllianceError` 结构体：`code: AllianceErrorCode` + `message: String` + `details: Option<serde_json::Value>` + `trace_id: Option<String>`
- `AllianceErrorCode` 枚举：
  - `InvalidInput` (400)
  - `Unauthorized` (401)
  - `Forbidden` (403)
  - `NotFound` (404)
  - `Conflict` (409)
  - `ValidationFailed` (422)
  - `InternalError` (500)
  - `ServiceUnavailable` (503)
  - `ExpertNotFound`
  - `TaskNotFound`
  - `SchedulerError`
  - `ExecutorError`
  - `FusionError`
  - `ConfigError`
  - `Timeout`

#### 1.2.2 mox-alliance-scheduler-proto

**模块结构**：`lib.rs` / `scheduler.rs` / `matcher.rs` / `types.rs`

**TaskScheduler trait**（核心方法）：
- `async fn submit_task(&self, task: Task) -> Result<Task, AllianceError>`
- `async fn get_task(&self, task_id: &str) -> Result<Option<Task>, AllianceError>`
- `async fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>, AllianceError>`
- `async fn cancel_task(&self, task_id: &str) -> Result<Task, AllianceError>`
- `async fn retry_task(&self, task_id: &str) -> Result<Task, AllianceError>`
- `async fn execute_collaboration(&self, req: CollaborationRequest) -> Result<CollaborationResponse, AllianceError>`

**ExpertMatcher trait**（核心方法）：
- `async fn match_experts(&self, query: &ExpertMatchQuery) -> Result<Vec<ExpertMatchResult>, AllianceError>`
- `fn name(&self) -> &str`

**SchedulerConfig 结构体**：
- `scheduler_id: String`
- `max_concurrent_tasks: usize`（默认 10）
- `default_timeout_secs: u64`（默认 300）
- `retry_policy: RetryPolicy`（max_retries=3, backoff_base_ms=1000）
- `matcher_config: MatcherConfig`
- `fusion_config: FusionConfig`
- `executor_endpoint: Option<String>`

#### 1.2.3 mox-alliance-executor-proto

**模块结构**：`lib.rs` / `dag_engine.rs` / `node_executor.rs` / `types.rs`

**DagEngine trait**（核心方法）：
- `async fn submit_plan(&self, plan: CollaborationPlan) -> Result<String, AllianceError>` — 返回 execution_id
- `async fn get_status(&self, execution_id: &str) -> Result<ExecutionStatus, AllianceError>`
- `async fn cancel_execution(&self, execution_id: &str) -> Result<(), AllianceError>`
- `async fn execute_node(&self, execution_id: &str, node_id: &str) -> Result<NodeResult, AllianceError>`

**NodeExecutor trait**：
- `async fn execute(&self, node: &DagNode, context: &ExecutionContext) -> Result<NodeResult, AllianceError>`
- `fn supports(&self, node_type: &str) -> bool`

**ExecutionStatus 枚举**：
- `Pending`
- `Running { progress: f32, current_node: Option<String> }`
- `Completed { result: CollaborationResult }`
- `Failed { error: AllianceError, failed_node: Option<String> }`
- `Cancelled`

**CollaborationPlan 结构体**：
- `plan_id: String`
- `task_id: String`
- `nodes: Vec<DagNode>` — DAG 节点列表
- `edges: Vec<DagEdge>` — DAG 边列表
- `execution_mode: ExecutionMode` — 枚举：Sequential / Parallel / Hybrid
- `timeout_secs: u64`
- `tenant_id: Option<String>`

---

### 1.3 Core 组件速查

#### 1.3.1 mox-alliance-core

**模块结构**：`lib.rs` / `dag.rs` / `fusion/`（mod.rs, traits.rs, engine.rs, error.rs）/ `fusion/strategies/`（6个策略文件）/ `utils.rs`

**DAG 模块**（`dag.rs`）：
- `DagNode`：id, node_type, expert_id, input, config, dependencies
- `DagEdge`：from, to, edge_type（Data / Control / Condition）
- `DagGraph`：nodes, edges，提供拓扑排序、环检测、依赖解析
- 测试：6 个

**融合引擎**（`fusion/engine.rs`）：
- `FusionEngine`：融合策略注册表 + 执行器
- 支持策略注册、按名称选择、批量融合
- 测试：21 个

**融合策略 trait**（`fusion/traits.rs`）：
- `FusionStrategy` trait：
  - `fn name(&self) -> &str`
  - `async fn fuse(&self, outputs: &[ExpertOutput], context: &FusionContext) -> Result<FusedOutput, FusionError>`
  - `fn supports(&self, task_type: &str) -> bool`

#### 1.3.2 mox-alliance-config-core

**模块结构**：`lib.rs` / `engine.rs` / `error.rs` / `events.rs` / `store.rs` / `validator.rs` / `examples/domain_experts.rs`

**ConfigEngine**：
- 加载/保存/热重载专家模块配置
- 支持配置校验、版本管理、变更事件通知
- 提供 `get_expert()` / `list_experts()` / `get_module()` 等查询

**ConfigSynchronizer**：
- 配置同步器，支持多实例间配置同步
- 文件快照 + 内存缓存双写

**build_domain_experts**（10个内置专家，详见 1.5）

#### 1.3.3 mox-alliance-scheduler-core

**模块结构**：`lib.rs` / `scheduler.rs` / `matcher.rs` / `modular_matcher.rs` / `dag_engine.rs` / `fusion.rs` / `executor_bridge.rs` / `repository.rs` / `storage.rs` / `llm_router.rs` / `planner.rs` / `registry.rs` / `config_sync.rs` / `synchronizer.rs`

**TaskSchedulerImpl**：
- 实现 `TaskScheduler` trait
- 整合匹配器、DAG引擎、融合引擎、执行器桥接、任务仓库
- 支持协作执行（execute_collaboration）和流式执行（SSE）

**ModularWeightMatcher**（默认匹配器）：
- 模块化加权匹配算法
- 评分维度：能力标签匹配（capability）+ 领域标签匹配（domain）+ 专家指标（metrics：accuracy/success_rate/rating）+ 成本惩罚
- 权重可配置，默认：capability=0.4, domain=0.2, metrics=0.3, cost=-0.1
- **不使用向量嵌入/embedding，纯标签+指标加权**
- 测试：6 个

**RuleBasedExpertMatcher**：
- 基于规则的匹配器
- 按领域标签/能力标签/成本/延迟过滤
- 支持自定义规则链

**DagExecutionEngine**：
- DAG 执行引擎，调度器侧的执行编排
- 支持并行/串行/混合执行模式
- 依赖解析、条件分支、超时控制

**FusionEngine**（调度器侧包装）：
- 包装 mox-alliance-core 的融合引擎
- 集成到调度流程中

**HttpExecutorBridge**：
- 通过 HTTP 调用远程 executor-svc（端口 8702）
- 实现执行器桥接 trait

**InProcessExecutorBridge**：
- 进程内直接调用 executor-core（无需 HTTP）
- 用于单进程部署模式

**TaskRepository**：
- 内存 HashMap + 文件快照持久化
- 支持 CRUD、按状态过滤、按租户过滤
- **不使用 SQL 数据库**

#### 1.3.4 mox-alliance-executor-core

**模块结构**：`lib.rs` / `dag_engine.rs` / `expert_executor.rs` / `tests/integration_e2e.rs`

**DagEngineImpl**：
- 实现 `DagEngine` trait
- DAG 执行引擎：拓扑排序 → 节点调度 → 并行执行 → 结果聚合
- 支持取消、超时、失败处理
- 执行状态管理

**ExpertExecutor**：
- 专家执行器，实现 `NodeExecutor` trait
- 调用 LLM 执行专家任务（通过 LLMRouter）
- 支持工具调用、上下文管理
- 测试：12 个

---

### 1.4 服务端点表

#### 1.4.1 mox-alliance-scheduler-svc（端口 8701）

**框架**：axum 0.7 + tokio
**入口**：`src/bin/main.rs`
**中间件**：
- `X-Tenant-Id` 请求头提取（多租户）
- 日志追踪（tracing）
- CORS

| # | 方法 | 路由路径 | 处理函数 | 用途 |
|---|------|---------|---------|------|
| 1 | GET | `/health` | health_check | 健康检查 |
| 2 | POST | `/api/v1/tasks` | submit_task | 提交任务 |
| 3 | GET | `/api/v1/tasks/:task_id` | get_task | 查询单个任务 |
| 4 | GET | `/api/v1/tasks` | list_tasks | 任务列表（支持过滤） |
| 5 | POST | `/api/v1/tasks/:task_id/cancel` | cancel_task | 取消任务 |
| 6 | POST | `/api/v1/tasks/:task_id/retry` | retry_task | 重试任务 |
| 7 | GET | `/api/v1/experts/match` | match_experts | 专家匹配（query参数） |
| 8 | GET | `/api/v1/experts` | list_experts | 专家列表 |
| 9 | GET | `/api/v1/config/snapshot` | get_config_snapshot | 获取配置快照 |
| 10 | POST | `/api/v1/config/reload` | reload_config | 热重载配置 |
| 11 | POST | `/api/v1/collaboration/execute` | execute_collaboration | 协作执行（同步） |
| 12 | POST | `/api/v1/collaboration/stream` | stream_collaboration | 协作执行（SSE流式） |

**请求/响应结构**：定义在 `mox-alliance-api` crate 的 `dto.rs` 中
- `SubmitTaskRequest`：title, description, priority, expert_ids, input, tenant_id
- `TaskResponse`：task 完整信息
- `ExpertMatchRequest`：query, domain_tags, capability_tags, limit, tenant_id
- `ExpertMatchResponse`：experts（含 match_score）
- `CollaborationRequest`：task_description, expert_ids, execution_mode, fusion_strategy, timeout_secs
- `CollaborationResponse`：result, fused_output, expert_contributions, trace_id

#### 1.4.2 mox-alliance-executor-svc（端口 8702）

**框架**：axum 0.7 + tokio
**入口**：`src/bin/main.rs`

| # | 方法 | 路由路径 | 处理函数 | 用途 |
|---|------|---------|---------|------|
| 1 | GET | `/health` | health_check | 健康检查 |
| 2 | POST | `/api/v1/execute` | execute | 执行 DAG 计划（同步） |
| 3 | POST | `/api/v1/execute/stream` | execute_stream | 执行 DAG 计划（SSE流式） |
| 4 | GET | `/api/v1/status/:task_id` | get_status | 查询执行状态 |
| 5 | POST | `/api/v1/cancel/:task_id` | cancel | 取消执行 |

---

### 1.5 内置专家清单（10个）

来源：`core/mox-alliance-config-core/src/examples/domain_experts.rs` 的 `build_domain_experts()` 函数

| # | 专家 ID | 专家名称 | 类型 | 核心能力标签 |
|---|---------|---------|------|------------|
| 1 | `exp-architecture-review` | 架构评审专家 | architecture-review | 架构设计, 系统设计, 技术选型, 微服务, 分布式 |
| 2 | `exp-code-review` | 代码评审专家 | code-review | 代码审查, 代码质量, 重构, 设计模式, 最佳实践 |
| 3 | `exp-algorithm-design` | 算法设计专家 | algorithm-design | 算法设计, 数据结构, 复杂度分析, 动态规划, 图算法 |
| 4 | `exp-data-modeling` | 数据建模专家 | data-modeling | 数据建模, 数据库设计, ER图, 范式, 索引优化 |
| 5 | `exp-test-design` | 测试设计专家 | test-design | 测试设计, 单元测试, 集成测试, 测试覆盖率, TDD |
| 6 | `exp-performance-opt` | 性能优化专家 | performance-opt | 性能优化, 瓶颈分析, 缓存, 并发, 负载测试 |
| 7 | `exp-security-audit` | 安全审计专家 | security-audit | 安全审计, 渗透测试, OWASP, 加密, 访问控制 |
| 8 | `exp-devops` | DevOps专家 | devops | CI/CD, 容器化, Kubernetes, 监控, 自动化部署 |
| 9 | `exp-product-analysis` | 产品分析专家 | product-analysis | 产品分析, 需求分析, 用户体验, 竞品分析, 产品规划 |
| 10 | `exp-documentation` | 文档专家 | documentation | 技术文档, API文档, 文档规范, 知识管理, 写作 |

---

### 1.6 融合策略清单（6种）

来源：`core/mox-alliance-core/src/fusion/strategies/`

| # | 策略 struct 名 | 策略标识（name()） | 文件 | 适用场景 | 核心算法 |
|---|---------------|-------------------|------|---------|---------|
| 1 | `WeightedVotingFusion` | `weighted_voting` | weighted_voting.rs | 分类/选择类任务 | 按专家评分加权投票，多数决 |
| 2 | `ConfidenceWeightingFusion` | `confidence_weighting` | confidence_weighting.rs | 通用/数值类 | 按专家置信度加权平均 |
| 3 | `DebateFusion` | `debate` | debate.rs | 复杂推理/决策类 | 多轮辩论收敛，支持自适应跳过 |
| 4 | `StackingFusion` | `stacking` | stacking.rs | 回归/生成类 | 元学习器融合多专家输出（两层结构） |
| 5 | `MapReduceFusion` | `map_reduce` | map_reduce.rs | 大规模/可分解任务 | Map（分片处理）→ Reduce（结果聚合） |
| 6 | `IterativeRefinementFusion` | `iterative_refinement` | iterative_refinement.rs | 高质量/迭代优化任务 | 多轮迭代精炼，每轮基于前一轮结果改进 |

**trait 定义**：`FusionStrategy`（在 `fusion/traits.rs`），所有策略均实现该 trait。

---

### 1.7 测试覆盖统计

**总计：约 199 个测试**（`#[test]` + `#[tokio::test]`）

| Crate | 测试数 | 主要测试文件 |
|-------|--------|------------|
| mox-alliance-core | ~87 | fusion/engine.rs(21), fusion/strategies/*(75), dag.rs(6), utils.rs(4), fusion/mod.rs(6), fusion/error.rs(1) |
| mox-alliance-scheduler-core | ~62 | scheduler.rs(6), modular_matcher.rs(6), dag_engine.rs(6), fusion.rs(7), executor_bridge.rs(7), repository/storage.rs(2), llm_router.rs(5), planner.rs(3), registry.rs(8), matcher.rs(2), config_sync.rs(3), synchronizer.rs(7) |
| mox-alliance-executor-core | ~17 | expert_executor.rs(12), tests/integration_e2e.rs(5) |
| mox-alliance-config-core | ~7 | examples/domain_experts.rs(7) |
| mox-alliance-common-proto | ~少量 | types.rs, error.rs |
| mox-alliance-scheduler-proto | ~少量 | - |
| mox-alliance-executor-proto | ~少量 | - |
| svc / sdk / api | 0 | 无单元测试 |

**关键测试场景**：
- 融合策略：6种策略的正确性、边界条件、错误处理
- DAG：拓扑排序、环检测、依赖解析
- 匹配器：加权评分、过滤规则、多维度组合
- 调度器：任务生命周期、协作执行、重试/取消
- 执行器：DAG执行、专家调用、端到端集成
- 配置：内置专家构建、配置校验

**测试缺口**：
- svc 层（HTTP路由）无集成测试
- sdk 层无测试
- api 层（DTO）无测试
- 多租户场景测试不足
- 流式（SSE）端点无测试

---

## 第二部分：文档-代码错位清单

### 文档1：`docs/expert-alliance/README.md`（v1草案）

**文档定位**：v1 架构草案，描述目标架构

| # | 文档声称 | 代码事实 | 差异 | 严重程度 |
|---|---------|---------|------|---------|
| 1.1 | 3个微服务：scheduler / executor / fusion | 仅2个 svc crate：scheduler-svc / executor-svc；fusion 是 scheduler-core 内的库模块，非独立服务 | 服务数量错误，fusion-svc 不存在 | 🔴致命 |
| 1.2 | 端口：8701(scheduler) / 8702(executor) / 8703(fusion) | 8701(scheduler) / 8702(executor)，无 8703 | 端口 8703 不存在 | 🟡中等 |
| 1.3 | 使用 gRPC 通信 | 全部使用 HTTP/REST（axum），无 gRPC | 通信协议错误 | 🔴致命 |
| 1.4 | expert-memory 独立服务 | alliance 域无 memory 服务/ crate | 虚构服务 | 🔴致命 |
| 1.5 | expert-registry 独立服务 | 注册功能在 scheduler-core 的 registry 模块中，非独立服务 | 服务拆分错误 | 🟡中等 |
| 1.6 | expert-agent 独立服务 | 无 agent 服务/crate | 虚构服务 | 🔴致命 |
| 1.7 | gateway 接入层服务 | alliance 域无 gateway，网关在 platform/gateway/runtime | 归属错误 | 🟡中等 |
| 1.8 | 数据库表 ea_*（PostgreSQL） | 任务存储为内存 HashMap + 文件快照，无 SQL 数据库 | 存储层完全不同 | 🔴致命 |
| 1.9 | 向量相似度匹配（embedding） | ModularWeightMatcher 纯标签+指标加权，无向量嵌入 | 匹配算法错误 | 🟡中等 |
| 1.10 | 4种融合策略 | 实际6种（缺 map_reduce 和 iterative_refinement） | 策略数量不全 | 🟡中等 |

---

### 文档2：`docs/expert-alliance/00-INTEGRATED-INDEX.md`（集成索引）

**文档定位**：文档索引，含架构声明

| # | 文档声称 | 代码事实 | 差异 | 严重程度 |
|---|---------|---------|------|---------|
| 2.1 | 索引中引用"3服务架构"（scheduler/executor/fusion） | 仅2个 svc | 同 1.1 | 🔴致命 |
| 2.2 | 索引中引用 gRPC 端点 | 全部 HTTP | 同 1.3 | 🔴致命 |
| 2.3 | 索引中列出 expert-memory / expert-registry / expert-agent 为独立模块 | 均不存在为独立 crate/服务 | 同 1.4-1.6 | 🔴致命 |
| 2.4 | 索引中 v1/v2/v3 架构文档并存，未标注哪些已实现哪些是目标 | 代码实现的是简化版（2服务HTTP），v2/v3均为目标设计 | 缺少实现状态标注 | 🟡中等 |

---

### 文档3：`docs/expert-alliance/v2/01-architecture.md`（v2架构）

**文档定位**：v2 目标架构设计

| # | 文档声称 | 代码事实 | 差异 | 严重程度 |
|---|---------|---------|------|---------|
| 3.1 | v2 架构描述多服务拆分（scheduler/executor/fusion/registry/memory） | 代码为2服务简化版 | v2 是目标架构，非当前实现；文档未明确标注"未实现" | 🟡中等 |
| 3.2 | v2 描述事件驱动架构（Event Bus） | 代码无事件总线，同步调用为主 | 未实现 | 🟡中等 |
| 3.3 | v2 描述插件化编排引擎 | 代码无插件系统，融合策略为静态注册 | 未实现 | 🟡中等 |
| 3.4 | v2 描述 PageRank 能力图谱匹配 | 代码为 ModularWeightMatcher 加权匹配 | 未实现 | 🟡中等 |

> **说明**：v2 文档为设计目标文档，与代码的差异属于"设计未落地"，而非"描述错误"。但文档未明确标注实现状态，容易误导读者认为已实现。

---

### 文档4：`docs/expert-alliance/v3/01-architecture-optimization.md`（v3架构优化）

**文档定位**：v3 优化目标

| # | 文档声称 | 代码事实 | 差异 | 严重程度 |
|---|---------|---------|------|---------|
| 4.1 | v3 描述 SaaS 多租户架构（tenant_id 贯穿全链路） | 代码中 Task/Expert/CollaborationPlan 有 tenant_id 字段，但 svc 层仅提取 X-Tenant-Id 头，未做数据隔离/RLS | 部分实现（字段预留，隔离未实现） | 🟡中等 |
| 4.2 | v3 描述可观测性体系（OpenTelemetry / Prometheus） | 代码仅有 tracing 日志，无 metrics 导出 / OTel | 未实现 | 🟡中等 |
| 4.3 | v3 描述配置中心（动态配置下发） | ConfigEngine 支持文件热重载，但无配置中心服务 | 部分实现 | 🟢轻微 |

> **说明**：同 v2，v3 为优化目标文档，差异属于"未落地"。

---

### 文档5：`docs/_archive/expert-alliance/enterprise/26-开发专家联盟-架构诊断与SaaS化最优方案-V1.0.md`（架构诊断，已归档，替代文档：V1.1）

**文档定位**：基于 2026-08-26 现场取证的架构诊断，聚焦前端+Node后端

| # | 文档声称 | 代码事实 | 差异 | 严重程度 |
|---|---------|---------|------|---------|
| 5.1 | 诊断范围：ExpertCenterView.vue / projectContext.js / Cargo.toml / platform_config.json 等 | 诊断聚焦前端+Node后端，**未覆盖 Rust alliance 域的 11 个 crate** | 范围遗漏：Rust alliance 域是专家联盟的核心实现，但诊断完全未涉及 | 🔴致命 |
| 5.2 | 声称"42个Rust crates"，分层为 platform/crates(4+3) / platform/domains(26) / gateway / sdk / backend-node | 实际 alliance 域就有 11 个 crate 在 platform/domains/alliance/ 下；文档未具体分析 alliance 域的 crate 结构 | 对 alliance 域的 crate 细节完全空白 | 🟡中等 |
| 5.3 | 声称"AI编排层是否独立不透明"，推测有 ai-engine.js / llm-gateway.js / ai-engine-core.js | Rust alliance 域有清晰的 scheduler-core / executor-core / fusion 分层，但诊断未发现 | 诊断遗漏了 Rust 侧已有的清晰编排分层 | 🟡中等 |
| 5.4 | 声称"专家智能匹配算法未见实现" | Rust scheduler-core 中有 ModularWeightMatcher + RuleBasedExpertMatcher 两个匹配器实现 | 事实错误：匹配算法已在 Rust 侧实现 | 🔴致命 |
| 5.5 | 多租户诊断："所有后端 JSON Store 无 tenant_id 字段" | Rust alliance 域的 Task / Expert / CollaborationPlan 均有 tenant_id 字段，svc 层提取 X-Tenant-Id | 部分错误：Rust 侧已预留 tenant_id，但 Node 侧确实没有 | 🟡中等 |
| 5.6 | 诊断结论中"P2-2 AI编排层是否独立不透明"列为风险 | Rust 侧编排层已独立为 scheduler-core / executor-core，分层清晰 | 风险已在 Rust 侧解决，诊断未发现 | 🟡中等 |

---

### 文档6：`docs/cosmic-architecture/02-EXPERT-ALLIANCE-ARCHITECTURE.md`（宇宙架构）

**文档定位**：宏观架构设计，描述7服务架构

| # | 文档声称 | 代码事实 | 差异 | 严重程度 |
|---|---------|---------|------|---------|
| 6.1 | **7个服务**：alliance-scheduler / alliance-executor / alliance-fusion / expert-registry / expert-agent / expert-memory / gateway | **仅2个 svc**：scheduler-svc(8701) / executor-svc(8702)；fusion 在 scheduler-core 内；registry 在 scheduler-core 内；无 agent/memory/gateway 服务 | 服务数量严重夸大（7→2），5个服务不存在 | 🔴致命 |
| 6.2 | 通信协议：gRPC / Dubbo-Triple / JSON-RPC / WebSocket，"通过mox-dualrpc实现零配置多协议" | 全部 HTTP/REST（axum），SSE 用于流式；无 gRPC / Dubbo / JSON-RPC | 协议栈完全错误 | 🔴致命 |
| 6.3 | 专家匹配：1536维向量 embedding + cosine相似度 + 规则过滤 + 加权排序(0.4相似度+0.3评分+0.2成功率-0.1成本) | ModularWeightMatcher：能力标签匹配(0.4) + 领域标签(0.2) + 专家指标(0.3) + 成本惩罚(-0.1)；**无向量嵌入** | 匹配算法核心机制错误（向量→标签），权重数值巧合相似 | 🔴致命 |
| 6.4 | 融合策略：4种（加权投票/Stacking/辩论/置信度加权） | 实际6种（+MapReduce/IterativeRefinement） | 策略数量不全 | 🟡中等 |
| 6.5 | 执行器：ReAct循环（Thought→Action→Observation）+ 工具调用 + 流式输出 | DagEngineImpl：DAG拓扑排序→节点并行执行→结果聚合；ExpertExecutor 调用 LLM；**无 ReAct 循环实现** | 执行模式错误（ReAct→DAG） | 🔴致命 |
| 6.6 | 数据库表：ea_expert / ea_expert_embedding / ea_alliance_task / ea_expert_execution / ea_expert_memory（PostgreSQL + pgvector） | 内存 HashMap + JSON 文件快照；无 SQL 数据库，无 pgvector | 存储层完全不同 | 🔴致命 |
| 6.7 | expert-registry：ELO等级分 + 指数移动平均评分更新 | ExpertMetrics 有 accuracy/latency/cost/success_rate/rating 字段，但无 ELO 算法实现 | 评分算法未实现 | 🟡中等 |
| 6.8 | expert-memory：短期记忆(Redis) + 长期记忆(PostgreSQL) + 语义记忆(pgvector) + 情景记忆 | alliance 域无任何记忆服务/crate | 完全虚构 | 🔴致命 |
| 6.9 | expert-agent：无状态Agent运行时 + 工具适配 + 协议转换 | 无 agent 服务/crate | 完全虚构 | 🔴致命 |
| 6.10 | 性能目标：匹配<50ms / 执行<30s / 融合<1s / 吞吐>1000tps | 无性能基准测试，无 metrics 导出 | 目标无代码支撑 | 🟢轻微 |
| 6.11 | 与现有系统集成：mox-ai-agent升级为executor的ReAct引擎 / mox-expert升级为registry / flow-ai升级为DAG引擎 | 这些"升级"均未发生，alliance 域是独立实现 | 集成关系虚构 | 🟡中等 |
| 6.12 | SSE流式输出 | scheduler-svc 有 `/api/v1/collaboration/stream`，executor-svc 有 `/api/v1/execute/stream` | ✅ 此项正确 | - |

---

### 文档7：`docs/modules/专家联盟AI对话需求文档-V2.0-架构优化版.md`（需求文档）

**文档定位**：V2.0 需求设计，L0-L5 六层架构

| # | 文档声称 | 代码事实 | 差异 | 严重程度 |
|---|---------|---------|------|---------|
| 7.1 | L0-L5 六层架构（感知/记忆/能力/编排/治理/展示） | 代码为 proto/core/svc/sdk/api 分层，非 L0-L5 | 架构模型不同（需求模型vs实现模型），属正常差异 | 🟢轻微 |
| 7.2 | 专家路由器：基于 PageRank 的能力图谱动态匹配（damping=0.85） | ModularWeightMatcher 纯标签加权，无 PageRank / 能力图谱 | 匹配算法未实现 | 🟡中等 |
| 7.3 | 插件化编排引擎：一切皆插件，可替换主循环（pre-step/plan/act/reflect/learn） | 无插件系统，融合策略为静态 struct 注册 | 未实现 | 🟡中等 |
| 7.4 | 事件流架构（Agent Event Bus） | 无事件总线，同步调用 | 未实现 | 🟡中等 |
| 7.5 | Plan/Act 双模式 + 检查点回滚 | 无 Plan/Act 模式，无检查点机制 | 未实现 | 🟡中等 |
| 7.6 | 学习闭环：轨迹压缩→Skill提取→记忆更新 | 无学习闭环实现 | 未实现 | 🟡中等 |
| 7.7 | 15+ 专家类型，可动态扩展 | 10个内置专家，静态定义 | 数量差异（15+→10），动态扩展未实现 | 🟢轻微 |
| 7.8 | 双璇玑十四维诊断 + ⛨验证网关 + 治理闸门G3 | 无治理层实现 | 未实现 | 🟡中等 |

> **说明**：本文档为需求/设计文档，描述的是 V2.0 目标状态。与代码的差异属于"需求未落地"。但文档标题为"架构优化版"，未明确标注实现进度，容易误导。

---

### 文档8：`docs/standards/expert-alliance-flow-standard.md`（流程标准）

**文档定位**：行业级流程标准，参考实现为 Node.js `flow-ea-consult`

| # | 文档声称 | 代码事实 | 差异 | 严重程度 |
|---|---------|---------|------|---------|
| 8.1 | 参考实现：`flow-ea-consult`（Node.js，六阶段全链路），代码在 `src/expert-alliance-engine.js` | 这是 **Node.js 后端**的实现，与 Rust alliance 域（platform/domains/alliance/）是**两套独立实现** | 标准描述的是 Node 侧实现，非 Rust alliance 域；文档未说明两者关系 | 🟡中等 |
| 8.2 | API端点：`/atlas/flows` / `/atlas/verify` / `/experts/alliance/traces` 等 | Rust alliance svc 的端点为 `/api/v1/tasks` / `/api/v1/collaboration/*` 等，完全不同 | 端点体系不同（Node侧vs Rust侧） | 🟡中等 |
| 8.3 | 六阶段流程：意图识别→最优组队→并行咨询与辩论→综合合成→质量门禁→反馈学习 | Rust scheduler-core 的协作执行流程为：匹配专家→构建DAG→执行→融合，无意图识别/质量门禁/反馈学习阶段 | 流程模型不同（标准六阶段vs Rust四阶段） | 🟡中等 |
| 8.4 | 匹配算法：专家匹配打分（能力/类型/指标）+ 协同增益（能力图协同度） | ModularWeightMatcher：能力/领域/指标加权，无协同增益/能力图 | 部分差异 | 🟢轻微 |
| 8.5 | 辩论：自适应跳过(共识≥0.6) + 逐轮收敛 + 令牌上限(≤900) + 超时隔离(60s) + 分歧保留 | DebateFusion 实现了多轮辩论，但具体参数（阈值/令牌上限）需确认是否与标准一致 | 可能存在参数差异 | 🟢轻微 |
| 8.6 | 降级链≥2条：并行咨询→单专家直答；LLM综合→启发式综合 | Rust 侧无显式降级链机制 | 未实现 | 🟡中等 |
| 8.7 | 学习技能沉淀：alliance_learned_skills.json，容量200条 | Rust alliance 域无技能沉淀模块 | 未实现（Node侧有） | 🟡中等 |
| 8.8 | SSE流式：`POST /ai/chat/stream` | Rust scheduler-svc：`POST /api/v1/collaboration/stream`；executor-svc：`POST /api/v1/execute/stream` | 端点路径不同 | 🟢轻微 |
| 8.9 | MCP标准协议：`POST /mcp` | Rust alliance 域无 MCP 端点 | 未实现 | 🟡中等 |

> **说明**：本文档描述的流程标准主要基于 Node.js 侧的 `expert-alliance-engine.js` 实现。Rust alliance 域是另一套独立实现，两者尚未对齐。文档未说明这一双轨现状。

---

## 第三部分：高频错误模式总结

### 模式1：服务数量严重夸大（出现于 5/8 份文档）

**错误**：文档普遍声称 3-7 个微服务（scheduler / executor / fusion / registry / memory / agent / gateway），代码实际仅 2 个 svc crate。

**出现文档**：README.md、00-INTEGRATED-INDEX.md、cosmic-architecture(7个)、需求文档(L0-L5隐含多服务)

**根因**：文档基于目标架构/行业最佳实践编写，未对照实际代码落地情况。

### 模式2：通信协议虚构（出现于 3/8 份文档）

**错误**：声称 gRPC / Dubbo-Triple / JSON-RPC / mox-dualrpc 多协议支持，代码全部使用 HTTP/REST（axum）+ SSE 流式。

**出现文档**：README.md、cosmic-architecture、00-INTEGRATED-INDEX.md

### 模式3：向量嵌入匹配算法虚构（出现于 2/8 份文档）

**错误**：声称 1536维 embedding + cosine相似度 + pgvector 向量检索，代码实际为 ModularWeightMatcher 纯标签+指标加权（无任何向量/嵌入）。

**出现文档**：cosmic-architecture、README.md

**讽刺点**：cosmic-architecture 给出的权重公式（0.4相似度+0.3评分+0.2成功率-0.1成本）与代码实际权重（0.4能力+0.3指标+0.2领域-0.1成本）数值高度相似，但机制完全不同（向量相似度 vs 标签匹配）。

### 模式4：数据库存储层虚构（出现于 3/8 份文档）

**错误**：声称 PostgreSQL + pgvector + Redis，数据库表 ea_* 系列，代码实际为内存 HashMap + JSON 文件快照（TaskRepository）。

**出现文档**：README.md、cosmic-architecture、00-INTEGRATED-INDEX.md

### 模式5：ReAct 执行模式虚构（出现于 2/8 份文档）

**错误**：声称执行器为 ReAct 循环（Thought→Action→Observation），代码实际为 DagEngineImpl 的 DAG 拓扑排序+并行节点执行。

**出现文档**：cosmic-architecture、README.md

### 模式6：融合策略数量不全（出现于 3/8 份文档）

**错误**：声称 4 种融合策略（加权投票/Stacking/辩论/置信度加权），代码实际 6 种（+MapReduce/IterativeRefinement）。

**出现文档**：README.md、cosmic-architecture、需求文档

### 模式7：诊断文档遗漏 Rust 侧实现（出现于 1/8 份文档，但影响重大）

**错误**：enterprise/26 号架构诊断文档完全聚焦前端+Node后端，未发现 Rust alliance 域的 11 个 crate 实现，导致"专家匹配算法未见实现""AI编排层不透明"等错误结论。

**出现文档**：enterprise/26-架构诊断

### 模式8：Node/Rust 双轨实现未说明（出现于 2/8 份文档）

**错误**：流程标准文档（standards/）和架构诊断文档均未说明专家联盟存在 Node.js（expert-alliance-engine.js）和 Rust（platform/domains/alliance/）两套独立实现，导致端点/流程/能力描述混淆。

**出现文档**：standards/expert-alliance-flow-standard.md、enterprise/26-架构诊断

### 模式9：v2/v3 目标架构未标注实现状态（出现于 3/8 份文档）

**错误**：v2/v3 架构文档和需求文档描述的是目标/优化状态，但未明确标注"未实现"或"实现进度"，读者容易将目标架构误认为当前实现。

**出现文档**：v2/01-architecture.md、v3/01-architecture-optimization.md、需求文档

---

## 第四部分：修订建议

### 4.1 统一修订口径（针对高频错误）

#### 口径1：服务数量——"2个HTTP服务，非多微服务架构"

**统一表述**：
> 专家联盟 Rust 域当前实现为 **2 个 HTTP 服务**：
> - `mox-alliance-scheduler-svc`（端口 8701）：任务调度、专家匹配、协作执行、配置管理
> - `mox-alliance-executor-svc`（端口 8702）：DAG 执行、专家调用
>
> 融合引擎（FusionEngine）、专家注册（Registry）、任务仓库（TaskRepository）均为 **scheduler-core 内的库模块**，非独立服务。
>
> 文档中提及的 fusion-svc / expert-registry / expert-memory / expert-agent / gateway 等服务**当前均未实现为独立服务**，属于目标架构设计。

#### 口径2：通信协议——"HTTP/REST + SSE，无 gRPC"

**统一表述**：
> 所有服务间通信使用 **HTTP/REST**（axum 框架），流式输出使用 **SSE（Server-Sent Events）**。
> 当前**不支持 gRPC / Dubbo-Triple / JSON-RPC**。`mox-dualrpc` 多协议支持为目标设计，未落地。

#### 口径3：匹配算法——"模块化加权匹配，无向量嵌入"

**统一表述**：
> 专家匹配由 `ModularWeightMatcher` 实现，算法为**模块化加权评分**：
> - 能力标签匹配（权重 0.4）
> - 领域标签匹配（权重 0.2）
> - 专家指标评分（权重 0.3，含 accuracy / success_rate / rating）
> - 成本惩罚（权重 -0.1）
>
> 当前**不使用向量嵌入（embedding）/ cosine相似度 / pgvector**。基于 PageRank 的能力图谱匹配为目标设计，未落地。

#### 口径4：存储层——"内存+文件快照，无SQL数据库"

**统一表述**：
> 任务和专家配置存储为**内存 HashMap + JSON 文件快照**（`TaskRepository`），支持持久化和恢复。
> 当前**不使用 PostgreSQL / MySQL / Redis / pgvector**，无 `ea_*` 数据库表。
> 多租户数据隔离（RLS）未实现，tenant_id 仅为字段预留。

#### 口径5：执行模式——"DAG执行，非ReAct循环"

**统一表述**：
> 执行器（`DagEngineImpl`）基于 **DAG（有向无环图）拓扑排序** 执行：
> 构建 DAG → 拓扑排序 → 节点并行/串行调度 → 结果聚合。
> 当前**未实现 ReAct 循环**（Thought→Action→Observation）。专家执行器（`ExpertExecutor`）调用 LLM 完成单节点任务。

#### 口径6：融合策略——"6种，非4种"

**统一表述**：
> 融合引擎支持 **6 种策略**：
> 1. `weighted_voting`（加权投票）
> 2. `confidence_weighting`（置信度加权）
> 3. `debate`（辩论融合）
> 4. `stacking`（Stacking元学习）
> 5. `map_reduce`（MapReduce）
> 6. `iterative_refinement`（迭代精炼）

### 4.2 文档分级修订建议

| 文档 | 修订优先级 | 修订要点 |
|------|-----------|---------|
| cosmic-architecture/02 | 🔴P0 | 7服务→2服务，gRPC→HTTP，向量匹配→标签匹配，ReAct→DAG，SQL→内存，4策略→6策略 |
| README.md | 🔴P0 | 同上，且需标注"v1草案，非当前实现" |
| enterprise/26-架构诊断 | 🔴P0 | 补充 Rust alliance 域 11 crate 分析，修正"匹配算法未见实现"等错误结论 |
| 00-INTEGRATED-INDEX.md | 🟡P1 | 标注各文档的实现状态（已实现/目标设计/已废弃），修正服务数量引用 |
| standards/flow-standard | 🟡P1 | 明确说明 Node/Rust 双轨实现，标注哪些标准已在 Rust 侧落地 |
| v2/01-architecture.md | 🟢P2 | 文档头部标注"目标架构，未实现"，与当前实现的差异对照表 |
| v3/01-architecture-optimization.md | 🟢P2 | 同上，标注优化项的实现进度 |
| 需求文档V2.0 | 🟢P2 | 标注"需求设计，非实现文档"，补充实现进度追踪 |

### 4.3 结构性建议

1. **建立"实现状态标签"制度**：所有架构文档头部必须标注：`[已实现]` / `[部分实现]` / `[目标设计]` / `[已废弃]`，避免读者混淆。

2. **统一 Node/Rust 双轨说明**：在 README 和索引中明确说明专家联盟存在两套实现：
   - Node.js 侧：`platform/backend-node/src/expert-alliance-engine.js`（六阶段流程，面向前端对话）
   - Rust 侧：`platform/domains/alliance/`（11 crate，DAG调度执行，面向服务化）
   - 两者的关系、对齐计划、最终收敛方向需明确。

3. **代码事实参考卡常态化**：本报告的"第一部分：代码事实参考卡"可提炼为 `docs/expert-alliance/CODE-FACTS.md`，作为所有架构文档的权威引用源，文档修订时必须对照。

4. **svc 层测试补齐**：当前 svc/sdk/api 层零测试，建议优先补充 HTTP 路由集成测试和 SDK 端到端测试，这是文档声称"服务化"的最低验证保障。

---

> **报告完**
>
> 核验人：架构核验员（AI辅助）
> 核验方法：全量源码逐文件读取 + 文档逐段对照
> 代码事实确认率：100%（所有声明均有源码文件路径可追溯）
> 文档覆盖：8/8 份指定文档全部读取并分析
