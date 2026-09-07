// Domain route definitions; authentication is applied by the host router.
export default [
  { path: '/', redirect: '/dashboard' },
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/auth/Login.vue'),
    meta: { title: '登录' }
  },
  {
    path: '/portal',
    name: 'Portal',
    component: () => import('@/views/misc/PortalHome.vue'),
    meta: { title: '门户' }
  },
  {
    path: '/hall',
    name: 'BusinessHall',
    component: () => import('@/views/misc/BusinessHall.vue'),
    meta: { title: '业务大厅' }
  },


]
