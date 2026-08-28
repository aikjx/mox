<template>
  <div class="page-container">
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">大模型网关配置</h2>
        <p class="page-subtitle">多 Provider 统一管理 · 智能路由 · 一键连通性测试</p>
      </div>
      <div class="page-header-actions">
        <el-button :loading="loading" @click="loadAll"><el-icon><Refresh /></el-icon> 刷新</el-button>
        <el-button type="primary" @click="openAddDialog"><el-icon><Plus /></el-icon> 添加渠道</el-button>
      </div>
    </div>

    <div class="page-content">

    <div class="grid grid-4 kpi-row">
      <div class="panel kpi" v-for="k in kpis" :key="k.label">
        <div class="kpi-value" :class="{ ok: k.ok, bad: k.bad }">
          <el-icon v-if="k.icon"><component :is="k.icon" /></el-icon> {{ k.value }}
        </div>
        <div class="kpi-label">{{ k.label }}</div>
        <div class="kpi-bar"><i :style="{ width: k.pct + '%', background: k.color }"></i></div>
      </div>
    </div>

    <div class="grid grid-2 main-row">
      <div class="panel card-pad">
        <div class="section-head">
          <h3 class="section-title">渠道列表</h3>
          <el-tag type="info" size="small">{{ providers.length }} 个 Provider</el-tag>
        </div>
        <div v-if="!providers.length" class="empty-state">
          <el-empty description="暂无渠道配置，点击右上角添加" :image-size="80" />
        </div>
        <div v-else class="provider-list">
          <div
            v-for="p in providers"
            :key="p.id"
            class="provider-card"
            :class="{ active: p.active, disabled: !p.enabled }"
          >
            <div class="provider-header">
              <div class="provider-icon" :style="{ background: getProviderColor(p.type) }">
                {{ getProviderInitials(p.name) }}
              </div>
              <div class="provider-info">
                <div class="provider-name">
                  {{ p.name }}
                  <el-tag v-if="p.active" type="success" size="small" effect="dark">当前使用</el-tag>
                  <el-tag v-if="!p.enabled" type="info" size="small">已禁用</el-tag>
                  <el-tag v-if="p.has_key && p.enabled" type="warning" size="small">✓ 已配置</el-tag>
                  <el-tag v-if="!p.has_key && p.enabled" type="danger" size="small">✗ 无 API Key</el-tag>
                </div>
                <div class="provider-meta">
                  <span class="model">{{ p.model }}</span>
                  <span class="sep">·</span>
                  <span class="base-url">{{ maskUrl(p.base_url) }}</span>
                </div>
              </div>
            </div>
            <div class="provider-actions">
              <el-switch
                v-model="p.enabled"
                @change="toggleProvider(p)"
                active-text="启用"
                inactive-text="禁用"
              />
              <el-button size="small" text @click="testConnection(p)" :loading="testingId === p.id">
                <el-icon><Connection /></el-icon> 测试
              </el-button>
              <el-button size="small" text type="primary" @click="editProvider(p)">
                <el-icon><Edit /></el-icon> 编辑
              </el-button>
              <el-button
                v-if="!p.active && p.enabled"
                size="small"
                text
                type="success"
                @click="setActive(p)"
              >
                <el-icon><Select /></el-icon> 设为默认
              </el-button>
              <el-popconfirm title="确定删除此渠道？" @confirm="removeProvider(p)">
                <template #reference>
                  <el-button size="small" text type="danger">
                    <el-icon><Delete /></el-icon> 删除
                  </el-button>
                </template>
              </el-popconfirm>
            </div>
          </div>
        </div>
      </div>

      <div class="panel card-pad">
        <div class="section-head">
          <h3 class="section-title">智能路由配置</h3>
          <el-tag type="info" size="small">路由策略</el-tag>
        </div>
        <div class="routing-config">
          <el-form :model="routingConfig" label-width="120px" label-position="right">
            <el-form-item label="路由策略">
              <el-select v-model="routingConfig.strategy" style="width: 100%">
                <el-option label="优先顺序（按列表顺序使用）" value="priority" />
                <el-option label="轮询（Round Robin）" value="round_robin" />
                <el-option label="权重（按权重分配流量）" value="weighted" />
                <el-option label="故障转移（主备切换）" value="failover" />
              </el-select>
            </el-form-item>
            <el-form-item label="启用故障转移">
              <el-switch v-model="routingConfig.fallback" />
            </el-form-item>
            <el-form-item label="启用负载均衡">
              <el-switch v-model="routingConfig.load_balance" />
            </el-form-item>
            <el-form-item label="可用渠道">
              <div class="routing-providers">
                <div
                  v-for="id in routingProviders"
                  :key="id"
                  class="routing-provider"
                  :class="{ active: isActive(id) }"
                  @click="toggleRoutingProvider(id)"
                >
                  {{ getProviderName(id) }}
                  <el-icon v-if="routingConfig.providers.includes(id)"><Check /></el-icon>
                </div>
              </div>
            </el-form-item>
            <el-form-item v-if="routingConfig.strategy === 'weighted'" label="权重配置">
              <div class="weight-config">
                <div v-for="id in routingConfig.providers" :key="id" class="weight-item">
                  <span>{{ getProviderName(id) }}</span>
                  <el-slider
                    v-model="routingConfig.weights[id]"
                    :min="1"
                    :max="100"
                    show-input
                    :show-input-controls="false"
                  />
                </div>
                <el-empty v-if="!routingConfig.providers.length" description="请先选择渠道" :image-size="60" />
              </div>
            </el-form-item>
          </el-form>
          <el-button type="primary" :loading="savingRouting" @click="saveRouting" style="width: 100%">
            <el-icon><Check /></el-icon> 保存路由配置
          </el-button>
        </div>
      </div>
    </div>

    <div class="panel card-pad web-search-panel">
      <div class="section-head">
        <h3 class="section-title">联网搜索配置</h3>
        <div class="section-head-right">
          <el-tag :type="webSearchReady ? 'success' : 'info'" size="small">
            {{ webSearchReady ? '● 可用' : '○ 未就绪' }}
          </el-tag>
          <el-switch v-model="webSearchConfig.enabled" active-text="启用联网" />
        </div>
      </div>
      <p class="ws-panel-desc">
        开启后，AI 对话页的「联网」开关即生效：回答前自动检索实时网页信息并注入上下文，
        解决模型知识截止导致的日期错误、时效信息缺失等问题。
      </p>
      <el-form :model="webSearchConfig" label-width="120px" label-position="right" class="ws-form">
        <div class="ws-form-row">
          <el-form-item label="搜索引擎">
            <el-select v-model="webSearchConfig.engine" style="width: 100%" placeholder="选择搜索引擎">
              <el-option
                v-for="e in webEngines"
                :key="e.id"
                :label="e.name + (e.needKey ? '（需 API Key）' : '（免费）')"
                :value="e.id"
              />
            </el-select>
            <div class="ws-engine-desc">{{ currentEngineDesc }}</div>
          </el-form-item>
          <el-form-item label="最大结果数">
            <el-input-number v-model="webSearchConfig.max_results" :min="1" :max="10" style="width: 100%" />
          </el-form-item>
        </div>
        <div class="ws-form-row">
          <el-form-item v-if="currentEngineNeedKey" label="API Key">
            <el-input
              v-model="webSearchConfig.api_key"
              type="password"
              show-password
              :placeholder="webSearchConfig.api_key_masked ? '已配置（' + webSearchConfig.api_key_masked + '），留空则不修改' : '请输入 API Key'"
            />
          </el-form-item>
          <el-form-item v-if="webSearchConfig.engine === 'searxng'" label="Base URL">
            <el-input v-model="webSearchConfig.base_url" placeholder="http://localhost:8888" />
          </el-form-item>
          <el-form-item label="超时(ms)">
            <el-input-number v-model="webSearchConfig.timeout_ms" :min="2000" :max="30000" :step="1000" style="width: 100%" />
          </el-form-item>
        </div>
      </el-form>
      <div class="ws-actions">
        <el-button :loading="savingWebSearch" type="primary" @click="saveWebSearchConfig">
          <el-icon><Check /></el-icon> 保存配置
        </el-button>
        <el-button :loading="testingWebSearch" @click="runWebSearchTest">
          <el-icon><Search /></el-icon> 搜索测试
        </el-button>
      </div>
      <el-alert
        v-if="webSearchTestResult"
        :title="webSearchTestResult.message"
        :type="webSearchTestResult.success ? 'success' : 'error'"
        :closable="true"
        style="margin-top: 12px"
      />
    </div>

    <div class="panel card-pad preset-panel">
      <div class="section-head">
        <h3 class="section-title">快速添加预设渠道</h3>
        <el-tag type="info" size="small">点击卡片自动填充配置</el-tag>
      </div>
      <div class="preset-grid">
        <div
          v-for="preset in presets"
          :key="preset.id"
          class="preset-card"
          @click="usePreset(preset)"
        >
          <div class="preset-icon" :style="{ background: getProviderColor(preset.id) }">
            {{ preset.name.charAt(0) }}
          </div>
          <div class="preset-info">
            <div class="preset-name">{{ preset.name }}</div>
            <div class="preset-desc">{{ preset.description }}</div>
            <div class="preset-models">
              <el-tag v-for="m in preset.models.slice(0, 3)" :key="m" size="small" type="info" effect="plain">
                {{ m }}
              </el-tag>
              <el-tag v-if="preset.models.length > 3" size="small" type="info" effect="plain">
                +{{ preset.models.length - 3 }}
              </el-tag>
            </div>
          </div>
          <el-icon class="preset-arrow"><ArrowRight /></el-icon>
        </div>
      </div>
    </div>

    <div class="grid grid-2 stats-panel">
      <div class="panel card-pad">
        <div class="section-head">
          <h3 class="section-title">用量统计</h3>
          <el-tag type="info" size="small">企业级监控</el-tag>
        </div>
        <div class="stats-summary">
          <div class="stat-item">
            <div class="stat-value">{{ stats.total_tokens.toLocaleString() }}</div>
            <div class="stat-label">Token 总量</div>
          </div>
          <div class="stat-item">
            <div class="stat-value">{{ stats.total_requests }}</div>
            <div class="stat-label">请求次数</div>
          </div>
          <div class="stat-item">
            <div class="stat-value">{{ stats.success_rate }}%</div>
            <div class="stat-label">成功率</div>
          </div>
          <div class="stat-item">
            <div class="stat-value">{{ stats.providers }}</div>
            <div class="stat-label">使用渠道数</div>
          </div>
        </div>
        <div class="usage-list" v-if="Object.keys(usage).length">
          <div class="usage-item" v-for="(data, id) in usage" :key="id">
            <div class="usage-header">
              <span class="usage-name">{{ getProviderName(id) }}</span>
              <span class="usage-tokens">{{ data.total_tokens.toLocaleString() }} tokens</span>
            </div>
            <el-progress :percentage="Math.min(100, Math.round(data.total_tokens / Math.max(stats.total_tokens, 1) * 100))" :stroke-width="8" :color="getProgressColor(data.total_tokens)"/>
          </div>
        </div>
        <div v-else class="empty-state">
          <el-empty description="暂无用量数据" :image-size="60" />
        </div>
      </div>

      <div class="panel card-pad">
        <div class="section-head">
          <h3 class="section-title">请求日志</h3>
          <el-tag type="info" size="small">最近 50 条</el-tag>
        </div>
        <div class="log-list" v-if="logs.length">
          <div class="log-item" v-for="(log, i) in logs" :key="i" :class="log.status">
            <div class="log-status">
              <el-icon v-if="log.status === 'success'"><CircleCheckFilled /></el-icon>
              <el-icon v-else><CircleCloseFilled /></el-icon>
            </div>
            <div class="log-info">
              <div class="log-header">
                <span class="log-provider">{{ getProviderName(log.provider) }}</span>
                <span class="log-latency">{{ log.latency_ms }}ms</span>
              </div>
              <div class="log-time">{{ formatTime(log.timestamp) }}</div>
              <div v-if="log.error" class="log-error">{{ log.error }}</div>
            </div>
          </div>
        </div>
        <div v-else class="empty-state">
          <el-empty description="暂无请求日志" :image-size="60" />
        </div>
      </div>
    </div>
    </div>

    <el-dialog v-model="dialogVisible" :title="editingId ? '编辑渠道' : '添加渠道'" width="640px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="100px" label-position="right">
        <el-form-item label="渠道类型" prop="provider">
          <el-select v-model="form.provider" @change="onProviderChange" style="width: 100%">
            <el-option
              v-for="p in presets"
              :key="p.id"
              :label="p.name"
              :value="p.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="渠道名称" prop="name">
          <el-input v-model="form.name" placeholder="自定义渠道名称" />
        </el-form-item>
        <el-form-item label="Base URL" prop="base_url">
          <el-input v-model="form.base_url" placeholder="https://api.example.com/v1" />
        </el-form-item>
        <el-form-item label="API Key" prop="api_key">
          <el-input
            v-model="form.api_key"
            type="password"
            show-password
            placeholder="sk-xxx"
          />
        </el-form-item>
        <el-form-item label="模型" prop="model">
          <div class="model-select">
            <el-select
              v-model="form.model"
              filterable
              allow-create
              default-first-option
              placeholder="选择或输入模型名"
              style="flex: 1"
            >
              <el-option
                v-for="m in availableModels"
                :key="m"
                :label="m"
                :value="m"
              />
            </el-select>
            <el-button
              type="primary"
              plain
              :loading="discovering"
              @click="discoverModels"
            >
              <el-icon><Search /></el-icon> 自动发现
            </el-button>
          </div>
        </el-form-item>
        <el-form-item label="温度">
          <el-slider v-model="form.temperature" :min="0" :max="2" :step="0.1" show-input />
        </el-form-item>
        <el-form-item label="最大 Token">
          <el-input-number v-model="form.max_tokens" :min="1" :max="128000" style="width: 100%" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="form.description" type="textarea" :rows="2" placeholder="渠道描述（可选）" />
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="form.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="saveProvider">
          <el-icon><Check /></el-icon> 保存
        </el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="testDialogVisible" title="连通性测试" width="480px">
      <div v-if="testResult" class="test-result">
        <div class="test-status" :class="testResult.success ? 'success' : 'fail'">
          <el-icon :size="48"><component :is="testResult.success ? 'CircleCheckFilled' : 'CircleCloseFilled'" /></el-icon>
          <div class="test-label">{{ testResult.success ? '连接成功' : '连接失败' }}</div>
        </div>
        <div class="test-details">
          <div class="detail-item">
            <span class="label">延迟</span>
            <span class="value">{{ testResult.latencyMs }} ms</span>
          </div>
          <div class="detail-item">
            <span class="label">消息</span>
            <span class="value">{{ testResult.message }}</span>
          </div>
          <div v-if="testResult.models && testResult.models.length" class="detail-item">
            <span class="label">检测到的模型</span>
            <div class="model-list">
              <el-tag v-for="m in testResult.models" :key="m" size="small" type="info">{{ m }}</el-tag>
            </div>
          </div>
        </div>
      </div>
      <div v-else class="test-loading">
        <el-icon :size="48" class="spin"><Loading /></el-icon>
        <p>正在测试连接...</p>
      </div>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Refresh, Plus, Edit, Delete, Check, ArrowRight,
  Connection, Search, Loading, Select, CircleCheckFilled, CircleCloseFilled
} from '@element-plus/icons-vue'
import * as api from '@/api'

