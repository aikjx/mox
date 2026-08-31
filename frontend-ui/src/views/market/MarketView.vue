<template>
  <div class="page-container">
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">算子商城</h2>
        <p class="page-subtitle">把「需求 + 业务流程图」作为算子包沉淀下来，别人可随机浏览、克隆后继续编辑</p>
      </div>
      <div class="page-header-actions">
        <!-- 审核模式开关（管理员可见） -->
        <el-tooltip v-if="appStore.isAdmin" content="审核模式（管理员）" placement="bottom">
          <el-switch
            v-model="appStore.marketReviewEnabled"
            active-text="审核开"
            inactive-text="审核关"
            @change="onReviewModeChange"
          />
        </el-tooltip>
        <el-button @click="randomOne" :loading="randoming">
          <el-icon><MagicStick /></el-icon> 随机一个
        </el-button>
        <el-button type="primary" @click="showUpload = true">
          <el-icon><Upload /></el-icon> 上传算子包
        </el-button>
      </div>
    </div>

    <!-- 主 Tab：商城列表 / 审核管理 -->
    <el-tabs v-model="mainTab" class="main-tabs">
      <el-tab-pane label="算子商城" name="market">
        <div class="page-toolbar">
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
            <!-- 审核状态筛选 -->
            <el-select v-model="statusFilter" placeholder="审核状态" style="width: 130px" size="default">
              <el-option label="全部状态" value="all" />
              <el-option label="已上架" value="approved" />
              <el-option label="待审核" value="pending" />
              <el-option label="已驳回" value="rejected" />
            </el-select>
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

        <div class="page-content">
        <!-- 审核模式开启提示 -->
        <el-alert
          v-if="appStore.marketReviewEnabled"
          type="info"
          :closable="false"
          show-icon
          style="margin-bottom: 12px"
        >
          商城审核模式已开启：上传的算子包需审核通过后才会上架。
        </el-alert>

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
            :class="{ 'card-pending': p.review_status === 'pending', 'card-rejected': p.review_status === 'rejected' }"
            @click="openDetail(p.id)"
          >
            <div class="card-top">
              <span class="badge primary">{{ p.category || '未分类' }}</span>
              <div class="card-top-right">
                <!-- 审核状态标签 -->
                <el-tag v-if="p.review_status === 'pending'" type="warning" size="small" effect="light">待审核</el-tag>
                <el-tag v-else-if="p.review_status === 'rejected'" type="danger" size="small" effect="light">已驳回</el-tag>
                <el-tag v-else type="success" size="small" effect="plain">已上架</el-tag>
                <span class="ver">v{{ p.version }}</span>
              </div>
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
              <el-button
                size="small"
                type="primary"
                plain
                :disabled="p.review_status === 'pending' || p.review_status === 'rejected'"
                @click.stop="clonePkg(p.id)"
              >
                <el-icon><CopyDocument /></el-icon> 克隆编辑
              </el-button>
            </div>
            <!-- 驳回原因展示 -->
            <div v-if="p.review_status === 'rejected' && p.reject_reason" class="card-reject-reason">
              <el-icon><WarningFilled /></el-icon> 驳回原因：{{ p.reject_reason }}
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
                <!-- 审核状态标签 -->
                <el-tag v-if="row.review_status === 'pending'" type="warning" size="small" effect="light">待审核</el-tag>
                <el-tag v-else-if="row.review_status === 'rejected'" type="danger" size="small" effect="light">已驳回</el-tag>
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
          <el-table-column label="审核状态" width="110" align="center">
            <template #default="{ row }">
              <el-tag v-if="row.review_status === 'pending'" type="warning" size="small">待审核</el-tag>
              <el-tag v-else-if="row.review_status === 'rejected'" type="danger" size="small">已驳回</el-tag>
              <el-tag v-else type="success" size="small">已上架</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="downloads" label="下载量" sortable width="110" align="right" />
          <el-table-column prop="rating" label="评分" sortable width="110" align="right">
            <template #default="{ row }">{{ (row.rating || 0).toFixed(1) }}</template>
          </el-table-column>
          <el-table-column prop="clone_count" label="克隆" sortable width="90" align="right" />
          <el-table-column label="操作" width="180" align="right">
            <template #default="{ row }">
              <el-button size="small" type="primary" link @click.stop="openDetail(row.id)">查看</el-button>
              <el-button
                size="small"
                link
                :disabled="row.review_status === 'pending' || row.review_status === 'rejected'"
                @click.stop="clonePkg(row.id)"
              >克隆</el-button>
            </template>
          </el-table-column>
        </el-table>
        </div>
      </el-tab-pane>

      <!-- 审核管理 Tab（仅管理员可见） -->
      <el-tab-pane v-if="appStore.isAdmin" label="审核管理" name="review">
        <div class="page-toolbar">
          <div class="review-stats">
            <el-statistic title="待审核" :value="pendingCount" value-color="#f59e0b" />
            <el-statistic title="已通过" :value="approvedCount" value-color="#10b981" />
            <el-statistic title="已驳回" :value="rejectedCount" value-color="#ef4444" />
          </div>
          <div class="toolbar-right">
            <el-select v-model="reviewStatusFilter" style="width: 140px">
              <el-option label="全部" value="all" />
              <el-option label="待审核" value="pending" />
              <el-option label="已通过" value="approved" />
              <el-option label="已驳回" value="rejected" />
            </el-select>
            <el-button @click="load">
              <el-icon><Refresh /></el-icon> 刷新
            </el-button>
          </div>
        </div>

        <div class="page-content">
          <el-table :data="filteredReviewList" stripe style="width: 100%">
            <el-table-column prop="name" label="算子包名称" min-width="200">
              <template #default="{ row }">
                <b>{{ row.name }}</b>
                <div class="list-sub">{{ row.summary || '暂无简介' }}</div>
              </template>
            </el-table-column>
            <el-table-column prop="category" label="分类" width="100" />
            <el-table-column prop="author" label="作者" width="100" />
            <el-table-column prop="version" label="版本" width="90" />
            <el-table-column label="审核状态" width="110" align="center">
              <template #default="{ row }">
                <el-tag v-if="row.review_status === 'pending'" type="warning" size="small">待审核</el-tag>
                <el-tag v-else-if="row.review_status === 'rejected'" type="danger" size="small">已驳回</el-tag>
                <el-tag v-else type="success" size="small">已通过</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="提交时间" width="170">
              <template #default="{ row }">{{ formatDate(row.created_at || row.updated_at) }}</template>
            </el-table-column>
            <el-table-column label="审核时间" width="170">
              <template #default="{ row }">{{ row.reviewed_at ? formatDate(row.reviewed_at) : '—' }}</template>
            </el-table-column>
            <el-table-column label="驳回原因" min-width="160">
              <template #default="{ row }">
                <span v-if="row.reject_reason" :title="row.reject_reason" class="reject-reason-text">{{ row.reject_reason }}</span>
                <span v-else>—</span>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="200" align="center" fixed="right">
              <template #default="{ row }">
                <template v-if="row.review_status === 'pending'">
                  <el-button size="small" type="success" link @click.stop="approvePkg(row)">
                    <el-icon><Check /></el-icon> 通过
                  </el-button>
                  <el-button size="small" type="danger" link @click.stop="rejectPkg(row)">
                    <el-icon><Close /></el-icon> 驳回
                  </el-button>
                </template>
                <template v-else>
                  <el-button size="small" link @click.stop="openDetail(row.id)">查看</el-button>
                  <el-button v-if="row.review_status === 'rejected'" size="small" type="warning" link @click.stop="approvePkg(row)">
                    重新通过
                  </el-button>
                </template>
              </template>
            </el-table-column>
          </el-table>
          <el-empty v-if="!filteredReviewList.length" description="暂无审核记录" :image-size="60" />
        </div>
      </el-tab-pane>
    </el-tabs>

    <!-- 上传弹窗（3 步骤 = 企业最优：基础信息 → 详细需求 → 元信息） -->
    <el-dialog v-model="showUpload" title="上传算子包（3 步完成）" width="620px" :close-on-click-modal="false" @closed="resetForm" @open="uploadStep = 0">
      <el-steps :active="uploadStep" finish-status="success" align-center style="margin-bottom: 16px;">
        <el-step title="基础信息" description="名称、分类、标签" />
        <el-step title="详细需求" description="这是最核心一步" />
        <el-step title="元信息" description="版本、下载量、评分" />
      </el-steps>

      <!-- 审核模式提示 -->
      <el-alert v-if="appStore.marketReviewEnabled" type="info" :closable="false" show-icon style="margin-bottom: 16px">
        当前已开启审核模式，提交后需管理员审核通过才会上架。
      </el-alert>

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
        <el-button v-if="uploadStep === 2" type="primary" :loading="uploading" @click="doUpload">
          {{ appStore.marketReviewEnabled ? '提交审核' : '提交上传' }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 驳回弹窗 -->
    <el-dialog v-model="showRejectDialog" title="驳回算子包" width="460px" :close-on-click-modal="false">
      <el-alert type="warning" :closable="false" show-icon style="margin-bottom: 16px">
        请填写驳回理由，提交后作者将收到通知。
      </el-alert>
      <el-form label-width="80px">
        <el-form-item label="算子包">
          <span>{{ rejectingPkg?.name }}</span>
        </el-form-item>
        <el-form-item label="驳回理由" required>
          <el-input
            v-model="rejectReason"
            type="textarea"
            :rows="4"
            maxlength="500"
            show-word-limit
            placeholder="请说明驳回原因，帮助作者改进"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showRejectDialog = false">取消</el-button>
        <el-button type="danger" :disabled="!rejectReason.trim()" @click="confirmReject">确认驳回</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Search, Upload, MagicStick, CopyDocument, Connection, List, Clock, Star,
  VideoPlay, Shop, Refresh, Check, Close, WarningFilled
} from '@element-plus/icons-vue'
import { marketList, marketRandom, marketUpload, marketClone } from '@/api'
import { pushSearchHistory, getSearchHistory } from '@/globalShortcuts'
import { useAppStore } from '@/stores/app.store'

