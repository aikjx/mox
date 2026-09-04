# 专家联盟（Expert Alliance）功能覆盖分析报告

> 项目路径：`platform/domains/ai/svc/mox-ai-expert-svc`
> 报告日期：2026-08-31

---

## 一、功能覆盖总览

| # | 功能模块 | 状态 | 核心实现文件 | API 端点 |
|---|---------|------|-------------|---------|
| 1 | 专家注册 | 已补全 | `services.rs` + `types.rs` | `POST /api/alliance/experts/register` |
| 2 | 专家列表/详情 | 已补全 | `services.rs` + `server.rs` | `GET /api/alliance/experts` / `/:id` |
| 3 | 智能路由/匹配 | 已补全 | `services.rs` + `alliance/intent.rs` + `alliance/team.rs` | `POST /api/alliance/route` |
| 4 | 单专家咨询 | 已补全 | `services.rs` + `alliance/debate.rs` | `POST /api/alliance/consult` |
| 5 | 多专家协同咨询 | 已补全 | `services.rs` + `alliance/debate.rs` | `POST /api/alliance/multi-consult` |
| 6 | 专家辩论（同步） | 已补全 | `services.rs` + `alliance/mod.rs` | `POST /api/alliance/debate` |
| 7 | 专家辩论（SSE 流式） | 已补全 | `server.rs` (SSE handler) | `GET /api/alliance/debate/stream` |
| 8 | mox 模块化系统架构分析（同步） | 已补全 | `services.rs` + `alliance/mod.rs` | `POST /api/alliance/full` |
| 9 | mox 模块化系统架构分析（SSE 流式） | 已补全 | `server.rs` (SSE handler) | `GET /api/alliance/full/stream` |
| 10 | 任务编排引擎 | 新建 | `alliance/orchestration.rs` | `POST /api/alliance/orchestrate` |
| 11 | 算法分析引擎 | 新建 | `alliance/algorithm.rs` | `POST /api/alliance/algorithm-analysis` |
| 12 | 专家概览 | 已补全 | `services.rs` | `GET /api/alliance/overview` |
| 13 | 专家指标 | 已补全 | `services.rs` | `GET /api/alliance/metrics` |

**总计：13 个功能点，全部覆盖。**

---

## 二、各功能模块详细分析

### 2.1 专家注册（Expert Registration）

**状态：已补全**

**原有实现：**
- `expert_traits.rs` 中定义了 `ExpertRegistry` trait（`register` / `unregister` / `list` / `get`）
- `services.rs` 中 `RegistryImpl` 实现了 `ExpertRegistry`，但仅为进程内内存实现
- 缺少对外暴露的 HTTP API 及请求/响应 DTO

**补全内容：**
- `types.rs`：新增 `RegisterExpertRequest` / `RegisterExpertResponse` / `ExpertInfo` DTO
- `services.rs`：`AllianceService::register_expert()` 统一封装注册逻辑
- `server.rs`：新增 `POST /api/alliance/experts/register` 路由及 handler
- `server.rs`：新增 `GET /api/alliance/experts`（列表）和 `GET /api/alliance/experts/:id`（详情）

**关键代码：**
```rust
// types.rs - 注册请求
pub struct RegisterExpertRequest {
    pub id: String,
    pub name: String,
    #[serde(default = "default_domain_star")]
    pub domain: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub description: String,
    pub dimension: Option<String>,
}
```

---

### 2.2 智能路由/专家匹配（Intelligent Routing）

**状态：已补全**

**原有实现：**
- `alliance/intent.rs`：意图分类器（`classify_intent`），支持 8 种意图类型
- `alliance/team.rs`：专家组队优化器（`optimize_team`），基于领域匹配 + 互补性评分
- `expert_traits.rs`：`AllianceOrchestrator` trait 定义了 `route_experts`
- `services.rs`：`AllianceRouter` 实现了路由逻辑
- 缺少对外暴露的 HTTP API

**补全内容：**
- `types.rs`：新增 `RouteExpertsRequest` / `RouteExpertsResponse` DTO
- `services.rs`：`AllianceService::route_experts()` 封装路由逻辑
- `server.rs`：新增 `POST /api/alliance/route` 路由及 handler

**路由策略：**
1. 意图分类 → 识别用户查询的意图类型
2. 领域匹配 → 从注册中心筛选匹配领域的专家
3. 能力互补 → 优化团队组成，最大化能力覆盖
4. 置信度排序 → 按匹配度返回 Top-N 专家

---

### 2.3 专家咨询（Expert Consultation）

**状态：已补全**