const loading = ref(false)
const saving = ref(false)
const discovering = ref(false)
const savingRouting = ref(false)
const testingId = ref(null)

const providers = ref([])
const presets = ref([])
const health = ref({})
const stats = ref({ total_tokens: 0, total_requests: 0, success_rate: 0, providers: 0, recent: [] })
const usage = ref({})
const logs = ref([])

const dialogVisible = ref(false)
const testDialogVisible = ref(false)
const editingId = ref(null)
const testResult = ref(null)

const formRef = ref(null)
const form = reactive({
  provider: 'deepseek',
  name: '',
  base_url: '',
  model: '',
  api_key: '',
  description: '',
  temperature: 0.7,
  max_tokens: 2048,
  enabled: false
})

const formRules = {
  provider: [{ required: true, message: '请选择渠道类型', trigger: 'change' }],
  name: [{ required: true, message: '请输入渠道名称', trigger: 'blur' }],
  base_url: [{ required: true, message: '请输入 Base URL', trigger: 'blur' }],
  model: [{ required: true, message: '请选择或输入模型', trigger: 'change' }]
}

const routingConfig = reactive({
  strategy: 'priority',
  providers: [],
  fallback: true,
  load_balance: false,
  weights: {}
})

const routingProviders = computed(() => providers.value.map(p => p.id))
const routingPresets = computed(() => presets.value.map(p => p.id))

