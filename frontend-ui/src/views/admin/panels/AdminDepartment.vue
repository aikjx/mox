<template>
  <div class="adm-dept">
    <!-- 左侧：部门树 -->
    <div class="dept-tree-panel panel card-pad">
      <div class="tree-header">
        <h3 class="section-title">组织架构</h3>
        <div class="tree-actions">
          <el-input
            v-model="searchKeyword"
            placeholder="搜索部门"
            :prefix-icon="Search"
            clearable
            size="small"
            style="width: 180px"
          />
          <el-button :icon="Plus" type="primary" size="small" @click="handleAddRoot">新增</el-button>
        </div>
      </div>
      <div class="tree-toolbar">
        <el-button link size="small" :icon="ArrowDown" @click="expandAll">展开全部</el-button>
        <el-button link size="small" :icon="ArrowUp" @click="collapseAll">折叠全部</el-button>
      </div>
      <div class="tree-wrapper" v-loading="treeLoading">
        <el-tree
          ref="deptTreeRef"
          :data="deptTree"
          :props="treeProps"
          node-key="id"
          :expand-on-click-node="false"
          :default-expanded-keys="defaultExpandedKeys"
          :highlight-current="true"
          draggable
          :filter-node-method="filterNode"
          @node-click="handleNodeClick"
          @node-drop="handleNodeDrop"
        >
          <template #default="{ node, data }">
            <div class="tree-node" :class="{ 'is-current': currentDeptId === data.id }">
              <span class="node-icon">
                <el-icon :size="14"><component :is="data.children?.length ? 'OfficeBuilding' : 'User'" /></el-icon>
              </span>
              <span class="node-label">{{ data.name }}</span>
              <span class="node-count">({{ data.userCount || 0 }})</span>
              <span class="node-ops">
                <el-dropdown trigger="click" @command="(cmd) => handleTreeCommand(cmd, data)">
                  <el-icon class="more-icon" :size="14"><MoreFilled /></el-icon>
                  <template #dropdown>
                    <el-dropdown-menu>
                      <el-dropdown-item command="add"><el-icon><Plus /></el-icon>新增子部门</el-dropdown-item>
                      <el-dropdown-item command="edit"><el-icon><Edit /></el-icon>编辑</el-dropdown-item>
                      <el-dropdown-item command="delete" divided><el-icon><Delete /></el-icon>删除</el-dropdown-item>
                    </el-dropdown-menu>
                  </template>
                </el-dropdown>
              </span>
            </div>
          </template>
        </el-tree>
        <el-empty v-if="!treeLoading && !deptTree.length" description="暂无部门数据" :image-size="60" />
      </div>
    </div>

    <!-- 右侧：部门详情 -->
    <div class="dept-detail-panel">
      <div v-if="currentDept" class="panel card-pad" style="margin-bottom: 16px">
        <div class="detail-header">
          <div>
            <h3 class="section-title">{{ currentDept.name }}</h3>
            <div class="dept-meta">
              <span class="badge" :class="currentDept.status === 1 ? 'success' : 'warning'">
                {{ currentDept.status === 1 ? '启用' : '停用' }}
              </span>
              <span class="meta-item">编码：{{ currentDept.code }}</span>
              <span class="meta-item">人数：{{ currentDept.userCount || 0 }} 人</span>
            </div>
          </div>
          <div class="detail-actions">
            <el-button :icon="Edit" @click="handleEdit(currentDept)">编辑部门</el-button>
          </div>
        </div>
      </div>

      <el-tabs v-model="activeTab" class="detail-tabs">
        <!-- 基本信息 -->
        <el-tab-pane label="基本信息" name="info">
          <div class="panel card-pad">
            <h3 class="section-title">基本信息</h3>
            <el-descriptions :column="2" border>
              <el-descriptions-item label="部门名称">{{ currentDept?.name || '-' }}</el-descriptions-item>
              <el-descriptions-item label="部门编码">{{ currentDept?.code || '-' }}</el-descriptions-item>
              <el-descriptions-item label="上级部门">{{ currentDept?.parentName || '顶级部门' }}</el-descriptions-item>
              <el-descriptions-item label="负责人">{{ currentDept?.leaderName || '-' }}</el-descriptions-item>
              <el-descriptions-item label="联系电话">{{ currentDept?.phone || '-' }}</el-descriptions-item>
              <el-descriptions-item label="邮箱">{{ currentDept?.email || '-' }}</el-descriptions-item>
              <el-descriptions-item label="排序号">{{ currentDept?.sort ?? '-' }}</el-descriptions-item>
              <el-descriptions-item label="状态">
                <el-tag :type="currentDept?.status === 1 ? 'success' : 'info'" size="small">
                  {{ currentDept?.status === 1 ? '启用' : '停用' }}
                </el-tag>
              </el-descriptions-item>
              <el-descriptions-item label="创建时间">{{ formatTime(currentDept?.createdAt) }}</el-descriptions-item>
              <el-descriptions-item label="备注" :span="2">{{ currentDept?.remark || '-' }}</el-descriptions-item>
            </el-descriptions>
          </div>
        </el-tab-pane>

        <!-- 岗位管理 -->
        <el-tab-pane label="岗位管理" name="post">
          <div class="panel card-pad">
            <div class="toolbar">
              <div class="toolbar-left">
                <span class="badge info">共 {{ postList.length }} 个岗位</span>
              </div>
              <div class="toolbar-right">
                <el-button :icon="Refresh" size="small" :loading="postLoading" @click="loadPosts">刷新</el-button>
                <el-button type="primary" :icon="Plus" size="small" @click="openPostForm()">新增岗位</el-button>
              </div>
            </div>
            <el-table :data="postList" v-loading="postLoading" stripe style="width: 100%">
              <el-table-column prop="name" label="岗位名称" min-width="160" />
              <el-table-column prop="code" label="岗位编码" min-width="140" />
              <el-table-column prop="sort" label="排序" width="80" align="center" />
              <el-table-column label="状态" width="100" align="center">
                <template #default="{ row }">
                  <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
                    {{ row.status === 1 ? '启用' : '停用' }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="createdAt" label="创建时间" width="180">
                <template #default="{ row }">{{ formatTime(row.createdAt) }}</template>
              </el-table-column>
              <el-table-column label="操作" width="160" fixed="right" align="center">
                <template #default="{ row }">
                  <el-button type="primary" link size="small" @click="openPostForm(row)">编辑</el-button>
                  <el-button type="danger" link size="small" @click="handleDeletePost(row)">删除</el-button>
                </template>
              </el-table-column>
            </el-table>
          </div>
        </el-tab-pane>

        <!-- 人员列表 -->
        <el-tab-pane label="人员列表" name="users">
          <div class="panel card-pad">
            <div class="toolbar">
              <div class="toolbar-left">
                <span class="badge info">共 {{ userTotal }} 人</span>
              </div>
              <div class="toolbar-right">
                <el-button :icon="User" type="primary" size="small" @click="goToUserManage">
                  跳转用户管理
                </el-button>
              </div>
            </div>
            <el-table :data="userList" v-loading="userLoading" stripe style="width: 100%">
              <el-table-column label="头像" width="60" align="center">
                <template #default="{ row }">
                  <el-avatar :size="32" :src="row.avatar">{{ row.nickname?.charAt(0) || row.username?.charAt(0) }}</el-avatar>
                </template>
              </el-table-column>
              <el-table-column prop="username" label="用户名" min-width="120" />
              <el-table-column prop="nickname" label="昵称" min-width="120" />
              <el-table-column prop="postName" label="岗位" min-width="120" />
              <el-table-column prop="phone" label="手机号" width="130" />
              <el-table-column prop="email" label="邮箱" min-width="180" />
              <el-table-column label="状态" width="80" align="center">
                <template #default="{ row }">
                  <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
                    {{ row.status === 1 ? '启用' : '停用' }}
                  </el-tag>
                </template>
              </el-table-column>
            </el-table>
            <div class="pagination-wrap" v-if="userTotal > 0">
              <el-pagination
                v-model:current-page="userPage"
                v-model:page-size="userPageSize"
                :total="userTotal"
                :page-sizes="[10, 20, 50]"
                layout="total, sizes, prev, pager, next, jumper"
                background
                @size-change="loadUsers"
                @current-change="loadUsers"
              />
            </div>
          </div>
        </el-tab-pane>
      </el-tabs>

      <div v-else class="panel card-pad empty-detail">
        <el-empty description="请选择左侧部门查看详情" :image-size="100">
          <template #image>
            <el-icon :size="64" color="var(--text-quaternary)"><OfficeBuilding /></el-icon>
          </template>
        </el-empty>
      </div>
    </div>

    <!-- 部门表单对话框 -->
    <el-dialog v-model="deptFormVisible" :title="deptForm.id ? '编辑部门' : '新增部门'" width="560px" destroy-on-close>
      <el-form ref="deptFormRef" :model="deptForm" :rules="deptFormRules" label-width="90px">
        <el-form-item label="部门名称" prop="name">
          <el-input v-model="deptForm.name" placeholder="请输入部门名称" maxlength="64" show-word-limit />
        </el-form-item>
        <el-form-item label="部门编码" prop="code">
          <el-input v-model="deptForm.code" placeholder="请输入部门编码" maxlength="32" show-word-limit />
        </el-form-item>
        <el-form-item label="上级部门" prop="parentId">
          <el-tree-select
            v-model="deptForm.parentId"
            :data="deptTreeForSelect"
            :props="{ label: 'name', value: 'id', children: 'children' }"
            node-key="id"
            check-strictly
            :render-after-expand="false"
            placeholder="请选择上级部门（不选为顶级）"
            clearable
            filterable
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="负责人" prop="leaderId">
          <el-select
            v-model="deptForm.leaderId"
            placeholder="请选择负责人"
            filterable
            clearable
            remote
            :remote-method="searchUsers"
            :loading="userSearchLoading"
            style="width: 100%"
          >
            <el-option v-for="u in userOptions" :key="u.id" :label="`${u.nickname || u.username} (${u.username})`" :value="u.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="联系电话" prop="phone">
          <el-input v-model="deptForm.phone" placeholder="请输入联系电话" maxlength="20" />
        </el-form-item>
        <el-form-item label="邮箱" prop="email">
          <el-input v-model="deptForm.email" placeholder="请输入邮箱" maxlength="128" />
        </el-form-item>
        <el-form-item label="排序号" prop="sort">
          <el-input-number v-model="deptForm.sort" :min="0" :max="999" style="width: 120px" />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-radio-group v-model="deptForm.status">
            <el-radio :value="1">启用</el-radio>
            <el-radio :value="0">停用</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="deptForm.remark" type="textarea" :rows="3" maxlength="255" show-word-limit />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="deptFormVisible = false">取消</el-button>
        <el-button type="primary" :loading="deptFormSubmitting" @click="submitDeptForm">确定</el-button>
      </template>
    </el-dialog>

    <!-- 岗位表单对话框 -->
    <el-dialog v-model="postFormVisible" :title="postForm.id ? '编辑岗位' : '新增岗位'" width="480px" destroy-on-close>
      <el-form ref="postFormRef" :model="postForm" :rules="postFormRules" label-width="90px">
        <el-form-item label="岗位名称" prop="name">
          <el-input v-model="postForm.name" placeholder="请输入岗位名称" maxlength="64" show-word-limit />
        </el-form-item>
        <el-form-item label="岗位编码" prop="code">
          <el-input v-model="postForm.code" placeholder="请输入岗位编码" maxlength="32" show-word-limit />
        </el-form-item>
        <el-form-item label="排序号" prop="sort">
          <el-input-number v-model="postForm.sort" :min="0" :max="999" style="width: 120px" />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-radio-group v-model="postForm.status">
            <el-radio :value="1">启用</el-radio>
            <el-radio :value="0">停用</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="postForm.remark" type="textarea" :rows="3" maxlength="255" show-word-limit />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="postFormVisible = false">取消</el-button>
        <el-button type="primary" :loading="postFormSubmitting" @click="submitPostForm">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, watch, onMounted, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Search, Plus, Edit, Delete, Refresh, User, OfficeBuilding,
  ArrowDown, ArrowUp, MoreFilled
} from '@element-plus/icons-vue'
import {
  getDeptTree, getDeptDetail, createDept, updateDept, deleteDept,
  getPostByDept, createPost, updatePost, deletePost,
  getDeptUserList, getUserList
} from '@/api'

