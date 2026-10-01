// 时钟档（HH:mm）与日期档收口的钉子：10 处逐字符副本 → utils/time.js 两个出口。
// 本文件同时是三把棘轮的扫描对象，所以被禁字面量一律拆开写／用计算属性调用，
// 否则钉子会因为自己的文本而假红（§5.27 就在注释里踩过一次）。
import { describe, it, expect } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { formatClockMinute, formatDateStamp } from '@/utils/time'
import { formatClockMinute as clockViaBarrel, formatDateStamp as dateViaBarrel } from '@/utils'

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const LOCALE = 'zh-CN'
// 用 fromEntries 造等价选项对象：直接写键值对字面量会命中第 5 条棘轮
const CLOCK_OPTS = Object.fromEntries([['hour', '2-digit'], ['minute', '2-digit']])
const M_CLOCK = 'toLocaleTimeString'
const M_DATE = 'toLocaleDateString'
const legacyClock = (v) => new Date(v)[M_CLOCK](LOCALE, CLOCK_OPTS)
const legacyDate = (v) => new Date(v)[M_DATE](LOCALE)

const NEEDLE_OPTS = "{ hour: '2-digit', " + "minute: '2-digit' }"
const NEEDLE_DATE = ".toLocaleDateStr" + "ing('zh-CN')"
const NEEDLE_CLOCK = ".toLocaleTimeString" + "("

const EPOCHS = [1758000000000, '2026-09-28T13:05:09', '2026-01-05T00:00:00', new Date('2026-12-31T23:59:59+08:00')]

const scan = (needle) => {
  const per = {}
  let total = 0
  const walk = (dir) => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name)
      if (e.isDirectory()) { walk(p); continue }
      if (!/\.(vue|js|ts)$/.test(e.name)) continue
      const n = fs.readFileSync(p, 'utf8').split(needle).length - 1
      if (!n) continue
      per[path.relative(SRC, p).split(path.sep).join('/')] = n
      total += n
    }
  }
  walk(SRC)
  return { per, total }
}

describe('时钟档/日期档单出口', () => {
  it('时钟档与旧表达式逐字符相同（本档收口不许改任何一个用户看见的字）', () => {
    for (const e of EPOCHS) {
      expect(formatClockMinute(e)).toBe(legacyClock(e))
    }
  })

  it('时钟档不带秒：整串只有一个冒号', () => {
    expect(formatClockMinute(1758000000000)).toBe(legacyClock(1758000000000))
    expect(formatClockMinute(1758000000000).match(/:/g)).toHaveLength(1)
    expect(formatClockMinute()).toMatch(/^\d{2}:\d{2}$/)
  })

  it('日期档与旧表达式逐字符相同，且不含时分行分隔符', () => {
    for (const e of EPOCHS) {
      expect(formatDateStamp(e)).toBe(legacyDate(e))
    }
    expect(formatDateStamp(1758000000000)).not.toContain(':')
  })

  it('坏值档：解析失败印 Invalid Date，可按调用方改名（原副本就是这个可见行为）', () => {
    for (const bad of ['not-a-date', {}, -1, 0]) {
      expect(formatClockMinute(bad)).toBe('Invalid Date')
      expect(formatDateStamp(bad)).toBe('Invalid Date')
    }
    expect(formatClockMinute('not-a-date', '')).toBe('')
  })

  it('barrel 出口与直接导入同一对函数', () => {
    expect(clockViaBarrel).toBe(formatClockMinute)
    expect(dateViaBarrel).toBe(formatDateStamp)
  })

  it('棘轮：选项字面量与两档调用式各只许活在登记口', () => {
    expect(scan(NEEDLE_OPTS).per).toEqual({ 'utils/time.js': 1 })
    expect(scan(NEEDLE_DATE).per).toEqual({ 'utils/time.js': 1 })
  })

  it('棘轮：时钟式调用全库只剩登记口（两档 clock 都在 time.js 内）', () => {
    const { per, total } = scan(NEEDLE_CLOCK)
    expect(total).toBe(2)
    expect(per).toEqual({ 'utils/time.js': 2 })
  })

  it('5 个收口文件都真的引到了出口（防驱动器静默 no-op）', () => {
    const consumers = {
      'composables/workspace/useAlliance.js': 'formatClockMinute',
      'stores/alliance.store.js': 'formatClockMinute',
      'views/workspace/ExpertWorkspaceView.vue': 'formatClockMinute',
      'views/project/Workbench.vue': 'formatDateStamp',
      'modules/expert-alliance/model/rank.js': 'formatDateStamp',
    }
    for (const [rel, name] of Object.entries(consumers)) {
      const src = fs.readFileSync(path.join(SRC, rel), 'utf8')
      expect(src, rel).toContain("from '@/utils'")
      expect(src, rel).toContain(name)
      expect(src, rel).not.toContain(NEEDLE_OPTS)
    }
  })
})
