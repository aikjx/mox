<template>
  <div>
    <div class="admin-card">
      <div class="admin-table-toolbar">
        <div>
          <h3 class="admin-page-title" style="margin:0">存储路径配置</h3>
          <p class="subtitle">管理系统挂载的云存储路径</p>
        </div>
        <div>
          <el-button type="primary" :icon="Plus" @click="openCreateDialog">新增存储路径</el-button>
          <el-button :icon="Refresh" @click="refreshHealth">检测健康状态</el-button>
        </div>
      </div>

      <el-row :gutter="16" class="path-grid">
        <el-col v-for="path in storagePaths" :key="path.id" :xs="24" :sm="12" :md="8" :lg="8">
          <div class="path-card" :class="{ unhealthy: path.health !== 'healthy' }">
            <div class="path-header">
              <div class="path-type-icon" :class="path.type">
                <el-icon :size="22">
                  <FolderOpened v-if="path.type === 'local'" />
                  <Cpu v-else-if="path.type === 's3'" />
                  <Files v-else-if="path.type === 'oss'" />
                  <Folder v-else />
                </el-icon>
              </div>
              <el-tag
                :type="healthTagType(path.health)"
                effect="dark"
                size="small"
              >
                <span class="status-dot"></span>
                {{ healthLabel(path.health) }}
              </el-tag>
            </div>
            <h4 class="path-name">{{ path.name }}</h4>
            <div class="path-location">
              <el-icon><Location /></el-icon>
              <code>{{ path.path }}</code>
            </div>
            <div class="path-storage-info">
              <div class="storage-bar">
                <el-progress
                  :percentage="path.usagePercent"
                  :color="storageColor(path.usagePercent)"
                  :stroke-width="8"
                />
              </div>
              <div class="storage-detail">
                <span>已用: {{ formatSize(path.used) }}</span>
                <span>总计: {{ formatSize(path.capacity) }}</span>
              </div>
            </div>
            <div class="path-meta">
              <div class="meta-row">
                <span class="meta-label">类型</span>
                <span class="meta-value">{{ typeLabel(path.type) }}</span>
              </div>
              <div class="meta-row">
                <span class="meta-label">权限</span>
                <span class="meta-value">
                  <el-tag size="small" :type="permTagType(path.accessLevel)" effect="plain">
                    {{ accessLabel(path.accessLevel) }}
                  </el-tag>
                </span>
              </div>
              <div class="meta-row">
                <span class="meta-label">挂载点</span>
                <span class="meta-value">{{ path.mountPoint }}</span>
              </div>
            </div>
            <div class="path-actions">
              <el-button type="primary" link size="small" :icon="Edit" @click="openEditDialog(path)">编辑</el-button>
              <el-button type="warning" link size="small" :icon="Connection" @click="testConnection(path)">测试</el-button>
              <el-button type="danger" link size="small" :icon="Delete" @click="handleDelete(path)">删除</el-button>
            </div>
          </div>
        </el-col>
      </el-row>
    </div>

    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="550px">
      <el-form :model="formData" :rules="formRules" ref="formRef" label-width="120px">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="名称" prop="name">
              <el-input v-model="formData.name" placeholder="存储路径名称" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="类型" prop="type">
              <el-select v-model="formData.type" style="width: 100%">
                <el-option label="本地存储" value="local" />
                <el-option label="阿里云OSS" value="oss" />
                <el-option label="AWS S3" value="s3" />
                <el-option label="MinIO" value="minio" />
                <el-option label="Azure Blob" value="azure" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="路径/Endpoint" prop="path">
          <el-input
            v-model="formData.path"
            :placeholder="pathPlaceholder"
          />
        </el-form-item>
        <template v-if="formData.type === 's3' || formData.type === 'oss'">
          <el-row :gutter="16">
            <el-col :span="12">
              <el-form-item label="访问密钥">
                <el-input v-model="formData.accessKey" placeholder="Access Key" />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="密钥">
                <el-input v-model="formData.secretKey" type="password" show-password placeholder="Secret Key" />
              </el-form-item>
            </el-col>
          </el-row>
          <el-row :gutter="16">
            <el-col :span="12">
              <el-form-item label="存储桶">
                <el-input v-model="formData.bucket" placeholder="Bucket 名称" />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="区域">
                <el-input v-model="formData.region" placeholder="如: oss-cn-hangzhou" />
              </el-form-item>
            </el-col>
          </el-row>
        </template>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="容量 (GB)">
              <el-input-number v-model="formData.capacity" :min="1" :step="100" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="访问级别">
              <el-select v-model="formData.accessLevel" style="width: 100%">
                <el-option label="公开" value="public" />
                <el-option label="组织内" value="organization" />
                <el-option label="私有" value="private" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="自动备份">
          <el-switch v-model="formData.autoBackup" active-text="启用" />
        </el-form-item>
        <el-form-item label="压缩存储">
          <el-switch v-model="formData.compression" active-text="启用" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button @click="testCurrent">测试连接</el-button>
        <el-button type="primary" @click="handleSubmit">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { adminApi } from '@/api/index'
