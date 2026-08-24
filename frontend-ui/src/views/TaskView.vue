<template>
  <div class="task-view">
    <div class="task-header">
      <div class="task-title">
        <el-icon><List /></el-icon>
        <span>任务管理</span>
        <span class="badge">对话↔任务 双向转换</span>
      </div>
      <div class="task-tools">
        <el-button type="primary" @click="openCreate">
          <el-icon><Plus /></el-icon> 新建任务
        </el-button>
        <el-button @click="loadTasks">
          <el-icon><Refresh /></el-icon> 刷新
        </el-button>
      </div>
    </div>

    <div class="task-filters">
      <el-input
        v-model="searchText"
        placeholder="搜索任务标题/描述…（最近 5 条）"
        clearable
        style="width: 300px"
        @change="pushHistory('task', searchText)"
        @keyup.enter="pushHistory('task', searchText)"
      >
        <template #prefix><el-icon><Search /></el-icon></template>
      </el-input>
      <el-popover v-if="taskSearchHistory.length" placement="bottom" trigger="click" width="300">
        <template #reference>
          <el-button plain :icon="Clock">历史</el-button>
        </template>
        <div class="hist-list">
          <div
            v-for="h in taskSearchHistory"
            :key="h"
            class="hist-item"
            @click="searchText = h; pushHistory('task', h)"
          >
            <el-icon><Clock /></el-icon>{{ h }}
          </div>
          <el-button v-if="taskSearchHistory.length" link size="small" type="danger" @click="taskSearchHistory = clearHistory('task')">清空</el-button>
        </div>
      </el-popover>
      <el-select v-model="filterStatus" placeholder="状态" clearable style="width: 120px">
        <el-option label="待处理" value="todo" />
        <el-option label="进行中" value="in_progress" />
        <el-option label="已完成" value="done" />
        <el-option label="已取消" value="cancelled" />
      </el-select>
      <el-select v-model="filterPriority" placeholder="优先级" clearable style="width: 120px">
        <el-option label="高" value="high" />
        <el-option label="中" value="medium" />
        <el-option label="低" value="low" />
      </el-select>
      <el-select v-model="filterSource" placeholder="来源" clearable style="width: 140px">
        <el-option label="手动创建" value="manual" />
        <el-option label="对话转换" value="chat" />
      </el-select>
      <span class="filter-range">{{ pageFrom }}–{{ pageTo }} / {{ filteredTasks.length }} 条</span>
    </div>

    <div class="stats-row">
      <div class="stat-card" v-for="s in stats" :key="s.key" :style="{ borderLeftColor: s.color }">
        <div class="stat-value" :style="{ color: s.color }">{{ s.value }}</div>
        <div class="stat-label">{{ s.label }}</div>
      </div>
    </div>

    <div class="task-list" v-loading="loading">
      <div v-if="!filteredTasks.length && !loading" class="empty-cta">
        <el-empty description="还没有任务，用下面按钮 30 秒建一个，或 Ctrl+⇧+N 快捷键召唤" :image-size="90">
          <el-button type="primary" size="large" :icon="Plus" @click="openCreate">立即新建第一个任务</el-button>
        </el-empty>
      </div>
      <div
        v-for="task in pagedTasks"
        :key="task.id"
        class="task-card"
        :class="{ 'task-selected': selectedId === task.id }"
        @click="selectTask(task)"
      >
        <div class="task-card-header">
          <div class="task-priority" :class="task.priority">{{ priorityLabels[task.priority] }}</div>
          <el-tag :type="statusTagType(task.status)" size="small" effect="dark">{{ statusLabels[task.status] }}</el-tag>
          <el-tag v-if="task.source === 'chat'" type="warning" size="small">对话转换</el-tag>
          <el-tag v-else type="info" size="small">手动</el-tag>
          <span class="task-time">{{ formatTime(task.updated_at) }}</span>
        </div>
        <div class="task-card-title">{{ task.title }}</div>
        <div class="task-card-desc">{{ task.description || '无描述' }}</div>
        <div class="task-card-meta-row">
          <el-tooltip content="截止日期" placement="top">
            <span class="meta-chip" v-if="task.due_date"><el-icon><Clock /></el-icon>{{ formatDate(task.due_date) }}</span>
          </el-tooltip>
          <el-tooltip content="预估工时" placement="top">
            <span class="meta-chip" v-if="task.estimate_hours != null"><el-icon><Operation /></el-icon>{{ task.estimate_hours }}h</span>
          </el-tooltip>
        </div>
        <div class="task-card-steps" v-if="task.steps && task.steps.length">
          <div v-for="(step, i) in task.steps.slice(0, 3)" :key="i" class="step-item">
            <el-icon><CircleCheck /></el-icon> {{ step }}
          </div>
          <div v-if="task.steps.length > 3" class="step-more">等 {{ task.steps.length }} 项</div>
        </div>
        <div class="task-card-tags" v-if="task.tags && task.tags.length">
          <el-tag v-for="tag in task.tags.slice(0, 4)" :key="tag" size="small" type="info" effect="plain">{{ tag }}</el-tag>
        </div>
      </div>
    </div>

    <div v-if="filteredTasks.length > pageSize" class="pager-row">
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :page-sizes="[12, 24, 48, 96]"
        :total="filteredTasks.length"
        layout="sizes, prev, pager, next, jumper, total"
        background
      />
    </div>

    <el-drawer
      v-model="detailOpen"
      :title="currentTask?.title || '任务详情'"
      size="600px"
      :destroy-on-close="true"
    >
      <template v-if="currentTask">
        <div class="detail-section">
          <div class="detail-label">状态</div>
          <div>
            <el-select v-model="currentTask.status" size="small" @change="onStatusChange">
              <el-option label="待处理" value="todo" />
              <el-option label="进行中" value="in_progress" />
              <el-option label="已完成" value="done" />
              <el-option label="已取消" value="cancelled" />
            </el-select>
          </div>
        </div>
        <div class="detail-section">
          <div class="detail-label">优先级</div>
          <el-select v-model="currentTask.priority" size="small">
            <el-option label="高" value="high" />
            <el-option label="中" value="medium" />
            <el-option label="低" value="low" />
          </el-select>
        </div>
        <div class="detail-section">
          <div class="detail-label">描述</div>
          <el-input v-model="currentTask.description" type="textarea" :rows="3" />
        </div>
        <div class="detail-section" v-if="currentTask.steps && currentTask.steps.length">
          <div class="detail-label">执行步骤</div>
          <div class="detail-steps">
            <div v-for="(step, i) in currentTask.steps" :key="i" class="detail-step">
              <span class="step-num">{{ i + 1 }}</span>
              <span>{{ step }}</span>
            </div>
          </div>
        </div>
        <div class="detail-section" v-if="currentTask.ai_reply">
          <div class="detail-label">AI分析</div>
          <div class="ai-reply">{{ currentTask.ai_reply }}</div>
        </div>
        <div class="detail-section" v-if="currentTask.messages && currentTask.messages.length">
          <div class="detail-label">原始对话</div>
          <div class="chat-history">
            <div v-for="(m, i) in currentTask.messages.slice(-6)" :key="i" class="chat-msg" :class="m.role">
              <span class="chat-role">{{ m.role === 'user' ? '用户' : 'AI' }}</span>
              <span class="chat-content">{{ m.content }}</span>
            </div>
          </div>
        </div>

        <!-- 任务内置对话 -->
        <div class="detail-section task-chat-section">
          <div class="detail-label">💬 任务对话（保留对话模式）</div>
          <div class="task-chat">
            <div class="task-chat-messages" ref="taskChatScroll">
              <div v-for="(m, i) in taskChatMessages" :key="i" class="tc-msg" :class="m.role">
                <span class="tc-role">{{ m.role === 'user' ? '我' : 'AI' }}</span>
                <span class="tc-content">{{ m.content }}</span>
              </div>
              <div v-if="taskChatThinking" class="tc-thinking">
                <span></span><span></span><span></span>
              </div>
            </div>
            <div class="task-chat-input">
              <el-input
                v-model="taskChatDraft"
                size="small"
                placeholder="关于此任务的对话..."
                @keydown.enter.exact.prevent="sendTaskChat"
              />
              <el-button type="primary" size="small" :loading="taskChatThinking" @click="sendTaskChat">
                <el-icon><Promotion /></el-icon>
              </el-button>
            </div>
          </div>
        </div>
        <div class="detail-section">
          <div class="detail-label">标签</div>
          <div class="tags-edit">
            <el-tag
              v-for="(tag, i) in (currentTask.tags || [])"
              :key="i"
              closable
              @close="removeTag(i)"
              style="margin-right: 4px"
            >{{ tag }}</el-tag>
            <el-input
              v-model="newTag"
              size="small"
              placeholder="添加标签"
              style="width: 100px"
              @keyup.enter="addTag"
            />
          </div>
        </div>
        <div class="detail-actions">
          <el-button type="primary" @click="convertToChat">
            <el-icon><ChatDotRound /></el-icon> 转为对话
          </el-button>
          <el-button type="success" @click="executeTask">
            <el-icon><VideoPlay /></el-icon> 执行任务
          </el-button>
          <el-button @click="saveTask">
            <el-icon><Check /></el-icon> 保存
          </el-button>
          <el-button type="danger" @click="deleteTask">
            <el-icon><Delete /></el-icon> 删除
          </el-button>
        </div>
      </template>
    </el-drawer>

    <el-dialog v-model="createOpen" title="新建任务" width="560px" :close-on-click-modal="false">
      <!-- 6 字段完成度（企业最优：实时可感知进展感） -->
      <div class="create-progress">
        <el-progress
          :percentage="progressPercent"
          :stroke-width="10"
          :color="progressPercent === 100 ? '#10b981' : '#6366f1'"
        />
        <div class="progress-meta">
          <span>已完成 <b>{{ progressCount }}</b>/6 字段</span>
          <span class="progress-tip" v-if="progressPercent < 100">还差 {{ 6 - progressCount }} 项即可提交</span>
          <span class="progress-tip done" v-else>🎉 全部就绪，创建吧！</span>
        </div>
      </div>
      <el-form ref="createFormRef" :model="newTaskForm" :rules="createFormRules" label-width="86px">
        <el-form-item label="标题" prop="title">
          <el-input v-model="newTaskForm.title" maxlength="120" show-word-limit placeholder="任务标题（2~120 字）" />
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input v-model="newTaskForm.description" type="textarea" :rows="3" maxlength="1000" show-word-limit placeholder="任务描述（可选）" />
        </el-form-item>
        <div class="form-grid-2">
          <el-form-item label="优先级" prop="priority">
            <el-select v-model="newTaskForm.priority" style="width:100%">
              <el-option label="高（24h 内推进）" value="high" />
              <el-option label="中（本周完成）" value="medium" />
              <el-option label="低（排期待定）" value="low" />
            </el-select>
          </el-form-item>
          <el-form-item label="分类" prop="category">
            <el-select v-model="newTaskForm.category" style="width:100%">
              <el-option label="通用" value="general" />
              <el-option label="需求" value="requirement" />
              <el-option label="开发" value="development" />
              <el-option label="测试" value="testing" />
              <el-option label="运维" value="devops" />
            </el-select>
          </el-form-item>
        </div>
        <div class="form-grid-2">
          <el-form-item label="截止日期" prop="due_date">
            <el-date-picker
              v-model="newTaskForm.due_date"
              type="datetime"
              value-format="YYYY-MM-DDTHH:mm:ss"
              placeholder="默认 3 天后 18:00"
              style="width:100%"
            />
          </el-form-item>
          <el-form-item label="预估工时" prop="estimate_hours">
            <el-input-number
              v-model="newTaskForm.estimate_hours"
              :min="0"
              :max="8760"
              :step="0.5"
              :precision="2"
              style="width:100%"
              placeholder="小时，默认 2h"
            />
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="createOpen = false">取消</el-button>
        <el-button type="primary" :loading="creating" @click="createTask">创建任务</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="convertChatOpen" title="任务转对话" width="500px">
      <div class="convert-info">
        <p>正在将任务「<b>{{ currentTask?.title }}</b>」转换为AI对话...</p>
        <p class="muted">转换后的对话可在 AI 助手页面查看和继续。</p>
      </div>
      <div v-if="convertResult" class="convert-result">
        <div class="result-label">AI 回复预览：</div>
        <div class="result-content">{{ convertResult.reply }}</div>
      </div>
      <div v-else class="convert-loading">
        <el-icon class="spin"><Loading /></el-icon>
        <span>AI 分析中...</span>
      </div>
      <template #footer>
        <el-button @click="convertChatOpen = false">关闭</el-button>
        <el-button type="primary" :disabled="!convertResult" @click="goToChat">前往对话</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  List, Plus, Refresh, Search, CircleCheck, Check, Clock,
  Delete, ChatDotRound, VideoPlay, Loading, Promotion, Operation
} from '@element-plus/icons-vue'
import {
  getTasks, createTask as createTaskApi, updateTask,
  deleteTask as deleteTaskApi, convertTaskToChat, executeTask as executeTaskApi,
  aiChat
} from '@/api'
import { getSearchHistory, pushSearchHistory } from '@/globalShortcuts'

