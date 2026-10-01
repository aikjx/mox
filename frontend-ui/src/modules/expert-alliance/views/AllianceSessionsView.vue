<template>
  <div class="asv">
    <header class="asv-head">
      <div>
        <h1 class="asv-title">专家会话中心</h1>
        <p class="asv-sub">
          会话与消息的服务器侧真相：{{ kpi }}
          智能协作的每次多专家 / 辩论 / 智能咨询都会落一条会话，这里能看到并继续追加。
        </p>
      </div>
      <div class="asv-actions">
        <el-button :icon="MagicStick" @click="goCollab">去智能协作</el-button>
        <el-button :icon="Refresh" :loading="store.loading.list || store.loading.stats" @click="store.refreshAll()">
          刷新
        </el-button>
      </div>
    </header>

    <div class="asv-body">
      <div class="asv-card asv-list">
        <SessionListPanel :store="store" :expert-options="expertOptions" />
      </div>

      <div class="asv-card asv-thread">
        <SessionThreadPanel :store="store" />
      </div>

      <aside class="asv-side">
        <div class="asv-card">
          <SessionMetaPanel :store="store" />
        </div>
      </aside>
    </div>

    <div class="asv-wide">
      <div class="asv-card">
        <SessionStatsPanel :store="store" :expert-names="expertNames" />
      </div>
      <div class="asv-card">
        <SemanticSearchPanel :store="store" />
      </div>
    </div>

    <div class="asv-card asv-notes">
      <h2 class="asv-notes-title">这一页与后端的对应关系</h2>
      <ul class="asv-notes-list">
        <li>
          列表接口<strong>不返回消息正文</strong>，只给 <code>message_count</code>；打开会话才补发一次
          <code>GET /api/experts/sessions/:id</code>。所以列表翻得动，线程只在选中时取。
        </li>
        <li>
          追加消息的响应<strong>只有那条消息</strong>，后端不回会话快照——线程里的新增气泡是本地并入的，
          <code>last_active_at</code> 也取该消息的时间戳。
        </li>
        <li>
          PUT 是合并式更新，且只认 title / status / topic / tags / metadata：
          <strong>专家阵容与类型建完就改不了</strong>，metadata 的键也只能新增与改值、删不掉。
        </li>
        <li>
          status 与 role、msg_type 后端一律不校验。写成 <code>active/archived/closed</code> 之外的值不会报错，
          但统计的三档计数不含它——属性面板会把这种"隐形状态"标出来。
        </li>
        <li>
          两个检索都是<strong>字符 bigram Jaccard 的字面相似度</strong>（后端只有一个 <code>text_similarity</code>），
          跨会话那个接口名叫 semantic，实现并不是向量检索，页面按实现描述它。
        </li>
        <li>
          导出的 <code>download_url</code> 恒为 <code>null</code>，后端只给内容；
          复制与存盘都是浏览器本地动作，没有任何服务端地址可点。
        </li>
        <li>
          会话没有归属校验：<code>user_id</code> 只是列表的过滤条件之一，任何登录用户拿到 id 都能读到正文。
          这是当前后端的真实语义，界面不假称"我的会话"。
        </li>
        <li>
          后端把会话落在 SQLite（<code>data/experts.db</code>，写操作即时 <code>save_sessions()</code>），
          所以刷新页面与重启服务后这些会话仍在。
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup>
// 会话中心：只做装配与跨面板共用的候选（专家名单），接口调用与状态全在 alliance-sessions.store。
import { computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { MagicStick, Refresh } from '@element-plus/icons-vue'
import { useAllianceSessionsStore } from '@/modules/expert-alliance/store'
import { useAllianceExpertsStore } from '@/modules/expert-alliance/store'
import { SessionListPanel } from '@/modules/expert-alliance/components'
import { SessionThreadPanel } from '@/modules/expert-alliance/components'
import { SessionMetaPanel } from '@/modules/expert-alliance/components'
import { SessionStatsPanel } from '@/modules/expert-alliance/components'
import { SemanticSearchPanel } from '@/modules/expert-alliance/components'

const store = useAllianceSessionsStore()
const expertStore = useAllianceExpertsStore()
const router = useRouter()

const kpi = computed(() => {
  const s = store.stats
  if (!s) return store.list.total ? `列表命中 ${store.list.total} 个会话；` : ''
  return `${s.totalSessions} 个会话 / ${s.totalMessages} 条消息，今日新建 ${s.sessionsToday} 个；`
})

// 专家候选与名字都取自己加载的那一页注册表；越界的 id 只能标成"不在本页名单内"，
// 不能标成"专家不存在"——后端 expert_ids 从不校验成员资格。
const expertOptions = computed(() =>
  expertStore.experts.map((e) => ({ value: e.id, label: e.name || e.id }))
)
const expertNames = computed(() => {
  const map = {}
  for (const e of expertStore.experts) map[e.id] = e.name || e.id
  return map
})

function goCollab() {
  router.push('/alliance/collab')
}

onMounted(() => {
  store.refreshAll()
  if (!expertStore.experts.length) expertStore.loadExperts()
})
</script>

<style scoped>
.asv { display: flex; flex-direction: column; gap: 12px; }
.asv-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
.asv-title { margin: 0; font-size: 20px; color: var(--text-primary); }
.asv-sub { margin: 4px 0 0; font-size: 12px; color: var(--text-secondary); line-height: 1.6; }
.asv-actions { display: flex; align-items: center; gap: 8px; }
.asv-card {
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
  min-width: 0;
}
.asv-body { display: grid; gap: 12px; align-items: start; grid-template-columns: 320px minmax(0, 1fr) 360px; grid-template-areas: "list thread side"; }
.asv-list { grid-area: list; }
.asv-thread { grid-area: thread; }
.asv-side { grid-area: side; display: flex; flex-direction: column; gap: 12px; min-width: 0; }
.asv-wide { display: grid; gap: 12px; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); align-items: start; }
.asv-notes { display: flex; flex-direction: column; gap: 6px; }
.asv-notes-title { margin: 0; font-size: 13px; color: var(--text-primary); }
.asv-notes-list { margin: 0; padding-left: 16px; display: flex; flex-direction: column; gap: 6px; font-size: 12px; line-height: 1.6; color: var(--text-secondary); }
.asv-notes-list code { font-size: 11px; color: var(--accent-light); }
@media (max-width: 1400px) {
  .asv-body { grid-template-columns: 300px minmax(0, 1fr); grid-template-areas: "list thread" "side side"; }
}
@media (max-width: 1180px) {
  .asv-body { grid-template-columns: minmax(0, 1fr); grid-template-areas: "list" "thread" "side"; }
}
</style>
