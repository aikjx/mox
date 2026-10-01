// 后端响应 → 前端视图模型的唯一规范化层。字段名逐一取自 Rust handler 实测，不得凭想象加字段。
import {
  BOOKING_STATUS,
  EXPERT_AVAILABILITY,
  canCancelBooking,
  expertDisplayStatus,
  gradeOf,
  isTerminalNodeStatus,
  modeToDisplay,
  TASK_STATUS,
  NODE_STATUS
} from '@/modules/expert-alliance/contract'

import { COLLAB_MODE } from '@/modules/expert-alliance/contract'

const num = (v, d = 0) => (Number.isFinite(Number(v)) ? Number(v) : d)
const str = (v) => (v === undefined || v === null ? '' : String(v))
const arr = (v) => (Array.isArray(v) ? v : [])
const bool = (v) => v === true || v === 'true' || v === 1

export function normRuntime(raw) {
  const r = raw || {}
  return {
    executionReady: bool(r.execution_ready),
    schedulerReady: bool(r.scheduler_ready),
    executorReady: bool(r.executor_ready),
    mode: str(r.mode) || 'local_preview',
    message: str(r.message),
    // local_preview 下任务 DAG 由网关按模式模拟生成，progress/duration 非真实执行值
    simulated: str(r.mode) !== 'remote'
  }
}

export function normTask(raw) {
  const t = raw || {}
  return {
    id: str(t.task_id),
    title: str(t.title),
    description: str(t.description),
    status: str(t.status) || TASK_STATUS.PENDING,
    priority: str(t.priority),
    progress: num(t.progress),
    mode: str(t.mode),
    modeDisplay: modeToDisplay(str(t.mode)),
    createdAt: str(t.created_at),
    startedAt: str(t.started_at),
    completedAt: str(t.completed_at),
    durationMs: t.duration_ms === undefined || t.duration_ms === null ? null : num(t.duration_ms)
  }
}

export function normTaskList(payload) {
  return {
    items: arr(payload?.tasks).map(normTask),
    total: num(payload?.total),
    page: num(payload?.page, 1),
    pageSize: num(payload?.page_size, 20)
  }
}

export function normTaskCreate(raw, params = {}) {
  const r = raw || {}
  // 创建响应只回显 task_id/title/status/created_at，mode/priority 需从 params 取回
  const merged = {
    ...r,
    mode: r.mode ?? params.mode,
    priority: r.priority ?? params.priority,
    description: r.description ?? params.description
  }
  return {
    ...normTask(merged),
    taskType: str(r.task_type ?? params.task_type ?? 'general')
  }
}

export function normNode(raw) {
  const n = raw || {}
  return {
    id: str(n.node_id ?? n.id),
    name: str(n.name ?? n.label),
    label: str(n.label ?? n.name),
    expertId: str(n.expert_id),
    status: str(n.status) || NODE_STATUS.PENDING,
    dependencies: arr(n.dependencies).map(str),
    progress: n.progress === undefined ? null : num(n.progress),
    startedAt: str(n.started_at),
    completedAt: str(n.completed_at),
    durationMs: n.duration_ms === undefined || n.duration_ms === null ? null : num(n.duration_ms),
    errorMessage: str(n.error_message),
    position: n.position ? { x: num(n.position.x), y: num(n.position.y) } : null,
    terminal: isTerminalNodeStatus(str(n.status) || NODE_STATUS.PENDING)
  }
}

export function normNodeList(payload) {
  return { items: arr(payload?.nodes).map(normNode), total: num(payload?.total) }
}

export function normDag(payload) {
  return {
    nodes: arr(payload?.nodes).map(normNode),
    edges: arr(payload?.edges).map((e) => ({ source: str(e.source), target: str(e.target), label: str(e.label) })),
    stats: {
      total: num(payload?.stats?.total),
      completed: num(payload?.stats?.completed),
      running: num(payload?.stats?.running),
      pending: num(payload?.stats?.pending),
      failed: num(payload?.stats?.failed),
      skipped: num(payload?.stats?.skipped)
    }
  }
}

// logs/stream 只发默认 message 事件，data 即裸 LogEntry JSON
export function normLogEntry(raw) {
  const l = raw || {}
  return {
    seq: num(l.seq),
    ts: str(l.ts),
    level: str(l.level) || 'info',
    nodeId: str(l.node_id),
    message: str(l.message)
  }
}

export function normLogList(payload) {
  return {
    taskId: str(payload?.task_id),
    items: arr(payload?.logs).map(normLogEntry),
    total: num(payload?.total)
  }
}

export function normExecutionStatus(payload) {
  const p = payload || {}
  return {
    taskId: str(p.task_id),
    status: str(p.status),
    progress: num(p.progress),
    counts: {
      total: num(p.total_nodes),
      completed: num(p.completed_nodes),
      running: num(p.running_nodes),
      failed: num(p.failed_nodes),
      pending: num(p.pending_nodes),
      // alliance.rs:954 该字段固定 0，不可作为业务判据
      skipped: num(p.skipped_nodes),
      cancelled: num(p.cancelled_nodes)
    }
  }
}

export function normPlan(payload) {
  return {
    phases: arr(payload?.phases).map((ph) => ({
      phaseId: str(ph.phase_id),
      name: str(ph.name),
      expertId: str(ph.expert_id),
      status: str(ph.status),
      durationMs: num(ph.duration_ms),
      progress: num(ph.progress),
      dependencies: arr(ph.dependencies).map(str)
    })),
    totalPhases: num(payload?.total_phases),
    assignedExperts: arr(payload?.assigned_experts).map(str),
    source: str(payload?.source)
  }
}

const normContribution = (c) => ({
  nodeId: str(c?.node_id),
  expert: str(c?.expert),
  weight: num(c?.weight),
  contribution: str(c?.contribution)
})

