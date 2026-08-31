# Mox 专家联盟 — 智能自动化信息知识图谱关联关系系统

> 版本：v1.0 | 日期：2026-08-26 | 状态：设计草案
>
> 基于：[微服务架构设计](docs/microservices/README.md)

---

## 一、系统定位

### 1.1 什么是专家联盟

**专家联盟（Expert Alliance）** 是一个多领域专家 Agent 自动协作系统。每个专家是一个具备领域知识、工具能力和推理能力的自治 Agent，专家之间通过知识图谱的关联关系进行信息共享、任务分发和协同推理，自动完成复杂的跨领域任务。

```
用户请求 → 联盟调度器 → 专家识别 → 协作编排 → 多专家并行/串行执行 → 结果融合 → 输出
                │
                ▼
         知识图谱关联关系（专家-领域-工具-数据-历史案例）
```

### 1.2 核心价值

| 价值 | 说明 |
|------|------|
| **跨领域协作** | 单个专家无法解决的复杂问题，多专家自动协作完成 |
| **知识图谱驱动** | 专家能力、领域知识、历史案例全部图谱化，关联关系驱动协作 |
| **智能自动化** | 自动识别所需专家、自动编排协作流程、自动融合结果 |
| **可扩展** | 新专家即插即用，自动注册到联盟，自动参与协作 |
| **可追溯** | 完整的协作链路记录，每个决策可追溯到具体专家和知识来源 |

### 1.3 与现有架构的关系

专家联盟是现有微服务架构之上的**智能编排层**，复用已有服务能力：

```
┌─────────────────────────────────────────────────────────────┐
│                    专家联盟层（新增）                           │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐   │
│  │ 联盟调度器   │  │ 专家注册中心 │  │ 协作编排引擎       │   │
│  └─────────────┘  └─────────────┘  └───────────────────┘   │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐   │
│  │ 结果融合器   │  │ 知识关联引擎 │  │ 协作记忆/案例库    │   │
│  └─────────────┘  └─────────────┘  └───────────────────┘   │
└──────────────────────────┬──────────────────────────────────┘
                           │ gRPC / 事件
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    现有微服务层                                 │
│  mox-ai-svc  mox-agent-svc  mox-graph-svc  mox-graph-storage│
│  mox-flow-svc  mox-search-svc  mox-storage-svc  mox-tenant  │
│  ...（31个服务）                                               │
└─────────────────────────────────────────────────────────────┘
```

---

## 二、架构设计

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                            接入层                                      │
│  REST API / gRPC / WebSocket（流式协作过程）                          │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         联盟调度层（mox-expert-alliance-svc）          │
│                                                                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
│  │ 任务接收与解析 │  │ 专家识别与匹配 │  │ 协作计划生成              │  │
│  │ (NLP/意图识别)│  │ (图谱关联推理) │  │ (DAG/动态编排)           │  │
│  └──────────────┘  └──────────────┘  └──────────────────────────┘  │
│                                                                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
│  │ 协作执行引擎   │  │ 结果融合器    │  │ 协作记忆管理              │  │
│  │ (并行/串行/   │  │ (投票/加权/   │  │ (上下文传递/中间结果/    │  │
│  │  条件/循环)   │  │  辩论/仲裁)   │  │  历史案例)               │  │
│  └──────────────┘  └──────────────┘  └──────────────────────────┘  │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         专家层（多个专家 Agent）                        │
│                                                                       │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ │
│  │ 图谱构建  │ │ 数据分析  │ │ AI推理   │ │ 安全审计  │ │ 流程自动化│ │
│  │ 专家     │ │ 专家     │ │ 专家     │ │ 专家     │ │ 专家     │ │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘ │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ │
│  │ 数据治理  │ │ 知识融合  │ │ 搜索推荐  │ │ 运维监控  │ │ 业务专家  │ │
│  │ 专家     │ │ 专家     │ │ 专家     │ │ 专家     │ │ (可扩展)  │ │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘ │
│                                                                       │
│  每个专家 = 角色定义 + 领域知识(图谱子图) + 工具集 + 推理能力 + 记忆  │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         能力层（现有微服务）                             │
│  mox-ai-svc  mox-graph-svc  mox-search-svc  mox-flow-svc  ...     │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 服务拆分

