# 04 - 归一化接口设计

> 版本：v2.0 | 日期：2026-08-26 | 状态：企业级草案
>
> 前置：[00-需求分析](./00-requirements.md) | [01-架构设计](./01-architecture.md) | [02-领域模型](./02-domain-model.md) | [03-业务流程](./03-business-flow.md)

---

## 一、接口设计原则

| 原则 | 说明 |
|------|------|
| **Proto 优先** | 所有接口先定义 .proto，再生成各协议绑定 |
| **统一语义** | 同一能力在 gRPC/JSON-RPC/MCP/REST 下语义一致，仅序列化不同 |
| **版本化** | 接口版本通过 package 名或 path 前缀管理（v1/v2） |
| **向后兼容** | 只增不改不删（新增字段/方法，不修改已有字段语义） |
| **统一错误** | 所有协议使用统一错误码（ErrorCode 枚举），按协议规范映射 |
| **统一元数据** | 所有请求携带 RequestMeta（request_id/trace_id/tenant_id/user_id） |
| **幂等性** | 写操作支持 idempotency_key，重复调用结果一致 |

---

## 二、Proto 定义（核心契约）

### 2.1 公共类型

```protobuf
syntax = "proto3";
package mox.common.v1;

import "google/protobuf/struct.proto";
import "google/protobuf/timestamp.proto";

// === 请求元数据（所有请求必须携带，通过 gRPC metadata 传递）===
message RequestMeta {
  string request_id = 1;
  string trace_id = 2;
  string span_id = 3;
  string tenant_id = 4;
  string user_id = 5;
  string client_ip = 6;
  string user_agent = 7;
  string accept_language = 8;
  string idempotency_key = 9;
  int64 deadline_ms = 10;
}

// === 分页请求 ===
message PageRequest {
  uint32 page = 1;           // 从1开始
  uint32 page_size = 2;      // 默认20，最大100
  string sort_by = 3;
  SortOrder sort_order = 4;
  google.protobuf.Struct filter = 5;
}

enum SortOrder {
  SORT_ORDER_UNSPECIFIED = 0;
  SORT_ORDER_ASC = 1;
  SORT_ORDER_DESC = 2;
}

// === 分页响应 ===
message PageResponse {
  uint64 total = 1;
  uint32 page = 2;
  uint32 page_size = 3;
  uint32 total_pages = 4;
  bool has_next = 5;
  bool has_prev = 6;
}

// === 统一错误 ===
enum ErrorCode {
  ERROR_CODE_UNSPECIFIED = 0;
  ERROR_CODE_OK = 1;
  ERROR_CODE_UNKNOWN = 2;
  ERROR_CODE_INVALID_ARGUMENT = 3;
  ERROR_CODE_NOT_FOUND = 4;
  ERROR_CODE_ALREADY_EXISTS = 5;
  ERROR_CODE_PERMISSION_DENIED = 6;
  ERROR_CODE_UNAUTHENTICATED = 7;
  ERROR_CODE_RESOURCE_EXHAUSTED = 8;
  ERROR_CODE_UNAVAILABLE = 9;
  ERROR_CODE_DEADLINE_EXCEEDED = 10;
  ERROR_CODE_INTERNAL = 11;

  // 专家联盟 1000-1999
  ERROR_CODE_EXPERT_NOT_FOUND = 1000;
  ERROR_CODE_EXPERT_INVALID_DEFINITION = 1001;
  ERROR_CODE_EXPERT_TOOL_NOT_FOUND = 1002;
  ERROR_CODE_TASK_NOT_FOUND = 1100;
  ERROR_CODE_TASK_INVALID_STATUS = 1101;
  ERROR_CODE_TASK_CANCELLED = 1102;
  ERROR_CODE_TASK_TIMEOUT = 1103;
  ERROR_CODE_TASK_QUOTA_EXCEEDED = 1104;
  ERROR_CODE_NODE_EXECUTION_FAILED = 1200;
  ERROR_CODE_NODE_TIMEOUT = 1201;
  ERROR_CODE_NODE_RETRY_EXHAUSTED = 1202;
  ERROR_CODE_PLAN_GENERATION_FAILED = 1300;
  ERROR_CODE_PLAN_INVALID_DAG = 1301;
  ERROR_CODE_PLAN_CYCLE_DETECTED = 1302;
  ERROR_CODE_FUSION_FAILED = 1400;
  ERROR_CODE_CASE_NOT_FOUND = 1500;

  // 协议 2000-2999
  ERROR_CODE_PROTOCOL_NOT_SUPPORTED = 2000;
  ERROR_CODE_PROTOCOL_TRANSCODE_FAILED = 2001;
  ERROR_CODE_MCP_TOOL_NOT_FOUND = 2100;
  ERROR_CODE_MCP_TOOL_CALL_FAILED = 2101;
}

message ErrorDetail {
  ErrorCode code = 1;
  string message = 2;
  map<string, string> details = 3;
  string request_id = 4;
}
```

