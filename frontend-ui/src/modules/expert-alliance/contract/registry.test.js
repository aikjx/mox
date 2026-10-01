// 专家注册面契约测试：请求体形状、校验边界、合并式 patch 与软删后果。
// 每条断言的对面都是网关的一处代码事实（见 contract.test.js 的跨语言组），不是偏好。
import { describe, it, expect } from 'vitest'
import {
  EXPERT_ALIAS_FIELDS, EXPERT_PROFICIENCY_MAX, EXPERT_REGISTER_FIELDS, EXPERT_U32_MAX,
  EXPERT_UNMOUNTED_FIELDS, EXPERT_WRITE_IDENTITY, deleteConsequences, deleteResultText,
  expertDraftProblem, expertFieldProblem, expertFormDraft, expertPatch, expertProblems, registerBody
} from './registry.js'

const DRAFT = () => expertFormDraft({})

describe('registerBody', () => {
  it('只挂白名单里的键，且不带后端不收的驼峰名', () => {
    const body = registerBody({ ...DRAFT(), name: '张三' })
    expect(Object.keys(body).sort()).toEqual(['name'])
  })

  it('后端已给默认值的项留空即不发，交 minimal 决定', () => {
    const body = registerBody(DRAFT())
    expect(body).not.toHaveProperty('hourly_rate_cents')
    expect(body).not.toHaveProperty('tags')
  })

  it('与 minimal 同值的项一律不发：默认值只由后端持有', () => {
    const body = registerBody({
      ...DRAFT(),
      name: 'x',
      expertType: 'ai',
      pricingModel: 'free',
      verificationStatus: 'verified',
      timezone: 'Asia/Shanghai',
      languages: ['zh-CN', 'en'],
      hourlyRateCents: 0,
      availabilityStatus: 'online',
      maxConcurrent: 5
    })
    expect(Object.keys(body)).toEqual(['name'])
  })

  it('数组去空去重并保序', () => {
    const body = registerBody({ ...DRAFT(), name: 'x', skills: [' a ', 'a', '', 'b'] })
    expect(body.skills).toEqual(['a', 'b'])
  })

  it('能力项走对象形：保留后端原 id，新项才现造', () => {
    const body = registerBody({
      ...DRAFT(),
      name: 'x',
      capabilities: [
        { id: 'cap-old', name: '旧能力', domain: 'd', proficiency: 60, description: '' },
        { id: '', name: '新能力', domain: '', proficiency: 70, description: '' }
      ]
    })
    expect(body.capabilities).toEqual([
      { id: 'cap-old', name: '旧能力', domain: 'd', proficiency: 60, description: '' },
      { id: 'cap-新能力', name: '新能力', domain: '', proficiency: 70, description: '' }
    ])
  })

  it('availability 以嵌套对象发出', () => {
    const body = registerBody({ ...DRAFT(), name: 'x', availabilityStatus: 'busy', maxConcurrent: 3 })
    expect(body.availability).toEqual({ status: 'busy', max_concurrent: 3 })
  })

  it('不发兼容别名，也不发未挂载的运行态与绩效字段', () => {
    const body = registerBody({
      ...DRAFT(),
      name: 'x',
      type: 'custom',
      description: 'd',
      systemPrompt: 'p',
      metrics: { avg_rating: 5 },
      current_load: 9
    })
    for (const banned of ['type', 'description', 'systemPrompt', 'metrics', 'current_load']) {
      expect(body, banned).not.toHaveProperty(banned)
    }
    expect(body.availability?.current_load).toBeUndefined()
  })
})

