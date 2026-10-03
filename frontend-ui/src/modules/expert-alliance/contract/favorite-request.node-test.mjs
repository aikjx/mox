import test from 'node:test'
import assert from 'node:assert/strict'
import { createFavoriteRequest, favoriteRequestHeaders, favoriteRejectionIsDefinitive } from './favorite-request.js'

test('requests freeze a real UUID and stable expert identity', () => {
  const request = createFavoriteRequest('expert-a')
  assert.equal(request.expertId, 'expert-a')
  assert.match(request.key, /^[a-f0-9-]{36}$/)
  assert.ok(Object.isFrozen(request))
  assert.deepEqual(favoriteRequestHeaders(request.key), { 'Idempotency-Key': request.key })
  assert.notEqual(createFavoriteRequest('expert-a').key, request.key)
})
test('invalid identities and request keys fail before transport', () => {
  assert.throws(() => createFavoriteRequest(' '))
  for (const key of ['', '%invalid', 'x'.repeat(129), undefined]) assert.throws(() => favoriteRequestHeaders(key))
})
test('unknown errors retain attempts while explicit rejections release them', () => {
  for (const status of [400,401,403,404,409,413,422]) assert.ok(favoriteRejectionIsDefinitive({ status }))
  for (const status of [undefined,500,502,503,504]) assert.equal(favoriteRejectionIsDefinitive({ status }), false)
})
