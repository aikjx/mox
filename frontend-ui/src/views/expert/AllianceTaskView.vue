<!--
  专家联盟 · 协作任务视图
  双栏分层 + AI 助手混合模式
  左栏：任务列表（创建、筛选、任务卡片）
  右栏：任务详情（头部、DAG 协作图、执行日志、融合结果）
  悬浮：AI 助手对话面板
-->
<template>
  <div class="alliance-task-view">
    <div class="phi-layout">
      <!-- ============ 左栏：任务列表 ============ -->
      <aside class="col col-left">
        <!-- 创建任务按钮 -->
        <section class="card create-task-card">
          <el-button type="primary" class="create-btn" @click="showCreateDialog = true">
            <el-icon><Plus /></el-icon>
            创建协作任务
          </el-button>
          <div class="task-stats">
            <div class="ts-item">
              <span class="ts-num">{{ tasks.length }}</span>
              <span class="ts-label">总任务</span>
            </div>
            <div class="ts-item">
              <span class="ts-num running">{{ runningCount }}</span>
              <span class="ts-label">进行中</span>
            </div>
            <div class="ts-item">
              <span class="ts-num completed">{{ completedCount }}</span>
              <span class="ts-label">已完成</span>
            </div>
          </div>
        </section>

        <!-- 状态筛选 -->
        <section class="card filter-card">
          <div class="filter-tabs">
            <div
              v-for="tab in statusTabs"
              :key="tab.key"
              class="filter-tab"
              :class="{ active: statusFilter === tab.key }"
              @click="statusFilter = tab.key"
            >
              <span class="tab-dot" :style="{ background: tab.color }"></span>
              {{ tab.label }}
              <span class="tab-count">{{ getStatusCount(tab.key) }}</span>
            </div>
          </div>
          <div class="search-row">
            <el-input v-model="searchKeyword" placeholder="搜索任务…" clearable size="small">
              <template #prefix><el-icon><Search /></el-icon></template>
            </el-input>
          </div>
        </section>

        <!-- 任务列表 -->
        <section class="card task-list-card">
          <div class="card-header between">
            <span class="card-title">任务列表</span>
            <span class="card-sub">{{ filteredTasks.length }} 个任务</span>
          </div>
          <el-scrollbar class="task-scroll">
            <div v-if="filteredTasks.length === 0" class="empty-tasks">
              <el-empty description="暂无任务" :image-size="50" />
            </div>
            <div
              v-for="task in filteredTasks"
              :key="task.id"
              class="task-card"
              :class="{ active: selectedTaskId === task.id, [task.status]: true }"
              @click="selectTask(task.id)"
            >
              <div class="task-card-header">
                <span class="task-status-dot" :style="{ background: statusColor(task.status) }"></span>
                <span class="task-name">{{ task.name }}</span>
              </div>
              <div class="task-desc">{{ task.description }}</div>
              <div class="task-meta">
                <span class="task-strategy">
                  <el-icon><Share /></el-icon>
                  {{ strategyLabel(task.fusion_strategy) }}
                </span>
                <span class="task-experts">
                  <el-icon><User /></el-icon>
                  {{ task.expert_count }}位专家
                </span>
              </div>
              <div class="task-progress">
                <div class="tp-bar">
                  <div class="tp-fill" :style="{ width: task.progress + '%', background: statusColor(task.status) }"></div>
                </div>
                <span class="tp-text">{{ task.progress }}%</span>
              </div>
              <div class="task-footer">
                <span class="task-time">{{ formatTime(task.created_at) }}</span>
                <el-tag v-if="task.status === 'completed'" size="small" type="success" effect="light">
                  {{ task.duration }}
                </el-tag>
                <el-tag v-else-if="task.status === 'failed'" size="small" type="danger" effect="light">
                  失败
                </el-tag>
                <el-tag v-else-if="task.status === 'running'" size="small" type="primary" effect="light">
                  运行中
                </el-tag>
                <el-tag v-else-if="task.status === 'planning'" size="small" type="warning" effect="light">
                  规划中
                </el-tag>
                <el-tag v-else size="small" effect="plain">等待中</el-tag>
              </div>
            </div>
          </el-scrollbar>
        </section>
      </aside>

      <!-- ============ 右栏：任务详情 ============ -->
      <main class="col col-main">
        <template v-if="selectedTask">
          <!-- 任务详情头部 -->
          <section class="card task-header-card">
            <div class="task-header-top">
              <div class="task-title-area">
                <h2 class="task-title">{{ selectedTask.name }}</h2>
                <el-tag :type="statusTagType(selectedTask.status)" size="default" effect="light">
                  {{ statusLabel(selectedTask.status) }}
                </el-tag>
              </div>
              <div class="task-actions">
                <el-button size="small" @click="restartTask" :disabled="selectedTask.status === 'running'">
                  <el-icon><Refresh /></el-icon>
                  重新执行
                </el-button>
                <el-button size="small" type="primary" @click="toggleStream" v-if="selectedTask.status === 'running'">
                  <el-icon>{{ isStreaming ? <VideoPause /> : <VideoPlay /> }}</el-icon>
                  {{ isStreaming ? '暂停' : '继续' }}
                </el-button>
                <el-button size="small" type="success" v-if="selectedTask.status === 'completed'">
                  <el-icon><Download /></el-icon>
                  导出结果
                </el-button>
              </div>
            </div>
            <div class="task-header-desc">{{ selectedTask.description }}</div>
            <div class="task-progress-large">
              <div class="tpl-info">
                <span class="tpl-label">整体进度</span>
                <span class="tpl-value">{{ selectedTask.progress }}%</span>
              </div>
              <div class="tpl-bar">
                <div class="tpl-fill" :style="{ width: selectedTask.progress + '%' }"></div>
              </div>
              <div class="tpl-stats">
                <span><el-icon><Clock /></el-icon> 预计剩余 {{ selectedTask.eta }}</span>
                <span><el-icon><User /></el-icon> {{ selectedTask.expert_count }} 位专家协作</span>
                <span><el-icon><Share /></el-icon> {{ strategyLabel(selectedTask.fusion_strategy) }}</span>
              </div>
            </div>
          </section>

          <!-- 协作计划 DAG 图 -->
          <section class="card dag-card">
            <div class="card-header between">
              <div>
                <span class="card-title">协作计划 · DAG 流程图</span>
                <span class="card-sub">{{ dagNodes.length }} 节点 · {{ dagEdges.length }} 依赖</span>
              </div>
              <div class="dag-legend">
                <span v-for="type in expertTypeLegend" :key="type.key" class="legend-item">
                  <span class="legend-dot" :style="{ background: type.color }"></span>
                  {{ type.label }}
                </span>
              </div>
            </div>
            <div class="dag-canvas-wrap" ref="dagWrapRef">
              <canvas ref="dagCanvasRef" class="dag-canvas"></canvas>
            </div>
          </section>

          <!-- 执行日志 & 融合结果 双栏 -->
          <div class="bottom-row">
            <!-- 执行日志 -->
            <section class="card logs-card">
              <div class="card-header between">
                <div>
                  <span class="card-title">执行日志</span>
                  <span class="card-sub">实时流式输出</span>
                </div>
                <div class="log-actions">
                  <el-button size="small" text @click="clearLogs">
                    <el-icon><Delete /></el-icon> 清空
                  </el-button>
                  <el-button size="small" text @click="scrollLogsToBottom">
                    <el-icon><Bottom /></el-icon> 底部
                  </el-button>
                </div>
              </div>
              <div class="logs-container" ref="logsContainerRef">
                <div
                  v-for="(log, idx) in executionLogs"
                  :key="idx"
                  class="log-item"
                  :class="'log-' + log.level"
                >
                  <span class="log-time">[{{ log.time }}]</span>
                  <span class="log-expert" :style="{ color: expertColor(log.expert_type) }">
                    [{{ log.expert }}]
                  </span>
                  <span class="log-message">{{ log.message }}</span>
                </div>
                <div v-if="isStreaming" class="log-typing">
                  <span class="typing-dot"></span>
                  <span class="typing-dot"></span>
                  <span class="typing-dot"></span>
                </div>
              </div>
            </section>

            <!-- 融合结果 -->
            <section class="card fusion-card">
              <div class="card-header between">
                <div>
                  <span class="card-title">融合结果对比</span>
                  <span class="card-sub">六大策略</span>
                </div>
                <el-select v-model="fusionViewMode" size="small" style="width: 100px">
                  <el-option label="卡片视图" value="card" />
                  <el-option label="对比视图" value="compare" />
                </el-select>
              </div>

              <!-- 卡片视图 -->
              <div v-if="fusionViewMode === 'card'" class="fusion-cards">
                <div
                  v-for="strategy in fusionStrategies"
                  :key="strategy.key"
                  class="fusion-card-item"
                  :class="{ 'best-score': strategy.best }"
                >
                  <div class="fc-header">
                    <span class="fc-icon">{{ strategy.icon }}</span>
                    <span class="fc-name">{{ strategy.name }}</span>
                    <el-tag v-if="strategy.best" size="small" type="success" effect="dark">最优</el-tag>
                  </div>
                  <div class="fc-score">
                    <span class="fc-score-num">{{ strategy.score }}</span>
                    <span class="fc-score-unit">分</span>
                  </div>
                  <div class="fc-bar">
                    <div class="fc-bar-fill" :style="{ width: strategy.score + '%', background: strategy.color }"></div>
                  </div>
                  <div class="fc-metrics">
                    <div class="fc-metric">
                      <span class="fcm-label">置信度</span>
                      <span class="fcm-value">{{ strategy.confidence }}%</span>
                    </div>
                    <div class="fc-metric">
                      <span class="fcm-label">一致性</span>
                      <span class="fcm-value">{{ strategy.consistency }}%</span>
                    </div>
                  </div>
                  <div class="fc-summary">{{ strategy.summary }}</div>
                </div>
              </div>

              <!-- 对比视图 -->
              <div v-else class="fusion-compare">
                <div class="compare-row compare-header">
                  <span class="compare-name">策略</span>
                  <span class="compare-score">综合评分</span>
                  <span class="compare-bar-label">置信度 / 一致性 / 覆盖度</span>
                </div>
                <div v-for="strategy in fusionStrategies" :key="strategy.key" class="compare-row">
                  <span class="compare-name">
                    <span class="cn-icon">{{ strategy.icon }}</span>
                    {{ strategy.name }}
                  </span>
                  <span class="compare-score" :style="{ color: strategy.color }">
                    {{ strategy.score }}
                  </span>
                  <div class="compare-bars">
                    <div class="cb-item">
                      <div class="cb-bar"><div class="cb-fill" :style="{ width: strategy.confidence + '%', background: '#6366f1' }"></div></div>
                      <span class="cb-label">{{ strategy.confidence }}%</span>
                    </div>
                    <div class="cb-item">
                      <div class="cb-bar"><div class="cb-fill" :style="{ width: strategy.consistency + '%', background: '#10b981' }"></div></div>
                      <span class="cb-label">{{ strategy.consistency }}%</span>
                    </div>
                    <div class="cb-item">
                      <div class="cb-bar"><div class="cb-fill" :style="{ width: strategy.coverage + '%', background: '#0ea5e9' }"></div></div>
                      <span class="cb-label">{{ strategy.coverage }}%</span>
                    </div>
                  </div>
                </div>
              </div>
            </section>
          </div>
        </template>

        <!-- 未选择任务时的空状态 -->
        <div v-else class="empty-detail">
          <div class="empty-illustration">
            <div class="empty-orb">
              <svg viewBox="0 0 64 64" class="eo-svg">
                <defs>
                  <linearGradient id="grad1" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" style="stop-color:#6366f1;stop-opacity:1" />
                    <stop offset="100%" style="stop-color:#0ea5e9;stop-opacity:1" />
                  </linearGradient>
                </defs>
                <circle cx="32" cy="32" r="28" fill="none" stroke="url(#grad1)" stroke-width="2" stroke-dasharray="8 4" />
                <circle cx="32" cy="32" r="18" fill="none" stroke="url(#grad1)" stroke-width="1.5" opacity="0.5" />
                <circle cx="32" cy="32" r="6" fill="url(#grad1)" />
              </svg>
            </div>
          </div>
          <h3 class="empty-title">选择一个任务查看详情</h3>
          <p class="empty-sub">从左侧任务列表中选择任务，或创建新的协作任务</p>
          <el-button type="primary" @click="showCreateDialog = true">
            <el-icon><Plus /></el-icon>
            创建新任务
          </el-button>
        </div>
      </main>
    </div>

    <!-- ============ AI 助手悬浮按钮 ============ -->
    <div class="ai-assistant-float" @click="toggleAIPanel">
      <div class="aif-button" :class="{ active: showAIPanel }">
        <el-icon v-if="!showAIPanel" class="aif-icon"><ChatDotRound /></el-icon>
        <el-icon v-else class="aif-icon"><Close /></el-icon>
      </div>
      <div v-if="!showAIPanel" class="aif-tooltip">AI 助手</div>
    </div>

    <!-- ============ AI 助手侧边面板 ============ -->
    <transition name="slide-right">
      <div v-if="showAIPanel" class="ai-panel">
        <div class="ai-panel-header">
          <div class="ai-panel-title">
            <div class="ai-panel-avatar">
              <el-icon><MagicStick /></el-icon>
            </div>
            <div>
              <div class="ai-panel-name">联盟 AI 助手</div>
              <div class="ai-panel-status">
                <span class="status-dot online"></span>
                在线 · 随时为您服务
              </div>
            </div>
          </div>
          <el-button text @click="showAIPanel = false">
            <el-icon><Close /></el-icon>
          </el-button>
        </div>

        <div class="ai-panel-messages" ref="aiMessagesRef">
          <div class="ai-msg ai-msg-bot">
            <div class="msg-avatar bot">
              <el-icon><MagicStick /></el-icon>
            </div>
            <div class="msg-content">
              <div class="msg-bubble bot">
                你好！我是联盟 AI 助手 👋<br/><br/>
                我可以帮你：<br/>
                • 解释协作任务的执行状态<br/>
                • 分析融合结果的差异<br/>
                • 推荐最优融合策略<br/>
                • 解答专家联盟相关问题<br/><br/>
                有什么可以帮你的吗？
              </div>
            </div>
          </div>

          <div v-for="(msg, idx) in aiMessages" :key="idx" class="ai-msg" :class="msg.role === 'user' ? 'ai-msg-user' : 'ai-msg-bot'">
            <div class="msg-avatar" :class="msg.role === 'user' ? 'user' : 'bot'">
              <el-icon v-if="msg.role === 'user'"><User /></el-icon>
              <el-icon v-else><MagicStick /></el-icon>
            </div>
            <div class="msg-content">
              <div class="msg-bubble" :class="msg.role === 'user' ? 'user' : 'bot'">
                {{ msg.content }}
              </div>
            </div>
          </div>

          <div v-if="aiTyping" class="ai-msg ai-msg-bot">
            <div class="msg-avatar bot">
              <el-icon><MagicStick /></el-icon>
            </div>
            <div class="msg-content">
              <div class="msg-bubble bot typing-bubble">
                <span class="typing-dot"></span>
                <span class="typing-dot"></span>
                <span class="typing-dot"></span>
              </div>
            </div>
          </div>
        </div>

        <div class="ai-panel-quick">
          <div class="quick-title">快捷问题</div>
          <div class="quick-btns">
            <el-button v-for="q in quickQuestions" :key="q" size="small" @click="sendAIQuestion(q)">
              {{ q }}
            </el-button>
          </div>
        </div>

        <div class="ai-panel-input">
          <el-input
            v-model="aiInput"
            type="textarea"
            :rows="2"
            placeholder="输入你的问题…"
            @keydown.enter.ctrl="sendAIMessage"
          />
          <div class="input-actions">
            <span class="input-hint">Ctrl + Enter 发送</span>
            <el-button type="primary" :disabled="!aiInput.trim()" @click="sendAIMessage">
              发送 <el-icon><Promotion /></el-icon>
            </el-button>
          </div>
        </div>
      </div>
    </transition>
    <div v-if="showAIPanel" class="ai-panel-mask" @click="showAIPanel = false"></div>

    <!-- ============ 创建任务对话框 ============ -->
    <el-dialog
      v-model="showCreateDialog"
      title="创建协作任务"
      width="520px"
      :close-on-click-modal="false"
    >
      <el-form :model="newTaskForm" label-position="top">
        <el-form-item label="任务名称" required>
          <el-input v-model="newTaskForm.name" placeholder="输入任务名称，如：电商系统架构优化方案" maxlength="50" show-word-limit />
        </el-form-item>
        <el-form-item label="问题描述" required>
          <el-input
            v-model="newTaskForm.description"
            type="textarea"
            :rows="4"
            placeholder="详细描述你需要解决的问题，系统将自动匹配专家并规划协作流程…"
            maxlength="500"
            show-word-limit
          />
        </el-form-item>
        <el-form-item label="融合策略">
          <el-radio-group v-model="newTaskForm.fusion_strategy" class="strategy-radio-group">
            <el-radio v-for="s in fusionStrategyOptions" :key="s.key" :value="s.key" border>
              <span class="sr-icon">{{ s.icon }}</span>
              <span class="sr-name">{{ s.name }}</span>
            </el-radio>
          </el-radio-group>
          <div class="strategy-hint">{{ currentStrategyHint }}</div>
        </el-form-item>
        <el-form-item label="参与专家">
          <el-select v-model="newTaskForm.expert_ids" multiple placeholder="选择参与专家（留空则智能匹配）" style="width: 100%">
            <el-option v-for="exp in expertOptions" :key="exp.id" :label="exp.name" :value="exp.id">
              <span style="float: left">{{ exp.name }}</span>
              <span style="float: right; color: #8492a6; font-size: 12px">{{ exp.typeLabel }}</span>
            </el-option>
          </el-select>
          <div class="expert-hint">不选择专家时，系统将根据问题描述智能匹配最优专家组</div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreateDialog = false">取消</el-button>
        <el-button type="primary" :loading="creatingTask" @click="createTask">
          创建并启动
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Plus, Search, User, Refresh, Download, Clock, Share,
  ChatDotRound, Close, MagicStick, Promotion, Delete, Bottom,
  VideoPlay, VideoPause
} from '@element-plus/icons-vue'
import { EXPERT_TYPES } from '@/constants/expert.constants'

