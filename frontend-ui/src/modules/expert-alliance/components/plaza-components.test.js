// 广场组件装配测试：卡片只吃归一后的视图模型，面板只吃归一后的预约。
// 手写 Element Plus 替身（render 函数版，不依赖模板编译器），
// 既能覆盖作用域插槽与事件上抛，又不必把整个 EP 拉进测试环境。
import { describe, it, expect } from 'vitest'
import { h, provide, inject } from 'vue'
import { mount } from '@vue/test-utils'
import ExpertCard from './ExpertCard.vue'
import ExpertBookingPanel from './ExpertBookingPanel.vue'
import ExpertCapabilityMatrix from './ExpertCapabilityMatrix.vue'
import ExpertRankBoard from './ExpertRankBoard.vue'

/** 透传型替身：渲染为同名自定义元素，props 落为属性，默认插槽原样输出 */
const pass = (tag, props = []) => ({
  name: tag,
  props,
  setup(_props, { slots }) {
    return () => h(tag, { class: `ep-${tag}`, ..._props }, slots.default?.())
  }
})

const ElTag = pass('el-tag', ['type', 'size', 'effect'])
const ElIcon = pass('el-icon')
const ElEmpty = pass('el-empty', ['description', 'imageSize'])
const ElSkeleton = pass('el-skeleton', ['rows', 'animated'])
const ElAlert = pass('el-alert', ['type', 'title', 'description', 'showIcon', 'closable'])
const ElTooltip = pass('el-tooltip', ['content', 'placement', 'disabled'])
const ElProgress = pass('el-progress', ['percentage', 'strokeWidth', 'color'])
const ElButton = {
  name: 'el-button',
  props: ['type', 'size', 'text', 'plain', 'icon', 'loading', 'disabled'],
  setup(props, { slots }) {
    return () => h('button', { class: 'ep-btn', disabled: props.disabled ? '' : null }, slots.default?.())
  }
}
// 表格替身按真实 EP 的做法：表体把行数据 provide 给每个列，列逐行调用作用域插槽
const ROWS = Symbol('ep-rows')
const ElTable = {
  name: 'el-table',
  props: ['data'],
  setup(props, { slots }) {
    provide(ROWS, () => props.data || [])
    return () => h('div', { class: 'ep-table', 'data-rows': String((props.data || []).length) }, slots.default?.())
  }
}
// 无作用域插槽时按 prop 取值，与真实 EP 的列渲染语义一致
const ElTableColumn = {
  name: 'el-table-column',
  props: ['label', 'prop'],
  setup(props, { slots }) {
    const rows = inject(ROWS, () => [])
    const cell = (row) => (slots.default ? slots.default({ row }) : row[props.prop] ?? '')
    return () => h('div', { class: 'ep-col' }, rows().map((row, i) => h('div', { class: 'ep-cell', 'data-i': String(i) }, cell(row))))
  }
}
const ElPopconfirm = {
  name: 'el-popconfirm',
  props: ['title'],
  emits: ['confirm'],
  setup(_props, { slots, emit }) {
    return () => h('span', { class: 'ep-pop' }, [
      slots.reference?.(),
      h('button', { class: 'ep-pop-ok', onClick: () => emit('confirm') }, 'confirm')
    ])
  }
}

// 本页组件只用页签头，pane 渲染成可点击按钮来回写 v-model
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

// 目录面板的本地关键词框：只需 v-model 语义
const ElInput = {
  name: 'el-input',
  props: ['modelValue', 'placeholder', 'size', 'clearable', 'prefixIcon'],
  emits: ['update:modelValue'],
  setup(props, { emit }) {
    return () => h('input', {
      class: 'ep-input',
      value: props.modelValue,
      onInput: (e) => emit('update:modelValue', e.target.value)
    })
  }
}

const global = {
  components: { ElTag, ElIcon, ElEmpty, ElSkeleton, ElAlert, ElTooltip, ElProgress, ElButton, ElTable, ElTableColumn, ElPopconfirm, ElTabs, ElTabPane, ElInput }
}

