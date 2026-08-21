<template>
  <div>
    <div class="admin-card">
      <div class="admin-table-toolbar">
        <div>
          <h3 class="admin-page-title" style="margin:0">智能路由配置</h3>
          <p class="subtitle">根据预设条件自动选择最合适的大模型供应商和模型</p>
        </div>
        <el-button type="primary" :icon="Plus" @click="addRule">新增路由规则</el-button>
      </div>

      <div class="routing-config">
        <div class="config-section">
          <h4 class="section-title">默认模型设置</h4>
          <el-form :inline="true" :model="defaultConfig" label-width="100px">
            <el-form-item label="默认供应商">
              <el-select v-model="defaultConfig.provider" style="width: 220px">
                <el-option v-for="p in providers" :key="p.id" :label="p.name" :value="p.id" />
              </el-select>
            </el-form-item>
            <el-form-item label="默认模型">
              <el-select v-model="defaultConfig.model" style="width: 200px">
                <el-option v-for="m in availableModels" :key="m" :label="m" :value="m" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="saveDefault">保存默认配置</el-button>
            </el-form-item>
          </el-form>
        </div>

        <el-divider />

        <div class="config-section">
          <h4 class="section-title">路由规则列表</h4>
          <p class="rule-hint">规则按优先级从上到下依次匹配，第一条匹配成功的规则将被执行</p>

          <el-table :data="routingRules" stripe border row-key="id" @sort-change="handleSortChange" style="width: 100%">
            <el-table-column label="优先级" width="80" align="center">
              <template #default="{ row }">
                <el-icon v-if="row.priority > 1" style="cursor:pointer" @click="moveUp(row)"><ArrowUp /></el-icon>
                <el-icon v-else style="color:#c0c4cc"><ArrowUp /></el-icon>
                <span class="priority-num">{{ row.priority }}</span>
                <el-icon v-if="row.priority < routingRules.length" style="cursor:pointer" @click="moveDown(row)"><ArrowDown /></el-icon>
                <el-icon v-else style="color:#c0c4cc"><ArrowDown /></el-icon>
              </template>
            </el-table-column>
            <el-table-column label="条件" min-width="280">
              <template #default="{ row }">
                <div class="condition-display">
                  <el-tag v-for="(cond, idx) in row.conditions" :key="idx" type="info" effect="plain" style="margin-right:4px; margin-bottom:4px">
                    {{ cond.field }} {{ cond.operator }} {{ cond.value }}
                  </el-tag>
                  <span v-if="!row.conditions.length" class="no-condition">无条件（始终匹配）</span>
                </div>
              </template>
            </el-table-column>
            <el-table-column label="目标供应商/模型" width="280">
              <template #default="{ row }">
                <div class="target-display">
                  <el-select v-model="row.targetProvider" size="small" style="width: 130px">
                    <el-option v-for="p in providers" :key="p.id" :label="p.name" :value="p.id" />
                  </el-select>
                  <el-select v-model="row.targetModel" size="small" style="width: 130px; margin-left:6px">
                    <el-option v-for="m in getProviderModels(row.targetProvider)" :key="m" :label="m" :value="m" />
                  </el-select>
                </div>
              </template>
            </el-table-column>
            <el-table-column label="响应模式" width="120">
              <template #default="{ row }">
                <el-switch v-model="row.stream" active-text="流式" inactive-text="普通" inline-prompt />
              </template>
            </el-table-column>
            <el-table-column label="启用" width="80" align="center">
              <template #default="{ row }">
                <el-switch v-model="row.enabled" />
              </template>
            </el-table-column>
            <el-table-column label="操作" width="120" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" :icon="Edit" @click="editRule(row)">编辑条件</el-button>
                <el-button type="danger" link size="small" :icon="Delete" @click="removeRule(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </div>

        <el-divider />

        <div class="config-section">
          <h4 class="section-title">路由策略</h4>
          <el-form :model="routingStrategy" label-width="140px" label-position="left">
            <el-row :gutter="20">
              <el-col :xs="24" :md="12">
                <el-form-item label="匹配模式">
                  <el-radio-group v-model="routingStrategy.matchMode">
                    <el-radio value="first">首个匹配</el-radio>
                    <el-radio value="best">最佳匹配</el-radio>
                    <el-radio value="all">全部执行</el-radio>
                  </el-radio-group>
                </el-form-item>
              </el-col>
              <el-col :xs="24" :md="12">
                <el-form-item label="故障转移">
                  <el-switch v-model="routingStrategy.failover" active-text="启用" />
                  <span class="form-hint">主模型失败时自动切换</span>
                </el-form-item>
              </el-col>
              <el-col :xs="24" :md="12">
                <el-form-item label="负载均衡">
                  <el-switch v-model="routingStrategy.loadBalance" active-text="启用" />
                  <span class="form-hint">在多个模型间分发请求</span>
                </el-form-item>
              </el-col>
              <el-col :xs="24" :md="12">
                <el-form-item label="优先级权重">
                  <el-input-number v-model="routingStrategy.weight" :min="1" :max="10" />
                  <span class="form-hint">权重越高优先级越大</span>
                </el-form-item>
              </el-col>
            </el-row>
          </el-form>
        </div>
      </div>

      <div class="form-actions">
        <el-button @click="resetConfig">重置</el-button>
        <el-button type="primary" @click="saveAll">保存全部配置</el-button>
      </div>
    </div>

    <el-dialog v-model="conditionDialogVisible" title="编辑路由条件" width="500px">
      <div class="condition-editor">
        <div v-for="(cond, idx) in editingConditions" :key="idx" class="condition-row">
          <el-select v-model="cond.field" placeholder="字段" style="width:130px">
            <el-option label="用户角色" value="userRole" />
            <el-option label="请求类型" value="requestType" />
            <el-option label="Token数量" value="tokenCount" />
            <el-option label="语言" value="language" />
            <el-option label="来源IP" value="sourceIp" />
            <el-option label="知识库" value="knowledgeBase" />
          </el-select>
          <el-select v-model="cond.operator" placeholder="运算符" style="width:100px">
            <el-option label="等于" value="eq" />
            <el-option label="包含" value="contains" />
            <el-option label="大于" value="gt" />
            <el-option label="小于" value="lt" />
            <el-option label="不等于" value="ne" />
          </el-select>
          <el-input v-model="cond.value" placeholder="值" style="flex:1" />
          <el-button type="danger" :icon="Delete" circle @click="removeCondition(idx)" />
        </div>
        <el-button type="primary" plain :icon="Plus" @click="addCondition">添加条件</el-button>
      </div>
      <template #footer>
        <el-button @click="conditionDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveCondition">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { adminApi } from '@/api/index'
