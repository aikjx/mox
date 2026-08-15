import { createRouter, createWebHashHistory } from 'vue-router'
import PortalHome from '@/views/PortalHome.vue'
import Login from '@/views/Login.vue'
import Workbench from '@/views/Workbench.vue'
import BusinessHall from '@/views/BusinessHall.vue'

const routes = [
  { path: '/', name: 'portal', component: PortalHome, meta: { public: true, title: '企业门户首页' } },
  { path: '/login', name: 'login', component: Login, meta: { public: true, title: '统一登录' } },
  { path: '/workbench', name: 'workbench', component: Workbench, meta: { title: '智能工作台' } },
  { path: '/hall', name: 'hall', component: BusinessHall, meta: { title: '业务大厅' } },
]

const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

// 简易登录态守卫（复用 localStorage 中的 token，OUS 当前无真实鉴权，仅门户壳）
router.beforeEach((to) => {
  const token = localStorage.getItem('ous_token')
  if (!to.meta.public && !token) {
    return { name: 'login', query: { redirect: to.fullPath } }
  }
  return true
})

export default router
