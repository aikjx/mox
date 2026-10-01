// 算子市场模块声明。路由从 router/modules/market.js 迁入内核单源，meta 逐字保持。
import { defineModule } from '@/modules/_kernel/module-registry.js'

export const MARKET_MODULE = defineModule({
  name: 'market',
  title: '算子市场',
  version: '0.1.0',
  routes: [
    {
      path: '/market',
      name: 'Market',
      component: () => import('@/views/market/MarketView.vue'),
      meta: { title: '算子市场', module: 'market', layout: 'default', requiresAuth: true }
    },
    {
      path: '/market/:id',
      name: 'MarketDetail',
      component: () => import('@/views/market/MarketDetailView.vue'),
      meta: { title: '算子详情', module: 'market', layout: 'default', requiresAuth: true }
    }
  ],
  nav: []
}, import.meta.url)

export default MARKET_MODULE
