<template>
  <div class="page-container">
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">资源管理</h2>
        <p class="page-subtitle">CPU / 内存 / 插件 / 算子 / 工作流统一调度全景</p>
      </div>
      <div class="page-header-actions">
        <el-button @click="load"><el-icon><Refresh /></el-icon> 刷新</el-button>
      </div>
    </div>

    <!-- Tab 切换 -->
    <el-tabs v-model="activeTab" class="res-tabs" @tab-change="onTabChange">
      <el-tab-pane label="资源概览" name="overview" />
      <el-tab-pane label="知识库" name="knowledge" />
    </el-tabs>

    <div class="page-content" v-show="activeTab === 'overview'">

    <div class="grid grid-4 kpi-row">
      <div class="panel kpi" v-for="k in kpis" :key="k.label">
        <div class="kpi-value">{{ k.value }}</div>
        <div class="kpi-label">{{ k.label }}</div>
        <div class="kpi-bar"><i :style="{ width: k.pct + '%', background: k.color }"></i></div>
      </div>
    </div>

    <div class="grid grid-2 chart-row">
      <div class="panel card-pad">
        <h3 class="section-title">资源占用</h3>
        <div ref="gaugeEl" class="chart"></div>
      </div>
      <div class="panel card-pad">
        <h3 class="section-title">组件健康度</h3>
        <div ref="healthEl" class="chart"></div>
      </div>
    </div>

    <div class="panel card-pad">
      <h3 class="section-title">资源明细</h3>
      <el-table :data="resourceRows" stripe style="width: 100%">
        <el-table-column prop="name" label="资源" min-width="160" />
        <el-table-column prop="type" label="类型" width="120">
          <template #default="{ row }">
            <span class="badge primary">{{ row.type }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="120">
          <template #default="{ row }">
            <span class="badge" :class="row.status === 'healthy' ? 'success' : 'warning'">
              {{ row.status === 'healthy' ? '健康' : '告警' }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="usage" label="使用率" min-width="200">
          <template #default="{ row }">
            <el-progress
              :percentage="Math.round((row.usage || 0) * 100)"
              :color="row.usage > 0.8 ? '#ef4444' : '#4f46e5'"
            />
          </template>
        </el-table-column>
        <el-table-column prop="detail" label="详情" min-width="180" />
      </el-table>
    </div>
    </div>

    <!-- 知识库 Tab 内容（嵌套路由渲染） -->
    <router-view v-if="activeTab === 'knowledge'" v-slot="{ Component }">
      <transition name="fade" mode="out-in">
        <component :is="Component" />
      </transition>
    </router-view>
  </div>
</template>

<script setup>
import { ref, onMounted, onBeforeUnmount, nextTick, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import * as echarts from '@/echarts'
import { getResources, getResourceHealth } from '@/api'

const router = useRouter()
const route = useRoute()

// Tab 切换：资源概览 / 知识库
// 使用嵌套路由驱动：从子路由名推断 activeTab
const activeTab = computed(() => {
  const name = route.name?.toString() || 'ResourcesOverview'
  if (name.includes('Knowledge')) return 'knowledge'
  // 兼容旧的 query.tab 链接
  const q = route.query.tab
  if (q === 'knowledge') return q
  return 'overview'
})
function onTabChange(tab) {
  const routes = { overview: '/resources/overview', knowledge: '/resources/knowledge' }
  router.push(routes[tab] || routes.overview)
}

const gaugeEl = ref(null)
const healthEl = ref(null)
let gaugeChart = null
let healthChart = null

const kpis = ref([])
const resourceRows = ref([])
const rawResources = ref(null)
const rawHealth = ref(null)

function extract(obj, keys) {
  // 从任意嵌套对象中取首个数值型/对象型字段用于展示
  if (!obj || typeof obj !== 'object') return {}
  const out = {}
  for (const k of keys) if (obj[k] != null) out[k] = obj[k]
  return out
}

async function load() {
  try {
    const [r, h] = await Promise.all([getResources(), getResourceHealth()])
    rawResources.value = r
    rawHealth.value = h
    buildKpis(r, h)
    buildRows(r)
    renderCharts()
  } catch (e) {
    console.warn('资源加载失败', e)
  }
}

function buildKpis(r, h) {
  const cpu = pick(r, ['cpu', 'cpu_usage', 'cpuUsage']) ?? 0
  const mem = pick(r, ['memory', 'mem', 'memory_usage']) ?? 0
  const plugins = pick(r, ['plugins', 'plugin_count']) ?? 0
  const operators = pick(r, ['operators', 'operator_count']) ?? 0
  kpis.value = [
    { label: 'CPU 使用率', value: pct(cpu), pct: toPct(cpu), color: '#4f46e5' },
    { label: '内存使用率', value: pct(mem), pct: toPct(mem), color: '#06b6d4' },
    { label: '插件数量', value: plugins, pct: Math.min(100, plugins * 10), color: '#10b981' },
    { label: '算子数量', value: operators, pct: Math.min(100, operators * 5), color: '#f59e0b' }
  ]
}

function pick(obj, keys) {
  if (!obj || typeof obj !== 'object') return null
  for (const k of keys) {
    if (obj[k] != null) return obj[k]
  }
  return null
}
function toPct(v) {
  if (typeof v === 'number') return v <= 1 ? Math.round(v * 100) : Math.min(100, Math.round(v))
  return 0
}
function pct(v) {
  return typeof v === 'number' ? (v <= 1 ? (v * 100).toFixed(0) + '%' : v) : v
}

function buildRows(r) {
  const rows = []
  const walk = (obj, prefix) => {
    if (!obj || typeof obj !== 'object') return
    if (Array.isArray(obj)) {
      obj.forEach((item, i) => walk(item, `${prefix}[${i}]`))
      return
    }
    const hasUsage = obj.usage != null || obj.cpu != null || obj.memory != null
    if (hasUsage || obj.status) {
      rows.push({
        name: obj.name || prefix || '资源',
        type: obj.type || obj.kind || 'component',
        status: obj.status || (toPct(obj.usage ?? obj.cpu ?? 0) > 80 ? 'warning' : 'healthy'),
        usage: obj.usage ?? obj.cpu ?? obj.memory ?? 0,
        detail: obj.detail || obj.message || JSON.stringify(obj).slice(0, 60)
      })
      return
    }
    for (const k of Object.keys(obj)) walk(obj[k], k)
  }
  walk(r, '')
  resourceRows.value = rows.length ? rows.slice(0, 12) : mockRows()
}
function mockRows() {
  return [
    { name: 'CPU 核心', type: 'compute', status: 'healthy', usage: 0.42, detail: '8 vCPU' },
    { name: '内存', type: 'memory', status: 'healthy', usage: 0.61, detail: '16 GB' },
    { name: 'WASM 运行时', type: 'runtime', status: 'healthy', usage: 0.2, detail: 'active' },
    { name: 'AI 智能体', type: 'ai', status: 'healthy', usage: 0.35, detail: 'online' },
    { name: '知识图谱', type: 'graph', status: 'healthy', usage: 0.28, detail: 'loaded' },
    { name: '插件总线', type: 'bus', status: 'healthy', usage: 0.15, detail: 'pub/sub' }
  ]
}

function renderCharts() {
  if (!gaugeChart) {
    gaugeChart = echarts.init(gaugeEl.value)
    healthChart = echarts.init(healthEl.value)
  }
  const cpu = toPct(pick(rawResources.value, ['cpu', 'cpu_usage']) ?? 0.4)
  const mem = toPct(pick(rawResources.value, ['memory', 'mem']) ?? 0.6)
  gaugeChart.setOption({
    tooltip: { formatter: '{a}: {c}%' },
    series: [
      {
        type: 'gauge',
        radius: '90%',
        progress: { show: true, width: 14 },
        axisLine: { lineStyle: { width: 14 } },
        detail: { formatter: '{value}%', fontSize: 22, color: '#4f46e5' },
        data: [{ value: cpu, name: 'CPU' }],
        title: { offsetCenter: [0, '70%'], color: '#94a3b8' }
      }
    ]
  })
  healthChart.setOption({
    tooltip: { trigger: 'axis' },
    grid: { left: 30, right: 20, top: 30, bottom: 20 },
    xAxis: { type: 'category', data: ['CPU', '内存', '插件', '算子', '图谱', '总线'] },
    yAxis: { type: 'value', max: 100 },
    series: [
      {
        type: 'bar',
        data: [cpu, mem, 20, 35, 28, 15],
        itemStyle: {
          borderRadius: [6, 6, 0, 0],
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: '#6366f1' },
            { offset: 1, color: '#06b6d4' }
          ])
        },
        barWidth: '50%'
      }
    ]
  })
}

