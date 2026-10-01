// 导航配置 - 侧边栏模块、分组、子模块、阶段流程等
// 设计原则：80% 用户 80% 时间用的功能放侧边栏
//          二级功能通过页面内 Tabs 访问
//          三级低频功能通过全局搜索 (Ctrl+K) 和 AI 对话访问
// 模块化页面（src/modules/*）的导航项在文件末尾由模块登记表自动挂载，不要在此手抄。
import { collectNav } from '@/modules'

// ===== 一级导航模块（3 大域 × 11 个模块）=====
export const NAV_MODULES = [
  // —— 项目域 ——
  { key: 'dashboard', label: '工作台', icon: 'Odometer', path: '/dashboard', color: '#4f46e5', bg: '#eef2ff' },
  { key: 'tasks', label: '任务中心', icon: 'List', path: '/tasks', color: '#0ea5e9', bg: '#e0f2fe' },
  { key: 'resources', label: '资源中心', icon: 'Coin', path: '/resources', color: '#10b981', bg: '#ecfdf5' },

  // —— 能力域 ——
  { key: 'ai', label: 'AI 助手', icon: 'ChatDotRound', path: '/ai', color: '#ec4899', bg: '#fce7f3' },
  { key: 'graph', label: '知识图谱', icon: 'Share', path: '/graph', color: '#06b6d4', bg: '#ecfeff' },
  { key: 'operators', label: '算子引擎', icon: 'Cpu', path: '/operators', color: '#6366f1', bg: '#eef2ff' },
  { key: 'workflow', label: '工作流', icon: 'Operation', path: '/workflow', color: '#f59e0b', bg: '#fffbeb' },
  // 专家联盟工作台（用户主入口）
  { key: 'expert-workspace', label: '专家联盟', icon: 'User', path: '/expert-workspace', color: '#7c3aed', bg: '#ede9fe' },
  // 专家广场（专家发现与预约）
  { key: 'expert-plaza', label: '专家广场', icon: 'Shop', path: '/expert-plaza', color: '#a855f7', bg: '#faf5ff' },
  // 专家联盟管理后台（管理入口）
  { key: 'expert-center', label: '联盟管理', icon: 'Setting', path: '/expert-center', color: '#8b5cf6', bg: '#f5f3ff' },

  // —— 生态与管理 ——
  { key: 'market', label: '算子商城', icon: 'Shop', path: '/market', color: '#f43f5e', bg: '#ffe4e6' },
  { key: 'admin', label: '系统管理', icon: 'Setting', path: '/admin', color: '#475569', bg: '#f1f5f9' }
]

// ===== 侧边栏导航分组（3 大域）=====
export const NAV_GROUPS = [
  { key: 'project',  label: '项目域',   order: 0, items: ['dashboard', 'tasks', 'resources'] },
  { key: 'capability', label: '能力域', order: 1, items: ['ai', 'graph', 'operators', 'workflow', 'expert-workspace', 'expert-plaza', 'expert-center'] },
  { key: 'ecosystem',  label: '生态与管理', order: 2, items: ['market', 'admin'] }
]

