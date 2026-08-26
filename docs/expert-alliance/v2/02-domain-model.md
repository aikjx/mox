# 02 - 归一化领域模型

> 版本：v2.0 | 日期：2026-08-26 | 状态：企业级草案
>
> 前置：[00-需求分析](./00-requirements.md) | [01-架构设计](./01-architecture.md)

---

## 一、统一术语表

| 术语 | 英文 | 定义 | 同义词（禁止使用） |
|------|------|------|-------------------|
| 专家 | Expert | 具备领域知识和工具能力的自治 Agent | agent/bot/assistant/角色 |
| 能力 | Capability | 专家可执行的一类操作的抽象声明 | skill/function/feature |
| 工具 | Tool | 能力对应的具体可调用方法（gRPC/HTTP） | method/api/endpoint/接口 |
| 领域 | Domain | 知识/业务的分类范畴 | category/type/领域分类 |
| 任务 | Task | 用户提交的一次协作请求 | job/work/request/请求 |
| 节点 | Node | 协作计划 DAG 中的一个执行单元（一次专家调用） | step/stage/action/步骤 |
| 协作计划 | Plan | 任务的 DAG 执行图（节点+依赖边） | workflow/pipeline/dag/流程 |
| 协作模式 | Mode | 多专家协作的组织方式 | pattern/strategy/模式 |
| 融合策略 | Fusion | 多专家结果的合并方式 | merge/aggregate/合并 |
| 案例 | Case | 评分达标的历史任务，可复用 | template/blueprint/example/模板 |
| 工作记忆 | Working Memory | 当前任务的上下文/中间结果 | context/session/上下文 |
| 关联图谱 | Knowledge Graph | 专家-能力-领域-工具-数据-案例的关联网络 | graph/network/图谱 |

---

## 二、核心实体模型

### 2.1 实体关系总览

```
┌──────────┐     has_capability     ┌──────────────┐
│  Expert  │ ─────────────────────→ │  Capability  │
│  专家     │                        │  能力          │
└────┬─────┘                        └──────┬───────┘
     │ operates_in                           │ requires_tool
     ▼                                       ▼
┌──────────┐     subdomain_of       ┌──────────────┐
│  Domain  │ ←───────────────────── │    Tool      │
│  领域     │                        │  工具          │
└────┬─────┘                        └──────┬───────┘
     │ contains_data                         │ operates_on
     ▼                                       ▼
┌──────────┐     solved_by          ┌──────────────┐
│   Data   │ ←───────────────────── │    Case      │
│  数据     │                        │  案例          │
└──────────┘                        └──────┬───────┘
                                            │ used_capability
                                            ▼
                                     ┌──────────────┐
                                     │  Capability  │
                                     └──────────────┘

┌──────────┐     executed_by        ┌──────────────┐
│   Task   │ ─────────────────────→ │    Expert    │
│  任务     │                        │              │
└────┬─────┘                        └──────────────┘
     │
     │ has_node
     ▼
┌──────────┐
│   Node   │  （DAG 节点）
│  节点     │
└──────────┘
```

### 2.2 Expert（专家）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expert {
    // === 标识 ===
    pub expert_id: String,           // 唯一ID（UUID）
    pub tenant_id: String,           // 租户ID（"system"=系统内置）
    pub name: String,                // 名称（同租户唯一）
    pub version: String,             // 语义版本（semver）

    // === 描述 ===
    pub description: String,         // 人类可读描述
    pub role: ExpertRole,            // 角色枚举
    pub domains: Vec<String>,        // 领域ID列表
    pub capabilities: Vec<ExpertCapability>, // 能力声明列表

    // === 工具绑定 ===
    pub tools: Vec<ToolBinding>,     // 可调用工具列表

    // === 知识 ===
    pub knowledge: ExpertKnowledge,  // 领域知识（图谱子图引用）

    // === 性格/推理风格 ===
    pub personality: Personality,    // 推理风格配置

    // === 记忆配置 ===
    pub memory_config: MemoryConfig, // 记忆策略

    // === 运维 ===
    pub priority: u8,                // 优先级 1-10（冲突仲裁用）
    pub status: ExpertStatus,        // 状态
    pub health: ExpertHealth,        // 健康状态
    pub metadata: HashMap<String, String>, // 扩展元数据

    // === 时间 ===
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum ExpertRole {
    Analyst,        // 分析师
    Builder,        // 构建者
    Auditor,        // 审计者
    Coordinator,    // 协调者
    Reviewer,       // 审核者
    Executor,       // 执行者
    Researcher,     // 研究者
    Custom(String), // 自定义
}

