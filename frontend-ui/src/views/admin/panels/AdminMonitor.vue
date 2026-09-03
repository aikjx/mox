<template>
  <div class="page-container monitor-page">
    <!-- ===== 页头 ===== -->
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">系统监控</h2>
        <p class="page-subtitle">运行时健康度 · 服务质量 · 业务指标 · 实时告警观测</p>
      </div>
      <div class="page-header-actions">
        <div class="refresh-info" v-if="autoRefresh">
          <span class="countdown" :class="{ flash: dataFlash }">{{ refreshCountdown }}s</span>
          <span class="refresh-label">后自动刷新</span>
        </div>
        <el-switch
          v-model="autoRefresh"
          active-text="自动刷新"
          inactive-text="暂停"
          :width="72"
          @change="onAutoRefreshChange"
        />
        <el-button :loading="loading" @click="manualRefresh">
          <el-icon><Refresh /></el-icon> 手动刷新
        </el-button>
      </div>
    </div>

    <div class="page-content">

      <!-- ===== 一、系统资源指标 ===== -->
      <div class="section-block">
        <div class="section-header">
          <h3 class="section-title-bar"><span class="bar-icon sys"></span>系统资源</h3>
          <span class="section-hint">实时更新 · 最近采样 {{ lastUpdateTime }}</span>
        </div>
        <div class="grid grid-4 kpi-row">
          <div class="panel kpi" :class="{ 'kpi-flash': dataFlash }">
            <div class="kpi-top">
              <span class="kpi-label">CPU 使用率</span>
              <el-icon class="kpi-icon"><Cpu /></el-icon>
            </div>
            <div class="kpi-value" :class="cpuLevel">
              {{ systemMetrics.cpu.toFixed(1) }}%
            </div>
            <div class="kpi-sub">
              <el-progress :percentage="systemMetrics.cpu" :stroke-width="4" :color="cpuColor" />
            </div>
          </div>

          <div class="panel kpi" :class="{ 'kpi-flash': dataFlash }">
            <div class="kpi-top">
              <span class="kpi-label">内存使用率</span>
              <el-icon class="kpi-icon"><Coin /></el-icon>
            </div>
            <div class="kpi-value" :class="memLevel">
              {{ systemMetrics.memory.toFixed(1) }}%
            </div>
            <div class="kpi-sub">
              <span class="mem-detail">{{ formatGB(systemMetrics.memUsed) }} / {{ formatGB(systemMetrics.memTotal) }}</span>
            </div>
          </div>

          <div class="panel kpi" :class="{ 'kpi-flash': dataFlash }">
            <div class="kpi-top">
              <span class="kpi-label">磁盘使用率</span>
              <el-icon class="kpi-icon"><Files /></el-icon>
            </div>
            <div class="disk-list">
              <div class="disk-item" v-for="d in systemMetrics.disks" :key="d.name">
                <div class="disk-name">{{ d.name }}</div>
                <div class="disk-bar">
                  <div class="disk-bar-inner" :style="{ width: d.usage + '%', background: diskColor(d.usage) }"></div>
                </div>
                <div class="disk-val">{{ d.usage.toFixed(0) }}%</div>
              </div>
            </div>
          </div>

          <div class="panel kpi" :class="{ 'kpi-flash': dataFlash }">
            <div class="kpi-top">
              <span class="kpi-label">网络流量</span>
              <el-icon class="kpi-icon"><Connection /></el-icon>
            </div>
            <div class="net-values">
              <div class="net-row">
                <span class="net-dir up">↑ 上传</span>
                <span class="net-val">{{ formatSpeed(systemMetrics.netUpload) }}</span>
              </div>
              <div class="net-row">
                <span class="net-dir down">↓ 下载</span>
                <span class="net-val">{{ formatSpeed(systemMetrics.netDownload) }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ===== 二、服务质量指标 ===== -->
      <div class="section-block">
        <div class="section-header">
          <h3 class="section-title-bar"><span class="bar-icon qos"></span>服务质量</h3>
          <span class="section-hint">最近 1 小时统计</span>
        </div>
        <div class="grid grid-5 kpi-row">
          <div class="panel kpi small" :class="{ 'kpi-flash': dataFlash }">
            <div class="kpi-label">QPS</div>
            <div class="kpi-value qos-val">{{ qualityMetrics.qps.toFixed(0) }}</div>
            <div class="kpi-trend up" v-if="qualityMetrics.qpsTrend > 0">
              <el-icon><Top /></el-icon> {{ qualityMetrics.qpsTrend.toFixed(1) }}%
            </div>
            <div class="kpi-trend down" v-else>
              <el-icon><Bottom /></el-icon> {{ Math.abs(qualityMetrics.qpsTrend).toFixed(1) }}%
            </div>
          </div>

          <div class="panel kpi small" :class="{ 'kpi-flash': dataFlash }">
            <div class="kpi-label">错误率</div>
            <div class="kpi-value" :class="errLevel">{{ qualityMetrics.errorRate.toFixed(2) }}%</div>
            <div class="kpi-sub">
              {{ qualityMetrics.errorCount }} 次错误 / 小时
            </div>
          </div>

          <div class="panel kpi small" :class="{ 'kpi-flash': dataFlash }">
            <div class="kpi-label">平均延迟</div>
            <div class="kpi-value qos-val">{{ qualityMetrics.avgLatency }}<span class="unit">ms</span></div>
            <div class="kpi-sub">
              P50 {{ qualityMetrics.p50 }} · P95 {{ qualityMetrics.p95 }} · P99 {{ qualityMetrics.p99 }} ms
            </div>
          </div>

          <div class="panel kpi small" :class="{ 'kpi-flash': dataFlash }">
            <div class="kpi-label">在线用户</div>
            <div class="kpi-value qos-val">{{ qualityMetrics.onlineUsers }}</div>
            <div class="kpi-sub">
              峰值 {{ qualityMetrics.peakUsers }} 人
            </div>
          </div>

          <div class="panel kpi small" :class="{ 'kpi-flash': dataFlash }">
            <div class="kpi-label">活跃连接</div>
            <div class="kpi-value qos-val">{{ qualityMetrics.activeConnections }}</div>
            <div class="kpi-sub">
              WebSocket · 长连接
            </div>
          </div>
        </div>
      </div>

      <!-- ===== 三、业务指标 + 告警统计 ===== -->
      <div class="grid grid-2">
        <div class="section-block">
          <div class="section-header">
            <h3 class="section-title-bar"><span class="bar-icon biz"></span>业务指标</h3>
            <span class="section-hint">今日累计</span>
          </div>
          <div class="grid grid-4 kpi-row">
            <div class="panel kpi mini" :class="{ 'kpi-flash': dataFlash }">
              <div class="kpi-label">对话数</div>
              <div class="kpi-value biz-val">{{ businessMetrics.conversations }}</div>
            </div>
            <div class="panel kpi mini" :class="{ 'kpi-flash': dataFlash }">
              <div class="kpi-label">专家咨询</div>
              <div class="kpi-value biz-val">{{ businessMetrics.expertConsultations }}</div>
            </div>
            <div class="panel kpi mini" :class="{ 'kpi-flash': dataFlash }">
              <div class="kpi-label">工作流执行</div>
              <div class="kpi-value biz-val">{{ businessMetrics.workflowRuns }}</div>
            </div>
            <div class="panel kpi mini" :class="{ 'kpi-flash': dataFlash }">
              <div class="kpi-label">算子调用</div>
              <div class="kpi-value biz-val">{{ businessMetrics.operatorCalls }}</div>
            </div>
          </div>
        </div>

        <div class="section-block">
          <div class="section-header">
            <h3 class="section-title-bar"><span class="bar-icon alert"></span>告警统计</h3>
            <span class="section-hint">今日累计</span>
          </div>
          <div class="grid grid-4 kpi-row">
            <div class="panel kpi mini" :class="{ 'kpi-flash': dataFlash }">
              <div class="kpi-label">告警总数</div>
              <div class="kpi-value alert-total">{{ alertMetrics.total }}</div>
            </div>
            <div class="panel kpi mini" :class="{ 'kpi-flash': dataFlash }">
              <div class="kpi-label">严重 P0</div>
              <div class="kpi-value alert-p0">{{ alertMetrics.p0 }}</div>
            </div>
            <div class="panel kpi mini" :class="{ 'kpi-flash': dataFlash }">
              <div class="kpi-label">警告 P1</div>
              <div class="kpi-value alert-p1">{{ alertMetrics.p1 }}</div>
            </div>
            <div class="panel kpi mini" :class="{ 'kpi-flash': dataFlash }">
              <div class="kpi-label">通知 P2</div>
              <div class="kpi-value alert-p2">{{ alertMetrics.p2 }}</div>
            </div>
          </div>
        </div>
      </div>

      <!-- ===== 四、历史趋势图 ===== -->
      <div class="section-block">
        <div class="section-header">
          <h3 class="section-title-bar"><span class="bar-icon chart"></span>历史趋势</h3>
          <el-radio-group v-model="trendRange" size="small" @change="onTrendRangeChange">
            <el-radio-button value="1h">最近1小时</el-radio-button>
            <el-radio-button value="6h">最近6小时</el-radio-button>
            <el-radio-button value="24h">最近24小时</el-radio-button>
          </el-radio-group>
        </div>

        <div class="grid grid-2 chart-row">
          <div class="panel card-pad">
            <h3 class="chart-title">QPS 与错误率</h3>
            <div ref="qpsChartEl" class="chart tall"></div>
          </div>
          <div class="panel card-pad">
            <h3 class="chart-title">响应延迟分布 (P50 / P95 / P99)</h3>
            <div ref="latencyChartEl" class="chart tall"></div>
          </div>
        </div>

        <div class="grid grid-2 chart-row">
          <div class="panel card-pad">
            <h3 class="chart-title">CPU & 内存使用率</h3>
            <div ref="resChartEl" class="chart tall"></div>
          </div>
          <div class="panel card-pad">
            <h3 class="chart-title">业务量统计（最近 7 天）</h3>
            <div ref="bizChartEl" class="chart tall"></div>
          </div>
        </div>
      </div>

      <!-- ===== 五、服务健康状态总览 ===== -->
      <div class="section-block">
        <div class="section-header">
          <h3 class="section-title-bar"><span class="bar-icon health"></span>服务健康状态</h3>
          <span class="section-hint">
            <span class="health-summary up">{{ healthSummary.up }} 正常</span>
            <span class="health-summary warn">{{ healthSummary.warning }} 警告</span>
            <span class="health-summary down">{{ healthSummary.down }} 异常</span>
          </span>
        </div>
        <div class="panel card-pad">
          <el-table :data="serviceNodes" stripe style="width: 100%" height="280">
            <el-table-column prop="name" label="服务节点" min-width="180">
              <template #default="{ row }">
                <div class="svc-name">
                  <span class="svc-dot" :class="row.status"></span>
                  <b>{{ row.name }}</b>
                </div>
              </template>
            </el-table-column>
            <el-table-column prop="version" label="版本" width="100" />
            <el-table-column prop="cpu" label="CPU" width="100">
              <template #default="{ row }">
                <span :class="valLevel(row.cpu, 70, 90)">{{ row.cpu }}%</span>
              </template>
            </el-table-column>
            <el-table-column prop="memory" label="内存" width="100">
              <template #default="{ row }">
                <span :class="valLevel(row.memory, 75, 90)">{{ row.memory }}%</span>
              </template>
            </el-table-column>
            <el-table-column prop="qps" label="QPS" width="90" />
            <el-table-column prop="latency" label="延迟(ms)" width="100" />
            <el-table-column prop="uptime" label="运行时长" width="120" />
            <el-table-column label="操作" width="180" fixed="right">
              <template #default="{ row }">
                <el-button size="small" type="primary" link @click="jumpToLogs(row)">
                  <el-icon><Document /></el-icon> 日志
                </el-button>
                <el-button size="small" type="primary" link @click="jumpToTrace(row)">
                  <el-icon><Share /></el-icon> 链路追踪
                </el-button>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </div>

      <!-- ===== 六、告警规则配置 ===== -->
      <div class="section-block">
        <div class="section-header">
          <h3 class="section-title-bar"><span class="bar-icon rule"></span>告警规则配置</h3>
          <el-button type="primary" size="small" @click="showRuleDialog = true; editingRule = null">
            <el-icon><Plus /></el-icon> 新建规则
          </el-button>
        </div>
        <div class="panel card-pad">
          <el-table :data="alertRules" stripe style="width: 100%">
            <el-table-column prop="name" label="规则名称" min-width="160" />
            <el-table-column prop="metric" label="监控指标" width="120" />
            <el-table-column label="阈值条件" width="180">
              <template #default="{ row }">
                {{ row.operator }} {{ row.threshold }}{{ row.unit || '' }}
                <span class="rule-duration">持续 {{ row.duration }}</span>
              </template>
            </el-table-column>
            <el-table-column label="告警级别" width="100">
              <template #default="{ row }">
                <el-tag :type="levelTagType(row.level)" size="small">{{ row.level }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="channels" label="通知渠道" min-width="160">
              <template #default="{ row }">
                <el-tag v-for="c in row.channels" :key="c" size="small" class="channel-tag">{{ c }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="状态" width="90">
              <template #default="{ row }">
                <el-switch v-model="row.enabled" size="small" @change="toggleRule(row)" />
              </template>
            </el-table-column>
            <el-table-column label="操作" width="140" fixed="right">
              <template #default="{ row }">
                <el-button size="small" link type="primary" @click="editRule(row)">编辑</el-button>
                <el-button size="small" link type="danger" @click="deleteRule(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </div>

      <!-- ===== 原有：璇玑治理面板 ===== -->
      <div class="panel card-pad">
        <div class="mox-head">
          <div>
            <h3 class="section-title">璇玑 · 双璇玑十四维治理</h3>
            <p class="page-subtitle">业务七维 + 开发七维全维健康分；粘贴流程蓝图实时治理评分（璇玑最高权限校验）</p>
          </div>
          <div class="mox-actions">
            <el-upload
              action="#"
              :auto-upload="false"
              :show-file-list="false"
              accept=".json"
              :on-change="onFlowFile"
            >
              <el-button><el-icon><Upload /></el-icon> 载入蓝图</el-button>
            </el-upload>
            <el-button type="primary" :loading="governing" @click="runGovernance">
              <el-icon><MagicStick /></el-icon> 全维治理
            </el-button>
          </div>
        </div>

        <div class="grid grid-2 mox-body">
          <div>
            <div ref="radarEl" class="chart"></div>
            <el-input
              v-model="flowJson"
              type="textarea"
              :rows="6"
              placeholder='粘贴 FlowGraph JSON，例如 {"nodes":[{"id":"n1","type":"input"}],"edges":[]}'
              class="flow-input"
            />
          </div>
          <div class="gov-result">
            <div class="gov-badges">
              <span class="badge" :class="gateApproved ? 'success' : 'warning'">
                治理闸门：{{ gateApproved ? '通过' : (governed ? '拦截' : '待评') }}
              </span>
              <span class="badge info">璇玑：{{ mox }}</span>
              <span class="badge info">采纳建议：{{ adopted.length }}</span>
            </div>
            <h4 class="sub">采纳的优化建议</h4>
            <el-empty v-if="!adopted.length" description="暂无采纳建议" :image-size="60" />
            <ul class="suggest-list">
              <li v-for="(s, i) in adopted" :key="i">
                <b>{{ s.dimension }}</b> · {{ s.summary }}
              </li>
            </ul>
          </div>
        </div>
      </div>

      <!-- ===== 原有：执行日志 ===== -->
      <div class="panel card-pad">
        <h3 class="section-title">执行日志</h3>
        <el-table :data="logRows" stripe height="320" style="width: 100%">
          <el-table-column prop="time" label="时间" width="180" />
          <el-table-column prop="flow" label="算子链" min-width="220" />
          <el-table-column prop="status" label="状态" width="100">
            <template #default="{ row }">
              <span class="badge" :class="row.status === '成功' ? 'success' : 'warning'">{{ row.status }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="time_ms" label="耗时" width="100" />
          <el-table-column prop="dims" label="维度" min-width="120" />
        </el-table>
      </div>

    </div>

    <!-- ===== 告警规则编辑弹窗 ===== -->
    <el-dialog
      v-model="showRuleDialog"
      :title="editingRule ? '编辑告警规则' : '新建告警规则'"
      width="520px"
      @close="resetRuleForm"
    >
      <el-form :model="ruleForm" label-width="100px" class="rule-form">
        <el-form-item label="规则名称">
          <el-input v-model="ruleForm.name" placeholder="例如：CPU 使用率过高" />
        </el-form-item>
        <el-form-item label="监控指标">
          <el-select v-model="ruleForm.metric" placeholder="选择指标" style="width: 100%">
            <el-option label="CPU 使用率" value="CPU" />
            <el-option label="内存使用率" value="内存" />
            <el-option label="QPS" value="QPS" />
            <el-option label="错误率" value="错误率" />
            <el-option label="响应延迟" value="延迟" />
            <el-option label="磁盘使用率" value="磁盘" />
          </el-select>
        </el-form-item>
        <el-form-item label="阈值条件">
          <el-select v-model="ruleForm.operator" style="width: 90px; margin-right: 8px">
            <el-option label="大于" value=">" />
            <el-option label="小于" value="<" />
            <el-option label="等于" value="=" />
            <el-option label="大于等于" value=">=" />
            <el-option label="小于等于" value="<=" />
          </el-select>
          <el-input-number v-model="ruleForm.threshold" :min="0" :max="99999" style="width: 140px" />
          <span class="form-unit">{{ ruleForm.unit }}</span>
        </el-form-item>
        <el-form-item label="持续时间">
          <el-select v-model="ruleForm.duration" style="width: 100%">
            <el-option label="1 分钟" value="1分钟" />
            <el-option label="5 分钟" value="5分钟" />
            <el-option label="15 分钟" value="15分钟" />
            <el-option label="30 分钟" value="30分钟" />
          </el-select>
        </el-form-item>
        <el-form-item label="告警级别">
          <el-radio-group v-model="ruleForm.level">
            <el-radio-button value="P0">严重</el-radio-button>
            <el-radio-button value="P1">警告</el-radio-button>
            <el-radio-button value="P2">通知</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="通知渠道">
          <el-checkbox-group v-model="ruleForm.channels">
            <el-checkbox label="站内信" />
            <el-checkbox label="邮件" />
            <el-checkbox label="飞书" />
            <el-checkbox label="Webhook" />
          </el-checkbox-group>
        </el-form-item>
        <el-form-item label="启用状态">
          <el-switch v-model="ruleForm.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showRuleDialog = false">取消</el-button>
        <el-button type="primary" @click="saveRule">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import * as echarts from '@/echarts'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getStatus, getFullStatus, getLogs, getPlugins, moxHealth, moxOptimize } from '@/api'
import {
  getMetricsDetail, getMonitorQuality, getMonitorBusiness, getAlertsSummary,
  getMonitorNodes, getAlertRules, createAlertRule, updateAlertRule,
  deleteAlertRule, toggleAlertRule, getTimeseries, getBusinessTimeseries
} from '@/api/monitor.api'
import { useProject } from '@/composables/projectContext.js'

// ===== 基础状态 =====
const loading = ref(false)
const autoRefresh = ref(true)
const refreshInterval = 5 // 秒
const refreshCountdown = ref(5)
const dataFlash = ref(false)
const lastUpdateTime = ref('--:--:--')
const trendRange = ref('1h')

let refreshTimer = null
let flashTimer = null

// ===== 系统资源指标（演示占位：后端待提供 /actuator/metrics 详细指标聚合端点） =====
const systemMetrics = reactive({
  cpu: 35.2,
  memory: 58.4,
  memTotal: 32,
  memUsed: 18.7,
  disks: [
    { name: '系统盘 C:', usage: 62.5 },
    { name: '数据盘 D:', usage: 45.3 },
    { name: '备份盘 E:', usage: 78.1 }
  ],
  netUpload: 2.5, // MB/s
  netDownload: 18.3 // MB/s
})

const cpuLevel = computed(() => {
  if (systemMetrics.cpu >= 90) return 'bad'
  if (systemMetrics.cpu >= 70) return 'warn'
  return 'ok'
})
const cpuColor = computed(() => {
  if (systemMetrics.cpu >= 90) return '#f56c6c'
  if (systemMetrics.cpu >= 70) return '#e6a23c'
  return '#10b981'
})
const memLevel = computed(() => {
  if (systemMetrics.memory >= 90) return 'bad'
  if (systemMetrics.memory >= 75) return 'warn'
  return 'ok'
})

function diskColor(usage) {
  if (usage >= 90) return '#f56c6c'
  if (usage >= 75) return '#e6a23c'
  return '#10b981'
}

function formatGB(val) {
  return val.toFixed(1) + ' GB'
}

function formatSpeed(mbps) {
  if (mbps >= 1024) return (mbps / 1024).toFixed(2) + ' GB/s'
  return mbps.toFixed(1) + ' MB/s'
}

// ===== 服务质量指标（演示占位：后端待提供 /monitor/quality 端点） =====
const qualityMetrics = reactive({
  qps: 128.5,
  qpsTrend: 5.2,
  errorRate: 0.12,
  errorCount: 46,
  avgLatency: 42,
  p50: 28,
  p95: 125,
  p99: 380,
  onlineUsers: 1247,
  peakUsers: 2156,
  activeConnections: 892
})

const errLevel = computed(() => {
  if (qualityMetrics.errorRate >= 5) return 'bad'
  if (qualityMetrics.errorRate >= 1) return 'warn'
  return 'ok'
})

// ===== 业务指标（演示占位：后端待提供 /monitor/business 聚合端点） =====
const businessMetrics = reactive({
  conversations: 3562,
  expertConsultations: 128,
  workflowRuns: 892,
  operatorCalls: 12450
})

// ===== 告警统计（演示占位：后端待提供 /monitor/alerts/summary 端点） =====
const alertMetrics = reactive({
  total: 23,
  p0: 2,
  p1: 8,
  p2: 13
})

// ===== 服务节点状态（演示占位：后端待提供 /monitor/nodes 服务发现端点） =====
const serviceNodes = ref([])
const healthSummary = computed(() => {
  let up = 0, warning = 0, down = 0
  serviceNodes.value.forEach(n => {
    if (n.status === 'up') up++
    else if (n.status === 'warning') warning++
    else down++
  })
  return { up, warning, down }
})

function valLevel(val, warnThreshold, badThreshold) {
  if (val >= badThreshold) return 'bad'
  if (val >= warnThreshold) return 'warn'
  return 'ok'
}

// 后端待提供: 服务节点日志跳转端点 /monitor/nodes/{name}/logs
function jumpToLogs(row) {
  ElMessage.info(`跳转到 ${row.name} 的日志页面`)
}

// 后端待提供: 服务节点链路追踪跳转端点 /monitor/nodes/{name}/trace
function jumpToTrace(row) {
  ElMessage.info(`跳转到 ${row.name} 的链路追踪页面`)
}

// ===== 告警规则（演示占位：后端待提供 /monitor/alert-rules CRUD 端点，当前为本地内存操作） =====
const alertRules = ref([
  { id: 1, name: 'CPU 使用率过高', metric: 'CPU', operator: '>', threshold: 80, unit: '%', duration: '5分钟', level: 'P1', channels: ['站内信', '邮件'], enabled: true },
  { id: 2, name: '内存使用率告警', metric: '内存', operator: '>', threshold: 85, unit: '%', duration: '5分钟', level: 'P1', channels: ['站内信', '邮件'], enabled: true },
  { id: 3, name: '错误率过高', metric: '错误率', operator: '>', threshold: 2, unit: '%', duration: '1分钟', level: 'P0', channels: ['站内信', '邮件', '飞书'], enabled: true },
  { id: 4, name: 'P99 延迟过高', metric: '延迟', operator: '>', threshold: 500, unit: 'ms', duration: '5分钟', level: 'P1', channels: ['站内信'], enabled: true },
  { id: 5, name: '磁盘空间不足', metric: '磁盘', operator: '>', threshold: 90, unit: '%', duration: '15分钟', level: 'P2', channels: ['站内信', '邮件'], enabled: false },
  { id: 6, name: 'QPS 突降', metric: 'QPS', operator: '<', threshold: 10, unit: '', duration: '5分钟', level: 'P0', channels: ['站内信', '飞书', 'Webhook'], enabled: true }
])

const showRuleDialog = ref(false)
const editingRule = ref(null)
const ruleForm = reactive({
  name: '',
  metric: 'CPU',
  operator: '>',
  threshold: 80,
  unit: '%',
  duration: '5分钟',
  level: 'P1',
  channels: ['站内信'],
  enabled: true
})

function levelTagType(level) {
  if (level === 'P0') return 'danger'
  if (level === 'P1') return 'warning'
  return 'info'
}

// 告警规则启用/禁用：调用 PUT /monitor/alert-rules/{id}/toggle，失败保留本地状态
async function toggleRule(row) {
  try {
    await toggleAlertRule(row.id, row.enabled)
    ElMessage.success(`规则「${row.name}」已${row.enabled ? '启用' : '禁用'}`)
  } catch (e) {
    row.enabled = !row.enabled
    ElMessage.warning('规则状态同步失败，已回滚本地状态')
  }
}

function editRule(row) {
  editingRule.value = row
  Object.assign(ruleForm, row)
  showRuleDialog.value = true
}

// 告警规则删除：调用 DELETE /monitor/alert-rules/{id}，失败保留本地列表
async function deleteRule(row) {
  ElMessageBox.confirm(`确定删除规则「${row.name}」吗？`, '确认删除', {
    type: 'warning'
  }).then(async () => {
    try {
      await deleteAlertRule(row.id)
      const idx = alertRules.value.findIndex(r => r.id === row.id)
      if (idx >= 0) alertRules.value.splice(idx, 1)
      ElMessage.success('删除成功')
    } catch (e) {
      ElMessage.error('删除失败：' + e.message)
    }
  }).catch(() => {})
}

function resetRuleForm() {
  editingRule.value = null
  Object.assign(ruleForm, {
    name: '',
    metric: 'CPU',
    operator: '>',
    threshold: 80,
    unit: '%',
    duration: '5分钟',
    level: 'P1',
    channels: ['站内信'],
    enabled: true
  })
}

// 告警规则创建/更新：调用 POST/PUT /monitor/alert-rules，失败保留本地操作
async function saveRule() {
  if (!ruleForm.name.trim()) {
    ElMessage.warning('请输入规则名称')
    return
  }
  try {
    if (editingRule.value) {
      await updateAlertRule(editingRule.value.id, { ...ruleForm })
      const idx = alertRules.value.findIndex(r => r.id === editingRule.value.id)
      if (idx >= 0) Object.assign(alertRules.value[idx], { ...ruleForm })
      ElMessage.success('规则已更新')
    } else {
      const created = await createAlertRule({ ...ruleForm })
      const newRule = created && created.id ? created : { id: Date.now(), ...ruleForm }
      alertRules.value.push(newRule)
      ElMessage.success('规则已创建')
    }
    showRuleDialog.value = false
  } catch (e) {
    ElMessage.error('保存失败：' + e.message)
  }
}

// 监听指标变化，自动更新单位
watch(() => ruleForm.metric, (newVal) => {
  const unitMap = { 'CPU': '%', '内存': '%', '磁盘': '%', '错误率': '%', '延迟': 'ms', 'QPS': '' }
  ruleForm.unit = unitMap[newVal] || ''
})

// ===== 图表引用 =====
const qpsChartEl = ref(null)
const latencyChartEl = ref(null)
const resChartEl = ref(null)
const bizChartEl = ref(null)
const radarEl = ref(null)
const loadEl = ref(null)

let qpsChart = null
let latencyChart = null
let resChart = null
let bizChart = null
let radarChart = null
let chart = null // 原有 load chart

// ===== 演示占位：Mock 数据生成（后端待提供时序指标查询端点 /monitor/timeseries） =====
function generateTimeSeries(points, base, variance, trend = 0) {
  const data = []
  let val = base
  for (let i = 0; i < points; i++) {
    val = base + (Math.random() - 0.5) * variance * 2 + trend * (i / points - 0.5)
    val = Math.max(0, val)
    data.push(parseFloat(val.toFixed(2)))
  }
  return data
}

function generateTimeLabels(points, intervalSec = 60) {
  const labels = []
  const now = Date.now()
  for (let i = points - 1; i >= 0; i--) {
    const t = new Date(now - i * intervalSec * 1000)
    labels.push(t.toTimeString().slice(0, 5))
  }
  return labels
}

function getPointsForRange() {
  const map = { '1h': 60, '6h': 72, '24h': 96 }
  return map[trendRange.value] || 60
}

function getIntervalForRange() {
  const map = { '1h': 60, '6h': 300, '24h': 900 }
  return map[trendRange.value] || 60
}

// ===== QPS + 错误率双Y轴图 =====
function renderQpsChart() {
  if (!qpsChartEl.value) return
  if (!qpsChart) qpsChart = echarts.init(qpsChartEl.value)
  const points = getPointsForRange()
  const interval = getIntervalForRange()
  const labels = generateTimeLabels(points, interval)
  const qpsData = generateTimeSeries(points, qualityMetrics.qps, qualityMetrics.qps * 0.3, 10)
  const errData = generateTimeSeries(points, qualityMetrics.errorRate, 0.3, 0.1)

  qpsChart.setOption({
    tooltip: { trigger: 'axis', axisPointer: { type: 'cross' } },
    legend: { data: ['QPS', '错误率'], top: 0, textStyle: { color: '#94a3b8' } },
    grid: { left: 50, right: 50, top: 36, bottom: 30 },
    xAxis: { type: 'category', data: labels, axisLine: { lineStyle: { color: '#334155' } }, axisLabel: { color: '#94a3b8', fontSize: 11 } },
    yAxis: [
      { type: 'value', name: 'QPS', position: 'left', axisLine: { lineStyle: { color: '#6366f1' } }, splitLine: { lineStyle: { color: 'rgba(148,163,184,0.1)' } }, axisLabel: { color: '#94a3b8' } },
      { type: 'value', name: '错误率(%)', position: 'right', min: 0, axisLine: { lineStyle: { color: '#f56c6c' } }, splitLine: { show: false }, axisLabel: { color: '#94a3b8', formatter: '{value}%' } }
    ],
    series: [
      {
        name: 'QPS', type: 'line', smooth: true, data: qpsData, yAxisIndex: 0,
        itemStyle: { color: '#6366f1' },
        lineStyle: { width: 2 },
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: 'rgba(99,102,241,0.3)' },
            { offset: 1, color: 'rgba(99,102,241,0)' }
          ])
        }
      },
      {
        name: '错误率', type: 'line', smooth: true, data: errData, yAxisIndex: 1,
        itemStyle: { color: '#f56c6c' },
        lineStyle: { width: 2, type: 'dashed' }
      }
    ]
  })
}

