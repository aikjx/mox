<!-- AI 对话面板组件 - 可复用，支持 compact 嵌入模式和 full 完整模式 -->
<template>
  <div class="ai-chat-panel" :class="`mode-${mode}`">
    <!-- 对话消息区 -->
    <div v-if="!compactHeader" class="chat-header">
      <div class="chat-header-left">
        <div class="assistant-avatar" :style="{ background: aiStore.currentAssistantObj.gradient }">
          <span>{{ aiStore.currentAssistantObj.emoji }}</span>
        </div>
        <div class="assistant-info">
          <div class="assistant-name">{{ aiStore.currentAssistantObj.name }}</div>
          <div class="assistant-desc">{{ aiStore.currentAssistantObj.description }}</div>
        </div>
      </div>
      <div class="chat-header-right">
        <el-tag v-if="aiStore.isStreaming" type="primary" effect="light" size="small">
          <el-icon class="spin" style="margin-right:4px"><Loading /></el-icon>
          生成中
        </el-tag>
        <el-tag v-else-if="aiStore.isLoading" type="info" effect="light" size="small">
          思考中
        </el-tag>
        <el-button text size="small" @click="handleNewSession" title="新对话">
          <el-icon><Plus /></el-icon>
        </el-button>
        <el-button text size="small" @click="handleClear" title="清空">
          <el-icon><Delete /></el-icon>
        </el-button>
      </div>
    </div>

    <!-- 空状态 -->
    <div v-if="!aiStore.hasMessages" class="chat-empty">
      <div class="empty-icon" :style="{ background: aiStore.currentAssistantObj.gradient }">
        <span>{{ aiStore.currentAssistantObj.emoji }}</span>
      </div>
      <h3 class="empty-title">你好，我是{{ aiStore.currentAssistantObj.name }}</h3>
      <p class="empty-desc">{{ aiStore.currentAssistantObj.description }}</p>
      <div v-if="suggestions.length" class="empty-suggestions">
        <div
          v-for="(s, i) in suggestions"
          :key="i"
          class="suggestion-chip"
          @click="sendSuggestion(s)"
        >
          {{ s }}
        </div>
      </div>
    </div>

    <!-- 消息列表 -->
    <el-scrollbar v-else ref="scrollRef" class="chat-scroll">
      <div class="chat-messages">
        <div
          v-for="msg in aiStore.messages"
          :key="msg.id"
          class="message-row"
          :class="[msg.role, { error: msg.error }]"
        >
          <div v-if="msg.role === 'assistant'" class="msg-avatar ai" :style="{ background: aiStore.currentAssistantObj.gradient }">
            <span class="avatar-emoji">{{ aiStore.currentAssistantObj.emoji }}</span>
          </div>

          <div class="msg-content">
            <div v-if="msg.role === 'assistant'" class="msg-sender">
              {{ aiStore.currentAssistantObj.name }}
            </div>

            <div v-if="msg.role === 'user'" class="msg-bubble user-bubble">
              {{ msg.content }}
            </div>

            <div v-else class="msg-bubble ai-bubble">
              <div class="ai-msg-body" v-html="renderedContent(msg.content)"></div>
              <div v-if="msg.content === '' && aiStore.isStreaming" class="typing-indicator">
                <span class="typing-dot"></span>
                <span class="typing-dot"></span>
                <span class="typing-dot"></span>
              </div>
            </div>

            <div v-if="msg.role === 'assistant' && msg.content" class="msg-actions">
              <el-button text size="small" @click="copyMsg(msg.content)">
                <el-icon><CopyDocument /></el-icon>
              </el-button>
              <el-button text size="small" @click="handleRegenerate" title="重新生成">
                <el-icon><Refresh /></el-icon>
              </el-button>
            </div>
          </div>

          <div v-if="msg.role === 'user'" class="msg-avatar user">
            <el-icon><User /></el-icon>
          </div>
        </div>
      </div>
    </el-scrollbar>

    <!-- 输入区 -->
    <div class="chat-input-area" :class="{ 'has-sessions': aiStore.hasMessages }">
      <div class="input-wrap">
        <el-input
          v-model="inputText"
          type="textarea"
          :rows="1"
          :autosize="{ minRows: 1, maxRows: maxInputRows }"
          :placeholder="placeholder"
          resize="none"
          class="chat-input"
          :disabled="aiStore.isLoading"
          @keydown.enter.exact.prevent="handleSend"
        />
        <div class="input-actions">
          <el-button
            v-if="aiStore.isStreaming"
            type="danger"
            size="small"
            class="stop-btn"
            @click="handleStop"
          >
            <el-icon><VideoPause /></el-icon>
            停止
          </el-button>
          <el-button
            v-else
            type="primary"
            size="small"
            class="send-btn"
            @click="handleSend"
            :disabled="!inputText.trim() || aiStore.isLoading"
          >
            <el-icon><Promotion /></el-icon>
            发送
          </el-button>
        </div>
      </div>
      <div v-if="showHint" class="input-hint">
        Enter 发送 · Shift + Enter 换行
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, nextTick, computed } from 'vue'
import {
  Plus, Delete, User, CopyDocument, Refresh, Loading,
  Promotion, VideoPause
} from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { useAIStore } from '@/stores/ai.store'
import { renderMarkdown } from '@/utils/markdown'