export function normFusion(payload) {
  const p = payload || {}
  const body = p.fusion_result || p.result || {}
  const confidence = body.confidence === undefined || body.confidence === null ? null : num(body.confidence)
  return {
    taskId: str(p.task_id),
    status: str(p.status),
    fusionStatus: str(p.fusion_status),
    strategy: str(p.fusion_strategy),
    participatingNodes: arr(p.participating_nodes).map(str),
    summary: str(body.summary),
    confidence,
    grade: gradeOf(confidence),
    keyFindings: arr(body.key_findings).map(str),
    recommendations: arr(body.recommendations).map(str),
    expertContributions: arr(p.expert_contributions).map(normContribution),
    nodeContributions: arr(p.node_contributions).map(normContribution),
    fusedAt: str(p.fused_at)
  }
}

export function normAction(payload) {
  const p = payload || {}
  return { success: bool(p.success), message: str(p.message) }
}

export function normExpertSearch(payload) {
  return {
    items: arr(payload?.experts).map((e) => ({
      id: str(e.expert_id ?? e.id),
      name: str(e.name),
      description: str(e.description),
      domains: arr(e.domains).map(str),
      status: str(e.status),
      matchScore: num(e.match_score)
    })),
    total: num(payload?.total)
  }
}

export function normExpert(raw) {
  const e = raw || {}
  const av = isObject(e.availability) ? e.availability : {}
  const status = str(av.status)
  const metrics = isObject(e.metrics) ? e.metrics : {}
  const currentLoad = num(av.current_load)
  const maxConcurrent = num(av.max_concurrent)
  return {
    id: str(e.id ?? e.expert_id),
    name: str(e.name),
    avatar: str(e.avatar),
    title: str(e.title),
    organization: str(e.organization),
    bio: str(e.bio),
    domains: arr(e.domains).map(str),
    skills: arr(e.skills).map(str),
    tags: arr(e.tags).map(str),
    languages: arr(e.languages).map(str),
    // capabilities 在 Rust 侧是结构体数组（ExpertCapability），不是字符串
    capabilities: arr(e.capabilities).map(normCapability),
    availability: {
      status,
      lastActive: str(av.last_active),
      avgResponseMinutes: num(av.avg_response_minutes),
      currentLoad,
      maxConcurrent,
      // max_concurrent 为 0 时后端未设上限，负载率不可当作百分比展示
      loadRatio: maxConcurrent > 0 ? currentLoad / maxConcurrent : null
    },
    status: expertDisplayStatus(status) || EXPERT_AVAILABILITY.OFFLINE,
    online: status === EXPERT_AVAILABILITY.ONLINE,
    expertType: str(e.expert_type ?? e.type),
    pricingModel: str(e.pricing_model),
    hourlyRateCents: num(e.hourly_rate_cents),
    verificationStatus: str(e.verification_status),
    timezone: str(e.timezone),
    enabled: e.enabled === undefined ? true : bool(e.enabled),
    metrics: {
      totalConsultations: num(metrics.total_consultations),
      todayConsultations: num(metrics.today_consultations),
      avgRating: num(metrics.avg_rating),
      ratingCount: num(metrics.rating_count),
      resolutionRate: num(metrics.resolution_rate),
      firstResponseAccuracy: num(metrics.first_response_accuracy),
      totalServiceMinutes: num(metrics.total_service_minutes)
    },
    createdAt: str(e.created_at),
    updatedAt: str(e.updated_at)
  }
}

/**
 * 三个写端点的响应（POST / PUT / DELETE /api/experts/:id）。
 * 各自的标记键不同源：created / updated / deleted+soft_delete，这里只照抄后端给出的，
 * 缺失即 false——写操作是否生效由这些标记判定，不按 HTTP 200 判定。
 */
export function normExpertWrite(raw) {
  const d = isObject(raw) ? raw : {}
  return {
    expert: d.expert ? normExpert(d.expert) : null,
    id: str(d.id),
    created: bool(d.created),
    updated: bool(d.updated),
    deleted: bool(d.deleted),
    softDelete: bool(d.soft_delete),
    message: str(d.message)
  }
}

/**
 * GET /api/experts/stats —— 平台级统计。字段清单以 experts_registry.rs 的
 * experts_stats_real 里 `ok(json!({...}))` 为准，由 contract.test.js 逐键比对；
 * 存量广场页读的 expert_count/consult_count/good_rate/avg_response 在后端并不存在。
 */
export function normExpertStats(raw) {
  const s = raw || {}
  const domains = isObject(s.domains) ? s.domains : {}
  return {
    totalExperts: num(s.total_experts),
    onlineExperts: num(s.online_experts),
    busyExperts: num(s.busy_experts),
    offlineExperts: num(s.offline_experts),
    totalConsultations: num(s.total_consultations),
    todayConsultations: num(s.today_consultations),
    avgRating: num(s.avg_rating),
    avgResponseMinutes: num(s.avg_response_minutes),
    satisfactionRate: num(s.satisfaction_rate),
    domains: Object.entries(domains)
      .map(([name, count]) => ({ name, count: num(count) }))
      .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name, 'zh-CN')),
    ts: str(s.ts)
  }
}

function isObject(v) {
  return !!v && typeof v === 'object' && !Array.isArray(v)
}

/**
 * GET /api/experts/capabilities —— 平台能力目录（按 capability id 归并 enabled 专家）。
 * 后端已按 id 升序输出（experts_registry.rs:467-469），前端不再重排，
 * 否则页面上的顺序就不是后端承诺的那一个。
 */
export function normCapabilities(raw) {
  const s = raw || {}
  return {
    items: arr(s.capabilities).map((c) => ({
      id: str(c?.id),
      name: str(c?.name),
      domain: str(c?.domain),
      expertCount: num(c?.expert_count),
      avgProficiency: num(c?.avg_proficiency)
    })),
    total: num(s.total),
    domains: arr(s.domains).map(str)
  }
}

