<template>
  <el-dialog
    :model-value="modelValue"
    width="880px"
    top="4vh"
    class="fdd-dialog"
    append-to-body
    destroy-on-close
    :close-on-click-modal="false"
    @update:model-value="(v) => emit('update:modelValue', v)"
    @open="onOpen"
  >
    <!-- 企业级头部：左侧名称/状态/描述，最右侧快速操作（查看视频 · 查看日志） -->
    <template #header>
      <div class="fdd-header">
        <div class="fdd-titles">
          <div class="fdd-name-row">
            <span class="fdd-name">{{ flowDetail?.name || flowDetail?.id || '流程详情' }}</span>
            <span class="fdd-status" :class="statusInfo.cls"><i class="fdd-dot" />{{ statusInfo.text }}</span>
          </div>
          <div class="fdd-desc">{{ flowDetail?.description || '暂无流程描述' }}</div>
        </div>
        <div class="fdd-actions">
          <el-tooltip content="查看执行录屏视频" placement="top">
            <el-button class="fdd-btn video" :disabled="videoLoading" @click="openPanel('video')">
              <el-icon><VideoCamera /></el-icon> 查看视频
            </el-button>
          </el-tooltip>
          <el-tooltip content="查看执行日志" placement="top">
            <el-button class="fdd-btn log" :disabled="logLoading" @click="openPanel('log')">
              <el-icon><Document /></el-icon> 查看日志
            </el-button>
          </el-tooltip>
        </div>
      </div>
    </template>

    <!-- 概览信息条 -->
    <div class="fdd-overview">
      <div v-for="it in overview" :key="it.label" class="ov-item">
        <div class="ov-label">{{ it.label }}</div>
        <div class="ov-value" :class="{ 'ov-mono': it.mono }">
          <template v-if="it.copy">
            <span class="ov-copy" :title="'复制 ' + it.value" @click="copy(it.value)">{{ it.value }}</span>
            <el-icon class="ov-copy-ic" @click="copy(it.value)"><CopyDocument /></el-icon>
          </template>
          <template v-else>{{ it.value }}</template>
        </div>
      </div>
    </div>

    <!-- 节点 / 连线 -->
    <div class="fdd-body">
      <div class="fdd-col">
        <div class="fdd-col-title">
          <span>节点列表</span>
          <span class="fdd-col-count">{{ nodes.length }}</span>
        </div>
        <div v-if="nodes.length" class="fdd-nodes">
          <div v-for="(n, i) in nodes" :key="n.id || i" class="fdd-node">
            <span class="nd-idx">{{ String(i + 1).padStart(2, '0') }}</span>
            <span class="nd-type" :class="typeCls(n)">{{ typeText(n) }}</span>
            <span class="nd-name" :title="n.name">{{ n.name || n.id }}</span>
            <span v-if="n.tool" class="nd-tool mono">{{ n.tool }}</span>
            <span v-if="st(n)" class="nd-st" :class="st(n).cls"><i class="fdd-dot" />{{ st(n).text }}</span>
          </div>
        </div>
        <el-empty v-else description="暂无节点" :image-size="56" />
      </div>

      <div class="fdd-col">
        <div class="fdd-col-title">
          <span>连线拓扑</span>
          <span class="fdd-col-count">{{ edges.length }}</span>
        </div>
        <div v-if="edges.length" class="fdd-edges">
          <div v-for="(e, i) in edges" :key="e.id || i" class="fdd-edge">
            <span class="eg-badge">{{ i + 1 }}</span>
            <span class="eg-path mono">{{ e.from }} → {{ e.to }}</span>
            <span v-if="e.condition" class="eg-cond mono" :title="e.condition">if</span>
          </div>
        </div>
        <el-empty v-else description="暂无连线" :image-size="56" />
      </div>
    </div>

    <!-- 日志面板 -->
    <el-dialog v-model="logVisible" title="执行日志" width="780px" class="fdd-sub-dialog" append-to-body>
      <div class="log-toolbar">
        <span class="log-meta">{{ logMeta }}</span>
        <div class="log-tools">
          <el-button size="small" text :disabled="!logText" @click="copy(logText)">
            <el-icon><CopyDocument /></el-icon> 复制
          </el-button>
          <el-button size="small" text :loading="logLoading" @click="reloadLog">
            <el-icon><RefreshRight /></el-icon> 刷新
          </el-button>
        </div>
      </div>
      <div v-if="logLoading" class="log-loading">
        <el-icon class="is-loading"><Loading /></el-icon> 正在加载日志…
      </div>
      <pre v-else-if="logText" class="log-pre">{{ logText }}</pre>
      <el-empty v-else description="暂无该流程的执行日志" :image-size="60" />
    </el-dialog>

    <!-- 视频面板 -->
    <el-dialog v-model="videoVisible" title="执行录屏" width="860px" class="fdd-sub-dialog" append-to-body>
      <template v-if="videoUrl">
        <div class="video-box">
          <video :src="videoUrl" controls muted autoplay class="video-player" />
          <div class="video-meta mono">{{ videoUrl }}</div>
        </div>
      </template>
      <el-empty v-else description="暂无执行录屏视频" :image-size="70">
        <p class="video-tip">流程执行完成后，录屏回放会显示在这里</p>
      </el-empty>
    </el-dialog>
  </el-dialog>