// ===== 部门树 =====
const deptTreeRef = ref(null)
const treeLoading = ref(false)
const deptTree = ref([])
const searchKeyword = ref('')
const currentDeptId = ref(null)
const currentDept = ref(null)
const defaultExpandedKeys = ref([])
const activeTab = ref('info')

const treeProps = {
  label: 'name',
  children: 'children'
}

async function loadDeptTree() {
  treeLoading.value = true
  try {
    const data = await getDeptTree()
    deptTree.value = Array.isArray(data) ? data : (Array.isArray(data?.list) ? data.list : [])
  } catch (e) {
    console.warn('[AdminDepartment] 加载部门树失败:', e.message)
  } finally {
    treeLoading.value = false
  }
  // 默认展开第一级
  if (deptTree.value.length > 0) {
    defaultExpandedKeys.value = deptTree.value.map(d => d.id)
  }
}

// 搜索过滤
watch(searchKeyword, (val) => {
  deptTreeRef.value?.filter(val)
})

function filterNode(value, data) {
  if (!value) return true
  return data.name?.includes(value) || data.code?.includes(value)
}

// 展开/折叠
function expandAll() {
  const expand = (nodes) => {
    nodes.forEach(n => {
      deptTreeRef.value?.store.nodesMap[n.id]?.expand()
      if (n.children?.length) expand(n.children)
    })
  }
  expand(deptTree.value)
}

