---
title: 知识图谱关联关系设计
version: V1.0
authority: 🟡参考
doc_id: EA-DOC-005
last_updated: 2026-08-31
source_of_truth: 参考
---

# 知识图谱关联关系设计

> 版本：v1.0 | 日期：2026-08-26
>
> 前置：[专家联盟总览](docs/expert-alliance/README.md) | [专家注册与协作协议](docs/expert-alliance/expert-registry-and-protocol.md)

---

## 一、设计理念

专家联盟的知识图谱不是普通的"专家信息表"，而是一张**六元关联网络**，通过节点间的关联关系驱动整个协作系统的智能决策：

```
关联关系 = 系统的"神经连接"
  ├── 专家识别：通过 operates_in / has_capability 找到合适的专家
  ├── 协作编排：通过 collaborates_with / depends_on 找到最佳组合
  ├── 结果融合：通过 used_capability / solved_by 评估专家贡献
  └── 持续学习：每次协作更新关联边的权重（频率/成功率/效果）
```

---

## 二、本体定义（Ontology）

### 2.1 顶点类型（7种）

| 类型 | 标识 | 说明 | 核心属性 |
|------|------|------|----------|
| **专家** | `Expert` | 领域专家 Agent | expert_id, name, role, priority, status |
| **能力** | `Capability` | 专家具备的能力 | capability_id, name, input_types, output_types, confidence |
| **领域** | `Domain` | 知识/业务领域 | domain_id, name, parent_domain |
| **工具** | `Tool` | 可调用的微服务方法 | tool_id, service_name, method, async |
| **数据** | `Data` | 数据源/数据集 | data_id, type, source, sensitivity |
| **案例** | `Case` | 历史成功协作案例 | case_id, title, task_type, success_rate, rating |
| **任务** | `Task` | 协作任务实例 | task_id, status, created_at, completed_at |

### 2.2 边类型（12种）

| 边类型 | 起点 → 终点 | 说明 | 权重属性 |
|--------|------------|------|----------|
| `has_capability` | Expert → Capability | 专家具备某能力 | proficiency(0-1) |
| `operates_in` | Expert → Domain | 专家活跃于某领域 | expertise_level |
| `requires_tool` | Capability → Tool | 能力需要某工具 | mandatory(bool) |
| `operates_on` | Tool → Data | 工具操作某数据 | operation(r/w/x) |
| `contains_data` | Domain → Data | 领域包含某数据 | data_category |
| `solved_by` | Case/Task → Expert | 案例/任务由某专家解决 | contribution(0-1), role |
| `used_capability` | Case/Task → Capability | 案例/任务使用了某能力 | effectiveness(0-1) |
| `similar_to` | Case → Case | 案例间相似 | similarity(0-1), dimensions |
| `collaborates_with` | Expert → Expert | 专家间协作历史 | frequency, success_rate, avg_time |
| `depends_on` | Capability → Capability | 能力间依赖 | dependency_type |
| `subdomain_of` | Domain → Domain | 领域父子关系 | - |
| `executed_by` | Task → Expert | 任务由某专家执行（运行时） | node_id, status |

---

## 三、详细 Schema

### 3.1 Expert 顶点

```
VertexType: Expert
Description: 专家 Agent 定义

Properties:
  - expert_id:    string, unique, indexed    // 专家唯一ID
  - name:         string, indexed             // 专家名称
  - description:  text                         // 专家描述
  - role:         string, indexed             // 角色: analyst/builder/auditor/coordinator/...
  - version:      string                       // 版本号
  - priority:     int, indexed                // 优先级 1-10
  - status:       string, indexed             // active/inactive/maintenance
  - tenant_id:    string, indexed             // 租户ID（系统专家为 "system"）
  - created_at:   datetime
  - updated_at:   datetime

Indexes:
  - expert_id (unique)
  - (tenant_id, name) (unique)
  - status
  - role
```

### 3.2 Capability 顶点

