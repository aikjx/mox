// AI 域路由已迁入 src/modules/ai/index.js（defineModule 单源登记）。
// 仅保留纯 redirect 别名 /s/:token（无 component，内核不收）。
export default [
  // 兼容短链 /s/TOKEN → /share/<token>
  {
    path: '/s/:token',
    redirect: to => `/share/${to.params.token}`
  }
]
