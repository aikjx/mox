// 治理台 wire → 视图模型的唯一规范化层。
// 三条与其他域不同的口径，逐条来自 Rust 实测（governance-contract.test.js 现读源码守护）：
//  1. 这些 DTO 全带 #[serde(rename_all = "camelCase")] ⇒ 线上是 expertId / healthScore / pageSize，
//     不是联盟域那种 snake_case。读错形状不会报错，只会静默 undefined ⇒ 界面一片兜底值。
//  2. ts / updatedAt / lastUpdated 是 **epoch 秒**（unix_ts() = as_secs()），
//     而 utils/time.js 的 timeValue 把数字按毫秒解释 ⇒ 不在这一层乘 1000，界面会显示 1970 年。
//  3. expertStates / businessWeights / businessScores 都是以维度字符串为键的 map，
//     不是数组 ⇒ 不走 unwrapList，且展示顺序要由 DIM_IDS 决定（Object.values 的序不受控）。
import { timeValue } from '@/utils'
import {
  BUSINESS_DIM_IDS, DEV_DIM_IDS, DIM_IDS, LEAGUE, VETO_SEVERITY, dimLabel, healthTone, leagueLabel, leagueOf
} from '@/modules/governance/contract/dimensions'

const num = (v, d = 0) => (Number.isFinite(Number(v)) ? Number(v) : d)
const str = (v) => (v === undefined || v === null ? '' : String(v))
const arr = (v) => (Array.isArray(v) ? v : [])
const bool = (v) => v === true
const obj = (v) => (v && typeof v === 'object' && !Array.isArray(v) ? v : {})

/** epoch 秒 → 毫秒；拿不到值返回 NaN，由显示层决定兜底文案（这里不编时间）。 */
export function secsToMs(v) {
  if (v === null || v === undefined || v === '') return NaN
  const n = Number(v)
  if (!Number.isFinite(n) || n <= 0) return NaN
  // 后端给秒；若某端已给毫秒（>1e12）就不再乘，避免自我放大
  return n > 1e12 ? n : n * 1000
}

/** timeValue 是 utils/time.js 的单源口径（RFC3339 串 / Date / 数字都先落毫秒）。 */
export const wireTimeMs = (v) => {
  const ms = secsToMs(v)
  return Number.isFinite(ms) ? ms : timeValue(v)
}

export function normGateResult(raw) {
  if (!raw) return null
  const g = obj(raw)
  return {
    status: str(g.status),
    approved: bool(g.approved),
    slaOk: bool(g.slaOk),
    budgetOk: bool(g.budgetOk),
    blockingRisks: num(g.blockingRisks),
    algorithmVeto: bool(g.algorithmVeto),
    reason: str(g.reason)
  }
}

export function normVetoEvent(raw) {
  const e = obj(raw)
  return {
    id: str(e.id),
    flowId: str(e.flowId),
    flowName: str(e.flowName),
    expertId: str(e.expertId),
    dimension: str(e.dimension),
    dimLabel: dimLabel(str(e.dimension)),
    league: leagueOf(str(e.dimension)),
    reason: str(e.reason),
    severity: str(e.severity),
    isCritical: str(e.severity) === VETO_SEVERITY.critical,
    tsMs: wireTimeMs(e.ts),
    blocked: bool(e.blocked),
    gate: normGateResult(e.gateResult)
  }
}

/** ExpertStatus（map 的值形状）；维度串由调用方从键传进来。 */
export function normExpertState(dim, raw) {
  const s = obj(raw)
  const score = num(s.healthScore, NaN)
  return {
    dimension: str(s.dimension) || dim,
    expertId: str(s.expertId) || dim,
    dimLabel: dimLabel(str(s.dimension) || dim),
    league: leagueOf(str(s.dimension) || dim),
    healthScore: Number.isFinite(score) ? score : NaN,
    tone: healthTone(score),
    enabled: bool(s.enabled),
    lastUpdatedMs: wireTimeMs(s.lastUpdated),
    vetoCount: num(s.vetoCount),
    totalChecks: num(s.totalChecks),
    // 分母为 0 时后端给 0，界面要区分"没检查过"和"检查过零否决"
    checked: num(s.totalChecks) > 0
  }
}

function expertRowList(statesMap) {
  const map = obj(statesMap)
  return DIM_IDS.filter((id) => map[id]).map((id) => normExpertState(id, map[id]))
}

/** DashboardData → 视图模型 */
export function normDashboard(raw) {
  const d = obj(raw)
  const experts = expertRowList(d.expertStates)
  const byLeague = (league) => experts.filter((e) => e.league === league)
  return {
    serverTsMs: wireTimeMs(d.timestamp),
    counts: {
      totalFlows: num(d.totalFlows),
      approvedFlows: num(d.approvedFlows),
      blockedFlows: num(d.blockedFlows),
      draftFlows: num(d.draftFlows),
      reviewFlows: num(d.reviewFlows)
    },
    vetoRate: num(d.vetoRate),
    auditEventCount: num(d.auditEventCount),
    auditChainVerified: bool(d.auditChainVerified),
    leagueHealth: {
      [LEAGUE.business]: num(d.businessLeagueHealth),
      [LEAGUE.dev]: num(d.devLeagueHealth)
    },
    experts,
    businessExperts: byLeague(LEAGUE.business),
    devExperts: byLeague(LEAGUE.dev),
    recentVetoes: arr(d.recentVetoes).map(normVetoEvent)
  }
}

