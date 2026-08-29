# MOX 文档中心

> **唯一权威入口**。所有文档以本索引为准，历史文档已归一化。

---

## 权威文档（必读）

| 文档 | 说明 | 状态 |
|------|------|------|
| **[architecture.md](architecture.md)** | 统一架构规范 **v3.0-ai-powered** — AI 驱动全维平台：对话中心 + 四向弹框 + Agent 运行时 + 原有技术底座（DSQL/KG/权限/商店等） | 🟢 权威 |
| **[MOX-AI驱动全维平台-企业级设计-全维分析-v3.0.md](MOX-AI驱动全维平台-企业级设计-全维分析-v3.0.md)** | 全维分析 v3.0 — 完整设计决策：现状诊断、架构优化、业务流程优化、开源对标、路线图（architecture.md §0 的完整展开版） | 📘 设计决策依据 |
| **[operations-manual.md](operations-manual.md)** | 操作说明手册 v2.0 — 快速开始、平台使用、数据导出导入、应用发布安装、独立部署、域名绑定、运维监控、常见问题 | 🟢 权威 |

---

## v3.0 升级概览

MOX 已从「功能驱动的低代码工作台」跃迁为 **「AI 驱动的对话中心」**：

- **唯一入口**：AI 对话中心 = 首页，用户出思路，AI 多跑腿
- **四向弹框**：所有功能以弹框形式从对话中心弹出（右/顶/底/中），用户永不离开对话上下文
- **Agent 化**：业务流程封装为 Agent（DSL 即商品），AI 自动编排执行
- **全维自研**：纯 Rust Workspace，8 大 domain + 新增意图中枢/Agent 运行时/记忆/计费

> 详细见 `architecture.md` §0 及全维分析文档。

---

## 专项规范（参考）

| 文档 | 说明 |
|------|------|
| [data-exchange-spec.md](data-exchange-spec.md) | 数据交换规范 MXDEF v1.0 — 导出/导入/校验格式与命令详解 |
| [deployment-guide.md](deployment-guide.md) | 部署指南 v1.0 — 本地/Docker/远程/Nginx/Systemd完整部署流程 |
| [app-store-architecture.md](app-store-architecture.md) | 应用商店架构 v1.0 — MXAP包格式、发布流程、子系统运行时、安全机制、API文档 |

---

## 文档阅读路径

### 新手（第一次用）
1. `operations-manual.md` 第一章 → 5分钟跑起来
2. `operations-manual.md` 第二章 → 平台使用指南
3. `architecture.md` → 了解整体架构

### 开发者（做业务配置）
1. `operations-manual.md` 第二章 → DSQL配置、AI助手、权限
2. `architecture.md` 第四~七章 → DSQL/KG/权限/AI规范
3. `data-exchange-spec.md` → 数据导出导入

### 运维（部署上线）
1. `operations-manual.md` 第五~七章 → 独立部署、域名绑定、运维
2. `deployment-guide.md` → 完整部署指南
3. `operations-manual.md` 第八章 → 常见问题

### ISV/开发商（发布应用到商店）
1. `app-store-architecture.md` → 应用商店架构、MXAP格式
2. `operations-manual.md` 第四章 → 应用发布与安装
3. `operations-manual.md` 第五章 → 独立部署与交付

---

## 版本

| 组件 | 版本 | 日期 |
|------|------|------|
| 统一架构规范 | **3.0-ai-powered** | 2026-08-29 |
| 全维分析文档 | 3.0 | 2026-08-28 |
| 操作说明手册 | 2.0 | 2026-08-28 |
| MXDEF数据格式 | 1.0 | 2026-08-28 |
| MXAP应用包 | 1.0 | 2026-08-28 |

---

*MOX 全维低代码平台 — 配置驱动，零代码改业务*