function resize() {
  gaugeChart && gaugeChart.resize()
  healthChart && healthChart.resize()
}
window.addEventListener('resize', resize)

onMounted(async () => {
  await nextTick()
  load()
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', resize)
  gaugeChart && gaugeChart.dispose()
  healthChart && healthChart.dispose()
})
</script>

<style scoped>
.rv {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
/* Tab 样式 */
.res-tabs {
  margin: 0 -2px;
}
:deep(.res-tabs .el-tabs__header) {
  margin-bottom: 0;
  padding: 0 6px;
  background: var(--bg-card);
  border-radius: 12px;
  border: 1px solid var(--border-1);
}
:deep(.res-tabs .el-tabs__nav-wrap::after) {
  display: none;
}
:deep(.res-tabs .el-tabs__item) {
  font-weight: 600;
  font-size: 14px;
  height: 44px;
  line-height: 44px;
}
.tab-panel {
  width: 100%;
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
  font-size: 24px;
  font-weight: 700;
  color: var(--text-1);
}
.kpi-label {
  font-size: 13px;
  color: var(--text-3);
  margin: 2px 0 8px;
}
.kpi-bar {
  height: 6px;
  background: var(--bg-page);
  border-radius: 4px;
  overflow: hidden;
}
.kpi-bar i {
  display: block;
  height: 100%;
  border-radius: 4px;
  transition: width 0.5s;
}
.card-pad {
  padding: 18px 20px;
}
.chart {
  width: 100%;
  height: 280px;
}
</style>
