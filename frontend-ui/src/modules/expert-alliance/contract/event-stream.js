// Domain events are refresh hints, never an execution result or replay log.
export function createEventFrameParser(onFrame, limit = 1024 * 1024) {
  let line = '', kind = 'message', data = [], size = 0, afterCR = false
  function finishLine() {
    if (!line) {
      if (data.length) {
        const raw = data.join('\n')
        onFrame(kind, JSON.parse(raw), raw)
      }
      kind = 'message'; data = []; size = 0
    } else if (!line.startsWith(':')) {
      const colon = line.indexOf(':')
      const field = colon < 0 ? line : line.slice(0, colon)
      let value = colon < 0 ? '' : line.slice(colon + 1)
      if (value.startsWith(' ')) value = value.slice(1)
      if (field === 'event') kind = value
      if (field === 'data') data.push(value)
    }
    line = ''
  }
  return chunk => {
    for (const char of chunk) {
      if (afterCR && char === '\n') { afterCR = false; continue }
      afterCR = false
      if (++size > limit) throw new Error('SSE frame size limit exceeded')
      if (char === '\r' || char === '\n') {
        finishLine(); afterCR = char === '\r'
      } else line += char
    }
  }
}

export function createAllianceEventStream({ url, getToken, onEvent, onError, onOpen, onClose, onGap } = {}) {
  let controller = null
  function stop() { controller?.abort(); controller = null }
  async function start() {
    stop()
    const current = new AbortController()
    controller = current
    let reader
    try {
      const token = getToken?.()
      if (!token) throw new Error('请先登录再订阅联盟事件')
      const response = await fetch(url, {
        headers: { Accept: 'text/event-stream', Authorization: `Bearer ${token}` },
        credentials: 'include', signal: current.signal
      })
      if (!response.ok || !response.body) throw new Error(`事件流打开失败: HTTP ${response.status}`)
      if (!response.headers.get('content-type')?.toLowerCase().includes('text/event-stream')) {
        throw new Error('事件流响应类型错误')
      }
      if (current.signal.aborted || controller !== current) return
      reader = response.body.getReader()
      onOpen?.()
      const decoder = new TextDecoder('utf-8', { fatal: true })
      const parse = createEventFrameParser((kind, envelope, raw) => {
        if (current.signal.aborted || controller !== current) return
        if (kind === 'StreamGap') onGap?.(envelope)
        else onEvent?.(kind, envelope, raw)
      })
      while (!current.signal.aborted) {
        const { done, value } = await reader.read()
        if (current.signal.aborted || controller !== current) return
        if (done) { parse(decoder.decode()); onClose?.(); return }
        parse(decoder.decode(value, { stream: true }))
      }
    } catch (error) {
      if (!current.signal.aborted && controller === current) onError?.(error)
    } finally {
      current.abort()
      if (reader) {
        try { await reader.cancel() } catch { /* Connection may already be closed. */ }
        reader.releaseLock()
      }
      if (controller === current) controller = null
    }
  }
  return { start, stop }
}
