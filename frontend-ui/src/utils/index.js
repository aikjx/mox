// 工具函数统一出口（聚集）
// 规则：无命名冲突直接 re-export；冲突项（safeUrl / formatTime）以 kb 前缀别名导出，
//      避免 export * 造成 ambiguous binding。
export * from './message.utils'
export {
  escapeHtml, simpleMarkdownRender, truncateText, DOC_TYPES, getTypeLabel,
  getTagType, getStatusType, getStatusLabel, getActionLabel, mapDoc, getTagSize,
} from './knowledgeBase.utils'
export { safeUrl as kbSafeUrl, formatTime as kbFormatTime } from './knowledgeBase.utils'
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
