# 08 · 业务功能关联关系图

> **版本**: v1.0 · **日期**: 2026-08-27
> **说明**: 本文档定义 8 大业务域之间的功能关联、实体关系、调用链路和数据流向，是模块化架构的核心关联图谱。

---

## 一、8 域功能关联总图

### 1.1 域间依赖关系图

```mermaid
graph TD
    subgraph 接入层["L0-L1 接入与网关"]
        GW["🌐 Gateway<br/>31域路由注册中心<br/>/kg /ai /cloud /enterprise ..."]
        IAM["🔐 IAM<br/>身份认证 / RBAC / JWT"]
    end

    subgraph 编排层["L2 应用编排"]
        ENT["🏢 Enterprise<br/>P0-P12 流程引擎<br/>项目/设计/评审/部署/运维/复盘"]
        ORCH["⚙️ Orchestrator<br/>任务调度 / 工作流"]
        TEST["🧪 Test Harness<br/>集成测试 / 验证 / 冒烟"]
    end

    subgraph 业务域["L3-L4 8大业务域"]
        AI["🤖 AI 域<br/>意图识别 / 能力路由<br/>激活扩散 / CEM评分"]
        KG["🕸️ KG 域<br/>图算法 / 社区检测<br/>路径查找 / 知识图谱"]
        FLOW["🔄 Flow 域<br/>流程引擎 / WASM算子<br/>流程优化 / 桥接"]
        CLOUD["☁️ Cloud 域<br/>对象存储 / 卷管理<br/>元数据 / Chunk调度"]
        DATA["📊 Data 域<br/>数据归一化 / ETL<br/>合规 / 目录 / 公式计算"]
        VOICE["🎤 Voice 域<br/>ASR / TTS / 声纹<br/>语音意图 / DSP"]
        MARKET["🛒 Market 域<br/>插件注册 / 分发<br/>模板 / 计费"]
        STREAMS["🌊 Streams 域<br/>事件总线 / CDC<br/>流处理 / 实时数据"]
    end

    subgraph 基础层["L5 基础层"]
        FW["🧱 Framework<br/>error/auth/server/metrics<br/>resilience/tenant/health"]
        FOUND["🏗️ Foundation<br/>cloud-foundation<br/>platform-foundation<br/>observability"]
    end

    %% 网关到各域
    GW --> AI
    GW --> KG
    GW --> FLOW
    GW --> CLOUD
    GW --> DATA
    GW --> VOICE
    GW --> MARKET
    GW --> STREAMS
    GW --> ENT
    GW --> IAM

    %% 编排层调用
    ENT --> AI
    ENT --> KG
    ENT --> CLOUD
    ENT --> DATA
    ORCH --> FLOW
    ORCH --> STREAMS
    TEST --> AI
    TEST --> KG
    TEST --> DATA

    %% 域间关联
    AI --> KG
    AI --> DATA
    KG --> DATA
    KG --> CLOUD
    FLOW --> AI
    FLOW --> KG
    CLOUD --> DATA
    DATA --> STREAMS
    VOICE --> AI
    STREAMS --> KG

    %% 所有域依赖基础层
    AI --> FW
    KG --> FW
    FLOW --> FW
    CLOUD --> FW
    DATA --> FW
    VOICE --> FW
    MARKET --> FW
    STREAMS --> FW
    ENT --> FW
    GW --> FW

    KG --> FOUND
    CLOUD --> FOUND
    DATA --> FOUND

    style GW fill:#E3F2FD,stroke:#1565C0
    style ENT fill:#FFF3E0,stroke:#E65100
    style AI fill:#F3E5F5,stroke:#6A1B9A
    style KG fill:#E8F5E9,stroke:#2E7D32
    style CLOUD fill:#E0F7FA,stroke:#00838F
    style FW fill:#F5F5F5,stroke:#424242
```

### 1.2 域间调用关系说明