// ===== 状态管理 =====
const statusFilter = ref('all')
const searchKeyword = ref('')
const selectedTaskId = ref(null)
const showCreateDialog = ref(false)
const creatingTask = ref(false)
const showAIPanel = ref(false)
const aiInput = ref('')
const aiTyping = ref(false)
const fusionViewMode = ref('card')
const isStreaming = ref(false)

// ===== 模拟数据：任务列表 =====
const tasks = ref([
  {
    id: 'task-001',
    name: '电商系统微服务架构设计',
    description: '针对现有单体电商系统，设计微服务拆分方案，包括服务边界划分、API 网关、数据一致性策略等',
    status: 'completed',
    progress: 100,
    fusion_strategy: 'debate',
    expert_count: 5,
    created_at: '2024-01-15 10:30:00',
    duration: '12分30秒',
    eta: '已完成'
  },
  {
    id: 'task-002',
    name: '智能推荐算法优化',
    description: '优化现有推荐系统的召回和排序算法，提升 CTR 和转化率，解决冷启动问题',
    status: 'running',
    progress: 67,
    fusion_strategy: 'weighted_vote',
    expert_count: 4,
    created_at: '2024-01-16 14:20:00',
    duration: null,
    eta: '约 5 分钟'
  },
  {
    id: 'task-003',
    name: '数据中台架构规划',
    description: '设计企业级数据中台架构，包括数据分层、指标体系、数据治理、数据安全等',
    status: 'planning',
    progress: 15,
    fusion_strategy: 'iterative_refine',
    expert_count: 6,
    created_at: '2024-01-17 09:00:00',
    duration: null,
    eta: '规划中'
  },
  {
    id: 'task-004',
    name: '高并发秒杀系统设计',
    description: '设计支持百万级 QPS 的秒杀系统，包括流量削峰、库存扣减、分布式锁等核心技术',
    status: 'completed',
    progress: 100,
    fusion_strategy: 'map_reduce',
    expert_count: 4,
    created_at: '2024-01-14 16:45:00',
    duration: '8分15秒',
    eta: '已完成'
  },
  {
    id: 'task-005',
    name: '知识图谱构建方案',
    description: '从 0 到 1 构建企业知识图谱，包括本体设计、知识抽取、知识融合、图存储选型',
    status: 'pending',
    progress: 0,
    fusion_strategy: 'stacking',
    expert_count: 3,
    created_at: '2024-01-17 11:00:00',
    duration: null,
    eta: '等待启动'
  },
  {
    id: 'task-006',
    name: 'CI/CD 流水线优化',
    description: '优化现有 CI/CD 流水线，缩短构建时间，提升部署效率，增加自动化测试覆盖率',
    status: 'failed',
    progress: 45,
    fusion_strategy: 'confidence_weighted',
    expert_count: 3,
    created_at: '2024-01-13 08:30:00',
    duration: null,
    eta: '执行失败'
  }
])

