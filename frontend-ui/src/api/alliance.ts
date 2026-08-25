/**
 * 专家联盟 + 语音统一 API 层（FR-FE-01）
 *  - 7 类基准确认
 *  - 联盟 SSE 流
 *  - 语音 health + 三层回退
 * BASE_URL 默认 /api 转发到 Rust 网关 runtime-gateway；跨域未配时也可以直接走同源。
 */

export const ALLIANCE_BASE = (import.meta as any).env?.VITE_API_BASE ?? '/api'

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

/** GET /ai/engine/alliance/capabilities */
export async function getAllianceCapabilities(): Promise<AllianceCapabilities> {
  const r = await fetch(`${ALLIANCE_BASE}/ai/engine/alliance/capabilities`, {
    method: 'GET',
    headers: { 'Accept': 'application/json' },
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
    headers: {
      'Content-Type': 'application/json',
      'Accept': 'text/event-stream',
    },
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
    const r = await fetch(`${ALLIANCE_BASE}/voice/health`, { method: 'GET', headers: { Accept: 'application/json' } })
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