function collapseAll() {
  const collapse = (nodes) => {
    nodes.forEach(n => {
      deptTreeRef.value?.store.nodesMap[n.id]?.collapse()
      if (n.children?.length) collapse(n.children)
    })
  }
  collapse(deptTree.value)
}

// 节点点击
function handleNodeClick(data) {
  currentDeptId.value = data.id
  loadDeptDetail(data.id)
  loadPosts()
  loadUsers()
}

// 拖拽排序
async function handleNodeDrop(draggingNode, dropNode, dropType) {
  try {
    await updateDept(draggingNode.data.id, {
      parentId: dropType === 'inner' ? dropNode.data.id : (dropNode.data.parentId || 0),
      sort: draggingNode.data.sort
    })
    ElMessage.success('排序已更新')
    loadDeptTree()
  } catch (e) {
    ElMessage.warning('拖拽排序失败：' + e.message)
    loadDeptTree()
  }
}

// 树节点操作菜单
function handleTreeCommand(cmd, data) {
  if (cmd === 'add') {
    openDeptForm(null, data.id)
  } else if (cmd === 'edit') {
    openDeptForm(data)
  } else if (cmd === 'delete') {
    handleDelete(data)
  }
}

function handleAddRoot() {
  openDeptForm(null, 0)
}

// ===== 部门详情 =====
async function loadDeptDetail(id) {
  try {
    const data = await getDeptDetail(id)
    currentDept.value = data || null
  } catch (e) {
    // 从树中查找兜底
    const findInTree = (nodes) => {
      for (const n of nodes) {
        if (n.id === id) return n
        if (n.children?.length) {
          const found = findInTree(n.children)
          if (found) return found
        }
      }
      return null
    }
    currentDept.value = findInTree(deptTree.value) || null
  }
}

