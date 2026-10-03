/**
 * useCrudPage：一页 CRUD 的唯一逻辑持有者。
 * 替代 N 个面板里逐字重复的 loadList/openDialog/submit/delete 样板。
 * 性能约束：列表用 shallowRef（行对象不做深度响应式），schema 由调用方 markRaw。
 *
 * 两种分页模式：
 * - serverPagination: false（默认，小数据量）：api.list() 一次拉全量，客户端过滤
 * - serverPagination: true：api.list({pageNum,pageSize,...filters})，返回 {list,total}
 */
import { ref, shallowRef, reactive, computed, markRaw, watch, onBeforeUnmount } from 'vue'
import { ElMessage } from 'element-plus/es/components/message/index'
import { ElMessageBox } from 'element-plus/es/components/message-box/index'
import { useAuthStore } from '@/stores'

function normalizeList(data) {
  if (Array.isArray(data)) return data
  if (Array.isArray(data?.list)) return data.list
  if (Array.isArray(data?.data)) return data.data
  return []
}

export function useCrudPage(pageSchema) {
  const schema = markRaw(pageSchema)
  const serverMode = schema.list?.serverPagination === true

  const rows = shallowRef([])          // 行数据浅响应式：行引用不变就不 diff
  const loading = ref(false)
  const submitting = ref(false)
  const dialogVisible = ref(false)
  const editingRow = ref(null)

  // 分页（服务端模式）
  const page = ref(1)
  const pageSize = ref(schema.list?.defaultPageSize || 10)
  const total = ref(0)
  let scopeVersion = 0
  let listRequest = 0
  // 搜索变更时强制 DataTable 重挂载（重置其内部分页状态）
  const searchNonce = ref(0)

  // 筛选项：关键字 + search.fields 声明的字段
  const filters = reactive({ keyword: '' })
  for (const f of schema.search?.fields || []) {
    filters[f.prop] = f.defaultValue ?? ''
  }

  const isEdit = computed(() => !!editingRow.value)

  /** 服务端模式要发给后端的过滤参数（仅保留非空） */
  function serverFilterParams() {
    const params = {}
    const kw = (filters.keyword || '').trim()
    if (kw) params.keyword = kw
    for (const f of schema.search?.fields || []) {
      const v = filters[f.prop]
      if (v !== '' && v !== null && v !== undefined) params[f.prop] = v
    }
    return params
  }

  async function loadList() {
    const scope = scopeVersion
    const request = ++listRequest
    loading.value = true
    try {
      if (serverMode) {
        const data = await schema.api.list({
          pageNum: page.value,
          pageSize: pageSize.value,
          ...serverFilterParams(),
        })
        if (scope !== scopeVersion || request !== listRequest) return
        rows.value = normalizeList(data)
        total.value = data?.total ?? rows.value.length
      } else {
        const data = await schema.api.list()
        if (scope !== scopeVersion || request !== listRequest) return
        rows.value = normalizeList(data)
      }
    } catch (e) {
      if (scope !== scopeVersion || request !== listRequest) return
      rows.value = []
      total.value = 0
      ElMessage.error('加载列表失败: ' + (e?.message || e))
    } finally {
      if (scope === scopeVersion && request === listRequest) loading.value = false
    }
  }
  if (typeof schema.identityScope === 'function') {
    const auth = useAuthStore()
    watch(() => schema.identityScope(auth), () => {
      scopeVersion++
      rows.value = []
      total.value = 0
      page.value = 1
      searchNonce.value++
      dialogVisible.value = false
      editingRow.value = null
      submitting.value = false
      schema.onIdentityChange?.()
      loadList()
    }, { flush: 'sync' })
  }
  onBeforeUnmount(() => { scopeVersion++; listRequest++; schema.onIdentityChange?.() })

  // 客户端过滤（小数据集）：关键字按 name/code 模糊，其余字段精确匹配
  const filteredRows = computed(() => {
    if (serverMode) return rows.value
    let list = rows.value
    const kw = (filters.keyword || '').trim().toLowerCase()
    if (kw) {
      list = list.filter(r =>
        (r.name || '').toLowerCase().includes(kw) ||
        (r.code || '').toLowerCase().includes(kw)
      )
    }
    for (const f of schema.search?.fields || []) {
      const v = filters[f.prop]
      if (v !== '' && v !== null && v !== undefined) {
        list = list.filter(r => r[f.prop] === v)
      }
    }
    return list
  })

  function onSearch(form) {
    filters.keyword = form.keyword || ''
    for (const f of schema.search?.fields || []) {
      if (form[f.prop] !== undefined) filters[f.prop] = form[f.prop]
    }
    page.value = 1
    searchNonce.value++
    loadList()
  }

  function onPageChange({ page: p, size }) {
    page.value = p
    if (size) pageSize.value = size
    loadList()
  }

  function openCreate() {
    if (schema.readOnly) return
    editingRow.value = null
    dialogVisible.value = true
  }

  function openEdit(row) {
    if (schema.readOnly) return
    editingRow.value = row
    dialogVisible.value = true
  }

  async function onSubmit(formData) {
    if (schema.readOnly) return
    if (submitting.value) return
    const scope = scopeVersion
    submitting.value = true
    try {
      const payload = schema.form?.buildPayload
        ? schema.form.buildPayload(formData, isEdit.value, editingRow.value)
        : { ...formData }
      if (isEdit.value) {
        await schema.api.update(editingRow.value.id, payload)
        if (scope !== scopeVersion) return
        ElMessage.success('更新成功')
        dialogVisible.value = false
        await loadList()
      } else {
        const created = await schema.api.create(payload)
        if (scope !== scopeVersion) return
        ElMessage.success('创建成功')
        dialogVisible.value = false
        await loadList()
        // 创建后钩子（如 Access：明文 key 仅此一次，需弹窗展示）
        if (scope !== scopeVersion) return
        if (typeof schema.form?.afterCreate === 'function') {
          await schema.form.afterCreate(created, { reload: loadList })
        }
      }
    } catch (e) {
      if (scope === scopeVersion) ElMessage.error('操作失败: ' + (e?.message || e))
    } finally {
      if (scope === scopeVersion) submitting.value = false
    }
  }

  async function onDelete(row, action) {
    if (schema.readOnly) return
    const scope = scopeVersion
    const msg = typeof action?.confirm === 'function'
      ? action.confirm(row)
      : (action?.confirm || `确定删除「${row.name || row.code || ''}」？`)
    try {
      await ElMessageBox.confirm(msg, '删除确认', { type: 'warning' })
      if (scope !== scopeVersion) return
      await schema.api.remove(row.id)
      if (scope !== scopeVersion) return
      ElMessage.success('删除成功')
      await loadList()
    } catch (e) {
      if (scope === scopeVersion && e !== 'cancel' && e?.message) ElMessage.error('删除失败: ' + e.message)
    }
  }

  // 行动作分发：内置 edit/delete，其余走 schema 里的 handler(row, ctx)
  function runRowAction(action, row) {
    if (schema.readOnly) return
    if (action.action === 'edit') return openEdit(row)
    if (action.action === 'delete') return onDelete(row, action)
    if (typeof action.handler === 'function') {
      const scope = scopeVersion
      return action.handler(row, { loadList, ElMessage, isCurrent: () => scope === scopeVersion })
    }
  }

  return {
    rows, filteredRows, loading, submitting, dialogVisible, editingRow, isEdit, filters,
    page, pageSize, total, serverMode, searchNonce,
    loadList, onSearch, onPageChange, openCreate, openEdit, onSubmit, runRowAction,
  }
}
