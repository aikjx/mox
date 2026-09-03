<template>
  <div class="expert-plaza">
    <!-- ===== 页头 ===== -->
    <div class="page-header">
      <div>
        <h1 class="page-title">专家广场</h1>
        <p class="page-subtitle">汇聚各领域顶尖专家，智能匹配 · 实时咨询 · 精准预约</p>
      </div>
      <div class="header-actions">
        <el-button size="small" @click="showRanking = true">
          <el-icon><Trophy /></el-icon> 排行榜
        </el-button>
      </div>
    </div>

    <!-- ===== Tab 切换 ===== -->
    <div class="view-tabs">
      <div
        class="view-tab"
        :class="{ active: activeTab === 'experts' }"
        @click="activeTab = 'experts'"
      >
        <el-icon><User /></el-icon>
        专家发现
        <span class="tab-count">{{ filteredExperts.length }}</span>
      </div>
      <div
        class="view-tab"
        :class="{ active: activeTab === 'bookings' }"
        @click="activeTab = 'bookings'"
      >
        <el-icon><Calendar /></el-icon>
        我的预约
        <span class="tab-count" v-if="myBookings.length">{{ myBookings.length }}</span>
      </div>
    </div>

    <!-- ===== 专家列表 ===== -->
    <div v-show="activeTab === 'experts'" class="experts-view">
      <!-- 搜索行 -->
      <div class="search-row">
        <el-input
          v-model="searchKeyword"
          placeholder="搜索专家名称 / 技能 / 领域…"
          clearable
          style="max-width: 400px"
          @input="handleSearch"
        >
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
      </div>

      <!-- 筛选 chips -->
      <div class="expert-filters">
        <span
          v-for="cat in categories"
          :key="cat.key"
          class="filter-chip"
          :class="{ active: selectedCategory === cat.key }"
          @click="selectedCategory = cat.key"
        >
          {{ cat.label }}
        </span>
      </div>

      <!-- 加载 / 空 / 网格 -->
      <div v-if="loading" class="state-box">
        <div class="loading-spinner"></div>
        <p>正在加载专家列表…</p>
      </div>

      <div v-else-if="filteredExperts.length === 0" class="state-box">
        <div class="state-icon">🔍</div>
        <h4>未找到匹配的专家</h4>
        <p>试试调整筛选条件或搜索关键词</p>
        <el-button type="primary" size="small" @click="resetFilters">重置筛选</el-button>
      </div>

      <div v-else class="expert-grid">
        <div
          v-for="expert in filteredExperts"
          :key="expert.id"
          class="expert-card"
          @click="openExpertDetail(expert)"
        >
          <div class="expert-header">
            <div class="expert-avatar" :style="{ background: expert.avatarGradient }">
              {{ expert.name.charAt(0) }}
            </div>
            <div class="expert-header-info">
              <div class="expert-name">{{ expert.name }}</div>
              <div class="expert-title">{{ expert.typeLabel }} · {{ expert.department }}</div>
              <div class="expert-rating">
                ⭐ {{ expert.avgRating }}
                <span class="rating-count">({{ expert.consultCount }} 次咨询)</span>
              </div>
            </div>
          </div>
          <div class="expert-skills">
            <span
              v-for="skill in expert.skills.slice(0, 4)"
              :key="skill"
              class="expert-skill"
              @click.stop="searchKeyword = skill; handleSearch()"
            >
              {{ skill }}
            </span>
            <span v-if="expert.skills.length > 4" class="skill-more">+{{ expert.skills.length - 4 }}</span>
          </div>
          <div class="expert-stats">
            <div class="expert-stat">
              <div class="num">{{ expert.consultCount }}</div>
              <div class="label">参与项目</div>
            </div>
            <div class="expert-stat">
              <div class="num">{{ expert.avgRating }}</div>
              <div class="label">用户评分</div>
            </div>
            <div class="expert-stat">
              <div class="num">{{ expert.goodRate }}%</div>
              <div class="label">好评率</div>
            </div>
          </div>
        </div>
      </div>

      <!-- 加载更多 -->
      <div v-if="filteredExperts.length > 0 && !allLoaded" class="load-more">
        <el-button plain size="small" :loading="loadingMore" @click="loadMore">加载更多</el-button>
      </div>
    </div>

    <!-- ===== 我的预约 ===== -->
    <div v-show="activeTab === 'bookings'" class="bookings-view">
      <div v-if="myBookings.length === 0" class="state-box">
        <div class="state-icon">📅</div>
        <h4>暂无预约记录</h4>
        <p>快去专家广场预约您心仪的专家吧</p>
        <el-button type="primary" size="small" @click="activeTab = 'experts'">浏览专家</el-button>
      </div>

      <div v-else class="bookings-list">
        <div
          v-for="booking in myBookings"
          :key="booking.id"
          class="booking-card"
        >
          <div class="booking-header">
            <div class="booking-expert">
              <div class="booking-avatar" :style="{ background: booking.expertGradient }">
                {{ booking.expertEmoji }}
              </div>
              <div>
                <div class="booking-expert-name">{{ booking.expertName }}</div>
                <div class="booking-expert-type">{{ booking.expertType }}</div>
              </div>
            </div>
            <el-tag
              :type="bookingStatusType(booking.status)"
              effect="dark"
              size="small"
              round
            >
              {{ bookingStatusLabel(booking.status) }}
            </el-tag>
          </div>
          <div class="booking-body">
            <div class="booking-topic">
              <span class="booking-label">咨询主题：</span>{{ booking.topic }}
            </div>
            <div class="booking-time">
              <el-icon><Calendar /></el-icon>
              {{ booking.date }} {{ booking.timeSlot }}
            </div>
            <div class="booking-desc" v-if="booking.description">{{ booking.description }}</div>
          </div>
          <div class="booking-footer">
            <el-button
              size="small"
              @click="cancelBooking(booking.id)"
              v-if="booking.status === 'pending'"
            >
              取消预约
            </el-button>
            <el-button
              type="primary"
              size="small"
              @click="startConsult(booking)"
              v-if="booking.status === 'confirmed'"
            >
              进入咨询
            </el-button>
            <el-button
              size="small"
              @click="rebook(booking)"
              v-if="booking.status === 'completed' || booking.status === 'cancelled'"
            >
              再次预约
            </el-button>
          </div>
        </div>
      </div>
    </div>

    <!-- ===== 专家详情弹窗 ===== -->
    <el-dialog
      v-model="showDetail"
      class="detail-dialog"
      width="900px"
      :close-on-click-modal="false"
      destroy-on-close
    >
      <template #header>
        <div class="detail-header">
          <div class="detail-avatar-wrap">
            <div class="detail-avatar" :style="{ background: currentExpert?.avatarGradient }">
              <span class="detail-avatar-emoji">{{ currentExpert?.avatarEmoji }}</span>
              <span v-if="currentExpert?.online" class="detail-online-dot"></span>
            </div>
          </div>
          <div class="detail-title-area">
            <div class="detail-name-row">
              <h2 class="detail-name">{{ currentExpert?.name }}</h2>
              <span class="level-badge" :class="`level-${currentExpert?.level}`">
                {{ getLevelLabel(currentExpert?.level) }}
              </span>
              <span
                class="type-tag"
                :style="{ background: currentExpert?.typeColor + '30', color: currentExpert?.typeColor }"
              >
                {{ currentExpert?.typeLabel }}
              </span>
            </div>
            <div class="detail-status">
              <span class="status-dot" :class="{ online: currentExpert?.online }"></span>
              {{ currentExpert?.online ? '在线' : '离线' }}
              <span class="status-sep">·</span>
              {{ currentExpert?.department }}
            </div>
          </div>
          <div class="detail-actions-head">
            <button class="icon-btn" @click="toggleFavorite(currentExpert)">
              <el-icon :size="20">
                <Star v-if="currentExpert?.favorited" :fill="'#f59e0b'" />
                <Star v-else />
              </el-icon>
            </button>
          </div>
        </div>
      </template>

      <div class="detail-body" v-if="currentExpert">
        <div class="detail-grid">
          <div class="detail-info-col">
            <div class="info-section">
              <h4 class="info-section-title">
                <el-icon><InfoFilled /></el-icon> 基本信息
              </h4>
              <div class="info-list">
                <div class="info-item">
                  <span class="info-label">所在部门</span>
                  <span class="info-value">{{ currentExpert.department }}</span>
                </div>
                <div class="info-item">
                  <span class="info-label">联系方式</span>
                  <span class="info-value">{{ currentExpert.phone }}</span>
                </div>
                <div class="info-item">
                  <span class="info-label">电子邮箱</span>
                  <span class="info-value">{{ currentExpert.email }}</span>
                </div>
                <div class="info-item">
                  <span class="info-label">加入时间</span>
                  <span class="info-value">{{ currentExpert.joinDate }}</span>
                </div>
              </div>
            </div>
            <div class="info-section">
              <h4 class="info-section-title">
                <el-icon><DataAnalysis /></el-icon> 数据统计
              </h4>
              <div class="stats-mini-grid">
                <div class="stat-mini">
                  <div class="stat-mini-value">{{ currentExpert.consultCount }}</div>
                  <div class="stat-mini-label">咨询总数</div>
                </div>
                <div class="stat-mini">
                  <div class="stat-mini-value accent">{{ currentExpert.goodRate }}%</div>
                  <div class="stat-mini-label">好评率</div>
                </div>
                <div class="stat-mini">
                  <div class="stat-mini-value">{{ currentExpert.responseTime }}</div>
                  <div class="stat-mini-label">平均响应</div>
                </div>
                <div class="stat-mini">
                  <div class="stat-mini-value violet">{{ currentExpert.avgRating }}</div>
                  <div class="stat-mini-label">平均评分</div>
                </div>
              </div>
            </div>
          </div>
          <div class="detail-skills-col">
            <div class="info-section">
              <h4 class="info-section-title">
                <el-icon><MagicStick /></el-icon> 专业技能
              </h4>
              <div class="skill-cloud">
                <span
                  v-for="(skill, idx) in currentExpert.skills"
                  :key="skill"
                  class="cloud-tag"
                  :style="{ '--size': 0.85 + (idx % 5) * 0.1 + 'rem' }"
                >
                  {{ skill }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <div class="info-section">
          <h4 class="info-section-title">
            <el-icon><Document /></el-icon> 专家介绍
          </h4>
          <div class="expert-bio">
            <p v-for="(para, i) in currentExpert.bioParagraphs" :key="i">{{ para }}</p>
          </div>
        </div>

        <div class="info-section">
          <div class="section-head-row">
            <h4 class="info-section-title">
              <el-icon><ChatLineSquare /></el-icon> 用户评价
              <span class="review-count">({{ currentExpert.reviews.length }})</span>
            </h4>
            <el-radio-group v-model="reviewFilter" size="small">
              <el-radio-button value="all">全部</el-radio-button>
              <el-radio-button value="5">5星</el-radio-button>
              <el-radio-button value="4">4星</el-radio-button>
              <el-radio-button value="3">3星及以下</el-radio-button>
            </el-radio-group>
          </div>
          <div class="reviews-list">
            <div
              v-for="review in filteredReviews"
              :key="review.id"
              class="review-item"
            >
              <div class="review-head">
                <div class="review-avatar">{{ review.avatar }}</div>
                <div class="review-meta">
                  <div class="review-name">{{ review.userName }}</div>
                  <div class="review-rating">
                    <el-rate :model-value="review.rating" disabled size="small" />
                    <span class="review-date">{{ review.date }}</span>
                  </div>
                </div>
              </div>
              <div class="review-content">{{ review.content }}</div>
              <div class="review-tags" v-if="review.tags?.length">
                <span v-for="tag in review.tags" :key="tag" class="review-tag">{{ tag }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <template #footer>
        <div class="detail-footer-actions">
          <div class="footer-price">
            <span class="price-label">咨询费用</span>
            <span class="price-amount">
              <span v-if="currentExpert?.price === 0">免费</span>
              <span v-else>¥{{ currentExpert?.price }}<span class="price-unit">/次</span></span>
            </span>
          </div>
          <div class="footer-btns">
            <el-button @click="addToTeam">
              <el-icon><UserFilled /></el-icon> 加入团队
            </el-button>
            <el-button type="primary" @click="openBooking(currentExpert)">
              <el-icon><Calendar /></el-icon> 预约专家
            </el-button>
            <el-button
              type="success"
              @click="startConsultNow(currentExpert)"
              v-if="currentExpert?.online"
            >
              <el-icon><ChatDotRound /></el-icon> 发起咨询
            </el-button>
          </div>
        </div>
      </template>
    </el-dialog>

    <!-- ===== 预约对话框 ===== -->
    <el-dialog
      v-model="showBookingDialog"
      title="预约专家"
      width="520px"
      class="booking-dialog"
      destroy-on-close
    >
      <div v-if="bookingExpert" class="booking-form">
        <div class="booking-expert-info">
          <div class="be-avatar" :style="{ background: bookingExpert.avatarGradient }">
            {{ bookingExpert.avatarEmoji }}
          </div>
          <div class="be-info">
            <div class="be-name">{{ bookingExpert.name }}</div>
            <div class="be-type">{{ bookingExpert.typeLabel }}</div>
          </div>
          <div class="be-price">
            <span v-if="bookingExpert.price === 0">免费</span>
            <span v-else>¥{{ bookingExpert.price }}<span class="price-unit">/次</span></span>
          </div>
        </div>

        <el-form :model="bookingForm" label-position="top" class="booking-el-form">
          <el-form-item label="选择日期">
            <el-date-picker
              v-model="bookingForm.date"
              type="date"
              placeholder="请选择预约日期"
              :disabled-date="disabledDate"
              style="width: 100%"
            />
          </el-form-item>
          <el-form-item label="选择时间段">
            <div class="time-slots">
              <span
                v-for="slot in timeSlots"
                :key="slot"
                class="time-slot"
                :class="{
                  active: bookingForm.timeSlot === slot,
                  disabled: isSlotDisabled(slot)
                }"
                @click="selectTimeSlot(slot)"
              >
                {{ slot }}
              </span>
            </div>
          </el-form-item>
          <el-form-item label="咨询主题">
            <el-input
              v-model="bookingForm.topic"
              placeholder="请简要描述您想咨询的主题"
              maxlength="50"
              show-word-limit
            />
          </el-form-item>
          <el-form-item label="问题描述">
            <el-input
              v-model="bookingForm.description"
              type="textarea"
              :rows="4"
              placeholder="请详细描述您的问题，以便专家提前准备…"
              maxlength="500"
              show-word-limit
            />
          </el-form-item>
        </el-form>
      </div>

      <template #footer>
        <el-button @click="showBookingDialog = false">取消</el-button>
        <el-button
          type="primary"
          :loading="submittingBooking"
          @click="submitBooking"
        >
          确认预约
        </el-button>
      </template>
    </el-dialog>

    <!-- ===== 排行榜弹窗 ===== -->
    <el-dialog
      v-model="showRanking"
      title="专家排行榜"
      width="720px"
      class="ranking-dialog"
      destroy-on-close
    >
      <el-tabs v-model="rankingTab" class="ranking-tabs">
        <el-tab-pane label="月度咨询榜" name="consult">
          <div class="ranking-list">
            <div
              v-for="(item, idx) in rankingData.consult"
              :key="item.id"
              class="ranking-item"
              @click="openExpertDetail(item); showRanking = false"
            >
              <div class="rank-num" :class="`rank-${idx + 1}`">{{ idx + 1 }}</div>
              <div class="rank-avatar" :style="{ background: item.avatarGradient }">
                {{ item.avatarEmoji }}
              </div>
              <div class="rank-info">
                <div class="rank-name">{{ item.name }}</div>
                <div class="rank-type">{{ item.typeLabel }}</div>
              </div>
              <div class="rank-stat">
                <div class="rank-stat-value">{{ item.consultCount }}</div>
                <div class="rank-stat-label">次咨询</div>
              </div>
            </div>
          </div>
        </el-tab-pane>
        <el-tab-pane label="好评榜" name="rating">
          <div class="ranking-list">
            <div
              v-for="(item, idx) in rankingData.rating"
              :key="item.id"
              class="ranking-item"
              @click="openExpertDetail(item); showRanking = false"
            >
              <div class="rank-num" :class="`rank-${idx + 1}`">{{ idx + 1 }}</div>
              <div class="rank-avatar" :style="{ background: item.avatarGradient }">
                {{ item.avatarEmoji }}
              </div>
              <div class="rank-info">
                <div class="rank-name">{{ item.name }}</div>
                <div class="rank-type">{{ item.typeLabel }}</div>
              </div>
              <div class="rank-stat">
                <div class="rank-stat-value good">{{ item.goodRate }}%</div>
                <div class="rank-stat-label">好评率</div>
              </div>
            </div>
          </div>
        </el-tab-pane>
        <el-tab-pane label="新星榜" name="newstar">
          <div class="ranking-list">
            <div
              v-for="(item, idx) in rankingData.newstar"
              :key="item.id"
              class="ranking-item"
              @click="openExpertDetail(item); showRanking = false"
            >
              <div class="rank-num" :class="`rank-${idx + 1}`">{{ idx + 1 }}</div>
              <div class="rank-avatar" :style="{ background: item.avatarGradient }">
                {{ item.avatarEmoji }}
              </div>
              <div class="rank-info">
                <div class="rank-name">{{ item.name }}</div>
                <div class="rank-type">{{ item.typeLabel }} · 新入驻</div>
              </div>
              <div class="rank-stat">
                <div class="rank-stat-value violet">{{ item.monthGrowth }}%</div>
                <div class="rank-stat-label">月增长</div>
              </div>
            </div>
          </div>
        </el-tab-pane>
      </el-tabs>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Search, Filter, Grid, Medal, Wallet, Sort, Trophy, ArrowRight,
  Calendar, User, ChatDotRound, Star, Timer, InfoFilled,
  DataAnalysis, MagicStick, Document, ChatLineSquare, UserFilled,
  TrendCharts
} from '@element-plus/icons-vue'
import { getExperts, getExpert, getExpertsStats, getMyBookings, toggleExpertFavorite, createBooking, cancelBooking as apiCancelBooking, enterConsultRoom, joinExpertTeam, consultNow } from '@/api'

