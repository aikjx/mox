<template>
  <div>
    <el-row :gutter="16">
      <el-col :xs="24" :md="16">
        <div class="admin-card">
          <h3 class="admin-page-title">系统信息</h3>

          <div class="system-banner">
            <div class="banner-icon">
              <el-icon :size="40"><Monitor /></el-icon>
            </div>
            <div class="banner-info">
              <h2>{{ systemInfo.name }}</h2>
              <p>{{ systemInfo.description }}</p>
              <div class="banner-tags">
                <el-tag type="success" effect="dark">v{{ systemInfo.version }}</el-tag>
                <el-tag :type="systemInfo.status === 'running' ? 'success' : 'warning'">
                  {{ systemInfo.status === 'running' ? '运行中' : '维护中' }}
                </el-tag>
                <el-tag>构建 {{ systemInfo.build }}</el-tag>
              </div>
            </div>
            <div class="banner-actions">
              <el-button type="primary" :icon="Refresh" @click="refreshInfo">刷新信息</el-button>
              <el-button :icon="Download" @click="downloadDiagnostic">下载诊断</el-button>
            </div>
          </div>

          <el-descriptions :column="2" border>
            <el-descriptions-item label="系统名称">{{ systemInfo.name }}</el-descriptions-item>
            <el-descriptions-item label="版本号">
              <span class="version-badge">v{{ systemInfo.version }}</span>
            </el-descriptions-item>
            <el-descriptions-item label="发行日期">{{ systemInfo.releaseDate }}</el-descriptions-item>
            <el-descriptions-item label="构建编号">{{ systemInfo.build }}</el-descriptions-item>
            <el-descriptions-item label="运行时长">{{ systemInfo.uptime }}</el-descriptions-item>
            <el-descriptions-item label="服务器时间">{{ systemInfo.serverTime }}</el-descriptions-item>
            <el-descriptions-item label="Node版本">{{ systemInfo.nodeVersion }}</el-descriptions-item>
            <el-descriptions-item label="数据库版本">{{ systemInfo.dbVersion }}</el-descriptions-item>
            <el-descriptions-item label="操作系统" :span="2">{{ systemInfo.os }}</el-descriptions-item>
          </el-descriptions>

          <el-divider />

          <h4 class="section-title">已安装模块</h4>
          <div class="modules-grid">
            <div v-for="mod in installedModules" :key="mod.name" class="module-card">
              <div class="module-icon" :style="{ background: mod.color }">
                <el-icon :size="18"><component :is="mod.icon" /></el-icon>
              </div>
              <div class="module-info">
                <div class="module-name">{{ mod.name }}</div>
                <div class="module-version">v{{ mod.version }}</div>
              </div>
              <el-tag :type="mod.enabled ? 'success' : 'info'" size="small" effect="light">
                {{ mod.enabled ? '启用' : '禁用' }}
              </el-tag>
            </div>
          </div>
        </div>
      </el-col>

      <el-col :xs="24" :md="8">
        <div class="admin-card">
          <h3 class="admin-page-title">API端点</h3>
          <div class="api-list">
            <div v-for="api in apiEndpoints" :key="api.path" class="api-item">
              <span class="api-method" :class="api.method.toLowerCase()">{{ api.method }}</span>
              <code class="api-path">{{ api.path }}</code>
              <el-tooltip :content="api.desc" placement="right">
                <el-icon class="api-info"><InfoFilled /></el-icon>
              </el-tooltip>
            </div>
          </div>
          <el-button type="primary" plain size="small" @click="copyApiList" style="width:100%; margin-top:12px">
            复制API文档
          </el-button>
        </div>

        <div class="admin-card">
          <h3 class="admin-page-title">许可证信息</h3>
          <div class="license-info">
            <div class="license-key">
              <span class="label">授权类型</span>
              <el-tag type="danger" effect="dark">{{ license.type }}</el-tag>
            </div>
            <div class="license-key">
              <span class="label">有效期至</span>
              <span class="value">{{ license.expiry }}</span>
            </div>
            <div class="license-key">
              <span class="label">授权用户数</span>
              <span class="value">{{ license.users }} / {{ license.maxUsers }}</span>
            </div>
            <div class="license-key">
              <span class="label">授权节点</span>
              <span class="value">{{ license.nodes }}</span>
            </div>
          </div>
          <el-button type="primary" plain size="small" @click="renewLicense" style="width:100%; margin-top:12px">
            续期许可证
          </el-button>
        </div>

        <div class="admin-card">
          <h3 class="admin-page-title">技术支持</h3>
          <div class="support-info">
            <div class="support-item">
              <el-icon><Message /></el-icon>
              <span>support@infotopograph.com</span>
            </div>
            <div class="support-item">
              <el-icon><Link /></el-icon>
              <span>docs.infotopograph.com</span>
            </div>
            <div class="support-item">
              <el-icon><ChatDotRound /></el-icon>
              <span>在线客服 9:00-21:00</span>
            </div>
          </div>
        </div>
      </el-col>
    </el-row>

    <div class="admin-card">
      <h3 class="admin-page-title">更新日志</h3>
      <el-timeline>
        <el-timeline-item
          v-for="(log, idx) in changelog"
          :key="idx"
          :timestamp="log.date"
          :type="log.type"
        >
          <div class="changelog-item">
            <strong>v{{ log.version }}</strong>
            <div class="changelog-content">
              <ul>
                <li v-for="(item, i) in log.items" :key="i">{{ item }}</li>
              </ul>
            </div>
          </div>
        </el-timeline-item>
      </el-timeline>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { adminApi } from '@/api/index'
