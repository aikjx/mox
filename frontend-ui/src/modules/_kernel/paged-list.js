// misc 族分页列表的单一口径：网关 `platform/gateway/mox-platform-gateway-svc/src/misc.rs`
// 用 `json!` 内联拼出 `{items,total,page,page_size,total_pages,has_next,has_prev,filters}`，
// 查询键是 serde 默认 snake_case（`page/page_size/keyword/status/sort_by/sort_order`）。
//
// 为什么要这个文件：`/api/tasks` 与 `/api/projects` 的两个消费者此前各自猜列表键——
// ProjectsView 试 `data / data.list / data.data / data.tasks` 四个分支，TaskView 只认
// `Array.isArray(data)`——**四个分支里没有一个是真名 `items`**，于是后端有行也恒空。
// `src/api/http.js` 的拦截器已经把 `{code,msg,data}` 剥到 `data`，视图拿到的就是这层分页壳。
//
// 出参统一成 camelCase 给模板用（模板里 `page/pageSize/total` 是本仓既有写法），
// 入参一律 snake_case：实测发 `pageSize=5&sortBy=title` 会被**静默忽略**（响应回显
// `page_size:20, sort_by:"created_at"`，而发 snake_case 才生效）⇒ 猜大小写没有报错，只有筛错。
//
// 本文件的常量都不是抄来的：`paged-list.test.js` 现场解析 misc.rs 比对（含每族各自的
// `match sort_by` 臂——tasks 可按 title 排，projects 可按 name 排，两族不通用）。
import { unwrap } from './envelope'

export const PAGE_SIZE_MAX = 100
export const PAGE_SIZE_DEFAULT = 20

// wire 查询键（左＝视图侧 camelCase 状态名，右＝后端 serde 认的名字）
export const WIRE_QUERY_KEYS = {
  page: 'page',
  pageSize: 'page_size',
  keyword: 'keyword',
  status: 'status',
  sortBy: 'sort_by',
  sortOrder: 'sort_order'
}

// 分页壳的 8 个键（wire 名），normPage 逐个点名，不靠猜
export const WIRE_PAGE_KEYS = ['items', 'total', 'page', 'page_size', 'total_pages', 'has_next', 'has_prev', 'filters']

// misc.rs 里 `list_tasks_paginated` 的 `match sort_by` 臂（`_` 回落到 created_at）
export const TASK_SORTABLE = ['created_at', 'title', 'status', 'priority', 'progress', 'due_date']
// misc.rs 里 `list_projects_paginated` 的 `match sort_by` 臂
export const PROJECT_SORTABLE = ['created_at', 'name', 'status', 'progress', 'member_count', 'task_count']

/** camelCase 视图状态 → wire 查询串。
 *  `sortable` 必须显式给（`TASK_SORTABLE` / `PROJECT_SORTABLE`）：不给就不发 sort_by——
 *  后端对非法值是**静默回落**（实测 `sort_by=zzz` 只被回显进 `filters`，排序照旧 created_at），
 *  所以把非法值挡在前端才有诊断价值，而"传了却不生效"必须响。 */
export function pageQuery(input = {}, sortable = null) {
  const q = {}
  for (const [local, wire] of Object.entries(WIRE_QUERY_KEYS)) {
    const v = input[local]
    if (v === undefined || v === null || v === '') continue
    if (wire === 'page' || wire === 'page_size') {
      const n = Number(v)
      if (!Number.isFinite(n) || n < 1) continue
      q[wire] = String(wire === 'page_size' ? Math.min(n, PAGE_SIZE_MAX) : Math.floor(n))
      continue
    }
    if (wire === 'sort_by') {
      if (!Array.isArray(sortable) || !sortable.includes(String(v))) continue
      q[wire] = String(v)
      continue
    }
    if (wire === 'sort_order' && v !== 'asc' && v !== 'desc') continue
    q[wire] = String(v)
  }
  return q
}

/** 把响应（axios 响应／已剥壳 body／裸数组）归成分页视图模型。
 *  裸数组是**唯一**允许的降级：后端哪天直接返回数组也不至于白屏，但列表键不再靠猜。 */
export function normPage(res, { nesting = 'flat' } = {}) {
  const payload = Array.isArray(res) ? res : unwrap(res, { nesting })
  if (Array.isArray(payload)) {
    return {
      items: payload, total: payload.length, page: 1, pageSize: payload.length || PAGE_SIZE_MAX,
      totalPages: 1, hasNext: false, hasPrev: false, filters: null, degraded: true
    }
  }
  const p = payload && typeof payload === 'object' ? payload : {}
  const items = Array.isArray(p.items) ? p.items : []
  const total = Number.isFinite(p.total) ? p.total : items.length
  const pageSize = Number.isFinite(p.page_size) ? p.page_size : items.length
  return {
    items,
    total,
    page: Number.isFinite(p.page) ? p.page : 1,
    pageSize,
    totalPages: Number.isFinite(p.total_pages) ? p.total_pages : (pageSize > 0 ? Math.ceil(total / pageSize) : 0),
    hasNext: p.has_next === true,
    hasPrev: p.has_prev === true,
    filters: p.filters && typeof p.filters === 'object' ? p.filters : null,
    degraded: false
  }
}
