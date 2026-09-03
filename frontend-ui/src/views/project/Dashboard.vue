<template>
  <div class="page-container dashboard">
    <!-- 欢迎横幅 -->
    <div class="welcome panel">
      <div class="welcome-left">
        <div class="eyebrow">{{ projectEyebrow }}</div>
        <h1 class="page-title">
          <span class="gradient-text">{{ projectName }}</span>
          <span class="page-sub-title">项目工作台</span>
        </h1>
        <p class="page-subtitle">
          {{ projectDesc }}
        </p>
        <div class="quick">
          <el-button type="primary" @click="go('/ai')">
            <el-icon><ChatDotRound /></el-icon> AI 对话
          </el-button>
          <el-button @click="go('/graph')">
            <el-icon><Share /></el-icon> 图谱探索
          </el-button>
          <el-button @click="go('/operators')">
            <el-icon><Cpu /></el-icon> 算子执行
          </el-button>
          <el-button @click="go('/workflow')">
            <el-icon><Operation /></el-icon> 工作流
          </el-button>
        </div>
      </div>
      <div class="welcome-right">
        <!-- AI 快速对话卡片 -->
        <div class="ai-quick-card">
          <div class="aiqc-header">
            <div class="aiqc-title">
              <div class="aiqc-avatar"><el-icon><MagicStick /></el-icon></div>
              <div>
                <div class="aiqc-name">AI 助手</div>
                <div class="aiqc-status">
                  <span class="aiqc-dot"></span>
                  在线 · 随时为您服务
                </div>
              </div>
            </div>
            <el-button size="small" text type="primary" @click="go('/ai')">完整对话 →</el-button>
          </div>
          <div class="aiqc-body">
            <div class="aiqc-msg ai-msg">
              <div class="msg-bubble">
                你好！我是你的 AI 助手 👋<br/>
                当前是「{{ projectName }}」项目，有什么可以帮你的吗？
              </div>
            </div>
          </div>
          <div class="aiqc-suggestions">
            <div
              v-for="(q, i) in aiQuickSuggestions"
              :key="i"
              class="aiqc-sug"
              @click="sendQuickMsg(q)"
            >
              <span class="aiqc-sug-icon">{{ q.icon }}</span>
              <span class="aiqc-sug-text">{{ q.title }}</span>
            </div>
          </div>
          <div class="aiqc-input">
            <el-input
              v-model="aiInput"
              placeholder="输入你的问题，回车发送…"
              @keyup.enter="sendAIMsg"
              clearable
            >
              <template #append>
                <el-button type="primary" :icon="Promotion" @click="sendAIMsg">发送</el-button>
              </template>
            </el-input>
          </div>
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

    <!-- 项目进度 -->
    <div class="panel card-pad progress-panel">
      <div class="section-head">
        <h3 class="section-title">项目进度</h3>
        <el-button size="small" text type="primary" @click="go('/tasks')">查看全部任务 →</el-button>
      </div>
      <div class="phase-progress">
        <div
          v-for="(p, i) in projectPhases"
          :key="p.key"
          class="phase-item"
          :class="{ active: p.status === 'active', done: p.status === 'done' }"
        >
          <div class="phase-step">
            <div class="phase-dot" :style="{ background: p.status === 'done' ? '#10b981' : p.status === 'active' ? p.color : '#cbd5e1' }">
              <el-icon v-if="p.status === 'done'"><Select /></el-icon>
              <span v-else>{{ i + 1 }}</span>
            </div>
            <div class="phase-line" v-if="i < projectPhases.length - 1" :class="p.status === 'done' ? 'done' : ''"></div>
          </div>
          <div class="phase-info">
            <div class="phase-name">{{ p.label }}</div>
            <div class="phase-desc">{{ p.desc }}</div>
            <div class="phase-bar-wrap">
              <div class="phase-bar-bg">
                <div class="phase-bar-fill" :style="{ width: p.progress + '%', background: p.color }"></div>
              </div>
              <span class="phase-pct">{{ p.progress }}%</span>
            </div>
          </div>
        </div>
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
import { ref, onMounted, onBeforeUnmount, nextTick, computed } from 'vue'
import { useRouter } from 'vue-router'
import { Select, CaretTop, CaretBottom, MagicStick, Cpu, Share, ChatDotRound, Operation, List, VideoPlay, TrendCharts, CloseBold, Promotion } from '@element-plus/icons-vue'
import * as echarts from '@/echarts'
import { APP_VERSION, NAV_MODULES } from '@/types'
import { getStatus, getLogs, getProjectPhaseProgress } from '@/api'
import { useProject } from '@/composables/projectContext.js'

