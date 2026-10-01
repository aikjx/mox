<template>
  <section class="slp">
    <header class="slp-head">
      <h2 class="slp-title">会话列表</h2>
      <div class="slp-head-actions">
        <el-button size="small" :icon="Refresh" :loading="store.loading.list" @click="store.loadList()">刷新</el-button>
        <el-button size="small" type="primary" :icon="Plus" @click="creating = !creating">
          {{ creating ? '收起' : '新建会话' }}
        </el-button>
      </div>
    </header>

    <div class="slp-filters">
      <el-select :model-value="store.filters.status" size="small" placeholder="全部状态" clearable class="slp-w-sm"
        @update:model-value="(v) => onFilter('status', v)">
        <el-option v-for="s in SESSION_STATUSES" :key="s.value" :label="s.label" :value="s.value" />
      </el-select>
      <el-select :model-value="store.filters.sessionType" size="small" placeholder="全部类型" clearable class="slp-w-sm"
        @update:model-value="(v) => onFilter('sessionType', v)">
        <el-option v-for="t in SESSION_TYPES" :key="t.value" :label="t.label" :value="t.value" />
      </el-select>
      <el-input :model-value="store.filters.search" size="small" clearable placeholder="标题或主题关键字" class="slp-w-md"
        @update:model-value="(v) => onFilter('search', v)" @keyup.enter="store.loadList()" />
      <el-input :model-value="store.filters.userId" size="small" clearable placeholder="user_id（可选）" class="slp-w-md"
        @update:model-value="(v) => onFilter('userId', v)" @keyup.enter="store.loadList()" />
      <el-select :model-value="store.filters.pageSize" size="small" class="slp-w-xs"
        @update:model-value="(v) => onFilter('pageSize', v)">
        <el-option v-for="n in SESSION_PAGE_SIZES" :key="n" :label="`每页 ${n}`" :value="n" />
      </el-select>
      <el-button size="small" :loading="store.loading.list" @click="store.loadList()">查询</el-button>
    </div>

    <p class="slp-scope">
      命中 {{ store.list.total }} 个会话，第 {{ store.list.page }} / {{ store.totalPages }} 页。
      排序固定按创建时间倒序——后端没有 sort 参数，所以这里不摆排序控件。
    </p>

    <el-alert v-if="store.error.list" type="error" show-icon :closable="false" title="会话列表获取失败"
      :description="store.error.list" />

    <div v-if="creating" class="slp-create">
      <h3 class="slp-subtitle">新建会话</h3>
      <div class="slp-create-grid">
        <label class="slp-field">
          <span class="slp-label">标题</span>
          <el-input v-model="store.createDraft.title" size="small" :placeholder="`留空则记为「${fallbackTitle}」`" />
        </label>
        <label class="slp-field">
          <span class="slp-label">类型</span>
          <el-select v-model="store.createDraft.sessionType" size="small" class="slp-w-full">
            <el-option v-for="t in SESSION_TYPES" :key="t.value" :label="`${t.label}（${t.value}）`" :value="t.value" />
          </el-select>
        </label>
        <label class="slp-field">
          <span class="slp-label">发起用户 user_id</span>
          <el-input v-model="store.createDraft.userId" size="small" placeholder="留空即空串" />
        </label>
        <label class="slp-field slp-field-wide">
          <span class="slp-label">主题 / 首个问题</span>
          <el-input v-model="store.createDraft.topic" size="small" type="textarea" :rows="2"
            placeholder="列表页的搜索就搜这里" />
        </label>
        <label class="slp-field slp-field-wide">
          <span class="slp-label">关联专家</span>
          <el-select v-model="store.createDraft.expertIds" size="small" multiple filterable allow-create
            default-first-option class="slp-w-full" placeholder="可留空，之后也无法修改（PUT 不含 expert_ids）">
            <el-option v-for="o in expertOptions" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </label>
        <label class="slp-field slp-field-wide">
          <span class="slp-label">标签</span>
          <el-select v-model="store.createDraft.tags" size="small" multiple filterable allow-create
            default-first-option class="slp-w-full" placeholder="回车确认，可留空" />
        </label>
      </div>
      <el-alert v-if="store.error.create" type="error" show-icon :closable="false" title="无法创建"
        :description="store.error.create" />
      <div class="slp-create-actions">
        <span class="slp-dim">
          初始状态由后端固定为 active；user_id 是与上方过滤框同一根线的键——协作路径写的是 anonymous / enterprise-user，
          留空则这条会话永远不会被任何 user_id 过滤命中
        </span>
        <el-button size="small" type="primary" :loading="store.loading.create" @click="store.createSession()">
          创建并打开
        </el-button>
      </div>
    </div>

    <ul v-if="rows.length" class="slp-rows">
      <li v-for="row in rows" :key="row.id">
        <button class="slp-row" :class="{ 'is-active': row.id === store.selectedId }" type="button"
          @click="store.selectSession(row.id)">
          <span class="slp-row-title">{{ row.title }}</span>
          <span class="slp-row-tags">
            <el-tag size="small" effect="plain">{{ row.typeLabel }}</el-tag>
            <el-tag size="small" :type="row.statusTag" effect="plain">{{ row.statusLabel }}</el-tag>
            <el-tag v-if="row.expertCount" size="small" type="info" effect="plain">{{ row.expertCount }} 位专家</el-tag>
          </span>
          <span class="slp-row-meta">
            <span>{{ row.messageCount }} 条消息</span>
            <span>创建 {{ row.created }}</span>
            <span>活跃 {{ row.lastActive }}</span>
          </span>
          <span v-if="row.topicText" class="slp-row-topic">{{ row.topicText }}</span>
        </button>
      </li>
    </ul>
    <p v-else-if="!store.loading.list" class="slp-dim">
      当前过滤条件下没有会话{{ store.filters.status || store.filters.sessionType || store.filters.search || store.filters.userId ? '（试着清掉部分条件）' : '；可先「新建会话」，或从智能协作/图谱面板发起一次咨询——协作写路径会把会话落库，这里就是它的服务器侧真相' }}。
    </p>

    <div v-if="store.list.total > store.filters.pageSize" class="slp-pager">
      <el-pagination layout="prev, pager, next" :current-page="store.list.page" :page-size="store.filters.pageSize"
        :total="store.list.total" background @current-change="onPage" />
    </div>
  </section>
