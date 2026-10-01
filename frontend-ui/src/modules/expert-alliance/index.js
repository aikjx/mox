// 专家联盟前端模块声明。契约来自 Rust 权威源，路由与导航在此单源化。
import { defineModule } from '@/modules/_kernel/module-registry.js'
import { ENDPOINTS } from './contract/endpoints.js'

export { PHASE_IDS, PHASE_META, AUDIT_EVENTS_7 } from './contract/phases.js'
export * as AllianceEnums from './contract/enums.js'
export { ENDPOINTS } from './contract/endpoints.js'
export { createAllianceApi, allianceApi } from './api/alliance.api.js'

export const EXPERT_ALLIANCE_MODULE = defineModule({
  name: 'expert-alliance',
  title: '专家联盟',
  version: '1.0.0',
  endpoints: ENDPOINTS,
  routes: [
    // —— legacy 联盟主入口/管理/广场（原 router/modules/alliance.js 迁入，meta 逐字保持）——
    {
      path: '/expert-workspace',
      name: 'ExpertWorkspace',
      component: () => import('@/views/workspace/ExpertWorkspaceView.vue'),
      meta: {
        title: '专家联盟工作台',
        module: 'expert',
        layout: 'default',
        requiresAuth: true,
        isExpertAllianceMain: true,
        quickNav: [
          { key: 'graph', label: '知识图谱', path: '/graph', icon: 'Share', desc: '图谱探索与分析' },
          { key: 'knowledge', label: '知识库', path: '/resources/knowledge', icon: 'Coin', desc: '文档与知识管理' },
          { key: 'expert-center', label: '管理后台', path: '/expert-center', icon: 'Setting', desc: '联盟管理配置' },
          { key: 'ai', label: 'AI 对话', path: '/ai', icon: 'ChatDotRound', desc: '通用 AI 助手' }
        ],
        quickActions: [
          { key: 'register-expert', label: '注册专家', icon: 'Plus', event: 'mox:open-register-expert' },
          { key: 'new-debate', label: '发起辩论', icon: 'Aim', action: 'debate' },
          { key: 'multi-consult', label: '多专家咨询', icon: 'User', action: 'multi-consult' },
          { key: 'algo-analysis', label: '算法分析', icon: 'DataAnalysis', action: 'algorithm-analysis' }
        ]
      }
    },
    {
      path: '/expert-center',
      component: () => import('@/views/expert/ExpertCenterView.vue'),
      meta: {
        title: '专家联盟',
        module: 'expert',
        layout: 'default',
        requiresAuth: true,
        isExpertAdmin: true,
        requiresRole: ['super_admin', 'tenant_admin'],
        backTo: { path: '/expert-workspace', label: '返回工作台' }
      },
      redirect: '/expert-center/overview',
      children: [
        { path: '', redirect: '/expert-center/overview' },
        { path: 'overview', name: 'ExpertOverview', component: () => import('@/views/expert/panels/ExpertOverviewPanel.vue'), meta: { title: '联盟总览', requiresAuth: true } },
        { path: 'enterprise', name: 'ExpertEnterprise', component: () => import('@/views/expert/panels/ExpertEnterprisePanel.vue'), meta: { title: '企业管理', requiresAuth: true } },
        { path: 'orchestrator', name: 'ExpertOrchestrator', component: () => import('@/views/expert/panels/ExpertOrchestratorPanel.vue'), meta: { title: '编排引擎', requiresAuth: true } },
        { path: 'tasks', name: 'ExpertAllianceTasks', component: () => import('@/views/expert/AllianceTaskView.vue'), meta: { title: '联盟任务', requiresAuth: true } }
      ]
    },
    {
      path: '/expert-config',
      name: 'ExpertConfig',
      component: () => import('@/views/expert/ExpertConfigView.vue'),
      meta: {
        title: '专家配置',
        module: 'expert',
        layout: 'default',
        requiresAuth: true,
        requiresRole: ['super_admin', 'tenant_admin'],
        backTo: { path: '/expert-center', label: '返回联盟管理' }
      }
    },
    {
      path: '/expert-plaza',
      name: 'ExpertPlaza',
      component: () => import('@/views/expert/ExpertPlazaView.vue'),
      meta: { title: '专家广场', module: 'expert', layout: 'default', requiresAuth: true }
    },
    {
      path: '/alliance/console',
      name: 'AllianceConsole',
      component: () => import('./views/AllianceConsoleView.vue'),
      meta: {
        title: '联盟控制台',
        module: 'expert',
        layout: 'default',
        requiresAuth: true,
        // 破坏性写面（负载重置 / 调度配置写），与全站 ADMIN_GUARD 同口径
        requiresRole: ['super_admin', 'tenant_admin']
      }
    },
    {
      path: '/alliance/collab',
      name: 'AllianceCollab',
      component: () => import('./views/AllianceCollabView.vue'),
      meta: {
        title: '智能协作工作台',
        module: 'expert',
        layout: 'default',
        requiresAuth: true
      }
    },
    {
      path: '/alliance/graph',
      name: 'AllianceGraph',
      component: () => import('./views/AllianceGraphView.vue'),
      meta: {
        title: '专家协作图谱',
        module: 'expert',
        layout: 'default',
        requiresAuth: true,
        // 图谱重建为破坏性写（图版本号 +1 落盘），仅管理员
        requiresRole: ['super_admin', 'tenant_admin']
      }
    },
    {
      path: '/alliance/sessions',
      name: 'AllianceSessions',
      component: () => import('./views/AllianceSessionsView.vue'),
      meta: {
        title: '专家会话中心',
        module: 'expert',
        layout: 'default',
        requiresAuth: true
      }
    },
    {
      path: '/alliance/orchestration',
      name: 'AllianceOrchestration',
      component: () => import('./views/AllianceOrchestrationView.vue'),
      meta: {
        title: '专家编排台',
        module: 'expert',
        layout: 'default',
        requiresAuth: true,
        // 编排执行/计划生成均为写面；本系统角色模板无 operator 码，沿用全站管理员口径
        requiresRole: ['super_admin', 'tenant_admin']
      }
    },
    {
      path: '/alliance/experts',
      name: 'AllianceExperts',
      component: () => import('./views/AllianceExpertsView.vue'),
      meta: {
        title: '联盟专家广场',
        module: 'expert',
        layout: 'default',
        requiresAuth: true,
        // 专家注册/CRUD 为写面，仅管理员
        requiresRole: ['super_admin', 'tenant_admin']
      }
    }
  ],
  nav: [
    // —— legacy 联盟主入口/广场/管理（原 nav.config.js 手写副本迁入此处单源）——
    { key: 'expert-workspace', label: '联盟工作台', icon: 'Monitor', path: '/expert-workspace', section: '工作台', module: 'expert' },
    { key: 'expert-plaza', label: '专家广场', icon: 'User', path: '/expert-plaza', section: '工作台', module: 'expert' },
    { key: 'expert-center', label: '联盟管理', icon: 'Setting', path: '/expert-center', section: '管理', module: 'expert' },
    { key: 'expert-config', label: '专家配置', icon: 'Operation', path: '/expert-config', section: '管理', module: 'expert' },
    // —— 注册表派生的联盟模块页 ——
    { key: 'alliance-console', label: '联盟控制台', icon: 'Connection', path: '/alliance/console', section: '控制台', module: 'expert' },
    { key: 'alliance-collab', label: '智能协作', icon: 'MagicStick', path: '/alliance/collab', section: '控制台', module: 'expert' },
    { key: 'alliance-orchestration', label: '专家编排台', icon: 'SetUp', path: '/alliance/orchestration', section: '控制台', module: 'expert' },
    { key: 'alliance-graph', label: '协作图谱', icon: 'Share', path: '/alliance/graph', section: '控制台', module: 'expert' },
    { key: 'alliance-sessions', label: '会话中心', icon: 'ChatDotRound', path: '/alliance/sessions', section: '控制台', module: 'expert' },
    { key: 'alliance-experts', label: '联盟专家广场', icon: 'Avatar', path: '/alliance/experts', section: '控制台', module: 'expert' }
  ]
}, import.meta.url)

export default EXPERT_ALLIANCE_MODULE
