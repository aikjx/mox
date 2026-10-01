// 工作台会话面的归一化守卫。
//
// 这里为什么把**模块契约**的针写在一个模块外的测试文件里：本文件管的是"工作台外壳有没有真的用模块口径"，
// 不是契约自身的端点账。塞进 contract/sessions.test.js 会让模块测试替外壳接线买单——
// 改一个 views/workspace 的文件就要重跑整台模块变异电池。
//
// 三件事各自成通道，一条 it 只钉一条（合并会互相顶红）：
// 1. 字段名：后端从不发 updated_at / expert_count / mode（session_to_list_view 见网关
//    alliance/experts_session.rs:113-129 的序列化），只发 created_at / last_active_at / expert_ids / session_type。
// 2. 时间口径：RFC3339 字符串要经模块的 relativeTime+formatTime 两档，不许在视图里减毫秒。
// 3. 类型词表：session_type 取值是 single/multi/debate/enterprise，协作模式（smart/algorithm…）不是它。
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { readFileSync, readdirSync } from 'fs'
import path from 'path'
import ExpertPanel from './panels/ExpertPanel.vue'
import {
  SESSION_TYPES,
  sessionActivityAt,
  sessionListTitle,
  sessionTypeForCollabMode,
  sessionTypeLabel,
  sessionTypeTagType
} from '@/modules/expert-alliance/contract'
import { draftSession, normSession, sessionTimeText } from '@/modules/expert-alliance/model'

const MIN = 60 * 1000

/** 服务端真相：snake_case 线格式，时间与模块草稿行同出一个归一化器 */
function wireSession(overrides = {}) {
  return normSession({
    id: 'sess-wired-1',
    title: '灰度方案讨论',
    expert_ids: ['exp-a', 'exp-b', 'exp-c'],
    session_type: 'enterprise',
    status: 'active',
    created_at: '2026-09-20T02:00:00+00:00',
    last_active_at: new Date(Date.now() - 5 * MIN).toISOString(),
    ...overrides
  })
}

describe('sessionActivityAt：活跃优先、创建兜底', () => {
  it('两个键都在时取 lastActiveAt', () => {
    expect(sessionActivityAt({ lastActiveAt: 'L', createdAt: 'C' })).toBe('L')
  })
  it('缺 lastActiveAt 才回到 createdAt', () => {
    expect(sessionActivityAt({ createdAt: 'C' })).toBe('C')
    expect(sessionActivityAt({ lastActiveAt: '', createdAt: 'C' })).toBe('C')
  })
  it('两个都没有就是空串，不许编一个当前时间', () => {
    expect(sessionActivityAt({})).toBe('')
    expect(sessionActivityAt(undefined)).toBe('')
  })
})

describe('sessionTypeForCollabMode：协作模式 → session_type', () => {
  it('辩论优先于人数，1 人辩题也不会被说成多专家', () => {
    expect(sessionTypeForCollabMode('debate', 0)).toBe('debate')
    expect(sessionTypeForCollabMode('debate', 3)).toBe('debate')
  })
  it('其余模式按已选专家数分档：>1 为 multi，否则 single', () => {
    expect(sessionTypeForCollabMode('multi', 3)).toBe('multi')
    expect(sessionTypeForCollabMode('multi', 1)).toBe('single')
    expect(sessionTypeForCollabMode('smart', 2)).toBe('multi')
    expect(sessionTypeForCollabMode('route', 0)).toBe('single')
  })
  it('人数缺失/非数字/字符串数字都不炸，且产出永远落在后端词表内', () => {
    const values = SESSION_TYPES.map((t) => t.value)
    const cases = ['multi', 'smart', 'algorithm', 'route', '', undefined, null]
    for (const mode of cases) {
      for (const count of [undefined, NaN, '3', 'x', -1, 2]) {
        const got = sessionTypeForCollabMode(mode, count)
        expect(values, `mode=${String(mode)} count=${String(count)}`).toContain(got)
      }
    }
    expect(sessionTypeForCollabMode('multi', '3')).toBe('multi')
    expect(sessionTypeForCollabMode('multi', NaN)).toBe('single')
    expect(sessionTypeForCollabMode('multi', -1)).toBe('single')
  })
})

describe('sessionTypeTagType：配色分支与后端词表逐项对齐', () => {
  it('四个类型各得一个非兜底档，彼此不塌成同一色', () => {
    const tags = SESSION_TYPES.map((t) => sessionTypeTagType(t.value))
    tags.forEach((tag) => expect(tag).not.toBe('danger'))
    expect(new Set(tags).size).toBe(SESSION_TYPES.length)
  })
  it('旧私表里的 smart/algorithm 不是会话类型，野值一律 danger', () => {
    expect(sessionTypeTagType('smart')).toBe('danger')
    expect(sessionTypeTagType('algorithm')).toBe('danger')
    expect(sessionTypeTagType('')).toBe('danger')
    expect(sessionTypeTagType(undefined)).toBe('danger')
  })
})

