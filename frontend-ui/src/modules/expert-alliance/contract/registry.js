// 专家注册面契约：唯一权威是网关 experts_registry.rs 的 merge_expert_from_value(:35-173)
// 与 create_expert(:348) / update_expert(:385) / delete_expert(:408)。三条后端语义决定了本文件的形状：
// 1. POST 与 PUT 共用同一个 merge——白名单外的键被静默吃掉，发错字段不报错也不生效；
// 2. PUT 是合并式，数组与 capabilities 整值替换，所以没改动的数组一律不发；
// 3. DELETE 是软删（enabled=false），而 PUT 的 match 臂要求 enabled，
//    因此「先删后改」「删了再注册同一个 id」都走不通，删除必须按单向操作呈现。
//
// 表单值一律用 normExpert 的 camelCase 形状，wire 名只在本文件出现一次。
import {
  EXPERT_AVAILABILITY, EXPERT_TYPE, PRICING_MODEL, VERIFICATION_STATUS,
  availabilityLabel, expertTypeLabel, pricingLabel, verificationLabel
} from './enums.js'
import { expertDisplayName } from './graph.js'

/** u64 读数后被 `as u32` 截断（hourly_rate_cents / current_load / max_concurrent 皆如此） */
export const EXPERT_U32_MAX = 0xffffffff

/** capabilities 的 proficiency 在 Rust 侧是 u8，越界不是裁剪而是整条被 from_value 丢弃 */
export const EXPERT_PROFICIENCY_MAX = 255

const options = (enumObj, labelOf) => Object.values(enumObj).map((v) => ({ value: v, label: labelOf(v) }))

/**
 * 注册 / 编辑表单字段规格。
 * backendChecked=true 表示后端有对应的拒绝分支；false 表示照收不验，
 * 边界只是本表单的输入约束，界面不得写成"后端会拒绝"。
 */
export const EXPERT_REGISTER_FIELDS = Object.freeze([
  { key: 'name', label: '专家名称', wire: 'name', kind: 'text', required: true, backendChecked: true, hint: '空值后端 400「expert name is required」' },
  { key: 'title', label: '头衔', wire: 'title', kind: 'text', hint: '留空不影响：描述字段会顺带填它（见 EXPERT_ALIAS_FIELDS）' },
  { key: 'organization', label: '所属机构', wire: 'organization', kind: 'text' },
  { key: 'avatar', label: '头像地址', wire: 'avatar', kind: 'text' },
  { key: 'bio', label: '简介', wire: 'bio', kind: 'textarea' },
  { key: 'expertType', label: '专家类型', wire: 'expert_type', kind: 'select', options: options(EXPERT_TYPE, expertTypeLabel), default: 'ai' },
  { key: 'pricingModel', label: '计费模式', wire: 'pricing_model', kind: 'select', options: options(PRICING_MODEL, pricingLabel), default: 'free' },
  { key: 'hourlyRateCents', label: '时薪（分）', wire: 'hourly_rate_cents', kind: 'number', min: 0, max: EXPERT_U32_MAX, step: 100, default: 0 },
  { key: 'verificationStatus', label: '认证状态', wire: 'verification_status', kind: 'select', options: options(VERIFICATION_STATUS, verificationLabel), default: 'verified' },
  { key: 'timezone', label: '时区', wire: 'timezone', kind: 'text', default: 'Asia/Shanghai' },
  { key: 'languages', label: '语言', wire: 'languages', kind: 'tags', default: () => ['zh-CN', 'en'] },
  { key: 'domains', label: '领域', wire: 'domains', kind: 'tags', default: () => [] },
  { key: 'skills', label: '技能', wire: 'skills', kind: 'tags', default: () => [] },
  { key: 'tags', label: '标签', wire: 'tags', kind: 'tags', default: () => [] },
  { key: 'capabilities', label: '能力项', wire: 'capabilities', kind: 'capabilities', default: () => [] },
  // 在线状态在整个网关只有一处写入口（merge :128），没有心跳端点维护它，
  // 所以本表单是唯一能把专家置为 busy/offline 的地方——不挂载就等于永久冻结在注册默认值。
  { key: 'availabilityStatus', label: '在线状态', wire: 'availability.status', read: (e) => e?.availability?.status, kind: 'select', options: options(EXPERT_AVAILABILITY, availabilityLabel), default: 'online' },
  { key: 'maxConcurrent', label: '并发上限', wire: 'availability.max_concurrent', read: (e) => e?.availability?.maxConcurrent, kind: 'number', min: 0, max: EXPERT_U32_MAX, step: 1, default: 5 }
])