// ===== 二级子模块映射（一级模块 → 页面内 Tabs 子模块）=====
// 设计原则：每个一级模块下 2-6 个二级 Tabs，按使用频率排序
export const SUB_MODULES = {
  // —— 项目域 ——
  dashboard: [
    { key: 'overview', label: '概览', path: '/dashboard' },
    { key: 'projects', label: '项目列表', path: '/projects' }
  ],
  tasks: [
    { key: 'all', label: '全部任务', path: '/tasks' }
  ],
  resources: [
    { key: 'overview', label: '资源概览', path: '/resources' },
    { key: 'knowledge', label: '知识库', path: '/resources/knowledge' }
  ],

  // —— 能力域 ——
  ai: [
    { key: 'chat', label: 'AI 对话', path: '/ai' },
    { key: 'caomei', label: '需求编译', path: '/caomei' },
    { key: 'algolab', label: '算法实验室', path: '/algolab' },
    { key: 'botCenter', label: '机器人中心', path: '/botCenter' },
    { key: 'infinite-optimizer', label: '无穷维度优化', path: '/infinite-optimizer' },
    { key: 'melody2score', label: '旋律转谱', path: '/melody2score' }
  ],
  graph: [
    { key: 'explorer', label: '图谱探索', path: '/graph' },
    { key: 'fusion', label: '架构融合', path: '/mox-fusion' },
    { key: 'flow-graph', label: '流程图谱', path: '/flow-graph' }
  ],
  operators: [
    { key: 'all', label: '算子中心', path: '/operators' }
  ],
  workflow: [
    { key: 'flows', label: '流程编排', path: '/workflow' },
    { key: 'plugins', label: '插件中心', path: '/workflow/plugins' },
    { key: 'mcp', label: 'MCP 兼容', path: '/workflow/mcp' },
    { key: 'automation', label: '自动化', path: '/workflow/automation' },
    { key: 'browser', label: '浏览器自动化', path: '/browser' }
  ],
  // 专家联盟工作台（用户主入口）的二级模块
  'expert-workspace': [
    { key: 'collaboration', label: '专家协作', path: '/expert-workspace' },
    { key: 'exploration', label: '知识探索', path: '/expert-workspace?mode=exploration' },
    { key: 'orchestration', label: '任务编排', path: '/expert-workspace?mode=orchestration' },
    { key: 'analysis', label: '深度分析', path: '/expert-workspace?mode=analysis' },
    { key: 'expert-center', label: '联盟管理', path: '/expert-center' }
  ],
  // 专家广场（专家发现与预约）的二级模块
  'expert-plaza': [
    { key: 'discover', label: '发现专家', path: '/expert-plaza' },
    { key: 'ranking', label: '排行榜', path: '/expert-plaza?tab=ranking' },
    { key: 'appointments', label: '我的预约', path: '/expert-plaza?tab=appointments' },
    { key: 'workspace', label: '专家工作台', path: '/expert-workspace' }
  ],
  // 专家联盟管理后台的二级模块
  'expert-center': [
    { key: 'overview', label: '联盟总览', path: '/expert-center' },
    { key: 'enterprise', label: '企业管理', path: '/expert-center/enterprise' },
    { key: 'orchestrator', label: '编排引擎', path: '/expert-center/orchestrator' },
    { key: 'config', label: '专家配置', path: '/expert-config' },
    { key: 'workspace', label: '返回工作台', path: '/expert-workspace' }
  ],

  // —— 生态与管理 ——
  market: [
    { key: 'all', label: '算子商城', path: '/market' }
  ],
  admin: [
    { key: 'overview', label: '管理总览', path: '/admin' },
    { key: 'access', label: '访问凭证', path: '/admin/access' },
    { key: 'audit', label: '审计日志', path: '/admin/audit' },
    { key: 'storage', label: '存储与模块', path: '/admin/storage' },
    { key: 'hitl', label: 'HITL 审批', path: '/admin/hitl' },
    { key: 'monitor', label: '系统监控', path: '/admin/monitor' },
    { key: 'llm', label: '大模型配置', path: '/admin/llm' },
    { key: 'docs', label: 'API 文档', path: '/admin/docs' }
  ]
}

// ===== 三级隐藏模块（高级/低频，仅通过全局搜索和 AI 对话访问）=====
// 设计原则：极低频、入口深、面向专业用户的功能
export const HIDDEN_MODULES = [
  // 已全部纳入对应一级模块的二级 Tabs，此处保留作为未来扩展位
  // 新增长尾功能请先放这里，验证使用频率后再决定是否提升到二级
]

// ===== 5 阶段流程（与 PhasePipeline 对齐 · 按项目开发流程）=====
export const PROJECT_PHASES = [
  { key: 'requirement', label: '需求阶段', desc: 'AI 对话 · 需求编译 · 知识库', color: 'var(--cat-1-fill)', group: 's1-require' },
  { key: 'architecture', label: '架构阶段', desc: '知识图谱 · 专家联盟 · 架构融合', color: 'var(--cat-2-fill)', group: 's2-arch' },
  { key: 'develop', label: '开发阶段', desc: '算子 · 工作流 · 插件 · 自动化', color: 'var(--cat-3-fill)', group: 's3-dev' },
  { key: 'release', label: '发布阶段', desc: '监控 · 文档 · 系统管理', color: 'var(--cat-4-fill)', group: 's4-release' }
]