**原有实现：**
- `expert_traits.rs`：`ExpertConsultant` trait 定义了 `consult`
- `services.rs`：`ExpertServiceImpl` 实现了咨询逻辑（基于规则的模拟）
- `alliance/debate.rs`：`consult_and_debate` 实现了多专家咨询+辩论
- 缺少单专家咨询的 HTTP API

**补全内容：**
- `types.rs`：新增 `ConsultExpertRequest` / `ConsultExpertResponse` DTO
- `services.rs`：`AllianceService::consult_expert()` 封装单专家咨询
- `server.rs`：新增 `POST /api/alliance/consult` 路由及 handler

**咨询流程：**
1. 指定专家 ID → 直接咨询该专家
2. 未指定 → 走智能路由选择最佳匹配专家
3. 返回专家意见 + 置信度 + 相关引用

---

### 2.4 多专家协同（Multi-Expert Consultation）

**状态：已补全**

**原有实现：**
- `alliance/debate.rs`：`consult_and_debate` 实现了多专家并行咨询 + 交叉辩论
- 缺少多专家协同的独立 HTTP API

**补全内容：**
- `types.rs`：新增 `MultiConsultRequest` / `MultiConsultResponse` DTO
- `services.rs`：`AllianceService::multi_expert_consult()` 封装多专家协同
- `server.rs`：新增 `POST /api/alliance/multi-consult` 路由及 handler

**协同模式：**
- 并行咨询：所有专家同时生成初始意见
- 交叉审阅：专家互相阅读并评论彼此意见
- 共识评分：计算整体共识度与分歧点
- 最终建议：汇总生成综合建议

---

### 2.5 专家辩论（Expert Debate）

**状态：已补全（含 SSE 流式）**

**原有实现：**
- `alliance/mod.rs`：`AllianceEngine::run_pipeline()` 完整 6 阶段管线
  - Phase 1: Intent（意图识别）
  - Phase 2: Team（专家组队）
  - Phase 3: Debate（交叉辩论）
  - Phase 4: Synthesis（综合合成）
  - Phase 5: Gate（质量门禁）
  - Phase 6: Learn（学习沉淀）
- 缺少 HTTP API 和 SSE 流式接口

**补全内容：**
- `types.rs`：新增 `ExpertDebateRequest` / `ExpertDebateResponse` / `DebateOpinion` DTO
- `services.rs`：`AllianceService::expert_debate()` 封装辩论管线
- `server.rs`：新增 `POST /api/alliance/debate`（同步）
- `server.rs`：新增 `GET /api/alliance/debate/stream`（SSE 流式）

**SSE 事件流：**
- `event: done` → 辩论完成，携带完整结果 JSON
- `event: error` → 错误信息

> **注**：当前 SSE 实现为"批量执行后单事件推送"模式。生产环境可升级为逐阶段流式推送（使用 `tokio::sync::mpsc` channel）。

---

### 2.6 mox 模块化系统架构分析（Full Analysis）

**状态：已补全**

**原有实现：**
- `alliance/mod.rs`：`AllianceEngine::full_analysis()` 实现了完整的mox 模块化系统架构分析
- 缺少 HTTP API 和 SSE 流式接口

**补全内容：**
- `types.rs`：新增 `FullAnalysisRequest` / `FullAnalysisResponse` / `FullAnalysisOptions` DTO
- `services.rs`：`AllianceService::full_analysis()` 封装mox 模块化系统架构分析
- `server.rs`：新增 `POST /api/alliance/full`（同步）
- `server.rs`：新增 `GET /api/alliance/full/stream`（SSE 流式）

**mox 模块化系统架构分析 vs 专家辩论的区别：**
- 专家辩论：聚焦多方观点碰撞，输出共识/分歧
- mox 模块化系统架构分析：完整 6 阶段管线，含质量门禁评分 + 知识图谱沉淀

---

### 2.7 任务编排（Task Orchestration）

**状态：新建完成**

**原有实现：** 无独立编排引擎，管线执行硬编码在 `AllianceEngine` 中

**新建模块：** `alliance/orchestration.rs`

**核心类型：**
- `OrchestrationStrategy`：编排策略枚举（Sequential / Parallel / Pipeline）
- `OrchestrationEngine`：编排引擎（任务跟踪 + 执行调度）
- `OrchestrationTask`：任务状态（步骤、状态、耗时、评分）

**三种编排策略：**

| 策略 | 描述 | 适用场景 |
|------|------|---------|
| `sequential` | 按优先级顺序执行专家，逐步深入 | 需要渐进式推理的复杂问题 |
| `parallel` | 所有专家并行执行，结果汇总 | 需要多视角快速评估的问题 |
| `pipeline` | 完整 6 阶段专家联盟管线 | 需要高质量、可审计的深度分析 |

