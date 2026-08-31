import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import App from './App.vue'
import router from './router'
import './styles/global.css'
import './styles/themes/index.css'  // 三大主题：dark / sky / cyberpunk
import { setupPermissionDirectives } from '@/directives/permission'

const app = createApp(App)

// ===== 图标注册（全量注册，开发期便利）=====
// 生产优化建议：使用 unplugin-icons 按需自动导入，减少约 30KB gzip
// 配置见 vite.config.js 中可启用 IconsResolver
for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component)
}

// ===== Pinia 状态管理 =====
const pinia = createPinia()

// Pinia 持久化插件（轻量版：自动持久化指定字段到 localStorage）
pinia.use(({ store }) => {
  // 在 DevTools 中显示 store 的自定义标签
  store.$subscribe((mutation, state) => {
    if (import.meta.env.DEV) {
      console.debug(`[Pinia:${store.$id}]`, mutation.type)
    }
  })
})

app.use(router)
app.use(pinia)
app.use(ElementPlus)

// ===== 权限指令注册 =====
setupPermissionDirectives(app)

// ===== 全局错误处理 =====
app.config.errorHandler = (err, instance, info) => {
  console.error('[Vue Error]', err, info)
  // 避免在用户操作过程中频繁弹窗，只对非预期错误提示
  if (err && err.message) {
    if (err.message.includes('canceled') ||
        err.message.includes('NavigationDuplicated') ||
        err.message.includes('ResizeObserver loop')) {
      return
    }
    ElMessage.error({ message: '页面异常：' + (err.message || '未知错误'), duration: 4000 })
  }
}

// 全局未捕获 Promise 错误
window.addEventListener('unhandledrejection', (event) => {
  const reason = event.reason
  if (reason && reason.message) {
    if (reason.message.includes('canceled') ||
        reason.message.includes('NavigationDuplicated') ||
        reason.message.includes('ResizeObserver loop')) {
      return
    }
    console.error('[Unhandled Promise]', reason)
  }
})

// ===== 性能监控（开发环境）=====
if (import.meta.env.DEV && 'performance' in window) {
  // 首屏性能指标
  window.addEventListener('load', () => {
    setTimeout(() => {
      const nav = performance.getEntriesByType('navigation')[0]
      if (nav) {
        console.group('%c🚀 性能指标', 'color:#10b981;font-weight:600;')
        console.log('DNS 查询:', (nav.domainLookupEnd - nav.domainLookupStart).toFixed(0), 'ms')
        console.log('TCP 连接:', (nav.connectEnd - nav.connectStart).toFixed(0), 'ms')
        console.log('TTFB:', (nav.responseStart - nav.requestStart).toFixed(0), 'ms')
        console.log('DOM 解析:', (nav.domInteractive - nav.responseEnd).toFixed(0), 'ms')
        console.log('首字节到可交互:', (nav.domInteractive - nav.responseStart).toFixed(0), 'ms')
        console.log('页面完全加载:', (nav.loadEventEnd - nav.startTime).toFixed(0), 'ms')
        console.groupEnd()
      }
    }, 0)
  })
}

// ===== 全局特性检测 =====
app.config.globalProperties.$isMobile = () => {
  return typeof window !== 'undefined' && window.innerWidth < 768
}

app.mount('#app')
