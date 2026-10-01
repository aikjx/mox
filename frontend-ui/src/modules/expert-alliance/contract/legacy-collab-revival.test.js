// 旧协作函数复活台账（FE-MOD-GOV §5.5 同族门禁）。
//
// 为什么需要这本账：联盟有两条并行的协作调用面——模块契约（allianceApi.collaborate +
// contract/collab.js 的 wire 单源）与 src/api/experts.api.js 的六个裸函数。后者把请求体
// 交给调用方手写字段名，2026-09-27 实测工作台就是靠它发出 `question`（辩论要的是 topic）、
// `maxExperts`（serde 静默丢弃）、`mode`（后端从未有此字段），界面因此常年空白。
// 工作台的调用点本轮已收口为 0；剩余存量登记在下面，清零一个删一条，写 0 不算清账。
//
// 判据形态与 forbidden-revival 一致：纯函数扫描 + 逐文件台账双向相等 + 合成正/反对照。
import { describe, it, expect } from 'vitest'
import { readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'

import { ENDPOINTS } from './endpoints.js'
import { COLLAB_MODES } from './collab.js'

// 六个旧裸函数（与 api/experts.api.js 的导出名逐一对应）
const LEGACY_CALLERS = [
  'consultExpert',
  'multiExpertConsult',
  'expertDebate',
  'routeExperts',
  'intelligentConsult',
  'algorithmAnalysis'
]

// 只认「调用形态」：`\bNAME\s*\(`。定义行 `export const expertDebate = (payload) =>` 与
// 映射行 `expertDebate: normDebate` 都不含该形态，所以声明文件与模块契约都不必被排除。
const CALL_RE = new RegExp(`\\b(${LEGACY_CALLERS.join('|')})\\s*\\(`, 'g')

// 手写 wire 键字面量：视图绕过 contract/collab.js 自己拼请求体的形状。
// 契约层的合法形态是数组元素 'expert_ids' 与常量映射 expertIds: 'expert_ids'，均不带尾随冒号。
const WIRE_RE = /\bexpert_ids\s*:/g

function findRepoRoot(from) {
  let dir = from
  for (let i = 0; i < 8; i++) {
    try {
      readFileSync(path.join(dir, 'docs', 'API-REGISTRY.md'))
      return dir
    } catch (e) {
      dir = path.dirname(dir)
    }
  }
  throw new Error(`未找到仓库根（自 ${from} 向上）`)
}

const SRC_DIR = path.join(findRepoRoot(process.cwd()), 'frontend-ui', 'src')

/** 纯函数：给定文件文本点名旧协作调用与手写 wire 键（含行号），供真语料与变异体共用 */
export function findLegacyCollabRefs(text) {
  const lines = String(text || '').replace(/\r\n/g, '\n').split('\n')
  const hits = []
  lines.forEach((line, i) => {
    for (const m of line.matchAll(CALL_RE)) hits.push({ kind: 'caller', needle: m[1], line: i + 1 })
    for (const m of line.matchAll(WIRE_RE)) hits.push({ kind: 'wire', needle: 'expert_ids:', line: i + 1 })
  })
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

/** 扫描集大小（与 scanSrc 同一遍历逻辑，作为"零命中"的分母） */
export function scannedFileCount() {
  return walkJsVue(SRC_DIR).length
}

/** 真扫描：src 下全部非测试 .js/.vue，按文件汇总 kind→命中次数 */
export function scanSrc() {
  const byFile = new Map()
  for (const file of walkJsVue(SRC_DIR)) {
    const rel = path.relative(SRC_DIR, file).split(path.sep).join('/')
    const hits = findLegacyCollabRefs(readFileSync(file, 'utf8'))
    if (!hits.length) continue
    const counts = {}
    for (const h of hits) counts[h.kind] = (counts[h.kind] || 0) + 1
    byFile.set(rel, counts)
  }
  return byFile
}

// 2026-09-27 两处存量都已收口，台账为空 ⇒ 判据是「src 内零调用位点、零手写 expert_ids」：
//  - views/workspace/ExpertWorkspaceView.vue：三条协作流改走 allianceApi.collaborate + 投影行；
//  - views/expert/ExpertCenterView.vue：那六条调用连同 900 行 Tab 化改造前的残留脚本一起删除
//    （模板里从未有入口调它们，六条流全是不可达码；协作流的实现在模块里，见下一条判据）。
// 清零的位点必须删条目，写 0 不算清账（与 forbidden-revival 同规矩）。
const KNOWN_LEGACY_SITES = {}

// 扫描集下限沿用 2026-09-27 实测 257 个非测试 .js/.vue 留出的余量（只许变多）。
const SCAN_FLOOR = 250

describe('旧协作调用面不得复活（LEGACY_CALLERS 复活台账）', () => {
  it('六个旧函数名与 api/experts.api.js 的导出逐一对应（台账不能对着空气判）', () => {
    const decl = readFileSync(path.join(SRC_DIR, 'api', 'experts.api.js'), 'utf8')
    for (const name of LEGACY_CALLERS) {
      expect(decl.includes(`export const ${name} =`), `旧 API 层已无导出 ${name}`).toBe(true)
    }
  })

  it('扫描集非空：零命中必须来自"扫到了东西且没有"，不是来自"什么都没扫"', () => {
    const n = scannedFileCount()
    expect(n, `被扫描文件数 ${n} 低于下限 ${SCAN_FLOOR}，扫描面被缩小`).toBeGreaterThanOrEqual(SCAN_FLOOR)
  })

  it('存量位点与台账逐文件逐类相等（新增即红，清零须删条目）', () => {
    const actual = {}
    for (const [rel, counts] of scanSrc()) actual[rel] = counts
    expect(Object.keys(actual).sort()).toEqual(Object.keys(KNOWN_LEGACY_SITES).sort())
    for (const rel of Object.keys(KNOWN_LEGACY_SITES)) {
      expect(actual[rel], rel).toEqual(KNOWN_LEGACY_SITES[rel])
    }
  })

  it('工作区子树（views/workspace/**）零旧调用、零手写 wire：本轮收口结果单独钉住', () => {
    const ws = [...scanSrc().entries()].filter(([rel]) => rel.startsWith('views/workspace/'))
    expect(ws, `工作台又出现旧调用面: ${JSON.stringify(ws)}`).toEqual([])
  })

  it('收口不是靠删除功能：工作台确实在走模块契约的 collaborate', () => {
    const view = readFileSync(path.join(SRC_DIR, 'views', 'workspace', 'ExpertWorkspaceView.vue'), 'utf8')
    expect(view.includes("from '@/modules/expert-alliance/contract'")).toBe(true)
    expect(view.includes("from '@/modules/expert-alliance/model'")).toBe(true)
    expect(view.includes('allianceApi.collaborate(')).toBe(true)
    for (const panel of ['DebateDialog.vue', 'MultiConsultDialog.vue', 'SmartRouteDialog.vue']) {
      const text = readFileSync(path.join(SRC_DIR, 'views', 'workspace', 'panels', panel), 'utf8')
      expect(text.includes('@/modules/expert-alliance/'), panel).toBe(true)
    }
  })

  it('正对照：调用形态与手写 wire 都能被逐行点名（证明台账不是空转）', () => {
    const planted = [
      "const r = await expertDebate({ topic: 'x' })",
      "await multiExpertConsult({ question: 'q', expert_ids: ids })",
      "routingResult.value = await routeExperts({ question: q })",
      "await consultExpert(id, { priority: 'high' })",
      "await intelligentConsult({ question })",
      "await algorithmAnalysis({ code })",
      "return api.post('/experts/debate', { expert_ids: ids, rounds: 3 })"
    ]
    planted.forEach((line, i) => {
      const hits = findLegacyCollabRefs(line)
      expect(hits.length, `第 ${i + 1} 行未被点名: ${line}`).toBeGreaterThan(0)
      expect(hits.every((h) => h.line === 1), `行号错位: ${line}`).toBe(true)
    })
    expect(findLegacyCollabRefs(planted[1])).toEqual([
      { kind: 'caller', needle: 'multiExpertConsult', line: 1 },
      { kind: 'wire', needle: 'expert_ids:', line: 1 }
    ])
    // 第 7 行只有 wire，没有调用者：分类计数不能被混用
    expect(findLegacyCollabRefs(planted[6]).map((h) => h.kind)).toEqual(['wire'])
  })

  it('反对照：契约层与声明层的合法形态不得被点名（否则台账的 0 是遮蔽出来的）', () => {
    const legal = [
      "export const expertDebate = (payload) => http.post('/experts/debate', payload)",
      "  expertDebate: normDebate,",
      "    endpoint: 'expertDebate',",
      "    wires: ['topic', 'expert_ids', 'rounds'],",
      "  expertIds: 'expert_ids',",
      "  if (ids.length) body.expert_ids = ids",
      "const consultExpert = ref(null)",
      "  const res = await store.consultNow(consultExpert.value, { question })"
    ].join('\n')
    expect(findLegacyCollabRefs(legal), '合法形态被判坏 ⇒ 针是子串匹配而非调用形态').toEqual([])
    const contractSrc = readFileSync(
      path.join(SRC_DIR, 'modules', 'expert-alliance', 'contract', 'collab.js'), 'utf8')
    expect(findLegacyCollabRefs(contractSrc), '契约单源自身被点名 ⇒ 针过宽').toEqual([])
  })

  it('台账为空不等于判据空转：六个模式此刻都由模块契约实现（删的是不可达码，不是能力）', () => {
    // 空台账上的"零命中"必须配一个非空分母，否则就是把零证据打印成满把握判决。
    expect(Object.keys(KNOWN_LEGACY_SITES)).toEqual([])
    expect([...scanSrc().entries()]).toEqual([])
    const modes = COLLAB_MODES
    expect(modes).toHaveLength(6)
    for (const mode of modes) {
      expect(ENDPOINTS[mode.endpoint], `模式 ${mode.key} 的端点未在 endpoints.js 登记`).toBeDefined()
      expect(ENDPOINTS[mode.endpoint].path.startsWith('/api/'), mode.key).toBe(true)
    }
    // 模块 api 是唯一的调用出口：六条流各自都能被 collaborate 分派
    const apiSrc = readFileSync(
      path.join(SRC_DIR, 'modules', 'expert-alliance', 'api', 'alliance.api.js'), 'utf8')
    expect(apiSrc.includes('collaborate')).toBe(true)
  })

  it('专家中心页收口后仍是可用的外壳（删脚本没把页面删没）', () => {
    const view = readFileSync(path.join(SRC_DIR, 'views', 'expert', 'ExpertCenterView.vue'), 'utf8')
    expect(view.includes('ExpertOverviewPanel')).toBe(true)
    expect(view.includes('registerExpert')).toBe(true)
    expect(view.includes('<router-view')).toBe(true)
    // 六条协作流的入口从未在这个模板里存在，收口后也不该被顺手加回来
    expect(view.includes('question')).toBe(false)
  })
})
