<template>
  <div class="page-container">
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">需求编译 · Caomei</h2>
        <p class="page-subtitle">一句话说清系统要做的事，自动编译为蓝图（实体 + 功能点 + 流程图），支持对话式增量精化</p>
      </div>
      <div class="page-header-actions">
        <el-input
          v-model="name"
          placeholder="系统名称（可选）"
          class="name-input"
          clearable
        />
        <el-select v-model="tagInput" multiple placeholder="标签（可选）" class="tag-select" clearable>
          <el-option v-for="t in tagPresets" :key="t" :label="t" :value="t" />
        </el-select>
      </div>
    </div>

    <div class="page-content">

    <!-- 编译输入 -->
    <div class="panel card-pad compile-panel">
      <el-input
        v-model="requirement"
        type="textarea"
        :rows="3"
        placeholder="例如：我要做一个电商系统，包含商品管理、购物车、订单结算、支付、物流跟踪、会员积分、售后退货……"
      />
      <div class="compile-actions">
        <el-button type="primary" :loading="compiling" @click="doCompile">
          <el-icon v-if="!compiling"><MagicStick /></el-icon>
          <span>{{ compiling ? '正在编译蓝图…' : '编译为系统蓝图' }}</span>
        </el-button>
        <el-button :loading="loadingTpl" @click="loadTemplates">模板库</el-button>
        <el-button :disabled="!blueprint" :loading="refining" @click="refiningOpen = true">对话精化</el-button>
        <el-button :disabled="!blueprint" @click="applyToWorkbench">到工作台继续编辑</el-button>
      </div>
    </div>

    <!-- 模板库抽屉 -->
    <el-drawer v-model="tplOpen" title="Caomei 模板库" size="420px">
      <div class="tpl-overview" v-if="tplOverview">
        <div class="tpl-ov-row">
          <el-statistic :value="tplOverview.total || 0" title="模板总量" />
          <el-statistic :value="tplOverview.domain_filter || '全部'" title="领域筛选" />
          <el-statistic :value="tplOverview.keyword_filter || '无'" title="关键词筛选" />
        </div>
        <div class="tpl-hint" v-if="tplOverview.hint">{{ tplOverview.hint }}</div>
      </div>
      <div class="tpl-toolbar">
        <el-input v-model="tplKeyword" placeholder="搜索关键词" clearable size="small" @input="loadTemplates" />
        <el-select v-model="tplDomain" placeholder="领域筛选" clearable size="small" class="tpl-domain" @change="loadTemplates">
          <el-option v-for="d in domains" :key="d" :label="d" :value="d" />
        </el-select>
      </div>
      <div class="tpl-list">
        <div v-for="p in templates" :key="p.id" class="tpl-item">
          <div class="tpl-name">{{ p.name }}</div>
          <div class="tpl-meta">{{ p.category }} · {{ p.version }}</div>
          <div class="tpl-summary">{{ p.summary }}</div>
          <el-button size="small" text type="primary" @click="useTemplate(p)">以它为需求样例编译</el-button>
        </div>
        <el-empty v-if="!templates.length" description="暂无模板" :image-size="60" />
      </div>
    </el-drawer>

    <!-- 精化对话框 -->
    <el-dialog v-model="refiningOpen" title="对话式精化蓝图" width="520px">
      <el-input v-model="addition" type="textarea" :rows="3" placeholder="继续追加功能或调整，例如：再加一个优惠券功能，并且订单支持拆单" />
      <template #footer>
        <el-button @click="refiningOpen = false">取消</el-button>
        <el-button type="primary" :loading="refining" @click="doRefine">应用精化</el-button>
      </template>
    </el-dialog>

    <!-- 蓝图结果 -->
    <div v-if="blueprint" class="blueprint-wrap">
      <div class="panel card-pad">
        <div class="section-head">
          <div>
            <h3 class="section-title">蓝图 · {{ blueprint.name }}</h3>
            <span class="blueprint-id mono">{{ blueprint.blueprint_id }}</span>
          </div>
          <div class="stats">
            <el-statistic :value="blueprint.feature_count" title="功能点" />
            <el-statistic :value="blueprint.entities.length" title="实体" />
            <el-statistic :value="flowStats.nodes" title="流程节点" />
            <el-statistic :value="flowStats.edges" title="流程边" />
          </div>
        </div>
      </div>

      <div class="bp-grid">
        <div class="panel card-pad">
          <h4 class="bp-label">实体（Entities）</h4>
          <div class="entity-list">
            <div v-for="e in blueprint.entities" :key="e" class="entity">{{ e }}</div>
            <span v-if="!blueprint.entities.length" class="muted">无实体</span>
          </div>
        </div>

        <div class="panel card-pad">
          <h4 class="bp-label">功能点（Features）</h4>
          <div class="feat-list">
            <div v-for="(f, i) in blueprint.features" :key="i" class="feat">{{ f }}</div>
            <span v-if="!blueprint.features.length" class="muted">无功能点</span>
          </div>
        </div>
      </div>

      <div class="panel card-pad">
        <h4 class="bp-label">流程图（Flow）</h4>
        <div class="flow-view">
          <div class="flow-col">
            <div class="flow-label">节点</div>
            <div v-for="n in blueprint.flow.nodes" :key="n.id" class="flow-node">
              <span class="fn-kind" :class="'k-' + (n.kind || 'task')">{{ n.kind || 'task' }}</span>
              <span class="fn-name">{{ n.name }}</span>
              <span v-if="n.tool" class="fn-tool mono">{{ n.tool }}</span>
            </div>
          </div>
          <div class="flow-col">
            <div class="flow-label">依赖边</div>
            <div v-for="(e, i) in blueprint.flow.edges" :key="i" class="flow-edge">
              <span class="fe-from">{{ e.from }}</span>
              <el-icon class="fe-arrow"><Right /></el-icon>
              <span class="fe-to">{{ e.to }}</span>
            </div>
            <span v-if="!blueprint.flow.edges.length" class="muted">无边</span>
          </div>
        </div>
      </div>
    </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { ElMessage } from 'element-plus'
