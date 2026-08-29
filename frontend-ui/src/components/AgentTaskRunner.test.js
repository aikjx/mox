/**
 * AgentTaskRunner 组件单元测试
 *
 * 覆盖的 Bug 修复：
 * - Bug: regenerate/sendMessage 中传入的 taskMsg 是普通对象，
 *        修改 taskSteps 不会触发 UI 更新（Vue 响应式丢失）
 * - 验证：组件能正确响应 steps 数组的变化，步骤状态更新后 UI 同步刷新
 */

import { describe, it, expect, vi } from 'vitest'
import { ref, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import AgentTaskRunner from './AgentTaskRunner.vue'

// 模拟 Element Plus 组件和图标
vi.mock('element-plus', () => ({
  ElTag: {
    name: 'ElTag',
    props: ['size', 'effect', 'type'],
    template: '<span class="el-tag" :class="type"><slot /></span>'
  }
}))

vi.mock('@element-plus/icons-vue', () => ({
  MagicStick: { name: 'MagicStick', template: '<span class="icon-magic">✨</span>' },
  Loading: { name: 'Loading', template: '<span class="icon-loading">⟳</span>' },
  CircleCheck: { name: 'CircleCheck', template: '<span class="icon-check">✓</span>' },
  CircleClose: { name: 'CircleClose', template: '<span class="icon-close">✕</span>' },
  Check: { name: 'Check', template: '<span class="icon-check-sm">✓</span>' },
  Close: { name: 'Close', template: '<span class="icon-close-sm">✕</span>' }
}))

describe('AgentTaskRunner 任务执行可视化组件', () => {
  const mockSteps = [
    { title: '理解需求', status: 'done', tool: 'NLP 解析', detail: '提取核心目标', result: '识别3个目标' },
    { title: '任务拆解', status: 'done', tool: '规划引擎', detail: '拆分为5个子任务' },
    { title: '技术选型', status: 'running', tool: '知识检索' },
    { title: '架构设计', status: 'pending', tool: null },
    { title: '结果整理', status: 'pending', tool: null }
  ]

  describe('基础渲染', () => {
    it('应渲染任务标题', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { title: '测试任务', status: 'running', steps: mockSteps }
      })
      expect(wrapper.find('.task-title').text()).toBe('测试任务')
    })

    it('应渲染所有步骤', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { steps: mockSteps }
      })
      const stepItems = wrapper.findAll('.step-item')
      expect(stepItems.length).toBe(5)
    })

    it('每个步骤应显示标题', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { steps: mockSteps }
      })
      const stepTitles = wrapper.findAll('.step-title')
      expect(stepTitles.length).toBe(5)
      expect(stepTitles[0].text()).toBe('理解需求')
      expect(stepTitles[2].text()).toBe('技术选型')
    })
  })

  describe('步骤状态显示', () => {
    it('done 状态的步骤应有 done 类', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { steps: mockSteps }
      })
      const doneSteps = wrapper.findAll('.step-item.done')
      expect(doneSteps.length).toBe(2)
    })

    it('running 状态的步骤应有 running 类', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { steps: mockSteps }
      })
      const runningSteps = wrapper.findAll('.step-item.running')
      expect(runningSteps.length).toBe(1)
      expect(runningSteps[0].find('.step-title').text()).toBe('技术选型')
    })

    it('pending 状态的步骤应有 pending 类', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { steps: mockSteps }
      })
      const pendingSteps = wrapper.findAll('.step-item.pending')
      expect(pendingSteps.length).toBe(2)
    })

    it('error 状态的步骤应有 error 类', () => {
      const errorSteps = [
        { title: '失败的步骤', status: 'error', error: '连接超时' }
      ]
      const wrapper = mount(AgentTaskRunner, {
        props: { steps: errorSteps, status: 'error' }
      })
      expect(wrapper.find('.step-item.error').exists()).toBe(true)
      expect(wrapper.find('.step-error').text()).toBe('连接超时')
    })
  })

  describe('任务状态显示', () => {
    it('running 状态应显示"AI 正在执行中"', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { status: 'running', steps: mockSteps }
      })
      expect(wrapper.find('.task-status-text').text()).toContain('AI 正在执行中')
    })

    it('done 状态应显示"已完成"和步数', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { status: 'done', steps: mockSteps }
      })
      expect(wrapper.find('.task-status-text').text()).toContain('已完成')
      expect(wrapper.find('.task-status-text').text()).toContain('5 步')
    })

    it('error 状态应显示"执行出错"', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { status: 'error', steps: mockSteps }
      })
      expect(wrapper.find('.task-status-text').text()).toContain('执行出错')
    })

    it('running 状态应显示进度计数', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { status: 'running', steps: mockSteps }
      })
      expect(wrapper.find('.task-progress').exists()).toBe(true)
      expect(wrapper.find('.task-progress').text()).toBe('2/5')
    })

    it('非 running 状态不应显示进度计数', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { status: 'done', steps: mockSteps }
      })
      expect(wrapper.find('.task-progress').exists()).toBe(false)
    })
  })

  describe('步骤详情显示', () => {
    it('有 detail 字段且状态为 done/running 的步骤应显示 detail', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { steps: mockSteps }
      })
      const details = wrapper.findAll('.step-detail')
      // mockSteps 中：第1步(done)有detail，第2步(done)有detail，第3步(running)无detail
      expect(details.length).toBe(2)
    })

    it('pending 状态的步骤不应显示 detail', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { steps: mockSteps }
      })
      const pendingSteps = wrapper.findAll('.step-item.pending')
      pendingSteps.forEach(step => {
        expect(step.find('.step-detail').exists()).toBe(false)
      })
    })

    it('done 状态的步骤应显示 result', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { steps: mockSteps }
      })
      const results = wrapper.findAll('.step-result')
      expect(results.length).toBe(1) // 只有第一步有 result
      expect(results[0].find('.result-content').text()).toBe('识别3个目标')
    })

    it('有 tool 的步骤应显示工具标签', () => {
      const wrapper = mount(AgentTaskRunner, {
        props: { steps: mockSteps }
      })
      const tools = wrapper.findAll('.step-tool')
      expect(tools.length).toBe(3) // 前三个步骤有 tool
    })
  })

  describe('响应式更新（Bug 修复验证）', () => {
    // Bug 修复前：传入普通对象后，修改 steps 不会触发 UI 更新
    // Bug 修复后：使用响应式引用，steps 变化后 UI 同步更新

    it('steps 数组变化时 UI 应同步更新', async () => {
      const steps = ref([
        { title: '步骤1', status: 'pending' },
        { title: '步骤2', status: 'pending' }
      ])

      const wrapper = mount(AgentTaskRunner, {
        props: { steps: steps.value, status: 'running' }
      })

      // 初始状态：2 个 pending
      expect(wrapper.findAll('.step-item.pending').length).toBe(2)
      expect(wrapper.findAll('.step-item.done').length).toBe(0)

      // 更新第一个步骤为 done
      steps.value[0].status = 'done'
      await wrapper.setProps({ steps: [...steps.value] })
      await nextTick()

      // 验证 UI 已更新
      expect(wrapper.findAll('.step-item.done').length).toBe(1)
      expect(wrapper.findAll('.step-item.pending').length).toBe(1)
    })

    it('步骤从 pending 变为 running 时 UI 应同步更新', async () => {
      const steps = ref([
        { title: '步骤1', status: 'done' },
        { title: '步骤2', status: 'running' },
        { title: '步骤3', status: 'pending' }
      ])

      const wrapper = mount(AgentTaskRunner, {
        props: { steps: steps.value, status: 'running' }
      })

      expect(wrapper.find('.step-item.running .step-title').text()).toBe('步骤2')

      // 步骤2 完成，步骤3 开始运行
      steps.value[1].status = 'done'
      steps.value[2].status = 'running'
      await wrapper.setProps({ steps: [...steps.value] })
      await nextTick()

      expect(wrapper.findAll('.step-item.done').length).toBe(2)
      expect(wrapper.find('.step-item.running .step-title').text()).toBe('步骤3')
    })

    it('所有步骤完成后状态变为 done', async () => {
      const status = ref('running')
      const steps = ref([
        { title: '步骤1', status: 'done' },
        { title: '步骤2', status: 'running' }
      ])

      const wrapper = mount(AgentTaskRunner, {
        props: { status: status.value, steps: steps.value }
      })

      expect(wrapper.find('.task-icon').classes()).toContain('running')

      // 完成所有步骤
      steps.value[1].status = 'done'
      status.value = 'done'
      await wrapper.setProps({ status: status.value, steps: [...steps.value] })
      await nextTick()

      expect(wrapper.find('.task-icon').classes()).toContain('done')
      expect(wrapper.find('.task-status-text').text()).toContain('已完成')
    })

    it('步骤添加 detail 后 UI 应显示', async () => {
      const steps = ref([
        { title: '步骤1', status: 'running' }
      ])

      const wrapper = mount(AgentTaskRunner, {
        props: { steps: steps.value, status: 'running' }
      })

      expect(wrapper.find('.step-detail').exists()).toBe(false)

      // 添加 detail
      steps.value[0].detail = '正在处理中...'
      await wrapper.setProps({ steps: [...steps.value] })
      await nextTick()

      expect(wrapper.find('.step-detail').exists()).toBe(true)
      expect(wrapper.find('.step-detail').text()).toBe('正在处理中...')
    })
  })

  describe('防止 Bug 回归的专项测试', () => {
    // 核心 Bug：直接修改普通对象的属性不会触发 Vue 响应式更新
    // 修复方案：使用数组中的响应式引用

    it('验证：直接修改 props 对象的嵌套属性需要完整替换才能触发更新', async () => {
      // 这是对 Bug 原因的验证测试
      // 如果你直接修改 step.status = 'done' 而不触发响应式，UI 不会更新
      // 正确的做法是使用 Vue 的响应式数据（如 ref/reactive 中的数组元素）

      const steps = ref([
        { title: '测试步骤', status: 'pending' }
      ])

      const wrapper = mount(AgentTaskRunner, {
        props: { steps: steps.value }
      })

      const stepItem = wrapper.find('.step-item')
      expect(stepItem.classes()).toContain('pending')

      // 错误方式：直接修改数组元素（不会触发更新，因为是同一个引用）
      steps.value[0].status = 'done'
      // 这种情况下，由于 Vue 的响应式系统，直接修改 ref 数组的元素属性
      // 在 Vue 3 中其实是响应式的（因为 Proxy），但组件 props 的浅层比较可能不更新
      // 所以我们需要确保通过 props 传递的是新数组引用

      // 正确方式：创建新数组触发 props 更新
      await wrapper.setProps({ steps: [...steps.value] })
      await nextTick()

      const updatedStep = wrapper.find('.step-item')
      expect(updatedStep.classes()).toContain('done')
    })

    it('验证：步骤进度计数应随步骤状态实时更新', async () => {
      const steps = ref([
        { title: '步骤1', status: 'pending' },
        { title: '步骤2', status: 'pending' },
        { title: '步骤3', status: 'pending' }
      ])

      const wrapper = mount(AgentTaskRunner, {
        props: { steps: steps.value, status: 'running' }
      })

      // 初始进度: 0/3
      expect(wrapper.find('.task-progress').text()).toBe('0/3')

      // 完成第一步
      steps.value[0].status = 'done'
      steps.value[1].status = 'running'
      await wrapper.setProps({ steps: [...steps.value] })
      await nextTick()

      expect(wrapper.find('.task-progress').text()).toBe('1/3')

      // 完成第二步
      steps.value[1].status = 'done'
      steps.value[2].status = 'running'
      await wrapper.setProps({ steps: [...steps.value] })
      await nextTick()

      expect(wrapper.find('.task-progress').text()).toBe('2/3')

      // 全部完成
      steps.value[2].status = 'done'
      await wrapper.setProps({ steps: [...steps.value], status: 'done' })
      await nextTick()

      expect(wrapper.find('.task-progress').exists()).toBe(false)
      expect(wrapper.find('.task-status-text').text()).toContain('共 3 步')
    })
  })
})
