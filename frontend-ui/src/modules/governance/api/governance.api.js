// 治理台 API 层：端点取自 contract/endpoints.js，字段取自 model/normalize.js。
// 本层不做 UI 决策、不 catch 业务错误——错误一律以 ApiError 冒泡给 store 单点呈现。
import { http as defaultHttp } from '@/api'
import { ENDPOINTS, auditQuery, requestPath, vetoQuery } from '@/modules/governance/contract/endpoints'
import { unwrap } from '@/modules/_kernel/envelope'
import {
  normAssessSummary, normAuditPage, normDashboard, normExpertConfig, normExpertsStatus,
  normRbacConfig, normVetoPage
} from '@/modules/governance/model/normalize'

function call(httpClient, name, { params = {}, body, query } = {}) {
  const ep = ENDPOINTS[name]
  if (!ep) throw new Error(`未知治理端点: ${name}`)
  return httpClient
    .request({
      url: requestPath(name, params),
      method: ep.method,
      data: body,
      params: query,
      silent: true
    })
    .then((res) => unwrap(res, { nesting: ep.nesting }))
}

export function createGovernanceApi(httpClient = defaultHttp) {
  const get = (name, opts) => call(httpClient, name, opts)

  return {
    // 概览：DashboardData（expertStates 是以维度为键的 map，不走列表兜底）
    async getDashboard() {
      return normDashboard(await get('dashboard'))
    },

    // 十四维专家状态：业务 7 + 开发 7 两段
    async getExpertsStatus() {
      return normExpertsStatus(await get('expertsStatus'))
    },

    // 否决事件分页：查询键必须是 camelCase（VetoQuery 带 rename_all=camelCase，
    // 发 flow_id 会被静默忽略 ⇒ 筛选"生效了"但其实没筛）
    async listVetoEvents(input = {}) {
      return normVetoPage(await get('vetoEvents', { query: vetoQuery(input) }))
    },

    async listAuditLogs(input = {}) {
      return normAuditPage(await get('auditLogs', { query: auditQuery(input) }))
    },

    async getExpertConfig() {
      return normExpertConfig(await get('expertConfig'))
    },

    async getRbacConfig() {
      return normRbacConfig(await get('rbacConfig'))
    },

    // POST /assess 收 AssessRequest{flowId, flowName, flow: FlowGraph}。
    // flow 无 serde 缺省 ⇒ 三键缺一就是 422 且响应不是 {code,msg} 信封。
    // 本模块不提供"从界面拼一张 FlowGraph"的入口，方法先按契约透传，由调用方给整图。
    async assessFlow(input = {}) {
      return normAssessSummary(
        await call(httpClient, 'assess', {
          body: { flowId: input.flowId, flowName: input.flowName, flow: input.flow }
        })
      )
    }
  }
}

export const governanceApi = createGovernanceApi()