// normExpert 输出的形状，字段与 contract.test.js 的真实载荷夹具一致
const modelExpert = (over = {}) => ({
  id: 'e1',
  name: '林架构',
  title: '首席架构师',
  organization: '璇玑科技',
  bio: '分布式系统 12 年',
  domains: ['architecture', 'backend'],
  skills: ['Rust', 'gRPC', 'Kafka'],
  capabilities: [],
  availability: { status: 'online', lastActive: '', avgResponseMinutes: 4.5, currentLoad: 2, maxConcurrent: 5, loadRatio: 0.4 },
  status: 'online',
  online: true,
  expertType: 'human',
  pricingModel: 'paid',
  hourlyRateCents: 12000,
  verificationStatus: 'certified',
  timezone: 'Asia/Shanghai',
  languages: ['zh'],
  metrics: { totalConsultations: 180, todayConsultations: 3, avgRating: 4.7, ratingCount: 56, resolutionRate: 0.93, firstResponseAccuracy: 0.88, totalServiceMinutes: 0 },
  ...over
})

const mountCard = (over, favorite = false) =>
  mount(ExpertCard, { props: { expert: modelExpert(over), favorite }, global })

const action = (w, label) => w.findAll('.ae-actions button').find((b) => b.text() === label)

describe('ExpertCard', () => {
  it('呈现真实指标而不是占位值', () => {
    const w = mountCard()
    expect(w.text()).toContain('林架构')
    expect(w.text()).toContain('首席架构师 · 璇玑科技')
    expect(w.text()).toContain('4.7 / 5')
    expect(w.text()).toContain('180')
    expect(w.text()).toContain('93%')
    expect(w.text()).toContain('在线')
    expect(w.text()).toContain('2 / 5')
    expect(w.text()).toContain('按次计费 · 120 元/时')
  })

  it('无评分记录时显示「暂无评分」而不是 0.0', () => {
    const w = mountCard({ metrics: { ...modelExpert().metrics, avgRating: 0, ratingCount: 0, resolutionRate: 0 } })
    expect(w.text()).toContain('暂无评分')
    expect(w.text()).toContain('—')
  })

  it('max_concurrent=0 时负载不谎报为百分比', () => {
    const w = mountCard({ availability: { status: 'busy', avgResponseMinutes: 0, currentLoad: 1, maxConcurrent: 0, loadRatio: null } })
    expect(w.text()).toContain('1 · 未设上限')
    expect(w.text()).toContain('忙碌')
  })

  it('离线专家的即时咨询按钮禁用，并说明原因', () => {
    const w = mountCard({ online: false, availability: { ...modelExpert().availability, status: 'offline' } })
    expect(action(w, '即时咨询').attributes('disabled')).toBeDefined()
    expect(w.find('el-tooltip').attributes('content')).toContain('仅在线专家可即时接入')
  })

  it('四个意图各自独立上抛，卡片不改父状态', async () => {
    const w = mountCard()
    await w.find('.ae-star').trigger('click')
    await action(w, '详情').trigger('click')
    await action(w, '预约').trigger('click')
    await action(w, '即时咨询').trigger('click')
    expect(Object.keys(w.emitted())).toEqual(expect.arrayContaining(['favorite', 'view', 'book', 'consult']))
    expect(w.emitted('favorite')[0][0].id).toBe('e1')
  })

  it('收藏态只体现在样式与提示上', () => {
    expect(mountCard({}, true).classes()).toContain('is-favorite')
    expect(mountCard({}, false).classes()).not.toContain('is-favorite')
  })
})

const booking = (over = {}) => ({
  id: 'b1',
  expertId: 'e1',
  expertName: '林架构',
  userId: 'u',
  topic: '评审网关拆分方案',
  scheduledAt: '2026-09-25T02:00:00Z',
  durationMinutes: 60,
  status: 'pending',
  createdAt: '',
  cancellable: true,
  ...over
})

const mountPanel = (props = {}) =>
  mount(ExpertBookingPanel, {
    props: {
      bookings: [booking(), booking({ id: 'b2', status: 'completed', cancellable: false })],
      counts: { pending: 1, confirmed: 0, completed: 1, cancelled: 0 },
      loading: false,
      error: '',
      pendingId: '',
      ...props
    },
    global
  })

