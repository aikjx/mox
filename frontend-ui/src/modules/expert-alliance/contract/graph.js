// 专家协作图谱契约：唯一权威是网关 experts_graph.rs 的 8 个 handler，
// 以及 experts_common.rs 的 build_graph_from_registry（决定图里到底会出现什么节点/边）。
// 类型枚举、请求体字段、默认值与钳位一律取自 Rust，前端不得凭想象扩展（契约测试逐条核对）。
//
// 两条必须让界面说出来的后端语义：
// 1. 图是从专家注册表**派生**出来的内存态（可 POST rebuild 重建），不是独立维护的数据；
// 2. optimal-team 的 max_members 只有"0 视为缺省 5"这一条兜底，**没有上限**，
//    min_rating 也完全不校验区间——超过 5 只会筛出空团队，不会报错。

const num = (v) => (Number.isFinite(Number(v)) ? Number(v) : NaN)
const str = (v) => (v === undefined || v === null ? '' : String(v))
const list = (v) => (Array.isArray(v) ? v.map(str).filter(Boolean) : [])

/** 图中真实存在的节点类型：builder 只产出 expert 与 domain 两类 */
export const GRAPH_NODE_TYPE = Object.freeze({
  expert: 'expert',
  domain: 'domain'
})

export const GRAPH_NODE_TYPE_META = Object.freeze({
  expert: { label: '专家', abbr: '专' },
  domain: { label: '能力域', abbr: '域' }
})

// N4 写入端（POST/PUT /api/expert-graph/nodes）实际接受的 node_type，逐字取自
// experts_graph.rs 的 VALID_NODE_TYPES = ["expert","domain","capability"]。
// 注意与 GRAPH_NODE_TYPE 的分工：后者是「builder 派生图里真实出现的类型」，被 graph.test.js
// 与 Rust 源码双向钉死（builder 只产出 expert/domain）；capability 是手动 CRUD 才会出现的节点，
// 派生图经 rebuild 后不会自动带它。因此这里只作写入表单选项与画布兜底色，绝不并入 GRAPH_NODE_TYPE。
export const GRAPH_WRITE_NODE_TYPES = Object.freeze([
  { value: 'expert', label: '专家' },
  { value: 'domain', label: '能力域' },
  { value: 'capability', label: '能力点' }
])

/** 图中真实存在的边类型，权重来源逐字取自 builder */
export const GRAPH_EDGE_TYPE = Object.freeze({
  has_domain: 'has_domain',
  collaborates_with: 'collaborates_with'
})

export const GRAPH_EDGE_TYPES = Object.freeze([
  {
    value: GRAPH_EDGE_TYPE.has_domain,
    label: '专家—能力域',
    weightHint: '固定权重 1.0'
  },
  {
    value: GRAPH_EDGE_TYPE.collaborates_with,
    label: '专家协作',
    weightHint: '共享能力域的 Jaccard 相似度，仅 >0.1 时建边'
  }
])

/** builder 用 format!("domain-{}", domain) 造领域节点 id */
export const DOMAIN_NODE_ID_PREFIX = 'domain-'

export function isDomainNode(id) {
  return str(id).startsWith(DOMAIN_NODE_ID_PREFIX)
}

/**
 * 编码丢失的标签：实测 GET /api/expert-graph 里存在 label='???????' 与 '?????????' 的专家节点，
 * 而同一网关经 UTF-8 请求体写入的中文名可以原样读回（2026-09-27 往返探针），
 * 所以这串问号是**写入侧**就丢的字符，不是渲染层的字体问题。
 * 界面不得照抄一串问号，也不得拿它当真名去问专家。
 */
const LOST_LABEL_RE = /^[\s?]*$/

export function isLostGraphLabel(label) {
  return LOST_LABEL_RE.test(str(label))
}

/** id 短码：编码丢失时唯一还能指认这个节点的东西（exp-/domain- 前缀对读的人没有信息量） */
export function graphNodeShortId(id) {
  return str(id).replace(/^(exp|domain)-/, '').slice(0, 8)
}

/**
 * 节点的可读名，图上的标签、信息卡标题与"带节点去协作"的提问都用它。
 * 编码丢失时给"未命名节点 + id 短码"，宁可难读也不假装读得懂。
 */
