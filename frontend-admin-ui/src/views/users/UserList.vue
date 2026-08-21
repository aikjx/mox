<template>
  <div>
    <div class="admin-card">
      <div class="admin-table-toolbar">
        <div class="toolbar-left">
          <el-input
            v-model="searchText"
            placeholder="搜索用户名、邮箱或姓名"
            :prefix-icon="Search"
            style="width: 260px"
            clearable
            @keyup.enter="handleSearch"
          />
          <el-select v-model="filterRole" placeholder="角色" clearable style="width: 140px; margin-left: 10px">
            <el-option v-for="r in roleOptions" :key="r.value" :label="r.label" :value="r.value" />
          </el-select>
          <el-select v-model="filterStatus" placeholder="状态" clearable style="width: 120px; margin-left: 10px">
            <el-option label="启用" value="active" />
            <el-option label="禁用" value="inactive" />
          </el-select>
          <el-button type="primary" :icon="Search" @click="handleSearch" style="margin-left: 10px">搜索</el-button>
          <el-button :icon="Refresh" @click="resetSearch">重置</el-button>
        </div>
        <div class="toolbar-right">
          <el-button type="primary" :icon="Plus" @click="openCreateDialog">新增用户</el-button>
          <el-button :icon="Download">导出</el-button>
        </div>
      </div>

      <el-table :data="pagedUsers" v-loading="loading" stripe border style="width: 100%">
        <el-table-column type="index" label="#" width="50" />
        <el-table-column prop="username" label="用户名" width="140">
          <template #default="{ row }">
            <div class="user-cell">
              <el-avatar :size="32" :style="{ backgroundColor: getAvatarColor(row.username) }">
                {{ row.username.charAt(0).toUpperCase() }}
              </el-avatar>
              <span class="user-name">{{ row.username }}</span>
            </div>
          </template>
        </el-table-column>
        <el-table-column prop="realName" label="姓名" width="120" />
        <el-table-column prop="email" label="邮箱" width="200" />
        <el-table-column prop="role" label="角色" width="140">
          <template #default="{ row }">
            <el-tag :type="getRoleTagType(row.role)" effect="light">{{ getRoleLabel(row.role) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-switch
              v-model="row.status"
              active-value="active"
              inactive-value="inactive"
              @change="(val) => handleStatusChange(row, val)"
            />
          </template>
        </el-table-column>
        <el-table-column prop="lastLogin" label="最后登录" width="170" />
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" :icon="Edit" @click="openEditDialog(row)">编辑</el-button>
            <el-button type="warning" link size="small" :icon="Lock" @click="openRoleDialog(row)">角色</el-button>
            <el-button type="danger" link size="small" :icon="Delete" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="filteredUsers.length"
          layout="total, sizes, prev, pager, next, jumper"
          background
        />
      </div>
    </div>

    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="500px" @close="handleDialogClose">
      <el-form :model="formData" :rules="formRules" ref="formRef" label-width="100px">
        <el-form-item label="用户名" prop="username">
          <el-input v-model="formData.username" :disabled="isEdit" placeholder="请输入用户名" />
        </el-form-item>
        <el-form-item label="姓名" prop="realName">
          <el-input v-model="formData.realName" placeholder="请输入姓名" />
        </el-form-item>
        <el-form-item label="邮箱" prop="email">
          <el-input v-model="formData.email" placeholder="请输入邮箱" />
        </el-form-item>
        <el-form-item v-if="!isEdit" label="密码" prop="password">
          <el-input v-model="formData.password" type="password" placeholder="请输入密码" show-password />
        </el-form-item>
        <el-form-item v-if="isEdit" label="新密码">
          <el-input v-model="formData.newPassword" type="password" placeholder="留空则不修改" show-password />
        </el-form-item>
        <el-form-item label="角色" prop="role">
          <el-select v-model="formData.role" placeholder="请选择角色" style="width: 100%">
            <el-option v-for="r in roleOptions" :key="r.value" :label="r.label" :value="r.value" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-radio-group v-model="formData.status">
            <el-radio value="active">启用</el-radio>
            <el-radio value="inactive">禁用</el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="roleDialogVisible" title="分配角色" width="400px">
      <div class="role-assign-list">
        <div v-for="r in roleOptions" :key="r.value" class="role-assign-item">
          <el-checkbox
            v-model="assignedRoles"
            :value="r.value"
            :label="r.value"
          >
            {{ r.label }}
          </el-checkbox>
        </div>
      </div>
      <template #footer>
        <el-button @click="roleDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleRoleAssign">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { adminApi } from '@/api/index'
import { Search, Refresh, Plus, Download, Edit, Delete, Lock } from '@element-plus/icons-vue'

const loading = ref(false)
const searchText = ref('')
const filterRole = ref('')
const filterStatus = ref('')
const currentPage = ref(1)
const pageSize = ref(10)
const formRef = ref(null)

const roleOptions = ref([
  { value: 'super_admin', label: '超级管理员' },
  { value: 'admin', label: '系统管理员' },
  { value: 'operator', label: '运营人员' },
  { value: 'viewer', label: '只读用户' },
  { value: 'guest', label: '访客' }
])

const users = ref([
  { id: 1, username: 'admin', realName: '超级管理员', email: 'admin@example.com', role: 'super_admin', status: 'active', lastLogin: '2026-08-21 14:32:15' },
  { id: 2, username: 'zhangsan', realName: '张三', email: 'zhangsan@example.com', role: 'admin', status: 'active', lastLogin: '2026-08-21 10:15:22' },
  { id: 3, username: 'lisi', realName: '李四', email: 'lisi@example.com', role: 'operator', status: 'active', lastLogin: '2026-08-20 16:45:08' },
  { id: 4, username: 'wangwu', realName: '王五', email: 'wangwu@example.com', role: 'viewer', status: 'inactive', lastLogin: '2026-08-18 09:30:45' },
  { id: 5, username: 'zhaoliu', realName: '赵六', email: 'zhaoliu@example.com', role: 'operator', status: 'active', lastLogin: '2026-08-21 08:12:33' },
  { id: 6, username: 'qianqi', realName: '钱七', email: 'qianqi@example.com', role: 'viewer', status: 'active', lastLogin: '2026-08-19 14:22:18' },
  { id: 7, username: 'sunba', realName: '孙八', email: 'sunba@example.com', role: 'guest', status: 'inactive', lastLogin: '2026-08-15 11:05:42' },
  { id: 8, username: 'zhoujiu', realName: '周九', email: 'zhoujiu@example.com', role: 'admin', status: 'active', lastLogin: '2026-08-21 11:38:55' },
  { id: 9, username: 'wushi', realName: '吴十', email: 'wushi@example.com', role: 'operator', status: 'active', lastLogin: '2026-08-20 20:18:27' },
  { id: 10, username: 'admin01', realName: '管理员01', email: 'admin01@example.com', role: 'admin', status: 'active', lastLogin: '2026-08-21 13:44:11' },
  { id: 11, username: 'viewer02', realName: '只读02', email: 'viewer02@example.com', role: 'viewer', status: 'active', lastLogin: '2026-08-21 07:55:39' },
  { id: 12, username: 'temp_user', realName: '临时用户', email: 'temp@example.com', role: 'guest', status: 'inactive', lastLogin: '2026-08-10 16:40:22' }
])

const filteredUsers = computed(() => {
  return users.value.filter(u => {
    const matchSearch = !searchText.value ||
      u.username.includes(searchText.value) ||
      u.realName.includes(searchText.value) ||
      u.email.includes(searchText.value)
    const matchRole = !filterRole.value || u.role === filterRole.value
    const matchStatus = !filterStatus.value || u.status === filterStatus.value
    return matchSearch && matchRole && matchStatus
  })
})

const pagedUsers = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredUsers.value.slice(start, start + pageSize.value)
})

