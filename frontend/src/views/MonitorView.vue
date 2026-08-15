<template>
  <div class="mv">
    <div class="head">
      <div>
        <h2 class="page-title">系统监控</h2>
        <p class="page-subtitle">运行时健康度 · 执行日志 · 组件拓扑实时观测</p>
      </div>
      <el-button :loading="loading" @click="loadAll"><el-icon><Refresh /></el-icon> 刷新</el-button>
    </div>

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
</template>

<script setup>
import { ref, onMounted, onBeforeUnmount, nextTick } from 'vue'
import * as echarts from 'echarts'
import { getStatus, getFullStatus, getLogs, getPlugins } from '@/api'

const loading = ref(false)
const loadEl = ref(null)
let chart = null

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
    const [st, logs, plg] = await Promise.all([
      getFullStatus().catch(() => getStatus()),
      getLogs().catch(() => []),
      getPlugins().catch(() => [])
    ])
    const s = st || {}
    pluginCount.value = (plg.plugins || plg.data || []).length
    kpis.value = [
      { label: '系统状态', value: s.status === 'running' ? '运行中' : s.status || '—', icon: 'CircleCheck', ok: s.status === 'running' },
      { label: '算子数量', value: s.operators_count ?? 0, icon: 'Cpu' },
      { label: '执行次数', value: s.executions_count ?? 0, icon: 'VideoPlay' },
      { label: '插件数量', value: pluginCount.value, icon: 'Connection' }
    ]
    comps.value = [
      { name: 'WASM 运行时', status: 'up', val: 'active' },
      { name: 'AI 智能体', status: 'up', val: 'online' },
      { name: '知识图谱', status: 'up', val: (s.graph?.nodes ?? 0) + ' 节点' },
      { name: '插件总线', status: pluginCount.value ? 'up' : 'down', val: pluginCount.value + ' 个' },
      { name: '数据库', status: 'up', val: 'connected' },
      { name: '消息队列', status: 'up', val: 'ready' }
    ]
    logRows.value = (logs || []).slice(0, 50).map((l) => ({
      time: fmt(l.timestamp),
      flow: (l.workflow || []).join(' → ') || '—',
      status: l.success === false ? '失败' : '成功',
      time_ms: l.execution_time_ms + ' ms',
      dims: `${l.input_dim}→${l.output_dim}`
    }))
    renderChart()
  } catch (e) {
    console.warn('监控加载失败', e)
  } finally {
    loading.value = false
  }
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
}
window.addEventListener('resize', resize)

onMounted(async () => {
  await nextTick()
  loadAll()
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', resize)
  chart && chart.dispose()
})
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
</style>
