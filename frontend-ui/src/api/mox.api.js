// 璇玑mox 模块化系统架构治理与mox 模块化系统架构融合 API
import http from './http'

// ===== 璇玑mox 模块化系统架构治理 (双璇玑十四维) =====
// 维度清单与璇玑健康度
export const moxHealth = () => http.get('/mox/health')
// mox 模块化系统架构治理，返回 GovernanceReport
export const moxOptimize = (flow, tenant = 'default') =>
  http.post('/mox/optimize', { flow, tenant })
// mox 模块化系统架构融合发布
export const moxPublish = (payload) => http.post('/mox/publish', payload)
