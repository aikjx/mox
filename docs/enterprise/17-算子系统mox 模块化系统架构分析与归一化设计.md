# 算子系统 · mox 模块化系统架构分析与归一化设计

> **文档定位**：璇玑系统算子子系统的**权威技术设计文档**。本文是 `operator-core` crate 的完整分析、设计与演进规划，将"算子系统是什么、为什么这样设计、如何与知识图谱深度集成、企业级归一化优化方向"用一套数学公理体系串起来。
>
> **标准编号**：`OP-STD-V1.0`
> **版本**：v1.0 (ENT) · 最后更新 **2026-08-19**
> **承载底座**：`GR-STD-V1.0`（关图）＋ `PT-STD-V1.0`（六维绑定）＋ `AA-STD-V1.0`（mox 模块化系统架构流程）
> **权威等级**：🟢 权威（算子子系统唯一技术设计基准）

---

## 0 · 北极星：算子系统的一句话定义

> **算子系统是璇玑信息知识图谱关联关系系统的数学内核——它将一切计算抽象为算子，基于六条数学公理构建统一的高维计算框架，为知识图谱的关联关系运算、璇玑mox 模块化系统架构治理、双璇玑十四维诊断提供类型安全、守恒可验、资源可控的计算基础。**

### 0.1 算子系统的核心价值

| 维度 | 价值 | 工程语义 |
|------|------|----------|
| **统一抽象** | 万物皆算子 | 所有操作（图算法、业务流程、AI推理）统一为 `Operator` trait，消除概念孤岛 |
| **量化状态** | 高维状态向量 | `StateVector` 基于希尔伯特空间，使系统状态可度量、可分析、可比较 |
| **组合正确** | 范畴论组合 | 算子组合满足结合律与单位律，保证组合结果的数学正确性 |
| **自洽保证** | 守恒律校验 | L1/L2 守恒律在流水线各阶段实时校验，保证系统数学自洽性 |
| **企业可靠** | 资源约束 | CPU/内存/IO mox 模块化系统架构维度资源监控与限流，保证生产环境稳定性 |
| **安全扩展** | 单子模式 | `Monad` 封装副作用，支持纯函数式组合，避免副作用陷阱 |

### 0.2 六条数学公理

```
公理一：万物皆算子 ──── Operator trait 统一抽象
公理二：高维状态向量 ─── StateVector 希尔伯特空间
公理三：关联关系图 ──── KnowledgeGraph 加权有向图
公理四：范畴论态射 ──── Category 组合子满足结合律
公理五：资源约束优化 ─── ResourceCost / ResourceLimits
公理六：扩展性闭包 ──── Monad 单子模式
```

---

## 1 · 系统架构全景

### 1.1 模块结构

```text
operator-core (算子系统核心)
├── types.rs         类型系统：TypeIdentifier / TypePair / TypeCheck
├── state.rs         状态空间：StateVector（希尔伯特空间）
├── operator.rs      算子Trait：Operator / IdentityOperator / LinearOperator / FunctionOperator
├── category.rs      范畴论：ComposedOperator / Workflow / TensorProductOperator
├── conservation.rs  守恒律：ConservationLaw / ConservationChecker / ResidualMonitor
├── resource.rs      资源约束：ResourceCost / ResourceUsage / ResourceLimits / ResourceMonitor
├── monad.rs         单子模式：Op / StateOp / IO
└── engine.rs        流水线引擎：OperatorPipeline / StageResult / PipelineResult
```

### 1.2 外部依赖关系

| 关联模块 | 关系 | 依赖方向 |
|----------|------|----------|
| `graph-algorithms` | 公理三的具体实现：知识图谱运算 | 被 `operator-core` 引用 |
| `mox-expert` | 算子流水线的治理应用 | 引用 `operator-core` |
| `primiflow-fusion` | 算子融合的六维绑定 | 引用 `operator-core` |
| `flow-ai` | 算子求解的已验证引擎 | 引用 `operator-core` |
| `kg-hub` | 算子结果的知识图谱沉淀 | 引用 `operator-core` + `graph-algorithms` |

### 1.3 核心数据模型

#### StateVector（高维状态向量）

```rust
pub struct StateVector {
    pub data: DVector<f64>,      // 向量数据（nalgebra）
    pub dimension: usize,         // 维度
    pub timestamp: u64,          // 时间戳（毫秒）
    pub metadata: serde_json::Value,  // 元数据
}
```

