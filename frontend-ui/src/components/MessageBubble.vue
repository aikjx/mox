<template>
  <div class="msg" :class="[role, { system: msg.system }]" v-if="msg.system">
    <div class="system-msg">
      <div class="system-content" v-html="rendered"></div>
      <div v-if="msg.task_data" class="task-quick-actions">
        <el-button v-if="msg.task_data" size="small" type="primary" text @click="$emit('goto-task', msg.task_id)">
          查看任务
        </el-button>
      </div>
    </div>
  </div>
  <div class="msg" :class="role" v-else>
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
      <div v-if="webSources.length" class="web-sources">
        <div class="ws-head">
          <el-icon><Link /></el-icon>
          <span>联网检索（{{ msg.web_search.engine }} · {{ msg.web_search.duration_ms }}ms）</span>
        </div>
        <a v-for="(s, i) in webSources" :key="i" class="ws-item" :href="s.url" target="_blank" rel="noopener">
          <span class="ws-idx">[{{ i + 1 }}]</span>
          <span class="ws-title">{{ s.title || s.url }}</span>
        </a>
      </div>
      <div v-else-if="msg.web_search && msg.web_search.error" class="web-sources ws-error">
        <span>联网检索失败：{{ msg.web_search.error }}</span>
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
import { User, Cpu, Link } from '@element-plus/icons-vue'

const props = defineProps({
  msg: { type: Object, required: true },
})

defineEmits(['goto-task'])

const role = computed(() => props.msg.role)

const webSources = computed(() => {
  const ws = props.msg.web_search
  if (!ws || !ws.enabled || !Array.isArray(ws.sources)) return []
  return ws.sources.slice(0, 6)
})

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
    .replace(/\*([^*]+)\*/g, '<em>$1</em>')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank">$1</a>')
}

// 检测 Mermaid 图表
const MERMAID_REGEX = /^\s*(graph|flowchart|sequenceDiagram|classDiagram|stateDiagram|erDiagram|gantt|pie|journey)\b/