// ===== 联网搜索配置 =====
const webSearchConfig = reactive({
  enabled: false,
  engine: 'duckduckgo',
  api_key: '',
  api_key_masked: '',
  base_url: '',
  max_results: 5,
  timeout_ms: 10000
})
const webEngines = ref([])
const webSearchReady = ref(false)
const savingWebSearch = ref(false)
const testingWebSearch = ref(false)
const webSearchTestResult = ref(null)

const currentEngine = computed(() => webEngines.value.find(e => e.id === webSearchConfig.engine))
const currentEngineDesc = computed(() => currentEngine.value?.description || '')
const currentEngineNeedKey = computed(() => !!currentEngine.value?.needKey)

async function loadWebSearchConfig() {
  try {
    const res = await api.getWebSearchConfig()
    if (res) {
      Object.assign(webSearchConfig, {
        enabled: !!res.config?.enabled,
        engine: res.config?.engine || 'duckduckgo',
        api_key: '',
        api_key_masked: res.config?.api_key_masked || '',
        base_url: res.config?.base_url || '',
        max_results: res.config?.max_results || 5,
        timeout_ms: res.config?.timeout_ms || 10000
      })
      webEngines.value = res.engines || []
      webSearchReady.value = !!res.ready
    }
  } catch (e) { /* 静默降级：后端未就绪时不阻塞页面 */ }
}

