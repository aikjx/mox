import assert from 'node:assert/strict'
import http from 'node:http'
import { once } from 'node:events'
import { readFile } from 'node:fs/promises'
import { existsSync, readdirSync, appendFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { build } from '../../../frontend-ui/node_modules/vite/dist/node/index.js'
import vue from '../../../frontend-ui/node_modules/@vitejs/plugin-vue/dist/index.mjs'
import { chromium, expect } from '../../../frontend-ui/node_modules/@playwright/test/index.mjs'
const root = fileURLToPath(new URL('../../../', import.meta.url))
const progress = message => appendFileSync(new URL('./browser-progress.log', import.meta.url), `${new Date().toISOString()} ${message}\n`)
const frontend = path.join(root, 'frontend-ui')
const built = path.join(frontend, 'node_modules/.kb-entity-build')
const { MOX_KB_BASE: base, MOX_KB_TOKEN: token, MOX_KB_OTHER: other, MOX_KB_TARGET: target } = process.env
assert.ok(base && token && other && target, 'real Rust fixture descriptor required')
await build({ configFile: false, root, plugins: [vue()], resolve: { alias: {
  '@': frontend + '/src', vue: frontend + '/node_modules/vue/dist/vue.esm-bundler.js', pinia: frontend + '/node_modules/pinia/dist/pinia.mjs'
}}, build: { outDir: built, emptyOutDir: true, rollupOptions: { input: root + 'reports/html/20261004-kb-entity-normalization/index.html' } } })
// Forward actual Rust responses unchanged. No API substitution or fabricated entities.
const server = http.createServer(async (req, res) => {
  if (req.url.startsWith('/api/')) {
    const upstream = http.request(base + req.url, { method: req.method, headers: req.headers }, response => {
      res.writeHead(response.statusCode, response.headers); response.pipe(res)
    })
    upstream.on('error', () => res.destroy()); res.on('close', () => upstream.destroy()); req.pipe(upstream); return
  }
  const file = path.resolve(built, '.' + decodeURIComponent(new URL(req.url, 'http://localhost').pathname))
  if (!file.startsWith(built + path.sep)) { res.writeHead(403); res.end(); return }
  try {
    const bytes = await readFile(file)
    res.setHeader('Content-Type', file.endsWith('.js') ? 'text/javascript' : file.endsWith('.css') ? 'text/css' : 'text/html; charset=utf-8')
    res.end(bytes)
  } catch { res.writeHead(404); res.end() }
})
let browser
try {
  server.listen(0, '127.0.0.1'); await once(server, 'listening')
  let executablePath = process.env.MOX_PROBE_CHROMIUM || chromium.executablePath()
  if (!existsSync(executablePath)) {
    const cache = path.dirname(path.dirname(path.dirname(executablePath)))
    const suffix = process.platform === 'win32' ? 'chrome-win64/chrome.exe' : 'chrome-linux64/chrome'
    executablePath = readdirSync(cache).filter(name => /^chromium-\d+$/.test(name)).sort((a,b) => Number(b.split('-')[1])-Number(a.split('-')[1]))
      .map(name => path.join(cache,name,suffix)).find(candidate => existsSync(candidate)) || executablePath
  }
  browser = await chromium.launch({ headless: true, executablePath })
  const context = await browser.newContext({ viewport: { width: 1100, height: 800 } })
  await context.addInitScript(({ token, target }) => {
    localStorage.setItem('mox_access_token', token)
    localStorage.setItem('mox_user_info', JSON.stringify({ id:'alice',tenant_id:'a',roles:['user'],enabled:true }))
    window.__kbTarget = target
  }, { token, target })
  const page = await context.newPage(), errors = []
  page.on('pageerror', error => { errors.push(error.message); progress('pageerror ' + error.message) })
  page.on('response', response => { if (response.url().includes('/api/')) progress('HTTP ' + response.status() + ' ' + new URL(response.url()).pathname) })
  page.on('requestfailed', request => progress('failed ' + new URL(request.url()).pathname + ' ' + request.failure()?.errorText))
  await page.goto(`http://127.0.0.1:${server.address().port}/reports/html/20261004-kb-entity-normalization/index.html`)
  await expect(page.getByText('暂无可展示的实体引用')).toBeVisible({ timeout: 20000 })
  await page.getByRole('textbox', { name: '实体名称' }).fill('Rust')
  await page.getByRole('textbox', { name: '实体名称' }).press('Enter')
  const result = page.getByRole('list', { name: '实体搜索结果' })
  await result.getByRole('button').first().click()
  const linked = page.getByRole('list', { name: '已关联实体' })
  await expect(linked.getByRole('button')).toHaveCount(1)
  const check = await fetch(`${base}/api/kb/documents/${target}/entities`, { headers: { Authorization: `Bearer ${token}` } }).then(r => r.json())
  assert.equal(check.data.linked_entities.length, 1)
  await page.screenshot({ path: root + 'reports/html/20261004-kb-entity-normalization/assets/linked.png', fullPage: true })
  await linked.getByRole('button').press('Enter')
  await expect(linked.getByRole('button')).toHaveCount(0)
  await page.evaluate(token => window.__kbSwitch(token), other)
  await expect(page.getByRole('region', { name: '文档实体引用' }).getByRole('alert')).toBeVisible()
  await expect(result.getByRole('button')).toHaveCount(0)
  await expect(linked.getByRole('button')).toHaveCount(0)
  assert.deepEqual(errors, [])
  console.log('REAL_KB_BROWSER: keyboard search, persisted link, keyboard unlink, identity isolation passed')
} finally {
  if (browser) await browser.close()
  server.closeAllConnections(); await new Promise(resolve => server.close(resolve))
}
