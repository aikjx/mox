/** Plain-text in-app command contract; authority remains in the IAM-backed server. */
export function buildInAppPayload(draft) {
  const title = typeof draft.title === 'string' ? draft.title.trim() : ''
  const content = typeof draft.content === 'string' ? draft.content : ''
  const ids = typeof draft.receivers === 'string' ? draft.receivers.split(/[\s,，;；]+/u).filter(Boolean) : []
  const receivers = [...new Set(ids)].sort()
  if (!title || [...title].length > 200) throw new Error('标题须为 1–200 个字符')
  if (!content.trim() || [...content].length > 10000) throw new Error('正文须非空且不超过 10000 个字符')
  if (!receivers.length || receivers.length > 100 || receivers.some(id => [...id].length > 128)) throw new Error('请输入 1–100 个真实收件人 ID，每个不超过 128 个字符')
  if (!['system', 'task', 'alert', 'custom'].includes(draft.type)) throw new Error('消息类型无效')
  if (!['low', 'normal', 'high', 'urgent'].includes(draft.priority)) throw new Error('优先级无效')
  return { title, content, message_type: draft.type, priority: draft.priority, channels: ['in_app'], receiver_ids: receivers }
}

export function createSendAttempt(draft) {
  const payload = buildInAppPayload(draft)
  if (typeof globalThis.crypto?.randomUUID !== 'function') throw new Error('当前环境无法安全创建发送标识，请使用 HTTPS 或本机安全地址')
  Object.freeze(payload.receiver_ids)
  Object.freeze(payload.channels)
  return Object.freeze({ key: globalThis.crypto.randomUUID(), payload: Object.freeze(payload) })
}

export function readSendReceipt(data) {
  if (typeof data?.message_id !== 'string' || !data.message_id.trim()) throw new Error('未获得有效发送回执，请沿原请求确认结果')
  return data.message_id
}

export function isDefinitiveSendRejection(error) {
  return [400, 401, 403, 404, 409, 413, 422, 501].includes(error?.status)
}
