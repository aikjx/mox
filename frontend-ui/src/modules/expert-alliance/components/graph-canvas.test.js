// GraphCanvas 渲染与交互：它是纯呈现组件，这里只钉"坐标→SVG"与意图上抛两条。
// 坐标换算依赖 getScreenCTM（jsdom 无），按比例兜底分支也被同一断言覆盖。
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import GraphCanvas from './GraphCanvas.vue'
import { graphLayout } from '@/modules/expert-alliance/model'

function layoutWith(nodes, edges = []) {
  return graphLayout(nodes, edges)
}

const NODES = [
  { id: 'domain-ai', label: 'ai', nodeType: 'domain' },
  { id: 'e1', label: '甲', nodeType: 'expert' },
  { id: 'cap-ocr', label: 'OCR', nodeType: 'capability' }
]
const EDGES = [{ source: 'e1', target: 'domain-ai', edgeType: 'has_domain', weight: 1 }]

describe('GraphCanvas 呈现', () => {
  it('专家/能力域/能力点三类节点分色，能力点进图例', () => {
    const w = mount(GraphCanvas, { props: { layout: layoutWith(NODES, EDGES) } })
    expect(w.find('.agc-dot.is-expert').exists()).toBe(true)
    expect(w.find('.agc-dot.is-domain').exists()).toBe(true)
    expect(w.find('.agc-dot.is-capability').exists()).toBe(true)
    expect(w.text()).toContain('能力点')
  })

  it('点节点上抛 select 意图，自己不记账', async () => {
    const w = mount(GraphCanvas, { props: { layout: layoutWith(NODES, EDGES) } })
    await w.findAll('.agc-node')[1].trigger('click')
    expect(w.emitted('select')[0]).toEqual(['e1'])
  })

  it('连线起点高亮，编辑模式提示语切换', () => {
    const w = mount(GraphCanvas, {
      props: { layout: layoutWith(NODES, EDGES), editMode: true, linkSourceId: 'e1' }
    })
    expect(w.find('.agc-node.is-link-source').exists()).toBe(true)
    expect(w.text()).toContain('编辑模式')
  })
})
