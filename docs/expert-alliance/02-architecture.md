---
doc_id: EA-ARCH-001
title: 专家联盟系统总体架构
version: V1.0
authority: 🟢权威
last_updated: 2026-09-25
source_of_truth: 代码事实（platform/domains/alliance/ + gateway/src/alliance/）
---

# 专家联盟系统总体架构

> 状态：V1.0 目标态，部分结论已被 CURRENT-ARCHITECTURE.md V1.1 取代（见文中 ⚠️ 补记）。

## 1. 架构总览

### 1.1 设计原则

| 原则 | 说明 |
|------|------|
| **Rust 原生** | 核心全 Rust，无 GC，单二进制部署 |
| **Workspace 多模块** | 每个 crate 独立编译单元，天然隔离 |
| **Trait 驱动** | 所有可扩展点通过 Trait 抽象，可替换可 Mock |
| **契约先行** | proto 层定义 gRPC 契约，服务端客户端独立开发 |
| **无状态优先** | 执行器全内存无状态，状态外部化到 SQLite/Redis |
| **网关内联 + 独立服务混合** | 高频核心走网关进程内调用，重型调度独立部署 |

### 1.2 六层分层架构

```
┌─────────────────────────────────────────────────────────────┐
│  L6 接入层  gateway:3080（REST / WebSocket / SSE）            │
├─────────────────────────────────────────────────────────────┤
│  L5 网关内联模块  gateway/src/alliance/（11个.rs, 8680行）    │
│    experts_collaboration / experts_orchestration /           │
│    experts_dispatcher / experts_graph / experts_common /      │
│    experts_registry / experts_session / experts_db /          │
│    experts_ext / registry_client / mod                        │
├─────────────────────────────────────────────────────────────┤
│  L4 独立服务层  3个可部署服务                                   │
│    scheduler-svc:3100（任务调度/专家匹配/计划生成）            │
│    executor-svc:3200（DAG执行/节点调度/进度推送）              │
│    registry-svc:3400（专家注册中心/分级心跳聚合）               │
├─────────────────────────────────────────────────────────────┤
│  L3 核心层  6个 core crates（纯算法，无 IO）                   │
│    mox-alliance-core（DAG拓扑 + 6种融合策略）                  │
│    mox-alliance-scheduler-core（调度器业务逻辑）               │
│    mox-alliance-executor-core（执行器业务逻辑）                 │
│    mox-alliance-config-core（10大领域专家配置 + LLM路由）       │
│    mox-alliance-registry-core（注册核心）                       │
│    mox-alliance-boot-config（Nacos/命名/启动配置）              │
├─────────────────────────────────────────────────────────────┤
│  L2 契约层  4个 proto crates（协议先行）                        │
│    mox-alliance-common-proto（共享类型/错误码/Traits）          │
│    mox-alliance-scheduler-proto（调度器契约）                   │
│    mox-alliance-executor-proto（执行器契约）                     │
│    mox-alliance-registry-proto（注册中心契约）                   │
├─────────────────────────────────────────────────────────────┤
│  L1 SDK层  2个 SDK crates                                      │
│    mox-alliance-sdk（客户端 SDK）                               │
│    mox-alliance-http-sdk（HTTP 客户端 SDK）                     │
└─────────────────────────────────────────────────────────────┘
```

> ⚠️ **V1.1 核对补记（2026-09-29）**：本文此处所述的 `/ws/v1/*` WebSocket 推送在实现中不存在（全 crate `WebSocketUpgrade` 零命中）；实时性由 SSE `GET /api/alliance/tasks/:id/logs/stream` 承担。以 CURRENT-ARCHITECTURE.md V1.1 为准。

### 1.3 物理代码分布

```
platform/domains/alliance/
├── api/                          # DTO 定义（mox-alliance-api crate）
├── core/                         # 6个核心算法 crates
├── proto/                        # 4个契约 crates
├── sdk/                          # 2个客户端 SDK
├── svc/                          # 3个可独立部署的服务
│   ├── mox-alliance-scheduler-svc/  # :3100
│   ├── mox-alliance-executor-svc/   # :3200
│   └── mox-alliance-registry-svc/   # :3400
└── tools/rnacos/                 # 工具

platform/gateway/mox-platform-gateway-svc/src/alliance/
├── experts_collaboration.rs      # 2227行 - 协作编排/多轮辩论/结果融合
├── experts_orchestration.rs      # 1236行 - 拓扑排序/DAG预演/进程内台账
├── experts_dispatcher.rs         # 1173行 - 任务分发/专家路由/审计
├── experts_graph.rs              # 1099行 - 图谱关联/能力查询
├── experts_common.rs             # 1014行 - 共享状态/工具函数
├── experts_registry.rs           # 1009行 - 专家CRUD/可用性状态
├── experts_session.rs             # 959行 - 会话管理/多轮历史
├── experts_db.rs                  # 721行 - SQLite持久化
├── experts_ext.rs                 # 366行 - 扩展点/预约
├── registry_client.rs            # 92行 - 远程注册中心客户端
└── mod.rs                        # 28行 - 模块声明
```

---

## 2. 服务拓扑与调用关系

