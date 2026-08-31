# 璇玑 RelGraph · 文档中心

> **🌟 全新文档中心已上线**：交互式文档导航，支持搜索、分类筛选、按角色快速入门
>
> 👉 **[打开交互式文档中心](docs-hub/docs-hub.html)**

---

## 权威入口文档（必读）

| 文档 | 说明 | 状态 |
|------|------|------|
| **[architecture.md](architecture.md)** | 统一架构规范 **v3.0-ai-powered** — AI 驱动全维平台：对话中心 + 四向弹框 + Agent 运行时 + 原有技术底座 | 🟢 权威 |
| **[operations-manual.md](operations-manual.md)** | 操作说明手册 v2.0 — 快速开始、平台使用、数据导出导入、应用发布安装、运维监控 | 🟢 权威 |
| **[全维分析 v3.0](MOX-AI驱动全维平台-企业级设计-全维分析-v3.0.md)** | 完整设计决策：现状诊断、架构优化、业务流程优化、开源对标、路线图 | 📘 设计依据 |
| **[统一平台架构与API文档](unified-platform-architecture-docs.html)** | 六大归一化体系完整架构说明与 API 接口参考 | 🌐 HTML |

---

## 文档分类导航

### 🏢 企业级开发
- [企业级文档索引](enterprise/00-INDEX.md)
- [需求分析](enterprise/01-requirements.md) · [架构设计](enterprise/02-architecture.md) · [详细设计](enterprise/03-design.md)
- [测试验证报告](enterprise/11-全维测试验证优化修复报告.md) · [交付清单](enterprise/10-企业级交付清单.md)
- [三联盟模式顶层设计](enterprise/18-全域顶层总设计-三联盟模式-V1.0.md) · [算子系统归一化](enterprise/17-算子系统全维分析与归一化设计.md)
- [代码审计报告](enterprise/31-全维代码审计与验证报告-V1.0.md) · [可观测性体系设计](enterprise/31-可观测性体系设计文档-ADR-12.md)

### 🏛️ 架构设计
- [🏛️ 架构文档中心](architecture-hub.html) 🌐 **（一站式索引）** · [架构文档索引](architecture/README.md)
- [最优架构方案](architecture/OPTIMAL_ARCHITECTURE.md) · [归一化架构](architecture/NORMALIZED_ARCHITECTURE.md)
- [扩展开发指南](architecture/02-extension-guide.md) · [错误码参考](architecture/04-error-code-reference.md)
- [KG动态SQL架构](architecture/07-KG-DYNAMIC-SQL-ARCHITECTURE.md) · [全维低代码架构](architecture/08-FULL-DIMENSION-LOWCODE-ARCHITECTURE.md)
- [微服务架构](microservices/README.md) · [元架构总纲](cosmic-architecture/00-COSMIC-META-ARCHITECTURE.md)
- [代码库指南](architecture/13-PLATFORM-CODEBASE-GUIDE.md) · [仓库全地图](architecture/14-REPOSITORY-FULL-MAP.md)
- [ADR 架构决策记录](enterprise/29-跨域依赖规则与架构一致性治理-ADR-09.md) ~ [ADR-16](enterprise/35-模块化单体到微服务演进预案-ADR-16.md)

### 🧠 专家联盟
- [综合索引](expert-alliance/00-INTEGRATED-INDEX.md) · [企业级优化](expert-alliance/01-ENTERPRISE-OPTIMIZATION.md)
- [专家注册与协议](expert-alliance/expert-registry-and-protocol.md) · [知识图谱 Schema](expert-alliance/knowledge-graph-schema.md)
- [v2 完整文档集](expert-alliance/v2/README.md) · [v3 架构优化](expert-alliance/v3/01-architecture-optimization.md)

### 🔄 归一化统一平台
- [统一平台架构与API文档](unified-platform-architecture-docs.html) 🌐
- [产品手册 v3](mox-relgraph-product-handbook-v3.md)
- [TraceMatrix 六维追溯](full-dimensional/mox-tracematrix.html) 🌐

### 🚀 部署与运维
- [部署文档索引总图](../deploy/docs/DOCUMENT-INDEX.md)
- [企业级统一规格 v2.0](../deploy/docs/MOX-Enterprise-Unified-Spec-v2.0.md)
- [架构决策记录 ADR](../deploy/docs/MOX-Architecture-Decision-Records-v1.0.md)
- [部署指南](deployment-guide.md) · [运维操作手册](../deploy/docs/ops-manual.md)
- [HA容量与TCO规划](../deploy/docs/ha-capacity-tco.md) · [信创兼容矩阵](../deploy/docs/xinchuang-matrix.md)

### 📋 规范与标准
- [AI原生架构标准](standards/ai-native-architecture-standard.md)
- [数据交换规范 MXDEF](data-exchange-spec.md)
- [应用商店架构 MXAP](app-store-architecture.md)
- [引擎内核规范](standards/engine-kernel.md) · [专家联盟流程标准](standards/expert-alliance-flow-standard.md)

### 🧩 模块技术文档
- [模块文档索引](modules/)
- [PrimiFlow 设计蓝图](modules/PrimiFlow-设计蓝图.md)
- [AI 引擎主分析](modules/ai-engine-master-analysis.md)
- [Rust 企业开发指南](rust-enterprise/README.md)
- [业务处理流程](modules/business-process-flows.md)

### 📝 工作报告
- [工作报告索引](working-reports/README.md)
- [十项任务验收报告](working-reports/enterprise-10task-acceptance-report.md)
- [开源对比报告](working-reports/mox-vs-opensource-comparison-report.md)
- [算法优化对比报告](working-reports/perf-algorithm-optimization-contrast-report_20260824-080732.md)
- [竞品全维功能对比](enterprise/23-竞品全维功能对比与可用性判定报告-V1.0.md)

---

## 按角色阅读路径

### 👋 新手入门
1. [操作手册 第一章](operations-manual.md) → 5分钟跑起来
2. [操作手册 第二章](operations-manual.md) → 平台使用指南
3. [架构总览](architecture.md) → 了解整体架构

### 🏗️ 架构师
1. [架构文档中心](architecture-hub.html) → 完整架构文档索引 🌐
2. [企业级架构](enterprise/02-architecture.md) → 整体架构设计
3. [最优架构方案](architecture/OPTIMAL_ARCHITECTURE.md) → 技术方案选型
4. [ADR 决策记录](../deploy/docs/MOX-Architecture-Decision-Records-v1.0.md) → 历史决策

### 💻 开发者
1. [Rust 企业开发指南](rust-enterprise/README.md) → 开发环境与规范
2. [代码库指南](architecture/13-PLATFORM-CODEBASE-GUIDE.md) → 代码结构导航
3. [错误码参考](architecture/04-error-code-reference.md) → 调试排错

### 🚀 运维/DevOps
1. [部署指南](deployment-guide.md) → 完整部署流程
2. [运维手册](../deploy/docs/ops-manual.md) → 日常运维
3. [容量规划](../deploy/docs/ha-capacity-tco.md) → 资源规划

---

## 版本信息

| 组件 | 版本 | 日期 |
|------|------|------|
| 统一架构规范 | **3.0-ai-powered** | 2026-08-29 |
| 全维分析文档 | 3.0 | 2026-08-28 |
| 操作说明手册 | 2.0 | 2026-08-28 |
| 归一化统一平台 | 1.0 | 2026-08-30 |
| 文档中心 | 3.0 | 2026-08-30 |

---

> **更多文档？** 👉 打开 **[交互式文档中心](docs-hub/docs-hub.html)** 搜索和浏览全部 70+ 文档
