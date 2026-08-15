# 算子统一系统（OUS）企业级架构设计文档 v7.0

> 参考范式：[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) —— "Everything is a Plugin"（一切皆插件）
> 设计目标：在保留 OUS 范畴论/希尔伯特空间数学内核与 WASM 沙箱能力的基础上，引入**插件化运行时内核**、**会话日志溯源（Session Log as Source of Truth）**、**能力接缝（Seam）可替换架构**与**Turn/Agent 生命周期**，实现可组合、可审计、可热插拔的企业级一体化平台。

---

## 0. 文档导航

| 章节 | 内容 |
|------|------|
| 1. 设计哲学 | 从"算子框架"到"算子插件生态"的范式跃迁 |
| 2. 总体架构 | 分层拓扑 + 插件内核（Cordis 式） |
| 3. 插件内核设计 | Profile / Bundle / Seam / 事件域 |
| 4. 数学内核 | 6 大公理在架构中的落点 |
| 5. 运行时与生命周期 | Turn/Agent/Step、Waterfall 事件、会话日志 |
| 6. 编排与优化层 | FlowAI、Optimizer、专家联盟 |
| 7. 接入层 | API 网关、鉴权、实时流 |
| 8. 数据层 | 状态向量、知识图谱、持久化 |
| 9. 明确业务处理流程 | 13 条流程卡(端点/阶段/SLA/异常) + 状态机 + 跨流程编排 |
| 10. 企业级能力 | 可观测性、治理、安全(STRIDE)、多租户 |
| 11. 部署与交付 | 容器化、灰度、插件热加载、桌面打包 |
| 12. 迁移路线图 | 基于现有 crate 的重构步骤 |
| 13. 全形态产品矩阵 | 云/本地运行 · 云/本地 LLM · 浏览器 · 桌面打包 |
| 14. 插件 SDK | 算子开发契约 + bundle 清单 + 前端市场契约 |
| 15. CI/CD | 质量门禁 + 公理校验 + 灰度回滚 |
| 16. 灾备 | WAL 重放 + 快照 + 混沌工程 + RTO/RPO |
| 17. 成本模型 | 四形态 FinOps 对比 |
| 18. 开放生态 | 算子市场 + 跨形态同步 + 网络效应 |
| 19. 专家联盟全维处理内核 | 最高权限 · 璇玑 · 全维业务编排 |
| 20. 融合对标与产品定位 | 与 harness/Claude Code 的差异与优势 |
| 21. 沙箱安全纵深 | WASM 沙箱 + 能力令牌 + 纵深防御 |
| 22. 多模态与感知 | 文本/图/音/视频/结构化统一算子 |
| 23. 记忆与知识管理 | 短期/长期/程序性记忆 + 知识复利 |
| 24. 评测与回归 (Eval) | 公理门禁 + 行为回归 + 基准 |
| 25. i18n 与无障碍 | 多语言 + 无障碍 + 低带宽 |
| 26. 版本与兼容治理 | 语义化版本 + 兼容契约 + 升级 |
| 27. 路径与运行态隔离 | 架构代码路径 vs 工作路径 严格分离规范 |
| 28. 业务流程设计模块 | 可视化设计器·节点体系·DSL·校验·版本·模板市场 |

---

## 1. 设计哲学：Everything is an Operator Plugin

DeepSeek Harness 的核心主张是"**一切皆插件**"——没有任何特权核心，模型适配器、工具注册表、会话日志、Agent 循环本身都是插件，均可通过配置替换、挂载、卸载。OUS 将这一范式与自身数学内核融合，提出升级版主张：

> **一切皆算子（Operator），一切算子皆插件（Plugin）。**

### 1.1 双内核模型

| 维度 | DeepSeek Harness | OUS 升级 |
|------|------------------|----------|
| 底层框架 | Cordis（时空可组合） | **OUS-Cordis**：Rust 实现的插件上下文内核 `ctx` |
| 单元 | Plugin | **Operator-Plugin**（既是范畴论态射，又是可挂载插件） |
| 扩展点 | Service / Event / Effect | Service / Event / Effect **+ 范畴论组合律校验** |
| 状态 | Session Log（追加） | Session Log **+ 希尔伯特状态向量投影** |
| 卸载 | 自动撤销注册 | 自动撤销注册 **+ 守恒律回滚校验** |

### 1.2 设计原则

1. **无特权核心**：`runtime` 只提供 `ctx` 上下文与插件加载器，所有业务功能（算子执行、AI 编排、图谱、优化）都以插件挂载。
2. **可逆性**：插件 `mount` 时注册的所有 Service/Event Handler，在 `unmount` 时自动反注册（参考 `operator-core` 的 `monad` 单子模式封装副作用）。
3. **会话溯源**：模型/智能体可见的上下文**必须**来自追加式 Session Log，保证可重放、可审计（对应公理 2 状态向量不变式）。
4. **能力接缝**：文件系统、子进程、LLM、数据库等"外部世界"均通过 Seam 抽象，换实现即全局生效（参考 Harness 的 `fs/*`、`tools/*`、`telemetry/*` 接缝）。
5. **类型即契约**：算子组合通过 `TypePair::can_compose` 编译期/运行期双校验（公理 4 范畴论律）。

---

## 2. 总体架构（分层 + 插件内核）

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                      接入层 (Ingress / Edge)                                    │
│   Web UI (Vue3+Three.js)   REST/OpenAPI   WebSocket/SSE    CLI (dsh 式)          │
│   └─ API Gateway: 鉴权 · 限流 · 租户路由 · 请求追踪 · 协议转换                     │
└───────────────┬───────────────────────┬───────────────────────┬──────────────┘
                │                        │                       │
                ▼                        ▼                       ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                   插件运行时内核 (OUS-Cordis Runtime)                            │
