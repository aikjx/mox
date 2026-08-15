<template>
  <div class="chat">
    <SessionSidebar
      :sessions="sessions"
      :active-id="currentSession"
      :online="online"
      @select="selectSession"
      @new="newSession"
    />

    <div class="chat-main">
      <div class="chat-header">
        <div class="chat-title">
          <el-icon><ChatDotRound /></el-icon>
          <span>AI 智能助手</span>
          <span class="badge info">意图识别 · 算子推荐 · 算法分析</span>
        </div>
        <div class="chat-tools">
          <el-tooltip content="对话内容自动整理进知识图谱（全自动）" placement="bottom">
            <el-switch
              v-model="autoSync"
              inline-prompt
              active-text="自动入图"
              inactive-text="手动"
              @change="onToggleAutoSync"
            />
          </el-tooltip>
          <el-tooltip content="导出对话+图谱迁移包" placement="bottom">
            <el-button text @click="exportBundle"><el-icon><Download /></el-icon></el-button>
          </el-tooltip>
          <el-tooltip content="导入迁移包" placement="bottom">
            <el-button text @click="pickImport"><el-icon><Upload /></el-icon></el-button>
          </el-tooltip>
          <input ref="importInput" type="file" accept="application/json" hidden @change="onImportFile" />
          <el-button text @click="clearChat"><el-icon><Delete /></el-icon> 清空</el-button>
        </div>
      </div>

      <div ref="scrollEl" class="chat-body">
        <div v-if="!messages.length" class="empty">
          <div class="empty-orb"><el-icon><ChatLineRound /></el-icon></div>
          <p>我是算子统一系统 AI 助手，可以帮你<b>分析算法</b>、<b>推荐算子</b>、<b>解释图谱</b>。</p>
          <div class="suggestions">
            <el-tag v-for="q in quickQuestions" :key="q" class="q" @click="sendQuick(q)">{{ q }}</el-tag>
          </div>
        </div>
        <template v-else>
          <MessageBubble
            v-for="(m, i) in messages"
            :key="i"
            :msg="m"
          />
        </template>
        <div v-if="thinking" class="typing">
          <span></span><span></span><span></span>
        </div>
      </div>

      <div class="chat-input">
        <el-input
          v-model="draft"
          type="textarea"
          :rows="2"
          resize="none"
          placeholder="输入消息，Enter 发送 / Shift+Enter 换行"
          @keydown.enter.exact.prevent="send"
        />
        <el-button type="primary" :loading="thinking" @click="send">
          <el-icon><Promotion /></el-icon> 发送
        </el-button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, nextTick, onMounted, watch } from 'vue'
import { ElMessage } from 'element-plus'
import MessageBubble from '@/components/MessageBubble.vue'
import SessionSidebar from '@/components/SessionSidebar.vue'
import {
  aiChat,
  getAutoSyncStatus,
  toggleAutoSync,
  graphExport,
  graphImport
} from '@/api'

const sessions = ref([])
const currentSession = ref(null)
const messages = ref([])
const draft = ref('')
const thinking = ref(false)
const online = ref(false)
const scrollEl = ref(null)
// 对话自动→知识图谱 全自动同步开关（默认开）
const autoSync = ref(true)
const importInput = ref(null)

const quickQuestions = [
  '帮我推荐一个归一化算子',
  '解释一下知识图谱的中心性',
  '如何编排一个工作流链？',
  '算法复杂度怎么分析？'
]

const STORE_KEY = 'ous_sessions'

function loadStore() {
  try {
    const raw = localStorage.getItem(STORE_KEY)
    if (raw) {
      const data = JSON.parse(raw)
      sessions.value = data.sessions || []
      const cur = data.current
      if (cur && sessions.value.find((s) => s.id === cur)) {
        currentSession.value = cur
        messages.value = data.messages?.[cur] || []
      }
    }
  } catch (e) { /* ignore */ }
}
function persist() {
  try {
    localStorage.setItem(
      STORE_KEY,
      JSON.stringify({
        sessions: sessions.value,
        current: currentSession.value,
        messages: { [currentSession.value]: messages.value }
      })
    )
  } catch (e) { /* ignore */ }
}

function newSession() {
  const id = 's-' + Math.random().toString(36).slice(2, 9)
  const s = { id, title: '新会话', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }) }
  sessions.value.unshift(s)
  currentSession.value = id
  messages.value = []
  persist()
}
function selectSession(id) {
  currentSession.value = id
  const s = sessions.value.find((x) => x.id === id)
  messages.value = s.messages || []
  persist()
}
async function sendQuick(q) {
  draft.value = q
  await send()
}