// 供上级选择的树（排除当前节点及其子节点）
const deptTreeForSelect = computed(() => {
  if (!deptForm.value.id) return deptTree.value
  const filterTree = (nodes) => {
    return nodes
      .filter(n => n.id !== deptForm.value.id)
      .map(n => ({
        ...n,
        children: n.children ? filterTree(n.children) : []
      }))
  }
  return filterTree(deptTree.value)
})

// ===== 部门表单 =====
const deptFormVisible = ref(false)
const deptFormRef = ref(null)
const deptFormSubmitting = ref(false)
const deptForm = reactive({
  id: null,
  name: '',
  code: '',
  parentId: null,
  leaderId: null,
  phone: '',
  email: '',
  sort: 0,
  status: 1,
  remark: ''
})

const deptFormRules = {
  name: [{ required: true, message: '请输入部门名称', trigger: 'blur' }],
  code: [{ required: true, message: '请输入部门编码', trigger: 'blur' }]
}

function openDeptForm(row = null, parentId = null) {
  if (row) {
    Object.assign(deptForm, row)
  } else {
    Object.assign(deptForm, {
      id: null, name: '', code: '', parentId: parentId || null,
      leaderId: null, phone: '', email: '', sort: 0, status: 1, remark: ''
    })
  }
  deptFormVisible.value = true
  nextTick(() => {
    deptFormRef.value?.clearValidate()
  })
}

