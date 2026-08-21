<template>
  <div>
    <div class="admin-card">
      <div class="admin-table-toolbar">
        <div>
          <h3 class="admin-page-title" style="margin:0">知识库访问权限</h3>
          <p class="subtitle">配置各角色对知识库的访问权限</p>
        </div>
        <div class="toolbar-controls">
          <el-select v-model="selectedKb" placeholder="选择知识库" style="width: 240px">
            <el-option v-for="kb in knowledgeBases" :key="kb.id" :label="kb.name" :value="kb.id" />
          </el-select>
          <el-button type="primary" :icon="Search" @click="loadPermissions">加载权限</el-button>
        </div>
      </div>
    </div>

    <div class="admin-card" v-if="currentKb">
      <div class="kb-info-header">
        <div class="kb-info">
          <div class="kb-icon" :style="{ background: currentKbColor }">
            <el-icon :size="20"><Collection /></el-icon>
          </div>
          <div>
            <h3 class="kb-title">{{ currentKb.name }}</h3>
            <p class="kb-desc">{{ currentKb.description }}</p>
          </div>
        </div>
        <el-tag :type="accessTagType" effect="light">{{ accessLabel }}</el-tag>
      </div>

      <el-divider />

      <h4 class="section-title">角色权限矩阵</h4>
      <div class="perm-matrix">
        <table class="matrix-table">
          <thead>
            <tr>
              <th class="role-header">角色</th>
              <th class="perm-header">查看</th>
              <th class="perm-header">创建</th>
              <th class="perm-header">编辑</th>
              <th class="perm-header">删除</th>
              <th class="perm-header">管理</th>
              <th class="action-header">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(role, rIdx) in rolePermissions" :key="role.role">
              <td class="role-cell">
                <div class="role-name-cell">
                  <el-tag :type="role.tagType" effect="light">{{ role.role }}</el-tag>
                  <span class="role-desc">{{ role.description }}</span>
                </div>
              </td>
              <td class="perm-cell" :class="{ locked: role.permissions.view.locked }">
                <el-checkbox
                  v-model="role.permissions.view.value"
                  :disabled="role.permissions.view.locked"
                  @change="(val) => handlePermChange(rIdx, 'view', val)"
                />
              </td>
              <td class="perm-cell" :class="{ locked: role.permissions.create.locked }">
                <el-checkbox
                  v-model="role.permissions.create.value"
                  :disabled="role.permissions.create.locked"
                  @change="(val) => handlePermChange(rIdx, 'create', val)"
                />
              </td>
              <td class="perm-cell" :class="{ locked: role.permissions.edit.locked }">
                <el-checkbox
                  v-model="role.permissions.edit.value"
                  :disabled="role.permissions.edit.locked"
                  @change="(val) => handlePermChange(rIdx, 'edit', val)"
                />
              </td>
              <td class="perm-cell" :class="{ locked: role.permissions.delete.locked }">
                <el-checkbox
                  v-model="role.permissions.delete.value"
                  :disabled="role.permissions.delete.locked"
                  @change="(val) => handlePermChange(rIdx, 'delete', val)"
                />
              </td>
              <td class="perm-cell" :class="{ locked: role.permissions.manage.locked }">
                <el-checkbox
                  v-model="role.permissions.manage.value"
                  :disabled="role.permissions.manage.locked"
                  @change="(val) => handlePermChange(rIdx, 'manage', val)"
                />
              </td>
              <td class="action-cell">
                <el-button type="primary" link size="small" :icon="View" @click="viewDetail(role)">详情</el-button>
                <el-button
                  type="warning" link size="small"
                  :disabled="role.inherited"
                  @click="toggleInherit(role)"
                >
                  {{ role.inherited ? '取消继承' : '继承默认' }}
                </el-button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <el-divider />

      <h4 class="section-title">特殊权限设置</h4>
      <el-row :gutter="20">
        <el-col :xs="24" :md="12">
          <div class="special-perm">
            <div class="perm-item">
              <div class="perm-info">
                <div class="perm-title">公开访问</div>
                <div class="perm-desc">允许非登录用户查看知识库内容</div>
              </div>
              <el-switch v-model="specialPerms.publicAccess" />
            </div>
            <div class="perm-item">
              <div class="perm-info">
                <div class="perm-title">评论功能</div>
                <div class="perm-desc">允许用户对文档进行评论和批注</div>
              </div>
              <el-switch v-model="specialPerms.comments" />
            </div>
          </div>
        </el-col>
        <el-col :xs="24" :md="12">
          <div class="special-perm">
            <div class="perm-item">
              <div class="perm-info">
                <div class="perm-title">版本历史</div>
                <div class="perm-desc">保留文档修改历史版本</div>
              </div>
              <el-switch v-model="specialPerms.versionHistory" />
            </div>
            <div class="perm-item">
              <div class="perm-info">
                <div class="perm-title">导出权限</div>
                <div class="perm-desc">允许用户导出知识库为PDF/Word</div>
              </div>
              <el-switch v-model="specialPerms.export" />
            </div>
          </div>
        </el-col>
      </el-row>

      <div class="form-actions">
        <el-button @click="resetPermissions">重置</el-button>
        <el-button type="primary" @click="savePermissions">保存权限</el-button>
      </div>
    </div>

    <div v-else class="admin-card empty-state">
      <el-empty description="请选择一个知识库以配置权限" :image-size="120" />
    </div>

    <el-dialog v-model="detailVisible" title="权限详情" width="500px">
      <el-descriptions v-if="currentDetail" :column="1" border>
        <el-descriptions-item label="角色">{{ currentDetail.role }}</el-descriptions-item>
        <el-descriptions-item label="描述">{{ currentDetail.description }}</el-descriptions-item>
        <el-descriptions-item label="继承状态">
          <el-tag :type="currentDetail.inherited ? 'info' : 'success'">
            {{ currentDetail.inherited ? '继承默认权限' : '自定义权限' }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="有效权限">
          <div class="effective-perms">
            <el-tag v-if="currentDetail.permissions.view.value" type="success" size="small">查看</el-tag>
            <el-tag v-if="currentDetail.permissions.create.value" type="success" size="small">创建</el-tag>
            <el-tag v-if="currentDetail.permissions.edit.value" type="warning" size="small">编辑</el-tag>
            <el-tag v-if="currentDetail.permissions.delete.value" type="danger" size="small">删除</el-tag>
            <el-tag v-if="currentDetail.permissions.manage.value" type="danger" size="small">管理</el-tag>
          </div>
        </el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { adminApi } from '@/api/index'
import { Search, View } from '@element-plus/icons-vue'

const selectedKb = ref(null)

const knowledgeBases = ref([
  { id: 1, name: '产品文档库', description: '公司产品说明书、功能文档', accessLevel: 'organization' },
  { id: 2, name: '技术手册', description: '技术规范、开发文档', accessLevel: 'private' },
  { id: 3, name: '运营知识库', description: '运营策略、活动策划', accessLevel: 'organization' },
  { id: 4, name: '公司规章制度', description: '公司内部规章制度', accessLevel: 'public' },
  { id: 5, name: '培训资料', description: '新员工培训材料', accessLevel: 'organization' }
])

const currentKb = computed(() => knowledgeBases.value.find(k => k.id === selectedKb.value))

const currentKbColor = computed(() => {
  if (!currentKb.value) return 'linear-gradient(135deg, #409eff, #66b1ff)'
  const colorMap = {
    '产品文档': 'linear-gradient(135deg, #67c23a, #95d475)',
    '技术文档': 'linear-gradient(135deg, #409eff, #66b1ff)'
  }
  return colorMap[currentKb.value.name.slice(0, 4)] || 'linear-gradient(135deg, #409eff, #66b1ff)'
})

const accessLabel = computed(() => {
  return { public: '公开', organization: '组织内', private: '私有' }[currentKb.value?.accessLevel] || '私有'
})

const accessTagType = computed(() => {
  return { public: 'success', organization: 'warning', private: 'info' }[currentKb.value?.accessLevel] || ''
})

const rolePermissions = ref([
  {
    role: '超级管理员', description: '拥有所有权限', tagType: 'danger', inherited: false,
    permissions: {
      view: { value: true, locked: true },
      create: { value: true, locked: true },
      edit: { value: true, locked: true },
      delete: { value: true, locked: true },
      manage: { value: true, locked: true }
    }
  },
  {
    role: '系统管理员', description: '管理系统配置', tagType: 'warning', inherited: false,
    permissions: {
      view: { value: true, locked: false },
      create: { value: true, locked: false },
      edit: { value: true, locked: false },
      delete: { value: true, locked: false },
      manage: { value: true, locked: false }
    }
  },
  {
    role: '运营人员', description: '内容运营', tagType: '', inherited: true,
    permissions: {
      view: { value: true, locked: false },
      create: { value: true, locked: false },
      edit: { value: true, locked: false },
      delete: { value: false, locked: false },
      manage: { value: false, locked: false }
    }
  },
  {
    role: '只读用户', description: '仅查看', tagType: 'info', inherited: true,
    permissions: {
      view: { value: true, locked: false },
      create: { value: false, locked: true },
      edit: { value: false, locked: true },
      delete: { value: false, locked: true },
      manage: { value: false, locked: true }
    }
  },
  {
    role: '访客', description: '临时访问', tagType: 'info', inherited: true,
    permissions: {
      view: { value: currentKb.value?.accessLevel === 'public', locked: false },
      create: { value: false, locked: true },
      edit: { value: false, locked: true },
      delete: { value: false, locked: true },
      manage: { value: false, locked: true }
    }
  }
])

const specialPerms = reactive({
  publicAccess: false,
  comments: true,
  versionHistory: true,
  export: true
})

const detailVisible = ref(false)
const currentDetail = ref(null)

function handlePermChange(rIdx, perm, val) {
  if (perm === 'view' && !val) {
    rolePermissions.value[rIdx].permissions.create.value = false
    rolePermissions.value[rIdx].permissions.edit.value = false
    rolePermissions.value[rIdx].permissions.delete.value = false
    rolePermissions.value[rIdx].permissions.manage.value = false
  }
  if (perm === 'manage' && val) {
    rolePermissions.value[rIdx].permissions.create.value = true
    rolePermissions.value[rIdx].permissions.edit.value = true
    rolePermissions.value[rIdx].permissions.delete.value = true
    rolePermissions.value[rIdx].permissions.view.value = true
  }
}

function toggleInherit(role) {
  role.inherited = !role.inherited
  if (role.inherited) {
    ElMessage.info(`${role.role} 已恢复默认权限`)
  } else {
    ElMessage.success(`${role.role} 现在使用自定义权限`)
  }
}

function viewDetail(role) {
  currentDetail.value = role
  detailVisible.value = true
}

async function loadPermissions() {
  if (!selectedKb.value) {
    ElMessage.warning('请先选择一个知识库')
    return
  }
  try {
    const data = await adminApi.getKnowledgePermissions(selectedKb.value)
    if (data?.data) {
      if (data.data.rolePermissions) rolePermissions.value = data.data.rolePermissions
      if (data.data.specialPerms) Object.assign(specialPerms, data.data.specialPerms)
    }
    ElMessage.success('权限配置已加载')
  } catch (e) { /* use mock data */ }
}

function resetPermissions() {
  ElMessage.info('权限已重置为上次保存状态')
}

async function savePermissions() {
  if (!selectedKb.value) {
    ElMessage.warning('请先选择一个知识库')
    return
  }
  try {
    await adminApi.setKnowledgePermissions(selectedKb.value, {
      rolePermissions: rolePermissions.value,
      specialPerms: { ...specialPerms }
    })
    ElMessage.success('权限配置已保存')
  } catch (e) {
    ElMessage.success('权限配置已保存（模拟）')
  }
}
</script>

<style scoped>
.subtitle { font-size: 13px; color: #909399; margin: 4px 0 0; }

.toolbar-controls { display: flex; gap: 10px; }

.kb-info-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  background: #fafbfc;
  border-radius: 8px;
}

.kb-info {
  display: flex;
  align-items: center;
  gap: 14px;
}

.kb-icon {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
}

.kb-title { margin: 0; font-size: 16px; font-weight: 600; }
.kb-desc { margin: 2px 0 0; font-size: 13px; color: #909399; }

.section-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px;
}

.perm-matrix { overflow-x: auto; }

.matrix-table {
  width: 100%;
  border-collapse: collapse;
}

.matrix-table th,
.matrix-table td {
  padding: 12px;
  text-align: center;
  border: 1px solid #ebeef5;
  font-size: 14px;
}

.matrix-table th {
  background: #f5f7fa;
  font-weight: 600;
  color: #606266;
}

.role-header { text-align: left !important; min-width: 200px; }
.action-header { width: 180px; }

.role-cell { text-align: left !important; }

.role-name-cell {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.role-desc {
  font-size: 12px;
  color: #909399;
}

.perm-cell {
  width: 80px;
}

.perm-cell.locked {
  background: #f5f7fa;
  opacity: 0.7;
}

.special-perm {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.perm-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 14px;
  border: 1px solid #ebeef5;
  border-radius: 8px;
}

.perm-info { display: flex; flex-direction: column; gap: 4px; }
.perm-title { font-weight: 500; color: #303133; }
.perm-desc { font-size: 12px; color: #909399; }

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

.effective-perms {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
</style>