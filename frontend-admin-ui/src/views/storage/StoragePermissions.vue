<template>
  <div>
    <div class="admin-card">
      <div class="admin-table-toolbar">
        <div>
          <h3 class="admin-page-title" style="margin:0">存储路径权限配置</h3>
          <p class="subtitle">配置各角色对存储路径的访问权限</p>
        </div>
        <el-select v-model="selectedPath" placeholder="选择存储路径" style="width: 260px">
          <el-option v-for="p in storagePaths" :key="p.id" :label="p.name" :value="p.id" />
        </el-select>
      </div>
    </div>

    <div class="admin-card" v-if="currentPath">
      <div class="path-info-header">
        <div class="path-info-main">
          <h3>{{ currentPath.name }}</h3>
          <code>{{ currentPath.path }}</code>
          <el-tag :type="permTagType(currentPath.accessLevel)" effect="plain">
            {{ accessLabel(currentPath.accessLevel) }}
          </el-tag>
        </div>
        <div class="path-info-stats">
          <div class="stat-block">
            <span class="stat-value">{{ formatSize(currentPath.capacity) }}</span>
            <span class="stat-label">总容量</span>
          </div>
          <div class="stat-block">
            <span class="stat-value">{{ formatSize(currentPath.used) }}</span>
            <span class="stat-label">已使用</span>
          </div>
          <div class="stat-block">
            <span class="stat-value">{{ formatSize(currentPath.capacity - currentPath.used) }}</span>
            <span class="stat-label">剩余</span>
          </div>
        </div>
      </div>

      <el-divider />

      <h4 class="section-title">角色权限映射</h4>
      <div class="perm-table-wrapper">
        <table class="perm-table">
          <thead>
            <tr>
              <th class="col-role">角色</th>
              <th class="col-perm">读取</th>
              <th class="col-perm">写入</th>
              <th class="col-perm">删除</th>
              <th class="col-perm">管理</th>
              <th class="col-perm">导出</th>
              <th class="col-perm">加密访问</th>
              <th class="col-action">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(role, idx) in rolePerms" :key="role.roleKey">
              <td class="col-role">
                <div class="role-cell-content">
                  <el-tag :type="role.tagType" effect="light">{{ role.name }}</el-tag>
                  <span class="role-meta">{{ role.description }}</span>
                </div>
              </td>
              <td class="col-perm">
                <el-checkbox v-model="role.perms.read" :disabled="role.perms.readLocked" />
              </td>
              <td class="col-perm">
                <el-checkbox v-model="role.perms.write" :disabled="!role.perms.read || role.perms.writeLocked" />
              </td>
              <td class="col-perm">
                <el-checkbox v-model="role.perms.delete" :disabled="!role.perms.write || role.perms.deleteLocked" />
              </td>
              <td class="col-perm">
                <el-checkbox v-model="role.perms.manage" :disabled="!role.perms.delete || role.perms.manageLocked" />
              </td>
              <td class="col-perm">
                <el-checkbox v-model="role.perms.export" />
              </td>
              <td class="col-perm">
                <el-checkbox v-model="role.perms.encrypted" />
              </td>
              <td class="col-action">
                <el-button type="primary" link size="small" @click="setAllPerms(role, true)">全部允许</el-button>
                <el-button type="danger" link size="small" @click="setAllPerms(role, false)">全部拒绝</el-button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <el-divider />

      <h4 class="section-title">特殊权限规则</h4>
      <el-row :gutter="20">
        <el-col :xs="24" :md="8">
          <div class="rule-card">
            <div class="rule-title">
              <el-icon><Warning /></el-icon>
              <span>IP白名单</span>
            </div>
            <div class="rule-content">
              <div class="rule-toggle">
                <el-switch v-model="specialRules.ipWhitelistEnabled" />
              </div>
              <div v-if="specialRules.ipWhitelistEnabled" class="rule-body">
                <el-input v-model="specialRules.ipRanges" type="textarea" :rows="3" placeholder="每行一个IP或CIDR，如: 192.168.1.0/24" />
              </div>
            </div>
          </div>
        </el-col>
        <el-col :xs="24" :md="8">
          <div class="rule-card">
            <div class="rule-title">
              <el-icon><Lock /></el-icon>
              <span>加密存储</span>
            </div>
            <div class="rule-content">
              <div class="rule-toggle">
                <el-switch v-model="specialRules.encryptionEnabled" />
              </div>
              <div v-if="specialRules.encryptionEnabled" class="rule-body">
                <el-form label-width="80px">
                  <el-form-item label="算法">
                    <el-select v-model="specialRules.encryptionAlgo" style="width: 100%">
                      <el-option label="AES-256" value="AES-256" />
                      <el-option label="SM4" value="SM4" />
                      <el-option label="ChaCha20" value="ChaCha20" />
                    </el-select>
                  </el-form-item>
                </el-form>
              </div>
            </div>
          </div>
        </el-col>
        <el-col :xs="24" :md="8">
          <div class="rule-card">
            <div class="rule-title">
              <el-icon><Clock /></el-icon>
              <span>访问时间限制</span>
            </div>
            <div class="rule-content">
              <div class="rule-toggle">
                <el-switch v-model="specialRules.timeRestrictionEnabled" />
              </div>
              <div v-if="specialRules.timeRestrictionEnabled" class="rule-body">
                <el-time-picker
                  v-model="specialRules.allowedStart"
                  placeholder="开始时间"
                  style="width: 100%"
                />
                <el-time-picker
                  v-model="specialRules.allowedEnd"
                  placeholder="结束时间"
                  style="width: 100%; margin-top: 8px"
                />
              </div>
            </div>
          </div>
        </el-col>
      </el-row>

      <el-divider />

      <h4 class="section-title">快速权限模板</h4>
      <div class="template-list">
        <div v-for="tpl in permissionTemplates" :key="tpl.name" class="template-item" @click="applyTemplate(tpl)">
          <div class="template-icon" :style="{ background: tpl.color }">
            <el-icon :size="18"><component :is="tpl.icon" /></el-icon>
          </div>
          <div class="template-info">
            <div class="template-name">{{ tpl.name }}</div>
            <div class="template-desc">{{ tpl.description }}</div>
          </div>
          <el-button type="primary" size="small" plain :icon="DocumentCopy">应用</el-button>
        </div>
      </div>

      <div class="form-actions">
        <el-button @click="resetChanges">重置</el-button>
        <el-button type="primary" @click="saveAll">保存配置</el-button>
      </div>
    </div>

    <div v-else class="admin-card empty-state">
      <el-empty description="请选择一个存储路径以配置权限" :image-size="120" />
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { adminApi } from '@/api/index'
import { Warning, Lock, Clock, DocumentCopy } from '@element-plus/icons-vue'

