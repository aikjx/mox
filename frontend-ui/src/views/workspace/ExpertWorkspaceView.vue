<!--
  专家联盟统一工作台 · Expert Alliance Unified Workspace (Container)
  ======================================================
  架构原则：前端融合 · 后端模块化 · 组件化拆分
  本组件为路由入口容器，负责：
  - 共享状态管理（项目、专家、会话、消息等）
  - 数据获取调度
  - 跨面板事件协调
  - 生命周期管理
  子组件位于 ./panels/ 目录
  核心逻辑通过 composables 复用：useWhiteboard / useGraphCanvas / useTaskOrchestration
-->
<template>
  <div class="expert-workspace">
    <!-- 顶部全局工具栏 -->
    <WorkspaceHeader
      v-model:currentProject="currentProject"
      v-model:globalSearch="globalSearch"
      :project-options="projectOptions"
      :active-mode="activeMode"
      :mode-transitioning="modeTransitioning"
      :work-modes="workModes"
      :notif-count="notifCount"
      :has-notifications="hasNotifications"
      @project-change="onProjectChange"
      @switch-mode="switchWorkMode"
      @global-search="doGlobalSearch"
      @open-ai="openAIAssistant"
    />

    <!-- KPI 指标卡 -->
    <KpiPanel :kpi-cards="kpiCards" @kpi-click="onKpiClick" />

    <!-- 主工作区 · 三栏布局 -->
    <div class="ws-main">
      <!-- 左栏：专家联盟面板 -->
      <ExpertPanel
        :collapsed="leftCollapsed"
        :experts="experts"
        :experts-loading="expertsLoading"
        :active-expert="activeExpert"
        :selected-expert-ids="selectedExpertIds"
        :sessions="sessions"
        :sessions-loading="sessionsLoading"
        :active-session="activeSession"
        :active-mode="activeMode"
        @toggle-collapse="leftCollapsed = !leftCollapsed"
        @expert-click="handleExpertClick"
        @select-session="selectSession"
        @new-collaboration="newCollaboration"
        @open-debate="openDebateDialog"
        @trigger-orchestration="triggerOrchestration"
        @trigger-voting="triggerVoting"
        @open-multi-consult="openMultiConsultDialog"
        @open-register="showRegisterDialog = true"
        @open-smart-route="openSmartRouteDialog"
        @expand-and-select="(exp) => { leftCollapsed = false; selectExpert(exp) }"
        @expand-and-new-session="() => { leftCollapsed = false; newCollaboration() }"
      />

      <!-- 中栏：图谱画布 + 协作讨论 -->
      <main class="ws-center">
        <!-- 图谱画布（非编排模式时显示） -->
        <div v-show="activeMode !== 'orchestration'">
          <GraphCanvasPanel
            v-model:activeCanvasTool="activeCanvasTool"
            :current-layout="currentLayout"
            :selected-node="selectedNode"
            :graph-loading="graphLoading"
            :graph-analyzing="graphAnalyzing"
            :graph-nodes="graphNodes"
            :graph-edges="graphEdges"
            :graph-stats="graphStats"
            :svg-view-box="svgViewBox"
            :viewport="viewport"
            @zoom-in="zoomIn"
            @zoom-out="zoomOut"
            @fit-view="fitView"
            @switch-layout="switchLayout"
            @run-graph-algo="runGraphAlgo"
            @canvas-mousedown="onCanvasMouseDown"
            @canvas-mousemove="onCanvasMouseMove"
            @canvas-mouseup="onCanvasMouseUp"
            @canvas-wheel="onCanvasWheel"
            @select-node="selectNode"
            @node-mousedown="onNodeMouseDown"
            @clear-selected-node="selectedNode = null"
            @view-node-docs="viewNodeDocs"
            @ask-experts-about="askExpertsAbout"
          />
        </div>

        <!-- 任务编排模式视图 -->
        <TaskOrchestrationPanel
          v-show="activeMode === 'orchestration'"
          :task-orchestration="taskOrchestration"
          @update:task-orchestration="(v) => Object.assign(taskOrchestration, v)"
          :decomposing="decomposing"
          :orch-is-running="orchIsRunning"
          :active-subtask-id="activeSubtaskId"
          v-model:timeline-view="timelineView"
          :dragging-task-id="draggingTaskId"
          :drag-over-task-id="dragOverTaskId"
          :expert-drag-over-task-id="expertDragOverTaskId"
          :experts="experts"
          :gantt-slot-minutes="ganttSlotMinutes"
          @reset-all-tasks="resetAllTasks"
          @start-task-execution="startTaskExecution"
          @decompose-task="decomposeTask"
          @add-subtask-manually="addSubtaskManually"
          @collapse-all-subtasks="collapseAllSubtasks"
          @task-dragstart="onTaskDragStart"
          @task-dragend="onTaskDragEnd"
          @task-dragover="onTaskDragOver"
          @task-drop="onTaskDrop"
          @select-subtask="selectSubtask"
          @edit-subtask="editSubtask"
          @delete-subtask="deleteSubtask"
          @toggle-subtask-expand="toggleSubtaskExpand"
          @open-assign-dialog="openAssignDialog"
          @expert-dragstart="onExpertDragStart"
          @expert-dragend="onExpertDragEnd"
          @expert-dragover-task="onExpertDragOverTask"
          @expert-dragleave-task="onExpertDragLeaveTask"
          @expert-drop-on-task="onExpertDropOnTask"
          @unassign-expert="unassignExpert"
          @auto-assign-experts="autoAssignExperts"
        />

        <!-- 底部协作对话栏 -->
        <CollaborationPanel
          :expanded="collabExpanded"
          :alliance-running="allianceRunning"
          :mode-transitioning="modeTransitioning"
          :active-session="activeSession"
          :current-phase-label="currentPhaseLabel"
          :collab-messages="collabMessages"
          :typing-experts="typingExperts"
          :project-phases="projectPhases"
          :current-project-phase="currentProjectPhase"
          :collab-tabs="collabTabs"
          :active-collab-tab="activeCollabTab"
          :collab-members="collabMembers"
          :shared-files="sharedFiles"
          :history-panel-open="historyPanelOpen"
          :history-events="historyEvents"
          v-model:collab-input="collabInput"
          v-model:collab-mode="collabMode"
          :active-wb-tool="activeWbTool"
          :active-wb-color="activeWbColor"
          :wb-notes="wbNotes"
          :wb-texts="wbTexts"
          :wb-lines="wbLines"
          :wb-draw-paths="wbDrawPaths"
          :wb-current-path="wbCurrentPath"
          :wb-view-box="wbViewBox"
          @toggle-expand="collabExpanded = !collabExpanded"
          @toggle-history="historyPanelOpen = !historyPanelOpen"
          @jump-to-phase="jumpToPhase"
          @update:activeCollabTab="activeCollabTab = $event"
          @preview-file="previewFile"
          @download-file="downloadFile"
          @file-uploaded="(f) => sharedFiles.unshift(f)"
          @insert-node-ref="insertNodeRef"
          @send-to-whiteboard="sendToWhiteboard"
          @collab-mode-change="onCollabModeChange"
          @send-msg="sendCollabMsg"
          @stop-alliance="stopAlliance"
          @jump-to-history="jumpToHistory"
          @select-wb-tool="selectWbTool"
          @update:activeWbColor="activeWbColor = $event"
          @clear-whiteboard="clearWhiteboard"
          @wb-mousedown="onWbMouseDown"
          @wb-mousemove="onWbMouseMove"
          @wb-mouseup="onWbMouseUp"
          @start-drag-note="startDragNote"
          @delete-wb-note="deleteWbNote"
          @update-note-content="updateNoteContent"
          @start-drag-text="startDragText"
          @delete-wb-text="deleteWbText"
          @update-text-content="updateTextContent"
          @save-whiteboard="saveWhiteboard(activeSession)"
        />
      </main>

      <!-- 右栏：知识库云盘面板 -->
      <KnowledgeBasePanel
        :collapsed="rightCollapsed"
        :active-kb-tab="activeKbTab"
        :categories="categories"
        :documents="documents"
        :popular-tags="popularTags"
        :doc-versions="docVersions"
        :active-doc="activeDoc"
        :active-category="activeCategory"
        :expanded-categories="expandedCategories"
        :docs-loading="docsLoading"
        @toggle-collapse="rightCollapsed = !rightCollapsed"
        @switch-kb-tab="switchKbTab"
        @search-kb="searchKb"
        @select-category="selectCategory"
        @open-doc="openDoc"
        @filter-by-tag="filterByTag"
        @create-doc="createDoc"
        @expand-and-switch="(tab) => { rightCollapsed = false; activeKbTab = tab }"
      />
    </div>

    <!-- AI 助手浮窗 -->
    <AIAssistantPanel
      :visible="aiAssistantOpen"
      :capabilities="allianceCapabilitiesList"
      @close="aiAssistantOpen = false"
      @suggestion="aiSuggestion"
    />

    <!-- 注册专家对话框 -->
    <RegisterExpertDialog v-model="showRegisterDialog" @registered="onExpertRegistered" />

    <!-- 辩论对话框 -->
    <DebateDialog
      v-model:visible="showDebateDialog"
      v-model:topic="debateConfig.topic"
      v-model:mode="debateConfig.mode"
      v-model:rounds="debateConfig.rounds"
      :selected-expert-ids="debateConfig.selectedExpertIds"
      :status="debateStatus"
      :submitting="debateSubmitting"
      :experts="experts"
      @close="showDebateDialog = false"
      @start="startDebate"
      @toggle-expert="toggleDebateExpert"
    />

    <!-- 多专家咨询对话框 -->
    <MultiConsultDialog
      v-model:visible="showMultiConsultDialog"
      v-model:question="multiConsultConfig.question"
      v-model:mode="multiConsultConfig.mode"
      v-model:compare-view="multiConsultCompareView"
      :selected-expert-ids="multiConsultConfig.selectedExpertIds"
      :results="multiConsultResults"
      :submitting="multiConsultSubmitting"
      :experts="experts"
      @close="showMultiConsultDialog = false"
      @start="startMultiConsult"
      @toggle-expert="toggleMultiConsultExpert"
    />

    <!-- 智能匹配对话框 -->
    <SmartRouteDialog
      v-model:visible="showSmartRouteDialog"
      v-model:question="smartRouteQuestion"
      v-model:max-experts="smartRouteMaxExperts"
      :loading="smartRoutingLoading"
      :result="smartRouteResult"
      @close="showSmartRouteDialog = false"
      @do-route="doSmartRoute"
      @select-expert="selectRoutedExpert"
      @select-all="selectAllRoutedExperts"
    />

    <!-- 全局通知 -->
    <el-notification v-for="notif in notifications" :key="notif.id"
      :title="notif.title" :message="notif.message" :type="notif.type || 'info'"
      :duration="3000" @close="removeNotification(notif.id)"
    />
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { EXPERT_TYPES } from '@/constants/expert.constants'
import { runAllianceFullSSE, getAllianceCapabilities } from '@/api/alliance'
import {
  getExperts, getExpertSessions, expertDebate,
  multiExpertConsult, routeExperts
} from '@/api/experts.api.js'
import RegisterExpertDialog from '@/components/expert/RegisterExpertDialog.vue'
import {
  kbListDocuments, kbGetCategories, kbGetTags,
  kbSearch, kbGetVersions
} from '@/api/kb.api.js'
import { getProjects } from '@/api/projects.api.js'
import '@/styles/workspace.css'