```
VertexType: Capability
Description: 能力定义

Properties:
  - capability_id:      string, unique, indexed
  - name:               string, indexed
  - description:        text
  - input_types:        string[]     // ["text", "table", "graph", "image"]
  - output_types:       string[]
  - confidence:         float         // 能力本身的置信度 0-1
  - requires_expertise: string[]      // 前置专业知识标签
  - category:           string, indexed // reasoning/processing/analysis/generation/...

Indexes:
  - capability_id (unique)
  - name
  - category
```

### 3.3 Domain 顶点

```
VertexType: Domain
Description: 领域定义（树形）

Properties:
  - domain_id:   string, unique, indexed
  - name:        string, indexed
  - description: text
  - level:       int            // 层级（根=0）
  - path:        string         // 物化路径 "/root/child/grandchild"

Indexes:
  - domain_id (unique)
  - name
  - path
```

### 3.4 Tool 顶点

```
VertexType: Tool
Description: 工具定义（对应微服务 gRPC 方法）

Properties:
  - tool_id:      string, unique, indexed
  - name:         string, indexed
  - description:  text
  - service_name: string, indexed    // 对应的微服务名
  - method:       string             // gRPC 方法全限定名
  - async:        bool
  - parameters:   json               // 参数 schema
  - category:     string, indexed    // ai/graph/data/storage/flow/...

Indexes:
  - tool_id (unique)
  - (service_name, method) (unique)
  - category
```

### 3.5 Data 顶点

```
VertexType: Data
Description: 数据源/数据集

Properties:
  - data_id:     string, unique, indexed
  - name:        string, indexed
  - type:        string, indexed    // file/table/graph/text/api/stream
  - source:      string             // 来源系统
  - schema_ref:  string             // schema 引用
  - sensitivity: string, indexed    // public/internal/confidential/restricted
  - format:      string             // csv/json/parquet/...

Indexes:
  - data_id (unique)
  - type
  - sensitivity
```

### 3.6 Case 顶点

```
VertexType: Case
Description: 历史成功协作案例

Properties:
  - case_id:      string, unique, indexed
  - title:        string, indexed
  - description:  text
  - task_type:    string, indexed    // analysis/building/audit/automation
  - input_summary: text
  - output_summary: text
  - success_rate: float               // 历史复现成功率
  - rating:       float               // 用户评分 0-5
  - execution_time_ms: int            // 典型执行时间
  - expert_count: int                 // 参与专家数
  - created_at:   datetime, indexed
  - tenant_id:    string, indexed

Indexes:
  - case_id (unique)
  - task_type
  - (tenant_id, created_at)
  - rating
```

### 3.7 Task 顶点

```
VertexType: Task
Description: 协作任务实例（运行时数据，TTL 自动清理）

Properties:
  - task_id:      string, unique, indexed
  - title:        string
  - description:  text
  - status:       string, indexed    // pending/running/completed/failed/cancelled
  - tenant_id:    string, indexed
  - user_id:      string
  - created_at:   datetime, indexed
  - completed_at: datetime
  - duration_ms:  int
  - total_cost:   float
  - ttl:          datetime           // 过期时间（过期后归档或删除）

Indexes:
  - task_id (unique)
  - (tenant_id, status)
  - (tenant_id, created_at)
```

---

## 四、关联关系详细定义

### 4.1 has_capability（专家 → 能力）

```
EdgeType: has_capability
From: Expert
To: Capability
Description: 专家具备某项能力

Properties:
  - proficiency:  float    // 熟练度 0-1
  - acquired_at:  datetime // 获得时间
  - usage_count:  int      // 使用次数（统计）
  - success_rate: float    // 使用成功率（统计）

Cardinality: Many-to-Many

Usage:
  - 专家匹配：通过任务所需能力找到具备该能力的专家
  - 能力评估：proficiency + success_rate 综合评估专家在该能力上的水平
  - 持续学习：每次使用该能力完成任务后更新 usage_count 和 success_rate
```

### 4.2 operates_in（专家 → 领域）

