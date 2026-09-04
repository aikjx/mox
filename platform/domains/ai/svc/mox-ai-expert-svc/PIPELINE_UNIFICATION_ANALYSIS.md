# 两套管线架构对比分析与统一方案

> 分析对象：`mox-ai-expert-svc` 中并存的两套管线架构
> - **mox 模块化系统架构处理流水线** (`src/pipeline.rs`) — 同步、FlowGraph 驱动、面向出码治理
> - **联盟 6 阶段管线** (`src/alliance/gate.rs`) — 异步 SSE、自然语言驱动、面向咨询质量

---

## 一、两套管线职责边界对比

### 1.1 mox 模块化系统架构处理流水线（pipeline.rs）

| 维度 | 说明 |
|------|------|
| **输入** | `FlowGraph` + `GovernContext`（租户/主体/配额/策略） |
| **输出** | `GovernanceReport`（专家评分 + 优化报告 + 璇玑验证 + 治理闸门 + 审计链） |
| **核心目标** | 对流程图做多维专家诊断 + 算法验证 + 治理闸门，决定能否出码 |
| **执行模式** | 同步函数，一次性返回完整报告 |
| **阶段数** | 8 个硬编码步骤 |
| **专家模型** | 14 位专家（业务 7 维 + 开发 7 维），并行分析 |
| **闸门语义** | 通过/否决（布尔型），以算法否决为最高权限 |
| **适用场景** | 流程优化、代码生成前的治理门禁、企业合规检查 |

**阶段顺序：**
```
构建Harness → 维度着色(normalize) → 并行专家派发 → 裁决(reconcile)
→ flow-ai 优化求解 → 璇玑验证(verify) → 治理闸门(govern) → 审计收尾
```

### 1.2 联盟 6 阶段管线（alliance/gate.rs）

| 维度 | 说明 |
|------|------|
| **输入** | `AllianceRequest`（自然语言 query + 选项） |
| **输出** | `Vec<AllianceEvent>`（SSE 事件流）+ `Vec<AuditEvent>`（7 个审计事件） |
| **核心目标** | 对自然语言查询做意图分类 → 专家组队 → 辩论 → 合成 → 质量门禁 → 指标学习 |
| **执行模式** | 异步 `async fn`，逐阶段发射 SSE 事件 |
| **阶段数** | 6 阶段 + Done = 7 个 SSE 事件 |
| **专家模型** | 动态组队（3~7 人），基于意图分类选专家 |
| **闸门语义** | 四级评分（A/B/C/D），HC-8 加权公式，C 级可重试 |
| **适用场景** | mox 模块化系统架构分析咨询、ChatView 对话流、专家联盟服务 |

**阶段顺序：**
```
Intent → Team → Debate → Synthesize → Gate → Learn → Done
```

---

## 二、重叠区域与重复点

### 2.1 数据结构重复（最严重）

#### 2.1.1 GateResult 两套实现

| 特性 | `govern::GateResult` | `alliance::gate::GateResult` |
|------|---------------------|------------------------------|
| 字段 | status, approved, sla_ok, budget_ok, blocking_risks, algorithm_veto, reason, gates | score (GateScore), retried, suggestions, diagnose_id |
| 评分方式 | 布尔型 approved + 多维度原因 | 四级制 A/B/C/D + HC-8 公式 |
| 否决机制 | 算法否决（最高权限） | D 级阻断 |
| 重试机制 | 无 | C 级单次重试 |
| 审计关联 | AuditChain（哈希链） | AuditEvent（7 类事件） |

**问题：** 两者语义都是"质量门禁结果"，但结构、评分体系、否决机制完全不同。没有统一的抽象。

#### 2.1.2 AuditEvent 三套实现

| 特性 | `govern::AuditEvent` | `alliance::gate::AuditEvent` | `audit::event::AuditEvent` (Ext) |
|------|---------------------|------------------------------|----------------------------------|
| 定位 | 内部哈希链 | 管线阶段审计 | 外部合规标准（SOC2/GDPR） |
| 结构 | id, ts, subject, flow_id, action, decision, prev_hash, hash | event, trace_id, ts_ms, payload | event_id, timestamp, actor, action, resource, outcome, severity, chain_hash, content_hash, signature, tenant_id |
| 哈希算法 | DefaultHasher (FNV 风格) | 无哈希 | SHA-256 + HMAC 签名 |
| 链式结构 | AuditChain（Vec + prev_hash） | 扁平数组，trace_id 关联 | chain_hash 字段 |

**问题：** 三套审计事件各自为政，没有统一的审计事件总线。内部链、外部合规、管线阶段审计三者之间没有桥接。