/**
 * GET /api/experts/:id/metrics —— 单专家指标 + 后端派生三值。
 * metrics/availability 与详情接口是同一批结构体，故复用 normExpert 的字段映射，不再抄一遍键表。
 * 注意 derived.load_ratio 在 max_concurrent=0 时后端给 0，而 availability.loadRatio 给 null（不可当百分比）。
 */
export function normExpertMetrics(raw) {
  const s = raw || {}
  const d = isObject(s.derived) ? s.derived : {}
  const e = normExpert({ id: s.expert_id, metrics: s.metrics, availability: s.availability })
  return {
    expertId: e.id,
    metrics: e.metrics,
    availability: e.availability,
    derived: {
      rankPercentile: num(d.rank_percentile),
      loadRatio: num(d.load_ratio),
      efficiencyScore: num(d.efficiency_score)
    }
  }
}

function normCapability(c) {
  const x = c || {}
  return {
    id: str(x.id),
    name: str(x.name),
    domain: str(x.domain),
    proficiency: num(x.proficiency),
    description: str(x.description)
  }
}

export function normExpertList(payload) {
  return {
    items: arr(payload?.experts).map(normExpert),
    total: num(payload?.total),
    page: num(payload?.page, 1),
    pageSize: num(payload?.page_size, 20)
  }
}

// ===== 预约 / 收藏 / 即时咨询 / 咨询室 / 团队 =====
// 字段逐一取自 experts_ext.rs 的 Booking 与 ok(json!{...}) 字面量

export function normBooking(raw) {
  const b = raw || {}
  return {
    id: str(b.id),
    expertId: str(b.expert_id),
    expertName: str(b.expert_name),
    userId: str(b.user_id),
    topic: str(b.topic),
    scheduledAt: str(b.scheduled_at),
    durationMinutes: num(b.duration_minutes),
    status: str(b.status) || BOOKING_STATUS.PENDING,
    createdAt: str(b.created_at),
    cancellable: canCancelBooking(str(b.status) || BOOKING_STATUS.PENDING)
  }
}

export function normBookingList(payload) {
  const p = payload || {}
  return {
    items: arr(p.bookings).map(normBooking),
    total: num(p.total),
    counts: {
      pending: num(p.pending),
      confirmed: num(p.confirmed),
      completed: num(p.completed),
      cancelled: num(p.cancelled)
    }
  }
}

// PUT /experts/bookings/:id/cancel 只回 booking_id/status/cancelled_at/message
export function normBookingCancel(payload) {
  const p = payload || {}
  return {
    bookingId: str(p.booking_id),
    status: str(p.status) || BOOKING_STATUS.CANCELLED,
    cancelledAt: str(p.cancelled_at),
    message: str(p.message)
  }
}

export function normFavorite(payload) {
  const p = payload || {}
  return {
    expertId: str(p.expert_id),
    favorite: bool(p.favorite),
    action: str(p.action),
    updatedAt: str(p.updated_at)
  }
}

// consult_now 双分支都返回 200：专家不在线时 session_id 为 null，
// 前端必须以 sessionId 判空，不能把 status==='unavailable' 当成功。
export function normConsultNow(payload) {
  const p = payload || {}
  return {
    expertId: str(p.expert_id),
    sessionId: p.session_id === undefined || p.session_id === null ? null : str(p.session_id),
    status: str(p.status),
    channel: str(p.channel),
    topic: str(p.topic),
    question: p.question === undefined || p.question === null ? null : str(p.question),
    expertOnline: bool(p.expert_online),
    chatUrl: p.chat_url === undefined || p.chat_url === null ? null : str(p.chat_url),
    createdAt: str(p.created_at),
    message: str(p.message)
  }
}

export function normConsultRoom(payload) {
  const p = payload || {}
  const info = isObject(p.expert_info) ? p.expert_info : null
  return {
    bookingId: str(p.booking_id),
    roomId: str(p.room_id),
    roomToken: str(p.room_token),
    joinUrl: str(p.join_url),
    iceServers: arr(p.webrtc_config?.ice_servers).map((i) => str(i.urls)),
    expertInfo: info
      ? { id: str(info.id), name: str(info.name), title: str(info.title), avatar: str(info.avatar), online: bool(info.online) }
      : null,
    status: str(p.status),
    expiresIn: num(p.expires_in),
    createdAt: str(p.created_at)
  }
}

export function normTeamApplication(payload) {
  const p = payload || {}
  return {
    applicationId: str(p.application_id),
    // approved（专家已验证，自动入队）/ pending_approval（等管理员）
    status: str(p.status),
    teamId: str(p.team_id),
    expertId: str(p.expert_id),
    role: str(p.role),
    appliedAt: str(p.applied_at),
    estimatedReviewHours: num(p.estimated_review_hours),
    message: str(p.message)
  }
}

// ── 会话面：experts_session.rs 九个 handler 的出参归一化 ────────────────
// 两种形态必须一份归一化器同时吃下：
//   list_sessions → session_to_list_view(:113) 剥掉 messages、换成 message_count；
//   create/get/update → ok(json!(session)) 直接序列化 ExpertSession 结构体，带 messages 而无 message_count。
// 归一化后 messageCount 恒有值，messages 恒为数组（列表形态下是空数组，不是"没有"）。

export function normSessionMessage(raw) {
  const m = isObject(raw) ? raw : {}
  return {
    id: str(m.id),
    role: str(m.role),
    senderId: str(m.sender_id),
    senderName: str(m.sender_name),
    content: str(m.content),
    msgType: str(m.msg_type),
    attachments: arr(m.attachments),
    // 后端是 Option<u8>：缺失与 null 都归 null，界面才能区分"没评分"和"0 分"
    rating: m.rating === undefined || m.rating === null ? null : num(m.rating),
    createdAt: str(m.created_at)
  }
}