专家联盟在现有架构中新增以下服务：

| 服务 | 职责 | 类型 |
|------|------|------|
| **mox-expert-alliance-svc** | 联盟核心：调度/编排/执行/融合/记忆 | 核心服务（新增） |
| **mox-expert-registry-svc** | 专家注册中心：专家CRUD/能力声明/健康检查/发现 | 平台服务（新增） |
| **mox-expert-agent-svc** | 专家 Agent 运行时：专家实例管理/工具执行/推理调用 | 运行时服务（新增，复用 mox-agent-svc 扩展） |

**部署方式**：
- mox-expert-alliance-svc：无状态，Deployment，HPA 按任务队列长度扩缩
- mox-expert-registry-svc：无状态，Deployment，数据存 PostgreSQL
- mox-expert-agent-svc：有状态（专家实例/会话），StatefulSet 或 Deployment + Redis 会话

---

## 三、专家模型

### 3.1 专家定义

每个专家是一个结构化的 Agent 定义：

```rust
// proto/expert/v1/expert.proto

message ExpertDefinition {
  string expert_id = 1;                    // 专家唯一ID
  string name = 2;                          // 专家名称
  string description = 3;                   // 专家描述
  ExpertRole role = 4;                      // 角色
  repeated string domains = 5;              // 领域标签
  repeated ExpertCapability capabilities = 6; // 能力声明
  repeated ToolBinding tools = 7;           // 可调用工具
  ExpertKnowledge knowledge = 8;            // 领域知识（图谱子图引用）
  ExpertPersonality personality = 9;        // 性格/推理风格
  ExpertMemoryConfig memory = 10;           // 记忆配置
  int32 priority = 11;                      // 优先级（用于冲突仲裁）
  ExpertStatus status = 12;                 // 状态
  map<string, string> metadata = 13;        // 扩展元数据
}

message ExpertCapability {
  string capability_id = 1;
  string name = 2;
  string description = 3;
  repeated string input_types = 4;          // 可处理的输入类型
  repeated string output_types = 5;         // 可产出的输出类型
  float confidence = 6;                      // 能力置信度（0-1）
  repeated string requires_expertise = 7;   // 需要的前置专业知识
}

message ToolBinding {
  string tool_id = 1;
  string service_name = 2;                   // 对应的微服务
  string method = 3;                         // gRPC 方法
  repeated string parameters = 4;            // 参数映射
  bool async = 5;                             // 是否异步
}
```

### 3.2 内置专家清单

| 专家 | 领域 | 核心能力 | 调用服务 |
|------|------|----------|----------|
| **图谱构建专家** | 知识图谱 | 本体设计/实体抽取/关系抽取/图谱构建/质量评估 | mox-graph-svc, mox-ai-svc, mox-etl-svc |
| **数据分析专家** | 数据分析 | 数据探查/统计分析/趋势预测/异常检测/可视化建议 | mox-dataplane-svc, mox-ai-svc |
| **AI推理专家** | 人工智能 | 文本生成/摘要/翻译/分类/RAG检索/多模态理解 | mox-ai-svc, mox-search-svc |
| **安全审计专家** | 安全合规 | 权限审计/数据脱敏/合规检查/漏洞扫描/风险评估 | mox-compliance-svc, mox-auth-svc |
| **流程自动化专家** | 工作流 | 流程设计/任务编排/自动化执行/异常处理/流程优化 | mox-flow-svc, mox-operator-svc |
| **数据治理专家** | 数据治理 | 数据标准/质量规则/血缘分析/元数据管理/数据目录 | mox-catalog-svc, mox-dataplane-svc |
| **知识融合专家** | 知识融合 | 实体对齐/属性融合/冲突解决/知识补全/去重 | mox-fusion-svc, mox-graph-svc |
| **搜索推荐专家** | 搜索推荐 | 语义搜索/图谱检索/个性化推荐/相关性排序 | mox-search-svc, mox-graph-svc |
| **运维监控专家** | 运维 | 指标监控/告警分析/故障定位/容量规划/性能调优 | mox-o11y（基础设施） |
| **联盟协调专家** | 协调 | 任务分解/专家调度/冲突仲裁/结果评估/流程优化 | mox-expert-alliance-svc（自引用） |

### 3.3 专家注册与发现