async function send() {
  const text = draft.value.trim()
  if (!text || thinking.value) return
  if (!currentSession.value) newSession()
  const userMsg = { role: 'user', content: text, timestamp: Date.now() }
  messages.value.push(userMsg)
  const s = sessions.value.find((x) => x.id === currentSession.value)
  if (s && s.title === '新会话') s.title = text.slice(0, 14)
  draft.value = ''
  thinking.value = true
  await scroll()

  const placeholder = { role: 'assistant', content: '', timestamp: Date.now() }
  messages.value.push(placeholder)
  await scroll()

  try {
    const res = await aiChat({ session_id: currentSession.value, message: text })
    const full = res.response || res.message || '（无回复）'
    online.value = true
    // 流式打字效果
    let i = 0
    const timer = setInterval(() => {
      i += Math.max(2, Math.ceil(full.length / 40))
      placeholder.content = full.slice(0, i)
      scroll()
      if (i >= full.length) {
        clearInterval(timer)
        thinking.value = false
        if (res.referenced_operators) placeholder.referenced_operators = res.referenced_operators
        if (res.confidence != null) placeholder.confidence = res.confidence
        persist()
      }
    }, 28)
  } catch (e) {
    placeholder.content = '⚠️ ' + e.message
    online.value = false
    thinking.value = false
    ElMessage.error(e.message)
    persist()
  }
}

function clearChat() {
  messages.value = []
  persist()
}
function exportChat() {
  const text = messages.value.map((m) => `[${m.role}] ${m.content}`).join('\n\n')
  const blob = new Blob([text], { type: 'text/plain' })
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = 'chat-' + currentSession.value + '.txt'
  a.click()
  URL.revokeObjectURL(a.href)
}
async function scroll() {
  await nextTick()
  if (scrollEl.value) scrollEl.value.scrollTop = scrollEl.value.scrollHeight
}

watch(messages, persist, { deep: true })

onMounted(() => {
  loadStore()
  if (!sessions.value.length) newSession()
  // 拉取后端全自动同步开关状态
  getAutoSyncStatus()
    .then((r) => { if (r && r.data) autoSync.value = !!r.data.auto_sync })
    .catch(() => {})
})

// 切换对话自动入图开关（全自动）
async function onToggleAutoSync(val) {
  try {
    await toggleAutoSync(val)
    ElMessage.success(val ? '已开启：对话自动整理进知识图谱' : '已关闭：手动模式')
  } catch {
    ElMessage.error('切换同步开关失败')
  }
}

// 导出对话 + 知识图谱 为单文件迁移包
async function exportBundle() {
  try {
    const res = await graphExport()
    const blob = new Blob([JSON.stringify(res.data, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `operator-bundle-${new Date().toISOString().slice(0, 10)}.json`
    a.click()
    URL.revokeObjectURL(url)
    ElMessage.success('迁移包已导出（对话 + 知识图谱）')
  } catch {
    ElMessage.error('导出失败')
  }
}

// 选择导入文件
function pickImport() {
  importInput.value?.click()
}

// 导入迁移包（对话 + 图谱）
async function onImportFile(e) {
  const file = e.target.files?.[0]
  if (!file) return
  try {
    const text = await file.text()
    const bundle = JSON.parse(text)
    const res = await graphImport(bundle)
    ElMessage.success(`导入完成：会话 ${res.data.imported.sessions} / 节点 ${res.data.imported.nodes}`)
  } catch {
    ElMessage.error('导入失败：文件格式不合法')
  } finally {
    e.target.value = ''
  }
}
</script>

<style scoped>
.chat {
  display: flex;
  height: calc(100vh - var(--header-h) - 42px - 44px);
  background: #fff;
  border-radius: var(--radius);
  box-shadow: var(--shadow-sm);
  overflow: hidden;
}
.chat-main { flex: 1; display: flex; flex-direction: column; min-width: 0; }
.chat-header {
  height: 56px; display: flex; align-items: center; justify-content: space-between;
  padding: 0 20px; border-bottom: 1px solid var(--border);
}
.chat-title { display: flex; align-items: center; gap: 8px; font-weight: 700; font-size: 15px; }
.chat-tools { display: flex; align-items: center; gap: 4px; }
.chat-body { flex: 1; overflow-y: auto; padding: 20px; background: #f8fafc; }
.empty { text-align: center; color: var(--text-3); margin-top: 50px; }
.empty-orb {
  width: 72px; height: 72px; margin: 0 auto 16px; border-radius: 50%;
  display: grid; place-items: center; font-size: 32px; color: #fff;
  background: linear-gradient(135deg, var(--brand-light), var(--accent));
  box-shadow: 0 10px 30px rgba(99, 102, 241, 0.4);
}
.empty p { max-width: 420px; margin: 0 auto 16px; line-height: 1.7; }
.suggestions { display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; }
.q { cursor: pointer; }
.q:hover { background: var(--brand-soft); color: var(--brand-dark); }

.typing { display: flex; gap: 4px; padding: 12px 16px; width: fit-content; background: #fff; border-radius: 14px; margin-bottom: 14px; }
.typing span { width: 8px; height: 8px; border-radius: 50%; background: var(--text-3); animation: blink 1.2s infinite; }
.typing span:nth-child(2) { animation-delay: 0.2s; }
.typing span:nth-child(3) { animation-delay: 0.4s; }
@keyframes blink { 0%, 60%, 100% { opacity: 0.3; } 30% { opacity: 1; } }

.chat-input { display: flex; gap: 10px; padding: 14px 18px; border-top: 1px solid var(--border); align-items: flex-end; }
.chat-input :deep(.el-textarea) { flex: 1; }
</style>
