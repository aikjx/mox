<template>
  <div>
    <div class="stats-row">
      <div class="stat-card stat-pending">
        <div class="stat-icon">
          <el-icon :size="28"><Clock /></el-icon>
        </div>
        <div class="stat-info">
          <div class="stat-value">{{ pendingCount }}</div>
          <div class="stat-label">待审批</div>
        </div>
      </div>
      <div class="stat-card stat-completed">
        <div class="stat-icon">
          <el-icon :size="28"><CircleCheck /></el-icon>
        </div>
        <div class="stat-info">
          <div class="stat-value">{{ completedCount }}</div>
          <div class="stat-label">已完成</div>
        </div>
      </div>
      <div class="stat-card stat-today">
        <div class="stat-icon">
          <el-icon :size="28"><Calendar /></el-icon>
        </div>
        <div class="stat-info">
          <div class="stat-value">{{ todayCount }}</div>
          <div class="stat-label">今日审批量</div>
        </div>
      </div>
      <div class="stat-card stat-ws" :class="{ connected: wsConnected }">
        <div class="stat-icon">
          <el-icon :size="28"><Connection /></el-icon>
        </div>
        <div class="stat-info">
          <div class="stat-value">{{ wsConnected ? '已连接' : '断开' }}</div>
          <div class="stat-label">WebSocket</div>
        </div>
      </div>
    </div>

    <div class="admin-card">
      <div class="admin-table-toolbar">
        <div class="toolbar-left">
          <el-input
            v-model="searchText"
            placeholder="搜索事件ID、描述或流程ID"
            :prefix-icon="Search"
            style="width: 260px"
            clearable
          />
          <el-select v-model="filterStatus" placeholder="状态" clearable style="width: 140px; margin-left: 10px">
            <el-option label="待审批" value="pending" />
            <el-option label="已批准" value="approved" />
            <el-option label="已拒绝" value="denied" />
            <el-option label="修改后批准" value="modified_approved" />
          </el-select>
          <el-button type="primary" :icon="Search" style="margin-left: 10px" @click="handleSearch">搜索</el-button>
          <el-button :icon="Refresh" @click="resetSearch">重置</el-button>
        </div>
        <div class="toolbar-right">
          <el-button :icon="RefreshRight" @click="reconnectWs" :loading="reconnecting">重连</el-button>
        </div>
      </div>

      <el-table :data="pagedEvents" v-loading="loading" stripe border style="width: 100%">
        <el-table-column type="index" label="#" width="50" />
        <el-table-column prop="eventId" label="事件ID" width="180">
          <template #default="{ row }">
            <span class="mono-cell">{{ row.eventId }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="processId" label="流程ID" width="160">
          <template #default="{ row }">
            <span class="mono-cell">{{ row.processId || '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="description" label="描述" min-width="240" show-overflow-tooltip />
        <el-table-column prop="triggeredAt" label="触发时间" width="170" />
        <el-table-column prop="status" label="状态" width="120">
          <template #default="{ row }">
            <el-tag :type="getStatusTagType(row.status)" effect="light">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="240" fixed="right">
          <template #default="{ row }">
            <template v-if="row.status === 'pending'">
              <el-button type="success" size="small" :icon="Select" @click="handleApprove(row)">批准</el-button>
              <el-button type="danger" size="small" :icon="CloseBold" @click="handleDeny(row)">拒绝</el-button>
              <el-button type="warning" size="small" :icon="Edit" @click="openModifyDialog(row)">修改后批准</el-button>
            </template>
            <span v-else class="muted-text">-</span>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="filteredEvents.length"
          layout="total, sizes, prev, pager, next, jumper"
          background
        />
      </div>
    </div>

    <el-dialog v-model="modifyDialogVisible" title="修改后批准" width="680px">
      <div class="modify-desc">
        <div>事件ID：<span class="mono-cell">{{ currentEvent?.eventId }}</span></div>
        <div>流程ID：<span class="mono-cell">{{ currentEvent?.processId || '-' }}</span></div>
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
        <el-button @click="modifyDialogVisible = false">取消</el-button>
        <el-button type="primary" :icon="Select" @click="handleSubmitModify" :loading="submitting">提交批准</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Refresh, Select, CloseBold, Edit, Clock, CircleCheck, Calendar, Connection, RefreshRight } from '@element-plus/icons-vue'
import { adminApi } from '@/api/index'
import { hitlClient, hitlActions, onHitlEvent, onHitlConnection, onHitlStatus } from '@/utils/hitl-ws'

const loading = ref(false)
const reconnecting = ref(false)
const submitting = ref(false)
const searchText = ref('')
const filterStatus = ref('')
const currentPage = ref(1)
const pageSize = ref(10)

const events = ref([])
const wsConnected = ref(false)

const pendingCount = computed(() => events.value.filter(e => e.status === 'pending').length)
const completedCount = computed(() => events.value.filter(e => e.status !== 'pending').length)
const todayCount = computed(() => {
  const today = new Date().toISOString().slice(0, 10)
  return events.value.filter(e => {
    if (!e.completedAt) return false
    return String(e.completedAt).slice(0, 10) === today
  }).length
})

const filteredEvents = computed(() => {
  return events.value.filter(e => {
    const matchSearch = !searchText.value ||
      (e.eventId && e.eventId.includes(searchText.value)) ||
      (e.description && e.description.includes(searchText.value)) ||
      (e.processId && e.processId.includes(searchText.value))
    const matchStatus = !filterStatus.value || e.status === filterStatus.value
    return matchSearch && matchStatus
  })
})

const pagedEvents = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredEvents.value.slice(start, start + pageSize.value)
})

