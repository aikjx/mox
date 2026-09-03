<!--
  任务编排面板
  职责：任务智能拆解、子任务列表、专家分配看板、执行时间线（甘特图/列表）、风险预警
-->
<template>
  <div class="task-orch-view">
    <!-- 顶部：任务概览 + 控制栏 -->
    <div class="orch-top-bar glass-card">
      <div class="orch-progress-section">
        <div class="orch-progress-header">
          <span class="orch-progress-title">
            <span class="orch-title-icon">🎯</span>
            任务总览
          </span>
          <span class="orch-progress-stats">
            <el-tag size="small" type="success" effect="light">{{ orchProgress.completed }} 已完成</el-tag>
            <el-tag size="small" type="primary" effect="light">{{ orchProgress.inProgress }} 进行中</el-tag>
            <el-tag size="small" type="info" effect="light">{{ orchProgress.total - orchProgress.completed - orchProgress.inProgress }} 待处理</el-tag>
            <el-tag size="small" type="danger" effect="light" v-if="orchProgress.failed > 0">{{ orchProgress.failed }} 失败</el-tag>
          </span>
        </div>
        <div class="orch-progress-bar-wrap">
          <div class="orch-progress-bar">
            <div class="orch-progress-fill" :style="{ width: orchProgress.percentage + '%' }"></div>
            <div class="orch-progress-glow" :style="{ width: orchProgress.percentage + '%' }"></div>
          </div>
          <span class="orch-progress-text">{{ orchProgress.percentage }}%</span>
        </div>
      </div>
      <div class="orch-control-section">
        <el-select :model-value="taskOrchestration.executionMode" size="small" class="orch-mode-select" @update:model-value="emit('update:taskOrchestration', { ...taskOrchestration, executionMode: $event })">
          <el-option label="自动执行" value="auto" />
          <el-option label="手动执行" value="manual" />
        </el-select>
        <el-button size="small" class="orch-btn-secondary" @click="$emit('reset-all-tasks')">
          <el-icon><RefreshLeft /></el-icon>
          重置
        </el-button>
        <el-button
          size="small"
          type="primary"
          class="gradient-btn orch-btn-primary"
          @click="$emit('start-task-execution')"
          :disabled="taskOrchestration.subtasks.length === 0 || orchIsRunning"
          :loading="orchIsRunning"
        >
          <el-icon><Promotion /></el-icon>
          {{ orchIsRunning ? '执行中...' : '开始执行' }}
        </el-button>
      </div>
    </div>

    <!-- 三栏工作区 -->
    <div class="orch-main-area">
      <!-- ---- 左栏：任务拆解面板 ---- -->
      <div class="orch-panel orch-panel-left glass-card">
        <div class="orch-panel-header">
          <span class="orch-panel-title">
            <span class="orch-panel-icon">📋</span>
            任务拆解
          </span>
          <span class="orch-task-count">{{ taskOrchestration.subtasks.length }} 个子任务</span>
        </div>

        <!-- 原始任务输入 -->
        <div class="orch-task-input-section">
          <div class="orch-input-label">原始任务描述</div>
          <el-input
            :model-value="taskOrchestration.originalTask"
            type="textarea"
            :rows="3"
            placeholder="请输入需要完成的复杂任务描述，AI 将自动拆解为子任务…"
            resize="none"
            class="orch-task-input"
            @update:model-value="emit('update:taskOrchestration', { ...taskOrchestration, originalTask: $event })"
          />
          <div class="orch-input-actions">
            <el-button
              type="primary"
              class="gradient-btn orch-decompose-btn"
              @click="$emit('decompose-task')"
              :loading="decomposing"
              :disabled="!taskOrchestration.originalTask.trim()"
            >
              <el-icon><MagicStick /></el-icon>
              智能拆解
            </el-button>
            <el-button
              class="orch-add-manual-btn"
              @click="$emit('add-subtask-manually')"
            >
              <el-icon><Plus /></el-icon>
              手动添加
            </el-button>
          </div>
        </div>

        <!-- 子任务列表 -->
        <div class="orch-subtask-list">
          <div class="orch-list-header">
            <span class="orch-list-title">子任务列表</span>
            <div class="orch-list-actions">
              <el-button size="small" text @click="$emit('collapse-all-subtasks')">
                <el-icon><Fold /></el-icon>
                全部折叠
              </el-button>
            </div>
          </div>
          <el-scrollbar class="orch-subtask-scroll">
            <div
              v-for="(task, index) in taskOrchestration.subtasks"
              :key="task.id"
              class="orch-subtask-card"
              :class="{
                'is-selected': activeSubtaskId === task.id,
                'is-dragging': draggingTaskId === task.id,
                'drag-over': dragOverTaskId === task.id
              }"
              draggable="true"
              @dragstart="$emit('task-dragstart', $event, task)"
              @dragend="$emit('task-dragend')"
              @dragover.prevent="$emit('task-dragover', $event, task)"
              @drop="$emit('task-drop', $event, task)"
              @click="$emit('select-subtask', task)"
            >
              <div class="subtask-card-header">
                <div class="subtask-index" :style="{ background: subtaskPriorityGradient(task.priority) }">
                  {{ index + 1 }}
                </div>
                <div class="subtask-title-row">
                  <span class="subtask-title">{{ task.title }}</span>
                  <div class="subtask-status-badge" :class="'status-' + task.status">
                    <span class="status-dot"></span>
                    {{ subtaskStatusText(task.status) }}
                  </div>
                </div>
              </div>
              <div class="subtask-card-body">
                <p class="subtask-desc">{{ task.description }}</p>
                <div class="subtask-meta-row">
                  <el-tag
                    size="small"
                    effect="light"
                    :style="{ borderColor: expertColor(task.suggestedExpertType) + '50', color: expertColor(task.suggestedExpertType) }"
                  >
                    {{ expertEmoji(task.suggestedExpertType) }} {{ EXPERT_TYPES[task.suggestedExpertType] || task.suggestedExpertType }}
                  </el-tag>
                  <span class="subtask-time">
                    <el-icon><Clock /></el-icon>
                    {{ task.estimatedTime }}分钟
                  </span>
                </div>
                <div v-if="task.dependencies && task.dependencies.length > 0" class="subtask-deps">
                  <span class="deps-label">依赖:</span>
                  <span
                    v-for="depId in task.dependencies"
                    :key="depId"
                    class="dep-tag"
                  >
                    #{{ getSubtaskIndex(depId) + 1 }}
                  </span>
                </div>
              </div>
              <div class="subtask-card-actions">
                <button class="subtask-action-btn" @click.stop="$emit('edit-subtask', task)" title="编辑">
                  <el-icon><Edit /></el-icon>
                </button>
                <button class="subtask-action-btn delete" @click.stop="$emit('delete-subtask', task.id)" title="删除">
                  <el-icon><Delete /></el-icon>
                </button>
                <button class="subtask-action-btn" @click.stop="$emit('toggle-subtask-expand', task)" title="展开详情">
                  <el-icon><component :is="task.expanded ? 'ArrowUp' : 'ArrowDown'" /></el-icon>
                </button>
              </div>

              <!-- 展开的详情 -->
              <div v-if="task.expanded" class="subtask-expanded-detail">
                <div class="detail-section">
                  <div class="detail-label">分配专家</div>
                  <div class="assigned-experts">
                    <div
                      v-for="expId in task.expertIds"
                      :key="expId"
                      class="assigned-expert-avatar"
                      :style="{ background: expertGradient(getExpertById(expId)?.type) }"
                      :title="getExpertById(expId)?.name"
                    >
                      {{ expertEmoji(getExpertById(expId)?.type) }}
                    </div>
                    <button v-if="task.expertIds.length === 0" class="add-expert-btn" @click.stop="$emit('open-assign-dialog', task)">
                      <el-icon><Plus /></el-icon>
                      分配专家
                    </button>
                  </div>
                </div>
                <div v-if="task.result" class="detail-section">
                  <div class="detail-label">执行结果</div>
                  <div class="task-result-text">{{ task.result }}</div>
                </div>
              </div>
            </div>
            <el-empty v-if="taskOrchestration.subtasks.length === 0" description="暂无子任务，请输入任务描述并点击智能拆解" :image-size="60">
              <template #description>
                <div class="orch-empty-hint">
                  <p>输入任务描述后点击「智能拆解」</p>
                  <p class="hint-sub">AI 将自动分析并拆分为可执行的子任务</p>
                </div>
              </template>
            </el-empty>
          </el-scrollbar>
        </div>
      </div>

      <!-- ---- 中栏：专家分配区域 ---- -->
      <div class="orch-panel orch-panel-center glass-card">
        <div class="orch-panel-header">
          <span class="orch-panel-title">
            <span class="orch-panel-icon">👥</span>
            专家分配
          </span>
          <el-button size="small" text class="orch-auto-assign-btn" @click="$emit('auto-assign-experts')" :disabled="taskOrchestration.subtasks.length === 0">
            <el-icon><MagicStick /></el-icon>
            智能分配
          </el-button>
        </div>

        <!-- 专家池 -->
        <div class="orch-expert-pool">
          <div class="orch-pool-header">
            <span class="orch-pool-title">可用专家池</span>
            <span class="orch-pool-count">{{ availableExperts.length }} 位</span>
          </div>
          <div class="orch-expert-grid">
            <div
              v-for="expert in availableExperts"
              :key="expert.id"
              class="orch-expert-chip"
              :class="{ 'is-busy': expert.status === 'busy' }"
              draggable="true"
              @dragstart="$emit('expert-dragstart', $event, expert)"
              @dragend="$emit('expert-dragend')"
              :title="expert.name + ' - ' + (expert.capabilities?.join('、') || '')"
            >
              <div class="chip-avatar gradient-avatar" :style="{ background: expertGradient(expert.type) }">
                {{ expertEmoji(expert.type) }}
                <span class="chip-status-dot" :class="'dot-' + expert.status"></span>
              </div>
              <div class="chip-info">
                <span class="chip-name">{{ expert.name }}</span>
                <span class="chip-role">{{ EXPERT_TYPES[expert.type] }}</span>
              </div>
              <div class="chip-load" :title="'当前负载: ' + expertLoad(expert.id) + ' 个任务'">
                <div class="load-bar">
                  <div class="load-fill" :style="{ width: Math.min(expertLoad(expert.id) * 25, 100) + '%' }"></div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 任务分配看板 -->
        <div class="orch-assignment-board">
          <div class="orch-board-header">
            <span class="orch-board-title">任务分配看板</span>
            <span class="orch-board-hint">拖拽专家到任务卡片上进行分配</span>
          </div>
          <el-scrollbar class="orch-board-scroll">
            <div
              v-for="(task, index) in taskOrchestration.subtasks"
              :key="task.id"
              class="orch-task-assign-card"
              :class="['status-' + task.status, { 'drag-over': expertDragOverTaskId === task.id }]"
              @dragover.prevent="$emit('expert-dragover-task', $event, task)"
              @dragleave="$emit('expert-dragleave-task')"
              @drop="$emit('expert-drop-on-task', $event, task)"
              @click="$emit('select-subtask', task)"
            >
              <div class="assign-card-left">
                <div class="assign-task-index" :style="{ background: subtaskPriorityGradient(task.priority) }">
                  {{ index + 1 }}
                </div>
              </div>
              <div class="assign-card-body">
                <div class="assign-task-title-row">
                  <span class="assign-task-title">{{ task.title }}</span>
                  <el-tag size="small" :type="subtaskStatusTagType(task.status)" effect="light">
                    {{ subtaskStatusText(task.status) }}
                  </el-tag>
                </div>
                <p class="assign-task-desc">{{ task.description }}</p>
                <div class="assign-experts-row">
                  <div class="assigned-experts-list">
                    <div
                      v-for="expId in task.expertIds"
                      :key="expId"
                      class="assigned-expert-chip"
                      :style="{ borderColor: expertColor(getExpertById(expId)?.type) }"
                    >
                      <span class="chip-avatar-sm" :style="{ background: expertGradient(getExpertById(expId)?.type) }">
                        {{ expertEmoji(getExpertById(expId)?.type) }}
                      </span>
                      <span class="chip-name-sm">{{ getExpertById(expId)?.name }}</span>
                      <button class="chip-remove" @click.stop="$emit('unassign-expert', task.id, expId)">
                        <el-icon><Close /></el-icon>
                      </button>
                    </div>
                    <button
                      v-if="task.expertIds.length === 0"
                      class="add-expert-inline-btn"
                      @click.stop="$emit('open-assign-dialog', task)"
                    >
                      <el-icon><Plus /></el-icon>
                      分配专家
                    </button>
                  </div>
                </div>
              </div>
              <div class="assign-card-right">
                <div class="task-time-estimate">
                  <el-icon><Clock /></el-icon>
                  <span>{{ task.estimatedTime }}分钟</span>
                </div>
              </div>
            </div>
            <el-empty v-if="taskOrchestration.subtasks.length === 0" description="暂无待分配任务" :image-size="50" />
          </el-scrollbar>
        </div>
      </div>

      <!-- ---- 右栏：任务执行时间线 ---- -->
      <div class="orch-panel orch-panel-right glass-card">
        <div class="orch-panel-header">
          <span class="orch-panel-title">
            <span class="orch-panel-icon">📊</span>
            执行时间线
          </span>
          <div class="orch-timeline-actions">
            <el-button-group size="small">
              <el-button :type="timelineView === 'gantt' ? 'primary' : ''" @click="$emit('update:timelineView', 'gantt')">甘特图</el-button>
              <el-button :type="timelineView === 'list' ? 'primary' : ''" @click="$emit('update:timelineView', 'list')">列表</el-button>
            </el-button-group>
          </div>
        </div>

        <!-- 甘特图视图 -->
        <div v-show="timelineView === 'gantt'" class="orch-gantt-container">
          <div class="gantt-header">
            <div class="gantt-task-col">任务</div>
            <div class="gantt-time-col">
              <div class="gantt-time-scale">
                <span v-for="i in ganttTimeSlots" :key="i" class="time-slot">{{ i * ganttSlotMinutes }}分</span>
              </div>
            </div>
          </div>
          <el-scrollbar class="gantt-body-scroll">
            <div class="gantt-body">
              <div
                v-for="(task, index) in taskOrchestration.subtasks"
                :key="task.id"
                class="gantt-row"
                :class="{ 'is-selected': activeSubtaskId === task.id }"
                @click="$emit('select-subtask', task)"
              >
                <div class="gantt-task-label">
                  <span class="gantt-task-idx">{{ index + 1 }}.</span>
                  <span class="gantt-task-name">{{ task.title }}</span>
                </div>
                <div class="gantt-bar-area">
                  <div class="gantt-grid">
                    <div v-for="i in ganttTimeSlots" :key="i" class="grid-line"></div>
                  </div>
                  <div
                    class="gantt-task-bar"
                    :class="'status-' + task.status"
                    :style="{
                      left: (task.ganttOffset || 0) + '%',
                      width: Math.max(task.ganttWidth || 15, 8) + '%'
                    }"
                  >
                    <div class="gantt-bar-fill"></div>
                    <div class="gantt-bar-glow"></div>
                    <span class="gantt-bar-label">{{ task.estimatedTime }}分钟</span>
                  </div>
                </div>
              </div>
              <el-empty v-if="taskOrchestration.subtasks.length === 0" description="暂无任务时间线" :image-size="50" />
            </div>
          </el-scrollbar>
        </div>

        <!-- 列表视图 -->
        <div v-show="timelineView === 'list'" class="orch-timeline-list">
          <el-scrollbar class="timeline-scroll">
            <div class="timeline-list-inner">
              <div
                v-for="(task, index) in taskOrchestration.subtasks"
                :key="task.id"
                class="timeline-item"
                :class="{ 'is-selected': activeSubtaskId === task.id }"
                @click="$emit('select-subtask', task)"
              >
                <div class="timeline-dot" :class="'status-' + task.status">
                  <el-icon v-if="task.status === 'completed'"><CircleCheckFilled /></el-icon>
                  <span v-else>{{ index + 1 }}</span>
                </div>
                <div class="timeline-line" v-if="index < taskOrchestration.subtasks.length - 1"></div>
                <div class="timeline-content">
                  <div class="timeline-task-title">{{ task.title }}</div>
                  <div class="timeline-task-meta">
                    <span class="timeline-status" :class="'status-' + task.status">
                      {{ subtaskStatusText(task.status) }}
                    </span>
                    <span class="timeline-time">
                      <el-icon><Clock /></el-icon>
                      {{ task.estimatedTime }}分钟
                    </span>
                  </div>
                  <div v-if="task.expertIds && task.expertIds.length > 0" class="timeline-experts">
                    <div
                      v-for="expId in task.expertIds.slice(0, 3)"
                      :key="expId"
                      class="timeline-expert-avatar"
                      :style="{ background: expertGradient(getExpertById(expId)?.type) }"
                      :title="getExpertById(expId)?.name"
                    >
                      {{ expertEmoji(getExpertById(expId)?.type) }}
                    </div>
                    <span v-if="task.expertIds.length > 3" class="timeline-more-experts">
                      +{{ task.expertIds.length - 3 }}
                    </span>
                  </div>
                </div>
              </div>
              <el-empty v-if="taskOrchestration.subtasks.length === 0" description="暂无任务" :image-size="50" />
            </div>
          </el-scrollbar>
        </div>

        <!-- 风险预警区 -->
        <div v-if="riskTasks.length > 0" class="orch-risk-section">
          <div class="risk-section-header">
            <span class="risk-icon">⚠️</span>
            <span class="risk-title">风险预警</span>
            <el-badge :value="riskTasks.length" class="risk-badge" />
          </div>
          <div class="risk-task-list">
            <div v-for="task in riskTasks" :key="task.id" class="risk-task-item" @click="$emit('select-subtask', task)">
              <span class="risk-task-name">{{ task.title }}</span>
              <el-tag size="small" type="warning" effect="light">有风险</el-tag>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import {
  RefreshLeft, Promotion, MagicStick, Plus, Fold, Edit, Delete,
  Clock, Close, CircleCheckFilled
} from '@element-plus/icons-vue'
import { EXPERT_TYPES } from '@/constants/expert.constants'