const router = useRouter()
const route = useRoute()
const tasks = ref([])
const loading = ref(false)
const creating = ref(false)
const searchText = ref(route.query?.q || '')
const filterStatus = ref(route.query?.status || '')
const filterPriority = ref(route.query?.priority || '')
const filterSource = ref(route.query?.source || '')
const selectedId = ref(null)
const detailOpen = ref(false)
const createOpen = ref(false)
const convertChatOpen = ref(false)
const convertResult = ref(null)
const newTag = ref('')

// 列表页高级：分页 + 搜索历史 + URL 持久化
const page = ref(parseInt(route.query?.page || '1', 10) || 1)
const pageSize = ref(parseInt(route.query?.size || '12', 10) || 12)
const taskSearchHistory = ref(getSearchHistory('task'))

// 任务内置对话
const taskChatMessages = ref([])
const taskChatDraft = ref('')
const taskChatThinking = ref(false)
const taskChatScroll = ref(null)

const createFormRef = ref(null)

function defaultDueDate() {
  const d = new Date()
  d.setDate(d.getDate() + 3)
  d.setHours(18, 0, 0, 0)
  const pad = (n) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}
function emptyNewTaskForm() {
  return {
    title: '',
    description: '',
    priority: 'medium',
    category: 'general',
    due_date: defaultDueDate(),
    estimate_hours: 2
  }
}
const newTaskForm = ref(emptyNewTaskForm())
// 新建任务表单校验规则（必填 + 长度 + 枚举 + 数值范围）
const createFormRules = {
  title: [
    { required: true, message: '请输入任务标题', trigger: 'blur' },
    { min: 2, max: 120, message: '标题长度应为 2~120 字符', trigger: 'blur' }
  ],
  description: [
    { max: 1000, message: '描述不得超过 1000 字符', trigger: 'blur' }
  ],
  priority: [
    { required: true, message: '请选择优先级', trigger: 'change' },
    { validator: (_r, value, cb) => (['high', 'medium', 'low'].includes(value) ? cb() : cb(new Error('非法优先级枚举'))), trigger: 'change' }
  ],
  category: [
    { required: true, message: '请选择分类', trigger: 'change' },
    { validator: (_r, value, cb) => (['general', 'requirement', 'development', 'testing', 'devops'].includes(value) ? cb() : cb(new Error('非法分类枚举'))), trigger: 'change' }
  ],
  estimate_hours: [
    { validator: (_r, value, cb) => (value === null || value === undefined || (typeof value === 'number' && value >= 0 && value <= 8760) ? cb() : cb(new Error('工时范围 0~8760 小时'))), trigger: 'change' }
  ]
}

