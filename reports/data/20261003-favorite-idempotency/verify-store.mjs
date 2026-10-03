// Actual Vue/Pinia store, Axios transport and production Rust router through a TCP fault proxy.
import assert from 'node:assert/strict'
import http from 'node:http'
import { once } from 'node:events'
import { fileURLToPath } from 'node:url'
import { Window } from '../../../frontend-ui/node_modules/happy-dom/lib/index.js'
import { createServer } from '../../../frontend-ui/node_modules/vite/dist/node/index.js'
import { createPinia, setActivePinia } from '../../../frontend-ui/node_modules/pinia/dist/pinia.mjs'

const window = new Window()
globalThis.window = window
globalThis.document = window.document
globalThis.localStorage = window.localStorage
globalThis.sessionStorage = window.sessionStorage
let dropsRemaining = 1
let delayNext = false
let markDelayed, releaseDelayed
const delayed = new Promise(resolve => { markDelayed = resolve })
const release = new Promise(resolve => { releaseDelayed = resolve })
const keys = []
const proxy = http.createServer(async (req, res) => {
  try {
    keys.push(req.headers['idempotency-key'])
    const upstream = await fetch(process.env.FAVORITE_GATEWAY_URL + req.url, {
      method: req.method,
      headers: { Authorization: req.headers.authorization, 'Idempotency-Key': req.headers['idempotency-key'] }
    })
    const bytes = Buffer.from(await upstream.arrayBuffer())
    if (dropsRemaining > 0) { dropsRemaining--; res.destroy(); return }
    if (delayNext) { delayNext = false; markDelayed(); await release }
    res.writeHead(upstream.status, { 'Content-Type': 'application/json' })
    res.end(bytes)
  } catch (error) { res.destroy(error) }
})
proxy.listen(0, '127.0.0.1')
await once(proxy, 'listening')
const vite = await createServer({ root: fileURLToPath(new URL('../../../frontend-ui', import.meta.url)), server: { middlewareMode: true }, appType: 'custom' })
try {
  setActivePinia(createPinia())
  const { useAuthStore } = await vite.ssrLoadModule('/src/stores/auth.store.js')
  const { default: transport } = await vite.ssrLoadModule('/src/api/http.js')
  const { useAllianceExpertsStore } = await vite.ssrLoadModule('/src/modules/expert-alliance/store/alliance-experts.store.js')
  const auth = useAuthStore()
  auth.accessToken = process.env.FAVORITE_JWT
  auth.userInfo = { id: 'e2e-user', tenant_id: 'tenant-a' }
  transport.defaults.baseURL = `http://127.0.0.1:${proxy.address().port}/api`
  transport.defaults.adapter = 'http'
  const store = useAllianceExpertsStore()
  const result = await store.toggleFavorite({ id: 'e2e-shared-id', name: 'real expert' })
  assert.equal(result?.favorite, true, store.error.action)
  assert.equal(store.isFavorite('e2e-shared-id'), true)
  assert.equal(keys.length, 2)
  assert.equal(keys[0], keys[1], 'TCP failure retry must keep the original key')
  const next = await store.toggleFavorite({ id: 'e2e-shared-id', name: 'real expert' })
  assert.equal(next?.favorite, false, store.error.action)
  assert.notEqual(keys[2], keys[0])
  dropsRemaining = 3
  const unknown = await store.toggleFavorite({ id: 'e2e-shared-id', name: 'real expert' })
  assert.equal(unknown, null)
  assert.ok(store.error.action)
  assert.equal(store.isFavorite('e2e-shared-id'), false)
  assert.deepEqual(keys.slice(3, 6), Array(3).fill(keys[3]))
  const retry = await store.toggleFavorite({ id: 'e2e-shared-id', name: 'real expert' })
  assert.equal(retry?.favorite, true, store.error.action)
  assert.equal(keys[6], keys[3], 'manual retry must retain the uncertain attempt key')
  const cleared = await store.toggleFavorite({ id: 'e2e-shared-id', name: 'real expert' })
  assert.equal(cleared?.favorite, false)
  delayNext = true
  const pending = store.toggleFavorite({ id: 'e2e-shared-id', name: 'real expert' })
  await delayed
  auth.accessToken = process.env.FAVORITE_ROTATED_JWT
  releaseDelayed()
  assert.equal(await pending, null, 'old identity response must be ignored')
  const resumed = await store.toggleFavorite({ id: 'e2e-shared-id', name: 'real expert' })
  assert.equal(resumed?.favorite, true, store.error.action)
  assert.equal(keys[9], keys[8], 'same principal token rotation preserves an uncertain request key')
  auth.accessToken = ''
  assert.equal(store.favoriteCount, 0, 'identity change clears the local projection')
  const cleanup = await fetch(process.env.FAVORITE_GATEWAY_URL + '/api/experts/e2e-shared-id/favorite', {
    method: 'POST', headers: { Authorization: `Bearer ${process.env.FAVORITE_JWT}`, 'Idempotency-Key': 'probe-cleanup' }
  })
  assert.equal(cleanup.status, 200)
  assert.equal((await cleanup.json()).data.favorite, false)
  console.log('Actual Pinia/Axios/Rust HTTP chain passed: automatic and manual uncertain retries keep the same key; delayed old identity response ignored.')
} finally {
  await vite.close()
  proxy.closeAllConnections()
  await new Promise(resolve => proxy.close(resolve))
  window.happyDOM.abort()
}
