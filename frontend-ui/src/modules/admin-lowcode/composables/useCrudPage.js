/**
 * useCrudPage：一页 CRUD 的唯一逻辑持有者。
 * 替代 N 个面板里逐字重复的 loadList/openDialog/submit/delete 样板。
 * 性能约束：列表用 shallowRef（行对象不做深度响应式），schema 由调用方 markRaw。
 *
 * 两种分页模式：
 * - serverPagination: false（默认，小数据量）：api.list() 一次拉全量，客户端过滤
 * - serverPagination: true：api.list({pageNum,pageSize,...filters})，返回 {list,total}
 */
import { ref, shallowRef, reactive, computed, markRaw } from 'vue'
import { ElMessage } from 'element-plus/es/components/message/index'
import { ElMessageBox } from 'element-plus/es/components/message-box/index'

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
    loading.value = true
    try {
      if (serverMode) {
        const data = await schema.api.list({
          pageNum: page.value,
          pageSize: pageSize.value,
          ...serverFilterParams(),
        })
        rows.value = normalizeList(data)
        total.value = data?.total ?? rows.value.length
      } else {
        const data = await schema.api.list()
        rows.value = normalizeList(data)
      }
    } catch (e) {
      ElMessage.error('加载列表失败: ' + (e?.message || e))
    } finally {
      loading.value = false
    }
  }

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
    editingRow.value = null
    dialogVisible.value = true
  }

  function openEdit(row) {
    editingRow.value = row
    dialogVisible.value = true
  }

  async function onSubmit(formData) {
    submitting.value = true
    try {
      const payload = schema.form?.buildPayload
        ? schema.form.buildPayload(formData, isEdit.value, editingRow.value)
        : { ...formData }
      if (isEdit.value) {
        await schema.api.update(editingRow.value.id, payload)
        ElMessage.success('更新成功')
        dialogVisible.value = false
        await loadList()
      } else {
        const created = await schema.api.create(payload)
        ElMessage.success('创建成功')
        dialogVisible.value = false
        await loadList()
        // 创建后钩子（如 Access：明文 key 仅此一次，需弹窗展示）
        if (typeof schema.form?.afterCreate === 'function') {
          await schema.form.afterCreate(created, { reload: loadList })
        }
      }
    } catch (e) {
      ElMessage.error('操作失败: ' + (e?.message || e))
    } finally {
      submitting.value = false
    }
  }

  async function onDelete(row, action) {
    const msg = typeof action?.confirm === 'function'
      ? action.confirm(row)
      : (action?.confirm || `确定删除「${row.name || row.code || ''}」？`)
    try {
      await ElMessageBox.confirm(msg, '删除确认', { type: 'warning' })
      await schema.api.remove(row.id)
      ElMessage.success('删除成功')
      await loadList()
    } catch (e) {
      if (e !== 'cancel' && e?.message) ElMessage.error('删除失败: ' + e.message)
    }
  }

  // 行动作分发：内置 edit/delete，其余走 schema 里的 handler(row, ctx)
  function runRowAction(action, row) {
    if (action.action === 'edit') return openEdit(row)
    if (action.action === 'delete') return onDelete(row, action)
    if (typeof action.handler === 'function') {
      return action.handler(row, { loadList, ElMessage })
    }
  }

  return {
    rows, filteredRows, loading, submitting, dialogVisible, editingRow, isEdit, filters,
    page, pageSize, total, serverMode, searchNonce,
    loadList, onSearch, onPageChange, openCreate, openEdit, onSubmit, runRowAction,
  }
}
