// 时间显示口径单源（utils/time.js）的守卫，外加一本"全库还有几处自写时间副本"的账。
//
// 为什么值得单独一本账：本轮之前全库在 16 个文件里自写了 17 处 formatTime/relativeTime，分四族，
// 其中 A 族三份函数体逐字符相同（sha256 bc1b7a3afd18a278）、B 族三份亦同（791ebd8c5100169d）。
// 重复本身只是腐烂的入口，真正咬人的是**输入口径**：
// 后端下发 RFC3339 字符串（now_iso()＝chrono to_rfc3339），工作台 KB 面板那份写 `Date.now() - ts` 得到 NaN，
// 于是所有 `diff < X` 恒假，"刚刚/N 分钟前/N 小时前"三档一次也没触发过——界面悄悄退化成 M/D。
import { describe, it, expect } from 'vitest'
import { readFileSync, readdirSync, statSync } from 'fs'
import path from 'path'
import * as utils from '@/utils'
import { formatDateTime, formatDateTimeLocaleOr, relativeTimeText, timeAgoOrDate, timeValue } from '@/utils'

const SRC_DIR = path.resolve(__dirname, '..')
const MINUTE = 60 * 1000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

const at = (offsetMs) => new Date(Date.now() - offsetMs)
const isoAt = (offsetMs) => at(offsetMs).toISOString()

