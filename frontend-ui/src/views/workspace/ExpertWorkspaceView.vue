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
            :store="graphStore"
            :graph-stats="graphStats"
            :graph-loading="graphLoading"
            :viewport-style="viewportStyle"
            @zoom-in="zoomIn"
            @zoom-out="zoomOut"
            @fit-view="fitView"
            @retry="loadGraphData"
            @open-graph-workbench="openGraphWorkbench"
            @canvas-mousedown="onCanvasMouseDown"
            @canvas-mousemove="onCanvasMouseMove"
            @canvas-mouseup="onCanvasMouseUp"
            @canvas-wheel="onCanvasWheel"
            @select-node="selectNode"
            @clear-selected-node="clearSelectedNode"
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
          @file-uploaded="(f) => handleFileUpload(f)"
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
          @save-whiteboard="handleSaveWhiteboard(activeSession)"
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
      v-model:rounds="debateConfig.rounds"
      :selected-expert-ids="debateConfig.selectedExpertIds"
      :problem="debateProblem"
      :note="debateNote"
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
      v-model:compare-view="multiConsultCompareView"
      :selected-expert-ids="multiConsultConfig.selectedExpertIds"
      :problem="multiConsultProblem"
      :results="multiConsultResults"
      :fusion="multiConsultFusion"
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
      :problem="smartRouteProblem"
      :loading="smartRoutingLoading"
      :candidates="smartRouteRows"
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
import { formatClockMinute } from '@/utils'
import { ref, reactive, computed, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import { allianceApi } from '@/modules/expert-alliance/api'
// 协作流一律走模块契约：入参字段名、上下界、结果口径都只在契约层出现一次。
// 视图自己拼 body 就是 defects 的产地——后端字段是 topic 时发 question 会吃 422，
// 发 camelCase 的 maxExperts 会被 serde 静默丢弃。
import { COLLAB_MODE, collabControlValue, collabProblem, collabTemplateNote, collabMode as collabModeOf, graphNodeLabel, SESSION_PAGE, sessionActivityAt, sessionListTitle, sessionTypeForCollabMode } from '@/modules/expert-alliance/contract'
import {
  collabCandidateItems, collabContributionItems, collabDebateTurns, collabResultNote,
  draftSession, expertPickable, expertVisualKey, sessionTimeText
} from '@/modules/expert-alliance/model'
import { ElMessage } from 'element-plus/es/components/message/index'
import { ElMessageBox } from 'element-plus/es/components/message-box/index'
import { expertColor, expertEmoji } from '@/constants'
import { RegisterExpertDialog } from '@/components'
import {
  kbListDocuments, kbGetCategories, kbGetTags,
  kbSearch, kbGetVersions, kbCreateDocument
} from '@/api'
import { getProjects } from '@/api'
import { unwrap, unwrapList } from '@/modules/_kernel/envelope'
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
import { useWhiteboard } from '@/composables'
import { useGraphCanvas } from '@/composables'
import { useTaskOrchestration } from '@/composables'
import { useAlliance } from '@/composables'
import { useWorkspaceData } from '@/composables'

// ========== 布局状态 ==========
const leftCollapsed = ref(false)
const rightCollapsed = ref(false)
const collabExpanded = ref(true)
const aiAssistantOpen = ref(false)
const historyPanelOpen = ref(false)

// ========== KPI 指标卡（数据由 useWorkspaceData 加载） ==========

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
    const list = unwrapList(unwrap(data), 'items')
    if (list.length) {
      projectOptions.value = list.map((p) => ({ id: p.id, name: p.name || p.title || '未命名项目' }))
      if (!projectOptions.value.find((p) => p.id === currentProject.value)) currentProject.value = projectOptions.value[0].id
    }
  } catch (e) { ElMessage.error(e?.message || '加载项目列表失败') }
}

function onProjectChange() {
  loadExperts(); loadSessions(); loadGraphData(); loadDocuments()
  reloadOnProjectChange()
}
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
    // 后端不读 project_id，也没有 status=active 这一档（availability 只有 online/busy/offline/away），
    // 旧代码带这两个查询条件只会拿到空表。
    const { items } = await allianceApi.listExperts()
    experts.value = items
  } catch (e) { experts.value = []; ElMessage.error(e?.message || '加载专家列表失败') }
  finally { expertsLoading.value = false }
}