describe('字段校验', () => {
  it('空名称被拦住，理由指向后端那条 400', () => {
    expect(expertDraftProblem({ ...DRAFT(), name: '   ' })).toMatch(/不能为空/)
    expect(expertFieldProblem('name', '')).toMatch(/名称/)
  })

  it('时薪小数被拒：后端按 as_u64 读，小数是静默丢弃而不是四舍五入', () => {
    expect(expertFieldProblem('hourlyRateCents', 12.5)).toMatch(/整数/)
  })

  it('时薪超过 u32 上限被拒：as u32 会截成另一个数', () => {
    expect(expertFieldProblem('hourlyRateCents', EXPERT_U32_MAX + 1)).toMatch(/截/)
    expect(expertFieldProblem('hourlyRateCents', EXPERT_U32_MAX)).toBe('')
  })

  it('枚举越界被拒（后端照收不验，所以只有这一层拦得住）', () => {
    expect(expertFieldProblem('expertType', 'alien')).toMatch(/expert_type|human/)
    expect(expertFieldProblem('availabilityStatus', 'online')).toBe('')
    // 留空合法：不发这个键，由后端默认值决定
    expect(expertFieldProblem('pricingModel', '')).toBe('')
  })

  it('熟练度必须手填且在 u8 之内', () => {
    const row = { name: 'a', domain: '', proficiency: undefined, description: '' }
    expect(expertFieldProblem('capabilities', [row])).toMatch(/0–255|整数/)
    expect(expertFieldProblem('capabilities', [{ ...row, proficiency: EXPERT_PROFICIENCY_MAX + 1 }])).toMatch(/u8/)
    expect(expertFieldProblem('capabilities', [{ ...row, proficiency: 80 }])).toBe('')
  })

  it('全空的能力行直接忽略，不会逼用户填一条无意义记录', () => {
    expect(expertFieldProblem('capabilities', [{ name: '', domain: '', proficiency: undefined, description: '' }])).toBe('')
  })

  it('一次列出所有问题而不是只报第一条', () => {
    const list = expertProblems({ ...DRAFT(), name: '', hourlyRateCents: 1.5, expertType: 'nope' })
    expect(list.length).toBe(3)
  })
})

describe('expertFormDraft', () => {
  it('从 normExpert 的嵌套形状读回在线状态与并发上限', () => {
    const draft = expertFormDraft({
      name: 'x',
      capabilities: [{ id: 'cap-1', name: 'a', domain: 'b', proficiency: 90, description: '' }],
      availability: { status: 'busy', maxConcurrent: 2 }
    })
    expect(draft.availabilityStatus).toBe('busy')
    expect(draft.maxConcurrent).toBe(2)
    expect(draft.capabilities).toHaveLength(1)
  })

  it('缺字段时回落与 ExpertDescriptor::minimal 同源的值', () => {
    const draft = DRAFT()
    expect(draft.languages).toEqual(['zh-CN', 'en'])
    expect(draft.timezone).toBe('Asia/Shanghai')
    expect(draft.availabilityStatus).toBe('online')
    expect(draft.hourlyRateCents).toBe(0)
  })

  it('每个挂载字段都有 wire 名与 kind，供 api 层与视图共用', () => {
    for (const f of EXPERT_REGISTER_FIELDS) {
      expect(f.wire, f.key).toBeTruthy()
      expect(['text', 'textarea', 'select', 'number', 'tags', 'capabilities']).toContain(f.kind)
    }
  })

  it('后端给的空数组是「真的没有」，不会显示成默认语言', () => {
    expect(expertFormDraft({ name: 'x', languages: [] }).languages).toEqual([])
    expect(expertFormDraft({ name: 'x' }).languages).toEqual(['zh-CN', 'en'])
  })
})

