/**
 * 项目成员归一化 —— 成员列表的唯一解码口径
 *
 * /projects/:id/members 在不同部署下可能返回数组，也可能返回 { members: [...] } 包壳；
 * 单条记录的字段名同样有多种写法（user_id / username / name）。
 * 这些写法此前各抄一份（工作台一份、项目页一份，其中一份漏掉了 members 包壳，
 * 另一份的角色三元表达式恒不成立），故收敛到此。
 */

// 成员角色 → 中文文案（后端只给 owner/admin/editor/viewer/host/expert 这类码值）
export const MEMBER_ROLE_TEXT = {
  owner: '负责人',
  host: '主持人',
  admin: '管理员',
  editor: '编辑者',
  viewer: '查看者',
  expert: '专家',
  member: '成员'
}

// 后端未给头像底色时的兜底，与 avatarColor 的取值口径无关，仅用于渐变占位
const MEMBER_AVATAR_FALLBACK = 'linear-gradient(135deg, #6366f1, #06b6d4)'

export function memberRoleText(role) {
  return MEMBER_ROLE_TEXT[role] || role || '成员'
}

// raw 可以是数组、{ members } / { items } / { data } 包壳，或 null
export function normalizeProjectMembers(raw) {
  let list = raw
  if (!Array.isArray(list) && list && typeof list === 'object') {
    list = list.members || list.items || list.data || []
  }
  if (!Array.isArray(list)) return []
  return list.map((m, i) => {
    const name = m?.name || m?.username || m?.user_id || `成员 ${i + 1}`
    return {
      id: m?.id || m?.user_id || `member-${i}`,
      name,
      // avatar 是头像格子里显示的字/图，消费方（工作台成员栈、项目页成员卡）都按此名取
      avatar: m?.avatar || String(name).charAt(0),
      color: m?.color || MEMBER_AVATAR_FALLBACK,
      status: m?.status || 'active',
      role: m?.role || 'expert',
      roleText: memberRoleText(m?.role),
      joinedAt: m?.joined_at || m?.created_at || ''
    }
  })
}

// member_count 是列表/详情里的真实计数；没有它时只能退到已取到的成员条数
export function projectMemberCount(p) {
  if (p && typeof p.member_count === 'number') return p.member_count
  if (p && typeof p.memberCount === 'number') return p.memberCount
  if (p && Array.isArray(p.members)) return p.members.length
  return 0
}
