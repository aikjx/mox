// 被禁端点复活台账（FE-MOD-GOV §5.5）：模块契约判为「后端不存在 / 恒零桩 / 非网关契约」的
// 路径，不得在 src 的 .js/.vue 里被引用。清单外的引用即红；台账内的引用被修好后必须删条目，
// 否则「已清零却仍登记」同样算失败（双向比较）。
//
// 为什么需要这本账：2026-09-27 之前，联盟工作台主入口一直调编排器独占的 SSE 与能力端点，
// 恒 404 却因 catch 兜底成写死文案而从未暴露——没有门禁，被禁端点会被反复"复活"。
import { describe, it, expect } from 'vitest'
import { readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'

import { FORBIDDEN_ENDPOINTS } from './endpoints.js'

function findRepoRoot(from) {
  let dir = from
  for (let i = 0; i < 8; i++) {
    const probe = path.join(dir, 'docs', 'API-REGISTRY.md')
    try {
      readFileSync(probe)
      return dir
    } catch (e) {
      dir = path.dirname(dir)
    }
  }
  throw new Error(`未找到仓库根（自 ${from} 向上）`)
}

const ROOT = findRepoRoot(process.cwd())
const SRC_DIR = path.join(ROOT, 'frontend-ui', 'src')
// 声明被禁清单的文件本身必然出现这些路径，不参与扫描（与 rel 同为正斜杠形态）
const DECLARATION = 'modules/expert-alliance/contract/endpoints.js'

/** 被禁路径 → 扫描针：同时收去前缀形态（legacy http.get 走相对路径，不带 /api） */
export function forbiddenNeedles() {
  const needles = new Set()
  for (const entry of FORBIDDEN_ENDPOINTS) {
    needles.add(entry.path)
    const bare = entry.path.replace(/^\/api/, '')
    if (bare) needles.add(bare)
  }
  return [...needles]
}

/** 纯函数：给定文件文本找出被禁端点引用（含行号），供真语料与变异体共用 */
export function findForbiddenRefs(text) {
  const hits = []
  const lines = text.replace(/\r\n/g, '\n').split('\n')
  for (const needle of forbiddenNeedles()) {
    lines.forEach((line, i) => {
      if (line.includes(needle)) hits.push({ needle, line: i + 1 })
    })
  }
  return hits
}

function walkJsVue(dir, out = []) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) walkJsVue(full, out)
    else if (/\.((js)|(vue))$/.test(entry.name) && !/\.test\.js$/.test(entry.name)) out.push(full)
  }
  return out
}

/** 扫描集大小（与 scanSrc 同一遍历逻辑，供"零命中的分母"断言） */
export function scannedFileCount() {
  return walkJsVue(SRC_DIR).length
}

/** 真扫描：src 下全部非测试 .js/.vue（测试文件按设计要断言被禁清单，故排除） */
export function scanSrc() {
  const byFile = new Map()
  for (const file of walkJsVue(SRC_DIR)) {
    const rel = path.relative(SRC_DIR, file).split(path.sep).join('/')
    if (rel === DECLARATION) continue
    const hits = findForbiddenRefs(readFileSync(file, 'utf8'))
    if (hits.length) byFile.set(rel, [...new Set(hits.map((h) => h.needle))].sort())
  }
  return byFile
}

// 已知复活位点。2026-09-27 本轮把最后四处收口：api/ai.api.js 删掉网关侧 content 恒退化的
// 专家对话导出；stores/ai.store.js 改走模块六模式契约；composables/useSSE.js 的示例 URL 换成
// 真实存在的任务日志流；stores/alliance.store.js 的运行面（假设后端不存在的流式端点）停用。
// 台账现为空 ⇒ 本文件的判据是「src 内零允许引用」：任何文件（含注释）写出被禁路径即红。
const KNOWN_REVIVALS = {}

// 扫描集大小是"零命中"的分母：没有它，一次把 src 扫成空目录的改动也能打印出漂亮的零。
// 下限为 2026-09-27 实测 257 个非测试 .js/.vue 文件留出的余量（只许变多，不许变少）。
const SCAN_FLOOR = 250

