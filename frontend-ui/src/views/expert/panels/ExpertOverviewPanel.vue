<!--
  专家联盟 · 总览页
  φ 黄金比例三栏布局：左 382px : 中自适应 : 右 420px
  左栏：阶段导航 + 专家库
  中栏：AI 咨询工作台（5 种模式）
  右栏：知识图谱 + 项目进度 + 专家绩效
-->
<template>
  <div class="expert-overview">
    <div class="phi-layout">
      <!-- ============ 左栏：阶段导航 + 专家库 ============ -->
      <aside class="col col-left">
        <!-- 阶段导航 -->
        <section class="card phase-nav-card">
          <div class="card-header">
            <span class="card-title">项目阶段</span>
            <span class="card-sub">4 阶段全维流程</span>
          </div>
          <div class="phase-list">
            <div
              v-for="(p, idx) in phases"
              :key="p.key"
              class="phase-item"
              :class="{ active: currentPhase === p.key }"
              @click="currentPhase = p.key"
            >
              <div class="phase-index" :style="{ background: p.color }">{{ idx + 1 }}</div>
              <div class="phase-info">
                <div class="phase-name">{{ p.label }}</div>
                <div class="phase-desc">{{ p.desc }}</div>
              </div>
            </div>
          </div>
        </section>

        <!-- 专家库 -->
        <section class="card experts-card" v-loading="expertsLoading">
          <div class="card-header between">
            <div>
              <span class="card-title">专家联盟</span>
              <span class="count-badge">{{ filteredExperts.length }} 位</span>
            </div>
            <div style="display:flex;align-items:center;gap:8px">
              <el-button text size="small" @click="loadExperts" title="刷新">
                <el-icon><Refresh /></el-icon>
              </el-button>
              <el-switch
                v-model="smartMatch"
                size="small"
                active-text="智能"
                inactive-text="手动"
              />
            </div>
          </div>

          <div class="filter-row">
            <el-select v-model="filterType" placeholder="类型" clearable size="small" style="width: 100px">
              <el-option v-for="t in expertTypes" :key="t" :label="typeLabel(t)" :value="t" />
            </el-select>
            <el-input v-model="keyword" placeholder="搜索专家…" clearable size="small" style="flex: 1">
              <template #prefix><el-icon><Search /></el-icon></template>
            </el-input>
          </div>

          <el-scrollbar class="expert-scroll">
            <template v-if="expertsError">
              <div class="error-state">
                <el-empty :description="expertsError" :image-size="50">
                  <el-button size="small" type="primary" @click="loadExperts">重试</el-button>
                </el-empty>
              </div>
            </template>
            <template v-else>
              <div
                v-for="exp in filteredExperts"
                :key="exp.id"
                class="expert-item"
                :class="{ selected: isSelected(exp.id) }"
                @click="toggleExpert(exp)"
              >
                <div class="expert-avatar" :style="{ background: expertColor(exp.type) }">
                  {{ expertEmoji(exp.type) }}
                </div>
                <div class="expert-info">
                  <div class="expert-name">{{ exp.name }}</div>
                  <div class="expert-type">{{ typeLabel(exp.type) }}</div>
                  <div class="expert-tags">
                    <span v-for="cap in (exp.capabilities || []).slice(0,2)" :key="cap" class="cap-tag">{{ cap }}</span>
                  </div>
                </div>
                <div v-if="isSelected(exp.id)" class="expert-check">
                  <el-icon><CircleCheckFilled /></el-icon>
                </div>
              </div>
              <el-empty v-if="!expertsLoading && filteredExperts.length === 0" description="暂无匹配专家" :image-size="40" />
            </template>
          </el-scrollbar>

          <div v-if="selectedExperts.length > 0" class="selected-summary">
            <span>已选 {{ selectedExperts.length }} 位专家</span>
            <el-button text size="small" type="danger" @click="clearSelection">清空</el-button>
          </div>
        </section>
      </aside>

      <!-- ============ 中栏：AI 咨询工作台 ============ -->
      <main class="col col-mid">
        <section class="card chat-card">
          <!-- 咨询模式切换 -->
          <div class="mode-switcher">
            <div class="mode-tabs">
              <div
                v-for="(mode, key) in consultModes"
                :key="key"
                class="mode-tab"
                :class="{ active: aiStore.consultMode === key }"
                @click="aiStore.setConsultMode(key)"
                :title="mode.desc"
              >
                {{ mode.label }}
              </div>
            </div>
            <div class="mode-hint">
              {{ aiStore.currentConsultMode.desc }}
            </div>
          </div>

          <!-- AI 对话面板 -->
          <div class="chat-container">
            <AIChatPanel
              mode="compact"
              :compact-header="false"
              :placeholder="currentPlaceholder"
              :show-hint="true"
              :max-input-rows="4"
            />
          </div>
        </section>
      </main>

      <!-- ============ 右栏：图谱 + 进度 + 绩效 ============ -->
      <aside class="col col-right">
        <!-- 知识图谱 -->
        <section class="card graph-card">
          <div class="card-header between">
            <span class="card-title">知识图谱</span>
            <el-button text size="small" @click="goToGraph">
              查看大图 <el-icon><ArrowRight /></el-icon>
            </el-button>
          </div>
          <div class="graph-visual" ref="graphRef">
            <canvas ref="canvasRef"></canvas>
            <div class="graph-stats">
              <div class="stat">
                <span class="stat-num">{{ graphData.nodes.length }}</span>
                <span class="stat-label">节点</span>
              </div>
              <div class="stat">
                <span class="stat-num">{{ graphData.edges.length }}</span>
                <span class="stat-label">关系</span>
              </div>
              <div class="stat">
                <span class="stat-num">7</span>
                <span class="stat-label">类型</span>
              </div>
            </div>
          </div>
        </section>

        <!-- 项目进度 -->
        <section class="card progress-card">
          <div class="card-header">
            <span class="card-title">项目推进</span>
            <span class="card-sub">联盟绩效</span>
          </div>
          <div class="progress-ring-wrap">
            <div class="progress-ring" :style="{ '--p': projectProgress + '%' }">
              <svg viewBox="0 0 120 120">
                <circle class="ring-bg" cx="60" cy="60" r="52" />
                <circle
                  class="ring-fg"
                  cx="60" cy="60" r="52"
                  :style="{ strokeDashoffset: 326.7 - (326.7 * projectProgress / 100) }"
                />
              </svg>
              <div class="ring-center">
                <div class="ring-num">{{ projectProgress }}%</div>
                <div class="ring-label">整体进度</div>
              </div>
            </div>
          </div>
          <div class="phase-progress-list">
            <div v-for="p in phases" :key="p.key" class="phase-progress-row">
              <div class="pp-name">{{ p.label }}</div>
              <div class="pp-bar">
                <div class="pp-fill" :style="{ width: phaseProgress[p.key] + '%', background: p.color }"></div>
              </div>
              <div class="pp-val">{{ phaseProgress[p.key] }}%</div>
            </div>
          </div>
        </section>

        <!-- 专家绩效 Top -->
        <section class="card ranking-card">
          <div class="card-header">
            <span class="card-title">专家绩效榜</span>
            <span class="card-sub">Top 5</span>
          </div>
          <div class="ranking-list">
            <div
              v-for="(exp, idx) in topExperts"
              :key="exp.id"
              class="ranking-item"
            >
              <div class="rank-idx" :class="'rank-' + (idx + 1)">{{ idx + 1 }}</div>
              <div class="rank-avatar" :style="{ background: expertColor(exp.type) }">
                {{ expertEmoji(exp.type) }}
              </div>
              <div class="rank-info">
                <div class="rank-name">{{ exp.name }}</div>
                <div class="rank-type">{{ typeLabel(exp.type) }}</div>
              </div>
              <div class="rank-score">
                <div class="score-val">{{ Math.round((exp.metrics?.success_rate || 0) * 100) }}%</div>
                <div class="score-label">成功率</div>
              </div>
            </div>
          </div>
        </section>
      </aside>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, nextTick, watch } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  Search, CircleCheckFilled, ArrowRight, Refresh
} from '@element-plus/icons-vue'
import { useAIStore, CONSULT_MODES } from '@/stores/ai.store'
import AIChatPanel from '@/components/ai/AIChatPanel.vue'
import { EXPERT_TYPES } from '@/constants/expert.constants'
import { getExperts, getExpertGraph, getExpertOverview } from '@/api/experts.api.js'

