import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  { path: '/', redirect: '/dashboard' },
  {
    path: '/dashboard',
    name: 'Dashboard',
    component: () => import('@/views/Dashboard.vue'),
    meta: { title: '工作台' }
  },
  {
    path: '/portal',
    name: 'Portal',
    component: () => import('@/views/PortalHome.vue'),
    meta: { title: '门户' }
  },
  {
    path: '/hall',
    name: 'BusinessHall',
    component: () => import('@/views/BusinessHall.vue'),
    meta: { title: '业务大厅' }
  },
  {
    path: '/workbench',
    name: 'Workbench',
    component: () => import('@/views/Workbench.vue'),
    meta: { title: '工作台执行' }
  },
  {
    path: '/operators',
    name: 'Operators',
    component: () => import('@/views/OperatorsView.vue'),
    meta: { title: '算子中心' }
  },
  {
    path: '/graph',
    name: 'Graph',
    component: () => import('@/views/GraphView.vue'),
    meta: { title: '知识图谱' }
  },
  {
    path: '/ai',
    name: 'AI',
    component: () => import('@/views/ChatView.vue'),
    meta: { title: 'AI 助手' }
  },
  {
    path: '/resources',
    name: 'Resources',
    component: () => import('@/views/ResourcesView.vue'),
    meta: { title: '资源管理' }
  },
  {
    path: '/workflow',
    name: 'Workflow',
    component: () => import('@/views/WorkflowView.vue'),
    meta: { title: '工作流编排' }
  },
  {
    path: '/plugins',
    name: 'Plugins',
    component: () => import('@/views/PluginsView.vue'),
    meta: { title: 'AI 插件' }
  },
  {
    path: '/browser',
    name: 'Browser',
    component: () => import('@/views/BrowserView.vue'),
    meta: { title: '浏览器自动化' }
  },
  {
    path: '/monitor',
    name: 'Monitor',
    component: () => import('@/views/MonitorView.vue'),
    meta: { title: '系统监控' }
  },
  {
    path: '/docs',
    name: 'Docs',
    component: () => import('@/views/DocsView.vue'),
    meta: { title: 'API 文档' }
  },
  {
    path: '/market',
    name: 'Market',
    component: () => import('@/views/MarketView.vue'),
    meta: { title: '算子商城' }
  },
  {
    path: '/market/:id',
    name: 'MarketDetail',
    component: () => import('@/views/MarketDetailView.vue'),
    meta: { title: '算子详情' }
  },
  {
    path: '/mcp',
    name: 'Mcp',
    component: () => import('@/views/McpView.vue'),
    meta: { title: 'MCP 兼容中心' }
  },
  {
    path: '/automation',
    name: 'Automation',
    component: () => import('@/views/AutomationView.vue'),
    meta: { title: 'AI 自动化' }
  },
  {
    path: '/caomei',
    name: 'Caomei',
    component: () => import('@/views/CaomeiView.vue'),
    meta: { title: '需求编译' }
  },
  {
    path: '/algolab',
    name: 'AlgoLab',
    component: () => import('@/views/AlgoLabView.vue'),
    meta: { title: '算法实验室' }
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/dashboard'
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

export default router
