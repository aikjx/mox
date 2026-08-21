<template>
  <div class="auto">
    <!-- 左：需求对话 -->
    <div class="auto-left">
      <div class="auto-head">
        <el-icon><MagicStick /></el-icon>
        <span>AI 自动化中枢</span>
        <span class="badge">需求驱动 · 自动代码 · 异常自修复</span>
      </div>

      <el-input
        v-model="requirement"
        type="textarea"
        :rows="4"
        resize="none"
        placeholder="用一句话描述你的业务，例如：做一个商城，有商品、购物车、下单、支付、退货"
        @keydown.ctrl.enter.exact.prevent="generate"
      />

      <div class="quick">
        <el-tag
          v-for="q in quick"
          :key="q"
          class="q"
          @click="requirement = q"
        >{{ q }}</el-tag>
      </div>

      <el-button
        type="primary"
        :loading="loading"
        class="gen-btn"
        @click="generate"
      >
        <el-icon><Promotion /></el-icon> 生成自动化方案
      </el-button>

      <!-- 生成后的操作 -->
      <template v-if="assetId">
        <el-divider>已生成资产：{{ assetName }}</el-divider>
        <el-button-group class="ops">
          <el-button :loading="running" @click="runAsset">
            <el-icon><VideoPlay /></el-icon> 沙箱实跑 + 自修复
          </el-button>
          <el-button @click="loadPermissions">
            <el-icon><Lock /></el-icon> 查看 RBAC 权限
          </el-button>
          <el-button @click="copyRefine = !copyRefine">
            <el-icon><EditPen /></el-icon> 继续对话迭代
          </el-button>
        </el-button-group>

        <el-collapse v-if="copyRefine" class="refine-box">
          <el-collapse-item title="在已有方案上补充功能（继续编辑）" name="1">
            <el-input
              v-model="refineText"
              type="textarea"
              :rows="3"
              placeholder="例如：再增加一个积分兑换功能，会员可消耗积分抵现"
            />
            <el-button
              type="primary"
              size="small"
              :loading="refining"
              @click="refine"
            >提交迭代</el-button>
          </el-collapse-item>
        </el-collapse>
      </template>

      <!-- 运行/修复结果 -->
      <el-alert
        v-if="lastRun"
        class="run-alert"
        :type="lastRun.run.success ? 'success' : (lastRun.fix && lastRun.fix.applied ? 'warning' : 'error')"
        :title="runTitle"
        :closable="false"
        show-icon
      >
        <template #default>
          <div v-if="lastRun.fix">
            异常类别：<b>{{ lastRun.fix.category }}</b><br />
            修复方式：<b>{{ fixSourceLabel(lastRun.fix.source) }}</b><br />
            {{ lastRun.fix.note }}
          </div>
          <div v-if="lastRun.run.stderr_tail" class="stderr">
            <pre>{{ lastRun.run.stderr_tail }}</pre>
          </div>
        </template>
      </el-alert>
    </div>

    <!-- 右：结果可视化 -->
    <div class="auto-right">
      <el-empty v-if="!summary" description="输入需求，AI 将自动生成业务处理流程图、处理逻辑、关联权限与代码" />

      <template v-else>
        <el-tabs v-model="tab">
          <el-tab-pane label="流程蓝图" name="flow">
            <el-descriptions :column="2" border size="small">
              <el-descriptions-item label="功能点">{{ summary.feature_count }}</el-descriptions-item>
              <el-descriptions-item label="实体">{{ summary.entity_count }}</el-descriptions-item>
              <el-descriptions-item label="流程节点">{{ summary.node_count }}</el-descriptions-item>
              <el-descriptions-item label="流程连线">{{ summary.edge_count }}</el-descriptions-item>
            </el-descriptions>
            <div class="features">
              <el-tag v-for="f in summary.features" :key="f" class="f">{{ f }}</el-tag>
            </div>
            <pre class="mermaid">{{ mermaid }}</pre>
          </el-tab-pane>

          <el-tab-pane label="自动代码" name="code">
            <el-tabs tab-position="left">
              <el-tab-pane label="Python 主流程" name="py">
                <el-input
                  v-model="editablePython"
                  type="textarea"
                  :rows="20"
                  class="code-edit"
                  @change="onCodeEdit"
                />
                <el-button size="small" type="success" @click="saveCode">保存并回写流程图</el-button>
              </el-tab-pane>
              <el-tab-pane label="SQL 建表" name="sql">
                <pre class="code-view">{{ sqlCode }}</pre>
              </el-tab-pane>
              <el-tab-pane label="Vue 前端" name="vue">
                <pre class="code-view">{{ vueCode }}</pre>
              </el-tab-pane>
            </el-tabs>
          </el-tab-pane>

          <el-tab-pane label="RBAC 权限" name="rbac">
            <el-table v-if="perms.length" :data="permsFlat" size="small" border max-height="420">
              <el-table-column prop="role" label="角色" width="120" />
              <el-table-column prop="resource" label="资源" width="140" />
              <el-table-column prop="action" label="动作" width="120" />
              <el-table-column prop="perm" label="权限串" />
            </el-table>
            <el-empty v-else description="暂无权限，点击左侧「查看 RBAC 权限」" />
          </el-tab-pane>
        </el-tabs>
      </template>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, computed } from 'vue'
import { ElMessage } from 'element-plus'
import {
  automationChat,
  automationRefine,
  automationRun,
  automationPermissions,
  automationUpdate
} from '@/api'

const requirement = ref('')
const loading = ref(false)
const assetId = ref('')
const assetName = ref('')
const summary = ref(null)
const mermaid = ref('')
const editablePython = ref('')
const sqlCode = ref('')
const vueCode = ref('')
const lastRun = ref(null)
const running = ref(false)
const refining = ref(false)
const copyRefine = ref(false)
const refineText = ref('')
const perms = ref([])
const tab = ref('flow')

