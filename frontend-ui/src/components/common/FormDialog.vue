<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    :width="width"
    destroy-on-close
    :close-on-click-modal="false"
    @update:model-value="handleVisibleChange"
    @closed="handleClosed"
  >
    <el-form ref="formRef" :model="formData" :rules="mergedRules" :label-width="labelWidth">
      <!-- Tabs mode -->
      <el-tabs v-if="hasTabs" v-model="activeTab">
        <el-tab-pane v-for="tab in tabList" :key="tab" :label="tab" :name="tab">
          <el-form-item
            v-for="field in fieldsByTab(tab)"
            :key="field.prop"
            :label="field.label"
            :prop="field.prop"
          >
            <template v-if="field.type === 'slot'">
              <slot :name="`field-${field.prop}`" :form-data="formData" :field="field" />
            </template>
            <el-input
              v-else-if="field.type === 'input'"
              v-model="formData[field.prop]"
              :placeholder="field.placeholder || `请输入${field.label}`"
              :type="field.inputType || 'text'"
              :maxlength="field.maxlength"
              :show-word-limit="field.showWordLimit"
              :show-password="field.showPassword"
              :clearable="field.clearable !== false"
              :disabled="isDisabled(field)"
            />
            <el-input
              v-else-if="field.type === 'textarea'"
              v-model="formData[field.prop]"
              type="textarea"
              :rows="field.rows || 3"
              :maxlength="field.maxlength"
              :show-word-limit="field.showWordLimit"
              :placeholder="field.placeholder || `请输入${field.label}`"
              :disabled="isDisabled(field)"
            />
            <el-input-number
              v-else-if="field.type === 'number'"
              v-model="formData[field.prop]"
              :min="field.min"
              :max="field.max"
              :step="field.step || 1"
              :disabled="isDisabled(field)"
            />
            <el-select
              v-else-if="field.type === 'select'"
              v-model="formData[field.prop]"
              :placeholder="field.placeholder || `请选择${field.label}`"
              :multiple="field.multiple"
              :filterable="field.filterable"
              :clearable="field.clearable !== false"
              :style="field.style || 'width: 100%'"
              :disabled="isDisabled(field)"
            >
              <el-option v-for="opt in field.options || []" :key="opt.value" :label="opt.label" :value="opt.value" />
            </el-select>
            <el-radio-group v-else-if="field.type === 'radio'" v-model="formData[field.prop]" :disabled="isDisabled(field)">
              <el-radio v-for="opt in field.options || []" :key="opt.value" :value="opt.value">{{ opt.label }}</el-radio>
            </el-radio-group>
            <el-switch
              v-else-if="field.type === 'switch'"
              v-model="formData[field.prop]"
              :active-value="field.activeValue !== undefined ? field.activeValue : true"
              :inactive-value="field.inactiveValue !== undefined ? field.inactiveValue : false"
              :disabled="isDisabled(field)"
            />
            <el-date-picker
              v-else-if="field.type === 'date'"
              v-model="formData[field.prop]"
              :type="field.dateType || 'date'"
              :value-format="field.valueFormat || 'YYYY-MM-DD'"
              :placeholder="field.placeholder || `请选择${field.label}`"
              :start-placeholder="field.startPlaceholder"
              :end-placeholder="field.endPlaceholder"
              :range-separator="field.rangeSeparator"
              :style="field.style || 'width: 100%'"
              :disabled="isDisabled(field)"
            />
            <el-tree-select
              v-else-if="field.type === 'treeSelect'"
              v-model="formData[field.prop]"
              :data="field.treeData || []"
              :props="field.treeProps || { label: 'label', value: 'value', children: 'children' }"
              :node-key="field.nodeKey || 'id'"
              :check-strictly="field.checkStrictly !== false"
              :render-after-expand="false"
              :placeholder="field.placeholder || `请选择${field.label}`"
              :filterable="field.filterable !== false"
              :clearable="field.clearable !== false"
              :style="field.style || 'width: 100%'"
              :disabled="isDisabled(field)"
            />
            <el-input
              v-else
              v-model="formData[field.prop]"
              :placeholder="field.placeholder || `请输入${field.label}`"
              :disabled="isDisabled(field)"
            />
          </el-form-item>
        </el-tab-pane>
      </el-tabs>

      <!-- Flat mode -->
      <template v-else>
        <el-form-item
          v-for="field in visibleFields"
          :key="field.prop"
          :label="field.label"
          :prop="field.prop"
        >
          <template v-if="field.type === 'slot'">
            <slot :name="`field-${field.prop}`" :form-data="formData" :field="field" />
          </template>
          <el-input
            v-else-if="field.type === 'input'"
            v-model="formData[field.prop]"
            :placeholder="field.placeholder || `请输入${field.label}`"
            :type="field.inputType || 'text'"
            :maxlength="field.maxlength"
            :show-word-limit="field.showWordLimit"
            :show-password="field.showPassword"
            :clearable="field.clearable !== false"
            :disabled="isDisabled(field)"
          />
          <el-input
            v-else-if="field.type === 'textarea'"
            v-model="formData[field.prop]"
            type="textarea"
            :rows="field.rows || 3"
            :maxlength="field.maxlength"
            :show-word-limit="field.showWordLimit"
            :placeholder="field.placeholder || `请输入${field.label}`"
            :disabled="isDisabled(field)"
          />
          <el-input-number
            v-else-if="field.type === 'number'"
            v-model="formData[field.prop]"
            :min="field.min"
            :max="field.max"
            :step="field.step || 1"
            :disabled="isDisabled(field)"
          />
          <el-select
            v-else-if="field.type === 'select'"
            v-model="formData[field.prop]"
            :placeholder="field.placeholder || `请选择${field.label}`"
            :multiple="field.multiple"
            :filterable="field.filterable"
            :clearable="field.clearable !== false"
            :style="field.style || 'width: 100%'"
            :disabled="isDisabled(field)"
          >
            <el-option v-for="opt in field.options || []" :key="opt.value" :label="opt.label" :value="opt.value" />
          </el-select>
          <el-radio-group v-else-if="field.type === 'radio'" v-model="formData[field.prop]" :disabled="isDisabled(field)">
            <el-radio v-for="opt in field.options || []" :key="opt.value" :value="opt.value">{{ opt.label }}</el-radio>
          </el-radio-group>
          <el-switch
            v-else-if="field.type === 'switch'"
            v-model="formData[field.prop]"
            :active-value="field.activeValue !== undefined ? field.activeValue : true"
            :inactive-value="field.inactiveValue !== undefined ? field.inactiveValue : false"
            :disabled="isDisabled(field)"
          />
          <el-date-picker
            v-else-if="field.type === 'date'"
            v-model="formData[field.prop]"
            :type="field.dateType || 'date'"
            :value-format="field.valueFormat || 'YYYY-MM-DD'"
            :placeholder="field.placeholder || `请选择${field.label}`"
            :start-placeholder="field.startPlaceholder"
            :end-placeholder="field.endPlaceholder"
            :range-separator="field.rangeSeparator"
            :style="field.style || 'width: 100%'"
            :disabled="isDisabled(field)"
          />
          <el-tree-select
            v-else-if="field.type === 'treeSelect'"
            v-model="formData[field.prop]"
            :data="field.treeData || []"
            :props="field.treeProps || { label: 'label', value: 'value', children: 'children' }"
            :node-key="field.nodeKey || 'id'"
            :check-strictly="field.checkStrictly !== false"
            :render-after-expand="false"
            :placeholder="field.placeholder || `请选择${field.label}`"
            :filterable="field.filterable !== false"
            :clearable="field.clearable !== false"
            :style="field.style || 'width: 100%'"
            :disabled="isDisabled(field)"
          />
          <el-input
            v-else
            v-model="formData[field.prop]"
            :placeholder="field.placeholder || `请输入${field.label}`"
            :disabled="isDisabled(field)"
          />
        </el-form-item>
      </template>

      <!-- Extra content slot -->
      <slot />
    </el-form>

    <template #footer>
      <el-button @click="handleCancel">{{ cancelText }}</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit">{{ submitText }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { ref, reactive, computed, watch, nextTick } from 'vue'

