import test from 'node:test'
import assert from 'node:assert/strict'
import http from 'node:http'
import { once } from 'node:events'
import { createAllianceEventStream, createEventFrameParser } from './event-stream.js'

test('parser preserves SSE whitespace, split CRLF and rejects invalid JSON', () => {
  const frames = []
  const parse = createEventFrameParser((...args) => frames.push(args))
  parse(': heartbeat\r\nevent: PlanCreated\r\ndata: {"title":"  中文  "}\r')
  parse('\n\r\n')
  assert.equal(frames[0][1].title, '  中文  ')
  assert.throws(() => parse('event: PlanCreated\ndata: broken\n\n'), /JSON/)
})

test('parser bounds incomplete frames and handles multiline data and bare CR', () => {
  const frames = []
  const parse = createEventFrameParser((...args) => frames.push(args), 80)
  parse('event: StreamGap\rdata: {"reason":\rdata: "lagged"}\r\r')
  assert.equal(frames[0][1].reason, 'lagged')
  assert.throws(() => parse('x'.repeat(81)), /limit/)
})

test('real TCP subscription uses bearer, replaces previous connection and aborts reads', async () => {
  const requests = []
  const server = http.createServer((req, res) => {
    requests.push({ url: req.url, auth: req.headers.authorization })
    res.writeHead(200, { 'Content-Type': 'text/event-stream' })
    res.write('event: PlanCreated\ndata: {"id":"real-event"}\n\n')
  })
  server.listen(0, '127.0.0.1')
  await once(server, 'listening')
  const frames = []
  const errors = []
  const stream = createAllianceEventStream({
    url: `http://127.0.0.1:${server.address().port}/api/alliance/events/stream`,
    getToken: () => 'real-session-token',
    onEvent: (...args) => frames.push(args), onError: e => errors.push(e)
  })
  try {
    const first = stream.start()
    while (frames.length < 1) await new Promise(resolve => setTimeout(resolve, 10))
    const second = stream.start()
    await first
    while (frames.length < 2) await new Promise(resolve => setTimeout(resolve, 10))
    stream.stop()
    await second
    assert.deepEqual(requests, Array(2).fill({ url: '/api/alliance/events/stream', auth: 'Bearer real-session-token' }))
    assert.equal(errors.length, 0)
  } finally {
    stream.stop()
    server.closeAllConnections()
    await new Promise(resolve => server.close(resolve))
  }
})

test('missing identity rejects before opening a connection', async () => {
  const errors = []
  const stream = createAllianceEventStream({ url: 'http://127.0.0.1:1', getToken: () => '', onError: e => errors.push(e) })
  await stream.start()
  assert.match(errors[0].message, /登录/)
})
