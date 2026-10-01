// 算子中心模块声明。路由从 router/modules/operators.js 迁入内核单源，meta 逐字保持。
import { defineModule } from '@/modules/_kernel/module-registry.js'

export const OPERATORS_MODULE = defineModule({
  name: 'operators',
  title: '算子中心',
  version: '0.1.0',
  routes: [
    {
      path: '/operators',
      name: 'Operators',
      component: () => import('@/views/operators/OperatorsView.vue'),
      meta: { title: '算子中心', module: 'operators', layout: 'default', requiresAuth: true }
    }
  ],
  nav: []
}, import.meta.url)

export default OPERATORS_MODULE
