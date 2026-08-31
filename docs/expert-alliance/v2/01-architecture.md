---
title: 01 - 企业级架构设计
version: V2.0
authority: 🟢权威
doc_id: EA-DOC-011
last_updated: 2026-08-31
source_of_truth: V2.0目标架构设计（未落地）
---

# 01 - 企业级架构设计

> 版本：v2.0 | 日期：2026-08-26 | 状态：企业级草案
>
> 前置：[00-全维需求分析](docs/expert-alliance/v2/00-requirements.md)


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:8081 / executor-svc:8082），10个内置领域专家，任务仓库为内存+文件快照。

---

## 一、架构总览

### 1.1 设计原则

| 原则 | 说明 |
|------|------|
| **单一职责** | 每个服务/专家只负责一个明确的领域 |
| **独立部署** | 每个服务可独立部署、独立扩缩、独立回滚 |
| **API 优先** | 先定义接口契约（.proto），再实现 |
| **故障隔离** | 单服务/单专家故障不影响整体系统 |
| **渐进式拆分** | 从模块化单体起步，逐步拆分为微服务 |
| **知识图谱驱动** | 专家匹配/协作编排/结果融合全部基于图谱关联关系 |
| **多协议共存** | gRPC（内部）+ JSON-RPC/MCP（对外）+ REST（兼容）+ WebSocket（实时） |
| **全维 Rust** | 后端核心用 Rust，AI 推理用 Python sidecar |

> 术语注释：前文架构图中"联盟调度器"指专家匹配与任务调度组件，对应代码实体 `TaskScheduler` trait。

### 1.2 七层架构模型

```
┌─────────────────────────────────────────────────────────────────────┐
│  L7  接入层（Gateway）                                                │
│  多协议单端口：REST / gRPC / JSON-RPC / MCP / WebSocket              │
│  认证 / 限流 / 路由 / 协议转码 / 租户解析                              │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │ gRPC / 事件
┌──────────────────────────────────▼──────────────────────────────────┐
│  L6  专家联盟层（Expert Alliance）— 新增                              │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐  │
│  │ 联盟调度器   │ │ 协作编排引擎 │ │ 结果融合器   │ │ 协作记忆    │  │
│  │ (alliance)  │ │ (orchestrator)│ │ (fusion)    │ │ (memory)    │  │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘  │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────────┐    │
│  │ 专家注册中心 │ │ 专家Agent运行时│ │ 知识图谱关联引擎            │    │
│  │ (registry)  │ │ (agent-runtime)│ │ (kg-engine)                │    │
│  └─────────────┘ └─────────────┘ └─────────────────────────────┘    │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │ gRPC
┌──────────────────────────────────▼──────────────────────────────────┐
│  L5  业务服务层（Business Services）— 现有31个微服务                   │
│  AI引擎 / 知识图谱 / 数据存储 / 流程算子 / 业务治理 / 平台能力         │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
┌──────────────────────────────────▼──────────────────────────────────┐
│  L4  共享库层（Shared Libraries）                                      │
│  mox-rpc / mox-config / mox-o11y / mox-db / mox-tenant /            │
│  mox-auth / mox-resilience / mox-mcp / mox-expert-core              │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
┌──────────────────────────────────▼──────────────────────────────────┐
│  L3  数据层（Data）                                                    │
│  PostgreSQL / 自研图存储(RocksDB+Raft) / Redis / NATS JetStream /    │
│  MinIO / pgvector / TimescaleDB                                        │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
┌──────────────────────────────────▼──────────────────────────────────┐
│  L2  容器编排层（Container Orchestration）                             │
│  Kubernetes / Deployment / StatefulSet / HPA / PDB / Service /        │
│  ConfigMap / Secret / Ingress / Istio(可选)                           │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
┌──────────────────────────────────▼──────────────────────────────────┐
│  L1  基础设施层（Infrastructure）                                      │
│  计算 / 存储 / 网络 / 负载均衡 / DNS / TLS / 监控 / 日志 / CI/CD      │
└─────────────────────────────────────────────────────────────────────┘
```


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:8081 / executor-svc:8082），10个内置领域专家，任务仓库为内存+文件快照。

