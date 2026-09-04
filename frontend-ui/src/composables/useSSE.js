/**
 * 通用 SSE (Server-Sent Events) Composable
 *
 * 职责：
 * - 统一管理 SSE 连接生命周期（连接/断开/重连）
 * - 解析 SSE 事件格式（event: / data: / id:）
 * - 指数退避重连策略
 * - 超时检测与心跳保活
 * - 消息队列与事件分发
 *
 * 使用方式：
 * ```js
 * const { connect, disconnect, isConnected, lastEvent, events } = useSSE({
 *   url: '/api/alliance/stream',
 *   method: 'POST',
 *   body: { query: '...' },
 *   onEvent: (event) => console.log(event),
 *   onError: (err) => console.error(err),
 *   maxRetries: 3,
 *   timeoutMs: 30000,
 * })
 * connect()
 * ```
 */
import { ref, computed, onUnmounted } from 'vue'

/**
 * SSE 连接状态
 * @readonly
 * @enum {string}
 */
export const SSEState = {
  IDLE: 'idle',           // 未连接
  CONNECTING: 'connecting', // 连接中
  CONNECTED: 'connected',   // 已连接
  RECONNECTING: 'reconnecting', // 重连中
  CLOSED: 'closed',         // 已关闭
  ERROR: 'error',           // 错误
}

/**
 * 创建 SSE 连接
 *
 * @param {Object} options - 配置选项
 * @param {string} options.url - SSE 端点 URL
 * @param {string} [options.method='GET'] - HTTP 方法（GET 或 POST）
 * @param {Object} [options.body] - POST 请求体（method=POST 时使用）
 * @param {Object} [options.headers] - 自定义请求头
 * @param {Function} [options.onEvent] - 事件回调 (event: SSEEvent) => void
 * @param {Function} [options.onError] - 错误回调 (error: Error) => void
 * @param {Function} [options.onOpen] - 连接打开回调
 * @param {Function} [options.onClose] - 连接关闭回调
 * @param {number} [options.maxRetries=3] - 最大重连次数
 * @param {number} [options.baseDelayMs=1000] - 重连基础延迟（指数退避）
 * @param {number} [options.maxDelayMs=30000] - 重连最大延迟
 * @param {number} [options.timeoutMs=60000] - 连接超时（毫秒）
 * @param {number} [options.heartbeatIntervalMs=0] - 心跳间隔（0=禁用）
 * @param {boolean} [options.autoReconnect=true] - 是否自动重连
 * @returns {Object} SSE 控制器
 */