// ===== 状态配置 =====
const statusTabs = [
  { key: 'all', label: '全部', color: '#64748b' },
  { key: 'running', label: '运行中', color: '#6366f1' },
  { key: 'planning', label: '规划中', color: '#f59e0b' },
  { key: 'pending', label: '等待中', color: '#94a3b8' },
  { key: 'completed', label: '已完成', color: '#10b981' },
  { key: 'failed', label: '失败', color: '#ef4444' }
]

function statusColor(status) {
  const colors = {
    pending: '#94a3b8',
    planning: '#f59e0b',
    running: '#6366f1',
    completed: '#10b981',
    failed: '#ef4444'
  }
  return colors[status] || '#94a3b8'
}

function statusLabel(status) {
  const labels = {
    pending: '等待中',
    planning: '规划中',
    running: '运行中',
    completed: '已完成',
    failed: '失败'
  }
  return labels[status] || status
}

function statusTagType(status) {
  const types = {
    pending: 'info',
    planning: 'warning',
    running: 'primary',
    completed: 'success',
    failed: 'danger'
  }
  return types[status] || 'info'
}

// ===== 计算属性 =====
const filteredTasks = computed(() => {
  let list = tasks.value
  if (statusFilter.value !== 'all') {
    list = list.filter(t => t.status === statusFilter.value)
  }
  if (searchKeyword.value.trim()) {
    const kw = searchKeyword.value.toLowerCase()
    list = list.filter(t =>
      t.name.toLowerCase().includes(kw) ||
      t.description.toLowerCase().includes(kw)
    )
  }
  return list
})

const selectedTask = computed(() =>
  tasks.value.find(t => t.id === selectedTaskId.value) || null
)

const runningCount = computed(() => tasks.value.filter(t => t.status === 'running').length)
const completedCount = computed(() => tasks.value.filter(t => t.status === 'completed').length)

function getStatusCount(key) {
  if (key === 'all') return tasks.value.length
  return tasks.value.filter(t => t.status === key).length
}