// ===== 顶栏⚡新建命令（6 项，按 4 阶段顺序排）=====
export const QUICK_CREATE_COMMANDS = [
  { key: 'project',    label: '新建项目',     icon: 'Folder',      tip: 'S0 启动跟进',         action: 'event', event: 'mox:open-create-project' },
  { key: 'task',       label: '新建任务',     icon: 'List',        tip: 'Ctrl + Shift + N',    action: 'event', event: 'mox:open-create-task' },
  { key: 'ai-session', label: '新建 AI 对话', icon: 'ChatDotRound',tip: 'AI 助手 · φ 模式',    action: 'route', route: '/ai', query: { fresh: '1' } },
  { key: 'expert',     label: '注册专家',     icon: 'User',        tip: '专家联盟招募',        action: 'event', event: 'mox:open-register-expert' },
  { key: 'workflow',   label: '新建工作流',   icon: 'Operation',   tip: 'S3 方案设计',         action: 'route', route: '/workflow', query: { action: 'create' } },
  { key: 'market',     label: '上传算子包',   icon: 'Shop',        tip: 'S4 注册算子',         action: 'route', route: '/market', query: { action: 'upload' } }
]

// ===== 快捷键分组（Shift + ? 弹 Drawer 展示给用户）=====
export const HOTKEY_GROUPS = [
  {
    group: '全局',
    items: [
      { keys: ['Ctrl', 'K'], desc: '聚焦全局搜索（命令面板）' },
      { keys: ['Ctrl', '⇧', 'P'], desc: '同上（命令面板，兼容 VS Code 用户）' },
      { keys: ['Ctrl', '⇧', 'N'], desc: '弹出新建任务 Dialog（任何页面）' },
      { keys: ['Shift', '?'], desc: '打开 / 关闭本快捷键帮助' },
      { keys: ['Alt', '1..9'], desc: '按导航分组 1-9 顺序跳转到对应模块' }
    ]
  },
  {
    group: '表单与列表',
    items: [
      { keys: ['Esc'], desc: '关闭当前 Dialog / Drawer / 取消搜索' },
      { keys: ['Enter'], desc: '提交聚焦中的表单 / 搜索（已在 10+ 页面启用）' },
      { keys: ['Ctrl', 'Enter'], desc: 'AI 场景下提交长文本（Chat / Automation 中）' },
      { keys: ['⌫ / Backspace'], desc: '在列表内清空筛选（需列表聚焦）' }
    ]
  },
  {
    group: '专家联盟工作台',
    items: [
      { keys: ['Ctrl', 'E'], desc: '打开专家联盟工作台' },
      { keys: ['Ctrl', '⇧', 'D'], desc: '发起专家辩论' },
      { keys: ['Ctrl', '⇧', 'M'], desc: '多专家协同咨询' },
      { keys: ['Ctrl', '⇧', 'R'], desc: '智能路由匹配专家' }
    ]
  }
]

// ===== 图标侧栏导航（64px 栏，emoji 图标，3 组）=====
export const ICON_NAV_GROUPS = [
  { key: 'project', label: '项目', items: [
    { key: 'dashboard', label: '工作台', icon: '📊', path: '/dashboard' },
    { key: 'projects', label: '项目中心', icon: '📁', path: '/projects' },
    { key: 'tasks', label: '任务中心', icon: '✅', path: '/tasks' },
  ]},
  { key: 'capability', label: '能力', items: [
    { key: 'ai', label: 'AI助手', icon: '🤖', path: '/ai' },
    { key: 'graph', label: '知识图谱', icon: '🕸️', path: '/graph' },
    { key: 'operators', label: '算子引擎', icon: '⚙️', path: '/operators' },
    { key: 'workflow', label: '工作流', icon: '🔄', path: '/workflow' },
    { key: 'expert', label: '专家联盟', icon: '👥', path: '/expert-workspace' },
    // —— 注册表派生的专家联盟模块页（collectNav 单源，勿手改路由；与 expert-alliance/index.js nav 一一对应）——
    { key: 'alliance-console', label: '联盟控制台', icon: '🎛️', path: '/alliance/console' },
    { key: 'alliance-collab', label: '智能协作', icon: '🧠', path: '/alliance/collab' },
    { key: 'alliance-orchestration', label: '专家编排台', icon: '🧭', path: '/alliance/orchestration' },
    { key: 'alliance-graph', label: '协作图谱', icon: '🔗', path: '/alliance/graph' },
    { key: 'alliance-sessions', label: '会话中心', icon: '💬', path: '/alliance/sessions' },
    { key: 'alliance-experts', label: '联盟专家广场', icon: '🎓', path: '/alliance/experts' },
  ]},
  { key: 'ecosystem', label: '生态', items: [
    { key: 'market', label: '算子商城', icon: '🛒', path: '/market' },
    { key: 'admin', label: '系统设置', icon: '⚡', path: '/admin', bottom: true },
  ]},
]

