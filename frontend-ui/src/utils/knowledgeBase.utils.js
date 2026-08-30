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

export function formatTime(ts) {
  if (!ts) return '-'
  const d = new Date(ts)
  if (isNaN(d)) return '-'
  const pad = n => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

export function truncateText(text, max) {
  if (!text) return ''
  return text.length > max ? text.slice(0, max) + '...' : text
}

// ========== 文档类型/状态映射 ==========

export const DOC_TYPES = [
  { value: 'article', label: '文章' },
  { value: 'tutorial', label: '教程' },
  { value: 'api', label: 'API 文档' },
  { value: 'design', label: '设计文档' },
  { value: 'report', label: '报告' },
  { value: 'spec', label: '规范' }
]

export function getTypeLabel(type) {
  return DOC_TYPES.find(t => t.value === type)?.label || type
}

export function getTagType(type) {
  const map = { article: 'info', tutorial: 'success', api: 'warning', design: 'info', report: 'danger', spec: 'info' }
  return map[type] || undefined
}

export function getStatusType(status) {
  return { published: 'success', draft: 'warning', archived: 'info' }[status] || 'info'
}

export function getStatusLabel(status) {
  return { published: '已发布', draft: '草稿', archived: '归档' }[status] || status
}

export function getActionLabel(action) {
  return { create: '创建', update: '更新', delete: '删除', analyze: 'AI 分析', revert: '回滚', link: '关联图谱' }[action] || action
}

// ========== 数据映射 ==========

export function mapDoc(d) {
  return { ...d, version_count: d.version || 1, ai_analyzed: !!d.aiAnalysis }
}

// ========== 标签尺寸计算 ==========

export function getTagSize(tags, count) {
  const min = 12, max = 20
  const maxCount = Math.max(...tags.map(t => t.count))
  if (maxCount === 0) return min
  return min + (count / maxCount) * (max - min)
}