### 2.2 专家联盟服务 Proto

```protobuf
syntax = "proto3";
package mox.expert.alliance.v1;

import "mox/common/v1/common.proto";
import "google/protobuf/struct.proto";
import "google/protobuf/timestamp.proto";

// ============================================================
//  联盟核心服务（任务管理/协作执行/结果获取）
// ============================================================
service ExpertAllianceService {
  // === 任务管理 ===
  rpc CreateTask(CreateTaskRequest) returns (CreateTaskResponse);
  rpc CancelTask(CancelTaskRequest) returns (CancelTaskResponse);
  rpc GetTask(GetTaskRequest) returns (GetTaskResponse);
  rpc ListTasks(ListTasksRequest) returns (ListTasksResponse);
  rpc GetTaskExecution(GetTaskExecutionRequest) returns (GetTaskExecutionResponse);
  rpc GetTaskResult(GetTaskResultRequest) returns (GetTaskResultResponse);
  rpc RetryNode(RetryNodeRequest) returns (RetryNodeResponse);
  rpc Intervene(InterveneRequest) returns (InterveneResponse);

  // === 流式 ===
  rpc SubscribeTaskProgress(SubscribeTaskProgressRequest) returns (stream TaskProgressEvent);
  rpc StreamTaskOutput(StreamTaskOutputRequest) returns (stream TaskOutputChunk);
}

// ============================================================
//  专家注册中心服务
// ============================================================
service ExpertRegistryService {
  rpc RegisterExpert(RegisterExpertRequest) returns (RegisterExpertResponse);
  rpc UpdateExpert(UpdateExpertRequest) returns (UpdateExpertResponse);
  rpc DeregisterExpert(DeregisterExpertRequest) returns (DeregisterExpertResponse);
  rpc GetExpert(GetExpertRequest) returns (GetExpertResponse);
  rpc ListExperts(ListExpertsRequest) returns (ListExpertsResponse);
  rpc MatchExperts(MatchExpertsRequest) returns (MatchExpertsResponse);
  rpc GetExpertHealth(GetExpertHealthRequest) returns (GetExpertHealthResponse);
  rpc ListTools(ListToolsRequest) returns (ListToolsResponse);
  rpc RefreshTools(RefreshToolsRequest) returns (RefreshToolsResponse);
}

// ============================================================
//  专家 Agent 运行时服务
// ============================================================
service ExpertAgentService {
  rpc ExecuteNode(ExecuteNodeRequest) returns (ExecuteNodeResponse);
  rpc StreamExecute(StreamExecuteRequest) returns (stream StreamExecuteResponse);
  rpc GetAgentState(GetAgentStateRequest) returns (GetAgentStateResponse);
  rpc CancelExecution(CancelExecutionRequest) returns (CancelExecutionResponse);
}

// ============================================================
//  知识图谱服务（专家联盟专用）
// ============================================================
service ExpertKgService {
  rpc MatchExpertsByGraph(MatchExpertsByGraphRequest) returns (MatchExpertsByGraphResponse);
  rpc RecommendCollaboration(RecommendCollaborationRequest) returns (RecommendCollaborationResponse);
  rpc SearchCases(SearchCasesRequest) returns (SearchCasesResponse);
  rpc GetCase(GetCaseRequest) returns (GetCaseResponse);
  rpc PromoteCase(PromoteCaseRequest) returns (PromoteCaseResponse);
  rpc UpdateEdgeWeights(UpdateEdgeWeightsRequest) returns (UpdateEdgeWeightsResponse);
  rpc InitializeGraph(InitializeGraphRequest) returns (InitializeGraphResponse);
}
```

