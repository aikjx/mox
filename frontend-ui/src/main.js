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

const app = createApp(App)

for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component)
}

// ===== Pinia 状态管理 =====
const pinia = createPinia()

// Pinia DevTools 配置（开发环境自动启用，生产环境可关闭）
pinia.use(({ store }) => {
  // 在 DevTools 中显示 store 的自定义标签
  store.$subscribe((mutation, state) => {
    // 可在此处添加状态变更日志（开发调试用）
    if (import.meta.env.DEV) {
      console.debug(`[Pinia:${store.$id}]`, mutation.type, mutation.payload)
    }
  })
})

app.use(router)
app.use(pinia)
app.use(ElementPlus)
app.config.errorHandler = (err, instance, info) => {
  console.error('[Vue Error]', err, info)
  // 避免在用户操作过程中频繁弹窗，只对非预期错误提示
  if (err && err.message && !err.message.includes('canceled') && !err.message.includes('NavigationDuplicated')) {
    ElMessage.error({ message: '页面异常：' + (err.message || '未知错误'), duration: 4000 })
  }
}

// 全局未捕获 Promise 错误
window.addEventListener('unhandledrejection', (event) => {
  const reason = event.reason
  if (reason && reason.message) {
    if (reason.message.includes('canceled') || reason.message.includes('NavigationDuplicated')) return
    console.error('[Unhandled Promise]', reason)
  }
})

app.mount('#app')