**关键能力**：
- `norm()` → L2 范数（能量）
- `norm_l1()` → L1 范数（概率和）
- `normalize()` → 归一化到单位范数
- `normalize_probability()` → 归一化到概率分布
- `dot()` → 内积
- `residual()` → 计算与期望态的残差
- `apply_matrix()` → 线性变换

#### OperatorMetadata（算子元数据）

```rust
pub struct OperatorMetadata {
    pub id: String,              // 唯一ID
    pub name: String,            // 算子名称
    pub version: String,         // 版本号
    pub description: String,     // 描述
    pub input_type: String,      // 输入类型标识
    pub output_type: String,     // 输出类型标识
    pub resource_cost: ResourceCost,  // 资源消耗模型
    pub author: String,          // 作者
    pub tags: Vec<String>,       // 标签
}
```

#### Operator（核心算子Trait）

```rust
pub trait Operator: Send + Sync + TypeCheck {
    fn metadata(&self) -> OperatorMetadata;
    fn apply(&self, input: &StateVector, ctx: &mut ExecutionContext) -> Result<StateVector>;
    fn resource_cost(&self) -> ResourceCost { ... }
    fn execute(&self, input: &StateVector, ctx: &mut ExecutionContext) -> Result<ExecutionResult> { ... }
    fn compose<O: Operator + 'static>(self, other: O) -> ComposedOperator { ... }
    fn into_arc(self) -> Arc<dyn Operator> { ... }
}
```

---

## 2 · 六条公理深度解析

### 2.1 公理一：万物皆算子

**核心命题**：璇玑系统中一切计算、变换、推理、分析，都可以抽象为 `Operator` trait 的实现。

**工程实现**：
- `IdentityOperator`：恒等算子，输出等于输入
- `LinearOperator`：线性变换算子 `y = Mx + b`
- `FunctionOperator`：函数算子，包装任意闭包为算子
- `ComposedOperator`：组合算子 `g ∘ f`
- `Workflow`：算子工作流（有序算子序列）
- `TensorProductOperator`：张量积算子（并行执行）

**设计收益**：
- 统一接口：所有操作遵循同一 `Operator` trait，可互换、可组合、可测试
- 类型安全：`TypeCheck` trait 保证输入输出类型匹配
- 资源感知：每个算子声明自身资源消耗，支持资源调度
- 可观测：执行过程自动记录日志与残差

### 2.2 公理二：高维状态向量

**核心命题**：系统状态用希尔伯特空间中的向量表示，支持线性运算、归一化、内积、残差计算。

**工程实现**：
- 基于 `nalgebra::DVector<f64>` 的稠密向量
- 支持 L1/L2 范数、归一化、内积、残差
- 支持线性变换（矩阵乘法）
- 时间戳 + 元数据支持状态版本追踪

**设计收益**：
- 量化可度量：系统状态可精确度量和比较
- 数学可分析：支持谱分析、拉普拉斯运算等
- 可归一化：支持概率分布归一化、能量归一化
- 支持守恒律：为守恒律检查提供数学基础

### 2.3 公理三：关联关系图

**核心命题**：信息实体及其关系构成加权有向图，图运算本身也是算子。

**工程实现**（`graph-algorithms` crate）：
- `KnowledgeGraph`：基于 `petgraph::DiGraph` 的加权有向图
- 节点：`KnowledgeNode`（含嵌入向量、激活值）
- 边：`KnowledgeEdge`（含权重、关系类型）
- 运算：邻接矩阵、拉普拉斯、PageRank、社区发现、激活传播、推荐

**设计收益**：
- 图运算即算子：PageRank、社区发现等图算法实现为 `Operator`
- 节点状态即向量：每个节点携带 `StateVector` 作为属性
- 双向集成：图谱结构 ↔ 算子流水线深度绑定

### 2.4 公理四：范畴论态射

**核心命题**：算子满足范畴论的结合律和单位律，组合结果在数学上正确。

**工程实现**：
- `ComposedOperator`：`g ∘ f` 满足结合律 `(h ∘ g) ∘ f = h ∘ (g ∘ f)`
- `Workflow`：有序算子序列，类型检查保证可组合
- `TensorProductOperator`：并行组合两个算子
- 类型系统：`TypePair::can_compose()` 编译期保证类型匹配

