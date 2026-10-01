// 智能协作流程契约：六种协作意图的后端入参/边界/出参口径单源。
// 权威源：platform/gateway/mox-platform-gateway-svc/src/alliance/experts_collaboration.rs
// 每个 endpoint/字段/上下界都由 contract.test.js 与 Rust handler 逐字核对；
// 后端加字段或改 clamp 而没同步前端时，测试先红。
//
// 为什么必须在这一层写死 wire 名：Rust body 结构体只给 RouteBody 的两个字段配了 alias，
// 其余字段传 camelCase 会被 serde 静默丢弃——存量页面的「参与专家数选了没用」就是这么来的
// （views/workspace/ExpertWorkspaceView.vue:600 发 maxExperts，后端认 max_experts/top_n）。

export const COLLAB_MODE = Object.freeze({
  ROUTE: 'route',
  SINGLE: 'single',
  MULTI: 'multi',
  DEBATE: 'debate',
  SMART: 'smart',
  ALGORITHM: 'algorithm'
})

// debate()：少于 2 名直接 400，超过 4 名的部分不会上场
export const DEBATE_MIN_PARTICIPANTS = 2
export const DEBATE_PARTICIPANT_CAP = 4

/**
 * 模式表。wires 是该模式允许进入请求体的后端字段名（含主文本字段与数值控件），
 * 只发列出的键：serde 对未知字段静默丢弃，多发的键不报错，
 * 只会让「前端填了、后端没看见」变成无感知的功能缺失。
 * controls 的 min/max/default 逐一取自 handler 的 clamp/unwrap_or 字面量。
 */
export const COLLAB_MODES = Object.freeze([
  {
    key: COLLAB_MODE.ROUTE,
    label: '智能路由',
    endpoint: 'expertRoute',
    field: 'question',
    fieldLabel: '要路由的问题',
    placeholder: '例如：网关层如何拆分才能兼顾灰度与成本？',
    // route_query 只按约束挑选专家，不生成任何回复
    expertChoice: 'none',
    wires: ['question', 'domain', 'max_experts', 'constraints'],
    controls: [{ wire: 'max_experts', label: '返回候选数', min: 1, max: 20, default: 5 }],
    // constraints 的三个子键逐一取自 route_query() 的 unwrap_or 默认值：等于默认即不发
    constraintFields: [
      { key: 'min_rating', label: '候选最低评分', kind: 'number', min: 0, max: 5, step: 0.5, neutral: 0, hint: '0 表示不限' },
      { key: 'max_response_time', label: '候选最长响应（分钟）', kind: 'number', min: 1, max: 240, step: 5, neutral: null, hint: '留空表示不限' },
      { key: 'require_online', label: '只路由在线专家', kind: 'switch', neutral: false, hint: '后端按 availability==online 过滤' }
    ],
    resultKind: 'routing',
    outcome: '只给候选与推荐，不产出专家回复'
  },
  {
    key: COLLAB_MODE.SINGLE,
    label: '单专家咨询',
    endpoint: 'expertConsult',
    field: 'question',
    fieldLabel: '向所选专家提问',
    placeholder: '例如：评审我们的多活租约选主方案',
    // 专家走路径参数，body 结构体里也没有可读的 context（见 COLLAB_LAZY_FIELDS）
    expertChoice: 'one',
    wires: ['question'],
    controls: [],
    resultKind: 'answer',
    outcome: '1 位专家的结构化回复（分析 + 方案 + 置信度）'
  },
  {
    key: COLLAB_MODE.MULTI,
    label: '多专家协同',
    endpoint: 'multiConsult',
    field: 'question',
    fieldLabel: '协同咨询问题',
    placeholder: '例如：设计一套风控规则引擎，需兼顾可解释性',
    // 未指定专家时走 match_top_experts（阈值 0.3），匹配不到直接 404，故提示可选而非必需
    expertChoice: 'optional',
    wires: ['question', 'expert_ids', 'domain', 'max_experts'],
    controls: [{ wire: 'max_experts', label: '参与专家数', min: 1, max: 10, default: 3 }],
    resultKind: 'fusion',
    outcome: '多位专家并行作答后加权融合，给出共识度与主导方案'
  },
  {
    key: COLLAB_MODE.DEBATE,
    label: '专家辩论',
    endpoint: 'expertDebate',
    // 后端字段是 topic：发 question 会被 axum 以「missing field `topic`」拒成 422
    field: 'topic',
    fieldLabel: '辩题',
    placeholder: '例如：微服务架构是否优于单体架构？',
    expertChoice: 'optional',
    wires: ['topic', 'expert_ids', 'rounds'],
    controls: [{ wire: 'rounds', label: '辩论轮数', min: 1, max: 10, default: 3 }],
    resultKind: 'debate',
    participantCap: DEBATE_PARTICIPANT_CAP,
    // debate() 的每条论点出自 generate_debate_point() 的 format!、分数是 seed 扰动，
    // 全程不调模型（experts_collaboration.rs:530-536 与 :592-606）⇒ 界面必须标注来源。
    generatedBy: 'template',
    outcome: '正反方逐轮交锋（后端模板生成，非模型推理），按可复现扰动评分后裁决'
  },
  {
    key: COLLAB_MODE.SMART,
    label: '智能咨询',
    endpoint: 'intelligentConsult',
    field: 'question',
    fieldLabel: '问题描述',
    placeholder: '例如：大模型 RAG 方案如何选型？',
    // 后端按 classify_intent 自行匹配最佳专家，前端无法指定，也没有领域过滤入口
    expertChoice: 'none',
    wires: ['question', 'context'],
    controls: [],
    resultKind: 'answer',
    outcome: '自动意图分类 + 匹配专家，回复附行动项与风险评估'
  },
  {
    key: COLLAB_MODE.ALGORITHM,
    label: '算法分析',
    endpoint: 'algorithmAnalysis',
    field: 'algorithm_description',
    fieldLabel: '算法描述',
    placeholder: '例如：使用递归求解斐波那契数列，未做记忆化',
    // 只有 description 参与推断，constraints/requirements 后端仅回显
    expertChoice: 'none',
    wires: ['algorithm_description'],
    controls: [],
    resultKind: 'complexity',
    outcome: '按描述关键词推断时间/空间复杂度，给出可行性与优化建议'
  }
])

