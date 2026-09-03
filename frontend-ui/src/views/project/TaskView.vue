<template>
  <div class="task-view">
    <!-- ===== 页头 ===== -->
    <div class="page-header">
      <div>
        <h1 class="page-title">任务管理</h1>
        <p class="page-subtitle">管理所有项目的任务和待办事项</p>
      </div>
      <div class="header-actions">
        <el-input
          v-model="searchKeyword"
          placeholder="搜索任务..."
          clearable
          style="width: 220px"
          @input="handleSearch"
        >
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <el-button type="primary" @click="openCreate">
          <el-icon><Plus /></el-icon> 新建任务
        </el-button>
      </div>
    </div>

    <!-- ===== 筛选 chips ===== -->
    <div class="filter-row">
      <span
        v-for="f in statusFilters"
        :key="f.key"
        class="filter-chip"
        :class="{ active: currentFilter === f.key }"
        @click="handleFilterChange(f.key)"
      >
        {{ f.label }} ({{ getStatusCount(f.key) }})
      </span>
    </div>

    <!-- ===== 任务列表 ===== -->
    <div class="task-list" v-if="filteredTasks.length > 0" v-loading="loading">
      <div
        v-for="task in filteredTasks"
        :key="task.id"
        class="task-item"
        :class="{ done: task.status === 'done' }"
        @click="openDetail(task)"
      >
        <!-- checkbox -->
        <div
          class="task-checkbox"
          :class="{ done: task.status === 'done' }"
          @click.stop="toggleTaskStatus(task)"
        >
          <span v-if="task.status === 'done'" class="check-icon">✓</span>
        </div>

        <div class="task-info">
          <div class="task-title-row">
            <span class="task-title">{{ task.title }}</span>
            <span
              class="task-priority"
              :class="`priority-${mapPriority(task.priority)}`"
            >
              {{ getPriorityLabel(mapPriority(task.priority)) }}
            </span>
          </div>
          <div class="task-desc" v-if="task.description">{{ task.description }}</div>
          <div class="task-meta">
            <span class="meta-item">
              <el-icon><User /></el-icon>
              {{ task.assignee || '未分配' }}
            </span>
            <span class="meta-item">
              <el-icon><Calendar /></el-icon>
              {{ task.due_date || '无截止' }}
            </span>
            <span class="meta-item" v-if="task.project">
              <el-icon><Folder /></el-icon>
              {{ task.project }}
            </span>
          </div>
          <!-- 进度条 -->
          <div class="task-progress" v-if="task.progress !== undefined && task.progress > 0">
            <div class="progress-bar">
              <div class="progress-fill" :style="{ width: task.progress + '%' }"></div>
            </div>
            <span class="progress-text">{{ task.progress }}%</span>
          </div>
        </div>

        <div class="task-actions" @click.stop>
          <el-button size="small" text @click="openEdit(task)">
            <el-icon><Edit /></el-icon>
          </el-button>
          <el-button size="small" text type="danger" @click="deleteTask(task)">
            <el-icon><Delete /></el-icon>
          </el-button>
        </div>
      </div>
    </div>

    <div v-else class="empty-state">
      <div class="empty-icon">📋</div>
      <h3>暂无任务</h3>
      <p>点击右上角"新建任务"创建第一个任务</p>
    </div>

    <!-- ===== 服务端分页（大数据量时启用） ===== -->
    <div v-if="useServerPagination && total > 0" class="pagination-row">
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        background
        @current-change="handlePageChange"
        @size-change="handlePageSizeChange"
      />
    </div>

    <!-- ===== 新建/编辑弹窗 ===== -->
    <el-dialog
      v-model="showDialog"
      :title="editingTask ? '编辑任务' : '新建任务'"
      width="560px"
      class="task-dialog"
      destroy-on-close
    >
      <el-form :model="taskForm" label-position="top">
        <el-form-item label="任务标题" required>
          <el-input v-model="taskForm.title" placeholder="请输入任务标题" />
        </el-form-item>
        <el-form-item label="任务描述">
          <el-input
            v-model="taskForm.description"
            type="textarea"
            :rows="3"
            placeholder="请输入任务描述"
          />
        </el-form-item>
        <div class="form-row">
          <el-form-item label="优先级">
            <el-select v-model="taskForm.priority" style="width: 100%">
              <el-option label="高" value="high" />
              <el-option label="中" value="medium" />
              <el-option label="低" value="low" />
            </el-select>
          </el-form-item>
          <el-form-item label="状态">
            <el-select v-model="taskForm.status" style="width: 100%">
              <el-option label="待办" value="todo" />
              <el-option label="进行中" value="in_progress" />
              <el-option label="已完成" value="done" />
              <el-option label="已取消" value="cancelled" />
            </el-select>
          </el-form-item>
        </div>
        <div class="form-row">
          <el-form-item label="负责人">
            <el-input v-model="taskForm.assignee" placeholder="请输入负责人" />
          </el-form-item>
          <el-form-item label="截止日期">
            <el-date-picker
              v-model="taskForm.due_date"
              type="date"
              placeholder="选择截止日期"
              style="width: 100%"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
        </div>
        <el-form-item label="所属项目">
          <el-input v-model="taskForm.project" placeholder="请输入项目名称（可选）" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showDialog = false">取消</el-button>
        <el-button type="primary" @click="saveTask" :loading="saving">保存</el-button>
      </template>
    </el-dialog>

    <!-- ===== 任务详情抽屉 ===== -->
    <el-drawer
      v-model="showDetail"
      :title="currentTask?.title || '任务详情'"
      size="480px"
      class="task-drawer"
    >
      <div v-if="currentTask" class="detail-content">
        <div class="detail-section">
          <div class="detail-row">
            <span class="detail-label">状态</span>
            <el-tag :type="statusTagType(currentTask.status)" size="small" effect="dark">
              {{ statusLabel(currentTask.status) }}
            </el-tag>
          </div>
          <div class="detail-row">
            <span class="detail-label">优先级</span>
            <span class="task-priority" :class="`priority-${mapPriority(currentTask.priority)}`">
              {{ getPriorityLabel(mapPriority(currentTask.priority)) }}
            </span>
          </div>
          <div class="detail-row">
            <span class="detail-label">负责人</span>
            <span class="detail-value">{{ currentTask.assignee || '未分配' }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">截止日期</span>
            <span class="detail-value">{{ currentTask.due_date || '无截止' }}</span>
          </div>
          <div class="detail-row" v-if="currentTask.project">
            <span class="detail-label">所属项目</span>
            <span class="detail-value">{{ currentTask.project }}</span>
          </div>
          <div class="detail-row" v-if="currentTask.created_at">
            <span class="detail-label">创建时间</span>
            <span class="detail-value">{{ currentTask.created_at }}</span>
          </div>
        </div>

        <div class="detail-section" v-if="currentTask.description">
          <h4 class="detail-section-title">任务描述</h4>
          <p class="detail-desc">{{ currentTask.description }}</p>
        </div>

        <div class="detail-section" v-if="currentTask.progress !== undefined">
          <h4 class="detail-section-title">完成进度</h4>
          <div class="detail-progress">
            <div class="progress-bar-large">
              <div class="progress-fill-large" :style="{ width: currentTask.progress + '%' }"></div>
            </div>
            <span class="progress-percent">{{ currentTask.progress }}%</span>
          </div>
        </div>

        <div class="detail-actions">
          <el-button type="primary" @click="openEdit(currentTask)">
            <el-icon><Edit /></el-icon> 编辑
          </el-button>
          <el-button @click="toggleTaskStatus(currentTask)">
            {{ currentTask.status === 'done' ? '标记为待办' : '标记为完成' }}
          </el-button>
          <el-button type="danger" @click="deleteTask(currentTask)">
            <el-icon><Delete /></el-icon> 删除
          </el-button>
        </div>
      </div>
    </el-drawer>
  </div>
</template>

<script setup>
import { ref, computed, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Plus, Search, Edit, Delete, User, Calendar, Folder
} from '@element-plus/icons-vue'
import { getTasks, createTask, updateTask, deleteTask as apiDeleteTask, getTasksPaginated } from '@/api'

// ===== 状态 =====
const tasks = ref([])
const currentFilter = ref('all')
const searchKeyword = ref('')
const showDialog = ref(false)
const showDetail = ref(false)
const editingTask = ref(null)
const currentTask = ref(null)
const saving = ref(false)
const loading = ref(false)

// ===== 服务端分页状态 =====
const page = ref(1)
const pageSize = ref(20)
const total = ref(0)
const useServerPagination = ref(false)
const sortBy = ref('')
const sortOrder = ref('')

const taskForm = reactive({
  title: '',
  description: '',
  priority: 'medium',
  status: 'todo',
  assignee: '',
  due_date: '',
  project: ''
})

// 状态筛选
const statusFilters = [
  { key: 'all', label: '全部' },
  { key: 'todo', label: '待办' },
  { key: 'in_progress', label: '进行中' },
  { key: 'done', label: '已完成' },
  { key: 'cancelled', label: '已取消' }
]

// ===== 计算属性 =====
const filteredTasks = computed(() => {
  let result = [...tasks.value]

  // 状态筛选
  if (currentFilter.value !== 'all') {
    result = result.filter(t => t.status === currentFilter.value)
  }

  // 搜索
  if (searchKeyword.value.trim()) {
    const kw = searchKeyword.value.toLowerCase()
    result = result.filter(t =>
      t.title.toLowerCase().includes(kw) ||
      (t.description || '').toLowerCase().includes(kw) ||
      (t.assignee || '').toLowerCase().includes(kw)
    )
  }

  // 排序：进行中 > 待办 > 已完成 > 已取消
  const order = { in_progress: 0, todo: 1, done: 2, cancelled: 3 }
  result.sort((a, b) => (order[a.status] ?? 9) - (order[b.status] ?? 9))

  return result
})

// ===== 方法 =====
function getStatusCount(key) {
  if (key === 'all') return tasks.value.length
  return tasks.value.filter(t => t.status === key).length
}

function statusLabel(status) {
  const map = { todo: '待办', in_progress: '进行中', done: '已完成', cancelled: '已取消' }
  return map[status] || status
}

function statusTagType(status) {
  const map = { todo: 'info', in_progress: 'warning', done: 'success', cancelled: 'danger' }
  return map[status] || 'info'
}

function mapPriority(p) {
  if (p === 'high' || p === 'urgent') return 'high'
  if (p === 'low') return 'low'
  return 'mid'
}

function getPriorityLabel(p) {
  const map = { high: '高优', mid: '中优', low: '低优' }
  return map[p] || '中优'
}

async function toggleTaskStatus(task) {
  const newStatus = task.status === 'done' ? 'todo' : 'done'
  const newProgress = newStatus === 'done' ? 100 : 0
  // 先更新本地状态以获得即时反馈
  task.status = newStatus
  task.progress = newProgress
  try {
    await updateTask(task.id, { status: newStatus, progress: newProgress })
    if (newStatus === 'done') {
      ElMessage.success('任务已标记为完成 🎉')
    } else {
      ElMessage.info('任务已恢复为待办')
    }
  } catch (e) {
    // 回滚本地状态
    task.status = newStatus === 'done' ? 'todo' : 'done'
    task.progress = newStatus === 'done' ? 0 : 100
    ElMessage.error('更新任务状态失败，请重试')
  }
}

function openCreate() {
  editingTask.value = null
  Object.assign(taskForm, {
    title: '',
    description: '',
    priority: 'medium',
    status: 'todo',
    assignee: '',
    due_date: '',
    project: ''
  })
  showDialog.value = true
}

function openEdit(task) {
  editingTask.value = task
  Object.assign(taskForm, {
    title: task.title,
    description: task.description || '',
    priority: task.priority || 'medium',
    status: task.status || 'todo',
    assignee: task.assignee || '',
    due_date: task.due_date || '',
    project: task.project || ''
  })
  showDetail.value = false
  showDialog.value = true
}

function openDetail(task) {
  currentTask.value = task
  showDetail.value = true
}

async function saveTask() {
  if (!taskForm.title.trim()) {
    ElMessage.warning('请输入任务标题')
    return
  }
  saving.value = true
  try {
    if (editingTask.value) {
      // 编辑
      await updateTask(editingTask.value.id, { ...taskForm })
      Object.assign(editingTask.value, taskForm)
      ElMessage.success('任务更新成功')
    } else {
      // 新建
      const newTask = await createTask({ ...taskForm })
      tasks.value.unshift({
        ...newTask,
        id: newTask.id || 't_' + Date.now(),
        progress: taskForm.status === 'done' ? 100 : 0,
        created_at: new Date().toLocaleString()
      })
      ElMessage.success('任务创建成功')
    }
    showDialog.value = false
  } catch (e) {
    ElMessage.error('保存失败：' + e.message)
  } finally {
    saving.value = false
  }
}

async function deleteTask(task) {
  try {
    await ElMessageBox.confirm(
      `确定要删除任务「${task.title}」吗？`,
      '删除确认',
      { type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消' }
    )
    await apiDeleteTask(task.id)
    const idx = tasks.value.findIndex(t => t.id === task.id)
    if (idx !== -1) tasks.value.splice(idx, 1)
    if (currentTask.value?.id === task.id) showDetail.value = false
    ElMessage.success('任务已删除')
  } catch (e) {
    if (e !== 'cancel') {
      ElMessage.error('删除失败：' + e.message)
    }
  }
}

async function loadTasks() {
  loading.value = true
  if (useServerPagination.value) {
    try {
      const params = {
        page: page.value,
        page_size: pageSize.value,
        keyword: searchKeyword.value.trim() || undefined,
        status: currentFilter.value !== 'all' ? currentFilter.value : undefined,
        sort_by: sortBy.value || undefined,
        sort_order: sortOrder.value || undefined
      }
      Object.keys(params).forEach(k => params[k] === undefined && delete params[k])
      const result = await getTasksPaginated(params)
      if (result && Array.isArray(result.items)) {
        tasks.value = result.items
        total.value = result.total || 0
        page.value = result.page || page.value
        pageSize.value = result.page_size || pageSize.value
        loading.value = false
        return
      }
      // 响应非分页格式，降级为客户端模式
      useServerPagination.value = false
      total.value = 0
    } catch (e) {
      useServerPagination.value = false
      total.value = 0
      ElMessage.error(e?.message || '服务端分页加载失败，已降级为客户端模式')
    }
  }
  // 客户端模式
  try {
    const data = await getTasks()
    if (Array.isArray(data) && data.length > 0) {
      tasks.value = data
    } else {
      tasks.value = []
    }
  } catch (e) {
    tasks.value = []
    ElMessage.error(e?.message || '任务加载失败')
  } finally {
    loading.value = false
  }
}

function handlePageChange(p) {
  page.value = p
  loadTasks()
}

function handlePageSizeChange(size) {
  pageSize.value = size
  page.value = 1
  loadTasks()
}

function handleSearch() {
  if (useServerPagination.value) {
    page.value = 1
    loadTasks()
  }
}

function handleFilterChange(key) {
  currentFilter.value = key
  if (useServerPagination.value) {
    page.value = 1
    loadTasks()
  }
}

onMounted(() => {
  loadTasks()
})
</script>

<style scoped>
.task-view {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  background: var(--bg-primary);
}

/* 页头 */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
  flex-wrap: wrap;
}
.page-title {
  font-size: 22px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 4px;
}
.page-subtitle {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 0;
}
.header-actions {
  display: flex;
  gap: 10px;
  align-items: center;
  flex-shrink: 0;
}

/* 筛选 chips */
.filter-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

/* 分页 */
.pagination-row {
  display: flex;
  justify-content: center;
  padding: 16px 0;
}
.filter-chip {
  padding: 4px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 16px;
  font-size: 12px;
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
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
}
.task-item {
  display: flex;
  gap: 12px;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background 0.2s;
  align-items: flex-start;
}
.task-item:last-child {
  border-bottom: none;
}
.task-item:hover {
  background: var(--bg-hover);
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
  gap: 10px;
  margin-bottom: 4px;
}
.task-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.4;
}
.task-priority {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 500;
  flex-shrink: 0;
}
.priority-high { background: var(--danger-dim); color: var(--danger); }
.priority-mid { background: var(--warning-dim); color: var(--warning); }
.priority-low { background: var(--success-dim); color: var(--success); }

