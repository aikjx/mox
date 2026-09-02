<template>
  <div class="adm-role">
    <!-- 搜索栏 -->
    <div class="panel card-pad search-panel">
      <el-form :model="searchForm" inline label-width="70px">
        <el-form-item label="角色名称">
          <el-input
            v-model="searchForm.keyword"
            placeholder="角色名称/编码"
            clearable
            style="width: 220px"
            @keyup.enter="handleSearch"
          />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="searchForm.status" placeholder="全部" clearable style="width: 120px">
            <el-option label="启用" :value="1" />
            <el-option label="停用" :value="0" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :icon="Search" @click="handleSearch">搜索</el-button>
          <el-button :icon="Refresh" @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </div>

    <!-- 角色列表 -->
    <div class="panel card-pad">
      <div class="toolbar">
        <div class="toolbar-left">
          <span class="badge primary">共 {{ total }} 个角色</span>
        </div>
        <div class="toolbar-right">
          <el-button :icon="Refresh" :loading="loading" @click="loadList">刷新</el-button>
          <el-button type="primary" :icon="Plus" @click="openRoleForm()" v-role="'admin'">新增角色</el-button>
        </div>
      </div>

      <el-table :data="tableData" v-loading="loading" stripe style="width: 100%">
        <el-table-column prop="name" label="角色名称" min-width="140">
          <template #default="{ row }">
            <span class="role-name-cell">
              {{ row.name }}
              <el-tag v-if="row.builtin" size="small" type="info" style="margin-left: 6px">内置</el-tag>
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="code" label="角色编码" min-width="140" />
        <el-table-column label="角色类型" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="row.builtin ? '' : 'success'" size="small">
              {{ row.builtin ? '内置' : '自定义' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="数据权限" width="130" align="center">
          <template #default="{ row }">
            {{ getDataScopeLabel(row.dataScope) }}
          </template>
        </el-table-column>
        <el-table-column prop="sort" label="排序" width="80" align="center" />
        <el-table-column label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
              {{ row.status === 1 ? '启用' : '停用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="createdAt" label="创建时间" width="180">
          <template #default="{ row }">{{ formatTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="320" fixed="right" align="center">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openRoleForm(row)">编辑</el-button>
            <el-button type="primary" link size="small" @click="openMenuPermDialog(row)">菜单权限</el-button>
            <el-button type="primary" link size="small" @click="openDataPermDialog(row)">数据权限</el-button>
            <el-dropdown trigger="click" @command="(cmd) => handleMoreAction(cmd, row)">
              <el-button type="primary" link size="small">
                更多<el-icon class="el-icon--right"><ArrowDown /></el-icon>
              </el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item command="users"><el-icon><User /></el-icon>查看用户</el-dropdown-item>
                  <el-dropdown-item command="copy"><el-icon><CopyDocument /></el-icon>复制角色</el-dropdown-item>
                  <el-dropdown-item command="delete" divided :disabled="row.builtin" v-role="'admin'">
                    <el-icon><Delete /></el-icon>
                    {{ row.builtin ? '内置角色不可删' : '删除' }}
                  </el-dropdown-item>
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

    <!-- 角色表单对话框 -->
    <el-dialog v-model="roleFormVisible" :title="roleForm.id ? '编辑角色' : '新增角色'" width="520px" destroy-on-close>
      <el-form ref="roleFormRef" :model="roleForm" :rules="roleFormRules" label-width="100px">
        <el-form-item label="角色名称" prop="name">
          <el-input v-model="roleForm.name" placeholder="请输入角色名称" maxlength="64" show-word-limit />
        </el-form-item>
        <el-form-item label="角色编码" prop="code">
          <el-input v-model="roleForm.code" placeholder="请输入角色编码" maxlength="32" show-word-limit :disabled="roleForm.builtin" />
        </el-form-item>
        <el-form-item label="角色类型">
          <el-tag :type="roleForm.builtin ? '' : 'success'" size="small">
            {{ roleForm.builtin ? '内置角色' : '自定义角色' }}
          </el-tag>
          <span class="form-tip">内置角色不可删除，编码不可修改</span>
        </el-form-item>
        <el-form-item label="数据权限" prop="dataScope">
          <el-select v-model="roleForm.dataScope" placeholder="请选择数据权限" style="width: 100%">
            <el-option label="全部数据" value="all" />
            <el-option label="本部门数据" value="dept" />
            <el-option label="本部门及以下" value="deptAndChild" />
            <el-option label="仅本人数据" value="self" />
            <el-option label="自定义数据权限" value="custom" />
          </el-select>
        </el-form-item>
        <el-form-item label="排序号" prop="sort">
          <el-input-number v-model="roleForm.sort" :min="0" :max="999" style="width: 120px" />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-radio-group v-model="roleForm.status">
            <el-radio :value="1">启用</el-radio>
            <el-radio :value="0">停用</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="roleForm.remark" type="textarea" :rows="3" maxlength="255" show-word-limit />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="roleFormVisible = false">取消</el-button>
        <el-button type="primary" :loading="roleFormSubmitting" @click="submitRoleForm">确定</el-button>
      </template>
    </el-dialog>

    <!-- 分配菜单权限弹窗 -->
    <el-dialog v-model="menuPermVisible" title="分配菜单权限" width="900px" destroy-on-close class="menu-perm-dialog">
      <div class="menu-perm-header">
        <span class="perm-title">为角色「{{ menuPermRole?.name }}」分配权限</span>
        <div class="perm-tabs">
          <el-radio-group v-model="permType" size="small">
            <el-radio-button value="menu">菜单权限</el-radio-button>
            <el-radio-button value="button">按钮权限</el-radio-button>
            <el-radio-button value="api">接口权限</el-radio-button>
          </el-radio-group>
        </div>
      </div>

      <div class="menu-perm-toolbar">
        <div class="toolbar-left">
          <el-input
            v-model="menuSearchKeyword"
            placeholder="搜索菜单"
            :prefix-icon="Search"
            clearable
            size="small"
            style="width: 220px"
          />
        </div>
        <div class="toolbar-right">
          <el-button size="small" :icon="ArrowDown" @click="expandAllMenu">展开全部</el-button>
          <el-button size="small" :icon="ArrowUp" @click="collapseAllMenu">折叠全部</el-button>
          <el-button size="small" :icon="Select" @click="selectAllMenu">全选</el-button>
          <el-button size="small" :icon="Close" @click="clearAllMenu">清空</el-button>
        </div>
      </div>

      <div class="menu-perm-body">
        <div class="menu-tree-col">
          <div class="col-title">菜单树</div>
          <div class="tree-container">
            <el-tree
              ref="menuTreeRef"
              :data="menuTree"
              :props="{ label: 'name', children: 'children' }"
              node-key="id"
              show-checkbox
              :default-checked-keys="defaultMenuKeys"
              :default-expanded-keys="defaultExpandedMenuKeys"
              :filter-node-method="filterMenuNode"
              :expand-on-click-node="false"
              @node-click="handleMenuNodeClick"
              @check="handleMenuCheck"
            >
              <template #default="{ data }">
                <span class="menu-tree-node">
                  <el-icon :size="14" v-if="data.icon"><component :is="data.icon" /></el-icon>
                  <span class="menu-name">{{ data.name }}</span>
                  <el-tag v-if="data.type === 'M'" size="small" type="primary" effect="plain">目录</el-tag>
                  <el-tag v-else-if="data.type === 'C'" size="small" type="success" effect="plain">菜单</el-tag>
                  <el-tag v-else size="small" type="warning" effect="plain">按钮</el-tag>
                </span>
              </template>
            </el-tree>
          </div>
        </div>

        <div class="button-perm-col">
          <div class="col-title">按钮级权限</div>
          <div class="button-perm-container">
            <div v-if="selectedMenu" class="selected-menu-info">
              <el-icon :size="16" color="var(--brand-600)"><Menu /></el-icon>
              <span class="selected-menu-name">{{ selectedMenu.name }}</span>
            </div>
            <el-empty v-else description="请选择左侧菜单查看按钮权限" :image-size="60" />

            <div v-if="selectedMenu && buttonPerms.length > 0" class="button-perm-list">
              <el-checkbox-group v-model="selectedButtonPerms">
                <div class="button-perm-item" v-for="btn in buttonPerms" :key="btn.id">
                  <el-checkbox :value="btn.id">
                    <span class="btn-name">{{ btn.name }}</span>
                    <span class="btn-code">({{ btn.code }})</span>
                  </el-checkbox>
                </div>
              </el-checkbox-group>
            </div>

            <div v-if="selectedMenu && apiPerms.length > 0" class="api-perm-section">
              <div class="sub-title">接口权限</div>
              <el-checkbox-group v-model="selectedApiPerms">
                <div class="api-perm-item" v-for="api in apiPerms" :key="api.id">
                  <el-checkbox :value="api.id">
                    <el-tag size="small" :type="apiMethodTag(api.method)">
                      {{ api.method }}
                    </el-tag>
                    <span class="api-path">{{ api.path }}</span>
                  </el-checkbox>
                </div>
              </el-checkbox-group>
            </div>
          </div>
        </div>
      </div>

      <template #footer>
        <el-button @click="resetMenuPerms">重置</el-button>
        <el-button type="primary" :loading="menuPermSubmitting" @click="submitMenuPerms">保存</el-button>
      </template>
    </el-dialog>

    <!-- 分配数据权限弹窗 -->
    <el-dialog v-model="dataPermVisible" title="分配数据权限" width="560px" destroy-on-close>
      <el-alert
        :title="`为角色「${dataPermRole?.name}」配置数据权限范围`"
        type="info"
        :closable="false"
        style="margin-bottom: 16px"
      />
      <el-form label-width="100px">
        <el-form-item label="数据范围">
          <el-radio-group v-model="dataScopeForm" style="display: flex; flex-direction: column; gap: 10px">
            <el-radio value="all">
              <div class="scope-item">
                <span class="scope-name">全部数据</span>
                <span class="scope-desc">可查看所有部门的数据</span>
              </div>
            </el-radio>
            <el-radio value="dept">
              <div class="scope-item">
                <span class="scope-name">本部门数据</span>
                <span class="scope-desc">仅可查看本部门的数据</span>
              </div>
            </el-radio>
            <el-radio value="deptAndChild">
              <div class="scope-item">
                <span class="scope-name">本部门及以下</span>
                <span class="scope-desc">可查看本部门及下级部门的数据</span>
              </div>
            </el-radio>
            <el-radio value="self">
              <div class="scope-item">
                <span class="scope-name">仅本人数据</span>
                <span class="scope-desc">仅可查看自己创建的数据</span>
              </div>
            </el-radio>
            <el-radio value="custom">
              <div class="scope-item">
                <span class="scope-name">自定义数据权限</span>
                <span class="scope-desc">手动选择可访问的部门</span>
              </div>
            </el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item v-if="dataScopeForm === 'custom'" label="选择部门">
          <div class="custom-dept-tree-wrapper">
            <el-tree
              ref="dataPermDeptTreeRef"
              :data="deptTree"
              :props="{ label: 'name', children: 'children' }"
              node-key="id"
              show-checkbox
              :default-checked-keys="customDeptCheckedKeys"
              :default-expanded-keys="deptDefaultExpanded"
            />
          </div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dataPermVisible = false">取消</el-button>
        <el-button type="primary" :loading="dataPermSubmitting" @click="submitDataPerms">保存</el-button>
      </template>
    </el-dialog>

    <!-- 查看用户列表弹窗 -->
    <el-dialog v-model="userListVisible" title="角色用户列表" width="700px" destroy-on-close>
      <el-table :data="roleUserList" v-loading="roleUserLoading" stripe style="width: 100%">
        <el-table-column label="头像" width="60" align="center">
          <template #default="{ row }">
            <el-avatar :size="32" :src="row.avatar">
              {{ row.nickname?.charAt(0) || row.username?.charAt(0) }}
            </el-avatar>
          </template>
        </el-table-column>
        <el-table-column prop="username" label="用户名" min-width="120" />
        <el-table-column prop="nickname" label="昵称" min-width="120" />
        <el-table-column prop="deptName" label="部门" min-width="140" />
        <el-table-column prop="phone" label="手机号" width="130" />
        <el-table-column label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
              {{ row.status === 1 ? '启用' : '停用' }}
            </el-tag>
          </template>
        </el-table-column>
      </el-table>
      <div class="pagination-wrap" v-if="roleUserTotal > 0">
        <el-pagination
          v-model:current-page="roleUserPage"
          v-model:page-size="roleUserPageSize"
          :total="roleUserTotal"
          :page-sizes="[10, 20, 50]"
          layout="total, sizes, prev, pager, next, jumper"
          background
          small
          @size-change="loadRoleUsers"
          @current-change="loadRoleUsers"
        />
      </div>
    </el-dialog>

    <!-- 复制角色对话框 -->
    <el-dialog v-model="copyVisible" title="复制角色" width="420px" destroy-on-close>
      <el-alert type="info" :closable="false" title="将复制角色的基本信息和权限配置" style="margin-bottom: 14px" />
      <el-form label-width="90px">
        <el-form-item label="原角色">
          <el-input :value="copySourceRole?.name" disabled />
        </el-form-item>
        <el-form-item label="新角色名称" required>
          <el-input v-model="copyForm.name" placeholder="请输入新角色名称" maxlength="64" />
        </el-form-item>
        <el-form-item label="新角色编码" required>
          <el-input v-model="copyForm.code" placeholder="请输入新角色编码" maxlength="32" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="copyVisible = false">取消</el-button>
        <el-button type="primary" :loading="copySubmitting" @click="submitCopyRole">确认复制</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, watch, onMounted, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Search, Plus, Refresh, Delete, Edit, ArrowDown, ArrowUp,
  User, CopyDocument, Select, Close, Menu
} from '@element-plus/icons-vue'
import {
  getRoleList, getRoleDetail, createRole, updateRole, deleteRole,
  getRoleMenuPerms, assignRoleMenuPerms,
  getRoleDataPerms, assignRoleDataPerms,
  getRoleUsers, copyRole,
  getMenuTree, getDeptTree
} from '@/api'

// ===== 数据权限标签 =====
const DATA_SCOPE_MAP = {
  all: '全部数据',
  dept: '本部门数据',
  deptAndChild: '本部门及以下',
  self: '仅本人数据',
  custom: '自定义'
}

function getDataScopeLabel(scope) {
  return DATA_SCOPE_MAP[scope] || '-'
}

function apiMethodTag(method) {
  const map = { GET: 'success', POST: 'primary', PUT: 'warning', DELETE: 'danger', PATCH: 'info' }
  return map[method?.toUpperCase()] || 'info'
}

// ===== 搜索与列表 =====
const loading = ref(false)
const tableData = ref([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(20)

const searchForm = reactive({
  keyword: '',
  status: null
})

async function loadList() {
  loading.value = true
  try {
    const data = await getRoleList({
      page: page.value,
      pageSize: pageSize.value,
      keyword: searchForm.keyword || undefined,
      status: searchForm.status ?? undefined
    })
    tableData.value = data?.list || data?.records || (Array.isArray(data) ? data : [])
    total.value = data?.total ?? tableData.value.length
  } catch (e) {
    console.warn('[AdminRole] 加载角色列表失败:', e.message)
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
  searchForm.status = null
  page.value = 1
  loadList()
}

// ===== 角色表单 =====
const roleFormVisible = ref(false)
const roleFormRef = ref(null)
const roleFormSubmitting = ref(false)
const roleForm = reactive({
  id: null,
  name: '',
  code: '',
  builtin: false,
  dataScope: 'dept',
  sort: 0,
  status: 1,
  remark: ''
})

const roleFormRules = {
  name: [{ required: true, message: '请输入角色名称', trigger: 'blur' }],
  code: [{ required: true, message: '请输入角色编码', trigger: 'blur' }],
  dataScope: [{ required: true, message: '请选择数据权限', trigger: 'change' }]
}

function openRoleForm(row = null) {
  if (row) {
    Object.assign(roleForm, {
      id: row.id,
      name: row.name,
      code: row.code,
      builtin: row.builtin || false,
      dataScope: row.dataScope || 'dept',
      sort: row.sort ?? 0,
      status: row.status ?? 1,
      remark: row.remark || ''
    })
  } else {
    Object.assign(roleForm, {
      id: null, name: '', code: '', builtin: false,
      dataScope: 'dept', sort: 0, status: 1, remark: ''
    })
  }
  roleFormVisible.value = true
  nextTick(() => {
    roleFormRef.value?.clearValidate()
  })
}

async function submitRoleForm() {
  try {
    await roleFormRef.value.validate()
  } catch { return }

  roleFormSubmitting.value = true
  try {
    const payload = { ...roleForm }
    if (roleForm.id) {
      await updateRole(roleForm.id, payload)
      ElMessage.success('角色更新成功')
    } else {
      await createRole(payload)
      ElMessage.success('角色创建成功')
    }
    roleFormVisible.value = false
    await loadList()
  } catch (e) {
    ElMessage.error((roleForm.id ? '更新' : '创建') + '失败：' + e.message)
  } finally {
    roleFormSubmitting.value = false
  }
}

// ===== 更多操作 =====
function handleMoreAction(cmd, row) {
  if (cmd === 'users') {
    openUserListDialog(row)
  } else if (cmd === 'copy') {
    openCopyDialog(row)
  } else if (cmd === 'delete') {
    handleDelete(row)
  }
}

async function handleDelete(row) {
  if (row.builtin) {
    ElMessage.warning('内置角色不可删除')
    return
  }
  try {
    await ElMessageBox.confirm(
      `确定删除角色「${row.name}」吗？删除后拥有该角色的用户将失去相关权限。`,
      '删除确认',
      { type: 'warning' }
    )
    await deleteRole(row.id)
    ElMessage.success('删除成功')
    await loadList()
  } catch (e) {
    if (e !== 'cancel' && e?.message) ElMessage.error('删除失败：' + e.message)
  }
}

// ===== 查看用户列表 =====
const userListVisible = ref(false)
const roleUserLoading = ref(false)
const roleUserList = ref([])
const roleUserTotal = ref(0)
const roleUserPage = ref(1)
const roleUserPageSize = ref(10)
const currentRoleForUsers = ref(null)

async function openUserListDialog(row) {
  currentRoleForUsers.value = row
  roleUserPage.value = 1
  userListVisible.value = true
  await loadRoleUsers()
}

async function loadRoleUsers() {
  if (!currentRoleForUsers.value) return
  roleUserLoading.value = true
  try {
    const data = await getRoleUsers(currentRoleForUsers.value.id, {
      page: roleUserPage.value,
      pageSize: roleUserPageSize.value
    })
    roleUserList.value = data?.list || data?.records || (Array.isArray(data) ? data : [])
    roleUserTotal.value = data?.total ?? roleUserList.value.length
  } catch (e) {
    console.warn('[AdminRole] 加载角色用户失败:', e.message)
  } finally {
    roleUserLoading.value = false
  }
}

// ===== 复制角色 =====
const copyVisible = ref(false)
const copySubmitting = ref(false)
const copySourceRole = ref(null)
const copyForm = reactive({ name: '', code: '' })

function openCopyDialog(row) {
  copySourceRole.value = row
  copyForm.name = row.name + '_副本'
  copyForm.code = row.code + '_copy'
  copyVisible.value = true
}

async function submitCopyRole() {
  if (!copyForm.name.trim()) {
    ElMessage.warning('请输入新角色名称')
    return
  }
  if (!copyForm.code.trim()) {
    ElMessage.warning('请输入新角色编码')
    return
  }
  copySubmitting.value = true
  try {
    await copyRole(copySourceRole.value.id, {
      name: copyForm.name.trim(),
      code: copyForm.code.trim()
    })
    ElMessage.success('角色复制成功')
    copyVisible.value = false
    await loadList()
  } catch (e) {
    ElMessage.error('角色复制失败：' + e.message)
  } finally {
    copySubmitting.value = false
  }
}

// ===== 菜单权限 =====
const menuPermVisible = ref(false)
const menuPermSubmitting = ref(false)
const menuPermRole = ref(null)
const menuTreeRef = ref(null)
const menuTree = ref([])
const menuSearchKeyword = ref('')
const permType = ref('menu')
const selectedMenu = ref(null)
const defaultMenuKeys = ref([])
const defaultExpandedMenuKeys = ref([])
const selectedButtonPerms = ref([])
const selectedApiPerms = ref([])

async function openMenuPermDialog(row) {
  menuPermRole.value = row
  menuPermVisible.value = true
  selectedMenu.value = null
  selectedButtonPerms.value = []
  selectedApiPerms.value = []

  // 加载菜单树
  try {
    const data = await getMenuTree()
    menuTree.value = Array.isArray(data) ? data : (Array.isArray(data?.list) ? data.list : [])
  } catch (e) {
    console.warn('[AdminRole] 菜单树加载失败:', e.message)
  }

  // 默认展开第一级
  defaultExpandedMenuKeys.value = menuTree.value.map(m => m.id)

  // 加载角色已有菜单权限
  try {
    const perms = await getRoleMenuPerms(row.id)
    defaultMenuKeys.value = perms?.menuIds || perms?.checkedKeys || []
  } catch (e) {
    console.warn('[AdminRole] 菜单权限加载失败:', e.message)
  }
}

// 按钮权限和接口权限
const buttonPerms = computed(() => {
  if (!selectedMenu.value?.children) return []
  return selectedMenu.value.children.filter(c => c.type === 'F' && c.code)
})

const apiPerms = computed(() => {
  if (!selectedMenu.value?.children) return []
  return selectedMenu.value.children.filter(c => c.type === 'F' && c.apiPath)
})

function handleMenuNodeClick(data) {
  selectedMenu.value = data
  // 同步当前菜单下按钮权限的选中状态
  if (data.children?.length) {
    const checked = menuTreeRef.value?.getCheckedKeys(false) || []
    const halfChecked = menuTreeRef.value?.getHalfCheckedKeys() || []
    const allChecked = [...checked, ...halfChecked]
    selectedButtonPerms.value = data.children
      .filter(c => c.type === 'F' && c.code && allChecked.includes(c.id))
      .map(c => c.id)
  } else {
    selectedButtonPerms.value = []
  }
}

function handleMenuCheck() {
  // 更新按钮权限选中状态
  if (selectedMenu.value?.children?.length) {
    const checked = menuTreeRef.value?.getCheckedKeys(false) || []
    selectedButtonPerms.value = selectedMenu.value.children
      .filter(c => c.type === 'F' && checked.includes(c.id))
      .map(c => c.id)
  }
}

// 搜索过滤
watch(menuSearchKeyword, (val) => {
  menuTreeRef.value?.filter(val)
})

function filterMenuNode(value, data) {
  if (!value) return true
  return data.name?.includes(value) || data.code?.includes(value)
}

// 展开/折叠
function expandAllMenu() {
  const expand = (nodes) => {
    nodes.forEach(n => {
      menuTreeRef.value?.store.nodesMap[n.id]?.expand()
      if (n.children?.length) expand(n.children)
    })
  }
  expand(menuTree.value)
}

function collapseAllMenu() {
  const collapse = (nodes) => {
    nodes.forEach(n => {
      menuTreeRef.value?.store.nodesMap[n.id]?.collapse()
      if (n.children?.length) collapse(n.children)
    })
  }
  collapse(menuTree.value)
}

// 全选/清空
function selectAllMenu() {
  const getAllIds = (nodes) => {
    let ids = []
    nodes.forEach(n => {
      ids.push(n.id)
      if (n.children?.length) ids = ids.concat(getAllIds(n.children))
    })
    return ids
  }
  const allIds = getAllIds(menuTree.value)
  menuTreeRef.value?.setCheckedKeys(allIds)
}

function clearAllMenu() {
  menuTreeRef.value?.setCheckedKeys([])
}

function resetMenuPerms() {
  menuTreeRef.value?.setCheckedKeys(defaultMenuKeys.value)
  ElMessage.info('已重置为保存前的状态')
}

async function submitMenuPerms() {
  menuPermSubmitting.value = true
  try {
    const checkedKeys = menuTreeRef.value?.getCheckedKeys(false) || []
    await assignRoleMenuPerms(menuPermRole.value.id, {
      menuIds: checkedKeys
    })
    ElMessage.success('菜单权限保存成功')
    menuPermVisible.value = false
  } catch (e) {
    ElMessage.error('保存失败：' + e.message)
  } finally {
    menuPermSubmitting.value = false
  }
}

// ===== 数据权限 =====
const dataPermVisible = ref(false)
const dataPermSubmitting = ref(false)
const dataPermRole = ref(null)
const dataScopeForm = ref('dept')
const dataPermDeptTreeRef = ref(null)
const customDeptCheckedKeys = ref([])
const deptDefaultExpanded = ref([])

const deptTree = ref([])

async function loadDeptTree() {
  try {
    const data = await getDeptTree()
    deptTree.value = Array.isArray(data) ? data : []
  } catch (e) {
    console.warn('[AdminRole] 部门树加载失败:', e.message)
  }
  deptDefaultExpanded.value = deptTree.value.map(d => d.id)
}

async function openDataPermDialog(row) {
  dataPermRole.value = row
  dataScopeForm.value = row.dataScope || 'dept'
  customDeptCheckedKeys.value = []

  try {
    const perms = await getRoleDataPerms(row.id)
    dataScopeForm.value = perms?.dataScope || row.dataScope || 'dept'
    customDeptCheckedKeys.value = perms?.deptIds || []
  } catch (e) {
    console.warn('[AdminRole] 数据权限加载失败:', e.message)
  }

  dataPermVisible.value = true
}

async function submitDataPerms() {
  dataPermSubmitting.value = true
  try {
    let deptIds = []
    if (dataScopeForm.value === 'custom' && dataPermDeptTreeRef.value) {
      deptIds = dataPermDeptTreeRef.value.getCheckedKeys(false)
    }
    await assignRoleDataPerms(dataPermRole.value.id, {
      dataScope: dataScopeForm.value,
      deptIds
    })
    ElMessage.success('数据权限保存成功')
    dataPermVisible.value = false
    await loadList()
  } catch (e) {
    ElMessage.error('保存失败：' + e.message)
  } finally {
    dataPermSubmitting.value = false
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
})
</script>

<style scoped>
.adm-role {
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

.toolbar-left {
  display: flex;
  gap: 8px;
  align-items: center;
}

.pagination-wrap {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

.role-name-cell {
  display: flex;
  align-items: center;
  font-weight: 500;
}

.form-tip {
  font-size: 12px;
  color: var(--text-quaternary);
  margin-left: 8px;
}

/* 菜单权限弹窗 */
.menu-perm-dialog :deep(.el-dialog__body) {
  padding-top: 0;
}

.menu-perm-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border-ghost);
  margin-bottom: 12px;
}

.perm-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.menu-perm-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  flex-wrap: wrap;
  gap: 8px;
}

.menu-perm-body {
  display: flex;
  gap: 12px;
  height: 480px;
}

.menu-tree-col {
  flex: 1.5;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-ghost);
  border-radius: 10px;
  overflow: hidden;
}

.button-perm-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-ghost);
  border-radius: 10px;
  overflow: hidden;
}

.col-title {
  padding: 10px 14px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  background: var(--bg-surface-2);
  border-bottom: 1px solid var(--border-ghost);
}

.tree-container {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.menu-tree-node {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
}

.menu-name {
  flex: 1;
}

.button-perm-container {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
}

.selected-menu-info {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  background: var(--brand-50);
  border-radius: 8px;
  margin-bottom: 12px;
}

.selected-menu-name {
  font-weight: 600;
  color: var(--brand-700);
  font-size: 14px;
}

.button-perm-list {
  margin-bottom: 16px;
}

.button-perm-item {
  padding: 8px 10px;
  border-radius: 6px;
  transition: background 0.15s;
}

.button-perm-item:hover {
  background: var(--bg-surface-2);
}

.btn-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}

.btn-code {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-left: 4px;
}

.api-perm-section {
  border-top: 1px solid var(--border-ghost);
  padding-top: 12px;
}

.sub-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.api-perm-item {
  padding: 6px 10px;
  border-radius: 6px;
  transition: background 0.15s;
}

.api-perm-item:hover {
  background: var(--bg-surface-2);
}

.api-path {
  font-size: 12px;
  color: var(--text-secondary);
  margin-left: 6px;
  font-family: Consolas, Monaco, monospace;
}

/* 数据权限 */
.scope-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  line-height: 1.4;
}

.scope-name {
  font-weight: 500;
  color: var(--text-primary);
  font-size: 13px;
}

.scope-desc {
  font-size: 12px;
  color: var(--text-tertiary);
}

.custom-dept-tree-wrapper {
  border: 1px solid var(--border-ghost);
  border-radius: 8px;
  padding: 10px;
  max-height: 260px;
  overflow-y: auto;
  width: 100%;
}
</style>