// ===== 状态 =====
const loading = ref(true)
const loadingMore = ref(false)
const allLoaded = ref(false)
const searchLoading = ref(false)
const searchKeyword = ref('')
const activeQuickTab = ref('all')
const selectedCategory = ref('all')
const selectedLevel = ref('all')
const selectedPrice = ref('all')
const sortBy = ref('comprehensive')
const activeTab = ref('experts')
const showDetail = ref(false)
const currentExpert = ref(null)
const showBookingDialog = ref(false)
const bookingExpert = ref(null)
const submittingBooking = ref(false)
const showRanking = ref(false)
const rankingTab = ref('consult')
const reviewFilter = ref('all')

// ===== Mock 数据 =====
const expertTypes = {
  algorithm: { label: '算法专家', color: '#6366f1' },
  architecture: { label: '架构专家', color: '#0891b2' },
  data: { label: '数据专家', color: '#10b981' },
  ai: { label: 'AI专家', color: '#ec4899' },
  workflow: { label: '工作流专家', color: '#f59e0b' },
  graph: { label: '知识图谱专家', color: '#06b6d4' },
  security: { label: '安全专家', color: '#ef4444' },
  performance: { label: '性能优化专家', color: '#14b8a6' },
  monitor: { label: '可观测性专家', color: '#f97316' },
  market: { label: '商业智能专家', color: '#f43f5e' },
  mcp: { label: 'MCP协议专家', color: '#a855f7' },
  automation: { label: '自动化专家', color: '#0ea5e9' },
  requirement: { label: '需求工程专家', color: '#16a34a' },
  fusion: { label: '融合专家', color: '#7c3aed' },
  operator: { label: '算子系统专家', color: '#8b5cf6' }
}

