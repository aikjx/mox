/**
 * 专家类型视觉属性单源（expert.constants）的守卫。
 *
 * 这一份存在的理由：expertColor / expertEmoji 曾在 7 个视图里各存一份逐字副本，
 * expertGradient 在 2 个视图里各一份 —— 副本之间一旦有人单独改色，同一个专家类型
 * 就会在不同页面显示成不同颜色（实测 ExpertCenterView 与 workspace 面板就分叉了：
 * requirement 一处 #16a34a 一处 #f43f5e，architecture 一处 #0891b2 一处 #6366f1）。
 *
 * 2026-09-25 补的第二道：只盯 `function expertColor(` 这种签名是漏的 —— 分叉的三处
 * 都是 `const colors = {…}` / `const typeColors = {…}` 这种**本地表**，且其中一处住在
 * src/components（旧守卫只扫 src/views），于是 643 个用例全绿而页面互相不同色。
 * 所以这里改成结构化判据：任何一张以 ≥5 个专家类型为键的本地对象表都算副本，
 * 扫描面覆盖 views + components。
 */
import { describe, expect, it } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import * as EP_ICONS from '@element-plus/icons-vue'
import {
  EXPERT_TYPES,
  EXPERT_TYPE_SHORT_LABELS,
  EXPERT_ICONS,
  EXPERT_COLORS,
  EXPERT_EMOJIS,
  EXPERT_GRADIENTS,
  EXPERT_COLOR_FALLBACK,
  expertColor,
  expertEmoji,
  expertGradient,
} from './expert.constants'

const SRC = path.resolve(__dirname, '..')
const ROOTS = ['views', 'components'].map((d) => path.join(SRC, d))
const DUP_SIGS = ['function expertColor(', 'function expertEmoji(', 'function expertGradient(']
const TYPE_KEYS = new Set(Object.keys(EXPERT_TYPES))

const TABLES = {
  EXPERT_TYPES,
  EXPERT_TYPE_SHORT_LABELS,
  EXPERT_ICONS,
  EXPERT_COLORS,
  EXPERT_EMOJIS,
  EXPERT_GRADIENTS,
}

function walk(dir, out = []) {
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, ent.name)
    if (ent.isDirectory()) walk(p, out)
    else if (/\.(vue|js)$/.test(ent.name)) out.push(p)
  }
  return out
}

function scannedFiles() {
  return ROOTS.flatMap((r) => walk(r)).filter((f) => !/\.(test|spec)\.js$/.test(f))
}

function textOf(f) {
  return typeof f === 'string' ? fs.readFileSync(f, 'utf8') : f.text
}
function nameOf(f) {
  return path.basename(typeof f === 'string' ? f : f.name)
}

/** 命中签名即视为"视图里又抄了一份本地实现"。 */
function duplicateSites(files) {
  const hits = []
  for (const f of files) {
    const text = textOf(f)
    for (const sig of DUP_SIGS) if (text.includes(sig)) hits.push(`${nameOf(f)}:${sig}`)
  }
  return hits
}

/**
 * 结构化判据：按类型编号的本地字面量表，不管它叫 colors 还是 typeColors 还是别的。
 * 两种实测存在的形状都要盖住：
 *   ① 逐行表（含 ExpertPlazaView 那种 `algorithm: { label, color }` 嵌套表 —— 它躲得过
 *      "只匹配扁平 {…}" 的写法，所以这里按"连续多少行以类型名为键"来数）；
 *   ② 挤在一行里的表（逐行法只数到 1 行，所以再补一条"扁平字面量里类型键占比"）。
 * 占比这条同时负责把 GraphView 那种**同名不同域**的表挡在外面：它的 typeLabels 有 28 个键，
 * 其中 8 个恰好与专家类型重名（data / ai / graph / custom / operator / workflow / requirement），
 * 占比 8/28；真的副本是 15/15、16/16。
 */
const LINE_KEY = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:/
const MIN_RUN = 5
const MIN_FLAT_KEYS = 5
const MIN_FLAT_SHARE = 0.6

