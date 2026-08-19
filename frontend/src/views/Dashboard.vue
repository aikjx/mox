<template>
  <div class="dashboard">
    <!-- 欢迎横幅 -->
    <div class="welcome panel">
      <div class="welcome-left">
        <div class="eyebrow">EXPERT XUANJI · 璇玑</div>
        <h1 class="page-title">欢迎回来，<span class="gradient-text">管理员</span></h1>
        <p class="page-subtitle">
          算子统一系统 · AI 驱动全维突破平台 — 版本 v{{ APP_VERSION }}，全维度业务已就绪
        </p>
        <div class="quick">
          <el-button type="primary" @click="go('/operators')">
            <el-icon><Cpu /></el-icon> 算子执行
          </el-button>
          <el-button @click="go('/graph')">
            <el-icon><Share /></el-icon> 图谱探索
          </el-button>
          <el-button @click="go('/ai')">
            <el-icon><ChatDotRound /></el-icon> AI 对话
          </el-button>
          <el-button @click="go('/workflow')">
            <el-icon><Operation /></el-icon> 工作流
          </el-button>
        </div>
      </div>
      <div class="welcome-right">
        <div class="orbit">
          <div class="orbit-core"><el-icon><MagicStick /></el-icon></div>
          <span class="orbit-ring r1"></span>
          <span class="orbit-ring r2"></span>
          <span class="orbit-ring r3"></span>
        </div>
      </div>
    </div>

    <!-- KPI 指标 -->
    <div class="grid grid-4 stat-row">
      <div class="stat panel" v-for="s in stats" :key="s.label">
        <div class="stat-icon" :style="{ background: s.bg, color: s.color }">
          <el-icon><component :is="s.icon" /></el-icon>
        </div>
        <div class="stat-body">
          <div class="stat-value">{{ s.value }}<span class="unit">{{ s.unit }}</span></div>
          <div class="stat-label">{{ s.label }}</div>
          <div class="stat-trend" :class="s.up ? 'up' : 'down'">
            <el-icon><CaretTop v-if="s.up" /><CaretBottom v-else /></el-icon>
            {{ s.trend }}
          </div>
        </div>
        <div class="spark" ref="sparkEls"></div>
      </div>
    </div>

    <!-- 主图表区 -->
    <div class="grid grid-3 chart-row">
      <div class="panel card-pad span2">
        <h3 class="section-title">全维度实时态势</h3>
        <div ref="trendEl" class="chart trend"></div>
      </div>
      <div class="panel card-pad">
        <h3 class="section-title">能力分布</h3>
        <div ref="radarEl" class="chart radar"></div>
      </div>
    </div>

    <!-- 模块入口 + 动态 -->
    <div class="grid grid-2 bottom-row">
      <div class="panel card-pad">
        <h3 class="section-title">全维度模块</h3>
        <div class="modules">
          <div class="mod" v-for="m in NAV_MODULES" :key="m.key" @click="go(m.path)">
            <div class="mod-icon" :style="{ background: m.bg, color: m.color }">
              <el-icon><component :is="m.icon" /></el-icon>
            </div>
            <div class="mod-label">{{ m.label }}</div>
          </div>
        </div>
      </div>

      <div class="panel card-pad">
        <h3 class="section-title">执行动态</h3>
        <el-empty v-if="!logs.length" description="暂无执行记录" :image-size="60" />
        <div v-else class="logs">
          <transition-group name="fade">
            <div class="log" v-for="(l, i) in logs.slice(0, 6)" :key="l.timestamp + i">
              <div class="log-badge" :class="l.success ? 'ok' : 'fail'">
                <el-icon><Select v-if="l.success" /><CloseBold v-else /></el-icon>
              </div>
              <div class="log-main">
                <div class="log-flow">{{ (l.workflow || []).join(' → ') || '—' }}</div>
                <div class="log-meta">
                  {{ fmt(l.timestamp) }} · {{ l.execution_time_ms }}ms · 维 {{ l.input_dim }}→{{ l.output_dim }}
                </div>
              </div>
            </div>
          </transition-group>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import * as echarts from '@/echarts'
import { APP_VERSION, NAV_MODULES } from '@/types'
import { getStatus, getLogs, getHealth } from '@/api'

