<template>
  <div class="workflow-flows-panel" v-loading="loading">
    <!-- 工具栏 -->
    <div class="flows-toolbar">
      <div class="toolbar-left">
        <span class="flows-title">流程编排</span>
        <el-tag size="small" type="info">{{ flows.length }} 个流程</el-tag>
      </div>
      <div class="toolbar-right">
        <el-input
          v-model="keyword"
          placeholder="搜索流程..."
          clearable
          size="default"
          style="width: 220px"
          :prefix-icon="Search"
        />
        <el-button :icon="Refresh" @click="loadFlows" :loading="loading">刷新</el-button>
        <el-button type="primary" :icon="Plus" @click="openCreateDialog">新建流程</el-button>
      </div>
    </div>

    <!-- 错误态 -->
    <div v-if="error" class="flows-error">
      <el-empty :description="error" :image-size="80">
        <el-button type="primary" @click="loadFlows">重试</el-button>
      </el-empty>
    </div>

    <!-- 流程列表 -->
    <template v-else>
      <div v-if="filteredFlows.length" class="flows-grid">
        <div
          v-for="flow in filteredFlows"
          :key="flow.id"
          class="flow-card"
          @click="selectFlow(flow)"
        >
          <div class="flow-card-header">
            <div class="flow-icon" :style="{ background: flowColor(flow.type) }">
              <el-icon :size="20"><component :is="flowIcon(flow.type)" /></el-icon>
            </div>
            <div class="flow-info">
              <div class="flow-name">{{ flow.name || '未命名流程' }}</div>
              <div class="flow-type">{{ typeLabel(flow.type) }}</div>
            </div>
            <el-tag :type="statusTagType(flow.status)" size="small">
              {{ statusLabel(flow.status) }}
            </el-tag>
          </div>
          <div class="flow-card-body">
            <p class="flow-desc">{{ flow.description || '暂无描述' }}</p>
            <div class="flow-meta">
              <span><el-icon><Timer /></el-icon> {{ flow.updated_at || flow.created_at || '—' }}</span>
              <span v-if="flow.node_count != null"><el-icon><Connection /></el-icon> {{ flow.node_count }} 节点</span>
              <span v-if="flow.run_count != null"><el-icon><VideoPlay /></el-icon> {{ flow.run_count }} 次运行</span>
            </div>
          </div>
          <div class="flow-card-actions" @click.stop>
            <el-button size="small" type="primary" plain :icon="VideoPlay" @click="executeFlow(flow)">执行</el-button>
            <el-button size="small" :icon="Edit" @click="editFlow(flow)">编辑</el-button>
            <el-button size="small" type="danger" plain :icon="Delete" @click="removeFlow(flow)">删除</el-button>
          </div>
        </div>
      </div>

      <!-- 空态 -->
      <div v-else class="flows-empty">
        <el-empty description="暂无流程，点击上方新建流程开始编排" :image-size="100">
          <el-button type="primary" :icon="Plus" @click="openCreateDialog">新建流程</el-button>
        </el-empty>
      </div>
    </template>

    <!-- 新建/编辑流程 Dialog -->
    <el-dialog v-model="dialog.visible" :title="dialog.isEdit ? '编辑流程' : '新建流程'" width="480px">
      <el-form :model="dialog.form" label-width="80px">
        <el-form-item label="流程名称">
          <el-input v-model="dialog.form.name" placeholder="如：智能文档处理流程" maxlength="60" />
        </el-form-item>
        <el-form-item label="流程类型">
          <el-select v-model="dialog.form.type" placeholder="选择类型" style="width: 100%">
            <el-option label="数据处理" value="data" />
            <el-option label="AI 推理" value="ai" />
            <el-option label="自动化" value="automation" />
            <el-option label="集成" value="integration" />
            <el-option label="自定义" value="custom" />
          </el-select>
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="dialog.form.description" type="textarea" :rows="3" placeholder="流程说明（可选）" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialog.visible = false">取消</el-button>
        <el-button type="primary" :loading="dialog.saving" @click="saveFlow">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Plus, Search, Refresh, Edit, Delete, VideoPlay,
  Connection, Timer, Cpu, DataAnalysis, MagicStick, Setting, Tools
} from '@element-plus/icons-vue'
import { getFlows, createFlow, deleteFlow, executeFlow as apiExecuteFlow } from '@/api/workflow.api.js'

const loading = ref(false)
const error = ref('')
const flows = ref([])
const keyword = ref('')

const dialog = ref({
  visible: false,
  isEdit: false,
  saving: false,
  form: { id: null, name: '', type: 'custom', description: '' }
})

const filteredFlows = computed(() => {
  if (!keyword.value.trim()) return flows.value
  const kw = keyword.value.trim().toLowerCase()
  return flows.value.filter(f =>
    (f.name || '').toLowerCase().includes(kw) ||
    (f.description || '').toLowerCase().includes(kw)
  )
})

