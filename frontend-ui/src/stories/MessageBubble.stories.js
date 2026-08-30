/**
 * 消息气泡组件 - Storybook 故事
 *
 * 展示 MessageBubble 组件在不同场景下的渲染效果
 */
import MessageBubble from '../components/MessageBubble.vue'

export default {
  title: '消息/MessageBubble',
  component: MessageBubble,
  tags: ['autodocs'],
  argTypes: {
    role: {
      control: 'select',
      options: ['user', 'assistant', 'system'],
    },
  },
}

// 基础用户消息
export const UserMessage = {
  args: {
    msg: {
      id: 'msg-001',
      role: 'user',
      content: '你好，请帮我分析一下这个项目的架构。',
      timestamp: Date.now() - 300000,
    },
  },
}

// 基础助手消息
export const AssistantMessage = {
  args: {
    msg: {
      id: 'msg-002',
      role: 'assistant',
      content: '好的，我来为您分析项目架构。\n\n## 架构概览\n\n该项目采用 **前后端分离** 架构，主要包含以下模块：\n\n1. **前端层** - Vue 3 + Element Plus\n2. **API 层** - RESTful API 接口\n3. **服务层** - 业务逻辑处理\n4. **数据层** - PostgreSQL + Redis\n\n### 核心特性\n\n- 支持流式响应\n- 多模型切换\n- 插件扩展机制',
      timestamp: Date.now() - 240000,
    },
  },
}

// 带代码块的消息
export const WithCodeBlock = {
  args: {
    msg: {
      id: 'msg-003',
      role: 'assistant',
      content: '以下是一个简单的 Python 示例：\n\n```python\ndef hello(name: str) -> str:\n    """问候函数"""\n    return f"Hello, {name}!"\n\nif __name__ == "__main__":\n    print(hello("World"))\n```\n\n这段代码展示了基本的函数定义和类型注解。',
      timestamp: Date.now() - 180000,
    },
  },
}

// 带列表的消息
export const WithLists = {
  args: {
    msg: {
      id: 'msg-004',
      role: 'assistant',
      content: '## 优化建议\n\n### 性能优化\n\n- [x] 启用 Gzip 压缩\n- [x] 图片懒加载\n- [ ] CDN 加速\n- [ ] 数据库索引优化\n\n### 安全加固\n\n1. 输入验证\n2. SQL 注入防护\n3. XSS 过滤\n4. CSRF Token',
      timestamp: Date.now() - 120000,
    },
  },
}

// 系统消息
export const SystemMessage = {
  args: {
    msg: {
      id: 'msg-005',
      role: 'system',
      content: '会话已创建。欢迎使用璇玑助手！',
      timestamp: Date.now() - 600000,
    },
  },
}

// 带算子引用的消息
export const WithOperators = {
  args: {
    msg: {
      id: 'msg-006',
      role: 'assistant',
      content: '已完成数据分析，共调用 3 个算子。',
      timestamp: Date.now() - 60000,
      referenced_operators: [
        { id: 'op-1', name: '数据清洗', status: 'success' },
        { id: 'op-2', name: '特征工程', status: 'success' },
        { id: 'op-3', name: '模型训练', status: 'running' },
      ],
    },
  },
}