| 源域 | 目标域 | 调用场景 | 关键 API |
|---|---|---|---|
| Gateway | 全部 8 域 | 路由分发 | `/kg/v1/*`, `/ai/engine/*`, ... |
| Enterprise | AI | 需求分析辅助 | `POST /ai/engine/analyze` |
| Enterprise | KG | 设计图谱/复盘知识 | `POST /kg/v1/ingest`, `GET /kg/v1/neighborhood` |
| Enterprise | Cloud | 部署产物存储 | `POST /cloud/v1/objects` |
| Enterprise | Data | 项目数据查询 | `GET /data/v1/query` |
| AI | KG | 意图相关知识推荐 | `GET /kg/v1/neighborhood?center={intent}` |
| AI | Data | 意图归一化数据 | `POST /data/v1/norm/intent` |
| KG | Data | 图谱数据持久化 | `POST /data/v1/store` |
| KG | Cloud | 图谱大文件存储 | `POST /cloud/s3/{bucket}/{key}` |
| Flow | AI | 流程节点智能决策 | `POST /ai/engine/process` |
| Flow | KG | 流程依赖图谱查询 | `GET /kg/v1/path?src={nodeA}&dst={nodeB}` |
| Cloud | Data | 存储元数据管理 | `POST /data/v1/catalog` |
| Data | Streams | 数据变更事件推送 | `POST /streams/v1/events` |
| Voice | AI | 语音转文字后意图识别 | `POST /ai/engine/process` |
| Streams | KG | 实时事件入图 | `POST /kg/v1/ingest` |
| Test | AI/KG/Data | 集成测试验证 | `POST /test-harness/v1/integration` |

---

## 二、核心业务实体关系图（ER）

### 2.1 mox 模块化系统架构实体 ER 图

```mermaid
erDiagram
    USER ||--o{ REQUIREMENT : "提交"
    USER ||--o{ PROJECT : "负责"
    USER ||--o{ INCIDENT : "处理"
    ROLE ||--o{ USER : "分配"
    PERMISSION ||--o{ ROLE : "授予"

    REQUIREMENT ||--|| PROJECT : "转化为"
    PROJECT ||--o{ SRS : "产生"
    SRS ||--o{ USER_STORY : "包含"
    USER_STORY ||--o{ ACCEPTANCE_CRITERIA : "定义"

    SRS ||--|| SDD : "驱动"
    SDD ||--o{ API_CONTRACT : "定义"
    SDD ||--o{ DATA_MODEL : "定义"
    SDD ||--o{ ADR : "记录"

    SDD ||--o{ REVIEW : "提交"
    REVIEW ||--o{ VIOLATION : "发现"
    REVIEW ||--o{ APPROVAL : "获得"

    SDD ||--o{ CODE_COMMIT : "实现"
    CODE_COMMIT ||--o{ UNIT_TEST : "覆盖"
    CODE_COMMIT ||--o{ CRATE : "属于"

    CODE_COMMIT ||--o{ INTEGRATION_TEST : "集成"
    INTEGRATION_TEST ||--o{ TEST_REPORT : "生成"
    TEST_REPORT ||--o{ SECURITY_FINDING : "包含"
    TEST_REPORT ||--o{ PERF_BASELINE : "记录"

    TEST_REPORT ||--|| RELEASE : "通过后发布"
    RELEASE ||--o{ DEPLOYMENT : "部署"
    DEPLOYMENT ||--o{ ROLLBACK : "触发"

    DEPLOYMENT ||--o{ ALERT : "产生"
    ALERT ||--o{ INCIDENT : "升级为"
    INCIDENT ||--o{ POSTMORTEM : "复盘"
    INCIDENT ||--o{ CHANGE_REQUEST : "引发"

    POSTMORTEM ||--o{ LESSON_LEARNED : "总结"
    LESSON_LEARNED ||--o{ BEST_PRACTICE : "提炼"
    LESSON_LEARNED ||--o{ ANTI_PATTERN : "标记"

    PROJECT ||--o{ RETROSPECTIVE : "复盘"
    RETROSPECTIVE ||--o{ KNOWLEDGE_NODE : "沉淀为"
    KNOWLEDGE_NODE ||--o{ KNOWLEDGE_EDGE : "关联"

    %% KG 域实体
    KNOWLEDGE_NODE {
        string id PK
        string label
        string type
        json properties
    }
    KNOWLEDGE_EDGE {
        string id PK
        string source FK
        string target FK
        string label
        float weight
    }

    %% AI 域实体
    INTENT_RESULT {
        string id PK
        string intent_type
        float confidence
        string routed_capability
    }
    CAPABILITY {
        string id PK
        string name
        string domain
        string version
    }

    %% Cloud 域实体
    STORAGE_BUCKET {
        string id PK
        string name
        string region
        string policy
    }
    STORAGE_OBJECT {
        string id PK
        string bucket FK
        string key
        bigint size
        string checksum
    }
    CHUNK {
        string id PK
        string object FK
        int index
        bigint size
        string node_id
    }

    %% Data 域实体
    DATASET {
        string id PK
        string name
        string domain
        string schema
    }
    DATA_LINEAGE {
        string id PK
        string source FK
        string target FK
        string transform_type
    }
```

