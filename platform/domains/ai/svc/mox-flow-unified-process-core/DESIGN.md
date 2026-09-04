# 流程引擎统一方案设计文档

## 1. 现状分析

### 1.1 五套流程引擎概览

| 引擎 | 所在服务 | 定位 | 核心类型 | 执行模式 |
|------|---------|------|---------|---------|
| flow_engine | agent-svc | 流程图驱动 AI 综合处理核心 | `NodeType` (16种) / `FlowDefinition` | 顺序执行 + 条件分支 |
| workflow_engine | agent-svc | BPMN 风格业务流程引擎 | `WorkflowNodeType` (11种) / `BusinessWorkflow` | BFS 执行 + 并行 + 子流程 |
| pipeline (expert) | expert-svc | mox 模块化系统架构治理流水线 | `GovernanceReport` / 专家插件体系 | 瀑布式阶段流水线 |
| alliance/gate | expert-svc | 联盟管线 + 质量门禁 | `GateScore` / `AlliancePhase` | 6阶段 SSE 管线 |
| pipeline (flow) | flow-svc | 全链路优化流水线 | `NodeKind` (10种) / `FlowGraph` / `OptimizationReport` | 6阶段优化管线 |

### 1.2 重复点详细对比

#### 1.2.1 节点类型定义重复

三套独立的节点类型枚举，语义高度重叠但命名不一致：

**flow_engine::NodeType** (16种):
```
Start, End, Task, Guard, Decision, Event,
LLM, Browser, HttpRequest, Operator,
Condition, Transform, Script,
DataInput, DataOutput, Parallel
```

**types::WorkflowNodeType** (11种):
```
Start, End, Operator, Condition, Parallel,
SubWorkflow, UserTask, Script, AiTask, PluginCall, Delay
```

**flow_svc::model::NodeKind** (10种):
```
Start, End, Task, Decision,
ParallelFork, ParallelJoin,
LoopStart, LoopEnd, Guard, SubFlow
```

语义映射关系：

| 统一语义 | flow_engine | workflow_engine | flow_svc model |
|---------|-------------|-----------------|----------------|
| 开始 | Start | Start | Start |
| 结束 | End | End | End |
| 任务 | Task / Operator | Operator | Task + ToolKind |
| 条件分支 | Condition | Condition | Decision |
| 并行 | Parallel | Parallel | ParallelFork/Join |
| 子流程 | (无) | SubWorkflow | SubFlow |
| AI能力 | LLM | AiTask | Task + ToolKind::Llm |
| 脚本 | Script | Script | (无) |
| 数据IO | DataInput/Output | (无) | (无) |
| 数据转换 | Transform | (无) | (无) |
| 守卫/门禁 | Guard | (无) | Guard |
| 用户任务 | (无) | UserTask | (无，ToolKind::Human) |
| HTTP请求 | HttpRequest | (无，PluginCall间接) | Task + ToolKind::Http |
| 浏览器 | Browser | (无) | Task + ToolKind::Browser |
| 插件调用 | (无) | PluginCall | (无) |
| 延迟 | (无) | Delay | (无) |
| 循环 | (无) | (无，Condition模拟) | LoopStart/LoopEnd |
| 事件 | Event | (无) | (无) |

**结论**：flow_svc 的 `NodeKind + ToolKind` 二级分类设计最合理（控制节点 vs 工作节点分离），应作为统一类型的基础。

#### 1.2.2 执行引擎逻辑重复

三处独立的执行/遍历逻辑：

| 特性 | flow_engine | workflow_engine | flow_svc pipeline |
|-----|-------------|-----------------|-------------------|
| DAG 校验 | validate_flow + detect_cycle | (无，依赖 BFS 步数防护) | topo_order |
| 循环检测 | DFS 算法 | MAX_EXEC_STEPS 步数防护 | (DAG 假设) |
| 执行遍历 | 顺序单步 (loop + current_node_id) | BFS 队列 (VecDeque) | 拓扑排序分层 |
| 条件分支 | evaluate_condition + 边条件 | eval_condition + true/false_path | Conditional 边 |
| 变量传递 | variables HashMap + 模板替换 | instance.variables + 输出合并 | (数据流分析) |
| 模板替换 | apply_template ({{var}}) | apply_template (${var}) | (无) |
| 执行结果 | NodeExecutionResult | NodeExecutionRecord | (无，输出报告) |
| 错误处理 | FlowError 枚举 | OperatorError + anyhow | (无，panic-free) |