export function normSession(raw) {
  const s = isObject(raw) ? raw : {}
  const messages = arr(s.messages).map(normSessionMessage)
  const meta = isObject(s.metadata) ? s.metadata : {}
  return {
    id: str(s.id),
    title: str(s.title),
    expertIds: arr(s.expert_ids).map(str),
    userId: str(s.user_id),
    sessionType: str(s.session_type),
    status: str(s.status),
    topic: str(s.topic),
    messageCount: s.message_count === undefined || s.message_count === null ? messages.length : num(s.message_count),
    tags: arr(s.tags).map(str),
    metadata: Object.keys(meta).map((key) => ({ key, value: meta[key] })),
    messages,
    createdAt: str(s.created_at),
    lastActiveAt: str(s.last_active_at),
    archivedAt: str(s.archived_at),
    archived: !!s.archived_at
  }
}

export function normSessionList(payload) {
  const p = isObject(payload) ? payload : {}
  return {
    items: arr(p.sessions).map(normSession),
    total: num(p.total),
    page: num(p.page, 1),
    pageSize: num(p.page_size, 20)
  }
}

/** session_stats(:262)：avg_* 在无数据时后端返回 0.0，归一化不把它伪装成 null */
export function normSessionStats(payload) {
  const p = isObject(payload) ? payload : {}
  const dist = isObject(p.session_type_distribution) ? p.session_type_distribution : {}
  return {
    totalSessions: num(p.total_sessions),
    activeSessions: num(p.active_sessions),
    archivedSessions: num(p.archived_sessions),
    closedSessions: num(p.closed_sessions),
    totalMessages: num(p.total_messages),
    avgMessagesPerSession: num(p.avg_messages_per_session),
    avgSessionDurationMinutes: num(p.avg_session_duration_minutes),
    sessionsToday: num(p.sessions_today),
    topExpertsBySessions: arr(p.top_experts_by_sessions).map((e) => ({ expertId: str(e.expert_id), count: num(e.count) })),
    typeDistribution: Object.keys(dist).map((type) => ({ type, count: num(dist[type]) })),
    ts: str(p.ts)
  }
}

/** 会话内字面相似检索：rank 由后端按分数降序从 1 起编号（:487-496） */
export function normSimilarSearch(payload) {
  const p = isObject(payload) ? payload : {}
  return {
    query: str(p.query),
    sessionId: str(p.session_id),
    results: arr(p.results).map((r) => ({
      message: normSessionMessage(r.message),
      similarityScore: num(r.similarity_score),
      rank: num(r.rank)
    })),
    totalFound: num(p.total_found)
  }
}

/** 全域检索：结果自带所属会话 id 与标题，后端不回显过滤条件，故此处不发明字段 */
export function normSemanticSearch(payload) {
  const p = isObject(payload) ? payload : {}
  return {
    query: str(p.query),
    results: arr(p.results).map((r) => ({
      sessionId: str(r.session_id),
      sessionTitle: str(r.session_title),
      message: normSessionMessage(r.message),
      similarityScore: num(r.similarity_score)
    })),
    totalSessionsScanned: num(p.total_sessions_scanned),
    totalMessagesScanned: num(p.total_messages_scanned)
  }
}

/** 导出：download_url 恒 null(:586)，downloadUrl 保留 null 就是为了不让界面画出假链接 */
export function normSessionExport(payload) {
  const p = isObject(payload) ? payload : {}
  return {
    sessionId: str(p.session_id),
    format: str(p.format),
    exportedAt: str(p.exported_at),
    content: normSession(p.content),
    downloadUrl: p.download_url === undefined || p.download_url === null ? null : str(p.download_url),
    messageCount: num(p.message_count)
  }
}

export function normSessionArchive(payload) {
  const p = isObject(payload) ? payload : {}
  return {
    sessionId: str(p.session_id),
    status: str(p.status),
    archivedAt: str(p.archived_at),
    messageCount: num(p.message_count)
  }
}

export function normSessionDelete(payload) {
  const p = isObject(payload) ? payload : {}
  return { deleted: bool(p.deleted), sessionId: str(p.session_id) }
}

export function normDispatcherStatus(payload) {
  const p = payload || {}
  return {
    engineStatus: str(p.engine_status),
    currentStrategy: str(p.current_strategy),
    activeDispatches: num(p.active_dispatches),
    totalDispatches: num(p.total_dispatches),
    successRate: num(p.success_rate),
    avgDispatchMs: num(p.avg_dispatch_ms),
    circuitBreakers: arr(p.circuit_breakers).map((c) => ({
      expertId: str(c.expert_id),
      failureCount: num(c.failure_count),
      state: str(c.state)
    })),
    expertLoads: arr(p.expert_loads).map((l) => ({
      expertId: str(l.expert_id),
      currentLoad: num(l.current_load),
      maxConcurrent: num(l.max_concurrent),
      loadRatio: num(l.load_ratio)
    })),
    lastDispatchAt: str(p.last_dispatch_at),
    serverTs: str(p.ts)
  }
}

/**
 * POST /dispatcher/reset/:id 的回执（experts_dispatcher.rs:769-808）。
 * 后端对未知 id 也返回 reset:true 且 previous_load 走 unwrap_or(0)，所以这里
 * 不派生任何"存在性"字段：界面只能用 previousLoad 与 resetAt 说话。
 */
export function normDispatcherReset(payload) {
  const p = isObject(payload) ? payload : {}
  return {
    expertId: str(p.expert_id),
    reset: bool(p.reset),
    previousLoad: num(p.previous_load),
    previousFailures: num(p.previous_failures),
    resetAt: str(p.reset_at),
    // reason 是 Option<String>：不发即为 null，与"发了空串"在后端不区分，故归一成空串
    reason: str(p.reason)
  }
}