export function useSSE(options = {}) {
  const {
    url,
    method = 'GET',
    body = null,
    headers = {},
    onEvent = null,
    onError = null,
    onOpen = null,
    onClose = null,
    maxRetries = 3,
    baseDelayMs = 1000,
    maxDelayMs = 30000,
    timeoutMs = 60000,
    heartbeatIntervalMs = 0,
    autoReconnect = true,
  } = options

  // ===== 状态 =====
  const state = ref(SSEState.IDLE)
  const isConnected = computed(() => state.value === SSEState.CONNECTED)
  const isConnecting = computed(() =>
    state.value === SSEState.CONNECTING || state.value === SSEState.RECONNECTING
  )
  const retryCount = ref(0)
  const lastEventId = ref(null)
  const lastEvent = ref(null)
  const events = ref([])
  const error = ref(null)

  // ===== 内部变量 =====
  let eventSource = null
  let abortController = null
  let timeoutTimer = null
  let heartbeatTimer = null
  let reconnectTimer = null
  let buffer = ''
  let currentEvent = { event: 'message', data: '', id: null }
  let isManuallyClosed = false

  // ===== 工具函数 =====

  /**
   * 计算指数退避延迟
   */
  function calcBackoffDelay(attempt) {
    const delay = baseDelayMs * Math.pow(2, attempt)
    // 添加随机抖动（±20%）
    const jitter = delay * (Math.random() * 0.4 - 0.2)
    return Math.min(delay + jitter, maxDelayMs)
  }

  /**
   * 清除所有定时器
   */
  function clearTimers() {
    if (timeoutTimer) {
      clearTimeout(timeoutTimer)
      timeoutTimer = null
    }
    if (heartbeatTimer) {
      clearInterval(heartbeatTimer)
      heartbeatTimer = null
    }
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
  }

  /**
   * 重置连接状态
   */
  function resetState() {
    buffer = ''
    currentEvent = { event: 'message', data: '', id: null }
    error.value = null
  }

  /**
   * 解析 SSE 原始数据块
   * SSE 格式：
   *   event: event-name\n
   *   data: data-line-1\n
   *   data: data-line-2\n
   *   id: event-id\n
   *   \n  (空行表示事件结束)
   */
  function parseSSEChunk(chunk) {
    buffer += chunk
    const parsedEvents = []

    // 按空行分割事件
    let boundary
    while ((boundary = buffer.indexOf('\n\n')) !== -1) {
      const rawEvent = buffer.slice(0, boundary)
      buffer = buffer.slice(boundary + 2)

      const event = { event: 'message', data: '', id: lastEventId.value }
      const dataLines = []

      for (const line of rawEvent.split('\n')) {
        if (line.startsWith(':')) {
          // 注释行（心跳），忽略
          continue
        }
        const colonIndex = line.indexOf(':')
        if (colonIndex === -1) {
          // 没有冒号，整行作为字段名，值为空
          event[line.trim()] = ''
        } else {
          const field = line.slice(0, colonIndex).trim()
          const value = line.slice(colonIndex + 1).trim()
          if (field === 'event') {
            event.event = value
          } else if (field === 'data') {
            dataLines.push(value)
          } else if (field === 'id') {
            event.id = value
          } else if (field === 'retry') {
            // 客户端可以忽略，使用自己的重连策略
          } else {
            event[field] = value
          }
        }
      }

      event.data = dataLines.join('\n')

      // 尝试解析 JSON 数据
      if (event.data) {
        try {
          event.payload = JSON.parse(event.data)
        } catch {
          event.payload = event.data
        }
      }

      parsedEvents.push(event)
    }

    return parsedEvents
  }

  /**
   * 分发事件
   */
  function dispatchEvent(event) {
    lastEvent.value = event
    if (event.id) {
      lastEventId.value = event.id
    }
    events.value.push(event)
    // 限制事件队列长度，防止内存泄漏
    if (events.value.length > 1000) {
      events.value = events.value.slice(-500)
    }
    if (onEvent) {
      try {
        onEvent(event)
      } catch (e) {
        console.warn('[useSSE] onEvent callback error:', e)
      }
    }
  }

  // ===== 核心方法 =====

  /**
   * 建立 SSE 连接
   * 支持 GET（EventSource）和 POST（fetch + ReadableStream）两种方式
   */
  async function connect() {
    if (isConnecting.value || isConnected.value) {
      console.warn('[useSSE] Already connected or connecting')
      return
    }

    isManuallyClosed = false
    resetState()
    state.value = SSEState.CONNECTING

    // 设置连接超时
    timeoutTimer = setTimeout(() => {
      handleError(new Error(`SSE connection timeout after ${timeoutMs}ms`))
    }, timeoutMs)

    try {
      if (method.toUpperCase() === 'GET') {
        await connectWithEventSource()
      } else {
        await connectWithFetch()
      }
    } catch (e) {
      handleError(e)
    }
  }

  /**
   * 使用 EventSource API 连接（仅支持 GET）
   */
  function connectWithEventSource() {
    return new Promise((resolve, reject) => {
      const urlWithParams = new URL(url, window.location.origin)
      if (lastEventId.value) {
        urlWithParams.searchParams.set('Last-Event-ID', lastEventId.value)
      }

      eventSource = new EventSource(urlWithParams.toString(), {
        withCredentials: true,
      })

      eventSource.onopen = () => {
        clearTimeout(timeoutTimer)
        state.value = SSEState.CONNECTED
        retryCount.value = 0
        startHeartbeat()
        if (onOpen) onOpen()
        resolve()
      }

      eventSource.onmessage = (e) => {
        dispatchEvent({
          event: 'message',
          data: e.data,
          id: e.lastEventId || null,
          payload: safeParseJSON(e.data),
        })
      }

      eventSource.onerror = (e) => {
        if (eventSource.readyState === EventSource.CLOSED) {
          handleError(new Error('SSE connection closed'))
        } else {
          // EventSource 会自动重连，这里只记录
          console.warn('[useSSE] EventSource error, will auto-reconnect')
        }
        reject(e)
      }

      // 支持自定义事件类型
      eventSource.addEventListener('phase_started', (e) => {
        dispatchEvent({ event: 'phase_started', data: e.data, payload: safeParseJSON(e.data) })
      })
      eventSource.addEventListener('phase_data', (e) => {
        dispatchEvent({ event: 'phase_data', data: e.data, payload: safeParseJSON(e.data) })
      })
      eventSource.addEventListener('progress', (e) => {
        dispatchEvent({ event: 'progress', data: e.data, payload: safeParseJSON(e.data) })
      })
      eventSource.addEventListener('complete', (e) => {
        dispatchEvent({ event: 'complete', data: e.data, payload: safeParseJSON(e.data) })
        // 完成后自动关闭
        disconnect()
      })
      eventSource.addEventListener('error', (e) => {
        dispatchEvent({ event: 'error', data: e.data, payload: safeParseJSON(e.data) })
      })
    })
  }

  /**
   * 使用 fetch + ReadableStream 连接（支持 POST 和自定义头）
   */
  async function connectWithFetch() {
    abortController = new AbortController()

    const requestHeaders = {
      'Accept': 'text/event-stream',
      'Cache-Control': 'no-cache',
      ...headers,
    }

    // 自动添加认证 token
    const token = getAuthToken()
    if (token) {
      requestHeaders['Authorization'] = `Bearer ${token}`
    }

    const response = await fetch(url, {
      method: method.toUpperCase(),
      headers: requestHeaders,
      body: body ? JSON.stringify(body) : undefined,
      signal: abortController.signal,
    })

    if (!response.ok) {
      throw new Error(`SSE HTTP error: ${response.status} ${response.statusText}`)
    }

    const contentType = response.headers.get('content-type') || ''
    if (!contentType.includes('text/event-stream') && !contentType.includes('application/x-ndjson')) {
      throw new Error(`Unexpected content-type: ${contentType}`)
    }

    clearTimeout(timeoutTimer)
    state.value = SSEState.CONNECTED
    retryCount.value = 0
    startHeartbeat()
    if (onOpen) onOpen()

    const reader = response.body.getReader()
    const decoder = new TextDecoder('utf-8')

    try {
      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        const chunk = decoder.decode(value, { stream: true })
        const parsedEvents = parseSSEChunk(chunk)

        for (const event of parsedEvents) {
          dispatchEvent(event)
          // 完成事件自动关闭
          if (event.event === 'complete' || event.event === 'done') {
            disconnect()
            return
          }
        }
      }
    } catch (e) {
      if (e.name === 'AbortError') {
        // 主动取消，不触发重连
        return
      }
      throw e
    } finally {
      reader.releaseLock()
    }

    // 正常结束
    if (!isManuallyClosed) {
      state.value = SSEState.CLOSED
      if (onClose) onClose()
    }
  }

  /**
   * 断开连接
   */
  function disconnect() {
    isManuallyClosed = true
    clearTimers()

    if (eventSource) {
      eventSource.close()
      eventSource = null
    }
    if (abortController) {
      abortController.abort()
      abortController = null
    }

    state.value = SSEState.CLOSED
    if (onClose) onClose()
  }

  /**
   * 手动重连
   */
  function reconnect() {
    disconnect()
    retryCount.value = 0
    setTimeout(() => connect(), 100)
  }

  /**
   * 错误处理
   */
  function handleError(err) {
    clearTimers()
    error.value = err
    state.value = SSEState.ERROR

    if (onError) {
      try {
        onError(err)
      } catch (e) {
        console.warn('[useSSE] onError callback error:', e)
      }
    }

    // 自动重连
    if (autoReconnect && !isManuallyClosed && retryCount.value < maxRetries) {
      retryCount.value++
      const delay = calcBackoffDelay(retryCount.value - 1)
      state.value = SSEState.RECONNECTING
      console.warn(`[useSSE] Reconnecting in ${Math.round(delay)}ms (attempt ${retryCount.value}/${maxRetries})`)
      reconnectTimer = setTimeout(() => {
        if (!isManuallyClosed) {
          connect()
        }
      }, delay)
    } else if (retryCount.value >= maxRetries) {
      console.error(`[useSSE] Max retries (${maxRetries}) exceeded`)
      state.value = SSEState.CLOSED
    }
  }

  /**
   * 启动心跳保活
   */
  function startHeartbeat() {
    if (heartbeatIntervalMs <= 0) return
    heartbeatTimer = setInterval(() => {
      // 心跳通过发送注释行实现，这里只检测是否有数据
      // 如果需要主动心跳，可以在这里发送 ping
    }, heartbeatIntervalMs)
  }

  // ===== 辅助函数 =====

  function safeParseJSON(str) {
    try {
      return JSON.parse(str)
    } catch {
      return str
    }
  }

  function getAuthToken() {
    try {
      return localStorage.getItem('mox_token') || null
    } catch {
      return null
    }
  }

  // ===== 清理 =====
  onUnmounted(() => {
    disconnect()
  })

  // ===== 返回 =====
  return {
    // 状态
    state,
    isConnected,
    isConnecting,
    retryCount,
    lastEventId,
    lastEvent,
    events,
    error,
    // 方法
    connect,
    disconnect,
    reconnect,
    // 常量
    SSEState,
  }
}

export default useSSE