**重复核心算法**：
- 循环检测：`flow_engine` 有完整 DFS 实现，`workflow_engine` 用步数上限降级防护
- 条件表达式求值：两套独立实现（`evaluate_condition` vs `eval_condition`），语法略有不同（`{{var}}` vs `${var}`）
- 模板变量替换：两套独立实现
- DAG 遍历：三种不同遍历策略

#### 1.2.3 错误类型定义重复

| flow_engine::FlowError | workflow_engine (OperatorError) |
|-----------------------|--------------------------------|
| NodeNotFound(String) | (通过 anyhow 间接表达) |
| CycleDetected(String) | (通过 MAX_EXEC_STEPS 间接防护) |
| ExecutionFailed(String) | OperatorError::Other |
| ConditionError(String) | OperatorError::Other |
| InvalidConfig(String) | (通过 create_flow 校验返回) |

expert-svc 和 flow-svc 没有统一的流程错误类型，使用各自领域的错误枚举。

#### 1.2.4 数据结构重复

**流程图定义结构**（三套几乎同构）：

```
flow_engine::FlowDefinition
  ├── id, name, description
  ├── nodes: Vec<FlowNode>
  ├── edges: Vec<FlowEdge>
  ├── variables: HashMap<String, Value>
  └── created_at / updated_at

types::BusinessWorkflow
  ├── id, name, description
  ├── nodes: Vec<WorkflowNode>
  ├── edges: Vec<WorkflowEdge>
  ├── variables: HashMap<String, Value>
  ├── start_node_id
  └── created_at

model::FlowGraph
  ├── id, name
  ├── nodes: Vec<FlowNode>
  ├── edges: Vec<FlowEdge>
  ├── pools: Vec<ResourcePool>
  └── rules: Vec<ExpertRule>
```

**节点结构**（三套）：

```
flow_engine::FlowNode
  ├── id, name
  ├── node_type: NodeType
  ├── config: Value (无类型 JSON)
  └── position: Option<Position>

types::WorkflowNode
  ├── id, name
  ├── node_type: WorkflowNodeType
  ├── config: WorkflowNodeConfig (tagged enum, 有类型)
  └── position: Option<NodePosition>

model::FlowNode
  ├── id, name
  ├── kind: NodeKind
  ├── tool: Option<ToolKind>
  ├── duration_ms
  ├── accesses: Vec<Access>
  ├── tags: Vec<String>
  ├── transactional, idempotent
  └── props: BTreeMap<String, String>
```

**边结构**（三套）：

```
flow_engine::FlowEdge     → id, source, target, condition
types::WorkflowEdge       → id, source, target, condition
model::FlowEdge           → from, to, kind: EdgeKind, condition
```

**执行结果**（两套）：

```
flow_engine::FlowExecutionResult
  ├── flow_id, flow_name, success
  ├── node_results: Vec<NodeExecutionResult>
  ├── output, variables, execution_time_ms, error

types::WorkflowResult
  ├── instance: WorkflowInstance
  ├── final_output
  ├── execution_log: Vec<String>
  └── metrics: WorkflowMetrics
```

---

## 2. 统一方案设计

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                   各业务服务 (差异化层)                       │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  agent-svc  │  │  expert-svc  │  │    flow-svc      │   │
│  │  (AI能力)   │  │  (治理能力)  │  │  (优化/调度能力)  │   │
│  └──────┬──────┘  └──────┬───────┘  └────────┬─────────┘   │
│         │ 适配层          │ 适配层            │ 适配层       │
└─────────┼─────────────────┼───────────────────┼─────────────┘
          │                 │                   │
          ▼                 ▼                   ▼