**设计收益**：
- 组合正确：算子组合的类型安全在编译期保证
- 工作流安全：`Workflow::then()` 检查类型兼容性
- 并行安全：张量积支持安全并行执行

### 2.5 公理五：资源约束优化

**核心命题**：每个算子声明资源消耗模型，系统实时监控并约束资源使用。

**工程实现**：
- `ResourceCost`：算子声明的预期资源消耗（CPU周期、内存、磁盘IO、网络IO）
- `ResourceUsage`：实际资源使用情况（时间、峰值内存、IO量）
- `ResourceLimits`：资源限制配置
- `ResourceMonitor`：资源实时监控器

**设计收益**：
- 成本可预测：算子执行前即可预估资源消耗
- 过载可防护：资源超限时自动阻断
- 调度可优化：基于资源声明的智能调度

### 2.6 公理六：扩展性闭包

**核心命题**：算子组合形成 Monad（单子），支持纯函数式的链式组合。

**工程实现**：
- `Op<T>`：结果单子，封装可能失败的计算，满足三大 Monad 定律
  - 左单位律：`return a >>= f = f a`
  - 右单位律：`m >>= return = m`
  - 结合律：`(m >>= f) >>= g = m >>= (λx -> f x >>= g)`
- `StateOp<S, A>`：状态单子，封装带状态的计算
- `IO<A>`：IO 单子，封装副作用

**设计收益**：
- 错误安全：链式调用中错误自动传播
- 状态安全：状态转换可组合、可追溯
- 副作用隔离：IO 操作显式标记，避免隐式副作用

---

## 3 · 算子与知识图谱的集成关系

### 3.1 集成架构

```
┌─────────────────────────────────────────────────────────────┐
│                    算子系统 (operator-core)                    │
│                                                              │
│  Operator Pipeline ──→ StateVector ──→ Conservation Check    │
│         │                    │                     │         │
│         │                    ▼                     │         │
│         │              ┌─────────────┐             │         │
│         └──────────────→│ KnowledgeGraph │←────────┘         │
│                        └─────────────┘                      │
│                                                              │
│  graph-algorithms: 邻接矩阵 / 拉普拉斯 / PageRank / 社区发现  │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 四层集成

#### 层一：节点状态 = 算子状态

- 知识图谱每个节点携带 `StateVector` 作为嵌入向量
- 算子的输入/输出直接作为图谱节点的状态变化
- 支持节点状态的时间序列追踪

#### 层二：图算法 = 算子实例

- PageRank 算法 → 实现为 `Operator`
- 社区发现 → 实现为 `Operator`
- 激活传播 → 实现为 `Operator`
- 智能推荐 → 实现为 `Operator`

#### 层三：图谱运算 = 算子流水线

- 邻接矩阵计算 → `OperatorPipeline`
- 拉普拉斯运算 → `OperatorPipeline`
- 中心性分析 → `OperatorPipeline`

#### 层四：守恒律 = 图谱校验

- L1 守恒律 → 图谱节点概率分布检查
- L2 守恒律 → 图谱能量分布检查
- 守恒闸门 → 图谱变更阻断

---

## 4 · 算子在璇玑mox 模块化系统架构治理中的角色

### 4.1 八阶段流程中的算子贡献

| 阶段 | 算子系统贡献 | 核心类型 |
|------|-------------|----------|
| **S1 需求接入** | `StateVector` 初始化需求状态 | `StateVector::from_vec()` |
| **S2 归一化** | `Workflow` 算子链进行维度着色 | `Workflow::then()` |
| **S3 双璇玑诊断** | `TensorProductOperator` 并行执行多专家 | `TensorProductOperator::new()` |
| **S4 归一化裁决** | `Monad` 模式封装裁决副作用 | `Op::bind()` |
| **S5 flow-ai 求解** | `ComposedOperator` 组合 CPM/RCPSP | `Operator::compose()` |
| **S6 ⛨验证网关** | `ConservationChecker` 守恒律阻断 | `ConservationChecker::check_all()` |
| **S7 治理闸门** | `ResourceMonitor` 资源约束校验 | `ResourceMonitor::check_limits()` |
| **S8 出码/出图** | 算子结果沉淀为图谱节点/边 | `KnowledgeGraph::add_node()` |

### 4.2 双璇玑十四维中的算子

```
业务璇玑（七维）         开发璇玑（七维）
┌─────────────────┐    ┌─────────────────┐
│ Business        │    │ Architecture    │
│ Algorithm       │    │ Security_Code   │
│ Permission ⛨   │    │ Code_Quality    │
│ Resource ⛨      │    │ Performance     │
│ Security ⛨      │    │ Testing        │
│ Data ⛨          │    │ Documentation   │
│ Observability   │    │ Maintainability │
└─────────────────┘    └─────────────────┘
        │                        │
        ▼                        ▼
    Operator Pipeline ──→ Conservation Check
        │                        │
        ▼                        ▼
    Reconcile (归一化裁决) ──→ 守恒闸门