const props = defineProps({
  taskOrchestration: { type: Object, required: true },
  decomposing: { type: Boolean, default: false },
  orchIsRunning: { type: Boolean, default: false },
  activeSubtaskId: { type: String, default: null },
  timelineView: { type: String, default: 'gantt' },
  draggingTaskId: { type: String, default: null },
  dragOverTaskId: { type: String, default: null },
  expertDragOverTaskId: { type: String, default: null },
  experts: { type: Array, default: () => [] },
  ganttSlotMinutes: { type: Number, default: 15 }
})

const emit = defineEmits([
  'update:timelineView', 'update:taskOrchestration', 'reset-all-tasks', 'start-task-execution',
  'decompose-task', 'add-subtask-manually', 'collapse-all-subtasks',
  'task-dragstart', 'task-dragend', 'task-dragover', 'task-drop',
  'select-subtask', 'edit-subtask', 'delete-subtask', 'toggle-subtask-expand',
  'open-assign-dialog', 'expert-dragstart', 'expert-dragend',
  'expert-dragover-task', 'expert-dragleave-task', 'expert-drop-on-task',
  'unassign-expert', 'auto-assign-experts'
])

const SUBTASK_STATUS_MAP = {
  pending: { label: '待分配', icon: '📋', color: '#64748b' },
  waiting: { label: '等待中', icon: '⏳', color: '#f59e0b' },
  inProgress: { label: '进行中', icon: '🚀', color: '#3b82f6' },
  reviewing: { label: '审核中', icon: '🔍', color: '#8b5cf6' },
  completed: { label: '已完成', icon: '✅', color: '#10b981' },
  atRisk: { label: '有风险', icon: '⚠️', color: '#f97316' },
  failed: { label: '失败', icon: '❌', color: '#ef4444' },
  archived: { label: '已归档', icon: '📦', color: '#64748b' }
}

