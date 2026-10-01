<template>
  <div class="page-container">
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">算法分析实验室 · AlgoLab</h2>
        <p class="page-subtitle">粘贴代码 → 自动识别算法类型并生成结构化流程图；另附算法类型目录与空间光速螺旋模型分析</p>
      </div>
      <div class="page-header-actions">
        <el-button type="primary" @click="goAIOptimize">
          <el-icon><Promotion /></el-icon> AI优化算法
        </el-button>
      </div>
    </div>

    <div class="page-content">

    <el-tabs v-model="tab" class="algo-tabs">
      <!-- 算法分析 -->
      <el-tab-pane label="算法分析" name="analyze">
        <div class="panel card-pad">
          <div class="toolbar">
            <el-input
              v-model="code"
              type="textarea"
              :rows="10"
              placeholder="粘贴一段代码（支持 Python / JS / Rust / C++ / Java 启发式识别）……"
              class="code-area"
            />
            <div class="toolbar-actions">
              <el-select v-model="algoType" placeholder="算法类型（可选）" class="type-select" clearable>
                <el-option v-for="t in typeOptions" :key="t.id" :label="t.name" :value="t.id" />
              </el-select>
              <el-button type="primary" :loading="analyzing" @click="doAnalyze">
                <el-icon v-if="!analyzing"><Cpu /></el-icon>
                <span>{{ analyzing ? '分析中…' : '开始分析' }}</span>
              </el-button>
            </div>
          </div>

          <!-- 分析结果 -->
          <div v-if="analysis" class="result">
            <div class="result-head">
              <h4 class="result-title">分析结果</h4>
              <el-tag v-if="analysis.algorithm" size="small" type="warning">{{ analysis.algorithm }}</el-tag>
              <el-tag v-if="analysis.complexity" size="small">{{ analysis.complexity }}</el-tag>
            </div>
            <div v-if="analysis.summary" class="summary">{{ analysis.summary }}</div>
            <div class="result-grid">
              <div v-if="analysis.nodes?.length" class="panel sub">
                <div class="sub-label">节点</div>
                <div v-for="n in analysis.nodes" :key="n.id" class="anode">
                  <span class="an-kind" :class="'k-' + (n.kind || '')">{{ n.kind || 'task' }}</span>
                  <span class="an-name">{{ n.name }}</span>
                </div>
              </div>
              <div v-if="analysis.edges?.length" class="panel sub">
                <div class="sub-label">依赖</div>
                <div v-for="(e, i) in analysis.edges" :key="i" class="aedge">
                  {{ e.from }} <el-icon class="a-arrow"><Right /></el-icon> {{ e.to }}
                </div>
              </div>
              <div v-if="analysis.steps?.length" class="panel sub">
                <div class="sub-label">步骤</div>
                <ol class="asteps">
                  <li v-for="(s, i) in analysis.steps" :key="i">{{ s }}</li>
                </ol>
              </div>
            </div>
            <el-empty v-if="!analysis.nodes?.length && !analysis.steps?.length" description="未识别出结构化流程" :image-size="60" />
          </div>
        </div>
      </el-tab-pane>

      <!-- 类型目录 -->
      <el-tab-pane label="算法类型目录" name="types">
        <div class="panel card-pad">
          <el-button size="small" text :loading="loadingTypes" @click="loadTypes">刷新</el-button>
          <div class="type-grid">
            <div v-for="t in typeCatalog" :key="t.id" class="type-card">
              <div class="type-name">{{ t.name }}</div>
              <div class="type-id mono">{{ t.id }}</div>
              <div class="type-algos">
                <el-tag v-for="a in t.algorithms" :key="a" size="small" class="algo-tag">{{ a }}</el-tag>
              </div>
              <el-button size="small" text type="primary" @click="useType(t)">以该类型分析</el-button>
            </div>
            <el-empty v-if="!typeCatalog.length" description="暂无类型" :image-size="60" />
          </div>
        </div>
      </el-tab-pane>

      <!-- 螺旋模型 -->
      <el-tab-pane label="空间光速螺旋模型" name="spiral">
        <div class="panel card-pad">
          <div class="spiral-inputs">
            <el-input-number v-model="spiral.curvature" :precision="6" :step="0.1" label="曲率 κ" />
            <el-input-number v-model="spiral.torsion" :precision="6" :step="0.1" label="挠率 τ" />
            <el-input-number v-model="spiral.step_h" :precision="4" :step="0.5" label="一周步长 h" />
            <el-input-number v-model="spiral.radius" :precision="2" :step="0.1" label="半径（可选）" />
            <el-input-number v-model="spiral.speed" :precision="3" :step="1e6" label="速率（默认 c）" />
            <el-button type="primary" :loading="spiraling" @click="doSpiral">开始分析</el-button>
          </div>

          <div v-if="spiralReport" class="result">
            <div class="verdict">{{ spiralReport.verdict }}</div>
            <div class="result-grid">
              <div v-if="spiralReport.kinematics" class="panel sub">
                <div class="sub-label">运动学</div>
                <pre class="kine">{{ JSON.stringify(spiralReport.kinematics, null, 2) }}</pre>
              </div>
              <div v-if="spiralReport.dimension_checks?.length" class="panel sub">
                <div class="sub-label">量纲检查</div>
                <div v-for="(c, i) in spiralReport.dimension_checks" :key="i" class="check" :class="c.ok === false ? 'bad' : 'good'">
                  {{ c.message }}
                </div>
              </div>
              <div v-if="spiralReport.reliable_parts?.length" class="panel sub">
                <div class="sub-label">可靠部分</div>
                <ul class="plain-list">
                  <li v-for="(p, i) in spiralReport.reliable_parts" :key="i">{{ p }}</li>
                </ul>
              </div>
              <div v-if="spiralReport.extra_assumptions?.length" class="panel sub">
                <div class="sub-label">额外公设（不可靠）</div>
                <ul class="plain-list">
                  <li v-for="(p, i) in spiralReport.extra_assumptions" :key="i">{{ p }}</li>
                </ul>
              </div>
            </div>
          </div>
        </div>
      </el-tab-pane>
    </el-tabs>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus/es/components/message/index'
