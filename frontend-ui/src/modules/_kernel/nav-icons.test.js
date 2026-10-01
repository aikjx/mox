// 图标名登记口的守卫：把"字符串图标名能不能解析出来"变成可判定量。
//
// 事实面（本文件现场扫描，不抄副本）：
//   - vite.config.js:63 用 unplugin-vue-components + epSubpathResolver，它只改写**模板标签**；
//     模板里传**字符串**的两条通道编译期没人管 ⇒ 必须靠 main.js 的运行期 app.component 注册。
//   - 通道 A：数据表 `icon: 'PascalCase'`；通道 B：内联三元 `:is="x ? 'Name' : 'Name'"`。
//   - 注册动作在 src/modules/_kernel/nav-icons.js 的 registerNavIcons()。
//
// 牙齿（2026-09-27 实测 6/6 变异体全部打红并逐字节还原，驱动在仓库外 %TEMP%\mox-icon-mut）：
//   M1 注释掉 main.js 的 registerNavIcons(app) → 格 2 红（语句级锚点，见该格注释）
//   M2 封闭集删掉一名（Aim）              → 格 1 红（使用位点报未登记），连累格 5/7/9
//   M3 封闭集多登记一个未使用名            → 格 3 红（登记未使用），连累格 7
//   M4 把通道 A 的正则改坏（icon→icons）   → 格 3 红（分母下界塌缩）+ 格 4 红
//   M5 navIcon 丢掉 hasOwnProperty 守卫    → 格 5 红（constructor 泄漏成组件）
//   M6 加一行 `import * as ALL_ICONS`      → 格 7 红（首屏炸弹形态）
// 另：格 9 用真 Vue 挂载做机制见证（注册后字符串名渲染出 <svg>，未登记名渲染成未知元素）。
import { readFileSync, readdirSync, statSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { createApp, h, resolveDynamicComponent } from 'vue'

import { NAV_ICONS, navIcon, navIconNames, registerNavIcons } from './nav-icons'

const HERE = path.dirname(fileURLToPath(import.meta.url))
const SRC = path.resolve(HERE, '../..')
const MAIN_JS = path.join(SRC, 'main.js')
const VITE_CONFIG = path.resolve(SRC, '..', 'vite.config.js')
const SELF = fileURLToPath(import.meta.url)

const read = (p) => readFileSync(p, 'utf-8').replace(/\r\n/g, '\n')

function sourceFiles(dir, acc = []) {
  for (const entry of readdirSync(dir)) {
    const p = path.join(dir, entry)
    if (statSync(p).isDirectory()) {
      sourceFiles(p, acc)
    } else if (/\.(vue|js)$/.test(entry) && !/\.test\.js$/.test(entry) && p !== SELF) {
      acc.push(p)
    }
  }
  return acc
}

// 通道 A：`icon: 'Aim'` 形态的数据字面量（允许单/双引号与冒号后空格）
const DATA_LITERAL = /icon:\s*'([A-Z][A-Za-z0-9]*)'/g
// :is 绑定表达式（含 v-bind 简写），值里再挑引号包住的 PascalCase 名 —— 通道 B
const IS_BINDING = /(?::is|v-bind:is)="([^"]*)"/g
const QUOTED_NAME = /'([A-Z][A-Za-z0-9]*)'/g

/** 扫描一批源码，返回 { names: Map<名, 使用处[]>, sites: 命中处数 } */
function scanIconStrings(files, { dataLiteral = DATA_LITERAL, isBinding = IS_BINDING } = {}) {
  const names = new Map()
  let sites = 0
  const hit = (text, file) => {
    const lines = text.split('\n')
    for (const re of [dataLiteral, isBinding]) {
      re.lastIndex = 0
      let m
      while ((m = re.exec(text))) {
        if (re === dataLiteral) {
          sites += 1
          const list = names.get(m[1]) || []
          list.push(`${file}:A`)
          names.set(m[1], list)
        } else {
          QUOTED_NAME.lastIndex = 0
          let q
          while ((q = QUOTED_NAME.exec(m[1]))) {
            sites += 1
            const list = names.get(q[1]) || []
            list.push(`${file}:B`)
            names.set(q[1], list)
          }
        }
      }
    }
    return lines.length
  }
  for (const f of files) hit(read(f), path.relative(SRC, f).replace(/\\/g, '/'))
  return { names, sites }
}

