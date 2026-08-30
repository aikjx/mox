// 璇玑全维治理与全维融合 API
import http from './http'

// ===== 璇玑全维治理 (双璇玑十四维) =====
// 维度清单与璇玑健康度
export const moxHealth = () => http.get('/mox/health')
// 全维治理，返回 GovernanceReport
export const moxOptimize = (flow, tenant = 'default') =>
  http.post('/mox/optimize', { flow, tenant })
// 全维融合发布
export const moxPublish = (payload) => http.post('/mox/publish', payload)
