<template>
  <div class="page-container expert-center">
    <!-- 简洁页头 -->
    <div class="page-header compact-header">
      <div class="header-left">
        <div class="brand-mini">
          <div class="brand-dot"></div>
          <span class="brand-name">专家联盟</span>
          <el-tag v-if="currentProject" size="small" class="project-tag" effect="plain">
            <el-icon><Folder /></el-icon>
            {{ currentProject.name }}
          </el-tag>
        </div>
      </div>
      <div class="header-right">
        <el-button size="small" type="primary" plain @click="showRegister = true">
          <el-icon><Plus /></el-icon> 注册专家
        </el-button>
        <el-button size="small" @click="ensureProject">
          <el-icon><FolderAdd /></el-icon> 切换项目
        </el-button>
      </div>
    </div>

    <!-- Tab 切换 -->
    <el-tabs v-model="activeTab" class="expert-tabs compact-tabs" @tab-change="onTabChange">
      <el-tab-pane label="联盟总览" name="overview" />
      <el-tab-pane label="联盟任务" name="tasks" />
      <el-tab-pane label="企业管理" name="enterprise" />
      <el-tab-pane label="编排引擎" name="orchestrator" />
    </el-tabs>

    <!-- 总览 Tab → 新组件 -->
    <div v-show="activeTab === 'overview'" class="tab-content">
      <ExpertOverviewPanel />
    </div>

    <!-- 企业管理 / 编排引擎 Tab 内容（嵌套路由渲染） -->
    <router-view v-if="activeTab !== 'overview'" v-slot="{ Component }">
      <transition name="fade" mode="out-in">
        <component :is="Component" />
      </transition>
    </router-view>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  Plus, Refresh, Close, Promotion, ChatDotRound, MagicStick,
  DataAnalysis, Timer, TrendCharts, Cpu, Guide, Search, Folder, FolderAdd
} from '@element-plus/icons-vue'
import {
  getExperts, registerExpert, consultExpert, multiExpertConsult, expertDebate,
  routeExperts, intelligentConsult, algorithmAnalysis,
  getExpertMetrics, getExpertOverview
} from '@/api'
import { useProject } from '@/composables/projectContext.js'
import ExpertOverviewPanel from './panels/ExpertOverviewPanel.vue'

const router = useRouter()
const route = useRoute()

// Tab 切换：联盟总览 / 企业管理 / 编排引擎
// 使用嵌套路由驱动：从子路由名推断 activeTab
const activeTab = computed(() => {
  const name = route.name?.toString() || 'ExpertOverview'
  if (name.includes('Tasks')) return 'tasks'
  if (name.includes('Enterprise')) return 'enterprise'
  if (name.includes('Orchestrator')) return 'orchestrator'
  // 兼容旧的 query.tab 链接
  const q = route.query.tab
  if (q === 'tasks' || q === 'enterprise' || q === 'orchestrator') return q
  return 'overview'
})
function onTabChange(tab) {
  const routes = {
    overview: '/expert-center/overview',
    tasks: '/expert-center/tasks',
    enterprise: '/expert-center/enterprise',
    orchestrator: '/expert-center/orchestrator'
  }
  router.push(routes[tab] || routes.overview)
}
// 项目上下文（来自顶栏 ProjectPicker，共享状态）
const { currentProject, ensureProjectContext, createAndSelect } = useProject()

const experts = ref([])
const filterType = ref('')
const keyword = ref('')
const selectedExpertIds = ref([])
const mode = ref('smart')
const question = ref('')
const consulting = ref(false)
const routingLoading = ref(false)
const results = ref([])
const debateSummary = ref('')
const algorithmResult = ref(null)
const rounds = ref(2)
const debateStrategy = ref('round_robin')
const smartMode = ref(true)

const showRegister = ref(false)
const registering = ref(false)
const newExpert = ref({ name: '', type: 'algorithm', capabilities_str: '', description: '', systemPrompt: '' })

const overview = ref(null)
const overviewLoading = ref(false)
const metricsList = ref([])
const routingResult = ref(null)
const useGraphData = ref(false)
const graphDataJson = ref('')

const typeLabels = {
  algorithm: '算法专家', architecture: '架构专家', data: '数据专家',
  ai: 'AI专家', workflow: '工作流专家', operator: '算子系统专家',
  graph: '知识图谱专家', security: '安全专家', performance: '性能优化专家',
  monitor: '可观测性专家', market: '商业智能专家', mcp: 'MCP协议专家',
  automation: '自动化专家', requirement: '需求工程专家', fusion: '融合专家',
  custom: '自定义专家'
}

const expertTypes = computed(() => Object.keys(typeLabels))

function typeLabel(t) { return typeLabels[t] || t }

function getColor(type) {
  const colors = {
    algorithm: '#6366f1', architecture: '#0891b2', data: '#10b981',
    ai: '#ec4899', workflow: '#f59e0b', operator: '#8b5cf6',
    graph: '#06b6d4', security: '#ef4444', performance: '#14b8a6',
    monitor: '#f97316', market: '#f43f5e', mcp: '#a855f7',
    automation: '#0ea5e9', requirement: '#16a34a', fusion: '#7c3aed',
    custom: '#64748b'
  }
  return colors[type] || colors.custom
}

function getColorByType(type) { return getColor(type) }

function getIcon(type) {
  const icons = {
    algorithm: 'TrendCharts', architecture: 'Grid', data: 'Coin',
    ai: 'MagicStick', workflow: 'Operation', operator: 'Cpu',
    graph: 'Share', security: 'Lock', performance: 'Lightning',
    monitor: 'DataLine', market: 'Shop', mcp: 'Link',
    automation: 'MagicStick', requirement: 'Tickets', fusion: 'Aim',
    custom: 'User'
  }
  return icons[type] || icons.custom
}

function getSuccessColor(rate) {
  if (!rate) return '#ef4444'
  if (rate >= 0.95) return '#10b981'
  if (rate >= 0.85) return '#3b82f6'
  if (rate >= 0.7) return '#f59e0b'
  return '#ef4444'
}

const filteredExperts = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  return experts.value.filter(e => {
    const matchType = !filterType.value || e.type === filterType.value
    const matchKw = !kw ||
      e.name.toLowerCase().includes(kw) ||
      (e.description || '').toLowerCase().includes(kw) ||
      (e.capabilities || []).some(c => c.toLowerCase().includes(kw))
    return matchType && matchKw
  })
})

function getExpertName(id) {
  return experts.value.find(e => e.id === id)?.name || id
}

function toggleSelect(exp) {
  if (exp.status !== 'active') {
    ElMessage.warning('该专家当前不在线')
    return
  }
  const idx = selectedExpertIds.value.indexOf(exp.id)
  if (idx !== -1) {
    selectedExpertIds.value.splice(idx, 1)
  } else {
    if (mode.value === 'single' && selectedExpertIds.value.length >= 1) {
      selectedExpertIds.value = [exp.id]
    } else {
      selectedExpertIds.value.push(exp.id)
    }
  }
}

