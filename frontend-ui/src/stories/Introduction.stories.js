/**
 * 璇玑前端组件库 - 总览
 *
 * 组件库架构与使用指南
 */

export default {
  title: '入门/Introduction',
  tags: ['autodocs'],
}

export const Overview = {
  render: () => ({
    template: `
      <div style="padding: 24px; max-width: 800px; margin: 0 auto;">
        <h1>璇玑 · 前端组件库</h1>
        <p>基于 Vue 3 + Element Plus 的企业级组件库</p>

        <h2>技术栈</h2>
        <ul>
          <li><strong>框架：</strong>Vue 3 (Composition API)</li>
          <li><strong>UI 库：</strong>Element Plus</li>
          <li><strong>构建工具：</strong>Vite</li>
          <li><strong>状态管理：</strong>Pinia</li>
          <li><strong>路由：</strong>Vue Router 4</li>
          <li><strong>图表：</strong>ECharts</li>
        </ul>

        <h2>组件分类</h2>

        <h3>基础组件</h3>
        <ul>
          <li>TheSidebar - 侧边导航栏</li>
          <li>TheTopbar - 顶部导航栏</li>
          <li>TabBar - 标签栏</li>
          <li>ProjectPicker - 项目选择器</li>
        </ul>

        <h3>业务组件</h3>
        <ul>
          <li>MessageBubble - 消息气泡</li>
          <li>FlowDetailDialog - 流程详情对话框</li>
          <li>ProjectPicker - 项目选择器</li>
          <li>AgentFlowPanel - Agent 流程面板</li>
          <li>AgentTaskRunner - Agent 任务运行器</li>
        </ul>

        <h3>视图面板（按业务域分组）</h3>
        <ul>
          <li>project/panels/KnowledgeBasePanel - 知识库面板</li>
          <li>expert/panels/ExpertEnterprisePanel - 企业管理面板</li>
          <li>expert/panels/ExpertOrchestratorPanel - 编排引擎面板</li>
          <li>workflow/panels/PluginsPanel - 插件中心面板</li>
          <li>workflow/panels/McpPanel - MCP 兼容面板</li>
          <li>workflow/panels/AutomationPanel - 自动化面板</li>
          <li>admin/panels/ - 8 个系统管理子面板</li>
        </ul>

        <h3>Composables</h3>
        <ul>
          <li>useTheme - 主题管理</li>
          <li>useProject - 项目上下文</li>
          <li>useKnowledgeBase - 知识库业务逻辑</li>
          <li>useMessageActions - 消息操作（评分/收藏/分享/朗读）</li>
        </ul>

        <h3>Pinia Stores</h3>
        <ul>
          <li>app.store - 应用级状态（主题、侧边栏等）</li>
          <li>project.store - 项目状态</li>
          <li>ui.store - UI 交互状态</li>
          <li>user.store - 用户信息与认证</li>
        </ul>

        <h2>目录结构</h2>
        <pre>
src/
├── components/      # 通用组件
├── views/           # 页面视图（按业务域分组）
│   ├── project/     # 项目域
│   ├── ai/          # AI 域
│   ├── graph/       # 图谱域
│   ├── workflow/    # 工作流域
│   ├── expert/      # 专家联盟
│   ├── market/      # 算子商城
│   ├── operators/   # 算子中心
│   ├── admin/       # 系统管理
│   └── misc/        # 其他
├── composables/     # 组合式函数
├── stores/          # Pinia 状态管理
├── api/             # API 层（按领域拆分）
├── constants/       # 常量定义
├── utils/           # 工具函数
├── router/          # 路由配置
└── styles/          # 全局样式
        </pre>

        <h2>开发规范</h2>
        <ol>
          <li>使用 Composition API + <code>&lt;script setup&gt;</code></li>
          <li>组件命名使用 PascalCase</li>
          <li>Props 必须定义类型</li>
          <li>复杂业务逻辑提取为 composable</li>
          <li>状态管理优先使用 Pinia</li>
          <li>API 调用统一在 api/ 目录下</li>
        </ol>
      </div>
    `,
  }),
}