const props = defineProps({
  visible: { type: Boolean, default: false },
  title: { type: String, default: '' },
  formSchema: { type: Array, default: () => [] },
  width: { type: String, default: '500px' },
  submitText: { type: String, default: '确定' },
  cancelText: { type: String, default: '取消' },
  editData: { type: Object, default: null },
  submitting: { type: Boolean, default: false },
  labelWidth: { type: String, default: '90px' }
})

const emit = defineEmits(['update:visible', 'submit', 'cancel'])

const formRef = ref(null)
const activeTab = ref('')

function buildDefaults() {
  const defaults = {}
  props.formSchema.forEach(f => {
    if (f.prop !== undefined) {
      defaults[f.prop] = f.defaultValue !== undefined ? f.defaultValue : ''
    }
  })
  return defaults
}

const formData = reactive(buildDefaults())

watch(() => props.editData, (val) => {
  if (val) {
    Object.assign(formData, buildDefaults(), val)
  } else {
    Object.assign(formData, buildDefaults())
  }
}, { immediate: true })

watch(() => props.visible, (val) => {
  if (val) {
    nextTick(() => {
      formRef.value?.clearValidate()
    })
  }
})

const hasTabs = computed(() => props.formSchema.some(f => f.tab))
const tabList = computed(() => {
  const tabs = []
  props.formSchema.forEach(f => {
    if (f.tab && !tabs.includes(f.tab)) tabs.push(f.tab)
  })
  return tabs
})

function fieldsByTab(tab) {
  return props.formSchema.filter(f => f.tab === tab && isVisible(f))
}

const visibleFields = computed(() => props.formSchema.filter(f => isVisible(f)))

function isVisible(field) {
  if (field.visible === undefined) return true
  if (typeof field.visible === 'function') return field.visible(formData)
  return !!field.visible
}

function isDisabled(field) {
  if (field.disabled === undefined) return false
  if (typeof field.disabled === 'function') return field.disabled(formData)
  return !!field.disabled
}

const mergedRules = computed(() => {
  const rules = {}
  props.formSchema.forEach(f => {
    if (f.rules && f.prop) {
      rules[f.prop] = f.rules.map(rule => {
        if (typeof rule.validator === 'function') {
          const orig = rule.validator
          return {
            ...rule,
            validator: (r, value, callback) => orig(r, value, callback, formData)
          }
        }
        return rule
      })
    }
  })
  return rules
})

defineExpose({
  formData,
  validate: () => formRef.value?.validate(),
  clearValidate: () => formRef.value?.clearValidate(),
  resetFields: () => formRef.value?.resetFields()
})

function handleVisibleChange(val) {
  emit('update:visible', val)
}

function handleClosed() {
  Object.assign(formData, buildDefaults())
}

function handleCancel() {
  emit('cancel')
  emit('update:visible', false)
}

async function handleSubmit() {
  try {
    await formRef.value.validate()
  } catch {
    return
  }
  emit('submit', { ...formData })
}
</script>
