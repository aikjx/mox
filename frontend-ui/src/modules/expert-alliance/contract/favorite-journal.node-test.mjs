import assert from 'node:assert/strict'
import test from 'node:test'
import { Window } from '../../../../node_modules/happy-dom/lib/index.js'
import { favoriteJournalScope, readFavoriteJournal, writeFavoriteJournal } from './favorite-journal.js'
import { createFavoriteRequest } from './favorite-request.js'

test('real sessionStorage retains immutable pending keys and isolates tenant/user namespaces', () => {
  const window = new Window()
  const storage = window.sessionStorage
  const a = favoriteJournalScope({ id: 'user', tenant_id: 'a' })
  const b = favoriteJournalScope({ id: 'user', tenant_id: 'b' })
  const other = favoriteJournalScope({ id: 'another', tenant_id: 'a' })
  const attempt = createFavoriteRequest('expert')
  writeFavoriteJournal(storage, a, new Map([['expert', attempt]]))
  const restored = readFavoriteJournal(storage, a)
  assert.equal(restored.get('expert').key, attempt.key)
  assert.equal(Object.isFrozen(restored.get('expert')), true)
  assert.equal(readFavoriteJournal(storage, b).size, 0)
  assert.equal(readFavoriteJournal(storage, other).size, 0)
  writeFavoriteJournal(storage, a, new Map())
  assert.equal(storage.getItem(a), null)
  window.happyDOM.abort()
})

test('corrupt, duplicate and oversized journals are rejected without rewriting storage', () => {
  const window = new Window()
  const storage = window.sessionStorage
  const scope = favoriteJournalScope({ id: 'user', tenant_id: 'a' })
  const attempt = createFavoriteRequest('expert')
  assert.throws(() => createFavoriteRequest('界'.repeat(86)))
  for (const raw of ['{bad', '{}', JSON.stringify([{ expertId: 'expert', key: 'bad' }]), JSON.stringify([{ expertId: '界'.repeat(86), key: attempt.key }]), JSON.stringify([attempt, attempt]), 'x'.repeat(100001)]) {
    storage.setItem(scope, raw)
    assert.throws(() => readFavoriteJournal(storage, scope))
    assert.equal(storage.getItem(scope), raw)
  }
  assert.throws(() => favoriteJournalScope({ id: 'user' }))
  assert.throws(() => writeFavoriteJournal(storage, scope, new Map(Array.from({ length: 101 }, (_, i) => [String(i), createFavoriteRequest(String(i))]))))
  window.happyDOM.abort()
})
