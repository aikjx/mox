# mox-pipeline-framework 架构说明

## 一、概述

`mox-pipeline-framework` 是从 `mox-ai-expert-svc` 的 `pipeline_core` 模块中独立出来的通用管线框架 crate。它提供了一套可复用的阶段式管线执行引擎，支持同步/异步执行、钩子扩展、插件化架构和审计追踪。

**位置**：`platform/domains/foundation/mox-pipeline-framework/`

---

## 二、模块结构

```
mox-pipeline-framework/
├── Cargo.toml
└── src/
    ├── lib.rs        # crate 入口，统一 re-export
    ├── phase.rs      # PhaseId trait, NamedPhase, PhaseHandler, PhaseStatus, PhaseExecution
    ├── context.rs    # PipelineContext, PipelineInput, PipelineOptions, ServiceRegistry, DataBag
    ├── result.rs     # PhaseResult trait, GenericPhaseResult
    ├── pipeline.rs   # Pipeline trait, SyncPipeline, AsyncPipeline, PipelineBuilder
    ├── hooks.rs      # HookRegistry, HookChain, HookEvent, HookError
    ├── plugin.rs     # Plugin trait, PluginRegistry, ExtensionPoint, PluginError
    ├── error.rs      # PipelineError (PL05xxx 错误码段)
    ├── events.rs     # PipelineEvent, PhaseEvent
    └── audit.rs      # UnifiedAuditChain, AuditSink, AuditEvent (本地最小实现)
```

---

## 三、与原 pipeline_core 的关系

### 3.1 保留的核心概念

| 原 pipeline_core 概念 | 新框架对应 | 说明 |
|----------------------|-----------|------|
| `Phase` 枚举 | `PhaseId` trait + `NamedPhase` | 从具体枚举泛化为 trait，支持自定义阶段类型 |
| `PhaseHandler` trait | `PhaseHandler<P: PhaseId>` | 泛型化，绑定到具体阶段类型 |
| `PipelineContext` | `PipelineContext<P: PhaseId>` | 泛型化，移除 expert 特定字段 |
| `Pipeline` trait | `Pipeline<P: PhaseId>` | 泛型化 |
| `SyncPipeline` | `SyncPipeline<P: PhaseId>` | 泛型化 |
| `AsyncPipeline` | `AsyncPipeline<P: PhaseId>` | 泛型化（feature: async） |
| `PipelineBuilder` | `PipelineBuilder<P: PhaseId>` | 泛型化 |
| `HookRegistry` | `HookRegistry<P: PhaseId>` | 泛型化 |
| `HookChain` | `HookChain<P: PhaseId>` | 泛型化 |
| `PhaseEvent` | `PhaseEvent<P: PhaseId>` | 泛型化 |
| `PhaseStatus` | `PhaseStatus` | 保持不变 |
| `PhaseExecution` | `PhaseExecution<P: PhaseId>` | 泛型化 |

### 3.2 剥离的 expert 特定内容

| 移除的内容 | 替代方案 |
|-----------|---------|
| `Phase` 枚举（Normalize/Analyze/Reconcile 等） | 使用者定义自己的阶段枚举，实现 `PhaseId` trait |
| `UnifiedGateResult` | `GenericPhaseResult` 提供通用结果，或使用者自定义 `PhaseResult` 实现 |
| `GateResult` trait | 统一为 `PhaseResult` trait |
| expert 特定的 context 字段 | 通过 `DataBag` 扩展点存储领域特定数据 |
| expert 特定的输入类型 | `PipelineInput` 使用 `serde_json::Value` 泛型表示 |

### 3.3 增强的能力

| 新增能力 | 来源 | 说明 |
|---------|------|------|
| `Plugin` trait | harness.rs 整合 | 插件生命周期管理 |
| `PluginRegistry` | harness.rs 整合 | 插件注册、依赖解析、启用/禁用 |
| `ExtensionPoint` | harness.rs 整合 | 瀑布式扩展点机制 |
| `ServiceRegistry` | 新增 | 类型安全的服务注入容器 |
| `NamedPhase` | 新增 | 基于字符串的动态阶段标识，适合快速原型 |
| `PipelineError` 独立模块 | 重构 | PL05xxx 错误码段，支持 mox-error feature |

---

## 四、与 harness.rs 的关系

### 4.1 整合的插件机制

`harness.rs` 中的插件化运行时被整合为 `plugin.rs` 模块，主要映射关系：

| harness.rs 概念 | 新框架对应 | 变化 |
|----------------|-----------|------|
| `Plugin` trait | `Plugin` trait | 简化为核心生命周期方法 |
| `PluginRegistry` | `PluginRegistry` | 增加依赖排序、循环检测 |
| `ExtensionPoint` | `ExtensionPoint` | 保留瀑布式调用语义 |
| `PluginStatus` | `PluginState` | 重命名，状态更清晰 |
| `load/enable/disable/unload` | `load/enable/disable/unload` | 保持一致 |

### 4.2 与管线的集成方式

插件可以通过以下方式扩展管线行为：

1. **注册服务**：向 `ServiceRegistry` 注入服务实例，供阶段处理器使用
2. **注册钩子**：向 `HookRegistry` 添加 pre/post 钩子
3. **扩展点**：通过 `ExtensionPoint` 提供可替换的实现
4. **阶段处理器**：注册 `PhaseHandler` 实现

---

## 五、expert-svc 迁移路径

### 5.1 迁移步骤

