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
        <section class="card experts-card">
          <div class="card-header between">
            <div>
              <span class="card-title">专家联盟</span>
              <span class="count-badge">{{ filteredExperts.length }} 位</span>
            </div>
            <el-switch
              v-model="smartMatch"
              size="small"
              active-text="智能"
              inactive-text="手动"
            />
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
            <el-empty v-if="filteredExperts.length === 0" description="暂无匹配专家" :image-size="40" />
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
import {
  Search, CircleCheckFilled, ArrowRight
} from '@element-plus/icons-vue'
import { useAIStore, CONSULT_MODES } from '@/stores/ai.store'
import AIChatPanel from '@/components/ai/AIChatPanel.vue'
import { EXPERT_TYPES } from '@/constants/expert.constants'

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

// ===== 专家数据 =====
const expertTypes = Object.keys(EXPERT_TYPES)
const keyword = ref('')
const filterType = ref('')
const smartMatch = ref(true)

const experts = ref([
  { id: 'exp-001', name: '林算法', type: 'algorithm', status: 'active',
    capabilities: ['动态规划', '图算法', '复杂度分析'],
    metrics: { total_consults: 1286, success_rate: 0.97, avg_duration: 1200 } },
  { id: 'exp-002', name: '陈架构', type: 'architecture', status: 'active',
    capabilities: ['微服务', 'DDD', '高可用设计'],
    metrics: { total_consults: 2103, success_rate: 0.95, avg_duration: 1800 } },
  { id: 'exp-003', name: '王数据', type: 'data', status: 'active',
    capabilities: ['数据建模', 'ETL', '数据治理'],
    metrics: { total_consults: 856, success_rate: 0.98, avg_duration: 950 } },
  { id: 'exp-004', name: '张AI', type: 'ai', status: 'active',
    capabilities: ['LLM', 'RAG', 'Prompt工程'],
    metrics: { total_consults: 3241, success_rate: 0.94, avg_duration: 2100 } },
  { id: 'exp-005', name: '李工作流', type: 'workflow', status: 'active',
    capabilities: ['流程编排', 'BPM', '自动化'],
    metrics: { total_consults: 678, success_rate: 0.96, avg_duration: 1500 } },
  { id: 'exp-006', name: '赵图谱', type: 'graph', status: 'active',
    capabilities: ['图数据库', 'Cypher', '图计算'],
    metrics: { total_consults: 945, success_rate: 0.93, avg_duration: 1600 } },
  { id: 'exp-007', name: '孙安全', type: 'security', status: 'active',
    capabilities: ['渗透测试', '安全审计', '合规'],
    metrics: { total_consults: 523, success_rate: 0.99, avg_duration: 2200 } },
  { id: 'exp-008', name: '周性能', type: 'performance', status: 'active',
    capabilities: ['性能调优', '压测', '缓存策略'],
    metrics: { total_consults: 712, success_rate: 0.92, avg_duration: 1400 } }
])

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

// ===== 项目进度 =====
const phaseProgress = ref({
  requirement: 75,
  architecture: 60,
  develop: 35,
  release: 15
})

const projectProgress = computed(() => {
  const vals = Object.values(phaseProgress.value)
  return Math.round(vals.reduce((a, b) => a + b, 0) / vals.length)
})

// ===== 知识图谱 =====
const canvasRef = ref(null)
const graphData = ref({
  nodes: [
    { id: 'n1', label: '用户', type: 'actor', x: 100, y: 80 },
    { id: 'n2', label: '项目', type: 'goal', x: 200, y: 50 },
    { id: 'n3', label: '需求', type: 'usecase', x: 300, y: 90 },
    { id: 'n4', label: '数据', type: 'data', x: 150, y: 160 },
    { id: 'n5', label: '技术', type: 'tech', x: 250, y: 180 },
    { id: 'n6', label: '架构', type: 'usecase', x: 350, y: 160 },
    { id: 'n7', label: '交付', type: 'end', x: 200, y: 230 }
  ],
  edges: [
    { s: 'n1', t: 'n2' }, { s: 'n2', t: 'n3' }, { s: 'n3', t: 'n4' },
    { s: 'n4', t: 'n5' }, { s: 'n5', t: 'n6' }, { s: 'n6', t: 'n7' },
    { s: 'n2', t: 'n5' }, { s: 'n3', t: 'n6' }
  ]
})

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
  ctx.strokeStyle = '#cbd5e1'
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
onMounted(() => {
  aiStore.setScope('project', 'current-project')
  aiStore.ensureSession()
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
  background: #fff;
  border-radius: 12px;
  border: 1px solid #e2e8f0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.card-header {
  padding: 12px 14px;
  border-bottom: 1px solid #f1f5f9;
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
  color: #0f172a;
}

.card-sub {
  font-size: 11px;
  color: #94a3b8;
}

.count-badge {
  background: #eef2ff;
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
  background: #f8fafc;
}

.phase-item.active {
  background: #eef2ff;
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
  color: #1e293b;
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
  border-bottom: 1px solid #f1f5f9;
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
  border-bottom: 1px solid #f8fafc;
  transition: all 0.12s;
  position: relative;
}

.expert-item:hover {
  background: #f8fafc;
}

.expert-item.selected {
  background: #f0fdf4;
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
  color: #1e293b;
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
  background: #f1f5f9;
  color: #475569;
  border-radius: 4px;
}

.expert-check {
  color: #10b981;
  font-size: 16px;
}

.selected-summary {
  padding: 8px 12px;
  border-top: 1px solid #f1f5f9;
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  color: #64748b;
  background: #f8fafc;
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
  border-bottom: 1px solid #f1f5f9;
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}

.mode-tabs {
  display: flex;
  gap: 2px;
  background: #f1f5f9;
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
  color: #334155;
}

.mode-tab.active {
  background: #fff;
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
  color: #0f172a;
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
  color: #0f172a;
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
  background: #e2e8f0;
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
  color: #475569;
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
  background: #f8fafc;
}

.rank-idx {
  width: 20px;
  height: 20px;
  border-radius: 4px;
  display: grid;
  place-items: center;
  font-size: 11px;
  font-weight: 700;
  background: #f1f5f9;
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
  color: #1e293b;
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
