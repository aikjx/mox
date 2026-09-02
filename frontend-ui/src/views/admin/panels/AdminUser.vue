<template>
  <div class="adm-user">
    <!-- 搜索栏 -->
    <div class="panel card-pad search-panel">
      <el-form :model="searchForm" inline label-width="70px">
        <el-form-item label="关键字">
          <el-input
            v-model="searchForm.keyword"
            placeholder="用户名/昵称/手机号/邮箱"
            clearable
            style="width: 220px"
            @keyup.enter="handleSearch"
          />
        </el-form-item>
        <el-form-item label="部门">
          <el-tree-select
            v-model="searchForm.deptId"
            :data="deptTree"
            :props="{ label: 'name', value: 'id', children: 'children' }"
            node-key="id"
            check-strictly
            :render-after-expand="false"
            placeholder="全部部门"
            clearable
            filterable
            style="width: 200px"
          />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="searchForm.status" placeholder="全部" clearable style="width: 120px">
            <el-option label="启用" :value="1" />
            <el-option label="停用" :value="0" />
          </el-select>
        </el-form-item>
        <el-form-item label="创建时间">
          <el-date-picker
            v-model="searchForm.dateRange"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
            value-format="YYYY-MM-DD"
            style="width: 260px"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :icon="Search" @click="handleSearch">搜索</el-button>
          <el-button :icon="Refresh" @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </div>

    <!-- 用户列表 -->
    <div class="panel card-pad">
      <div class="toolbar">
        <div class="toolbar-left">
          <span class="badge primary">共 {{ total }} 位用户</span>
        </div>
        <div class="toolbar-right">
          <el-button :icon="Refresh" :loading="loading" @click="loadList">刷新</el-button>
          <el-button type="primary" :icon="Plus" @click="openUserForm()" v-role="'admin'">新增用户</el-button>
        </div>
      </div>

      <el-table :data="tableData" v-loading="loading" stripe style="width: 100%" @sort-change="handleSortChange">
        <el-table-column label="头像" width="70" align="center">
          <template #default="{ row }">
            <el-avatar :size="36" :src="row.avatar">
              {{ row.nickname?.charAt(0) || row.username?.charAt(0) }}
            </el-avatar>
          </template>
        </el-table-column>
        <el-table-column prop="username" label="用户名" min-width="120" sortable="custom" />
        <el-table-column prop="nickname" label="昵称" min-width="120" />
        <el-table-column prop="deptName" label="部门" min-width="140" />
        <el-table-column prop="postName" label="岗位" min-width="120" />
        <el-table-column prop="phone" label="手机号" width="130" />
        <el-table-column prop="email" label="邮箱" min-width="180" />
        <el-table-column label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-switch
              v-model="row.status"
              :active-value="1"
              :inactive-value="0"
              :loading="row._statusLoading"
              @change="(val) => handleStatusChange(row, val)"
            />
          </template>
        </el-table-column>
        <el-table-column prop="createdAt" label="创建时间" width="180" sortable="custom">
          <template #default="{ row }">{{ formatTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="280" fixed="right" align="center">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">详情</el-button>
            <el-button type="primary" link size="small" @click="openUserForm(row)">编辑</el-button>
            <el-button type="warning" link size="small" @click="handleResetPwd(row)">重置密码</el-button>
            <el-dropdown trigger="click" @command="(cmd) => handleMoreAction(cmd, row)">
              <el-button type="primary" link size="small">
                更多<el-icon class="el-icon--right"><ArrowDown /></el-icon>
              </el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item command="role"><el-icon><Setting /></el-icon>分配角色</el-dropdown-item>
                  <el-dropdown-item :command="row.status === 1 ? 'disable' : 'enable'">
                    <el-icon><component :is="row.status === 1 ? 'SwitchButton' : 'CircleCheck'" /></el-icon>
                    {{ row.status === 1 ? '停用' : '启用' }}
                  </el-dropdown-item>
                  <el-dropdown-item command="delete" divided v-role="'admin'"><el-icon><Delete /></el-icon>删除</el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrap">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :total="total"
          :page-sizes="[10, 20, 50, 100]"
          layout="total, sizes, prev, pager, next, jumper"
          background
          @size-change="loadList"
          @current-change="loadList"
        />
      </div>
    </div>

    <!-- 用户表单对话框 -->
    <el-dialog v-model="userFormVisible" :title="userForm.id ? '编辑用户' : '新增用户'" width="640px" destroy-on-close>
      <el-form ref="userFormRef" :model="userForm" :rules="userFormRules" label-width="90px">
        <el-tabs v-model="userFormTab">
          <el-tab-pane label="基本信息" name="basic">
            <el-form-item label="用户名" prop="username">
              <el-input v-model="userForm.username" placeholder="请输入用户名" maxlength="32" :disabled="!!userForm.id" />
            </el-form-item>
            <el-form-item label="昵称" prop="nickname">
              <el-input v-model="userForm.nickname" placeholder="请输入昵称" maxlength="32" />
            </el-form-item>
            <el-form-item v-if="!userForm.id" label="密码" prop="password">
              <el-input v-model="userForm.password" type="password" placeholder="请输入密码" maxlength="32" show-password />
            </el-form-item>
            <el-form-item v-if="!userForm.id" label="确认密码" prop="confirmPassword">
              <el-input v-model="userForm.confirmPassword" type="password" placeholder="请再次输入密码" maxlength="32" show-password />
            </el-form-item>
            <el-form-item label="头像">
              <el-upload
                class="avatar-uploader"
                :show-file-list="false"
                :before-upload="beforeAvatarUpload"
                :http-request="handleAvatarUpload"
                accept="image/*"
              >
                <el-avatar v-if="userForm.avatar" :size="80" :src="userForm.avatar" />
                <el-icon v-else :size="28" color="var(--text-quaternary)"><Plus /></el-icon>
              </el-upload>
              <div class="upload-tip">支持 JPG/PNG，建议尺寸 200x200</div>
            </el-form-item>
          </el-tab-pane>

          <el-tab-pane label="组织信息" name="org">
            <el-form-item label="所属部门" prop="deptId">
              <el-tree-select
                v-model="userForm.deptId"
                :data="deptTree"
                :props="{ label: 'name', value: 'id', children: 'children' }"
                node-key="id"
                check-strictly
                :render-after-expand="false"
                placeholder="请选择部门"
                filterable
                style="width: 100%"
              />
            </el-form-item>
            <el-form-item label="岗位" prop="postId">
              <el-select v-model="userForm.postId" placeholder="请选择岗位" clearable filterable style="width: 100%">
                <el-option v-for="p in postOptions" :key="p.id" :label="p.name" :value="p.id" />
              </el-select>
            </el-form-item>
            <el-form-item label="手机号" prop="phone">
              <el-input v-model="userForm.phone" placeholder="请输入手机号" maxlength="20" />
            </el-form-item>
            <el-form-item label="邮箱" prop="email">
              <el-input v-model="userForm.email" placeholder="请输入邮箱" maxlength="128" />
            </el-form-item>
          </el-tab-pane>

          <el-tab-pane label="账号设置" name="account">
            <el-form-item label="状态" prop="status">
              <el-radio-group v-model="userForm.status">
                <el-radio :value="1">启用</el-radio>
                <el-radio :value="0">停用</el-radio>
              </el-radio-group>
            </el-form-item>
            <el-form-item label="用户类型" prop="userType">
              <el-radio-group v-model="userForm.userType">
                <el-radio value="normal">普通用户</el-radio>
                <el-radio value="admin">管理员</el-radio>
              </el-radio-group>
            </el-form-item>
            <el-form-item label="备注" prop="remark">
              <el-input v-model="userForm.remark" type="textarea" :rows="4" maxlength="255" show-word-limit />
            </el-form-item>
          </el-tab-pane>
        </el-tabs>
      </el-form>
      <template #footer>
        <el-button @click="userFormVisible = false">取消</el-button>
        <el-button type="primary" :loading="userFormSubmitting" @click="submitUserForm">确定</el-button>
      </template>
    </el-dialog>

    <!-- 用户详情对话框 -->
    <el-dialog v-model="detailVisible" title="用户详情" width="560px">
      <div v-if="detailData" class="user-detail">
        <div class="detail-avatar-row">
          <el-avatar :size="72" :src="detailData.avatar">
            {{ detailData.nickname?.charAt(0) || detailData.username?.charAt(0) }}
          </el-avatar>
          <div class="detail-user-info">
            <div class="detail-name">
              {{ detailData.nickname || detailData.username }}
              <el-tag :type="detailData.status === 1 ? 'success' : 'info'" size="small" style="margin-left: 8px">
                {{ detailData.status === 1 ? '启用' : '停用' }}
              </el-tag>
            </div>
            <div class="detail-sub">@{{ detailData.username }}</div>
          </div>
        </div>
        <el-descriptions :column="2" border size="small">
          <el-descriptions-item label="部门">{{ detailData.deptName || '-' }}</el-descriptions-item>
          <el-descriptions-item label="岗位">{{ detailData.postName || '-' }}</el-descriptions-item>
          <el-descriptions-item label="手机号">{{ detailData.phone || '-' }}</el-descriptions-item>
          <el-descriptions-item label="邮箱">{{ detailData.email || '-' }}</el-descriptions-item>
          <el-descriptions-item label="用户类型">
            <el-tag size="small" :type="detailData.userType === 'admin' ? 'danger' : ''">
              {{ detailData.userType === 'admin' ? '管理员' : '普通用户' }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="创建时间">{{ formatTime(detailData.createdAt) }}</el-descriptions-item>
          <el-descriptions-item label="最后登录">{{ formatTime(detailData.lastLoginAt) }}</el-descriptions-item>
          <el-descriptions-item label="备注" :span="2">{{ detailData.remark || '-' }}</el-descriptions-item>
        </el-descriptions>
      </div>
    </el-dialog>

    <!-- 重置密码对话框 -->
    <el-dialog v-model="resetPwdVisible" title="重置密码" width="420px">
      <el-alert
        type="warning"
        :closable="false"
        :title="`即将重置用户「${resetPwdUser?.nickname || resetPwdUser?.username}」的密码`"
        style="margin-bottom: 14px"
      />
      <div class="reset-pwd-row">
        <span class="reset-pwd-label">新密码：</span>
        <el-input v-model="newPassword" readonly style="flex: 1">
          <template #append>
            <el-button :icon="RefreshRight" @click="generatePassword">生成</el-button>
          </template>
        </el-input>
        <el-button :icon="CopyDocument" @click="copyPassword" style="margin-left: 8px">复制</el-button>
      </div>
      <template #footer>
        <el-button @click="resetPwdVisible = false">取消</el-button>
        <el-button type="primary" :loading="resetPwdLoading" @click="confirmResetPwd">确认重置</el-button>
      </template>
    </el-dialog>

    <!-- 分配角色弹窗 -->
    <el-dialog v-model="roleDialogVisible" title="分配角色" width="640px" destroy-on-close>
      <div class="role-assign-header">
        <span>为用户「{{ assignRoleUser?.nickname || assignRoleUser?.username }}」分配角色</span>
      </div>
      <el-input
        v-model="roleSearchKeyword"
        placeholder="搜索角色"
        :prefix-icon="Search"
        clearable
        size="small"
        style="margin-bottom: 12px; width: 240px"
      />
      <div class="role-list" v-loading="roleLoading">
        <el-checkbox-group v-model="selectedRoleIds">
          <div class="role-item" v-for="r in filteredRoles" :key="r.id">
            <el-checkbox :value="r.id" :disabled="r.builtin && r.type === 'system'">
              <div class="role-item-info">
                <span class="role-name">{{ r.name }}</span>
                <span class="role-code">({{ r.code }})</span>
                <el-tag v-if="r.builtin" size="small" type="info" style="margin-left: 8px">内置</el-tag>
              </div>
            </el-checkbox>
          </div>
        </el-checkbox-group>
        <el-empty v-if="!roleLoading && filteredRoles.length === 0" description="暂无角色" :image-size="60" />
      </div>

      <el-divider>数据权限</el-divider>
      <el-radio-group v-model="dataScope" style="margin-bottom: 12px">
        <el-radio value="all">全部数据</el-radio>
        <el-radio value="dept">本部门数据</el-radio>
        <el-radio value="deptAndChild">本部门及以下</el-radio>
        <el-radio value="self">仅本人数据</el-radio>
        <el-radio value="custom">自定义数据权限</el-radio>
      </el-radio-group>
      <div v-if="dataScope === 'custom'" class="custom-dept-tree">
        <el-tree
          :data="deptTree"
          :props="{ label: 'name', children: 'children' }"
          node-key="id"
          show-checkbox
          :default-checked-keys="customDeptIds"
          ref="customDeptTreeRef"
        />
      </div>

      <template #footer>
        <el-button @click="roleDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="roleAssignLoading" @click="submitRoleAssign">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Search, Plus, Refresh, Delete, Edit, ArrowDown, Setting,
  CopyDocument, RefreshRight, SwitchButton, CircleCheck
} from '@element-plus/icons-vue'
import {
  getUserList, getUserDetail, createUser, updateUser, deleteUser,
  resetUserPwd, changeUserStatus, getUserRoles, assignUserRoles,
  getDeptTree, getPostList, getRoleList
} from '@/api'