// 6 字段完成度：title / description(可空，填了算) / priority / category / due_date / estimate_hours
const progressCount = computed(() => {
  const f = newTaskForm.value || {}
  let n = 0
  if (f.title && String(f.title).trim().length >= 2) n++
  if (f.description && String(f.description).trim().length >= 1) n++
  else n++ // 描述可空 → 视作"已就绪"
  if (['high', 'medium', 'low'].includes(f.priority)) n++
  if (['general', 'requirement', 'development', 'testing', 'devops'].includes(f.category)) n++
  if (f.due_date && String(f.due_date).length >= 10) n++
  if (f.estimate_hours !== '' && f.estimate_hours != null) n++
  return Math.min(n, 6)
})
const progressPercent = computed(() => Math.round((progressCount.value / 6) * 100))

const currentTask = computed(() => tasks.value.find(t => t.id === selectedId.value))

const statusLabels = { todo: '待处理', in_progress: '进行中', done: '已完成', cancelled: '已取消' }
const priorityLabels = { high: '🔥高', medium: '⚡中', low: '💤低' }

function statusTagType(s) {
  return { todo: 'info', in_progress: 'warning', done: 'success', cancelled: 'danger' }[s] || 'info'
}

const filteredTasks = computed(() => {
  return tasks.value.filter(t => {
    if (searchText.value) {
      const q = String(searchText.value).toLowerCase()
      if (!t.title.toLowerCase().includes(q) && !(t.description || '').toLowerCase().includes(q)) return false
    }
    if (filterStatus.value && t.status !== filterStatus.value) return false
    if (filterPriority.value && t.priority !== filterPriority.value) return false
    if (filterSource.value && t.source !== filterSource.value) return false
    return true
  })
})
const pagedTasks = computed(() => {
  const s = (page.value - 1) * pageSize.value
  return filteredTasks.value.slice(s, s + pageSize.value)
})
const pageFrom = computed(() => (filteredTasks.value.length ? (page.value - 1) * pageSize.value + 1 : 0))
const pageTo = computed(() => Math.min(page.value * pageSize.value, filteredTasks.value.length))

