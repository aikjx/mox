/**
 * palette.js 的两条出口各自要能被证伪：
 *   - cat() 只产 var() 引用，且下标取模对 0 / 负数也不逃出 1…8；
 *   - catColor() 把档位读成具体色值，读缓存、随 data-theme 作废，并且**不许自带兜底色**。
 * 每条断言都配了反例样本；跑真色值的活儿的验收在浏览器里做（见下文的 computed 重算用例）。
 */
import { describe, it, expect, afterEach, vi } from 'vitest'
import { computed } from 'vue'

const flush = () => new Promise((r) => setTimeout(r, 0))

// 模块级缓存与观察者：每条用例重新装载一份，避免上一轮的缓存冒充这一轮的结论
async function loadPalette() {
  vi.resetModules()
  return await import('./palette')
}

// 样式表读取要么给一张表，要么给一个函数（后者用来数"问了几次"，缓存才不是口头承诺）
function stubComputedStyle(source) {
  const original = window.getComputedStyle
  const get = (name) => (typeof source === 'function' ? source(name) : source[name] ?? '')
  window.getComputedStyle = () => ({ getPropertyValue: get })
  return () => { window.getComputedStyle = original }
}

describe('cat() —— CSS 侧引用', () => {
  it('产出 var(--cat-N)，不在 JS 里落成色值', async () => {
    const { cat } = await loadPalette()
    expect(cat(1)).toBe('var(--cat-1)')
    expect(cat(8)).toBe('var(--cat-8)')
  })

  it('下标越界折回 1…8，0 和负数也不例外', async () => {
    const { cat, CAT_COUNT } = await loadPalette()
    expect(cat(CAT_COUNT + 1)).toBe('var(--cat-1)')
    expect(cat(0)).toBe(`var(--cat-${CAT_COUNT})`)
    expect(cat(-7)).toBe('var(--cat-1)')
  })
})

describe('catColor() —— canvas 侧取值', () => {
  let restore

  afterEach(() => {
    restore?.()
    restore = undefined
    document.documentElement.removeAttribute('data-theme')
  })

  it('读的是命名空间里的当前值，不是写死的色值', async () => {
    const { catColor } = await loadPalette()
    restore = stubComputedStyle({ '--cat-1': ' #abcdef ' })
    expect(catColor(1)).toBe('#abcdef')
  })

  it('档位读不到就返回空串：宁可看得见地缺，也不补兜底色', async () => {
    const { catColor } = await loadPalette()
    restore = stubComputedStyle({ '--cat-2': '#112233' })
    expect(catColor(9)).toBe('')
    expect(catColor(2)).toBe('#112233')
  })

  it('同一档只问一次样式表，换肤后作废缓存', async () => {
    const { catColor } = await loadPalette()
    const table = { '--cat-3': '#000001' }
    const spy = vi.fn((name) => table[name] ?? '')
    restore = stubComputedStyle(spy)
    expect(catColor(3)).toBe('#000001')
    table['--cat-3'] = '#000002'
    expect(catColor(3)).toBe('#000001')
    expect(spy).toHaveBeenCalledTimes(1)
    document.documentElement.setAttribute('data-theme', 'dark')
    await flush()
    expect(catColor(3)).toBe('#000002')
    expect(spy).toHaveBeenCalledTimes(2)
  })

  it('换肤后，读它的 computed 会重算（图表跟皮肤走的那条链）', async () => {
    const { catColor, themeRevision } = await loadPalette()
    const table = { '--cat-4': '#0a0a0a' }
    restore = stubComputedStyle(table)
    const seriesColor = computed(() => (void themeRevision.value, catColor(4)))
    expect(seriesColor.value).toBe('#0a0a0a')
    table['--cat-4'] = '#0b0b0b'
    expect(seriesColor.value).toBe('#0a0a0a')
    document.documentElement.setAttribute('data-theme', 'sky')
    await flush()
    expect(seriesColor.value).toBe('#0b0b0b')
  })
})

describe('catFill() / catInk() —— 压字三档成对出口', () => {
  it('CSS 侧：底读 --cat-N-fill、字读 --on-cat-N，都不落成色值', async () => {
    const { catFill, catInk } = await loadPalette()
    expect(catFill(1)).toBe('var(--cat-1-fill)')
    expect(catInk(1)).toBe('var(--on-cat-1)')
    expect(catFill(8)).toBe('var(--cat-8-fill)')
    expect(catInk(8)).toBe('var(--on-cat-8)')
  })

  it('底与字说的是同一个档（越界取模后也不许错档：错档＝拿 A 的底压 B 的字，AA 失效）', async () => {
    const { catFill, catInk, CAT_COUNT } = await loadPalette()
    for (const n of [0, -7, CAT_COUNT + 1, CAT_COUNT + 9]) {
      const slot = (s) => /--(?:cat-|on-cat-)(\d+)/.exec(s)[1]
      expect(slot(catFill(n))).toBe(slot(catInk(n)))
    }
    expect(catFill(0)).toBe(`var(--cat-${CAT_COUNT}-fill)`)
    expect(catInk(0)).toBe(`var(--on-cat-${CAT_COUNT})`)
  })

  it('canvas 侧：读的是 -fill 与 on- 两档，不是文字档 --cat-N', async () => {
    const { catFillColor, catInkColor } = await loadPalette()
    const restore = stubComputedStyle({
      '--cat-5': '#000005', '--cat-5-fill': '#050005', '--on-cat-5': '#005005',
    })
    try {
      expect(catFillColor(5)).toBe('#050005')
      expect(catInkColor(5)).toBe('#005005')
      expect(catFillColor(6)).toBe('')
      expect(catInkColor(6)).toBe('')
    } finally {
      restore()
    }
  })
})

describe('withAlpha() —— 从档位派生透明度', () => {
  it('6 位/3 位 hex 都能压成 rgba，认不出的输入原样交回', async () => {
    const { withAlpha } = await loadPalette()
    expect(withAlpha('#6366f1', 0.3)).toBe('rgba(99,102,241,0.3)')
    expect(withAlpha('#abc', 1)).toBe('rgba(170,187,204,1)')
    expect(withAlpha('', 0.3)).toBe('')
    expect(withAlpha('rgba(1,2,3,0.5)', 0.3)).toBe('rgba(1,2,3,0.5)')
  })
})

describe('模块自身', () => {
  it('源码里不含任何 hex 字面量（扫描器本身带正例，防止空转）', async () => {
    // ?raw 而不是 fs.readFileSync(new URL(...))：vitest 下 import.meta.url 是 http 协议，不是 file
    const { default: source } = await import('./palette.js?raw')
    const HEX = /#[0-9a-fA-F]{3,8}\b/g
    expect(source.match(HEX)).toBe(null)
    expect('cat(1) -> #6366f1'.match(HEX)).toEqual(['#6366f1'])
  })
})