</template>

<script setup>
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { VideoCamera, Document, CopyDocument, RefreshRight, Loading } from '@element-plus/icons-vue'
import { getLogs } from '@/api'

const props = defineProps({
  modelValue: { type: Boolean, default: false },
  flowDetail: { type: Object, default: null },
  /** 打开时直接定位到指定面板：'' | 'video' | 'log' */
  initialPanel: { type: String, default: '' }
})
const emit = defineEmits(['update:modelValue'])

const logVisible = ref(false)
const videoVisible = ref(false)
const logText = ref('')
const logLoading = ref(false)
const logSource = ref('')
const videoUrl = ref('')
const videoLoading = ref(false)

/* ===== 状态 ===== */
const STATUS_MAP = {
  completed: ['success', '已完成'],
  success: ['success', '已完成'],
  failed: ['danger', '执行失败'],
  error: ['danger', '执行失败'],
  running: ['warning', '执行中'],
  executing: ['warning', '执行中'],
  pending: ['info', '待执行'],
  draft: ['info', '草稿'],
  stopped: ['info', '已停止']
}
const statusInfo = computed(() => {
  const f = props.flowDetail || {}
  let raw = f.status || ''
  if (!raw && f.result) raw = f.result.success ? 'success' : 'failed'
  if (!raw && f.execution) raw = f.execution.status || ''
  const hit = STATUS_MAP[String(raw).toLowerCase()] || STATUS_MAP.draft
  return { text: hit[1], cls: hit[0] }
})

/* ===== 概览 ===== */
const nodes = computed(() => {
  const f = props.flowDetail || {}
  return (f.nodes || []).map((n) => ({ ...n, kind: n.kind || n.node_type || n.type || 'task' }))
})
const edges = computed(() => {
  const f = props.flowDetail || {}
  return (f.edges || []).map((e) => ({
    ...e,
    from: e.from ?? e.source ?? '?',
    to: e.to ?? e.target ?? '?'
  }))
})
function fmtTime(s) {
  if (!s) return '—'
  const d = new Date(s)
  if (Number.isNaN(d.getTime())) return String(s)
  const p = (x) => String(x).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}
const overview = computed(() => {
  const f = props.flowDetail || {}
  return [
    { label: '流程 ID', value: f.id || '—', mono: true, copy: !!f.id },
    { label: '节点数', value: String(nodes.value.length) },
    { label: '连线数', value: String(edges.value.length) },
    { label: '更新时间', value: fmtTime(f.updated_at || f.created_at) },
    { label: '状态', value: statusInfo.value.text }
  ]
})

/* ===== 节点类型着色 ===== */
const TYPE_CLS = {
  start: 'green',
  end: 'gray',
  task: 'blue',
  operator: 'indigo',
  llm: 'violet',
  browser: 'cyan',
  http: 'orange',
  httprequest: 'orange',
  condition: 'amber',
  decision: 'amber',
  transform: 'teal',
  script: 'slate',
  parallel: 'pink',
  io: 'cyan',
  dataoutput: 'teal',
  datainput: 'teal',
  guard: 'amber',
  event: 'slate'
}
function typeText(n) {
  const t = n.kind || 'task'
  return { start: '开始', end: '结束', llm: 'AI', browser: '浏览器', http: 'HTTP', httprequest: 'HTTP', operator: '算子', condition: '条件', decision: '决策', transform: '转换', script: '脚本', parallel: '并行', datainput: '输入', dataoutput: '输出', io: 'IO', guard: '闸门', event: '事件' }[String(t).toLowerCase()] || t
}
function typeCls(n) {
  return 'nd-type--' + (TYPE_CLS[String(n.kind || 'task').toLowerCase()] || 'blue')
}