// ===== 延迟分布图 =====
function renderLatencyChart() {
  if (!latencyChartEl.value) return
  if (!latencyChart) latencyChart = echarts.init(latencyChartEl.value)
  const points = getPointsForRange()
  const interval = getIntervalForRange()
  const labels = generateTimeLabels(points, interval)
  const p50Data = generateTimeSeries(points, qualityMetrics.p50, 8, 3)
  const p95Data = generateTimeSeries(points, qualityMetrics.p95, 30, 10)
  const p99Data = generateTimeSeries(points, qualityMetrics.p99, 80, 20)

  latencyChart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { data: ['P50', 'P95', 'P99'], top: 0, textStyle: { color: '#94a3b8' } },
    grid: { left: 50, right: 20, top: 36, bottom: 30 },
    xAxis: { type: 'category', data: labels, axisLine: { lineStyle: { color: '#334155' } }, axisLabel: { color: '#94a3b8', fontSize: 11 } },
    yAxis: { type: 'value', name: 'ms', axisLine: { lineStyle: { color: '#334155' } }, splitLine: { lineStyle: { color: 'rgba(148,163,184,0.1)' } }, axisLabel: { color: '#94a3b8' } },
    series: [
      { name: 'P50', type: 'line', smooth: true, data: p50Data, itemStyle: { color: '#10b981' }, lineStyle: { width: 2 } },
      { name: 'P95', type: 'line', smooth: true, data: p95Data, itemStyle: { color: '#f59e0b' }, lineStyle: { width: 2 } },
      { name: 'P99', type: 'line', smooth: true, data: p99Data, itemStyle: { color: '#ef4444' }, lineStyle: { width: 2 } }
    ]
  })
}

