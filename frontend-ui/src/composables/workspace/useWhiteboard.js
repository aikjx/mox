/**
 * 白板功能 Composable
 * 职责：便签、文本框、连线、自由画笔的状态管理与交互逻辑
 */
import { ref } from 'vue'
import { ElMessage } from 'element-plus'

export function useWhiteboard(addHistoryEvent) {
  const whiteboardRef = ref(null)
  const activeWbTool = ref('select')
  const activeWbColor = ref('#7c3aed')
  const wbNotes = ref([])
  const wbTexts = ref([])
  const wbLines = ref([])
  const wbDrawPaths = ref([])
  const wbCurrentPath = ref('')
  const wbViewBox = ref('0 0 800 400')

  let wbDrawing = false
  let wbDragElement = null
  let wbDragOffset = { x: 0, y: 0 }
  let wbPathPoints = []

  function selectWbTool(tool) { activeWbTool.value = tool }

  function onWbMouseDown(e) {
    const rect = whiteboardRef.value?.getBoundingClientRect()
    if (!rect) return
    const x = e.clientX - rect.left
    const y = e.clientY - rect.top
    if (activeWbTool.value === 'note') { addWbNote('新便签', '', x, y); activeWbTool.value = 'select' }
    else if (activeWbTool.value === 'text') { addWbText('新文本', x, y); activeWbTool.value = 'select' }
    else if (activeWbTool.value === 'pen') { wbDrawing = true; wbPathPoints = [{ x, y }]; wbCurrentPath.value = `M ${x} ${y}` }
    else if (activeWbTool.value === 'line') { wbDrawing = true; wbPathPoints = [{ x, y }] }
  }

  function onWbMouseMove(e) {
    const rect = whiteboardRef.value?.getBoundingClientRect()
    if (!rect) return
    const x = e.clientX - rect.left
    const y = e.clientY - rect.top
    if (wbDragElement) { wbDragElement.x = x - wbDragOffset.x; wbDragElement.y = y - wbDragOffset.y }
    if (wbDrawing && activeWbTool.value === 'pen' && wbPathPoints.length > 0) {
      wbPathPoints.push({ x, y })
      wbCurrentPath.value = wbPathPoints.map((p, i) => i === 0 ? `M ${p.x} ${p.y}` : `L ${p.x} ${p.y}`).join(' ')
    }
  }

  function onWbMouseUp() {
    if (wbDrawing && activeWbTool.value === 'pen' && wbPathPoints.length > 1) {
      wbDrawPaths.value.push({ d: wbCurrentPath.value, color: activeWbColor.value })
      addHistoryEvent?.('whiteboard', '添加画笔路径', '白板内容已更新')
    }
    wbDrawing = false; wbDragElement = null; wbCurrentPath.value = ''; wbPathPoints = []
  }

  function addWbNote(title, content = '', x = 100, y = 80) {
    wbNotes.value.push({ id: 'note-' + Date.now(), title, content, x, y, color: activeWbColor.value + '20' })
    addHistoryEvent?.('whiteboard', `添加便签「${title}」`, '白板内容已更新')
  }

  function startDragNote(e, note) {
    if (activeWbTool.value === 'eraser') { deleteWbNote(note.id); return }
    if (activeWbTool.value !== 'select') return
    const rect = whiteboardRef.value?.getBoundingClientRect()
    if (!rect) return
    wbDragOffset.x = e.clientX - rect.left - note.x
    wbDragOffset.y = e.clientY - rect.top - note.y
    wbDragElement = note
  }

  function deleteWbNote(id) {
    const idx = wbNotes.value.findIndex(n => n.id === id)
    if (idx >= 0) { wbNotes.value.splice(idx, 1); addHistoryEvent?.('whiteboard', '删除便签', '白板内容已更新') }
  }

  function updateNoteContent(e, note) { note.content = e.target.innerText }

  function addWbText(content, x = 100, y = 100) {
    wbTexts.value.push({ id: 'text-' + Date.now(), content, x, y, color: activeWbColor.value })
    addHistoryEvent?.('whiteboard', '添加文本框', '白板内容已更新')
  }

  function startDragText(e, text) {
    if (activeWbTool.value === 'eraser') { deleteWbText(text.id); return }
    if (activeWbTool.value !== 'select') return
    const rect = whiteboardRef.value?.getBoundingClientRect()
    if (!rect) return
    wbDragOffset.x = e.clientX - rect.left - text.x
    wbDragOffset.y = e.clientY - rect.top - text.y
    wbDragElement = text
  }

  function deleteWbText(id) {
    const idx = wbTexts.value.findIndex(t => t.id === id)
    if (idx >= 0) { wbTexts.value.splice(idx, 1); addHistoryEvent?.('whiteboard', '删除文本框', '白板内容已更新') }
  }

  function updateTextContent(e, text) { text.content = e.target.innerText }

  function clearWhiteboard() {
    wbNotes.value = []; wbTexts.value = []; wbLines.value = []; wbDrawPaths.value = []
    addHistoryEvent?.('whiteboard', '清空画布', '白板已清空')
    ElMessage.success('白板已清空')
  }

  function saveWhiteboard(activeSession) {
    const data = { notes: wbNotes.value, texts: wbTexts.value, lines: wbLines.value, drawPaths: wbDrawPaths.value }
    if (activeSession?.value) localStorage.setItem('wb_' + activeSession.value.id, JSON.stringify(data))
    addHistoryEvent?.('whiteboard', '保存白板内容', '白板已保存')
    ElMessage.success('白板内容已保存')
  }

  return {
    whiteboardRef, activeWbTool, activeWbColor, wbNotes, wbTexts, wbLines,
    wbDrawPaths, wbCurrentPath, wbViewBox,
    selectWbTool, onWbMouseDown, onWbMouseMove, onWbMouseUp,
    addWbNote, startDragNote, deleteWbNote, updateNoteContent,
    addWbText, startDragText, deleteWbText, updateTextContent,
    clearWhiteboard, saveWhiteboard
  }
}