async function saveWebSearchConfig() {
  savingWebSearch.value = true
  try {
    const payload = {
      enabled: webSearchConfig.enabled,
      engine: webSearchConfig.engine,
      base_url: webSearchConfig.base_url,
      max_results: webSearchConfig.max_results,
      timeout_ms: webSearchConfig.timeout_ms
    }
    if (webSearchConfig.api_key) payload.api_key = webSearchConfig.api_key
    const res = await api.updateWebSearchConfig(payload)
    if (res?.config) {
      webSearchConfig.api_key = ''
      webSearchConfig.api_key_masked = res.config.api_key_masked || ''
      webSearchReady.value = !!res.ready
    }
    ElMessage.success('联网搜索配置已保存')
  } catch (e) {
    ElMessage.error('保存失败: ' + e.message)
  } finally {
    savingWebSearch.value = false
  }
}

async function runWebSearchTest() {
  testingWebSearch.value = true
  webSearchTestResult.value = null
  try {
    const res = await api.testWebSearch()
    webSearchTestResult.value = res || { success: false, message: '无响应' }
    if (res?.success) webSearchReady.value = true
  } catch (e) {
    webSearchTestResult.value = { success: false, message: e.message }
  } finally {
    testingWebSearch.value = false
  }
}

const availableModels = computed(() => {
  const preset = presets.value.find(p => p.id === form.provider)
  return preset ? preset.models : []
})