```
EdgeType: operates_in
From: Expert
To: Domain
Description: 专家活跃于某个领域

Properties:
  - expertise_level: string  // beginner/intermediate/expert/master
  - task_count:      int     // 在该领域完成的任务数
  - avg_rating:      float   // 在该领域的平均评分

Cardinality: Many-to-Many

Usage:
  - 专家匹配：通过任务领域找到该领域的专家
  - 领域推荐：expertise_level + task_count + avg_rating 排序
  - 跨领域发现：通过 subdomain_of 关系向上查找父领域专家
```

### 4.3 requires_tool（能力 → 工具）

```
EdgeType: requires_tool
From: Capability
To: Tool
Description: 某项能力需要调用某个工具

Properties:
  - mandatory:      bool    // 是否必需（false=可选替代）
  - default_params: json    // 默认参数
  - usage_frequency: float  // 使用频率（统计）

Cardinality: Many-to-Many

Usage:
  - 工具可用性检查：专家匹配时验证所需工具对应的服务是否健康
  - 工具调用路由：Agent 执行时通过此关系找到要调用的工具
  - 影响分析：某个服务下线时，找到受影响的能力和专家
```

### 4.4 collaborates_with（专家 ↔ 专家）

```
EdgeType: collaborates_with
From: Expert
To: Expert
Description: 专家间的协作历史（双向边）

Properties:
  - frequency:          int     // 协作次数
  - success_rate:       float   // 协作成功率
  - avg_duration_ms:    int     // 平均协作时长
  - last_collaboration: datetime // 最近一次协作时间
  - compatibility_score: float   // 兼容性评分（综合计算）

Cardinality: Many-to-Many (symmetric)

Usage:
  - 协作编排：选择专家组合时，优先选择 collaborates_with 成功率高的组合
  - 团队推荐：给定一个核心专家，推荐与其协作效果好的搭档
  - 冷启动：新专家没有协作历史时，通过能力/领域相似度推荐组合
```

### 4.5 solved_by（案例/任务 → 专家）

```
EdgeType: solved_by
From: Case / Task
To: Expert
Description: 案例或任务由某个专家参与解决

Properties:
  - contribution: float    // 贡献度 0-1（所有专家贡献度之和=1）
  - role:         string   // primary/supporting/reviewer/coordinator
  - node_id:      string   // 对应的执行节点ID（Task 用）
  - rating:       float    // 该专家在此次任务中的表现评分

Cardinality: Many-to-Many

Usage:
  - 专家匹配：找到相似案例 → 查看 solved_by 专家 → 推荐表现好的专家
  - 贡献分析：分析每个专家在不同类型任务中的贡献度
  - 结果融合：参考历史案例中各专家的贡献度设置融合权重
```

### 4.6 used_capability（案例/任务 → 能力）

```
EdgeType: used_capability
From: Case / Task
To: Capability
Description: 案例或任务使用了某项能力

Properties:
  - effectiveness: float   // 该能力在此次任务中的效果 0-1
  - usage_count:   int     // 使用次数

Cardinality: Many-to-Many

Usage:
  - 能力推荐：相似案例使用了哪些能力 → 推荐给新任务
  - 能力评估：统计某能力在各类任务中的 effectiveness
  - 能力缺口：任务需要的能力在案例中效果差 → 提示需要新能力/新专家
```

### 4.7 similar_to（案例 ↔ 案例）

```
EdgeType: similar_to
From: Case
To: Case
Description: 案例间的相似度

Properties:
  - similarity:  float     // 综合相似度 0-1
  - dimensions:  json      // 各维度相似度 {domain: 0.9, task_type: 1.0, input: 0.7}
  - computed_at: datetime

Cardinality: Many-to-Many

Usage:
  - 案例检索：输入新任务 → 找到最相似的历史案例
  - 协作计划参考：相似案例用了什么专家组合/协作模式/融合策略
  - 案例去重：similarity > 0.95 的案例可能重复，提示合并
```

---

## 五、图谱推理

### 5.1 专家匹配查询（图遍历）

