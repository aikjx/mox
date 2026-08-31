/**
 * 专家联盟 + 语音统一 API 层（FR-FE-01）
 *  - 7 类基准确认
 *  - 联盟 SSE 流
 *  - 语音 health + 三层回退
 *  - 专家注册 / 列表 / 咨询 / 协同 / 辩论 / 智能路由 / 算法分析 / 概览指标
 * BASE_URL 默认 /api 转发到 Rust 网关 runtime-gateway；跨域未配时也可以直接走同源。
 */

export const ALLIANCE_BASE = (import.meta as any).env?.VITE_API_BASE ?? ''

/** 读取前端可用的 Bearer 令牌：优先 env（Vite 注入时为 VITE_OUS_API_TOKEN，Node dev proxy 端用 OUS_API_TOKEN）。
 *  前端本地运行时，vite.config 的 proxyReq 会兜底注入进程 env OUS_API_TOKEN，所以这里返回空字符串时不会阻断。 */
function getBearerToken(): string | null {
  const env = (import.meta as any).env ?? {}
  return env.VITE_OUS_API_TOKEN || env.OUS_API_TOKEN || null
}

export function authHeaders(extra: Record<string, string> = {}): Record<string, string> {
  const h: Record<string, string> = { ...extra }
  const t = getBearerToken()
  if (t) h['Authorization'] = `Bearer ${t}`
  return h
}

// ============================================================================
// 类型定义
// ============================================================================

export type AlliancePhase =
  | 'intent' | 'team' | 'debate' | 'synthesize' | 'gate' | 'learn' | 'done'

export interface AllianceSSEFrame {
  phase: AlliancePhase
  phase_index: number
  payload: any
  trace_id: string
  latency_ms: number
  ts: string
  degraded?: boolean
  degrade_reason?: string | null
}

export interface AllianceCapabilities {
  version: string
  phases: string[]
  intent_classes_7: string[]
  dimensions_14: Array<[string, number]>
  hc_params: Record<string, string>
  audit_events_7: string[]
  health: string
}

export interface AllianceFullRequest {
  query: string
  session_id?: string | null
  idempotency_key?: string | null
  context?: Record<string, string>
  enable_llm_debate?: boolean
  team_size?: number
  retry_on_c?: boolean
}

export interface VoiceHealth {
  ok: boolean
  asr?: { ready: boolean; model: string; backend: string }
  tts?: {
    ready: boolean
    engines: Array<{ name: string; available: boolean; license: string; note?: string }>
    active: string
  }
  endpoints?: Record<string, string>
  upstream_unreachable?: boolean
  fallback_action?: string
}

// ---- 专家相关类型 ----

/** 专家领域标签 */
export type ExpertDomain =
  | 'algorithm' | 'architecture' | 'data' | 'ai' | 'workflow'
  | 'operator' | 'graph' | 'security' | 'performance' | 'monitor'
  | 'market' | 'mcp' | 'automation' | 'requirement' | 'fusion'

/** 专家状态 */
export type ExpertStatus = 'online' | 'busy' | 'thinking' | 'offline' | 'indexing'

/** 专家信息 */
export interface Expert {
  id: string
  name: string
  avatar?: string
  role: string
  domain: ExpertDomain
  domains?: ExpertDomain[]
  status: ExpertStatus
  description?: string
  capabilities?: string[]
  tags?: string[]
  color?: string
  rating?: number
  consult_count?: number
  created_at?: string
  updated_at?: string
  enterprise_id?: string
  model_backend?: string
  prompt_template?: string
  is_active?: boolean
}

/** 专家注册请求 */
export interface ExpertRegisterRequest {
  name: string
  role: string
  domain: ExpertDomain
  domains?: ExpertDomain[]
  description?: string
  capabilities?: string[]
  tags?: string[]
  avatar?: string
  color?: string
  model_backend?: string
  prompt_template?: string
  enterprise_id?: string
  system_prompt?: string
  temperature?: number
  top_p?: number
  max_tokens?: number
}

