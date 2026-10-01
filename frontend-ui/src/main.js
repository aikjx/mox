import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { ElMessage } from 'element-plus/es/components/message/index'
import { ElLoading } from 'element-plus/es/components/loading/index'
// element-plus 按需化：基础变量 + 函数式/指令样式显式引入（须先于 global.css 主题覆写）
import 'element-plus/es/components/base/style/css'
import 'element-plus/es/components/message/style/css'
import 'element-plus/es/components/message-box/style/css'
import 'element-plus/es/components/loading/style/css'
import App from './App.vue'
import router from './router'
import './styles/global.css'
import './styles/themes/index.css'  // 三大主题：dark / sky / cyberpunk
import { setupPermissionDirectives } from '@/directives'
import { registerNavIcons } from '@/modules'

const app = createApp(App)

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

// ===== 图标名解析口 =====
// vite.config.js 的 unplugin-vue-components resolver 只改写**模板标签**（<el-icon>、<ElIconAim>），
// 而模板里有两类位点传的是**字符串**：数据表 icon:'Aim'（实测 127 处）与内联三元
// :is="open ? 'ArrowUp' : 'ArrowDown'"（14 处）⇒ 这些名字编译期没人改写，运行期也没有登记项，
// 一律 "Failed to resolve component" 渲染成空白。这里按封闭集 app.component 注册把它们接上：
// 名单与理由见 src/modules/_kernel/nav-icons.js，nav-icons.test.js 全库扫描守这张表。
registerNavIcons(app)

// ===== v-loading 指令（按需引入，函数式 API 不会被模板解析器覆盖）=====
app.directive('loading', ElLoading.directive)

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
