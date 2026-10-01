<!--
  图谱画布面板（工作台侧外壳）
  职责：只负责工作台特有的视口与动作条；图谱渲染、字段口径、布局与邻域详情全部来自联盟模块
  （components/GraphCanvas.vue、components/GraphNodeInspector.vue、store/alliance-graph.store.js）。
  这里不再自己画 SVG、不再自己归一化后端字段——那两份重复实现曾在标签上裁出 'clou'/'????'、
  在类型上读错 node_type、在空态上抛 ReferenceError，并把 setTimeout 演的"分析"当成结果。
-->
<template>
  <div class="ws-graph-section">
    <!-- 视口工具条：只摆后端撑得住的动作（增删节点/边、四种布局切换、"图谱分析"都已随重复实现一起退场）-->
    <div class="ws-canvas-toolbar">
      <div class="ws-canvas-tools-left">
        <button class="ws-canvas-tool" title="放大" @click="$emit('zoom-in')">
          <el-icon><ZoomIn /></el-icon>
        </button>
        <button class="ws-canvas-tool" title="缩小" @click="$emit('zoom-out')">
          <el-icon><ZoomOut /></el-icon>
        </button>
        <button class="ws-canvas-tool" title="复原视图" @click="$emit('fit-view')">
          <el-icon><FullScreen /></el-icon>
        </button>
      </div>

      <div class="ws-canvas-tools-right">
        <div class="ws-graph-stats">
          <span class="ws-stat-item"><strong>{{ graphStats.nodes }}</strong> 节点</span>
          <span class="ws-stat-divider">·</span>
          <span class="ws-stat-item"><strong>{{ graphStats.edges }}</strong> 关系</span>
          <span class="ws-stat-divider">·</span>
          <span class="ws-stat-item"><strong>{{ graphStats.expertNodes }}</strong> 专家</span>
          <span class="ws-stat-divider">·</span>
          <span class="ws-stat-item"><strong>{{ graphStats.domainNodes }}</strong> 能力域</span>
          <span v-if="graphStats.density" class="ws-stat-divider">·</span>
          <span v-if="graphStats.density" class="ws-stat-item">密度 {{ graphStats.density.toFixed(3) }}</span>
        </div>
        <el-button size="small" plain @click="$emit('open-graph-workbench')">
          <el-icon><DataAnalysis /></el-icon>
          图谱工作台
        </el-button>
      </div>
    </div>

    <!-- 画布区：拖拽平移、滚轮缩放由外壳掌管，连线与节点仍交给模块组件按布局绘制 -->
    <div
      class="ws-graph-canvas"
      @mousedown="$emit('canvas-mousedown', $event)"
      @mousemove="$emit('canvas-mousemove', $event)"
      @mouseup="$emit('canvas-mouseup')"
      @wheel="$emit('canvas-wheel', $event)"
    >
      <div class="ws-graph-viewport" :style="viewportStyle">
        <GraphCanvas
          :layout="store.layout"
          :selected-id="store.selectedId"
          @select="$emit('select-node', $event)"
        />
      </div>

      <el-alert v-if="store.error.graph" class="ws-graph-alert" type="error" show-icon :closable="false"
        title="协作图谱获取失败" :description="`${store.error.graph}；工作台不清空其他面板。`">
        <el-button size="small" @click="$emit('retry')">重试</el-button>
      </el-alert>

      <p v-else-if="!store.layout.nodes.length && !graphLoading" class="ws-graph-empty">
        图为空：注册表里还没有专家，或图谱尚未构建。可在专家中心注册专家后到「图谱工作台」点「重建图谱」。
      </p>

      <div v-if="graphLoading" class="ws-graph-loading">
        <el-icon class="is-loading ws-loading-icon"><Loading /></el-icon>
        <span>加载图谱数据…</span>
      </div>
    </div>

    <!-- 选中节点：邻域与协作者直接复用模块检视器，动作条只留工作台特有的两条 -->
    <div v-if="selectedNode" class="ws-graph-inspector">
      <div class="ws-graph-inspector-head">
        <span class="ws-graph-inspector-title">{{ graphNodeLabel(selectedNode) }}</span>
        <el-button size="small" text @click="$emit('clear-selected-node')">收起</el-button>
      </div>
      <GraphNodeInspector :store="store" />
      <div class="ws-graph-actions">
        <el-button size="small" @click="$emit('view-node-docs', selectedNode)">
          <el-icon><Document /></el-icon>
          按名称检索知识库
        </el-button>
        <el-button size="small" type="primary" @click="$emit('ask-experts-about', selectedNode)">
          <el-icon><ChatDotRound /></el-icon>
          带这个节点去协作
        </el-button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import {
  ZoomIn, ZoomOut, FullScreen, DataAnalysis, Document, ChatDotRound, Loading
} from '@element-plus/icons-vue'
import { graphNodeLabel } from '@/modules/expert-alliance/contract'
import { GraphCanvas, GraphNodeInspector } from '@/modules/expert-alliance/components'

const props = defineProps({
  store: { type: Object, required: true },
  graphStats: { type: Object, required: true },
  graphLoading: { type: Boolean, default: false },
  viewportStyle: { type: Object, default: () => ({}) }
})

defineEmits([
  'zoom-in', 'zoom-out', 'fit-view', 'retry', 'open-graph-workbench',
  'canvas-mousedown', 'canvas-mousemove', 'canvas-mouseup', 'canvas-wheel',
  'select-node', 'clear-selected-node', 'view-node-docs', 'ask-experts-about'
])

const selectedNode = computed(() => props.store.selectedNode)
</script>

<style scoped>
.ws-graph-viewport { will-change: transform; }
.ws-graph-alert { margin: 8px 0 0; }
.ws-graph-empty { margin: 8px 0 0; font-size: 12px; color: var(--text-muted); }
.ws-graph-inspector {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--border-light);
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.ws-graph-inspector-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.ws-graph-inspector-title { font-size: 14px; font-weight: 600; color: var(--text-primary); }
.ws-graph-actions { display: flex; flex-wrap: wrap; gap: 8px; }
</style>
