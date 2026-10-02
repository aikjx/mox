/** Shared credential list contract for handwritten and schema-driven pages. */
export function parseBoundedPage(data, kind = '列表') {
  if (!Array.isArray(data?.items) || !Number.isSafeInteger(data.total) || data.total < 0
    || !Number.isSafeInteger(data.page) || data.page < 1
    || !Number.isSafeInteger(data.page_size) || data.page_size < 1 || data.page_size > 100
    || data.items.length > data.page_size || data.total < data.items.length) {
    throw new Error(`${kind}分页响应格式无效`)
  }
  return data
}
export function parseApiKeyPage(data) { return parseBoundedPage(data, '凭证') }
export function parseAuditPage(data) { return parseBoundedPage(data, '审计') }

const eligibilityLabels = {
  eligible: '基础配置可用',
  revoked_or_inactive: '已撤销或停用',
  unsupported_scope: '旧权限范围未支持',
  missing_identity: '身份归属缺失',
  inactive_user: '账号已停用',
  inactive_tenant: '租户已停用',
  expired: '已过期',
  invalid_expiry: '到期时间无效',
}
export function apiKeyEligibilityLabel(value) {
  return Object.hasOwn(eligibilityLabels, value) ? eligibilityLabels[value] : '未评估'
}
