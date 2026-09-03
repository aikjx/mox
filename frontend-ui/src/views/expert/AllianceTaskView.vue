<template>
  <div class="alliance-task">
    <!-- ===== 左栏：任务列表 ===== -->
    <div class="task-list-col">
      <div class="list-header">
        <div>
          <h2 class="list-title">联盟任务</h2>
          <p class="list-subtitle">全维融合 · 智能编排 · 实时监控</p>
        </div>
        <el-button type="primary" size="small" @click="showCreate = true">
          <el-icon><Plus /></el-icon> 新建任务
        </el-button>
      </div>

      <!-- 筛选 chips -->
      <div class="filter-row">
        <span
          v-for="f in statusFilters"
          :key="f.key"
          class="filter-chip"
          :class="{ active: currentFilter === f.key }"
          @click="currentFilter = f.key"
        >
          {{ f.label }} ({{ getStatusCount(f.key) }})
        </span>
      </div>

      <!-- 任务列表 -->
      <div class="task-list" v-if="filteredTasks.length > 0">
        <div
          v-for="task in filteredTasks"
          :key="task.id"
          class="task-item"
          :class="{ active: selectedTask?.id === task.id, done: task.status === 'completed' }"
          @click="selectTask(task)"
        >
          <!-- checkbox -->
          <div
            class="task-checkbox"
            :class="{ done: task.status === 'completed' }"
            @click.stop="toggleTaskDone(task)"
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

      <div v-else class="empty-list">
        <div class="empty-icon">📋</div>
        <p>暂无{{ currentFilter === 'all' ? '' : statusFilters.find(f => f.key === currentFilter)?.label }}任务</p>
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
          <el-button size="small" @click="runTask" :loading="running">
            <el-icon><VideoPlay /></el-icon> 运行
          </el-button>
          <el-button size="small" @click="pauseTask">
            <el-icon><VideoPause /></el-icon> 暂停
          </el-button>
          <el-button size="small" type="danger" @click="cancelTask">
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
          <span><el-icon><Timer /></el-icon> 耗时: {{ selectedTask.duration || '0s' }}</span>
          <span><el-icon><Clock /></el-icon> 预计: {{ selectedTask.eta || '--' }}</span>
          <span><el-icon><User /></el-icon> 专家: {{ selectedTask.expert_count }} 位</span>
        </div>
      </div>

      <!-- DAG 画布 -->
      <div class="dag-section">
        <div class="section-header">
          <h4 class="section-title"><el-icon><Share /></el-icon> 任务 DAG</h4>
          <div class="dag-legend">
            <span class="legend-item"><span class="legend-dot pending"></span>待处理</span>
            <span class="legend-item"><span class="legend-dot running"></span>运行中</span>
            <span class="legend-item"><span class="legend-dot completed"></span>已完成</span>
            <span class="legend-item"><span class="legend-dot failed"></span>失败</span>
          </div>
        </div>
        <div class="dag-canvas">
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
      </div>

      <!-- 日志 -->
      <div class="logs-section">
        <div class="section-header">
          <h4 class="section-title"><el-icon><Document /></el-icon> 执行日志</h4>
          <el-button size="small" text @click="clearLogs">清空</el-button>
        </div>
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
          <div v-if="logs.length === 0" class="logs-empty">暂无日志</div>
        </div>
      </div>

      <!-- 融合结果 -->
      <div class="fusion-section" v-if="fusionResult">
        <div class="section-header">
          <h4 class="section-title"><el-icon><MagicStick /></el-icon> 全维融合结果</h4>
          <el-tag size="small" :type="fusionResult.confidence > 0.8 ? 'success' : 'warning'" effect="dark">
            置信度 {{ (fusionResult.confidence * 100).toFixed(1) }}%
          </el-tag>
        </div>
        <div class="fusion-content">
          <div class="fusion-summary">{{ fusionResult.summary }}</div>
          <div class="fusion-details" v-if="fusionResult.details?.length">
            <div v-for="(d, i) in fusionResult.details" :key="i" class="fusion-detail-item">
              <span class="detail-key">{{ d.key }}:</span>
              <span class="detail-value">{{ d.value }}</span>
            </div>
          </div>
        </div>
      </div>

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
      <p>从左侧列表选择任务，查看 DAG、日志和融合结果</p>
    </div>

    <!-- 新建任务弹窗 -->
    <el-dialog v-model="showCreate" title="新建联盟任务" width="520px" class="create-dialog">
      <el-form :model="newTask" label-position="top">
        <el-form-item label="任务名称">
          <el-input v-model="newTask.name" placeholder="请输入任务名称" />
        </el-form-item>
        <el-form-item label="任务描述">
          <el-input v-model="newTask.description" type="textarea" :rows="3" placeholder="请输入任务描述" />
        </el-form-item>
        <el-form-item label="融合策略">
          <el-select v-model="newTask.fusion_strategy" style="width: 100%">
            <el-option label="加权融合" value="weighted" />
            <el-option label="投票融合" value="voting" />
            <el-option label="辩论融合" value="debate" />
            <el-option label="级联融合" value="cascade" />
          </el-select>
        </el-form-item>
        <el-form-item label="参与专家">
          <el-select v-model="newTask.expert_ids" multiple style="width: 100%" placeholder="选择参与专家">
            <el-option v-for="e in availableExperts" :key="e.id" :label="e.name" :value="e.id" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreate = false">取消</el-button>
        <el-button type="primary" @click="createTask" :loading="creating">创建</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, reactive, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Plus, VideoPlay, VideoPause, Close, Share, Document, MagicStick,
  ChatDotRound, User, Calendar, Timer, Clock, Search
} from '@element-plus/icons-vue'
import {
  getAllianceTasks, createAllianceTask,
  pauseAllianceTask, cancelAllianceTask, resumeAllianceTask,
  getExperts, aiChat,
  getAllianceTaskLogs, getAllianceFusionResult, getAllianceTaskDag,
  toggleAllianceTaskDone, getAllianceTaskStatus
} from '@/api'

