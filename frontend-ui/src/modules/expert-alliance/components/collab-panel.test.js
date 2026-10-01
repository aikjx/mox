// 协作工作台装配测试：视图只吃归一后的结果，且只显示契约声明过的输入项。
// Element Plus 用 render 函数手写替身，与 plaza-components.test.js 同一套约定。
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { h, inject, nextTick, provide } from 'vue'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'

const { api } = vi.hoisted(() => ({ api: { collaborate: vi.fn() } }))
vi.mock('../api/alliance.api.js', () => ({ allianceApi: api }))

const { useAllianceCollabStore } = await import('../store/alliance-collab.store.js')
const { COLLAB_MODE, COLLAB_MODES } = await import('../contract/collab.js')
const {
  normRouteResult, normMultiConsult, normDebate, normIntelligentConsult, normAlgorithmAnalysis, normSingleConsult
} = await import('../model/normalize.js')
const ExpertCollabPanel = (await import('./ExpertCollabPanel.vue')).default

/** 透传型替身：渲染为同名自定义元素，props 落为属性 */
const pass = (tag, props = []) => ({
  name: tag,
  props,
  setup(_props, { slots }) {
    return () => h(tag, { class: `ep-${tag}`, ..._props }, slots.default?.())
  }
})

const ElAlert = pass('el-alert', ['type', 'title', 'description', 'showIcon', 'closable'])
const ElButton = {
  name: 'el-button',
  props: ['type', 'size', 'text', 'loading', 'disabled'],
  setup(props, { slots }) {
    return () => h('button', { class: 'ep-btn', disabled: props.disabled ? '' : null }, slots.default?.())
  }
}
// v-model 替身：写回走 update:modelValue，与真实 EP 的输入语义一致
const ElInput = {
  name: 'el-input',
  props: ['modelValue', 'type', 'placeholder', 'disabled', 'size', 'rows'],
  emits: ['update:modelValue'],
  setup(props, { emit }) {
    return () => h(props.type === 'textarea' ? 'textarea' : 'input', {
      class: 'ep-input',
      placeholder: props.placeholder || '',
      disabled: props.disabled ? '' : null,
      value: props.modelValue ?? '',
      onInput: (e) => emit('update:modelValue', e.target.value)
    })
  }
}
const ElInputNumber = {
  name: 'el-input-number',
  props: ['modelValue', 'min', 'max', 'step', 'size', 'disabled', 'valueOnClear'],
  emits: ['update:modelValue'],
  setup(props, { emit }) {
    return () => h('input', {
      class: 'ep-number',
      'data-min': String(props.min),
      'data-max': String(props.max),
      'data-clears-to': String(props.valueOnClear),
      value: String(props.modelValue ?? ''),
      onInput: (e) => emit('update:modelValue', Number(e.target.value))
    })
  }
}
const ElSwitch = {
  name: 'el-switch',
  props: ['modelValue', 'size', 'disabled'],
  emits: ['update:modelValue'],
  setup(props, { emit }) {
    return () => h('button', {
      class: 'ep-switch',
      'data-on': String(!!props.modelValue),
      onClick: () => emit('update:modelValue', !props.modelValue)
    })
  }
}

const SET_TAB = Symbol('ep-set-tab')
const ElTabs = {
  name: 'el-tabs',
  props: ['modelValue'],
  emits: ['update:modelValue'],
  setup(props, { slots, emit }) {
    provide(SET_TAB, (name) => emit('update:modelValue', name))
    return () => h('div', { class: 'ep-tabs', 'data-active': String(props.modelValue) }, slots.default?.())
  }
}
const ElTabPane = {
  name: 'el-tab-pane',
  props: ['name', 'label'],
  setup(props) {
    const set = inject(SET_TAB, () => {})
    return () => h('button', { class: 'ep-tab', onClick: () => set(props.name) }, props.label)
  }
}

const global = { components: { ElAlert, ElButton, ElInput, ElInputNumber, ElSwitch, ElTabs, ElTabPane } }

const expert = (id, over = {}) => ({
  id, name: `专家${id}`, title: 'T', status: 'online', online: true,
  availability: { status: 'online', avgResponseMinutes: 4, currentLoad: 1 },
  metrics: { totalConsultations: 10, avgRating: 4.5, ratingCount: 3, resolutionRate: 0.9 },
  ...over
})

const mountPanel = (over = {}, experts = [expert('e1'), expert('e2')]) => {
  const store = useAllianceCollabStore()
  if (over.mode) store.setMode(over.mode)
  Object.assign(store.input, over.input || {})
  Object.assign(store.controls, over.controls || {})
  if (over.result) store.result = over.result
  if (over.errorRun) store.error.run = over.errorRun
  if (over.history) store.history = over.history
  return { wrapper: mount(ExpertCollabPanel, { props: { store, experts }, global }), store }
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
})

