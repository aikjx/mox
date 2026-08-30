// UI Store - 全局 UI 交互状态：通知、加载、抽屉、模态框等
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

/**
 * 通知项结构
 * { id, type, title, message, duration, read }
 */
let _notificationId = 0

export const useUiStore = defineStore('ui', () => {
  // ===== State =====

  // 全局通知列表
  const notifications = ref([])

  // 全局加载状态（用于页面级加载遮罩）
  const globalLoading = ref(false)
  const globalLoadingText = ref('加载中…')

  // AI 全屏模式
  const aiFullscreen = ref(false)

  // 当前阶段（用于全维流程条）
  const currentPhase = ref('s1')

  // 命令面板可见性
  const commandPaletteOpen = ref(false)

  // ===== Getters =====

  // 未读通知数
  const unreadCount = computed(() => {
    return notifications.value.filter(n => !n.read).length
  })

  // 最新通知
  const latestNotification = computed(() => {
    return notifications.value[0] || null
  })

  // ===== Notification Actions =====

  function notify({ type = 'info', title = '', message = '', duration = 4000 }) {
    const id = ++_notificationId
    const item = { id, type, title, message, read: false, time: Date.now() }
    notifications.value.unshift(item)

    // 自动移除
    if (duration > 0) {
      setTimeout(() => {
        removeNotification(id)
      }, duration)
    }

    return id
  }

  function success(title, message = '', duration = 3000) {
    return notify({ type: 'success', title, message, duration })
  }

  function error(title, message = '', duration = 5000) {
    return notify({ type: 'error', title, message, duration })
  }

  function warning(title, message = '', duration = 4000) {
    return notify({ type: 'warning', title, message, duration })
  }

  function info(title, message = '', duration = 4000) {
    return notify({ type: 'info', title, message, duration })
  }

  function removeNotification(id) {
    const idx = notifications.value.findIndex(n => n.id === id)
    if (idx > -1) {
      notifications.value.splice(idx, 1)
    }
  }

  function markAsRead(id) {
    const item = notifications.value.find(n => n.id === id)
    if (item) item.read = true
  }

  function markAllAsRead() {
    notifications.value.forEach(n => n.read = true)
  }

  function clearNotifications() {
    notifications.value = []
  }

  // ===== Global Loading Actions =====

  function showLoading(text = '加载中…') {
    globalLoadingText.value = text
    globalLoading.value = true
  }

  function hideLoading() {
    globalLoading.value = false
  }

  // ===== AI Fullscreen Actions =====

  function toggleAIFullscreen() {
    aiFullscreen.value = !aiFullscreen.value
  }

  function enterAIFullscreen() {
    aiFullscreen.value = true
  }

  function exitAIFullscreen() {
    aiFullscreen.value = false
  }

  // ===== Phase Actions =====

  function setPhase(phase) {
    currentPhase.value = phase
  }

  // ===== Command Palette Actions =====

  function openCommandPalette() {
    commandPaletteOpen.value = true
  }

  function closeCommandPalette() {
    commandPaletteOpen.value = false
  }

  function toggleCommandPalette() {
    commandPaletteOpen.value = !commandPaletteOpen.value
  }

  return {
    // State
    notifications,
    globalLoading,
    globalLoadingText,
    aiFullscreen,
    currentPhase,
    commandPaletteOpen,
    // Getters
    unreadCount,
    latestNotification,
    // Notification Actions
    notify,
    success,
    error,
    warning,
    info,
    removeNotification,
    markAsRead,
    markAllAsRead,
    clearNotifications,
    // Global Loading Actions
    showLoading,
    hideLoading,
    // AI Fullscreen Actions
    toggleAIFullscreen,
    enterAIFullscreen,
    exitAIFullscreen,
    // Phase Actions
    setPhase,
    // Command Palette Actions
    openCommandPalette,
    closeCommandPalette,
    toggleCommandPalette,
  }
})
