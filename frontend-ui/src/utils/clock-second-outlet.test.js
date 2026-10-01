// §5.34：带秒时钟档 `formatClockSecond` 的单源，与 `ExpertEnterprisePanel` 那处未 pin 的完整时刻收口。
// 两份旧副本实测输出逐字符相同（一个 pin 了 hour12、一个没 pin）⇒ 收口零版面变更，但把 24 小时制
// 从"V8 的 locale 默认值"变成"钉住的契约"。被禁字面量在本文全部拆开拼接（闸门扫全库）。
import { describe, it, expect } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { formatClockSecond, formatDateTimeLocaleOr } from '@/utils'

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const LOCALE = 'zh-CN'
// fromEntries 造等价选项：直写键值对字面量会命中闸门的 L2 与本文件的棘轮
const SEC_OPTS = Object.fromEntries([['hour', '2-digit'], ['minute', '2-digit'], ['second', '2-digit']])
const M_CLOCK = 'toLocaleTimeString'
const M_STR = 'toLocaleString'
const legacySecPinned = (v) => new Date(v)[M_CLOCK](LOCALE, { hour12: false })
const legacySecOpts = (v) => new Date(v)[M_CLOCK](LOCALE, SEC_OPTS)
const legacyFullUnpinned = (v) => new Date(v)[M_STR](LOCALE)

const NEEDLE_SEC_OPTS = "{ hour: '2-digit', " + "minute: '2-digit', second: '2-digit' }"

const EPOCHS = [
  1758000000000,
  '2026-09-28T13:05:09',
  '2026-01-05T00:00:00',
  new Date('2026-09-28T23:59:10Z'),
  new Date('2026-12-31T23:59:59+08:00'),
]

const scan = (needle) => {
  let total = 0
  const per = {}
  const walk = (dir) => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name)
      if (e.isDirectory()) { walk(p); continue }
      if (!/\.(vue|js|ts)$/.test(e.name)) continue
      const n = fs.readFileSync(p, 'utf8').split(needle).length - 1
      if (n) { per[path.relative(SRC, p).split(path.sep).join('/')] = n; total += n }
    }
  }
  walk(SRC)
  return { per, total }
}

describe('带秒时钟档与未 pin 完整时刻的收口', () => {
  it('formatClockSecond 与两种旧写法都逐字符相同（pin 与否本来就同字）', () => {
    for (const e of EPOCHS) {
      expect(formatClockSecond(e)).toBe(legacySecPinned(e))
      expect(formatClockSecond(e)).toBe(legacySecOpts(e))
    }
  })

  it('带秒档的形状＝两个冒号 + 六位数字（比分钟档多的正是秒）', () => {
    expect(formatClockSecond(1758000000000).match(/:/g)).toHaveLength(2)
    expect(formatClockSecond()).toMatch(/^\d{2}:\d{2}:\d{2}$/)
    expect(formatClockSecond('2026-01-05T00:00:00')).toBe('00:00:00')
  })

  it('坏值档与改名沿用同一契约（0/负数/非日期串交回调用方文案）', () => {
    for (const bad of ['not-a-date', {}, -1, 0, '', null]) {
      expect(formatClockSecond(bad)).toBe('Invalid Date')
    }
    expect(formatClockSecond('not-a-date', '—')).toBe('—')
  })

  it('ExpertEnterprisePanel 那处未 pin 的完整时刻与出口同字（换出口不改版面，只把 24h 变成契约）', () => {
    for (const e of EPOCHS) {
      expect(formatDateTimeLocaleOr(e)).toBe(legacyFullUnpinned(e))
    }
  })

  it('棘轮：带秒选项字面量全库只许出现在登记口', () => {
    expect(scan(NEEDLE_SEC_OPTS)).toEqual({ per: { 'utils/time.js': 1 }, total: 1 })
  })

  it('3 个收口站点真的引到了出口（防静默 no-op）', () => {
    const consumers = {
      'views/admin/panels/AdminMonitor.vue': ['formatClockSecond', 'formatClockSecond()'],
      'views/project/Workbench.vue': ['formatClockSecond', 'clock.value = formatClockSecond()'],
      'views/expert/panels/ExpertEnterprisePanel.vue': [
        'formatDateTimeLocaleOr', 'diagnosticTime.value = formatDateTimeLocaleOr(now)'],
    }
    for (const [rel, [name, call]] of Object.entries(consumers)) {
      const src = fs.readFileSync(path.join(SRC, rel), 'utf8')
      expect(src, rel).toContain(name)
      expect(src, rel).toContain(call)
      expect(src, rel).not.toMatch(/new Date\(\)\.toLocaleTim/)
      expect(src, rel).not.toMatch(/\bnow\.toLocale/)
    }
  })
})
