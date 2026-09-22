# 璇玑 RelGraph · 文档中心

> **本目录的唯一入口**。文档结构权威规范：**[ARCHITECTURE-OF-DOCS.md](ARCHITECTURE-OF-DOCS.md)**（DOC-GOV-ARC-V1.0）
> 🌐 交互式文档中心：**[docs-hub/docs-hub.html](docs-hub/docs-hub.html)** · 自动生成开发者文档中心：**[docs-hub/developer-docs.html](docs-hub/developer-docs.html)**

---

## 一、分层地图（L0 ~ L8）

物理目录与文档层 1:1 对应；**一个主题一个目录，一个事实一个权威源**。

| 层 | 位置 | 职责 | 权威 |
|:--:|------|------|:----:|
| **L0** | [`README.md`](README.md) · [`docs-hub/`](docs-hub/README.md) | 门面与交互式导航 | 🟢 |
| **L1** | [`CORE-CAPABILITIES.md`](CORE-CAPABILITIES.md) · [`ROADMAP-DOMAINS.md`](ROADMAP-DOMAINS.md) · [`API-REGISTRY.md`](API-REGISTRY.md) | 能力 / 路线图 / 接口总账 | 🟢 |
| **L2** | [`architecture/`](architecture/README.md) | 架构事实：总览·分层·元架构·微服务·Rust 企业级·领域·工具链 | 🟢 |
| **L3** | [`modules/`](modules/README.md) · [`expert-alliance/`](expert-alliance/README.md) | 业务域与模块：依赖、页面、路由、产品手册 | 🟢/🟡 |
| **L4** | [`api/`](api/README.md) · [`database/`](database/README.md) | 接口契约（REST/TCP/端口/清单）与数据模型 | 🟢 |
| **L5** | [`standards/`](standards/README.md) · [`normalization/`](normalization/README.md) | 标准规范与归一化 SSoT 索引 | 🟢 |
| **L6** | [`enterprise/`](enterprise/00-INDEX.md) · [`specifications/`](specifications/README.md) | 企业级需求→架构→设计→交付+ADR；规格驱动任务包 | 🟢 |
| **L7** | [`working-reports/`](working-reports/README.md) | 过程报告、验证证据、审计、基准数据 | 🟡 |
| **L8** | [`_archive/`](_archive/) | 历史快照，只读，禁止作为权威引用 | 🔴 |

