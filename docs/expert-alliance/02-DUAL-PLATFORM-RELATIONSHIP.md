# 专家联盟双平台架构关系说明

> **标题**：专家联盟双平台架构关系说明
> **版本**：V1.0
> **权威等级**：🟢权威
> **编号**：EA-DOC-002
> **文档层级**：L2架构设计层
> **最后更新日期**：2026-08-31
> **主责联盟**：开发联盟 R
> **单源声明**：本文档是专家联盟Node.js层与Rust alliance域双平台并存关系的唯一权威承载。冲突时以 `docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md` 为准。
> **编制依据**：`docs/working-reports/expert-alliance-doc-inventory-20260831.md` §5裁决组C3、`docs/modules/business-process-flowcharts.md` 第九章、`docs/modules/专家联盟V2.0-集成对齐分析报告.md`、`docs/working-reports/expert-alliance-code-alignment-20260831.md`

---

## 1. 概述

专家联盟（Expert Alliance）当前存在**两套并行实现**：Node.js 平台层与 Rust alliance 域。两者并非替代关系，而是历史演进过程中形成的并存架构。本文档明确两层的边界、功能映射、通信方式与长期迁移策略，消除"哪层是权威"的困惑。

### 1.1 两层定位

| 维度 | Node.js 平台层 | Rust alliance 域 |
|------|---------------|-----------------|
| 代码路径 | `platform/backend-node/` | `platform/domains/alliance/` |
| 技术栈 | Node.js (Express) | Rust (Axum + Tokio) |
| 监听端口 | `:3010` | `:3100`（scheduler-svc）/ `:3200`（executor-svc） |
| 业务域数量 | 23 个业务域 | 11 个 crate（proto×3 / core×4 / svc×2 / sdk×1 / api×1） |
| 专家联盟实现 | 较早实现（`expert-alliance.js` / `expert-alliance-engine.js`） | 当前活跃开发（scheduler-core / executor-core） |
| 定位 | 全业务统一入口，含AI引擎、知识图谱、专家联盟等23域 | 专家联盟专项模块化重构，聚焦调度与执行 |
| 成熟度 | 生产运行，功能完整 | 2026-08-31修复后11 crate全部编译通过，持续迭代中 |

### 1.2 并存原因

1. **历史演进**：Node.js 层是专家联盟的最初实现，承载了完整的六阶段流程（EAF标准）与前端对话能力。
2. **架构升级**：Rust alliance 域是后续模块化重构的产物，采用 DDD 分层（proto/core/svc/sdk/api），目标是更高性能与可维护性。
3. **渐进迁移**：两层功能尚未完全对齐，Rust 层缺少部分 Node 层已实现的能力（如学习闭环、降级链显式实现、MCP协议端点），因此需要并存过渡。

---

## 2. 功能映射表

### 2.1 专家联盟核心功能映射

| 功能 | Node.js 层实现 | Rust alliance 域实现 | 对齐状态 |
|------|---------------|---------------------|---------|
| 意图识别 | `expert-alliance-engine.js` `classifyIntent()` | scheduler-core `Planner` / `LLMRouter` | ⚠️ 部分对齐（Rust侧无独立意图分类阶段） |
| 专家匹配/组队 | `expert-alliance-engine.js` `composeTeam()`（能力×协同−负载多目标评分） | `ModularWeightMatcher`（能力标签0.4 + 领域标签0.2 + 指标0.3 + 成本-0.1）+ `RuleBasedExpertMatcher` | ✅ 功能对齐，算法不同 |
| 并行咨询与辩论 | `expert-alliance-engine.js` `deliberate()`（多轮辩论 + 共识率≥0.6自适应跳过） | `DebateFusion` 策略（多轮辩论收敛） | ⚠️ 部分对齐（Rust侧融合策略含辩论，但参数需确认） |
| 综合合成 | `expert-alliance-engine.js` `synthesize()`（加权置信度） | 6种融合策略（weighted_voting / confidence_weighting / debate / stacking / map_reduce / iterative_refinement） | ✅ Rust侧能力更丰富 |
| 质量门禁 | `expert-alliance-engine.js` `qualityGate()`（C级单次重试闭环） | 无独立门禁阶段（融合后直接输出） | ❌ Rust侧未实现 |
| 反馈学习 | `expert-alliance-engine.js` `learn()`（意图先验学习 + `alliance_learned_skills.json` 容量200条） | 无学习闭环实现 | ❌ Rust侧未实现 |
| 降级链 | 降级链#1显式实现（并行咨询→单专家直答；LLM综合→启发式综合） | 无显式降级链机制 | ❌ Rust侧未实现 |
| SSE流式输出 | `POST /ai/engine/alliance/full`（SSE，7帧事件） | `POST /api/v1/collaboration/stream`（scheduler-svc）+ `POST /api/v1/execute/stream`（executor-svc） | ⚠️ 端点路径不同，功能对齐 |
| 专家CRUD | `/experts` 系列路由 | `GET /api/v1/experts` / `GET /api/v1/experts/match`（scheduler-svc） | ⚠️ 端点路径不同 |
| 多租户 | 部分实现（Node层） | Task/Expert/CollaborationPlan有tenant_id字段，svc层提取`X-Tenant-Id`头，数据隔离未实现 | ⚠️ 字段预留，隔离未实现 |