// 子组件导入
import WorkspaceHeader from './panels/WorkspaceHeader.vue'
import KpiPanel from './panels/KpiPanel.vue'
import ExpertPanel from './panels/ExpertPanel.vue'
import GraphCanvasPanel from './panels/GraphCanvasPanel.vue'
import TaskOrchestrationPanel from './panels/TaskOrchestrationPanel.vue'
import CollaborationPanel from './panels/CollaborationPanel.vue'
import KnowledgeBasePanel from './panels/KnowledgeBasePanel.vue'
import AIAssistantPanel from './panels/AIAssistantPanel.vue'
import DebateDialog from './panels/DebateDialog.vue'
import MultiConsultDialog from './panels/MultiConsultDialog.vue'
import SmartRouteDialog from './panels/SmartRouteDialog.vue'

// Composables 导入
import { useWhiteboard } from '@/composables/workspace/useWhiteboard.js'
import { useGraphCanvas } from '@/composables/workspace/useGraphCanvas.js'
import { useTaskOrchestration } from '@/composables/workspace/useTaskOrchestration.js'
import { useAlliance } from '@/composables/workspace/useAlliance.js'
import {
  getMockExperts, getMockSessions, getMockDocs,
  getMockCategories, getMockTags, getMockVersions
} from './mockData.js'

