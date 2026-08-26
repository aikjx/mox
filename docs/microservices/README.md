# 微服务独立部署架构优化方案

> 版本：v1.0 | 日期：2026-08-26 | 状态：草案
>
> 适用范围：infotopograph / 璇玑（Mox）全栈平台后端

## 文档导航

| 序号 | 文档 | 核心内容 |
|------|------|----------|
| 00 | [核心原则](./00-principles.md) | 独立部署的 12 条铁律、康威定律、领域驱动设计 |
| 01 | [服务边界优化](./01-service-boundaries.md) | 36 个服务的边界重划、合并拆分建议、Bounded Context |
| 02 | [通信架构优化](./02-communication.md) | gRPC 优先、服务间调用拓扑、事件驱动、API 网关、契约管理 |
| 03 | [数据架构优化](./03-data.md) | Database per Service、数据一致性、Saga、CQRS、自研图存储多租户 |
| 04 | [部署架构优化](./04-deployment.md) | K8s 部署规范、HPA、PDB、优雅启停、配置中心、服务发现、CI/CD |
| 05 | [可观测性·安全·弹性](./05-observability-security-resilience.md) | 三大支柱、零信任、mTLS、熔断降级限流、混沌工程 |
| 06 | [实施路线图](./06-roadmap.md) | 6 阶段 24 周实施计划、风险矩阵、验收标准 |

## 为什么要独立部署

当前 infotopograph 后端采用 **Cargo Workspace 单体构建**（`mox-server` single-binary），36 个服务模块编译进一个二进制。这种模式在开发初期效率高，但随着服务数量增长和业务复杂度提升，暴露以下问题：

| 问题 | 影响 | 严重度 |
|------|------|--------|
| **单点部署** | 任何一个服务的变更都需要全量重新部署，发布风险大 | 🔴 高 |
| **无法独立扩缩容** | AI 服务高并发时只能整体扩容，浪费资源（图谱/存储服务不需要扩容） | 🔴 高 |
| **故障爆炸半径大** | 一个服务 OOM 会导致整个二进制崩溃，所有服务不可用 | 🔴 高 |
| **技术栈绑定** | 所有服务必须用相同 Rust 版本、相同依赖版本，无法按需升级 | 🟡 中 |
| **团队协作阻塞** | 多个团队同时修改不同服务，合并冲突频繁，发布需要协调 | 🟡 中 |
| **启动慢** | 单二进制加载所有服务初始化逻辑，冷启动时间长（>30s） | 🟡 中 |
| **资源隔离差** | CPU/内存争抢，AI 推理占满 GPU 时影响其他服务 | 🟡 中 |

**独立部署的微服务架构**是解决以上问题的标准方案，也是企业级平台的必由之路。

## 架构演进路线

```
阶段一：单体 Workspace（当前）
  mox-server single-binary，36 模块编译进一个二进制
         │
         ▼
阶段二：模块化单体（Modular Monolith）
  清晰的模块边界 + 内部 API 契约 + 独立数据库 Schema
  为拆分做准备，不急于拆服务
         │
         ▼
阶段三：核心服务优先拆分
  网关、AI、图谱存储、认证 等高并发/高变更服务先独立部署
  其余服务暂时保留在单体中
         │
         ▼
阶段四：全面微服务化（目标）
  所有服务独立部署、独立扩缩容、独立数据库
  服务间通过 gRPC + 事件驱动通信
```

**关键原则：不要一步到位，渐进式拆分。** 先模块化单体，再按业务价值和变更频率逐步拆分，避免过早微服务化带来的分布式复杂度。

## 核心设计决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| **RPC 框架** | tonic (gRPC) | Rust 生态最成熟、流式原生、Protobuf 强类型、跨语言 |
| **服务发现** | K8s Service + CoreDNS（起步）→ Nacos（规模化） | K8s 原生零额外组件，Nacos 支持权重路由/灰度 |
| **配置中心** | K8s ConfigMap + Secret（起步）→ Nacos（规模化） | 渐进式，避免过早引入额外组件 |
| **消息队列** | NATS JetStream（推荐）/ RabbitMQ (lapin) | NATS 轻量 Rust 原生好，RabbitMQ 企业级成熟 |
| **数据库策略** | Database per Service（每服务独立 DB/Schema） | 数据隔离、独立扩展、避免耦合 |
| **API 网关** | 自研 Rust 网关（axum + tonic-web） | 多协议单端口、租户识别、限流熔断 |
| **可观测性** | OpenTelemetry + Jaeger + Prometheus + Grafana + Loki | 行业标准、全链路追踪 |
| **服务间认证** | mTLS（Istio 或自研） | 零信任、服务身份认证 |
| **部署编排** | Kubernetes + Helm + ArgoCD (GitOps) | 行业标准、声明式部署 |

## 服务清单（36 → 优化后）

详见 [01-服务边界优化](./01-service-boundaries.md)。

优化后服务分类：

| 类别 | 服务数 | 服务列表 |
|------|--------|----------|
| **接入层** | 1 | mox-gateway-svc |
| **平台能力** | 5 | mox-auth-svc, mox-tenant-svc, mox-metering-svc, mox-notification-svc, mox-system-svc |
| **AI 引擎** | 3 | mox-ai-svc, mox-agent-svc, mox-expert-svc |
| **知识图谱** | 5 | mox-graph-svc, mox-graph-storage-svc, mox-graph-algo-svc, mox-graph-streams-svc, mox-graph-meta-svc |
| **数据与存储** | 4 | mox-storage-svc, mox-etl-svc, mox-dataplane-svc, mox-search-svc |
| **流程与算子** | 3 | mox-flow-svc, mox-flow-fusion-svc, mox-operator-svc |
| **业务与治理** | 5 | mox-compliance-svc, mox-fusion-svc, mox-catalog-svc, mox-market-svc, mox-optimizer-svc |
| **Sidecar** | 1 | ai-inference (Python, GPU) |
| **合计** | **31 服务 + 1 Sidecar** | |

（从 36 个模块合并优化为 31 个独立部署服务，减少了职责重叠的模块）

## 快速开始

1. 阅读 [00-核心原则](./00-principles.md) 理解设计理念
2. 阅读 [01-服务边界](./01-service-boundaries.md) 了解服务划分
3. 阅读 [02-通信架构](./02-communication.md) 了解服务间如何通信
4. 阅读 [04-部署架构](./04-deployment.md) 了解如何独立部署
5. 按 [06-实施路线图](./06-roadmap.md) 逐步推进

---

*本文档为 infotopograph 项目内部架构设计文档，未经授权不得外传。*