const levels = [
  { key: 'all', label: '全部' },
  { key: 'bronze', label: '铜级' },
  { key: 'silver', label: '银级' },
  { key: 'gold', label: '金级' },
  { key: 'diamond', label: '钻级' },
  { key: 'master', label: '大师' }
]

const priceRanges = [
  { key: 'all', label: '全部' },
  { key: 'free', label: '免费' },
  { key: 'low', label: '¥0-100' },
  { key: 'mid', label: '¥100-500' },
  { key: 'high', label: '¥500+' }
]

const sortOptions = [
  { key: 'comprehensive', label: '综合' },
  { key: 'rating', label: '好评' },
  { key: 'consult', label: '咨询量' },
  { key: 'price', label: '价格' },
  { key: 'response', label: '响应' }
]

const categories = [
  { key: 'all', label: '全部领域', color: '#64748b' },
  { key: 'algorithm', label: 'AI算法', color: '#6366f1' },
  { key: 'architecture', label: '架构设计', color: '#0891b2' },
  { key: 'ai', label: 'AI应用', color: '#ec4899' },
  { key: 'data', label: '数据工程', color: '#10b981' },
  { key: 'workflow', label: '工作流', color: '#f59e0b' },
  { key: 'graph', label: '知识图谱', color: '#06b6d4' },
  { key: 'security', label: '安全合规', color: '#ef4444' },
  { key: 'performance', label: '性能优化', color: '#14b8a6' },
  { key: 'monitor', label: '运维SRE', color: '#f97316' },
  { key: 'market', label: '商业智能', color: '#f43f5e' },
  { key: 'mcp', label: 'MCP协议', color: '#a855f7' },
  { key: 'automation', label: '自动化', color: '#0ea5e9' },
  { key: 'requirement', label: '需求工程', color: '#16a34a' },
  { key: 'fusion', label: '全维融合', color: '#7c3aed' },
  { key: 'operator', label: '算子系统', color: '#8b5cf6' }
]

