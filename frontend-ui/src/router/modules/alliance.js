// Domain route definitions; authentication is applied by the host router.
export default [  // ===== 专家联盟统一工作台（主入口）=====
  {
    path: '/expert-workspace',
    name: 'ExpertWorkspace',
    component: () => import('@/views/workspace/ExpertWorkspaceView.vue'),
    meta: {
      title: '专家联盟工作台',
      requiresAuth: true,
      isExpertAllianceMain: true,
      // 工作台快捷导航配置：一键直达核心模块
      quickNav: [
        { key: 'graph', label: '知识图谱', path: '/graph', icon: 'Share', desc: '图谱探索与分析' },
        { key: 'knowledge', label: '知识库', path: '/resources/knowledge', icon: 'Coin', desc: '文档与知识管理' },
        { key: 'expert-center', label: '管理后台', path: '/expert-center', icon: 'Setting', desc: '联盟管理配置' },
        { key: 'ai', label: 'AI 对话', path: '/ai', icon: 'ChatDotRound', desc: '通用 AI 助手' }
      ],
      // 工作台内快捷操作
      quickActions: [
        { key: 'register-expert', label: '注册专家', icon: 'Plus', event: 'mox:open-register-expert' },
        { key: 'new-debate', label: '发起辩论', icon: 'Aim', action: 'debate' },
        { key: 'multi-consult', label: '多专家咨询', icon: 'User', action: 'multi-consult' },
        { key: 'algo-analysis', label: '算法分析', icon: 'DataAnalysis', action: 'algorithm-analysis' }
      ]
    }
  },

  // 兼容：/expert 重定向到工作台主入口
  { path: '/expert', redirect: '/expert-workspace' },
  { path: '/alliance', redirect: '/expert-workspace' },

  // ===== 专家联盟管理后台（嵌套路由）=====
  {
    path: '/expert-center',
    component: () => import('@/views/expert/ExpertCenterView.vue'),
    meta: {
      title: '专家联盟',
      requiresAuth: true,
      isExpertAdmin: true,
      // 管理后台快捷返回工作台
      backTo: { path: '/expert-workspace', label: '返回工作台' }
    },
    redirect: '/expert-center/overview',
    children: [
      { path: '', redirect: '/expert-center/overview' },
      {
        path: 'overview',
        name: 'ExpertOverview',
        component: () => import('@/views/expert/panels/ExpertOverviewPanel.vue'),
        meta: { title: '联盟总览', requiresAuth: true }
      },
      {
        path: 'enterprise',
        name: 'ExpertEnterprise',
        component: () => import('@/views/expert/panels/ExpertEnterprisePanel.vue'),
        meta: { title: '企业管理', requiresAuth: true }
      },
      {
        path: 'orchestrator',
        name: 'ExpertOrchestrator',
        component: () => import('@/views/expert/panels/ExpertOrchestratorPanel.vue'),
        meta: { title: '编排引擎', requiresAuth: true }
      },
      {
        path: 'tasks',
        name: 'ExpertAllianceTasks',
        component: () => import('@/views/expert/AllianceTaskView.vue'),
        meta: { title: '联盟任务', requiresAuth: true }
      }
    ]
  },
  // 专家配置引擎（mox 模块化系统架构可配置）
  {
    path: '/expert-config',
    name: 'ExpertConfig',
    component: () => import('@/views/expert/ExpertConfigView.vue'),
    meta: {
      title: '专家配置',
      requiresAuth: true,
      backTo: { path: '/expert-center', label: '返回联盟管理' }
    }
  },
  // 兼容旧路径
  { path: '/expert-enterprise', redirect: '/expert-center/enterprise' },
  { path: '/expert-orchestrator', redirect: '/expert-center/orchestrator' },

  // ===== 专家联盟广场 =====
  {
    path: '/expert-plaza',
    name: 'ExpertPlaza',
    component: () => import('@/views/expert/ExpertPlazaView.vue'),
    meta: { title: '专家广场', requiresAuth: true }
  },


]