// ===== 融合策略 =====
const fusionStrategyOptions = [
  { key: 'weighted_vote', name: '加权投票', icon: '⚖️', hint: '根据专家权重进行加权投票，适用于明确的决策场景' },
  { key: 'confidence_weighted', name: '置信度加权', icon: '🎯', hint: '基于各专家输出的置信度进行加权融合' },
  { key: 'stacking', name: '堆叠融合', icon: '📊', hint: '多层级堆叠融合，将专家输出作为特征进行二次学习' },
  { key: 'debate', name: '辩论融合', icon: '💬', hint: '多轮交叉辩论，通过观点碰撞达成共识' },
  { key: 'map_reduce', name: 'Map-Reduce', icon: '🔀', hint: '分而治之，先并行分析再汇总归纳' },
  { key: 'iterative_refine', name: '迭代精炼', icon: '🔄', hint: '多轮迭代逐步精炼，每次基于前一轮结果优化' }
]

function strategyLabel(key) {
  const s = fusionStrategyOptions.find(s => s.key === key)
  return s ? s.name : key
}

// ===== 融合结果模拟数据 =====
const fusionStrategies = ref([
  {
    key: 'weighted_vote',
    name: '加权投票',
    icon: '⚖️',
    score: 86,
    confidence: 88,
    consistency: 82,
    coverage: 90,
    color: '#6366f1',
    best: false,
    summary: '基于专家权重的投票结果，架构专家意见占主导，整体方案偏保守但稳妥。'
  },
  {
    key: 'confidence_weighted',
    name: '置信度加权',
    icon: '🎯',
    score: 89,
    confidence: 92,
    consistency: 85,
    coverage: 88,
    color: '#0ea5e9',
    best: true,
    summary: '综合各专家置信度的加权结果，算法专家在性能评估方面置信度最高，数据专家在一致性方面表现突出。'
  },
  {
    key: 'stacking',
    name: '堆叠融合',
    icon: '📊',
    score: 84,
    confidence: 85,
    consistency: 88,
    coverage: 82,
    color: '#10b981',
    best: false,
    summary: '两层堆叠融合结果，第一层专家输出作为特征，第二层元学习器综合判断。'
  },
  {
    key: 'debate',
    name: '辩论融合',
    icon: '💬',
    score: 91,
    confidence: 90,
    consideration: 95,
    consistency: 78,
    coverage: 95,
    color: '#ec4899',
    best: false,
    summary: '经过三轮辩论后的综合结论，覆盖了更多边缘情况和备选方案，但一致性稍低。'
  },
  {
    key: 'map_reduce',
    name: 'Map-Reduce',
    icon: '🔀',
    score: 83,
    confidence: 80,
    consistency: 90,
    coverage: 85,
    color: '#f59e0b',
    best: false,
    summary: '分而治之的结果，各子问题由专精专家处理，最终汇总为完整方案。'
  },
  {
    key: 'iterative_refine',
    name: '迭代精炼',
    icon: '🔄',
    score: 88,
    confidence: 86,
    consistency: 92,
    coverage: 87,
    color: '#8b5cf6',
    best: false,
    summary: '经过五轮迭代精炼后的方案，每轮都基于前一轮反馈进行优化，一致性最高。'
  }
])

// ===== 专家选项 =====
const expertOptions = ref([
  { id: 'exp-001', name: '林算法', type: 'algorithm', typeLabel: '算法专家' },
  { id: 'exp-002', name: '陈架构', type: 'architecture', typeLabel: '架构专家' },
  { id: 'exp-003', name: '王数据', type: 'data', typeLabel: '数据专家' },
  { id: 'exp-004', name: '张AI', type: 'ai', typeLabel: 'AI专家' },
  { id: 'exp-005', name: '李工作流', type: 'workflow', typeLabel: '工作流专家' },
  { id: 'exp-006', name: '赵图谱', type: 'graph', typeLabel: '知识图谱专家' },
  { id: 'exp-007', name: '孙安全', type: 'security', typeLabel: '安全专家' },
  { id: 'exp-008', name: '周性能', type: 'performance', typeLabel: '性能优化专家' }
])

function expertColor(type) {
  const colors = {
    algorithm: '#6366f1', architecture: '#6366f1', data: '#10b981',
    ai: '#ec4899', workflow: '#f59e0b', graph: '#06b6d4',
    security: '#ef4444', performance: '#f97316', monitor: '#14b8a6',
    market: '#8b5cf6', mcp: '#0ea5e9', automation: '#84cc16',
    requirement: '#f43f5e', fusion: '#a855f7', operator: '#64748b'
  }
  return colors[type] || '#6366f1'
}

// ===== DAG 图数据 =====
const dagNodes = ref([
  { id: 'start', label: '任务启动', type: 'start', x: 50, y: 120 },
  { id: 'exp-arch', label: '陈架构\n架构分析', type: 'architecture', x: 180, y: 60 },
  { id: 'exp-algo', label: '林算法\n复杂度评估', type: 'algorithm', x: 180, y: 120 },
  { id: 'exp-data', label: '王数据\n数据建模', type: 'data', x: 180, y: 180 },
  { id: 'exp-perf', label: '周性能\n性能预估', type: 'performance', x: 350, y: 60 },
  { id: 'exp-sec', label: '孙安全\n安全审计', type: 'security', x: 350, y: 180 },
  { id: 'fusion', label: '融合决策', type: 'fusion', x: 500, y: 120 },
  { id: 'end', label: '任务完成', type: 'end', x: 620, y: 120 }
])

const dagEdges = ref([
  { source: 'start', target: 'exp-arch' },
  { source: 'start', target: 'exp-algo' },
  { source: 'start', target: 'exp-data' },
  { source: 'exp-arch', target: 'exp-perf' },
  { source: 'exp-algo', target: 'exp-perf' },
  { source: 'exp-algo', target: 'exp-sec' },
  { source: 'exp-data', target: 'exp-sec' },
  { source: 'exp-perf', target: 'fusion' },
  { source: 'exp-sec', target: 'fusion' },
  { source: 'fusion', target: 'end' }
])

const expertTypeLegend = [
  { key: 'architecture', label: '架构', color: '#6366f1' },
  { key: 'algorithm', label: '算法', color: '#6366f1' },
  { key: 'data', label: '数据', color: '#10b981' },
  { key: 'performance', label: '性能', color: '#f97316' },
  { key: 'security', label: '安全', color: '#ef4444' },
  { key: 'fusion', label: '融合', color: '#a855f7' }
]

// ===== DAG Canvas 渲染 =====
const dagCanvasRef = ref(null)
const dagWrapRef = ref(null)

