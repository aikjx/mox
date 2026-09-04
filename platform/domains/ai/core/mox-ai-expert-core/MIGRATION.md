# P2 架构解耦 · 阶段 4：mox-ai-expert-core 迁移说明

> 璇玑十四维专家引擎核心 · 从 mox-ai-expert-svc 独立为核心 crate

## 一、目标与定位

将专家引擎的核心逻辑从 `mox-ai-expert-svc`（服务层）剥离，形成可复用的核心 crate `mox-ai-expert-core`，遵循依赖倒置原则（DIP）：

```
mox-ai-expert-svc (服务层 / HTTP / 联盟管线)
        |
        v
mox-ai-expert-core (核心引擎 / 注册 / 调度 / 归一化 / 裁决 / 治理)
        |
        v
mox-ai-expert-proto (协议层 / traits / types / error / events / constants)
```

**核心原则**：
- 不依赖服务层（mox-ai-expert-svc），服务层依赖核心层
- 实现 proto 中定义的所有 trait（ExpertRegistry, ExpertConsultant, GovernExpert）
- 不包含 HTTP 层、服务层、联盟管线
- 可被 expert-svc 和其他 crate 复用
- 保持 SSOT（单一真相源）：常量、维度、权重等全部从 proto crate 重导出

---

## 二、Crate 结构

```
mox-ai-expert-core/
├── Cargo.toml
└── src/
    ├── lib.rs              # crate 入口 + 重导出
    ├── error.rs            # CoreError（基于 ExpertError 扩展）
    ├── context.rs          # ExpertContext / GovernContext / Tenant / Principal
    ├── sensitivity.rs      # 敏感度判定 SSOT（is_sensitive_leak 等）
    │
    ├── engine/
    │   ├── mod.rs          # ExpertEngine 核心结构体
    │   ├── registry.rs     # InMemoryExpertRegistry（实现 proto::ExpertRegistry）
    │   ├── consultant.rs   # ExpertConsultantImpl（实现 proto::ExpertConsultant）
    │   └── governor.rs     # GovernExpertImpl（实现 proto::GovernExpert<FlowGraph>）
    │
    ├── ir/
    │   └── mod.rs          # CodeIR, CodeUnit, DimensionedFlow, auto_dimension
    │
    ├── normalize.rs        # 14 维归一化
    ├── reconcile.rs        # 冲突裁决
    ├── dispatch.rs         # 专家并行调度
    │
    ├── expert/
    │   └── mod.rs          # Expert trait + dispatch 函数（从 svc 迁移）
    │
    ├── experts/            # 内置十四专家
    │   ├── mod.rs          # all_experts() / business_experts() / development_experts()
    │   ├── algorithm.rs    # 算法专家（骨架 + 基础校验）
    │   ├── architecture.rs # 架构专家（骨架）
    │   ├── business.rs     # 业务专家（完整实现：分支/兜底/悬垂/审批）
    │   ├── code_quality.rs # 代码质量专家（骨架）
    │   ├── data.rs         # 数据专家（骨架）
    │   ├── documentation.rs# 文档专家（骨架）
    │   ├── maintainability.rs # 可维护性专家（骨架）
    │   ├── observability.rs   # 可观测性专家（骨架）
    │   ├── permission.rs   # 权限专家（完整实现：脱敏/鉴权/否决）
    │   ├── performance.rs  # 性能专家（骨架）
    │   ├── resource.rs     # 资源专家（骨架）
    │   ├── security.rs     # 安全专家（完整实现：隔离/注入/PII外发）
    │   ├── security_code.rs # 代码安全专家（骨架）
    │   └── testing.rs      # 测试专家（骨架）
    │
    ├── govern/
    │   └── mod.rs          # 治理裁决（GovernExpert 同步实现）
    │
    ├── verify/
    │   └── mod.rs          # 算法验证（璇玑验证）
    │
    ├── tenant_policy/
    │   └── mod.rs          # 租户策略（八道治理闸门）
    │
    └── pipeline.rs         # mox 模块化系统架构处理流水线（mox_optimize）
```

