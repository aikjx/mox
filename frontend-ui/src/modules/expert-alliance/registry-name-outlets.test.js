/**
 * 注册表姓名的丢码出口收口（每一个"这个人叫什么"的出口都要过同一套三口径）。
 *
 * 归因（只读探针，2026-09-27）：`data/experts.db` 的 experts 表里两行 name 存的就是字面问号
 * （id exp-af875a6f60d943e6964fcef4db0ab73f → '?????????'；exp-cbf168cdeded4127bb812dca4d2b2936 →
 * '???????'），graph_nodes 镜像同一批 id ⇒ 丢失发生在写入/入库那一次，读路径干净，取不回来。
 * 所以本文件钉的不是"修好名字"，而是：丢了码的必须在每个出口都显形成 id 短码，
 * 而"注册表里根本没有这一行"必须仍然是调用方自己说的话。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import path from 'node:path'

const { api } = vi.hoisted(() => ({
  api: {
    listExperts: vi.fn(),
    listMyBookings: vi.fn(),
    expertsStats: vi.fn(),
    listExpertCapabilities: vi.fn(),
    deleteExpert: vi.fn(),
    dispatcherStatus: vi.fn()
  }
}))
vi.mock('./api/alliance.api.js', () => ({ allianceApi: api }))

const {
  expertDisplayName, expertNameOr, deleteConsequences, dispatchResetLines, COLLAB_MODES, collabMode
} = await import('./contract/index.js')
const { collabContributionItems, collabCandidateItems } = await import('./model/collabLists.js')
const { collabChatSpeaker } = await import('./model/collabChat.js')
const { buildBoard, RANK_BOARD } = await import('./model/rank.js')
const { useAllianceExpertsStore } = await import('./store/alliance-experts.store.js')
const { useAllianceConsoleStore } = await import('./store/alliance-console.store.js')

const LOST_ID = 'exp-af875a6f60d943e6964fcef4db0ab73f'
const LOST = { id: LOST_ID, name: '?????????' }
const LOST_SHORT = '未命名专家 af875a6f'
const NAMED = { id: 'exp-00000001', name: '甲' }
const Q = /[?]/

/** answer 型模式（resultKind==='answer' 才会走发言者姓名那条分支） */
const modeKeyOf = (m) => m.key ?? m.value ?? m.id
const ANSWER_MODE = COLLAB_MODES.map(modeKeyOf).find((k) => collabMode(k)?.resultKind === 'answer')

/** vitest 下 import.meta.url 不是 file: 协议，从 cwd 向上定位模块目录（同 contract.test.js 的做法） */
function findModuleDir(dir) {
  let d = dir
  for (let i = 0; i < 8; i++) {
    const cand = path.join(d, 'src', 'modules', 'expert-alliance')
    if (existsSync(path.join(cand, 'contract', 'graph.js'))) return cand
    d = path.dirname(d)
  }
  throw new Error('找不到模块目录: ' + dir)
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
  api.listMyBookings.mockResolvedValue({ items: [], total: 0, counts: {} })
  api.expertsStats.mockResolvedValue({ total: 0, online: 0 })
  api.listExpertCapabilities.mockResolvedValue({ items: [], capabilities: [] })
})

describe('注册表姓名的三口径单源', () => {
  it('好名字原样，丢了码的显形成 id 短码', () => {
    expect(expertDisplayName(NAMED)).toBe('甲')
    expect(expertDisplayName(LOST)).toBe(LOST_SHORT)
  })

  it('压根没给名字时把话交回调用方，不许冒充"丢了码"', () => {
    expect(expertNameOr({ id: LOST_ID, name: '' }, '（注册表里没有名字）')).toBe('（注册表里没有名字）')
    expect(expertNameOr(null, '（注册表里没有名字）')).toBe('（注册表里没有名字）')
    expect(expertNameOr(LOST, '（注册表里没有名字）')).toBe(LOST_SHORT)
  })

  it('没有 id 可指认时只说未命名专家，不编造人名', () => {
    expect(expertDisplayName({ name: '???' })).toBe('未命名专家')
    expect(expertDisplayName({})).toBe('未命名专家')
  })
})

describe('注册表姓名的文字出口逐个显形', () => {
  it('删除后果清单的首条点名丢码专家', () => {
    const line = deleteConsequences(LOST)[0]
    expect(line).toContain(LOST_SHORT)
    expect(Q.test(line)).toBe(false)
  })

  it('调度重置确认：丢了码显形，没名字照裸 id 走', () => {
    expect(dispatchResetLines(LOST)[0]).toContain(LOST_SHORT)
    expect(dispatchResetLines({ id: 'exp-999999999' })[0]).toContain('exp-999999999')
  })

  it('协作贡献行的姓名过同一口径', () => {
    const rows = collabContributionItems({ contributions: [{ id: LOST_ID, name: LOST.name, answer: { solution: 'x' } }] })
    expect(rows[0].name).toBe(LOST_SHORT)
  })

  it('路由候选行的姓名过同一口径', () => {
    const rows = collabCandidateItems({ candidates: [{ id: LOST_ID, name: LOST.name, title: '工程师' }] })
    expect(rows[0].name).toBe(LOST_SHORT)
  })

  it('聊天发言者：丢了码显形，一个名字都没有才回落模式中文名', () => {
    expect(ANSWER_MODE, '夹具失效：找不到 answer 型模式，这条针等于空转').toBeTruthy()
    // 正对照：好名字必须原样出来，否则说明根本没走进姓名分支
    expect(collabChatSpeaker({ mode: ANSWER_MODE, expertName: '甲' })).toBe('甲')
    expect(collabChatSpeaker({ mode: ANSWER_MODE, expertName: LOST.name, expert: { id: LOST_ID } })).toBe(LOST_SHORT)
    expect(collabChatSpeaker({ mode: ANSWER_MODE })).toBe('匹配专家')
  })

  it('榜单行名过同一口径，同分排序也不看问号串', () => {
    const rows = buildBoard(RANK_BOARD.CONSULTATIONS, [
      { id: NAMED.id, name: NAMED.name, metrics: { totalConsultations: 5 } },
      { id: LOST_ID, name: LOST.name, metrics: { totalConsultations: 3 } }
    ]).rows
    expect(rows.map((r) => r.name)).toEqual(['甲', LOST_SHORT])
  })
})

