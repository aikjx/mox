# 架构层入口 — Architecture Documentation Hub

> **层定位**：L2 架构层。MOX / 璇玑 RelGraph 平台全部架构事实的唯一归档层。
> 🎨 **可视化版**：[architecture-hub.html](./architecture-hub.html)（推荐浏览）
> 上层入口：[文档中心](../README.md) · 结构规范：[ARCHITECTURE-OF-DOCS.md](../ARCHITECTURE-OF-DOCS.md)

---

## 一、本层目录结构

| 子目录 | 主题 | 入口 |
|--------|------|------|
| `meta/` | COSMIC 元架构（9 大能力域 / 5 级扩展点） | [meta/README.md](./meta/README.md) |
| `microservices/` | 微服务独立部署架构（12 铁律 / 36 服务边界） | [microservices/README.md](./microservices/README.md) |
| `rust-enterprise/` | Rust 企业级开发指南（6 层 / 8 域） | [rust-enterprise/README.md](./rust-enterprise/README.md) |
| `ai/` | AI 统一智能系统架构 | [ai/ai-unified-intelligent-system-architecture.html](./ai/ai-unified-intelligent-system-architecture.html) 🌐 |
| `graph/` | 信息关联关系图（关图）产物与需求基线 | [graph/requests/README.md](./graph/requests/README.md) |
| `full-dimensional/` | 全维 TraceMatrix 与需求基线 | [full-dimensional/00-README.md](./full-dimensional/00-README.md) |
| `plugin/` | VSCode 插件架构与兼容性 | [plugin/](./plugin/) |
| `assets/` | 结构化产物（基准数据 / 审批流 / 架构度量） | [assets/](./assets/) |

编号系列 `02~14` 为**同层内规范文档**（扩展/错误码/归一化/RPC/KG-SQL/低代码/性能/DSQL/门户/运行时/代码库/全地图）。

---

## 二、架构总览

| 文档 | 路径 | 说明 |
|------|------|------|
| 架构总览 v3.0 | [`architecture.md`](./architecture.md) | **权威入口**：AI 驱动平台架构总览（对话中心 + 四向弹框 + Agent 运行时 + 技术底座） |
| 操作说明手册 v2.0 | [`operations-manual.md`](./operations-manual.md) | 快速开始 / 平台使用 / 数据导入导出 / 应用发布 / 运维监控 |
| 归一化架构 | [`NORMALIZED_ARCHITECTURE.md`](./NORMALIZED_ARCHITECTURE.md) | 标准分层模型与模块组织 |
| 最优架构方案 | [`OPTIMAL_ARCHITECTURE.md`](./OPTIMAL_ARCHITECTURE.md) | 性能 / 可维护性 / 扩展性平衡选型 |
| 领域优先布局 | [`DOMAIN_FIRST_LAYOUT.md`](./DOMAIN_FIRST_LAYOUT.md) | 领域优先目录布局设计 |
| 仓库全地图 | [`14-REPOSITORY-FULL-MAP.md`](./14-REPOSITORY-FULL-MAP.md) | 代码仓库全景地图 |
| 统一基线 | [`MOX-UNIFIED-ENTERPRISE-BASELINE-v1.0.md`](./MOX-UNIFIED-ENTERPRISE-BASELINE-v1.0.md) | 企业级统一基线 v1.0 |
| 模块组合 | [`MOX-MODULE-COMPOSITION-v1.md`](./MOX-MODULE-COMPOSITION-v1.md) | 模块组合与装配关系 |
| 元架构总纲 | [`meta/00-COSMIC-META-ARCHITECTURE.md`](./meta/00-COSMIC-META-ARCHITECTURE.md) | 设计哲学、9 大能力域、5 级扩展点 |
| 元架构索引 | [`meta/README.md`](./meta/README.md) | 元架构文档导航 |
| 全域顶层总设计 | [`../enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`](../enterprise/18-全域顶层总设计-三联盟模式-V1.0.md) | 三联盟模式全域顶层设计 |
| 归一化与权威链 | [`../enterprise/22-全文档归一化总控卡与权威链单源映射表-V1.0.md`](../enterprise/22-全文档归一化总控卡与权威链单源映射表-V1.0.md) | 权威级别与依赖关系单源映射 |

---

## 三、核心架构

### 分层架构

