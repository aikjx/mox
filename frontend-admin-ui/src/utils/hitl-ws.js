const HITL_WS_PATH = '/ws/hitl'

const ACTIONS = {
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
      console.log('[HITL WS] Connected')
      this._emit('connection', { status: 'connected' })
    }

    this.ws.onmessage = (event) => {
      this._handleMessage(event.data)
    }

    this.ws.onerror = (err) => {
      console.error('[HITL WS] Error:', err)
      this._emit('connection', { status: 'error', error: err })
    }

    this.ws.onclose = () => {
      console.warn('[HITL WS] Disconnected')
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
    handlers.forEach(fn => {
      try {
        fn(payload)
      } catch (err) {
        console.error('[HITL WS] Listener error:', err)
      }
    })
  }

  subscribe(eventType, handler) {
    if (typeof handler !== 'function') return () => {}
    const handlers = this.listeners.get(eventType) || []
    handlers.push(handler)
    this.listeners.set(eventType, handlers)
    return () => this.unsubscribe(eventType, handler)
  }

  unsubscribe(eventType, handler) {
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

  send(action, payload) {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      console.warn('[HITL WS] Cannot send, socket not connected')
      return false
    }
    const message = { action, ...payload }
    this.ws.send(JSON.stringify(message))
    return true
  }

  sendAction(eventId, action, modifiedPayload = null) {
    if (!Object.values(ACTIONS).includes(action)) {
      console.warn('[HITL WS] Invalid action:', action)
      return false
    }
    const payload = { eventId, action }
    if (modifiedPayload !== null && modifiedPayload !== undefined) {
      payload.modifiedPayload = modifiedPayload
    }
    return this.send('hitl_action', payload)
  }

  isConnected() {
    return !!(this.ws && this.ws.readyState === WebSocket.OPEN)
  }
}

export const hitlClient = new HitlWebSocketClient()

export const hitlActions = ACTIONS

export function onHitlEvent(handler) {
  return hitlClient.subscribe('hitl_event', handler)
}

export function onHitlStatus(handler) {
  return hitlClient.subscribe('hitl_status', handler)
}

export function onHitlConnection(handler) {
  return hitlClient.subscribe('connection', handler)
}

export default hitlClient
