# 专家联盟（alliance）与 AI 专家域 · 只读代码侦察报告

- 侦察对象：`platform/domains/alliance/` 与 `platform/domains/ai/` 下专家/联盟相关 crate
- 侦察方式：只读读源码 + Cargo.toml + 静态 grep；**未运行任何 cargo 命令，未修改/新建/删除任何业务文件**
- 侦察日期：2026-09-27
- workspace：edition 2021 / nightly，根 Cargo.toml 聚合约 149 crate

> 测试计数说明：下文「单测」数来自 `#[\[tokio::]test\]` 粗计，仅统计 `src/*.rs` 顶层（未递归子目录），子目录 `#[cfg(test)]` 未全计入，故实际单测数普遍高于表内数字；集成测数来自各 crate `tests/` 目录。

---

## 一、每 crate 现状表

### 1. alliance 域（platform/domains/alliance）

| crate（路径） | 职责 | 已实现（真实逻辑） | 桩 / 缺口 | 测试覆盖 |
|---|---|---|---|---|
| `api`（mox-alliance-api） | 对外 HTTP DTO 层 | `dto.rs`：任务提交/查询/动作/人工完成等请求响应体；纯类型 | 无业务逻辑，仅 DTO | 0 |
| `proto/common-proto` | 联盟通用协议 SSOT | `Task/Node/CollaborationPlan/Expert/Capability/ToolBinding/Domain/ExpertHealth`；`TaskStatus` 7 态状态机 `can_transition_to`；`CollaborationPlan.validate()`（拓扑排序环检测+依赖校验）；`AllianceMode`(7) / `FusionStrategy`(9) 枚举；模块 LLM/Graph 配置 + `merge_with_global`；`ApiKeySource` 解析；`MatchingWeights`；领域事件 `AllianceEvent/TaskEvent/NodeEvent/ExpertEvent` + NATS subject 命名 | 纯协议层，无逻辑 | 9 单测（状态机转换全覆盖） |
| `proto/executor-proto` | 执行器协议契约 | `DagEngine` trait（start/pause/resume/cancel/status/nodes/get_node/skip_node/get_fusion_output）；`NodeExecutor` trait + 请求/结果；`ExecutionOptions`(重试/超时/fail_fast)；`ExecutorConfig`；`FusionOutput`；`ExecutionStatus` | 纯契约，无实现 | 0 |
| `proto/registry-proto` | 注册中心协议契约 | **两套专家实体**：静态目录 `Expert`（人工档案，CRUD DTO）+ 运行实例 `RegisteredInstance`（endpoint/capabilities/lease/负载/InstanceStatus 4 态）；`ExpertDirectory` / `InstanceRegistry` trait；租约 `is_alive_at/is_expired_at` | `ExpertDirectory::get_expert_metrics` / `get_platform_overview` 为默认「未实现」占位（契约预留） | 3 单测（租约/serde 契约） |
| `proto/scheduler-proto` | 调度器协议契约 | `TaskScheduler` trait（submit/cancel/pause/resume/**complete_task 人工终态**/get/generate_plan/queue/list/running）；`TaskSubmitRequest.idempotency_key`；`ExpertMatcher` trait（match/get/refresh_cache/infer_domains）；`MatchScoreBreakdown` 五维明细；`PlanGenerationRequest/Response` | 纯契约 | 0 |
| `core/mox-alliance-core` | 共享纯算法内核 | DAG 工具；融合：`rrf_fusion/weighted/voting/best_of/concatenate/merge_json`；`FusionEngine` 统一调度 6 大策略（加权投票/置信度加权/堆叠/辩论/MapReduce/迭代精炼）+ 各策略纯实现 | Stacking 在标量场景降级为置信度加权（无训练数据）；非完全体 | ~30 单测（策略矩阵全覆盖） |
| `core/registry-core` | 分级心跳聚合（零 IO 纯计算） | node→rack→cell 10:1:1 聚合；`NodeBeat/GroupDigest/AggregatedRenewal`；成员状态映射；健康度 Healthy/Degraded/Suspect | 纯算法，无 IO | 7 单测 |
| `core/scheduler-core` | 调度器核心（14 文件，最大 core） | `TaskSchedulerImpl`：容量/并发上限、**幂等键回放**、状态机咽喉点强制、匹配→计划→派发→同步回填；`reconcile_active_tasks`（leader 专属孤儿任务对账，宁可漏判不误判）；`ModularWeightMatcher`（生产主路径，专家独立权重）+ `RuleBasedExpertMatcher`（兼容遗留）；`SimplePlanGenerator`（7 种模式 DAG 生成 + Dynamic 路由规则 + 匹配分→融合权重）；`LlmRouter`（多 Provider 路由+熔断）；`ExecutorBridge` trait + InProcess/Noop/Http(feature)；`ExpertRegistryBridge` + `ExpertSynchronizer`；存储 `TaskRepository`（InMemory/File/Sqlite feature）；租约选主 `LeaderElector`；metrics | Dynamic 模式为**规划期一次性选型**，执行期按中间结果改写拓扑未实现（planner 注释已诚实声明）；无独立队列 worker tick（submit 内联驱动） | 119+ 单测（顶层计；子目录更多） |
| `core/executor-core` | DAG 执行引擎（纯真实） | `DagEngineImpl`（1373 行）：控制通道 Start/Restore/Pause/Resume/Cancel/SkipNode、依赖调度、进度追踪、Dynamic 条件路由、尾部融合产出 `FusionOutput`；`ExpertNodeExecutor` 调 `mox_ai_expert_proto::ExpertConsultant`，带 tokio 超时 + 指数退避重试 + 熔断字符串识别 + 统计；`condition.rs` 条件求值；`state_sink` 持久化端口；`fusion.rs` 融合；`MockNodeExecutor`（feature=mock） | node_executor 字段当前仅 DI 预留（`#[allow(dead_code)]`） | 39 单测 + 6 集成（含 bench） |
| `core/config-core` | 模块化配置引擎 | `ConfigEngine`：每模块独立 LLM/Graph 配置、热更新 broadcast、版本化+回滚、写入前校验；`ConfigStore` trait + 内存实现；`examples/domain_experts.rs` 内置 10 大领域专家配置 | 配置校验里 `ApiKeySource::Inherit => unreachable!()`（理论不可达分支） | 5 单测 |
| `core/boot-config` | 服务引导配置加载 | yml + `MOX_ALLIANCE_*` 环境变量覆盖链；`ConfigStoreChain`；专家模块 yml overlay 合并；Nacos config/naming（feature 可选） | Nacos 依赖为可选 feature；e2e 测试需本机 rnacos | 23 单测 + 5 集成（nacos_e2e/naming_e2e gated） |
| `svc/scheduler-svc` | 调度器 HTTP 服务（:3100） | axum 路由 `/tasks`(POST/GET) `/tasks/:id`(GET/动作) `/experts/search` `/health` `/metrics` `/leadership`；装配 http-bridge+sqlite+nacos；HA 租约选主 | — | 10 单测 + 11 集成 |
| `svc/registry-svc` | 注册中心 HTTP 服务（:3400） | axum：静态目录 CRUD `/api/v1/experts` + 实例注册/心跳/发现 `/api/registry/experts`；SQLite 存储；主动 HTTP 健康探测 `HttpHealthProbe` + 周期摘除；分级聚合核心接入 | `get_expert_metrics/get_platform_overview` 未实现（见 proto） | 14 单测 + 10 集成 |
| `svc/executor-svc` | 执行器 HTTP 服务（:3200） | axum：`/tasks/:id/status|nodes|nodes/:node_id(skip)|result|cancel|pause|resume` + `/internal/executions`；装配真实 `ExpertNodeExecutor`（经 `mox_ai_expert_svc::expert_traits::llm_consultant`）；`SqliteExecutionStateSink` 持久化 | 注释声明「纯真实执行，无 Mock 路径」 | 9 单测 + 6 集成 |
| `sdk/mox-alliance-sdk` | 客户端 SDK | `AllianceClient` reqwest 调 scheduler（任务提交/查询/取消/动作），带 X-Tenant-Id/X-User-Id | — | 10 集成 |
| `sdk/mox-alliance-http-sdk` | 可挂载网关路由 `/alliance/v1` | `build_alliance_router`；`RemoteAllianceClient`；重导出 common-proto 类型与命名映射 | **自带一套遗留预览态执行模型**（`NodeExecStatus`/`ExecNode`/内存 DAG 演示数据），与 proto 的 `Node/NodeStatus` 平行重复 | 15 单测 |

