// 专家会话契约：唯一权威是网关 alliance/experts_session.rs 的 11 个 handler
// 与 experts_common.rs 的 ExpertSession(:242) / SessionMessage(:212) / parse_pagination(:875)。
//
// 三条决定本文件每一行的后端语义：
// 1. 请求体全部走 serde 默认反序列化——字段名写错不报错，只被静默丢弃；
//    所以 body 只能由这里生成，视图与 store 不得手拼字段名。
// 2. 会话与消息的 status/role/msg_type **都不在后端校验**（create_session:154、
//    update_session:361、append_message:422 一律 unwrap_or_default / 直接落库），
//    这里的枚举只用于选项与提示，不得声称"后端会拒绝越界值"。
// 3. 相似检索是字符 bigram Jaccard（text_similarity:827），不是语义模型。
//    界面必须写"字面相似"，不得借用 AI 语义的话头。

const num = (v) => (Number.isFinite(Number(v)) ? Number(v) : NaN)
const str = (v) => (v === undefined || v === null ? '' : String(v))
const list = (v) => (Array.isArray(v) ? v.map(str).filter(Boolean) : [])
const unset = (v) => v === undefined || v === null || v === ''

// ── 枚举：取值清单来自字段文档注释，缺省值来自 default_* 函数体 ─────────

/** ExpertSession.session_type（experts_common.rs:249 文档 + :282 default_session_type） */
export const SESSION_TYPE_DEFAULT = 'single'
export const SESSION_TYPES = Object.freeze([
  { value: 'single', label: '单专家' },
  { value: 'multi', label: '多专家' },
  { value: 'debate', label: '辩论' },
  { value: 'enterprise', label: '企业级' }
])

/** ExpertSession.status（:251 文档 + :283 default_session_status）；后端不校验，任意字符串可写入 */
export const SESSION_STATUS_DEFAULT = 'active'
export const SESSION_STATUSES = Object.freeze([
  { value: 'active', label: '进行中' },
  { value: 'archived', label: '已归档' },
  { value: 'closed', label: '已关闭' }
])

/** session_stats 的 status 计数分支只认这三个值（experts_session.rs:281-287 的 match _ => {}），
 *  写入其它值不会报错，但三档统计里一处都不计——这是"后端不校验"的真实代价，界面要能看出来。 */
export const SESSION_STATUS_COUNTED = Object.freeze(['active', 'archived', 'closed'])

/** SessionMessage.role（experts_common.rs:216 文档）；后端不校验 */
export const MESSAGE_ROLES = Object.freeze([
  { value: 'user', label: '提问用户' },
  { value: 'expert', label: '专家答复' },
  { value: 'system', label: '系统' },
  { value: 'assistant', label: '助手' }
])
// 键常量由上表派生，不另立第二份事实源（§5.44 同一规矩：视图比较用常量，字面量只出现在这一处）
export const MESSAGE_ROLE = Object.freeze(
  Object.fromEntries(MESSAGE_ROLES.map((r) => [r.value.toUpperCase(), r.value]))
)

/** SessionMessage.msg_type（:228 文档 + :238 default_msg_type）；后端不校验 */
export const MSG_TYPE_DEFAULT = 'text'
export const MSG_TYPES = Object.freeze([
  { value: 'text', label: '纯文本' },
  { value: 'markdown', label: 'Markdown' },
  { value: 'code', label: '代码' },
  { value: 'image', label: '图片' },
  { value: 'file', label: '文件' }
])

/**
 * rating 的语义区间与硬边界：字段文档写 0–5（:234），但类型是 Option<u8>——
 * 只有 >255 或非整数才被 serde 拒成 422，21 分照样入库。
 * 另外后端没有"改消息"端点，rating 只能在追加那一刻写入，之后不可回改。
 */
export const MESSAGE_RATING = Object.freeze({ min: 0, max: 5, wireMax: 255 })

export function sessionTypeLabel(value) {
  return SESSION_TYPES.find((t) => t.value === value)?.label || str(value) || '未分类'
}

export function sessionStatusLabel(value) {
  const hit = SESSION_STATUSES.find((s) => s.value === value)
  if (hit) return hit.label
  return str(value) ? `${str(value)}（后端未统计此状态）` : '未知状态'
}