export function graphNodeLabel(node) {
  const label = str(node?.label)
  if (label && !isLostGraphLabel(label)) return label
  const short = graphNodeShortId(node?.id)
  return short ? `未命名节点 ${short}` : '未命名节点'
}

/**
 * 注册表里一个专家的可读名，与 graphNodeLabel 共用同一套判丢与短码口径（不另起第二张表）。
 * 只管"名字丢了/没名字"这一档；若调用方知道**注册表里根本没有这一行**，
 * 那句话要调用方自己说（例如调度台的"（注册表里没有名字）"），不许混进这里来。
 */
export function expertDisplayName(expert) {
  const name = str(expert?.name)
  if (name && !isLostGraphLabel(name)) return name
  const short = graphNodeShortId(expert?.id)
  return short ? `未命名专家 ${short}` : '未命名专家'
}

/**
 * 区分"名字丢了"与"压根没给名字"两种缺名的出口专用。
 * 一个字符都没有（后端字段为空/缺失）时把话交回调用方：那一档的意思是"这里没有这个人可指认"，
 * 不许在本模块编造人名；给了名字但名字丢了码的，仍走 expertDisplayName 显形成 id 短码。
 * 判空口径与 graphNodeLabel 一致：只看是否为空串，空白串算"给过但丢了"。
 */
export function expertNameOr(expert, absentText) {
  const name = str(expert?.name)
  if (!name) return str(absentText)
  return expertDisplayName(expert)
}

export function nodeTypeMeta(nodeType) {
  return GRAPH_NODE_TYPE_META[nodeType] || { label: str(nodeType) || '未知', abbr: '?' }
}

export function edgeTypeMeta(edgeType) {
  return GRAPH_EDGE_TYPES.find((e) => e.value === edgeType) || { label: str(edgeType) || '未知', weightHint: '' }
}

// ── 最优团队组建（POST /api/expert-graph/optimal-team）─────────────────

/** 缺省值取自 post_optimal_team 的 unwrap_or，不是前端偏好 */
export const OPTIMAL_TEAM_DEFAULTS = Object.freeze({
  maxMembers: 5,
  minRating: 4
})

/**
 * 请求体字段规格：key 为 wire 名（serde 无 alias，写错名会被静默丢弃），
 * mounted 表示本模块是否真的发该键。
 */
export const OPTIMAL_TEAM_FIELDS = Object.freeze([
  { key: 'required_skills', mounted: true, kind: 'tags', label: '所需技能', default: [] },
  { key: 'required_domains', mounted: true, kind: 'tags', label: '所需能力域', default: [] },
  { key: 'max_members', mounted: true, kind: 'number', label: '团队人数上限', min: 1, max: 20, default: 5 },
  { key: 'min_rating', mounted: true, kind: 'number', label: '最低评分', min: 0, max: 5, step: 0.1, default: 4 },
  { key: 'goal', mounted: true, kind: 'text', label: '目标描述', default: '' },
  { key: 'constraints', mounted: false, kind: 'unknown', label: '约束', default: null }
])

/**
 * max_members 与 min_rating 的真实后端语义，界面必须照此说明而不是假装受控。
 */
export const OPTIMAL_TEAM_BACKEND_RULES = Object.freeze({
  maxMembers: '后端无上限：仅把 0 当缺省 5，填 20 就真的尝试选 20 人（候选人不足时自然停止）',
  minRating: '后端不校验区间：>5 只会因无候选人而返回空团队，不会报错',
  goal: '仅当技能与能力域都为空时，后端才从 goal 文本按别名表提取真实存在的域/技能 id',
  availability: '候选门槛为 enabled 且 avg_rating ≥ min_rating 且 status ≠ offline；覆盖值 = 命中数 × (评分/5) × 可用系数（online 1.0 / busy 0.6 / away 0.4 / 其他 0.2）'
})

/** find_optimal_team 给成员打的 role 标签，取自 handler 的三个分支 */
export const OPTIMAL_TEAM_ROLES = Object.freeze({
  domain_lead: '域负责人',
  skill_expert: '技能专家',
  consultant: '顾问'
})

export function optimalTeamRoleLabel(role) {
  const key = str(role)
  return OPTIMAL_TEAM_ROLES[key] || key || '未标注'
}