/**
 * 后端收得、但本模块故意不做录入入口的键。逐条给理由，契约测试断言它们仍是 merge 认识的键，
 * 后端删字段时这里会红，而不是让表单默默继续发一个没人读的键。
 */
export const EXPERT_UNMOUNTED_FIELDS = Object.freeze([
  { key: 'metrics', reason: '评分/咨询量/解决率是排行榜与派生指标的数据源（task #6 就是为撤掉虚构榜单而做的），开成输入框即可人工造榜' },
  { key: 'availability.current_load', reason: '运行态负载，只有调度器与 reset 会动它，录入框改它没有语义' },
  { key: 'availability.avg_response_minutes', reason: '响应时长应由真实会话统计得出，手填即假指标' },
  { key: 'availability.last_active', reason: '活跃时间戳，注册时由后端盖 now' },
  { key: 'metadata', reason: '增量 KV、无键约束，做成 JSON 文本框就是"能填但没人会用"的控件' },
  { key: 'id', source: 'create_expert', reason: '自定义 id 一旦被软删就永久占用（create 查的是含停用记录的整表），交给 exp-<uuid> 生成更安全' }
])

/** 兼容别名：merge 里另有三支会改写正名字段，本模块不使用它们，测试负责确认后端仍保留 */
export const EXPERT_ALIAS_FIELDS = Object.freeze([
  { key: 'type', writesTo: 'tags[0] + metadata.type', note: '去重后插入首位，和 tags 同时发时顺序由 merge 决定' },
  { key: 'description', writesTo: 'bio（title 为空时并填 title）', note: '晚于 bio 执行，同发则 bio 被覆盖' },
  { key: 'systemPrompt', writesTo: 'metadata.system_prompt', note: '空串不发' }
])

export function expertField(key) {
  return EXPERT_REGISTER_FIELDS.find((f) => f.key === key) || null
}

const listOf = (v) => (Array.isArray(v) ? v : [])
const cleanList = (v) => [...new Set(listOf(v).map((s) => String(s ?? '').trim()).filter(Boolean))]
const cleanCaps = (v) => listOf(v)
  .map((c) => ({
    id: String(c?.id ?? '').trim(),
    name: String(c?.name ?? '').trim(),
    domain: String(c?.domain ?? '').trim(),
    proficiency: Number(c?.proficiency),
    description: String(c?.description ?? '').trim()
  }))
  .filter((c) => c.name || c.domain || Number.isFinite(c.proficiency) || c.description)

/** 归一化后的专家 → 表单草稿（编辑与注册共用一份形状） */
export function expertFormDraft(expert = {}) {
  const draft = {}
  for (const f of EXPERT_REGISTER_FIELDS) {
    const raw = f.read ? f.read(expert) : expert?.[f.key]
    // undefined/null 才回默认值：后端给的空数组是「真的没有」，不能显示成默认的 zh-CN/en
    const seed = raw === undefined || raw === null
      ? (typeof f.default === 'function' ? f.default() : (f.default ?? ''))
      : raw
    if (f.kind === 'tags') draft[f.key] = cleanList(seed)
    else if (f.kind === 'capabilities') draft[f.key] = cleanCaps(seed)
    else draft[f.key] = seed
  }
  return draft
}

/** 注册默认草稿：与 ExpertDescriptor::minimal 的默认值同源，不额外发明"合理初值" */
export function emptyExpertDraft() {
  return expertFormDraft({})
}

// minimal 的默认值抄在上面这份草稿里，所以注册时"等于默认值"就等于"不发这条"，
// 让后端成为唯一的默认值持有者——两处各写一份默认值，早晚会对不上。
const MINIMAL_DEFAULTS = emptyExpertDraft()

