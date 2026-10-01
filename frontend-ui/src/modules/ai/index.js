// AI 域模块声明。路由从 router/modules/ai.js 迁入内核单源，meta 逐字保持（裸页 bare 映射 layout:blank 并保留 bare 标记）。
import { defineModule } from '@/modules/_kernel/module-registry.js'

export const AI_MODULE = defineModule({
  name: 'ai',
  title: 'AI 能力域',
  version: '0.1.0',
  routes: [
    {
      path: '/ai',
      name: 'AI',
      component: () => import('@/views/ai/ChatView.vue'),
      meta: { title: 'AI 助手', module: 'ai', layout: 'default', requiresAuth: true }
    },
    {
      path: '/share/:token',
      name: 'ShareSnapshot',
      component: () => import('@/views/ai/ChatView.vue'),
      meta: { title: '分享对话', module: 'ai', layout: 'blank', shareMode: true, bare: true }
    },
    {
      path: '/caomei',
      name: 'Caomei',
      component: () => import('@/views/ai/CaomeiView.vue'),
      meta: { title: '需求编译', module: 'ai', layout: 'default', requiresAuth: true }
    },
    {
      path: '/algolab',
      name: 'AlgoLab',
      component: () => import('@/views/ai/AlgoLabView.vue'),
      meta: { title: '算法实验室', module: 'ai', layout: 'default', requiresAuth: true }
    },
    {
      path: '/infinite-optimizer',
      name: 'InfiniteOptimizer',
      component: () => import('@/views/ai/InfiniteOptimizerView.vue'),
      meta: { title: '无穷维度优化', module: 'ai', layout: 'default', requiresAuth: true }
    },
    {
      path: '/botCenter',
      name: 'BotCenter',
      component: () => import('@/views/ai/BotCenterView.vue'),
      meta: { title: '机器人中心', module: 'ai', layout: 'default', requiresAuth: true }
    },
    {
      path: '/melody2score',
      name: 'Melody2Score',
      component: () => import('@/views/ai/Melody2ScoreView.vue'),
      meta: { title: '旋律转谱', module: 'ai', layout: 'default', requiresAuth: true }
    }
  ],
  nav: []
}, import.meta.url)

export default AI_MODULE
