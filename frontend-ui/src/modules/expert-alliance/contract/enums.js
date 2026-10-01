// 联盟枚举契约。每个集合都注明 Rust 权威源位置，前端只做投影与展示映射；
// 与源的一致性由 contract/contract.test.js 读取源码断言守护。

// alliance.rs:326-334（网关归一后的任务状态线名）
export const TASK_STATUS = Object.freeze({
  PENDING: 'pending',
  PLANNING: 'planning',
  RUNNING: 'running',
  PAUSED: 'paused',
  COMPLETED: 'completed',
  FAILED: 'failed',
  CANCELLED: 'cancelled'
})

// alliance.rs:52-61（NodeExecStatus）；proto NodeStatus 另有 ready
export const NODE_STATUS = Object.freeze({
  PENDING: 'pending',
  READY: 'ready',
  RUNNING: 'running',
  COMPLETED: 'completed',
  FAILED: 'failed',
  SKIPPED: 'skipped',
  CANCELLED: 'cancelled'
})

// alliance.rs:338-345
export const PRIORITY = Object.freeze({ LOW: 'low', NORMAL: 'normal', HIGH: 'high', CRITICAL: 'critical' })

// naming.rs:71-81 mode_serde —— **请求提交用这套**（serde snake_case）
export const MODE_WIRE = Object.freeze({
  SEQUENTIAL: 'sequential',
  PARALLEL: 'parallel',
  ITERATIVE: 'iterative',
  HIERARCHICAL: 'hierarchical',
  DEBATE: 'debate',
  VOTING: 'voting',
  DYNAMIC: 'dynamic'
})

// naming.rs:57-67 mode_display —— **响应展示用这套**，与 MODE_WIRE 不同名，不可互换
export const MODE_DISPLAY = Object.freeze({
  SEQUENTIAL: 'single_expert',
  PARALLEL: 'expert_alliance',
  ITERATIVE: 'human_in_loop',
  HIERARCHICAL: 'autonomous',
  DEBATE: 'debate',
  VOTING: 'voting',
  DYNAMIC: 'dynamic'
})

// common-proto/src/types.rs:159-178（serde snake_case 变体）
export const FUSION_STRATEGY = Object.freeze({
  VOTING: 'voting',
  WEIGHTED: 'weighted',
  CONFIDENCE_WEIGHTED: 'confidence_weighted',
  CONCATENATION: 'concatenation',
  BEST_OF: 'best_of',
  STACKING: 'stacking',
  DEBATE: 'debate',
  MAP_REDUCE: 'map_reduce',
  ITERATIVE: 'iterative'
})

// quality.rs:16-25
export const GRADE = Object.freeze({ A: 'A', B: 'B', C: 'C', D: 'D' })

// quality.rs GATE_THRESHOLDS：A≥0.90 B≥0.80 C≥0.70，其余 D（阻断）
export const GATE_THRESHOLDS = Object.freeze({ A: 0.9, B: 0.8, C: 0.7 })

export const TASK_STATUS_LABELS = Object.freeze({
  pending: '待执行',
  planning: '规划中',
  running: '执行中',
  paused: '已暂停',
  completed: '已完成',
  failed: '失败',
  cancelled: '已取消'
})

export const NODE_STATUS_LABELS = Object.freeze({
  pending: '待执行',
  ready: '就绪',
  running: '执行中',
  completed: '已完成',
  failed: '失败',
  skipped: '已跳过',
  cancelled: '已取消'
})

// 展示名按网关 mode_display 口径；标签同时接受 mode_serde 与 mode_display 两套线名
const MODE_LABEL_TABLE = Object.freeze({
  single_expert: '单专家',
  expert_alliance: '专家联盟',
  human_in_loop: '人机协同',
  autonomous: '自主执行',
  debate: '辩论',
  voting: '投票',
  dynamic: '动态组队',
  sequential: '单专家',
  parallel: '专家联盟',
  iterative: '人机协同',
  hierarchical: '自主执行'
})

