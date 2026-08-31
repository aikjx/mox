---
title: 专家注册中心与协作协议
version: V1.0
authority: 🟡参考
doc_id: EA-DOC-004
last_updated: 2026-08-31
source_of_truth: 参考
---

# 专家注册中心与协作协议

> 版本：v1.0 | 日期：2026-08-26
>
> 前置：[专家联盟总览](docs/expert-alliance/README.md)

---

## 一、专家注册中心

### 1.1 架构

```
┌─────────────────────────────────────────────────────┐
│              mox-expert-registry-svc                  │
│                                                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │ 专家注册API  │  │ 专家发现API  │  │ 健康检查器   │ │
│  │ (CRUD)      │  │ (匹配/搜索)  │  │ (心跳/状态)  │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘ │
│         │                  │                  │        │
│         └──────────────────┼──────────────────┘        │
│                            ▼                             │
│  ┌─────────────────────────────────────────────────┐   │
│  │              专家注册表（PostgreSQL）              │   │
│  │  experts / capabilities / tools / domains /      │   │
│  │  expert_capabilities / expert_domains / health   │   │
│  └─────────────────────────────────────────────────┘   │
│                            │                             │
│                            ▼ 事件同步
│  ┌─────────────────────────────────────────────────┐   │
│  │           专家图谱（mox-graph-storage）            │   │
│  │  Expert/Capability/Domain/Tool 节点 + 关联边      │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

### 1.2 数据模型

```sql
-- 专家表
CREATE TABLE experts (
    expert_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL,              -- 租户ID（系统专家为 system）
    name            VARCHAR(128) NOT NULL,
    description     TEXT,
    role            VARCHAR(64) NOT NULL,       -- analyst/builder/auditor/coordinator...
    version         VARCHAR(32) DEFAULT '1.0.0',
    priority        INT DEFAULT 5,               -- 1-10，越高越优先
    status          VARCHAR(32) DEFAULT 'active', -- active/inactive/maintenance
    personality     JSONB,                       -- 性格/推理风格配置
    memory_config   JSONB,                       -- 记忆配置
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(tenant_id, name)
);