### 2.1 运行时进程

| 进程 | 端口 | 职责 | 部署方式 |
|------|------|------|---------|
| platform-gateway-svc | 3080 | 统一入口：REST/WS/内联专家逻辑 | Deployment |
| alliance-scheduler-svc | 3100 | 任务调度、专家匹配、计划生成 | Deployment |
| alliance-executor-svc | 3200 | DAG执行、节点调度、进度推送 | Deployment |
| alliance-registry-svc | 3400 | 专家注册中心、分级心跳聚合 | Deployment |

### 2.2 进程间调用链

```
用户/前端
    ↓ REST/WebSocket
gateway:3080
    ↓ 进程内调用
alliance/ 模块（专家注册/会话/协作/图谱/编排）
    ↓ HTTP
scheduler-svc:3100（匹配/计划/排队）
    ↓ HTTP
executor-svc:3200（DAG执行/节点调度）
    ↓
底层微服务（AI/图谱/搜索/存储等）
```

> ⚠️ **V1.1 核对补记（2026-09-29）**：本文此处所述的 `/ws/v1/*` WebSocket 推送在实现中不存在（全 crate `WebSocketUpgrade` 零命中）；实时性由 SSE `GET /api/alliance/tasks/:id/logs/stream` 承担。以 CURRENT-ARCHITECTURE.md V1.1 为准。

**关键说明：**
- 当前服务间通信为 **HTTP 短调用**，非 gRPC 长连接
- gRPC :50051 端口在 framework 层保留，但专家联盟域未使用
- 网关内联模块与独立服务之间通过 HTTP 调用，不是进程内直调

---

## 3. 核心组件职责

### 3.1 网关内联模块

| 模块 | 职责 | 关键方法 |
|------|------|---------|
| experts_collaboration.rs | 协作编排核心 | run_debate() 多轮辩论循环 |
| experts_orchestration.rs | DAG 拓扑管理 | topological_sort() Kahn算法+环检测 |
| experts_dispatcher.rs | 任务分发 | 专家路由、调用转发、审计记录 |
| experts_graph.rs | 知识图谱关联 | 专家关系网络、能力图谱查询 |
| experts_registry.rs | 专家注册 | CRUD、availability.status 登记值 |
| experts_session.rs | 会话管理 | 对话上下文、多轮历史 |
| experts_db.rs | 持久化 | SQLite 存储（IamRepository） |

### 3.2 独立服务核心

| 服务 | 核心组件 | 职责 |
|------|---------|------|
| scheduler-svc | RuleBasedExpertMatcher | 5维评分专家匹配 |
| | SimplePlanGenerator | 7种模式 DAG 计划生成 |
| | Synchronizer | 专家状态同步到注册中心 |
| executor-svc | DAG Executor | 拓扑排序执行、节点调度、进度推送 |
| registry-svc | Registry Core | 专家 CRUD、分级心跳聚合 |

---

## 4. 架构归一化

### 4.1 分层依赖规则

| 规则 | 说明 |
|------|------|
| 单向依赖 | 上层可依赖下层，下层不可依赖上层 |
| 同层隔离 | L4 各服务之间通过 HTTP 调用，不直接依赖内部实现 |
| 领域隔离 | alliance 域不依赖其他业务域的内部实现，只通过 SDK/API 调用 |
| 基础纯净 | L2/L3 只依赖第三方库 + 同层 proto，不依赖业务模块 |

### 4.2 Crate 依赖矩阵

```
gateway-svc ──→ alliance-api (DTO)
            ──→ alliance-http-sdk
            ──→ experts_db (SQLite)

scheduler-svc ──→ scheduler-core
              ──→ scheduler-proto
              ──→ common-proto

executor-svc ──→ executor-core
             ──→ executor-proto
             ──→ common-proto

registry-svc ──→ registry-core
             ──→ registry-proto
             ──→ common-proto

*-core ──→ *-proto
       ──→ common-proto

*-proto ──→ 无业务依赖（纯类型定义）
```

---

## 5. 技术选型

| 层 | 技术 | 版本 |
|---|------|------|
| Web 框架 | axum | 0.7+ |
| 异步运行时 | tokio | 1.x |
| gRPC | tonic | 0.10+ |
| 数据库 | sqlx（SQLite/PostgreSQL） | 0.7+ |
| 序列化 | serde / serde_json | 1.x |
| 日志 | tracing + tracing-subscriber | 0.3+ |
| 错误处理 | thiserror + anyhow | 1.x |
| 配置 | config-rs + serde | 0.13+ |
| ID 生成 | uuid | 1.x |
| 并发原语 | parking_lot::RwLock | 0.12+ |

---

## 6. 演进路线

| 阶段 | 目标 | 当前状态 |
|------|------|---------|
| Phase 1 | 单机演示：网关内联 + 3个独立服务 | ✅ 已落地 |
| Phase 2 | 生产级：PostgreSQL + Redis + gRPC 长连接 | 🔄 规划中 |
| Phase 3 | 全微服务化：7服务拆分（fusion/memory 独立） | 📋 路线图 |
| Phase 4 | 云原生：K8s + HPA + 多租户隔离 | 📋 路线图 |
