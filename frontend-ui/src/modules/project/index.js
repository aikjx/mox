// 项目域模块声明。路由从 router/modules/project.js 迁入内核单源，meta 逐字保持（仅父路由注入 module/layout）。
import { defineModule } from '@/modules/_kernel/module-registry.js'

export const PROJECT_MODULE = defineModule({
  name: 'project',
  title: '项目域',
  version: '0.1.0',
  routes: [
    {
      path: '/dashboard',
      name: 'Dashboard',
      component: () => import('@/views/project/Dashboard.vue'),
      meta: { title: '工作台', module: 'project', layout: 'default', requiresAuth: true }
    },
    {
      path: '/projects',
      name: 'Projects',
      component: () => import('@/views/project/ProjectsView.vue'),
      meta: { title: '项目中心', module: 'project', layout: 'default', requiresAuth: true }
    },
    {
      path: '/tasks',
      name: 'Tasks',
      component: () => import('@/views/project/TaskView.vue'),
      meta: { title: '任务管理', module: 'project', layout: 'default', requiresAuth: true }
    },
    {
      path: '/resources',
      component: () => import('@/views/project/ResourcesView.vue'),
      meta: {
        title: '资源管理',
        module: 'project',
        layout: 'default',
        quickNav: [
          { key: 'expert-workspace', label: '专家工作台', path: '/expert-workspace', icon: 'User' },
          { key: 'graph', label: '知识图谱', path: '/graph', icon: 'Share' }
        ]
      },
      redirect: '/resources/overview',
      children: [
        { path: '', redirect: '/resources/overview' },
        {
          path: 'overview',
          name: 'ResourcesOverview',
          component: () => import('@/views/project/panels/ResourcesOverviewPanel.vue'),
          meta: { title: '资源概览', requiresAuth: true }
        },
        {
          path: 'knowledge',
          name: 'ResourcesKnowledge',
          component: () => import('@/views/project/panels/KnowledgeBasePanel.vue'),
          meta: {
            title: '知识库',
            requiresAuth: true,
            backTo: { path: '/expert-workspace', label: '返回工作台' }
          }
        }
      ]
    },
    {
      path: '/workbench',
      name: 'Workbench',
      component: () => import('@/views/project/Workbench.vue'),
      meta: { title: '工作台执行', module: 'project', layout: 'default', requiresAuth: true }
    }
  ],
  nav: []
}, import.meta.url)

export default PROJECT_MODULE