import { Monitor, Refresh, Download, InfoFilled, Message, Link, ChatDotRound } from '@element-plus/icons-vue'

const systemInfo = reactive({
  name: '璇玑 OUS',
  description: '企业级知识管理与AI智能平台',
  version: '3.0.0',
  build: '20260821',
  releaseDate: '2026-08-15',
  status: 'running',
  uptime: '12天 5小时 32分钟',
  serverTime: new Date().toLocaleString('zh-CN'),
  nodeVersion: 'v20.11.0',
  dbVersion: 'PostgreSQL 16.4',
  os: 'Ubuntu 22.04 LTS (Linux 5.15.0-76-generic x86_64)'
})

const installedModules = ref([
  { name: '用户认证', version: '3.0.0', enabled: true, icon: 'User', color: 'linear-gradient(135deg, #409eff, #66b1ff)' },
  { name: 'LLM网关', version: '2.8.1', enabled: true, icon: 'Cpu', color: 'linear-gradient(135deg, #67c23a, #95d475)' },
  { name: '知识库引擎', version: '2.5.0', enabled: true, icon: 'Collection', color: 'linear-gradient(135deg, #e6a23c, #f0c78a)' },
  { name: '向量检索', version: '1.9.2', enabled: true, icon: 'Search', color: 'linear-gradient(135deg, #8e44ad, #bb6bd9)' },
  { name: '文件存储', version: '2.1.0', enabled: true, icon: 'FolderOpened', color: 'linear-gradient(135deg, #00bcd4, #4dd0e1)' },
  { name: '审计日志', version: '1.5.0', enabled: true, icon: 'Document', color: 'linear-gradient(135deg, #ff9800, #ffb74d)' },
  { name: '数据导入', version: '1.2.0', enabled: false, icon: 'Upload', color: 'linear-gradient(135deg, #f56c6c, #f89898)' },
  { name: '报表中心', version: '1.0.0', enabled: true, icon: 'DataAnalysis', color: 'linear-gradient(135deg, #16a085, #48c9b0)' }
])

const apiEndpoints = ref([
  { method: 'GET', path: '/api/admin/users', desc: '获取用户列表' },
  { method: 'POST', path: '/api/admin/users', desc: '创建用户' },
  { method: 'GET', path: '/api/admin/roles', desc: '获取角色列表' },
  { method: 'GET', path: '/api/admin/llm/providers', desc: '获取LLM供应商' },
  { method: 'POST', path: '/api/admin/llm/routing', desc: '保存路由配置' },
  { method: 'GET', path: '/api/admin/knowledge', desc: '获取知识库列表' },
  { method: 'GET', path: '/api/admin/storage/paths', desc: '获取存储路径' },
  { method: 'GET', path: '/api/admin/system/config', desc: '获取系统配置' }
])