```
专家开发者 → 注册专家定义（Proto/JSON）→ mox-expert-registry-svc
                                              │
                                              ├── 验证专家定义（工具存在性/能力完整性）
                                              ├── 写入专家注册表（PostgreSQL）
                                              ├── 发布专家注册事件（NATS: expert.registered）
                                              └── 更新知识图谱（专家节点 + 能力/领域/工具关联边）

联盟调度器 → 查询可用专家 → mox-expert-registry-svc
                │
                ├── 按领域/能力/输入类型匹配
                ├── 健康检查过滤（只返回活跃专家）
                └── 按优先级/置信度排序
```

---

## 四、知识图谱关联关系设计

### 4.1 专家联盟知识图谱

专家联盟的核心是一张**"专家-能力-领域-工具-数据-案例"六元关联图谱**：

```
┌─────────┐     has_capability     ┌──────────┐
│  专家    │ ─────────────────────→ │  能力    │
│ Expert   │                        │Capability│
└────┬────┘                        └────┬─────┘
     │                                    │
     │ operates_in                       │ produces
     ▼                                    ▼
┌─────────┐     requires_tool      ┌──────────┐
│  领域    │ ←───────────────────── │  工具    │
│ Domain   │                        │  Tool    │
└────┬────┘                        └────┬─────┘
     │                                    │
     │ contains_data                      │ operates_on
     ▼                                    ▼
┌─────────┐     solved_by         ┌──────────┐
│  数据    │ ←──────────────────── │  案例    │
│ Data     │                        │  Case    │
└─────────┘                        └──────────┘
```

### 4.2 本体定义（Schema）

```
# 专家联盟本体（存储在 mox-graph-meta-svc）

## 顶点类型（Vertex Types）

Expert:
  properties: expert_id, name, description, role, priority, status, version
  indexes: expert_id(unique), name, domains(multi-value)

Capability:
  properties: capability_id, name, description, confidence, input_types, output_types
  indexes: capability_id(unique), name

Domain:
  properties: domain_id, name, description, parent_domain
  indexes: domain_id(unique), name

Tool:
  properties: tool_id, name, service_name, method, async, parameters
  indexes: tool_id(unique), service_name

Data:
  properties: data_id, name, type, source, schema_ref, sensitivity
  indexes: data_id(unique), type

Case:
  properties: case_id, title, description, task_type, success_rate, rating
  indexes: case_id(unique), task_type

Task:
  properties: task_id, description, status, created_at, completed_at
  indexes: task_id(unique), status

## 边类型（Edge Types）

has_capability: Expert → Capability
  properties: proficiency(0-1), acquired_at

operates_in: Expert → Domain
  properties: expertise_level(beginner/intermediate/expert/master)

requires_tool: Capability → Tool
  properties: mandatory(bool), default_params

operates_on: Tool → Data
  properties: operation(read/write/execute)

contains_data: Domain → Data
  properties: data_category

solved_by: Case → Expert
  properties: contribution(0-1), role(primary/supporting)

used_capability: Case → Capability
  properties: effectiveness(0-1)

similar_to: Case → Case
  properties: similarity(0-1), dimensions

collaborates_with: Expert → Expert
  properties: frequency, success_rate, avg_collaboration_time

depends_on: Capability → Capability
  properties: dependency_type(prerequisite/complementary)
```

### 4.3 关联关系驱动的协作

知识图谱的关联关系是专家协作的核心驱动力：

#### 4.3.1 专家识别

```
用户任务 → 任务解析（提取领域/能力/数据需求）
         → 图谱查询：
           1. 找到匹配的 Domain 节点
           2. 通过 operates_in 找到相关 Expert
           3. 通过 has_capability 验证能力匹配
           4. 通过 requires_tool 验证工具可用性
           5. 通过 solved_by 找到历史相似案例
           6. 综合评分排序 → 推荐专家列表
```

#### 4.3.2 协作编排

```
任务分解 → 子任务依赖图（DAG）
         → 图谱推理：
           1. 每个子任务匹配最佳专家
           2. 通过 collaborates_with 找到协作历史好的专家组合
           3. 通过 depends_on 确定能力依赖链
           4. 生成协作计划（哪些专家并行/串行）
```

#### 4.3.3 结果融合

