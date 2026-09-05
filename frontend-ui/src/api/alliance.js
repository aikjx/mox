// 联盟引擎 API - 已重构为统一 http.js fetcher
// SSE 流式端点保留原生 fetch，鉴权头对齐 http.js 请求拦截器（getToken）
import http from './http'
import { normalizeTask, normalizeTaskList, normalizeTaskLogs, normalizeTaskDag, normalizeTaskFusion, unwrapTaskPayload } from './allianceTaskModel'
import { getToken } from '@/utils/secureStorage'

// ===== 联盟引擎能力 =====
export async function getAllianceCapabilities() {
  return http.get('/ai/engine/alliance/capabilities')
}

// ===== SSE: 联盟全流程流式执行 =====
export async function runAllianceFullSSE(req, onFrame) {
  const token = getToken()
  const headers = {
    'Content-Type': 'application/json',
    'Accept': 'text/event-stream'
  }
  if (token) headers['Authorization'] = `Bearer ${token}`

  const resp = await fetch('/api/ai/engine/alliance/full', {
    method: 'POST',
    headers,
    body: JSON.stringify(req)
  })
  if (!resp.ok) {
    throw new Error(`alliance/full HTTP ${resp.status}`)
  }
  const reader = resp.body?.getReader()
  if (!reader) throw new Error('No readable stream')
  const decoder = new TextDecoder('utf-8')
  let buffer = ''
  let lastTraceId = ''
  let currentEventName = ''
  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    buffer += decoder.decode(value, { stream: true })
    let idx
    while ((idx = buffer.indexOf('\n\n')) >= 0) {
      const rawEvent = buffer.slice(0, idx)
      buffer = buffer.slice(idx + 2)
      let data = ''
      for (const line of rawEvent.split('\n')) {
        if (line.startsWith('event:')) currentEventName = line.slice(6).trim()
        else if (line.startsWith('data:')) data += line.slice(5).trimStart()
      }
      if (!data) continue
      if (data === '[DONE]') {
        reader.releaseLock()
        return lastTraceId
      }
      try {
        const frame = JSON.parse(data)
        if (frame.trace_id) lastTraceId = frame.trace_id
        const cont = onFrame(frame)
        if (cont === false) {
          reader.releaseLock()
          return lastTraceId
        }
      } catch (e) {
        // dev only: SSE 帧解析失败属内部流处理错误
        console.warn('[alliance.sse] frame parse failed:', data, e)
      }
    }
  }
  reader.releaseLock()
  return lastTraceId
}

// 向后兼容别名
export const runAllianceTask = runAllianceFullSSE

// ===== 语音健康检查（保留降级兜底） =====
export async function getVoiceHealth() {
  try {
    return await http.get('/voice/health')
  } catch (e) {
    return {
      ok: false,
      upstream_unreachable: true,
      fallback_action: 'AC-22 三层回退（连不上 Rust 网关）：直接 browser Web Speech Synthesis',
      tts: {
        ready: false,
        active: 'browser_tts',
        engines: [
          { name: 'cosyvoice2', available: false, license: 'Apache-2.0' },
          { name: 'fish_s2_pro', available: false, license: 'Research', note: '默认禁用，Research License' }
        ]
      }
    }
  }
}

// ===== 专家注册/列表/咨询 =====
export async function allianceRegisterExpert(payload) {
  return http.post('/experts', payload)
}

export async function allianceGetExperts(params = {}) {
  return http.get('/experts', { params })
}

export async function allianceConsultExpert(expertId, payload) {
  return http.post(`/experts/${encodeURIComponent(expertId)}/consult`, { ...payload, expert_id: expertId })
}

export async function allianceMultiExpertConsult(payload) {
  return http.post('/experts/multi-consult', payload)
}

// ===== 专家辩论（支持 SSE 流模式） =====
export async function allianceExpertDebate(payload, onFrame) {
  if (!payload.stream || !onFrame) {
    return http.post('/experts/debate', payload)
  }
  // SSE 流模式
  const token = getToken()
  const headers = {
    'Content-Type': 'application/json',
    'Accept': 'text/event-stream'
  }
  if (token) headers['Authorization'] = `Bearer ${token}`

  const resp = await fetch('/api/experts/debate', {
    method: 'POST',
    headers,
    body: JSON.stringify(payload)
  })
  if (!resp.ok) throw new Error(`experts/debate HTTP ${resp.status}`)
  const reader = resp.body?.getReader()
  if (!reader) throw new Error('No readable stream')
  const decoder = new TextDecoder('utf-8')
  let buffer = ''
  let finalResult = null
  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    buffer += decoder.decode(value, { stream: true })
    let idx
    while ((idx = buffer.indexOf('\n\n')) >= 0) {
      const rawEvent = buffer.slice(0, idx)
      buffer = buffer.slice(idx + 2)
      let data = ''
      for (const line of rawEvent.split('\n')) {
        if (line.startsWith('data:')) data += line.slice(5).trimStart()
      }
      if (!data) continue
      if (data === '[DONE]') {
        reader.releaseLock()
        if (!finalResult) throw new Error('辩论未返回最终结果')
        return finalResult
      }
      try {
        const parsed = JSON.parse(data)
        if (parsed.type === 'result' || parsed.conclusion) {
          finalResult = parsed
        } else if (onFrame) {
          const frame = parsed
          const cont = onFrame(frame)
          if (cont === false) {
            reader.releaseLock()
            return finalResult || {}
          }
        }
      } catch (e) {
        // dev only: SSE 帧解析失败属内部流处理错误
        console.warn('[alliance.debate] frame parse failed:', data, e)
      }
    }
  }
  reader.releaseLock()
  if (!finalResult) throw new Error('辩论未返回最终结果')
  return finalResult
}