describe('ExpertBookingPanel', () => {
  it('汇总卡按服务端计数呈现', () => {
    const cells = mountPanel().findAll('.ab-cell')
    expect(cells).toHaveLength(5)
    expect(cells[0].text()).toContain('全部')
    expect(cells[1].text()).toContain('待确认')
    expect(cells[3].text()).toContain('已完成')
  })

  it('列表按服务端状态计数渲染，终态预约不给取消入口', () => {
    const w = mountPanel()
    expect(w.find('.ep-table').attributes('data-rows')).toBe('2')
    expect(w.findAll('.ep-pop')).toHaveLength(1)
    expect(w.text()).toContain('评审网关拆分方案')
  })

  it('摘要卡点击可按状态过滤，再点回到「全部」并高亮它', async () => {
    const w = mountPanel()
    expect(w.findAll('.ab-cell.is-active')[0].text()).toContain('全部')
    await w.findAll('.ab-cell')[1].find('button').trigger('click')
    expect(w.findAll('.ab-cell.is-active')).toHaveLength(1)
    expect(w.find('.ep-table').attributes('data-rows')).toBe('1')
    await w.findAll('.ab-cell')[1].find('button').trigger('click')
    expect(w.findAll('.ab-cell.is-active')[0].text()).toContain('全部')
    expect(w.find('.ep-table').attributes('data-rows')).toBe('2')
  })

  it('取消与咨询室意图上抛给 store', async () => {
    const w = mountPanel()
    await w.find('.ep-pop-ok').trigger('click')
    expect(w.emitted('cancel')[0][0].id).toBe('b1')
    const roomBtn = w.findAll('button').find((b) => b.text().includes('咨询室'))
    await roomBtn.trigger('click')
    expect(w.emitted('room')[0][0].id).toBe('b1')
  })

  it('后端未按登录用户过滤这一现状由面板自陈，不冒充个人视图', () => {
    expect(mountPanel().find('.ab-caption').text()).toContain('未按登录用户过滤')
  })

  it('加载失败与空态互斥，都以服务端事实为准', () => {
    const failed = mountPanel({ error: '网关 502' })
    expect(failed.find('el-alert').attributes('title')).toContain('预约列表加载失败')
    expect(failed.find('.ep-table').exists()).toBe(false)
    const empty = mountPanel({ bookings: [], counts: { pending: 0, confirmed: 0, completed: 0, cancelled: 0 } })
    expect(empty.find('el-alert').exists()).toBe(false)
    expect(empty.find('el-empty').attributes('description')).toContain('还没有预约')
  })
})

const mountRank = (experts = [
  modelExpert(),
  modelExpert({ id: 'e2', name: '周算法', metrics: { ...modelExpert().metrics, totalConsultations: 260 } })
]) => mount(ExpertRankBoard, { props: { experts }, global })

describe('ExpertRankBoard', () => {
  it('三张榜单都在，默认咨询量榜，并自陈样本只覆盖当前页', () => {
    const w = mountRank()
    expect(w.findAll('.ep-tab').map((t) => t.text())).toEqual(['咨询量榜', '评分榜', '新晋榜'])
    expect(w.find('.ep-tabs').attributes('data-active')).toBe('consultations')
    expect(w.text()).toContain('样本仅当前页 2 位专家，其中 2 位有可排名数据')
    const items = w.findAll('.ar-item')
    expect(items).toHaveLength(2)
    expect(items[0].text()).toContain('周算法')
    expect(items[0].find('.ar-value').text()).toBe('260 次')
    expect(items[0].find('.ar-rank').text()).toBe('1')
  })

  it('切页签即换口径，规则文案来自榜单定义', async () => {
    const w = mountRank()
    await w.findAll('.ep-tab')[1].trigger('click')
    expect(w.find('.ep-tabs').attributes('data-active')).toBe('rating')
    expect(w.find('.ar-rule').text()).toContain('avg_rating')
  })

  it('没有真实评分样本时不给名次，只给原因', async () => {
    const w = mountRank([modelExpert({ metrics: { ...modelExpert().metrics, ratingCount: 0 } })])
    await w.findAll('.ep-tab')[1].trigger('click')
    expect(w.findAll('.ar-item')).toHaveLength(0)
    expect(w.text()).toContain('其中 0 位有可排名数据')
    expect(w.find('el-empty').attributes('description')).toContain('真实评分')
  })
})

