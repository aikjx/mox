// 项目 Store - Pinia 兼容接口
//
// 状态统一：所有项目状态以 composables/projectContext.js 为唯一真实源。
// 本 Pinia store 仅重新暴露相同的单例 refs，不维护独立状态，避免双份状态。
// 组件应优先使用 useProject()（来自 projectContext），本 store 供 Pinia 生态兼容。
import { defineStore } from 'pinia'
import { computed } from 'vue'
import {
  currentProject,
  projectList,
  listLoading,
  projectReady,
  loadProjectList,
  loadCurrentById,
  setCurrentProject,
  ensureProjectContext,
  createAndSelect,
} from '@/composables/projectContext.js'

export const useProjectStore = defineStore('project', () => {
  // Getters（基于同一批 refs 派生）
  const currentProjectId = computed(() => currentProject.value?.id || null)
  const hasProjects = computed(() => projectList.value.length > 0)

  // 事件处理（委托给 projectContext 的窗口事件机制，保持行为一致）
  async function handleProjectUpdated(id) {
    const pid = id || currentProject.value?.id
    if (!pid) { await loadProjectList(); return }
    await Promise.all([loadProjectList(), loadCurrentById(pid, { force: true })])
  }

  async function handleProjectDeleted(id) {
    if (id && currentProject.value?.id === id) {
      currentProject.value = null
    }
    await loadProjectList()
    if (!currentProject.value && projectList.value.length) {
      const firstActive = projectList.value.find((p) => p.status === 'active')
      await setCurrentProject((firstActive || projectList.value[0]).id)
    }
  }

  return {
    // State（与 projectContext 同一批 refs，状态唯一）
    currentProject,
    projectList,
    listLoading,
    projectReady,
    // Getters
    currentProjectId,
    hasProjects,
    // Actions（委托给 projectContext 实现）
    loadProjectList,
    loadCurrentById,
    setCurrentProject,
    ensureProjectContext,
    createAndSelect,
    handleProjectUpdated,
    handleProjectDeleted,
  }
})