// types.rs:186-198 fusion_label() 中文口径
export const FUSION_LABELS = Object.freeze({
  voting: '投票融合',
  weighted: '加权融合',
  confidence_weighted: '置信度加权融合',
  concatenation: '拼接融合',
  best_of: '择优融合',
  stacking: '堆叠融合（元学习器）',
  debate: '辩论融合',
  map_reduce: 'Map-Reduce 融合',
  iterative: '迭代融合'
})

export const GRADE_LABELS = Object.freeze({
  A: '优秀 · 优质交付',
  B: '良好 · 标准交付',
  C: '合格 · 有条件通过',
  D: '不合格 · 阻断'
})

// alliance.rs:361-366（专家后端状态 → 前端展示态）
const EXPERT_DISPLAY = Object.freeze({
  active: 'online',
  inactive: 'offline',
  maintenance: 'busy',
  deprecated: 'error'
})

// ===== 专家注册中心域枚举 =====
// 权威源：platform/gateway/mox-platform-gateway-svc/src/alliance/experts_common.rs 的文档注释
// （Rust 侧以 String 存储，取值清单只写在 /// 注释里，故 contract.test.js 直接解析注释比对）

// ExpertAvailability.status
export const EXPERT_AVAILABILITY = Object.freeze({
  ONLINE: 'online',
  BUSY: 'busy',
  OFFLINE: 'offline',
  AWAY: 'away'
})

// ExpertDescriptor.expert_type
export const EXPERT_TYPE = Object.freeze({ HUMAN: 'human', AI: 'ai', HYBRID: 'hybrid' })

// ExpertDescriptor.pricing_model
export const PRICING_MODEL = Object.freeze({ FREE: 'free', PAID: 'paid', SUBSCRIPTION: 'subscription' })

// ExpertDescriptor.verification_status
export const VERIFICATION_STATUS = Object.freeze({
  UNVERIFIED: 'unverified',
  VERIFIED: 'verified',
  CERTIFIED: 'certified'
})

// experts_ext.rs：Booking.status（pending 为创建默认值，cancelled/completed 为终态）
export const BOOKING_STATUS = Object.freeze({
  PENDING: 'pending',
  CONFIRMED: 'confirmed',
  COMPLETED: 'completed',
  CANCELLED: 'cancelled'
})

export const EXPERT_AVAILABILITY_LABELS = Object.freeze({
  online: '在线',
  busy: '忙碌',
  offline: '离线',
  away: '离开'
})

export const EXPERT_TYPE_LABELS = Object.freeze({
  human: '人类专家',
  ai: 'AI 专家',
  hybrid: '人机协同'
})

export const PRICING_MODEL_LABELS = Object.freeze({
  free: '免费',
  paid: '按次计费',
  subscription: '订阅制'
})

// 排序项文案：键必须与 endpoints.js 的 EXPERT_SORT（后端 list_experts 认识的 sort 值）一一对应
export const EXPERT_SORT_LABELS = Object.freeze({
  rating: '评分优先',
  consultations: '咨询量优先',
  name: '名称 A→Z'
})

export const VERIFICATION_LABELS = Object.freeze({
  unverified: '未认证',
  verified: '已认证',
  certified: '官方认证'
})

export const BOOKING_STATUS_LABELS = Object.freeze({
  pending: '待确认',
  confirmed: '已确认',
  completed: '已完成',
  cancelled: '已取消'
})

const TERMINAL_TASK = new Set([TASK_STATUS.COMPLETED, TASK_STATUS.FAILED, TASK_STATUS.CANCELLED])
const TERMINAL_NODE = new Set([NODE_STATUS.COMPLETED, NODE_STATUS.FAILED, NODE_STATUS.SKIPPED, NODE_STATUS.CANCELLED])
const ACTIVE_TASK = new Set([TASK_STATUS.PLANNING, TASK_STATUS.RUNNING])
// experts_ext.rs:209 明确拒绝 cancelled/completed 的取消请求
const CANCELLABLE_BOOKING = new Set([BOOKING_STATUS.PENDING, BOOKING_STATUS.CONFIRMED])

