import assert from 'node:assert/strict'
import http from 'node:http'
import net from 'node:net'
import { once } from 'node:events'
import { fileURLToPath } from 'node:url'
import { mkdir } from 'node:fs/promises'
import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { appendFileSync } from 'node:fs'
import { build } from '../../../frontend-ui/node_modules/vite/dist/node/index.js'
import vue from '../../../frontend-ui/node_modules/@vitejs/plugin-vue/dist/index.mjs'
import { chromium, expect } from '../../../frontend-ui/node_modules/@playwright/test/index.mjs'

const root=fileURLToPath(new URL('../../../',import.meta.url))
const progress = step => appendFileSync(fileURLToPath(new URL('./browser-progress.log',import.meta.url)),new Date().toISOString()+' '+step+'\n')
const frontend=fileURLToPath(new URL('../../../frontend-ui/',import.meta.url))
const {MOX_PROBE_BASE:base,MOX_PROBE_TOKEN:primary,MOX_PROBE_OTHER_TOKEN:other}=process.env
let rejectCursor=false,delayStats=false,heldStats=null
const streams=new Set(), sockets=new Set(), sseHeaders=[]
const proxy=http.createServer((req,res)=>{
  const headers={...req.headers}
  if(req.url.includes('/events/stream')) {
    progress('SSE request '+String(req.headers['last-event-id'])+' reject='+rejectCursor)
    sseHeaders.push(req.headers['last-event-id'])
    if(rejectCursor) headers['last-event-id']='evt-missing'
    streams.add(res);res.on('close',()=>streams.delete(res))
  }
  const hold=delayStats && req.url==='/api/experts/stats'
  if(hold) delayStats=false
  const upstream=http.request(base+req.url,{method:req.method,headers},response=>{
    if(req.url.includes('/events/stream'))progress('SSE status '+response.statusCode)
    if(hold) {
      const chunks=[]
      response.on('data',chunk=>chunks.push(chunk))
      response.on('end',()=>{heldStats=()=>{res.writeHead(response.statusCode,response.headers);res.end(Buffer.concat(chunks))}})
    } else {res.writeHead(response.statusCode,response.headers);res.flushHeaders();response.pipe(res)}
  })
  upstream.on('error',()=>res.destroy())
  res.on('close',()=>upstream.destroy())
  req.pipe(upstream)
})
proxy.on('connection',socket=>{sockets.add(socket);socket.on('close',()=>sockets.delete(socket))})
proxy.listen(0,'127.0.0.1');await once(proxy,'listening')
const reservation=net.createServer().listen(0,'127.0.0.1');await once(reservation,'listening')
const port=reservation.address().port;await new Promise(r=>reservation.close(r))
const alias={'@':frontend+'src',vue:frontend+'node_modules/vue/dist/vue.esm-bundler.js',pinia:frontend+'node_modules/pinia/dist/pinia.mjs'}
const built=frontend+'node_modules/.recovery-build'
await build({configFile:false,root,plugins:[vue()],resolve:{alias},build:{outDir:built,emptyOutDir:true,rollupOptions:{input:root+'reports/html/20261003-event-page-recovery/index.html'}}})
progress('harness built')
const vite=http.createServer(async(req,res)=>{
  if(req.url.startsWith('/api/')) {
    const upstream=http.request(`http://127.0.0.1:${proxy.address().port}`+req.url,{method:req.method,headers:req.headers},response=>{
      res.writeHead(response.statusCode,response.headers);res.flushHeaders();response.pipe(res)
    })
    upstream.on('error',()=>res.destroy());res.on('close',()=>upstream.destroy());req.pipe(upstream)
    return
  }
  const file=path.resolve(built,'.'+decodeURIComponent(new URL(req.url,'http://localhost').pathname))
  if(!file.startsWith(path.resolve(built)+path.sep)){res.writeHead(403);res.end();return}
  try {
    const bytes=await readFile(file)
    res.setHeader('Content-Type',file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':'text/html; charset=utf-8')
    res.end(bytes)
  } catch {res.writeHead(404);res.end()}
})
let browser,page
try {
  vite.listen(port,'127.0.0.1');await once(vite,'listening')
  progress('vite listening')
  browser=await chromium.launch({headless:true,executablePath:'C:/Users/mo/AppData/Local/ms-playwright/chromium-1228/chrome-win64/chrome.exe'})
  progress('browser launched')
  const context=await browser.newContext({viewport:{width:1100,height:750}})
  await context.addInitScript(({token})=>{
    localStorage.setItem('mox_access_token',token)
    localStorage.setItem('mox_user_info',JSON.stringify({id:'recovery-user',tenant_id:'mine',roles:['tenant_admin'],enabled:true}))
  },{token:primary})
  page=await context.newPage()
  const errors=[]
  page.on('console',message=>{if(message.type()==='error')progress('console: '+message.text())})
  page.on('pageerror',error=>{errors.push(error.message);progress('page error: '+error.message)})
  page.on('requestfailed',request=>progress('request failed: '+request.url()+' '+request.failure()?.errorText))
  const before=await fetch(base+'/api/experts',{headers:{Authorization:`Bearer ${primary}`}})
  const initial=(await before.json()).data.experts.length
  progress('authority read completed')
  await page.goto(`http://127.0.0.1:${port}/reports/html/20261003-event-page-recovery/index.html`,{waitUntil:'domcontentloaded',timeout:20000})
  progress('page loaded')
  const panel=page.getByRole('region',{name:'实时更新状态'})
  await expect(panel.getByRole('status')).toContainText('实时更新已连接',{timeout:20000})
  progress('live status observed')
  await expect(page.getByTestId('count')).toHaveText(`已读取专家：${initial}`)
  const created=await fetch(base+'/api/experts',{method:'POST',headers:{Authorization:`Bearer ${primary}`,'Content-Type':'application/json'},body:JSON.stringify({id:'browser-created',name:'browser-created'})})
  assert.equal(created.status,200)
  await expect(page.getByTestId('count')).toHaveText(`已读取专家：${initial+1}`)
  rejectCursor=true
  await panel.getByRole('button',{name:'重新连接'}).click()
  await expect(panel.getByRole('status')).toContainText('历史续传不可用')
  progress('410 observed')
  assert.ok(sseHeaders.at(-1)?.startsWith('evt-'))
  const assets=fileURLToPath(new URL('../../html/20261003-event-page-recovery/assets/',import.meta.url))
  await mkdir(assets,{recursive:true})
  await page.screenshot({path:assets+'recovery-error.png',fullPage:true})
  rejectCursor=false
  await panel.getByRole('button',{name:'重新连接'}).press('Enter')
  await expect(panel.getByRole('status')).toContainText('实时更新已连接',{timeout:20000})
  assert.equal(sseHeaders.at(-1),undefined)
  delayStats=true
  await page.evaluate(()=>{window.__lateStats=window.__recoveryTest.loadStats()})
  await expect.poll(()=>!!heldStats).toBe(true)
  await page.evaluate(token=>window.__recoveryTest.switchIdentity(token),other)
  await expect(panel.getByRole('status')).toContainText('实时更新未连接')
  await panel.getByRole('button',{name:'重新连接'}).click()
  heldStats();heldStats=null
  await page.evaluate(()=>window.__lateStats)
  await expect(panel.getByRole('status')).toContainText('实时更新已连接',{timeout:20000})
  await expect(page.getByTestId('count')).toHaveText('已读取专家：1')
  await expect(page.getByTestId('stats')).toHaveText('统计专家：1')
  const observed = await page.evaluate(()=>window.__recoveryTest.otherTenantStats)
  assert.ok(observed.length>0)
  assert.ok(observed.every(count=>count===1),'Old tenant statistics must never commit, even transiently')
  progress('late stats rejected')
  assert.equal(sseHeaders.at(-1),undefined)
  assert.deepEqual(errors,[])
  await page.screenshot({path:assets+'recovery-success.png',fullPage:true})
  console.log('REAL_BROWSER_RECOVERY: visible 410, manual reconnect, actual expert refresh, identity cursor reset and stale stats rejection passed')
} catch(error) {
  if(page) progress('page text: '+await page.locator('body').innerText({timeout:2000}).catch(()=>''))
  progress('failure: '+error.stack)
  throw error
} finally {
  progress('cleanup')
  heldStats?.()
  for(const res of streams) res.destroy()
  for(const socket of sockets) socket.destroy()
  await browser?.close()
  vite.closeAllConnections()
  await new Promise(resolve=>vite.close(resolve))
  await new Promise(r=>proxy.close(r))
}
