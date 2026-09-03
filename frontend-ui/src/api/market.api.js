// 市场域 API - 模板审核
import http from './http'

// ===== 模板审核（通过/拒绝） =====
export const reviewMarketTemplate = (templateId, data) =>
  http.post(`/market/${encodeURIComponent(templateId)}/review`, data)
