<!--
  多专家咨询对话框
  职责：问题输入、专家选择、结果展示（对比/列表）、融合结论。
  行数据一律来自 model/collabLists 的投影，视图不碰后端字段名；
  契约里没有的「并行/串行模式」不再作为选项出现（MultiConsultBody 无此字段，发了即丢）。
-->
<template>
  <el-dialog
    :model-value="visible"
    @update:model-value="emit('close')"
    title="多专家咨询"
    width="560px"
    :close-on-click-modal="!submitting"
    class="multi-consult-dialog"
  >
    <el-form label-width="88px" label-position="right">
      <el-form-item label="咨询问题" required>
        <el-input
          :model-value="question"
          type="textarea"
          :rows="3"
          placeholder="请输入您想咨询的问题…"
          maxlength="500"
          show-word-limit
          resize="none"
          @update:model-value="emit('update:question', $event)"
        />
      </el-form-item>

      <el-form-item label="选择专家">
        <div class="consult-expert-picker">
          <div class="consult-expert-list">
            <div
              v-for="exp in pickableExperts"
              :key="exp.id"
              class="consult-expert-chip"
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
          <div class="consult-expert-count">
            已选 <b>{{ selectedExpertIds.length }}</b> 位专家
            <span class="consult-count-hint">（不选则由后端按问题匹配度自动挑选）</span>
          </div>
        </div>
      </el-form-item>

      <el-form-item label="结果展示">
        <el-switch
          :model-value="compareView"
          active-text="对比视图"
          inactive-text="列表视图"
          @update:model-value="emit('update:compareView', $event)"
        />
      </el-form-item>
    </el-form>

    <!-- 咨询结果展示 -->
    <div v-if="results.length > 0" class="consult-results-section">
      <div class="results-section-head">
        <span class="results-section-title">
          <el-icon><DocumentCopy /></el-icon>
          咨询结果
        </span>
        <el-tag size="small" type="success" effect="light">
          {{ results.length }} 位专家已回答
        </el-tag>
      </div>

      <!-- 对比视图 -->
      <div v-if="compareView" class="compare-view">
        <div class="compare-grid">
          <div v-for="row in results" :key="row.key" class="compare-card">
            <div class="compare-card-head" :style="{ borderTopColor: rowColor(row) }">
              <div class="compare-expert">
                <span class="compare-avatar" :style="{ background: rowColor(row) }">
                  {{ expertEmoji(rowVisualKey(row)) }}
                </span>
                <span class="compare-name">{{ row.name }}</span>
              </div>
              <el-tag size="small" type="primary" effect="light" v-if="row.confidence">
                置信度 {{ confidenceText(row.confidence) }}
              </el-tag>
            </div>
            <div class="compare-card-body">
              <div class="compare-content">{{ row.text }}</div>
              <div class="row-badges">
                <el-tag size="small" :type="row.blocked ? 'danger' : (row.modelBacked ? 'success' : 'warning')" effect="plain">
                  {{ row.sourceText }}
                </el-tag>
                <el-tag v-if="row.blocked" size="small" type="danger" effect="dark">已被治理闸门拦截</el-tag>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 列表视图 -->
      <div v-else class="list-view">
        <div v-for="row in results" :key="row.key" class="result-item-card">
          <div class="result-item-head">
            <span class="result-avatar" :style="{ background: rowColor(row) }">
              {{ expertEmoji(rowVisualKey(row)) }}
            </span>
            <span class="result-name">{{ row.name }}</span>
            <el-tag v-if="row.confidence" size="small" type="primary" effect="light">
              置信度 {{ confidenceText(row.confidence) }}
            </el-tag>
            <el-tag size="small" :type="row.blocked ? 'danger' : (row.modelBacked ? 'success' : 'warning')" effect="plain">
              {{ row.sourceText }}
            </el-tag>
          </div>
          <div class="result-item-body">{{ row.text }}</div>
        </div>
      </div>

      <!-- 融合结论：共识度取自两两相似度，单专家时无从比对 -->
      <div v-if="fusion && fusion.summary" class="fusion-block">
        <div class="fusion-head">融合结论</div>
        <p class="fusion-summary">{{ fusion.summary }}</p>
        <dl class="fusion-meta">
          <div><dt>共识度</dt><dd>{{ consensusText(fusion, results.length) }}</dd></div>
          <div v-if="fusion.dominantView"><dt>主导观点</dt><dd>{{ fusion.dominantView }}</dd></div>
          <div v-if="fusion.confidence"><dt>融合置信度</dt><dd>{{ confidenceText(fusion.confidence) }}</dd></div>
          <div v-if="fusion.blocked || fusion.vetoed"><dt>治理</dt><dd>该结论已被治理闸门标记</dd></div>
        </dl>
      </div>
    </div>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="$emit('close')" :disabled="submitting">关闭</el-button>
        <el-tooltip :disabled="!problem" :content="problem" placement="top">
          <span>
            <el-button
              type="primary"
              :loading="submitting"
              :disabled="!!problem"
              @click="$emit('start')"
            >
              <el-icon><Connection /></el-icon>
              <span>开始咨询</span>
            </el-button>
          </span>
        </el-tooltip>
      </div>
    </template>
  </el-dialog>
</template>

<script setup>
import { computed } from 'vue'
import { CircleCheckFilled, DocumentCopy, Connection } from '@element-plus/icons-vue'
import {
  availabilityLabel, confidenceText, consensusText
} from '@/modules/expert-alliance/contract'
import { expertPickable, expertVisualKey } from '@/modules/expert-alliance/model'
import { expertColor, expertEmoji } from '@/constants'

const props = defineProps({
  visible: { type: Boolean, default: false },
  question: { type: String, default: '' },
  selectedExpertIds: { type: Array, default: () => [] },
  problem: { type: String, default: '' },
  compareView: { type: Boolean, default: false },
  results: { type: Array, default: () => [] },
  fusion: { type: Object, default: null },
  submitting: { type: Boolean, default: false },
  experts: { type: Array, default: () => [] }
})

const emit = defineEmits(['close', 'start', 'toggle-expert', 'update:question', 'update:compareView'])

const pickableExperts = computed(() => props.experts.filter(expertPickable))
const expertById = computed(() => new Map(props.experts.map((e) => [e.id, e])))

// 投影行只带 id/名字，配色要回查花名册那一行的领域键
function rowVisualKey(row) {
  return expertVisualKey(expertById.value.get(row.id)) || row.name
}
function rowColor(row) {
  return expertColor(rowVisualKey(row))
}
</script>

<style scoped>
.chip-status,
.consult-count-hint {
  font-size: 11px;
  color: var(--el-text-color-secondary);
}
.row-badges {
  display: flex;
  gap: 6px;
  margin-top: 8px;
}
.fusion-block {
  margin-top: 12px;
  padding: 10px 12px;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 8px;
}
.fusion-head {
  font-size: 13px;
  font-weight: 600;
}
.fusion-summary {
  margin: 6px 0;
  font-size: 13px;
  white-space: pre-wrap;
}
.fusion-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin: 0;
  font-size: 12px;
}
.fusion-meta dt {
  color: var(--el-text-color-secondary);
  display: inline;
}
.fusion-meta dd {
  display: inline;
  margin: 0 0 0 4px;
}
</style>