### 2.3 核心消息定义

```protobuf
// === 任务 ===
message CreateTaskRequest {
  string title = 1;
  string description = 2;                    // 自然语言描述
  string task_type = 3;
  CollaborationPreference preference = 4;
  repeated DataReference inputs = 5;
  TaskConstraints constraints = 6;
}

message CollaborationPreference {
  CollaborationMode mode = 1;
  repeated string preferred_expert_ids = 2;
  repeated string excluded_expert_ids = 3;
  FusionStrategy fusion_strategy = 4;
  uint32 max_experts = 5;
  bool human_in_the_loop = 6;
  uint64 timeout_ms = 7;
}

enum CollaborationMode {
  COLLABORATION_MODE_UNSPECIFIED = 0;
  COLLABORATION_MODE_AUTO = 1;
  COLLABORATION_MODE_SERIAL = 2;
  COLLABORATION_MODE_PARALLEL = 3;
  COLLABORATION_MODE_DEBATE = 4;
  COLLABORATION_MODE_HIERARCHICAL = 5;
  COLLABORATION_MODE_ITERATIVE = 6;
  COLLABORATION_MODE_DYNAMIC = 7;
}

enum FusionStrategy {
  FUSION_STRATEGY_UNSPECIFIED = 0;
  FUSION_STRATEGY_MAJORITY_VOTE = 1;
  FUSION_STRATEGY_WEIGHTED_VOTE = 2;
  FUSION_STRATEGY_CONCATENATE = 3;
  FUSION_STRATEGY_BEST_OF = 4;
  FUSION_STRATEGY_DEBATE_ARBITRATE = 5;
  FUSION_STRATEGY_ITERATIVE_REFINE = 6;
}

message TaskConstraints {
  uint64 timeout_ms = 1;
  float max_cost = 2;
  float min_quality = 3;
  uint32 max_iterations = 4;
  map<string, string> custom = 5;
}

message DataReference {
  string ref_id = 1;
  string data_type = 2;
  string location = 3;
  string schema = 4;
  int64 size_bytes = 5;
  map<string, string> metadata = 6;
}

message CreateTaskResponse {
  string task_id = 1;
  TaskStatus status = 2;
  CollaborationPlan plan_preview = 3;
  google.protobuf.Timestamp created_at = 4;
}

enum TaskStatus {
  TASK_STATUS_UNSPECIFIED = 0;
  TASK_STATUS_PENDING = 1;
  TASK_STATUS_PLANNING = 2;
  TASK_STATUS_RUNNING = 3;
  TASK_STATUS_PAUSED = 4;
  TASK_STATUS_COMPLETED = 5;
  TASK_STATUS_FAILED = 6;
  TASK_STATUS_CANCELLED = 7;
}

message CollaborationPlan {
  string plan_id = 1;
  string task_id = 2;
  CollaborationMode mode = 3;
  repeated PlanNode nodes = 4;
  repeated PlanEdge edges = 5;
  FusionStrategy fusion_strategy = 6;
  uint32 max_iterations = 7;
  uint64 timeout_ms = 8;
  google.protobuf.Timestamp generated_at = 9;
}

message PlanNode {
  string node_id = 1;
  string expert_id = 2;
  string expert_name = 3;
  NodeType node_type = 4;
  repeated string input_keys = 5;
  repeated string output_keys = 6;
  map<string, string> config = 7;
  NodeStatus status = 8;
  google.protobuf.Struct result = 9;
  string error = 10;
  repeated ExpertThought thoughts = 11;
  NodeMetrics metrics = 12;
}

enum NodeType {
  NODE_TYPE_UNSPECIFIED = 0;
  NODE_TYPE_EXECUTE = 1;
  NODE_TYPE_CONDITION = 2;
  NODE_TYPE_LOOP = 3;
  NODE_TYPE_FUSION = 4;
  NODE_TYPE_HUMAN_REVIEW = 5;
  NODE_TYPE_PARALLEL = 6;
  NODE_TYPE_JOIN = 7;
}

enum NodeStatus {
  NODE_STATUS_UNSPECIFIED = 0;
  NODE_STATUS_PENDING = 1;
  NODE_STATUS_READY = 2;
  NODE_STATUS_RUNNING = 3;
  NODE_STATUS_SUCCESS = 4;
  NODE_STATUS_FAILED = 5;
  NODE_STATUS_SKIPPED = 6;
  NODE_STATUS_TIMEOUT = 7;
  NODE_STATUS_RETRYING = 8;
}

message PlanEdge {
  string from_node = 1;
  string to_node = 2;
  string data_mapping = 3;
}

message ExpertThought {
  string phase = 1;
  string content = 2;
  int64 timestamp_ms = 3;
  repeated string referenced_knowledge = 4;
}

message NodeMetrics {
  int64 start_time_ms = 1;
  int64 end_time_ms = 2;
  int64 duration_ms = 3;
  uint32 tool_calls = 4;
  uint32 ai_calls = 5;
  int64 tokens_used = 6;
  float cost = 7;
}

// === 任务进度事件 ===
message TaskProgressEvent {
  string task_id = 1;
  string event_type = 2;        // task_started/node_started/node_completed/node_failed/progress/task_completed/task_failed
  string node_id = 3;
  string expert_id = 4;
  string expert_name = 5;
  NodeStatus node_status = 6;
  float progress = 7;
  google.protobuf.Struct data = 8;
  int64 timestamp_ms = 9;
}

// === 专家定义 ===
message ExpertDefinition {
  string expert_id = 1;
  string name = 2;
  string description = 3;
  string role = 4;
  string version = 5;
  repeated string domains = 6;
  repeated ExpertCapability capabilities = 7;
  repeated ToolBinding tools = 8;
  ExpertKnowledge knowledge = 9;
  ExpertPersonality personality = 10;
  ExpertMemoryConfig memory_config = 11;
  uint32 priority = 12;
  ExpertStatus status = 13;
  map<string, string> metadata = 14;
}

message ExpertCapability {
  string capability_id = 1;
  string name = 2;
  string description = 3;
  repeated string input_types = 4;
  repeated string output_types = 5;
  float confidence = 6;
  float proficiency = 7;
}

message ToolBinding {
  string tool_id = 1;
  string service_name = 2;
  string method = 3;
  bool async = 4;
  map<string, string> default_params = 5;
}

message ExpertKnowledge {
  repeated string graph_subgraph_refs = 1;
  repeated string document_refs = 2;
  map<string, string> custom = 3;
}

message ExpertPersonality {
  string reasoning_style = 1;   // concise/detailed/balanced
  float verbosity = 2;           // 0-1
  float conservatism = 3;        // 0-1
  map<string, string> custom = 4;
}

message ExpertMemoryConfig {
  bool enable_working_memory = 1;
  bool enable_session_memory = 2;
  bool enable_long_term_memory = 3;
  uint32 max_working_memory_items = 4;
  uint32 session_memory_ttl_hours = 5;
}

enum ExpertStatus {
  EXPERT_STATUS_UNSPECIFIED = 0;
  EXPERT_STATUS_ACTIVE = 1;
  EXPERT_STATUS_INACTIVE = 2;
  EXPERT_STATUS_MAINTENANCE = 3;
  EXPERT_STATUS_DEPRECATED = 4;
}
```