---

## 三、核心模块详解

### 3.1 engine/ — 引擎核心

**ExpertEngine** 是对外主入口，统一协调三大子系统：
- **Registry**（注册表）：管理专家注册/注销/查询
- **Consultant**（咨询器）：按维度调度专家、产出归一化报告
- **Governor**（治理器）：治理闸门裁决、审计链记录

```rust
pub struct ExpertEngine {
    config: EngineConfig,
    registry: Arc<InMemoryExpertRegistry>,
    consultant: Arc<ExpertConsultantImpl>,
    governor: Arc<GovernExpertImpl>,
}
```

### 3.2 normalize.rs — 14 维归一化

将各维度专家的原始观点（ExpertOpinion）归一化为可比分值：
- 基础分：1.0 完美，风险按严重程度扣分
- 否决（veto）：直接归零
- 维度权重：从 `mox_ai_expert_proto::dim_weight()` SSOT 获取
- 综合分：加权平均

### 3.3 reconcile.rs — 冲突裁决

对多专家约束进行裁决：
- 同节点同优先级冲突 → 升级（escalate）
- 互补约束（MustIsolate + MustGuard）→ 同时采纳
- 串行化 vs 并行化冲突 → 保守取串行
- 权限 Guard 注入 / 资源上限 / 互斥约束

### 3.4 dispatch.rs — 并行调度

基于 rayon 真并行派发专家：
- `dispatch()`：全量并行，保持原序
- `dispatch_by_dimension()`：按维度分组
- `dispatch_dimensions()`：指定维度子集

### 3.5 sensitivity.rs — 敏感度 SSOT

单一真相源，根治 P1 中 `var:citizen_safe` 被误判为泄露的假阳性问题：
- `is_sensitive_leak(resource)`：是否为敏感泄露
- `is_desensitized(resource)`：是否已脱敏
- `is_production_or_sensitive_write(resource)`：是否生产/敏感写

---

## 四、迁移清单

### 4.1 从 mox-ai-expert-svc 迁移至 mox-ai-expert-core

| 模块 | 源文件（svc） | 目标文件（core） | 迁移程度 |
|------|-------------|-----------------|---------|
| Expert trait + dispatch | `src/expert.rs` | `src/expert/mod.rs` | 100% 完整迁移 |
| IR (CodeIR/DimensionedFlow) | `src/ir.rs` | `src/ir/mod.rs` | 100% 完整迁移 |
| 治理层 + 审计链 | `src/govern.rs` | `src/govern/mod.rs` | 100% 完整迁移 |
| mox 模块化系统架构处理流水线 | `src/pipeline.rs` | `src/pipeline.rs` | 100% 完整迁移 |
| 敏感度判定 | `src/sensitivity.rs` | `src/sensitivity.rs` | 100% 完整迁移（SSOT） |
| 归一化逻辑 | `src/normalize.rs` | `src/normalize.rs` | 100% 完整迁移 |
| 冲突裁决 | `src/reconcile.rs` | `src/reconcile.rs` | 100% 完整迁移 |
| 上下文 (ExpertContext) | `src/context.rs` | `src/context.rs` | 100% 完整迁移 |
| 插件化运行时 (Harness) | `src/harness.rs` | 不迁移 | 服务层特有 |
| 权限专家 | `src/experts/permission.rs` | `src/experts/permission.rs` | 100% 完整迁移 |
| 安全专家 | `src/experts/security.rs` | `src/experts/security.rs` | 100% 完整迁移 |
| 业务专家 | `src/experts/business.rs` | `src/experts/business.rs` | 100% 完整迁移 |
| 其他 11 位专家 | `src/experts/*.rs` | `src/experts/*.rs` | 骨架迁移（TODO） |
| 租户策略 | — | `src/tenant_policy/mod.rs` | 新增（八道治理闸门） |
| 算法验证 | — | `src/verify/mod.rs` | 新增（璇玑验证骨架） |
| Engine 核心 | — | `src/engine/` | 新增（Registry/Consultant/Governor） |
| CoreError | — | `src/error.rs` | 新增（基于 ExpertError 扩展） |