---

## 二、专家联盟服务拆分

### 2.1 服务清单

专家联盟在现有31个微服务基础上，新增 **5 个服务** + **1 个 Sidecar**：

| 服务 | 职责 | 类型 | 部署 | 扩缩依据 |
|------|------|------|------|----------|
| **mox-gateway-svc** | 多协议接入/认证/限流/路由/协议转码 | 接入层 | Deployment | QPS |
| **mox-expert-alliance-svc** | 联盟调度/协作编排/执行/结果融合/记忆 | 核心服务 | Deployment | 任务队列长度 |
| **mox-expert-registry-svc** | 专家注册/发现/匹配/健康检查/工具注册 | 平台服务 | Deployment | QPS（缓存为主） |
| **mox-expert-agent-svc** | 专家Agent运行时/工具调用/AI推理/知识检索 | 运行时服务 | Deployment | 并发Agent数 |
| **mox-expert-kg-svc** | 专家联盟知识图谱/关联推理/案例库/持续学习 | 核心服务 | Deployment | 图谱查询QPS |
| **ai-inference-sidecar** | Python AI推理（与agent-svc同Pod） | Sidecar | 同Pod | - |

### 2.2 服务职责详细定义

#### mox-expert-alliance-svc（联盟核心）

```
职责：
  ├── 任务接收与解析（自然语言→结构化任务描述）
  ├── 专家识别与匹配（调用 registry-svc + 图谱推理）
  ├── 协作计划生成（DAG：节点=专家调用，边=数据依赖）
  ├── 协作执行引擎（DAG调度/并行执行/依赖管理/状态追踪）
  ├── 结果融合（6种策略：投票/加权/拼接/择优/辩论/迭代）
  ├── 协作记忆管理（工作记忆/会话记忆/长期记忆/案例库）
  ├── 任务进度实时推送（WebSocket/SSE）
  ├── 人工干预处理（暂停/修改计划/指定专家/跳过节点）
  └── 任务事件发布（NATS：task.created/progress/completed/failed）

数据：
  ├── 任务表（tasks）：PostgreSQL
  ├── 节点执行表（task_nodes）：PostgreSQL
  ├── 工作记忆：Redis（任务级 TTL）
  ├── 会话记忆：Redis（用户级 TTL 24h）
  └── 案例库：通过 kg-svc 写入知识图谱

依赖：
  ├── mox-expert-registry-svc（专家匹配）
  ├── mox-expert-agent-svc（节点执行）
  ├── mox-expert-kg-svc（图谱推理/案例检索）
  ├── mox-tenant-svc（租户配额）
  ├── mox-metering-svc（用量计量）
  └── mox-notification-svc（通知）
```

#### mox-expert-registry-svc（专家注册中心）

```
职责：
  ├── 专家 CRUD（注册/更新/注销/查询）
  ├── 专家定义验证（工具存在性/能力完整性/命名冲突）
  ├── 专家版本管理（semver/灰度发布）
  ├── 专家健康检查（心跳/成功率/延迟/错误率）
  ├── 专家搜索（按领域/能力/名称/状态）
  ├── 专家匹配（按任务描述→图谱推理→评分排序）
  ├── 工具自动注册（从 gRPC 服务反射发现工具）
  ├── 领域树管理（Domain 树形结构）
  ├── 能力定义管理（Capability 定义）
  └── 注册事件发布（NATS：expert.registered/updated/deregistered）

数据：
  ├── 专家表（experts）：PostgreSQL
  ├── 能力表（capabilities）：PostgreSQL
  ├── 工具表（tools）：PostgreSQL
  ├── 领域表（domains）：PostgreSQL
  ├── 关联表（expert_capabilities/expert_domains/capability_tools）：PostgreSQL
  ├── 健康状态表（expert_health）：PostgreSQL
  └── 本地缓存（专家列表/匹配结果）：Redis

依赖：
  ├── mox-expert-kg-svc（图谱同步：专家/能力/领域/工具节点）
  ├── gRPC 服务反射（工具自动发现）
  └── mox-tenant-svc（租户隔离）
```

#### mox-expert-agent-svc（专家Agent运行时）

