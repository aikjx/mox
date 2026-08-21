<template>
  <div>
    <div class="admin-card">
      <div class="admin-table-toolbar">
        <div>
          <h3 class="admin-page-title" style="margin:0">LLM用量统计</h3>
          <p class="subtitle">查看大模型调用量、Token消耗和费用统计</p>
        </div>
        <div class="time-selector">
          <el-radio-group v-model="timeRange" @change="loadData">
            <el-radio-button value="today">今日</el-radio-button>
            <el-radio-button value="7days">近7天</el-radio-button>
            <el-radio-button value="30days">近30天</el-radio-button>
            <el-radio-button value="custom">自定义</el-radio-button>
          </el-radio-group>
          <el-date-picker
            v-if="timeRange === 'custom'"
            v-model="customRange"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
            value-format="YYYY-MM-DD"
            style="margin-left: 10px"
          />
        </div>
      </div>

      <el-row :gutter="16" class="summary-row">
        <el-col :xs="12" :sm="6" :md="6">
          <div class="summary-card">
            <div class="summary-icon blue">
              <el-icon :size="24"><Cpu /></el-icon>
            </div>
            <div class="summary-info">
              <div class="summary-value">{{ summary.totalTokens }}</div>
              <div class="summary-label">总Token消耗</div>
            </div>
          </div>
        </el-col>
        <el-col :xs="12" :sm="6" :md="6">
          <div class="summary-card">
            <div class="summary-icon green">
              <el-icon :size="24"><Document /></el-icon>
            </div>
            <div class="summary-info">
              <div class="summary-value">{{ summary.requestCount }}</div>
              <div class="summary-label">请求总数</div>
            </div>
          </div>
        </el-col>
        <el-col :xs="12" :sm="6" :md="6">
          <div class="summary-card">
            <div class="summary-icon orange">
              <el-icon :size="24"><Wallet /></el-icon>
            </div>
            <div class="summary-info">
              <div class="summary-value">{{ summary.totalCost }}</div>
              <div class="summary-label">总费用 (USD)</div>
            </div>
          </div>
        </el-col>
        <el-col :xs="12" :sm="6" :md="6">
          <div class="summary-card">
            <div class="summary-icon purple">
              <el-icon :size="24"><Timer /></el-icon>
            </div>
            <div class="summary-info">
              <div class="summary-value">{{ summary.avgLatency }}</div>
              <div class="summary-label">平均响应 (ms)</div>
            </div>
          </div>
        </el-col>
      </el-row>
    </div>

    <el-row :gutter="16">
      <el-col :xs="24" :md="14">
        <div class="admin-card">
          <h3 class="admin-page-title">Token消耗趋势</h3>
          <div class="chart-container">
            <div class="area-chart">
              <svg viewBox="0 0 600 200" class="chart-svg">
                <defs>
                  <linearGradient id="gradBlue" x1="0%" y1="0%" x2="0%" y2="100%">
                    <stop offset="0%" stop-color="#409eff" stop-opacity="0.4" />
                    <stop offset="100%" stop-color="#409eff" stop-opacity="0" />
                  </linearGradient>
                  <linearGradient id="gradGreen" x1="0%" y1="0%" x2="0%" y2="100%">
                    <stop offset="0%" stop-color="#67c23a" stop-opacity="0.4" />
                    <stop offset="100%" stop-color="#67c23a" stop-opacity="0" />
                  </linearGradient>
                </defs>
                <line v-for="i in 5" :key="'h'+i" x1="40" :y1="i * 40" x2="590" :y2="i * 40" stroke="#ebeef5" stroke-dasharray="4" />
                <g v-for="(item, idx) in tokenData" :key="'g'+idx">
                  <text :x="50 + idx * 55" y="195" fill="#909399" font-size="10">{{ item.label }}</text>
                </g>
                <path
                  :d="buildAreaPath(tokenData.map(d => d.input), 40, 180)"
                  fill="url(#gradBlue)"
                />
                <path
                  :d="buildLinePath(tokenData.map(d => d.input), 40, 180)"
                  fill="none"
                  stroke="#409eff"
                  stroke-width="2"
                />
                <path
                  :d="buildAreaPath(tokenData.map(d => d.output), 40, 180)"
                  fill="url(#gradGreen)"
                />
                <path
                  :d="buildLinePath(tokenData.map(d => d.output), 40, 180)"
                  fill="none"
                  stroke="#67c23a"
                  stroke-width="2"
                />
              </svg>
            </div>
            <div class="chart-legend">
              <span class="legend-item"><span class="legend-dot blue"></span>输入Token</span>
              <span class="legend-item"><span class="legend-dot green"></span>输出Token</span>
            </div>
          </div>
        </div>
      </el-col>

      <el-col :xs="24" :md="10">
        <div class="admin-card">
          <h3 class="admin-page-title">按供应商请求分布</h3>
          <div class="provider-stats">
            <div v-for="p in providerStats" :key="p.name" class="provider-stat-item">
              <div class="stat-header">
                <span class="stat-name">{{ p.name }}</span>
                <span class="stat-value">{{ p.requests.toLocaleString() }} 次</span>
              </div>
              <el-progress
                :percentage="p.percent"
                :color="p.color"
                :stroke-width="14"
                :text-inside="true"
              />
              <div class="stat-detail">
                <span>Token: {{ (p.tokens / 1000).toFixed(1) }}K</span>
                <span>费用: ${{ p.cost.toFixed(2) }}</span>
                <span>平均: {{ p.avgTokens }} tokens/次</span>
              </div>
            </div>
          </div>
        </div>
      </el-col>
    </el-row>

    <el-row :gutter="16">
      <el-col :xs="24" :md="12">
        <div class="admin-card">
          <h3 class="admin-page-title">热门模型排行</h3>
          <el-table :data="topModels" stripe>
            <el-table-column label="排名" width="70" align="center">
              <template #default="{ $index }">
                <span :class="['rank-badge', $index < 3 ? `rank-${$index + 1}` : '']">
                  {{ $index + 1 }}
                </span>
              </template>
            </el-table-column>
            <el-table-column prop="model" label="模型" />
            <el-table-column prop="provider" label="供应商" />
            <el-table-column prop="requests" label="调用次数" sortable />
            <el-table-column prop="cost" label="费用 (USD)" sortable />
          </el-table>
        </div>
      </el-col>

      <el-col :xs="24" :md="12">
        <div class="admin-card">
          <h3 class="admin-page-title">Token使用分布</h3>
          <div class="donut-chart">
            <el-progress
              v-for="p in providerStats"
              :key="'d'+p.name"
              type="dashboard"
              :percentage="p.percent"
              :color="p.color"
              :width="140"
              :stroke-width="12"
            >
              <template #default>
                <div class="donut-label">
                  <div class="donut-name">{{ p.name }}</div>
                  <div class="donut-percent">{{ p.percent }}%</div>
                </div>
              </template>
            </el-progress>
          </div>
        </div>
      </el-col>
    </el-row>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import { adminApi } from '@/api/index'
