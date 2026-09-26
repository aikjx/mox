# 治理机制建设：admin 冒烟测试 + 图标侧栏 alliance 入口补全

> 日期：2026-09-26　范围：`frontend-ui/`（Vite Vue3 + Pinia + Element Plus + vitest + @vue/test-utils）
> 目标：①给 schema 化 AdminConfig/AdminAccess（低代码引擎路径）与核心手写面板 AdminUser/AdminRole 补冒烟；②补齐 6 个 `/alliance/*` 页的图标侧栏入口，做到导航注册→挂载零漂移；③vitest 在既有基线上全绿。

---

## 1. 新增测试清单与覆盖点

测试风格对齐 `src/modules/expert-alliance/components/plaza-components.test.js`：手写 Element Plus 透传替身（render 函数版），只隔离重型表栈与网络层，**不拉整个 EP 进测试环境**；`@/api` 用 `vi.mock` 整体替换，调用结果由各用例自行控制，不 mock 业务逻辑。

### 1.1 `src/modules/admin-lowcode/schema-crud-page.smoke.test.js`（4 例）
走 `useCrudPage + SchemaCrudPage` 引擎路径，喂**真实 pageSchema**（`config.page.js` / `access.page.js`），只 stub `SearchForm/DataTable/FormDialog/SchemaRenderer/el-button`。

| 用例 | 覆盖点 |
|------|--------|
| config 页 · 挂载即渲染：加载中表格进入 loading 态，resolve 后落出行数据 | `onMounted→loadList` 真实触发；服务端分页参数 `{pageNum,pageSize}` 透传；**列表加载态**（pending 时 `data-loading=true`）；resolve 后行数正确 |
| config 页 · 工具栏与行操作入口存在 | **增**：工具栏「新增参数」按钮存在；**改/删**：每行「编辑」「删除」按钮渲染；点编辑 → FormDialog 标题 = `editTitle`；点新增 → 标题 = `createTitle` |
| config 页 · 列表请求失败 | **错误态**：list reject 后不渲染行，`ElMessage.error` 收到「加载列表失败」 |
| access 页 · 归一化凭证行，吊销入口仅对活跃凭证 | `listKeys` 归一化（status→active）；**行动作 `show` 条件**：活跃行有「吊销」、已吊销行无；工具栏「新建凭证」存在 |

### 1.2 `src/views/admin/panels/AdminUser.smoke.test.js`（3 例）
手写面板轻量冒烟：mock `@/api` 14 个函数，stub `FormDialog`，el-table/el-table-column 用仓库既有的 provide-rows 替身驱动行，其余 EP 标签按未知元素渲染。

| 用例 | 覆盖点 |
|------|--------|
| 渲染不崩：列表落地、总数徽标与新增入口 | mount 不抛错；`getUserList` 调用一次；「共 2 位用户」徽标；「新增用户」按钮存在 |
| 行渲染：两行用户可见，详情/编辑行操作存在 | el-table 行数 = 2；用户名渲染；行操作「详情」「编辑」存在 |
| 点「新增用户」打开表单对话框 | FormDialog 由 hidden 变可见，标题 =「新增用户」 |

### 1.3 `src/views/admin/panels/AdminRole.smoke.test.js`（3 例）
同 AdminUser 策略；额外 stub 关闭态权限弹窗里的 `el-tree`（其作用域插槽 `#default="{ data }"` 在未解析时以空 scope 渲染会崩溃）。

| 用例 | 覆盖点 |
|------|--------|
| 渲染不崩：角色列表落地、总数徽标与新增入口 | `getRoleList` 调用一次；「共 2 个角色」；「新增角色」按钮 |
| 行渲染：两行角色可见，编辑/菜单权限行操作存在 | el-table 行数 = 2；角色名渲染；行操作「编辑」「菜单权限」存在 |
| 点「新增角色」打开表单对话框 | FormDialog 变可见，标题 =「新增角色」 |

> 注：`admin-lowcode` 目录在 health-check 台账里原本是「❌ 未登记 defineModule」的半拉子脚手架（其页面由 `router/modules/system.js` 以 props 挂载）。本次只按任务要求补**引擎路径冒烟**，不把它注册进内核注册表——那是 module-governance 待办第 4 条，不在本次范围。

---

## 2. 图标侧栏改动说明

### 2.1 判断正确补法
导航有三层互不从属的来源（module-governance §3）：
1. `collectNav()` —— 注册表派生（`expert-alliance/index.js` 已声明 6 个 `/alliance/*` nav）；
2. `MODULE_SIDEBAR_CONFIG` —— TheSidebar 模块侧栏（6 个 nav 项已由 `nav.config.js` 末尾自动挂载循环全部落入 `expert.sections[控制台]`，§3.1 已确认零漂移）；
3. `ICON_NAV_GROUPS` —— 64px IconSidebar，**独立硬编码清单**。