const quick = [
  '做一个商城，有商品、购物车、下单、支付、退货',
  '做一个博客，有文章、评论、点赞、收藏',
  '做一个会员积分系统，签到、积分兑换、等级权益',
  '做一个工单系统，提交、分配、处理、关闭'
]

const permsFlat = computed(() =>
  perms.value.map((p) => ({
    role: p.role,
    resource: p.resource,
    action: p.action,
    perm: `${p.resource}:${p.action}`
  }))
)

const runTitle = computed(() => {
  if (!lastRun.value) return ''
  const r = lastRun.value
  if (r.run.success) return '运行成功，无异常'
  if (r.fix && r.fix.applied) return `运行异常已自动修复（${fixSourceLabel(r.fix.source)}）`
  return '运行异常，需人工介入'
})

function fixSourceLabel(s) {
  return { rule: '规则兜底补丁', llm: '大模型生成修复', none: '仅分析未修复' }[s] || s
}

async function generate() {
  if (!requirement.value.trim()) {
    ElMessage.warning('请先描述业务需求')
    return
  }
  loading.value = true
  try {
    const resp = await automationChat({ requirement: requirement.value, name: '' })
    assetId.value = resp.asset_id
    assetName.value = resp.name
    summary.value = resp.blueprint_summary
    mermaid.value = resp.mermaid
    editablePython.value = resp.code.python
    sqlCode.value = resp.code.sql
    vueCode.value = resp.code.vue
    ElMessage.success(`已生成 ${resp.blueprint_summary.feature_count} 个功能点、${resp.rbac_count} 条权限`)
    tab.value = 'flow'
  } catch (e) {
    ElMessage.error(e.message || '生成失败')
  } finally {
    loading.value = false
  }
}

async function refine() {
  if (!assetId.value || !refineText.value.trim()) return
  refining.value = true
  try {
    const resp = await automationRefine(assetId.value, {
      requirement: refineText.value,
      name: ''
    })
    summary.value = resp.blueprint_summary
    mermaid.value = resp.mermaid
    assetName.value = resp.name
    editablePython.value = resp.code.python
    sqlCode.value = resp.code.sql
    vueCode.value = resp.code.vue
    ElMessage.success('已迭代更新方案')
  } catch (e) {
    ElMessage.error(e.message || '迭代失败')
  } finally {
    refining.value = false
  }
}

async function runAsset() {
  if (!assetId.value) return
  running.value = true
  try {
    const resp = await automationRun(assetId.value, { timeout_sec: 15 })
    lastRun.value = resp
    if (resp.updated_code_python) {
      editablePython.value = resp.updated_code_python
      ElMessage.success('异常已自动修复并回写')
    } else if (resp.run.success) {
      ElMessage.success('运行成功')
    } else {
      ElMessage.warning('运行异常，未能自动修复')
    }
    tab.value = 'code'
  } catch (e) {
    ElMessage.error(e.message || '运行失败')
  } finally {
    running.value = false
  }
}

async function loadPermissions() {
  if (!assetId.value) return
  try {
    const resp = await automationPermissions(assetId.value)
    perms.value = resp.permissions
    tab.value = 'rbac'
  } catch (e) {
    ElMessage.error(e.message || '获取权限失败')
  }
}

// 用户在前端直接编辑 Python 后，仅本地更新（保存时一并回写后端）
function onCodeEdit() {
  /* 编辑即生效到 editablePython 响应式变量 */
}
// 保存：把前端编辑的代码回写后端资产（实现「可继续编辑流程」持久化）
async function saveCode() {
  if (!assetId.value) return
  try {
    await automationUpdate(assetId.value, {
      python: editablePython.value,
      sql: sqlCode.value,
      vue: vueCode.value
    })
    ElMessage.success('已保存并回写到 AI 自动化资产')
  } catch (e) {
    ElMessage.error(e.message || '保存失败')
  }
}
</script>

<style scoped>
.auto {
  display: grid;
  grid-template-columns: 420px 1fr;
  gap: 16px;
  height: 100%;
}
.auto-left {
  background: #fff;
  border-radius: 12px;
  padding: 16px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.06);
  overflow: auto;
}
.auto-head {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 18px;
  font-weight: 700;
  margin-bottom: 12px;
}
.badge {
  font-size: 11px;
  font-weight: 500;
  color: #f97316;
  background: #ffedd5;
  padding: 2px 8px;
  border-radius: 10px;
}
.quick {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin: 10px 0;
}
.q { cursor: pointer; }
.gen-btn { width: 100%; margin-bottom: 8px; }
.ops { display: flex; width: 100%; }
.ops .el-button { flex: 1; }
.refine-box { margin-top: 10px; }
.run-alert { margin-top: 12px; }
.stderr pre {
  background: #1e293b;
  color: #fca5a5;
  padding: 8px;
  border-radius: 6px;
  max-height: 160px;
  overflow: auto;
  font-size: 12px;
}
.auto-right {
  background: #fff;
  border-radius: 12px;
  padding: 16px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.06);
  overflow: auto;
}
.features { margin: 10px 0; display: flex; flex-wrap: wrap; gap: 6px; }
.mermaid {
  background: #0f172a;
  color: #e2e8f0;
  padding: 12px;
  border-radius: 8px;
  font-size: 12px;
  white-space: pre-wrap;
  line-height: 1.5;
}
.code-view {
  background: #0f172a;
  color: #a5f3fc;
  padding: 12px;
  border-radius: 8px;
  font-size: 12px;
  overflow: auto;
  max-height: 480px;
}
.code-edit { font-family: monospace; }
</style>
