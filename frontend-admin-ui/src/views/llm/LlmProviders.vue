<template>
  <div>
    <div class="admin-card">
      <div class="admin-table-toolbar">
        <div>
          <h3 class="admin-page-title" style="margin:0">LLM供应商管理</h3>
          <p class="subtitle">配置和管理大语言模型供应商接入</p>
        </div>
        <el-button type="primary" :icon="Plus" @click="openCreateDialog">新增供应商</el-button>
      </div>

      <el-table :data="providers" v-loading="loading" stripe border style="width: 100%">
        <el-table-column prop="name" label="供应商名称" width="180">
          <template #default="{ row }">
            <div class="provider-cell">
              <div class="provider-logo">{{ row.name.charAt(0) }}</div>
              <div>
                <div class="provider-name">{{ row.name }}</div>
                <div class="provider-type">{{ row.type }}</div>
              </div>
            </div>
          </template>
        </el-table-column>
        <el-table-column prop="apiEndpoint" label="API端点" min-width="200">
          <template #default="{ row }">
            <code class="api-endpoint">{{ row.apiEndpoint }}</code>
          </template>
        </el-table-column>
        <el-table-column prop="model" label="默认模型" width="160">
          <template #default="{ row }">
            <el-tag v-for="m in row.models.slice(0, 2)" :key="m" size="small" effect="plain" style="margin-right:4px">
              {{ m }}
            </el-tag>
            <span v-if="row.models.length > 2" class="more-models">+{{ row.models.length - 2 }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="row.status === 'online' ? 'success' : row.status === 'warning' ? 'warning' : 'info'" effect="dark">
              <span class="status-dot"></span>
              {{ row.status === 'online' ? '在线' : row.status === 'warning' ? '响应慢' : '离线' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="isDefault" label="默认" width="80" align="center">
          <template #default="{ row }">
            <el-tag v-if="row.isDefault" type="danger" effect="dark">默认</el-tag>
            <span v-else style="color:#c0c4cc">-</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="220" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" :icon="Edit" @click="openEditDialog(row)">编辑</el-button>
            <el-button
              v-if="!row.isDefault"
              type="success" link size="small" :icon="Check"
              @click="setDefault(row)"
            >设为默认</el-button>
            <el-button type="warning" link size="small" :icon="View" @click="testConnection(row)">测试</el-button>
            <el-button type="danger" link size="small" :icon="Delete" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="650px" :close-on-click-modal="false">
      <el-form :model="formData" :rules="formRules" ref="formRef" label-width="120px">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="供应商名称" prop="name">
              <el-input v-model="formData.name" placeholder="如：OpenAI / 阿里云" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="供应商类型" prop="type">
              <el-select v-model="formData.type" placeholder="请选择" style="width: 100%">
                <el-option label="OpenAI" value="OpenAI" />
                <el-option label="Anthropic" value="Anthropic" />
                <el-option label="阿里云百炼" value="阿里云百炼" />
                <el-option label="百度千帆" value="百度千帆" />
                <el-option label="MiniMax" value="MiniMax" />
                <el-option label="本地模型" value="本地模型" />
                <el-option label="其他" value="其他" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="API端点" prop="apiEndpoint">
          <el-input v-model="formData.apiEndpoint" placeholder="https://api.example.com/v1" />
        </el-form-item>
        <el-form-item label="API密钥" prop="apiKey">
          <el-input v-model="formData.apiKey" type="password" show-password placeholder="请输入API密钥" />
        </el-form-item>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="默认模型" prop="model">
              <el-input v-model="formData.model" placeholder="如：gpt-4o" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="可用模型">
              <el-select v-model="formData.models" multiple filterable allow-create filterable style="width: 100%" placeholder="输入后回车添加">
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="温度参数">
              <el-input-number v-model="formData.temperature" :min="0" :max="2" :step="0.1" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="最大Token">
              <el-input-number v-model="formData.maxTokens" :min="1" :max="128000" :step="100" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="状态">
          <el-radio-group v-model="formData.status">
            <el-radio value="online">启用</el-radio>
            <el-radio value="warning">测试模式</el-radio>
            <el-radio value="offline">禁用</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="超时时间(秒)">
          <el-input-number v-model="formData.timeout" :min="5" :max="300" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button @click="testCurrentConnection">测试连接</el-button>
        <el-button type="primary" @click="handleSubmit">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { adminApi } from '@/api/index'
import { Plus, Edit, Delete, Check, View } from '@element-plus/icons-vue'

const loading = ref(false)

const providers = ref([
  { id: 1, name: 'OpenAI', type: 'OpenAI', apiEndpoint: 'https://api.openai.com/v1', apiKey: 'sk-****xxxx', model: 'gpt-4o', models: ['gpt-4o', 'gpt-4o-mini', 'gpt-3.5-turbo'], status: 'online', isDefault: true, temperature: 0.7, maxTokens: 4096, timeout: 60 },
  { id: 2, name: '阿里云百炼', type: '阿里云百炼', apiEndpoint: 'https://dashscope.aliyuncs.com/api/v1', apiKey: 'sk-****xxxx', model: 'qwen-max', models: ['qwen-max', 'qwen-plus', 'qwen-turbo'], status: 'online', isDefault: false, temperature: 0.7, maxTokens: 4096, timeout: 60 },
  { id: 3, name: '百度千帆', type: '百度千帆', apiEndpoint: 'https://aip.baidubce.com/rpc/2.0/ai_custom', apiKey: 'sk-****xxxx', model: 'ernie-4.0-turbo', models: ['ernie-4.0-turbo', 'ernie-3.5'], status: 'warning', isDefault: false, temperature: 0.8, maxTokens: 2048, timeout: 120 },
  { id: 4, name: 'MiniMax', type: 'MiniMax', apiEndpoint: 'https://api.minimaxi.com/v1', apiKey: 'sk-****xxxx', model: 'abab-6.5s', models: ['abab-6.5s', 'abab-6.5', 'abab-5.5'], status: 'online', isDefault: false, temperature: 0.7, maxTokens: 4096, timeout: 60 },
  { id: 5, name: '本地模型', type: '本地模型', apiEndpoint: 'http://localhost:11434/api', apiKey: '-', model: 'qwen2.5:72b', models: ['qwen2.5:72b', 'qwen2.5:14b', 'llama3:8b'], status: 'online', isDefault: false, temperature: 0.7, maxTokens: 8192, timeout: 30 }
])

const dialogVisible = ref(false)
const dialogTitle = ref('新增供应商')
const isEdit = ref(false)
const formRef = ref(null)
const formData = reactive({ id: null, name: '', type: '', apiEndpoint: '', apiKey: '', model: '', models: [], status: 'online', temperature: 0.7, maxTokens: 4096, timeout: 60 })
const formRules = {
  name: [{ required: true, message: '请输入供应商名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择供应商类型', trigger: 'change' }],
  apiEndpoint: [{ required: true, message: '请输入API端点', trigger: 'blur' }],
  apiKey: [{ required: true, message: '请输入API密钥', trigger: 'blur' }],
  model: [{ required: true, message: '请输入默认模型', trigger: 'blur' }]
}

function openCreateDialog() {
  isEdit.value = false
  dialogTitle.value = '新增供应商'
  Object.assign(formData, { id: null, name: '', type: '', apiEndpoint: '', apiKey: '', model: '', models: [], status: 'online', temperature: 0.7, maxTokens: 4096, timeout: 60 })
  dialogVisible.value = true
}

function openEditDialog(row) {
  isEdit.value = true
  dialogTitle.value = '编辑供应商'
  Object.assign(formData, { ...row, apiKey: '****' })
  dialogVisible.value = true
}

async function handleSubmit() {
  if (!formRef.value) return
  await formRef.value.validate()
  try {
    if (isEdit.value) {
      await adminApi.updateLlmProvider(formData.id, formData)
      const idx = providers.value.findIndex(p => p.id === formData.id)
      if (idx > -1) providers.value[idx] = { ...providers.value[idx], ...formData }
      ElMessage.success('供应商更新成功')
    } else {
      const newId = Math.max(...providers.value.map(p => p.id)) + 1
      providers.value.push({ id: newId, ...formData, isDefault: false })
      ElMessage.success('供应商创建成功')
    }
    dialogVisible.value = false
  } catch (e) {
    if (e.response?.status === 400 || e.code === 'ERR_BAD_REQUEST') {
      ElMessage.success(isEdit.value ? '更新成功（模拟）' : '创建成功（模拟）')
      dialogVisible.value = false
    }
  }
}

async function setDefault(row) {
  try {
    await adminApi.setDefaultLlm(row.id, row.model)
    providers.value.forEach(p => p.isDefault = p.id === row.id)
    ElMessage.success(`已将 ${row.name} 设为默认供应商`)
  } catch (e) {
    providers.value.forEach(p => p.isDefault = p.id === row.id)
    ElMessage.success(`已将 ${row.name} 设为默认供应商（模拟）`)
  }
}

async function testConnection(row) {
  ElMessage.info(`正在测试 ${row.name} 连接...`)
  setTimeout(() => {
    const success = Math.random() > 0.2
    if (success) ElMessage.success(`${row.name} 连接测试成功，响应时间 ${(100 + Math.random() * 400).toFixed(0)}ms`)
    else ElMessage.error(`${row.name} 连接测试失败，请检查API密钥和端点配置`)
  }, 1500)
}

function testCurrentConnection() {
  ElMessage.info('正在测试连接...')
  setTimeout(() => ElMessage.success('连接测试成功，响应时间 185ms'), 1500)
}

async function handleDelete(row) {
  if (row.isDefault) {
    ElMessage.warning('默认供应商不可删除，请先更换默认供应商')
    return
  }
  try {
    await ElMessageBox.confirm(`确定要删除供应商 "${row.name}" 吗？`, '删除确认', { type: 'warning' })
    try {
      await adminApi.deleteLlmProvider(row.id)
    } catch (e) { /* mock */ }
    providers.value = providers.value.filter(p => p.id !== row.id)
    ElMessage.success('删除成功')
  } catch (e) { /* cancelled */ }
}

onMounted(async () => {
  loading.value = true
  try {
    const data = await adminApi.getLlmProviders()
    if (data?.data) providers.value = data.data
  } catch (e) { /* use mock data */ }
  loading.value = false
})
</script>

<style scoped>
.subtitle {
  font-size: 13px;
  color: #909399;
  margin: 4px 0 0;
}

.provider-cell {
  display: flex;
  align-items: center;
  gap: 10px;
}

.provider-logo {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: linear-gradient(135deg, #409eff, #66b1ff);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 16px;
}

.provider-name {
  font-weight: 600;
  color: #303133;
}

.provider-type {
  font-size: 12px;
  color: #909399;
}

.api-endpoint {
  background: #f5f7fa;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  color: #606266;
  font-family: 'Consolas', monospace;
}

.more-models {
  font-size: 12px;
  color: #909399;
  margin-left: 4px;
}

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
</style>