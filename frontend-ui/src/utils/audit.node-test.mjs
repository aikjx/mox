import { test } from 'node:test'
import assert from 'node:assert/strict'
import { parseAuditPage } from './api-key.js'
import { validatePageSchema } from '../modules/admin-lowcode/contract/pageSchema.js'

test('audit contract preserves bounded total and unknown HTTP status', () => {
  const value = { items: [{ id: 'entry', statusCode: null }], total: 130, page: 1, page_size: 20 }
  assert.equal(parseAuditPage(value), value)
  assert.equal(parseAuditPage(value).items[0].statusCode, null)
  assert.throws(() => parseAuditPage([]))
  assert.throws(() => parseAuditPage({ ...value, page_size: 101 }))
})
test('read-only schema needs actual list capability without write stubs', () => {
  const schema = { key: 'audit', readOnly: true, api: { list: parseAuditPage }, list: { columns: [{ prop: 'id' }] } }
  assert.deepEqual(validatePageSchema(schema), [])
  assert.ok(validatePageSchema({ ...schema, readOnly: false }).some(error => error.includes('api.create')))
  assert.ok(validatePageSchema({ ...schema, readOnly: 'true' }).some(error => error.includes('readOnly')))
})
test('read-only schema rejects forms and declarative write entry points', () => {
  const schema = { key: 'audit', readOnly: true, api: { list: parseAuditPage }, list: { columns: [{ prop: 'id' }] } }
  for (const value of [
    { ...schema, form: { fields: [] } },
    { ...schema, toolbar: [{ label: 'create', action: 'create' }] },
    { ...schema, list: { ...schema.list, rowActions: [{ label: 'delete', action: 'delete' }] } },
  ]) assert.ok(validatePageSchema(value).some(error => error.includes('只读')))
})
