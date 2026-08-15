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
      <div v-if="msg.confidence != null" class="conf">
        <span class="conf-label">置信度</span>
        <el-progress :percentage="Math.round(msg.confidence * 100)" :stroke-width="6" :show-text="false" style="width:90px" />
        <span class="conf-val">{{ (msg.confidence * 100).toFixed(0) }}%</span>
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

function esc(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}
function inline(s) {
  return s
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
}

// 轻量 markdown：代码块、列表、加粗、行内代码、换行
const rendered = computed(() => {
  let text = props.msg.content || ''
  const blocks = []
  text = text.replace(/```([\s\S]*?)```/g, (_, code) => {
    blocks.push(code.replace(/^\n/, ''))
    return ' BLOCK' + (blocks.length - 1) + ' '
  })
  const lines = text.split('\n')
  let html = ''
  let inUl = false
  let inOl = false
  const closeLists = () => {
    if (inUl) { html += '</ul>'; inUl = false }
    if (inOl) { html += '</ol>'; inOl = false }
  }
  for (const raw of lines) {
    const bMatch = raw.match(/^\s*BLOCK(\d+)\s*$/)
    if (bMatch) {
      closeLists()
      html += '<pre class="code">' + esc(blocks[+bMatch[1]]) + '</pre>'
      continue
    }
    if (/^\s*[-*]\s+/.test(raw)) {
      if (!inUl) { closeLists(); html += '<ul>'; inUl = true }
      html += '<li>' + inline(esc(raw.replace(/^\s*[-*]\s+/, ''))) + '</li>'
    } else if (/^\s*\d+\.\s+/.test(raw)) {
      if (!inOl) { closeLists(); html += '<ol>'; inOl = true }
      html += '<li>' + inline(esc(raw.replace(/^\s*\d+\.\s+/, ''))) + '</li>'
    } else {
      closeLists()
      html += raw.trim() === '' ? '<br/>' : '<p>' + inline(esc(raw)) + '</p>'
    }
  }
  closeLists()
  return html
})
</script>

<style scoped>
.msg { display: flex; gap: 10px; margin: 14px 0; }
.msg.user { flex-direction: row-reverse; }
.avatar {
  width: 36px; height: 36px; border-radius: 10px; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  background: rgba(99, 102, 241, 0.18); color: var(--primary-2);
  border: 1px solid rgba(99, 102, 241, 0.35);
}
.msg.assistant .avatar {
  background: rgba(6, 182, 212, 0.16); color: var(--accent);
  border-color: rgba(6, 182, 212, 0.35);
}
.bubble {
  max-width: 78%; padding: 10px 14px; border-radius: 14px;
  background: var(--bg-panel-2); border: 1px solid var(--border);
}
.msg.user .bubble {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.22), rgba(99, 102, 241, 0.1));
  border-color: rgba(99, 102, 241, 0.4);
}
.head { display: flex; gap: 8px; align-items: baseline; margin-bottom: 4px; }
.name { font-weight: 600; font-size: 13px; }
.time { font-size: 11px; color: var(--text-dim); }
.content { font-size: 14px; line-height: 1.65; word-break: break-word; }
.content :deep(p) { margin: 4px 0; }
.content :deep(ul), .content :deep(ol) { margin: 4px 0; padding-left: 20px; }
.content :deep(li) { margin: 2px 0; }
.content :deep(code) {
  background: rgba(99, 102, 241, 0.12); padding: 1px 5px;
  border-radius: 4px; font-family: monospace; font-size: 12.5px;
}
.content :deep(pre.code) {
  background: #0a0f1e; color: #c7d2fe; border: 1px solid var(--border);
  border-radius: 8px; padding: 10px; overflow: auto;
  font-family: monospace; font-size: 12.5px; margin: 8px 0; white-space: pre;
}
.ops { margin-top: 8px; display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
.ops-label { font-size: 12px; color: var(--text-dim); }
.tag {
  font-size: 12px; background: var(--brand-soft); color: var(--brand-dark);
  padding: 1px 8px; border-radius: 6px;
}
.conf { margin-top: 8px; display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-dim); }
.conf-val { font-weight: 600; color: var(--brand); }
</style>
