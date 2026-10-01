// 专家榜单：口径只认后端 ExpertMetrics 真实返回的字段。
// 权威源 platform/gateway/mox-platform-gateway-svc/src/alliance/experts_common.rs::ExpertMetrics，
// 字段名由 contract/contract.test.js 锁定；后端删字段时那边先红，这里不会悄悄造假。

import { formatDateStamp } from '@/utils'
import { expertDisplayName } from '@/modules/expert-alliance/contract'

export const RANK_BOARD = Object.freeze({
  CONSULTATIONS: 'consultations',
  RATING: 'rating',
  NEWCOMERS: 'newcomers'
})

export const RANK_BOARDS = Object.freeze([
  Object.freeze({
    key: RANK_BOARD.CONSULTATIONS,
    label: '咨询量榜',
    valueLabel: '累计咨询',
    rule: 'metrics.total_consultations 降序；次数为 0 表示后端没有成交样本，不上榜。'
  }),
  Object.freeze({
    key: RANK_BOARD.RATING,
    label: '评分榜',
    valueLabel: '平均评分',
    rule: 'metrics.avg_rating 降序；rating_count 为 0 时评分是默认零值而非真实口碑，不上榜。'
  }),
  Object.freeze({
    key: RANK_BOARD.NEWCOMERS,
    label: '新晋榜',
    valueLabel: '注册时间',
    rule: 'created_at 倒序；注册时间为空（后端 #[serde(default)] 给空串）的不上榜。'
  })
])

export const RANK_LIMIT = 10

export function rankBoardMeta(key) {
  return RANK_BOARDS.find((b) => b.key === key) || null
}

/**
 * 把一位专家折算成榜单 datum；不可排名时返回 null，由调用方过滤而不是补零。
 * 这里出现的每个字段都必须存在于 normExpert 的输出，即后端真实回传的字段。
 */
function datumOf(expert, key) {
  if (!expert || !expert.name) return null
  const metrics = expert.metrics || {}
  const number = (v) => (Number.isFinite(Number(v)) ? Number(v) : null)

  if (key === RANK_BOARD.CONSULTATIONS) {
    const total = number(metrics.totalConsultations)
    if (!total || total <= 0) return null
    return { value: total, display: `${total} 次`, secondary: `今日 ${number(metrics.todayConsultations) ?? 0} 次` }
  }

  if (key === RANK_BOARD.RATING) {
    const rating = number(metrics.avgRating)
    const count = number(metrics.ratingCount)
    if (rating === null || !count || count <= 0) return null
    return { value: rating, display: rating.toFixed(1), secondary: `${count} 人评` }
  }

  if (key === RANK_BOARD.NEWCOMERS) {
    const raw = String(expert.createdAt || '')
    const stamp = Date.parse(raw)
    if (!raw || Number.isNaN(stamp)) return null
    return { value: stamp, display: formatDateStamp(stamp), secondary: raw.slice(0, 19).replace('T', ' ') }
  }

  return null
}

/**
 * 纯函数榜单：先按指标降序，同分再按名称（zh-CN）稳定排序，最后截取前 limit 名。
 * @returns {{rows: Array, board: object, sampleSize: number, eligible: number}}
 */
export function buildBoard(key, experts, limit = RANK_LIMIT) {
  const board = rankBoardMeta(key)
  const list = Array.isArray(experts) ? experts : []
  if (!board) return { rows: [], board: null, sampleSize: list.length, eligible: 0 }

  const scored = []
  for (const expert of list) {
    const datum = datumOf(expert, key)
    if (!datum) continue
    scored.push({
      id: expert.id,
      name: expertDisplayName(expert),
      subtitle: expert.title || expert.organization || '—',
      online: !!expert.online,
      ...datum
    })
  }

  scored.sort((a, b) => (
    b.value - a.value || a.name.localeCompare(b.name, 'zh-CN')
  ))

  return {
    rows: scored.slice(0, Math.max(0, limit)).map((row, i) => ({ ...row, rank: i + 1 })),
    board,
    sampleSize: list.length,
    eligible: scored.length
  }
}
