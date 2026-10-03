// 会话契约门禁：contract/sessions.js、model/normalize.js 两处前端单源必须与网关
// alliance/experts_session.rs 的 11 个 handler、experts_common.rs 的
// ExpertSession / SessionMessage / parse_pagination / text_similarity 逐字对齐。
// 请求体一律走 serde 默认反序列化：字段名写错不报错、只被静默丢弃，
// 所以"前端发出的键后端有没有"是本文件最要紧的一条断言。
import { describe, it, expect } from 'vitest'
import { readFileSync, existsSync } from 'node:fs'
import path from 'node:path'

import {
  ARCHIVE_STATUS, CREATE_SESSION_FIELDS, MESSAGE_RATING, MESSAGE_ROLES, MSG_TYPES, MSG_TYPE_DEFAULT, UPDATE_SESSION_FIELDS,
  SEMANTIC_SCORE_FLOOR, SEMANTIC_SEARCH_DEFAULTS, SESSION_PAGE, SESSION_PAGE_SIZES, SESSION_QUERY_KEYS,
  SESSION_STATUS_COUNTED, SESSION_STATUSES, SESSION_STATUS_DEFAULT, SESSION_NOT_FOUND_HINT, SESSION_TYPES,
  SESSION_TYPE_DEFAULT, SIMILAR_SEARCH_DEFAULTS, THREAD_RENDER_LIMIT,
  activeMinutes, appendMessageBody, appendMessageProblem, createSessionBody, createSessionProblem,
  exportFileName, exportText, isNotFound, messageRoleLabel, metadataRowsToObject, msgTypeLabel,
  normalizeFilters, semanticScopeText, semanticSearchBody, semanticSearchProblem, sessionDraftTitle,
  sessionListQuery, sessionPageCount, sessionStatusProblem, sessionStatusLabel, sessionTypeLabel,
  sessionUpdatePatch, similarSearchBody, similarSearchProblem, similarTruncated, threadWindow
} from './sessions.js'
import { ENDPOINTS } from './endpoints.js'
import {
  normSession, normSessionDelete, normSessionExport, normSessionList, normSessionMessage, normSessionStats,
  normSemanticSearch, normSimilarSearch
} from '../model/normalize.js'

function findRepoRoot(from) {
  let dir = from
  for (let i = 0; i < 8; i++) {
    if (existsSync(path.join(dir, 'docs/API-REGISTRY.md')) && existsSync(path.join(dir, 'platform'))) return dir
    const parent = path.dirname(dir)
    if (parent === dir) break
    dir = parent
  }
  throw new Error(`未找到仓库根（自 ${from} 向上）`)
}

const ROOT = findRepoRoot(process.cwd())
const src = (rel) => readFileSync(path.join(ROOT, rel), 'utf8').replace(/\r\n/g, '\n')

const SESSION_RS = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_session.rs')
const COMMON_RS = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_common.rs')
const ACTUATOR_RS = src('platform/gateway/mox-platform-gateway-svc/src/actuator.rs')
const REGISTRY_MD = src('docs/API-REGISTRY.md')
const NORMALIZE_JS = src('frontend-ui/src/modules/expert-alliance/model/normalize.js')

/** 从 openIdx 的 { 起取平衡块（跳过字符串字面量里的括号） */
function balanced(text, openIdx) {
  let depth = 0
  let inStr = false
  let esc = false
  for (let i = openIdx; i < text.length; i++) {
    const c = text[i]
    if (esc) { esc = false; continue }
    if (inStr) {
      if (c === '\\') esc = true
      else if (c === '"') inStr = false
      continue
    }
    if (c === '"') { inStr = true; continue }
    if (c === '{' || c === '[' || c === '(') depth++
    else if (c === '}' || c === ']' || c === ')') {
      depth--
      if (depth === 0) return text.slice(openIdx, i + 1)
    }
  }
  throw new Error('块未闭合，解析器该修了')
}

function rustFn(header, text = SESSION_RS) {
  const at = text.indexOf(header)
  expect(at, `Rust 源里找不到 ${header}`).toBeGreaterThan(-1)
  return balanced(text, text.indexOf('{', at))
}

