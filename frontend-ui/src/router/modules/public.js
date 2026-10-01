// Domain route definitions; authentication is applied by the host router.
export default [
  { path: '/', redirect: '/dashboard' },
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/auth/Login.vue'),
    // bare：独立全屏页，不渲染应用侧栏/顶栏/引导，避免登录页嵌套在应用框架内
    meta: { title: '登录', bare: true }
  },
  {
    // 网关已匿名开放 POST /api/auth/register（config.rs public_paths），
    // 页面 Register.vue 的表单字段与该端点契约一致，故此处同为 bare 游客页。
    path: '/register',
    name: 'Register',
    component: () => import('@/views/auth/Register.vue'),
    meta: { title: '注册', bare: true }
  },
  {
    path: '/portal',
    name: 'Portal',
    component: () => import('@/views/public/PortalHome.vue'),
    meta: { title: '门户', bare: true }
  },
  {
    path: '/hall',
    name: 'BusinessHall',
    component: () => import('@/views/public/BusinessHall.vue'),
    meta: { title: '业务大厅', bare: true }
  },


]
