<!--
  专家辩论对话框
  职责：辩题输入、专家选择、轮数设置、发起辩论。
  字段口径来自 @/modules/expert-alliance/contract：这里只发 topic/expert_ids/rounds，
  轮数上下界与参与上限都取自契约，视图不再自带一份后端结构体里不存在的「辩论模式」。
-->
<template>
  <el-dialog
    :model-value="visible"
    @update:model-value="emit('close')"
    title="发起专家辩论"
    width="520px"
    :close-on-click-modal="!submitting"
    class="debate-dialog"
  >
    <el-form label-width="88px" label-position="right">
      <el-form-item label="辩题" required>
        <el-input
          :model-value="topic"
          type="textarea"
          :rows="2"
          placeholder="请输入辩论主题…"
          maxlength="200"
          show-word-limit
          resize="none"
          @update:model-value="emit('update:topic', $event)"
        />
      </el-form-item>

      <el-form-item label="参与专家" required>
        <div class="debate-expert-picker">
          <div class="debate-expert-list">
            <div
              v-for="exp in pickableExperts"
              :key="exp.id"
              class="debate-expert-chip"
              :class="{ selected: selectedExpertIds.includes(exp.id) }"
              @click="$emit('toggle-expert', exp.id)"
            >
              <span class="chip-avatar" :style="{ background: expertColor(expertVisualKey(exp)) }">
                {{ expertEmoji(expertVisualKey(exp)) }}
              </span>
              <span class="chip-name">{{ exp.name }}</span>
              <span class="chip-status">{{ availabilityLabel(exp.status) }}</span>
              <el-icon v-if="selectedExpertIds.includes(exp.id)" class="chip-check"><CircleCheckFilled /></el-icon>
            </div>
          </div>
          <div class="debate-expert-count">
            已选 <b>{{ selectedExpertIds.length }}</b> 位专家（至少 {{ DEBATE_MIN_PARTICIPANTS }} 位）
          </div>
          <div v-if="capacityText" class="debate-capacity">{{ capacityText }}</div>
        </div>
      </el-form-item>

      <el-form-item label="辩论轮次">
        <el-input-number
          :model-value="rounds"
          :min="roundsBound.min"
          :max="roundsBound.max"
          size="small"
          @update:model-value="emit('update:rounds', $event)"
        />
        <span class="form-hint">轮（后端按轮次模板生成正反方论点）</span>
      </el-form-item>

      <el-form-item label="辩论状态">
        <el-tag :type="statusTagType" effect="light" size="small">
          {{ statusLabel }}
        </el-tag>
      </el-form-item>
    </el-form>

    <p v-if="note" class="debate-note">{{ note }}</p>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="$emit('close')" :disabled="submitting">取消</el-button>
        <el-tooltip :disabled="!problem" :content="problem" placement="top">
          <span>
            <el-button
              type="primary"
              :loading="submitting"
              :disabled="!!problem"
              @click="$emit('start')"
            >
              <el-icon><Flag /></el-icon>
              <span>开始辩论</span>
            </el-button>
          </span>
        </el-tooltip>
      </div>
    </template>
  </el-dialog>
</template>

<script setup>
import { computed } from 'vue'
import { CircleCheckFilled, Flag } from '@element-plus/icons-vue'
import {
  COLLAB_MODE, DEBATE_MIN_PARTICIPANTS, availabilityLabel,
  collabMode, debateCapacityText
} from '@/modules/expert-alliance/contract'
import { expertPickable, expertVisualKey } from '@/modules/expert-alliance/model'
import { expertColor, expertEmoji } from '@/constants'

const props = defineProps({
  visible: { type: Boolean, default: false },
  topic: { type: String, default: '' },
  selectedExpertIds: { type: Array, default: () => [] },
  problem: { type: String, default: '' },
  note: { type: String, default: '' },
  rounds: { type: Number, default: 3 },
  status: { type: String, default: 'preparing' },
  submitting: { type: Boolean, default: false },
  experts: { type: Array, default: () => [] }
})

const emit = defineEmits(['close', 'start', 'toggle-expert', 'update:topic', 'update:rounds'])

const def = collabMode(COLLAB_MODE.DEBATE)
const roundsBound = computed(() => def.controls.find((c) => c.wire === 'rounds'))
const capacityText = computed(() => debateCapacityText(props.selectedExpertIds.length))
// 停用专家后端一律 403，离线只是响应慢，不该被藏起来
const pickableExperts = computed(() => props.experts.filter(expertPickable))

const statusLabel = computed(() => {
  const map = { preparing: '准备中', ongoing: '进行中', summarized: '已总结' }
  return map[props.status] || '准备中'
})

const statusTagType = computed(() => {
  const map = { preparing: 'info', ongoing: 'warning', summarized: 'success' }
  return map[props.status] || 'info'
})
</script>

<style scoped>
.debate-note {
  margin: 8px 0 0;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.chip-status {
  font-size: 11px;
  color: var(--el-text-color-secondary);
}
.debate-capacity {
  margin-top: 4px;
  font-size: 12px;
  color: var(--el-color-warning);
}
</style>
