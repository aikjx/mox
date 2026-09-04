# 架构文档中心 — Architecture Documentation Hub

> 本文档是架构文档的核心入口，汇集了 MOX 平台所有架构相关文档。
>
> 🎨 **可视化版**：[architecture-hub.html](../architecture-hub.html)（推荐浏览）

---

## 快速导航

| 分类 | 说明 | 文档数 |
|------|------|--------|
| [🏛️ 架构总览](#架构总览) | 顶层设计、架构哲学、整体架构图 | 6+ |
| [🧱 核心架构](#核心架构) | 分层架构、微服务、元架构、Rust 企业级 | 12+ |
| [🌐 领域架构](#领域架构) | 专家联盟、AI引擎、低代码、知识图谱 | 15+ |
| [⚙️ 技术架构](#技术架构) | 通信、数据、部署、安全、可观测性 | 10+ |
| [📐 架构规范](#架构规范与标准) | 编码规范、归一化、错误码、扩展指南 | 10+ |
| [📜 决策记录](#架构决策记录-adr) | ADR 架构决策记录 | 8 |
| [🚀 性能优化](#性能与优化) | 性能分析、优化方案、验证报告 | 5+ |

---

## 架构总览

| 文档 | 路径 | 说明 |
|------|------|------|
| 架构总览 v3.0 | [`architecture.md`](../architecture.md) | **权威入口**：MOX AI 驱动mox 模块化系统架构平台 v3.0 架构总览 |
| 元架构总纲 | [`cosmic-architecture/00-COSMIC-META-ARCHITECTURE.md`](../cosmic-architecture/00-COSMIC-META-ARCHITECTURE.md) | COSMIC 元架构设计哲学与 9 大能力域 |
| 元架构文档索引 | [`cosmic-architecture/README.md`](../cosmic-architecture/README.md) | 元架构文档目录与导航 |
| 全域顶层总设计 | [`enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`](../enterprise/18-全域顶层总设计-三联盟模式-V1.0.md) | 三联盟模式全域顶层总设计 V1.0 |
| 文档归一化与权威链 | [`enterprise/22-全文档归一化总控卡与权威链单源映射表-V1.0.md`](../enterprise/22-全文档归一化总控卡与权威链单源映射表-V1.0.md) | 全文档权威级别与依赖关系映射 |

---

## 核心架构

### 分层架构

| 文档 | 说明 |
|------|------|
| [`NORMALIZED_ARCHITECTURE.md`](./NORMALIZED_ARCHITECTURE.md) | 归一化架构设计，标准分层模型与模块组织 |
| [`OPTIMAL_ARCHITECTURE.md`](./OPTIMAL_ARCHITECTURE.md) | 最优架构方案，平衡性能/可维护性/扩展性 |
| [`DOMAIN_FIRST_LAYOUT.md`](./DOMAIN_FIRST_LAYOUT.md) | 领域优先的目录布局架构设计 |
| [`14-REPOSITORY-FULL-MAP.md`](./14-REPOSITORY-FULL-MAP.md) | 代码仓库全景地图 |

### 微服务架构

| 文档 | 说明 |
|------|------|
| [`microservices/README.md`](../microservices/README.md) | **入口**：微服务独立部署架构优化方案 |
| [`microservices/00-principles.md`](../microservices/00-principles.md) | 独立部署 12 条铁律、DDD 原则 |
| [`microservices/01-service-boundaries.md`](../microservices/01-service-boundaries.md) | 36 个服务边界重划与 Bounded Context |
| [`microservices/02-communication.md`](../microservices/02-communication.md) | gRPC 通信、事件驱动、API 网关 |
| [`microservices/03-data.md`](../microservices/03-data.md) | Database per Service、Saga、CQRS |
| [`microservices/04-deployment.md`](../microservices/04-deployment.md) | K8s 部署、HPA、CI/CD |
| [`microservices/05-observability-security-resilience.md`](../microservices/05-observability-security-resilience.md) | 可观测性、零信任、熔断降级 |
| [`microservices/06-roadmap.md`](../microservices/06-roadmap.md) | 6 阶段 24 周实施路线图 |

### 元架构

| 文档 | 说明 |
|------|------|
| [`cosmic-architecture/00-COSMIC-META-ARCHITECTURE.md`](../cosmic-architecture/00-COSMIC-META-ARCHITECTURE.md) | 元架构总纲：设计哲学、9 大能力域、5 级扩展点 |
| [`cosmic-architecture/01-DATABASE-DDL.sql`](../cosmic-architecture/01-DATABASE-DDL.sql) | 企业级数据库 DDL（23 张核心表） |
| [`cosmic-architecture/03-DATABASE-DESIGN-SPEC.md`](../cosmic-architecture/03-DATABASE-DESIGN-SPEC.md) | 数据库设计规范 |

### Rust 企业级架构

| 文档 | 说明 |
|------|------|
| [`rust-enterprise/README.md`](../rust-enterprise/README.md) | **入口**：纯 Rust 企业级模块化架构总览 |
| [`rust-enterprise/01-architecture-overview.md`](../rust-enterprise/01-architecture-overview.md) | 6 层架构、8 业务域详解 |
| [`rust-enterprise/03-module-inventory.md`](../rust-enterprise/03-module-inventory.md) | 模块清单与职责划分 |

---

## 领域架构

### 专家联盟（核心域）

| 文档 | 说明 |
|------|------|
| [`expert-alliance/README.md`](../expert-alliance/README.md) | **入口**：专家联盟系统定位与整体架构 |
| [`expert-alliance/architecture/system-architecture-design.html`](../expert-alliance/architecture/system-architecture-design.html) | 系统架构设计可视化文档 |
| [`cosmic-architecture/02-EXPERT-ALLIANCE-ARCHITECTURE.md`](../cosmic-architecture/02-EXPERT-ALLIANCE-ARCHITECTURE.md) | 元架构版：7 服务 + 1 Sidecar |
| [`expert-alliance/v2/README.md`](../expert-alliance/v2/README.md) | V2 完整设计文档集（7 篇） |
| [`expert-alliance/v3/01-architecture-optimization.md`](../expert-alliance/v3/01-architecture-optimization.md) | V3 模块化架构优化 |

### AI 引擎架构

| 文档 | 说明 |
|------|------|
| [`ai-architecture/ai-unified-intelligent-system-architecture.html`](../ai-architecture/ai-unified-intelligent-system-architecture.html) | AI 统一智能系统架构可视化 |
| [`modules/ai-engine-master-analysis.md`](../modules/ai-engine-master-analysis.md) | AI 引擎深度分析报告 |

### 低代码与动态 SQL

| 文档 | 说明 |
|------|------|
| [`08-FULL-DIMENSION-LOWCODE-ARCHITECTURE.md`](./08-FULL-DIMENSION-LOWCODE-ARCHITECTURE.md) | mox 模块化系统架构低代码九层架构、行业融合引擎 |
| [`07-KG-DYNAMIC-SQL-ARCHITECTURE.md`](./07-KG-DYNAMIC-SQL-ARCHITECTURE.md) | KG 驱动动态 SQL、字段级权限 |
| [`10-DSQL-CORE-FULL-DIMENSIONAL-VALIDATION.md`](./10-DSQL-CORE-FULL-DIMENSIONAL-VALIDATION.md) | DSQL mox 模块化系统架构验证与竞品对比 |

### 企业级架构

| 文档 | 说明 |
|------|------|
| [`enterprise-architecture/mox-zettabyte-architecture.html`](../enterprise-architecture/mox-zettabyte-architecture.html) | 企业级架构可视化展示 |
| [`enterprise/02-architecture.md`](../enterprise/02-architecture.md) | 企业级mox 模块化系统架构平台架构设计 |

---

## 技术架构

| 文档 | 说明 |
|------|------|
| [`06-rpc-integration-guide.md`](./06-rpc-integration-guide.md) | RPC/gRPC/REST 快速对接手册 |
| [`13-PLATFORM-CODEBASE-GUIDE.md`](./13-PLATFORM-CODEBASE-GUIDE.md) | 平台代码库开发指南 |
| [`microservices/02-communication.md`](../microservices/02-communication.md) | 通信架构优化 |
| [`microservices/03-data.md`](../microservices/03-data.md) | 数据架构优化 |
| [`microservices/04-deployment.md`](../microservices/04-deployment.md) | 部署架构优化 |
| [`microservices/05-observability-security-resilience.md`](../microservices/05-observability-security-resilience.md) | 可观测性·安全·弹性 |

---

## 架构规范与标准

| 文档 | 说明 |
|------|------|
| [`standards/ai-native-architecture-standard.md`](../standards/ai-native-architecture-standard.md) | AI 原生架构规范（分层模型/域包结构/门禁） |
| [`standards/expert-alliance-flow-standard.md`](../standards/expert-alliance-flow-standard.md) | 专家联盟流程标准 |
| [`02-extension-guide.md`](./02-extension-guide.md) | 扩展开发指南（零改动核心架构） |
| [`04-error-code-reference.md`](./04-error-code-reference.md) | 6 位错误码体系完整参考 |
| [`05-normalization-checklist.md`](./05-normalization-checklist.md) | 10 大类归一化检查清单 |
| [`specs/PT-Primi-架构规范-V1.0-完整版.md`](../specs/PT-Primi-架构规范-V1.0-完整版.md) | Primi 架构规范 V1.0 |

---

## 架构决策记录 (ADR)

> 完整列表见 enterprise/ 目录

| ADR | 文档 | 主题 |
|-----|------|------|
| ADR-09 | [`跨域依赖规则与架构一致性治理`](../enterprise/29-跨域依赖规则与架构一致性治理-ADR-09.md) | 跨域依赖规则 |
| ADR-11 | [`网关瘦身审计与方案文档`](../enterprise/30-网关瘦身审计与方案文档-ADR-11.md) | 网关瘦身 |
| ADR-12 | [`可观测性体系设计文档`](../enterprise/31-可观测性体系设计文档-ADR-12.md) | 可观测性 |
| ADR-13 | [`API层契约设计文档`](../enterprise/32-API层契约设计文档-ADR-13.md) | API 契约 |
| ADR-14 | [`持久化工作流引擎设计文档`](../enterprise/33-持久化工作流引擎设计文档-ADR-14.md) | 工作流引擎 |
| ADR-15 | [`voice域独立化分析与决策报告`](../enterprise/34-voice域独立化分析与决策报告-ADR-15.md) | Voice 域独立 |
| ADR-16 | [`模块化单体到微服务演进预案`](../enterprise/35-模块化单体到微服务演进预案-ADR-16.md) | 微服务演进 |
| — | [`架构违规修复执行计划`](../enterprise/36-架构违规修复执行计划.md) | 违规修复计划 |

---

## 性能与优化

| 文档 | 说明 |
|------|------|
| [`09-ROCKSDB-PERFORMANCE-OPTIMIZATION.md`](./09-ROCKSDB-PERFORMANCE-OPTIMIZATION.md) | RocksDB FFI 性能分析与优化 |
| [`12-MOX-RUNTIME-ENGINE-DELIVERY.md`](./12-MOX-RUNTIME-ENGINE-DELIVERY.md) | 运行时引擎交付与性能指标 |
| [`10-DSQL-CORE-FULL-DIMENSIONAL-VALIDATION.md`](./10-DSQL-CORE-FULL-DIMENSIONAL-VALIDATION.md) | DSQL 性能验证 |
| [`cosmic-architecture/03-DATABASE-DESIGN-SPEC.md`](../cosmic-architecture/03-DATABASE-DESIGN-SPEC.md) | 数据库设计性能规范 |

---

## 架构分层速查

```
L6 接入层     → Gateway + API
L5 集成层     → mox-platform-integration-core (核心枢纽)
L4 对接能力层 → AI / Plugin / Enterprise / Connector
L3 领域服务层 → 8域 (kg/ai/flow/data/cloud/voice/market/platform)
L2 平台核心层 → iam/system/meta/orchestrator/datastore/operator
L1 基础框架层 → framework/foundation/observability
```

## 扩展模式速查

```
实现Trait → 实现Factory → 注册到Registry → 加配置 → 自动组装
核心代码零改动 ✅
```

---

## 文档维护

- **可视化索引**：[architecture-hub.html](../architecture-hub.html)
- **更新频率**：架构变更时同步更新
- **负责人**：架构开发联盟
- **最后整理**：2026-08-31