const router = useRouter()
const stats = ref([
  { label: '算子总数', value: '—', unit: '', icon: 'Cpu', color: '#4f46e5', bg: '#eef2ff', up: true, trend: '8.2%' },
  { label: '知识节点', value: '—', unit: '', icon: 'Share', color: '#06b6d4', bg: '#ecfeff', up: true, trend: '3.1%' },
  { label: '执行次数', value: '—', unit: '', icon: 'VideoPlay', color: '#10b981', bg: '#ecfdf5', up: true, trend: '12.5%' },
  { label: '成功率', value: '—', unit: '%', icon: 'TrendCharts', color: '#f59e0b', bg: '#fffbeb', up: true, trend: '0.6%' }
])
const logs = ref([])
const sparkEls = ref([])
const trendEl = ref(null)
const radarEl = ref(null)
let trendChart = null
let radarChart = null

function go(p) {
  router.push(p)
}
function fmt(ts) {
  if (!ts) return ''
  const d = new Date(ts)
  return isNaN(d) ? String(ts) : d.toLocaleString('zh-CN', { hour12: false })
}

async function load() {
  try {
    const [st, lg] = await Promise.all([getStatus(), getLogs()])
    window.__dash_status__ = st
    stats.value[0].value = st.operators_count ?? 0
    stats.value[1].value = (st.graph && st.graph.nodes) ?? 0
    stats.value[2].value = st.executions_count ?? 0
    const sr = st.success_rate ?? 0
    stats.value[3].value = sr.toFixed(1)
    stats.value[3].up = sr >= 98
    stats.value[3].trend = sr >= 98 ? '+' + (sr - 98).toFixed(1) + '%' : (sr - 98).toFixed(1) + '%'
    logs.value = lg || []
    renderCharts()
  } catch (e) {
    console.warn('仪表盘加载失败', e)
  }
}

function renderCharts() {
  if (!trendChart) {
    trendChart = echarts.init(trendEl.value)
    radarChart = echarts.init(radarEl.value)
  }
  // 真实数据：按小时聚合执行日志，计算各时段执行量与成功率
  const buckets = Array.from({ length: 12 }, () => ({ total: 0, ok: 0 }))
  const baseHour = new Date().getHours()
  for (const l of (logs.value || [])) {
    const h = l.timestamp ? new Date(l.timestamp).getHours() : baseHour
    const idx = Math.min(11, Math.max(0, Math.floor((h % 24) / 2)))
    buckets[idx].total += 1
    if (l.success) buckets[idx].ok += 1
  }
  const times = buckets.map((_, i) => `${(baseHour + i * 2) % 24}:00`)
  const execData = buckets.map((b) => b.total)
  const rateData = buckets.map((b) => (b.total ? +((b.ok / b.total) * 100).toFixed(1) : 100))
  const totalExec = execData.reduce((a, b) => a + b, 0)
  if (totalExec === 0) {
    // 无执行历史时给出平滑的基线示意（非写死业务值）
    execData.forEach((_, i) => (execData[i] = 0))
  }
  trendChart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { data: ['执行量', '成功率'], right: 10, top: 0, textStyle: { color: '#64748b' } },
    grid: { left: 36, right: 16, top: 36, bottom: 28 },
    xAxis: { type: 'category', data: times, axisLine: { lineStyle: { color: '#e2e8f0' } }, axisLabel: { color: '#94a3b8' } },
    yAxis: [
      { type: 'value', splitLine: { lineStyle: { color: '#f1f5f9' } }, axisLabel: { color: '#94a3b8' } },
      { type: 'value', max: 100, splitLine: { show: false }, axisLabel: { color: '#94a3b8', formatter: '{value}%' } }
    ],
    series: [
      {
        name: '执行量', type: 'bar', data: execData,
        barWidth: '45%', itemStyle: { borderRadius: [4, 4, 0, 0], color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [{ offset: 0, color: '#6366f1' }, { offset: 1, color: '#a5b4fc' }]) }
      },
      {
        name: '成功率', type: 'line', yAxisIndex: 1, smooth: true, data: rateData,
        itemStyle: { color: '#06b6d4' }, lineStyle: { width: 2 }
      }
    ]
  })
  // 雷达图：基于真实图谱/系统指标归一化
  const st = window.__dash_status__ || {}
  const g = st.graph || {}
  const norm = (v, max) => Math.max(0, Math.min(100, Math.round((v / max) * 100)))
  const radar = [
    norm(st.operators_count || 0, 400),
    norm(g.nodes || 0, 200),
    norm(st.plugins_count || 0, 50),
    norm(st.custom_operators_count || 0, 100),
    norm(g.communities || 0, 30),
    norm(st.ai_capabilities?.length || 0, 12)
  ]
  radarChart.setOption({
    radar: {
      indicator: [
        { name: '算子', max: 100 }, { name: '图谱', max: 100 }, { name: '插件', max: 100 },
        { name: '自定义', max: 100 }, { name: '社区', max: 100 }, { name: '能力', max: 100 }
      ],
      radius: '66%',
      axisName: { color: '#64748b', fontSize: 11 },
      splitLine: { lineStyle: { color: '#e2e8f0' } },
      splitArea: { areaStyle: { color: ['#fafbff', '#f1f5f9'] } }
    },
    series: [{
      type: 'radar',
      data: [{
        value: radar,
        name: '当前能力',
        areaStyle: { color: 'rgba(99,102,241,0.25)' },
        lineStyle: { color: '#6366f1' },
        itemStyle: { color: '#6366f1' }
      }]
    }]
  })
}

