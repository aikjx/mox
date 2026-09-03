/**
 * useWorkspaceData — 工作台域数据加载 Composable
 * =====================================================
 * 企业级模块化封装：KPI / 通知 / 成员 / 阶段 / 文件 / 历史
 * 遵循项目既有 composable 模式（useWhiteboard / useGraphCanvas / useTaskOrchestration）
 *
 * 设计原则：
 * - 单一职责：仅负责工作台域数据获取与状态管理
 * - 真实数据：所有数据必须来自真实 API 调用，失败时显示空状态 + 错误提示
 * - 加载态透明：每个数据源独立 loading，便于 UI 精细化控制
 * - 可组合：返回 ref + 方法，调用方按需解构
 */
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import {
  getUnreadCount,
  getWorkspaceKpi,
  getProjectMembers,
  getProjectPhases,
  getProjectFiles,
  uploadProjectFile,
  getFilePreview,
  getFileDownload,
  saveWhiteboard,
  getWorkspaceHistory
} from '@/api/workspace.api.js'

// ========== 工具函数 ==========
function unwrap(res) {
  if (res && typeof res === 'object') {
    if (Array.isArray(res.data)) return res.data
    if (Array.isArray(res.list)) return res.list
    if (Array.isArray(res.items)) return res.items
    if (res.data && typeof res.data === 'object') return res.data
  }
  return res
}

function nowTime() {
  const d = new Date()
  return d.getHours().toString().padStart(2, '0') + ':' + d.getMinutes().toString().padStart(2, '0')
}

/**
 * useWorkspaceData
 * @param {Ref} currentProject - 当前项目 ref
 */