describe('输入区：只出现该模式后端认识的控件', () => {
  it('六个模式各占一个页签，切换即改写 store.mode', async () => {
    const { wrapper, store } = mountPanel()
    const tabs = wrapper.findAll('.ep-tab')
    expect(tabs.map((t) => t.text())).toEqual(COLLAB_MODES.map((m) => m.label))
    await tabs[3].trigger('click')
    expect(store.mode).toBe(COLLAB_MODE.DEBATE)
  })

  it('主输入框按模式字段取放：切到辩论写的是 topic', async () => {
    const { wrapper, store } = mountPanel({ mode: COLLAB_MODE.SMART })
    const box = wrapper.find('textarea.ep-input')
    await box.setValue('智能咨询的问题')
    expect(store.input.question).toBe('智能咨询的问题')

    store.setMode(COLLAB_MODE.DEBATE)
    await nextTick()
    expect(wrapper.find('textarea.ep-input').attributes('placeholder')).toContain('微服务')
    await wrapper.find('textarea.ep-input').setValue('辩题')
    expect(store.input.topic).toBe('辩题')
    // question 是另一个后端字段，不该被辩题覆盖
    expect(store.input.question).toBe('智能咨询的问题')
  })

  it('数值控件与文案逐条落在后端边界上', () => {
    const { wrapper } = mountPanel({ mode: COLLAB_MODE.ROUTE })
    const num = wrapper.find('.ep-number')
    expect(num.attributes('data-min')).toBe('1')
    expect(num.attributes('data-max')).toBe('20')
    expect(wrapper.find('.acw-hint').text()).toContain('max_experts 允许 1–20')
    expect(wrapper.find('.acw-hint').text()).toContain('夹到边界')
  })

  it('领域过滤只出现在读该字段的模式，补充上下文只出现在智能咨询', () => {
    const labelsOf = (mode) => {
      const { wrapper } = mountPanel({ mode })
      return wrapper.findAll('.acw-field label, .acw-field .acw-label').map((n) => n.text()).join('|')
    }
    expect(labelsOf(COLLAB_MODE.ROUTE)).toContain('领域过滤')
    expect(labelsOf(COLLAB_MODE.MULTI)).toContain('领域过滤')
    expect(labelsOf(COLLAB_MODE.SMART)).toContain('补充上下文')
    expect(labelsOf(COLLAB_MODE.DEBATE)).not.toContain('领域过滤')
    expect(labelsOf(COLLAB_MODE.ALGORITHM)).not.toMatch('领域过滤|补充上下文')
  })

  it('路由的三条嵌套约束只在路由出现，且说明写的是后端子键名', () => {
    const { wrapper } = mountPanel({ mode: COLLAB_MODE.ROUTE })
    const hints = wrapper.findAll('.acw-hint').map((n) => n.text())
    expect(hints.filter((t) => t.startsWith('constraints.'))).toEqual([
      'constraints.min_rating：0 表示不限',
      'constraints.max_response_time：留空表示不限',
      'constraints.require_online：后端按 availability==online 过滤'
    ])
    // 布尔项用开关；两条数值项都可清空回「不限」，清空后该子键不进请求体
    expect(wrapper.findAll('.ep-switch').length).toBe(1)
    expect(wrapper.findAll('.ep-number[data-clears-to="null"]').length).toBe(2)
    // 主控件（max_experts）没有「不限」语义，不能被清空
    expect(wrapper.find('.ep-number[data-clears-to="undefined"]').attributes('data-max')).toBe('20')

    const other = mountPanel({ mode: COLLAB_MODE.MULTI }).wrapper
    expect(other.findAll('.acw-hint').map((n) => n.text()).filter((t) => t.startsWith('constraints.'))).toEqual([])
  })

  it('选专家的模式才渲染专家区；不选由后端自动匹配，路由与智能咨询不给选', () => {
    expect(mountPanel({ mode: COLLAB_MODE.ROUTE }).wrapper.find('.acw-picker').exists()).toBe(false)
    expect(mountPanel({ mode: COLLAB_MODE.SMART }).wrapper.find('.acw-picker').exists()).toBe(false)
    const { wrapper } = mountPanel({ mode: COLLAB_MODE.MULTI })
    expect(wrapper.findAll('.acw-chip')).toHaveLength(2)
    expect(wrapper.find('.acw-picker-head').text()).toContain('不选则由后端自动匹配')
  })

  it('辩论超上限时说明只有前 4 位上场，而不是假装都算', async () => {
    const experts = [1, 2, 3, 4, 5, 6].map((i) => expert(`e${i}`))
    const { wrapper, store } = mountPanel({ mode: COLLAB_MODE.DEBATE, input: { topic: 't' } }, experts)
    for (const chip of wrapper.findAll('.acw-chip')) await chip.trigger('click')
    expect(store.pickedCount).toBe(6)
    expect(wrapper.find('.acw-capacity').text()).toBe('已选 6 位，后端仅取前 4 位上场')
  })

  it('校验不过就不给运行，并把原因写在按钮旁', () => {
    const { wrapper } = mountPanel({ mode: COLLAB_MODE.SINGLE })
    expect(wrapper.find('.acw-warn').text()).toBe('请填写向所选专家提问')
    expect(wrapper.find('.acw-run .ep-btn').attributes('disabled')).toBe('')
  })
})