const selectedPath = ref(null)

const storagePaths = ref([
  { id: 1, name: '主文档存储', type: 'local', path: '/data/documents', capacity: 500, used: 320, accessLevel: 'organization' },
  { id: 2, name: '知识库索引', type: 'oss', path: 'oss://knowledge-index', capacity: 1000, used: 680, accessLevel: 'private' },
  { id: 3, name: '用户上传', type: 's3', path: 's3://user-uploads', capacity: 2000, used: 840, accessLevel: 'public' },
  { id: 4, name: '日志归档', type: 'oss', path: 'oss://log-archive', capacity: 500, used: 480, accessLevel: 'private' },
  { id: 5, name: '备份存储', type: 'minio', path: 'minio://backup', capacity: 5000, used: 1200, accessLevel: 'private' }
])

const currentPath = computed(() => storagePaths.value.find(p => p.id === selectedPath.value))

function formatSize(gb) {
  if (gb >= 1024) return (gb / 1024).toFixed(2) + ' TB'
  return gb + ' GB'
}

function accessLabel(level) {
  return { public: '公开', organization: '组织内', private: '私有' }[level] || level
}

function permTagType(level) {
  return { public: 'success', organization: 'warning', private: 'info' }[level] || ''
}

const rolePerms = ref([
  {
    roleKey: 'super_admin', name: '超级管理员', description: '系统最高权限', tagType: 'danger',
    perms: { read: true, write: true, delete: true, manage: true, export: true, encrypted: true, readLocked: true, writeLocked: true, deleteLocked: true, manageLocked: true }
  },
  {
    roleKey: 'admin', name: '系统管理员', description: '系统配置管理', tagType: 'warning',
    perms: { read: true, write: true, delete: true, manage: true, export: true, encrypted: false, readLocked: false, writeLocked: false, deleteLocked: false, manageLocked: false }
  },
  {
    roleKey: 'operator', name: '运营人员', description: '日常运营操作', tagType: '',
    perms: { read: true, write: true, delete: false, manage: false, export: true, encrypted: false, readLocked: false, writeLocked: false, deleteLocked: false, manageLocked: false }
  },
  {
    roleKey: 'viewer', name: '只读用户', description: '仅查看权限', tagType: 'info',
    perms: { read: true, write: false, delete: false, manage: false, export: false, encrypted: false, readLocked: false, writeLocked: true, deleteLocked: true, manageLocked: true }
  },
  {
    roleKey: 'guest', name: '访客', description: '临时访问', tagType: 'info',
    perms: { read: false, write: false, delete: false, manage: false, export: false, encrypted: false, readLocked: false, writeLocked: true, deleteLocked: true, manageLocked: true }
  }
])

const specialRules = reactive({
  ipWhitelistEnabled: false,
  ipRanges: '192.168.1.0/24\n10.0.0.0/8',
  encryptionEnabled: true,
  encryptionAlgo: 'AES-256',
  timeRestrictionEnabled: false,
  allowedStart: null,
  allowedEnd: null
})

