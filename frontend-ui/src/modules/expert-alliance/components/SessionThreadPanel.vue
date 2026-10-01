<template>
  <section class="sth">
    <el-alert v-if="store.error.detail" type="error" show-icon :closable="false" title="会话详情获取失败"
      :description="`${store.error.detail}；正文只有这一个端点能取到，列表里的 message_count 不含内容。`" />

    <p v-else-if="!session" class="sth-empty">从左侧选择一个会话，或新建一条。打开会话会补发一次详情请求——列表接口刻意剥掉了 messages。</p>

    <template v-else>
      <header class="sth-head">
        <div class="sth-head-main">
          <h2 class="sth-title">{{ session.title || '（无标题会话）' }}</h2>
          <p class="sth-id">
            <code>{{ session.id }}</code>
            <span>· 类型 {{ typeLabel }}</span>
            <span>· 状态 <em :class="{ 'sth-invisible': !statusCounted }">{{ statusLabel }}</em></span>
            <span>· {{ session.messageCount }} 条消息</span>
            <span v-if="session.userId">· 发起人 {{ session.userId }}</span>
          </p>
        </div>
        <el-tag v-if="session.archived" size="small" type="info">已归档 {{ formatTime(session.archivedAt) }}</el-tag>
      </header>

      <dl class="sth-facts">
        <div class="sth-fact"><dt>创建</dt><dd>{{ formatTime(session.createdAt) }}</dd></div>
        <div class="sth-fact"><dt>最近活跃</dt><dd>{{ formatTime(session.lastActiveAt) }}{{ activeHint }}</dd></div>
        <div class="sth-fact"><dt>关联专家</dt><dd>{{ expertText }}</dd></div>
        <div class="sth-fact"><dt>标签</dt><dd>{{ session.tags.join('、') || '—' }}</dd></div>
      </dl>
      <p v-if="session.topic" class="sth-topic">{{ session.topic }}</p>

      <div class="sth-thread">
        <p v-if="!messages.length" class="sth-dim">
          这条会话还没有消息。协作写路径（多专家 / 辩论 / 智能咨询）会自动落会话与消息，也可以直接在下方追加。
        </p>
        <ul v-else class="sth-msgs">
          <li v-for="m in messages" :key="m.id" class="sth-msg">
            <div class="sth-msg-head">
              <el-tag size="small" :type="roleTag(m.role)" effect="plain">{{ roleLabel(m.role) }}</el-tag>
              <span v-if="m.senderName || m.senderId" class="sth-msg-sender">{{ m.senderName || m.senderId }}</span>
              <span class="sth-dim">{{ formatTime(m.createdAt) }}</span>
              <el-tag v-if="m.msgType && m.msgType !== 'text'" size="small" type="info" effect="plain">
                {{ msgTypeLabel(m.msgType) }}
              </el-tag>
              <span v-if="m.rating !== null" class="sth-msg-rating">评分 {{ m.rating }}{{ ratingOutOfRange(m.rating) }}</span>
              <span v-if="m.attachments.length" class="sth-dim">{{ m.attachments.length }} 项附件</span>
            </div>
            <pre class="sth-msg-body">{{ m.content }}</pre>
          </li>
        </ul>
        <p v-if="hiddenCount" class="sth-hidden">
          为控制渲染量，本列表只显示最近 {{ THREAD_RENDER_LIMIT }} 条（共 {{ messagesTotal }} 条）——
          后端详情接口一次返回全部消息，没有分页，截断只发生在这里。
        </p>
      </div>

      <div class="sth-composer">
        <h3 class="sth-subtitle">追加消息</h3>
        <div class="sth-composer-grid">
          <label class="sth-field">
            <span class="sth-label">角色（后端必填，缺失即 422）</span>
            <el-select v-model="store.composer.role" size="small" class="sth-w-full">
              <el-option v-for="r in MESSAGE_ROLES" :key="r.value" :label="`${r.label}（${r.value}）`" :value="r.value" />
            </el-select>
          </label>
          <label class="sth-field">
            <span class="sth-label">消息类型</span>
            <el-select v-model="store.composer.msgType" size="small" class="sth-w-full">
              <el-option v-for="t in MSG_TYPES" :key="t.value" :label="`${t.label}（${t.value}）`" :value="t.value" />
            </el-select>
          </label>
          <label class="sth-field">
            <span class="sth-label">发送者名称（后端不查注册表，显示什么就得填什么）</span>
            <el-input v-model="store.composer.senderName" size="small" placeholder="可留空" />
          </label>
          <label class="sth-field">
            <span class="sth-label">发送者 ID</span>
            <el-input v-model="store.composer.senderId" size="small" placeholder="专家 ID / 用户 ID，可留空" />
          </label>
          <label class="sth-field">
            <span class="sth-label">评分（0–5，只在追加时写入）</span>
            <el-input-number v-model="store.composer.rating" size="small" :min="MESSAGE_RATING.min"
              :max="MESSAGE_RATING.max" controls-position="right" class="sth-w-full" />
          </label>
        </div>
        <label class="sth-field">
          <span class="sth-label">正文（后端必填；原样入库，界面按纯文本呈现）</span>
          <el-input v-model="store.composer.content" size="small" type="textarea" :rows="3" maxlength="2000"
            show-word-limit placeholder="输入要追加到这条会话的消息内容" />
        </label>
        <el-alert v-if="store.error.append" type="error" show-icon :closable="false" title="无法追加"
          :description="store.error.append" />
        <div class="sth-composer-actions">
          <span class="sth-dim">{{ composerHint }}</span>
          <el-button size="small" type="primary" :loading="store.loading.append" :disabled="!!composerProblem"
            @click="store.appendMessage()">
            追加
          </el-button>
        </div>
        <p v-if="archivedAppendNote" class="sth-warn">{{ archivedAppendNote }}</p>
      </div>

      <div class="sth-similar">
        <h3 class="sth-subtitle">会话内字面相似检索</h3>
        <p class="sth-dim">
          算法是字符 bigram 的 Jaccard 相似度（后端 text_similarity），只看字面重合，<strong>不是语义模型</strong>；
          命中范围仅本会话的消息。
        </p>
        <div class="sth-similar-form">
          <el-input v-model="store.similarDraft.query" size="small" placeholder="检索词（必填）" class="sth-w-md"
            @keyup.enter="store.runSimilarSearch()" />
          <el-input-number v-model="store.similarDraft.topK" size="small" :min="1" :max="200" controls-position="right"
            placeholder="返回条数" class="sth-w-opts" />
          <el-input-number v-model="store.similarDraft.minScore" size="small" :min="0" :max="1" :step="0.05"
            :precision="2" controls-position="right" placeholder="阈值" class="sth-w-opts" />
          <el-button size="small" :loading="store.loading.similar" @click="store.runSimilarSearch()">检索</el-button>
        </div>
        <p class="sth-dim">
          缺省：返回 {{ SIMILAR_SEARCH_DEFAULTS.topK }} 条、阈值 {{ SIMILAR_SEARCH_DEFAULTS.minScore
          }}；两者留空即不发送，由后端取自身缺省。top_k 后端没有上限，这里的 200 只是本表单的输入约束。
        </p>
        <el-alert v-if="store.error.similar" type="error" show-icon :closable="false" title="检索失败"
          :description="store.error.similar" />
        <template v-else-if="similar">
          <p v-if="!similar.results.length" class="sth-dim">
            没有命中：本会话 {{ session.messageCount }} 条消息里没有相似度 ≥ 阈值的（检索词「{{ similar.query }}」）。
          </p>
          <ul v-else class="sth-similar-rows">
            <li v-for="r in similar.results" :key="r.message.id" class="sth-similar-row">
              <span class="sth-similar-rank">#{{ r.rank }}</span>
              <span class="sth-similar-score">{{ (r.similarityScore * 100).toFixed(1) }}%</span>
              <span class="sth-dim">{{ roleLabel(r.message.role) }}</span>
              <span class="sth-similar-text">{{ r.preview }}</span>
              <span v-if="r.trailing" class="sth-dim">{{ r.trailing }}</span>
            </li>
          </ul>
          <p v-if="truncatedNote" class="sth-warn">{{ truncatedNote }}</p>
        </template>
      </div>
    </template>
  </section>