// URL 筛选参数同步（可复制链接给同事，打开直接看同一筛选视图）
function syncQueryToRoute() {
  const q = {}
  if (searchText.value) q.q = String(searchText.value).trim().slice(0, 120)
  if (filterStatus.value) q.status = filterStatus.value
  if (filterPriority.value) q.priority = filterPriority.value
  if (filterSource.value) q.source = filterSource.value
  if (page.value !== 1) q.page = String(page.value)
  if (pageSize.value !== 12) q.size = String(pageSize.value)
  router.replace({ path: route.path, query: q })
}

const stats = computed(() => {
  const total = tasks.value.length
  const todo = tasks.value.filter(t => t.status === 'todo').length
  const inProg = tasks.value.filter(t => t.status === 'in_progress').length
  const done = tasks.value.filter(t => t.status === 'done').length
  return [
    { key: 'total', label: '总任务', value: total, color: '#6366f1' },
    { key: 'todo', label: '待处理', value: todo, color: '#f59e0b' },
    { key: 'in_progress', label: '进行中', value: inProg, color: '#06b6d4' },
    { key: 'done', label: '已完成', value: done, color: '#10b981' }
  ]
})

function pushHistory(key, term) {
  taskSearchHistory.value = pushSearchHistory(key, term)
  syncQueryToRoute()
}
function clearHistory(key) {
  pushSearchHistory(key, '__CLEAR__', 0)
  return []
}