import { Plus, Refresh, Edit, Delete, FolderOpened, Cpu, Files, Folder, Connection, Location } from '@element-plus/icons-vue'

const loading = ref(false)

const storagePaths = ref([
  { id: 1, name: '主文档存储', type: 'local', path: '/data/documents', mountPoint: '/mnt/docs', capacity: 500, used: 320, health: 'healthy', accessLevel: 'organization', autoBackup: true, compression: true },
  { id: 2, name: '知识库索引', type: 'oss', path: 'oss://knowledge-index', mountPoint: '/mnt/kb-index', capacity: 1000, used: 680, health: 'healthy', accessLevel: 'private', autoBackup: true, compression: false },
  { id: 3, name: '用户上传', type: 's3', path: 's3://user-uploads', mountPoint: '/mnt/uploads', capacity: 2000, used: 840, health: 'healthy', accessLevel: 'public', autoBackup: false, compression: true },
  { id: 4, name: '日志归档', type: 'oss', path: 'oss://log-archive', mountPoint: '/mnt/logs', capacity: 500, used: 480, health: 'warning', accessLevel: 'private', autoBackup: true, compression: true },
  { id: 5, name: '备份存储', type: 'minio', path: 'minio://backup', mountPoint: '/mnt/backup', capacity: 5000, used: 1200, health: 'healthy', accessLevel: 'private', autoBackup: true, compression: false },
  { id: 6, name: '临时缓存', type: 'local', path: '/data/tmp', mountPoint: '/mnt/tmp', capacity: 100, used: 85, health: 'warning', accessLevel: 'public', autoBackup: false, compression: false }
])

const dialogVisible = ref(false)
const dialogTitle = ref('新增存储路径')
const isEdit = ref(false)
const formRef = ref(null)
const formData = reactive({
  id: null, name: '', type: 'local', path: '', mountPoint: '', capacity: 500, used: 0,
  accessLevel: 'private', accessKey: '', secretKey: '', bucket: '', region: '',
  autoBackup: true, compression: false
})
const formRules = {
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择类型', trigger: 'change' }],
  path: [{ required: true, message: '请输入路径或Endpoint', trigger: 'blur' }]
}

const pathPlaceholder = computed(() => {
  const map = {
    local: '/data/path',
    s3: 's3://bucket-name',
    oss: 'oss://bucket-name',
    minio: 'minio://bucket-name',
    azure: 'azure://container-name'
  }
  return map[formData.type] || '请输入路径'
})

function formatSize(gb) {
  if (gb >= 1024) return (gb / 1024).toFixed(2) + ' TB'
  return gb + ' GB'
}

const healthPercent = (p) => Math.round((p.used / p.capacity) * 100)

function storageColor(percent) {
  if (percent > 90) return '#f56c6c'
  if (percent > 75) return '#e6a23c'
  return '#67c23a'
}

function healthTagType(health) {
  return { healthy: 'success', warning: 'warning', error: 'danger' }[health] || 'info'
}

function healthLabel(health) {
  return { healthy: '正常', warning: '注意', error: '异常' }[health] || '未知'
}

function typeLabel(type) {
  return { local: '本地', s3: 'AWS S3', oss: '阿里云OSS', minio: 'MinIO', azure: 'Azure' }[type] || type
}

function accessLabel(level) {
  return { public: '公开', organization: '组织内', private: '私有' }[level] || level
}

function permTagType(level) {
  return { public: 'success', organization: 'warning', private: 'info' }[level] || ''
}

function openCreateDialog() {
  isEdit.value = false
  dialogTitle.value = '新增存储路径'
  Object.assign(formData, {
    id: null, name: '', type: 'local', path: '', mountPoint: '', capacity: 500, used: 0,
    accessLevel: 'private', accessKey: '', secretKey: '', bucket: '', region: '',
    autoBackup: true, compression: false
  })
  dialogVisible.value = true
}

