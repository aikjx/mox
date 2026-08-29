<template>
  <div class="agent-task-runner">
    <!-- 任务标题 -->
    <div class="task-header">
      <div class="task-icon" :class="status">
        <el-icon v-if="status === 'running'"><Loading class="spin" /></el-icon>
        <el-icon v-else-if="status === 'done'"><CircleCheck /></el-icon>
        <el-icon v-else-if="status === 'error'"><CircleClose /></el-icon>
        <el-icon v-else><MagicStick /></el-icon>
      </div>
      <div class="task-info">
        <div class="task-title">{{ title }}</div>
        <div class="task-status-text">
          <span v-if="status === 'running'">AI 正在执行中…</span>
          <span v-else-if="status === 'done'">已完成 · 共 {{ steps.length }} 步</span>
          <span v-else-if="status === 'error'">执行出错</span>
          <span v-else>准备执行</span>
        </div>
      </div>
      <div class="task-progress" v-if="status === 'running'">
        {{ completedSteps }}/{{ steps.length }}
      </div>
    </div>

    <!-- 执行步骤时间线 -->
    <div class="task-steps">
      <div
        v-for="(step, idx) in steps"
        :key="idx"
        class="step-item"
        :class="step.status"
      >
        <div class="step-indicator">
          <div class="step-dot">
            <el-icon v-if="step.status === 'done'"><Check /></el-icon>
            <el-icon v-else-if="step.status === 'running'" class="spin"><Loading /></el-icon>
            <el-icon v-else-if="step.status === 'error'"><Close /></el-icon>
            <span v-else>{{ idx + 1 }}</span>
          </div>
          <div class="step-line" v-if="idx < steps.length - 1"></div>
        </div>
        <div class="step-content">
          <div class="step-title">{{ step.title }}</div>
          <div class="step-tool" v-if="step.tool">
            <el-tag size="small" effect="plain" :type="step.status === 'error' ? 'danger' : 'info'">
              {{ step.tool }}
            </el-tag>
          </div>
          <div class="step-detail" v-if="step.detail && (step.status === 'done' || step.status === 'running')">
            {{ step.detail }}
          </div>
          <div class="step-result" v-if="step.result && step.status === 'done'">
            <div class="result-label">输出</div>
            <div class="result-content">{{ step.result }}</div>
          </div>
          <div class="step-error" v-if="step.error && step.status === 'error'">
            {{ step.error }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { MagicStick, Loading, CircleCheck, CircleClose, Check, Close } from '@element-plus/icons-vue'

const props = defineProps({
  title: { type: String, default: 'AI 正在处理你的任务' },
  status: { type: String, default: 'running' }, // pending | running | done | error
  steps: {
    type: Array,
    default: () => [
      { title: '理解需求', status: 'done', tool: 'NLP 解析', detail: '提取核心目标和约束条件' },
      { title: '任务拆解', status: 'done', tool: '规划引擎', detail: '拆分为 3 个子任务', result: '生成执行计划' },
      { title: '执行中…', status: 'running', tool: '知识图谱查询' }
    ]
  }
})

const completedSteps = computed(() => props.steps.filter(s => s.status === 'done').length)
</script>

<style scoped>
.agent-task-runner {
  background: var(--bg-surface-2, #f8fafc);
  border: 1px solid var(--border-soft, #e2e8f0);
  border-radius: 12px;
  padding: 16px;
  margin: 8px 0;
}

.task-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}
.task-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 20px;
  flex-shrink: 0;
}
.task-icon.running {
  background: linear-gradient(135deg, var(--brand, #6366f1), var(--accent, #06b6d4));
  color: #fff;
}
.task-icon.done {
  background: linear-gradient(135deg, #10b981, #059669);
  color: #fff;
}
.task-icon.error {
  background: linear-gradient(135deg, #ef4444, #dc2626);
  color: #fff;
}
.task-info {
  flex: 1;
  min-width: 0;
}
.task-title {
  font-weight: 600;
  font-size: 14px;
  color: var(--text-primary, #1e293b);
  margin-bottom: 2px;
}
.task-status-text {
  font-size: 12px;
  color: var(--text-tertiary, #64748b);
}
.task-progress {
  font-size: 12px;
  font-weight: 600;
  color: var(--brand, #6366f1);
  background: var(--brand-50, #eef2ff);
  padding: 4px 10px;
  border-radius: 20px;
}

/* 步骤时间线 */
.task-steps {
  padding-left: 4px;
}
.step-item {
  display: flex;
  gap: 12px;
  position: relative;
}
.step-indicator {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex-shrink: 0;
  width: 24px;
}
.step-dot {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 12px;
  font-weight: 600;
  flex-shrink: 0;
  z-index: 1;
}
.step-item.pending .step-dot {
  background: var(--bg-surface, #fff);
  border: 2px solid var(--border-soft, #e2e8f0);
  color: var(--text-quaternary, #94a3b8);
}
.step-item.running .step-dot {
  background: var(--brand, #6366f1);
  color: #fff;
  box-shadow: 0 0 0 4px var(--brand-50, #eef2ff);
}
.step-item.done .step-dot {
  background: #10b981;
  color: #fff;
}
.step-item.error .step-dot {
  background: #ef4444;
  color: #fff;
}
.step-line {
  flex: 1;
  width: 2px;
  min-height: 24px;
  background: var(--border-soft, #e2e8f0);
  margin: 4px 0;
}
.step-item.done .step-line {
  background: #10b981;
}

.step-content {
  flex: 1;
  padding-bottom: 16px;
  min-width: 0;
}
.step-item:last-child .step-content {
  padding-bottom: 0;
}
.step-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary, #1e293b);
  margin-bottom: 4px;
}
.step-item.pending .step-title {
  color: var(--text-quaternary, #94a3b8);
  font-weight: 500;
}
.step-tool {
  margin-bottom: 4px;
}
.step-detail {
  font-size: 12px;
  color: var(--text-secondary, #475569);
  line-height: 1.6;
  margin-bottom: 4px;
}
.step-result {
  background: var(--bg-surface, #fff);
  border: 1px solid var(--border-ghost, #f1f5f9);
  border-radius: 8px;
  padding: 8px 10px;
  margin-top: 6px;
}
.result-label {
  font-size: 10px;
  font-weight: 600;
  color: var(--text-tertiary, #64748b);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 4px;
}
.result-content {
  font-size: 12px;
  color: var(--text-secondary, #475569);
  line-height: 1.5;
}
.step-error {
  font-size: 12px;
  color: var(--danger, #ef4444);
  background: var(--danger-50, #fef2f2);
  padding: 6px 10px;
  border-radius: 6px;
  margin-top: 4px;
}

.spin {
  animation: spin 1s linear infinite;
}
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