const router = useRouter()
const aiStore = useAIStore()

// ===== 阶段 =====
const phases = [
  { key: 'requirement', label: '需求阶段', desc: 'AI 对话 · 需求编译', color: '#6366f1' },
  { key: 'architecture', label: '架构阶段', desc: '知识图谱 · 专家联盟', color: '#06b6d4' },
  { key: 'develop', label: '开发阶段', desc: '算子 · 工作流 · 自动化', color: '#10b981' },
  { key: 'release', label: '发布阶段', desc: '监控 · 文档 · 管理', color: '#f59e0b' }
]

const currentPhase = ref('architecture')

// ===== 咨询模式 =====
const consultModes = CONSULT_MODES

const currentPlaceholder = computed(() => {
  const mode = aiStore.consultMode
  const placeholders = {
    smart: '描述你的问题，AI 将自动选择最优专家协作模式…',
    single: '选择左侧专家后，输入你的问题进行一对一咨询…',
    multi: '已选 ' + aiStore.selectedExpertIds.length + ' 位专家，输入问题开始协同分析…',
    debate: '输入辩题，多位专家将展开多轮交叉辩论…',
    algorithm: '描述你的算法问题，AI 将分析复杂度并推荐方案…'
  }
  return placeholders[mode] || '输入你的问题…'
})

