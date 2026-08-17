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
        target: 'http://localhost:3000',
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
            if (id.includes('3d-force-graph') || id.includes('/three/')) return 'vendor-3d'
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