// ========== 协作会话 ==========
const sessions = ref([])
const sessionsLoading = ref(false)
const activeSession = ref(null)

/**
 * 本地草稿行走与服务端同一个出口（model/normalize.js 的 draftSession）：两者会出现在同一个列表里，必须同形
 * （同键名、同为 RFC3339 字符串）。旧版草稿在这里手写 created_at/updated_at 加 Date.now() 毫秒数，
 * 于是"活跃度"一栏对服务端行与草稿行各走一套时间口径，而 updated_at 后端根本没有。
 * 草稿不落库（工作台从不调用 sessionCreate），reload 即消失——这是既有行为，本次不动它，
 * 只在注释里留痕：界面不许把草稿说成已保存。
 */
function localSessionRow(title, expertIds, mode) {
  const picked = [...(expertIds || [])]
  return draftSession({
    id: 'sess-' + Date.now(),
    title,
    expertIds: picked,
    sessionType: sessionTypeForCollabMode(mode, picked.length),
    at: new Date().toISOString()
  })
}

async function loadSessions() {
  sessionsLoading.value = true
  try {
    // 走模块 api：信封解包与字段投影都只在它那里做一次。旧版在这里猜响应形状
    // （res.data / res 两种都是数组才收），而后端返回的是 {sessions,total,page,page_size}，
    // 两个分支都不成立 ⇒ 列表恒空；project_id 由 http 层统一注入，limit 与 page_size 同义。
    const { items } = await allianceApi.listSessions({ pageSize: SESSION_PAGE.defaultSize })
    sessions.value = items
  } catch (e) { sessions.value = []; ElMessage.error(e?.message || '加载会话失败') }
  finally { sessionsLoading.value = false }
}

function selectSession(session) {
  activeSession.value = session
  collabMessages.value = [{ id: Date.now(), role: 'system', name: '系统', avatar: '📢', color: '#64748b', time: sessionTimeText(sessionActivityAt(session)), text: `已进入「${sessionListTitle(session)}」协作会话` }]
}