// ===== 专家数据（API 驱动） =====
const expertTypes = Object.keys(EXPERT_TYPES)
const keyword = ref('')
const filterType = ref('')
const smartMatch = ref(true)
const expertsLoading = ref(false)
const expertsError = ref('')

const experts = ref([])

async function loadExperts() {
  expertsLoading.value = true
  expertsError.value = ''
  try {
    const data = await getExperts({ page: 1, page_size: 100 })
    // 兼容多种返回结构：数组 / { list } / { data }
    if (Array.isArray(data)) {
      experts.value = data
    } else if (Array.isArray(data?.list)) {
      experts.value = data.list
    } else if (Array.isArray(data?.data)) {
      experts.value = data.data
    } else if (Array.isArray(data?.experts)) {
      experts.value = data.experts
    } else {
      experts.value = []
    }
  } catch (e) {
    expertsError.value = e?.message || '专家列表加载失败'
    experts.value = []
    ElMessage.error('专家列表加载失败：' + (e?.message || '未知错误'))
  } finally {
    expertsLoading.value = false
  }
}

const filteredExperts = computed(() => {
  let list = experts.value
  if (filterType.value) {
    list = list.filter(e => e.type === filterType.value)
  }
  if (keyword.value) {
    const kw = keyword.value.toLowerCase()
    list = list.filter(e =>
      e.name.toLowerCase().includes(kw) ||
      e.capabilities.some(c => c.toLowerCase().includes(kw))
    )
  }
  return list
})

const selectedExperts = computed(() =>
  experts.value.filter(e => aiStore.selectedExpertIds.includes(e.id))
)

const topExperts = computed(() =>
  [...experts.value]
    .sort((a, b) => (b.metrics?.success_rate || 0) - (a.metrics?.success_rate || 0))
    .slice(0, 5)
)

function typeLabel(type) {
  return EXPERT_TYPES[type] || type
}

function expertColor(type) {
  const colors = {
    algorithm: '#6366f1', architecture: '#6366f1', data: '#10b981',
    ai: '#ec4899', workflow: '#f59e0b', graph: '#06b6d4',
    security: '#ef4444', performance: '#f97316', monitor: '#14b8a6',
    market: '#8b5cf6', mcp: '#0ea5e9', automation: '#84cc16',
    requirement: '#f43f5e', fusion: '#a855f7', operator: '#64748b',
    custom: '#64748b'
  }
  return colors[type] || '#6366f1'
}

function expertEmoji(type) {
  const emojis = {
    algorithm: '🧮', architecture: '🏗️', data: '🔗',
    ai: '🤖', workflow: '⚡', graph: '🕸️',
    security: '🔒', performance: '🚀', monitor: '📊',
    market: '📈', mcp: '🔌', automation: '🤖',
    requirement: '📋', fusion: '🔀', operator: '⚙️',
    custom: '👤'
  }
  return emojis[type] || '👤'
}

