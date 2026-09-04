<template>
  <div class="adm-menu">
    <div class="panel card-pad">
      <div class="toolbar">
        <div class="toolbar-left">
          <el-input
            v-model="searchKeyword"
            placeholder="搜索菜单名称 / 权限标识"
            clearable
            style="width: 260px"
            :prefix-icon="Search"
            @keyup.enter="handleSearch"
            @clear="handleSearch"
          />
          <el-button type="primary" :icon="Search" @click="handleSearch">搜索</el-button>
          <el-button :icon="Refresh" @click="resetSearch">重置</el-button>
        </div>
        <div class="toolbar-right">
          <el-button type="primary" :icon="Plus" @click="handleAdd(null)">新增菜单</el-button>
          <el-button :icon="Expand" @click="expandAll">展开全部</el-button>
          <el-button :icon="Fold" @click="collapseAll">折叠全部</el-button>
        </div>
      </div>

      <el-table
        ref="menuTableRef"
        :data="filteredMenuTree"
        v-loading="loading"
        row-key="id"
        default-expand-all
        :tree-props="{ children: 'children', hasChildren: 'hasChildren' }"
        stripe
        style="width: 100%"
      >
        <el-table-column prop="name" label="菜单名称" min-width="200">
          <template #default="{ row }">
            <el-icon v-if="row.icon" :size="16" style="vertical-align: -3px; margin-right: 6px; color: var(--text-2)">
              <component :is="row.icon" />
            </el-icon>
            <span>{{ row.name }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="icon" label="图标" width="80" align="center">
          <template #default="{ row }">
            <el-icon v-if="row.icon" :size="18" style="color: var(--text-2)">
              <component :is="row.icon" />
            </el-icon>
            <span v-else class="muted">-</span>
          </template>
        </el-table-column>
        <el-table-column prop="sort" label="排序" width="80" align="center" />
        <el-table-column prop="permission" label="权限标识" min-width="160">
          <template #default="{ row }">
            <span v-if="row.permission" class="mono">{{ row.permission }}</span>
            <span v-else class="muted">-</span>
          </template>
        </el-table-column>
        <el-table-column prop="component" label="组件路径" min-width="180">
          <template #default="{ row }">
            <span v-if="row.component" class="mono">{{ row.component }}</span>
            <span v-else class="muted">-</span>
          </template>
        </el-table-column>
        <el-table-column prop="path" label="路由路径" min-width="160">
          <template #default="{ row }">
            <span v-if="row.path" class="mono">{{ row.path }}</span>
            <span v-else class="muted">-</span>
          </template>
        </el-table-column>
        <el-table-column prop="type" label="类型" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="typeTagType(row.type)" size="small">{{ typeLabel(row.type) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="90" align="center">
          <template #default="{ row }">
            <el-switch
              :model-value="row.status === 1"
              disabled
              size="small"
            />
          </template>
        </el-table-column>
        <el-table-column prop="createdAt" label="创建时间" width="180">
          <template #default="{ row }">{{ fmtTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="220" fixed="right" align="center">
          <template #default="{ row }">
            <el-button type="primary" link size="small" :icon="Plus" @click="handleAdd(row)">新增子菜单</el-button>
            <el-button type="primary" link size="small" :icon="Edit" @click="handleEdit(row)">编辑</el-button>
            <el-button type="danger" link size="small" :icon="Delete" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <el-empty v-if="!loading && !filteredMenuTree.length" description="暂无菜单数据" />
    </div>

    <!-- 菜单表单对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="640px"
      :close-on-click-modal="false"
    >
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="100px">
        <el-form-item label="上级菜单" prop="parentId">
          <el-tree-select
            v-model="form.parentId"
            :data="menuTreeOptions"
            :props="{ label: 'name', value: 'id', children: 'children' }"
            check-strictly
            placeholder="顶级菜单"
            clearable
            style="width: 100%"
            :disabled="form.id != null && form.type === 'M'"
          />
        </el-form-item>
        <el-form-item label="菜单类型" prop="type">
          <el-radio-group v-model="form.type">
            <el-radio value="M">目录</el-radio>
            <el-radio value="C">菜单</el-radio>
            <el-radio value="F">按钮</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="菜单名称" prop="name">
          <el-input v-model="form.name" placeholder="请输入菜单名称" maxlength="50" show-word-limit />
        </el-form-item>
        <el-form-item v-if="form.type !== 'F'" label="图标" prop="icon">
          <el-input v-model="form.icon" placeholder="请输入图标名称，如 Menu" maxlength="50" />
          <div class="form-tip">填写 Element Plus 图标名称，例如：Menu、Setting、User 等</div>
        </el-form-item>
        <el-form-item label="排序号" prop="sort">
          <el-input-number v-model="form.sort" :min="0" :max="999" controls-position="right" />
        </el-form-item>
        <el-form-item v-if="form.type === 'F'" label="权限标识" prop="permission">
          <el-input v-model="form.permission" placeholder="如 system:user:list" maxlength="100" />
          <div class="form-tip">按钮权限标识，用于接口权限校验</div>
        </el-form-item>
        <el-form-item v-if="form.type !== 'F'" label="权限标识" prop="permission">
          <el-input v-model="form.permission" placeholder="如 system:user:list（选填）" maxlength="100" />
        </el-form-item>
        <el-form-item v-if="form.type === 'C'" label="路由路径" prop="path">
          <el-input v-model="form.path" placeholder="如 system/user" maxlength="200" />
        </el-form-item>
        <el-form-item v-if="form.type === 'C'" label="组件路径" prop="component">
          <el-input v-model="form.component" placeholder="如 system/user/index" maxlength="200" />
        </el-form-item>
        <el-form-item v-if="form.type === 'C'" label="路由参数" prop="query">
          <el-input
            v-model="form.query"
            type="textarea"
            :rows="2"
            placeholder='JSON 格式，如 {"id": "1"}'
            maxlength="500"
          />
        </el-form-item>
        <el-form-item v-if="form.type === 'C'" label="是否外链" prop="isFrame">
          <el-radio-group v-model="form.isFrame">
            <el-radio :value="1">是</el-radio>
            <el-radio :value="0">否</el-radio>
          </el-radio-group>
          <div class="form-tip">外链菜单将在新窗口打开</div>
        </el-form-item>
        <el-form-item v-if="form.type === 'C'" label="是否缓存" prop="isCache">
          <el-radio-group v-model="form.isCache">
            <el-radio :value="1">是</el-radio>
            <el-radio :value="0">否</el-radio>
          </el-radio-group>
          <div class="form-tip">启用后使用 keep-alive 缓存页面</div>
        </el-form-item>
        <el-form-item v-if="form.type !== 'F'" label="是否显示" prop="visible">
          <el-radio-group v-model="form.visible">
            <el-radio :value="1">是</el-radio>
            <el-radio :value="0">否</el-radio>
          </el-radio-group>
          <div class="form-tip">隐藏后菜单不显示但仍可通过路由访问</div>
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-radio-group v-model="form.status">
            <el-radio :value="1">启用</el-radio>
            <el-radio :value="0">停用</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="form.remark" type="textarea" :rows="2" placeholder="请输入备注" maxlength="200" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Refresh, Plus, Edit, Delete, Expand, Fold } from '@element-plus/icons-vue'
import { getMenuTree, createMenu, updateMenu, deleteMenu } from '@/api'

const loading = ref(false)
const submitting = ref(false)
const searchKeyword = ref('')
const menuTree = ref([])
const menuTableRef = ref(null)

const dialogVisible = ref(false)
const dialogMode = ref('add')
const dialogTitle = computed(() => dialogMode.value === 'add' ? '新增菜单' : '编辑菜单')

const formRef = ref(null)
const form = reactive({
  id: null,
  parentId: null,
  type: 'M',
  name: '',
  icon: '',
  sort: 0,
  permission: '',
  path: '',
  component: '',
  query: '',
  isFrame: 0,
  isCache: 0,
  visible: 1,
  status: 1,
  remark: ''
})

const formRules = {
  name: [{ required: true, message: '请输入菜单名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择菜单类型', trigger: 'change' }],
  sort: [{ required: true, message: '请输入排序号', trigger: 'blur' }]
}

// 过滤后的菜单树
const filteredMenuTree = computed(() => {
  const keyword = searchKeyword.value.trim().toLowerCase()
  if (!keyword) return menuTree.value
  return filterMenuTree(menuTree.value, keyword)
})

// 菜单树选项（用于上级菜单选择，排除当前节点及其子节点）
const menuTreeOptions = computed(() => {
  if (!form.id) return [{ id: null, name: '顶级菜单', children: menuTree.value }]
  const filtered = excludeNodeAndChildren(menuTree.value, form.id)
  return [{ id: null, name: '顶级菜单', children: filtered }]
})

function filterMenuTree(tree, keyword) {
  const result = []
  for (const node of tree) {
    const nameMatch = node.name?.toLowerCase().includes(keyword)
    const permMatch = node.permission?.toLowerCase().includes(keyword)
    const children = node.children?.length ? filterMenuTree(node.children, keyword) : []
    if (nameMatch || permMatch || children.length) {
      result.push({ ...node, children })
    }
  }
  return result
}

function excludeNodeAndChildren(tree, excludeId) {
  const result = []
  for (const node of tree) {
    if (node.id === excludeId) continue
    const children = node.children?.length ? excludeNodeAndChildren(node.children, excludeId) : []
    result.push({ ...node, children })
  }
  return result
}

function typeLabel(type) {
  const map = { M: '目录', C: '菜单', F: '按钮' }
  return map[type] || type
}

function typeTagType(type) {
  const map = { M: 'primary', C: 'success', F: 'info' }
  return map[type] || 'info'
}

function fmtTime(t) {
  if (!t) return '-'
  try { return new Date(t).toLocaleString() } catch { return String(t) }
}

async function loadMenuTree() {
  loading.value = true
  try {
    const data = await getMenuTree()
    menuTree.value = Array.isArray(data) ? data : (Array.isArray(data?.list) ? data.list : [])
  } catch (e) {
    ElMessage.error('菜单树加载失败: ' + (e?.message || e))
  } finally {
    loading.value = false
  }
}

function handleSearch() {
  // 搜索通过 computed 过滤实现
}

function resetSearch() {
  searchKeyword.value = ''
}

function expandAll() {
  const rows = menuTableRef.value?.store?.states?.rows || []
  rows.forEach(row => {
    menuTableRef.value?.toggleRowExpansion(row, true)
  })
}

function collapseAll() {
  const rows = menuTableRef.value?.store?.states?.rows || []
  rows.forEach(row => {
    menuTableRef.value?.toggleRowExpansion(row, false)
  })
}

function resetForm() {
  Object.assign(form, {
    id: null,
    parentId: null,
    type: 'M',
    name: '',
    icon: '',
    sort: 0,
    permission: '',
    path: '',
    component: '',
    query: '',
    isFrame: 0,
    isCache: 0,
    visible: 1,
    status: 1,
    remark: ''
  })
  formRef.value?.clearValidate()
}

function handleAdd(row) {
  resetForm()
  dialogMode.value = 'add'
  if (row) {
    form.parentId = row.id
    form.type = row.type === 'M' ? 'C' : 'F'
  } else {
    form.parentId = null
    form.type = 'M'
  }
  dialogVisible.value = true
}

function handleEdit(row) {
  resetForm()
  dialogMode.value = 'edit'
  Object.assign(form, {
    id: row.id,
    parentId: row.parentId || null,
    type: row.type || 'C',
    name: row.name || '',
    icon: row.icon || '',
    sort: row.sort ?? 0,
    permission: row.permission || '',
    path: row.path || '',
    component: row.component || '',
    query: row.query || '',
    isFrame: row.isFrame ?? 0,
    isCache: row.isCache ?? 0,
    visible: row.visible ?? 1,
    status: row.status ?? 1,
    remark: row.remark || ''
  })
  nextTick(() => {
    dialogVisible.value = true
  })
}

async function handleSubmit() {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
  } catch {
    return
  }
  submitting.value = true
  try {
    const payload = { ...form }
    if (dialogMode.value === 'add') {
      await createMenu(payload)
      ElMessage.success('新增成功')
    } else {
      await updateMenu(form.id, payload)
      ElMessage.success('修改成功')
    }
    dialogVisible.value = false
    await loadMenuTree()
  } catch (e) {
    ElMessage.error((dialogMode.value === 'add' ? '新增失败：' : '修改失败：') + e.message)
  } finally {
    submitting.value = false
  }
}

function hasChildren(row) {
  return row.children && row.children.length > 0
}

async function handleDelete(row) {
  if (hasChildren(row)) {
    ElMessage.warning('存在子菜单，请先删除子菜单')
    return
  }
  try {
    await ElMessageBox.confirm(
      `确定删除菜单「${row.name}」吗？删除后不可恢复。`,
      '删除确认',
      { type: 'warning' }
    )
    await deleteMenu(row.id)
    ElMessage.success('删除成功')
    await loadMenuTree()
  } catch (e) {
    if (e !== 'cancel' && e?.message) ElMessage.error('删除失败：' + e.message)
  }
}

onMounted(loadMenuTree)
</script>

<style scoped>
.toolbar { display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px; flex-wrap: wrap; gap: 10px; }
.toolbar-left { display: flex; gap: 8px; align-items: center; }
.toolbar-right { display: flex; gap: 8px; }
.mono {
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
  color: var(--text-2);
  word-break: break-all;
}
.muted { color: var(--text-3); }
.form-tip { font-size: 12px; color: var(--text-3); margin-top: 4px; }
</style>