describe('sessionListTitle / sessionTimeText', () => {
  it('空标题不许渲染成一行空白', () => {
    expect(sessionListTitle({ id: 'sess-9', title: '' })).toBe('（无标题）sess-9')
    expect(sessionListTitle({ id: 'sess-9', title: '甲' })).toBe('甲')
  })
  it('一天以内相对、超出回到绝对时间（绝对值只断年份，时区不参与断言）', () => {
    expect(sessionTimeText(new Date(Date.now() - 5 * MIN).toISOString())).toBe('5 分钟前')
    const stale = sessionTimeText('2026-09-01T00:00:00+00:00')
    expect(stale).not.toContain('分钟前')
    expect(stale).not.toContain('小时前')
    expect(stale).toContain('2026')
  })
  it('没有时间与坏时间各自显形，不落进同一档', () => {
    expect(sessionTimeText('')).toBe('—')
    expect(sessionTimeText('not-a-date')).toBe('not-a-date')
  })
  it('相对档四档都从会话这一行真的打得通（副本委托后这里是唯一接线证据）', () => {
    expect(sessionTimeText(new Date(Date.now() - 20 * 1000).toISOString())).toBe('刚刚')
    expect(sessionTimeText(new Date(Date.now() - 3 * 3600000).toISOString())).toBe('3 小时前')
    expect(sessionTimeText(new Date(Date.now() + 5 * MIN).toISOString())).toBe('稍后')
    expect(sessionTimeText(new Date(Date.now() - 3 * 86400000).toISOString())).toContain('/')
  })
})

describe('draftSession：草稿行与服务端行同形', () => {
  const AT = '2026-09-27T06:00:00.000Z'
  const draft = draftSession({ id: 'sess-d', title: '草稿', expertIds: ['exp-a', 'exp-b'], sessionType: 'multi', at: AT })
  const server = wireSession({ id: 'sess-d', title: '草稿', expert_ids: ['exp-a', 'exp-b'], session_type: 'multi', created_at: AT, last_active_at: AT })

  it('键集合与服务端行逐字相同（两套形状＝同一个列表里两套口径）', () => {
    expect(Object.keys(draft).sort()).toEqual(Object.keys(server).sort())
  })
  it('at 同时落进 created 与 lastActive，抄的是后端创建那一刻', () => {
    expect(draft.createdAt).toBe(AT)
    expect(draft.lastActiveAt).toBe(AT)
  })
  it('草稿从未落库，所以不带服务端状态；界面不许把它读成 active', () => {
    expect(draft.status).toBe('')
    expect(draft.archived).toBe(false)
  })
  it('缺成员/缺类型时各自为空值，计数读得出 0 而不是崩', () => {
    const empty = draftSession({ id: 'sess-e', title: '', at: AT })
    expect(empty.expertIds).toEqual([])
    expect(empty.sessionType).toBe('')
    // 先认字段本身：只写 sessionActivityAt 会被 created_at 兜底通道遮蔽（撤掉 last_active_at 也照样绿）
    expect(empty.lastActiveAt).toBe(AT)
    expect(sessionActivityAt(empty)).toBe(AT)
    expect(sessionListTitle(empty)).toBe('（无标题）sess-e')
  })
})

describe('ExpertPanel 会话行渲染：只认后端真有的键', () => {
  const mountPanel = (sessions) =>
    mount(ExpertPanel, { props: { collapsed: false, sessions, activeSession: null } })

  it('专家数取 expertIds.length，不是后端没有的 expert_count', () => {
    const w = mountPanel([wireSession()])
    expect(w.find('.ws-session-experts').text()).toBe('3 位专家')
  })
  it('空成员的行显示 0 位而不是隐藏', () => {
    // 归一化之后对象里本来就没有 expert_count，只把行数改成 0 的话，读错键（s.expert_count || 0）与读对键
    // 都得 0 ⇒ 这一枚针必须混进一个"后端哪天给了个不一致的 expert_count"才能区分两个方向
    const row = { ...wireSession({ expert_ids: undefined }), expert_count: 7 }
    const w = mountPanel([row])
    expect(w.find('.ws-session-experts').text()).toBe('0 位专家')
  })
  it('时间取 lastActiveAt：读错成 created_at 会掉到绝对档', () => {
    const w = mountPanel([wireSession()])
    expect(w.find('.ws-session-time').text()).toBe('5 分钟前')
  })
  it('服务端行现在有类型标签，且文案与配色都来自 session_type 词表', () => {
    const w = mountPanel([wireSession()])
    expect(w.find('.ws-session-mode').text()).toBe('企业级')
    expect(w.find('.ws-session-mode').html()).toContain('type="info"')
    expect(w.find('.ws-session-mode').html()).not.toContain('智能路由')
  })
  it('未知类型原样显形并给 danger，不许换成中性色掩盖', () => {
    const w = mountPanel([wireSession({ session_type: 'enterprise_plus' })])
    expect(w.find('.ws-session-mode').text()).toBe('enterprise_plus')
    expect(sessionTypeLabel('enterprise_plus')).toBe('enterprise_plus')
    expect(w.find('.ws-session-mode').html()).toContain('type="danger"')
  })
  it('空标题行渲染出短码标题', () => {
    const w = mountPanel([wireSession({ title: '' })])
    expect(w.find('.ws-session-title').text()).toBe('（无标题）sess-wired-1')
  })
})