async function loadTasks() {
  loading.value = true
  try {
    const r = await getTasks()
    tasks.value = Array.isArray(r) ? r : (r.tasks || [])
  } catch (e) {
    ElMessage.error('加载任务失败: ' + e.message)
  } finally {
    loading.value = false
  }
}

function selectTask(task) {
  selectedId.value = task.id
  detailOpen.value = true
  // 初始化任务内置对话
  taskChatMessages.value = []
  if (task.ai_reply) {
    taskChatMessages.value.push({ role: 'assistant', content: task.ai_reply, timestamp: Date.now() })
  }
  if (task.result) {
    taskChatMessages.value.push({ role: 'assistant', content: `📊 执行结果: ${task.result}`, timestamp: Date.now() })
  }
}

async function sendTaskChat() {
  const text = taskChatDraft.value.trim()
  if (!text || taskChatThinking.value || !currentTask.value) return
  taskChatMessages.value.push({ role: 'user', content: text, timestamp: Date.now() })
  taskChatDraft.value = ''
  taskChatThinking.value = true
  await nextTick()
  taskChatScroll.value?.scrollTo({ top: 1e9 })

  try {
    const context = taskChatMessages.value.map(m => ({ role: m.role, content: m.content }))
    const systemPrompt = `你正在协助执行任务「${currentTask.value.title}」。任务描述：${currentTask.value.description}。请根据任务上下文回答用户的问题。`
    const res = await aiChat({
      session_id: `task_${currentTask.value.id}`,
      message: text,
      system_prompt: systemPrompt,
      additional_messages: context.slice(-10)
    })
    const reply = (res.reply || res.response || res.message || '（无回复）').toString()
    taskChatMessages.value.push({ role: 'assistant', content: reply, timestamp: Date.now() })
  } catch (e) {
    taskChatMessages.value.push({ role: 'assistant', content: '⚠️ ' + (e.message || '请求失败'), timestamp: Date.now() })
  } finally {
    taskChatThinking.value = false
    await nextTick()
    taskChatScroll.value?.scrollTo({ top: 1e9 })
  }
}

