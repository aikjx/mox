<!--
  通用搜索表单组件
  关键字 + 筛选条件 + 搜索/重置按钮
-->
<template>
  <div class="common-search-form">
    <el-form :inline="true" :model="form" class="search-form-inline" @submit.prevent>
      <!-- 关键字搜索 -->
      <el-form-item v-if="showKeyword" :label="keywordLabel">
        <el-input
          v-model="form.keyword"
          :placeholder="keywordPlaceholder"
          clearable
          :prefix-icon="Search"
          class="keyword-input"
          @keyup.enter="handleSearch"
          @clear="handleReset"
        />
      </el-form-item>

      <!-- 动态筛选字段 -->
      <el-form-item v-for="field in fields" :key="field.prop" :label="field.label">
        <!-- 下拉选择 -->
        <el-select v-if="field.type === 'select'" v-model="form[field.prop]" :placeholder="field.placeholder || '请选择'" clearable class="filter-select" @change="handleFieldChange(field)">
          <el-option v-for="opt in field.options" :key="opt.value" :label="opt.label" :value="opt.value" />
        </el-select>
        <!-- 日期选择 -->
        <el-date-picker v-else-if="field.type === 'date'" v-model="form[field.prop]" type="date" :placeholder="field.placeholder || '选择日期'" value-format="YYYY-MM-DD" class="filter-date" @change="handleFieldChange(field)" />
        <!-- 日期范围 -->
        <el-date-picker v-else-if="field.type === 'daterange'" v-model="form[field.prop]" type="daterange" range-separator="至" start-placeholder="开始日期" end-placeholder="结束日期" value-format="YYYY-MM-DD" class="filter-daterange" @change="handleFieldChange(field)" />
        <!-- 数字输入 -->
        <el-input-number v-else-if="field.type === 'number'" v-model="form[field.prop]" :min="field.min" :max="field.max" :step="field.step || 1" class="filter-number" @change="handleFieldChange(field)" />
        <!-- 文本输入（默认） -->
        <el-input v-else v-model="form[field.prop]" :placeholder="field.placeholder || '请输入'" clearable class="filter-input" @keyup.enter="handleSearch" @clear="handleFieldChange(field)" />
      </el-form-item>

      <!-- 操作按钮 -->
      <el-form-item class="search-actions">
        <el-button type="primary" :icon="Search" @click="handleSearch">搜索</el-button>
        <el-button :icon="RefreshLeft" @click="handleReset">重置</el-button>
        <el-button v-if="showAdvanced" text type="primary" @click="advancedVisible = !advancedVisible">
          {{ advancedVisible ? '收起' : '高级筛选' }}
          <el-icon><component :is="advancedVisible ? 'ArrowUp' : 'ArrowDown'" /></el-icon>
        </el-button>
      </el-form-item>
    </el-form>
  </div>
</template>

<script setup>
import { ref, reactive, watch } from 'vue'
import { Search, RefreshLeft } from '@element-plus/icons-vue'

const props = defineProps({
  fields: { type: Array, default: () => [] },
  showKeyword: { type: Boolean, default: true },
  keywordLabel: { type: String, default: '关键字' },
  keywordPlaceholder: { type: String, default: '请输入关键字搜索...' },
  showAdvanced: { type: Boolean, default: false },
  immediate: { type: Boolean, default: false }
})

const emit = defineEmits(['search', 'reset', 'field-change', 'update:modelValue'])

const form = reactive({ keyword: '' })
const advancedVisible = ref(false)

// 初始化字段默认值
props.fields.forEach(field => {
  if (form[field.prop] === undefined) {
    form[field.prop] = field.defaultValue !== undefined ? field.defaultValue : (field.type === 'daterange' ? [] : '')
  }
})

watch(form, (val) => { emit('update:modelValue', { ...val }) }, { deep: true })

function handleSearch() { emit('search', { ...form }) }
function handleReset() {
  form.keyword = ''
  props.fields.forEach(field => { form[field.prop] = field.defaultValue !== undefined ? field.defaultValue : (field.type === 'daterange' ? [] : '') })
  emit('reset')
  emit('search', { ...form })
}
function handleFieldChange(field) { emit('field-change', { field, value: form[field.prop] }) }
</script>

<style scoped>
.common-search-form { padding: 16px; background: var(--el-bg-color); border-radius: 8px; margin-bottom: 16px; }
.search-form-inline { display: flex; flex-wrap: wrap; gap: 0; }
.keyword-input { width: 240px; }
.filter-select, .filter-input { width: 180px; }
.filter-date, .filter-number { width: 180px; }
.filter-daterange { width: 280px; }
.search-actions { margin-left: auto; }
</style>