// 轻量 markdown：代码块、列表、加粗、表格、标题、Mermaid、换行
const rendered = computed(() => {
  let text = props.msg.content || ''
  const blocks = []
  
  // 提取代码块和 mermaid 块
  text = text.replace(/```mermaid\n([\s\S]*?)```/g, (_, code) => {
    blocks.push({ type: 'mermaid', content: code.trim() })
    return ' BLOCK' + (blocks.length - 1) + ' '
  })
  text = text.replace(/```([\s\S]*?)```/g, (_, code) => {
    blocks.push({ type: 'code', content: code.replace(/^\n/, '') })
    return ' BLOCK' + (blocks.length - 1) + ' '
  })
  
  const lines = text.split('\n')
  let html = ''
  let inUl = false
  let inOl = false
  let inTable = false
  let tableHeader = false
  const tableRows = []
  
  const closeLists = () => {
    if (inUl) { html += '</ul>'; inUl = false }
    if (inOl) { html += '</ol>'; inOl = false }
  }
  const closeTable = () => {
    if (inTable) {
      html += '<table>'
      for (const row of tableRows) {
        html += '<tr>'
        for (const cell of row) {
          html += `<td>${inline(esc(cell))}</td>`
        }
        html += '</tr>'
      }
      html += '</table>'
      tableRows.length = 0
      inTable = false
      tableHeader = false
    }
  }
  
  for (const raw of lines) {
    const bMatch = raw.match(/^\s*BLOCK(\d+)\s*$/)
    if (bMatch) {
      closeLists()
      closeTable()
      const block = blocks[+bMatch[1]]
      if (block.type === 'mermaid') {
        html += `<div class="mermaid-block" data-mermaid="${encodeURIComponent(block.content)}">`
        html += '<pre style="background:#f8fafc;border:1px solid #e2e8f0;border-radius:8px;padding:12px;font-size:12px;color:#475569;white-space:pre-wrap;word-break:break-word;">'
        html += '📊 Mermaid 流程图<br/>'
        html += '<code style="color:#64748b;font-family:monospace;">' + esc(block.content) + '</code>'
        html += '</pre>'
        html += '</div>'
      } else {
        html += '<pre class="code">' + esc(block.content) + '</pre>'
      }
      continue
    }
    
    // 检测表格行
    if (/^\s*\|.+\|\s*$/.test(raw)) {
      closeLists()
      const cells = raw.replace(/^\s*\|/, '').replace(/\|\s*$/, '').split('|').map(c => c.trim())
      
      // 检测是否为分隔行
      if (/^\s*\|[\s-|:]+\|\s*$/.test(raw)) {
        tableHeader = true
        continue
      }
      
      if (!inTable) {
        inTable = true
        tableRows.length = 0
      }
      
      if (tableHeader && tableRows.length === 0) {
        // 第一行作为表头
        html += '<table class="md-table"><thead><tr>'
        for (const cell of cells) {
          html += `<th>${inline(esc(cell))}</th>`
        }
        html += '</tr></thead><tbody>'
        tableHeader = false
      } else {
        tableRows.push(cells)
      }
      continue
    } else if (inTable) {
      closeTable()
    }
    
    // 检测标题
    const headingMatch = raw.match(/^(#{1,6})\s+(.+)/)
    if (headingMatch) {
      closeLists()
      const level = headingMatch[1].length
      const content = inline(headingMatch[2].trim())
      html += `<h${level} class="md-heading md-h${level}">${content}</h${level}>`
      continue
    }
    
    // 检测水平线
    if (/^\s*---+\s*$/.test(raw)) {
      closeLists()
      html += '<hr class="md-hr"/>'
      continue
    }
    
    // 检测引用
    if (/^\s*>\s+/.test(raw)) {
      closeLists()
      html += '<blockquote class="md-blockquote">' + inline(esc(raw.replace(/^\s*>\s+/, ''))) + '</blockquote>'
      continue
    }
    
    // 检测分割线后的强调文本
    if (/^\*\*\*.+\*\*\*$/.test(raw.trim())) {
      closeLists()
      html += '<div class="md-emphasis">' + inline(esc(raw.trim())) + '</div>'
      continue
    }
    
    if (/^\s*[-*]\s+/.test(raw)) {
      closeLists()
      if (!inUl) { html += '<ul>'; inUl = true }
      html += '<li>' + inline(esc(raw.replace(/^\s*[-*]\s+/, ''))) + '</li>'
    } else if (/^\s*\d+\.\s+/.test(raw)) {
      closeLists()
      if (!inOl) { html += '<ol>'; inOl = true }
      html += '<li>' + inline(esc(raw.replace(/^\s*\d+\.\s+/, ''))) + '</li>'
    } else {
      closeLists()
      closeTable()
      html += raw.trim() === '' ? '<br/>' : '<p>' + inline(esc(raw)) + '</p>'
    }
  }
  closeLists()
  closeTable()
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
.content :deep(.md-table) {
  width: 100%; border-collapse: collapse; margin: 10px 0; font-size: 13px;
}
.content :deep(.md-table th) {
  background: #f1f5f9; font-weight: 700; text-align: left;
  padding: 8px 12px; border: 1px solid #e2e8f0; color: #334155;
}
.content :deep(.md-table td) {
  padding: 6px 12px; border: 1px solid #e2e8f0; color: #475569;
}
.content :deep(.md-table tr:nth-child(even)) td {
  background: #f8fafc;
}
.content :deep(.md-heading) {
  margin: 12px 0 8px; font-weight: 700; color: #0f172a; line-height: 1.3;
}
.content :deep(.md-h1) { font-size: 20px; border-bottom: 2px solid #e2e8f0; padding-bottom: 6px; }
.content :deep(.md-h2) { font-size: 17px; color: #1e293b; }
.content :deep(.md-h3) { font-size: 15px; color: #334155; }
.content :deep(.md-h4), .content :deep(.md-h5), .content :deep(.md-h6) { font-size: 14px; }
.content :deep(.md-hr) {
  border: none; border-top: 2px solid #e2e8f0; margin: 14px 0;
}
.content :deep(.md-blockquote) {
  border-left: 4px solid #7c3aed; padding: 8px 14px; margin: 10px 0;
  background: #f5f3ff; border-radius: 0 8px 8px 0; color: #4c1d95; font-style: italic;
}
.content :deep(.md-emphasis) {
  text-align: center; font-weight: 700; color: #7c3aed; margin: 12px 0;
}
.content :deep(.mermaid-block) {
  margin: 10px 0; border-radius: 8px; overflow: hidden;
}
.ops { margin-top: 8px; display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
.web-sources {
  margin-top: 10px; padding: 8px 10px;
  border-top: 1px dashed var(--border, #e2e8f0);
  display: flex; flex-direction: column; gap: 5px;
}
.ws-head {
  display: flex; align-items: center; gap: 5px;
  font-size: 12px; color: #0891b2; font-weight: 600;
}
.ws-item {
  display: flex; gap: 6px; align-items: baseline;
  font-size: 12.5px; color: #475569; text-decoration: none;
  line-height: 1.5; overflow: hidden;
}
.ws-item:hover .ws-title { color: #0891b2; text-decoration: underline; }
.ws-idx { color: #0891b2; font-weight: 600; flex-shrink: 0; }
.ws-title {
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  max-width: 100%; transition: color 0.15s;
}
.ws-error { font-size: 12px; color: #b45309; }
.ops-label { font-size: 12px; color: var(--text-dim); }
.tag {
  font-size: 12px; background: var(--brand-soft); color: var(--brand-dark);
  padding: 1px 8px; border-radius: 6px;
}
.conf { margin-top: 8px; display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-dim); }
.conf-val { font-weight: 600; color: var(--brand); }

/* 系统消息样式 */
.msg.system {
  flex-direction: column;
  margin: 10px 0;
}
.system-msg {
  background: linear-gradient(135deg, #f0f9ff, #f0fdf4);
  border: 1px solid #bae6fd;
  border-radius: 10px;
  padding: 10px 14px;
  max-width: 90%;
  margin: 0 auto;
  box-shadow: 0 1px 3px rgba(0,0,0,0.04);
}
.system-content {
  font-size: 13px;
  line-height: 1.7;
  color: #0369a1;
  white-space: pre-wrap;
}
.system-content :deep(p) { margin: 2px 0; }
.task-quick-actions {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px dashed #bae6fd;
  display: flex;
  justify-content: flex-end;
}
</style>