async function loadFlows() {
  loading.value = true
  error.value = ''
  try {
    const data = await getFlows()
    if (Array.isArray(data)) {
      flows.value = data
    } else if (Array.isArray(data?.list)) {
      flows.value = data.list
    } else if (Array.isArray(data?.data)) {
      flows.value = data.data
    } else if (Array.isArray(data?.flows)) {
      flows.value = data.flows
    } else {
      flows.value = []
    }
  } catch (e) {
    error.value = e?.message || '流程列表加载失败'
    flows.value = []
    ElMessage.error('流程列表加载失败：' + (e?.message || '未知错误'))
  } finally {
    loading.value = false
  }
}

function openCreateDialog() {
  dialog.value = {
    visible: true,
    isEdit: false,
    saving: false,
    form: { id: null, name: '', type: 'custom', description: '' }
  }
}

function editFlow(flow) {
  dialog.value = {
    visible: true,
    isEdit: true,
    saving: false,
    form: { id: flow.id, name: flow.name || '', type: flow.type || 'custom', description: flow.description || '' }
  }
}

async function saveFlow() {
  const f = dialog.value.form
  if (!f.name.trim()) {
    ElMessage.warning('请输入流程名称')
    return
  }
  dialog.value.saving = true
  try {
    if (dialog.value.isEdit) {
      ElMessage.info('编辑功能开发中')
    } else {
      await createFlow({ name: f.name, type: f.type, description: f.description })
      ElMessage.success('流程创建成功')
    }
    dialog.value.visible = false
    await loadFlows()
  } catch (e) {
    ElMessage.error('保存失败：' + (e?.message || '未知错误'))
  } finally {
    dialog.value.saving = false
  }
}

async function removeFlow(flow) {
  try {
    await ElMessageBox.confirm(
      `确定删除流程「${flow.name || '未命名'}」吗？删除后不可恢复。`,
      '删除确认',
      { type: 'warning' }
    )
    await deleteFlow(flow.id)
    ElMessage.success('删除成功')
    await loadFlows()
  } catch (e) {
    if (e !== 'cancel' && e?.message) {
      ElMessage.error('删除失败：' + e.message)
    }
  }
}

async function executeFlow(flow) {
  try {
    await apiExecuteFlow({ flow_id: flow.id, id: flow.id })
    ElMessage.success(`流程「${flow.name || '未命名'}」已开始执行`)
  } catch (e) {
    ElMessage.error('执行失败：' + (e?.message || '未知错误'))
  }
}

function selectFlow(flow) {
  ElMessage.info(`已选择流程：${flow.name || '未命名'}`)
}

function typeLabel(type) {
  const map = { data: '数据处理', ai: 'AI 推理', automation: '自动化', integration: '集成', custom: '自定义' }
  return map[type] || type || '自定义'
}

function statusLabel(status) {
  const map = { active: '已启用', inactive: '已停用', running: '运行中', error: '异常', draft: '草稿' }
  return map[status] || status || '草稿'
}

function statusTagType(status) {
  const map = { active: 'success', inactive: 'info', running: 'warning', error: 'danger', draft: 'info' }
  return map[status] || 'info'
}

function flowColor(type) {
  const map = {
    data: '#10b981', ai: '#6366f1', automation: '#f59e0b',
    integration: '#06b6d4', custom: '#64748b'
  }
  return map[type] || '#64748b'
}

function flowIcon(type) {
  const map = {
    data: DataAnalysis, ai: Cpu, automation: MagicStick,
    integration: Connection, custom: Setting
  }
  return map[type] || Tools
}

onMounted(() => {
  loadFlows()
})
</script>

<style scoped>
.workflow-flows-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 16px;
  box-sizing: border-box;
  background: var(--bg-primary, #0f1117);
}

.flows-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
  flex-shrink: 0;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.flows-title {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary, #e8eaed);
}

.toolbar-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.flows-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 14px;
  overflow-y: auto;
  flex: 1;
  padding-right: 4px;
}

.flow-card {
  background: var(--bg-card, #1a1d2e);
  border: 1px solid var(--border, #2d3148);
  border-radius: 10px;
  padding: 16px;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  flex-direction: column;
}

.flow-card:hover {
  border-color: var(--accent, #6366f1);
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.15);
  transform: translateY(-2px);
}

.flow-card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.flow-icon {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  color: #fff;
  flex-shrink: 0;
}

.flow-info {
  flex: 1;
  min-width: 0;
}

.flow-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary, #e8eaed);
  margin-bottom: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.flow-type {
  font-size: 11px;
  color: var(--text-muted, #6b7280);
}

.flow-card-body {
  flex: 1;
  margin-bottom: 12px;
}

.flow-desc {
  font-size: 12px;
  color: var(--text-secondary, #9aa0b4);
  line-height: 1.5;
  margin: 0 0 10px 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.flow-meta {
  display: flex;
  gap: 14px;
  font-size: 11px;
  color: var(--text-muted, #6b7280);
  flex-wrap: wrap;
}

.flow-meta span {
  display: flex;
  align-items: center;
  gap: 4px;
}

.flow-card-actions {
  display: flex;
  gap: 6px;
  padding-top: 12px;
  border-top: 1px solid var(--border-ghost, #252840);
}

.flows-empty,
.flows-error {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