watch(filteredEvents, () => {
  const maxPage = Math.max(1, Math.ceil(filteredEvents.value.length / pageSize.value))
  if (currentPage.value > maxPage) currentPage.value = maxPage
})

const modifyDialogVisible = ref(false)
const currentEvent = ref(null)
const modifiedPayloadText = ref('')

const originalPayloadText = computed(() => {
  if (!currentEvent.value?.payload) return ''
  try {
    return JSON.stringify(currentEvent.value.payload, null, 2)
  } catch {
    return String(currentEvent.value.payload)
  }
})

function getStatusLabel(status) {
  const map = {
    pending: '待审批',
    approved: '已批准',
    denied: '已拒绝',
    modified_approved: '修改后批准'
  }
  return map[status] || status
}

function getStatusTagType(status) {
  const map = {
    pending: 'warning',
    approved: 'success',
    denied: 'danger',
    modified_approved: ''
  }
  return map[status] || 'info'
}

function normalizeEvent(evt) {
  if (!evt) return null
  return {
    eventId: evt.eventId || evt.id || evt.event_id || '',
    processId: evt.processId || evt.process_id || '',
    description: evt.description || evt.desc || evt.message || '',
    triggeredAt: evt.triggeredAt || evt.triggered_at || evt.createdAt || new Date().toLocaleString(),
    status: evt.status || 'pending',
    payload: evt.payload || evt.originalPayload || evt.data || null,
    completedAt: evt.completedAt || evt.completed_at || null,
    raw: evt
  }
}

function upsertEvent(evt) {
  const norm = normalizeEvent(evt)
  if (!norm || !norm.eventId) return
  const idx = events.value.findIndex(e => e.eventId === norm.eventId)
  if (idx === -1) {
    events.value.unshift(norm)
  } else {
    events.value[idx] = { ...events.value[idx], ...norm }
  }
}

function handleHitlEvent(payload) {
  if (payload?.eventId || payload?.id) {
    upsertEvent(payload)
    ElMessage.info(`收到新的 HITL 事件：${payload.eventId || payload.id}`)
  }
}

function handleHitlStatus(payload) {
  if (payload?.eventId && payload?.status) {
    const idx = events.value.findIndex(e => e.eventId === payload.eventId)
    if (idx !== -1) {
      events.value[idx].status = payload.status
      events.value[idx].completedAt = payload.completedAt || new Date().toLocaleString()
    }
  }
}

function handleConnection(payload) {
  wsConnected.value = payload?.status === 'connected'
}

function handleSearch() { currentPage.value = 1 }
function resetSearch() {
  searchText.value = ''
  filterStatus.value = ''
  currentPage.value = 1
}