/**
 * 会话"最近活跃"取哪个键，只有一个口径：后端创建时把 created_at 与 last_active_at 写成同一时刻
 * （experts_session.rs:172-173），之后只推进 last_active_at，所以活跃优先、创建兜底。
 * 界面侧曾读 `session.updated_at` —— 列表与详情的序列化里根本没有这个键
 * （session_to_list_view:113-129 / ExpertSession:242），于是"活跃度"一直显示的是创建时间。
 */
export function sessionActivityAt(session) {
  return str(session?.lastActiveAt) || str(session?.createdAt)
}

/**
 * 协作模式 → 落库的 session_type。工作台新建草稿行时只有协作模式
 * （route/single/multi/debate/smart/algorithm，见 contract/collab.js）与已选专家数，
 * 而后端字段词表是 single/multi/debate/enterprise，两套词表不相交：
 * 翻译只在这里做一次，视图不得再自建 mode→类型 私表。
 * 'debate' 两边同名是巧合而非同一件事，故按字面量取值，不从 collab.js 引用来避免契约内环。
 * enterprise 不由本地草稿产出：只有网关编排路径写它（experts_collaboration.rs:1470）。
 */
export function sessionTypeForCollabMode(mode, expertCount = 0) {
  if (str(mode) === 'debate') return 'debate'
  const n = num(expertCount)
  return Number.isFinite(n) && n > 1 ? 'multi' : 'single'
}

/**
 * session_type → el-tag 配色。分支集合必须与 SESSION_TYPES 逐项对齐：
 * 多一个键是界面上永不可达的死档，少一个键会让新类型静默掉进兜底。
 * 越界值给 danger 而不是 info——后端不校验 session_type（create_session:154 直接落库），
 * 写进去的野值必须在界面上看得见，不许用中性色掩盖。
 */
export function sessionTypeTagType(value) {
  const type = str(value)
  if (type === 'single') return 'primary'
  if (type === 'multi') return 'success'
  if (type === 'debate') return 'warning'
  if (type === 'enterprise') return 'info'
  return 'danger'
}

/**
 * 会话标题的显示口径。create_session 不校验 title（空 body 也合法），
 * 所以"标题为空"是后端可达状态，界面不许渲染成一行空白当标题。
 */
export function sessionListTitle(session) {
  const title = str(session?.title)
  return title || `（无标题）${str(session?.id)}`
}

export function messageRoleLabel(value) {
  return MESSAGE_ROLES.find((r) => r.value === value)?.label || str(value)
}

export function msgTypeLabel(value) {
  return MSG_TYPES.find((t) => t.value === value)?.label || str(value)
}

// ── 列表查询（GET /api/experts/sessions，list_sessions:191）──────────────

/**
 * parse_pagination（experts_common.rs:875-882）逐字对应：
 * page 缺省 1 且 `.max(1)`；page_size 取 `page_size` **或** `limit`，缺省 20，`clamp(1, 200)`。
 * limit 只是别名，本模块一律发 page_size，避免两套键互相覆盖。
 */
export const SESSION_PAGE = Object.freeze({ defaultPage: 1, defaultSize: 20, minSize: 1, maxSize: 200 })
export const SESSION_PAGE_SIZES = Object.freeze([20, 50, 100, 200])

/** 本地字段 → wire 名，逐一对应 list_sessions:196-200 的 params.get(...) */
export const SESSION_QUERY_KEYS = Object.freeze({
  status: 'status',
  sessionType: 'session_type',
  expertId: 'expert_id',
  userId: 'user_id',
  search: 'search',
  page: 'page',
  pageSize: 'page_size'
})

/**
 * search 命中范围是 title **或** topic 的小写包含（:216-222），不含消息正文；
 * 排序固定 created_at 降序（:237），后端没有 sort 参数，界面不得摆出"排序"控件。
 * 空串一律不发：`?status=` 会被后端当有效过滤（Some("") 比较），把结果清成空表。
 */