</template>

<script setup>
// 会话线程面板：读详情 + 追加消息 + 会话内字面相似检索。
// 三个后端事实决定了这里的文案与控件：
// 1. messages 只有本详情接口给，列表不给；
// 2. 追加的响应只有那条消息，store 负责并线；
// 3. rating 是 Option<u8>，后端只挡 >255，业务区间 0–5 由这里约束，且写进去就改不了（无消息编辑端点）。
import { computed } from 'vue'
import {
  MESSAGE_ROLES,
  MESSAGE_RATING,
  MSG_TYPES,
  SIMILAR_SEARCH_DEFAULTS,
  SESSION_STATUS_COUNTED,
  THREAD_RENDER_LIMIT,
  activeMinutes,
  appendMessageProblem,
  messageRoleLabel,
  msgTypeLabel,
  sessionStatusLabel,
  sessionTypeLabel,
  similarTruncated,
  threadWindow
} from '@/modules/expert-alliance/contract'
import { clip, formatTime } from '@/modules/expert-alliance/model'

const props = defineProps({ store: { type: Object, required: true } })
const store = props.store

const session = computed(() => store.detail)
const typeLabel = computed(() => sessionTypeLabel(session.value?.sessionType))
const statusLabel = computed(() => sessionStatusLabel(session.value?.status))
const statusCounted = computed(() => SESSION_STATUS_COUNTED.includes(session.value?.status))
const expertText = computed(() => session.value?.expertIds.join('、') || '未关联（PUT 也改不了这一项）')

const windowed = computed(() => threadWindow(store.messages))
const messages = computed(() => windowed.value.rows)
const messagesTotal = computed(() => store.messages.length)
const hiddenCount = computed(() => windowed.value.hidden)

