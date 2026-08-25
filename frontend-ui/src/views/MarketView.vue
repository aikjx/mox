<template>
  <div class="market">
    <div class="head">
      <div>
        <h2 class="page-title">算子商城</h2>
        <p class="page-subtitle">把「需求 + 业务流程图」作为算子包沉淀下来，别人可随机浏览、克隆后继续编辑</p>
      </div>
      <div class="head-actions">
        <el-button @click="randomOne" :loading="randoming">
          <el-icon><MagicStick /></el-icon> 随机一个
        </el-button>
        <el-button type="primary" @click="showUpload = true">
          <el-icon><Upload /></el-icon> 上传算子包
        </el-button>
      </div>
    </div>

    <!-- 过滤条 -->
    <div class="toolbar">
      <el-input
        v-model="kw"
        placeholder="搜索名称 / 简介…（最近 5 条）"
        clearable
        :prefix-icon="Search"
        style="width: 300px"
        @change="pushHist('market', kw)"
        @keyup.enter="pushHist('market', kw)"
      />
      <el-popover v-if="marketSearchHistory.length" placement="bottom" trigger="click" width="300">
        <template #reference>
          <el-button plain :icon="Clock">历史</el-button>
        </template>
        <div class="hist-list">
          <div
            v-for="h in marketSearchHistory"
            :key="h"
            class="hist-item"
            @click="kw = h; pushHist('market', h)"
          >
            <el-icon><Clock /></el-icon>{{ h }}
          </div>
          <el-button link size="small" type="danger" @click="marketSearchHistory = clearHist('market')">清空</el-button>
        </div>
      </el-popover>

      <div class="cats">
        <span
          v-for="c in categories"
          :key="c"
          class="cat"
          :class="{ on: cat === c }"
          @click="cat = c"
        >{{ c === 'all' ? '全部' : c }}</span>
      </div>

      <div class="toolbar-right">
        <el-select v-model="sort" placeholder="排序" style="width: 150px">
          <el-option label="最新发布" value="newest" />
          <el-option label="最热门（下载量）" value="hot" />
          <el-option label="评分最高" value="rating" />
        </el-select>
        <el-radio-group v-model="viewMode" size="default" class="view-mode-switch">
          <el-radio-button label="card">
            <el-icon><Shop /></el-icon> 卡片
          </el-radio-button>
          <el-radio-button label="list">
            <el-icon><List /></el-icon> 列表
          </el-radio-button>
        </el-radio-group>
      </div>
    </div>

    <!-- 空态 CTA -->
    <div v-if="!sortedFiltered.length && viewMode === 'card'" class="empty-cta">
      <el-empty description="商城还空着！上传第一个算子包，让同事一键克隆复用" :image-size="90">
        <el-button type="primary" size="large" :icon="Upload" @click="showUpload = true">立即上传第一个算子包</el-button>
      </el-empty>
    </div>

    <!-- 卡片列表视图 -->
    <div v-if="sortedFiltered.length && viewMode === 'card'" class="grid grid-cards">
      <div
        v-for="p in sortedFiltered"
        :key="p.id"
        class="card"
        @click="openDetail(p.id)"
      >
        <div class="card-top">
          <span class="badge primary">{{ p.category || '未分类' }}</span>
          <span class="ver">v{{ p.version }}</span>
        </div>
        <h3 class="card-name">{{ p.name }}</h3>
        <p class="card-summary">{{ p.summary || '暂无简介' }}</p>
        <div class="card-tags">
          <span v-for="t in (p.tags || []).slice(0,5)" :key="t" class="tag">{{ t }}</span>
        </div>
        <div class="card-meta">
          <span title="克隆次数"><el-icon><CopyDocument /></el-icon> {{ p.clone_count || 0 }}</span>
          <span title="节点数"><el-icon><Connection /></el-icon> {{ p.node_count || 0 }}</span>
          <span title="功能点"><el-icon><List /></el-icon> {{ p.feature_count || 0 }}</span>
          <span title="下载量"><el-icon><VideoPlay /></el-icon> {{ p.downloads || 0 }}</span>
          <span title="评分"><el-icon><Star /></el-icon> {{ (p.rating || 0).toFixed(1) }}</span>
        </div>
        <div class="card-foot">
          <span class="author">{{ p.author || '匿名' }}</span>
          <el-button size="small" type="primary" plain @click.stop="clonePkg(p.id)">
            <el-icon><CopyDocument /></el-icon> 克隆编辑
          </el-button>
        </div>
      </div>
    </div>

    <!-- 列表视图（企业用户看属性的高频视图） -->
    <el-table
      v-if="sortedFiltered.length && viewMode === 'list'"
      :data="sortedFiltered"
      stripe
      style="width: 100%"
      @row-click="(r) => openDetail(r.id)"
    >
      <el-table-column prop="name" label="算子包名称" min-width="240">
        <template #default="{ row }">
          <div class="list-name">
            <b>{{ row.name }}</b>
            <el-tag size="small" effect="plain">{{ row.category || '未分类' }}</el-tag>
          </div>
          <div class="list-sub">{{ row.summary || '暂无简介' }}</div>
        </template>
      </el-table-column>
      <el-table-column label="标签" min-width="220">
        <template #default="{ row }">
          <el-tag v-for="t in (row.tags || []).slice(0,4)" :key="t" size="small" style="margin-right:4px" type="info" effect="plain">{{ t }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="版本 / 作者" min-width="180">
        <template #default="{ row }">v{{ row.version }} · {{ row.author || '匿名' }}</template>
      </el-table-column>
      <el-table-column prop="downloads" label="下载量" sortable width="110" align="right" />
      <el-table-column prop="rating" label="评分" sortable width="110" align="right">
        <template #default="{ row }">{{ (row.rating || 0).toFixed(1) }}</template>
      </el-table-column>
      <el-table-column prop="clone_count" label="克隆" sortable width="90" align="right" />
      <el-table-column label="操作" width="140" align="right">
        <template #default="{ row }">
          <el-button size="small" type="primary" link @click.stop="openDetail(row.id)">查看</el-button>
          <el-button size="small" link @click.stop="clonePkg(row.id)">克隆</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 上传弹窗（3 步骤 = 企业最优：基础信息 → 详细需求 → 元信息） -->
    <el-dialog v-model="showUpload" title="上传算子包（3 步完成）" width="620px" :close-on-click-modal="false" @closed="resetForm" @open="uploadStep = 0">
      <el-steps :active="uploadStep" finish-status="success" align-center style="margin-bottom: 16px;">
        <el-step title="基础信息" description="名称、分类、标签" />
        <el-step title="详细需求" description="这是最核心一步" />
        <el-step title="元信息" description="版本、下载量、评分" />
      </el-steps>

      <el-form ref="uploadFormRef" label-width="84px" :model="form" :rules="uploadFormRules">
        <template v-if="uploadStep === 0">
          <el-form-item label="名称" prop="name">
            <el-input v-model="form.name" maxlength="60" show-word-limit placeholder="算子包名称（2~60 字）" />
          </el-form-item>
          <el-form-item label="分类" prop="category">
            <el-input v-model="form.category" maxlength="32" placeholder="如：平台/编排（可自定义）" />
          </el-form-item>
          <el-form-item label="作者" prop="author">
            <el-input v-model="form.author" maxlength="32" placeholder="你的名字" />
          </el-form-item>
          <el-form-item label="标签" prop="tags">
            <el-select v-model="form.tags" multiple filterable allow-create default-first-option placeholder="可输入新建标签（单条 ≤16 字，最多 10 条）" style="width:100%">
              <el-option v-for="t in tagOptions" :key="t" :label="t" :value="t" />
            </el-select>
          </el-form-item>
        </template>

        <template v-if="uploadStep === 1">
          <el-alert type="info" :closable="false" style="margin-bottom: 14px;" show-icon>
            需求描述写得越清晰，别人克隆后理解越快；这是最关键的字段。
          </el-alert>
          <el-form-item label="简介" prop="summary">
            <el-input v-model="form.summary" type="textarea" :rows="2" maxlength="240" show-word-limit placeholder="一句话说明这个算子包解决什么" />
          </el-form-item>
          <el-form-item label="需求描述" prop="requirement">
            <el-input
              v-model="form.requirement"
              type="textarea"
              :rows="6"
              maxlength="3000"
              show-word-limit
              placeholder="★ 这是最核心的部分：把需求写清楚，其他（流程图/功能）都可据此快速调整"
            />
          </el-form-item>
        </template>

        <template v-if="uploadStep === 2">
          <el-form-item label="版本号" prop="version">
            <el-input v-model="form.version" maxlength="16" placeholder="语义化版本，如 1.0.0" />
          </el-form-item>
          <el-form-item label="下载量" prop="downloads">
            <el-input-number v-model="form.downloads" :min="0" :max="1000000000" :step="1" style="width:100%" />
          </el-form-item>
          <el-form-item label="初始评分" prop="rating">
            <el-rate v-model="form.rating" show-score text-color="#64748b" score-template="{value}" />
          </el-form-item>
        </template>
      </el-form>

      <template #footer>
        <el-button @click="showUpload = false">取消</el-button>
        <el-button v-if="uploadStep > 0" @click="uploadStep--">上一步</el-button>
        <el-button v-if="uploadStep < 2" type="primary" @click="nextStep()">下一步</el-button>
        <el-button v-if="uploadStep === 2" type="primary" :loading="uploading" @click="doUpload">提交上传</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import { Search, Upload, MagicStick, CopyDocument, Share, Connection, List, Clock, Star, VideoPlay, Shop } from '@element-plus/icons-vue'
import { marketList, marketRandom, marketUpload, marketClone } from '@/api'
import { pushSearchHistory, getSearchHistory } from '@/globalShortcuts'

const router = useRouter()
const route = useRoute()
const packages = ref([])
const kw = ref('')
const cat = ref('all')
const sort = ref('newest')       // 排序：newest / hot / rating
const viewMode = ref('card')     // 视图：card / list
const randoming = ref(false)
const uploading = ref(false)
const showUpload = ref(false)
const uploadStep = ref(0)        // 上传步骤 0/1/2
const marketSearchHistory = ref([])  // 搜索历史

const uploadFormRef = ref(null)
const tagOptions = ['编排', '流程图', '企业级', 'AI', '数据', '算法', '可视化']
const form = ref(emptyForm())

// 初始化搜索历史（onMounted 后读 localStorage，SSR 安全）
onMounted(() => {
  marketSearchHistory.value = getSearchHistory('market', 5)
  // URL 预填：?q=xxx / ?viewMode=list 等（可刷新不丢失）
  if (route.query?.q) kw.value = String(route.query.q)
  if (route.query?.viewMode === 'list' || route.query?.viewMode === 'card') {
    viewMode.value = String(route.query.viewMode)
  }
  if (route.query?.sort) sort.value = String(route.query.sort)
  // Query 驱动无状态化：/market?action=upload 自动开上传表单
  window.addEventListener('mox:open-market-upload', _onMarketUploadCmd)
})
onBeforeUnmount(() => {
  window.removeEventListener('mox:open-market-upload', _onMarketUploadCmd)
})
function _onMarketUploadCmd() { showUpload.value = true }

function emptyForm() {
  return {
    name: '', category: '', author: '', summary: '', tags: [], requirement: '',
    version: '1.0.0', downloads: 0, rating: 0
  }
}
function resetForm() {
  form.value = emptyForm()
  uploadStep.value = 0
  uploadFormRef.value?.clearValidate?.()
}

// 搜索历史存取（包装 globalShortcuts，保持 API 简洁）
function pushHist(key, term) {
  marketSearchHistory.value = pushSearchHistory(key, term, 5)
  // 同步写入 URL query：刷新不丢失搜索意图
  router.replace({
    path: route.path,
    query: { ...route.query, q: term || undefined }
  })
}
function clearHist(key) {
  pushSearchHistory(key, '__CLEAR__', 0)
  marketSearchHistory.value = []
  return []
}

// 3 步上传表单：切换步长前先 validate 对应字段
async function nextStep() {
  if (!uploadFormRef.value) { uploadStep.value++; return }
  let fields = []
  if (uploadStep.value === 0) fields = ['name', 'category', 'author', 'tags']
  if (uploadStep.value === 1) fields = ['summary', 'requirement']
  if (uploadStep.value === 2) fields = ['version', 'downloads']
  try {
    if (fields.length) await uploadFormRef.value.validate(fields)
    uploadStep.value = Math.min(2, uploadStep.value + 1)
  } catch (_e) {
    ElMessage.warning('请先修复当前步骤中的高亮字段')
  }
}

// 商城上传表单校验：字段长度、必填、枚举数量、数值范围
const uploadFormRules = {
  name: [
    { required: true, message: '请填写算子包名称', trigger: 'blur' },
    { min: 2, max: 60, message: '名称长度 2~60 字符', trigger: 'blur' }
  ],
  category: [{ max: 32, message: '分类最多 32 字符', trigger: 'blur' }],
  author: [{ max: 32, message: '作者最多 32 字符', trigger: 'blur' }],
  summary: [{ max: 240, message: '简介最多 240 字符', trigger: 'blur' }],
  requirement: [
    { required: true, message: '需求描述不能为空（这是最核心的）', trigger: 'blur' },
    { min: 8, max: 3000, message: '需求描述长度 8~3000 字符', trigger: 'blur' }
  ],
  tags: [
    {
      type: 'array',
      validator: (_r, value, cb) => {
        if (!Array.isArray(value)) return cb()
        if (value.length > 10) return cb(new Error('最多选择 10 个标签'))
        for (const t of value) {
          if (!t || String(t).length > 16) return cb(new Error('每条标签长度应 ≤16 字符'))
        }
        cb()
      },
      trigger: 'change'
    }
  ],
  version: [{ max: 16, message: '版本号最多 16 字符', trigger: 'blur' }],
  downloads: [
    {
      validator: (_r, value, cb) =>
        value == null || (typeof value === 'number' && value >= 0 && value <= 1_000_000_000)
          ? cb()
          : cb(new Error('下载量取值范围 0~1000000000')),
      trigger: 'change'
    }
  ]
}

const categories = computed(() => {
  const set = new Set(packages.value.map((p) => p.category || '未分类'))
  return ['all', ...Array.from(set)]
})
const filtered = computed(() => {
  const pkgs = Array.isArray(packages.value) ? packages.value : []
  const k = kw.value.trim().toLowerCase()
  return pkgs.filter((p) => {
    const safe = p || {}
    const matchK = !k ||
      String(safe.name || '').toLowerCase().includes(k) ||
      String(safe.summary || '').toLowerCase().includes(k)
    const matchC = cat.value === 'all' || (safe.category || '未分类') === cat.value
    return matchK && matchC
  })
})
const sortedFiltered = computed(() => {
  const arr = filtered.value ? filtered.value.slice() : []
  const s = sort.value
  if (s === 'hot') arr.sort((a, b) => (b.downloads || 0) - (a.downloads || 0))
  else if (s === 'rating') arr.sort((a, b) => (b.rating || 0) - (a.rating || 0))
  else arr.sort((a, b) => new Date(b.updated_at || b.created_at || 0) - new Date(a.updated_at || a.created_at || 0))
  return arr
})

// viewMode / sort 变化 → 同步到 URL query（刷新保留）
watch([viewMode, sort], ([vm, st]) => {
  router.replace({
    path: route.path,
    query: { ...route.query, viewMode: vm, sort: st }
  })
})

async function load() {
  try {
    const r = await marketList()
    // 兼容：后端 /market 曾返回 Object { packages: [] } 或 Array 直出
    packages.value = Array.isArray(r) ? r : (r?.packages || [])
  } catch (e) {
    ElMessage.error('加载商城失败：' + e.message)
  }
}
function openDetail(id) {
  router.push(`/market/${id}`)
}
async function clonePkg(id) {
  try {
    const r = await marketClone(id)
    // 契约兼容：后端 market clone 已解包 {success:true, data: cloneItem}，因此 r 即为克隆项
    const cloned = r && r.id ? r : (r?.package || r)
    if (!cloned?.id) throw new Error('后端未返回克隆结果 id')
    ElMessage.success('已克隆，进入编辑')
    router.push(`/market/${cloned.id}`)
  } catch (e) {
    ElMessage.error(e.message)
  }
}
async function randomOne() {
  randoming.value = true
  try {
    const r = await marketRandom()
    // /market/random 已解包为数组（5 个），取首个推送详情
    const list = Array.isArray(r) ? r : (r?.package ? [r.package] : [])
    if (!list.length) {
      ElMessage.info('暂无算子包')
      return
    }
    router.push(`/market/${list[0].id}`)
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    randoming.value = false
  }
}
async function doUpload() {
  try {
    await uploadFormRef.value?.validate()
  } catch (_e) {
    ElMessage.warning('请先修复表单中的高亮错误')
    return
  }
  uploading.value = true
  try {
    // 表单 -> API DTO 映射：统一字段类型与默认值
    const payload = {
      name: String(form.value.name || '').trim(),
      category: String(form.value.category || 'general').trim() || 'general',
      author: String(form.value.author || '').trim(),
      summary: String(form.value.summary || '').trim(),
      // tags 数组：去重 + 转字符串 + 修剪
      tags: Array.from(new Set((form.value.tags || []).map((t) => String(t || '').trim()).filter(Boolean))),
      requirement: String(form.value.requirement || '').trim(),
      version: String(form.value.version || '1.0.0').trim() || '1.0.0',
      // Number 强制：el-input-number 已是 Number，但再次保证后端入库类型
      downloads: form.value.downloads == null ? 0 : Number(form.value.downloads),
      rating: 0
    }
    const r = await marketUpload(payload)
    // 解包后 r 即后端 data=uploadedItem（含 id）
    const id = r?.id
    if (!id) throw new Error('上传未返回 id')
    ElMessage.success('上传成功')
    showUpload.value = false
    await load()
    router.push(`/market/${id}`)
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    uploading.value = false
  }
}

onMounted(load)
</script>

<style scoped>
.market { display: flex; flex-direction: column; gap: 16px; }
.head { display: flex; align-items: center; justify-content: space-between; }
.head-actions { display: flex; gap: 10px; }
.toolbar { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; }
.cats { display: flex; gap: 6px; flex-wrap: wrap; }
.cat {
  font-size: 12px; padding: 4px 12px; border-radius: 999px;
  background: var(--bg-page); color: var(--text-2); cursor: pointer; transition: all 0.15s;
}
.cat:hover { color: var(--brand); }
.cat.on { background: var(--brand); color: #fff; }
.grid-cards {
  display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 16px;
}
.card {
  background: #fff; border: 1px solid var(--border); border-radius: 14px; padding: 16px 18px;
  cursor: pointer; transition: all 0.2s; display: flex; flex-direction: column; gap: 10px;
}
.card:hover { box-shadow: 0 12px 30px rgba(15, 23, 42, 0.1); transform: translateY(-2px); border-color: var(--brand-light); }
.card-top { display: flex; justify-content: space-between; align-items: center; }
.ver { font-size: 11px; color: var(--text-3); }
.card-name { font-size: 15px; font-weight: 700; color: var(--text-1); }
.card-summary { font-size: 12px; color: var(--text-3); line-height: 1.6; min-height: 38px; }
.card-tags { display: flex; gap: 5px; flex-wrap: wrap; }
.tag { font-size: 11px; padding: 2px 8px; border-radius: 6px; background: var(--brand-soft); color: var(--brand-dark); }
.card-meta { display: flex; gap: 14px; font-size: 12px; color: var(--text-2); }
.card-meta span { display: inline-flex; align-items: center; gap: 4px; }
.card-foot { display: flex; justify-content: space-between; align-items: center; border-top: 1px solid var(--border-light); padding-top: 10px; }
.author { font-size: 12px; color: var(--text-3); }
.hint { font-size: 12px; color: var(--text-3); line-height: 1.5; }
.badge { font-size: 11px; padding: 2px 9px; border-radius: 999px; }
.badge.primary { background: var(--brand); color: #fff; }
</style>
