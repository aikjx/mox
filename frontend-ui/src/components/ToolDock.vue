<template>
  <div class="tool-dock" :class="{ collapsed: isCollapsed }">
    <div class="dock-toggle" @click="toggleCollapse" :title="isCollapsed ? '展开工具面板' : '收起工具面板'">
      <el-icon :size="14"><component :is="isCollapsed ? Expand : Fold" /></el-icon>
    </div>
    <div class="dock-scroll" v-show="!isCollapsed">
      <div
        v-for="tool in tools"
        :key="tool.key"
        class="dock-item"
        :class="{ active: activeTool === tool.key }"
        @click="$emit('select', tool.key)"
      >
        <div class="dock-icon" :style="{ background: tool.bg, color: tool.color }">
          <el-icon><component :is="tool.icon" /></el-icon>
        </div>
        <span class="dock-label">{{ tool.label }}</span>
        <span v-if="tool.badge" class="dock-badge">{{ tool.badge }}</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { Odometer, Share, List, Collection, Cpu, Operation, Coin, Monitor, Connection, Briefcase, Fold, Expand } from '@element-plus/icons-vue'

defineProps({
  activeTool: { type: String, default: '' }
})
defineEmits(['select'])

const isCollapsed = ref(false)

function toggleCollapse() {
  isCollapsed.value = !isCollapsed.value
  try { localStorage.setItem('ous_tool_dock_collapsed', isCollapsed.value ? '1' : '0') } catch {}
}

onMounted(() => {
  try {
    const saved = localStorage.getItem('ous_tool_dock_collapsed')
    if (saved === '1') isCollapsed.value = true
  } catch {}
})

const tools = [
  { key: 'project', label: '项目', icon: Briefcase, color: '#7c3aed', bg: '#ede9fe' },
  { key: 'knowledge', label: '知识库', icon: Collection, color: '#0d9488', bg: '#ccfbf1' },
  { key: 'tasks', label: '任务', icon: List, color: '#0ea5e9', bg: '#e0f2fe' },
  { key: 'graph', label: '图谱', icon: Share, color: '#06b6d4', bg: '#ecfeff' },
  { key: 'operators', label: '算子', icon: Cpu, color: '#6366f1', bg: '#eef2ff' },
  { key: 'workflow', label: '工作流', icon: Operation, color: '#f59e0b', bg: '#fffbeb' },
  { key: 'resources', label: '资源', icon: Coin, color: '#10b981', bg: '#ecfdf5' },
  { key: 'plugins', label: '插件', icon: Connection, color: '#8b5cf6', bg: '#f3e8ff' },
  { key: 'monitor', label: '监控', icon: Monitor, color: '#ef4444', bg: '#fef2f2' },
  { key: 'dashboard', label: '总览', icon: Odometer, color: '#4f46e5', bg: '#eef2ff' }
]
</script>

<style scoped>
.tool-dock {
  width: 68px;
  background: linear-gradient(180deg, #f8fafc 0%, #f1f5f9 100%);
  border-left: 1px solid #e2e8f0;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 8px 0;
  flex-shrink: 0;
  transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.tool-dock.collapsed {
  width: 32px;
  padding: 8px 0;
}

.dock-toggle {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  display: grid;
  place-items: center;
  cursor: pointer;
  color: #94a3b8;
  margin-bottom: 6px;
  transition: all 0.2s;
  flex-shrink: 0;
}

.dock-toggle:hover {
  background: rgba(99, 102, 241, 0.1);
  color: #6366f1;
}

.tool-dock.collapsed .dock-toggle {
  margin-bottom: 0;
}

.dock-scroll {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.dock-item {
  position: relative;
  width: 52px;
  height: 52px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  margin: 2px 0;
}

.dock-item:hover {
  background: rgba(99, 102, 241, 0.08);
  transform: translateX(-2px);
}

.dock-item.active {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.12), rgba(6, 182, 212, 0.12));
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.2);
}

.dock-item.active::before {
  content: '';
  position: absolute;
  left: -10px;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 24px;
  background: linear-gradient(180deg, #6366f1, #06b6d4);
  border-radius: 0 3px 3px 0;
}

.dock-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 18px;
  transition: transform 0.2s;
}

.dock-item:hover .dock-icon {
  transform: scale(1.08);
}

.dock-label {
  font-size: 10px;
  margin-top: 3px;
  color: #64748b;
  font-weight: 500;
  white-space: nowrap;
}

.dock-item.active .dock-label {
  color: #4f46e5;
  font-weight: 600;
}

.dock-badge {
  position: absolute;
  top: 4px;
  right: 6px;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  background: #ef4444;
  color: #fff;
  border-radius: 8px;
  font-size: 10px;
  font-weight: 600;
  display: grid;
  place-items: center;
}
</style>