const dialogVisible = ref(false)
const dialogTitle = ref('新增用户')
const isEdit = ref(false)
const formData = reactive({ id: null, username: '', realName: '', email: '', password: '', newPassword: '', role: 'viewer', status: 'active' })
const formRules = {
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
  realName: [{ required: true, message: '请输入姓名', trigger: 'blur' }],
  email: [{ required: true, message: '请输入邮箱', trigger: 'blur' }, { type: 'email', message: '请输入正确的邮箱', trigger: 'blur' }],
  password: [{ required: true, message: '请输入密码', trigger: 'blur' }],
  role: [{ required: true, message: '请选择角色', trigger: 'change' }]
}

const roleDialogVisible = ref(false)
const assignedRoles = ref([])
const currentEditUser = ref(null)

function getAvatarColor(name) {
  const colors = ['#409eff', '#67c23a', '#e6a23c', '#f56c6c', '#909399', '#8e44ad', '#16a085']
  let hash = 0
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash)
  return colors[Math.abs(hash) % colors.length]
}

function getRoleLabel(role) {
  return roleOptions.value.find(r => r.value === role)?.label || role
}

function getRoleTagType(role) {
  const map = { super_admin: 'danger', admin: 'warning', operator: '', viewer: 'info', guest: 'info' }
  return map[role] || ''
}