// ===== CPU + 内存趋势图 =====
function renderResChart() {
  if (!resChartEl.value) return
  if (!resChart) resChart = echarts.init(resChartEl.value)
  const points = getPointsForRange()
  const interval = getIntervalForRange()
  const labels = generateTimeLabels(points, interval)
  const cpuData = generateTimeSeries(points, systemMetrics.cpu, 10, 5)
  const memData = generateTimeSeries(points, systemMetrics.memory, 5, 3)

  resChart.setOption({
    tooltip: { trigger: 'axis', valueFormatter: v => v + '%' },
    legend: { data: ['CPU', '内存'], top: 0, textStyle: { color: '#94a3b8' } },
    grid: { left: 45, right: 20, top: 36, bottom: 30 },
    xAxis: { type: 'category', data: labels, axisLine: { lineStyle: { color: '#334155' } }, axisLabel: { color: '#94a3b8', fontSize: 11 } },
    yAxis: { type: 'value', max: 100, axisLabel: { color: '#94a3b8', formatter: '{value}%' }, splitLine: { lineStyle: { color: 'rgba(148,163,184,0.1)' } } },
    series: [
      {
        name: 'CPU', type: 'line', smooth: true, data: cpuData,
        itemStyle: { color: '#6366f1' },
        lineStyle: { width: 2 },
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: 'rgba(99,102,241,0.25)' },
            { offset: 1, color: 'rgba(99,102,241,0)' }
          ])
        }
      },
      {
        name: '内存', type: 'line', smooth: true, data: memData,
        itemStyle: { color: '#10b981' },
        lineStyle: { width: 2 },
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: 'rgba(16,185,129,0.25)' },
            { offset: 1, color: 'rgba(16,185,129,0)' }
          ])
        }
      }
    ]
  })
}