```
多专家结果 → 图谱分析：
           1. 通过 used_capability 分析各专家使用的能力
           2. 通过 solved_by 对比历史案例的成功模式
           3. 通过 similar_to 找到相似案例的融合策略
           4. 生成融合方案（投票/加权/辩论/仲裁）
```

#### 4.3.4 协作记忆

```
每次协作完成 → 写入图谱：
  1. 创建 Task 节点
  2. Task → solved_by → 参与的 Expert（记录贡献度）
  3. Task → used_capability → 使用的 Capability
  4. Task → similar_to → 历史相似 Case
  5. 更新 Expert 间的 collaborates_with 边（频率/成功率）
  6. 如果结果优秀 → 提升为 Case（案例库）
```

---

## 五、协作引擎设计

### 5.1 协作模式

| 模式 | 说明 | 适用场景 |
|------|------|----------|
| **串行（Pipeline）** | 专家A输出 → 专家B输入 → 专家C... | 数据处理流水线：抽取→清洗→融合→入库 |
| **并行（Fan-out/Fan-in）** | 多个专家同时处理，结果汇总融合 | 多视角分析：各领域专家独立分析后融合 |
| **辩论（Debate）** | 多个专家对同一问题给出不同观点，互相质询 | 决策类问题：风险评估/方案选择 |
| **分层（Hierarchical）** | 协调专家分解任务，子专家执行，协调专家汇总 | 复杂任务：联盟协调专家主导 |
| **迭代（Iterative）** | 专家A处理 → 专家B审核 → 不通过返回A重做 | 质量要求高的任务：内容生成+审核 |
| **动态（Dynamic）** | 根据中间结果动态决定下一步调用哪个专家 | 探索性任务：研究/问题排查 |

### 5.2 协作计划生成

```rust
// 协作计划（DAG）
message CollaborationPlan {
  string plan_id = 1;
  string task_id = 2;
  repeated PlanNode nodes = 3;
  repeated PlanEdge edges = 4;          // 依赖关系
  CollaborationMode mode = 5;
  FusionStrategy fusion_strategy = 6;
  int32 max_iterations = 7;
  TimeoutConfig timeout = 8;
}

message PlanNode {
  string node_id = 1;
  string expert_id = 2;
  string expert_name = 3;
  repeated string input_keys = 4;       // 从哪些上游节点获取输入
  repeated string output_keys = 5;      // 产出哪些输出
  NodeType type = 6;                     // execute/condition/loop/fusion
  map<string, string> config = 7;
  RetryPolicy retry = 8;
  TimeoutConfig timeout = 9;
}

message PlanEdge {
  string from_node = 1;
  string to_node = 2;
  string data_mapping = 3;               // 输出→输入映射
  EdgeCondition condition = 4;           // 条件边（条件节点用）
}
```

### 5.3 协作执行引擎

```
┌─────────────────────────────────────────────────────────┐
│                    协作执行引擎                             │
│                                                           │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐ │
│  │  DAG 调度器  │───→│ 节点执行器   │───→│ 状态管理器   │ │
│  │ (拓扑排序/   │    │ (调用专家/   │    │ (进度/中间   │ │
│  │  依赖检查/   │    │  工具执行/   │    │  结果/错误/  │ │
│  │  并行调度)   │    │  超时/重试)  │    │  事件)      │ │
│  └─────────────┘    └─────────────┘    └──────┬──────┘ │
│                                                  │        │
│  ┌─────────────┐    ┌─────────────┐             │        │
│  │ 事件总线     │←───│ 协作记忆     │←────────────┘        │
│  │ (NATS)      │    │ (上下文/案例) │                      │
│  └─────────────┘    └─────────────┘                      │
└─────────────────────────────────────────────────────────┘
```

**执行流程**：

1. **计划加载**：加载 CollaborationPlan，构建 DAG
2. **拓扑排序**：计算节点执行顺序，识别可并行节点
3. **节点调度**：
   - 检查所有上游依赖是否完成
   - 收集上游节点的输出作为输入
   - 调用对应专家 Agent 执行
   - 处理超时/重试/降级
4. **状态更新**：更新节点状态（pending/running/success/failed/skipped）
5. **事件发布**：每个状态变更发布事件（前端可实时订阅）
6. **完成判断**：所有节点完成或遇到不可恢复错误 → 进入结果融合

