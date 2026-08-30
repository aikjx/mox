// 消息相关工具函数
// 从 MessageBubble 中提取的通用工具

/**
 * 安全 URL 处理：过滤危险协议
 */
export function safeUrl(url) {
  if (!url) return ''
  const trimmed = String(url).trim()
  if (!trimmed) return ''
  // 允许的协议
  const allowedProtocols = ['http:', 'https:', 'ftp:', 'mailto:', 'tel:']
  try {
    const u = new URL(trimmed, window.location.href)
    if (allowedProtocols.includes(u.protocol)) {
      return trimmed
    }
    return ''
  } catch {
    // 相对路径或锚点
    if (trimmed.startsWith('/') || trimmed.startsWith('#') || trimmed.startsWith('./')) {
      return trimmed
    }
    // 无协议的域名，自动补全 https
    if (/^[\w-]+(\.[\w-]+)+/.test(trimmed)) {
      return 'https://' + trimmed
    }
    return ''
  }
}

/**
 * 格式化文件大小
 */
export function formatSize(bytes) {
  if (bytes == null || isNaN(bytes)) return '-'
  const b = Number(bytes)
  if (b < 1024) return b + ' B'
  if (b < 1024 * 1024) return (b / 1024).toFixed(1) + ' KB'
  if (b < 1024 * 1024 * 1024) return (b / (1024 * 1024)).toFixed(1) + ' MB'
  return (b / (1024 * 1024 * 1024)).toFixed(2) + ' GB'
}

/**
 * 格式化时长（毫秒 → mm:ss 或 hh:mm:ss）
 */
export function formatDuration(ms) {
  if (!ms || ms < 0) return '0:00'
  const totalSeconds = Math.floor(ms / 1000)
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  const pad = n => String(n).padStart(2, '0')
  if (hours > 0) return `${hours}:${pad(minutes)}:${pad(seconds)}`
  return `${minutes}:${pad(seconds)}`
}

/**
 * 格式化时间戳
 */
export function formatTime(ts) {
  if (!ts) return '-'
  const d = new Date(ts)
  if (isNaN(d)) return '-'
  const pad = n => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

/**
 * 根据文件类型获取图标组件名
 */
export function artifactIcon(type) {
  const map = {
    pdf: 'Document',
    doc: 'Document',
    docx: 'Document',
    txt: 'Document',
    md: 'Document',
    xls: 'Grid',
    xlsx: 'Grid',
    csv: 'Grid',
    ppt: 'VideoCamera',
    pptx: 'VideoCamera',
    png: 'Picture',
    jpg: 'Picture',
    jpeg: 'Picture',
    gif: 'Picture',
    svg: 'Picture',
    mp4: 'VideoCamera',
    mov: 'VideoCamera',
    mp3: 'Headset',
    wav: 'Headset',
    zip: 'Folder',
    rar: 'Folder',
    '7z': 'Folder',
    code: 'Files',
    js: 'Files',
    ts: 'Files',
    py: 'Files',
    json: 'Files',
    html: 'Files',
    css: 'Files',
  }
  const ext = String(type || '').toLowerCase()
  return map[ext] || 'Document'
}

/**
 * Markdown 转纯文本（简易版，用于朗读、预览等）
 */
export function mdToPlainText(md) {
  if (!md) return ''
  return String(md)
    // 代码块
    .replace(/```[\s\S]*?```/g, (m) => {
      const code = m.replace(/```\w*\n?/g, '').trim()
      return '[代码] ' + code.slice(0, 100) + (code.length > 100 ? '…' : '')
    })
    // 行内代码
    .replace(/`([^`]+)`/g, '$1')
    // 图片
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, '[图片: $1]')
    // 链接
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
    // 标题
    .replace(/^#{1,6}\s+/gm, '')
    // 粗体/斜体
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/\*([^*]+)\*/g, '$1')
    // 列表
    .replace(/^[-*+]\s+/gm, '• ')
    // 引用
    .replace(/^>\s?/gm, '')
    // 水平线
    .replace(/^---+$/gm, '—')
    // HTML 标签
    .replace(/<[^>]+>/g, '')
    // 多余空行
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}

/**
 * 通用复制文本（兼容多种方式）
 */
export async function copyTextUniversal(text, successMsg = '已复制', showToast = true) {
  if (!text) return false
  try {
    // 优先使用 Clipboard API
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text)
      if (showToast) {
        // 延迟导入避免循环依赖
        const { ElMessage } = await import('element-plus')
        ElMessage.success({ message: successMsg, duration: 1500 })
      }
      return true
    }
  } catch (_) { /* fall through */ }

  try {
    // Fallback: textarea + execCommand
    const ta = document.createElement('textarea')
    ta.value = text
    ta.style.position = 'fixed'
    ta.style.opacity = '0'
    ta.style.left = '-9999px'
    document.body.appendChild(ta)
    ta.select()
    const ok = document.execCommand('copy')
    document.body.removeChild(ta)
    if (ok && showToast) {
      const { ElMessage } = await import('element-plus')
      ElMessage.success({ message: successMsg, duration: 1500 })
    }
    return ok
  } catch (_) {
    return false
  }
}

/**
 * 生成稳定的消息 ID（用于本地存储 key）
 */
export function stableMsgId(msg) {
  if (!msg) return 'unknown'
  return msg.id || msg.message_id || msg.timestamp?.toString() || String(Math.random())
}

/**
 * 从 DOM 同步代码块内容（用于复制代码）
 */
export function syncBlocksFromDom(rootEl) {
  if (!rootEl) return []
  const blocks = []
  const pres = rootEl.querySelectorAll('pre')
  pres.forEach((pre, idx) => {
    const codeEl = pre.querySelector('code')
    const lang = codeEl?.className?.match(/language-(\w+)/)?.[1] || ''
    const text = codeEl?.textContent || pre.textContent || ''
    blocks.push({
      index: idx,
      language: lang,
      text: text,
      element: pre,
    })
  })
  return blocks
}