```

每个维度的专家诊断都实现为 `Operator`，双璇玑并行通过 `TensorProductOperator` 实现。

---

## 5 · 归一化优化设计（企业级演进路线图）

### 5.1 优化项总览

| 编号 | 优化项 | 优先级 | 影响范围 | 状态 |
|------|--------|--------|----------|------|
| OP-NORM-01 | 算子类型系统归一化 | **P0** | operator-core | 📋 设计中 |
| OP-NORM-02 | 算子元数据归一化注册表 | **P0** | operator-core + 全局 | 📋 设计中 |
| OP-NORM-03 | 守恒律与图谱深度绑定 | **P1** | operator-core + graph-algorithms | 📋 设计中 |
| OP-NORM-04 | 流水线引擎并行化扩展 | **P1** | operator-core | 📋 设计中 |
| OP-NORM-05 | WASM 算子沙箱 | **P2** | operator-wasm | 📋 框架存在 |
| OP-NORM-06 | 全链路可观测性 | **P1** | operator-core + runtime | 📋 设计中 |

### 5.2 OP-NORM-01：算子类型系统归一化

**现状问题**：
- `OperatorMetadata.input_type` / `output_type` 使用 `String` 类型
- 类型检查在运行时进行，无法编译期保证
- `TypePair` 的组合检查正确但未被算子元数据利用

**优化方案**：

```rust
// Step 1: OperatorMetadata 改用 TypeIdentifier
pub struct OperatorMetadata {
    pub input_type: TypeIdentifier,   // 替代 String
    pub output_type: TypeIdentifier,  // 替代 String
    // ... 其他字段保持不变
}

// Step 2: 构造时自动从 TypeCheck 获取类型
impl OperatorMetadata {
    pub fn from_op<T: Operator>(op: &T) -> Self {
        Self {
            input_type: op.input_type(),
            output_type: op.output_type(),
            // ...
        }
    }
}

// Step 3: 编译期组合检查
fn validate_composition<A: Operator, B: Operator>(a: &A, b: &B) -> Result<()> {
    let pair_a = a.type_pair();
    let pair_b = b.type_pair();
    if !pair_a.can_compose(&pair_b) {
        return Err(OperatorError::CompositionError(
            format!("类型不匹配: {} → {}", pair_a, pair_b)
        ));
    }
    Ok(())
}
```

**验收标准**：
- [ ] 所有算子的 `input_type`/`output_type` 使用 `TypeIdentifier`
- [ ] 组合检查在构造时执行，而非运行时
- [ ] 现有测试全部通过

### 5.3 OP-NORM-02：算子元数据归一化注册表

**现状问题**：
- 算子定义分散在代码（`operator-core`）和数据（`operators.json`）中
- 缺少算子版本管理和依赖声明
- 无法追踪算子的血缘关系（谁组合了谁）

**优化方案**：

```rust
pub struct OperatorRegistry {
    operators: HashMap<String, RegisteredOperator>,
    versions: HashMap<String, Vec<OperatorVersion>>,
    lineage: HashMap<String, Vec<String>>,  // 算子血缘
}

pub struct RegisteredOperator {
    pub id: String,
    pub metadata: OperatorMetadata,
    pub capability: OperatorCapability,
    pub dependencies: Vec<String>,
    pub since_version: String,
    pub deprecated: bool,
}

pub struct OperatorCapability {
    pub input_types: Vec<TypeIdentifier>,
    pub output_types: Vec<TypeIdentifier>,
    pub resource_profile: ResourceCost,
    pub conservation_constraints: Vec<ConservationLaw>,
    pub parallel_safe: bool,
}