---

## 三、多协议映射

### 3.1 协议对照表

| 能力 | gRPC | JSON-RPC 2.0 | MCP | REST | WebSocket |
|------|------|---------------|-----|------|-----------|
| 创建任务 | `CreateTask` | `expert.alliance.CreateTask` | - | `POST /api/v1/expert/tasks` | - |
| 查询任务 | `GetTask` | `expert.alliance.GetTask` | - | `GET /api/v1/expert/tasks/{id}` | - |
| 任务列表 | `ListTasks` | `expert.alliance.ListTasks` | - | `GET /api/v1/expert/tasks` | - |
| 取消任务 | `CancelTask` | `expert.alliance.CancelTask` | - | `DELETE /api/v1/expert/tasks/{id}` | - |
| 执行详情 | `GetTaskExecution` | `expert.alliance.GetTaskExecution` | - | `GET /api/v1/expert/tasks/{id}/execution` | - |
| 获取结果 | `GetTaskResult` | `expert.alliance.GetTaskResult` | - | `GET /api/v1/expert/tasks/{id}/result` | - |
| 重试节点 | `RetryNode` | `expert.alliance.RetryNode` | - | `POST /api/v1/expert/tasks/{id}/nodes/{nid}/retry` | - |
| 人工干预 | `Intervene` | `expert.alliance.Intervene` | - | `POST /api/v1/expert/tasks/{id}/intervene` | - |
| 实时进度 | - | - | - | - | `WS /ws/v1/expert/tasks/{id}/progress` |
| 流式输出 | `StreamTaskOutput` (server stream) | - | - | - | `WS /ws/v1/expert/tasks/{id}/output` |
| 专家注册 | `RegisterExpert` | `expert.registry.RegisterExpert` | - | `POST /api/v1/expert/experts` | - |
| 专家列表 | `ListExperts` | `expert.registry.ListExperts` | - | `GET /api/v1/expert/experts` | - |
| 专家匹配 | `MatchExperts` | `expert.registry.MatchExperts` | - | `GET /api/v1/expert/match?task=...` | - |
| 工具列表 | `ListTools` | `expert.registry.ListTools` | `tools/list` | `GET /api/v1/expert/tools` | - |
| 调用工具 | - | - | `tools/call` | - | - |
| 资源列表 | - | - | `resources/list` | - | - |
| 读取资源 | - | - | `resources/read` | - | - |

