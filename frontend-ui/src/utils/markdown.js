// Markdown 安全渲染工具
// 使用 markdown-it，禁用 HTML 注入，防止 XSS
import MarkdownIt from 'markdown-it'

// 创建 markdown 实例，安全配置
const md = new MarkdownIt({
  html: false,        // 禁用 HTML 标签注入（关键：防 XSS）
  linkify: true,      // 自动识别链接
  typographer: true,  // 排版优化
  breaks: true        // 换行转 <br>
})

// 自定义链接安全属性
md.renderer.rules.link_open = function (tokens, idx, options, env, self) {
  const token = tokens[idx]
  // 确保所有链接都有安全属性
  const hrefIndex = token.attrIndex('href')
  if (hrefIndex >= 0) {
    const href = token.attrs[hrefIndex][1]
    // 阻止 javascript: 协议
    if (/^javascript:/i.test(href.trim())) {
      token.attrs[hrefIndex][1] = '#'
    }
  }
  // 添加安全属性
  token.attrSet('target', '_blank')
  token.attrSet('rel', 'noopener noreferrer')
  return self.renderToken(tokens, idx, options)
}

// 代码块样式增强
md.renderer.rules.fence = function (tokens, idx, options, env, self) {
  const token = tokens[idx]
  const lang = token.info.trim()
  const code = token.content
  const langClass = lang ? ` class="language-${lang}"` : ''
  return `<pre class="code-block"><code${langClass}>${escapeHtml(code)}</code></pre>`
}

// 行内代码
md.renderer.rules.code_inline = function (tokens, idx) {
  const token = tokens[idx]
  return `<code class="inline-code">${escapeHtml(token.content)}</code>`
}

function escapeHtml(text) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

/**
 * 安全渲染 Markdown 为 HTML
 * @param {string} content - Markdown 文本
 * @returns {string} 安全的 HTML 字符串
 */
export function renderMarkdown(content) {
  if (!content) return ''
  try {
    return md.render(content)
  } catch (e) {
    console.warn('Markdown 渲染失败:', e)
    return escapeHtml(content)
  }
}

/**
 * 纯文本转义（用于非 markdown 场景）
 */
export function escapeText(text) {
  return escapeHtml(text || '')
}

export default md