describe('expertPatch（合并式更新）', () => {
  it('没有改动就是空 patch，不发请求', () => {
    const e = { name: 'x', domains: ['a'], availability: { status: 'online', maxConcurrent: 5 } }
    expect(expertPatch(e, { ...expertFormDraft(e) }).patch).toEqual({})
  })

  it('只发改动过的键，未动的数组不出现（否则会被清空）', () => {
    const before = { name: 'x', bio: 'keep', domains: ['a', 'b'], tags: ['t'] }
    const { patch } = expertPatch(before, { ...expertFormDraft(before), name: 'y' })
    expect(patch).toEqual({ name: 'y' })
  })

  it('数组清空发的是整值空数组，那是用户主动清空', () => {
    const before = { name: 'x', skills: ['a'] }
    const { patch } = expertPatch(before, { ...expertFormDraft(before), skills: [] })
    expect(patch).toEqual({ skills: [] })
  })

  it('嵌套字段只带改动的那个子键', () => {
    const before = { name: 'x', availability: { status: 'online', maxConcurrent: 5 } }
    const { patch } = expertPatch(before, { ...expertFormDraft(before), availabilityStatus: 'offline' })
    expect(patch).toEqual({ availability: { status: 'offline' } })
  })

  it('清空名称不会被发出去：PUT 分支不复核 name，后端会把专家改成无名', () => {
    const before = { name: 'x' }
    const { patch, problem } = expertPatch(before, { ...expertFormDraft(before), name: '' })
    expect(patch).not.toHaveProperty('name')
    expect(problem).toMatch(/不能为空/)
  })

  it('非法值既不发送也给出原因', () => {
    const before = { name: 'x', hourlyRateCents: 100 }
    const { patch, problem } = expertPatch(before, { ...expertFormDraft(before), hourlyRateCents: 1.5 })
    expect(patch).not.toHaveProperty('hourly_rate_cents')
    expect(problem).toMatch(/整数/)
  })

  it('空 patch 与校验失败可区分：前者无 problem', () => {
    const before = { name: 'x' }
    const r = expertPatch(before, expertFormDraft(before))
    expect(r.problem).toBe('')
    expect(Object.keys(r.patch)).toHaveLength(0)
  })
})

describe('软删语义', () => {
  it('后果清单逐条给出后端位置', () => {
    const lines = deleteConsequences({ id: 'exp-1', name: '张三' })
    expect(lines.length).toBeGreaterThanOrEqual(4)
    for (const line of lines) expect(line).toMatch(/\.rs[:\d]/)
    expect(lines.join('\n')).toMatch(/张三/)
  })

  it('消失是全域事件：清单要把协作、编排、组队也点出来', () => {
    const all = deleteConsequences({ id: 'exp-1' }).join('\n')
    for (const src of ['experts_registry.rs', 'experts_dispatcher.rs', 'experts_collaboration.rs', 'experts_orchestration.rs', 'experts_graph.rs', 'experts_db.rs']) {
      expect(all, `后果清单漏了 ${src} 这一处 enabled 口径`).toContain(src)
    }
  })

  it('响应文案承认不可逆，而不是"删除成功"', () => {
    expect(deleteResultText({ deleted: true, softDelete: true })).toMatch(/无法再启用/)
    expect(deleteResultText({ deleted: true, softDelete: false })).toMatch(/口径/)
    expect(deleteResultText({ deleted: false })).toMatch(/未确认/)
  })
})

describe('写面身份提示', () => {
  it('只说"没有角色判定"，不替后端编造授权模型', () => {
    expect(EXPERT_WRITE_IDENTITY.statement).toMatch(/没有角色判定/)
    expect(EXPERT_WRITE_IDENTITY.statement).toMatch(/任何已认证身份/)
    expect(EXPERT_WRITE_IDENTITY.statement).not.toMatch(/管理员|无权限|权限不足|授权/)
  })

  it('每条前提都带着代码位置', () => {
    expect(EXPERT_WRITE_IDENTITY.evidence.length).toBeGreaterThanOrEqual(3)
    for (const ref of EXPERT_WRITE_IDENTITY.evidence) expect(ref).toMatch(/\.[a-z]+:\d+/)
  })
})

describe('契约自陈', () => {
  it('挂载字段数与后端白名单同量级，避免整片字段没人管', () => {
    expect(EXPERT_REGISTER_FIELDS.length).toBeGreaterThanOrEqual(15)
  })

  it('未挂载与别名字段都各自带着理由', () => {
    for (const f of EXPERT_UNMOUNTED_FIELDS) expect(f.reason.length).toBeGreaterThan(10)
    for (const a of EXPERT_ALIAS_FIELDS) expect(a.writesTo).toMatch(/[a-z_]+/)
  })
})
