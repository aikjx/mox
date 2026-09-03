/**
 * 知识库业务逻辑 Composable
 * 封装文档 CRUD、搜索筛选、版本管理、AI 分析等核心业务逻辑
 *
 * 用法：
 *   import { useKnowledgeBase } from '@/composables/useKnowledgeBase'
 *   const { documents, loading, fetchDocuments, viewDocument, ... } = useKnowledgeBase()
 */

import { ref, reactive, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import * as api from '@/api'
import { useProject } from '@/composables/projectContext.js'
import {
  mapDoc,
  DOC_TYPES,
  getTypeLabel,
  getTagType,
  getStatusType,
  getStatusLabel,
  getActionLabel,
  formatTime,
  truncateText,
  getTagSize,
  simpleMarkdownRender
} from '@/utils/knowledgeBase.utils.js'

export function useKnowledgeBase() {
  const { currentProject } = useProject()

  // ========== State ==========
  const documents = ref([])
  const categories = ref([
    {
      id: 'tech', name: '技术文档', count: 24,
      children: [
        { id: 'tech-frontend', name: '前端开发', count: 12 },
        { id: 'tech-backend', name: '后端开发', count: 8 },
        { id: 'tech-architecture', name: '架构设计', count: 4 }
      ]
    },
    {
      id: 'business', name: '业务文档', count: 18,
      children: [
        { id: 'business-requirement', name: '需求文档', count: 10 },
        { id: 'business-design', name: '设计文档', count: 8 }
      ]
    },
    {
      id: 'product', name: '产品文档', count: 12,
      children: [
        { id: 'product-manual', name: '用户手册', count: 6 },
        { id: 'product-api', name: 'API 文档', count: 6 }
      ]
    },
    { id: 'ops', name: '运营文档', count: 9 }
  ])

  const tags = ref([
    { id: 't1', name: 'Vue', count: 15 },
    { id: 't2', name: 'Python', count: 12 },
    { id: 't3', name: '架构', count: 10 },
    { id: 't4', name: 'API', count: 8 },
    { id: 't5', name: '设计模式', count: 7 },
    { id: 't6', name: '性能', count: 6 },
    { id: 't7', name: '安全', count: 5 },
    { id: 't8', name: '测试', count: 4 },
    { id: 't9', name: '部署', count: 3 },
    { id: 't10', name: '数据库', count: 3 }
  ])

  const selectedDoc = ref(null)
  const docVersions = ref([])
  const docHistory = ref([])
  const aiAnalysis = ref(null)
  const entities = ref([])
  const linkedEntities = ref([])
  const stats = ref({ total: 63, categories: 5, versions: 187, analyzed: 41 })

  const loading = ref(false)
  const saving = ref(false)
  const searchQuery = ref('')
  const filterCategory = ref('')
  const filterType = ref('')
  const filterStatus = ref('')
  const filterTag = ref('')
  const filterDateRange = ref(null)
  const viewMode = ref('list')
  const selectedDocs = ref([])

  // 详情 & 对话框状态
  const detailVisible = ref(false)
  const detailTab = ref('content')
  const detailMode = ref('view')
  const editVisible = ref(false)
  const isEditing = ref(false)
  const showLinkDialog = ref(false)
  const compareVisible = ref(false)
  const compareFrom = ref(null)
  const compareTo = ref(null)
  const linkSearchQuery = ref('')
  const searchResults = ref([])

  const editForm = reactive({
    id: null,
    title: '',
    content: '',
    type: 'article',
    category: '',
    tags: [],
    description: '',
    auto_save: false,
    version_note: ''
  })

  const formRules = {
    title: [
      { required: true, message: '请输入文档标题', trigger: 'blur' },
      { min: 2, max: 200, message: '标题长度在 2 到 200 个字符', trigger: 'blur' }
    ],
    type: [{ required: true, message: '请选择文档类型', trigger: 'change' }],
    content: [{ required: true, message: '请输入文档内容', trigger: 'blur' }]
  }

  // ========== Computed ==========

  const statCards = computed(() => [
    { label: '文档总数', value: stats.value.total ?? 0, icon: 'Document', color: '#6366f1', bg: '#eef2ff' },
    { label: '分类数', value: stats.value.categories ?? 0, icon: 'Folder', color: '#06b6d4', bg: '#ecfeff' },
    { label: '版本总数', value: stats.value.versions ?? 0, icon: 'Clock', color: '#10b981', bg: '#ecfdf5' },
    { label: '已分析', value: stats.value.analyzed ?? 0, icon: 'MagicStick', color: '#f59e0b', bg: '#fffbeb' }
  ])

  const filteredDocuments = computed(() => {
    let result = [...documents.value]
    if (searchQuery.value.trim()) {
      const q = searchQuery.value.toLowerCase()
      result = result.filter(
        d =>
          d.title?.toLowerCase().includes(q) ||
          d.description?.toLowerCase().includes(q) ||
          d.tags?.some(t => t.toLowerCase().includes(q))
      )
    }
    if (filterType.value) result = result.filter(d => d.type === filterType.value)
    if (filterStatus.value) result = result.filter(d => d.status === filterStatus.value)
    if (filterTag.value) result = result.filter(d => d.tags?.includes(filterTag.value))
    if (filterCategory.value) result = result.filter(d => d.category === filterCategory.value)
    if (filterDateRange.value?.length === 2) {
      const [start, end] = filterDateRange.value
      result = result.filter(d => {
        const t = new Date(d.updated_at).getTime()
        return t >= start.getTime() && t <= end.getTime()
      })
    }
    return result.sort((a, b) => new Date(b.updated_at) - new Date(a.updated_at))
  })

  const renderedContent = computed(() => {
    if (!selectedDoc.value) return ''
    return simpleMarkdownRender(selectedDoc.value.content || '')
  })

  const renderedCompareFrom = computed(() => {
    if (!compareFrom.value) return ''
    return simpleMarkdownRender(compareFrom.value.content || '')
  })

  const renderedCompareTo = computed(() => {
    if (!compareTo.value) return ''
    return simpleMarkdownRender(compareTo.value.content || '')
  })


  // ========== API Methods ==========

  async function fetchDocuments() {
    loading.value = true
    try {
      const params = {}
      if (searchQuery.value) params.q = searchQuery.value
      if (filterType.value) params.type = filterType.value
      if (filterStatus.value) params.status = filterStatus.value
      if (filterTag.value) params.tag = filterTag.value
      if (filterCategory.value) params.category = filterCategory.value
      if (filterDateRange.value?.length === 2) {
        params.start_date = filterDateRange.value[0].toISOString()
        params.end_date = filterDateRange.value[1].toISOString()
      }
      const data = await api.kbListDocuments(params)
      const list = Array.isArray(data) ? data : (data?.items || data?.documents || [])
      documents.value = list.map(mapDoc)
    } catch (e) {
      documents.value = []
      console.error('[kb] fetchDocuments failed:', e)
    } finally {
      loading.value = false
    }
  }

  async function fetchCategories() {
    try {
      const data = await api.kbGetCategories()
      if (Array.isArray(data) && data.length) {
        categories.value = data
      }
    } catch (e) { categories.value = []; console.error('[kb] fetchCategories failed:', e) }
  }

  async function fetchTags() {
    try {
      const data = await api.kbGetTags()
      if (Array.isArray(data) && data.length) {
        tags.value = data
      }
    } catch (e) { categories.value = []; console.error('[kb] fetchCategories failed:', e) }
  }

  async function fetchStats() {
    try {
      const data = await api.kbGetStats()
      if (data) {
        stats.value = data
      }
    } catch (e) {
      stats.value = {}
      console.error('[kb] fetchStats failed:', e)
    }
  }

  async function fetchVersions(docId) {
    try {
      const data = await api.kbGetVersions(docId)
      docVersions.value = Array.isArray(data) ? data : (data?.items || [])
    } catch {
      docVersions.value = [
        { id: 'v1', version: 1, note: '初始版本', created_at: selectedDoc.value?.updated_at }
      ]
    }
  }

  async function fetchHistory(docId) {
    try {
      const data = await api.kbGetHistory(docId)
      docHistory.value = Array.isArray(data) ? data : (data?.items || [])
    } catch {
      docHistory.value = [
        { action: 'create', user: '张三', detail: '创建文档', created_at: selectedDoc.value?.updated_at }
      ]
    }
  }

  async function loadAiAnalysis(docId) {
    try {
      const data = await api.kbAnalyze(docId)
      aiAnalysis.value = data
      entities.value = data?.entities || []
    } catch {
      aiAnalysis.value = null
    }
  }

  async function analyzeDocument(docId) {
    if (!docId) return
    try {
      const data = await api.kbAnalyze(docId)
      aiAnalysis.value = data
      entities.value = data?.entities || []
      ElMessage.success('AI 分析完成')
    } catch {
      ElMessage.error('AI 分析失败')
    }
  }

  async function viewDocument(doc) {
    if (!doc) return
    selectedDoc.value = doc
    detailTab.value = 'content'
    detailMode.value = 'view'
    detailVisible.value = true
    try {
      const fullDoc = await api.kbGetDocument(doc.id)
      if (fullDoc) {
        selectedDoc.value = mapDoc(fullDoc)
      }
    } catch { /* use existing data */ }
    fetchVersions(doc.id)
    fetchHistory(doc.id)
    if (doc.ai_analyzed) {
      loadAiAnalysis(doc.id)
    }
  }

  function closeDetail() {
    detailVisible.value = false
    selectedDoc.value = null
    docVersions.value = []
    docHistory.value = []
    aiAnalysis.value = null
    entities.value = []
    linkedEntities.value = []
  }

  function handleDetailTabChange(tab) {
    if (tab === 'analysis' && selectedDoc.value?.ai_analyzed && !aiAnalysis.value) {
      loadAiAnalysis(selectedDoc.value.id)
    }
    if (tab === 'history' && docHistory.value.length === 0) {
      fetchHistory(selectedDoc.value?.id)
    }
    if (tab === 'versions' && docVersions.value.length === 0) {
      fetchVersions(selectedDoc.value?.id)
    }
  }

  // ========== CRUD Operations ==========

  function openCreateDialog() {
    isEditing.value = false
    Object.assign(editForm, {
      id: null, title: '', content: '', type: 'article',
      category: '', tags: [], description: '',
      auto_save: false, version_note: ''
    })
    editVisible.value = true
  }

  function openEditDialog(doc) {
    if (!doc) return
    isEditing.value = true
    Object.assign(editForm, {
      id: doc.id,
      title: doc.title,
      content: doc.content || '',
      type: doc.type,
      category: doc.category,
      tags: doc.tags || [],
      description: doc.description || '',
      auto_save: false,
      version_note: ''
    })
    editVisible.value = true
  }

  async function saveDocument(formEl) {
    if (!formEl) return
    try {
      await formEl.validate()
    } catch {
      return
    }
    saving.value = true
    try {
      if (isEditing.value) {
        await api.kbUpdateDocument(editForm.id, editForm)
        ElMessage.success('更新成功')
      } else {
        await api.kbCreateDocument(editForm)
        ElMessage.success('创建成功')
      }
      editVisible.value = false
      fetchDocuments()
      fetchStats()
    } catch (e) {
      ElMessage.error('保存失败：' + (e.message || '未知错误'))
    } finally {
      saving.value = false
    }
  }

  async function saveFromDetail() {
    if (!selectedDoc.value) return
    saving.value = true
    try {
      await api.kbUpdateDocument(selectedDoc.value.id, { content: editForm.content })
      selectedDoc.value.content = editForm.content
      detailMode.value = 'view'
      ElMessage.success('保存成功')
      fetchVersions(selectedDoc.value.id)
    } catch {
      ElMessage.error('保存失败')
    } finally {
      saving.value = false
    }
  }

  async function deleteDocument(doc) {
    try {
      await ElMessageBox.confirm(`确定删除文档「${doc.title}」吗？`, '确认删除', {
        type: 'warning',
        confirmButtonText: '删除',
        cancelButtonText: '取消'
      })
    } catch {
      return
    }
    try {
      await api.kbDeleteDocument(doc.id)
      ElMessage.success('已删除')
      fetchDocuments()
      fetchStats()
    } catch {
      ElMessage.error('删除失败')
    }
  }

  async function revertVersion(versionId) {
    try {
      await ElMessageBox.confirm('确定回滚到此版本吗？当前版本将保存为历史记录。', '确认回滚', {
        type: 'warning'
      })
    } catch {
      return
    }
    try {
      await api.kbRevertVersion(selectedDoc.value.id, versionId)
      ElMessage.success('回滚成功')
      fetchVersions(selectedDoc.value.id)
      fetchHistory(selectedDoc.value.id)
    } catch {
      ElMessage.error('回滚失败')
    }
  }

  function viewVersion(ver) {
    compareFrom.value = ver
    compareTo.value = selectedDoc.value
    compareVisible.value = true
  }

  function compareWithPrevious(idx) {
    if (idx >= docVersions.value.length - 1) return
    compareFrom.value = docVersions.value[idx + 1]
    compareTo.value = docVersions.value[idx]
    compareVisible.value = true
  }

  // ========== Search & Filter Actions ==========

  function handleSearch() {
    fetchDocuments()
  }

  function resetFilters() {
    searchQuery.value = ''
    filterType.value = ''
    filterStatus.value = ''
    filterTag.value = ''
    filterCategory.value = ''
    filterDateRange.value = null
    fetchDocuments()
  }

  function handleCategoryClick(node) {
    filterCategory.value = node?.name || ''
    fetchDocuments()
  }

  function handleTagClick(tag) {
    filterTag.value = filterTag.value === tag.name ? '' : tag.name
    fetchDocuments()
  }

  function toggleSelectDoc(doc) {
    const idx = selectedDocs.value.indexOf(doc.id)
    if (idx >= 0) selectedDocs.value.splice(idx, 1)
    else selectedDocs.value.push(doc.id)
  }

  // ========== Entity Linking ==========

  async function searchEntities() {
    if (!linkSearchQuery.value.trim()) return
    try {
      const data = await api.kbSearchEntities({ q: linkSearchQuery.value })
      searchResults.value = Array.isArray(data) ? data : (data?.items || [])
    } catch {
      searchResults.value = []
    }
  }

  async function linkEntity(ent) {
    try {
      await api.kbLinkEntity(selectedDoc.value.id, ent.id)
      linkedEntities.value.push(ent)
      showLinkDialog.value = false
      ElMessage.success('关联成功')
    } catch {
      ElMessage.error('关联失败')
    }
  }

  async function unlinkEntity(ent) {
    try {
      await api.kbUnlinkEntity(selectedDoc.value.id, ent.id)
      const idx = linkedEntities.value.findIndex(e => e.id === ent.id)
      if (idx > -1) linkedEntities.value.splice(idx, 1)
      ElMessage.success('已解除关联')
    } catch {
      ElMessage.error('操作失败')
    }
  }

  // ========== Init ==========

  function init() {
    fetchCategories()
    fetchTags()
    fetchDocuments()
    fetchStats()
  }

  return {
    // State
    documents,
    categories,
    tags,
    selectedDoc,
    docVersions,
    docHistory,
    aiAnalysis,
    entities,
    linkedEntities,
    stats,
    loading,
    saving,
    searchQuery,
    filterCategory,
    filterType,
    filterStatus,
    filterTag,
    filterDateRange,
    viewMode,
    selectedDocs,
    detailVisible,
    detailTab,
    detailMode,
    editVisible,
    isEditing,
    showLinkDialog,
    compareVisible,
    compareFrom,
    compareTo,
    linkSearchQuery,
    searchResults,
    editForm,
    formRules,
    // Computed
    statCards,
    filteredDocuments,
    renderedContent,
    renderedCompareFrom,
    renderedCompareTo,
    // Constants
    docTypes: DOC_TYPES,
    // Methods - 工具函数
    getTagSize: (count) => getTagSize(tags.value, count),
    getTagType,
    getTypeLabel,
    getStatusType,
    getStatusLabel,
    getActionLabel,
    truncateText,
    formatTime,
    // Methods - 搜索筛选
    handleSearch,
    resetFilters,
    handleCategoryClick,
    handleTagClick,
    toggleSelectDoc,
    // Methods - 数据获取
    fetchDocuments,
    fetchCategories,
    fetchTags,
    fetchStats,
    // Methods - 文档操作
    viewDocument,
    closeDetail,
    handleDetailTabChange,
    openCreateDialog,
    openEditDialog,
    saveDocument,
    saveFromDetail,
    deleteDocument,
    analyzeDocument,
    // Methods - 版本管理
    viewVersion,
    compareWithPrevious,
    revertVersion,
    // Methods - 实体关联
    searchEntities,
    linkEntity,
    unlinkEntity,
    // Init
    init
  }
}

export default useKnowledgeBase
