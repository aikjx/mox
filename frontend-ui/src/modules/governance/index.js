// 联盟治理台模块声明。双璇玑十四维的否决事件、专家健康与审计链，
// 后端在编排器 :3001（routes/governance.rs 十条路由），经网关 :3080 通配反代可达。
import { defineModule } from '@/modules/_kernel/module-registry.js'
import { ENDPOINTS } from './contract/endpoints.js'

export const GOVERNANCE_MODULE = defineModule({
  name: 'governance',
  title: '联盟治理台',
  version: '0.1.0',
  endpoints: ENDPOINTS,
  routes: [
    {
      path: '/alliance/governance',
      name: 'AllianceGovernance',
      component: () => import('./views/GovernanceConsoleView.vue'),
      meta: {
        title: '联盟治理台',
        module: 'expert',
        layout: 'default',
        requiresAuth: true,
        // 后端对 /api/governance/* 只做认证、没有任何 permission 判定
        // （governance:read 只出现在 routes/governance.rs 的文档注释里），
        // 故此处不设 requiresPermission：设了就是把页面锁在一个后端并不执行的名义权限上。
        backTo: { path: '/alliance/console', label: '返回联盟控制台' }
      }
    }
  ],
  nav: [
    { key: 'alliance-governance', label: '联盟治理台', icon: 'Aim', path: '/alliance/governance', section: '控制台', module: 'expert' }
  ]
}, import.meta.url)

export { ENDPOINTS } from './contract/endpoints.js'
export * from './contract/dimensions.js'
export { createGovernanceApi, governanceApi } from './api/governance.api.js'
export { useGovernanceStore } from './store/governance.store.js'

export default GOVERNANCE_MODULE
