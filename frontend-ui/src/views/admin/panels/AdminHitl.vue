<template>
  <div class="adm-hitl">
    <div class="grid grid-4 kpi-row">
      <div class="panel kpi">
        <div class="kpi-icon warn"><el-icon :size="22"><Clock /></el-icon></div>
        <div><div class="kpi-value">{{ pendingCount }}</div><div class="kpi-label">待审批</div></div>
      </div>
      <div class="panel kpi">
        <div class="kpi-icon ok"><el-icon :size="22"><CircleCheckFilled /></el-icon></div>
        <div><div class="kpi-value">{{ completedCount }}</div><div class="kpi-label">已处理</div></div>
      </div>
      <div class="panel kpi">
        <div class="kpi-icon info"><el-icon :size="22"><Bell /></el-icon></div>
        <div><div class="kpi-value">{{ events.length }}</div><div class="kpi-label">本会话事件</div></div>
      </div>
      <div class="panel kpi">
        <div class="kpi-icon" :class="wsConnected ? 'ok' : 'bad'"><el-icon :size="22"><Connection /></el-icon></div>
        <div><div class="kpi-value">{{ wsConnected ? '已连接' : '未连接' }}</div><div class="kpi-label">WebSocket</div></div>
      </div>
    </div>

    <div class="panel card-pad">
      <div class="toolbar">
        <div class="toolbar-left">
          <el-input
            v-model="searchText"
            placeholder="搜索事件ID / 流程 / 描述"
            :prefix-icon="Search"
            clearable
            style="width: 260px"
          />
        </div>
        <div class="toolbar-right">
          <span class="muted">审批通道：Rust 网关 /ws/hitl（默认 :3001，经 Vite /ws 代理）</span>
          <el-button :icon="RefreshRight" :loading="reconnecting" @click="reconnect">重连</el-button>
        </div>
      </div>

      <el-alert
        v-if="!wsConnected"
        type="info"
        :closable="false"
        title="HITL 通道未连接：请确认 Rust 网关已启动（platform/gateway，默认端口 3001）"
        style="margin-bottom: 12px"
      />

      <el-table :data="pagedEvents" v-loading="loading" stripe style="width: 100%">
        <el-table-column prop="id" label="事件ID" width="200" show-overflow-tooltip>
          <template #default="{ row }"><span class="mono">{{ row.id }}</span></template>
        </el-table-column>
        <el-table-column prop="flowName" label="流程" min-width="150" show-overflow-tooltip>
          <template #default="{ row }">{{ row.flowName || row.flowId || '-' }}</template>
        </el-table-column>
        <el-table-column prop="kind" label="类型" width="110" />
        <el-table-column prop="description" label="描述" min-width="220" show-overflow-tooltip />
        <el-table-column prop="requester" label="发起方" width="110" />
        <el-table-column label="触发时间" width="170">
          <template #default="{ row }">{{ fmtTs(row.ts) }}</template>
        </el-table-column>
        <el-table-column label="状态" width="120">
          <template #default="{ row }">
            <span class="badge" :class="statusCls(row.status)">{{ statusLabel(row.status) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="240" fixed="right">
          <template #default="{ row }">
            <template v-if="row.status === 'pending'">
              <el-button type="success" size="small" @click="handleApprove(row)">批准</el-button>
              <el-button type="danger" size="small" @click="handleDeny(row)">拒绝</el-button>
              <el-button type="warning" size="small" @click="openModify(row)">修改后批准</el-button>
            </template>
            <span v-else class="muted">-</span>
          </template>
        </el-table-column>
      </el-table>

      <div class="pager">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50]"
          :total="filteredEvents.length"
          layout="total, sizes, prev, pager, next"
          background
        />
      </div>
    </div>

    <el-dialog v-model="modifyVisible" title="修改后批准" width="640px">
      <div class="modify-desc">
        <div>事件ID：<span class="mono">{{ currentEvent?.id }}</span></div>
        <div>流程：{{ currentEvent?.flowName || currentEvent?.flowId || '-' }}</div>
        <div>描述：{{ currentEvent?.description }}</div>
      </div>
      <el-form label-width="100px">
        <el-form-item label="原始 Payload">
          <pre class="payload-pre">{{ originalPayloadText }}</pre>
        </el-form-item>
        <el-form-item label="修改后 Payload">
          <el-input
            v-model="modifiedPayloadText"
            type="textarea"
            :rows="10"
            placeholder='输入 JSON，例如：{"key": "value"}'
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="modifyVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submitModify">提交批准</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, RefreshRight, Clock, CircleCheckFilled, Bell, Connection } from '@element-plus/icons-vue'
import {
  hitlClient, HITL_ACTIONS,
  onHitlEvent, onHitlActionResult, onHitlPendingList, onHitlConnection
} from '@/utils/hitl-ws'