const kpis = computed(() => {
  const total = providers.value.length
  const enabled = providers.value.filter(p => p.enabled).length
  const withKey = providers.value.filter(p => p.enabled && p.has_key).length
  const active = providers.value.find(p => p.active)
  
  return [
    { label: '渠道总数', value: total, icon: 'Connection', color: '#6366f1', ok: total > 0 },
    { label: '已启用', value: enabled, icon: 'CircleCheck', color: '#10b981', ok: enabled > 0 },
    { label: '已配置 Key', value: withKey, icon: 'Key', color: '#f59e0b', ok: withKey > 0 },
    { label: '当前使用', value: active?.name || '无', icon: 'Select', color: active ? '#06b6d4' : '#94a3b8', ok: !!active }
  ]
})

async function loadAll() {
  loading.value = true
  try {
    const [providersRes, presetsRes, healthRes, statsRes, usageRes, logsRes] = await Promise.all([
      api.getLlmProviders(),
      api.getLlmPresets(),
      api.getLlmHealth(),
      api.getLlmStats().catch(() => ({})),
      api.getLlmUsage().catch(() => ({})),
      api.getLlmLogs(50).catch(() => [])
    ])
    providers.value = providersRes || []
    presets.value = (presetsRes || []).map((p) => ({ ...p, models: Array.isArray(p.models) ? p.models : [] }))
    health.value = healthRes || {}
    stats.value = { total_tokens: 0, total_requests: 0, success_rate: 0, providers: 0, recent: [], ...(statsRes || {}) }
    usage.value = usageRes || {}
    logs.value = logsRes || []
  } catch (e) {
    ElMessage.error('加载失败: ' + e.message)
  } finally {
    loading.value = false
  }
}

