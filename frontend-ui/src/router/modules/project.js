// Domain route definitions; authentication is applied by the host router.
export default [  // ===== 项目域 =====
  {
    path: '/dashboard',
    name: 'Dashboard',
    component: () => import('@/views/project/Dashboard.vue'),
    meta: { title: '工作台', requiresAuth: true }
  },
  {
    path: '/projects',
    name: 'Projects',
    component: () => import('@/views/project/ProjectsView.vue'),
    meta: { title: '项目中心', requiresAuth: true }
  },
  {
    path: '/tasks',
    name: 'Tasks',
    component: () => import('@/views/project/TaskView.vue'),
    meta: { title: '任务管理', requiresAuth: true }
  },
  {
    path: '/resources',
    component: () => import('@/views/project/ResourcesView.vue'),
    meta: {
      title: '资源管理',
      // 知识库页面可一键跳回专家工作台
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
          // 知识库页面快捷返回工作台
          backTo: { path: '/expert-workspace', label: '返回工作台' }
        }
      }
    ]
  },
  {
    path: '/workbench',
    name: 'Workbench',
    component: () => import('@/views/project/Workbench.vue'),
    meta: { title: '工作台执行', requiresAuth: true }
  },


]
