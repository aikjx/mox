import { describe, expect, it } from 'vitest'
import {
  memberRoleText,
  normalizeProjectMembers,
  projectMemberCount,
} from './projectMember.utils.js'

// 成员解码口径的唯一实现。这里每一条断言都对应一个真实踩过的形状：
// 网关 GET /projects/:id/members 返回的是 { members: [] } 包壳而不是裸数组，
// 之前工作台那份实现只认裸数组，后端一旦给出成员也会被静默丢空。
describe('项目成员归一化', () => {
  it('包壳与裸数组两种返回都能解码', () => {
    expect(normalizeProjectMembers({ members: [{ user_id: 'u1' }] })).toHaveLength(1)
    expect(normalizeProjectMembers([{ user_id: 'u1' }])).toHaveLength(1)
    expect(normalizeProjectMembers({ items: [{ user_id: 'u1' }, { user_id: 'u2' }] })).toHaveLength(2)
  })

  it('非数组且无包壳字段时解码为空，而不是抛错', () => {
    expect(normalizeProjectMembers(null)).toEqual([])
    expect(normalizeProjectMembers(undefined)).toEqual([])
    expect(normalizeProjectMembers('boom')).toEqual([])
    expect(normalizeProjectMembers({ total: 0 })).toEqual([])
  })

  it('字段名三写法取一：name > username > user_id，头像取名字首字', () => {
    const [a] = normalizeProjectMembers([{ name: '张三' }])
    const [b] = normalizeProjectMembers([{ username: 'lisi' }])
    const [c] = normalizeProjectMembers([{ user_id: 'wangwu' }])
    expect([a.name, b.name, c.name]).toEqual(['张三', 'lisi', 'wangwu'])
    expect(a.avatar).toBe('张')
    expect(c.avatar).toBe('w')
  })

  it('后端显式给了 avatar 与 color 时不被本地默认覆盖', () => {
    const [m] = normalizeProjectMembers([{ user_id: 'u1', avatar: '🤖', color: 'var(--cat-1)' }])
    expect(m.avatar).toBe('🤖')
    expect(m.color).toBe('var(--cat-1)')
  })

  it('缺字段的兜底：id 用序号、status 用 active、role 用 expert', () => {
    const [m] = normalizeProjectMembers([{ name: '只有名字' }])
    expect(m.id).toBe('member-0')
    expect(m.status).toBe('active')
    expect(m.role).toBe('expert')
    expect(normalizeProjectMembers([{}])[0].name).toBe('成员 1')
  })

  it('id 优先取后端给的 id，而不是 user_id', () => {
    expect(normalizeProjectMembers([{ id: 'x', user_id: 'u' }])[0].id).toBe('x')
  })

  it('角色码值翻成中文，未知码值原样透出', () => {
    expect(memberRoleText('owner')).toBe('负责人')
    expect(memberRoleText('viewer')).toBe('查看者')
    expect(memberRoleText('robot')).toBe('robot')
    expect(memberRoleText(undefined)).toBe('成员')
    const [m] = normalizeProjectMembers([{ user_id: 'u1', role: 'host' }])
    expect(m.roleText).toBe('主持人')
  })

  it('joinedAt 兼容 joined_at 与 created_at 两种写法', () => {
    expect(normalizeProjectMembers([{ user_id: 'u1', joined_at: '2026-09-01' }])[0].joinedAt).toBe('2026-09-01')
    expect(normalizeProjectMembers([{ user_id: 'u1', created_at: '2026-09-02' }])[0].joinedAt).toBe('2026-09-02')
    expect(normalizeProjectMembers([{ user_id: 'u1' }])[0].joinedAt).toBe('')
  })
})

describe('项目成员计数', () => {
  it('member_count 优先于已取到的成员条数', () => {
    expect(projectMemberCount({ member_count: 7, members: [1, 2] })).toBe(7)
  })

  it('member_count 为 0 时就是 0，不退到 members 长度', () => {
    expect(projectMemberCount({ member_count: 0, members: [1, 2] })).toBe(0)
  })

  it('没有 member_count 才退回成员数组长度', () => {
    expect(projectMemberCount({ members: [1, 2, 3] })).toBe(3)
    expect(projectMemberCount({})).toBe(0)
    expect(projectMemberCount(null)).toBe(0)
  })

  it('驼峰 memberCount 同样可读（后端两种序列化）', () => {
    expect(projectMemberCount({ memberCount: 5 })).toBe(5)
  })
})