const { currentProject } = useProject()
const router = useRouter()

// 项目上下文
const projectName = computed(() => currentProject.value?.name || '我的项目')
const projectEyebrow = computed(() => {
  const statusMap = { active: '进行中', planning: '规划中', done: '已完成', archived: '已归档' }
  const status = statusMap[currentProject.value?.status] || '进行中'
  return `${status} · 璇玑系统 v${APP_VERSION}`
})
const projectDesc = computed(() => {
  if (currentProject.value?.description) return currentProject.value.description
  return '以项目为根，AI 驱动的知识图谱与全维业务处理平台 — 需求 → 架构 → 开发 → 发布 全流程贯通'
})

// 项目阶段进度：优先使用后端数据，失败降级为演示占位
const phaseProgressData = ref(null)

async function loadPhaseProgress() {
  const pid = currentProject.value?.id
  if (!pid) return
  try {
    const data = await getProjectPhaseProgress(pid)
    if (data) phaseProgressData.value = data
  } catch (e) { /* 保留演示占位 */ }
}

const projectPhases = computed(() => {
  const phase = currentProject.value?.phase || 'requirement'
  const phases = [
    { key: 'requirement', label: '需求阶段', desc: '需求采集与分析', color: '#6366f1', progress: 0, status: 'pending' },
    { key: 'architecture', label: '架构阶段', desc: '知识图谱构建', color: '#06b6d4', progress: 0, status: 'pending' },
    { key: 'develop', label: '开发阶段', desc: '算子与工作流', color: '#10b981', progress: 0, status: 'pending' },
    { key: 'release', label: '发布阶段', desc: '监控与优化', color: '#f59e0b', progress: 0, status: 'pending' }
  ]
  // 根据当前阶段模拟进度
  const phaseOrder = ['requirement', 'architecture', 'develop', 'release']
  const curIdx = phaseOrder.indexOf(phase)
  return phases.map((p, i) => {
    // 优先使用后端返回的阶段进度
    if (phaseProgressData.value && phaseProgressData.value.phases) {
      const backendPhase = phaseProgressData.value.phases.find(ph => ph.key === p.key || ph.name === p.key)
      if (backendPhase) {
        return { ...p, progress: backendPhase.progress ?? 0, status: backendPhase.status || 'pending' }
      }
    }
    if (i < curIdx) {
      return { ...p, progress: 100, status: 'done' }
    } else if (i === curIdx) {
      const realProgress = phaseProgressData.value?.current_progress ?? 65
      return { ...p, progress: realProgress, status: 'active' }
    }
    return p
  })
})

// AI 快速对话
const aiInput = ref('')
const aiQuickSuggestions = computed(() => {
  const phase = currentProject.value?.phase || 'default'
  const suggestions = {
    requirement: [
      { icon: '📋', title: '帮我梳理需求', prompt: '请帮我系统梳理当前项目的需求，从功能需求、非功能需求、约束条件三个维度进行结构化分析。' },
      { icon: '🎯', title: '分析用户画像', prompt: '请帮我分析这个项目的目标用户群体，构建用户画像。' },
      { icon: '🏗️', title: '推荐技术架构', prompt: '基于当前项目的需求特点，请推荐一套合适的技术架构方案。' }
    ],
    architecture: [
      { icon: '🔗', title: '设计图谱Schema', prompt: '请帮我设计知识图谱的Schema，包括实体类型、关系类型和属性定义。' },
      { icon: '📐', title: '系统架构设计', prompt: '请设计一个完整的系统架构方案，包括分层架构、模块划分和数据流。' },
      { icon: '🧵', title: '工作流设计', prompt: '请帮我设计核心业务的工作流编排方案。' }
    ],
    develop: [
      { icon: '📊', title: '推荐算子', prompt: '请根据当前项目特点，推荐适合的图计算算子和数据处理算子。' },
      { icon: '🔗', title: '中心性分析', prompt: '请系统解释知识图谱的度中心性、介数中心性、紧密中心性三种指标。' },
      { icon: '🧵', title: '编排工作流', prompt: '请帮我编排一个完整的数据处理工作流。' }
    ],
    release: [
      { icon: '📈', title: '性能监控方案', prompt: '请设计系统的监控方案，包括应用性能、业务指标和基础设施监控。' },
      { icon: '🚀', title: '部署方案', prompt: '请设计生产环境的部署方案，包括容器化、CI/CD和灰度发布。' },
      { icon: '📝', title: '迭代规划', prompt: '基于当前项目完成情况，请规划下一阶段的迭代重点。' }
    ]
  }
  return suggestions[phase] || suggestions.requirement
})

