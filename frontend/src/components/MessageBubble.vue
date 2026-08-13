<template>
  <div class="msg" :class="role">
    <div class="avatar">
      <el-icon v-if="role === 'user'"><User /></el-icon>
      <el-icon v-else><Cpu /></el-icon>
    </div>
    <div class="bubble">
      <div class="head">
        <span class="name">{{ role === 'user' ? '我' : '算子智能体' }}</span>
        <span class="time">{{ fmtTime }}</span>
      </div>
      <div class="content" v-html="rendered"></div>
      <div v-if="msg.referenced_operators && msg.referenced_operators.length" class="ops">
        <span class="ops-label">引用算子：</span>
        <span v-for="op in msg.referenced_operators" :key="op" class="tag">{{ op }}</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { User, Cpu } from '@element-plus/icons-vue'

const props = defineProps({
  msg: { type: Object, required: true },
})

const role = computed(() => props.msg.role)

const fmtTime = computed(() => {
  const t = props.msg.timestamp
  if (!t) return ''
  const d = new Date(t)
  return isNaN(d) ? '' : d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
})

// 简单换行 + 代码块渲染（不做复杂 markdown，保证安全）
const rendered = computed(() => {
  const text = (props.msg.content || '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  return text
    .replace(/```([\s\S]*?)```/g, '<pre class="code">$1</pre>')
    .replace(/\n/g, '<br/>')
})
</script>

<style scoped>
.msg {
  display: flex;
  gap: 10px;
  margin: 14px 0;
}
.msg.user {
  flex-direction: row-reverse;
}
.avatar {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  background: rgba(99, 102, 241, 0.18);
  color: var(--primary-2);
  border: 1px solid rgba(99, 102, 241, 0.35);
}
.msg.assistant .avatar {
  background: rgba(6, 182, 212, 0.16);
  color: var(--accent);
  border-color: rgba(6, 182, 212, 0.35);
}
.bubble {
  max-width: 76%;
  padding: 10px 14px;
  border-radius: 14px;
  background: var(--bg-panel-2);
  border: 1px solid var(--border);
}
.msg.user .bubble {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.22), rgba(99, 102, 241, 0.1));
  border-color: rgba(99, 102, 241, 0.4);
}
.head {
  display: flex;
  gap: 8px;
  align-items: baseline;
  margin-bottom: 4px;
}
.name {
  font-weight: 600;
  font-size: 13px;
}
.time {
  font-size: 11px;
  color: var(--text-dim);
}
.content {
  font-size: 14px;
  line-height: 1.65;
  white-space: normal;
  word-break: break-word;
}
.content :deep(pre.code) {
  background: #0a0f1e;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px;
  overflow: auto;
  font-family: 'JetBrains Mono', monospace;
  font-size: 12.5px;
  margin: 8px 0;
}
.ops {
  margin-top: 8px;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.ops-label {
  font-size: 12px;
  color: var(--text-dim);
}
</style>