// ========== 布局状态 ==========
const leftCollapsed = ref(false)
const rightCollapsed = ref(false)
const collabExpanded = ref(true)
const aiAssistantOpen = ref(false)
const hasNotifications = ref(true)
const notifCount = ref(3)
const historyPanelOpen = ref(false)

// ========== KPI 指标卡 ==========
const kpiCards = ref([
  { key: 'experts', icon: '👥', value: 12, label: '在线专家', trend: 8, gradient: 'linear-gradient(135deg, #7c3aed, #06b6d4)' },
  { key: 'sessions', icon: '💬', value: 28, label: '协作会话', trend: 15, gradient: 'linear-gradient(135deg, #ec4899, #8b5cf6)' },
  { key: 'docs', icon: '📄', value: 156, label: '知识文档', trend: 5, gradient: 'linear-gradient(135deg, #10b981, #14b8a6)' },
  { key: 'tasks', icon: '🎯', value: 7, label: '进行中任务', trend: -2, gradient: 'linear-gradient(135deg, #f59e0b, #ef4444)' }
])

function onKpiClick(key) {
  if (key === 'experts') leftCollapsed.value = false
  else if (key === 'docs') { rightCollapsed.value = false; activeKbTab.value = 'docs' }
  else if (key === 'sessions') { collabExpanded.value = true; activeCollabTab.value = 'discussion' }
  else if (key === 'tasks') activeMode.value = 'orchestration'
}

// ========== 工作模式 ==========
const savedMode = localStorage.getItem('expert_workspace_mode')
const activeMode = ref(savedMode || 'collaboration')
const modeTransitioning = ref(false)
const workModes = [
  { key: 'exploration', label: '知识探索', iconComp: 'Search', gradient: 'linear-gradient(135deg, #06b6d4, #3b82f6)' },
  { key: 'collaboration', label: '专家协作', iconComp: 'UserFilled', gradient: 'linear-gradient(135deg, #7c3aed, #06b6d4)' },
  { key: 'orchestration', label: '任务编排', iconComp: 'SetUp', gradient: 'linear-gradient(135deg, #f59e0b, #ef4444)' },
  { key: 'analysis', label: '深度分析', iconComp: 'DataAnalysis', gradient: 'linear-gradient(135deg, #10b981, #14b8a6)' }
]

function switchWorkMode(mode) {
  if (activeMode.value === mode) return
  modeTransitioning.value = true
  activeMode.value = mode
  localStorage.setItem('expert_workspace_mode', mode)
  if (mode === 'exploration') { leftCollapsed.value = true; rightCollapsed.value = false }
  else if (mode === 'collaboration') { leftCollapsed.value = false; collabExpanded.value = true }
  else if (mode === 'orchestration') { leftCollapsed.value = false; rightCollapsed.value = false }
  else if (mode === 'analysis') { leftCollapsed.value = true; rightCollapsed.value = true }
  setTimeout(() => { modeTransitioning.value = false }, 400)
  addHistoryEvent('mode', `切换到${workModes.find(m => m.key === mode)?.label || ''}模式`, '工作模式已切换')
}

// ========== 项目 ==========
const currentProject = ref('xuanji')
const globalSearch = ref('')
const projectOptions = ref([
  { id: 'xuanji', name: '璇玑知识工程' },
  { id: 'mox', name: 'MOX 平台架构' },
  { id: 'ailab', name: 'AI 算法实验室' }
])

async function loadProjects() {
  try {
    const data = await getProjects()
    const list = Array.isArray(data) ? data : (data?.list || data?.items || data?.projects || [])
    if (list.length) {
      projectOptions.value = list.map((p) => ({ id: p.id, name: p.name || p.title || '未命名项目' }))
      if (!projectOptions.value.find((p) => p.id === currentProject.value)) currentProject.value = projectOptions.value[0].id
    }
  } catch (e) { console.warn('[ExpertWorkspace] 加载项目列表失败:', e?.message) }
}

function onProjectChange() { loadExperts(); loadSessions(); loadGraphData(); loadDocuments() }
function doGlobalSearch() {
  if (!globalSearch.value.trim()) return
  expertSearch.value = globalSearch.value
  kbSearchQuery.value = globalSearch.value
  ElMessage.info(`正在全局搜索「${globalSearch.value}」…`)
}

// ========== 专家数据 ==========
const experts = ref([])
const expertsLoading = ref(false)
const expertSearch = ref('')
const activeExpert = ref(null)
const selectedExpertIds = ref([])
const notifications = ref([])

// 专家工具函数
function expertColor(type) {
  const colors = { algorithm: '#6366f1', architecture: '#6366f1', data: '#10b981', ai: '#ec4899', workflow: '#f59e0b', graph: '#06b6d4', security: '#ef4444', performance: '#f97316', monitor: '#14b8a6', market: '#8b5cf6', mcp: '#0ea5e9', automation: '#84cc16', requirement: '#f43f5e', fusion: '#a855f7', operator: '#64748b', custom: '#64748b' }
  return colors[type] || '#6366f1'
}
function expertEmoji(type) {
  const emojis = { algorithm: '🧮', architecture: '🏗️', data: '🔗', ai: '🤖', workflow: '⚡', graph: '🕸️', security: '🔒', performance: '🚀', monitor: '📊', market: '📈', mcp: '🔌', automation: '🤖', requirement: '📋', fusion: '🔀', operator: '⚙️', custom: '👤' }
  return emojis[type] || '👤'
}
function selectExpert(expert) { activeExpert.value = expert }
function handleExpertClick(expert) {
  selectExpert(expert)
  const idx = selectedExpertIds.value.indexOf(expert.id)
  if (idx >= 0) selectedExpertIds.value.splice(idx, 1)
  else selectedExpertIds.value.push(expert.id)
}