function getProviderColor(type) {
  const colors = {
    deepseek: 'linear-gradient(135deg,#4f46e5,#7c3aed)',
    volcengine: 'linear-gradient(135deg,#f97316,#ea580c)',
    qwen: 'linear-gradient(135deg,#06b6d4,#0891b2)',
    zhipu: 'linear-gradient(135deg,#8b5cf6,#7c3aed)',
    openai: 'linear-gradient(135deg,#10b981,#059669)',
    anthropic: 'linear-gradient(135deg,#ef4444,#dc2626)',
    google: 'linear-gradient(135deg,#4285f4,#3b82f6)',
    ollama: 'linear-gradient(135deg,#64748b,#475569)',
    local: 'linear-gradient(135deg,#22c55e,#16a34a)',
    custom: 'linear-gradient(135deg,#94a3b8,#64748b)'
  }
  return colors[type] || colors.custom
}

function getProviderInitials(name) {
  if (!name) return '?'
  return name.slice(0, 2).toUpperCase()
}

function maskUrl(url) {
  if (!url) return ''
  try {
    return url.replace(/\/+$/, '')
  } catch {
    return url
  }
}

function getProviderName(id) {
  const p = providers.value.find(x => x.id === id)
  return p?.name || id
}

function getProgressColor(tokens) {
  if (tokens > 1000000) return '#ef4444'
  if (tokens > 100000) return '#f59e0b'
  if (tokens > 10000) return '#06b6d4'
  return '#10b981'
}

function formatTime(ts) {
  if (!ts) return ''
  try {
    const d = new Date(ts)
    return d.toLocaleString('zh-CN', { 
      month: '2-digit', 
      day: '2-digit', 
      hour: '2-digit', 
      minute: '2-digit', 
      second: '2-digit' 
    })
  } catch {
    return ts
  }
}

function isActive(id) {
  return id === routingConfig.providers[0]
}

function toggleProvider(p) {
  if (p.enabled) {
    api.enableLlmProvider(p.id).then(() => {
      ElMessage.success(`已启用 ${p.name}`)
    }).catch(e => ElMessage.error(e.message))
  } else {
    api.disableLlmProvider(p.id).then(() => {
      ElMessage.success(`已禁用 ${p.name}`)
    }).catch(e => ElMessage.error(e.message))
  }
}

async function testConnection(p) {
  testingId.value = p.id
  testResult.value = null
  testDialogVisible.value = true
  try {
    const result = await api.testLlmProvider(p.id)
    testResult.value = result
  } catch (e) {
    testResult.value = { success: false, message: e.message, latencyMs: 0 }
  } finally {
    testingId.value = null
  }
}

function setActive(p) {
  ElMessageBox.confirm(`将「${p.name}」设为默认渠道？`, '确认', {
    type: 'warning'
  }).then(async () => {
    try {
      await api.setActiveProvider(p.id)
      ElMessage.success(`已切换到 ${p.name}`)
      await loadAll()
    } catch (e) {
      ElMessage.error(e.message)
    }
  }).catch(() => {})
}

function removeProvider(p) {
  api.removeLlmProvider(p.id).then(() => {
    ElMessage.success('已删除')
    loadAll()
  }).catch(e => ElMessage.error(e.message))
}

function openAddDialog() {
  editingId.value = null
  Object.assign(form, {
    provider: 'deepseek',
    name: '',
    base_url: '',
    model: '',
    api_key: '',
    description: '',
    temperature: 0.7,
    max_tokens: 2048,
    enabled: false
  })
  onProviderChange('deepseek')
  dialogVisible.value = true
}

function editProvider(p) {
  editingId.value = p.id
  Object.assign(form, {
    provider: p.type,
    name: p.name,
    base_url: p.base_url,
    model: p.model,
    api_key: p.api_key || '',
    description: p.description || '',
    temperature: p.temperature || 0.7,
    max_tokens: p.max_tokens || 2048,
    enabled: p.enabled
  })
  dialogVisible.value = true
}

function usePreset(preset) {
  editingId.value = null
  Object.assign(form, {
    provider: preset.id,
    name: preset.name,
    base_url: preset.base_url,
    model: preset.models[0] || '',
    api_key: '',
    description: preset.description,
    temperature: 0.7,
    max_tokens: 2048,
    enabled: false
  })
  dialogVisible.value = true
}