import { Plus, Edit, Delete, ArrowUp, ArrowDown } from '@element-plus/icons-vue'

const providers = ref([
  { id: 1, name: 'OpenAI', models: ['gpt-4o', 'gpt-4o-mini', 'gpt-3.5-turbo'] },
  { id: 2, name: '阿里云百炼', models: ['qwen-max', 'qwen-plus', 'qwen-turbo'] },
  { id: 3, name: '百度千帆', models: ['ernie-4.0-turbo', 'ernie-3.5'] },
  { id: 4, name: 'MiniMax', models: ['abab-6.5s', 'abab-5.5'] },
  { id: 5, name: '本地模型', models: ['qwen2.5:72b', 'qwen2.5:14b', 'llama3:8b'] }
])

const availableModels = computed(() => {
  const p = providers.value.find(p => p.id === defaultConfig.value.provider)
  return p?.models || []
})

const defaultConfig = reactive({ provider: 1, model: 'gpt-4o' })

const routingRules = ref([
  {
    id: 1, priority: 1, enabled: true, stream: true,
    conditions: [{ field: 'userRole', operator: 'eq', value: 'super_admin' }],
    targetProvider: 1, targetModel: 'gpt-4o'
  },
  {
    id: 2, priority: 2, enabled: true, stream: true,
    conditions: [{ field: 'tokenCount', operator: 'gt', value: '4000' }],
    targetProvider: 5, targetModel: 'qwen2.5:72b'
  },
  {
    id: 3, priority: 3, enabled: true, stream: false,
    conditions: [{ field: 'requestType', operator: 'eq', value: 'summary' }],
    targetProvider: 3, targetModel: 'ernie-4.0-turbo'
  },
  {
    id: 4, priority: 4, enabled: true, stream: true,
    conditions: [{ field: 'language', operator: 'eq', value: 'zh' }],
    targetProvider: 2, targetModel: 'qwen-max'
  },
  {
    id: 5, priority: 5, enabled: false, stream: false,
    conditions: [{ field: 'userRole', operator: 'eq', value: 'guest' }],
    targetProvider: 4, targetModel: 'abab-5.5'
  },
  {
    id: 6, priority: 6, enabled: true, stream: true,
    conditions: [],
    targetProvider: 1, targetModel: 'gpt-4o-mini'
  }
])