pub enum ExpertStatus {
    Active,         // 活跃（可被匹配）
    Inactive,       // 停用
    Maintenance,    // 维护中
    Deprecated,     // 已弃用（保留历史，不接受新任务）
}
```

### 2.3 Capability（能力）

```rust
pub struct Capability {
    pub capability_id: String,
    pub name: String,                // 唯一名称
    pub description: String,
    pub category: CapabilityCategory, // 能力分类
    pub input_types: Vec<String>,    // 可处理的输入类型
    pub output_types: Vec<String>,   // 可产出的输出类型
    pub confidence: f32,             // 能力本身置信度 0-1
    pub requires_expertise: Vec<String>, // 前置专业知识
    pub tools: Vec<String>,          // 所需工具ID列表
}

pub enum CapabilityCategory {
    Reasoning,      // 推理
    Processing,     // 处理
    Analysis,       // 分析
    Generation,     // 生成
    Retrieval,      // 检索
    Transformation, // 转换
    Validation,     // 校验
    Custom(String),
}
```

### 2.4 Tool（工具）

```rust
pub struct Tool {
    pub tool_id: String,
    pub name: String,                // 唯一名称
    pub description: String,
    pub service_name: String,        // 对应的微服务名
    pub method: String,              // gRPC 方法全限定名
    pub r#async: bool,               // 是否异步
    pub parameters: JsonSchema,      // 参数 schema（JSON Schema）
    pub returns: JsonSchema,         // 返回值 schema
    pub category: String,            // 工具分类
    pub timeout_ms: u64,             // 默认超时
    pub retry_policy: RetryPolicy,   // 重试策略
}
```

### 2.5 Domain（领域）

```rust
pub struct Domain {
    pub domain_id: String,
    pub name: String,                // 唯一名称
    pub description: String,
    pub parent_domain_id: Option<String>, // 父领域（树形）
    pub level: u32,                  // 层级（根=0）
    pub path: String,                // 物化路径 "/root/child/grandchild"
}
```

### 2.6 Task（任务）

```rust
pub struct Task {
    // === 标识 ===
    pub task_id: String,
    pub tenant_id: String,
    pub user_id: String,

    // === 描述 ===
    pub title: String,
    pub description: String,         // 自然语言描述
    pub task_type: TaskType,

    // === 配置 ===
    pub preference: CollaborationPreference, // 协作偏好
    pub inputs: Vec<DataReference>,  // 输入数据
    pub constraints: TaskConstraints, // 约束（超时/预算/质量）

    // === 执行状态 ===
    pub status: TaskStatus,
    pub plan: Option<CollaborationPlan>, // 协作计划（DAG）
    pub progress: f32,               // 进度 0-1
    pub current_node: Option<String>, // 当前执行节点

    // === 结果 ===
    pub result: Option<TaskResult>,  // 最终结果
    pub error: Option<TaskError>,    // 错误信息

    // === 时间 ===
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
}

pub enum TaskStatus {
    Pending,        // 等待调度
    Planning,       // 生成协作计划中
    Running,        // 执行中
    Paused,         // 已暂停（人工干预）
    Completed,      // 已完成
    Failed,         // 失败
    Cancelled,      // 已取消
}
```

### 2.7 Node（DAG 节点）

```rust
pub struct PlanNode {
    pub node_id: String,
    pub task_id: String,
    pub expert_id: String,
    pub expert_name: String,

