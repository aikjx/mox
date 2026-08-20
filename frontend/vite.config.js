import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  server: {
    host: '0.0.0.0',
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:3002',
        changeOrigin: true
      }
    }
  },
  build: {
    // 关闭 Vite 自动清空输出目录：避免 rmSync 触发安全删除批量守卫（>50 文件）。
    // 旧资源为孤立文件（index.html 由新构建覆盖），不影响运行。
    emptyOutDir: false,
    chunkSizeWarningLimit: 2000,
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
            return 'vendor'
          }
        }
      }
    }
  }
})