#### 2.1.3 阶段/管线概念无统一抽象

- `pipeline.rs`：8 步硬编码流程，无阶段枚举，无 Pipeline trait
- `alliance/mod.rs`：`AlliancePhase` 枚举（7 阶段），但只服务于联盟管线
- `harness.rs`：`WaterfallEvent` 枚举（4 个扩展点），只服务于插件运行时

**问题：** 没有统一的 `Phase` 概念和 `Pipeline` trait，两套管线各自演化。

### 2.2 执行模式差异

| 特性 | mox 模块化系统架构流水线 (pipeline) | 联盟管线 (alliance) |
|------|----------------------|---------------------|
| 同步/异步 | 同步 (`fn`) | 异步 (`async fn`) |
| 返回方式 | 一次性返回完整结构体 | SSE 事件流（Vec\<Event\>） |
| 错误处理 | 隐式（返回结构体中含结果） | `Result<T, AllianceError>` |
| 进度可见性 | 无（黑盒） | 每阶段一个事件，前端可展示进度 |
| 可中断性 | 不可中断 | D 级闸门可提前终止（阻断） |

### 2.3 治理闸门重复实现

| 特性 | govern::govern() | alliance::evaluate_gate() |
|------|------------------|---------------------------|
| 输入 | ReconciledPlan + OptimizationReport + FlowStatus + 配额 | IntentResult + TeamResult + DebateResult |
| 输出 | GateResult (approved 布尔) | GateScore (四级制 + 总分) |
| 评分维度 | 算法否决 + 状态机 + 阻断冲突 + SLA + 预算 | Quality + Speed + TokenEfficiency + Stability |
| 权重公式 | 硬编码布尔与逻辑 | HC-8: 0.55Q + 0.20S + 0.10T + 0.15St |
| 重试机制 | 无 | C 级单次重试 |
| 升级路径 | 算法否决 > 治理闸门 | 无升级（D 级直接阻断） |

**问题：** 两套闸门面向不同场景，但本质都是"评估质量 → 判定是否放行"。核心模式（输入 → 评分 → 分级 → 决策）一致，但实现完全独立。

### 2.4 插件机制与管线编排的关系

- `harness.rs` 提供了强大的插件化运行时（Plugin trait + Waterfall 扩展点 + 事件总线 + 可逆副作用）
- 但 `harness` 只被 `pipeline.rs` 使用，且仅用于专家派发和闸门前后钩子
- `alliance` 管线完全没有使用 harness，它自己硬编码了 6 阶段顺序
- harness 的 `WaterfallEvent` 只有 4 个（PreAnalyze/PostAnalyze/PreGate/PostGate），无法覆盖 alliance 的 7 个阶段

---

## 三、统一方案设计

### 3.1 设计原则

1. **单一管线核心**：定义统一的 `Pipeline` trait 和 `Phase` 枚举，两套管线共享同一核心骨架
2. **阶段可插拔**：每个阶段是一个 `PhaseHandler`，可注册、可替换、可热插拔
3. **上下文贯穿**：统一的 `PipelineContext` 在各阶段间流动，携带输入、中间结果、审计链
4. **钩子机制**：复用 harness 的瀑布扩展点思想，但扩展为通用的 `pre_phase` / `post_phase` 钩子
5. **审计统一**：统一审计事件模型，内部链 + 外部合规双写由核心自动完成
6. **同步/异步统一**：核心支持两种执行模式，具体实现按需选择
7. **向后兼容**：两套现有管线可逐步迁移，不破坏现有 API

### 3.2 核心抽象层次

```
┌─────────────────────────────────────────────────────┐
│  Pipeline trait (统一核心)                           │
│  ┌───────────────────────────────────────────────┐  │
│  │  PipelineContext (贯穿上下文)                  │  │
│  │  - trace_id / tenant / principal / quota      │  │
│  │  - phase_results: Map<Phase, PhaseResult>     │  │
│  │  - audit_chain: UnifiedAuditChain             │  │
│  │  - bag: HashMap<String, Box<dyn Any>>         │  │
│  └───────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────┐  │
│  │  Phase enum (统一阶段定义)                     │  │
│  │  - Normalize / Analyze / Reconcile / Optimize │  │
│  │  - Verify / Gate / Learn / Done               │  │
│  │  + 自定义扩展阶段 (Custom(&str))              │  │
│  └───────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────┐  │
│  │  PhaseHandler trait (阶段处理器)               │  │
│  │  - phase() -> Phase                           │  │
│  │  - execute(&mut PipelineContext) -> PhaseResult │
│  └───────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────┐  │
│  │  Hook机制 (pre_phase / post_phase)            │  │
│  │  - 复用 harness waterfall 责任链模式           │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
          ▲                    ▲
          │                    │
┌─────────┴──────┐    ┌────────┴─────────┐
│ mox 模块化系统架构优化管线    │    │ 联盟分析管线      │
│ (MoxPipeline)  │    │ (AlliancePipeline)│
│  复用 6 个     │    │  复用 5 个        │
│  通用阶段      │    │  通用阶段         │
│  + 2 个特有    │    │  + 3 个特有       │
└────────────────┘    └──────────────────┘
```

