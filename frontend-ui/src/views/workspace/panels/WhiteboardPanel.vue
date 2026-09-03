<!--
  白板面板
  职责：便签、文本框、连线、自由画笔、画布交互、保存
-->
<template>
  <div class="ws-whiteboard-content-inner">
    <div class="ws-whiteboard-toolbar">
      <button
        v-for="tool in whiteboardTools"
        :key="tool.key"
        class="ws-wb-tool"
        :class="{ active: activeWbTool === tool.key }"
        :title="tool.label"
        @click="$emit('select-wb-tool', tool.key)"
      >
        <span class="ws-wb-tool-icon">{{ tool.icon }}</span>
        <span class="ws-wb-tool-label">{{ tool.label }}</span>
      </button>
      <div class="ws-wb-tool-divider"></div>
      <div class="ws-wb-color-picker">
        <span class="wb-color-label">颜色</span>
        <button
          v-for="color in wbColors"
          :key="color"
          class="wb-color-dot"
          :class="{ active: activeWbColor === color }"
          :style="{ background: color }"
          @click="$emit('update:activeWbColor', color)"
        ></button>
      </div>
      <div class="ws-wb-tool-divider"></div>
      <button class="ws-wb-tool" title="清空画布" @click="$emit('clear-whiteboard')">
        <el-icon><Delete /></el-icon>
        <span class="ws-wb-tool-label">清空</span>
      </button>
    </div>
    <div
      class="ws-whiteboard-canvas"
      ref="whiteboardRef"
      @mousedown="$emit('wb-mousedown', $event)"
      @mousemove="$emit('wb-mousemove', $event)"
      @mouseup="$emit('wb-mouseup', $event)"
      @mouseleave="$emit('wb-mouseup', $event)"
    >
      <!-- 便签 -->
      <div
        v-for="note in wbNotes"
        :key="note.id"
        class="wb-sticky-note"
        :style="{ left: note.x + 'px', top: note.y + 'px', background: note.color }"
        @mousedown.stop="$emit('start-drag-note', $event, note)"
      >
        <div class="wb-note-header">
          <span class="wb-note-title">{{ note.title || '便签' }}</span>
          <button class="wb-note-delete" @click.stop="$emit('delete-wb-note', note.id)" title="删除">×</button>
        </div>
        <div class="wb-note-content" contenteditable="true" @blur="$emit('update-note-content', $event, note)">{{ note.content }}</div>
      </div>
      <!-- 文本框 -->
      <div
        v-for="text in wbTexts"
        :key="text.id"
        class="wb-text-box"
        :style="{ left: text.x + 'px', top: text.y + 'px', color: text.color }"
        @mousedown.stop="$emit('start-drag-text', $event, text)"
      >
        <div contenteditable="true" @blur="$emit('update-text-content', $event, text)">{{ text.content || '双击编辑文本' }}</div>
        <button class="wb-text-delete" @click.stop="$emit('delete-wb-text', text.id)" title="删除">×</button>
      </div>
      <!-- SVG 连线和画笔 -->
      <svg class="wb-draw-layer" :viewBox="wbViewBox">
        <!-- 连线 -->
        <line
          v-for="line in wbLines"
          :key="line.id"
          :x1="line.x1" :y1="line.y1"
          :x2="line.x2" :y2="line.y2"
          :stroke="line.color"
          stroke-width="2"
          stroke-dasharray="5,5"
        />
        <!-- 自由画笔路径 -->
        <path
          v-for="(path, idx) in wbDrawPaths"
          :key="'path-' + idx"
          :d="path.d"
          :stroke="path.color"
          stroke-width="2"
          fill="none"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <!-- 当前绘制的线 -->
        <path
          v-if="wbCurrentPath"
          :d="wbCurrentPath"
          :stroke="activeWbColor"
          stroke-width="2"
          fill="none"
          stroke-linecap="round"
          stroke-linejoin="round"
          opacity="0.8"
        />
      </svg>
      <!-- 空状态提示 -->
      <div v-if="wbNotes.length === 0 && wbTexts.length === 0 && wbDrawPaths.length === 0 && wbLines.length === 0" class="wb-empty-hint">
        <div class="wb-empty-icon">🎨</div>
        <div class="wb-empty-text">选择工具开始创作</div>
        <div class="wb-empty-tips">便签 · 连线 · 画笔 · 文本</div>
      </div>
    </div>
    <div class="ws-whiteboard-footer">
      <span class="wb-stats">便签: {{ wbNotes.length }} | 文本: {{ wbTexts.length }} | 连线: {{ wbLines.length }}</span>
      <el-button size="small" type="primary" plain @click="$emit('save-whiteboard')">
        <el-icon><CollectionTag /></el-icon>
        保存白板
      </el-button>
    </div>
  </div>
</template>

<script setup>
import { Delete, CollectionTag } from '@element-plus/icons-vue'

defineProps({
  activeWbTool: { type: String, default: 'select' },
  activeWbColor: { type: String, default: '#7c3aed' },
  wbNotes: { type: Array, default: () => [] },
  wbTexts: { type: Array, default: () => [] },
  wbLines: { type: Array, default: () => [] },
  wbDrawPaths: { type: Array, default: () => [] },
  wbCurrentPath: { type: String, default: '' },
  wbViewBox: { type: String, default: '0 0 800 400' }
})

defineEmits([
  'select-wb-tool', 'update:activeWbColor', 'clear-whiteboard',
  'wb-mousedown', 'wb-mousemove', 'wb-mouseup',
  'start-drag-note', 'delete-wb-note', 'update-note-content',
  'start-drag-text', 'delete-wb-text', 'update-text-content',
  'save-whiteboard'
])

const whiteboardTools = [
  { key: 'select', label: '选择', icon: '👆' },
  { key: 'note', label: '便签', icon: '📝' },
  { key: 'line', label: '连线', icon: '➖' },
  { key: 'pen', label: '画笔', icon: '🖌️' },
  { key: 'text', label: '文本', icon: '🔤' },
  { key: 'eraser', label: '橡皮擦', icon: '🧹' }
]

const wbColors = ['#7c3aed', '#06b6d4', '#10b981', '#f59e0b', '#ef4444', '#ec4899', '#64748b']
</script>