const orchProgress = computed(() => {
  const subtasks = props.taskOrchestration.subtasks
  const total = subtasks.length
  const completed = subtasks.filter(t => t.status === 'completed').length
  const inProgress = subtasks.filter(t => t.status === 'inProgress' || t.status === 'reviewing').length
  const failed = subtasks.filter(t => t.status === 'failed').length
  const percentage = total > 0 ? Math.round((completed / total) * 100) : 0
  return { total, completed, inProgress, failed, percentage }
})

const availableExperts = computed(() => props.experts.filter(e => e.status !== 'offline'))

const riskTasks = computed(() => props.taskOrchestration.subtasks.filter(t => t.status === 'atRisk'))

const ganttTimeSlots = computed(() => {
  const totalMinutes = props.taskOrchestration.subtasks.reduce((sum, t) => sum + (t.estimatedTime || 0), 0)
  return Math.max(Math.ceil(totalMinutes / props.ganttSlotMinutes), 4)
})

function getExpertById(id) {
  return props.experts.find(e => e.id === id)
}

function getSubtaskIndex(id) {
  return props.taskOrchestration.subtasks.findIndex(t => t.id === id)
}

function expertLoad(expertId) {
  return props.taskOrchestration.subtasks.filter(t =>
    t.expertIds?.includes(expertId) &&
    ['inProgress', 'reviewing', 'pending', 'waiting'].includes(t.status)
  ).length
}