```
输入：任务描述（领域标签 + 能力需求 + 输入类型）

查询步骤：
1. 找到 Domain 节点（匹配领域标签）
   MATCH (d:Domain) WHERE d.name IN $domains
   
2. 通过 operates_in 找到相关 Expert
   MATCH (d:Domain)<-[:operates_in]-(e:Expert)
   WHERE e.status = 'active'
   
3. 通过 has_capability 验证能力匹配
   MATCH (e:Expert)-[:has_capability]->(c:Capability)
   WHERE c.name IN $required_capabilities
     AND $input_type IN c.input_types
   
4. 通过 requires_tool 验证工具可用性
   MATCH (c:Capability)-[:requires_tool {mandatory: true}]->(t:Tool)
   WHERE t.service_name IN $healthy_services
   
5. 通过 solved_by 找到历史相似案例中的专家（加分）
   MATCH (case:Case)-[:solved_by]->(e:Expert)
   WHERE case.task_type = $task_type
     AND case.rating >= 4.0
   
6. 综合评分排序
   ORDER BY (domain_score * 0.3 + capability_score * 0.4 + 
             case_score * 0.2 + health_score * 0.1) DESC
   LIMIT $top_k
```

### 5.2 协作组合推荐（多跳推理）

```
输入：选定的核心专家 + 任务需求

查询步骤：
1. 找到核心专家
   MATCH (core:Expert {expert_id: $core_expert_id})
   
2. 通过 collaborates_with 找到协作效果好的搭档
   MATCH (core)-[cw:collaborates_with]->(partner:Expert)
   WHERE cw.success_rate >= 0.8
     AND partner.status = 'active'
   
3. 验证搭档是否具备任务所需的其他能力（互补性）
   MATCH (partner)-[:has_capability]->(c:Capability)
   WHERE c.name IN $remaining_capabilities
   
4. 通过 operates_in 验证领域匹配
   MATCH (partner)-[:operates_in]->(d:Domain)
   WHERE d.name IN $domains
   
5. 综合排序（协作历史 + 能力互补 + 领域匹配）
   ORDER BY (cw.success_rate * 0.4 + capability_coverage * 0.4 + domain_score * 0.2) DESC
```

### 5.3 案例检索（相似度匹配）

```
输入：新任务描述

查询步骤：
1. 找到同类型、同领域的案例
   MATCH (c:Case)
   WHERE c.task_type = $task_type
     AND c.tenant_id = $tenant_id
   
2. 通过 used_capability 找到使用了相似能力的案例
   MATCH (c)-[:used_capability]->(cap:Capability)
   WHERE cap.name IN $required_capabilities
   
3. 通过 similar_to 找到最相似的案例
   MATCH (c)-[s:similar_to]->(similar:Case)
   WHERE s.similarity >= 0.7
   
4. 按评分和成功率排序
   ORDER BY (c.rating * 0.5 + c.success_rate * 0.3 + s.similarity * 0.2) DESC
   LIMIT 5
```

---

## 六、图谱初始化

### 6.1 初始数据

系统启动时初始化以下基础数据：

**领域树（Domain）**：
```
知识图谱
  ├── 图谱构建
  ├── 图谱查询
  ├── 图谱推理
  └── 图谱治理
数据分析
  ├── 统计分析
  ├── 趋势预测
  ├── 异常检测
  └── 数据可视化
人工智能
  ├── 自然语言处理
  ├── 文本生成
  ├── 语义理解
  └── 多模态
安全合规
  ├── 权限审计
  ├── 数据脱敏
  ├── 合规检查
  └── 风险评估
工作流
  ├── 流程设计
  ├── 任务编排
  └── 自动化执行
数据治理
  ├── 数据标准
  ├── 数据质量
  ├── 元数据管理
  └── 数据目录
```

**能力定义（Capability）**：20+ 种基础能力，对应各领域的核心操作。

**工具定义（Tool）**：自动从现有31个微服务的 gRPC 接口扫描生成。

### 6.2 初始化流程

