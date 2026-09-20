// 璇玑架构治理与架构融合 API
import http from './http'

// ===== 璇玑架构治理 (双璇玑十四维) =====
// 维度清单与璇玑健康度
export const moxHealth = () => http.get('/mox/health')
// 架构治理，返回 GovernanceReport
export const moxOptimize = (flow, tenant = 'default') =>
  http.post('/mox/optimize', { flow, tenant })
// 架构融合发布
export const moxPublish = (payload) => http.post('/mox/publish', payload)