export function sessionListQuery(filters = {}) {
  const query = {}
  for (const [local, wire] of Object.entries(SESSION_QUERY_KEYS)) {
    const raw = filters[local]
    if (unset(raw) || str(raw).trim() === '') continue
    if (local === 'page' || local === 'pageSize') {
      const n = num(raw)
      if (!Number.isFinite(n)) continue
      query[wire] = local === 'page' ? Math.max(1, Math.trunc(n)) : Math.trunc(clampSize(n))
      continue
    }
    query[wire] = str(raw).trim()
  }
  return query
}

function clampSize(n) {
  return Math.min(Math.max(Math.trunc(n), SESSION_PAGE.minSize), SESSION_PAGE.maxSize)
}

/** 后端 total 是过滤后全量；页码越界不报错，只返回空 items（skip 超出即空） */
export function sessionPageCount(total, pageSize) {
  const size = clampSize(num(pageSize) || SESSION_PAGE.defaultSize)
  return Math.max(1, Math.ceil((Number(total) || 0) / size))
}

/**
 * 分页回传的 page 由后端从 offset 反算（:245），所以前端不必纠正它，
 * 但当 total=0 时界面仍应停留在第 1 页。
 */
export function normalizeFilters(filters = {}) {
  return {
    status: str(filters.status),
    sessionType: str(filters.sessionType),
    expertId: str(filters.expertId),
    userId: str(filters.userId),
    search: str(filters.search).trim(),
    page: Math.max(1, Math.trunc(num(filters.page) || SESSION_PAGE.defaultPage)),
    pageSize: Math.trunc(clampSize(num(filters.pageSize) || SESSION_PAGE.defaultSize))
  }
}

// ── 创建（POST /api/experts/sessions，create_session:154）────────────────

/** CreateSessionBody 字段面（:36-52）：七个字段全 Option，空 body 也是合法请求 */
export const CREATE_SESSION_FIELDS = Object.freeze(['title', 'expert_ids', 'user_id', 'session_type', 'topic', 'tags', 'metadata'])

/** 未填 title 时给一个可辨认的标题，否则入库的是空串，列表里只剩一串 UUID */
export function sessionDraftTitle(draft = {}) {
  const title = str(draft.title).trim()
  if (title) return title
  const topic = str(draft.topic).trim()
  if (topic) return topic.length > 24 ? `${topic.slice(0, 24)}…` : topic
  return '未命名会话'
}

export function createSessionBody(draft = {}) {
  const body = {}
  const title = str(draft.title).trim()
  if (title) body.title = title
  const experts = list(draft.expertIds)
  if (experts.length) body.expert_ids = experts
  const userId = str(draft.userId).trim()
  if (userId) body.user_id = userId
  const type = str(draft.sessionType).trim()
  if (type && type !== SESSION_TYPE_DEFAULT) body.session_type = type
  const topic = str(draft.topic).trim()
  if (topic) body.topic = topic
  const tags = list(draft.tags)
  if (tags.length) body.tags = tags
  const metadata = plainObject(draft.metadata)
  if (metadata) body.metadata = metadata
  return body
}

/** 空对象与缺省在后端等价（:171 unwrap_or_default），没必要发一个 {} 占位 */
function plainObject(v) {
  if (!v || typeof v !== 'object' || Array.isArray(v)) return null
  return Object.keys(v).length ? v : null
}

/** 创建表单的前端自检：只有类型是硬约束（下拉越界即后端收一个统计不到的值） */
export function createSessionProblem(draft = {}) {
  const type = str(draft.sessionType).trim()
  if (type && !SESSION_TYPES.some((t) => t.value === type)) return `会话类型只能是 ${SESSION_TYPES.map((t) => t.value).join(' / ')} 之一`
  return ''
}

// ── 更新（PUT /api/experts/sessions/:id，update_session:361）─────────────

/** UpdateSessionBody 字段面（:57-66）：注意**没有** expert_ids / user_id / session_type，
 *  发给它们的唯一后果是被 serde 丢弃——想换专家阵容只能新建会话。 */
export const UPDATE_SESSION_FIELDS = Object.freeze(['title', 'status', 'topic', 'tags', 'metadata'])

/** 会话的 metadata 有两种真实形状：wire 上是 Map，归一化后是 [{key,value}] 行。
 *  差分只能按键比，所以先统一成对象——否则会把手工行当成不存在的键，把"删不掉"报成一行索引。 */
