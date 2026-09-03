<!--
  协作讨论面板（底部栏）
  职责：阶段进度、协作Tab（讨论/白板/文件）、消息列表、输入区、成员展示、历史记录
-->
<template>
  <div class="ws-collab-bar glass-card" :class="{ expanded: expanded, 'is-running': allianceRunning, 'mode-transition': modeTransitioning }">
    <div class="ws-collab-gradient-bar"></div>

    <div class="ws-collab-header" @click="$emit('toggle-expand')">
      <div class="ws-collab-title">
        <div class="ws-collab-title-icon">
          <el-icon v-if="allianceRunning" class="ws-pulse-icon"><Promotion /></el-icon>
          <el-icon v-else><ChatLineSquare /></el-icon>
        </div>
        <span class="ws-collab-title-text">协作讨论 · {{ activeSession?.title || '未开始' }}</span>
        <el-tag v-if="allianceRunning" size="small" effect="light" class="ws-running-tag gradient-tag">
          {{ currentPhaseLabel }}
        </el-tag>
        <span class="ws-collab-count">{{ collabMessages.length }} 条消息</span>
        <span v-if="typingExperts.length > 0" class="ws-typing-indicator">
          <span class="typing-dots-mini"><i></i><i></i><i></i></span>
          {{ typingExperts.map(e => e.name).join('、') }} 正在思考
        </span>
      </div>
      <div class="ws-collab-header-actions" @click.stop>
        <button class="ws-header-action-btn" :class="{ active: historyPanelOpen }" title="历史记录" @click="$emit('toggle-history')">
          <el-icon><RefreshRight /></el-icon>
        </button>
        <div class="ws-collab-toggle">
          <el-icon v-if="expanded"><ArrowDown /></el-icon>
          <el-icon v-else><ArrowUp /></el-icon>
        </div>
      </div>
    </div>

    <div v-if="expanded" class="ws-collab-body">
      <!-- 阶段进度可视化 -->
      <div class="ws-phase-progress-bar">
        <div
          v-for="(phase, idx) in projectPhases"
          :key="phase.key"
          class="ws-phase-item"
          :class="{ active: currentProjectPhase === idx, done: currentProjectPhase > idx, 'clickable': true }"
          @click="$emit('jump-to-phase', idx)"
        >
          <div class="ws-phase-dot-wrapper">
            <div class="ws-phase-dot">
              <el-icon v-if="currentProjectPhase > idx"><CircleCheckFilled /></el-icon>
              <span v-else>{{ idx + 1 }}</span>
            </div>
            <div v-if="idx < projectPhases.length - 1" class="ws-phase-connector" :class="{ filled: currentProjectPhase > idx }"></div>
          </div>
          <span class="ws-phase-label">{{ phase.label }}</span>
        </div>
      </div>

      <!-- 协作内容 Tab -->
      <div class="ws-collab-tabs">
        <button
          v-for="tab in collabTabs"
          :key="tab.key"
          class="ws-collab-tab"
          :class="{ active: activeCollabTab === tab.key }"
          @click="$emit('update:activeCollabTab', tab.key)"
        >
          <el-icon class="ws-collab-tab-icon"><component :is="tab.icon" /></el-icon>
          <span>{{ tab.label }}</span>
          <el-badge v-if="tab.badge" :value="tab.badge" class="ws-tab-badge" />
        </button>
        <div class="ws-collab-tabs-right">
          <!-- 协作成员 -->
          <div class="ws-collab-members">
            <div
              v-for="(member, idx) in collabMembers.slice(0, 4)"
              :key="member.id"
              class="ws-member-avatar"
              :style="{ background: member.color, zIndex: 10 - idx }"
              :title="member.name + ' - ' + memberStatusText(member.status)"
            >
              {{ member.avatar }}
              <span class="ws-member-status-dot" :class="'status-' + member.status"></span>
            </div>
            <div v-if="collabMembers.length > 4" class="ws-member-avatar more-avatar" :title="`还有 ${collabMembers.length - 4} 位成员`">
              +{{ collabMembers.length - 4 }}
            </div>
          </div>
        </div>
      </div>

      <!-- ===== 讨论 Tab ===== -->
      <div v-show="activeCollabTab === 'discussion'" class="ws-tab-content ws-discussion-content">
        <!-- 文件栏 -->
        <div v-if="sharedFiles.length > 0" class="ws-files-bar">
          <div class="ws-files-bar-header">
            <span class="ws-files-bar-title">
              <el-icon><FolderOpened /></el-icon>
              共享文件 ({{ sharedFiles.length }})
            </span>
            <el-button size="small" text class="ws-files-bar-toggle" @click="filesBarExpanded = !filesBarExpanded">
              {{ filesBarExpanded ? '收起' : '展开' }}
              <el-icon><component :is="filesBarExpanded ? 'ArrowUp' : 'ArrowDown'" /></el-icon>
            </el-button>
          </div>
          <div v-show="filesBarExpanded" class="ws-files-list">
            <div
              v-for="file in sharedFiles"
              :key="file.id"
              class="ws-file-card"
              @click="$emit('preview-file', file)"
            >
              <div class="ws-file-icon" :class="'file-' + file.type">
                {{ fileIconEmoji(file.type) }}
              </div>
              <div class="ws-file-info">
                <div class="ws-file-name">{{ file.name }}</div>
                <div class="ws-file-meta">{{ file.size }} · {{ file.uploader }} · {{ file.time }}</div>
              </div>
              <el-button size="small" text class="ws-file-download" @click.stop="$emit('download-file', file)" title="下载">
                <el-icon><Download /></el-icon>
              </el-button>
            </div>
          </div>
        </div>

        <el-scrollbar class="ws-collab-messages" ref="messagesScrollRef">
          <div v-for="msg in collabMessages" :key="msg.id" class="ws-collab-msg" :class="[msg.role, msg.phase ? `phase-${msg.phase}` : '']">
            <div class="ws-collab-msg-avatar gradient-avatar" :style="{ background: msg.color || 'linear-gradient(135deg, #6366f1, #06b6d4)' }">
              {{ msg.avatar || '?' }}
            </div>
            <div class="ws-collab-msg-content">
              <div class="ws-collab-msg-meta">
                <span class="ws-collab-msg-name">{{ msg.name }}</span>
                <span v-if="msg.status" class="ws-msg-status" :class="'status-' + msg.status" :title="msgStatusText(msg.status)">
                  <el-icon v-if="msg.status === 'sent'"><CircleCheckFilled /></el-icon>
                  <el-icon v-else-if="msg.status === 'thinking'" class="pulse-icon"><Loading /></el-icon>
                  <el-icon v-else-if="msg.status === 'done'"><CircleCheckFilled /></el-icon>
                  <el-icon v-else-if="msg.status === 'failed'"><Warning /></el-icon>
                </span>
                <span v-if="msg.phase" class="ws-collab-msg-phase">
                  <el-tag size="small" effect="plain" :type="phaseTagType(msg.phase)">{{ phaseLabel(msg.phase) }}</el-tag>
                </span>
                <span class="ws-collab-msg-time">{{ msg.time }}</span>
              </div>
              <div class="ws-collab-msg-text" v-html="formatMessageText(msg.text)"></div>
              <div v-if="msg.files && msg.files.length > 0" class="ws-msg-files">
                <div
                  v-for="file in msg.files"
                  :key="file.id"
                  class="ws-msg-file-chip"
                  @click="$emit('preview-file', file)"
                >
                  <span class="msg-file-icon">{{ fileIconEmoji(file.type) }}</span>
                  <span class="msg-file-name">{{ file.name }}</span>
                </div>
              </div>
            </div>
          </div>
          <!-- 正在输入提示 -->
          <div v-for="expert in typingExperts" :key="expert.id" class="ws-collab-msg assistant ws-typing">
            <div class="ws-collab-msg-avatar gradient-avatar" :style="{ background: expert.color }">
              {{ expert.avatar }}
            </div>
            <div class="ws-collab-msg-content">
              <div class="ws-collab-msg-meta">
                <span class="ws-collab-msg-name">{{ expert.name }}</span>
                <span class="ws-msg-status status-thinking" title="正在思考">
                  <el-icon class="pulse-icon"><Loading /></el-icon>
                </span>
              </div>
              <div class="ws-typing-dots">
                <span></span><span></span><span></span>
              </div>
            </div>
          </div>
          <div v-if="allianceRunning && typingExperts.length === 0" class="ws-collab-msg assistant ws-typing">
            <div class="ws-collab-msg-avatar gradient-avatar" style="background: linear-gradient(135deg, #7c3aed, #06b6d4)">
              🤖
            </div>
            <div class="ws-collab-msg-content">
              <div class="ws-collab-msg-meta">
                <span class="ws-collab-msg-name">AI 协作中</span>
              </div>
              <div class="ws-typing-dots">
                <span></span><span></span><span></span>
              </div>
            </div>
          </div>
        </el-scrollbar>

        <div class="ws-collab-input-area">
          <div v-if="dragOver" class="ws-drop-zone"
            @dragover.prevent="dragOver = true"
            @dragleave="dragOver = false"
            @drop.prevent="handleFileDrop"
          >
            <el-icon class="drop-zone-icon"><Upload /></el-icon>
            <div class="drop-zone-text">释放文件以上传</div>
          </div>

          <div class="ws-collab-input-tools">
            <el-upload
              :show-file-list="false"
              :before-upload="handleBeforeFileUpload"
              multiple
              class="ws-upload-trigger"
            >
              <el-button size="small" text class="ws-tool-mini-btn" title="上传文件">
                <el-icon><Paperclip /></el-icon>
              </el-button>
            </el-upload>
            <el-button size="small" text class="ws-tool-mini-btn" title="引用图谱节点" @click="$emit('insert-node-ref')">
              <el-icon><Share /></el-icon>
            </el-button>
            <el-button size="small" text class="ws-tool-mini-btn" title="添加到白板" @click="$emit('send-to-whiteboard')">
              <el-icon><CollectionTag /></el-icon>
            </el-button>
            <el-select :model-value="collabMode" size="small" class="ws-mode-select" @update:model-value="emit('update:collabMode', $event)" @change="$emit('collab-mode-change')">
              <el-option label="智能路由" value="smart" />
              <el-option label="单专家咨询" value="single" />
              <el-option label="多专家协同" value="multi" />
              <el-option label="专家辩论" value="debate" />
              <el-option label="算法分析" value="algorithm" />
            </el-select>
          </div>
          <div class="ws-collab-input-row">
            <el-input
              :model-value="collabInput"
              class="ws-collab-input-field"
              type="textarea"
              :rows="2"
              placeholder="输入问题或指令… (Enter 发送，Shift+Enter 换行)"
              resize="none"
              @update:model-value="emit('update:collabInput', $event)"
              @keydown.enter.exact.prevent="$emit('send-msg')"
            />
            <el-button type="primary" class="ws-send-btn gradient-btn" @click="$emit('send-msg')" :loading="allianceRunning">
              <el-icon v-if="!allianceRunning"><Promotion /></el-icon>
              <span>{{ allianceRunning ? '运行中' : '发送' }}</span>
            </el-button>
            <el-button v-if="allianceRunning" type="danger" plain class="ws-stop-btn" @click="$emit('stop-alliance')">
              <el-icon><Close /></el-icon>
              停止
            </el-button>
          </div>
        </div>
      </div>

      <!-- ===== 白板 Tab ===== -->
      <div v-show="activeCollabTab === 'whiteboard'" class="ws-tab-content ws-whiteboard-content">
        <WhiteboardPanel
          :active-wb-tool="activeWbTool"
          :active-wb-color="activeWbColor"
          :wb-notes="wbNotes"
          :wb-texts="wbTexts"
          :wb-lines="wbLines"
          :wb-draw-paths="wbDrawPaths"
          :wb-current-path="wbCurrentPath"
          :wb-view-box="wbViewBox"
          @select-wb-tool="$emit('select-wb-tool', $event)"
          @update:activeWbColor="$emit('update:activeWbColor', $event)"
          @clear-whiteboard="$emit('clear-whiteboard')"
          @wb-mousedown="$emit('wb-mousedown', $event)"
          @wb-mousemove="$emit('wb-mousemove', $event)"
          @wb-mouseup="$emit('wb-mouseup', $event)"
          @start-drag-note="$emit('start-drag-note', $event)"
          @delete-wb-note="$emit('delete-wb-note', $event)"
          @update-note-content="$emit('update-note-content', $event)"
          @start-drag-text="$emit('start-drag-text', $event)"
          @delete-wb-text="$emit('delete-wb-text', $event)"
          @update-text-content="$emit('update-text-content', $event)"
          @save-whiteboard="$emit('save-whiteboard')"
        />
      </div>

      <!-- ===== 文件 Tab ===== -->
      <div v-show="activeCollabTab === 'files'" class="ws-tab-content ws-files-content">
        <FilePanel
          :shared-files="sharedFiles"
          @preview-file="$emit('preview-file', $event)"
          @download-file="$emit('download-file', $event)"
          @file-uploaded="$emit('file-uploaded', $event)"
        />
      </div>

      <!-- 历史记录侧边栏 -->
      <HistoryPanel
        :visible="historyPanelOpen"
        :history-events="historyEvents"
        @close="$emit('toggle-history')"
        @jump-to-history="$emit('jump-to-history', $event)"
      />
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import {
  Promotion, ChatLineSquare, RefreshRight, ArrowDown, ArrowUp,
  CircleCheckFilled, FolderOpened, Download, Loading, Warning,
  Upload, Paperclip, Share, CollectionTag, Close
} from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import WhiteboardPanel from './WhiteboardPanel.vue'
import FilePanel from './FilePanel.vue'
import HistoryPanel from './HistoryPanel.vue'

