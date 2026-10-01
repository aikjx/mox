<template>
  <section class="sgs">
    <h2 class="sgs-title">全域消息检索</h2>
    <p class="sgs-dim">
      跨会话扫全部消息正文（semantic_search）。同样是大字符 bigram 的 Jaccard 相似度，<strong>不是向量语义检索</strong>——
      名字叫 semantic，实现是字面相似，这里按实现说。
    </p>

    <div class="sgs-form">
      <el-input v-model="store.semanticDraft.query" size="small" placeholder="检索词（必填，后端无缺省）" class="sgs-w-grow"
        @keyup.enter="store.runSemanticSearch()" />
      <el-select :model-value="store.semanticDraft.sessionType" size="small" placeholder="全部类型" clearable
        class="sgs-w-sm" @update:model-value="(v) => (store.semanticDraft.sessionType = v || '')">
        <el-option v-for="t in SESSION_TYPES" :key="t.value" :label="t.label" :value="t.value" />
      </el-select>
      <el-input v-model="store.semanticDraft.expertId" size="small" placeholder="限定某位专家的会话" class="sgs-w-md" />
      <el-input-number v-model="store.semanticDraft.topK" size="small" :min="1" :max="200" controls-position="right"
        placeholder="条数" class="sgs-w-opts" />
      <el-button size="small" type="primary" :icon="Search" :loading="store.loading.semantic"
        @click="store.runSemanticSearch()">
        检索
      </el-button>
    </div>
    <p class="sgs-dim">
      后端门槛只有一条：相似度 &gt; 0 才进结果，没有可配的阈值（本接口没有 min_score 字段，发过去会被丢掉）。
      缺省返回 {{ SEMANTIC_SEARCH_DEFAULTS.topK }} 条。
    </p>

    <el-alert v-if="store.error.semantic" type="error" show-icon :closable="false" title="全域检索失败"
      :description="store.error.semantic" />

    <template v-else-if="semantic">
      <p class="sgs-scope">{{ scopeText }}</p>
      <p v-if="!semantic.results.length" class="sgs-dim">
        没有任何消息与「{{ semantic.query }}」字面相似。检索词越短，bigram 越少，越容易全零。
      </p>
      <ul v-else class="sgs-rows">
        <li v-for="(r, i) in semantic.results" :key="`${r.sessionId}-${r.message.id}`" class="sgs-row">
          <span class="sgs-rank">{{ i + 1 }}</span>
          <div class="sgs-row-main">
            <div class="sgs-row-head">
              <button class="sgs-jump" type="button" title="打开这条会话" @click="store.openSemanticResult(r)">
                {{ r.sessionTitle || '（无标题会话）' }}
              </button>
              <el-tag size="small" effect="plain">{{ roleLabel(r.message.role) }}</el-tag>
              <span class="sgs-dim">{{ formatTime(r.message.createdAt) }}</span>
              <span class="sgs-score">{{ (r.similarityScore * 100).toFixed(1) }}% 相似</span>
            </div>
            <p class="sgs-snippet">{{ r.preview }}<span v-if="r.trailing">…</span></p>
            <p class="sgs-dim">会话 {{ r.sessionId }}</p>
          </div>
        </li>
      </ul>
    </template>
    <p v-else class="sgs-dim">尚未检索。输入检索词后点「检索」。</p>
  </section>
</template>

<script setup>
// 全域检索面板：semantic_search 的响应只有 query / results / total_*_scanned 四类键，
// 既不回显过滤条件、也不给会话状态，所以过滤条件由 store.semanticFilter 自己带着，
// 而"归档会话照样命中"这句只能作为口径说明写在页面上，不能标在单条结果上。
import { computed } from 'vue'
import { Search } from '@element-plus/icons-vue'
import {
  SEMANTIC_SEARCH_DEFAULTS,
  SESSION_TYPES,
  messageRoleLabel,
  semanticScopeText
} from '@/modules/expert-alliance/contract'
import { clip, formatTime } from '@/modules/expert-alliance/model'

const props = defineProps({ store: { type: Object, required: true } })
const store = props.store

const semantic = computed(() => {
  const s = store.semantic
  if (!s) return null
  return {
    ...s,
    results: s.results.map((r) => {
      const c = clip(r.message.content, 180)
      return { ...r, preview: c.text, trailing: c.truncated }
    })
  }
})

const scopeText = computed(() => semanticScopeText(store.semantic || {}, store.semanticFilter))
const roleLabel = (role) => messageRoleLabel(role)
</script>

<style scoped>
.sgs { display: flex; flex-direction: column; gap: 8px; }
.sgs-title { margin: 0; font-size: 15px; color: var(--text-primary); }
.sgs-dim { margin: 0; font-size: 12px; color: var(--text-muted); line-height: 1.6; word-break: break-word; }
.sgs-form { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.sgs-w-grow { flex: 1 1 220px; min-width: 180px; }
.sgs-w-sm { width: 120px; }
.sgs-w-md { width: 180px; }
.sgs-w-opts { width: 110px; }
.sgs-scope { margin: 0; font-size: 12px; color: var(--accent-light); }
.sgs-rows { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
.sgs-row { display: flex; gap: 8px; padding: 8px; border: 1px solid var(--border-light); border-radius: var(--radius-sm); background: var(--bg-card); }
.sgs-rank { font-size: 12px; color: var(--text-muted); font-variant-numeric: tabular-nums; }
.sgs-row-main { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
.sgs-row-head { display: flex; align-items: baseline; gap: 6px; flex-wrap: wrap; font-size: 12px; }
.sgs-jump { background: none; border: 0; padding: 0; font: inherit; color: var(--accent-light); cursor: pointer; word-break: break-word; text-align: left; }
.sgs-jump:hover { text-decoration: underline; }
.sgs-score { color: var(--text-secondary); font-variant-numeric: tabular-nums; }
.sgs-snippet { margin: 0; font-size: 12px; line-height: 1.6; color: var(--text-primary); word-break: break-word; }
</style>
