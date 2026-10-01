<!--
  知识库云盘面板（右栏）
  职责：文档列表、分类树、标签云、版本历史、搜索、上传
-->
<template>
  <aside
    class="ws-panel ws-panel-right"
    :class="{ collapsed: collapsed }"
  >
    <div class="ws-panel-header">
      <button class="ws-panel-toggle" @click="$emit('toggle-collapse')" :title="collapsed ? '展开' : '收起'">
        <el-icon v-if="!collapsed"><ArrowRight /></el-icon>
        <el-icon v-else><ArrowLeft /></el-icon>
      </button>
      <span v-if="!collapsed" class="ws-panel-title">
        <span class="ws-panel-icon">📚</span>
        知识库云盘
      </span>
    </div>

    <div v-if="!collapsed" class="ws-panel-body">
      <!-- Tab 切换 -->
      <div class="ws-kb-tabs">
        <button
          v-for="tab in kbTabs"
          :key="tab.key"
          class="ws-kb-tab"
          :class="{ active: activeKbTab === tab.key }"
          @click="$emit('switch-kb-tab', tab.key)"
        >
          <el-icon><component :is="tab.icon" /></el-icon>
          <span>{{ tab.label }}</span>
        </button>
      </div>

      <!-- 搜索 -->
      <div class="ws-kb-search">
        <el-input v-model="searchQuery" placeholder="搜索文档…" clearable size="small" @keyup.enter="$emit('search-kb')" @clear="$emit('search-kb')">
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
      </div>

      <!-- 文档列表 -->
      <div v-if="activeKbTab === 'docs'" class="ws-kb-docs">
        <!-- 分类树 -->
        <div class="ws-doc-categories">
          <div
            v-for="cat in categories"
            :key="cat.id"
            class="ws-doc-category"
            :class="{ active: activeCategory === cat.id }"
            @click="$emit('select-category', cat)"
          >
            <div class="ws-doc-category-header">
              <el-icon v-if="expandedCategories.includes(cat.id)"><FolderOpened /></el-icon>
              <el-icon v-else><Folder /></el-icon>
              <span class="ws-cat-name">{{ cat.name }}</span>
              <span class="ws-cat-count">{{ cat.count || 0 }}</span>
            </div>
          </div>
        </div>

        <el-divider class="ws-kb-divider" />

        <!-- 文档列表：行形状来自 /kb/documents 的 DocSummary（id/title/category/status/updated_at 五个键） -->
        <el-scrollbar class="ws-doc-scroll">
          <div
            v-for="doc in filteredDocs"
            :key="doc.id"
            class="ws-doc-item"
            :class="{ active: activeDoc?.id === doc.id }"
            @click="$emit('open-doc', doc)"
          >
            <span class="ws-doc-icon">{{ getCategoryIcon(doc.category) }}</span>
            <div class="ws-doc-info">
              <div class="ws-doc-name">{{ doc.title }}</div>
              <div class="ws-doc-meta">
                {{ getCategoryLabel(doc.category) }} · {{ docMetaText(doc) }}
              </div>
            </div>
            <el-tag v-if="isAiAnalyzed(doc.status)" :type="getStatusType(doc.status)" size="small" effect="plain" round>
              {{ getStatusLabel(doc.status) }}
            </el-tag>
          </div>
          <el-empty v-if="filteredDocs.length === 0 && docsLoading" description="加载中…" :image-size="40" />
          <el-empty v-else-if="filteredDocs.length === 0" description="暂无文档" :image-size="40" />
        </el-scrollbar>
      </div>

      <!-- 标签云 -->
      <div v-if="activeKbTab === 'tags'" class="ws-kb-tags">
        <div class="ws-tag-section-title">热门标签</div>
        <div class="ws-tag-cloud">
          <span
            v-for="tag in popularTags"
            :key="tag.name"
            class="ws-tag-cloud-item"
            :style="{ fontSize: tag.fontSize + 'px' }"
            @click="$emit('filter-by-tag', tag)"
          >
            {{ tag.name }}
            <span class="ws-tag-count">{{ tag.count }}</span>
          </span>
        </div>
      </div>

      <!-- 版本历史 -->
      <div v-if="activeKbTab === 'versions'" class="ws-kb-versions">
        <div class="ws-version-current-doc" v-if="activeDoc">
          <el-icon><Document /></el-icon>
          <span>{{ activeDoc.title }}</span>
        </div>
        <el-scrollbar class="ws-version-scroll">
          <div
            v-for="(ver, idx) in docVersions"
            :key="ver.version"
            class="ws-version-item"
            :class="{ latest: idx === 0 }"
          >
            <div class="ws-version-header">
              <span class="ws-version-badge">{{ idx === 0 ? '当前版本' : '历史版本' }}</span>
              <span class="ws-version-time">{{ formatTime(ver.created_at) }}</span>
            </div>
            <div class="ws-version-label">{{ ver.version }}</div>
            <div class="ws-version-author">{{ ver.title }} · {{ ver.note || '（无备注）' }}</div>
          </div>
          <el-empty v-if="docVersions.length === 0" description="暂无版本记录" :image-size="40" />
        </el-scrollbar>
      </div>

      <!-- 快捷操作 -->
      <div class="ws-kb-actions">
        <el-upload
          :show-file-list="false"
          :before-upload="handleBeforeUpload"
          class="ws-kb-upload"
        >
          <el-button size="small" class="ws-kb-action-btn">
            <el-icon><Upload /></el-icon>
            上传文档
          </el-button>
        </el-upload>
        <el-button size="small" type="primary" class="ws-kb-action-btn" @click="$emit('create-doc')">
          <el-icon><Edit /></el-icon>
          新建
        </el-button>
      </div>
    </div>

    <!-- 折叠状态图标 -->
    <div v-else class="ws-collapsed-icons ws-collapsed-right">
      <button class="ws-collapsed-icon-btn" title="知识库" @click="$emit('expand-and-switch', 'docs')">
        <span>📚</span>
      </button>
      <button class="ws-collapsed-icon-btn" title="标签" @click="$emit('expand-and-switch', 'tags')">
        <span>🏷️</span>
      </button>
      <button class="ws-collapsed-icon-btn" title="版本" @click="$emit('expand-and-switch', 'versions')">
        <span>📋</span>
      </button>
    </div>
  </aside>
