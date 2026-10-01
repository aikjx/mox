// §5.35：F4 族（手工拼接日期/时间数字）的收口账与棘轮。
// 现测（reports/data/f4-census-20260929.txt）：收口前 10 处 / 6 种字段组合，收口后 7 处（含登记口 utils/time.js
// 与被排除的 .test.js）。本轮只并"逐字节相同"的 3 处（零版面变更），其余 5 处按"档不同不并"登记。
// 本文件自己的棘轮扫描必须排除 .test.js，否则会命中本文件头注释与下面的旧写法副本。
import { describe, it, expect } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { formatClockMinute, formatDateTime } from '@/utils'

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const p2 = (n) => String(n).padStart(2, '0')

// 三处被收口的旧写法：逐字从磁盘抄回（收口前的锚点行号见 reports/data/f4-census-20260929.txt）
const legacyClockA = (t) => { const d = new Date(t); return p2(d.getHours()) + ':' + p2(d.getMinutes()) }
const legacyClockB = (t) => {
  const d = new Date(t)
  return d.getHours().toString().padStart(2, '0') + ':' + d.getMinutes().toString().padStart(2, '0')
}
const legacyTable = (t) => {
  const d = new Date(t)
  return `${d.getFullYear()}-${p2(d.getMonth() + 1)}-${p2(d.getDate())} ${p2(d.getHours())}:${p2(d.getMinutes())}`
}

const EPOCHS = [
  1758000000000,
  Date.UTC(2026, 0, 1, 0, 0, 0),
  Date.UTC(2026, 8, 28, 15, 7, 5),
  Date.UTC(2026, 11, 31, 23, 59, 59),
  Date.UTC(2026, 9, 1, 4, 5, 0),
]

// 棘轮针：读字段 + 同行有拼接痕迹（模板插值 / padStart / + 字符串）
const GETTER = '\\.get(?:FullYear|Month|Date|Hours|Minutes|Seconds|Day|Time)\\('
const ASSEMBLY = 'padStart|\\$\\{|toString\\(\\)\\s*\\+|\\+\\s*[\'"]|\'[T:-]\''
const needleLine = (ln) => new RegExp(GETTER).test(ln) && new RegExp(ASSEMBLY).test(ln)

const scan = () => {
  const per = {}
  let total = 0
  let files = 0
  const walk = (dir) => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name)
      if (e.isDirectory()) { walk(p); continue }
      if (!/\.(vue|js|ts)$/.test(e.name)) continue
      if (/\.(test|stories)\.js$/.test(e.name)) continue
      files++
      const hits = fs.readFileSync(p, 'utf8').split('\n').filter(needleLine).length
      if (hits) { per[path.relative(SRC, p).split(path.sep).join('/')] = hits; total += hits }
    }
  }
  walk(SRC)
  return { per, total, files }
}

// 余账登记（档不同不并，逐处理由见 §5.35 的表）：utils/time.js 是登记口本身
const REGISTERED = {
  'utils/time.js': 1,
  'components/MessageBubble.vue': 1,
  'views/admin/panels/AdminLogs.vue': 1,
  'views/ai/ChatView.vue': 1,
  'views/expert/ExpertPlazaView.vue': 1,
  'views/expert/panels/ExpertEnterprisePanel.vue': 1,
}
const REGISTERED_TOTAL = Object.values(REGISTERED).reduce((a, b) => a + b, 0)

describe('F4 手工拼接档的收口与余账', () => {
  it('时钟档两处旧写法与 formatClockMinute 逐字符相同（本档收口不许改任何一个用户看见的字）', () => {
    for (const e of EPOCHS) {
      expect(formatClockMinute(e)).toBe(legacyClockA(e))
      expect(formatClockMinute(e)).toBe(legacyClockB(e))
    }
  })

  it('表格档旧写法与 formatDateTime 逐字符相同（坏值文案交回调用方）', () => {
    for (const e of EPOCHS) {
      expect(formatDateTime(e, '—')).toBe(legacyTable(e))
    }
    // 不断言绝对值：这台机是 UTC+8，钉死 '2025-09-16 13:20' 会在别的时区假红（只钉形状）
    expect(formatDateTime(1758000000000, '—')).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$/)
  })

  it('非正时刻是本轮唯一的有意版面变更：旧写法印 1970 前后，出口回到空态文案', () => {
    expect(formatDateTime(0, '—')).toBe('—')
    expect(formatDateTime(-1, '—')).toBe('—')
    // 旧写法确实会印出一个日期串（不是空态）——这就是被有意撤掉的版面
    expect(legacyTable(1)).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$/)
  })

  it('FlowDetailDialog 的坏值原样回显契约还在（出口只给空态文案，不合并这一支）', () => {
    const src = fs.readFileSync(path.join(SRC, 'components/FlowDetailDialog.vue'), 'utf8')
    expect(src).toContain('Number.isNaN(d.getTime())')
    expect(src).toMatch(/return String\(s\)/)
    expect(src).toMatch(/formatDateTime\(d, '—'\)/)
  })

  it('三个收口站点真的经 barrel 引到了出口（防静默 no-op；深路径 @/utils/time 会被 E6 判 ERROR）', () => {
    const consumers = {
      'components/MessageBubble.vue': 'formatClockMinute',
      'composables/workspace/useWorkspaceData.js': 'formatClockMinute',
      'components/FlowDetailDialog.vue': 'formatDateTime',
    }
    for (const [rel, name] of Object.entries(consumers)) {
      const src = fs.readFileSync(path.join(SRC, rel), 'utf8')
      expect(src, rel).toMatch(new RegExp(`import \\{[^}]*\\b${name}\\b[^}]*\\} from ['"]@/utils['"]`))
      expect(src, rel).not.toMatch(/from ['"]@\/utils\/time['"]/)
    }
  })

  it('useWorkspaceData 的同名包装已随收口删除（不留一行纯别名）', () => {
    const src = fs.readFileSync(path.join(SRC, 'composables/workspace/useWorkspaceData.js'), 'utf8')
    expect(src).not.toMatch(/function nowTime\b/)
    expect(src.match(/formatClockMinute\(\)/g)).toHaveLength(4)
  })

  it('棘轮：手工拼接余账按文件登记＝6 处（登记口 1 ＋ 档不同不并 5），总数从登记表现推', () => {
    const { per, total, files } = scan()
    console.log(`[F4] 扫描 ${files} 个文件，余账 ${total} 处`)
    // 分母只当塌缩保护（与闸门 L4 同口径），实测值每次印出来；判集塌缩时"0 命中"与"没有缺陷"同形
    expect(files).toBeGreaterThanOrEqual(100)
    expect(total).toBe(REGISTERED_TOTAL)
    expect(per).toEqual(REGISTERED)
  })

  it('已收口的两处不再残留字段拼接（台账缺席即证明，不靠回忆）', () => {
    const { per } = scan()
    for (const rel of ['composables/workspace/useWorkspaceData.js', 'components/FlowDetailDialog.vue']) {
      expect(per, rel).not.toHaveProperty(rel)
    }
  })
})
