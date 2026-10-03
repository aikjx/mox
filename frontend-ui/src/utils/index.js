// 工具函数统一出口（聚集）
// 规则：无命名冲突直接 re-export；冲突项（safeUrl）以 kb 前缀别名导出，避免 export * 造成 ambiguous binding。
// formatTime 曾经的"两份同体副本"已并到 utils/time.js 的 formatDateTime，这里不再需要别名。
export * from './message.utils'
export {
  escapeHtml, simpleMarkdownRender, truncateText, KB_CATEGORIES, KB_STATUSES,
  getCategoryLabel, getCategoryTagType, getCategoryIcon, isAiAnalyzed,
  getStatusType, getStatusLabel, getActionLabel, mapDoc, getTagSize, kbEntityMutation,
} from './knowledgeBase.utils'
export { safeUrl as kbSafeUrl } from './knowledgeBase.utils'
export { timeValue, formatDateTime, formatDateTimeLocale, formatDateTimeLocaleOr, formatClockMinute, formatClockSecond, formatDateStamp, relativeTimeText, timeAgoOrDate } from './time'
export * from './projectMember.utils'
export { renderMarkdown, escapeText } from './markdown'
export {
  secureSetItem, secureGetItem, secureRemoveItem, isExpired, getRemainingTime,
  secureClear, setToken, getToken, removeToken, hasValidToken,
} from './secureStorage'
export {
  HITL_ACTIONS, hitlClient, onHitlEvent, onHitlActionResult,
  onHitlPendingList, onHitlConnection, onHitlError,
} from './hitl-ws'
export { parseBoundedPage, parseApiKeyPage, parseAuditPage, apiKeyEligibilityLabel } from './api-key'
