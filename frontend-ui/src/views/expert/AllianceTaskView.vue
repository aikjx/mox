<template>
  <div class="alliance-task">
    <!-- ===== 左栏：任务列表 ===== -->
    <div class="task-list-col" v-loading="tasksLoading">
      <div class="list-header">
        <div>
          <h2 class="list-title">联盟任务</h2>
          <p class="list-subtitle">描述目标，让专家协作完成任务</p>
        </div>
        <el-button type="primary" size="small" @click="showCreate = true" :disabled="!runtimeReady">
          <el-icon><Plus /></el-icon> 新建任务
        </el-button>
      </div>

      <!-- 筛选 chips -->
      <div class="filter-row">
        <button
          type="button"
          v-for="f in statusFilters"
          :key="f.key"
          class="filter-chip"
          :class="{ active: currentFilter === f.key }"
          @click="currentFilter = f.key"
        >
          {{ f.label }} ({{ getStatusCount(f.key) }})
        </button>
      </div>

      <el-alert v-if="tasksError" type="error" :closable="false" show-icon title="任务列表加载失败" :description="tasksError" style="margin-bottom:8px" />
      <el-button size="small" :loading="tasksLoading || runtimeLoading" @click="refreshWorkspace">刷新任务</el-button>
      <el-alert v-if="!runtimeReady" type="warning" :closable="false" :title="runtimeMessage" style="margin-top:8px" />
      <el-alert v-if="pollError" type="warning" :closable="false" :title="pollError" style="margin-top:8px" />

      <!-- 任务列表 -->
      <div class="task-list" v-if="filteredTasks.length > 0">
        <div
          v-for="task in filteredTasks"
          :key="task.id"
          class="task-item"
          :class="{ active: selectedTask?.id === task.id, done: task.status === 'completed' }"
          @click="selectTask(task)"
          role="button" tabindex="0" :aria-label="task.name"
          @keydown.enter="selectTask(task)" @keydown.space.prevent="selectTask(task)"
        >
          <!-- Execution status is read-only; completion comes from the executor. -->
          <div
            class="task-checkbox"
            :class="{ done: task.status === 'completed' }"
            :title="statusLabel(task.status)"
          >
            <span v-if="task.status === 'completed'" class="check-icon">✓</span>
          </div>

          <div class="task-info">
            <div class="task-title-row">
              <span class="task-title">{{ task.name }}</span>
              <span
                class="task-priority"
                :class="`priority-${getPriority(task)}`"
              >
                {{ getPriorityLabel(getPriority(task)) }}
              </span>
            </div>
            <div class="task-desc">{{ task.description }}</div>
            <div class="task-meta">
              <span class="meta-item">
                <el-icon><User /></el-icon>
                {{ getAssignee(task) }}
              </span>
              <span class="meta-item">
                <el-icon><Calendar /></el-icon>
                {{ getDueDate(task) }}
              </span>
              <span class="meta-item progress-meta">
                <span class="progress-bar-mini">
                  <span class="progress-fill-mini" :style="{ width: task.progress + '%' }"></span>
                </span>
                {{ task.progress }}%
              </span>
            </div>
          </div>

          <div class="task-status-badge" :class="task.status">
            {{ statusLabel(task.status) }}
          </div>
        </div>
      </div>

      <div v-else-if="!tasksLoading && !tasksError" class="empty-list">
        <el-empty :description="currentFilter === 'all' ? '暂无任务' : '暂无' + (statusFilters.find(f => f.key === currentFilter)?.label || '') + '任务'" :image-size="60" />
      </div>
    </div>

    <!-- ===== 右栏：任务详情 ===== -->
    <div class="task-detail-col" v-if="selectedTask">
      <!-- 任务头部 -->
      <div class="detail-header">
        <div>
          <h3 class="detail-title">{{ selectedTask.name }}</h3>
          <p class="detail-desc">{{ selectedTask.description }}</p>
        </div>
        <div class="detail-actions">
          <el-button size="small" @click="runTask" :loading="actionPending" :disabled="!canResume">
            <el-icon><VideoPlay /></el-icon> {{ selectedTask.status === 'paused' ? '继续执行' : '启动任务' }}
          </el-button>
          <el-button size="small" @click="pauseTask" :disabled="actionPending || !canPause">
            <el-icon><VideoPause /></el-icon> 暂停
          </el-button>
          <el-button size="small" type="danger" @click="cancelTask" :disabled="actionPending || !canCancel">
            <el-icon><Close /></el-icon> 取消
          </el-button>
        </div>
      </div>

      <!-- 进度条 -->
      <div class="progress-section">
        <div class="progress-label">
          <span>执行进度</span>
          <span class="progress-value">{{ selectedTask.progress }}%</span>
        </div>
        <div class="progress-bar-large">
          <div class="progress-fill-large" :style="{ width: selectedTask.progress + '%' }"></div>
        </div>
        <div class="progress-stats">
          <span><el-icon><Timer /></el-icon> 耗时: {{ selectedTask.duration_ms != null ? (selectedTask.duration_ms / 1000).toFixed(1) + 's' : '--' }}</span>
          <span><el-icon><Clock /></el-icon> 预计: {{ selectedTask.eta || '--' }}</span>
          <span><el-icon><User /></el-icon> 专家: {{ selectedTask.expert_count ?? dagNodes.length }} 位</span>
        </div>
      </div>

      <!-- DAG 画布 -->
      <div class="dag-section" v-loading="dagLoading">
        <div class="section-header">
          <h4 class="section-title"><el-icon><Share /></el-icon> 执行流程</h4>
          <div class="dag-legend">
            <span class="legend-item"><span class="legend-dot pending"></span>待处理</span>
            <span class="legend-item"><span class="legend-dot running"></span>运行中</span>
            <span class="legend-item"><span class="legend-dot completed"></span>已完成</span>
            <span class="legend-item"><span class="legend-dot failed"></span>失败</span>
          </div>
        </div>
        <el-alert v-if="dagError" type="error" :closable="false" show-icon title="执行流程加载失败" style="margin-bottom:8px" />
        <div class="dag-canvas" v-if="dagNodes.length > 0 || dagEdges.length > 0">
          <svg class="dag-svg" :viewBox="`0 0 ${dagWidth} ${dagHeight}`">
            <!-- 连线 -->
            <line
              v-for="(edge, i) in dagEdges"
              :key="'e' + i"
              :x1="edge.x1" :y1="edge.y1"
              :x2="edge.x2" :y2="edge.y2"
              class="dag-edge"
              :class="edge.status"
            />
            <!-- 节点 -->
            <g
              v-for="node in dagNodes"
              :key="node.id"
              class="dag-node-group"
              :transform="`translate(${node.x}, ${node.y})`"
            >
              <rect
                class="dag-node"
                :class="node.status"
                x="-60" y="-20"
                width="120" height="40"
                rx="8"
              />
              <text class="dag-node-label" x="0" y="-2" text-anchor="middle">{{ node.name }}</text>
              <text class="dag-node-sub" x="0" y="12" text-anchor="middle">{{ node.type }}</text>
            </g>
          </svg>
        </div>
        <el-empty v-else-if="!dagLoading && !dagError" description="任务规划后将在这里显示执行流程" :image-size="50" />
      </div>

      <!-- 日志 -->
      <div class="logs-section" v-loading="logsLoading">
        <div class="section-header">
          <h4 class="section-title"><el-icon><Document /></el-icon> 执行记录</h4>
          <el-button size="small" text @click="clearLogs">清空显示</el-button>
        </div>
        <el-alert v-if="logsError" type="error" :closable="false" show-icon title="执行记录加载失败" style="margin-bottom:8px" />
        <p class="form-hint">执行记录来自节点最新状态；完整过程以服务审计日志为准。</p>
        <div class="logs-container" ref="logsContainer">
          <div
            v-for="(log, i) in logs"
            :key="i"
            class="log-line"
            :class="log.level"
          >
            <span class="log-time">{{ log.time }}</span>
            <span class="log-level" :class="log.level">{{ log.level.toUpperCase() }}</span>
            <span class="log-msg">{{ log.message }}</span>
          </div>
          <div v-if="logs.length === 0 && !logsLoading" class="logs-empty">暂无日志</div>
        </div>
      </div>

      <!-- 融合结果 -->
      <div class="fusion-section" v-if="fusionResult" v-loading="fusionLoading">
        <div class="section-header">
          <h4 class="section-title"><el-icon><MagicStick /></el-icon> 任务结果</h4>
          <el-tag v-if="Number.isFinite(fusionResult.confidence)" size="small" :type="fusionResult.confidence > 0.8 ? 'success' : 'warning'" effect="dark">
            置信度 {{ (fusionResult.confidence * 100).toFixed(1) }}%
          </el-tag>
        </div>
        <div class="fusion-content">
          <div class="fusion-summary">{{ fusionResult.summary || '执行结果' }}</div>
          <ul v-if="fusionResult.key_findings?.length"><li v-for="(finding, i) in fusionResult.key_findings" :key="i">{{ finding }}</li></ul>
          <details><summary>查看完整结果</summary><pre class="result-json">{{ JSON.stringify(fusionResult.raw || fusionResult, null, 2) }}</pre></details>
          <div class="fusion-details" v-if="fusionResult.details?.length">
            <div v-for="(d, i) in fusionResult.details" :key="i" class="fusion-detail-item">
              <span class="detail-key">{{ d.key }}:</span>
              <span class="detail-value">{{ d.value }}</span>
            </div>
          </div>
        </div>
      </div>
      <el-alert v-else-if="fusionError" type="error" :closable="false" show-icon title="融合结果加载失败" style="margin-bottom:12px" />

      <!-- AI 助手 -->
      <div class="ai-assistant">
        <div class="section-header">
          <h4 class="section-title"><el-icon><ChatDotRound /></el-icon> AI 助手</h4>
          <el-tag size="small" type="primary" effect="dark">智能</el-tag>
        </div>
        <div class="ai-chat">
          <div class="ai-messages">
            <div v-for="(msg, i) in aiMessages" :key="i" class="ai-msg" :class="msg.role">
              <div class="msg-avatar">{{ msg.role === 'user' ? '我' : 'AI' }}</div>
              <div class="msg-bubble">{{ msg.content }}</div>
            </div>
          </div>
          <div class="ai-input-row">
            <el-input
              v-model="aiInput"
              placeholder="询问任务状态、优化建议..."
              @keyup.enter="sendAiMessage"
            >
              <template #append>
                <el-button @click="sendAiMessage" :loading="aiLoading">发送</el-button>
              </template>
            </el-input>
          </div>
        </div>
      </div>
    </div>

    <!-- 未选中任务 -->
    <div v-else class="no-task-selected">
      <div class="no-task-icon">🎯</div>
      <h3>选择一个任务查看详情</h3>
      <p>从左侧列表选择任务，查看执行流程、日志和结果</p>
    </div>

    <!-- 新建任务弹窗 -->
    <el-dialog v-model="showCreate" title="新建联盟任务" width="min(520px, 94vw)" class="create-dialog">
      <el-form :model="newTask" label-position="top">
        <el-form-item label="任务名称">
          <el-input v-model="newTask.name" placeholder="例如：评估订单系统的性能与安全风险" maxlength="200" show-word-limit />
        </el-form-item>
        <el-form-item label="任务描述">
          <el-input v-model="newTask.description" type="textarea" :rows="3" placeholder="说明背景、要解决的问题和期望交付物。描述越具体，专家匹配越准确。" maxlength="10000" show-word-limit />
        </el-form-item>
        <el-form-item label="融合策略">
          <el-select v-model="newTask.fusion_strategy" style="width: 100%">
            <el-option label="加权融合" value="weighted" />
            <el-option label="投票融合" value="voting" />
            <el-option label="辩论融合" value="debate" />
            <el-option label="择优汇总" value="best_of" />
          </el-select>
        </el-form-item>
        <p class="form-hint">调度服务根据任务描述自动匹配专家。提交后可在执行流程中查看参与专家。</p>
      </el-form>
      <template #footer>
        <el-button @click="showCreate = false">取消</el-button>
        <el-button type="primary" @click="createTask" :loading="creating">创建</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, reactive, onMounted, onBeforeUnmount } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus, VideoPlay, VideoPause, Close, Share, Document, MagicStick, ChatDotRound, User, Calendar, Timer, Clock, Search } from '@element-plus/icons-vue'
