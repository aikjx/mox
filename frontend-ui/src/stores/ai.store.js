// AI 对话 Store - 会话管理、消息、流式响应、助手选择
// 支持两种模式：global（全局对话）、project（项目内对话）
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { aiChat, getChatHistory } from '@/api'
import { allianceApi } from '@/modules/expert-alliance/api'
import { COLLAB_MODES, collabMode as collabModeDef, collabProblem } from '@/modules/expert-alliance/contract'
import { collabChatText } from '@/modules/expert-alliance/model'
import { getToken } from '@/utils'

const STORAGE_PREFIX = 'mox.ai.v2'

// 助手定义
export const ASSISTANTS = {
  general: {
    key: 'general', name: '全能助手小通', emoji: '✨',
    gradient: 'linear-gradient(135deg, #ec4899, #8b5cf6)',
    description: '通用对话，可处理各类问题',
    systemPrompt: '你是璇玑系统的全能 AI 助手，帮助用户完成各种任务。'
  },
  architect: {
    key: 'architect', name: '架构师小智', emoji: '🏗️',
    gradient: 'linear-gradient(135deg, #6366f1, #8b5cf6)',
    description: '系统架构设计与技术选型',
    systemPrompt: '你是资深架构师，擅长系统架构设计、技术选型和性能优化。'
  },
  analyst: {
    key: 'analyst', name: '分析师小研', emoji: '📊',
    gradient: 'linear-gradient(135deg, #06b6d4, #0ea5e9)',
    description: '数据分析与研究报告',
    systemPrompt: '你是数据分析师，擅长数据分析、竞品研究和报告撰写。'
  },
  data: {
    key: 'data', name: '数据工程师小数', emoji: '🔗',
    gradient: 'linear-gradient(135deg, #10b981, #059669)',
    description: '数据建模与治理方案',
    systemPrompt: '你是数据工程师，擅长数据建模、数据治理和 ETL 设计。'
  },
  product: {
    key: 'product', name: '产品经理小策', emoji: '💡',
    gradient: 'linear-gradient(135deg, #f59e0b, #f97316)',
    description: '需求分析与产品设计',
    systemPrompt: '你是产品经理，擅长需求分析、产品设计和用户体验。'
  },
  devops: {
    key: 'devops', name: '运维工程师小运', emoji: '⚙️',
    gradient: 'linear-gradient(135deg, #ef4444, #dc2626)',
    description: '运维部署与 CI/CD',
    systemPrompt: '你是运维工程师，擅长部署运维、监控告警和 CI/CD 流水线。'
  }
}

// 咨询模式表：联盟六模式的键/文案/输入框提示全部由模块契约 COLLAB_MODES 投影，此处不再手抄。
// 'general' 是本 store 特有的一档（通用助手对话，不进联盟契约），契约里没有对应模式。
// 原先这里是第 3 份手写副本，且把 'smart' 标成"智能路由"，与契约里 route=智能路由 相互矛盾。
export const CONSULT_MODES = Object.freeze({
  general: {
    key: 'general',
    label: '通用助手',
    desc: '不与专家联盟协作，直接和所选助手对话',
    placeholder: '输入你的问题，按所选助手人格直接回答…'
  },
  ...Object.fromEntries(COLLAB_MODES.map((m) => [m.key, {
    key: m.key,
    label: m.label,
    desc: m.outcome || m.placeholder,
    placeholder: m.placeholder
  }]))
})

function genId(prefix = 'id') {
  return prefix + '_' + Date.now().toString(36) + '_' + Math.random().toString(36).slice(2, 8)
}

function storageKey(scope, projectId) {
  if (scope === 'project' && projectId) {
    return `${STORAGE_PREFIX}.project.${projectId}`
  }
  return `${STORAGE_PREFIX}.global`
}

function currentKey(scope, projectId) {
  return storageKey(scope, projectId) + '.current'
}