```
职责：
  ├── 专家 Agent 实例管理（创建/复用/销毁）
  ├── Agent 执行循环（ReAct：理解→规划→执行→观察→审核）
  ├── 工具调用器（gRPC 调用底层微服务，带超时/重试/熔断）
  ├── AI 推理调用（通过 ai-inference-sidecar，支持流式）
  ├── 知识检索（图谱查询/语义搜索/向量检索）
  ├── 工作记忆管理（当前任务上下文/中间结果）
  ├── 专家思考过程记录（可解释性）
  ├── 流式输出（AI生成过程实时推送）
  ├── 工具调用结果观察与反思（失败后自动调整）
  └── Agent 实例池化（复用减少初始化开销）

数据：
  ├── Agent 实例状态：内存（无状态，会话存在 Redis）
  ├── 工具调用日志：通过 o11y 上报
  └── 专家思考过程：写入 task_nodes 表（通过 alliance-svc）

依赖：
  ├── ai-inference-sidecar（Python AI推理，Unix Domain Socket）
  ├── 所有底层 gRPC 服务（工具调用）
  ├── mox-expert-kg-svc（知识检索）
  ├── mox-search-svc（语义搜索）
  └── mox-tenant-svc（租户上下文）
```

#### mox-expert-kg-svc（专家联盟知识图谱）

```
职责：
  ├── 六元关联图谱管理（Expert/Capability/Domain/Tool/Data/Case 顶点）
  ├── 12种关联边管理（has_capability/operates_in/requires_tool/...）
  ├── 图谱推理（专家匹配/协作组合推荐/案例检索/相似案例）
  ├── 图谱遍历（多跳查询/BFS/DFS/路径查询）
  ├── 案例库管理（案例提升/检索/相似度计算）
  ├── 持续学习（任务完成→更新边权重/统计）
  ├── 图谱初始化（领域树/能力定义/工具注册/内置专家）
  ├── 图谱工具自动注册（从 gRPC 反射创建 Tool 节点）
  ├── 图谱多租户隔离（系统专家共享，自定义按租户）
  └── 图谱变更事件发布（NATS：kg.vertex.created/kg.edge.updated）

数据：
  ├── 图谱数据：自研图存储（mox-graph-storage-svc，RocksDB+Raft）
  ├── 图谱 Schema/本体：mox-graph-meta-svc
  └── 图谱查询缓存：Redis

依赖：
  ├── mox-graph-storage-svc（底层图存储，gRPC调用）
  ├── mox-graph-meta-svc（Schema管理）
  └── mox-tenant-svc（租户隔离：VID前缀方案）
```

### 2.3 服务间调用关系

```
                    ┌─────────────────────┐
                    │  mox-gateway-svc    │
                    │  (多协议接入)        │
                    └──────────┬──────────┘
                               │ gRPC
                    ┌──────────▼──────────┐
                    │ mox-expert-alliance  │
                    │ (联盟核心)            │
                    └──┬───────┬───────┬──┘
                       │       │       │
              gRPC     │       │ gRPC  │ gRPC
                       ▼       ▼       ▼
              ┌──────────┐ ┌────────┐ ┌──────────┐
              │ registry │ │ agent  │ │   kg     │
              │ (注册中心)│ │(运行时) │ │(知识图谱) │
              └────┬─────┘ └───┬────┘ └────┬─────┘
                   │            │            │
                   │ gRPC       │ gRPC       │ gRPC
                   ▼            ▼            ▼
              ┌─────────────────────────────────────┐
              │     现有31个微服务（底层能力）        │
              │  ai / graph / storage / flow / ...  │
              └─────────────────────────────────────┘
```


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:8081 / executor-svc:8082），10个内置领域专家，任务仓库为内存+文件快照。

---

## 三、多协议网关架构

### 3.1 单端口多协议