function sendQuickMsg(q) {
  // 跳转到 AI 助手并带上预设问题
  router.push({ path: '/ai', query: { prompt: encodeURIComponent(q.prompt), from: 'dashboard' } })
}

function sendAIMsg() {
  const msg = aiInput.value.trim()
  if (!msg) return
  router.push({ path: '/ai', query: { prompt: encodeURIComponent(msg), from: 'dashboard' } })
  aiInput.value = ''
}

const stats = ref([
  { label: '算子总数', value: '—', unit: '', icon: 'Cpu', color: '#4f46e5', bg: 'rgba(99,102,241,.15)', up: true, trend: '8.2%' },
  { label: '知识节点', value: '—', unit: '', icon: 'Share', color: '#06b6d4', bg: 'rgba(6,182,212,.15)', up: true, trend: '3.1%' },
  { label: '执行次数', value: '—', unit: '', icon: 'VideoPlay', color: '#10b981', bg: 'rgba(16,185,129,.15)', up: true, trend: '12.5%' },
  { label: '成功率', value: '—', unit: '%', icon: 'TrendCharts', color: '#f59e0b', bg: 'rgba(245,158,11,.15)', up: true, trend: '0.6%' }
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
    let st = null
    let lg = null
    try {
      const results = await Promise.all([getStatus(), getLogs()])
      st = results[0]
      lg = results[1]
    } catch (apiErr) {
      console.warn('API请求失败，使用默认数据', apiErr)
      st = null
      lg = null
    }
    
    if (st && st.success !== undefined && st.data !== undefined) {
      st = st.data
    }
    if (lg && lg.success !== undefined && lg.data !== undefined) {
      lg = lg.data
    }
    
    window.__dash_status__ = st || {}
    // 演示占位：API 无数据时的兜底默认值
    stats.value[0].value = (st && st.operators_count) ?? 8
    stats.value[1].value = (st && st.graph && st.graph.nodes) ?? 23
    stats.value[2].value = (st && st.executions_count) ?? 15
    const sr = (st && st.success_rate) ?? 98.5
    stats.value[3].value = sr.toFixed ? sr.toFixed(1) : sr
    stats.value[3].up = sr >= 98
    stats.value[3].trend = sr >= 98 ? '+' + (sr - 98).toFixed(1) + '%' : (sr - 98).toFixed(1) + '%'
    
    const logsArr = Array.isArray(lg) ? lg : []
    logs.value = logsArr.length > 0 ? logsArr : generateMockLogs()
    
    if (trendEl.value && radarEl.value) {
      renderCharts()
    }
  } catch (e) {
    console.warn('仪表盘加载失败', e)
  }
}

