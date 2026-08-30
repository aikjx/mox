/**
 * 知识库面板组件 - Storybook 故事
 *
 * 展示 KnowledgeBasePanel 组件的核心功能
 */
import KnowledgeBasePanel from '../views/project/panels/KnowledgeBasePanel.vue'

export default {
  title: '业务组件/KnowledgeBasePanel',
  component: KnowledgeBasePanel,
  tags: ['autodocs'],
}

// 默认状态
export const Default = {
  args: {},
}

// 紧凑模式
export const Compact = {
  args: {
    compact: true,
  },
}