// ===== 业务量柱状图 =====
function renderBizChart() {
  if (!bizChartEl.value) return
  if (!bizChart) bizChart = echarts.init(bizChartEl.value)

  const days = []
  const now = new Date()
  for (let i = 6; i >= 0; i--) {
    const d = new Date(now.getTime() - i * 86400000)
    days.push((d.getMonth() + 1) + '/' + d.getDate())
  }

  bizChart.setOption({
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    legend: { data: ['对话数', '专家咨询', '工作流执行', '算子调用'], top: 0, textStyle: { color: '#94a3b8', fontSize: 11 } },
    grid: { left: 50, right: 20, top: 40, bottom: 30 },
    xAxis: { type: 'category', data: days, axisLine: { lineStyle: { color: '#334155' } }, axisLabel: { color: '#94a3b8', fontSize: 11 } },
    yAxis: { type: 'value', axisLabel: { color: '#94a3b8' }, splitLine: { lineStyle: { color: 'rgba(148,163,184,0.1)' } } },
    // 演示占位：业务量柱状图硬编码数据，后端待提供 /monitor/business/timeseries 端点
    series: [
      { name: '对话数', type: 'bar', data: [2800, 3100, 2950, 3400, 3200, 3600, 3562], itemStyle: { color: '#6366f1', borderRadius: [4, 4, 0, 0] }, barWidth: 12 },
      { name: '专家咨询', type: 'bar', data: [95, 110, 102, 118, 125, 132, 128], itemStyle: { color: '#10b981', borderRadius: [4, 4, 0, 0] }, barWidth: 12 },
      { name: '工作流执行', type: 'bar', data: [720, 800, 760, 850, 900, 880, 892], itemStyle: { color: '#f59e0b', borderRadius: [4, 4, 0, 0] }, barWidth: 12 },
      { name: '算子调用', type: 'bar', data: [10000, 11200, 10500, 12000, 11800, 12300, 12450], itemStyle: { color: '#8b5cf6', borderRadius: [4, 4, 0, 0] }, barWidth: 12 }
    ]
  })
}

