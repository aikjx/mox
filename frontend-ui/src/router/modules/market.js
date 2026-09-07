// Domain route definitions; authentication is applied by the host router.
export default [  // ===== 算子商城 =====
  {
    path: '/market',
    name: 'Market',
    component: () => import('@/views/market/MarketView.vue'),
    meta: { title: '算子商城', requiresAuth: true }
  },
  {
    path: '/market/:id',
    name: 'MarketDetail',
    component: () => import('@/views/market/MarketDetailView.vue'),
    meta: { title: '算子详情', requiresAuth: true }
  },


]
