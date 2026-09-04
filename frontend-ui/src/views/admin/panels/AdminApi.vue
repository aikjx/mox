<template>
  <div class="page-container api-page">
    <!-- ===== 页头 ===== -->
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">接口管理</h2>
        <p class="page-subtitle">网关全部 API 注册表 · 分层/域/状态筛选 · 按需启停（Spring Boot Actuator 风格管理面）</p>
      </div>
      <div class="page-header-actions">
        <span class="badge" :class="health.status === 'UP' ? 'success' : (health.status ? 'warning' : 'info')">
          网关 {{ health.status || '未知' }}
        </span>
        <span v-if="health.uptime_secs" class="uptime-hint">运行 {{ fmtUptime(health.uptime_secs) }}</span>
        <el-button :icon="Refresh" :loading="loading" @click="loadAll">刷新</el-button>
      </div>
    </div>

    <!-- ===== 统计条 ===== -->
    <div class="grid grid-4 stat-row">
      <div class="panel stat-card">
        <div class="stat-label">接口总数</div>
        <div class="stat-value accent">{{ stats.total }}</div>
      </div>
      <div class="panel stat-card">
        <div class="stat-label">已启用</div>
        <div class="stat-value success">{{ stats.enabled }}</div>
      </div>
      <div class="panel stat-card">
        <div class="stat-label">已停用</div>
        <div class="stat-value danger">{{ stats.disabled }}</div>
      </div>
      <div class="panel stat-card">
        <div class="stat-label">平均延迟</div>
        <div class="stat-value">
          <span class="mono">{{ fmtLatency(metrics.latency_avg_ms) }}</span>
          <span v-if="metrics.latency_avg_ms != null" class="stat-unit">ms</span>
        </div>
      </div>
    </div>

    <!-- ===== 筛选工具栏 ===== -->
    <div class="panel card-pad">
      <div class="toolbar">
        <div class="toolbar-left">
          <el-input
            v-model="filters.q"
            placeholder="搜索 ID / 路径 / 域 / 描述"
            clearable
            :prefix-icon="Search"
            style="width: 240px"
            @keyup.enter="onSearch"
            @clear="onSearch"
          />
          <el-select v-model="filters.layer" placeholder="分层" clearable style="width: 110px" @change="loadMappings">
            <el-option v-for="l in layerOptions" :key="l" :value="l" :label="l" />
          </el-select>
          <el-select v-model="filters.domain" placeholder="域" clearable style="width: 130px" @change="loadMappings">
            <el-option v-for="d in domainOptions" :key="d" :value="d" :label="d" />
          </el-select>
          <el-select v-model="filters.status" placeholder="状态" clearable style="width: 120px" @change="loadMappings">
            <el-option v-for="s in statusOptions" :key="s" :value="s" :label="s" />
          </el-select>
          <el-switch v-model="filters.only_enabled" active-text="仅启用" @change="loadMappings" />
          <el-button text type="primary" @click="resetFilters">重置</el-button>
        </div>
        <div class="toolbar-right">
          <span class="filtered-hint">匹配 {{ routes.length }} / {{ stats.total }} 条</span>
        </div>
      </div>
    </div>

    <!-- ===== 接口表格 ===== -->
    <div class="panel card-pad">
      <el-table
        :data="pagedRoutes"
        v-loading="loading"
        stripe
        style="width: 100%"
        :row-class-name="rowClassName"
      >
        <el-table-column prop="id" label="接口 ID" min-width="160" show-overflow-tooltip>
          <template #default="{ row }">
            <span class="route-id" :class="{ disabled: !row.enabled }">{{ row.id }}</span>
          </template>
        </el-table-column>
        <el-table-column label="方法" width="84" align="center">
          <template #default="{ row }">
            <el-tag :type="methodTagType(row.method)" size="small" effect="dark">{{ row.method }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="path" label="路径" min-width="220" show-overflow-tooltip>
          <template #default="{ row }">
            <code class="path-mono">{{ row.path }}</code>
          </template>
        </el-table-column>
        <el-table-column label="分层" width="76" align="center">
          <template #default="{ row }">
            <el-tag size="small" effect="plain">{{ row.layer }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="域" width="104">
          <template #default="{ row }">
            <el-tag :type="domainTagType(row.domain)" size="small" effect="plain">{{ row.domain }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="92" align="center">
          <template #default="{ row }">
            <span class="badge" :class="statusBadge(row.status)">{{ row.status }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="description" label="描述" min-width="200" show-overflow-tooltip />
        <el-table-column label="启用" width="92" align="center">
          <template #default="{ row }">
            <el-switch
              :model-value="row.enabled"
              :disabled="isManagementRoute(row)"
              :title="isManagementRoute(row) ? '管理面端点不允许停用（防自锁）' : (row.enabled ? '点击停用该接口' : '点击启用该接口')"
              :before-change="() => toggleEnabled(row)"
            />
          </template>
        </el-table-column>
        <el-table-column label="操作" width="90" fixed="right" align="center">
          <template #default="{ row }">
            <el-button size="small" link type="primary" :icon="View" @click="openDetail(row)">详情</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pager">
        <el-pagination
          background
          layout="total, sizes, prev, pager, next"
          :total="routes.length"
          :page-sizes="[10, 20, 50, 100]"
          v-model:current-page="page"
          v-model:page-size="pageSize"
        />
      </div>
    </div>

    <!-- ===== 接口详情 ===== -->
    <el-dialog v-model="detailVisible" :title="detail ? `接口详情 · ${detail.id}` : '接口详情'" width="640px">
      <el-descriptions v-if="detail" :column="2" border>
        <el-descriptions-item label="接口 ID" :span="2">
          <span class="route-id">{{ detail.id }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="方法">
          <el-tag :type="methodTagType(detail.method)" size="small" effect="dark">{{ detail.method }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="状态">
          <span class="badge" :class="statusBadge(detail.status)">{{ detail.status }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="路径" :span="2">
          <code class="path-mono">{{ detail.path }}</code>
        </el-descriptions-item>
        <el-descriptions-item label="分层">
          <el-tag size="small" effect="plain">{{ detail.layer }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="域">
          <el-tag :type="domainTagType(detail.domain)" size="small" effect="plain">{{ detail.domain }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="启用状态">
          <el-tag :type="detail.enabled ? 'success' : 'danger'" size="small">{{ detail.enabled ? '已启用' : '已停用' }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="管理面端点">
          <el-tag :type="isManagementRoute(detail) ? 'warning' : 'info'" size="small">
            {{ isManagementRoute(detail) ? '受保护（不可停用）' : '普通业务端点' }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="描述" :span="2">{{ detail.description }}</el-descriptions-item>
      </el-descriptions>
      <el-alert
        v-if="detail && isManagementRoute(detail)"
        type="warning"
        :closable="false"
        title="管理面端点禁止停用（防自锁）：/health、/metrics、/actuator/*"
        style="margin-top: 14px"
      />
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Search, View } from '@element-plus/icons-vue'
import {
  getApiMappings, getApiDetail, enableApi, disableApi,
  getActuatorMetrics, getActuatorHealth
} from '@/api'

// ===== 状态 =====
const loading = ref(false)
const routes = ref([])
const stats = reactive({ total: 0, enabled: 0, disabled: 0 })
const metrics = reactive({ latency_avg_ms: null, requests_total: 0 })
const health = reactive({ status: '', uptime_secs: 0 })

const filters = reactive({ q: '', layer: '', domain: '', status: '', only_enabled: false })
const layerOptions = ref([])
const domainOptions = ref([])
const statusOptions = ref([])

const page = ref(1)
const pageSize = ref(20)

const pagedRoutes = computed(() => {
  const start = (page.value - 1) * pageSize.value
  return routes.value.slice(start, start + pageSize.value)
})

// ===== 管理面端点判定（与后端 is_management 对齐：防自锁）=====
function isManagementRoute(row) {
  const p = (row && row.path) || ''
  return p === '/health' || p === '/metrics' || p.startsWith('/actuator')
}

// ===== 展示映射 =====
const METHOD_TAGS = { GET: 'success', POST: 'primary', PUT: 'warning', DELETE: 'danger', ANY: 'info' }
function methodTagType(m) { return METHOD_TAGS[m] || 'info' }
function domainTagType(d) {
  const map = { Actuator: 'info', KG: 'success', AI: 'primary', Alliance: 'warning', Security: 'danger', Proxy: 'info' }
  return map[d] || 'info'
}
function statusBadge(s) {
  if (s === 'ready') return 'success'
  return 'info'
}
function rowClassName({ row }) { return row.enabled ? '' : 'row-disabled' }

// ===== 加载 =====
async function loadHealth() {
  try {
    const res = await getActuatorHealth()
    health.status = res?.status || ''
    health.uptime_secs = res?.uptime_secs || 0
  } catch (e) {
    health.status = 'DOWN'
    ElMessage.error('健康检查加载失败: ' + (e?.message || e))
  }
}

async function loadMetrics() {
  try {
    const res = await getActuatorMetrics()
    const m = res?.measurements || {}
    metrics.latency_avg_ms = m.latency_avg_ms ?? null
    metrics.requests_total = m.requests_total ?? 0
  } catch (e) {
    ElMessage.error('指标加载失败: ' + (e?.message || e))
  }
}

// 无过滤拉取一次：构建筛选选项 + 全局统计
async function loadOptions() {
  try {
    const res = await getApiMappings()
    const ctx = res?.contexts?.['mox-gateway'] || {}
    const list = ctx.routes || []
    layerOptions.value = [...new Set(list.map(r => r.layer).filter(Boolean))].sort()
    domainOptions.value = [...new Set(list.map(r => r.domain).filter(Boolean))].sort()
    statusOptions.value = [...new Set(list.map(r => r.status).filter(Boolean))].sort()
    stats.total = res?.total ?? list.length
    stats.disabled = res?.disabled_total ?? 0
    stats.enabled = stats.total - stats.disabled
  } catch (e) {
    ElMessage.error('加载接口注册表失败：' + e.message)
  }
}

async function loadMappings() {
  loading.value = true
  try {
    const params = {}
    if (filters.q.trim()) params.q = filters.q.trim()
    if (filters.layer) params.layer = filters.layer
    if (filters.domain) params.domain = filters.domain
    if (filters.status) params.status = filters.status
    if (filters.only_enabled) params.only_enabled = '1'
    const res = await getApiMappings(params)
    const ctx = res?.contexts?.['mox-gateway'] || {}
    routes.value = ctx.routes || []
    stats.total = res?.total ?? routes.value.length
    stats.disabled = res?.disabled_total ?? 0
    stats.enabled = stats.total - stats.disabled
    page.value = 1
  } catch (e) {
    ElMessage.error('加载接口列表失败：' + e.message)
  } finally {
    loading.value = false
  }
}

function loadAll() {
  loadHealth()
  loadMetrics()
  loadOptions()
  loadMappings()
}

function onSearch() { page.value = 1; loadMappings() }
function resetFilters() {
  filters.q = ''
  filters.layer = ''
  filters.domain = ''
  filters.status = ''
  filters.only_enabled = false
  loadMappings()
}

// ===== 启停（企业级确认 + 后端结果回滚）=====
async function toggleEnabled(row) {
  const next = !row.enabled
  // 停用为高风险操作：二次确认
  if (!next) {
    try {
      await ElMessageBox.confirm(
        `停用接口「${row.id}」(${row.method} ${row.path}) 后，请求将立即返回 403（API_DISABLED）。确定停用吗？`,
        '停用确认',
        { type: 'warning', confirmButtonText: '停用', confirmButtonClass: 'el-button--danger' }
      )
    } catch {
      return false
    }
  }
  try {
    const res = next ? await enableApi(row.id) : await disableApi(row.id)
    if (res && res.ok === false) throw new Error(res.error || '操作被拒绝')
    row.enabled = next
    ElMessage.success(res?.message || (next ? `接口「${row.id}」已启用` : `接口「${row.id}」已停用`))
    // 刷新统计与列表（服务端重新计算 disabled_total）
    loadOptions()
    loadMappings()
    return true
  } catch (e) {
    ElMessage.error('操作失败：' + e.message)
    return false
  }
}

// ===== 详情 =====
const detailVisible = ref(false)
const detail = ref(null)
async function openDetail(row) {
  detailVisible.value = true
  detail.value = { ...row }
  try {
    const res = await getApiDetail(row.id)
    if (res && res.ok !== false) detail.value = { ...res }
  } catch (e) {
    ElMessage.warning('详情刷新失败：' + e.message)
  }
}

// ===== 工具 =====
function fmtUptime(secs) {
  const d = Math.floor(secs / 86400)
  const h = Math.floor((secs % 86400) / 3600)
  const m = Math.floor((secs % 3600) / 60)
  if (d > 0) return `${d}天 ${h}小时`
  if (h > 0) return `${h}小时 ${m}分`
  return `${m}分`
}
function fmtLatency(v) {
  if (v == null) return '--'
  return Number(v) % 1 === 0 ? String(v) : Number(v).toFixed(1)
}

onMounted(loadAll)
</script>

<style scoped>
.api-page { padding-bottom: 32px; }

.page-header-actions { display: flex; align-items: center; gap: 10px; }
.uptime-hint { font-size: 12px; color: var(--text-secondary); }

/* 统计条 */
.stat-card { padding: 14px 18px; display: flex; flex-direction: column; gap: 6px; }
.stat-label { font-size: 12px; color: var(--text-muted); }
.stat-value { font-size: 24px; font-weight: 700; line-height: 1.2; }
.stat-value.accent { color: var(--accent-light); }
.stat-value.success { color: var(--success); }
.stat-value.danger { color: var(--danger); }
.stat-unit { font-size: 13px; font-weight: 500; color: var(--text-muted); margin-left: 2px; }
.mono { font-family: Consolas, Monaco, monospace; }

/* 工具栏 */
.toolbar { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
.toolbar-left { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.toolbar-right { display: flex; align-items: center; gap: 8px; }
.filtered-hint { font-size: 12px; color: var(--text-muted); }

/* 表格 */
.route-id { font-weight: 600; color: var(--text-primary); font-family: Consolas, Monaco, monospace; font-size: 13px; }
.route-id.disabled { color: var(--text-muted); text-decoration: line-through; }
.path-mono { font-family: Consolas, Monaco, monospace; font-size: 12.5px; color: var(--accent-light); background: var(--bg-tertiary); padding: 2px 6px; border-radius: 4px; }
:deep(.row-disabled) { opacity: 0.55; }
.pager { display: flex; justify-content: flex-end; margin-top: 14px; }
</style>