function onProviderChange(providerId) {
  const preset = presets.value.find(p => p.id === providerId)
  if (preset) {
    form.base_url = preset.base_url
    form.model = preset.models[0] || ''
    if (!form.name || form.name === form.provider) {
      form.name = preset.name
    }
  }
}

async function discoverModels() {
  discovering.value = true
  try {
    const result = await api.discoverLlmModels(editingId.value || form.provider)
    if (result.success && result.models?.length) {
      availableModels.value.length = 0
      result.models.forEach(m => {
        const modelId = typeof m === 'string' ? m : m.id
        if (!form.model) form.model = modelId
        availableModels.value.push(modelId)
      })
      ElMessage.success(`发现 ${result.models.length} 个模型`)
    } else {
      ElMessage.warning(result.message || '未发现可用模型')
    }
  } catch (e) {
    ElMessage.error('发现模型失败: ' + e.message)
  } finally {
    discovering.value = false
  }
}

async function saveProvider() {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
  } catch {
    return
  }
  
  saving.value = true
  try {
    if (editingId.value) {
      await api.updateLlmProvider(editingId.value, form)
      ElMessage.success('更新成功')
    } else {
      await api.addLlmProvider(form)
      ElMessage.success('添加成功')
    }
    dialogVisible.value = false
    await loadAll()
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    saving.value = false
  }
}

function toggleRoutingProvider(id) {
  const idx = routingConfig.providers.indexOf(id)
  if (idx >= 0) {
    routingConfig.providers.splice(idx, 1)
    delete routingConfig.weights[id]
  } else {
    routingConfig.providers.push(id)
    if (!routingConfig.weights[id]) {
      routingConfig.weights[id] = 50
    }
  }
}

async function saveRouting() {
  savingRouting.value = true
  try {
    await api.updateLlmRouting(routingConfig)
    ElMessage.success('路由配置已保存')
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    savingRouting.value = false
  }
}

onMounted(() => {
  loadAll()
  loadWebSearchConfig()
})
</script>

<style scoped>
.llm-config {
  padding: 0;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 20px;
}

.page-title {
  font-size: 20px;
  font-weight: 700;
  color: #1e293b;
  margin: 0 0 4px 0;
}

.page-subtitle {
  font-size: 13px;
  color: #64748b;
  margin: 0;
}

.head-actions {
  display: flex;
  gap: 10px;
}

.kpi-row {
  margin-bottom: 20px;
}

.main-row {
  margin-bottom: 20px;
}

.panel {
  background: #fff;
  border-radius: 12px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
}

.section-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

/* ===== 联网搜索配置 ===== */
.web-search-panel { margin-top: 18px; }
.section-head-right { display: flex; align-items: center; gap: 14px; }
.ws-panel-desc {
  font-size: 13px; color: #64748b; line-height: 1.7;
  margin: -4px 0 16px; padding: 10px 14px;
  background: #f0f9ff; border: 1px solid #bae6fd; border-radius: 8px;
}
.ws-form :deep(.el-form-item) { margin-bottom: 14px; }
.ws-form-row { display: grid; grid-template-columns: 1fr 1fr; gap: 0 24px; }
.ws-engine-desc { font-size: 12px; color: #94a3b8; line-height: 1.6; margin-top: 2px; }
.ws-actions { display: flex; gap: 10px; margin-top: 4px; }
@media (max-width: 900px) { .ws-form-row { grid-template-columns: 1fr; } }

.section-title {
  font-size: 16px;
  font-weight: 600;
  color: #1e293b;
  margin: 0;
}

.provider-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.provider-card {
  padding: 16px;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  transition: all 0.2s;
  background: #fff;
}

.provider-card:hover {
  border-color: #6366f1;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.1);
}

.provider-card.active {
  border-color: #22c55e;
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.05), transparent);
}

.provider-card.disabled {
  opacity: 0.65;
}

.provider-header {
  display: flex;
  gap: 12px;
  margin-bottom: 12px;
}

.provider-icon {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-weight: 700;
  font-size: 14px;
  flex-shrink: 0;
}

.provider-info {
  flex: 1;
  min-width: 0;
}