function handleEdit(row) {
  openDeptForm(row)
}

async function submitDeptForm() {
  try {
    await deptFormRef.value.validate()
  } catch { return }

  deptFormSubmitting.value = true
  try {
    const payload = { ...deptForm }
    if (deptForm.id) {
      await updateDept(deptForm.id, payload)
      ElMessage.success('部门更新成功')
    } else {
      await createDept(payload)
      ElMessage.success('部门创建成功')
    }
    deptFormVisible.value = false
    await loadDeptTree()
    if (currentDeptId.value) {
      loadDeptDetail(currentDeptId.value)
    }
  } catch (e) {
    ElMessage.error((deptForm.id ? '更新' : '创建') + '失败：' + e.message)
  } finally {
    deptFormSubmitting.value = false
  }
}

async function handleDelete(row) {
  try {
    await ElMessageBox.confirm(
      `确定删除部门「${row.name}」吗？删除后该部门下的子部门和数据将被处理。`,
      '删除确认',
      { type: 'warning' }
    )
    await deleteDept(row.id)
    ElMessage.success('删除成功')
    if (currentDeptId.value === row.id) {
      currentDeptId.value = null
      currentDept.value = null
    }
    await loadDeptTree()
  } catch (e) {
    if (e !== 'cancel' && e?.message) ElMessage.error('删除失败：' + e.message)
  }
}

// ===== 岗位管理 =====
const postLoading = ref(false)
const postList = ref([])
const postFormVisible = ref(false)
const postFormRef = ref(null)
const postFormSubmitting = ref(false)
const postForm = reactive({
  id: null, name: '', code: '', sort: 0, status: 1, remark: '', deptId: null
})

const postFormRules = {
  name: [{ required: true, message: '请输入岗位名称', trigger: 'blur' }],
  code: [{ required: true, message: '请输入岗位编码', trigger: 'blur' }]
}

async function loadPosts() {
  if (!currentDeptId.value) return
  postLoading.value = true
  try {
    const data = await getPostByDept(currentDeptId.value)
    postList.value = Array.isArray(data) ? data : (Array.isArray(data?.list) ? data.list : [])
  } catch (e) {
    console.warn('[AdminDepartment] 加载岗位列表失败:', e.message)
  } finally {
    postLoading.value = false
  }
}

function openPostForm(row = null) {
  if (row) {
    Object.assign(postForm, row)
  } else {
    Object.assign(postForm, {
      id: null, name: '', code: '', sort: 0, status: 1, remark: '', deptId: currentDeptId.value
    })
  }
  postFormVisible.value = true
  nextTick(() => {
    postFormRef.value?.clearValidate()
  })
}

async function submitPostForm() {
  try {
    await postFormRef.value.validate()
  } catch { return }

  postFormSubmitting.value = true
  try {
    const payload = { ...postForm, deptId: currentDeptId.value }
    if (postForm.id) {
      await updatePost(postForm.id, payload)
      ElMessage.success('岗位更新成功')
    } else {
      await createPost(payload)
      ElMessage.success('岗位创建成功')
    }
    postFormVisible.value = false
    await loadPosts()
  } catch (e) {
    ElMessage.error((postForm.id ? '更新' : '创建') + '失败：' + e.message)
  } finally {
    postFormSubmitting.value = false
  }
}

