# 8 个 legacy 业务域迁入 defineModule 治理报告

> 生成时间：2026-09-27 ｜ 仓库：`infotopograph/frontend-ui/`
> 前置：`module-governance.md`（体检）、`permission-guard-fix.md`（守卫）、`governance-wrapup.md`（收尾）
> 目标：把 `router/modules/*.js` 手写路由迁入 `src/modules/<域>/index.js` 的 `defineModule` 单源登记，行为零改变。

---

## 0. 结论速览

- **8 域全部迁入**：project / ai / graph / workflow / market / operators / system 新建模块；alliance 剩余 legacy 路由并入既有 expert-alliance 模块。
- **行为零改变**：路由 path/name/权限 meta/裸页标记逐字搬运；仅向内核强制的 `meta.title/module/layout` 补齐。
- **纯 redirect 别名**（无 component，内核不收）留在 `router/modules/*.js` 作为零路由兜底。
- **验证**：迁移相关 vitest **3 文件 14 例全绿**；`vite build` **通过（4348 模块，1m51s）**。

---

## 1. 逐域迁移前后对照

| 域 | 新模块目录 | 迁入路由（顶层） | 嵌套 children | nav | endpoints | 留在 router/modules 的 redirect 别名 |
|----|-----------|:---:|:---:|:---:|:---:|------|
| project | `modules/project/` | 5（dashboard/projects/tasks/resources/workbench） | /resources 下 2 | N/A 页内 | — | 无 |
| ai | `modules/ai/` | 7（ai/share/caomei/algolab/infinite-optimizer/botCenter/melody2score） | — | N/A 页内 | — | `/s/:token` |
| graph | `modules/graph/` | 3（graph/mox-fusion/flow-graph） | — | N/A 页内 | — | 无 |
| workflow | `modules/workflow/` | 2（workflow/browser） | /workflow 下 4 | N/A 页内 | — | `/plugins` `/mcp` `/automation` |
| market | `modules/market/` | 2（market/market/:id） | — | N/A 页内 | — | 无 |
| operators | `modules/operators/` | 1（operators） | — | N/A 页内 | — | 无 |
| system | `modules/system/` | 1（/admin） | /admin 下 17 | N/A 页内 | — | `/monitor` `/docs` `/llm-config` `/knowledge-base` |
| alliance | 并入 `modules/expert-alliance/` | 4（expert-workspace/expert-center/expert-config/expert-plaza） | /expert-center 下 4 | 4 项（task2 已迁） | 既有 ENDPOINTS | `/expert` `/alliance` `/expert-enterprise` `/expert-orchestrator` |

> nav 列：project/ai/graph/workflow/market/operators/system 七域本就靠页面内 Tabs 导航，TheSidebar 不渲染侧栏（见 module-governance.md §3.2），故 nav 为空、记 N/A，health-check 语义已从 ❌ 改为「N/A 页内导航」。

## 2. meta 处理说明（行为零改变约束下的必要注入）

内核 `defineModule` 强制每条顶层路由 `meta.{title,module,layout}`，layout ∈ `default|blank`。迁移时：
- **title**：原文件已全部具备，逐字保留。
- **module**：新增域标签（project/ai/graph/workflow/market/operators/admin/expert），不影响守卫（守卫只读 requiresAuth/requiresRole/requiresPermission）。
- **layout**：业务页统一 `default`；裸页 `/share/:token` 用 `blank`，并**保留原 `bare:true` `shareMode:true`** 标记。
- **requiresAuth / requiresRole / requiresPermission / backTo / quickNav / quickActions / isExpertAdmin / isExpertAllianceMain**：逐字搬运，未改判定。

## 3. 改动文件清单

**新建模块**
- `modules/project/index.js`、`modules/ai/index.js`、`modules/graph/index.js`、`modules/workflow/index.js`、`modules/market/index.js`、`modules/operators/index.js`、`modules/system/index.js`
- `modules/domain-migration.test.js`（4 例：路由收集/meta 四要素/模块登记/share 裸页）

**修改**
- `modules/expert-alliance/index.js`（routes 追加 4 条 legacy 路由）
- `modules/index.js`（追加 7 行 import）
- `modules/health-check.js`（空 nav 语义 ❌→N/A）
- `router/modules/{project,ai,graph,workflow,market,operators,system,alliance}.js`（删已迁路由，仅留 redirect 别名）

## 4. 四要素终态快照（health-check 输出）

- registeredNotMounted = []（零漂移）
- handwritten = []（侧栏手写副本清零）
- 各新模块 routes ✅ / nav N/A 页内 / tests ✅（由 domain-migration.test.js 覆盖）
- 模块总数：expert-alliance + admin-lowcode + project + ai + graph + workflow + market + operators + system = **9 个已登记模块**

## 5. 验证结果

| 项 | 命令 | 结果 |
|----|------|------|
| 迁移装配 | `npx vitest run src/modules/domain-migration.test.js wiring.test.js health-check.test.js` | ✅ 3 文件 / 14 例 |
| 构建 | `npx vite build` | ✅ 4348 模块，built in 1m51s，exit 0 |
| 全量 vitest | — | ⚠️ jsdom 全量超时，未一次复跑（分批策略下同前） |

## 6. 剩余缺口

1. **全量 vitest 总数字**未在本轮复跑（jsdom 环境耗时长）；CI 应在内存充足环境跑全量。
2. 新域模块 `endpoints` 留空（无契约表不强造），后续可按 expert-alliance 范式补 `contract/endpoints.js`。
3. 新域 nav 为空（页内 Tabs），图标侧栏 ICON_NAV_GROUPS 未收录这些域页——属设计内，非漂移。
4. GUI 路由行为未做浏览器 E2E（真实 IAM 受限）；以构建 + 装配测试 + 代码审查为准。
5. AdminLlm `catFill` 未导入真 bug、AdminMonitor 轮询句柄（前轮遗留）未处理。