function openCreate() {
  newTaskForm.value = emptyNewTaskForm()
  nextTick(() => { createFormRef.value?.clearValidate?.() })
  createOpen.value = true
}

// 顶栏「⚡ 新建」/ 快捷键 / ?action=create query 通用入口（全局事件）
function onGlobalOpenTaskCreate() {
  openCreate()
}

async function createTask() {
  try {
    await createFormRef.value?.validate()
  } catch (_e) {
    ElMessage.warning('请检查必填项与格式校验（顶部有完成度提示）')
    return
  }
  creating.value = true
  try {
    // 表单 -> API DTO 映射：统一字段名、类型转换与默认值
    const payload = {
      title: String(newTaskForm.value.title || '').trim(),
      description: String(newTaskForm.value.description || '').trim(),
      priority: newTaskForm.value.priority || 'medium',
      category: newTaskForm.value.category || 'general',
      status: 'todo',
      due_date: newTaskForm.value.due_date || null,
      estimate_hours: newTaskForm.value.estimate_hours === '' || newTaskForm.value.estimate_hours == null
        ? null
        : Number(newTaskForm.value.estimate_hours),
      tags: [],
      created_at: new Date().toISOString()
    }
    await createTaskApi(payload)
    createOpen.value = false
    ElMessage.success('任务已创建')
    await loadTasks()
  } catch (e) {
    ElMessage.error('创建失败: ' + e.message)
  } finally {
    creating.value = false
  }
}

async function onStatusChange() {
  if (!currentTask.value) return
  try {
    await updateTask(currentTask.value.id, { status: currentTask.value.status })
  } catch { ElMessage.error('状态更新失败') }
}

async function saveTask() {
  if (!currentTask.value) return
  try {
    await updateTask(currentTask.value.id, currentTask.value)
    ElMessage.success('任务已保存')
  } catch (e) {
    ElMessage.error('保存失败: ' + e.message)
  }
}

async function deleteTask() {
  if (!currentTask.value) return
  try {
    await ElMessageBox.confirm('确定删除此任务吗？', '确认', { type: 'warning' })
    await deleteTaskApi(currentTask.value.id)
    detailOpen.value = false
    ElMessage.success('任务已删除')
    await loadTasks()
  } catch (e) {
    if (e !== 'cancel') ElMessage.error('删除失败')
  }
}

async function executeTask() {
  if (!currentTask.value) return
  try {
    await executeTaskApi(currentTask.value.id, { status: 'in_progress' })
    ElMessage.success('任务开始执行')
    await loadTasks()
  } catch (e) {
    ElMessage.error('执行失败: ' + e.message)
  }
}

async function convertToChat() {
  if (!currentTask.value) return
  convertResult.value = null
  convertChatOpen.value = true
  try {
    const r = await convertTaskToChat(currentTask.value.id)
    convertResult.value = r
  } catch (e) {
    ElMessage.error('转换失败: ' + e.message)
    convertChatOpen.value = false
  }
}

function goToChat() {
  convertChatOpen.value = false
  router.push('/ai')
}

function addTag() {
  if (!newTag.value.trim() || !currentTask.value) return
  if (!currentTask.value.tags) currentTask.value.tags = []
  currentTask.value.tags.push(newTag.value.trim())
  newTag.value = ''
}

function removeTag(idx) {
  if (!currentTask.value?.tags) return
  currentTask.value.tags.splice(idx, 1)
}

function formatTime(iso) {
  if (!iso) return ''
  const d = new Date(iso)
  return d.toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
}

onMounted(async () => {
  window.addEventListener('xuanji:open-create-task', onGlobalOpenTaskCreate)
  await loadTasks()
  // 支持从AI对话跳转并自动打开任务
  if (route.query.task) {
    const task = tasks.value.find(t => t.id === route.query.task)
    if (task) selectTask(task)
  }
  // ?action=create（顶栏快捷/外部链接过来）直接开 Dialog
  if (route.query.action === 'create') openCreate()
})