import { MagicStick, Right } from '@element-plus/icons-vue'
import { useProject } from '@/composables/projectContext.js'
import { caomeiCompile, caomeiRefine, caomeiTemplates, marketList } from '@/api'

const requirement = ref('')
const name = ref('')
const tagInput = ref([])
const tagPresets = ['电商', 'OA', '制造', '支付', '数据分析', 'AI']
const compiling = ref(false)

const blueprint = ref(null)
const flowStats = computed(() => {
  const flow = blueprint.value?.flow
  if (!flow) return { nodes: 0, edges: 0 }
  return { nodes: flow.nodes?.length || 0, edges: flow.edges?.length || 0 }
})

async function doCompile() {
  if (!requirement.value.trim()) {
    ElMessage.warning('请先描述你的需求')
    return
  }
  compiling.value = true
  try {
    const d = await caomeiCompile({
      requirement: requirement.value,
      name: name.value || undefined,
      tags: tagInput.value.length ? tagInput.value : undefined,
    })
    if (d.success === false) throw new Error(d.error || '编译失败')
    blueprint.value = normalizeBlueprint(d)
    ElMessage.success(`蓝图已生成：${blueprint.value.feature_count} 个功能点`)
  } catch (e) {
    ElMessage.error('编译失败：' + e.message)
  } finally {
    compiling.value = false
  }
}

// 归一化后端蓝图结构：兼容 {blueprint:{...}} 与直接 {...} 两种契约，并兜底默认字段，避免 undefined.length 崩溃
function normalizeBlueprint(d) {
  const bp = (d && typeof d === 'object' && d.blueprint && typeof d.blueprint === 'object') ? d.blueprint : (d || {})
  return {
    name: bp.name || '未命名蓝图',
    blueprint_id: bp.blueprint_id || 'bp-未分配',
    feature_count: Number(bp.feature_count ?? 0) || 0,
    entities: Array.isArray(bp.entities) ? bp.entities : [],
    features: Array.isArray(bp.features) ? bp.features : [],
    flow: {
      nodes: Array.isArray(bp.flow?.nodes) ? bp.flow.nodes : [],
      edges: Array.isArray(bp.flow?.edges) ? bp.flow.edges : [],
    },
  }
}