/** 专家注册响应 */
export interface ExpertRegisterResponse {
  expert_id: string
  name: string
  status: 'registered' | 'pending_approval' | 'active'
  created_at: string
  message?: string
}

/** 专家列表查询参数 */
export interface ExpertListParams {
  page?: number
  page_size?: number
  domain?: ExpertDomain
  status?: ExpertStatus
  keyword?: string
  tags?: string[]
  enterprise_id?: string
  sort_by?: 'rating' | 'consult_count' | 'created_at' | 'name'
  sort_order?: 'asc' | 'desc'
}

/** 专家列表分页响应 */
export interface ExpertListResponse {
  items: Expert[]
  total: number
  page: number
  page_size: number
  has_more: boolean
}

/** 单专家咨询请求 */
export interface ExpertConsultRequest {
  expert_id: string
  query: string
  session_id?: string
  context?: Record<string, any>
  project_id?: string
  stream?: boolean
  temperature?: number
  max_tokens?: number
}

/** 单专家咨询响应 */
export interface ExpertConsultResponse {
  expert_id: string
  expert_name: string
  answer: string
  confidence?: number
  references?: Array<{ title: string; url?: string; snippet?: string }>
  latency_ms: number
  trace_id: string
  session_id?: string
}

/** 多专家协同请求 */
export interface MultiExpertConsultRequest {
  expert_ids: string[]
  query: string
  session_id?: string
  context?: Record<string, any>
  project_id?: string
  strategy?: 'parallel' | 'sequential' | 'hierarchical'
  stream?: boolean
}

/** 多专家协同响应 */
export interface MultiExpertConsultResponse {
  session_id: string
  trace_id: string
  results: Array<{
    expert_id: string
    expert_name: string
    answer: string
    confidence?: number
    latency_ms: number
    order: number
  }>
  summary?: string
  total_latency_ms: number
}

/** 专家辩论请求 */
export interface ExpertDebateRequest {
  topic: string
  expert_ids: string[]
  session_id?: string
  context?: Record<string, any>
  project_id?: string
  rounds?: number
  enable_voting?: boolean
  stream?: boolean
}

/** 辩论阶段 */
export type DebatePhase = 'opening' | 'argument' | 'rebuttal' | 'voting' | 'conclusion'

/** 专家辩论 SSE 帧 */
export interface ExpertDebateFrame {
  phase: DebatePhase
  round: number
  expert_id: string
  expert_name: string
  content: string
  timestamp: string
  trace_id: string
}

/** 专家辩论结果 */
export interface ExpertDebateResult {
  session_id: string
  trace_id: string
  topic: string
  total_rounds: number
  winner?: string
  votes?: Record<string, number>
  conclusion: string
  participants: string[]
  total_latency_ms: number
}

/** 智能路由请求 */
export interface RouteExpertsRequest {
  query: string
  context?: Record<string, any>
  project_id?: string
  max_experts?: number
  min_confidence?: number
  domain_filter?: ExpertDomain[]
}

/** 智能路由匹配结果 */
export interface RoutedExpert {
  expert: Expert
  match_score: number
  match_reason: string
  recommended_role: 'primary' | 'supporting' | 'reviewer'
}

/** 智能路由响应 */
export interface RouteExpertsResponse {
  query: string
  intent_class?: string
  matched_experts: RoutedExpert[]
  recommended_team_size: number
  trace_id: string
  latency_ms: number
}

/** 智能咨询请求（一键路由 + 协同）*/
export interface IntelligentConsultRequest {
  query: string
  context?: Record<string, any>
  project_id?: string
  session_id?: string
  max_experts?: number
  strategy?: 'auto' | 'debate' | 'collaborative' | 'single_best'
  stream?: boolean
}

/** 智能咨询响应 */
export interface IntelligentConsultResponse {
  session_id: string
  trace_id: string
  query: string
  strategy_used: string
  expert_count: number
  answer: string
  confidence: number
  expert_contributions: Array<{
    expert_id: string
    expert_name: string
    contribution: string
    weight: number
  }>
  total_latency_ms: number
}

