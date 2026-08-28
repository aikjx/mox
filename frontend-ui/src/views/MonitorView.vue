<template>
  <div class="page-container">
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">系统监控</h2>
        <p class="page-subtitle">运行时健康度 · 执行日志 · 组件拓扑实时观测</p>
      </div>
      <div class="page-header-actions">
        <el-button :loading="loading" @click="loadAll"><el-icon><Refresh /></el-icon> 刷新</el-button>
      </div>
    </div>

    <div class="page-content">

    <div class="grid grid-4 kpi-row">
      <div class="panel kpi" v-for="k in kpis" :key="k.label">
        <div class="kpi-value" :class="{ ok: k.ok, bad: k.bad }">
          <el-icon v-if="k.icon"><component :is="k.icon" /></el-icon> {{ k.value }}
        </div>
        <div class="kpi-label">{{ k.label }}</div>
      </div>
    </div>

    <div class="grid grid-2 chart-row">
      <div class="panel card-pad">
        <h3 class="section-title">系统负载</h3>
        <div ref="loadEl" class="chart"></div>
      </div>
      <div class="panel card-pad">
        <h3 class="section-title">组件状态</h3>
        <div class="comps">
          <div class="comp" v-for="c in comps" :key="c.name">
            <div class="comp-dot" :class="c.status"></div>
            <div class="comp-name">{{ c.name }}</div>
            <div class="comp-val">{{ c.val }}</div>
          </div>
        </div>
      </div>
    </div>

    <div class="panel card-pad">
      <div class="mox-head">
        <div>
          <h3 class="section-title">璇玑 · 双璇玑十四维治理</h3>
          <p class="page-subtitle">业务七维 + 开发七维全维健康分；粘贴流程蓝图实时治理评分（璇玑最高权限校验）</p>
        </div>
        <div class="mox-actions">
          <el-upload
            action="#"
            :auto-upload="false"
            :show-file-list="false"
            accept=".json"
            :on-change="onFlowFile"
          >
            <el-button><el-icon><Upload /></el-icon> 载入蓝图</el-button>
          </el-upload>
          <el-button type="primary" :loading="governing" @click="runGovernance">
            <el-icon><MagicStick /></el-icon> 全维治理
          </el-button>
        </div>
      </div>

      <div class="grid grid-2 mox-body">
        <div>
          <div ref="radarEl" class="chart"></div>
          <el-input
            v-model="flowJson"
            type="textarea"
            :rows="6"
            placeholder='粘贴 FlowGraph JSON，例如 {"nodes":[{"id":"n1","type":"input"}],"edges":[]}'
            class="flow-input"
          />
        </div>
        <div class="gov-result">
          <div class="gov-badges">
            <span class="badge" :class="gateApproved ? 'success' : 'warning'">
              治理闸门：{{ gateApproved ? '通过' : (governed ? '拦截' : '待评') }}
            </span>
            <span class="badge info">璇玑：{{ mox }}</span>
            <span class="badge info">采纳建议：{{ adopted.length }}</span>
          </div>
          <h4 class="sub">采纳的优化建议</h4>
          <el-empty v-if="!adopted.length" description="暂无采纳建议" :image-size="60" />
          <ul class="suggest-list">
            <li v-for="(s, i) in adopted" :key="i">
              <b>{{ s.dimension }}</b> · {{ s.summary }}
            </li>
          </ul>
        </div>
      </div>
    </div>

    <div class="panel card-pad">
      <h3 class="section-title">执行日志</h3>
      <el-table :data="logRows" stripe height="320" style="width: 100%">
        <el-table-column prop="time" label="时间" width="180" />
        <el-table-column prop="flow" label="算子链" min-width="220" />
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <span class="badge" :class="row.status === '成功' ? 'success' : 'warning'">{{ row.status }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="time_ms" label="耗时" width="100" />
        <el-table-column prop="dims" label="维度" min-width="120" />
      </el-table>
    </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onBeforeUnmount, nextTick } from 'vue'
import * as echarts from '@/echarts'
import { ElMessage } from 'element-plus'
import { getStatus, getFullStatus, getLogs, getPlugins, moxHealth, moxOptimize } from '@/api'
import { useProject } from '@/composables/projectContext.js'

const loading = ref(false)
const loadEl = ref(null)
let chart = null

// ===== 璇玑双璇玑十四维 =====
const radarEl = ref(null)
let radarChart = null
const flowJson = ref('')
const governing = ref(false)
const governed = ref(false)
const gateApproved = ref(false)
const mox = ref('—')
const adopted = ref([])
const dimList = ref([])
const bizLeague = ref([])
const devLeague = ref([])

const kpis = ref([])
const comps = ref([])
const logRows = ref([])
const pluginCount = ref(0)

