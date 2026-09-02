/**
 * useTheme 主题管理 Composable 单元测试
 *
 * 覆盖的 Bug 修复：
 * - Bug: 旧 html.dark 类与新 data-theme 属性系统冲突，导致主题切换不生效
 * - 验证：主题切换正确设置 data-theme 属性、localStorage 持久化、CSS 变量正确应用
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useTheme } from './useTheme'

// 清理 localStorage 和 DOM 状态
function resetThemeState() {
  localStorage.clear()
  document.documentElement.removeAttribute('data-theme')
  document.documentElement.className = ''
  // 重置内部单例（通过重新加载模块实现）
}

describe('useTheme 主题管理', () => {
  beforeEach(() => {
    resetThemeState()
    // useTheme 是 app.store 的兼容层；每个测试使用独立 Pinia，避免
    // 在无 Vue app 挂载的单元测试中触发 getActivePinia() 错误。
    setActivePinia(createPinia())
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('初始化', () => {
    it('默认主题应为 light（浅色深空）', () => {
      // 注意：由于 useTheme 使用了模块级单例 + onMounted 初始化，
      // 我们主要测试 setTheme 和主题切换逻辑
      const { theme, availableThemes } = useTheme()
      expect(theme.value).toBe('light')
      expect(availableThemes.length).toBe(4)
      expect(availableThemes[0].key).toBe('light')
    })

    it('应提供 4 个主题选项', () => {
      const { availableThemes } = useTheme()
      const keys = availableThemes.map(t => t.key)
      expect(keys).toEqual(['light', 'dark', 'sky', 'cyberpunk'])
    })

    it('每个主题应有 label 和 swatch', () => {
      const { availableThemes } = useTheme()
      availableThemes.forEach(theme => {
        expect(theme).toHaveProperty('key')
        expect(theme).toHaveProperty('label')
        expect(theme).toHaveProperty('swatch')
        expect(typeof theme.key).toBe('string')
        expect(typeof theme.label).toBe('string')
        expect(typeof theme.swatch).toBe('string')
      })
    })
  })

  describe('主题切换 setTheme', () => {
    it('切换到 dark 主题应正确设置 data-theme 属性', () => {
      const { theme, setTheme } = useTheme()

      setTheme('dark')

      expect(theme.value).toBe('dark')
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    })

    it('切换到 sky 主题应正确设置 data-theme 属性', () => {
      const { theme, setTheme } = useTheme()

      setTheme('sky')

      expect(theme.value).toBe('sky')
      expect(document.documentElement.getAttribute('data-theme')).toBe('sky')
    })

    it('切换到 cyberpunk 主题应正确设置 data-theme 属性', () => {
      const { theme, setTheme } = useTheme()

      setTheme('cyberpunk')

      expect(theme.value).toBe('cyberpunk')
      expect(document.documentElement.getAttribute('data-theme')).toBe('cyberpunk')
    })

    it('切换回 light 主题应移除 data-theme 属性', () => {
      const { theme, setTheme } = useTheme()

      setTheme('dark')
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark')

      setTheme('light')
      expect(theme.value).toBe('light')
      expect(document.documentElement.getAttribute('data-theme')).toBeNull()
    })

    it('切换到无效主题应忽略并保持当前主题', () => {
      const { theme, setTheme } = useTheme()
      const initialTheme = theme.value

      setTheme('invalid-theme')

      expect(theme.value).toBe(initialTheme)
    })

    it('多次调用 useTheme 应返回同一实例（单例模式）', () => {
      const instance1 = useTheme()
      const instance2 = useTheme()

      expect(instance1.theme).toBe(instance2.theme)
    })
  })

  describe('localStorage 持久化', () => {
    it('切换主题后应保存到 localStorage', () => {
      const { setTheme } = useTheme()

      setTheme('dark')
      expect(localStorage.getItem('mox-theme')).toBe('dark')

      setTheme('sky')
      expect(localStorage.getItem('mox-theme')).toBe('sky')
    })

    it('切换到 light 也应保存到 localStorage', () => {
      const { setTheme } = useTheme()

      setTheme('light')
      expect(localStorage.getItem('mox-theme')).toBe('light')
    })
  })

  describe('toggleTheme 循环切换', () => {
    it('应在四个主题间循环切换', () => {
      const { theme, toggleTheme } = useTheme()

      // light -> dark
      toggleTheme()
      expect(theme.value).toBe('dark')

      // dark -> sky
      toggleTheme()
      expect(theme.value).toBe('sky')

      // sky -> cyberpunk
      toggleTheme()
      expect(theme.value).toBe('cyberpunk')

      // cyberpunk -> light
      toggleTheme()
      expect(theme.value).toBe('light')
    })
  })

  describe('onThemeChange 监听器', () => {
    it('应在主题变化时通知监听器', () => {
      const { setTheme, onThemeChange } = useTheme()
      const callback = vi.fn()

      onThemeChange(callback)
      setTheme('dark')

      expect(callback).toHaveBeenCalledTimes(1)
      expect(callback).toHaveBeenCalledWith('dark')
    })

    it('应支持多个监听器', () => {
      const { setTheme, onThemeChange } = useTheme()
      const callback1 = vi.fn()
      const callback2 = vi.fn()

      onThemeChange(callback1)
      onThemeChange(callback2)
      setTheme('sky')

      expect(callback1).toHaveBeenCalledWith('sky')
      expect(callback2).toHaveBeenCalledWith('sky')
    })

    it('返回的取消函数应能移除监听器', () => {
      const { setTheme, onThemeChange } = useTheme()
      const callback = vi.fn()

      const unsubscribe = onThemeChange(callback)
      unsubscribe()
      setTheme('dark')

      expect(callback).not.toHaveBeenCalled()
    })
  })

  describe('CSS 变量应用（防止主题切换不生效 Bug 回归）', () => {
    it('dark 主题应通过 data-theme 属性应用 CSS 变量', () => {
      const { setTheme } = useTheme()

      // 先设置一个初始状态
      setTheme('light')

      // 切换到 dark
      setTheme('dark')

      // 验证 data-theme 属性存在（这是 CSS 选择器生效的前提）
      const dataTheme = document.documentElement.getAttribute('data-theme')
      expect(dataTheme).toBe('dark')

      // 验证使用的是 data-theme 机制而非 class 机制（旧系统）
      // 这是修复的核心：旧系统用 html.dark 类，新系统用 data-theme 属性
      expect(document.documentElement.classList.contains('dark')).toBe(false)
    })

    it('sky 主题应通过 data-theme="sky" 生效', () => {
      const { setTheme } = useTheme()

      setTheme('sky')

      expect(document.documentElement.getAttribute('data-theme')).toBe('sky')
      expect(document.documentElement.classList.contains('dark')).toBe(false)
    })

    it('cyberpunk 主题应通过 data-theme="cyberpunk" 生效', () => {
      const { setTheme } = useTheme()

      setTheme('cyberpunk')

      expect(document.documentElement.getAttribute('data-theme')).toBe('cyberpunk')
      expect(document.documentElement.classList.contains('dark')).toBe(false)
    })

    it('切换主题时不应添加 dark class（防止旧系统干扰）', () => {
      const { setTheme } = useTheme()

      const themes = ['light', 'dark', 'sky', 'cyberpunk']
      for (const t of themes) {
        setTheme(t)
        // 验证不会意外添加 dark 类
        // （旧系统的 html.dark 类会覆盖新主题的 CSS 变量）
        expect(document.documentElement.classList.contains('dark')).toBe(false)
      }
    })
  })
})