### 3.3 统一 Phase 枚举设计

```rust
pub enum Phase {
    // ---- 通用基础阶段 ----
    Normalize,    // 归一化/预处理（维度着色 / 意图分类）
    Analyze,      // 分析（专家并行 / 辩论咨询）
    Reconcile,    // 裁决/合成（多观点归一）
    Gate,         // 质量门禁（评分 + 分级 + 决策）
    Learn,        // 指标学习（可选）
    Done,         // 完成（收尾 + 审计）

    // ---- mox 模块化系统架构管线特有 ----
    Optimize,     // flow-ai 优化求解
    Verify,       // 璇玑算法验证

    // ---- 联盟管线特有 ----
    Team,         // 专家组队
    Synthesize,   // 合成输出

    // ---- 扩展 ----
    Custom(&'static str),
}
```

### 3.4 统一 PipelineContext 设计

- 持有：trace_id、租户、主体、配额、策略
- 阶段结果映射：`HashMap<Phase, Box<dyn PhaseResult>>`
- 审计链：统一审计（内部链 + 外部 sink 双写）
- 扩展 bag：`HashMap<String, Box<dyn Any + Send + Sync>>`
- 瀑布钩子状态：可被钩子读写

### 3.5 统一 PhaseResult 设计

```rust
pub trait PhaseResult: Send + Sync + std::fmt::Debug {
    fn phase(&self) -> Phase;
    fn success(&self) -> bool;
    fn payload(&self) -> serde_json::Value;
    fn as_any(&self) -> &dyn Any;
}
```

每个具体阶段的结果类型实现此 trait，同时保留其具体类型供下游使用。

### 3.6 统一审计事件机制

将三套审计事件收敛为统一模型：

- **核心事件结构**：基于 `audit::event::ExtAuditEvent`（最完整，符合合规标准）
- **内部哈希链**：保留 `govern::AuditChain` 的防篡改验证能力
- **阶段事件自动发射**：Pipeline 核心在每个 `pre_phase` / `post_phase` 自动生成审计事件
- **多 sink 支持**：Syslog / S3 / 内部链 / 自定义 sink，由 `audit` 模块统一管理

### 3.7 两套管线如何基于统一核心实现

#### 3.7.1 mox 模块化系统架构优化管线（MoxPipeline）

```
Normalize    → Analyze     → Reconcile  → Optimize   → Verify     → Gate       → Learn(可选) → Done
(auto_dim.)    (run_experts)  (reconcile)  (optimize)   (verify)     (govern)     (指标学习)
复用通用阶段    复用通用阶段    复用通用阶段   特有阶段     特有阶段      复用通用阶段   复用通用阶段
```

- **输入**：`FlowGraph + GovernContext`
- **输出**：`GovernanceReport`
- **执行模式**：同步
- **复用的通用阶段**：Normalize, Analyze, Reconcile, Gate, Learn, Done (6个)
- **特有阶段**：Optimize, Verify (2个)

#### 3.7.2 联盟分析管线（AlliancePipeline）

```
Normalize  → Team    → Analyze    → Synthesize → Gate     → Learn    → Done
(classify)  (组队)    (debate)     (合成)       (HC-8)     (学习)
通用阶段    特有阶段   通用阶段     特有阶段     通用阶段   通用阶段   通用阶段
```

- **输入**：`AllianceRequest`
- **输出**：`Vec<AllianceEvent>` (SSE)
- **执行模式**：异步 SSE
- **复用的通用阶段**：Normalize, Analyze, Gate, Learn, Done (5个)
- **特有阶段**：Team, Synthesize (2个)

---

## 四、迁移路径

### 阶段一：建立统一核心（低风险）

1. 新建 `src/pipeline_core/` 模块，包含：
   - `mod.rs` - 模块入口
   - `phase.rs` - Phase 枚举 + PhaseHandler trait
   - `context.rs` - PipelineContext
   - `result.rs` - PhaseResult trait
   - `pipeline.rs` - Pipeline trait + 通用执行器
   - `hooks.rs` - 钩子机制（从 harness 迁移并泛化）
   - `audit.rs` - 统一审计桥接