// ===== 专家路由/智能咨询/算法分析 =====
export async function allianceRouteExperts(payload) {
  return http.post('/experts/route', payload)
}

export async function allianceIntelligentConsult(payload) {
  return http.post('/experts/intelligent-consult', payload)
}

export async function allianceAlgorithmAnalysis(payload) {
  return http.post('/experts/algorithm-analysis', payload)
}

// ===== 专家概览/指标 =====
export async function allianceGetExpertOverview() {
  return http.get('/experts/overview')
}

export async function allianceGetExpertMetrics() {
  return http.get('/experts/metrics')
}

export async function allianceGetSingleExpertMetrics(expertId) {
  return http.get(`/experts/${encodeURIComponent(expertId)}/metrics`)
}

// ===== 联盟任务 CRUD =====
export async function createAllianceTask(payload) {
  return normalizeTask(await http.post('/alliance/tasks', { title: payload.title ?? payload.name, description: payload.description, fusion_strategy: payload.fusion_strategy }, { _retry: 0, silent: true }))
}

export async function getAllianceTasks(params = {}) {
  return normalizeTaskList(await http.get('/alliance/tasks', { params, _retry: 0, silent: true }))
}

export async function getAllianceTask(taskId) {
  return normalizeTask(await http.get(`/alliance/tasks/${encodeURIComponent(taskId)}`, { _retry: 0, silent: true }))
}

export async function getCollaborationPlan(taskId) {
  return http.get(`/alliance/tasks/${encodeURIComponent(taskId)}/plan`)
}

// ===== SSE: 任务执行日志流 =====
export async function getExecutionLogsSSE(taskId, onLog) {
  const token = getToken()
  const headers = { 'Accept': 'text/event-stream' }
  if (token) headers['Authorization'] = `Bearer ${token}`

  const resp = await fetch(`/api/alliance/tasks/${encodeURIComponent(taskId)}/logs/stream`, {
    method: 'GET',
    headers
  })
  if (!resp.ok) throw new Error(`alliance/task/logs HTTP ${resp.status}`)
  const reader = resp.body?.getReader()
  if (!reader) throw new Error('No readable stream')
  const decoder = new TextDecoder('utf-8')
  let buffer = ''
  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    buffer += decoder.decode(value, { stream: true })
    let idx
    while ((idx = buffer.indexOf('\n\n')) >= 0) {
      const rawEvent = buffer.slice(0, idx)
      buffer = buffer.slice(idx + 2)
      let data = ''
      for (const line of rawEvent.split('\n')) {
        if (line.startsWith('data:')) data += line.slice(5).trimStart()
      }
      if (!data) continue
      if (data === '[DONE]') {
        reader.releaseLock()
        return
      }
      try {
        const entry = JSON.parse(data)
        const cont = onLog(entry)
        if (cont === false) {
          reader.releaseLock()
          return
        }
      } catch (e) {
        // dev only: SSE 帧解析失败属内部流处理错误
        console.warn('[alliance.logs] frame parse failed:', data, e)
      }
    }
  }
  reader.releaseLock()
}

// ===== 融合结果/任务控制 =====
export async function getFusionResults(taskId) {
  return http.get(`/alliance/tasks/${encodeURIComponent(taskId)}/fusion`)
}

export async function pauseAllianceTask(taskId) {
  return http.post(`/alliance/tasks/${encodeURIComponent(taskId)}/pause`)
}

export async function resumeAllianceTask(taskId) {
  return http.post(`/alliance/tasks/${encodeURIComponent(taskId)}/resume`)
}

export async function cancelAllianceTask(taskId) {
  return http.post(`/alliance/tasks/${encodeURIComponent(taskId)}/cancel`)
}

export async function retryAllianceTask(taskId) {
  return http.post(`/alliance/tasks/${encodeURIComponent(taskId)}/retry`)
}

export async function getAllianceStats() {
  return http.get('/alliance/stats')
}

// ===== 新增：联盟任务域端点（Task 2） =====
export async function getAllianceTaskLogs(taskId, params = {}) {
  return normalizeTaskLogs(await http.get(`/alliance/tasks/${encodeURIComponent(taskId)}/logs`, { params, _retry: 0, silent: true }))
}

export async function getAllianceFusionResult(taskId) {
  return normalizeTaskFusion(await http.get(`/alliance/tasks/${encodeURIComponent(taskId)}/fusion-result`, { _retry: 0, silent: true }))
}

export async function getAllianceTaskDag(taskId) {
  return normalizeTaskDag(await http.get(`/alliance/tasks/${encodeURIComponent(taskId)}/dag`, { _retry: 0, silent: true }))
}

export async function toggleAllianceTaskDone(taskId) {
  return http.put(`/alliance/tasks/${encodeURIComponent(taskId)}/toggle-done`)
}

export async function getAllianceTaskStatus(taskId) {
  return normalizeTask(await http.get(`/alliance/tasks/${encodeURIComponent(taskId)}/status`, { _retry: 0, silent: true }))
}

export async function getAllianceRuntime() {
  return unwrapTaskPayload(await http.get('/alliance/runtime', { _retry: 0, silent: true }))
}
