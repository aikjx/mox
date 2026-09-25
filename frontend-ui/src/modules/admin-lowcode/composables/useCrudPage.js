/**
 * useCrudPage：一页 CRUD 的唯一逻辑持有者。
 * 替代 17 个面板里逐字重复的 loadList/openDialog/submit/delete 样板。
 * 性能约束：列表用 shallowRef（行对象不做深度响应式），schema 由调用方 markRaw。
 */
import { ref, shallowRef, reactive, computed, markRaw } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'

function normalizeList(data) {
  if (Array.isArray(data)) return data
  if (Array.isArray(data?.list)) return data.list
  if (Array.isArray(data?.data)) return data.data
  return []
}

export function useCrudPage(pageSchema) {
  const schema = markRaw(pageSchema)

  const rows = shallowRef([])          // 行数据浅响应式：行引用不变就不 diff
  const loading = ref(false)
  const submitting = ref(false)
  const dialogVisible = ref(false)
  const editingRow = ref(null)

  // 筛选项：关键字 + search.fields 声明的字段
  const filters = reactive({ keyword: '' })
  for (const f of schema.search?.fields || []) {
    filters[f.prop] = f.defaultValue ?? ''
  }

  const isEdit = computed(() => !!editingRow.value)

  async function loadList() {
    loading.value = true
    try {
      const data = await schema.api.list()
      rows.value = normalizeList(data)
    } catch (e) {
      ElMessage.error('加载列表失败: ' + (e?.message || e))
    } finally {
      loading.value = false
    }
  }

  // 客户端过滤（小数据集）：关键字按 name/code 模糊，其余字段精确匹配
  const filteredRows = computed(() => {
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
    // 客户端过滤场景无需重新拉取；服务端分页场景才重拉
    if (schema.list?.serverPagination === true && schema.list?.serverFilter) {
      loadList()
    }
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
      } else {
        await schema.api.create(payload)
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      await loadList()
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
    loadList, onSearch, openCreate, openEdit, onSubmit, runRowAction,
  }
}
