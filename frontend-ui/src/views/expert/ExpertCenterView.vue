<template>
  <div class="expert-center">
    <!-- 简洁页头 -->
    <div class="center-header">
      <div class="header-left">
        <div class="brand-mini">
          <div class="brand-dot"></div>
          <span class="brand-name">专家联盟</span>
          <el-tag v-if="currentProject" size="small" class="project-tag" effect="dark">
            <el-icon><Folder /></el-icon>
            {{ currentProject.name }}
          </el-tag>
        </div>
      </div>
      <div class="header-right">
        <el-button size="small" type="primary" @click="showRegister = true">
          <el-icon><Plus /></el-icon> 注册专家
        </el-button>
        <el-button size="small" @click="ensureProject">
          <el-icon><FolderAdd /></el-icon> 切换项目
        </el-button>
      </div>
    </div>

    <!-- Tab 切换 -->
    <el-tabs v-model="activeTab" class="center-tabs" @tab-change="onTabChange">
      <el-tab-pane label="联盟总览" name="overview" />
      <el-tab-pane label="联盟任务" name="tasks" />
      <el-tab-pane label="企业管理" name="enterprise" />
      <el-tab-pane label="编排引擎" name="orchestrator" />
    </el-tabs>

    <!-- 总览 Tab → 新组件 -->
    <div v-show="activeTab === 'overview'" class="tab-content">
      <ExpertOverviewPanel />
    </div>

    <!-- 企业管理 / 编排引擎 Tab 内容（嵌套路由渲染） -->
    <router-view v-if="activeTab !== 'overview'" v-slot="{ Component }">
      <transition name="fade" mode="out-in">
        <component :is="Component" />
      </transition>
    </router-view>

    <!-- 注册专家弹窗 -->
    <el-dialog v-model="showRegister" title="注册专家" width="520px" :close-on-click-modal="false">
      <el-form label-width="90px" label-position="right">
        <el-form-item label="专家名称" required>
          <el-input v-model="newExpert.name" placeholder="例如：算法策略专家·玄策" />
        </el-form-item>
        <el-form-item label="专家类型">
          <el-select v-model="newExpert.type" style="width: 100%">
            <el-option v-for="t in expertTypes" :key="t" :label="typeLabel(t)" :value="t" />
          </el-select>
        </el-form-item>
        <el-form-item label="能力标签">
          <el-input v-model="newExpert.capabilities_str" placeholder="逗号分隔，例如：架构设计,微服务,性能优化" />
        </el-form-item>
        <el-form-item label="专家描述">
          <el-input v-model="newExpert.description" type="textarea" :rows="3" placeholder="一句话介绍该专家的擅长领域" />
        </el-form-item>
        <el-form-item label="系统提示词">
          <el-input v-model="newExpert.systemPrompt" type="textarea" :rows="4" placeholder="定义该专家的行为模式与回答风格（可选）" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showRegister = false">取消</el-button>
        <el-button type="primary" :loading="registering" @click="doRegister">注册</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage } from 'element-plus/es/components/message/index'
import { Plus, Folder, FolderAdd } from '@element-plus/icons-vue'
import { allianceApi } from '@/modules/expert-alliance/api'
import { EXPERT_TYPES } from '@/constants'
import { useProject } from '@/composables'
import ExpertOverviewPanel from './panels/ExpertOverviewPanel.vue'

const router = useRouter()
const route = useRoute()

// 本页只是外壳：四个 Tab 由嵌套路由渲染，数据都在各自面板里。
// 曾经挂在下面的六条协作流（即时咨询/多专家/辩论/路由/智能/算法）是 Tab 化改造前的残留：
// 模板里没有任何入口调它们，却在每次进页面时随 onMounted 发出总览/指标/需求图谱三次请求。
// 协作流的唯一实现现在在模块里（@/modules/expert-alliance + 工作台 allianceApi.collaborate）。
const activeTab = computed(() => {
  const name = route.name?.toString() || 'ExpertOverview'
  if (name.includes('Tasks')) return 'tasks'
  if (name.includes('Enterprise')) return 'enterprise'
  if (name.includes('Orchestrator')) return 'orchestrator'
  // 兼容旧的 query.tab 链接
  const q = route.query.tab
  if (q === 'tasks' || q === 'enterprise' || q === 'orchestrator') return q
  return 'overview'
})