describe('ExpertCapabilityMatrix', () => {
  // normCapabilities 的输出形状：items 已按后端 id 升序；domains 次序照后端给的原样（BTreeSet 字母序），
  // 夹具故意用非字母序，才能证明组件没有自己重排
  const CAPS = [
    { id: 'cap-a', name: '意图拆解', domain: 'ai', expertCount: 5, avgProficiency: 91.2 },
    { id: 'cap-b', name: '架构评审', domain: 'architecture', expertCount: 3, avgProficiency: 88.5 },
    { id: 'cap-c', name: '知识建模', domain: 'architecture', expertCount: 1, avgProficiency: 70 }
  ]
  const mountMatrix = (over = {}) => mount(ExpertCapabilityMatrix, {
    props: { data: { items: CAPS, total: 3, domains: ['architecture', 'ai'] }, ...over },
    global
  })

  it('按领域分组呈现能力行、人数与平均熟练度，分组次序沿用后端', () => {
    const w = mountMatrix()
    expect(w.findAll('.ecm-group-name').map((g) => g.text())).toEqual(['architecture', 'ai'])
    const rows = w.findAll('.ecm-item')
    expect(rows.map((r) => r.find('.ecm-item-name').text())).toEqual(['架构评审', '知识建模', '意图拆解'])
    expect(rows[0].text()).toContain('3 人')
    expect(w.findAll('.ecm-item-bar').map((b) => b.attributes('percentage'))).toEqual(['89', '70', '91'])
    expect(w.find('.ecm-count').text()).toBe('3 项能力 · 2 个领域')
    expect(w.findAll('.ecm-group')[0].text()).toContain('2 项能力 · 4 人次')
    expect(w.findAll('.ecm-group')[1].text()).toContain('1 项能力 · 5 人次')
  })

  it('点领域上抛 pick，并高亮当前生效的领域', async () => {
    const w = mountMatrix({ activeDomain: 'ai' })
    expect(w.findAll('.ecm-group')[1].classes()).toContain('is-active')
    await w.findAll('.ecm-group-btn')[0].trigger('click')
    expect(w.emitted('pick')).toEqual([['architecture']])
  })

  it('关键词只在本页过滤，计数说明是筛出还是全量', async () => {
    const w = mountMatrix()
    await w.find('input').setValue('知识')
    expect(w.findAll('.ecm-item')).toHaveLength(1)
    expect(w.find('.ecm-count').text()).toContain('筛出 1 / 3 项能力 · 1 个领域')
    await w.find('input').setValue('不存在的能力')
    expect(w.findAll('.ecm-group')).toHaveLength(0)
    expect(w.find('el-empty').attributes('description')).toBe('没有匹配这个关键词的能力项')
  })

  it('加载中不出空态，出错时不拿旧目录冒充结果', () => {
    const loading = mountMatrix({ loading: true })
    expect(loading.find('.ecm-loading').exists()).toBe(true)
    expect(loading.findAll('.ecm-group')).toHaveLength(0)
    const errored = mountMatrix({ error: '能力目录 502' })
    expect(errored.find('el-alert').attributes('description')).toBe('能力目录 502')
    expect(errored.findAll('.ecm-group')).toHaveLength(0)
  })

  it('三种"没有"分开措辞：尚未加载 / 没有已启用专家 / 被关键词筛空', () => {
    const notLoaded = mount(ExpertCapabilityMatrix, { props: { data: null }, global })
    expect(notLoaded.find('el-empty').attributes('description')).toBe('能力目录尚未加载')
    const noRows = mount(ExpertCapabilityMatrix, {
      props: { data: { items: [], total: 0, domains: [] } }, global
    })
    expect(noRows.find('.ecm-count').text()).toBe('目录为空')
    expect(noRows.find('el-empty').attributes('description')).toContain('已启用专家')
  })

  it('说明文字承认后端不接收 capability_id，筛选只到领域这一层', () => {
    expect(mountMatrix().text()).toContain('后端列表接口不接收 capability_id')
  })
})
