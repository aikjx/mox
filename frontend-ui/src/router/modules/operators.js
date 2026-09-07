// Domain route definitions; authentication is applied by the host router.
export default [  // ===== 算子中心 =====
  {
    path: '/operators',
    name: 'Operators',
    component: () => import('@/views/operators/OperatorsView.vue'),
    meta: { title: '算子中心', requiresAuth: true }
  },


]