export function isTerminalTaskStatus(status) {
  return TERMINAL_TASK.has(status)
}

export function isActiveTaskStatus(status) {
  return ACTIVE_TASK.has(status)
}

export function isTerminalNodeStatus(status) {
  return TERMINAL_NODE.has(status)
}

/**
 * 置信度 → 门禁等级；null/undefined 视为未评。
 * @param {number|null|undefined} score 0..1
 * @returns {'A'|'B'|'C'|'D'|null}
 */
export function gradeOf(score) {
  if (score === null || score === undefined || score === '') return null
  const v = Number(score)
  if (!Number.isFinite(v)) return null
  if (v >= GATE_THRESHOLDS.A) return GRADE.A
  if (v >= GATE_THRESHOLDS.B) return GRADE.B
  if (v >= GATE_THRESHOLDS.C) return GRADE.C
  return GRADE.D
}

export function expertDisplayStatus(raw) {
  return EXPERT_DISPLAY[raw] ?? raw ?? 'offline'
}

export function taskStatusLabel(status) {
  return TASK_STATUS_LABELS[status] ?? status ?? ''
}

export function nodeStatusLabel(status) {
  return NODE_STATUS_LABELS[status] ?? status ?? ''
}

export function modeLabel(mode) {
  return MODE_LABEL_TABLE[mode] ?? mode ?? ''
}

/**
 * mode_serde 传输名 → mode_display 展示名；已是展示名时原样返回。
 */
export function modeToDisplay(mode) {
  const entry = Object.entries(MODE_WIRE).find(([, v]) => v === mode)
  return entry ? MODE_DISPLAY[entry[0]] : mode
}

// 标签表统一口径：命中返回中文，未命中回原值（后端新增枚举时页面不至于空白）
const Labeled = (key, table) => table[key] ?? key ?? ''

export function fusionLabel(strategy) {
  return Labeled(strategy, FUSION_LABELS)
}

export function gradeLabel(grade) {
  return Labeled(grade, GRADE_LABELS)
}

export function availabilityLabel(status) {
  return Labeled(status, EXPERT_AVAILABILITY_LABELS)
}

export function expertTypeLabel(type) {
  return Labeled(type, EXPERT_TYPE_LABELS)
}

export function pricingLabel(model) {
  return Labeled(model, PRICING_MODEL_LABELS)
}

export function sortLabel(sort) {
  return Labeled(sort, EXPERT_SORT_LABELS)
}

/**
 * 计费文案：free 与未设置都归为「免费」（后端 default_pricing 即 free）；计费型但费率缺失/为 0 时不谎报价格。
 * hourly_rate_cents 单位为分，取自 experts_common.rs 的 ExpertDescriptor.hourly_rate_cents。
 */
export function pricingText(model, hourlyRateCents) {
  if (!model || model === PRICING_MODEL.FREE) return PRICING_MODEL_LABELS[PRICING_MODEL.FREE]
  const cents = Number(hourlyRateCents)
  if (!Number.isFinite(cents) || cents <= 0) return `${pricingLabel(model)} · 未设价`
  return `${pricingLabel(model)} · ${Math.round(cents / 100)} 元/时`
}

export function verificationLabel(status) {
  return Labeled(status, VERIFICATION_LABELS)
}

export function bookingStatusLabel(status) {
  return Labeled(status, BOOKING_STATUS_LABELS)
}

/** 专家是否可直接即时咨询（experts_registry.rs:754 只有 online 才创建会话） */
export function isConsultable(status) {
  return status === EXPERT_AVAILABILITY.ONLINE
}

/** 预约是否可取消，与后端 experts_ext.rs:208-211 的判据一致 */
export function canCancelBooking(status) {
  return CANCELLABLE_BOOKING.has(status)
}

