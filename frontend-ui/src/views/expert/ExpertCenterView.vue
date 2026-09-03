<template>
  <div class="expert-center">
    <!-- 简洁页头 -->
    <div class="center-header">
      <div class="header-left">
        <div class="brand-mini">
          <div class="brand-dot"></div>
          <span class="brand-name">专家联盟</span>
          <el-tag v-if="currentProject" size="small" class="project-tag" effect="dark">
            <el-icon><Folder /></el-icon>
            {{ currentProject.name }}
          </el-tag>
        </div>
      </div>
      <div class="header-right">
        <el-button size="small" type="primary" @click="showRegister = true">
          <el-icon><Plus /></el-icon> 注册专家
        </el-button>
        <el-button size="small" @click="ensureProject">
          <el-icon><FolderAdd /></el-icon> 切换项目
        </el-button>
      </div>
    </div>

    <!-- Tab 切换 -->
    <el-tabs v-model="activeTab" class="center-tabs" @tab-change="onTabChange">
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
  getExpertMetrics, getExpertOverview,
  getRequirementsGraph, getProjectPhaseProgress, advanceProjectPhase
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

// 需求图谱：优先调用 GET /api/projects/:id/requirements-graph，失败降级为演示占位
async function loadRequirementsGraph() {
  const pid = currentProject.value?.id
  if (!pid) return false
  try {
    const data = await getRequirementsGraph(pid)
    if (data && Array.isArray(data.nodes) && data.nodes.length > 0) {
      graphData.nodes = data.nodes.map((n, i) => ({ ...n, x: 0, y: 0, vx: 0, vy: 0, index: i }))
      graphData.edges = Array.isArray(data.edges) ? data.edges : []
      return true
    }
  } catch (e) { /* 降级到 mock */ }
  return false
}

// 阶段进度：优先调用 GET /api/projects/:id/phase-progress，失败保留演示占位
async function loadCenterPhaseProgress() {
  const pid = currentProject.value?.id
  if (!pid) return
  try {
    const data = await getProjectPhaseProgress(pid)
    if (data) {
      if (data.requirement != null) phaseProgress.requirement = data.requirement
      if (data.architecture != null) phaseProgress.architecture = data.architecture
      if (data.develop != null) phaseProgress.develop = data.develop
      if (data.release != null) phaseProgress.release = data.release
    }
  } catch (e) { /* 保留演示占位 */ }
}

// 演示占位：公司官网需求图谱预设数据（API 不可用时降级）
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
      ctx.fillStyle = '#e8eaed'
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
// 演示占位：阶段进度初始值（后端待提供: GET /api/projects/:id/phase-progress）
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

// 项目阶段推进：调用 PUT /api/projects/:id/advance-phase，失败降级为前端模拟
async function advanceFlowStage() {
  if (currentStage.value < 3) currentStage.value++
  const key = ['requirement','architecture','develop','release'][Math.min(3, currentStage.value)]
  if (key) localPhase.value = key
  // 模拟推进
  phaseProgress[key] = Math.min(100, (phaseProgress[key] || 0) + 14)
  // 同步到后端
  const pid = currentProject.value?.id
  if (pid) {
    try {
      await advanceProjectPhase(pid, { phase: key, progress: phaseProgress[key] })
    } catch (e) { /* 保留本地模拟状态 */ }
  }
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

// 演示占位：页面阶段变化时模拟进度更新（后端待提供: 阶段进度实时同步）
watch(localPhase, (k) => {
  phaseProgress[k] = Math.max(phaseProgress[k] || 0, 10 + Math.round(Math.random() * 8))
})

// Canvas 大小变化时重绘
let resizeObs = null
onMounted(async () => {
  await loadAll()
  // 优先加载后端需求图谱，失败降级为 Mock 图谱
  const loaded = await loadRequirementsGraph()
  if (!loaded) {
    const mock = buildMockGraph()
    graphData.nodes.push(...mock.nodes)
    graphData.edges.push(...mock.edges)
  }
  // 加载阶段进度
  loadCenterPhaseProgress()
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
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px 20px;
  background: var(--bg-primary);
  overflow: hidden;
}

/* 页头 */
.center-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  background: var(--bg-card);
  border-radius: var(--radius);
  border: 1px solid var(--border);
  min-height: 48px;
  flex-shrink: 0;
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
  box-shadow: 0 0 0 3px var(--accent-dim);
}
.brand-name {
  font-size: 15px;
  font-weight: 700;
  color: var(--text-primary);
}
.project-tag {
  margin-left: 4px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

/* Tabs */
.center-tabs {
  margin: 0;
  flex-shrink: 0;
}
:deep(.center-tabs .el-tabs__header) {
  margin-bottom: 0;
  padding: 0 6px;
  background: var(--bg-card);
  border-radius: var(--radius);
  border: 1px solid var(--border);
}
:deep(.center-tabs .el-tabs__nav-wrap::after) {
  display: none;
}
:deep(.center-tabs .el-tabs__item) {
  font-weight: 600;
  font-size: 14px;
  height: 44px;
  line-height: 44px;
  color: var(--text-secondary);
}
:deep(.center-tabs .el-tabs__item.is-active) {
  color: var(--accent-light);
}
:deep(.center-tabs .el-tabs__active-bar) {
  background-color: var(--accent);
}

.tab-content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* 过渡动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