/** POST /dispatcher/reset-all 的回执（:836-840）—— 只有计数与 id 清单，没有逐项旧值 */
export function normDispatcherResetAll(payload) {
  const p = isObject(payload) ? payload : {}
  return {
    resetCount: num(p.reset_count),
    resetExpertIds: arr(p.reset_expert_ids).map((v) => String(v ?? '')),
    resetAt: str(p.reset_at)
  }
}

// ── 协作图谱：experts_graph.rs 八个 handler 的出参归一化 ─────────────────
// 图的真实构成来自 experts_common.rs 的 build_graph_from_registry：
// 节点只有 expert / domain 两类，边只有 has_domain / collaborates_with 两类，
// 后者的 properties 携带 shared_domains 与 similarity。
// 注意：图谱节点与任务 DAG 节点同名不同物，不可复用 normNode。

export function normGraphNode(raw) {
  const n = isObject(raw) ? raw : {}
  const p = isObject(n.properties) ? n.properties : {}
  return {
    id: str(n.id),
    label: str(n.label),
    nodeType: str(n.node_type),
    title: str(p.title),
    domains: arr(p.domains).map(str),
    avgRating: p.avg_rating === undefined || p.avg_rating === null ? null : num(p.avg_rating),
    availability: str(p.status)
  }
}

function normGraphEdge(raw) {
  const e = isObject(raw) ? raw : {}
  const p = isObject(e.properties) ? e.properties : {}
  return {
    source: str(e.source),
    target: str(e.target),
    edgeType: str(e.edge_type),
    weight: num(e.weight),
    sharedDomains: arr(p.shared_domains).map(str)
  }
}

export function normGraph(payload) {
  const p = payload || {}
  return {
    nodes: arr(p.nodes).map(normGraphNode),
    edges: arr(p.edges).map(normGraphEdge),
    stats: {
      nodeCount: num(p.stats?.node_count),
      edgeCount: num(p.stats?.edge_count),
      expertCount: num(p.stats?.expert_count),
      domainCount: num(p.stats?.domain_count),
      avgDegree: num(p.stats?.avg_degree),
      density: num(p.stats?.density)
    },
    builtAt: str(p.built_at),
    version: num(p.version)
  }
}

export function normGraphStats(payload) {
  const p = payload || {}
  return {
    totalNodes: num(p.total_nodes),
    totalEdges: num(p.total_edges),
    expertNodes: num(p.expert_nodes),
    domainNodes: num(p.domain_nodes),
    collaborationEdges: num(p.collaboration_edges),
    domainEdges: num(p.domain_edges),
    avgClusteringCoefficient: num(p.avg_clustering_coefficient),
    connectedComponents: num(p.connected_components),
    largestComponentSize: num(p.largest_component_size),
    topCentralityExperts: arr(p.top_centrality_experts).map((e) => ({
      id: str(e.id),
      name: str(e.name),
      degree: num(e.degree),
      degreeCentrality: num(e.degree_centrality),
      betweenness: num(e.betweenness)
    })),
    density: num(p.density),
    ts: str(p.ts)
  }
}

export function normNeighbors(payload) {
  const p = payload || {}
  return {
    nodeId: str(p.node_id),
    nodeLabel: str(p.node_label),
    nodeType: str(p.node_type),
    neighbors: arr(p.neighbors).map((n) => ({
      id: str(n.id),
      label: str(n.label),
      nodeType: str(n.node_type),
      edgeType: str(n.edge_type),
      weight: num(n.weight),
      // out = 该节点是边起点，in = 是终点；builder 的协作边是无向的
      direction: str(n.direction),
      sharedDomains: arr(n.properties?.shared_domains).map(str)
    })),
    neighborCount: num(p.neighbor_count)
  }
}

export function normCollaborators(payload) {
  const p = payload || {}
  return {
    expertId: str(p.expert_id),
    collaborators: arr(p.collaborators).map((c) => ({
      rank: num(c.collaboration_rank),
      id: str(c.id),
      name: str(c.name),
      collaborationWeight: num(c.collaboration_weight),
      sharedDomains: arr(c.shared_domains).map(str)
    })),
    // total_collaborators 是截断前的全量数，与 collaborators.length 不等即说明被 limit 截了
    totalCollaborators: num(p.total_collaborators)
  }
}

export function normGraphPath(payload) {
  const p = payload || {}
  return {
    source: str(p.source),
    target: str(p.target),
    path: arr(p.path).map((n) => ({
      nodeId: str(n.node_id),
      label: str(n.label),
      nodeType: str(n.node_type)
    })),
    pathLength: num(p.path_length),
    totalWeight: num(p.total_weight),
    found: bool(p.found)
  }
}

export function normCommunities(payload) {
  const p = payload || {}
  return {
    communities: arr(p.communities).map((c) => ({
      communityId: str(c.community_id),
      size: num(c.size),
      memberIds: arr(c.member_ids).map(str),
      memberLabels: arr(c.member_labels).map(str),
      internalEdges: num(c.internal_edges),
      externalEdges: num(c.external_edges),
      modularityContribution: num(c.modularity_contribution)
    })),
    totalCommunities: num(p.total_communities),
    modularity: num(p.modularity),
    // 后端硬编码回传算法名，界面据此说明"社区"是标签传播的结果而非人工分组
    algorithm: str(p.algorithm),
    iterations: num(p.iterations),
    converged: bool(p.converged)
  }
}

export function normGraphRebuild(payload) {
  const p = payload || {}
  return {
    rebuilt: bool(p.rebuilt),
    previousVersion: num(p.previous_version),
    newVersion: num(p.new_version),
    nodeCount: num(p.node_count),
    edgeCount: num(p.edge_count),
    expertCount: num(p.expert_count),
    builtAt: str(p.built_at),
    durationMs: num(p.duration_ms)
  }
}