export function useWorkspaceData(currentProject) {
  // ========== 状态 ==========
  const notifCount = ref(0)
  const hasNotifications = ref(false)
  const kpiCards = ref([])
  const kpiLoading = ref(false)
  const collabMembers = ref([])
  const membersLoading = ref(false)
  const projectPhases = ref([])
  const currentProjectPhase = ref(1)
  const phasesLoading = ref(false)
  const sharedFiles = ref([])
  const filesLoading = ref(false)
  const historyEvents = ref([])
  const historyLoading = ref(false)
  // 错误状态
  const kpiError = ref('')
  const membersError = ref('')
  const phasesError = ref('')
  const filesError = ref('')
  const historyError = ref('')

  // ========== 1. 通知未读数 ==========
  async function loadUnreadCount() {
    try {
      const res = await getUnreadCount()
      const count = typeof res === 'number' ? res : (res?.data?.count ?? res?.count ?? 0)
      notifCount.value = count
      hasNotifications.value = count > 0
    } catch (e) {
      console.warn('[workspace] 加载通知未读数失败:', e?.message)
      notifCount.value = 0
      hasNotifications.value = false
      ElMessage.error('通知未读数加载失败：' + (e?.message || '未知错误'))
    }
  }

  // ========== 2. KPI 聚合统计 ==========
  async function loadKpi() {
    kpiLoading.value = true
    kpiError.value = ''
    try {
      const res = await getWorkspaceKpi()
      const data = unwrap(res)
      if (data && Array.isArray(data) && data.length) {
        kpiCards.value = data.map((item, idx) => ({
          key: item.key || item.metric || `kpi-${idx}`,
          icon: item.icon || '📊',
          value: item.value ?? item.count ?? 0,
          label: item.label || item.name || `指标${idx + 1}`,
          trend: item.trend ?? item.change ?? 0,
          gradient: item.gradient || 'linear-gradient(135deg, #6366f1, #06b6d4)'
        }))
      } else {
        kpiCards.value = []
      }
    } catch (e) {
      console.warn('[workspace] 加载 KPI 失败:', e?.message)
      kpiError.value = e?.message || 'KPI 数据加载失败'
      kpiCards.value = []
      ElMessage.error('KPI 数据加载失败：' + (e?.message || '未知错误'))
    } finally {
      kpiLoading.value = false
    }
  }

  // ========== 3. 项目成员 ==========
  async function loadMembers() {
    membersLoading.value = true
    membersError.value = ''
    try {
      const pid = currentProject?.value
      const res = await getProjectMembers(pid)
      const list = unwrap(res)
      if (Array.isArray(list) && list.length) {
        collabMembers.value = list.map((m, idx) => ({
          id: m.id || m.user_id || `member-${idx}`,
          name: m.name || m.username || '成员',
          avatar: m.avatar || (m.name ? m.name.charAt(0) : '👤'),
          color: m.color || 'linear-gradient(135deg, #6366f1, #06b6d4)',
          status: m.status || 'active',
          role: m.role || (m.role === 'host' ? 'host' : 'expert')
        }))
      } else {
        collabMembers.value = []
      }
    } catch (e) {
      console.warn('[workspace] 加载项目成员失败:', e?.message)
      membersError.value = e?.message || '项目成员加载失败'
      collabMembers.value = []
      ElMessage.error('项目成员加载失败：' + (e?.message || '未知错误'))
    } finally {
      membersLoading.value = false
    }
  }

  // ========== 4. 项目阶段 ==========
  async function loadPhases() {
    phasesLoading.value = true
    phasesError.value = ''
    try {
      const pid = currentProject?.value
      const res = await getProjectPhases(pid)
      const data = unwrap(res)
      if (data) {
        const phases = Array.isArray(data) ? data : (data.phases || data.list || [])
        if (phases.length) {
          projectPhases.value = phases.map((p, idx) => ({
            key: p.key || p.id || p.code || `phase-${idx}`,
            label: p.label || p.name || `阶段${idx + 1}`
          }))
        } else {
          projectPhases.value = []
        }
        const currentIdx = data.current_phase ?? data.currentPhase ?? data.current_index ?? 1
        currentProjectPhase.value = typeof currentIdx === 'number' ? currentIdx : 1
      } else {
        projectPhases.value = []
      }
    } catch (e) {
      console.warn('[workspace] 加载项目阶段失败:', e?.message)
      phasesError.value = e?.message || '项目阶段加载失败'
      projectPhases.value = []
      ElMessage.error('项目阶段加载失败：' + (e?.message || '未知错误'))
    } finally {
      phasesLoading.value = false
    }
  }

  function jumpToPhase(idx) {
    currentProjectPhase.value = idx
    const phase = projectPhases.value[idx]
    appendHistory('phase', `进入「${phase?.label || ''}」阶段`, '项目阶段已切换')
    ElMessage.info(`已切换到「${phase?.label || ''}」阶段`)
  }

  // ========== 5. 项目文件 ==========
  async function loadFiles() {
    filesLoading.value = true
    filesError.value = ''
    try {
      const pid = currentProject?.value
      const res = await getProjectFiles(pid, { limit: 50 })
      const list = unwrap(res)
      if (Array.isArray(list) && list.length) {
        sharedFiles.value = list.map((f, idx) => ({
          id: f.id || f.file_id || `file-${idx}`,
          name: f.name || f.filename || '未命名文件',
          type: f.type || f.mime_type || inferFileType(f.name),
          size: f.size || formatSize(f.size_bytes || f.size),
          uploader: f.uploader || f.uploaded_by || '未知',
          time: f.time || f.created_at || nowTime()
        }))
      } else {
        sharedFiles.value = []
      }
    } catch (e) {
      console.warn('[workspace] 加载项目文件失败:', e?.message)
      filesError.value = e?.message || '项目文件加载失败'
      sharedFiles.value = []
      ElMessage.error('项目文件加载失败：' + (e?.message || '未知错误'))
    } finally {
      filesLoading.value = false
    }
  }

  async function handleFileUpload(file) {
    try {
      const pid = currentProject?.value
      const res = await uploadProjectFile(pid, file)
      const uploaded = unwrap(res)
      const newFile = {
        id: uploaded?.id || `file-${Date.now()}`,
        name: file.name,
        type: inferFileType(file.name),
        size: formatSize(file.size),
        uploader: '我',
        time: nowTime()
      }
      sharedFiles.value.unshift(newFile)
      appendHistory('file', '上传文件', file.name)
      ElMessage.success(`文件「${file.name}」上传成功`)
      return true
    } catch (e) {
      console.warn('[workspace] 文件上传失败:', e?.message)
      ElMessage.error(`文件上传失败：${e?.message || '未知错误'}`)
      return false
    }
  }

  async function previewFile(file) {
    if (file.type === 'image') {
      ElMessage.info(`正在预览图片：${file.name}`)
      return
    }
    try {
      await getFilePreview(file.id)
      ElMessage.info(`正在打开文档：${file.name}`)
    } catch (e) {
      console.warn('[workspace] 文件预览失败:', e?.message)
      ElMessage.error(`文件预览失败：${e?.message || '未知错误'}`)
    }
  }

  async function downloadFile(file) {
    try {
      const blob = await getFileDownload(file.id)
      const url = window.URL.createObjectURL(new Blob([blob]))
      const a = document.createElement('a')
      a.href = url
      a.download = file.name
      a.click()
      window.URL.revokeObjectURL(url)
      ElMessage.success(`开始下载：${file.name}`)
    } catch (e) {
      console.warn('[workspace] 文件下载失败:', e?.message)
      ElMessage.error(`文件下载失败：${e?.message || '未知错误'}`)
    }
  }

  // ========== 6. 历史记录 ==========
  async function loadHistory() {
    historyLoading.value = true
    historyError.value = ''
    try {
      const res = await getWorkspaceHistory({ limit: 50 })
      const list = unwrap(res)
      if (Array.isArray(list) && list.length) {
        historyEvents.value = list.map((h, idx) => ({
          id: h.id || h.event_id || `h-${idx}`,
          type: h.type || h.event_type || 'message',
          title: h.title || h.event_title || '事件',
          description: h.description || h.detail || '',
          time: h.time || h.created_at || nowTime()
        }))
      } else {
        historyEvents.value = []
      }
    } catch (e) {
      console.warn('[workspace] 加载历史记录失败:', e?.message)
      historyError.value = e?.message || '历史记录加载失败'
      historyEvents.value = []
      ElMessage.error('历史记录加载失败：' + (e?.message || '未知错误'))
    } finally {
      historyLoading.value = false
    }
  }

  function appendHistory(type, title, description) {
    historyEvents.value.unshift({ id: 'h-' + Date.now(), type, title, description, time: nowTime() })
    if (historyEvents.value.length > 50) historyEvents.value = historyEvents.value.slice(0, 50)
  }

  function jumpToHistory(item) {
    if (item.type === 'file') {
      // 由调用方处理 tab 切换
    } else if (item.type === 'whiteboard') {
      // 由调用方处理
    }
    ElMessage.info(`跳转到：${item.title}`)
  }

  // ========== 7. 白板持久化 ==========
  async function persistWhiteboard(sessionId, data) {
    try {
      await saveWhiteboard(sessionId, data)
      return true
    } catch (e) {
      console.warn('[workspace] 白板持久化失败:', e?.message)
      return false
    }
  }

  // ========== 批量加载 ==========
  async function loadAllWorkspaceData() {
    await Promise.allSettled([
      loadUnreadCount(),
      loadKpi(),
      loadMembers(),
      loadPhases(),
      loadFiles(),
      loadHistory()
    ])
  }

  // 项目切换时重新加载项目相关数据
  async function reloadOnProjectChange() {
    await Promise.allSettled([
      loadMembers(),
      loadPhases(),
      loadFiles()
    ])
  }

  return {
    // 状态
    notifCount, hasNotifications,
    kpiCards, kpiLoading, kpiError,
    collabMembers, membersLoading, membersError,
    projectPhases, currentProjectPhase, phasesLoading, phasesError,
    sharedFiles, filesLoading, filesError,
    historyEvents, historyLoading, historyError,
    // 方法
    loadUnreadCount, loadKpi, loadMembers, loadPhases, loadFiles, loadHistory,
    loadAllWorkspaceData, reloadOnProjectChange,
    jumpToPhase, handleFileUpload, previewFile, downloadFile,
    appendHistory, jumpToHistory, persistWhiteboard
  }
}

// ========== 辅助函数 ==========
function inferFileType(name) {
  if (!name) return 'file'
  const ext = name.split('.').pop().toLowerCase()
  if (['pdf'].includes(ext)) return 'pdf'
  if (['doc', 'docx', 'txt', 'md'].includes(ext)) return 'doc'
  if (['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp'].includes(ext)) return 'image'
  if (['xls', 'xlsx', 'csv'].includes(ext)) return 'excel'
  if (['ppt', 'pptx'].includes(ext)) return 'ppt'
  if (['zip', 'rar', '7z', 'tar', 'gz'].includes(ext)) return 'archive'
  return 'file'
}

function formatSize(bytes) {
  if (!bytes || typeof bytes !== 'number') return '未知'
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB'
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + ' MB'
  return (bytes / 1073741824).toFixed(2) + ' GB'
}