import * as api from '@/api'
import { useAllianceTasks, taskActions } from '@/composables/useAllianceTasks'

const taskState = useAllianceTasks(api)
const { tasks, selectedTask, tasksLoading, tasksError, logs, fusionResult, dagNodesData, dagEdgesData,
  logsLoading, fusionLoading, dagLoading, logsError, fusionError, dagError, pollError, actionPending,
  selectTask, loadTasks, performAction, startPolling, dispose } = taskState
const currentFilter = ref('all'), showCreate = ref(false), creating = ref(false), logsContainer = ref(null)
const newTask = reactive({ name: '', description: '', fusion_strategy: 'weighted' })
const statusFilters = [
  { key: 'all', label: '全部' }, { key: 'pending', label: '待处理' },
  { key: 'running', label: '运行中' }, { key: 'paused', label: '已暂停' },
  { key: 'completed', label: '已完成' }, { key: 'failed', label: '失败' }, { key: 'cancelled', label: '已取消' },
]
const filteredTasks = computed(() => currentFilter.value === 'all' ? tasks.value : tasks.value.filter(t => t.status === currentFilter.value))
const dagNodes = computed(() => dagNodesData.value || [])
const dagEdges = computed(() => dagEdgesData.value || [])
const dagWidth = computed(() => Math.max(740, ...dagNodes.value.map(n => n.x + 90)))
const dagHeight = computed(() => Math.max(140, ...dagNodes.value.map(n => n.y + 60)))
const canResume = computed(() => runtimeReady.value && taskActions.resume.has(selectedTask.value?.status))
const canPause = computed(() => runtimeReady.value && taskActions.pause.has(selectedTask.value?.status))
const canCancel = computed(() => runtimeReady.value && taskActions.cancel.has(selectedTask.value?.status))
const getStatusCount = key => key === 'all' ? tasks.value.length : tasks.value.filter(t => t.status === key).length
const statusLabel = status => ({ pending: '待处理', planning: '规划中', ready: '已就绪', running: '运行中', paused: '已暂停', completed: '已完成', failed: '失败', cancelled: '已取消', unknown: '状态待确认' }[status] || status)
const getPriority = task => task.priority || 'normal'
const getPriorityLabel = p => ({ critical: '紧急', high: '高优', normal: '中优', mid: '中优', low: '低优' }[p] || '中优')
const getAssignee = task => task.assignee || '自动匹配专家'
const getDueDate = task => task.due_date || '--'