/** 结构体字段：逐行解析，属性行归到紧随其后的字段上（跨字段正则会张冠李戴） */
function structFields(structBody) {
  const out = []
  let pending = []
  for (const line of structBody.split('\n')) {
    const attr = line.match(/^\s*(#\[.+\])\s*$/)
    if (attr) { pending.push(attr[1]); continue }
    const field = line.match(/^\s*(?:pub\s+)?(\w+)\s*:\s*(.+?),\s*$/)
    if (field) {
      out.push({ name: field[1], type: field[2].trim(), attrs: pending })
      pending = []
    }
  }
  expect(out.length, '一个字段都没解析出来，结构体写法变了').toBeGreaterThan(0)
  return out
}

const fieldNames = (body) => structFields(body).map((f) => f.name)
/** 没有 #[serde(default)] 的字段：请求体缺它即 422 */
const requiredFields = (body) =>
  structFields(body).filter((f) => !f.attrs.some((a) => /serde\(default/.test(a))).map((f) => f.name)

/** 字段文档注释里的取值清单：`/// 会话类型：single / multi / debate / enterprise` */
function docEnum(structBody, field) {
  const lines = structBody.split('\n')
  const idx = lines.findIndex((l) => new RegExp(`^\\s*(?:pub\\s+)?${field}\\s*:`).test(l))
  expect(idx, `结构体里没有字段 ${field}`).toBeGreaterThan(-1)
  let doc = ''
  for (let i = idx - 1; i >= 0; i--) {
    const line = lines[i]
    if (/^\s*#\[/.test(line)) continue
    if (!/^\s*\/\/\//.test(line)) break
    doc = line.replace(/^\s*\/\/\/\s*/, '') + doc
  }
  const listed = doc.match(/：\s*([a-z_ /]+)\s*$/)?.[1]
  expect(listed, `${field} 的文档注释没给出取值清单，前端枚举就成了无源之水`).toBeTruthy()
  return listed.split('/').map((s) => s.trim()).filter(Boolean)
}

const jsonStarts = (body) => {
  const starts = []
  let at = body.indexOf('json!({')
  while (at >= 0) {
    starts.push(body.indexOf('{', at))
    at = body.indexOf('json!({', at + 7)
  }
  expect(starts.length, '该 handler 里没有 json! 字面量').toBeGreaterThan(0)
  return starts
}

/** 收集函数里每个 json! 对象的顶层键（并集） */
function jsonFaces(body) {
  const top = new Set()
  for (const open of jsonStarts(body)) {
    const obj = balanced(body, open)
    let depth = 0
    let inStr = false
    let esc = false
    let cursor = 0
    let pending = null
    for (let i = 0; i < obj.length; i++) {
      const c = obj[i]
      if (esc) { esc = false; continue }
      if (inStr) {
        if (c === '\\') esc = true
        else if (c === '"') { inStr = false; pending = obj.slice(cursor, i) }
        continue
      }
      if (c === '"') { inStr = true; cursor = i + 1; continue }
      if (c === '{' || c === '[' || c === '(') depth++
      else if (c === '}' || c === ']' || c === ')') depth--
      else if (c === ':' && depth === 1 && pending !== null) {
        top.add(pending)
        pending = null
      }
    }
  }
  expect(top.size, '一个顶层键都没解析出来').toBeGreaterThan(0)
  return top
}

function normFn(name) {
  const at = NORMALIZE_JS.indexOf(`export function ${name}(`)
  expect(at, `normalize.js 里找不到 ${name}`).toBeGreaterThan(-1)
  return balanced(NORMALIZE_JS, NORMALIZE_JS.indexOf('{', at))
}

const readsKey = (fnSrc, key) => new RegExp(`\\.${key}(?:\\?\\.|\\b)`).test(fnSrc)
const snakeReads = (fnSrc) =>
  [...new Set([...fnSrc.matchAll(/\.([a-z][a-z0-9]*(?:_[a-z0-9]+)+)\b/g)].map((m) => m[1]))]

/** 双向对齐：后端给的键前端必须都读，前端读的 wire 键后端必须真给 */
function assertFace(handlerHeader, normName, extraSrc = '') {
  const body = rustFn(handlerHeader)
  const face = jsonFaces(body)
  const fnSrc = normFn(normName)
  for (const key of face) {
    expect(readsKey(fnSrc, key), `${normName} 没读后端返回的 ${key}（能力被吞掉）`).toBe(true)
  }
  for (const key of snakeReads(fnSrc)) {
    expect(`${body}\n${extraSrc}`, `${normName} 读了 ${key}，后端不产出它`).toContain(`"${key}"`)
  }
  return face
}

/** 前端发出的每个键后端都得有：多一个键不会报错，只会被 serde 静默丢弃 */
function assertWireKeys(obj, allowed, label) {
  for (const key of Object.keys(obj)) {
    expect(allowed, `${label} 发出了后端没有的字段 ${key}`).toContain(key)
  }
}

/** build_experts_session_router 的 (path → 允许的方法/handler) */
function routed(routerBody) {
  const chunks = routerBody.split('.route(').slice(1)
  return chunks.map((chunk) => {
    const path = chunk.match(/"([^"]+)"/)?.[1]
    expect(path, '路由块里没有路径字面量').toBeTruthy()
    const pairs = [...chunk.matchAll(/\b(get|post|put|delete)\((\w+)\)/g)]
      .map((m) => ({ method: m[1].toUpperCase(), handler: m[2] }))
    expect(pairs.length, `${path} 一个 handler 都没挂上`).toBeGreaterThan(0)
    return { path, methods: pairs.map((p) => p.method), handlers: pairs.map((p) => p.handler) }
  })
}

const sessionEndpoints = () =>
  Object.entries(ENDPOINTS).filter(([name]) => name === 'sessionsList' || name === 'semanticSearch' || name.startsWith('session'))

const ROUTED = routed(rustFn('pub fn build_experts_session_router'))
const BY_PATH = Object.fromEntries(ROUTED.map((r) => [r.path, r]))
const CREATE_BODY_RS = rustFn('pub struct CreateSessionBody')
const UPDATE_BODY_RS = rustFn('pub struct UpdateSessionBody')
const APPEND_BODY_RS = rustFn('pub struct AppendMessageBody')
const SIMILAR_BODY_RS = rustFn('pub struct SimilarSearchBody')
const SEMANTIC_BODY_RS = rustFn('pub struct SemanticSearchBody')
const SESSION_MODEL_RS = rustFn('pub struct ExpertSession', COMMON_RS)
const MESSAGE_MODEL_RS = rustFn('pub struct SessionMessage', COMMON_RS)
const LIST_RS = rustFn('async fn list_sessions(')
const STATS_RS = rustFn('async fn session_stats(')
const CREATE_RS = rustFn('async fn create_session(')
const UPDATE_RS = rustFn('async fn update_session(')
const APPEND_RS = rustFn('async fn append_message(')
const SIMILAR_RS = rustFn('async fn similar_search(')
const SEMANTIC_RS = rustFn('async fn semantic_search(')
const EXPORT_RS = rustFn('async fn export_session(')
const ARCHIVE_RS = rustFn('async fn archive_session(')
const DELETE_RS = rustFn('async fn delete_session(')
const PAGINATION_RS = rustFn('pub fn parse_pagination(', COMMON_RS)
const SIMILARITY_RS = rustFn('pub fn text_similarity(', COMMON_RS)

describe('会话端点面', () => {
  it('契约里的 11 条与 build_experts_session_router 注册的方法逐字等集', () => {
    const pairs = ROUTED.flatMap((r) => r.methods.map((m) => `${m} ${r.path}`))
    expect(pairs.length).toBe(11)
    const contracted = sessionEndpoints()
    expect(contracted.length).toBe(11)
    expect(contracted.map(([, ep]) => `${ep.method} ${ep.path}`).sort()).toEqual(pairs.sort())
    for (const [name, ep] of contracted) {
      expect(ep.nesting, `${name} 信封不是 flat`).toBe('flat')
      expect(BY_PATH[ep.path].handlers.length, `${ep.path} handler 数为 0`).toBeGreaterThan(0)
    }
  })

  it('registry 行与 actuator 登记、API-REGISTRY.md 三处同源；ANY 只作动词通配', () => {
    for (const [name, ep] of sessionEndpoints()) {
      const row = ACTUATOR_RS.match(new RegExp(`r\\("${ep.registry}", "(\\w+)", "([^"]+)"`))
      expect(row, `actuator 未登记 ${name}（${ep.registry}）`).toBeTruthy()
      expect(row[2], `${ep.registry} 登记的路径与契约不符`).toBe(ep.path)
      if (row[1] !== ep.method) {
        expect(row[1], `${ep.registry} 是 ${row[1]} 行，不是 ${ep.method}`).toBe('ANY')
        expect(['GET', 'PUT', 'DELETE'], `${name} 借用了 ANY 行，但它不是 ANY 覆盖的动词`).toContain(ep.method)
      }
      expect(REGISTRY_MD, `API-REGISTRY.md 缺 ${ep.registry}`).toContain(`\`${ep.registry}\``)
    }
    // 会话家族只有 detail 一行是 ANY，且恰好被 详情/更新/删除 三条契约共用
    const anyRows = [...ACTUATOR_RS.matchAll(/r\("(experts\.session[.\w]*)", "ANY"/g)].map((m) => m[1])
    expect(anyRows).toEqual(['experts.session.detail'])
    const shared = new Set(sessionEndpoints().map(([, ep]) => ep.registry))
    expect(shared.size).toBe(9)
  })

  it('列表与创建同路径不同方法，注册表为此开了两行', () => {
    const same = sessionEndpoints().filter(([, ep]) => ep.path === '/api/experts/sessions')
    expect(same.map(([, ep]) => ep.method).sort()).toEqual(['GET', 'POST'])
    expect(new Set(same.map(([, ep]) => ep.registry)).size).toBe(2)
    expect(BY_PATH['/api/experts/sessions'].methods.sort()).toEqual(['GET', 'POST'])
  })

  it('stats 抢在 :id 之前注册，否则会被当会话 id——前端两行都得存在', () => {
    const at = ROUTED.findIndex((r) => r.path === '/api/experts/sessions/stats')
    expect(at).toBeGreaterThan(-1)
    expect(ROUTED[at + 1].path).toBe('/api/experts/sessions/:id')
    expect(sessionEndpoints().map(([, ep]) => ep.path)).toContain('/api/experts/sessions/stats')
  })
})

describe('请求体字段面', () => {
  it('CreateSessionBody 七个字段全 Option：空 body 合法，前端也就只发填了的', () => {
    expect(fieldNames(CREATE_BODY_RS).sort()).toEqual([...CREATE_SESSION_FIELDS].sort())
    for (const f of structFields(CREATE_BODY_RS)) {
      expect(f.type, `${f.name} 不再是 Option，缺它即 422，界面文案要改`).toMatch(/^Option</)
    }
    expect(createSessionBody({})).toEqual({})
    expect(createSessionBody()).toEqual({})
    const full = createSessionBody({
      title: '  架构评审  ', expertIds: ['e1', '', 'e2'], userId: ' u1 ',
      sessionType: 'multi', topic: '拆库', tags: ['rust', ''], metadata: { priority: 'high' }
    })
    assertWireKeys(full, CREATE_SESSION_FIELDS, 'createSessionBody')
    expect(full).toEqual({
      title: '架构评审', expert_ids: ['e1', 'e2'], user_id: 'u1',
      session_type: 'multi', topic: '拆库', tags: ['rust'], metadata: { priority: 'high' }
    })
  })

  it('等于后端缺省值的 session_type 不发；空 metadata 不占位', () => {
    expect(createSessionBody({ title: 't', sessionType: 'single' })).not.toHaveProperty('session_type')
    expect(createSessionBody({ title: 't', metadata: {} })).not.toHaveProperty('metadata')
    expect(createSessionBody({ title: 't', metadata: [] })).not.toHaveProperty('metadata')
    expect(createSessionProblem({ sessionType: 'single' })).toBe('')
    expect(createSessionProblem({ sessionType: 'nope' })).toContain('single')
    expect(createSessionProblem({})).toBe('')
  })

  it('标题兜底链是前端给的：后端 title 缺省是空串，不是报错', () => {
    expect(CREATE_RS).toMatch(/title: body\.title\.unwrap_or_default\(\)/)
    expect(sessionDraftTitle({ title: ' 甲 ', topic: '乙' })).toBe('甲')
    expect(sessionDraftTitle({ topic: '如何拆分单体' })).toBe('如何拆分单体')
    expect(sessionDraftTitle({ topic: 'x'.repeat(40) })).toBe(`${'x'.repeat(24)}…`)
    expect(sessionDraftTitle({})).toBe('未命名会话')
    expect(sessionDraftTitle()).toBe('未命名会话')
  })

  it('UpdateSessionBody 只收 5 个键：换专家/换类型/换发起人没有端点', () => {
    expect(fieldNames(UPDATE_BODY_RS).sort()).toEqual([...UPDATE_SESSION_FIELDS].sort())
    expect(fieldNames(UPDATE_BODY_RS)).not.toContain('expert_ids')
    expect(fieldNames(UPDATE_BODY_RS)).not.toContain('session_type')
    const { patch } = sessionUpdatePatch(
      { title: '旧', status: 'active', topic: '旧主题', tags: ['a'], metadata: { k: 'v' } },
      { title: '新', status: 'closed', topic: '新主题', tags: ['a', 'b'], expertIds: ['x'], metadata: { k: 'v', p: '1' } }
    )
    assertWireKeys(patch, UPDATE_SESSION_FIELDS, 'sessionUpdatePatch')
    expect(patch).toEqual({ title: '新', status: 'closed', topic: '新主题', tags: ['a', 'b'], metadata: { p: '1' } })
  })

  it('合并式更新：与当前同值的键不发，metadata 只能改不能删', () => {
    const current = { title: '甲', status: 'active', topic: '', tags: ['a'], metadata: { keep: '1' } }
    expect(sessionUpdatePatch(current, { ...current }).patch).toEqual({})
    const same = sessionUpdatePatch(current, { title: ' 甲 ', status: 'active', topic: '', metadata: { keep: '1' } })
    expect(same.patch).toEqual({})
    expect(same.unchangedMetadataKeys).toEqual(['keep'])
    // draft 没带 tags 就不能动 tags：后端是覆写，漏传键不该清掉标签
    const gone = sessionUpdatePatch(current, { title: '甲', metadata: {} })
    expect(gone.patch).not.toHaveProperty('tags')
    expect(gone.unremovableMetadataKeys).toEqual(['keep'])
    expect(sessionUpdatePatch(current, { tags: [] }).patch).toEqual({ tags: [] })
    // current.metadata 也可能是归一化后的 [{key,value}] 行：差分按键走，不能把数组索引当键
    const rows = sessionUpdatePatch(
      { title: '甲', metadata: [{ key: 'priority', value: 'high' }, { key: 'score', value: 3 }] },
      { title: '甲', metadata: { priority: 'high', fresh: '1' } }
    )
    expect(rows.patch).toEqual({ metadata: { fresh: '1' } })
    expect(rows.unchangedMetadataKeys).toEqual(['priority'])
    expect(rows.unremovableMetadataKeys).toEqual(['score'])
    // 库里是数字 3、表单显示 '3'：这不是用户改过的，不能被回写成字符串
    const coerced = sessionUpdatePatch(
      { title: '甲', metadata: [{ key: 'n', value: 3 }, { key: 'b', value: true }] },
      { title: '甲', metadata: { n: '3', b: 'true' } }
    )
    expect(coerced.patch).toEqual({})
    expect(coerced.unchangedMetadataKeys.sort()).toEqual(['b', 'n'])
    expect(sessionUpdatePatch({ metadata: { n: 3 } }, { metadata: { n: '4' } }).patch).toEqual({ metadata: { n: '4' } })
    // 后端 update_session 只有 insert，没有 remove：上面那条 unremovable 断言就是它的镜像
    expect(UPDATE_RS).toMatch(/session\.metadata\.insert\(k, v\)/)
    expect(UPDATE_RS).not.toMatch(/metadata\.remove/)
  })

  it('metadata 行按字符串上送，不偷偷 JSON.parse', () => {
    expect(metadataRowsToObject([{ key: 'n', value: 1 }, { key: ' b ', value: true }, { key: '  ', value: 'x' }]))
      .toEqual({ n: '1', b: 'true' })
    expect(metadataRowsToObject(null)).toEqual({})
  })

  it('AppendMessageBody 的必填就是前端拦的那两个：role 与 content 无 serde(default)', () => {
    expect(fieldNames(APPEND_BODY_RS).sort())
      .toEqual(['attachments', 'content', 'msg_type', 'rating', 'role', 'sender_id', 'sender_name'].sort())
    expect(requiredFields(APPEND_BODY_RS)).toEqual(['role', 'content'])
    expect(appendMessageProblem({})).toContain('role')
    expect(appendMessageProblem({ role: 'user' })).toContain('content')
    expect(appendMessageProblem({ role: 'user', content: '甲' })).toBe('')
    expect(appendMessageProblem({ role: 'user', content: '甲', rating: 6 })).toContain('0–5')
    expect(appendMessageProblem({ role: 'user', content: '甲', rating: 5 })).toBe('')
    expect(APPEND_RS).toMatch(/role: body\.role/)
    expect(APPEND_RS).toMatch(/content: body\.content/)
  })

  it('消息 body：sender 名字后端不查注册表，等于缺省的 msg_type 不发', () => {
    const body = appendMessageBody({
      role: ' expert ', content: ' 原文 ', senderId: 'e1', senderName: '张三',
      msgType: 'text', attachments: [], rating: 4.7
    })
    assertWireKeys(body, fieldNames(APPEND_BODY_RS), 'appendMessageBody')
    expect(body).toEqual({ role: 'expert', content: ' 原文 ', sender_id: 'e1', sender_name: '张三', rating: 4 })
    expect(appendMessageBody({ role: 'user', content: 'x', msgType: 'markdown' })).toEqual({
      role: 'user', content: 'x', msg_type: 'markdown'
    })
    expect(APPEND_RS).toMatch(/body\.sender_name\.unwrap_or_default\(\)/)
    expect(APPEND_RS).toMatch(/msg_type: body\.msg_type\.unwrap_or_else\(\|\| "text"\.into\(\)\)/)
  })

  it('rating 是 Option<u8>：文档区间 0–5，硬边界在 255', () => {
    const rating = structFields(MESSAGE_MODEL_RS).find((f) => f.name === 'rating')
    expect(rating.type).toBe('Option<u8>')
    expect(rating.attrs.join(' ')).toMatch(/serde\(default\)/)
    expect(MESSAGE_RATING).toEqual({ min: 0, max: 5, wireMax: 255 })
    expect(appendMessageProblem({ role: 'user', content: 'x', rating: 21 })).toContain('0–5')
    // 0 分必须发得出去：rating 是 Option<u8>，0 与"没评分"在后端可区分
    expect(appendMessageBody({ role: 'user', content: 'x', rating: 0 })).toEqual({ role: 'user', content: 'x', rating: 0 })
  })

  it('两个检索 body 各自只有 3 个键，query 必填', () => {
    expect(fieldNames(SIMILAR_BODY_RS).sort()).toEqual(['min_score', 'query', 'top_k'].sort())
    expect(fieldNames(SEMANTIC_BODY_RS).sort()).toEqual(['expert_id', 'query', 'session_type', 'top_k'].sort())
    expect(requiredFields(SIMILAR_BODY_RS)).toEqual(['query'])
    expect(requiredFields(SEMANTIC_BODY_RS)).toEqual(['query'])
    expect(similarSearchProblem({})).toContain('query')
    expect(semanticSearchProblem({})).toContain('query')
    expect(semanticSearchProblem({ query: 'x', sessionType: 'nope' })).toContain('single')
    expect(similarSearchBody({ query: ' 微服务 ', topK: 5, minScore: 0.1 })).toEqual({ query: '微服务' })
    assertWireKeys(similarSearchBody({ query: 'q', topK: 20, minScore: 0 }), fieldNames(SIMILAR_BODY_RS), 'similarSearchBody')
    expect(similarSearchBody({ query: 'q', topK: 20, minScore: 0 })).toEqual({ query: 'q', top_k: 20, min_score: 0 })
    // 全域检索没有 min_score：发出去只会被丢弃，所以 body 里不许出现
    const sem = semanticSearchBody({ query: 'q', topK: 3, minScore: 0.9, sessionType: 'multi', expertId: 'e1' })
    assertWireKeys(sem, fieldNames(SEMANTIC_BODY_RS), 'semanticSearchBody')
    expect(sem).toEqual({ query: 'q', top_k: 3, session_type: 'multi', expert_id: 'e1' })
    expect(semanticSearchBody({ query: 'q', topK: 10 })).toEqual({ query: 'q' })
  })
})

describe('缺省值与钳位', () => {
  it('parse_pagination 的字面量就是 SESSION_PAGE', () => {
    expect(PAGINATION_RS).toMatch(/params\.get\("page"\)[\s\S]*?\.unwrap_or\(1\)/)
    expect(PAGINATION_RS).toMatch(/\.unwrap_or\(20\)/)
    expect(PAGINATION_RS).toMatch(/page\.max\(1\)/)
    expect(PAGINATION_RS).toMatch(/page_size\.clamp\((\d+),\s*(\d+)\)/)
    const [, min, max] = PAGINATION_RS.match(/page_size\.clamp\((\d+),\s*(\d+)\)/)
    expect(SESSION_PAGE).toEqual({ defaultPage: 1, defaultSize: 20, minSize: Number(min), maxSize: Number(max) })
    // limit 只是 page_size 的别名；前端只发一套键，免得互相覆盖
    expect(PAGINATION_RS).toMatch(/\.or_else\(\|\| params\.get\("limit"\)\)/)
    expect(sessionListQuery({ pageSize: 500 })).toEqual({ page_size: 200 })
    expect(sessionListQuery({ pageSize: 0 })).toEqual({ page_size: 1 })
    expect(sessionListQuery({ pageSize: -3 })).toEqual({ page_size: 1 })
    expect(sessionListQuery({ page: 0 })).toEqual({ page: 1 })
    expect(sessionListQuery({ page: 2.9 })).toEqual({ page: 2 })
    expect(sessionListQuery({ pageSize: 50 })).not.toHaveProperty('limit')
    for (const n of SESSION_PAGE_SIZES) {
      expect(n % 1).toBe(0)
      expect(n).toBeGreaterThanOrEqual(SESSION_PAGE.minSize)
      expect(n).toBeLessThanOrEqual(SESSION_PAGE.maxSize)
    }
    expect(SESSION_PAGE_SIZES).toContain(SESSION_PAGE.defaultSize)
  })

  it('空串过滤条件一律不发：后端 Some("") 是真的相等比较，会把结果清光', () => {
    expect(LIST_RS).toMatch(/params\.get\("status"\)\.map\(\|s\| s\.as_str\(\)\)/)
    expect(LIST_RS).toMatch(/if s\.status != st/)
    expect(sessionListQuery({ status: '', search: '   ', userId: null })).toEqual({})
    // 过滤键面：后端 params.get 的键 = 前端会发的 5 个 + 分页器认的 limit 别名
    const seen = new Set([...`${LIST_RS}${PAGINATION_RS}`.matchAll(/params\.get\("(\w+)"\)/g)].map((m) => m[1]))
    expect([...seen].sort()).toEqual([...Object.values(SESSION_QUERY_KEYS), 'limit'].sort())
    expect(Object.values(SESSION_QUERY_KEYS)).not.toContain('limit')
    expect(sessionListQuery({ search: '甲' })).toEqual({ search: '甲' })
    // 后端没有排序参数：界面摆"排序"控件就是假的
    expect(seen.has('sort')).toBe(false)
    expect(normalizeFilters({ page: 'abc', pageSize: '999', search: ' 甲 ' })).toEqual({
      status: '', sessionType: '', expertId: '', userId: '', search: '甲', page: 1, pageSize: 200
    })
  })

  it('两个检索的 unwrap_or 字面量即前端缺省', () => {
    expect(Number(SIMILAR_RS.match(/top_k\.unwrap_or\((\d+)\)/)?.[1])).toBe(SIMILAR_SEARCH_DEFAULTS.topK)
    expect(Number(SIMILAR_RS.match(/min_score\.unwrap_or\(([0-9.]+)\)/)?.[1])).toBe(SIMILAR_SEARCH_DEFAULTS.minScore)
    expect(Number(SEMANTIC_RS.match(/top_k\.unwrap_or\((\d+)\)/)?.[1])).toBe(SEMANTIC_SEARCH_DEFAULTS.topK)
    expect(Object.keys(SEMANTIC_SEARCH_DEFAULTS)).toEqual(['topK'])
    expect(similarSearchProblem({ query: 'q', topK: 0 })).toContain('至少 1')
    expect(similarSearchProblem({ query: 'q', minScore: 1.5 })).toContain('0–1')
    expect(similarSearchProblem({ query: 'q' })).toBe('')
  })

  it('页码越界不报错，只返回空表：total=0 时界面仍停在第 1 页', () => {
    expect(LIST_RS).toMatch(/\.skip\(offset\)/)
    expect(LIST_RS).toMatch(/\.take\(page_size\)/)
    expect(sessionPageCount(0, 20)).toBe(1)
    expect(sessionPageCount(41, 20)).toBe(3)
    expect(sessionPageCount(41, 0)).toBe(3)
    expect(sessionPageCount(undefined, undefined)).toBe(1)
  })
})

describe('出参面双向对齐', () => {
  it('7 个 json! handler 的顶层键全被各自的归一化读到，且没有凭空读的键', () => {
    const pairs = [
      ['async fn list_sessions(', 'normSessionList'],
      ['async fn session_stats(', 'normSessionStats'],
      ['async fn similar_search(', 'normSimilarSearch'],
      ['async fn semantic_search(', 'normSemanticSearch'],
      ['async fn export_session(', 'normSessionExport'],
      ['async fn archive_session(', 'normSessionArchive'],
      ['async fn delete_session(', 'normSessionDelete']
    ]
    for (const [header, normName] of pairs) {
      expect(assertFace(header, normName, COMMON_RS).size, normName).toBeGreaterThan(1)
    }
  })

  it('列表项是投影后的视图：14 个键、含可信租户与 message_count、不含 messages', () => {
    const view = rustFn('fn session_to_list_view(')
    const face = [...jsonFaces(view)].sort()
    expect(face).toEqual([
      'archived_at', 'created_at', 'expert_ids', 'id', 'last_active_at', 'message_count', 'metadata',
      'session_type', 'status', 'tags', 'tenant_id', 'title', 'topic', 'user_id'
    ])
    expect(face).not.toContain('messages')
    expect(LIST_RS).toMatch(/\.map\(session_to_list_view\)/)
    const rows = normSessionList({ sessions: [{ id: 's1', message_count: 3 }], total: 3, page: 1, page_size: 20 })
    expect(rows.items[0].messageCount).toBe(3)
    expect(rows.items[0].messages).toEqual([])
    expect(normSessionList(null)).toEqual({ items: [], total: 0, page: 1, pageSize: 20 })
    // 详情视图（结构体直接序列化）没有 message_count，只能数 messages
    expect(normSession({ id: 's1', messages: [{ id: 'm1' }] }).messageCount).toBe(1)
  })

  it('ExpertSession 与 SessionMessage 的字段面全被读到，一个都没漏', () => {
    const sFace = fieldNames(SESSION_MODEL_RS)
    expect(sFace.length).toBe(14)
    expect(normSession({ tenant_id: 'tenant-a' }).tenantId).toBe('tenant-a')
    const sSrc = normFn('normSession')
    for (const key of sFace) expect(readsKey(sSrc, key), `normSession 丢了会话字段 ${key}`).toBe(true)
    for (const key of snakeReads(sSrc)) expect(SESSION_MODEL_RS + '\n' + SESSION_RS).toContain(`"${key}"`)

    const mFace = fieldNames(MESSAGE_MODEL_RS)
    expect(mFace.sort()).toEqual(
      ['attachments', 'content', 'created_at', 'id', 'msg_type', 'rating', 'role', 'sender_id', 'sender_name'].sort()
    )
    const mSrc = normFn('normSessionMessage')
    for (const key of mFace) expect(readsKey(mSrc, key), `normSessionMessage 丢了消息字段 ${key}`).toBe(true)
  })

  it('rating 缺失与 0 分必须可区分，因为后端是 Option<u8>', () => {
    expect(normSessionMessage({ id: 'm' }).rating).toBe(null)
    expect(normSessionMessage({ id: 'm', rating: null }).rating).toBe(null)
    expect(normSessionMessage({ id: 'm', rating: 0 }).rating).toBe(0)
    expect(structFields(MESSAGE_MODEL_RS).find((f) => f.name === 'rating').type).toBe('Option<u8>')
  })

  it('统计面键精确（外层 11 个 + top_experts 行内 2 个），type_distribution 是 map 不是数组', () => {
    const face = [...jsonFaces(STATS_RS)].sort()
    expect(face).toEqual([
      'active_sessions', 'archived_sessions', 'avg_messages_per_session', 'avg_session_duration_minutes',
      'closed_sessions', 'count', 'expert_id', 'session_type_distribution', 'sessions_today',
      'top_experts_by_sessions', 'total_messages', 'total_sessions', 'ts'
    ])
    expect(STATS_RS).toMatch(/type_dist: HashMap<String, u64>/)
    const s = normSessionStats({
      total_sessions: 2, active_sessions: 1, archived_sessions: 1, total_messages: 3,
      avg_messages_per_session: 1.5, session_type_distribution: { single: 1, multi: 1 },
      top_experts_by_sessions: [{ expert_id: 'e1', count: 2 }]
    })
    expect(s.typeDistribution).toEqual([{ type: 'single', count: 1 }, { type: 'multi', count: 1 }])
    expect(s.topExpertsBySessions[0]).toEqual({ expertId: 'e1', count: 2 })
    expect(s.closedSessions).toBe(0)
    expect(s.avgSessionDurationMinutes).toBe(0)
  })

  it('top_experts 按计数降序 take(10)，同计数先后取决于 HashMap 迭代序', () => {
    expect(STATS_RS).toMatch(/top_experts\.sort_by\(\|a, b\| b\.1\.cmp\(&a\.1\)\)/)
    expect(STATS_RS).toMatch(/\.take\(10\)/)
    expect(STATS_RS).toMatch(/expert_counts\.into_iter\(\)/)
    const ids = normSessionStats({
      top_experts_by_sessions: Array.from({ length: 12 }, (_, i) => ({ expert_id: `e${i}`, count: 1 }))
    }).topExpertsBySessions.map((e) => e.expertId)
    expect(ids.length).toBe(12)
  })

  it('时长与消息均值口径：无数据得 0.0，负数不计数', () => {
    expect(STATS_RS).toMatch(/if dur >= 0\.0/)
    expect(STATS_RS).toMatch(/duration_count > 0/)
    expect(activeMinutes('2026-09-01T10:00:00Z', '2026-09-01T10:30:00Z')).toBe(30)
    expect(activeMinutes('2026-09-01T10:30:00Z', '2026-09-01T10:00:00Z')).toBe(null)
    expect(activeMinutes('', 'x')).toBe(null)
  })
})

describe('后端不校验的枚举', () => {
  it('四个取值清单来自字段文档注释，不是前端自己挑的', () => {
    expect(docEnum(SESSION_MODEL_RS, 'session_type')).toEqual(SESSION_TYPES.map((t) => t.value))
    expect(docEnum(SESSION_MODEL_RS, 'status')).toEqual(SESSION_STATUSES.map((s) => s.value))
    expect(docEnum(MESSAGE_MODEL_RS, 'role')).toEqual(MESSAGE_ROLES.map((r) => r.value))
    expect(docEnum(MESSAGE_MODEL_RS, 'msg_type')).toEqual(MSG_TYPES.map((t) => t.value))
    for (const group of [SESSION_TYPES, SESSION_STATUSES, MESSAGE_ROLES, MSG_TYPES]) {
      for (const item of group) expect(item.label, `${item.value} 缺中文标签`).not.toBe(item.value)
    }
  })

  it('三个 default_* 函数体与 handler 的 unwrap_or_else 都指向前端那个缺省值', () => {
    expect(rustFn('fn default_session_type()', COMMON_RS)).toContain('"single"')
    expect(rustFn('fn default_session_status()', COMMON_RS)).toContain('"active"')
    expect(rustFn('fn default_msg_type()', COMMON_RS)).toContain('"text"')
    expect(CREATE_RS).toMatch(/session_type: body\.session_type\.unwrap_or_else\(\|\| "single"\.into\(\)\)/)
    expect(CREATE_RS).toMatch(/status: "active"\.into\(\)/)
    expect(SESSION_TYPE_DEFAULT).toBe('single')
    expect(SESSION_STATUS_DEFAULT).toBe('active')
    expect(MSG_TYPE_DEFAULT).toBe('text')
    expect(sessionTypeLabel('single')).toBe('单专家')
    expect(sessionTypeLabel('future')).toBe('future')
    expect(messageRoleLabel('expert')).toBe('专家答复')
    expect(msgTypeLabel('code')).toBe('代码')
  })

  it('status 不校验但只统计三档：写出第四个值会在统计里隐形', () => {
    const arms = [...STATS_RS.matchAll(/"(\w+)" => \w+ \+= 1/g)].map((m) => m[1])
    expect([...new Set(arms)].sort()).toEqual([...SESSION_STATUS_COUNTED].sort())
    expect(STATS_RS).toMatch(/_ => \{\}/)
    for (const fn of [CREATE_RS, UPDATE_RS, ARCHIVE_RS]) expect(fn, `${fn.slice(0, 20)} 里竟有状态校验`).not.toMatch(/status\.as_str\(\)/)
    expect(UPDATE_RS).toMatch(/session\.status = status/)
    expect(sessionStatusLabel('active')).toBe('进行中')
    expect(sessionStatusLabel('paused')).toContain('未统计')
    expect(sessionStatusLabel('')).toBe('未知状态')
    expect(sessionStatusProblem('closed')).toBe('')
    expect(sessionStatusProblem('paused')).toContain('隐形')
    expect(sessionStatusProblem('')).toBe('')
  })

  it('role 与 msg_type 同样直落库，越界只影响显示不影响写入', () => {
    expect(APPEND_RS).toMatch(/role: body\.role/)
    expect(APPEND_RS).toMatch(/msg_type: body\.msg_type\.unwrap_or_else/)
    expect(APPEND_RS).not.toMatch(/matches!\(/)
    expect(APPEND_RS).not.toMatch(/role\.is_empty/)
  })

  it('归档是强制写三个字段，不是软删', () => {
    expect(ARCHIVE_RS).toMatch(/session\.status = "archived"\.into\(\)/)
    expect(ARCHIVE_RS).toMatch(/session\.archived_at = Some\(now\.clone\(\)\)/)
    expect(ARCHIVE_STATUS).toBe('archived')
    expect(jsonFaces(ARCHIVE_RS)).toContain('archived_at')
  })
})

describe('字面相似检索：不是语义模型', () => {
  it('text_similarity 是字符 bigram Jaccard，空串得 0，单字相等才得 1', () => {
    expect(SIMILARITY_RS).toMatch(/\.windows\(2\)/)
    expect(SIMILARITY_RS).toMatch(/intersection \/ union/)
    expect(SIMILARITY_RS).toMatch(/if a_lower\.is_empty\(\) \|\| b_lower\.is_empty\(\)/)
    expect(SIMILARITY_RS).toMatch(/if a_lower == b_lower \{ 1\.0 \} else \{ 0\.0 \}/)
    expect(SIMILAR_RS).toMatch(/text_similarity\(&body\.query, &m\.content\)/)
    expect(SEMANTIC_RS).toMatch(/text_similarity\(&body\.query, &msg\.content\)/)
  })

  it('会话内检索按 score >= min_score 过滤，全域检索只要求 score > 0', () => {
    expect(SIMILAR_RS).toMatch(/\.filter\(\|\(_, score\)\| \*score >= min_score\)/)
    expect(SEMANTIC_RS).toMatch(/if score > 0\.0 \{/)
    expect(SEMANTIC_RS).not.toMatch(/min_score/)
    expect(SEMANTIC_SCORE_FLOOR).toBe(0)
    // 后端不回显过滤条件，口径文本只能由调用方把自己的条件交代进去
    expect(jsonFaces(SEMANTIC_RS)).toEqual(new Set(['query', 'results', 'total_sessions_scanned', 'total_messages_scanned', 'session_id', 'session_title', 'message', 'similarity_score']))
    const payload = normSemanticSearch({ total_sessions_scanned: 4, total_messages_scanned: 90, results: [{ session_id: 's1' }] })
    expect(semanticScopeText(payload, { expertId: 'e1' })).toContain('专家 e1')
    expect(semanticScopeText(payload, {})).toContain('含已归档会话')
    expect(semanticScopeText(payload, { sessionType: 'multi' })).toContain('多专家')
    expect(payload.results[0].sessionId).toBe('s1')
  })

  it('rank 从 1 起编号；total_found 是截断前命中数', () => {
    expect(SIMILAR_RS).toMatch(/"rank": idx \+ 1/)
    expect(SIMILAR_RS).toMatch(/let total_found = scored\.len\(\)/)
    expect(SIMILAR_RS).toMatch(/\.take\(top_k\)/)
    expect(similarTruncated({ totalFound: 9, results: [1, 2] })).toBe(true)
    expect(similarTruncated({ totalFound: 2, results: [1, 2] })).toBe(false)
    expect(similarTruncated({})).toBe(false)
    const s = normSimilarSearch({ results: [{ message: { id: 'm', role: 'user' }, similarity_score: 0.42, rank: 1 }], total_found: 7 })
    expect(s.results[0].similarityScore).toBe(0.42)
    expect(s.totalFound).toBe(7)
  })

  it('详情一次给全量 messages，所以截断显示是前端的事，且要说出来', () => {
    const detail = rustFn('async fn get_session(')
    expect(detail).toMatch(/ok\(json!\(session\)\)/)
    expect(detail).not.toMatch(/\.take\(|\.skip\(|\.truncate\(/)
    const msgs = Array.from({ length: 250 }, (_, i) => ({ id: `m${i}` }))
    const win = threadWindow(msgs, THREAD_RENDER_LIMIT)
    expect(win.rows.length).toBe(THREAD_RENDER_LIMIT)
    expect(win.hidden).toBe(50)
    expect(win.rows[0].id).toBe('m50')
    expect(threadWindow(msgs).rows.length).toBe(THREAD_RENDER_LIMIT)
    expect(threadWindow(null).hidden).toBe(0)
  })
})

describe('导出、删除与 404', () => {
  it('download_url 恒为 null：下载只能在前端做', () => {
    expect(EXPORT_RS).toMatch(/"download_url": null/)
    expect(jsonFaces(EXPORT_RS)).toContain('content')
    const e = normSessionExport({ session_id: 's1', format: 'json', content: { id: 's1', messages: [{ id: 'm' }] } })
    expect(e.downloadUrl).toBe(null)
    expect(e.content.messageCount).toBe(1)
    expect(normSessionExport({ download_url: 'x' }).downloadUrl).toBe('x')
    expect(exportText({ a: 1 })).toContain('"a": 1')
    expect(exportFileName('s1', '2026-09-01T10:00:00Z')).toBe('expert-session-s1-2026-09-01.json')
    expect(exportFileName('', '')).toBe('expert-session-unknown.json')
  })

  it('删除响应 { deleted, session_id }，不存在与已删除同形：都是 404 文案', () => {
    expect(DELETE_RS).toMatch(/"deleted": true/)
    const four04 = [...SESSION_RS.matchAll(/err\(404, format!\("session not found: \{id\}"\)\)/g)]
    expect(four04.length).toBeGreaterThanOrEqual(6)
    expect(SESSION_NOT_FOUND_HINT).toBe('session not found')
    expect(isNotFound({ msg: 'session not found: s1' })).toBe(true)
    expect(isNotFound({ message: 'boom' })).toBe(false)
    expect(isNotFound(null)).toBe(false)
    expect(normSessionDelete({ deleted: true, session_id: 's1' })).toEqual({ deleted: true, sessionId: 's1' })
  })

  it('写操作后立即 save_sessions，界面不能声称只改内存', () => {
    for (const fn of [CREATE_RS, UPDATE_RS, APPEND_RS, DELETE_RS, ARCHIVE_RS]) {
      expect(fn, '持久化调用不见了').toContain('save_sessions(')
    }
  })
})
