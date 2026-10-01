// 图谱域模块声明。路由从 router/modules/graph.js 迁入内核单源，meta 逐字保持（仅注入 module/layout）。
import { defineModule } from '@/modules/_kernel/module-registry.js'

export const GRAPH_MODULE = defineModule({
  name: 'graph',
  title: '知识图谱',
  version: '0.1.0',
  routes: [
    {
      path: '/graph',
      name: 'Graph',
      component: () => import('@/views/graph/GraphView.vue'),
      meta: {
        title: '知识图谱',
        module: 'graph',
        layout: 'default',
        requiresAuth: true,
        quickNav: [
          { key: 'expert-workspace', label: '专家工作台', path: '/expert-workspace', icon: 'User' },
          { key: 'knowledge', label: '知识库', path: '/resources/knowledge', icon: 'Coin' }
        ],
        backTo: { path: '/expert-workspace', label: '返回工作台' }
      }
    },
    {
      path: '/mox-fusion',
      name: 'MoxFusion',
      component: () => import('@/views/graph/MoxFusionView.vue'),
      meta: {
        title: '架构融合',
        module: 'graph',
        layout: 'default',
        requiresAuth: true,
        backTo: { path: '/expert-workspace', label: '返回工作台' }
      }
    },
    {
      path: '/flow-graph',
      name: 'FlowGraph',
      component: () => import('@/views/graph/FlowGraph.vue'),
      meta: {
        title: '流程图',
        module: 'graph',
        layout: 'default',
        requiresAuth: true,
        backTo: { path: '/expert-workspace', label: '返回工作台' }
      }
    }
  ],
  nav: []
}, import.meta.url)

export default GRAPH_MODULE
