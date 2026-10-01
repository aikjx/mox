# 前端架构（`frontend-ui/`）

> 层定位：L2 架构层 · `architecture/frontend/` 主题目录　·　入口
> 上层：[架构层 README](../README.md) · [文档结构规范](../../ARCHITECTURE-OF-DOCS.md)

覆盖范围：外壳与布局分层、导航与路由 meta 契约、样式令牌与 CSS 作用域归属、组件边界。
后端/Rust 侧的 6 层架构见 [NORMALIZED_ARCHITECTURE.md](../NORMALIZED_ARCHITECTURE.md)，本目录**只**记前端事实。

## 文档

| 文档 | 编号 | 状态 | 说明 |
|------|------|------|------|
| [FRONTEND-LAYOUT-REFACTOR-PLAN-v1.0.md](./FRONTEND-LAYOUT-REFACTOR-PLAN-v1.0.md) | FE-LAY-REF-V1.0 | 🟡 方案待评审 | 外壳/布局/导航/样式四类结构性缺陷的 12 条证据、根因、P0~P2 实施计划、门禁建议与待决问题 |
| [FRONTEND-MODULE-GOVERNANCE-v1.0.md](./FRONTEND-MODULE-GOVERNANCE-v1.0.md) | FE-MOD-GOV-V1.0 | 🟢 生效 | 前端模块化治理单源：模块目录、统一出口、导入/命名/边界规则、element-plus 按需门禁、性能基线（2026-09-26） |
| [FRONTEND-MODULE.md（expert-alliance 主题，📦 已归档）](../../expert-alliance/_archive/v1/FRONTEND-MODULE.md) | EA-DOC-FE-MODULE | 📦 归档 | `src/modules/` 模块化前端与联盟控制台的实现态；现行治理规则见 FE-MOD-GOV-V1.0 |

## 现状速览（采集 2026-09-23，模块化落地后）

- 栈：Vite 5 + Vue 3.4 + Element Plus 2.4 + Pinia，hash 路由，dev `:3020`。
- 路由：`src/router/index.js` 平铺聚合 11 个 `router/modules/*.js` + `collectRoutes()` 派生的模块路由（必须排在通配兜底之前，由 `src/modules/wiring.test.js` 守卫），**尚无 layout 父节点**；外壳由 `src/App.vue` 无条件挂载。
- 模块化：`src/modules/`（`_kernel/` 信封归一 + 模块登记，`expert-alliance/` 契约·模型·API·store·视图）已上线首个模块页 `/alliance/console`。
- 导航单源：`constants/nav.config.js` 289 行；模块页面导航由 `collectNav()` 自动挂载，模块归属判定收敛到 `composables/useActiveModule.js`（此前 5 处各抄一份前缀表）；侧栏条目仅带 `path` 者可跳转，编造的 `count`/`badge` 已清空（14 处）。
- 视图：`src/views/` 72 个 `.vue`；最大单文件 `views/expert/ExpertConfigView.vue` 5414 行。
- 样式：`styles/global.css` 532 行（含 137 个 `:root` 令牌）+ 三主题各 ~100 令牌 + `styles/workspace.css` 4329 行（非 scoped，由单个 view 引入）。
- 未决：FE-LAY-REF 的 P0 布局分层（`layouts/DefaultLayout.vue` + `meta.layout` 生效）、登录页外壳泄漏、`styles/tokens.css` 令牌外提。