function metaAsObject(md) {
  if (Array.isArray(md)) {
    const out = {}
    for (const row of md) {
      const key = str(row?.key).trim()
      if (key) out[key] = row?.value
    }
    return out
  }
  return md && typeof md === 'object' ? md : {}
}

const isPrimitive = (v) => v === null || typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean'

/**
 * metadata 值的"算不算改过"。表单只能收集字符串，库里却可能是数字/布尔：
 * 3 与 '3' 在文本框里长得一模一样，用户没改过它，就不该被回写成字符串。
 */
function sameMetaValue(before, after) {
  if (JSON.stringify(before) === JSON.stringify(after)) return true
  return isPrimitive(before) && typeof after === 'string' && String(before) === after
}

/**
 * 合并式更新的请求体：只发与当前值不同的键。
 * 与 dispatchPatch 同理——发一个与后端同值的键没有坏处，但"我改了什么"就查不出来了，
 * 而且 metadata 是**逐键合并、无法删除**（:390-394 只有 insert），
 * 所以 metadata 只提交新增/改值的键，并如实报告删不掉的键。
 */
export function sessionUpdatePatch(current = {}, draft = {}) {
  const patch = {}
  const dropped = []
  const title = str(draft.title).trim()
  if (title && title !== str(current.title)) patch.title = title
  const status = str(draft.status).trim()
  if (status && status !== str(current.status)) patch.status = status
  const topic = str(draft.topic).trim()
  if (topic && topic !== str(current.topic)) patch.topic = topic
  // 只有 draft 真带了 tags 数组才提交：后端是覆写语义，调用方漏传一个键就清掉标签不可接受；
  // 要清空标签须显式发 []。
  const tags = list(draft.tags)
  if (Array.isArray(draft.tags) && JSON.stringify(tags) !== JSON.stringify(list(current.tags))) patch.tags = tags

  const before = metaAsObject(current.metadata)
  const after = metaAsObject(draft.metadata)
  const merged = {}
  for (const [k, v] of Object.entries(after)) {
    if (k in before && sameMetaValue(before[k], v)) dropped.push(k)
    else merged[k] = v
  }
  const removed = Object.keys(before).filter((k) => !(k in after))
  if (Object.keys(merged).length) patch.metadata = merged
  return { patch, unchangedMetadataKeys: dropped, unremovableMetadataKeys: removed }
}

/**
 * metadata 表单行 → 后端要的 Map<String, Value>。
 * 值按字符串上送：后端类型是 Value（可容纳数字/布尔/嵌套对象），但本表单收集的是文本，
 * 在这里偷偷 JSON.parse 会让"填 1"变成数字 1、"填 true"变成布尔，界面与库里的值就对不上了。
 */
export function metadataRowsToObject(rows = []) {
  const out = {}
  for (const row of Array.isArray(rows) ? rows : []) {
    const key = str(row?.key).trim()
    if (!key) continue
    out[key] = str(row?.value)
  }
  return out
}

/** status 越界要提醒：后端照写，但一旦写出 active/archived/closed 之外的值就再也进不了统计 */
export function sessionStatusProblem(status) {
  const v = str(status).trim()
  if (!v) return ''
  if (SESSION_STATUS_COUNTED.includes(v)) return ''
  return `状态 ${v} 后端会照收，但 session_stats 的三档计数不含它，该会话在统计里隐形`
}

// ── 追加消息（POST /api/experts/sessions/:id/messages，append_message:422）

/**
 * AppendMessageBody（:71-85）：role 与 content **无 serde(default)**，缺任一个 axum 直接 422，
 * 所以这里必填校验是"少发一次必败请求"，不是替后端把关。
 * 响应只有那条消息本身（:448 ok(json!(message))），没有会话，故 store 要自己把它并进线程。
 */
