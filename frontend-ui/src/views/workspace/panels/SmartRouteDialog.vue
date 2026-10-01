<!--
  智能匹配专家对话框
  职责：问题描述输入、智能路由匹配、候选专家展示与选择。
  路由只排序不作答（后端 route_query 不产回复），因此这里不出现任何"回复"字段；
  候选行取自 model/collabLists 的投影，推荐数量上下界取自契约。
-->
<template>
  <el-dialog
    :model-value="visible"
    @update:model-value="emit('close')"
    title="智能匹配专家"
    width="500px"
    :close-on-click-modal="!loading"
    class="smart-route-dialog"
  >
    <div class="smart-route-intro">
      <div class="intro-icon">🧠</div>
      <div class="intro-text">
        <div class="intro-title">智能路由</div>
        <div class="intro-desc">按问题从注册表挑选候选专家并给出推荐序，不生成回复</div>
      </div>
    </div>

    <el-form label-width="88px" label-position="right">
      <el-form-item label="问题描述" required>
        <el-input
          :model-value="question"
          type="textarea"
          :rows="3"
          placeholder="请描述您的问题或需求…"
          maxlength="300"
          show-word-limit
          resize="none"
          @update:model-value="emit('update:question', $event)"
          @keyup.enter.ctrl="$emit('do-route')"
        />
      </el-form-item>

      <el-form-item label="候选数量">
        <el-input-number
          :model-value="maxExperts"
          :min="bound.min"
          :max="bound.max"
          size="small"
          @update:model-value="emit('update:maxExperts', $event)"
        />
        <span class="form-hint">位专家</span>
      </el-form-item>
    </el-form>

    <div class="smart-route-action">
      <el-tooltip :disabled="!problem" :content="problem" placement="top">
        <span>
          <el-button
            type="primary"
            :loading="loading"
            :disabled="!!problem"
            @click="$emit('do-route')"
            class="smart-route-btn"
          >
            <el-icon><Compass /></el-icon>
            <span>{{ loading ? '匹配中…' : '开始智能匹配' }}</span>
          </el-button>
        </span>
      </el-tooltip>
    </div>

    <!-- 匹配结果 -->
    <div v-if="candidates.length" class="smart-route-results">
      <div class="route-result-head">
        <span class="route-result-title">匹配结果</span>
        <el-tag size="small" type="success" effect="light">
          {{ candidates.length }} 位候选
        </el-tag>
        <span v-if="result && result.totalScanned" class="route-scanned">
          已扫描注册表 {{ result.totalScanned }} 位
        </span>
      </div>

      <div class="route-expert-list">
        <div
          v-for="(item, idx) in candidates"
          :key="item.key"
          class="route-expert-item"
          :class="{ recommended: item.recommended }"
        >
          <div class="route-rank">{{ idx + 1 }}</div>
          <div class="route-avatar" :style="{ background: expertColor(item.visualKey) }">
            {{ expertEmoji(item.visualKey) }}
          </div>
          <div class="route-info">
            <div class="route-name">
              {{ item.name }}
              <el-tag v-if="item.recommended" size="small" type="warning" effect="dark">推荐</el-tag>
            </div>
            <div class="route-type">{{ item.subtitle || '专家' }} · {{ availabilityLabel(item.status) }}</div>
            <div v-if="item.reason" class="route-reason">{{ item.reason }}</div>
            <div v-if="item.avgRating" class="route-rating">评分 {{ item.avgRating.toFixed(2) }}</div>
          </div>
          <div class="route-score">
            <div class="score-ring" :style="{ '--score': item.matchScore }">
              <span>{{ (item.matchScore * 100).toFixed(0) }}%</span>
            </div>
            <span class="score-label">匹配度</span>
          </div>
          <el-button
            size="small"
            type="primary"
            plain
            class="route-select-btn"
            @click="$emit('select-expert', item)"
          >选择</el-button>
        </div>
      </div>

      <div class="route-actions-footer">
        <el-button size="small" @click="$emit('select-all')">
          <el-icon><CircleCheckFilled /></el-icon>
          一键选择全部候选
        </el-button>
      </div>
    </div>

    <div v-else-if="loading" class="smart-route-loading">
      <el-icon class="is-loading loading-spinner"><Loading /></el-icon>
      <span>正在分析您的问题并匹配专家…</span>
    </div>
  </el-dialog>
</template>

<script setup>
import { computed } from 'vue'
import { Compass, CircleCheckFilled, Loading } from '@element-plus/icons-vue'
import { COLLAB_MODE, availabilityLabel, collabMode } from '@/modules/expert-alliance/contract'
import { expertColor, expertEmoji } from '@/constants'

const props = defineProps({
  visible: { type: Boolean, default: false },
  question: { type: String, default: '' },
  maxExperts: { type: Number, default: 5 },
  problem: { type: String, default: '' },
  loading: { type: Boolean, default: false },
  candidates: { type: Array, default: () => [] },
  result: { type: Object, default: null }
})

const emit = defineEmits(['close', 'do-route', 'select-expert', 'select-all', 'update:question', 'update:maxExperts'])

// 上下界与后端 clamp 同序：这里允许填的区间就是后端会用的区间
const bound = computed(() =>
  collabMode(COLLAB_MODE.ROUTE).controls.find((c) => c.wire === 'max_experts')
)
</script>

<style scoped>
.route-scanned {
  margin-left: 8px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.route-rating {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.route-expert-item.recommended {
  border-color: var(--el-color-warning-light-5);
}
</style>