### 2.2 核心实体关联说明

| 实体 | 关联实体 | 关系类型 | 说明 |
|---|---|---|---|
| Requirement | Project | 1:1 | 需求转化为项目 |
| Project | SRS | 1:N | 一个项目产生多份需求规格 |
| SRS | SDD | 1:1 | 需求驱动系统设计 |
| SDD | CodeCommit | 1:N | 设计文档对应多次代码提交 |
| CodeCommit | TestReport | N:1 | 多次提交汇总为一次测试报告 |
| TestReport | Release | 1:1 | 测试通过后发布 |
| Release | Deployment | 1:N | 一次发布可部署到多环境 |
| Deployment | Alert | 1:N | 部署后产生监控告警 |
| Alert | Incident | N:1 | 多个告警可升级为一个事件 |
| Incident | Postmortem | 1:1 | 每个事件有一份复盘 |
| Postmortem | KnowledgeNode | 1:N | 复盘沉淀为多个知识节点 |
| KnowledgeNode | KnowledgeEdge | N:N | 知识节点间通过边关联 |

---

## 三、跨域调用链路图

### 3.1 典型场景：需求输入到知识沉淀（全链路）

```mermaid
sequenceDiagram
    actor User as 👤 用户
    participant GW as 🌐 Gateway
    participant AI as 🤖 AI引擎
    participant ENT as 🏢 Enterprise
    participant KG as 🕸️ 知识图谱
    participant DATA as 📊 Data
    participant CLOUD as ☁️ Cloud
    participant TEST as 🧪 Test
    participant OBS as 📈 Observability

    %% P0 需求输入
    User->>GW: POST /ai/engine/process {input}
    GW->>AI: classify_intent(request)
    AI->>AI: 8类意图识别 + 置信度计算
    AI->>KG: GET /kg/v1/neighborhood?center={intent}
    KG-->>AI: 相关知识节点
    AI->>DATA: POST /data/v1/norm/intent {intent_result}
    DATA-->>AI: 归一化完成
    AI-->>GW: IntentResponse {intent, confidence, routed_capability}
    GW-->>User: 200 OK {requirement_id}

    %% P1-P4 立项到评审
    User->>GW: POST /enterprise/v1/projects {requirement_id}
    GW->>ENT: 创建项目 + 立项审批
    ENT->>KG: POST /kg/v1/ingest {project_node}
    ENT->>AI: POST /ai/engine/analyze {capability:architecture_analyzer}
    AI-->>ENT: 架构分析结果 + CEM评分
    ENT->>KG: POST /kg/v1/ingest {sdd_node, api_contracts}
    ENT->>ENT: P4 架构评审（红线检查）
    ENT-->>GW: 评审通过 {review_id}

    %% P5-P7 开发到测试
    Note over TEST: CI/CD 触发（Git Webhook）
    TEST->>AI: 代码质量分析
    TEST->>KG: 依赖图谱验证
    TEST->>DATA: 测试数据准备
    TEST->>TEST: 单元测试 + 集成测试 + 性能测试
    TEST-->>GW: 测试报告 {test_report_id}

    %% P8 部署
    GW->>ENT: POST /enterprise/v1/deploy {release_id}
    ENT->>CLOUD: 上传二进制产物
    CLOUD-->>ENT: 存储完成 {object_url}
    ENT->>ENT: 蓝绿部署 + 健康检查
    ENT->>OBS: 注册监控指标
    ENT-->>GW: 部署完成 {deployment_id}

    %% P9-P10 监控运维
    OBS->>OBS: 实时监控 + 告警检测
    OBS->>ENT: 告警触发 {alert_id}
    ENT->>KG: GET /kg/v1/path?src={alert}&dst={root_cause}
    KG-->>ENT: 根因路径
    ENT->>ENT: 故障处置 + Postmortem
    ENT->>CLOUD: 存储 Postmortem 文档

    %% P11-P12 复盘沉淀
    ENT->>KG: POST /kg/v1/ingest {lessons, best_practices, anti_patterns}
    KG->>KG: CNM 社区自动归类
    KG->>AI: PPR 个性化推荐计算
    AI-->>User: 知识推荐（反哺 P0）
```

