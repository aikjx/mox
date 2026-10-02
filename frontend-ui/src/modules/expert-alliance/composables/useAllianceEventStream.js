// T4 事件总线对外出口 · SSE 事件帧前端消费接入点。
//
// 后端：GET /api/alliance/events/stream（experts_streams.rs），按认证租户下发本租户业务事件帧：
//   event: <AllianceEventKind>   // PlanCreated / PlanStatusChanged / ExpertRegistered / ExpertDisabled
//   data:  <事件信封 JSON>        // { id, type, tenant, occurred_at, ... }
//
// 本文件是「真实接入点」：直接 fetch 该端点、按 SSE 帧切分、把 (kind, envelope) 回调出去。
// 它目前未被任何视图 import（避免改动既有 1044 vitest 基线）；要在视图里实时刷新，
// 在 AllianceOrchestrationView / 联盟控制台 onMounted 里调用 start()、onUnmounted 调 stop() 即可，
// 收到事件后去拉对应列表（plan / expert registry）即可完成局部刷新。
import { ENDPOINTS, requestPath } from './endpoints'

/**
 * 订阅 T4 事件帧流。
 *
 * @param {object}   opts
 * @param {(kind: string, envelope: object, raw: string) => void} opts.onEvent  每帧回调
 * @param {(err: Error) => void}   [opts.onError]
 * @param {() => void}              [opts.onOpen]
 * @returns {{ start: () => Promise<void>, stop: () => void }}
 */
export function useAllianceEventStream({ onEvent, onError, onOpen } = {}) {
  const path = requestPath('allianceEventStream', {})
  let controller = null
  let stopped = false

  async function start() {
    stopped = false
    // 走与全站一致的相对基址；鉴权由浏览器凭据/拦截器附带的 cookie 或网关既有注入负责，
    // 与既有任务日志流（getExecutionLogsSSE）同一套访问语义。
    const resp = await fetch(path, {
      headers: { Accept: 'text/event-stream' },
      credentials: 'include'
    })
    if (!resp.ok || !resp.body) {
      throw new Error(`事件流打开失败: HTTP ${resp.status}`)
    }
    onOpen && onOpen()

    controller = new AbortController()
    const reader = resp.body.getReader()
    const decoder = new TextDecoder('utf-8')
    let buf = ''

    // SSE 帧以空行（\n\n 或 \r\n\r\n）分隔；每帧内可能有 event: 与 data: 多行。
    const dispatch = (block) => {
      let kind = 'message'
      const dataLines = []
      for (const line of block.split(/\r?\n/)) {
        if (line.startsWith('event:')) kind = line.slice(6).trim()
        else if (line.startsWith('data:')) dataLines.push(line.slice(5).trim())
      }
      if (!dataLines.length) return
      const raw = dataLines.join('\n')
      let envelope = {}
      try { envelope = JSON.parse(raw) } catch { envelope = { raw } }
      onEvent && onEvent(kind, envelope, raw)
    }

    while (!stopped) {
      const { done, value } = await reader.read()
      if (done) break
      buf += decoder.decode(value, { stream: true })
      let sep
      while ((sep = buf.search(/\r?\n\r?\n/)) >= 0) {
        const block = buf.slice(0, sep)
        buf = buf.slice(sep).replace(/^\r?\n\r?\n/, '')
        if (block.trim()) dispatch(block)
      }
    }
  }

  function stop() {
    stopped = true
    controller && controller.abort()
  }

  return {
    start: () => start().catch((e) => onError && onError(e)),
    stop
  }
}

// 静态引用端点名，便于 contract.test.js 把本文件识别为 allianceEventStream 的真实消费点。
export const ALLIANCE_EVENT_STREAM_ENDPOINT = ENDPOINTS.allianceEventStream
