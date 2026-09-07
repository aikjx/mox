import { createRouter, createWebHashHistory } from 'vue-router'
import { ElMessage } from 'element-plus'
import { useAuthStore } from '@/stores/auth.store'
import { usePermissionStore } from '@/stores/permission.store'

import publicRoutes from "./modules/public.js"
import projectRoutes from "./modules/project.js"
import aiRoutes from "./modules/ai.js"
import graphRoutes from "./modules/graph.js"
import workflowRoutes from "./modules/workflow.js"
import allianceRoutes from "./modules/alliance.js"
import marketRoutes from "./modules/market.js"
import operatorsRoutes from "./modules/operators.js"
import systemRoutes from "./modules/system.js"
import fallbackRoutes from "./modules/fallback.js"

const routes = [
  ...publicRoutes,
  ...projectRoutes,
  ...aiRoutes,
  ...graphRoutes,
  ...workflowRoutes,
  ...allianceRoutes,
  ...marketRoutes,
  ...operatorsRoutes,
  ...systemRoutes,
  ...fallbackRoutes,
]

const router = createRouter({
  history: createWebHashHistory(),
  routes,
  scrollBehavior() {
    // 路由切换时滚动到顶部，避免残留滚动位置
    return { top: 0 }
  }
})

// ===== 企业级路由守卫 =====
const DEFAULT_TITLE = '璇玑系统 · Mox Graph System'

// 不需要登录即可访问的页面白名单
const WHITE_LIST = ['/login', '/portal', '/hall', '/share', '/s/', '/403']

function isInWhiteList(path) {
  return WHITE_LIST.some(p => path === p || path.startsWith(p))
}

function getToken() {
  // 使用安全存储读取 token（自动兼容旧版 localStorage key：mox-token / ous_api_token / ous_token）
  return useAuthStore().accessToken
}

router.beforeEach(async (to, from, next) => {
  // 动态设置页面标题
  const pageTitle = to.meta?.title
  document.title = pageTitle ? `${pageTitle} · 璇玑系统` : DEFAULT_TITLE

  // 白名单页面直接放行
  if (isInWhiteList(to.path)) {
    next()
    return
  }

  // 检查登录状态
  const token = getToken()
  if (!token) {
    // 未登录，跳转到登录页，携带重定向地址
    next({
      path: '/login',
      query: { redirect: to.fullPath }
    })
    return
  }

  // 登录后首次加载权限
  const permissionStore = usePermissionStore()
  if (!permissionStore.loaded) {
    try {
      await permissionStore.loadPermissions()
    } catch (e) {
      console.warn('[Router] 权限加载失败，继续访问:', e?.message)
    }
  }

  // 路由权限校验（meta.requiresPermission）
  const requiresPerm = to.meta?.requiresPermission
  if (requiresPerm) {
    let hasAccess = false
    if (Array.isArray(requiresPerm)) {
      // 数组：任一权限满足即可
      hasAccess = permissionStore.hasAnyPermission(requiresPerm)
    } else {
      // 字符串：单个权限
      hasAccess = permissionStore.hasPermission(requiresPerm)
    }

    if (!hasAccess) {
      ElMessage.warning('抱歉，您没有访问该页面的权限')
      next({
        path: '/403',
        query: { redirect: to.fullPath }
      })
      return
    }
  }

  // 路由角色校验（meta.requiresRole）
  const requiresRole = to.meta?.requiresRole
  if (requiresRole) {
    let hasAccess = false
    if (Array.isArray(requiresRole)) {
      hasAccess = permissionStore.hasAnyRole(requiresRole)
    } else {
      hasAccess = permissionStore.hasRole(requiresRole)
    }

    if (!hasAccess) {
      ElMessage.warning('抱歉，您的角色无访问权限')
      next({
        path: '/403',
        query: { redirect: to.fullPath }
      })
      return
    }
  }

  next()
})

router.afterEach((to) => {
  // 路由切换后滚动到顶部
  window.scrollTo?.(0, 0)

  // 路由切换后清理可能残留的全局加载状态
  window.dispatchEvent(new CustomEvent('router:changed', { detail: { path: to.path } }))

  // 记录页面访问轨迹（用于用户行为分析）
  try {
    const history = JSON.parse(localStorage.getItem('mox_nav_history') || '[]')
    history.unshift({
      path: to.path,
      title: to.meta?.title || '',
      timestamp: Date.now()
    })
    // 只保留最近 20 条
    localStorage.setItem('mox_nav_history', JSON.stringify(history.slice(0, 20)))
  } catch {}
})

// 全局路由错误处理
router.onError((err) => {
  console.error('[Router Error]', err)
  // 组件加载失败时（如网络中断），尝试刷新页面
  if (err.message?.includes('Failed to fetch dynamically imported module') ||
      err.message?.includes('Loading chunk')) {
    ElMessage.warning('资源加载失败，正在重试...')
    setTimeout(() => window.location.reload(), 1500)
  }
})

export default router
