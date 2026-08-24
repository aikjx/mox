// 产品体验增强：全局快捷键 composable（Phase A3）
// 设计要点：
// ① 所有事件以 window.addEventListener('keydown') 绑定，onBeforeUnmount 时通过返回的 dispose 解绑
// ② 快捷键避开浏览器默认：Ctrl+K（搜索框聚焦）、Ctrl+Shift+P（命令面板备用）、Ctrl+Shift+N（新建任务 Dialog）、Shift+?（帮助 Drawer）、Alt+1..9（跳 1-9 号模块）
// ③ 全部使用 CustomEvent 通知视图层（例如 TaskView 监听到 'xuanji:open-create-task' 后自行打开 Dialog），避免跨组件强耦合

import { onBeforeUnmount } from 'vue'

/**
 * @param {Object} api — 功能句柄集合（由 App.vue 传入，引用响应式变量）
 * @param {()=>void} api.focusSearch — 聚焦全局搜索输入框
 * @param {()=>void} api.toggleHelpDrawer — 切换快捷键帮助 Drawer 开/关
 * @param {Array<{path:string, key:string}>} api.navModules — NAV_MODULES 顺序即 Alt+1..N
 * @param {(path:string, query?:object)=>void} api.pushRoute — 路由跳转
 */
export function useGlobalShortcuts(api) {
  const handler = (e) => {
    if (!e || !e.target) return
    const tag = (e.target && e.target.tagName && e.target.tagName.toLowerCase()) || ''
    // 若用户正在内容编辑控件（input/textarea/contenteditable）中，除 Esc / 帮助外，其它快捷键取消（避免误触发）
    const isEditable = tag === 'input' || tag === 'textarea' || (e.target && e.target.isContentEditable)
    const ctrlOrMeta = Boolean(e.ctrlKey || e.metaKey)

    // Shift + ?  = 打开快捷键帮助（任何时刻生效，即使在 input 里也可用）
    if (!ctrlOrMeta && !e.altKey && e.shiftKey && (e.key === '?' || e.key === '/' || e.key === '？')) {
      e.preventDefault()
      api.toggleHelpDrawer()
      return
    }
    // Esc 统一解焦点：在搜索框/input 中，先 blur 一次
    if (e.key === 'Escape') {
      if (isEditable && e.target && typeof e.target.blur === 'function') {
        e.target.blur()
      }
      return
    }

    if (isEditable) return

    // Ctrl + K / Ctrl+Shift+P 聚焦全局搜索
    if (ctrlOrMeta && (e.key === 'k' || e.key === 'K')) {
      e.preventDefault()
      api.focusSearch()
      return
    }
    if (ctrlOrMeta && e.shiftKey && (e.key === 'p' || e.key === 'P')) {
      e.preventDefault()
      api.focusSearch()
      return
    }

    // Ctrl + Shift + N → 全局派发「新建任务打开 Dialog」事件
    if (ctrlOrMeta && e.shiftKey && (e.key === 'n' || e.key === 'N')) {
      e.preventDefault()
      window.dispatchEvent(new CustomEvent('xuanji:open-create-task'))
      return
    }

    // Alt + 1..9 → 跳转到 NAV_MODULES[0..8] 对应模块
    if (e.altKey && !ctrlOrMeta && /^[1-9]$/.test(e.key)) {
      const idx = parseInt(e.key, 10) - 1
      const mods = api.navModules || []
      const m = mods[idx]
      if (m && m.path) {
        e.preventDefault()
        api.pushRoute(m.path)
      }
    }
  }

  window.addEventListener('keydown', handler)
  const dispose = () => window.removeEventListener('keydown', handler)
  try { onBeforeUnmount(dispose) } catch (_) { /* 非 setup 调用时忽略，由 App.vue 自己解绑 */ }
  return dispose
}

// 轻量工具：push query 到路由（保留现有 query）
export function mergeQuery(base, extra) {
  return { ...(base || {}), ...(extra || {}) }
}

// 搜索历史存取（前缀避免其他工程冲突）
const HISTORY_PREFIX = 'xuanji_search_'
const HISTORY_LIMIT = 5

export function getSearchHistory(key, limit = HISTORY_LIMIT) {
  try {
    const raw = localStorage.getItem(HISTORY_PREFIX + key)
    const arr = raw ? JSON.parse(raw) : []
    return Array.isArray(arr) ? arr.slice(0, limit) : []
  } catch {
    return []
  }
}

/**
 * 写/清空搜索历史
 *  @param limit 为 0 且 term === '__CLEAR__' 表示清空历史
 */
export function pushSearchHistory(key, term, limit = HISTORY_LIMIT) {
  if (limit <= 0) {
    try { localStorage.removeItem(HISTORY_PREFIX + key) } catch {}
    return []
  }
  if (!term || term === '__CLEAR__' || !String(term).trim()) {
    return getSearchHistory(key, limit)
  }
  const t = String(term).trim()
  const arr = [t, ...getSearchHistory(key, limit).filter((x) => x !== t)].slice(0, limit)
  try { localStorage.setItem(HISTORY_PREFIX + key, JSON.stringify(arr)) } catch {}
  return arr
}
