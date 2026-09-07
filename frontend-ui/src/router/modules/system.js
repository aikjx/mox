// Domain route definitions; authentication is applied by the host router.
export default [  // ===== 系统管理（嵌套路由） =====
  {
    path: '/admin',
    component: () => import('@/views/admin/AdminView.vue'),
    meta: { title: '系统管理', requiresAuth: true, requiresRole: ['admin'] },
    redirect: '/admin/overview',
    children: [
      { path: '', redirect: '/admin/overview' },
      {
        path: 'overview',
        name: 'AdminOverview',
        component: () => import('@/views/admin/panels/AdminOverview.vue'),
        meta: { title: '管理总览', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'user',
        name: 'AdminUser',
        component: () => import('@/views/admin/panels/AdminUser.vue'),
        meta: { title: '用户管理', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'role',
        name: 'AdminRole',
        component: () => import('@/views/admin/panels/AdminRole.vue'),
        meta: { title: '角色管理', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'department',
        name: 'AdminDepartment',
        component: () => import('@/views/admin/panels/AdminDepartment.vue'),
        meta: { title: '部门管理', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'access',
        name: 'AdminAccess',
        component: () => import('@/views/admin/panels/AdminAccess.vue'),
        meta: { title: '访问凭证', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'audit',
        name: 'AdminAudit',
        component: () => import('@/views/admin/panels/AdminAudit.vue'),
        meta: { title: '审计日志', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'menu',
        name: 'AdminMenu',
        component: () => import('@/views/admin/panels/AdminMenu.vue'),
        meta: { title: '菜单管理', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'dict',
        name: 'AdminDict',
        component: () => import('@/views/admin/panels/AdminDict.vue'),
        meta: { title: '字典管理', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'config',
        name: 'AdminConfig',
        component: () => import('@/views/admin/panels/AdminConfig.vue'),
        meta: { title: '参数配置', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'storage',
        name: 'AdminStorage',
        component: () => import('@/views/admin/panels/AdminStorage.vue'),
        meta: { title: '存储与模块', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'hitl',
        name: 'AdminHitl',
        component: () => import('@/views/admin/panels/AdminHitl.vue'),
        meta: { title: 'HITL 审批', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'monitor',
        name: 'AdminMonitor',
        component: () => import('@/views/admin/panels/AdminMonitor.vue'),
        meta: { title: '系统监控', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'api',
        name: 'AdminApi',
        component: () => import('@/views/admin/panels/AdminApi.vue'),
        meta: { title: '接口管理', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'logs',
        name: 'AdminLogs',
        component: () => import('@/views/admin/panels/AdminLogs.vue'),
        meta: { title: '在线日志', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'llm',
        name: 'AdminLlm',
        component: () => import('@/views/admin/panels/AdminLlm.vue'),
        meta: { title: '大模型配置', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'docs',
        name: 'AdminDocs',
        component: () => import('@/views/admin/panels/AdminDocs.vue'),
        meta: { title: 'API 文档', requiresAuth: true, requiresRole: ['admin'] }
      }
    ]
  },
  // 兼容旧路径
  { path: '/monitor', redirect: '/admin/monitor' },
  { path: '/docs', redirect: '/admin/docs' },
  { path: '/llm-config', redirect: '/admin/llm' },
  { path: '/knowledge-base', redirect: '/resources/knowledge' },


]
