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
