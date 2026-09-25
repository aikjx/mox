# 低代码引擎试点报告：AdminTenant 重构

> 日期：2026-09-26
> 蓝图：`reports/markdown/lowcode-dynamic-vue-analysis.md` 第 3.1–3.6 节
> 试点路由：`/admin/tenant-lc`（与手写版 `/admin/tenant` 并存可对比）

---

## 1. 新增文件清单（纯新增，未改动任何现有面板）

| 文件 | 行数 | 职责 |
|---|---:|---|
| `src/modules/admin-lowcode/contract/pageSchema.js` | 78 | PageSchema DSL 规范 + 纯函数校验 |
| `src/modules/admin-lowcode/contract/pageSchema.test.js` | 42 | DSL 校验单测（7 用例） |
| `src/modules/admin-lowcode/engine/widgetRegistry.js` | 30 | 列控件注册表（text/tag + custom 注册） |
| `src/modules/admin-lowcode/engine/SchemaRenderer.vue` | 16 | 递归渲染器（render 函数 + h()） |
| `src/modules/admin-lowcode/engine/SchemaCrudPage.vue` | 140 | 整页编排器（SearchForm+DataTable+FormDialog） |
| `src/modules/admin-lowcode/composables/useCrudPage.js` | 128 | 列表/筛选/增删改/分页唯一逻辑 |
| `src/modules/admin-lowcode/pages/tenant.page.js` | 135 | 租户页 DSL 声明 |
| `src/router/modules/system.js` | +11 | 仅加一条 `tenant-lc` 懒加载路由 |
| **合计新增** | **569** | 其中**引擎可复用 434 行**，**租户页声明 135 行** |

复用了既有 common 组件（未重造）：`DataTable.vue` / `FormDialog.vue` / `SearchForm.vue`。

## 2. 行数对比

| 项 | 行数 |
|---|---:|
| 原 `AdminTenant.vue`（手写，含样式） | **291** |
| 试点 `tenant.page.js`（页面声明，含注释/统计/handler） | **135** |
| 引擎一次性投入（5 文件 + 测试，后续所有页面共享） | **434** |

- **本页声明行数**：291 → 135，**降幅 54%**。
- **引擎 434 行是沉没成本**：下一个 CRUD 页面只需再加一份 ~120 行的 `*.page.js`，不再写引擎。按报告 P1 推广 6 个面板估算，后续每页成本从 ~400 行手写降到 ~120 行声明。
- 注意：本页仍带了"切换租户/启停"两个自定义 handler（~40 行），纯标准 CRUD 页面的 DSL 会更短。

## 3. 功能等价核对表（对照 AdminTenant.vue 逐项）

| 功能 | 原版实现 | 试点实现 | 结果 |
|---|---|---|---|
| 统计卡 ×4（总数/活跃/试用/企业版） | 组件内 computed | `schema.stats[]` + 行级 value 函数 | ✅ 等价 |
| 关键字搜索（name/code 模糊） | filteredList computed | `useCrudPage.filteredRows` 同款逻辑 | ✅ 等价 |
| 状态下拉筛选 | el-select + filteredList | `search.fields[status].select` | ✅ 等价 |
| 表格列：编码/名称/隔离模式/套餐/状态/创建时间 | 手写 el-table-column | `list.columns[]` + tag 控件 | ✅ 等价 |
| 状态/套餐标签着色 | planTag/statusLabel 函数 | `tagTypeOf(row)` 逐行映射 | ✅ 等价 |
| 新增租户按钮 + 刷新按钮 | 手写 el-button | `toolbar[]` | ✅ 等价 |
| 新增/编辑对话框（编码编辑态禁用、状态仅编辑可见） | 手写 el-dialog+el-form | `FormDialog` + `disabledOnEdit/visibleOnEdit` | ✅ 等价 |
| 创建/编辑 payload 差异（创建不带 status，编辑不带 code/mode） | 组件内分支 | `form.buildPayload(formData,isEdit)` | ✅ 等价 |
| 删除确认 + 删除 | ElMessageBox.confirm | `rowActions[].confirm` + 引擎统一 onDelete | ✅ 等价 |
| 启用/停用（T001 隐藏） | toggleStatus | `show: row=>row.code!=='T001'` + handler | ✅ 等价 |
| 切换租户（确认弹窗 + dispatch 事件） | 独立 switchVisible 对话框 | 行 handler 内 ElMessageBox.confirm + 同款 dispatch | ✅ 等价 |
| 行点击打开编辑 | handleRowClick→openEdit | DataTable `@row-click="openEdit"` | ✅ 等价 |
| 列表归一化（数组/list/data 兼容） | 手写三元 | useCrudPage.normalizeList | ✅ 等价 |