const activeHint = computed(() => {
  const minutes = activeMinutes(session.value?.createdAt, session.value?.lastActiveAt)
  return minutes === null ? '' : `（距创建 ${minutes < 60 ? `${minutes.toFixed(1)} 分钟` : `${(minutes / 60).toFixed(1)} 小时`}）`
})

const composerProblem = computed(() => appendMessageProblem(store.composer))
const composerHint = computed(() => composerProblem.value || '正文按原样入库；追加后列表的消息数会同步重取')
const archivedAppendNote = computed(() =>
  session.value?.archived ? '这条会话已归档，但后端不禁止继续追加消息（append_message 不看 status）——界面照发，落库照常。' : ''
)

const similar = computed(() =>
  store.similar
    ? {
        ...store.similar,
        results: store.similar.results.map((r) => {
          const c = clip(r.message.content, 120)
          return { ...r, preview: c.text, trailing: c.truncated ? '…' : '' }
        })
      }
    : null
)
const truncatedNote = computed(() =>
  similar.value && similarTruncated(similar.value)
    ? `命中 ${similar.value.totalFound} 条，只列了前 ${similar.value.results.length} 条——想看完请调高「返回条数」。`
    : ''
)

const roleLabel = (role) => messageRoleLabel(role)
const roleTag = (role) => {
  if (role === 'user') return 'primary'
  if (role === 'expert') return 'success'
  if (role === 'system') return 'info'
  return 'warning'
}
// 后端不校验评分，21 分照样入库；出现越界值只能标出来，不能假装是 0–5
const ratingOutOfRange = (rating) => (rating > MESSAGE_RATING.max || rating < MESSAGE_RATING.min ? '（超出 0–5：后端按 u8 照收）' : '')
</script>

<style scoped>
.sth { display: flex; flex-direction: column; gap: 10px; }
.sth-title { margin: 0; font-size: 15px; color: var(--text-primary); word-break: break-word; }
.sth-subtitle { margin: 0; font-size: 13px; color: var(--text-primary); }
.sth-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; flex-wrap: wrap; }
.sth-head-main { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
.sth-id { margin: 0; display: flex; gap: 6px; flex-wrap: wrap; font-size: 12px; color: var(--text-muted); }
.sth-id code { font-size: 11px; color: var(--accent-light); word-break: break-all; }
.sth-id em { font-style: normal; color: var(--text-secondary); }
.sth-invisible { color: var(--warning); }
.sth-empty, .sth-dim { margin: 0; font-size: 12px; color: var(--text-muted); }
.sth-facts { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 6px; margin: 0; }
.sth-fact { padding: 6px 8px; border: 1px solid var(--border-light); border-radius: var(--radius-sm); background: var(--bg-card); }
.sth-fact dt { font-size: 12px; color: var(--text-muted); }
.sth-fact dd { margin: 2px 0 0; font-size: 12px; color: var(--text-primary); word-break: break-word; }
.sth-topic { margin: 0; padding: 8px; font-size: 12px; line-height: 1.6; color: var(--text-secondary); border-left: 2px solid var(--accent); background: var(--bg-secondary); border-radius: var(--radius-sm); }
.sth-thread { display: flex; flex-direction: column; gap: 6px; max-height: 460px; overflow-y: auto; padding: 8px; border: 1px solid var(--border-light); border-radius: var(--radius-sm); background: var(--bg-secondary); }
.sth-msgs { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
.sth-msg { display: flex; flex-direction: column; gap: 4px; padding: 8px; border: 1px solid var(--border-light); border-radius: var(--radius-sm); background: var(--bg-card); }
.sth-msg-head { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; font-size: 12px; }
.sth-msg-sender { color: var(--text-secondary); word-break: break-all; }
.sth-msg-rating { color: var(--warning); }
.sth-msg-body { margin: 0; font-family: inherit; font-size: 13px; line-height: 1.65; color: var(--text-primary); white-space: pre-wrap; word-break: break-word; }
.sth-hidden { margin: 0; font-size: 12px; color: var(--warning); }
.sth-composer, .sth-similar { display: flex; flex-direction: column; gap: 8px; padding-top: 10px; border-top: 1px solid var(--border-light); }
.sth-composer-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 8px; }
.sth-field { display: flex; flex-direction: column; gap: 4px; }
.sth-label { font-size: 12px; color: var(--text-secondary); }
.sth-composer-actions { display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap; }
.sth-w-full { width: 100%; }
.sth-w-md { width: 200px; }
.sth-w-opts { width: 120px; }
.sth-warn { margin: 0; font-size: 12px; color: var(--warning); }
.sth-similar-form { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.sth-similar-rows { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
.sth-similar-row { display: flex; align-items: baseline; gap: 6px; flex-wrap: wrap; font-size: 12px; color: var(--text-secondary); }
.sth-similar-rank { color: var(--text-muted); }
.sth-similar-score { color: var(--accent-light); font-variant-numeric: tabular-nums; }
.sth-similar-text { color: var(--text-primary); word-break: break-word; }
</style>