/** 单字段校验：返回中文原因，空串表示可发 */
export function expertFieldProblem(field, value) {
  const f = typeof field === 'string' ? expertField(field) : field
  if (!f) return `未知字段 ${field}`
  if (f.kind === 'text' || f.kind === 'textarea') {
    if (f.required && !String(value ?? '').trim()) return `${f.label}不能为空`
    return ''
  }
  if (f.kind === 'select') {
    if (!String(value ?? '')) return ''
    return f.options.some((o) => o.value === value) ? '' : `${f.label}必须是 ${f.options.map((o) => o.value).join(' / ')}`
  }
  if (f.kind === 'tags') {
    return listOf(value).some((s) => typeof s !== 'string') ? `${f.label}只能是文本项` : ''
  }
  if (f.kind === 'capabilities') {
    for (const c of cleanCaps(value)) {
      if (!c.name) return '能力项必须有名称'
      if (!Number.isInteger(c.proficiency) || c.proficiency < 0 || c.proficiency > EXPERT_PROFICIENCY_MAX) {
        return `能力「${c.name}」的熟练度需为 0–${EXPERT_PROFICIENCY_MAX} 的整数：proficiency 在 Rust 侧是 u8，越界整条被丢弃且不会报错`
      }
    }
    return ''
  }
  const n = Number(value)
  if (!Number.isInteger(n)) return `${f.label}必须是整数：后端按 as_u64 取，小数会被静默丢掉`
  if (n < f.min || n > f.max) return `${f.label}需在 ${f.min}–${f.max} 之间：超过 2³²−1 会被 as u32 截断成另一个数`
  return ''
}

export function expertProblems(draft = {}) {
  const out = []
  for (const f of EXPERT_REGISTER_FIELDS) {
    const problem = expertFieldProblem(f, draft[f.key])
    if (problem) out.push(problem)
  }
  return out
}

export function expertDraftProblem(draft = {}) {
  return expertProblems(draft)[0] || ''
}

const setDeep = (body, wire, value) => {
  const parts = wire.split('.')
  let cur = body
  for (const p of parts.slice(0, -1)) cur = cur[p] ??= {}
  cur[parts.at(-1)] = value
}

/** 单个字段 → wire 值（数组去重去空、能力项补 id） */
function wireValue(f, value) {
  if (f.kind === 'tags') return cleanList(value)
  if (f.kind === 'capabilities') {
    // 字符串简写会让后端把 proficiency 硬编码成 85、domain 落到 tags[0]（merge :105-123），
    // 所以本模块走对象形，熟练度由用户填、空 domain 就留空。
    return cleanCaps(value).map((c) => ({
      // 编辑时后端给的 id 要原样带回：能力目录是按 capability id 聚合的，重新生成会把老项当成新项
      id: c.id || `cap-${c.name}`,
      name: c.name,
      domain: c.domain,
      proficiency: c.proficiency,
      description: c.description
    }))
  }
  if (f.kind === 'number') return Number(value)
  if (f.kind === 'text' || f.kind === 'textarea' || f.kind === 'select') return String(value ?? '').trim()
  return value
}

const isCleanEmpty = (f, value) => {
  const v = wireValue(f, value)
  if (f.kind === 'tags' || f.kind === 'capabilities') return v.length === 0
  if (f.kind === 'number') return false
  return v === ''
}

/**
 * 表单草稿 → POST 请求体。空值与「等于 minimal 默认值」的项都不发，
 * 缺的字段由后端 ExpertDescriptor::minimal 决定，前端不自造第二套「合理初值」。
 * 校验不过的草稿由 expertDraftProblem 拦下，不发给后端换 400。
 */
export function registerBody(draft = {}) {
  const body = {}
  for (const f of EXPERT_REGISTER_FIELDS) {
    const value = draft[f.key]
    if (isCleanEmpty(f, value)) continue
    if (sameWire(f, value, MINIMAL_DEFAULTS[f.key])) continue
    setDeep(body, f.wire, wireValue(f, draft[f.key]))
  }
  return body
}