const license = reactive({
  type: '企业版',
  expiry: '2027-12-31',
  users: 28,
  maxUsers: 100,
  nodes: '3 节点集群'
})

const changelog = ref([
  {
    version: '3.0.0', date: '2026-08-15', type: 'success',
    items: [
      '全新AI驱动的知识管理引擎',
      '支持多LLM供应商智能路由',
      '新增RBAC权限管理系统',
      '优化大规模知识库检索性能',
      '全新管理控制台UI设计'
    ]
  },
  {
    version: '2.5.0', date: '2026-06-20', type: 'warning',
    items: [
      '增加向量数据库支持',
      '文档解析引擎升级',
      'API性能优化30%'
    ]
  },
  {
    version: '2.0.0', date: '2026-03-10', type: 'info',
    items: [
      '企业级权限体系',
      '多租户支持',
      '审计日志功能'
    ]
  }
])

function refreshInfo() {
  systemInfo.serverTime = new Date().toLocaleString('zh-CN')
  ElMessage.success('系统信息已刷新')
}

function downloadDiagnostic() {
  ElMessage.success('诊断报告已生成，将在下载中心提供')
}

function copyApiList() {
  ElMessage.success('API端点列表已复制到剪贴板')
}

function renewLicense() {
  ElMessage.info('请联系销售团队续期许可证')
}

onMounted(async () => {
  try {
    const data = await adminApi.getSystemInfo()
    if (data?.data) {
      Object.assign(systemInfo, data.data)
    }
  } catch (e) { /* use mock data */ }
})
</script>

<style scoped>
.system-banner {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 20px;
  background: linear-gradient(135deg, #ecf5ff, #f0f9ff);
  border-radius: 10px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.banner-icon {
  width: 72px;
  height: 72px;
  border-radius: 16px;
  background: linear-gradient(135deg, #409eff, #66b1ff);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
}

.banner-info { flex: 1; min-width: 200px; }
.banner-info h2 { margin: 0 0 6px; font-size: 22px; }
.banner-info p { margin: 0 0 10px; color: #606266; }

.banner-tags { display: flex; gap: 8px; }

.banner-actions { display: flex; gap: 8px; }

.version-badge {
  background: #409eff;
  color: #fff;
  padding: 2px 10px;
  border-radius: 12px;
  font-size: 13px;
  font-weight: 500;
}

.section-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px;
}

.modules-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 12px;
}

.module-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border: 1px solid #ebeef5;
  border-radius: 8px;
  transition: all 0.2s;
}

.module-card:hover {
  border-color: #409eff;
  box-shadow: 0 2px 8px rgba(64, 158, 255, 0.1);
}

.module-icon {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
}

.module-info { flex: 1; }
.module-name { font-weight: 600; font-size: 14px; }
.module-version { font-size: 12px; color: #909399; }

.api-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.api-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  background: #fafbfc;
  border-radius: 6px;
  font-size: 13px;
}

.api-method {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  color: #fff;
  min-width: 50px;
  text-align: center;
}

.api-method.get { background: #67c23a; }
.api-method.post { background: #409eff; }
.api-method.put { background: #e6a23c; }
.api-method.delete { background: #f56c6c; }

.api-path {
  flex: 1;
  font-family: 'Consolas', monospace;
  color: #606266;
  font-size: 12px;
}

.api-info { color: #c0c4cc; cursor: help; }

.license-info {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.license-key {
  display: flex;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid #f2f3f5;
  font-size: 13px;
}

.license-key .label { color: #909399; }
.license-key .value { color: #303133; font-weight: 500; }

.support-info {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.support-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  background: #fafbfc;
  border-radius: 6px;
  font-size: 13px;
  color: #606266;
}

.changelog-item strong {
  font-size: 15px;
  color: #303133;
}

.changelog-content ul {
  margin: 8px 0 0;
  padding-left: 20px;
  color: #606266;
  font-size: 13px;
}

.changelog-content li { margin-bottom: 4px; }
</style>