impl OperatorRegistry {
    pub fn register(&mut self, op: Arc<dyn Operator>) -> Result<()>;
    pub fn resolve(&self, id: &str, version: Option<&str>) -> Result<Arc<dyn Operator>>;
    pub fn lineage(&self, id: &str) -> Vec<String>;
    pub fn capabilities(&self) -> Vec<OperatorCapability>;
    pub fn find_compatible(&self, input_type: &TypeIdentifier, output_type: &TypeIdentifier) -> Vec<&RegisteredOperator>;
}
```

**验收标准**：
- [ ] 所有算子通过 `OperatorRegistry` 统一注册
- [ ] 支持版本管理与血缘追踪
- [ ] 支持按能力查询兼容算子
- [ ] 现有测试全部通过

### 5.4 OP-NORM-03：守恒律与图谱深度绑定

**现状问题**：
- `ConservationChecker` 独立于图谱运行
- 仅在流水线末端检查，无法实时拦截
- 图谱变更时无守恒保护

**优化方案**：

```rust
// 图谱级守恒保护
pub struct GuardedGraph {
    inner: KnowledgeGraph,
    checker: ConservationChecker,
}

impl GuardedGraph {
    pub fn add_node(&mut self, node: KnowledgeNode) -> Result<()> {
        // 守恒预检查
        self.checker.check_all(&node.state_vector)?;
        self.inner.add_node(node);
        Ok(())
    }

    pub fn add_edge(&mut self, edge: KnowledgeEdge) -> Result<()> {
        // 获取关联节点状态
        let source_state = self.inner.get_node(&edge.source)?.state_vector;
        let target_state = self.inner.get_node(&edge.target)?.state_vector;
        
        // 守恒检查
        let combined = source_state.combine(&target_state)?;
        self.checker.check_all(&combined)?;
        
        self.inner.add_edge(edge);
        Ok(())
    }
}
```

**验收标准**：
- [ ] 图谱 `add_node`/`add_edge` 操作自动守恒检查
- [ ] 违规变更在操作时即阻断
- [ ] 支持图谱级守恒闸门配置
- [ ] 现有测试全部通过

### 5.5 OP-NORM-04：流水线引擎并行化扩展

**现状问题**：
- `OperatorPipeline::run()` 严格串行执行
- `TensorProductOperator` 仅做数据分割模拟并行
- 无真正的并行执行能力

**优化方案**：

```rust
pub struct OperatorPipeline {
    stages: Vec<Arc<dyn Operator>>,
    checker: ConservationChecker,
    strict: bool,
    convergence_window: usize,
    parallel_groups: Vec<Vec<usize>>,  // 并行分组
}

impl OperatorPipeline {
    /// 配置并行分组
    pub fn with_parallel_groups(mut self, groups: Vec<Vec<usize>>) -> Self {
        self.parallel_groups = groups;
        self
    }

    /// 并行执行
    pub fn run_parallel(&self, input: &StateVector, config: &SystemConfig) -> Result<PipelineResult> {
        if self.parallel_groups.is_empty() {
            return self.run(input, config);  // 退化为串行
        }
        
        // 按并行分组执行
        for group in &self.parallel_groups {
            let results: Vec<_> = group.par_iter().map(|&idx| {
                self.stages[idx].execute(&current, &mut ctx)
            }).collect();
            
            // 聚合结果，检查守恒
            self.aggregate_and_check(&results)?;
        }
        // ...
    }
}
```

**验收标准**：
- [ ] 支持配置算子的并行分组
- [ ] 并行执行结果与串行一致
- [ ] 守恒检查在并行边界正确执行
- [ ] 现有测试全部通过

### 5.6 OP-NORM-05：WASM 算子沙箱

**现状**：
- `operator-wasm` crate 存在但功能为空壳
- 缺少 WASM 算子的加载、执行、资源隔离能力

**优化方案**：

```rust
pub struct WasmOperator {
    module: wasmtime::Module,
    instance: wasmtime::Instance,
    memory: wasmtime::Memory,
    resource_quota: ResourceQuota,
}

impl Operator for WasmOperator {
    fn apply(&self, input: &StateVector, ctx: &mut ExecutionContext) -> Result<StateVector> {
        // 1. 检查资源配额
        self.resource_quota.check(&ctx.resources)?;
        
        // 2. 将输入向量写入 WASM 内存
        self.write_state_vector(input)?;
        
        // 3. 调用 WASM 导出函数
        let output_ptr = self.instance.call_export("apply", input_ptr)?;
        
        // 4. 从 WASM 内存读取输出
        self.read_state_vector(output_ptr)
    }
}