> 规范新增文档前请先过 5 问：属哪一层？同类是否已有目录？权威等级？是否与既有 🟢 冲突？是否已在层 `README.md` 登记？
> 详见 [ARCHITECTURE-OF-DOCS.md §7 治理门禁](ARCHITECTURE-OF-DOCS.md#7-治理门禁)。

---

## 二、权威单源（SSOT）

| 事实 | 唯一权威 | 校验 |
|------|----------|------|
| 接口 ↔ 实现 | [`API-REGISTRY.md`](API-REGISTRY.md)（223 路由 / 46 域） | `python scripts/gen-api-registry.py` |
| 端口分配 | [`api/PORT-REGISTRY.md`](api/PORT-REGISTRY.md) | `python scripts/verify-ports.py` |
| 模块与代码目录 | [`modules/CODE-CATALOG.md`](modules/CODE-CATALOG.md) | `python tools/module_catalog.py --check` |
| 术语表 | [`enterprise/GLOSSARY.md`](enterprise/GLOSSARY.md) | 人工评审 |
| 权威链与归一化关系 | [`enterprise/22-全文档归一化总控卡与权威链单源映射表-V1.0.md`](enterprise/22-全文档归一化总控卡与权威链单源映射表-V1.0.md) | 人工评审 |
| 企业级文档进度 | [`enterprise/00-INDEX.md`](enterprise/00-INDEX.md) | `python scripts/verify-doc-ep038.py` |

---

## 三、各层入口

### L1 概览层
- **[CORE-CAPABILITIES.md](CORE-CAPABILITIES.md)** — 核心能力与解决的问题（知识图谱 + 多智能体 + 统一服务化）
- **[ROADMAP-DOMAINS.md](ROADMAP-DOMAINS.md)** — 46 域路线图与验收标准
- **[API-REGISTRY.md](API-REGISTRY.md)** — API 注册表（接口↔实现一一对应）

### L2 架构层 → [架构文档中心](architecture/README.md)
- [统一架构规范 v3.0-ai-powered](architecture/architecture.md) · [操作说明手册 v2.0](architecture/operations-manual.md)
- [最优架构方案](architecture/OPTIMAL_ARCHITECTURE.md) · [归一化架构](architecture/NORMALIZED_ARCHITECTURE.md) · [领域优先布局](architecture/DOMAIN_FIRST_LAYOUT.md)
- [🏛️ 架构文档中心 HTML](architecture/architecture-hub.html) 🌐 · [统一平台架构与API文档](architecture/unified-platform-architecture-docs.html) 🌐
- 子层： [元架构 meta/](architecture/meta/README.md) · [微服务](architecture/microservices/README.md) · [Rust 企业级](architecture/rust-enterprise/README.md) · [AI 架构](architecture/ai/) · [关图 graph/](architecture/graph/) · [全维 TraceMatrix](architecture/full-dimensional/00-README.md) · [VSCode 插件](architecture/plugin/)
- [代码库指南](architecture/13-PLATFORM-CODEBASE-GUIDE.md) · [仓库全地图](architecture/14-REPOSITORY-FULL-MAP.md) · [扩展开发指南](architecture/02-extension-guide.md) · [错误码参考](architecture/04-error-code-reference.md)

### L3 域与模块层
- **[统一模块导航](modules/README.md)** · [自动生成全仓代码目录](modules/CODE-CATALOG.md) · [产品手册 v3](modules/mox-relgraph-product-handbook-v3.md)
- [业务处理流程](modules/business-process-flows.md) · [业务流程图集](modules/business-process-flowcharts.md) · [AI 引擎主分析](modules/ai-engine-master-analysis.md)
- **[专家联盟综合索引](expert-alliance/00-INTEGRATED-INDEX.md)** · [企业级优化](expert-alliance/01-ENTERPRISE-OPTIMIZATION.md) · [专家注册与协议](expert-alliance/expert-registry-and-protocol.md) · [知识图谱 Schema](expert-alliance/knowledge-graph-schema.md) · [v2](expert-alliance/v2/README.md) / [v3](expert-alliance/v3/01-architecture-optimization.md)

### L4 接口与数据层
- [API 规范](api/API-SPECIFICATION.md) · [传输加密开关](api/API-CRYPTO-TRANSPORT.md) · [端口注册表](api/PORT-REGISTRY.md) · [TCP 规范](api/TCP-SPECIFICATION.md) · [模块清单 Schema](api/mox-module-manifest.schema.json)
- [数据库文档](database/README.md) · [数据交换规范 MXDEF](architecture/data-exchange-spec.md) · [应用商店架构 MXAP](architecture/app-store-architecture.md)

### L5 规范与治理层
- [AI 原生架构标准](standards/ai-native-architecture-standard.md) · [引擎内核规范](standards/engine-kernel.md) · [引擎宇宙规范](standards/engine-universe.md)
- [专家联盟流程标准](standards/expert-alliance-flow-standard.md) · [特性开关 v2.1](standards/feature-flags-v2.1.md) · [架构数据分离](standards/architecture-data-separation.md)
- **[文档归一化 SSoT 枢纽](normalization/README.md)** · [BP](normalization/BP-INDEX.md) · [API](normalization/API-INDEX.md) · [ARC](normalization/ARC-INDEX.md) · [VAL](normalization/VAL-INDEX.md) · [TPL](normalization/TPL-INDEX.md)

### L6 企业级与规格层
- **[企业级文档索引](enterprise/00-INDEX.md)** · [需求分析](enterprise/01-requirements.md) · [架构设计](enterprise/02-architecture.md) · [详细设计](enterprise/03-design.md)
- [三联盟模式顶层设计](enterprise/18-全域顶层总设计-三联盟模式-V1.0.md) · [需求规格说明书](enterprise/21-璇玑（Aura）软件研发数字孪生中台-企业级需求规格说明书-V1.0.md) · [企业级处理流程规范](enterprise/37-企业级处理流程规范-V1.0.md)
- [ADR-09 跨域依赖治理](enterprise/29-跨域依赖规则与架构一致性治理-ADR-09.md) → [ADR-16 微服务演进](enterprise/35-模块化单体到微服务演进预案-ADR-16.md)
- [规格任务包索引](specifications/README.md) · [信息关联关系图开发规范](specifications/GR-STD-信息关联关系图开发规范-V1.0.md) · [Primi 架构规范](specifications/PT-Primi-架构规范-V1.0-完整版.md) · [业务功能规划与架构数据关系分析](specifications/OUS-业务功能规划与架构数据关系分析.md)

### L7 报告与验证层
- **[工作报告索引](working-reports/README.md)** · [验证原始证据](working-reports/verification/README.md) · [审计与就绪度评估](working-reports/audits/)
- [十项任务验收报告](working-reports/enterprise-10task-acceptance-report.md) · [开源对比报告](working-reports/mox-vs-opensource-comparison-report.md) · [算法优化对比](working-reports/perf-algorithm-optimization-contrast-report_20260824-080732.md)

### 部署与运维（跨仓目录）
- [部署文档索引总图](../deploy/docs/DOCUMENT-INDEX.md) · [企业级统一规格 v2.0](../deploy/docs/MOX-Enterprise-Unified-Spec-v2.0.md) · [ADR 全量](../deploy/docs/MOX-Architecture-Decision-Records-v1.0.md)
- [部署指南](architecture/deployment-guide.md) · [运维操作手册](../deploy/docs/ops-manual.md) · [HA 容量与 TCO](../deploy/docs/ha-capacity-tco.md) · [信创兼容矩阵](../deploy/docs/xinchuang-matrix.md)

---

## 四、按角色阅读路径

| 角色 | 路径 |
|------|------|
| 👋 **新手** | [操作手册](architecture/operations-manual.md) → [架构总览](architecture/architecture.md) → [核心能力](CORE-CAPABILITIES.md) |
| 🏗️ **架构师** | [架构文档中心](architecture/README.md) → [企业级架构](enterprise/02-architecture.md) → [最优架构方案](architecture/OPTIMAL_ARCHITECTURE.md) → [ADR 全量](../deploy/docs/MOX-Architecture-Decision-Records-v1.0.md) |
| 💻 **开发者** | [Rust 企业开发指南](architecture/rust-enterprise/README.md) → [代码库指南](architecture/13-PLATFORM-CODEBASE-GUIDE.md) → [错误码参考](architecture/04-error-code-reference.md) → [扩展开发指南](architecture/02-extension-guide.md) |
| 🧠 **AI/专家联盟** | [专家联盟综合索引](expert-alliance/00-INTEGRATED-INDEX.md) → [元架构版专家联盟](architecture/meta/02-EXPERT-ALLIANCE-ARCHITECTURE.md) → [AI 统一智能系统架构](architecture/ai/) |
| 🚀 **运维/DevOps** | [部署指南](architecture/deployment-guide.md) → [运维手册](../deploy/docs/ops-manual.md) → [容量规划](../deploy/docs/ha-capacity-tco.md) → [端口注册表](api/PORT-REGISTRY.md) |

---

## 五、治理与门禁

| 门禁 | 命令 |
|------|------|
| 文档链接有效性 | `python scripts/check-doc-links.py` |
| API 注册表新鲜度 | `python scripts/gen-api-registry.py` |
| 端口漂移 | `python scripts/verify-ports.py` |
| 模块清单漂移 | `python tools/module_catalog.py --check` |
| 企业级文档核对 | `python scripts/verify-doc-ep038.py` |
| 文档门禁汇总 | `scripts/ci-gate.ps1 -Gate G6` |

结构变更流程（路径即契约）：登记迁移映射 → 移动 → 修引用 → 更新层 README 与本规范 → CI G6 全绿。
见 [ARCHITECTURE-OF-DOCS.md §8](ARCHITECTURE-OF-DOCS.md#8-变更流程路径即契约)。

---

## 六、版本信息

| 组件 | 版本 | 日期 |
|------|------|------|
| 统一架构规范 | 3.0-ai-powered | 2026-08-29 |
| 操作说明手册 | 2.0 | 2026-08-28 |
| 归一化统一平台 | 1.0 | 2026-08-30 |
| 文档体系架构规范 | **1.0** | **2026-09-13** |
| 文档中心 | 4.0（分层化） | 2026-09-13 |