async function loadExperts() {
  expertsLoading.value = true
  try {
    const res = await getExperts({ project_id: currentProject.value, status: 'active' })
    if (res && Array.isArray(res.data)) experts.value = res.data
    else if (res && Array.isArray(res)) experts.value = res
    else experts.value = getMockExperts()
  } catch (e) { console.warn('[workspace] 加载专家列表失败:', e); experts.value = getMockExperts() }
  finally { expertsLoading.value = false }
}

// ========== 协作会话 ==========
const sessions = ref([])
const sessionsLoading = ref(false)
const activeSession = ref(null)

async function loadSessions() {
  sessionsLoading.value = true
  try {
    const res = await getExpertSessions({ project_id: currentProject.value, limit: 20 })
    if (res && Array.isArray(res.data)) sessions.value = res.data
    else if (res && Array.isArray(res)) sessions.value = res
    else sessions.value = getMockSessions()
  } catch (e) { console.warn('[workspace] 加载会话失败:', e); sessions.value = getMockSessions() }
  finally { sessionsLoading.value = false }
}

function selectSession(session) {
  activeSession.value = session
  collabMessages.value = [{ id: Date.now(), role: 'system', name: '系统', avatar: '📢', color: '#64748b', time: formatTime(session.updated_at), text: `已进入「${session.title}」协作会话` }]
}

function newCollaboration() {
  activeMode.value = 'collaboration'
  collabExpanded.value = true
  const newSess = { id: 'sess-' + Date.now(), title: '新协作会话', expert_count: selectedExpertIds.value.length || 0, mode: collabMode.value, created_at: Date.now(), updated_at: Date.now() }
  sessions.value.unshift(newSess)
  selectSession(newSess)
  ElMessage.success('已创建新的协作会话')
}

// ========== 协作 Tab 配置 ==========
const activeCollabTab = ref('discussion')
const collabTabs = computed(() => [
  { key: 'discussion', label: '讨论', icon: 'ChatLineSquare', badge: collabMessages.value.length },
  { key: 'whiteboard', label: '白板', icon: 'CollectionTag', badge: wbNotes.value.length + wbTexts.value.length || null },
  { key: 'files', label: '文件', icon: 'FolderOpened', badge: sharedFiles.value.length || null }
])

// ========== 协作成员 / 阶段 / 文件 ==========
const collabMembers = ref([
  { id: 'user-1', name: '我', avatar: 'U', color: 'linear-gradient(135deg, #7c3aed, #06b6d4)', status: 'active', role: 'host' },
  { id: 'exp-002', name: '陈架构', avatar: '🏗️', color: 'linear-gradient(135deg, #6366f1, #06b6d4)', status: 'active', role: 'expert' },
  { id: 'exp-004', name: '张AI', avatar: '🤖', color: 'linear-gradient(135deg, #ec4899, #8b5cf6)', status: 'active', role: 'expert' },
  { id: 'exp-006', name: '赵图谱', avatar: '🕸️', color: 'linear-gradient(135deg, #06b6d4, #3b82f6)', status: 'busy', role: 'expert' },
  { id: 'exp-001', name: '林算法', avatar: '🧮', color: 'linear-gradient(135deg, #6366f1, #8b5cf6)', status: 'active', role: 'expert' }
])
const typingExperts = ref([])
const projectPhases = ref([
  { key: 'requirement', label: '需求分析' }, { key: 'architecture', label: '架构设计' },
  { key: 'development', label: '开发实现' }, { key: 'testing', label: '测试验证' },
  { key: 'release', label: '发布上线' }
])
const currentProjectPhase = ref(1)

function jumpToPhase(idx) {
  currentProjectPhase.value = idx
  const phase = projectPhases.value[idx]
  addHistoryEvent('phase', `进入「${phase.label}」阶段`, '项目阶段已切换')
  ElMessage.info(`已切换到「${phase.label}」阶段`)
}

const sharedFiles = ref([
  { id: 'f-001', name: '架构设计文档.pdf', type: 'pdf', size: '2.4 MB', uploader: '陈架构', time: '10:30' },
  { id: 'f-002', name: '需求规格说明书.docx', type: 'doc', size: '1.8 MB', uploader: '我', time: '09:15' },
  { id: 'f-003', name: '系统架构图.png', type: 'image', size: '856 KB', uploader: '张AI', time: '昨天' },
  { id: 'f-004', name: '接口定义.xlsx', type: 'excel', size: '342 KB', uploader: '赵图谱', time: '昨天' }
])

function previewFile(file) {
  if (file.type === 'image') ElMessage.info(`正在预览图片：${file.name}`)
  else ElMessage.info(`正在打开文档：${file.name}`)
}
function downloadFile(file) { ElMessage.success(`开始下载：${file.name}`) }
function sendToWhiteboard() {
  if (collabInput.value.trim()) { addWbNote(collabInput.value.substring(0, 20), collabInput.value); ElMessage.success('已添加到白板') }
  else ElMessage.warning('请先输入内容')
}

// ========== 历史记录 ==========
const historyEvents = ref([
  { id: 'h-001', type: 'message', title: '陈架构 发送了消息', description: '关于微服务架构的建议...', time: '10:45' },
  { id: 'h-002', type: 'file', title: '上传文件', description: '架构设计文档.pdf', time: '10:30' },
  { id: 'h-003', type: 'phase', title: '进入架构设计阶段', description: '项目阶段已切换', time: '10:00' },
  { id: 'h-004', type: 'whiteboard', title: '添加便签', description: '核心架构思路', time: '09:45' },
  { id: 'h-005', type: 'mode', title: '切换到专家协作模式', description: '工作模式已切换', time: '09:30' }
])