const searchText = ref('')
const currentPage = ref(1)
const pageSize = ref(10)
const events = ref([])
const wsConnected = ref(false)
const reconnecting = ref(false)
const loading = ref(true)

const modifyVisible = ref(false)
const currentEvent = ref(null)
const modifiedPayloadText = ref('')
const submitting = ref(false)

const pendingCount = computed(() => events.value.filter(e => e.status === 'pending').length)
const completedCount = computed(() => events.value.filter(e => e.status !== 'pending').length)

const filteredEvents = computed(() => {
  const q = searchText.value.trim()
  if (!q) return events.value
  return events.value.filter(e =>
    (e.id && e.id.includes(q)) ||
    (e.flowName && e.flowName.includes(q)) ||
    (e.flowId && e.flowId.includes(q)) ||
    (e.description && e.description.includes(q))
  )
})

const pagedEvents = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredEvents.value.slice(start, start + pageSize.value)
})

watch(filteredEvents, () => {
  const maxPage = Math.max(1, Math.ceil(filteredEvents.value.length / pageSize.value))
  if (currentPage.value > maxPage) currentPage.value = maxPage
})

const originalPayloadText = computed(() => {
  if (!currentEvent.value?.payload) return ''
  try { return JSON.stringify(currentEvent.value.payload, null, 2) } catch { return String(currentEvent.value.payload) }
})

function fmtTs(ts) {
  if (!ts) return '-'
  try { return new Date(ts * 1000).toLocaleString() } catch { return String(ts) }
}

function statusLabel(s) {
  const map = { pending: '待审批', approved: '已批准', denied: '已拒绝', modified_approved: '修改后批准' }
  return map[s] || s || '待审批'
}

function statusCls(s) {
  const map = { pending: 'warning', approved: 'success', denied: 'warning', modified_approved: 'primary' }
  return map[s] || 'info'
}

// 网关事件归一化（camelCase：id/flowId/flowName/kind/description/payload/requester/ts）
function normalizeEvent(evt) {
  if (!evt) return null
  const id = evt.id || evt.eventId || evt.event_id
  if (!id) return null
  return {
    id,
    flowId: evt.flowId || evt.flow_id || '',
    flowName: evt.flowName || evt.flow_name || '',
    kind: evt.kind || '',
    description: evt.description || evt.desc || '',
    payload: evt.payload ?? null,
    requester: evt.requester || '',
    ts: evt.ts || evt.triggeredAt || '',
    status: 'pending'
  }
}

function upsertEvent(evt) {
  const norm = normalizeEvent(evt)
  if (!norm) return
  const idx = events.value.findIndex(e => e.id === norm.id)
  if (idx === -1) {
    events.value.unshift(norm)
  } else {
    // 已有记录保留其审批状态
    events.value[idx] = { ...norm, status: events.value[idx].status }
  }
}

function handleHitlEvent(payload) {
  if (payload?.data) {
    upsertEvent(payload.data)
    ElMessage.info(`收到新的 HITL 事件：${payload.data.description || payload.data.id}`)
  }
}

// 网关动作结果：{type:'action_result', success, record:{event_id, action, ...}, error}
function handleActionResult(payload) {
  const rec = payload?.record
  if (!rec?.event_id) return
  const idx = events.value.findIndex(e => e.id === rec.event_id)
  if (idx === -1) return
  if (payload.success) {
    const statusMap = {
      [HITL_ACTIONS.APPROVE]: 'approved',
      [HITL_ACTIONS.DENY]: 'denied',
      [HITL_ACTIONS.MODIFY_APPROVE]: 'modified_approved'
    }
    events.value[idx].status = statusMap[rec.action] || 'approved'
    ElMessage.success(`事件已处理：${statusLabel(events.value[idx].status)}`)
  } else {
    events.value[idx].status = 'pending'
    ElMessage.error(`审批动作失败：${payload.error || '未知错误'}`)
  }
}

