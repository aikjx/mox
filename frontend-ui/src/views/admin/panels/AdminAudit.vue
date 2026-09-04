<template>
  <div class="adm-audit">
    <!-- Tab 切换 -->
    <div class="audit-tabs">
      <div
        class="audit-tab"
        :class="{ active: activeTab === 'operlog' }"
        @click="switchTab('operlog')"
      >
        <el-icon :size="16"><Document /></el-icon>
        <span>操作日志</span>
      </div>
      <div
        class="audit-tab"
        :class="{ active: activeTab === 'logininfor' }"
        @click="switchTab('logininfor')"
      >
        <el-icon :size="16"><User /></el-icon>
        <span>登录日志</span>
      </div>
    </div>

    <!-- 操作日志 -->
    <div v-show="activeTab === 'operlog'" class="panel card-pad">
      <div class="filters">
        <el-input v-model="operFilters.title" placeholder="操作模块" clearable style="width: 160px" />
        <el-select v-model="operFilters.businessType" placeholder="操作类型" clearable style="width: 130px">
          <el-option label="新增" :value="1" />
          <el-option label="修改" :value="2" />
          <el-option label="删除" :value="3" />
          <el-option label="查询" :value="4" />
          <el-option label="导出" :value="5" />
          <el-option label="导入" :value="6" />
          <el-option label="其他" :value="99" />
        </el-select>
        <el-input v-model="operFilters.operName" placeholder="操作人员" clearable style="width: 130px" />
        <el-select v-model="operFilters.status" placeholder="状态" clearable style="width: 100px">
          <el-option label="成功" :value="1" />
          <el-option label="失败" :value="0" />
        </el-select>
        <el-date-picker
          v-model="operFilters.timeRange"
          type="datetimerange"
          range-separator="至"
          start-placeholder="开始时间"
          end-placeholder="结束时间"
          value-format="YYYY-MM-DD HH:mm:ss"
          style="width: 340px"
        />
        <el-button type="primary" :icon="Search" :loading="operLoading" @click="loadOperLog">查询</el-button>
        <el-button :icon="Refresh" @click="resetOperFilters">重置</el-button>
        <div class="filter-spacer" />
        <el-button type="success" :icon="Download" :loading="exportingOper" @click="handleExportOper">导出</el-button>
        <el-button type="danger" :icon="Delete" :loading="cleaningOper" @click="handleCleanOper">清空</el-button>
      </div>

      <el-table :data="operLogList" v-loading="operLoading" stripe style="width: 100%" max-height="520">
        <el-table-column prop="id" label="日志编号" width="100" align="center" />
        <el-table-column prop="title" label="操作模块" min-width="120" />
        <el-table-column prop="businessType" label="操作类型" width="90" align="center">
          <template #default="{ row }">
            <el-tag :type="businessTypeTag(row.businessType)" size="small">{{ businessTypeLabel(row.businessType) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="description" label="操作描述" min-width="180" show-overflow-tooltip />
        <el-table-column prop="operName" label="操作人员" width="100" />
        <el-table-column prop="deptName" label="部门" width="110" />
        <el-table-column prop="method" label="请求方式" width="90" align="center">
          <template #default="{ row }">
            <el-tag :type="methodTag(row.method)" size="small">{{ row.method }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="url" label="请求地址" min-width="200" show-overflow-tooltip>
          <template #default="{ row }">
            <span class="mono">{{ row.url }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="ip" label="请求IP" width="130" align="center">
          <template #default="{ row }">
            <span class="mono">{{ row.ip }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="location" label="操作地点" width="120" />
        <el-table-column prop="browser" label="浏览器" width="110" />
        <el-table-column prop="os" label="操作系统" width="110" />
        <el-table-column prop="status" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 1 ? 'success' : 'danger'" size="small">
              {{ row.status === 1 ? '成功' : '失败' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="msg" label="返回消息" min-width="140" show-overflow-tooltip />
        <el-table-column prop="operTime" label="操作时间" width="170">
          <template #default="{ row }">{{ fmtTime(row.operTime) }}</template>
        </el-table-column>
        <el-table-column prop="cost" label="耗时" width="80" align="center">
          <template #default="{ row }">
            <span class="mono">{{ row.cost }}ms</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="80" fixed="right" align="center">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openOperDetail(row)">详情</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-row">
        <el-pagination
          v-model:current-page="operPagination.pageNum"
          v-model:page-size="operPagination.pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="operPagination.total"
          layout="total, sizes, prev, pager, next, jumper"
          background
          @size-change="loadOperLog"
          @current-change="loadOperLog"
        />
      </div>

      <el-empty v-if="!operLoading && !operLogList.length" description="暂无操作日志" />
    </div>

    <!-- 登录日志 -->
    <div v-show="activeTab === 'logininfor'" class="panel card-pad">
      <div class="filters">
        <el-input v-model="loginFilters.userName" placeholder="用户名称" clearable style="width: 160px" />
        <el-input v-model="loginFilters.ipaddr" placeholder="IP地址" clearable style="width: 160px" />
        <el-select v-model="loginFilters.status" placeholder="状态" clearable style="width: 100px">
          <el-option label="成功" :value="1" />
          <el-option label="失败" :value="0" />
        </el-select>
        <el-date-picker
          v-model="loginFilters.timeRange"
          type="datetimerange"
          range-separator="至"
          start-placeholder="开始时间"
          end-placeholder="结束时间"
          value-format="YYYY-MM-DD HH:mm:ss"
          style="width: 340px"
        />
        <el-button type="primary" :icon="Search" :loading="loginLoading" @click="loadLoginLog">查询</el-button>
        <el-button :icon="Refresh" @click="resetLoginFilters">重置</el-button>
        <div class="filter-spacer" />
        <el-button type="success" :icon="Download" :loading="exportingLogin" @click="handleExportLogin">导出</el-button>
        <el-button type="danger" :icon="Delete" :loading="cleaningLogin" @click="handleCleanLogin">清空</el-button>
      </div>

      <el-table :data="loginLogList" v-loading="loginLoading" stripe style="width: 100%" max-height="520">
        <el-table-column prop="id" label="日志编号" width="100" align="center" />
        <el-table-column prop="userName" label="用户名称" width="130" />
        <el-table-column prop="loginType" label="登录类型" width="110" align="center">
          <template #default="{ row }">
            <el-tag :type="loginTypeTag(row.loginType)" size="small">{{ loginTypeLabel(row.loginType) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="ipaddr" label="登录IP" width="140" align="center">
          <template #default="{ row }">
            <span class="mono">{{ row.ipaddr }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="loginLocation" label="登录地点" width="130" />
        <el-table-column prop="browser" label="浏览器" width="120" />
        <el-table-column prop="os" label="操作系统" width="120" />
        <el-table-column prop="status" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === '1' || row.status === 1 ? 'success' : 'danger'" size="small">
              {{ row.status === '1' || row.status === 1 ? '成功' : '失败' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="msg" label="提示消息" min-width="160" show-overflow-tooltip />
        <el-table-column prop="loginTime" label="登录时间" width="170">
          <template #default="{ row }">{{ fmtTime(row.loginTime) }}</template>
        </el-table-column>
      </el-table>

      <div class="pagination-row">
        <el-pagination
          v-model:current-page="loginPagination.pageNum"
          v-model:page-size="loginPagination.pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="loginPagination.total"
          layout="total, sizes, prev, pager, next, jumper"
          background
          @size-change="loadLoginLog"
          @current-change="loadLoginLog"
        />
      </div>

      <el-empty v-if="!loginLoading && !loginLogList.length" description="暂无登录日志" />
    </div>

    <!-- 操作日志详情弹窗 -->
    <el-dialog v-model="operDetailVisible" title="操作日志详情" width="720px">
      <div v-if="currentOperLog" class="detail-container">
        <el-descriptions :column="2" border size="small">
          <el-descriptions-item label="日志编号">{{ currentOperLog.id }}</el-descriptions-item>
          <el-descriptions-item label="操作模块">{{ currentOperLog.title }}</el-descriptions-item>
          <el-descriptions-item label="操作类型">
            <el-tag :type="businessTypeTag(currentOperLog.businessType)" size="small">
              {{ businessTypeLabel(currentOperLog.businessType) }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="操作人员">{{ currentOperLog.operName }}</el-descriptions-item>
          <el-descriptions-item label="部门">{{ currentOperLog.deptName || '-' }}</el-descriptions-item>
          <el-descriptions-item label="操作时间">{{ fmtTime(currentOperLog.operTime) }}</el-descriptions-item>
          <el-descriptions-item label="请求方式">
            <el-tag :type="methodTag(currentOperLog.method)" size="small">{{ currentOperLog.method }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="请求地址">
            <span class="mono">{{ currentOperLog.url }}</span>
          </el-descriptions-item>
          <el-descriptions-item label="请求IP">
            <span class="mono">{{ currentOperLog.ip }}</span>
          </el-descriptions-item>
          <el-descriptions-item label="操作地点">{{ currentOperLog.location }}</el-descriptions-item>
          <el-descriptions-item label="浏览器">{{ currentOperLog.browser }}</el-descriptions-item>
          <el-descriptions-item label="操作系统">{{ currentOperLog.os }}</el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag :type="currentOperLog.status === 1 ? 'success' : 'danger'" size="small">
              {{ currentOperLog.status === 1 ? '成功' : '失败' }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="耗时">{{ currentOperLog.cost }}ms</el-descriptions-item>
        </el-descriptions>

        <div class="detail-section">
          <div class="detail-section-title">请求参数</div>
          <pre class="detail-pre">{{ formatJson(currentOperLog.operParam) }}</pre>
        </div>

        <div class="detail-section">
          <div class="detail-section-title">响应结果</div>
          <pre class="detail-pre">{{ formatJson(currentOperLog.jsonResult) }}</pre>
        </div>

        <div v-if="currentOperLog.status !== 1 && currentOperLog.errorMsg" class="detail-section">
          <div class="detail-section-title error">异常信息</div>
          <pre class="detail-pre error-pre">{{ currentOperLog.errorMsg }}</pre>
        </div>
      </div>
      <template #footer>
        <el-button @click="operDetailVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Refresh, Download, Delete, Document, User } from '@element-plus/icons-vue'
import {
  getOperLogList, cleanOperLog, exportOperLog,
  getLoginLogList, cleanLoginLog, exportLoginLog
} from '@/api'

const activeTab = ref('operlog')

function switchTab(tab) {
  activeTab.value = tab
  if (tab === 'operlog' && !operLogList.value.length) {
    loadOperLog()
  } else if (tab === 'logininfor' && !loginLogList.value.length) {
    loadLoginLog()
  }
}

function fmtTime(t) {
  if (!t) return '-'
  try { return new Date(t).toLocaleString() } catch { return String(t) }
}

function formatJson(val) {
  if (!val) return '-'
  try {
    if (typeof val === 'string') {
      return JSON.stringify(JSON.parse(val), null, 2)
    }
    return JSON.stringify(val, null, 2)
  } catch {
    return String(val)
  }
}

// ===== 操作日志 =====
const operLoading = ref(false)
const exportingOper = ref(false)
const cleaningOper = ref(false)
const operLogList = ref([])

const operFilters = reactive({
  title: '',
  businessType: null,
  operName: '',
  status: null,
  timeRange: []
})

const operPagination = reactive({
  pageNum: 1,
  pageSize: 10,
  total: 0
})

const operDetailVisible = ref(false)
const currentOperLog = ref(null)

function businessTypeLabel(type) {
  const map = { 1: '新增', 2: '修改', 3: '删除', 4: '查询', 5: '导出', 6: '导入', 99: '其他' }
  return map[type] || '未知'
}

function businessTypeTag(type) {
  const map = { 1: 'success', 2: 'warning', 3: 'danger', 4: 'info', 5: 'primary', 6: 'primary', 99: 'info' }
  return map[type] || 'info'
}

function methodTag(method) {
  const map = { GET: 'success', POST: 'primary', PUT: 'warning', DELETE: 'danger' }
  return map[method] || 'info'
}

async function loadOperLog() {
  operLoading.value = true
  try {
    const params = {
      pageNum: operPagination.pageNum,
      pageSize: operPagination.pageSize,
      title: operFilters.title.trim(),
      businessType: operFilters.businessType,
      operName: operFilters.operName.trim(),
      status: operFilters.status
    }
    if (operFilters.timeRange?.length === 2) {
      params.beginTime = operFilters.timeRange[0]
      params.endTime = operFilters.timeRange[1]
    }
    const data = await getOperLogList(params)
    if (data && Array.isArray(data.rows || data.list)) {
      operLogList.value = data.rows || data.list
      operPagination.total = data.total || 0
    } else if (Array.isArray(data)) {
      operLogList.value = data
      operPagination.total = data.length
    } else {
      throw new Error('数据格式错误')
    }
  } catch (e) {
    ElMessage.error('操作日志加载失败: ' + (e?.message || e))
  } finally {
    operLoading.value = false
  }
}

function resetOperFilters() {
  operFilters.title = ''
  operFilters.businessType = null
  operFilters.operName = ''
  operFilters.status = null
  operFilters.timeRange = []
  operPagination.pageNum = 1
  loadOperLog()
}

function openOperDetail(row) {
  currentOperLog.value = row
  operDetailVisible.value = true
}

async function handleExportOper() {
  exportingOper.value = true
  try {
    const params = {
      title: operFilters.title.trim(),
      businessType: operFilters.businessType,
      operName: operFilters.operName.trim(),
      status: operFilters.status
    }
    if (operFilters.timeRange?.length === 2) {
      params.beginTime = operFilters.timeRange[0]
      params.endTime = operFilters.timeRange[1]
    }
    const blob = await exportOperLog(params)
    const url = window.URL.createObjectURL(new Blob([blob]))
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', `操作日志_${Date.now()}.xlsx`)
    document.body.appendChild(link)
    link.click()
    link.remove()
    window.URL.revokeObjectURL(url)
    ElMessage.success('导出成功')
  } catch (e) {
    ElMessage.error('导出失败：' + e.message)
  } finally {
    exportingOper.value = false
  }
}

async function handleCleanOper() {
  try {
    await ElMessageBox.confirm(
      '确定清空所有操作日志吗？清空后数据不可恢复，建议先导出备份。',
      '清空确认',
      { type: 'warning' }
    )
    cleaningOper.value = true
    try {
      await cleanOperLog()
      ElMessage.success('清空成功')
    } catch (e) {
      ElMessage.error('清空失败：' + e.message)
    }
    await loadOperLog()
  } catch (e) {
    if (e !== 'cancel') { /* ignore */ }
  } finally {
    cleaningOper.value = false
  }
}

// ===== 登录日志 =====
const loginLoading = ref(false)
const exportingLogin = ref(false)
const cleaningLogin = ref(false)
const loginLogList = ref([])

const loginFilters = reactive({
  userName: '',
  ipaddr: '',
  status: null,
  timeRange: []
})

const loginPagination = reactive({
  pageNum: 1,
  pageSize: 10,
  total: 0
})

function loginTypeLabel(type) {
  const map = { account: '账号登录', sms: '短信登录', thirdparty: '第三方登录' }
  return map[type] || type
}

function loginTypeTag(type) {
  const map = { account: 'primary', sms: 'success', thirdparty: 'warning' }
  return map[type] || 'info'
}

async function loadLoginLog() {
  loginLoading.value = true
  try {
    const params = {
      pageNum: loginPagination.pageNum,
      pageSize: loginPagination.pageSize,
      userName: loginFilters.userName.trim(),
      ipaddr: loginFilters.ipaddr.trim(),
      status: loginFilters.status
    }
    if (loginFilters.timeRange?.length === 2) {
      params.beginTime = loginFilters.timeRange[0]
      params.endTime = loginFilters.timeRange[1]
    }
    const data = await getLoginLogList(params)
    if (data && Array.isArray(data.rows || data.list)) {
      loginLogList.value = data.rows || data.list
      loginPagination.total = data.total || 0
    } else if (Array.isArray(data)) {
      loginLogList.value = data
      loginPagination.total = data.length
    } else {
      throw new Error('数据格式错误')
    }
  } catch (e) {
    ElMessage.error('登录日志加载失败: ' + (e?.message || e))
  } finally {
    loginLoading.value = false
  }
}

function resetLoginFilters() {
  loginFilters.userName = ''
  loginFilters.ipaddr = ''
  loginFilters.status = null
  loginFilters.timeRange = []
  loginPagination.pageNum = 1
  loadLoginLog()
}

async function handleExportLogin() {
  exportingLogin.value = true
  try {
    const params = {
      userName: loginFilters.userName.trim(),
      ipaddr: loginFilters.ipaddr.trim(),
      status: loginFilters.status
    }
    if (loginFilters.timeRange?.length === 2) {
      params.beginTime = loginFilters.timeRange[0]
      params.endTime = loginFilters.timeRange[1]
    }
    const blob = await exportLoginLog(params)
    const url = window.URL.createObjectURL(new Blob([blob]))
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', `登录日志_${Date.now()}.xlsx`)
    document.body.appendChild(link)
    link.click()
    link.remove()
    window.URL.revokeObjectURL(url)
    ElMessage.success('导出成功')
  } catch (e) {
    ElMessage.error('导出失败：' + e.message)
  } finally {
    exportingLogin.value = false
  }
}

async function handleCleanLogin() {
  try {
    await ElMessageBox.confirm(
      '确定清空所有登录日志吗？清空后数据不可恢复，建议先导出备份。',
      '清空确认',
      { type: 'warning' }
    )
    cleaningLogin.value = true
    try {
      await cleanLoginLog()
      ElMessage.success('清空成功')
    } catch (e) {
      ElMessage.error('清空失败：' + e.message)
    }
    await loadLoginLog()
  } catch (e) {
    if (e !== 'cancel') { /* ignore */ }
  } finally {
    cleaningLogin.value = false
  }
}

onMounted(loadOperLog)
</script>

<style scoped>
.audit-tabs {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}
.audit-tab {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 9px 18px;
  border-radius: 10px;
  background: var(--bg-panel);
  border: 1px solid var(--border-light);
  box-shadow: var(--shadow-sm);
  font-size: 13px;
  font-weight: 500;
  color: var(--text-2);
  cursor: pointer;
  transition: all var(--transition);
}
.audit-tab:hover { color: var(--brand); border-color: var(--brand); }
.audit-tab.active {
  background: linear-gradient(135deg, var(--brand-light), var(--brand-dark));
  color: #fff;
  border-color: transparent;
  box-shadow: 0 8px 22px rgba(79, 70, 229, 0.35);
}
.filters { display: flex; gap: 10px; flex-wrap: wrap; margin-bottom: 14px; align-items: center; }
.filter-spacer { flex: 1; }
.mono {
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
  color: var(--text-2);
  word-break: break-all;
}
.pagination-row {
  display: flex;
  justify-content: flex-end;
  margin-top: 14px;
}
.detail-container { padding: 4px 0; }
.detail-section { margin-top: 16px; }
.detail-section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-2);
  margin-bottom: 8px;
}
.detail-section-title.error { color: var(--el-color-danger); }
.detail-pre {
  background: var(--bg-panel-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px 16px;
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 240px;
  overflow-y: auto;
}
.error-pre {
  background: #fef0f0;
  border-color: #fbc4c4;
  color: #f56c6c;
}
</style>