/* ===== 节点执行状态（若有执行结果） ===== */
const resultsMap = computed(() => {
  const f = props.flowDetail || {}
  const arr = f.node_results || f.result?.node_results || f.execution?.node_results || []
  const m = {}
  for (const r of arr) m[r.node_id || r.nodeId || r.node] = r
  return m
})
function st(n) {
  const r = resultsMap.value[n.id]
  if (!r) return null
  const s = String(r.status || '').toLowerCase()
  if (s.includes('success') || s === 'done' || s === 'ok') return { cls: 'success', text: '成功' }
  if (s.includes('fail') || s.includes('error')) return { cls: 'danger', text: '失败' }
  if (s.includes('run') || s === 'pending' || s === 'waiting') return { cls: 'warning', text: '执行中' }
  return { cls: 'info', text: s || '—' }
}

/* ===== 视频 ===== */
function pickFirst(v) {
  if (Array.isArray(v)) return v.find((x) => typeof x === 'string' && x.trim())
  return typeof v === 'string' && v.trim() ? v : ''
}
function resolveVideo() {
  const f = props.flowDetail || {}
  const keys = [
    'video', 'video_url', 'videoUrl', 'video_path', 'videoPath',
    'record_url', 'recordUrl', 'recording', 'replay_url', 'replayUrl',
    'screenshot_url', 'screenshotUrl'
  ]
  for (const k of keys) {
    const v = pickFirst(f[k])
    if (v) return v
  }
  for (const n of nodes.value) {
    for (const k of ['video', 'video_url', 'videoUrl', 'record_url']) {
      const v = pickFirst(n[k])
      if (v) return v
    }
  }
  return ''
}

/* ===== 日志 ===== */
function buildInlineLog() {
  const f = props.flowDetail || {}
  const direct = [
    f.log, f.logs, f.log_content, f.logContent, f.execution_log,
    f.executionLog, f.log_text, f.logText, f.trace
  ].find((x) => typeof x === 'string' && x.trim())
  if (direct) return direct
  const arr = f.node_results || f.result?.node_results || f.execution?.node_results || []
  if (!arr.length) return ''
  const lines = []
  lines.push(`[${fmtTime(f.updated_at || f.created_at)}] 流程「${f.name || f.id}」执行日志`)
  for (const r of arr) {
    const st2 = r.status || 'info'
    lines.push(
      `[${st2}] ${r.node_type || 'node'}「${r.node_name || r.node_id}」` +
        (r.duration_ms != null ? ` ${r.duration_ms}ms` : '') +
        (r.error ? ` → ${r.error}` : '')
    )
    if (r.output != null) lines.push('  output: ' + JSON.stringify(r.output).slice(0, 400))
  }
  return lines.join('\n')
}
async function fetchSystemLogs() {
  try {
    const r = await getLogs()
    const list = r?.logs || r?.data || (Array.isArray(r) ? r : [])
    const f = props.flowDetail || {}
    const keys = [f.id, f.name, f.title].filter(Boolean)
    const lines = []
    for (const it of list) {
      if (typeof it === 'string') {
        if (!keys.length || keys.some((k) => it.includes(k))) lines.push(it)
      } else {
        const row = [
          it.time || it.ts || it.timestamp,
          it.level || it.level_name || 'info',
          it.message || it.msg || it.text || JSON.stringify(it)
        ].filter(Boolean).join('  ')
        if (row && (!keys.length || keys.some((k) => row.includes(k)))) lines.push(row)
      }
    }
    return lines.join('\n')
  } catch {
    return ''
  }
}
async function loadLogs() {
  logLoading.value = true
  try {
    const inline = buildInlineLog()
    if (inline) {
      logText.value = inline
      logSource.value = '内嵌执行结果'
      return
    }
    const sys = await fetchSystemLogs()
    logText.value = sys
    logSource.value = '系统日志（自动过滤）'
  } finally {
    logLoading.value = false
  }
}
async function reloadLog() {
  logText.value = ''
  await loadLogs()
}
const logMeta = computed(() => {
  const f = props.flowDetail || {}
  return `流程「${f.name || f.id || '—'}」 · ${logSource.value || '—'}`
})