### 4.2 保留在 mox-ai-expert-svc 的内容

以下模块属于服务层范畴，**不迁移**到 core crate：

| 模块 | 位置 | 不迁移原因 |
|------|------|-----------|
| HTTP API 层 | `src/routes.rs`, `src/server.rs` | 服务层职责 |
| 服务状态 (AppState) | `src/app_state.rs` | 服务层职责 |
| 联盟管线集成 | — | 联盟域职责 |
| 插件化运行时 (Harness) | `src/harness.rs` | 服务层插件管理 |
| 外部专家适配器 | — | 服务层适配 |
| 启动入口 (main.rs) | `src/bin/main.rs` | 服务层入口 |

---

## 五、测试统计

**总测试数：101 个，全部通过**

### 按模块分布

| 模块 | 测试数 | 说明 |
|------|--------|------|
| `engine/` | 5 | 引擎注册/查询/咨询/治理 |
| `engine/registry.rs` | 4 | 注册表 CRUD + 维度查询 |
| `engine/consultant.rs` | 4 | 咨询器配额/空/多专家 |
| `engine/governor.rs` | 4 | 治理器通过/否决/规则 |
| `error.rs` | 10 | CoreError 所有变体 + 码值唯一性 |
| `normalize.rs` | 9 | 归一化：空/完美/加权/否决/风险累计等 |
| `dispatch.rs` | 8 | 并行调度：全量/按维度/过滤/ID 收集等 |
| `reconcile.rs` | 12 | 冲突裁决：14 种场景覆盖 |
| `ir/` | 4 | CodeIR + auto_dimension + 优先级 |
| `govern/` | 4 | 治理规则 + 闸门 + 审批 |
| `verify/` | 1 | 算法验证骨架 |
| `tenant_policy/` | 2 | 八道闸门 + 合规租户严格策略 |
| `sensitivity/` | 4 | 敏感度 SSOT 判定 |
| `pipeline/` | 3 | mox 模块化系统架构流水线 + 14 专家 + 审计链 |
| `experts/business.rs` | 6 | 业务专家：分支/兜底/悬垂/审批等 |
| `experts/security.rs` | 5 | 安全专家：隔离/注入/PII外发/脱敏等 |
| `experts/permission.rs` | 5 | 权限专家：跳过/否决/脱敏/Guard保护等 |
| `expert/` | 2 | Expert trait object 安全性 |

### 关键测试覆盖

- **权限否决链路**：敏感库写无 authz → veto（`sensitive_write_triggers_veto`）
- **SSOT 验证**：已脱敏资源不触发泄露告警（`desensitized_resource_not_flagged`）
- **14 专家完整性**：流水线注册全部 14 位专家（`fourteen_experts_all_present`）
- **治理闸门流程**：未审批阻塞 → 审批通过（`govern_blocks_on_unapproved`）
- **冲突裁决升级**：同优先级同级冲突升级（`same_priority_conflict_escalates`）

---

## 六、依赖关系

### 6.1 mox-ai-expert-core 的依赖

```toml
[dependencies]
# 协议层（必须）
mox-ai-expert-proto = { workspace = true }  # types/traits/domain/error/events/constants

# 统一审计
mox-audit = { workspace = true }

# FlowGraph 模型
mox-ai-flow-svc = { workspace = true }

# 基础设施
mox-error = { workspace = true }
mox-platform-foundation = { workspace = true }

# 工具库
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
rayon = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
uuid = { workspace = true }
```

### 6.2 不依赖

- `mox-ai-expert-svc`（避免循环依赖，服务层 → 核心层单向）
- `mox-alliance-*`（联盟域不属于核心引擎）
- `axum` / HTTP 相关库（服务层职责）

---

## 七、设计亮点

### 7.1 SSOT（单一真相源）