### 3.2 典型场景：KG 图查询跨域调用

```mermaid
sequenceDiagram
    actor Client as 📱 客户端
    participant GW as 🌐 Gateway
    participant KGSVC as 🕸️ KG Service
    participant ALGO as 🧮 KG Algo Core
    participant STORE as 💾 KG Storage
    participant DATA as 📊 Data
    participant CLOUD as ☁️ Cloud

    Client->>GW: GET /kg/v1/neighborhood?center=P0&depth=2
    GW->>GW: JWT 验证 + RBAC 鉴权
    GW->>GW: 限流检查（resilience）
    GW->>KGSVC: 转发请求

    KGSVC->>STORE: 查询节点 P0 是否存在
    STORE-->>KGSVC: 节点存在

    KGSVC->>ALGO: neighborhood_subgraph("P0", 2, 100)
    ALGO->>ALGO: CSR 邻接表 BFS 扩展
    ALGO->>ALGO: 构建子图（nodes + edges + meta）
    ALGO-->>KGSVC: NeighborhoodResult

    KGSVC->>DATA: 补充节点业务属性（从数据目录）
    DATA-->>KGSVC: 节点属性

    KGSVC->>CLOUD: 如需大文件（图谱快照），获取下载URL
    CLOUD-->>KGSVC: presigned URL（可选）

    KGSVC-->>GW: 200 OK {nodes, edges, meta}
    GW->>GW: metrics 记录（QPS + 延迟）
    GW-->>Client: 200 OK
```

### 3.3 典型场景：AI 意图识别跨域调用

```mermaid
sequenceDiagram
    actor User as 👤 用户
    participant GW as 🌐 Gateway
    participant AISVC as 🤖 AI Service
    participant INTENT as 🧠 AI Intent Core
    participant KG as 🕸️ KG
    participant DATA as 📊 Data

    User->>GW: POST /ai/engine/process {input, context, options}
    GW->>GW: 认证 + 限流 + 日志
    GW->>AISVC: 转发请求

    AISVC->>INTENT: classify_intent(request)
    INTENT->>INTENT: 输入预处理（分词/去噪）
    INTENT->>INTENT: 特征提取（关键词/语义向量）
    INTENT->>INTENT: 8类意图概率计算

    INTENT->>KG: 查询意图相关知识（邻域扩展）
    KG-->>INTENT: 相关能力节点

    INTENT->>INTENT: score_alliance_candidates(intent, context)
    INTENT->>INTENT: 7类能力匹配 + 4维打分
    INTENT->>INTENT: 选择最优能力

    INTENT->>DATA: 记录意图结果（数据归一化）
    DATA-->>INTENT: 记录完成

    INTENT-->>AISVC: IntentResponse {intent, confidence, routed_capability, candidates}
    AISVC-->>GW: 200 OK
    GW-->>User: 200 OK {trace_id}
```

