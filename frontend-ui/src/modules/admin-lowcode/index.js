// 低代码引擎管理台模块声明。SchemaCrudPage + pages/*.page.js 纯声明驱动，
// 路由从 router/modules/system.js 迁入内核单源（避免手写副本与注册表两处漂移）。
import { defineModule } from '@/modules/_kernel/module-registry.js'
import { ENDPOINTS } from './contract/endpoints.js'
import { tenantPage } from './pages/tenant.page.js'
import { configPage } from './pages/config.page.js'
import { accessPage } from './pages/access.page.js'
import { auditPage } from './pages/audit.page.js'

export { tenantPage, configPage, accessPage, auditPage }
export const loadSchemaCrudPage = () => import('./engine/SchemaCrudPage.vue')

const ADMIN_GUARD = { requiresAuth: true, requiresRole: ['super_admin', 'tenant_admin'] }

export const ADMIN_LOWCODE_MODULE = defineModule({
  name: 'admin-lowcode',
  title: '低代码管理台',
  version: '0.1.0',
  endpoints: ENDPOINTS,
  routes: [
    {
      path: '/admin/iam-audit-lc', name: 'AdminIamAuditLc', component: loadSchemaCrudPage,
      props: () => ({ pageSchema: auditPage }),
      meta: { title: 'IAM 安全审计', module: 'admin', layout: 'default', requiresAuth: true }
    },
    {
      path: '/admin/tenant-lc',
      name: 'AdminTenantLc',
      component: () => import('./engine/SchemaCrudPage.vue'),
      props: () => ({ pageSchema: tenantPage }),
      meta: { title: '租户管理(低代码试点)', module: 'admin', layout: 'default', ...ADMIN_GUARD }
    },
    {
      path: '/admin/config-lc',
      name: 'AdminConfigLc',
      component: () => import('./engine/SchemaCrudPage.vue'),
      props: () => ({ pageSchema: configPage }),
      meta: { title: '参数配置(低代码)', module: 'admin', layout: 'default', ...ADMIN_GUARD }
    },
    {
      path: '/admin/access-lc',
      name: 'AdminAccessLc',
      component: () => import('./engine/SchemaCrudPage.vue'),
      props: () => ({ pageSchema: accessPage }),
      meta: { title: '访问凭证(低代码)', module: 'admin', layout: 'default', ...ADMIN_GUARD }
    }
  ],
  nav: [
    { key: 'lc-iam-audit', label: 'IAM 安全审计', icon: 'Document', path: '/admin/iam-audit-lc', section: '低代码试点', module: 'admin' },
    { key: 'lc-tenant', label: '租户管理(低代码)', icon: 'Grid', path: '/admin/tenant-lc', section: '低代码试点', module: 'admin' },
    { key: 'lc-config', label: '参数配置(低代码)', icon: 'Tools', path: '/admin/config-lc', section: '低代码试点', module: 'admin' },
    { key: 'lc-access', label: '访问凭证(低代码)', icon: 'Key', path: '/admin/access-lc', section: '低代码试点', module: 'admin' }
  ]
}, import.meta.url)

export default ADMIN_LOWCODE_MODULE