### 2. ai 域专家相关 crate（platform/domains/ai）

| crate（路径） | 职责 | 已实现 | 桩 / 缺口 | 测试覆盖 |
|---|---|---|---|---|
| `proto/mox-ai-expert-proto` | 璇玑专家协议 SSOT | `Dimension` 14 维（业务 7 + 开发 7）；`ExpertOpinion/ExpertMeta/ConsultQuery/ConsultReport/TaskSpec/RoutingDecision/Risk/Suggestion/Constraint`；trait `ExpertRegistry/ExpertConsultant/AllianceOrchestrator`；`GovernExpert/GovernContext`；SSOT 常量 `DIM_PRIORITY/THRESHOLD/WEIGHTS/CONFLICT_ESCALATE_GAP` | trait 方法 Result 暂为 `anyhow::Result`（未统一到 ExpertResult，注释声明后续阶段替换） | 66 单测 |
| `core/mox-ai-expert-core` | 十四维专家引擎核心（P2 阶段4） | `ExpertEngine` 统一入口（Registry/Consultant/Governor）；IR `CodeIR/DimensionedFlow`；14 维归一化；`dispatch` rayon 真并行；`reconcile` 维度优先级裁决+冲突升级；govern 闸门；verify 守恒不变量；context/敏感度 | **14 位专家中 11 位为骨架**（`analyze` 直接返回 `ExpertOpinion::empty`，标 TODO）；`govern/pipeline/verify/tenant_policy` 亦为骨架待迁移 | 33 单测 |
| `core/mox-ai-alliance-engine` | 联盟 6 阶段管线（P2 阶段5） | Intent（双路 RRF 7 分类）→Team（组队+安全强制替换）→Debate（并行咨询+共识）→Synthesize→Gate（HC-8 评分 A/B/C/D）→Learn（维度增益）；`OrchestrationEngine` 3 策略；`AlgorithmAnalyzer`；`IntelligentRouter`；KG connector trait；pg/llm-http 可选 feature；依赖 `mox-unified-contract` | **自行重定义 `ExpertMeta/ExpertRegistry/ExpertOpinion/ExpertConsultant/LocalRuleConsultant`**（仅复用 proto 的 `Dimension`），未复用 proto 同名 trait | 96 单测 + 5 集成 |
| `svc/mox-ai-expert-svc` | 璇玑专家服务（L4，HTTP/RBAC/审计） | **真实 14 专家实现**在此（`experts/*.rs` 基于 flow-svc model 做关键路径/缓存/算力路由等真实分析）；`harness.run_experts` rayon 并行；rbac 权限链；audit（syslog/S3 SigV4）；verify/*（CEM/code_rt/conflict/data_dep/gains/topology）；pipeline_core 阶段/钩子；llm/*（chat/consultant/react/router/tools）；flow_loader YAML；`alliance/kg_connector`（http/mock/sdk） | `src/alliance/` 是**第二套 6 阶段管线**（自带 `AlliancePhase/AllianceEvent/AllianceRequest/AllianceError` + intent/team/debate/gate/orchestration/algorithm），与 mox-ai-alliance-engine 平行 | 58 单测 + 68 集成 |
| `svc/mox-ai-agent-svc` | 通用智能体框架 | 对话引擎、workflow_engine、dialogue_graph、`engine/multi_agent` 多智能体编排、browser_automation、plugin_bus、parallel_executor、llm_client、KG 接入 | 与专家联盟域仅松耦合；自带一套 `MultiAgentOrchestrator`，未复用 alliance 的 Task/DAG 模型 | 89 单测 + 6 集成 |

---

## 二、五维功能缺口清单

### 1) 专家建模
- **存在四套互不统属的「专家」模型**：
  - alliance `common-proto::Expert`（运行时专家：capabilities/tools/health/priority）
  - alliance `registry-proto::Expert`（静态目录档案）+ `RegisteredInstance`（运行实例）
  - ai `expert-proto::Dimension/ExpertMeta` + svc 本地 `expert::Expert/ExpertOpinion`
  - ai `alliance-engine` 本地 `ExpertMeta/ExpertOpinion/ExpertRegistry/ExpertConsultant`
- `mox-ai-expert-core` 11/14 专家为空骨架（`empty` opinion）；真实逻辑在 svc 侧，core 与 svc 专家实现尚未合流。
- alliance 域**没有 Team 概念**（只有 CollaborationPlan/Nodes）；Team 仅存在于 ai-alliance-engine。

### 2) 注册
- alliance registry-svc 实例注册/心跳/租约/发现/健康探测**已真实落地**。
- 缺口：`ExpertDirectory::get_expert_metrics`、`get_platform_overview` 仍为契约默认未实现。
- scheduler 匹配器面向内存 `ModularWeightMatcher`，由 synchronizer 从 config-core 内置 10 领域专家灌入；与 registry-svc 实例发现之间的运行期打通依赖 `HttpExpertRegistryBridge`（feature 门控）。

### 3) 调度
- 已实现：提交容量/并发上限、幂等键、状态机强制（终态不可离开）、优先级队列、暂停/恢复/取消/人工完成、leader 专属孤儿对账。
- 缺口：无独立后台 tick 驱动队列（提交即内联规划派发）；Dynamic 模式仅规划期启发式选型，**执行期按中间结果改写拓扑未实现**；多 executor 实例间负载感知路由仅有 weight 字段，无真实调度策略。

### 4) 执行
- 已实现：DAG 依赖调度、超时、指数退避重试、fail_fast、节点 skip（人工干预）、恢复续跑（已完成节点不重跑）、尾部融合落 `fusion_result`、状态持久化端口。
- 缺口：**人审/中断不是一等公民**——仅有 `skip_node` + 人工 `complete_task`，没有「DAG 节点挂起等待人工审批输入后再继续」的 first-class approval gate；http-sdk 里的「人工评审」节点是演示数据。

### 5) 协作编排
- 已实现：7 种 AllianceMode 计划生成、9 种 FusionStrategy 真算法、辩论/投票/MapReduce/迭代等并行/分层/辩论编排。
- 缺口：6 阶段管线（Intent→Team→Debate→Synthesize→Gate→Learn）**存在两份**（ai-alliance-engine 与 svc/alliance），第三处在 expert-svc 历史模块，未收敛为单一实现。

---

## 三、与三个指定外部 crate 的依赖关系

| 外部 crate | alliance 域 | ai-expert-proto | ai-expert-core | ai-alliance-engine | ai-expert-svc | ai-agent-svc |
|---|---|---|---|---|---|---|
| `shared/mox-unified-contract` | 无引用 | 无 | 无 | **有**（SSOT-5 阶段名/审计事件名） | **有**（同左） | 无 |
| `mox-unified-algo-core` | 无 | 无 | 无 | 无 | 无 | 无 |
| `data/mox-data-norm-core` | 无 | 无 | 无 | 无 | 无 | 无 |

补充：alliance 域内部跨域依赖为 `executor-core → mox-ai-expert-proto`（path 依赖，借 `ExpertConsultant` trait）与 `executor-svc → mox-ai-expert-svc`（path 依赖，取真实 llm_consultant 装配）；ai-expert-core 依赖 `mox-ai-flow-core` + `mox-audit`；ai-alliance-engine 依赖 `mox-pipeline-framework`(async/audit)。

---

## 四、alliance 域 vs ai/expert、ai-alliance-engine 重叠 / 不一致清单

1. **6 阶段管线双份**：`mox-ai-alliance-engine`（独立 crate）与 `mox-ai-expert-svc/src/alliance/`（svc 内模块）各自定义 `AlliancePhase/AllianceEvent/AllianceRequest/AllianceError` 与 intent/team/debate/gate/orchestration/algorithm，功能注释自述「与原 svc 联盟模块完全对齐」——即搬迁未删旧。
2. **trait 重复定义**：`ai-alliance-engine` 本地 `ExpertRegistry/ExpertConsultant/ExpertMeta/ExpertOpinion` 与 `ai-expert-proto` 同名 trait/类型并存，仅复用了 `Dimension`。
3. **专家概念四套**（见二.1）：common-proto::Expert、registry-proto::Expert+RegisteredInstance、proto Dimension/ExpertMeta、engine 本地 ExpertMeta。
4. **同名混淆**：`mox-alliance-core::fusion::traits::FusionStrategy<Item>`（泛型 trait）与 `common-proto::FusionStrategy`（枚举）同名；engine.rs 中以 `FusionStrategyType` 别名消解。
5. **执行状态模型重复**：http-sdk 自定义 `NodeExecStatus/ExecNode`，与 common-proto `NodeStatus/Node` 平行。
6. **专家实现双轨**：expert-core 的 11 个空骨架专家 vs svc 真实专家；svc 对 core 的 `AlgorithmExpert` 做 `pub use` 后又为其实现 svc 本地 `Expert` trait（双 trait 分发）。
7. **匹配器双份**：`RuleBasedExpertMatcher`（遗留，http-sdk/旧路径用）与 `ModularWeightMatcher`（生产主路径）并存，lib.rs 已注明勿新增前者使用方。
8. **TaskStatus 多源**：common-proto 有权威状态机，ai-alliance-engine 又重导出本地 `TaskStatus`（orchestration 用）。

---

## 五、建议的「安全增量点」（只新增文件 + 补测试，不动 workspace/CI/既有文件语义）

1. **纯算法融合策略**：在 `mox-alliance-core/src/fusion/strategies/` 新增策略文件 + 在 `strategies/mod.rs` 补一行导出 + 单测；不改动 `FusionStrategy` 枚举与 engine 匹配臂即可零风险落地（engine 匹配臂如需新分支属最小既有文件编辑，可后续单独评审）。
2. **executor-core 条件算子**：在 `condition.rs` 旁新增纯函数比较算子 + 单测（已有 `CompareOp/Operand` 模式可照抄）。
3. **新集成测试文件**：在已有 `tests/` 目录（如 executor-svc / scheduler-svc / registry-svc 的 `tests/`）新增 `*.rs`，纯新增、不碰 src。
4. **ai-alliance-engine 纯逻辑**：intent/learning/algorithm 为零 IO 默认编译，可在其内新增分析函数 + `#[cfg(test)]`，不触发 pg/llm-http 依赖。
5. **填补 expert-core 骨架专家**：可在 `experts/*.rs` 内把 `ExpertOpinion::empty(...)` 替换为真实规则分析（这是既有文件编辑，但局限于该 crate 内部、不动公共 API 形态）。
6. **registry-core 聚合边界用例**：新增 rack 宕机/跨 cell 合并的纯计算测试。

---

## 六、明确「不要做」的动作

- **不要**改根 `Cargo.toml` / workspace members，不要新增、重命名、移动任何 crate。
- **不要**修改任何 `Cargo.toml` 依赖（不加依赖、不改 feature 默认集）。
- **不要**改 CI / GitHub Actions / 构建脚本。
- **不要**做任何 git 操作（add/commit/push/checkout/branch/stash）。
- **不要**跑 `cargo check/build/test/clippy`（本阶段只读；用户已明确排除）。
- **不要**改动既有源码逻辑来「顺手修」重叠问题——本报告仅侦察，收敛方案需另行评审。
- 报告产物本身：已写入 `reports/alliance_recon.md`（目录无同名旧文件，未覆盖任何东西）。
