// 会话状态：列表 / 详情（含完整 messages）/ 统计 / 检索四区各自记账。
// 四区是四份独立接口，一处失败不得连带清空已取到的部分（与图谱面同一原则）。
//
// 三条必须写下来的后端事实，决定了这里的动作形状：
// 1. 只有 GET /sessions/:id 返回 messages，列表返回的是 message_count，
//    所以"打开会话"必须补一次详情请求，不能指望列表里带正文；
// 2. POST /sessions/:id/messages 的响应只有那条消息，后端不回会话快照，
//    追加成功后由本 store 把它并进线程并推出 last_active_at；
// 3. PUT 是合并式更新，metadata 只能逐键写入、**无法删除**（experts_session.rs:390-394 只有 insert），
//    所以改动一律先经 contract/sessionUpdatePatch 差分，未改的键不进请求体。
import { defineStore } from 'pinia'
import { computed, reactive, ref } from 'vue'
import { allianceApi } from '@/modules/expert-alliance/api'
import {
  ARCHIVE_STATUS,
  SESSION_PAGE,
  appendMessageProblem,
  createSessionProblem,
  exportFileName,
  exportText,
  isNotFound,
  metadataRowsToObject,
  normalizeFilters,
  semanticSearchProblem,
  sessionPageCount,
  sessionStatusProblem,
  sessionUpdatePatch,
  similarSearchProblem
} from '@/modules/expert-alliance/contract'

const emptyList = () => ({ items: [], total: 0, page: SESSION_PAGE.defaultPage, pageSize: SESSION_PAGE.defaultSize })

