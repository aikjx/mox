<template>
  <div>
    <el-row :gutter="16" class="kpi-row">
      <el-col :xs="12" :sm="12" :md="6" :lg="6">
        <div class="admin-stat-card stat-blue">
          <div class="stat-icon"><el-icon :size="28"><User /></el-icon></div>
          <div class="stat-content">
            <div class="stat-value">{{ kpi.activeUsers }}</div>
            <div class="stat-label">活跃用户</div>
          </div>
          <div class="stat-trend up">↑ 12.5%</div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="12" :md="6" :lg="6">
        <div class="admin-stat-card stat-green">
          <div class="stat-icon"><el-icon :size="28"><Collection /></el-icon></div>
          <div class="stat-content">
            <div class="stat-value">{{ kpi.totalKnowledgeBases }}</div>
            <div class="stat-label">知识库总数</div>
          </div>
          <div class="stat-trend up">↑ 3.2%</div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="12" :md="6" :lg="6">
        <div class="admin-stat-card stat-orange">
          <div class="stat-icon"><el-icon :size="28"><Cpu /></el-icon></div>
          <div class="stat-content">
            <div class="stat-value">{{ kpi.llmCallsToday }}</div>
            <div class="stat-label">今日LLM调用</div>
          </div>
          <div class="stat-trend up">↑ 28.7%</div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="12" :md="6" :lg="6">
        <div class="admin-stat-card stat-purple">
          <div class="stat-icon"><el-icon :size="28"><FolderOpened /></el-icon></div>
          <div class="stat-content">
            <div class="stat-value">{{ kpi.storageUsed }}</div>
            <div class="stat-label">已用存储</div>
          </div>
          <div class="stat-trend down">↓ 2.1%</div>
        </div>
      </el-col>
    </el-row>

    <el-row :gutter="16">
      <el-col :xs="24" :md="16">
        <div class="admin-card">
          <div class="card-header">
            <h3 class="admin-page-title" style="margin:0">系统状态概览</h3>
            <el-tag :type="statusTagType" effect="dark">{{ systemStatusText }}</el-tag>
          </div>
          <div class="status-grid">
            <div class="status-item">
              <span class="status-label">CPU 使用率</span>
              <el-progress :percentage="systemStatus.cpu" :stroke-width="10" :color="progressColor" />
              <span class="status-value">{{ systemStatus.cpu }}%</span>
            </div>
            <div class="status-item">
              <span class="status-label">内存使用率</span>
              <el-progress :percentage="systemStatus.memory" :stroke-width="10" :color="progressColor" />
              <span class="status-value">{{ systemStatus.memory }}%</span>
            </div>
            <div class="status-item">
              <span class="status-label">磁盘占用</span>
              <el-progress :percentage="systemStatus.disk" :stroke-width="10" :color="progressColor" />
              <span class="status-value">{{ systemStatus.disk }}%</span>
            </div>
            <div class="status-item">
              <span class="status-label">网络IO</span>
              <el-progress :percentage="systemStatus.network" :stroke-width="10" :color="progressColor" />
              <span class="status-value">{{ systemStatus.network }}%</span>
            </div>
          </div>
        </div>
      </el-col>

      <el-col :xs="24" :md="8">
        <div class="admin-card">
          <h3 class="admin-page-title">快捷操作</h3>
          <div class="quick-actions">
            <el-button type="primary" :icon="Plus" @click="$router.push('/users/list')">创建用户</el-button>
            <el-button type="success" :icon="FolderAdd" @click="$router.push('/knowledge/list')">新建知识库</el-button>
            <el-button type="warning" :icon="Setting" @click="$router.push('/system/general')">系统设置</el-button>
            <el-button type="info" :icon="Refresh" @click="refreshData">刷新数据</el-button>
          </div>
          <el-divider />
          <h4 class="section-title">在线模块</h4>
          <div class="module-list">
            <div v-for="mod in modules" :key="mod.name" class="module-item">
              <span class="module-dot" :class="mod.status"></span>
              <span class="module-name">{{ mod.name }}</span>
              <span class="module-status">{{ mod.status === 'online' ? '正常' : mod.status === 'warning' ? '警告' : '异常' }}</span>
            </div>
          </div>
        </div>
      </el-col>
    </el-row>

    <el-row :gutter="16">
      <el-col :xs="24" :md="14">
        <div class="admin-card">
          <div class="card-header">
            <h3 class="admin-page-title" style="margin:0">最近活动</h3>
            <el-link type="primary" @click="$router.push('/users/audit')">查看全部</el-link>
          </div>
          <el-timeline>
            <el-timeline-item
              v-for="(activity, index) in recentActivities"
              :key="index"
              :timestamp="activity.time"
              :type="activity.type"
              :icon="activity.icon"
            >
              <div class="activity-item">
                <span class="activity-user">{{ activity.user }}</span>
                <span class="activity-action">{{ activity.action }}</span>
                <span class="activity-target">{{ activity.target }}</span>
              </div>
            </el-timeline-item>
          </el-timeline>
        </div>
      </el-col>

      <el-col :xs="24" :md="10">
        <div class="admin-card">
          <h3 class="admin-page-title">LLM调用趋势</h3>
          <div class="chart-placeholder">
            <div class="bar-chart">
              <div v-for="(item, idx) in llmTrend" :key="idx" class="bar-item">
                <div class="bar-wrapper">
                  <div class="bar" :style="{ height: item.percent + '%' }" :class="item.type"></div>
                </div>
                <span class="bar-label">{{ item.label }}</span>
              </div>
            </div>
          </div>
          <div class="chart-legend">
            <span class="legend-item"><span class="legend-dot blue"></span>GPT调用</span>
            <span class="legend-item"><span class="legend-dot green"></span>本地模型</span>
          </div>
        </div>

        <div class="admin-card">
          <h3 class="admin-page-title">存储分布</h3>
          <div class="storage-dist">
            <el-progress
              v-for="item in storageDistribution"
              :key="item.name"
              :percentage="item.percent"
              :color="item.color"
              :stroke-width="14"
              :text-inside="true"
            >
              <span class="storage-label">{{ item.name }} ({{ item.size }})</span>
            </el-progress>
          </div>
        </div>
      </el-col>
    </el-row>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { adminApi } from '@/api/index'