┌─────────────────────────────────────────────────────────────┐
│           mox-flow-unified-process-core (统一核心)           │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
│  │  类型层  │  │  执行层  │  │  工具层  │  │  扩展点   │  │
│  │  types   │  │ executor │  │  utils   │  │ extension │  │
│  └──────────┘  └──────────┘  └──────────┘  └───────────┘  │
│                                                             │
│  UnifiedNodeType / UnifiedFlowGraph / UnifiedFlowError      │
│  FlowExecutor trait / NodeHandler trait                     │
│  模板引擎 / 条件求值器 / DAG 校验 / 循环检测                 │
│  节点扩展注册机制 / 阶段钩子机制                             │
└─────────────────────────────────────────────────────────────┘
```

**设计原则**：
1. **核心只做通用事**：DAG 校验、遍历调度、变量传递、条件求值 —— 这些是三套引擎共有的
2. **差异化通过扩展实现**：AI 调用、治理闸门、代码生成 —— 各服务通过实现 `NodeHandler` trait 注入
3. **向后兼容**：各服务现有类型通过 `From`/`Into` trait 桥接到统一类型
4. **零性能损失**：核心层不引入额外堆分配，trait 对象使用静态分发优先

### 2.2 统一类型定义

#### 2.2.1 节点类型 (UnifiedNodeKind + UnifiedToolKind)

采用 flow_svc 的二级分类设计（控制节点 + 工具类型），扩展以覆盖另外两套引擎的全部语义：

```rust
/// 节点语义类型（控制节点 vs 工作节点 分离）
pub enum UnifiedNodeKind {
    // === 控制节点 (is_control = true) ===
    Start,          // 流程起点
    End,            // 流程终点
    Decision,       // 排他判断分支
    ParallelFork,   // 并行网关：分叉
    ParallelJoin,   // 并行网关：汇合
    LoopStart,      // 循环入口
    LoopEnd,        // 循环出口
    Guard,          // 守卫/门禁节点
    SubFlow,        // 子流程引用