```
客户端请求 → :8080
              │
              ├─ Content-Type: application/grpc
              │    → gRPC Handler（tonic，内部服务间）
              │
              ├─ Path: /rpc + Content-Type: application/json
              │    → JSON-RPC Handler（jsonrpsee，对外灵活API）
              │
              ├─ Path: /mcp + Content-Type: application/json
              │    → MCP Handler（JSON-RPC子集，AI工具调用）
              │
              ├─ Path: /api/v1/*
              │    → REST Handler（axum，兼容现有前端）
              │
              ├─ Upgrade: websocket + Path: /ws
              │    → WebSocket Handler（实时进度/流式输出）
              │
              ├─ Path: /metrics
              │    → Prometheus Handler
              │
              └─ Path: /health
                   → Health Check Handler
```

### 3.2 协议转码层

```
JSON-RPC / MCP 请求
    │
    ▼
┌─────────────────────────────────────┐
│  协议转码层（Gateway 内置）            │
│                                       │
│  1. method 解析：                     │
│     JSON-RPC: "graph.VertexService.GetVertex"
│     MCP: "tools/call" + params.name="graph.create_vertex"
│                                       │
│  2. 查转码路由表（从 .proto 生成）：   │
│     jsonrpc_method → grpc_service + grpc_method
│                                       │
│  3. JSON params → Protobuf message：  │
│     serde_json → prost message        │
│                                       │
│  4. 调用 gRPC 后端（tonic 客户端）：   │
│     带拦截器链（租户→认证→Trace→限流） │
│                                       │
│  5. Protobuf response → JSON：        │
│     prost message → serde_json        │
│                                       │
│  6. 包装为协议响应：                   │
│     JSON-RPC: { jsonrpc, result, id } │
│     MCP: { content: [{type:text, text}], isError } │
└─────────────────────────────────────┘
```

### 3.3 MCP 工具自动发现

```
网关启动 / 定时刷新
    │
    ▼
┌─────────────────────────────────────┐
│  MCP 工具自动发现                      │
│                                       │
│  1. 服务发现：                         │
│     K8s Service / Nacos → 所有 gRPC 服务列表
│                                       │
│  2. gRPC Server Reflection：          │
│     调用每个服务的 reflection API     │
│     获取 .proto 文件描述（service + method + message）│
│                                       │
│  3. 筛选可暴露为工具的方法：           │
│     - 方法标注了 [tool] 选项（推荐）  │
│     - 或按命名约定（如 *Service/*）   │
│     - 或白名单配置                     │
│                                       │
│  4. 生成 MCP Tool 描述：              │
│     name: "{service}.{method}"        │
│     description: 从 proto 注释提取     │
│     inputSchema: 从 request message   │
│       生成 JSON Schema                 │
│                                       │
│  5. 缓存工具列表：                     │
│     Redis + 本地内存缓存               │
│     TTL 5分钟，支持主动刷新            │
│                                       │
│  6. 响应 MCP tools/list：             │
│     返回缓存的工具列表                 │
└─────────────────────────────────────┘
```


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:8081 / executor-svc:8082），10个内置领域专家，任务仓库为内存+文件快照。

---

## 四、部署架构

### 4.1 K8s 部署单元

| 服务 | Workload | 副本 | 资源 | 存储 | HPA | PDB |
|------|----------|------|------|------|-----|-----|
| mox-gateway-svc | Deployment | 3 | 2C4G | 无 | CPU>60% | minAvailable=2 |
| mox-expert-alliance-svc | Deployment | 3 | 4C8G | 无 | 队列长度>50 | minAvailable=2 |
| mox-expert-registry-svc | Deployment | 2 | 2C4G | 无 | CPU>60% | minAvailable=1 |
| mox-expert-agent-svc | Deployment | 3+ | 4C8G | 无 | 并发Agent>20 | minAvailable=2 |
| mox-expert-kg-svc | Deployment | 3 | 2C4G | 无 | QPS>100 | minAvailable=2 |
| ai-inference-sidecar | Sidecar | 同agent | 4C8G | 无 | - | - |

### 4.2 有状态依赖

| 依赖 | 部署 | 说明 |
|------|------|------|
| PostgreSQL | StatefulSet | 任务/专家/注册数据 |
| 自研图存储 | StatefulSet | 专家联盟知识图谱（RocksDB+Raft+PVC） |
| Redis | StatefulSet | 缓存/会话/工作记忆/限流 |
| NATS JetStream | StatefulSet | 事件总线/消息队列 |
| MinIO | StatefulSet | 对象存储（任务结果/导出文件） |

