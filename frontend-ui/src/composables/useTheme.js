/**
 * MOX Theme Manager · 主题管理器
 * 三大默认主题：light（默认深空浅色） / dark / sky / cyberpunk
 *
 * 用法：
 *   import { useTheme } from '@/composables/useTheme'
 *   const { theme, setTheme, toggleTheme } = useTheme()
 *
 * 或直接在组件中：
 *   import useTheme from '@/composables/useTheme'
 */

import { ref, watch, onMounted } from 'vue'

const THEME_KEY = 'mox-theme'
const DEFAULT_THEME = 'light'

const availableThemes = [
  { key: 'light',     label: '浅色深空', swatch: 'linear-gradient(135deg, #f6f8fc, #e0e7ff)' },
  { key: 'dark',      label: '暗黑模式', swatch: 'linear-gradient(135deg, #0a0e1a, #1e293b)' },
  { key: 'sky',       label: '天蓝科技', swatch: 'linear-gradient(135deg, #0ea5e9, #06b6d4)' },
  { key: 'cyberpunk', label: '赛博朋克', swatch: 'linear-gradient(135deg, #00d4ff, #b14aff)' },
]

// 全局单例（避免多实例重复初始化）
let _theme = null
let _listeners = new Set()

function getStoredTheme() {
  if (typeof localStorage === 'undefined') return DEFAULT_THEME
  return localStorage.getItem(THEME_KEY) || DEFAULT_THEME
}

function setStoredTheme(theme) {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem(THEME_KEY, theme)
}

function applyTheme(theme) {
  if (typeof document === 'undefined') return
  const root = document.documentElement

  // 先禁用过渡，避免切换时闪烁
  root.classList.add('theme-no-transition')

  // 移除旧主题属性
  root.removeAttribute('data-theme')

  // 非默认主题才设置 data-theme
  if (theme && theme !== 'light') {
    root.setAttribute('data-theme', theme)
  }

  // 强制重排后恢复过渡
  // eslint-disable-next-line no-unused-expressions
  root.offsetHeight
  requestAnimationFrame(() => {
    root.classList.remove('theme-no-transition')
  })
}

function detectSystemTheme() {
  if (typeof window === 'undefined' || !window.matchMedia) return 'light'
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

/**
 * 主题管理 Composable
 * @returns {{ theme: import('vue').Ref<string>, setTheme: Function, toggleTheme: Function, availableThemes: Array }}
 */
export function useTheme() {
  if (!_theme) {
    _theme = ref(getStoredTheme())
  }

  // 应用主题到 DOM
  const apply = (theme) => {
    applyTheme(theme)
    setStoredTheme(theme)
    // 通知所有监听者
    _listeners.forEach(fn => fn(theme))
  }

  // 切换主题（循环切换）
  const toggleTheme = () => {
    const keys = availableThemes.map(t => t.key)
    const idx = keys.indexOf(_theme.value)
    const nextIdx = (idx + 1) % keys.length
    setTheme(keys[nextIdx])
  }

  // 设置主题
  const setTheme = (themeKey) => {
    if (!availableThemes.some(t => t.key === themeKey)) {
      console.warn(`[useTheme] 未知主题: ${themeKey}`)
      return
    }
    _theme.value = themeKey
    apply(themeKey)
  }

  // 监听变化
  const onThemeChange = (fn) => {
    _listeners.add(fn)
    return () => _listeners.delete(fn)
  }

  // 挂载时初始化
  onMounted(() => {
    const stored = getStoredTheme()
    if (stored) {
      apply(stored)
      _theme.value = stored
    } else {
      // 首次访问：检测系统偏好
      const systemTheme = detectSystemTheme()
      apply(systemTheme)
      _theme.value = systemTheme
    }
  })

  // 监听系统主题变化（仅当用户未手动设置时跟随系统）
  if (typeof window !== 'undefined' && window.matchMedia) {
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
      if (!localStorage.getItem(THEME_KEY)) {
        const newTheme = e.matches ? 'dark' : 'light'
        apply(newTheme)
        _theme.value = newTheme
      }
    })
  }

  return {
    theme: _theme,
    setTheme,
    toggleTheme,
    availableThemes,
    onThemeChange,
  }
}

export default useTheme