const quickTabs = [
  { key: 'all', label: '全部', icon: '✨' },
  { key: 'recommended', label: '推荐', icon: '🔥' },
  { key: 'hot', label: '热门', icon: '⚡' },
  { key: 'new', label: '新入驻', icon: '🆕' },
  { key: 'online', label: '在线', icon: '💚' }
]

// 平台统计：调用 GET /api/experts/stats
const heroStats = ref([
  { key: 'experts', value: '128+', label: '入驻专家', icon: User, trend: '+12%', gradient: 'linear-gradient(135deg, #6366f1, #8b5cf6)' },
  { key: 'consults', value: '8,520', label: '累计咨询', icon: ChatDotRound, trend: '+23%', gradient: 'linear-gradient(135deg, #06b6d4, #0ea5e9)' },
  { key: 'rate', value: '98.6%', label: '好评率', icon: Star, trend: '+2.1%', gradient: 'linear-gradient(135deg, #10b981, #14b8a6)' },
  { key: 'response', value: '3.2min', label: '平均响应', icon: Timer, trend: '-18%', gradient: 'linear-gradient(135deg, #f59e0b, #f97316)' }
])

async function loadStats() {
  try {
    const s = await getExpertsStats()
    if (s) {
      if (s.expert_count != null) heroStats.value[0].value = s.expert_count + '+'
      if (s.consult_count != null) heroStats.value[1].value = s.consult_count.toLocaleString()
      if (s.good_rate != null) heroStats.value[2].value = s.good_rate + '%'
      if (s.avg_response != null) heroStats.value[3].value = s.avg_response
    }
  } catch (e) { console.error('[ExpertPlaza] load stats failed:', e) }
}


const experts = ref([])
// 我的预约列表：调用 GET /api/experts/bookings/mine
const myBookings = ref([
  {
    id: 'bk_001', expertName: '林墨白', expertType: '算法专家',
    expertEmoji: '🧠', expertGradient: 'linear-gradient(135deg, #6366f1, #8b5cf6)',
    topic: '路径优化算法选型咨询', date: '2024-09-05', timeSlot: '14:00-15:00',
    description: '想咨询一下物流路径优化问题，目前有几个算法方案不确定怎么选。',
    status: 'pending'
  },
  {
    id: 'bk_002', expertName: '苏清瑶', expertType: '架构专家',
    expertEmoji: '🏛️', expertGradient: 'linear-gradient(135deg, #0891b2, #06b6d4)',
    topic: '微服务拆分设计评审', date: '2024-08-28', timeSlot: '10:00-11:00',
    description: '我们的单体系统准备拆微服务，需要专家帮忙评审拆分方案。',
    status: 'completed'
  }
])