### 5.4 结果融合策略

| 策略 | 说明 | 适用 |
|------|------|------|
| **多数投票** | 各专家结果投票，取多数 | 分类/判断类任务 |
| **加权投票** | 按专家置信度/历史成功率加权 | 有专家质量差异的场景 |
| **拼接合并** | 各专家结果拼接为完整输出 | 各专家负责不同部分 |
| **择优选择** | 按评分选择最优结果 | 有明确评估标准 |
| **辩论仲裁** | 专家互相质询，协调专家仲裁 | 观点冲突的决策类任务 |
| **迭代精炼** | 一个专家生成，另一个审核修改 | 质量要求高的内容生成 |

---

## 六、与现有服务的集成

### 6.1 服务调用关系

```
mox-expert-alliance-svc
  ├──→ mox-expert-registry-svc    (专家查询/注册)
  ├──→ mox-expert-agent-svc       (专家实例执行)
  │     ├──→ mox-ai-svc           (AI推理/生成)
  │     ├──→ mox-graph-svc        (图谱查询/操作)
  │     ├──→ mox-graph-storage-svc(底层图存储)
  │     ├──→ mox-search-svc       (搜索/检索)
  │     ├──→ mox-flow-svc         (工作流执行)
  │     ├──→ mox-etl-svc          (数据处理)
  │     ├──→ mox-storage-svc      (文件存储)
  │     ├──→ mox-compliance-svc   (安全审计)
  │     ├──→ mox-fusion-svc       (数据融合)
  │     ├──→ mox-catalog-svc      (数据目录)
  │     └──→ mox-dataplane-svc    (数据路由)
  ├──→ mox-tenant-svc             (租户上下文/配额)
  ├──→ mox-auth-svc               (权限校验)
  ├──→ mox-metering-svc           (用量计量)
  └──→ mox-notification-svc       (通知)
```

### 6.2 事件通信

| 事件主题 | 发布者 | 订阅者 | 说明 |
|----------|--------|--------|------|
| `expert.alliance.task.created` | alliance-svc | 所有 | 新协作任务创建 |
| `expert.alliance.task.progress` | alliance-svc | 前端/监控 | 任务进度更新 |
| `expert.alliance.task.completed` | alliance-svc | 所有 | 任务完成 |
| `expert.alliance.task.failed` | alliance-svc | 告警/前端 | 任务失败 |
| `expert.agent.node.started` | agent-svc | alliance-svc | 节点开始执行 |
| `expert.agent.node.completed` | agent-svc | alliance-svc | 节点执行完成 |
| `expert.agent.node.failed` | agent-svc | alliance-svc | 节点执行失败 |
| `expert.agent.stream.output` | agent-svc | 前端 | 流式输出（AI生成等） |
| `expert.registry.expert.registered` | registry-svc | alliance-svc | 新专家注册 |
| `expert.registry.expert.updated` | registry-svc | alliance-svc | 专家定义更新 |
| `expert.case.created` | alliance-svc | graph-svc | 新案例入库（写入图谱） |

### 6.3 多租户

专家联盟完全复用现有多租户架构：
- 每个租户有独立的专家集合（系统内置专家所有租户共享，自定义专家租户隔离）
- 协作任务、案例库、协作记忆按租户隔离（L1 逻辑隔离，tenant_id）
- 专家调用的资源消耗计入租户配额（mox-metering-svc）

---

## 七、API 设计

### 7.1 核心 gRPC 接口