### 3.2 JSON-RPC 映射规范

**请求格式**：
```json
{
  "jsonrpc": "2.0",
  "method": "expert.alliance.CreateTask",
  "params": {
    "title": "图谱构建",
    "description": "把CSV构建成知识图谱",
    "preference": { "mode": "AUTO" }
  },
  "id": "req-123"
}
```

**method 命名规范**：`{service_group}.{ServiceName}.{MethodName}`
- `expert.alliance.CreateTask`
- `expert.registry.MatchExperts`
- `expert.agent.ExecuteNode`

**响应格式**：
```json
{
  "jsonrpc": "2.0",
  "result": {
    "task_id": "task-123",
    "status": "PENDING",
    "created_at": "2026-08-26T10:00:00Z"
  },
  "id": "req-123"
}
```

**错误格式**：
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Invalid params",
    "data": {
      "error_code": "ERROR_CODE_TASK_NOT_FOUND",
      "request_id": "req-123",
      "details": { "task_id": "not-found" }
    }
  },
  "id": "req-123"
}
```

### 3.3 MCP 映射规范

MCP 是 JSON-RPC 的应用层，使用标准 MCP 方法名：

**标准方法**：
| MCP 方法 | 映射到 | 说明 |
|----------|--------|------|
| `initialize` | 网关内置 | 能力协商 |
| `notifications/initialized` | 网关内置 | 客户端初始化完成通知 |
| `tools/list` | `expert.registry.ListTools` | 列出可用工具 |
| `tools/call` | JSON-RPC→gRPC转码 | 调用工具（动态映射到gRPC方法） |
| `resources/list` | `expert.kg.ListResources` | 列出可用资源 |
| `resources/read` | `expert.kg.ReadResource` | 读取资源 |
| `prompts/list` | 网关内置 | 列出提示词模板 |
| `prompts/get` | 网关内置 | 获取提示词 |
| `logging/message` | 网关内置 | 日志通知 |

**tools/call 动态映射**：
```
MCP tools/call params.name = "graph.create_vertex"
    → 查工具注册表 → gRPC service=graph.VertexService, method=CreateVertex
    → JSON-RPC→gRPC转码 → 调用后端
    → 结果包装为 MCP CallToolResult