§3.3 的缺口是第 ③ 层：6 个 `/alliance/*` 页在第 ① 层已声明、第 ② 层已挂载，唯独没进第 ③ 层。因此**不需要再加 nav 声明**（那会与注册表重复），正确补法是**在 `ICON_NAV_GROUPS` 常量里补 6 个图标入口**。

### 2.2 具体改动
`src/constants/nav.config.js` 的 `ICON_NAV_GROUPS`「能力」组，在既有「专家联盟 `/expert-workspace`」之后追加 6 项，与 `expert-alliance/index.js` 的 nav 一一对应：

| key | 标签 | emoji | path |
|-----|------|-------|------|
| alliance-console | 联盟控制台 | 🎛️ | /alliance/console |
| alliance-collab | 智能协作 | 🧠 | /alliance/collab |
| alliance-orchestration | 专家编排台 | 🧭 | /alliance/orchestration |
| alliance-graph | 协作图谱 | 🔗 | /alliance/graph |
| alliance-sessions | 会话中心 | 💬 | /alliance/sessions |
| alliance-experts | 联盟专家广场 | 🎓 | /alliance/experts |

- 放在「能力」组而非「生态」组：专家联盟属能力域，与既有 `expert` 项同组聚拢。
- **未改动任何既有导航项**：`dashboard/projects/tasks/ai/graph/operators/workflow/expert/market/admin` 原样保留；底部「系统设置 ⚡」仍为 `bottom` 项。
- 6 个 path 都是 `expert-alliance/index.js` 已注册的真实路由，无死链（wiring.test.js 全量守护）。

### 2.3 把"已知漂移"升级为硬门禁
`src/modules/health-check.test.js` 原用例「注册表派生的导航不进图标侧栏属已知漂移（仅记录，不断言）」只断言 `Array.isArray`。修复后漂移已为 `[]`，故将其升级为**零漂移硬断言**：
```js
it('注册表派生的导航项全部进图标侧栏（导航注册→挂载零漂移）', () => {
  expect(h.navDrift.iconMissingModulePages, `图标侧栏缺失模块页: ...`).toEqual([])
})
```
今后任何注册表 nav 项漏进图标侧栏，health-check 直接红灯，防止回退。

---

## 3. health-check 复跑结果

命令：`npx vitest run src/modules/health-check.test.js`（连带 `wiring.test.js`）→ **10/10 通过**。

关键台账输出：

- `注册了但未挂载: []`
- **`图标侧栏缺失的模块页: []`** ← 本次修复目标，漂移清零
- `expert-alliance`：✅ 6 条路由 / ✅ 6 项已挂载 / ✅ 契约往返 / ✅ 18 个测试文件
- `admin-lowcode`：tests 列由「⚠️ 1 个」→「✅ 2 个测试文件」
- admin 面板表：`AdminUser.vue`、`AdminRole.vue` 由「❌ 无测试」→「✅」

剩余 `handwritten` 漂移 4 条（`/expert-workspace` `/expert-plaza` `/expert-center` `/expert-config`）是 §3.2 已登记的 **legacy 手写侧栏条目**，属"待迁入注册表"的既有治理债，不在本次「6 个 /alliance/* 进图标栏」范围内，未改动。

---

## 4. vitest 全量结果

命令：`npx vitest run` → **Test Files 35 passed (35) / Tests 700 passed (700)**，exit 0。

- 既有基线 31 文件 / 684 用例全绿之上，新增 3 个测试文件 / 10 个用例（4 + 3 + 3），全部通过。
- 未破坏任何既有面板功能；wiring 6 例、health-check 4 例、pageSchema 10 例、stores/api 等其余用例全绿。

---

## 5. 遗留缺口

1. **admin 17 面板仍有 15 个无测试**：本次只补 AdminUser/AdminRole 两个核心手写面板；AdminConfig/AdminAccess 的低代码引擎路径已测，但手写旧面板 `views/admin/panels/AdminConfig.vue`/`AdminAccess.vue` 本身未单独冒烟（二者功能已被 `*-lc` 低代码页等价覆盖）。其余 13 面板（Audit/Llm/Monitor/Hitl 等）按同模式可后续补。
2. **admin-lowcode 未注册进内核**：仍无 `index.js`/`defineModule`/`endpoints.js` 契约（module-governance 待办 #4）。本次仅补引擎冒烟，未做注册表迁移。
3. **4 个 expert legacy 侧栏条目待迁入注册表**（§3.2）：`/expert-workspace` 等仍手写在 `MODULE_SIDEBAR_CONFIG`，未走 `defineModule`。
4. **图标侧栏未来自动化缺口**：`ICON_NAV_GROUPS` 仍是硬编码清单，靠 health-check 单向守护"注册表 nav 必须出现"。长期可考虑让其由 `collectNav()` 自动派生，消除手抄面。
5. 手写面板冒烟中少量 `[Vue warn] Failed to resolve component: el-dialog/el-radio...` 为刻意不拉 EP 的良性告警，不影响断言；如需净化可后续补更多替身。