```protobuf
// proto/expert/alliance/v1/alliance.proto

service ExpertAllianceService {
  // 创建协作任务
  rpc CreateTask(CreateTaskRequest) returns (CreateTaskResponse);
  // 取消任务
  rpc CancelTask(CancelTaskRequest) returns (CancelTaskResponse);
  // 获取任务状态
  rpc GetTask(GetTaskRequest) returns (GetTaskResponse);
  // 列出任务
  rpc ListTasks(ListTasksRequest) returns (ListTasksResponse);
  // 获取任务执行详情（节点级）
  rpc GetTaskExecution(GetTaskExecutionRequest) returns (GetTaskExecutionResponse);
  // 流式订阅任务进度
  rpc SubscribeTaskProgress(SubscribeTaskProgressRequest) returns (stream TaskProgressEvent);
  // 获取协作结果
  rpc GetTaskResult(GetTaskResultRequest) returns (GetTaskResultResponse);
  // 重新执行失败节点
  rpc RetryNode(RetryNodeRequest) returns (RetryNodeResponse);
  // 人工干预（指定专家/修改计划）
  rpc Intervene(InterveneRequest) returns (InterveneResponse);
}

service ExpertRegistryService {
  rpc RegisterExpert(RegisterExpertRequest) returns (RegisterExpertResponse);
  rpc UpdateExpert(UpdateExpertRequest) returns (UpdateExpertResponse);
  rpc DeregisterExpert(DeregisterExpertRequest) returns (DeregisterExpertResponse);
  rpc GetExpert(GetExpertRequest) returns (GetExpertResponse);
  rpc ListExperts(ListExpertsRequest) returns (ListExpertsResponse);
  rpc MatchExperts(MatchExpertsRequest) returns (MatchExpertsResponse);  // 按任务匹配专家
  rpc GetExpertHealth(GetExpertHealthRequest) returns (GetExpertHealthResponse);
}

service ExpertAgentService {
  rpc ExecuteNode(ExecuteNodeRequest) returns (ExecuteNodeResponse);
  rpc StreamExecute(StreamExecuteRequest) returns (stream StreamExecuteResponse);
  rpc GetAgentState(GetAgentStateRequest) returns (GetAgentStateResponse);
}
```

### 7.2 REST API（网关转码）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/expert/tasks` | 创建协作任务 |
| GET | `/api/v1/expert/tasks` | 列出任务 |
| GET | `/api/v1/expert/tasks/{task_id}` | 获取任务详情 |
| DELETE | `/api/v1/expert/tasks/{task_id}` | 取消任务 |
| GET | `/api/v1/expert/tasks/{task_id}/execution` | 执行详情 |
| GET | `/api/v1/expert/tasks/{task_id}/result` | 协作结果 |
| POST | `/api/v1/expert/tasks/{task_id}/nodes/{node_id}/retry` | 重试节点 |
| GET | `/api/v1/expert/experts` | 列出专家 |
| GET | `/api/v1/expert/experts/{expert_id}` | 专家详情 |
| POST | `/api/v1/expert/experts` | 注册专家 |
| GET | `/api/v1/expert/match?task=...` | 匹配专家 |
| WS | `/ws/v1/expert/tasks/{task_id}/progress` | 实时进度 |

---

## 八、典型场景

### 8.1 场景一：智能图谱构建

**用户请求**："帮我把这一批 CSV 数据构建成知识图谱，并做质量评估"

**协作流程**：
```
1. 联盟调度器解析任务 → 识别领域：知识图谱构建
2. 图谱匹配专家：数据分析专家 + 图谱构建专家 + 安全审计专家
3. 生成协作计划（串行 Pipeline）：
   ┌─────────────────┐
   │ 数据分析专家     │ 探查CSV结构/数据质量/识别实体和关系
   └────────┬────────┘
            ▼
   ┌─────────────────┐
   │ 图谱构建专家     │ 本体设计/实体抽取/关系抽取/图谱构建
   └────────┬────────┘
            ▼
   ┌─────────────────┐
   │ 安全审计专家     │ 敏感数据检查/脱敏建议/合规评估
   └────────┬────────┘
            ▼
   ┌─────────────────┐
   │ 结果融合器       │ 汇总：图谱统计 + 质量报告 + 安全建议
   └─────────────────┘
4. 执行完成 → 写入协作记忆 → 优秀案例提升为 Case
```

### 8.2 场景二：跨领域智能分析

**用户请求**："分析我们的客户流失原因，并给出挽回方案"

**协作流程**：
```
1. 任务解析 → 多领域：数据分析 + AI推理 + 业务知识
2. 匹配专家：数据分析专家 + AI推理专家 + 搜索推荐专家
3. 协作计划（并行 + 融合）：
   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
   │ 数据分析专家  │  │ AI推理专家    │  │ 搜索推荐专家  │
   │ 统计分析/     │  │ 文本挖掘/     │  │ 历史案例/     │
   │ 趋势/异常     │  │ 归因分析      │  │ 相似客户      │
   └──────┬───────┘  └──────┬───────┘  └──────┬───────┘
          │                   │                   │
          └───────────────────┼───────────────────┘
                              ▼
                   ┌─────────────────────┐
                   │ 结果融合（加权投票）  │
                   │ 综合三方结果生成报告  │
                   └──────────┬──────────┘
                              ▼
                   ┌─────────────────────┐
                   │ AI推理专家（二次）    │
                   │ 基于分析结果生成挽回方案│
                   └─────────────────────┘
```