function drawDAG() {
  const canvas = dagCanvasRef.value
  const wrap = dagWrapRef.value
  if (!canvas || !wrap) return

  const ctx = canvas.getContext('2d')
  const rect = wrap.getBoundingClientRect()
  const dpr = window.devicePixelRatio || 1
  canvas.width = rect.width * dpr
  canvas.height = rect.height * dpr
  canvas.style.width = rect.width + 'px'
  canvas.style.height = rect.height + 'px'
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)

  const W = rect.width
  const H = rect.height

  // 缩放和居中
  const nodes = dagNodes.value
  const edges = dagEdges.value

  const minX = Math.min(...nodes.map(n => n.x))
  const maxX = Math.max(...nodes.map(n => n.x))
  const minY = Math.min(...nodes.map(n => n.y))
  const maxY = Math.max(...nodes.map(n => n.y))

  const graphW = maxX - minX + 80
  const graphH = maxY - minY + 60
  const scale = Math.min((W - 40) / graphW, (H - 40) / graphH, 1.2)
  const offsetX = (W - graphW * scale) / 2 - minX * scale + 40
  const offsetY = (H - graphH * scale) / 2 - minY * scale + 30

  function getPos(node) {
    return {
      x: node.x * scale + offsetX,
      y: node.y * scale + offsetY
    }
  }

  // 画边（带箭头）
  ctx.strokeStyle = '#cbd5e1'
  ctx.lineWidth = 1.5
  edges.forEach(e => {
    const s = nodes.find(n => n.id === e.source)
    const t = nodes.find(n => n.id === e.target)
    if (!s || !t) return
    const sp = getPos(s)
    const tp = getPos(t)

    // 贝塞尔曲线
    const dx = tp.x - sp.x
    const cp1x = sp.x + dx * 0.5
    const cp2x = tp.x - dx * 0.5

    ctx.beginPath()
    ctx.moveTo(sp.x + 30 * scale, sp.y)
    ctx.bezierCurveTo(cp1x + 30 * scale, sp.y, cp2x - 30 * scale, tp.y, tp.x - 30 * scale, tp.y)
    ctx.stroke()

    // 箭头
    const angle = Math.atan2(0, tp.x - sp.x)
    const arrowX = tp.x - 30 * scale
    const arrowY = tp.y
    ctx.fillStyle = '#cbd5e1'
    ctx.beginPath()
    ctx.moveTo(arrowX, arrowY)
    ctx.lineTo(arrowX - 6, arrowY - 4)
    ctx.lineTo(arrowX - 6, arrowY + 4)
    ctx.closePath()
    ctx.fill()
  })

  // 画节点
  nodes.forEach(n => {
    const pos = getPos(n)
    const color = n.type === 'start' ? '#10b981'
      : n.type === 'end' ? '#6366f1'
      : n.type === 'fusion' ? '#a855f7'
      : expertColor(n.type)

    const nodeW = 90 * scale
    const nodeH = 44 * scale
    const rx = pos.x - nodeW / 2
    const ry = pos.y - nodeH / 2

    // 阴影
    ctx.shadowColor = color + '30'
    ctx.shadowBlur = 8
    ctx.shadowOffsetY = 2

    // 节点背景
    ctx.beginPath()
    const r = 8 * scale
    ctx.moveTo(rx + r, ry)
    ctx.lineTo(rx + nodeW - r, ry)
    ctx.quadraticCurveTo(rx + nodeW, ry, rx + nodeW, ry + r)
    ctx.lineTo(rx + nodeW, ry + nodeH - r)
    ctx.quadraticCurveTo(rx + nodeW, ry + nodeH, rx + nodeW - r, ry + nodeH)
    ctx.lineTo(rx + r, ry + nodeH)
    ctx.quadraticCurveTo(rx, ry + nodeH, rx, ry + nodeH - r)
    ctx.lineTo(rx, ry + r)
    ctx.quadraticCurveTo(rx, ry, rx + r, ry)
    ctx.closePath()
    ctx.fillStyle = '#fff'
    ctx.fill()

    ctx.shadowColor = 'transparent'

    // 顶部色条
    ctx.beginPath()
    ctx.moveTo(rx + r, ry)
    ctx.lineTo(rx + nodeW - r, ry)
    ctx.quadraticCurveTo(rx + nodeW, ry, rx + nodeW, ry + r)
    ctx.lineTo(rx + nodeW, ry + 4 * scale)
    ctx.lineTo(rx, ry + 4 * scale)
    ctx.lineTo(rx, ry + r)
    ctx.quadraticCurveTo(rx, ry, rx + r, ry)
    ctx.closePath()
    ctx.fillStyle = color
    ctx.fill()

    // 边框
    ctx.strokeStyle = color + '40'
    ctx.lineWidth = 1
    ctx.beginPath()
    ctx.moveTo(rx + r, ry)
    ctx.lineTo(rx + nodeW - r, ry)
    ctx.quadraticCurveTo(rx + nodeW, ry, rx + nodeW, ry + r)
    ctx.lineTo(rx + nodeW, ry + nodeH - r)
    ctx.quadraticCurveTo(rx + nodeW, ry + nodeH, rx + nodeW - r, ry + nodeH)
    ctx.lineTo(rx + r, ry + nodeH)
    ctx.quadraticCurveTo(rx, ry + nodeH, rx, ry + nodeH - r)
    ctx.lineTo(rx, ry + r)
    ctx.quadraticCurveTo(rx, ry, rx + r, ry)
    ctx.closePath()
    ctx.stroke()

    // 文字
    ctx.fillStyle = '#1e293b'
    ctx.font = `${11 * scale}px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", sans-serif`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    const lines = n.label.split('\n')
    lines.forEach((line, i) => {
      ctx.fillText(line, pos.x, pos.y + (i - (lines.length - 1) / 2) * 14 * scale + 2 * scale)
    })
  })
}

// ===== 执行日志 =====
const executionLogs = ref([])
const logsContainerRef = ref(null)
let streamInterval = null

const mockLogTemplates = [
  { expert: '陈架构', expert_type: 'architecture', level: 'info', messages: [
    '开始分析系统架构现状...',
    '识别出 3 个核心服务边界...',
    '建议采用领域驱动设计进行服务拆分...',
    '输出架构设计文档 v1.0...',
    '完成架构方案评审，共 5 个备选方案...'
  ]},
  { expert: '林算法', expert_type: 'algorithm', level: 'info', messages: [
    '评估核心业务逻辑复杂度...',
    '时间复杂度: O(n log n)，空间复杂度: O(n)...',
    '推荐使用动态规划优化路径查找...',
    '完成算法选型报告...',
    '性能预估：单节点 QPS 可达 5000+...'
  ]},
  { expert: '王数据', expert_type: 'data', level: 'info', messages: [
    '分析数据模型设计...',
    '识别出 12 个核心实体和 28 个关系...',
    '建议采用分库分表策略...',
    '设计数据同步方案（CDC + 消息队列）...',
    '完成数据治理框架设计...'
  ]},
  { expert: '周性能', expert_type: 'performance', level: 'info', messages: [
    '执行性能压力测试...',
    '当前瓶颈：数据库连接池不足...',
    '建议引入 Redis 缓存层...',
    '优化后预估性能提升 300%...',
    '输出性能优化建议报告...'
  ]},
  { expert: '孙安全', expert_type: 'security', level: 'warn', messages: [
    '启动安全审计扫描...',
    '发现 2 个高危漏洞：SQL 注入风险...',
    '建议使用参数化查询和 ORM...',
    '补充身份认证和授权机制...',
    '完成安全审计报告，风险等级：中...'
  ]},
  { expert: '融合引擎', expert_type: 'fusion', level: 'success', messages: [
    '收集所有专家输出...',
    '执行加权投票融合...',
    '执行置信度加权融合...',
    '执行辩论融合（3 轮）...',
    '执行 Map-Reduce 融合...',
    '执行迭代精炼融合（5 轮）...',
    '生成六大策略对比报告...',
    '推荐策略：置信度加权融合（综合评分 89）...',
    '任务执行完成！'
  ]}
]

function generateLog() {
  const template = mockLogTemplates[Math.floor(Math.random() * mockLogTemplates.length)]
  const message = template.messages[Math.floor(Math.random() * template.messages.length)]
  const now = new Date()
  const time = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')}`
  return {
    time,
    expert: template.expert,
    expert_type: template.expert_type,
    level: template.level,
    message
  }
}

function startStreaming() {
  if (streamInterval) return
  isStreaming.value = true
  streamInterval = setInterval(() => {
    if (executionLogs.value.length > 100) {
      executionLogs.value.shift()
    }
    executionLogs.value.push(generateLog())
    nextTick(() => {
      scrollLogsToBottom()
    })
  }, 800)
}

function stopStreaming() {
  if (streamInterval) {
    clearInterval(streamInterval)
    streamInterval = null
  }
  isStreaming.value = false
}

function toggleStream() {
  if (isStreaming.value) {
    stopStreaming()
  } else {
    startStreaming()
  }
}

function scrollLogsToBottom() {
  if (logsContainerRef.value) {
    logsContainerRef.value.scrollTop = logsContainerRef.value.scrollHeight
  }
}

function clearLogs() {
  executionLogs.value = []
}

// ===== AI 助手 =====
const aiMessages = ref([])
const aiMessagesRef = ref(null)