.task-desc {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.5;
  margin-bottom: 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}
.task-meta {
  display: flex;
  gap: 14px;
  font-size: 11px;
  color: var(--text-muted);
  flex-wrap: wrap;
}
.meta-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

/* 进度条 */
.task-progress {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}
.progress-bar {
  flex: 1;
  max-width: 200px;
  height: 5px;
  background: var(--bg-tertiary);
  border-radius: 3px;
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 3px;
  transition: width 0.3s;
}
.progress-text {
  font-size: 11px;
  color: var(--text-muted);
  font-weight: 600;
}

.task-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
  opacity: 0;
  transition: opacity 0.2s;
}
.task-item:hover .task-actions {
  opacity: 1;
}

/* 空状态 */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  gap: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
}
.empty-icon {
  font-size: 48px;
}
.empty-state h3 {
  font-size: 16px;
  color: var(--text-primary);
  margin: 0;
}
.empty-state p {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 0;
}

/* 表单 */
.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

/* 详情抽屉 */
.detail-content {
  display: flex;
  flex-direction: column;
  gap: 20px;
}
.detail-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.detail-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
}
.detail-label {
  font-size: 13px;
  color: var(--text-muted);
}
.detail-value {
  font-size: 13px;
  color: var(--text-primary);
  font-weight: 500;
}
.detail-section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 4px;
}
.detail-desc {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.7;
  margin: 0;
  background: var(--bg-tertiary);
  padding: 12px;
  border-radius: var(--radius-sm);
}
.detail-progress {
  display: flex;
  align-items: center;
  gap: 12px;
}
.progress-bar-large {
  flex: 1;
  height: 8px;
  background: var(--bg-tertiary);
  border-radius: 4px;
  overflow: hidden;
}
.progress-fill-large {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), var(--accent-light));
  border-radius: 4px;
}
.progress-percent {
  font-size: 14px;
  font-weight: 600;
  color: var(--accent-light);
}
.detail-actions {
  display: flex;
  gap: 10px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}

/* 弹窗/抽屉深色 */
:deep(.el-dialog),
:deep(.el-drawer) {
  background: var(--bg-secondary);
}
:deep(.el-dialog__title),
:deep(.el-drawer__title) {
  color: var(--text-primary);
}
:deep(.el-dialog__body),
:deep(.el-drawer__body) {
  color: var(--text-secondary);
}
:deep(.el-form-item__label) {
  color: var(--text-secondary);
}
:deep(.el-drawer__header) {
  border-bottom: 1px solid var(--border);
  margin-bottom: 16px;
}
</style>