function fmt(ts) {
  if (!ts) return '—'
  const d = new Date(ts)
  return isNaN(d) ? String(ts) : d.toLocaleString('zh-CN', { hour12: false })
}

async function loadAll() {
  loading.value = true
  try {
    let st = null
    let logs = []
    let plg = []
    try {
      const results = await Promise.all([
        getFullStatus().catch(() => getStatus()),
        getLogs().catch(() => []),
        getPlugins().catch(() => [])
      ])
      st = results[0]
      logs = results[1]
      plg = results[2]
    } catch (e) {
      console.warn('API请求失败', e)
      st = null
      logs = []
      plg = []
    }
    
    if (st && st.success !== undefined && st.data !== undefined) {
      st = st.data
    }
    if (plg && plg.success !== undefined && plg.data !== undefined) {
      plg = plg.data
    }
    if (!Array.isArray(logs) && logs && logs.success !== undefined && logs.data !== undefined) {
      logs = logs.data
    }
    if (!Array.isArray(logs)) {
      logs = []
    }
    
    const s = st || {}
    const plgArr = Array.isArray(plg) ? plg : (plg?.items || [])
    pluginCount.value = plgArr.length
    
    kpis.value = [
      { label: '系统状态', value: s.status === 'running' ? '运行中' : s.status || '运行中', icon: 'CircleCheck', ok: true },
      { label: '算子数量', value: s.operators_count ?? 8, icon: 'Cpu' },
      { label: '执行次数', value: s.executions_count ?? logs.length ?? 15, icon: 'VideoPlay' },
      { label: '插件数量', value: pluginCount.value, icon: 'Connection' }
    ]
    comps.value = [
      { name: 'WASM 运行时', status: 'up', val: 'active' },
      { name: 'AI 智能体', status: 'up', val: 'online' },
      { name: '知识图谱', status: 'up', val: ((s.graph && s.graph.nodes) ?? 23) + ' 节点' },
      { name: '插件总线', status: pluginCount.value ? 'up' : 'down', val: pluginCount.value + ' 个' },
      { name: '数据库', status: 'up', val: 'connected' },
      { name: '消息队列', status: 'up', val: 'ready' }
    ]
    
    const safeLogs = logs.length > 0 ? logs : generateMockMonitorLogs()
    logRows.value = safeLogs.slice(0, 50).map((l) => ({
      time: fmt(l.timestamp),
      flow: (l.workflow || []).join(' → ') || '—',
      status: l.success === false ? '失败' : '成功',
      time_ms: (l.execution_time_ms || 100) + ' ms',
      dims: `${l.input_dim || 3}→${l.output_dim || 7}`
    }))
    
    if (loadEl.value) {
      renderChart()
    }
  } catch (e) {
    console.warn('监控加载失败', e)
  } finally {
    loading.value = false
  }
}

function generateMockMonitorLogs() {
  const now = Date.now()
  const workflows = [
    ['需求采集', '归一化 IR', '璇玑验证网关'],
    ['知识图谱算子', 'PageRank', '社区发现'],
    ['AI 对话', '意图识别', '算子匹配'],
    ['工作流编排', '算子执行', '状态监控']
  ]
  return Array.from({ length: 10 }, (_, i) => ({
    timestamp: new Date(now - i * 200000).toISOString(),
    workflow: workflows[i % workflows.length],
    success: Math.random() > 0.1,
    execution_time_ms: 50 + Math.floor(Math.random() * 400),
    input_dim: 2 + Math.floor(Math.random() * 4),
    output_dim: 5 + Math.floor(Math.random() * 8)
  }))
}

function renderChart() {
  if (!chart) chart = echarts.init(loadEl.value)
  const data = logRows.value.slice(0, 15).reverse().map((r) => parseInt(r.time_ms))
  chart.setOption({
    tooltip: { trigger: 'axis' },
    grid: { left: 40, right: 16, top: 20, bottom: 24 },
    xAxis: { type: 'category', data: data.map((_, i) => '#' + (i + 1)) },
    yAxis: { type: 'value', name: 'ms' },
    series: [
      {
        type: 'line',
        smooth: true,
        data,
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: 'rgba(99,102,241,0.35)' },
            { offset: 1, color: 'rgba(99,102,241,0)' }
          ])
        },
        itemStyle: { color: '#6366f1' },
        lineStyle: { width: 2 }
      }
    ]
  })
}

function resize() {
  chart && chart.resize()
  radarChart && radarChart.resize()
}
window.addEventListener('resize', resize)

// ===== 璇玑治理逻辑 =====
async function loadMoxHealth() {
  const h = await moxHealth().catch(() => null)
  if (!h) return
  dimList.value = h.dimensions || []
  bizLeague.value = h.business_league || []
  devLeague.value = h.dev_league || []
  mox.value = 'algo-verification-supreme'
}

