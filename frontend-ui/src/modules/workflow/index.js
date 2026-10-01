// 工作流域模块声明。路由从 router/modules/workflow.js 迁入内核单源，meta 逐字保持（仅父路由注入 module/layout）。
// 纯 redirect 别名（/plugins /mcp /automation）无 component，留在 router/modules/workflow.js。
import { defineModule } from '@/modules/_kernel/module-registry.js'

export const WORKFLOW_MODULE = defineModule({
  name: 'workflow',
  title: '工作流',
  version: '0.1.0',
  routes: [
    {
      path: '/workflow',
      component: () => import('@/views/workflow/WorkflowView.vue'),
      meta: { title: '工作流编排', module: 'workflow', layout: 'default', requiresAuth: true },
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
    {
      path: '/browser',
      name: 'Browser',
      component: () => import('@/views/workflow/BrowserView.vue'),
      meta: { title: '浏览器自动化', module: 'workflow', layout: 'default', requiresAuth: true }
    }
  ],
  nav: []
}, import.meta.url)

export default WORKFLOW_MODULE
