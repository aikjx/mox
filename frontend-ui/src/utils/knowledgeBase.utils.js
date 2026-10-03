/**
 * 知识库工具函数
 * Markdown 渲染、格式化、映射等纯函数
 */

// ========== Markdown 渲染器 ==========
// 安全加固：先 HTML-escape 用户文本再套 markdown，且链接仅允许安全协议，杜绝 v-html 存储型 XSS。

export function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

export function safeUrl(u) {
  const trimmed = (u || '').trim()
  if (!/^(https?:|mailto:|tel:|#)/i.test(trimmed)) return ''
  return trimmed.replace(/&/g, '&amp;').replace(/"/g, '&quot;')
}

/**
 * 简易 Markdown 渲染器
 * 支持：标题、粗体、斜体、行内代码、段落、换行、链接
 * @param {string} text
 * @returns {string} HTML 字符串（已转义，安全）
 */
export function simpleMarkdownRender(text) {
  if (!text) return ''
  let html = escapeHtml(text)
    .replace(/^### (.*$)/gm, '<h3>$1</h3>')
    .replace(/^## (.*$)/gm, '<h2>$1</h2>')
    .replace(/^# (.*$)/gm, '<h1>$1</h1>')
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.*?)\*/g, '<em>$1</em>')
    .replace(/`(.*?)`/g, '<code class="inline-code">$1</code>')
    .replace(/\n\n/g, '</p><p>')
    .replace(/\n/g, '<br/>')
    .replace(/\[(.*?)\]\((.*?)\)/g, (m, label, url) => {
      const href = safeUrl(url)
      if (!href) return label // 非法协议：仅显示文本，不渲染为链接
      return `<a href="${href}" target="_blank" rel="noopener noreferrer">${label}</a>`
    })
  return `<p>${html}</p>`
}

// ========== 格式化工具 ==========
// 时间口径不在这里：见 utils/time.js 的 formatDateTime（此前本文件与 message.utils.js 各抄了一份逐字符相同的副本）

export function truncateText(text, max) {
  if (!text) return ''
  return text.length > max ? text.slice(0, max) + '...' : text
}

// ========== 文档分类 / 状态词表（wire 真相）==========
// 后端 `mox-kb-svc` 的文档对象根本没有"类型"这个字段：分类只有 document.rs:28-33 的 CATEGORIES
// 四档（id 落在 doc.category 上），状态只有 model.rs:14-16 的 draft/analyzed/linked 三种。
// 这一处此前拿的是另一套词表（article/tutorial/api/design/report/spec 与
// published/draft/archived），与 wire 零重叠 ⇒ 按"类型"筛选恒为空、状态标签永远走原样显形的兜底。

export const KB_CATEGORIES = [
  { value: 'cat-tech', label: '技术文档', tagType: 'info', icon: '💻' },
  { value: 'cat-dialogue', label: '对话沉淀', tagType: 'success', icon: '💬' },
  { value: 'cat-business', label: '业务文档', tagType: 'warning', icon: '📊' },
  { value: 'cat-research', label: '研究文档', tagType: 'danger', icon: '🔍' }
]

export const KB_STATUSES = [
  { value: 'draft', label: '草稿', tagType: 'warning' },
  { value: 'analyzed', label: '已分析', tagType: 'success' },
  { value: 'linked', label: '已关联图谱', tagType: 'primary' }
]

const CATEGORY_BY_ID = Object.fromEntries(KB_CATEGORIES.map((c) => [c.value, c]))
const STATUS_BY_ID = Object.fromEntries(KB_STATUSES.map((s) => [s.value, s]))

/** 分类/状态不在词表里时原样显形——后端哪天加一档，界面要看得见而不是显示空白 */
export function getCategoryLabel(id) {
  return CATEGORY_BY_ID[id]?.label || id
}

export function getCategoryTagType(id) {
  return CATEGORY_BY_ID[id]?.tagType || 'info'
}

export function getCategoryIcon(id) {
  return CATEGORY_BY_ID[id]?.icon || '📄'
}

export function getStatusType(status) {
  return STATUS_BY_ID[status]?.tagType || 'info'
}

export function getStatusLabel(status) {
  return STATUS_BY_ID[status]?.label || status
}

/** AI 分析的痕迹只写在 status 上（analyzed 与 linked 都经过分析），wire 上没有 aiAnalysis */
export function isAiAnalyzed(status) {
  return status === 'analyzed' || status === 'linked'
}

export function getActionLabel(action) {
  return { create: '创建', update: '更新', delete: '删除', analyze: 'AI 分析', revert: '回滚', link: '关联图谱' }[action] || action
}

// ========== 数据映射 ==========

export function mapDoc(d) {
  // 版本数与"分析过"都从真实键推导：KbDocument 上是 current_version + versions[]（历史快照，不含当前版），
  // 而 aiAnalysis / version 这两个键后端从来不发。
  return {
    ...d,
    version_count: (d.versions?.length || 0) + 1,
    ai_analyzed: isAiAnalyzed(d.status)
  }
}

// Carry server-issued provenance and concurrency tokens; never invent defaults.
export function kbEntityMutation(doc, entity, linksRevision) {
  if (!doc?.current_version || !entity?.source_doc_id || !entity?.source_version || !entity?.id ||
      ![doc.acl_revision, entity.source_acl_revision, linksRevision].every(v => Number.isSafeInteger(v) && v >= 0)) {
    throw new Error('请刷新文档与实体来源后重试')
  }
  return {
    entity_id: entity.id, source_doc_id: entity.source_doc_id,
    source_version: entity.source_version, source_acl_revision: entity.source_acl_revision,
    expected_current_version: doc.current_version, expected_acl_revision: doc.acl_revision,
    expected_links_revision: linksRevision
  }
}

// ========== 标签尺寸计算 ==========

export function getTagSize(tags, count) {
  const min = 12, max = 20
  const maxCount = Math.max(...tags.map(t => t.count))
  if (maxCount === 0) return min
  return min + (count / maxCount) * (max - min)
}