describe('结果区：按模式取渲染口径', () => {
  it('路由只给候选与推荐，不编造专家回复', () => {
    const { wrapper } = mountPanel({
      mode: COLLAB_MODE.ROUTE,
      result: normRouteResult({
        query: '微服务拆分',
        matched_experts: [{ id: 'e1', name: '林架构', title: '首席', domains: ['architecture'], skills: [], match_score: 0.512, availability: { status: 'online', avg_response_minutes: 4, current_load: 1 }, metrics: { total_consultations: 180, avg_rating: 4.7, resolution_rate: 0.93 } }],
        routing_decision: { recommended_expert_id: 'e1', reason: '匹配度最高且在线', alternative_ids: [] },
        total_scanned: 41,
        ts: '2026-09-23T18:05:00Z'
      })
    })
    expect(wrapper.find('.acw-recommend b').text()).toBe('推荐 林架构')
    expect(wrapper.find('.acw-recommend p').text()).toBe('匹配度最高且在线')
    expect(wrapper.text()).toContain('共扫描 41 位在册专家')
    expect(wrapper.text()).toContain('路由只排序作答，不产出回复')
    // 候选表 1 行；推荐卡不算行，避免把「推荐」重复计成候选
    expect(wrapper.findAll('.acw-rows .acw-row')).toHaveLength(1)
  })

  it('融合结果：单专家的共识度显示为无从比对，而不是 1.00', () => {
    const { wrapper } = mountPanel({
      mode: COLLAB_MODE.MULTI,
      result: normMultiConsult({
        session_id: 's1',
        question: 'q',
        experts: [{ id: 'e1', name: '林架构', match_score: 0.319, answer: { analysis: '析', solution: '案', references: [], confidence: 0.94, source: 'llm' } }],
        fused_answer: { summary: '综合1位专家', consensus_score: 1, dominant_view: '【林架构】案', alternative_views: [], confidence: 0.61 },
        created_at: 't'
      })
    })
    expect(wrapper.text()).toContain('—（单专家无从比对共识度）')
    expect(wrapper.text()).not.toContain('共识度 1.00')
    expect(wrapper.text()).toContain('真实模型作答')
    expect(wrapper.findAll('.acw-answer')).toHaveLength(1)
  })

  it('融合结论被否决时给出拦截说明而不是方案正文', () => {
    const { wrapper } = mountPanel({
      mode: COLLAB_MODE.MULTI,
      result: normMultiConsult({
      session_id: 's2',
      question: 'q',
      experts: [{ id: 'e1', name: '甲', match_score: 0.9, answer: { analysis: 'a', solution: '【已拦截】高危', references: [], confidence: 0, vetoed: true, veto_reason: '高危操作', blocked: true } }],
        fused_answer: { summary: '1 位专家的回复全部被治理闸门否决', consensus_score: 0, dominant_view: '', alternative_views: [], confidence: 0, vetoed: true, blocked: true },
        created_at: 't'
      })
    })
    const alert = wrapper.find('.ep-el-alert')
    expect(alert.attributes('type')).toBe('warning')
    expect(alert.attributes('title')).toBe('结果被治理闸门拦截')
    // 文案要符合后端行为：正文被替换成带原因的说明，而不是「什么都没有」
    expect(alert.attributes('description')).toContain('替换后的拦截说明')
    expect(alert.attributes('description')).toContain('原因：高危操作')
  })

  it('辩论结果按轮次展开，并给出裁决与双方分数', () => {
    const { wrapper } = mountPanel({
      mode: COLLAB_MODE.DEBATE,
      result: normDebate({
        debate_id: 'd1', topic: '微服务是否优于单体', rounds: 2,
        participants: [{ id: 'e1', name: '甲', side: 'pro', final_score: 0.72 }, { id: 'e2', name: '乙', side: 'con', final_score: 0.51 }],
        debate_log: [{ round: 1, pro_argument: '正方一', con_argument: '反方一', pro_score: 0.4, con_score: 0.3 }, { round: 2, pro_argument: '正方二', con_argument: '反方二', pro_score: 0.32, con_score: 0.21 }],
        verdict: { winner: '甲', summary: '甲胜出', key_points: ['要点一'], consensus_level: '中等共识' },
        created_at: 't'
      })
    })
    expect(wrapper.find('.acw-recommend b').text()).toBe('甲胜出')
    expect(wrapper.findAll('.acw-round')).toHaveLength(2)
    // 上场名单来自后端裁决结果，不是前端本地选中项
    expect(wrapper.findAll('.acw-rows .acw-row')).toHaveLength(2)
    expect(wrapper.text()).toContain('第 2 轮 · 正方 0.32 / 反方 0.21')
    expect(wrapper.text()).toContain('2 位上场')
  })

  it('智能咨询结果带意图分类、行动项与风险评估', () => {
    const { wrapper } = mountPanel({
      mode: COLLAB_MODE.SMART,
      result: normIntelligentConsult({
        consultation_id: 'c1', question: 'q', intent: 'architecture',
        matched_expert: { id: 'e1', name: '林架构', title: '首席' },
        answer: { analysis: '分析', solution: '方案', action_items: ['先做现状盘点'], risk_assessment: { technical_risk: 't', schedule_risk: 's', resource_risk: 'r', overall_level: 'medium' }, references: ['《架构指南》'], confidence: 0.8 },
        related_experts: [{ id: 'e2', name: '乙', title: 'T2', domains: [] }],
        created_at: 't'
      })
    })
    expect(wrapper.text()).toContain('意图分类：架构')
    expect(wrapper.text()).toContain('先做现状盘点')
    expect(wrapper.text()).toContain('风险评估（后端固定为 medium 档）')
    expect(wrapper.text()).toContain('相关专家：乙（T2）')
  })

  it('单专家咨询结果不带意图（该端点不产出 intent）', () => {
    const { wrapper } = mountPanel({
      mode: COLLAB_MODE.SINGLE,
      result: normSingleConsult({ session_id: 's', expert_id: 'e1', expert_name: '林架构', question: 'q', answer: { analysis: 'a', solution: 's', references: [], confidence: 0 }, created_at: 't' })
    })
    expect(wrapper.find('.acw-answer-head b').text()).toBe('林架构')
    expect(wrapper.text()).not.toContain('意图分类')
    // confidence 为 0 表示后端没给可信度，显示成占位而不是 0%
    expect(wrapper.text()).toContain('置信度 —')
  })

  it('算法分析结果按渐近阶上色，并列出阻塞/风险/建议', () => {
    const { wrapper } = mountPanel({
      mode: COLLAB_MODE.ALGORITHM,
      result: normAlgorithmAnalysis({
        analysis_id: 'algo-1', algorithm_description: '递归求解', input_constraints: null, requirements: null,
        complexity: { time_complexity: 'O(2^n)', space_complexity: 'O(n)', big_o_notation: 'Time: O(2^n), Space: O(n)', explanation: '指数级' },
        feasibility: { score: 0.35, blockers: ['n>30 超时'], risks: ['常数因子'] },
        recommended_experts: [{ id: 'e1', name: '数专家', title: 'T', domains: ['math'], match_score: 0.4 }],
        optimization_suggestions: ['引入记忆化'],
        created_at: 't'
      })
    })
    expect(wrapper.find('.acw-tag').classes()).toContain('is-danger')
    expect(wrapper.text()).toContain('Time: O(2^n), Space: O(n)')
    expect(wrapper.text()).toContain('可行性 35%')
    expect(wrapper.findAll('.acw-block')).toHaveLength(3)
    expect(wrapper.text()).toContain('数专家 匹配度 0.400')
  })

  it('失败只显示错误条，历史里留一条失败记录', async () => {
    const { wrapper } = mountPanel({
      mode: COLLAB_MODE.MULTI,
      input: { question: 'q' },
      errorRun: '网关 502',
      history: [{ mode: 'multi', modeLabel: '多专家协同', refId: '', text: 'q', at: 't', ok: false }]
    })
    expect(wrapper.find('.ep-el-alert').attributes('title')).toBe('协作失败：网关 502')
    expect(wrapper.find('.acw-side').text()).toBe('失败')
    expect(wrapper.find('.acw-side').classes()).toContain('is-failed')
  })

  it('运行记录说明后端会话才是持久记录', () => {
    const { wrapper } = mountPanel({
      mode: COLLAB_MODE.MULTI,
      history: [{ mode: 'multi', modeLabel: '多专家协同', refId: 'sess-9', text: 'q', at: 't', ok: true }]
    })
    expect(wrapper.find('.acw-history-head').text()).toContain('后端会把协作写入专家会话')
    expect(wrapper.find('.acw-side').text()).toBe('sess-9')
  })
})