const props = defineProps({
  expanded: { type: Boolean, default: true },
  allianceRunning: { type: Boolean, default: false },
  modeTransitioning: { type: Boolean, default: false },
  activeSession: { type: Object, default: null },
  currentPhaseLabel: { type: String, default: '准备中' },
  collabMessages: { type: Array, default: () => [] },
  typingExperts: { type: Array, default: () => [] },
  projectPhases: { type: Array, default: () => [] },
  currentProjectPhase: { type: Number, default: 0 },
  collabTabs: { type: Array, default: () => [] },
  activeCollabTab: { type: String, default: 'discussion' },
  collabMembers: { type: Array, default: () => [] },
  sharedFiles: { type: Array, default: () => [] },
  historyPanelOpen: { type: Boolean, default: false },
  historyEvents: { type: Array, default: () => [] },
  collabInput: { type: String, default: '' },
  collabMode: { type: String, default: 'smart' },
  // Whiteboard props
  activeWbTool: { type: String, default: 'select' },
  activeWbColor: { type: String, default: '#7c3aed' },
  wbNotes: { type: Array, default: () => [] },
  wbTexts: { type: Array, default: () => [] },
  wbLines: { type: Array, default: () => [] },
  wbDrawPaths: { type: Array, default: () => [] },
  wbCurrentPath: { type: String, default: '' },
  wbViewBox: { type: String, default: '0 0 800 400' }
})

