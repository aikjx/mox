// Domain route definitions; authentication is applied by the host router.
export default [  // ===== 图谱域 =====
  {
    path: '/graph',
    name: 'Graph',
    component: () => import('@/views/graph/GraphView.vue'),
    meta: {
      title: '知识图谱',
      requiresAuth: true,
      // 图谱页面快捷导航：可一键跳回专家工作台
      quickNav: [
        { key: 'expert-workspace', label: '专家工作台', path: '/expert-workspace', icon: 'User' },
        { key: 'knowledge', label: '知识库', path: '/resources/knowledge', icon: 'Coin' }
      ],
      backTo: { path: '/expert-workspace', label: '返回工作台' }
    }
  },
  {
    path: '/mox-fusion',
    name: 'MoxFusion',
    component: () => import('@/views/graph/MoxFusionView.vue'),
    meta: {
      title: 'mox 模块化系统架构融合',
      requiresAuth: true,
      backTo: { path: '/expert-workspace', label: '返回工作台' }
    }
  },
  {
    path: '/flow-graph',
    name: 'FlowGraph',
    component: () => import('@/views/graph/FlowGraph.vue'),
    meta: {
      title: '流程图',
      requiresAuth: true,
      backTo: { path: '/expert-workspace', label: '返回工作台' }
    }
  },


]