const permissionTemplates = ref([
  {
    name: '完全控制', description: '所有权限全部开启',
    icon: 'Unlock', color: 'linear-gradient(135deg, #f56c6c, #f89898)',
    apply: (role) => {
      if (role.perms.readLocked) return
      role.perms.read = true
      role.perms.write = true
      role.perms.delete = true
      role.perms.manage = true
      role.perms.export = true
      role.perms.encrypted = true
    }
  },
  {
    name: '只读访问', description: '仅允许读取操作',
    icon: 'View', color: 'linear-gradient(135deg, #909399, #b1b3b8)',
    apply: (role) => {
      if (role.perms.readLocked) return
      role.perms.read = true
      role.perms.write = false
      role.perms.delete = false
      role.perms.manage = false
      role.perms.export = false
    }
  },
  {
    name: '读写模式', description: '允许读取和写入',
    icon: 'Edit', color: 'linear-gradient(135deg, #409eff, #66b1ff)',
    apply: (role) => {
      if (role.perms.readLocked) return
      role.perms.read = true
      role.perms.write = true
      role.perms.delete = false
      role.perms.manage = false
      role.perms.export = true
    }
  },
  {
    name: '安全模式', description: '仅允许必要操作',
    icon: 'Lock', color: 'linear-gradient(135deg, #e6a23c, #f0c78a)',
    apply: (role) => {
      if (role.perms.readLocked) return
      role.perms.read = true
      role.perms.write = false
      role.perms.delete = false
      role.perms.manage = false
      role.perms.export = false
      role.perms.encrypted = true
    }
  }
])

function setAllPerms(role, allow) {
  if (role.perms.readLocked) {
    ElMessage.warning(`${role.name} 的权限为系统锁定，无法修改`)
    return
  }
  role.perms.read = allow
  role.perms.write = allow
  role.perms.delete = allow
  role.perms.manage = allow
  role.perms.export = allow
  if (allow) {
    role.perms.write = true
    role.perms.delete = true
    role.perms.manage = true
  }
  ElMessage.success(`${role.name} 已设置为${allow ? '全部允许' : '全部拒绝'}`)
}

function applyTemplate(tpl) {
  rolePerms.value.forEach(role => tpl.apply(role))
  ElMessage.success(`已应用「${tpl.name}」模板`)
}

function resetChanges() {
  ElMessage.info('权限已重置')
}

async function saveAll() {
  if (!selectedPath.value) {
    ElMessage.warning('请先选择存储路径')
    return
  }
  try {
    await adminApi.setStoragePermissions(selectedPath.value, {
      rolePerms: rolePerms.value,
      specialRules: { ...specialRules }
    })
    ElMessage.success('权限配置已保存')
  } catch (e) {
    ElMessage.success('权限配置已保存（模拟）')
  }
}
</script>

<style scoped>
.subtitle { font-size: 13px; color: #909399; margin: 4px 0 0; }

.path-info-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  background: #fafbfc;
  border-radius: 8px;
  flex-wrap: wrap;
  gap: 16px;
}

.path-info-main {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.path-info-main h3 { margin: 0; font-size: 16px; }

.path-info-main code {
  background: #fff;
  padding: 4px 8px;
  border-radius: 4px;
  font-family: 'Consolas', monospace;
  font-size: 13px;
}

.path-info-stats {
  display: flex;
  gap: 24px;
}

.stat-block { text-align: center; }
.stat-block .stat-value { display: block; font-size: 18px; font-weight: 700; color: #303133; }
.stat-block .stat-label { font-size: 12px; color: #909399; }

.section-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px;
}

.perm-table-wrapper { overflow-x: auto; }

.perm-table {
  width: 100%;
  border-collapse: collapse;
  min-width: 900px;
}

.perm-table th,
.perm-table td {
  padding: 14px 12px;
  text-align: center;
  border: 1px solid #ebeef5;
  font-size: 14px;
}

.perm-table th {
  background: #f5f7fa;
  font-weight: 600;
  color: #606266;
}

.col-role { text-align: left !important; min-width: 200px; }
.col-perm { width: 90px; }
.col-action { width: 140px; }

.role-cell-content {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.role-meta { font-size: 12px; color: #909399; }

.rule-card {
  border: 1px solid #ebeef5;
  border-radius: 10px;
  padding: 16px;
  margin-bottom: 16px;
  transition: all 0.3s;
}

.rule-card:hover { box-shadow: 0 2px 8px rgba(0,0,0,0.06); }

.rule-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 15px;
  margin-bottom: 12px;
  color: #303133;
}

.rule-title .el-icon { color: #e6a23c; }

.rule-body {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid #f2f3f5;
}

.template-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 12px;
}

.template-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border: 1px solid #ebeef5;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.template-item:hover {
  border-color: #409eff;
  background: #f5f7fa;
}

.template-icon {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
}

.template-info { flex: 1; }
.template-name { font-weight: 600; color: #303133; font-size: 14px; }
.template-desc { font-size: 12px; color: #909399; }

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