### 4.3 网络策略

```
网关（Ingress）
  │
  ├─→ gateway-svc（:8080，多协议）
  │     │
  │     ├─→ alliance-svc（:50051 gRPC）
  │     │     ├─→ registry-svc（:50051）
  │     │     ├─→ agent-svc（:50051）
  │     │     └─→ kg-svc（:50051）
  │     │           └─→ graph-storage-svc（:50051）
  │     │
  │     └─→ 直接调用底层服务（REST→gRPC转码）
  │
  └─→ 监控（:9090 Prometheus / :3000 Grafana）

NetworkPolicy：
  - 网关只能访问 alliance/registry/agent/kg + 底层服务
  - alliance 只能访问 registry/agent/kg + tenant/metering/notification
  - agent 只能访问底层服务 + kg + search + sidecar
  - kg 只能访问 graph-storage + graph-meta
  - 所有服务只能访问 PostgreSQL/Redis/NATS/MinIO（对应端口）
```


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:8081 / executor-svc:8082），10个内置领域专家，任务仓库为内存+文件快照。

---

## 五、与现有架构的集成

### 5.1 复用现有服务

专家联盟不重复开发底层能力，全部通过 gRPC 调用现有31个微服务：

| 联盟能力 | 调用的现有服务 | 调用方式 |
|----------|---------------|----------|
| AI 推理/生成/RAG | mox-ai-svc | gRPC + sidecar流式 |
| 图谱查询/操作 | mox-graph-svc | gRPC |
| 底层图存储 | mox-graph-storage-svc | gRPC |
| 图算法 | mox-graph-algo-svc | gRPC |
| 语义/向量搜索 | mox-search-svc | gRPC |
| 工作流执行 | mox-flow-svc | gRPC |
| 数据处理/ETL | mox-etl-svc | gRPC |
| 对象存储 | mox-storage-svc | gRPC |
| 数据治理/审计 | mox-compliance-svc | gRPC |
| 数据融合 | mox-fusion-svc | gRPC |
| 数据目录 | mox-catalog-svc | gRPC |
| 认证授权 | mox-auth-svc | gRPC |
| 租户管理 | mox-tenant-svc | gRPC |
| 用量计量 | mox-metering-svc | gRPC |
| 通知 | mox-notification-svc | gRPC |

### 5.2 共享库扩展

在现有共享库基础上，新增专家联盟专用库：

| 库 | 职责 | 复用 |
|----|------|------|
| **mox-expert-core** | 专家定义/Agent trait/工具调用/记忆接口 | 依赖 mox-rpc/mox-config |
| **mox-mcp** | MCP 协议实现（标准方法/工具描述/转码） | 依赖 jsonrpsee/mox-rpc |
| **mox-alliance-client** | 联盟 SDK（任务创建/进度订阅/结果获取） | 依赖 mox-rpc |


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:8081 / executor-svc:8082），10个内置领域专家，任务仓库为内存+文件快照。

---

## 六、架构演进路线

```
Phase 0（当前）：模块化单体
  - 所有能力在 mox-server 单二进制中
  - 无 gRPC，无多租户

Phase 1（W1-4）：基础建设
  - 共享库（mox-rpc/mox-config/mox-o11y/...）
  - Proto 定义
  - K8s 环境

Phase 2（W5-10）：核心拆分
  - 网关/平台能力/图存储gRPC化/AI服务
  - 多租户 L1 隔离

Phase 3（W11-16）：服务化推进
  - 所有31个服务拆分
  - 异步事件通信

Phase 4（W17-20）：企业级增强
  - 可观测性/安全/弹性/灰度

Phase 5（W21-24）：专家联盟上线 ★
  - 5个新服务部署
  - 10个内置专家注册
  - 知识图谱初始化
  - 多协议网关（含MCP）
  - 5个核心场景验证

Phase 6（W25+）：持续优化
  - 新专家扩展
  - 案例库积累
  - 持续学习优化
  - 性能调优
```

---

*下一篇：[02-归一化领域模型](docs/expert-alliance/v2/02-domain-model.md)*