/**
 * 编辑前后 → PUT 请求体：只发改动过的键（合并式更新）。
 * 数组改动发的是**整值**，因为后端是整体替换而不是逐元素合并；
 * 未改动的数组不发，否则会把后端里自己没读到的值清空。
 */
export function expertPatch(before = {}, after = {}) {
  const patch = {}
  const problems = []
  const a = { ...emptyExpertDraft(), ...before }
  const b = { ...emptyExpertDraft(), ...after }
  for (const f of EXPERT_REGISTER_FIELDS) {
    if (!(f.key in b)) continue
    const problem = expertFieldProblem(f, b[f.key])
    if (problem) {
      problems.push(problem)
      continue
    }
    if (sameWire(f, a[f.key], b[f.key])) continue
    setDeep(patch, f.wire, wireValue(f, b[f.key]))
  }
  return { patch, problem: problems[0] || '' }
}

function sameWire(f, x, y) {
  return JSON.stringify(wireValue(f, x)) === JSON.stringify(wireValue(f, y))
}

/**
 * 删除前的后果清单。每条都对应网关一处可读的代码事实，界面不得简化成"确定删除？"。
 * @param {{id?:string,name?:string}} expert
 */
export function deleteConsequences(expert = {}) {
  const who = expertDisplayName(expert)
  return [
    `${who} 会从列表、详情、派生指标、能力目录里消失——这四处都在过滤 enabled（experts_registry.rs:177,258,326,565）`,
    '调度器统计、协作选路、编排与最优团队也只看 enabled 专家（experts_dispatcher.rs:518、experts_collaboration.rs:767,997,1083,1374、experts_orchestration.rs:688、experts_graph.rs:475-477）',
    '记录仍在注册表里占着这个 id：再注册同一个 id 返回 400「expert id already exists」（experts_registry.rs:362-365 查的是含停用记录的整表）',
    '网关没有任何把 enabled 写回 true 的端点，PUT 又要求 enabled（experts_registry.rs:392），所以停用后既改不回来也删不回来',
    '删除会落盘（experts_registry.rs:418 → experts_db.rs:197 的 enabled 列），重启进程也回不来'
  ]
}

/** 软删响应 → 面向用户的说明（后端回的是英文 message，界面不该原样贴给用户） */
export function deleteResultText(result) {
  if (!result?.deleted) return '后端未确认删除'
  return result.softDelete
    ? '已停用（软删）：该专家对所有读接口不可见，且无法再启用'
    : '后端回了 deleted，但未标记 soft_delete——口径与契约不符，请勿当作硬删除展示'
}

/**
 * 谁能调用这三条写端点——两条网关事实，界面措辞不得越过它们：
 * 1. 业务路由统一挂 auth_middleware，而 AuthConfig::default() 的 enabled 为 true
 *    （config.rs:37、modules.rs:186）⇒ 写请求必须带身份，匿名请求在中间件层就 401；
 *    外壳的路由守卫已经保证了进得来这一页就一定有身份（router/index.js:74），所以界面不再重复置灰。
 * 2. 专家注册/编辑/停用 handler 没有角色判定；可选身份用于租户及审计：
 *    任何持令牌的调用方都能注册 / 编辑 / 停用，且停用不可逆。
 *    所以文案只能说"没有角色判定"，不能说"仅管理员可操作"——后端并没有这件事。
 */
export const EXPERT_WRITE_IDENTITY = Object.freeze({
  statement: '这三条写请求都带当前登录身份发出；网关对专家写路径没有角色判定，任何已认证身份都能注册 / 编辑 / 停用，身份只被写进审计链的行动者字段（没带身份则记为 system）。全域唯一一处 403 在协作面：目标专家已被禁用时拒绝，拒的是对象状态而不是调用方是谁',
  evidence: ['config.rs:41', 'modules.rs:231', 'router/index.js:74', 'experts_common.rs:803', 'experts_dispatcher.rs:596', 'experts_collaboration.rs:789']
})

/** 会诊房间凭证状态的取值常量。wire 权威 experts_registry.rs:672-676
 *  只产出 available / waiting，且由 expert 的 availability.status == "online" 推出（没有第三态）。 */
export const ROOM_STATUS = Object.freeze({ AVAILABLE: 'available', WAITING: 'waiting' })