### 8.3 场景三：自动化数据治理

**用户请求**："对新接入的数据源做完整的数据治理流程"

**协作流程**：
```
1. 任务解析 → 数据治理全流程
2. 匹配专家：数据治理专家 + 安全审计专家 + 知识融合专家
3. 协作计划（分层，联盟协调专家主导）：
   联盟协调专家
   ├── 数据治理专家：数据标准检查/质量规则定义/元数据提取
   ├── 安全审计专家：敏感数据识别/脱敏/权限建议
   ├── 知识融合专家：与现有数据实体对齐/去重/融合
   └── 联盟协调专家：汇总治理报告 + 生成治理工作流（写入 mox-flow-svc）
```

---

## 九、实施路线

### 9.1 与微服务路线图的集成

专家联盟在微服务架构的**阶段三（服务化推进）**之后启动，作为**阶段四（企业级增强）**的智能层：

| 阶段 | 时间 | 专家联盟相关工作 |
|------|------|-----------------|
| 阶段一 | W1-4 | 共享库建设中预留专家联盟相关库（mox-expert-core） |
| 阶段二 | W5-10 | mox-agent-svc 拆分时预留专家 Agent 扩展点 |
| 阶段三 | W11-16 | 所有服务拆分完成，专家联盟依赖的基础服务就绪 |
| **阶段四** | **W17-20** | **专家联盟开发（核心功能）** |
| 阶段五 | W21+ | 专家联盟持续优化/新专家扩展/案例库积累 |

### 9.2 专家联盟开发计划（4周）

| 周 | 任务 | 交付物 |
|----|------|--------|
| W17 | 专家注册中心 + 专家定义 Proto + 内置专家定义 | mox-expert-registry-svc + 10个内置专家定义 |
| W17 | 专家联盟知识图谱本体设计 + 图谱初始化 | 本体 Schema + 专家/能力/领域/工具节点 |
| W18 | 联盟调度器：任务解析 + 专家匹配（图谱推理） | mox-expert-alliance-svc 核心调度 |
| W18 | 协作计划生成器：DAG 生成 + 6种协作模式 | 计划生成引擎 |
| W19 | 协作执行引擎：DAG 调度 + 节点执行 + 状态管理 | 执行引擎 + 事件发布 |
| W19 | 专家 Agent 运行时：工具调用 + AI推理 + 记忆 | mox-expert-agent-svc |
| W20 | 结果融合器：6种融合策略 | 融合引擎 |
| W20 | 协作记忆 + 案例库 + 前端实时进度（WS） | 记忆/案例 + 前端集成 |
| W20 | 3个典型场景验证 + 压测 + Bug修复 | 场景验证报告 |

---

## 十、总结

**Mox 专家联盟**是构建在微服务架构之上的智能编排层，核心设计理念：

1. **知识图谱驱动协作**：专家-能力-领域-工具-数据-案例六元关联图谱，用关联关系驱动专家识别、协作编排、结果融合和记忆积累
2. **多专家自动协作**：6种协作模式（串行/并行/辩论/分层/迭代/动态），自动编排、自动执行、自动融合
3. **即插即用的专家体系**：10个内置专家 + 可扩展的自定义专家，注册中心统一管理
4. **完全复用现有架构**：3个新增服务（alliance/registry/agent），复用全部31个现有微服务的能力
5. **可追溯的协作过程**：完整的执行链路记录、流式进度推送、协作记忆和案例库

专家联盟让系统从"工具集合"升级为"智能团队"——用户只需描述目标，系统自动组织合适的专家、编排协作流程、融合多方结果，最终交付高质量的复杂任务成果。

---

*相关文档：[微服务架构设计](docs/microservices/README.md) | [服务边界优化](docs/microservices/01-service-boundaries.md) | [通信架构优化](docs/microservices/02-communication.md)*
