// 双璇玑十四维的唯一前端词表。权威源：
//   platform/domains/platform/svc/mox-platform-orchestrator-svc/src/handlers/governance.rs
//   —— 该文件里同一份十四维清单手抄了三遍（ExpertConfig::default 的两张权重表、
//      experts_status_handler 的两个数组、trigger_governance 的两个数组），
//      governance-contract.test.js 逐处现读并要求三处 + 本表四者完全一致、分母为 14。
// 维度串既是 expertStates / businessWeights 的 map 键，也是 VetoEvent.dimension 的值 ⇒ 它是连接三张表的轴。

export const LEAGUE = Object.freeze({
  business: 'business',
  dev: 'dev'
})

export const DIMENSIONS = Object.freeze([
  { id: 'business', league: LEAGUE.business, label: '业务一致性', note: '流程是否服务既定业务目标' },
  { id: 'algorithm', league: LEAGUE.business, label: '算法正确性', note: '璇玑验证所在维，该维否决不可被权限或合规覆盖' },
  { id: 'permission', league: LEAGUE.business, label: '权限合规', note: '主体角色与资源权限匹配' },
  { id: 'resource', league: LEAGUE.business, label: '资源占用', note: 'CPU/内存/并发额度' },
  { id: 'security', league: LEAGUE.business, label: '安全', note: '注入、越权、敏感面' },
  { id: 'data', league: LEAGUE.business, label: '数据治理', note: '来源、血缘、留存' },
  { id: 'observability', league: LEAGUE.business, label: '可观测性', note: '日志、指标、追踪是否齐备' },
  { id: 'api_compat', league: LEAGUE.dev, label: '接口兼容', note: '契约破坏性变更' },
  { id: 'performance', league: LEAGUE.dev, label: '性能', note: '时延与吞吐回退' },
  { id: 'maintainability', league: LEAGUE.dev, label: '可维护性', note: '复杂度与重复' },
  { id: 'testing', league: LEAGUE.dev, label: '测试覆盖', note: '断言与用例是否兜住改动' },
  { id: 'style', league: LEAGUE.dev, label: '风格一致', note: '命名与格式约定' },
  { id: 'cost', league: LEAGUE.dev, label: '成本', note: 'Token 与算力开销' },
  { id: 'sensitive', league: LEAGUE.dev, label: '敏感信息', note: '密钥、隐私泄漏面' }
])

export const DIM_IDS = Object.freeze(DIMENSIONS.map((d) => d.id))

export const BUSINESS_DIM_IDS = Object.freeze(
  DIMENSIONS.filter((d) => d.league === LEAGUE.business).map((d) => d.id)
)
export const DEV_DIM_IDS = Object.freeze(
  DIMENSIONS.filter((d) => d.league === LEAGUE.dev).map((d) => d.id)
)

const BY_ID = new Map(DIMENSIONS.map((d) => [d.id, d]))

/** 未知维度不编标签：原样回显串，界面上"看得见这是个没登记的维"。 */
export function dimLabel(id) {
  return BY_ID.get(id)?.label ?? id
}

export function dimNote(id) {
  return BY_ID.get(id)?.note ?? ''
}

export function leagueOf(id) {
  return BY_ID.get(id)?.league ?? ''
}

export function leagueLabel(league) {
  return { [LEAGUE.business]: '业务璇玑', [LEAGUE.dev]: '开发璇玑' }[league] ?? league
}

// ── 阈值口径 ────────────────────────────────────────────────────────────────
// ExpertConfig::default() 的三档缺省值（handlers/governance.rs:406-410）。
// 只作为"配置没取到"时的兜底显示，且必须标明是缺省值：
// trigger_governance 判定哪些维度产生否决事件用的是**字面量 0.5**（:996-999 的 filter），
// 它不读 expert_config.thresholds ⇒ 改 veto_threshold 不会改变否决事件的生成条件，
// 这一条在治理台界面上要如实写出来，否则用户以为调阈值能调否决。
export const DEFAULT_THRESHOLDS = Object.freeze({
  vetoThreshold: 0.3,
  warnThreshold: 0.6,
  healthMin: 0.5
})

/** 硬编码在否决生成里的那个 0.5，界面文案要引用它而不是配置值 */
export const VETO_EVENT_THRESHOLD_LITERAL = 0.5

/** 健康分 → 色调。阈值来自配置（有则用配置，无则缺省值 + 标注）。 */
export function healthTone(score, thresholds = DEFAULT_THRESHOLDS) {
  const s = Number(score)
  if (!Number.isFinite(s)) return 'unknown'
  if (s < (thresholds.vetoThreshold ?? DEFAULT_THRESHOLDS.vetoThreshold)) return 'danger'
  if (s < (thresholds.warnThreshold ?? DEFAULT_THRESHOLDS.warnThreshold)) return 'warning'
  return 'success'
}

// ── 词表：wire 值逐枚点名，来源见注释 ──────────────────────────────────────
// VetoEvent.severity：trigger_governance 只产这两档（score<0.3 critical，否则 warning）
export const VETO_SEVERITY = Object.freeze({
  critical: 'critical',
  warning: 'warning'
})

// GateResultDto.status = format!("{:?}", FlowStatus)，枚举在 mox-ai-expert-core/src/govern/mod.rs:22-29
export const FLOW_STATUS = Object.freeze(['Draft', 'Review', 'Approved', 'Blocked', 'Deprecated'])

export const FLOW_STATUS_LABEL = Object.freeze({
  Draft: '草稿',
  Review: '待评审',
  Approved: '放行',
  Blocked: '拦截',
  Deprecated: '已废弃'
})

export function flowStatusLabel(status) {
  return FLOW_STATUS_LABEL[status] ?? status
}

// AuditLogEntry.decision：trigger_governance 写 "approved"/"blocked"（:1043），
// 配置类端点写的是 `version=N` 这样的自由串（:759）⇒ 界面不许把它当枚举渲染。
export const AUDIT_DECISION = Object.freeze({ approved: 'approved', blocked: 'blocked' })
