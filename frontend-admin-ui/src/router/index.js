import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  {
    path: '/',
    redirect: '/dashboard'
  },
  {
    path: '/dashboard',
    name: 'Dashboard',
    component: () => import('@/views/Dashboard.vue'),
    meta: { title: '管理仪表盘' }
  },
  {
    path: '/users',
    redirect: '/users/list',
    meta: { title: '用户与权限' }
  },
  {
    path: '/users/list',
    name: 'UserList',
    component: () => import('@/views/users/UserList.vue'),
    meta: { title: '用户管理' }
  },
  {
    path: '/users/roles',
    name: 'RolePermission',
    component: () => import('@/views/users/RolePermission.vue'),
    meta: { title: '角色权限' }
  },
  {
    path: '/users/audit',
    name: 'AuditLog',
    component: () => import('@/views/users/AuditLog.vue'),
    meta: { title: '审计日志' }
  },
  {
    path: '/llm',
    redirect: '/llm/providers',
    meta: { title: '大模型配置' }
  },
  {
    path: '/llm/providers',
    name: 'LlmProviders',
    component: () => import('@/views/llm/LlmProviders.vue'),
    meta: { title: '模型供应商' }
  },
  {
    path: '/llm/routing',
    name: 'LlmRouting',
    component: () => import('@/views/llm/LlmRouting.vue'),
    meta: { title: '智能路由配置' }
  },
  {
    path: '/llm/usage',
    name: 'LlmUsage',
    component: () => import('@/views/llm/LlmUsage.vue'),
    meta: { title: '用量统计' }
  },
  {
    path: '/knowledge',
    redirect: '/knowledge/list',
    meta: { title: '知识库管理' }
  },
  {
    path: '/knowledge/list',
    name: 'KnowledgeList',
    component: () => import('@/views/knowledge/KnowledgeList.vue'),
    meta: { title: '知识库列表' }
  },
  {
    path: '/knowledge/categories',
    name: 'KnowledgeCategories',
    component: () => import('@/views/knowledge/KnowledgeCategories.vue'),
    meta: { title: '分类管理' }
  },
  {
    path: '/knowledge/permissions',
    name: 'KnowledgePermissions',
    component: () => import('@/views/knowledge/KnowledgePermissions.vue'),
    meta: { title: '访问权限' }
  },
  {
    path: '/storage',
    redirect: '/storage/paths',
    meta: { title: '云盘与存储' }
  },
  {
    path: '/storage/paths',
    name: 'StoragePaths',
    component: () => import('@/views/storage/StoragePaths.vue'),
    meta: { title: '存储路径配置' }
  },
  {
    path: '/storage/permissions',
    name: 'StoragePermissions',
    component: () => import('@/views/storage/StoragePermissions.vue'),
    meta: { title: '访问权限' }
  },
  {
    path: '/system',
    redirect: '/system/general',
    meta: { title: '系统设置' }
  },
  {
    path: '/system/general',
    name: 'SystemGeneral',
    component: () => import('@/views/system/SystemGeneral.vue'),
    meta: { title: '通用设置' }
  },
  {
    path: '/system/security',
    name: 'SystemSecurity',
    component: () => import('@/views/system/SystemSecurity.vue'),
    meta: { title: '安全策略' }
  },
  {
    path: '/system/about',
    name: 'SystemAbout',
    component: () => import('@/views/system/SystemAbout.vue'),
    meta: { title: '系统信息' }
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

export default router