function subtaskStatusText(status) {
  return SUBTASK_STATUS_MAP[status]?.label || status
}

function subtaskStatusTagType(status) {
  const map = {
    pending: 'info', waiting: 'warning', inProgress: 'primary',
    reviewing: 'warning', completed: 'success', atRisk: 'danger',
    failed: 'danger', archived: 'info'
  }
  return map[status] || 'info'
}

function subtaskPriorityGradient(priority) {
  const gradients = {
    high: 'linear-gradient(135deg, #ef4444, #f97316)',
    medium: 'linear-gradient(135deg, #f59e0b, #eab308)',
    low: 'linear-gradient(135deg, #10b981, #14b8a6)'
  }
  return gradients[priority] || gradients.medium
}

function expertColor(type) {
  const colors = {
    algorithm: '#6366f1', architecture: '#6366f1', data: '#10b981',
    ai: '#ec4899', workflow: '#f59e0b', graph: '#06b6d4',
    security: '#ef4444', performance: '#f97316', monitor: '#14b8a6',
    market: '#8b5cf6', mcp: '#0ea5e9', automation: '#84cc16',
    requirement: '#f43f5e', fusion: '#a855f7', operator: '#64748b',
    custom: '#64748b'
  }
  return colors[type] || '#6366f1'
}

