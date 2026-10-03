// 严格投影真实订阅，不以默认对象冒充成功结果。
export function normWebhook(row) {
  if (!row || typeof row.id !== 'string' || !row.id || typeof row.url !== 'string'
      || typeof row.tenant !== 'string' || typeof row.created_at !== 'string'
      || !Array.isArray(row.event_types) || !row.event_types.every(type => typeof type === 'string')) {
    throw new Error('订阅响应格式无效，请刷新后核对')
  }
  return { id: row.id, url: row.url, tenantId: row.tenant, eventTypes: [...row.event_types], createdAt: row.created_at }
}

export function normWebhookList(payload) {
  if (!Array.isArray(payload?.webhooks) || !Number.isSafeInteger(payload.total) || payload.total < 0) {
    throw new Error('订阅列表响应格式无效')
  }
  return { items: payload.webhooks.map(normWebhook), total: payload.total }
}
