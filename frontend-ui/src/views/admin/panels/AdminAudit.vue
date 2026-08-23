<template>
  <div class="adm-audit">
    <div class="panel card-pad">
      <div class="filters">
        <el-input v-model="filters.action" placeholder="动作（如 api_key_created）" clearable style="width: 220px" />
        <el-input v-model="filters.actor" placeholder="操作者" clearable style="width: 160px" />
        <el-date-picker
          v-model="filters.since"
          type="datetime"
          placeholder="起始时间"
          style="width: 200px"
        />
        <el-select v-model="filters.limit" style="width: 120px">
          <el-option label="最近 100 条" :value="100" />
          <el-option label="最近 200 条" :value="200" />
          <el-option label="最近 500 条" :value="500" />
        </el-select>
        <el-button type="primary" :icon="Search" :loading="loading" @click="load">查询</el-button>
        <el-button :icon="Refresh" @click="reset">重置</el-button>
      </div>

      <el-table :data="logs" v-loading="loading" stripe style="width: 100%">
        <el-table-column type="expand">
          <template #default="{ row }">
            <pre class="detail-pre">{{ JSON.stringify(row.details, null, 2) }}</pre>
          </template>
        </el-table-column>
        <el-table-column prop="timestamp" label="时间" width="200">
          <template #default="{ row }">{{ fmtTime(row.timestamp) }}</template>
        </el-table-column>
        <el-table-column prop="action" label="动作" min-width="180">
          <template #default="{ row }">
            <span class="badge" :class="actionCls(row.action)">{{ row.action }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="actor" label="操作者" width="120" />
        <el-table-column label="详情" min-width="240">
          <template #default="{ row }">
            <span class="mono">{{ detailText(row.details) }}</span>
          </template>
        </el-table-column>
      </el-table>

      <el-empty v-if="!loading && !logs.length" description="暂无审计记录" />
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Search, Refresh } from '@element-plus/icons-vue'
import { getAuditLogs } from '@/api'

const loading = ref(false)
const logs = ref([])
const filters = reactive({ action: '', actor: '', since: null, limit: 100 })

function fmtTime(t) {
  if (!t) return '-'
  try { return new Date(t).toLocaleString() } catch { return String(t) }
}

function detailText(details) {
  if (!details) return '-'
  try { return JSON.stringify(details) } catch { return String(details) }
}

function actionCls(action) {
  if (!action) return 'info'
  if (action.includes('failed') || action.includes('exceeded') || action.includes('revoked')) return 'warning'
  if (action.includes('created')) return 'primary'
  return 'info'
}

async function load() {
  loading.value = true
  try {
    const params = { limit: filters.limit }
    if (filters.action) params.action = filters.action.trim()
    if (filters.actor) params.actor = filters.actor.trim()
    if (filters.since) params.since = new Date(filters.since).toISOString()
    const data = await getAuditLogs(params)
    logs.value = Array.isArray(data) ? data : []
  } catch (e) {
    ElMessage.error('加载审计日志失败：' + e.message)
  } finally {
    loading.value = false
  }
}

function reset() {
  filters.action = ''
  filters.actor = ''
  filters.since = null
  filters.limit = 100
  load()
}

onMounted(load)
</script>

<style scoped>
.filters { display: flex; gap: 10px; flex-wrap: wrap; margin-bottom: 14px; align-items: center; }
.mono {
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
  color: var(--text-2);
  word-break: break-all;
}
.detail-pre {
  background: var(--bg-panel-2);
  border-radius: 8px;
  padding: 12px 16px;
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