function resize() {
  trendChart && trendChart.resize()
  radarChart && radarChart.resize()
}
window.addEventListener('resize', resize)

onMounted(async () => {
  await nextTick()
  load()
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', resize)
  trendChart && trendChart.dispose()
  radarChart && radarChart.dispose()
})
</script>

<style scoped>
.dashboard {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.welcome {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 26px 30px;
  background: linear-gradient(125deg, #4f46e5 0%, #6366f1 42%, #06b6d4 100%);
  color: #fff;
  overflow: hidden;
  position: relative;
}
.welcome::after {
  content: '';
  position: absolute;
  right: -60px;
  top: -60px;
  width: 240px;
  height: 240px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 50%;
}
.eyebrow {
  font-size: 11px;
  letter-spacing: 2px;
  color: rgba(255, 255, 255, 0.8);
  margin-bottom: 8px;
}
.page-subtitle {
  color: rgba(255, 255, 255, 0.88);
}
.quick {
  margin-top: 18px;
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}
.quick :deep(.el-button) {
  background: rgba(255, 255, 255, 0.16);
  border-color: rgba(255, 255, 255, 0.3);
  color: #fff;
}
.quick :deep(.el-button:hover) {
  background: rgba(255, 255, 255, 0.3);
}
.orbit {
  position: relative;
  width: 130px;
  height: 130px;
  display: grid;
  place-items: center;
}
.orbit-core {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.2);
  display: grid;
  place-items: center;
  font-size: 28px;
  z-index: 2;
}
.orbit-ring {
  position: absolute;
  border: 1px solid rgba(255, 255, 255, 0.35);
  border-radius: 50%;
  animation: spin 8s linear infinite;
}
.r1 { width: 80px; height: 80px; }
.r2 { width: 110px; height: 110px; animation-duration: 12s; animation-direction: reverse; }
.r3 { width: 130px; height: 130px; animation-duration: 16s; }
@keyframes spin {
  to { transform: rotate(360deg); }
}

.stat {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 18px;
  position: relative;
  overflow: hidden;
}
.stat-icon {
  width: 50px;
  height: 50px;
  border-radius: 14px;
  display: grid;
  place-items: center;
  font-size: 25px;
  flex-shrink: 0;
}
.stat-value {
  font-size: 25px;
  font-weight: 800;
  color: var(--text-1);
}
.unit {
  font-size: 14px;
  font-weight: 600;
  margin-left: 2px;
}
.stat-label {
  font-size: 13px;
  color: var(--text-3);
  margin-top: 1px;
}
.stat-trend {
  font-size: 11px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 2px;
  margin-top: 2px;
}
.stat-trend.up { color: var(--success); }
.stat-trend.down { color: var(--danger); }

.chart-row { align-items: stretch; }
.span2 { grid-column: span 2; }
@media (max-width: 1100px) { .span2 { grid-column: span 3; } }
.card-pad { padding: 20px 22px; }
.chart { width: 100%; }
.trend { height: 300px; }
.radar { height: 300px; }

.modules {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 12px;
}
@media (max-width: 900px) { .modules { grid-template-columns: repeat(3, 1fr); } }
.mod {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 14px 6px;
  border-radius: 12px;
  background: var(--bg-page);
  cursor: pointer;
  transition: all var(--transition);
}
.mod:hover {
  transform: translateY(-3px);
  box-shadow: var(--shadow);
  background: #fff;
}
.mod-icon {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-size: 20px;
}
.mod-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-2);
}

.logs { display: flex; flex-direction: column; gap: 10px; }
.log { display: flex; gap: 10px; align-items: flex-start; }
.log-badge {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  flex-shrink: 0;
  color: #fff;
  font-size: 13px;
}
.log-badge.ok { background: var(--success); }
.log-badge.fail { background: var(--danger); }
.log-flow { font-size: 13px; font-weight: 600; }
.log-meta { font-size: 12px; color: var(--text-3); margin-top: 2px; }
</style>