| 文档 | 说明 |
|------|------|
| [`NORMALIZED_ARCHITECTURE.md`](./NORMALIZED_ARCHITECTURE.md) | 归一化架构设计，标准分层模型与模块组织 |
| [`OPTIMAL_ARCHITECTURE.md`](./OPTIMAL_ARCHITECTURE.md) | 最优架构方案 |
| [`DOMAIN_FIRST_LAYOUT.md`](./DOMAIN_FIRST_LAYOUT.md) | 领域优先的目录布局架构设计 |
| [`13-PLATFORM-CODEBASE-GUIDE.md`](./13-PLATFORM-CODEBASE-GUIDE.md) | 平台代码库开发指南 |
| [`14-REPOSITORY-FULL-MAP.md`](./14-REPOSITORY-FULL-MAP.md) | 代码仓库全景地图 |

### 微服务架构（`microservices/`）

| 文档 | 说明 |
|------|------|
| [`microservices/README.md`](./microservices/README.md) | **入口**：微服务独立部署架构优化方案 |
| [`microservices/00-principles.md`](./microservices/00-principles.md) | 独立部署 12 条铁律、DDD 原则 |
| [`microservices/01-service-boundaries.md`](./microservices/01-service-boundaries.md) | 36 个服务边界重划与 Bounded Context |
| [`microservices/02-communication.md`](./microservices/02-communication.md) | gRPC 通信、事件驱动、API 网关 |
| [`microservices/03-data.md`](./microservices/03-data.md) | Database per Service、Saga、CQRS |
| [`microservices/04-deployment.md`](./microservices/04-deployment.md) | K8s 部署、HPA、CI/CD |
| [`microservices/05-observability-security-resilience.md`](./microservices/05-observability-security-resilience.md) | 可观测性、零信任、熔断降级 |
| [`microservices/06-roadmap.md`](./microservices/06-roadmap.md) | 6 阶段 24 周实施路线图 |

### 元架构（`meta/`）

| 文档 | 说明 |
|------|------|
| [`meta/00-COSMIC-META-ARCHITECTURE.md`](./meta/00-COSMIC-META-ARCHITECTURE.md) | 元架构总纲：设计哲学、9 大能力域、5 级扩展点 |
| [`meta/01-DATABASE-DDL.sql`](./meta/01-DATABASE-DDL.sql) | 企业级数据库 DDL（23 张核心表） |
| [`meta/02-EXPERT-ALLIANCE-ARCHITECTURE.md`](./meta/02-EXPERT-ALLIANCE-ARCHITECTURE.md) | 元架构版：7 服务 + 1 Sidecar |
| [`meta/03-DATABASE-DESIGN-SPEC.md`](./meta/03-DATABASE-DESIGN-SPEC.md) | 数据库设计规范 |

### Rust 企业级架构（`rust-enterprise/`）

| 文档 | 说明 |
|------|------|
| [`rust-enterprise/README.md`](./rust-enterprise/README.md) | **入口**：纯 Rust 企业级模块化架构总览 |
| [`rust-enterprise/01-architecture-overview.md`](./rust-enterprise/01-architecture-overview.md) | 6 层架构、8 业务域详解 |
| [`rust-enterprise/03-module-inventory.md`](./rust-enterprise/03-module-inventory.md) | 模块清单与职责划分 |
| [`rust-enterprise/07-build-and-test.md`](./rust-enterprise/07-build-and-test.md) | 构建与测试指南 |

---

## 四、领域架构

### 专家联盟（核心域）

| 文档 | 说明 |
|------|------|
| [`../expert-alliance/README.md`](../expert-alliance/README.md) | **入口**：专家联盟系统定位与整体架构 |
| [`../expert-alliance/architecture/system-architecture-design.html`](../expert-alliance/architecture/system-architecture-design.html) | 系统架构设计可视化 🌐 |
| [`meta/02-EXPERT-ALLIANCE-ARCHITECTURE.md`](./meta/02-EXPERT-ALLIANCE-ARCHITECTURE.md) | 元架构版：7 服务 + 1 Sidecar |
| [`../expert-alliance/v2/README.md`](../expert-alliance/v2/README.md) | V2 完整设计文档集（7 篇） |
| [`../expert-alliance/v3/01-architecture-optimization.md`](../expert-alliance/v3/01-architecture-optimization.md) | V3 模块化架构优化 |

### AI 引擎架构