// ===== 原有：系统负载图 =====
function renderChart() {
  if (!loadEl.value) return
  if (!chart) chart = echarts.init(loadEl.value)
  const data = logRows.value.slice(0, 15).reverse().map((r) => parseInt(r.time_ms))
  chart.setOption({
    tooltip: { trigger: 'axis' },
    grid: { left: 40, right: 16, top: 20, bottom: 24 },
    xAxis: { type: 'category', data: data.map((_, i) => '#' + (i + 1)) },
    yAxis: { type: 'value', name: 'ms' },
    series: [
      {
        type: 'line',
        smooth: true,
        data,
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: 'rgba(99,102,241,0.35)' },
            { offset: 1, color: 'rgba(99,102,241,0)' }
          ])
        },
        itemStyle: { color: '#6366f1' },
        lineStyle: { width: 2 }
      }
    ]
  })
}

// ===== 璇玑雷达图 =====
function renderRadar(scores) {
  if (!radarEl.value) return
  if (!radarChart) radarChart = echarts.init(radarEl.value)
  const dims = scores.length ? scores.map((s) => s[0]) : dimList.value
  const vals = scores.length ? scores.map((s) => Math.round(s[1] * 100)) : dims.map(() => 60)
  radarChart.setOption({
    tooltip: {},
    legend: { data: ['健康分'], bottom: 0, textStyle: { color: '#94a3b8' } },
    radar: {
      indicator: dims.map((d) => ({ name: d, max: 100 })),
      radius: '62%',
      axisName: { color: '#cbd5e1', fontSize: 11 },
      splitArea: { areaStyle: { color: ['rgba(99,102,241,0.05)', 'rgba(99,102,241,0.10)'] } }
    },
    series: [
      {
        type: 'radar',
        name: '健康分',
        data: [{ value: vals, name: '健康分' }],
        areaStyle: { color: 'rgba(99,102,241,0.30)' },
        lineStyle: { color: '#6366f1', width: 2 },
        itemStyle: { color: '#6366f1' }
      }
    ]
  })
}