---

## 四、数据流向图

### 4.1 全域数据流向总览

```mermaid
flowchart LR
    subgraph 数据源["数据源"]
        USER_INPUT["用户输入<br/>对话/语音/图片"]
        SYS_LOG["系统日志<br/>应用/基础设施"]
        EXT_DATA["外部数据<br/>API/第三方"]
    end

    subgraph 接入处理["接入与处理"]
        GW["Gateway<br/>路由/认证/限流"]
        VOICE["Voice域<br/>ASR/TTS/DSP"]
        AI["AI域<br/>意图识别/能力路由"]
        DATA_NORM["Data域<br/>归一化/ETL"]
    end

    subgraph 核心存储["核心存储"]
        KG_STORE["KG图谱存储<br/>节点/边/元数据"]
        DATA_STORE["Data存储<br/>SQLite/PostgreSQL"]
        CLOUD_OBJ["Cloud对象存储<br/>S3/大文件/快照"]
        META_STORE["元数据存储<br/>目录/血缘/分类"]
    end

    subgraph 消费输出["消费与输出"]
        SEARCH["搜索/查询<br/>KG查询/数据检索"]
        ANALYTICS["分析/报表<br/>统计/可视化"]
        RECOMMEND["智能推荐<br/>PPR个性化推荐"]
        MONITOR["监控告警<br/>指标/日志/追踪"]
        API_OUT["API输出<br/>JSON/OpenAPI"]
    end

    %% 数据源 → 接入
    USER_INPUT --> GW
    USER_INPUT --> VOICE
    SYS_LOG --> MONITOR
    EXT_DATA --> GW

    %% 接入 → 处理
    GW --> AI
    VOICE --> AI
    AI --> DATA_NORM
    GW --> DATA_NORM

    %% 处理 → 存储
    AI --> KG_STORE
    DATA_NORM --> DATA_STORE
    DATA_NORM --> META_STORE
    AI --> CLOUD_OBJ
    KG_STORE --> CLOUD_OBJ

    %% 存储 → 消费
    KG_STORE --> SEARCH
    DATA_STORE --> SEARCH
    META_STORE --> SEARCH
    KG_STORE --> RECOMMEND
    DATA_STORE --> ANALYTICS
    META_STORE --> ANALYTICS
    CLOUD_OBJ --> API_OUT
    MONITOR --> API_OUT

    %% 回流
    RECOMMEND -.->|知识反哺| AI
    ANALYTICS -.->|模式识别| KG_STORE

    style USER_INPUT fill:#F3E5F5,stroke:#6A1B9A
    style GW fill:#E3F2FD,stroke:#1565C0
    style AI fill:#F3E5F5,stroke:#6A1B9A
    style KG_STORE fill:#E8F5E9,stroke:#2E7D32
    style CLOUD_OBJ fill:#E0F7FA,stroke:#00838F
    style RECOMMEND fill:#FFF8E1,stroke:#F57F17
```

### 4.2 KG 图谱数据生命周期

```mermaid
stateDiagram-v2
    [*] --> 数据采集: 多渠道输入
    数据采集 --> 归一化: AI意图识别 + 数据抽取
    归一化 --> 实体抽取: NER + 关系抽取
    实体抽取 --> 图谱构建: 节点创建 + 边关联
    图谱构建 --> 质量校验: 重复检测 + 一致性检查
    质量校验 --> 图谱存储: CSR压缩 + 元数据索引
    图谱存储 --> 图计算: PageRank/CNM/Brandes/harmonic
    图计算 --> 知识服务: 邻域查询/路径查找/社区检测
    知识服务 --> 智能推荐: PPR个性化推荐
    智能推荐 --> 数据采集: 知识反哺（闭环）

    质量校验 --> 数据采集: 校验失败回退
    图谱存储 --> 快照归档: 定时快照
    快照归档 --> [*]: 冷存储
```

---

## 五、域间依赖矩阵

### 5.1 编译期依赖矩阵

