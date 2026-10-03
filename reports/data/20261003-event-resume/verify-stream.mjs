import assert from 'node:assert/strict'
import { createAllianceEventStream } from '../../../frontend-ui/src/modules/expert-alliance/contract/event-stream.js'

const { MOX_PROBE_URL: url, MOX_PROBE_TOKEN: token, MOX_PROBE_CURSOR: cursor, MOX_PROBE_EXPECTED: expected } = process.env
assert.ok(url && token && cursor && expected)
let timer
let resolveFrame, rejectFrame
const frame = new Promise((resolve, reject) => { resolveFrame = resolve; rejectFrame = reject })
const stream = createAllianceEventStream({
  url, getToken: () => token,
  onEvent: (kind, envelope) => {
    try { assert.equal(kind, 'ExpertRegistered'); assert.equal(envelope.id, expected); resolveFrame() }
    catch (error) { rejectFrame(error) }
  }, onError: rejectFrame
})
const running = stream.start({ lastEventId: cursor })
try {
  await Promise.race([frame, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('durable frame did not reach frontend engine')), 2000) })])
} finally {
  clearTimeout(timer)
  stream.stop()
  await running
}
const errors = []
const unavailable = createAllianceEventStream({ url, getToken: () => token, onError: error => errors.push(error) })
await unavailable.start({ lastEventId: 'evt-missing' })
assert.equal(errors.length, 1)
assert.equal(errors[0].status, 410)
await unavailable.start({ lastEventId: 'x'.repeat(257) })
assert.equal(errors.length, 2)
assert.match(errors[1].message, /Last-Event-ID/)
console.log('REAL_FRONTEND_RESUME: replay frame, 410 recovery signal, invalid cursor rejection passed')