describe('棘轮：工作台会话面不许再出现后端没有的键', () => {
  /**
   * 扫描 src/views/workspace 与 src/composables/workspace 的代码行。
   * 整行注释豁免——本文件的注释要指名旧键名说明病灶，那是文档不是读字段；
   * 代价是把代码挪进注释行可以躲过这一格，所以同一批文件另有"必须出现"的正面对账。
   */
  const ROOTS = ['src/views/workspace', 'src/composables/workspace']
  /** 会话形态的读法：整个工作台都不许出现 */
  const BANNED_ALL = [
    { re: /\bsession(s)?\.(updated_at|expert_count|mode)\b/, why: '后端没有这些键，成员数在 expert_ids、类型在 session_type' },
    { re: /\.expert_count\b/, why: '后端从不发 expert_count，成员数在 expert_ids' },
    { re: /\bgetExpertSessions\b/, why: '会话列表走 allianceApi.listSessions，信封解包只在一处' }
  ]
  /** 裸 .updated_at / s.mode 只在这两个"渲染会话"的文件里算病灶：
   *  KnowledgeBasePanel 读的是文档对象的 doc.updated_at，那是另一个面的账，本单元不裁。 */
  const BANNED_SESSION_FILES = [
    { re: /\.updated_at\b/, why: '会话的时间只有 created_at / last_active_at' },
    { re: /\bs\.(mode|expert_count|updated_at)\b/, why: '投影行的变量 s 上只有 normSession 那套键' }
  ]
  const REQUIRED = {
    'src/views/workspace/ExpertWorkspaceView.vue': [
      'allianceApi.listSessions(',
      'sessionActivityAt(',
      'sessionTypeForCollabMode(',
      'draftSession('
    ],
    'src/views/workspace/panels/ExpertPanel.vue': [
      'sessionActivityAt(',
      'sessionListTitle(',
      'sessionTypeTagType('
    ]
  }

  function codeLines(rel, src) {
    return src
      .split('\n')
      .map((line, i) => ({ n: i + 1, line }))
      .filter(({ line }) => !/^\s*(\/\/|\*|\/\*|<!--)/.test(line))
  }

  function walk(dir, prefix, out) {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const abs = path.join(dir, entry.name)
      const rel = prefix + entry.name
      if (entry.isDirectory()) walk(abs, rel + '/', out)
      else if (/\.(js|vue)$/.test(entry.name) && !/\.test\.js$/.test(entry.name)) {
        out.push({ rel, src: readFileSync(abs, 'utf-8') })
      }
    }
    return out
  }

  const root = path.resolve(process.cwd()).replace(/\\/g, '/')
  const files = ROOTS.flatMap((r) => walk(path.join(root, r), `${r}/`, []))

  it('扫描集非空，且覆盖到两个会话面文件', () => {
    expect(files.length).toBeGreaterThan(0)
    const seen = files.map((f) => f.rel)
    for (const want of Object.keys(REQUIRED)) {
      expect(seen, `扫描集里没有 ${want}，这一格会空转`).toContain(want)
    }
  })

  it('没有后端不存在的键被读', () => {
    const hits = []
    for (const { rel, src } of files) {
      const rules = Object.prototype.hasOwnProperty.call(REQUIRED, rel)
        ? BANNED_ALL.concat(BANNED_SESSION_FILES)
        : BANNED_ALL
      for (const { n, line } of codeLines(rel, src)) {
        for (const { re, why } of rules) {
          if (re.test(line)) hits.push(`${rel}:${n} ${why} → ${line.trim()}`)
        }
      }
    }
    expect(hits, `工作台会话面又在猜字段名：${hits.join(' ;; ')}`).toEqual([])
  })

  it('模块口径确实被接上（防止靠删代码通过上一格）', () => {
    for (const [rel, needles] of Object.entries(REQUIRED)) {
      const file = files.find((f) => f.rel === rel)
      const code = codeLines(rel, file.src).map((x) => x.line).join('\n')
      for (const needle of needles) {
        expect(code, `${rel} 里找不到 ${needle}，接线被撤了`).toContain(needle)
      }
    }
  })

  it('两个会话面文件不再自带时间格式化函数', () => {
    for (const rel of Object.keys(REQUIRED)) {
      const file = files.find((f) => f.rel === rel)
      const code = codeLines(rel, file.src).map((x) => x.line).join('\n')
      expect(code, `${rel} 又自带了一个时间格式化器`).not.toMatch(/function formatTime\b/)
      expect(code).not.toMatch(/const formatTime\s*=/)
    }
  })
})