│  ctx 上下文树 ──▶ Profile/Bundle 加载器 ──▶ 事件总线(EventBus) ──▶ Seam 注册表   │
│  ├─ core/session      会话日志(追加式源)      ctx.sessions                       │
│  ├─ core/operator     算子注册与组合校验       ctx.operators                      │
│  ├─ core/system-prompt 提示/工具 schema 组装  ctx.systemPrompt                   │
│  ├─ core/agent        Agent 注册表 + agent/* 事件  ctx.agents                    │
│  ├─ core/agent-loop   Turn/Step 驱动          ctx.agentLoop                     │
│  ├─ core/scope        每 Agent 作用域原语       (库, 无键)                       │
│  └─ llm/llm           消息/流词汇 + 适配器接缝  ctx.llm                          │
└───────────────┬───────────────────────┬───────────────────────┬──────────────┘
                │                        │                       │
                ▼                        ▼                       ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                      编排与优化层 (Orchestration)                                │
│  flow-ai: 拓扑/数据流/关键路径/冲突消解/调度/代码生成                             │
│  optimizer: DAG 调度 & 资源约束 & 关键路径优化 (ctx.scheduler)                    │
│  ai-agent: 工作流引擎/对话/浏览器自动化/插件总线 (ctx.workflow, ctx.browser)       │
│  expert-alliance: 多专家协同/IR/治理/验证 (ctx.experts, ctx.govern)               │
│  hermes-flow-bridge: 外部流系统对接/录制/回放 (ctx.bridge)                        │
└───────────────┬───────────────────────┬───────────────────────┬──────────────┘
                │                        │                       │
                ▼                        ▼                       ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                      算子内核 (Operator Core)                                    │
│  operator-core: 算子 trait / 高维向量 / 范畴论 / 单子 / 守恒律                    │
│  operator-graph: 加权有向图 / PageRank / 拉普拉斯矩阵 / 社群发现                  │
│  operator-wasm: WASM 沙箱执行 / 热加载插件 (Seam: ctx.wasm)                       │
│  business-catalog: 业务算子目录 (算子注册表的持久化实现)                          │
└───────────────┬───────────────────────┬───────────────────────┬──────────────┘
                │                        │                       │
                ▼                        ▼                       ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                      数据 / 执行 / 外系统层                                      │
│  状态存储(向量DB)  图存储(Neo4j/PG)  业务库  消息队列  第三方 API  WASM 插件沙箱  │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 2.1 与现有 crate 的映射

| 现有 crate | 新架构角色 | ctx 键 |
|-----------|-----------|--------|
| `runtime` | 接入层 + 插件加载器（瘦身后仅保留 `ctx` 与路由） | `ctx`（根） |
| `operator-core` | `core/operator` + `core/scope` + `core/state` | `ctx.operators` |
| `operator-graph` | `core/graph`（会话日志的图投影） | `ctx.graph` |
| `operator-wasm` | `core/wasm`（Seam 提供者：外部算子执行世界） | `ctx.wasm` |
| `flow-ai` | `orchestration/flow` | `ctx.flow` |
| `optimizer` | `orchestration/scheduler` | `ctx.scheduler` |
| `ai-agent` | `core/agent` + `core/agent-loop` + `agent/browser` | `ctx.agents` |
| `expert-alliance` | `core/experts` + `govern` | `ctx.experts` |
| `business-catalog` | 算子注册表持久化实现（而非核心） | `ctx.catalog` |
| `hermes-flow-bridge` | 外部流系统 Seam 适配 | `ctx.bridge` |

---

## 3. 插件内核设计（OUS-Cordis）

### 3.1 Profile 与 Bundle

借鉴 Harness 的 `dsh.profile` / `dsh.bundle` 声明式组合：

- **Profile（配置档案）**：命名组合，存于 Harness Home（OUS 中为 `$OUS_HOME/config`）。模板：
  - `web`：Web UI + API 网关 + 全部编排能力
  - `headless`：无 UI 一次性批处理运行器（CLI/CI）
  - `edge`：仅算子内核 + WASM，资源受限边缘节点
- **Bundle（包）**：Cordis 配置行 + 挂载代码的发行格式。每个 crate 的 `Cargo.toml` 增加 `[package.metadata.ous]` 段声明 `bundle` 与依赖的 `profile`。
- **声明方式**：
  ```toml
  [package.metadata.ous]
  bundle = "operator-core"
  provides = ["ctx.operators", "ctx.state"]
  requires = []
  ```

### 3.2 加载顺序（Layering）

```
空入口
  → profile 所列 bundle 顺序加载
    → profile 的 patch (cordis.patch.yml)
      → home 级 patch ($OUS_HOME/cordis.patch.yml)
        → --patch 命令行覆盖层
```

调试命令（对标 `dsh --profile web --dump-config`）：
```bash
operator-server --profile web --dump-config
```
输出实际启动的插件树与 Seam 绑定，用于审计"到底加载了哪些算子/适配器"。

### 3.3 能力接缝（Seam）

可替换能力的三角色：**服务定义 / 服务提供者 / 消费者**。换一个提供者即全局改变产品行为。

| Seam 域 | 事件域 | 默认实现 | 可替换示例 |
|---------|--------|----------|-----------|
| `llm/*` | `llm/stream` | DeepSeek/OpenAI 云端适配 | 本地 Ollama/vLLM/llama.cpp、Azure、通义、文心、混合隧道（§13.3） |
| `wasm/*` | `wasm/execute` | Wasmer 沙箱 | WASMEdge、WASI-NN |
| `fs/*` | `fs/read\|write` | 本地文件系统 | S3、对象存储、Git |
| `shell/*` | `shell/spawn` | 本地子进程 | 远程沙箱、K8s Job |
| `db/*` | `db/query` | PostgreSQL | MySQL、ClickHouse |
| `telemetry/*` | `telemetry/emit` | OTel stdout | Prometheus、Jaeger、Loki |
| `graph/*` | `graph/mutate` | petgraph 内存 | Neo4j、TuGraph |

### 3.4 事件域（Event Domains）

插件通过事件挂载行为，不形成导入环：

- **会话事件**（`session/*`）：持久事实，追加日志并广播 `session/event`，重载后存活。
- **Agent 事件**（`agent/*`）：携带活 Agent，观察/拦截进行中工作（见 §5）。
- **算子事件**（`operator/*`）：算子注册/组合/执行前后钩子。
- **能力事件**（`llm/*`、`wasm/*`、`fs/*`、`telemetry/*` 等）：向 Seam 附策略/适配器。

---

## 4. 数学内核（6 大公理在架构中的落点）

| # | 公理 | 架构落点 | 校验方式 |
|---|------|----------|----------|
| 1 | 万物皆算子 | 每个 Plugin 暴露 `Operator` trait；runtime 无特权核心 | `operator-core::operator::Operator` |
| 2 | 状态高维向量 | `StateVector` 为会话日志的投影；模型上下文由日志派生 | `state::derive_messages()` 不变式 |
| 3 | 关联关系加权有向图 | `operator-graph` 作为知识图谱 Seam 实现 | 邻接矩阵/拉普拉斯/PageRank（`graph/centrality`） |
| 4 | 范畴论态射规则 | 算子组合满足单位律/结合律；`TypePair::can_compose` 双校验 | `category` 模块单测 + `verify_axioms.py` |
| 5 | 资源约束优化 | `optimizer` 的 DAG 调度受 `resource::ResourcePool` 约束 | `schedule::efficiency()` 指标 |
| 6 | 扩展性闭包 | `monad` 封装副作用；WASM 插件通过单子接口注入 | `monad` 三定律测试 |

**守恒律（贯穿全局）**：`conservation::ConservationChecker` 在每次 Turn 结束校验 `StateVector` 的 L1（概率）、L2（能量）、Sum 守恒，残差超阈值触发 `residual_monitor` 告警与回滚。

---

## 5. 运行时与生命周期（Turn / Agent / Step）

将 Harness 的 Agent Loop 与 OUS 的算子执行统一为**统一生命周期**。

### 5.1 概念

- **Step（步）**：一次模型请求 + 工具/算子调用。
- **Turn（轮）**：零或多个 Step；首个输入认领前开启，无欠账时关闭。
- **Session（会话）**：追加式 `SessionEvent` 日志，跨 Turn 持久。

### 5.2 Waterfall 事件流

```
turn/start
  → 认领输入 + 入队消息
  → 组装提示(systemPrompt) + 算子/工具 schema
  → agent/pre-step        (可拒|可改写)  ── 调 next()
  → step/start
      → 追加 user/message
      → 派生历史(deriveMessages)
      → agent/request → llm/stream → assistant/* → operator/* → tool/*
  → step/end
  → [若欠请求或新输入到达 → 下一步]
  → agent/turn-stopping   (串行, 无 next())
  → turn/end
      → conservation::check_all(state)   # 公理守恒校验
      → session/event 追加
```

需要 `next()` 的可拦截事件：`agent/pre-step`、`agent/request`、`llm/stream`、`operator/pre-execute|execute|post-execute`、`tools/pre-execute|execute|post-execute`。

### 5.3 输入模型

- 单一 Inbox（对标 Harness 单 inbox）；部分消息即时唤醒 Agent，其余注入上下文等待下次消息。
- `pre-step` 决定模型所见：可改写、拒认领；拒认领或空首认领仍关闭耐久 Turn（记录尝试次数，防饥饿）。

### 5.4 会话日志为源（Session Log as Source of Truth）

- 模型上下文 = `SessionEvent` 日志的投影（`derive_messages()`）。
- 原始 `assistant/chunk` 保留以支持重放/UI 流式渲染。
- **不变式**：模型可见即已日志；新模型可见输入必须是新的 session 事件。扩展需在 `SessionEventMap` 注册并从日志渲染。
- 派生用途：fork（`ctx.sessions.fork()`）、resume、转录、遥测、持久化，均源于此流。

### 5.5 作用域（Scope）

- 用 `agent.ctx` 将注册限定于单 Agent（多租户隔离）。
- `ctx.sessions.fork()` 支持会话分叉（A/B 实验、假设推演）。

### 5.6 端到端时序图（典型 AI 对话 + 算子编排）

```
用户浏览器           API网关            agent-loop         llm/* Seam        operator-wasm       core/session
   │                   │                    │                   │                 │                 │
   │─ POST /api/ai/chat─▶│                    │                   │                 │                 │
   │                   │─ 鉴权/租户注入 ─▶    │                   │                 │                 │
   │                   │                    │─ turn/start ───────────────────────────────────────────▶│
   │                   │                    │─ 认领输入+入队      │                 │                 │
   │                   │                    │─ systemPrompt 组装  │                 │                 │
   │                   │                    │─ agent/pre-step ──▶│                 │                 │
   │                   │                    │─ step/start        │                 │                 │
   │                   │                    │─ derive_messages()─▶│                 │                 │
   │                   │                    │─ agent/request ──────────────────────▶│                 │
   │                   │                    │                   │─ stream chunk ──▶│                 │
   │◀── SSE chunk ─────│◀──────────────────│◀──────────────────│                 │                 │
   │                   │                    │   (重复直到 stop)   │                 │                 │
   │                   │                    │─ operator/pre-execute ───────────────▶│                 │
   │                   │                    │                   │                 │─ WASM 沙箱执行 ─▶│
   │                   │                    │─ operator/post-execute◀──────────────│                 │
   │                   │                    │─ agent/turn-stopping (无 next)        │                 │
   │                   │                    │─ conservation::check_all() ───────────────────────────▶│
   │                   │                    │   (失败→回滚至 Turn 前)              │                 │
   │                   │                    │─ turn/end ────────────────────────────────────────────▶│
   │◀── 完整响应 ──────│◀──────────────────│                   │                 │                 │
```

**关键不变量**：`core/session` 在 `turn/start` 与 `turn/end` 之间追加所有事实事件；`conservation` 校验失败触发回滚，保证状态向量守恒（公理 6 + 守恒律）。

---

## 6. 编排与优化层

### 6.1 FlowAI（拓扑 / 数据流 / 关键路径 / 冲突消解 / 调度 / 代码生成）

- `topology`：实体-关系拓扑图，支持语义搜索、最短路径、路由规划、影响集分析。
- `conflict`：并发组冲突检测 + `auto_repair`（自动修复拓扑冲突）。
- `schedule`：基于依赖的 Slot/Pool 调度 + `route_models`（模型路由）。
- `codegen`：流程 → Python/Rust 代码生成 + `reverse_from_python`（逆向工程已有脚本为算子）。
- `pipeline`：拓扑驱动的优化报告（`OptimizationReport`），量化增益。

### 6.2 Optimizer（DAG 调度 & 资源约束）

- 关键路径分析（CPM）、资源池约束（`resource::ResourcePool`）、调度效率指标。
- 与 `flow-ai::schedule` 协同：FlowAI 出拓扑，Optimizer 出可执行 DAG 调度计划。

### 6.3 AI-Agent（工作流 / 对话 / 浏览器自动化 / 插件总线）

- `workflow_engine`：业务工作流注册/模板/实例管理（对接 `business-catalog`）。
- `conversation`：会话引擎（`get_or_create_session`），桥接 §5 的 Session Log。
- `browser_automation`：浏览器会话/任务/自然语言解析（`parse_natural_language`）。
- `flow_engine`：可视化流程定义/校验/执行（对接前端 Three.js 流程图）。
- `algorithm`：算法复杂度分析（`AlgorithmAnalyzer`）。

### 6.4 Expert-Alliance（多专家协同 / IR / 治理 / 验证）

- 多专家联盟求解、信息检索管线、治理策略（`govern`）、结果验证。
- 关键场景：专家间结论冲突时，由 `flow-ai::conflict` 消解 + `conservation` 校验收敛。

### 6.5 Hermes-Flow-Bridge（外部流系统对接）

- 对接、录制、回放、状态同步；作为 `bridge` Seam，将外部流（如 Hermes）纳入统一算子图。

---

## 7. 接入层（Edge / Ingress）

### 7.1 API 网关职责

| 能力 | 实现 |
|------|------|
| 鉴权 | JWT/OAuth2 + 租户上下文注入（`ctx.auth`） |
| 限流 | 令牌桶，按租户/算子维度（`telemetry/*`） |
| 租户路由 | `agent.ctx` 作用域隔离 |
| 请求追踪 | `trace_id` 贯穿 Waterfall 事件 |
| 协议转换 | REST ↔ WebSocket/SSE ↔ CLI |

### 7.2 现有 REST API 归类（已存在于 `runtime/src/main.rs`）

- **算子**：`/api/operators`、`/api/execute`、`/api/plugins`
- **图谱**：`/api/graph/*`（node/edge/neighbors/centrality/communities/path/pagerank/activate/recommend）
- **AI**：`/api/ai/chat`、`/api/ai/flows`、`/api/ai/workflows`、`/api/ai/browser`、`/api/ai/llm`、`/api/analyze/spiral`
- **算子商城（§18 资产侧）**：`/api/market/`（列表/过滤）、`/api/market/random`（随机）、`/api/market/:id`（详情/更新）、`/api/market/upload`（上传）、`/api/market/:id/clone`（克隆）、`/api/market/:id`（DELETE）。GET 接口免登录白名单，写操作受 §7.1 鉴权。数据契约与前端见 `docs/market-module.md`。
- **系统**：`/api/health`、`/api/status`、`/api/logs`

> 重构后：路由由 `runtime` 插件挂载，每个 `ctx.*` 子系统声明自己的路由组，网关统一聚合。

### 7.3 实时流

- WebSocket/SSE 推送 `session/event` 与 `assistant/chunk`，前端 Three.js 实时渲染状态向量与算子 DAG。

### 7.4 运行形态通道（Run Mode Channels）

接入层对四种产品形态（§13）统一抽象为同一组 Channel，差异仅在部署拓扑：

| 形态 | 前端通道 | 后端位置 | LLM 通道 |
|------|----------|----------|----------|
| 云+云 | HTTPS (CDN) | 云 VM/容器 | `llm/*` → 云端 API |
| 本地+本地 | http://localhost:3000 | 本机二进制 | `llm/*` → localhost:11434 |
| 云+本地 | HTTPS | 云 VM | `llm/*` → TLS 隧道回本地 |
| 本地+云 | http://localhost:3000 | 本机二进制 | `llm/*` → 云端 API |
| 桌面 App | Tauri WebView (tauri://localhost) | 同二进制内嵌 | `llm/*` 可云可本地 |

> 网关不感知形态差异：所有形态经同一套 REST/WS 接口与 `cordis.patch.yml` 配置驱动，真正"一次编排、处处运行"。

---

## 8. 数据层

| 数据 | 存储 | 对应模块 |
|------|------|----------|
| 会话日志（追加式） | 事件流存储（WAL / Kafka） | `core/session` |
| 状态向量 | 向量库（内存+持久化） | `operator-core::state` |
| 知识图谱 | 图库（petgraph/Neo4j） | `operator-graph` |
| 算子注册表 | 关系库 + 业务目录 | `business-catalog` |
| 执行日志/指标 | 时序库（Prometheus/Loki） | `telemetry/*` |

**不变式**：状态向量与图谱均为会话日志的投影，重放日志可完整重建系统状态（对应公理 2）。

---

## 9. 明确业务处理流程（Business Process Specification）

> 本章把"功能"升级为**可执行的业务处理流程**：每条流程用统一模板描述 —— 触发(端点) → 处理阶段(真实 crate/函数) → 输出 → SLA → 异常分支。所有流程遵循统一的状态机与处理流水线，保证"模块化、业务流程化"。

### 9.0 业务处理流水线通用模板（Business Process Pipeline）

任何业务请求都流经以下标准阶段（与 §5 Waterfall 事件对齐）：

```
[接入] → [鉴权/租户路由] → [意图归一化→状态向量投影(公理2)]
   → [编排决策: 单算子 / FLOW / SUPER_EXPERT(§19)]
   → [执行: Seam 调用 + 算子 DAG + 守恒校验(§4 公理6)]
   → [出参: 结构化响应 / SSE 流 / 工作流实例]
   → [沉淀: session/event 追加 + 图谱加权边(§23)]
   → [可观测: telemetry/* 埋点(§10)]
```

**统一状态机**（复用 `ai-agent::types` 真实定义）：

```
Pending ──▶ Running ──┬─▶ Completed
                       ├─▶ WaitingUser (用户任务/审批接缝 approval/*)
                       └─▶ Failed ──┬─▶ Retry(指数退避, ≤3) ──▶ Running
                                    └─▶ Rollback(守恒律回滚, §5.4)
节点级 NodeStatus: Pending → Running → Completed / Failed / Skipped(条件分支未命中)
```

### 9.1 核心业务处理流程卡（13 条，基于 runtime 真实端点）

每条格式：**触发端点 → 阶段 → 输出 → SLA → 异常**。

#### P-01 算子注册流程
- **触发**：`POST /api/operators/register`（`register_operator`）
- **阶段**：① 校验算子元数据(name/类型/in-out schema) → ② `TypePair::can_compose` 类型契约校验(公理4) → ③ `business-catalog` 持久化注册 → ④ 广播 `operator/*` 事件 → ⑤ `core/operator` 加载可用
- **输出**：`OperatorInfo` 列表更新 / 注册确认
- **SLA**：< 100ms（不含 WASM 编译）
- **异常**：类型不兼容→拒绝并记录；WASM 校验失败→`Failed`+审计

#### P-02 算子执行流程
- **触发**：`POST /api/execute`
- **阶段**：① 鉴权 → ② 构建 DAG(`flow-ai::topology`) → ③ `optimizer` 关键路径调度 → ④ `operator-wasm` 沙箱执行(能力令牌 §21) → ⑤ `conservation::check_all` 守恒校验 → ⑥ SSE 推回
- **输出**：执行结果 + `ExecutionLog`(`get_logs`)
- **SLA**：P99 < 50ms/call（算子本身）
- **异常**：超时→`guard/` 熔断回 `Failed`；守恒残差超阈→回滚至 Turn 前

#### P-03 AI 对话流程
- **触发**：`POST /api/ai/chat`（`ai_chat`）
- **阶段**：① `conversation` 取/建会话 → ② `systemPrompt` 组装(含 RAG 检索 §23) → ③ `agent-loop` Turn → ④ `llm/stream` 流式 → ⑤ `session/event` 追加 → ⑥ SSE 流式响应
- **输出**：`ChatResponse` + 流式 chunk
- **SLA**：TTFT 本地<800ms / 云<600ms（§9.2）
- **异常**：LLM 宕机→混沌恢复(§16)；上下文超限→`derive_messages` 截断重投影

#### P-04 工作流编排流程（BPMN 风格）
- **触发**：`POST /api/ai/workflows/execute`（`execute_business_workflow`）
- **阶段**：① 加载 `BusinessWorkflow`(`workflow_engine`) → ② 校验节点(顺序/条件/并行/子流程/用户任务/AI任务/插件/算子) → ③ 生成 `WorkflowInstance`(状态机 §9.0) → ④ 逐节点执行→`node_executions` → ⑤ 汇总结论 `WorkflowResult`
- **输出**：`WorkflowResult`(completed/failed_nodes 计数) + 实例(`list_workflow_instances`)
- **SLA**：单节点<1s；整流程依节点数线性
- **异常**：`WaitingUser` 挂起待人工；无限循环→步数超限拦截(`flow_engine` 已含"执行步数超限"防护)；失败→状态机 `Failed`+重试

#### P-05 流程画布校验/执行流程
- **触发**：`POST /api/ai/flows`（`create_flow`/`validate_flow`/`execute_flow`）
- **阶段**：① `flow_engine::validate_flow` 拓扑校验 → ② `agent/turn-stopping` 触发 `execute_flow` → ③ DAG 驱动执行 → ④ Three.js 实时渲染
- **输出**：流程定义 / 校验报告 / 执行结果
- **SLA**：校验<200ms
- **异常**：环依赖→校验失败拒绝

#### P-06 浏览器自动化流程
- **触发**：`POST /api/ai/browser/natural`（`browser_natural`）
- **阶段**：① `parse_natural_language` 自然语言解析 → ② `browser_automation` 建会话/任务 → ③ 执行步骤(`execute_browser_steps`) → ④ 结果回写会话
- **输出**：任务执行结果 JSON
- **SLA**：单步<2s
- **异常**：`NavigationFailed`/`InteractionFailed`→重试+记录

#### P-07 知识图谱构建/分析流程
- **触发**：`POST /api/graph/node` | `/api/graph/edge`（`add_node`/`add_edge`）
- **阶段**：① 加节点/边 → ② `operator-graph` 计算 `centrality`/`communities`/`pagerank` → ③ 前端 3D 力导向渲染
- **输出**：`GraphData`(`get_graph`) + `GraphStats`
- **SLA**：PageRank 10w 节点<2s（§9.2）
- **异常**：重复边→幂等；权重非法→拒绝

#### P-08 冲突消解毒
- **触发**：并发算子组提交
- **阶段**：① `flow-ai::conflict::detect` → ② `auto_repair` 自动修复 → ③ `optimizer` 重调度
- **输出**：修复后 DAG
- **SLA**：<500ms
- **异常**：无法自动修复→升级 `SUPER_EXPERT`(§19)

#### P-09 资源优化流程
- **触发**：DAG + `ResourcePool` 提交
- **阶段**：① `optimizer` 关键路径(CPM) → ② `efficiency` 指标 → ③ `OptimizationReport`
- **输出**：调度计划 + 增益报告
- **SLA**：<300ms
- **异常**：约束不可满足→返回松弛建议

#### P-10 专家协同流程
- **触发**：多专家结论汇聚
- **阶段**：① `expert-alliance` 聚合 → ② `conflict` 消解 → ③ `conservation` 收敛校验 → ④ 加权边沉淀(§23)
- **输出**：共识结论 + 图谱强化
- **SLA**：<1s
- **异常**：分歧超阈→`SUPER_EXPERT` 仲裁(§19)

#### P-11 外部流对接流程
- **触发**：`hermes-flow-bridge` 事件
- **阶段**：① 录制外部事件 → ② 映射为算子 → ③ 入统一图 → ④ 回放
- **输出**：统一算子实例
- **SLA**：录制<50ms/事件
- **异常**：映射失败→死信队列

#### P-12 插件热加载流程
- **触发**：`plugins/*.wasm` 落盘
- **阶段**：① 目录监听 → ② `core/wasm` 原子热加载 → ③ `operator/*` 注册 → ④ 不重启生效
- **输出**：算子列表更新(`list_plugins`)
- **SLA**：单 bundle<1s
- **异常**：加载失败→保留旧版+告警

#### P-13 SUPER_EXPERT 全维处理流程
- **触发**：用户以 `SUPER_EXPERT` 模式发起
- **阶段**：① 收口→状态向量投影 → ② 专家联盟分诊 → ③ 璇玑产出 DAG → ④ All-Domain Bus 跨子系统执行 → ⑤ 守恒收敛 → ⑥ 治理复核(`govern`) → ⑦ 沉淀图谱 + 自进化提案
- **输出**：跨域求解结果 + 算子市场提案
- **SLA**：依复杂度，流式反馈
- **异常**：任一子系统越权→`approval/*` 拦截；守恒失败→全量回滚

### 9.2 业务处理 SLA 总览（性能容量模型）

| 流程 | 关键 SLA | 形态差异 |
|------|----------|----------|
| P-03 对话 | TTFT 本地<800ms / 云<600ms | 本地 LLM 直连 vs 边缘 PoP |
| P-02 算子 | P99<50ms/call | WASM 编译缓存 |
| P-07 图谱 | PageRank 10w<2s | 可选 Neo4j 卸载 |
| P-12 热加载 | <1s/bundle | 原子替换 |
| 状态重建 | 10k 事件<200ms | 增量投影+快照 |

> 完整性能容量指标与弹性策略见 §9.4。

### 9.3 跨流程编排状态机

- 流程间可组合：`P-04 工作流` 的"算子节点"调用 `P-02`，"AI 节点"调用 `P-03`，"浏览器节点"调用 `P-06` → **业务流程化即"把流程卡当算子连成 DAG"**。
- 统一状态机保证嵌套流程的状态传播：子流程 `Failed` → 父流程 `Failed` + 重试/回滚；`WaitingUser` 向上透传。
- `flow-ai` 负责跨流程 DAG 的拓扑/关键路径/冲突消解（§6.1），`optimizer` 负责资源约束调度（§6.2）。

### 9.4 性能与容量模型

| 指标 | 形态 | 目标值 | 设计杠杆 |
|------|------|--------|----------|
| 首 Token 延迟 (TTFT) | 本地 LLM (Ollama) | < 800ms | 本地 `llm/*` 直连 + 流式 `assistant/chunk` |
| 首 Token 延迟 (TTFT) | 云 LLM | < 600ms | 边缘 PoP + 连接复用 + `llm/stream` 背压 |
| 算子执行 P99 | WASM 沙箱 | < 50ms/call | Wasmer 编译缓存 + 配额熔断 |
| 单节点并发会话 | 云 `web` | 5k+ | `runtime` 无状态 + 会话外置（§8） |
| 状态向量重建 | 重放 10k 事件 | < 200ms | 增量投影 + 快照点 |
| 图谱 PageRank (10w 节点) | `operator-graph` | < 2s | 邻接矩阵批处理 + 可选 Neo4j 卸载 |
| 插件热加载 | 单 bundle | < 1s | `core/wasm` 监听 + 原子替换 |

**弹性策略**：云形态基于会话数 HPA；本地形态基于本机核数固定 worker 池；桌面形态单进程内 `tokio` 多工，无独立扩缩。

---

## 10. 企业级能力

### 10.1 可观测性
- 结构化日志（`tracing`）、指标（Prometheus）、链路追踪（OpenTelemetry/Jaeger）。
- `telemetry/*` Seam 统一导出；每 Turn 自动埋点 Waterfall 事件耗时。

### 10.2 治理与权限
- `expert-alliance::govern`：算子/插件白名单、租户配额、敏感算子审批。
- 多租户：`agent.ctx` 作用域隔离 + 租户级 `cordis.patch.yml`。

### 10.3 安全

- WASM 沙箱隔离第三方算子（`operator-wasm`），资源/系统调用受限（CPU/内存/网络配额 + 超时熔断）。
- 凭据管理 Seam（`credential/*`），不落明文（KMS/OS Keychain/Vault）。
- 审批接缝（`approval/*`）：高危算子执行前人工/策略审批。
- 传输安全：全形态强制 TLS（云形态 HSTS + mTLS 服务间；桌面形态本地 loopback 亦走自签名证书）。

#### 10.3.1 威胁模型（STRIDE）

| 威胁类 | 场景 | 缓解（对应架构点） |
|--------|------|-------------------|
| **S** 仿冒 | 伪造租户调用算子 | JWT/OAuth2 + `ctx.auth` 租户注入（§7.1） |
| **T** 篡改 | 篡改会话日志/状态向量 | 会话日志追加写 + 哈希链；重放校验 `derive_messages` 不变式（§5.4） |
| **R** 抵赖 | 算子执行后否认 | `session/event` 全量审计 + 不可变 WAL（§8） |
| **I** 信息泄露 | 跨租户读状态向量 | `agent.ctx` 作用域隔离 + 租户级 `cordis.patch.yml`（§5.5） |
| **D** 拒绝服务 | 恶意算子耗尽算力 | 令牌桶限流 + WASM 配额熔断 + 守恒律回滚（§10.3/§10.4） |
| **E** 提权 | 插件越权访问主机 | 无特权核心 + Seam 仅暴露受控能力，禁止直接 syscall（§1.2/§3.3） |

#### 10.3.2 本地形态的隐私边界
- **本地+本地形态（§13.2）零数据出网**：API Key、会话、状态向量全部存于本机 `$OUS_HOME`；LLM 推理在 `localhost`，不经过任何中转。
- 桌面 App（§13.6）默认离线优先，仅用户显式切"云端 LLM"时才出网，并在 UI 明示"当前数据将发送至云端"。

---

### 10.4 可靠性
- 守恒律回滚：Turn 结束 `conservation::check_all` 失败 → 状态向量回滚至 Turn 前。
- 取消与错误恢复：`agent/turn-stopping` 串行处理取消信号。

### 10.5 多租户与弹性
- 水平扩展：`runtime` 无状态，会话状态外置（§8）。
- 灰度：Profile/Bundle 按租户选择性加载。

---

## 11. 部署与交付

### 11.1 构建
```bash
cargo build --release          # 后端（插件内核 + 各 bundle）
cd frontend && npm run build   # 前端 dist/
python3 verify_axioms.py       # 数学自洽性门禁
```

### 11.2 运行形态
| Profile | 场景 | 端口 |
|---------|------|------|
| `web` | 全功能 + Web UI | 3000 (HTTP) + 3080 (UI, 对标 dsh web) |
| `headless` | CI/批处理 | 无 UI |
| `edge` | 边缘算子节点 | 最小 footprint |

### 11.3 插件热加载
- `plugins/` 目录监听 → `core/wasm` 热加载 → 不重启更新算子生态。
- Bundle 热更新：`operator-server --reload-bundle <name>`。

### 11.4 容器化（建议）
```dockerfile
FROM rust:1.81 AS build
COPY . /src && RUN cargo build --release
FROM debian:bookworm-slim
COPY --from=build /src/target/release/operator-server /usr/local/bin/
COPY plugins/ /opt/ous/plugins/
CMD ["operator-server", "--profile", "web"]
```

### 11.5 四种形态部署矩阵

| 形态 | 启动命令 | 前端 | LLM |
|------|----------|------|-----|
| 云+云 | `operator-server --profile web`（容器/K8s） | CDN/对象存储 | `OUS_LLM_MODE=cloud` |
| 本地+本地 | `./start.sh`（`--profile edge`） | http://localhost:3000 | `OUS_LLM_MODE=local` (Ollama) |
| 云+本地 | 云部署 + 本地 LLM 经隧道 | HTTPS | `OUS_LLM_MODE=hybrid` |
| 桌面 | `cargo tauri build` → 安装包 | 内嵌 WebView | 运行时开关切换 |

### 11.6 桌面打包（Tauri）

```bash
# 1. 构建前端（相对路径，适配 WebView/子路径）
cd frontend && VITE_BASE=./ npm run build && cd ..
# 2. Tauri 内嵌 operator-server 并打包
cd desktop && cargo tauri build   # .msi / .dmg / .AppImage / .deb
# 3. 算子生态 OTA：桌面内 --reload-bundle 热更新
```

> Tauri 与 OUS 同为 Rust 技术栈，二进制内嵌后端无独立端口冲突，体积小、启动快、离线可用。

---

## 12. 迁移路线图（基于现有 crate）

| 阶段 | 目标 | 改造点 |
|------|------|--------|
| P1 | 抽取 `ctx` 内核 | `runtime` 瘦身为插件加载器；新增 `crates/core-context`（OUS-Cordis） |
| P2 | 插件化声明 | 各 crate `Cargo.toml` 加 `[package.metadata.ous]` bundle 声明 |
| P3 | Seam 抽象 | 将 LLM/DB/FS/Telemetry 抽为 Seam 接口，默认实现挂载为插件 |
| P4 | Session Log | `conversation` 升级为追加式 `core/session`，统一 `derive_messages` |
| P5 | Turn 生命周期 | `ai-agent` 的 `agent-loop` 对齐 §5 Waterfall 事件 |
| P6 | 守恒校验闭环 | `conservation` 接入 Turn 结束，失败回滚 |
| P7 | 企业能力 | 多租户 `agent.ctx`、治理、OTel 可观测性 |
| P8 | 沙箱纵深 | `capability/*` 能力令牌 + `benches/sandbox_fuzz` 模糊测试（§21） |
| P9 | 多模态 | `vision/*` `audio/*` `video/*` 算子 Seam + 统一状态向量投影（§22） |
| P10 | 记忆/知识 | RAG 检索管线 + 元记忆加权边（§23） |
| P11 | 评测体系 | 补齐 `benches/` `tests/` + 模型层 Eval 数据集（§24） |
| P12 | 国际化 | `vue-i18n` + 后端 locale 模板 + A11y（§25） |
| P13 | 版本治理 | bundle `api` 版本校验 + 会话 schema 版本化 + 桌面 OTA（§26） |
| P14 | 路径隔离 | `OUS_HOME` 工作路径 + `runtime` 写路径收口 + `start.sh` 改造（§27） |
| P15 | 流程设计模块 | 前端 Three.js 设计器 + DSL 校验矩阵 + 版本化/模板市场（§28） |

---

---

## 13. 全形态产品矩阵（核心产品力）

> **产品愿景**：OUS 是一套"一次编排、处处运行"的算子智能体平台——
> 后端可跑在**云电脑**或**本地电脑**；LLM 可接**云端**或**本地**；前端可通过**浏览器**访问，也可**打包为桌面 App**。

四大维度正交组合，形成统一但灵活的产品矩阵：

```
        运行位置 ────┬── 云电脑 (Cloud VM / K8s / 容器)
                     └── 本地电脑 (笔记本 / 工作站 / 边缘)
                          │
        LLM 来源 ────┬── 云 LLM (DeepSeek / OpenAI / Azure / 通义 / 文心)
                     └── 本地 LLM (Ollama / vLLM / llama.cpp / LM Studio)
                          │
        访问形态 ────┬── 浏览器 (Web UI, SPA + Three.js 3D)
                     └── 桌面 App (Tauri / Electron 打包)
```

### 13.1 形态一：云电脑 + 云 LLM（SaaS / 私有云）

- **运行**：后端 `operator-server --profile web` 部署于云 VM/容器，前端经 CDN/对象存储托管。
- **LLM**：`llm/*` Seam 绑定云端适配（`ctx.llm` 指向 DeepSeek/OpenAI，带 API Key 托管于 `credential/*`）。
- **访问**：浏览器 `https://<domain>`，WebSocket/SSE 实时流。
- **适用**：企业多租户、弹性算力、团队协作。
- **多租户**：`agent.ctx` 作用域隔离 + 租户级 `cordis.patch.yml`。

### 13.2 形态二：本地电脑 + 本地 LLM（隐私 / 离线 / 边缘）

- **运行**：`start.sh` 一键启动（cargo build + verify_axioms + 启服务），`--profile edge` 极小 footprint。
- **LLM**：`llm/*` Seam 绑定本地适配（`ctx.llm` 指向 `http://localhost:11434` Ollama 或 vLLM），**零数据出网**，满足数据合规。
- **访问**：浏览器 `http://localhost:3000`（同机）或局域网内其他设备浏览器。
- **适用**：个人开发者、内网/涉密环境、离线推理、数据不出域。
- **WASM 沙箱**：本地同样运行 `operator-wasm`，第三方算子隔离执行。

### 13.3 形态三：云电脑 + 本地 LLM（混合 / 算力分离）

- **场景**：重编排算力在云，敏感推理在本地——通过网络隧道把本地 LLM 暴露为远端 `llm/*` 端点。
- **实现**：`llm/*` Seam 的"远程本地"适配器，云端 `ctx.llm` 指向经 TLS 隧道回连的本地推理服务。

### 13.4 形态四：本地电脑 + 云 LLM（轻端 + 强模型）

- **场景**：低端本地机只跑前端/轻量算子，`llm/*` 指向云端，平衡成本与体验。

### 13.5 浏览器访问（Web UI）

- 前端 Vue3 + Element Plus + Three.js 3D 力导向图，SPA 经 Vite 构建为 `dist/`。
- `vite.config.js` 的 `/api` 代理在 dev 指向 `:3000`；**生产构建改为相对路径 `base: './'`**，使静态资源可被任意域名/子路径/桌面 WebView 加载。
- 实时通道：WebSocket/SSE 推送 `session/event` + `assistant/chunk`，3D 图实时渲染状态向量与算子 DAG。
- 自适应：移动端/桌面端响应式布局（Element Plus 栅格）。

### 13.6 桌面 App 打包（Tauri 优先）

- **方案选择**：推荐 **Tauri**（Rust 内核，体积小 ~10MB，复用系统 WebView），与 OUS 的 Rust 技术栈天然契合；备选 Electron（体积大但生态成熟）。
- **架构**：Tauri 的 Rust 侧直接内嵌 `operator-server` 作为后台进程（同一二进制，无独立端口冲突），前端 WebView 经 `tauri://localhost` 或 `http://127.0.0.1:<port>` 访问。
- **LLM 自由**：桌面 App 内置"云端/本地"切换开关，绑定不同 `llm/*` Seam 实现，用户可一键在 DeepSeek 与 Ollama 间切换。
- **离线优先**：无网络时自动降级为本地 LLM + 本地算子，联网时无缝切回云端。
- **打包产物**：
  ```bash
  # Tauri 侧（前端构建后）
  cd frontend && npm run build
  cd ../desktop && cargo tauri build   # 产出 .msi / .dmg / .AppImage / .deb
  ```
- **自动更新**：Tauri 内置 updater，配合插件 bundle 热加载实现"算子生态 OTA"。

### 13.7 统一配置开关（Run Profile Matrix）

| 维度 | 配置项 | 取值示例 |
|------|--------|----------|
| 运行位置 | `OUS_RUN_MODE` | `cloud` / `local` |
| LLM 来源 | `OUS_LLM_MODE` | `cloud` / `local` / `hybrid` |
| LLM 端点 | `OUS_LLM_BASE` | `https://api.deepseek.com` / `http://localhost:11434` |
| 访问形态 | `OUS_UI_MODE` | `browser` / `desktop` |
| 前端 Base | `VITE_BASE` | `./`（桌面/子路径） / `/`（根域） |

> 所有开关经环境变量 + `cordis.patch.yml` 双层覆盖，符合 §3.2 加载顺序，无需改代码即可在全形态间切换。

### 13.8 前端构建适配（vite.config.js 增强）

为支持桌面/子路径，生产构建需相对路径（已在 §13.5 提及），配置要点：

```js
export default defineConfig({
  base: process.env.VITE_BASE || '/',   // 桌面/子路径置 './'
  build: { outDir: 'dist', assetsDir: 'assets' },
  // dev 代理不变
})
```

---

## 14. 插件 SDK 与开发契约（Operator SDK）

为让"万物皆算子插件"真正可扩展，定义统一开发契约：

### 14.1 算子插件骨架（Rust）

```rust
// crates/operator-wasm/examples/hello_operator.rs
use ous_core::operator::{Operator, OperatorContext, StateVector};
use ous_core::monad::Effect;

#[derive(Default)]
pub struct HelloOperator;

impl Operator for HelloOperator {
    fn name(&self) -> &str { "hello" }
    // 公理 4：类型契约，编译期校验可组合性
    fn in_type(&self) -> TypePair { TypePair::text() }
    fn out_type(&self) -> TypePair { TypePair::text() }

    fn execute(&self, ctx: &mut OperatorContext, state: &mut StateVector) -> Effect<()> {
        // 副作用经 monad 封装，保证 §1.2 可逆性
        ctx.emit("operator/post-execute", "hello world");
        Effect::pure(())
    }
}
```

- 编译为 WASM → 放入 `plugins/` → `core/wasm` 热加载（§11.3）。
- `TypePair::can_compose` 在注册时校验（公理 4），不兼容组合被拒绝并记录。

### 14.2 插件清单（bundle manifest）

```toml
# plugins/hello-operator/Cargo.toml 片段
[package.metadata.ous]
bundle = "hello-operator"
provides = ["ctx.operators.hello"]
requires = ["ctx.wasm", "ctx.operators"]
seam = "operator/*"
```

### 14.3 前端算子市场契约
- 算子元数据（name/类型/in-out schema/图标）经 `/api/operators` 暴露，前端 Three.js 自动渲染节点。
- 第三方可发布"算子卡片"包（含图标 + 描述 + 示例），前端动态加载，无需重build。

---

## 15. CI/CD 与质量门禁

| 阶段 | 动作 | 门禁 |
|------|------|------|
| 提交前 | `cargo fmt` + `clippy` + `verify_axioms.py` | 非零即阻断 |
| 构建 | `cargo build --release` + `frontend npm run build` | 失败阻断 |
| 测试 | 各 crate 单测 + `monad` 三定律 + 范畴论律 | 覆盖率 < 80% 告警 |
| 集成 | 启动 `--profile web` + 端到端对话/执行流 | Waterfall 事件全绿 |
| 发布 | 打 tag → 构建镜像/桌面安装包 → 推送 `credential/*` 签名 | 签名校验 |
| 灰度 | 按租户 `cordis.patch.yml` 放量 bundle | 监控 `telemetry/*` 异常自动回滚 |

**公理门禁**：`verify_axioms.py` 在 CI 中作为合并必须通过的硬门禁（附录 B），任何破坏数学自洽的提交被拒。

---

## 16. 灾备与数据一致性

- **会话日志 WAL**：所有 `session/event` 追加写，支持从 WAL 重放到任意时间点（对应公理 2）。
- **快照 + 增量**：状态向量定期快照，恢复时快照 + 重放后续事件。
- **多副本**：云形态会话存储主从 + 跨 AZ；本地形态 `$OUS_HOME` 可定时备份到对象存储。
- **混沌工程**：定期注入 WASM 超时/LLM 宕机，验证守恒律回滚与 `agent/turn-stopping` 取消恢复（§10.4）。
- **RTO/RPO**：云形态 RPO<1s（WAL 同步）、RTO<30s；本地形态 RPO=备份周期、RTO=本机重启。

---

## 17. 成本模型（FinOps）

| 成本项 | 云+云 | 本地+本地 | 云+本地 | 桌面 |
|--------|-------|-----------|---------|------|
| 算力 | 按实例计费 | 本机折旧（≈0） | 云编排 + 本地推理 | 本机（≈0） |
| LLM Token | 云端按量 | 本地电费（≈0） | 本地推理（≈0） | 可云可本地 |
| 存储 | 对象/图库 | 本机磁盘 | 混合 | 本机 |
| 网络 | CDN/流量 | 局域网（≈0） | 隧道流量 | 仅云端 LLM 时 |
| 运维 | 平台方 | 自管 | 混合 | 自管 |

> **核心卖点**：本地+本地 / 桌面形态将边际成本压到近零，契合"个人开发者/内网/离线"场景；云形态按租户弹性计费，契合企业。

---

## 18. 开放生态与算子市场（Network Effects）

- **算子市场（执行层）**：开发者发布 WASM 算子 → 签名上架 → 用户一键安装（`plugins/` + `business-catalog`）。
- **需求/流程图资产市场（资产层，已落地）**：将"需求描述 + 可编辑业务流程图 + 功能点清单"作为算子包（OperatorPackage）沉淀，供他人随机浏览、克隆后继续编辑。详见 `docs/market-module.md`。该资产层与执行层互补——商城产出的结构化流程图可经 §28 DSL 转为可执行 `BusinessWorkflow`，形成"设计↔市场"飞轮。
- **跨形态同步**：登录账号后，本地桌面与云端可同步"我的编排/工作流"（端到端加密，密钥在 `credential/*`）。
- **可移植性**：同一份工作流定义（DAG + 算子绑定）在四种形态（§13）间无缝迁移——"一次编排、处处运行"的终极体现。
- **贡献回流**：用户自研优质算子可反哺市场，形成算子网络效应，使 OUS 成为"算子界的插件商店 + 智能体 OS"。

---

---

## 19. 专家联盟全维处理内核（Expert Alliance — 最高权限全维模式）

> **设计定位**：这是 OUS 的"超级大脑"层——当用户以 `SUPER_EXPERT` 模式发起请求时，系统调度**专家联盟 + 璇玑**，以**最高权限**跨全部子系统（算子内核 / 图谱 / 优化 / 编排 / 数据 / 外系统）进行全维处理，并受守恒律与治理接缝约束。对标 harness 的 `self-modification/`（agent 可改自身运行时）但更强：联盟不仅能改运行时，还能改算子、改图谱、改调度策略。

### 19.1 两层联盟结构

```
                        ┌─────────────────────────────────┐
                        │   SUPER_EXPERT 调度中枢 (最高权限)  │
                        │   ctx.alliance.super            │
                        └───────────┬───────────┬─────────┘
                                    │           │
                  ┌─────────────────▼──┐   ┌─────▼──────────────────┐
                  │  专家联盟 ExpertPool │   │  璇玑 AlgoPool      │
                  │  ctx.experts        │   │  ctx.algo              │
                  ├─ 架构专家            │   ├─ 优化算法 (DAG/关键路径)│
                  ├─ 领域专家(业务)     │   ├─ 图算法 (PageRank/社群) │
                  ├─ 安全/合规专家      │   ├─ 数值/符号计算          │
                  ├─ 检索(IR)专家       │   ├─ 机器学习/推理          │
                  └────────┬───────────┘   └────┬───────────────────┘
                           │                    │
              ┌────────────▼────────────────────▼───────────┐
              │   全维执行总线 (All-Domain Bus)               │
              │   算子内核 · 图谱 · 优化 · 编排 · 数据 · 外系统 │
              └─────────────────────────────────────────────┘
```

### 19.2 最高权限的边界与约束（不失控）

"最高权限"不等于"无约束"。参考 harness `guard/`（循环卫生、工具超时）与 OUS 守恒律，定义受控的最高权限：

| 权限 | 范围 | 约束（熔断/审计） |
|------|------|-------------------|
| 改算子 | 注册/卸载/热加载 WASM 算子 | `approval/*` 审批 + `session/event` 全量审计 |
| 改图谱 | 增删节点/边、重算中心性 | 守恒律校验 `delta(L1/L2/Sum)` 超阈回滚 |
| 改调度 | 调整 DAG/资源池/模型路由 | `optimizer::schedule` 重算效率，下降则拒绝 |
| 改提示 | 改写 `systemPrompt` 模板 | 经 `govern` 策略校验，禁止注入越权指令 |
| 调外系统 | 经 Seam 调用 DB/LLM/FS | 租户配额 + `guard/` 超时 + 限流 |
| 自我进化 | 挂载新插件（对标 harness self-modification） | 沙箱试跑 + 公理门禁通过才生效 |

> 最高权限 = "可跨域调度一切资源"，但每一次变更都经**事件总线广播 + 守恒校验 + 审计留痕**，确保可逆、可归因、可回滚。

### 19.3 全维处理工作流（SUPER_EXPERT Turn）

```
1. 收口：归一化用户意图 → 状态向量投影（公理 2）
2. 分诊：专家联盟并行评估 → 领取各自子目标（并发组）
3. 冲突消解：flow-ai::conflict::detect + auto_repair（§6.3）
4. 算法编排：璇玑产出 DAG 调度计划（optimizer）
5. 全维执行：All-Domain Bus 并行调用各子系统算子
6. 收敛校验：conservation::check_all（状态向量守恒）
7. 治理复核：govern 策略 + 合规专家签字（可并行/可拒）
8. 沉淀：结果写回会话日志 + 图谱加权边（强化后续推理）
9. 自进化（可选）：若发现更优算子组合 → 提案上架算子市场
```

### 19.4 与普通模式的差异

| 模式 | 调度范围 | 权限 | 适用 |
|------|----------|------|------|
| `SINGLE` | 单个算子/单 Agent | 受限 | 常规问答、单任务 |
| `FLOW` | 编排层 DAG | 编排内 | 工作流、批处理 |
| `SUPER_EXPERT` | 全子系统 + 自我进化 | 最高(受控) | 复杂跨域难题、系统级优化、自动研发 |

### 19.5 璇玑与数学内核的协同

璇玑直接消费 OUS 6 大公理作为"先验约束"：
- 优化算法在 `ResourcePool` 约束下搜索，目标函数含守恒残差惩罚项 → 保证解不破坏状态向量守恒。
- 图算法把专家结论作为新加权边注入 `operator-graph`，使后续 PageRank/社群发现自然"吸收"专家知识（知识复利）。
- 数值/符号计算经 `monad` 封装，保证可重放（会话日志溯源）。

---

## 20. 融合对标与产品定位（OUS vs deepseek-harness vs Claude Code）

### 20.1 三者关系

```
Claude Code ──(hook 桥接)──▶ deepseek-harness ──(范式吸收)──▶ OUS
  单体 coding                 插件运行时内核        算子智能体 OS
  agent 产品                   (一切皆插件)          (数学内核+全形态+联盟)
```

- **吸收 harness 的**：无特权核心、Seam 可替换、会话日志溯源、自我修改元能力。
- **吸收 Claude Code 的**：开箱即用的 coding 体验（通过挂载 `shell/`、`fs/`、`lsp/` 等价 Seam 实现，OUS 已有 `ai-agent::browser_automation` 与 WASM 执行基础）。
- **OUS 独有超越**：范畴论/希尔伯特数学内核、守恒律回滚、专家联盟最高权限全维模式、四形态全运行（云/本地 × 云/本地 LLM × 浏览器/桌面）、算子市场网络效应。

### 20.2 为什么 OUS 是"最最好的 AI 产品"

| 维度 | OUS 优势 |
|------|----------|
| 数学严谨 | 6 公理 + 守恒律，结果可证明收敛、可回滚 |
| 全形态 | 一次编排、处处运行（云/本地、云/本地 LLM、浏览器/桌面） |
| 可进化 | 插件化 + 算子市场 + 自我修改，越用越强 |
| 最高权限处理 | 专家联盟+璇玑跨域全维求解，受控不失控 |
| 隐私/成本 | 本地+本地近零成本、零数据出网 |
| 企业级 | STRIDE 安全、多租户、灾备、FinOps、CI 公理门禁 |

### 20.3 模块化与业务流程化落地原则

- **模块化**：每个能力 = 一个 Seam 提供者的插件（llm/wasm/fs/shell/graph/optimizer/experts…），独立开发、独立测试、独立热加载。
- **业务流程化**：12 条端到端业务流（§9）+ SUPER_EXPERT 工作流（§19.3）全部以 DAG 形式在 `flow-ai` 中定义、可视化（Three.js）、可复用、可组合。
- **组合即产品**：用户/开发者把"算子卡片"拖入画布连成流程 → 一键发布为插件/工作流/桌面功能，形成"搭积木式"构建 AI 应用。

---

---

## 21. 沙箱安全纵深（Sandbox Defense-in-Depth）

基于 §10.3 的 WASM 沙箱，构建四层纵深防御，确保"最高权限全维处理（§19）"也不突破安全边界：

| 层 | 机制 | 实现 |
|----|------|------|
| L1 隔离 | WASM 线性内存 + 无原生 syscall | `operator-wasm` (Wasmer/WASMEdge) |
| L2 能力令牌 | 算子声明所需能力，运行时签发短时令牌 | `capability/*` Seam：`fs:read:/tmp`、`net:out:deepseek` |
| L3 资源熔断 | CPU/内存/网络/时长硬配额 + 超时杀死 | `resource::ResourcePool` + `guard/` 超时 |
| L4 行为审计 | 所有 Seam 调用经 `session/event` 留痕 | `telemetry/*` + 不可变 WAL（§16） |

- **最小权限默认**：算子默认零能力，须显式声明并经 `approval/*` 审批（对标 harness `permission/`）。
- **逃逸防护**：禁用 `dyn` 间接跳转滥用、限制 `table.gy`、校验 WASI 导入白名单；定期模糊测试 `benches/sandbox_fuzz`。
- **跨租户**：能力令牌绑定租户作用域（`agent.ctx`），禁止越界访问其他租户 `$OUS_HOME`。

## 22. 多模态与感知（Multimodal & Perception）

把"万物皆算子"扩展到多模态，使 OUS 能处理文本/图像/音频/视频/结构化数据：

| 模态 | 算子 Seam | 落点 |
|------|-----------|------|
| 文本 | `llm/*` + `operator/*` | 现有核心 |
| 图像 | `vision/*` | 多模态 LLM / 视觉模型，输出结构化描述算子 |
| 音频 | `audio/*` | ASR（语音→文本算子）+ TTS（文本→语音算子） |
| 视频 | `video/*` | 抽帧 + 时序理解 + 关键帧摘要算子 |
| 结构化 | `data/*` | CSV/JSON/DB 表 → 状态向量投影算子 |
| 3D/图 | `graph/*` | Three.js 可视化 + 几何算子 |

- **统一表示**：所有模态经 `state::StateVector` 投影为统一高维向量（公理 2），使多模态可被同一套范畴论组合律（公理 4）编排。
- **感知→动作闭环**：浏览器自动化（§6.3）+ 视觉算子构成"看-想-做"智能体闭环，SUPER_EXPERT 模式（§19）可跨模态调度。

## 23. 记忆与知识管理（Memory & Knowledge）

| 记忆类型 | 存储 | 对应模块 | 生命周期 |
|----------|------|----------|----------|
| 短期(工作) | 会话日志投影 | `core/session`（§5.4） | Turn/Session |
| 长期(语义) | 向量库 + 图 | `operator-core::state` + `operator-graph` | 持久 |
| 情节( episodic) | WAL 事件流 | `session/event`（§16） | 可重放 |
| 程序性(技能) | 算子/工作流 | `business-catalog` + `flow-ai` | 版本化 |
| 元记忆(自省) | 专家结论边 | `operator-graph` 加权边（§19.5） | 知识复利 |

- **检索增强（RAG）**：`expert-alliance::ir` 管线从长期记忆检索相关状态向量/图谱子图，注入 `systemPrompt`（对标 harness `context-engineering/`）。
- **遗忘与演进**：低频记忆经 PageRank 衰减降权；专家联盟结论持续强化为加权边 → 系统越用越聪明。

## 24. 评测与回归（Eval & Regression）

基于现有 `verify_axioms.py`（附录 B）扩展为三层评测体系，`benches/` 与 `tests/` 目录补齐：

| 层 | 内容 | 门禁 |
|----|------|------|
| 公理层 | `verify_axioms.py`：6 公理 + 守恒律 + 单子三律 | 合并硬阻断（§15） |
| 单元层 | 各 crate `#[test]` + `benches/` 性能基 | 覆盖率 ≥ 80% |
| 行为层 | `tests/` 端到端：对话/执行/SUPER_EXPERT 流 | Waterfall 全绿 |
| 模型层 | 算子质量 Eval 数据集（准确率/延迟/守恒残差） | 回归超阈值告警 |

- **基准方法论**：`benches/` 用 Criterion 跑算子执行 P99、状态向量重建、PageRank 吞吐，CI 出趋势图防性能退化。
- **混沌 Eval**：注入 LLM 宕机/WASM 超时，验证守恒回滚与 `agent/turn-stopping` 取消恢复（§16）。

## 25. 国际化与无障碍（i18n & A11y）

- **i18n**：前端 `vue-i18n` + 后端 `systemPrompt` 模板按 `Accept-Language`/租户 locale 渲染；算子元数据多语。
- **无障碍**：Three.js 图提供键盘导航 + 高对比主题 + 屏幕阅读器 ARIA；对话流支持字幕（TTS 算子 §22）。
- **低带宽**：桌面/边缘形态（§13）支持离线包与增量同步，弱网可用。
- **多端一致**：浏览器（§13.5）与桌面（§13.6）共享同一 Vue3 构建，UI 自适应响应式。

## 26. 版本与兼容治理（Versioning & Compatibility）

| 项 | 策略 |
|----|------|
| 内核版本 | 语义化 `MAJOR.MINOR.PATCH`，破坏性变更走 `MAJOR` |
| 插件契约 | `bundle manifest` 声明 `api = "1.x"`，加载时校验兼容 |
| 会话格式 | `SessionEvent` schema 版本化，旧日志可重放（§5.4） |
| 算子兼容 | `TypePair` 向后兼容（公理 4），不兼容组合拒绝并记录 |
| 升级 | 灰度放量（§15）+ `cordis.patch.yml` 租户级回滚 |
| 桌面 OTA | Tauri updater + 插件 bundle 热加载（§13.6） |

- **兼容契约**：任何内核 API 变更须提供 deprecation 期与迁移脚本；`verify_axioms.py` 含契约测试防隐性破坏。

---

---

## 27. 路径与运行态隔离规范（Code Path vs Work Path）

> **核心原则**：**架构代码路径（源码/构建/产物）与工作路径（运行态数据）必须物理分离，绝不可混放。** 这是工程可维护性、可部署性、多实例隔离与灾备（§16）的基石。当前 `runtime` 存在违反此原则的代码（见 §27.4），须按本规范改造。

### 27.1 双路径模型

| 路径 | 名称 | 内容 | 生命周期 | 是否入版本库 |
|------|------|------|----------|--------------|
| **CODE_PATH** | 架构代码路径 | crate 源码、Cargo.toml、构建脚本、部署清单 | 随版本发布 | 是（git） |
| **WORK_PATH** | 工作路径 | 插件、会话日志、知识图谱快照、工作流实例、LLM 配置、运行时日志 | 随运行产生 | 否（.gitignore） |

- CODE_PATH 由构建产物（`target/release/operator-server`、前端 `dist/`）代表，但**产物也不应回写源码树**（见 §27.4）。
- WORK_PATH 由环境变量 `OUS_HOME` 指定，默认 `~/.ous`（云形态 `/var/lib/ous`，桌面形态 `<用户数据目录>/ous`）。

### 27.2 标准目录树

```
CODE_PATH (仓库根 /usr/local/ous 等)            WORK_PATH ($OUS_HOME)
├── crates/                源码                      ├── plugins/          热加载 WASM 算子
├── frontend/             前端源码                   ├── sessions/         会话日志 WAL(§5.4)
├── Cargo.toml            构建定义                   ├── graph/            知识图谱快照(§23)
├── build.rs              构建脚本                   ├── workflows/        工作流实例(§9 P-04)
├── docs/                 架构文档                   ├── llm/              LLM 配置(加密, §10.3)
├── start.sh              启动脚本                   ├── logs/             运行时日志(§10.1)
└── target/release/       构建产物*                  ├── catalog/          业务算子目录持久化(§6.5)
                                            (*可置于 CODE_PATH 或独立 ARTIFACT_PATH)   └── config/          cordis.patch.yml(§3.2)

注意：CODE_PATH 与 WORK_PATH 不得存在父子/重叠关系；CI 中 WORK_PATH 必须为独立挂载卷。
```

### 27.3 配置约定

```bash
# 启动时必须显式分离；缺失 OUS_HOME 时拒绝以源码目录为工作路径
export OUS_HOME=/var/lib/ous          # 工作路径(独立卷)
export OUS_CODE=/usr/local/ous        # 代码/产物路径(只读挂载)
operator-server \
  --code $OUS_CODE \
  --home $OUS_HOME \
  --plugins $OUS_HOME/plugins \
  --profile web
```

- `runtime` 解析路径规则：**所有写操作（插件落盘、会话、日志、配置）只允许落在 `$OUS_HOME` 子树内**；写向 CODE_PATH 视为安全违规（§21 L4 审计拦截）。
- 前端 `dist/` 由构建独立产出，运行时经 `ServeDir` 指向 `$OUS_CODE/frontend/dist` 或专门的 `ARTIFACT_PATH`，**不在仓库内生成**。

### 27.4 当前代码违规点与改造（对齐真实 main.rs）

| 行 | 现状 | 问题 | 改造 |
|----|------|------|------|
| 255 | `WasmPluginManager::new("./plugins")` | 插件写进源码目录 `./plugins` | 改 `$OUS_HOME/plugins`，由 `--plugins` 注入 |
| 381 | `ServeDir::new("./frontend/dist")` | 前端产物混源码树，且需先 build | 指向 `$OUS_CODE/frontend/dist` 或 `ARTIFACT_PATH` |
| 43–55 | `chat_sessions`/`saved_workflows`/`execution_logs` 全内存 | 重启丢失、无法多实例、无持久工作路径 | 持久化到 `$OUS_HOME/sessions` `$OUS_HOME/workflows` `$OUS_HOME/logs`（§8 数据层） |
| — | `start.sh` 未设 `OUS_HOME` | 默认落到 CWD（可能即源码树） | `start.sh` 显式 `export OUS_HOME=${OUS_HOME:-~/.ous}` 并 `mkdir -p` |
| 430（market 模块，`crates/runtime/src/market.rs`） | `MARKET_DIR = "./data/market"` 写进源码目录 | 商城资产落 CODE_PATH，违反双路径原则（§27.1） | 改为 `$OUS_HOME/market`，由 `OUS_HOME` 注入；种子数据仅在目录为空时生成 |

> 改造后，`git status` 不再出现运行态文件（插件/日志/会话），仓库纯净；同一份 CODE_PATH 可同时服务多个 WORK_PATH 实例（多租户/多环境隔离）。

### 27.5 与四形态（§13）及灾备（§16）的关系

- **本地+本地（§13.2）**：`OUS_HOME=~/.ous`，纯本机，备份即打包该目录。
- **云形态（§13.1）**：`OUS_HOME=/var/lib/ous` 为独立 PVC，与镜像（CODE_PATH）解耦，便于滚动更新不丢数据。
- **桌面（§13.6）**：Tauri 的 `app_data_dir` 作 `OUS_HOME`，与安装目录（CODE_PATH）严格分离。
- **灾备（§16）**：只备份 `WORK_PATH`；CODE_PATH 由镜像/安装包重建，无需备份。

### 27.6 静态检查（CI 门禁）

- 新增 lint 规则：源码树内禁止硬写 `./plugins`、`./frontend/dist`、`./data` 等相对运行路径（参考 §15 门禁）。
- `verify_axioms.py` 之外增 `verify_paths.py`：启动后用 `OUS_HOME` 指向临时目录，断言无任何写操作触及 CODE_PATH。

---

---

## 28. 业务流程设计模块（Business Process Design Module）

> **定位**：让"业务流程"成为一等公民的设计/建模/校验/版本化/市场化的工程模块。用户在 Three.js 画布上拖拽节点连成 DAG，系统自动校验、生成可执行 `FlowDefinition`/`BusinessWorkflow`，并可发布为模板/插件（§18）。本模块把 §9 的"业务处理流程卡"从静态文档升级为**用户可构建、可复用、可组合的资产**。
> 现有底座：`ai-agent::flow_engine`（`FlowNode`/`NodeType`/`FlowDefinition`/环检测/执行）、`ai-agent::types`（归一化 BPMN 节点、`AlgorithmFlow` 复杂度分析）、`ai-agent::workflow_engine`（`BusinessWorkflow` 实例状态机）。

### 28.1 模块分层

```
┌──────────────────────────────────────────────────────────────┐
│  设计层 (Design Plane)                                         │
│  Three.js 画布 · 节点面板 · 连线 · 属性表单 · 实时校验提示       │
├──────────────────────────────────────────────────────────────┤
│  模型层 (Model Plane)                                          │
│  FlowDefinition / BusinessWorkflow / AlgorithmFlow (types.rs)   │
│  NodeType 体系 + 流程 DSL (§28.3)                               │
├──────────────────────────────────────────────────────────────┤
│  校验层 (Validate Plane)                                        │
│  拓扑校验(环/孤儿) · 类型契约(公理4) · 资源约束 · 治理策略(§10)   │
├──────────────────────────────────────────────────────────────┤
│  执行层 (Execute Plane)   —— 复用 §9 流程卡                    │
│  FlowEngine.execute_flow / WorkflowEngine.run / SUPER_EXPERT    │
├──────────────────────────────────────────────────────────────┤
│  资产层 (Asset Plane)                                          │
│  版本化存储($OUS_HOME/workflows) · 模板市场(§18) · 算子市场      │
└──────────────────────────────────────────────────────────────┘
```

### 28.2 节点类型体系（Node Type System）

复用 `flow_engine::NodeType` 并扩展为统一"业务节点分类"：

| 类别 | 节点 | 语义 | 对应流程卡(§9) |
|------|------|------|----------------|
| 控制 | Start / End / Parallel / Merge | 流程边界与并发 | — |
| AI | LLM | 模型调用(`llm/*` Seam) | P-03 |
| 感知 | Browser | 浏览器自动化(`browser_automation`) | P-06 |
| 集成 | HttpRequest / DataInput / DataOutput | 外系统/数据 Seam | P-11 |
| **算子** | Operator | WASM 算子执行(`operator-wasm`) | P-02 |
| 逻辑 | Condition / Decision | 条件分支(边 `condition`) | P-08 冲突 |
| 变换 | Transform / Script | 数据转换/自定义脚本 | P-09 |
| 业务 | Workflow | 子流程(`BusinessWorkflow` 复用) | P-04 |
| 算法 | Algorithm | `AlgorithmFlow`(复杂度分析+优化建议) | P-09/P-10 |
| 专家 | Expert | `expert-alliance` 节点(§19) | P-13 |

> 每个节点声明 `in/out TypePair`（公理 4），连边时实时校验类型可组合；不兼容连线在画布红框提示（§28.4）。

### 28.3 流程 DSL（Process DSL）

流程以声明式 JSON 描述（即 `FlowDefinition`），并支持两种表达方式互转：

```json
{
  "id": "flow_demo_01",
  "name": "客服工单自动处理",
  "nodes": [
    {"id":"s","node_type":"Start"},
    {"id":"n1","node_type":"DataInput","config":{"source":"ticket_api"}},
    {"id":"n2","node_type":"LLM","config":{"prompt":"分类工单","model":"auto"}},
    {"id":"n3","node_type":"Condition","condition":"severity=='high'"},
    {"id":"n4","node_type":"Operator","config":{"operator_id":"escalate"}},
    {"id":"n5","node_type":"Browser","config":{"task":"通知值班"}},
    {"id":"e","node_type":"End"}
  ],
  "edges": [
    {"source":"s","target":"n1"},
    {"source":"n1","target":"n2"},
    {"source":"n2","target":"n3"},
    {"source":"n3","target":"n4","condition":"true"},
    {"source":"n3","target":"e","condition":"false"},
    {"source":"n4","target":"n5"},
    {"source":"n5","target":"e"}
  ],
  "variables": {"severity": "low"}
}
```

- DSL 经 `FlowEngine::validate_flow` 校验（环检测 `CycleDetected`、孤儿节点、类型契约）。
- 与 §9 流程卡双向映射：设计器导出的 DSL ≡ 流程卡的可机读形态；`save_workflow` 存为 `BusinessWorkflow`（§9 P-04）。

### 28.4 校验矩阵（Validate Plane）

| 校验 | 规则 | 失败处理 |
|------|------|----------|
| 拓扑 | 无环、单 Start/End、无孤儿节点 | `FlowError::CycleDetected`/`InvalidConfig` |
| 类型契约(公理4) | 边 source.out 与 target.in `TypePair::can_compose` | 画布红框 + 拒绝保存 |
| 资源约束 | 节点资源需求 ⊆ `ResourcePool` | `optimizer` 提示降级/拆分 |
| 治理策略(§10) | 高危算子需 `approval/*` 审批节点 | 保存时挂起待审批 |
| 语义 | 变量引用存在、条件可求值 | `ConditionError` 提示 |

### 28.5 版本化与发布

- 每次保存生成语义化版本 `flow_demo_01@1.2.3`（§26），`SessionEvent` schema 兼容可重放（§5.4）。
- 发布为**模板**：存入资产层 `$OUS_HOME/workflows`，可上架算子市场（§18）供他人一键安装。
- 子流程复用：`Workflow` 节点引用已发布模板，形成"流程的组合递归"（§9.3 跨流程编排）。

### 28.6 与业务处理流程卡（§9）的闭环

```
用户画布设计 ──DSL──▶ 校验层 ──▶ 存为 BusinessWorkflow(§9 P-04)
                               │
                               ▼
                  执行层(FlowEngine.execute_flow) ──▶ 结果/日志($OUS_HOME/logs)
                               │
                               ▼
                  §9 流程卡(触发/阶段/输出/SLA/异常) 反向标注到画布节点
                               │
                               ▼
                  优化层(AlgorithmFlow 复杂度+优化建议) 提示重构 ──▶ 回到设计层
```

即：**设计即文档、执行即监控、优化即重构建议**，形成业务流程的"设计-执行-优化"飞轮。

### 28.7 SUPER_EXPERT 自动流程生成（§19 联动）

- 用户以自然语言描述目标 → `SUPER_EXPERT` 调度专家联盟(§19) → 生成候选 `FlowDefinition` DSL。
- 经 §28.4 校验矩阵自动校验，不通过则璇玑(§19)自修复（对标 `flow-ai::conflict::auto_repair`）。
- 用户确认后一键发布为模板（§28.5）→ 沉淀算子市场（§18）→ 知识复利（§23）。

### 28.8 与路径隔离（§27）的关系

- 设计产物（DSL/模板/实例）全部落 `$OUS_HOME/workflows`，**不污染 CODE_PATH**；多租户各自 WORK_PATH 隔离（§27.5）。

### 28.9 实证：用 OUS 搭建企业门户网站（验证是否好用）

为验证"业务流程设计模块 + 全形态 + 现有能力"是否真能快速产出可用产品，已基于本系统落地一个**企业门户网站**（代码见 `frontend/`，运行说明见 `frontend/PORTAL_README.md`）：

- **改动量小**：新增 4 个 Vue 页面（`PortalHome`/`Login`/`Workbench`/`BusinessHall`）+ 1 个 `router`，**后端零改动**，全部复用 §9 既有端点（`/api/ai/chat`、`/api/operators`、`/api/ai/flows`）。
- **门户即编排产物**：门户首页的 AI 客服浮窗直接调 `/api/ai/chat`；业务大厅拉取已注册算子并一键执行流程——即"用业务流程设计模块（§28）产出的能力，直接对外提供服务"。
- **全形态就绪**：登录壳可选 运行形态(云/本地) + LLM 来源(云/本地)，与 §13 产品矩阵一致。
- **验证结论**：OUS 的"模块化 + 业务流程化"确实好用——半天即可由现有算子/流程能力拼出带导航、登录、AI 助手、业务大厅的企业门户；证明 §9/§28 的流程卡与 DSL 不是文档空谈，而是可服务化的真实资产。

> 局限：编写环境的 Node 运行时出现稳定性异常，未能在本会话完成自动 `vite build`；代码为标准 Vue3 + vue-router4，在本地稳定 Node 环境 `npm install && npm run dev` 即可运行（详见 `frontend/PORTAL_README.md`）。

---

## 附录 A：关键命令对照

| DeepSeek Harness | OUS 对应 |
|------------------|----------|
| `npx @deepseek-ai/dsh web` | `operator-server --profile web` |
| `dsh --profile web --dump-config` | `operator-server --profile web --dump-config` |
| 插件 Topic `dsh-plugin` | `plugins/*.wasm` + `business-catalog` |
| `session/event` 日志 | `core/session` + `StateVector` 投影 |

## 附录 B：公理验证门禁

```bash
python3 verify_axioms.py
```
覆盖：算子组合、希尔伯特向量、图算法、范畴论律、资源约束、单子三定律、守恒律。CI 中作为合并门禁。

---

*本文档融合 DeepSeek Harness 的"一切皆插件"范式与 Claude Code 的开箱 coding 体验，叠加 OUS 自有的范畴论/希尔伯特数学内核、守恒律回滚、专家联盟最高权限全维处理模式，以及沙箱纵深安全、多模态感知、记忆知识、Eval 评测、i18n/无障碍、版本治理、**代码路径与工作路径严格隔离**、**业务流程设计模块（可视化设计-校验-执行-优化飞轮）** 等企业级能力，将 OUS 设计为"可组合、可审计、可热插拔、全形态运行、可自我进化"的企业级算子智能体 OS——一次编排、处处运行（云/本地、云/本地 LLM、浏览器/桌面），以专家联盟+璇玑实现最高权限全维求解，以算子市场形成开放生态网络效应。*
