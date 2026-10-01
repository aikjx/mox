<template>
  <div class="awb">
    <header class="awb-head">
      <div>
        <h1 class="awb-title">智能协作工作台</h1>
        <p class="awb-sub">路由 / 单专家 / 多专家 / 辩论 / 智能咨询 / 算法分析 —— 六个原生端点的唯一入口，契约对齐网关 :3080</p>
      </div>
      <div class="awb-actions">
        <el-tag type="info" effect="plain" size="small">候选专家 {{ experts.length }} 位</el-tag>
        <el-button :icon="Refresh" :loading="expertStore.loading.list" @click="reloadExperts">刷新专家</el-button>
      </div>
    </header>

    <el-alert v-if="expertStore.error.list" class="awb-alert" type="error" show-icon :closable="false"
      title="专家名单加载失败" :description="`${expertStore.error.list}；协作仍可提交，未选专家时由后端自动匹配。`" />

    <div class="awb-body">
      <ExpertCollabPanel class="awb-panel" :store="collab" :experts="experts" />
      <aside class="awb-side">
        <h2 class="awb-side-title">与联盟任务的分工</h2>
        <p class="awb-side-text">
          本页直连 <code>POST /api/experts/&lt;mode&gt;</code>：一次请求即拿到协作结果，后端不落库为任务，也没有阶段管线与实时日志。
        </p>
        <p class="awb-side-text">
          <router-link class="awb-link" to="/alliance/console">联盟控制台</router-link>走
          <code>POST /api/alliance/tasks</code>：建任务后由编排器按 7 阶段推进，可看进度、日志流与质量门。
        </p>
        <p class="awb-side-text">
          两套能力在后端是不同 handler 族，信封也不同（本页 flat，任务侧 nested），因此前端不共用 store，也不互相伪造状态。
        </p>
        <h2 class="awb-side-title">结果里不会出现什么</h2>
        <ul class="awb-side-list">
          <li>多专家自动匹配可能 404：后端相似度阈值 0.3，无专家过线时直接报错而不是返回空列表。</li>
          <li>辩论最多 4 位上场，即使勾选更多或把上限调高。</li>
          <li>治理闸门否决时只有拦截说明，没有专家原文；面板会标出否决原因。</li>
          <li>模板兜底作答与真实模型作答在面板上分开标注，置信度取后端返回值。</li>
        </ul>
      </aside>
    </div>
  </div>
</template>

<script setup>
// 协作工作台页面：只提供页面外壳与专家候选名单，表单/校验/结果渲染全在 ExpertCollabPanel 与其 store。
import { computed, onMounted } from 'vue'
import { Refresh } from '@element-plus/icons-vue'
import { useAllianceCollabStore } from '@/modules/expert-alliance/store'
import { useAllianceExpertsStore } from '@/modules/expert-alliance/store'
import { ExpertCollabPanel } from '@/modules/expert-alliance/components'

const collab = useAllianceCollabStore()
const expertStore = useAllianceExpertsStore()

// 候选名单取广场 store 的当前页；协作端点自带自动匹配，名单为空也不阻塞提交
const experts = computed(() => expertStore.experts)

function reloadExperts() {
  expertStore.loadExperts()
}

onMounted(() => {
  if (!expertStore.experts.length) expertStore.loadExperts()
})
</script>

<style scoped>
.awb { display: flex; flex-direction: column; gap: 12px; }
.awb-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
.awb-title { margin: 0; font-size: 20px; color: var(--text-primary); }
.awb-sub { margin: 4px 0 0; font-size: 12px; color: var(--text-secondary); }
.awb-actions { display: flex; align-items: center; gap: 8px; }
.awb-body { display: grid; grid-template-columns: minmax(0, 1fr) 280px; gap: 12px; align-items: start; }
.awb-panel { min-width: 0; }
.awb-side {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
}
.awb-side-title { margin: 0; font-size: 13px; color: var(--text-primary); }
.awb-side-text, .awb-side-list { margin: 0; font-size: 12px; line-height: 1.6; color: var(--text-secondary); }
.awb-side-list { padding-left: 16px; }
.awb-side code { font-size: 11px; color: var(--text-primary); }
.awb-link { color: var(--accent-light); }
@media (max-width: 1180px) {
  .awb-body { grid-template-columns: minmax(0, 1fr); }
}
</style>