.provider-name {
  font-size: 15px;
  font-weight: 600;
  color: #1e293b;
  margin-bottom: 4px;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.provider-meta {
  font-size: 12px;
  color: #64748b;
  display: flex;
  align-items: center;
  gap: 6px;
}

.provider-meta .sep {
  color: #cbd5e1;
}

.provider-meta .model {
  color: #6366f1;
  font-weight: 500;
}

.provider-meta .base-url {
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.provider-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  padding-top: 12px;
  border-top: 1px solid #f1f5f9;
}

.routing-config {
  padding: 8px 0;
}

.routing-providers {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.routing-provider {
  padding: 6px 12px;
  border: 1px solid #e2e8f0;
  border-radius: 20px;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  gap: 4px;
  color: #64748b;
}

.routing-provider.active {
  border-color: #6366f1;
  background: #eef2ff;
  color: #4f46e5;
}

.weight-config {
  width: 100%;
}

.weight-item {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.weight-item span {
  width: 100px;
  font-size: 13px;
  color: #64748b;
}

.weight-item :deep(.el-slider) {
  flex: 1;
}

.preset-panel {
  margin-top: 20px;
}

.preset-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 12px;
}

.preset-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s;
  background: #fff;
}

.preset-card:hover {
  border-color: #6366f1;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.15);
  transform: translateY(-2px);
}

.preset-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-weight: 700;
  font-size: 16px;
  flex-shrink: 0;
}

.preset-info {
  flex: 1;
  min-width: 0;
}

.preset-name {
  font-size: 14px;
  font-weight: 600;
  color: #1e293b;
  margin-bottom: 2px;
}

.preset-desc {
  font-size: 12px;
  color: #64748b;
  margin-bottom: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preset-models {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.preset-arrow {
  color: #cbd5e1;
  font-size: 16px;
}

.model-select {
  display: flex;
  gap: 8px;
  width: 100%;
}

.model-select .el-select {
  flex: 1;
}

.empty-state {
  padding: 40px 0;
}

.test-result {
  padding: 20px 0;
}

.test-status {
  text-align: center;
  margin-bottom: 20px;
}

.test-status.success {
  color: #22c55e;
}

.test-status.fail {
  color: #ef4444;
}

.test-label {
  font-size: 18px;
  font-weight: 600;
  margin-top: 8px;
}

.test-details {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.detail-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.detail-item .label {
  font-size: 12px;
  color: #94a3b8;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.detail-item .value {
  font-size: 14px;
  color: #1e293b;
}

.model-list {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.test-loading {
  text-align: center;
  padding: 40px 0;
  color: #64748b;
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

:deep(.el-form-item) {
  margin-bottom: 16px;
}

:deep(.el-dialog__body) {
  padding-top: 10px;
}

.stats-panel {
  margin-top: 20px;
}

.stats-summary {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
  margin-bottom: 20px;
}

.stat-item {
  text-align: center;
  padding: 12px 8px;
  background: #f8fafc;
  border-radius: 10px;
}

.stat-value {
  font-size: 24px;
  font-weight: 700;
  color: #1e293b;
  line-height: 1.2;
}

.stat-label {
  font-size: 12px;
  color: #64748b;
  margin-top: 4px;
}

.usage-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.usage-item {
  padding: 12px;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
}

.usage-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 8px;
}

.usage-name {
  font-size: 13px;
  font-weight: 500;
  color: #1e293b;
}

.usage-tokens {
  font-size: 12px;
  color: #6366f1;
  font-weight: 600;
}

.log-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 400px;
  overflow-y: auto;
}

.log-item {
  display: flex;
  gap: 10px;
  padding: 10px;
  border-radius: 8px;
  background: #f8fafc;
  transition: all 0.2s;
}

.log-item.success {
  border-left: 3px solid #22c55e;
}

.log-item.failed {
  border-left: 3px solid #ef4444;
}

.log-status {
  flex-shrink: 0;
  display: flex;
  align-items: flex-start;
  padding-top: 2px;
}

.log-item.success .log-status {
  color: #22c55e;
}

.log-item.failed .log-status {
  color: #ef4444;
}

.log-info {
  flex: 1;
  min-width: 0;
}

.log-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}

.log-provider {
  font-size: 13px;
  font-weight: 500;
  color: #1e293b;
}

.log-latency {
  font-size: 12px;
  color: #64748b;
  font-family: monospace;
}

.log-time {
  font-size: 11px;
  color: #94a3b8;
}

.log-error {
  font-size: 12px;
  color: #ef4444;
  margin-top: 4px;
}
</style>