impl WasmOperator {
    pub fn from_bytes(bytes: &[u8], quota: ResourceQuota) -> Result<Self>;
    pub fn from_file(path: &str, quota: ResourceQuota) -> Result<Self>;
    pub fn resource_usage(&self) -> ResourceUsage;
    pub fn halt(&mut self) -> Result<()>;  // 强制终止
}
```

**验收标准**：
- [ ] 支持从字节码/文件加载 WASM 算子
- [ ] CPU/内存资源隔离
- [ ] 超时终止能力
- [ ] 算子签名验证
- [ ] 现有测试全部通过

### 5.7 OP-NORM-06：全链路可观测性

**现状问题**：
- `ExecutionResult.logs` 仅记录文本日志
- 缺少算子输入/输出快照
- 缺少残差变化曲线
- 缺少算子调用链追踪

**优化方案**：

```rust
pub struct OperatorTrace {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operator_id: String,
    pub input_snapshot: Option<StateVector>,
    pub output_snapshot: Option<StateVector>,
    pub residual: f64,
    pub resources_used: ResourceUsage,
    pub duration_ms: u64,
    pub timestamp: u64,
    pub children: Vec<OperatorTrace>,
}

pub struct TraceCollector {
    traces: HashMap<String, Vec<OperatorTrace>>,
    config: TraceConfig,
}

impl TraceCollector {
    pub fn record(&mut self, trace: OperatorTrace);
    pub fn get_trace(&self, trace_id: &str) -> Option<&Vec<OperatorTrace>>;
    pub fn residual_curve(&self, trace_id: &str) -> Vec<(u64, f64)>;
    pub fn resource_timeline(&self, trace_id: &str) -> Vec<(u64, ResourceUsage)>;
    pub fn export_json(&self) -> serde_json::Value;
}
```

**验收标准**：
- [ ] 每个算子执行生成 `OperatorTrace`
- [ ] 支持 Trace ID 链路追踪
- [ ] 残差曲线可视化数据
- [ ] 资源时间线数据导出
- [ ] 现有测试全部通过

---

## 6 · 测试策略

### 6.1 单元测试

| 测试模块 | 覆盖范围 | 状态 |
|----------|----------|------|
| `operator.rs` | Identity / Linear / Function 算子 | ✅ 已实现 |
| `state.rs` | StateVector 创建/范数/归一化/残差 | ✅ 已实现 |
| `category.rs` | 组合算子/工作流/张量积/范畴律 | ✅ 已实现 |
| `conservation.rs` | L1/L2/总和守恒/残差监控 | ✅ 已实现 |
| `resource.rs` | 资源成本/使用/限制/监控 | ✅ 已实现 |
| `monad.rs` | Op/StateOp/IO 单子定律 | ✅ 已实现 |

### 6.2 集成测试

| 测试模块 | 覆盖范围 | 状态 |
|----------|----------|------|
| `tests/pipeline.rs` | 流水线端到端测试 | ✅ 已实现 |
| `graph-algorithms` | 图谱运算端到端 | ✅ 已实现 |
| `mox-expert` | 双璇玑mox 模块化系统架构集成 | ✅ 已实现 |

### 6.3 验收标准

| 维度 | 标准 | 目标值 |
|------|------|--------|
| 测试覆盖 | workspace 全量通过 | **644+ passed / 0 failed** |
| 静态质量 | `cargo clippy` | **0 warning** |
| 性能 | 算子流水线吞吐量 | ≥ **1000 ops/sec** |
| 安全 | 类型安全检查 | 编译期保证 |
| 守恒 | 守恒残差闸门 | 阻断级校验 |

---

## 7 · 变更记录

| 版本 | 说明 |
|------|------|
| v1.0 (ENT) | 首版算子系统mox 模块化系统架构分析与归一化设计文档：六条数学公理深度解析、与知识图谱四层集成关系、璇玑mox 模块化系统架构治理中的角色定位、六项归一化优化设计方案（OP-NORM-01~06）、企业级验收标准。 |

---

*本文为活文档，随算子系统演进持续迭代。任何结构变更须在变更记录留痕，并同步更新 `00-INDEX.md`。*