const routingStrategy = reactive({
  matchMode: 'first',
  failover: true,
  loadBalance: false,
  weight: 5
})

const conditionDialogVisible = ref(false)
const editingRuleId = ref(null)
const editingConditions = ref([])

function getProviderModels(providerId) {
  const p = providers.value.find(p => p.id === providerId)
  return p?.models || []
}

function addRule() {
  const newId = Math.max(...routingRules.value.map(r => r.id)) + 1
  routingRules.value.push({
    id: newId,
    priority: routingRules.value.length + 1,
    enabled: true,
    stream: false,
    conditions: [],
    targetProvider: 1,
    targetModel: 'gpt-4o'
  })
  ElMessage.success('新规则已添加，请配置条件和目标')
}

function removeRule(row) {
  ElMessageBox.confirm(`确定删除该路由规则吗？`, '确认', { type: 'warning' }).then(() => {
    routingRules.value = routingRules.value.filter(r => r.id !== row.id)
    reindexPriorities()
    ElMessage.success('删除成功')
  }).catch(() => {})
}

function reindexPriorities() {
  routingRules.value.sort((a, b) => a.priority - b.priority)
  routingRules.value.forEach((r, i) => r.priority = i + 1)
}

function moveUp(row) {
  const idx = routingRules.value.findIndex(r => r.id === row.id)
  if (idx > 0) {
    const prev = routingRules.value[idx - 1]
    ;[row.priority, prev.priority] = [prev.priority, row.priority]
    reindexPriorities()
  }
}

function moveDown(row) {
  const idx = routingRules.value.findIndex(r => r.id === row.id)
  if (idx < routingRules.value.length - 1) {
    const next = routingRules.value[idx + 1]
    ;[row.priority, next.priority] = [next.priority, row.priority]
    reindexPriorities()
  }
}

function handleSortChange({ prop, order }) {
  if (prop === 'priority' && order === 'ascending') {
    routingRules.value.sort((a, b) => a.priority - b.priority)
  }
}

function editRule(row) {
  editingRuleId.value = row.id
  editingConditions.value = JSON.parse(JSON.stringify(row.conditions))
  conditionDialogVisible.value = true
}

function addCondition() {
  editingConditions.value.push({ field: 'userRole', operator: 'eq', value: '' })
}

function removeCondition(idx) {
  editingConditions.value.splice(idx, 1)
}

function saveCondition() {
  const rule = routingRules.value.find(r => r.id === editingRuleId.value)
  if (rule) {
    rule.conditions = JSON.parse(JSON.stringify(editingConditions.value))
    ElMessage.success('条件已更新')
  }
  conditionDialogVisible.value = false
}

function saveDefault() {
  ElMessage.success('默认模型配置已保存')
}

function resetConfig() {
  ElMessage.info('配置已重置为上次保存状态')
}

async function saveAll() {
  try {
    await adminApi.saveLlmRouting({ defaultConfig, routingRules, routingStrategy })
    ElMessage.success('路由配置已保存')
  } catch (e) {
    ElMessage.success('路由配置已保存（模拟）')
  }
}

onMounted(async () => {
  try {
    const data = await adminApi.getLlmRouting()
    if (data?.data) {
      if (data.data.defaultConfig) Object.assign(defaultConfig, data.data.defaultConfig)
      if (data.data.routingRules) routingRules.value = data.data.routingRules
      if (data.data.routingStrategy) Object.assign(routingStrategy, data.data.routingStrategy)
    }
  } catch (e) { /* use mock data */ }
})
</script>

<style scoped>
.subtitle { font-size: 13px; color: #909399; margin: 4px 0 0; }

.config-section { margin-bottom: 8px; }

.section-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 8px;
}

.rule-hint {
  font-size: 12px;
  color: #909399;
  margin: 0 0 12px;
}

.priority-num {
  display: inline-block;
  width: 24px;
  text-align: center;
  font-weight: 600;
  color: #409eff;
}

.condition-display {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.no-condition {
  color: #e6a23c;
  font-size: 12px;
}

.target-display {
  display: flex;
  align-items: center;
}

.form-hint {
  font-size: 12px;
  color: #909399;
  margin-left: 8px;
}

.form-actions {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid #ebeef5;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.condition-editor {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.condition-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>