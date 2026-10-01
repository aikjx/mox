// 专家联盟业务路由已并入 src/modules/expert-alliance/index.js（defineModule 单源登记）。
// 本文件仅保留纯 redirect 别名（无 component，内核不收）。
export default [
  // 兼容：/expert、/alliance 重定向到工作台主入口
  { path: '/expert', redirect: '/expert-workspace' },
  { path: '/alliance', redirect: '/expert-workspace' },
  // 兼容旧路径
  { path: '/expert-enterprise', redirect: '/expert-center/enterprise' },
  { path: '/expert-orchestrator', redirect: '/expert-center/orchestrator' }
]
