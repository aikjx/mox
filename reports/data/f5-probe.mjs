// F5 等价性现测（只读）：display.js 的 relativeTime 与出口 relativeTimeText 是否同字。
// 旧写法从磁盘现推（正则找回整段函数体 → new Function），不抄我自己的记忆。
import fs from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'

const SRC = 'D:/a10/aikjx/gitcode/infotopograph/frontend-ui/src'
const { timeValue, relativeTimeText } = await import(pathToFileURL(path.resolve(SRC, 'utils/time.js')).href)

// 从磁盘取 relativeTime 的函数体（截到匹配的右括号），就地造出同一份实现
const src = fs.readFileSync(path.resolve(SRC, 'modules/expert-alliance/model/display.js'), 'utf8')
const m = src.match(/export function relativeTime\(iso, now = Date\.now\(\)\) \{([\s\S]*?)\n\}/)
if (!m) { console.log('INVALID: 没在磁盘上找到 relativeTime 的定义'); process.exit(1) }
const body = m[1]
const str = (v) => (v === undefined || v === null ? '' : String(v))
// eslint-disable-next-line no-new-func
const diskRelativeTime = new Function('iso', 'now', 'str', body)
console.log(`已从磁盘取到函数体，${body.split('\n').length} 行，分支串逐字打印：`)
for (const ln of body.split('\n').map((l) => l.trim()).filter(Boolean)) console.log('   ' + ln)

const MIN = 60000, HOUR = 60 * MIN, DAY = 24 * HOUR
const NOW = Date.UTC(2026, 8, 28, 12, 0, 0)
const SHAPES = [
  ['RFC3339 刚刚', new Date(NOW - 5000).toISOString()],
  ['RFC3339 N分钟前', new Date(NOW - 7 * MIN).toISOString()],
  ['RFC3339 N小时前', new Date(NOW - 5 * HOUR).toISOString()],
  ['RFC3339 超一天', new Date(NOW - 30 * DAY).toISOString()],
  ['RFC3339 未来', new Date(NOW + 3 * MIN).toISOString()],
  ['epoch 毫秒数字串', String(NOW - 9 * MIN)],
  ['Date 对象', new Date(NOW - 4 * MIN)],
  ['带 Z 的秒级串', '2026-09-28T12:00:00Z'],
  ['无时区串', '2026-09-28T12:00:00'],
  ['坏串', 'not-a-date'],
  ['空串', ''],
  ['null', null],
  ['undefined', undefined],
  ['数字 0', 0],
]

let diff = 0, same = 0
for (const [label, v] of SHAPES) {
  const a = diskRelativeTime(v, NOW, str)
  const b = relativeTimeText(v, NOW)
  const tag = a === b ? 'EQ ' : 'NEQ'
  if (a === b) same++; else diff++
  console.log(`${tag} ${label.padEnd(18)} 盘上=${JSON.stringify(a)} 出口=${JSON.stringify(b)}`)
}
console.log(`判决：${same} 同字 / ${diff} 异字（异字必须逐条定价，不许当噪声）`)
