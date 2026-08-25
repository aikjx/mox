// 全局项目上下文（璇玑专家联盟 · 以项目为根）
// - provide/inject + reactive + localStorage 持久化
// - 视图刷新 / 路由跳转 / 浏览器重开 都保持「当前项目」不变
import { ref, computed, watch, onMounted, onBeforeUnmount, provide, inject } from 'vue'
import { getProjects, getProject, createProject } from '@/api'

const STORAGE_KEY = 'mox.currentProject.v1'

// —— 共享单例状态（供任意视图或组件 import 直接用） ——
const currentProject = ref(null)
const projectList = ref([])
const listLoading = ref(false)
const projectReady = ref(false)

let listeners = new Set()
function notifyChange(changed) {
  for (const fn of listeners) {
    try { fn(changed) } catch {}
  }
}

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
    currentProject.value = null
    try { localStorage.removeItem(STORAGE_KEY) } catch {}
  }
  projectReady.value = true
}

async function setCurrentProject(id) {
  if (!id) {
    currentProject.value = null
    try { localStorage.removeItem(STORAGE_KEY) } catch {}
    notifyChange({ id: null })
    return
  }
  await loadCurrentById(id, { force: true })
  try { localStorage.setItem(STORAGE_KEY, String(id)) } catch {}
  notifyChange({ id })
}

async function ensureProjectContext() {
  if (projectReady.value && projectList.value.length) return
  await loadProjectList()
  let saved = null
  try { saved = localStorage.getItem(STORAGE_KEY) } catch {}
  let id = saved
  // 本地没存 + 有列表 → 选第一个进行中项目，否则第一个
  if (!id && projectList.value.length) {
    const firstActive = projectList.value.find((p) => p.status === 'active')
    id = (firstActive || projectList.value[0])?.id
  }
  await loadCurrentById(id)
}

function createAndSelect(form) {
  return createProject(form).then(async (p) => {
    await loadProjectList()
    await setCurrentProject(p.id)
    return p
  })
}

function onChange(fn) {
  listeners.add(fn)
  return () => listeners.delete(fn)
}

// —— Vue provide / inject 钩子（用于组件内 useProject()） ——
const PROVIDE_KEY = 'MOX_PROJECT_CONTEXT'

export function provideProjectContext() {
  const ctx = {
    currentProject,
    projectList,
    listLoading,
    projectReady,
    setCurrentProject,
    ensureProjectContext,
    createAndSelect,
    loadProjectList,
    onChange
  }
  provide(PROVIDE_KEY, ctx)
  // App 级启动：初始化一次
  ensureProjectContext().catch(() => {})
  return ctx
}

export function useProject() {
  const existing = inject(PROVIDE_KEY, null)
  if (existing) return existing
  // 未 provide 也能 import 正常使用（非组件场景）
  return {
    currentProject,
    projectList,
    listLoading,
    projectReady,
    setCurrentProject,
    ensureProjectContext,
    createAndSelect,
    loadProjectList,
    onChange
  }
}

// 当项目本身变更时（比如在 ProjectsView 中编辑了名字），通知全局刷新
window.addEventListener?.('mox:project-updated', async (e) => {
  const id = e?.detail?.id || currentProject.value?.id
  if (!id) { await loadProjectList(); return }
  await Promise.all([loadProjectList(), loadCurrentById(id, { force: true })])
  notifyChange({ id, updated: true })
})
window.addEventListener?.('mox:project-deleted', async (e) => {
  const id = e?.detail?.id
  if (id && currentProject.value?.id === id) {
    try { localStorage.removeItem(STORAGE_KEY) } catch {}
    currentProject.value = null
  }
  await loadProjectList()
  if (!currentProject.value && projectList.value.length) {
    const firstActive = projectList.value.find((p) => p.status === 'active')
    await setCurrentProject((firstActive || projectList.value[0]).id)
  }
  notifyChange({ id, deleted: true })
})
