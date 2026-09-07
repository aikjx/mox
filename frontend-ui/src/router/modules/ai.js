// Domain route definitions; authentication is applied by the host router.
export default [  // ===== AI 域 =====
  {
    path: '/ai',
    name: 'AI',
    component: () => import('@/views/ai/ChatView.vue'),
    meta: { title: 'AI 助手', requiresAuth: true }
  },
  // 分享快照：#/share/<base64-snapshot> → 用 ChatView 渲染（解析 token 并恢复对话）
  {
    path: '/share/:token',
    name: 'ShareSnapshot',
    component: () => import('@/views/ai/ChatView.vue'),
    meta: { title: '分享对话', shareMode: true }
  },
  // 兼容短链 /s/TOKEN
  {
    path: '/s/:token',
    redirect: to => `/share/${to.params.token}`
  },
  {
    path: '/caomei',
    name: 'Caomei',
    component: () => import('@/views/ai/CaomeiView.vue'),
    meta: { title: '需求编译', requiresAuth: true }
  },
  {
    path: '/algolab',
    name: 'AlgoLab',
    component: () => import('@/views/ai/AlgoLabView.vue'),
    meta: { title: '算法实验室', requiresAuth: true }
  },
  {
    path: '/infinite-optimizer',
    name: 'InfiniteOptimizer',
    component: () => import('@/views/ai/InfiniteOptimizerView.vue'),
    meta: { title: '无穷维度优化', requiresAuth: true }
  },
  {
    path: '/botCenter',
    name: 'BotCenter',
    component: () => import('@/views/ai/BotCenterView.vue'),
    meta: { title: '机器人中心', requiresAuth: true }
  },
  {
    path: '/melody2score',
    name: 'Melody2Score',
    component: () => import('@/views/ai/Melody2ScoreView.vue'),
    meta: { title: '旋律转谱', requiresAuth: true }
  },


]