import { Cpu, Right, Promotion } from '@element-plus/icons-vue'
import { useProject } from '@/composables'
import { analyzeAlgorithm, getAlgorithmTypes, analyzeSpiral } from '@/api'

const router = useRouter()

// AI优化算法：跳转到AI助手，带上算法上下文
function goAIOptimize() {
  router.push({ path: '/ai', query: { source: 'algolab', action: 'optimize' } })
}

const tab = ref('analyze')
const code = ref('')
const algoType = ref('')
const analyzing = ref(false)
const analysis = ref(null)

const typeCatalog = ref([])
const loadingTypes = ref(false)
const typeOptions = computed(() =>
  typeCatalog.value.map((t) => ({ id: t.id, name: t.name }))
)

async function doAnalyze() {
  if (!code.value.trim()) {
    ElMessage.warning('请先粘贴代码')
    return
  }
  analyzing.value = true
  try {
    const d = await analyzeAlgorithm({
      code: code.value,
      algorithm_type: algoType.value || undefined,
    })
    if (d.success === false) throw new Error(d.error || '分析失败')
    analysis.value = d
  } catch (e) {
    ElMessage.error('分析失败：' + e.message)
  } finally {
    analyzing.value = false
  }
}

async function loadTypes() {
  loadingTypes.value = true
  try {
    const d = await getAlgorithmTypes()
    typeCatalog.value = d.types || []
  } catch (e) {
    ElMessage.error('类型目录加载失败：' + e.message)
  } finally {
    loadingTypes.value = false
  }
}
function useType(t) {
  algoType.value = t.id
  tab.value = 'analyze'
  ElMessage.success(`已选择「${t.name}」，粘贴代码后点击开始分析`)
}

// 螺旋模型
const spiraling = ref(false)
const spiralReport = ref(null)
const spiral = ref({
  curvature: 1.0,
  torsion: 1.0,
  step_h: 1.0,
  radius: null,
  speed: null,
})
async function doSpiral() {
  spiraling.value = true
  try {
    const d = await analyzeSpiral({
      curvature: spiral.value.curvature,
      torsion: spiral.value.torsion,
      step_h: spiral.value.step_h,
      radius: spiral.value.radius ?? undefined,
      speed: spiral.value.speed ?? undefined,
    })
    spiralReport.value = d
  } catch (e) {
    ElMessage.error('螺旋分析失败：' + e.message)
  } finally {
    spiraling.value = false
  }
}