const props = defineProps({
  mode: { type: String, default: 'full' }, // 'full' | 'compact'
  compactHeader: { type: Boolean, default: false },
  placeholder: { type: String, default: '输入你的问题…  Enter 发送，Shift + Enter 换行' },
  showHint: { type: Boolean, default: true },
  maxInputRows: { type: Number, default: 4 },
  suggestions: { type: Array, default: () => [] }
})

const emit = defineEmits(['send', 'new-session', 'mode-change'])

const aiStore = useAIStore()

const inputText = ref('')
const scrollRef = ref(null)

function renderedContent(content) {
  return renderMarkdown(content || '')
}

function scrollToBottom() {
  nextTick(() => {
    if (scrollRef.value) {
      const wrap = scrollRef.value.$el || scrollRef.value
      const scrollbar = wrap.querySelector('.el-scrollbar__wrap') || wrap
      scrollbar.scrollTop = scrollbar.scrollHeight
    }
  })
}

async function handleSend() {
  const text = inputText.value.trim()
  if (!text) return
  if (aiStore.isLoading) return

  inputText.value = ''
  emit('send', text)
  await aiStore.sendMessage(text, { stream: true })
  scrollToBottom()
}

function handleStop() {
  aiStore.stopGeneration()
  ElMessage.info('已停止生成')
}

function sendSuggestion(text) {
  inputText.value = text
  handleSend()
}

function handleNewSession() {
  aiStore.newSession()
  inputText.value = ''
  emit('new-session')
}

function handleClear() {
  aiStore.clearCurrentSession()
}

function handleRegenerate() {
  aiStore.regenerate()
  scrollToBottom()
}

function copyMsg(content) {
  navigator.clipboard.writeText(content).then(() => {
    ElMessage.success('已复制')
  }).catch(() => {
    ElMessage.warning('复制失败')
  })
}

// 自动滚动
watch(
  () => aiStore.messages.length,
  () => scrollToBottom()
)
watch(
  () => aiStore.messages[aiStore.messages.length - 1]?.content,
  () => scrollToBottom()
)
</script>

<style scoped>
.ai-chat-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-card);
  min-height: 0;
}

/* 头部 */
.chat-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-ghost);
  flex-shrink: 0;
}

.chat-header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.assistant-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 18px;
}

.assistant-info {
  display: flex;
  flex-direction: column;
}

.assistant-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.3;
}

.assistant-desc {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
}

.chat-header-right {
  display: flex;
  align-items: center;
  gap: 4px;
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* 空状态 */
.chat-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 20px;
  text-align: center;
}

.empty-icon {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 28px;
  margin-bottom: 16px;
}

.empty-title {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0 0 6px;
}

.empty-desc {
  font-size: 13px;
  color: #64748b;
  margin: 0 0 20px;
  max-width: 320px;
}

.empty-suggestions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
  max-width: 400px;
}

.suggestion-chip {
  padding: 6px 14px;
  background: var(--bg-tertiary);
  border-radius: 16px;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.suggestion-chip:hover {
  background: var(--accent-dim);
  color: #6366f1;
}

/* 消息列表 */
.chat-scroll {
  flex: 1;
  overflow: hidden;
  min-height: 0;
}

.chat-messages {
  padding: 16px;
}

.message-row {
  display: flex;
  gap: 10px;
  margin-bottom: 16px;
  align-items: flex-start;
}

.message-row.user {
  flex-direction: row-reverse;
}

.msg-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  flex-shrink: 0;
  font-size: 14px;
}

.msg-avatar.ai {
  color: #fff;
}

.msg-avatar.user {
  background: var(--bg-tertiary);
  color: var(--text-secondary);
}

.avatar-emoji {
  font-size: 16px;
}

.msg-content {
  max-width: calc(100% - 42px);
  min-width: 0;
}

.message-row.user .msg-content {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
}

.msg-sender {
  font-size: 11px;
  font-weight: 600;
  color: #64748b;
  margin-bottom: 3px;
  padding-left: 2px;
}

