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
          <el-tooltip content="从后端恢复会话历史（跨设备同步）" placement="bottom">
            <el-button text @click="openBackendHistory"><el-icon><Clock /></el-icon></el-button>
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

      <!-- 后端历史恢复 -->
      <el-dialog v-model="historyOpen" title="从后端恢复会话" width="440px">
        <div class="hist-tip">这些会话由后端持久化（跨设备共享），点击即可载入对话历史。</div>
        <el-empty v-if="!backendSessions.length" description="暂无后端会话" :image-size="60" />
        <div v-else class="hist-list">
          <div
            class="hist-item"
            v-for="s in backendSessions"
            :key="s.id"
            @click="restoreFromBackend(s)"
          >
            <div class="hist-title">{{ s.title || s.id }}</div>
            <div class="hist-meta">{{ s.id }} · {{ s.updated_at || '' }}</div>
          </div>
        </div>
      </el-dialog>

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
import { ref, nextTick, onMounted, onUnmounted, watch } from 'vue'
import { ElMessage } from 'element-plus'
import MessageBubble from '@/components/MessageBubble.vue'
import SessionSidebar from '@/components/SessionSidebar.vue'
import {
  aiChat,
  getAutoSyncStatus,
  toggleAutoSync,
  graphExport,
  graphImport,
  listDialogueSessions,
  getChatHistory
} from '@/api'

const sessions = ref([])
const currentSession = ref(null)
const messages = ref([])
// 所有会话的消息映射（单一事实源）：切换会话不再丢失历史
const messagesMap = ref({})
const draft = ref('')
const thinking = ref(false)
const online = ref(false)
const scrollEl = ref(null)
// 对话自动→知识图谱 全自动同步开关（默认开）
const autoSync = ref(true)
const importInput = ref(null)
// 流式打字定时器句柄：组件卸载时必须清理，避免泄漏
let streamTimer = null

// 后端会话历史（跨设备恢复）
const historyOpen = ref(false)
const backendSessions = ref([])
const loadingHistory = ref(false)
async function openBackendHistory() {
  historyOpen.value = true
  if (backendSessions.value.length) return
  loadingHistory.value = true
  try {
    const r = await listDialogueSessions()
    backendSessions.value = r.sessions || []
  } catch (e) {
    ElMessage.error('后端会话加载失败：' + e.message)
  } finally {
    loadingHistory.value = false
  }
}
async function restoreFromBackend(s) {
  if (loadingHistory.value) return
  loadingHistory.value = true
  try {
    const msgs = await getChatHistory(s.id)
    const list = Array.isArray(msgs) ? msgs : []
    if (!list.length) {
      ElMessage.info('该会话暂无后端聊天记录（仅自动入图会话会持久化）')
      return
    }
    // 后端 ChatMessage 转为前端 MessageBubble 格式
    setMessages(list.map((m) => ({
      role: String(m.role || '').toLowerCase() === 'user' ? 'user' : 'assistant',
      content: m.content || '',
      timestamp: m.timestamp || Date.now(),
      referenced_operators: m.referenced_operators || [],
      confidence: m.metadata && m.metadata.confidence != null
        ? m.metadata.confidence
        : undefined,
    })))
    // 同步到本地会话列表，保证可切换
    if (!sessions.value.find((x) => x.id === s.id)) {
      sessions.value.unshift({
        id: s.id,
        title: s.title || s.id,
        time: (s.updated_at || '').slice(0, 16).replace('T', ' '),
      })
    }
    currentSession.value = s.id
    persist()
    historyOpen.value = false
    ElMessage.success(`已恢复 ${messages.value.length} 条历史消息`)
    await scroll()
  } catch (e) {
    ElMessage.error('恢复失败：' + e.message)
  } finally {
    loadingHistory.value = false
  }
}

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
        messagesMap.value = data.messages || {}
        messages.value = messagesMap.value[cur] || []
      }
    }
  } catch (e) { /* ignore */ }
}
// 同时写入当前会话消息与映射，保证两者引用一致
function setMessages(arr) {
  messages.value = arr
  messagesMap.value = { ...messagesMap.value, [currentSession.value]: arr }
}
function persist() {
  try {
    const msgs = {}
    for (const s of sessions.value) msgs[s.id] = messagesMap.value[s.id] || []
    localStorage.setItem(
      STORE_KEY,
      JSON.stringify({
        sessions: sessions.value,
        current: currentSession.value,
        messages: msgs
      })
    )
  } catch (e) { /* ignore */ }
}

function newSession() {
  const id = 's-' + Math.random().toString(36).slice(2, 9)
  const s = { id, title: '新会话', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }) }
  sessions.value.unshift(s)
  currentSession.value = id
  setMessages([])
  persist()
}
function selectSession(id) {
  currentSession.value = id
  setMessages(messagesMap.value[id] || [])
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
    streamTimer = setInterval(() => {
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
  setMessages([])
  persist()
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
    .then((r) => { if (r) autoSync.value = !!(r.enabled ?? r.auto_sync ?? r.data?.auto_sync) })
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
    const blob = new Blob([JSON.stringify(res, null, 2)], { type: 'application/json' })
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
    ElMessage.success(`导入完成：会话 ${res.imported.sessions} / 节点 ${res.imported.nodes}`)
  } catch {
    ElMessage.error('导入失败：文件格式不合法')
  } finally {
    e.target.value = ''
  }
}
onUnmounted(() => { if (streamTimer) clearInterval(streamTimer) })
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
.hist-tip {
  font-size: 12px; color: var(--text-3); margin-bottom: 12px; line-height: 1.6;
}
.hist-list { display: flex; flex-direction: column; gap: 8px; max-height: 360px; overflow-y: auto; }
.hist-item {
  border: 1px solid var(--border); border-radius: 10px; padding: 10px 12px; cursor: pointer;
  transition: all 0.2s;
}
.hist-item:hover { border-color: var(--brand); background: var(--brand-soft, #eef4ff); }
.hist-title { font-weight: 700; font-size: 14px; margin-bottom: 4px; }
.hist-meta { font-size: 12px; color: var(--text-3); font-family: var(--font-mono, monospace); }
</style>