// 演示占位：API 返回空时的兜底执行日志
function generateMockLogs() {
  const now = Date.now()
  const workflows = [
    ['需求采集', '归一化 IR', '双联盟十四维特派', '归一化裁决', '璇玑验证网关'],
    ['数据输入', '知识图谱算子', 'PageRank 计算', '社区发现'],
    ['浏览器自动化', '页面解析', '数据提取', '报告生成'],
    ['AI 对话', '意图识别', '算子匹配', '结果聚合'],
    ['工作流编排', '算子执行', '状态监控', '异常处理']
  ]
  return Array.from({ length: 15 }, (_, i) => ({
    timestamp: new Date(now - i * 300000).toISOString(),
    workflow: workflows[i % workflows.length],
    success: Math.random() > 0.1,
    execution_time_ms: 50 + Math.floor(Math.random() * 500),
    input_dim: 2 + Math.floor(Math.random() * 5),
    output_dim: 5 + Math.floor(Math.random() * 10)
  }))
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
    legend: { data: ['执行量', '成功率'], right: 10, top: 0, textStyle: { color: '#9aa0b4' } },
    grid: { left: 36, right: 16, top: 36, bottom: 28 },
    xAxis: { type: 'category', data: times, axisLine: { lineStyle: { color: '#3a3f5a' } }, axisLabel: { color: '#94a3b8' } },
    yAxis: [
      { type: 'value', splitLine: { lineStyle: { color: '#2d3148' } }, axisLabel: { color: '#94a3b8' } },
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
      axisName: { color: '#9aa0b4', fontSize: 11 },
      splitLine: { lineStyle: { color: '#3a3f5a' } },
      splitArea: { areaStyle: { color: ['#1e2130', '#242838'] } }
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
  loadPhaseProgress()
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
.page-sub-title {
  font-size: 18px;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.85);
  margin-left: 10px;
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

/* AI 快速对话卡片 */
.ai-quick-card {
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.25);
  border-radius: 16px;
  padding: 16px;
  width: 340px;
  color: #fff;
}
.aiqc-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}
.aiqc-title {
  display: flex;
  align-items: center;
  gap: 10px;
}
.aiqc-avatar {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  background: linear-gradient(135deg, #ec4899, #8b5cf6);
  display: grid;
  place-items: center;
  font-size: 20px;
  color: #fff;
}
.aiqc-name {
  font-weight: 700;
  font-size: 14px;
}
.aiqc-status {
  font-size: 11px;
  opacity: 0.8;
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 2px;
}
.aiqc-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #34d399;
  box-shadow: 0 0 6px #34d399;
  animation: pulse 2s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
.aiqc-header .el-button {
  color: rgba(255, 255, 255, 0.9);
}
.aiqc-header .el-button:hover {
  color: #fff;
}
.aiqc-body {
  margin-bottom: 12px;
}
.aiqc-msg {
  display: flex;
}
.aiqc-msg.ai-msg {
  justify-content: flex-start;
}
.msg-bubble {
  max-width: 100%;
  padding: 10px 14px;
  border-radius: 12px 12px 12px 4px;
  background: rgba(255, 255, 255, 0.2);
  font-size: 13px;
  line-height: 1.6;
}
.aiqc-suggestions {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}
.aiqc-sug {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 8px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
}
.aiqc-sug:hover {
  background: rgba(255, 255, 255, 0.2);
  border-color: rgba(255, 255, 255, 0.3);
  transform: translateX(2px);
}
.aiqc-sug-icon {
  font-size: 14px;
}
.aiqc-sug-text {
  flex: 1;
}
.aiqc-input :deep(.el-input__wrapper) {
  background: rgba(255, 255, 255, 0.15);
  border-radius: 10px;
  box-shadow: none;
}
.aiqc-input :deep(.el-input__inner) {
  color: #fff;
}
.aiqc-input :deep(.el-input__inner::placeholder) {
  color: rgba(255, 255, 255, 0.5);
}
.aiqc-input :deep(.el-input-group__append) {
  background: transparent;
  border: none;
  padding: 0 4px 0 0;
}
.aiqc-input :deep(.el-button) {
  border-radius: 8px;
  background: linear-gradient(135deg, #ec4899, #8b5cf6);
  border: none;
  color: #fff;
}
.aiqc-input :deep(.el-button:hover) {
  opacity: 0.9;
}

/* 响应式：小屏隐藏AI卡片 */
@media (max-width: 900px) {
  .welcome-right { display: none; }
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

/* 项目进度 */
.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 14px;
}
.progress-panel {
  margin-bottom: 0;
}
.phase-progress {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
}
@media (max-width: 900px) {
  .phase-progress { grid-template-columns: repeat(2, 1fr); }
}
.phase-item {
  display: flex;
  gap: 12px;
}
.phase-step {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex-shrink: 0;
}
.phase-dot {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  color: #fff;
  font-weight: 700;
  font-size: 13px;
}
.phase-line {
  width: 2px;
  flex: 1;
  min-height: 20px;
  background: var(--border);
  margin-top: 4px;
}
.phase-line.done {
  background: #10b981;
}
.phase-info {
  flex: 1;
  min-width: 0;
}
.phase-name {
  font-weight: 700;
  font-size: 14px;
  color: var(--text-1);
}
.phase-desc {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 2px;
}
.phase-bar-wrap {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}
.phase-bar-bg {
  flex: 1;
  height: 6px;
  background: var(--bg-tertiary);
  border-radius: 3px;
  overflow: hidden;
}
.phase-bar-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.6s ease;
}
.phase-pct {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-2);
  min-width: 32px;
  text-align: right;
}

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
  background: var(--bg-hover);
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
