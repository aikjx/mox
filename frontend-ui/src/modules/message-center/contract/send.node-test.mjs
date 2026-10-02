import { test } from 'node:test'
import assert from 'node:assert/strict'
import { buildInAppPayload, createSendAttempt, readSendReceipt, isDefinitiveSendRejection } from './send.js'

const draft = { receivers: 'user-b, user-a\nuser-b', title: ' 标题 ', content: '正文\n  缩进', type: 'custom', priority: 'normal' }
test('command has canonical actual recipients and no request-provided identity', () => {
  const value = buildInAppPayload({ ...draft, sender_id: 'forged', tenant_id: 'foreign', channels: ['email'] })
  assert.deepEqual(value.receiver_ids, ['user-a', 'user-b'])
  assert.deepEqual(value.channels, ['in_app'])
  assert.equal(value.title, '标题')
  assert.equal(value.content, draft.content)
  assert.equal(value.sender_id, undefined)
  assert.equal(value.tenant_id, undefined)
})
test('invalid and oversized input is refused without a command', () => {
  for (const patch of [{ title: '' }, { content: ' ' }, { receivers: '' }, { type: 'fake' }, { priority: 'fake' },
    { title: '😀'.repeat(201) }, { content: 'x'.repeat(10001) }, { receivers: Array.from({ length: 101 }, (_, i) => `user-${i}`).join(',') }]) {
    assert.throws(() => buildInAppPayload({ ...draft, ...patch }))
  }
})
test('attempt freezes the original payload and owns an actual cryptographic key', () => {
  const editable = { ...draft }
  const first = createSendAttempt(editable)
  editable.content = 'different'
  assert.equal(first.payload.content, draft.content)
  assert.ok(Object.isFrozen(first.payload.receiver_ids))
  assert.throws(() => first.payload.receiver_ids.push('other'))
  assert.match(first.key, /^[0-9a-f-]{36}$/)
  assert.notEqual(createSendAttempt(draft).key, first.key)
})
test('success needs an actual nonempty receipt', () => {
  assert.equal(readSendReceipt({ message_id: 'msg-real' }), 'msg-real')
  for (const value of [null, {}, { success: true }, { message_id: ' ' }]) assert.throws(() => readSendReceipt(value))
})
test('network, timeout and storage errors remain uncertain', () => {
  for (const status of [undefined, 408, 429, 500, 502, 503, 504]) assert.equal(isDefinitiveSendRejection({ status }), false)
  for (const status of [400, 401, 403, 409, 501]) assert.equal(isDefinitiveSendRejection({ status }), true)
})
