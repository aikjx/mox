// 榜单口径回归：只允许后端 ExpertMetrics/created_at 参与排名，
// 缺真实数据的专家必须被排除而不是补零凑数（存量页正是靠补零造出了假榜单）。
import { describe, it, expect } from 'vitest'
import { RANK_BOARDS, RANK_BOARD, RANK_LIMIT, rankBoardMeta, buildBoard } from './rank.js'

const expert = (over = {}) => ({
  id: 'e1',
  name: '林架构',
  title: '首席架构师',
  organization: '璇玑科技',
  online: true,
  createdAt: '2026-01-05T08:00:00Z',
  metrics: {
    totalConsultations: 180,
    todayConsultations: 3,
    avgRating: 4.7,
    ratingCount: 56,
    resolutionRate: 0.93,
    firstResponseAccuracy: 0.88,
    totalServiceMinutes: 5400
  },
  ...over
})

const metrics = (over) => ({ ...expert().metrics, ...over })

describe('榜单定义', () => {
  it('三张榜单各有 key/label/valueLabel/rule，且规则都点名后端字段', () => {
    expect(RANK_BOARDS.map((b) => b.key)).toEqual(['consultations', 'rating', 'newcomers'])
    for (const b of RANK_BOARDS) {
      expect(b.label.length).toBeGreaterThan(1)
      expect(b.valueLabel.length).toBeGreaterThan(1)
      expect(b.rule).toMatch(/metrics\.|created_at/)
    }
    expect(rankBoardMeta('nope')).toBeNull()
  })
})

describe('咨询量榜', () => {
  it('按累计咨询降序，同分按名称定序', () => {
    const list = [
      expert({ id: 'a', name: '赵热', metrics: metrics({ totalConsultations: 90 }) }),
      expert({ id: 'b', name: '钱稳', metrics: metrics({ totalConsultations: 300 }) }),
      expert({ id: 'c', name: '孙平', metrics: metrics({ totalConsultations: 90 }) })
    ]
    const { rows } = buildBoard(RANK_BOARD.CONSULTATIONS, list)
    expect(rows.map((r) => r.id)).toEqual(['b', 'c', 'a'])
    expect(rows.map((r) => r.rank)).toEqual([1, 2, 3])
    expect(rows[0].display).toBe('300 次')
    expect(rows[1].secondary).toBe('今日 3 次')
    expect(rows[0].subtitle).toBe('首席架构师')
  })

  it('累计咨询为 0 表示没有成交样本，不上榜', () => {
    const { rows, eligible, sampleSize } = buildBoard(RANK_BOARD.CONSULTATIONS, [
      expert({ id: 'a' }),
      expert({ id: 'b', metrics: metrics({ totalConsultations: 0 }) }),
      expert({ id: 'c', metrics: metrics({ totalConsultations: null }) })
    ])
    expect(rows.map((r) => r.id)).toEqual(['a'])
    expect(eligible).toBe(1)
    expect(sampleSize).toBe(3)
  })
})

describe('评分榜', () => {
  it('评分要有评分人数才算数，缺人数的默认零值被排除', () => {
    const list = [
      expert({ id: 'a', name: '甲', metrics: metrics({ avgRating: 4.9, ratingCount: 0 }) }),
      expert({ id: 'b', name: '乙', metrics: metrics({ avgRating: 4.25, ratingCount: 8 }) }),
      expert({ id: 'c', name: '丙', metrics: metrics({ avgRating: 4.9, ratingCount: 120 }) })
    ]
    const { rows } = buildBoard(RANK_BOARD.RATING, list)
    expect(rows.map((r) => r.id)).toEqual(['c', 'b'])
    expect(rows[0].display).toBe('4.9')
    expect(rows[1].display).toBe('4.3')
    expect(rows[0].secondary).toBe('120 人评')
  })

  it('全员无评分时榜单为空，交由界面说明原因', () => {
    const { rows, eligible } = buildBoard(RANK_BOARD.RATING, [
      expert({ metrics: metrics({ avgRating: 0, ratingCount: 0 }) })
    ])
    expect(rows).toEqual([])
    expect(eligible).toBe(0)
  })
})

describe('新晋榜', () => {
  it('按注册时间倒序，时间缺失或非法的不参与', () => {
    const list = [
      expert({ id: 'old', createdAt: '2025-03-01T00:00:00Z' }),
      expert({ id: 'new', createdAt: '2026-09-02T00:00:00Z' }),
      expert({ id: 'blank', createdAt: '' }),
      expert({ id: 'junk', createdAt: 'not-a-date' })
    ]
    const { rows, eligible } = buildBoard(RANK_BOARD.NEWCOMERS, list)
    expect(rows.map((r) => r.id)).toEqual(['new', 'old'])
    expect(eligible).toBe(2)
    expect(rows[1].secondary).toBe('2025-03-01 00:00:00')
  })
})

describe('榜单边界', () => {
  it('默认截取前 10 名，排名号连续', () => {
    const list = Array.from({ length: 13 }, (_, i) => ({
      ...expert({ id: `e${i}`, name: `专家${i}` }),
      metrics: metrics({ totalConsultations: i + 1 })
    }))
    const { rows, eligible } = buildBoard(RANK_BOARD.CONSULTATIONS, list)
    expect(rows).toHaveLength(RANK_LIMIT)
    expect(rows[0].id).toBe('e12')
    expect(rows[rows.length - 1].rank).toBe(RANK_LIMIT)
    expect(eligible).toBe(13)
  })

  it('未知榜单、非数组入参与空列表都不抛错', () => {
    expect(buildBoard('nope', [expert()]).rows).toEqual([])
    expect(buildBoard(RANK_BOARD.RATING, undefined)).toMatchObject({ rows: [], sampleSize: 0 })
    expect(buildBoard(RANK_BOARD.RATING, []).eligible).toBe(0)
  })

  it('不改动入参，也不新增输入里没有的字段', () => {
    const source = [expert()]
    const snapshot = JSON.parse(JSON.stringify(source))
    const { rows } = buildBoard(RANK_BOARD.CONSULTATIONS, source)
    expect(source).toEqual(snapshot)
    expect(Object.keys(rows[0]).sort()).toEqual(
      ['display', 'id', 'name', 'online', 'rank', 'secondary', 'subtitle', 'value'].sort()
    )
  })

  it('无名专家不参与排名，界面没有可指向的实体', () => {
    expect(buildBoard(RANK_BOARD.CONSULTATIONS, [expert({ name: '' })]).rows).toEqual([])
  })
})