function addHistoryEvent(type, title, description) {
  const now = new Date()
  const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
  historyEvents.value.unshift({ id: 'h-' + Date.now(), type, title, description, time })
  if (historyEvents.value.length > 50) historyEvents.value = historyEvents.value.slice(0, 50)
}

function jumpToHistory(item) {
  if (item.type === 'file') activeCollabTab.value = 'files'
  else if (item.type === 'whiteboard') activeCollabTab.value = 'whiteboard'
  ElMessage.info(`跳转到：${item.title}`)
}

// ========== 快捷键 ==========
function handleKeydown(e) {
  if (e.ctrlKey && ['1', '2', '3', '4'].includes(e.key)) {
    e.preventDefault()
    const idx = parseInt(e.key) - 1
    if (workModes[idx]) switchWorkMode(workModes[idx].key)
  }
  if (e.ctrlKey && e.key === 'k') { e.preventDefault(); doGlobalSearch() }
}

// ========== 注册专家回调 ==========
function onExpertRegistered(expertData) {
  ElMessage.success(`专家「${expertData.name || '新专家'}」注册成功`)
  if (expertData && !experts.value.find(e => e.id === expertData.id)) {
    experts.value.unshift({ id: expertData.id, name: expertData.name, type: expertData.type, status: 'active', capabilities: expertData.capabilities || [], metrics: expertData.metrics || { total_consults: 0, success_rate: 0.95 } })
  }
  loadExperts()
}

// ========== 对话框状态 ==========
const showRegisterDialog = ref(false)
const showDebateDialog = ref(false)
const showMultiConsultDialog = ref(false)
const showSmartRouteDialog = ref(false)
const debateSubmitting = ref(false)
const multiConsultSubmitting = ref(false)
const smartRoutingLoading = ref(false)

// ========== 辩论 ==========
const debateConfig = reactive({ topic: '', selectedExpertIds: [], mode: 'adversarial', rounds: 3 })
const debateStatus = ref('preparing')
const debateMessages = ref([])
const debateSummary = ref('')
const canStartDebate = computed(() => debateConfig.topic.trim() && debateConfig.selectedExpertIds.length >= 2)

function openDebateDialog() {
  debateConfig.topic = ''
  debateConfig.selectedExpertIds = [...selectedExpertIds.value]
  debateConfig.mode = 'adversarial'
  debateConfig.rounds = 3
  debateStatus.value = 'preparing'
  debateMessages.value = []
  debateSummary.value = ''
  showDebateDialog.value = true
}
function toggleDebateExpert(id) {
  const idx = debateConfig.selectedExpertIds.indexOf(id)
  if (idx >= 0) debateConfig.selectedExpertIds.splice(idx, 1)
  else debateConfig.selectedExpertIds.push(id)
}

async function startDebate() {
  if (!canStartDebate.value) return
  debateSubmitting.value = true
  debateStatus.value = 'ongoing'
  debateMessages.value = []
  debateSummary.value = ''
  try {
    const result = await expertDebate({ question: debateConfig.topic, expert_ids: debateConfig.selectedExpertIds, rounds: debateConfig.rounds, mode: debateConfig.mode })
    const history = result?.history || result?.data?.history || []
    history.forEach((round, roundIdx) => {
      const results = round.results || []
      results.forEach(r => {
        if (r.success) debateMessages.value.push({ id: Date.now() + roundIdx * 100 + Math.random(), expert: r.expert, response: r.response, round: roundIdx + 1, confidence: r.confidence })
      })
    })
    debateSummary.value = result?.final_synthesis || result?.data?.final_synthesis || ''
    debateStatus.value = 'summarized'
    appendDebateToCollab()
    ElMessage.success(`辩论完成，共 ${debateConfig.rounds} 轮`)
  } catch (e) { console.warn('[debate] 辩论 API 调用失败:', e); await simulateDebate() }
  finally { debateSubmitting.value = false }
}

async function simulateDebate() {
  const selectedExperts = experts.value.filter(e => debateConfig.selectedExpertIds.includes(e.id))
  if (selectedExperts.length < 2) { ElMessage.error('请至少选择 2 位专家'); debateStatus.value = 'preparing'; return }
  debateMessages.value = []
  for (let round = 1; round <= debateConfig.rounds; round++) {
    for (const exp of selectedExperts) {
      await new Promise(r => setTimeout(r, 300 + Math.random() * 400))
      debateMessages.value.push({ id: Date.now() + Math.random(), expert: { id: exp.id, name: exp.name, type: exp.type }, response: `【第${round}轮 · ${exp.name}】从${EXPERT_TYPES[exp.type] || '专业'}角度来看，「${debateConfig.topic.slice(0, 20)}」这个问题的核心在于${round === 1 ? '明确定义和边界' : round === 2 ? '深入分析技术方案的优劣' : '综合评估可行性和风险'}。我认为应该采用${['渐进式迭代', '模块化设计', '数据驱动决策'][round % 3]}的方法来解决。`, round, confidence: 0.85 + Math.random() * 0.12 })
    }
  }
  debateSummary.value = `## 辩论总结\n\n经过 ${debateConfig.rounds} 轮激烈讨论，${selectedExperts.map(e => e.name).join('、')} 等专家从不同角度对「${debateConfig.topic}」进行了深入分析。\n\n### 核心共识\n- 问题具有多维度复杂性，需要跨领域协作\n- 建议采用分阶段实施策略，降低风险\n- 数据驱动决策是关键成功因素\n\n### 建议方案\n综合各方观点，建议采用「${debateConfig.mode === 'adversarial' ? '混合架构' : '协同推进'}」策略，充分发挥各领域专家优势，分阶段落地实施。`
  debateStatus.value = 'summarized'
  appendDebateToCollab()
  ElMessage.warning('辩论服务暂不可用，已生成模拟辩论结果')
}

