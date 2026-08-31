<template>
  <div class="page-container">
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">浏览器自动化</h2>
        <p class="page-subtitle">自然语言驱动的可视化浏览器操作 · 模板 / 会话 / 实时执行</p>
      </div>
      <div class="page-header-actions">
        <el-button @click="showSecuritySettings = true">
          <el-icon><Setting /></el-icon> 安全设置
        </el-button>
        <el-button type="primary" @click="goAIDrive">
          <el-icon><Promotion /></el-icon> AI驱动浏览器
        </el-button>
        <el-button @click="loadAll"><el-icon><Refresh /></el-icon> 刷新</el-button>
      </div>
    </div>

    <div class="page-content">

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

    <!-- 安全等级提示条 -->
    <div class="security-banner" :class="securityLevel">
      <el-icon><Warning /></el-icon>
      <span>当前安全等级：<b>{{ securityLevelLabel }}</b></span>
      <span class="banner-desc">{{ securityLevelDesc }}</span>
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

      <el-tab-pane label="操作审计" name="audit">
        <div class="panel card-pad">
          <div class="audit-head">
            <h3 class="section-title">操作审计日志</h3>
            <div class="audit-actions">
              <el-select v-model="auditFilter" size="small" style="width: 140px">
                <el-option label="全部操作" value="all" />
                <el-option label="自然语言执行" value="natural" />
                <el-option label="模板运行" value="template" />
                <el-option label="会话关闭" value="close" />
              </el-select>
              <el-button size="small" @click="clearAuditLog">清空日志</el-button>
            </div>
          </div>
          <el-table :data="filteredAuditLog" stripe style="width: 100%" max-height="500">
            <el-table-column prop="time" label="时间" width="180">
              <template #default="{ row }">{{ formatTime(row.time) }}</template>
            </el-table-column>
            <el-table-column prop="type" label="操作类型" width="120">
              <template #default="{ row }">
                <el-tag :type="auditTypeTag(row.type)" size="small">{{ auditTypeLabel(row.type) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="targetUrl" label="目标URL" min-width="200">
              <template #default="{ row }">{{ row.targetUrl || '—' }}</template>
            </el-table-column>
            <el-table-column prop="content" label="操作内容" min-width="240">
              <template #default="{ row }">
                <span class="audit-content" :title="row.content">{{ row.content }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="result" label="结果" width="100">
              <template #default="{ row }">
                <el-tag :type="row.result === 'success' ? 'success' : 'danger'" size="small">
                  {{ row.result === 'success' ? '成功' : '失败' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="detail" label="详情" width="200">
              <template #default="{ row }">{{ row.detail || '—' }}</template>
            </el-table-column>
          </el-table>
          <el-empty v-if="!filteredAuditLog.length" description="暂无审计记录" :image-size="60" />
        </div>
      </el-tab-pane>
    </el-tabs>
    </div>

    <!-- 安全设置弹窗 -->
    <el-dialog v-model="showSecuritySettings" title="浏览器自动化安全设置" width="560px">
      <el-form label-width="110px">
        <!-- 安全等级 -->
        <el-form-item label="安全等级">
          <el-radio-group v-model="securityLevel">
            <el-radio value="low">低</el-radio>
            <el-radio value="medium">中</el-radio>
            <el-radio value="high">高</el-radio>
          </el-radio-group>
          <div class="form-tip">{{ securityLevelDesc }}</div>
        </el-form-item>

        <!-- 域名白名单 -->
        <el-form-item label="域名白名单">
          <div class="whitelist-panel">
            <div class="whitelist-input">
              <el-input
                v-model="newDomain"
                placeholder="输入域名，如 baidu.com 或 github.com"
                size="small"
                style="flex: 1"
                @keyup.enter="addDomain"
              />
              <el-button size="small" type="primary" @click="addDomain" style="margin-left: 8px">添加</el-button>
            </div>
            <div class="whitelist-tags">
              <el-tag
                v-for="d in domainWhitelist"
                :key="d"
                closable
                size="small"
                style="margin-right: 6px; margin-bottom: 6px"
                @close="removeDomain(d)"
              >{{ d }}</el-tag>
              <span v-if="!domainWhitelist.length" class="empty-tip">暂无白名单域名，所有域名将被拦截（高安全等级下）</span>
            </div>
            <div class="form-tip">
              白名单中的域名允许访问。支持精确匹配（如 baidu.com）和通配符（如 *.example.com）。
            </div>
          </div>
        </el-form-item>

        <!-- 高危操作确认 -->
        <el-form-item label="高危操作确认">
          <el-checkbox v-model="confirmSensitiveDomain">访问敏感域名需确认</el-checkbox>
          <el-checkbox v-model="confirmFileDownload">文件下载需确认</el-checkbox>
          <el-checkbox v-model="confirmFormSubmit">表单提交需确认</el-checkbox>
          <div class="form-tip">开启后，执行涉及上述操作的任务前将弹出人工确认对话框。</div>
        </el-form-item>

        <!-- 敏感域名列表 -->
        <el-form-item label="敏感域名">
          <div class="whitelist-panel">
            <div class="whitelist-input">
              <el-input
                v-model="newSensitiveDomain"
                placeholder="输入敏感域名，如 bank.com"
                size="small"
                style="flex: 1"
                @keyup.enter="addSensitiveDomain"
              />
              <el-button size="small" type="warning" @click="addSensitiveDomain" style="margin-left: 8px">添加</el-button>
            </div>
            <div class="whitelist-tags">
              <el-tag
                v-for="d in sensitiveDomains"
                :key="d"
                type="warning"
                closable
                size="small"
                style="margin-right: 6px; margin-bottom: 6px"
                @close="removeSensitiveDomain(d)"
              >{{ d }}</el-tag>
              <span v-if="!sensitiveDomains.length" class="empty-tip">暂无敏感域名</span>
            </div>
          </div>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="showSecuritySettings = false">取消</el-button>
        <el-button type="primary" @click="saveSecuritySettings">保存设置</el-button>
      </template>
    </el-dialog>

    <!-- 高危操作确认弹窗 -->
    <el-dialog v-model="showRiskConfirm" title="高危操作确认" width="480px" :close-on-click-modal="false">
      <el-alert type="warning" :closable="false" show-icon style="margin-bottom: 16px">
        检测到以下操作存在安全风险，请确认是否继续执行。
      </el-alert>
      <div class="risk-items">
        <div v-for="(item, idx) in riskItems" :key="idx" class="risk-item">
          <el-icon color="#f59e0b"><Warning /></el-icon>
          <span>{{ item }}</span>
        </div>
      </div>
      <div class="risk-task">
        <div class="risk-task-label">任务内容：</div>
        <div class="risk-task-content">{{ pendingTaskContent }}</div>
      </div>
      <template #footer>
        <el-button @click="cancelRiskConfirm">取消</el-button>
        <el-button type="primary" @click="confirmRiskExecute">确认执行</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Promotion, Refresh, VideoPlay, Close, Setting, Warning } from '@element-plus/icons-vue'
import {
  getBrowserTemplates,
  getBrowserSessions,
  closeBrowserSession,
  browserNatural,
  executeBrowserTask
} from '@/api'

const router = useRouter()

// AI驱动浏览器：跳转到AI助手，带上浏览器自动化上下文
function goAIDrive() {
  router.push({ path: '/ai', query: { source: 'browser', action: 'drive' } })
}

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

// ==================== 安全设置相关 ====================
const SECURITY_STORAGE_KEY = 'browser-security-settings'
const AUDIT_LOG_KEY = 'browser-audit-log'

const showSecuritySettings = ref(false)
const securityLevel = ref('medium')
const domainWhitelist = ref(['baidu.com', 'github.com', 'bing.com'])
const sensitiveDomains = ref(['bank.com', 'paypal.com', 'alipay.com'])
const confirmSensitiveDomain = ref(true)
const confirmFileDownload = ref(true)
const confirmFormSubmit = ref(true)

// 临时输入
const newDomain = ref('')
const newSensitiveDomain = ref('')

const securityLevelLabel = computed(() => {
  const map = { low: '低', medium: '中', high: '高' }
  return map[securityLevel.value] || '中'
})

const securityLevelDesc = computed(() => {
  const map = {
    low: '仅记录审计日志，不限制域名访问',
    medium: '启用域名白名单检查，高危操作需确认',
    high: '严格白名单模式，所有外域访问均需确认'
  }
  return map[securityLevel.value] || ''
})

function loadSecuritySettings() {
  try {
    const stored = localStorage.getItem(SECURITY_STORAGE_KEY)
    if (stored) {
      const s = JSON.parse(stored)
      securityLevel.value = s.securityLevel || 'medium'
      domainWhitelist.value = s.domainWhitelist || ['baidu.com', 'github.com', 'bing.com']
      sensitiveDomains.value = s.sensitiveDomains || ['bank.com', 'paypal.com', 'alipay.com']
      confirmSensitiveDomain.value = s.confirmSensitiveDomain !== false
      confirmFileDownload.value = s.confirmFileDownload !== false
      confirmFormSubmit.value = s.confirmFormSubmit !== false
    }
  } catch (e) {
    console.warn('[browser-security] 加载设置失败', e)
  }
}

function saveSecuritySettings() {
  try {
    const s = {
      securityLevel: securityLevel.value,
      domainWhitelist: domainWhitelist.value,
      sensitiveDomains: sensitiveDomains.value,
      confirmSensitiveDomain: confirmSensitiveDomain.value,
      confirmFileDownload: confirmFileDownload.value,
      confirmFormSubmit: confirmFormSubmit.value
    }
    localStorage.setItem(SECURITY_STORAGE_KEY, JSON.stringify(s))
    ElMessage.success('安全设置已保存')
    showSecuritySettings.value = false
  } catch (e) {
    ElMessage.error('保存失败：' + e.message)
  }
}

function addDomain() {
  const d = newDomain.value.trim().toLowerCase()
  if (!d) return
  if (domainWhitelist.value.includes(d)) {
    ElMessage.warning('该域名已在白名单中')
    return
  }
  domainWhitelist.value.push(d)
  newDomain.value = ''
}

function removeDomain(d) {
  domainWhitelist.value = domainWhitelist.value.filter((x) => x !== d)
}

function addSensitiveDomain() {
  const d = newSensitiveDomain.value.trim().toLowerCase()
  if (!d) return
  if (sensitiveDomains.value.includes(d)) {
    ElMessage.warning('该域名已在敏感列表中')
    return
  }
  sensitiveDomains.value.push(d)
  newSensitiveDomain.value = ''
}

function removeSensitiveDomain(d) {
  sensitiveDomains.value = sensitiveDomains.value.filter((x) => x !== d)
}

// 从文本中提取可能的域名
function extractDomainsFromText(text) {
  const domainRegex = /(?:https?:\/\/)?([a-zA-Z0-9][-a-zA-Z0-9]*\.[-a-zA-Z0-9.]+[a-zA-Z])/gi
  const matches = text.match(domainRegex) || []
  return [...new Set(matches.map((m) => m.replace(/^https?:\/\//, '').toLowerCase()))]
}

// 检查域名是否在白名单中（支持通配符 *.domain.com）
function isDomainInWhitelist(domain) {
  const d = domain.toLowerCase()
  return domainWhitelist.value.some((w) => {
    if (w.startsWith('*.')) {
      const suffix = w.slice(2) // 去掉 *.
      return d.endsWith(suffix) || d === suffix
    }
    return d === w || d.endsWith('.' + w)
  })
}

// 检查是否为敏感域名
function isSensitiveDomain(domain) {
  const d = domain.toLowerCase()
  return sensitiveDomains.value.some((s) => d === s || d.endsWith('.' + s))
}

// 检测文本中的高危操作关键词
function detectHighRiskOperations(text) {
  const risks = []
  const t = text.toLowerCase()

  // 检测文件下载关键词
  if (confirmFileDownload.value && /(下载|download|保存文件|save.*file|export)/i.test(t)) {
    risks.push('涉及文件下载操作')
  }

  // 检测表单提交关键词
  if (confirmFormSubmit.value && /(提交|submit|登录|login|注册|register|付款|支付|pay|结算)/i.test(t)) {
    risks.push('涉及表单提交/登录操作')
  }

  // 检测敏感域名
  const domains = extractDomainsFromText(text)
  if (confirmSensitiveDomain.value) {
    const sensitiveFound = domains.filter((d) => isSensitiveDomain(d))
    if (sensitiveFound.length) {
      risks.push(`访问敏感域名：${sensitiveFound.join(', ')}`)
    }
  }

  return risks
}

// 执行前安全检查
// 返回 { allowed: boolean, reason: string, risks: string[] }
function securityCheck(text) {
  const result = { allowed: true, reason: '', risks: [] }

  // 低安全等级：仅记录，不拦截
  if (securityLevel.value === 'low') {
    return result
  }

  const domains = extractDomainsFromText(text)

  // 中/高安全等级：检查高危操作
  const risks = detectHighRiskOperations(text)
  result.risks = risks

  // 高安全等级：严格白名单
  if (securityLevel.value === 'high' && domains.length) {
    const notAllowed = domains.filter((d) => !isDomainInWhitelist(d))
    if (notAllowed.length) {
      result.allowed = false
      result.reason = `以下域名不在白名单中，高安全等级下禁止访问：${notAllowed.join(', ')}`
      return result
    }
  }

  // 中安全等级：白名单外的域名也需要确认
  if (securityLevel.value === 'medium' && domains.length) {
    const notInWhitelist = domains.filter((d) => !isDomainInWhitelist(d))
    if (notInWhitelist.length) {
      result.risks.push(`访问非白名单域名：${notInWhitelist.join(', ')}`)
    }
  }

  return result
}

// ==================== 高危操作确认 ====================
const showRiskConfirm = ref(false)
const riskItems = ref([])
const pendingTaskContent = ref('')
let pendingExecuteFn = null

function cancelRiskConfirm() {
  showRiskConfirm.value = false
  pendingExecuteFn = null
  ElMessage.info('已取消执行')
}

function confirmRiskExecute() {
  showRiskConfirm.value = false
  if (pendingExecuteFn) {
    const fn = pendingExecuteFn
    pendingExecuteFn = null
    fn()
  }
}

// ==================== 操作审计日志 ====================
const auditLog = ref([])
const auditFilter = ref('all')

const filteredAuditLog = computed(() => {
  if (auditFilter.value === 'all') return auditLog.value
  return auditLog.value.filter((a) => a.type === auditFilter.value)
})

function loadAuditLog() {
  try {
    const stored = localStorage.getItem(AUDIT_LOG_KEY)
    if (stored) {
      auditLog.value = JSON.parse(stored)
    }
  } catch (e) {
    console.warn('[browser-audit] 加载日志失败', e)
  }
}

function saveAuditLog() {
  try {
    // 最多保留 200 条
    if (auditLog.value.length > 200) {
      auditLog.value = auditLog.value.slice(0, 200)
    }
    localStorage.setItem(AUDIT_LOG_KEY, JSON.stringify(auditLog.value))
  } catch (e) {
    console.warn('[browser-audit] 保存日志失败', e)
  }
}

function addAuditLog(type, content, targetUrl, result, detail) {
  const entry = {
    id: Date.now() + Math.random(),
    time: new Date().toISOString(),
    type,
    content,
    targetUrl: targetUrl || '',
    result,
    detail: detail || ''
  }
  auditLog.value.unshift(entry)
  saveAuditLog()
}

function clearAuditLog() {
  ElMessageBox.confirm('确定要清空所有审计日志吗？此操作不可恢复。', '确认', {
    type: 'warning'
  }).then(() => {
    auditLog.value = []
    saveAuditLog()
    ElMessage.success('日志已清空')
  }).catch(() => {})
}

function auditTypeLabel(type) {
  const map = {
    natural: '自然语言执行',
    template: '模板运行',
    close: '会话关闭',
    security_block: '安全拦截'
  }
  return map[type] || type
}

function auditTypeTag(type) {
  const map = {
    natural: 'primary',
    template: 'success',
    close: 'info',
    security_block: 'danger'
  }
  return map[type] || 'info'
}

function formatTime(iso) {
  try {
    const d = new Date(iso)
    return d.toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    })
  } catch (e) {
    return iso
  }
}

// ==================== 业务逻辑（带安全检查） ====================
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

  // 安全检查
  const check = securityCheck(task.value)
  if (!check.allowed) {
    ElMessage.error(check.reason)
    // 记录拦截日志
    addAuditLog('security_block', task.value, extractDomainsFromText(task.value).join(', '), 'blocked', check.reason)
    return
  }

  // 提取目标URL用于审计
  const domains = extractDomainsFromText(task.value)
  const targetUrl = domains[0] || ''

  // 如果有高危操作，需要确认
  if (check.risks.length > 0) {
    riskItems.value = check.risks
    pendingTaskContent.value = task.value
    pendingExecuteFn = () => doRunNatural(targetUrl)
    showRiskConfirm.value = true
    return
  }

  await doRunNatural(targetUrl)
}

async function doRunNatural(targetUrl) {
  running.value = true
  naturalResult.value = null
  const taskContent = task.value
  try {
    naturalResult.value = await browserNatural({ prompt: task.value })
    ElMessage.success('任务已提交')
    addAuditLog('natural', taskContent, targetUrl, 'success', '任务已提交执行')
    await loadAll()
  } catch (e) {
    ElMessage.error(e.message)
    addAuditLog('natural', taskContent, targetUrl, 'failed', e.message)
  } finally {
    running.value = false
  }
}

async function runTpl(t) {
  const tplName = t.name || t.id
  const tplDesc = t.description || ''
  const text = tplName + ' ' + tplDesc

  // 安全检查
  const check = securityCheck(text)
  if (!check.allowed) {
    ElMessage.error(check.reason)
    addAuditLog('security_block', `运行模板: ${tplName}`, '', 'blocked', check.reason)
    return
  }

  const domains = extractDomainsFromText(text)
  const targetUrl = domains[0] || ''

  // 如果有高危操作，需要确认
  if (check.risks.length > 0) {
    riskItems.value = check.risks
    pendingTaskContent.value = `运行模板：${tplName}`
    pendingExecuteFn = () => doRunTpl(t, tplName, targetUrl)
    showRiskConfirm.value = true
    return
  }

  await doRunTpl(t, tplName, targetUrl)
}

async function doRunTpl(t, tplName, targetUrl) {
  try {
    await executeBrowserTask({ task_id: t.id, variables: {} })
    ElMessage.success('模板已运行')
    addAuditLog('template', `运行模板: ${tplName}`, targetUrl, 'success', `模板ID: ${t.id}`)
    await loadAll()
  } catch (e) {
    ElMessage.error(e.message)
    addAuditLog('template', `运行模板: ${tplName}`, targetUrl, 'failed', e.message)
  }
}

async function closeSess(s) {
  try {
    await closeBrowserSession(s.id)
    ElMessage.success('会话已关闭')
    addAuditLog('close', `关闭会话: ${s.id}`, s.url || '', 'success', '')
    await loadAll()
  } catch (e) {
    ElMessage.error(e.message)
    addAuditLog('close', `关闭会话: ${s.id}`, s.url || '', 'failed', e.message)
  }
}

onMounted(() => {
  loadSecuritySettings()
  loadAuditLog()
  loadAll()
})
</script>

<style scoped>
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

/* 安全等级提示条 */
.security-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
  border-radius: 10px;
  margin-bottom: 12px;
  font-size: 13px;
}
.security-banner.low {
  background: #ecfdf5;
  color: #065f46;
  border: 1px solid #a7f3d0;
}
.security-banner.medium {
  background: #fffbeb;
  color: #92400e;
  border: 1px solid #fde68a;
}
.security-banner.high {
  background: #fef2f2;
  color: #991b1b;
  border: 1px solid #fecaca;
}
.security-banner .banner-desc {
  color: var(--text-3);
  margin-left: auto;
  font-size: 12px;
}

/* 白名单面板 */
.whitelist-panel {
  width: 100%;
}
.whitelist-input {
  display: flex;
  margin-bottom: 10px;
}
.whitelist-tags {
  min-height: 32px;
}
.empty-tip {
  font-size: 12px;
  color: var(--text-3);
}
.form-tip {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 6px;
  line-height: 1.5;
}

/* 审计日志 */
.audit-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.audit-actions {
  display: flex;
  gap: 8px;
}
.audit-content {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.section-title {
  margin: 0 0 12px 0;
  font-size: 15px;
  font-weight: 600;
}

/* 风险确认弹窗 */
.risk-items {
  margin-bottom: 16px;
}
.risk-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #fffbeb;
  border-radius: 6px;
  margin-bottom: 6px;
  font-size: 13px;
  color: #92400e;
}
.risk-task {
  background: #f8fafc;
  padding: 12px;
  border-radius: 8px;
}
.risk-task-label {
  font-size: 12px;
  color: var(--text-3);
  margin-bottom: 4px;
}
.risk-task-content {
  font-size: 13px;
  color: var(--text-1);
  word-break: break-all;
}
</style>
