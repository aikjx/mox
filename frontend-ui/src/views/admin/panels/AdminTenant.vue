<template>
  <div class="tenant-panel">
    <!-- 顶部统计卡片 -->
    <div class="stat-row">
      <div class="stat-card" v-for="s in stats" :key="s.key" :style="{ '--c': s.color }">
        <div class="stat-label">{{ s.label }}</div>
        <div class="stat-value">{{ s.value }}</div>
        <div class="stat-icon">{{ s.icon }}</div>
      </div>
    </div>

    <!-- 工具栏 -->
    <div class="toolbar">
      <div class="search-box">
        <el-input v-model="keyword" placeholder="搜索租户名称/编码" clearable @keyup.enter="handleSearch" style="width:240px;">
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <el-select v-model="statusFilter" placeholder="状态" clearable style="width:120px;margin-left:8px;" @change="loadList">
          <el-option label="活跃" value="active" />
          <el-option label="停用" value="inactive" />
          <el-option label="试用" value="trial" />
        </el-select>
      </div>
      <div class="action-box">
        <el-button type="primary" @click="openCreate" :icon="Plus">新建租户</el-button>
        <el-button @click="loadList" :icon="Refresh">刷新</el-button>
      </div>
    </div>

    <!-- 租户表格 -->
    <el-table :data="filteredList" v-loading="loading" stripe style="width:100%" @row-click="handleRowClick">
      <el-table-column prop="code" label="租户编码" width="140">
        <template #default="{ row }">
          <el-tag size="small" :type="row.status === 'active' ? 'success' : 'info'" effect="plain">{{ row.code }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="name" label="租户名称" min-width="180" />
      <el-table-column prop="mode" label="隔离模式" width="110">
        <template #default="{ row }">
          <el-tag size="small" :type="row.mode === 'physical' ? 'warning' : ''" effect="plain">{{ row.mode === 'physical' ? '物理隔离' : '逻辑隔离' }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="plan" label="套餐" width="100">
        <template #default="{ row }">
          <el-tag size="small" :type="planTag(row.plan)" effect="plain">{{ planLabel(row.plan) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="status" label="状态" width="90">
        <template #default="{ row }">
          <el-tag size="small" :type="row.status === 'active' ? 'success' : row.status === 'trial' ? 'warning' : 'danger'">{{ statusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="createdAt" label="创建时间" width="170" />
      <el-table-column label="操作" width="240" fixed="right">
        <template #default="{ row }">
          <el-button size="small" type="primary" link @click.stop="switchTo(row)">切换</el-button>
          <el-button size="small" link @click.stop="openEdit(row)">编辑</el-button>
          <el-button size="small" type="warning" link @click.stop="toggleStatus(row)" v-if="row.code !== 'T001'">{{ row.status === 'active' ? '停用' : '启用' }}</el-button>
          <el-button size="small" type="danger" link @click.stop="handleDelete(row)" v-if="row.code !== 'T001'">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 新建/编辑对话框 -->
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑租户' : '新建租户'" width="520px" @close="resetForm">
      <el-form :model="form" label-width="100px" ref="formRef" :rules="rules">
        <el-form-item label="租户编码" prop="code">
          <el-input v-model="form.code" placeholder="如 T002" :disabled="isEdit" />
        </el-form-item>
        <el-form-item label="租户名称" prop="name">
          <el-input v-model="form.name" placeholder="如 某某科技有限公司" />
        </el-form-item>
        <el-form-item label="隔离模式">
          <el-radio-group v-model="form.mode">
            <el-radio value="logical">逻辑隔离</el-radio>
            <el-radio value="physical">物理隔离</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="套餐">
          <el-select v-model="form.plan" style="width:100%">
            <el-option label="免费版" value="free" />
            <el-option label="专业版" value="pro" />
            <el-option label="企业版" value="enterprise" />
            <el-option label="旗舰版" value="ultimate" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态" v-if="isEdit">
          <el-radio-group v-model="form.status">
            <el-radio value="active">活跃</el-radio>
            <el-radio value="inactive">停用</el-radio>
            <el-radio value="trial">试用</el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="submitForm" :loading="submitting">{{ isEdit ? '保存' : '创建' }}</el-button>
      </template>
    </el-dialog>

    <!-- 切换租户确认 -->
    <el-dialog v-model="switchVisible" title="切换租户" width="400px">
      <div style="padding:12px 0;">
        <el-alert type="info" :closable="false" show-icon>
          即将切换到租户 <b>{{ switchTarget?.name }}</b>（{{ switchTarget?.code }}），切换后将刷新当前会话。
        </el-alert>
      </div>
      <template #footer>
        <el-button @click="switchVisible = false">取消</el-button>
        <el-button type="primary" @click="confirmSwitch">确认切换</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Plus, Refresh } from '@element-plus/icons-vue'
import { getTenantList, createTenant, updateTenant, deleteTenant, switchTenant } from '@/api'

const loading = ref(false)
const submitting = ref(false)
const list = ref([])
const keyword = ref('')
const statusFilter = ref('')
const dialogVisible = ref(false)
const switchVisible = ref(false)
const switchTarget = ref(null)
const isEdit = ref(false)
const editId = ref('')
const formRef = ref(null)

const form = reactive({
  code: '',
  name: '',
  mode: 'logical',
  plan: 'free',
  status: 'active'
})

const rules = {
  code: [{ required: true, message: '请输入租户编码', trigger: 'blur' }],
  name: [{ required: true, message: '请输入租户名称', trigger: 'blur' }]
}

const stats = computed(() => [
  { key: 'total', label: '租户总数', value: list.value.length, icon: '🏢', color: '#8BC8EA' },
  { key: 'active', label: '活跃租户', value: list.value.filter(t => t.status === 'active').length, icon: '✅', color: '#52C41A' },
  { key: 'trial', label: '试用租户', value: list.value.filter(t => t.status === 'trial').length, icon: '⏳', color: '#FAAD14' },
  { key: 'enterprise', label: '企业版', value: list.value.filter(t => t.plan === 'enterprise' || t.plan === 'ultimate').length, icon: '💎', color: '#C9A7E8' }
])

const filteredList = computed(() => {
  let r = list.value
  if (keyword.value) {
    const k = keyword.value.toLowerCase()
    r = r.filter(t => t.name.toLowerCase().includes(k) || t.code.toLowerCase().includes(k))
  }
  if (statusFilter.value) {
    r = r.filter(t => t.status === statusFilter.value)
  }
  return r
})

async function loadList() {
  loading.value = true
  try {
    const data = await getTenantList()
    list.value = Array.isArray(data) ? data : (data?.list || data?.data || [])
  } catch (e) {
    ElMessage.error('加载租户列表失败: ' + (e?.message || e))
  } finally {
    loading.value = false
  }
}

function handleSearch() { loadList() }

function planLabel(p) {
  return { free: '免费', pro: '专业', enterprise: '企业', ultimate: '旗舰' }[p] || p
}
function planTag(p) {
  return { free: 'info', pro: '', enterprise: 'warning', ultimate: 'danger' }[p] || ''
}
function statusLabel(s) {
  return { active: '活跃', inactive: '停用', trial: '试用' }[s] || s
}

function openCreate() {
  isEdit.value = false
  editId.value = ''
  Object.assign(form, { code: '', name: '', mode: 'logical', plan: 'free', status: 'active' })
  dialogVisible.value = true
}

function openEdit(row) {
  isEdit.value = true
  editId.value = row.id
  Object.assign(form, { code: row.code, name: row.name, mode: row.mode, plan: row.plan, status: row.status })
  dialogVisible.value = true
}

function resetForm() {
  formRef.value?.resetFields()
}

async function submitForm() {
  await formRef.value?.validate()
  submitting.value = true
  try {
    if (isEdit.value) {
      await updateTenant(editId.value, { name: form.name, status: form.status, plan: form.plan })
      ElMessage.success('租户更新成功')
    } else {
      await createTenant({ code: form.code, name: form.name, mode: form.mode, plan: form.plan })
      ElMessage.success('租户创建成功')
    }
    dialogVisible.value = false
    loadList()
  } catch (e) {
    ElMessage.error('操作失败: ' + (e?.message || e))
  } finally {
    submitting.value = false
  }
}

async function toggleStatus(row) {
  const newStatus = row.status === 'active' ? 'inactive' : 'active'
  try {
    await updateTenant(row.id, { status: newStatus })
    ElMessage.success(`租户已${newStatus === 'active' ? '启用' : '停用'}`)
    loadList()
  } catch (e) {
    ElMessage.error('状态切换失败: ' + (e?.message || e))
  }
}

async function handleDelete(row) {
  await ElMessageBox.confirm(`确定删除租户「${row.name}」？该操作不可恢复，租户下所有数据将被清除。`, '删除确认', { type: 'warning' })
  try {
    await deleteTenant(row.id)
    ElMessage.success('租户已删除')
    loadList()
  } catch (e) {
    ElMessage.error('删除失败: ' + (e?.message || e))
  }
}

function switchTo(row) {
  switchTarget.value = row
  switchVisible.value = true
}

async function confirmSwitch() {
  try {
    const res = await switchTenant(switchTarget.value.id)
    if (res?.switched) {
      ElMessage.success(`已切换到租户「${switchTarget.value.name}」`)
      switchVisible.value = false
      // 触发全局租户切换事件（由 App 层监听刷新会话）
      window.dispatchEvent(new CustomEvent('tenant-switched', { detail: res }))
    }
  } catch (e) {
    ElMessage.error('切换失败: ' + (e?.message || e))
  }
}

function handleRowClick(row) { openEdit(row) }

onMounted(() => loadList())
</script>

<style scoped>
.tenant-panel { padding: 0; }
.stat-row { display: flex; gap: 12px; margin-bottom: 16px; flex-wrap: wrap; }
.stat-card {
  flex: 1 1 140px; min-width: 120px; padding: 14px 16px;
  background: linear-gradient(135deg, rgba(255,255,255,0.9), rgba(255,255,255,0.7));
  border: 1px solid rgba(0,0,0,0.06); border-radius: 12px;
  position: relative; overflow: hidden;
}
.stat-card::before {
  content: ''; position: absolute; left: 0; top: 0; bottom: 0; width: 3px;
  background: var(--c, #8BC8EA);
}
.stat-label { font-size: 12px; color: #6B7280; }
.stat-value { font-size: 24px; font-weight: 700; color: #1A1B1C; margin-top: 4px; }
.stat-icon { position: absolute; right: 12px; top: 50%; transform: translateY(-50%); font-size: 28px; opacity: 0.3; }
.toolbar { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; flex-wrap: wrap; gap: 8px; }
.search-box { display: flex; align-items: center; }
.action-box { display: flex; gap: 8px; }
</style>
