// Domain route definitions; authentication is applied by the host router.
export default [  // ===== 工作流域（嵌套路由） =====
  {
    path: '/workflow',
    component: () => import('@/views/workflow/WorkflowView.vue'),
    meta: { title: '工作流编排', requiresAuth: true },
    redirect: '/workflow/flows',
    children: [
      { path: '', redirect: '/workflow/flows' },
      {
        path: 'flows',
        name: 'WorkflowFlows',
        component: () => import('@/views/workflow/panels/WorkflowFlowsPanel.vue'),
        meta: { title: '流程编排', requiresAuth: true }
      },
      {
        path: 'plugins',
        name: 'WorkflowPlugins',
        component: () => import('@/views/workflow/panels/PluginsPanel.vue'),
        meta: { title: '插件中心', requiresAuth: true }
      },
      {
        path: 'mcp',
        name: 'WorkflowMcp',
        component: () => import('@/views/workflow/panels/McpPanel.vue'),
        meta: { title: 'MCP 兼容', requiresAuth: true }
      },
      {
        path: 'automation',
        name: 'WorkflowAutomation',
        component: () => import('@/views/workflow/panels/AutomationPanel.vue'),
        meta: { title: '自动化', requiresAuth: true }
      }
    ]
  },
  // 兼容旧路径
  { path: '/plugins', redirect: '/workflow/plugins' },
  { path: '/mcp', redirect: '/workflow/mcp' },
  { path: '/automation', redirect: '/workflow/automation' },
  {
    path: '/browser',
    name: 'Browser',
    component: () => import('@/views/workflow/BrowserView.vue'),
    meta: { title: '浏览器自动化', requiresAuth: true }
  },


]
