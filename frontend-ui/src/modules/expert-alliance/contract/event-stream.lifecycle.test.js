// T4 SSE 传输引擎（contract/event-stream.js）的连接 / 解析 / 断开生命周期：
// 用一个喂真实后端 SSE 帧形状的假 Response 证明——start 真发 fetch、帧真解析进 onEvent、
// stop 真 abort；无 token 时连都不开。不依赖真实后端 / 浏览器 EventSource。
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createAllianceEventStream } from './event-stream.js'

// 真实后端线形状（experts_streams.rs:82 `Event::event(name).data(data)`）：
// event:<Kind> 与 data:<信封JSON> 各一行，帧间空行分隔。
const FRAME = 'event:PlanStatusChanged\n'
  + 'data:{"id":"evt-1","type":"PlanStatusChanged","plan_id":"plan-9","from":"draft","to":"running","execution_id":"exec-1","source":"execute_plan_handler","tenant":"tenant-a","occurred_at":"2026-10-02T10:00:00Z"}\n\n'
// 15s KeepAlive 心跳是 `:` 注释行（axum KeepAlive），无 data，不该产生事件。
const HEARTBEAT = ': ping\n\n'

function fakeResponse(texts) {
  const enc = new TextEncoder()
  const chunks = texts.map((t) => enc.encode(t))
  let i = 0
  const reader = {
    read: async () => (i < chunks.length ? { done: false, value: chunks[i++] } : { done: true, value: undefined }),
    cancel: vi.fn(async () => {}),
    releaseLock: vi.fn(() => {})
  }
  return {
    ok: true,
    body: { getReader: () => reader },
    headers: { get: (n) => (n.toLowerCase() === 'content-type' ? 'text/event-stream; charset=utf-8' : null) },
    __reader: reader
  }
}

const flushed = () => new Promise((r) => setTimeout(r, 5))

beforeEach(() => {
  vi.stubGlobal('fetch', vi.fn())
})

describe('T4 SSE 传输引擎', () => {
  it('start 真发 fetch（带 Bearer + event-stream 头），把真实帧解析进 onEvent', async () => {
    const res = fakeResponse([HEARTBEAT, FRAME])
    fetch.mockResolvedValue(res)
    const events = []
    const errors = []

    const stream = createAllianceEventStream({
      url: '/api/alliance/events/stream',
      getToken: () => 'real-token',
      onEvent: (kind, envelope) => events.push({ kind, envelope }),
      onError: (e) => errors.push(e)
    })
    const done = stream.start()
    while (events.length < 1) await flushed()
    stream.stop()
    await done

    expect(fetch).toHaveBeenCalledTimes(1)
    const [url, init] = fetch.mock.calls[0]
    expect(url).toBe('/api/alliance/events/stream')
    expect(init.headers.Authorization).toBe('Bearer real-token')
    expect(init.headers.Accept).toBe('text/event-stream')

    // 心跳注释帧被丢弃，只有真实业务帧进 onEvent，且字段是后端信封原貌
    expect(errors).toHaveLength(0)
    expect(events).toHaveLength(1)
    expect(events[0].kind).toBe('PlanStatusChanged')
    expect(events[0].envelope).toMatchObject({
      id: 'evt-1', plan_id: 'plan-9', from: 'draft', to: 'running', execution_id: 'exec-1'
    })
  })

  it('stop 真断连：cancel 读端、退出读循环', async () => {
    const enc = new TextEncoder()
    // 第一帧立即给；之后读端挂起（模拟真实 SSE 空闲等待），直到被 stop cancel。
    let releasePending = null
    const pending = new Promise((r) => { releasePending = r })
    const reader = {
      read: vi.fn(async () => {
        if (reader.read.mock.calls.length === 1) return { done: false, value: enc.encode(FRAME) }
        await pending // 永远不自然完成，只能被 stop 打断
        return { done: true, value: undefined }
      }),
      cancel: vi.fn(async () => {}),
      releaseLock: vi.fn(() => {})
    }
    const res = {
      ok: true,
      body: { getReader: () => reader },
      headers: { get: (n) => (n.toLowerCase() === 'content-type' ? 'text/event-stream' : null) }
    }
    fetch.mockResolvedValue(res)

    const events = []
    const stream = createAllianceEventStream({
      url: '/api/alliance/events/stream',
      getToken: () => 'real-token',
      onEvent: (kind) => events.push(kind)
    })
    const done = stream.start()
    while (events.length < 1) await flushed()
    stream.stop()
    releasePending() // 让挂起的 read 返回，走 finally cancel
    await done
    // stop 后读端被 cancel、start() 的 promise 收尾（不再挂在流上）
    expect(reader.cancel).toHaveBeenCalled()
    expect(events).toEqual(['PlanStatusChanged'])
  })

  it('无 token 时不发任何连接（先身份后网络）', async () => {
    const errors = []
    const stream = createAllianceEventStream({
      url: '/api/alliance/events/stream',
      getToken: () => '',
      onError: (e) => errors.push(e)
    })
    await stream.start()
    expect(fetch).not.toHaveBeenCalled()
    expect(errors[0].message).toMatch(/登录/)
  })
})