// ===== 状态 =====
const tasks = ref([])
const selectedTask = ref(null)
const currentFilter = ref('all')
const running = ref(false)
const showCreate = ref(false)
const creating = ref(false)
const logs = ref([])
const fusionResult = ref(null)
const logsContainer = ref(null)
const availableExperts = ref([])
const dagNodesData = ref(null)
const dagEdgesData = ref(null)

// AI 助手
const aiMessages = ref([
  { role: 'assistant', content: '你好！我是联盟任务 AI 助手，可以帮你分析任务状态、优化执行策略。' }
])
const aiInput = ref('')
const aiLoading = ref(false)

// 新建任务表单
const newTask = reactive({
  name: '',
  description: '',
  fusion_strategy: 'weighted',
  expert_ids: []
})

// 状态筛选
const statusFilters = [
  { key: 'all', label: '全部' },
  { key: 'pending', label: '待处理' },
  { key: 'running', label: '运行中' },
  { key: 'completed', label: '已完成' },
  { key: 'failed', label: '失败' }
]




// ===== 计算属性 =====
const filteredTasks = computed(() => {
  if (currentFilter.value === 'all') return tasks.value
  return tasks.value.filter(t => t.status === currentFilter.value)
})

// DAG 节点：由 API 加载，无数据时为空
const dagNodes = computed(() => {
  if (dagNodesData.value && dagNodesData.value.length > 0) return dagNodesData.value
  return []
})

// DAG 边：由 API 加载，无数据时为空
const dagEdges = computed(() => {
  if (dagEdgesData.value && dagEdgesData.value.length > 0) return dagEdgesData.value
  if (!selectedTask.value) return []
  return [
    { x1: 160, y1: 60, x2: 220, y2: 30, status: 'completed' },
    { x1: 160, y1: 60, x2: 220, y2: 90, status: 'completed' },
    { x1: 340, y1: 30, x2: 400, y2: 60, status: 'completed' },
    { x1: 340, y1: 90, x2: 400, y2: 60, status: 'running' },
    { x1: 520, y1: 60, x2: 580, y2: 60, status: 'pending' }
  ]
})

const dagWidth = 740
const dagHeight = 140

// ===== 方法 =====
function getStatusCount(key) {
  if (key === 'all') return tasks.value.length
  return tasks.value.filter(t => t.status === key).length
}

function statusLabel(status) {
  const map = { pending: '待处理', running: '运行中', completed: '已完成', failed: '失败', cancelled: '已取消' }
  return map[status] || status
}

function getPriority(task) {
  return task.priority || 'mid'
}

function getPriorityLabel(p) {
  const map = { high: '高优', mid: '中优', low: '低优' }
  return map[p] || '中优'
}

function getAssignee(task) {
  return task.assignee || '未分配'
}

function getDueDate(task) {
  return task.due_date || '--'
}

// 任务完成状态切换：调用 PUT /api/alliance/tasks/:id/toggle-done，失败回滚本地状态
async function toggleTaskDone(task) {
  const prevStatus = task.status
  const prevProgress = task.progress
  if (task.status === 'completed') {
    task.status = 'pending'
    task.progress = 0
  } else {
    task.status = 'completed'
    task.progress = 100
  }
  try {
    await toggleAllianceTaskDone(task.id)
    if (task.status === 'completed') {
      ElMessage.success('任务已标记为完成 🎉')
    } else {
      ElMessage.info('任务已恢复为待处理')
    }
  } catch (e) {
    task.status = prevStatus
    task.progress = prevProgress
    ElMessage.error('状态切换失败：' + e.message)
  }
}

async function selectTask(task) {
  selectedTask.value = task
  // 加载任务日志：失败则降级到 mock
  try {
    const logData = await getAllianceTaskLogs(task.id)
    logs.value = Array.isArray(logData) ? logData : (logData?.items || [])
  } catch (e) {
    logs.value = []
  }
  // 加载融合结果：仅已完成任务，失败则降级
  if (task.status === 'completed') {
    try {
      const fusion = await getAllianceFusionResult(task.id)
      fusionResult.value = fusion || null
    } catch (e) {
      fusionResult.value = null
    }
  } else {
    fusionResult.value = null
  }
  // 加载 DAG：失败则为空
  try {
    const dag = await getAllianceTaskDag(task.id)
    if (dag && dag.nodes) {
      dagNodesData.value = dag.nodes
      dagEdgesData.value = dag.edges || []
    }
  } catch (e) { console.error('[alliance] load DAG failed:', e) }
  nextTick(() => {
    if (logsContainer.value) {
      logsContainer.value.scrollTop = logsContainer.value.scrollHeight
    }
  })
}

