<template>
  <div class="expert-center">
    <div class="head">
      <div>
        <h2 class="page-title">专家联盟中心</h2>
        <p class="page-subtitle">管理企业级专家团队，实现智能路由、多专家协同与算法分析</p>
      </div>
      <div class="head-actions">
        <el-button @click="loadOverview" :loading="overviewLoading">
          <el-icon><DataAnalysis /></el-icon> 系统概览
        </el-button>
        <el-button type="primary" @click="showRegister = true">
          <el-icon><Plus /></el-icon> 注册专家
        </el-button>
        <el-button @click="loadAll"><el-icon><Refresh /></el-icon> 刷新</el-button>
      </div>
    </div>

    <!-- 系统概览卡片 -->
    <div v-if="overview" class="overview-grid">
      <div class="overview-card">
        <div class="overview-value">{{ overview.total_experts }}</div>
        <div class="overview-label">专家总数</div>
      </div>
      <div class="overview-card active">
        <div class="overview-value">{{ overview.active_experts }}</div>
        <div class="overview-label">在线专家</div>
      </div>
      <div class="overview-card">
        <div class="overview-value">{{ overview.expert_types?.length || 0 }}</div>
        <div class="overview-label">专家类型</div>
      </div>
      <div class="overview-card">
        <div class="overview-value">{{ overview.total_consults }}</div>
        <div class="overview-label">累计咨询</div>
      </div>
      <div class="overview-card success">
        <div class="overview-value">{{ (overview.avg_success_rate * 100).toFixed(1) }}%</div>
        <div class="overview-label">成功率</div>
      </div>
      <div class="overview-card">
        <div class="overview-value">{{ overview.capabilities_count }}</div>
        <div class="overview-label">能力标签</div>
      </div>
    </div>

    <div class="grid grid-3 main-grid">
      <!-- 专家列表 -->
      <div class="panel card-pad span1">
        <div class="panel-head">
          <h3 class="section-title">专家库（{{ experts.length }}）</h3>
          <el-switch v-model="smartMode" active-text="智能模式" inactive-text="" size="small" />
        </div>
        <div class="filter-bar">
          <el-select v-model="filterType" placeholder="专家类型" clearable size="small">
            <el-option v-for="t in expertTypes" :key="t" :label="typeLabel(t)" :value="t" />
          </el-select>
          <el-input v-model="keyword" placeholder="搜索专家" clearable size="small" style="flex: 1" />
        </div>
        <el-scrollbar height="480px">
          <div
            v-for="exp in filteredExperts"
            :key="exp.id"
            class="expert-card"
            :class="{ sel: isSelected(exp.id), offline: exp.status !== 'active' }"
            @click="toggleSelect(exp)"
          >
            <div class="expert-avatar" :style="{ background: getColor(exp.type) }">
              <el-icon><component :is="getIcon(exp.type)" /></el-icon>
            </div>
            <div class="expert-info">
              <div class="expert-name">
                {{ exp.name }}
                <span v-if="exp.metrics?.total_consults" class="consult-count" :title="`咨询次数`">
                  {{ exp.metrics.total_consults }}
                </span>
              </div>
              <div class="expert-type">{{ typeLabel(exp.type) }}</div>
              <div class="expert-stats" v-if="exp.metrics">
                <span class="stat-item">
                  <el-icon><TrendCharts /></el-icon>
                  {{ (exp.metrics.success_rate * 100).toFixed(0) }}%
                </span>
                <span class="stat-item">
                  <el-icon><Timer /></el-icon>
                  {{ Math.round(exp.metrics.avg_duration || 0) }}ms
                </span>
              </div>
              <div class="expert-caps">
                <el-tag v-for="cap in exp.capabilities.slice(0, 3)" :key="cap" size="small" type="info" effect="plain">
                  {{ cap }}
                </el-tag>
              </div>
            </div>
            <div class="expert-status" :class="exp.status">
              {{ exp.status === 'active' ? '在线' : '离线' }}
            </div>
          </div>
          <el-empty v-if="!filteredExperts.length" description="暂无专家" :image-size="60" />
        </el-scrollbar>
      </div>

      <!-- 咨询工作台 -->
      <div class="panel card-pad span2">
        <div class="consult-header">
          <h3 class="section-title">专家咨询工作台</h3>
          <div class="mode-switch">
            <el-radio-group v-model="mode" size="small">
              <el-radio-button value="smart">智能路由</el-radio-button>
              <el-radio-button value="single">单专家</el-radio-button>
              <el-radio-button value="multi">多专家</el-radio-button>
              <el-radio-button value="debate">辩论</el-radio-button>
              <el-radio-button value="algorithm">算法分析</el-radio-button>
            </el-radio-group>
          </div>
        </div>

        <!-- 智能路由模式 -->
        <div v-if="mode === 'smart'" class="smart-mode">
          <div class="mode-desc">
            <el-icon><MagicStick /></el-icon>
            <span>系统将自动分析问题意图，智能匹配合适的专家，并选择最优协作模式</span>
          </div>
          <el-input
            v-model="question"
            type="textarea"
            :rows="3"
            placeholder="请输入你的问题，系统将自动路由到最合适的专家..."
          />
          <div v-if="routingResult" class="routing-info">
            <div class="routing-title">智能路由结果：</div>
            <div class="routing-detail">
              <el-tag :type="routingResult.intent?.primary ? 'primary' : 'info'" effect="dark">
                主要意图：{{ routingResult.intent?.primary || '通用' }}
              </el-tag>
              <span class="muted">置信度: {{ (routingResult.intent?.confidence * 100 || 0).toFixed(0) }}%</span>
              <div class="routing-experts">
                <span
                  v-for="s in routingResult.selected?.slice(0, 3)"
                  :key="s.expert.id"
                  class="chip"
                  :title="`匹配分: ${s.score.toFixed(1)}`"
                >
                  {{ s.expert.name }}
                  <span class="score">({{ s.score.toFixed(1) }})</span>
                </span>
              </div>
            </div>
          </div>
          <div class="action-row">
            <el-button
              :loading="consulting"
              :disabled="!question.trim()"
              @click="doSmartRoute"
              type="primary"
            >
              <el-icon><Promotion /></el-icon> 智能路由咨询
            </el-button>
            <el-button
              :loading="routingLoading"
              :disabled="!question.trim()"
              @click="doRouteOnly"
            >
              <el-icon><Guide /></el-icon> 仅路由分析
            </el-button>
          </div>
        </div>

        <!-- 单专家模式 -->
        <div v-else-if="mode === 'single'" class="single-consult">
          <div class="selected-area">
            <span class="muted">已选择：</span>
            <template v-if="selectedCount()">
              <span class="chip" v-for="id in selectedExpertIds" :key="id">
                {{ getExpertName(id) }}
                <el-icon class="chip-x" @click="removeExpert(id)"><Close /></el-icon>
              </span>
            </template>
            <span v-else class="muted">请从左侧选择一位专家</span>
          </div>
          <div class="consult-input">
            <el-input
              v-model="question"
              type="textarea"
              :rows="3"
              placeholder="请输入你的问题..."
            />
            <el-button
              type="primary"
              :loading="consulting"
              :disabled="!selectedCount() || !question.trim()"
              @click="doConsult"
              style="width: 100%; margin-top: 10px"
            >
              <el-icon><Promotion /></el-icon> 开始咨询
            </el-button>
          </div>
        </div>

        <!-- 多专家模式 -->
        <div v-else-if="mode === 'multi'" class="multi-consult">
          <div class="selected-area">
            <span class="muted">已选择 {{ selectedCount() }} 位专家：</span>
            <span class="chip" v-for="id in selectedExpertIds" :key="id">
              {{ getExpertName(id) }}
            </span>
          </div>
          <el-input
            v-model="question"
            type="textarea"
            :rows="3"
            placeholder="请输入需要多位专家协同分析的问题..."
          />
          <el-button
            type="primary"
            :loading="consulting"
            :disabled="selectedCount() < 2 || !question.trim()"
            @click="doMultiConsult"
            style="width: 100%; margin-top: 10px"
          >
            <el-icon><ChatDotRound /></el-icon> 协同分析
          </el-button>
        </div>

        <!-- 辩论模式 -->
        <div v-else-if="mode === 'debate'" class="debate-mode">
          <div class="selected-area">
            <span class="muted">已选择 {{ selectedCount() }} 位专家参与辩论：</span>
            <span class="chip" v-for="id in selectedExpertIds" :key="id">
              {{ getExpertName(id) }}
            </span>
          </div>
          <div class="debate-config">
            <el-input-number v-model="rounds" :min="2" :max="5" label="辩论轮数" />
            <el-select v-model="debateStrategy" label="辩论策略" style="width: 140px">
              <el-option value="round_robin" label="轮流发言" />
              <el-option value="cross_examine" label="交叉质询" />
            </el-select>
          </div>
          <el-input
            v-model="question"
            type="textarea"
            :rows="3"
            placeholder="请输入辩论主题..."
          />
          <el-button
            type="primary"
            :loading="consulting"
            :disabled="selectedCount() < 2 || !question.trim()"
            @click="doDebate"
            style="width: 100%; margin-top: 10px"
          >
            <el-icon><MagicStick /></el-icon> 开始辩论
          </el-button>
        </div>

        <!-- 算法分析模式 -->
        <div v-else-if="mode === 'algorithm'" class="algorithm-mode">
          <div class="mode-desc">
            <el-icon><Cpu /></el-icon>
            <span>算法联盟将自动调度算法专家和图谱专家，进行深度算法分析</span>
          </div>
          <el-input
            v-model="question"
            type="textarea"
            :rows="3"
            placeholder="请输入需要算法分析的问题（如：图的最短路径、算法复杂度、排序优化等）..."
          />
          <div class="graph-data-area">
            <el-checkbox v-model="useGraphData">使用图谱数据进行分析</el-checkbox>
            <el-input
              v-if="useGraphData"
              v-model="graphDataJson"
              type="textarea"
              :rows="4"
              placeholder='图谱数据 JSON 格式：{"nodes":[{"id":"n1"}],"edges":[{"source":"n1","target":"n2"}]}'
            />
          </div>
          <el-button
            type="primary"
            :loading="consulting"
            :disabled="!question.trim()"
            @click="doAlgorithmAnalysis"
            style="width: 100%; margin-top: 10px"
          >
            <el-icon><DataAnalysis /></el-icon> 算法分析
          </el-button>
        </div>

        <!-- 路由结果展示 -->
        <div v-if="results.length" class="results">
          <h4 class="results-title">咨询结果</h4>
          <el-scrollbar height="320px">
            <div v-for="(r, i) in results" :key="i" class="result-item">
              <div class="result-head">
                <span class="expert-badge" :style="{ background: getColorByType(r.expert?.type) }">
                  {{ r.expert?.name || '专家' }}
                </span>
                <div class="result-meta">
                  <span v-if="r.confidence" class="confidence">置信度: {{ (r.confidence * 100).toFixed(0) }}%</span>
                  <span v-if="r.duration_ms" class="duration">{{ r.duration_ms }}ms</span>
                  <el-tag v-if="r.round" size="small" type="warning">第{{ r.round }}轮</el-tag>
                </div>
              </div>
              <div class="result-content">{{ r.response }}</div>
            </div>
          </el-scrollbar>
        </div>

        <!-- 算法分析结果 -->
        <div v-if="algorithmResult" class="algorithm-result">
          <h4 class="results-title">算法分析结果</h4>
          <div class="algo-section" v-if="algorithmResult.analysis?.graph">
            <h5>图谱分析</h5>
            <div class="graph-stats">
              <span class="stat-chip">节点: {{ algorithmResult.analysis.graph.stats?.nodeCount }}</span>
              <span class="stat-chip">边: {{ algorithmResult.analysis.graph.stats?.edgeCount }}</span>
              <span class="stat-chip">密度: {{ algorithmResult.analysis.graph.stats?.density }}</span>
              <span class="stat-chip">平均度: {{ algorithmResult.analysis.graph.stats?.avgDegree }}</span>
            </div>
            <div v-if="algorithmResult.analysis.graph.topNodes?.length" class="top-nodes">
              <div class="top-nodes-title">Top 节点（PageRank）:</div>
              <div class="node-list">
                <span
                  v-for="n in algorithmResult.analysis.graph.topNodes.slice(0, 5)"
                  :key="n.id"
                  class="node-chip"
                >
                  #{{ n.rank }} {{ n.id }} ({{ n.pagerank }})
                </span>
              </div>
            </div>
          </div>
          <div class="algo-section" v-if="algorithmResult.analysis?.algorithm">
            <h5>算法建议</h5>
            <div v-for="(a, i) in algorithmResult.analysis.algorithm.analyses" :key="i" class="algo-item">
              <div class="algo-name">{{ a.algorithm }}</div>
              <div class="algo-rec">{{ a.recommendation }}</div>
              <div class="algo-complexity">
                时间: {{ a.complexity.time }} | 空间: {{ a.complexity.space }}
              </div>
            </div>
          </div>
          <div v-if="algorithmResult.analysis?.ai_insight" class="ai-insight">
            <h5><el-icon><MagicStick /></el-icon> AI 深度洞察</h5>
            <div class="insight-content">{{ algorithmResult.analysis.ai_insight }}</div>
          </div>
        </div>

        <!-- 辩论总结 -->
        <div v-if="debateSummary" class="debate-summary">
          <h4 class="results-title">辩论综合结论</h4>
          <div class="debate-final">{{ debateSummary }}</div>
        </div>
      </div>
    </div>

    <!-- 专家指标面板 -->
    <div v-if="metricsList.length" class="metrics-panel">
      <h3 class="section-title">专家绩效指标</h3>
      <el-table :data="metricsList" stripe size="small" style="width: 100%">
        <el-table-column prop="expert.name" label="专家" width="140">
          <template #default="{ row }">
            <span :style="{ color: getColorByType(row.expert.type), fontWeight: 600 }">
              {{ row.expert.name }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="expert.type" label="类型" width="120">
          <template #default="{ row }">
            {{ typeLabel(row.expert.type) }}
          </template>
        </el-table-column>
        <el-table-column prop="metrics.total_consults" label="咨询次数" width="100" sortable />
        <el-table-column label="成功率" width="120">
          <template #default="{ row }">
            <el-progress
              :percentage="Math.round((row.metrics?.success_rate || 0) * 100)"
              :stroke-width="8"
              :color="getSuccessColor(row.metrics?.success_rate)"
            />
          </template>
        </el-table-column>
        <el-table-column label="平均置信度" width="120">
          <template #default="{ row }">
            <el-progress
              :percentage="Math.round((row.metrics?.avg_confidence || 0) * 100)"
              :stroke-width="8"
              color="#6366f1"
            />
          </template>
        </el-table-column>
        <el-table-column prop="metrics.avg_duration" label="平均耗时(ms)" width="130" sortable />
      </el-table>
    </div>

    <!-- 注册弹窗 -->
    <el-dialog v-model="showRegister" title="注册新专家" width="520px">
      <el-form label-width="90px">
        <el-form-item label="专家名称">
          <el-input v-model="newExpert.name" placeholder="如：数据库专家" />
        </el-form-item>
        <el-form-item label="专家类型">
          <el-select v-model="newExpert.type" style="width: 100%">
            <el-option v-for="t in expertTypes" :key="t" :label="typeLabel(t)" :value="t" />
          </el-select>
        </el-form-item>
        <el-form-item label="能力标签">
          <el-input v-model="newExpert.capabilities_str" placeholder="用逗号分隔，如：性能优化,索引调优" />
        </el-form-item>
        <el-form-item label="专家描述">
          <el-input v-model="newExpert.description" type="textarea" :rows="2" />
        </el-form-item>
        <el-form-item label="系统提示词">
          <el-input
            v-model="newExpert.systemPrompt"
            type="textarea"
            :rows="3"
            placeholder="专家专属的 System Prompt，用于定义专家角色和行为"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showRegister = false">取消</el-button>
        <el-button type="primary" :loading="registering" @click="doRegister">注册</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, watch } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Plus, Refresh, Close, Promotion, ChatDotRound, MagicStick,
  DataAnalysis, Timer, TrendCharts, Cpu, Guide
} from '@element-plus/icons-vue'
import {
  getExperts, registerExpert, consultExpert, multiExpertConsult, expertDebate,
  routeExperts, intelligentConsult, algorithmAnalysis,
  getExpertMetrics, getExpertOverview
} from '@/api'

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