const quickQuestions = [
  '当前任务进度如何？',
  '推荐哪个融合策略？',
  '解释一下 DAG 图',
  '专家表现怎么样？'
]

const aiResponses = {
  '当前任务进度如何？': '当前任务「智能推荐算法优化」已完成 67%，共有 4 位专家参与协作。目前算法专家和数据专家已完成分析，性能专家正在进行性能压测，安全专家等待输入。预计还需要约 5 分钟完成全部流程。',
  '推荐哪个融合策略？': '根据当前任务特点（算法优化类），我推荐使用「置信度加权融合」策略。理由：\n1. 算法专家在复杂度评估方面置信度高达 92%\n2. 各专家领域分工明确，交叉少\n3. 置信度加权能更好地发挥各专家的领域优势\n综合评分 89 分，为当前最优策略。',
  '解释一下 DAG 图': 'DAG（有向无环图）展示了专家协作的依赖关系：\n\n• 第一层：架构分析、复杂度评估、数据建模并行执行\n• 第二层：性能预估依赖架构和算法的输出\n• 第三层：安全审计依赖算法和数据的输出\n• 第四层：融合决策汇总所有专家结果\n• 最终：任务完成\n\n这种设计最大化了并行度，同时保证了依赖关系的正确性。',
  '专家表现怎么样？': '本次任务中各专家表现：\n\n• 林算法（算法专家）：置信度 92%，响应速度快，分析深入 ⭐\n• 陈架构（架构专家）：置信度 88%，方案考虑全面\n• 王数据（数据专家）：置信度 85%，数据模型设计合理\n• 周性能（性能专家）：进行中...\n\n整体专家团队配合良好，输出质量较高。'
}

function toggleAIPanel() {
  showAIPanel.value = !showAIPanel.value
  if (showAIPanel.value) {
    nextTick(() => {
      scrollAIMessagesToBottom()
    })
  }
}

function sendAIQuestion(question) {
  aiInput.value = question
  sendAIMessage()
}

function sendAIMessage() {
  const content = aiInput.value.trim()
  if (!content) return

  aiMessages.value.push({ role: 'user', content })
  aiInput.value = ''
  aiTyping.value = true

  nextTick(() => scrollAIMessagesToBottom())

  // 模拟 AI 回复
  setTimeout(() => {
    const response = aiResponses[content] ||
      `关于「${content}」，我来为你分析一下...\n\n根据当前任务的执行状态和专家输出，我的建议是：\n1. 首先关注整体进度和关键路径\n2. 优先查看置信度最高的专家意见\n3. 对比多种融合策略的优劣\n\n如果需要更详细的分析，请告诉我具体想了解哪方面。`

    aiMessages.value.push({ role: 'assistant', content: response })
    aiTyping.value = false
    nextTick(() => scrollAIMessagesToBottom())
  }, 1000 + Math.random() * 1000)
}

function scrollAIMessagesToBottom() {
  if (aiMessagesRef.value) {
    aiMessagesRef.value.scrollTop = aiMessagesRef.value.scrollHeight
  }
}

// ===== 创建任务 =====
const newTaskForm = ref({
  name: '',
  description: '',
  fusion_strategy: 'confidence_weighted',
  expert_ids: []
})

const currentStrategyHint = computed(() => {
  const s = fusionStrategyOptions.find(s => s.key === newTaskForm.value.fusion_strategy)
  return s ? s.hint : ''
})