async function createTask() {
  if (creating.value || !runtimeReady.value) return
  if (!newTask.name.trim() || !newTask.description.trim()) {
    ElMessage.warning('请填写任务名称和具体目标')
    return
  }
  creating.value = true
  try {
    const task = await api.createAllianceTask({ title: newTask.name.trim(), description: newTask.description.trim(), fusion_strategy: newTask.fusion_strategy })
    showCreate.value = false
    Object.assign(newTask, { name: '', description: '', fusion_strategy: 'weighted' })
    currentFilter.value = 'all'
    ElMessage.success('任务已提交')
    await loadTasks(task.id)
  } catch (error) { ElMessage.error(`提交失败：${error.message || '请稍后重试'}`) }
  finally { creating.value = false }
}
async function act(action) {
  try { if (await performAction(action)) ElMessage.success('操作已受理') }
  catch (error) { ElMessage.error(`操作失败：${error.message || '请重试'}`) }
}
const runTask = () => act('resume')
const pauseTask = () => act('pause')
const cancelTask = () => act('cancel')
const clearLogs = () => { logs.value = [] }
const aiMessages = ref([{ role: 'assistant', content: '告诉我你想了解的任务问题，我会结合当前状态提供建议。' }])
const aiInput = ref(''), aiLoading = ref(false)
async function sendAiMessage() {
  if (aiLoading.value || !aiInput.value.trim()) return
  const message = aiInput.value.trim(), task = selectedTask.value
  aiMessages.value.push({ role: 'user', content: message })
  aiInput.value = ''; aiLoading.value = true
  try {
    const context = task ? `任务：${task.name}，状态：${statusLabel(task.status)}，进度：${task.progress}%。日志：${logs.value.slice(-10).map(l => l.message).join('；')}` : '当前未选择任务。'
    const response = await api.aiChat({ message: `请根据以下任务状态和日志分析，缺少信息时明确说明。${context}\n用户问题：${message}` })
    const content = response?.data?.content || response?.content || response?.data?.message || response?.message || (typeof response === 'string' ? response : '')
    if (!content) throw new Error('AI 服务没有返回有效内容')
    aiMessages.value.push({ role: 'assistant', content })
  } catch (error) { aiMessages.value.push({ role: 'assistant', content: `暂时无法分析：${error.message}。可以重试，任务执行不受影响。` }) }
  finally { aiLoading.value = false }
}
const runtimeReady = ref(false), runtimeLoading = ref(false), runtimeMessage = ref('正在检查任务服务…')
async function checkRuntime() {
  runtimeLoading.value = true
  try {
    const status = await api.getAllianceRuntime()
    runtimeReady.value = status.execution_ready === true
    runtimeMessage.value = status.message || '任务执行服务尚未就绪'
  } catch (error) { runtimeReady.value = false; runtimeMessage.value = `无法检查任务服务：${error.message}` }
  finally { runtimeLoading.value = false }
}
async function refreshWorkspace() { await Promise.all([loadTasks(), checkRuntime()]) }
onMounted(async () => { await refreshWorkspace(); startPolling() })
onBeforeUnmount(dispose)
</script>

