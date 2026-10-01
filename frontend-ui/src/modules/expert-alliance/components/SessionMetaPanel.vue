<template>
  <section class="smp">
    <h2 class="smp-title">会话属性与生命周期</h2>
    <p v-if="!session" class="smp-dim">选中会话后可编辑。后端 PUT 只认 title / status / topic / tags / metadata 五个键——
      expert_ids、user_id、session_type 发过去会被 serde 静默丢弃，想换阵容只能新建会话。</p>

    <template v-else>
      <div class="smp-form">
        <label class="smp-field">
          <span class="smp-label">标题</span>
          <el-input v-model="store.editDraft.title" size="small" maxlength="120" show-word-limit />
        </label>
        <label class="smp-field">
          <span class="smp-label">状态</span>
          <el-select v-model="store.editDraft.status" size="small" class="smp-w-full" allow-create filterable>
            <el-option v-for="s in SESSION_STATUSES" :key="s.value" :label="`${s.label}（${s.value}）`" :value="s.value" />
          </el-select>
        </label>
        <label class="smp-field smp-field-wide">
          <span class="smp-label">主题 / 首个问题</span>
          <el-input v-model="store.editDraft.topic" size="small" type="textarea" :rows="2" />
        </label>
        <label class="smp-field smp-field-wide">
          <span class="smp-label">标签</span>
          <el-select v-model="store.editDraft.tags" size="small" multiple filterable allow-create default-first-option
            class="smp-w-full" placeholder="回车确认" />
        </label>
      </div>

      <el-alert v-if="store.editStatusProblem" type="warning" show-icon :closable="false" title="这个状态后端照收"
        :description="store.editStatusProblem" />
      <el-alert v-if="store.error.update" type="error" show-icon :closable="false" title="更新失败"
        :description="store.error.update" />

      <div class="smp-meta">
        <h3 class="smp-subtitle">metadata 键值</h3>
        <p class="smp-dim">
          后端是 <code>Map&lt;String, Value&gt;</code> 且合并式写入：只能新增或改值，<strong>没有删除某个键的端点</strong>。
          值按字符串上送（表单收的就是文本，不做 JSON 猜测）。
        </p>
        <ul v-if="store.metaRows.length" class="smp-meta-rows">
          <li v-for="(row, i) in store.metaRows" :key="`${row.key}-${i}`" class="smp-meta-row">
            <el-input :model-value="row.key" size="small" placeholder="键" class="smp-meta-key"
              @update:model-value="(v) => store.setMetaRow(i, 'key', v)" />
            <el-input :model-value="row.value" size="small" placeholder="值" class="smp-meta-value"
              @update:model-value="(v) => store.setMetaRow(i, 'value', v)" />
            <el-button size="small" text :icon="Delete" @click="store.removeMetaRow(i)">移除</el-button>
          </li>
        </ul>
        <p v-else class="smp-dim">这条会话还没有 metadata。</p>
        <div class="smp-meta-actions">
          <el-button size="small" :icon="Plus" @click="store.addMetaRow()">加一行</el-button>
          <span class="smp-dim">键为空的行不会发出</span>
        </div>
      </div>

      <div class="smp-actions">
        <el-button size="small" type="primary" :loading="store.loading.update" @click="store.saveEdit()">
          保存改动
        </el-button>
        <el-button size="small" :loading="store.loading.export" @click="store.exportSession()">导出</el-button>
        <el-popconfirm title="归档后只能靠改 status 回 active，没有反归档端点，确定？" width="260"
          @confirm="store.archiveSession()">
          <template #reference>
            <el-button size="small" :loading="store.loading.archive" :disabled="session.archived">归档</el-button>
          </template>
        </el-popconfirm>
        <el-popconfirm title="删除是硬删，后端没有回收站，确定？" width="240" @confirm="store.deleteSession()">
          <template #reference>
            <el-button size="small" type="danger" plain :loading="store.loading.remove">删除</el-button>
          </template>
        </el-popconfirm>
      </div>
      <p v-if="store.error.archive" class="smp-danger">归档失败：{{ store.error.archive }}</p>
      <p v-if="store.error.remove" class="smp-danger">删除失败：{{ store.error.remove }}</p>
      <p v-if="store.notice" class="smp-notice">{{ store.notice }}</p>

      <div v-if="exportBox" class="smp-export">
        <h3 class="smp-subtitle">导出结果</h3>
        <p class="smp-dim">
          响应里的 <code>download_url</code> 恒为 null——后端只给内容，不给地址，所以下面这份文本与本地保存动作都在浏览器里完成。
        </p>
        <ul class="smp-export-meta">
          <li>格式：{{ exportBox.format }}</li>
          <li>消息数：{{ exportBox.messageCount }}</li>
          <li>导出时间：{{ formatTime(exportBox.exportedAt) }}</li>
          <li>下载地址：{{ exportBox.downloadUrl === null ? '后端不提供（恒 null）' : exportBox.downloadUrl }}</li>
        </ul>
        <pre class="smp-export-body">{{ exportBox.text }}</pre>
        <div class="smp-export-actions">
          <el-button size="small" :icon="Download" @click="download">存为 {{ exportBox.fileName }}</el-button>
          <el-button size="small" :icon="CopyDocument" @click="copy">复制 JSON</el-button>
          <span v-if="copyNote" class="smp-notice">{{ copyNote }}</span>
        </div>
        <p class="smp-dim">
          导出内容与详情接口同源（完整 ExpertSession，含 messages），区别只在它多包了一层元信息。
        </p>
      </div>
      <el-alert v-else-if="store.error.export" type="error" show-icon :closable="false" title="导出失败"
        :description="store.error.export" />
    </template>
  </section>
