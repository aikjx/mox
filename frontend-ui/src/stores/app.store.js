// 应用级 Store - UI 状态、主题、侧边栏、健康状态等
import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'

const THEME_KEY = 'mox-theme'
const DEFAULT_THEME = 'light'
const REVIEW_KEY = 'mox-market-review'
const ADMIN_KEY = 'mox-is-admin'

export const availableThemes = [
  { key: 'light',     label: '浅色深空', swatch: 'linear-gradient(135deg, #f6f8fc, #e0e7ff)' },
  { key: 'dark',      label: '暗黑模式', swatch: 'linear-gradient(135deg, #0a0e1a, #1e293b)' },
  { key: 'sky',       label: '天蓝科技', swatch: 'linear-gradient(135deg, #0ea5e9, #06b6d4)' },
  { key: 'cyberpunk', label: '赛博朋克', swatch: 'linear-gradient(135deg, #00d4ff, #b14aff)' },
]

const themeKeys = availableThemes.map(t => t.key)

function getStoredTheme() {
  if (typeof localStorage === 'undefined') return DEFAULT_THEME
  return localStorage.getItem(THEME_KEY) || DEFAULT_THEME
}

function setStoredTheme(theme) {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem(THEME_KEY, theme)
}

function applyThemeToDOM(theme) {
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

export const useAppStore = defineStore('app', () => {
  // ===== State =====
  const sidebarCollapsed = ref(false)
  const theme = ref(DEFAULT_THEME)
  const health = ref({ status: 'pending', label: '连接中…' })
  const helpDrawerOpen = ref(false)
  const marketReviewEnabled = ref(false)
  const isAdmin = ref(false)

  // ===== Getters =====
  const isDark = computed(() => theme.value === 'dark')
  const currentThemeInfo = computed(() => {
    return availableThemes.find(t => t.key === theme.value) || availableThemes[0]
  })

  // ===== Theme Actions =====
  function setTheme(themeKey) {
    if (!themeKeys.includes(themeKey)) {
      // dev only: 未知主题键属内部配置错误，静默使用默认
      console.warn(`[appStore] 未知主题: ${themeKey}`)
      return
    }
    theme.value = themeKey
    applyThemeToDOM(themeKey)
    setStoredTheme(themeKey)
  }

  function toggleTheme() {
    const idx = themeKeys.indexOf(theme.value)
    const nextIdx = (idx + 1) % themeKeys.length
    setTheme(themeKeys[nextIdx])
  }

  function initTheme() {
    const stored = getStoredTheme()
    if (stored && themeKeys.includes(stored)) {
      setTheme(stored)
    } else {
      // 首次访问：检测系统偏好
      const systemTheme = detectSystemTheme()
      setTheme(systemTheme)
    }

    // 监听系统主题变化（仅当用户未手动设置时跟随系统）
    if (typeof window !== 'undefined' && window.matchMedia) {
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
        if (!localStorage.getItem(THEME_KEY)) {
          const newTheme = e.matches ? 'dark' : 'light'
          setTheme(newTheme)
        }
      })
    }
  }

  // ===== Sidebar Actions =====
  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }

  function setSidebarCollapsed(val) {
    sidebarCollapsed.value = val
  }

  // ===== Health Actions =====
  function setHealth(status, label) {
    health.value = { status, label }
  }

  // ===== Help Drawer Actions =====
  function toggleHelpDrawer() {
    helpDrawerOpen.value = !helpDrawerOpen.value
  }

  function openHelpDrawer() {
    helpDrawerOpen.value = true
  }

  function closeHelpDrawer() {
    helpDrawerOpen.value = false
  }

  // ===== Market Review Actions =====
  function loadMarketReview() {
    if (typeof localStorage === 'undefined') return
    const stored = localStorage.getItem(REVIEW_KEY)
    if (stored !== null) {
      marketReviewEnabled.value = stored === 'true'
    }
    const adminStored = localStorage.getItem(ADMIN_KEY)
    if (adminStored !== null) {
      isAdmin.value = adminStored === 'true'
    }
  }

  function setMarketReviewEnabled(val) {
    marketReviewEnabled.value = !!val
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(REVIEW_KEY, String(!!val))
    }
  }

  function toggleMarketReview() {
    setMarketReviewEnabled(!marketReviewEnabled.value)
  }

  function setIsAdmin(val) {
    isAdmin.value = !!val
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(ADMIN_KEY, String(!!val))
    }
  }

  function toggleAdmin() {
    setIsAdmin(!isAdmin.value)
  }

  return {
    // State
    sidebarCollapsed,
    theme,
    health,
    helpDrawerOpen,
    marketReviewEnabled,
    isAdmin,
    // Getters
    isDark,
    currentThemeInfo,
    availableThemes,
    // Theme Actions
    setTheme,
    toggleTheme,
    initTheme,
    // Sidebar Actions
    toggleSidebar,
    setSidebarCollapsed,
    // Health Actions
    setHealth,
    // Help Drawer Actions
    toggleHelpDrawer,
    openHelpDrawer,
    closeHelpDrawer,
    // Market Review Actions
    loadMarketReview,
    setMarketReviewEnabled,
    toggleMarketReview,
    setIsAdmin,
    toggleAdmin,
  }
})