/** 本地 camel 输入键 → 后端 body 字段；只列 handler 真正读的键 */
export const COLLAB_WIRE = Object.freeze({
  question: 'question',
  topic: 'topic',
  expertIds: 'expert_ids',
  domain: 'domain',
  context: 'context',
  maxExperts: 'max_experts',
  rounds: 'rounds',
  constraints: 'constraints'
})

// 输入键与 wire 同名（模式的主文本字段随模式变化，同名可免去一层查表）
const CONTROL_WIRES = new Set(['max_experts', 'rounds'])

/**
 * 结构体接受、但 handler 从不读的键：发出去即石沉大海，前端既不发也不渲染回显。
 * struct 为 Rust 结构体名，mode 为其所属模式。
 */
export const COLLAB_LAZY_FIELDS = Object.freeze([
  { struct: 'ConsultBody', key: 'context', mode: COLLAB_MODE.SINGLE, reason: 'consult_expert() 全文未使用 body.context' },
  { struct: 'ConsultBody', key: 'priority', mode: COLLAB_MODE.SINGLE, reason: 'consult_expert() 未使用 body.priority' },
  { struct: 'DebateBody', key: 'stance', mode: COLLAB_MODE.DEBATE, reason: 'debate() 未使用 body.stance，正反方由引擎自行分配' },
  { struct: 'IntelligentConsultBody', key: 'history', mode: COLLAB_MODE.SMART, reason: 'intelligent_consult() 未使用 body.history，无多轮上下文' }
])

export function collabMode(key) {
  return COLLAB_MODES.find((m) => m.key === key) || null
}

/**
 * 该模式的结果是否由后端模板（而非模型）产出：是则返回必须出现在界面上的标注文案，否则空串。
 * 口径只在这里定义一次，聊天正文与列表型界面共用，避免一处标注一处不标。
 */
export function collabTemplateNote(mode) {
  return mode?.generatedBy === 'template'
    ? '（辩论内容由后端模板生成，未经真实模型推理）'
    : ''
}

export function collabModeKeys() {
  return COLLAB_MODES.map((m) => m.key)
}

/** 该模式的请求体是否接受某后端字段：视图按它决定要不要渲染对应输入项 */
export function collabAccepts(mode, wire) {
  return !!mode && (mode.wires || []).includes(wire)
}

/** 某模式的控件取值（缺省回落 default，并夹到 [min,max]，与后端 clamp 同序） */
export function collabControlValue(mode, values = {}) {
  return (mode.controls || []).map((c) => {
    const raw = Number(values[c.wire] ?? c.default)
    const v = Number.isFinite(raw) ? raw : c.default
    return { wire: c.wire, label: c.label, value: Math.min(Math.max(v, c.min), c.max) }
  })
}

/**
 * 路由约束取值：数值夹到边界、空串按「不限」，开关按布尔。
 * 是否进入请求体由 collabBody() 按 neutral 判定，视图只负责回显当前值。
 */
export function collabConstraintValues(mode, values = {}) {
  return (mode?.constraintFields || []).map((f) => {
    if (f.kind === 'switch') return { ...f, value: values[f.key] === undefined ? f.neutral : !!values[f.key] }
    const raw = values[f.key]
    const n = Number(raw)
    // 空值与非法值都回落到后端的默认口径（0 / 无上限），而不是造出一个 0 分或 0 分钟
    const value = raw === '' || raw === null || raw === undefined || !Number.isFinite(n)
      ? f.neutral
      : Math.min(Math.max(n, f.min), f.max)
    return { ...f, value }
  })
}

/**
 * 运行前校验：把后端的 400/404/422 条件前移到 UI，返回中文原因（空串表示可发）。
 * 规则逐条来自 handler，见 contract.test.js 的「前端校验与后端拒绝条件互斥」。
 */