export function normOptimalTeam(payload) {
  const p = payload || {}
  return {
    teamId: str(p.team_id),
    requiredSkills: arr(p.required_skills).map(str),
    requiredDomains: arr(p.required_domains).map(str),
    members: arr(p.team_members).map((m) => ({
      id: str(m.id),
      name: str(m.name),
      title: str(m.title),
      role: str(m.role),
      coveredSkills: arr(m.covered_skills).map(str),
      coveredDomains: arr(m.covered_domains).map(str),
      matchScore: num(m.match_score),
      avgRating: num(m.avg_rating)
    })),
    coverage: {
      requiredTotal: num(p.coverage?.required_total),
      coveredCount: num(p.coverage?.covered_count),
      coverageRatio: num(p.coverage?.coverage_ratio),
      missingSkills: arr(p.coverage?.missing_skills).map(str),
      missingDomains: arr(p.coverage?.missing_domains).map(str)
    },
    teamScore: num(p.team_score),
    strategy: str(p.selection_strategy),
    createdAt: str(p.created_at)
  }
}

// ── 智能协作：experts_collaboration.rs 六个 handler 的出参归一化 ──────────
// 模板降级路径只回 {analysis,solution,references,confidence}，
// 真实 LLM 路径额外带 source/vetoed/veto_reason/governance_warnings，
// 治理拦截时再带 blocked/governance——因此这些字段一律「有则尊重，无则不臆造」。

export function normExpertAnswer(raw) {
  const a = isObject(raw) ? raw : {}
  return {
    analysis: str(a.analysis),
    solution: str(a.solution),
    references: arr(a.references).map(str),
    confidence: num(a.confidence),
    // 'llm' 表示走了真实模型，空串表示模板降级：二者可信度不同，必须区分呈现
    source: str(a.source),
    vetoed: bool(a.vetoed),
    blocked: bool(a.blocked),
    vetoReason: str(a.veto_reason),
    warnings: isObject(a.governance_warnings) ? a.governance_warnings : null
  }
}

/** 协作结果的统一判定：拦截优先于成功，供 UI 单点选择呈现口径 */
export function collabOutcome(result) {
  const r = result || {}
  if (r.fusion?.blocked || r.answer?.blocked || r.answer?.vetoed) return 'blocked'
  return r.mode ? 'ok' : 'empty'
}

export function normRouteResult(raw) {
  const p = raw || {}
  const decision = isObject(p.routing_decision) ? p.routing_decision : {}
  return {
    mode: COLLAB_MODE.ROUTE,
    query: str(p.query),
    candidates: arr(p.matched_experts).map((e) => {
      const av = isObject(e.availability) ? e.availability : {}
      const mt = isObject(e.metrics) ? e.metrics : {}
      return {
        id: str(e.id),
        name: str(e.name),
        title: str(e.title),
        domains: arr(e.domains).map(str),
        skills: arr(e.skills).map(str),
        matchScore: num(e.match_score),
        status: str(av.status),
        avgResponseMinutes: num(av.avg_response_minutes),
        currentLoad: num(av.current_load),
        avgRating: num(mt.avg_rating),
        totalConsultations: num(mt.total_consultations),
        resolutionRate: num(mt.resolution_rate)
      }
    }),
    recommendation: {
      expertId: str(decision.recommended_expert_id),
      reason: str(decision.reason),
      alternativeIds: arr(decision.alternative_ids).map(str)
    },
    totalScanned: num(p.total_scanned),
    ts: str(p.ts)
  }
}

export function normSingleConsult(raw) {
  const p = raw || {}
  return {
    mode: COLLAB_MODE.SINGLE,
    sessionId: str(p.session_id),
    expertId: str(p.expert_id),
    expertName: str(p.expert_name),
    question: str(p.question),
    answer: normExpertAnswer(p.answer),
    createdAt: str(p.created_at)
  }
}

export function normMultiConsult(raw) {
  const p = raw || {}
  const f = isObject(p.fused_answer) ? p.fused_answer : {}
  return {
    mode: COLLAB_MODE.MULTI,
    sessionId: str(p.session_id),
    question: str(p.question),
    // 后端字段名是 experts：存量页面读 results/successful，那两个键在本仓库任何实现里都不存在
    contributions: arr(p.experts).map((e) => ({
      id: str(e.id),
      name: str(e.name),
      matchScore: num(e.match_score),
      answer: normExpertAnswer(e.answer)
    })),
    fusion: {
      summary: str(f.summary),
      consensusScore: num(f.consensus_score),
      dominantView: str(f.dominant_view),
      alternativeViews: arr(f.alternative_views).map(str),
      confidence: num(f.confidence),
      vetoed: bool(f.vetoed),
      blocked: bool(f.blocked)
    },
    createdAt: str(p.created_at)
  }
}

export function normDebate(raw) {
  const p = raw || {}
  const v = isObject(p.verdict) ? p.verdict : {}
  return {
    mode: COLLAB_MODE.DEBATE,
    debateId: str(p.debate_id),
    topic: str(p.topic),
    rounds: num(p.rounds),
    participants: arr(p.participants).map((e) => ({
      id: str(e.id),
      name: str(e.name),
      side: str(e.side),
      finalScore: num(e.final_score)
    })),
    log: arr(p.debate_log).map((r) => ({
      round: num(r.round),
      proArgument: str(r.pro_argument),
      conArgument: str(r.con_argument),
      proScore: num(r.pro_score),
      conScore: num(r.con_score)
    })),
    verdict: {
      winner: str(v.winner),
      summary: str(v.summary),
      keyPoints: arr(v.key_points).map(str),
      consensusLevel: str(v.consensus_level)
    },
    createdAt: str(p.created_at)
  }
}