function expertGradient(type) {
  const gradients = {
    algorithm: 'linear-gradient(135deg, #6366f1, #8b5cf6)',
    architecture: 'linear-gradient(135deg, #6366f1, #06b6d4)',
    data: 'linear-gradient(135deg, #10b981, #14b8a6)',
    ai: 'linear-gradient(135deg, #ec4899, #8b5cf6)',
    workflow: 'linear-gradient(135deg, #f59e0b, #ef4444)',
    graph: 'linear-gradient(135deg, #06b6d4, #3b82f6)',
    security: 'linear-gradient(135deg, #ef4444, #f97316)',
    performance: 'linear-gradient(135deg, #f97316, #f59e0b)',
    monitor: 'linear-gradient(135deg, #14b8a6, #10b981)',
    market: 'linear-gradient(135deg, #8b5cf6, #ec4899)',
    mcp: 'linear-gradient(135deg, #0ea5e9, #06b6d4)',
    automation: 'linear-gradient(135deg, #84cc16, #10b981)',
    requirement: 'linear-gradient(135deg, #f43f5e, #ec4899)',
    fusion: 'linear-gradient(135deg, #a855f7, #7c3aed)',
    operator: 'linear-gradient(135deg, #64748b, #475569)',
    custom: 'linear-gradient(135deg, #64748b, #475569)'
  }
  return gradients[type] || 'linear-gradient(135deg, #7c3aed, #06b6d4)'
}

function expertEmoji(type) {
  const emojis = {
    algorithm: '🧮', architecture: '🏗️', data: '🔗',
    ai: '🤖', workflow: '⚡', graph: '🕸️',
    security: '🔒', performance: '🚀', monitor: '📊',
    market: '📈', mcp: '🔌', automation: '🤖',
    requirement: '📋', fusion: '🔀', operator: '⚙️',
    custom: '👤'
  }
  return emojis[type] || '👤'
}
</script>