const appStore = useAppStore()
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

// 审核相关
const mainTab = ref('market')
const statusFilter = ref('all')  // 商城列表的审核状态筛选
const reviewStatusFilter = ref('pending') // 审核列表的状态筛选
const showRejectDialog = ref(false)
const rejectingPkg = ref(null)
const rejectReason = ref('')

const uploadFormRef = ref(null)
const tagOptions = ['编排', '流程图', '企业级', 'AI', '数据', '算法', '可视化']
const form = ref(emptyForm())

// 初始化
onMounted(() => {
  // 加载 app store 中的审核配置
  appStore.loadMarketReview?.()

  marketSearchHistory.value = getSearchHistory('market', 5)
  if (route.query?.q) kw.value = String(route.query.q)
  if (route.query?.viewMode === 'list' || route.query?.viewMode === 'card') {
    viewMode.value = String(route.query.viewMode)
  }
  if (route.query?.sort) sort.value = String(route.query.sort)
  if (route.query?.tab === 'review' && appStore.isAdmin) {
    mainTab.value = 'review'
  }
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

function onReviewModeChange(val) {
  ElMessage.success(val ? '审核模式已开启' : '审核模式已关闭')
}

// 搜索历史存取
function pushHist(key, term) {
  marketSearchHistory.value = pushSearchHistory(key, term, 5)
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

// 3 步上传表单校验
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

// 商城上传表单校验规则
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

// 过滤：关键词 + 分类 + 审核状态
const filtered = computed(() => {
  const pkgs = Array.isArray(packages.value) ? packages.value : []
  const k = kw.value.trim().toLowerCase()
  return pkgs.filter((p) => {
    const safe = p || {}
    const matchK = !k ||
      String(safe.name || '').toLowerCase().includes(k) ||
      String(safe.summary || '').toLowerCase().includes(k)
    const matchC = cat.value === 'all' || (safe.category || '未分类') === cat.value
    // 审核状态筛选
    const status = safe.review_status || 'approved'
    const matchS = statusFilter.value === 'all' || status === statusFilter.value
    return matchK && matchC && matchS
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

// 审核统计
const pendingCount = computed(() =>
  packages.value.filter((p) => p.review_status === 'pending').length
)
const approvedCount = computed(() =>
  packages.value.filter((p) => p.review_status === 'approved' || !p.review_status).length
)
const rejectedCount = computed(() =>
  packages.value.filter((p) => p.review_status === 'rejected').length
)

// 审核列表过滤
const filteredReviewList = computed(() => {
  const list = packages.value || []
  if (reviewStatusFilter.value === 'all') return list
  if (reviewStatusFilter.value === 'approved') {
    return list.filter((p) => p.review_status === 'approved' || !p.review_status)
  }
  return list.filter((p) => p.review_status === reviewStatusFilter.value)
})

// viewMode / sort 变化 → 同步到 URL query
watch([viewMode, sort], ([vm, st]) => {
  router.replace({
    path: route.path,
    query: { ...route.query, viewMode: vm, sort: st }
  })
})

watch(mainTab, (tab) => {
  router.replace({
    path: route.path,
    query: { ...route.query, tab }
  })
})

function formatDate(val) {
  if (!val) return '—'
  try {
    const d = new Date(val)
    return d.toLocaleString('zh-CN', {
      year: 'numeric', month: '2-digit', day: '2-digit',
      hour: '2-digit', minute: '2-digit'
    })
  } catch (e) {
    return val
  }
}

async function load() {
  try {
    const r = await marketList()
    let list = Array.isArray(r) ? r : (r?.packages || [])
    // 为每个包补充默认审核状态（兼容旧数据）
    list = list.map((p) => ({
      ...p,
      review_status: p.review_status || 'approved'
    }))
    packages.value = list
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
    const list = Array.isArray(r) ? r : (r?.package ? [r.package] : [])
    // 只随机已上架的
    const approved = list.filter((p) => !p.review_status || p.review_status === 'approved')
    if (!approved.length) {
      ElMessage.info('暂无已上架的算子包')
      return
    }
    router.push(`/market/${approved[0].id}`)
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
    const reviewEnabled = appStore.marketReviewEnabled
    const payload = {
      name: String(form.value.name || '').trim(),
      category: String(form.value.category || 'general').trim() || 'general',
      author: String(form.value.author || '').trim(),
      summary: String(form.value.summary || '').trim(),
      tags: Array.from(new Set((form.value.tags || []).map((t) => String(t || '').trim()).filter(Boolean))),
      requirement: String(form.value.requirement || '').trim(),
      version: String(form.value.version || '1.0.0').trim() || '1.0.0',
      downloads: form.value.downloads == null ? 0 : Number(form.value.downloads),
      rating: 0,
      // 审核模式开启时，状态为待审核
      review_status: reviewEnabled ? 'pending' : 'approved'
    }
    const r = await marketUpload(payload)
    const id = r?.id
    if (!id) throw new Error('上传未返回 id')

    if (reviewEnabled) {
      ElMessage.success('已提交审核，管理员审核通过后将上架')
    } else {
      ElMessage.success('上传成功，已上架')
    }
    showUpload.value = false
    await load()
    router.push(`/market/${id}`)
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    uploading.value = false
  }
}

// ===== 审核操作 =====
function approvePkg(pkg) {
  ElMessageBox.confirm(
    `确定要通过「${pkg.name}」的审核吗？通过后该算子包将正式上架。`,
    '审核通过确认',
    {
      confirmButtonText: '通过',
      cancelButtonText: '取消',
      type: 'success'
    }
  ).then(async () => {
    try {
      // 前端模拟更新（实际项目中应调用审核 API）
      const idx = packages.value.findIndex((p) => p.id === pkg.id)
      if (idx !== -1) {
        packages.value[idx] = {
          ...packages.value[idx],
          review_status: 'approved',
          reviewed_at: new Date().toISOString(),
          reject_reason: ''
        }
      }
      ElMessage.success('审核已通过，算子包已上架')
    } catch (e) {
      ElMessage.error(e.message)
    }
  }).catch(() => {})
}

function rejectPkg(pkg) {
  rejectingPkg.value = pkg
  rejectReason.value = ''
  showRejectDialog.value = true
}

function confirmReject() {
  if (!rejectReason.value.trim()) {
    ElMessage.warning('请填写驳回理由')
    return
  }
  const pkg = rejectingPkg.value
  if (!pkg) return

  // 前端模拟更新（实际项目中应调用审核 API）
  const idx = packages.value.findIndex((p) => p.id === pkg.id)
  if (idx !== -1) {
    packages.value[idx] = {
      ...packages.value[idx],
      review_status: 'rejected',
      reviewed_at: new Date().toISOString(),
      reject_reason: rejectReason.value.trim()
    }
  }
  ElMessage.success('已驳回，作者将收到通知')
  showRejectDialog.value = false
  rejectingPkg.value = null
  rejectReason.value = ''
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
.card.card-pending {
  border-color: #fde68a;
  background: linear-gradient(180deg, #fffbeb 0%, #fff 30%);
}
.card.card-rejected {
  border-color: #fecaca;
  background: linear-gradient(180deg, #fef2f2 0%, #fff 30%);
}
.card-top { display: flex; justify-content: space-between; align-items: center; }
.card-top-right { display: flex; align-items: center; gap: 6px; }
.ver { font-size: 11px; color: var(--text-3); }
.card-name { font-size: 15px; font-weight: 700; color: var(--text-1); }
.card-summary { font-size: 12px; color: var(--text-3); line-height: 1.6; min-height: 38px; }
.card-tags { display: flex; gap: 5px; flex-wrap: wrap; }
.tag { font-size: 11px; padding: 2px 8px; border-radius: 6px; background: var(--brand-soft); color: var(--brand-dark); }
.card-meta { display: flex; gap: 14px; font-size: 12px; color: var(--text-2); flex-wrap: wrap; }
.card-meta span { display: inline-flex; align-items: center; gap: 4px; }
.card-foot { display: flex; justify-content: space-between; align-items: center; border-top: 1px solid var(--border-light); padding-top: 10px; }
.author { font-size: 12px; color: var(--text-3); }
.hint { font-size: 12px; color: var(--text-3); line-height: 1.5; }
.badge { font-size: 11px; padding: 2px 9px; border-radius: 999px; }
.badge.primary { background: var(--brand); color: #fff; }

/* 主 Tab 样式 */
.main-tabs {
  margin-top: -4px;
}
.main-tabs :deep(.el-tabs__header) {
  margin-bottom: 12px;
}

/* 列表视图名称 */
.list-name {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.list-sub {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 4px;
}

/* 审核统计 */
.review-stats {
  display: flex;
  gap: 32px;
  align-items: center;
}

/* 驳回原因 */
.reject-reason-text {
  font-size: 12px;
  color: #ef4444;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.card-reject-reason {
  font-size: 11px;
  color: #dc2626;
  background: #fef2f2;
  padding: 6px 8px;
  border-radius: 6px;
  display: flex;
  align-items: flex-start;
  gap: 4px;
  line-height: 1.4;
}

/* 搜索历史 */
.hist-list {
  max-height: 300px;
  overflow-y: auto;
}
.hist-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  cursor: pointer;
  border-radius: 4px;
  font-size: 13px;
  color: var(--text-2);
}
.hist-item:hover {
  background: var(--bg-page);
  color: var(--brand);
}
</style>