export function normIntelligentConsult(raw) {
  const p = raw || {}
  const answer = isObject(p.answer) ? p.answer : {}
  const risk = isObject(answer.risk_assessment) ? answer.risk_assessment : {}
  const matched = isObject(p.matched_expert) ? p.matched_expert : {}
  return {
    mode: COLLAB_MODE.SMART,
    consultationId: str(p.consultation_id),
    question: str(p.question),
    intent: str(p.intent),
    expert: { id: str(matched.id), name: str(matched.name), title: str(matched.title) },
    answer: Object.assign(normExpertAnswer(answer), {
      actionItems: arr(answer.action_items).map(str),
      risk: {
        technical: str(risk.technical_risk),
        schedule: str(risk.schedule_risk),
        resource: str(risk.resource_risk),
        level: str(risk.overall_level)
      }
    }),
    relatedExperts: arr(p.related_experts).map((e) => ({
      id: str(e.id),
      name: str(e.name),
      title: str(e.title),
      domains: arr(e.domains).map(str)
    })),
    createdAt: str(p.created_at)
  }
}

export function normAlgorithmAnalysis(raw) {
  const p = raw || {}
  const c = isObject(p.complexity) ? p.complexity : {}
  const f = isObject(p.feasibility) ? p.feasibility : {}
  return {
    mode: COLLAB_MODE.ALGORITHM,
    analysisId: str(p.analysis_id),
    description: str(p.algorithm_description),
    // input_constraints / requirements 后端原样回显，未传时为 null
    echoed: { inputConstraints: p.input_constraints ?? '', requirements: p.requirements ?? '' },
    complexity: {
      time: str(c.time_complexity),
      space: str(c.space_complexity),
      bigO: str(c.big_o_notation),
      explanation: str(c.explanation)
    },
    feasibility: {
      score: num(f.score),
      blockers: arr(f.blockers).map(str),
      risks: arr(f.risks).map(str)
    },
    suggestions: arr(p.optimization_suggestions).map(str),
    recommendedExperts: arr(p.recommended_experts).map((e) => ({
      id: str(e.id),
      name: str(e.name),
      title: str(e.title),
      domains: arr(e.domains).map(str),
      matchScore: num(e.match_score)
    })),
    createdAt: str(p.created_at)
  }
}

/** 一次协作结果的后端标识：咨询/辩论/分析各落自己的 id，供运行历史引用 */
export function collabRefId(result) {
  const r = result || {}
  return r.sessionId || r.debateId || r.consultationId || r.analysisId || r.ts || ''
}

/** 协作结果里可复述的问题原文（辩论用 topic，算法分析用描述） */
export function collabQuestionText(result) {
  const r = result || {}
  return r.query || r.question || r.topic || r.description || ''
}

/**
 * PUT /api/alliance/tasks/:id/toggle-done —— 同一端点两条分支的形状不同：
 * 本地 handler 给 previous_status/current_status/toggled/completed_at（alliance.rs:1646-1657），
 * 远程分支只回 {success,message}，task_id 落在信封同级的 params 里，剥完 data 就不存在
 * （alliance_remote.rs:543-552）。所以方向只能由 toggled 或远程的 success 判定，
 * 缺证据就是 unknown —— 拿 HTTP 200 当"已重开"会把远程的 409 语义抹平。
 */
export function normToggleDone(raw) {
  const s = raw || {}
  const local = Object.prototype.hasOwnProperty.call(s, 'toggled')
  const remote = !local && s.success === true
  return {
    branch: local ? 'local' : remote ? 'remote' : 'unknown',
    taskId: str(s.task_id),
    previousStatus: str(s.previous_status),
    currentStatus: str(s.current_status),
    completedAt: s.completed_at === undefined || s.completed_at === null ? null : str(s.completed_at),
    direction: local ? (s.toggled ? 'completed' : 'reopened') : remote ? 'completed' : 'unknown',
    message: str(s.message)
  }
}

/**
 * POST /api/experts/dispatcher/dispatch —— 选人结果。
 * strategy_used 是后端各分支自己写回的字面量（specified / 四种策略 / best_match(fallback)，
 * experts_dispatcher.rs:245,262,284,301,309,320），前端不改写也不"归一"成配置值。
 * assigned_experts 由注册表 filter_map 得到（:584-596），查无此人的 id 已被后端丢掉，
 * 这里不用 match_scores 的键补人——补出来的会是一个界面能点、后端不认的假专家。
 */
export function normDispatch(raw) {
  const s = raw || {}
  return {
    dispatchId: str(s.dispatch_id),
    taskType: str(s.task_type),
    strategyUsed: str(s.strategy_used),
    status: str(s.status),
    createdAt: str(s.created_at),
    assigned: arr(s.assigned_experts).map((e) => ({
      id: str(e?.id),
      name: str(e?.name),
      matchScore: num(e?.match_score),
      loadRatio: num(e?.load_ratio)
    }))
  }
}

// ── 编排面（experts_orchestration.rs 的六个 handler）───────────────────
// 本段只做 snake→camel 与缺字段兜底，**不判断内容真伪**：哪一格是查表文案、哪一格是常量，
// 全部由 contract/orchestration.js 的 ORCH_FIELD_PROVENANCE / ORCH_SIMULATED 单源，视图按字段路径取角标。

const normOrchExpert = (e) => ({ id: str(e?.id), name: str(e?.name), title: str(e?.title) })

/** fuse_results 出参（:432-438），orchestrate 的 result 与 execute 的 final_result 共用 */
function normOrchFusion(raw) {
  const f = isObject(raw) ? raw : null
  if (!f) return null
  return {
    summary: str(f.summary),
    keyFindings: arr(f.key_findings).map((v) => str(v)),
    stepSummaries: arr(f.step_summaries).map((v) => str(v)),
    recommendations: arr(f.recommendations).map((v) => str(v)),
    confidence: num(f.confidence),
    fusionStrategy: str(f.fusion_strategy)
  }
}

