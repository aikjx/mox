/**
 * AssistantSelector 组件单元测试
 *
 * 覆盖的 Bug 修复：
 * - Bug: AssistantSelector 内部使用 ref 管理状态，未与父组件 v-model 同步，
 *        导致初始选中状态错误、切换后父组件状态不一致
 * - 验证：v-model 双向绑定正常、初始状态正确、change 事件正确触发
 */

import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import AssistantSelector from './AssistantSelector.vue'

// 模拟 Element Plus 的 ElTag 和 Check 图标
vi.mock('element-plus', () => ({
  ElTag: {
    name: 'ElTag',
    props: ['size', 'effect'],
    template: '<span class="el-tag"><slot /></span>'
  }
}))

vi.mock('@element-plus/icons-vue', () => ({
  Check: {
    name: 'Check',
    template: '<span class="check-icon">✓</span>'
  }
}))

describe('AssistantSelector 助手选择器组件', () => {
  const mockAssistants = [
    { id: 'architect', name: '架构师小智', emoji: '🏗️', desc: '系统架构专家', tags: ['架构', '设计'], gradient: 'linear-gradient(135deg, #6366f1, #8b5cf6)' },
    { id: 'analyst', name: '分析师小研', emoji: '📊', desc: '需求分析专家', tags: ['分析', '调研'], gradient: 'linear-gradient(135deg, #06b6d4, #0ea5e9)' },
    { id: 'general', name: '全能助手小通', emoji: '✨', desc: '通用任务协调', tags: ['综合', '调度'], gradient: 'linear-gradient(135deg, #ec4899, #8b5cf6)' },
  ]

  describe('基础渲染', () => {
    it('应渲染所有助手卡片', () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'general' }
      })
      const cards = wrapper.findAll('.assistant-card')
      expect(cards.length).toBe(6) // 实际组件有6个助手
    })

    it('应显示标题和副标题', () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'general' }
      })
      expect(wrapper.find('.selector-title').text()).toContain('选择 AI 助手')
      expect(wrapper.find('.selector-subtitle').text()).toContain('不同专家各有所长')
    })
  })

  describe('v-model 双向绑定（Bug 修复验证）', () => {
    it('初始选中状态应与 modelValue prop 一致', () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'architect' }
      })

      const activeCard = wrapper.find('.assistant-card.active')
      expect(activeCard.exists()).toBe(true)
      expect(activeCard.find('.assistant-name').text()).toBe('架构师小智')
    })

    it('初始选中 general 时应高亮全能助手', () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'general' }
      })

      const activeCard = wrapper.find('.assistant-card.active')
      expect(activeCard.find('.assistant-name').text()).toBe('全能助手小通')
    })

    it('点击不同助手卡片应触发 update:modelValue 事件', async () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'general' }
      })

      const cards = wrapper.findAll('.assistant-card')
      // 点击第二个卡片（分析师小研）
      await cards[1].trigger('click')

      const updateEvents = wrapper.emitted('update:modelValue')
      expect(updateEvents).toBeTruthy()
      expect(updateEvents.length).toBe(1)
      expect(updateEvents[0][0]).toBe('analyst')
    })

    it('点击不同助手应触发 change 事件并携带完整助手信息', async () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'general' }
      })

      const cards = wrapper.findAll('.assistant-card')
      await cards[0].trigger('click') // 架构师

      const changeEvents = wrapper.emitted('change')
      expect(changeEvents).toBeTruthy()
      expect(changeEvents.length).toBe(1)

      const payload = changeEvents[0][0]
      expect(payload).toHaveProperty('id', 'architect')
      expect(payload).toHaveProperty('name', '架构师小智')
      expect(payload).toHaveProperty('emoji', '🏗️')
    })

    it('父组件更新 modelValue 后选中状态应同步更新', async () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'general' }
      })

      // 初始选中 general
      let activeCard = wrapper.find('.assistant-card.active')
      expect(activeCard.find('.assistant-name').text()).toBe('全能助手小通')

      // 父组件改变 modelValue
      await wrapper.setProps({ modelValue: 'analyst' })

      // 选中状态应同步更新
      activeCard = wrapper.find('.assistant-card.active')
      expect(activeCard.find('.assistant-name').text()).toBe('分析师小研')
    })

    it('点击当前已选中的卡片仍应触发事件', async () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'architect' }
      })

      const cards = wrapper.findAll('.assistant-card')
      await cards[0].trigger('click') // 点击已选中的架构师

      const updateEvents = wrapper.emitted('update:modelValue')
      expect(updateEvents.length).toBe(1)
      expect(updateEvents[0][0]).toBe('architect')
    })
  })

  describe('助手卡片内容', () => {
    it('每个卡片应显示头像、名称、描述和标签', () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'general' }
      })

      const firstCard = wrapper.find('.assistant-card')
      expect(firstCard.find('.assistant-avatar').exists()).toBe(true)
      expect(firstCard.find('.assistant-emoji').exists()).toBe(true)
      expect(firstCard.find('.assistant-name').exists()).toBe(true)
      expect(firstCard.find('.assistant-desc').exists()).toBe(true)
      expect(firstCard.find('.assistant-tags').exists()).toBe(true)
    })

    it('选中的卡片应显示勾选图标', () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'architect' }
      })

      const activeCard = wrapper.find('.assistant-card.active')
      expect(activeCard.find('.assistant-check').exists()).toBe(true)
    })

    it('未选中的卡片不应显示勾选图标', () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'architect' }
      })

      const inactiveCards = wrapper.findAll('.assistant-card:not(.active)')
      inactiveCards.forEach(card => {
        expect(card.find('.assistant-check').exists()).toBe(false)
      })
    })
  })

  describe('样式类名', () => {
    it('选中的卡片应有 active 类', async () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'general' }
      })

      const activeCards = wrapper.findAll('.assistant-card.active')
      expect(activeCards.length).toBe(1)
    })

    it('切换选中后 active 类应正确移动', async () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'architect' }
      })

      let cards = wrapper.findAll('.assistant-card')
      expect(cards[0].classes()).toContain('active')
      expect(cards[1].classes()).not.toContain('active')

      await wrapper.setProps({ modelValue: 'analyst' })

      cards = wrapper.findAll('.assistant-card')
      expect(cards[0].classes()).not.toContain('active')
      expect(cards[1].classes()).toContain('active')
    })
  })

  describe('防止 Bug 回归的专项测试', () => {
    // 修复前的 Bug：组件内部用 ref('architect') 管理状态，导致：
    // 1. 初始值永远是 architect，不随 prop 变化
    // 2. 父组件改变 modelValue 时，组件内部状态不更新
    // 3. 点击后只更新内部 ref，不通知父组件（或者通知了但不同步）

    it('初始选中状态应由 modelValue prop 决定，而非硬编码的 architect', () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'data' } // 数据工程师小数
      })

      const activeCard = wrapper.find('.assistant-card.active')
      const activeName = activeCard.find('.assistant-name').text()

      // Bug 修复前：永远返回 '架构师小智'（硬编码的初始值）
      // Bug 修复后：应返回 '数据工程师小数'（prop 传入的值）
      expect(activeName).toBe('数据工程师小数')
      expect(activeName).not.toBe('架构师小智')
    })

    it('v-model 应实现真正的双向绑定', async () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'architect', 'onUpdate:modelValue': (val) => wrapper.setProps({ modelValue: val }) }
      })

      // 初始状态
      expect(wrapper.find('.assistant-card.active .assistant-name').text()).toBe('架构师小智')

      // 模拟用户点击切换
      const cards = wrapper.findAll('.assistant-card')
      await cards[5].trigger('click') // 全能助手

      // 等待 Vue 更新
      await wrapper.vm.$nextTick()

      // 验证事件触发
      const updateEvents = wrapper.emitted('update:modelValue')
      expect(updateEvents).toBeTruthy()
      expect(updateEvents[0][0]).toBe('general')
    })

    it('change 事件应携带完整的助手对象信息', async () => {
      const wrapper = mount(AssistantSelector, {
        props: { modelValue: 'architect' }
      })

      const cards = wrapper.findAll('.assistant-card')
      await cards[2].trigger('click') // 数据工程师

      const changeEvents = wrapper.emitted('change')
      expect(changeEvents).toBeTruthy()

      const assistant = changeEvents[0][0]
      // 验证返回完整对象，而不是只有 id
      expect(typeof assistant).toBe('object')
      expect(assistant).toHaveProperty('id')
      expect(assistant).toHaveProperty('name')
      expect(assistant).toHaveProperty('emoji')
      expect(assistant).toHaveProperty('desc')
      expect(assistant).toHaveProperty('tags')
      expect(Array.isArray(assistant.tags)).toBe(true)
    })
  })
})
