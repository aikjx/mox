import { defineConfig, loadEnv } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

// 双保险 RBAC 令牌：VITE_OUS_API_TOKEN（dev时前端可用）+ OUS_API_TOKEN（vite proxy进程级注入）
// 关键（2026-08-26）：Vite 在 vite.config.js 求值完成后才运行 dotenv，
// 所以必须用官方 API `loadEnv(mode, dir, prefixes)` 显式加载 .env.{mode}.local / .env.{mode} / .env.local / .env。
// `prefixes=''` 表示加载所有变量（不按 VITE_ 前缀过滤），便于 process.env 级代理注入。
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  // 合并到 process.env（方便后续 process.env.GATEWAY_URL 之类写法直取）
  for (const [k, v] of Object.entries(env)) {
    if (!(k in process.env)) process.env[k] = v
  }
  const _TOKEN = env.VITE_OUS_API_TOKEN || env.OUS_API_TOKEN || ''
  console.log('[vite-config] mode=', mode, '_TOKEN=', _TOKEN ? `set(${_TOKEN.length}chars)` : 'EMPTY')
  console.log('[vite-config] GATEWAY_URL=', process.env.GATEWAY_URL || 'default :3001')

  return {
  plugins: [vue()],
  define: _TOKEN ? {
    'import.meta.env.VITE_OUS_API_TOKEN': JSON.stringify(_TOKEN),
    'import.meta.env.OUS_API_TOKEN':       JSON.stringify(_TOKEN),
  } : {},
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  server: {
    host: '0.0.0.0',
    port: 3020,
    headers: {
      // 安全响应头（开发环境保持宽松）
      'X-Content-Type-Options': 'nosniff',
      'X-Frame-Options': 'SAMEORIGIN',
      'Referrer-Policy': 'strict-origin-when-cross-origin'
    },
    proxy: (() => {
      const GW = process.env.GATEWAY_URL || 'http://localhost:3001'
      const AUTH = _TOKEN ? `Bearer ${_TOKEN}` : ''
      // Vite 官方推荐：configure(proxy, options) —— 在 http-proxy 实例创建后注册事件，100% 触发。
      // 相比直接写 on 对象（部分 Vite 5.x 补丁版本里被内部 wrapper 覆盖），configure 永远生效。
      /** @param {import('http-proxy')} proxy */
      const mkConfigure = (label) => (proxy) => {
        proxy.on('proxyReq', (proxyReq, req) => {
          console.log(`[proxy:${label}] ${req.method} ${req.url} | incoming-auth=${!!req.headers['authorization']} inject_len=${AUTH.length}`)
          if (AUTH && !req.headers['authorization']) {
            proxyReq.setHeader('Authorization', AUTH)
          }
          const accept = (req.headers['accept'] || '').toString()
          if (accept.includes('text/event-stream')) {
            proxyReq.setHeader('Accept-Encoding', 'identity')
            proxyReq.setHeader('Connection', 'keep-alive')
          }
        })
        proxy.on('error', (err, req, res) => {
          console.error(`[proxy:${label}:err] ${req?.method} ${req?.url}`, err?.message)
        })
        proxy.on('proxyRes', (proxyRes, req) => {
          console.log(`[proxy:${label}:res] ${proxyRes.statusCode} ${req.method} ${req.url}`)
        })
      }
      const mkAuthOnlyConfigure = (label) => (proxy) => {
        proxy.on('proxyReq', (proxyReq, req) => {
          if (AUTH && !req.headers['authorization']) proxyReq.setHeader('Authorization', AUTH)
        })
      }
      console.log('[vite-config] proxy target GW=', GW, 'AUTH_LEN=', AUTH.length)
      return {
        // ========== Rust 专家联盟网关（:3001）==========
        '/ai/engine': {
          target: GW,
          changeOrigin: true,
          configure: mkConfigure('ai-engine'),
        },
        // /voice/* → xiaobai_voice 服务代理（公开端点，3717 不可达时网关返回降级 JSON）
        '/voice': {
          target: GW,
          changeOrigin: true,
          configure: mkAuthOnlyConfigure('voice'),
        },
        // HITL 人机协同审批 WebSocket：由 Rust 网关承载（默认 :3001，可用 GATEWAY_URL 覆盖）
        '/ws': {
          target: GW,
          ws: true,
          changeOrigin: true,
          configure: mkAuthOnlyConfigure('ws'),
        },
        // ========== Rust 后端网关（:8080）—— 原 Node :3010 已迁移 ==========
        '/api': {
          target: 'http://localhost:8080',
          changeOrigin: true,
        },
      }
    })()
  },
  build: {
    // 关闭 Vite 自动清空输出目录：避免 rmSync 触发安全删除批量守卫（>50 文件）。
    // 旧资源为孤立文件（index.html 由新构建覆盖），不影响运行。
    emptyOutDir: false,
    chunkSizeWarningLimit: 2000,
    // 压缩：企业级发布产物，ES 压缩减少体积，保留可调试的注释剥离
    minify: 'esbuild',
    cssCodeSplit: true,
    reportCompressedSize: true,
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL('./index.html', import.meta.url))
      },
      output: {
        // 第三方大依赖独立分包，避免全部塞进主包，减小首屏体积
        manualChunks(id) {
          if (id.includes('node_modules')) {
            // 3d-force-graph 整棵依赖树走 rollup 默认分包（随 GraphView 懒加载 chunk 加载）：
            // 它包含 three / three-forcegraph / d3-* / ngraph.* / lodash-es / internmap 等，
            // 其中 lodash-es 被 element-plus 共用、internmap 被 d3-array 独用。
            // 若强行拆入 vendor-3d，会与 'vendor'（axios/lodash-es 等）形成 chunk 循环依赖，
            // 且 main 静态依赖 'vendor'，导致首屏就执行 3d 代码而整页白屏（SO is not a constructor）。
            const is3dTree =
              id.includes('3d-force-graph') ||
              id.includes('/three/') ||
              id.includes('three-forcegraph') ||
              id.includes('three-render-objects') ||
              id.includes('force-graph') ||
              id.includes('kapsule') ||
              id.includes('accessor-fn') ||
              id.includes('data-bind-mapper') ||
              id.includes('ngraph') ||
              id.includes('tinycolor2') ||
              id.includes('internmap') ||
              id.includes('propagating-hammerjs') ||
              id.includes('@tweenjs') ||
              id.includes('d3-')
            if (is3dTree) return undefined
            if (id.includes('echarts') || id.includes('zrender')) return 'vendor-echarts'
            if (id.includes('element-plus') || id.includes('@element-plus')) return 'vendor-element'
            if (id.includes('vue') || id.includes('vue-router')) return 'vendor-vue'
            // vexflow/3d 已随懒加载视图拆分：保持 vendor 仅通用依赖
            if (id.includes('axios')) return 'vendor-axios'
            return 'vendor'
          }
        }
      }
    }
  }
  } // ← 闭合 return { 配置对象
}) // ← 闭合 defineConfig 箭头函数
