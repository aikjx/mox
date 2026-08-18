'use strict'
/**
 * OUS 后端信息服务入口（零依赖 Node.js）。
 * - 内置 HTTP 路由（支持 :param）
 * - Bearer 鉴权（dev-secret-token / OUS_API_TOKEN）
 * - 静态托管 frontend/dist（系统统一入口）
 * - 内存存储 + JSON 落盘
 */
const http = require('http')
const fs = require('fs')
const path = require('path')
const { Store } = require('./store')
const graph = require('./graph')
const xuanji = require('./xuanji')
const { seedAll } = require('./seed')
const registerRoutes = require('./routes')

const PORT = parseInt(process.env.PORT || '3000', 10)
const DIST = path.resolve(__dirname, '..', '..', 'frontend', 'dist')
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

// ---------- 存储与种子 ----------
const store = new Store()
;['graph_nodes', 'graph_edges', 'operators', 'market', 'plugins', 'workflows', 'flows', 'resources', 'caomei_templates', 'llm_config', 'automation', 'dialogue_sessions', 'settings'].forEach(
  (n) => store.load(n)
)
const seedInfo = seedAll(store)
console.log('[seed]', JSON.stringify(seedInfo))

// ---------- 路由表 ----------
const routes = [] // {method, segs:[], params:[], literal:bool, handler}
function route(method, pattern, handler) {
  const segs = pattern.split('/').filter(Boolean)
  const params = []
  const literal = !segs.some((s) => s.startsWith(':'))
  segs.forEach((s) => {
    if (s.startsWith(':')) params.push(s.slice(1))
  })
  routes.push({ method, segs, params, literal, handler })
}

function matchRoute(method, pathname) {
  const segs = pathname.split('/').filter(Boolean)
  // 第一遍：精确字面路由
  for (const r of routes) {
    if (r.method !== method) continue
    if (!r.literal) continue
    if (r.segs.length === segs.length && r.segs.every((s, i) => s === segs[i])) return { handler: r.handler, params: {} }
  }
  // 第二遍：参数路由
  for (const r of routes) {
    if (r.method !== method) continue
    if (r.literal) continue
    if (r.segs.length !== segs.length) continue
    const params = {}
    let ok = true
    for (let i = 0; i < r.segs.length; i++) {
      const s = r.segs[i]
      if (s.startsWith(':')) params[s.slice(1)] = segs[i]
      else if (s !== segs[i]) {
        ok = false
        break
      }
    }
    if (ok) return { handler: r.handler, params }
  }
  return null
}

// ---------- 响应辅助 ----------
function sendJSON(res, code, obj) {
  const data = JSON.stringify(obj)
  res.writeHead(code, {
    'Content-Type': 'application/json; charset=utf-8',
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Headers': '*',
    'Access-Control-Allow-Methods': 'GET,POST,PUT,DELETE,OPTIONS'
  })
  res.end(data)
}
function sendError(res, code, msg) {
  sendJSON(res, code, { error: msg, code })
}
function sendText(res, code, text) {
  res.writeHead(code, { 'Content-Type': 'text/plain; charset=utf-8' })
  res.end(text)
}

const PUBLIC = new Set(['/api/health', '/api/status', '/api/status/full', '/api/docs'])
function authOk(req) {
  const auth = req.headers['authorization'] || ''
  const m = /^Bearer\s+(.+)$/.exec(auth)
  if (!m) return false
  const tok = m[1].trim()
  if (tok === 'dev-secret-token') return true
  if (process.env.OUS_API_TOKEN && tok === process.env.OUS_API_TOKEN) return true
  return false
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
    // SPA 回退：无扩展名路径返回 index.html；否则 404
    if (!path.extname(rel)) {
      const idx = path.join(DIST, 'index.html')
      fs.readFile(idx, (e, buf) => {
        if (e) {
          res.writeHead(503, { 'Content-Type': 'text/plain; charset=utf-8' })
          res.end('前端尚未构建。请在 frontend/ 执行 npm run build，或先启动后端提供 API。')
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

// ---------- 请求体解析 ----------
function readBody(req, cb) {
  const chunks = []
  let size = 0
  req.on('data', (c) => {
    size += c.length
    if (size > 5 * 1024 * 1024) {
      req.destroy()
      cb(new Error('请求体过大'))
      return
    }
    chunks.push(c)
  })
  req.on('end', () => {
    const raw = Buffer.concat(chunks).toString('utf8')
    if (!raw) return cb(null, {})
    try {
      cb(null, JSON.parse(raw))
    } catch (e) {
      cb(null, {})
    }
  })
  req.on('error', (e) => cb(e))
}

// ---------- 请求处理器 ----------
function requestHandler(req, res) {
  const url = new URL(req.url, 'http://localhost')
  const pathname = decodeURIComponent(url.pathname)
  const query = Object.fromEntries(url.searchParams.entries())

  if (req.method === 'OPTIONS') {
    res.writeHead(204, {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Headers': '*',
      'Access-Control-Allow-Methods': 'GET,POST,PUT,DELETE,OPTIONS'
    })
    return res.end()
  }

  LOGS.push(`[${new Date().toISOString()}] ${req.method} ${pathname}`)
  if (LOGS.length > 500) LOGS.shift()

  // API 路由
  if (pathname.startsWith('/api/') || pathname === '/api') {
    const matched = matchRoute(req.method, pathname)
    if (!matched) {
      return sendError(res, 404, 'API 不存在: ' + pathname)
    }
    if (!PUBLIC.has(pathname) && !authOk(req)) {
      return sendError(res, 401, '鉴权失败：请提供 Bearer 令牌（开发期可用 dev-secret-token）')
    }
    if (req.method === 'GET' || req.method === 'DELETE') {
      try {
        matched.handler({ req, res, params: matched.params, query, body: {} })
      } catch (e) {
        sendError(res, 500, '内部错误: ' + e.message)
      }
      return
    }
    readBody(req, (err, body) => {
      if (err) return sendError(res, 400, err.message)
      try {
        matched.handler({ req, res, params: matched.params, query, body })
      } catch (e) {
        sendError(res, 500, '内部错误: ' + e.message)
      }
    })
    return
  }

  // 静态资源 / SPA
  serveStatic(req, res, pathname)
}

// ---------- 服务器工厂（便于测试启动独立实例）----------
function createServer() {
  return http.createServer(requestHandler)
}

// ---------- 注册路由 ----------
registerRoutes({ store, xuanji, graph, sendJSON, sendError, sendText, route, logs: LOGS, startTime })

const server = createServer()

// 仅当直接运行 `node src/server.js` 时自动监听；被 require 时不占端口（测试隔离）
if (require.main === module) {
  server.listen(PORT, () => {
    console.log(`[ous-backend] 监听 http://localhost:${PORT}  (静态=${DIST})`)
  })
}

module.exports = { server, store, createServer, requestHandler }

// ---------- 优雅关闭 ----------
function shutdown(signal) {
  console.log(`[ous-backend] 收到 ${signal}，正在关闭...`)
  try {
    store.persistAll()
  } catch (e) {
    console.error('[ous-backend] 落盘失败:', e.message)
  }
  server.close(() => process.exit(0))
  // 兜底：5s 内未完成关闭则强制退出，避免挂起
  setTimeout(() => process.exit(0), 5000).unref()
}
process.on('SIGINT', () => shutdown('SIGINT'))
process.on('SIGTERM', () => shutdown('SIGTERM'))
