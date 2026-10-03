import assert from 'node:assert/strict'
import http from 'node:http'
import { once } from 'node:events'
import { createAllianceEventRecovery } from '../../../frontend-ui/src/modules/expert-alliance/contract/event-recovery.js'

const { MOX_PROBE_BASE: base, MOX_PROBE_TOKEN: primary, MOX_PROBE_OTHER_TOKEN: other } = process.env
let token = primary, badCursor = false, state, refreshes = 0, count = 0, lastIncomingCursor, lastExpert
const sockets = new Set()
const proxy = http.createServer((req,res) => {
  lastIncomingCursor = req.headers['last-event-id']
  const headers = { ...req.headers }
  if (badCursor) headers['last-event-id'] = 'evt-missing'
  const upstream = http.request(base + req.url, { method:req.method, headers }, response => {
    res.writeHead(response.statusCode,response.headers)
    res.flushHeaders()
    response.pipe(res)
  })
  upstream.on('error', () => res.destroy())
  res.on('close', () => upstream.destroy())
  req.pipe(upstream)
})
proxy.on('connection',socket => { sockets.add(socket); socket.on('close',()=>sockets.delete(socket)) })
proxy.listen(0,'127.0.0.1')
await once(proxy,'listening')
const wait = async (condition) => {
  const until = Date.now()+4000
  while (!condition()) { assert.ok(Date.now()<until,'recovery condition timed out'); await new Promise(r=>setTimeout(r,10)) }
}
const recovery = createAllianceEventRecovery({
  url:`http://127.0.0.1:${proxy.address().port}/api/alliance/events/stream`,getToken:()=>token,
  onState:next=>{state=next},
  onEvent:(_,envelope)=>{lastExpert=envelope.expert_id},
  refresh:async()=>{
    const response = await fetch(base+'/api/experts',{headers:{Authorization:`Bearer ${token}`}})
    assert.equal(response.status,200)
    count=(await response.json()).data.experts.length
    refreshes++
  }
})
try {
  let running=recovery.start()
  await wait(()=>state.state==='live' && refreshes===1)
  assert.equal(count,2)
  const created=await fetch(base+'/api/experts',{method:'POST',headers:{Authorization:`Bearer ${primary}`,'Content-Type':'application/json'},body:JSON.stringify({id:'controller-created',name:'controller-created'})})
  assert.equal(created.status,200)
  await wait(()=>lastExpert==='controller-created')
  badCursor=true
  const rejected=recovery.start()
  await running; await rejected
  assert.equal(state.state,'error')
  assert.ok(lastIncomingCursor?.startsWith('evt-'))
  badCursor=false
  running=recovery.start()
  await wait(()=>state.state==='live' && refreshes>=2)
  assert.equal(lastIncomingCursor,undefined,'410 clears the unavailable cursor')
  assert.equal(count,3)
  const before=refreshes
  const flooded=await fetch(base+'/test/flood',{method:'POST',headers:{Authorization:`Bearer ${primary}`}})
  assert.equal(flooded.status,204)
  await wait(()=>refreshes>before && state.state==='live')
  recovery.stop(); await running
  token=other
  running=recovery.start()
  await wait(()=>state.state==='live' && count===1)
  assert.equal(lastIncomingCursor,undefined,'identity invalidation never carries the old cursor')
  recovery.stop(); await running
  console.log('REAL_RECOVERY: refresh, cursor rejection, real broadcast gap and identity invalidation passed')
} finally {
  recovery.stop()
  for(const socket of sockets) socket.destroy()
  await new Promise(r=>proxy.close(r))
}
