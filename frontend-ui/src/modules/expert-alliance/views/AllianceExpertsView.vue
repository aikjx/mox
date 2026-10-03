<template>
  <div class="ax">
    <header class="ax-head">
      <div>
        <h1 class="ax-title">联盟专家广场</h1>
        <p class="ax-sub">专家发现 · 预约 · 即时咨询，数据源 GET /api/experts，契约对齐网关 :3080</p>
      </div>
      <div class="ax-head-actions">
        <el-tag type="info" effect="plain" size="small">本页在线 {{ store.onlineCount }} / {{ store.experts.length }}</el-tag>
        <el-button :icon="Trophy" @click="rankVisible = true">排行榜</el-button>
        <el-button v-role-any="['super_admin', 'tenant_admin']" type="primary" :icon="Plus" @click="openRegister">注册专家</el-button>
        <el-button :icon="Refresh" :loading="store.loading.list" @click="reload">刷新</el-button>
      </div>
    </header>

    <div class="ax-stats">
      <div v-for="s in statsCells" :key="s.label" class="ax-kpi">
        <b>{{ s.value }}</b><span>{{ s.label }}</span>
      </div>
    </div>
    <p class="ax-muted">
      {{ store.loading.stats
        ? '平台统计加载中…'
        : (store.stats ? '上列为 GET /api/experts/stats 的平台级口径，与本页列表分属两套统计。' : `平台统计未加载：${store.error.stats || '后端未返回数据'}`) }}
    </p>

    <el-alert v-if="store.error.list" class="ax-alert" type="error" show-icon :closable="false"
      title="专家列表加载失败" :description="store.error.list" />
    <el-alert v-else-if="store.error.action" class="ax-alert" type="error" show-icon closable
      title="操作未成功" :description="store.error.action" @close="store.error.action = ''" />
    <el-alert v-else-if="store.notice" class="ax-alert" type="success" show-icon closable
      title="操作结果" :description="store.notice" @close="store.notice = ''" />

    <div class="ax-toolbar">
      <el-input
        v-model="keyword"
        class="ax-search"
        placeholder="按名称 / 技能 / 领域搜索（服务端匹配，命中阈值 0.3）"
        clearable
        :prefix-icon="Search"
      />
      <el-select v-model="store.filters.status" class="ax-select" placeholder="在线状态" clearable @change="applyFilter">
        <el-option v-for="s in availabilityOptions" :key="s.value" :label="s.label" :value="s.value" />
      </el-select>
      <el-select v-model="store.filters.expertType" class="ax-select" placeholder="专家类型" clearable @change="applyFilter">
        <el-option v-for="t in typeOptions" :key="t.value" :label="t.label" :value="t.value" />
      </el-select>
      <el-select v-model="store.filters.sort" class="ax-select ax-sort" placeholder="排序" clearable @change="applyFilter">
        <el-option v-for="o in sortOptions" :key="o.value" :label="o.label" :value="o.value" />
      </el-select>
      <el-switch v-model="favoriteOnly" active-text="只看本页收藏" />
      <el-tag v-if="store.filters.domain" class="ax-domain-tag" type="primary" effect="light" size="small" closable
        @close="clearDomainFilter">领域筛选：{{ store.filters.domain }}</el-tag>
      <el-button text :icon="RefreshLeft" @click="resetAll">重置</el-button>
      <span class="ax-count">{{ countText }}</span>
    </div>

    <el-tabs v-model="tab" class="ax-tabs">
      <el-tab-pane name="experts" :label="`专家发现 (${store.total})`">
        <div v-if="store.loading.list" class="ax-grid">
          <el-skeleton v-for="i in 8" :key="i" :rows="4" animated />
        </div>

        <el-empty v-else-if="!shownExperts.length" :image-size="80"
          :description="store.experts.length ? '当前收藏集合里没有专家' : '没有符合条件的专家，试试放宽筛选或换关键词'" />

        <div v-else class="ax-grid">
          <ExpertCard
            v-for="e in shownExperts"
            :key="e.id"
            :expert="e"
            :favorite="store.isFavorite(e.id)"
            @view="openDetail"
            @book="openBooking"
            @consult="openConsult"
            @favorite="store.toggleFavorite"
          />
        </div>

        <div v-if="store.total > store.pageSize" class="ax-pager">
          <el-pagination
            layout="prev, pager, next"
            :current-page="store.page"
            :page-size="store.pageSize"
            :total="store.total"
            background
            @current-change="onPage"
          />
        </div>
      </el-tab-pane>

      <el-tab-pane name="match" :label="`智能匹配 (${store.expertMatches?.total ?? 0})`">
        <div class="ax-toolbar">
          <el-input
            v-model="matchQuery"
            class="ax-search"
            placeholder="用一句话描述要交给专家的任务"
            clearable
            :prefix-icon="Search"
            @keyup.enter="runMatch"
          />
          <el-button type="primary" :loading="store.loading.match" :disabled="matchQuery.trim().length < 2" @click="runMatch">
            开始匹配
          </el-button>
          <el-button text :disabled="!store.expertMatches && !store.error.match" @click="store.clearExpertMatches()">清空</el-button>
        </div>
        <p class="ax-muted">
          上列为 POST {{ ENDPOINTS.expertSearch.path }} 的匹配结果：由联盟匹配器按任务描述打分，
          走的是匹配器自己的专家集，与本页广场列表分属两套口径，id 不保证能在广场中查到。
          {{ store.expertMatches ? `本次描述「${store.expertMatches.query}」` : '' }}
          {{ matchDomains.length ? ` · 限定领域 ${matchDomains.join('、')}` : ' · 未限定领域' }}
        </p>

        <el-alert v-if="store.error.match" class="ax-alert" type="error" show-icon :closable="false"
          title="专家匹配未成功" :description="store.error.match" />

        <template v-else-if="store.expertMatches">
          <el-empty v-if="!store.expertMatches.items.length" :image-size="80" description="匹配器没有给出任何候选专家" />
          <div v-else class="ax-grid">
            <div v-for="m in store.expertMatches.items" :key="m.id" class="ax-match">
              <div class="ax-match-head">
                <b>{{ expertNameOr(m, m.id) }}</b>
                <span class="ax-muted ax-score">匹配分 {{ m.matchScore.toFixed(3) }}</span>
              </div>
              <i class="ax-bar"><i class="ax-bar-fill" :style="{ width: matchBarWidth(m.matchScore) }" /></i>
              <p class="ax-muted">{{ m.description || '（该专家未填写描述）' }}</p>
              <p class="ax-muted">领域：{{ m.domains.join('、') || '—' }} · 状态：{{ availabilityLabel(m.status) }}</p>
              <!-- U2 匹配透明化：后端带逐维演算时展开「为什么匹配」 -->
              <MatchExplainPanel :scores="m.scores" :match-score="m.matchScore" :match-reason="m.matchReason" />
              <p class="ax-muted ax-match-id">id {{ m.id }}</p>
            </div>
          </div>
        </template>

        <el-empty v-else :image-size="80" description="写一句任务描述，让匹配器按领域与能力打分后给出候选专家" />
      </el-tab-pane>

      <el-tab-pane name="bookings" :label="`预约记录 (${store.bookings.length})`">
        <ExpertBookingPanel
          :bookings="store.bookings"
          :counts="store.bookingCounts"
          :loading="store.loading.bookings"
          :error="store.error.bookings"
          :pending-id="cancelingId"
          @cancel="onCancelBooking"
          @room="onOpenRoom"
        />
      </el-tab-pane>

      <el-tab-pane name="capabilities" :label="`能力目录 (${capabilityTotal})`">
        <ExpertCapabilityMatrix
          :data="store.capabilities"
          :loading="store.loading.capabilities"
          :error="store.error.capabilities"
          :active-domain="store.filters.domain"
          @pick="pickDomain"
        />
      </el-tab-pane>
    </el-tabs>

    <!-- 专家详情 -->
    <el-drawer v-model="detailVisible" :title="expertNameOr(detail, '专家详情')" size="480px">
      <div v-if="detail" class="ax-detail">
        <el-descriptions :column="1" border size="small">
          <el-descriptions-item label="ID">{{ detail.id }}</el-descriptions-item>
          <el-descriptions-item label="头衔">{{ detail.title || '—' }}</el-descriptions-item>
          <el-descriptions-item label="所属">{{ detail.organization || '—' }}</el-descriptions-item>
          <el-descriptions-item label="类型">{{ expertTypeLabel(detail.expertType) }}</el-descriptions-item>
          <el-descriptions-item label="认证">{{ verificationLabel(detail.verificationStatus) }}</el-descriptions-item>
          <el-descriptions-item label="在线状态">
            {{ availabilityLabel(detail.availability.status) }} · 负载 {{ detail.availability.currentLoad }}
            <template v-if="detail.availability.maxConcurrent"> / {{ detail.availability.maxConcurrent }}</template>
          </el-descriptions-item>
          <el-descriptions-item label="平均响应">
            {{ detail.availability.avgResponseMinutes ? detail.availability.avgResponseMinutes + ' 分钟' : '—' }}
          </el-descriptions-item>
          <el-descriptions-item label="计费">{{ pricing }}</el-descriptions-item>
          <el-descriptions-item label="时区">{{ detail.timezone || '—' }}</el-descriptions-item>
          <el-descriptions-item label="语言">{{ detail.languages.join('、') || '—' }}</el-descriptions-item>
          <el-descriptions-item label="简介">{{ detail.bio || '—' }}</el-descriptions-item>
        </el-descriptions>

        <h4 class="ax-h4">能力项</h4>
        <el-table v-if="detail.capabilities.length" :data="detail.capabilities" size="small">
          <el-table-column prop="name" label="能力" min-width="120" />
          <el-table-column prop="domain" label="领域" min-width="90" />
          <el-table-column label="熟练度" width="120">
            <template #default="{ row }"><el-progress :percentage="row.proficiency" :stroke-width="6" /></template>
          </el-table-column>
        </el-table>
        <p v-else class="ax-muted">该专家未登记结构化能力项。</p>

        <h4 class="ax-h4">绩效</h4>
        <div class="ax-kpis">
          <div class="ax-kpi"><b>{{ metricsText.rating }}</b><span>评分</span></div>
          <div class="ax-kpi"><b>{{ detail.metrics.totalConsultations }}</b><span>累计咨询</span></div>
          <div class="ax-kpi"><b>{{ detail.metrics.todayConsultations }}</b><span>今日咨询</span></div>
          <div class="ax-kpi"><b>{{ metricsText.resolution }}</b><span>解决率</span></div>
          <div class="ax-kpi"><b>{{ metricsText.accuracy }}</b><span>首答正确率</span></div>
        </div>

        <h4 class="ax-h4">派生指标（后端计算）</h4>
        <p v-if="store.loading.metrics" class="ax-muted">正在取 {{ expertNameOr(detail, detail.id) }} 的派生指标…</p>
        <template v-else-if="detailMetrics">
          <div class="ax-kpis ax-kpis-derived">
            <div v-for="c in derivedCells" :key="c.label" class="ax-kpi" :title="c.note">
              <b>{{ c.value }}</b><span>{{ c.label }}</span>
            </div>
          </div>
          <p class="ax-muted">三值均出自 {{ METRICS_ENDPOINT.path }}，按已启用专家当场算出；专家被停用后该接口返回 404，不是清零。</p>
        </template>
        <p v-else class="ax-muted">派生指标未取到：{{ store.error.metrics || '后端未返回数据' }}</p>

        <div class="ax-detail-actions">
          <el-button :icon="Star" @click="store.toggleFavorite(detail)">
            {{ store.isFavorite(detail.id) ? '取消收藏' : '收藏' }}
          </el-button>
          <el-button :icon="Calendar" @click="openBooking(detail)">预约</el-button>
          <el-button type="primary" :icon="ChatDotRound" :disabled="!detail.online" @click="openConsult(detail)">即时咨询</el-button>
          <el-button v-role-any="['super_admin', 'tenant_admin']" :icon="EditPen" @click="openEdit(detail)">编辑</el-button>
          <el-button v-role-any="['super_admin', 'tenant_admin']" :icon="Delete" @click="openDisable(detail)">停用</el-button>
        </div>
        <p class="ax-muted">
          「停用」是软删：后端把 enabled 置 false 并落盘，全网关没有再启用的端点，PUT 也要求 enabled，
          所以这一步不可逆，且这个 id 之后不能再注册（详见 DELETE /api/experts/:id 契约）。
        </p>
      </div>
    </el-drawer>

    <!-- 预约下单 -->
    <el-dialog v-model="bookingVisible" :title="`预约 ${expertNameOr(bookingExpert, '')}`" width="480px">
      <el-form :model="bookingForm" label-width="88px" label-position="left">
        <el-form-item label="咨询主题" required>
          <el-input v-model="bookingForm.topic" maxlength="60" show-word-limit placeholder="一句话说明要解决的问题" />
        </el-form-item>
        <el-form-item label="计划时间">
          <el-date-picker v-model="bookingForm.scheduledAt" type="datetime" placeholder="留空则默认 24 小时后" value-format="YYYY-MM-DDTHH:mm:ss" />
        </el-form-item>
        <el-form-item label="时长">
          <el-select v-model="bookingForm.durationMinutes">
            <el-option v-for="m in [30, 60, 90, 120]" :key="m" :label="`${m} 分钟`" :value="m" />
          </el-select>
        </el-form-item>
      </el-form>
      <p class="ax-muted">预约提交后状态为「待确认」，由专家侧确认；后端会校验专家是否仍在注册中心。</p>
      <template #footer>
        <el-button @click="bookingVisible = false">取消</el-button>
        <el-button type="primary" :loading="store.loading.action" :disabled="!bookingReady" @click="submitBooking">提交预约</el-button>
      </template>
    </el-dialog>

    <!-- 即时咨询 -->
    <el-dialog v-model="consultVisible" :title="`即时咨询 · ${expertNameOr(consultExpert, '')}`" width="520px">
      <el-form :model="consultForm" label-width="88px" label-position="left">
        <el-form-item label="话题">
          <el-input v-model="consultForm.topic" maxlength="40" />
        </el-form-item>
        <el-form-item label="问题">
          <el-input v-model="consultForm.question" type="textarea" :rows="4" placeholder="想问这位专家什么？" />
        </el-form-item>
      </el-form>
      <el-alert v-if="consultResult" class="ax-alert" :type="consultResult.sessionId ? 'success' : 'warning'" show-icon :closable="false"
        :title="consultResult.sessionId ? '会话已创建' : '未能接入会话'" :description="consultResultText" />
      <template #footer>
        <el-button @click="consultVisible = false">关闭</el-button>
        <el-button type="primary" :loading="store.loading.action" @click="submitConsult">接入会话</el-button>
      </template>
    </el-dialog>

    <!-- 专家排行榜 -->
    <el-dialog v-model="rankVisible" title="专家排行榜" width="520px">
      <ExpertRankBoard :experts="store.experts" />
      <template #footer>
        <el-button @click="rankVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <!-- 咨询室凭据 -->
    <el-dialog v-model="roomVisible" title="咨询室凭据" width="480px">
      <el-descriptions v-if="room" :column="1" border size="small">
        <el-descriptions-item label="房间号">{{ room.roomId }}</el-descriptions-item>
        <el-descriptions-item label="接入令牌">{{ room.roomToken }}</el-descriptions-item>
        <el-descriptions-item label="接入路径">{{ room.joinUrl }}</el-descriptions-item>
        <el-descriptions-item label="房间状态">{{ room.status === ROOM_STATUS.AVAILABLE ? '专家在线，可进入' : '等待专家上线' }}</el-descriptions-item>
        <el-descriptions-item label="有效期">{{ room.expiresIn }} 秒</el-descriptions-item>
        <el-descriptions-item label="ICEServer">{{ room.iceServers.join(' · ') }}</el-descriptions-item>
      </el-descriptions>
      <p class="ax-muted">前端尚未挂载实时房间页，此处只呈现后端签名的接入凭据。</p>
    </el-dialog>

    <!-- 注册 / 编辑专家 -->
    <el-dialog v-model="registryVisible" :title="registryMode === 'edit' ? `编辑 ${expertNameOr(registryExpert, '')}` : '注册专家'" width="760px" top="6vh">
      <ExpertRegistryForm ref="registryForm" :key="registryKey" :expert="registryExpert" />
      <el-alert v-if="store.error.action" class="ax-modal-alert" type="error" show-icon :closable="false"
        :title="registryMode === 'edit' ? '改动未保存' : '专家未注册'" :description="store.error.action" />
      <p class="ax-muted">{{ EXPERT_WRITE_IDENTITY.statement }}</p>
      <template #footer>
        <el-button @click="registryVisible = false">取消</el-button>
        <el-button type="primary" :loading="store.loading.action" :disabled="!registryReady" @click="submitRegistry">
          {{ registryMode === 'edit' ? '保存改动' : '注册' }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 停用确认：把后果逐条摆出来，而不是只问一句"确定吗" -->
    <el-dialog v-model="disableVisible" :title="`停用 ${expertNameOr(disableTarget, '')}`" width="560px">
      <ol class="ax-consequences">
        <li v-for="line in disableLines" :key="line">{{ line }}</li>
      </ol>
      <el-alert v-if="store.error.action" class="ax-modal-alert" type="error" show-icon :closable="false"
        title="停用未生效" :description="store.error.action" />
      <template #footer>
        <el-button @click="disableVisible = false">返回</el-button>
        <el-button type="danger" :loading="store.loading.action" @click="submitDisable">确认停用</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import {
  Calendar, ChatDotRound, Delete, EditPen, Plus, Refresh, RefreshLeft, Search, Star, Trophy
} from '@element-plus/icons-vue'
import { ExpertCard } from '@/modules/expert-alliance/components'
import { ExpertBookingPanel } from '@/modules/expert-alliance/components'
import { ExpertCapabilityMatrix } from '@/modules/expert-alliance/components'
import { ExpertRankBoard } from '@/modules/expert-alliance/components'
import { MatchExplainPanel } from '@/modules/expert-alliance/components'
import { ExpertRegistryForm } from '@/modules/expert-alliance/components'
import { useAllianceExpertsStore } from '@/modules/expert-alliance/store'
import { useAllianceEventStream } from '@/modules/expert-alliance/composables/useAllianceEventStream'
import {
  EXPERT_AVAILABILITY, EXPERT_TYPE,
  availabilityLabel, expertDerivedCells, expertStatsCells, expertTypeLabel, pricingText, sortLabel, verificationLabel
} from '@/modules/expert-alliance/contract'
import { ROOM_STATUS } from '@/modules/expert-alliance/contract'
import { ENDPOINTS, EXPERT_SORT } from '@/modules/expert-alliance/contract'
import { deleteConsequences, EXPERT_WRITE_IDENTITY } from '@/modules/expert-alliance/contract'
import { expertNameOr } from '@/modules/expert-alliance/contract'

const METRICS_ENDPOINT = ENDPOINTS.expertMetrics

const store = useAllianceExpertsStore()

const tab = ref('experts')
const keyword = ref('')
const favoriteOnly = ref(false)
const cancelingId = ref('')

const availabilityOptions = Object.values(EXPERT_AVAILABILITY).map((v) => ({ value: v, label: availabilityLabel(v) }))
const typeOptions = Object.values(EXPERT_TYPE).map((v) => ({ value: v, label: expertTypeLabel(v) }))
const sortOptions = EXPERT_SORT.map((v) => ({ value: v, label: sortLabel(v) }))

const detailVisible = ref(false)
const detail = ref(null)
const bookingVisible = ref(false)
const bookingExpert = ref(null)
const bookingForm = reactive({ topic: '', scheduledAt: '', durationMinutes: 60 })
const consultVisible = ref(false)
const consultExpert = ref(null)
const consultForm = reactive({ topic: '即时咨询', question: '' })
const consultResult = ref(null)
const roomVisible = ref(false)
const room = ref(null)
const rankVisible = ref(false)

// 智能匹配：描述与广场关键词是两处输入，把 keyword 作为起念递过来一次即可，
// 之后互不牵连，免得改动匹配描述顺手把列表筛选也改了。
const matchQuery = ref('')
const matchDomains = computed(() => (store.filters.domain ? [store.filters.domain] : []))

// 注册 / 编辑：草稿在 ExpertRegistryForm 内持有，registryKey 换人即重挂载，
// 免得上一位专家的未提交改动跟着跳到下一位的表单里。
const registryVisible = ref(false)
const registryMode = ref('register')
const registryExpert = ref(null)
const registryKey = ref(0)
const registryForm = ref(null)
const disableVisible = ref(false)
const disableTarget = ref(null)

const shownExperts = computed(() => (
  favoriteOnly.value ? store.experts.filter((e) => store.isFavorite(e.id)) : store.experts
))
// 服务端 total 与本页可见数在有本地过滤时不一致，必须说明是哪一层在生效
const countText = computed(() => (
  shownExperts.value.length === store.experts.length
    ? `共 ${store.total} 位专家`
    : `本页显示 ${shownExperts.value.length} / ${store.experts.length} 位（服务端共 ${store.total} 位）`
))
const bookingReady = computed(() => bookingForm.topic.trim().length > 1)
const pricing = computed(() => (
  detail.value ? pricingText(detail.value.pricingModel, detail.value.hourlyRateCents) : '—'
))
const metricsText = computed(() => {
  const m = detail.value?.metrics
  if (!m) return { rating: '—', resolution: '—', accuracy: '—' }
  return {
    rating: m.ratingCount > 0 ? `${m.avgRating.toFixed(1)} (${m.ratingCount})` : '暂无',
    resolution: m.resolutionRate > 0 ? `${Math.round(m.resolutionRate * 100)}%` : '暂无',
    accuracy: m.firstResponseAccuracy > 0 ? `${Math.round(m.firstResponseAccuracy * 100)}%` : '暂无'
  }
})
// KPI 文案在契约层裁决（expertStatsCells），视图只负责摆放
const statsCells = computed(() => expertStatsCells(store.stats))

const capabilityTotal = computed(() => store.capabilities?.total ?? 0)
// store 只留一份派生指标，必须核对它属于哪位专家，否则上一位的数会跟到下一位名下
const detailMetrics = computed(() => (
  store.expertMetrics && store.expertMetrics.expertId === detail.value?.id ? store.expertMetrics : null
))
const derivedCells = computed(() => expertDerivedCells(detailMetrics.value?.derived))

// 表单里的校验原因由契约层给出，视图只显示第一条并把提交键禁用
const registryReady = computed(() => !registryForm.value?.problem)
const disableLines = computed(() => deleteConsequences(disableTarget.value || {}))
// 弹窗里的错误条只该反映这一次写的结果：开面即清掉别处留下的旧错，
// 否则一次收藏失败会在下一次打开注册弹窗时被读成"专家未注册"。
watch([registryVisible, disableVisible], ([reg, dis]) => {
  if (reg || dis) store.error.action = ''
})

const consultResultText = computed(() => {
  const r = consultResult.value
  if (!r) return ''
  if (!r.sessionId) return r.message || '专家当前不在线'
  // 后端返回 /chat/{id}，本 SPA 尚无该路由，只呈现凭据不做假跳转
  return `会话 ${r.sessionId} 已创建（渠道 ${r.channel}），会话页路径 ${r.chatUrl} 尚未在前端挂载。`
})

let searchTimer = null
watch(keyword, (v) => {
  clearTimeout(searchTimer)
  searchTimer = setTimeout(() => {
    store.filters.search = v.trim()
    store.page = 1
    store.loadExperts()
  }, 350)
})

async function applyFilter() {
  store.page = 1
  await store.loadExperts()
}

function runMatch() {
  return store.searchExpertMatches({ query: matchQuery.value, domains: matchDomains.value })
}

/** 匹配分的量纲后端没有承诺，只按本次候选集里的最大值相对画条，不假装它是百分比 */
function matchBarWidth(score) {
  const items = store.expertMatches?.items ?? []
  const max = items.reduce((acc, m) => Math.max(acc, m.matchScore), 0)
  if (!(max > 0)) return '0%'
  return `${Math.max(2, Math.round((score / max) * 100))}%`
}

async function resetAll() {
  clearTimeout(searchTimer)
  keyword.value = ''
  favoriteOnly.value = false
  store.resetFilters()
  await store.loadExperts()
}

async function reload() {
  await Promise.all([store.loadExperts(), store.loadBookings(), store.loadStats()])
  if (store.capabilities) store.loadCapabilities()
}

function onPage(next) {
  store.goPage(next)
  store.loadExperts()
}

function openDetail(expert) {
  detail.value = expert
  detailVisible.value = true
  store.loadExpertMetrics(expert.id)
}

/** 能力目录点领域名：落到后端唯一认得的这一层过滤，再回列表页 */
async function pickDomain(domain) {
  store.filters.domain = store.filters.domain === domain ? '' : domain
  store.page = 1
  tab.value = 'experts'
  await store.loadExperts()
}

async function clearDomainFilter() {
  store.filters.domain = ''
  store.page = 1
  await store.loadExperts()
}

function openBooking(expert) {
  bookingExpert.value = expert
  bookingForm.topic = ''
  bookingForm.scheduledAt = ''
  bookingForm.durationMinutes = 60
  bookingVisible.value = true
}

async function submitBooking() {
  if (!bookingReady.value) return
  const ok = await store.createBooking({
    expertId: bookingExpert.value.id,
    topic: bookingForm.topic.trim(),
    // el-date-picker 给的是本地墙钟串，后端要 RFC3339，统一在提交处转一次
    scheduledAt: bookingForm.scheduledAt ? new Date(bookingForm.scheduledAt).toISOString() : '',
    durationMinutes: bookingForm.durationMinutes
  })
  if (ok) bookingVisible.value = false
}

function openConsult(expert) {
  consultExpert.value = expert
  consultForm.topic = '即时咨询'
  consultForm.question = ''
  consultResult.value = null
  consultVisible.value = true
}

async function submitConsult() {
  const res = await store.consultNow(consultExpert.value, {
    topic: consultForm.topic.trim() || '即时咨询',
    question: consultForm.question.trim()
  })
  if (res) consultResult.value = res
}

function openRegister() {
  registryMode.value = 'register'
  registryExpert.value = null
  registryKey.value += 1
  registryVisible.value = true
}

function openEdit(expert) {
  registryMode.value = 'edit'
  registryExpert.value = expert
  registryKey.value += 1
  registryVisible.value = true
}

async function submitRegistry() {
  const draft = registryForm.value?.draft
  if (!draft) return
  const saved = registryMode.value === 'edit'
    ? await store.saveExpert(registryExpert.value, draft)
    : await store.registerExpert(draft)
  // null 有两种：校验没过 / 后端报错，两者都留在弹窗里让用户改；
  // 「没有改动」那条 store 只给 notice 不给 error，此时直接关窗
  if (!saved && store.error.action) return
  if (saved && registryMode.value === 'edit') {
    if (detail.value?.id === saved.id) {
      detail.value = saved
      // 并发上限与在线状态都会改变派生指标，抽屉开着就得跟着重取
      store.loadExpertMetrics(saved.id)
    }
  }
  registryVisible.value = false
}

function openDisable(expert) {
  disableTarget.value = expert
  disableVisible.value = true
}

async function submitDisable() {
  const res = await store.removeExpert(disableTarget.value)
  if (!res) return
  disableVisible.value = false
  if (detail.value?.id === res.id) {
    detailVisible.value = false
    detail.value = null
  }
}

async function onCancelBooking(booking) {
  cancelingId.value = booking.id
  await store.cancelBooking(booking.id)
  cancelingId.value = ''
}

async function onOpenRoom(booking) {
  const res = await store.openRoom(booking.id)
  if (res) {
    room.value = res
    roomVisible.value = true
  }
}

// T4 SSE 真实挂载：本路由 /alliance/experts 已 requiresRole 限管理员（index.js:152），
// 故开流只对管理员；别的管理员注册/停用/改档专家时，本页收到带 expert_id 的帧 →
// store.applyRegistryEvent 防抖真拉列表，免手动刷新。连接失败静默（读数仍可手动「刷新」）。
const eventStream = useAllianceEventStream({
  onEvent: (kind, envelope) => store.applyRegistryEvent(kind, envelope),
  onError: () => {}
})

onMounted(async () => {
  await store.loadExperts()
  // 预约与平台统计都是次要面板，首屏列表之后再取，避免与列表争抢同一渲染帧
  store.loadBookings()
  store.loadStats()
  eventStream.start()
})

onUnmounted(() => eventStream.stop())

watch(tab, (v) => {
  if (v === 'bookings' && !store.bookings.length && !store.loading.bookings) store.loadBookings()
  if (v === 'capabilities' && !store.capabilities && !store.loading.capabilities) store.loadCapabilities()
})
</script>

<style scoped>
.ax {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
}
.ax-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}
.ax-title {
  margin: 0;
  font-size: 19px;
  font-weight: 600;
  color: var(--text-primary);
}
.ax-sub {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-muted);
}
.ax-head-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ax-alert { margin: 0; }
.ax-modal-alert { margin: 0 0 10px; }
.ax-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  padding: 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.ax-search { width: 320px; }
.ax-select { width: 132px; }
.ax-sort { width: 120px; }
.ax-count {
  margin-left: auto;
  font-size: 12px;
  color: var(--text-muted);
}
.ax-tabs { flex: 1; min-height: 0; }
.ax-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 12px;
  padding: 12px 0;
}
.ax-pager {
  display: flex;
  justify-content: center;
  padding: 10px 0 4px;
}
.ax-detail {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.ax-h4 {
  margin: 6px 0 0;
  font-size: 13px;
  color: var(--text-primary);
}
.ax-stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(112px, 1fr));
  gap: 8px;
  margin-top: 12px;
}
.ax-kpis {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 8px;
}
.ax-kpis-derived { grid-template-columns: repeat(3, minmax(0, 1fr)); }
.ax-kpi {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
  text-align: center;
}
.ax-kpi b { font-size: 14px; color: var(--text-primary); }
.ax-kpi span { font-size: 11px; color: var(--text-muted); }
.ax-detail-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 8px;
}
.ax-consequences {
  margin: 0;
  padding-left: 20px;
  font-size: 12px;
  line-height: 1.9;
  color: var(--text-secondary);
}
.ax-muted {
  margin: 8px 0 0;
  font-size: 11px;
  color: var(--text-muted);
}
.ax-match {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
}
.ax-match-head {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 8px;
  font-size: 13px;
  color: var(--text-primary);
}
.ax-score { margin: 0; font-size: 11px; white-space: nowrap; }
.ax-match-id { word-break: break-all; }
.ax-bar {
  display: block;
  height: 4px;
  border-radius: var(--radius-sm);
  background: var(--bg-hover);
  overflow: hidden;
}
.ax-bar-fill {
  display: block;
  height: 100%;
  background: var(--brand-fill);
}
</style>