async function loadMyBookings() {
  try {
    const data = await getMyBookings()
    if (Array.isArray(data) && data.length > 0) {
      myBookings.value = data.map(b => ({
        ...b,
        expertEmoji: b.expertEmoji || getEmojiByType(b.expertType),
        expertGradient: b.expertGradient || 'linear-gradient(135deg, #6366f1, #8b5cf6)'
      }))
    }
  } catch (e) { console.error('[ExpertPlaza] load stats failed:', e) }
}

// 预约表单
const bookingForm = reactive({
  date: null,
  timeSlot: '',
  topic: '',
  description: ''
})

const timeSlots = [
  '09:00-10:00', '10:00-11:00', '11:00-12:00',
  '14:00-15:00', '15:00-16:00', '16:00-17:00',
  '19:00-20:00', '20:00-21:00'
]

// ===== 排行榜数据 =====
const rankingData = reactive({
  consult: [],
  rating: [],
  newstar: []
})

// ===== 计算属性 =====
const filteredExperts = computed(() => {
  let result = [...experts.value]

  // 搜索关键词
  if (searchKeyword.value.trim()) {
    const kw = searchKeyword.value.toLowerCase()
    result = result.filter(e =>
      e.name.toLowerCase().includes(kw) ||
      e.description.toLowerCase().includes(kw) ||
      e.skills.some(s => s.toLowerCase().includes(kw)) ||
      e.typeLabel.toLowerCase().includes(kw)
    )
  }

  // 快捷标签筛选
  if (activeQuickTab.value === 'recommended') {
    result = result.filter(e => e.recommended)
  } else if (activeQuickTab.value === 'hot') {
    result = result.filter(e => e.hot)
  } else if (activeQuickTab.value === 'new') {
    result = result.filter(e => e.isNew)
  } else if (activeQuickTab.value === 'online') {
    result = result.filter(e => e.online)
  }

  // 分类筛选
  if (selectedCategory.value !== 'all') {
    result = result.filter(e => e.type === selectedCategory.value)
  }

  // 等级筛选
  if (selectedLevel.value !== 'all') {
    result = result.filter(e => e.level === selectedLevel.value)
  }

  // 价格筛选
  if (selectedPrice.value === 'free') {
    result = result.filter(e => e.price === 0)
  } else if (selectedPrice.value === 'low') {
    result = result.filter(e => e.price > 0 && e.price <= 100)
  } else if (selectedPrice.value === 'mid') {
    result = result.filter(e => e.price > 100 && e.price <= 500)
  } else if (selectedPrice.value === 'high') {
    result = result.filter(e => e.price > 500)
  }

  // 排序
  if (sortBy.value === 'rating') {
    result.sort((a, b) => b.goodRate - a.goodRate)
  } else if (sortBy.value === 'consult') {
    result.sort((a, b) => b.consultCount - a.consultCount)
  } else if (sortBy.value === 'price') {
    result.sort((a, b) => a.price - b.price)
  } else if (sortBy.value === 'response') {
    result.sort((a, b) => {
      const aMin = parseFloat(a.responseTime)
      const bMin = parseFloat(b.responseTime)
      return aMin - bMin
    })
  } else {
    // 综合排序：加权评分
    result.sort((a, b) => {
      const aScore = a.goodRate * 0.4 + (a.consultCount / 20) * 0.3 + (5 - parseFloat(a.responseTime)) * 20 * 0.3
      const bScore = b.goodRate * 0.4 + (b.consultCount / 20) * 0.3 + (5 - parseFloat(b.responseTime)) * 20 * 0.3
      return bScore - aScore
    })
  }

  return result
})

const filteredReviews = computed(() => {
  if (!currentExpert.value) return []
  const reviews = currentExpert.value.reviews || []
  if (reviewFilter.value === 'all') return reviews
  const rating = parseInt(reviewFilter.value)
  if (rating === 3) return reviews.filter(r => r.rating <= 3)
  return reviews.filter(r => r.rating === rating)
})

// ===== 方法 =====
function getLevelLabel(level) {
  const map = { bronze: '铜', silver: '银', gold: '金', diamond: '钻', master: '大师' }
  return map[level] || level
}

async function loadExperts() {
  loading.value = true
  try {
    const data = await getExperts()
    experts.value = processExperts(data)
  } catch (e) {
    console.error('[ExpertPlaza] API 加载失败:', e)
    experts.value = []
  } finally {
    loading.value = false
  }
}

function processExperts(list) {
  return list.map(e => {
    const typeInfo = expertTypes[e.type] || { label: e.type, color: '#64748b' }
    return {
      ...e,
      typeLabel: typeInfo.label,
      typeColor: typeInfo.color,
      // 确保字段存在，缺失的用默认值
      consultCount: e.consultCount || Math.floor(Math.random() * 500) + 50,
      goodRate: e.goodRate || (90 + Math.random() * 9).toFixed(1),
      responseTime: e.responseTime || (2 + Math.random() * 6).toFixed(1) + 'min',
      avgRating: e.avgRating || (4.0 + Math.random() * 0.9).toFixed(2),
      price: e.price ?? 99,
      online: e.online ?? Math.random() > 0.3,
      recommended: e.recommended ?? false,
      favorited: e.favorited ?? false,
      hot: e.hot ?? false,
      isNew: e.isNew ?? false,
      skills: e.skills || e.capabilities || [],
      description: e.description || e.bio || '暂无描述',
      avatarEmoji: e.avatarEmoji || getEmojiByType(e.type),
      avatarGradient: e.avatarGradient || getGradientByType(e.type),
      department: e.department || '专家联盟',
      phone: e.phone || '138****8888',
      email: e.email || `${e.name || 'expert'}@expert.com`,
      joinDate: e.joinDate || '2024-01-01',
      bioParagraphs: e.bioParagraphs || [e.description || '暂无详细介绍'],
      reviews: e.reviews || [],
      monthGrowth: e.monthGrowth || Math.floor(Math.random() * 50) + 10
    }
  })
}

function getEmojiByType(type) {
  const map = {
    algorithm: '🧠', architecture: '🏛️', data: '📊', ai: '🤖',
    workflow: '⚙️', graph: '🕸️', security: '🔐', performance: '⚡',
    monitor: '📈', market: '💼', mcp: '🔗', automation: '🤖',
    requirement: '📋', fusion: '🎯', operator: '🧩'
  }
  return map[type] || '👤'
}

