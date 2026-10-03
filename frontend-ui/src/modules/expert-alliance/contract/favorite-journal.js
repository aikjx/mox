// Pending intentions only. Committed favorite state always comes from the gateway.
export function favoriteJournalScope(user) {
  if (!user?.id || !user?.tenant_id) throw new Error('收藏操作需要完整的用户和租户身份')
  return `alliance.favorite.pending.v1:${encodeURIComponent(user.tenant_id)}:${encodeURIComponent(user.id)}`
}

export function readFavoriteJournal(storage, scope) {
  const raw = storage.getItem(scope)
  if (raw === null) return new Map()
  if (raw.length > 100000) throw new Error('待确认收藏记录超过容量限制')
  const rows = JSON.parse(raw)
  if (!Array.isArray(rows) || rows.length > 100) throw new Error('待确认收藏记录格式错误')
  const attempts = new Map()
  for (const row of rows) {
    if (!row || typeof row.expertId !== 'string' || !row.expertId.trim() || new TextEncoder().encode(row.expertId).length > 256 || !/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(row.key) || attempts.has(row.expertId)) {
      throw new Error('待确认收藏记录损坏，请保留记录并联系管理员核实')
    }
    attempts.set(row.expertId, Object.freeze({ expertId: row.expertId, key: row.key }))
  }
  return attempts
}

export function writeFavoriteJournal(storage, scope, attempts) {
  if (attempts.size > 100) throw new Error('待确认收藏请求已达到 100 条，请先恢复处理')
  if (attempts.size) storage.setItem(scope, JSON.stringify([...attempts.values()]))
  else storage.removeItem(scope)
}