</template>

<script setup>
// 会话属性与生命周期面板：PUT 合并式更新、归档、删除、导出。
// 这里不发没改动的键（contract/sessionUpdatePatch 负责差分），也不画假的下载链接。
import { computed, ref } from 'vue'
import { CopyDocument, Delete, Download, Plus } from '@element-plus/icons-vue'
import { SESSION_STATUSES } from '@/modules/expert-alliance/contract'
import { formatTime } from '@/modules/expert-alliance/model'

const props = defineProps({ store: { type: Object, required: true } })
const store = props.store
const copyNote = ref('')

const session = computed(() => store.detail)

const exportBox = computed(() => {
  const e = store.exportResult
  if (!e) return null
  return {
    format: e.format,
    messageCount: e.messageCount,
    exportedAt: e.exportedAt,
    downloadUrl: e.downloadUrl,
    fileName: store.exportDownloadName(),
    text: store.exportPayloadText()
  }
})

function download() {
  const text = exportBox.value?.text || ''
  const name = exportBox.value?.fileName || 'expert-session.json'
  // createObjectURL 必须 revoke，否则每次导出都在内存里留一个引用
  const url = URL.createObjectURL(new Blob([text], { type: 'application/json' }))
  const a = document.createElement('a')
  a.href = url
  a.download = name
  a.click()
  URL.revokeObjectURL(url)
  copyNote.value = `已在本地保存 ${name}`
}

async function copy() {
  const text = exportBox.value?.text || ''
  try {
    await navigator.clipboard.writeText(text)
    copyNote.value = '导出内容已复制到剪贴板'
  } catch {
    // 非安全上下文里 clipboard 不可用，此时上面的文本框就是可用的兜底
    copyNote.value = '剪贴板不可用（多为非 HTTPS 环境），请从上方文本框手动复制'
  }
}
</script>

<style scoped>
.smp { display: flex; flex-direction: column; gap: 10px; }
.smp-title { margin: 0; font-size: 15px; color: var(--text-primary); }
.smp-subtitle { margin: 0; font-size: 13px; color: var(--text-primary); }
.smp-dim { margin: 0; font-size: 12px; color: var(--text-muted); line-height: 1.6; }
.smp-dim code { font-size: 11px; color: var(--accent-light); }
.smp-form { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 8px; }
.smp-field { display: flex; flex-direction: column; gap: 4px; }
.smp-field-wide { grid-column: 1 / -1; }
.smp-label { font-size: 12px; color: var(--text-secondary); }
.smp-w-full { width: 100%; }
.smp-meta, .smp-export { display: flex; flex-direction: column; gap: 6px; padding-top: 10px; border-top: 1px solid var(--border-light); }
.smp-meta-rows { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
.smp-meta-row { display: flex; align-items: center; gap: 6px; }
.smp-meta-key { flex: 0 0 38%; }
.smp-meta-value { flex: 1 1 auto; min-width: 0; }
.smp-meta-actions, .smp-actions, .smp-export-actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.smp-actions { padding-top: 4px; }
.smp-notice { margin: 0; font-size: 12px; color: var(--success); }
.smp-danger { margin: 0; font-size: 12px; color: var(--danger); }
.smp-export-meta { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 10px; font-size: 12px; color: var(--text-muted); }
.smp-export-body { margin: 0; max-height: 220px; overflow: auto; padding: 8px; font-size: 11px; line-height: 1.5; color: var(--text-secondary); background: var(--bg-secondary); border: 1px solid var(--border-light); border-radius: var(--radius-sm); white-space: pre; }
</style>