function runHits(text, label) {
  const hits = []
  let run = 0
  let start = 0
  text.split(/\r?\n/).forEach((ln, ix) => {
    const m = ln.match(LINE_KEY)
    if (m && TYPE_KEYS.has(m[1])) {
      if (run === 0) start = ix + 1
      run += 1
    } else {
      if (run >= MIN_RUN) hits.push(`${label}: 连续 ${run} 行以专家类型为键（L${start}）`)
      run = 0
    }
  })
  if (run >= MIN_RUN) hits.push(`${label}: 连续 ${run} 行以专家类型为键（尾部）`)
  return hits
}

function flatHits(text, label) {
  const hits = []
  for (const block of text.matchAll(/\{[^{}]*\}/g)) {
    const pairs = [...block[0].matchAll(/([A-Za-z_][A-Za-z0-9_]*)\s*:\s*'[^'\\]*'/g)]
    const seen = new Set(pairs.map((p) => p[1]).filter((k) => TYPE_KEYS.has(k)))
    if (seen.size >= MIN_FLAT_KEYS && seen.size / pairs.length >= MIN_FLAT_SHARE) {
      hits.push(`${label}: 扁平表含 ${seen.size}/${pairs.length} 个专家类型键`)
    }
  }
  return hits
}

function localTypeTables(files) {
  const hits = []
  for (const f of files) {
    const text = textOf(f)
    const label = nameOf(f)
    hits.push(...runHits(text, label))
    hits.push(...flatHits(text, label))
  }
  return [...new Set(hits)]
}

describe('专家类型视觉属性单源', () => {
  it('枚举里每个类型都有色 / 图标 / 渐变，缺一个就换页失色', () => {
    for (const type of Object.keys(EXPERT_TYPES)) {
      expect(EXPERT_COLORS, `EXPERT_COLORS 缺 ${type}`).toHaveProperty(type)
      expect(EXPERT_EMOJIS, `EXPERT_EMOJIS 缺 ${type}`).toHaveProperty(type)
      expect(EXPERT_GRADIENTS, `EXPERT_GRADIENTS 缺 ${type}`).toHaveProperty(type)
    }
    expect(Object.keys(EXPERT_TYPES).length).toBeGreaterThan(10)
  })

  it('六份表键集合完全一致（custom 这类"只在一处有名字"就是分叉的起点）', () => {
    const c = Object.keys(EXPERT_COLORS).sort()
    for (const [name, table] of Object.entries(TABLES)) {
      expect(Object.keys(table).sort(), `${name} 键集与 EXPERT_COLORS 不符`).toEqual(c)
    }
    // 这条正是 custom 的来源：EXPERT_TYPES 曾少一个 custom，而色/图标/渐变三表都有它
    expect(EXPERT_TYPES).toHaveProperty('custom')
  })

  it('色值必须是 6 位 hex 字面量（写歪了不会被静默当字符串渲染）', () => {
    for (const [k, v] of Object.entries(EXPERT_COLORS)) {
      expect(v, `${k}=${v}`).toMatch(/^#[0-9a-f]{6}$/)
    }
  })

  it('短标签必须真的更短：图例宽度靠的是它，退化成全名等于把这条判据写了个寂寞', () => {
    for (const [type, short] of Object.entries(EXPERT_TYPE_SHORT_LABELS)) {
      const full = EXPERT_TYPES[type]
      expect(short, `${type} 短标签与全名相同`).not.toBe(full)
      expect([...short].length, `${type} 短标签(${short}) 不短于全名(${full})`).toBeLessThan([...full].length)
    }
  })

  it('图标名必须是 Element Plus 真实导出，写错了只是静默空白', () => {
    for (const [type, icon] of Object.entries(EXPERT_ICONS)) {
      expect(EP_ICONS, `EXPERT_ICONS.${type} = ${icon} 不是 EP 图标`).toHaveProperty(icon)
    }
  })

  it('取到的是该类型的登记色，不是兜底色', () => {
    // data 与兜底色不同值：一旦实现退化成"永远返回兜底"，第二条断言会先红
    expect(EXPERT_COLORS.data).not.toBe(EXPERT_COLOR_FALLBACK)
    expect(expertColor('data')).toBe(EXPERT_COLORS.data)
  })

  it('未知类型落兜底，而不是 undefined', () => {
    expect(expertColor('no-such-type')).toBe(EXPERT_COLOR_FALLBACK)
    expect(expertEmoji('no-such-type')).toBeTruthy()
    expect(expertGradient('no-such-type')).toContain('linear-gradient')
  })
})

describe('视图/组件不许再抄一份本地实现', () => {
  const plantedColorTable = {
    name: 'PlantedColor.vue',
    text: "const colors = { algorithm: '#0891b2', architecture: '#0891b2', data: '#0891b2', ai: '#0891b2', workflow: '#0891b2' }",
  }
  const plantedLabelTable = {
    name: 'PlantedLabel.vue',
    text: "const typeLabels = { algorithm: '算法', architecture: '架构', data: '数据', ai: 'AI', workflow: '工作流', graph: '图谱' }",
  }
  const plantedSignature = { name: 'PlantedSig.vue', text: 'function expertColor(type) { return "#6366f1" }' }
  // 只有 4 个类型键的表不算重述（例如文件扩展名图标表）；这条是判据的下界反例
  const belowThreshold = { name: 'Fine.vue', text: "const m = { algorithm: 'a', data: 'b', ai: 'c', workflow: 'd', pdf: 'e' }" }

  it('正例必须被抓到（否则下面那条零命中等于空转）', () => {
    expect(localTypeTables([plantedColorTable])).toHaveLength(1)
    expect(localTypeTables([plantedLabelTable])).toHaveLength(1)
    expect(duplicateSites([plantedSignature])).toHaveLength(1)
    expect(localTypeTables([belowThreshold])).toEqual([])
  })

  it('两种形状各自可破：嵌套逐行表 与 挤成一行的表', () => {
    // ① ExpertPlazaView 真实形状：一行一个类型、值是嵌套对象 —— 扁平扫描看不见
    const nested = { name: 'Nested.vue', text: "const expertTypes = {\n  algorithm: { label: '算法专家', color: '#6366f1' },\n  architecture: { label: '架构专家', color: '#0891b2' },\n  data: { label: '数据专家', color: '#10b981' },\n  ai: { label: 'AI专家', color: '#ec4899' },\n  workflow: { label: '工作流专家', color: '#f59e0b' }\n}" }
    expect(runHits(nested.text, nested.name)).toHaveLength(1)
    expect(flatHits(nested.text, nested.name)).toEqual([])
    // ② 全挤在一行：逐行法只数到 1 行，靠占比这条兜住
    const oneLine = { name: 'OneLine.vue', text: "const c = { algorithm: '#1', architecture: '#2', data: '#3', ai: '#4', workflow: '#5' }" }
    expect(flatHits(oneLine.text, oneLine.name)).toHaveLength(1)
    expect(runHits(oneLine.text, oneLine.name)).toEqual([])
    expect(localTypeTables([nested, oneLine])).toHaveLength(2)
  })

  it('同名不同域的表不算副本：GraphView 的 28 键算子表是实测反例', () => {
    // 8 个键与专家类型重名，但既非连续 5 行、占比也只有 8/28
    const collide = 'const typeLabels = {\n  core: \'核心算子\',\n  data: \'数据\',\n  aux: \'辅助\',\n  ai: \'智能\',\n  misc1: \'a\',\n  graph: \'图谱\',\n  misc2: \'b\',\n  operator: \'算子\',\n  misc3: \'c\',\n  workflow: \'流程\',\n  misc4: \'d\',\n  requirement: \'需求\',\n  misc5: \'e\',\n  custom: \'自定义\',\n  misc6: \'f\',\n  sec: \'g\'\n}'
    expect(runHits(collide, 'GraphView.vue')).toEqual([])
    expect(flatHits(collide, 'GraphView.vue')).toEqual([])
    expect(fs.existsSync(path.join(SRC, 'views/graph/GraphView.vue'))).toBe(true)
  })

  it('src/views + src/components 扫描集合非空且零命中', () => {
    const files = scannedFiles()
    expect(files.length).toBeGreaterThan(80)
    expect(files.filter((f) => f.includes(`${path.sep}components${path.sep}`)).length).toBeGreaterThan(10)
    expect(duplicateSites(files)).toEqual([])
    expect(localTypeTables(files)).toEqual([])
  })
})