// ===== 数据刷新逻辑 =====
// 演示占位：模拟指标波动（后端待提供实时指标推送）
function updateMockMetrics() {
  // 系统资源小幅波动
  systemMetrics.cpu = Math.max(10, Math.min(95, systemMetrics.cpu + (Math.random() - 0.5) * 8))
  systemMetrics.memory = Math.max(30, Math.min(92, systemMetrics.memory + (Math.random() - 0.5) * 3))
  systemMetrics.memUsed = parseFloat((systemMetrics.memTotal * systemMetrics.memory / 100).toFixed(1))
  systemMetrics.disks.forEach(d => {
    d.usage = Math.max(20, Math.min(95, d.usage + (Math.random() - 0.5) * 2))
  })
  systemMetrics.netUpload = parseFloat(Math.max(0.1, systemMetrics.netUpload + (Math.random() - 0.5) * 1).toFixed(1))
  systemMetrics.netDownload = parseFloat(Math.max(0.5, systemMetrics.netDownload + (Math.random() - 0.5) * 5).toFixed(1))

  // 服务质量波动
  qualityMetrics.qps = parseFloat(Math.max(10, qualityMetrics.qps + (Math.random() - 0.5) * 30).toFixed(1))
  qualityMetrics.qpsTrend = parseFloat(((Math.random() - 0.3) * 10).toFixed(1))
  qualityMetrics.errorRate = parseFloat(Math.max(0.01, Math.min(8, qualityMetrics.errorRate + (Math.random() - 0.5) * 0.3)).toFixed(2))
  qualityMetrics.errorCount = Math.floor(qualityMetrics.qps * 3600 * qualityMetrics.errorRate / 100)
  qualityMetrics.avgLatency = Math.max(10, Math.floor(qualityMetrics.avgLatency + (Math.random() - 0.5) * 10))
  qualityMetrics.p50 = Math.max(5, Math.floor(qualityMetrics.p50 + (Math.random() - 0.5) * 5))
  qualityMetrics.p95 = Math.max(50, Math.floor(qualityMetrics.p95 + (Math.random() - 0.5) * 20))
  qualityMetrics.p99 = Math.max(100, Math.floor(qualityMetrics.p99 + (Math.random() - 0.5) * 50))
  qualityMetrics.onlineUsers = Math.max(100, Math.floor(qualityMetrics.onlineUsers + (Math.random() - 0.5) * 50))
  qualityMetrics.peakUsers = Math.max(qualityMetrics.onlineUsers, qualityMetrics.peakUsers)
  qualityMetrics.activeConnections = Math.max(50, Math.floor(qualityMetrics.activeConnections + (Math.random() - 0.5) * 30))

  // 业务指标缓慢增长
  businessMetrics.conversations += Math.floor(Math.random() * 5)
  businessMetrics.expertConsultations += Math.random() > 0.7 ? 1 : 0
  businessMetrics.workflowRuns += Math.floor(Math.random() * 3)
  businessMetrics.operatorCalls += Math.floor(Math.random() * 20)

  // 更新时间
  lastUpdateTime.value = new Date().toTimeString().slice(0, 8)

  // 闪烁提示
  triggerFlash()
}

function triggerFlash() {
  dataFlash.value = true
  if (flashTimer) clearTimeout(flashTimer)
  flashTimer = setTimeout(() => {
    dataFlash.value = false
  }, 300)
}

// 演示占位：模拟服务节点状态（后端待提供 /monitor/nodes 服务发现端点）
function refreshServiceNodes() {
  serviceNodes.value = [
    { name: 'API 网关', status: 'up', version: 'v2.1.0', cpu: 45, memory: 62, qps: 128, latency: 42, uptime: '15天 6小时' },
    { name: '认证服务', status: 'up', version: 'v1.8.3', cpu: 23, memory: 48, qps: 56, latency: 18, uptime: '30天 12小时' },
    { name: '对话服务', status: 'up', version: 'v3.0.1', cpu: 72, memory: 78, qps: 89, latency: 95, uptime: '7天 3小时' },
    { name: '知识图谱', status: 'warning', version: 'v2.4.0', cpu: 85, memory: 82, qps: 34, latency: 120, uptime: '10天 8小时' },
    { name: '工作流引擎', status: 'up', version: 'v1.5.2', cpu: 38, memory: 55, qps: 45, latency: 68, uptime: '20天 5小时' },
    { name: '算子执行器', status: 'up', version: 'v2.2.1', cpu: 52, memory: 60, qps: 72, latency: 55, uptime: '12天 10小时' },
    { name: '文件存储', status: 'up', version: 'v1.3.0', cpu: 15, memory: 35, qps: 28, latency: 22, uptime: '45天 2小时' },
    { name: '消息队列', status: 'down', version: 'v2.0.0', cpu: 0, memory: 0, qps: 0, latency: '-', uptime: '已离线' }
  ]
  // 小幅波动
  serviceNodes.value.forEach(n => {
    if (n.status !== 'down') {
      n.cpu = Math.max(5, Math.min(99, n.cpu + Math.floor((Math.random() - 0.5) * 8)))
      n.memory = Math.max(10, Math.min(99, n.memory + Math.floor((Math.random() - 0.5) * 4)))
      n.qps = Math.max(0, n.qps + Math.floor((Math.random() - 0.5) * 10))
    }
  })
}

async function loadMonitorData() {
  // 系统资源指标
  try {
    const m = await getMetricsDetail()
    if (m) {
      if (m.cpu != null) systemMetrics.cpu = m.cpu
      if (m.memory != null) systemMetrics.memory = m.memory
      if (m.memTotal != null) systemMetrics.memTotal = m.memTotal
      if (m.memUsed != null) systemMetrics.memUsed = m.memUsed
      if (Array.isArray(m.disks)) systemMetrics.disks = m.disks
      if (m.netUpload != null) systemMetrics.netUpload = m.netUpload
      if (m.netDownload != null) systemMetrics.netDownload = m.netDownload
    }
  } catch (e) { /* 保留演示占位数据 */ }

  // 服务质量指标
  try {
    const q = await getMonitorQuality()
    if (q) Object.assign(qualityMetrics, q)
  } catch (e) { /* 保留演示占位数据 */ }

  // 业务指标
  try {
    const b = await getMonitorBusiness()
    if (b) Object.assign(businessMetrics, b)
  } catch (e) { /* 保留演示占位数据 */ }

  // 告警统计
  try {
    const a = await getAlertsSummary()
    if (a) Object.assign(alertMetrics, a)
  } catch (e) { /* 保留演示占位数据 */ }

  // 服务节点
  try {
    const nodes = await getMonitorNodes()
    if (Array.isArray(nodes) && nodes.length > 0) {
      serviceNodes.value = nodes
    }
  } catch (e) { /* 保留演示占位数据 */ }

  // 告警规则
  try {
    const rules = await getAlertRules()
    if (Array.isArray(rules) && rules.length > 0) {
      alertRules.value = rules
    }
  } catch (e) { /* 保留演示占位数据 */ }
}

