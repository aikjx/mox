// 时间显示口径的单源。
// 之所以要有这个文件：本轮之前全库在 16 个文件里自写了 17 处 formatTime/relativeTime（display.js 一个文件两处），
// 分四族，其中两族各有三份逐字符相同（实测：A 族三份函数体 sha256 同为 bc1b7a3afd18a278，B 族同为 791ebd8c5100169d）。
// 最要紧的一条不是重复，而是**输入口径**：后端把时间写成 RFC3339 字符串
// （见 platform/domains/kg/svc/mox-kb-svc/src/model.rs 的 now_iso()，chrono to_rfc3339），
// 而副本们拿 Date.now() 去减这个字符串 ⇒ NaN ⇒ 所有 `diff < X` 判据恒假 ⇒ 相对档一次都不会触发，
// 界面悄悄退化成"只有 M/D"。timeValue 就是为了让任何形状都能先落成毫秒数再判。

const MINUTE = 60 * 1000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

const pad2 = (n) => String(n).padStart(2, '0')

/** 任何形状的时间输入 → 毫秒数；拿不到就返回 NaN（调用方决定显示什么，这里不编值） */
export function timeValue(ts) {
  if (ts === null || ts === undefined || ts === '') return NaN
  if (ts instanceof Date) return Number.isNaN(ts.getTime()) ? NaN : ts.getTime()
  if (typeof ts === 'number') return Number.isFinite(ts) ? ts : NaN
  const raw = String(ts).trim()
  if (!raw) return NaN
  // 后端偶发把 epoch ms 写成数字串；new Date('1758000000000') 在引擎里是 Invalid Date，必须先转数
  if (/^\d+$/.test(raw)) return Number(raw)
  const t = Date.parse(raw)
  return Number.isFinite(t) ? t : NaN
}

/** 表格口径：YYYY-MM-DD HH:mm（本地时区）。坏值一律显示 empty，不印 Invalid Date。
 *  0 与负数按"没有值"处理：本轮合并掉的四份旧副本全都写 `if (!ts) return '-'`，这里不改契约。 */
export function formatDateTime(ts, empty = '-') {
  const t = timeValue(ts)
  if (!(t > 0)) return empty
  const d = new Date(t)
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())} ${pad2(d.getHours())}:${pad2(d.getMinutes())}`
}

/**
 * 相对档：稍后 / 刚刚 / N 分钟前 / N 小时前；超过一天返回空串，由调用方回到绝对档。
 * 档位与联盟模块 model/display.js 的 relativeTime 一致——这里不裁"超过一天显示什么"，
 * 只显示"这一档说不出话"，让 timeAgoOrDate 去补绝对档。
 */
export function relativeTimeText(ts, now = Date.now()) {
  const t = timeValue(ts)
  if (!(t > 0)) return ''
  const diff = now - t
  if (diff < 0) return '稍后'
  if (diff < MINUTE) return '刚刚'
  if (diff < HOUR) return `${Math.floor(diff / MINUTE)} 分钟前`
  if (diff < DAY) return `${Math.floor(diff / HOUR)} 小时前`
  return ''
}

/** 列表行的一把时间：一天以内走相对档，超出回到绝对档（两档成对用，单独用会掉档） */
export function timeAgoOrDate(ts, now = Date.now(), empty = '') {
  return relativeTimeText(ts, now) || formatDateTime(ts, empty)
}

/**
 * 完整时刻档：固定 `zh-CN` ＋ 24 小时制（本轮收口的六份副本逐字符都是这一形）。
 * 为什么要有这一枚而不复用 formatDateTime：两档的版面不同（`YYYY-MM-DD HH:mm` vs
 * locale 的 `2026/9/28 01:38:00`），各面板已按后者排版，合并会改用户看见的字。
 * 坏值返回 null 而不是 "Invalid Date"：印什么由调用方决定（各家空值占位是 '-'／''／'—'／原样回显，
 * 这四种契约留在调用方，不在这里统一——统一的只有"能解析时怎么印"）。
 */
export function formatDateTimeLocale(ts) {
  const t = timeValue(ts)
  if (!(t > 0)) return null
  return new Date(t).toLocaleString('zh-CN', { hour12: false })
}

/**
 * 完整时刻档 + 双重空态（无值 → 占位符，解析失败 → `Invalid Date`）。
 * 这 4 份 `fmtTime` 副本（AdminAccess／AdminAudit／AdminConfig／AdminMenu）逐字符相同：
 * 块体 sha256 前 12 位 4117530d3121 × 4。它们的 `catch { return String(t) }` 是死支——
 * 坏串走 `new Date(...)` 再做 locale 格式化不会抛，而是返回字符串 "Invalid Date"，所以界面上真实出现过的
 * 契约就是"坏值印 Invalid Date"，这里原样保留可见行为（不改契约，只去重复）。
 * 与上面一档的差别只有默认语言：这一族原先写无参 `toLocaleString()`＝跟随浏览器区域设置，
 * 归一到固定 `zh-CN` 是本轮唯一有意的显示口径变更（zh-CN 客户端输出逐字符不变）。
 */
export function formatDateTimeLocaleOr(ts, empty = '-') {
  if (!ts) return empty
  return formatDateTimeLocale(ts) ?? 'Invalid Date'
}

const CLOCK_OPTS = { hour: '2-digit', minute: '2-digit' }

/**
 * 时钟档 `HH:mm`（固定 zh-CN，不带秒）。这一族全库 7 处副本逐字符相同
 * （useAlliance.js ×1／alliance.store.js ×3／Workbench.vue ×1／ExpertWorkspaceView.vue ×2，
 * 实测见 reports/data/locale-family-census.txt），且原本就写死了 `zh-CN`，
 * 所以本档收口**不改任何一个用户看见的字**——纯粹去重复。
 * 与 formatDateTimeLocaleOr 的分工：那档印完整时刻，这档只印到分钟。
 */
export function formatClockMinute(ts = Date.now(), invalid = 'Invalid Date') {
  const t = timeValue(ts)
  if (!(t > 0)) return invalid
  return new Date(t).toLocaleTimeString('zh-CN', CLOCK_OPTS)
}

/** 日期档（固定 zh-CN，不带时分秒）：rank.js ×1／Workbench.vue ×2 三份副本的单源，同样零版面变更。 */
export function formatDateStamp(ts = Date.now(), invalid = 'Invalid Date') {
  const t = timeValue(ts)
  if (!(t > 0)) return invalid
  return new Date(t).toLocaleDateString('zh-CN')
}

const CLOCK_SEC_OPTS = { hour: '2-digit', minute: '2-digit', second: '2-digit' }

/**
 * 带秒时钟档 `HH:mm:ss`。两份同档副本：`AdminMonitor.vue` 自己写了 `hour12: false`，
 * `Workbench.vue` 只写 `'zh-CN'` 没 pin——实测（三个样本，含午夜与跨 UTC 日）两种写法输出逐字符相同，
 * 所以"没 pin"当前不是版面缺陷，而是把 24 小时制押在 V8 的 locale 默认值上。收成单档后 pin 成为契约。
 */
export function formatClockSecond(ts = Date.now(), invalid = 'Invalid Date') {
  const t = timeValue(ts)
  if (!(t > 0)) return invalid
  return new Date(t).toLocaleTimeString('zh-CN', CLOCK_SEC_OPTS)
}