</template>

<script setup>
// 会话列表面板：过滤条件与分页都在 store.filters 里，本组件只发意图。
// 空串一律不发（后端把 Some("") 当有效过滤，会把结果清成空表），这条规则在 contract/sessionListQuery。
import { computed, ref } from 'vue'
import { Plus, Refresh } from '@element-plus/icons-vue'
import {
  SESSION_PAGE_SIZES,
  SESSION_STATUSES,
  SESSION_TYPES,
  sessionDraftTitle,
  sessionStatusLabel,
  sessionTypeLabel
} from '@/modules/expert-alliance/contract'
import { formatTime } from '@/modules/expert-alliance/model'

const props = defineProps({
  store: { type: Object, required: true },
  expertOptions: { type: Array, default: () => [] }
})
const store = props.store
const creating = ref(false)

const fallbackTitle = computed(() => sessionDraftTitle(store.createDraft))

function onFilter(key, value) {
  store.setFilter(key, value ?? '')
}

function onPage(page) {
  store.setFilter('page', page)
  store.loadList()
}

const statusTagType = (status) => {
  if (status === 'active') return 'success'
  if (status === 'archived') return 'info'
  if (status === 'closed') return 'warning'
  // 统计只认三个值，写成别的状态就是"库里存在但统计隐形"
  return 'danger'
}

const rows = computed(() =>
  (store.list.items || []).map((s) => ({
    id: s.id,
    title: s.title || `（无标题）${s.id}`,
    typeLabel: sessionTypeLabel(s.sessionType),
    statusLabel: sessionStatusLabel(s.status),
    statusTag: statusTagType(s.status),
    expertCount: s.expertIds.length,
    messageCount: s.messageCount,
    created: formatTime(s.createdAt),
    lastActive: formatTime(s.lastActiveAt),
    topicText: s.topic.length > 90 ? `${s.topic.slice(0, 90)}…` : s.topic
  }))
)
</script>

<style scoped>
.slp { display: flex; flex-direction: column; gap: 10px; }
.slp-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap; }
.slp-title { margin: 0; font-size: 15px; color: var(--text-primary); }
.slp-subtitle { margin: 0; font-size: 13px; color: var(--text-primary); }
.slp-head-actions, .slp-filters { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.slp-scope { margin: 0; font-size: 12px; color: var(--text-muted); }
.slp-dim { margin: 0; font-size: 12px; color: var(--text-muted); }
.slp-w-xs { width: 96px; }
.slp-w-sm { width: 132px; }
.slp-w-md { width: 180px; }
.slp-w-full { width: 100%; }
.slp-create { display: flex; flex-direction: column; gap: 8px; padding: 10px; border: 1px dashed var(--border); border-radius: var(--radius-sm); background: var(--bg-secondary); }
.slp-create-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 8px; }
.slp-field { display: flex; flex-direction: column; gap: 4px; }
.slp-field-wide { grid-column: 1 / -1; }
.slp-label { font-size: 12px; color: var(--text-secondary); }
.slp-create-actions { display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap; }
.slp-rows { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
.slp-row { display: flex; flex-direction: column; gap: 4px; width: 100%; padding: 8px 10px; text-align: left; font: inherit; color: inherit; background: var(--bg-card); border: 1px solid var(--border-light); border-radius: var(--radius-sm); cursor: pointer; transition: border-color var(--dur-2) var(--ease), background var(--dur-2) var(--ease); }
.slp-row:hover { background: var(--bg-hover); }
.slp-row.is-active { border-color: var(--accent); background: var(--accent-dim); }
.slp-row-title { font-size: 13px; color: var(--text-primary); word-break: break-word; }
.slp-row-tags { display: flex; gap: 4px; flex-wrap: wrap; }
.slp-row-meta { display: flex; gap: 10px; flex-wrap: wrap; font-size: 12px; color: var(--text-muted); }
.slp-row-topic { font-size: 12px; color: var(--text-secondary); word-break: break-word; }
.slp-pager { display: flex; justify-content: center; }
</style>