**HTTP API：** `POST /api/alliance/orchestrate`

---

### 2.8 算法分析（Algorithm Analysis）

**状态：新建完成**

**原有实现：** 无

**新建模块：** `alliance/algorithm.rs`

**核心类型：**
- `AnalysisDimension`：分析维度枚举
- `AlgorithmAnalyzer`：算法分析引擎
- `AlgorithmCheckItem`：检查项（规则名、描述、是否通过、是否阻断、详情）

**五大分析维度：**

| 维度 | 检查内容 | 检查项数 |
|------|---------|---------|
| `complexity` | 时间复杂度 / 空间复杂度 / 可扩展性 | 5 项 |
| `correctness` | 边界条件 / 逻辑完备性 / 异常处理 | 5 项 |
| `optimization` | 性能优化 / 可读性 / 可维护性 / 重复代码 | 5 项 |
| `security` | 注入风险 / 溢出风险 / 权限问题 / 敏感数据 | 5 项 |
| `data_flow` | 变量生命周期 / 依赖关系 / 数据一致性 / 状态管理 | 5 项 |
| `all` | 以上全部维度综合分析 | 25 项 |

**HTTP API：** `POST /api/alliance/algorithm-analysis`

**分析结果包含：**
- 各维度检查项明细（通过/未通过/阻断级别）
- 总体通过率
- 是否触发阻断（vetoed）
- 摘要说明
- 优化建议列表
- 分析耗时

---

### 2.9 专家概览/指标（Overview / Metrics）

**状态：已补全**

**原有实现：** 无

**补全内容：**
- `types.rs`：新增 `AllianceOverview` / `AllianceMetrics` DTO
- `services.rs`：`AllianceService::overview()` + `AllianceService::metrics()`
- `server.rs`：新增 `GET /api/alliance/overview` + `GET /api/alliance/metrics`

**概览数据（Overview）：**
- 运行时长（uptime）
- 注册专家总数
- 覆盖领域数
- 总咨询次数
- 总辩论次数
- 总全量分析次数
- 平均辩论耗时
- 平均门禁评分
- 平均学习产出数

**指标数据（Metrics）：**
- 意图分布（intent_distribution）
- 领域分布（domain_distribution）
- 专家能力评分排行（top_experts）
- 各维度通过率（dimension_pass_rates）
- 延迟分布（latency_percentiles: p50/p90/p99）

---

## 三、API 端点完整清单

### 3.1 专家管理

| 方法 | 路径 | 功能 | 请求体 | 响应体 |
|------|------|------|--------|--------|
| POST | `/api/alliance/experts/register` | 注册专家 | `RegisterExpertRequest` | `RegisterExpertResponse` |
| GET | `/api/alliance/experts` | 专家列表 | Query: `domain?`, `limit?` | `{ experts: ExpertInfo[] }` |
| GET | `/api/alliance/experts/:id` | 专家详情 | Path: `id` | `ExpertInfo` 或 404 |

### 3.2 智能路由

| 方法 | 路径 | 功能 | 请求体 | 响应体 |
|------|------|------|--------|--------|
| POST | `/api/alliance/route` | 智能路由匹配 | `RouteExpertsRequest` | `RouteExpertsResponse` |

### 3.3 专家咨询

| 方法 | 路径 | 功能 | 请求体 | 响应体 |
|------|------|------|--------|--------|
| POST | `/api/alliance/consult` | 单专家咨询 | `ConsultExpertRequest` | `ConsultExpertResponse` |
| POST | `/api/alliance/multi-consult` | 多专家协同 | `MultiConsultRequest` | `MultiConsultResponse` |

### 3.4 专家辩论

| 方法 | 路径 | 功能 | 请求体 | 响应体 |
|------|------|------|--------|--------|
| POST | `/api/alliance/debate` | 专家辩论（同步） | `ExpertDebateRequest` | `ExpertDebateResponse` |
| GET | `/api/alliance/debate/stream` | 专家辩论（SSE 流） | Query: `query`, `team_size?` | SSE: `done` event |

### 3.5 mox 模块化系统架构分析

| 方法 | 路径 | 功能 | 请求体 | 响应体 |
|------|------|------|--------|--------|
| POST | `/api/alliance/full` | mox 模块化系统架构分析（同步） | `FullAnalysisRequest` | `FullAnalysisResponse` |
| GET | `/api/alliance/full/stream` | mox 模块化系统架构分析（SSE 流） | Query: `query` | SSE: `done` event |

### 3.6 任务编排