| 源域 \ 目标域 | AI | KG | Flow | Cloud | Data | Voice | Market | Streams | Platform | Framework |
|---|---|---|---|---|---|---|---|---|---|---|
| **AI** | - | ✅ | - | - | ✅ | - | - | - | - | ✅ |
| **KG** | - | - | - | ✅ | ✅ | - | - | - | - | ✅ |
| **Flow** | ✅ | ✅ | - | - | - | - | - | - | - | ✅ |
| **Cloud** | - | - | - | - | ✅ | - | - | - | - | ✅ |
| **Data** | - | - | - | - | - | - | - | ✅ | - | ✅ |
| **Voice** | ✅ | - | - | - | - | - | - | - | - | ✅ |
| **Market** | - | - | - | ✅ | ✅ | - | - | - | - | ✅ |
| **Streams** | - | ✅ | - | - | ✅ | - | - | - | - | ✅ |
| **Platform** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | - | ✅ |
| **Gateway** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

> ✅ = 编译期依赖（Cargo.toml 中声明）；- = 无直接依赖

### 5.2 运行期调用矩阵

| 源域 \ 目标域 | AI | KG | Flow | Cloud | Data | Voice | Market | Streams | Platform | Observability |
|---|---|---|---|---|---|---|---|---|---|---|
| **AI** | - | 🔄 | - | - | 🔄 | - | - | - | - | 📊 |
| **KG** | - | - | - | 🔄 | 🔄 | - | - | - | - | 📊 |
| **Flow** | 🔄 | 🔄 | - | - | - | - | - | 🔄 | - | 📊 |
| **Cloud** | - | - | - | - | 🔄 | - | - | 🔄 | - | 📊 |
| **Data** | - | - | - | - | - | - | - | 🔄 | - | 📊 |
| **Voice** | 🔄 | - | - | - | - | - | - | - | - | 📊 |
| **Market** | - | - | - | 🔄 | 🔄 | - | - | - | - | 📊 |
| **Streams** | - | 🔄 | - | - | 🔄 | - | - | - | - | 📊 |
| **Platform** | 🔄 | 🔄 | 🔄 | 🔄 | 🔄 | 🔄 | 🔄 | 🔄 | - | 📊 |
| **Gateway** | 🔄 | 🔄 | 🔄 | 🔄 | 🔄 | 🔄 | 🔄 | 🔄 | 🔄 | 📊 |

> 🔄 = 运行期 HTTP/gRPC 调用；📊 = 指标/日志/追踪上报

---

## 六、模块化边界规则

### 6.1 域间通信规则

1. **必须通过 trait 接口**：域间通信必须通过定义在 foundation 层的 trait，禁止直接引用其他域的 impl
2. **禁止循环依赖**：A→B 且 B→A 为循环依赖，必须通过第三方域或事件总线解耦
3. **数据传输用 DTO**：跨域数据传输必须使用 DTO（Data Transfer Object），禁止直接传递内部实体
4. **错误码域隔离**：每个域有独立的错误码前缀（AI=4xxx, KG=5xxx, ...），便于问题定位

### 6.2 分层依赖规则

```
L0 接入层 → L1 网关 → L2 编排 → L3 服务 → L4 内核 → L5 基础
    ↑ 禁止反向依赖 ↑
```

- 上层可依赖下层，下层不可依赖上层
- L4 算法内核仅依赖 L5 基础层，不依赖任何业务服务
- L3 服务域间通过 L5 foundation trait 通信

### 6.3 模块拆分原则

| 原则 | 说明 |
|---|---|
| 单一职责 | 每个 crate 只负责一个明确的业务能力 |
| 高内聚 | 相关功能放在同一个 crate |
| 低耦合 | 域间通过 trait 接口，不直接依赖实现 |
| 可独立测试 | 每个 crate 可独立编写单元测试 |
| 可独立部署 | 模块化单体可按域拆分为微服务（ADR-16） |

---

*详见 [01-architecture-overview.md](./01-architecture-overview.md) 获取分层架构详解，[02-business-flow.md](./02-business-flow.md) 获取 P0-P12 业务流程图。*