// ===== 搜索与列表 =====
const loading = ref(false)
const tableData = ref([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(20)
const sortField = ref('')
const sortOrder = ref('')

const searchForm = reactive({
  keyword: '',
  deptId: null,
  status: null,
  dateRange: []
})

async function loadList() {
  loading.value = true
  try {
    const params = {
      page: page.value,
      pageSize: pageSize.value,
      keyword: searchForm.keyword || undefined,
      deptId: searchForm.deptId || undefined,
      status: searchForm.status ?? undefined,
      startTime: searchForm.dateRange?.[0] || undefined,
      endTime: searchForm.dateRange?.[1] || undefined,
      sortField: sortField.value || undefined,
      sortOrder: sortOrder.value || undefined
    }
    const data = await getUserList(params)
    tableData.value = data?.list || data?.records || (Array.isArray(data) ? data : [])
    total.value = data?.total ?? tableData.value.length
  } catch (e) {
    console.warn('[AdminUser] 加载用户列表失败:', e.message)
  } finally {
    loading.value = false
  }
}

function handleSearch() {
  page.value = 1
  loadList()
}

function handleReset() {
  searchForm.keyword = ''
  searchForm.deptId = null
  searchForm.status = null
  searchForm.dateRange = []
  page.value = 1
  loadList()
}

function handleSortChange({ prop, order }) {
  sortField.value = order ? prop : ''
  sortOrder.value = order === 'ascending' ? 'asc' : order === 'descending' ? 'desc' : ''
  loadList()
}

// ===== 部门树 =====
const deptTree = ref([])

async function loadDeptTree() {
  try {
    const data = await getDeptTree()
    deptTree.value = Array.isArray(data) ? data : []
  } catch (e) {
    console.warn('[AdminUser] 部门树加载失败:', e.message)
  }
}

// ===== 岗位选项 =====
const postOptions = ref([])

async function loadPosts() {
  try {
    const data = await getPostList({ pageSize: 100 })
    postOptions.value = data?.list || data?.records || (Array.isArray(data) ? data : [])
  } catch (e) {
    console.warn('[AdminUser] 岗位加载失败:', e.message)
  }
}

// ===== 用户表单 =====
const userFormVisible = ref(false)
const userFormRef = ref(null)
const userFormSubmitting = ref(false)
const userFormTab = ref('basic')
const userForm = reactive({
  id: null,
  username: '',
  nickname: '',
  password: '',
  confirmPassword: '',
  avatar: '',
  deptId: null,
  postId: null,
  phone: '',
  email: '',
  status: 1,
  userType: 'normal',
  remark: ''
})

const validateConfirmPwd = (rule, value, callback) => {
  if (value !== userForm.password) {
    callback(new Error('两次输入的密码不一致'))
  } else {
    callback()
  }
}

const userFormRules = {
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
  nickname: [{ required: true, message: '请输入昵称', trigger: 'blur' }],
  password: [{ required: true, message: '请输入密码', trigger: 'blur' }, { min: 6, message: '密码至少6位', trigger: 'blur' }],
  confirmPassword: [{ required: true, message: '请确认密码', trigger: 'blur' }, { validator: validateConfirmPwd, trigger: 'blur' }],
  deptId: [{ required: true, message: '请选择部门', trigger: 'change' }]
}

function openUserForm(row = null) {
  userFormTab.value = 'basic'
  if (row) {
    Object.assign(userForm, {
      id: row.id,
      username: row.username,
      nickname: row.nickname || '',
      password: '',
      confirmPassword: '',
      avatar: row.avatar || '',
      deptId: row.deptId || null,
      postId: row.postId || null,
      phone: row.phone || '',
      email: row.email || '',
      status: row.status ?? 1,
      userType: row.userType || 'normal',
      remark: row.remark || ''
    })
  } else {
    Object.assign(userForm, {
      id: null, username: '', nickname: '', password: '', confirmPassword: '',
      avatar: '', deptId: null, postId: null, phone: '', email: '',
      status: 1, userType: 'normal', remark: ''
    })
  }
  userFormVisible.value = true
}

async function submitUserForm() {
  try {
    await userFormRef.value.validate()
  } catch { return }

  userFormSubmitting.value = true
  try {
    const payload = { ...userForm }
    delete payload.confirmPassword
    if (userForm.id) {
      delete payload.password
      await updateUser(userForm.id, payload)
      ElMessage.success('用户更新成功')
    } else {
      await createUser(payload)
      ElMessage.success('用户创建成功')
    }
    userFormVisible.value = false
    await loadList()
  } catch (e) {
    ElMessage.error((userForm.id ? '更新' : '创建') + '失败：' + e.message)
  } finally {
    userFormSubmitting.value = false
  }
}

// 头像上传
function beforeAvatarUpload(file) {
  const isImage = file.type.startsWith('image/')
  const isLt2M = file.size / 1024 / 1024 < 2
  if (!isImage) {
    ElMessage.error('只能上传图片文件')
    return false
  }
  if (!isLt2M) {
    ElMessage.error('图片大小不能超过 2MB')
    return false
  }
  return true
}

function handleAvatarUpload({ file }) {
  // Mock: 使用本地预览
  const reader = new FileReader()
  reader.onload = (e) => {
    userForm.avatar = e.target.result
  }
  reader.readAsDataURL(file)
}

// ===== 详情 =====
const detailVisible = ref(false)
const detailData = ref(null)

async function handleView(row) {
  try {
    const data = await getUserDetail(row.id)
    detailData.value = data
  } catch (e) {
    detailData.value = row
  }
  detailVisible.value = true
}

// ===== 状态切换 =====
async function handleStatusChange(row, val) {
  try {
    await ElMessageBox.confirm(
      `确定${val === 1 ? '启用' : '停用'}用户「${row.nickname || row.username}」吗？`,
      '状态确认',
      { type: 'warning' }
    )
    row._statusLoading = true
    try {
      await changeUserStatus(row.id, val)
      ElMessage.success(`已${val === 1 ? '启用' : '停用'}`)
    } catch (e) {
      row.status = val === 1 ? 0 : 1
      ElMessage.error('操作失败：' + e.message)
    } finally {
      row._statusLoading = false
    }
  } catch {
    row.status = val === 1 ? 0 : 1
  }
}

// ===== 重置密码 =====
const resetPwdVisible = ref(false)
const resetPwdLoading = ref(false)
const resetPwdUser = ref(null)
const newPassword = ref('')

function generatePassword() {
  const chars = 'ABCDEFGHJKMNPQRSTWXYZabcdefhijkmnprstwxyz2345678'
  let pwd = ''
  for (let i = 0; i < 10; i++) {
    pwd += chars.charAt(Math.floor(Math.random() * chars.length))
  }
  newPassword.value = pwd
}

function handleResetPwd(row) {
  resetPwdUser.value = row
  generatePassword()
  resetPwdVisible.value = true
}

async function copyPassword() {
  try {
    await navigator.clipboard.writeText(newPassword.value)
    ElMessage.success('已复制到剪贴板')
  } catch {
    ElMessage.warning('复制失败，请手动复制')
  }
}

async function confirmResetPwd() {
  resetPwdLoading.value = true
  try {
    await resetUserPwd(resetPwdUser.value.id, { password: newPassword.value })
    ElMessage.success('密码重置成功')
    resetPwdVisible.value = false
  } catch (e) {
    ElMessage.error('重置失败：' + e.message)
  } finally {
    resetPwdLoading.value = false
  }
}

// ===== 更多操作 =====
function handleMoreAction(cmd, row) {
  if (cmd === 'role') {
    openRoleDialog(row)
  } else if (cmd === 'disable' || cmd === 'enable') {
    handleStatusChange(row, cmd === 'enable' ? 1 : 0)
  } else if (cmd === 'delete') {
    handleDelete(row)
  }
}

async function handleDelete(row) {
  try {
    await ElMessageBox.confirm(
      `确定删除用户「${row.nickname || row.username}」吗？删除后不可恢复。`,
      '删除确认',
      { type: 'warning' }
    )
    await deleteUser(row.id)
    ElMessage.success('删除成功')
    await loadList()
  } catch (e) {
    if (e !== 'cancel' && e?.message) ElMessage.error('删除失败：' + e.message)
  }
}

// ===== 分配角色 =====
const roleDialogVisible = ref(false)
const roleLoading = ref(false)
const roleAssignLoading = ref(false)
const assignRoleUser = ref(null)
const roleList = ref([])
const selectedRoleIds = ref([])
const roleSearchKeyword = ref('')
const dataScope = ref('dept')
const customDeptIds = ref([])
const customDeptTreeRef = ref(null)

const filteredRoles = computed(() => {
  if (!roleSearchKeyword.value) return roleList.value
  const kw = roleSearchKeyword.value.toLowerCase()
  return roleList.value.filter(r =>
    r.name.toLowerCase().includes(kw) || r.code.toLowerCase().includes(kw)
  )
})

async function openRoleDialog(row) {
  assignRoleUser.value = row
  roleDialogVisible.value = true
  roleLoading.value = true
  try {
    // 加载所有角色
    const allRoles = await getRoleList({ pageSize: 100 })
    roleList.value = allRoles?.list || allRoles?.records || (Array.isArray(allRoles) ? allRoles : [])

    // 加载用户已有角色
    const userRoles = await getUserRoles(row.id)
    const roles = userRoles?.list || userRoles?.roles || (Array.isArray(userRoles) ? userRoles : [])
    selectedRoleIds.value = roles.map(r => r.id || r.roleId)
    dataScope.value = userRoles?.dataScope || 'dept'
    customDeptIds.value = userRoles?.deptIds || []
  } catch (e) {
    console.warn('[AdminUser] 加载角色失败:', e.message)
  } finally {
    roleLoading.value = false
  }
}

async function submitRoleAssign() {
  roleAssignLoading.value = true
  try {
    let deptIds = customDeptIds.value
    if (dataScope.value === 'custom' && customDeptTreeRef.value) {
      deptIds = customDeptTreeRef.value.getCheckedKeys(false)
    }
    await assignUserRoles(assignRoleUser.value.id, {
      roleIds: selectedRoleIds.value,
      dataScope: dataScope.value,
      deptIds: dataScope.value === 'custom' ? deptIds : []
    })
    ElMessage.success('角色分配成功')
    roleDialogVisible.value = false
  } catch (e) {
    ElMessage.error('分配失败：' + e.message)
  } finally {
    roleAssignLoading.value = false
  }
}

// ===== 工具函数 =====
function formatTime(t) {
  if (!t) return '-'
  try {
    const d = new Date(t)
    return d.toLocaleString('zh-CN', { hour12: false })
  } catch {
    return String(t)
  }
}

onMounted(() => {
  loadList()
  loadDeptTree()
  loadPosts()
})
</script>

<style scoped>
.adm-user {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.search-panel {
  margin-bottom: 0;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 14px;
  flex-wrap: wrap;
  gap: 10px;
}

.toolbar-right {
  display: flex;
  gap: 8px;
}

.pagination-wrap {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

.user-detail {
  padding: 8px 0;
}

.detail-avatar-row {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 16px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--border-ghost);
}

.detail-user-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.detail-name {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.detail-sub {
  font-size: 13px;
  color: var(--text-tertiary);
}

.avatar-uploader {
  :deep(.el-upload) {
    border: 1px dashed var(--border-soft);
    border-radius: 10px;
    cursor: pointer;
    width: 80px;
    height: 80px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-surface-2);
    transition: all 0.2s;
  }
  :deep(.el-upload:hover) {
    border-color: var(--brand-500);
  }
}

.upload-tip {
  font-size: 12px;
  color: var(--text-quaternary);
  margin-top: 6px;
}

.reset-pwd-row {
  display: flex;
  align-items: center;
  gap: 4px;
}

.reset-pwd-label {
  font-size: 14px;
  color: var(--text-secondary);
  flex-shrink: 0;
}

.role-assign-header {
  font-size: 14px;
  color: var(--text-secondary);
  margin-bottom: 12px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border-ghost);
}

.role-list {
  max-height: 240px;
  overflow-y: auto;
  border: 1px solid var(--border-ghost);
  border-radius: 8px;
  padding: 8px;
}

.role-item {
  padding: 8px 10px;
  border-radius: 6px;
  transition: background 0.15s;
}

.role-item:hover {
  background: var(--bg-surface-2);
}

.role-item-info {
  display: flex;
  align-items: center;
  font-size: 13px;
}

.role-name {
  font-weight: 500;
  color: var(--text-primary);
}

.role-code {
  color: var(--text-tertiary);
  font-size: 12px;
  margin-left: 6px;
}

.custom-dept-tree {
  border: 1px solid var(--border-ghost);
  border-radius: 8px;
  padding: 10px;
  max-height: 220px;
  overflow-y: auto;
}
</style>