    // === 工作节点 (is_control = false) ===
    Task,           // 通用任务（绑定 ToolKind）
    Script,         // 自定义脚本
    DataInput,      // 数据输入
    DataOutput,     // 数据输出
    Transform,      // 数据转换
    UserTask,       // 人工任务
    Delay,          // 延迟等待
    Event,          // 事件触发
}
```

```rust
/// 工具类别（Task 节点的具体执行器类型）
pub enum UnifiedToolKind {
    Compute,    // 纯计算
    Llm,        // 大模型
    Browser,    // 浏览器 RPA
    Database,   // 数据库
    Http,       // HTTP 请求
    File,       // 文件读写
    Shell,      // 桌面/Shell
    Operator,   // 算子（OUS 体系）
    Plugin,     // 插件调用
    Human,      // 人工审批
}
```

#### 2.2.2 流程图 (UnifiedFlowGraph)

融合三套定义的全部字段，取最丰富的超集：

```rust
pub struct UnifiedFlowGraph {
    pub id: String,
    pub name: String,
    pub description: String,
    pub nodes: Vec<UnifiedFlowNode>,
    pub edges: Vec<UnifiedFlowEdge>,
    pub variables: HashMap<String, serde_json::Value>,
    pub pools: Vec<UnifiedResourcePool>,
    pub rules: Vec<UnifiedExpertRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

#### 2.2.3 节点 (UnifiedFlowNode)

```rust
pub struct UnifiedFlowNode {
    pub id: String,
    pub name: String,
    pub kind: UnifiedNodeKind,
    pub tool: Option<UnifiedToolKind>,
    pub config: UnifiedNodeConfig,      // 类型化配置，替代无类型 JSON
    pub duration_ms: u64,               // 预估耗时
    pub accesses: Vec<UnifiedAccess>,   // 数据访问声明
    pub tags: Vec<String>,              // 语义标签
    pub position: Option<UnifiedPosition>,
    pub transactional: bool,
    pub idempotent: bool,
    pub props: BTreeMap<String, String>,
}
```

#### 2.2.4 节点配置 (UnifiedNodeConfig)

借鉴 `WorkflowNodeConfig` 的 tagged enum 设计，统一所有节点的配置结构：

```rust
#[serde(tag = "type")]
pub enum UnifiedNodeConfig {
    Start,
    End,
    Task {
        tool_config: serde_json::Value,
    },
    Decision {
        expression: String,
    },
    Parallel {
        merge_strategy: UnifiedMergeStrategy,
    },
    Loop {
        condition: String,
        max_iterations: u32,
    },
    Guard {
        guard_type: String,
        rule_id: Option<String>,
    },
    SubFlow {
        flow_id: String,
        input_mapping: HashMap<String, String>,
        output_mapping: HashMap<String, String>,
    },
    Script {
        language: String,
        code: String,
    },
    DataInput {
        value: Option<serde_json::Value>,
        source: Option<String>,
    },
    DataOutput {
        target: Option<String>,
    },
    Transform {
        template: String,
    },
    UserTask {
        assignee: Option<String>,
        form: serde_json::Value,
    },
    Delay {
        duration_ms: u64,
    },
    Event {
        event_type: String,
        payload: serde_json::Value,
    },
}
```

#### 2.2.5 边 (UnifiedFlowEdge)

```rust
pub struct UnifiedFlowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: UnifiedEdgeKind,
    pub condition: Option<String>,
}

pub enum UnifiedEdgeKind {
    Sequence,       // 顺序控制流
    Conditional,    // 条件分支
    Exception,      // 异常流
    InferredData,   // 推断的数据依赖
    Mutex,          // 资源互斥序
}
```

#### 2.2.6 错误类型 (UnifiedFlowError)

统一全部错误语义，使用 thiserror：

```rust
#[derive(Debug, Error)]
pub enum UnifiedFlowError {
    // === 结构校验错误 ===
    #[error("节点不存在: {0}")]
    NodeNotFound(String),

    #[error("边引用不存在的节点: {edge} -> {node}")]
    EdgeRefNotFound { edge: String, node: String },

    #[error("缺少 Start 节点")]
    MissingStartNode,

    #[error("缺少 End 节点")]
    MissingEndNode,

    #[error("检测到循环: {0}")]
    CycleDetected(String),

    #[error("配置无效: {0}")]
    InvalidConfig(String),

    // === 执行错误 ===
    #[error("节点执行失败: {node_id} - {reason}")]
    NodeExecutionFailed { node_id: String, reason: String },

    #[error("条件表达式求值错误: {0}")]
    ConditionError(String),

    #[error("执行步数超限 (>{max_steps})，疑似无限循环")]
    ExecutionStepsExceeded { max_steps: usize },

    #[error("子流程调用失败: {flow_id} - {reason}")]
    SubFlowFailed { flow_id: String, reason: String },

    // === 扩展错误 ===
    #[error("扩展处理器错误: {handler} - {source}")]
    ExtensionError {
        handler: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}
```

#### 2.2.7 执行结果

```rust
pub struct UnifiedExecutionResult {
    pub flow_id: String,
    pub flow_name: String,
    pub success: bool,
    pub node_results: Vec<UnifiedNodeResult>,
    pub final_output: Option<serde_json::Value>,
    pub variables: HashMap<String, serde_json::Value>,
    pub execution_time_ms: u64,
    pub error: Option<String>,
}

pub struct UnifiedNodeResult {
    pub node_id: String,
    pub node_name: String,
    pub node_kind: String,
    pub status: UnifiedNodeStatus,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedNodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Blocked,    // 被 Guard 阻断
    Waiting,    // 等待用户/外部事件
}
```

### 2.3 统一执行器设计

#### 2.3.1 FlowExecutor trait

核心执行器 trait，定义流程执行的标准接口：

```rust
/// 流程执行器 —— 统一所有流程引擎的执行入口
#[async_trait]
pub trait FlowExecutor: Send + Sync {
    /// 执行流程图
    async fn execute(
        &self,
        graph: &UnifiedFlowGraph,
        input: HashMap<String, serde_json::Value>,
    ) -> Result<UnifiedExecutionResult, UnifiedFlowError>;

    /// 校验流程图结构（DAG、节点引用、循环检测等）
    fn validate(&self, graph: &UnifiedFlowGraph) -> Result<(), UnifiedFlowError>;

    /// 注册节点处理器
    fn register_handler(&mut self, kind: UnifiedNodeKind, handler: Box<dyn NodeHandler>);

    /// 获取节点处理器
    fn get_handler(&self, kind: &UnifiedNodeKind) -> Option<&dyn NodeHandler>;
}
```

#### 2.3.2 NodeHandler trait

节点处理器 —— 扩展点，各服务通过实现此 trait 注入差异化能力：

```rust
/// 节点处理器 —— 每个具体节点类型的执行逻辑
#[async_trait]
pub trait NodeHandler: Send + Sync {
    /// 节点类型标识
    fn kind(&self) -> UnifiedNodeKind;

    /// 执行节点
    async fn execute(
        &self,
        node: &UnifiedFlowNode,
        context: &ExecutionContext<'_>,
    ) -> Result<UnifiedNodeResult, UnifiedFlowError>;

    /// 是否可以并行执行（默认 true）
    fn is_parallelizable(&self) -> bool { true }

    /// 预估执行耗时（用于调度，默认 0 表示未知）
    fn estimate_duration_ms(&self, _node: &UnifiedFlowNode) -> u64 { 0 }
}
```

#### 2.3.3 ExecutionContext

执行上下文 —— 节点执行时的环境信息：

```rust
pub struct ExecutionContext<'a> {
    pub variables: &'a HashMap<String, serde_json::Value>,
    pub previous_outputs: &'a HashMap<String, serde_json::Value>,
    pub flow_id: &'a str,
    pub trace_id: &'a str,
    pub extensions: &'a ExtensionRegistry,
}
```

#### 2.3.4 内置执行器 (DagFlowExecutor)

核心内置执行器，实现 DAG 遍历 + 节点调度：

```rust
/// DAG 流程执行器 —— 内置统一执行引擎
pub struct DagFlowExecutor {
    handlers: HashMap<UnifiedNodeKind, Box<dyn NodeHandler>>,
    max_execution_steps: usize,
    default_parallel: bool,
}
```

核心执行逻辑（伪代码）：
```
1. validate(graph) → 结构校验
2. 初始化 variables (合并 input + graph.variables)
3. 拓扑排序 → 确定执行层次
4. 按层遍历（同层可并行）：
   a. 对每个节点：查找 handler → handler.execute()
   b. 更新 variables 和 outputs
   c. Decision 节点：计算条件 → 选择分支
   d. Guard 节点：失败则阻断后续
5. 收集结果 → UnifiedExecutionResult
```

### 2.4 各服务适配层设计

#### 2.4.1 agent-svc 适配层

**职责**：
- 将 `FlowDefinition` 和 `BusinessWorkflow` 转换为 `UnifiedFlowGraph`
- 注入 AI 相关节点处理器（LLM、Browser、Operator、PluginCall）
- 保留对话会话管理能力

**关键类型转换**：

```rust
// flow_engine → 统一核心
impl From<FlowDefinition> for UnifiedFlowGraph { ... }
impl From<NodeType> for UnifiedNodeKind { ... }

// workflow_engine → 统一核心
impl From<BusinessWorkflow> for UnifiedFlowGraph { ... }
impl From<WorkflowNodeType> for UnifiedNodeKind { ... }
impl From<WorkflowNodeConfig> for UnifiedNodeConfig { ... }
```

**AI 能力节点处理器**：
- `LlmNodeHandler`：调用 LLMClient
- `OperatorNodeHandler`：调用算子服务
- `PluginNodeHandler`：调用插件总线
- `BrowserNodeHandler`：浏览器自动化

#### 2.4.2 expert-svc 适配层

**职责**：
- 将治理流水线的各个阶段抽象为流程节点
- 注入治理/专家相关节点处理器（专家评估、质量门禁、审计）
- 保留插件化运行时（HarnessCtx）

**治理节点处理器**：
- `ExpertEvaluateHandler`：并行派发专家评估
- `ReconcileHandler`：裁决归一化
- `VerifyHandler`：璇玑验证网关
- `GovernGateHandler`：治理闸门
- `AuditHandler`：审计链记录

#### 2.4.3 flow-svc 适配层

**职责**：
- `FlowGraph` 与 `UnifiedFlowGraph` 双向转换（flow-svc 是最接近统一模型的）
- 注入优化相关节点处理器
- 保留并行化、调度、代码生成等优化能力

**优化节点处理器**：
- `DataflowAnalyzeHandler`：数据流分析
- `ConflictDetectHandler`：冲突检测
- `ScheduleHandler`：资源受限调度
- `CodegenHandler`：代码生成

### 2.5 统一工具层

从三套引擎中提取通用工具函数到核心库：

| 工具 | flow_engine | workflow_engine | 统一后 |
|-----|-------------|-----------------|--------|
| 模板替换 | apply_template ({{var}}) | apply_template (${var}) | 统一为 `{{var}}` 语法，兼容 `${var}` |
| 条件求值 | evaluate_condition | eval_condition | 统一表达式语法 |
| 循环检测 | detect_cycle (DFS) | MAX_EXEC_STEPS | DFS 算法 + 步数防护双保险 |
| DAG 拓扑排序 | (无) | (无) | 新增 topo_sort |
| 变量解析 | resolve_template | (无) | 统一 resolve_variables |

---

## 3. 迁移路径

### 阶段一：核心库搭建（低风险，无破坏性）
1. 创建 `mox-flow-unified-process-core` crate
2. 实现统一类型定义（`types.rs`）
3. 实现通用工具层（模板、条件求值、DAG 校验）
4. 实现 `DagFlowExecutor` 和 `NodeHandler` trait
5. 编写单元测试覆盖核心逻辑

### 阶段二：flow-svc 适配（最接近统一模型，风险最低）
1. `FlowGraph` 实现 `From<UnifiedFlowGraph>` 和 `Into<UnifiedFlowGraph>`
2. `NodeKind` ↔ `UnifiedNodeKind` 双向转换
3. 优化流水线改造为使用统一执行器 + 优化节点处理器
4. 验证优化结果一致性

### 阶段三：agent-svc flow_engine 适配
1. `FlowDefinition` → `UnifiedFlowGraph` 转换
2. `NodeType` → `UnifiedNodeKind` 转换
3. 用 `DagFlowExecutor` 替换 `FlowEngine::execute_flow`
4. 内置节点（Start/End/Condition/Transform 等）迁移为内置 Handler
5. 验证现有模板和测试通过

### 阶段四：agent-svc workflow_engine 适配
1. `BusinessWorkflow` → `UnifiedFlowGraph` 转换
2. `WorkflowNodeConfig` → `UnifiedNodeConfig` 转换
3. 用 `DagFlowExecutor` 替换 `WorkflowEngine::execute`
4. BPMN 特性（并行合并、子流程、用户任务）作为 Handler 实现
5. 验证所有内置模板通过

### 阶段五：expert-svc 适配
1. 治理流水线重构为统一流程定义 + 治理节点处理器
2. 联盟管线重构为统一流程定义 + 联盟节点处理器
3. 验证治理报告输出一致性

### 阶段六：清理与优化
1. 删除各服务中的重复类型和逻辑
2. 统一错误处理体系
3. 性能基准测试与优化
4. 完善文档与示例

---

## 4. 代码骨架目录结构

```
mox-flow-unified-process-core/
├── Cargo.toml
├── DESIGN.md               # 本文档
├── src/
│   ├── lib.rs              # 模块导出
│   ├── types.rs            # 统一类型定义
│   ├── error.rs            # 统一错误类型
│   ├── executor/
│   │   ├── mod.rs
│   │   ├── trait.rs        # FlowExecutor / NodeHandler trait
│   │   ├── context.rs      # ExecutionContext
│   │   └── dag.rs          # DagFlowExecutor 内置实现
│   ├── handlers/           # 内置节点处理器
│   │   ├── mod.rs
│   │   ├── control.rs      # Start/End/Decision/Parallel
│   │   ├── data.rs         # DataInput/DataOutput/Transform
│   │   └── script.rs       # Script 节点
│   ├── utils/
│   │   ├── mod.rs
│   │   ├── template.rs     # 模板变量替换
│   │   ├── condition.rs    # 条件表达式求值
│   │   ├── dag.rs          # DAG 校验 / 拓扑排序 / 循环检测
│   │   └── variables.rs    # 变量解析工具
│   └── extension/
│       ├── mod.rs
│       └── registry.rs     # 扩展注册机制
```

---

## 5. 关键设计决策

### 5.1 为什么选 flow_svc 的二级分类（NodeKind + ToolKind）

1. **语义清晰**：控制节点（拓扑）和工作节点（执行）职责分离
2. **扩展性好**：新增工具类型不需要改动节点类型枚举
3. **已被验证**：expert-svc 的治理和优化逻辑都基于此模型
4. **映射简单**：另外两套引擎的节点类型都可以清晰映射

### 5.2 为什么用 trait 扩展而不是 feature flag

1. **各服务独立演进**：agent-svc 加 AI 能力不需要影响 expert-svc
2. **编译期裁剪**：未使用的 handler 不会被编译进二进制
3. **运行时灵活**：可以动态注册/替换处理器（便于测试 mock）
4. **依赖清晰**：核心库不依赖任何业务服务

### 5.3 为什么保留三套引擎的适配层而不是直接替换

1. **向后兼容**：现有调用方不需要修改
2. **渐进式迁移**：可以按节点类型逐步迁移
3. **风险可控**：每个阶段都可以独立验证
4. **特性保留**：各服务的差异化能力（如 expert-svc 的 Harness 插件体系）不受影响
