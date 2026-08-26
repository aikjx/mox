'use strict'
/**
 * OUS 边缘入口（零依赖 Node.js）。
 *
 * 职责已收敛为两件，不再实现任何领域逻辑：
 *   1) 静态托管 frontend/dist（系统统一入口，默认 :3000）
 *   2) 将 /api/* 反向代理到 Rust operator-server（默认 :3001，可用
 *      OPERATOR_SERVER_URL 覆盖，例如 http://localhost:3001）
 *
 * 所有业务 / 治理 / 验证逻辑均由 Rust(crates/) 承载，Node 仅做边缘转发，
 * 从而确保出码必经 Rust ⛨验证网关 + 治理 8 闸门（单一系统真相，杜绝旁路）。
 */
const http = require('http')
const fs = require('fs')
const path = require('path')

const PORT = parseInt(process.env.PORT || '3000', 10)
const DIST = path.resolve(__dirname, '..', '..', 'frontend', 'dist')
const UPSTREAM = (process.env.OPERATOR_SERVER_URL || 'http://localhost:3001').replace(/\/+$/, '')
const RUST_ENTERPRISE_URL = (process.env.RUST_ENTERPRISE_URL || 'http://localhost:3002').replace(/\/+$/, '')
const LOGS = []
const startTime = Date.now()

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.mjs': 'application/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.ico': 'image/x-icon',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.map': 'application/json'
}

// ---------- 静态托管 ----------
function serveStatic(req, res, pathname) {
  let rel = pathname === '/' ? '/index.html' : pathname
  const filePath = path.normalize(path.join(DIST, rel))
  if (!filePath.startsWith(DIST)) {
    res.writeHead(403)
    return res.end('Forbidden')
  }
  fs.stat(filePath, (err, st) => {
    if (!err && st.isFile()) {
      const ext = path.extname(filePath).toLowerCase()
      res.writeHead(200, { 'Content-Type': MIME[ext] || 'application/octet-stream' })
      fs.createReadStream(filePath).pipe(res)
      return
    }
    // SPA 回退：无扩展名路径返回 index.html；根路径（/index.html 缺失）同走 503 提示；否则 404
    if (!path.extname(rel) || rel === '/index.html') {
      const idx = path.join(DIST, 'index.html')
      fs.readFile(idx, (e, buf) => {
        if (e) {
          res.writeHead(503, { 'Content-Type': 'text/plain; charset=utf-8' })
          res.end('前端尚未构建。请在 frontend/ 执行 npm run build，或由 Rust 服务托管。')
          return
        }
        res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' })
        res.end(buf)
      })
      return
    }
    res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' })
    res.end('Not Found')
  })
}

// ---------- 反向代理（通用） ----------
function proxyTo(req, res, pathname, search, target) {
  const u = new URL(pathname + search, target)
  const headers = {}
  for (const [k, v] of Object.entries(req.headers)) headers[k.toLowerCase()] = v
  headers.host = u.host

  const upstream = http.request(
    {
      method: req.method,
      hostname: u.hostname,
      port: u.port,
      path: u.pathname + u.search,
      headers
    },
    (upRes) => {
      res.writeHead(upRes.statusCode, upRes.headers)
      upRes.pipe(res)
    }
  )

  upstream.on('error', (e) => {
    if (!res.headersSent) {
      res.writeHead(502, { 'Content-Type': 'application/json; charset=utf-8' })
      res.end(JSON.stringify({ error: '上游 Rust 服务不可达: ' + e.message, upstream: target }))
    } else {
      res.destroy()
    }
  })

  if (req.method === 'GET' || req.method === 'HEAD' || req.method === 'OPTIONS') {
    upstream.end()
  } else {
    req.pipe(upstream)
  }
}

// 为旧调用保留 alias（proxy → UPSTREAM）
function proxy(req, res, pathname, search) {
  proxyTo(req, res, pathname, search, UPSTREAM)
}

// ---------- 请求处理器 ----------
function requestHandler(req, res) {
  const url = new URL(req.url, 'http://localhost')
  const pathname = decodeURIComponent(url.pathname)
  const search = url.search

  LOGS.push(`[${new Date().toISOString()}] ${req.method} ${pathname}`)
  if (LOGS.length > 500) LOGS.shift()

  // 双反代：/api/enterprise 精确前缀先匹配到企业级 Rust 真源（端口3002），
  // pathRewrite: ^/api/enterprise -> /api/enterprise/v1
  if (pathname === '/api/enterprise' || pathname.startsWith('/api/enterprise/')) {
    const rewritten = '/api/enterprise/v1' + pathname.slice('/api/enterprise'.length)
    return proxyTo(req, res, rewritten, search, RUST_ENTERPRISE_URL)
  }
  // 其余 /api 请求转发到 Rust operator-server（含 OPTIONS 预检，由 Rust CORS 层处理）
  if (pathname === '/api' || pathname.startsWith('/api/')) {
    return proxy(req, res, pathname, search)
  }
  // 其余走静态资源 / SPA
  serveStatic(req, res, pathname)
}

// ---------- 服务器工厂（便于测试启动独立实例）----------
function createServer() {
  return http.createServer(requestHandler)
}

const server = createServer()

// 仅当直接运行 `node src/server.js` 时自动监听；被 require 时不占端口（测试隔离）
if (require.main === module) {
  server.listen(PORT, () => {
    console.log(`[ous-backend] 边缘入口 http://localhost:${PORT}`)
    console.log(`  · /api/enterprise  →  企业级 Rust ${RUST_ENTERPRISE_URL} (pathRewrite -> /api/enterprise/v1)`)
    console.log(`  · /api/*          →  Rust operator ${UPSTREAM}`)
    console.log(`  · 静态/SPA         →  ${DIST}`)
  })
}

module.exports = { server, createServer, requestHandler, UPSTREAM, RUST_ENTERPRISE_URL, proxyTo }

// ---------- 优雅关闭 ----------
function shutdown(signal) {
  console.log(`[ous-backend] 收到 ${signal}，正在关闭...`)
  server.close(() => process.exit(0))
  setTimeout(() => process.exit(0), 5000).unref()
}
process.on('SIGINT', () => shutdown('SIGINT'))
process.on('SIGTERM', () => shutdown('SIGTERM'))