function handleSearch() { currentPage.value = 1 }
function resetSearch() {
  searchText.value = ''
  filterRole.value = ''
  filterStatus.value = ''
  currentPage.value = 1
}

function openCreateDialog() {
  isEdit.value = false
  dialogTitle.value = '新增用户'
  Object.assign(formData, { id: null, username: '', realName: '', email: '', password: '', newPassword: '', role: 'viewer', status: 'active' })
  dialogVisible.value = true
}

function openEditDialog(row) {
  isEdit.value = true
  dialogTitle.value = '编辑用户'
  Object.assign(formData, { ...row, password: '', newPassword: '' })
  dialogVisible.value = true
}

function handleDialogClose() {
  formRef.value?.resetFields()
}

async function handleSubmit() {
  if (!formRef.value) return
  await formRef.value.validate()
  try {
    if (isEdit.value) {
      await adminApi.updateUser(formData.id, formData)
      const idx = users.value.findIndex(u => u.id === formData.id)
      if (idx > -1) users.value[idx] = { ...users.value[idx], ...formData }
      ElMessage.success('用户更新成功')
    } else {
      const newId = Math.max(...users.value.map(u => u.id)) + 1
      users.value.push({ id: newId, ...formData, lastLogin: '-' })
      ElMessage.success('用户创建成功')
    }
    dialogVisible.value = false
  } catch (e) {
    if (e.response?.status === 400 || e.code === 'ERR_BAD_REQUEST') {
      if (isEdit.value) ElMessage.success('用户更新成功（模拟）')
      else ElMessage.success('用户创建成功（模拟）')
      dialogVisible.value = false
    }
  }
}

function handleStatusChange(row, val) {
  ElMessage.success(`用户 ${row.username} 状态已${val === 'active' ? '启用' : '禁用'}`)
}

async function handleDelete(row) {
  try {
    await ElMessageBox.confirm(`确定要删除用户 "${row.username}" 吗？此操作不可恢复。`, '删除确认', { type: 'warning' })
    try {
      await adminApi.deleteUser(row.id)
    } catch (e) { /* mock */ }
    users.value = users.value.filter(u => u.id !== row.id)
    ElMessage.success('删除成功')
  } catch (e) { /* cancelled */ }
}

function openRoleDialog(row) {
  currentEditUser.value = row
  assignedRoles.value = row.role ? [row.role] : []
  roleDialogVisible.value = true
}

function handleRoleAssign() {
  if (currentEditUser.value) {
    const newRole = assignedRoles.value[0] || 'viewer'
    currentEditUser.value.role = newRole
    ElMessage.success('角色分配成功')
  }
  roleDialogVisible.value = false
}

onMounted(async () => {
  loading.value = true
  try {
    const data = await adminApi.getUsers()
    if (data?.data) users.value = data.data
  } catch (e) { /* use mock data */ }
  loading.value = false
})
</script>

<style scoped>
.user-cell {
  display: flex;
  align-items: center;
  gap: 10px;
}

.user-name {
  font-weight: 500;
  color: #303133;
}

.pagination-wrapper {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}

.role-assign-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.role-assign-item {
  padding: 8px 12px;
  border: 1px solid #ebeef5;
  border-radius: 6px;
  transition: border-color 0.3s;
}

.role-assign-item:hover {
  border-color: #409eff;
}
</style>