const emit = defineEmits([
  'toggle-expand', 'toggle-history', 'jump-to-phase', 'update:activeCollabTab',
  'preview-file', 'download-file', 'file-uploaded', 'insert-node-ref',
  'send-to-whiteboard', 'collab-mode-change', 'send-msg', 'stop-alliance',
  'jump-to-history', 'update:collabInput', 'update:collabMode',
  // Whiteboard events
  'select-wb-tool', 'update:activeWbColor', 'clear-whiteboard',
  'wb-mousedown', 'wb-mousemove', 'wb-mouseup',
  'start-drag-note', 'delete-wb-note', 'update-note-content',
  'start-drag-text', 'delete-wb-text', 'update-text-content',
  'save-whiteboard'
])

const filesBarExpanded = ref(true)
const dragOver = ref(false)

function memberStatusText(status) {
  const map = { active: '在线', busy: '忙碌', offline: '离线', idle: '空闲' }
  return map[status] || '在线'
}

function msgStatusText(status) {
  const map = { sent: '已发送', thinking: '正在思考', done: '已完成', failed: '失败' }
  return map[status] || ''
}

function phaseLabel(phase) {
  const alliancePhases = [
    { key: 'intent', label: '意图识别' },
    { key: 'team', label: '组队匹配' },
    { key: 'debate', label: '专家辩论' },
    { key: 'synthesize', label: '综合归纳' },
    { key: 'gate', label: '质量把关' },
    { key: 'learn', label: '知识学习' },
    { key: 'done', label: '完成' }
  ]
  const p = alliancePhases.find(p => p.key === phase)
  return p?.label || phase
}