### 2.2 非专家联盟功能（仅Node层）

以下功能仅在 Node.js 层实现，Rust alliance 域不涉及：

| 功能域 | Node.js 实现 | 说明 |
|--------|-------------|------|
| AI 引擎统一编排 | `ai-engine.js`（四端点：capabilities/metrics/process/analyze） | 全业务AI能力统一入口 |
| 知识图谱 | `graph/` 相关模块 | 图谱构建、查询、关图治理 |
| LLM 网关 | `llm-gateway.js`（多模型路由、加密、限流） | 多AI引擎接入 |
| 会话存储 | `session-store.js`（会话CRUD、向量搜索） | 对话状态管理 |
| 业务流程引擎 | `flow-ai` / `workflow` 相关 | 企业级BP流程 |
| mox-expert融合优化 | 通过网关路由 `/api/mox/optimize` | 璇玑融合8步管线 |

---

## 3. 两层通信方式

### 3.1 当前状态

**当前为独立部署，无直接调用。**

- Node.js 层（:3010）和 Rust alliance 域（:3100/:3200）各自独立启动、独立监听。
- 前端通过网关（`platform/gateway/runtime/`）路由到不同后端：
  - `/ai/engine/*`、`/experts/*`、`/ai/chat` → Node.js 层（:3010）
  - `/api/v1/alliance/*`、`/api/v1/tasks`、`/api/v1/collaboration/*` → Rust scheduler-svc（:3100）
  - `/api/v1/execute/*` → Rust executor-svc（:3200）
  - `/api/mox/*` → mox-expert 融合引擎（通过网关路由）
- 两层之间**无内部 API 调用、无共享数据库、无消息队列通信**。

### 3.2 网关路由层

`platform/gateway/runtime/` 作为统一入口，根据路径前缀将请求分发到不同后端。这是当前两层并存的核心协调机制。

```
客户端请求
    │
    ▼
┌─────────────────────┐
│  Gateway (:3000?)   │  路径前缀路由
└────────┬────────────┘
         │
    ┌────┴────┬────────────┬─────────────┐
    ▼         ▼            ▼             ▼
 Node层    Rust调度     Rust执行      mox-expert
 (:3010)   (:3100)      (:3200)       (融合)
 /ai/engine /api/v1/    /api/v1/      /api/mox/
 /experts   tasks        execute       optimize
 /ai/chat   collaboration
```

> ⚠️ **注意**：网关端口与路由规则以 `platform/gateway/runtime/` 实际配置为准，上图为逻辑示意。

---

## 4. API 端点对照

### 4.1 专家联盟相关端点

| 功能 | Node.js 层端点 | Rust alliance 域端点 | 说明 |
|------|--------------|---------------------|------|
| 提交任务 | — | `POST /api/v1/tasks`（scheduler-svc） | Rust侧新增 |
| 查询任务 | — | `GET /api/v1/tasks/:task_id` | Rust侧新增 |
| 专家匹配 | `/experts/match`（Node层） | `GET /api/v1/experts/match`（scheduler-svc） | 路径不同 |
| 专家列表 | `/experts`（Node层） | `GET /api/v1/experts`（scheduler-svc） | 路径不同 |
| 协作执行（同步） | — | `POST /api/v1/collaboration/execute` | Rust侧新增 |
| 协作执行（SSE流） | `POST /ai/engine/alliance/full` | `POST /api/v1/collaboration/stream` | 路径不同 |
| DAG执行（同步） | — | `POST /api/v1/execute`（executor-svc） | Rust侧新增 |
| DAG执行（SSE流） | — | `POST /api/v1/execute/stream` | Rust侧新增 |
| EAF标准入口 | `POST /ai/engine/alliance/full`(SSE) | 无此端点 | EAF标准基于Node实现 |
| 能力查询 | `GET /ai/engine/alliance/capabilities` | `GET /api/v1/config/snapshot` | 功能类似，路径不同 |
| 健康检查 | — | `GET /health`（两个svc均有） | Rust侧 |