| 方法 | 路径 | 功能 | 请求体 | 响应体 |
|------|------|------|--------|--------|
| POST | `/api/alliance/orchestrate` | 执行编排任务 | `OrchestrationRequest` | `OrchestrationResponse` |

### 3.7 算法分析

| 方法 | 路径 | 功能 | 请求体 | 响应体 |
|------|------|------|--------|--------|
| POST | `/api/alliance/algorithm-analysis` | 算法分析 | `AlgorithmAnalysisRequest` | `AlgorithmAnalysisResponse` |

### 3.8 概览与指标

| 方法 | 路径 | 功能 | 请求体 | 响应体 |
|------|------|------|--------|--------|
| GET | `/api/alliance/overview` | 联盟概览 | - | `AllianceOverview` |
| GET | `/api/alliance/metrics` | 联盟指标 | - | `AllianceMetrics` |

---

## 四、文件变更清单

### 4.1 新建文件

| 文件 | 说明 | 代码行数（约） |
|------|------|---------------|
| `src/alliance/orchestration.rs` | 任务编排引擎 | ~380 行 |
| `src/alliance/algorithm.rs` | 算法分析引擎 | ~450 行 |

### 4.2 修改文件

| 文件 | 修改内容 |
|------|---------|
| `src/types.rs` | 新增 ~25 个 DTO 结构体（注册/咨询/辩论/编排/算法/概览等） |
| `src/services.rs` | 新增 `AllianceService` 结构体（统一门面，~500 行） |
| `src/server.rs` | 新增 13 个 API 路由及对应 handler（~400 行） |
| `src/alliance/mod.rs` | 新增 `orchestration` 和 `algorithm` 模块声明 |
| `src/bin/mox.rs` | AppState 新增 `alliance` 字段 |
| `Cargo.toml` | 新增 `futures-util` 依赖（SSE 流所需） |

---

## 五、架构设计说明

### 5.1 分层架构

```
┌─────────────────────────────────────────┐
│          HTTP 层 (server.rs)            │
│  路由注册 + Handler + 请求/响应转换       │
├─────────────────────────────────────────┤
│        服务门面层 (services.rs)          │
│  AllianceService - 统一业务入口           │
│  (注册/咨询/辩论/编排/算法/指标)          │
├─────────────────────────────────────────┤
│        联盟核心层 (alliance/)            │
│  intent / team / debate / gate / learn   │
│  orchestration / algorithm / kg_connector│
├─────────────────────────────────────────┤
│        Trait 抽象层 (expert_traits.rs)   │
│  ExpertRegistry / ExpertConsultant / ... │
├─────────────────────────────────────────┤
│        类型层 (types.rs)                 │
│  DTO / 共享结构体 / 错误类型              │
└─────────────────────────────────────────┘
```

### 5.2 AllianceService 设计

`AllianceService` 作为**业务门面（Facade）**，统一封装所有专家联盟功能：
- 内部协调 `RegistryImpl` / `ExpertServiceImpl` / `AllianceRouter`
- 持有 `AlgorithmAnalyzer`（带状态：分析次数统计）
- 持有 `OrchestrationEngine`（带状态：任务跟踪）
- 提供原子计数器（咨询次数、辩论次数、全量分析次数）
- 持有意图分布统计 HashMap
- 持有专家历史评分记录（用于 metrics 计算）

---

## 六、编译验证结果

```
$ cargo check --lib
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo check --bin mox
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

- 库编译：通过，0 errors
- 二进制编译：通过，0 errors
- 仅存警告：均为原有代码中的未使用字段/导入（非本次修改引入）

---

## 七、后续优化建议

1. **SSE 真正流式**：当前 SSE 实现为"批量执行后单事件推送"。建议使用 `tokio::sync::mpsc` channel 实现逐阶段流式推送，让前端实时看到管线进度。

2. **持久化存储**：当前专家注册、任务状态均为内存存储。建议接入数据库（如 PostgreSQL）或 Redis 实现持久化。

3. **真实 LLM 接入**：当前专家咨询为规则模拟。建议接入大模型 API（如 OpenAI / 豆包 / 通义千问）实现真正的 AI 专家。

4. **知识图谱集成**：`kg_connector` 模块已有雏形，建议完善知识图谱的查询和写入能力，增强专家领域知识。

5. **认证鉴权**：当前所有 API 均为公开访问。建议接入 API Key 或 OAuth2 认证。

6. **限流与熔断**：高并发场景下建议添加限流（rate limiting）和熔断（circuit breaker）机制。

7. **前端对齐**：需与前端 `alliance.ts` 中的 API 定义逐一核对，确保字段命名和数据结构完全一致。