/** experts_status 端点的两段。
 *  ⚠️ 这两段的键是 snake_case（business_league / dev_league / average_health）：
 *  该 handler 出参是 serde_json::json! 字面量，字面量的键**不受** struct 上的
 *  #[serde(rename_all = "camelCase")] 影响 ⇒ 同一响应里数组元素（ExpertStatus）是 camelCase、
 *  外层壳是 snake_case。read 错一个字母不报错，只静默 undefined。 */
function normLeagueSection(raw, league) {
  const s = obj(raw)
  const experts = arr(s.experts).map((e) => normExpertState('', e))
  return {
    league,
    leagueLabel: leagueLabel(league),
    dimensions: arr(s.dimensions).map(str),
    experts,
    averageHealth: num(s.average_health)
  }
}

export function normExpertsStatus(raw) {
  const p = obj(raw)
  return {
    serverTsMs: wireTimeMs(p.timestamp),
    // 响应里 mox 键在 Rust 侧写了两遍（:580 与 :591），serde_json 后写覆盖前写 ⇒ 只余一个值，
    // 且两个值都不是判据。界面不读它。
    business: normLeagueSection(p.business_league, LEAGUE.business),
    dev: normLeagueSection(p.dev_league, LEAGUE.dev)
  }
}

/** 该端点的 experts 数组来自 states.get(dim) 的 filter_map ⇒ 缺维就是缺行，要显式点名缺了谁 */
export function missingDims(presentIds) {
  const have = new Set(arr(presentIds).map(str))
  return DIM_IDS.filter((id) => !have.has(id))
}

/** 分页壳的键名逐端点名：
 *  veto/events 出自 json! 字面量 ⇒ page_size / total_pages（snake）
 *  audit/logs 出自 AuditLogResponse struct ⇒ pageSize / totalPages（camel）
 *  同一个服务、相邻两个端点，两套口径。 */
function normPage(raw, { listKey, pageSizeKey, totalPagesKey }, rowFn) {
  const p = obj(raw)
  const rows = arr(p[listKey]).map(rowFn)
  return {
    rows,
    total: num(p.total),
    page: num(p.page, 1),
    pageSize: num(p[pageSizeKey]),
    totalPages: num(p[totalPagesKey]),
    // 后端 total_pages 由 div_ceil 算出；有数据却报 0 页说明 total 与列表不一致
    pageMetaConsistent: rows.length === 0 || num(p[totalPagesKey]) > 0
  }
}

export function normVetoPage(raw) {
  return normPage(raw, { listKey: 'events', pageSizeKey: 'page_size', totalPagesKey: 'total_pages' }, normVetoEvent)
}

export function normAuditEntry(raw) {
  const e = obj(raw)
  return {
    id: str(e.id),
    tsMs: wireTimeMs(e.ts),
    subject: str(e.subject),
    flowId: str(e.flowId),
    action: str(e.action),
    decision: str(e.decision),
    prevHash: str(e.prevHash),
    hash: str(e.hash)
  }
}

export function normAuditPage(raw) {
  return normPage(raw, { listKey: 'entries', pageSizeKey: 'pageSize', totalPagesKey: 'totalPages' }, normAuditEntry)
}

/** 权重 map → 按维度顺序的行；缺权重的维显式标 absent，不补 1.0。 */
function normWeightMap(raw, league) {
  const map = obj(raw)
  const order = league === LEAGUE.business ? BUSINESS_DIM_IDS : DEV_DIM_IDS
  return order.map((id) => ({
    dimension: id,
    dimLabel: dimLabel(id),
    league,
    present: Object.prototype.hasOwnProperty.call(map, id),
    weight: Number.isFinite(Number(map[id])) ? Number(map[id]) : NaN
  }))
}

export function normExpertConfig(raw) {
  const c = obj(raw)
  const th = obj(c.thresholds)
  return {
    version: num(c.version),
    updatedAtMs: wireTimeMs(c.updatedAt),
    updatedBy: str(c.updatedBy),
    businessWeights: normWeightMap(c.businessWeights, LEAGUE.business),
    devWeights: normWeightMap(c.devWeights, LEAGUE.dev),
    thresholds: {
      vetoThreshold: num(th.vetoThreshold, NaN),
      warnThreshold: num(th.warnThreshold, NaN),
      healthMin: num(th.healthMin, NaN)
    }
  }
}

export function normRbacConfig(raw) {
  const c = obj(raw)
  return {
    version: num(c.version),
    updatedAtMs: wireTimeMs(c.updatedAt),
    updatedBy: str(c.updatedBy),
    roles: arr(c.roles).map((r) => ({
      role: str(obj(r).role),
      permissions: arr(obj(r).permissions).map(str),
      description: str(obj(r).description)
    }))
  }
}

/** GovernanceReportSummary（POST /assess 的响应） */
export function normAssessSummary(raw) {
  const s = obj(raw)
  const scores = (map, league) => {
    const m = obj(map)
    const order = league === LEAGUE.business ? BUSINESS_DIM_IDS : DEV_DIM_IDS
    return order
      .filter((id) => Number.isFinite(Number(m[id])))
      .map((id) => ({ dimension: id, dimLabel: dimLabel(id), league, score: Number(m[id]), tone: healthTone(m[id]) }))
  }
  return {
    flowId: str(s.flowId),
    flowName: str(s.flowName),
    businessScores: scores(s.businessScores, LEAGUE.business),
    devScores: scores(s.devScores, LEAGUE.dev),
    businessLeagueScore: num(s.businessLeagueScore),
    devLeagueScore: num(s.devLeagueScore),
    gate: normGateResult(s.gateResult),
    adoptionCount: num(s.adoptionCount),
    suggestionCount: num(s.suggestionCount),
    moxPassed: bool(s.moxPassed),
    tsMs: wireTimeMs(s.ts)
  }
}
