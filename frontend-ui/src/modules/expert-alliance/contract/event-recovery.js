import { createAllianceEventStream } from './event-stream.js'

export const EVENT_CONNECTION = Object.freeze({
  IDLE: 'idle', CONNECTING: 'connecting', REFRESHING: 'refreshing',
  LIVE: 'live', DISCONNECTED: 'disconnected', ERROR: 'error'
})

// One transport, one coalesced refresh worker, one in-memory cursor per identity.
export function createAllianceEventRecovery({ url, getToken, refresh, onEvent, onState } = {}) {
  if (typeof refresh !== 'function') throw new Error('事件恢复必须提供权威刷新函数')
  let epoch = 0, connected = false, cursor = '', refreshJob = null, refreshPending = false
  const publish = (state, message = '') => onState?.({ state, message })

  function refreshCurrent() {
    refreshPending = true
    if (refreshJob?.epoch === epoch) return refreshJob.promise
    const job = { epoch }
    refreshJob = job
    job.promise = (async () => {
      while (refreshPending && job.epoch === epoch && connected) {
        refreshPending = false
        publish(EVENT_CONNECTION.REFRESHING)
        try { await refresh() }
        catch (error) {
          if (job.epoch === epoch && connected) {
            publish(EVENT_CONNECTION.ERROR, error?.message || '页面数据刷新失败，请重试连接或手动刷新')
          }
          return
        }
        if (job.epoch === epoch && connected) publish(EVENT_CONNECTION.LIVE)
      }
    })().finally(() => { if (refreshJob === job) refreshJob = null })
    return job.promise
  }

  const transport = createAllianceEventStream({
    url, getToken,
    onOpen: () => { connected = true; void refreshCurrent() },
    onEvent: (kind, envelope, raw) => {
      onEvent?.(kind, envelope, raw)
      if (typeof envelope?.id === 'string' && /^[\x21-\x7e]{1,256}$/.test(envelope.id)) cursor = envelope.id
    },
    onGap: () => { cursor = ''; void refreshCurrent() },
    onClose: () => { connected = false; publish(EVENT_CONNECTION.DISCONNECTED, '实时连接已结束，请重新连接；页面数据仍可手动刷新') },
    onError: error => {
      connected = false
      if ([409, 410, 401, 403].includes(error?.status)) cursor = ''
      const message = [409, 410].includes(error?.status)
        ? '历史续传不可用，请重新连接并刷新页面数据'
        : [401, 403].includes(error?.status)
          ? '当前身份无法订阅，请重新登录或检查权限'
          : '实时更新中断，请重试连接；页面数据仍可手动刷新'
      publish(EVENT_CONNECTION.ERROR, message)
    }
  })

  function start() {
    epoch++; connected = false; refreshPending = false
    publish(EVENT_CONNECTION.CONNECTING)
    return transport.start(cursor ? { lastEventId: cursor } : {})
  }
  function stop() {
    epoch++; connected = false; cursor = ''; refreshPending = false
    transport.stop()
    publish(EVENT_CONNECTION.IDLE, '实时更新已停止，可重新连接')
  }
  return { start, stop }
}
