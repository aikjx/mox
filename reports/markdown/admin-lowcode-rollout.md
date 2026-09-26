# 低代码引擎 P1 推广报告：Config / Access 迁移

> 日期：2026-09-26
> 前置：`admin-lowcode-pilot.md`（tenant 试点）
> 试点路由：`/admin/config-lc`、`/admin/access-lc`（与手写版并存）

---

## 1. 重要修正：5 个候选面板的真实分类

线二任务清单列了 5 个面板。逐行读完源码后，按"是否标准 CRUD"重新分类（纠正试点报告第 6 节仅凭行数的估计）：

| 面板 | 行数 | 真实性质 | 结论 |
|---|---:|---|---|
| AdminConfig | 336 | 标准 CRUD + 服务端分页 | ✅ 已迁移 |
| AdminAccess | 226 | 列表+创建+吊销（含自定义校验面板） | ✅ CRUD 面迁移 |
| AdminLogs | 361 | **SSE 实时日志控制台**（tail -f、级别调整、清空） | ❌ 保留手写 |
| AdminApi | 397 | **只读注册表**（无增删改，内联启用开关+详情弹窗） | ❌ 保留手写 |
| AdminHitl | 379 | **WebSocket 审批流**（事件推送、批准/拒绝/修改 payload） | ❌ 保留手写 |

理由：Logs/Hitl 是事件驱动（SSE/WS）实时页，Api 是只读注册表——三者都没有"列表+新增对话框+删除"的 CRUD 循环，强行塞进 CRUD 引擎会丢功能。引擎定位是"标准 CRUD"，不是"万能页"。

## 2. 引擎补强内容（P1 前置）

| 缺口 | 落地 |
|---|---|
| 服务端分页 | `useCrudPage.js`：`serverPagination:true` 时 `api.list({pageNum,pageSize,...filters})`，回写 `rows/total`；`onPageChange` 接 DataTable `page-change` 事件；搜索变更 `searchNonce` 重挂载重置内部分页 |
| 新列控件 | `widgetRegistry.js`：新增 `dict`（静态字典 map→标签）、`date`（时间格式化，支持 emptyText）、`arrayTags`（权限数组多标签） |
| checkboxGroup 表单控件 | `common/FormDialog.vue` 增加 `type:'checkboxGroup'` 分支（纯新增，不影响既有 input/select/radio）；DSL 白名单同步加入 |
| 创建后钩子 | `useCrudPage.onSubmit`：`form.afterCreate(created, ctx)`，用于 Access 明文 key 一次性弹窗 |
| 行动作 disabled | `SchemaCrudPage` actions 槽支持 `a.disabled(row)`（Config 内置参数禁删） |

## 3. 行数对比

| 面板 | 原手写 | 新声明 | 降幅 |
|---|---:|---:|---:|
| AdminConfig | 336 | **78** | -77% |
| AdminAccess | 226 | **94** | -58% |

引擎增量（复用，不随页面数增长）：useCrudPage 140 行（重写）、widgetRegistry 40 行、FormDialog +6 行、SchemaCrudPage 微调。

## 4. 功能等价核对表

### 4.1 Config（/admin/config-lc）

| 功能 | 结果 |
|---|---|
| name/key 双文本搜索 | ✅ search.fields 两个 input |
| 服务端分页（pageNum/pageSize/total） | ✅ serverPagination 透传 |
| 列：名称/键名/键值/内置标签/状态标签/备注/创建时间 | ✅ dict+date 控件 |
| 编辑（键名编辑态禁用） | ✅ disabledOnEdit |
| 删除（内置参数禁删） | ✅ rowAction.disabled |
| 刷新缓存工具栏按钮 | ✅ toolbar.handler |
| 表单校验（name/key/value 必填） | ✅ FormDialog rules |

### 4.2 Access（/admin/access-lc）

| 功能 | 结果 |
|---|---|
| 列表：名称/权限标签/创建时间/最近使用/状态 | ✅ arrayTags+date+tag |
| 行归一化（status→active、scopes→permissions） | ✅ api.list 包装函数 |
| 新建凭证（名称+权限 checkboxGroup） | ✅ checkboxGroup 控件 |
| 明文 key 一次性展示 | ✅ afterCreate 钩子弹窗 |
| 吊销确认+吊销（已吊销行不显示按钮） | ✅ rowAction.show+handler |
| 凭证校验面板 / 复制按钮 | ⚠️ 保留手写（开发工具，非 CRUD） |

## 5. 验证结果

- **vitest**：`Test Files 30 passed`，`Tests 680 passed`（基线 30/677，新增 config/access/checkboxGroup 3 用例，零回归）。
- **生产构建**：`vite build` 通过，新 SFC 编译入懒加载 chunk。
- **API curl 实测**：
  - `GET /api/system/config` → 401（路由存活，鉴权拦截）
  - `GET /api/security/api-keys` → 401（路由存活，鉴权拦截）
  - 写操作依赖登录态，按约定以代码审查为准。

## 6. 性能四件套（保持）

h() 渲染（widgetRegistry）/ shallowRef+v-memo（useCrudPage 行数据 + 单元格）/ 路由懒加载 / markRaw（两个 page 导出即 markRaw）。

## 7. 剩余缺口与下一批建议

**缺口**：
1. Access 的"凭证校验面板"与复制按钮未进引擎——需引擎支持 `page.extraPanels` 槽位，下批再做。
2. 服务端分页当前仅 Config 一例；User/Post 等大列表接入时再压测。
3. 行级按钮 loading（吊销/启停）未做。

**下一批**：AdminDict（主子表）、AdminMenu（树表，treeSelect 已支持）。
**保留手写（再次确认）**：AdminMonitor / AdminLlm / AdminRole / AdminUser / AdminLogs(SSE) / AdminHitl(WS) / AdminApi(只读注册表)。