describe('广场与调度台 store 的姓名出口', () => {
  it('候选下拉不吃丢码姓名', () => {
    const store = useAllianceExpertsStore()
    store.experts = [NAMED, LOST]
    expect(store.expertOptions).toEqual([{ value: NAMED.id, label: '甲' }, { value: LOST_ID, label: LOST_SHORT }])
  })

  it('id→名字映射不吃丢码姓名', () => {
    const store = useAllianceExpertsStore()
    store.experts = [NAMED, LOST]
    expect(store.expertNames).toEqual({ [NAMED.id]: '甲', [LOST_ID]: LOST_SHORT })
  })

  it('停用通知点名丢码专家而不是印问号串', async () => {
    api.deleteExpert.mockResolvedValue({ deleted: true, softDelete: true })
    const store = useAllianceExpertsStore()
    store.experts = [LOST]
    await store.removeExpert(LOST)
    expect(store.notice).toContain(LOST_SHORT)
    expect(store.notice).not.toContain('?????????')
  })

  it('负载行：目录里有这行但名字丢了 → 短码；没有这行 → 空串交给视图说', () => {
    const store = useAllianceConsoleStore()
    store.dispatchCandidates = [{ id: LOST_ID, name: LOST.name }]
    store.dispatcherStatus = {
      expertLoads: [
        { expertId: LOST_ID, currentLoad: 1, maxConcurrent: 3 },
        { expertId: 'exp-not-in-catalog', currentLoad: 0, maxConcurrent: 3 }
      ]
    }
    expect(store.loadRows.map((r) => r.name)).toEqual([LOST_SHORT, ''])
  })
})

describe('棘轮：模块内不许再出现绕过三口径的姓名出口', () => {
  /** 全模块扫描（跳过测试文件），命中按 `相对路径|整行` 记账，行号不入账以免任何上方编辑都掀桌 */
  const scanModule = (re) => {
    const root = findModuleDir(process.cwd()) + path.sep
    const found = []
    const walk = (dir, prefix) => {
      for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const abs = path.join(dir, entry.name)
        const rel = prefix + entry.name
        if (entry.isDirectory()) {
          walk(abs, rel + '/')
          continue
        }
        if (!/\.(js|vue)$/.test(entry.name) || /\.test\.js$/.test(entry.name)) continue
        readFileSync(abs, 'utf-8').split('\n').forEach((line) => {
          if (re.test(line)) found.push(`${rel}|${line.trim()}`)
        })
      }
    }
    walk(root, '')
    return found
  }
  const ledger = (name, found, allowed) => {
    const extra = found.filter((x) => !allowed.includes(x)).sort()
    const stale = allowed.filter((x) => !found.includes(x)).sort()
    expect(extra, `${name}：出现了没过三口径的出口 → ${extra.join(' ;; ')}`).toEqual([])
    expect(stale, `${name}：台账里有已经不存在的行（收掉就删条目，别留空位）→ ${stale.join(' ;; ')}`).toEqual([])
    expect(found.length, `${name}：命中数与台账数不符`).toBe(allowed.length)
  }

  it('裸 .name 兜底只剩台账登记的那几处，且每处都不是"展示一个人的名字"', () => {
    // 逐条理由：① 能力名不是人名 ② 写入侧草稿 ③ 过滤"这条能力有没有内容" ④ 图谱节点归一化（展示走 graphNodeLabel）
    // ⑤ 空串=目录里没这行，那句话由视图自己说（丢码分支已由 store 补成短码）
    const ALLOWED = [
      'components/ExpertCapabilityMatrix.vue|<span class="ecm-item-name" :title="c.id">{{ c.name || c.id }}</span>',
      'contract/registry.js|name: String(c?.name ?? \'\').trim(),',
      'contract/registry.js|.filter((c) => c.name || c.domain || Number.isFinite(c.proficiency) || c.description)',
      'model/normalize.js|name: str(n.name ?? n.label),',
      'views/AllianceConsoleView.vue|<span class="acks-name">{{ row.name || \'（目录里没有名字）\' }}</span>'
    ]
    ledger('裸兜底账', scanModule(/\.name\s*(\|\||\?\?)/), ALLOWED)
  })

  it('模板直出 .name 的位置只剩台账登记的那几处', () => {
    // 逐条理由：① 行名已在 model/rank.js 过 expertDisplayName ② 能力名输入框，不是人名
    // ③④ 编排流程的"步骤名"，不是人名（同名两处，各占一条账）
    const ALLOWED = [
      'components/ExpertRankBoard.vue|<span class="ar-name">{{ row.name }}</span>',
      'components/ExpertRegistryForm.vue|<el-input v-model="cap.name" placeholder="能力名称" :maxlength="60" />',
      'views/AllianceOrchestrationView.vue|<span class="aov-step-name">{{ s.name }}</span>',
      'views/AllianceOrchestrationView.vue|<span class="aov-step-name">{{ s.name }}</span>'
    ]
    ledger('模板直出账', scanModule(/\{\{\s*[\w?.$]+\.name\s*\}\}|(?:v-model|:title)="[\w?.$]+\.name"/), ALLOWED)
  })
})
