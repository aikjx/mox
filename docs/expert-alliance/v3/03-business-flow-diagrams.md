---
title: 03 - mox 模块化系统架构业务流程图
version: V3.0
authority: 🟢权威
doc_id: EA-DOC-022
last_updated: 2026-08-31
source_of_truth: V3.0架构优化业务流程
---
# 03 - mox 模块化系统架构业务流程图

> 版本：v3.0 | 日期：2026-08-26
>
> 前置：[01-架构优化分析](docs/expert-alliance/v3/01-architecture-optimization.md) | [02-架构需求矩阵](docs/expert-alliance/v3/02-requirements-matrix.md)

---

## 目录

1. [系统总体架构图](#一系统总体架构图)
2. [端到端主流程](#二端到端主流程)
3. [专家匹配流程](#三专家匹配流程)
4. [协作计划生成流程](#四协作计划生成流程)
5. [DAG执行引擎流程](#五dag执行引擎流程)
6. [Agent ReAct循环流程](#六agent-react循环流程)
7. [结果融合流程](#七结果融合流程)
8. [协作记忆与图谱学习流程](#八协作记忆与图谱学习流程)
9. [异常处理流程](#九异常处理流程)
10. [人工干预流程](#十人工干预流程)
11. [MCP调用流程](#十一mcp调用流程)
12. [多协议网关路由](#十二多协议网关路由)
13. [服务间调用时序图](#十三服务间调用时序图)
14. [知识图谱关联关系图](#十四知识图谱关联关系图)
15. [部署架构图](#十五部署架构图)

---

## 一、系统总体架构图

```mermaid
graph TB
    subgraph 接入层["接入层"]
        GH["gateway-http :8080<br/>REST/JSON-RPC/MCP/WebSocket"]
        GG["gateway-grpc :50051<br/>gRPC 服务间通信"]
    end

    subgraph 联盟核心层["联盟核心层（v3拆分）"]
        SCH["alliance-scheduler<br/>任务调度/专家匹配/计划生成"]
        EXE["alliance-executor<br/>DAG执行/节点调度/进度推送"]
        FUS["alliance-fusion<br/>结果融合/质量评估/迭代精炼"]
    end

    subgraph 能力层["能力层"]
        REG["expert-registry<br/>专家CRUD/健康检查/工具发现"]
        AGT["expert-agent<br/>ReAct循环/工具调用/AI推理"]
        MEM["expert-memory<br/>统一记忆/案例库/图谱学习"]
    end

    subgraph Sidecar["AI推理"]
        AI["ai-inference-sidecar<br/>Python (UDS通信)"]
    end

    subgraph 数据层["数据层"]
        PG[("PostgreSQL<br/>任务/专家/案例/审计")]
        GS[("自研图存储<br/>RocksDB+Raft<br/>专家联盟知识图谱")]
        RD[("Redis<br/>缓存/会话/工作记忆/限流")]
        NS[("NATS JetStream<br/>事件总线/进度推送/DLQ")]
        MO[("MinIO<br/>任务结果/导出文件")]
    end

    subgraph 底层服务["现有31个微服务"]
        S1["mox-ai-svc"]
        S2["mox-graph-svc"]
        S3["mox-search-svc"]
        S4["mox-flow-svc"]
        S5["mox-compliance-svc"]
        S6["...其他26个服务"]
    end

    GH --> SCH
    GH --> EXE
    GH --> FUS
    GH --> REG
    GG --> SCH
    GG --> EXE
    GG --> FUS
    GG --> REG
    GG --> AGT
    GG --> MEM

    SCH --> REG
    SCH --> MEM
    SCH --> GS
    EXE --> AGT
    EXE --> MEM
    EXE --> NS
    FUS --> AGT
    FUS --> MEM
    AGT --> AI
    AGT --> S1
    AGT --> S2
    AGT --> S3
    AGT --> S4
    AGT --> S5
    AGT --> S6
    MEM --> PG
    MEM --> GS
    MEM --> RD
    REG --> PG
    REG --> RD
    SCH --> PG
    EXE --> PG
    EXE --> RD
    GH --> RD
```

---

## 二、端到端主流程

```mermaid
flowchart TD
    A["用户输入自然语言描述"] --> B["gateway-http<br/>认证/限流/租户解析/协议转码"]
    B --> C["alliance-scheduler<br/>1.任务解析 NLP提取领域/能力/数据需求"]
    C --> D["2.专家匹配<br/>图谱推理+综合评分→Top N专家"]
    D --> E["3.案例检索<br/>相似历史案例→协作模式建议"]
    E --> F["4.协作计划生成<br/>DAG: 节点=专家调用, 边=数据依赖"]
    F --> G["5.计划验证<br/>无环检测/可达性/输入完整性"]
    G --> H{"验证通过?"}
    H -->|否| I["返回计划生成错误<br/>降级为单专家串行"]
    H -->|是| J["alliance-executor<br/>6.DAG执行引擎<br/>拓扑排序+并行调度"]
    I --> J
    J --> K["7.节点执行循环<br/>对每个Ready节点:"]
    K --> L["调用 expert-agent<br/>ReAct循环执行"]
    L --> M{"节点成功?"}
    M -->|是| N["存储结果+更新进度<br/>WebSocket推送"]
    M -->|否| O["异常处理<br/>重试/替代专家/降级跳过"]
    O --> P{"可恢复?"}
    P -->|是| L
    P -->|否| Q{"关键路径?"}
    Q -->|是| R["任务失败"]
    Q -->|否| N
    N --> S{"所有节点完成?"}
    S -->|否| K
    S -->|是| T["alliance-fusion<br/>8.结果融合 6种策略"]
    T --> U["9.质量评估<br/>完整性/一致性/准确性"]
    U --> V{"质量达标?"}
    V -->|否,未达迭代上限| W["迭代精炼<br/>返回审核修改"]
    W --> T
    V -->|是或达上限| X["alliance-memory<br/>10.协作记忆更新"]
    X --> Y["工作记忆归档+会话记忆更新"]
    Y --> Z{"评分≥4?"}
    Z -->|是| AA["提升为案例<br/>写入知识图谱"]
    Z -->|否| AB["仅更新统计"]
    AA --> AC["图谱学习<br/>更新边权重(频率/成功率/效果)"]
    AB --> AC
    AC --> AD["11.结果交付<br/>JSON/导出/通知"]
    R --> AE["返回错误+失败节点信息"]
```

---

## 三、专家匹配流程

```mermaid
flowchart TD
    A["输入: 任务描述"] --> B["1.任务解析"]
    B --> C["提取领域标签<br/>(NLP分类+关键词匹配)"]
    C --> D["提取能力需求<br/>(识别操作类型)"]
    D --> E["识别输入/输出类型"]
    E --> F["提取约束条件<br/>(超时/预算/质量)"]

    F --> G["2.图谱推理查询"]
    G --> H["MATCH (d:Domain)<br/>WHERE d.name IN 领域标签"]
    H --> I["MATCH (d)<-[:operates_in]-(e:Expert)<br/>WHERE e.status=active<br/>AND e.tenant_id IN (当前租户,'system')"]
    I --> J["MATCH (e)-[:has_capability]->(c:Capability)<br/>WHERE c.name IN 能力需求<br/>AND 输入类型 IN c.input_types"]
    J --> K["MATCH (c)-[:requires_tool{mandatory:true}]->(t:Tool)<br/>WHERE t.service_name IN 健康服务"]
    K --> L["可选: MATCH (case:Case)-[:solved_by]->(e)<br/>WHERE case.task_type=当前类型<br/>AND case.rating≥4.0"]

    L --> M["3.综合评分 (0-1)"]
    M --> N["domain_match × 0.25<br/>(领域匹配度)"]
    N --> O["capability_match × 0.30<br/>(能力匹配度+熟练度)"]
    O --> P["health_status × 0.10<br/>(healthy=1, degraded=0.5)"]
    P --> Q["historical_performance × 0.15<br/>(成功率+平均评分)"]
    Q --> R["collaboration_compatibility × 0.10<br/>(与已选专家协作历史)"]
    R --> S["priority_bonus × 0.10<br/>(专家优先级/10)"]
    S --> T["总分 = 各项加权之和"]

    T --> U["4.排序与筛选"]
    U --> V["按 score 降序排列"]
    V --> W["过滤 score<0.3 (最低阈值)"]
    W --> X["去重 (同领域只保留Top 2)"]
    X --> Y["限制最大数量 (默认5)"]
    Y --> Z["输出: 专家列表<br/>(含评分/匹配能力/匹配领域/健康状态)"]
```

---

## 四、协作计划生成流程

```mermaid
flowchart TD
    A["输入: 任务描述+匹配专家列表+相似案例"] --> B["1.协作模式选择"]
    B --> C{"用户指定模式?"}
    C -->|是| D["使用指定模式"]
    C -->|否| E{"有相似案例?"}
    E -->|是| F["参考案例的协作模式"]
    E -->|否| G["自动选择"]
    F --> H
    G --> H["判断任务特征"]
    H --> I{"可分解为独立子任务?"}
    I -->|是| J["Parallel 并行"]
    I -->|否| K{"有严格先后依赖?"}
    K -->|是| L["Serial 串行"]
    K -->|否| M{"需要多视角决策?"}
    M -->|是| N["Debate 辩论"]
    M -->|否| O{"任务复杂需要协调?"}
    O -->|是| P["Hierarchical 分层"]
    O -->|否| Q{"质量要求高需要审核?"}
    Q -->|是| R["Iterative 迭代"]
    Q -->|否| S["Dynamic 动态"]
    D --> T
    J --> T
    L --> T
    N --> T
    P --> T
    R --> T
    S --> T["确定协作模式"]

    T --> U["2.任务分解 (生成节点)"]
    U --> V["对每个匹配专家创建Execute节点"]
    V --> W["特殊节点:"]
    W --> X["多专家并行→Parallel网关+Join汇聚"]
    X --> Y["需要融合→Fusion节点"]
    Y --> Z["需要人工审核→HumanReview节点"]
    Z --> AA["条件分支→Condition节点"]

    AA --> AB["3.依赖分析 (生成边)"]
    AB --> AC["数据依赖:<br/>节点A输出类型∈节点B输入类型→A→B边"]
    AC --> AD["控制依赖:<br/>Parallel网关→所有分支<br/>Join汇聚←所有分支<br/>Fusion节点←所有上游"]
    AD --> AE["循环检测: DFS检测环"]

    AE --> AF{"有环?"}
    AF -->|是| AG["报错 PlanCycleDetected<br/>返回用户调整"]
    AF -->|否| AH["4.计划验证"]
    AH --> AI["所有节点输入有来源?"]
    AI --> AJ["所有节点可达?"]
    AJ --> AK["估算总执行时间(关键路径)"]

    AK --> AL["5.输出 CollaborationPlan"]
    AL --> AM["plan_id, task_id, mode"]
    AM --> AN["nodes: [节点列表]"]
    AN --> AO["edges: [边列表]"]
    AO --> AP["fusion_strategy, max_iterations, timeout"]
```

---

## 五、DAG执行引擎流程

```mermaid
flowchart TD
    A["输入: CollaborationPlan"] --> B["1.初始化"]
    B --> C["所有节点状态=Pending"]
    C --> D["计算每个节点入度(上游依赖数)"]
    D --> E["入度=0的节点→Ready"]
    E --> F["创建执行上下文(任务级工作记忆)"]

    F --> G["2.调度循环"]
    G --> H{"存在Ready节点且未全部完成?"}
    H -->|否| I["完成判断"]
    H -->|是| J["2a.批量获取Ready节点"]
    J --> K["按优先级排序"]
    K --> L["限制并发数(默认10)"]

    L --> M["2b.并行执行节点"]
    M --> N["对每个Ready节点:"]
    N --> O["状态→Running"]
    O --> P["收集上游节点输出作为输入"]
    P --> Q["调用 expert-agent (gRPC)"]
    Q --> R["等待结果(带超时)"]
    R --> S{"成功?"}
    S -->|是| T["状态=Success, 存储结果"]
    S -->|否| U["异常处理"]
    U --> V{"可重试?"}
    V -->|是| W["指数退避重试(≤3次)"]
    W --> X{"重试成功?"}
    X -->|是| T
    X -->|否| Y{"有替代专家?"}
    Y -->|是| Z["切换替代专家重试"]
    Z --> AA{"成功?"}
    AA -->|是| T
    AA -->|否| AB["状态=Failed"]
    Y -->|否| AB
    V -->|否| AB

    T --> AC["2c.更新依赖"]
    AB --> AC
    AC --> AD["遍历下游节点:"]
    AD --> AE["下游入度-1"]
    AE --> AF{"入度=0?"}
    AF -->|是| AG["状态=Ready<br/>(Condition节点需额外判断条件)"]
    AF -->|否| AH["保持Pending"]
    AG --> AI["2d.进度推送"]
    AH --> AI
    AI --> AJ["计算总进度=完成节点数/总节点数"]
    AJ --> AK["WebSocket推送给前端"]
    AK --> AL["发布task.progress事件(NATS)"]
    AL --> G

    I --> AM{"所有节点Success?"}
    AM -->|是| AN["进入结果融合"]
    AM -->|否| AO{"存在Failed关键路径节点?"}
    AO -->|是| AP["任务失败"]
    AO -->|否| AQ["降级: 跳过Failed节点<br/>已完成节点进入融合"]
    AQ --> AN
```

---

## 六、Agent ReAct循环流程

```mermaid
flowchart TD
    A["输入: 节点配置(专家ID+输入+配置)"] --> B["1.理解 Understand"]
    B --> C["解析任务目标<br/>(从节点配置+上游输入)"]
    C --> D["识别可用工具<br/>(从专家定义的tools列表)"]
    D --> E["检索相关知识<br/>(图谱查询/语义搜索/历史案例)"]
    E --> F["输出: 理解摘要+可用工具列表+相关知识"]

    F --> G["2.规划 Plan"]
    G --> H["分解为执行步骤(可能多步)"]
    H --> I["每步选择工具+参数"]
    I --> J["评估步骤间依赖关系"]
    J --> K["输出: 执行计划(步骤列表)"]

    K --> L["3.执行 Act (循环每一步)"]
    L --> M["选择工具"]
    M --> N["构造参数<br/>(从输入+上下文提取)"]
    N --> O["调用工具 (gRPC)"]
    O --> P{"工具类型?"}
    P -->|AI推理| Q["调用 ai-inference-sidecar<br/>(UDS通信, 支持流式)"]
    P -->|图谱操作| R["调用 mox-graph-svc"]
    P -->|数据处理| S["调用对应数据服务"]
    P -->|其他| T["调用对应微服务"]
    Q --> U["记录工具调用结果"]
    R --> U
    S --> U
    T --> U
    U --> V["更新工作记忆"]

    V --> W["4.观察 Observe"]
    W --> X["汇总所有工具调用结果"]
    X --> Y["检查结果是否满足目标"]
    Y --> Z["识别异常/错误/不完整"]

    Z --> AA["5.审核 Review"]
    AA --> AB{"结果满足目标 AND 质量达标?"}
    AB -->|是| AC["输出最终结果"]
    AB -->|否| AD{"未达最大迭代次数?"}
    AD -->|是| AE["调整策略<br/>回到规划步骤"]
    AE --> G
    AD -->|否| AF["输出当前最佳结果+警告"]

    AC --> AG["输出 NodeOutput"]
    AF --> AG
    AG --> AH["outputs: 输出数据"]
    AH --> AI["thoughts: 思考过程<br/>(每步理解/规划/观察)"]
    AI --> AJ["metrics: 工具调用次数/AI调用次数/Token消耗/耗时"]
```

---

## 七、结果融合流程

```mermaid
flowchart TD
    A["输入: 多专家结果列表+融合策略"] --> B["1.策略路由"]
    B --> C{"融合策略?"}

    C -->|MajorityVote 多数投票| D["分类/判断类任务"]
    D --> E["统计各专家结果"]
    E --> F["取多数一致的结果"]
    F --> G["输出融合结果"]

    C -->|WeightedVote 加权投票| H["有专家质量差异"]
    H --> I["获取各专家权重<br/>(历史成功率+贡献度+置信度)"]
    I --> J["加权计算最终结果"]
    J --> G

    C -->|Concatenate 拼接合并| K["各专家负责不同部分"]
    K --> L["按输出类型/职责拼接"]
    L --> M["去重+格式统一"]
    M --> G

    C -->|BestOf 择优选择| N["有明确评估标准"]
    N --> O["对每个结果评分<br/>(完整性/准确性/相关性)"]
    O --> P["选择评分最高的结果"]
    P --> G

    C -->|DebateArbitrate 辩论仲裁| Q["观点冲突的决策类"]
    Q --> R["各专家陈述观点"]
    R --> S["互相质询(调用专家互评)"]
    S --> T["协调专家仲裁<br/>(综合各方观点)"]
    T --> G

    C -->|IterativeRefine 迭代精炼| U["高质量内容生成"]
    U --> V["专家A生成初稿"]
    V --> W["专家B审核修改"]
    W --> X{"质量达标 OR 达迭代上限?"}
    X -->|否| Y["返回专家A修改"]
    Y --> V
    X -->|是| Z["输出最终版本"]
    Z --> G

    G --> AA["2.质量评估"]
    AA --> AB["完整性: 是否覆盖所有需求点"]
    AB --> AC["一致性: 各部分是否矛盾"]
    AC --> AD["准确性: 事实/数据是否正确"]
    AD --> AE["输出质量评分(0-1)"]

    AE --> AF{"质量≥阈值?"}
    AF -->|是| AG["输出最终融合结果"]
    AF -->|否,未达上限| AH["迭代精炼<br/>返回融合步骤"]
    AH --> B
    AF -->|否,达上限| AI["输出当前结果+质量警告"]
    AI --> AG
```

---

## 八、协作记忆与图谱学习流程

```mermaid
flowchart TD
    A["触发: 任务完成"] --> B["1.工作记忆归档"]
    B --> C["从Redis读取任务工作记忆"]
    C --> D["序列化为归档格式"]
    D --> E["写入PostgreSQL task_archives表"]
    E --> F["删除Redis中的工作记忆"]

    F --> G["2.会话记忆更新"]
    G --> H["读取用户会话记忆"]
    H --> I["更新: 最近任务/偏好/常用专家"]
    I --> J["写回Redis (TTL 24h)"]

    J --> K["3.案例提升判断"]
    K --> L{"任务评分≥4.0?"}
    L -->|否| M["仅更新统计数据"]
    L -->|是| N["4.创建案例"]
    N --> O["生成case_id"]
    O --> P["提取任务摘要/输入输出/协作快照"]
    P --> Q["生成语义向量(pgvector)"]
    Q --> R["写入PostgreSQL cases表"]
    R --> S["写入知识图谱:"]
    S --> T["创建Case顶点"]
    T --> U["创建solved_by边→参与专家<br/>(记录contribution/role/rating)"]
    U --> V["创建used_capability边→使用的能力<br/>(记录effectiveness)"]
    V --> W["计算与现有Case的similar_to边"]

    W --> X["5.图谱学习(异步批量)"]
    M --> X
    X --> Y["更新has_capability边:"]
    Y --> Z["usage_count+1, 重新计算success_rate"]
    Z --> AA["更新operates_in边:"]
    AA --> AB["task_count+1, 重新计算avg_rating"]
    AB --> AC["更新collaborates_with边(专家对):"]
    AC --> AD["frequency+1, 重新计算success_rate/avg_duration"]
    AD --> AE["更新solved_by边(案例→专家):"]
    AE --> AF["更新contribution/rating统计"]

    AF --> AG["6.发布记忆更新事件"]
    AG --> AH["expert.memory.updated (NATS)"]
    AH --> AI["通知相关服务更新缓存"]
```

---

## 九、异常处理流程

```mermaid
flowchart TD
    A["节点执行失败"] --> B["1.错误类型判断"]
    B --> C{"错误类型?"}

    C -->|可重试错误<br/>(网络/5xx/超时)| D["2.指数退避重试"]
    D --> E["第1次: 等待100ms"]
    E --> F["第2次: 等待200ms+抖动"]
    F --> G["第3次: 等待400ms+抖动"]
    G --> H{"重试成功?"}
    H -->|是| I["节点Success"]
    H -->|否| J["3.重试耗尽处理"]

    C -->|超时错误| K["2.超时重试(≤2次)"]
    K --> L{"重试成功?"}
    L -->|是| I
    L -->|否| J

    C -->|业务错误<br/>(参数/权限/校验)| M["立即失败(不重试)"]
    M --> J

    C -->|不可恢复错误<br/>(专家不存在/工具不可用)| N["立即失败(不重试)"]
    N --> J

    J --> O["4.替代专家判断"]
    O --> P{"同领域同能力有替代专家<br/>(评分≥0.5)?"}
    P -->|是| Q["5.切换替代专家重试"]
    Q --> R{"成功?"}
    R -->|是| I
    R -->|否| S["6.降级判断"]
    P -->|否| S

    S --> T{"是否关键路径节点?"}
    T -->|否| U["7.降级跳过"]
    U --> V["标记节点Skipped"]
    V --> W["继续执行下游节点<br/>(下游输入用默认值/空值)"]
    W --> X["任务继续(降级完成)"]

    T -->|是| Y["8.任务失败"]
    Y --> Z["标记任务Failed"]
    Z --> AA["记录失败节点+错误信息"]
    AA --> AB["通知用户(WebSocket+推送)"]
    AB --> AC["支持人工重试/干预"]
```

---

## 十、人工干预流程

```mermaid
flowchart TD
    A["触发: HumanReview节点 或 用户主动暂停"] --> B["任务状态→Paused"]
    B --> C["推送通知(飞书/邮件/站内信)"]
    C --> D["前端展示待审核页面"]

    D --> E["页面内容:"]
    E --> F["当前DAG执行状态图"]
    F --> G["已完成节点的结果"]
    G --> H["待审核节点的输入/建议输出"]
    H --> I["操作按钮组"]

    I --> J{"用户操作?"}

    J -->|通过 Approve| K["任务恢复 Paused→Running"]
    K --> L["继续执行后续节点"]

    J -->|拒绝 Reject| M["节点标记Failed"]
    M --> N["按异常处理流程处理"]

    J -->|修改计划 Modify| O["更新DAG"]
    O --> P["添加/删除/修改节点"]
    P --> Q["重新验证计划"]
    Q --> R["任务恢复执行"]

    J -->|指定专家 Assign| S["替换当前节点的专家"]
    S --> T["重新执行该节点"]

    J -->|跳过节点 Skip| U["标记节点Skipped"]
    U --> V["继续执行下游(用默认输入)"]

    J -->|取消任务 Cancel| W["终止所有运行中节点"]
    W --> X["任务状态→Cancelled"]
    X --> Y["释放资源/清理临时数据"]
```

---

## 十一、MCP调用流程

```mermaid
sequenceDiagram
    participant C as AI客户端<br/>(Claude/Cursor)
    participant G as gateway-http<br/>MCP适配层
    participant R as expert-registry<br/>工具注册
    participant S as alliance-scheduler<br/>(转码调用)
    participant B as 底层gRPC服务

    Note over C,G: 1. 初始化握手
    C->>G: POST /mcp {method:"initialize", params:{protocolVersion, capabilities}}
    G-->>C: {result:{protocolVersion:"2024-11-05", capabilities:{tools:{},resources:{}}}}

    Note over C,G: 2. 客户端通知初始化完成
    C->>G: POST /mcp {method:"notifications/initialized"}

    Note over C,G: 3. 获取可用工具列表
    C->>G: POST /mcp {method:"tools/list"}
    G->>R: 查询工具缓存/自动发现
    R->>B: gRPC Server Reflection (扫描所有服务)
    B-->>R: .proto描述 (service+method+message)
    R->>R: 生成MCP Tool描述 (name+description+inputSchema)
    R-->>G: 工具列表
    G-->>C: {result:{tools:[{name,description,inputSchema}]}}

    Note over C,G: 4. 调用工具
    C->>G: POST /mcp {method:"tools/call", params:{name:"graph.create_vertex", arguments:{vid,vtype}}}
    G->>G: JSON-RPC→gRPC转码
    Note right of G: 查路由表: graph.create_vertex<br/>→ gRPC graph.VertexService.CreateVertex<br/>→ JSON arguments → Protobuf CreateVertexRequest
    G->>S: gRPC调用 (带租户/认证/Trace)
    S->>B: gRPC调用底层服务
    B-->>S: Protobuf响应
    S-->>G: gRPC响应
    G->>G: Protobuf→JSON → 包装为MCP CallToolResult
    G-->>C: {result:{content:[{type:"text",text:"{...json result...}"}], isError:false}}
```

---

## 十二、多协议网关路由

```mermaid
flowchart TD
    A["客户端请求 :8080/:50051"] --> B{"端口?"}

    B -->|:50051 gRPC端口| C["gateway-grpc"]
    C --> D["纯HTTP/2 gRPC"]
    D --> E["服务间路由/负载均衡"]
    E --> F["内部gRPC服务"]

    B -->|:8080 HTTP端口| G["gateway-http"]
    G --> H{"Path?"}

    H -->|/rpc| I["JSON-RPC 2.0 Handler"]
    I --> J["解析JSON-RPC请求"]
    J --> K["查转码路由表<br/>(method→gRPC service+method)"]
    K --> L["JSON params→Protobuf request"]
    L --> M["gRPC调用后端(tonic)"]
    M --> N["Protobuf response→JSON"]
    N --> O["包装JSON-RPC响应"]

    H -->|/mcp| P["MCP Handler (JSON-RPC子集)"]
    P --> Q{"MCP method?"}
    Q -->|initialize| R["能力协商响应"]
    Q -->|tools/list| S["查工具注册表→返回Tool列表"]
    Q -->|tools/call| T["同JSON-RPC转码→调用gRPC"]
    Q -->|resources/*| U["资源列表/读取"]
    Q -->|prompts/*| V["提示词模板"]
    Q -->|其他| W["标准JSON-RPC处理"]

    H -->|/api/v1/*| X["REST Handler (axum)"]
    X --> Y["REST路由匹配"]
    Y --> Z["参数提取(path/query/body)"]
    Z --> AA["gRPC调用后端(转码)"]
    AA --> AB["包装统一响应格式<br/>{code,message,data,request_id}"]

    H -->|/ws| AC["WebSocket Handler"]
    AC --> AD["协议升级"]
    AD --> AE["订阅NATS事件流"]
    AE --> AF["实时推送进度/输出"]

    H -->|/metrics| AG["Prometheus Handler"]
    H -->|/health| AH["Health Check Handler"]
```

---

## 十三、服务间调用时序图

```mermaid
sequenceDiagram
    participant U as 用户
    participant GH as gateway-http
    participant SCH as alliance-scheduler
    participant REG as expert-registry
    participant MEM as expert-memory
    participant GS as 图存储
    participant EXE as alliance-executor
    participant AGT as expert-agent
    participant AI as ai-inference
    participant B as 底层服务
    participant FUS as alliance-fusion

    U->>GH: POST /api/v1/expert/tasks (自然语言)
    GH->>SCH: gRPC CreateTask
    SCH->>SCH: 1.任务解析 (NLP)
    SCH->>GS: 2.图谱推理查询 (专家匹配)
    GS-->>SCH: 候选专家+关联
    SCH->>MEM: 3.案例检索 (相似历史)
    MEM-->>SCH: 案例列表
    SCH->>SCH: 4.生成协作计划 (DAG)
    SCH->>SCH: 5.计划验证
    SCH-->>GH: task_id + 计划预览
    GH-->>U: task_id

    Note over SCH,EXE: 异步执行 (通过NATS事件触发)
    SCH->>EXE: NATS task.created (含计划)
    EXE->>EXE: 6.DAG初始化 (拓扑排序)

    loop 每个Ready节点 (并行)
        EXE->>AGT: gRPC ExecuteNode (专家+输入)
        AGT->>AGT: ReAct: 理解→规划
        AGT->>B: gRPC 工具调用
        B-->>AGT: 工具结果
        AGT->>AI: UDS AI推理 (流式)
        AI-->>AGT: 流式输出
        AGT->>AGT: ReAct: 观察→审核
        AGT-->>EXE: 节点结果+思考过程
        EXE->>EXE: 更新依赖+进度
        EXE->>GH: WebSocket 进度推送
        GH->>U: 实时进度
    end

    EXE->>FUS: NATS 所有节点完成 (含结果)
    FUS->>FUS: 7.结果融合 (6种策略)
    FUS->>AGT: gRPC (辩论/迭代模式需要专家参与)
    AGT-->>FUS: 专家互评/修改
    FUS->>FUS: 8.质量评估
    FUS->>MEM: 9.记忆更新+案例提升+图谱学习 (异步)
    MEM->>GS: 写入案例顶点+更新边权重
    FUS-->>EXE: 融合结果
    EXE->>GH: WebSocket 最终结果
    GH->>U: 结果交付

    U->>GH: GET /api/v1/expert/tasks/{id}/result
    GH->>SCH: gRPC GetTaskResult
    SCH-->>GH: 完整结果+执行详情
    GH-->>U: JSON结果
```

---

## 十四、知识图谱关联关系图

```mermaid
graph LR
    subgraph 顶点类型["7种顶点"]
        E["Expert<br/>专家"]
        C["Capability<br/>能力"]
        D["Domain<br/>领域"]
        T["Tool<br/>工具"]
        DA["Data<br/>数据"]
        CA["Case<br/>案例"]
        TA["Task<br/>任务(运行时)"]
    end

    E -->|has_capability<br/>proficiency/success_rate| C
    E -->|operates_in<br/>expertise_level/task_count| D
    C -->|requires_tool<br/>mandatory| T
    T -->|operates_on<br/>operation| DA
    D -->|contains_data<br/>category| DA
    CA -->|solved_by<br/>contribution/role/rating| E
    CA -->|used_capability<br/>effectiveness| C
    CA -->|similar_to<br/>similarity/dimensions| CA
    E -->|collaborates_with<br/>frequency/success_rate| E
    C -->|depends_on<br/>dependency_type| C
    D -->|subdomain_of| D
    TA -->|executed_by<br/>node_id/status| E

    style E fill:#e1f5fe
    style C fill:#f3e5f5
    style D fill:#e8f5e9
    style T fill:#fff3e0
    style DA fill:#fce4ec
    style CA fill:#fff9c4
    style TA fill:#efebe9
```

---

## 十五、部署架构图

```mermaid
graph TB
    subgraph 外部["外部访问"]
        LB["负载均衡器<br/>(云厂商LB/Nginx)"]
    end

    subgraph K8s["Kubernetes 集群"]
        subgraph 接入NS["expert-gateway 命名空间"]
            GH["gateway-http<br/>Deployment x3<br/>:8080"]
            GG["gateway-grpc<br/>Deployment x2<br/>:50051"]
        end

        subgraph 核心NS["expert-core 命名空间"]
            SCH["alliance-scheduler<br/>Deployment x3<br/>HPA: CPU>60%"]
            EXE["alliance-executor<br/>Deployment x3+<br/>HPA: 队列长度"]
            FUS["alliance-fusion<br/>Deployment x2<br/>HPA: 融合任务数"]
        end

        subgraph 能力NS["expert-capability 命名空间"]
            REG["expert-registry<br/>Deployment x2"]
            AGT["expert-agent<br/>Deployment x3+<br/>HPA: 并发Agent数"]
            MEM["expert-memory<br/>Deployment x3"]
        end

        subgraph 数据NS["data 命名空间"]
            PG["PostgreSQL<br/>StatefulSet x2<br/>主从+WAL"]
            GS["自研图存储<br/>StatefulSet x3<br/>RocksDB+Raft+PVC"]
            RD["Redis<br/>StatefulSet x3<br/>哨兵模式"]
            NS["NATS JetStream<br/>StatefulSet x3"]
            MO["MinIO<br/>StatefulSet x4<br/>纠删码"]
        end

        subgraph 可观测NS["observability 命名空间"]
            OT["OTel Collector"]
            PM["Prometheus"]
            JG["Jaeger"]
            LK["Loki"]
            GF["Grafana"]
            AM["Alertmanager"]
        end
    end

    subgraph 外部服务["外部/现有服务"]
        B["现有31个微服务<br/>(独立部署)"]
        AI["Python AI推理<br/>(作为agent的sidecar)"]
    end

    LB --> GH
    LB --> GG
    GH --> SCH
    GH --> EXE
    GH --> FUS
    GH --> REG
    GG --> SCH
    GG --> EXE
    GG --> FUS
    GG --> REG
    GG --> AGT
    GG --> MEM

    SCH --> REG
    SCH --> MEM
    SCH --> GS
    EXE --> AGT
    EXE --> MEM
    EXE --> NS
    FUS --> AGT
    FUS --> MEM
    AGT --> AI
    AGT --> B

    SCH --> PG
    EXE --> PG
    REG --> PG
    MEM --> PG
    MEM --> GS
    GH --> RD
    SCH --> RD
    EXE --> RD
    AGT --> RD
    EXE --> NS
    GH --> MO
    EXE --> MO

    GH --> OT
    SCH --> OT
    EXE --> OT
    FUS --> OT
    REG --> OT
    AGT --> OT
    MEM --> OT
    OT --> PM
    OT --> JG
    OT --> LK
    PM --> GF
    JG --> GF
    LK --> GF
    PM --> AM
```

---

## 十六、状态机总图

```mermaid
stateDiagram-v2
    [*] --> Pending: 创建任务

    Pending --> Planning: 调度器接收
    Planning --> Running: 计划生成完成
    Planning --> Failed: 计划生成失败

    Running --> Paused: 人工暂停/HumanReview节点
    Paused --> Running: 恢复执行
    Paused --> Cancelled: 取消任务

    Running --> Completed: 所有节点成功+融合完成
    Running --> Failed: 关键路径节点失败
    Running --> Cancelled: 用户取消

    Failed --> Running: 人工重试(修复后)
    Completed --> [*]
    Cancelled --> [*]
    Failed --> [*]

    note right of Running
        节点状态子状态机:
        Pending → Ready → Running → Success
                              ↘ Failed → Retrying → Success
                              ↘ Timeout
                              ↘ Skipped (条件不满足)
    end note
```

---

*文档导航：[README](docs/expert-alliance/v3/README.md) | [01-架构优化分析](docs/expert-alliance/v3/01-architecture-optimization.md) | [02-架构需求矩阵](docs/expert-alliance/v3/02-requirements-matrix.md) | [03-mox 模块化系统架构业务流程图](docs/expert-alliance/v3/03-business-flow-diagrams.md)*