function isSelected(id) { return selectedExpertIds.value.includes(id) }
function selectedCount() { return selectedExpertIds.value.length }
function removeExpert(id) {
  const idx = selectedExpertIds.value.indexOf(id)
  if (idx !== -1) selectedExpertIds.value.splice(idx, 1)
}

async function loadExperts() {
  try {
    experts.value = await getExperts()
  } catch (e) {
    ElMessage.error('加载专家列表失败：' + e.message)
  }
}

async function loadOverview() {
  overviewLoading.value = true
  try {
    overview.value = await getExpertOverview()
  } catch (e) {
    ElMessage.error('加载系统概览失败：' + e.message)
  } finally {
    overviewLoading.value = false
  }
}

async function loadMetrics() {
  try {
    const data = await getExpertMetrics()
    metricsList.value = data.metrics || []
  } catch (e) {
    console.error('加载指标失败：', e.message)
  }
}

async function loadAll() {
  await Promise.all([loadExperts(), loadOverview(), loadMetrics()])
  ElMessage.success('数据已刷新')
}

async function doRouteOnly() {
  if (!question.value.trim()) return
  routingLoading.value = true
  try {
    routingResult.value = await routeExperts({
      question: question.value,
      maxExperts: 3
    })
  } catch (e) {
    ElMessage.error('路由分析失败：' + e.message)
  } finally {
    routingLoading.value = false
  }
}