/**
 * 平台统计 KPI 文案。入参为 normExpertStats 的输出（键即 /api/experts/stats 的真实响应键）。
 * 计数型字段 0 是事实（如今日 0 次），照实显示；评分/响应在无样本时后端给 0，属默认零值，显示「—」而不谎报。
 */
export function expertStatsCells(stats) {
  const s = stats || {}
  const int = (v, suffix = '') => {
    const n = Number(v)
    return Number.isFinite(n) ? `${n}${suffix}` : '—'
  }
  const measured = (v, suffix = '', digits) => {
    const n = Number(v)
    if (!Number.isFinite(n) || n <= 0) return '—'
    return `${digits === undefined ? n : n.toFixed(digits)}${suffix}`
  }
  return [
    { label: '平台专家', value: int(s.totalExperts, ' 位') },
    { label: '在线 / 忙碌 / 离线', value: stats ? `${int(s.onlineExperts)} / ${int(s.busyExperts)} / ${int(s.offlineExperts)}` : '—' },
    { label: '累计咨询', value: int(s.totalConsultations, ' 次') },
    { label: '今日咨询', value: int(s.todayConsultations, ' 次') },
    { label: '平均评分', value: measured(s.avgRating, '', 1) },
    { label: '平均响应', value: measured(s.avgResponseMinutes, ' 分钟') }
  ]
}

/**
 * 单专家派生指标文案。入参为 normExpertMetrics 输出的 derived 三值，
 * 全部由后端算（experts_registry.rs:569-585），界面不得声称是本地排名。
 * 权重是写死的 评分 40% + 解决率 30% + 空闲度 30%；load_ratio 后端不夹上限
 * （current_load 超 max_concurrent 时会 > 1），只有 efficiency 里才 .min(1.0)。
 */
export function expertDerivedCells(derived) {
  const d = derived || {}
  const n = (v) => (Number.isFinite(Number(v)) && v !== null && v !== '' ? Number(v) : null)
  const percent = (v, digits = 0) => (n(v) === null ? '—' : `${(n(v) * 100).toFixed(digits)}%`)
  return [
    { label: '评分百分位', value: n(d.rankPercentile) === null ? '—' : `${n(d.rankPercentile).toFixed(1)}%`, note: '已启用专家里评分低于自己的人数占比（严格小于，同分不计入）' },
    { label: '当前负载率', value: percent(d.loadRatio), note: 'current_load / max_concurrent，可超过 100%；后端对无上限的专家给 0' },
    { label: '效率分', value: percent(d.efficiencyScore, 1), note: '评分 40% + 解决率 30% + 空闲度 30%，由后端算' }
  ]
}

/**
 * 调度器熔断器状态。wire 权威：
 * platform/gateway/mox-platform-gateway-svc/src/alliance/experts_dispatcher.rs:494-509
 * —— 只有 failure_count > 0 的专家进表，state 由 count >= circuit_breaker_threshold 二分为 open/closed；
 * half_open 只存在于 scheduler-core 的 llm_router 内部配置（:120 half_open_probes），不在本字段值域内。
 */
export const BREAKER_STATE = Object.freeze({ OPEN: 'open', CLOSED: 'closed' })

export const BREAKER_STATE_LABELS = Object.freeze({
  open: '已熔断',
  closed: '正常'
})

export function breakerStateLabel(state) {
  return Labeled(state, BREAKER_STATE_LABELS)
}

/**
 * 任务融合结果状态（字段 fusion_status）。wire 权威：
 * platform/domains/alliance/sdk/mox-alliance-http-sdk/src/alliance.rs:544（completed|partial）、
 * :1235（completed|pending）、alliance_remote.rs:969（pending）
 * —— 三档集合 {pending, partial, completed}，与 TASK_STATUS/NODE_STATUS 只是同串不同域。
 */
export const FUSION_STATUS = Object.freeze({ PENDING: 'pending', PARTIAL: 'partial', COMPLETED: 'completed' })