import { Cpu, Document, Wallet, Timer } from '@element-plus/icons-vue'

const timeRange = ref('7days')
const customRange = ref([])

const summary = reactive({
  totalTokens: '2,847,392',
  requestCount: '18,456',
  totalCost: '1,247.83',
  avgLatency: '342'
})

const tokenData = ref([
  { label: '周一', input: 65, output: 40 },
  { label: '周二', input: 78, output: 52 },
  { label: '周三', input: 52, output: 35 },
  { label: '周四', input: 88, output: 65 },
  { label: '周五', input: 95, output: 72 },
  { label: '周六', input: 40, output: 25 },
  { label: '周日', input: 35, output: 22 }
])

const providerStats = ref([
  { name: 'OpenAI', requests: 8456, tokens: 1245000, cost: 547.83, avgTokens: 147, percent: 45, color: '#409eff' },
  { name: '阿里云百炼', requests: 5234, tokens: 785000, cost: 324.50, avgTokens: 150, percent: 28, color: '#67c23a' },
  { name: '本地模型', requests: 2345, tokens: 456000, cost: 89.20, avgTokens: 194, percent: 13, color: '#e6a23c' },
  { name: 'MiniMax', requests: 1567, tokens: 234000, cost: 156.30, avgTokens: 149, percent: 8, color: '#909399' },
  { name: '百度千帆', requests: 854, tokens: 127000, cost: 130.00, avgTokens: 149, percent: 6, color: '#f56c6c' }
])