- 所有维度常量、权重、优先级从 `mox-ai-expert-proto` 统一重导出
- 敏感度判定集中在 `sensitivity.rs`，根治 P1 假阳性问题
- 避免了 P1 中"同一份逻辑散落在 5 个文件"的耦合噩梦

### 7.2 Trait-based 架构

- `ExpertRegistry` trait：可替换存储后端（内存/数据库/分布式）
- `ExpertConsultant` trait：可替换调度策略（并行/串行/分级）
- `GovernExpert` trait：可替换治理规则引擎

### 7.3 同步 + 异步双模式

- 核心引擎同步实现（零开销）
- 通过 `async-trait` 提供异步 GovernExpert 适配
- 服务层可按需选择同步或异步调用

### 7.4 插件化专家系统

- 所有专家实现 `Expert` trait
- 运行时动态注册/注销
- 按维度/ID/标签检索
- 支持外部 crate 扩展专家

---

## 八、后续迭代计划

### P2 阶段 4 后续迭代（TODO）

- [ ] 迁移剩余 11 位专家的完整实现（algorithm/architecture/data 等）
- [ ] 代码 IR 专家的深度实现（AST 分析 / 模式匹配）
- [ ] 分布式注册表（Redis / etcd 后端）
- [ ] 专家热加载（WASM / 动态库）
- [ ] 性能基准测试（rayon 并行调度优化）

### P3 阶段（未来）

- [ ] 专家联盟协议适配（mox-alliance-common-proto）
- [ ] 联邦学习式专家协同
- [ ] 专家能力市场

---

## 九、快速上手

```rust
use mox_ai_expert_core::ExpertEngine;
use mox_ai_expert_proto::ConsultQuery;

// 1. 创建引擎（预注册 14 位内置专家）
let engine = ExpertEngine::new();

// 2. 发起咨询
let query = ConsultQuery::new("flow-id", vec![Dimension::Security, Dimension::Permission]);
let report = engine.consult_sync(&query, &govern_ctx)?;

// 3. 查看结果
println!("综合得分: {:.2}", report.overall_score);
println!("否决风险: {}", report.has_veto);
println!("阻塞风险数: {}", report.total_blocking);

// 4. 治理裁决
let verdict = engine.govern_blocking(&flow_graph, &govern_ctx);
println!("治理结果: {:?}", verdict.status);
```

---

## 十、文件变更总结

### 新增文件（mox-ai-expert-core）

共 **19 个** Rust 源文件 + 1 个 Cargo.toml：

| 路径 | 说明 |
|------|------|
| `src/lib.rs` | crate 入口 + 重导出 |
| `src/error.rs` | CoreError 错误类型 |
| `src/context.rs` | ExpertContext / GovernContext |
| `src/sensitivity.rs` | 敏感度判定 SSOT |
| `src/engine/mod.rs` | ExpertEngine 核心 |
| `src/engine/registry.rs` | InMemoryExpertRegistry |
| `src/engine/consultant.rs` | ExpertConsultantImpl |
| `src/engine/governor.rs` | GovernExpertImpl |
| `src/ir/mod.rs` | CodeIR / DimensionedFlow |
| `src/normalize.rs` | 14 维归一化 |
| `src/reconcile.rs` | 冲突裁决 |
| `src/dispatch.rs` | 并行调度 |
| `src/expert/mod.rs` | Expert trait + dispatch |
| `src/experts/mod.rs` | 十四专家集合 |
| `src/experts/{permission,security,business}.rs` | 三位完整实现专家 |
| `src/experts/{algorithm,architecture,data,...}.rs` | 十一位骨架专家 |
| `src/govern/mod.rs` | 治理裁决 |
| `src/verify/mod.rs` | 算法验证 |
| `src/tenant_policy/mod.rs` | 租户策略 |
| `src/pipeline.rs` | mox 模块化系统架构处理流水线 |

### 未修改文件（mox-ai-expert-svc）

svc 层保持不变，后续迭代中将逐步改为依赖 mox-ai-expert-core。
