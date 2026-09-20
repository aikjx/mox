<template>
  <div class="dag-viewer">
    <div class="dag-header">
      <h3>DAG 执行可视化</h3>
      <div class="dag-progress">
        <span>进度: {{ completedCount }}/{{ totalNodes }} 节点完成</span>
        <el-progress :percentage="progressPercent" :stroke-width="8" />
      </div>
    </div>

    <div class="dag-canvas">
      <div
        v-for="node in nodes"
        :key="node.id"
        class="dag-node"
        :class="['status-' + node.status, { 'is-active': node.status === 'running' }]"
        :style="{ left: node.x + 'px', top: node.y + 'px' }"
      >
        <div class="node-status-icon">{{ statusIcon(node.status) }}</div>
        <div class="node-name">{{ node.name }}</div>
        <div v-if="node.duration_ms" class="node-duration">{{ node.duration_ms }}ms</div>
      </div>
    </div>

    <div class="dag-legend">
      <span v-for="item in legend" :key="item.status" class="legend-item">
        <span class="legend-dot" :style="{ background: item.color }"></span>
        {{ item.label }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

interface DagNode {
  id: string
  name: string
  status: 'pending' | 'running' | 'completed' | 'failed' | 'skipped'
  x: number
  y: number
  duration_ms?: number
}

const props = defineProps<{
  nodes?: DagNode[]
}>()

const defaultNodes: DagNode[] = [
  { id: 'node-1', name: '需求分析', status: 'completed', x: 50, y: 50, duration_ms: 1200 },
  { id: 'node-2', name: '架构设计', status: 'running', x: 250, y: 50 },
  { id: 'node-3', name: '数据建模', status: 'pending', x: 250, y: 150 },
  { id: 'node-4', name: '方案评审', status: 'pending', x: 450, y: 100 },
  { id: 'node-5', name: '融合输出', status: 'pending', x: 650, y: 100 },
]

const nodes = ref(props.nodes || defaultNodes)

const completedCount = computed(() =>
  nodes.value.filter(n => n.status === 'completed').length
)
const totalNodes = computed(() => nodes.value.length)
const progressPercent = computed(() =>
  Math.round((completedCount.value / totalNodes.value) * 100)
)

const legend = [
  { status: 'pending', label: '待执行', color: '#909399' },
  { status: 'running', label: '执行中', color: '#409EFF' },
  { status: 'completed', label: '已完成', color: '#67C23A' },
  { status: 'failed', label: '失败', color: '#F56C6C' },
  { status: 'skipped', label: '已跳过', color: '#E6A23C' },
]

function statusIcon(status: string): string {
  const icons: Record<string, string> = {
    pending: '○',
    running: '⏱',
    completed: '✓',
    failed: '✗',
    skipped: '⊘',
  }
  return icons[status] || '○'
}
</script>

<style scoped>
.dag-viewer {
  background: #fff;
  border-radius: 8px;
  padding: 16px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}

.dag-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.dag-header h3 {
  margin: 0;
  font-size: 16px;
  color: #303133;
}

.dag-progress {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 200px;
}

.dag-canvas {
  position: relative;
  height: 250px;
  background: #f8f9fa;
  border-radius: 4px;
  overflow: hidden;
}

.dag-node {
  position: absolute;
  width: 120px;
  padding: 8px 12px;
  border-radius: 6px;
  background: #fff;
  border: 2px solid #909399;
  text-align: center;
  transition: all 0.3s ease;
}

.dag-node.status-completed {
  border-color: #67C23A;
  background: #f0f9eb;
}

.dag-node.status-running {
  border-color: #409EFF;
  background: #ecf5ff;
  animation: pulse 1.5s infinite;
}

.dag-node.status-failed {
  border-color: #F56C6C;
  background: #fef0f0;
}

.dag-node.status-skipped {
  border-color: #E6A23C;
  background: #fdf6ec;
  opacity: 0.7;
}

.node-status-icon {
  font-size: 16px;
  margin-bottom: 4px;
}

.node-name {
  font-size: 13px;
  color: #303133;
  font-weight: 500;
}

.node-duration {
  font-size: 11px;
  color: #909399;
  margin-top: 2px;
}

.dag-legend {
  display: flex;
  gap: 16px;
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid #ebeef5;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #606266;
}

.legend-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

@keyframes pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(64, 158, 255, 0.4); }
  50% { box-shadow: 0 0 0 8px rgba(64, 158, 255, 0); }
}
</style>
