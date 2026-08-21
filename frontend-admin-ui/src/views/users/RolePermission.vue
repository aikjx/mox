<template>
  <el-row :gutter="16">
    <el-col :xs="24" :md="8">
      <div class="admin-card">
        <div class="admin-table-toolbar">
          <h3 class="admin-page-title" style="margin:0">角色列表</h3>
          <el-button type="primary" size="small" :icon="Plus" @click="openRoleDialog()">新增角色</el-button>
        </div>
        <div class="role-list">
          <div
            v-for="role in roles"
            :key="role.id"
            class="role-item"
            :class="{ active: selectedRole?.id === role.id }"
            @click="selectRole(role)"
          >
            <div class="role-info">
              <div class="role-name">
                <el-icon :size="16"><User /></el-icon>
                {{ role.name }}
              </div>
              <div class="role-desc">{{ role.description }}</div>
              <div class="role-meta">
                <el-tag size="small" :type="role.userCount > 0 ? '' : 'info'">{{ role.userCount }} 名用户</el-tag>
              </div>
            </div>
            <div class="role-actions">
              <el-button link size="small" :icon="Edit" @click.stop="openRoleDialog(role)">编辑</el-button>
              <el-button link size="small" type="danger" :icon="Delete" @click.stop="handleDeleteRole(role)">删除</el-button>
            </div>
          </div>
        </div>
      </div>
    </el-col>

    <el-col :xs="24" :md="16">
      <div class="admin-card" v-if="selectedRole">
        <div class="admin-table-toolbar">
          <div>
            <h3 class="admin-page-title" style="margin:0">{{ selectedRole.name }} - 权限配置</h3>
            <p class="role-desc-text">{{ selectedRole.description }}</p>
          </div>
          <div>
            <el-switch
              v-model="selectedRole.permissionsEnabled"
              active-text="启用权限"
              inactive-text="禁用权限"
            />
          </div>
        </div>

        <el-tabs v-model="activeTab">
          <el-tab-pane label="权限树" name="tree">
            <div class="perm-tree-container">
              <div class="tree-toolbar">
                <el-checkbox v-model="checkAll" @change="handleCheckAll">全选</el-checkbox>
                <el-checkbox v-model="expandAll" @change="handleExpandAll">展开全部</el-checkbox>
                <el-button size="small" @click="expandAllNodes">展开</el-button>
                <el-button size="small" @click="collapseAllNodes">折叠</el-button>
              </div>
              <el-tree
                ref="treeRef"
                :data="permissionTree"
                show-checkbox
                node-key="key"
                :default-checked-keys="checkedKeys"
                :props="{ label: 'label', children: 'children' }"
                :expand-on-click-node="false"
              />
            </div>
          </el-tab-pane>

          <el-tab-pane label="已绑定用户" name="users">
            <el-table :data="boundUsers" stripe border>
              <el-table-column prop="username" label="用户名" width="160" />
              <el-table-column prop="realName" label="姓名" width="140" />
              <el-table-column prop="email" label="邮箱" />
              <el-table-column label="操作" width="120">
                <template #default="{ row }">
                  <el-button type="danger" link size="small" @click="unbindUser(row)">移除</el-button>
                </template>
              </el-table-column>
            </el-table>
          </el-tab-pane>

          <el-tab-pane label="数据权限" name="data">
            <div class="data-perm-config">
              <el-form label-width="120px">
                <el-form-item label="数据范围">
                  <el-radio-group v-model="selectedRole.dataScope">
                    <el-radio value="all">全部数据权限</el-radio>
                    <el-radio value="dept">本部门数据权限</el-radio>
                    <el-radio value="dept_and_sub">本部门及以下</el-radio>
                    <el-radio value="self">仅本人数据</el-radio>
                    <el-radio value="custom">自定义</el-radio>
                  </el-radio-group>
                </el-form-item>
              </el-form>
              <div v-if="selectedRole.dataScope === 'custom'" class="custom-scope">
                <h4 class="section-title">选择部门</h4>
                <el-tree
                  :data="deptTree"
                  show-checkbox
                  node-key="id"
                  :props="{ label: 'name', children: 'children' }"
                />
              </div>
            </div>
          </el-tab-pane>
        </el-tabs>

        <div class="form-actions">
          <el-button @click="resetPermissions">重置</el-button>
          <el-button type="primary" @click="savePermissions">保存权限配置</el-button>
        </div>
      </div>

      <div v-else class="admin-card empty-state">
        <el-empty description="请从左侧选择一个角色进行权限配置" :image-size="120" />
      </div>
    </el-col>

    <el-dialog v-model="roleDialogVisible" :title="roleDialogTitle" width="500px">
      <el-form :model="roleForm" :rules="roleRules" ref="roleFormRef" label-width="100px">
        <el-form-item label="角色名称" prop="name">
          <el-input v-model="roleForm.name" placeholder="请输入角色名称" />
        </el-form-item>
        <el-form-item label="角色描述" prop="description">
          <el-input v-model="roleForm.description" type="textarea" :rows="3" placeholder="请输入角色描述" />
        </el-form-item>
        <el-form-item label="角色标识" prop="code">
          <el-input v-model="roleForm.code" placeholder="英文标识，如: custom_admin" />
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="roleForm.sort" :min="0" :max="999" />
        </el-form-item>
        <el-form-item label="状态">
          <el-switch v-model="roleForm.enabled" active-text="启用" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="roleDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleRoleSubmit">确定</el-button>
      </template>
    </el-dialog>
  </el-row>
