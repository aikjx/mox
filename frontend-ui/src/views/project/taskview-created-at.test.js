// task #24 第 3 处的钉子：TaskView 的 created_at 曾经是"展示串写进数据字段"（写侧污染）。
// 网关把 created_at 当字符串按字典序排序（platform/gateway/mox-platform-gateway-svc/src/misc.rs），
// 所以只有 RFC3339 可比；浏览器 locale 的展示串会让该行在列表里永久排错位。
// 被禁字面量在这里全部拆开拼接——本文件也在常驻闸门的扫描集内，整条写出来会自己命中自己。
import { describe, it, expect } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { formatDateTimeLocaleOr } from '@/utils'

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const REPO = path.resolve(SRC, '..')
const VIEW = path.join(SRC, 'views/project/TaskView.vue')
const GATE = path.join(REPO, '../scripts/gate/check-locale-format-outlets.py')

// L1 形状：`new Date()` 直接跟无参 toLocaleString（拆成三段，避免与本文件里的正则同源）
const L1_NEW_DATE = 'new Date' + '()'
const L1_TOL = '.to' + 'LocaleString()'
const L1_SHAPE = L1_NEW_DATE + L1_TOL

const src = fs.readFileSync(VIEW, 'utf8')

describe('TaskView created_at 写侧与展示侧口径', () => {
  it('写侧优先采用服务端回显，只在回显缺席时造 ISO（不再覆盖 newTask.created_at）', () => {
    expect(src).toContain('created_at: newTask.created_at || new Date().toISOString()')
  })

  it('写侧禁令：本文件不许再出现浏览器 locale 展示串', () => {
    expect(src).not.toContain(L1_SHAPE)
    expect(src.match(new RegExp(L1_TOL.replace('.', '\\.'), 'g')) || []).toHaveLength(0)
  })

  it('展示侧走单出口，且只走 barrel（深路径会被门禁 E6 判 ERROR）', () => {
    expect(src).toContain("import { formatDateTimeLocaleOr } from '@/utils'")
    expect(src).not.toContain("from '@/utils/time'")
    expect(src).toContain('{{ formatDateTimeLocaleOr(currentTask.created_at) }}')
    expect(src.match(/\{\{ currentTask\.created_at \}\}/g) || []).toHaveLength(0)
  })

  it('RFC3339 的 created_at 能被出口读成时间（而不是落成 Invalid Date 或原样回显）', () => {
    const wire = '2026-09-28T23:59:10Z'
    const shown = formatDateTimeLocaleOr(wire)
    expect(shown).not.toBe('Invalid Date')
    expect(shown).not.toBe(wire)
    // 展示是本地挂钟（本机 +08:00 ⇒ 日期会翻到 09-29），所以钉"形状 + 指回同一瞬间"，不钉具体日
    const m = /^(\d{4})[/-](\d{1,2})[/-](\d{1,2})\D+(\d{1,2}):(\d{2}):(\d{2})$/.exec(shown)
    expect(m, shown).toBeTruthy()
    const [, Y, Mo, D, H, Mi, S] = m
    expect(new Date(Number(Y), Number(Mo) - 1, Number(D), Number(H), Number(Mi), Number(S)).getTime())
      .toBe(Date.parse(wire))
  })

  it('闸门台账已随本次收口收缩：TaskView 条目已删，其余条目指向的文件仍在', () => {
    const ledger = fs.readFileSync(GATE, 'utf8').match(/EXPECTED_UNPINNED = \{([\s\S]*?)\n\}/)[1]
    const keys = [...ledger.matchAll(/"([^"]+)":\s*(\d+)/g)].map(m => [m[1], Number(m[2])])
    expect(keys.map(k => k[0])).not.toContain('views/project/TaskView.vue')
    expect(keys.length).toBe(2)
    // 存在性检查（必要条件，不是充分条件）：台账指向的文件若已无该形状，就是该删的旧条目
    for (const [rel, n] of keys) {
      const text = fs.readFileSync(path.join(SRC, rel), 'utf8')
      const hits = (text.match(new RegExp(L1_TOL.replace('.', '\\.'), 'g')) || []).length
      expect(hits, rel).toBeGreaterThanOrEqual(n)
    }
  })
})
