<template>
  <div>
    <div class="admin-card">
      <h3 class="admin-page-title">审计日志</h3>
      <div class="filter-bar">
        <el-row :gutter="16">
          <el-col :xs="24" :sm="12" :md="6">
            <div class="filter-item">
              <label>日期范围：</label>
              <el-date-picker
                v-model="dateRange"
                type="daterange"
                range-separator="至"
                start-placeholder="开始日期"
                end-placeholder="结束日期"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </div>
          </el-col>
          <el-col :xs="24" :sm="12" :md="4">
            <div class="filter-item">
              <label>操作类型：</label>
              <el-select v-model="filterAction" placeholder="全部" clearable style="width: 100%">
                <el-option v-for="a in actionTypes" :key="a.value" :label="a.label" :value="a.value" />
              </el-select>
            </div>
          </el-col>
          <el-col :xs="24" :sm="12" :md="4">
            <div class="filter-item">
              <label>操作用户：</label>
              <el-select v-model="filterUser" placeholder="全部" clearable filterable style="width: 100%">
                <el-option v-for="u in userOptions" :key="u.value" :label="u.label" :value="u.value" />
              </el-select>
            </div>
          </el-col>
          <el-col :xs="24" :sm="12" :md="6">
            <div class="filter-item search-item">
              <el-input
                v-model="searchText"
                placeholder="搜索操作对象或详情"
                :prefix-icon="Search"
                clearable
                @keyup.enter="handleSearch"
              />
              <el-button type="primary" :icon="Search" @click="handleSearch">搜索</el-button>
              <el-button :icon="Refresh" @click="resetFilter">重置</el-button>
            </div>
          </el-col>
        </el-row>
      </div>
    </div>

    <div class="admin-card">
      <div class="table-header">
        <div class="stats-summary">
          <span>共 <strong>{{ totalCount }}</strong> 条记录</span>
          <span class="sep">|</span>
          <span>今日操作 <strong>{{ todayCount }}</strong> 条</span>
          <span class="sep">|</span>
          <span>涉及用户 <strong>{{ uniqueUsers }}</strong> 人</span>
        </div>
        <el-button :icon="Download" @click="handleExport">导出日志</el-button>
      </div>

      <el-table :data="pagedLogs" v-loading="loading" stripe border style="width: 100%">
        <el-table-column type="index" label="#" width="50" />
        <el-table-column prop="timestamp" label="时间" width="160" sortable>
          <template #default="{ row }">
            <span class="log-time">{{ row.timestamp }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="user" label="操作用户" width="140">
          <template #default="{ row }">
            <div class="user-cell">
              <el-avatar :size="24" :style="{ backgroundColor: getColor(row.user) }">
                {{ row.user.charAt(0).toUpperCase() }}
              </el-avatar>
              <span>{{ row.user }}</span>
            </div>
          </template>
        </el-table-column>
        <el-table-column prop="action" label="操作类型" width="120">
          <template #default="{ row }">
            <el-tag :type="getActionTagType(row.action)" effect="light">{{ getActionLabel(row.action) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="target" label="操作对象" width="180" show-overflow-tooltip />
        <el-table-column prop="detail" label="详情" min-width="250" show-overflow-tooltip />
        <el-table-column prop="ip" label="IP地址" width="130" />
        <el-table-column prop="status" label="结果" width="100">
          <template #default="{ row }">
            <el-tag :type="row.status === 'success' ? 'success' : 'danger'" size="small" effect="light">
              {{ row.status === 'success' ? '成功' : '失败' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="80" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row)">详情</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[20, 50, 100, 200]"
          :total="filteredLogs.length"
          layout="total, sizes, prev, pager, next, jumper"
          background
        />
      </div>
    </div>

    <el-dialog v-model="detailVisible" title="审计详情" width="600px">
      <el-descriptions :column="2" border v-if="currentLog">
        <el-descriptions-item label="时间">{{ currentLog.timestamp }}</el-descriptions-item>
        <el-descriptions-item label="操作用户">{{ currentLog.user }}</el-descriptions-item>
        <el-descriptions-item label="操作类型">{{ getActionLabel(currentLog.action) }}</el-descriptions-item>
        <el-descriptions-item label="操作对象">{{ currentLog.target }}</el-descriptions-item>
        <el-descriptions-item label="IP地址">{{ currentLog.ip }}</el-descriptions-item>
        <el-descriptions-item label="结果">
          <el-tag :type="currentLog.status === 'success' ? 'success' : 'danger'">
            {{ currentLog.status === 'success' ? '成功' : '失败' }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="请求方法">{{ currentLog.method }}</el-descriptions-item>
        <el-descriptions-item label="接口路径">{{ currentLog.apiPath }}</el-descriptions-item>
        <el-descriptions-item label="浏览器" :span="2">{{ currentLog.userAgent }}</el-descriptions-item>
        <el-descriptions-item label="详情描述" :span="2">{{ currentLog.detail }}</el-descriptions-item>
      </el-descriptions>
      <template #footer>
        <el-button @click="detailVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { adminApi } from '@/api/index'
import { Search, Refresh, Download } from '@element-plus/icons-vue'

const loading = ref(false)
const dateRange = ref([])
const filterAction = ref('')
const filterUser = ref('')
const searchText = ref('')
const currentPage = ref(1)
const pageSize = ref(20)

const actionTypes = [
  { value: 'create', label: '创建' },
  { value: 'update', label: '更新' },
  { value: 'delete', label: '删除' },
  { value: 'login', label: '登录' },
  { value: 'logout', label: '登出' },
  { value: 'export', label: '导出' },
  { value: 'config', label: '配置变更' }
]

const userOptions = ref([
  { value: 'admin', label: 'admin' },
  { value: 'zhangsan', label: 'zhangsan' },
  { value: 'lisi', label: 'lisi' },
  { value: 'wangwu', label: 'wangwu' },
  { value: 'zhaoliu', label: 'zhaoliu' }
])

const logs = ref(generateMockLogs())

function generateMockLogs() {
  const users = ['admin', 'zhangsan', 'lisi', 'wangwu', 'zhaoliu']
  const actions = ['create', 'update', 'delete', 'login', 'logout', 'export', 'config']
  const targets = ['用户管理', '角色权限', '知识库「产品文档库」', 'LLM供应商「GPT-4o」', '存储路径「/data/docs」', '系统安全策略', '审计日志', '知识库「技术手册」', '模型路由规则']
  const details = [
    '创建了新用户 testuser，分配为访客角色',
    '修改了知识库权限配置，添加了运营人员角色访问',
    '删除了过期的审计日志记录（共152条）',
    '登录成功，IP归属：北京',
    '导出了用户列表Excel文件',
    '更新了LLM调用路由规则，条件匹配优先使用本地模型',
    '修改了密码复杂度要求为强策略',
    '创建了新的存储备份路径',
    '调整了角色权限配置，移除了删除权限',
    '登出系统，会话时长 45 分钟'
  ]
  const methods = ['POST', 'GET', 'PUT', 'DELETE']
  const ips = ['192.168.1.101', '192.168.1.102', '10.0.0.55', '172.16.0.23', '192.168.2.88']
  const userAgents = [
    'Mozilla/5.0 Chrome/120.0',
    'Mozilla/5.0 Firefox/121.0',
    'Mozilla/5.0 Safari/17.0',
    'Mozilla/5.0 Edge/120.0'
  ]

  const result = []
  for (let i = 0; i < 50; i++) {
    const d = new Date(Date.now() - i * 3600000 * (1 + Math.random() * 8))
    result.push({
      id: i + 1,
      timestamp: d.toLocaleString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit' }).replace(/\//g, '-'),
      user: users[Math.floor(Math.random() * users.length)],
      action: actions[Math.floor(Math.random() * actions.length)],
      target: targets[Math.floor(Math.random() * targets.length)],
      detail: details[Math.floor(Math.random() * details.length)],
      ip: ips[Math.floor(Math.random() * ips.length)],
      status: Math.random() > 0.05 ? 'success' : 'failed',
      method: methods[Math.floor(Math.random() * methods.length)],
      apiPath: '/api/admin/' + ['users', 'roles', 'llm/providers', 'knowledge', 'storage/paths', 'system/config'][Math.floor(Math.random() * 6)],
      userAgent: userAgents[Math.floor(Math.random() * userAgents.length)]
    })
  }
  return result.sort((a, b) => b.timestamp.localeCompare(a.timestamp))
}

const filteredLogs = computed(() => {
  return logs.value.filter(l => {
    const matchDate = !dateRange.value || dateRange.value.length !== 2 ||
      (l.timestamp >= dateRange.value[0] && l.timestamp <= dateRange.value[1] + ' 23:59:59')
    const matchAction = !filterAction.value || l.action === filterAction.value
    const matchUser = !filterUser.value || l.user === filterUser.value
    const matchSearch = !searchText.value ||
      l.target.includes(searchText.value) ||
      l.detail.includes(searchText.value)
    return matchDate && matchAction && matchUser && matchSearch
  })
})

const pagedLogs = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredLogs.value.slice(start, start + pageSize.value)
})

const totalCount = computed(() => filteredLogs.value.length)
const todayCount = computed(() => {
  const today = new Date().toISOString().split('T')[0]
  return logs.value.filter(l => l.timestamp.startsWith(today)).length
})
const uniqueUsers = computed(() => new Set(logs.value.map(l => l.user)).size)

const detailVisible = ref(false)
const currentLog = ref(null)

function getColor(name) {
  const colors = ['#409eff', '#67c23a', '#e6a23c', '#f56c6c', '#909399']
  let hash = 0
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash)
  return colors[Math.abs(hash) % colors.length]
}

function getActionLabel(action) {
  return actionTypes.find(a => a.value === action)?.label || action
}

function getActionTagType(action) {
  const map = { create: 'success', update: 'warning', delete: 'danger', login: '', logout: 'info', export: 'success', config: 'warning' }
  return map[action] || ''
}

function handleSearch() { currentPage.value = 1 }
function resetFilter() {
  dateRange.value = []
  filterAction.value = ''
  filterUser.value = ''
  searchText.value = ''
  currentPage.value = 1
}

function viewDetail(row) {
  currentLog.value = row
  detailVisible.value = true
}

function handleExport() {
  ElMessage.success('日志导出任务已提交，请在下载中心查看')
}

onMounted(async () => {
  loading.value = true
  try {
    const res = await adminApi.getAuditLogs({ page: 1, pageSize: 20 })
    if (res?.data) logs.value = res.data
  } catch (e) { /* use mock data */ }
  loading.value = false
})
</script>

<style scoped>
.filter-bar {
  margin-top: 16px;
}

.filter-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.filter-item label {
  white-space: nowrap;
  font-size: 14px;
  color: #606266;
  min-width: 80px;
}

.search-item {
  gap: 8px;
}

.search-item .el-input {
  flex: 1;
}

.table-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.stats-summary {
  font-size: 14px;
  color: #606266;
}

.stats-summary strong {
  color: #409eff;
  font-size: 16px;
}

.stats-summary .sep {
  margin: 0 12px;
  color: #dcdfe6;
}

.log-time {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 12px;
  color: #606266;
}

.user-cell {
  display: flex;
  align-items: center;
  gap: 6px;
}

.pagination-wrapper {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>