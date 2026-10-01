// 样式契约门禁：本模块的视觉只能用已定义的主题令牌，且各页面/组件的类名不得互相抢占。
// 后者源于一次真实故障：控制台与协作面板都用根类名 `ac`，页面内 querySelector('.ac') 命中了控制台。
import { describe, it, expect } from 'vitest'
import { readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'

const SRC = path.join(process.cwd(), 'src')
const MODULE = path.join(SRC, 'modules', 'expert-alliance')

const walk = (dir) =>
  readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)]
  )

const vueFiles = walk(MODULE).filter((f) => f.endsWith('.vue'))

// <style> 块内容（可能有多段）
const stylesOf = (file) =>
  (readFileSync(file, 'utf8').match(/<style[^>]*>([\s\S]*?)<\/style>/g) || [])
    .map((b) => b.replace(/<style[^>]*>/, '').replace(/<\/style>/, ''))
    .join('\n')

// 主题令牌权威源：global.css 的 :root 与所有主题文件
const tokenSources = [
  path.join(SRC, 'styles', 'global.css'),
  ...readdirSync(path.join(SRC, 'styles', 'themes'))
    .filter((f) => f.endsWith('.css'))
    .map((f) => path.join(SRC, 'styles', 'themes', f))
]
const DEFINED = new Set(
  tokenSources
    .map((f) => readFileSync(f, 'utf8'))
    .join('\n')
    .match(/--[a-z0-9-]+(?=\s*:)/g) || []
)

