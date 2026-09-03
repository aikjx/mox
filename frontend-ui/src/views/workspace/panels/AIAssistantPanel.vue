<!--
  AI 助手浮窗
  职责：快捷指令、联盟能力展示
-->
<template>
  <div v-if="visible" class="ws-ai-assistant">
    <div class="ws-ai-header">
      <span class="ws-ai-icon">🤖</span>
      <span class="ws-ai-title">AI 协作助手</span>
      <button class="ws-ai-close" @click="$emit('close')">
        <el-icon><Close /></el-icon>
      </button>
    </div>
    <div class="ws-ai-body">
      <div class="ws-ai-suggestions">
        <div class="ws-ai-suggest-title">快捷指令</div>
        <div class="ws-ai-suggest-grid">
          <button class="ws-ai-suggest-btn" @click="$emit('suggestion', '分析图谱')">
            <span class="ws-suggest-icon">🔍</span>
            分析当前图谱
          </button>
          <button class="ws-ai-suggest-btn" @click="$emit('suggestion', '推荐专家')">
            <span class="ws-suggest-icon">👥</span>
            推荐相关专家
          </button>
          <button class="ws-ai-suggest-btn" @click="$emit('suggestion', '生成报告')">
            <span class="ws-suggest-icon">📝</span>
            生成研究报告
          </button>
          <button class="ws-ai-suggest-btn" @click="$emit('suggestion', '知识问答')">
            <span class="ws-suggest-icon">❓</span>
            知识问答
          </button>
        </div>
      </div>
      <div class="ws-ai-capabilities">
        <div class="ws-ai-suggest-title">联盟能力</div>
        <div class="ws-ai-cap-list">
          <div v-for="cap in capabilities" :key="cap" class="ws-cap-item">
            <el-icon><CircleCheckFilled /></el-icon>
            <span>{{ cap }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { Close, CircleCheckFilled } from '@element-plus/icons-vue'

defineProps({
  visible: { type: Boolean, default: false },
  capabilities: { type: Array, default: () => [] }
})

defineEmits(['close', 'suggestion'])
</script>
