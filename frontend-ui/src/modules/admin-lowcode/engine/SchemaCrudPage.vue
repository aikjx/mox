<script setup>
/**
 * SchemaCrudPage：整页编排器。
 * 组合既有 common 组件：SearchForm + DataTable + FormDialog，逻辑在 useCrudPage。
 * pageSchema 由路由 props 注入，模块级常量 markRaw，避免响应式代理开销。
 */
import { computed, onMounted, markRaw } from 'vue'
import DataTable from '@/components/common/DataTable.vue'
import FormDialog from '@/components/common/FormDialog.vue'
import SearchForm from '@/components/common/SearchForm.vue'
import { useCrudPage } from '../composables/useCrudPage.js'
import SchemaRenderer from './SchemaRenderer.vue'
import { assertPageSchema } from '../contract/pageSchema.js'

const props = defineProps({ pageSchema: { type: Object, required: true } })

// DSL 校验：dev 期快速失败；markRaw：schema 永不被响应式代理
const schema = markRaw(assertPageSchema(props.pageSchema))

const {
  filteredRows, loading, submitting, dialogVisible, editingRow, isEdit,
  total, serverMode, searchNonce,
  loadList, onSearch, onPageChange, openCreate, openEdit, onSubmit, runRowAction,
} = useCrudPage(schema)

const columns = computed(() => schema.list.columns)
// 需要自定义列渲染（tag/dict/date/arrayTags 等）的列，按 prop 名挂到 DataTable 的动态 cell slot
const widgetColumns = computed(() => schema.list.columns.filter(c => c.widget))
const searchFields = computed(() => schema.search?.fields || [])

// FormDialog 只认 visible/disabled: bool|fn(formData)；把 schema 的 disabledOnEdit/visibleOnEdit
// 翻译成闭包，闭包捕获 isEdit
const dialogFormSchema = computed(() => (schema.form?.fields || []).map(f => ({
  ...f,
  disabled: f.disabledOnEdit ? () => isEdit.value : f.disabled,
  visible: f.visibleOnEdit ? () => isEdit.value : (f.visible !== undefined ? f.visible : true),
})))

const stats = computed(() => (schema.stats || []).map(s => ({
  ...s,
  value: typeof s.value === 'function' ? s.value(filteredRows.value) : s.value,
})))

const visibleRowActions = (row) =>
  (schema.list.rowActions || []).filter(a => !a.show || a.show(row))

const rowActionLabel = (a, row) =>
  typeof a.label === 'function' ? a.label(row) : a.label

function onToolbarAction(t) {
  if (t.action === 'create') openCreate()
  else if (t.action === 'reload') loadList()
  else if (typeof t.handler === 'function') t.handler({ loadList })
}

onMounted(loadList)
</script>

<template>
  <div class="schema-crud-page">
    <!-- 统计卡（可选） -->
    <div v-if="stats.length" class="stat-row">
      <div class="stat-card" v-for="s in stats" :key="s.label" :style="{ '--c': s.color }">
        <div class="stat-label">{{ s.label }}</div>
        <div class="stat-value">{{ s.value }}</div>
        <div class="stat-icon">{{ s.icon }}</div>
      </div>
    </div>

    <!-- 搜索区（可选） -->
    <SearchForm
      v-if="schema.search"
      :fields="searchFields"
      :show-keyword="schema.search.showKeyword !== false"
      :keyword-placeholder="schema.search.keywordPlaceholder"
      @search="onSearch"
    />

    <!-- 工具栏 -->
    <div v-if="schema.toolbar?.length" class="toolbar">
      <el-button
        v-for="t in schema.toolbar" :key="t.label"
        :type="t.type || 'primary'" :icon="t.icon" :loading="t.loading"
        @click="onToolbarAction(t)"
      >{{ t.label }}</el-button>
    </div>

    <!-- 表格：serverMode 透传 total 并监听 page-change；searchNonce 变更时重挂载重置内部分页 -->
    <DataTable
      :key="searchNonce"
      :data="filteredRows"
      :columns="columns"
      :loading="loading"
      :row-key="schema.list.rowKey || 'id'"
      :show-pagination="schema.list.showPagination !== false"
      :server-pagination="serverMode"
      :total="total"
      @row-click="openEdit"
      @page-change="onPageChange"
    >
      <!-- 动态具名 slot：每个需要 widget 渲染的列一个；v-memo 限制单元格仅在依赖变化时重渲 -->
      <template
        v-for="col in widgetColumns" #[`cell-${col.prop}`]="{ row }" :key="col.prop"
      >
        <SchemaRenderer v-memo="[row.id, row.status, row.name, row.enabled]" :node="col" :row="row" />
      </template>
      <template #actions="{ row }">
        <el-button
          v-for="a in visibleRowActions(row)" :key="rowActionLabel(a, row)"
          size="small" :type="a.type || 'primary'" link
          :disabled="a.disabled ? a.disabled(row) : false"
          @click.stop="runRowAction(a, row)"
        >{{ rowActionLabel(a, row) }}</el-button>
      </template>
    </DataTable>

    <!-- 新增/编辑对话框（可选） -->
    <FormDialog
      v-if="schema.form"
      v-model:visible="dialogVisible"
      :title="isEdit ? (schema.form.editTitle || '编辑') : (schema.form.createTitle || '新增')"
      :form-schema="dialogFormSchema"
      :edit-data="editingRow"
      :submitting="submitting"
      :width="schema.form.width || '520px'"
      :label-width="schema.form.labelWidth || '90px'"
      @submit="onSubmit"
    />
  </div>
</template>

<style scoped>
.stat-row { display: flex; gap: 12px; margin-bottom: 16px; flex-wrap: wrap; }
.stat-card {
  flex: 1 1 140px; min-width: 120px; padding: 14px 16px;
  background: linear-gradient(135deg, rgba(255,255,255,0.9), rgba(255,255,255,0.7));
  border: 1px solid rgba(0,0,0,0.06); border-radius: 12px;
  position: relative; overflow: hidden;
}
.stat-card::before {
  content: ''; position: absolute; left: 0; top: 0; bottom: 0; width: 3px;
  background: var(--c, #8BC8EA);
}
.stat-label { font-size: 12px; color: var(--text-tertiary); }
.stat-value { font-size: 24px; font-weight: 700; color: #1A1B1C; margin-top: 4px; }
.stat-icon { position: absolute; right: 12px; top: 50%; transform: translateY(-50%); font-size: 28px; opacity: 0.3; }
.toolbar { display: flex; gap: 8px; margin-bottom: 12px; }
</style>