| 文档 | 说明 |
|------|------|
| [`ai/ai-unified-intelligent-system-architecture.html`](./ai/ai-unified-intelligent-system-architecture.html) | AI 统一智能系统架构可视化 🌐 |
| [`ai/README.md`](./ai/README.md) | AI 架构层说明 |
| [`../modules/ai-engine-master-analysis.md`](../modules/ai-engine-master-analysis.md) | AI 引擎深度分析报告 |

### 低代码与动态 SQL

| 文档 | 说明 |
|------|------|
| [`08-FULL-DIMENSION-LOWCODE-ARCHITECTURE.md`](./08-FULL-DIMENSION-LOWCODE-ARCHITECTURE.md) | 低代码九层架构、行业融合引擎 |
| [`07-KG-DYNAMIC-SQL-ARCHITECTURE.md`](./07-KG-DYNAMIC-SQL-ARCHITECTURE.md) | KG 驱动动态 SQL、字段级权限 |
| [`10-DSQL-CORE-FULL-DIMENSIONAL-VALIDATION.md`](./10-DSQL-CORE-FULL-DIMENSIONAL-VALIDATION.md) | DSQL 核心全维验证与竞品对比 |
| [`full-dimensional/00-README.md`](./full-dimensional/00-README.md) | 全维 TraceMatrix 六维追溯 |

### 企业级架构

| 文档 | 说明 |
|------|------|
| [`../enterprise/mox-zettabyte-architecture.html`](../enterprise/mox-zettabyte-architecture.html) | 企业级架构可视化 🌐 |
| [`../enterprise/02-architecture.md`](../enterprise/02-architecture.md) | 企业级平台架构设计 |
| [`ARCHITECTURE-ENTERPRISE.md`](./ARCHITECTURE-ENTERPRISE.md) | 企业级架构扩展说明 |
| [`ARCHITECTURE_DESIGN_v3.1.md`](./ARCHITECTURE_DESIGN_v3.1.md) | 架构设计稿 v3.1 |
| [`ARCHITECTURE_SAAS_PRIVATE.md`](./ARCHITECTURE_SAAS_PRIVATE.md) | SaaS / 私有化部署架构 |
| [`app-store-architecture.md`](./app-store-architecture.md) | 应用商店架构 MXAP |

---

## 五、技术架构

| 文档 | 说明 |
|------|------|
| [`06-rpc-integration-guide.md`](./06-rpc-integration-guide.md) | RPC / gRPC / REST 快速对接手册 |
| [`data-exchange-spec.md`](./data-exchange-spec.md) | 数据交换规范 MXDEF |
| [`deployment-guide.md`](./deployment-guide.md) | 部署指南（详见 [`../../deploy/docs/DOCUMENT-INDEX.md`](../../deploy/docs/DOCUMENT-INDEX.md)） |
| [`microservices/02-communication.md`](./microservices/02-communication.md) | 通信架构优化 |
| [`microservices/03-data.md`](./microservices/03-data.md) | 数据架构优化 |
| [`microservices/04-deployment.md`](./microservices/04-deployment.md) | 部署架构优化 |
| [`microservices/05-observability-security-resilience.md`](./microservices/05-observability-security-resilience.md) | 可观测性 · 安全 · 弹性 |
| [`graph/`](./graph/) | 信息关联关系图产物（mmd / 请求基线） |

---

## 六、架构规范与标准

| 文档 | 说明 |
|------|------|
| [`../standards/ai-native-architecture-standard.md`](../standards/ai-native-architecture-standard.md) | AI 原生架构规范（分层模型 / 域包结构 / 门禁） |
| [`../standards/expert-alliance-flow-standard.md`](../standards/expert-alliance-flow-standard.md) | 专家联盟流程标准 |
| [`02-extension-guide.md`](./02-extension-guide.md) | 扩展开发指南（零改动核心架构） |
| [`04-error-code-reference.md`](./04-error-code-reference.md) | 6 位错误码体系完整参考 |
| [`05-normalization-checklist.md`](./05-normalization-checklist.md) | 10 大类归一化检查清单 |
| [`../specifications/PT-Primi-架构规范-V1.0-完整版.md`](../specifications/PT-Primi-架构规范-V1.0-完整版.md) | Primi 架构规范 V1.0 |

---

## 七、架构决策记录（ADR）

> ADR 正文归 L6 企业级层（[`../enterprise/`](../enterprise/00-INDEX.md)），本层仅做导航。

