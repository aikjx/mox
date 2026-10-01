<template>
  <section class="erf">
    <p class="erf-lede">
      字段清单与边界取自网关 <code class="erf-code">merge_expert_from_value</code>：白名单之外的键会被静默丢弃，
      所以本表单只挂载后端真正认得的项。
    </p>

    <div class="erf-grid">
      <label v-for="f in scalarFields" :key="f.key" class="erf-item" :class="{ 'erf-item-wide': f.kind === 'textarea' }">
        <span class="erf-label">
          {{ f.label }}<em v-if="f.required" class="erf-req">必填</em>
        </span>
        <el-select v-if="f.kind === 'select'" v-model="draft[f.key]" class="erf-control" placeholder="未设置">
          <el-option v-for="o in f.options" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
        <el-input-number
          v-else-if="f.kind === 'number'"
          v-model="draft[f.key]"
          class="erf-control"
          :min="f.min"
          :max="f.max"
          :step="f.step"
          :controls="false"
        />
        <el-input
          v-else-if="f.kind === 'textarea'"
          v-model="draft[f.key]"
          class="erf-control"
          type="textarea"
          :rows="3"
          maxlength="500"
          show-word-limit
        />
        <el-input v-else v-model="draft[f.key]" class="erf-control" :maxlength="120" />
        <span class="erf-note">{{ f.hint || fieldHint(f) }}</span>
      </label>
    </div>

    <div v-for="f in listFields" :key="f.key" class="erf-block">
      <span class="erf-label">{{ f.label }}</span>
      <el-select
        v-model="draft[f.key]"
        class="erf-control"
        multiple
        filterable
        allow-create
        default-first-option
        :reserve-keyword="false"
        :placeholder="`输入后回车添加${f.label}`"
      />
      <span class="erf-note">{{ f.hint || `${f.label}为整值替换：编辑时清空再保存即真的清空` }}</span>
    </div>

    <div class="erf-block">
      <span class="erf-label">能力项</span>
      <p class="erf-note">
        熟练度必须手填：后端对字符串简写会把 proficiency 硬编码成 85、domain 落到首个标签（experts_registry.rs:105-123），
        那不是一个测出来的数。
      </p>
      <div v-for="(cap, i) in draft.capabilities" :key="i" class="erf-cap">
        <el-input v-model="cap.name" placeholder="能力名称" :maxlength="60" />
        <el-input v-model="cap.domain" placeholder="领域（可空）" :maxlength="40" />
        <el-input-number
          v-model="cap.proficiency"
          :min="0"
          :max="EXPERT_PROFICIENCY_MAX"
          :step="5"
          :controls="false"
          placeholder="0–255"
        />
        <el-input v-model="cap.description" placeholder="说明（可空）" :maxlength="120" />
        <el-button text :icon="Delete" @click="draft.capabilities.splice(i, 1)">移除</el-button>
      </div>
      <div>
        <el-button :icon="Plus" @click="draft.capabilities.push({ id: '', name: '', domain: '', proficiency: undefined, description: '' })">
          添加能力项
        </el-button>
      </div>
    </div>

    <el-alert v-if="problem" class="erf-alert" type="error" show-icon :closable="false" title="还不能提交" :description="problem" />

    <p class="erf-lede erf-unmounted">
      本表单不录入：{{ unmountedKeys }}。理由分两类——绩效类由真实咨询写入（开成输入框即可人工造榜），
      运行态类只有调度器会动；这些键后端照收，挂载点记在 <code class="erf-code">EXPERT_UNMOUNTED_FIELDS</code>。
    </p>
  </section>
</template>

<script setup>
import { computed, reactive } from 'vue'
import { Delete, Plus } from '@element-plus/icons-vue'
import {
  EXPERT_PROFICIENCY_MAX, EXPERT_REGISTER_FIELDS, EXPERT_UNMOUNTED_FIELDS,
  expertDraftProblem, expertFormDraft
} from '@/modules/expert-alliance/contract'

// 草稿在这里持有，视图通过 ref 取用；打开时由外层 :key 重挂载，
// 所以「编辑另一位专家」不会带着上一位的未提交改动。
const props = defineProps({
  expert: { type: Object, default: null }
})

const scalarFields = EXPERT_REGISTER_FIELDS.filter((f) => ['text', 'textarea', 'select', 'number'].includes(f.kind))
const listFields = EXPERT_REGISTER_FIELDS.filter((f) => f.kind === 'tags')
const unmountedKeys = EXPERT_UNMOUNTED_FIELDS.map((f) => f.key).join(' · ')

const draft = reactive(expertFormDraft(props.expert || {}))

const problem = computed(() => expertDraftProblem(draft))

/** 数字项把后端语义写在界面上：越界不是报错，而是静默截断或整条丢弃 */
function fieldHint(f) {
  if (f.key === 'hourlyRateCents') return '整数分，上限 2³²−1：后端按 u64 读再 as u32 存，超界会截成另一个数'
  if (f.key === 'maxConcurrent') return '同上 u32 上限；0 表示不设上限，负载率因此不可当百分比看'
  if (f.kind === 'select') return '取值由本表单约束，后端照收不验'
  return ''
}

defineExpose({ draft, problem })
</script>

<style scoped>
.erf { display: flex; flex-direction: column; gap: 14px; }
.erf-lede { margin: 0; font-size: 12px; line-height: 1.6; color: var(--text-secondary); }
.erf-code { font-size: 11px; color: var(--text-primary); }
.erf-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
.erf-item { display: flex; flex-direction: column; gap: 4px; }
.erf-item-wide { grid-column: span 2; }
.erf-label { font-size: 12px; color: var(--text-secondary); }
.erf-req {
  margin-left: 6px;
  font-size: 11px;
  font-style: normal;
  color: var(--danger);
}
.erf-control { width: 100%; }
.erf-note { font-size: 11px; line-height: 1.5; color: var(--text-muted); }
.erf-block { display: flex; flex-direction: column; gap: 6px; }
.erf-cap { display: grid; grid-template-columns: 1.2fr 1fr 72px 1.4fr auto; gap: 6px; align-items: center; }
.erf-alert { margin-top: 2px; }
.erf-unmounted { border-top: 1px solid var(--border); padding-top: 10px; }

@media (max-width: 720px) {
  .erf-grid { grid-template-columns: minmax(0, 1fr); }
  .erf-item-wide { grid-column: span 1; }
  .erf-cap { grid-template-columns: minmax(0, 1fr) auto; }
}
</style>