/** 不假设测试机时区：期望值直接用同一时刻的本地分量算出来 */
function localStamp(d) {
  const p = (n) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

describe('timeValue：先把任何形状落成毫秒，再谈档位', () => {
  it('epoch 毫秒数原样通过', () => {
    expect(timeValue(1758000000000)).toBe(1758000000000)
  })

  it('数字串要转成数（new Date("1758000000000") 是 Invalid Date，不转就整列没有档）', () => {
    expect(timeValue('1758000000000')).toBe(1758000000000)
  })

  it('RFC3339 字符串与 Date 对象都读得出同一毫秒', () => {
    const d = at(5 * MINUTE)
    expect(timeValue(d.toISOString())).toBe(d.getTime())
    expect(timeValue(d)).toBe(d.getTime())
  })

  it('空值与坏值给 NaN，由调用方决定显示什么', () => {
    for (const junk of [null, undefined, '', '   ', '不是时间', {}, [], Number.NaN, new Date('x')]) {
      expect(Number.isNaN(timeValue(junk)), JSON.stringify(junk)).toBe(true)
    }
  })
})

describe('formatDateTime：表格口径 YYYY-MM-DD HH:mm', () => {
  it('ISO 串与毫秒数出同一个本地戳', () => {
    const d = at(3 * HOUR)
    const want = localStamp(d)
    expect(formatDateTime(d.toISOString())).toBe(want)
    expect(formatDateTime(d.getTime())).toBe(want)
  })

  it('个位数月/日/时/分补零，不补就会漂出 "2026-9-2 7:05" 这种排不齐的列', () => {
    const padded = formatDateTime(new Date(2026, 8, 2, 7, 5))
    expect(padded).toBe('2026-09-02 07:05')
    expect(padded).not.toMatch(/(^|[^\d-])9-2/)
  })

  it('空值与坏值都走缺省档，绝不印 Invalid Date', () => {
    expect(formatDateTime('')).toBe('-')
    expect(formatDateTime(null)).toBe('-')
    expect(formatDateTime('乱码')).toBe('-')
    expect(formatDateTime('乱码', '（无）')).toBe('（无）')
  })

  it('0 按"没有值"处理（合并掉的四份旧副本全都写 if (!ts)，这里不改契约）', () => {
    expect(formatDateTime(0)).toBe('-')
    expect(relativeTimeText(0)).toBe('')
    expect(timeAgoOrDate(0)).toBe('')
    expect(timeValue(0)).toBe(0)
  })
})

describe('relativeTimeText：本轮修的正是"相对档对 RFC3339 字符串永不触发"', () => {
  it('回归针：ISO 字符串的 5 分钟前要说得出"5 分钟前"', () => {
    expect(relativeTimeText(isoAt(5 * MINUTE))).toBe('5 分钟前')
  })

  it('刚刚 / N 小时前两档各自就位', () => {
    expect(relativeTimeText(isoAt(20 * 1000))).toBe('刚刚')
    expect(relativeTimeText(isoAt(2 * HOUR + 30 * MINUTE))).toBe('2 小时前')
  })

  it('整点边界归下一档：正好 60 分钟是"1 小时前"，不是"59 分钟前"', () => {
    const now = 1758000000000
    expect(relativeTimeText(now - 59 * MINUTE, now)).toBe('59 分钟前')
    expect(relativeTimeText(now - HOUR, now)).toBe('1 小时前')
  })

  it('未来的时间说"稍后"，不许说"刚刚"（负数 diff 会一路落到 <60000 那档）', () => {
    expect(relativeTimeText(isoAt(-10 * MINUTE))).toBe('稍后')
  })

  it('超过一天就闭嘴，把显示交回绝对档', () => {
    expect(relativeTimeText(isoAt(DAY + MINUTE))).toBe('')
    expect(relativeTimeText(isoAt(30 * DAY))).toBe('')
  })

  it('坏值与空值说不出话（返回空串而不是  NaN 串）', () => {
    expect(relativeTimeText('')).toBe('')
    expect(relativeTimeText('乱码')).toBe('')
  })
})

describe('timeAgoOrDate：两档成对用，单独用必掉档', () => {
  it('一天以内给相对档', () => {
    expect(timeAgoOrDate(isoAt(5 * MINUTE))).toBe('5 分钟前')
  })

  it('超一天回到绝对档，不再只给一个没有年份的 M/D', () => {
    const d = at(3 * DAY)
    expect(timeAgoOrDate(d.toISOString())).toBe(localStamp(d))
  })

  it('空值默认给空串（列表行里宁可不占位），要占位自己传', () => {
    expect(timeAgoOrDate('')).toBe('')
    expect(timeAgoOrDate('', Date.now(), '-')).toBe('-')
  })
})

describe('barrel 出口：旧的"两份同体副本靠别名并存"必须消失', () => {
  it('四个新出口都在 @/utils 上', () => {
    for (const name of ['timeValue', 'formatDateTime', 'relativeTimeText', 'timeAgoOrDate']) {
      expect(typeof utils[name], name).toBe('function')
    }
  })

  it('formatTime / kbFormatTime 不再从 barrel 导出（它们曾是逐字符相同的两份）', () => {
    expect(Object.keys(utils)).not.toContain('formatTime')
    expect(Object.keys(utils)).not.toContain('kbFormatTime')
  })
})

// ---- 台账：全库自写时间副本 ----

const COPY_RE = /^\s*(export\s+)?(async\s+)?(function\s+(formatTime|relativeTime)\b|const\s+(formatTime|relativeTime)\s*=(?!=))/
const DEF_FILES = ['.js', '.ts', '.vue']

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const p = path.join(dir, name)
    if (name === 'node_modules' || name === 'dist') continue
    if (statSync(p).isDirectory()) walk(p, out)
    else if (DEF_FILES.includes(path.extname(name))) out.push(p)
  }
  return out
}

function countCopies(text) {
  let n = 0
  for (const line of text.split('\n')) {
    const t = line.trim()
    if (t.startsWith('//') || t.startsWith('*') || t.startsWith('/*')) continue
    if (COPY_RE.test(line)) n += 1
  }
  return n
}

function scanCopies() {
  const hits = {}
  for (const file of walk(SRC_DIR)) {
    if (file.endsWith('.test.js') || file.endsWith('.test.ts')) continue
    const n = countCopies(readFileSync(file, 'utf8'))
    if (n) hits[path.relative(SRC_DIR, file).split(path.sep).join('/')] = n
  }
  return hits
}