    // === 执行配置 ===
    pub node_type: NodeType,
    pub inputs: Vec<NodeInputRef>,   // 上游输入引用
    pub outputs: Vec<NodeOutputDef>, // 输出定义
    pub config: HashMap<String, String>, // 节点配置
    pub retry_policy: RetryPolicy,
    pub timeout_ms: u64,

    // === 执行状态 ===
    pub status: NodeStatus,
    pub result: Option<NodeOutput>,  // 执行结果
    pub error: Option<String>,
    pub thoughts: Vec<ExpertThought>, // 专家思考过程
    pub metrics: NodeMetrics,         // 执行指标

    // === 时间 ===
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub enum NodeType {
    Execute,        // 普通执行节点（专家调用）
    Condition,      // 条件节点（if/else 分支）
    Loop,           // 循环节点（for/while）
    Fusion,         // 融合节点（多结果合并）
    HumanReview,    // 人工审核节点（HITL）
    Parallel,       // 并行网关（fan-out）
    Join,           // 汇聚网关（fan-in）
}

pub enum NodeStatus {
    Pending,        // 等待上游
    Ready,          // 上游完成，等待调度
    Running,        // 执行中
    Success,        // 成功
    Failed,         // 失败
    Skipped,        // 跳过（条件不满足）
    Timeout,        // 超时
    Retrying,       // 重试中
}
```

### 2.8 CollaborationPlan（协作计划）

```rust
pub struct CollaborationPlan {
    pub plan_id: String,
    pub task_id: String,
    pub mode: CollaborationMode,
    pub nodes: Vec<PlanNode>,
    pub edges: Vec<PlanEdge>,        // 依赖关系
    pub fusion_strategy: FusionStrategy,
    pub max_iterations: u32,
    pub timeout_ms: u64,
    pub generated_at: DateTime<Utc>,
    pub generator: String,            // 生成者（auto/human/expert-id）
}

pub struct PlanEdge {
    pub from_node: String,
    pub to_node: String,
    pub data_mapping: String,         // 输出→输入映射（JSONPath）
    pub condition: Option<EdgeCondition>, // 条件边
}

pub enum CollaborationMode {
    Auto,           // 自动选择最佳模式
    Serial,         // 串行 Pipeline
    Parallel,       // 并行 Fan-out/Fan-in
    Debate,         // 辩论
    Hierarchical,   // 分层（协调专家主导）
    Iterative,      // 迭代（生成→审核→重做）
    Dynamic,        // 动态（中间结果驱动）
}

pub enum FusionStrategy {
    MajorityVote,   // 多数投票
    WeightedVote,   // 加权投票
    Concatenate,    // 拼接合并
    BestOf,         // 择优选择
    DebateArbitrate,// 辩论仲裁
    IterativeRefine,// 迭代精炼
}
```

### 2.9 Case（案例）

```rust
pub struct Case {
    pub case_id: String,
    pub tenant_id: String,
    pub source_task_id: String,       // 来源任务ID

    // === 描述 ===
    pub title: String,
    pub description: String,
    pub task_type: TaskType,
    pub input_summary: String,
    pub output_summary: String,

    // === 协作快照 ===
    pub expert_ids: Vec<String>,      // 参与专家
    pub mode: CollaborationMode,
    pub fusion_strategy: FusionStrategy,
    pub plan_snapshot: CollaborationPlan, // 计划快照

    // === 评分 ===
    pub rating: f32,                  // 用户评分 0-5
    pub success_rate: f32,            // 复现成功率
    pub execution_time_ms: u64,       // 典型执行时间

