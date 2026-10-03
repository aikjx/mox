export function createFavoriteRequest(expertId) {
  if (typeof expertId !== 'string' || !expertId.trim()) throw new Error('缺少专家 ID')
  if (new TextEncoder().encode(expertId).length > 256) throw new Error('专家 ID 超过 256 字节限制')
  if (!globalThis.crypto?.randomUUID) throw new Error('当前环境无法生成安全请求键')
  return Object.freeze({ expertId, key: globalThis.crypto.randomUUID() })
}

export function favoriteRequestHeaders(key) {
  if (typeof key !== 'string' || !/^[A-Za-z0-9_.:-]{1,128}$/.test(key)) throw new Error('收藏请求键格式无效')
  return { 'Idempotency-Key': key }
}

export function favoriteQueryBody(expertIds) {
  if (!Array.isArray(expertIds) || expertIds.length > 100) throw new Error('收藏批次最多支持 100 位专家')
  const body = {}
  body.expert_ids = [...expertIds]
  return body
}

export function favoriteRejectionIsDefinitive(error) {
  return [400, 401, 403, 404, 409, 413, 422].includes(Number(error?.status ?? error?.response?.status))
}
