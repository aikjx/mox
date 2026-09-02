import { createRouter, createWebHashHistory } from 'vue-router'
import { ElMessage } from 'element-plus'
import { getToken as getSecureToken } from '@/utils/secureStorage'
import { usePermissionStore } from '@/stores/permission.store'

const routes = [
  { path: '/', redirect: '/dashboard' },
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/misc/Login.vue'),
    meta: { title: '登录' }
  },
  {
    path: '/portal',
    name: 'Portal',
    component: () => import('@/views/misc/PortalHome.vue'),
    meta: { title: '门户' }
  },
  {
    path: '/hall',
    name: 'BusinessHall',
    component: () => import('@/views/misc/BusinessHall.vue'),
    meta: { title: '业务大厅' }
  },

  // ===== 项目域 =====
  {
    path: '/dashboard',
    name: 'Dashboard',
    component: () => import('@/views/project/Dashboard.vue'),
    meta: { title: '工作台', requiresAuth: true }
  },
  {
    path: '/projects',
    name: 'Projects',
    component: () => import('@/views/project/ProjectsView.vue'),
    meta: { title: '项目中心', requiresAuth: true }
  },
  {
    path: '/tasks',
    name: 'Tasks',
    component: () => import('@/views/project/TaskView.vue'),
    meta: { title: '任务管理', requiresAuth: true }
  },
  {
    path: '/resources',
    component: () => import('@/views/project/ResourcesView.vue'),
    meta: {
      title: '资源管理',
      // 知识库页面可一键跳回专家工作台
      quickNav: [
        { key: 'expert-workspace', label: '专家工作台', path: '/expert-workspace', icon: 'User' },
        { key: 'graph', label: '知识图谱', path: '/graph', icon: 'Share' }
      ]
    },
    redirect: '/resources/overview',
    children: [
      { path: '', redirect: '/resources/overview' },
      {
        path: 'overview',
        name: 'ResourcesOverview',
        component: () => import('@/views/project/panels/ResourcesOverviewPanel.vue'),
        meta: { title: '资源概览', requiresAuth: true }
      },
      {
        path: 'knowledge',
        name: 'ResourcesKnowledge',
        component: () => import('@/views/project/panels/KnowledgeBasePanel.vue'),
        meta: {
          title: '知识库',
          requiresAuth: true,
          // 知识库页面快捷返回工作台
          backTo: { path: '/expert-workspace', label: '返回工作台' }
        }
      }
    ]
  },
  {
    path: '/workbench',
    name: 'Workbench',
    component: () => import('@/views/project/Workbench.vue'),
    meta: { title: '工作台执行', requiresAuth: true }
  },

  // ===== AI 域 =====
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

  // ===== 图谱域 =====
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
      title: '全维融合',
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

  // ===== 工作流域（嵌套路由） =====
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

  // ===== 专家联盟统一工作台（主入口）=====
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
  // 专家配置引擎（全维可配置）
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

  // ===== 算子商城 =====
  {
    path: '/market',
    name: 'Market',
    component: () => import('@/views/market/MarketView.vue'),
    meta: { title: '算子商城', requiresAuth: true }
  },
  {
    path: '/market/:id',
    name: 'MarketDetail',
    component: () => import('@/views/market/MarketDetailView.vue'),
    meta: { title: '算子详情', requiresAuth: true }
  },

  // ===== 算子中心 =====
  {
    path: '/operators',
    name: 'Operators',
    component: () => import('@/views/operators/OperatorsView.vue'),
    meta: { title: '算子中心', requiresAuth: true }
  },

  // ===== 系统管理（嵌套路由） =====
  {
    path: '/admin',
    component: () => import('@/views/admin/AdminView.vue'),
    meta: { title: '系统管理', requiresAuth: true, requiresRole: ['admin'] },
    redirect: '/admin/overview',
    children: [
      { path: '', redirect: '/admin/overview' },
      {
        path: 'overview',
        name: 'AdminOverview',
        component: () => import('@/views/admin/panels/AdminOverview.vue'),
        meta: { title: '管理总览', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'user',
        name: 'AdminUser',
        component: () => import('@/views/admin/panels/AdminUser.vue'),
        meta: { title: '用户管理', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'role',
        name: 'AdminRole',
        component: () => import('@/views/admin/panels/AdminRole.vue'),
        meta: { title: '角色管理', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'department',
        name: 'AdminDepartment',
        component: () => import('@/views/admin/panels/AdminDepartment.vue'),
        meta: { title: '部门管理', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'access',
        name: 'AdminAccess',
        component: () => import('@/views/admin/panels/AdminAccess.vue'),
        meta: { title: '访问凭证', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'audit',
        name: 'AdminAudit',
        component: () => import('@/views/admin/panels/AdminAudit.vue'),
        meta: { title: '审计日志', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'menu',
        name: 'AdminMenu',
        component: () => import('@/views/admin/panels/AdminMenu.vue'),
        meta: { title: '菜单管理', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'dict',
        name: 'AdminDict',
        component: () => import('@/views/admin/panels/AdminDict.vue'),
        meta: { title: '字典管理', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'config',
        name: 'AdminConfig',
        component: () => import('@/views/admin/panels/AdminConfig.vue'),
        meta: { title: '参数配置', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'storage',
        name: 'AdminStorage',
        component: () => import('@/views/admin/panels/AdminStorage.vue'),
        meta: { title: '存储与模块', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'hitl',
        name: 'AdminHitl',
        component: () => import('@/views/admin/panels/AdminHitl.vue'),
        meta: { title: 'HITL 审批', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'monitor',
        name: 'AdminMonitor',
        component: () => import('@/views/admin/panels/AdminMonitor.vue'),
        meta: { title: '系统监控', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'llm',
        name: 'AdminLlm',
        component: () => import('@/views/admin/panels/AdminLlm.vue'),
        meta: { title: '大模型配置', requiresAuth: true, requiresRole: ['admin'] }
      },
      {
        path: 'docs',
        name: 'AdminDocs',
        component: () => import('@/views/admin/panels/AdminDocs.vue'),
        meta: { title: 'API 文档', requiresAuth: true, requiresRole: ['admin'] }
      }
    ]
  },
  // 兼容旧路径
  { path: '/monitor', redirect: '/admin/monitor' },
  { path: '/docs', redirect: '/admin/docs' },
  { path: '/llm-config', redirect: '/admin/llm' },
  { path: '/knowledge-base', redirect: '/resources/knowledge' },

  // 403 无权限页面
  {
    path: '/403',
    name: 'Forbidden',
    component: () => import('@/views/misc/Forbidden.vue'),
    meta: { title: '无访问权限' }
  },

  // 404 兜底
  {
    path: '/:pathMatch(.*)*',
    redirect: '/dashboard'
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes,
  scrollBehavior() {
    // 路由切换时滚动到顶部，避免残留滚动位置
    return { top: 0 }
  }
})

// ===== 企业级路由守卫 =====
const DEFAULT_TITLE = '璇玑系统 · Mox Graph System'

// 不需要登录即可访问的页面白名单
const WHITE_LIST = ['/login', '/portal', '/hall', '/share', '/s/', '/403']

function isInWhiteList(path) {
  return WHITE_LIST.some(p => path === p || path.startsWith(p))
}

function getToken() {
  // 使用安全存储读取 token（自动兼容旧版 localStorage key：mox-token / ous_api_token / ous_token）
  return getSecureToken()
}

router.beforeEach(async (to, from, next) => {
  // 动态设置页面标题
  const pageTitle = to.meta?.title
  document.title = pageTitle ? `${pageTitle} · 璇玑系统` : DEFAULT_TITLE

  // 白名单页面直接放行
  if (isInWhiteList(to.path)) {
    next()
    return
  }

  // 检查登录状态
  const token = getToken()
  if (!token) {
    // 未登录，跳转到登录页，携带重定向地址
    next({
      path: '/login',
      query: { redirect: to.fullPath }
    })
    return
  }

  // 登录后首次加载权限
  const permissionStore = usePermissionStore()
  if (!permissionStore.loaded) {
    try {
      await permissionStore.loadPermissions()
    } catch (e) {
      console.warn('[Router] 权限加载失败，继续访问:', e?.message)
    }
  }

  // 路由权限校验（meta.requiresPermission）
  const requiresPerm = to.meta?.requiresPermission
  if (requiresPerm) {
    let hasAccess = false
    if (Array.isArray(requiresPerm)) {
      // 数组：任一权限满足即可
      hasAccess = permissionStore.hasAnyPermission(requiresPerm)
    } else {
      // 字符串：单个权限
      hasAccess = permissionStore.hasPermission(requiresPerm)
    }

    if (!hasAccess) {
      ElMessage.warning('抱歉，您没有访问该页面的权限')
      next({
        path: '/403',
        query: { redirect: to.fullPath }
      })
      return
    }
  }

  // 路由角色校验（meta.requiresRole）
  const requiresRole = to.meta?.requiresRole
  if (requiresRole) {
    let hasAccess = false
    if (Array.isArray(requiresRole)) {
      hasAccess = permissionStore.hasAnyRole(requiresRole)
    } else {
      hasAccess = permissionStore.hasRole(requiresRole)
    }

    if (!hasAccess) {
      ElMessage.warning('抱歉，您的角色无访问权限')
      next({
        path: '/403',
        query: { redirect: to.fullPath }
      })
      return
    }
  }

  next()
})

router.afterEach((to) => {
  // 路由切换后滚动到顶部
  window.scrollTo?.(0, 0)

  // 路由切换后清理可能残留的全局加载状态
  window.dispatchEvent(new CustomEvent('router:changed', { detail: { path: to.path } }))

  // 记录页面访问轨迹（用于用户行为分析）
  try {
    const history = JSON.parse(localStorage.getItem('mox_nav_history') || '[]')
    history.unshift({
      path: to.path,
      title: to.meta?.title || '',
      timestamp: Date.now()
    })
    // 只保留最近 20 条
    localStorage.setItem('mox_nav_history', JSON.stringify(history.slice(0, 20)))
  } catch {}
})

// 全局路由错误处理
router.onError((err) => {
  console.error('[Router Error]', err)
  // 组件加载失败时（如网络中断），尝试刷新页面
  if (err.message?.includes('Failed to fetch dynamically imported module') ||
      err.message?.includes('Loading chunk')) {
    ElMessage.warning('资源加载失败，正在重试...')
    setTimeout(() => window.location.reload(), 1500)
  }
})

export default router