async function performAction(row, action, modifiedPayload = undefined) {
  try {
    await adminApi.submitHitlAction(row.eventId, action, modifiedPayload)
  } catch (e) { /* mock HTTP */ }

  hitlClient.sendAction(row.eventId, action, modifiedPayload)

  const statusMap = {
    [hitlActions.APPROVE]: 'approved',
    [hitlActions.DENY]: 'denied',
    [hitlActions.MODIFY_APPROVE]: 'modified_approved'
  }
  const idx = events.value.findIndex(e => e.eventId === row.eventId)
  if (idx !== -1) {
    events.value[idx].status = statusMap[action] || row.status
    events.value[idx].completedAt = new Date().toLocaleString()
  }
  ElMessage.success(`事件 ${row.eventId} 已${getStatusLabel(statusMap[action])}`)
}

async function handleApprove(row) {
  try {
    await ElMessageBox.confirm(`确定批准事件 ${row.eventId} 吗？`, '批准确认', { type: 'success' })
    await performAction(row, hitlActions.APPROVE)
  } catch (e) { /* cancelled */ }
}

async function handleDeny(row) {
  try {
    await ElMessageBox.confirm(`确定拒绝事件 ${row.eventId} 吗？`, '拒绝确认', { type: 'warning' })
    await performAction(row, hitlActions.DENY)
  } catch (e) { /* cancelled */ }
}

function openModifyDialog(row) {
  currentEvent.value = row
  try {
    modifiedPayloadText.value = row.payload ? JSON.stringify(row.payload, null, 2) : ''
  } catch {
    modifiedPayloadText.value = ''
  }
  modifyDialogVisible.value = true
}

async function handleSubmitModify() {
  if (!currentEvent.value) return
  let parsedPayload = null
  const text = modifiedPayloadText.value.trim()
  if (text) {
    try {
      parsedPayload = JSON.parse(text)
    } catch {
      ElMessage.error('修改后的 Payload 必须是合法的 JSON')
      return
    }
  }
  submitting.value = true
  try {
    await performAction(currentEvent.value, hitlActions.MODIFY_APPROVE, parsedPayload)
    modifyDialogVisible.value = false
  } finally {
    submitting.value = false
  }
}

async function reconnectWs() {
  reconnecting.value = true
  hitlClient.disconnect()
  hitlClient.connect()
  setTimeout(() => { reconnecting.value = false }, 1500)
}

let offHitlEvent = null
let offHitlStatus = null
let offConnection = null

onMounted(async () => {
  loading.value = true
  try {
    const data = await adminApi.getHitlPending()
    const list = data?.data || []
    if (Array.isArray(list)) {
      list.forEach(item => upsertEvent(item))
    }
  } catch (e) { /* use default empty list */ }
  loading.value = false

  offHitlEvent = onHitlEvent(handleHitlEvent)
  offHitlStatus = onHitlStatus(handleHitlStatus)
  offConnection = onHitlConnection(handleConnection)
  hitlClient.connect()
})

onBeforeUnmount(() => {
  if (offHitlEvent) offHitlEvent()
  if (offHitlStatus) offHitlStatus()
  if (offConnection) offConnection()
  hitlClient.disconnect()
})
</script>

<style scoped>
.stats-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 16px;
}

.stat-card {
  background: #fff;
  border-radius: 8px;
  padding: 20px;
  display: flex;
  align-items: center;
  gap: 16px;
  border: 1px solid #ebeef5;
  transition: transform 0.2s, box-shadow 0.2s;
}

.stat-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
}

.stat-pending .stat-icon { background: #e6a23c; }
.stat-completed .stat-icon { background: #67c23a; }
.stat-today .stat-icon { background: #409eff; }
.stat-ws .stat-icon { background: #f56c6c; }
.stat-ws.connected .stat-icon { background: #67c23a; }

.stat-info { flex: 1; }
.stat-value { font-size: 24px; font-weight: 600; color: #303133; line-height: 1.2; }
.stat-label { font-size: 13px; color: #909399; margin-top: 4px; }

.mono-cell {
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
  color: #606266;
}

.muted-text { color: #c0c4cc; }

.pagination-wrapper {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}

.modify-desc {
  background: #f5f7fa;
  border-radius: 6px;
  padding: 12px 16px;
  margin-bottom: 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 13px;
  color: #606266;
}

.payload-pre {
  background: #f5f7fa;
  border: 1px solid #ebeef5;
  border-radius: 4px;
  padding: 12px;
  max-height: 240px;
  overflow: auto;
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}

@media (max-width: 1200px) {
  .stats-row { grid-template-columns: repeat(2, 1fr); }
}
</style>
