// 工作流、流程图、自动化、插件、MCP、浏览器 API
import http from './http'

// ===== 工作流 =====
export const getWorkflowTemplates = () => http.get('/ai/workflows/templates')
export const getWorkflows = () => http.get('/ai/workflows')
export const saveWorkflow = (payload) => http.post('/ai/workflows/save', payload)
export const executeWorkflowDef = (payload) => http.post('/ai/workflows/execute', payload)
export const getWorkflowInstances = () => http.get('/ai/workflows/instances')

// ===== 流程图 (FlowGraph IR) =====
export const getFlows = () => http.get('/ai/flows')
export const createFlow = (payload) => http.post('/ai/flows', payload)
export const getFlow = (id) => http.get(`/ai/flows/${encodeURIComponent(id)}`)
export const deleteFlow = (id) => http.delete(`/ai/flows/${encodeURIComponent(id)}`)
export const validateFlow = (payload) => http.post('/ai/flows/validate', payload)
export const executeFlow = (payload) => http.post('/ai/flows/execute', payload)
export const getFlowNodeTypes = () => http.get('/ai/flows/node-types')

// ===== AI 插件 =====
export const getAiPlugins = () => http.get('/ai/plugins')
export const registerAiPlugin = (payload) => http.post('/ai/plugins/register', payload)
export const sendPluginMessage = (payload) => http.post('/ai/plugins/send-message', payload)
// 插件拓扑
export const getPluginTopology = () => http.get('/ai/plugins/topology')

// ===== MCP 兼容层 (Model Context Protocol) =====
export const mcpListTools = () =>
  http.post('/mcp', { jsonrpc: '2.0', id: 1, method: 'tools/list', params: {} })
export const mcpCall = (name, args) =>
  http.post('/mcp', { jsonrpc: '2.0', id: 2, method: 'tools/call', params: { name, arguments: args } })

// ===== AI 自动化中枢 =====
export const getAutomations = () => http.get('/automation')
/** @deprecated 请使用 getAutomations */
export const automationList = getAutomations
export const automationChat = (payload) => http.post('/automation/chat', payload)
export const automationRefine = (id, payload) => http.post(`/automation/${encodeURIComponent(id)}/refine`, payload)
export const automationRun = (id, payload) => http.post(`/automation/${encodeURIComponent(id)}/run`, payload)
export const automationPermissions = (id) => http.get(`/automation/${encodeURIComponent(id)}/permissions`)
export const automationUpdate = (id, payload) => http.put(`/automation/${encodeURIComponent(id)}`, payload)

// ===== 浏览器自动化 =====
export const getBrowserTemplates = () => http.get('/ai/browser/templates')
export const getBrowserSessions = () => http.get('/ai/browser/sessions')
export const getBrowserSession = (id) => http.get(`/ai/browser/sessions/${encodeURIComponent(id)}`)
export const closeBrowserSession = (id) =>
  http.delete(`/ai/browser/sessions/${encodeURIComponent(id)}`)
export const executeBrowserTask = (payload) => http.post('/ai/browser/execute-task', payload)
export const executeBrowserSteps = (payload) => http.post('/ai/browser/execute-steps', payload)
export const executeBrowserAction = (payload) => http.post('/ai/browser/execute-action', payload)
export const browserNatural = (payload) => http.post('/ai/browser/natural', payload)