function phaseTagType(phase) {
  const types = {
    intent: 'info', team: 'primary', debate: 'warning',
    synthesize: 'success', gate: 'danger', learn: '', done: 'success'
  }
  return types[phase] || 'info'
}

function formatMessageText(text) {
  if (!text) return ''
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/\n/g, '<br/>')
}

function fileIconEmoji(type) {
  const icons = { pdf: '📕', doc: '📘', image: '🖼️', excel: '📗', ppt: '📙', zip: '📦', code: '💻', other: '📄' }
  return icons[type] || '📄'
}

function getFileType(filename) {
  const ext = filename.split('.').pop()?.toLowerCase()
  if (['pdf'].includes(ext)) return 'pdf'
  if (['doc', 'docx', 'txt', 'md'].includes(ext)) return 'doc'
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext)) return 'image'
  if (['xls', 'xlsx', 'csv'].includes(ext)) return 'excel'
  if (['ppt', 'pptx'].includes(ext)) return 'ppt'
  if (['zip', 'rar', '7z', 'tar', 'gz'].includes(ext)) return 'zip'
  if (['js', 'ts', 'py', 'java', 'go', 'cpp', 'html', 'css', 'vue', 'json'].includes(ext)) return 'code'
  return 'other'
}

function formatFileSize(bytes) {
  if (!bytes) return '未知'
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / 1048576).toFixed(1) + ' MB'
}

function handleBeforeFileUpload(file) {
  const type = getFileType(file.name)
  const newFile = {
    id: 'f-' + Date.now(),
    name: file.name,
    type: type,
    size: formatFileSize(file.size),
    uploader: '我',
    time: '刚刚'
  }
  emit('file-uploaded', newFile)
  ElMessage.success(`文件「${file.name}」上传成功`)
  return false
}

function handleFileDrop(e) {
  dragOver.value = false
  const files = e.dataTransfer?.files
  if (files && files.length > 0) {
    Array.from(files).forEach(file => {
      handleBeforeFileUpload(file)
    })
  }
}
</script>