// 通道 C：模板里裸写的图标标签 `<Refresh />` / `<MoreFilled :size="14" />`。
// resolver 只认 ElIconXxx / ElFoo 两种前缀 ⇒ 裸名不会被自动导入；文件里若也没有本地
// import，就是运行期 "Failed to resolve component"（浏览器实测确实有此告警）。
const ICONS_PKG = path.resolve(SRC, '..', 'node_modules/@element-plus/icons-vue/dist/index.js')
const PKG_ICON_NAMES = (() => {
  const blocks = [...read(ICONS_PKG).matchAll(/export\s*\{([^}]*)\}/g)]
  expect(blocks.length, '图标包入口里没有 export { … } 块，通道 C 的判定集会空转').toBeGreaterThan(0)
  const out = new Set()
  for (const b of blocks) {
    for (const spec of b[1].split(',')) {
      const name = spec.trim().split(/\s+as\s+/).pop()
      if (/^[A-Z][A-Za-z0-9]*$/.test(name)) out.add(name)
    }
  }
  expect(out.size, '图标包导出的名字数为 0 ⇒ 本扫描是假阴').toBeGreaterThan(200)
  return out
})()

const BARE_TAG = /<([A-Z][A-Za-z0-9]*)(?=[\s/>])/g
const LOCAL_IMPORT = /import\s*(?:\{([^}]*)\}|(\w+))\s*from\s*['"][^'"]+['"]/g

/** 通道 C 扫描：返回 { names: Map<名, 使用处[]>, sites } */
function scanBareIconTags(files) {
  const names = new Map()
  let sites = 0
  for (const f of files) {
    if (!f.endsWith('.vue')) continue
    const text = read(f)
    const local = new Set()
    let im
    LOCAL_IMPORT.lastIndex = 0
    while ((im = LOCAL_IMPORT.exec(text))) {
      if (im[2]) local.add(im[2])
      for (const spec of (im[1] || '').split(',')) {
        const s = spec.trim()
        if (!s) continue
        const parts = s.split(/\s+as\s+/)
        local.add((parts[1] || parts[0]).trim())
      }
    }
    let m
    BARE_TAG.lastIndex = 0
    while ((m = BARE_TAG.exec(text))) {
      const name = m[1]
      if (local.has(name) || !PKG_ICON_NAMES.has(name)) continue
      sites += 1
      const list = names.get(name) || []
      list.push(`${path.relative(SRC, f).replace(/\\/g, '/')}:C`)
      names.set(name, list)
    }
  }
  return { names, sites }
}

const FILES = sourceFiles(SRC)
const SCAN = scanIconStrings(FILES)
const C_SCAN = scanBareIconTags(FILES)
const USED = [...new Set([...SCAN.names.keys(), ...C_SCAN.names.keys()])].sort()
const REGISTERED = navIconNames().sort()

// 使用位点的分布：A/B 两通道各自的数量，用于钉"扫描器没有瞎"
const A_COUNT = FILES.reduce((n, f) => n + (read(f).match(DATA_LITERAL) || []).length, 0)
const B_COUNT = FILES.reduce((n, f) => {
  const t = read(f)
  let total = 0
  let m
  IS_BINDING.lastIndex = 0
  while ((m = IS_BINDING.exec(t))) {
    QUOTED_NAME.lastIndex = 0
    while (QUOTED_NAME.exec(m[1])) total += 1
  }
  return n + total
}, 0)