function isSelected(id) {
  return aiStore.selectedExpertIds.includes(id)
}

function toggleExpert(exp) {
  const idx = aiStore.selectedExpertIds.indexOf(exp.id)
  if (idx >= 0) {
    aiStore.selectedExpertIds.splice(idx, 1)
  } else {
    if (aiStore.consultMode === 'single') {
      aiStore.selectedExpertIds = [exp.id]
    } else {
      aiStore.selectedExpertIds.push(exp.id)
    }
    // 自动切换到对应助手
    if (exp.type === 'architecture') aiStore.setAssistant('architect')
    else if (exp.type === 'data') aiStore.setAssistant('data')
    else if (exp.type === 'ai') aiStore.setAssistant('general')
  }
}

function clearSelection() {
  aiStore.selectedExpertIds = []
}

// ===== 项目进度（API 驱动，从 overview 数据派生） =====
const phaseProgress = ref({
  requirement: 0,
  architecture: 0,
  develop: 0,
  release: 0
})

const projectProgress = computed(() => {
  const vals = Object.values(phaseProgress.value)
  if (!vals.length) return 0
  return Math.round(vals.reduce((a, b) => a + b, 0) / vals.length)
})

// ===== 知识图谱（API 驱动） =====
const canvasRef = ref(null)
const graphData = ref({ nodes: [], edges: [] })
const graphLoading = ref(false)

async function loadGraph() {
  graphLoading.value = true
  try {
    const data = await getExpertGraph()
    if (data && Array.isArray(data.nodes) && Array.isArray(data.edges)) {
      graphData.value = data
    } else if (data?.data && Array.isArray(data.data.nodes)) {
      graphData.value = data.data
    }
  } catch (e) {
    console.warn('[ExpertOverview] 图谱加载失败:', e?.message)
    // 保留空图谱，不阻断页面
  } finally {
    graphLoading.value = false
  }
}

async function loadOverview() {
  try {
    const data = await getExpertOverview()
    if (data && typeof data === 'object') {
      // 尝试从 overview 数据中提取阶段进度
      if (data.phase_progress && typeof data.phase_progress === 'object') {
        phaseProgress.value = { ...phaseProgress.value, ...data.phase_progress }
      } else if (data.phases && Array.isArray(data.phases)) {
        const map = {}
        data.phases.forEach(p => {
          if (p.key && p.progress != null) map[p.key] = p.progress
        })
        if (Object.keys(map).length) phaseProgress.value = { ...phaseProgress.value, ...map }
      }
    }
  } catch (e) {
    console.warn('[ExpertOverview] 概览数据加载失败:', e?.message)
  }
}

function drawGraph() {
  const canvas = canvasRef.value
  if (!canvas) return
  const ctx = canvas.getContext('2d')
  const rect = canvas.parentElement.getBoundingClientRect()
  canvas.width = rect.width
  canvas.height = rect.height

  const nodes = graphData.value.nodes
  const edges = graphData.value.edges
  const w = canvas.width
  const h = canvas.height

  // 简单力导向布局
  const centerX = w / 2
  const centerY = h / 2
  const scale = Math.min(w, h) / 250

  // 画边
  ctx.strokeStyle = '#3a3f5a'
  ctx.lineWidth = 1.5
  edges.forEach(e => {
    const s = nodes.find(n => n.id === e.s)
    const t = nodes.find(n => n.id === e.t)
    if (s && t) {
      ctx.beginPath()
      ctx.moveTo(centerX + (s.x - 200) * scale, centerY + (s.y - 130) * scale)
      ctx.lineTo(centerX + (t.x - 200) * scale, centerY + (t.y - 130) * scale)
      ctx.stroke()
    }
  })

  // 画节点
  const colors = {
    actor: '#6366f1', goal: '#ec4899', usecase: '#06b6d4',
    data: '#10b981', tech: '#f59e0b', end: '#8b5cf6'
  }

  nodes.forEach(n => {
    const x = centerX + (n.x - 200) * scale
    const y = centerY + (n.y - 130) * scale
    const r = n.type === 'goal' ? 22 : 18

    // 光晕
    ctx.beginPath()
    ctx.arc(x, y, r + 4, 0, Math.PI * 2)
    ctx.fillStyle = (colors[n.type] || '#6366f1') + '20'
    ctx.fill()

    // 节点
    ctx.beginPath()
    ctx.arc(x, y, r, 0, Math.PI * 2)
    ctx.fillStyle = colors[n.type] || '#6366f1'
    ctx.fill()

    // 标签
    ctx.fillStyle = '#fff'
    ctx.font = '10px sans-serif'
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(n.label, x, y)
  })
}

