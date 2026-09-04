<template>
  <div class="page-container logs-page">
    <!-- ===== 页头 ===== -->
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">在线日志</h2>
        <p class="page-subtitle">网关运行时日志 · 级别动态调整 · SSE 实时推送（tail -f）</p>
      </div>
      <div class="page-header-actions">
        <div class="live-chip" :class="{ on: streaming }">
          <span class="live-dot"></span>{{ streaming ? '实时推送中' : '已暂停' }}
        </div>
        <el-button :icon="Refresh" :loading="loading" @click="loadLogs">刷新</el-button>
        <el-button type="danger" plain :icon="Delete" @click="handleClear">清空</el-button>
      </div>
    </div>

    <!-- ===== 日志级别控制 ===== -->
    <div class="panel card-pad">
      <div class="toolbar">
        <div class="toolbar-left">
          <span class="level-label">全局级别</span>
          <el-select v-model="selectedLevel" style="width: 120px" @change="applyLevel">
            <el-option v-for="l in LEVELS" :key="l" :value="l" :label="l" />
          </el-select>
          <span class="level-hint">仅记录 ≥ 该级别（TRACE=0 … ERROR=4），写入后即时生效</span>
        </div>
        <div class="toolbar-right">
          <span class="badge info">缓冲 {{ buffered }} 条</span>
          <span class="badge" :class="effectiveLevelBadge">{{ effectiveLevel }} 生效</span>
        </div>
      </div>
    </div>

    <!-- ===== 查询筛选 ===== -->
    <div class="panel card-pad">
      <div class="toolbar">
        <div class="toolbar-left">
          <el-select v-model="queryLevel" placeholder="级别 ≥" clearable style="width: 130px" :disabled="streaming" @change="loadLogs">
            <el-option v-for="l in LEVELS" :key="l" :value="l" :label="`≥ ${l}`" />
          </el-select>
          <el-input
            v-model="search"
            placeholder="搜索消息 / 目标 / 级别"
            clearable
            :prefix-icon="Search"
            :disabled="streaming"
            style="width: 260px"
            @keyup.enter="loadLogs"
            @clear="loadLogs"
          />
          <el-select v-model="limit" style="width: 110px" :disabled="streaming" @change="loadLogs">
            <el-option :value="50" label="50 条" />
            <el-option :value="100" label="100 条" />
            <el-option :value="200" label="200 条" />
            <el-option :value="500" label="500 条" />
          </el-select>
          <el-button text type="primary" :disabled="streaming" @click="resetFilters">重置</el-button>
        </div>
        <div class="toolbar-right">
          <span v-if="streaming" class="stream-hint">实时推送为全量流，不受级别/关键词过滤</span>
          <el-switch v-model="autoScroll" active-text="自动滚动" />
          <el-switch v-model="streaming" active-text="实时推送" @change="onStreamChange" />
        </div>
      </div>
    </div>

    <!-- ===== 日志控制台（终端风格）===== -->
    <div class="panel log-console-wrap" v-loading="loading">
      <div ref="consoleEl" class="log-console" @scroll.passive="onConsoleScroll">
        <div v-for="e in displayLogs" :key="e.seq" class="log-line">
          <span class="l-seq">#{{ e.seq }}</span>
          <span class="l-ts">{{ fmtTs(e.ts) }}</span>
          <span class="l-level" :class="'lv-' + e.level.toLowerCase()">{{ padLevel(e.level) }}</span>
          <span class="l-target">{{ e.target }}</span>
          <span class="l-msg">{{ e.message }}</span>
        </div>
        <div v-if="!displayLogs.length" class="log-empty">
          {{ streaming ? '已连接，等待实时日志…' : '暂无日志 · 点击「刷新」或开启「实时推送」' }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Delete, Search } from '@element-plus/icons-vue'
import {
  getLoggers, setLoggerLevel, getOnlineLogs, clearOnlineLogs, openLogTail
} from '@/api'

// ===== 常量 =====
const LEVELS = ['TRACE', 'DEBUG', 'INFO', 'WARN', 'ERROR']
const MAX_ROWS = 1000 // 控制台最大行数（防内存膨胀）

// ===== 状态 =====
const loading = ref(false)
const logs = ref([])          // 展示列表（时间正序，新日志在底部）
const lastSeq = ref(0)        // 已展示的最大 seq，用于 SSE 去重
const buffered = ref(0)
const effectiveLevel = ref('INFO')
const selectedLevel = ref('INFO')

