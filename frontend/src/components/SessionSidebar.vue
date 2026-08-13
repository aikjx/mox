<template>
  <div class="sidebar">
    <div class="brand">
      <div class="logo">🧠</div>
      <div class="brand-text">
        <div class="b-title">算子统一系统</div>
        <div class="b-sub">Operator Unified System</div>
      </div>
    </div>

    <div class="new-chat">
      <el-button type="primary" round @click="$emit('new')" class="full">
        <el-icon><Plus /></el-icon> 新建对话
      </el-button>
    </div>

    <div class="sessions">
      <div
        v-for="s in sessions"
        :key="s.id"
        class="session"
        :class="{ active: s.id === activeId }"
        @click="$emit('select', s.id)"
      >
        <el-icon><ChatLineRound /></el-icon>
        <div class="s-info">
          <div class="s-title">{{ s.title || '新会话' }}</div>
          <div class="s-time">{{ s.time }}</div>
        </div>
      </div>
    </div>

    <div class="foot">
      <div class="status" :class="{ ok: online }">
        <i class="dot"></i>{{ online ? '后端已连接' : '连接中…' }}
      </div>
      <div class="ver">v3.0 · MIT</div>
    </div>
  </div>
</template>

<script setup>
import { Plus, ChatLineRound } from '@element-plus/icons-vue'

defineProps({
  sessions: { type: Array, default: () => [] },
  activeId: { type: String, default: '' },
  online: { type: Boolean, default: false },
})
defineEmits(['new', 'select'])
</script>

<style scoped>
.sidebar {
  width: 256px;
  flex-shrink: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-panel);
  border-right: 1px solid var(--border);
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 16px;
  border-bottom: 1px solid var(--border);
}
.logo {
  font-size: 26px;
}
.b-title { font-weight: 700; font-size: 15px; }
.b-sub { font-size: 11px; color: var(--text-dim); }
.new-chat { padding: 12px 14px; }
.full { width: 100%; }
.sessions {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px;
}
.session {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 10px;
  border-radius: 10px;
  cursor: pointer;
  color: var(--text-dim);
  transition: 0.15s;
}
.session:hover { background: var(--bg-panel-2); color: var(--text); }
.session.active {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.25), rgba(99, 102, 241, 0.1));
  color: var(--text);
}
.s-info { min-width: 0; }
.s-title {
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.s-time { font-size: 11px; opacity: 0.6; }
.foot {
  padding: 12px 16px;
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
  color: var(--text-dim);
}
.status { display: flex; align-items: center; gap: 6px; }
.status .dot {
  width: 8px; height: 8px; border-radius: 50%;
  background: var(--warn);
}
.status.ok .dot { background: var(--success); box-shadow: 0 0 8px var(--success); }
</style>