function goToGraph() {
  router.push('/graph')
}

// ===== 生命周期 =====
onMounted(async () => {
  aiStore.setScope('project', 'current-project')
  aiStore.ensureSession()
  // 并行加载专家列表、图谱、概览数据
  await Promise.all([loadExperts(), loadGraph(), loadOverview()])
  nextTick(() => {
    drawGraph()
    window.addEventListener('resize', drawGraph)
  })
})

watch(currentPhase, () => {
  // 切换阶段时更新智能匹配的专家推荐
})
</script>

<style scoped>
.expert-overview {
  height: 100%;
  width: 100%;
}

.phi-layout {
  display: flex;
  gap: 12px;
  height: 100%;
  padding: 12px;
  box-sizing: border-box;
}

.col {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
}

.col-left {
  width: 320px;
  flex-shrink: 0;
}

.col-mid {
  flex: 1;
  min-width: 0;
}

.col-right {
  width: 340px;
  flex-shrink: 0;
}

.card {
  background: var(--bg-card);
  border-radius: 12px;
  border: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.card-header {
  padding: 12px 14px;
  border-bottom: 1px solid var(--border-ghost);
  display: flex;
  align-items: center;
  gap: 8px;
}

.card-header.between {
  justify-content: space-between;
}

.card-title {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-primary);
}

.card-sub {
  font-size: 11px;
  color: #94a3b8;
}

.count-badge {
  background: var(--accent-dim);
  color: #6366f1;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 600;
  margin-left: 6px;
}

/* ===== 阶段导航 ===== */
.phase-nav-card {
  flex-shrink: 0;
}

.phase-list {
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.phase-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;
}

.phase-item:hover {
  background: var(--bg-tertiary);
}

.phase-item.active {
  background: var(--accent-dim);
}

.phase-index {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}

.phase-info {
  flex: 1;
  min-width: 0;
}

.phase-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.3;
}

.phase-item.active .phase-name {
  color: #4f46e5;
}

.phase-desc {
  font-size: 11px;
  color: #94a3b8;
  margin-top: 2px;
}

/* ===== 专家库 ===== */
.experts-card {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.filter-row {
  padding: 10px 12px;
  display: flex;
  gap: 8px;
  border-bottom: 1px solid var(--border-ghost);
}

.expert-scroll {
  flex: 1;
  overflow: hidden;
  min-height: 0;
}

.expert-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  cursor: pointer;
  border-bottom: 1px solid var(--border-ghost);
  transition: all 0.12s;
  position: relative;
}

.expert-item:hover {
  background: var(--bg-tertiary);
}

.expert-item.selected {
  background: var(--success-dim);
  border-left: 3px solid #10b981;
  padding-left: 9px;
}

.expert-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 18px;
  flex-shrink: 0;
}

.expert-info {
  flex: 1;
  min-width: 0;
}

.expert-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.3;
}

.expert-type {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
}

.expert-tags {
  display: flex;
  gap: 4px;
  margin-top: 4px;
  flex-wrap: wrap;
}

.cap-tag {
  font-size: 10px;
  padding: 1px 6px;
  background: var(--bg-tertiary);
  color: var(--text-secondary);
  border-radius: 4px;
}

.expert-check {
  color: #10b981;
  font-size: 16px;
}

.selected-summary {
  padding: 8px 12px;
  border-top: 1px solid var(--border-ghost);
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  color: #64748b;
  background: var(--bg-tertiary);
}

.error-state {
  padding: 20px 10px;
}