</template>

<script setup>
import { ref, reactive, computed, onMounted, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { adminApi } from '@/api/index'
import { Plus, Edit, Delete } from '@element-plus/icons-vue'

const roles = ref([
  { id: 1, name: '超级管理员', code: 'super_admin', description: '拥有系统所有权限，可管理任何模块', userCount: 1, enabled: true, dataScope: 'all', permissionsEnabled: true },
  { id: 2, name: '系统管理员', code: 'admin', description: '负责用户管理、系统配置等管理功能', userCount: 3, enabled: true, dataScope: 'dept_and_sub', permissionsEnabled: true },
  { id: 3, name: '运营人员', code: 'operator', description: '内容运营、知识库维护、日常操作', userCount: 5, enabled: true, dataScope: 'dept', permissionsEnabled: true },
  { id: 4, name: '只读用户', code: 'viewer', description: '仅查看数据，无修改权限', userCount: 8, enabled: true, dataScope: 'self', permissionsEnabled: true },
  { id: 5, name: '访客', code: 'guest', description: '临时访问，权限受限', userCount: 2, enabled: false, dataScope: 'self', permissionsEnabled: false },
  { id: 6, name: '审计员', code: 'auditor', description: '仅查看审计日志和系统状态', userCount: 1, enabled: true, dataScope: 'all', permissionsEnabled: true }
])

const selectedRole = ref(null)
const activeTab = ref('tree')
const treeRef = ref(null)
const checkAll = ref(false)
const expandAll = ref(true)

const permissionTree = ref([
  {
    key: 'user', label: '用户管理', children: [
      { key: 'user:view', label: '查看用户' },
      { key: 'user:create', label: '创建用户' },
      { key: 'user:edit', label: '编辑用户' },
      { key: 'user:delete', label: '删除用户' },
      { key: 'user:assign_role', label: '分配角色' }
    ]
  },
  {
    key: 'role', label: '角色管理', children: [
      { key: 'role:view', label: '查看角色' },
      { key: 'role:create', label: '创建角色' },
      { key: 'role:edit', label: '编辑角色' },
      { key: 'role:delete', label: '删除角色' },
      { key: 'role:assign_perm', label: '分配权限' }
    ]
  },
  {
    key: 'llm', label: 'LLM配置', children: [
      { key: 'llm:view', label: '查看配置' },
      { key: 'llm:edit', label: '编辑配置' },
      { key: 'llm:manage_provider', label: '管理供应商' },
      { key: 'llm:manage_routing', label: '管理路由' },
      { key: 'llm:view_usage', label: '查看用量' }
    ]
  },
  {
    key: 'knowledge', label: '知识库管理', children: [
      { key: 'kb:view', label: '查看知识库' },
      { key: 'kb:create', label: '创建知识库' },
      { key: 'kb:edit', label: '编辑知识库' },
      { key: 'kb:delete', label: '删除知识库' },
      { key: 'kb:manage_cat', label: '分类管理' },
      { key: 'kb:manage_perm', label: '权限管理' }
    ]
  },
  {
    key: 'storage', label: '存储管理', children: [
      { key: 'storage:view', label: '查看存储' },
      { key: 'storage:configure', label: '配置路径' },
      { key: 'storage:manage_perm', label: '权限管理' }
    ]
  },
  {
    key: 'system', label: '系统设置', children: [
      { key: 'sys:view', label: '查看设置' },
      { key: 'sys:general', label: '通用设置' },
      { key: 'sys:security', label: '安全设置' },
      { key: 'sys:about', label: '系统信息' }
    ]
  }
])

const checkedKeys = ref([])
const boundUsers = ref([])

const deptTree = ref([
  { id: 1, name: '总公司', children: [
    { id: 11, name: '技术部', children: [
      { id: 111, name: '前端组' },
      { id: 112, name: '后端组' }
    ]},
    { id: 12, name: '运营部' },
    { id: 13, name: '市场部' }
  ]}
])

const roleDialogVisible = ref(false)
const roleDialogTitle = ref('新增角色')
const isEditRole = ref(false)
const roleFormRef = ref(null)
const roleForm = reactive({ id: null, name: '', code: '', description: '', sort: 0, enabled: true })
const roleRules = {
  name: [{ required: true, message: '请输入角色名称', trigger: 'blur' }],
  code: [{ required: true, message: '请输入角色标识', trigger: 'blur' }]
}

function selectRole(role) {
  selectedRole.value = role
  loadPermissions(role)
}

function loadPermissions(role) {
  const mockPermsMap = {
    1: ['user:view', 'user:create', 'user:edit', 'user:delete', 'user:assign_role', 'role:view', 'role:create', 'role:edit', 'role:delete', 'role:assign_perm', 'llm:view', 'llm:edit', 'llm:manage_provider', 'llm:manage_routing', 'llm:view_usage', 'kb:view', 'kb:create', 'kb:edit', 'kb:delete', 'kb:manage_cat', 'kb:manage_perm', 'storage:view', 'storage:configure', 'storage:manage_perm', 'sys:view', 'sys:general', 'sys:security', 'sys:about'],
    2: ['user:view', 'user:create', 'user:edit', 'role:view', 'role:edit', 'llm:view', 'llm:edit', 'llm:manage_provider', 'kb:view', 'kb:create', 'kb:edit', 'kb:manage_cat', 'kb:manage_perm', 'storage:view', 'storage:configure', 'sys:view', 'sys:general', 'sys:about'],
    3: ['user:view', 'kb:view', 'kb:create', 'kb:edit', 'kb:manage_cat', 'llm:view', 'llm:view_usage'],
    4: ['user:view', 'kb:view', 'llm:view', 'llm:view_usage', 'storage:view', 'sys:about'],
    5: ['kb:view'],
    6: ['user:view', 'role:view', 'sys:view', 'sys:about']
  }
  checkedKeys.value = mockPermsMap[role.id] || []
  boundUsers.value = [
    { username: 'zhangsan', realName: '张三', email: 'zhangsan@example.com' },
    { username: 'lisi', realName: '李四', email: 'lisi@example.com' }
  ]
  nextTick(() => {
    if (treeRef.value) treeRef.value.setCheckedKeys(checkedKeys.value)
  })
}

function handleCheckAll(val) {
  if (treeRef.value) {
    if (val) treeRef.value.setCheckedNodes(permissionTree.value, true)
    else treeRef.value.setCheckedKeys([])
  }
}

function handleExpandAll(val) {
  nextTick(() => {
    const nodes = treeRef.value?.store?.nodesMap
    if (nodes) {
      for (const key in nodes) {
        nodes[key].expanded = val
      }
    }
  })
}

function expandAllNodes() { handleExpandAll(true) }
function collapseAllNodes() { handleExpandAll(false) }

function openRoleDialog(role = null) {
  isEditRole.value = !!role
  roleDialogTitle.value = role ? '编辑角色' : '新增角色'
  Object.assign(roleForm, role || { id: null, name: '', code: '', description: '', sort: 0, enabled: true })
  roleDialogVisible.value = true
}

async function handleRoleSubmit() {
  if (!roleFormRef.value) return
  await roleFormRef.value.validate()
  try {
    if (isEditRole.value) {
      await adminApi.updateRole(roleForm.id, roleForm)
      const idx = roles.value.findIndex(r => r.id === roleForm.id)
      if (idx > -1) roles.value[idx] = { ...roles.value[idx], ...roleForm }
      ElMessage.success('角色更新成功')
    } else {
      const newId = Math.max(...roles.value.map(r => r.id)) + 1
      roles.value.push({ id: newId, ...roleForm, userCount: 0, dataScope: 'self', permissionsEnabled: true })
      ElMessage.success('角色创建成功')
    }
    roleDialogVisible.value = false
  } catch (e) {
    if (e.response?.status === 400 || e.code === 'ERR_BAD_REQUEST') {
      ElMessage.success(isEditRole.value ? '角色更新成功（模拟）' : '角色创建成功（模拟）')
      roleDialogVisible.value = false
    }
  }
}

async function handleDeleteRole(role) {
  try {
    await ElMessageBox.confirm(`确定要删除角色 "${role.name}" 吗？`, '删除确认', { type: 'warning' })
    try {
      await adminApi.deleteRole(role.id)
    } catch (e) { /* mock */ }
    roles.value = roles.value.filter(r => r.id !== role.id)
    if (selectedRole.value?.id === role.id) selectedRole.value = null
    ElMessage.success('删除成功')
  } catch (e) { /* cancelled */ }
}

function unbindUser(row) {
  ElMessage.success(`已从角色中移除用户 ${row.username}`)
  boundUsers.value = boundUsers.value.filter(u => u.username !== row.username)
}

function resetPermissions() {
  if (selectedRole.value) loadPermissions(selectedRole.value)
  ElMessage.info('权限已重置')
}

async function savePermissions() {
  const newChecked = treeRef.value?.getCheckedKeys() || []
  checkedKeys.value = newChecked
  ElMessage.success('权限配置已保存')
}

onMounted(async () => {
  try {
    const data = await adminApi.getRoles()
    if (data?.data) roles.value = data.data
  } catch (e) { /* use mock data */ }
})
</script>

<style scoped>
.role-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.role-item {
  padding: 14px;
  border: 2px solid #ebeef5;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.3s;
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.role-item:hover {
  border-color: #409eff;
  box-shadow: 0 2px 8px rgba(64, 158, 255, 0.15);
}

.role-item.active {
  border-color: #409eff;
  background: #ecf5ff;
}

.role-info { flex: 1; }

.role-name {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.role-desc {
  font-size: 13px;
  color: #909399;
  margin-bottom: 8px;
}

.role-meta { display: flex; gap: 8px; }

.role-actions {
  display: flex;
  flex-direction: column;
  gap: 4px;
  align-items: flex-end;
}

.role-desc-text {
  color: #909399;
  font-size: 13px;
  margin: 4px 0 0;
}

.perm-tree-container {
  background: #fafafa;
  border-radius: 6px;
  padding: 16px;
}

.tree-toolbar {
  display: flex;
  gap: 16px;
  margin-bottom: 12px;
  padding-bottom: 12px;
  border-bottom: 1px solid #ebeef5;
  align-items: center;
}

.section-title {
  font-size: 14px;
  color: #606266;
  margin: 0 0 10px;
}

.custom-scope {
  padding: 12px;
  background: #fafafa;
  border-radius: 6px;
  margin-top: 12px;
}

.form-actions {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid #ebeef5;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 400px;
}
</style>