### 4.2 端点并存原则

1. **不强制统一**：两套端点并存，前端根据功能需求选择调用层。
2. **网关路由**：通过 `platform/gateway/runtime/` 统一入口路由，前端无需感知后端差异。
3. **迁移时更新**：长期迁移过程中，Node层端点逐步标记为`deprecated`，流量切至Rust层。

---

## 5. 长期迁移策略建议

### 5.1 迁移原则

1. **功能对齐先行**：Rust 层需补齐 Node 层已实现但 Rust 侧缺失的能力（质量门禁、反馈学习、降级链、MCP协议）后，方可迁移对应流量。
2. **灰度切流**：通过网关路由权重调整，按功能域逐步将流量从 Node 层切至 Rust 层，避免一次性切换风险。
3. **双跑验证**：迁移期间关键功能双跑（Node + Rust 同时处理），比对结果一致性，确认 Rust 层输出正确后再切流。
4. **保留兼容层**：Node 层端点在迁移后保留一段时间（标记 `deprecated`），确保前端平滑过渡。

### 5.2 迁移阶段建议

| 阶段 | 时间建议 | 核心动作 | 验收标准 |
|------|---------|---------|---------|
| **阶段一：能力补齐** | M1-M2 | Rust层补齐质量门禁、反馈学习、降级链三大缺失能力 | 6种融合策略全测试通过；门禁/学习/降级有对应实现与测试 |
| **阶段二：端点对齐** | M2-M3 | Rust层新增与Node层兼容的API端点（或网关做路径转换） | 前端无需修改即可通过网关调用Rust层 |
| **阶段三：灰度切流** | M3-M4 | 按功能域逐步切流：先专家匹配/列表（低风险），再协作执行（中风险），最后EAF全链路（高风险） | 每个功能域双跑验证≥1周，结果一致率≥99% |
| **阶段四：Node层瘦身** | M4-M5 | Node层专家联盟相关代码标记`deprecated`，移除或归档；保留AI引擎/知识图谱等非专家联盟功能 | Node层专家联盟代码量减少≥80%；专家联盟流量100%走Rust层 |
| **阶段五：收敛完成** | M5+ | 完全移除Node层专家联盟实现；更新所有文档引用；统一术语 | 仓库中无Node层专家联盟代码残留；文档全部引用Rust alliance域 |

### 5.3 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| Rust层功能缺失导致切流后能力降级 | 用户体验下降 | 严格按阶段一→三顺序执行，未补齐不切流 |
| 双跑期间结果不一致 | 数据混乱 | 建立结果比对机制，不一致时以Node层为准并记录Rust层缺陷 |
| 网关路由配置错误 | 流量路由到错误后端 | 网关配置变更需经过测试环境验证 + 三联盟评审 |
| 前端硬编码Node层端点 | 切流后前端报错 | 提前排查前端代码，所有端点通过网关统一入口调用，禁止硬编码后端地址 |

---

## 6. 参考文档

| 文档 | 路径 | 引用内容 |
|------|------|---------|
| 盘点报告 §5 裁决组C3 | `docs/working-reports/expert-alliance-doc-inventory-20260831.md` | 双平台并存裁决 |
| 代码对齐报告 | `docs/working-reports/expert-alliance-code-alignment-20260831.md` | Rust alliance域11 crate代码事实 |
| 集成对齐分析报告 | `docs/modules/专家联盟V2.0-集成对齐分析报告.md` | Node层专家联盟模块映射 |
| 业务流程图第九章 | `docs/modules/business-process-flowcharts.md` | Node平台层23业务域总览 |
| 归一化规范 | `docs/standards/expert-alliance-normalization-mode.md` | 文档-代码对齐要求 |
| 架构修复报告 | `docs/alliance-architecture-fix-report-20260831.html` | Rust层修复后状态 |

---

**变更记录**

| 版本 | 日期 | 变更内容 | 签字 |
|------|------|---------|------|
| V1.0 | 2026-08-31 | 首发：双平台架构关系说明，含两层定位、功能映射表、通信方式、API对照、长期迁移策略 | 开发联盟 R |

---

**版权所有**：© 2026 璇玑 RelGraph · 算子统一系统（OUS）· 三联盟
**文档版本**：V1.0 ｜ **发布日期**：2026-08-31