export function optimalTeamFields() {
  return OPTIMAL_TEAM_FIELDS.map((f) => f.key)
}

const fieldOf = (key) => OPTIMAL_TEAM_FIELDS.find((f) => f.key === key) || {}

/**
 * 生成请求体：只发对结果有意义的键。
 * - 空数组与后端 serde(default) 等价，不发；
 * - 数值等于后端缺省时不发，避免"前端替后端记住默认值"；
 * - goal 与显式需求互斥（后端只在两者都空时才读 goal），同时填时不发 goal，
 *   界面上也要说明以显式需求为准。
 */
export function optimalTeamBody(input = {}) {
  const body = {}
  const skills = list(input.requiredSkills)
  const domains = list(input.requiredDomains)
  if (skills.length) body.required_skills = skills
  if (domains.length) body.required_domains = domains

  const goal = str(input.goal).trim()
  if (goal && !skills.length && !domains.length) body.goal = goal

  const maxMembers = num(input.maxMembers)
  if (Number.isFinite(maxMembers) && maxMembers > 0 && maxMembers !== OPTIMAL_TEAM_DEFAULTS.maxMembers) {
    body.max_members = Math.trunc(maxMembers)
  }
  const minRating = num(input.minRating)
  if (Number.isFinite(minRating) && minRating >= 0 && minRating !== OPTIMAL_TEAM_DEFAULTS.minRating) {
    body.min_rating = minRating
  }
  return body
}

/** 提交前的前端自检：越界后端不会拦，所以由前端拦在输入面；未填则按后端缺省放行 */
export function optimalTeamProblem(input = {}) {
  const skills = list(input.requiredSkills)
  const domains = list(input.requiredDomains)
  const goal = str(input.goal).trim()
  if (!skills.length && !domains.length && !goal) return '请至少给出技能、能力域或一句目标描述'
  const unset = (v) => v === undefined || v === null || v === ''
  if (!unset(input.maxMembers)) {
    const maxMembers = num(input.maxMembers)
    if (!Number.isFinite(maxMembers) || maxMembers < 1) return '团队人数上限至少为 1'
    if (maxMembers > fieldOf('max_members').max) return `团队人数上限不超过 ${fieldOf('max_members').max}`
  }
  if (!unset(input.minRating)) {
    const minRating = num(input.minRating)
    if (!Number.isFinite(minRating) || minRating < 0 || minRating > 5) return '最低评分须在 0–5 之间'
  }
  return ''
}

/** 覆盖率人话：后端给的是 ratio 分子分母，界面必须说清缺什么 */
export function coverageText(coverage = {}) {
  const total = Number(coverage.requiredTotal) || 0
  const covered = Number(coverage.coveredCount) || 0
  const missing = [...(coverage.missingSkills || []), ...(coverage.missingDomains || [])]
  if (!total) return '未给出需求项，后端按全覆盖（ratio 1）返回空团队'
  const ratio = Math.round((Number(coverage.coverageRatio) || 0) * 100)
  const tail = missing.length ? `，未覆盖：${missing.join('、')}` : '，需求已全部覆盖'
  return `覆盖 ${covered}/${total}（${ratio}%）${tail}`
}

// ── 协作者列表（GET /api/expert-graph/collaborators/:id）───────────────

/** handler: params.get("limit").and_then(parse).unwrap_or(10)，非正整数一律回落 10 */
export const COLLABORATOR_LIMIT_DEFAULT = 10
export const COLLABORATOR_LIMITS = Object.freeze([5, 10, 20, 50])

/** 只有正整数才值得发出去；其余取值后端都会退回 10，发了反而误导 */
export function collaboratorQuery(limit) {
  const v = num(limit)
  if (!Number.isFinite(v) || v < 1 || v === COLLABORATOR_LIMIT_DEFAULT) return undefined
  return { limit: Math.trunc(v) }
}

/** 后端排序键：collaboration_weight 降序，rank 从 1 起 */
export function collaboratorRows(payload = {}) {
  const rows = Array.isArray(payload.collaborators) ? payload.collaborators : []
  return rows.map((c) => ({
    rank: Number(c.collaborationRank) || 0,
    id: c.id,
    name: c.name,
    weight: Number(c.collaborationWeight) || 0,
    sharedDomains: Array.isArray(c.sharedDomains) ? c.sharedDomains : []
  }))
}