/* ===== 打开面板 ===== */
function openPanel(p) {
  if (p === 'video') {
    videoLoading.value = true
    videoUrl.value = resolveVideo()
    videoVisible.value = true
    videoLoading.value = false
  } else if (p === 'log') {
    logVisible.value = true
    loadLogs()
  }
}
function onOpen() {
  if (props.initialPanel === 'video' || props.initialPanel === 'log') openPanel(props.initialPanel)
}

/* ===== 复制 ===== */
async function copy(text) {
  if (!text) {
    ElMessage.info('暂无可复制内容')
    return
  }
  try {
    await navigator.clipboard.writeText(String(text))
    ElMessage.success('已复制')
  } catch {
    ElMessage.warning('复制失败，请手动选择复制')
  }
}
</script>

<style scoped>
.fdd-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding-right: 4px;
}
.fdd-titles {
  min-width: 0;
}
.fdd-name-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.fdd-name {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 480px;
}
.fdd-status {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
  white-space: nowrap;
  flex-shrink: 0;
}
.fdd-status.success { background: var(--success-50); color: #047857; }
.fdd-status.danger { background: #fef2f2; color: #b91c1c; }
.fdd-status.warning { background: var(--warning-50); color: #b45309; }
.fdd-status.info { background: var(--accent-dim); color: #4338ca; }
.fdd-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
  flex-shrink: 0;
}
.fdd-desc {
  margin-top: 4px;
  font-size: 13px;
  color: var(--text-3);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
/* 最右快速操作区 */
.fdd-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
  align-items: center;
}
.fdd-btn {
  border-radius: 8px;
  font-weight: 600;
}
.fdd-btn.video {
  --el-button-bg-color: #eef2ff;
  --el-button-border-color: #c7d2fe;
  --el-button-text-color: #4f46e5;
  --el-button-hover-bg-color: #e0e7ff;
  --el-button-hover-border-color: #a5b4fc;
  --el-button-hover-text-color: #4338ca;
  --el-button-active-bg-color: #e0e7ff;
  --el-button-active-border-color: #a5b4fc;
  --el-button-active-text-color: #4338ca;
}
.fdd-btn.log {
  --el-button-bg-color: #fff7ed;
  --el-button-border-color: #fed7aa;
  --el-button-text-color: #ea580c;
  --el-button-hover-bg-color: #ffedd5;
  --el-button-hover-border-color: #fdba74;
  --el-button-hover-text-color: #c2410c;
  --el-button-active-bg-color: #ffedd5;
  --el-button-active-border-color: #fdba74;
  --el-button-active-text-color: #c2410c;
}

/* 概览信息条 */
.fdd-overview {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 10px;
  background: var(--bg-panel-2);
  border: 1px solid var(--border-light);
  border-radius: 12px;
  padding: 12px 14px;
  margin-bottom: 14px;
}
.ov-label {
  font-size: 11px;
  color: var(--text-3);
  margin-bottom: 3px;
  letter-spacing: 0.4px;
}
.ov-value {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-1);
  display: flex;
  align-items: center;
  gap: 4px;
  white-space: nowrap;
  overflow: hidden;
}
.ov-mono {
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-weight: 500;
}
.ov-copy {
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ov-copy:hover {
  color: var(--brand);
}
.ov-copy-ic {
  font-size: 13px;
  color: var(--text-3);
  cursor: pointer;
  flex-shrink: 0;
}
.ov-copy-ic:hover {
  color: var(--brand);
}

/* 主体两栏 */
.fdd-body {
  display: grid;
  grid-template-columns: 1.4fr 1fr;
  gap: 14px;
  max-height: 420px;
}
.fdd-col {
  border: 1px solid var(--border-light);
  border-radius: 12px;
  background: var(--bg-panel-2);
  padding: 12px;
  display: flex;
  flex-direction: column;
  min-height: 200px;
  overflow: hidden;
}
.fdd-col-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 700;
  color: var(--text-1);
  margin-bottom: 10px;
}
.fdd-col-count {
  background: var(--brand-soft);
  color: var(--brand-dark);
  font-size: 11px;
  font-weight: 700;
  padding: 1px 8px;
  border-radius: 999px;
}
.fdd-nodes,
.fdd-edges {
  overflow-y: auto;
  padding-right: 4px;
  flex: 1;
}
.fdd-node {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 8px;
  border-radius: 8px;
  background: var(--bg-panel);
  margin-bottom: 6px;
  border: 1px solid var(--border-light);
  transition: border-color 0.18s, box-shadow 0.18s;
}
.fdd-node:hover {
  border-color: var(--border);
  box-shadow: var(--shadow-sm);
}
.nd-idx {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-3);
  font-family: 'JetBrains Mono', Consolas, monospace;
  flex-shrink: 0;
}
.nd-type {
  font-size: 11px;
  font-weight: 700;
  padding: 1px 8px;
  border-radius: 6px;
  flex-shrink: 0;
  line-height: 18px;
}
.nd-type--green { background: var(--success-50); color: #059669; }
.nd-type--gray { background: var(--bg-tertiary); color: var(--text-secondary); }
.nd-type--blue { background: #eff6ff; color: #2563eb; }
.nd-type--indigo { background: var(--accent-dim); color: #4f46e5; }
.nd-type--violet { background: #f5f3ff; color: #7c3aed; }
.nd-type--cyan { background: var(--accent-50); color: #0891b2; }
.nd-type--orange { background: #fff7ed; color: #ea580c; }
.nd-type--amber { background: var(--warning-50); color: #d97706; }
.nd-type--teal { background: #f0fdfa; color: #0d9488; }
.nd-type--slate { background: var(--bg-tertiary); color: var(--text-secondary); }
.nd-type--pink { background: #fdf2f8; color: #db2777; }
.nd-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-1);
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.nd-tool {
  font-size: 11px;
  color: var(--text-3);
  background: var(--bg-panel-2);
  padding: 1px 6px;
  border-radius: 5px;
  flex-shrink: 0;
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.nd-st {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  font-weight: 600;
  flex-shrink: 0;
}
.nd-st.success { color: #047857; }
.nd-st.danger { color: #b91c1c; }
.nd-st.warning { color: #b45309; }
.nd-st.info { color: #64748b; }

.fdd-edge {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 8px;
  background: var(--bg-panel);
  margin-bottom: 6px;
  border: 1px solid var(--border-light);
}
.eg-badge {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--brand-soft);
  color: var(--brand-dark);
  font-size: 10px;
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.eg-path {
  font-size: 12px;
  color: var(--text-2);
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.eg-cond {
  font-size: 10px;
  font-weight: 700;
  color: #b45309;
  background: var(--warning-50);
  padding: 1px 6px;
  border-radius: 5px;
  flex-shrink: 0;
}
.mono {
  font-family: 'JetBrains Mono', Consolas, monospace;
}

/* 日志面板 */
.log-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}
.log-meta {
  font-size: 12px;
  color: var(--text-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.log-tools {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}
.log-loading {
  display: flex;
  align-items: center;
  gap: 8px;
  justify-content: center;
  padding: 40px 0;
  color: var(--text-3);
  font-size: 13px;
}
.log-pre {
  background: #0b1020;
  color: #a5b4fc;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
  line-height: 1.7;
  padding: 14px 16px;
  border-radius: 10px;
  max-height: 420px;
  overflow: auto;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}

/* 视频面板 */
.video-box {
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid var(--border-light);
  background: #000;
}
.video-player {
  width: 100%;
  max-height: 480px;
  display: block;
  background: #000;
}
.video-meta {
  padding: 8px 12px;
  font-size: 11px;
  color: var(--text-3);
  background: var(--bg-panel-2);
  border-top: 1px solid var(--border-light);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.video-tip {
  font-size: 12px;
  color: var(--text-3);
  margin: 0;
}
</style>

<style>
/* 弹窗外壳（teleport 到 body，需全局选择器） */
.fdd-dialog {
  border-radius: 16px;
  overflow: hidden;
  box-shadow: var(--shadow-lg);
}
.fdd-dialog .el-dialog__header {
  padding: 16px 22px 14px;
  margin-right: 0;
  border-bottom: 1px solid var(--border-light);
  background: linear-gradient(180deg, #ffffff 0%, #f8fafc 100%);
}
.fdd-dialog .el-dialog__headerbtn {
  top: 16px;
}
.fdd-dialog .el-dialog__body {
  padding: 14px 22px 20px;
}
.fdd-sub-dialog {
  border-radius: 14px;
  overflow: hidden;
}
.fdd-sub-dialog .el-dialog__header {
  padding: 14px 20px 12px;
  margin-right: 0;
  border-bottom: 1px solid var(--border-light);
}
.fdd-sub-dialog .el-dialog__body {
  padding: 14px 20px 18px;
}
</style>