<style scoped>
.alliance-task {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 16px;
  padding: 16px 20px;
  background: var(--bg-primary);
  overflow: hidden;
}

/* ===== 左栏 ===== */
.task-list-col {
  width: 380px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px;
  overflow: hidden;
}
.list-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-shrink: 0;
}
.list-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 2px;
}
.list-subtitle {
  font-size: 12px;
  color: var(--text-muted);
  margin: 0;
}

/* 筛选 chips */
.filter-row {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  flex-shrink: 0;
}
.filter-chip {
  padding: 3px 10px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 14px;
  font-size: 11px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
  user-select: none;
}
.filter-chip:hover {
  border-color: var(--border-light);
  color: var(--text-primary);
}
.filter-chip.active {
  background: var(--accent-dim);
  border-color: var(--accent);
  color: var(--accent-light);
  font-weight: 600;
}

/* 任务列表 */
.task-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-right: 4px;
}
.task-item {
  display: flex;
  gap: 10px;
  padding: 12px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.2s;
}
.task-item:hover {
  background: var(--bg-hover);
  border-color: var(--border-light);
}
.task-item.active {
  border-color: var(--accent);
  background: var(--accent-dim);
}
.task-item.done .task-title {
  text-decoration: line-through;
  color: var(--text-muted);
}

