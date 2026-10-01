/**
 * 分类色板的 JS 侧出口。真值只在主题命名空间里（global.css 的 --cat-1…--cat-8 及各皮肤覆写），
 * 这里两条出口都不复制色值。
 *
 *   cat(n)      → 'var(--cat-N)'。给 <style> 块和 :style 用，浏览器自己跟着换肤。
 *   catColor(n) → 具体色值。给 canvas（echarts）和 SVG 的 fill/stroke 呈现属性用——
 *                 那两处按 CSS 声明解析不了 var()，只能在运行时把档位读成颜色。
 *
 * 读不到档时返回空串：宁可图上少一档颜色、看得见地错，也不在这里补一个兜底色值。
 */
import { ref } from 'vue'

export const CAT_COUNT = 8

// 1 基下标取模，负数与 0 也落回 1…CAT_COUNT（JS 的 % 会保留符号，所以补一次 CAT_COUNT）
const wrap = (n) => (((n - 1) % CAT_COUNT) + CAT_COUNT) % CAT_COUNT + 1

export const cat = (n) => `var(--cat-${wrap(n)})`

export const catColor = (n) => readVar(`--cat-${wrap(n)}`)

/**
 * 压字三档（--cat-N-fill 底 / --on-cat-N 字）的 CSS 侧出口：给 <style> 块和 :style 用，
 * 浏览器自己跟着换肤。字色与底色是成对契约，单取底档而字色写死就是跌破 AA。
 */
export const catFill = (n) => `var(--cat-${wrap(n)}-fill)`
export const catInk = (n) => `var(--on-cat-${wrap(n)})`

/** 同一契约的画布/SVG 侧出口：那两处按 CSS 声明解析不了 var()，只能运行时读档。 */
export const catFillColor = (n) => readVar(`--cat-${wrap(n)}-fill`)
export const catInkColor = (n) => readVar(`--on-cat-${wrap(n)}`)

/** 读任意档位（--text-tertiary / --border-soft 这些非分类色也走同一张嘴）。 */
export const tokenColor = (name) => readVar(name.startsWith('--') ? name : `--${name}`)

/**
 * 把色值压成带透明度的 rgba —— 画布上的渐变端点要的是具体色值，
 * 而透明度是几何量不是颜色，所以由它派生，不算第二份色值真值。
 * 认不出的输入原样返回（读不到档时还是"看得见地缺"）。
 */
export function withAlpha(color, alpha) {
  const m = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.exec(color)
  if (!m) return color
  const hex = m[1].length === 3 ? m[1].split('').map((c) => c + c).join('') : m[1]
  const [r, g, b] = [0, 2, 4].map((i) => parseInt(hex.slice(i, i + 2), 16))
  return `rgba(${r},${g},${b},${alpha})`
}

// 换肤时一起作废：缓存要清，读它的 computed 要重算，所以两件事挂在同一个 revision 上。
export const themeRevision = ref(0)
const cache = new Map()

if (typeof document !== 'undefined' && typeof MutationObserver === 'function') {
  new MutationObserver(() => {
    cache.clear()
    themeRevision.value += 1
  }).observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] })
}

function readVar(name) {
  if (typeof window === 'undefined' || typeof document === 'undefined') return ''
  void themeRevision.value
  if (!cache.has(name)) {
    cache.set(name, window.getComputedStyle(document.documentElement).getPropertyValue(name).trim())
  }
  return cache.get(name)
}
