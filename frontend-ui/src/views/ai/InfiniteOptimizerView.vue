<template>
  <div class="page-container infinite-optimizer">
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">无穷维度优化实验室</h2>
        <p class="page-subtitle">
          CEM 交叉熵进化策略 · 连续高维配置空间自动寻优 · 多引擎横向对比 · 收敛性科学验证
        </p>
      </div>
      <div class="page-header-actions">
        <el-button type="primary" @click="goAIDeepOptimize">
          <el-icon><Promotion /></el-icon> AI深度优化
        </el-button>
        <span class="badge">DeepSeek</span>
        <span class="badge">OpenAI</span>
        <span class="badge">Claude</span>
        <span class="badge">豆包</span>
        <span class="badge">千问</span>
        <span class="badge">Kimi</span>
      </div>
    </div>

    <div class="page-content">

    <!-- KPI -->
    <div class="kpi-grid">
      <div class="kpi">
        <div class="kpi-label">当前最优得分</div>
        <div class="kpi-value">{{ bestScoreDisplay }}</div>
        <div class="kpi-hint">多目标加权（质量×速度×效率×稳定）</div>
      </div>
      <div class="kpi">
        <div class="kpi-label">运行状态</div>
        <div class="kpi-value" :class="'st-' + (status.status || 'idle')">{{ statusText }}</div>
        <div class="kpi-hint">{{ statusHint }}</div>
      </div>
      <div class="kpi">
        <div class="kpi-label">已评估配置</div>
        <div class="kpi-value">{{ status.evaluated_configs ?? 0 }}</div>
        <div class="kpi-hint">迭代 {{ status.iteration ?? 0 }}/{{ status.total_iterations ?? '—' }}</div>
      </div>
      <div class="kpi">
        <div class="kpi-label">优化维度数</div>
        <div class="kpi-value">{{ (status.dimensions && status.dimensions.length) || dimsCount }}</div>
        <div class="kpi-hint">温度 × 专家路由 × 上下文 × 引擎权重</div>
      </div>
    </div>

    <!-- 控制面板 -->
    <div class="panel card-pad">
      <div class="section-head">
        <h3 class="section-title">优化控制</h3>
        <div class="section-head-right">
          <el-tag v-if="status.converged" type="success" size="small">已收敛</el-tag>
          <el-button v-if="isRunning" type="danger" plain size="small" @click="stopRun">停止</el-button>
          <el-button v-else type="primary" size="small" :loading="starting" @click="startRun">
            <el-icon><VideoPlay /></el-icon> 启动自动寻优
          </el-button>
        </div>
      </div>
      <div class="ctrl-grid">
        <div class="ctrl-item">
          <label>迭代轮数（上限）</label>
          <el-input-number v-model="form.iterations" :min="1" :max="30" :disabled="isRunning" />
        </div>
        <div class="ctrl-item">
          <label>种群规模 / 轮</label>
          <el-input-number v-model="form.population" :min="3" :max="12" :disabled="isRunning" />
        </div>
        <div class="ctrl-item">
          <label>评估模式</label>
          <el-select v-model="form.evaluation_mode" :disabled="isRunning" style="width: 160px">
            <el-option label="快速（确定性校验）" value="fast" />
            <el-option label="完整（含弱分兜底）" value="full" />
          </el-select>
        </div>
        <div class="ctrl-item">
          <label>自动收敛停止</label>
          <el-switch v-model="autoConverge" disabled active-text="σ̄&lt;0.06 或 3 轮无改进" />
        </div>
      </div>
      <p class="method-desc">
        方法：将 AI 管线配置视为连续高维空间（"无穷维度"），用交叉熵方法（CEM）采样→评估→精英更新→分布收缩，
        自动迭代至收敛；每轮在 7 类基准任务上真实调用引擎打分，全程确定性校验、可复现。
      </p>
    </div>

    <!-- 收敛曲线 + 维度状态 -->
    <div class="two-col">
      <div class="panel card-pad">
        <div class="section-head">
          <h3 class="section-title">收敛曲线</h3>
          <div class="legend">
            <span class="lg lg-best">最优</span>
            <span class="lg lg-mean">种群均值</span>
          </div>
        </div>
        <div v-if="convergenceData.length" ref="convChartRef" class="chart-echarts"></div>
        <div v-else class="chart-empty">尚无收敛数据，启动寻优后实时绘制</div>
      </div>

      <div class="panel card-pad">
        <div class="section-head">
          <h3 class="section-title">维度敏感度排序</h3>
          <span class="mini-hint">|Pearson 相关| 越大越关键</span>
        </div>
        <div v-if="sensitivity.length" ref="sensChartRef" class="chart-echarts sens-chart"></div>
        <div v-else class="chart-empty">完成至少一轮迭代后计算</div>
      </div>
    </div>

    <!-- 最优配置 -->
    <div class="panel card-pad" v-if="bestConfig">
      <div class="section-head">
        <h3 class="section-title">最优配置（CEM 收敛解）</h3>
        <div class="section-head-right">
          <el-button type="primary" size="small" :loading="applying" @click="applyBest">
            <el-icon><Check /></el-icon> 应用到系统（激活引擎+路由权重）
          </el-button>
        </div>
      </div>
      <div class="best-grid">
        <div class="best-item"><label>采样温度</label><b>{{ bestConfig.temperature }}</b></div>
        <div class="best-item"><label>专家路由强度</label><b>{{ bestConfig.expert_routing }}</b></div>
        <div class="best-item"><label>上下文深度</label><b>{{ bestConfig.context_depth }} 轮</b></div>
      </div>
      <div class="weights">
        <div class="w-head">引擎路由权重（softmax 归一）</div>
        <div class="w-rows">
          <div v-for="(w, id) in bestConfig.provider_weights" :key="id" class="w-row">
            <span class="w-name">{{ engineName(id) }}</span>
            <div class="w-track"><div class="w-bar" :style="{ width: (w * 100).toFixed(1) + '%' }"></div></div>
            <span class="w-val">{{ (w * 100).toFixed(1) }}%</span>
          </div>
        </div>
      </div>
      <el-alert v-if="applyResult" :title="applyResult" type="success" :closable="true" style="margin-top: 12px" />
    </div>

    <!-- 多引擎对比 -->
    <div class="panel card-pad">
      <div class="section-head">
        <h3 class="section-title">多引擎横向对比</h3>
        <div class="section-head-right">
          <el-button size="small" :loading="comparing" @click="runComparison" :disabled="isRunning">
            <el-icon><DataAnalysis /></el-icon> 运行全引擎对比评测
          </el-button>
        </div>
      </div>
      <p class="method-desc">
        同一基准集（7 类任务 × 确定性校验）对所有已配置引擎独立评测：质量 / 延迟 / token 效率 / 稳定性四维打分，
        未配置引擎（OpenAI、Claude、豆包、千问、Kimi 等）一并列出，接入后即可参与对比。
      </p>
      <el-table :data="comparisonRows" size="small" class="cmp-table" :row-class-name="cmpRowClass">
        <el-table-column prop="name" label="引擎" min-width="150">
          <template #default="{ row }">
            <b>{{ row.name }}</b>
            <span v-if="row.model" class="dim"> · {{ row.model }}</span>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag :type="row.configured ? 'success' : 'info'" size="small">
              {{ row.configured ? '已接入' : '未配置' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="综合得分" width="100">
          <template #default="{ row }">{{ row.total_score != null ? row.total_score.toFixed(3) : '—' }}</template>
        </el-table-column>
        <el-table-column label="质量" width="80">
          <template #default="{ row }">{{ row.scores ? row.scores.quality.toFixed(2) : '—' }}</template>
        </el-table-column>
        <el-table-column label="延迟" width="80">
          <template #default="{ row }">{{ row.scores ? row.scores.latency.toFixed(2) : '—' }}</template>
        </el-table-column>
        <el-table-column label="稳定性" width="80">
          <template #default="{ row }">{{ row.scores ? row.scores.stability.toFixed(2) : '—' }}</template>
        </el-table-column>
        <el-table-column label="平均耗时" width="100">
          <template #default="{ row }">{{ row.scores && row.scores.avg_latency_ms ? row.scores.avg_latency_ms + 'ms' : '—' }}</template>
        </el-table-column>
        <el-table-column prop="verdict" label="评语" min-width="180">
          <template #default="{ row }">
            <span v-if="!row.configured" class="dim">在「LLM 配置」页添加 API Key 即可参与评测</span>
            <span v-else>{{ verdictOf(row) }}</span>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <!-- 基准任务 -->
    <div class="panel card-pad">
      <div class="section-head">
        <h3 class="section-title">基准测试集（7 维能力）</h3>
        <span class="mini-hint">全部确定性校验，结果可复现</span>
      </div>
      <div class="bench-grid">
        <div v-for="b in benchmarks" :key="b.id" class="bench-item">
          <div class="bench-cat">{{ b.category }}</div>
          <div class="bench-prompt">{{ b.prompt }}</div>
          <div class="bench-meta">校验: {{ b.check_type }} · 权重 {{ b.weight }}</div>
        </div>
      </div>
    </div>

    <!-- 历史运行 -->
    <div class="panel card-pad" v-if="runHistory.length">
      <div class="section-head">
        <h3 class="section-title">历史优化运行</h3>
        <span class="mini-hint">最优 {{ historyBest ? historyBest.score.toFixed(3) : '—' }} · 共 {{ runHistory.length }} 次</span>
      </div>
      <el-table :data="runHistory" size="small">
        <el-table-column prop="id" label="运行 ID" width="170" />
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag :type="row.converged ? 'success' : 'warning'" size="small">{{ row.converged ? '收敛' : '达上限' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="iterations" label="轮数" width="60" />
        <el-table-column prop="evaluated_configs" label="评估数" width="70" />
        <el-table-column label="最优得分" width="90">
          <template #default="{ row }">{{ row.best ? row.best.score.toFixed(3) : '—' }}</template>
        </el-table-column>
        <el-table-column prop="convergence_reason" label="收敛原因" min-width="200" />
        <el-table-column label="时间" width="160">
          <template #default="{ row }">{{ (row.finished_at || row.started_at || '').replace('T', ' ').slice(0, 19) }}</template>
        </el-table-column>
      </el-table>
    </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { VideoPlay, Check, DataAnalysis, Promotion } from '@element-plus/icons-vue'
import * as echarts from '@/echarts'
import * as api from '@/api'

const router = useRouter()

// AI深度优化：跳转到AI助手，带上优化上下文
function goAIDeepOptimize() {
  router.push({ path: '/ai', query: { source: 'infinite-optimizer', action: 'deep-optimize' } })
}

const form = ref({ iterations: 6, population: 5, evaluation_mode: 'fast' })
const autoConverge = ref(true)
const starting = ref(false)
const comparing = ref(false)
const applying = ref(false)
const applyResult = ref('')
const status = ref({})
const results = ref({ runs: [], best: null })
const comparison = ref(null)
const benchmarks = ref([])
const pollTimer = ref(null)
const convChartRef = ref(null)
const sensChartRef = ref(null)
let convChartInst = null
let sensChartInst = null

const isRunning = computed(() => status.value.status === 'running')
const bestConfig = computed(() => {
  if (status.value.best && status.value.best.config) return status.value.best.config
  const h = results.value.best
  return h ? h.config : null
})
const bestScoreDisplay = computed(() => {
  if (status.value.best) return status.value.best.score.toFixed(3)
  if (results.value.best) return results.value.best.score.toFixed(3)
  return '—'
})
const dimsCount = computed(() => (status.value.dimensions ? status.value.dimensions.length : 0))
const statusText = computed(() => {
  const map = { running: '寻优中', completed: '已完成', stopped: '已停止', failed: '失败', idle: '待启动' }
  return map[status.value.status] || '待启动'
})
const statusHint = computed(() => {
  if (status.value.status === 'running') return 'CEM 自动迭代直至收敛'
  if (status.value.converged) return status.value.convergence_reason || '已收敛'
  if (status.value.error) return status.value.error
  return '历史运行 ' + (results.value.runs ? results.value.runs.length : 0) + ' 次'
})
const convergenceData = computed(() => status.value.convergence || [])
const sensitivity = computed(() => {
  if (status.value.sensitivity && status.value.sensitivity.length) return status.value.sensitivity
  const runs = results.value.runs || []
  const last = runs[0]
  return (last && last.sensitivity) || []
})
const runHistory = computed(() => results.value.runs || [])
const historyBest = computed(() => results.value.best || null)
const comparisonRows = computed(() => (comparison.value && comparison.value.rows) || [])

// ---- ECharts 收敛曲线 + 维度敏感度 ----

/**
 * 数据降采样：当数据点超过阈值时，使用 LTTB 算法降采样到目标点数
 * 简化版：等间距采样 + 保留极值点
 */
function downsampleData(data, targetPoints = 200, threshold = 500) {
  if (!data || data.length <= threshold || data.length <= targetPoints) {
    return data
  }
  const n = data.length
  const result = []
  const step = n / targetPoints
  // 保留第一个点
  result.push(data[0])
  for (let i = 1; i < targetPoints - 1; i++) {
    const start = Math.floor((i - 0.5) * step)
    const end = Math.floor((i + 0.5) * step)
    const s = Math.max(1, start)
    const e = Math.min(n - 1, end)
    // 在区间内找 best 值的极值点（更能保留曲线形状）
    let bestIdx = s
    let bestVal = data[s].best
    for (let j = s + 1; j <= e; j++) {
      if (data[j].best > bestVal) {
        bestVal = data[j].best
        bestIdx = j
      }
    }
    result.push(data[bestIdx])
  }
  // 保留最后一个点
  result.push(data[n - 1])
  return result
}

function initConvChart() {
  if (!convChartRef.value || convChartInst) return
  convChartInst = echarts.init(convChartRef.value)
  convChartInst.setOption({
    grid: { left: 50, right: 20, top: 20, bottom: 40 },
    tooltip: {
      trigger: 'axis',
      formatter: (params) => {
        const p = params[0]
        return `第 ${p.axisValue} 轮<br/>最优：${p.data[1]?.toFixed(3) ?? '—'}<br/>均值：${params[1]?.data[1]?.toFixed(3) ?? '—'}`
      },
    },
    legend: {
      data: ['最优', '种群均值'],
      right: 10,
      top: 0,
      textStyle: { fontSize: 12, color: '#64748b' },
    },
    xAxis: {
      type: 'category',
      name: '迭代轮次',
      nameLocation: 'middle',
      nameGap: 25,
      nameTextStyle: { fontSize: 11, color: '#94a3b8' },
      axisLabel: { fontSize: 11, color: '#94a3b8' },
      axisLine: { lineStyle: { color: '#e2e8f0' } },
    },
    yAxis: {
      type: 'value',
      min: 0,
      max: 1,
      axisLabel: { fontSize: 11, color: '#94a3b8', formatter: (v) => v.toFixed(2) },
      splitLine: { lineStyle: { color: '#e2e8f0' } },
    },
    dataZoom: [
      {
        type: 'inside',
        start: 0,
        end: 100,
        zoomLock: false,
      },
      {
        type: 'slider',
        start: 0,
        end: 100,
        height: 20,
        bottom: 5,
        borderColor: '#e2e8f0',
        fillerColor: 'rgba(8, 145, 178, 0.12)',
        handleStyle: { color: '#0891b2' },
        textStyle: { fontSize: 10, color: '#94a3b8' },
      },
    ],
    series: [
      {
        name: '最优',
        type: 'line',
        smooth: true,
        symbol: 'circle',
        symbolSize: 6,
        lineStyle: { color: '#0891b2', width: 2.5 },
        itemStyle: { color: '#0891b2' },
        data: [],
      },
      {
        name: '种群均值',
        type: 'line',
        smooth: true,
        symbol: 'circle',
        symbolSize: 4,
        lineStyle: { color: '#a78bfa', width: 1.5, type: 'dashed' },
        itemStyle: { color: '#a78bfa' },
        data: [],
      },
    ],
  })
}

function renderConvChart() {
  if (!convChartInst) return
  const raw = convergenceData.value
  if (!raw.length) return
  const data = downsampleData(raw, 200, 500)
  const xData = data.map((d) => d.iteration)
  const bestData = data.map((d) => d.best)
  const meanData = data.map((d) => d.mean)
  convChartInst.setOption({
    xAxis: { data: xData },
    series: [
      { name: '最优', data: bestData },
      { name: '种群均值', data: meanData },
    ],
  })
}

function initSensChart() {
  if (!sensChartRef.value || sensChartInst) return
  sensChartInst = echarts.init(sensChartRef.value)
  sensChartInst.setOption({
    grid: { left: 110, right: 60, top: 10, bottom: 10 },
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'shadow' },
      formatter: (params) => {
        const p = params[0]
        const item = sensitivity.value.find((s) => s.dimension === p.name)
        if (!item) return p.name
        return `${item.dimension}<br/>相关系数：${item.correlation > 0 ? '+' : ''}${item.correlation}<br/>μ=${item.mu}  σ=${item.sigma}`
      },
    },
    xAxis: {
      type: 'value',
      min: -1,
      max: 1,
      axisLabel: { fontSize: 11, color: '#94a3b8', formatter: (v) => v.toFixed(1) },
      splitLine: { lineStyle: { color: '#e2e8f0' } },
    },
    yAxis: {
      type: 'category',
      inverse: true,
      axisLabel: { fontSize: 12, color: '#334155', fontWeight: 600 },
      axisLine: { show: false },
      axisTick: { show: false },
    },
    series: [
      {
        type: 'bar',
        barWidth: 14,
        label: {
          show: true,
          position: 'right',
          fontSize: 11,
          fontWeight: 700,
          color: '#0f172a',
          formatter: (p) => {
            const v = p.data
            return v > 0 ? '+' + v.toFixed(2) : v.toFixed(2)
          },
        },
        data: [],
      },
    ],
  })
}

function renderSensChart() {
  if (!sensChartInst) return
  const data = sensitivity.value
  if (!data.length) return
  const yData = data.map((d) => d.dimension)
  const barData = data.map((d) => ({
    value: d.correlation,
    itemStyle: {
      color: d.correlation >= 0
        ? { type: 'linear', x: 0, y: 0, x2: 1, y2: 0, colorStops: [{ offset: 0, color: '#22d3ee' }, { offset: 1, color: '#0891b2' }] }
        : { type: 'linear', x: 0, y: 0, x2: 1, y2: 0, colorStops: [{ offset: 0, color: '#fb923c' }, { offset: 1, color: '#ea580c' }] },
      borderRadius: [0, 4, 4, 0],
    },
  }))
  sensChartInst.setOption({
    yAxis: { data: yData },
    series: [{ data: barData }],
  })
}

function resizeCharts() {
  convChartInst && convChartInst.resize()
  sensChartInst && sensChartInst.resize()
}
function engineName(id) {
  const p = (status.value.providers || []).find((x) => x.id === id)
  if (p) return p.name
  const names = { deepseek: 'DeepSeek', openai: 'OpenAI', anthropic: 'Claude', volcengine: '豆包', qwen: '千问', kimi: 'Kimi', zhipu: '智谱', google: 'Gemini' }
  return names[id] || id
}
function verdictOf(row) {
  if (!row.scores) return '—'
  const s = row.scores
  if (s.stability < 1) return `稳定性不足（${(s.stability * 100).toFixed(0)}% 成功），建议检查网络或额度`
  if (s.quality >= 0.85 && s.latency >= 0.6) return '质量与速度俱佳，推荐主力引擎'
  if (s.quality >= 0.85) return '质量优秀，延迟偏高，适合复杂任务路由'
  if (s.quality >= 0.6) return '质量中等，可通过专家路由与温度调优提升'
  return '质量偏低，建议仅作对比参考'
}
function cmpRowClass({ row }) {
  return row.configured ? '' : 'row-dim'
}

// ---- 操作 ----
async function startRun() {
  starting.value = true
  try {
    const res = await api.startInfiniteOptimize(form.value)
    ElMessage.success(`寻优已启动：${res.dimensions} 维 · ${res.providers.length} 个引擎`)
    pollStatus()
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    starting.value = false
  }
}
async function stopRun() {
  try {
    await api.stopInfiniteOptimize()
    ElMessage.warning('已请求停止，当前候选评估完成后停止')
  } catch (e) {
    ElMessage.error(e.message)
  }
}
async function runComparison() {
  comparing.value = true
  try {
    const res = await api.runProviderComparison()
    comparison.value = res
    ElMessage.success('全引擎对比完成')
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    comparing.value = false
  }
}
async function applyBest() {
  applying.value = true
  applyResult.value = ''
  try {
    const res = await api.applyBestConfig(null)
    applyResult.value = '已应用：' + (res.applied || []).join('；')
    ElMessage.success('最优配置已应用到系统')
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    applying.value = false
  }
}

async function pollStatus() {
  try {
    status.value = await api.getInfiniteOptimizeStatus()
    if (status.value.status === 'running' && !pollTimer.value) {
      pollTimer.value = setInterval(pollStatus, 2000)
    }
    if (status.value.status !== 'running' && pollTimer.value) {
      clearInterval(pollTimer.value)
      pollTimer.value = null
      loadResults()
    }
  } catch (e) { /* 静默重试 */ }
}
async function loadResults() {
  try {
    results.value = await api.getInfiniteOptimizeResults()
  } catch (e) { /* 静默 */ }
}
async function loadStatic() {
  try {
    const [bench, cmp] = await Promise.all([api.getInfiniteBenchmarks(), api.getProviderComparison()])
    benchmarks.value = (bench && bench.benchmarks) || []
    if (cmp && cmp.rows && cmp.rows.length) comparison.value = cmp
  } catch (e) { /* 静默 */ }
}

onMounted(async () => {
  loadStatic()
  loadResults()
  pollStatus()
  await nextTick()
  if (convergenceData.value.length) {
    initConvChart()
    renderConvChart()
  }
  if (sensitivity.value.length) {
    initSensChart()
    renderSensChart()
  }
  window.addEventListener('resize', resizeCharts)
})
onUnmounted(() => {
  if (pollTimer.value) clearInterval(pollTimer.value)
  window.removeEventListener('resize', resizeCharts)
  if (convChartInst) {
    convChartInst.dispose()
    convChartInst = null
  }
  if (sensChartInst) {
    sensChartInst.dispose()
    sensChartInst = null
  }
})

// 监听数据变化，更新图表
watch(convergenceData, async (val) => {
  if (val && val.length) {
    await nextTick()
    if (!convChartInst) initConvChart()
    renderConvChart()
  }
})
watch(sensitivity, async (val) => {
  if (val && val.length) {
    await nextTick()
    if (!sensChartInst) initSensChart()
    renderSensChart()
  }
})
</script>

<style scoped>
.infinite-optimizer { display: flex; flex-direction: column; gap: 18px; }
.page-head { display: flex; justify-content: space-between; align-items: flex-end; gap: 20px; }
.page-head h2 { margin: 0 0 6px; font-size: 22px; }
.sub { margin: 0; color: #64748b; font-size: 13px; }
.head-badges { display: flex; gap: 6px; flex-wrap: wrap; }
.badge {
  font-size: 11.5px; padding: 3px 10px; border-radius: 999px;
  background: #eef2ff; color: #4f46e5; border: 1px solid #e0e7ff; font-weight: 600;
}
.kpi-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 14px; }
.kpi {
  background: var(--bg-panel, #fff); border: 1px solid var(--border, #e2e8f0);
  border-radius: 12px; padding: 14px 16px;
}
.kpi-label { font-size: 12px; color: #64748b; margin-bottom: 6px; }
.kpi-value { font-size: 26px; font-weight: 700; color: #0f172a; font-variant-numeric: tabular-nums; }
.kpi-value.st-running { color: #0891b2; }
.kpi-value.st-completed { color: #16a34a; }
.kpi-value.st-failed { color: #dc2626; }
.kpi-hint { font-size: 11.5px; color: #94a3b8; margin-top: 4px; }
.panel { background: var(--bg-panel, #fff); border: 1px solid var(--border, #e2e8f0); border-radius: 12px; }
.card-pad { padding: 18px 20px; }
.section-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px; }
.section-head-right { display: flex; align-items: center; gap: 10px; }
.section-title { margin: 0; font-size: 15px; }
.mini-hint { font-size: 12px; color: #94a3b8; }
.ctrl-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; }
.ctrl-item { display: flex; flex-direction: column; gap: 6px; }
.ctrl-item label { font-size: 12.5px; color: #475569; font-weight: 600; }
.method-desc {
  font-size: 12.5px; color: #64748b; line-height: 1.8; margin: 14px 0 0;
  padding: 10px 14px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px;
}
.two-col { display: grid; grid-template-columns: 3fr 2fr; gap: 14px; }
.chart-echarts { width: 100%; height: 260px; }
.sens-chart { height: 300px; }
.chart-empty {
  display: flex; align-items: center; justify-content: center; height: 180px;
  color: #94a3b8; font-size: 13px; background: #f8fafc; border-radius: 8px;
}
.legend { display: flex; gap: 12px; }
.lg { font-size: 12px; color: #64748b; display: flex; align-items: center; gap: 4px; }
.lg::before { content: ''; width: 14px; height: 3px; border-radius: 2px; display: inline-block; }
.lg-best::before { background: #0891b2; }
.lg-mean::before { background: #a78bfa; }
.best-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; margin-bottom: 16px; }
.best-item {
  padding: 12px 14px; background: #ecfeff; border: 1px solid #a5f3fc; border-radius: 10px;
  display: flex; flex-direction: column; gap: 4px;
}
.best-item label { font-size: 12px; color: #0e7490; }
.best-item b { font-size: 20px; color: #155e75; }
.weights { border-top: 1px dashed #e2e8f0; padding-top: 12px; }
.w-head { font-size: 12.5px; color: #475569; font-weight: 600; margin-bottom: 8px; }
.w-rows { display: flex; flex-direction: column; gap: 7px; }
.w-row { display: grid; grid-template-columns: 130px 1fr 56px; gap: 10px; align-items: center; font-size: 12.5px; }
.w-name { color: #334155; }
.w-track { height: 10px; background: #f1f5f9; border-radius: 5px; overflow: hidden; }
.w-bar { height: 100%; background: linear-gradient(90deg, #6366f1, #0891b2); border-radius: 5px; }
.w-val { text-align: right; font-variant-numeric: tabular-nums; color: #0f172a; font-weight: 600; }
.cmp-table :deep(.row-dim) { opacity: 0.55; }
.dim { color: #94a3b8; font-size: 12px; }
.bench-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 12px; }
.bench-item { border: 1px solid #e2e8f0; border-radius: 10px; padding: 12px 14px; background: #fafbfd; }
.bench-cat { font-size: 12px; color: #0891b2; font-weight: 700; margin-bottom: 6px; }
.bench-prompt { font-size: 12.5px; color: #334155; line-height: 1.6; margin-bottom: 6px; }
.bench-meta { font-size: 11px; color: #94a3b8; }
@media (max-width: 1100px) {
  .kpi-grid { grid-template-columns: repeat(2, 1fr); }
  .ctrl-grid { grid-template-columns: repeat(2, 1fr); }
  .two-col { grid-template-columns: 1fr; }
}
</style>