describe('被禁端点不得复活（FORBIDDEN_ENDPOINTS 复活台账）', () => {
  it('被禁清单非空且每条都有理由与路径', () => {
    expect(FORBIDDEN_ENDPOINTS.length).toBeGreaterThan(0)
    for (const entry of FORBIDDEN_ENDPOINTS) {
      expect(entry.path.startsWith('/api/'), entry.path).toBe(true)
      expect(entry.reason.length, entry.path).toBeGreaterThan(4)
    }
  })

  it('针集覆盖每条被禁路径的两种形态（带/不带 /api 前缀）', () => {
    const needles = new Set(forbiddenNeedles())
    for (const entry of FORBIDDEN_ENDPOINTS) {
      expect(needles.has(entry.path), `缺带前缀针 ${entry.path}`).toBe(true)
      expect(needles.has(entry.path.replace(/^\/api/, '')), `缺去前缀针 ${entry.path}`).toBe(true)
    }
  })

  it('扫描集非空：零命中必须来自"扫到了东西且没有"，不是来自"什么都没扫"', () => {
    const n = scannedFileCount()
    expect(n, `被扫描文件数 ${n} 低于下限 ${SCAN_FLOOR}，扫描面被缩小`).toBeGreaterThanOrEqual(SCAN_FLOOR)
  })

  it('src 内的被禁端点引用与台账逐文件逐针相等（新增即红，清零须删条目）', () => {
    const actual = {}
    for (const [rel, needles] of scanSrc()) actual[rel] = needles
    expect(Object.keys(actual).sort()).toEqual(Object.keys(KNOWN_REVIVALS).sort())
    for (const rel of Object.keys(KNOWN_REVIVALS)) {
      expect(actual[rel], rel).toEqual(KNOWN_REVIVALS[rel])
    }
  })

  it('正对照：检测器对合成源真能逐行点名（证明上面那条不是空转）', () => {
    const planted = [
      "export async function bad() { return http.get('/alliance/stats') }",
      "await fetch('/api/ai/engine/alliance/full', { method: 'POST' })",
      "http.get('/ai/engine/alliance/capabilities')",
      "new EventSource('/api/alliance/stream')",
      "http.get('/api/expert-graph/overview')",
      "http.post('/api/ai/expert-chat', body)"
    ]
    // 每一行都必须被点名，且行号回指该行自身（去前缀形态是被写出的那条的子串，两种针都该响）
    planted.forEach((line, i) => {
      const hits = findForbiddenRefs(line)
      expect(hits.length, `第 ${i + 1} 行未被点名: ${line}`).toBeGreaterThan(0)
      expect(hits.every((h) => h.line === 1), `行号错位: ${line}`).toBe(true)
    })
    const detected = [...new Set(findForbiddenRefs(planted.join('\n')).map((h) => h.needle))]
    for (const line of planted) {
      const written = FORBIDDEN_ENDPOINTS.map((e) => e.path).find((p) => line.includes(p)) ||
        forbiddenNeedles().find((n) => line.includes(n))
      expect(detected, `未检出 ${written}`).toContain(written)
    }
    expect(findForbiddenRefs(planted.join('\n')).every((h) => h.line >= 1 && h.line <= planted.length)).toBe(true)
    // 反面对照：合法的网关路径不得被点名，否则台账里的"零命中"是遮蔽出来的
    expect(findForbiddenRefs("http.get('/api/experts/stats')\nhttp.get('/api/experts/capabilities')")).toEqual([])
  })

  it('/api/alliance/stats 恒零桩在 src 内已零引用（真语料侧见证，非只靠变异体）', () => {
    const hits = [...scanSrc().values()].flat().filter((n) => n === '/alliance/stats' || n === '/api/alliance/stats')
    expect(hits).toEqual([])
  })
})

describe('联盟工作台的取数与阶段文案挂在模块契约单源上', () => {
  const ws = readFileSync(path.join(SRC_DIR, 'composables', 'workspace', 'useAlliance.js'), 'utf8').replace(/\r\n/g, '\n')
  const panel = readFileSync(path.join(SRC_DIR, 'views', 'workspace', 'panels', 'CollaborationPanel.vue'), 'utf8').replace(/\r\n/g, '\n')

  it('useAlliance 经模块 api/contract 取数，不再从旧 @/api 拉联盟 SSE', () => {
    expect(ws.includes("from '@/modules/expert-alliance/api'")).toBe(true)
    expect(ws.includes("from '@/modules/expert-alliance/contract'")).toBe(true)
    expect(ws.includes("from '@/api'")).toBe(false)
    expect(ws.includes('runAllianceFullSSE')).toBe(false)
  })

  it('七阶段文案只从 contract/phases.js 取，面板内不残留手写副本', () => {
    expect(ws.includes("PHASE_IDS")).toBe(true)
    expect(ws.includes("label: '组队匹配'")).toBe(false)
    expect(panel.includes("from '@/modules/expert-alliance/contract'")).toBe(true)
    expect(panel.includes('组队匹配')).toBe(false)
    // 手写副本一旦回来，这里必然先出现 key/label 字面量表
    expect(panel.includes("{ key: 'intent', label:")).toBe(false)
  })

  it('工作台能力清单改读真实能力目录，不再读后端从未产出的键', () => {
    const view = readFileSync(path.join(SRC_DIR, 'views', 'workspace', 'ExpertWorkspaceView.vue'), 'utf8').replace(/\r\n/g, '\n')
    expect(view.includes('listExpertCapabilities')).toBe(true)
    expect(view.includes('intent_classes_7')).toBe(false)
    expect(view.includes('14 维度评估')).toBe(false)
  })
})