async function doSmartRoute() {
  if (!question.value.trim()) return
  consulting.value = true
  results.value = []
  debateSummary.value = ''
  algorithmResult.value = null

  try {
    const result = await intelligentConsult({
      question: question.value,
      mode: 'auto'
    })

    routingResult.value = result.routing

    if (result.mode === 'single') {
      results.value = [{
        expert: result.expert,
        response: result.response,
        confidence: result.metadata?.confidence,
        duration_ms: result.metadata?.duration_ms
      }]
    } else if (result.mode === 'multi') {
      results.value = result.results.filter(r => r.success).map(r => ({
        expert: r.expert,
        response: r.response,
        confidence: r.confidence,
        duration_ms: r.duration_ms
      }))
    } else if (result.mode === 'debate') {
      results.value = []
      result.history.forEach((round, idx) => {
        round.results.forEach(r => {
          if (r.success) {
            results.value.push({
              expert: r.expert,
              response: r.response,
              round: idx + 1,
              confidence: r.confidence,
              duration_ms: r.duration_ms
            })
          }
        })
      })
      debateSummary.value = result.final_synthesis
    }

    ElMessage.success(`智能路由完成，模式: ${result.mode}`)
    await loadMetrics()
  } catch (e) {
    ElMessage.error('智能咨询失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doConsult() {
  const expertId = selectedExpertIds.value[0]
  if (!expertId || !question.value.trim()) return

  consulting.value = true
  results.value = []
  debateSummary.value = ''
  algorithmResult.value = null

  try {
    const result = await consultExpert(expertId, {
      messages: [{ role: 'user', content: question.value }]
    })
    results.value = [{
      expert: { id: expertId, name: getExpertName(expertId) },
      response: result.response,
      confidence: result.metadata?.confidence,
      duration_ms: result.metadata?.duration_ms
    }]
    ElMessage.success('咨询完成')
  } catch (e) {
    ElMessage.error('咨询失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doMultiConsult() {
  const expertIds = [...selectedExpertIds.value]
  if (expertIds.length < 2 || !question.value.trim()) return

  consulting.value = true
  results.value = []
  debateSummary.value = ''
  algorithmResult.value = null

  try {
    const result = await multiExpertConsult({
      question: question.value,
      expert_ids: expertIds
    })
    results.value = result.results.filter(r => r.success).map(r => ({
      expert: r.expert,
      response: r.response,
      confidence: r.confidence,
      duration_ms: r.duration_ms
    }))
    ElMessage.success(`协同分析完成，共 ${result.successful} 位专家参与`)
  } catch (e) {
    ElMessage.error('协同分析失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doDebate() {
  const expertIds = [...selectedExpertIds.value]
  if (expertIds.length < 2 || !question.value.trim()) return

  consulting.value = true
  results.value = []
  debateSummary.value = ''
  algorithmResult.value = null

  try {
    const result = await expertDebate({
      question: question.value,
      expert_ids: expertIds,
      rounds: rounds.value
    })
    results.value = []
    result.history.forEach((round, idx) => {
      round.results.forEach(r => {
        if (r.success) {
          results.value.push({
            expert: r.expert,
            response: r.response,
            round: idx + 1,
            confidence: r.confidence
          })
        }
      })
    })
    debateSummary.value = result.final_synthesis
    ElMessage.success(`辩论完成，共 ${rounds.value} 轮`)
  } catch (e) {
    ElMessage.error('辩论失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doAlgorithmAnalysis() {
  if (!question.value.trim()) return

  consulting.value = true
  results.value = []
  debateSummary.value = ''
  algorithmResult.value = null

  try {
    let graphData = null
    if (useGraphData.value && graphDataJson.value.trim()) {
      try {
        graphData = JSON.parse(graphDataJson.value)
      } catch (e) {
        ElMessage.error('图谱数据 JSON 格式错误')
        consulting.value = false
        return
      }
    }

    const result = await algorithmAnalysis({
      question: question.value,
      graphData,
      options: {}
    })
    algorithmResult.value = result
    ElMessage.success('算法分析完成')
  } catch (e) {
    ElMessage.error('算法分析失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doRegister() {
  if (!newExpert.value.name || !newExpert.value.type) {
    ElMessage.warning('请填写专家名称和类型')
    return
  }

  registering.value = true
  try {
    await registerExpert({
      name: newExpert.value.name,
      type: newExpert.value.type,
      capabilities: (newExpert.value.capabilities_str || '').split(',').map(s => s.trim()).filter(Boolean),
      description: newExpert.value.description,
      systemPrompt: newExpert.value.systemPrompt
    })
    ElMessage.success('注册成功')
    showRegister.value = false
    newExpert.value = { name: '', type: 'algorithm', capabilities_str: '', description: '', systemPrompt: '' }
    await loadExperts()
  } catch (e) {
    ElMessage.error('注册失败：' + e.message)
  } finally {
    registering.value = false
  }
}

watch(mode, () => {
  if (mode.value !== 'single') {
    selectedExpertIds.value = []
  }
})

// ==================== 璇玑 Mox Graph · 简易 Canvas 力导向渲染 ====================
const graphCanvasRef = ref(null)
const graphStageRef = ref(null)
const graphData = reactive({ nodes: [], edges: [] })
const graphStats = computed(() => {
  const n = graphData.nodes.length
  const e = graphData.edges.length
  const density = n > 1 ? ((2 * e) / (n * (n - 1))).toFixed(2) : '0.00'
  return { nodes: n, edges: e, density }
})
let rafId = 0

// 公司官网需求图谱 · 预设数据（用于无项目场景的展示）
function buildMockGraph() {
  const pj = currentProject.value ? currentProject.value.name : '公司官网'
  const NODES = [
    { id: 'P1', type: 'project', label: pj, fixed: true },
    { id: 'G1', type: 'goal', label: '品牌展示' },
    { id: 'G2', type: 'goal', label: '线索转化' },
    { id: 'G3', type: 'goal', label: 'SEO 排名' },
    { id: 'A1', type: 'actor', label: '访客' },
    { id: 'A2', type: 'actor', label: '运营' },
    { id: 'A3', type: 'actor', label: '管理员' },
    { id: 'U1', type: 'usecase', label: '首页浏览' },
    { id: 'U2', type: 'usecase', label: '产品介绍' },
    { id: 'U3', type: 'usecase', label: '表单留资' },
    { id: 'U4', type: 'usecase', label: '新闻/博客' },
    { id: 'U5', type: 'usecase', label: '后台管理' },
    { id: 'D1', type: 'data', label: '用户线索' },
    { id: 'D2', type: 'data', label: '内容数据' },
    { id: 'D3', type: 'data', label: '产品数据' },
    { id: 'T1', type: 'tech', label: 'Vue 3 + Vite' },
    { id: 'T2', type: 'tech', label: 'Element Plus' },
    { id: 'T3', type: 'tech', label: 'NestJS + Postgres' },
    { id: 'T4', type: 'tech', label: 'SEO SSR' },
    { id: 'E1', type: 'end', label: '性能验收' },
    { id: 'E2', type: 'end', label: '上线 Checklist' }
  ]
  const EDGES = [
    ['P1', 'G1', 'contains', '包含'],
    ['P1', 'G2', 'contains', '包含'],
    ['P1', 'G3', 'contains', '包含'],
    ['P1', 'A1', 'serves', '服务于'],
    ['P1', 'A2', 'serves', '服务于'],
    ['P1', 'A3', 'serves', '服务于'],
    ['G1', 'U1', 'realizedBy', '通过'],
    ['G1', 'U2', 'realizedBy', '通过'],
    ['G2', 'U3', 'realizedBy', '通过'],
    ['G3', 'U4', 'realizedBy', '通过'],
    ['A3', 'U5', 'perform', '执行'],
    ['U1', 'T1', 'implement', '实现'],
    ['U1', 'T2', 'implement', '实现'],
    ['U3', 'D1', 'produce', '产生'],
    ['U4', 'D2', 'produce', '产生'],
    ['U2', 'D3', 'read', '读取'],
    ['D1', 'T3', 'persist', '持久化'],
    ['D2', 'T3', 'persist', '持久化'],
    ['D3', 'T3', 'persist', '持久化'],
    ['U4', 'T4', 'optimize', 'SEO优化'],
    ['E1', 'G1', 'verify', '验证'],
    ['E2', 'P1', 'gate', '门禁']
  ]
  return {
    nodes: NODES.map((n, i) => ({ ...n, x: 0, y: 0, vx: 0, vy: 0, index: i })),
    edges: EDGES.map((e) => ({ source: e[0], target: e[1], type: e[2], label: e[3] }))
  }
}

function layoutSeed(nodes) {
  const w = 360
  const h = 320
  const cx = w / 2
  const cy = h / 2
  // 中心项目，其余按环形分散
  nodes.forEach((n, i) => {
    if (n.fixed) {
      n.x = cx; n.y = cy
    } else {
      const angle = (i / Math.max(1, nodes.length - 1)) * Math.PI * 2 + 0.2
      const r = 110 + (i % 3) * 18
      n.x = cx + Math.cos(angle) * r
      n.y = cy + Math.sin(angle) * r
    }
    n.vx = 0; n.vy = 0
  })
}

function simulateOnce(nodes, edges, w, h) {
  // 简易物理
  const cx = w / 2
  const cy = h / 2
  // 中心弱吸附
  nodes.forEach((n) => {
    if (n.fixed) { n.vx *= 0.5; n.vy *= 0.5; return }
    const dx = cx - n.x
    const dy = cy - n.y
    n.vx += dx * 0.0004
    n.vy += dy * 0.0004
  })
  // 斥力
  for (let i = 0; i < nodes.length; i++) {
    const a = nodes[i]
    for (let j = i + 1; j < nodes.length; j++) {
      const b = nodes[j]
      let dx = a.x - b.x
      let dy = a.y - b.y
      let d2 = dx * dx + dy * dy + 0.001
      const force = 1800 / d2
      const d = Math.sqrt(d2)
      dx /= d; dy /= d
      if (!a.fixed) { a.vx += dx * force; a.vy += dy * force }
      if (!b.fixed) { b.vx -= dx * force; b.vy -= dy * force }
    }
  }
  // 弹簧
  edges.forEach((e) => {
    const s = nodes.find((n) => n.id === e.source)
    const t = nodes.find((n) => n.id === e.target)
    if (!s || !t) return
    let dx = t.x - s.x
    let dy = t.y - s.y
    let d = Math.sqrt(dx * dx + dy * dy) + 0.001
    const rest = 78
    const diff = (d - rest) / d
    const f = diff * 0.015
    dx *= f; dy *= f
    if (!s.fixed) { s.vx += dx; s.vy += dy }
    if (!t.fixed) { t.vx -= dx; t.vy -= dy }
  })
  // 阻尼 & 位置
  nodes.forEach((n) => {
    n.vx *= 0.82
    n.vy *= 0.82
    n.x += n.vx
    n.y += n.vy
    // 边界
    n.x = Math.max(18, Math.min(w - 18, n.x))
    n.y = Math.max(18, Math.min(h - 18, n.y))
  })
}

const TYPE_STYLE = {
  project:  { color: '#6366f1', r: 16, label: '项目' },
  goal:     { color: '#0ea5e9', r: 12, label: '目标' },
  actor:    { color: '#f59e0b', r: 12, label: '角色' },
  usecase:  { color: '#10b981', r: 11, label: '用例' },
  data:     { color: '#ef4444', r: 11, label: '数据' },
  tech:     { color: '#8b5cf6', r: 11, label: '技术' },
  end:      { color: '#64748b', r: 12, label: '验收' }
}

function drawGraph() {
  const canvas = graphCanvasRef.value
  const stage = graphStageRef.value
  if (!canvas || !stage) return
  const W = stage.clientWidth
  const H = stage.clientHeight || 320
  const dpr = window.devicePixelRatio || 1
  canvas.width = W * dpr
  canvas.height = H * dpr
  canvas.style.width = W + 'px'
  canvas.style.height = H + 'px'
  const ctx = canvas.getContext('2d')
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)

  function tick() {
    simulateOnce(graphData.nodes, graphData.edges, W, H)
    ctx.clearRect(0, 0, W, H)
    // 边
    graphData.edges.forEach((e) => {
      const s = graphData.nodes.find((n) => n.id === e.source)
      const t = graphData.nodes.find((n) => n.id === e.target)
      if (!s || !t) return
      ctx.beginPath()
      ctx.strokeStyle = 'rgba(100,116,139,0.34)'
      ctx.lineWidth = 1
      ctx.moveTo(s.x, s.y)
      ctx.lineTo(t.x, t.y)
      ctx.stroke()
    })
    // 节点
    graphData.nodes.forEach((n) => {
      const ts = TYPE_STYLE[n.type] || TYPE_STYLE.goal
      ctx.beginPath()
      ctx.fillStyle = ts.color
      ctx.arc(n.x, n.y, ts.r, 0, Math.PI * 2)
      ctx.globalAlpha = 0.92
      ctx.fill()
      ctx.globalAlpha = 0.22
      ctx.strokeStyle = ts.color
      ctx.lineWidth = 4
      ctx.stroke()
      ctx.globalAlpha = 1
      // 标签
      ctx.font = '11px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif'
      ctx.fillStyle = '#1e293b'
      ctx.textAlign = 'center'
      ctx.textBaseline = 'top'
      ctx.fillText(n.label, n.x, n.y + ts.r + 4)
    })
    rafId = requestAnimationFrame(tick)
  }
  cancelAnimationFrame(rafId)
  layoutSeed(graphData.nodes)
  // 快速跑几步以形成较好初始布局
  for (let i = 0; i < 40; i++) simulateOnce(graphData.nodes, graphData.edges, W, H)
  tick()
}

function randomizeGraph() {
  const mock = buildMockGraph()
  graphData.nodes.splice(0, graphData.nodes.length, ...mock.nodes)
  graphData.edges.splice(0, graphData.edges.length, ...mock.edges)
  nextTick(() => drawGraph())
}

// ==================== 快捷问法 / 流程阶段 ====================
const showMetricsFull = ref(false)
const showMoreQuick = ref(false)

const PHASES = [
  { key: 'requirement', label: '📋 需求阶段', desc: '项目对话 / 需求编译 / 知识库', color: '#6366f1' },
  { key: 'architecture', label: '🏗️ 架构阶段', desc: '知识图谱 / 专家联盟 / 全维融合', color: '#06b6d4' },
  { key: 'develop',     label: '⚙️ 开发阶段', desc: '算子 / 工作流 / 插件 / 自动化', color: '#10b981' },
  { key: 'release',     label: '🚀 发布阶段', desc: '监控 / 文档 / 系统管理', color: '#f59e0b' }
]

const FLOW_STAGES = [
  { key: 'req',   label: '①需求采集 · 意图识别', hint: '用自然语言描述你的项目或问题，系统自动采集角色/目标/约束' },
  { key: 'arch',  label: '②架构设计 · 专家辩论', hint: '架构 / 算法 / 数据 / 图谱 多位专家组队辩论' },
  { key: 'impl',  label: '③实现方案 · 工作流编排', hint: '输出 PRD / ERD / 流程图，并编排开发工作流' },
  { key: 'test',  label: '④开发测试 · 迭代修复', hint: '通过算法联盟 + 浏览器任务，驱动开发/测试循环' },
  { key: 'acc',   label: '⑤验收发布 · 门禁审批', hint: '生成验收清单，对接发布与监控看板' },
  { key: 'done',  label: '⑥归档沉淀 · 知识库',   hint: '产物归档入知识库，并形成最佳实践沉淀' }
]
const localPhase = ref('requirement')
const currentStage = ref(0)
const requirementFlowMode = ref(false)
const phaseProgress = reactive({ requirement: 8, architecture: 0, develop: 0, release: 0 })
const phaseDone = computed(() => ({
  requirement: phaseProgress.requirement >= 100,
  architecture: phaseProgress.architecture >= 100,
  develop:     phaseProgress.develop >= 100,
  release:     phaseProgress.release >= 100
}))
const phaseCompleteCount = computed(() => Object.values(phaseDone.value).filter(Boolean).length)
const projectOverall = computed(() => {
  const values = Object.values(phaseProgress)
  if (!values.length) return 0
  return Math.round(values.reduce((a, b) => a + b, 0) / values.length)
})

function selectPhase(key) {
  localPhase.value = key
  const mapStage = { requirement: 0, architecture: 1, develop: 2, release: 3 }
  currentStage.value = mapStage[key] ?? 0
  // 同步通知顶部 PhasePipeline
  try {
    window.dispatchEvent(new CustomEvent('mox:set-phase', { detail: { key } }))
  } catch (_) {}
}

function advanceFlowStage() {
  if (currentStage.value < 3) currentStage.value++
  const key = ['requirement','architecture','develop','release'][Math.min(3, currentStage.value)]
  if (key) localPhase.value = key
  // 模拟推进
  phaseProgress[key] = Math.min(100, (phaseProgress[key] || 0) + 14)
}

function runFullFlow() {
  // 前端演示版：逐步推进进度（后端完整流水线由 /ai 页的 alliance SSE 负责）
  doSmartRoute()
}

// ==================== 快捷问法 ====================
const QUICK_QUESTIONS = [
  { icon: '🏢', label: '生成公司官网的需求图谱', hint: '覆盖角色/目标/用例/数据/技术', prompt: '生成「公司官网」的全维需求图谱：含角色、目标、用例、数据、技术选型与验收清单。', chip: '需求知识图谱' },
  { icon: '🧾', label: '需求知识图谱', hint: '生成项目需求的结构化知识图谱', prompt: '请生成当前项目的需求知识图谱，并输出节点与关系边列表。', chip: '需求知识图谱' },
  { icon: '🏗', label: '需求架构（S1）', hint: '输入自定义问题，自动生成架构草案', prompt: '请对当前项目做全维的需求架构分析：业务场景、角色、核心用例、非功能需求。', chip: '需求知识图谱' },
  { icon: '⚙️', label: '算法分', hint: '复杂度 / 推荐 / 数据结构选型', prompt: '对当前对话/问题涉及的算法进行复杂度分析并给出推荐方案。', chip: '算法分析' },
  { icon: '🩸', label: '血清空（初始化会话）', hint: '清空上下文，重新开始项目分析', prompt: '（清空上下文）请从 0 开始重新分析当前项目。', chip: '自定义问题' },
  { icon: '📥', label: '导入需求', hint: '上传文档/JSON 解析为需求节点', prompt: '请导入并解析以下需求文件：', chip: '自定义问题' },
  { icon: '📌', label: '转任务', hint: '将当前问题拆解为可执行任务清单', prompt: '请将本次分析拆解为任务清单，并给出优先级与负责人。', chip: '自定义问题' },
  { icon: '🚀', label: '创建项目', hint: '以当前问题创建一个全新的项目', prompt: '基于以上分析，创建一个新项目并输出项目信息、初始里程碑与阶段划分。', chip: '自定义问题' }
]
const visibleQuickQuestions = computed(() => showMoreQuick.value ? QUICK_QUESTIONS : QUICK_QUESTIONS.slice(0, 4))
const selectedChip = ref('需求知识图谱')
const customPlaceholder = computed(() => {
  if (selectedChip.value === '算法分') return '算法分：输入你想分析的算法/数据结构问题…'
  if (selectedChip.value === '生成公司官网的需求图谱') return '生成公司官网的需求图谱：请描述行业、目标客群、期望栏目…'
  if (selectedChip.value === '需求知识图谱') return '需求知识图谱：请描述项目/产品，系统将生成结构化知识图谱…'
  if (selectedChip.value === '需求架构（S1）') return '需求架构：输入自定义问题，系统将全维分析处理…'
  if (selectedChip.value === '血清空（初始化会话）') return '血清空：请确认要重置上下文，并输入新项目的描述…'
  return '自定义问题：输入你的问题，系统将全维分析处理（支持生成需求图谱 / 架构 / 算法 / 任务拆解 / 创建项目）…'
})

function pickQuick(q) {
  selectedChip.value = q.chip || q.label
  question.value = q.prompt
  if (q.label === '血清空（初始化会话）') {
    results.value = []
    debateSummary.value = ''
    algorithmResult.value = null
    routingResult.value = null
    ElMessage.success('已血清空当前联盟输出，可重新开始分析')
  }
}

// ==================== 流程模式动作 ====================
const modeLabelMap = {
  smart: '智能路由 · 全维',
  single: '单专家',
  multi: '多专家协同',
  debate: '专家辩论',
  algorithm: '算法分析 · 算法分'
}

function askQuickAnalysis() {
  // 快速分析 = 简化版智能路由，仅路由分析 + 咨询
  Promise.all([doRouteOnly()]).then(() => {
    if (routingResult.value?.selected?.length) {
      doSmartRoute()
    }
  })
}
function addFollowUp() {
  if (!question.value.trim()) return
  ElMessage.success('已追加为后续问题 · 发送给 AI 助手 X 可继续追问')
  openNewChatView(question.value)
}
function clearCurrentConversation() {
  results.value = []
  debateSummary.value = ''
  algorithmResult.value = null
  routingResult.value = null
  question.value = ''
  ElMessage.success('已清空当前联盟对话输出')
}
function importGraphData() {
  useGraphData.value = true
  graphDataJson.value = '{"nodes":[{"id":"n1","label":"A"},{"id":"n2","label":"B"}],"edges":[{"source":"n1","target":"n2","type":"rel"}]}'
  ElMessage.success('已注入示例图谱 JSON（可编辑后开始分析）')
}
function convertQuestionToTask() {
  if (!question.value.trim()) return
  ElMessage.success(`已将问题转为任务 · ${question.value.slice(0, 24)}...`)
}
function createProjectFromQuestion() {
  if (!question.value.trim()) return
  ensureAndInjectProject()
}
async function ensureProject() {
  ensureAndInjectProject()
}

// 启动全维开发：跳转到AI助手，带上项目上下文
function startFullDev() {
  ensureAndInjectProject()
  router.push({ path: '/ai', query: { source: 'expert', action: 'full-dev' } })
}

function ensureAndInjectProject() {
  if (currentProject.value) {
    ElMessage.info(`当前项目：${currentProject.value.name}，可继续跟进。`)
    return
  }
  const suggestion = question.value
    ? (question.value.slice(0, 16) + '…')
    : '新璇玑项目'
  const pj = {
    id: 'pj_' + Date.now().toString(36),
    name: suggestion || '璇玑联盟新项目',
    description: question.value || '由专家联盟创建',
    category: selectedChip.value === '生成公司官网的需求图谱' ? '官网/营销' : '定制软件',
    status: '规划中'
  }
  try {
    // 若 createAndSelect 存在则用；不存在则降级本地模拟
    if (typeof createAndSelect === 'function') {
      createAndSelect(pj)
    }
    ElMessage.success(`已创建并选择项目：${pj.name}`)
  } catch (e) {
    ElMessage.warning(e.message || '创建项目失败')
  }
}
function openNewChatView(withInitial) {
  const query = {}
  if (withInitial && String(withInitial).trim()) query.initial = encodeURIComponent(withInitial)
  if (currentProject.value?.id) query.projectId = currentProject.value.id
  router.push({ path: '/ai', query })
}
function goToGraphPage() {
  const q = currentProject.value?.id ? { projectId: currentProject.value.id } : {}
  router.push({ path: '/graph', query })
}
function copyText(t) {
  try {
    const ta = document.createElement('textarea')
    ta.value = t || ''
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
    ElMessage.success('已复制到剪贴板')
  } catch (_) {
    ElMessage.warning('复制失败，请手动选择文本')
  }
}
function appendResultAsInput(r) {
  const before = question.value ? question.value + '\n' : ''
  question.value = before + '【后续分析】' + String((r && r.response) || '').slice(0, 240)
}
function exportConversation() {
  const payload = {
    project: currentProject.value || null,
    mode: mode.value,
    question: question.value,
    routing: routingResult.value || null,
    results: results.value,
    algorithmResult: algorithmResult.value || null,
    debateSummary: debateSummary.value || null,
    exportedAt: new Date().toISOString()
  }
  try {
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `expert-alliance-${Date.now()}.json`
    a.click()
    URL.revokeObjectURL(url)
    ElMessage.success('已导出联盟对话 + 路由 + 结果为 JSON')
  } catch (_) {
    ElMessage.warning('导出失败')
  }
}

// 页面阶段变化：更新 PhasePipeline 的进度（演示）
watch(localPhase, (k) => {
  phaseProgress[k] = Math.max(phaseProgress[k] || 0, 10 + Math.round(Math.random() * 8))
})

// Canvas 大小变化时重绘
let resizeObs = null
onMounted(async () => {
  await loadAll()
  // 初始化 Mock 图谱
  const mock = buildMockGraph()
  graphData.nodes.push(...mock.nodes)
  graphData.edges.push(...mock.edges)
  await nextTick()
  try {
    if (window.ResizeObserver && graphStageRef.value) {
      resizeObs = new ResizeObserver(() => drawGraph())
      resizeObs.observe(graphStageRef.value)
    }
  } catch (_) {}
  drawGraph()
})
onBeforeUnmount(() => {
  cancelAnimationFrame(rafId)
  if (resizeObs) try { resizeObs.disconnect() } catch (_) {}
})
</script>

<style scoped>
.expert-center {
  display: flex;
  flex-direction: column;
  gap: 10px;
  height: 100%;
}

/* 简洁页头 */
.compact-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  background: #fff;
  border-radius: 12px;
  border: 1px solid #e2e8f0;
  height: 48px;
  min-height: 48px;
}

.header-left, .header-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.brand-mini {
  display: flex;
  align-items: center;
  gap: 10px;
}

.brand-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: linear-gradient(135deg, #6366f1, #10b981);
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
}

.brand-name {
  font-size: 15px;
  font-weight: 700;
  color: #0f172a;
}

.project-tag {
  margin-left: 4px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

/* Tab 样式 */
.compact-tabs {
  margin: 0;
}
:deep(.expert-tabs .el-tabs__header) {
  margin-bottom: 0;
  padding: 0 6px;
  background: var(--bg-card);
  border-radius: 12px;
  border: 1px solid var(--border-1);
}
:deep(.expert-tabs .el-tabs__nav-wrap::after) {
  display: none;
}
:deep(.expert-tabs .el-tabs__item) {
  font-weight: 600;
  font-size: 14px;
  height: 44px;
  line-height: 44px;
}
.tab-panel {
  width: 100%;
}
.tab-content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 4px 2px 0;
}
.head-left { flex: 1; min-width: 0; }
.head-brand {
  display: flex;
  align-items: center;
  gap: 14px;
}
.brand-mark {
  width: 46px; height: 46px; flex-shrink: 0;
  background: #ffffff;
  border-radius: 14px;
  display: grid;
  place-items: center;
  box-shadow: 0 4px 14px -6px rgba(99,102,241,0.28);
  border: 1px solid rgba(99,102,241,0.12);
}
.bm-svg { width: 34px; height: 34px; }
.head-titles { min-width: 0; flex: 1; }
.head-title-row {
  display: flex; align-items: center; gap: 10px;
  margin-bottom: 2px;
}
.page-title {
  margin: 0;
  font-size: 19px;
  font-weight: 800;
  letter-spacing: 0.2px;
  background: linear-gradient(135deg, #6366f1, #0ea5e9 55%, #10b981);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}
.ver-tag {
  background: linear-gradient(135deg, #6366f1, #0ea5e9) !important;
  border: none !important;
  font-size: 11px;
  padding: 0 8px;
  height: 22px;
  line-height: 20px;
}
.ver-tag-alt {
  color: #6366f1 !important;
  border-color: #c7d2fe !important;
  font-size: 11px;
  height: 22px;
  line-height: 20px;
  padding: 0 8px;
}
.page-subtitle {
  margin: 0;
  font-size: 12.5px;
  color: var(--text-3);
  line-height: 1.7;
}
.hl-project {
  color: var(--brand-dark);
  background: var(--brand-soft);
  padding: 1px 8px;
  border-radius: 6px;
  font-weight: 600;
}
.proj-cat {
  margin-left: 6px;
  font-size: 11px;
  color: #10b981;
  background: #d1fae5;
  padding: 1px 6px;
  border-radius: 6px;
}
.muted-plain { color: var(--text-3); }
.link-like { cursor: pointer; color: var(--brand); text-underline-offset: 3px; }
.link-like:hover { color: var(--brand-dark); }
.head-actions {
  display: flex; gap: 8px; align-items: center;
  flex-shrink: 0;
}

.phi-shell {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(300px, 382px) minmax(0, 1fr) minmax(340px, 420px);
  grid-template-rows: 1fr;
  gap: 14px;
}
.col {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
  min-height: 0;
}
@media (max-width: 1400px) {
  .phi-shell {
    grid-template-columns: 300px 1fr;
  }
  .phi-shell .col-right {
    grid-column: 1 / -1;
  }
}
@media (max-width: 900px) {
  .phi-shell {
    grid-template-columns: 1fr;
  }
}

.card {
  background: #ffffff;
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 16px 16px 16px;
  box-shadow: 0 1px 2px rgba(15,23,42,0.03);
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
  min-height: 0;
}
.card-tight { padding: 12px 14px; }
.card-head {
  display: flex;
  align-items: baseline;
  gap: 10px;
}
.card-head.between { justify-content: space-between; align-items: center; }
.card-title {
  font-size: 13px;
  font-weight: 700;
  color: #0f172a;
  letter-spacing: 0.2px;
}
.card-title-wrap { display: flex; align-items: center; gap: 8px; }
.card-sub {
  font-size: 11px;
  color: var(--text-3);
}
.count-pill {
  font-size: 11px;
  color: var(--brand-dark);
  background: var(--brand-soft);
  padding: 2px 8px;
  border-radius: 999px;
  font-weight: 600;
}
.section-title {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
}

.phase-nav .phase-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.phase-row {
  display: grid;
  grid-template-columns: 30px 1fr 60px 14px;
  align-items: center;
  gap: 10px;
  padding: 10px 10px;
  border-radius: 11px;
  cursor: pointer;
  transition: all 0.18s ease;
  border: 1px solid transparent;
}
.phase-row:hover { background: #f8fafc; }
.phase-row.active {
  background: linear-gradient(135deg, rgba(99,102,241,0.08), rgba(16,185,129,0.06));
  border-color: rgba(99,102,241,0.25);
  box-shadow: inset 3px 0 0 var(--brand, #6366f1);
}
.phase-row.done { opacity: 0.9; }
.phase-idx {
  width: 26px; height: 26px;
  border-radius: 8px;
  color: #fff;
  font-size: 12px;
  font-weight: 800;
  display: grid;
  place-items: center;
}
.phase-body { min-width: 0; }
.phase-name {
  font-size: 13px;
  font-weight: 700;
  color: #0f172a;
  line-height: 1.2;
}
.phase-desc {
  font-size: 11px;
  color: var(--text-3);
  margin-top: 2px;
  line-height: 1.4;
}
.phase-bar {
  height: 4px;
  background: #e2e8f0;
  border-radius: 999px;
  overflow: hidden;
}
.phase-bar-fill {
  height: 100%;
  border-radius: 999px;
  transition: width 0.3s ease;
}
.phase-chev {
  color: #cbd5e1;
  font-size: 18px;
  line-height: 1;
  text-align: right;
}
.phase-row.active .phase-chev { color: var(--brand-dark); }

.experts-card { flex: 1; min-height: 0; }
.exp-scroll {
  max-height: 52vh;
  min-height: 300px;
  flex: 1;
}
.filter-bar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.expert-card {
  position: relative;
  display: grid;
  grid-template-columns: 40px 1fr auto;
  align-items: start;
  gap: 10px;
  padding: 10px 10px;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.18s ease;
  border: 1.5px solid transparent;
  margin-bottom: 6px;
  background: #ffffff;
}
.expert-card:hover { background: #f8fafc; border-color: #e2e8f0; }
.expert-card.sel {
  background: linear-gradient(135deg, rgba(99,102,241,0.07), rgba(14,165,233,0.05));
  border-color: #6366f1;
  box-shadow: 0 6px 18px -14px rgba(99,102,241,0.55);
}
.expert-card.offline { opacity: 0.6; }
.expert-avatar {
  width: 40px;
  height: 40px;
  border-radius: 11px;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 18px;
}
.expert-info { min-width: 0; }
.expert-name-row { display: flex; align-items: center; gap: 6px; }
.expert-name { font-weight: 700; font-size: 13px; color: #0f172a; }
.consult-count {
  font-size: 10px;
  background: var(--brand);
  color: #fff;
  padding: 1px 6px;
  border-radius: 10px;
  font-weight: 600;
}
.expert-type { font-size: 11px; color: var(--text-3); margin: 2px 0; }
.expert-stats { display: flex; gap: 8px; font-size: 10.5px; color: var(--text-3); margin: 3px 0; }
.stat-item { display: inline-flex; align-items: center; gap: 2px; }
.expert-caps { display: flex; gap: 4px; flex-wrap: wrap; }
.expert-check {
  width: 20px; height: 20px;
  border-radius: 50%;
  background: var(--brand);
  color: #fff;
  font-size: 12px;
  font-weight: 800;
  display: grid;
  place-items: center;
  align-self: center;
}
.sel-summary {
  margin-top: 4px;
  background: #f8fafc;
  border: 1px dashed #cbd5e1;
  border-radius: 10px;
  padding: 8px 10px;
}
.sel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}
.sel-title { font-size: 12px; font-weight: 700; color: #0f172a; }
.sel-chips { display: flex; gap: 6px; flex-wrap: wrap; }
.chip-sel {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 6px 3px 6px;
  border-radius: 999px;
  background: #fff;
  border: 1px solid #e2e8f0;
  font-size: 11.5px;
  font-weight: 600;
  color: #334155;
}
.chip-dot { width: 8px; height: 8px; border-radius: 50%; }

.mid-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.mh-left { display: flex; align-items: center; gap: 12px; min-width: 0; }
.mh-logo {
  width: 42px; height: 42px;
  border-radius: 12px;
  background: linear-gradient(135deg, rgba(99,102,241,0.10), rgba(14,165,233,0.08));
  display: grid;
  place-items: center;
}
.ai-logo-svg { width: 30px; height: 30px; }
.mh-text { min-width: 0; }
.mh-title {
  font-size: 14.5px;
  font-weight: 800;
  color: #0f172a;
  display: flex;
  align-items: center;
  gap: 8px;
}
.badge-ver {
  font-size: 10.5px;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
  padding: 1px 8px;
  border-radius: 999px;
  font-weight: 600;
}
.mh-sub {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 2px;
}
.chip-selected {
  display: inline-block;
  padding: 1px 8px;
  background: linear-gradient(135deg, #ede9fe, #dbeafe);
  color: #4338ca;
  border-radius: 6px;
  font-weight: 700;
  font-size: 11.5px;
  margin: 0 2px;
}
.mh-right { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }

.quick-q-card { gap: 8px; }
.qq-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.qq-label {
  font-size: 12px;
  font-weight: 700;
  color: #0f172a;
}
.qq-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}
@media (max-width: 1100px) { .qq-grid { grid-template-columns: 1fr; } }
.qq-btn {
  display: grid;
  grid-template-columns: 26px 1fr 16px;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  cursor: pointer;
  text-align: left;
  transition: all 0.16s ease;
}
.qq-btn:hover {
  border-color: #c7d2fe;
  box-shadow: 0 6px 16px -10px rgba(99,102,241,0.45);
  transform: translateY(-1px);
  background: linear-gradient(135deg, #fafbff, #f8fbff);
}
.qq-btn.active {
  border-color: #6366f1;
  background: linear-gradient(135deg, rgba(99,102,241,0.08), rgba(14,165,233,0.06));
  box-shadow: inset 0 0 0 1px #6366f1;
}
.qq-emoji {
  font-size: 18px;
  line-height: 1;
  text-align: center;
}
.qq-text { min-width: 0; }
.qq-name {
  font-size: 12.5px;
  font-weight: 700;
  color: #0f172a;
  line-height: 1.2;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.qq-desc {
  font-size: 10.5px;
  color: var(--text-3);
  margin-top: 2px;
  line-height: 1.3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.qq-go { color: #cbd5e1; font-size: 13px; }
.qq-btn:hover .qq-go { color: #6366f1; }

.panel-col {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.ch-left { display: flex; flex-direction: column; gap: 2px; }
.proj-link {
  font-size: 11.5px;
  color: var(--text-3);
}
.proj-link b { color: var(--brand-dark); }
.consult-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.mode-switch { display: flex; }

.flow-head-bar {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 14px;
  padding: 10px 14px;
  background: linear-gradient(135deg, #fafbff, #f8fbff);
  border: 1px solid #e0e7ff;
  border-radius: 11px;
}
.fh-label { display: flex; align-items: center; gap: 10px; }
.fh-idx {
  width: 28px; height: 28px;
  border-radius: 9px;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
  display: grid;
  place-items: center;
  font-size: 13px;
  font-weight: 800;
}
.fh-text { font-size: 13px; font-weight: 700; color: #0f172a; }
.fh-desc { font-size: 11.5px; color: var(--text-3); line-height: 1.5; }
.fh-actions { display: flex; gap: 6px; flex-wrap: wrap; justify-content: flex-end; }

.mode-block {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.big-input .el-textarea__inner {
  min-height: 104px !important;
  padding: 12px 14px !important;
  font-size: 13.5px !important;
  line-height: 1.65 !important;
  border-radius: 12px !important;
}
.mode-desc {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  background: linear-gradient(135deg, #f8fafc, #f1f5f9);
  border-radius: 10px;
  font-size: 12.5px;
  color: var(--text-2);
}
.action-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.action-row .el-button {
  flex: 1 1 140px;
}
.act-cta {
  font-weight: 700;
  background: linear-gradient(135deg, #6366f1, #0ea5e9) !important;
  border: none !important;
}
.graph-data-area { display: flex; flex-direction: column; gap: 8px; }
.debate-config { display: flex; gap: 12px; align-items: center; flex-wrap: wrap; }

.selected-area {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  align-items: center;
}
.chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: var(--brand-soft);
  color: var(--brand-dark);
  padding: 3px 8px;
  border-radius: 7px;
  font-size: 12px;
  font-weight: 600;
}
.chip .score { font-size: 10px; opacity: 0.7; }
.chip-x { cursor: pointer; font-size: 11px; }
.chip-x:hover { color: var(--danger, #ef4444); }
.muted { color: var(--text-3); font-size: 12.5px; }

.routing-info {
  background: linear-gradient(135deg, rgba(99,102,241,0.06), rgba(14,165,233,0.05));
  border-radius: 10px;
  padding: 10px 12px;
  border: 1px solid #c7d2fe;
}
.routing-title { font-weight: 700; font-size: 12.5px; margin-bottom: 6px; color: #312e81; }
.routing-detail { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
.routing-experts { display: flex; flex-wrap: wrap; gap: 5px; width: 100%; margin-top: 4px; }

.results-block {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  border-top: 1px solid var(--border);
  padding-top: 10px;
  gap: 8px;
}
.rb-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
.rb-tools { display: flex; gap: 4px; }
.results-scroll { flex: 1; min-height: 240px; }
.results-title { font-size: 13.5px; font-weight: 800; margin: 0; }

.result-item {
  background: linear-gradient(180deg, #fafbff, #fff);
  border: 1px solid #e2e8f0;
  border-radius: 11px;
  padding: 12px;
  margin-bottom: 8px;
}
.result-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  gap: 8px;
  flex-wrap: wrap;
}
.result-meta { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.expert-badge {
  color: #fff;
  padding: 3px 10px;
  border-radius: 7px;
  font-size: 12px;
  font-weight: 700;
}
.confidence { font-size: 11.5px; color: var(--text-3); }
.duration { font-size: 11.5px; color: var(--text-3); }
.result-content {
  font-size: 13px;
  line-height: 1.75;
  color: #1e293b;
  white-space: pre-wrap;
  word-break: break-word;
}
.result-ops {
  margin-top: 8px;
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.algorithm-result { display: flex; flex-direction: column; gap: 10px; margin-top: 6px; }
.algo-section {
  background: #f8fafc;
  border-radius: 10px;
  padding: 12px;
}
.algo-section h5 { font-size: 12.5px; font-weight: 700; margin: 0 0 8px; }
.graph-stats { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 8px; }
.stat-chip {
  background: var(--brand-soft);
  color: var(--brand-dark);
  padding: 3px 9px;
  border-radius: 7px;
  font-size: 11px;
  font-weight: 600;
}
.top-nodes-title { font-size: 11px; color: var(--text-3); margin-bottom: 6px; }
.node-list { display: flex; gap: 5px; flex-wrap: wrap; }
.node-chip {
  background: #eef2ff;
  color: #4338ca;
  padding: 2px 8px;
  border-radius: 6px;
  font-size: 11px;
  font-family: ui-monospace, Menlo, Consolas, monospace;
}
.algo-item {
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  padding: 8px 10px;
  margin-bottom: 6px;
}
.algo-name { font-weight: 700; font-size: 12.5px; }
.algo-rec { font-size: 12px; color: var(--text-2); margin: 3px 0; }
.algo-complexity { font-size: 11px; color: var(--text-3); }

.ai-insight {
  background: linear-gradient(135deg, #fef3c7, #fde68a);
  border-radius: 10px;
  padding: 12px;
}
.ai-insight h5 {
  font-size: 12.5px;
  font-weight: 700;
  margin: 0 0 6px;
  display: flex; align-items: center; gap: 6px;
  color: #92400e;
}
.insight-content { font-size: 12.5px; line-height: 1.8; white-space: pre-wrap; color: #78350f; }

.debate-summary {
  margin-top: 6px;
  padding: 12px;
  background: linear-gradient(135deg, #f8fafc, #eef2ff);
  border-radius: 11px;
  border: 1px solid #e0e7ff;
}
.debate-final { font-size: 13px; line-height: 1.8; white-space: pre-wrap; color: #1e1b4b; }

.conv-empty {
  flex: 1;
  min-height: 180px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  gap: 10px;
  padding: 16px 8px;
  background:
    radial-gradient(520px 260px at 50% 0%, rgba(99,102,241,0.06), transparent),
    radial-gradient(520px 260px at 50% 100%, rgba(16,185,129,0.06), transparent);
  border-radius: 14px;
  border: 1px dashed #e2e8f0;
}
.empty-orb {
  width: 64px; height: 64px;
  display: grid;
  place-items: center;
  animation: orbSpin 8s linear infinite;
}
.eo-svg { width: 64px; height: 64px; }
@keyframes orbSpin { to { transform: rotate(360deg); } }
.empty-title { font-size: 14px; color: #1e293b; font-weight: 600; }
.empty-title b { color: var(--brand-dark); }
.empty-sub {
  font-size: 12px;
  color: var(--text-3);
  max-width: 460px;
  line-height: 1.7;
}

.right-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
.rh-left { display: flex; align-items: center; gap: 10px; }
.rh-logo {
  width: 42px; height: 42px;
  border-radius: 12px;
  background: linear-gradient(135deg, rgba(16,185,129,0.10), rgba(99,102,241,0.08));
  display: grid;
  place-items: center;
}
.rh-svg { width: 30px; height: 30px; }
.rh-text { min-width: 0; }
.rh-title { font-size: 13.5px; font-weight: 800; color: #0f172a; }
.rh-sub { font-size: 11.5px; color: var(--text-3); margin-top: 2px; }
.rh-right { display: flex; gap: 6px; flex-shrink: 0; }

.mox-canvas-card { gap: 10px; padding-bottom: 10px; }
.mc-legend {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.lg {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 999px;
  background: #f1f5f9;
  color: #334155;
  font-weight: 600;
}
.lg::before {
  content: '';
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 6px;
  vertical-align: -1px;
}
.lg-project::before  { background: #6366f1; }
.lg-goal::before    { background: #0ea5e9; }
.lg-actor::before   { background: #f59e0b; }
.lg-usecase::before { background: #10b981; }
.lg-data::before    { background: #ef4444; }
.lg-tech::before    { background: #8b5cf6; }
.lg-end::before     { background: #64748b; }

.mc-stage {
  position: relative;
  width: 100%;
  height: 320px;
  background:
    radial-gradient(520px 200px at 50% -20%, rgba(99,102,241,0.06), transparent),
    linear-gradient(135deg, #fafbfc, #f8fafc);
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  overflow: hidden;
}
.mc-stage canvas { display: block; }
.mc-stats { display: flex; gap: 6px; flex-wrap: wrap; }

.progress-card { gap: 12px; }
.progress-top {
  display: grid;
  grid-template-columns: 130px 1fr;
  align-items: center;
  gap: 14px;
}
.pt-info { min-width: 0; }
.pt-name {
  font-size: 13.5px;
  font-weight: 800;
  color: #0f172a;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pt-cat { font-size: 11.5px; color: var(--text-3); margin-top: 2px; }
.pt-rows { margin-top: 10px; display: flex; flex-direction: column; gap: 4px; }
.pt-row {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
}
.pt-k { color: var(--text-3); }
.pt-v { color: #0f172a; font-weight: 700; }

.pgs { display: flex; flex-direction: column; gap: 8px; }
.pg-row {
  display: grid;
  grid-template-columns: 120px 1fr;
  align-items: center;
  gap: 10px;
}
.pg-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11.5px;
  font-weight: 600;
  color: #334155;
}
.pg-dot { width: 8px; height: 8px; border-radius: 50%; }

.ov-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
  margin-top: 4px;
}
.ov-cell {
  background: #f8fafc;
  border-radius: 10px;
  padding: 10px 8px;
  text-align: center;
  border: 1px solid #e2e8f0;
}
.ov-cell.active {
  background: linear-gradient(135deg, #dbeafe, #bfdbfe);
  border-color: #3b82f6;
}
.ov-cell.success {
  background: linear-gradient(135deg, #dcfce7, #bbf7d0);
  border-color: #22c55e;
}
.ov-v {
  font-size: 18px;
  font-weight: 800;
  color: #1e293b;
}
.ov-k {
  font-size: 10.5px;
  color: var(--text-3);
  margin-top: 2px;
}

.metrics-card { gap: 8px; }
.mini-table { font-size: 12px; }
.mini-table :deep(.el-table__cell) {
  padding: 6px 8px !important;
}

.overview-grid, .overview-card, .span1, .span2, .card-pad, .main-grid, .panel-head,
.expert-status, .consult-input, .metrics-panel { display: none; }
</style>
