// Actual Pinia + Axios + production Rust HTTP routes. A TCP proxy delays an actual response.
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
let delayNext = false
let markDelayed, releaseDelayed
const delayed = new Promise(resolve => { markDelayed = resolve })
const release = new Promise(resolve => { releaseDelayed = resolve })
const calls = []
const proxy = http.createServer(async (req, res) => {
  try {
    calls.push({ method: req.method, path: req.url })
    const parts = []
    for await (const part of req) parts.push(part)
    const upstream = await fetch(process.env.WEBHOOK_GATEWAY_URL + req.url, {
      method: req.method,
      headers: { Authorization: req.headers.authorization, 'Content-Type': 'application/json' },
      body: req.method === 'GET' ? undefined : Buffer.concat(parts)
    })
    const bytes = Buffer.from(await upstream.arrayBuffer())
    if (delayNext && req.method === 'GET') { delayNext = false; markDelayed(); await release }
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
  const { useAllianceWebhooksStore } = await vite.ssrLoadModule('/src/modules/expert-alliance/store/alliance-webhooks.store.js')
  const auth = useAuthStore()
  const identify = (jwt, tenant, roles = ['tenant_admin']) => {
    auth.accessToken = jwt
    auth.userInfo = { id: 'e2e-user', tenant_id: tenant, roles }
  }
  identify(process.env.WEBHOOK_ADMIN_JWT, 'a')
  transport.defaults.baseURL = `http://127.0.0.1:${proxy.address().port}/api`
  transport.defaults.adapter = 'http'
  const store = useAllianceWebhooksStore()
  assert.equal(await store.load(), true, store.error)
  assert.equal(store.total, 0)
  const input = { url: `${process.env.WEBHOOK_ORIGIN}/receive`, eventTypes: ['ExpertRegistered'] }
  const created = await store.create(input)
  assert.ok(created, store.error)
  assert.equal(created.tenantId, 'a')
  assert.equal(store.items[0].id, created.id)
  assert.equal(store.total, 1)
  delayNext = true
  const stale = store.load()
  await delayed
  identify(process.env.WEBHOOK_B_JWT, 'b')
  assert.equal(store.items.length, 0, 'tenant switch must synchronously clear old addresses')
  assert.equal(await store.load(), true, store.error)
  assert.equal(store.total, 0)
  releaseDelayed()
  assert.equal(await stale, false)
  assert.equal(store.items.length, 0, 'late tenant A payload must not reach tenant B projection')
  assert.equal(await store.remove(created.id), null, 'cross-tenant HTTP DELETE must be rejected')
  assert.ok(store.error)
  // Forge local UI role while keeping a genuine non-admin JWT: server must remain authoritative.
  identify(process.env.WEBHOOK_NORMAL_JWT, 'a')
  assert.equal(await store.create(input), null)
  assert.ok(store.error)
  assert.equal(await store.load(), null)
  assert.equal(store.loaded, false)
  identify(process.env.WEBHOOK_ADMIN_JWT, 'a')
  assert.equal(await store.load(), true)
  assert.equal(store.total, 1)
  assert.equal(await store.remove(created.id), true, store.error)
  assert.equal(store.total, 0)
  identify('', 'a', [])
  const before = calls.length
  assert.equal(await store.create(input), null)
  assert.equal(await store.load(), null)
  assert.equal(calls.length, before, 'unauthenticated frontend sends no subscriptions request')
  console.log(JSON.stringify({ realFrontend: true, transport: 'Axios HTTP to Rust through TCP proxy', requests: calls.length, cases: ['CRUD', 'tenant isolation', 'late response isolation', 'server RBAC despite forged UI role', 'logout blocks requests'], browserE2E: false }))
} finally {
  await vite.close()
  proxy.closeAllConnections()
  await new Promise(resolve => proxy.close(resolve))
  window.happyDOM.abort()
}
