<template>
  <div class="bv">
    <div class="head">
      <div>
        <h2 class="page-title">浏览器自动化</h2>
        <p class="page-subtitle">自然语言驱动的可视化浏览器操作 · 模板 / 会话 / 实时执行</p>
      </div>
      <el-button @click="loadAll"><el-icon><Refresh /></el-icon> 刷新</el-button>
    </div>

    <div class="grid grid-4 kpi-row">
      <div class="panel kpi">
        <div class="kpi-value">{{ sessions.length }}</div>
        <div class="kpi-label">活跃会话</div>
      </div>
      <div class="panel kpi">
        <div class="kpi-value">{{ templates.length }}</div>
        <div class="kpi-label">操作模板</div>
      </div>
      <div class="panel kpi">
        <div class="kpi-value">{{ totalSteps }}</div>
        <div class="kpi-label">累计步骤</div>
      </div>
      <div class="panel kpi">
        <div class="kpi-value success">{{ successRate }}%</div>
        <div class="kpi-label">成功率</div>
      </div>
    </div>

    <el-tabs v-model="tab">
      <el-tab-pane label="自然语言执行" name="natural">
        <div class="panel card-pad">
          <h3 class="section-title">自然语言指令</h3>
          <el-input
            v-model="task"
            type="textarea"
            :rows="3"
            placeholder="例如：打开百度，搜索璇玑系统，并截图"
          />
          <div class="examples">
            <el-tag v-for="e in examples" :key="e" class="ex" @click="task = e">{{ e }}</el-tag>
          </div>
          <el-button type="primary" :loading="running" @click="runNatural" style="margin-top: 12px">
            <el-icon><VideoPlay /></el-icon> 执行任务
          </el-button>
          <div v-if="naturalResult" class="out">
            <pre>{{ JSON.stringify(naturalResult, null, 2) }}</pre>
          </div>
        </div>
      </el-tab-pane>

      <el-tab-pane label="模板库" name="tpl">
        <div class="grid grid-3">
          <div class="panel tpl-card" v-for="t in templates" :key="t.id">
            <div class="tpl-name">{{ t.name || t.id }}</div>
            <div class="tpl-desc">{{ t.description }}</div>
            <el-button size="small" type="primary" plain @click="runTpl(t)">运行</el-button>
          </div>
        </div>
      </el-tab-pane>

      <el-tab-pane label="会话监控" name="sess">
        <div class="grid grid-2">
          <div class="panel card-pad" v-for="s in sessions" :key="s.id">
            <div class="sess-head">
              <span class="sess-id">{{ s.id }}</span>
              <span class="badge info">{{ s.status || 'active' }}</span>
              <el-button size="small" text type="danger" @click="closeSess(s)">
                <el-icon><Close /></el-icon>
              </el-button>
            </div>
            <div class="sess-meta">
              步骤 {{ s.steps_completed || 0 }} / {{ s.total_steps || 0 }} ·
              网址 {{ s.url || '—' }}
            </div>
            <el-progress
              :percentage="Math.round(((s.steps_completed || 0) / (s.total_steps || 1)) * 100)"
              :stroke-width="8"
            />
          </div>
          <el-empty v-if="!sessions.length" description="暂无会话" :image-size="60" />
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import {
  getBrowserTemplates,
  getBrowserSessions,
  closeBrowserSession,
  browserNatural,
  executeBrowserTask
} from '@/api'

const tab = ref('natural')
const templates = ref([])
const sessions = ref([])
const task = ref('')
const running = ref(false)
const naturalResult = ref(null)

const examples = [
  '打开百度并搜索璇玑系统',
  '访问 github.com 并登录',
  '打开电商网站加入购物车并结算',
  '截图当前页面并保存'
]

const totalSteps = computed(() =>
  sessions.value.reduce((a, s) => a + (s.total_steps || 0), 0)
)
const successRate = computed(() => {
  const done = sessions.value.length
  if (!done) return 100
  const ok = sessions.value.filter((s) => (s.steps_completed || 0) >= (s.total_steps || 1)).length
  return Math.round((ok / done) * 100)
})

async function loadAll() {
  try {
    const [t, s] = await Promise.all([
      getBrowserTemplates().catch(() => []),
      getBrowserSessions().catch(() => [])
    ])
    templates.value = t.templates || t.data || t || []
    sessions.value = s.sessions || s.data || s || []
  } catch (e) {
    ElMessage.error('加载失败：' + e.message)
  }
}

async function runNatural() {
  if (!task.value.trim()) {
    ElMessage.warning('请输入任务指令')
    return
  }
  running.value = true
  naturalResult.value = null
  try {
    naturalResult.value = await browserNatural({ prompt: task.value })
    ElMessage.success('任务已提交')
    await loadAll()
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    running.value = false
  }
}

async function runTpl(t) {
  try {
    await executeBrowserTask({ task_id: t.id, variables: {} })
    ElMessage.success('模板已运行')
    await loadAll()
  } catch (e) {
    ElMessage.error(e.message)
  }
}

async function closeSess(s) {
  try {
    await closeBrowserSession(s.id)
    ElMessage.success('会话已关闭')
    await loadAll()
  } catch (e) {
    ElMessage.error(e.message)
  }
}

onMounted(loadAll)
</script>

<style scoped>
.bv {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.kpi {
  padding: 16px 18px;
}
.kpi-value {
  font-size: 24px;
  font-weight: 700;
}
.kpi-value.success {
  color: var(--success);
}
.kpi-label {
  font-size: 13px;
  color: var(--text-3);
  margin-top: 2px;
}
.card-pad {
  padding: 18px 20px;
}
.examples {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 10px;
}
.ex {
  cursor: pointer;
}
.ex:hover {
  background: var(--brand-soft);
  color: var(--brand-dark);
}
.out {
  margin-top: 14px;
  background: #0b1020;
  color: #a5b4fc;
  padding: 12px;
  border-radius: 10px;
  font-size: 12px;
  overflow: auto;
  max-height: 240px;
}
.tpl-card {
  padding: 16px 18px;
}
.tpl-name {
  font-weight: 700;
  font-size: 15px;
}
.tpl-desc {
  font-size: 13px;
  color: var(--text-3);
  margin: 8px 0 12px;
  min-height: 36px;
}
.sess-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}
.sess-id {
  font-weight: 700;
  flex: 1;
  font-family: monospace;
}
.sess-meta {
  font-size: 12px;
  color: var(--text-3);
  margin-bottom: 8px;
}
</style>
