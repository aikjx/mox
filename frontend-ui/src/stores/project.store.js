// 项目 Store - 当前项目、项目列表等
// 从 projectContext composable 迁移而来，保持兼容
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getProjects, getProject, createProject, registerProjectIdGetter } from '@/api'

const STORAGE_KEY = 'mox.currentProject.v1'

export const useProjectStore = defineStore('project', () => {
  // State
  const currentProject = ref(null)
  const projectList = ref([])
  const listLoading = ref(false)
  const projectReady = ref(false)

  // 把当前项目 id 暴露给请求层
  registerProjectIdGetter(() => currentProject.value?.id || null)

  // Getters
  const currentProjectId = computed(() => currentProject.value?.id || null)
  const hasProjects = computed(() => projectList.value.length > 0)

  // Actions
  async function loadProjectList() {
    listLoading.value = true
    try {
      const list = await getProjects()
      projectList.value = list
    } finally {
      listLoading.value = false
    }
  }

  async function loadCurrentById(id, { force = false } = {}) {
    if (!id) { currentProject.value = null; projectReady.value = true; return }
    if (currentProject.value?.id === id && !force) { projectReady.value = true; return }
    try {
      const p = await getProject(id)
      currentProject.value = p
    } catch {
      // 已保存的项目失效 → 兜底自动选第一个进行中项目
      currentProject.value = null
      try { localStorage.removeItem(STORAGE_KEY) } catch {}
      const fallback = (projectList.value || []).find((p) => p.status === 'active') || projectList.value[0]
      if (fallback) {
        currentProject.value = fallback
        try { localStorage.setItem(STORAGE_KEY, String(fallback.id)) } catch {}
      }
    }
    projectReady.value = true
  }

  async function setCurrentProject(id) {
    if (!id) {
      currentProject.value = null
      try { localStorage.removeItem(STORAGE_KEY) } catch {}
      return
    }
    await loadCurrentById(id, { force: true })
    try { localStorage.setItem(STORAGE_KEY, String(id)) } catch {}
  }

  async function ensureProjectContext() {
    if (projectReady.value && projectList.value.length) return
    await loadProjectList()
    let saved = null
    try { saved = localStorage.getItem(STORAGE_KEY) } catch {}
    let id = saved
    if (!id && projectList.value.length) {
      const firstActive = projectList.value.find((p) => p.status === 'active')
      id = (firstActive || projectList.value[0])?.id
    }
    await loadCurrentById(id)
  }

  async function createAndSelect(form) {
    const p = await createProject(form)
    await loadProjectList()
    await setCurrentProject(p.id)
    return p
  }

  async function handleProjectUpdated(id) {
    const pid = id || currentProject.value?.id
    if (!pid) { await loadProjectList(); return }
    await Promise.all([loadProjectList(), loadCurrentById(pid, { force: true })])
  }

  async function handleProjectDeleted(id) {
    if (id && currentProject.value?.id === id) {
      try { localStorage.removeItem(STORAGE_KEY) } catch {}
      currentProject.value = null
    }
    await loadProjectList()
    if (!currentProject.value && projectList.value.length) {
      const firstActive = projectList.value.find((p) => p.status === 'active')
      await setCurrentProject((firstActive || projectList.value[0]).id)
    }
  }

  return {
    // State
    currentProject,
    projectList,
    listLoading,
    projectReady,
    // Getters
    currentProjectId,
    hasProjects,
    // Actions
    loadProjectList,
    loadCurrentById,
    setCurrentProject,
    ensureProjectContext,
    createAndSelect,
    handleProjectUpdated,
    handleProjectDeleted
  }
})