    // === 时间 ===
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub use_count: u32,
}
```

---

## 三、状态机

### 3.1 Task 状态机

```
                    ┌─────────┐
                    │ Pending │←──────────────────┐
                    └────┬────┘                    │
                         │ 调度器接收                │ 取消
                         ▼                           │
                    ┌─────────┐                    │
                    │Planning │                    │
                    └────┬────┘                    │
                         │ 计划生成完成              │
                         ▼                           │
                    ┌─────────┐    暂停           │
                    │ Running │──────────────┐    │
                    └────┬────┘              │    │
                         │                    │    │
          ┌──────────────┼──────────────┐    │    │
          │              │              │    │    │
          ▼              ▼              ▼    │    │
    ┌──────────┐  ┌──────────┐  ┌──────────┐│    │
    │Completed │  │  Failed  │  │ Cancelled│◄────┘
    └──────────┘  └──────────┘  └──────────┘
          ▲              │
          │              │ 重试
          └──────────────┘
           (人工干预后重试)

    ┌─────────┐
    │ Paused  │───恢复──→ Running
    └─────────┘
```

### 3.2 Node 状态机

```
┌─────────┐   上游完成    ┌─────────┐
│ Pending │──────────────→│  Ready  │
└─────────┘               └────┬────┘
                                │ 调度
                                ▼
                          ┌─────────┐
                          │ Running │
                          └────┬────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
              ▼                ▼                ▼
        ┌──────────┐   ┌──────────┐   ┌──────────┐
        │ Success  │   │  Failed  │   │ Timeout  │
        └──────────┘   └────┬─────┘   └────┬─────┘
                             │                │
                             │ 可重试          │ 可重试
                             ▼                ▼
                        ┌──────────┐   ┌──────────┐
                        │ Retrying │   │ Retrying │
                        └────┬─────┘   └────┬─────┘
                             │                │
                             └────────┬───────┘
                                      │
                                      ▼ (重试成功)
                                 ┌──────────┐
                                 │ Success  │
                                 └──────────┘

        ┌──────────┐
        │ Skipped  │ (条件不满足/人工跳过)
        └──────────┘
```

### 3.3 Expert 状态机

```
┌──────────┐   注册    ┌──────────┐
│ (不存在) │──────────→│  Active  │
└──────────┘           └────┬─────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
        ┌──────────┐   ┌──────────┐   ┌──────────┐
        │ Inactive │   │Maintenance│   │Deprecated│
        └────┬─────┘   └────┬─────┘   └──────────┘
             │              │
             └──────┬───────┘
                    │ 恢复
                    ▼
              ┌──────────┐
              │  Active  │
              └──────────┘
```

---

## 四、归一化数据契约

### 4.1 统一请求元数据

所有跨服务请求必须携带以下元数据（gRPC metadata / HTTP header / JSON-RPC params.meta）：

```rust
pub struct RequestMeta {
    pub request_id: String,       // 请求唯一ID（全链路追踪）
    pub trace_id: String,         // 链路追踪ID
    pub span_id: String,          // 当前Span ID
    pub tenant_id: String,        // 租户ID
    pub user_id: String,          // 用户ID
    pub auth_token: String,       // 认证Token
    pub client_ip: String,        // 客户端IP
    pub user_agent: String,       // 客户端UA
    pub accept_language: String,  // 语言偏好
    pub idempotency_key: Option<String>, // 幂等键
    pub deadline_ms: Option<u64>, // 截止时间
}
```

### 4.2 统一分页请求

```rust
pub struct PageRequest {
    pub page: u32,                 // 页码（从1开始）
    pub page_size: u32,            // 每页大小（默认20，最大100）
    pub sort_by: Option<String>,   // 排序字段
    pub sort_order: Option<SortOrder>, // asc/desc
    pub filter: Option<JsonValue>, // 过滤条件
}
```

### 4.3 统一分页响应

```rust
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}
```

### 4.4 统一错误码

```rust
pub enum ErrorCode {
    // === 通用 0-99 ===
    Ok = 0,
    Unknown = 1,
    InvalidArgument = 2,
    NotFound = 3,
    AlreadyExists = 4,
    PermissionDenied = 5,
    Unauthenticated = 6,
    ResourceExhausted = 7,
    Unavailable = 8,
    DeadlineExceeded = 9,
    Internal = 10,