/** simulate_step_execution 出参（:265-273）。expert 恒为 null——执行器传的是空专家表（:488） */
function normOrchStepResult(raw) {
  const r = isObject(raw) ? raw : null
  if (!r) return null
  return {
    stepId: str(r.step_id),
    stepType: str(r.step_type),
    summary: str(r.summary),
    keyFindings: arr(r.key_findings).map((v) => str(v)),
    expert: isObject(r.expert) ? normOrchExpert(r.expert) : null,
    confidence: num(r.confidence),
    executedAt: str(r.executed_at)
  }
}

/** 计划里的步骤：plan/generate 给 7 键（:710-718），orchestrate 只给 4 键（:642-647），缺的一律空值 */
function normOrchPlanStep(raw) {
  const s = isObject(raw) ? raw : {}
  return {
    stepId: str(s.step_id),
    name: str(s.name),
    description: str(s.description),
    expertId: str(s.expert_id),
    stepType: str(s.step_type),
    dependsOn: arr(s.depends_on).map((v) => str(v)),
    status: str(s.status)
  }
}

/** POST /api/experts/orchestrate → 视图模型（响应 8 键，:649-666） */
export function normOrchestration(payload) {
  const p = isObject(payload) ? payload : {}
  const plan = isObject(p.plan) ? p.plan : {}
  const ex = isObject(p.execution) ? p.execution : {}
  return {
    orchestrationId: str(p.orchestration_id),
    task: str(p.task),
    taskType: str(p.task_type),
    experts: arr(p.experts).map(normOrchExpert),
    planId: str(plan.plan_id),
    steps: arr(plan.steps).map(normOrchPlanStep),
    execution: {
      status: str(ex.status),
      stepsCompleted: num(ex.steps_completed),
      stepsTotal: num(ex.steps_total),
      durationMs: num(ex.duration_ms)
    },
    result: normOrchFusion(p.result),
    createdAt: str(p.created_at)
  }
}

/** POST /api/experts/plan/generate → 视图模型（响应 8 键，:720-729；status 恒 draft） */
export function normOrchPlan(payload) {
  const p = isObject(payload) ? payload : {}
  return {
    planId: str(p.plan_id),
    task: str(p.task),
    taskType: str(p.task_type),
    experts: arr(p.experts).map(normOrchExpert),
    steps: arr(p.steps).map(normOrchPlanStep),
    fusionStrategy: str(p.fusion_strategy),
    status: str(p.status),
    createdAt: str(p.created_at)
  }
}

/**
 * POST /api/experts/plan/execute → 视图模型。
 * 成功形状 9 键、成环失败形状 8 键（:518-528 / :453-462），两者都从 ok() 出去，
 * 所以 `error` 与 `finalResult` 不会同时出现，且 failed 时它们各自为 null。
 */
export function normOrchExecution(payload) {
  const p = isObject(payload) ? payload : {}
  return {
    planId: str(p.plan_id),
    executionId: str(p.execution_id),
    status: str(p.status),
    overallStatus: str(p.overall_status),
    error: str(p.error),
    stepsTotal: num(p.steps_total),
    durationMs: num(p.duration_ms),
    completedAt: str(p.completed_at),
    stepsExecuted: arr(p.steps_executed).map((s) => {
      const x = isObject(s) ? s : {}
      return {
        stepId: str(x.step_id),
        name: str(x.name),
        status: str(x.status),
        durationMs: num(x.duration_ms),
        result: normOrchStepResult(x.result)
      }
    }),
    finalResult: normOrchFusion(p.final_result)
  }
}

/** GET /api/experts/orchestration/stats → 视图模型（14 键，:820-835） */
export function normOrchStats(payload) {
  const p = isObject(payload) ? payload : {}
  return {
    totalPlans: num(p.total_plans),
    plansDraft: num(p.plans_draft),
    plansReady: num(p.plans_ready),
    plansRunning: num(p.plans_running),
    plansCompleted: num(p.plans_completed),
    plansFailed: num(p.plans_failed),
    totalExecutions: num(p.total_executions),
    successRate: num(p.success_rate),
    avgDurationMs: num(p.avg_duration_ms),
    avgStepsPerPlan: num(p.avg_steps_per_plan),
    topUsedExperts: arr(p.top_used_experts).map((e) => ({ expertId: str(e?.expert_id), usageCount: num(e?.usage_count) })),
    fusionStrategyDistribution: isObject(p.fusion_strategy_distribution) ? p.fusion_strategy_distribution : {},
    taskTypeDistribution: isObject(p.task_type_distribution) ? p.task_type_distribution : {},
    serverTs: str(p.ts)
  }
}

/** GET /api/experts/orchestration/history → 视图模型（响应 4 键，行是 OrchestrationRecord 12 键） */
export function normOrchHistory(payload) {
  const p = isObject(payload) ? payload : {}
  return {
    total: num(p.total),
    page: num(p.page),
    pageSize: num(p.page_size),
    records: arr(p.records).map((r) => {
      const x = isObject(r) ? r : {}
      return {
        executionId: str(x.execution_id),
        planId: str(x.plan_id),
        taskType: str(x.task_type),
        status: str(x.status),
        expertIds: arr(x.expert_ids).map((v) => str(v)),
        stepsCompleted: num(x.steps_completed),
        stepsTotal: num(x.steps_total),
        resultSummary: str(x.result_summary),
        result: normOrchFusion(x.result),
        createdAt: str(x.created_at),
        completedAt: str(x.completed_at),
        durationMs: num(x.duration_ms)
      }
    })
  }
}