describe('主题令牌', () => {
  it('模块根样式文件确实解析出了令牌，否则本门禁是空转', () => {
    expect(DEFINED.size).toBeGreaterThan(40)
    expect(DEFINED.has('--brand')).toBe(true)
  })

  it('样式里引用的每个 var(--x) 都在主题中有定义', () => {
    for (const file of vueFiles) {
      const used = [...new Set((stylesOf(file).match(/var\(--[a-z0-9-]+/g) || []).map((s) => s.slice(4)))]
      for (const token of used) {
        expect(DEFINED.has(token), `${path.basename(file)} 用了未定义的令牌 ${token}（浏览器会静默丢弃该声明）`).toBe(true)
      }
    }
  })

  it('颜色一律走令牌，不出现裸 hex / rgb() / hsl()', () => {
    for (const file of vueFiles) {
      const css = stylesOf(file)
      const rel = path.relative(MODULE, file)
      expect(css, rel).not.toMatch(/#[0-9a-fA-F]{3,8}\b/)
      expect(css, rel).not.toMatch(/\brgba?\(/)
      expect(css, rel).not.toMatch(/\bhsla?\(/)
    }
  })

  it('圆角用令牌而不是裸 px', () => {
    for (const file of vueFiles) {
      const css = stylesOf(file)
      expect(css, path.relative(MODULE, file)).not.toMatch(/border-radius:\s*[\d.]+px/)
    }
  })
})

describe('外壳令牌可读性', () => {
  const globalCss = readFileSync(path.join(SRC, 'styles', 'global.css'), 'utf8')
  const root = globalCss.match(/:root\s*\{([\s\S]*?)\n\}/)[1]
  const declared = {}
  for (const m of root.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) declared[m[1]] = m[2].trim()
  const valueOf = (token, seen = 0) => {
    if (seen > 4) throw new Error(`${token} 变量自引用成环`)
    const v = declared[token]
    if (v === undefined) throw new Error(`global.css 的 :root 里没有 ${token}`)
    const ind = v.match(/^var\((--[\w-]+)\)$/)
    return ind ? valueOf(ind[1], seen + 1) : v
  }
  const rgb = (v) => {
    const hex = v.match(/^#([\da-f]{6})$/i)
    if (hex) return [0, 2, 4].map((i) => parseInt(hex[1].slice(i, i + 2), 16))
    const fn = v.match(/rgba?\(([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)/)
    if (fn) return [+fn[1], +fn[2], +fn[3]]
    return null
  }
  const lum = ([r, g, b]) => {
    const f = (c) => (c /= 255) <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
    return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
  }
  const contrast = (fg, bg) => {
    const [a, b] = [lum(fg), lum(bg)]
    return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)
  }
  // 卡面族：本模块的降级面孔都落在这几块实心面上。--bg-hover 是行/块的悬停态，
  // 品牌色文字（--accent-light 在其上 4.43:1）不为它改品牌取值，但灰阶正文必须过，
  // 所以下面两个用例分别用 SOLID 与 SOLID+hover。
  const SOLID = ['--bg-primary', '--bg-secondary', '--bg-tertiary', '--bg-card']
  const surface = (t) => rgb(valueOf(t))
  const grays = ['--text-primary', '--text-secondary', '--text-tertiary', '--text-quaternary', '--text-muted']

  it('模块里当作正文色的每个令牌，在全部实心面上都到 AA 4.5:1', () => {
    const used = new Set()
    for (const file of vueFiles) {
      for (const m of stylesOf(file).matchAll(/(?:^|[\s;{])color:\s*var\((--[\w-]+)\)/g)) used.add(m[1])
    }
    expect(used.size).toBeGreaterThan(3)
    for (const token of used) {
      const fg = rgb(valueOf(token))
      expect(fg, `${token} 在 global.css 里解析不出颜色，门禁判不了`).toBeTruthy()
      for (const bg of SOLID) {
        const r = contrast(fg, surface(bg))
        expect(r, `${token}=${valueOf(token)} 压在 ${bg} 上只有 ${r.toFixed(2)}:1`).toBeGreaterThanOrEqual(4.5)
      }
    }
  })

  it('灰阶五级连悬停面都不低于 AA', () => {
    for (const token of grays) {
      for (const bg of [...SOLID, '--bg-hover']) {
        const r = contrast(rgb(valueOf(token)), surface(bg))
        expect(r, `${token}=${valueOf(token)} 压在 ${bg} 上 ${r.toFixed(2)}:1`).toBeGreaterThanOrEqual(4.5)
      }
    }
  })

  it('默认（深色）外壳下 EP 的 light-8/9 序列不得是近白色，且空白底令牌必须覆写', () => {
    let checked = 0
    for (const m of root.matchAll(/(--el-color-[\w-]+-light-[89])\s*:\s*([^;]+);/g)) {
      // 允许写成 var() 别名（归一之后真值在主题命名空间里），所以取解析终值而不是声明原文。
      const c = rgb(valueOf(m[1]))
      checked++
      expect(c && lum(c) > 0.6, `${m[1]} = ${valueOf(m[1])} 是近白色：深色外壳上读它的组件会带着白底坐在卡面里`).toBe(false)
    }
    expect(checked).toBeGreaterThan(6)
    expect(declared['--el-fill-color-blank'], 'EP 有 28 个组件样式表读 --el-fill-color-blank，外壳不覆写就是 #fff').toBeDefined()
    expect(/^var\(|^#[0-9a-f]{3,6}$/i.test(declared['--el-fill-color-blank']), '空白底必须引令牌').toBe(true)
  })
})

describe('类名归属', () => {
  const rootClassOf = (file) => {
    const m = readFileSync(file, 'utf8').match(/<template>\s*<\w[\w-]*[^>]*class="([^"]+)"/)
    expect(m, `${path.basename(file)} 根元素没有 class，无法纳入门禁`).toBeTruthy()
    return m[1].trim()
  }
  // 只取「块级」类名（选择器里位于起始、组合符或 @media 括号之后的那个 `.x`）：
  // `.a.is-x` 中的 `.is-x` 是修饰符，必须与自有块级类同现才命中，跨文件重名不构成误命中；
  // 块级类才可能被 querySelector 直接抓到，也正是 `ac` 撞名的那一类。
  const classesOf = (file) => {
    const selectors = stylesOf(file).replace(/\{[^{}]*\}/g, '{}')
    return new Set(
      [...selectors.matchAll(/(?:^|[\s>+~,{:])\.([a-z][a-z0-9-]*)/g)]
        .map((m) => m[1])
        .filter((c) => !c.startsWith('el-'))
    )
  }

  it('每个 .vue 的根类名在本模块内唯一', () => {
    const roots = vueFiles.map(rootClassOf)
    expect(new Set(roots).size).toBe(roots.length)
  })

  it('两个 .vue 之间不共享任何样式类名，跨页选择器不会误命中', () => {
    const seen = new Map()
    for (const file of vueFiles) {
      for (const cls of classesOf(file)) {
        const prev = seen.get(cls)
        expect(prev, `类名 .${cls} 同时出现在 ${prev} 与 ${path.basename(file)}`).toBeUndefined()
        seen.set(cls, path.basename(file))
      }
    }
    expect(seen.size).toBeGreaterThan(50)
  })
})

// ===== 解析器口径的两道门（2026-09-25 事故驱动）=====
// 起因：别名层的说明注释里写了 `-sidebar-*/-input`，其中「星号紧跟斜杠」让块注释在 138 字符处提前闭合，
// 剩下的中文成了非法选择器，浏览器于是**整条 :root 规则都不吃** —— 全站令牌没有定义。
// 上面那些用例都是按正则读文件文本的，读得到就照旧绿，所以这里必须换成"按规则读"的口径。
const stripCommentsLikeBrowser = (text) => {
  let out = ''
  for (let i = 0; i < text.length; i++) {
    if (text[i] === '/' && text[i + 1] === '*') {
      const end = text.indexOf('*/', i + 2)
      const stop = end < 0 ? text.length : end + 2
      out += text.slice(i, stop).replace(/[^\n]/g, ' ') // 保行号，内容抹平
      i = stop - 1
    } else out += text[i]
  }
  return out
}
// 只取顶层块（深度 0 的 {...}），选择器就是它前面那段代码文本
const topLevelBlocks = (code) => {
  const blocks = []
  let depth = 0, selStart = 0
  for (let i = 0; i < code.length; i++) {
    const ch = code[i]
    if (ch === '{') {
      if (depth === 0) blocks.push({ sel: code.slice(selStart, i), body: '', selEnd: i })
      depth++
    } else if (ch === '}') {
      depth--
      if (depth === 0) { blocks[blocks.length - 1].body = code.slice(blocks[blocks.length - 1].selEnd + 1, i); selStart = i + 1 }
    }
  }
  return depth === 0 ? blocks : [...blocks, { sel: 'UNCLOSED', body: '', selEnd: -1 }]
}
const parseDecls = (body) => {
  const m = {}
  for (const d of body.matchAll(/(--[\w-]+)\s*:\s*([^;]+)/g)) if (!(d[1] in m)) m[d[1]] = d[2].trim()
  return m
}

describe('规则真的被解析器采纳', () => {
  const files = tokenSources.filter((f) => f.endsWith('.css'))

  it('没有哪条顶层规则的选择器里混进了注释正文（星号斜杠事故的指纹）', () => {
    let checked = 0
    for (const file of files) {
      for (const b of topLevelBlocks(stripCommentsLikeBrowser(readFileSync(file, 'utf8')))) {
        checked++
        expect(b.sel, `${path.basename(file)} 有未闭合的块`).not.toBe('UNCLOSED')
        expect(/[\u4e00-\u9fff`]/.test(b.sel), `${path.basename(file)}：{ 之前的选择器含中文/反引号 ⇒ 说明上文有注释提前闭合`).toBe(false)
      }
    }
    expect(checked).toBeGreaterThan(20)
  })

  it('global.css 的 :root 剥掉注释后仍然在，且别名层与灰阶全在里面', () => {
    const code = stripCommentsLikeBrowser(readFileSync(path.join(SRC, 'styles', 'global.css'), 'utf8'))
    const roots = topLevelBlocks(code).filter((b) => b.sel.trim() === ':root')
    expect(roots.length, '解析器眼里的 :root 块数量：0 就是整条规则被丢了').toBe(1)
    for (const t of ['--bg-card:', '--bg-hover:', '--bg-row-hover:', '--text-tertiary:', '--text-quaternary:', '--el-fill-color-blank:']) {
      expect(roots[0].body, `:root 里少了 ${t}`).toContain(t)
    }
  })
})

describe('四套皮肤的灰阶可读性', () => {
  // 默认皮肤 = global.css 的 :root；另外三套 = 各自主题文件里那条裸的 html[data-theme="x"]。
  // 外壳名（--bg-card 等）在 global 里是别名，取值要沿着"皮肤覆写 → global 兜底"的链解析下去。
  const globalCss = readFileSync(path.join(SRC, 'styles', 'global.css'), 'utf8')
  // 丢了 :root 时这里要给出**指名的红**，不能让整份文件在收集期崩掉（那会把另外 13 条用例一起带走，
  // 也看不出是哪道门起的作用）——2026-09-25 那次事故的复盘结论。
  const globalRootBlock = topLevelBlocks(stripCommentsLikeBrowser(globalCss)).find((b) => b.sel.trim() === ':root')
  const GLOBAL = globalRootBlock ? parseDecls(globalRootBlock.body) : null
  const rootAdopted = () => expect(GLOBAL, 'global.css 的 :root 没有被解析器采纳（整条规则被丢了）').toBeTruthy()
  const skins = { default: {} }
  for (const file of readdirSync(path.join(SRC, 'styles', 'themes')).filter((f) => f.endsWith('.css'))) {
    const code = stripCommentsLikeBrowser(readFileSync(path.join(SRC, 'styles', 'themes', file), 'utf8'))
    for (const b of topLevelBlocks(code)) {
      const m = b.sel.trim().match(/^html\[data-theme="([a-z]+)"\]$/)
      if (m) skins[m[1]] = { ...parseDecls(b.body), ...skins[m[1]] }
    }
  }
  const skinsWithOwnBlock = Object.keys(skins).filter((k) => k === 'default' || Object.keys(skins[k]).length)
  const lookup = (name, skin, seen = 0) => {
    if (seen > 6) throw new Error(`${skin} 的 ${name} 变量链不成环解析：自引用`)
    const v = skins[skin]?.[name] ?? GLOBAL[name]
    if (v === undefined) throw new Error(`${skin} 皮肤下 ${name} 没有定义`)
    const ind = v.match(/^var\((--[\w-]+)\)$/)
    return ind ? lookup(ind[1], skin, seen + 1) : v
  }
  const rgb = (v) => {
    const hex = v.match(/^#([\da-f]{6})$/i)
    if (hex) return [0, 2, 4].map((i) => parseInt(hex[1].slice(i, i + 2), 16))
    const fn = v.match(/rgba?\(([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)/)
    if (fn) return [+fn[1], +fn[2], +fn[3]]
    return null
  }
  const lum = ([r, g, b]) => {
    const f = (c) => (c /= 255) <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
    return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
  }
  const contrast = (fg, bg) => {
    const [a, b] = [lum(fg), lum(bg)]
    return +((Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)).toFixed(2)
  }
  const SOLID = ['--bg-primary', '--bg-secondary', '--bg-tertiary', '--bg-card', '--bg-hover', '--bg-row-hover']
  // 正文级必须过 AA；quaternary 是占位/禁用级，只压 3.0 ——暗色皮肤上它不可能"既弱于 tertiary 又 ≥4.5"，
  // 这是明写的规则，不是静默跳过。
  const BODY = ['--text-primary', '--text-secondary', '--text-tertiary', '--text-muted']

  it('三套主题加默认皮肤一共四套都在门里', () => {
    expect(skinsWithOwnBlock.sort()).toEqual(['cyberpunk', 'dark', 'default', 'sky'])
  })

  it('正文级灰阶在每个实心面（含新的行悬停档）上都不低于 AA 4.5:1', () => {
    rootAdopted()
    for (const skin of skinsWithOwnBlock) {
      for (const fg of BODY) {
        for (const bg of SOLID) {
          const f = rgb(lookup(fg, skin))
          const b = rgb(lookup(bg, skin))
          expect(b, `${skin} 的 ${bg} 解析不出颜色（=${lookup(bg, skin)}）`).toBeTruthy()
          const r = contrast(f, b)
          expect(r, `${skin}：${fg}=${lookup(fg, skin)} 压在 ${bg}=${lookup(bg, skin)} 上只有 ${r}:1`).toBeGreaterThanOrEqual(4.5)
        }
      }
    }
  })

  it('占位/禁用级至少 3.0:1，且行悬停档确实比卡面更抬起', () => {
    rootAdopted()
    for (const skin of skinsWithOwnBlock) {
      const q = rgb(lookup('--text-quaternary', skin))
      for (const bg of SOLID) {
        const r = contrast(q, rgb(lookup(bg, skin)))
        expect(r, `${skin}：--text-quaternary=${lookup('--text-quaternary', skin)} 压在 ${bg} 上 ${r}:1`).toBeGreaterThanOrEqual(3)
      }
      const [row, card] = [lum(rgb(lookup('--bg-row-hover', skin))), lum(rgb(lookup('--bg-card', skin)))]
      expect(Math.abs(row - card) > 0.004, `${skin}：行悬停面 ${lookup('--bg-row-hover', skin)} 与卡面 ${lookup('--bg-card', skin)} 看不出差别`).toBe(true)
    }
  })

  // EP 的语义色对（提示条/告警条就是这一对：light-9 是底、主色是字）。它们在默认皮肤下看着还行，
  // 换到浅色皮肤就会翻车 —— sky 下实测 #f56c6c 压在 #fef0f0 上只有 2.61:1，每页 9~10 处。
  // 所以判据必须是"每套皮肤各自可读"，而不是"底不许接近白"（那只在深色外壳成立）。
  // 待检类型**从模块模板里派生**，不是手写的名单：上一版手写 ['danger','success','warning'] 漏了 info，
  // 于是外壳把 --el-color-info 写成深色皮肤的字面量没人管，sky 白皮下 el-tag type=info 实测 2.54:1，门禁全绿。
  const EP_TYPES = [...new Set(
    (vueFiles
      .map((f) => readFileSync(f, 'utf8'))
      .join('\n')
      .match(/<el-(?:tag|alert)\b[^>]*?\stype="([a-z]+)"/g) || [])
      .map((s) => /\stype="([a-z]+)"/.exec(s)[1])
      .filter((t) => !['primary', 'text', 'default'].includes(t))
  )].sort()

  it('派生出的待检语义类型非空（否则上面那条门禁是空转）', () => {
    expect(EP_TYPES.length, `模块模板里没解析出任何 el-tag/el-alert 的 type`).toBeGreaterThanOrEqual(3)
  })

  it('EP 语义色对（主色字 × light-9 底）在四套皮肤上都不低于 AA 4.5:1', () => {
    rootAdopted()
    const withAlpha = (v) => {
      const hex = v.match(/^#([\da-f]{6})$/i)
      if (hex) return { c: [0, 2, 4].map((i) => parseInt(hex[1].slice(i, i + 2), 16)), a: 1 }
      const fn = v.match(/rgba?\(([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)/)
      return fn ? { c: [+fn[1], +fn[2], +fn[3]], a: fn[4] === undefined ? 1 : +fn[4] } : null
    }
    for (const skin of skinsWithOwnBlock) {
      const page = withAlpha(lookup('--bg-primary', skin))
      for (const k of EP_TYPES) {
        const decl = GLOBAL[`--el-color-${k}`]
        expect(decl, `模块用了 type="${k}"，但 --el-color-${k} 无人覆写 ⇒ 组件退回 theme-chalk 字面量`).toBeTruthy()
        expect(decl, `--el-color-${k}=${decl} 是外壳字面量 ⇒ 换肤覆写不到它，语义色有了第二份真值`).toContain('var(--')
        const fg = withAlpha(lookup(`--el-color-${k}`, skin))
        const bgDecl = lookup(`--el-color-${k}-light-9`, skin)
        const bg = withAlpha(bgDecl)
        expect(bg, `${skin}：--el-color-${k}-light-9=${bgDecl} 解析不出颜色`).toBeTruthy()
        const composited = bg.c.map((v, i) => Math.round(page.c[i] + (v - page.c[i]) * bg.a))
        const r = contrast(fg.c, composited)
        expect(r, `${skin}：--el-color-${k}=${lookup(`--el-color-${k}`, skin)} 压在 light-9 底 ${bgDecl}（合成后 rgb(${composited.join(',')})）上只有 ${r}:1`).toBeGreaterThanOrEqual(4.5)
      }
    }
  })

  // 上面那条全绿，真机的 error toast 却仍是 #f56c6c 压在 #fef0f0 上（实测 3.16:1）。根因：
  // theme-chalk 里 error 是与 danger **平行**的另一套字面量，而 message / alert / notification /
  // check-tag / link / tag 的报错态读的是 --el-color-error 这一族（当场查了载入的规则：33 处引用）。
  // 判据因此不能只盯"我们覆写过的名字"，必须盯"组件实际读的那个名字有没有第二份真值"。
  it('error 家族逐条别名到 danger，不允许留 theme-chalk 的字面量', () => {
    rootAdopted()
    const suffixes = ['', '-light-3', '-light-5', '-light-7', '-light-8', '-light-9', '-dark-2']
    for (const s of suffixes) {
      const name = `--el-color-error${s}`
      const decl = GLOBAL[name]
      expect(decl, `${name} 无人覆写 ⇒ 组件退回 theme-chalk 的 #f56c6c 字面量`).toBeTruthy()
      expect(decl, `${name}=${decl} 没指向 danger 家族，语义红有了第二份真值`).toContain('--el-color-danger')
    }
    for (const skin of skinsWithOwnBlock) {
      for (const s of ['', '-light-8', '-light-9']) {
        const a = lookup(`--el-color-error${s}`, skin)
        const b = lookup(`--el-color-danger${s}`, skin)
        expect(a, `${skin}：--el-color-error${s} 别名链解析不出值`).toBeTruthy()
        expect(a, `${skin}：error${s}=${a} 与 danger${s}=${b} 不同值`).toBe(b)
      }
    }
  })

  // 「字压在色块上」是第三类病因，和前两类（灰阶、语义色对）都不一样：填充色与字色必须**成对**决定。
  // 实测把 --accent 当底的地方白字只有 1.81（dark #22d3ee）/ 2.43（sky #06b6d4）/ 3.97（cyberpunk #b14aff）:1，
  // 而 --brand-500 当底在默认皮肤下也只有 4.47:1 —— 所以亮档留给装饰，填充走 --brand-fill，
  // 压在填充上的字走 --on-brand（霓虹皮肤保留身份色的方式是反过来把字改深，不是把底压暗）。
  it('品牌填充档与 on-brand 字色成对可读，四套皮肤各自覆写', () => {
    rootAdopted()
    for (const skin of skinsWithOwnBlock) {
      const on = rgb(lookup('--on-brand', skin))
      for (const name of ['--brand-fill', '--brand-fill-hover']) {
        const fill = rgb(lookup(name, skin))
        expect(fill, `${skin}：${name}=${lookup(name, skin)} 解析不出颜色`).toBeTruthy()
        const r = contrast(on, fill)
        expect(r, `${skin}：--on-brand=${lookup('--on-brand', skin)} 压在 ${name}=${lookup(name, skin)} 上只有 ${r}:1`).toBeGreaterThanOrEqual(4.5)
      }
      if (skin !== 'default') {
        for (const name of ['--brand-fill', '--brand-fill-hover', '--on-brand']) {
          expect(skins[skin][name], `${skin} 没有覆写 ${name} ⇒ 换肤后按钮底仍是靛蓝，跟皮肤无关`).toBeTruthy()
        }
      }
    }
  })

  // 外壳里那四处"字压在品牌块上"的站点必须读这一对。判据取正向（规则块里必须出现 var(--brand-fill)），
  // 不写"禁止 --accent"：否定式字面禁令换个写法就绕过去了。
  it('外壳的四个品牌填充站点真的接到了档位上', () => {
    const sites = [
      ['TheTopbar.vue', '.quick-create-btn {'],
      ['TheTopbar.vue', '.user-avatar {'],
      ['IconSidebar.vue', '.icon-logo {'],
      ['TheSidebar.vue', '.ms-sidebar-item.active .ms-item-count {'],
    ]
    for (const [file, sel] of sites) {
      const code = readFileSync(path.join(SRC, 'components', 'layout', file), 'utf8')
      const at = code.indexOf(sel)
      expect(at, `${file} 里找不到 ${sel}`).toBeGreaterThan(-1)
      const block = code.slice(at, code.indexOf('}', at))
      expect(block, `${file} 的 ${sel} 没读 --brand-fill，品牌底又变成局部烤死的常量`)
        .toMatch(/var\(--brand-fill(-hover)?\)/)
    }
  })

  // 同一个红不能既当"正文级红字"又当"色块底"：默认皮肤下白字压 --danger(#f87171) 只有 2.77:1（实测角标），
  // 而 cyberpunk 的 #ff2d55 当字压卡面 4.41、压行悬停面 3.83。⇒ --danger 专职文字/描边，
  // --danger-fill 专职底，--on-danger 是那块底上的字（霓虹皮把字改深，而不是把身份色压暗）。
  // 三个语义色（danger / warning / success）此前都是一个令牌兼两份工：既当"正文级彩色字"，
  // 又当"色块底"。同一个值不可能两边都过：默认皮肤下白字压 #f87171 只有 2.77、压 #10b981 只有 2.54、
  // 压 #f59e0b 只有 2.15（实测 .acw-tag / .icon-nav-badge）；而 cyberpunk 的 #ff2d55 当字压卡面又只有 4.41。
  // ⇒ 每个语义拆三档：--x 专职文字与描边，--x-fill 专职色块底，--on-x 是那块底上的字。
  // 霓虹皮肤保留身份色的办法是把字改深（--on-x 取该皮肤自己的最暗档），不是把身份色压暗。
  const SEM = ['danger', 'warning', 'success']

  it('三个语义色的三档（文字 / 填充 / 底上字）在四套皮肤上各自成立', () => {
    rootAdopted()
    for (const sem of SEM) {
      for (const skin of skinsWithOwnBlock) {
        const fill = rgb(lookup(`--${sem}-fill`, skin))
        expect(fill, `${skin}：--${sem}-fill=${lookup(`--${sem}-fill`, skin)} 解析不出颜色`).toBeTruthy()
        const on = rgb(lookup(`--on-${sem}`, skin))
        const r = contrast(on, fill)
        expect(r, `${skin}：--on-${sem}=${lookup(`--on-${sem}`, skin)} 压在 --${sem}-fill=${lookup(`--${sem}-fill`, skin)} 上只有 ${r}:1`).toBeGreaterThanOrEqual(4.5)
        for (const bg of SOLID) {
          const rr = contrast(rgb(lookup(`--${sem}`, skin)), rgb(lookup(bg, skin)))
          expect(rr, `${skin}：--${sem}(文字)=${lookup(`--${sem}`, skin)} 压在 ${bg}=${lookup(bg, skin)} 上只有 ${rr}:1`).toBeGreaterThanOrEqual(4.5)
        }
        if (skin !== 'default') {
          for (const name of [`--${sem}`, `--${sem}-fill`, `--on-${sem}`]) {
            expect(skins[skin][name], `${skin} 没有覆写 ${name} ⇒ 换肤后该语义色又退回外壳的字面量`).toBeTruthy()
          }
        }
      }
    }
  })

  const FILL_SITES = [
    [path.join(SRC, 'components', 'layout', 'IconSidebar.vue'), '.icon-nav-badge {', 'danger'],
    [path.join(SRC, 'components', 'layout', 'IconSidebar.vue'), '.icon-health-dot.down {', 'danger'],
    [path.join(MODULE, 'components', 'ExpertCollabPanel.vue'), '.acw-tag.is-danger {', 'danger'],
    [path.join(MODULE, 'components', 'ExpertCollabPanel.vue'), '.acw-tag.is-warning {', 'warning'],
    [path.join(MODULE, 'components', 'ExpertCollabPanel.vue'), '.acw-tag.is-success {', 'success'],
  ]

  it('把语义色当底的站点真的读填充档，而不是各自烤常量', () => {
    for (const [file, sel, sem] of FILL_SITES) {
      const code = readFileSync(file, 'utf8')
      const at = code.indexOf(sel)
      expect(at, `${path.basename(file)} 里找不到 ${sel}（站点被搬走或改名，本门禁就空转）`).toBeGreaterThan(-1)
      const block = code.slice(at, code.indexOf('}', at))
      expect(block, `${path.basename(file)} 的 ${sel} 没读 --${sem}-fill`).toContain(`var(--${sem}-fill)`)
      if (sel !== '.icon-health-dot.down {') {
        expect(block, `${path.basename(file)} 的 ${sel} 换了底却没换字 ⇒ 字与底不成对`).toContain(`var(--on-${sem})`)
      }
    }
  })
})
