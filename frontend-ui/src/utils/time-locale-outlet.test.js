// F2 族（zh-CN + hour12:false 那把 toLocaleString 全等写法）8 处逐字符副本收口后的钉子。
// 标题里故意不把被禁字面量整条写出来——下面第 5 条棘轮会扫本文件，写了就自己命中自己。
// 三条线：输出等价（换了出口不许改版面）／坏值契约（null 交回调用方兜底）／棘轮（这个字面量只许活在 utils/time.js）。
import { describe, it, expect } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { formatDateTimeLocale } from '@/utils/time'
import { formatDateTimeLocale as viaBarrel } from '@/utils'

// 旧写法现场重建（用变量拆开，否则下面第 5 条棘轮会在本文件里命中自己）
const LEGACY_LOCALE = 'zh-CN'
const LEGACY_OPTS = { hour12: false }
const legacyFormat = (v) => new Date(v).toLocaleString(LEGACY_LOCALE, LEGACY_OPTS)

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')

// 拆成两段拼接：本文件既是棘轮的扫描对象，就不能把被测字面量整条写在里面
const FORBIDDEN = ".toLocaleString(" + "'zh-CN', { hour12: false })"

const SAMPLES = [
  '2026-09-28T13:05:09',
  '2026-01-05T00:00:00',
  '2026-12-31T23:59:59+08:00',
  1758000000000,
  new Date('2026-09-28T04:03:02.123456Z'),
  '2026-09-28 09:00:00',
]

describe('formatDateTimeLocale 单出口', () => {
  it('对可解析输入与旧表达式逐字符相同', () => {
    for (const s of SAMPLES) {
      expect(formatDateTimeLocale(s)).toBe(legacyFormat(s))
    }
  })

  it('hour12 确实关掉：零点打印 00:00:00 而不是 12:00:00', () => {
    expect(formatDateTimeLocale('2026-01-05T00:00:00')).toMatch(/00:00:00$/)
  })

  it('坏值返回 null，把空态文案交回调用方（各面板契约不同：- / — / 原样回显）', () => {
    for (const bad of [0, -1, '', '   ', null, undefined, NaN, 'not-a-date', {}]) {
      expect(formatDateTimeLocale(bad)).toBe(null)
    }
  })

  it('barrel 出口与直接导入同一个函数', () => {
    expect(viaBarrel).toBe(formatDateTimeLocale)
  })

  it('epoch 毫秒数字串不再落成 Invalid Date（旧写法 new Date("1758000000000") 是 Invalid Date）', () => {
    expect(formatDateTimeLocale('1758000000000')).toBe(legacyFormat(1758000000000))
    expect(legacyFormat('1758000000000')).toBe('Invalid Date')
  })

  it('棘轮：F2 字面量全库只许剩登记口那 1 处', () => {
    const hits = []
    const walk = (dir) => {
      for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
        const p = path.join(dir, e.name)
        if (e.isDirectory()) { walk(p); continue }
        if (!/\.(vue|js|ts)$/.test(e.name)) continue
        const n = fs.readFileSync(p, 'utf8').split(FORBIDDEN).length - 1
        if (n) hits.push(`${path.relative(SRC, p).split(path.sep).join('/')}:${n}`)
      }
    }
    walk(SRC)
    expect(hits).toEqual(['utils/time.js:1'])
  })

  it('出口消费者都真的引到了出口（防驱动器静默 no-op）', () => {
    const consumers = [
      'views/admin/panels/AdminMonitor.vue',
      'views/project/Dashboard.vue',
      'modules/expert-alliance/components/ExpertBookingPanel.vue',
      'modules/expert-alliance/model/display.js',
    ]
    for (const rel of consumers) {
      const src = fs.readFileSync(path.join(SRC, rel), 'utf8')
      // 只钉"这个名字经由 barrel 进来"，不钉同一句里还有哪些同伴名字——
      // 同伴集合会随别的收口轮次增减，钉整句等于让别的单元改不动这一行（§5.34 撞上过）
      expect(src, rel).toMatch(/import \{[^}]*\bformatDateTimeLocale\b[^}]*\} from '@\/utils'/)
      expect(src, rel).toContain('formatDateTimeLocale(d)')
      expect(src, rel).not.toContain(FORBIDDEN)
    }
  })

  it('四处把副本收成别名 import（§5.36），引的是带空态文案的那一档', () => {
    const aliased = [
      'views/admin/panels/AdminDepartment.vue',
      'views/admin/panels/AdminRole.vue',
      'views/admin/panels/AdminUser.vue',
      'views/expert/panels/ExpertOrchestratorPanel.vue',
    ]
    for (const rel of aliased) {
      const src = fs.readFileSync(path.join(SRC, rel), 'utf8')
      expect(src, rel).toMatch(/import \{[^}]*formatDateTimeLocaleOr\s+as\s+formatTime[^}]*\} from '@\/utils'/)
      expect(src, rel).not.toMatch(/^\s*function\s+formatTime\(/m)
      expect(src, rel).not.toContain(FORBIDDEN)
    }
  })
})