// 搜索/筛选/分页任意变化 → URL 同步（不重复刷新）
watch([searchText, filterStatus, filterPriority, filterSource, page, pageSize], () => {
  if (page.value < 1) page.value = 1
  // 若当前页超过总页数，回到第 1 页
  if (pageSize.value > 0 && (page.value - 1) * pageSize.value >= filteredTasks.value.length) page.value = 1
  syncQueryToRoute()
}, { flush: 'post' })

// 监听路由变化，自动打开任务
watch(() => route.query.task, async (taskId) => {
  if (!taskId) return
  if (!tasks.value.length) await loadTasks()
  const task = tasks.value.find(t => t.id === taskId)
  if (task) selectTask(task)
  else {
    await loadTasks()
    const t = tasks.value.find(tt => tt.id === taskId)
    if (t) selectTask(t)
  }
})
watch(() => route.query.action, (a) => { if (a === 'create') openCreate() })

onBeforeUnmount(() => {
  window.removeEventListener('xuanji:open-create-task', onGlobalOpenTaskCreate)
})

function formatDate(d) {
  if (!d) return ''
  const dt = new Date(d)
  if (isNaN(dt)) return String(d)
  return `${dt.getFullYear()}-${String(dt.getMonth()+1).padStart(2,'0')}-${String(dt.getDate()).padStart(2,'0')} ${String(dt.getHours()).padStart(2,'0')}:${String(dt.getMinutes()).padStart(2,'0')}`
}
</script>