/* ===== 中栏对话区 ===== */
.chat-card {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.mode-switcher {
  padding: 10px 14px;
  border-bottom: 1px solid var(--border-ghost);
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}

.mode-tabs {
  display: flex;
  gap: 2px;
  background: var(--bg-tertiary);
  padding: 3px;
  border-radius: 8px;
}

.mode-tab {
  padding: 5px 10px;
  font-size: 12px;
  color: #64748b;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}

.mode-tab:hover {
  color: var(--text-secondary);
}

.mode-tab.active {
  background: var(--bg-card);
  color: #4f46e5;
  font-weight: 600;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
}

.mode-hint {
  font-size: 11px;
  color: #94a3b8;
}

.chat-container {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

/* ===== 右栏 ===== */
.graph-card {
  flex-shrink: 0;
}

.graph-visual {
  position: relative;
  height: 180px;
  background: linear-gradient(135deg, #f8fafc, #f1f5f9);
}

.graph-visual canvas {
  width: 100%;
  height: 100%;
  display: block;
}

.graph-stats {
  position: absolute;
  bottom: 8px;
  left: 0;
  right: 0;
  display: flex;
  justify-content: center;
  gap: 20px;
}

.stat {
  text-align: center;
}

.stat-num {
  display: block;
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
  line-height: 1.2;
}

.stat-label {
  font-size: 10px;
  color: #64748b;
}

/* 进度卡片 */
.progress-card {
  flex-shrink: 0;
}

.progress-ring-wrap {
  display: flex;
  justify-content: center;
  padding: 16px 0 8px;
}

.progress-ring {
  position: relative;
  width: 100px;
  height: 100px;
}

.progress-ring svg {
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}

.ring-bg {
  fill: none;
  stroke: #e2e8f0;
  stroke-width: 8;
}

.ring-fg {
  fill: none;
  stroke: url(#ringGrad);
  stroke: #6366f1;
  stroke-width: 8;
  stroke-linecap: round;
  stroke-dasharray: 326.7;
  transition: stroke-dashoffset 0.5s ease;
}

.ring-center {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

.ring-num {
  font-size: 22px;
  font-weight: 800;
  color: var(--text-primary);
  line-height: 1.2;
}

.ring-label {
  font-size: 10px;
  color: #64748b;
  margin-top: 2px;
}

.phase-progress-list {
  padding: 0 14px 14px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.phase-progress-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
}

.pp-name {
  width: 56px;
  color: #64748b;
  flex-shrink: 0;
}

.pp-bar {
  flex: 1;
  height: 6px;
  background: var(--bg-tertiary);
  border-radius: 3px;
  overflow: hidden;
}

.pp-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.3s ease;
}

.pp-val {
  width: 32px;
  text-align: right;
  color: var(--text-secondary);
  font-weight: 600;
  flex-shrink: 0;
}

/* 绩效榜 */
.ranking-card {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.ranking-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 10px 10px;
}

.ranking-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 6px;
  border-radius: 6px;
  transition: background 0.12s;
}

.ranking-item:hover {
  background: var(--bg-tertiary);
}

.rank-idx {
  width: 20px;
  height: 20px;
  border-radius: 4px;
  display: grid;
  place-items: center;
  font-size: 11px;
  font-weight: 700;
  background: var(--bg-tertiary);
  color: #64748b;
  flex-shrink: 0;
}

.rank-idx.rank-1 { background: #fef3c7; color: #d97706; }
.rank-idx.rank-2 { background: #e0f2fe; color: #0284c7; }
.rank-idx.rank-3 { background: #fce7f3; color: #db2777; }

.rank-avatar {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 14px;
  flex-shrink: 0;
}

.rank-info {
  flex: 1;
  min-width: 0;
}

.rank-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.3;
}

.rank-type {
  font-size: 10px;
  color: #94a3b8;
  margin-top: 1px;
}

.rank-score {
  text-align: right;
  flex-shrink: 0;
}

.score-val {
  font-size: 13px;
  font-weight: 700;
  color: #10b981;
  line-height: 1.2;
}

.score-label {
  font-size: 10px;
  color: #94a3b8;
}

/* 响应式 */
@media (max-width: 1280px) {
  .col-right {
    width: 300px;
  }
  .col-left {
    width: 280px;
  }
}

@media (max-width: 1024px) {
  .col-right {
    display: none;
  }
}
</style>