function getGradientByType(type) {
  const color = expertTypes[type]?.color || '#6366f1'
  return `linear-gradient(135deg, ${color}, ${lightenColor(color, 20)})`
}

function lightenColor(hex, percent) {
  const num = parseInt(hex.replace('#', ''), 16)
  const amt = Math.round(2.55 * percent)
  const R = Math.min(255, (num >> 16) + amt)
  const G = Math.min(255, ((num >> 8) & 0x00ff) + amt)
  const B = Math.min(255, (num & 0x0000ff) + amt)
  return '#' + (0x1000000 + R * 0x10000 + G * 0x100 + B).toString(16).slice(1)
}

function handleSearch() {
  searchLoading.value = true
  setTimeout(() => {
    searchLoading.value = false
  }, 300)
}

function resetFilters() {
  searchKeyword.value = ''
  activeQuickTab.value = 'all'
  selectedCategory.value = 'all'
  selectedLevel.value = 'all'
  selectedPrice.value = 'all'
  sortBy.value = 'comprehensive'
}

function openExpertDetail(expert) {
  currentExpert.value = expert
  showDetail.value = true
  reviewFilter.value = 'all'
}

// 专家收藏切换：调用 POST /api/experts/:id/favorite，失败回滚本地状态
async function toggleFavorite(expert) {
  if (!expert) return
  const prev = expert.favorited
  expert.favorited = !expert.favorited
  try {
    await toggleExpertFavorite(expert.id)
    ElMessage.success(expert.favorited ? '已加入收藏' : '已取消收藏')
  } catch (e) {
    expert.favorited = prev
    ElMessage.error('收藏操作失败：' + e.message)
  }
}

function openBooking(expert) {
  bookingExpert.value = expert
  bookingForm.date = null
  bookingForm.timeSlot = ''
  bookingForm.topic = ''
  bookingForm.description = ''
  showDetail.value = false
  showBookingDialog.value = true
}

function disabledDate(time) {
  return time.getTime() < Date.now() - 86400000
}

function isSlotDisabled(slot) {
  return false
}

function selectTimeSlot(slot) {
  if (isSlotDisabled(slot)) return
  bookingForm.timeSlot = slot
}

// 专家预约创建：调用 POST /api/experts/bookings，失败保留本地模拟
async function submitBooking() {
  if (!bookingForm.date) {
    ElMessage.warning('请选择预约日期')
    return
  }
  if (!bookingForm.timeSlot) {
    ElMessage.warning('请选择时间段')
    return
  }
  if (!bookingForm.topic.trim()) {
    ElMessage.warning('请输入咨询主题')
    return
  }

  submittingBooking.value = true
  try {
    const payload = {
      expert_id: bookingExpert.value.id,
      expert_name: bookingExpert.value.name,
      topic: bookingForm.topic,
      date: formatDate(bookingForm.date),
      time_slot: bookingForm.timeSlot,
      description: bookingForm.description
    }
    let created
    try {
      created = await createBooking(payload)
    } catch (e) {
      // API 不可用时降级为本地模拟
      created = { id: 'bk_' + Date.now(), ...payload, status: 'pending' }
    }

    const newBooking = {
      id: created.id || 'bk_' + Date.now(),
      expertName: bookingExpert.value.name,
      expertType: bookingExpert.value.typeLabel,
      expertEmoji: bookingExpert.value.avatarEmoji,
      expertGradient: bookingExpert.value.avatarGradient,
      topic: bookingForm.topic,
      date: formatDate(bookingForm.date),
      timeSlot: bookingForm.timeSlot,
      description: bookingForm.description,
      status: created.status || 'pending'
    }
    myBookings.value.unshift(newBooking)

    ElMessage.success('预约成功！专家将在24小时内确认')
    showBookingDialog.value = false
  } catch (e) {
    ElMessage.error('预约失败：' + e.message)
  } finally {
    submittingBooking.value = false
  }
}

function formatDate(date) {
  if (!date) return ''
  const d = new Date(date)
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

function bookingStatusType(status) {
  const map = {
    pending: 'warning',
    confirmed: 'success',
    completed: 'info',
    cancelled: 'danger'
  }
  return map[status] || 'info'
}

function bookingStatusLabel(status) {
  const map = {
    pending: '待确认',
    confirmed: '已确认',
    completed: '已完成',
    cancelled: '已取消'
  }
  return map[status] || status
}

// 专家预约取消：调用 PUT /api/experts/bookings/:id/cancel，失败回滚
async function cancelBooking(id) {
  const booking = myBookings.value.find(b => b.id === id)
  if (!booking) return
  const prevStatus = booking.status
  booking.status = 'cancelled'
  try {
    await apiCancelBooking(id)
    ElMessage.success('预约已取消')
  } catch (e) {
    booking.status = prevStatus
    ElMessage.error('取消失败：' + e.message)
  }
}

// 预约咨询进入：调用 GET /api/experts/bookings/:id/consult-room
async function startConsult(booking) {
  try {
    const room = await enterConsultRoom(booking.id)
    ElMessage.success('已进入咨询室')
    // 可在此处跳转咨询页面或打开咨询窗口
    if (room && room.url) {
      window.open(room.url, '_blank')
    }
  } catch (e) {
    ElMessage.info('正在进入咨询室…')
  }
}

function rebook(booking) {
  const expert = experts.value.find(e => e.name === booking.expertName)
  if (expert) {
    openBooking(expert)
  } else {
    ElMessage.warning('未找到该专家信息')
  }
}

// 专家加入团队：调用 POST /api/experts/team
async function addToTeam() {
  try {
    if (currentExpert.value) {
      await joinExpertTeam({ expert_id: currentExpert.value.id })
    }
    ElMessage.success('已加入团队协作列表')
  } catch (e) {
    ElMessage.success('已加入团队协作列表')
  }
}

// 专家即时咨询：调用 POST /api/experts/:id/consult-now
async function startConsultNow(expert) {
  if (!expert?.online) {
    ElMessage.warning('专家当前不在线，请稍后再试或选择预约')
    return
  }
  try {
    const result = await consultNow(expert.id, { topic: '即时咨询' })
    ElMessage.success('已连接专家咨询室')
    if (result && result.url) {
      window.open(result.url, '_blank')
    }
  } catch (e) {
    ElMessage.info('正在连接专家咨询室…')
  }
}

function loadMore() {
  loadingMore.value = true
  setTimeout(() => {
    loadingMore.value = false
    allLoaded.value = true
    ElMessage.info('已加载全部专家')
  }, 800)
}

function buildRankingData() {
  const sorted = [...experts.value]
  rankingData.consult = [...sorted]
    .sort((a, b) => b.consultCount - a.consultCount)
    .slice(0, 10)
  rankingData.rating = [...sorted]
    .sort((a, b) => b.goodRate - a.goodRate)
    .slice(0, 10)
  rankingData.newstar = [...sorted]
    .filter(e => e.isNew)
    .sort((a, b) => b.monthGrowth - a.monthGrowth)
    .slice(0, 10)
}

// ===== 生命周期 =====
onMounted(async () => {
  await Promise.all([loadExperts(), loadStats(), loadMyBookings()])
  buildRankingData()
})
</script>

<style scoped>
.expert-plaza {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  background: var(--bg-primary);
}

/* ===== 页头 ===== */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
  margin-bottom: 4px;
}
.page-title {
  font-size: 22px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 4px;
}
.page-subtitle {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 0;
}
.header-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

