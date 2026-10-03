import { beforeAll, afterAll, it, expect } from 'vitest'
import { spawn } from 'node:child_process'
import { createRequire } from 'node:module'
import { mkdtemp, readFile, writeFile, unlink, rmdir } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import http from './http'
import { registerProjectIdGetter } from './http'
import { kbGetDocument, kbGetEntities, kbSearchEntities, kbLinkEntity, kbUnlinkEntity } from './kb.api'
import { kbEntityMutation } from '@/utils'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { useAuthStore } from '@/stores'
import KnowledgeEntityLinks from '../components/knowledge/KnowledgeEntityLinks.vue'
const nodeAxios = createRequire(import.meta.url)('axios')
let child, done, folder, identity, previousBase, previousAdapter
let stderr = ''
beforeAll(async () => {
  localStorage.clear()
  folder = await mkdtemp(path.join(tmpdir(), 'mox-kb-client-'))
  const ready = path.join(folder, 'ready.json')
  child = spawn('cargo', ['test', '-p', 'mox-platform-gateway-svc', '--test', 'kb_entity_gateway', '--', '--nocapture'], {
    cwd: path.resolve(process.cwd(), '..'),
    env: { ...process.env, MOX_KB_ENTITY_READY: ready, MOX_KB_ENTITY_STOP: path.join(folder, 'stop') },
    stdio: ['ignore', 'ignore', 'pipe']
  })
  child.stderr.on('data', chunk => { stderr = (stderr + chunk.toString()).slice(-8000) })
  done = new Promise((resolve, reject) => { child.on('error', reject); child.on('exit', resolve) })
  // Includes a cold Rust compilation; request and interaction deadlines remain separate.
  const deadline = Date.now() + 300000
  while (Date.now() < deadline) {
    try { identity = JSON.parse(await readFile(ready, 'utf8')); break } catch { /* Await atomic descriptor. */ }
    if (child.exitCode !== null) throw new Error(`Rust fixture exited: ${stderr}`)
    await new Promise(resolve => setTimeout(resolve, 100))
  }
  if (!identity) throw new Error(`Rust fixture readiness timed out: ${stderr}`)
  await unlink(ready)
  previousBase = http.defaults.baseURL
  previousAdapter = http.defaults.adapter
  http.defaults.baseURL = identity.base + '/api'
  http.defaults.adapter = nodeAxios.getAdapter('http')
  localStorage.setItem('mox_access_token', identity.token)
  setActivePinia(createPinia())
  const auth = useAuthStore()
  auth.accessToken = identity.token
  auth.userInfo = { id: 'alice', tenant_id: 'a', roles: ['user'], enabled: true }
  registerProjectIdGetter(() => 'test-selected-project')
}, 305000)
afterAll(async () => {
  localStorage.clear()
  registerProjectIdGetter(null)
  if (previousBase !== undefined) http.defaults.baseURL = previousBase
  if (previousAdapter !== undefined) http.defaults.adapter = previousAdapter
  if (!folder) return
  const stop = path.join(folder, 'stop')
  await writeFile(stop, 'stop')
  if (!identity && child?.exitCode === null) child.kill()
  if (child) expect(await done, stderr).toBe(0)
  await unlink(stop)
  await rmdir(folder)
}, 15000)

it('real Axios/JWT/FS search, association, stale conflict, delete and tenant isolation', async () => {
  const entities = await kbSearchEntities({ q: 'Rust', type: 'tech', limit: 1 })
  expect(entities).toHaveLength(1)
  const doc = await kbGetDocument(identity.target)
  const before = await kbGetEntities(doc.id)
  const request = kbEntityMutation(doc, entities[0], before.links_revision)
  const linked = await kbLinkEntity(doc.id, request)
  expect(linked.links_revision).toBe(1)
  expect(linked.linked_entities[0].source_doc_id).toBe(entities[0].source_doc_id)
  await expect(kbLinkEntity(doc.id, request)).rejects.toMatchObject({ response: { status: 409 } })
  const after = await kbUnlinkEntity(doc.id, kbEntityMutation(doc, linked.linked_entities[0], linked.links_revision))
  expect(after.links_revision).toBe(2)
  expect(after.linked_entities).toEqual([])
  expect(() => kbEntityMutation(doc, { id: 'invented' }, 2)).toThrow()
  localStorage.setItem('mox_access_token', identity.foreign_token)
  useAuthStore().accessToken = identity.foreign_token
  expect(await kbSearchEntities({ q: 'Rust' })).toEqual([])
  await expect(kbGetEntities(doc.id)).rejects.toMatchObject({ response: { status: 404 } })
}, 20000)

it('actual Vue component reads and mutates real KB associations', async () => {
  localStorage.setItem('mox_access_token', identity.token)
  localStorage.setItem('mox_user_info', JSON.stringify({ id: 'alice', tenant_id: 'a' }))
  const pinia = createPinia()
  setActivePinia(pinia)
  useAuthStore().accessToken = identity.token
  const doc = await kbGetDocument(identity.target)
  const wrapper = mount(KnowledgeEntityLinks, { props: { document: doc }, global: { plugins: [pinia] } })
  const waitFor = async predicate => {
    for (let i = 0; i < 60; i++) {
      if (predicate()) return
      await new Promise(resolve => setTimeout(resolve, 50))
    }
    throw new Error(wrapper.text())
  }
  try {
    await waitFor(() => wrapper.text().includes('暂无可展示'))
    await wrapper.get('input').setValue('Rust')
    await wrapper.get('form').trigger('submit')
    await waitFor(() => wrapper.find('ul[aria-label="实体搜索结果"] button').exists())
    await wrapper.get('ul[aria-label="实体搜索结果"] button').trigger('click')
    await waitFor(() => wrapper.find('ul[aria-label="已关联实体"] button').exists())
    expect((await kbGetEntities(doc.id)).linked_entities).toHaveLength(1)
    await wrapper.get('ul[aria-label="已关联实体"] button').trigger('click')
    await waitFor(() => !wrapper.find('ul[aria-label="已关联实体"] button').exists())
    expect((await kbGetEntities(doc.id)).linked_entities).toEqual([])
  } finally { wrapper.unmount() }
}, 20000)