    // === 专家联盟 1000-1999 ===
    ExpertNotFound = 1000,
    ExpertInvalidDefinition = 1001,
    ExpertToolNotFound = 1002,
    ExpertCapabilityConflict = 1003,
    ExpertHealthCheckFailed = 1004,

    TaskNotFound = 1100,
    TaskInvalidStatus = 1101,
    TaskCancelled = 1102,
    TaskTimeout = 1103,
    TaskQuotaExceeded = 1104,

    NodeExecutionFailed = 1200,
    NodeTimeout = 1201,
    NodeRetryExhausted = 1202,

    PlanGenerationFailed = 1300,
    PlanInvalidDag = 1301,
    PlanCycleDetected = 1302,

    FusionFailed = 1400,
    FusionNoResult = 1401,

    CaseNotFound = 1500,
    CaseSimilarityFailed = 1501,

    // === 协议 2000-2999 ===
    ProtocolNotSupported = 2000,
    ProtocolTranscodeFailed = 2001,
    McpToolNotFound = 2100,
    McpToolCallFailed = 2101,
}
```

---

## 五、领域事件

### 5.1 事件清单

| 事件主题 | 发布者 | 说明 |
|----------|--------|------|
| `expert.alliance.task.created` | alliance-svc | 任务创建 |
| `expert.alliance.task.planning` | alliance-svc | 开始生成计划 |
| `expert.alliance.task.planned` | alliance-svc | 计划生成完成 |
| `expert.alliance.task.started` | alliance-svc | 任务开始执行 |
| `expert.alliance.task.progress` | alliance-svc | 任务进度更新 |
| `expert.alliance.task.paused` | alliance-svc | 任务暂停 |
| `expert.alliance.task.resumed` | alliance-svc | 任务恢复 |
| `expert.alliance.task.completed` | alliance-svc | 任务完成 |
| `expert.alliance.task.failed` | alliance-svc | 任务失败 |
| `expert.alliance.task.cancelled` | alliance-svc | 任务取消 |
| `expert.alliance.node.started` | alliance-svc | 节点开始执行 |
| `expert.alliance.node.completed` | alliance-svc | 节点执行完成 |
| `expert.alliance.node.failed` | alliance-svc | 节点执行失败 |
| `expert.alliance.node.retrying` | alliance-svc | 节点重试 |
| `expert.alliance.fusion.started` | alliance-svc | 结果融合开始 |
| `expert.alliance.fusion.completed` | alliance-svc | 结果融合完成 |
| `expert.registry.expert.registered` | registry-svc | 专家注册 |
| `expert.registry.expert.updated` | registry-svc | 专家更新 |
| `expert.registry.expert.deregistered` | registry-svc | 专家注销 |
| `expert.registry.expert.health_changed` | registry-svc | 专家健康状态变更 |
| `expert.kg.case.created` | kg-svc | 案例创建 |
| `expert.kg.edge.updated` | kg-svc | 关联边权重更新 |

### 5.2 事件格式（CloudEvent）

```json
{
  "specversion": "1.0",
  "type": "expert.alliance.task.progress",
  "source": "mox-expert-alliance-svc",
  "id": "evt-abc123",
  "time": "2026-08-26T10:00:00Z",
  "datacontenttype": "application/json",
  "subject": "task-123",
  "tenant_id": "tenant-456",
  "data": {
    "task_id": "task-123",
    "node_id": "node-789",
    "expert_id": "expert-graph-builder",
    "expert_name": "图谱构建专家",
    "status": "completed",
    "progress": 65.0,
    "duration_ms": 1234
  }
}
```

---

*下一篇：[03-全路径业务流程](./03-business-flow.md)*
