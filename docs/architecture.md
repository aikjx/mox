# 算子统一系统（OUS）企业级架构设计文档 v4.0

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
| 9. 全维度业务流程 | 12 条端到端业务流 + 性能容量模型 |
| 10. 企业级能力 | 可观测性、治理、安全(STRIDE)、多租户 |
| 11. 部署与交付 | 容器化、灰度、插件热加载、桌面打包 |
| 12. 迁移路线图 | 基于现有 crate 的重构步骤 |
| 13. 全形态产品矩阵 | 云/本地运行 · 云/本地 LLM · 浏览器 · 桌面打包 |
| 14. 插件 SDK | 算子开发契约 + bundle 清单 + 前端市场契约 |
| 15. CI/CD | 质量门禁 + 公理校验 + 灰度回滚 |
| 16. 灾备 | WAL 重放 + 快照 + 混沌工程 + RTO/RPO |
| 17. 成本模型 | 四形态 FinOps 对比 |
| 18. 开放生态 | 算子市场 + 跨形态同步 + 网络效应 |
| 19. 专家联盟全维处理内核 | 最高权限 · 算法联盟 · 全维业务编排 |
| 20. 融合对标与产品定位 | 与 harness/Claude Code 的差异与优势 |
| 21. 沙箱安全纵深 | WASM 沙箱 + 能力令牌 + 纵深防御 |
| 22. 多模态与感知 | 文本/图/音/视频/结构化统一算子 |
| 23. 记忆与知识管理 | 短期/长期/程序性记忆 + 知识复利 |
| 24. 评测与回归 (Eval) | 公理门禁 + 行为回归 + 基准 |
| 25. i18n 与无障碍 | 多语言 + 无障碍 + 低带宽 |
| 26. 版本与兼容治理 | 语义化版本 + 兼容契约 + 升级 |

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

## 9. 全维度业务流程（12 条端到端）

1. **算子注册流**：开发者提交 WASM/本地算子 → `business-catalog` 注册 → `operator/*` 事件广播 → `core/operator` 类型校验（公理 4）→ 可用。
2. **算子执行流**：`/api/execute` → 鉴权 → 构建 DAG → `optimizer` 调度 → `operator-wasm` 沙箱执行 → `conservation` 校验 → 结果经 SSE 推回。
3. **AI 对话流**：`/api/ai/chat` → `conversation` 取会话 → `systemPrompt` 组装 → `agent-loop` Turn → `llm/stream` → `session/event` 追加 → 流式响应。
4. **工作流编排流**：`/api/ai/flows` 建图 → `flow_engine::validate_flow` → `agent/turn-stopping` 触发 `execute_flow` → DAG 执行。
5. **浏览器自动化流**：`/api/ai/browser/natural` → `parse_natural_language` → `browser_automation` 建会话/任务 → 执行 → 结果回写会话。
6. **知识图谱构建流**：业务事件 → `operator-graph` 加节点/边 → `centrality`/`communities`/`pagerank` 计算 → 前端 3D 力导向渲染。
7. **冲突消解毒**：并发算子组 → `flow-ai::conflict::detect` → `auto_repair` → 重调度。
8. **资源优化流**：DAG + `ResourcePool` → `optimizer` 关键路径 → `efficiency` 指标 → 报告。
9. **专家协同流**：多专家结论 → `expert-alliance` 聚合 → `conflict` 消解 → `conservation` 收敛校验。
10. **外部流对接流**：`hermes-flow-bridge` 录制外部事件 → 映射为算子 → 入统一图 → 回放。
11. **插件热加载流**：`plugins/*.wasm` 放入目录 → 监听 → `core/wasm` 热加载 → `operator/*` 注册 → 不重启生效。
12. **审计回放流**：任意会话 → `ctx.sessions.fork()` + `derive_messages` → 重放历史状态向量 → 合规审计。
13. **SUPER_EXPERT 全维处理流**（§19）：用户以最高权限模式发起 → 专家联盟分诊 + 算法联盟产出 DAG → All-Domain Bus 跨子系统并行执行 → 守恒收敛校验 → 治理复核 → 沉淀图谱 + 自进化提案。

### 9.1 性能与容量模型

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

- **算子市场**：开发者发布 WASM 算子 → 签名上架 → 用户一键安装（`plugins/` + `business-catalog`）。
- **跨形态同步**：登录账号后，本地桌面与云端可同步"我的编排/工作流"（端到端加密，密钥在 `credential/*`）。
- **可移植性**：同一份工作流定义（DAG + 算子绑定）在四种形态（§13）间无缝迁移——"一次编排、处处运行"的终极体现。
- **贡献回流**：用户自研优质算子可反哺市场，形成算子网络效应，使 OUS 成为"算子界的插件商店 + 智能体 OS"。

---

---

## 19. 专家联盟全维处理内核（Expert Alliance — 最高权限全维模式）

> **设计定位**：这是 OUS 的"超级大脑"层——当用户以 `SUPER_EXPERT` 模式发起请求时，系统调度**专家联盟 + 算法联盟**，以**最高权限**跨全部子系统（算子内核 / 图谱 / 优化 / 编排 / 数据 / 外系统）进行全维处理，并受守恒律与治理接缝约束。对标 harness 的 `self-modification/`（agent 可改自身运行时）但更强：联盟不仅能改运行时，还能改算子、改图谱、改调度策略。

### 19.1 两层联盟结构

```
                        ┌─────────────────────────────────┐
                        │   SUPER_EXPERT 调度中枢 (最高权限)  │
                        │   ctx.alliance.super            │
                        └───────────┬───────────┬─────────┘
                                    │           │
                  ┌─────────────────▼──┐   ┌─────▼──────────────────┐
                  │  专家联盟 ExpertPool │   │  算法联盟 AlgoPool      │
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
4. 算法编排：算法联盟产出 DAG 调度计划（optimizer）
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

### 19.5 算法联盟与数学内核的协同

算法联盟直接消费 OUS 6 大公理作为"先验约束"：
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
| 最高权限处理 | 专家联盟+算法联盟跨域全维求解，受控不失控 |
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

*本文档融合 DeepSeek Harness 的"一切皆插件"范式与 Claude Code 的开箱 coding 体验，叠加 OUS 自有的范畴论/希尔伯特数学内核、守恒律回滚与专家联盟最高权限全维处理模式，将 OUS 设计为"可组合、可审计、可热插拔、全形态运行、可自我进化"的企业级算子智能体 OS——一次编排、处处运行（云/本地、云/本地 LLM、浏览器/桌面），以专家联盟+算法联盟实现最高权限全维求解，以算子市场形成开放生态网络效应。*
