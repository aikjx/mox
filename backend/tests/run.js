'use strict'
/**
 * OUS 边缘入口冒烟测试（零依赖 Node.js）。
 * 仅校验边缘行为：服务可启动、/api/* 已不再由本地处理（应反代到 Rust）、
 * 静态资源路径不崩溃。业务/治理逻辑改由 Rust(crates/) 承载，不再在此测试。
 *
 * 用法：node tests/run.js   （或通过 npm test）
 */
const http = require('http')
const { createServer } = require('../src/server')

function get(port, pathname) {
  return new Promise((resolve, reject) => {
    const req = http.get({ host: 'localhost', port, path: pathname }, (res) => {
      let body = ''
      res.on('data', (c) => (body += c))
      res.on('end', () => resolve({ status: res.statusCode, body }))
    })
    req.on('error', reject)
  })
}

;(async () => {
  const srv = createServer()
  await new Promise((r) => srv.listen(0, r))
  const port = srv.address().port
  let failed = 0

  // 1) /api/* 不得由本地返回 404（说明已反代出去，而非本地处理）
  const api = await get(port, '/api/health')
  if (api.status === 404) {
    console.error('FAIL  /api/health 仍由本地处理（应反代到 Rust）')
    failed++
  } else {
    console.log(`OK    /api/health -> ${api.status}（已反代，非本地 404）`)
  }

  // 2) 静态资源：未构建时返回 503，已构建时返回 200/html（不得崩溃）
  const root = await get(port, '/')
  if (root.status === 200 || root.status === 503) {
    console.log(`OK    / -> ${root.status}（静态托管正常）`)
  } else {
    console.error(`FAIL  / -> ${root.status}`)
    failed++
  }

  srv.close(() => {
    console.log(failed === 0 ? '\n冒烟测试通过 ✅' : `\n冒烟测试失败 ❌ (${failed})`)
    process.exit(failed === 0 ? 0 : 1)
  })
})().catch((e) => {
  console.error('冒烟测试异常:', e)
  process.exit(1)
})
