<template>
  <div class="chat">
    <div class="chat-head">
      <div class="title">
        <el-icon><ChatDotRound /></el-icon>
        <span>AI 智能对话</span>
      </div>
      <div class="subtitle">基于六大公理的统一算子智能体 · 会话 {{ sessionShort }}</div>
    </div>

    <div class="messages" ref="scroll">
      <div v-if="!messages.length" class="empty">
        <el-icon class="empty-icon"><Cpu /></el-icon>
        <p>我是算子统一系统智能体，可以帮你：</p>
        <ul>
          <li>查询 / 推荐算子</li>
          <li>编排并执行业务流程</li>
          <li>分析算法、可视化知识图谱</li>
        </ul>
        <div class="quick">
          <el-button
            v-for="q in quickStarters"
            :key="q"
            size="small"
            round
            @click="send(q)"
          >{{ q }}</el-button>
        </div>
      </div>

      <MessageBubble v-for="m in messages" :key="m.id || m._k" :msg="m" />

      <div v-if="loading" class="typing">
        <span class="dot"></span><span class="dot"></span><span class="dot"></span>
        <span class="t">智能体正在推理…</span>
      </div>
    </div>

    <!-- 快捷操作卡 -->
    <div v-if="pendingActions.length" class="actions">
      <div class="actions-title">可执行操作</div>
      <div class="action-cards">
        <div
          v-for="a in pendingActions"
          :key="a.id"
          class="action-card"
          @click="onAction(a)"
        >
          <el-icon><MagicStick /></el-icon>
          <div class="a-body">
            <div class="a-label">{{ a.label }}</div>
            <div class="a-type">{{ a.action_type }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 推荐算子 -->
    <div v-if="recommended.length" class="reco">
      <span class="reco-label">推荐算子：</span>
      <span v-for="op in recommended" :key="op" class="tag">{{ op }}</span>
    </div>

    <!-- 建议问题 -->
    <div v-if="suggestions.length" class="suggest">
      <el-tag
        v-for="s in suggestions"
        :key="s"
        class="suggest-tag"
        @click="send(s)"
      >{{ s }}</el-tag>
    </div>

    <div class="input-bar">
      <el-input
        v-model="draft"
        type="textarea"
        :rows="2"
        resize="none"
        placeholder="输入你的问题，例如：帮我用 linear + relu 编排一个流程"
        @keydown.enter.exact.prevent="submit"
      />
      <el-button type="primary" :loading="loading" @click="submit" class="send-btn">
        <el-icon><Promotion /></el-icon>发送
      </el-button>
    </div>
  </div>
</template>

<script setup>
import { ref, nextTick, computed, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { ChatDotRound, Cpu, MagicStick, Promotion } from '@element-plus/icons-vue'
import MessageBubble from '@/components/MessageBubble.vue'
import { aiChat } from '@/api'
import { Role } from '@/types'

const props = defineProps({
  sessionId: { type: String, default: '' },
})
const emit = defineEmits(['action', 'thinking'])

const messages = ref([])
const draft = ref('')
const loading = ref(false)
const suggestions = ref([])
const recommended = ref([])
const pendingActions = ref([])
const scroll = ref(null)

const sessionShort = computed(() => (props.sessionId ? props.sessionId.slice(0, 8) : '新会话'))
const quickStarters = ['列出所有算子', '推荐一个分类流程', '展示知识图谱', '帮我分析一个算法']

let _k = 0
function pushUser(text) {
  messages.value.push({
    _k: ++_k,
    id: '',
    role: Role.User,
    content: text,
    timestamp: new Date().toISOString(),
    metadata: {},
    referenced_operators: [],
  })
}
function pushAssistant(resp) {
  messages.value.push({
    _k: ++_k,
    ...resp.message,
    role: resp.message.role || Role.Assistant,
  })
  suggestions.value = resp.suggestions || []
  recommended.value = resp.recommended_operators || []
  pendingActions.value = resp.actions || []
  if (resp.workflow_suggestion) {
    pendingActions.value.push({
      id: 'wf-' + Date.now(),
      label: '执行推荐流程：' + resp.workflow_suggestion.join(' → '),
      action_type: 'execute_workflow',
      payload: { operators: resp.workflow_suggestion },
    })
  }
}

async function submit() {
  const text = draft.value.trim()
  if (!text || loading.value) return
  draft.value = ''
  await send(text)
}

async function send(text) {
  if (!text || loading.value) return
  suggestions.value = []
  recommended.value = []
  pendingActions.value = []
  pushUser(text)
  loading.value = true
  emit('thinking', true)
  await scrollBottom()
  try {
    const resp = await aiChat(props.sessionId || null, text)
    pushAssistant(resp)
  } catch (e) {
    ElMessage.error(e.message || '对话失败')
  } finally {
    loading.value = false
    emit('thinking', false)
    await scrollBottom()
  }
}

function onAction(a) {
  emit('action', a)
  pendingActions.value = pendingActions.value.filter((x) => x.id !== a.id)
}

async function scrollBottom() {
  await nextTick()
  if (scroll.value) scroll.value.scrollTop = scroll.value.scrollHeight
}

watch(
  () => props.sessionId,
  () => {
    messages.value = []
    suggestions.value = []
    recommended.value = []
    pendingActions.value = []
  }
)

defineExpose({ send })
</script>

<style scoped>
.chat {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
}
.chat-head {
  padding: 14px 18px;
  border-bottom: 1px solid var(--border);
}
.title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 700;
}
.subtitle {
  font-size: 12px;
  color: var(--text-dim);
  margin-top: 2px;
}
.messages {
  flex: 1;
  overflow-y: auto;
  padding: 8px 18px;
}
.empty {
  text-align: center;
  color: var(--text-dim);
  margin-top: 12vh;
}
.empty-icon {
  font-size: 46px;
  color: var(--primary);
  margin-bottom: 8px;
}
.empty ul {
  list-style: none;
  padding: 0;
  margin: 10px 0;
  font-size: 13px;
}
.empty li {
  margin: 4px 0;
}
.quick {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
  margin-top: 14px;
}
.typing {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 10px 4px;
  color: var(--text-dim);
  font-size: 13px;
}
.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--primary-2);
  animation: blink 1.2s infinite both;
}
.dot:nth-child(2) { animation-delay: 0.2s; }
.dot:nth-child(3) { animation-delay: 0.4s; }
.t { margin-left: 6px; }
@keyframes blink {
  0%, 80%, 100% { opacity: 0.25; }
  40% { opacity: 1; }
}
.actions {
  padding: 6px 18px;
  border-top: 1px solid var(--border);
}
.actions-title {
  font-size: 12px;
  color: var(--text-dim);
  margin-bottom: 6px;
}
.action-cards {
  display: flex;
  gap: 10px;
  overflow-x: auto;
  padding-bottom: 4px;
}
.action-card {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--bg-panel-2);
  border: 1px solid var(--border);
  border-radius: 10px;
  cursor: pointer;
  transition: 0.15s;
}
.action-card:hover {
  border-color: var(--primary);
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
}
.a-body { text-align: left; }
.a-label { font-size: 13px; }
.a-type { font-size: 11px; color: var(--text-dim); }
.reco {
  padding: 6px 18px;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.reco-label { font-size: 12px; color: var(--text-dim); }
.suggest {
  padding: 0 18px 8px;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.suggest-tag {
  cursor: pointer;
}
.input-bar {
  padding: 12px 18px 16px;
  border-top: 1px solid var(--border);
  display: flex;
  gap: 10px;
  align-items: flex-end;
}
.send-btn {
  height: 52px;
}
</style>