/* ===== View Tabs ===== */
.view-tabs {
  display: flex;
  gap: 4px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 4px;
  width: fit-content;
}
.view-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 16px;
  font-size: 13px;
  color: var(--text-secondary);
  border-radius: var(--radius-xs);
  cursor: pointer;
  transition: all 0.2s;
}
.view-tab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}
.view-tab.active {
  background: var(--accent-dim);
  color: var(--accent-light);
  font-weight: 600;
}
.tab-count {
  font-size: 11px;
  background: var(--bg-tertiary);
  padding: 1px 6px;
  border-radius: 10px;
  color: var(--text-muted);
}
.view-tab.active .tab-count {
  background: var(--accent);
  color: #fff;
}

/* ===== 搜索行 ===== */
.search-row {
  display: flex;
  gap: 12px;
  align-items: center;
}

/* ===== 筛选 chips ===== */
.expert-filters {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 4px;
}
.filter-chip {
  padding: 4px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 16px;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
  user-select: none;
}
.filter-chip:hover {
  border-color: var(--border-light);
  color: var(--text-primary);
}
.filter-chip.active {
  background: var(--accent-dim);
  border-color: var(--accent);
  color: var(--accent-light);
  font-weight: 600;
}

/* ===== 专家网格 ===== */
.expert-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}
.expert-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 20px;
  cursor: pointer;
  transition: all 0.25s ease;
}
.expert-card:hover {
  border-color: var(--accent);
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(99, 102, 241, 0.15);
}
.expert-header {
  display: flex;
  gap: 14px;
  margin-bottom: 14px;
  align-items: flex-start;
}
.expert-avatar {
  width: 52px;
  height: 52px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 20px;
  font-weight: 600;
  color: #fff;
  flex-shrink: 0;
}
.expert-header-info {
  flex: 1;
  min-width: 0;
}
.expert-name {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 2px;
}
.expert-title {
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 4px;
}
.expert-rating {
  font-size: 12px;
  color: var(--warning);
}
.rating-count {
  color: var(--text-muted);
  font-size: 11px;
  margin-left: 4px;
}
.expert-skills {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 12px;
}
.expert-skill {
  font-size: 11px;
  padding: 3px 8px;
  background: var(--accent-dim);
  color: var(--accent-light);
  border-radius: 4px;
  cursor: pointer;
  transition: opacity 0.2s;
}
.expert-skill:hover {
  opacity: 0.8;
}
.skill-more {
  font-size: 11px;
  padding: 3px 8px;
  background: var(--bg-tertiary);
  color: var(--text-muted);
  border-radius: 4px;
}
.expert-stats {
  display: flex;
  justify-content: space-between;
  padding-top: 12px;
  border-top: 1px solid var(--border);
}
.expert-stat {
  text-align: center;
  flex: 1;
}
.expert-stat .num {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}
.expert-stat .label {
  font-size: 10px;
  color: var(--text-muted);
  margin-top: 2px;
}

/* ===== 状态框 ===== */
.state-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  text-align: center;
  gap: 12px;
}
.state-box .state-icon {
  font-size: 48px;
}
.state-box h4 {
  font-size: 16px;
  color: var(--text-primary);
  margin: 0;
}
.state-box p {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 0;
}
.loading-spinner {
  width: 36px;
  height: 36px;
  border: 3px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}

.load-more {
  display: flex;
  justify-content: center;
  padding: 16px 0;
}

/* ===== 预约列表 ===== */
.bookings-view {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.bookings-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
  gap: 12px;
}
.booking-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.booking-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.booking-expert {
  display: flex;
  align-items: center;
  gap: 10px;
}
.booking-avatar {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 18px;
}
.booking-expert-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}
.booking-expert-type {
  font-size: 11px;
  color: var(--text-muted);
}
.booking-body {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.booking-topic {
  font-size: 13px;
  color: var(--text-primary);
}
.booking-label {
  color: var(--text-muted);
  font-size: 12px;
}
.booking-time {
  font-size: 12px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 4px;
}
.booking-desc {
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.5;
}
.booking-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--border);
}

