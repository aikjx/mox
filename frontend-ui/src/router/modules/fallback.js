// Domain route definitions; authentication is applied by the host router.
export default [  // 403 无权限页面
  {
    path: '/403',
    name: 'Forbidden',
    component: () => import('@/views/misc/Forbidden.vue'),
    meta: { title: '无访问权限' }
  },

  // 404 兜底
  {
    path: '/:pathMatch(.*)*',
    redirect: '/dashboard'
  }

]