<style scoped>
.task-view {
  padding: 0;
}
.task-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.task-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 18px;
  font-weight: 700;
}
.task-title .badge {
  font-size: 12px;
  font-weight: 500;
  padding: 2px 10px;
  background: linear-gradient(135deg, #6366f1, #06b6d4);
  color: #fff;
  border-radius: 12px;
}
.task-tools {
  display: flex;
  gap: 8px;
}
.task-filters {
  display: flex;
  gap: 10px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}
.stats-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
  margin-bottom: 16px;
}
.stat-card {
  background: #fff;
  border-radius: 12px;
  padding: 16px 20px;
  border-left: 4px solid;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
}
.stat-value {
  font-size: 28px;
  font-weight: 800;
  line-height: 1.2;
}
.stat-label {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 4px;
}
.task-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 14px;
}
.task-card {
  background: #fff;
  border-radius: 12px;
  padding: 16px;
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid var(--border);
  position: relative;
}
.task-card:hover {
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
  transform: translateY(-1px);
}
.task-card.task-selected {
  border-color: var(--brand);
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
}
.task-card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  flex-wrap: wrap;
}
.task-priority {
  font-size: 12px;
  font-weight: 600;
}
.task-priority.high { color: #ef4444; }
.task-priority.medium { color: #f59e0b; }
.task-priority.low { color: #64748b; }
.task-time {
  margin-left: auto;
  font-size: 11px;
  color: var(--text-3);
}
.task-card-title {
  font-size: 15px;
  font-weight: 700;
  margin-bottom: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.task-card-desc {
  font-size: 13px;
  color: var(--text-2);
  margin-bottom: 8px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.task-card-steps {
  margin-bottom: 8px;
}
.step-item {
  font-size: 12px;
  color: var(--text-2);
  display: flex;
  align-items: center;
  gap: 4px;
  margin-bottom: 3px;
}
.step-item .el-icon { color: #10b981; font-size: 12px; }
.step-more {
  font-size: 11px;
  color: var(--text-3);
  margin-top: 2px;
}
.task-card-tags {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}
.task-card-meta-row {
  display: flex; gap: 8px; flex-wrap: wrap;
  margin-bottom: 8px;
}
.meta-chip {
  display: inline-flex; align-items: center; gap: 4px;
  background: var(--bg-page);
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 12px;
  color: var(--text-2);
  border: 1px solid var(--border);
}
.meta-chip .el-icon { font-size: 12px; color: var(--brand-dark); }

.empty-cta {
  grid-column: 1 / -1;
  background: #fff;
  border-radius: 16px;
  padding: 36px 0;
  border: 2px dashed var(--border);
}

.pager-row {
  margin-top: 18px;
  display: flex; justify-content: flex-end;
}
.filter-range {
  margin-left: auto;
  font-size: 12px;
  color: var(--text-3);
  align-self: center;
  padding-right: 6px;
}
.hist-list { display: flex; flex-direction: column; gap: 4px; max-height: 260px; overflow: auto; }
.hist-item {
  display: flex; align-items: center; gap: 8px;
  padding: 6px 10px; border-radius: 8px; cursor: pointer;
  font-size: 13px; color: var(--text-1);
}
.hist-item:hover { background: var(--brand-soft); color: var(--brand-dark); }
.hist-item .el-icon { font-size: 13px; color: var(--text-3); }

/* 新建任务进度条 */
.create-progress {
  margin-bottom: 10px;
  padding: 12px 14px;
  border-radius: 12px;
  background: linear-gradient(135deg, rgba(99,102,241,0.05), rgba(6,182,212,0.05));
  border: 1px solid rgba(99,102,241,0.1);
}
.progress-meta {
  display: flex; align-items: center; justify-content: space-between;
  margin-top: 8px;
  font-size: 12px; color: var(--text-2);
}
.progress-meta b { color: var(--brand-dark); }
.progress-tip.done { color: #059669; font-weight: 600; }

.form-grid-2 {
  display: grid; grid-template-columns: 1fr 1fr; gap: 0 14px;
}
@media (max-width: 720px) {
  .form-grid-2 { grid-template-columns: 1fr; }
}

.detail-section {
  margin-bottom: 20px;
}
.detail-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-3);
  margin-bottom: 8px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.detail-steps {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.detail-step {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: #f8fafc;
  border-radius: 8px;
  font-size: 13px;
}
.step-num {
  width: 24px;
  height: 24px;
  background: var(--brand);
  color: #fff;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}
.ai-reply {
  background: linear-gradient(135deg, #f0f4ff, #f0fdfa);
  border: 1px solid #e0e7ff;
  border-radius: 10px;
  padding: 12px;
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
}
.chat-history {
  max-height: 200px;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px;
}
.chat-msg {
  padding: 6px 8px;
  margin-bottom: 4px;
  border-radius: 6px;
  font-size: 12px;
}
.chat-msg.user { background: #eff6ff; }
.chat-msg.assistant { background: #f0fdf4; }
.chat-role {
  font-weight: 600;
  margin-right: 6px;
  font-size: 11px;
}
.tags-edit {
  display: flex;
  align-items: center;
  gap: 8px;
}
.detail-actions {
  display: flex;
  gap: 8px;
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}

.convert-info p {
  margin: 8px 0;
}
.convert-info .muted {
  font-size: 13px;
  color: var(--text-3);
}
.convert-result {
  margin-top: 16px;
  padding: 12px;
  background: #f0fdf4;
  border: 1px solid #bbf7d0;
  border-radius: 8px;
}
.result-label {
  font-size: 12px;
  font-weight: 600;
  color: #10b981;
  margin-bottom: 8px;
}
.result-content {
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
}
.convert-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 24px;
  color: var(--text-3);
}
.convert-loading .spin {
  font-size: 32px;
  animation: spin 1s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

/* 任务内置对话样式 */
.task-chat-section {
  border-top: 1px solid var(--border);
  padding-top: 16px;
  margin-top: 16px;
}
.task-chat {
  background: #f8fafc;
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
}
.task-chat-messages {
  max-height: 240px;
  overflow-y: auto;
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.tc-msg {
  display: flex;
  gap: 8px;
  font-size: 13px;
  line-height: 1.5;
}
.tc-msg.user {
  flex-direction: row-reverse;
}
.tc-role {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-3);
  flex-shrink: 0;
}
.tc-msg.user .tc-role {
  color: var(--brand);
}
.tc-content {
  background: #fff;
  padding: 6px 10px;
  border-radius: 8px;
  max-width: 85%;
  word-break: break-word;
}
.tc-msg.user .tc-content {
  background: linear-gradient(135deg, #eef2ff, #e0e7ff);
}
.tc-thinking {
  display: flex;
  gap: 3px;
  padding: 4px 8px;
}
.tc-thinking span {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-3);
  animation: blink 1.2s infinite;
}
.tc-thinking span:nth-child(2) { animation-delay: 0.2s; }
.tc-thinking span:nth-child(3) { animation-delay: 0.4s; }
@keyframes blink { 0%, 60%, 100% { opacity: 0.3; } 30% { opacity: 1; } }
.task-chat-input {
  display: flex;
  gap: 6px;
  padding: 8px 10px;
  border-top: 1px solid var(--border);
  background: #fff;
}
.task-chat-input .el-input {
  flex: 1;
}
</style>