/* checkbox */
.task-checkbox {
  width: 18px;
  height: 18px;
  border: 2px solid var(--border-light);
  border-radius: 4px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  cursor: pointer;
  transition: all 0.2s;
  margin-top: 2px;
}
.task-checkbox:hover {
  border-color: var(--accent);
}
.task-checkbox.done {
  background: var(--success);
  border-color: var(--success);
}
.check-icon {
  color: #fff;
  font-size: 12px;
  font-weight: 700;
}

.task-info {
  flex: 1;
  min-width: 0;
}
.task-title-row {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 8px;
  margin-bottom: 4px;
}
.task-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.4;
}
.task-priority {
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 500;
  flex-shrink: 0;
}
.priority-high { background: var(--danger-dim); color: var(--danger); }
.priority-mid { background: var(--warning-dim); color: var(--warning); }
.priority-low { background: var(--success-dim); color: var(--success); }

.task-desc {
  font-size: 11px;
  color: var(--text-muted);
  line-height: 1.4;
  margin-bottom: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}
.task-meta {
  display: flex;
  gap: 10px;
  font-size: 10px;
  color: var(--text-muted);
  flex-wrap: wrap;
}
.meta-item {
  display: inline-flex;
  align-items: center;
  gap: 3px;
}
.progress-meta {
  align-items: center;
  gap: 6px;
}
.progress-bar-mini {
  width: 40px;
  height: 4px;
  background: var(--bg-card);
  border-radius: 2px;
  overflow: hidden;
}
.progress-fill-mini {
  height: 100%;
  background: var(--accent);
  border-radius: 2px;
  transition: width 0.3s;
}