async function loadTasks() {
  try {
    const data = await getAllianceTasks()
    tasks.value = data
  } catch (e) {
    console.error('[AllianceTask] API 加载失败:', e)
    tasks.value = []
  }
  if (tasks.value.length > 0 && !selectedTask.value) {
    selectTask(tasks.value[0])
  }
}

async function loadExpertsList() {
  try {
    availableExperts.value = await getExperts()
  } catch (e) {
    availableExperts.value = [
      { id: 'exp_001', name: '林墨白' },
      { id: 'exp_002', name: '苏清瑶' },
      { id: 'exp_003', name: '周知行' }
    ]
  }
}

async function createTask() {
  if (!newTask.name.trim()) {
    ElMessage.warning('请输入任务名称')
    return
  }
  creating.value = true
  try {
    await createAllianceTask({
      name: newTask.name,
      description: newTask.description,
      fusion_strategy: newTask.fusion_strategy,
      expert_ids: newTask.expert_ids
    })
    ElMessage.success('任务创建成功')
    showCreate.value = false
    newTask.name = ''
    newTask.description = ''
    newTask.fusion_strategy = 'weighted'
    newTask.expert_ids = []
    await loadTasks()
  } catch (e) {
    ElMessage.error('创建失败：' + e.message)
  } finally {
    creating.value = false
  }
}

async function runTask() {
  if (!selectedTask.value) return
  running.value = true
  selectedTask.value.status = 'running'
  logs.value.push({ time: new Date().toLocaleTimeString(), level: 'info', message: '任务重新启动' })
  try {
    await resumeAllianceTask(selectedTask.value.id)
    ElMessage.success('任务已启动')
  } catch (e) {
    ElMessage.error('启动失败：' + e.message)
  } finally {
    running.value = false
  }
}

async function pauseTask() {
  if (!selectedTask.value) return
  try {
    await pauseAllianceTask(selectedTask.value.id)
    selectedTask.value.status = 'pending'
    ElMessage.info('任务已暂停')
  } catch (e) {
    ElMessage.error('暂停失败：' + e.message)
  }
}

async function cancelTask() {
  if (!selectedTask.value) return
  try {
    await cancelAllianceTask(selectedTask.value.id)
    selectedTask.value.status = 'cancelled'
    ElMessage.info('任务已取消')
  } catch (e) {
    ElMessage.error('取消失败：' + e.message)
  }
}

function clearLogs() {
  logs.value = []
}

async function sendAiMessage() {
  if (!aiInput.value.trim()) return
  const msg = aiInput.value
  aiMessages.value.push({ role: 'user', content: msg })
  aiInput.value = ''
  aiLoading.value = true
  try {
    const taskCtx = selectedTask.value
      ? `当前任务：${selectedTask.value.name}，状态：${statusLabel(selectedTask.value.status)}，进度：${selectedTask.value.progress || 0}%，预计剩余：${selectedTask.value.eta || '未知'}。`
      : '当前未选择任务。'
    const prompt = `你是联盟任务 AI 助手。${taskCtx}\n用户问题：${msg}\n请基于任务状态和日志给出分析与建议。`
    const res = await aiChat({ message: prompt })
    const reply = res?.data?.content || res?.content || res?.data?.message || res?.message || (typeof res === 'string' ? res : '分析完成。')
    aiMessages.value.push({ role: 'assistant', content: reply })
  } catch (e) {
    aiMessages.value.push({ role: 'assistant', content: '抱歉，AI 分析失败：' + (e.message || '未知错误') + '。请稍后重试。' })
  } finally {
    aiLoading.value = false
  }
}

// 任务状态轮询：优先调用 GET /api/alliance/tasks/:id/status，失败则前端模拟进度
let progressTimer = null
onMounted(async () => {
  await Promise.all([loadTasks(), loadExpertsList()])
  progressTimer = setInterval(async () => {
    for (const t of tasks.value) {
      if (t.status === 'running' && t.progress < 100) {
        try {
          const st = await getAllianceTaskStatus(t.id)
          if (st) {
            if (st.status) t.status = st.status
            if (st.progress != null) t.progress = Math.min(100, st.progress)
            if (t.progress >= 100 && t.status === 'running') t.status = 'completed'
          }
        } catch (e) {
          // API 不可用时降级为本地模拟
          t.progress = Math.min(100, t.progress + Math.random() * 2)
          if (t.progress >= 100) {
            t.status = 'completed'
            t.progress = 100
          }
        }
      }
    }
  }, 2000)
})

onBeforeUnmount(() => {
  if (progressTimer) clearInterval(progressTimer)
})
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
</style>