/** 算法分析请求 */
export interface AlgorithmAnalysisRequest {
  algorithm: string
  data: Record<string, any>
  params?: Record<string, any>
  project_id?: string
}

/** 算法分析响应 */
export interface AlgorithmAnalysisResponse {
  algorithm: string
  result: Record<string, any>
  metrics?: Record<string, number>
  visualization?: {
    type: 'graph' | 'chart' | 'table' | 'heatmap'
    data: any
  }
  latency_ms: number
  trace_id: string
}

/** 专家概览数据 */
export interface ExpertOverview {
  total_experts: number
  online_experts: number
  total_domains: number
  total_consultations: number
  today_consultations: number
  avg_response_time_ms: number
  avg_rating: number
  active_sessions: number
  top_domains: Array<{ domain: ExpertDomain; count: number }>
  recent_activity: Array<{
    id: string
    type: 'consult' | 'debate' | 'register' | 'update'
    expert_name: string
    timestamp: string
    summary: string
  }>
}

/** 专家指标数据 */
export interface ExpertMetrics {
  total_experts: number
  active_experts: number
  total_consultations: number
  consultations_today: number
  consultations_7d: number
  consultations_30d: number
  avg_response_time_ms: number
  avg_confidence: number
  avg_rating: number
  debate_count: number
  multi_expert_count: number
  success_rate: number
  domain_distribution: Array<{ domain: ExpertDomain; count: number; percentage: number }>
  status_distribution: Record<ExpertStatus, number>
  trend_data: Array<{ date: string; consultations: number; experts: number }>
}

/** 单专家指标 */
export interface SingleExpertMetrics {
  expert_id: string
  total_consultations: number
  avg_response_time_ms: number
  avg_rating: number
  avg_confidence: number
  success_rate: number
  debate_participations: number
  last_active_at: string
  weekly_trend: Array<{ date: string; count: number }>
}

// ============================================================================
// 联盟基础能力 API
// ============================================================================

/** GET /ai/engine/alliance/capabilities */
export async function getAllianceCapabilities(): Promise<AllianceCapabilities> {
  const r = await fetch(`${ALLIANCE_BASE}/ai/engine/alliance/capabilities`, {
    method: 'GET',
    headers: authHeaders({ 'Accept': 'application/json' }),
  })
  if (!r.ok) throw new Error(`alliance/capabilities HTTP ${r.status}`)
  return await r.json()
}

/**
 * POST /ai/engine/alliance/full → SSE（FR-FE-02/05）
 * @param onFrame 每帧回调，返回 false 表示终止
 * @returns 最后一帧 trace_id（若有）
 */
export async function runAllianceFullSSE(
  req: AllianceFullRequest,
  onFrame: (frame: AllianceSSEFrame) => boolean | void
): Promise<string> {
  // 使用 fetch + ReadableStream.getReader() 解析 SSE（比 EventSource 更灵活：支持 POST body + headers）
  const resp = await fetch(`${ALLIANCE_BASE}/ai/engine/alliance/full`, {
    method: 'POST',
    headers: authHeaders({
      'Content-Type': 'application/json',
      'Accept': 'text/event-stream',
    }),
    body: JSON.stringify(req),
  })
  if (!resp.ok) {
    throw new Error(`alliance/full HTTP ${resp.status}`)
  }
  const reader = resp.body?.getReader()
  if (!reader) throw new Error('No readable stream')
  const decoder = new TextDecoder('utf-8')
  let buffer = ''
  let lastTraceId = ''
  let currentEventName = ''

  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    buffer += decoder.decode(value, { stream: true })
    // SSE 切分：\n\n
    let idx: number
    while ((idx = buffer.indexOf('\n\n')) >= 0) {
      const rawEvent = buffer.slice(0, idx)
      buffer = buffer.slice(idx + 2)
      // 解析多行 event/data
      let data = ''
      for (const line of rawEvent.split('\n')) {
        if (line.startsWith('event:')) currentEventName = line.slice(6).trim()
        else if (line.startsWith('data:')) data += line.slice(5).trimStart()
      }
      if (!data) continue
      if (data === '[DONE]') {
        reader.releaseLock()
        return lastTraceId
      }
      try {
        const frame: AllianceSSEFrame = JSON.parse(data)
        if (frame.trace_id) lastTraceId = frame.trace_id
        const cont = onFrame(frame)
        if (cont === false) {
          reader.releaseLock()
          return lastTraceId
        }
      } catch (e) {
        console.warn('[alliance.sse] frame parse failed:', data, e)
      }
    }
  }
  reader.releaseLock()
  return lastTraceId
}

