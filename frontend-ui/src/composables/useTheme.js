/**
 * MOX Theme Manager · 主题管理器（兼容层）
 * 底层已迁移至 Pinia app.store，此文件为向后兼容的 composable 封装
 *
 * 用法：
 *   import { useTheme } from '@/composables/useTheme'
 *   const { theme, setTheme, toggleTheme } = useTheme()
 *
 * 推荐新代码直接使用：
 *   import { useAppStore } from '@/stores'
 *   const appStore = useAppStore()
 *   // appStore.theme / appStore.setTheme() / appStore.toggleTheme()
 */

import { computed } from 'vue'
import { useAppStore, availableThemes } from '@/stores/app.store'

// 监听者集合（兼容旧 API）
const _listeners = new Set()

/**
 * 主题管理 Composable（兼容层）
 * @returns {{ theme: import('vue').ComputedRef<string>, setTheme: Function, toggleTheme: Function, availableThemes: Array, onThemeChange: Function }}
 */
export function useTheme() {
  const appStore = useAppStore()

  // 确保主题已初始化
  if (typeof window !== 'undefined' && appStore.theme === 'light') {
    appStore.initTheme()
  }

  // 兼容旧的 ref 风格访问
  const theme = computed(() => appStore.theme)

  function setTheme(themeKey) {
    const oldTheme = appStore.theme
    appStore.setTheme(themeKey)
    if (oldTheme !== appStore.theme) {
      _listeners.forEach(fn => fn(appStore.theme))
    }
  }

  function toggleTheme() {
    const oldTheme = appStore.theme
    appStore.toggleTheme()
    if (oldTheme !== appStore.theme) {
      _listeners.forEach(fn => fn(appStore.theme))
    }
  }

  function onThemeChange(fn) {
    _listeners.add(fn)
    return () => _listeners.delete(fn)
  }

  return {
    theme,
    setTheme,
    toggleTheme,
    availableThemes,
    onThemeChange,
  }
}

export default useTheme