function newCollaboration() {
  activeMode.value = 'collaboration'
  collabExpanded.value = true
  const newSess = localSessionRow('新协作会话', selectedExpertIds.value, collabMode.value)
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

// ========== 协作成员 / 阶段 / 文件 / 历史（由 useWorkspaceData 提供） ==========
// 状态：collabMembers, projectPhases, currentProjectPhase, sharedFiles, historyEvents
// 方法：jumpToPhase, previewFile, downloadFile, handleFileUpload, appendHistory, jumpToHistory
// 初始化见下方 useWorkspaceData() 调用

function sendToWhiteboard() {
  if (collabInput.value.trim()) { addWbNote(collabInput.value.substring(0, 20), collabInput.value); ElMessage.success('已添加到白板') }
  else ElMessage.warning('请先输入内容')
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
  // 列表整体重取：视图不拼第第二套行形状，注册响应与列表行的差异交给归一化层
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
// 输入键与后端 wire 的对应、轮数上下界、发起前的可发条件全部取自契约层，
// 视图不再自带一份「adversarial/roundtable」这类后端结构体里不存在的字段。
const debateDef = collabModeOf(COLLAB_MODE.DEBATE)
const debateNote = collabTemplateNote(debateDef)
const debateDefaults = Object.fromEntries(collabControlValue(debateDef, {}).map((c) => [c.wire, c.value]))
const debateConfig = reactive({ topic: '', selectedExpertIds: [], rounds: debateDefaults.rounds })
const debateStatus = ref('preparing')
const debateTurns = ref([])
const debateSummary = ref('')
const debateProblem = computed(() => collabProblem(debateDef, debateConfig))
const canStartDebate = computed(() => !debateProblem.value)

const expertById = computed(() => new Map(experts.value.map((e) => [e.id, e])))
// 辩论参与者只带 id/name/side，配色要落回花名册那一行才与全站的色系一致
function debateSpeakerExpert(result, side) {
  const p = (result?.participants || []).find((x) => x.side === side)
  return p ? expertById.value.get(p.id) : null
}

function openDebateDialog() {
  debateConfig.topic = ''
  debateConfig.selectedExpertIds = [...selectedExpertIds.value]
  debateConfig.rounds = debateDefaults.rounds
  debateStatus.value = 'preparing'
  debateTurns.value = []
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
  debateTurns.value = []
  debateSummary.value = ''
  try {
    const result = await allianceApi.collaborate(COLLAB_MODE.DEBATE, {
      topic: debateConfig.topic,
      expertIds: debateConfig.selectedExpertIds,
      rounds: debateConfig.rounds
    })
    debateTurns.value = collabDebateTurns(result)
    debateSummary.value = result.verdict.summary
    debateStatus.value = 'summarized'
    appendDebateToCollab(result)
    ElMessage.success(`辩论完成，共 ${result.rounds} 轮，${result.verdict.winner || '未见'}占优`)
  } catch (e) { debateStatus.value = 'preparing'; ElMessage.error(`辩论服务调用失败：${e?.message || '未知错误'}`) }
  finally { debateSubmitting.value = false }
}

function appendDebateToCollab(result) {
  if (!activeSession.value) {
    const newSess = localSessionRow(debateConfig.topic.slice(0, 20) + '…', debateConfig.selectedExpertIds, COLLAB_MODE.DEBATE)
    sessions.value.unshift(newSess)
    selectSession(newSess)
  }
  const now = () => formatClockMinute()
  collabMessages.value.push({ id: Date.now(), role: 'system', name: '辩论系统', avatar: '⚔️', color: '#ef4444', time: now(), text: `【辩论开始】辩题：${result.topic}` })
  debateTurns.value.forEach(turn => {
    const speaker = debateSpeakerExpert(result, turn.side)
    const key = expertVisualKey(speaker)
    collabMessages.value.push({ id: Date.now() + Math.random(), role: 'expert', name: `${turn.sideLabel}·${turn.name}`, avatar: expertEmoji(key), color: expertColor(key), phase: 'debate', time: now(), text: `${turn.text}（第 ${turn.round} 轮 ${turn.score.toFixed(2)} 分）` })
  })
  if (debateSummary.value) collabMessages.value.push({ id: Date.now() + 999, role: 'assistant', name: '辩论裁决', avatar: '📝', color: '#10b981', phase: 'synthesize', time: now(), text: `${debateSummary.value}${collabResultNote(result)}` })
  scrollMessagesToBottom()
}

// ========== 多专家咨询 ==========
const multiDef = collabModeOf(COLLAB_MODE.MULTI)
const multiConsultConfig = reactive({ question: '', selectedExpertIds: [] })
const multiConsultResults = ref([])
const multiConsultFusion = ref(null)
const multiConsultCompareView = ref(false)
const multiConsultProblem = computed(() => collabProblem(multiDef, multiConsultConfig))
const canStartMultiConsult = computed(() => !multiConsultProblem.value)

function openMultiConsultDialog() {
  multiConsultConfig.question = ''
  multiConsultConfig.selectedExpertIds = [...selectedExpertIds.value]
  multiConsultResults.value = []
  multiConsultFusion.value = null
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
  multiConsultFusion.value = null
  try {
    const result = await allianceApi.collaborate(COLLAB_MODE.MULTI, {
      question: multiConsultConfig.question,
      expertIds: multiConsultConfig.selectedExpertIds
    })
    // 后端回的键是 experts，旧代码读 results/successful 那两个键在本仓库任何实现里都不存在
    multiConsultResults.value = collabContributionItems(result)
    multiConsultFusion.value = result.fusion
    ElMessage.success(`咨询完成，共 ${multiConsultResults.value.length} 位专家参与`)
  } catch (e) { ElMessage.error(`多专家咨询服务调用失败：${e?.message || '未知错误'}`) }
  finally { multiConsultSubmitting.value = false }
}

// ========== 智能路由匹配 ==========
const routeDef = collabModeOf(COLLAB_MODE.ROUTE)
const routeDefaults = Object.fromEntries(collabControlValue(routeDef, {}).map((c) => [c.wire, c.value]))
const smartRouteQuestion = ref('')
const smartRouteResult = ref(null)
const smartRouteMaxExperts = ref(routeDefaults.max_experts)
const smartRouteRows = computed(() => collabCandidateItems(smartRouteResult.value))
const smartRouteProblem = computed(() => collabProblem(routeDef, { question: smartRouteQuestion.value }))

function openSmartRouteDialog() {
  smartRouteQuestion.value = ''
  smartRouteResult.value = null
  smartRouteMaxExperts.value = routeDefaults.max_experts
  showSmartRouteDialog.value = true
}

async function doSmartRoute() {
  if (smartRouteProblem.value) return
  smartRoutingLoading.value = true
  smartRouteResult.value = null
  try {
    smartRouteResult.value = await allianceApi.collaborate(COLLAB_MODE.ROUTE, {
      question: smartRouteQuestion.value,
      maxExperts: smartRouteMaxExperts.value
    })
    ElMessage.success(`智能匹配完成，候选 ${smartRouteRows.value.length} 位`)
  } catch (e) {
    ElMessage.error(e?.message || '智能路由服务调用失败')
  } finally { smartRoutingLoading.value = false }
}

function selectRoutedExpert(item) {
  const id = item?.id
  if (!id) return
  if (!selectedExpertIds.value.includes(id)) selectedExpertIds.value.push(id)
  ElMessage.success(`已选择专家「${item.name || '专家'}」`)
}

function selectAllRoutedExperts() {
  let added = 0
  smartRouteRows.value.forEach(item => {
    if (item.id && !selectedExpertIds.value.includes(item.id)) { selectedExpertIds.value.push(item.id); added++ }
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
// 节点名一律经 contract/graph.js 的 graphNodeLabel：后端写入侧丢过编码（label='???????'），
// 直接拼进提问串等于让专家去猜一串问号，那里会换成"未命名节点 + id 短码"。
function viewNodeDocs(node) {
  rightCollapsed.value = false
  activeKbTab.value = 'docs'
  kbSearchQuery.value = graphNodeLabel(node)
  searchKb()
}

function askExpertsAbout(node) {
  collabExpanded.value = true
  collabInput.value = `请专家们分析一下「${graphNodeLabel(node)}」的相关情况，包括其定义、关联关系和应用场景。`
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

// ========== 工作台域数据（useWorkspaceData composable）==========
const {
  notifCount, hasNotifications,
  kpiCards, kpiLoading,
  collabMembers, membersLoading,
  projectPhases, currentProjectPhase, phasesLoading,
  sharedFiles, filesLoading,
  historyEvents, historyLoading,
  loadUnreadCount, loadKpi, loadMembers, loadPhases, loadFiles, loadHistory,
  loadAllWorkspaceData, reloadOnProjectChange,
  jumpToPhase, handleFileUpload, previewFile, downloadFile,
  appendHistory, jumpToHistory, persistWhiteboard
} = useWorkspaceData(currentProject)

// addHistoryEvent 包装器：供 useWhiteboard / useTaskOrchestration / switchWorkMode 调用
function addHistoryEvent(type, title, description) {
  appendHistory(type, title, description)
}

// 白板保存：本地 localStorage + 后端持久化（双写）
async function handleSaveWhiteboard(activeSession) {
  saveWhiteboard(activeSession)
  const sessionId = activeSession?.value?.id
  if (sessionId) {
    const data = { notes: wbNotes.value, texts: wbTexts.value, lines: wbLines.value, drawPaths: wbDrawPaths.value }
    await persistWhiteboard(sessionId, data)
  }
}

function insertNodeRef() {
  if (selectedNode.value) collabInput.value += `【节点：${graphNodeLabel(selectedNode.value)}】`
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
    documents.value = unwrapList(unwrap(res), 'items')
  } catch (e) { documents.value = []; ElMessage.error(e?.message || '加载文档失败') }
  finally { docsLoading.value = false }
}

async function loadCategories() {
  try {
    categories.value = unwrapList(unwrap(await kbGetCategories()))
    expandedCategories.value = categories.value.map(c => c.id)
  } catch (e) { categories.value = []; expandedCategories.value = []; ElMessage.error(e?.message || '加载分类失败') }
}

async function loadTags() {
  try {
    // /kb/tags 给的是裸数组 [{name,count}]，没有 tag 这个别名键
    popularTags.value = unwrapList(unwrap(await kbGetTags())).map(t => ({
      name: t.name,
      count: t.count || 0,
      fontSize: 12 + Math.min(t.count || 0, 20) * 0.5
    }))
  } catch (e) { popularTags.value = []; ElMessage.error(e?.message || '加载标签失败') }
}

async function loadVersions(docId) {
  try {
    // /kb/documents/:id/versions 的 payload 是 {doc_id,versions}
    docVersions.value = unwrapList(unwrap(await kbGetVersions(docId)), 'versions')
  } catch (e) { docVersions.value = []; ElMessage.error(e?.message || '加载版本失败') }
}

function selectCategory(cat) { activeCategory.value = activeCategory.value === cat.id ? null : cat.id }
function openDoc(doc) { activeDoc.value = doc; if (activeKbTab.value === 'versions') loadVersions(doc.id) }

async function searchKb() {
  if (!kbSearchQuery.value.trim()) { if (documents.value.length === 0) loadDocuments(); return }
  docsLoading.value = true
  try {
    // /kb/search 的 payload 是 {results,graph_hits,total}，行形状是 SearchHit（snippet/score，没有 updated_at）
    documents.value = unwrapList(unwrap(await kbSearch({ query: kbSearchQuery.value, limit: 50 })), 'results')
  } catch (e) { documents.value = []; ElMessage.error(e?.message || '检索失败') }
  finally { docsLoading.value = false }
}

function filterByTag(tag) { kbSearchQuery.value = tag.name; activeKbTab.value = 'docs'; searchKb() }
async function createDoc() {
  try {
    const { value: title } = await ElMessageBox.prompt('请输入文档标题', '新建文档', {
      confirmButtonText: '创建',
      cancelButtonText: '取消',
      inputPlaceholder: '如：需求分析文档',
      inputValidator: (v) => !!v?.trim() || '文档标题不能为空'
    })
    const payload = { title: title.trim(), project_id: currentProject.value }
    const created = await kbCreateDocument(payload)
    ElMessage.success(`文档「${title.trim()}」创建成功`)
    await loadDocuments()
    if (created?.id) openDoc(created)
  } catch (e) {
    if (e !== 'cancel') ElMessage.error(e?.message || '创建文档失败')
  }
}

// ========== AI 助手 ==========
// 能力清单取自 GET /api/experts/capabilities（只统计 enabled 专家，后端按 capability id 升序）。
// 原实现读的是编排器独占端点、且取该端点从未产出的一个键，所以界面一直落在写死文案上；
// 被禁端点已于 2026-09-27 归一化撤除（见模块 contract/endpoints.js）。
const allianceCapabilitiesList = ref([])

async function loadAllianceCapabilities() {
  try {
    const caps = await allianceApi.listExpertCapabilities()
    allianceCapabilitiesList.value = caps.items.map((c) => `${c.name}·${c.expertCount} 位专家`)
  } catch (e) { allianceCapabilitiesList.value = [] }
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
// 会话时间不在这里格式化：一天内相对、超出绝对的两档口径由 model/display.js 的 sessionTimeText 单源给出，
// 旧版在此另写一份，且把入参当毫秒数减（Date.now() - '2026-…' = NaN），服务端行永远落到 M/D 兜底档。

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
  graphStore, selectedNode, graphLoading, graphStats, viewportStyle,
  loadGraphData, selectNode, clearSelectedNode, zoomIn, zoomOut, fitView,
  onCanvasMouseDown, onCanvasMouseMove, onCanvasMouseUp, onCanvasWheel
} = useGraphCanvas()

function openGraphWorkbench() {
  router.push('/alliance/graph')
}

function addOrchMessage(msg) {
  collabMessages.value.push({ id: Date.now() + Math.random(), time: formatClockMinute(), ...msg })
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
  loadAllWorkspaceData()
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
  // 成员数就是 expertIds 本身；旧版这里写的是 expert_count —— 后端没有这个键，
  // 它只是给面板提供了一个假字段，reload 后（服务端行）计数立刻变 0。
  if (activeSession.value) activeSession.value.expertIds = [...selectedExpertIds.value]
}, { deep: true })
</script>
