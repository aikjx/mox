// 系统管理路由已迁入 src/modules/system/index.js（/admin 嵌套）。
// 低代码试点页（/admin/{tenant,config,access}-lc）在 src/modules/admin-lowcode/index.js。
// 本文件仅保留纯 redirect 别名（无 component，内核不收）。
export default [
  // 兼容旧路径
  { path: '/monitor', redirect: '/admin/monitor' },
  { path: '/docs', redirect: '/admin/docs' },
  { path: '/llm-config', redirect: '/admin/llm' },
  { path: '/knowledge-base', redirect: '/resources/knowledge' }
]
