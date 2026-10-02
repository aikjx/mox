import { test } from 'node:test'
import assert from 'node:assert/strict'
import { parseApiKeyPage, apiKeyEligibilityLabel } from './api-key.js'
import { validatePageSchema } from '../modules/admin-lowcode/contract/pageSchema.js'

test('credential page accepts actual contract and preserves server totals', () => {
  const page = { items: [{ id: 'actual-id' }], total: 130, page: 7, page_size: 20 }
  assert.equal(parseApiKeyPage(page), page)
  assert.equal(parseApiKeyPage({ items: [], total: 130, page: 999, page_size: 20 }).total, 130)
})
test('credential page rejects legacy arrays and malformed bounded responses', () => {
  const valid = { items: [], total: 0, page: 1, page_size: 20 }
  for (const value of [[], null, {}, { ...valid, total: -1 }, { ...valid, total: '0' },
    { ...valid, page: 0 }, { ...valid, page_size: 101 }, { ...valid, items: [1] },
    { ...valid, total: 101, items: Array(21) }]) {
    assert.throws(() => parseApiKeyPage(value))
  }
})
test('configuration labels never turn unknown state into an authentication claim', () => {
  assert.equal(apiKeyEligibilityLabel('eligible'), '基础配置可用')
  assert.equal(apiKeyEligibilityLabel('expired'), '已过期')
  assert.equal(apiKeyEligibilityLabel('unsupported_scope'), '旧权限范围未支持')
  for (const value of ['toString', 'future_state', undefined, null]) {
    assert.equal(apiKeyEligibilityLabel(value), '未评估')
  }
})
test('identity boundary hooks reject schema configuration errors', () => {
  const schema = { key: 'access', api: { list: parseApiKeyPage, create: parseApiKeyPage, update: parseApiKeyPage, remove: parseApiKeyPage }, list: { columns: [{ prop: 'id' }] } }
  assert.deepEqual(validatePageSchema(schema), [])
  for (const key of ['identityScope', 'onIdentityChange']) {
    assert.deepEqual(validatePageSchema({ ...schema, [key]: parseApiKeyPage }), [])
    assert.ok(validatePageSchema({ ...schema, [key]: 'invalid-hook' }).some(error => error.includes(key)))
  }
})