.task-status-badge {
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 10px;
  font-weight: 600;
  flex-shrink: 0;
  align-self: flex-start;
}
.task-status-badge.pending { background: var(--bg-tertiary); color: var(--text-secondary); }
.task-status-badge.running { background: var(--accent-dim); color: var(--accent-light); }
.task-status-badge.completed { background: var(--success-dim); color: var(--success); }
.task-status-badge.failed { background: var(--danger-dim); color: var(--danger); }
.task-status-badge.cancelled { background: var(--bg-tertiary); color: var(--text-muted); }

.empty-list {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 13px;
}
.empty-icon {
  font-size: 36px;
}

/* ===== 右栏 ===== */
.task-detail-col {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
  padding-right: 4px;
}
.detail-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px;
  flex-shrink: 0;
}
.detail-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 4px;
}
.detail-desc {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 0;
}
.detail-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

/* 进度 */
.progress-section {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px;
  flex-shrink: 0;
}
.progress-label {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 8px;
}
.progress-value {
  font-weight: 600;
  color: var(--accent-light);
}
.progress-bar-large {
  height: 8px;
  background: var(--bg-tertiary);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 10px;
}
.progress-fill-large {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), var(--accent-light));
  border-radius: 4px;
  transition: width 0.5s;
}
.progress-stats {
  display: flex;
  gap: 16px;
  font-size: 11px;
  color: var(--text-muted);
}
.progress-stats span {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

/* 通用 section */
.dag-section, .logs-section, .fusion-section, .ai-assistant {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px;
}
.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
  display: flex;
  align-items: center;
  gap: 6px;
}
.dag-legend {
  display: flex;
  gap: 12px;
  font-size: 11px;
  color: var(--text-muted);
}
.legend-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
.legend-dot.pending { background: var(--text-muted); }
.legend-dot.running { background: var(--accent); }
.legend-dot.completed { background: var(--success); }
.legend-dot.failed { background: var(--danger); }

/* DAG */
.dag-canvas {
  background: var(--bg-tertiary);
  border-radius: var(--radius-sm);
  padding: 12px;
  overflow-x: auto;
}
.dag-svg {
  width: 100%;
  min-width: 700px;
  height: 140px;
}
.dag-edge {
  stroke: var(--border-light);
  stroke-width: 2;
  fill: none;
}
.dag-edge.completed { stroke: var(--success); }
.dag-edge.running { stroke: var(--accent); stroke-dasharray: 4 4; animation: dash 1s linear infinite; }
.dag-edge.pending { stroke: var(--border); }
@keyframes dash { to { stroke-dashoffset: -8; } }

.dag-node {
  fill: var(--bg-card);
  stroke: var(--border-light);
  stroke-width: 1.5;
}
.dag-node.completed { fill: var(--success-dim); stroke: var(--success); }
.dag-node.running { fill: var(--accent-dim); stroke: var(--accent); }
.dag-node.pending { fill: var(--bg-card); stroke: var(--border); }
.dag-node.failed { fill: var(--danger-dim); stroke: var(--danger); }
.dag-node-label {
  fill: var(--text-primary);
  font-size: 11px;
  font-weight: 600;
}
.dag-node-sub {
  fill: var(--text-muted);
  font-size: 9px;
}