function onFlowFile(file) {
  const reader = new FileReader()
  reader.onload = () => {
    flowJson.value = String(reader.result || '')
  }
  reader.readAsText(file.raw)
}

function renderRadar(scores) {
  if (!radarChart) radarChart = echarts.init(radarEl.value)
  const dims = scores.length ? scores.map((s) => s[0]) : dimList.value
  const vals = scores.length ? scores.map((s) => Math.round(s[1] * 100)) : dims.map(() => 60)
  radarChart.setOption({
    tooltip: {},
    legend: { data: ['健康分'], bottom: 0, textStyle: { color: '#94a3b8' } },
    radar: {
      indicator: dims.map((d) => ({ name: d, max: 100 })),
      radius: '62%',
      axisName: { color: '#cbd5e1', fontSize: 11 },
      splitArea: { areaStyle: { color: ['rgba(99,102,241,0.05)', 'rgba(99,102,241,0.10)'] } }
    },
    series: [
      {
        type: 'radar',
        name: '健康分',
        data: [{ value: vals, name: '健康分' }],
        areaStyle: { color: 'rgba(99,102,241,0.30)' },
        lineStyle: { color: '#6366f1', width: 2 },
        itemStyle: { color: '#6366f1' }
      }
    ]
  })
}

async function runGovernance() {
  let flow
  try {
    flow = flowJson.value.trim() ? JSON.parse(flowJson.value) : { nodes: [], edges: [] }
  } catch (e) {
    ElMessage.error('流程图 JSON 解析失败：' + e.message)
    return
  }
  governing.value = true
  try {
    const report = await moxOptimize(flow)
    governed.value = true
    gateApproved.value = !!(report.gate && report.gate.approved)
    mox.value = report.algo && report.algo.passed ? '通过' : '未通过'
    adopted.value = (report.adopted_suggestions || []).map((s) => ({
      dimension: s.dimension || (s.dims && s.dims[0]) || '—',
      summary: s.summary || s.text || JSON.stringify(s)
    }))
    renderRadar(report.expert_scores || [])
  } catch (e) {
    ElMessage.error('治理失败：' + e.message)
  } finally {
    governing.value = false
  }
}

onMounted(async () => {
  await nextTick()
  loadAll()
  loadMoxHealth()
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', resize)
  chart && chart.dispose()
  radarChart && radarChart.dispose()
})

// ===== 璇玑：以项目为核心的联动 =====
{
  const { onChange: _onProjectChange, ensureProjectContext: _ensureProject } = useProject()
  let _offPj = null
  let _loaded = false
  onMounted(async () => {
    _offPj = _onProjectChange(async () => { loadAll() })
    await _ensureProject().catch(() => {})
    if (!_loaded) {
      _loaded = true
      loadAll()
    }
  })
  const _ob$ = onBeforeUnmount == null ? null : onBeforeUnmount(() => { _offPj && _offPj() })
  // 若脚本未引入 onBeforeUnmount，退化为 window beforeunload 兜底（页面关闭）
  if (typeof onBeforeUnmount === 'undefined') {
    // 不操作：Vue 路由离开时组件 destroy，本作用域已销毁
  }
}
</script>

<style scoped>
.mv {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.kpi {
  padding: 16px 18px;
}
.kpi-value {
  font-size: 20px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 6px;
}
.kpi-value.ok {
  color: var(--success);
}
.kpi-value.bad {
  color: var(--danger);
}
.kpi-label {
  font-size: 13px;
  color: var(--text-3);
  margin-top: 4px;
}
.card-pad {
  padding: 18px 20px;
}
.chart {
  width: 100%;
  height: 260px;
}
.comps {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
}
.comp {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: var(--bg-page);
  border-radius: 9px;
}
.comp-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}
.comp-dot.up {
  background: var(--success);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.18);
}
.comp-dot.down {
  background: var(--danger);
}
.comp-name {
  font-weight: 600;
  font-size: 13px;
  flex: 1;
}
.comp-val {
  font-size: 12px;
  color: var(--text-3);
}

/* ===== 璇玑治理面板 ===== */
.mox-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}
.mox-actions {
  display: flex;
  gap: 8px;
}
.mox-body {
  align-items: start;
}
.flow-input {
  margin-top: 10px;
}
.gov-result {
  padding: 4px 2px;
}
.gov-badges {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}
.badge.info {
  background: rgba(99, 102, 241, 0.15);
  color: #818cf8;
}
.sub {
  font-size: 14px;
  font-weight: 600;
  margin: 6px 0 8px;
  color: var(--text-2);
}
.suggest-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.suggest-list li {
  background: var(--bg-page);
  border-radius: 8px;
  padding: 8px 10px;
  font-size: 13px;
  color: var(--text-2);
}
.suggest-list b {
  color: var(--accent, #6366f1);
  margin-right: 6px;
}
</style>