1. **添加依赖**：`mox-ai-expert-svc/Cargo.toml` 中添加 `mox-pipeline-framework` 依赖
2. **定义阶段枚举**：将原 `Phase` 枚举保留在 expert-svc 中，为其实现 `PhaseId` trait
3. **定义领域结果**：保留 `UnifiedGateResult`，为其实现 `PhaseResult` trait
4. **替换导入**：将 `pipeline_core::*` 替换为 `mox_pipeline_framework::*`
5. **保留领域逻辑**：expert 特定的阶段处理器、钩子实现保留在 expert-svc 中
6. **插件化改造**：将 expert 中的各功能模块改造为 `Plugin` 实现

### 5.2 示例：Phase 枚举迁移

```rust
// expert-svc 中保留 Phase 枚举并实现 PhaseId
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExpertPhase {
    Normalize,
    Analyze,
    Reconcile,
    Validate,
    Generate,
    Finalize,
}

impl PhaseId for ExpertPhase {
    fn name(&self) -> &str {
        match self {
            Self::Normalize => "normalize",
            Self::Analyze => "analyze",
            Self::Reconcile => "reconcile",
            Self::Validate => "validate",
            Self::Generate => "generate",
            Self::Finalize => "finalize",
        }
    }

    fn is_terminal(&self) -> bool {
        matches!(self, Self::Finalize)
    }

    fn is_blocking(&self) -> bool {
        matches!(self, Self::Validate)
    }

    fn order(&self) -> u32 {
        match self {
            Self::Normalize => 10,
            Self::Analyze => 20,
            Self::Reconcile => 30,
            Self::Validate => 40,
            Self::Generate => 50,
            Self::Finalize => 100,
        }
    }
}
```

### 5.3 示例：使用 PipelineBuilder

```rust
use mox_pipeline_framework::prelude::*;

let pipeline = PipelineBuilder::new("expert-pipeline")
    .phase(ExpertPhase::Normalize, NormalizeHandler)
    .phase(ExpertPhase::Analyze, AnalyzeHandler)
    .phase(ExpertPhase::Reconcile, ReconcileHandler)
    .hook(AuditHook::new())
    .hook(MetricsHook::new())
    .build_sync();

let mut ctx = PipelineContext::new(input, options);
let result = pipeline.run(&mut ctx);
```

---

## 六、测试统计

### 6.1 测试汇总

| 模块 | 测试数量 | 说明 |
|------|---------|------|
| phase | 14 | PhaseId trait、NamedPhase、PhaseStatus、PhaseExecution、PhaseHandler |
| context | 13 | PipelineContext、DataBag、ServiceRegistry、PipelineOptions |
| result | 8 | PhaseResult trait、GenericPhaseResult、downcast |
| hooks | 10 | HookChain、HookRegistry、HookEvent、审计钩子、指标钩子 |
| pipeline | 10 (sync) + 2 (async) | SyncPipeline、AsyncPipeline、PipelineBuilder、gate、skip |
| plugin | 13 | PluginRegistry、ExtensionPoint、依赖解析、生命周期 |
| error | 6 | PipelineError、错误码唯一性、PL05xxx 前缀 |
| events | 5 | PhaseEvent、PipelineEvent |
| audit | 6 | UnifiedAuditChain、防篡改、自定义 sink |
| lib (集成) | 5 | 端到端管线执行、re-export 验证 |
| **总计** | **85 (default) / 87 (with async)** | |

### 6.2 测试覆盖率方向

| 覆盖类型 | 状态 |
|---------|------|
| 单元测试 | 所有核心模块均有单元测试 |
| 集成测试 | lib_tests 模块提供端到端测试 |
| 边界情况 | 空管线、跳过阶段、阻塞阶段、错误传播 |
| 泛型验证 | 通过 NamedPhase 和自定义枚举验证泛型正确性 |
| 异步测试 | async feature 下验证 AsyncPipeline |

### 6.3 运行命令

```bash
# 默认特性（同步管线）
cargo test

# 启用异步管线
cargo test --features async

# 启用 mox-error 集成（需要 workspace 中存在 mox-error）
cargo test --features mox-error

# 启用审计集成（需要 workspace 中存在 mox-audit）
cargo test --features audit
```

---

## 七、Feature Flags

| Feature | 默认 | 说明 |
|---------|------|------|
| `default` | 是 | 同步管线 + 审计 + 插件 |
| `async` | 否 | 启用 AsyncPipeline 和异步阶段处理器 |
| `audit` | 否 | 集成 mox-audit crate（需 workspace 中存在） |
| `mox-error` | 否 | 集成 mox-error crate（需 workspace 中存在） |

---

## 八、错误码段 PL05xxx

| 错误码 | 含义 |
|--------|------|
| PL05001 | 阶段执行失败 |
| PL05002 | 管线配置错误 |
| PL05003 | 阶段未找到 |
| PL05004 | 上下文操作错误 |
| PL05005 | 钩子执行失败 |
| PL05010 | 插件未找到 |
| PL05011 | 插件已存在 |
| PL05012 | 插件加载失败 |
| PL05013 | 插件依赖缺失 |
| PL05014 | 插件循环依赖 |
| PL05015 | 扩展点未找到 |
| PL05020 | 审计链验证失败 |
| PL05099 | 未知管线错误 |

---

## 九、设计原则

1. **泛型优先**：核心类型均以 `P: PhaseId` 为泛型参数，支持任意阶段标识
2. **Trait 抽象**：通过 trait 定义扩展点（PhaseHandler, PhaseResult, Hook, Plugin, AuditSink）
3. **可选依赖**：mox-error、mox-audit 等内部 crate 通过 feature flag 可选集成
4. **零成本抽象**：泛型单态化，无运行时开销
5. **类型安全**：ServiceRegistry 提供类型安全的服务注入
6. **可测试性**：所有核心组件均可独立测试，不依赖外部服务