// 现存副本：display.js 剩下的一份是 formatTime（坏值原样回显的契约，出口给不了，见 §5.36）；
// 它的 relativeTime 副本本轮已实测 13/14 输入形状与出口逐字相同并委托掉。其余每一处都是待收口的副本。
// AdminDepartment/AdminRole/AdminUser/ExpertOrchestratorPanel 四处本轮收成 `import { … as formatTime }`：
// 它们本就在调 formatDateTimeLocale，只是先用 new Date(t) 转一道，于是数字串形状退化成 'Invalid Date'。
const KNOWN_COPIES = {
  'components/MessageBubble.vue': 1,
  'modules/expert-alliance/components/ExpertBookingPanel.vue': 1,
  'modules/expert-alliance/model/display.js': 1,
  'views/admin/panels/AdminLlm.vue': 1,
  'views/admin/panels/AdminSso.vue': 1,
  'views/ai/ChatView.vue': 1,
  'views/expert/panels/ExpertEnterprisePanel.vue': 1,
  'views/workflow/BrowserView.vue': 1
}

const CLEARED_ALIAS = [
  'views/admin/panels/AdminDepartment.vue',
  'views/admin/panels/AdminRole.vue',
  'views/admin/panels/AdminUser.vue',
  'views/expert/panels/ExpertOrchestratorPanel.vue'
]

const CLEARED_DELEGATED = [
  'views/project/panels/KnowledgeBasePanel.vue',
  'views/workspace/panels/KnowledgeBasePanel.vue'
]
// 这两处是把副本整个删掉的（它们本身就在 utils/ 里，再 import barrel 就是环），所以只钉"不许抄回来"
const CLEARED_DROPPED = ['utils/knowledgeBase.utils.js', 'utils/message.utils.js']

describe('自写时间副本台账（新增即红，清零须删条目）', () => {
  it('扫描集非空：零命中必须来自"扫到了且没有"，不是来自"什么都没扫"', () => {
    // 实测本目录 .js/.ts/.vue 共 322 个（node_modules 与 dist 不计），下限留一点余量给删除
    expect(walk(SRC_DIR).length).toBeGreaterThanOrEqual(300)
  })

  it('逐文件计数与台账相等', () => {
    const actual = scanCopies()
    expect(Object.keys(actual).sort()).toEqual(Object.keys(KNOWN_COPIES).sort())
    for (const rel of Object.keys(KNOWN_COPIES)) {
      expect(actual[rel], rel).toBe(KNOWN_COPIES[rel])
    }
  })

  it('已收口的四处不再自带定义；两处委托的确实从 barrel 取', () => {
    for (const rel of [...CLEARED_DELEGATED, ...CLEARED_DROPPED]) {
      const text = readFileSync(path.join(SRC_DIR, rel), 'utf8')
      expect(countCopies(text), `${rel} 又抄回一份`).toBe(0)
    }
    for (const rel of CLEARED_DELEGATED) {
      expect(readFileSync(path.join(SRC_DIR, rel), 'utf8'), `${rel} 没接线`).toContain("from '@/utils'")
    }
    for (const rel of CLEARED_DROPPED) {
      expect(readFileSync(path.join(SRC_DIR, rel), 'utf8'), `${rel} 又导出同名`).not.toMatch(/export\s+(function|const)\s+formatTime\b/)
    }
  })

  it('正对照：定义形态能被点名（证明台账不是空转）', () => {
    const planted = [
      'function formatTime(ts) {',
      'export function relativeTime(iso, now = Date.now()) {',
      'const formatTime = (ts) => ts',
      'export const relativeTime = function (x) { return x }'
    ]
    planted.forEach((line, i) => {
      expect(countCopies(line), `第 ${i + 1} 行未被点名: ${line}`).toBe(1)
    })
  })

  it('反对照：别名导入、调用点与注释不得被点名', () => {
    const legal = [
      "import { formatDateTime as formatTime } from '@/utils'",
      '  const t = formatTime(doc.updated_at)',
      '// function formatTime 旧副本已删',
      '  const formatTimezone = ref(0)',
      '  relativeTimezone: 1',
      '  return formatTimeFn(x)'
    ]
    legal.forEach((line, i) => {
      expect(countCopies(line), `第 ${i + 1} 行被误伤: ${line}`).toBe(0)
    })
  })
})

