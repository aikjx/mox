// 系统管理域模块声明。/admin 嵌套路由从 router/modules/system.js 迁入内核单源，meta 逐字保持（仅父路由注入 module/layout）。
// 纯 redirect 别名（/monitor /docs /llm-config /knowledge-base）留在 router/modules/system.js。
import { defineModule } from '@/modules/_kernel/module-registry.js'

const ADMIN_ROLE = ['super_admin', 'tenant_admin']

export const SYSTEM_MODULE = defineModule({
  name: 'system',
  title: '系统管理',
  version: '0.1.0',
  routes: [
    {
      path: '/admin',
      component: () => import('@/views/admin/AdminView.vue'),
      meta: { title: '系统管理', module: 'admin', layout: 'default', requiresAuth: true, requiresRole: ADMIN_ROLE },
      redirect: '/admin/overview',
      children: [
        { path: '', redirect: '/admin/overview' },
        { path: 'overview', name: 'AdminOverview', component: () => import('@/views/admin/panels/AdminOverview.vue'), meta: { title: '管理总览', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'tenant', name: 'AdminTenant', component: () => import('@/views/admin/panels/AdminTenant.vue'), meta: { title: '租户管理', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'user', name: 'AdminUser', component: () => import('@/views/admin/panels/AdminUser.vue'), meta: { title: '用户管理', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'role', name: 'AdminRole', component: () => import('@/views/admin/panels/AdminRole.vue'), meta: { title: '角色管理', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'department', name: 'AdminDepartment', component: () => import('@/views/admin/panels/AdminDepartment.vue'), meta: { title: '部门管理', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'access', name: 'AdminAccess', component: () => import('@/views/admin/panels/AdminAccess.vue'), meta: { title: '访问凭证', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'audit', name: 'AdminAudit', component: () => import('@/views/admin/panels/AdminAudit.vue'), meta: { title: '审计日志', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'menu', name: 'AdminMenu', component: () => import('@/views/admin/panels/AdminMenu.vue'), meta: { title: '菜单管理', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'dict', name: 'AdminDict', component: () => import('@/views/admin/panels/AdminDict.vue'), meta: { title: '字典管理', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'config', name: 'AdminConfig', component: () => import('@/views/admin/panels/AdminConfig.vue'), meta: { title: '参数配置', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'storage', name: 'AdminStorage', component: () => import('@/views/admin/panels/AdminStorage.vue'), meta: { title: '存储与模块', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'hitl', name: 'AdminHitl', component: () => import('@/views/admin/panels/AdminHitl.vue'), meta: { title: 'HITL 审批', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'monitor', name: 'AdminMonitor', component: () => import('@/views/admin/panels/AdminMonitor.vue'), meta: { title: '系统监控', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'api', name: 'AdminApi', component: () => import('@/views/admin/panels/AdminApi.vue'), meta: { title: '接口管理', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'logs', name: 'AdminLogs', component: () => import('@/views/admin/panels/AdminLogs.vue'), meta: { title: '在线日志', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'llm', name: 'AdminLlm', component: () => import('@/views/admin/panels/AdminLlm.vue'), meta: { title: '大模型配置', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'docs', name: 'AdminDocs', component: () => import('@/views/admin/panels/AdminDocs.vue'), meta: { title: 'API 文档', requiresAuth: true, requiresRole: ADMIN_ROLE } },
        { path: 'sso', name: 'AdminSso', component: () => import('@/views/admin/panels/AdminSso.vue'), meta: { title: '企业 SSO', requiresAuth: true, requiresRole: ADMIN_ROLE } }
      ]
    }
  ],
  nav: []
}, import.meta.url)

export default SYSTEM_MODULE
