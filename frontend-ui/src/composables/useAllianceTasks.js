import { ref } from 'vue'

const activeStates = new Set(['pending', 'planning', 'ready', 'running'])
export const taskActions = {
  resume: new Set(['pending', 'ready', 'paused']),
  pause: new Set(['running']),
  cancel: new Set(['pending', 'planning', 'ready', 'running', 'paused']),
}

export function useAllianceTasks(api, { intervalMs = 2000 } = {}) {
  const tasks = ref([]), selectedTask = ref(null), tasksLoading = ref(false), tasksError = ref('')
  const logs = ref([]), fusionResult = ref(null), dagNodesData = ref([]), dagEdgesData = ref([])
  const logsLoading = ref(false), fusionLoading = ref(false), dagLoading = ref(false)
  const logsError = ref(''), fusionError = ref(''), dagError = ref(''), pollError = ref('')
  const actionPending = ref(false)
  let selectionVersion = 0, listVersion = 0, disposed = false, timer = null, polling = false, pollEnabled = false
  const message = error => error?.message || '服务暂时不可用，请重试'

  async function selectTask(task) {
    const version = ++selectionVersion
    selectedTask.value = task
    logs.value = []; fusionResult.value = null; dagNodesData.value = []; dagEdgesData.value = []
    logsError.value = ''; fusionError.value = ''; dagError.value = ''
    logsLoading.value = !!task; dagLoading.value = !!task; fusionLoading.value = task?.status === 'completed'
    if (!task) return
    const current = () => !disposed && version === selectionVersion
    await Promise.all([
      api.getAllianceTaskLogs(task.id).then(value => { if (current()) logs.value = value })
        .catch(error => { if (current()) logsError.value = message(error) })
        .finally(() => { if (current()) logsLoading.value = false }),
      api.getAllianceTaskDag(task.id).then(value => {
        if (current()) { dagNodesData.value = value.nodes; dagEdgesData.value = value.edges }
      }).catch(error => { if (current()) dagError.value = message(error) })
        .finally(() => { if (current()) dagLoading.value = false }),
      task.status === 'completed' ? api.getAllianceFusionResult(task.id)
        .then(value => { if (current()) fusionResult.value = value })
        .catch(error => { if (current()) fusionError.value = message(error) })
        .finally(() => { if (current()) fusionLoading.value = false }) : Promise.resolve(),
    ])
  }

  async function loadTasks(preferredId) {
    const version = ++listVersion
    tasksLoading.value = true; tasksError.value = ''
    try {
      const items = await api.getAllianceTasks()
      if (disposed || version !== listVersion) return
      tasks.value = items
      const id = preferredId || selectedTask.value?.id
      await selectTask(items.find(task => task.id === id) || items[0] || null)
    } catch (error) {
      if (!disposed && version === listVersion) tasksError.value = message(error)
      // Preserve the last successful list during temporary outages.
    } finally { if (!disposed && version === listVersion) tasksLoading.value = false }
  }

  async function refreshTask(task) {
    const status = await api.getAllianceTaskStatus(task.id)
    if (disposed) return
    const current = tasks.value.find(item => item.id === task.id)
    if (!current) return
    current.status = status.status
    current.progress = status.progress
    if (selectedTask.value?.id === task.id) await selectTask(current)
  }

  async function performAction(action) {
    const task = selectedTask.value
    if (actionPending.value || !task || !taskActions[action]?.has(task.status)) return false
    actionPending.value = true
    try {
      const operation = { resume: api.resumeAllianceTask, pause: api.pauseAllianceTask, cancel: api.cancelAllianceTask }[action]
      await operation(task.id)
      // Never update whichever task happens to be selected after the request.
      try { await refreshTask(task) }
      catch (error) { pollError.value = `操作已受理，状态同步失败：${message(error)}` }
      return true
    } finally { actionPending.value = false }
  }

  async function poll() {
    if (polling || disposed || actionPending.value) return
    polling = true
    pollError.value = ''
    try {
      // Bounded, sequential requests; the next cycle starts after this finishes.
      for (const task of [...tasks.value]) {
        if (disposed) break
        if (!activeStates.has(task.status)) continue
        try { await refreshTask(task) }
        catch (error) { if (!disposed) pollError.value = `状态更新暂时中断，保留上次结果：${message(error)}` }
      }
    } finally { polling = false }
  }

  function startPolling() {
    if (pollEnabled || disposed) return
    pollEnabled = true
    const tick = async () => {
      await poll()
      if (!disposed && pollEnabled) timer = setTimeout(tick, intervalMs)
    }
    timer = setTimeout(tick, intervalMs)
  }

  function dispose() {
    disposed = true; pollEnabled = false; selectionVersion++; listVersion++
    clearTimeout(timer)
  }

  return { tasks, selectedTask, tasksLoading, tasksError, logs, fusionResult, dagNodesData, dagEdgesData,
    logsLoading, fusionLoading, dagLoading, logsError, fusionError, dagError, pollError, actionPending,
    selectTask, loadTasks, performAction, poll, startPolling, dispose }
}