function appendDebateToCollab() {
  if (!activeSession.value) {
    const newSess = { id: 'sess-' + Date.now(), title: debateConfig.topic.slice(0, 20) + '…', expert_count: debateConfig.selectedExpertIds.length, mode: 'debate', created_at: Date.now(), updated_at: Date.now() }
    sessions.value.unshift(newSess)
    selectSession(newSess)
  }
  collabMessages.value.push({ id: Date.now(), role: 'system', name: '辩论系统', avatar: '⚔️', color: '#ef4444', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), text: `【辩论开始】主题：${debateConfig.topic}` })
  debateMessages.value.forEach(msg => {
    collabMessages.value.push({ id: Date.now() + Math.random(), role: 'expert', name: msg.expert?.name || '专家', avatar: expertEmoji(msg.expert?.type), color: expertColor(msg.expert?.type), phase: 'debate', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), text: msg.response })
  })
  if (debateSummary.value) collabMessages.value.push({ id: Date.now() + 999, role: 'assistant', name: '辩论总结', avatar: '📝', color: '#10b981', phase: 'synthesize', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), text: debateSummary.value })
  scrollMessagesToBottom()
}

// ========== 多专家咨询 ==========
const multiConsultConfig = reactive({ question: '', selectedExpertIds: [], mode: 'parallel' })
const multiConsultResults = ref([])
const multiConsultCompareView = ref(false)
const canStartMultiConsult = computed(() => multiConsultConfig.question.trim() && multiConsultConfig.selectedExpertIds.length >= 1)

function openMultiConsultDialog() {
  multiConsultConfig.question = ''
  multiConsultConfig.selectedExpertIds = [...selectedExpertIds.value]
  multiConsultConfig.mode = 'parallel'
  multiConsultResults.value = []
  multiConsultCompareView.value = false
  showMultiConsultDialog.value = true
}
function toggleMultiConsultExpert(id) {
  const idx = multiConsultConfig.selectedExpertIds.indexOf(id)
  if (idx >= 0) multiConsultConfig.selectedExpertIds.splice(idx, 1)
  else multiConsultConfig.selectedExpertIds.push(id)
}

async function startMultiConsult() {
  if (!canStartMultiConsult.value) return
  multiConsultSubmitting.value = true
  multiConsultResults.value = []
  try {
    const result = await multiExpertConsult({ question: multiConsultConfig.question, expert_ids: multiConsultConfig.selectedExpertIds, mode: multiConsultConfig.mode })
    const results = result?.results || result?.data?.results || []
    multiConsultResults.value = results.filter(r => r.success).map(r => ({ expert: r.expert, response: r.response, confidence: r.confidence, duration_ms: r.duration_ms }))
    ElMessage.success(`咨询完成，共 ${multiConsultResults.value.length} 位专家参与`)
  } catch (e) { console.warn('[multiConsult] 多专家咨询 API 失败:', e); await simulateMultiConsult() }
  finally { multiConsultSubmitting.value = false }
}

async function simulateMultiConsult() {
  const selectedExperts = experts.value.filter(e => multiConsultConfig.selectedExpertIds.includes(e.id))
  multiConsultResults.value = []
  if (multiConsultConfig.mode === 'parallel') {
    await new Promise(r => setTimeout(r, 1000))
    selectedExperts.forEach((exp, idx) => {
      multiConsultResults.value.push({ expert: { id: exp.id, name: exp.name, type: exp.type }, response: `【${exp.name}的回答】关于「${multiConsultConfig.question.slice(0, 20)}」的问题，从${EXPERT_TYPES[exp.type] || '专业'}角度分析：\n\n1. 核心要点：问题涉及多个层面，需要系统思考\n2. 建议方案：采用${['分治法', '迭代法', '模块化'][idx % 3]}策略逐步解决\n3. 注意事项：需要关注边界条件和异常处理\n\n以上是我的初步分析，供参考。`, confidence: 0.8 + Math.random() * 0.18, duration_ms: 800 + Math.random() * 1200 })
    })
  } else {
    for (const exp of selectedExperts) {
      await new Promise(r => setTimeout(r, 600 + Math.random() * 600))
      multiConsultResults.value.push({ expert: { id: exp.id, name: exp.name, type: exp.type }, response: `【${exp.name}的回答】针对「${multiConsultConfig.question.slice(0, 20)}」这个问题，我的分析如下：\n\n首先，明确问题的核心目标和约束条件。其次，基于${EXPERT_TYPES[exp.type] || '专业领域'}的知识，推荐以下方案：\n- 方案A：保守稳妥，风险低\n- 方案B：激进高效，收益高\n- 方案C：折中平衡，适用性广\n\n建议根据实际情况选择合适的方案。`, confidence: 0.78 + Math.random() * 0.2, duration_ms: 600 + Math.random() * 800 })
    }
  }
  ElMessage.warning('咨询服务暂不可用，已生成模拟回答')
}

// ========== 智能路由匹配 ==========
const smartRouteQuestion = ref('')
const smartRouteResult = ref(null)
const smartRouteMaxExperts = ref(3)

function openSmartRouteDialog() {
  smartRouteQuestion.value = ''
  smartRouteResult.value = null
  smartRouteMaxExperts.value = 3
  showSmartRouteDialog.value = true
}

async function doSmartRoute() {
  if (!smartRouteQuestion.value.trim()) return
  smartRoutingLoading.value = true
  smartRouteResult.value = null
  try {
    const result = await routeExperts({ question: smartRouteQuestion.value, maxExperts: smartRouteMaxExperts.value })
    smartRouteResult.value = result?.data || result
    ElMessage.success('智能匹配完成')
  } catch (e) { console.warn('[routeExperts] 智能路由 API 失败:', e); simulateSmartRoute() }
  finally { smartRoutingLoading.value = false }
}