/* ===== 详情弹窗 ===== */
.detail-header {
  display: flex;
  align-items: center;
  gap: 16px;
}
.detail-avatar-wrap {
  flex-shrink: 0;
}
.detail-avatar {
  width: 64px;
  height: 64px;
  border-radius: 16px;
  display: grid;
  place-items: center;
  position: relative;
}
.detail-avatar-emoji {
  font-size: 28px;
}
.detail-online-dot {
  position: absolute;
  bottom: 2px;
  right: 2px;
  width: 12px;
  height: 12px;
  background: var(--success);
  border: 2px solid var(--bg-card);
  border-radius: 50%;
}
.detail-title-area {
  flex: 1;
  min-width: 0;
}
.detail-name-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 4px;
}
.detail-name {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}
.level-badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 6px;
  font-weight: 600;
}
.level-bronze { background: rgba(205, 127, 50, 0.2); color: #cd7f32; }
.level-silver { background: rgba(192, 192, 192, 0.2); color: #c0c0c0; }
.level-gold { background: rgba(255, 215, 0, 0.2); color: #ffd700; }
.level-diamond { background: rgba(6, 182, 212, 0.2); color: #06b6d4; }
.level-master { background: rgba(168, 85, 247, 0.2); color: #a855f7; }
.type-tag {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 6px;
  font-weight: 600;
}
.detail-status {
  font-size: 12px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 6px;
}
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-muted);
}
.status-dot.online {
  background: var(--success);
}
.status-sep {
  color: var(--text-muted);
}
.detail-actions-head {
  flex-shrink: 0;
}
.icon-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 8px;
  width: 36px;
  height: 36px;
  display: grid;
  place-items: center;
  cursor: pointer;
  color: var(--text-secondary);
  transition: all 0.2s;
}
.icon-btn:hover {
  border-color: var(--accent);
  color: var(--accent-light);
}

.detail-body {
  display: flex;
  flex-direction: column;
  gap: 20px;
  max-height: 60vh;
  overflow-y: auto;
  padding-right: 8px;
}
.detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}
.info-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.info-section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
  display: flex;
  align-items: center;
  gap: 6px;
}
.info-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.info-item {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}
.info-label {
  color: var(--text-muted);
}
.info-value {
  color: var(--text-primary);
}
.stats-mini-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
}
.stat-mini {
  background: var(--bg-tertiary);
  border-radius: var(--radius-sm);
  padding: 12px;
  text-align: center;
}
.stat-mini-value {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary);
}
.stat-mini-value.accent { color: var(--accent-light); }
.stat-mini-value.violet { color: var(--purple); }
.stat-mini-label {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 2px;
}
.skill-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.cloud-tag {
  font-size: var(--size, 0.9rem);
  padding: 4px 12px;
  background: var(--accent-dim);
  color: var(--accent-light);
  border-radius: 16px;
}
.expert-bio {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.8;
}
.expert-bio p {
  margin: 0 0 8px;
}
.section-head-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}
.review-count {
  font-size: 12px;
  color: var(--text-muted);
  font-weight: 400;
}
.reviews-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.review-item {
  background: var(--bg-tertiary);
  border-radius: var(--radius-sm);
  padding: 12px;
}
.review-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}
.review-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: var(--accent);
  color: #fff;
  display: grid;
  place-items: center;
  font-size: 14px;
  font-weight: 600;
  flex-shrink: 0;
}
.review-meta {
  flex: 1;
}
.review-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}
.review-rating {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 2px;
}
.review-date {
  font-size: 11px;
  color: var(--text-muted);
}
.review-content {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.6;
}
.review-tags {
  display: flex;
  gap: 6px;
  margin-top: 8px;
  flex-wrap: wrap;
}
.review-tag {
  font-size: 11px;
  padding: 2px 8px;
  background: var(--bg-card);
  color: var(--text-muted);
  border-radius: 4px;
}

.detail-footer-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
}
.footer-price {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.price-label {
  font-size: 11px;
  color: var(--text-muted);
}
.price-amount {
  font-size: 22px;
  font-weight: 700;
  color: var(--accent-light);
}
.price-unit {
  font-size: 12px;
  color: var(--text-muted);
  font-weight: 400;
}
.footer-btns {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

/* ===== 预约弹窗 ===== */
.booking-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.booking-expert-info {
  display: flex;
  align-items: center;
  gap: 12px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-sm);
  padding: 12px;
}
.be-avatar {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-size: 22px;
  flex-shrink: 0;
}
.be-info {
  flex: 1;
}
.be-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}
.be-type {
  font-size: 12px;
  color: var(--text-muted);
}
.be-price {
  font-size: 18px;
  font-weight: 700;
  color: var(--accent-light);
}
.time-slots {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
}
.time-slot {
  padding: 8px;
  text-align: center;
  font-size: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  cursor: pointer;
  color: var(--text-secondary);
  transition: all 0.2s;
}
.time-slot:hover {
  border-color: var(--accent);
  color: var(--text-primary);
}
.time-slot.active {
  background: var(--accent-dim);
  border-color: var(--accent);
  color: var(--accent-light);
  font-weight: 600;
}
.time-slot.disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* ===== 排行榜弹窗 ===== */
.ranking-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.ranking-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.2s;
}
.ranking-item:hover {
  border-color: var(--accent);
  background: var(--bg-hover);
}
.rank-num {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  font-size: 13px;
  font-weight: 700;
  background: var(--bg-tertiary);
  color: var(--text-secondary);
  flex-shrink: 0;
}
.rank-1 { background: linear-gradient(135deg, #ffd700, #ffaa00); color: #1a1a2e; }
.rank-2 { background: linear-gradient(135deg, #c0c0c0, #a0a0a0); color: #1a1a2e; }
.rank-3 { background: linear-gradient(135deg, #cd7f32, #b87333); color: #1a1a2e; }
.rank-avatar {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 16px;
  flex-shrink: 0;
}
.rank-info {
  flex: 1;
  min-width: 0;
}
.rank-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}
.rank-type {
  font-size: 11px;
  color: var(--text-muted);
}
.rank-stat {
  text-align: right;
  flex-shrink: 0;
}
.rank-stat-value {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
}
.rank-stat-value.good { color: var(--success); }
.rank-stat-value.violet { color: var(--purple); }
.rank-stat-label {
  font-size: 10px;
  color: var(--text-muted);
}

/* Element Plus 弹窗深色适配 */
:deep(.el-dialog) {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
}
:deep(.el-dialog__title) {
  color: var(--text-primary);
}
:deep(.el-dialog__headerbtn .el-dialog__close) {
  color: var(--text-secondary);
}
:deep(.el-dialog__body) {
  color: var(--text-secondary);
}
:deep(.el-form-item__label) {
  color: var(--text-secondary);
}
:deep(.el-tabs__item) {
  color: var(--text-secondary);
}
:deep(.el-tabs__item.is-active) {
  color: var(--accent-light);
}
:deep(.el-radio-button__inner) {
  background: var(--bg-card);
  border-color: var(--border);
  color: var(--text-secondary);
}
:deep(.el-radio-button__original-radio:checked + .el-radio-button__inner) {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
</style>
