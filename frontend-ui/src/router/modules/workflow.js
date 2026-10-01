// 工作流域路由已迁入 src/modules/workflow/index.js（defineModule 单源登记）。
// 仅保留纯 redirect 别名（无 component，内核不收）。
export default [
  // 兼容旧路径
  { path: '/plugins', redirect: '/workflow/plugins' },
  { path: '/mcp', redirect: '/workflow/mcp' },
  { path: '/automation', redirect: '/workflow/automation' }
]