export function appendMessageProblem(draft = {}) {
  if (!str(draft.role).trim()) return '必须选择发送角色（后端 role 无缺省，缺失即 422）'
  if (!str(draft.content).trim()) return '消息内容不能为空（后端 content 无缺省，缺失即 422）'
  const rating = draft.rating
  if (!unset(rating)) {
    const n = num(rating)
    if (!Number.isFinite(n) || n < MESSAGE_RATING.min || n > MESSAGE_RATING.max) {
      return `评分须在 ${MESSAGE_RATING.min}–${MESSAGE_RATING.max} 之间（字段类型 u8，超过 255 才会被后端拒成 422）`
    }
  }
  return ''
}

export function appendMessageBody(draft = {}) {
  const body = { role: str(draft.role).trim(), content: str(draft.content) }
  const senderId = str(draft.senderId).trim()
  if (senderId) body.sender_id = senderId
  // sender_name 后端不查注册表，只 unwrap_or_default（:431），界面显示谁就得上送谁
  const senderName = str(draft.senderName).trim()
  if (senderName) body.sender_name = senderName
  const msgType = str(draft.msgType).trim()
  if (msgType && msgType !== MSG_TYPE_DEFAULT) body.msg_type = msgType
  if (Array.isArray(draft.attachments) && draft.attachments.length) body.attachments = draft.attachments
  if (!unset(draft.rating)) {
    const n = num(draft.rating)
    if (Number.isFinite(n)) body.rating = Math.trunc(n)
  }
  return body
}

/** 消息条数上限由后端分页决定？不是——详情接口一次性返回全部 messages（get_session:347），
 *  长会话只能在前端截断显示，故把"截断"这件事写成一处，别在视图里散落 magic 数。 */
export const THREAD_RENDER_LIMIT = 200

export function threadWindow(messages = [], limit = THREAD_RENDER_LIMIT) {
  const all = Array.isArray(messages) ? messages : []
  if (all.length <= limit) return { rows: all, hidden: 0 }
  return { rows: all.slice(all.length - limit), hidden: all.length - limit }
}

// ── 会话内字面相似检索（POST /api/experts/sessions/:id/similar-search，:460）

/** top_k.unwrap_or(5) / min_score.unwrap_or(0.1)，过滤条件是 score >= min_score（:478） */
export const SIMILAR_SEARCH_DEFAULTS = Object.freeze({ topK: 5, minScore: 0.1 })

export function similarSearchProblem(draft = {}) {
  if (!str(draft.query).trim()) return '检索词不能为空（后端 query 无缺省，缺失即 422）'
  const topK = draft.topK
  if (!unset(topK) && (!Number.isFinite(num(topK)) || num(topK) < 1)) return '返回条数至少 1'
  const minScore = draft.minScore
  if (!unset(minScore) && (num(minScore) < 0 || num(minScore) > 1)) return '相似度阈值须在 0–1 之间'
  return ''
}

/**
 * 后端对 top_k 没有上限（usize 照收），界面给一个输入约束就够了，别声称后端会拦。
 * min_score 等于缺省时不发，与 graph body 同一口径：不让前端替后端记住默认值。
 */
export function similarSearchBody(draft = {}) {
  const body = { query: str(draft.query).trim() }
  const topK = num(draft.topK)
  if (Number.isFinite(topK) && topK >= 1 && Math.trunc(topK) !== SIMILAR_SEARCH_DEFAULTS.topK) {
    body.top_k = Math.trunc(topK)
  }
  const minScore = num(draft.minScore)
  if (Number.isFinite(minScore) && minScore >= 0 && minScore !== SIMILAR_SEARCH_DEFAULTS.minScore) {
    body.min_score = minScore
  }
  return body
}

/** total_found 是截断前命中数，与 results.length 不等时说明还有没展示的 */
export function similarTruncated(payload = {}) {
  return (Number(payload.totalFound) || 0) > (Array.isArray(payload.results) ? payload.results.length : 0)
}

// ── 全域语义检索（POST /api/experts/semantic-search，semantic_search:511）

/** top_k.unwrap_or(10)（:515）；**没有** min_score 字段，发出去只会被 serde 丢弃 */
export const SEMANTIC_SEARCH_DEFAULTS = Object.freeze({ topK: 10 })
/** 后端唯一的收录门槛是 score > 0.0（:536），不是可配阈值 */
export const SEMANTIC_SCORE_FLOOR = 0