// 精化
const refiningOpen = ref(false)
const addition = ref('')
const refining = ref(false)
async function doRefine() {
  if (!addition.value.trim()) {
    ElMessage.warning('请输入精化描述')
    return
  }
  refining.value = true
  try {
    const d = await caomeiRefine({
      blueprint_id: blueprint.value.blueprint_id,
      addition: addition.value,
    })
    if (d.success === false) throw new Error(d.error || '精化失败')
    const prev = blueprint.value || {}
    blueprint.value = {
      ...prev,
      feature_count: Number(d.feature_count ?? prev.feature_count) || 0,
      features: [...(Array.isArray(prev.features) ? prev.features : []), ...(Array.isArray(d.added_features) ? d.added_features : [])],
      // 保留原流程图，仅当后端明确返回新 flow 且非空时才替换
      flow: (d.flow && Array.isArray(d.flow.nodes) && d.flow.nodes.length) ? d.flow : (prev.flow || { nodes: [], edges: [] }),
    }
    addition.value = ''
    refiningOpen.value = false
    ElMessage.success(`精化完成，现共 ${blueprint.value.feature_count} 个功能点`)
  } catch (e) {
    ElMessage.error('精化失败：' + e.message)
  } finally {
    refining.value = false
  }
}

// 模板库
const tplOpen = ref(false)
const templates = ref([])
const tplOverview = ref(null)
const loadingTpl = ref(false)
const tplKeyword = ref('')
const tplDomain = ref('')
const domains = ['电商', 'OA', '制造', '支付', '数据分析', 'AI', '供应链', '金融']
async function loadTemplates() {
  loadingTpl.value = true
  tplOpen.value = true
  try {
    const [r, ov] = await Promise.all([
      marketList({ q: tplKeyword.value, category: tplDomain.value }),
      caomeiTemplates({ domain: tplDomain.value, keyword: tplKeyword.value }).catch(() => null)
    ])
    templates.value = r.packages || r.items || (Array.isArray(r) ? r : [])
    tplOverview.value = ov || null
  } catch (e) {
    ElMessage.error('模板加载失败：' + e.message)
  } finally {
    loadingTpl.value = false
  }
}
function useTemplate(p) {
  requirement.value = p.requirement || p.summary || p.name || ''
  name.value = p.name || ''
  tplOpen.value = false
  ElMessage.success('已载入模板需求，点击「编译为系统蓝图」生成')
}

// 到工作台继续编辑：把蓝图 flow 交给工作台加载（通过 localStorage 中转）
function applyToWorkbench() {
  const flow = blueprint.value.flow
  if (!flow) return
  try {
    localStorage.setItem('caomei_draft_flow', JSON.stringify(flow))
    ElMessage.success('蓝图已送达工作台，请打开「工作台」继续编辑')
    // 尝试跳转
    const nav = document.querySelector('a[href="/workbench"]')
    nav?.click?.()
  } catch {
    ElMessage.warning('蓝图保存失败')
  }
}

onMounted(() => {
  // 预置示例需求，降低上手门槛
  if (!requirement.value) {
    requirement.value = '我要做一个电商系统，包含商品管理、购物车、订单结算、支付、物流跟踪、会员积分、售后退货'
  }
})

// ===== 璇玑：以项目为核心的联动 =====
{
  const { onChange: _onProjectChange, ensureProjectContext: _ensureProject } = useProject()
  let _offPj = null
  onMounted(async () => {
    _offPj = _onProjectChange(async () => { null })
    await _ensureProject().catch(() => {})
    null
  })
  const _ob$ = onBeforeUnmount == null ? null : onBeforeUnmount(() => { _offPj && _offPj() })
  // 若脚本未引入 onBeforeUnmount，退化为 window beforeunload 兜底（页面关闭）
  if (typeof onBeforeUnmount === 'undefined') {
    // 不操作：Vue 路由离开时组件 destroy，本作用域已销毁
  }
}
</script>