import { Plus, FolderAdd, Refresh } from '@element-plus/icons-vue'

const kpi = ref({
  activeUsers: 1286,
  totalKnowledgeBases: 42,
  llmCallsToday: 8734,
  storageUsed: '2.4 TB'
})

const systemStatus = ref({
  cpu: 45,
  memory: 68,
  disk: 72,
  network: 30
})

const modules = ref([
  { name: '用户认证服务', status: 'online' },
  { name: 'LLM网关', status: 'online' },
  { name: '知识库检索', status: 'online' },
  { name: '文件存储服务', status: 'warning' },
  { name: '审计日志服务', status: 'online' }
])

const recentActivities = ref([
  { time: '2026-08-21 14:32', user: '张三', action: '创建了知识库', target: '产品文档库', type: 'primary', icon: 'Edit' },
  { time: '2026-08-21 13:15', user: '李四', action: '修改了用户权限', target: 'zhangsan', type: 'success', icon: 'Lock' },
  { time: '2026-08-21 12:08', user: 'admin', action: '添加了LLM供应商', target: 'MiniMax', type: 'warning', icon: 'Cpu' },
  { time: '2026-08-21 10:45', user: '王五', action: '删除了角色', target: '临时访客', type: 'danger', icon: 'Delete' },
  { time: '2026-08-21 09:20', user: '赵六', action: '登录系统', target: '-', type: 'info', icon: 'User' }
])