const topModels = ref([
  { model: 'gpt-4o', provider: 'OpenAI', requests: 4523, cost: 320.50 },
  { model: 'qwen-max', provider: '阿里云百炼', requests: 2834, cost: 189.20 },
  { model: 'gpt-4o-mini', provider: 'OpenAI', requests: 2345, cost: 98.30 },
  { model: 'qwen2.5:72b', provider: '本地模型', requests: 1234, cost: 45.00 },
  { model: 'abab-6.5s', provider: 'MiniMax', requests: 987, cost: 87.60 },
  { model: 'ernie-4.0-turbo', provider: '百度千帆', requests: 654, cost: 72.30 }
])

function buildLinePath(values, startX, baseY) {
  const max = 100
  const stepX = (590 - startX) / (values.length - 1)
  let d = ''
  values.forEach((v, i) => {
    const x = startX + i * stepX
    const y = baseY - (v / max) * (baseY - 20)
    d += (i === 0 ? 'M' : 'L') + x + ',' + y + ' '
  })
  return d
}

function buildAreaPath(values, startX, baseY) {
  const line = buildLinePath(values, startX, baseY)
  return line + `L${startX + (values.length - 1) * ((590 - startX) / (values.length - 1))},${baseY} L${startX},${baseY} Z`
}

async function loadData() {
  try {
    const params = { range: timeRange.value }
    if (timeRange.value === 'custom' && customRange.value?.length === 2) {
      params.startDate = customRange.value[0]
      params.endDate = customRange.value[1]
    }
    const data = await adminApi.getLlmUsage(params)
    if (data?.data) {
      if (data.data.summary) Object.assign(summary, data.data.summary)
      if (data.data.tokenData) tokenData.value = data.data.tokenData
      if (data.data.providerStats) providerStats.value = data.data.providerStats
      if (data.data.topModels) topModels.value = data.data.topModels
    }
  } catch (e) { /* use mock data */ }
}

onMounted(loadData)
</script>

<style scoped>
.subtitle { font-size: 13px; color: #909399; margin: 4px 0 0; }

.time-selector { display: flex; align-items: center; }

.summary-row { margin-bottom: 0; }

.summary-card {
  background: #fafbfc;
  border-radius: 8px;
  padding: 16px;
  display: flex;
  align-items: center;
  gap: 14px;
  border: 1px solid #ebeef5;
  transition: transform 0.2s, box-shadow 0.2s;
}

.summary-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

.summary-icon {
  width: 48px;
  height: 48px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
}

.summary-icon.blue { background: linear-gradient(135deg, #409eff, #66b1ff); }
.summary-icon.green { background: linear-gradient(135deg, #67c23a, #95d475); }
.summary-icon.orange { background: linear-gradient(135deg, #e6a23c, #f0c78a); }
.summary-icon.purple { background: linear-gradient(135deg, #8e44ad, #bb6bd9); }

.summary-value {
  font-size: 22px;
  font-weight: 700;
  color: #303133;
}

.summary-label {
  font-size: 13px;
  color: #909399;
}

.chart-container { padding: 10px 0; }

.chart-svg { width: 100%; height: 200px; }

.chart-legend {
  display: flex;
  gap: 20px;
  justify-content: center;
  margin-top: 10px;
  font-size: 13px;
  color: #606266;
}

.legend-item { display: flex; align-items: center; gap: 6px; }

.legend-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  display: inline-block;
}

.legend-dot.blue { background: #409eff; }
.legend-dot.green { background: #67c23a; }

.provider-stats {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.provider-stat-item {
  padding: 10px;
  background: #fafbfc;
  border-radius: 6px;
}

.stat-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 6px;
  font-size: 13px;
}

.stat-name { font-weight: 600; color: #303133; }
.stat-value { color: #409eff; font-weight: 600; }

.stat-detail {
  display: flex;
  gap: 16px;
  margin-top: 8px;
  font-size: 12px;
  color: #909399;
}

.rank-badge {
  display: inline-block;
  width: 24px;
  height: 24px;
  line-height: 24px;
  border-radius: 50%;
  background: #ebeef5;
  color: #606266;
  font-weight: 600;
}

.rank-badge.rank-1 { background: #ffd700; color: #fff; }
.rank-badge.rank-2 { background: #c0c4cc; color: #fff; }
.rank-badge.rank-3 { background: #cd7f32; color: #fff; }

.donut-chart {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  justify-content: space-around;
  padding: 10px 0;
}

.donut-label {
  text-align: center;
}

.donut-name {
  font-size: 12px;
  color: #606266;
}

.donut-percent {
  font-size: 16px;
  font-weight: 700;
  color: #303133;
}
</style>