async function loadAll() {
  loading.value = true
  try {
    let st = null
    let logs = []
    let plg = []
    try {
      const results = await Promise.all([
        getFullStatus().catch(() => getStatus()),
        getLogs().catch(() => []),
        getPlugins().catch(() => [])
      ])
      st = results[0]
      logs = results[1]
      plg = results[2]
    } catch (e) {
      console.warn('监控接口加载失败，使用空数据:', e)
    }
    
    if (st && st.success !== undefined && st.data !== undefined) {
      st = st.data
    }
    if (plg && plg.success !== undefined && plg.data !== undefined) {
      plg = plg.data
    }
    if (!Array.isArray(logs) && logs && logs.success !== undefined && logs.data !== undefined) {
      logs = logs.data
    }
    if (!Array.isArray(logs)) {
      logs = []
    }
    
    const s = st || {}
    const plgArr = Array.isArray(plg) ? plg : (plg?.items || [])
    pluginCount.value = plgArr.length
    
    // 原有 KPI（保留兼容，现有已迁移到新的分组中）
    kpis.value = [
      { label: '系统状态', value: s.status === 'running' ? '运行中' : s.status || '运行中', icon: 'CircleCheck', ok: true },
      { label: '算子数量', value: s.operators_count ?? 8, icon: 'Cpu' },
      { label: '执行次数', value: s.executions_count ?? logs.length ?? 15, icon: 'VideoPlay' },
      { label: '插件数量', value: pluginCount.value, icon: 'Connection' }
    ]
    comps.value = [
      { name: 'WASM 运行时', status: 'up', val: 'active' },
      { name: 'AI 智能体', status: 'up', val: 'online' },
      { name: '知识图谱', status: 'up', val: ((s.graph && s.graph.nodes) ?? 23) + ' 节点' },
      { name: '插件总线', status: pluginCount.value ? 'up' : 'down', val: pluginCount.value + ' 个' },
      { name: '数据库', status: 'up', val: 'connected' },
      { name: '消息队列', status: 'up', val: 'ready' }
    ]
    
    // 演示占位：日志为空时使用模拟日志兜底（后端 /api/logs 正常返回时不触发）
    const safeLogs = logs.length > 0 ? logs : generateMockMonitorLogs()
    logRows.value = safeLogs.slice(0, 50).map((l) => ({
      time: fmt(l.timestamp),
      flow: (l.workflow || []).join(' → ') || '—',
      status: l.success === false ? '失败' : '成功',
      time_ms: (l.execution_time_ms || 100) + ' ms',
      dims: `${l.input_dim || 3}→${l.output_dim || 7}`
    }))
    
    // 加载监控域真实数据（失败则保留演示占位）
    await loadMonitorData().catch(() => {})
    
    // 更新 mock 数据 & 服务节点
    updateMockMetrics()
    refreshServiceNodes()
    
    // 渲染所有图表
    await nextTick()
    renderAllCharts()
  } catch (e) {
    console.warn('监控加载失败', e)
  } finally {
    loading.value = false
  }
}

function renderAllCharts() {
  renderQpsChart()
  renderLatencyChart()
  renderResChart()
  renderBizChart()
  if (loadEl.value) renderChart()
  if (radarEl.value && dimList.value.length) renderRadar([])
}

// ===== 实时刷新机制 =====
function startAutoRefresh() {
  stopAutoRefresh()
  refreshCountdown.value = refreshInterval
  refreshTimer = setInterval(() => {
    refreshCountdown.value--
    if (refreshCountdown.value <= 0) {
      refreshCountdown.value = refreshInterval
      doRefresh()
    }
  }, 1000)
}

function stopAutoRefresh() {
  if (refreshTimer) {
    clearInterval(refreshTimer)
    refreshTimer = null
  }
}

function doRefresh() {
  updateMockMetrics()
  refreshServiceNodes()
  // 轻量刷新：只更新图表数据，不重新 init
  if (qpsChart) renderQpsChart()
  if (latencyChart) renderLatencyChart()
  if (resChart) renderResChart()
}

function manualRefresh() {
  refreshCountdown.value = refreshInterval
  loadAll()
}

function onAutoRefreshChange(val) {
  if (val) {
    startAutoRefresh()
    ElMessage.success('自动刷新已开启')
  } else {
    stopAutoRefresh()
    ElMessage.info('自动刷新已暂停')
  }
}

function onTrendRangeChange() {
  renderQpsChart()
  renderLatencyChart()
  renderResChart()
}

// ===== 原有：辅助函数 & Mock =====
const kpis = ref([])
const comps = ref([])
const logRows = ref([])
const pluginCount = ref(0)

function fmt(ts) {
  if (!ts) return '—'
  const d = new Date(ts)
  return isNaN(d) ? String(ts) : d.toLocaleString('zh-CN', { hour12: false })
}

// 演示占位：生成模拟执行日志（后端 /api/logs 正常返回时不使用）
function generateMockMonitorLogs() {
  const now = Date.now()
  const workflows = [
    ['需求采集', '归一化 IR', '璇玑验证网关'],
    ['知识图谱算子', 'PageRank', '社区发现'],
    ['AI 对话', '意图识别', '算子匹配'],
    ['工作流编排', '算子执行', '状态监控']
  ]
  return Array.from({ length: 10 }, (_, i) => ({
    timestamp: new Date(now - i * 200000).toISOString(),
    workflow: workflows[i % workflows.length],
    success: Math.random() > 0.1,
    execution_time_ms: 50 + Math.floor(Math.random() * 400),
    input_dim: 2 + Math.floor(Math.random() * 4),
    output_dim: 5 + Math.floor(Math.random() * 8)
  }))
}

// ===== 璇玑治理逻辑（保留原有）=====
const flowJson = ref('')
const governing = ref(false)
const governed = ref(false)
const gateApproved = ref(false)
const mox = ref('—')
const adopted = ref([])
const dimList = ref([])
const bizLeague = ref([])
const devLeague = ref([])

async function loadMoxHealth() {
  const h = await moxHealth().catch(() => null)
  if (!h) return
  dimList.value = h.dimensions || []
  bizLeague.value = h.business_league || []
  devLeague.value = h.dev_league || []
  mox.value = 'algo-verification-supreme'
}

function onFlowFile(file) {
  const reader = new FileReader()
  reader.onload = () => {
    flowJson.value = String(reader.result || '')
  }
  reader.readAsText(file.raw)
}

async function runGovernance() {
  let flow
  try {
    flow = flowJson.value.trim() ? JSON.parse(flowJson.value) : { nodes: [], edges: [] }
  } catch (e) {
    ElMessage.error('流程图 JSON 解析失败：' + e.message)
    return
  }
  governing.value = true
  try {
    const report = await moxOptimize(flow)
    governed.value = true
    gateApproved.value = !!(report.gate && report.gate.approved)
    mox.value = report.algo && report.algo.passed ? '通过' : '未通过'
    adopted.value = (report.adopted_suggestions || []).map((s) => ({
      dimension: s.dimension || (s.dims && s.dims[0]) || '—',
      summary: s.summary || s.text || JSON.stringify(s)
    }))
    renderRadar(report.expert_scores || [])
  } catch (e) {
    ElMessage.error('治理失败：' + e.message)
  } finally {
    governing.value = false
  }
}

// ===== resize =====
function resize() {
  chart && chart.resize()
  radarChart && radarChart.resize()
  qpsChart && qpsChart.resize()
  latencyChart && latencyChart.resize()
  resChart && resChart.resize()
  bizChart && bizChart.resize()
}
window.addEventListener('resize', resize)

