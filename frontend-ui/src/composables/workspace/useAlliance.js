/**
 * 联盟协作 Composable
 * 职责：工作台协作对话的消息流与阶段指示；取数一律经 expert-alliance 模块契约。
 *
 * 迁移说明（2026-09-27）：原先直连编排器 :3001 独占的联盟全流程 SSE 已撤销——
 * 该端点不在网关 :3080 契约内（模块 `contract/endpoints.js` 的 FORBIDDEN_ENDPOINTS 明令禁止），
 * 从浏览器调用恒 404，七阶段"进度"也就从未真按帧推进过。现改走模块登记的六模式原生端点，
 * 阶段指示只在拿到结果后置为终态，不再臆造中间进度。
 */
import { ref, computed, nextTick } from 'vue'
import { ElMessage } from 'element-plus/es/components/message/index'
import { allianceApi } from '@/modules/expert-alliance/api'
import {
  PHASE_IDS, collabMode as collabModeDef, collabProblem, phaseLabel
} from '@/modules/expert-alliance/contract'
import { collabChatSpeaker, collabChatText, collabChatPhase } from '@/modules/expert-alliance/model'

// 阶段序列与文案由 Rust 单源投影（contract/phases.js），此处不再另写一份
const DONE_PHASE = PHASE_IDS.indexOf('done')

function timeText() {
  return new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
}

/** 归一化结果 → 一条聊天消息：正文口径由模块的 model/collabChat.js 单源决定 */
function resultMessage(result) {
  return {
    id: Date.now() + Math.random(),
    role: 'expert',
    name: collabChatSpeaker(result),
    avatar: '🤝',
    color: '#6366f1',
    time: timeText(),
    phase: collabChatPhase(result),
    text: collabChatText(result)
  }
}

export function useAlliance(expertColor, expertEmoji, selectedExpertIds, currentProject, collabMode, activeSession, newCollaboration) {
  const collabMessages = ref([])
  const collabInput = ref('')
  const allianceRunning = ref(false)
  const currentPhaseIndex = ref(-1)
  const messagesScrollRef = ref(null)

  const currentPhaseLabel = computed(() => {
    if (allianceRunning.value) return '处理中'
    if (currentPhaseIndex.value < 0) return '准备中'
    return phaseLabel(PHASE_IDS[currentPhaseIndex.value])
  })

  async function sendCollabMsg() {
    if (!collabInput.value.trim() || allianceRunning.value) return
    const text = collabInput.value.trim()
    collabInput.value = ''
    collabMessages.value.push({ id: Date.now(), role: 'user', name: '我', avatar: 'U', color: 'linear-gradient(135deg, #6366f1, #06b6d4)', time: timeText(), text })
    scrollMessagesToBottom()
    if (!activeSession.value) newCollaboration?.()
    await runAlliance(text)
  }

  async function runAlliance(query) {
    const def = collabModeDef(collabMode.value)
    if (!def) {
      ElMessage.error('未知协作模式：' + collabMode.value)
      return null
    }
    const input = { [def.field]: query, expertIds: [...selectedExpertIds.value] }
    const blocked = collabProblem(def, input)
    if (blocked) {
      ElMessage.warning(blocked)
      return null
    }
    allianceRunning.value = true
    currentPhaseIndex.value = -1
    try {
      const result = await allianceApi.collaborate(def.key, input)
      collabMessages.value.push(resultMessage(result))
      scrollMessagesToBottom()
      currentPhaseIndex.value = DONE_PHASE
      return result
    } catch (e) {
      console.warn('[alliance] 协作调用失败:', e)
      ElMessage.error('联盟协作调用失败：' + (e?.msg || e?.message || '未知错误'))
      collabMessages.value.push({ id: Date.now(), role: 'system', name: '系统', avatar: '⚠️', color: '#f59e0b', time: timeText(), text: '协作调用失败，请稍后重试' })
      return null
    }
    finally {
      allianceRunning.value = false
      setTimeout(() => { currentPhaseIndex.value = -1 }, 2000)
    }
  }

  function stopAlliance() {
    // 原生端点是请求-响应式，浏览器侧只能放弃这次等待，后端会话仍留痕
    allianceRunning.value = false
    collabMessages.value.push({ id: Date.now(), role: 'system', name: '系统', avatar: '⚠️', color: '#f59e0b', time: timeText(), text: '已放弃本次等待，会话记录可在「会话中心」查看' })
  }

  function scrollMessagesToBottom() {
    nextTick(() => { if (messagesScrollRef.value) messagesScrollRef.value.scrollTo?.({ top: 99999, behavior: 'smooth' }) })
  }

  function appendMessage(msg) {
    collabMessages.value.push({ id: Date.now() + Math.random(), time: timeText(), ...msg })
    scrollMessagesToBottom()
  }

  return {
    collabMessages, collabInput, allianceRunning, currentPhaseIndex,
    messagesScrollRef, currentPhaseLabel,
    sendCollabMsg, runAlliance, stopAlliance, scrollMessagesToBottom, appendMessage
  }
}
