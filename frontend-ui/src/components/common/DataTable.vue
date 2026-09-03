<!--
  通用数据表格组件
  封装 el-table，支持分页、排序、自定义列、空态、加载态
-->
<template>
  <div class="common-data-table">
    <!-- 加载态 -->
    <div v-if="loading" class="table-loading-overlay">
      <LoadingState :text="loadingText" />
    </div>

    <!-- 表格 -->
    <el-table
      ref="tableRef"
      :data="pagedData"
      :border="border"
      :stripe="stripe"
      :height="height"
      :max-height="maxHeight"
      :row-key="rowKey"
      :empty-text="''"
      @sort-change="handleSortChange"
      @selection-change="handleSelectionChange"
      @row-click="handleRowClick"
    >
      <!-- 选择列 -->
      <el-table-column v-if="selectable" type="selection" width="50" align="center" />
      <!-- 序号列 -->
      <el-table-column v-if="showIndex" type="index" label="#" width="60" align="center" :index="indexMethod" />
      <!-- 自定义列 -->
      <el-table-column
        v-for="col in columns"
        :key="col.prop || col.label"
        :prop="col.prop"
        :label="col.label"
        :width="col.width"
        :min-width="col.minWidth"
        :align="col.align || 'left'"
        :sortable="col.sortable || false"
        :fixed="col.fixed"
        :show-overflow-tooltip="col.tooltip !== false"
      >
        <template #default="scope">
          <slot :name="`cell-${col.prop}`" :row="scope.row" :column="col" :index="scope.$index">
            <span v-if="col.formatter">{{ col.formatter(scope.row[col.prop], scope.row) }}</span>
            <span v-else>{{ scope.row[col.prop] }}</span>
          </slot>
        </template>
      </el-table-column>
      <!-- 操作列 -->
      <el-table-column v-if="$slots.actions || actions.length" label="操作" :width="actionsWidth" align="center" fixed="right">
        <template #default="scope">
          <slot name="actions" :row="scope.row" :index="scope.$index">
            <el-button
              v-for="action in actions"
              :key="action.key"
              size="small"
              :type="action.type || 'primary'"
              :link="action.link !== false"
              @click.stop="action.handler?.(scope.row)"
            >
              {{ action.label }}
            </el-button>
          </slot>
        </template>
      </el-table-column>
      <!-- 空态 -->
      <template #empty>
        <EmptyState v-if="!loading" :icon="emptyIcon" :text="emptyText" :action-text="emptyActionText" @action="$emit('empty-action')" />
      </template>
    </el-table>

    <!-- 分页 -->
    <Pagination
      v-if="showPagination"
      v-model:current-page="currentPage"
      v-model:page-size="pageSize"
      :total="total"
      :page-sizes="pageSizes"
      @size-change="handleSizeChange"
      @current-change="handleCurrentChange"
    />
  </div>
</template>

<script setup>
import { ref, computed, watch } from 'vue'
import Pagination from './Pagination.vue'
import EmptyState from './EmptyState.vue'
import LoadingState from './LoadingState.vue'

const props = defineProps({
  data: { type: Array, default: () => [] },
  columns: { type: Array, default: () => [] },
  actions: { type: Array, default: () => [] },
  actionsWidth: { type: [String, Number], default: 150 },
  loading: { type: Boolean, default: false },
  loadingText: { type: String, default: '加载中...' },
  border: { type: Boolean, default: true },
  stripe: { type: Boolean, default: true },
  height: { type: [String, Number], default: null },
  maxHeight: { type: [String, Number], default: null },
  rowKey: { type: [String, Function], default: 'id' },
  selectable: { type: Boolean, default: false },
  showIndex: { type: Boolean, default: true },
  showPagination: { type: Boolean, default: true },
  total: { type: Number, default: 0 },
  pageSizes: { type: Array, default: () => [10, 20, 50, 100] },
  emptyIcon: { type: String, default: '📭' },
  emptyText: { type: String, default: '暂无数据' },
  emptyActionText: { type: String, default: '' },
  serverPagination: { type: Boolean, default: false }
})

const emit = defineEmits([
  'update:currentPage', 'update:pageSize', 'page-change', 'size-change',
  'sort-change', 'selection-change', 'row-click', 'empty-action', 'action'
])

const tableRef = ref(null)
const currentPage = ref(1)
const pageSize = ref(props.pageSizes[0] || 10)

const pagedData = computed(() => {
  if (props.serverPagination) return props.data
  const start = (currentPage.value - 1) * pageSize.value
  return props.data.slice(start, start + pageSize.value)
})

const effectiveTotal = computed(() => props.serverPagination ? props.total : props.data.length)

watch(() => props.data, () => { if (!props.serverPagination && currentPage.value > 1 && pagedData.value.length === 0) currentPage.value = 1 })

function indexMethod(index) { return (currentPage.value - 1) * pageSize.value + index + 1 }
function handleSortChange({ prop, order }) { emit('sort-change', { prop, order }) }
function handleSelectionChange(selection) { emit('selection-change', selection) }
function handleRowClick(row) { emit('row-click', row) }
function handleSizeChange(size) { pageSize.value = size; currentPage.value = 1; emit('size-change', size); emit('page-change', { page: 1, size }) }
function handleCurrentChange(page) { currentPage.value = page; emit('page-change', { page, size: pageSize.value }) }
</script>

<style scoped>
.common-data-table { position: relative; width: 100%; }
.table-loading-overlay {
  position: absolute; top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(255, 255, 255, 0.8); z-index: 10;
  display: flex; align-items: center; justify-content: center;
}
</style>
