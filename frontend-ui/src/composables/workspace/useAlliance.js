/**
 * 联盟协作 Composable
 * 职责：SSE 联盟协作、消息管理、阶段控制
 */
import { ref, computed, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import { runAllianceFullSSE } from '@/api/alliance'

export function useAlliance(expertColor, expertEmoji, selectedExpertIds, currentProject, collabMode, activeSession, newCollaboration) {
  const collabMessages = ref([])
  const collabInput = ref('')
  const allianceRunning = ref(false)
  const currentPhaseIndex = ref(-1)
  const messagesScrollRef = ref(null)
  let allianceAbortController = null

  const alliancePhases = [
    { key: 'intent', label: '意图识别' }, { key: 'team', label: '组队匹配' },
    { key: 'debate', label: '专家辩论' }, { key: 'synthesize', label: '综合归纳' },
    { key: 'gate', label: '质量把关' }, { key: 'learn', label: '知识学习' },
    { key: 'done', label: '完成' }
  ]

  const currentPhaseLabel = computed(() => {
    if (currentPhaseIndex.value < 0) return '准备中'
    return alliancePhases[currentPhaseIndex.value]?.label || '处理中'
  })

  function selectedExpertNames() {
    // 需要从外部传入 experts，这里用 selectedExpertIds 占位
    return selectedExpertIds.value.length + ' 位专家'
  }

  async function sendCollabMsg() {
    if (!collabInput.value.trim() || allianceRunning.value) return
    const text = collabInput.value.trim()
    collabInput.value = ''
    collabMessages.value.push({ id: Date.now(), role: 'user', name: '我', avatar: 'U', color: 'linear-gradient(135deg, #6366f1, #06b6d4)', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), text })
    scrollMessagesToBottom()
    if (!activeSession.value) newCollaboration?.()
    await runAlliance(text)
  }

  async function runAlliance(query) {
    allianceRunning.value = true
    currentPhaseIndex.value = 0
    try {
      await runAllianceFullSSE(
        { query, session_id: activeSession.value?.id, enable_llm_debate: collabMode.value === 'debate', team_size: selectedExpertIds.value.length || 3, context: { project_id: currentProject.value, mode: collabMode.value, selected_experts: JSON.stringify(selectedExpertIds.value) } },
        (frame) => { handleAllianceFrame(frame) }
      )
    } catch (e) {
      console.warn('[alliance] SSE 调用失败:', e)
      ElMessage.error('联盟协作调用失败：' + (e?.message || '未知错误'))
      collabMessages.value.push({ id: Date.now(), role: 'system', name: '系统', avatar: '⚠️', color: '#f59e0b', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), text: '协作调用失败，请稍后重试' })
    }
    finally {
      allianceRunning.value = false
      currentPhaseIndex.value = alliancePhases.length - 1
      setTimeout(() => { currentPhaseIndex.value = -1 }, 2000)
    }
  }

  function handleAllianceFrame(frame) {
    const phaseIdx = alliancePhases.findIndex(p => p.key === frame.phase)
    if (phaseIdx >= 0) currentPhaseIndex.value = phaseIdx
    if (frame.payload) {
      let msg = null
      if (frame.phase === 'intent') msg = { id: Date.now() + Math.random(), role: 'assistant', name: '意图分析', avatar: '🎯', color: '#6366f1', phase: 'intent', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), text: frame.payload.intent || frame.payload.summary || '正在分析您的问题意图…' }
      else if (frame.phase === 'team') { const experts = frame.payload.experts || frame.payload.team || []; msg = { id: Date.now() + Math.random(), role: 'assistant', name: '组队匹配', avatar: '👥', color: '#06b6d4', phase: 'team', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), text: `已匹配 ${experts.length} 位专家：${experts.map(e => e.name || e).join('、')}` } }
      else if (frame.phase === 'debate') msg = { id: Date.now() + Math.random(), role: 'expert', name: frame.payload.expert_name || '专家发言', avatar: (frame.payload.expert_name || '专')[0], color: expertColor?.(frame.payload.expert_type), phase: 'debate', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), text: frame.payload.content || frame.payload.argument || '' }
      else if (frame.phase === 'synthesize') msg = { id: Date.now() + Math.random(), role: 'assistant', name: '综合归纳', avatar: '📝', color: '#10b981', phase: 'synthesize', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), text: frame.payload.summary || frame.payload.synthesis || '正在综合各方观点…' }
      else if (frame.phase === 'done') msg = { id: Date.now() + Math.random(), role: 'assistant', name: '协作完成', avatar: '✅', color: '#10b981', phase: 'done', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), text: frame.payload.final_answer || frame.payload.result || '协作完成，以上是综合结果。' }
      if (msg && msg.text) { collabMessages.value.push(msg); scrollMessagesToBottom() }
    }
  }

  function stopAlliance() {
    allianceRunning.value = false
    if (allianceAbortController) { allianceAbortController.abort(); allianceAbortController = null }
    collabMessages.value.push({ id: Date.now(), role: 'system', name: '系统', avatar: '⚠️', color: '#f59e0b', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), text: '协作已被用户停止' })
  }

  function scrollMessagesToBottom() {
    nextTick(() => { if (messagesScrollRef.value) messagesScrollRef.value.scrollTo?.({ top: 99999, behavior: 'smooth' }) })
  }

  function appendMessage(msg) {
    collabMessages.value.push({ id: Date.now() + Math.random(), time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }), ...msg })
    scrollMessagesToBottom()
  }

  return {
    collabMessages, collabInput, allianceRunning, currentPhaseIndex,
    messagesScrollRef, currentPhaseLabel,
    sendCollabMsg, runAlliance, stopAlliance, scrollMessagesToBottom, appendMessage
  }
}