describe('图标名登记口：字符串必须能解析成组件', () => {
  it('格 1 · 三条通道的每个图标名（字符串或裸标签）都在封闭集里（漏登记即红）', () => {
    const unregistered = USED.filter((n) => !Object.prototype.hasOwnProperty.call(NAV_ICONS, n))
    expect(unregistered, `未登记的图标名：${unregistered.join(', ')}`).toEqual([])
  })

  it('格 2 · main.js 真的挂了运行期注册（撤掉调用即红）', () => {
    const main = read(MAIN_JS)
    expect(main).toMatch(/import\s*\{[^}]*registerNavIcons[^}]*\}\s*from\s*'@\/modules'/)
    // 语句级锚点：注释掉的 `// registerNavIcons(app)` 不算挂载（变异体 M1 就是这个形态）
    expect(/^\s*registerNavIcons\(\s*app\s*\)\s*$/m.test(main), '入口没有实际调用 registerNavIcons').toBe(true)
    // 注册只在入口发生一次，且不在别处散点 app.component（封闭集只此一口）
    expect((main.match(/^\s*app\.component\(/gm) || []).length).toBe(0)
    const scattered = FILES.filter(
      (f) => f !== MAIN_JS && f !== path.join(HERE, 'nav-icons.js') && /^\s*app\.component\(/m.test(read(f))
    )
    expect(scattered.map((f) => path.relative(SRC, f)), '别处散点全局注册').toEqual([])
  })

  it('格 3 · 封闭集不多不少：登记未使用即红，且数量与实测口径一致', () => {
    const usedAny = (n) => SCAN.names.has(n) || C_SCAN.names.has(n)
    const dead = REGISTERED.filter((n) => !usedAny(n))
    expect(dead, `登记了但没有任何字符串/裸标签位点使用：${dead.join(', ')}`).toEqual([])
    // 分母下界：扫描集塌缩（正则被改坏 / 目录走错）时这里先红，而不是"全绿通过"
    expect(A_COUNT, '通道 A（icon: 字面量）命中数低于实测下界').toBeGreaterThanOrEqual(127)
    expect(B_COUNT, '通道 B（:is 内联字符串）命中数低于实测下界').toBeGreaterThanOrEqual(28)
    expect(C_SCAN.sites, '通道 C（裸图标标签）命中数低于实测下界').toBeGreaterThanOrEqual(38)
    expect(C_SCAN.names.size, '通道 C 的不同名低于实测下界').toBeGreaterThanOrEqual(23)
    expect(NAV_ICONS).toBeTypeOf('object')
    expect(Object.isFrozen(NAV_ICONS)).toBe(true)
  })

  it('格 4 · 正对照：植入一个未登记名，扫描器必须当场抓到', () => {
    // 已注册的那一枚从表里取（写死名字会让本格随封闭集增删而假红/假绿）
    const KNOWN = REGISTERED[0]
    const poisoned = [
      "const nav = [{ icon: 'NoSuchIconAnywhere' }]",
      `<div :is="open ? 'AlsoNoSuchIcon' : '${KNOWN}'" />`
    ].join('\n')
    const found = new Set()
    for (const re of [DATA_LITERAL, IS_BINDING]) {
      re.lastIndex = 0
      let m
      while ((m = re.exec(poisoned))) {
        if (re === DATA_LITERAL) found.add(m[1])
        else {
          QUOTED_NAME.lastIndex = 0
          let q
          while ((q = QUOTED_NAME.exec(m[1]))) found.add(q[1])
        }
      }
    }
    expect([...found].sort()).toEqual([KNOWN, 'AlsoNoSuchIcon', 'NoSuchIconAnywhere'].sort())
    const leaked = [...found].filter((n) => !Object.prototype.hasOwnProperty.call(NAV_ICONS, n))
    expect(leaked.sort()).toEqual(['AlsoNoSuchIcon', 'NoSuchIconAnywhere'])
    // 通道 C 的正对照：包里有、表里没有的裸标签必须被同一条判定路径挑出来。
    // （挑一个封闭集之外的真实图标名，避免"任何大写标签都算"的假判定）
    const bareCandidate = [...PKG_ICON_NAMES].sort().find((n) => !Object.prototype.hasOwnProperty.call(NAV_ICONS, n))
    expect(bareCandidate, '图标包里已没有一个"未登记"的名，本格会失去牙齿').toBeTruthy()
    const fixture = `<el-icon><${bareCandidate} /></el-icon>`
    BARE_TAG.lastIndex = 0
    const caught = [...fixture.matchAll(BARE_TAG)].map((x) => x[1]).filter((n) => PKG_ICON_NAMES.has(n))
    expect(caught).toEqual([bareCandidate])
    expect(PKG_ICON_NAMES.has('Component'), '包名集混进了非图标名 ⇒ 过滤失效').toBe(false)
  })

  it('格 5 · 未登记/非 PascalCase 一律回 null，由调用方回落到文本', () => {
    expect(navIcon('Aim')).toBeTruthy()
    expect(navIcon('NoSuchIconAnywhere')).toBeNull()
    expect(navIcon('arrow-up')).toBeNull()
    expect(navIcon('🔍')).toBeNull()
    expect(navIcon(undefined)).toBeNull()
    expect(navIcon(null)).toBeNull()
    expect(navIcon(42)).toBeNull()
    expect(navIcon('constructor')).toBeNull()
  })

  it('格 6 · registerNavIcons 注册数 = 表长，且逐项同名同物', () => {
    const calls = []
    const app = { component: (name, comp) => calls.push([name, comp]) }
    const n = registerNavIcons(app)
    expect(n).toBe(REGISTERED.length)
    expect(calls.length).toBe(REGISTERED.length)
    for (const [name, comp] of calls) {
      expect(NAV_ICONS[name], `注册项 ${name} 与表内不同一对象`).toBe(comp)
      expect(comp, `图标 ${name} 解析为空`).toBeTruthy()
    }
  })

  it('格 7 · 不许整包导入图标库（293 个导出的首屏炸弹），逐个点名是唯一形态', () => {
    const own = read(path.join(HERE, 'nav-icons.js'))
    expect(own).not.toMatch(/import\s*\*\s*as/)
    expect(own).toMatch(/from\s*'@element-plus\/icons-vue'/)
    const importBlock = own.slice(own.indexOf('import {'), own.indexOf("from '@element-plus/icons-vue'"))
    const imported = (importBlock.match(/^\s*[A-Z][A-Za-z0-9]*,?$/gm) || []).map((s) => s.trim().replace(/,$/, ''))
    expect(imported.length).toBe(REGISTERED.length)
    expect(imported.sort()).toEqual(REGISTERED)
  })

  it('格 8 · 前提事实仍在：resolver 只改写标签，字符串位点必须靠运行期注册', () => {
    const vite = read(VITE_CONFIG)
    expect(vite).toContain('unplugin-vue-components')
    // resolver 的图标分支是"标签名"改写（ElIconXxx → Xxx），与 :is 传字符串无关
    expect(vite).toMatch(/\^ElIcon\.\+/)
  })

  it('格 9 · 真 Vue 见证：注册后字符串组件名真的挂载得出 <svg>，未登记的仍落空', () => {
    const app = createApp({ render: () => h('span', [h(resolveDynamicComponent('Aim'))]) })
    registerNavIcons(app)
    expect(app.component('Aim')).toBeTruthy()
    expect(app.component('NoSuchIconAnywhere')).toBeFalsy()
    const el = document.createElement('div')
    app.mount(el)
    try {
      expect(el.innerHTML, '字符串解析成功时应渲染出图标 SVG').toContain('<svg')
    } finally {
      app.unmount()
    }
    // 反面对照：未登记名走同一条渲染路径 ⇒ 渲染成未知元素而不是 svg
    const app2 = createApp({ render: () => h('span', [h(resolveDynamicComponent('NoSuchIconAnywhere'))]) })
    registerNavIcons(app2)
    const el2 = document.createElement('div')
    app2.mount(el2)
    try {
      expect(el2.innerHTML).not.toContain('<svg')
    } finally {
      app2.unmount()
    }
  })
})
