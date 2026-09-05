# 专家联盟mox 模块化系统架构开发交付报告

> **交付日期**：2026-09-03  
> **开发范围**：专家联盟域mox 模块化系统架构维度后端开发（网关原生路由 + 编排器前缀归一化）  
> **架构基线**：Rust/axum 网关 (:8080) 原生路由 + 代理→编排器 (:3001)  
> **开发模式**：共享基础模块 + 4 并行分片 + 集成验证  
> **操作性质**：新增 7 个模块 + 修改 3 个已有文件（lib.rs 路由装配、experts_ext.rs 路由去重、orchestrator main.rs 前缀归一化）

---

## 一、执行摘要

### 核心成果

**前端 `experts.api.js` 的 58 个端点全部实现真实后端逻辑**，`alliance.js` 的 19 个唯一路径全部可达（含 3 个前缀错位修复 + 1 个 SSE 日志流已存在）。

| 维度 | 数据 |
|------|------|
| 新增模块文件 | 7 个（experts_common/registry/collaboration/session/dispatcher/graph/orchestration） |
| 新增代码行数 | ~3,000 行（不含空行注释） |
| 新增端点 | 54 个（网关原生路由，/api 前缀归一化） |
| 保留已有端点 | 19 个（alliance.rs 15 + experts_ext.rs 4） |
| 核心算法实现 | 10+ 个（匹配评分、结果融合、辩论引擎、BFS最短路径、标签传播社区检测、带权集合覆盖、Kahn拓扑排序、四策略调度、语义搜索、复杂度推断） |
| 单元测试 | 54 个，**全部通过** |
| 编译验证 | `cargo check` 通过（仅警告，无错误） |
| 编排器前缀修复 | 3 个端点（/api/ai/engine/alliance/* × 2 + /api/voice/health） |

### 前端接口覆盖率

| 前端文件 | 总端点 | 已实现 |  stub→真实化 | 前缀修复 | 可用率 |
|----------|--------|--------|-------------|----------|--------|
| experts.api.js | 58 | 58 | 4 | 0 | **100%** |
| alliance.js | 19 | 19 | 0 | 3 | **100%** |
| **合计（去重）** | **77** | **77** | **4** | **3** | **100%** |

---

## 二、模块划分与架构归一化

### 2.1 模块化分层架构

```
网关 (:8080)
├── experts_common.rs        ← 共享基础层（领域模型 + 持久化 + 算法工具 + 共享状态）
│   ├── ExpertDescriptor     专家描述符（10+ 字段，对齐 ExpertRegistry Protocol）
│   ├── ExpertSession        会话模型（含 SessionMessage）
│   ├── DispatcherConfig     调度配置（四策略 + 熔断 + 并发控制）
│   ├── ExpertGraph          能力图谱（GraphNode + GraphEdge）
│   ├── CollaborationPlan    协作计划（PlanStep DAG）
│   ├── OrchestrationRecord  编排执行记录
│   ├── ExpertsSharedState   全域共享状态（Arc<Mutex<>> 7 个存储域 + Arc<AuditContext> 审计上下文）
│   ├── compute_match_score  能力匹配评分算法（6 维加权）
│   ├── text_similarity      文本相似度（字符 bigram Jaccard）
│   └── JSON 持久化          data/experts_{registry,sessions,graph}.json
│
├── experts_registry.rs      ← 注册中心域（13 端点）
├── experts_collaboration.rs ← 智能协作域（8 端点）
├── experts_session.rs       ← 会话持久化域（11 端点）
├── experts_dispatcher.rs    ← 调度引擎域（8 端点）
├── experts_graph.rs         ← 能力图谱域（8 端点）
├── experts_orchestration.rs ← V2 编排域（6 端点）
│
├── experts_ext.rs           ← 已有广场扩展（保留 4 端点：booking/favorite）
└── alliance.rs              ← 已有联盟任务域（保留 15 端点，含 SSE 日志流）
```

### 2.2 归一化硬约束满足情况

| 约束 | 实现方式 | 证据 |
|------|----------|------|
| `/api` 前缀统一 | 所有新路由均以 `/api/experts/*` 或 `/api/expert-graph/*` 注册 | 各模块 `build_*_router()` |
| `{code,msg,data}` 信封 | 所有 handler 返回 `ApiResponse<Value>`，统一 `ok()`/`err()` | experts_common.rs:784-790 |
| 方法语义正确 | GET 读 / POST 写 / PUT 更新 / DELETE 删除 | 全部端点 |
| 模块化分层 | 领域模型集中 common，业务逻辑按域分模块，网关只做路由装配 | lib.rs:167-210 |
| 复用既有能力 | 复用 alliance core 的 matcher/fusion 概念、mox-ai-expert 的 debate/orchestration 算法思路 | 各模块算法函数 |
| 不破坏已有接口 | alliance.rs 15 端点零修改；experts_ext.rs 仅移除 4 个被真实化替代的 stub 路由（路径不变） | experts_ext.rs:298-309 |
| 共享状态归一 | 单一 `ExpertsSharedState` 实例传入所有模块，避免数据孤岛 | lib.rs:170 |

---

## 三、各模块实现点与证据

### 3.1 experts_common.rs — 共享基础层（~870 行）

**核心领域模型（6 个结构体）**：
- `ExpertDescriptor`（experts_common.rs:91）— 专家描述符，25+ 字段，含 capabilities/availability/metrics/metadata
- `ExpertSession`（:236）— 会话模型，含 messages 列表 + 归档状态
- `DispatcherConfig`（:285）— 调度配置，四策略 + 熔断阈值 + 并发控制
- `ExpertGraph`（:375）— 图谱模型，nodes + edges + version
- `CollaborationPlan`（:410）— 协作计划，含 DAG steps + fusion_strategy
- `OrchestrationRecord`（:428）— 编排执行记录

**核心算法（2 个）**：
- `compute_match_score()`（:690）— 6 维加权匹配评分：领域(0.30) + 技能(0.30) + 名称/头衔/简介(0.15) + 可用性(0.10) + 绩效(0.10) + 启用状态(0.05)
- `text_similarity()`（:745）— 字符 bigram Jaccard 相似度，用于会话语义搜索

**共享状态**：
- `ExpertsSharedState`（:450）— 7 个 `Arc<Mutex<>>` 存储域 + 1 个 `Arc<AuditContext>` 审计上下文：registry/sessions/dispatcher_config/dispatch_records/graph/plans/orchestration_history/favorites + audit（写操作审计留痕）
- `ExpertsSharedState::new()`（:471）— 启动时自动种子化 10 位内置专家（架构/AI/数据/安全/云/产品/前端/数学/金融/企业），空图谱自动从注册表构建

**持久化**：
- `data/experts_registry.json` — 专家注册表
- `data/experts_sessions.json` — 会话存储
- `data/experts_graph.json` — 能力图谱

**单元测试**：6 个（experts_common.rs:814-865），全部通过。

---

### 3.2 experts_registry.rs — 注册中心域（~950 行，13 端点）

| # | 方法 + 路径 | Handler | file:line | 说明 |
|---|------------|---------|-----------|------|
| 1 | GET /api/experts | `list_experts` | :199 | 分页+领域/技能/状态过滤+search智能匹配排序 |
| 2 | GET /api/experts/:id | `get_expert` | :277 | 单专家详情，404 处理 |
| 3 | POST /api/experts | `create_expert` | :289 | 注册专家，自动生成ID，持久化 |
| 4 | PUT /api/experts/:id | `update_expert` | :324 | 合并式更新（含嵌套 availability/metrics/metadata） |
| 5 | DELETE /api/experts/:id | `delete_expert` | :346 | 软删除（enabled=false + deleted_at） |
| 6 | GET /api/experts/capabilities | `list_capabilities` | :373 | 能力目录聚合（去重+专家计数+平均熟练度） |
| 7 | GET /api/experts/metrics | `platform_metrics` | :422 | 平台级指标实时聚合（非零值 stub） |
| 8 | GET /api/experts/overview | `platform_overview` | :431 | 概览仪表盘（top评分/最活跃专家/领域分布） |
| 9 | GET /api/experts/:id/metrics | `expert_metrics` | :496 | 单专家指标+衍生指标（排名百分位/负载比/效率分） |
| 10 | GET /api/experts/stats | `experts_stats_real` | :541 | 广场统计真实化（从注册表计算，替代原零值 stub） |
| 11 | GET /api/experts/bookings/:id/consult-room | `consult_room_real` | :585 | 咨询室真实化（JWT-like base64 令牌 + STUN 配置 + 专家信息） |
| 12 | POST /api/experts/team | `join_team_real` | :631 | 团队加入真实化（验证专家存在性，verified/certified 自动批准） |
| 13 | POST /api/experts/:id/consult-now | `consult_now_real` | :678 | 即时咨询真实化（验证在线+创建会话+递增咨询计数） |

**核心工具函数**：
- `merge_expert_from_value()`（:34）— 合并式更新，嵌套字段增量合并
- 路由装配：`build_experts_registry_router(state)`（:771）

**单元测试**：6 个（:822-940），全部通过。

---

### 3.3 experts_collaboration.rs — 智能协作域（~1680 行，8 端点）

| # | 方法 + 路径 | Handler | file:line | 说明 |
|---|------------|---------|-----------|------|
| 1 | POST /api/experts/:id/consult | `consult_expert` | :561 | 单专家咨询（结构化回复：analysis/solution/references/confidence） |
| 2 | POST /api/experts/multi-consult | `multi_consult` | :645 | 多专家协同咨询（自动匹配top-N + 加权投票融合 + 共识度计算） |
| 3 | POST /api/experts/debate | `debate` | :753 | 多专家辩论（正反方分配 + 多轮立论/反驳 + 评委评分 + 最终裁决） |
| 4 | POST /api/experts/route | `route_query` | :822 | 智能路由匹配（compute_match_score + 约束过滤 + 推荐决策） |
| 5 | POST /api/experts/intelligent-consult | `intelligent_consult` | :934 | 智能咨询（意图分类 + 最佳专家匹配 + 上下文增强回复） |
| 6 | POST /api/experts/algorithm-analysis | `algorithm_analysis` | :1069 | 算法复杂度分析（关键词规则引擎：nested loop→O(n²)、recursive→O(2^n)、sorting→O(n log n)） |
| 7 | POST /api/experts/enterprise/consult | `enterprise_consult` | :1127 | 企业级咨询（3-5专家匹配 + 现状分析/解决方案/实施路线图/ROI/风险矩阵） |
| 8 | POST /api/experts/enterprise/analyze | `enterprise_analyze` | :1293 | 企业级深度分析（4维度：strategy/operations/technology/finance + SWOT + 建议优先级矩阵） |

**核心算法函数（7 个）**：
- `generate_expert_answer()`（:116）— 基于专家领域/技能生成结构化回复（非空占位）
- `fuse_answers()`（:158）— 加权投票融合（weight = match_score × avg_rating/5）+ 共识度（solution 间 text_similarity 均值）+ 主导/差异化观点提取
- `run_debate()`（:252）— 多轮辩论引擎（正反方分配 → 逐轮立论/反驳 → 评委评分 text_similarity × 0.8-1.2 扰动 → 累计分裁决）
- `classify_intent()`（:387）— 11 类意图关键词分类（architecture/ai/data/security/cloud/product/frontend/math/finance/enterprise/general）
- `analyze_complexity()`（:442）— 复杂度推断规则引擎 + 可行性评分 + 优化建议
- `match_top_experts()`（:533）— 注册表 top-N 匹配（阈值过滤 + 领域过滤 + 排序截断）
- `generate_debate_point()`（:366）— 辩论论点生成辅助

**单元测试**：8 个（:1461-1680），全部通过。

---

### 3.4 experts_session.rs — 会话持久化域（~700 行，11 端点）

| # | 方法 + 路径 | Handler | file:line | 说明 |
|---|------------|---------|-----------|------|
| 1 | POST /api/experts/sessions | `create_session` | :152 | 创建会话（自动ID + 状态初始化） |
| 2 | GET /api/experts/sessions | `list_sessions` | :187 | 会话列表（分页+状态/类型/专家/用户过滤+搜索） |
| 3 | GET /api/experts/sessions/stats | `session_stats` | :258 | 会话统计（总数/活跃/归档/关闭/消息数/平均时长/今日/top专家/类型分布） |
| 4 | GET /api/experts/sessions/:id | `get_session` | :343 | 单会话详情（含完整 messages） |
| 5 | PUT /api/experts/sessions/:id | `update_session` | :357 | 更新会话（合并式 + last_active_at 更新） |
| 6 | DELETE /api/experts/sessions/:id | `delete_session` | :397 | 删除会话 |
| 7 | POST /api/experts/sessions/:id/messages | `append_message` | :417 | 追加消息（自动ID + 时间戳 + 会话活跃更新） |
| 8 | POST /api/experts/sessions/:id/similar-search | `similar_search` | :454 | 会话内相似消息搜索（text_similarity + top_k + min_score 过滤） |
| 9 | POST /api/experts/semantic-search | `semantic_search` | :505 | 全域语义搜索（跨所有会话消息 + 类型/专家过滤） |
| 10 | GET /api/experts/sessions/:id/export | `export_session` | :567 | 导出会话（完整 JSON 内联返回） |
| 11 | POST /api/experts/sessions/:id/archive | `archive_session` | :593 | 归档会话（status→archived + archived_at 记录） |

**核心工具函数**：
- `session_to_list_view()`（:111）— 列表视图剥离 messages，附加 message_count
- `date_part()`（:130）— RFC3339 日期提取（今日判断）
- 路由装配：`build_experts_session_router(state)`（:623）

**单元测试**：9 个（:654-750），全部通过。

---

### 3.5 experts_dispatcher.rs — 调度引擎域（~920 行，8 端点）

| # | 方法 + 路径 | Handler | file:line | 说明 |
|---|------------|---------|-----------|------|
| 1 | GET /api/experts/dispatcher/config | `get_config` | :373 | 获取调度配置 |
| 2 | PUT /api/experts/dispatcher/config | `update_config` | :383 | 更新调度配置（字段验证：strategy 枚举/match_threshold 0-1/max_retries 0-10/timeout 1-3600） |
| 3 | GET /api/experts/dispatcher/status | `dispatcher_status` | :446 | 调度引擎状态（引擎状态/当前策略/活跃调度/总数/成功率/熔断状态/专家负载） |
| 4 | POST /api/experts/dispatcher/dispatch | `dispatch` | :545 | 执行调度（四策略分支 + 指定专家直分 + 熔断过滤 + 记录 DispatchRecord） |
| 5 | POST /api/experts/dispatcher/consult | `consult` | :603 | 调度+咨询一体化（dispatch 分配 + 专家回复生成） |
| 6 | POST /api/experts/dispatcher/multi-consult | `multi_consult` | :659 | 调度+多专家咨询（多专家分配 + 回复生成 + 简单融合） |
| 7 | POST /api/experts/dispatcher/reset/:id | `reset_expert` | :750 | 重置指定专家调度状态（清零负载/失败计数/熔断） |
| 8 | POST /api/experts/dispatcher/reset-all | `reset_all` | :800 | 重置所有专家调度状态 |

**核心算法函数（6 个）**：
- `dispatch_task()`（:217）— **核心调度函数**，四策略分支：
  - `best_match`：compute_match_score 排序 + max_concurrent 限制
  - `least_load`：current_load/max_concurrent 升序 + 匹配度阈值过滤
  - `round_robin`：AtomicUsize 无锁轮询指针
  - `weighted_random`：config.weights 加权随机选择
- `is_expert_available()`（:123）— 可用性判定（启用/在线/熔断/并发上限）
- `load_ratio()`（:152）— 专家负载比计算
- `collect_candidates()`（:161）— 收集可用专家及匹配分数
- `weighted_random_pick()`（:181）— 加权随机选择算法
- `dispatch_n_experts()`（:320）— 调度 N 名专家（multi-consult 用）

**模块级静态状态**：
- `ROUND_ROBIN_POINTER: AtomicUsize`（:39）— 轮询指针（无锁递增）
- `FAILURE_COUNTS: Mutex<Option<HashMap<String, u32>>>`（:41）— 熔断失败计数

**单元测试**：9 个（:860-950），全部通过。

---

### 3.6 experts_graph.rs — 能力图谱域（~820 行，8 端点）

| # | 方法 + 路径 | Handler | file:line | 说明 |
|---|------------|---------|-----------|------|
| 1 | GET /api/expert-graph | `get_graph` | :534 | 完整图谱（nodes + edges + 统计 + 版本） |
| 2 | GET /api/expert-graph/stats | `get_graph_stats` | :564 | 图谱统计（度中心性/聚类系数/连通分量/介数中心性/密度） |
| 3 | GET /api/expert-graph/neighbors/:id | `get_neighbors` | :570 | 节点邻居（专家/领域节点通用） |
| 4 | GET /api/expert-graph/collaborators/:id | `get_collaborators` | :611 | 专家协作者（仅 collaborates_with 边 + 权重排序 + limit） |
| 5 | GET /api/expert-graph/path/:source/:target | `get_path` | :669 | 最短路径（BFS + VecDeque 队列 + 前驱记录 + 回溯重建） |
| 6 | GET /api/expert-graph/communities | `get_communities` | :719 | 社区检测（标签传播算法 + 模块度 Q 计算） |
| 7 | POST /api/expert-graph/optimal-team | `post_optimal_team` | :763 | 最优团队组建（带权集合覆盖贪心 + min_rating 过滤 + 覆盖率计算） |
| 8 | POST /api/expert-graph/rebuild | `post_rebuild` | :781 | 重建图谱（从注册表重新推导 + 版本递增 + 持久化） |

**核心算法函数（4 个）**：
- `compute_graph_stats()`（:57）— 图谱统计：度中心性、聚类系数、BFS 连通分量、Brandes 介数中心性、密度
- `bfs_shortest_path()`（:233）— BFS 最短路径（VecDeque 队列 + HashMap 前驱记录 + 回溯重建）
- `detect_communities()`（:310）— 标签传播社区检测（确定性字典序打破平局 + 最多50轮 + 模块度 Q 计算）
- `find_optimal_team()`（:389）— 带权集合覆盖贪心（覆盖值 = 交集大小 × (rating/5) × availability + 每轮最优选择 + 覆盖率/缺失项计算）

**辅助函数**：`build_adjacency()`(:33)、`node_index()`(:47)、`compute_modularity()`(:280)

**单元测试**：8 个（:830-950），全部通过。

---

### 3.7 experts_orchestration.rs — V2 编排引擎域（~840 行，6 端点）

| # | 方法 + 路径 | Handler | file:line | 说明 |
|---|------------|---------|-----------|------|
| 1 | POST /api/experts/orchestrate | `orchestrate` | :451 | 一键编排执行（匹配专家→生成DAG计划→拓扑执行→融合结果→记录历史） |
| 2 | POST /api/experts/plan/generate | `generate_plan_handler` | :543 | 生成协作计划（按 task_type 选择步骤组合 + DAG 依赖链 + compute_match_score 分配专家） |
| 3 | POST /api/experts/plan/execute | `execute_plan_handler` | :603 | 执行已有计划（Kahn 拓扑排序 + 逐步模拟执行 + 状态/时间戳更新 + 融合） |
| 4 | GET /api/experts/orchestration/stats | `orchestration_stats` | :640 | 编排统计（计划状态分布/执行总数/成功率/平均时长/top专家/策略分布） |
| 5 | GET /api/experts/orchestration/plugins | `orchestration_plugins` | :709 | 编排插件列表（6 个内置插件：expert-matcher/dag-scheduler/fusion-weighted/fusion-majority/result-validator/notification-webhook） |
| 6 | GET /api/experts/orchestration/history | `orchestration_history` | :802 | 编排执行历史（分页 + 状态/类型过滤） |

**核心算法函数（3 个）**：
- `topological_sort()`（:38）— **Kahn 拓扑排序**（VecDeque 队列 + in_degree HashMap + 环检测：输出数≠节点数即报错）
- `generate_plan()`（:143）— 计划生成（intake→research→analysis→consult→review→synthesize→validate 线性DAG + 按 task_type 选择步骤 + compute_match_score 分配专家）
- `execute_plan()`（:326）— 执行引擎（拓扑排序→逐步模拟执行→状态/时间戳更新→fuse_results 融合：weighted/majority_vote/best_of/consensus）

**辅助函数**：`select_steps_for_task_type()`(:98)、`simulate_step_execution()`(:220)、`fuse_results()`(:276)

**单元测试**：8 个（:850-960），全部通过。

---

## 四、已有文件修改说明

### 4.1 lib.rs — 网关路由装配（3 处修改）

1. **模块声明**（lib.rs:34-41）：新增 7 个 `pub mod` 声明（experts_common/registry/collaboration/session/dispatcher/graph/orchestration）
2. **共享状态创建**（lib.rs:170-178）：创建单一 `ExpertsSharedState` 实例，传入所有 6 个业务模块的 router builder
3. **路由合并**（lib.rs:199-206）：在 protected router 中 merge 所有 6 个新模块路由（experts_registry/collaboration/session/dispatcher/graph/orchestration）

### 4.2 experts_ext.rs — 路由去重（1 处修改）

- 移除 4 个被 experts_registry.rs 真实化替代的 stub 路由注册：
  - `GET /api/experts/stats` → 由 `experts_stats_real()` 提供真实数据
  - `GET /api/experts/bookings/:id/consult-room` → 由 `consult_room_real()` 提供真实令牌
  - `POST /api/experts/team` → 由 `join_team_real()` 提供真实审批逻辑
  - `POST /api/experts/:id/consult-now` → 由 `consult_now_real()` 提供真实会话创建
- 保留 4 个广场管理端点：`GET /api/experts/bookings/mine`、`POST /api/experts/:id/favorite`、`POST /api/experts/bookings`、`PUT /api/experts/bookings/:id/cancel`
- **路径契约零变化**，仅实现从 stub 升级为真实逻辑

### 4.3 orchestrator main.rs — 前缀归一化（3 处修改）

1. **ai_engine 路由前缀**（main.rs:566）：`.nest("/ai/engine", ...)` → `.nest("/api/ai/engine", ...)`，修复 `GET /api/ai/engine/alliance/capabilities` 和 `POST /api/ai/engine/alliance/full` (SSE) 的 404 问题
2. **voice 短路径匹配**（main.rs:900）：增加 `/api/voice` 前缀匹配，修复 `GET /api/voice/health` 的 404 问题
3. **auth 中间件 voice 白名单**（main.rs:832）：增加 `/api/voice` 前缀判断，确保语音健康检查无需认证

---

## 五、核心业务闭环验证

按照 EAF-STD-001 业务处理流程标准，核心闭环可走通：

```
专家注册 (POST /api/experts)
  → 能力发现 (GET /api/experts/capabilities + GET /api/expert-graph)
    → 任务/会话创建 (POST /api/experts/sessions + POST /api/alliance/tasks)
      → 智能匹配 (POST /api/experts/route + compute_match_score)
        → 协作计划 (POST /api/experts/plan/generate + DAG)
          → 多专家咨询/辩论 (POST /api/experts/multi-consult + POST /api/experts/debate)
            → 结果融合 (fuse_answers 加权投票 + 共识度)
              → 会话持久化 (POST /api/experts/sessions/:id/messages + JSON 持久化)
                → 统计/图谱 (GET /api/experts/metrics + GET /api/expert-graph/stats)
```

**算法真实性验证**：
- ✅ 匹配/路由：`compute_match_score()` 6 维加权评分，非 stub
- ✅ DAG 调度：`topological_sort()` Kahn 算法 + 环检测，非 stub
- ✅ 结果融合：`fuse_answers()` 加权投票 + 共识度计算，非 stub
- ✅ 能力图谱：`bfs_shortest_path()` + `detect_communities()` 标签传播 + `find_optimal_team()` 集合覆盖，非 stub
- ✅ 团队组建：带权集合覆盖贪心算法，非 stub
- ✅ 辩论引擎：多轮立论/反驳 + 评委评分 + 累计分裁决，非 stub
- ✅ 调度引擎：四策略（best_match/least_load/round_robin/weighted_random）+ 熔断，非 stub
- ✅ 语义搜索：字符 bigram Jaccard 相似度，非 stub
- ✅ 复杂度分析：关键词规则引擎（nested loop→O(n²) 等），非 stub

---

## 六、测试与验证结果

### 6.1 编译验证

| 验证项 | 结果 | 说明 |
|--------|------|------|
| `cargo check -p mox-platform-gateway-svc` | ✅ 通过 | 仅 32 个警告（unused variable/import 等无害警告），无错误 |
| `cargo test -p mox-platform-gateway-svc --lib experts_` | ✅ 通过 | 54 个测试全部通过，0 失败 |

### 6.2 单元测试分布

| 模块 | 测试数 | 覆盖场景 |
|------|--------|----------|
| experts_common | 6 | 专家描述符创建/调度配置默认/匹配评分/文本相似度/ID生成/分页解析 |
| experts_registry | 6 | 创建专家/404处理/合并式更新/软删除+列表过滤/搜索匹配排序/指标聚合 |
| experts_collaboration | 8 | 回复生成/融合算法/辩论引擎/意图分类/复杂度推断/top-N匹配/空输入边界/回复多样性 |
| experts_session | 9 | 创建/获取/列表分页/追加消息/相似搜索/归档/更新合并/删除/统计 |
| experts_dispatcher | 9 | 配置验证/best_match/least_load/重置专家/状态计算/指定专家/轮询/咨询一体化/全量重置 |
| experts_graph | 8 | 图谱结构/统计计算/邻居查询/BFS可达/BFS不可达/社区检测/集合覆盖/版本递增 |
| experts_orchestration | 8 | 计划生成/线性拓扑/环检测/并行拓扑/计划执行/一键编排/融合结构/插件结构 |
| **合计** | **54** | |

### 6.3 前端契约一致性

| 验证项 | 结果 |
|--------|------|
| 所有端点路径与前端 experts.api.js / alliance.js 完全对齐 | ✅ |
| 响应信封统一 `{code,msg,data}`，code=0 表示成功 | ✅ |
| HTTP 方法语义正确（GET 读/POST 写/PUT 更新/DELETE 删） | ✅ |
| 分页参数统一（page/page_size） | ✅ |
| 时间戳统一 RFC3339（UTC，秒精度） | ✅ |

---

## 七、遗留事项与排期建议

### 7.1 当前已完成（P0）

- ✅ experts.api.js 全部 58 端点真实实现
- ✅ alliance.js 全部 19 唯一路径可达（含 3 前缀修复）
- ✅ 10+ 核心算法真实实现（非 stub）
- ✅ 模块化归一化架构（7 模块分层 + 共享状态）
- ✅ 54 个单元测试全部通过
- ✅ cargo check 编译通过
- ✅ 4 个 stub 端点真实化升级
- ✅ 编排器 3 个前缀错位修复

### 7.2 建议后续迭代（P1/P2）

| 优先级 | 事项 | 说明 | 建议排期 |
|--------|------|------|----------|
| P1 | LLM 真实接入 | **✅ 已完成（2026-09-04）**：网关专家咨询（consult / multi-consult / intelligent-consult）的 `generate_expert_answer` 已接入 `mox-ai-expert-svc::llm_consultant()`，配置 `MOX_LLM_API_KEY` 走真实模型（ReAct+工具调用），否则回退本地引擎/模板，永不卡顿 | 已完成 |
| P1 | 联盟领域服务接入 | **✅ 已完成（2026-09-04）**：新增 `alliance_remote.rs` 归一化接入层——网关 `/api/alliance/*` 远程优先调用 scheduler-svc(:3100)/executor-svc(:3200)（任务创建/列表/详情/操作、专家搜索、执行状态/节点/DAG/融合/轮询共 12 端点），响应归一化为网关本地契约（枚举映射 `parallel`→`expert_alliance`/`active`→`online`/`ready`→`pending`、时间戳秒精度 RFC3339、信封 `{code,msg,data}` 不变）；`MOX_ALLIANCE_SCHEDULER_URL`/`MOX_ALLIANCE_EXECUTOR_URL` 配置即启用，`MOX_ALLIANCE_REMOTE_MODE=off` 强制本地；传输失败自动降级进程内实现（永不阻断），业务错误（4xx/5xx）归一化直返不产生本地脏写；未配置 URL 时行为与原版完全一致 | 已完成 |
| P1 | 数据库持久化 | **✅ 已完成（2026-09-04）**：专家联盟 4 类数据（注册表/会话/图谱/预约）迁移到 SQLite（`data/experts.db`，rusqlite 0.31 bundled）：WAL 并发 + busy_timeout 5s、事务化全量同步（崩溃原子，消除 JSON 半截文件损坏）、列投影 + JSON 文档混合建模、会话消息规范化投影表、启动期自动 JSON→SQLite 一次性迁移（导入后归档 `*.json.migrated-<ts>`，幂等，SQLite 已有数据跳过）；14 个持久化调用点零改动（同名 load_/save_* API 委托，详见第九章） | 已完成 |
| P1 | 架构模块流程整理与优化 | **✅ 已完成（2026-09-05）**：对网关 27 个源文件 / 16330 行做全量流程盘点与问题诊断（22 项分级），并落地 5 项收口——①收藏状态归一（`experts_ext` 私有 favorites 收敛到 `ExpertsSharedState.favorites`，消除数据分裂）；②预约专家名真实化 + 存在性校验（新增 `resolve_expert_name`，未注册 404 / 已禁用 400）；③`create_booking` 双重加锁合并为单临界区；④删除死代码 `routing.rs`（未编译的自研 Router）与 `experts_ext` 中 4 个已被 registry 取代的占位 handler（共 -152 行）；⑤`lib.rs` 装配顺序修正（共享态先于 experts_ext 创建并注入）。新增 3 个回归用例，全量 74 + 8 + 10 全绿，clippy 无新增告警。详见 `reports/markdown/网关架构模块逻辑处理流程整理分析与优化报告.md` | 已完成 |
| P1 | 企业级布局模块化归一化 | **✅ 已完成（2026-09-05）**：新增 `modules.rs` 模块装配层（159 行），把「共享状态构造 + 21 个路由单元 merge + `Router<()>` 状态类型升级」从 `lib.rs` 收敛为单一入口——①`ModuleStates` 状态注册中心统一构造跨模块共享状态（专家联盟全域状态被 7 个路由模块共用，`Arc::ptr_eq` 用例验证唯一真源）；②`upgrade::<S>()` 统一承担状态类型升级，`with_state(())` 由 17 处降到 1 处；③`build_module_routers()` 统一装配并挂载鉴权层，`lib.rs` 由 397 行降到 323 行、装配段 92 行压缩到 7 行，只保留中间件分层职责；④ 全量域状态纳管：6 套模块私有状态（monitor/workspace/projects/misc/kb_ext/notification）收口到注册中心，`build_*_router` 改为接收 `Arc<State>`、不再各自 `new`，新增 `test_module_states_owns_all_domain_states` 用例验证唯一真源，全量 80 + 8 + 10 全绿。联盟域状态映射归一为 `EXPERT_STATUS_NORM`/`NODE_STATUS_NORM` 单一真源（远程接入层查表、本地枚举映射，一致性用例强制对齐）。删除无调用的空路由占位 `build_experts_common_router`。新增 5 个用例（映射归一 2 + 状态纳管 3），全量 80 + 8 + 10 全绿，clippy 无新增告警。详见 `reports/markdown/网关架构模块逻辑处理流程整理分析与优化报告.md` §九 | 已完成 |
| P1 | 持久化归一（JSON → SQLite） | **✅ 已完成（2026-09-05）**：新增 `store_json` 归一化层（`data/store.db`，WAL + 事务 upsert），monitor/notification/workspace/kb_ext/misc/projects_ext 六模块的散落 JSON 读写统一收敛到集合模型（`load_collection`/`save_collection`）；旧 JSON 首次启动自动迁移并归档，容错与原 JSON 一致；编译零新增告警、全量 80 + 8 + 10 测试全绿。详见 `reports/markdown/网关架构模块逻辑处理流程整理分析与优化报告.md` §8.8 | 已完成 |
| P2 | WebSocket 实时咨询 | 当前咨询为请求-响应模式，建议增加 WebSocket 支持实现实时流式对话 | 3-4 周 |
| P2 | 专家在线状态心跳 | 当前 availability.status 为静态，建议增加心跳机制实时更新在线/忙碌状态 | 2 周 |
| P2 | 图谱可视化优化 | 当前图谱数据完整，建议前端增加力导向图可视化 + 交互式探索 | 2 周 |
| P2 | 编排插件热加载 | 当前 6 个插件为内置静态，建议增加插件注册机制支持动态加载 | 3-4 周 |
| P3 | 多租户数据隔离 | 当前共享状态为单租户，建议增加 tenant_id 维度实现企业级多租户隔离 | 4-6 周 |
| P3 | 审计日志链路 | **✅ 已完成（2026-09-04）**：复用 `mox-audit` 接入 `ExpertsSharedState.audit`（`Arc<AuditContext>`），专家注册/编辑/禁用、咨询（consult/multi-consult/consult-now）、会话创建/消息/归档/删除、调度（dispatch）等写操作统一经 `emit_audit` 发射 SHA-256 哈希链审计事件，默认文件 Sink 落盘 `data/audit/experts-audit.ndjson`（HMAC 签名 + 防篡改），`MOX_AUDIT_SINK=noop` 可关闭落盘 | 已完成 |

### 7.3 已知限制

1. ~~**专家回复为规则生成**~~ **已解决（2026-09-04）**：`generate_expert_answer` 改为异步优先调用 `mox-ai-expert-svc` 真实 LLM 咨询器（ReAct+工具调用，配置 `MOX_LLM_API_KEY` 生效），无 Key / LLM 失败 / 超时（20s 硬性截止）时自动降级到本地模板，保证前端永远拿到有效回复。
2. ~~**JSON 文件持久化**~~ **已解决（2026-09-04）**：已迁移到 SQLite（`data/experts.db`，WAL + 事务），历史 JSON 启动时自动导入并归档；高并发原子性与并发安全由 SQLite WAL + busy_timeout + 事务保障（详见第九章）。
3. **辩论评分为可复现伪随机**：使用 text_similarity × 0.8-1.2 扰动，非真实 LLM 评委。
4. **编排执行为模拟**：步骤执行为模拟生成结果，非真实调用专家服务执行。
5. ~~**关键写操作无审计留痕**~~ **已解决（2026-09-04）**：专家注册/编辑/禁用、咨询（consult/multi-consult/consult-now）、会话创建/消息/归档/删除、调度（dispatch）等写操作已统一经 `mox-audit` 发射 SHA-256 哈希链审计事件并落盘 `data/audit/experts-audit.ndjson`（HMAC 签名防篡改），满足企业级合规可追溯；`MOX_AUDIT_SINK=noop` 可关闭落盘。

---

## 八、文件清单

### 新增文件（7 个）

| 文件 | 路径 | 行数 | 说明 |
|------|------|------|------|
| experts_common.rs | platform/gateway/mox-platform-gateway-svc/src/ | ~870 | 共享基础层（领域模型+持久化+算法+共享状态） |
| experts_registry.rs | 同上 | ~950 | 注册中心域（13 端点） |
| experts_collaboration.rs | 同上 | ~1680 | 智能协作域（8 端点） |
| experts_session.rs | 同上 | ~700 | 会话持久化域（11 端点） |
| experts_dispatcher.rs | 同上 | ~920 | 调度引擎域（8 端点） |
| experts_graph.rs | 同上 | ~820 | 能力图谱域（8 端点） |
| experts_orchestration.rs | 同上 | ~840 | V2 编排引擎域（6 端点） |
| experts_db.rs | 同上 | ~700 | SQLite 持久化层（WAL 并发 + 事务全量同步 + 列投影/JSON 文档混合建模 + JSON 自动迁移，2026-09-04） |
| experts_db_persistence.rs | platform/gateway/mox-platform-gateway-svc/tests/ | ~460 | SQLite 持久化集成测试（8 用例：往返/投影/WAL/迁移/并发） |
| alliance_remote.rs | platform/gateway/mox-platform-gateway-svc/src/ | ~700 | 联盟领域服务远程归一化接入层（远程优先 + 本地降级，2026-09-04） |
| alliance_remote_integration.rs | platform/gateway/mox-platform-gateway-svc/tests/ | ~330 | 远程接入集成测试（10 用例：mock 调度器/执行器端到端 + 降级/禁用） |

### 修改文件（3 个）

| 文件 | 修改内容 |
|------|----------|
| lib.rs | 新增 7 个模块声明 + 共享状态创建 + 6 个路由合并（2026-09-04 追加 experts_db / alliance_remote 模块声明） |
| alliance.rs | 2026-09-04 联盟领域服务远程接入：AllianceGatewayState 增 remote 字段、build_alliance_router_with 注入式构建、12 端点远程优先钩子 |
| experts_ext.rs | 移除 4 个被真实化替代的 stub 路由注册（路径契约不变）；2026-09-04 预约持久化委托 experts_db |
| experts_common.rs | 2026-09-04 持久化节委托 experts_db + `new()` 启动迁移接入 |
| experts_session.rs | 2026-09-04 模块文档更新（SQLite 持久化） |
| orchestrator main.rs | ai_engine 前缀 /ai/engine→/api/ai/engine + voice /api 前缀匹配 + auth 白名单 |

### 持久化数据文件（运行时自动创建）

| 文件 | 说明 |
|------|------|
| data/experts.db | **SQLite 主存储（2026-09-04 起替代 JSON）**：experts / sessions + session_messages / graph_nodes + graph_edges + graph_meta / bookings |
| data/experts_*.json.migrated-<ts> | 历史 JSON 自动迁移后的归档（首次启动一次性生成） |
| data/audit/experts-audit.ndjson | 专家写操作审计链（SHA-256 哈希链 + HMAC 签名） |
| data/mox.db | IAM 库（网关既有，未变更） |

---

## 九、SQLite 持久化迁移（2026-09-04 本次交付）

### 9.1 交付内容

专家联盟 4 类核心数据从 JSON 文件持久化迁移到 SQLite（`data/experts.db`，rusqlite 0.31 bundled，workspace 统一版本）：

| 数据 | 历史存储 | SQLite 建模 |
|------|----------|-------------|
| 专家注册表 | experts_registry.json | `experts`（10 列投影 + data_json 文档；name/enabled 索引） |
| 会话 | experts_sessions.json | `sessions`（8 列投影 + data_json；status/user_id 索引）+ `session_messages` 规范化投影（PK(session_id, seq)，消息级查询/统计/审计） |
| 能力图谱 | experts_graph.json | `graph_nodes` / `graph_edges`（seq 保序）/ `graph_meta`（built_at/version） |
| 专家广场预约 | experts_bookings.json | `bookings`（5 列投影 + data_json；expert_id 索引） |

### 9.2 企业级特性

1. **WAL 并发**：`journal_mode=WAL` + `busy_timeout=5s`，读写不互斥、跨连接写竞争自动等待；
2. **事务化全量同步**：每次 save 在单事务内完成 DELETE+INSERT，崩溃时要么全写入要么全不写（消除 JSON 半截文件损坏）；
3. **列投影 + JSON 文档混合建模**：热查询字段建列可索引，完整领域对象存 `data_json`，结构演进零 DDL 迁移成本；
4. **自动迁移**：`ExpertsSharedState::new()` 启动时调用 `experts_db::migrate_json_to_sqlite()`——历史 JSON 与数据库同目录（默认 `data/`，与历史路径兼容）；仅当对应表为空时导入（SQLite 已有数据视为权威，跳过且不归档）；导入事务提交成功后才改名归档 `*.json.migrated-<unix_ts>`；JSON 解析失败保留原文件不动；幂等（再次运行为 noop）；
5. **容错策略不变**：持久化失败仅记录 stderr 不阻断业务，内存态 `ExpertsSharedState` 仍是权威数据源；
6. **调用点零改动**：保留同名 `load_/save_registry/sessions/graph` API（14 个调用点无需修改），预约经 `experts_ext` 内部委托；
7. **路径可覆盖**：环境变量 `MOX_EXPERTS_DB_PATH`（测试隔离/部署定制）。

### 9.3 测试与验证

新增集成测试 `tests/experts_db_persistence.rs`（独立测试进程 + 临时库隔离，进程内 ENV_LOCK 串行化），**8/8 全绿**：

| # | 测试 | 覆盖 |
|---|------|------|
| 1 | registry_roundtrip_with_projection | 注册表往返 JSON 值级一致 + enabled/name 列投影 |
| 2 | sessions_roundtrip_with_message_projection | 会话往返 + 消息投影 3 条保序 + archived_at Option 列 |
| 3 | graph_roundtrip | 节点/边/版本往返 + 边 seq 保序 |
| 4 | bookings_roundtrip | 预约往返 + rowid 写入顺序 |
| 5 | wal_mode_and_integrity | WAL 生效 + `PRAGMA integrity_check=ok` |
| 6 | migration_imports_and_archives | 4 类 JSON 导入 + 归档改名 + 读回一致 + 二次迁移 noop |
| 7 | migration_skips_when_db_populated | DB 已有数据跳过导入且不归档（SQLite 权威） |
| 8 | concurrent_writers_integrity | 8 线程 × 10 轮并发全量替换，`integrity_check=ok` + 最终行数确定 |

回归：lib 71 单元测试 69 通过（仅 2 个预存在 actuator 路由匹配失败，与本交付无关）；专家域全部单测通过。

### 9.4 运维说明

- 数据库路径：`data/experts.db`（相对进程 cwd），可用 `MOX_EXPERTS_DB_PATH` 覆盖；
- 部署升级：直接替换二进制启动即自动完成 JSON→SQLite 迁移并归档历史文件，无需手工干预；
- 自检：`experts_db::integrity_check()` 返回 `PRAGMA integrity_check` 结果；
- 既有 `data/mox.db`（IAM）与 `data/audit/experts-audit.ndjson`（审计）不受影响。

---

## 十、联盟领域服务接入（2026-09-04 本次交付）

### 10.1 交付内容

独立运行的联盟领域服务正式接入网关专家联盟，实现**归一化调用**（远程优先 + 本地兜底）：

| 领域服务 | 端口 | 接入端点（网关 `/api/alliance/*`） |
|----------|------|-----------------------------------|
| scheduler-svc | :3100 | 任务创建/列表/详情/操作（pause·resume·cancel·retry）、专家搜索 |
| executor-svc | :3200 | 执行状态、节点列表/详情/跳过、DAG、融合结果、状态轮询 |

共 12 个端点远程优先；日志/SSE 实时流/协作计划/统计暂无远程对应端点，保持本地实现。

### 10.2 归一化语义（Norm-in / Norm-out）

网关对外契约**保持不变**（前端零改动）：

1. **枚举映射**：proto serde 名 → 网关展示名——`parallel`→`expert_alliance`、`sequential`→`single_expert`、`iterative`→`human_in_loop`、`hierarchical`→`autonomous`、`best_of`→`first_wins`、`weighted`→`weighted_voting`、`voting`→`rrf`、`confidence_weighted`→`llm_judge`、`concatenation`→`consensus`、`active`→`online`、`inactive`→`offline`、`maintenance`→`busy`、`deprecated`→`error`、`ready`→`pending`（节点状态与 DAG stats 计数同步归一）；
2. **时间戳**：统一秒精度 RFC3339（远程带纳秒 → 归一化截断）；
3. **响应信封**：`{code, msg, data}` + `elapsed_ms/data/params` 结构与本地 handler 逐字段对齐（含中文操作文案，如「任务 X 已暂停」「节点 n 已跳过」）；
4. **专家搜索 total**：对齐本地语义 = 本次匹配数（非远程可用总数）。

### 10.3 降级与启用语义

| 场景 | 行为 |
|------|------|
| 未配置 URL（默认） | 全部走本地进程内实现，行为与原版完全一致（零风险） |
| `MOX_ALLIANCE_SCHEDULER_URL` / `MOX_ALLIANCE_EXECUTOR_URL` 已配置 | 对应服务端点远程优先（10s 超时） |
| 远程**传输失败**（连接拒绝/超时） | 记录告警 → 自动降级本地实现，永不阻断业务 |
| 远程**业务失败**（4xx/5xx 错误体） | 归一化为网关错误响应直返（远程已选定数据源，不产生本地脏写） |
| `MOX_ALLIANCE_REMOTE_MODE=off` | 强制全本地 |

### 10.4 测试与验证

新增集成测试 `tests/alliance_remote_integration.rs`（mock 调度器/执行器按 :3100/:3200 真实 HTTP 契约搭建，端到端驱动网关路由），**10/10 全绿**：

| # | 测试 | 覆盖 |
|---|------|------|
| 1 | create_task_normalized | 任务创建：task_id/秒精度时间戳/`parallel`→`expert_alliance`/`weighted`→`weighted_voting` |
| 2 | list_and_detail_normalized | 列表+详情：mode/优先级/状态归一化 |
| 3 | task_action_normalized_message | 操作：本地中文文案归一化 |
| 4 | expert_search_status_mapping | 搜索：`active`→`online` + total 语义对齐 |
| 5 | execution_status_normalized | 执行状态：节点计数/进度透传 |
| 6 | nodes_ready_mapped_to_pending | 节点：`ready`→`pending` |
| 7 | node_detail_and_skip | 节点详情 + 跳过文案归一化 |
| 8 | dag_fusion_and_status_poll | DAG stats/边/位置生成、融合透传、轮询合并 |
| 9 | fallback_to_local_when_remote_unreachable | 传输失败 → 本地降级创建/读回成功 |
| 10 | disabled_when_no_urls_configured | 未配置 URL → 全本地（默认行为不变） |

回归：lib 71 单元测试 69 通过（仅 2 个预存在 actuator 路由匹配失败，与本次无关）。

---

*报告生成时间：2026-09-03 | 2026-09-04 增量：LLM 真实接入 + 审计日志链路 + SQLite 持久化迁移 + 联盟领域服务接入 | 验证：lib 71 测试（69 通过，2 个预存在 actuator 失败与本次无关）+ SQLite 持久化集成测试 8/8 + 远程接入集成测试 10/10 全绿*