export function semanticSearchProblem(draft = {}) {
  if (!str(draft.query).trim()) return '检索词不能为空（后端 query 无缺省，缺失即 422）'
  const type = str(draft.sessionType).trim()
  if (type && !SESSION_TYPES.some((t) => t.value === type)) return `会话类型只能是 ${SESSION_TYPES.map((t) => t.value).join(' / ')} 之一`
  return ''
}

export function semanticSearchBody(draft = {}) {
  const body = { query: str(draft.query).trim() }
  const topK = num(draft.topK)
  if (Number.isFinite(topK) && topK >= 1 && Math.trunc(topK) !== SEMANTIC_SEARCH_DEFAULTS.topK) body.top_k = Math.trunc(topK)
  const type = str(draft.sessionType).trim()
  if (type) body.session_type = type
  const expertId = str(draft.expertId).trim()
  if (expertId) body.expert_id = expertId
  return body
}

/**
 * 全域检索没有 status 过滤：归档会话照样命中（:522-532 只过滤 session_type / expert_id）。
 * 界面必须把它当事实说出来，而不是让用户以为搜的是"进行中的会话"。
 * @param payload 后端响应（只有 query / results / total_*_scanned 四类键）
 * @param filter  本次请求自己的过滤条件——后端不回显它们，只能由调用方交代，不得伪造成响应字段
 */
export function semanticScopeText(payload = {}, filter = {}) {
  const s = Number(payload.totalSessionsScanned) || 0
  const m = Number(payload.totalMessagesScanned) || 0
  const hits = Array.isArray(payload.results) ? payload.results.length : 0
  const filters = []
  if (filter.sessionType) filters.push(`类型 ${sessionTypeLabel(filter.sessionType)}`)
  if (filter.expertId) filters.push(`专家 ${filter.expertId}`)
  return `扫描 ${s} 个会话 / ${m} 条消息${filters.length ? `（过滤：${filters.join('、')}）` : '（含已归档会话）'}，命中 ${hits} 条`
}

// ── 导出与归档（export_session:573 / archive_session:599）

/**
 * 导出响应里 download_url **恒为 null**（:586 写死），所以"下载"只能在前端完成：
 * content 是完整 ExpertSession（含 messages），把它序列化成文本再交给浏览器。
 */
export function exportText(content) {
  try {
    return JSON.stringify(content ?? null, null, 2)
  } catch {
    return ''
  }
}

export function exportFileName(sessionId, exportedAt) {
  const stamp = str(exportedAt).slice(0, 10)
  return `expert-session-${str(sessionId) || 'unknown'}${stamp ? `-${stamp}` : ''}.json`
}

/** 归档是"强制写 status=archived + archived_at"（:605-607），不是软删；恢复没有专门端点，只能 PUT status */
export const ARCHIVE_STATUS = 'archived'

/** 删除响应 { deleted, session_id }（:413），404 与"已删"同形，界面按成功走 */
export const SESSION_NOT_FOUND_HINT = 'session not found'

export function isNotFound(error) {
  return str(error?.msg || error?.message).includes(SESSION_NOT_FOUND_HINT)
}

/** 时间线：created_at 与 last_active_at 差值，供"活跃时长"类展示复用，
 *  与后端 duration_minutes（experts_session.rs:141）同口径：负值不计。 */
export function activeMinutes(created, lastActive) {
  const a = Date.parse(str(created))
  const b = Date.parse(str(lastActive))
  if (!Number.isFinite(a) || !Number.isFinite(b)) return null
  const minutes = (b - a) / 60000
  return minutes >= 0 ? minutes : null
}

/** 会话状态取值常量。wire 权威 platform/gateway/mox-platform-gateway-svc/src/alliance/
 *   experts_common.rs:260（doc 注释 active/archived/closed）与 :286（default_session_status = "active"）
 *   experts_session.rs:284/285（统计只认 archived/closed）与 :612（归档强写 archived）
 * 视图比较状态时不许把这三个取值重新打字：判据 L12 的禁串集合就从这里现推。 */
export const SESSION_STATUS = Object.freeze({ ACTIVE: 'active', ARCHIVED: 'archived', CLOSED: 'closed' })
