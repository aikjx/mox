// misc 族分页读路径的守卫：判据的期望值全部**现读自网关 Rust 源**，不抄副本。
//
// 事实面（本文件现场扫描 platform/gateway/mox-platform-gateway-svc/src/misc.rs）：
//   - 查询串：`struct PaginationQuery` 的 6 个字段名（serde 默认 snake_case）；
//   - 出参壳：`list_tasks_paginated` / `list_projects_paginated` 里 `ok(json!({…}))` 的 8 个键；
//   - 可排序字段：两族各自的 `match sort_by { "x" => … }` 臂 + `_ =>` 回落字段（**两族不通用**）；
//   - page_size 上限：`clamp(1, N)`。
// 为什么要这么钉：ProjectsView 的 loadTasks 此前猜 `data / data.list / data.data / data.tasks`
// 四个键，TaskView 只认 `Array.isArray(data)`——真名是 `items`，一个都没猜中 ⇒ 后端有行也恒空。
// 而猜查询键更阴：发 `pageSize=5`（camelCase）后端**静默忽略**并用默认值，响应里没有任何报错。
import { readdirSync, readFileSync, statSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

import { normPage, pageQuery, PAGE_SIZE_DEFAULT, PAGE_SIZE_MAX, PROJECT_SORTABLE, TASK_SORTABLE, WIRE_PAGE_KEYS, WIRE_QUERY_KEYS } from './paged-list'
import { ApiError } from './envelope'

const HERE = path.dirname(fileURLToPath(import.meta.url))
const SRC = path.resolve(HERE, '../..')
const REPO = path.resolve(SRC, '../..')
const MISC_RS = path.join(REPO, 'platform', 'gateway', 'mox-platform-gateway-svc', 'src', 'misc.rs')

const read = (p) => readFileSync(p, 'utf-8').replace(/\r\n/g, '\n')

/** 扫 src 下所有源文件（含测试与 .vue，禁令按注释同口径）。 */
function walk(dir, acc = []) {
  for (const entry of readdirSync(dir)) {
    const p = path.join(dir, entry)
    if (statSync(p).isDirectory()) walk(p, acc)
    else if (/\.(vue|js|ts)$/.test(entry)) acc.push(p)
  }
  return acc
}

/** 取 `async fn <name>( … ) -> … { … }` 的函数体（按花括号配平，注释里的括号一起数即可）。 */
function fnBody(text, name) {
  const at = text.indexOf(`async fn ${name}(`)
  if (at < 0) throw new Error(`misc.rs 里没有函数 ${name}：读路径的事实源改名了`)
  const open = text.indexOf('{', text.indexOf('->', at))
  let depth = 0
  for (let i = open; i < text.length; i++) {
    if (text[i] === '{') depth++
    else if (text[i] === '}') {
      depth -= 1
      if (depth === 0) return text.slice(open + 1, i)
    }
  }
  throw new Error(`函数 ${name} 的花括号没配平`)
}

/** 出参壳的顶层键：`ok(json!({ "k": … }))` 里深度为 1 的 `"k":`（嵌套的 filters 内部键不算）。 */
function shellKeys(body) {
  const at = body.lastIndexOf('ok(json!(')
  expect(at, '函数体里没有 ok(json!(…) 出参').toBeGreaterThan(-1)
  const seg = body.slice(at)
  const start = seg.indexOf('{')
  const out = []
  let depth = 0
  for (let i = start; i < seg.length; i++) {
    const c = seg[i]
    if (c === '{' || c === '[') { depth += 1; continue }
    if (c === '}' || c === ']') {
      depth -= 1
      if (depth === 0) break
      continue
    }
    if (depth === 1 && c === '"') {
      const m = /^"([a-z_][a-z0-9_]*)"\s*:/.exec(seg.slice(i))
      if (m) out.push(m[1])
      i = seg.indexOf('"', i + 1)
    }
  }
  return [...new Set(out)]
}

/** `match sort_by { "a" => …, _ => x.sort_by(… b.<field> …) }` ⇒ 显式臂 + 回落字段。 */
function sortableFields(body) {
  const at = body.indexOf('match sort_by')
  expect(at, '函数体里没有 match sort_by：可排序字段的事实源改名了').toBeGreaterThan(-1)
  const arm = /\n\s*"([a-z_]+)"\s*=>/
  const arms = []
  const rest = body.slice(at)
  const re = new RegExp(arm.source, 'g')
  let m
  while ((m = re.exec(rest))) arms.push(m[1])
  const dflt = rest.match(/_ =>\s*\w+\.sort_by\(\|[^|]*\|\s*if descending \{ \w+\.(\w+)\.cmp/)
  expect(dflt, '找不到 `_ =>` 回落排序字段').toBeTruthy()
  return { arms, fallback: dflt[1] }
}

const MISC = read(MISC_RS)
const TASK_FN = fnBody(MISC, 'list_tasks_paginated')
const PROJECT_FN = fnBody(MISC, 'list_projects_paginated')
const QUERY_STRUCT = (MISC.match(/struct PaginationQuery\s*\{([^}]*)\}/) || [])[1]
expect(QUERY_STRUCT, 'misc.rs 里没有 struct PaginationQuery ⇒ 查询键判集会空转').toBeTruthy()
const RUST_QUERY_KEYS = [...QUERY_STRUCT.matchAll(/^\s*(\w+)\s*:/gm)].map((x) => x[1])
const TASK_SHELL = shellKeys(TASK_FN)
const PROJECT_SHELL = shellKeys(PROJECT_FN)
const TASK_SORT = sortableFields(TASK_FN)
const PROJECT_SORT = sortableFields(PROJECT_FN)
const RUST_CLAMP = Number((TASK_FN.match(/page_size\.unwrap_or\((\d+)\)\.clamp\((\d+),\s*(\d+)\)/) || [])[3])
const RUST_DEFAULT_PAGE_SIZE = Number((TASK_FN.match(/page_size\.unwrap_or\((\d+)\)/) || [])[1])
const TASK_ITEM_FIELDS = [...(MISC.match(/struct TaskItem\s*\{([^}]*)\}/) || [, ''])[1].matchAll(/^\s*(\w+)\s*:/gm)].map((x) => x[1])

// 判集非空＝扫描器没瞎（本文件所有期望都从这几个集合来）
expect(RUST_QUERY_KEYS.length, 'PaginationQuery 字段数为 0').toBeGreaterThanOrEqual(6)
expect(TASK_SHELL.length, 'tasks 出参壳键数为 0').toBeGreaterThanOrEqual(8)
expect(TASK_SORT.arms.length, 'tasks 显式排序臂为 0').toBeGreaterThanOrEqual(5)
expect(PROJECT_SORT.arms.length, 'projects 显式排序臂为 0').toBeGreaterThanOrEqual(5)
expect(Number.isFinite(RUST_CLAMP), '没读出 page_size 上限').toBe(true)
expect(TASK_ITEM_FIELDS.length, 'TaskItem 字段数为 0').toBeGreaterThanOrEqual(8)

/** 按 Rust 出参壳拼一份 wire 响应（键名逐个来自 TASK_SHELL，不写死任何串）。 */
function wirePage(items, extra = {}) {
  const vals = {
    items,
    total: items.length + 7,
    page: 2,
    page_size: items.length || 1,
    total_pages: 3,
    has_next: true,
    has_prev: true,
    filters: { keyword: null, status: null, sort_by: TASK_SORT.fallback, sort_order: 'desc' },
    ...extra
  }
  const shell = {}
  for (const k of TASK_SHELL) shell[k] = vals[k]
  return shell
}

/** 按 `async function 名(` 切出函数体（花括号配平），用于"每个取列表的函数都必须过归一化"。 */
function asyncFnBodies(text) {
  const out = []
  const re = /async function\s+(\w+)\s*\(/g
  let m
  while ((m = re.exec(text))) {
    const open = text.indexOf('{', text.indexOf(')', m.index))
    if (open < 0) continue
    let depth = 0
    let i = open
    for (; i < text.length; i++) {
      if (text[i] === '{') depth += 1
      else if (text[i] === '}') {
        depth -= 1
        if (depth === 0) break
      }
    }
    out.push({ name: m[1], body: text.slice(open, i + 1) })
  }
  return out
}

describe('misc 族分页读路径：查询串与出参壳都按 wire 真名走', () => {
  it('格 1 · 事实源自洽：两族出参壳同键集，且可排序字段两族确实不通用', () => {
    expect(PROJECT_SHELL).toEqual(TASK_SHELL)
    expect(TASK_SHELL).toEqual(WIRE_PAGE_KEYS)
    // 若哪天两族排序字段被合并，本格先红——paged-list.js 里两份表就得跟着改
    expect(TASK_SORT.arms.filter((x) => !PROJECT_SORT.arms.includes(x)).length, 'tasks 独有排序臂为空 ⇒ 两族表可能已合并').toBeGreaterThan(0)
    expect(PROJECT_SORT.arms.filter((x) => !TASK_SORT.arms.includes(x)).length, 'projects 独有排序臂为空 ⇒ 两族表可能已合并').toBeGreaterThan(0)
  })

  it('格 2 · 前端常量逐项等于 Rust 现读值（常量不许自己证自己）', () => {
    expect(Object.values(WIRE_QUERY_KEYS).sort()).toEqual([...RUST_QUERY_KEYS].sort())
    expect(TASK_SORTABLE).toEqual([TASK_SORT.fallback, ...TASK_SORT.arms])
    expect(PROJECT_SORTABLE).toEqual([PROJECT_SORT.fallback, ...PROJECT_SORT.arms])
    expect(PAGE_SIZE_MAX).toBe(RUST_CLAMP)
    expect(PAGE_SIZE_DEFAULT).toBe(RUST_DEFAULT_PAGE_SIZE)
  })

  it('格 3 · normPage 吃真出参壳：8 个键全部落到 camelCase 视图模型', () => {
    const items = [{ id: 't1' }, { id: 't2' }]
    const p = normPage(wirePage(items))
    expect(p.items).toEqual(items)
    expect(p.degraded).toBe(false)
    expect([p.total, p.page, p.pageSize, p.totalPages, p.hasNext, p.hasPrev]).toEqual([items.length + 7, 2, items.length, 3, true, true])
    expect(p.filters).toMatchObject({ sort_by: TASK_SORT.fallback })
    // 已剥壳的 body 与 axios 响应两种入口都要吃（http.js 的拦截器只剥一层信封）
    expect(normPage({ status: 200, data: { code: 0, msg: 'ok', data: wirePage(items) } }).items).toEqual(items)
    expect(normPage({ code: 0, msg: 'ok', data: wirePage(items) }).total).toBe(items.length + 7)
  })

  it('格 4 · 正对照：被废掉的猜键必须仍然恒空（否则本单元判据失去牙齿）', () => {
    const items = [{ id: 't1' }]
    for (const guessed of ['data', 'list', 'tasks', 'records']) {
      const poisoned = { [guessed]: items, total: 1 }
      expect(normPage(poisoned).items, `猜键 ${guessed} 又被当成列表了`).toEqual([])
      expect(normPage(poisoned).degraded).toBe(false)
    }
    // 真名换掉后（Rust 侧改名）视图会立刻看见空列表，而不是静默兼容一个错键
    expect(normPage(wirePage(items)).items.length).toBe(1)
  })

  it('格 5 · pageQuery 只发 snake_case 且逐键对应 Rust 字段', () => {
    const q = pageQuery({
      page: 2, pageSize: 20, keyword: 'kb', status: 'active', sortBy: TASK_SORT.arms[0], sortOrder: 'asc'
    }, TASK_SORTABLE)
    expect(Object.keys(q).sort()).toEqual([...RUST_QUERY_KEYS].sort())
    expect(q).toMatchObject({ page: '2', page_size: '20', sort_by: TASK_SORT.arms[0], sort_order: 'asc' })
    // camelCase 键一个都不许出现在串里（实测会被后端静默忽略 ⇒ 只有筛错，没有报错）
    expect(JSON.stringify(q)).not.toMatch(/pageSize|sortBy|sortOrder/)
    // 空值不发，非法不发，越界夹到 clamp 上限
    expect(pageQuery({ page: 0, pageSize: -1, keyword: '', status: '', sortBy: '', sortOrder: '' })).toEqual({})
    expect(pageQuery({ pageSize: 9e9 }).page_size).toBe(String(PAGE_SIZE_MAX))
    expect(pageQuery({ sortBy: PROJECT_SORT.arms[0] }, TASK_SORTABLE), '跨族排序字段不许放行').toEqual({})
    expect(pageQuery({ sortOrder: 'sideways' }, TASK_SORTABLE)).toEqual({})
    // 不给 sortable ⇒ 宁可不发 sort_by（后端对非法值是静默回落，不能把"传了不生效"留给线上发现）
    expect(pageQuery({ sortBy: TASK_SORT.arms[0] })).toEqual({})
    expect(pageQuery({ sortBy: TASK_SORT.arms[0] }, PROJECT_SORTABLE)).toEqual({})
  })

  it('格 6 · 裸数组是唯一的降级形状，且带 degraded 标记', () => {
    const p = normPage([{ id: 'x' }])
    expect(p.items).toEqual([{ id: 'x' }])
    expect(p.degraded).toBe(true)
    expect(normPage(null).items).toEqual([])
    expect(normPage(undefined).degraded).toBe(false)
  })

  it('格 7 · 错误信封不许被当成空列表咽下去', () => {
    expect(() => normPage({ code: 500, msg: '任务表读取失败', data: null })).toThrow(ApiError)
    try {
      normPage({ code: 500, msg: 'x', data: null })
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError)
      expect(e.code).toBe(500)
    }
  })

  it('格 8 · 消费者账：每个取 misc 列表的函数都必须过 normPage，猜键链与幻影端点都不许复活', () => {
    const VIEWS = ['views/project/ProjectsView.vue', 'views/project/TaskView.vue']
    const fetchers = []
    for (const rel of VIEWS) {
      for (const fn of asyncFnBodies(read(path.join(SRC, rel)))) {
        if (!/\bget(?:Tasks|Projects)\s*\(/.test(fn.body)) continue
        fetchers.push(`${rel}:${fn.name}`)
        expect(fn.body, `${rel} 的 ${fn.name}() 直接吃了未归一的出参`).toMatch(/normPage\(/)
        expect(fn.body, `${rel} 的 ${fn.name}() 又用猜键取列表`).not.toMatch(/Array\.isArray\(\s*\w+\s*\)\s*\?\s*\w+\s*:/)
      }
    }
    // 分母：接线位点数由源码现数，塌缩（改函数名/改写法）时本格先红而不是静默通过
    expect(fetchers.length, '取 misc 列表的函数数低于实测下界 ⇒ 本探针空转').toBeGreaterThanOrEqual(5)
    // 幻影路由 src 级禁令（含注释，与 contract/forbidden-revival 同口径）。
    // 写法上刻意不把路径拼成引号包住的完整字面量 ⇒ 本正则匹配不到自己的源码，无需自豁免。
    const PHANTOM = /["'`]\/(?:tasks|projects)\/paginated|get(?:Tasks|Projects)Paginated/
    const files = walk(SRC)
    const offenders = files
      .map((f) => path.relative(SRC, f).replace(/\\/g, '/'))
      .filter((rel) => PHANTOM.test(read(path.join(SRC, rel))))
    expect(files.length, '扫描集塌缩 ⇒ 本禁令是假阴').toBeGreaterThanOrEqual(200)
    expect(offenders, `幻影分页端点又出现了：${offenders.join(', ')}`).toEqual([])
    // 列表与分页是同一条路由：/api/tasks 与 /api/projects 都注册在 misc.rs 上
    expect(MISC).toMatch(/\.route\("\/api\/tasks",\s*get\(list_tasks_paginated\)\)/)
    expect(MISC).toMatch(/\.route\("\/api\/projects",\s*get\(list_projects_paginated\)\)/)
  })

  it('格 9 · TaskItem 的 wire 字段名不许被就地改名（视图直接绑这些键）', () => {
    for (const f of ['id', 'title', 'status', 'priority', 'progress']) {
      expect(TASK_ITEM_FIELDS, `TaskItem 少了 ${f}`).toContain(f)
    }
    // 视图里绑定的任务字段必须都在出参结构体里（漏一个就是渲染成 undefined）
    const bound = new Set()
    for (const rel of ['views/project/TaskView.vue', 'views/project/ProjectsView.vue']) {
      for (const m of read(path.join(SRC, rel)).matchAll(/\bt\.(id|title|description|status|priority|progress|due_date|assignee|project_id)\b/g)) bound.add(m[1])
    }
    expect(bound.size, '视图里没绑定任何 TaskItem 字段 ⇒ 探针空转').toBeGreaterThan(3)
    const missing = [...bound].filter((k) => !TASK_ITEM_FIELDS.includes(k))
    expect(missing, `视图绑定了 TaskItem 里没有的字段：${missing.join(', ')}`).toEqual([])
  })
})