<style scoped>
.caomei-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.page-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}
.page-title { font-size: 20px; font-weight: 800; margin: 0; }
.page-sub { color: var(--text-3); font-size: 13px; margin: 4px 0 0; }
.page-actions { display: flex; gap: 8px; }
.name-input { width: 180px; }
.tag-select { width: 180px; }
.panel { background: var(--bg-card, #fff); border: 1px solid var(--border); border-radius: 12px; }
.card-pad { padding: 16px; }
.compile-actions { display: flex; gap: 10px; margin-top: 12px; }
.blueprint-wrap { display: flex; flex-direction: column; gap: 14px; }
.section-head { display: flex; align-items: center; justify-content: space-between; }
.section-title { font-size: 16px; font-weight: 700; margin: 0; }
.blueprint-id { font-size: 12px; color: var(--text-3); }
.stats { display: flex; gap: 28px; }
.bp-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
.bp-label { font-size: 13px; font-weight: 700; margin: 0 0 10px; color: var(--text-2, #555); }
.entity-list, .feat-list { display: flex; flex-wrap: wrap; gap: 8px; }
.entity {
  background: var(--brand-bg, #eef4ff);
  color: var(--brand, #3b6fe0);
  border-radius: 8px;
  padding: 4px 10px;
  font-size: 13px;
}
.feat {
  background: var(--bg-page, #f5f7fa);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 4px 10px;
  font-size: 13px;
}
.flow-view { display: grid; grid-template-columns: 1fr 1fr; gap: 20px; }
.flow-label { font-size: 12px; color: var(--text-3); margin-bottom: 8px; }
.flow-node {
  display: flex; align-items: center; gap: 8px;
  padding: 6px 10px; border: 1px solid var(--border); border-radius: 8px;
  margin-bottom: 6px; font-size: 13px;
}
.fn-kind {
  font-size: 11px; padding: 1px 6px; border-radius: 6px;
  background: var(--bg-page, #f5f7fa); color: var(--text-3);
}
.k-gate { background: #fff3e0; color: #b26a00; }
.k-send { background: #e8f5e9; color: #2e7d32; }
.k-ai { background: #f3e5f5; color: #6a1b9a; }
.fn-name { font-weight: 600; }
.fn-tool { color: var(--text-3); font-size: 12px; }
.flow-edge {
  display: flex; align-items: center; gap: 6px;
  padding: 4px 10px; font-size: 13px; margin-bottom: 4px;
}
.fe-from, .fe-to { font-family: var(--font-mono, monospace); }
.fe-arrow { color: var(--text-3); font-size: 12px; }
.mono { font-family: var(--font-mono, monospace); }
.muted { color: var(--text-3); font-size: 12px; }
.tpl-overview {
  background: var(--brand-soft, #eef4ff);
  border: 1px dashed var(--brand, #3b6fe0);
  border-radius: 10px;
  padding: 12px;
  margin-bottom: 12px;
}
.tpl-ov-row { display: flex; gap: 24px; }
.tpl-ov-tags { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 8px; }
.tpl-hint { font-size: 12px; color: var(--text-3); margin-top: 8px; line-height: 1.6; }
.tpl-toolbar { display: flex; gap: 8px; margin-bottom: 12px; }
.tpl-domain { width: 130px; }
.tpl-list { display: flex; flex-direction: column; gap: 10px; }
.tpl-item {
  border: 1px solid var(--border); border-radius: 10px; padding: 10px;
}
.tpl-name { font-weight: 700; font-size: 14px; }
.tpl-meta { font-size: 12px; color: var(--text-3); margin: 2px 0 6px; }
.tpl-summary { font-size: 13px; color: var(--text-2, #555); margin-bottom: 6px; }
</style>