function simulateSmartRoute() {
  const question = smartRouteQuestion.value.toLowerCase()
  const scoredExperts = experts.value.filter(e => e.status === 'active').map(e => {
    let baseScore = 0.5 + Math.random() * 0.3
    const typeMatch = question.includes(e.type) ? 0.15 : 0
    const capMatch = e.capabilities?.some(c => question.includes(c.toLowerCase())) ? 0.1 : 0
    return { ...e, score: Math.min(0.98, baseScore + typeMatch + capMatch), reason: `基于「${EXPERT_TYPES[e.type]}」领域专长和${e.capabilities?.[0] || '相关'}技能匹配` }
  }).sort((a, b) => b.score - a.score).slice(0, smartRouteMaxExperts.value)
  smartRouteResult.value = { selected: scoredExperts, question: smartRouteQuestion.value, mode: 'auto', reasoning: `根据问题描述中的关键词和领域特征，从 ${experts.value.length} 位专家中筛选出最佳匹配` }
  ElMessage.warning('智能路由服务暂不可用，已生成模拟匹配结果')
}

function selectRoutedExpert(item) {
  const id = item.id || item.expert_id
  if (!id) return
  if (!selectedExpertIds.value.includes(id)) selectedExpertIds.value.push(id)
  ElMessage.success(`已选择专家「${item.name || item.expert_name}」`)
}

function selectAllRoutedExperts() {
  const items = smartRouteResult.value?.selected || []
  let added = 0
  items.forEach(item => {
    const id = item.id || item.expert_id
    if (id && !selectedExpertIds.value.includes(id)) { selectedExpertIds.value.push(id); added++ }
  })
  if (added > 0) ElMessage.success(`已添加 ${added} 位推荐专家`)
  else ElMessage.info('推荐专家均已选中')
  showSmartRouteDialog.value = false
}

// ========== 全局事件 / 快捷工具 ==========
function handleOpenRegisterExpert() { showRegisterDialog.value = true }
function handleOpenExpertDebate() { openDebateDialog() }
function handleOpenMultiConsult() { openMultiConsultDialog() }
function handleSmartRouteExpert() { openSmartRouteDialog() }

function triggerDebate() {
  if (selectedExpertIds.value.length < 2) { ElMessage.warning('请至少选择 2 位专家进行辩论'); return }
  activeMode.value = 'debate'
  collabExpanded.value = true
  collabMode.value = 'debate'
  newCollaboration()
  collabInput.value = `请以下专家就[主题]展开辩论：${selectedExpertNames()}`
}

function triggerOrchestration() {
  activeMode.value = 'orchestration'
  collabExpanded.value = true
  collabMode.value = 'multi'
  if (taskOrchestration.subtasks.length === 0) {
    taskOrchestration.originalTask = '设计并实现一个基于知识图谱的智能问答系统，要求支持多轮对话和上下文理解'
  }
}

function triggerVoting() {
  if (selectedExpertIds.value.length < 2) { ElMessage.warning('请至少选择 2 位专家参与投票'); return }
  collabExpanded.value = true
  collabInput.value = `请以下专家就方案进行投票：${selectedExpertNames()}`
}

function selectedExpertNames() {
  return experts.value.filter(e => selectedExpertIds.value.includes(e.id)).map(e => e.name).join('、')
}

// ========== 图谱节点交互 ==========
function viewNodeDocs(node) {
  rightCollapsed.value = false
  activeKbTab.value = 'docs'
  kbSearchQuery.value = node.fullName || node.label
  searchKb()
}

function askExpertsAbout(node) {
  collabExpanded.value = true
  collabInput.value = `请专家们分析一下「${node.fullName || node.label}」的相关情况，包括其定义、关联关系和应用场景。`
  if (!activeSession.value) newCollaboration()
}

// ========== 协作对话（useAlliance composable）==========
const collabMode = ref('smart')
function onCollabModeChange() { /* 模式变化时的处理 */ }

const {
  collabMessages, collabInput, allianceRunning, currentPhaseIndex,
  messagesScrollRef, currentPhaseLabel,
  sendCollabMsg, stopAlliance, scrollMessagesToBottom, appendMessage
} = useAlliance(expertColor, expertEmoji, selectedExpertIds, currentProject, collabMode, activeSession, newCollaboration)

function insertNodeRef() {
  if (selectedNode.value) collabInput.value += `【节点：${selectedNode.value.fullName || selectedNode.value.label}】`
}

// ========== 知识库 ==========
const activeKbTab = ref('docs')
const kbSearchQuery = ref('')
const activeDoc = ref(null)
const docsLoading = ref(false)
const categories = ref([])
const documents = ref([])
const popularTags = ref([])
const docVersions = ref([])
const activeCategory = ref(null)
const expandedCategories = ref([])

async function switchKbTab(tab) {
  activeKbTab.value = tab
  if (tab === 'docs') { if (documents.value.length === 0) loadDocuments(); if (categories.value.length === 0) loadCategories() }
  else if (tab === 'tags') { if (popularTags.value.length === 0) loadTags() }
  else if (tab === 'versions') { if (activeDoc.value) loadVersions(activeDoc.value.id) }
}

async function loadDocuments() {
  docsLoading.value = true
  try {
    const res = await kbListDocuments({ project_id: currentProject.value, limit: 50 })
    if (res && Array.isArray(res.data)) documents.value = res.data
    else if (res && Array.isArray(res)) documents.value = res
    else documents.value = getMockDocs()
  } catch (e) { console.warn('[workspace] 加载文档失败:', e); documents.value = getMockDocs() }
  finally { docsLoading.value = false }
}

async function loadCategories() {
  try {
    const res = await kbGetCategories()
    if (res && Array.isArray(res.data)) categories.value = res.data
    else if (res && Array.isArray(res)) categories.value = res
    else categories.value = getMockCategories()
    expandedCategories.value = categories.value.map(c => c.id)
  } catch (e) { categories.value = getMockCategories(); expandedCategories.value = categories.value.map(c => c.id) }
}

async function loadTags() {
  try {
    const res = await kbGetTags()
    if (res && Array.isArray(res.data)) popularTags.value = res.data.map(t => ({ name: t.name || t.tag, count: t.count || 0, fontSize: 12 + Math.min(t.count || 0, 20) * 0.5 }))
    else popularTags.value = getMockTags()
  } catch (e) { popularTags.value = getMockTags() }
}