const llmTrend = ref([
  { label: '周一', percent: 65, type: 'blue' },
  { label: '周二', percent: 78, type: 'green' },
  { label: '周三', percent: 52, type: 'blue' },
  { label: '周四', percent: 88, type: 'green' },
  { label: '周五', percent: 95, type: 'blue' },
  { label: '周六', percent: 40, type: 'green' },
  { label: '周日', percent: 35, type: 'blue' }
])

const storageDistribution = ref([
  { name: '文档存储', size: '890 GB', percent: 37, color: '#409eff' },
  { name: '知识库索引', size: '520 GB', percent: 22, color: '#67c23a' },
  { name: '用户上传', size: '480 GB', percent: 20, color: '#e6a23c' },
  { name: '系统日志', size: '320 GB', percent: 13, color: '#f56c6c' },
  { name: '其他', size: '190 GB', percent: 8, color: '#909399' }
])

const systemStatusText = computed(() => {
  const avg = (systemStatus.value.cpu + systemStatus.value.memory + systemStatus.value.disk) / 3
  if (avg > 85) return '系统繁忙'
  if (avg > 70) return '注意'
  return '运行正常'
})

const statusTagType = computed(() => {
  const avg = (systemStatus.value.cpu + systemStatus.value.memory + systemStatus.value.disk) / 3
  if (avg > 85) return 'danger'
  if (avg > 70) return 'warning'
  return 'success'
})

const progressColor = computed(() => {
  return (percentage) => {
    if (percentage > 80) return '#f56c6c'
    if (percentage > 60) return '#e6a23c'
    return '#67c23a'
  }
})

async function refreshData() {
  ElMessage.success('数据已刷新')
}

onMounted(async () => {
  try {
    const data = await adminApi.getSystemInfo()
    if (data) {
      kpi.value.activeUsers = data.activeUsers || kpi.value.activeUsers
    }
  } catch (e) {
    console.log('使用模拟数据')
  }
})
</script>

<style scoped>
.kpi-row { margin-bottom: 16px; }

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.status-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.status-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.status-label {
  font-size: 13px;
  color: #606266;
  font-weight: 500;
}

.status-value {
  font-size: 12px;
  color: #909399;
  text-align: right;
}

.quick-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.section-title {
  font-size: 14px;
  color: #606266;
  margin: 0 0 10px;
}

.module-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.module-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.module-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #67c23a;
}

.module-dot.warning { background: #e6a23c; }
.module-dot.error { background: #f56c6c; }

.module-name { flex: 1; color: #303133; }
.module-status { color: #909399; font-size: 12px; }

.activity-item {
  display: flex;
  gap: 4px;
  font-size: 13px;
}

.activity-user {
  font-weight: 600;
  color: #303133;
}

.activity-action { color: #606266; }
.activity-target { color: #409eff; font-weight: 500; }

.chart-placeholder {
  height: 160px;
  display: flex;
  align-items: flex-end;
  justify-content: space-around;
  padding: 0 4px;
  margin-bottom: 12px;
}

.bar-chart {
  display: flex;
  width: 100%;
  justify-content: space-around;
  height: 100%;
  align-items: flex-end;
}

.bar-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: 1;
  height: 100%;
}

.bar-wrapper {
  flex: 1;
  width: 100%;
  display: flex;
  align-items: flex-end;
  justify-content: center;
}

.bar {
  width: 60%;
  min-height: 8px;
  border-radius: 4px 4px 0 0;
  transition: height 0.3s;
}

.bar.blue { background: linear-gradient(180deg, #409eff, #66b1ff); }
.bar.green { background: linear-gradient(180deg, #67c23a, #95d475); }

.bar-label {
  font-size: 11px;
  color: #909399;
  margin-top: 4px;
}

.chart-legend {
  display: flex;
  gap: 20px;
  font-size: 12px;
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

.storage-dist {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.storage-label {
  font-size: 12px;
  color: #fff;
  text-shadow: 0 1px 2px rgba(0,0,0,0.3);
}
</style>