/**
 * 任务编排 Composable
 * 职责：任务智能拆解、子任务CRUD、专家分配、任务执行、甘特图布局
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'

export function useTaskOrchestration(experts, expertColor, expertEmoji, addHistoryEvent, addOrchMessage) {
  const taskOrchestration = reactive({
    originalTask: '', subtasks: [], executionMode: 'auto',
    progress: { total: 0, completed: 0, inProgress: 0, failed: 0, percentage: 0 }
  })
  const decomposing = ref(false)
  const orchIsRunning = ref(false)
  const activeSubtaskId = ref(null)
  const timelineView = ref('gantt')
  const draggingTaskId = ref(null)
  const dragOverTaskId = ref(null)
  const draggingExpert = ref(null)
  const expertDragOverTaskId = ref(null)
  const ganttSlotMinutes = ref(15)

  function getExpertById(id) { return experts?.value?.find(e => e.id === id) }
  function getSubtaskIndex(id) { return taskOrchestration.subtasks.findIndex(t => t.id === id) }
  function expertLoad(expertId) {
    return taskOrchestration.subtasks.filter(t =>
      t.expertIds?.includes(expertId) && ['inProgress', 'reviewing', 'pending', 'waiting'].includes(t.status)
    ).length
  }
  function generateTaskId() { return 'task-' + Date.now() + '-' + Math.random().toString(36).substr(2, 6) }

  async function decomposeTask() {
    if (!taskOrchestration.originalTask.trim()) { ElMessage.warning('请先输入任务描述'); return }
    decomposing.value = true
    try {
      await new Promise(resolve => setTimeout(resolve, 1500))
      const taskDesc = taskOrchestration.originalTask
      const subtasks = generateMockSubtasks(taskDesc)
      taskOrchestration.subtasks = subtasks
      updateGanttLayout()
      ElMessage.success(`已智能拆解为 ${subtasks.length} 个子任务`)
      addHistoryEvent?.('task', '任务智能拆解', `将「${taskDesc.substring(0, 20)}...」拆解为 ${subtasks.length} 个子任务`)
    } catch (e) { console.error('[orchestration] 任务拆解失败:', e); ElMessage.error('任务拆解失败，请重试') }
    finally { decomposing.value = false }
  }

  function generateMockSubtasks(taskDesc) {
    const templates = [
      { title: '需求分析与定义', desc: '深入分析任务需求，明确目标、边界和验收标准', type: 'requirement', time: 15, priority: 'high' },
      { title: '方案设计与架构规划', desc: '设计整体解决方案，规划技术架构和实现路径', type: 'architecture', time: 20, priority: 'high' },
      { title: '核心算法/模型研发', desc: '研发核心算法或AI模型，实现关键功能', type: 'algorithm', time: 30, priority: 'high' },
      { title: '数据处理与准备', desc: '数据采集、清洗、标注和预处理工作', type: 'data', time: 25, priority: 'medium' },
      { title: '系统集成与联调', desc: '各模块集成开发，接口联调测试', type: 'architecture', time: 20, priority: 'medium' },
      { title: '质量保障与测试', desc: '功能测试、性能测试、安全审计', type: 'security', time: 15, priority: 'medium' },
      { title: '文档编写与交付', desc: '编写技术文档、用户手册，准备交付物', type: 'requirement', time: 10, priority: 'low' }
    ]
    const numTasks = Math.min(Math.max(Math.floor(taskDesc.length / 30) + 3, 4), 7)
    const selected = templates.slice(0, numTasks)
    return selected.map((tpl, idx) => ({
      id: generateTaskId(), title: tpl.title, description: tpl.desc, priority: tpl.priority,
      status: idx === 0 ? 'pending' : 'waiting', suggestedExpertType: tpl.type, expertIds: [],
      dependencies: idx === 0 ? [] : [selected[0].id || ''], estimatedTime: tpl.time,
      startTime: null, endTime: null, result: '', messages: [], expanded: false, ganttOffset: 0, ganttWidth: 0
    })).map((task, idx, arr) => { if (idx > 0) task.dependencies = [arr[idx - 1].id]; return task })
  }

  function updateGanttLayout() {
    const subtasks = taskOrchestration.subtasks
    if (subtasks.length === 0) return
    const totalMinutes = subtasks.reduce((sum, t) => sum + t.estimatedTime, 0)
    let cumulativeTime = 0
    subtasks.forEach(task => { task.ganttOffset = (cumulativeTime / totalMinutes) * 100; task.ganttWidth = (task.estimatedTime / totalMinutes) * 100; cumulativeTime += task.estimatedTime })
  }

  function addSubtaskManually() {
    const newTask = { id: generateTaskId(), title: '新子任务', description: '请编辑任务描述...', priority: 'medium', status: 'pending', suggestedExpertType: 'custom', expertIds: [], dependencies: [], estimatedTime: 15, startTime: null, endTime: null, result: '', messages: [], expanded: true, ganttOffset: 0, ganttWidth: 0 }
    taskOrchestration.subtasks.push(newTask)
    updateGanttLayout()
    activeSubtaskId.value = newTask.id
  }

  function editSubtask(task) { activeSubtaskId.value = task.id; task.expanded = true; ElMessage.info('请在展开的详情中编辑任务信息') }

  function deleteSubtask(taskId) {
    const idx = getSubtaskIndex(taskId)
    if (idx >= 0) {
      const task = taskOrchestration.subtasks[idx]
      taskOrchestration.subtasks.forEach(t => { t.dependencies = t.dependencies.filter(d => d !== taskId) })
      taskOrchestration.subtasks.splice(idx, 1)
      updateGanttLayout()
      ElMessage.success('子任务已删除')
    }
  }

  function toggleSubtaskExpand(task) { task.expanded = !task.expanded }
  function collapseAllSubtasks() { taskOrchestration.subtasks.forEach(t => t.expanded = false) }
  function selectSubtask(task) { activeSubtaskId.value = task.id }

  // 任务拖拽
  function onTaskDragStart(e, task) { draggingTaskId.value = task.id; e.dataTransfer.effectAllowed = 'move'; e.dataTransfer.setData('text/plain', task.id) }
  function onTaskDragEnd() { draggingTaskId.value = null; dragOverTaskId.value = null }
  function onTaskDragOver(e, task) { if (draggingTaskId.value && draggingTaskId.value !== task.id) dragOverTaskId.value = task.id }
  function onTaskDrop(e, targetTask) {
    const draggedId = draggingTaskId.value
    if (!draggedId || draggedId === targetTask.id) return
    const draggedIdx = getSubtaskIndex(draggedId)
    const targetIdx = getSubtaskIndex(targetTask.id)
    if (draggedIdx >= 0 && targetIdx >= 0) {
      const [removed] = taskOrchestration.subtasks.splice(draggedIdx, 1)
      taskOrchestration.subtasks.splice(targetIdx, 0, removed)
      updateGanttLayout()
      ElMessage.success('任务顺序已调整')
    }
    draggingTaskId.value = null; dragOverTaskId.value = null
  }

  // 专家拖拽分配
  function onExpertDragStart(e, expert) { draggingExpert.value = expert; e.dataTransfer.effectAllowed = 'copy'; e.dataTransfer.setData('text/plain', expert.id) }
  function onExpertDragEnd() { draggingExpert.value = null; expertDragOverTaskId.value = null }
  function onExpertDragOverTask(e, task) { expertDragOverTaskId.value = task.id }
  function onExpertDragLeaveTask() { expertDragOverTaskId.value = null }
  function onExpertDropOnTask(e, task) {
    const expert = draggingExpert.value
    if (!expert) return
    if (!task.expertIds.includes(expert.id)) { task.expertIds.push(expert.id); ElMessage.success(`已将 ${expert.name} 分配到「${task.title}」`) }
    else ElMessage.info('该专家已分配到此任务')
    draggingExpert.value = null; expertDragOverTaskId.value = null
  }

  function unassignExpert(taskId, expertId) {
    const task = taskOrchestration.subtasks.find(t => t.id === taskId)
    if (task) { task.expertIds = task.expertIds.filter(id => id !== expertId); ElMessage.success('已取消专家分配') }
  }

  async function autoAssignExperts() {
    if (taskOrchestration.subtasks.length === 0) { ElMessage.warning('请先创建子任务'); return }
    try {
      await new Promise(resolve => setTimeout(resolve, 1000))
      let assignedCount = 0
      taskOrchestration.subtasks.forEach(task => {
        const matchingExperts = experts?.value?.filter(e => e.type === task.suggestedExpertType && e.status !== 'offline') || []
        if (matchingExperts.length > 0) {
          const bestExpert = matchingExperts.sort((a, b) => expertLoad(a.id) - expertLoad(b.id))[0]
          if (!task.expertIds.includes(bestExpert.id)) { task.expertIds = [bestExpert.id]; assignedCount++ }
        } else {
          const available = experts?.value?.filter(e => e.status !== 'offline') || []
          if (available.length > 0) {
            const bestExpert = available.sort((a, b) => expertLoad(a.id) - expertLoad(b.id))[0]
            if (!task.expertIds.includes(bestExpert.id)) { task.expertIds = [bestExpert.id]; assignedCount++ }
          }
        }
      })
      ElMessage.success(`已智能分配 ${assignedCount} 个任务`)
      addHistoryEvent?.('task', '专家智能分配', `为 ${assignedCount} 个子任务自动匹配了专家`)
    } catch (e) { console.error('[orchestration] 智能分配失败:', e); ElMessage.error('智能分配失败，请重试') }
  }

  function openAssignDialog(task) { /* 分配专家对话框可扩展 */ }

  async function startTaskExecution() {
    if (taskOrchestration.subtasks.length === 0) { ElMessage.warning('请先创建子任务'); return }
    const unassigned = taskOrchestration.subtasks.filter(t => t.expertIds.length === 0)
    if (unassigned.length > 0) { ElMessage.warning(`有 ${unassigned.length} 个任务未分配专家，是否使用智能分配？`); return }
    orchIsRunning.value = true
    ElMessage.success('任务执行已启动')
    addHistoryEvent?.('task', '开始任务执行', '任务编排流程已启动')
    if (taskOrchestration.executionMode === 'auto') executeTasksAuto()
  }

  async function executeTasksAuto() {
    const tasks = [...taskOrchestration.subtasks]
    for (const task of tasks) {
      if (!orchIsRunning.value) break
      const depsCompleted = task.dependencies.every(depId => { const depTask = taskOrchestration.subtasks.find(t => t.id === depId); return depTask?.status === 'completed' })
      if (!depsCompleted) { task.status = 'waiting'; continue }
      task.status = 'inProgress'; task.startTime = Date.now()
      const expert = getExpertById(task.expertIds[0])
      addOrchMessage?.({ role: 'assistant', name: expert?.name || 'AI专家', avatar: expertEmoji?.(expert?.type) || '🤖', color: expertColor?.(expert?.type) || '#6366f1', text: `开始执行「${task.title}」...`, status: 'thinking', phase: 'orchestration' })
      await new Promise(resolve => setTimeout(resolve, Math.min(task.estimatedTime * 50, 2000)))
      const success = Math.random() > 0.1
      if (success) {
        task.status = 'completed'; task.endTime = Date.now()
        task.result = `「${task.title}」执行完成，结果符合预期。\n核心产出：${task.description}的详细方案和实现代码。`
        addOrchMessage?.({ role: 'assistant', name: expert?.name || 'AI专家', avatar: expertEmoji?.(expert?.type) || '🤖', color: expertColor?.(expert?.type) || '#6366f1', text: `✅ **${task.title}** 执行完成\n\n${task.result}`, status: 'done', phase: 'orchestration' })
      } else {
        task.status = 'failed'; task.result = '执行过程中遇到问题，需要人工介入。'
        addOrchMessage?.({ role: 'assistant', name: expert?.name || 'AI专家', avatar: expertEmoji?.(expert?.type) || '🤖', color: '#ef4444', text: `❌ **${task.title}** 执行失败\n\n执行过程中遇到异常，请检查任务配置或重新分配专家。`, status: 'failed', phase: 'orchestration' })
      }
      updateGanttLayout()
    }
    const allDone = taskOrchestration.subtasks.every(t => ['completed', 'failed', 'archived'].includes(t.status))
    if (allDone) {
      orchIsRunning.value = false
      const successCount = taskOrchestration.subtasks.filter(t => t.status === 'completed').length
      ElMessage.success(`任务执行完成：${successCount}/${taskOrchestration.subtasks.length} 成功`)
      addOrchMessage?.({ role: 'system', name: '系统', avatar: '📊', color: '#10b981', text: `🎯 **任务编排完成**\n\n- 总任务数：${taskOrchestration.subtasks.length}\n- 成功完成：${successCount}\n- 失败：${taskOrchestration.subtasks.length - successCount}\n- 完成率：${Math.round((successCount / taskOrchestration.subtasks.length) * 100)}%`, status: 'done', phase: 'orchestration' })
      addHistoryEvent?.('task', '任务编排完成', `完成率 ${Math.round((successCount / taskOrchestration.subtasks.length) * 100)}%`)
    }
  }

  function resetAllTasks() {
    taskOrchestration.subtasks.forEach(task => { task.status = task.dependencies.length > 0 ? 'waiting' : 'pending'; task.startTime = null; task.endTime = null; task.result = '' })
    orchIsRunning.value = false
    updateGanttLayout()
    ElMessage.success('所有任务已重置')
  }

  return {
    taskOrchestration, decomposing, orchIsRunning, activeSubtaskId, timelineView,
    draggingTaskId, dragOverTaskId, draggingExpert, expertDragOverTaskId, ganttSlotMinutes,
    getExpertById, getSubtaskIndex, expertLoad, decomposeTask, addSubtaskManually,
    editSubtask, deleteSubtask, toggleSubtaskExpand, collapseAllSubtasks, selectSubtask,
    onTaskDragStart, onTaskDragEnd, onTaskDragOver, onTaskDrop,
    onExpertDragStart, onExpertDragEnd, onExpertDragOverTask, onExpertDragLeaveTask, onExpertDropOnTask,
    unassignExpert, autoAssignExperts, openAssignDialog, startTaskExecution, resetAllTasks
  }
}
