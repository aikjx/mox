<template>
  <div class="assistant-selector">
    <div class="selector-header">
      <div class="selector-title">选择 AI 助手</div>
      <div class="selector-subtitle">不同专家各有所长，按需切换</div>
    </div>

    <div class="assistant-list">
      <div
        v-for="a in assistants"
        :key="a.id"
        class="assistant-card"
        :class="{ active: currentAssistant === a.id }"
        @click="selectAssistant(a.id)"
      >
        <div class="assistant-avatar" :style="{ background: a.gradient }">
          <span class="assistant-emoji">{{ a.emoji }}</span>
        </div>
        <div class="assistant-info">
          <div class="assistant-name">{{ a.name }}</div>
          <div class="assistant-desc">{{ a.desc }}</div>
          <div class="assistant-tags">
            <el-tag v-for="(tag, i) in a.tags" :key="i" size="small" effect="plain">
              {{ tag }}
            </el-tag>
          </div>
        </div>
        <div class="assistant-check" v-if="currentAssistant === a.id">
          <el-icon><Check /></el-icon>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { Check } from '@element-plus/icons-vue'

const props = defineProps({
  modelValue: {
    type: String,
    default: 'general'
  }
})

const emit = defineEmits(['change', 'update:modelValue'])

const currentAssistant = computed(() => props.modelValue)

const assistants = [
  {
    id: 'architect',
    name: '架构师小智',
    emoji: '🏗️',
    desc: '系统架构与技术选型专家',
    tags: ['架构设计', '技术选型', '性能优化'],
    gradient: 'linear-gradient(135deg, #6366f1, #8b5cf6)'
  },
  {
    id: 'analyst',
    name: '分析师小研',
    emoji: '📊',
    desc: '需求分析与业务建模专家',
    tags: ['需求分析', '业务建模', '竞品调研'],
    gradient: 'linear-gradient(135deg, #06b6d4, #0ea5e9)'
  },
  {
    id: 'data',
    name: '数据工程师小数',
    emoji: '🔗',
    desc: '知识图谱与数据工程专家',
    tags: ['图谱构建', '数据治理', 'ETL 设计'],
    gradient: 'linear-gradient(135deg, #10b981, #059669)'
  },
  {
    id: 'product',
    name: '产品经理小策',
    emoji: '💡',
    desc: '产品规划与用户体验专家',
    tags: ['产品设计', '用户体验', '原型规划'],
    gradient: 'linear-gradient(135deg, #f59e0b, #f97316)'
  },
  {
    id: 'devops',
    name: '运维工程师小运',
    emoji: '⚙️',
    desc: '部署运维与安全合规专家',
    tags: ['CI/CD', '容器化', '安全审计'],
    gradient: 'linear-gradient(135deg, #ef4444, #dc2626)'
  },
  {
    id: 'general',
    name: '全能助手小通',
    emoji: '✨',
    desc: '通用任务协调与综合处理',
    tags: ['多轮规划', '工具编排', '任务调度'],
    gradient: 'linear-gradient(135deg, #ec4899, #8b5cf6)'
  }
]

function selectAssistant(id) {
  const selected = assistants.find(a => a.id === id)
  emit('update:modelValue', id)
  emit('change', selected)
}
</script>

<style scoped>
.assistant-selector {
  background: var(--bg-surface, #fff);
  border: 1px solid var(--border-soft, #e2e8f0);
  border-radius: 14px;
  padding: 16px;
}

.selector-header {
  margin-bottom: 14px;
}
.selector-title {
  font-weight: 700;
  font-size: 15px;
  color: var(--text-primary, #1e293b);
  margin-bottom: 2px;
}
.selector-subtitle {
  font-size: 12px;
  color: var(--text-tertiary, #64748b);
}

.assistant-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.assistant-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border: 1px solid var(--border-ghost, #f1f5f9);
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.15s ease;
  position: relative;
}
.assistant-card:hover {
  border-color: var(--border-soft, #e2e8f0);
  background: var(--bg-surface-2, #f8fafc);
}
.assistant-card.active {
  border-color: var(--brand, #6366f1);
  background: var(--brand-50, #eef2ff);
}

.assistant-avatar {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
}
.assistant-emoji {
  font-size: 22px;
}

.assistant-info {
  flex: 1;
  min-width: 0;
}
.assistant-name {
  font-weight: 600;
  font-size: 13px;
  color: var(--text-primary, #1e293b);
  margin-bottom: 2px;
}
.assistant-desc {
  font-size: 11px;
  color: var(--text-tertiary, #64748b);
  margin-bottom: 6px;
}
.assistant-tags {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}
.assistant-tags :deep(.el-tag) {
  font-size: 10px;
  padding: 0 6px;
  height: 18px;
  line-height: 16px;
}

.assistant-check {
  color: var(--brand, #6366f1);
  font-size: 18px;
  flex-shrink: 0;
}
</style>