// ===== 模块侧栏配置（按路由模块动态渲染）=====
export const MODULE_SIDEBAR_CONFIG = {
  dashboard: {
    title: '工作台', subtitle: '项目概览与快捷入口',
    sections: [
      { title: '快捷', items: [
        { key: 'overview', label: '总览', icon: '📊' },
        { key: 'recent', label: '最近项目', icon: '🕐' },
        { key: 'starred', label: '收藏', icon: '⭐' },
      ]},
    ]
  },
  projects: {
    title: '项目中心', subtitle: '项目管理与协作',
    sections: [
      { title: '项目', items: [
        { key: 'all', label: '全部项目', icon: '📋' },
        { key: 'active', label: '进行中', icon: '🔄' },
        { key: 'paused', label: '已暂停', icon: '⏸️' },
        { key: 'done', label: '已完成', icon: '✅' },
      ]},
      { title: '我的', items: [
        { key: 'starred', label: '收藏的项目', icon: '⭐' },
        { key: 'mine', label: '我负责的', icon: '👤' },
        { key: 'recent', label: '最近访问', icon: '🕐' },
      ]}
    ]
  },
  tasks: {
    title: '任务中心', subtitle: '任务与待办管理',
    sections: [
      { title: '任务', items: [
        { key: 'all', label: '全部任务', icon: '📋' },
        { key: 'today', label: '今日待办', icon: '☀️' },
        { key: 'week', label: '本周任务', icon: '📅' },
        { key: 'done', label: '已完成', icon: '✅' },
      ]},
      { title: '优先级', items: [
        { key: 'high', label: '高优先级', icon: '🔴' },
        { key: 'mid', label: '中优先级', icon: '🟡' },
        { key: 'low', label: '低优先级', icon: '🟢' },
      ]}
    ]
  },
  expert: {
    title: '专家联盟', subtitle: '专家协作与管理',
    // 工作台/管理/控制台三组条目全部由 expert-alliance/index.js 的 collectNav 自动挂载，勿手抄。
    sections: []
  },
  ai: { title: 'AI助手', subtitle: '智能对话', sections: [] },
  graph: { title: '知识图谱', subtitle: '可视化探索', sections: [] },
  operators: { title: '算子引擎', subtitle: '算子中心', sections: [] },
  workflow: { title: '工作流', subtitle: '流程编排', sections: [] },
  market: { title: '算子商城', subtitle: '发现与分享', sections: [] },
  admin: { title: '系统设置', subtitle: '配置与管理', sections: [] },
}

// ===== 模块化页面导航自动挂载 =====
// src/modules/* 声明的 nav 落入其 meta.module 对应模块的侧栏；新增模块页面不需要再改本文件。
for (const item of collectNav()) {
  const cfg = MODULE_SIDEBAR_CONFIG[item.module]
  if (!cfg) continue
  const title = item.section || '模块页面'
  let section = cfg.sections.find(s => s.title === title)
  if (!section) { section = { title, items: [] }; cfg.sections.push(section) }
  if (!section.items.some(i => i.path === item.path)) {
    section.items.push({ key: item.key, label: item.label, icon: item.icon, path: item.path })
  }
}