function handleConnection(payload) {
  wsConnected.value = payload?.status === 'connected'
  if (wsConnected.value) loading.value = false
}

function handlePendingList(payload) {
  if (Array.isArray(payload?.items)) {
    payload.items.forEach(upsertEvent)
  }
  loading.value = false
}

async function performAction(row, action, modifiedPayload = null) {
  const sent = hitlClient.sendAction(row.id, action, { modifiedPayload })
  if (!sent) {
    ElMessage.error('WebSocket 未连接，无法发送审批动作')
    return false
  }
  // 状态由 action_result 推送回填；这里先乐观标记处理中
  const idx = events.value.findIndex(e => e.id === row.id)
  if (idx !== -1) events.value[idx].status = 'processing'
  return true
}

async function handleApprove(row) {
  try {
    await ElMessageBox.confirm(`确定批准事件「${row.description || row.id}」吗？`, '批准确认', { type: 'success' })
    await performAction(row, HITL_ACTIONS.APPROVE)
  } catch { /* cancelled */ }
}

async function handleDeny(row) {
  try {
    await ElMessageBox.confirm(`确定拒绝事件「${row.description || row.id}」吗？`, '拒绝确认', { type: 'warning' })
    await performAction(row, HITL_ACTIONS.DENY)
  } catch { /* cancelled */ }
}

function openModify(row) {
  currentEvent.value = row
  try {
    modifiedPayloadText.value = row.payload ? JSON.stringify(row.payload, null, 2) : ''
  } catch {
    modifiedPayloadText.value = ''
  }
  modifyVisible.value = true
}

async function submitModify() {
  if (!currentEvent.value) return
  let parsed = null
  const text = modifiedPayloadText.value.trim()
  if (text) {
    try { parsed = JSON.parse(text) } catch {
      ElMessage.error('修改后的 Payload 必须是合法的 JSON')
      return
    }
  }
  submitting.value = true
  try {
    const ok = await performAction(currentEvent.value, HITL_ACTIONS.MODIFY_APPROVE, parsed)
    if (ok) modifyVisible.value = false
  } finally {
    submitting.value = false
  }
}

function reconnect() {
  reconnecting.value = true
  hitlClient.disconnect()
  hitlClient.connect()
  setTimeout(() => { reconnecting.value = false }, 1500)
}

let offEvent, offResult, offPending, offConn

onMounted(() => {
  offEvent = onHitlEvent(handleHitlEvent)
  offResult = onHitlActionResult(handleActionResult)
  offPending = onHitlPendingList(handlePendingList)
  offConn = onHitlConnection(handleConnection)
  hitlClient.connect()
  // 兜底：3 秒后无论是否连接成功都关闭 loading
  setTimeout(() => { loading.value = false }, 3000)
})

onBeforeUnmount(() => {
  offEvent?.()
  offResult?.()
  offPending?.()
  offConn?.()
  hitlClient.disconnect()
})
</script>

<style scoped>
.kpi-row { margin-bottom: 16px; }
.kpi { display: flex; align-items: center; gap: 14px; padding: 18px; }
.kpi-icon {
  width: 46px;
  height: 46px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
  background: var(--brand-soft);
  color: var(--brand);
}
.kpi-icon.ok { background: var(--success-50); color: var(--success); }
.kpi-icon.warn { background: var(--warning-50); color: var(--warning); }
.kpi-icon.info { background: var(--accent-50); color: #0e7490; }
.kpi-icon.bad { background: #fef2f2; color: var(--danger); }
.kpi-value { font-size: 20px; font-weight: 700; color: var(--text-1); line-height: 1.2; }
.kpi-label { font-size: 12px; color: var(--text-3); margin-top: 3px; }
.toolbar { display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px; flex-wrap: wrap; gap: 10px; }
.toolbar-right { display: flex; align-items: center; gap: 12px; }
.muted { font-size: 12px; color: var(--text-3); }
.mono { font-family: Consolas, Monaco, monospace; font-size: 12px; color: var(--text-2); }
.pager { margin-top: 14px; display: flex; justify-content: flex-end; }
.modify-desc {
  background: var(--bg-panel-2);
  border-radius: 8px;
  padding: 12px 16px;
  margin-bottom: 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 13px;
  color: var(--text-2);
}
.payload-pre {
  background: var(--bg-panel-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px;
  max-height: 240px;
  overflow: auto;
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