async function loadVersions(docId) {
  try {
    const res = await kbGetVersions(docId)
    if (res && Array.isArray(res.data)) docVersions.value = res.data
    else if (res && Array.isArray(res)) docVersions.value = res
    else docVersions.value = getMockVersions()
  } catch (e) { docVersions.value = getMockVersions() }
}

function selectCategory(cat) { activeCategory.value = activeCategory.value === cat.id ? null : cat.id }
function openDoc(doc) { activeDoc.value = doc; if (activeKbTab.value === 'versions') loadVersions(doc.id) }

async function searchKb() {
  if (!kbSearchQuery.value.trim()) { if (documents.value.length === 0) loadDocuments(); return }
  docsLoading.value = true
  try {
    const res = await kbSearch({ query: kbSearchQuery.value, project_id: currentProject.value })
    if (res && Array.isArray(res.data)) documents.value = res.data
  } catch (e) { /* 使用前端过滤 */ }
  finally { docsLoading.value = false }
}

function filterByTag(tag) { kbSearchQuery.value = tag.name; activeKbTab.value = 'docs'; searchKb() }
function createDoc() { ElMessage.info('新建文档功能开发中…') }

// ========== AI 助手 ==========
const allianceCapabilitiesList = ref([])

async function loadAllianceCapabilities() {
  try {
    const caps = await getAllianceCapabilities()
    if (caps?.intent_classes_7) allianceCapabilitiesList.value = caps.intent_classes_7
    else allianceCapabilitiesList.value = ['7 类意图识别', '专家智能匹配', '多轮交叉辩论', '综合方案归纳', '质量闸门把关', '知识增量学习', '14 维度评估']
  } catch (e) { allianceCapabilitiesList.value = ['7 类意图识别', '专家智能匹配', '多轮交叉辩论', '综合方案归纳', '质量闸门把关', '知识增量学习', '14 维度评估'] }
}

function openAIAssistant() { aiAssistantOpen.value = !aiAssistantOpen.value }
function aiSuggestion(type) {
  collabInput.value = `请执行：${type}`
  aiAssistantOpen.value = false
  collabExpanded.value = true
  if (!activeSession.value) newCollaboration()
}

// ========== 通知 ==========
function removeNotification(id) {
  const idx = notifications.value.findIndex(n => n.id === id)
  if (idx >= 0) notifications.value.splice(idx, 1)
}

// ========== 工具函数 ==========
function formatTime(ts) {
  if (!ts) return ''
  const now = Date.now()
  const diff = now - ts
  if (diff < 60000) return '刚刚'
  if (diff < 3600000) return Math.floor(diff / 60000) + '分钟前'
  if (diff < 86400000) return Math.floor(diff / 3600000) + '小时前'
  if (diff < 604800000) return Math.floor(diff / 86400000) + '天前'
  const d = new Date(ts)
  return `${d.getMonth() + 1}/${d.getDate()}`
}

// ========== Composables 初始化 ==========
const {
  activeWbTool, activeWbColor, wbNotes, wbTexts, wbLines,
  wbDrawPaths, wbCurrentPath, wbViewBox,
  selectWbTool, onWbMouseDown, onWbMouseMove, onWbMouseUp,
  addWbNote, startDragNote, deleteWbNote, updateNoteContent,
  addWbText, startDragText, deleteWbText, updateTextContent,
  clearWhiteboard, saveWhiteboard
} = useWhiteboard(addHistoryEvent)

const {
  activeCanvasTool, currentLayout, selectedNode, graphLoading, graphAnalyzing,
  viewport, svgViewBox, graphNodes, graphEdges, graphStats,
  loadGraphData, selectNode, switchLayout, zoomIn, zoomOut, fitView, runGraphAlgo,
  onCanvasMouseDown, onCanvasMouseMove, onCanvasMouseUp, onCanvasWheel, onNodeMouseDown
} = useGraphCanvas(expertColor)

function addOrchMessage(msg) {
  collabMessages.value.push({ id: Date.now() + Math.random(), time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), ...msg })
  if (messagesScrollRef.value) nextTick(() => { messagesScrollRef.value.scrollTo?.({ top: 999999, behavior: 'smooth' }) })
}

const {
  taskOrchestration, decomposing, orchIsRunning, activeSubtaskId, timelineView,
  draggingTaskId, dragOverTaskId, expertDragOverTaskId, ganttSlotMinutes,
  decomposeTask, addSubtaskManually, editSubtask, deleteSubtask,
  toggleSubtaskExpand, collapseAllSubtasks, selectSubtask,
  onTaskDragStart, onTaskDragEnd, onTaskDragOver, onTaskDrop,
  onExpertDragStart, onExpertDragEnd, onExpertDragOverTask, onExpertDragLeaveTask, onExpertDropOnTask,
  unassignExpert, autoAssignExperts, openAssignDialog, startTaskExecution, resetAllTasks
} = useTaskOrchestration(experts, expertColor, expertEmoji, addHistoryEvent, addOrchMessage)

// ========== 生命周期 ==========
onMounted(() => {
  loadProjects(); loadExperts(); loadSessions(); loadGraphData()
  loadCategories(); loadDocuments(); loadTags(); loadAllianceCapabilities()
  window.addEventListener('mox:open-register-expert', handleOpenRegisterExpert)
  window.addEventListener('mox:open-expert-debate', handleOpenExpertDebate)
  window.addEventListener('mox:open-multi-consult', handleOpenMultiConsult)
  window.addEventListener('mox:smart-route-expert', handleSmartRouteExpert)
  window.addEventListener('keydown', handleKeydown)
})

onBeforeUnmount(() => {
  window.removeEventListener('mox:open-register-expert', handleOpenRegisterExpert)
  window.removeEventListener('mox:open-expert-debate', handleOpenExpertDebate)
  window.removeEventListener('mox:open-multi-consult', handleOpenMultiConsult)
  window.removeEventListener('mox:smart-route-expert', handleSmartRouteExpert)
  window.removeEventListener('keydown', handleKeydown)
})

watch(selectedExpertIds, () => {
  if (activeSession.value) activeSession.value.expert_count = selectedExpertIds.value.length
}, { deep: true })
</script>