export function collabProblem(mode, input = {}) {
  if (!mode) return '未知协作模式'
  const text = String(input[mode.field] ?? '').trim()
  if (!text) return `请填写${mode.fieldLabel}`
  const ids = arrOf(input.expertIds)
  if (mode.expertChoice === 'one' && ids.length !== 1) return '请先选择 1 位专家'
  if (mode.expertChoice === 'none' && ids.length) return `${mode.label}由后端自行匹配专家，无需选择`
  // debate()：debaters.len() < 2 → 400
  if (mode.key === COLLAB_MODE.DEBATE && ids.length > 0 && ids.length < DEBATE_MIN_PARTICIPANTS) {
    return `辩论至少需要 ${DEBATE_MIN_PARTICIPANTS} 名专家`
  }
  return ''
}

/**
 * 输入 → 后端 body：只发 mode.wires 列出的键，空值不发。
 * 单专家咨询的专家走路径参数，因此不落在 body 里。
 */
export function collabBody(mode, input = {}) {
  const controls = collabControlValue(mode, input)
  const body = {}
  for (const wire of mode.wires) {
    if (CONTROL_WIRES.has(wire)) continue
    if (wire === COLLAB_WIRE.expertIds) {
      // 后端按所选专家作答；未选则由 match_top_experts 自动匹配
      if (mode.key === COLLAB_MODE.SINGLE || mode.expertChoice === 'none') continue
      const ids = arrOf(input.expertIds)
      if (ids.length) body[wire] = ids
      continue
    }
    if (wire === COLLAB_WIRE.constraints) {
      // 只发偏离后端默认值的子键：route_query 对缺失键 unwrap_or 到 0.0 / 无上限 / false
      const picked = {}
      for (const f of collabConstraintValues(mode, input)) {
        if (f.kind === 'switch') {
          if (f.value !== f.neutral) picked[f.key] = f.value
        } else if (f.value !== null && f.value !== f.neutral) picked[f.key] = f.value
      }
      if (Object.keys(picked).length) body[wire] = picked
      continue
    }
    const raw = input[wire]
    const value = typeof raw === 'string' ? raw.trim() : raw
    if (value === '' || value === undefined || value === null) continue
    body[wire] = value
  }
  for (const c of controls) {
    if (mode.wires.includes(c.wire)) body[c.wire] = c.value
  }
  return body
}

/** 辩论参与上限提示文案：后端 take(4)，多选不会全部上场 */
export function debateCapacityText(picked) {
  return picked > DEBATE_PARTICIPANT_CAP
    ? `已选 ${picked} 位，后端仅取前 ${DEBATE_PARTICIPANT_CAP} 位上场`
    : ''
}

// classify_intent（experts_collaboration.rs）把问题映射到固定领域，
// intelligent-consult 的 intent 字段取值即这份清单，前端必须有文案。
export const INTENT_LABELS = Object.freeze({
  architecture: '架构',
  ai: '人工智能',
  data: '数据',
  security: '安全',
  cloud: '云原生',
  product: '产品',
  frontend: '前端',
  math: '数学与算法',
  finance: '金融',
  enterprise: '企业',
  general: '通用'
})

export function intentLabel(intent) {
  const key = strOf(intent)
  return INTENT_LABELS[key] || key || '未知'
}

// analyze_complexity 只产出这 6 种记号；按渐近序排列以便前端判断「越靠后越危险」。
export const COMPLEXITY_ORDER = Object.freeze(['O(1)', 'O(log n)', 'O(n)', 'O(n log n)', 'O(n²)', 'O(2^n)'])

export function complexityLevel(notation) {
  const idx = COMPLEXITY_ORDER.indexOf(strOf(notation))
  if (idx < 0) return { tag: 'info', label: strOf(notation) || '未知' }
  if (idx >= 4) return { tag: 'danger', label: `${COMPLEXITY_ORDER[idx]} 高风险` }
  if (idx >= 3) return { tag: 'warning', label: `${COMPLEXITY_ORDER[idx]} 可控` }
  return { tag: 'success', label: `${COMPLEXITY_ORDER[idx]} 良好` }
}

// ── 协作结果文案：口径写在契约层，视图不得各自发明格式化 ─────────────
/** 回复来源：模板降级与真实模型的可信度不同，必须显式区分呈现 */
export function answerSourceText(answer) {
  const a = answer || {}
  if (a.blocked || a.vetoed) return '已被治理闸门拦截'
  return a.source === 'llm' ? '真实模型作答' : '模板兜底作答'
}

export function confidenceText(value) {
  const n = Number(value)
  return Number.isFinite(n) && n > 0 ? `${Math.round(n * 100)}%` : '—'
}

/**
 * 共识度取自两两 text_similarity 的均值：不足 2 位贡献者时后端恒给 1.0，
 * 那是「无从比对」而不是「完全一致」，不能显示成高分共识。
 */
export function consensusText(fusion, contributors) {
  if (Number(contributors) < 2) return '—（单专家无从比对共识度）'
  const n = Number(fusion?.consensusScore)
  return Number.isFinite(n) ? n.toFixed(2) : '—'
}

const arrOf = (v) => (Array.isArray(v) ? v.filter((x) => x !== '' && x !== null && x !== undefined) : [])
const strOf = (v) => (v === undefined || v === null ? '' : String(v))