function createTask() {
  if (!newTaskForm.value.name.trim()) {
    ElMessage.warning('请输入任务名称')
    return
  }
  if (!newTaskForm.value.description.trim()) {
    ElMessage.warning('请输入问题描述')
    return
  }

  creatingTask.value = true

  // 模拟创建过程
  setTimeout(() => {
    const newTask = {
      id: 'task-' + Date.now(),
      name: newTaskForm.value.name,
      description: newTaskForm.value.description,
      status: 'planning',
      progress: 0,
      fusion_strategy: newTaskForm.value.fusion_strategy,
      expert_count: newTaskForm.value.expert_ids.length || 4,
      created_at: new Date().toLocaleString('zh-CN', { hour12: false }).replace(/\//g, '-'),
      duration: null,
      eta: '规划中'
    }
    tasks.value.unshift(newTask)
    selectedTaskId.value = newTask.id
    showCreateDialog.value = false
    creatingTask.value = false
    ElMessage.success('任务创建成功，正在规划协作方案...')

    // 模拟任务启动
    setTimeout(() => {
      const task = tasks.value.find(t => t.id === newTask.id)
      if (task) {
        task.status = 'running'
        task.progress = 10
        task.eta = '约 10 分钟'
      }
      // 启动日志流
      executionLogs.value = []
      startStreaming()
    }, 2000)

    // 重置表单
    newTaskForm.value = {
      name: '',
      description: '',
      fusion_strategy: 'confidence_weighted',
      expert_ids: []
    }
  }, 1500)
}

// ===== 任务操作 =====
function selectTask(id) {
  selectedTaskId.value = id
  const task = tasks.value.find(t => t.id === id)
  if (task && task.status === 'running') {
    executionLogs.value = []
    startStreaming()
  } else {
    stopStreaming()
    // 已完成的任务加载历史日志
    if (task && task.status === 'completed') {
      loadMockLogs()
    } else {
      executionLogs.value = []
    }
  }
  nextTick(() => {
    drawDAG()
  })
}

function loadMockLogs() {
  executionLogs.value = []
  for (let i = 0; i < 20; i++) {
    const log = generateLog()
    const mins = Math.floor(i / 3)
    const secs = (i * 7) % 60
    log.time = `10:${String(30 + mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`
    executionLogs.value.push(log)
  }
}

function restartTask() {
  if (!selectedTask.value) return
  const task = tasks.value.find(t => t.id === selectedTaskId.value)
  if (task) {
    task.status = 'running'
    task.progress = 0
    task.eta = '约 10 分钟'
    executionLogs.value = []
    startStreaming()
    ElMessage.success('任务已重新启动')
  }
}

function formatTime(timeStr) {
  if (!timeStr) return ''
  // 简化显示
  return timeStr.split(' ')[0]?.slice(5) || timeStr
}

// ===== 生命周期 =====
let resizeObs = null

onMounted(() => {
  // 默认选中第一个任务
  if (tasks.value.length > 0) {
    selectedTaskId.value = tasks.value[0].id
    if (tasks.value[0].status === 'running') {
      startStreaming()
    } else if (tasks.value[0].status === 'completed') {
      loadMockLogs()
    }
  }

  nextTick(() => {
    drawDAG()
    try {
      if (window.ResizeObserver && dagWrapRef.value) {
        resizeObs = new ResizeObserver(() => drawDAG())
        resizeObs.observe(dagWrapRef.value)
      }
    } catch (e) {
      console.warn('ResizeObserver not supported')
    }
  })
})

onBeforeUnmount(() => {
  stopStreaming()
  if (resizeObs) {
    try { resizeObs.disconnect() } catch (e) {}
  }
})

// 监听任务切换重绘 DAG
watch(selectedTaskId, () => {
  nextTick(() => drawDAG())
})
</script>

<style scoped>
.alliance-task-view {
  height: 100%;
  width: 100%;
  position: relative;
  overflow: hidden;
}

.phi-layout {
  display: flex;
  gap: 12px;
  height: 100%;
  padding: 12px;
  box-sizing: border-box;
}

.col {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
}

.col-left {
  width: 320px;
  flex-shrink: 0;
}

.col-main {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
}

.card {
  background: #fff;
  border-radius: 12px;
  border: 1px solid #e2e8f0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.card-header {
  padding: 12px 14px;
  border-bottom: 1px solid #f1f5f9;
  display: flex;
  align-items: center;
  gap: 8px;
}

.card-header.between {
  justify-content: space-between;
}

.card-title {
  font-size: 13px;
  font-weight: 700;
  color: #0f172a;
}

.card-sub {
  font-size: 11px;
  color: #94a3b8;
}

/* ===== 左栏：创建任务 ===== */
.create-task-card {
  padding: 12px;
  flex-shrink: 0;
  gap: 10px;
}

.create-btn {
  width: 100%;
  height: 40px;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  border: none;
  font-weight: 600;
}

.create-btn:hover {
  opacity: 0.92;
}

.task-stats {
  display: flex;
  justify-content: space-around;
  padding-top: 4px;
}

.ts-item {
  text-align: center;
}

.ts-num {
  display: block;
  font-size: 18px;
  font-weight: 800;
  color: #0f172a;
  line-height: 1.2;
}

.ts-num.running { color: #6366f1; }
.ts-num.completed { color: #10b981; }

.ts-label {
  font-size: 11px;
  color: #94a3b8;
}

/* ===== 筛选卡片 ===== */
.filter-card {
  padding: 10px;
  flex-shrink: 0;
  gap: 8px;
}

.filter-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.filter-tab {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  font-size: 11px;
  color: #64748b;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
}

.filter-tab:hover {
  background: #f8fafc;
  color: #334155;
}

.filter-tab.active {
  background: #eef2ff;
  color: #4f46e5;
  font-weight: 600;
}

.tab-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.tab-count {
  font-size: 10px;
  opacity: 0.7;
}

.search-row {
  padding: 0 2px;
}

/* ===== 任务列表 ===== */
.task-list-card {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.task-scroll {
  flex: 1;
  overflow: hidden;
  min-height: 0;
  padding: 8px;
}

.empty-tasks {
  padding: 30px 0;
}

.task-card {
  padding: 10px 12px;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  margin-bottom: 8px;
  cursor: pointer;
  transition: all 0.15s;
  background: #fff;
}

.task-card:hover {
  border-color: #c7d2fe;
  box-shadow: 0 4px 12px -6px rgba(99, 102, 241, 0.2);
}

.task-card.active {
  border-color: #6366f1;
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.04), rgba(14, 165, 233, 0.03));
  box-shadow: 0 4px 12px -4px rgba(99, 102, 241, 0.25);
}

.task-card-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.task-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.task-name {
  font-size: 13px;
  font-weight: 700;
  color: #0f172a;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.task-desc {
  font-size: 11px;
  color: #64748b;
  line-height: 1.4;
  margin-bottom: 8px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.task-meta {
  display: flex;
  gap: 10px;
  margin-bottom: 6px;
  font-size: 11px;
  color: #94a3b8;
}

.task-meta .el-icon {
  font-size: 11px;
  margin-right: 2px;
}

.task-strategy, .task-experts {
  display: flex;
  align-items: center;
}

.task-progress {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.tp-bar {
  flex: 1;
  height: 4px;
  background: #e2e8f0;
  border-radius: 2px;
  overflow: hidden;
}

.tp-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.3s ease;
}

.tp-text {
  font-size: 10px;
  color: #64748b;
  width: 30px;
  text-align: right;
  flex-shrink: 0;
}

.task-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 10px;
  color: #94a3b8;
}

/* ===== 右栏：任务头部 ===== */
.task-header-card {
  padding: 16px 18px;
  flex-shrink: 0;
  gap: 12px;
}

.task-header-top {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 12px;
}

.task-title-area {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.task-title {
  margin: 0;
  font-size: 18px;
  font-weight: 800;
  color: #0f172a;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}

.task-actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.task-header-desc {
  font-size: 13px;
  color: #64748b;
  line-height: 1.6;
}

.task-progress-large {
  background: linear-gradient(135deg, #f8fafc, #f1f5f9);
  border-radius: 10px;
  padding: 12px 16px;
}

.tpl-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.tpl-label {
  font-size: 12px;
  color: #64748b;
  font-weight: 600;
}

.tpl-value {
  font-size: 20px;
  font-weight: 800;
  color: #6366f1;
}

.tpl-bar {
  height: 8px;
  background: #e2e8f0;
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 10px;
}

.tpl-fill {
  height: 100%;
  border-radius: 4px;
  background: linear-gradient(90deg, #6366f1, #0ea5e9);
  transition: width 0.5s ease;
}

.tpl-stats {
  display: flex;
  gap: 16px;
  font-size: 11px;
  color: #64748b;
}

.tpl-stats .el-icon {
  margin-right: 4px;
  font-size: 12px;
}

.tpl-stats span {
  display: flex;
  align-items: center;
}

/* ===== DAG 图 ===== */
.dag-card {
  flex-shrink: 0;
}

.dag-legend {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: #64748b;
}

.legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.dag-canvas-wrap {
  height: 260px;
  background:
    radial-gradient(400px 150px at 30% 20%, rgba(99, 102, 241, 0.05), transparent),
    radial-gradient(400px 150px at 70% 80%, rgba(14, 165, 233, 0.05), transparent),
    linear-gradient(135deg, #fafbfc, #f8fafc);
  position: relative;
}

.dag-canvas {
  width: 100%;
  height: 100%;
  display: block;
}

/* ===== 底部双栏 ===== */
.bottom-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  flex: 1;
  min-height: 0;
}

.logs-card, .fusion-card {
  min-height: 0;
  display: flex;
  flex-direction: column;
}

/* ===== 执行日志 ===== */
.log-actions {
  display: flex;
  gap: 4px;
}

.logs-container {
  flex: 1;
  overflow-y: auto;
  padding: 10px 12px;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 11.5px;
  line-height: 1.7;
  background: #f8fafc;
  min-height: 200px;
  max-height: 320px;
}

.log-item {
  margin-bottom: 2px;
  word-break: break-all;
}

.log-time {
  color: #94a3b8;
  margin-right: 6px;
}

.log-expert {
  font-weight: 600;
  margin-right: 6px;
}

.log-message {
  color: #334155;
}

.log-info .log-message { color: #334155; }
.log-warn .log-message { color: #d97706; }
.log-error .log-message { color: #dc2626; }
.log-success .log-message { color: #059669; font-weight: 600; }

.log-typing {
  display: flex;
  gap: 3px;
  padding: 4px 0;
}

.typing-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: #cbd5e1;
  animation: typingBounce 1.2s infinite;
}

.typing-dot:nth-child(2) { animation-delay: 0.15s; }
.typing-dot:nth-child(3) { animation-delay: 0.3s; }

@keyframes typingBounce {
  0%, 60%, 100% { transform: translateY(0); opacity: 0.4; }
  30% { transform: translateY(-4px); opacity: 1; }
}

/* ===== 融合结果 ===== */
.fusion-cards {
  flex: 1;
  overflow-y: auto;
  padding: 10px 12px;
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
  min-height: 200px;
  max-height: 320px;
}

.fusion-card-item {
  padding: 10px 12px;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  background: #fff;
  transition: all 0.15s;
  position: relative;
}

.fusion-card-item:hover {
  border-color: #c7d2fe;
  box-shadow: 0 4px 12px -6px rgba(99, 102, 241, 0.2);
}

.fusion-card-item.best-score {
  border-color: #10b981;
  background: linear-gradient(135deg, rgba(16, 185, 129, 0.04), rgba(14, 165, 233, 0.03));
}

.fc-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
}

.fc-icon {
  font-size: 16px;
}

.fc-name {
  font-size: 12px;
  font-weight: 700;
  color: #0f172a;
  flex: 1;
}

.fc-score {
  display: flex;
  align-items: baseline;
  gap: 2px;
  margin-bottom: 4px;
}

.fc-score-num {
  font-size: 22px;
  font-weight: 800;
  color: #0f172a;
  line-height: 1;
}

.fc-score-unit {
  font-size: 11px;
  color: #94a3b8;
}

.fc-bar {
  height: 4px;
  background: #e2e8f0;
  border-radius: 2px;
  overflow: hidden;
  margin-bottom: 8px;
}

.fc-bar-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.5s ease;
}

.fc-metrics {
  display: flex;
  gap: 10px;
  margin-bottom: 6px;
}

.fc-metric {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.fcm-label {
  font-size: 10px;
  color: #94a3b8;
}

.fcm-value {
  font-size: 11px;
  font-weight: 600;
  color: #475569;
}

.fc-summary {
  font-size: 10.5px;
  color: #64748b;
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* 对比视图 */
.fusion-compare {
  flex: 1;
  overflow-y: auto;
  padding: 10px 12px;
  min-height: 200px;
  max-height: 320px;
}

.compare-row {
  display: grid;
  grid-template-columns: 110px 50px 1fr;
  gap: 10px;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid #f1f5f9;
}

.compare-row:last-child {
  border-bottom: none;
}

.compare-header {
  font-size: 10px;
  font-weight: 700;
  color: #94a3b8;
  text-transform: uppercase;
  padding-bottom: 6px;
  border-bottom: 2px solid #e2e8f0;
}

.compare-name {
  font-size: 12px;
  font-weight: 600;
  color: #334155;
  display: flex;
  align-items: center;
  gap: 4px;
}

.cn-icon {
  font-size: 14px;
}

.compare-score {
  font-size: 16px;
  font-weight: 800;
  text-align: center;
}

.compare-bar-label {
  font-size: 10px;
  color: #94a3b8;
  display: flex;
  justify-content: space-around;
}

.compare-bars {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.cb-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.cb-bar {
  flex: 1;
  height: 6px;
  background: #e2e8f0;
  border-radius: 3px;
  overflow: hidden;
}

.cb-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.5s ease;
}

.cb-label {
  font-size: 10px;
  color: #64748b;
  width: 32px;
  text-align: right;
  flex-shrink: 0;
}

/* ===== 空状态 ===== */
.empty-detail {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 14px;
  text-align: center;
  padding: 40px 20px;
  background:
    radial-gradient(500px 250px at 50% 30%, rgba(99, 102, 241, 0.06), transparent),
    radial-gradient(500px 250px at 50% 70%, rgba(16, 185, 129, 0.06), transparent);
  border-radius: 12px;
  border: 1px dashed #e2e8f0;
  min-height: 400px;
}

.empty-illustration {
  margin-bottom: 4px;
}

.empty-orb {
  width: 80px;
  height: 80px;
  animation: orbSpin 12s linear infinite;
}

.eo-svg {
  width: 100%;
  height: 100%;
}

@keyframes orbSpin {
  to { transform: rotate(360deg); }
}

.empty-title {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: #1e293b;
}

.empty-sub {
  margin: 0;
  font-size: 13px;
  color: #94a3b8;
  max-width: 360px;
  line-height: 1.6;
}

/* ===== AI 助手悬浮按钮 ===== */
.ai-assistant-float {
  position: fixed;
  right: 24px;
  bottom: 32px;
  z-index: 1000;
  cursor: pointer;
}

.aif-button {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  display: grid;
  place-items: center;
  color: #fff;
  box-shadow: 0 8px 24px -8px rgba(99, 102, 241, 0.5);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.aif-button:hover {
  transform: scale(1.08);
  box-shadow: 0 12px 32px -8px rgba(99, 102, 241, 0.6);
}

.aif-button.active {
  transform: rotate(90deg);
  background: linear-gradient(135deg, #ef4444, #f97316);
}

.aif-icon {
  font-size: 24px;
}

.aif-tooltip {
  position: absolute;
  right: 64px;
  top: 50%;
  transform: translateY(-50%);
  background: #1e293b;
  color: #fff;
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 12px;
  white-space: nowrap;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.2s;
}

.ai-assistant-float:hover .aif-tooltip {
  opacity: 1;
}

.aif-tooltip::after {
  content: '';
  position: absolute;
  right: -4px;
  top: 50%;
  transform: translateY(-50%) rotate(45deg);
  width: 8px;
  height: 8px;
  background: #1e293b;
}

/* ===== AI 助手侧边面板 ===== */
.ai-panel {
  position: fixed;
  right: 0;
  top: 0;
  bottom: 0;
  width: 380px;
  background: #fff;
  box-shadow: -8px 0 24px -12px rgba(0, 0, 0, 0.15);
  z-index: 1001;
  display: flex;
  flex-direction: column;
  border-left: 1px solid #e2e8f0;
}

.ai-panel-mask {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.3);
  z-index: 1000;
  backdrop-filter: blur(2px);
}

.slide-right-enter-active,
.slide-right-leave-active {
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-right-enter-from,
.slide-right-leave-to {
  transform: translateX(100%);
}

.ai-panel-header {
  padding: 14px 16px;
  border-bottom: 1px solid #f1f5f9;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.04), rgba(14, 165, 233, 0.03));
}

.ai-panel-title {
  display: flex;
  align-items: center;
  gap: 10px;
}

.ai-panel-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 20px;
}

.ai-panel-name {
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
}

.ai-panel-status {
  font-size: 11px;
  color: #64748b;
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 2px;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #94a3b8;
}

.status-dot.online {
  background: #10b981;
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.2);
}

.ai-panel-messages {
  flex: 1;
  overflow-y: auto;
  padding: 14px 12px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: #f8fafc;
}

.ai-msg {
  display: flex;
  gap: 8px;
  max-width: 85%;
}

.ai-msg-bot {
  align-self: flex-start;
}

.ai-msg-user {
  align-self: flex-end;
  flex-direction: row-reverse;
}

.msg-avatar {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 14px;
  flex-shrink: 0;
}

.msg-avatar.bot {
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
}

.msg-avatar.user {
  background: #e2e8f0;
  color: #475569;
}

.msg-content {
  min-width: 0;
}

.msg-bubble {
  padding: 8px 12px;
  border-radius: 12px;
  font-size: 13px;
  line-height: 1.6;
  word-break: break-word;
  white-space: pre-wrap;
}

.msg-bubble.bot {
  background: #fff;
  color: #1e293b;
  border: 1px solid #e2e8f0;
  border-top-left-radius: 4px;
}

.msg-bubble.user {
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
  border-top-right-radius: 4px;
}

.typing-bubble {
  display: flex;
  gap: 4px;
  padding: 10px 14px;
}

.typing-bubble .typing-dot {
  width: 6px;
  height: 6px;
  background: #94a3b8;
}

.ai-panel-quick {
  padding: 10px 12px;
  border-top: 1px solid #f1f5f9;
  background: #fff;
}

.quick-title {
  font-size: 11px;
  font-weight: 700;
  color: #94a3b8;
  margin-bottom: 8px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.quick-btns {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.quick-btns .el-button {
  font-size: 11px;
  padding: 4px 10px;
  height: 26px;
}

.ai-panel-input {
  padding: 10px 12px;
  border-top: 1px solid #f1f5f9;
  background: #fff;
}

.input-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 6px;
}

.input-hint {
  font-size: 10px;
  color: #94a3b8;
}

/* ===== 创建任务对话框 ===== */
.strategy-radio-group {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  width: 100%;
}

.strategy-radio-group :deep(.el-radio-button__inner) {
  width: 100%;
  text-align: center;
  padding: 8px 10px;
}

.sr-icon {
  font-size: 16px;
  margin-right: 4px;
}

.sr-name {
  font-size: 12px;
}

.strategy-hint {
  font-size: 11px;
  color: #64748b;
  margin-top: 8px;
  padding: 8px 10px;
  background: #f8fafc;
  border-radius: 6px;
  line-height: 1.5;
}

.expert-hint {
  font-size: 11px;
  color: #94a3b8;
  margin-top: 6px;
}

/* ===== 响应式 ===== */
@media (max-width: 1280px) {
  .col-left {
    width: 280px;
  }
  .bottom-row {
    grid-template-columns: 1fr;
  }
  .fusion-cards {
    grid-template-columns: repeat(3, 1fr);
  }
}

@media (max-width: 1024px) {
  .col-left {
    width: 260px;
  }
  .fusion-cards {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