// ===== 生命周期 =====
onMounted(async () => {
  await nextTick()
  loadAll()
  loadMoxHealth()
  if (autoRefresh.value) {
    startAutoRefresh()
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', resize)
  stopAutoRefresh()
  if (flashTimer) clearTimeout(flashTimer)
  chart && chart.dispose()
  radarChart && radarChart.dispose()
  qpsChart && qpsChart.dispose()
  latencyChart && latencyChart.dispose()
  resChart && resChart.dispose()
  bizChart && bizChart.dispose()
})

// ===== 璇玑：以项目为核心的联动（保留原有）=====
{
  const { onChange: _onProjectChange, ensureProjectContext: _ensureProject } = useProject()
  let _offPj = null
  let _loaded = false
  onMounted(async () => {
    _offPj = _onProjectChange(async () => { loadAll() })
    await _ensureProject().catch(() => {})
    if (!_loaded) {
      _loaded = true
      loadAll()
    }
  })
  const _ob$ = onBeforeUnmount == null ? null : onBeforeUnmount(() => { _offPj && _offPj() })
  if (typeof onBeforeUnmount === 'undefined') {
    // 不操作：Vue 路由离开时组件 destroy，本作用域已销毁
  }
}
</script>

<style scoped>
.monitor-page {
  display: flex;
  flex-direction: column;
  gap: 0;
}

/* ===== 页头刷新控制 ===== */
.page-header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}
.refresh-info {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 13px;
  color: var(--text-3);
}
.countdown {
  font-weight: 700;
  color: var(--accent, #6366f1);
  font-size: 14px;
  min-width: 20px;
  text-align: center;
  transition: transform 0.2s;
}
.countdown.flash {
  animation: pulse 0.3s ease;
}
.refresh-label {
  font-size: 12px;
}

/* ===== 区块 ===== */
.section-block {
  margin-bottom: 18px;
}
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.section-title-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  margin: 0;
  color: var(--text-1);
}
.bar-icon {
  display: inline-block;
  width: 4px;
  height: 16px;
  border-radius: 2px;
}
.bar-icon.sys { background: #6366f1; }
.bar-icon.qos { background: #10b981; }
.bar-icon.biz { background: #f59e0b; }
.bar-icon.alert { background: #ef4444; }
.bar-icon.chart { background: #8b5cf6; }
.bar-icon.health { background: #06b6d4; }
.bar-icon.rule { background: #ec4899; }

.section-hint {
  font-size: 12px;
  color: var(--text-3);
}

/* ===== KPI 卡片 ===== */
.kpi {
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  transition: box-shadow 0.3s ease;
}
.kpi-flash {
  animation: kpiFlash 0.3s ease;
}
@keyframes kpiFlash {
  0% { box-shadow: 0 0 0 0 rgba(99, 102, 241, 0.4); }
  50% { box-shadow: 0 0 0 4px rgba(99, 102, 241, 0.2); }
  100% { box-shadow: 0 0 0 0 rgba(99, 102, 241, 0); }
}
@keyframes pulse {
  0% { transform: scale(1); }
  50% { transform: scale(1.3); }
  100% { transform: scale(1); }
}
.kpi-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.kpi-icon {
  color: var(--text-3);
  font-size: 16px;
}
.kpi-value {
  font-size: 24px;
  font-weight: 700;
  line-height: 1.2;
}
.kpi-value.ok { color: var(--success, #10b981); }
.kpi-value.warn { color: var(--warning, #e6a23c); }
.kpi-value.bad { color: var(--danger, #f56c6c); }
.kpi-value.qos-val { color: var(--accent, #6366f1); }
.kpi-value.biz-val { color: #f59e0b; }
.kpi-value.alert-total { color: #8b5cf6; }
.kpi-value.alert-p0 { color: #ef4444; }
.kpi-value.alert-p1 { color: #f59e0b; }
.kpi-value.alert-p2 { color: #3b82f6; }

.kpi-value .unit {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-3);
  margin-left: 2px;
}
.kpi-label {
  font-size: 12px;
  color: var(--text-3);
}
.kpi-sub {
  font-size: 11px;
  color: var(--text-3);
}
.kpi-trend {
  font-size: 11px;
  display: flex;
  align-items: center;
  gap: 2px;
}
.kpi-trend.up { color: #ef4444; }
.kpi-trend.down { color: #10b981; }

.kpi.small .kpi-value {
  font-size: 20px;
}
.kpi.mini .kpi-value {
  font-size: 22px;
}
.kpi.mini .kpi-label {
  margin-bottom: 4px;
}

/* ===== 磁盘列表 ===== */
.disk-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 4px;
}
.disk-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}
.disk-name {
  width: 70px;
  color: var(--text-2);
  flex-shrink: 0;
}
.disk-bar {
  flex: 1;
  height: 6px;
  background: var(--bg-page);
  border-radius: 3px;
  overflow: hidden;
}
.disk-bar-inner {
  height: 100%;
  border-radius: 3px;
  transition: width 0.3s ease;
}
.disk-val {
  width: 36px;
  text-align: right;
  color: var(--text-3);
  font-weight: 600;
  flex-shrink: 0;
}

/* ===== 网络流量 ===== */
.net-values {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 4px;
}
.net-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
}
.net-dir {
  font-weight: 600;
}
.net-dir.up { color: #f59e0b; }
.net-dir.down { color: #10b981; }
.net-val {
  color: var(--text-2);
  font-weight: 600;
}

.mem-detail {
  font-size: 11px;
  color: var(--text-3);
}

/* ===== 图表 ===== */
.chart-row {
  gap: 16px;
  margin-bottom: 16px;
}
.chart-row:last-child {
  margin-bottom: 0;
}
.chart {
  width: 100%;
  height: 260px;
}
.chart.tall {
  height: 280px;
}
.chart-title {
  font-size: 13px;
  font-weight: 600;
  margin: 0 0 8px 0;
  color: var(--text-2);
}

/* ===== 服务健康状态 ===== */
.health-summary {
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 10px;
  margin-left: 6px;
  font-weight: 600;
}
.health-summary.up {
  background: rgba(16, 185, 129, 0.12);
  color: #10b981;
}
.health-summary.warn {
  background: rgba(245, 158, 11, 0.12);
  color: #f59e0b;
}
.health-summary.down {
  background: rgba(239, 68, 68, 0.12);
  color: #ef4444;
}

.svc-name {
  display: flex;
  align-items: center;
  gap: 8px;
}
.svc-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}
.svc-dot.up {
  background: var(--success, #10b981);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.18);
}
.svc-dot.warning {
  background: var(--warning, #e6a23c);
  box-shadow: 0 0 0 3px rgba(230, 162, 60, 0.18);
}
.svc-dot.down {
  background: var(--danger, #f56c6c);
  box-shadow: 0 0 0 3px rgba(245, 108, 108, 0.18);
  animation: blink 1.5s infinite;
}
@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.bad { color: var(--danger, #f56c6c) !important; font-weight: 600; }
.warn { color: var(--warning, #e6a23c) !important; font-weight: 600; }
.ok { color: var(--success, #10b981) !important; }

/* ===== 告警规则 ===== */
.rule-duration {
  font-size: 11px;
  color: var(--text-3);
  margin-left: 6px;
}
.channel-tag {
  margin-right: 4px;
  margin-bottom: 2px;
}
.rule-form .form-unit {
  margin-left: 8px;
  color: var(--text-3);
  font-size: 13px;
}

/* ===== 原有样式保留 ===== */
.mv {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.card-pad {
  padding: 18px 20px;
}
.comps {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
}
.comp {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: var(--bg-page);
  border-radius: 9px;
}
.comp-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}
.comp-dot.up {
  background: var(--success);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.18);
}
.comp-dot.down {
  background: var(--danger);
}
.comp-name {
  font-weight: 600;
  font-size: 13px;
  flex: 1;
}
.comp-val {
  font-size: 12px;
  color: var(--text-3);
}

/* ===== 璇玑治理面板 ===== */
.mox-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}
.mox-actions {
  display: flex;
  gap: 8px;
}
.mox-body {
  align-items: start;
}
.flow-input {
  margin-top: 10px;
}
.gov-result {
  padding: 4px 2px;
}
.gov-badges {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}
.badge.info {
  background: rgba(99, 102, 241, 0.15);
  color: #818cf8;
}
.sub {
  font-size: 14px;
  font-weight: 600;
  margin: 6px 0 8px;
  color: var(--text-2);
}
.suggest-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.suggest-list li {
  background: var(--bg-page);
  border-radius: 8px;
  padding: 8px 10px;
  font-size: 13px;
  color: var(--text-2);
}
.suggest-list b {
  color: var(--accent, #6366f1);
  margin-right: 6px;
}

/* ===== 响应式 ===== */
@media (max-width: 1400px) {
  .grid-5 {
    grid-template-columns: repeat(3, 1fr) !important;
  }
}
@media (max-width: 1024px) {
  .grid-4, .grid-5 {
    grid-template-columns: repeat(2, 1fr) !important;
  }
  .grid-2 {
    grid-template-columns: 1fr !important;
  }
}
</style>