| ADR | 文档 | 主题 |
|-----|------|------|
| ADR-09 | [`跨域依赖规则与架构一致性治理`](../enterprise/29-跨域依赖规则与架构一致性治理-ADR-09.md) | 跨域依赖规则 |
| ADR-11 | [`网关瘦身审计与方案文档`](../enterprise/30-网关瘦身审计与方案文档-ADR-11.md) | 网关瘦身 |
| ADR-12 | [`可观测性体系设计文档`](../enterprise/31-可观测性体系设计文档-ADR-12.md) | 可观测性 |
| ADR-13 | [`API 层契约设计文档`](../enterprise/32-API层契约设计文档-ADR-13.md) | API 契约 |
| ADR-14 | [`持久化工作流引擎设计文档`](../enterprise/33-持久化工作流引擎设计文档-ADR-14.md) | 工作流引擎 |
| ADR-15 | [`voice 域独立化分析与决策报告`](../enterprise/34-voice域独立化分析与决策报告-ADR-15.md) | Voice 域独立 |
| ADR-16 | [`模块化单体到微服务演进预案`](../enterprise/35-模块化单体到微服务演进预案-ADR-16.md) | 微服务演进 |
| — | [`架构违规修复执行计划`](../enterprise/36-架构违规修复执行计划.md) | 违规修复计划 |

---

## 八、性能与优化

| 文档 | 说明 |
|------|------|
| [`09-ROCKSDB-PERFORMANCE-OPTIMIZATION.md`](./09-ROCKSDB-PERFORMANCE-OPTIMIZATION.md) | RocksDB FFI 性能分析与优化 |
| [`12-MOX-RUNTIME-ENGINE-DELIVERY.md`](./12-MOX-RUNTIME-ENGINE-DELIVERY.md) | 运行时引擎交付与性能指标 |
| [`10-DSQL-CORE-FULL-DIMENSIONAL-VALIDATION.md`](./10-DSQL-CORE-FULL-DIMENSIONAL-VALIDATION.md) | DSQL 性能验证 |
| [`meta/03-DATABASE-DESIGN-SPEC.md`](./meta/03-DATABASE-DESIGN-SPEC.md) | 数据库设计性能规范 |
| [`assets/bench_results_round7.json`](./assets/bench_results_round7.json) | 联盟基准数据（round7，🟡 产物） |

### 其他规划文档

| 文档 | 说明 |
|------|------|
| [`AI-UNIFIED-OPTIMIZATION-PLAN.md`](./AI-UNIFIED-OPTIMIZATION-PLAN.md) | AI 统一优化计划 |
| [`ARCHITECTURE_OPTIMIZATION.md`](./ARCHITECTURE_OPTIMIZATION.md) | 架构优化记录 |
| [`SYSTEM-OVERVIEW.md`](./SYSTEM-OVERVIEW.md) | 系统总览 |
| [`MODULARITY.md`](./MODULARITY.md) | 模块化说明 |
| [`BUSINESS-FLOWS.md`](./BUSINESS-FLOWS.md) | 业务流程总览 |
| [`11-ENTERPRISE-WEBSITE-LOWCODE-IMPLEMENTATION.md`](./11-ENTERPRISE-WEBSITE-LOWCODE-IMPLEMENTATION.md) | 企业门户低代码实现 |
| [`../standards/project-atlas.md`](../standards/project-atlas.md) · [`../standards/project-registry-v2.md`](../standards/project-registry-v2.md) | 项目图谱与注册表 |

---

## 九、速查卡

```
L6 接入层     → Gateway + API
L5 集成层     → mox-platform-integration-core (核心枢纽)
L4 对接能力层 → AI / Plugin / Enterprise / Connector
L3 领域服务层 → 8域 (kg/ai/flow/data/cloud/voice/market/platform)
L2 平台核心层 → iam/system/meta/orchestrator/datastore/operator
L1 基础框架层 → framework/foundation/observability
```

```
扩展模式：实现Trait → 实现Factory → 注册到Registry → 加配置 → 自动组装（核心代码零改动 ✅）
```

---

## 十、维护

- **可视化索引**：[architecture-hub.html](./architecture-hub.html) · [统一平台架构与API文档](./unified-platform-architecture-docs.html)
- **结构规范**：[ARCHITECTURE-OF-DOCS.md](../ARCHITECTURE-OF-DOCS.md)
- **更新频率**：架构变更时同步更新；新增文档须在本页登记
- **负责人**：架构开发联盟　**最后整理**：2026-09-13（L2 层收敛：meta / microservices / rust-enterprise / ai / graph / plugin / full-dimensional / assets）