</template>

<script setup>
import { computed, ref } from 'vue'
import {
  Search, ArrowLeft, ArrowRight, Folder, FolderOpened,
  Document, Upload, Edit
} from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus/es/components/message/index'
import { getCategoryIcon, getCategoryLabel, getStatusType, getStatusLabel, isAiAnalyzed } from '@/utils'

const props = defineProps({
  collapsed: { type: Boolean, default: false },
  activeKbTab: { type: String, default: 'docs' },
  categories: { type: Array, default: () => [] },
  documents: { type: Array, default: () => [] },
  popularTags: { type: Array, default: () => [] },
  docVersions: { type: Array, default: () => [] },
  activeDoc: { type: Object, default: null },
  activeCategory: { type: String, default: null },
  expandedCategories: { type: Array, default: () => [] },
  docsLoading: { type: Boolean, default: false }
})

defineEmits([
  'toggle-collapse', 'switch-kb-tab', 'search-kb', 'select-category',
  'open-doc', 'filter-by-tag', 'create-doc', 'expand-and-switch'
])

const searchQuery = ref('')

const kbTabs = [
  { key: 'docs', icon: 'Document', label: '文档' },
  { key: 'tags', icon: 'CollectionTag', label: '标签' },
  { key: 'versions', icon: 'RefreshRight', label: '版本' }
]

const filteredDocs = computed(() => {
  let list = props.documents
  // 分类键是 doc.category（值＝cat-tech 这类 id），后端没有 category_id
  if (props.activeCategory) {
    list = list.filter(d => d.category === props.activeCategory)
  }
  if (searchQuery.value) {
    const kw = searchQuery.value.toLowerCase()
    list = list.filter(d =>
      (d.title || '').toLowerCase().includes(kw) ||
      (d.tags || []).some(t => (t || '').toLowerCase().includes(kw))
    )
  }
  return list
})

// 检索行（SearchHit：snippet/score，没有 updated_at）与列表行（DocSummary：updated_at）共用一行，
// 所以这里按"这一行有没有时间"分支，而不是给不存在的 size/graph_linked 编个占位值。
function docMetaText(doc) {
  if (doc.updated_at) return formatTime(doc.updated_at)
  if (typeof doc.score === 'number') return `相关度 ${doc.score.toFixed(2)}`
  return '—'
}

// 旧副本拿 Date.now() 去减后端下发的 RFC3339 字符串 ⇒ NaN ⇒ 所有 diff < X 恒假 ⇒ "刚刚/N 分钟前/N 小时前"
// 三档一次也没触发过，界面只会掉到 M/D。现在相对档与绝对档都由 utils/time.js 判（它先 timeValue 再减）。
import { timeAgoOrDate as formatTime } from '@/utils'

function handleBeforeUpload(file) {
  ElMessage.info(`正在上传：${file.name}`)
  return false
}
</script>
