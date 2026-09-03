<!--
  通用分页组件
  封装 el-pagination，统一 page/page_size 事件
-->
<template>
  <div class="common-pagination">
    <el-pagination
      v-model:current-page="innerCurrentPage"
      v-model:page-size="innerPageSize"
      :page-sizes="pageSizes"
      :total="total"
      :layout="layout"
      :background="background"
      :small="small"
      :disabled="disabled"
      :hide-on-single-page="hideOnSinglePage"
      @size-change="handleSizeChange"
      @current-change="handleCurrentChange"
    />
  </div>
</template>

<script setup>
import { ref, watch } from 'vue'

const props = defineProps({
  currentPage: { type: Number, default: 1 },
  pageSize: { type: Number, default: 10 },
  total: { type: Number, default: 0 },
  pageSizes: { type: Array, default: () => [10, 20, 50, 100] },
  layout: { type: String, default: 'total, sizes, prev, pager, next, jumper' },
  background: { type: Boolean, default: true },
  small: { type: Boolean, default: false },
  disabled: { type: Boolean, default: false },
  hideOnSinglePage: { type: Boolean, default: false }
})

const emit = defineEmits(['update:currentPage', 'update:pageSize', 'size-change', 'current-change', 'page-change'])

const innerCurrentPage = ref(props.currentPage)
const innerPageSize = ref(props.pageSize)

watch(() => props.currentPage, (v) => { innerCurrentPage.value = v })
watch(() => props.pageSize, (v) => { innerPageSize.value = v })

function handleSizeChange(size) {
  innerPageSize.value = size
  innerCurrentPage.value = 1
  emit('update:pageSize', size)
  emit('update:currentPage', 1)
  emit('size-change', size)
  emit('page-change', { page: 1, size })
}

function handleCurrentChange(page) {
  innerCurrentPage.value = page
  emit('update:currentPage', page)
  emit('current-change', page)
  emit('page-change', { page, size: innerPageSize.value })
}
</script>

<style scoped>
.common-pagination { display: flex; justify-content: flex-end; padding: 16px 0; }
</style>