onMounted(loadTypes)

// ===== 璇玑：以项目为核心的联动 =====
{
  const { onChange: _onProjectChange, ensureProjectContext: _ensureProject } = useProject()
  let _offPj = null
  let _loaded = false
  onMounted(async () => {
    _offPj = _onProjectChange(async () => { loadTypes() })
    await _ensureProject().catch(() => {})
    if (!_loaded) {
      _loaded = true
      loadTypes()
    }
  })
  const _ob$ = onBeforeUnmount == null ? null : onBeforeUnmount(() => { _offPj && _offPj() })
  // 若脚本未引入 onBeforeUnmount，退化为 window beforeunload 兜底（页面关闭）
  if (typeof onBeforeUnmount === 'undefined') {
    // 不操作：Vue 路由离开时组件 destroy，本作用域已销毁
  }
}
</script>

<style scoped>
/* 节点种类章与结论框原先写死一套「淡染底 + 深字」的浅色皮专用色表（三处章各一种 hue），
   在深色三皮下是卡片上三块白斑。迁到 --x-dim + --x（外壳淡染章的既有写法，见 AllianceTaskView 的优先级章）。
   AI 章的字取 --brand-accent 而非 --accent：后者在默认皮/浅皮下压淡染底只有 3.49 / 2.28:1，跌破 AA。
   旧值不在此复述 —— 裸 hex 台账把注释里的字面量一并计入。 */
.algo-page { display: flex; flex-direction: column; gap: 14px; }
.page-head { margin-bottom: 4px; }
.page-title { font-size: 20px; font-weight: 800; margin: 0; }
.page-sub { color: var(--text-3); font-size: 13px; margin: 4px 0 0; }
.panel { background: var(--bg-card); border: 1px solid var(--border); border-radius: 12px; }
.card-pad { padding: 16px; }
.toolbar-actions { display: flex; gap: 10px; margin-top: 12px; align-items: center; }
.type-select { width: 200px; }
.result { margin-top: 16px; }
.result-head { display: flex; align-items: center; gap: 10px; margin-bottom: 10px; }
.result-title { font-size: 15px; font-weight: 700; margin: 0; }
.summary { color: var(--text-2); font-size: 13px; margin-bottom: 12px; }
.result-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.panel.sub { padding: 12px; }
.sub-label { font-size: 12px; color: var(--text-3); margin-bottom: 8px; }
.anode {
  display: flex; align-items: center; gap: 8px;
  padding: 5px 8px; border: 1px solid var(--border); border-radius: 6px;
  margin-bottom: 5px; font-size: 13px;
}
.an-kind {
  font-size: 11px; padding: 1px 6px; border-radius: 5px;
  background: var(--bg-page); color: var(--text-3);
}
.k-gate { background: var(--warning-dim); color: var(--warning); }
.k-send { background: var(--success-dim); color: var(--success); }
.k-ai { background: var(--accent-dim); color: var(--brand-accent); }
.an-name { font-weight: 600; }
.aedge { font-family: var(--font-mono, monospace); font-size: 13px; margin-bottom: 4px; display: flex; align-items: center; gap: 4px; }
.a-arrow { color: var(--text-3); font-size: 12px; }
.asteps { margin: 0; padding-left: 18px; font-size: 13px; line-height: 1.8; }
.type-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 12px; margin-top: 12px; }
.type-card { border: 1px solid var(--border); border-radius: 10px; padding: 12px; }
.type-name { font-weight: 700; font-size: 14px; }
.type-id { font-size: 11px; color: var(--text-3); margin-bottom: 8px; }
.type-algos { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 8px; }
.algo-tag { font-size: 12px; }
.spiral-inputs { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; }
.verdict {
  background: var(--warning-dim); border: 1px solid var(--warning); color: var(--warning);
  border-radius: 8px; padding: 10px 12px; font-size: 13px; margin-bottom: 12px;
}
.kine { font-size: 12px; margin: 0; white-space: pre-wrap; }
.check { font-size: 13px; padding: 4px 0; }
.check.good { color: var(--success); }
.check.bad { color: var(--danger); }
.plain-list { margin: 0; padding-left: 18px; font-size: 13px; line-height: 1.8; }
.mono { font-family: var(--font-mono, monospace); }
</style>