function onTabChange(tab) {
  const routes = {
    overview: '/expert-center/overview',
    tasks: '/expert-center/tasks',
    enterprise: '/expert-center/enterprise',
    orchestrator: '/expert-center/orchestrator'
  }
  router.push(routes[tab] || routes.overview)
}

// 项目上下文（来自顶栏，共享状态）
const { currentProject, createAndSelect } = useProject()

const showRegister = ref(false)
const registering = ref(false)
const newExpert = ref({ name: '', type: 'algorithm', capabilities_str: '', description: '', systemPrompt: '' })

const expertTypes = computed(() => Object.keys(EXPERT_TYPES))

function typeLabel(t) { return EXPERT_TYPES[t] || t }

async function doRegister() {
  if (!newExpert.value.name || !newExpert.value.type) {
    ElMessage.warning('请填写专家名称和类型')
    return
  }
  registering.value = true
  try {
    // 模块契约 draft 形状：expertType/bio 取代 legacy 的 type/description 别名；
    // capabilities 字符串简写对齐后端 merge 的硬编码 proficiency=85；systemPrompt 走 metadata，模块契约不挂录入入口
    await allianceApi.registerExpert({
      name: newExpert.value.name,
      expertType: newExpert.value.type,
      bio: newExpert.value.description,
      capabilities: (newExpert.value.capabilities_str || '').split(',').map((s) => ({ name: s.trim(), proficiency: 85 })).filter((c) => c.name)
    })
    ElMessage.success('注册成功')
    showRegister.value = false
    newExpert.value = { name: '', type: 'algorithm', capabilities_str: '', description: '', systemPrompt: '' }
  } catch (e) {
    ElMessage.error('注册失败：' + e.message)
  } finally {
    registering.value = false
  }
}

function ensureAndInjectProject() {
  if (currentProject.value) {
    ElMessage.info(`当前项目：${currentProject.value.name}，可继续跟进。`)
    return
  }
  const pj = {
    id: 'pj_' + Date.now().toString(36),
    name: '新璇玑项目',
    description: '由专家联盟创建',
    category: '定制软件',
    status: '规划中'
  }
  try {
    if (typeof createAndSelect === 'function') createAndSelect(pj)
    ElMessage.success(`已创建并选择项目：${pj.name}`)
  } catch (e) {
    ElMessage.warning(e.message || '创建项目失败')
  }
}

function ensureProject() {
  ensureAndInjectProject()
}
</script>

<style scoped>
.expert-center {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px 20px;
  background: var(--bg-primary);
  overflow: hidden;
}

/* 页头 */
.center-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  background: var(--bg-card);
  border-radius: var(--radius);
  border: 1px solid var(--border);
  min-height: 48px;
  flex-shrink: 0;
}
.header-left, .header-right {
  display: flex;
  align-items: center;
  gap: 10px;
}
.brand-mini {
  display: flex;
  align-items: center;
  gap: 10px;
}
.brand-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--cat-1), var(--cat-3));
  box-shadow: 0 0 0 3px var(--accent-dim);
}
.brand-name {
  font-size: 15px;
  font-weight: 700;
  color: var(--text-primary);
}
.project-tag {
  margin-left: 4px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

/* Tabs */
.center-tabs {
  margin: 0;
  flex-shrink: 0;
}
:deep(.center-tabs .el-tabs__header) {
  margin-bottom: 0;
  padding: 0 6px;
  background: var(--bg-card);
  border-radius: var(--radius);
  border: 1px solid var(--border);
}
:deep(.center-tabs .el-tabs__nav-wrap::after) {
  display: none;
}
:deep(.center-tabs .el-tabs__item) {
  font-weight: 600;
  font-size: 14px;
  height: 44px;
  line-height: 44px;
  color: var(--text-secondary);
}
:deep(.center-tabs .el-tabs__item.is-active) {
  color: var(--accent-light);
}
:deep(.center-tabs .el-tabs__active-bar) {
  background-color: var(--accent-fill); color: var(--on-accent);
}

.tab-content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* 过渡动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