onMounted(() => {
  loadAll()
})
</script>

<style scoped>
.expert-center {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.head-actions {
  display: flex;
  gap: 10px;
}

.overview-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 12px;
}
.overview-card {
  background: var(--bg-page);
  border-radius: 12px;
  padding: 16px;
  text-align: center;
  border: 1px solid var(--border);
  transition: transform 0.2s;
}
.overview-card:hover { transform: translateY(-2px); }
.overview-card.active { background: linear-gradient(135deg, #dbeafe, #bfdbfe); border-color: #3b82f6; }
.overview-card.success { background: linear-gradient(135deg, #dcfce7, #bbf7d0); border-color: #22c55e; }
.overview-value {
  font-size: 28px;
  font-weight: 800;
  color: var(--brand);
}
.overview-label {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 4px;
}

.span1 { grid-column: span 1; }
.span2 { grid-column: span 2; }
.card-pad { padding: 20px 22px; }
.main-grid { align-items: start; }

.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.filter-bar {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.expert-card {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  padding: 12px;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s;
  border: 2px solid transparent;
  margin-bottom: 8px;
}
.expert-card:hover { background: var(--bg-page); }
.expert-card.sel {
  background: var(--brand-soft);
  border-color: var(--brand);
}
.expert-card.offline { opacity: 0.6; }

.expert-avatar {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 20px;
  flex-shrink: 0;
}

.expert-info { flex: 1; min-width: 0; }
.expert-name { font-weight: 700; font-size: 14px; display: flex; align-items: center; gap: 6px; }
.consult-count {
  font-size: 10px;
  background: var(--brand);
  color: #fff;
  padding: 1px 6px;
  border-radius: 10px;
  font-weight: 500;
}
.expert-type { font-size: 12px; color: var(--text-3); margin: 2px 0; }
.expert-stats {
  display: flex;
  gap: 10px;
  margin: 4px 0;
  font-size: 11px;
  color: var(--text-3);
}
.stat-item { display: inline-flex; align-items: center; gap: 2px; }
.expert-caps { display: flex; gap: 4px; flex-wrap: wrap; }

.expert-status {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 999px;
  font-weight: 600;
}
.expert-status.active { background: #dcfce7; color: #16a34a; }
.expert-status.inactive { background: #f1f5f9; color: #94a3b8; }

.consult-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.selected-area {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  margin-bottom: 12px;
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
.chip-x:hover { color: var(--danger); }

.muted { color: var(--text-3); font-size: 13px; }
.consult-input { display: flex; flex-direction: column; }

.smart-mode, .algorithm-mode {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.mode-desc {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  background: linear-gradient(135deg, #f8fafc, #f1f5f9);
  border-radius: 10px;
  font-size: 13px;
  color: var(--text-2);
}

.routing-info {
  background: var(--brand-soft);
  border-radius: 10px;
  padding: 12px;
  border: 1px solid var(--brand);
}
.routing-title { font-weight: 700; font-size: 13px; margin-bottom: 8px; }
.routing-detail { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
.routing-experts { display: flex; flex-wrap: wrap; gap: 4px; width: 100%; margin-top: 6px; }

.action-row {
  display: flex;
  gap: 10px;
}
.action-row .el-button { flex: 1; }

.debate-config {
  display: flex;
  gap: 16px;
  margin: 10px 0;
}

.graph-data-area { margin: 12px 0; }

.results {
  margin-top: 20px;
  border-top: 1px solid var(--border);
  padding-top: 16px;
}
.results-title { font-size: 14px; font-weight: 700; margin-bottom: 12px; }

.result-item {
  background: var(--bg-page);
  border-radius: 10px;
  padding: 14px;
  margin-bottom: 10px;
}
.result-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.result-meta { display: flex; gap: 8px; align-items: center; }
.expert-badge {
  color: #fff;
  padding: 3px 10px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
}
.confidence { font-size: 12px; color: var(--text-3); }
.duration { font-size: 12px; color: var(--text-3); }
.result-content {
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
}

.algorithm-result {
  margin-top: 20px;
  border-top: 1px solid var(--border);
  padding-top: 16px;
}
.algo-section {
  background: var(--bg-page);
  border-radius: 10px;
  padding: 14px;
  margin-bottom: 12px;
}
.algo-section h5 { font-size: 13px; font-weight: 700; margin-bottom: 10px; }
.graph-stats { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 10px; }
.stat-chip {
  background: var(--brand-soft);
  color: var(--brand);
  padding: 4px 10px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
}
.top-nodes-title { font-size: 12px; color: var(--text-3); margin-bottom: 6px; }
.node-list { display: flex; gap: 6px; flex-wrap: wrap; }
.node-chip {
  background: #f1f5f9;
  padding: 3px 8px;
  border-radius: 6px;
  font-size: 11px;
  font-family: monospace;
}

.algo-item {
  background: #f8fafc;
  border-radius: 8px;
  padding: 10px;
  margin-bottom: 8px;
}
.algo-name { font-weight: 600; font-size: 13px; }
.algo-rec { font-size: 12px; color: var(--text-2); margin: 4px 0; }
.algo-complexity { font-size: 11px; color: var(--text-3); }

.ai-insight {
  background: linear-gradient(135deg, #fef3c7, #fde68a);
  border-radius: 10px;
  padding: 14px;
  margin-top: 12px;
}
.ai-insight h5 {
  font-size: 13px;
  font-weight: 700;
  margin-bottom: 8px;
  display: flex;
  align-items: center;
  gap: 6px;
}
.insight-content { font-size: 13px; line-height: 1.7; white-space: pre-wrap; }

.debate-summary {
  margin-top: 20px;
  border-top: 1px solid var(--border);
  padding: 16px;
  background: linear-gradient(135deg, #f8fafc, #f1f5f9);
  border-radius: 12px;
}
.debate-final { font-size: 13px; line-height: 1.8; white-space: pre-wrap; }

.mode-switch { display: flex; }
.debate-mode .el-input-number { width: auto; }

.metrics-panel {
  background: var(--bg-page);
  border-radius: 12px;
  padding: 20px;
  border: 1px solid var(--border);
}
.metrics-panel .section-title { margin-bottom: 16px; }
</style>