// ---- F5：模块内自写的"相对时间"副本委托给出口（治理文档 §5.36）----

const DISPLAY_REL = 'modules/expert-alliance/model/display.js'

describe('F5 相对档委托：页面侧只留一份实现', () => {
  const display = readFileSync(path.join(SRC_DIR, DISPLAY_REL), 'utf8')

  it('display.js 不再自写 relativeTime，相对档经 barrel 从出口取', () => {
    expect(countCopies(display), `${DISPLAY_REL} 又抄回一份时间定义`).toBe(KNOWN_COPIES[DISPLAY_REL])
    expect(display).not.toMatch(/function\s+relativeTime\s*\(/)
    expect(display).toMatch(/import\s*\{[^}]*\brelativeTimeText\b[^}]*\}\s*from\s*['"]@\/utils['"]/)
    expect(display).not.toMatch(/from\s*['"]@\/utils\/time['"]/)
    expect(display).toMatch(/return\s+relativeTimeText\(\s*iso\s*\)\s*\|\|/)
  })

  it('出口的四个相对档逐字就是页面那份副本的字形（带空格那一档也算）', () => {
    expect(relativeTimeText(at(7 * MINUTE))).toBe('7 分钟前')
    expect(relativeTimeText(at(5 * HOUR))).toBe('5 小时前')
    expect(relativeTimeText(at(-30 * 1000))).toBe('稍后')
    expect(relativeTimeText(at(DAY))).toBe('')
  })

  it('唯一异形已定价：epoch 毫秒数字串在旧副本上恒空转，出口读得出', () => {
    const past = String(Date.now() - 9 * MINUTE)
    expect(Number.isNaN(new Date(past).getTime())).toBe(true)
    expect(relativeTimeText(past)).toBe('9 分钟前')
  })

  it('模块侧保留的 formatTime 是"坏值原样回显"档，出口给不了它', () => {
    expect(display).toMatch(/Number\.isNaN\(d\.getTime\(\)\)\s*\?\s*raw\s*:/)
    expect(relativeTimeText('not-a-date')).toBe('')
  })

  it('四处别名 import 收口：本地定义没了，formatTime 这个名字仍从 barrel 来', () => {
    for (const rel of CLEARED_ALIAS) {
      const text = readFileSync(path.join(SRC_DIR, rel), 'utf8')
      expect(countCopies(text), `${rel} 又抄回一份`).toBe(0)
      expect(text, `${rel} 别名没接线`).toMatch(/import\s*\{[^}]*formatDateTimeLocaleOr\s+as\s+formatTime[^}]*\}\s*from\s*['"]@\/utils['"]/)
      expect(text, `${rel} 走了深路径`).not.toMatch(/from\s*['"]@\/utils\/time['"]/)
    }
  })

  it('别名收口的版面账只有epoch 毫秒数字串一形变好，其余逐字同', () => {
    const past = String(Date.now() - 9 * MINUTE)
    expect(formatDateTimeLocaleOr(new Date(past))).toBe('Invalid Date')
    expect(formatDateTimeLocaleOr(past)).not.toBe('Invalid Date')
    expect(formatDateTimeLocaleOr(past)).toMatch(/^\d{4}\/\d{1,2}\/\d{1,2}/)
    expect(formatDateTimeLocaleOr('')).toBe('-')
    expect(formatDateTimeLocaleOr('not-a-date')).toBe('Invalid Date')
  })
})