.msg-bubble {
  padding: 10px 14px;
  border-radius: 10px;
  line-height: 1.6;
  font-size: 13px;
  word-break: break-word;
}

.user-bubble {
  background: #6366f1;
  color: #fff;
  border-bottom-right-radius: 3px;
}

.ai-bubble {
  background: var(--bg-tertiary);
  color: var(--text-primary);
  border: 1px solid var(--border);
  border-bottom-left-radius: 3px;
}

.ai-msg-body :deep(h1),
.ai-msg-body :deep(h2),
.ai-msg-body :deep(h3),
.ai-msg-body :deep(h4) {
  margin: 12px 0 6px;
  font-weight: 700;
  color: var(--text-primary);
}

.ai-msg-body :deep(h1) { font-size: 16px; }
.ai-msg-body :deep(h2) { font-size: 15px; }
.ai-msg-body :deep(h3) { font-size: 14px; }
.ai-msg-body :deep(h4) { font-size: 13px; }

.ai-msg-body :deep(p) {
  margin: 6px 0;
}

.ai-msg-body :deep(ul),
.ai-msg-body :deep(ol) {
  margin: 6px 0;
  padding-left: 20px;
}

.ai-msg-body :deep(li) {
  margin: 3px 0;
}

.ai-msg-body :deep(code) {
  background: var(--bg-tertiary);
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 12px;
  font-family: 'SF Mono', 'Fira Code', monospace;
  color: #be123c;
}

.ai-msg-body :deep(pre) {
  background: #0f172a;
  color: #e2e8f0;
  padding: 10px 14px;
  border-radius: 6px;
  overflow-x: auto;
  margin: 8px 0;
}

.ai-msg-body :deep(pre code) {
  background: transparent;
  color: #e2e8f0;
  padding: 0;
  font-size: 12px;
}

.ai-msg-body :deep(a) {
  color: #6366f1;
  text-decoration: none;
}

.ai-msg-body :deep(a:hover) {
  text-decoration: underline;
}

.ai-msg-body :deep(blockquote) {
  border-left: 3px solid #c7d2fe;
  margin: 6px 0;
  padding-left: 10px;
  color: #64748b;
}

.ai-msg-body :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 8px 0;
}

.ai-msg-body :deep(th),
.ai-msg-body :deep(td) {
  border: 1px solid var(--border);
  padding: 6px 10px;
  text-align: left;
  font-size: 12px;
}

.ai-msg-body :deep(th) {
  background: var(--bg-tertiary);
  font-weight: 600;
}

.typing-indicator {
  display: flex;
  gap: 3px;
  padding: 4px 0;
}

.typing-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #94a3b8;
  animation: typing 1.4s infinite ease-in-out;
}

.typing-dot:nth-child(2) { animation-delay: 0.2s; }
.typing-dot:nth-child(3) { animation-delay: 0.4s; }

@keyframes typing {
  0%, 60%, 100% { transform: translateY(0); opacity: 0.5; }
  30% { transform: translateY(-5px); opacity: 1; }
}

.msg-actions {
  display: flex;
  gap: 2px;
  margin-top: 4px;
  padding-left: 2px;
  opacity: 0;
  transition: opacity 0.15s;
}

.message-row:hover .msg-actions {
  opacity: 1;
}

.message-row.error .ai-bubble {
  border-color: #fecaca;
  background: #fef2f2;
}

/* 输入区 */
.chat-input-area {
  padding: 10px 12px 12px;
  border-top: 1px solid var(--border-ghost);
  flex-shrink: 0;
}

.input-wrap {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 6px 6px 6px 12px;
  transition: all 0.15s;
}

.input-wrap:focus-within {
  border-color: #6366f1;
  background: var(--bg-card);
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
}

.chat-input {
  flex: 1;
  border: none;
}

.chat-input :deep(.el-textarea__inner) {
  border: none !important;
  box-shadow: none !important;
  background: transparent !important;
  padding: 4px 0;
  font-size: 13px;
  line-height: 1.5;
  resize: none;
  min-height: auto !important;
}

.input-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.send-btn, .stop-btn {
  height: 32px;
  border-radius: 6px;
  padding: 0 12px;
}

.input-hint {
  text-align: center;
  font-size: 10px;
  color: #94a3b8;
  margin-top: 6px;
}

/* compact 模式 */
.mode-compact .chat-messages {
  padding: 12px;
}

.mode-compact .msg-bubble {
  padding: 8px 12px;
  font-size: 12px;
}

.mode-compact .empty-title {
  font-size: 15px;
}

.mode-compact .empty-icon {
  width: 44px;
  height: 44px;
  font-size: 22px;
  margin-bottom: 12px;
}

.mode-compact .input-hint {
  display: none;
}
</style>
