# 璇玑专家服务（mox-ai-expert-svc）模块化重构设计文档

> **版本**: v1.0 (P2 架构解耦)
> **日期**: 2026-08-31
> **状态**: 设计评审稿
> **适用范围**: `platform/domains/ai/svc/mox-ai-expert-svc`

---

## 目录

1. [现状分析](#1-现状分析)
2. [God Module 问题诊断](#2-god-module-问题诊断)
3. [架构设计方案](#3-架构设计方案)
4. [独立 crate 拆分清单](#4-独立-crate-拆分清单)
5. [共享类型抽取方案](#5-共享类型抽取方案)
6. [统一错误类型方案](#6-统一错误类型方案)
7. [统一事件/审计协议方案](#7-统一事件审计协议方案)
8. [模块依赖关系图](#8-模块依赖关系图)
9. [迁移路线图](#9-迁移路线图)
10. [风险与缓解措施](#10-风险与缓解措施)

---

## 1. 现状分析

### 1.1 expert-svc 当前模块与职责清单

`mox-ai-expert-svc` 当前包含 **20+ 个模块**，横跨领域模型、核心引擎、服务编排、基础设施等多个层次：

| 模块 | 职责分类 | 核心类型/函数 | 应独立程度 |
|------|---------|--------------|-----------|
| `ir` | 领域模型 | `Dimension`, `CodeIR`, `DimensionedFlow` | 高（领域核心） |
| `expert` | 领域模型 | `Expert`, `ExpertOpinion`, `Constraint`, `Risk`, `Suggestion` | 高（领域核心） |
| `experts/` | 领域实现 | 14 个具体专家（security/permission/algorithm 等） | 高（可插件化） |
| `context` | 领域模型 | `Tenant`, `Principal`, `ResourceQuota`, `GovernContext`, `Capability` | 高（跨域共享） |
| `types` | API DTO | `ExpertMeta`, `ConsultQuery`, `ConsultReport`, `TaskSpec`, `RoutingDecision` + 20+ HTTP DTO | 高（协议层） |
| `expert_traits` | 领域抽象 | `ExpertRegistry`, `ExpertConsultant`, `AllianceOrchestrator` | 高（DIP 层） |
| `domain` | 领域抽象 | `GovernContext` trait, `GovernExpert` trait, `GovernVerdict` | 高（DIP 层） |
| `services` | 服务实现 | `RegistryImpl`, `ExpertServiceImpl`, `AllianceRouter`, `AllianceService` | 中（门面层） |
| `pipeline` | 核心引擎 | `mox_optimize`, `GovernanceReport` | 高（引擎核心） |
| `reconcile` | 核心引擎 | `reconcile`, `ReconciledPlan`, `ReconcileConflict` | 高（引擎核心） |
| `verify` | 核心引擎 | `verify`, `AlgoVerification`, `Check` | 高（引擎核心） |
| `govern` | 核心引擎 | `govern`, `apply_rules`, `AuditChain`, `AuditEvent`, `FlowStatus`, `GateResult` | 高（引擎核心） |
| `alliance/` | 业务编排 | `AllianceEngine`, `AlliancePhase`, `AllianceEvent`, `AllianceError` + intent/team/debate/gate/orchestration/algorithm/kg_connector | 高（独立子系统） |
| `pipeline_core/` | 基础设施 | `Pipeline`, `Phase`, `PhaseHandler`, `PipelineContext`, `WaterfallHook`, `UnifiedAuditEvent` | 高（通用框架） |
| `harness` | 基础设施 | `Plugin`, `PluginMeta`, `HarnessCtx`, `ExpertPlugin`, `WaterfallEvent` | 高（插件框架） |
| `audit/` | 横切能力 | `ExtAuditEvent`, `AuditSink`, `SyslogSink`, `S3Sink`, `MultiSink`, `AuditContext` | 高（平台级能力） |
| `rbac/` | 横切能力 | `RbacPolicy`, `Permission`, `check`, `RbacError` | 高（平台级能力） |
| `tenant_policy` | 横切能力 | `GateId`, 租户策略分层, 治理 8 闸门 | 高（治理能力） |
| `sensitivity` | 横切能力 | 敏感度判定 SSOT | 中（治理能力） |
| `flow_loader` | 适配层 | `FlowLoader`, `YamlFlowLoader`, `FlowDef`, `FlowLoadError` | 中（IO 适配） |
| `server` | 适配层 | HTTP 服务, Axum 路由 | 中（部署绑定） |
| `executor` | 基础设施 | 执行器 | 中 |
| `programming` | 领域扩展 | 编程相关 | 低 |
| `bench` | 测试工具 | Benchmark 框架 | 低（开发工具） |

### 1.2 与参考架构对比

#### mox-ai-core（良好实践）

```
mox-ai-core (7 modules)
├── providers       # AI Provider trait + 内置实现
├── registry        # Provider 注册表
├── router          # 模型路由器
├── chat            # 对话会话管理
├── graph           # 图谱抽象
├── reasoning       # 推理能力
└── prelude         # 统一预导入
```

**特点**: 职责清晰，单一核心（Provider 抽象 + Registry + Router），模块数 ≤ 7。

#### mox-flow-unified-arch-core（良好实践）

```
mox-flow-unified-arch-core (7 modules)
├── error           # 统一错误
├── types           # 统一类型
├── protocol        # 协议抽象
├── connector       # 连接器框架
├── adapter         # 适配器模式
├── unified_api     # 统一 API 网关
└── integration     # 集成管理
```

**特点**: 严格分层（error → types → protocol → connector/adapter → unified_api/integration），模块数 = 7。

#### mox-ai-expert-svc（现状问题）

```
mox-ai-expert-svc (20+ modules, 3+ submodule trees)
├── ir, expert, context, types       # 领域模型（散落 4 处）
├── expert_traits, domain            # DIP 抽象（散落 2 处）
├── pipeline, reconcile, verify, govern  # 核心引擎（散落 4 处）
├── alliance/{intent,team,debate,gate,algorithm,orchestration,kg_connector}  # 业务编排（7 子模块）
├── pipeline_core/{audit,context,hooks,phase,pipeline,result}  # 基础设施（6 子模块）
├── audit, rbac, tenant_policy, sensitivity  # 横切能力（4 模块）
├── harness, flow_loader, server, executor, programming, bench  # 其他（6 模块）
└── services                          # 上帝服务（1200+ 行 AllianService）
```

**问题**: 模块数是参考架构的 **3~4 倍**，职责横跨 L5 领域层到 L2 网关层，违反单一职责原则。

---

## 2. God Module 问题诊断

### 2.1 问题一：职责过载

`mox-ai-expert-svc` 一个 crate 承担了至少 **6 种截然不同的职责**：

1. **领域模型定义**（Dimension, Expert, Tenant, Principal...）
2. **核心引擎实现**（14 专家并行分析、裁决、验证、治理闸门）
3. **业务编排逻辑**（联盟 6 阶段管线：Intent→Team→Debate→Synthesize→Gate→Learn）
4. **横切基础设施**（审计、RBAC、插件框架、管线框架）
5. **API 协议层**（20+ HTTP DTO、3 个对外 trait）
6. **服务适配层**（HTTP server、YAML 加载器、Benchmark 工具）

### 2.2 问题二：通用类型散落

至少 **4 套审计事件模型**、**7 种错误类型**、**3 处领域抽象**散落在不同模块中：

#### 审计事件类型散落在 4 处：

| 位置 | 类型名 | 用途 | 哈希算法 | 字段差异 |
|------|--------|------|---------|---------|
| `govern.rs` | `AuditEvent` | 内部哈希链 | `DefaultHasher` (64bit) | id, ts, subject, flow_id, action, decision, prev_hash, hash |
| `audit/event.rs` | `ExtAuditEvent` | 外部合规（SOC2/GDPR） | SHA-256 | event_id, actor, action, resource, outcome, severity, chain_hash, content_hash, signature, tenant_id |
| `alliance/gate.rs` | `AuditEvent` | 管线阶段审计 | 未知 | 未知（需进一步确认） |
| `pipeline_core/audit.rs` | `UnifiedAuditEvent` | 统一审计（第 4 套） | SHA-256 | event_id, actor, action, resource_type, resource_id, outcome, severity, prev_hash, content_hash, tenant_id, trace_id, phase |

#### 错误类型散落在 7 处：

| 位置 | 类型名 | 实现方式 | 是否集成 mox-error |
|------|--------|---------|-------------------|
| `audit/error.rs` | `AuditError` | 手动 `Debug + Display + Error` | 否 |
| `rbac/error.rs` | `RbacError` | 手动 `Debug + Display + Error` | 否 |
| `alliance/mod.rs` | `AllianceError` | `thiserror::Error` | 否 |
| `flow_loader/mod.rs` | `FlowLoadError` | 手动 `Debug + Display + Error` | 否 |
| `flow_loader/validate.rs` | `ValidationError` | 手动 | 否 |
| `pipeline_core/pipeline.rs` | `PipelineError` | 未知 | 否 |
| `types.rs` | `Result<T>` | `anyhow::Result<T>` | 否（完全擦除类型） |

**关键发现**: 平台已有 `mox-error` crate（含完整错误码系统：`MoxError` + `ErrorDomain` + `define_domain_errors!` 宏），但 `mox-ai-expert-svc` **完全未使用**，反而各自为政地定义了 7 套错误。

#### 领域抽象散落在 3 处：

| 位置 | 抽象内容 | 依赖方向 |
|------|---------|---------|
| `expert_traits.rs` | `ExpertRegistry`, `ExpertConsultant`, `AllianceOrchestrator` | 依赖内部 concrete 实现 |
| `domain/mod.rs` | `GovernContext` trait, `GovernExpert` trait, `GovernVerdict` | 依赖 `mox_ai_flow_svc::FlowGraph` |
| `services.rs` | 所有 concrete 实现 + `AllianceService` 上帝门面 | 依赖几乎所有内部模块 |

### 2.3 问题三：协议定义不统一

#### 2.3.1 DTO 与领域类型混用

`types.rs` 中混合了：
- **领域级协议类型**: `ExpertMeta`, `ConsultQuery`, `ConsultReport`, `TaskSpec`, `RoutingDecision`（3 个 trait 使用）
- **HTTP API DTO**: `RegisterExpertRequest/Response`, `ExpertListQuery/Response`, `ConsultExpertRequest/Response`, `MultiExpertConsultRequest/Response`, `ExpertDebateRequest/Response` 等 20+ 种

**问题**: 领域协议类型与 HTTP 传输 DTO 混在同一模块，导致下游 crate 可能意外依赖 HTTP 层类型。

#### 2.3.2 审计协议三套并存

```
govern::AuditEvent (DefaultHasher + 8 字段)
      ↓ 内部使用
audit::ExtAuditEvent (SHA-256 + 13 字段 + HMAC 签名)
      ↓ 外部合规
pipeline_core::UnifiedAuditEvent (SHA-256 + 12 字段 + trace_id + phase)
      ↓ 试图统一但不彻底
alliance::AllianceEvent (SSE 事件，含 phase/payload/trace_id)
      ↓ 又一套事件模型
```

**问题**: 四套事件模型字段各异、哈希算法不统一（DefaultHasher vs SHA-256）、签名机制不一致，导致：
- 审计链路断裂（无法跨模块追溯）
- 合规审计时需要多次转换
- 新增审计点时不知道该用哪套

#### 2.3.3 错误处理三套范式

1. **`anyhow::Error` 模式**（`types::Result` + `expert_traits`）：完全类型擦除
2. **`thiserror` 枚举模式**（`AllianceError`）：结构化但孤立
3. **手动 Display 模式**（`AuditError`, `RbacError`, `FlowLoadError`）：重复劳动

**问题**: 跨模块错误传播时需要反复 `.map_err(|e| anyhow::anyhow!(...))`，丢失类型信息，不符合平台已有 `mox-error` 的错误码体系。

### 2.4 问题四：循环依赖风险

```
services.rs
  ├──→ expert_traits.rs (trait 定义)
  ├──→ types.rs (DTO)
  ├──→ experts/ (具体专家)
  ├──→ pipeline.rs (mox_optimize)
  ├──→ context.rs (GovernContext)
  ├──→ ir.rs (Dimension)
  ├──→ alliance::* (联盟引擎)
  └──→ alliance::team::build_expert_registry()
            ↓
         依赖 experts/
```

`AllianceService`（1200+ 行）作为上帝对象，直接依赖几乎所有内部模块，形成了**以 services.rs 为中心的辐条式依赖**，任何内部模块变更都可能牵动服务层。

---

## 3. 架构设计方案

### 3.1 分层架构总览

```
┌─────────────────────────────────────────────────────────────────┐
│                     L2 适配层 (Adapter)                          │
│  mox-ai-expert-server  │  mox-ai-expert-cli  │  mox-flow-loader  │
│  (HTTP/gRPC)           │  (命令行)            │  (YAML/JSON)      │
├─────────────────────────────────────────────────────────────────┤
│                     L3 服务层 (Service)                          │
│                    mox-ai-expert-svc (精简后)                    │
│         AllianceService 门面 + 依赖注入装配 + 业务编排            │
├─────────────────────────────────────────────────────────────────┤
│                     L4 领域核心层 (Domain Core)                  │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────────┐ │
│  │ mox-ai-     │  │ mox-ai-     │  │ mox-ai-alliance-engine   │ │
│  │ expert-core │  │ expert-proto│  │  (联盟 6 阶段管线)        │ │
│  │ (引擎核心)  │  │ (协议/类型) │  │                          │ │
│  └─────────────┘  └─────────────┘  └──────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                     L5 共享基础设施 (Shared Foundation)          │
│  ┌────────────┐  ┌───────────┐  ┌──────────┐  ┌─────────────┐  │
│  │ mox-error  │  │ mox-audit │  │ mox-rbac │  │ mox-pipeline│  │
│  │ (统一错误) │  │ (统一审计) │  │ (权限引擎)│  │ (管线框架)  │  │
│  └────────────┘  └───────────┘  └──────────┘  └─────────────┘  │
│  ┌────────────────────────────┐  ┌───────────────────────────┐  │
│  │ mox-platform-foundation    │  │ mox-platform-observability│  │
│  │ (元数据/分层/注册)          │  │ (日志/指标/Tracing)       │  │
│  └────────────────────────────┘  └───────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 设计原则

1. **DIP 依赖倒置**：上层依赖抽象，不依赖具体实现
2. **SSOT 单一真相源**：每种概念只有一个权威定义
3. **关注点分离**：领域模型、引擎实现、服务编排、协议定义各自独立
4. **可独立测试**：每个 crate 可单独编译、单独测试
5. **渐进式迁移**：分阶段解耦，每阶段都可独立交付

---

## 4. 独立 crate 拆分清单

### 4.1 新 crate 规划总览

| 序号 | Crate 名称 | 层级 | 职责边界 | 来源模块 | 优先级 |
|------|-----------|------|---------|---------|--------|
| 1 | `mox-ai-expert-proto` | L4 协议层 | 对外 trait + 共享类型 + 统一错误 + 统一事件 | `types.rs`, `expert_traits.rs`, `domain/mod.rs` 部分 | P0 |
| 2 | `mox-ai-expert-core` | L4 核心层 | 14 维专家引擎 + IR + 裁决 + 验证 + 治理 | `ir.rs`, `expert.rs`, `experts/`, `pipeline.rs`, `reconcile.rs`, `verify.rs`, `govern.rs`, `sensitivity.rs` | P0 |
| 3 | `mox-ai-alliance-engine` | L4 编排层 | 联盟 6 阶段管线（Intent→Team→Debate→Synthesize→Gate→Learn） | `alliance/` | P1 |
| 4 | `mox-audit` | L5 基础设施 | 统一审计引擎（事件模型 + Sink 框架 + Syslog/S3/多Sink） | `audit/` + `govern.rs` 审计链部分 + `pipeline_core/audit.rs` | P1 |
| 5 | `mox-rbac-engine` | L5 基础设施 | RBAC 权限引擎（资源级 + 继承链 + 策略） | `rbac/` | P1 |
| 6 | `mox-pipeline-framework` | L5 基础设施 | 通用管线框架（阶段编排 + 钩子 + 上下文流转） | `pipeline_core/` | P1 |
| 7 | `mox-plugin-harness` | L5 基础设施 | 插件化运行时（Plugin trait + HarnessCtx + 瀑布事件） | `harness.rs` | P2 |
| 8 | `mox-ai-expert-server` | L2 适配层 | HTTP/gRPC 服务端（Axum 路由 + handler） | `server.rs` + `types.rs` 中 HTTP DTO 部分 | P2 |
| 9 | `mox-flow-loader` | L3 适配层 | YAML/JSON 流程加载器（独立于 expert） | `flow_loader/` | P2 |

### 4.2 各 crate 详细设计

#### 4.2.1 mox-ai-expert-proto（协议层）

**定位**: 璇玑专家领域的协议定义 crate，所有下游依赖的唯一入口。

**职责边界**:
- 领域 trait 抽象（`ExpertRegistry`, `ExpertConsultant`, `AllianceOrchestrator`, `GovernExpert`, `GovernContext`）
- 领域值类型（`ExpertMeta`, `ConsultQuery`, `ConsultReport`, `TaskSpec`, `RoutingDecision`）
- 领域枚举（`Dimension`, `GovernLevel`, `GovernVerdict`）
- 统一错误类型（`ExpertError`，基于 `mox-error`）
- 统一事件类型（`ExpertEvent`, `AuditEvent` 投影）
- **不含任何具体实现**

**依赖**:
```
mox-ai-expert-proto
  ├── mox-error (统一错误码系统)
  ├── mox-platform-foundation (元数据)
  ├── serde (序列化)
  ├── async-trait (异步 trait)
  └── uuid (ID 生成)
```

**向下游暴露**:
```rust
pub mod traits;       // ExpertRegistry, ExpertConsultant, AllianceOrchestrator
pub mod domain;       // GovernContext, GovernExpert, GovernVerdict
pub mod types;        // ExpertMeta, ConsultQuery, ConsultReport...
pub mod error;        // ExpertError (统一错误)
pub mod events;       // 统一事件类型
pub mod constants;    // DIM_PRIORITY, DIM_THRESHOLD 等 SSOT 常量
```

#### 4.2.2 mox-ai-expert-core（核心引擎层）

**定位**: 璇玑 14 维专家分析引擎的核心实现。

**职责边界**:
- IR 模型（`Dimension`, `CodeIR`, `DimensionedFlow`）
- Expert trait + 14 个内置专家实现
- 裁决器（reconcile）
- 验证器（verify）
- 治理闸门（govern 8 闸）
- 敏感度判定（sensitivity SSOT）
- 核心管线 `mox_optimize` → `GovernanceReport`

**依赖**:
```
mox-ai-expert-core
  ├── mox-ai-expert-proto (协议层，依赖其 trait 抽象)
  ├── mox-audit (统一审计)
  ├── mox-ai-flow-svc (FlowGraph 模型)
  ├── rayon (并行计算)
  └── serde (序列化)
```

**注意**: 只实现 `mox-ai-expert-proto` 中定义的 trait，不对外暴露 concrete 类型。

#### 4.2.3 mox-ai-alliance-engine（联盟编排层）

**定位**: 专家联盟mox 模块化系统架构分析 6 阶段管线。

**职责边界**:
- 意图识别（Intent）
- 专家组队（Team）
- 并行咨询 + 辩论（Debate）
- 归一合成（Synthesize）
- 质量门禁（Gate）
- 指标学习（Learn）
- KG 连接器（kg_connector）
- 算法分析器（algorithm）
- 任务编排器（orchestration）

**依赖**:
```
mox-ai-alliance-engine
  ├── mox-ai-expert-proto (协议层)
  ├── mox-ai-expert-core (专家引擎)
  ├── mox-pipeline-framework (管线框架)
  ├── mox-audit (统一审计)
  ├── mox-ai-core (LLM 调用，用于 LLM 辩论模式)
  └── mox-kg-sdk (知识图谱)
```

#### 4.2.4 mox-audit（统一审计基础设施）

**定位**: 平台级统一审计引擎，可被所有服务复用。

**职责边界**:
- 统一审计事件模型（整合 4 套为 1 套）
- 审计链（哈希链 + 防篡改验证）
- Sink trait + 多 Sink 组合（MultiSink）
- Syslog Sink 实现
- S3 (WORM) Sink 实现
- 审计上下文（AuditContext）
- HMAC 签名验证

**依赖**:
```
mox-audit
  ├── mox-error (统一错误)
  ├── mox-platform-foundation
  ├── serde
  ├── sha2
  ├── chrono
  ├── uuid
  └── reqwest (S3 Sink)
```

**统一后的事件模型**:
```rust
pub struct AuditEvent {
    pub event_id: String,
    pub timestamp: DateTime<Utc>,
    pub actor: AuditActor,
    pub action: AuditAction,
    pub resource: AuditResource,
    pub outcome: AuditOutcome,
    pub severity: AuditSeverity,
    pub prev_hash: String,       // 哈希链（来自 AuditChain）
    pub content_hash: String,    // 内容哈希
    pub signature: Option<String>, // HMAC 签名（来自 ExtAuditEvent）
    pub tenant_id: String,
    pub trace_id: Option<Uuid>,  // trace 关联（来自 alliance）
    pub phase: Option<String>,   // 阶段关联（来自 pipeline_core）
    pub session_id: Option<String>,
    pub client_ip: Option<String>,
    pub extra: Map<String, Value>,
}
```

#### 4.2.5 mox-rbac-engine（RBAC 权限引擎）

**定位**: 平台级 RBAC 权限引擎。

**职责边界**:
- RBAC 策略模型
- 角色继承链
- 资源级权限检查
- 循环继承检测
- 与审计集成（权限拒绝自动审计）

**依赖**:
```
mox-rbac-engine
  ├── mox-error (统一错误)
  ├── mox-audit (审计集成)
  └── serde
```

#### 4.2.6 mox-pipeline-framework（通用管线框架）

**定位**: 通用阶段编排框架，可被 alliance、expert-core 等复用。

**职责边界**:
- `Pipeline` / `PipelineBuilder`
- `Phase` / `PhaseHandler` trait
- `PipelineContext` 上下文流转
- `WaterfallHook` 瀑布钩子
- 同步/异步统一执行
- 统一审计集成

**依赖**:
```
mox-pipeline-framework
  ├── mox-error
  ├── mox-audit
  ├── async-trait
  └── serde
```

#### 4.2.7 mox-plugin-harness（插件运行时）

**定位**: 插件化运行时框架。

**职责边界**:
- `Plugin` trait
- `PluginMeta` 元信息
- `HarnessCtx` 共享上下文
- `WaterfallEvent` 瀑布事件
- 插件依赖排序

**依赖**:
```
mox-plugin-harness
  ├── mox-error
  └── serde
```

#### 4.2.8 mox-ai-expert-server（HTTP 适配层）

**定位**: HTTP/gRPC 服务端适配。

**职责边界**:
- Axum 路由定义
- HTTP handler
- 请求/响应 DTO（HTTP 层专用）
- SSE 流式响应
- 健康检查

**依赖**:
```
mox-ai-expert-server
  ├── mox-ai-expert-proto (协议层)
  ├── mox-ai-expert-core (可选，用于装配)
  ├── mox-ai-alliance-engine (可选，用于装配)
  ├── axum
  └── tower-http
```

---

## 5. 共享类型抽取方案

### 5.1 类型分层原则

```
L5 平台共享类型 (mox-platform-foundation / mox-error / mox-audit)
  │
  ├── CrateMeta, AisLayer           ← mox-platform-foundation
  ├── MoxError, ErrorDomain         ← mox-error
  ├── AuditEvent, AuditSink...      ← mox-audit
  └── RbacPolicy, Permission...     ← mox-rbac-engine

L4 领域共享类型 (mox-ai-expert-proto)
  │
  ├── Dimension, GovernVerdict      ← 领域核心枚举/值对象
  ├── ExpertMeta, ConsultQuery...   ← 领域协议类型
  ├── ExpertRegistry trait...       ← 领域抽象 trait
  └── ExpertError                   ← 领域错误码（基于 mox-error）

L3 服务层类型 (mox-ai-expert-svc)
  │
  └── AllianceService (门面，不定义新类型)

L2 适配层类型 (mox-ai-expert-server)
  │
  ├── RegisterExpertRequest/Response  ← HTTP DTO
  ├── ExpertDebateRequest/Response    ← HTTP DTO
  └── ... 其他 HTTP 专用 DTO
```

### 5.2 具体抽取映射

#### 5.2.1 抽取到 mox-ai-expert-proto

| 源位置 | 类型 | 目标模块 | 说明 |
|--------|------|---------|------|
| `ir.rs::Dimension` | `Dimension` enum | `proto::types` | 领域核心枚举，SSOT |
| `ir.rs::DimensionTag` | `DimensionTag` | `proto::types` | 维度标签 |
| `expert.rs::ExpertOpinion` | `ExpertOpinion` | `proto::types` | 专家意见值对象 |
| `expert.rs::Constraint` | `Constraint` | `proto::types` | 约束值对象 |
| `expert.rs::Risk` | `Risk` | `proto::types` | 风险等级 |
| `expert.rs::Suggestion` | `Suggestion` | `proto::types` | 建议值对象 |
| `types.rs::ExpertMeta` | `ExpertMeta` | `proto::types` | 专家元数据 |
| `types.rs::ConsultQuery` | `ConsultQuery` | `proto::types` | 咨询查询 |
| `types.rs::ConsultReport` | `ConsultReport` | `proto::types` | 咨询报告 |
| `types.rs::TaskSpec` | `TaskSpec` | `proto::types` | 任务规格 |
| `types.rs::RoutingDecision` | `RoutingDecision` | `proto::types` | 路由决策 |
| `expert_traits.rs::ExpertRegistry` | trait | `proto::traits` | 注册表抽象 |
| `expert_traits.rs::ExpertConsultant` | trait | `proto::traits` | 咨询抽象 |
| `expert_traits.rs::AllianceOrchestrator` | trait | `proto::traits` | 编排抽象 |
| `domain/mod.rs::GovernContext` | trait | `proto::domain` | 治理上下文抽象 |
| `domain/mod.rs::GovernExpert` | trait | `proto::domain` | 治理专家抽象 |
| `domain/mod.rs::GovernVerdict` | struct | `proto::domain` | 治理裁决值对象 |
| `domain/mod.rs::GovernLevel` | enum | `proto::domain` | 治理等级 |
| `lib.rs::DIM_PRIORITY` | const | `proto::constants` | 维度优先级 SSOT |
| `lib.rs::DIM_THRESHOLD` | const | `proto::constants` | 维度门槛 SSOT |
| `lib.rs::CONFLICT_ESCALATE_PRIORITY_GAP` | const | `proto::constants` | 冲突升级门槛 |
| `lib.rs::NORMALIZATION_WEIGHTS` | const | `proto::constants` | 归一化权重 |
| `lib.rs::dim_priority()` | fn | `proto::constants` | 便捷查询 |
| `lib.rs::dim_threshold()` | fn | `proto::constants` | 便捷查询 |
| `tenant_policy.rs::GateId` | enum | `proto::governance` | 治理闸门 ID |

#### 5.2.2 抽取到 mox-error（AI 域扩展）

| 源位置 | 错误类型 | 目标位置 | 说明 |
|--------|---------|---------|------|
| 新增 | `ExpertError` | `mox_error::ai::expert` | 专家域错误码 |
| 新增 | `AllianceError` | `mox_error::ai::alliance` | 联盟域错误码 |
| 新增 | `AuditError` | `mox_error::platform::audit` | 审计域错误码 |
| 新增 | `RbacError` | `mox_error::user::rbac` | RBAC 错误码 |

**说明**: `mox-error` 已定义 `ErrorDomain::Ai` 和 `ErrorDomain::User`，可直接扩展子模块。审计属于平台级能力，放入 `platform::audit`。

#### 5.2.3 抽取到 mox-audit

| 源位置 | 类型 | 说明 |
|--------|------|------|
| `audit/event.rs::ExtAuditEvent` | `AuditEvent` | 统一事件模型主版本 |
| `audit/event.rs::AuditActor` | `AuditActor` | 行动者 |
| `audit/event.rs::AuditAction` | `AuditAction` | 操作类型枚举 |
| `audit/event.rs::AuditResource` | `AuditResource` | 操作对象 |
| `audit/event.rs::AuditOutcome` | `AuditOutcome` | 操作结果 |
| `audit/event.rs::AuditSeverity` | `AuditSeverity` | 严重程度 |
| `audit/event.rs::ActorSource` | `ActorSource` | 行动者来源 |
| `audit/sink.rs::AuditSink` | trait | Sink 抽象 |
| `audit/sink.rs::MultiSink` | struct | 多 Sink 组合 |
| `audit/sink.rs::NoopSink` | struct | 空实现 |
| `audit/syslog.rs::SyslogSink` | struct | Syslog 实现 |
| `audit/s3.rs::S3Sink` | struct | S3 实现 |
| `audit/integration.rs::AuditContext` | struct | 审计上下文 |
| `govern.rs::AuditChain` | 机制 | 合并到统一审计链 |
| `pipeline_core/audit.rs::UnifiedAuditEvent` | 类型 | 合并到主模型（已包含 trace_id/phase） |

#### 5.2.4 保留在 mox-ai-expert-core（不对外暴露）

| 类型 | 原因 |
|------|------|
| `Expert` trait | 内部引擎 trait，下游不直接使用 |
| `GovernanceReport` | 内部引擎输出，通过 `ConsultReport` 对外投影 |
| `CodeIR` | 内部 IR 模型，下游不直接依赖 |
| `DimensionedFlow` | 内部 IR 扩展 |
| `FlowStatus` | 内部状态机 |
| `GateResult` | 内部治理结果 |
| `ReconciledPlan` | 内部裁决结果 |
| 14 个具体专家 struct | 内部实现 |

#### 5.2.5 保留在 mox-ai-expert-server（HTTP 层 DTO）

以下类型是 HTTP API 专用，不进入领域协议层：

- `RegisterExpertRequest / RegisterExpertResponse`
- `ExpertListQuery / ExpertListResponse`
- `ExpertDetailResponse`
- `ConsultExpertRequest / ConsultExpertResponse`
- `MultiExpertConsultRequest / MultiExpertConsultResponse`
- `SingleExpertResult`
- `RouteExpertsRequest / RouteExpertsResponse`
- `RouteMatch`
- `ExpertDebateRequest / ExpertDebateResponse`
- `ExpertOpinionView`
- `DebateSseEvent`
- `AlgorithmAnalysisRequest / AlgorithmAnalysisResponse`
- `AlgoCheckItem`
- `OrchestrationRequest / OrchestrationResponse`
- `OrchestrationStep`
- `FullAnalysisRequest / FullAnalysisResponse`
- `FullAnalysisOptions`
- `AllianceOverview`
- `ExpertMetrics`
- `AllianceMetricsResponse`

---

## 6. 统一错误类型方案

### 6.1 现状问题

1. **7 套错误类型**：AuditError, RbacError, AllianceError, FlowLoadError, ValidationError, PipelineError, anyhow::Error
2. **未使用平台标准**：`mox-error` crate 已有完整错误码系统，但 expert-svc 完全未集成
3. **类型信息丢失**：`types::Result<T> = anyhow::Result<T>` 完全擦除错误类型
4. **跨模块转换困难**：模块间错误传播需要反复 `.map_err(|e| anyhow::anyhow!(...))`

### 6.2 设计方案

#### 6.2.1 三层错误体系

```
L1: 平台级错误 (mox-error)
    ├── MoxError (统一错误结构体)
    ├── ErrorDomain (域枚举：Ai, Flow, Kg, User, Platform...)
    ├── ErrorLevel (等级：Info/Warning/Error/Critical)
    └── define_domain_errors! 宏

L2: 领域级错误 (mox-ai-expert-proto/src/error.rs)
    ├── ExpertErrors (AI 域·专家模块·错误码常量)
    ├── AllianceErrors (AI 域·联盟模块·错误码常量)
    └── ExpertResult<T> = Result<T, MoxError>

L3: 模块级错误（各 crate 内部）
    ├── 内部使用 thiserror 枚举定义
    └── 通过 From/Into 转换为 MoxError 对外暴露
```

#### 6.2.2 错误码分配（AI 域·专家子域）

基于 `mox-error` 的 `ErrorDomain::Ai`（代码 `AI`）：

| 模块 | 模块码 | 错误码前缀 | 示例 |
|------|--------|-----------|------|
| 专家引擎 | 10 | AI10xxx | AI10001 (专家不存在) |
| 咨询服务 | 11 | AI11xxx | AI11001 (咨询超时) |
| 联盟编排 | 12 | AI12xxx | AI12001 (组队失败) |
| 治理闸门 | 13 | AI13xxx | AI13001 (治理否决) |
| 意图识别 | 14 | AI14xxx | AI14001 (意图分类失败) |
| 算法分析 | 15 | AI15xxx | AI15001 (算法验证失败) |

#### 6.2.3 统一错误代码骨架

```rust
// mox-ai-expert-proto/src/error.rs

use mox_error::{define_domain_errors, ErrorDomain, MoxError, MoxResult};

/// 专家域错误码（AI10xxx - AI15xxx）
pub mod expert {
    use super::*;

    define_domain_errors!(ExpertErrors, Ai,
        // 专家注册模块 (10)
        EXPERT_NOT_FOUND:     (10, 001, "专家不存在", 404, Warning),
        EXPERT_ALREADY_EXISTS:(10, 002, "专家已存在", 409, Warning),
        EXPERT_INVALID:       (10, 003, "专家配置无效", 400, Warning),
        EXPERT_DIMENSION_MISMATCH: (10, 004, "专家维度不匹配", 422, Warning),

        // 咨询模块 (11)
        CONSULT_TIMEOUT:      (11, 001, "专家咨询超时", 504, Error),
        CONSULT_FAILED:       (11, 002, "专家咨询失败", 500, Error),
        CONSULT_EMPTY_QUERY:  (11, 003, "咨询查询不能为空", 400, Warning),
        CONSULT_FLOW_INVALID: (11, 004, "流程图无效", 400, Warning),

        // 治理模块 (13)
        GOVERN_VETOED:        (13, 001, "治理闸门否决", 422, Error),
        GOVERN_SLA_EXCEEDED:  (13, 002, "SLA 超出限制", 422, Warning),
        GOVERN_BUDGET_EXCEEDED: (13, 003, "预算超出限制", 422, Warning),
    );
}

pub mod alliance {
    use super::*;

    define_domain_errors!(AllianceErrors, Ai,
        // 联盟编排模块 (12)
        TEAM_BUILD_FAILED:    (12, 001, "专家组队失败", 500, Error),
        INTENT_CLASSIFY_FAILED: (12, 002, "意图分类失败", 500, Error),
        DEBATE_TIMEOUT:       (12, 003, "专家辩论超时", 504, Error),
        GATE_BLOCKED:         (12, 004, "质量门禁不通过", 422, Error),
        UNAUTHORIZED:         (12, 005, "RBAC 未授权", 403, Warning),
    );
}

/// 便捷类型别名
pub type ExpertResult<T> = MoxResult<T>;
```

#### 6.2.4 迁移策略

1. **第一步**：在 `mox-ai-expert-proto` 中定义基于 `mox-error` 的统一错误
2. **第二步**：`expert_traits` 的 trait 签名从 `anyhow::Result<T>` 改为 `ExpertResult<T>`
3. **第三步**：各模块内部错误通过 `From` trait 转换为 `MoxError`
4. **第四步**：逐步替换 `anyhow::anyhow!(...)` 为具名错误码

---

## 7. 统一事件/审计协议方案

### 7.1 现状问题

1. **4 套审计事件模型**：govern::AuditEvent, audit::ExtAuditEvent, alliance/gate::AuditEvent, pipeline_core::UnifiedAuditEvent
2. **哈希算法不统一**：DefaultHasher (64位) vs SHA-256 (256位)
3. **字段不兼容**：有的用 `subject`，有的用 `actor`；有的用 `flow_id`，有的用 `resource_id`
4. **审计链路断裂**：内部链、外部合规链、管线阶段审计各自为政

### 7.2 统一方案

#### 7.2.1 单一真相源：mox-audit::AuditEvent

以 `ExtAuditEvent` 为基础，整合其他三套的特性：

```rust
// mox-audit 中的统一事件模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    // 基础标识
    pub event_id: String,           // 全局唯一事件 ID (UUID)
    pub timestamp: DateTime<Utc>,   // 事件时间（ISO8601 UTC）
    pub tenant_id: String,          // 租户 ID（多租户隔离）

    // 谁做的
    pub actor: AuditActor,          // 行动者（id + role + source）

    // 做了什么
    pub action: AuditAction,        // 操作类型（枚举）
    pub resource: AuditResource,    // 操作对象（type + id + name）

    // 结果如何
    pub outcome: AuditOutcome,      // 操作结果
    pub severity: AuditSeverity,    // 严重程度（RFC 5424）

    // 防篡改
    pub prev_hash: String,          // 前一事件哈希（链式追溯）
    pub content_hash: String,       // 本事件内容哈希（SHA-256）
    pub signature: Option<String>,  // HMAC 签名（可选）

    // 关联追踪
    pub trace_id: Option<Uuid>,     // 分布式追踪 ID
    pub session_id: Option<String>, // 会话 ID
    pub phase: Option<String>,      // 管线阶段（管线事件有）

    // 扩展
    pub client_ip: Option<String>,  // 客户端 IP
    pub extra: Map<String, Value>,  // 额外上下文
}
```

#### 7.2.2 统一审计链

```
应用层事件 ──→ AuditContext.emit(event) ──→ 哈希链追加
                                          ├─→ 内部存储（内存/DB）
                                          ├─→ SyslogSink（实时告警）
                                          ├─→ S3Sink（合规存档 WORM）
                                          └─→ 自定义 Sink（即插即用）
```

#### 7.2.3 各模块迁移映射

| 原模块 | 原类型 | 迁移目标 | 迁移说明 |
|--------|--------|---------|---------|
| `govern.rs` | `AuditEvent` + `AuditChain` | `mox_audit::AuditEvent` + `AuditChain` | 从 DefaultHasher 升级为 SHA-256，字段对齐 |
| `audit/event.rs` | `ExtAuditEvent` | `mox_audit::AuditEvent` | 增加 trace_id, phase 字段 |
| `alliance/gate.rs` | `AuditEvent` | `mox_audit::AuditEvent` | 直接复用，设置 phase 字段 |
| `pipeline_core/audit.rs` | `UnifiedAuditEvent` | `mox_audit::AuditEvent` | 合并到主模型，删除重复定义 |

#### 7.2.4 领域事件协议（mox-ai-expert-proto）

在协议层定义领域事件，供下游订阅：

```rust
// mox-ai-expert-proto/src/events.rs

use mox_audit::{AuditAction, AuditOutcome, AuditSeverity};
use serde::{Deserialize, Serialize};

/// 专家领域事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum ExpertDomainEvent {
    /// 专家注册
    ExpertRegistered { expert_id: String, dimension: String },
    /// 咨询开始
    ConsultStarted { consult_id: String, expert_id: String },
    /// 咨询完成
    ConsultCompleted { consult_id: String, score: f64, vetoed: bool },
    /// 治理裁决
    GovernVerdictIssued { gate_id: String, level: String, score: f64 },
    /// 联盟分析开始
    AllianceStarted { trace_id: String, team_size: usize },
    /// 联盟分析完成
    AllianceCompleted { trace_id: String, gate_grade: String, total_ms: u64 },
}

impl ExpertDomainEvent {
    /// 转换为统一审计事件
    pub fn to_audit_event(&self, actor: AuditActor, tenant_id: String) -> AuditEvent {
        // ... 转换逻辑
    }
}
```

---

## 8. 模块依赖关系图

### 8.1 重构后完整依赖图

```
                    ┌──────────────────────┐
                    │  mox-ai-expert-server│  L2 适配层
                    │  (HTTP/gRPC)         │
                    └─────────┬────────────┘
                              │ depends on
                              ▼
                    ┌──────────────────────┐
                    │  mox-ai-expert-svc    │  L3 服务层
                    │  (门面 + DI 装配)     │
                    └─────────┬────────────┘
                              │ depends on
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
    ┌─────────────────┐ ┌──────────────┐ ┌──────────────────┐
    │ mox-ai-expert-  │ │mox-ai-       │ │ mox-ai-alliance- │  L4 领域层
    │ core            │ │expert-proto  │ │ engine           │
    │ (14专家引擎)    │ │ (协议/类型)   │ │ (6阶段管线)      │
    └────────┬────────┘ └──────┬───────┘ └─────────┬────────┘
             │                 │                    │
             │ implements      │                    │
             └────────────────►│◄───────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        ▼                      ▼                      ▼
┌──────────────┐       ┌──────────────┐       ┌──────────────┐
│  mox-audit   │       │  mox-rbac    │       │mox-pipeline  │  L5 基础设施
│ (统一审计)   │       │ (权限引擎)   │       │  -framework   │
└──────┬───────┘       └──────┬───────┘       └──────┬───────┘
       │                      │                      │
       └──────────────────────┼──────────────────────┘
                              │
                    ┌─────────┴──────────┐
                    ▼                    ▼
            ┌──────────────┐    ┌──────────────────────┐
            │  mox-error   │    │ mox-platform-        │  L6 平台基础
            │ (错误码系统) │    │ foundation           │
            └──────────────┘    └──────────────────────┘
```

### 8.2 依赖方向规则

1. **自顶向下依赖**：上层可以依赖下层，下层不能依赖上层
2. **协议层最稳定**：`mox-ai-expert-proto` 变更频率最低
3. **核心层依赖协议**：`mox-ai-expert-core` 实现 proto 中的 trait，不被 proto 依赖
4. **基础设施层中立**：`mox-audit`、`mox-rbac`、`mox-pipeline-framework` 是通用能力，不依赖领域层
5. **服务层是组合根**：`mox-ai-expert-svc` 负责 DI 装配，不包含业务逻辑

### 8.3 循环依赖防护

| 风险点 | 防护措施 |
|--------|---------|
| core ↔ proto 循环 | proto 只定义 trait，core 实现 trait；core 可以依赖 proto，proto 绝不依赖 core |
| alliance-engine ↔ expert-core 循环 | alliance 通过 proto trait 调用专家，不直接依赖 core 的 concrete 类型 |
| audit ↔ rbac 循环 | rbac 依赖 audit（审计拒绝事件），audit 不依赖 rbac；audit 的 RBAC 检查通过 trait 注入 |

---

## 9. 迁移路线图

### 阶段 0：准备工作（0.5 人日）

**目标**: 建立基础设施，确保可渐进迁移

1. 创建 `mox-ai-expert-proto` crate 骨架
2. 创建 `mox-audit` crate 骨架
3. 在 `mox-error` 中预注册 AI 专家域错误码段
4. 确认 workspace Cargo.toml 配置

**交付物**:
- 空 crate + Cargo.toml
- CI 通过（空 crate 编译）

**风险**: 低

---

### 阶段 1：协议层抽取（2 人日）— P0

**目标**: 把对外协议从 expert-svc 独立出来，下游改依赖 proto

**步骤**:

1.1 抽取领域类型到 `mox-ai-expert-proto/src/types.rs`
- `Dimension`, `ExpertMeta`, `ConsultQuery`, `ConsultReport`, `TaskSpec`, `RoutingDecision`
- `GovernVerdict`, `GovernLevel`
- SSOT 常量：`DIM_PRIORITY`, `DIM_THRESHOLD` 等

1.2 抽取 trait 抽象到 `mox-ai-expert-proto/src/traits.rs`
- `ExpertRegistry`, `ExpertConsultant`, `AllianceOrchestrator`

1.3 抽取治理抽象到 `mox-ai-expert-proto/src/domain.rs`
- `GovernContext` trait, `GovernExpert` trait

1.4 定义统一错误到 `mox-ai-expert-proto/src/error.rs`
- 基于 `mox-error` 的专家域错误码
- `ExpertResult<T>` 类型别名

1.5 `mox-ai-expert-svc` 改为依赖 `mox-ai-expert-proto`
- 内部类型 re-export 改为从 proto 导入
- 保持对外 API 不变（通过 pub use 兼容）

**验证**:
- 现有测试全部通过
- 下游 crate 改依赖 proto 后可编译
- expert-svc 对外 API 100% 兼容

**风险**: 低（纯类型迁移，逻辑不变）

---

### 阶段 2：统一审计抽取（2 人日）— P1

**目标**: 4 套审计模型统一为 1 套，独立为 mox-audit crate

**步骤**:

2.1 创建 `mox-audit` crate
- 统一 `AuditEvent` 模型（整合 ExtAuditEvent + AuditChain + trace_id + phase）
- `AuditSink` trait + `MultiSink` + `NoopSink`
- `SyslogSink`, `S3Sink`
- `AuditContext` 统一入口

2.2 迁移 `audit/` 模块到 `mox-audit`
- 所有类型和实现搬移
- 修复字段不兼容（用 feature flag 或转换函数过渡）

2.3 迁移 `govern.rs` 中的 `AuditChain`
- 从 DefaultHasher 升级为 SHA-256
- 统一到 mox-audit 的审计链

2.4 迁移 `pipeline_core/audit.rs`
- 删除 `UnifiedAuditEvent` 定义，改用 `mox_audit::AuditEvent`

2.5 expert-svc 中保留 re-export 兼容

**验证**:
- 所有审计相关测试通过
- Syslog/S3 Sink 功能正常
- 哈希链验证正常

**风险**: 中（哈希算法变更可能影响历史数据，需提供迁移工具）

---

### 阶段 3：RBAC + 管线框架抽取（2 人日）— P1

**目标**: 横切基础设施独立

**步骤**:

3.1 创建 `mox-rbac-engine` crate
- 迁移 `rbac/` 全部内容
- 集成 `mox-audit`（权限拒绝自动审计）
- 统一错误到 `mox-error`

3.2 创建 `mox-pipeline-framework` crate
- 迁移 `pipeline_core/` 全部内容
- 集成 `mox-audit`（阶段事件自动审计）
- 集成 `mox-error`（统一错误）

3.3 expert-svc 改为依赖新 crate

**验证**:
- RBAC 测试通过
- 管线框架测试通过
- expert-svc 集成测试通过

**风险**: 低（逻辑不变，位置变更）

---

### 阶段 4：核心引擎抽取（3 人日）— P0

**目标**: 14 维专家引擎独立为 mox-ai-expert-core

**步骤**:

4.1 创建 `mox-ai-expert-core` crate
- 迁移 `ir.rs` (Dimension 已在 proto，此处放 CodeIR 等内部 IR)
- 迁移 `expert.rs` (Expert trait + 核心逻辑)
- 迁移 `experts/` (14 个具体专家)
- 迁移 `reconcile.rs` (裁决器)
- 迁移 `verify.rs` (验证器)
- 迁移 `govern.rs` (治理闸门，审计链已抽走)
- 迁移 `sensitivity.rs` (敏感度 SSOT)
- 迁移 `pipeline.rs` (mox_optimize 核心函数)

4.2 实现 proto 中的 trait
- `ExpertConsultant` → `ExpertServiceImpl`（在 core 中实现）
- `GovernExpert` → 具体治理实现

4.3 expert-svc 中 concrete 实现改为从 core 导入

4.4 harness 插件框架暂留 expert-svc，后续阶段处理

**验证**:
- 所有专家测试通过
- mox_optimize 输出与原实现一致（Golden Test）
- 性能无退化（bench 对比）

**风险**: 中（代码量大，需仔细验证功能一致性）

---

### 阶段 5：联盟引擎抽取（3 人日）— P1

**目标**: 6 阶段联盟管线独立

**步骤**:

5.1 创建 `mox-ai-alliance-engine` crate
- 迁移 `alliance/` 全部内容
- 依赖 `mox-ai-expert-proto` (trait 抽象)
- 依赖 `mox-pipeline-framework` (管线框架)
- 依赖 `mox-audit` (统一审计)

5.2 联盟引擎通过 trait 调用专家（不直接依赖 core）
- 用 `Arc<dyn ExpertConsultant>` 注入
- 用 `Arc<dyn ExpertRegistry>` 注入

5.3 expert-svc 中 `AllianceService` 简化为门面

**验证**:
- 联盟管线测试通过
- 6 阶段事件流正确
- mox 模块化系统架构分析功能正常

**风险**: 中（alliance 与 core 耦合较深，需通过 trait 解耦）

---

### 阶段 6：服务层瘦身 + 适配层独立（2 人日）— P2

**目标**: expert-svc 精简为纯门面 + 装配层

**步骤**:

6.1 创建 `mox-ai-expert-server` crate
- 迁移 `server.rs` (HTTP 路由 + handler)
- 迁移 `types.rs` 中的 HTTP DTO
- 依赖 `mox-ai-expert-proto` + `mox-ai-expert-svc` (门面)

6.2 `mox-ai-expert-svc` 瘦身
- 只保留 `AllianceService` 门面
- 只保留 DI 装配逻辑（构建各组件并注入）
- 不包含任何业务逻辑

6.3 harness 插件框架抽取到 `mox-plugin-harness`（可选）

**验证**:
- HTTP API 完全兼容
- 服务启动正常
- 所有端到端测试通过

**风险**: 低（逻辑不变，位置变更）

---

### 阶段 7：收尾与清理（1 人日）— P2

**目标**: 清理遗留代码，完善文档

**步骤**:
7.1 删除 expert-svc 中所有 re-export 兼容层
7.2 统一所有错误为 `MoxError`，彻底移除 anyhow 滥用
7.3 补充各 crate 文档和示例
7.4 更新架构图和依赖关系图

**总工作量**: 约 15.5 人日

---

## 10. 风险与缓解措施

### 10.1 技术风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| 迁移过程中破坏现有功能 | 中 | 高 | 1. 每阶段独立验证<br>2. 保留 re-export 兼容层<br>3. 丰富的 Golden Test |
| 性能退化 | 低 | 中 | 1. 基准测试对比<br>2. 关键路径性能回归测试 |
| 下游 crate 适配工作量大 | 中 | 中 | 1. proto 层 100% 兼容<br>2. 提供迁移脚本/指南<br>3. 分阶段逐步切换 |
| 循环依赖引入 | 低 | 高 | 1. 严格的依赖方向规则<br>2. CI 中检查依赖图<br>3. Code Review 重点关注 |

### 10.2 组织风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| 团队不习惯新架构 | 中 | 中 | 1. 详细的架构文档<br>2. 代码示例和最佳实践<br>3. 逐步迁移，边做边学 |
| 迁移期间新功能开发受阻 | 中 | 高 | 1. 迁移与功能开发并行<br>2. 兼容层保障功能可继续迭代<br>3. 优先级排序，先迁稳定模块 |

### 10.3 缓解策略总结

1. **兼容优先**: 每个阶段都通过 re-export 保持对外 API 兼容
2. **渐进迁移**: 7 个阶段，每阶段 1-3 人日，可独立交付
3. **测试护航**: 每个 crate 独立测试 + 集成测试 + Golden Test
4. **验证先行**: 先抽协议层，验证边界正确后再迁实现

---

## 附录 A：文件变更映射表

| 原文件 | 目标位置 | 阶段 |
|--------|---------|------|
| `src/types.rs` (领域类型部分) | `mox-ai-expert-proto/src/types.rs` | 1 |
| `src/expert_traits.rs` | `mox-ai-expert-proto/src/traits.rs` | 1 |
| `src/domain/mod.rs` | `mox-ai-expert-proto/src/domain.rs` | 1 |
| `src/ir.rs::Dimension` + 常量 | `mox-ai-expert-proto/src/types.rs` | 1 |
| `src/lib.rs` 常量 | `mox-ai-expert-proto/src/constants.rs` | 1 |
| `src/audit/` | `mox-audit/src/` | 2 |
| `src/govern.rs::AuditChain` | `mox-audit/src/chain.rs` | 2 |
| `src/pipeline_core/audit.rs` | `mox-audit/src/` (合并) | 2 |
| `src/rbac/` | `mox-rbac-engine/src/` | 3 |
| `src/pipeline_core/` | `mox-pipeline-framework/src/` | 3 |
| `src/ir.rs` (剩余部分) | `mox-ai-expert-core/src/ir.rs` | 4 |
| `src/expert.rs` | `mox-ai-expert-core/src/expert.rs` | 4 |
| `src/experts/` | `mox-ai-expert-core/src/experts/` | 4 |
| `src/reconcile.rs` | `mox-ai-expert-core/src/reconcile.rs` | 4 |
| `src/verify.rs` | `mox-ai-expert-core/src/verify.rs` | 4 |
| `src/govern.rs` (剩余部分) | `mox-ai-expert-core/src/govern.rs` | 4 |
| `src/sensitivity.rs` | `mox-ai-expert-core/src/sensitivity.rs` | 4 |
| `src/pipeline.rs` | `mox-ai-expert-core/src/pipeline.rs` | 4 |
| `src/alliance/` | `mox-ai-alliance-engine/src/` | 5 |
| `src/server.rs` | `mox-ai-expert-server/src/server.rs` | 6 |
| `src/types.rs` (HTTP DTO 部分) | `mox-ai-expert-server/src/dto.rs` | 6 |
| `src/harness.rs` | `mox-plugin-harness/src/` | 6 (可选) |
| `src/flow_loader/` | `mox-flow-loader/src/` | 6 (可选) |
| `src/services.rs` | 保留在 `mox-ai-expert-svc` | - |
| `src/context.rs` | 拆分到 proto + core | 1+4 |
| `src/tenant_policy.rs` | 拆分到 proto + core | 1+4 |

---

## 附录 B：重构前后对比

| 指标 | 重构前 | 重构后 | 改善 |
|------|--------|--------|------|
| crate 数量 | 1 | 9+ | 职责分离 |
| 模块数量 | 20+ | 每 crate 3-7 个 | 单一职责 |
| 错误类型数 | 7 套 | 1 套 (MoxError) | 统一 |
| 审计事件模型 | 4 套 | 1 套 | 统一 |
| 领域抽象位置 | 3 处 | 1 处 (proto) | SSOT |
| 单 crate 代码量 | ~15000+ 行 | ~2000 行 (svc 层) | 85%+ 瘦身 |
| 可独立测试性 | 差 | 好 | 每个 crate 独立测试 |
| 编译时间 | 长（单一大 crate） | 短（并行编译） | 显著改善 |
| 团队协作 | 易冲突 | 并行开发 | 改善 |
