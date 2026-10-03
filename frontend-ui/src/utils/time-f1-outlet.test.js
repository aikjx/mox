// F1 族（无参 locale 格式化＝输出跟随访问者浏览器区域设置）4 份逐字符相同副本收口后的钉子。
// 标题与注释都不写整条被禁字面量——第 5 条棘轮会扫本文件，写了就自己命中自己（本轮登记口注释就真踩过一次：基线多出 1 处假命中）。
import { describe, it, expect } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { formatDateTimeLocale, formatDateTimeLocaleOr } from '@/utils/time'
import { formatDateTimeLocaleOr as viaBarrel } from '@/utils'

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const FORBIDDEN = '.toLocaleString(' + ')'
// 无参等价写法：显式传 undefined 取宿主默认区域，避免把被禁字面量整条写进本文件
const legacyNoArg = (v) => new Date(v).toLocaleString(undefined)

// 收口后仍允许存在的位置（实测扫描数出，两种角色）：
//   数字千分位 3 处＝与时间无关的合法用途；时间戳 2 处＝两份契约各不相同，留在 §5.30 余账逐文件裁决。
// （TaskView 那处已在 §5.33 收口：它是展示串写进 created_at 数据字段，属写侧污染，不是展示契约差异。）
const REGISTERED = {
  'modules/admin-lowcode/engine/widgetRegistry.js': 1,
  'views/admin/panels/AdminHitl.vue': 1,
  'views/admin/panels/AdminLlm.vue': 2,
  'views/expert/ExpertPlazaView.vue': 1,
}
// 总数由台账求和导出，不再手抄第二个数字（两处副本会腐烂；逐文件计数另有 toEqual 钉住）
const REGISTERED_TOTAL = Object.values(REGISTERED).reduce((a, b) => a + b, 0)

const SAMPLES = [
  '2026-09-28T13:05:09',
  '2026-01-05T00:00:00',
  '2026-12-31T23:59:59+08:00',
  1758000000000,
  '2026-09-28 09:00:00',
  new Date('2026-09-28T04:03:02.123456Z'),
]

describe('formatDateTimeLocaleOr 单出口（F1 族收口）', () => {
  it('可解析输入与 F2 出口逐字符相同（两档共用同一"能解析时怎么印"）', () => {
    for (const s of SAMPLES) {
      expect(formatDateTimeLocaleOr(s)).toBe(formatDateTimeLocale(s))
    }
  })

  it('无值档：falsy 一律回落占位符，默认 - 且可由调用方改名', () => {
    for (const v of [0, '', null, undefined, NaN, false]) {
      expect(formatDateTimeLocaleOr(v)).toBe('-')
      expect(formatDateTimeLocaleOr(v, '—')).toBe('—')
    }
  })

  it('坏值档：解析失败印 Invalid Date，与旧副本的可见行为一致', () => {
    for (const bad of ['not-a-date', '   ', {}, [], -1]) {
      expect(formatDateTimeLocaleOr(bad)).toBe('Invalid Date')
    }
    // 旧写法里 catch { return String(t) } 是死支：坏值不抛，直接落成 Invalid Date
    expect(legacyNoArg('not-a-date')).toBe('Invalid Date')
  })

  it('barrel 出口与直接导入同一个函数', () => {
    expect(viaBarrel).toBe(formatDateTimeLocaleOr)
  })

  it('棘轮：被禁字面量的余量按文件登记，总数与登记表自证闭合', () => {
    const found = {}
    let total = 0
    const walk = (dir) => {
      for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
        const p = path.join(dir, e.name)
        if (e.isDirectory()) { walk(p); continue }
        if (!/\.(vue|js|ts)$/.test(e.name)) continue
        const n = fs.readFileSync(p, 'utf8').split(FORBIDDEN).length - 1
        if (!n) continue
        found[path.relative(SRC, p).split(path.sep).join('/')] = n
        total += n
      }
    }
    walk(SRC)
    expect(total).toBe(REGISTERED_TOTAL)
    expect(found).toEqual(REGISTERED)
  })

  it('4 个收口点都真的引到了出口，且旧副本体已不在文件里（防驱动器静默 no-op）', () => {
    const consumers = [
      'views/admin/panels/AdminAccess.vue',
      'views/admin/panels/AdminAudit.vue',
      'views/admin/panels/AdminConfig.vue',
      'views/admin/panels/AdminMenu.vue',
    ]
    for (const rel of consumers) {
      const src = fs.readFileSync(path.join(SRC, rel), 'utf8')
      expect(src, rel).toMatch(/import\s*\{\s*formatDateTimeLocaleOr as fmtTime(?:\s*,[^}]+)?\s*\}\s*from '@\/utils'/)
      expect(src, rel).toContain('fmtTime(')
      expect(src, rel).not.toContain('catch { return String(t) }')
    }
  })
})