// ── 图 RAG 多跳邻域扩展（POST /api/expert-graph/rag/expand，T2）──────────
//
// 后端语义（experts_graph.rs 七-C）：
// - max_depth 合法范围 1..=4，缺省 2；top_k 缺省 20 且必须 >0；
// - min_weight ∈ [0,1]，低于该值的边不沿其展开；
// - node_types 为空=不过滤；过滤作用在结果侧，路径仍保留完整节点序列；
// - aggregate_weight = 路径边权重乘积（w∈[0,1]，随深度衰减）；
// - rerank 当前固定为 graph_only（向量融合待 #27），前端不得假装有向量分数。

export const RAG_EXPAND_DEFAULTS = Object.freeze({
  maxDepth: 2,
  topK: 20,
  minWeight: 0
})

export const RAG_EXPAND_DEPTH_RANGE = Object.freeze({ min: 1, max: 4 })

/**
 * 生成请求体：只发对结果有意义的键。
 * - seeds 必发（非空字符串数组）；
 * - 数值等于后端缺省时不发；
 * - node_types 为空不发（后端缺省=不过滤）。
 */
export function ragExpandBody(input = {}) {
  const body = {}
  const seeds = list(input.seeds)
  if (!seeds.length) return body
  body.seeds = seeds
  const maxDepth = num(input.maxDepth)
  if (Number.isFinite(maxDepth) && maxDepth >= RAG_EXPAND_DEPTH_RANGE.min && maxDepth <= RAG_EXPAND_DEPTH_RANGE.max && maxDepth !== RAG_EXPAND_DEFAULTS.maxDepth) {
    body.max_depth = Math.trunc(maxDepth)
  }
  const topK = num(input.topK)
  if (Number.isFinite(topK) && topK > 0 && topK !== RAG_EXPAND_DEFAULTS.topK) {
    body.top_k = Math.trunc(topK)
  }
  const minWeight = num(input.minWeight)
  if (Number.isFinite(minWeight) && minWeight >= 0 && minWeight <= 1 && minWeight !== RAG_EXPAND_DEFAULTS.minWeight) {
    body.min_weight = minWeight
  }
  const types = list(input.nodeTypes)
  if (types.length) body.node_types = types
  return body
}

/** 提交前自检：seeds 空 / max_depth 越界后端会 400，由前端先挡 */
export function ragExpandProblem(input = {}) {
  const seeds = list(input.seeds)
  if (!seeds.length) return '请至少给出一个种子节点'
  const unset = (v) => v === undefined || v === null || v === ''
  if (!unset(input.maxDepth)) {
    const d = num(input.maxDepth)
    if (!Number.isFinite(d) || d < RAG_EXPAND_DEPTH_RANGE.min || d > RAG_EXPAND_DEPTH_RANGE.max) {
      return `跳数须在 ${RAG_EXPAND_DEPTH_RANGE.min}–${RAG_EXPAND_DEPTH_RANGE.max} 之间`
    }
  }
  if (!unset(input.topK)) {
    const k = num(input.topK)
    if (!Number.isFinite(k) || k < 1) return '返回条数至少为 1'
  }
  return ''
}

/** 召回行归一化：路径/首跳兜底成空数组，weight 兜底 0 */
export function ragExpandRows(payload = {}) {
  const results = Array.isArray(payload.results) ? payload.results : []
  return results.map((r) => {
    const node = r.node || {}
    const firstHops = Array.isArray(r.first_hops) ? r.first_hops : []
    return {
      id: String(node.id || ''),
      label: String(node.label || ''),
      nodeType: String(node.node_type || ''),
      depth: Number(r.depth) || 0,
      aggregateWeight: Number(r.aggregate_weight) || 0,
      path: Array.isArray(r.path) ? r.path.map(String) : [],
      firstHops: firstHops.map((fh) => ({
        from: String(fh.from || ''),
        to: String(fh.to || ''),
        edgeType: String(fh.edge_type || ''),
        weight: Number(fh.weight) || 0
      }))
    }
  })
}
