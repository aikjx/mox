// 呈现层小工具：只做"把后端字符串变得可读"这一件事，不参与任何数据判定。
// 之所以收在一处：同一份 RFC3339 在线程、列表、统计三处都要显示，
// 各自写一遍 toLocaleString 会在时区与缺省文案上漂出三个版本。
import { formatDateTimeLocale, relativeTimeText } from '@/utils'
const str = (v) => (v === undefined || v === null ? '' : String(v))

/** RFC3339 → 本地可读时间；解析不出来就原样显示并加标记，绝不编一个"未知"掩盖坏数据 */
export function formatTime(iso) {
  const raw = str(iso)
  if (!raw) return '—'
  const d = new Date(raw)
  return Number.isNaN(d.getTime()) ? raw : (formatDateTimeLocale(d) ?? raw)
}

/**
 * 列表里的一行时间：一天以内走相对时间，超出回到绝对时间。
 * 两个函数必须成对使用——单独用相对档会让超过一天的会话显示为空白，
 * 单独用 formatTime 又丢掉"刚刚/N 小时前"的活跃度。取值口径不在这里（见 contract 的 sessionActivityAt）。
 * 相对档走 `@/utils` 的 `relativeTimeText`（与本页曾经的自写副本实测 13/14 输入形状逐字相同，
 * 唯一异形的 epoch 毫秒数字串是出口更对的那一侧，见治理文档 §5.36）。
 */
export function sessionTimeText(iso) {
  return relativeTimeText(iso) || formatTime(iso)
}

export function minutesText(minutes) {
  const n = Number(minutes)
  if (!Number.isFinite(n)) return '—'
  if (n < 1) return `${Math.round(n * 60)} 秒`
  if (n < 60) return `${n.toFixed(1)} 分钟`
  return `${(n / 60).toFixed(1)} 小时`
}

/** 长文本折叠：消息正文可能很长，截断必须留痕，不能悄悄裁掉 */
export function clip(text, max = 160) {
  const s = str(text)
  if (s.length <= max) return { text: s, truncated: false }
  return { text: s.slice(0, max), truncated: true }
}

/**
 * 头像配色/取图用的键：@/constants 的 16 色板按领域命名（architecture/data/ai…），
 * 与后端 expert_type（human/ai/hybrid）不是同一套词表，直接拿 type 上色只会落到兜底色。
 * 因此以首个领域为主键，缺领域时退回 expertType；第三档 `type` 只覆盖前端内置调色板表
 * （constants/expert.constants.js 的行以 `type: 'algorithm'` 这类**领域名**作键，与后端 expert_type 不同词表）。
 */
export function expertVisualKey(expert) {
  const e = expert || {}
  const domain = (Array.isArray(e.domains) ? e.domains : []).map(str).find(Boolean)
  return domain || str(e.expertType) || str(e.type)
}

/** 能否被选中协作：后端对停用专家一律 403，与在线与否无关 */
export function expertPickable(expert) {
  return !!(expert && expert.enabled !== false)
}

// 状态点/角标的样式类名沿用 workspace.css 的 dot-active / dot-idle 一系（早于可用性词表），
// 数据侧一律用规范的 availability 值，只有落到 CSS 时才翻译一次。
const STATUS_CLASS = {
  online: 'active',
  busy: 'busy',
  away: 'idle',
  offline: 'offline',
  error: 'offline'
}

/** 可用性 → 样式类后缀；未知值落到 offline，不造"看起来在线"的假象 */
export function expertStatusClass(expert) {
  return STATUS_CLASS[str(expert?.status)] || 'offline'
}
