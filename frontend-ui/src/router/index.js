import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  { path: '/', redirect: '/ai' },
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/Login.vue'),
    meta: { title: '登录' }
  },
  {
    path: '/dashboard',
    name: 'Dashboard',
    component: () => import('@/views/Dashboard.vue'),
    meta: { title: '工作台' }
  },
  {
    path: '/projects',
    name: 'Projects',
    component: () => import('@/views/ProjectsView.vue'),
    meta: { title: '项目中心' }
  },
  {
    path: '/expert-center',
    name: 'ExpertCenter',
    component: () => import('@/views/ExpertCenterView.vue'),
    meta: { title: '专家联盟' }
  },
  {
    path: '/expert-enterprise',
    name: 'ExpertEnterprise',
    component: () => import('@/views/ExpertEnterpriseView.vue'),
    meta: { title: '企业级专家管理' }
  },
  {
    path: '/expert-orchestrator',
    name: 'ExpertOrchestrator',
    component: () => import('@/views/ExpertOrchestratorView.vue'),
    meta: { title: 'V2编排引擎' }
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
    path: '/xuanji-fusion',
    name: 'XuanjiFusion',
    component: () => import('@/views/XuanjiFusionView.vue'),
    meta: { title: '全维融合' }
  },
  {
    path: '/ai',
    name: 'AI',
    component: () => import('@/views/ChatView.vue'),
    meta: { title: 'AI 助手' }
  },
  {
    path: '/tasks',
    name: 'Tasks',
    component: () => import('@/views/TaskView.vue'),
    meta: { title: '任务管理' }
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
    path: '/infinite-optimizer',
    name: 'InfiniteOptimizer',
    component: () => import('@/views/InfiniteOptimizerView.vue'),
    meta: { title: '无穷维度优化' }
  },
  {
    path: '/botCenter',
    name: 'BotCenter',
    component: () => import('@/views/BotCenterView.vue'),
    meta: { title: '机器人中心' }
  },
  {
    path: '/knowledge-base',
    name: 'KnowledgeBase',
    component: () => import('@/views/KnowledgeBaseView.vue'),
    meta: { title: '云盘知识库' }
  },
  {
    path: '/llm-config',
    name: 'LlmConfig',
    component: () => import('@/views/LlmConfigView.vue'),
    meta: { title: '大模型配置' }
  },
  {
    path: '/melody2score',
    name: 'Melody2Score',
    component: () => import('@/views/Melody2ScoreView.vue'),
    meta: { title: '旋律转谱' }
  },
  {
    path: '/admin',
    name: 'Admin',
    component: () => import('@/views/admin/AdminView.vue'),
    meta: { title: '系统管理' }
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/ai'
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

export default router
