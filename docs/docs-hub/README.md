# 文档中心（交互式） — Docs Hub

> **层定位**：L0 入口层。`docs/` 的可视化与检索前端（纯静态，可直接双击打开，无需构建）。
> 上层入口：[文档中心](../README.md) · 结构规范：[ARCHITECTURE-OF-DOCS.md](../ARCHITECTURE-OF-DOCS.md)

---

## 一、本目录内容

| 文件/目录 | 说明 |
|-----------|------|
| [`docs-hub.html`](./docs-hub.html) | **主入口**：交互式文档导航（搜索、分类筛选、按角色快速入门） |
| [`developer-docs.html`](./developer-docs.html) | 开发者文档中心（自动生成版，聚焦代码/接口/模块） |
| `_shared/` | 共享样式与前端运行时（本目录内多页复用） |
| `assets/` | 图标、示例数据等静态资源 |

## 二、使用方式

- 本地：浏览器直接打开 `docs-hub.html`（`file://` 即可，无外部依赖）。
- 仓库内引用：从 [`../README.md`](../README.md) 顶部入口进入。
- 报告类 HTML 不放本目录，统一进 [`../../reports/html/`](../../reports/html/)（结构规范 §2.2）。

## 三、维护约定

1. 本目录**只放导航与渲染产物**，不放权威正文；正文一律在对应层目录，本页只做索引。
2. 导航项与 [`../README.md`](../README.md) 的分层地图保持一一对应，新增层/目录须同步。
3. `_shared/` 与 `assets/` 为共享资源，**禁止**复制到其它报告目录（仓库治理规则）。
4. 交互式页面引用的文档路径若发生迁移，必须随迁移一并更新，并用 `python scripts/gate/check-doc-links.py` 校验。

<a id="lowcode-directory-design"></a>
## 目录架构设计卡：低代码与动态配置（2026-10-01）

> 目标设计接缝；既有正文按原日期/类型解释，未实施能力不标已完成。

| 设计项 | 本目录约定 |
|---|---|
| 输入 | 现行主题索引、目录设计卡与权威声明 |
| 处理与边界 | 展示入口与来源类型，链接原文；不复制配置模型或发布状态表 |
| 输出 | 可用导航和低代码设计阅读路径 |
| 维护角色 | 文档导航owner（角色建议，未指派个人） |
| 配置语义 | [统一规范](docs/standards/lowcode-dynamic-configuration.md#model)，本目录不复制覆盖/生命周期规则 |
| 本目录设计 | [详细接缝](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories) |
| 验收 | 类型/依赖/权限/版本/异常/恢复按相关LC-Q条目补证；设计完成与运行验证分开 |

全目录关系见 [目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)。新增架构文档应符合 [文档设计契约](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#document-contract)，各主题拥有自己的事实主源。
