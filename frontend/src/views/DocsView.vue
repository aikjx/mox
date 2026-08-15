<template>
  <div class="docs">
    <div class="head">
      <div>
        <h2 class="page-title">API 文档</h2>
        <p class="page-subtitle">算子统一系统运行时 REST 接口全集 · 点击「试一试」实时调用</p>
      </div>
    </div>

    <el-input
      v-model="kw"
      placeholder="搜索接口路径 / 描述"
      clearable
      :prefix-icon="Search"
      style="max-width: 360px; margin-bottom: 16px"
    />

    <div class="grid grid-2">
      <div class="panel api-card" v-for="a in filtered" :key="a.path">
        <div class="api-top">
          <span class="method" :class="a.method.toLowerCase()">{{ a.method }}</span>
          <code class="path">/api{{ a.path }}</code>
        </div>
        <div class="api-desc">{{ a.desc }}</div>
        <div class="api-foot">
          <span class="tag">{{ a.group }}</span>
          <el-button size="small" type="primary" plain @click="tryIt(a)">试一试</el-button>
        </div>
        <pre v-if="a.show" class="res">{{ a.res || '调用中…' }}</pre>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { Search } from '@element-plus/icons-vue'
import * as api from '@/api'

const kw = ref('')

const endpoints = [
  { method: 'GET', path: '/health', group: '系统', desc: '健康检查', fn: () => api.getHealth() },
  { method: 'GET', path: '/status', group: '系统', desc: '系统状态摘要', fn: () => api.getStatus() },
  { method: 'GET', path: '/status/full', group: '系统', desc: '完整运行时状态', fn: () => api.getFullStatus() },
  { method: 'GET', path: '/logs', group: '系统', desc: '执行日志列表', fn: () => api.getLogs() },
  { method: 'GET', path: '/operators', group: '算子', desc: '算子库列表', fn: () => api.getOperators() },
  { method: 'POST', path: '/operators/register', group: '算子', desc: '注册自定义算子', fn: () => api.registerOperator({ id: 'demo', name: 'Demo', operator_type: 'function' }) },
  { method: 'POST', path: '/execute', group: '算子', desc: '执行算子工作流', fn: () => api.executeWorkflow({ workflow: ['identity', 'relu'], input: [1, 2, -3], parameters: {} }) },
  { method: 'GET', path: '/graph', group: '图谱', desc: '知识图谱数据', fn: () => api.getGraph() },
  { method: 'GET', path: '/graph/stats', group: '图谱', desc: '图谱统计', fn: () => api.getGraphStats() },
  { method: 'POST', path: '/ai/chat', group: 'AI', desc: 'AI 智能对话', fn: () => api.aiChat({ session_id: 'demo', message: '你好' }) },
  { method: 'GET', path: '/ai/resources', group: '资源', desc: '资源全景', fn: () => api.getResources() },
  { method: 'GET', path: '/ai/plugins', group: '插件', desc: 'AI 插件列表', fn: () => api.getAiPlugins() },
  { method: 'GET', path: '/ai/workflows/templates', group: '工作流', desc: '工作流模板', fn: () => api.getWorkflowTemplates() },
  { method: 'GET', path: '/ai/flows', group: '工作流', desc: '流程图列表', fn: () => api.getFlows() },
  { method: 'GET', path: '/ai/llm/config', group: 'AI', desc: 'LLM 配置', fn: () => api.getLlmConfig() },
  { method: 'GET', path: '/ai/browser/templates', group: '浏览器', desc: '浏览器模板', fn: () => api.getBrowserTemplates() },
  { method: 'GET', path: '/ai/browser/sessions', group: '浏览器', desc: '浏览器会话', fn: () => api.getBrowserSessions() },
  { method: 'GET', path: '/plugins', group: '系统', desc: '运行时插件', fn: () => api.getPlugins() }
]

const filtered = computed(() => {
  const k = kw.value.trim().toLowerCase()
  if (!k) return endpoints
  return endpoints.filter(
    (e) => e.path.includes(k) || e.desc.toLowerCase().includes(k) || e.group.toLowerCase().includes(k)
  )
})

async function tryIt(a) {
  a.show = true
  a.res = '调用中…'
  try {
    const r = await a.fn()
    a.res = JSON.stringify(r, null, 2)
  } catch (e) {
    a.res = '错误：' + e.message
  }
  // 触发响应式
  endpoints.splice(0, 0)
}
</script>

<style scoped>
.docs {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.head {
  margin-bottom: 8px;
}
.api-card {
  padding: 16px 18px;
}
.api-top {
  display: flex;
  align-items: center;
  gap: 10px;
}
.method {
  font-size: 11px;
  font-weight: 800;
  padding: 2px 8px;
  border-radius: 6px;
  letter-spacing: 0.5px;
}
.method.get {
  background: #ecfdf5;
  color: #047857;
}
.method.post {
  background: #eef2ff;
  color: #4338ca;
}
.method.delete {
  background: #fef2f2;
  color: #b91c1c;
}
.path {
  font-family: monospace;
  font-size: 13px;
  color: var(--text-1);
  word-break: break-all;
}
.api-desc {
  font-size: 13px;
  color: var(--text-2);
  margin: 10px 0;
}
.api-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.tag {
  font-size: 12px;
  color: var(--text-3);
  background: var(--bg-page);
  padding: 2px 10px;
  border-radius: 999px;
}
.res {
  margin-top: 10px;
  background: #0b1020;
  color: #a5b4fc;
  padding: 10px;
  border-radius: 8px;
  font-size: 11px;
  max-height: 200px;
  overflow: auto;
}
</style>