export const useAllianceSessionsStore = defineStore('allianceSessions', () => {
  const api = allianceApi

  const filters = reactive(normalizeFilters({}))
  const list = ref(emptyList())
  const stats = ref(null)
  const selectedId = ref('')
  const detail = ref(null)
  const similar = ref(null)
  const semantic = ref(null)
  /** 本次语义检索自己用了哪些过滤条件：后端不回显，只能由发起方记住 */
  const semanticFilter = ref({ sessionType: '', expertId: '' })
  const exportResult = ref(null)

  // userId 不在创建后清空：一次会话窗里通常都是同一个人发起，且它是过滤框同一根线上的键
  const createDraft = reactive({ title: '', sessionType: 'single', userId: '', topic: '', expertIds: [], tags: [] })
  const composer = reactive({ role: 'user', content: '', msgType: 'text', senderId: '', senderName: '', rating: null })
  const editDraft = reactive({ title: '', status: '', topic: '', tags: [] })
  const metaRows = ref([])
  const similarDraft = reactive({ query: '', topK: null, minScore: null })
  const semanticDraft = reactive({ query: '', topK: null, sessionType: '', expertId: '' })

  const loading = reactive({
    list: false, stats: false, detail: false, create: false, append: false,
    update: false, archive: false, remove: false, export: false, similar: false, semantic: false
  })
  const error = reactive({
    list: '', stats: '', detail: '', create: '', append: '',
    update: '', archive: '', remove: '', export: '', similar: '', semantic: ''
  })
  /** 一次性的操作回执（归档/删除/导出这类没有查询语义的动作，只能靠它把结果说给用户） */
  const notice = ref('')

  const totalPages = computed(() => sessionPageCount(list.value.total, filters.pageSize))
  const messages = computed(() => detail.value?.messages || [])
  const archived = computed(() => !!detail.value?.archivedAt)
  const composerProblem = computed(() => appendMessageProblem(composer))
  const createProblem = computed(() => createSessionProblem(createDraft))
  const editStatusProblem = computed(() => sessionStatusProblem(editDraft.status))

  function setFilter(key, value) {
    filters[key] = value
    // 除翻页本身以外的任何过滤改动都回到第 1 页：后端的 offset 是按过滤后集合算的
    if (key !== 'page') filters.page = SESSION_PAGE.defaultPage
  }

  async function loadList() {
    loading.list = true
    error.list = ''
    try {
      list.value = await api.listSessions({ ...filters })
    } catch (e) {
      error.list = e?.msg || e?.message || '会话列表获取失败'
      list.value = emptyList()
    } finally {
      loading.list = false
    }
  }

  async function loadStats() {
    loading.stats = true
    error.stats = ''
    try {
      stats.value = await api.sessionStats()
    } catch (e) {
      error.stats = e?.msg || e?.message || '会话统计获取失败'
      stats.value = null
    } finally {
      loading.stats = false
    }
  }

  /** 打开会话 = 补一次详情。列表项没有正文，这里不复制粘贴一份假线程。 */
  async function selectSession(id) {
    if (!id) return
    selectedId.value = id
    loading.detail = true
    error.detail = ''
    similar.value = null
    similarDraft.query = ''
    exportResult.value = null
    try {
      detail.value = await api.getSession(id)
      syncEditDraft()
    } catch (e) {
      error.detail = e?.msg || e?.message || '会话详情获取失败'
      detail.value = null
      // 后端对未知 id 返回 404 "session not found: {id}"，选中态必须一起清掉，
      // 否则右侧面板停在上一条，看起来像"点没反应"。
      if (isNotFound(e)) selectedId.value = ''
    } finally {
      loading.detail = false
    }
  }

  function syncEditDraft() {
    editDraft.title = detail.value?.title || ''
    editDraft.status = detail.value?.status || ''
    editDraft.topic = detail.value?.topic || ''
    editDraft.tags = [...(detail.value?.tags || [])]
    metaRows.value = (detail.value?.metadata || []).map((m) => ({ key: m.key, value: stringifyMeta(m.value) }))
  }

  function stringifyMeta(v) {
    if (v === null || v === undefined) return ''
    return typeof v === 'string' ? v : JSON.stringify(v)
  }

  // ── metadata 键值行：后端合并式写入，所以只能新增/改值，删除是本地假象 ──
  function addMetaRow() {
    metaRows.value = [...metaRows.value, { key: '', value: '' }]
  }

  function setMetaRow(index, field, value) {
    const rows = metaRows.value.map((r, i) => (i === index ? { ...r, [field]: value } : r))
    metaRows.value = rows
  }

  function removeMetaRow(index) {
    const row = metaRows.value[index]
    metaRows.value = metaRows.value.filter((_, i) => i !== index)
    // 已入库的键删不掉：这里如实说出来，而不是让用户以为删成功了
    if (row?.key && (detail.value?.metadata || []).some((m) => m.key === row.key)) {
      notice.value = `键 ${row.key} 已从表单移除，但后端没有删除 metadata 键的语义，它仍留在会话里；要清掉只能改写为空值后保存`
    }
  }

  async function createSession() {
    if (createProblem.value) {
      error.create = createProblem.value
      return null
    }
    loading.create = true
    error.create = ''
    try {
      const created = await api.createSession({ ...createDraft })
      createDraft.title = ''
      createDraft.topic = ''
      createDraft.expertIds = []
      createDraft.tags = []
      filters.page = SESSION_PAGE.defaultPage
      notice.value = `会话 ${created.id} 已创建，初始状态 active（后端固定，前端无从指定）`
      await loadList()
      await selectSession(created.id)
      return created
    } catch (e) {
      error.create = e?.msg || e?.message || '会话创建失败'
      return null
    } finally {
      loading.create = false
    }
  }

  /** 追加成功后后端只回那条消息，故本 store 负责并线并推出活跃时间 */
  async function appendMessage() {
    if (!selectedId.value) return null
    if (composerProblem.value) {
      error.append = composerProblem.value
      return null
    }
    loading.append = true
    error.append = ''
    try {
      const message = await api.appendSessionMessage(selectedId.value, { ...composer })
      if (detail.value) {
        detail.value.messages = [...(detail.value.messages || []), message]
        detail.value.messageCount = detail.value.messages.length
        detail.value.lastActiveAt = message.createdAt || detail.value.lastActiveAt
      }
      composer.content = ''
      composer.rating = null
      // 列表里的 message_count 已经落后一条，重取一次保持两侧同数
      await loadList()
      return message
    } catch (e) {
      error.append = e?.msg || e?.message || '消息追加失败'
      return null
    } finally {
      loading.append = false
    }
  }

  /** 合并式更新：只有与后端当前值不同的键进请求体 */
  async function saveEdit() {
    if (!detail.value) return null
    if (editStatusProblem.value) {
      error.update = editStatusProblem.value
      return null
    }
    loading.update = true
    error.update = ''
    try {
      const { patch, unremovableMetadataKeys } = sessionUpdatePatch(detail.value, {
        ...editDraft,
        metadata: metadataRowsToObject(metaRows.value)
      })
      if (!Object.keys(patch).length) {
        notice.value = '没有任何改动，未发出请求（后端是合并式更新，原值回写没有意义）'
        return null
      }
      detail.value = await api.updateSession(detail.value.id, patch)
      syncEditDraft()
      notice.value = unremovableMetadataKeys.length
        ? `已更新 ${Object.keys(patch).join('、')}；键 ${unremovableMetadataKeys.join('、')} 想删也删不掉，后端没有该语义`
        : `已更新 ${Object.keys(patch).join('、')}`
      await loadList()
      return detail.value
    } catch (e) {
      error.update = e?.msg || e?.message || '会话更新失败'
      return null
    } finally {
      loading.update = false
    }
  }

  async function archiveSession() {
    if (!detail.value) return null
    loading.archive = true
    error.archive = ''
    try {
      const result = await api.archiveSession(detail.value.id)
      // 归档响应只有三个字段，详情视图按它覆盖状态即可，不必整份重取
      detail.value = {
        ...detail.value,
        status: result.status || ARCHIVE_STATUS,
        archivedAt: result.archivedAt || detail.value.archivedAt,
        messageCount: result.messageCount ?? detail.value.messageCount
      }
      editDraft.status = detail.value.status
      notice.value = `会话已归档（${result.archivedAt || '后端未回时间'}），没有反归档端点，恢复只能把 status 改回 active 保存`
      await loadList()
      await loadStats()
      return result
    } catch (e) {
      error.archive = e?.msg || e?.message || '会话归档失败'
      return null
    } finally {
      loading.archive = false
    }
  }

  async function deleteSession() {
    if (!detail.value) return null
    const id = detail.value.id
    loading.remove = true
    error.remove = ''
    try {
      const result = await api.deleteSession(id)
      if (result.deleted) {
        detail.value = null
        selectedId.value = ''
        similar.value = null
        exportResult.value = null
      }
      notice.value = `会话 ${id} 已删除（后端只回 {deleted, session_id}，没有回收站）`
      await loadList()
      await loadStats()
      return result
    } catch (e) {
      error.remove = e?.msg || e?.message || '会话删除失败'
      return null
    } finally {
      loading.remove = false
    }
  }

  async function exportSession() {
    if (!detail.value) return null
    loading.export = true
    error.export = ''
    try {
      exportResult.value = await api.exportSession(detail.value.id)
      // download_url 恒 null：能给的只有文件名与文本，落地动作发生在浏览器里
      notice.value = `导出内容已就绪（${exportResult.value.messageCount} 条消息），后端不提供下载地址，请复制或存为本地文件`
      return exportResult.value
    } catch (e) {
      error.export = e?.msg || e?.message || '会话导出失败'
      exportResult.value = null
      return null
    } finally {
      loading.export = false
    }
  }

  function exportDownloadName() {
    return exportResult.value ? exportFileName(exportResult.value.sessionId, exportResult.value.exportedAt) : ''
  }

  function exportPayloadText() {
    return exportResult.value ? exportText(exportResult.value.content) : ''
  }

  async function runSimilarSearch() {
    if (!detail.value) return null
    const problem = similarSearchProblem(similarDraft)
    if (problem) {
      error.similar = problem
      similar.value = null
      return null
    }
    loading.similar = true
    error.similar = ''
    try {
      similar.value = await api.sessionSimilarSearch(detail.value.id, { ...similarDraft })
      return similar.value
    } catch (e) {
      error.similar = e?.msg || e?.message || '会话内检索失败'
      similar.value = null
      return null
    } finally {
      loading.similar = false
    }
  }

  async function runSemanticSearch() {
    const problem = semanticSearchProblem(semanticDraft)
    if (problem) {
      error.semantic = problem
      semantic.value = null
      return null
    }
    loading.semantic = true
    error.semantic = ''
    try {
      semantic.value = await api.semanticSearch({ ...semanticDraft })
      semanticFilter.value = { sessionType: semanticDraft.sessionType || '', expertId: semanticDraft.expertId || '' }
      return semantic.value
    } catch (e) {
      error.semantic = e?.msg || e?.message || '全域检索失败'
      semantic.value = null
      return null
    } finally {
      loading.semantic = false
    }
  }

  /** 命中行跳到那条会话：只换选中项，不去改列表过滤条件（后端没有"按会话 id 过滤"这个参数） */
  async function openSemanticResult(row) {
    const id = row?.sessionId
    if (!id) return ''
    await selectSession(id)
    return id
  }

  async function refreshAll() {
    await Promise.all([loadList(), loadStats()])
  }

  return {
    filters, list, stats, selectedId, detail, similar, semantic, semanticFilter, exportResult,
    createDraft, composer, editDraft, metaRows, similarDraft, semanticDraft, loading, error, notice,
    totalPages, messages, archived, composerProblem, createProblem, editStatusProblem,
    setFilter, loadList, loadStats, refreshAll, selectSession, createSession, appendMessage,
    saveEdit, archiveSession, deleteSession, exportSession, exportDownloadName, exportPayloadText,
    runSimilarSearch, runSemanticSearch, openSemanticResult, syncEditDraft,
    addMetaRow, setMetaRow, removeMetaRow
  }
})