```
系统启动
  │
  ▼
检查图谱是否已初始化（查询是否有 Domain 节点）
  │
  ├── 已初始化 → 跳过
  │
  └── 未初始化 →
      │
      ├── 1. 创建领域树（Domain 节点 + subdomain_of 边）
      ├── 2. 创建能力定义（Capability 节点）
      ├── 3. 扫描微服务 gRPC 接口 → 创建 Tool 节点
      ├── 4. 创建能力-工具关联（requires_tool 边）
      ├── 5. 注册内置专家（Expert 节点 + has_capability/operates_in 边）
      └── 6. 标记初始化完成
```

---

## 七、图谱更新（持续学习）

### 7.1 任务完成后的图谱更新

每次协作任务完成后，自动更新图谱：

```
任务完成（status=completed）
  │
  ├── 1. 创建 Task 节点（如果不存在）
  │
  ├── 2. 为每个参与的专家创建 executed_by 边
  │     Task -[executed_by {node_id, status, rating}]-> Expert
  │
  ├── 3. 更新专家间的 collaborates_with 边
  │     - frequency += 1
  │     - 重新计算 success_rate
  │     - 更新 avg_duration_ms
  │     - 更新 last_collaboration
  │
  ├── 4. 更新 has_capability 边的统计
  │     - usage_count += 1
  │     - 重新计算 success_rate
  │
  ├── 5. 如果任务评分 >= 4.0 → 提升为 Case
  │     - 创建 Case 节点
  │     - 创建 solved_by 边（复制 executed_by 的贡献度）
  │     - 创建 used_capability 边
  │     - 计算与现有 Case 的 similar_to 边
  │
  └── 6. 更新专家 operates_in 边的统计
        - task_count += 1
        - 重新计算 avg_rating
```

### 7.2 专家注册时的图谱更新

```
新专家注册
  │
  ├── 1. 创建 Expert 节点
  ├── 2. 为每个能力创建 has_capability 边
  │     （如果 Capability 节点不存在，先创建）
  ├── 3. 为每个领域创建 operates_in 边
  ├── 4. 验证 requires_tool 边（Tool 节点必须已存在）
  └── 5. 发布事件 expert.registry.expert.registered
```

---

## 八、与图存储的集成

专家联盟知识图谱完全复用自研的 `mox-graph-storage-svc`：

| 用途 | 图存储 API | 说明 |
|------|-----------|------|
| 顶点 CRUD | `CreateVertex` / `GetVertex` / `UpdateVertex` / `DeleteVertex` | 专家/能力/领域等节点 |
| 边 CRUD | `CreateEdge` / `GetEdge` / `UpdateEdge` / `DeleteEdge` | 关联关系 |
| 图遍历 | `Traverse` / `BFS` / `DFS` | 专家匹配/组合推荐 |
| 路径查询 | `ShortestPath` / `AllPaths` | 专家间关联路径 |
| 邻居查询 | `GetNeighbors` | 专家的能力/领域/工具 |
| 属性过滤 | `FilterVertices` / `FilterEdges` | 按状态/评分/类型过滤 |
| CDC | 变更订阅 | 图谱变更事件通知 |

**多租户隔离**：使用 VID 租户前缀方案（`vid = "{tenant_id}:{raw_vid}"`），系统内置专家使用 `system` 租户前缀，所有租户共享。

---

## 九、总结

专家联盟知识图谱的核心设计：

1. **六元关联网络**：Expert / Capability / Domain / Tool / Data / Case 六种核心顶点，12种关联边
2. **关联驱动智能**：专家匹配、协作编排、结果融合、持续学习全部基于图谱关联关系推理
3. **权重化边**：所有关联边都带有统计权重（熟练度/成功率/频率/贡献度），持续更新
4. **完全复用自研图存储**：零新存储依赖，通过 gRPC 调用 mox-graph-storage-svc
5. **持续学习闭环**：每次任务完成自动更新图谱，案例库自动积累，系统越用越智能

---

*文档导航：[README](docs/expert-alliance/README.md) | [专家注册与协作协议](docs/expert-alliance/expert-registry-and-protocol.md) | [知识图谱关联关系设计](docs/expert-alliance/knowledge-graph-schema.md)*