export const useAIStore = defineStore('ai', () => {
  // ===== State =====
  const sessions = ref([])
  const currentSessionId = ref(null)
  const messages = ref([])
  const currentAssistant = ref('general')
  const isLoading = ref(false)
  const isStreaming = ref(false)
  const streamController = ref(null)
  const currentScope = ref('global') // 'global' | 'project'
  const currentProjectId = ref(null)
  const consultMode = ref('general') // 咨询模式：'general' 或与契约同名的联盟模式
  const selectedExpertIds = ref([]) // 已选专家 ID

  // ===== Getters =====
  const currentSession = computed(() =>
    sessions.value.find(s => s.id === currentSessionId.value) || null
  )

  const currentAssistantObj = computed(() =>
    ASSISTANTS[currentAssistant.value] || ASSISTANTS.general
  )

  const sortedSessions = computed(() =>
    [...sessions.value].sort((a, b) => (b.updatedAt || 0) - (a.updatedAt || 0))
  )

  const hasMessages = computed(() => messages.value.length > 0)

  const lastUserMessage = computed(() =>
    [...messages.value].reverse().find(m => m.role === 'user')
  )

  const currentConsultMode = computed(() => CONSULT_MODES[consultMode.value] || CONSULT_MODES.general)

  /** 联盟模式判定：契约认识的键才走联盟契约，其余（含 'general' 与助手人格）走通用对话 */
  const allianceDef = computed(() => collabModeDef(consultMode.value))

  // ===== Actions =====

  // 切换作用域（全局/项目）
  function setScope(scope, projectId = null) {
    if (scope === currentScope.value && projectId === currentProjectId.value) return

    // 保存当前作用域数据
    saveToStorage()

    // 切换到新作用域
    currentScope.value = scope
    currentProjectId.value = scope === 'project' ? projectId : null

    // 加载新作用域数据
    loadFromStorage()
  }

  // 从本地存储加载
  function loadFromStorage() {
    try {
      const key = storageKey(currentScope.value, currentProjectId.value)
      const saved = localStorage.getItem(key)
      if (saved) {
        const parsed = JSON.parse(saved)
        sessions.value = parsed.sessions || []
      } else {
        sessions.value = []
      }

      const curKey = currentKey(currentScope.value, currentProjectId.value)
      const savedCurrent = localStorage.getItem(curKey)
      currentSessionId.value = savedCurrent || null

      // 如果有当前会话，加载消息
      if (currentSessionId.value) {
        loadMessages(currentSessionId.value)
      } else {
        messages.value = []
      }
    } catch (e) {
      console.warn('加载 AI 会话失败:', e)
      sessions.value = []
      messages.value = []
      currentSessionId.value = null
    }
  }

  // 保存到本地存储
  function saveToStorage() {
    try {
      const key = storageKey(currentScope.value, currentProjectId.value)
      const meta = sessions.value.map(s => ({
        id: s.id,
        title: s.title,
        subtitle: s.subtitle,
        status: s.status,
        assistant: s.assistant,
        mode: s.mode,
        createdAt: s.createdAt,
        updatedAt: s.updatedAt
      }))
      localStorage.setItem(key, JSON.stringify({ sessions: meta }))

      const curKey = currentKey(currentScope.value, currentProjectId.value)
      if (currentSessionId.value) {
        localStorage.setItem(curKey, currentSessionId.value)
      } else {
        localStorage.removeItem(curKey)
      }
    } catch (e) {
      console.warn('保存 AI 会话失败:', e)
    }
  }

  // 加载消息
  async function loadMessages(sessionId) {
    // 先尝试后端
    try {
      const history = await getChatHistory(sessionId)
      if (Array.isArray(history) && history.length > 0) {
        messages.value = history.map(h => ({
          id: h.id || genId('msg'),
          role: h.role || 'assistant',
          content: h.content || '',
          type: h.type || 'text',
          createdAt: h.created_at || Date.now()
        }))
        return
      }
    } catch {
      // 后端不可用，走本地
    }

    // 本地加载
    try {
      const msgKey = `${STORAGE_PREFIX}.msgs.${sessionId}`
      const saved = localStorage.getItem(msgKey)
      messages.value = saved ? JSON.parse(saved) : []
    } catch {
      messages.value = []
    }
  }

  // 保存消息
  function saveMessages() {
    if (!currentSessionId.value) return
    try {
      const msgKey = `${STORAGE_PREFIX}.msgs.${currentSessionId.value}`
      localStorage.setItem(msgKey, JSON.stringify(messages.value.slice(-100)))
    } catch {}
  }

  // 新建会话
  function newSession(assistantKey = null) {
    const id = genId('sess')
    const assistant = assistantKey || currentAssistant.value
    const session = {
      id,
      title: '新对话',
      subtitle: ASSISTANTS[assistant]?.name || 'AI 助手',
      status: 'running',
      assistant,
      mode: consultMode.value,
      scope: currentScope.value,
      projectId: currentProjectId.value,
      createdAt: Date.now(),
      updatedAt: Date.now()
    }
    sessions.value.unshift(session)
    currentSessionId.value = id
    messages.value = []
    saveToStorage()
    return id
  }

  // 选择会话
  async function selectSession(id) {
    if (currentSessionId.value === id) return
    currentSessionId.value = id
    messages.value = []

    const session = sessions.value.find(s => s.id === id)
    if (session?.assistant) {
      currentAssistant.value = session.assistant
    }
    if (session?.mode) {
      consultMode.value = session.mode
    }

    await loadMessages(id)
    saveToStorage()
  }

  // 删除会话
  function deleteSession(id) {
    const idx = sessions.value.findIndex(s => s.id === id)
    if (idx >= 0) {
      sessions.value.splice(idx, 1)
      try { localStorage.removeItem(`${STORAGE_PREFIX}.msgs.${id}`) } catch {}
    }
    if (currentSessionId.value === id) {
      if (sessions.value.length > 0) {
        selectSession(sessions.value[0].id)
      } else {
        currentSessionId.value = null
        messages.value = []
      }
    }
    saveToStorage()
  }

  // 切换助手
  function setAssistant(key) {
    if (!ASSISTANTS[key]) return
    currentAssistant.value = key
    if (currentSession.value) {
      currentSession.value.assistant = key
      currentSession.value.subtitle = ASSISTANTS[key].name
    }
    saveToStorage()
  }

  // 切换咨询模式
  function setConsultMode(mode) {
    if (!CONSULT_MODES[mode]) return
    consultMode.value = mode
    if (currentSession.value) {
      currentSession.value.mode = mode
    }
  }

  // 更新会话标题
  function updateSessionTitle(firstUserMsg) {
    const session = currentSession.value
    if (!session) return
    const title = firstUserMsg.slice(0, 30) + (firstUserMsg.length > 30 ? '…' : '')
    session.title = title
    session.updatedAt = Date.now()
    saveToStorage()
  }

  // 发送消息
  async function sendMessage(text, { stream = true } = {}) {
    if (!text.trim()) return
    if (isLoading.value || isStreaming.value) return

    if (!currentSessionId.value) {
      newSession()
    }

    const userMsg = {
      id: genId('msg'),
      role: 'user',
      content: text.trim(),
      type: 'text',
      createdAt: Date.now()
    }
    messages.value.push(userMsg)

    const userMsgs = messages.value.filter(m => m.role === 'user')
    if (userMsgs.length === 1) {
      updateSessionTitle(text.trim())
    }

    isLoading.value = true
    isStreaming.value = stream

    const aiMsgId = genId('msg')
    const aiMsg = {
      id: aiMsgId,
      role: 'assistant',
      content: '',
      type: 'text',
      createdAt: Date.now()
    }
    messages.value.push(aiMsg)

    try {
      if (stream) {
        await streamChatResponse(text, aiMsgId)
      } else {
        const result = await callChatAPI(text)
        const target = messages.value.find(m => m.id === aiMsgId)
        if (target) {
          target.content = result.content || result.message || '（无内容）'
        }
      }

      const session = currentSession.value
      if (session) {
        session.status = 'done'
        session.updatedAt = Date.now()
      }

      saveMessages()
      saveToStorage()
    } catch (e) {
      const target = messages.value.find(m => m.id === aiMsgId)
      if (target) {
        target.content = `❌ 请求失败：${e.message || '未知错误'}\n\n请检查后端服务是否正常运行，或稍后重试。`
        target.error = true
      }
    } finally {
      isLoading.value = false
      isStreaming.value = false
      streamController.value = null
    }
  }

  // 调用对话 API（非流式）
  async function callChatAPI(text) {
    const def = allianceDef.value
    if (def) return await callAllianceCollab(def, text)

    // 通用对话：助手人格只是 systemPrompt 的选择，不等于联盟专家，绝不因此改道联盟契约
    const payload = {
      message: text,
      session_id: currentSessionId.value,
      assistant: currentAssistant.value,
      scope: currentScope.value,
      project_id: currentProjectId.value,
      consult_mode: consultMode.value,
      selected_experts: selectedExpertIds.value,
      messages: messages.value
        .filter(m => m.role === 'user' || m.role === 'assistant')
        .map(m => ({ role: m.role, content: m.content }))
    }
    return await aiChat(payload)
  }

  /**
   * 联盟咨询：入参组装与校验一律经模块契约（contract/collab.js），返回沿用聊天框的 content 口径。
   * 契约的六个模式都是单轮请求（除 route 的 constraints 外没有可读的历史字段），
   * 所以这里不再像旧实现那样塞 messages——后端根本不看，塞了只会让人以为多轮生效了。
   */
  async function callAllianceCollab(def, text) {
    const input = { [def.field]: text, expertIds: [...selectedExpertIds.value] }
    const problem = collabProblem(def, input)
    if (problem) throw new Error(problem)
    const result = await allianceApi.collaborate(def.key, input)
    return { content: collabChatText(result), mode: def.key, consult_mode: def.key, result }
  }

  // 流式对话
  async function streamChatResponse(text, aiMsgId) {
    const payload = {
      message: text,
      session_id: currentSessionId.value,
      assistant: currentAssistant.value,
      scope: currentScope.value,
      project_id: currentProjectId.value,
      consult_mode: consultMode.value,
      selected_experts: selectedExpertIds.value,
      stream: true,
      messages: messages.value
        .filter(m => m.role === 'user' || m.role === 'assistant')
        .map(m => ({ role: m.role, content: m.content }))
    }

    try {
      const def = allianceDef.value
      if (def) {
        // 联盟六个模式在网关侧都是请求-响应式（没有对应 SSE），所以整段一次性写入，
        // 不再像旧实现那样去 fetch 被禁的专家对话端点（其 multi/debate 分支读的是
        // 子 handler 从不产出的键，返回正文恒为兜底串）。
        const allianceResult = await callAllianceCollab(def, text)
        const target = messages.value.find(m => m.id === aiMsgId)
        if (target) target.content = allianceResult.content
        return
      }
      const endpoint = '/api/ai/chat'

      const token = getToken()
                || import.meta.env?.VITE_API_TOKEN
                || 'dev-secret-token'

      const response = await fetch(endpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': 'Bearer ' + token
        },
        body: JSON.stringify(payload)
      })

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`)
      }

      const contentType = response.headers.get('content-type') || ''
      const isStream = contentType.includes('text/event-stream')
                     || contentType.includes('application/x-ndjson')

      if (isStream && response.body) {
        const reader = response.body.getReader()
        const decoder = new TextDecoder('utf-8')
        let buffer = ''
        let fullContent = ''

        streamController.value = reader

        while (true) {
          const { done, value } = await reader.read()
          if (done) break

          buffer += decoder.decode(value, { stream: true })

          const sseRegex = /data:\s*(.+?)\n\n/g
          let match
          while ((match = sseRegex.exec(buffer)) !== null) {
            const chunk = parseStreamChunk(match[1])
            if (chunk) {
              fullContent += chunk
              const target = messages.value.find(m => m.id === aiMsgId)
              if (target) target.content = fullContent
            }
            buffer = buffer.slice(match.index + match[0].length)
          }
        }

        streamController.value = null

        if (!fullContent.trim()) {
          const result = await callChatAPI(text)
          const target = messages.value.find(m => m.id === aiMsgId)
          if (target) target.content = result.content || result.message || ''
        }
      } else {
        // 非流式响应
        const data = await response.json()
        const content = data?.content || data?.data?.content || data?.message || ''
        const target = messages.value.find(m => m.id === aiMsgId)
        if (target) target.content = content
      }
    } catch (e) {
      if (e.name === 'AbortError') return
      // 回退到非流式
      try {
        const result = await callChatAPI(text)
        const target = messages.value.find(m => m.id === aiMsgId)
        if (target) target.content = result.content || result.message || ''
      } catch (e2) {
        throw e2
      }
    }
  }

  function parseStreamChunk(raw) {
    if (!raw) return ''
    try {
      const obj = JSON.parse(raw)
      return obj.content || obj.delta || obj.text || obj.message || ''
    } catch {
      return raw
    }
  }

  // 停止生成
  function stopGeneration() {
    if (streamController.value) {
      try { streamController.value.cancel() } catch {}
      streamController.value = null
    }
    isStreaming.value = false
    isLoading.value = false
  }

  // 重新生成
  async function regenerate() {
    const lastUser = [...messages.value].reverse().find(m => m.role === 'user')
    if (!lastUser) return

    const lastAiIdx = [...messages.value].reverse().findIndex(m => m.role === 'assistant')
    if (lastAiIdx >= 0) {
      const realIdx = messages.value.length - 1 - lastAiIdx
      messages.value.splice(realIdx, 1)
    }

    await sendMessage(lastUser.content)
  }

  // 清空当前会话
  function clearCurrentSession() {
    messages.value = []
    if (currentSession.value) {
      currentSession.value.status = 'running'
      currentSession.value.title = '新对话'
    }
    saveMessages()
    saveToStorage()
  }

  // 确保有会话（首次访问自动创建一个欢迎会话）
  function ensureSession() {
    if (!currentSessionId.value || sessions.value.length === 0) {
      newSession()
    } else if (!messages.value.length) {
      loadMessages(currentSessionId.value)
    }
  }

  // 从旧版本迁移数据（v1 → v2）
  function migrateFromV1() {
    try {
      const oldKey = 'mox.ai.sessions.v1'
      const oldCurrentKey = 'mox.ai.currentSession.v1'
      const oldData = localStorage.getItem(oldKey)
      if (!oldData) return false

      const parsed = JSON.parse(oldData)
      if (!parsed.sessions || parsed.sessions.length === 0) {
        localStorage.removeItem(oldKey)
        localStorage.removeItem(oldCurrentKey)
        return false
      }

      // 迁移到 v2 global 作用域
      const newKey = storageKey('global', null)
      const newCurKey = currentKey('global', null)

      // 如果 v2 已经有数据就不覆盖
      if (localStorage.getItem(newKey)) return false

      localStorage.setItem(newKey, JSON.stringify({ sessions: parsed.sessions }))
      const oldCurrent = localStorage.getItem(oldCurrentKey)
      if (oldCurrent) {
        localStorage.setItem(newCurKey, oldCurrent)
      }

      // 迁移消息
      parsed.sessions.forEach(s => {
        const oldMsgKey = `mox.ai.messages.${s.id}`
        const newMsgKey = `${STORAGE_PREFIX}.msgs.${s.id}`
        const oldMsgs = localStorage.getItem(oldMsgKey)
        if (oldMsgs && !localStorage.getItem(newMsgKey)) {
          localStorage.setItem(newMsgKey, oldMsgs)
        }
      })

      console.log('[AI] 已从 v1 迁移会话数据到 v2')
      return true
    } catch (e) {
      console.warn('迁移旧会话数据失败:', e)
      return false
    }
  }

  // ===== 初始化 =====
  // 先尝试迁移旧数据
  migrateFromV1()
  // 再加载当前作用域数据
  loadFromStorage()

  return {
    // State
    sessions,
    currentSessionId,
    messages,
    currentAssistant,
    isLoading,
    isStreaming,
    currentScope,
    currentProjectId,
    consultMode,
    selectedExpertIds,
    // Getters
    currentSession,
    currentAssistantObj,
    sortedSessions,
    hasMessages,
    lastUserMessage,
    currentConsultMode,
    allianceDef,
    // Actions
    setScope,
    newSession,
    selectSession,
    deleteSession,
    setAssistant,
    setConsultMode,
    sendMessage,
    stopGeneration,
    regenerate,
    clearCurrentSession,
    ensureSession,
    loadFromStorage,
    saveToStorage
  }
})