```

### 3.4 REST 映射规范

**RESTful 资源命名**：
- 集合：`/api/v1/expert/tasks`（复数）
- 单个：`/api/v1/expert/tasks/{task_id}`
- 子资源：`/api/v1/expert/tasks/{task_id}/nodes/{node_id}`
- 动作：`/api/v1/expert/tasks/{task_id}:cancel`（冒号动作）

**HTTP 方法**：
| 方法 | 用途 | 幂等 |
|------|------|------|
| GET | 查询 | 是 |
| POST | 创建/动作 | 否（需 idempotency_key） |
| PUT | 全量更新 | 是 |
| PATCH | 部分更新 | 否 |
| DELETE | 删除/取消 | 是 |

**统一响应格式**：
```json
{
  "code": 0,
  "message": "success",
  "data": { ... },
  "request_id": "req-123",
  "timestamp": "2026-08-26T10:00:00Z"
}
```

**错误响应（RFC 9457 Problem+JSON）**：
```json
{
  "type": "about:blank",
  "title": "Not Found",
  "status": 404,
  "detail": "任务 task-123 不存在",
  "instance": "/api/v1/expert/tasks/task-123",
  "code": "ERROR_CODE_TASK_NOT_FOUND",
  "request_id": "req-123"
}
```

---

## 四、协议转码机制

### 4.1 转码路由表（从 .proto 自动生成）

```rust
// build.rs 自动生成的转码路由表
pub struct GrpcRoute {
    pub jsonrpc_method: &'static str,   // "expert.alliance.CreateTask"
    pub rest_method: &'static str,       // "POST"
    pub rest_path: &'static str,         // "/api/v1/expert/tasks"
    pub grpc_service: &'static str,      // "mox.expert.alliance.v1.ExpertAllianceService"
    pub grpc_method: &'static str,       // "CreateTask"
    pub request_type: &'static str,      // "CreateTaskRequest"
    pub response_type: &'static str,     // "CreateTaskResponse"
}

pub const GRPC_ROUTES: &[GrpcRoute] = &[
    GrpcRoute {
        jsonrpc_method: "expert.alliance.CreateTask",
        rest_method: "POST",
        rest_path: "/api/v1/expert/tasks",
        grpc_service: "mox.expert.alliance.v1.ExpertAllianceService",
        grpc_method: "CreateTask",
        request_type: "CreateTaskRequest",
        response_type: "CreateTaskResponse",
    },
    // ... 所有方法自动生成
];
```

### 4.2 转码流程

```
JSON-RPC / REST 请求
    │
    ▼
1. 路由匹配：method/path → GrpcRoute
    │
    ▼
2. 参数提取：
   - JSON-RPC: params 对象
   - REST: path params + query + body
    │
    ▼