## 4. 性能四件套落点

| 硬约束 | 落地位置 |
|---|---|
| ① 组件注册表 + h()，禁止 compile 模板字符串 | `engine/widgetRegistry.js`（`h(ElTag,…)`）+ `SchemaRenderer.vue`（`setup: () => renderCell(...)` render 函数组件）；全引擎无一处字符串模板编译 |
| ② 列表 shallowRef + 表格行 v-memo | `composables/useCrudPage.js`：`const rows = shallowRef([])`；`SchemaCrudPage.vue`：`<SchemaRenderer v-memo="[row.id, row.status, row.plan, row.name]">` |
| ③ 引擎 chunk 路由懒加载 | `router/modules/system.js`：`component: () => import('@/modules/admin-lowcode/engine/SchemaCrudPage.vue')` |
| ④ pageSchema 用 markRaw | `pages/tenant.page.js` 导出即 `markRaw({...})`；`SchemaCrudPage.vue` 二次 `markRaw(assertPageSchema(...))`；`useCrudPage` 内 `markRaw(pageSchema)` |

## 5. 验证结果

- **vitest 全绿**：`Test Files 30 passed (30)`，`Tests 677 passed (677)`。基线 29 文件/670 用例 → 新增 `pageSchema.test.js` 7 用例，无回归。
- **生产构建**：`npx vite build` 通过，新 SFC 编译入 chunk（路由懒加载）。
- **API 实测**：`GET http://127.0.0.1:3020/api/tenant` → 401（路由存活、鉴权拦截）。增删改端点同前缀 `/tenant`（POST/PUT/DELETE），与 `system.api.js` 一致。GUI 登录需真实 IAM，按任务约定以 API 存在性 + 代码审查为准。

## 6. 推广建议

**可直接 schema 化（下一批）**：
`AdminAccess`(226)、`AdminStorage`(120)、`AdminConfig`(336)、`AdminLogs`(361)、`AdminApi`(397)、`AdminHitl`(379)——标准列表+对话框，约 1.5 天可全部迁移。

**需 slot/handler 逃生舱**：
`AdminDict`（左树右表 + 上下移排序，两个 DataTable + 联动 handler）、`AdminMenu`/`AdminDepartment`（树表，treeSelect 表单已支持）。

**保留手写**：
`AdminMonitor`（ECharts+轮询）、`AdminLlm`（连通性测试）、`AdminRole`（菜单树授权）、`AdminUser`（头像上传/角色分配）——其对话框部分可继续用 `FormDialog`，外壳不必进引擎。

## 7. 遗留缺口

1. 服务端分页尚未在引擎落地：当前 `useCrudPage` 是客户端过滤；P1 推广到 User/Post 等大列表时需补 `serverPagination + page/pageSize` 编排（`DataTable` 已支持 `serverPagination` 透传）。
2. `widgetRegistry` 目前只注册了 `text/tag` 两种列控件；select 类字典回显列、操作进度列等待按需注册。
3. AdminView 顶部 tab 未加入试点入口，当前需手动访问 `/admin/tenant-lc` 对比；正式推广后再决定是否并入主 tab。
4. 行动作的 loading 态（启停/切换按钮）目前未做逐行 disabled，与原版行为一致，后续可补。