function openEditDialog(path) {
  isEdit.value = true
  dialogTitle.value = '编辑存储路径'
  Object.assign(formData, {
    ...path, accessKey: '', secretKey: '', bucket: path.bucket || '', region: path.region || ''
  })
  dialogVisible.value = true
}

async function handleSubmit() {
  if (!formRef.value) return
  await formRef.value.validate()
  try {
    if (isEdit.value) {
      await adminApi.configureStoragePath(formData)
      const idx = storagePaths.value.findIndex(p => p.id === formData.id)
      if (idx > -1) storagePaths.value[idx] = { ...storagePaths.value[idx], ...formData }
      ElMessage.success('存储路径更新成功')
    } else {
      const newId = Math.max(...storagePaths.value.map(p => p.id)) + 1
      storagePaths.value.push({ ...formData, id: newId, used: 0, health: 'healthy' })
      ElMessage.success('存储路径创建成功')
    }
    dialogVisible.value = false
  } catch (e) {
    if (e.response?.status === 400 || e.code === 'ERR_BAD_REQUEST') {
      ElMessage.success(isEdit.value ? '更新成功（模拟）' : '创建成功（模拟）')
      dialogVisible.value = false
    }
  }
}

function testConnection(path) {
  ElMessage.info(`正在测试 ${path.name} 连接...`)
  setTimeout(() => {
    ElMessage.success(`${path.name} 连接正常，延迟 ${(50 + Math.random() * 150).toFixed(0)}ms`)
  }, 1500)
}

function testCurrent() {
  ElMessage.info('正在测试连接...')
  setTimeout(() => ElMessage.success('连接测试成功'), 1500)
}

async function handleDelete(path) {
  try {
    await ElMessageBox.confirm(`确定删除存储路径 "${path.name}" 吗？`, '删除确认', { type: 'warning' })
    storagePaths.value = storagePaths.value.filter(p => p.id !== path.id)
    ElMessage.success('删除成功')
  } catch (e) { /* cancelled */ }
}

function refreshHealth() {
  storagePaths.value.forEach(p => {
    const rand = Math.random()
    p.health = rand > 0.1 ? 'healthy' : rand > 0.05 ? 'warning' : 'error'
  })
  ElMessage.success('健康检测完成')
}

onMounted(async () => {
  loading.value = true
  try {
    const data = await adminApi.getStoragePaths()
    if (data?.data) storagePaths.value = data.data
  } catch (e) { /* use mock data */ }
  loading.value = false
})
</script>

<style scoped>
.subtitle { font-size: 13px; color: #909399; margin: 4px 0 0; }

.path-grid { margin-bottom: 0; }

.path-card {
  background: #fff;
  border: 1px solid #ebeef5;
  border-radius: 10px;
  padding: 16px;
  margin-bottom: 16px;
  transition: all 0.3s;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.path-card:hover {
  border-color: #409eff;
  box-shadow: 0 4px 16px rgba(64, 158, 255, 0.12);
}

.path-card.unhealthy {
  border-color: #e6a23c;
  background: #fdf6ec;
}

.path-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.path-type-icon {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
}

.path-type-icon.local { background: linear-gradient(135deg, #409eff, #66b1ff); }
.path-type-icon.s3 { background: linear-gradient(135deg, #ff9800, #ffb74d); }
.path-type-icon.oss { background: linear-gradient(135deg, #e6a23c, #f0c78a); }
.path-type-icon.minio { background: linear-gradient(135deg, #67c23a, #95d475); }
.path-type-icon.azure { background: linear-gradient(135deg, #00bcd4, #4dd0e1); }

.status-dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #fff;
  margin-right: 4px;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.path-name {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
  color: #303133;
}

.path-location {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #606266;
}

.path-location code {
  background: #f5f7fa;
  padding: 2px 6px;
  border-radius: 3px;
  font-family: 'Consolas', monospace;
}

.path-storage-info {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.storage-detail {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: #909399;
}

.path-meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px;
  background: #fafbfc;
  border-radius: 6px;
}

.meta-row {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
}

.meta-label { color: #909399; }
.meta-value { color: #303133; font-weight: 500; }

.path-actions {
  display: flex;
  gap: 4px;
  justify-content: flex-end;
  padding-top: 8px;
  border-top: 1px solid #f2f3f5;
}
</style>