-- 能力表
CREATE TABLE capabilities (
    capability_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(128) NOT NULL UNIQUE,
    description     TEXT,
    input_types     JSONB,                       -- ["text", "table", "graph"]
    output_types    JSONB,
    confidence      FLOAT DEFAULT 0.8,
    requires_expertise JSONB,                    -- 前置专业知识
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- 工具表
CREATE TABLE tools (
    tool_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(128) NOT NULL UNIQUE,
    service_name    VARCHAR(128) NOT NULL,       -- 对应的微服务
    method          VARCHAR(256) NOT NULL,       -- gRPC 方法全限定名
    async           BOOLEAN DEFAULT FALSE,
    parameters      JSONB,                        -- 参数 schema
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- 领域表（树形结构）
CREATE TABLE domains (
    domain_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(128) NOT NULL UNIQUE,
    description     TEXT,
    parent_domain_id UUID REFERENCES domains(domain_id),
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- 专家-能力关联
CREATE TABLE expert_capabilities (
    expert_id       UUID REFERENCES experts(expert_id) ON DELETE CASCADE,
    capability_id   UUID REFERENCES capabilities(capability_id) ON DELETE CASCADE,
    proficiency     FLOAT DEFAULT 0.8,           -- 熟练度 0-1
    acquired_at     TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY(expert_id, capability_id)
);

-- 专家-领域关联
CREATE TABLE expert_domains (
    expert_id       UUID REFERENCES experts(expert_id) ON DELETE CASCADE,
    domain_id       UUID REFERENCES domains(domain_id) ON DELETE CASCADE,
    expertise_level VARCHAR(32) DEFAULT 'intermediate', -- beginner/intermediate/expert/master
    PRIMARY KEY(expert_id, domain_id)
);

-- 能力-工具关联
CREATE TABLE capability_tools (
    capability_id   UUID REFERENCES capabilities(capability_id) ON DELETE CASCADE,
    tool_id         UUID REFERENCES tools(tool_id) ON DELETE CASCADE,
    mandatory       BOOLEAN DEFAULT TRUE,
    default_params  JSONB,
    PRIMARY KEY(capability_id, tool_id)
);

-- 专家健康状态
CREATE TABLE expert_health (
    expert_id       UUID PRIMARY KEY REFERENCES experts(expert_id) ON DELETE CASCADE,
    last_heartbeat  TIMESTAMPTZ,
    status          VARCHAR(32) DEFAULT 'healthy', -- healthy/unhealthy/degraded
    error_count     INT DEFAULT 0,
    success_rate    FLOAT DEFAULT 1.0,
    avg_latency_ms  FLOAT DEFAULT 0,
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

-- 索引
CREATE INDEX idx_experts_tenant ON experts(tenant_id);
CREATE INDEX idx_experts_status ON experts(status);
CREATE INDEX idx_expert_cap_expert ON expert_capabilities(expert_id);
CREATE INDEX idx_expert_domains_expert ON expert_domains(expert_id);
```

### 1.3 专家匹配算法

```rust
// 专家匹配评分
pub struct ExpertMatchResult {
    pub expert_id: String,
    pub expert_name: String,
    pub score: f64,               // 综合评分 0-1
    pub matched_capabilities: Vec<String>,
    pub matched_domains: Vec<String>,
    pub health_status: String,
}

pub async fn match_experts(
    &self,
    task: &TaskDescription,
    top_k: usize,
) -> Result<Vec<ExpertMatchResult>, RegistryError> {
    // 1. 从任务描述提取领域标签和能力需求
    let required_domains = extract_domains(task);
    let required_capabilities = extract_capabilities(task);
    let input_type = &task.input_type;

    // 2. 查询候选专家（基本过滤：状态active + 领域匹配）
    let candidates = self.db.query_candidates(&required_domains).await?;

    // 3. 对每个候选专家评分
    let mut results: Vec<ExpertMatchResult> = Vec::new();
    for expert in candidates {
        let mut score = 0.0;
        let mut matched_caps = Vec::new();
        let mut matched_doms = Vec::new();

        // 3.1 领域匹配分（权重 0.3）
        let domain_score = self.calculate_domain_score(
            &expert, &required_domains, &mut matched_doms
        );
        score += domain_score * 0.3;

        // 3.2 能力匹配分（权重 0.4）
        let capability_score = self.calculate_capability_score(
            &expert, &required_capabilities, input_type, &mut matched_caps
        );
        score += capability_score * 0.4;

        // 3.3 健康状态分（权重 0.15）
        let health_score = match expert.health_status.as_str() {
            "healthy" => 1.0,
            "degraded" => 0.5,
            _ => 0.0,
        };
        score += health_score * 0.15;

        // 3.4 历史表现分（权重 0.15）
        let performance_score = expert.success_rate * (1.0 - expert.error_rate);
        score += performance_score * 0.15;

        // 3.5 优先级加成
        score += (expert.priority as f64 / 10.0) * 0.05;

        results.push(ExpertMatchResult {
            expert_id: expert.expert_id,
            expert_name: expert.name,
            score,
            matched_capabilities: matched_caps,
            matched_domains: matched_doms,
            health_status: expert.health_status,
        });
    }

    // 4. 排序并返回 Top K
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results.truncate(top_k);

    Ok(results)
}
```

### 1.4 专家注册流程

```
开发者提交专家定义（JSON/YAML/Proto）
    │
    ▼
注册中心验证
    ├── 基本格式验证（必填字段/类型）
    ├── 工具存在性验证（tools 中的 service_name+method 是否存在）
    ├── 能力完整性验证（capability 是否有对应的 tool）
    ├── 命名冲突检查（同租户下 name 唯一）
    └── 图谱 Schema 验证（领域/能力是否在本体中定义）
    │
    ▼ 验证通过
写入 PostgreSQL（experts + 关联表）
    │
    ▼
同步写入知识图谱
    ├── 创建/更新 Expert 节点
    ├── 创建 has_capability 边 → Capability 节点
    ├── 创建 operates_in 边 → Domain 节点
    └── Capability 节点的 requires_tool 边已存在
    │
    ▼
发布事件 expert.registry.expert.registered
    │
    ▼
联盟调度器收到事件 → 更新本地专家缓存 → 新专家可参与协作
```

---

## 二、协作协议

### 2.1 任务协议

```protobuf
// 任务描述
message TaskDescription {
  string task_id = 1;
  string tenant_id = 2;
  string user_id = 3;
  string title = 4;
  string description = 5;                    // 自然语言描述
  TaskType type = 6;                         // analysis/building/audit/automation/custom
  repeated string domains = 7;               // 目标领域
  repeated string required_capabilities = 8; // 所需能力
  string input_type = 9;                     // 输入类型
  string output_type = 10;                   // 期望输出类型
  repeated DataReference inputs = 11;        // 输入数据引用
  map<string, string> constraints = 12;      // 约束（超时/预算/质量要求）
  CollaborationPreference preference = 13;   // 协作偏好
  string parent_task_id = 14;                // 父任务（子任务用）
}

message DataReference {
  string ref_id = 1;
  string data_type = 2;                      // file/table/graph/text/url
  string location = 3;                       // 存储位置/URL
  string schema = 4;                          // schema 描述
  int64 size_bytes = 5;
  map<string, string> metadata = 6;
}

message CollaborationPreference {
  CollaborationMode mode = 1;                // auto/serial/parallel/debate/hierarchical
  repeated string preferred_experts = 2;     // 指定专家
  repeated string excluded_experts = 3;      // 排除专家
  FusionStrategy fusion_strategy = 4;        // 指定融合策略
  int32 max_experts = 5;                     // 最大专家数
  bool human_in_the_loop = 6;                // 是否需要人工审核节点
  TimeoutConfig global_timeout = 7;
}
```

### 2.2 执行协议

```protobuf
// 节点执行请求
message ExecuteNodeRequest {
  string task_id = 1;
  string node_id = 2;
  string expert_id = 3;
  repeated NodeInput inputs = 4;             // 上游节点的输出
  map<string, string> config = 5;            // 节点配置
  TenantContext tenant_context = 6;
  string trace_id = 7;
}

message NodeInput {
  string from_node = 1;
  string output_key = 2;
  oneof value {
    string text_value = 3;
    bytes binary_value = 4;
    google.protobuf.Struct json_value = 5;
    DataReference data_ref = 6;
  }
  string content_type = 7;
}

// 节点执行响应
message ExecuteNodeResponse {
  string node_id = 1;
  NodeStatus status = 2;                      // success/failed/timeout
  repeated NodeOutput outputs = 3;
  NodeExecutionMetrics metrics = 4;
  string error_message = 5;
  repeated ExpertThought thoughts = 6;        // 专家推理过程（可解释性）
}

message NodeOutput {
  string key = 1;
  oneof value {
    string text_value = 2;
    bytes binary_value = 3;
    google.protobuf.Struct json_value = 4;
    DataReference data_ref = 5;
  }
  string content_type = 6;
  string description = 7;
}

message NodeExecutionMetrics {
  int64 start_time_ms = 1;
  int64 end_time_ms = 2;
  int64 duration_ms = 3;
  int32 tool_calls = 4;
  int32 ai_calls = 5;
  int64 tokens_used = 6;
  float cost = 7;
}

message ExpertThought {
  string phase = 1;                           // understand/plan/execute/review
  string content = 2;
  int64 timestamp_ms = 3;
  repeated string referenced_knowledge = 4;   // 引用的知识图谱节点
}
```

### 2.3 流式协议

```protobuf
// 流式执行（AI生成/长任务）
service ExpertAgentService {
  rpc StreamExecute(StreamExecuteRequest) returns (stream StreamExecuteResponse);
}

message StreamExecuteRequest {
  string task_id = 1;
  string node_id = 2;
  string expert_id = 3;
  repeated NodeInput inputs = 4;
  map<string, string> config = 5;
}

message StreamExecuteResponse {
  string event_type = 1;                      // thought/tool_start/tool_end/token/delta/error/complete
  string node_id = 2;
  oneof payload {
    ExpertThought thought = 3;                 // 专家思考
    ToolCallStart tool_start = 4;              // 工具调用开始
    ToolCallEnd tool_end = 5;                  // 工具调用结束
    string token_delta = 6;                    // AI 生成 token 增量
    NodeOutput partial_output = 7;             // 部分输出
    string error = 8;                          // 错误
    NodeExecutionMetrics metrics = 9;          // 最终指标
  }
  int64 timestamp_ms = 10;
}
```

### 2.4 事件协议

所有协作事件通过 NATS JetStream 发布，格式遵循 CloudEvent：

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
    "node_name": "数据分析专家",
    "status": "completed",
    "progress": 65,
    "timestamp": "2026-08-26T10:00:00Z"
  }
}
```

**事件主题清单**：

| 主题 | 说明 | 数据 |
|------|------|------|
| `expert.alliance.task.created` | 任务创建 | TaskDescription |
| `expert.alliance.task.progress` | 任务进度 | {task_id, node_id, status, progress} |
| `expert.alliance.task.completed` | 任务完成 | {task_id, result_summary, metrics} |
| `expert.alliance.task.failed` | 任务失败 | {task_id, error, failed_node} |
| `expert.alliance.node.started` | 节点开始 | {task_id, node_id, expert_id} |
| `expert.alliance.node.completed` | 节点完成 | {task_id, node_id, outputs, metrics} |
| `expert.alliance.node.failed` | 节点失败 | {task_id, node_id, error} |
| `expert.alliance.case.created` | 新案例 | {case_id, task_id, rating} |

---

## 三、专家 Agent 运行时

### 3.1 Agent 架构

```
┌─────────────────────────────────────────────────────┐
│              mox-expert-agent-svc                     │
│                                                       │
│  ┌─────────────────────────────────────────────┐     │
│  │              Agent 实例（每任务一个）           │     │
│  │                                               │     │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────────┐ │     │
│  │  │ 理解模块  │  │ 规划模块  │  │  执行模块    │ │     │
│  │  │(任务解析) │  │(步骤规划) │  │(工具/AI调用)│ │     │
│  │  └─────────┘  └─────────┘  └──────┬──────┘ │     │
│  │                                      │        │     │
│  │  ┌─────────┐  ┌─────────┐          │        │     │
│  │  │ 记忆模块  │  │ 审核模块  │◄─────────┘        │     │
│  │  │(短期/长期)│  │(结果检查) │                   │     │
│  │  └─────────┘  └─────────┘                   │     │
│  └─────────────────────────────────────────────┘     │
│                                                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │ 工具调用器   │  │ AI 调用器   │  │ 知识检索器   │ │
│  │ (gRPC客户端) │  │ (mox-ai)   │  │ (mox-graph) │ │
│  └─────────────┘  └─────────────┘  └─────────────┘ │
└─────────────────────────────────────────────────────┘
```

### 3.2 Agent 执行循环（ReAct 模式）

```
输入任务描述
    │
    ▼
┌──────────┐
│ 理解任务  │  解析任务目标/约束/输入
└────┬─────┘
     │
     ▼
┌──────────┐
│ 规划步骤  │  生成执行计划（可能多步）
└────┬─────┘
     │
     ▼
┌──────────┐     ┌──────────────┐
│ 执行步骤  │────→│ 选择工具/AI  │
└────┬─────┘     └──────┬───────┘
     │                    │
     │                    ▼
     │             ┌──────────────┐
     │             │ 调用工具/AI  │
     │             └──────┬───────┘
     │                    │
     │                    ▼
     │             ┌──────────────┐
     │             │ 观察结果      │
     │             └──────┬───────┘
     │                    │
     └────────────────────┘
              │
              ▼
       ┌──────────────┐
       │ 审核结果      │  是否满足目标？是否需要继续？
       └──────┬───────┘
              │
        ┌─────┴─────┐
        │           │
   不满足目标    满足目标
        │           │
        ▼           ▼
   继续循环     输出最终结果
   (更新记忆)   (写入协作记忆)
```

### 3.3 工具调用规范

每个专家可调用的工具通过 `ToolBinding` 声明，运行时统一通过 gRPC 调用：

```rust
// libs/mox-expert-core/src/tool_executor.rs

pub struct ToolExecutor {
    grpc_clients: HashMap<String, GrpcClient>,  // service_name → client
}

impl ToolExecutor {
    pub async fn execute(
        &self,
        tool: &ToolBinding,
        params: serde_json::Value,
        tenant_context: &TenantContext,
    ) -> Result<ToolResult, ToolError> {
        // 1. 参数校验（对照 tool.parameters schema）
        let validated_params = validate_params(&tool.parameters, &params)?;

        // 2. 构造 gRPC 请求
        let request = build_grpc_request(&tool.method, validated_params, tenant_context)?;

        // 3. 调用（带超时/重试/熔断）
        let client = self.grpc_clients.get(&tool.service_name)
            .ok_or_else(|| ToolError::ServiceNotFound(tool.service_name.clone()))?;

        let response = client.unary_call(&tool.method, request)
            .timeout(tool.timeout.unwrap_or(Duration::from_secs(30)))
            .await?;

        // 4. 解析响应
        Ok(ToolResult {
            tool_id: tool.tool_id.clone(),
            success: true,
            data: response,
            duration_ms: 0,
        })
    }
}
```

---

## 四、协作记忆

### 4.1 记忆层次

| 层次 | 存储 | 生命周期 | 内容 |
|------|------|----------|------|
| **工作记忆** | Agent 实例内存 | 单次任务 | 当前任务上下文/中间结果/推理过程 |
| **会话记忆** | Redis | 任务完成后 24h | 任务执行详情/节点输出/专家思考 |
| **长期记忆** | PostgreSQL + 知识图谱 | 永久 | 历史任务/案例/专家协作统计/偏好 |

### 4.2 案例库

优秀的协作结果自动提升为案例（Case），存储在知识图谱中：

```
任务完成 → 结果评分（用户反馈/自动评估）
         → 评分 > 阈值 → 提升为 Case
         → 写入图谱：
           Case 节点
           ├── solved_by → Expert（记录贡献度）
           ├── used_capability → Capability
           ├── operates_on → Data
           └── similar_to → 历史 Case（计算相似度）
```

案例库用于：
1. **专家匹配**：相似任务 → 找到历史案例 → 推荐案例中成功的专家组合
2. **协作计划生成**：参考历史案例的协作模式
3. **结果融合**：参考历史案例的融合策略
4. **持续优化**：分析案例成功率，优化专家配置和协作策略

---

*下一篇：[知识图谱关联关系设计](docs/expert-alliance/knowledge-graph-schema.md)*