3. JSON → Protobuf：
   serde_json::Value → prost::Message（通过 prost-reflect 或预生成代码）
    │
    ▼
4. 构造 gRPC 请求：
   - 添加 RequestMeta 到 metadata
   - 设置 deadline
    │
    ▼
5. 调用 gRPC 后端（tonic 客户端，带拦截器链）
    │
    ▼
6. Protobuf → JSON：
   prost::Message → serde_json::Value
    │
    ▼
7. 包装为协议响应：
   - JSON-RPC: { jsonrpc, result, id }
   - REST: { code, message, data, request_id }
   - MCP: { content: [{type:text, text}], isError }
```

### 4.3 错误码映射

| gRPC Status | JSON-RPC code | HTTP Status | MCP isError |
|-------------|---------------|-------------|-------------|
| OK | - | 200 | false |
| CANCELLED | -32800 | 499 | true |
| UNKNOWN | -32603 | 500 | true |
| INVALID_ARGUMENT | -32602 | 400 | true |
| DEADLINE_EXCEEDED | -32002 | 504 | true |
| NOT_FOUND | -32601 | 404 | true |
| ALREADY_EXISTS | -32009 | 409 | true |
| PERMISSION_DENIED | -32003 | 403 | true |
| RESOURCE_EXHAUSTED | -32004 | 429 | true |
| FAILED_PRECONDITION | -32010 | 400 | true |
| ABORTED | -32011 | 409 | true |
| OUT_OF_RANGE | -32012 | 400 | true |
| UNIMPLEMENTED | -32601 | 501 | true |
| INTERNAL | -32603 | 500 | true |
| UNAVAILABLE | -32001 | 503 | true |
| DATA_LOSS | -32013 | 500 | true |
| UNAUTHENTICATED | -32005 | 401 | true |

---

## 五、WebSocket 协议

### 5.1 任务进度推送

**连接**：`WS /ws/v1/expert/tasks/{task_id}/progress?token=<jwt>`

**服务端推送消息**：
```json
{
  "type": "task_progress",
  "task_id": "task-123",
  "event_type": "node_completed",
  "node_id": "node-1",
  "expert_id": "expert-graph-builder",
  "expert_name": "图谱构建专家",
  "node_status": "SUCCESS",
  "progress": 65.0,
  "data": { "output_summary": "创建了100个顶点, 200条边" },
  "timestamp_ms": 1724656800000
}
```

**event_type 枚举**：
- `task_started` / `task_planning` / `task_planned` / `task_running`
- `node_started` / `node_completed` / `node_failed` / `node_retrying`
- `fusion_started` / `fusion_completed`
- `task_paused` / `task_resumed`
- `task_completed` / `task_failed` / `task_cancelled`

### 5.2 流式输出推送

**连接**：`WS /ws/v1/expert/tasks/{task_id}/output?token=<jwt>`

**服务端推送消息**：
```json
{
  "type": "output_chunk",
  "task_id": "task-123",
  "node_id": "node-2",
  "expert_id": "expert-ai",
  "content_type": "text",
  "delta": "图谱构建完成。",
  "is_final": false,
  "timestamp_ms": 1724656800000
}
```

---

## 六、接口版本管理

### 6.1 版本策略

| 版本类型 | 说明 | 兼容性 |
|----------|------|--------|
| **主版本** | package 名 `v1`/`v2`，path 前缀 `/v1`/`v2` | 不兼容 |
| **次版本** | 新增方法/字段，不修改已有 | 向后兼容 |
| **补丁** | 修复bug，文档更新 | 完全兼容 |

### 6.2 废弃策略

1. 标记 `@deprecated`（proto 注解 + 文档）
2. 保留至少 2 个主版本周期
3. 日志记录废弃接口调用
4. 到期后移除

---

*下一篇：[05-数据架构](./05-data-architecture.md)*