/* 日志 */
.logs-container {
  background: var(--bg-tertiary);
  border-radius: var(--radius-sm);
  padding: 12px;
  max-height: 200px;
  overflow-y: auto;
  font-family: 'Consolas', 'Monaco', monospace;
}
.log-line {
  display: flex;
  gap: 8px;
  font-size: 11px;
  line-height: 1.8;
  padding: 2px 0;
}
.log-time { color: var(--text-muted); flex-shrink: 0; }
.log-level { font-weight: 700; width: 50px; flex-shrink: 0; }
.log-level.info { color: var(--accent-light); }
.log-level.warning { color: var(--warning); }
.log-level.error { color: var(--danger); }
.log-level.success { color: var(--success); }
.log-msg { color: var(--text-secondary); flex: 1; }
.logs-empty {
  text-align: center;
  color: var(--text-muted);
  font-size: 12px;
  padding: 20px;
}

/* 融合结果 */
.fusion-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.fusion-summary {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.7;
  background: var(--bg-tertiary);
  padding: 12px;
  border-radius: var(--radius-sm);
  border-left: 3px solid var(--accent);
}
.fusion-details {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 8px;
}
.fusion-detail-item {
  background: var(--bg-tertiary);
  padding: 8px 12px;
  border-radius: var(--radius-xs);
  font-size: 12px;
}
.detail-key { color: var(--text-muted); margin-right: 6px; }
.detail-value { color: var(--text-primary); font-weight: 600; }

/* AI 助手 */
.ai-chat {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.ai-messages {
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-height: 200px;
  overflow-y: auto;
  padding: 4px;
}
.ai-msg {
  display: flex;
  gap: 8px;
}
.ai-msg.user { flex-direction: row-reverse; }
.msg-avatar {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 11px;
  font-weight: 700;
  flex-shrink: 0;
}
.ai-msg.assistant .msg-avatar { background: var(--accent); color: #fff; }
.ai-msg.user .msg-avatar { background: var(--success); color: #fff; }
.msg-bubble {
  max-width: 80%;
  padding: 8px 12px;
  border-radius: var(--radius-sm);
  font-size: 12px;
  line-height: 1.6;
}
.ai-msg.assistant .msg-bubble {
  background: var(--bg-tertiary);
  color: var(--text-secondary);
}
.ai-msg.user .msg-bubble {
  background: var(--accent-dim);
  color: var(--accent-light);
}
.ai-input-row :deep(.el-input__wrapper) {
  background: var(--bg-tertiary);
  box-shadow: 0 0 0 1px var(--border) inset;
}
.ai-input-row :deep(.el-input__inner) {
  color: var(--text-primary);
}

/* 未选中 */
.no-task-selected {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--text-muted);
}
.no-task-icon {
  font-size: 48px;
}
.no-task-selected h3 {
  font-size: 16px;
  color: var(--text-primary);
  margin: 0;
}
.no-task-selected p {
  font-size: 13px;
  margin: 0;
}

/* 弹窗深色 */
:deep(.el-dialog) {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
}
:deep(.el-dialog__title) { color: var(--text-primary); }
:deep(.el-dialog__body) { color: var(--text-secondary); }
:deep(.el-form-item__label) { color: var(--text-secondary); }
.result-json { white-space: pre-wrap; overflow-wrap: anywhere; max-height: 360px; overflow: auto; font-size: 12px; }
.form-hint { color: var(--text-secondary); font-size: 13px; line-height: 1.6; }
.task-item:focus-visible, .filter-chip:focus-visible { outline: 2px solid var(--el-color-primary); outline-offset: 2px; }
.filter-chip { font: inherit; }
@media (max-width: 760px) {
  .alliance-task { flex-direction: column; overflow: auto; padding: 12px; }
  .task-list-col { width: 100%; flex-shrink: 0; max-height: 42vh; }
  .task-detail-col { min-width: 0; overflow: visible; flex-shrink: 0; }
  .detail-header { flex-wrap: wrap; gap: 12px; }
  .detail-actions { flex-wrap: wrap; }
  .filter-row { flex-wrap: wrap; }
  .progress-stats { flex-wrap: wrap; gap: 8px; }
}
</style>
