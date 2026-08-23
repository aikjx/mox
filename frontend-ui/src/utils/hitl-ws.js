// HITL 人机协同审批 WebSocket 客户端
// 对接 Rust 网关 /ws/hitl 真实协议（platform/gateway/runtime/src/handlers/hitl.rs）：
//   客户端 → 服务端：{type:'subscribe', filters} / {type:'list_pending', flow_id} /
//                   {type:'action', event_id, action, actor, comment, modified_payload}
//   服务端 → 客户端：{type:'connected'|'subscribed'|'hitl_event'|'action_result'|'pending_list'|'error'}
// 事件字段为 camelCase：{id, flowId, flowName, kind, description, payload, requester, ts}

const HITL_WS_PATH = '/ws/hitl'

export const HITL_ACTIONS = {
  APPROVE: 'APPROVE',
  DENY: 'DENY',
  MODIFY_APPROVE: 'MODIFY_APPROVE'
}

class HitlWebSocketClient {
  constructor() {
    this.ws = null
    this.listeners = new Map()
    this.reconnectTimer = null
    this.reconnectAttempts = 0
    this.maxReconnectAttempts = 10
    this.reconnectDelay = 2000
    this.manualClose = false
  }

  _buildUrl() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    return `${protocol}//${window.location.host}${HITL_WS_PATH}`
  }

  connect() {
    if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
      return this.ws
    }

    this.manualClose = false
    try {
      this.ws = new WebSocket(this._buildUrl())
    } catch (err) {
      console.error('[HITL WS] Failed to create connection:', err)
      this._scheduleReconnect()
      return null
    }

    this.ws.onopen = () => {
      this.reconnectAttempts = 0
      // 连接建立即订阅全量事件并拉取当前待审批列表
      this.sendRequest({ type: 'subscribe', filters: null })
      this.sendRequest({ type: 'list_pending', flow_id: null })
      this._emit('connection', { status: 'connected' })
    }

    this.ws.onmessage = (event) => this._handleMessage(event.data)

    this.ws.onerror = (err) => {
      console.error('[HITL WS] Error:', err)
      this._emit('connection', { status: 'error', error: err })
    }

    this.ws.onclose = () => {
      this._emit('connection', { status: 'disconnected' })
      if (!this.manualClose) {
        this._scheduleReconnect()
      }
    }

    return this.ws
  }

  disconnect() {
    this.manualClose = true
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    if (this.ws) {
      this.ws.close()
      this.ws = null
    }
  }

  _scheduleReconnect() {
    if (this.reconnectTimer || this.manualClose) return
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.warn('[HITL WS] Max reconnect attempts reached')
      return
    }
    this.reconnectAttempts += 1
    const delay = this.reconnectDelay * Math.min(this.reconnectAttempts, 5)
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null
      this.connect()
    }, delay)
  }

  sendRequest(req) {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      console.warn('[HITL WS] Cannot send, socket not connected')
      return false
    }
    this.ws.send(JSON.stringify(req))
    return true
  }

  // 审批动作：event_id + APPROVE/DENY/MODIFY_APPROVE（MODIFY_APPROVE 需 modifiedPayload）
  sendAction(eventId, action, { comment = null, modifiedPayload = null, actor = 'admin' } = {}) {
    if (!Object.values(HITL_ACTIONS).includes(action)) {
      console.warn('[HITL WS] Invalid action:', action)
      return false
    }
    return this.sendRequest({
      type: 'action',
      event_id: eventId,
      action,
      actor,
      comment,
      modified_payload: modifiedPayload
    })
  }

  _handleMessage(raw) {
    let payload
    try {
      payload = typeof raw === 'string' ? JSON.parse(raw) : raw
    } catch (err) {
      console.warn('[HITL WS] Invalid message format:', raw)
      return
    }
    const eventType = payload?.type || 'message'
    this._emit(eventType, payload)
    this._emit('message', payload)
  }

  _emit(eventType, payload) {
    const handlers = this.listeners.get(eventType) || []
    handlers.forEach((fn) => {
      try {
        fn(payload)
      } catch (err) {
        console.error('[HITL WS] Listener error:', err)
      }
    })
  }

  on(eventType, handler) {
    if (typeof handler !== 'function') return () => {}
    const handlers = this.listeners.get(eventType) || []
    handlers.push(handler)
    this.listeners.set(eventType, handlers)
    return () => this.off(eventType, handler)
  }

  off(eventType, handler) {
    if (!handler) {
      this.listeners.delete(eventType)
      return
    }
    const handlers = this.listeners.get(eventType)
    if (!handlers) return
    const idx = handlers.indexOf(handler)
    if (idx !== -1) handlers.splice(idx, 1)
    if (handlers.length === 0) this.listeners.delete(eventType)
  }

  isConnected() {
    return !!(this.ws && this.ws.readyState === WebSocket.OPEN)
  }
}

export const hitlClient = new HitlWebSocketClient()

export function onHitlEvent(handler) {
  return hitlClient.on('hitl_event', handler)
}

export function onHitlActionResult(handler) {
  return hitlClient.on('action_result', handler)
}

export function onHitlPendingList(handler) {
  return hitlClient.on('pending_list', handler)
}

export function onHitlConnection(handler) {
  return hitlClient.on('connection', handler)
}

export function onHitlError(handler) {
  return hitlClient.on('error', handler)
}

export default hitlClient