/** GET /voice/health（T12 专用） */
export async function getVoiceHealth(): Promise<VoiceHealth> {
  try {
    const r = await fetch(`${ALLIANCE_BASE}/voice/health`, { method: 'GET', headers: authHeaders({ Accept: 'application/json' }) })
    if (!r.ok) throw new Error(`HTTP ${r.status}`)
    return await r.json()
  } catch (e: any) {
    // 第三层回退（AC-22）：网关都连不上 → 完全本地 browser_tts
    return {
      ok: false,
      upstream_unreachable: true,
      fallback_action: 'AC-22 三层回退（连不上 Rust 网关）：直接 browser Web Speech Synthesis',
      tts: {
        ready: false,
        active: 'browser_tts',
        engines: [
          { name: 'cosyvoice2', available: false, license: 'Apache-2.0' },
          { name: 'fish_s2_pro', available: false, license: 'Research', note: '默认禁用，Research License' },
        ],
      },
    }
  }
}

// ============================================================================
// 专家管理 API
// ============================================================================

/** POST /experts - 专家注册 */
export async function registerExpert(
  payload: ExpertRegisterRequest
): Promise<ExpertRegisterResponse> {
  const r = await fetch(`${ALLIANCE_BASE}/experts`, {
    method: 'POST',
    headers: authHeaders({
      'Content-Type': 'application/json',
      'Accept': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
  if (!r.ok) throw new Error(`experts/register HTTP ${r.status}`)
  const data = await r.json()
  // 兼容 {success, data} 信封格式
  if (data && typeof data === 'object' && 'success' in data) {
    if (!data.success) throw new Error(data.error || data.message || '注册失败')
    return data.data
  }
  return data
}

/** GET /experts - 获取专家列表（分页 + 筛选） */
export async function getExperts(
  params: ExpertListParams = {}
): Promise<ExpertListResponse> {
  const query = new URLSearchParams()
  if (params.page != null) query.set('page', String(params.page))
  if (params.page_size != null) query.set('page_size', String(params.page_size))
  if (params.domain) query.set('domain', params.domain)
  if (params.status) query.set('status', params.status)
  if (params.keyword) query.set('keyword', params.keyword)
  if (params.tags?.length) query.set('tags', params.tags.join(','))
  if (params.enterprise_id) query.set('enterprise_id', params.enterprise_id)
  if (params.sort_by) query.set('sort_by', params.sort_by)
  if (params.sort_order) query.set('sort_order', params.sort_order)

  const url = `${ALLIANCE_BASE}/experts${query.toString() ? `?${query.toString()}` : ''}`
  const r = await fetch(url, {
    method: 'GET',
    headers: authHeaders({ 'Accept': 'application/json' }),
  })
  if (!r.ok) throw new Error(`experts/list HTTP ${r.status}`)
  const data = await r.json()
  if (data && typeof data === 'object' && 'success' in data) {
    if (!data.success) throw new Error(data.error || data.message || '获取专家列表失败')
    return data.data
  }
  return data
}

// ============================================================================
// 专家咨询 API
// ============================================================================

/** POST /experts/{id}/consult - 单专家咨询 */
export async function consultExpert(
  expertId: string,
  payload: Omit<ExpertConsultRequest, 'expert_id'>
): Promise<ExpertConsultResponse> {
  const r = await fetch(`${ALLIANCE_BASE}/experts/${encodeURIComponent(expertId)}/consult`, {
    method: 'POST',
    headers: authHeaders({
      'Content-Type': 'application/json',
      'Accept': 'application/json',
    }),
    body: JSON.stringify({ ...payload, expert_id: expertId }),
  })
  if (!r.ok) throw new Error(`experts/consult HTTP ${r.status}`)
  const data = await r.json()
  if (data && typeof data === 'object' && 'success' in data) {
    if (!data.success) throw new Error(data.error || data.message || '咨询失败')
    return data.data
  }
  return data
}

/** POST /experts/multi-consult - 多专家协同咨询 */
export async function multiExpertConsult(
  payload: MultiExpertConsultRequest
): Promise<MultiExpertConsultResponse> {
  const r = await fetch(`${ALLIANCE_BASE}/experts/multi-consult`, {
    method: 'POST',
    headers: authHeaders({
      'Content-Type': 'application/json',
      'Accept': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
  if (!r.ok) throw new Error(`experts/multi-consult HTTP ${r.status}`)
  const data = await r.json()
  if (data && typeof data === 'object' && 'success' in data) {
    if (!data.success) throw new Error(data.error || data.message || '多专家协同失败')
    return data.data
  }
  return data
}

// ============================================================================
// 专家辩论 API
// ============================================================================

/** POST /experts/debate - 专家辩论（SSE 流式） */
export async function expertDebate(
  payload: ExpertDebateRequest,
  onFrame?: (frame: ExpertDebateFrame) => boolean | void
): Promise<ExpertDebateResult> {
  // 如果不需要流式，走普通 JSON 响应
  if (!payload.stream || !onFrame) {
    const r = await fetch(`${ALLIANCE_BASE}/experts/debate`, {
      method: 'POST',
      headers: authHeaders({
        'Content-Type': 'application/json',
        'Accept': 'application/json',
      }),
      body: JSON.stringify(payload),
    })
    if (!r.ok) throw new Error(`experts/debate HTTP ${r.status}`)
    const data = await r.json()
    if (data && typeof data === 'object' && 'success' in data) {
      if (!data.success) throw new Error(data.error || data.message || '辩论失败')
      return data.data
    }
    return data
  }

  // SSE 流式辩论
  const resp = await fetch(`${ALLIANCE_BASE}/experts/debate`, {
    method: 'POST',
    headers: authHeaders({
      'Content-Type': 'application/json',
      'Accept': 'text/event-stream',
    }),
    body: JSON.stringify(payload),
  })
  if (!resp.ok) throw new Error(`experts/debate HTTP ${resp.status}`)
  const reader = resp.body?.getReader()
  if (!reader) throw new Error('No readable stream')
  const decoder = new TextDecoder('utf-8')
  let buffer = ''
  let finalResult: ExpertDebateResult | null = null

  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    buffer += decoder.decode(value, { stream: true })
    let idx: number
    while ((idx = buffer.indexOf('\n\n')) >= 0) {
      const rawEvent = buffer.slice(0, idx)
      buffer = buffer.slice(idx + 2)
      let data = ''
      for (const line of rawEvent.split('\n')) {
        if (line.startsWith('data:')) data += line.slice(5).trimStart()
      }
      if (!data) continue
      if (data === '[DONE]') {
        reader.releaseLock()
        if (!finalResult) throw new Error('辩论未返回最终结果')
        return finalResult
      }
      try {
        const parsed = JSON.parse(data)
        // 最终结果帧带 result 标记
        if (parsed.type === 'result' || parsed.conclusion) {
          finalResult = parsed
        } else if (onFrame) {
          const frame: ExpertDebateFrame = parsed
          const cont = onFrame(frame)
          if (cont === false) {
            reader.releaseLock()
            return finalResult || ({} as ExpertDebateResult)
          }
        }
      } catch (e) {
        console.warn('[alliance.debate] frame parse failed:', data, e)
      }
    }
  }
  reader.releaseLock()
  if (!finalResult) throw new Error('辩论未返回最终结果')
  return finalResult
}

// ============================================================================
// 智能路由 API
// ============================================================================

/** POST /experts/route - 智能路由匹配专家 */
export async function routeExperts(
  payload: RouteExpertsRequest
): Promise<RouteExpertsResponse> {
  const r = await fetch(`${ALLIANCE_BASE}/experts/route`, {
    method: 'POST',
    headers: authHeaders({
      'Content-Type': 'application/json',
      'Accept': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
  if (!r.ok) throw new Error(`experts/route HTTP ${r.status}`)
  const data = await r.json()
  if (data && typeof data === 'object' && 'success' in data) {
    if (!data.success) throw new Error(data.error || data.message || '智能路由失败')
    return data.data
  }
  return data
}

/** POST /experts/intelligent-consult - 智能咨询（一键路由 + 协同） */
export async function intelligentConsult(
  payload: IntelligentConsultRequest
): Promise<IntelligentConsultResponse> {
  const r = await fetch(`${ALLIANCE_BASE}/experts/intelligent-consult`, {
    method: 'POST',
    headers: authHeaders({
      'Content-Type': 'application/json',
      'Accept': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
  if (!r.ok) throw new Error(`experts/intelligent-consult HTTP ${r.status}`)
  const data = await r.json()
  if (data && typeof data === 'object' && 'success' in data) {
    if (!data.success) throw new Error(data.error || data.message || '智能咨询失败')
    return data.data
  }
  return data
}

// ============================================================================
// 算法分析 API
// ============================================================================

/** POST /experts/algorithm-analysis - 算法分析 */
export async function algorithmAnalysis(
  payload: AlgorithmAnalysisRequest
): Promise<AlgorithmAnalysisResponse> {
  const r = await fetch(`${ALLIANCE_BASE}/experts/algorithm-analysis`, {
    method: 'POST',
    headers: authHeaders({
      'Content-Type': 'application/json',
      'Accept': 'application/json',
    }),
    body: JSON.stringify(payload),
  })
  if (!r.ok) throw new Error(`experts/algorithm-analysis HTTP ${r.status}`)
  const data = await r.json()
  if (data && typeof data === 'object' && 'success' in data) {
    if (!data.success) throw new Error(data.error || data.message || '算法分析失败')
    return data.data
  }
  return data
}

// ============================================================================
// 概览与指标 API
// ============================================================================

/** GET /experts/overview - 获取专家联盟概览数据 */
export async function getExpertOverview(): Promise<ExpertOverview> {
  const r = await fetch(`${ALLIANCE_BASE}/experts/overview`, {
    method: 'GET',
    headers: authHeaders({ 'Accept': 'application/json' }),
  })
  if (!r.ok) throw new Error(`experts/overview HTTP ${r.status}`)
  const data = await r.json()
  if (data && typeof data === 'object' && 'success' in data) {
    if (!data.success) throw new Error(data.error || data.message || '获取概览失败')
    return data.data
  }
  return data
}

/** GET /experts/metrics - 获取专家联盟指标数据 */
export async function getExpertMetrics(): Promise<ExpertMetrics> {
  const r = await fetch(`${ALLIANCE_BASE}/experts/metrics`, {
    method: 'GET',
    headers: authHeaders({ 'Accept': 'application/json' }),
  })
  if (!r.ok) throw new Error(`experts/metrics HTTP ${r.status}`)
  const data = await r.json()
  if (data && typeof data === 'object' && 'success' in data) {
    if (!data.success) throw new Error(data.error || data.message || '获取指标失败')
    return data.data
  }
  return data
}

/** GET /experts/{id}/metrics - 获取单个专家指标 */
export async function getSingleExpertMetrics(expertId: string): Promise<SingleExpertMetrics> {
  const r = await fetch(`${ALLIANCE_BASE}/experts/${encodeURIComponent(expertId)}/metrics`, {
    method: 'GET',
    headers: authHeaders({ 'Accept': 'application/json' }),
  })
  if (!r.ok) throw new Error(`experts/metrics/${expertId} HTTP ${r.status}`)
  const data = await r.json()
  if (data && typeof data === 'object' && 'success' in data) {
    if (!data.success) throw new Error(data.error || data.message || '获取专家指标失败')
    return data.data
  }
  return data
}