async function handleDeletePost(row) {
  try {
    await ElMessageBox.confirm(
      `确定删除岗位「${row.name}」吗？`,
      '删除确认',
      { type: 'warning' }
    )
    await deletePost(row.id)
    ElMessage.success('删除成功')
    await loadPosts()
  } catch (e) {
    if (e !== 'cancel' && e?.message) ElMessage.error('删除失败：' + e.message)
  }
}

// ===== 人员列表 =====
const userLoading = ref(false)
const userList = ref([])
const userTotal = ref(0)
const userPage = ref(1)
const userPageSize = ref(10)

async function loadUsers() {
  if (!currentDeptId.value) return
  userLoading.value = true
  try {
    const data = await getDeptUserList(currentDeptId.value, {
      page: userPage.value,
      pageSize: userPageSize.value
    })
    userList.value = data?.list || data?.records || Array.isArray(data) ? data : []
    userTotal.value = data?.total || userList.value.length
  } catch (e) {
    console.warn('[AdminDepartment] 加载用户列表失败:', e.message)
  } finally {
    userLoading.value = false
  }
}

function goToUserManage() {
  // 跳转到用户管理并携带部门筛选参数
  window.dispatchEvent(new CustomEvent('admin:navigate-user', { detail: { deptId: currentDeptId.value } }))
  ElMessage.info('请在用户管理中查看完整列表')
}

// ===== 用户选择（负责人） =====
const userSearchLoading = ref(false)
const userOptions = ref([])

async function searchUsers(keyword) {
  if (!keyword) {
    userOptions.value = []
    return
  }
  userSearchLoading.value = true
  try {
    const data = await getUserList({ keyword, pageSize: 20 })
    userOptions.value = data?.list || data?.records || Array.isArray(data) ? data : []
  } catch (e) {
    console.warn('[AdminDepartment] 用户搜索失败:', e.message)
  } finally {
    userSearchLoading.value = false
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
  loadDeptTree()
})
</script>

<style scoped>
.adm-dept {
  display: flex;
  gap: 16px;
  height: calc(100vh - 220px);
  min-height: 500px;
}

.dept-tree-panel {
  width: 320px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.tree-header {
  margin-bottom: 10px;
}

.tree-header :deep(.section-title) {
  margin-bottom: 10px;
}

.tree-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.tree-toolbar {
  display: flex;
  gap: 4px;
  padding: 4px 0 8px;
  border-bottom: 1px solid var(--border-ghost);
  margin-bottom: 8px;
}

.tree-wrapper {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.tree-node {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  padding: 4px 0;
  font-size: 13px;
}

.tree-node.is-current {
  color: var(--brand-600);
  font-weight: 600;
}

.node-icon {
  color: var(--text-tertiary);
  flex-shrink: 0;
}

.node-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-count {
  color: var(--text-quaternary);
  font-size: 12px;
  flex-shrink: 0;
}

.node-ops {
  opacity: 0;
  transition: opacity 0.2s;
  flex-shrink: 0;
}

.tree-node:hover .node-ops {
  opacity: 1;
}

.more-icon {
  cursor: pointer;
  color: var(--text-tertiary);
  padding: 2px;
  border-radius: 4px;
}

.more-icon:hover {
  color: var(--brand-600);
  background: var(--brand-50);
}

.dept-detail-panel {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.detail-tabs {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.detail-tabs :deep(.el-tabs__content) {
  flex: 1;
  overflow-y: auto;
}

.detail-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.dept-meta {
  display: flex;
  gap: 12px;
  align-items: center;
  flex-wrap: wrap;
  margin-top: 6px;
}

.meta-item {
  font-size: 13px;
  color: var(--text-tertiary);
}

.detail-actions {
  display: flex;
  gap: 8px;
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

.empty-detail {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.pagination-wrap {
  display: flex;
  justify-content: flex-end;
  margin-top: 14px;
}
</style>