2. 统一审计事件模型：
   - 以 `audit::event::ExtAuditEvent` 为基础
   - 将 `govern::AuditChain` 的哈希链能力整合进去
   - 两套管线逐步迁移到统一审计

### 阶段二：迁移mox 模块化系统架构流水线（中等风险）

1. 为mox 模块化系统架构管线的每个步骤实现 `PhaseHandler`：
   - `NormalizeHandler` → 包装 `auto_dimension`
   - `AnalyzeHandler` → 包装 `run_experts`（复用 harness）
   - `ReconcileHandler` → 包装 `reconcile`
   - `OptimizeHandler` → 包装 `flow-ai optimize`
   - `VerifyHandler` → 包装 `verify`
   - `GateHandler` → 包装 `govern`
   - `LearnHandler` → 可选（目前mox 模块化系统架构管线无 Learn，留扩展位）

2. 用统一管线重写 `mox_optimize`，保持对外 API 不变

3. 所有现有测试应通过（行为等价）

### 阶段三：迁移联盟管线（中等风险）

1. 为联盟管线的每个阶段实现 `PhaseHandler`：
   - `IntentHandler` → 包装 `classify_intent`（映射到 Normalize）
   - `TeamHandler` → 包装 `optimize_team`（特有阶段）
   - `DebateHandler` → 包装 `consult_and_debate`（映射到 Analyze）
   - `SynthesizeHandler` → 特有阶段
   - `GateHandler` → 包装 `evaluate_gate`（复用通用 Gate 抽象）
   - `LearnHandler` → 包装 `learn_metrics`（复用通用 Learn 抽象）

2. 用统一管线重写 `run_full_pipeline`，保持 SSE 事件格式不变

3. 所有现有测试应通过

### 阶段四：深度整合（高价值）

1. **闸门统一**：将 `govern::govern()` 和 `alliance::evaluate_gate()` 统一为 `GateEngine`
   - 支持多种评分模型（布尔型 / 四级制 / 自定义）
   - 统一的重试机制
   - 统一的否决升级路径

2. **专家引擎统一**：mox 模块化系统架构 14 专家和联盟动态组队共享同一专家注册中心
   - harness 插件机制扩展到联盟管线
   - 专家可同时服务于两套管线

3. **审计完全统一**：消除三套 AuditEvent，只剩一套统一审计模型

### 风险评估

| 阶段 | 风险 | 缓解措施 |
|------|------|----------|
| 阶段一 | 低 | 新模块不影响现有代码 |
| 阶段二 | 中 | 保持 `mox_optimize` 签名不变，内部替换实现 |
| 阶段三 | 中 | 保持 SSE 事件格式不变，内部替换实现 |
| 阶段四 | 中高 | 闸门统一需要仔细对齐语义差异 |

---

## 五、建议放置位置

```
src/
├── pipeline_core/           ← 新增：统一管线核心
│   ├── mod.rs               ← 模块入口，导出核心类型
│   ├── phase.rs             ← Phase 枚举 + PhaseHandler trait
│   ├── context.rs           ← PipelineContext（贯穿上下文）
│   ├── result.rs            ← PhaseResult trait
│   ├── pipeline.rs          ← Pipeline trait + 同步/异步执行器
│   ├── hooks.rs             ← 钩子机制（pre_phase / post_phase）
│   └── audit.rs             ← 统一审计桥接
│
├── pipeline.rs              ← 保留：mox 模块化系统架构优化管线（迁移到核心）
├── alliance/                ← 保留：联盟管线（迁移到核心）
│   └── gate.rs              ← 保留：逐步迁移 PhaseHandler
├── harness.rs               ← 保留：插件化运行时（钩子机制迁移到核心后简化）
├── govern.rs                ← 保留：治理层（闸门逻辑迁移到 GateEngine）
└── audit/                   ← 保留：外部审计（与核心审计桥接对齐）
```

---

## 六、收益总结

1. **消除重复**：GateResult、AuditEvent、阶段编排等从 2~3 套收敛为 1 套
2. **能力复用**：新管线只需实现特有的 PhaseHandler，通用阶段开箱即用
3. **审计一致**：所有管线共享同一审计基础设施，合规证据链完整
4. **扩展灵活**：新增阶段 = 新增 PhaseHandler，无需改核心
5. **测试友好**：每个 PhaseHandler 可独立测试，管线集成测试可复用
6. **监控统一**：所有管线的阶段耗时、成功率等指标格式一致
7. **演进平滑**：两套管线可独立迁移，不破坏现有 API 和测试