// 查询条件
const queryLevel = ref('')
const search = ref('')
const limit = ref(200)

// 实时推送
const streaming = ref(false)
const autoScroll = ref(true)
const manualPinned = ref(false) // 用户上滑后暂停自动滚底

// SSE 句柄
let controller = null
let sseReader = null
let sseBuffer = ''

const displayLogs = computed(() => logs.value)

// ===== 时间格式化（ts 为 ISO UTC，转本地 HH:mm:ss.SSS）=====
function fmtTs(ts) {
  if (!ts) return '--:--:--.---'
  const d = new Date(ts)
  if (isNaN(d.getTime())) return String(ts)
  const p = (n, w = 2) => String(n).padStart(w, '0')
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}.${p(d.getMilliseconds(), 3)}`
}
function padLevel(l) { return String(l || '').padEnd(5, ' ') }

const effectiveLevelBadge = computed(() => {
  const rank = LEVELS.indexOf(effectiveLevel.value)
  if (rank >= 3) return 'warning'
  if (rank === 2) return 'success'
  return 'info'
})

// ===== 日志查询 =====
async function loadLoggers() {
  try {
    const res = await getLoggers()
    effectiveLevel.value = res?.effective_level || res?.configured_level || 'INFO'
    selectedLevel.value = effectiveLevel.value
    buffered.value = res?.buffered ?? 0
  } catch (e) {
    ElMessage.error('日志级别加载失败: ' + (e?.message || e))
  }
}

async function loadLogs() {
  loading.value = true
  try {
    const params = { limit: limit.value }
    if (queryLevel.value) params.level = queryLevel.value
    if (search.value.trim()) params.search = search.value.trim()
    const res = await getOnlineLogs(params)
    // 后端返回最新在前；控制台按时间正序展示（新日志在底部，tail -f 风格）
    const list = Array.isArray(res?.logs) ? res.logs.slice().reverse() : []
    logs.value = list
    lastSeq.value = list.length ? Math.max(...list.map(e => Number(e.seq) || 0)) : 0
    buffered.value = res?.buffered ?? buffered.value
    effectiveLevel.value = res?.min_level || effectiveLevel.value
    scrollToBottom()
  } catch (e) {
    ElMessage.error('加载日志失败：' + e.message)
  } finally {
    loading.value = false
  }
}

function resetFilters() {
  queryLevel.value = ''
  search.value = ''
  loadLogs()
}

// ===== 级别调整 =====
async function applyLevel(level) {
  try {
    const res = await setLoggerLevel(level)
    effectiveLevel.value = res?.configured_level || level
    ElMessage.success(`日志级别已调整为 ${effectiveLevel.value}`)
    loadLoggers()
    loadLogs()
  } catch (e) {
    // 回退选择框
    selectedLevel.value = effectiveLevel.value
    ElMessage.error('级别调整失败：' + e.message)
  }
}

// ===== 清空 =====
async function handleClear() {
  try {
    await ElMessageBox.confirm(
      `确定清空网关日志缓冲（当前 ${buffered.value} 条）吗？该操作不可恢复。`,
      '清空日志',
      { type: 'warning', confirmButtonText: '清空', confirmButtonClass: 'el-button--danger' }
    )
  } catch { return }
  try {
    const res = await clearOnlineLogs()
    logs.value = []
    lastSeq.value = 0
    buffered.value = 0
    ElMessage.success(`已清空 ${res?.cleared ?? ''} 条日志`)
  } catch (e) {
    ElMessage.error('清空失败：' + e.message)
  }
}

// ===== SSE 实时推送 =====
function onStreamChange(val) {
  if (val) startStream()
  else stopStream()
}

async function startStream() {
  stopStream()
  controller = new AbortController()
  streaming.value = true
  try {
    const resp = await openLogTail({ limit: Math.min(limit.value, 200) })
    if (!resp.ok || !resp.body) throw new Error(`SSE 连接失败：HTTP ${resp.status}`)
    sseReader = resp.body.getReader()
    sseBuffer = ''
    const decoder = new TextDecoder('utf-8')
    const pump = async () => {
      try {
        for (;;) {
          const { done, value } = await sseReader.read()
          if (done) break
          sseBuffer += decoder.decode(value, { stream: true })
          let idx
          while ((idx = sseBuffer.indexOf('\n\n')) >= 0) {
            const block = sseBuffer.slice(0, idx)
            sseBuffer = sseBuffer.slice(idx + 2)
            handleSseBlock(block)
          }
        }
      } catch (e) {
        if (e?.name !== 'AbortError') throw e
      }
    }
    pump().catch((e) => {
      // dev only: SSE 流中断属预期行为（用户停止/页面切换），仅开发环境记录
      console.warn('[在线日志] SSE 中断:', e?.message)
      streaming.value = false
    })
  } catch (e) {
    streaming.value = false
    ElMessage.error('实时推送连接失败：' + e.message)
  }
}

function handleSseBlock(block) {
  block.split('\n').forEach((line) => {
    // 忽略 keep-alive 注释行（: …）
    if (!line.startsWith('data:')) return
    const payload = line.slice(5).trim()
    if (!payload) return
    try {
      const entry = JSON.parse(payload)
      appendLiveEntry(entry)
    } catch { /* 忽略非 JSON 行 */ }
  })
}

function appendLiveEntry(entry) {
  const seq = Number(entry.seq)
  if (!seq || seq <= lastSeq.value) return // 去重（回放与实时重叠）
  lastSeq.value = seq
  logs.value.push({
    seq: entry.seq,
    ts: entry.ts,
    level: entry.level,
    target: entry.target,
    message: entry.message
  })
  // 行数上限：丢弃最旧
  if (logs.value.length > MAX_ROWS) logs.value.splice(0, logs.value.length - MAX_ROWS)
  if (autoScroll.value && !manualPinned.value) scrollToBottom()
}

function stopStream() {
  if (controller) controller.abort()
  controller = null
  sseReader = null
}

// ===== 滚动 =====
function scrollToBottom() {
  nextTick(() => {
    const el = consoleEl.value
    if (el) el.scrollTop = el.scrollHeight
  })
}
function onConsoleScroll() {
  const el = consoleEl.value
  if (!el) return
  manualPinned.value = el.scrollHeight - el.scrollTop - el.clientHeight > 24
}

// ===== 生命周期 =====
onMounted(async () => {
  await loadLoggers()
  await loadLogs()
})

onBeforeUnmount(() => {
  stopStream()
})

const consoleEl = ref(null)
</script>

<style scoped>
.logs-page { padding-bottom: 32px; }

.page-header-actions { display: flex; align-items: center; gap: 10px; }

/* 实时状态芯片 */
.live-chip { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 600; color: var(--text-muted); }
.live-chip .live-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--text-muted); }
.live-chip.on { color: var(--success); }
.live-chip.on .live-dot { background: var(--success); box-shadow: 0 0 0 4px var(--success-50); animation: livePulse 1.6s ease infinite; }
@keyframes livePulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* 工具栏 */
.toolbar { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
.toolbar-left { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.toolbar-right { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.level-label { font-size: 13px; color: var(--text-secondary); font-weight: 600; }
.level-hint { font-size: 12px; color: var(--text-muted); }
.stream-hint { font-size: 12px; color: var(--warning); }

/* 日志控制台 */
.log-console-wrap { padding: 12px; }
.log-console {
  height: 560px;
  overflow: auto;
  background: #0b0d12;
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  padding: 10px 12px;
  font-family: Consolas, 'JetBrains Mono', Menlo, Monaco, monospace;
  font-size: 12.5px;
  line-height: 1.65;
}
.log-line { display: flex; gap: 10px; white-space: nowrap; }
.log-line:hover { background: rgba(99, 102, 241, 0.06); }
.l-seq { color: #475569; min-width: 58px; text-align: right; flex-shrink: 0; }
.l-ts { color: #64748b; flex-shrink: 0; }
.l-level { font-weight: 700; min-width: 46px; flex-shrink: 0; }
.l-target { color: #94a3b8; min-width: 90px; max-width: 140px; overflow: hidden; text-overflow: ellipsis; flex-shrink: 0; }
.l-msg { color: #cbd5e1; overflow: hidden; text-overflow: ellipsis; }

/* 级别配色 */
.lv-trace { color: #64748b; }
.lv-debug { color: #22d3ee; }
.lv-info { color: #34d399; }
.lv-warn { color: #fbbf24; }
.lv-error { color: #f87171; }

.log-empty { color: #475569; text-align: center; padding: 48px 0; font-family: inherit; }
</style>
