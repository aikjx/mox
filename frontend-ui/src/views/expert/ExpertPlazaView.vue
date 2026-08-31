<template>
  <div class="expert-plaza">
    <!-- ===== Hero 区 ===== -->
    <section class="hero-section">
      <div class="hero-bg">
        <div class="hero-blob blob-1"></div>
        <div class="hero-blob blob-2"></div>
        <div class="hero-blob blob-3"></div>
        <div class="hero-grid-overlay"></div>
      </div>

      <div class="hero-content">
        <div class="hero-text">
          <div class="hero-badge">
            <span class="hero-badge-dot"></span>
            专家联盟 · 智慧中枢
          </div>
          <h1 class="hero-title">
            <span class="gradient-text">专家联盟广场</span>
          </h1>
          <p class="hero-subtitle">
            汇聚各领域顶尖专家，为您的项目保驾护航 · 智能匹配 · 实时咨询 · 精准预约
          </p>
        </div>

        <!-- 搜索框 -->
        <div class="hero-search">
          <div class="search-box glass-card">
            <el-icon class="search-icon"><Search /></el-icon>
            <input
              v-model="searchKeyword"
              type="text"
              class="search-input"
              placeholder="搜索专家名称 / 技能 / 领域…"
              @keyup.enter="handleSearch"
            />
            <el-button
              type="primary"
              class="search-btn"
              :loading="searchLoading"
              @click="handleSearch"
            >
              搜索
            </el-button>
          </div>
        </div>

        <!-- 快捷筛选标签 -->
        <div class="quick-filters">
          <span
            v-for="tab in quickTabs"
            :key="tab.key"
            class="quick-tab"
            :class="{ active: activeQuickTab === tab.key }"
            @click="activeQuickTab = tab.key"
          >
            <span class="quick-tab-icon">{{ tab.icon }}</span>
            {{ tab.label }}
          </span>
        </div>

        <!-- 统计数字卡片 -->
        <div class="stats-grid">
          <div
            v-for="(stat, idx) in heroStats"
            :key="stat.key"
            class="stat-card glass-card"
            :style="{ '--delay': idx * 0.1 + 's' }"
          >
            <div class="stat-icon" :style="{ background: stat.gradient }">
              <el-icon><component :is="stat.icon" /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-value">{{ stat.value }}</div>
              <div class="stat-label">{{ stat.label }}</div>
            </div>
            <div class="stat-trend up" v-if="stat.trend">
              <el-icon><TrendCharts /></el-icon>
              {{ stat.trend }}
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- ===== 主内容区 ===== -->
    <section class="main-section">
      <div class="plaza-layout">
        <!-- 左侧筛选边栏 -->
        <aside class="filter-sidebar glass-card">
          <div class="sidebar-header">
            <h3 class="sidebar-title">
              <el-icon><Filter /></el-icon>
              筛选条件
            </h3>
            <el-button link type="primary" size="small" @click="resetFilters">
              重置
            </el-button>
          </div>

          <!-- 专家分类 -->
          <div class="filter-group">
            <div class="filter-group-title">
              <el-icon><Grid /></el-icon>
              专家分类
            </div>
            <div class="filter-tags">
              <span
                v-for="cat in categories"
                :key="cat.key"
                class="filter-tag"
                :class="{ active: selectedCategory === cat.key }"
                @click="selectedCategory = cat.key"
              >
                <span class="tag-dot" :style="{ background: cat.color }"></span>
                {{ cat.label }}
              </span>
            </div>
          </div>

          <!-- 经验等级 -->
          <div class="filter-group">
            <div class="filter-group-title">
              <el-icon><Medal /></el-icon>
              经验等级
            </div>
            <div class="filter-tags">
              <span
                v-for="level in levels"
                :key="level.key"
                class="filter-tag level-tag"
                :class="{ active: selectedLevel === level.key, [`level-${level.key}`]: true }"
                @click="selectedLevel = level.key"
              >
                {{ level.label }}
              </span>
            </div>
          </div>

          <!-- 价格区间 -->
          <div class="filter-group">
            <div class="filter-group-title">
              <el-icon><Wallet /></el-icon>
              价格区间
            </div>
            <div class="price-options">
              <span
                v-for="price in priceRanges"
                :key="price.key"
                class="price-option"
                :class="{ active: selectedPrice === price.key }"
                @click="selectedPrice = price.key"
              >
                {{ price.label }}
              </span>
            </div>
          </div>

          <!-- 排序方式 -->
          <div class="filter-group">
            <div class="filter-group-title">
              <el-icon><Sort /></el-icon>
              排序方式
            </div>
            <el-radio-group v-model="sortBy" class="sort-radio">
              <el-radio-button
                v-for="s in sortOptions"
                :key="s.key"
                :value="s.key"
              >
                {{ s.label }}
              </el-radio-button>
            </el-radio-group>
          </div>

          <!-- 排行榜入口 -->
          <div class="ranking-entry" @click="showRanking = true">
            <div class="ranking-entry-icon">
              <el-icon><Trophy /></el-icon>
            </div>
            <div class="ranking-entry-text">
              <div class="ranking-entry-title">专家排行榜</div>
              <div class="ranking-entry-desc">月度咨询榜 · 好评榜 · 新星榜</div>
            </div>
            <el-icon class="ranking-entry-arrow"><ArrowRight /></el-icon>
          </div>

          <!-- 我的预约入口 -->
          <div class="booking-entry" @click="activeTab = 'bookings'">
            <div class="booking-entry-icon">
              <el-icon><Calendar /></el-icon>
            </div>
            <div class="booking-entry-text">
              <div class="booking-entry-title">我的预约</div>
              <div class="booking-entry-desc">{{ myBookings.length }} 条预约记录</div>
            </div>
            <el-icon class="booking-entry-arrow"><ArrowRight /></el-icon>
          </div>
        </aside>

        <!-- 主内容：专家卡片列表 / 我的预约 -->
        <main class="content-area">
          <!-- Tab 切换 -->
          <div class="content-tabs">
            <div
              class="content-tab"
              :class="{ active: activeTab === 'experts' }"
              @click="activeTab = 'experts'"
            >
              <el-icon><User /></el-icon>
              专家发现
              <span class="tab-count">{{ filteredExperts.length }}</span>
            </div>
            <div
              class="content-tab"
              :class="{ active: activeTab === 'bookings' }"
              @click="activeTab = 'bookings'"
            >
              <el-icon><Calendar /></el-icon>
              我的预约
              <span class="tab-count" v-if="myBookings.length">{{ myBookings.length }}</span>
            </div>
          </div>

          <!-- 专家列表 -->
          <div v-show="activeTab === 'experts'" class="experts-container">
            <div v-if="loading" class="loading-state">
              <div class="loading-spinner"></div>
              <p>正在加载专家列表…</p>
            </div>

            <div v-else-if="filteredExperts.length === 0" class="empty-state">
              <div class="empty-icon">🔍</div>
              <h4>未找到匹配的专家</h4>
              <p>试试调整筛选条件或搜索关键词</p>
              <el-button type="primary" @click="resetFilters">重置筛选</el-button>
            </div>

            <div v-else class="experts-grid">
              <div
                v-for="(expert, idx) in filteredExperts"
                :key="expert.id"
                class="expert-card glass-card"
                :style="{ '--delay': (idx % 8) * 0.05 + 's' }"
                @click="openExpertDetail(expert)"
              >
                <!-- 卡片顶部 -->
                <div class="card-top">
                  <div class="expert-avatar-lg" :style="{ background: expert.avatarGradient }">
                    <span class="avatar-emoji">{{ expert.avatarEmoji }}</span>
                    <span
                      v-if="expert.online"
                      class="online-dot"
                      :title="expert.online ? '在线' : '离线'"
                    ></span>
                  </div>
                  <button
                    class="favorite-btn"
                    :class="{ favorited: expert.favorited }"
                    @click.stop="toggleFavorite(expert)"
                  >
                    <el-icon :size="18">
                      <Star v-if="expert.favorited" :fill="'#f59e0b'" />
                      <Star v-else />
                    </el-icon>
                  </button>
                </div>

                <!-- 专家信息 -->
                <div class="expert-body">
                  <div class="expert-name-row">
                    <h3 class="expert-name">{{ expert.name }}</h3>
                    <span
                      class="level-badge"
                      :class="`level-${expert.level}`"
                    >
                      {{ getLevelLabel(expert.level) }}
                    </span>
                  </div>

                  <div class="expert-type-row">
                    <span
                      class="type-tag"
                      :style="{ background: expert.typeColor + '20', color: expert.typeColor }"
                    >
                      {{ expert.typeLabel }}
                    </span>
                    <span v-if="expert.recommended" class="recommend-tag">
                      <el-icon><Fire /></el-icon>
                      推荐
                    </span>
                  </div>

                  <p class="expert-desc">{{ expert.description }}</p>

                  <!-- 技能标签 -->
                  <div class="skill-tags">
                    <span
                      v-for="skill in expert.skills.slice(0, 4)"
                      :key="skill"
                      class="skill-tag"
                      @click.stop="searchKeyword = skill; handleSearch()"
                    >
                      {{ skill }}
                    </span>
                    <span v-if="expert.skills.length > 4" class="skill-more">
                      +{{ expert.skills.length - 4 }}
                    </span>
                  </div>
                </div>

                <!-- 底部统计栏 -->
                <div class="card-stats">
                  <div class="card-stat">
                    <el-icon><ChatDotRound /></el-icon>
                    <span>{{ expert.consultCount }}</span>
                  </div>
                  <div class="card-stat">
                    <el-icon><Star /></el-icon>
                    <span>{{ expert.goodRate }}%</span>
                  </div>
                  <div class="card-stat">
                    <el-icon><Timer /></el-icon>
                    <span>{{ expert.responseTime }}</span>
                  </div>
                </div>

                <!-- 底部操作 -->
                <div class="card-footer">
                  <div class="price-info">
                    <span class="price-value">
                      <span v-if="expert.price === 0">免费</span>
                      <span v-else>¥{{ expert.price }}<span class="price-unit">/次</span></span>
                    </span>
                  </div>
                  <el-button
                    type="primary"
                    size="small"
                    class="consult-btn"
                    @click.stop="openBooking(expert)"
                  >
                    立即咨询
                  </el-button>
                </div>
              </div>
            </div>

            <!-- 加载更多 -->
            <div v-if="filteredExperts.length > 0 && !allLoaded" class="load-more">
              <el-button plain :loading="loadingMore" @click="loadMore">
                加载更多
              </el-button>
            </div>
          </div>

          <!-- 我的预约列表 -->
          <div v-show="activeTab === 'bookings'" class="bookings-container">
            <div v-if="myBookings.length === 0" class="empty-state">
              <div class="empty-icon">📅</div>
              <h4>暂无预约记录</h4>
              <p>快去专家广场预约您心仪的专家吧</p>
              <el-button type="primary" @click="activeTab = 'experts'">浏览专家</el-button>
            </div>

            <div v-else class="bookings-list">
              <div
                v-for="booking in myBookings"
                :key="booking.id"
                class="booking-card glass-card"
              >
                <div class="booking-header">
                  <div class="booking-expert">
                    <div
                      class="booking-avatar"
                      :style="{ background: booking.expertGradient }"
                    >
                      {{ booking.expertEmoji }}
                    </div>
                    <div>
                      <div class="booking-expert-name">{{ booking.expertName }}</div>
                      <div class="booking-expert-type">{{ booking.expertType }}</div>
                    </div>
                  </div>
                  <el-tag
                    :type="bookingStatusType(booking.status)"
                    effect="light"
                    size="small"
                    round
                  >
                    {{ bookingStatusLabel(booking.status) }}
                  </el-tag>
                </div>

                <div class="booking-body">
                  <div class="booking-topic">
                    <span class="booking-label">咨询主题：</span>
                    {{ booking.topic }}
                  </div>
                  <div class="booking-time">
                    <el-icon><Calendar /></el-icon>
                    {{ booking.date }} {{ booking.timeSlot }}
                  </div>
                  <div class="booking-desc" v-if="booking.description">
                    {{ booking.description }}
                  </div>
                </div>

                <div class="booking-footer">
                  <el-button size="small" @click="cancelBooking(booking.id)" v-if="booking.status === 'pending'">
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
        </main>
      </div>
    </section>

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
            <div
              class="detail-avatar"
              :style="{ background: currentExpert?.avatarGradient }"
            >
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
                :style="{ background: currentExpert?.typeColor + '20', color: currentExpert?.typeColor }"
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
        <!-- 基本信息 + 数据统计 -->
        <div class="detail-grid">
          <!-- 左侧：基本信息 -->
          <div class="detail-info-col">
            <div class="info-section">
              <h4 class="info-section-title">
                <el-icon><InfoFilled /></el-icon>
                基本信息
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

            <!-- 数据统计 -->
            <div class="info-section">
              <h4 class="info-section-title">
                <el-icon><DataAnalysis /></el-icon>
                数据统计
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

          <!-- 右侧：专业技能 -->
          <div class="detail-skills-col">
            <div class="info-section">
              <h4 class="info-section-title">
                <el-icon><MagicStick /></el-icon>
                专业技能
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

        <!-- 专家介绍 -->
        <div class="info-section">
          <h4 class="info-section-title">
            <el-icon><Document /></el-icon>
            专家介绍
          </h4>
          <div class="expert-bio">
            <p v-for="(para, i) in currentExpert.bioParagraphs" :key="i">{{ para }}</p>
          </div>
        </div>

        <!-- 用户评价 -->
        <div class="info-section">
          <div class="section-head-row">
            <h4 class="info-section-title">
              <el-icon><ChatLineSquare /></el-icon>
              用户评价
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
                    <el-rate
                      :model-value="review.rating"
                      disabled
                      size="small"
                    />
                    <span class="review-date">{{ review.date }}</span>
                  </div>
                </div>
              </div>
              <div class="review-content">{{ review.content }}</div>
              <div class="review-tags" v-if="review.tags?.length">
                <span v-for="tag in review.tags" :key="tag" class="review-tag">
                  {{ tag }}
                </span>
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
              <el-icon><UserFilled /></el-icon>
              加入团队
            </el-button>
            <el-button type="primary" @click="openBooking(currentExpert)">
              <el-icon><Calendar /></el-icon>
              预约专家
            </el-button>
            <el-button
              type="success"
              @click="startConsultNow(currentExpert)"
              v-if="currentExpert?.online"
            >
              <el-icon><ChatDotRound /></el-icon>
              发起咨询
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
        <div class="booking-expert-info glass-card">
          <div
            class="be-avatar"
            :style="{ background: bookingExpert.avatarGradient }"
          >
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
  Calendar, User, ChatDotRound, Star, Timer, Fire, InfoFilled,
  DataAnalysis, MagicStick, Document, ChatLineSquare, UserFilled,
  TrendCharts
} from '@element-plus/icons-vue'
import { getExperts, getExpert } from '@/api'

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
  { key: 'all', label: '全部', color: '#64748b' },
  { key: 'algorithm', label: '算法', color: '#6366f1' },
  { key: 'architecture', label: '架构', color: '#0891b2' },
  { key: 'ai', label: 'AI', color: '#ec4899' },
  { key: 'data', label: '数据', color: '#10b981' },
  { key: 'workflow', label: '工作流', color: '#f59e0b' },
  { key: 'graph', label: '图谱', color: '#06b6d4' },
  { key: 'security', label: '安全', color: '#ef4444' },
  { key: 'performance', label: '性能', color: '#14b8a6' },
  { key: 'monitor', label: '监控', color: '#f97316' },
  { key: 'market', label: '市场', color: '#f43f5e' },
  { key: 'mcp', label: 'MCP', color: '#a855f7' },
  { key: 'automation', label: '自动化', color: '#0ea5e9' },
  { key: 'requirement', label: '需求', color: '#16a34a' },
  { key: 'fusion', label: '融合', color: '#7c3aed' },
  { key: 'operator', label: '算子', color: '#8b5cf6' }
]

const quickTabs = [
  { key: 'all', label: '全部', icon: '✨' },
  { key: 'recommended', label: '推荐', icon: '🔥' },
  { key: 'hot', label: '热门', icon: '⚡' },
  { key: 'new', label: '新入驻', icon: '🆕' },
  { key: 'online', label: '在线', icon: '💚' }
]

const heroStats = [
  { key: 'experts', value: '128+', label: '入驻专家', icon: User, trend: '+12%', gradient: 'linear-gradient(135deg, #6366f1, #8b5cf6)' },
  { key: 'consults', value: '8,520', label: '累计咨询', icon: ChatDotRound, trend: '+23%', gradient: 'linear-gradient(135deg, #06b6d4, #0ea5e9)' },
  { key: 'rate', value: '98.6%', label: '好评率', icon: Star, trend: '+2.1%', gradient: 'linear-gradient(135deg, #10b981, #14b8a6)' },
  { key: 'response', value: '3.2min', label: '平均响应', icon: Timer, trend: '-18%', gradient: 'linear-gradient(135deg, #f59e0b, #f97316)' }
]

// 专家 Mock 数据
const mockExperts = [
  {
    id: 'exp_001', name: '林墨白', type: 'algorithm', level: 'master',
    avatarEmoji: '🧠', avatarGradient: 'linear-gradient(135deg, #6366f1, #8b5cf6)',
    description: '十年算法研究经验，专注于复杂系统优化与智能决策算法设计。',
    skills: ['算法设计', '复杂度分析', '动态规划', '图算法', '机器学习', '深度学习'],
    consultCount: 1286, goodRate: 99.2, responseTime: '2.1min', avgRating: 4.95,
    price: 399, online: true, recommended: true, favorited: false, hot: true, isNew: false,
    department: '算法研究院', phone: '138****8888', email: 'linmobai@expert.com',
    joinDate: '2023-03-15', monthGrowth: 35,
    bioParagraphs: [
      '林墨白，资深算法专家，清华大学计算机科学博士，曾任某知名互联网公司首席算法工程师。',
      '在算法设计与分析领域拥有十余年深耕经验，主导过多个大型分布式系统的算法架构设计，在顶级会议发表论文 20 余篇。',
      '擅长将复杂业务问题抽象为算法模型，提供从理论分析到工程实现的全链路解决方案。'
    ],
    reviews: [
      { id: 'r1', userName: '张工', avatar: '张', rating: 5, date: '2024-08-15', content: '林老师的算法分析非常专业，一针见血地指出了我们系统的性能瓶颈，给出的优化方案效果显著！', tags: ['专业', '高效', '耐心'] },
      { id: 'r2', userName: '李产品', avatar: '李', rating: 5, date: '2024-08-10', content: '咨询了一个路径优化的问题，林老师给出了多种方案对比，还附带了复杂度分析，超值！', tags: ['专业', '思路清晰'] },
      { id: 'r3', userName: '王架构师', avatar: '王', rating: 4, date: '2024-08-05', content: '整体不错，就是响应稍微慢了点，可能是咨询的人太多了。', tags: ['专业'] }
    ]
  },
  {
    id: 'exp_002', name: '苏清瑶', type: 'architecture', level: 'diamond',
    avatarEmoji: '🏛️', avatarGradient: 'linear-gradient(135deg, #0891b2, #06b6d4)',
    description: '资深架构师，精通微服务、云原生架构设计，擅长高并发系统设计。',
    skills: ['微服务', '云原生', 'K8s', '高并发', 'DDD', '分布式事务'],
    consultCount: 956, goodRate: 98.5, responseTime: '3.5min', avgRating: 4.9,
    price: 299, online: true, recommended: true, favorited: false, hot: true, isNew: false,
    department: '架构设计部', phone: '139****6666', email: 'suqingyao@expert.com',
    joinDate: '2023-05-20', monthGrowth: 28,
    bioParagraphs: [
      '苏清瑶，资深架构师，拥有 12 年软件架构设计经验。',
      '曾主导多个千万级用户系统的架构设计与重构，在微服务、云原生、领域驱动设计方面有深厚积累。',
      '善于从业务视角出发设计技术架构，确保系统的可扩展性、可维护性和可演进性。'
    ],
    reviews: [
      { id: 'r4', userName: '陈技术总监', avatar: '陈', rating: 5, date: '2024-08-12', content: '苏老师的架构评审非常到位，帮我们规避了好几个潜在的技术坑，强烈推荐！', tags: ['专业', '经验丰富'] }
    ]
  },
  {
    id: 'exp_003', name: '周知行', type: 'ai', level: 'gold',
    avatarEmoji: '🤖', avatarGradient: 'linear-gradient(135deg, #ec4899, #f472b6)',
    description: 'AI 应用专家，专注大模型应用开发与 Agent 设计，落地经验丰富。',
    skills: ['LLM', 'Agent', 'RAG', 'Prompt工程', 'Fine-tuning', '多模态'],
    consultCount: 2150, goodRate: 97.8, responseTime: '1.8min', avgRating: 4.85,
    price: 199, online: true, recommended: true, favorited: true, hot: true, isNew: false,
    department: 'AI 应用实验室', phone: '137****5555', email: 'zhouzhixing@expert.com',
    joinDate: '2023-08-10', monthGrowth: 45,
    bioParagraphs: [
      '周知行，AI 应用专家，专注于大语言模型的工程化落地。',
      '曾在多家头部 AI 公司负责 LLM 应用开发，主导过多个智能助手、知识库问答系统的从 0 到 1 建设。',
      '在 RAG 优化、Agent 设计、Prompt Engineering 方面有丰富的实战经验。'
    ],
    reviews: [
      { id: 'r5', userName: '赵产品经理', avatar: '赵', rating: 5, date: '2024-08-18', content: '周老师帮我们设计的 RAG 方案效果提升明显，检索准确率提高了 30%！', tags: ['实用', '高效', '专业'] }
    ]
  },
  {
    id: 'exp_004', name: '钱若水', type: 'data', level: 'gold',
    avatarEmoji: '📊', avatarGradient: 'linear-gradient(135deg, #10b981, #14b8a6)',
    description: '数据科学家，精通数据分析、数据建模与数据可视化。',
    skills: ['数据分析', 'SQL', 'Python', '数据可视化', '统计建模', 'BI'],
    consultCount: 678, goodRate: 96.5, responseTime: '4.2min', avgRating: 4.7,
    price: 159, online: false, recommended: false, favorited: false, hot: false, isNew: false,
    department: '数据智能部', phone: '136****4444', email: 'qianruoshui@expert.com',
    joinDate: '2023-11-01', monthGrowth: 22,
    bioParagraphs: [
      '钱若水，数据科学家，统计学硕士。',
      '擅长从海量数据中挖掘商业洞察，为业务决策提供数据支撑。精通各类数据分析工具与可视化技术。'
    ],
    reviews: [
      { id: 'r6', userName: '孙运营', avatar: '孙', rating: 5, date: '2024-07-28', content: '数据分析思路清晰，帮我们理清了很多数据指标的定义问题。', tags: ['专业', '细心'] }
    ]
  },
  {
    id: 'exp_005', name: '吴云帆', type: 'workflow', level: 'silver',
    avatarEmoji: '⚙️', avatarGradient: 'linear-gradient(135deg, #f59e0b, #fbbf24)',
    description: '工作流编排专家，擅长复杂业务流程自动化设计与优化。',
    skills: ['工作流', 'BPM', '自动化', '流程优化', 'BPMN', '低代码'],
    consultCount: 345, goodRate: 95.2, responseTime: '5.1min', avgRating: 4.6,
    price: 99, online: true, recommended: false, favorited: false, hot: false, isNew: true,
    department: '流程自动化部', phone: '135****3333', email: 'wuyunfan@expert.com',
    joinDate: '2024-05-15', monthGrowth: 68,
    bioParagraphs: [
      '吴云帆，工作流编排专家，专注于企业级业务流程自动化。',
      '在 BPM、低代码平台、流程引擎方面有深入研究，帮助多家企业实现了业务流程的数字化转型。'
    ],
    reviews: [
      { id: 'r7', userName: '周经理', avatar: '周', rating: 4, date: '2024-08-02', content: '流程设计方案很实用，就是细节还需要打磨。', tags: ['实用'] }
    ]
  },
  {
    id: 'exp_006', name: '郑星图', type: 'graph', level: 'diamond',
    avatarEmoji: '🕸️', avatarGradient: 'linear-gradient(135deg, #06b6d4, #22d3ee)',
    description: '知识图谱专家，精通图谱构建、推理与可视化技术。',
    skills: ['知识图谱', '图数据库', 'Neo4j', '图推理', '实体抽取', '关系抽取'],
    consultCount: 823, goodRate: 98.1, responseTime: '3.8min', avgRating: 4.88,
    price: 259, online: true, recommended: true, favorited: false, hot: true, isNew: false,
    department: '知识图谱中心', phone: '134****2222', email: 'zhengxingtu@expert.com',
    joinDate: '2023-04-18', monthGrowth: 31,
    bioParagraphs: [
      '郑星图，知识图谱专家，北京大学计算机博士。',
      '在知识图谱构建、图神经网络、图推理算法方面有深入研究，发表顶级会议论文十余篇。',
      '主导过多个大型企业知识图谱项目，覆盖金融、医疗、教育等多个领域。'
    ],
    reviews: [
      { id: 'r8', userName: '吴技术', avatar: '吴', rating: 5, date: '2024-08-08', content: '图谱设计方案非常专业，从建模到查询优化都讲得很清楚。', tags: ['专业', '深入'] }
    ]
  },
  {
    id: 'exp_007', name: '冯铁山', type: 'security', level: 'gold',
    avatarEmoji: '🔐', avatarGradient: 'linear-gradient(135deg, #ef4444, #f87171)',
    description: '网络安全专家，专注应用安全、数据安全与攻防对抗。',
    skills: ['渗透测试', '代码审计', '安全架构', '数据安全', '等保合规', '应急响应'],
    consultCount: 567, goodRate: 97.3, responseTime: '4.5min', avgRating: 4.75,
    price: 299, online: false, recommended: false, favorited: false, hot: false, isNew: false,
    department: '安全攻防实验室', phone: '133****1111', email: 'fengtieshan@expert.com',
    joinDate: '2023-06-22', monthGrowth: 18,
    bioParagraphs: [
      '冯铁山，网络安全专家，拥有 15 年安全从业经验。',
      '曾任职于知名安全公司，主导过多个大型系统的安全评估与加固工作。',
      '在渗透测试、漏洞挖掘、安全架构设计方面有深厚功底。'
    ],
    reviews: []
  },
  {
    id: 'exp_008', name: '陈御风', type: 'performance', level: 'silver',
    avatarEmoji: '⚡', avatarGradient: 'linear-gradient(135deg, #14b8a6, #2dd4bf)',
    description: '性能优化专家，擅长系统性能调优与瓶颈分析。',
    skills: ['性能调优', 'JVM', '数据库优化', '压测', '缓存设计', '链路追踪'],
    consultCount: 423, goodRate: 96.8, responseTime: '3.2min', avgRating: 4.72,
    price: 179, online: true, recommended: false, favorited: false, hot: false, isNew: false,
    department: '性能优化组', phone: '132****9999', email: 'chenyufeng@expert.com',
    joinDate: '2023-09-30', monthGrowth: 25,
    bioParagraphs: [
      '陈御风，性能优化专家，专注于后端系统性能调优。',
      '精通 JVM 调优、数据库优化、缓存架构设计，曾将多个系统的性能提升数倍。'
    ],
    reviews: []
  },
  {
    id: 'exp_009', name: '卫观澜', type: 'monitor', level: 'bronze',
    avatarEmoji: '📈', avatarGradient: 'linear-gradient(135deg, #f97316, #fb923c)',
    description: '可观测性专家，专注监控告警、链路追踪与日志分析。',
    skills: ['Prometheus', 'Grafana', 'ELK', '链路追踪', 'APM', '告警设计'],
    consultCount: 198, goodRate: 94.5, responseTime: '6.5min', avgRating: 4.5,
    price: 89, online: true, recommended: false, favorited: false, hot: false, isNew: true,
    department: '可观测性团队', phone: '131****8888', email: 'weiguanlan@expert.com',
    joinDate: '2024-06-10', monthGrowth: 72,
    bioParagraphs: [
      '卫观澜，可观测性专家，专注于监控系统建设与运维效率提升。',
      '在 Prometheus、Grafana、ELK 等监控技术栈方面有丰富的实践经验。'
    ],
    reviews: []
  },
  {
    id: 'exp_010', name: '蒋明珠', type: 'market', level: 'gold',
    avatarEmoji: '💼', avatarGradient: 'linear-gradient(135deg, #f43f5e, #fb7185)',
    description: '商业智能专家，擅长数据分析驱动的业务增长策略。',
    skills: ['商业分析', '增长策略', '用户研究', '数据驱动', 'A/B测试', 'BI报表'],
    consultCount: 534, goodRate: 96.2, responseTime: '4.8min', avgRating: 4.68,
    price: 199, online: true, recommended: false, favorited: false, hot: false, isNew: false,
    department: '商业分析部', phone: '130****7777', email: 'jiangmingzhu@expert.com',
    joinDate: '2023-07-12', monthGrowth: 20,
    bioParagraphs: [
      '蒋明珠，商业智能专家，MBA 学位。',
      '擅长将数据分析与商业策略结合，帮助企业实现数据驱动的业务增长。曾服务于多家 500 强企业。'
    ],
    reviews: []
  },
  {
    id: 'exp_011', name: '韩子夜', type: 'mcp', level: 'silver',
    avatarEmoji: '🔗', avatarGradient: 'linear-gradient(135deg, #a855f7, #c084fc)',
    description: 'MCP 协议专家，精通模型上下文协议设计与集成。',
    skills: ['MCP', '协议设计', 'SDK开发', 'API设计', '模型集成', '工具调用'],
    consultCount: 267, goodRate: 95.8, responseTime: '4.0min', avgRating: 4.65,
    price: 149, online: true, recommended: false, favorited: false, hot: false, isNew: true,
    department: '协议标准组', phone: '158****6666', email: 'hanziye@expert.com',
    joinDate: '2024-04-20', monthGrowth: 58,
    bioParagraphs: [
      '韩子夜，MCP 协议专家，专注于模型上下文协议的研究与落地。',
      '深入参与 MCP 协议的设计与推广，在大模型工具调用、外部系统集成方面有丰富经验。'
    ],
    reviews: []
  },
  {
    id: 'exp_012', name: '杨帆', type: 'automation', level: 'gold',
    avatarEmoji: '🤖', avatarGradient: 'linear-gradient(135deg, #0ea5e9, #38bdf8)',
    description: '自动化测试专家，擅长自动化测试框架设计与 CI/CD 流水线建设。',
    skills: ['自动化测试', 'CI/CD', 'Jenkins', 'Selenium', '接口测试', '性能测试'],
    consultCount: 612, goodRate: 97.0, responseTime: '3.6min', avgRating: 4.78,
    price: 189, online: true, recommended: true, favorited: false, hot: false, isNew: false,
    department: '质量保障部', phone: '157****5555', email: 'yangfan@expert.com',
    joinDate: '2023-02-28', monthGrowth: 24,
    bioParagraphs: [
      '杨帆，自动化测试专家，拥有 10 年测试开发经验。',
      '主导过多个大型项目的自动化测试体系建设，在 CI/CD 流水线设计方面有独到见解。'
    ],
    reviews: []
  },
  {
    id: 'exp_013', name: '许文渊', type: 'requirement', level: 'diamond',
    avatarEmoji: '📋', avatarGradient: 'linear-gradient(135deg, #16a34a, #22c55e)',
    description: '需求工程专家，精通需求分析、产品设计与项目管理。',
    skills: ['需求分析', '产品设计', 'PRD撰写', '用户故事', '原型设计', '项目管理'],
    consultCount: 890, goodRate: 98.3, responseTime: '2.9min', avgRating: 4.86,
    price: 249, online: true, recommended: true, favorited: true, hot: true, isNew: false,
    department: '产品研究院', phone: '156****4444', email: 'xuwenyuan@expert.com',
    joinDate: '2023-01-15', monthGrowth: 30,
    bioParagraphs: [
      '许文渊，需求工程专家，资深产品经理。',
      '拥有 15 年产品与需求分析经验，擅长从复杂业务场景中提炼核心需求，输出高质量的产品方案。',
      '曾主导多个从 0 到 1 的产品设计，累计服务客户超过 200 家。'
    ],
    reviews: []
  },
  {
    id: 'exp_014', name: '何星野', type: 'fusion', level: 'master',
    avatarEmoji: '🎯', avatarGradient: 'linear-gradient(135deg, #7c3aed, #a855f7)',
    description: '全维融合专家，擅长跨领域知识融合与复杂系统协同设计。',
    skills: ['系统融合', '跨域协同', '架构设计', '战略规划', '技术选型', '团队管理'],
    consultCount: 456, goodRate: 99.0, responseTime: '5.5min', avgRating: 4.92,
    price: 499, online: false, recommended: true, favorited: false, hot: false, isNew: false,
    department: '融合战略部', phone: '155****3333', email: 'hexingye@expert.com',
    joinDate: '2022-12-01', monthGrowth: 15,
    bioParagraphs: [
      '何星野，全维融合专家，技术战略顾问。',
      '拥有 20 年 IT 行业经验，横跨算法、架构、数据、AI 等多个领域，擅长从全局视角进行系统融合设计。',
      '曾为多家大型企业提供数字化转型战略咨询服务。'
    ],
    reviews: []
  },
  {
    id: 'exp_015', name: '邓子墨', type: 'operator', level: 'silver',
    avatarEmoji: '🧩', avatarGradient: 'linear-gradient(135deg, #8b5cf6, #a78bfa)',
    description: '算子系统专家，专注算子开发、优化与算子库建设。',
    skills: ['算子开发', 'CUDA', '性能优化', '深度学习', '编译器', '异构计算'],
    consultCount: 312, goodRate: 96.0, responseTime: '4.3min', avgRating: 4.68,
    price: 179, online: true, recommended: false, favorited: false, hot: false, isNew: false,
    department: '算子研发部', phone: '154****2222', email: 'dengzimo@expert.com',
    joinDate: '2023-10-08', monthGrowth: 27,
    bioParagraphs: [
      '邓子墨，算子系统专家，专注于深度学习算子的开发与优化。',
      '精通 CUDA 编程、算子融合技术，在 GPU/CPU 异构计算方面有深入研究。'
    ],
    reviews: []
  },
  {
    id: 'exp_016', name: '沈书瑶', type: 'ai', level: 'bronze',
    avatarEmoji: '✨', avatarGradient: 'linear-gradient(135deg, #f472b6, #f9a8d4)',
    description: 'AI 产品经理，专注 AI 产品设计与用户体验优化。',
    skills: ['AI产品', '用户体验', '交互设计', 'A/B测试', '增长黑客', '用户研究'],
    consultCount: 145, goodRate: 93.8, responseTime: '7.2min', avgRating: 4.45,
    price: 69, online: true, recommended: false, favorited: false, hot: false, isNew: true,
    department: 'AI 产品组', phone: '153****1111', email: 'shenshuyao@expert.com',
    joinDate: '2024-07-01', monthGrowth: 85,
    bioParagraphs: [
      '沈书瑶，AI 产品经理，专注于 AI 产品的设计与用户体验优化。',
      '擅长将 AI 技术与用户需求结合，打造有温度的智能产品。新入驻专家，欢迎咨询体验！'
    ],
    reviews: []
  },
  {
    id: 'exp_017', name: '秦风', type: 'architecture', level: 'master',
    avatarEmoji: '🏯', avatarGradient: 'linear-gradient(135deg, #0e7490, #0891b2)',
    description: '首席架构师，企业级分布式系统架构设计权威。',
    skills: ['企业架构', '分布式', '微服务', '云原生', '中台设计', '技术战略'],
    consultCount: 1567, goodRate: 99.5, responseTime: '4.8min', avgRating: 4.98,
    price: 599, online: true, recommended: true, favorited: true, hot: true, isNew: false,
    department: '架构委员会', phone: '152****0000', email: 'qinfeng@expert.com',
    joinDate: '2022-10-01', monthGrowth: 12,
    bioParagraphs: [
      '秦风，首席架构师，架构委员会主席。',
      '20 年软件开发与架构设计经验，曾主导多个亿级用户系统的架构设计。',
      '在企业级架构、分布式系统、云原生技术方面有深厚造诣，是业内公认的架构权威。'
    ],
    reviews: []
  },
  {
    id: 'exp_018', name: '尤雨溪', type: 'algorithm', level: 'gold',
    avatarEmoji: '📐', avatarGradient: 'linear-gradient(135deg, #6366f1, #818cf8)',
    description: '算法竞赛金牌教练，擅长算法面试辅导与竞赛训练。',
    skills: ['算法竞赛', '数据结构', '动态规划', '图论', '数论', '计算几何'],
    consultCount: 789, goodRate: 97.5, responseTime: '2.5min', avgRating: 4.8,
    price: 199, online: false, recommended: false, favorited: false, hot: false, isNew: false,
    department: '算法教学部', phone: '151****9999', email: 'youyuxi@expert.com',
    joinDate: '2023-08-20', monthGrowth: 21,
    bioParagraphs: [
      '尤雨溪，算法竞赛金牌教练。',
      '曾多次获得 ACM-ICPC 区域赛金牌，指导学生获得各类算法竞赛奖项百余项。',
      '擅长将复杂算法问题化繁为简，帮助学员快速提升算法能力。'
    ],
    reviews: []
  },
  {
    id: 'exp_019', name: '卢明月', type: 'data', level: 'diamond',
    avatarEmoji: '🌙', avatarGradient: 'linear-gradient(135deg, #059669, #10b981)',
    description: '数据中台专家，专注数据治理与数据资产化建设。',
    skills: ['数据中台', '数据治理', '数据建模', '数仓设计', '数据资产', '数据质量'],
    consultCount: 645, goodRate: 98.0, responseTime: '4.1min', avgRating: 4.82,
    price: 279, online: true, recommended: true, favorited: false, hot: false, isNew: false,
    department: '数据中台部', phone: '150****8888', email: 'lumingyue@expert.com',
    joinDate: '2023-05-10', monthGrowth: 29,
    bioParagraphs: [
      '卢明月，数据中台专家，资深数据架构师。',
      '在数据治理、数据中台建设、数据资产化方面有丰富的实战经验。',
      '曾帮助多家大型企业完成数据中台从 0 到 1 的建设。'
    ],
    reviews: []
  },
  {
    id: 'exp_020', name: '高凌云', type: 'security', level: 'diamond',
    avatarEmoji: '🛡️', avatarGradient: 'linear-gradient(135deg, #dc2626, #ef4444)',
    description: '首席安全官，企业级安全体系建设与合规专家。',
    skills: ['安全体系', '等保2.0', 'ISO27001', '风险评估', '安全运营', '数据安全'],
    consultCount: 523, goodRate: 98.8, responseTime: '5.0min', avgRating: 4.9,
    price: 399, online: true, recommended: true, favorited: false, hot: false, isNew: false,
    department: '安全委员会', phone: '149****7777', email: 'gaolingyun@expert.com',
    joinDate: '2023-03-01', monthGrowth: 16,
    bioParagraphs: [
      '高凌云，首席安全官，CISSP、CISP 双认证。',
      '20 年信息安全从业经验，在企业级安全体系建设、等保合规、数据安全方面有深厚造诣。',
      '曾主导多家金融机构的安全体系建设与等保测评工作。'
    ],
    reviews: []
  }
]

const experts = ref([])
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
    console.warn('[ExpertPlaza] API 加载失败，使用 Mock 数据:', e.message)
    experts.value = processExperts(mockExperts)
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

function toggleFavorite(expert) {
  if (!expert) return
  expert.favorited = !expert.favorited
  ElMessage.success(expert.favorited ? '已加入收藏' : '已取消收藏')
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
    await new Promise(resolve => setTimeout(resolve, 800))

    const newBooking = {
      id: 'bk_' + Date.now(),
      expertName: bookingExpert.value.name,
      expertType: bookingExpert.value.typeLabel,
      expertEmoji: bookingExpert.value.avatarEmoji,
      expertGradient: bookingExpert.value.avatarGradient,
      topic: bookingForm.topic,
      date: formatDate(bookingForm.date),
      timeSlot: bookingForm.timeSlot,
      description: bookingForm.description,
      status: 'pending'
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

function cancelBooking(id) {
  const booking = myBookings.value.find(b => b.id === id)
  if (booking) {
    booking.status = 'cancelled'
    ElMessage.success('预约已取消')
  }
}

function startConsult(booking) {
  ElMessage.info('正在进入咨询室…')
}

function rebook(booking) {
  const expert = experts.value.find(e => e.name === booking.expertName)
  if (expert) {
    openBooking(expert)
  } else {
    ElMessage.warning('未找到该专家信息')
  }
}

function addToTeam() {
  ElMessage.success('已加入团队协作列表')
}

function startConsultNow(expert) {
  if (!expert?.online) {
    ElMessage.warning('专家当前不在线，请稍后再试或选择预约')
    return
  }
  ElMessage.info('正在连接专家咨询室…')
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
  await loadExperts()
  buildRankingData()
})
</script>

<style scoped>
/* ===== 全局变量 ===== */
.expert-plaza {
  --violet: #7c3aed;
  --cyan: #06b6d4;
  --gradient-primary: linear-gradient(135deg, #7c3aed 0%, #06b6d4 100%);
  --glass-bg: rgba(255, 255, 255, 0.6);
  --glass-border: rgba(255, 255, 255, 0.8);
  --glass-shadow: 0 8px 32px rgba(124, 58, 237, 0.1);
  min-height: 100vh;
  background: var(--bg-deep-sky);
}

html.dark .expert-plaza {
  --glass-bg: rgba(30, 41, 59, 0.6);
  --glass-border: rgba(148, 163, 184, 0.15);
  --glass-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
}

/* ===== 玻璃拟态基础类 ===== */
.glass-card {
  background: var(--glass-bg);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-xl);
  box-shadow: var(--glass-shadow);
}

/* ===== Hero 区 ===== */
.hero-section {
  position: relative;
  overflow: hidden;
  padding: 60px 40px 50px;
  background: linear-gradient(180deg,
    rgba(124, 58, 237, 0.08) 0%,
    rgba(6, 182, 212, 0.04) 50%,
    transparent 100%);
}

.hero-bg {
  position: absolute;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
}

.hero-blob {
  position: absolute;
  border-radius: 50%;
  filter: blur(80px);
  opacity: 0.4;
  animation: float 20s ease-in-out infinite;
}

.blob-1 {
  width: 400px;
  height: 400px;
  background: radial-gradient(circle, rgba(124, 58, 237, 0.3), transparent 70%);
  top: -100px;
  left: -100px;
}

.blob-2 {
  width: 300px;
  height: 300px;
  background: radial-gradient(circle, rgba(6, 182, 212, 0.3), transparent 70%);
  top: 50px;
  right: 10%;
  animation-delay: -7s;
}

.blob-3 {
  width: 350px;
  height: 350px;
  background: radial-gradient(circle, rgba(236, 72, 153, 0.2), transparent 70%);
  bottom: -100px;
  left: 30%;
  animation-delay: -14s;
}

@keyframes float {
  0%, 100% { transform: translate(0, 0) scale(1); }
  33% { transform: translate(30px, -30px) scale(1.05); }
  66% { transform: translate(-20px, 20px) scale(0.95); }
}

.hero-grid-overlay {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(124, 58, 237, 0.03) 1px, transparent 1px),
    linear-gradient(90deg, rgba(124, 58, 237, 0.03) 1px, transparent 1px);
  background-size: 40px 40px;
  mask-image: radial-gradient(ellipse at center, black 30%, transparent 70%);
}

.hero-content {
  position: relative;
  z-index: 1;
  max-width: 1400px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 32px;
}

.hero-text {
  text-align: center;
}

.hero-badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 16px;
  background: rgba(124, 58, 237, 0.1);
  border: 1px solid rgba(124, 58, 237, 0.2);
  border-radius: 999px;
  font-size: 13px;
  font-weight: 600;
  color: var(--violet);
  margin-bottom: 16px;
}

.hero-badge-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--violet);
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.hero-title {
  font-size: 48px;
  font-weight: 800;
  margin: 0 0 12px;
  line-height: 1.2;
}

.gradient-text {
  background: linear-gradient(135deg, var(--violet) 0%, var(--cyan) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.hero-subtitle {
  font-size: 16px;
  color: var(--text-tertiary);
  margin: 0;
  max-width: 600px;
}

/* 搜索框 */
.hero-search {
  width: 100%;
  max-width: 700px;
}

.search-box {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 8px 8px 20px;
  transition: all 0.3s var(--ease);
}

.search-box:hover {
  box-shadow: 0 12px 40px rgba(124, 58, 237, 0.15);
  transform: translateY(-1px);
}

.search-icon {
  font-size: 20px;
  color: var(--text-quaternary);
  flex-shrink: 0;
}

.search-input {
  flex: 1;
  border: none;
  outline: none;
  background: transparent;
  font-size: 15px;
  color: var(--text-primary);
  padding: 12px 0;
}

.search-input::placeholder {
  color: var(--text-quaternary);
}

.search-btn {
  height: 44px;
  padding: 0 28px;
  border-radius: var(--radius-lg) !important;
  background: var(--gradient-primary) !important;
  border: none !important;
  font-weight: 600;
  flex-shrink: 0;
}

/* 快捷筛选 */
.quick-filters {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  justify-content: center;
}

.quick-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 18px;
  background: var(--glass-bg);
  backdrop-filter: blur(10px);
  border: 1px solid var(--glass-border);
  border-radius: 999px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.25s var(--ease);
}

.quick-tab:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(124, 58, 237, 0.12);
}

.quick-tab.active {
  background: var(--gradient-primary);
  color: white;
  border-color: transparent;
  box-shadow: 0 6px 20px rgba(124, 58, 237, 0.3);
}

.quick-tab-icon {
  font-size: 14px;
}

/* 统计卡片 */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 20px;
  width: 100%;
  max-width: 1000px;
}

.stat-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px 24px;
  transition: all 0.3s var(--ease);
  animation: fadeInUp 0.6s var(--ease) backwards;
  animation-delay: var(--delay);
}

@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 16px 48px rgba(124, 58, 237, 0.15);
}

.stat-icon {
  width: 52px;
  height: 52px;
  border-radius: 14px;
  display: grid;
  place-items: center;
  color: white;
  font-size: 24px;
  flex-shrink: 0;
}

.stat-info {
  flex: 1;
  min-width: 0;
}

.stat-value {
  font-size: 24px;
  font-weight: 800;
  color: var(--text-primary);
  line-height: 1.2;
}

.stat-label {
  font-size: 13px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

.stat-trend {
  font-size: 12px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}

.stat-trend.up {
  color: var(--success);
}

/* ===== 主内容区 ===== */
.main-section {
  padding: 0 40px 60px;
  max-width: 1400px;
  margin: 0 auto;
}

.plaza-layout {
  display: grid;
  grid-template-columns: 260px 1fr;
  gap: 24px;
  align-items: start;
}

/* 左侧筛选边栏 */
.filter-sidebar {
  position: sticky;
  top: 20px;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.sidebar-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.sidebar-title .el-icon {
  color: var(--violet);
}

.filter-group {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.filter-group-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 6px;
}

.filter-group-title .el-icon {
  color: var(--cyan);
  font-size: 14px;
}

.filter-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.filter-tag {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 12px;
  background: var(--bg-surface-2);
  border: 1px solid transparent;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all 0.2s var(--ease);
}

.filter-tag:hover {
  background: var(--bg-input-focus);
  color: var(--text-secondary);
}

.filter-tag.active {
  background: rgba(124, 58, 237, 0.1);
  border-color: rgba(124, 58, 237, 0.3);
  color: var(--violet);
  font-weight: 600;
}

.tag-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.level-tag.level-bronze.active {
  background: rgba(205, 127, 50, 0.1);
  border-color: rgba(205, 127, 50, 0.3);
  color: #cd7f32;
}

.level-tag.level-silver.active {
  background: rgba(192, 192, 192, 0.15);
  border-color: rgba(150, 150, 150, 0.3);
  color: #808080;
}

.level-tag.level-gold.active {
  background: rgba(255, 215, 0, 0.15);
  border-color: rgba(255, 215, 0, 0.4);
  color: #b8860b;
}

.level-tag.level-diamond.active {
  background: rgba(6, 182, 212, 0.1);
  border-color: rgba(6, 182, 212, 0.3);
  color: var(--cyan);
}

.level-tag.level-master.active {
  background: rgba(124, 58, 237, 0.1);
  border-color: rgba(124, 58, 237, 0.3);
  color: var(--violet);
}

.price-options {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.price-option {
  padding: 8px 12px;
  border-radius: var(--radius-md);
  font-size: 13px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s var(--ease);
}

.price-option:hover {
  background: var(--bg-surface-2);
}

.price-option.active {
  background: linear-gradient(135deg, rgba(124, 58, 237, 0.1), rgba(6, 182, 212, 0.08));
  color: var(--violet);
  font-weight: 600;
}

.sort-radio {
  width: 100%;
}

:deep(.sort-radio .el-radio-button__inner) {
  width: 100%;
  font-size: 12px;
  padding: 8px 10px;
}

/* 排行榜 & 预约入口 */
.ranking-entry,
.booking-entry {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px;
  border-radius: var(--radius-lg);
  background: linear-gradient(135deg, rgba(245, 158, 11, 0.08), rgba(239, 68, 68, 0.06));
  border: 1px solid rgba(245, 158, 11, 0.15);
  cursor: pointer;
  transition: all 0.3s var(--ease);
}

.booking-entry {
  background: linear-gradient(135deg, rgba(124, 58, 237, 0.08), rgba(6, 182, 212, 0.06));
  border: 1px solid rgba(124, 58, 237, 0.15);
}

.ranking-entry:hover,
.booking-entry:hover {
  transform: translateX(4px);
  box-shadow: 0 8px 24px rgba(124, 58, 237, 0.12);
}

.ranking-entry-icon,
.booking-entry-icon {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-size: 20px;
  flex-shrink: 0;
}

.ranking-entry-icon {
  background: linear-gradient(135deg, #f59e0b, #ef4444);
  color: white;
}

.booking-entry-icon {
  background: linear-gradient(135deg, var(--violet), var(--cyan));
  color: white;
}

.ranking-entry-text,
.booking-entry-text {
  flex: 1;
  min-width: 0;
}

.ranking-entry-title,
.booking-entry-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.ranking-entry-desc,
.booking-entry-desc {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

.ranking-entry-arrow,
.booking-entry-arrow {
  color: var(--text-quaternary);
  flex-shrink: 0;
  transition: transform 0.2s var(--ease);
}

.ranking-entry:hover .ranking-entry-arrow,
.booking-entry:hover .booking-entry-arrow {
  transform: translateX(4px);
  color: var(--violet);
}

/* ===== 内容区 ===== */
.content-area {
  min-width: 0;
}

.content-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 20px;
  background: var(--glass-bg);
  backdrop-filter: blur(10px);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-xl);
  padding: 6px;
  width: fit-content;
}

.content-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 10px 20px;
  border-radius: var(--radius-lg);
  font-size: 14px;
  font-weight: 500;
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all 0.25s var(--ease);
}

.content-tab:hover {
  color: var(--text-secondary);
}

.content-tab.active {
  background: var(--gradient-primary);
  color: white;
  font-weight: 600;
  box-shadow: 0 4px 16px rgba(124, 58, 237, 0.3);
}

.tab-count {
  background: rgba(255, 255, 255, 0.2);
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
}

/* 专家网格 */
.experts-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 20px;
}

.expert-card {
  display: flex;
  flex-direction: column;
  padding: 20px;
  cursor: pointer;
  transition: all 0.35s cubic-bezier(0.22, 1, 0.36, 1);
  animation: fadeInUp 0.5s var(--ease) backwards;
  animation-delay: var(--delay);
  position: relative;
  overflow: hidden;
}

.expert-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--gradient-primary);
  opacity: 0;
  transition: opacity 0.3s var(--ease);
}

.expert-card:hover {
  transform: translateY(-6px);
  box-shadow: 0 20px 60px rgba(124, 58, 237, 0.18);
}

.expert-card:hover::before {
  opacity: 1;
}

.card-top {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 14px;
}

.expert-avatar-lg {
  width: 60px;
  height: 60px;
  border-radius: 18px;
  display: grid;
  place-items: center;
  position: relative;
  transition: transform 0.3s var(--ease);
}

.expert-card:hover .expert-avatar-lg {
  transform: scale(1.05) rotate(-3deg);
}

.avatar-emoji {
  font-size: 28px;
}

.online-dot {
  position: absolute;
  bottom: 2px;
  right: 2px;
  width: 12px;
  height: 12px;
  background: #10b981;
  border: 2px solid white;
  border-radius: 50%;
  animation: pulse-green 2s ease-in-out infinite;
}

html.dark .online-dot {
  border-color: var(--bg-surface);
}

@keyframes pulse-green {
  0%, 100% { box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.4); }
  50% { box-shadow: 0 0 0 6px rgba(16, 185, 129, 0); }
}

.favorite-btn {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  border: none;
  background: rgba(255, 255, 255, 0.8);
  backdrop-filter: blur(10px);
  color: var(--text-quaternary);
  cursor: pointer;
  display: grid;
  place-items: center;
  transition: all 0.25s var(--ease);
}

html.dark .favorite-btn {
  background: rgba(30, 41, 59, 0.8);
}

.favorite-btn:hover {
  color: #f59e0b;
  transform: scale(1.1);
}

.favorite-btn.favorited {
  color: #f59e0b;
}

/* 专家信息 */
.expert-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.expert-name-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.expert-name {
  font-size: 17px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.level-badge {
  font-size: 10px;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: 6px;
  flex-shrink: 0;
}

.level-badge.level-bronze {
  background: linear-gradient(135deg, #d4a574, #cd7f32);
  color: white;
}

.level-badge.level-silver {
  background: linear-gradient(135deg, #d1d5db, #9ca3af);
  color: white;
}

.level-badge.level-gold {
  background: linear-gradient(135deg, #fcd34d, #f59e0b);
  color: white;
}

.level-badge.level-diamond {
  background: linear-gradient(135deg, #22d3ee, #06b6d4);
  color: white;
}

.level-badge.level-master {
  background: linear-gradient(135deg, #a855f7, #7c3aed);
  color: white;
}

.expert-type-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.type-tag {
  font-size: 11px;
  font-weight: 600;
  padding: 3px 10px;
  border-radius: 999px;
}

.recommend-tag {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 10px;
  font-weight: 600;
  padding: 2px 6px;
  background: linear-gradient(135deg, #fef3c7, #fde68a);
  color: #92400e;
  border-radius: 6px;
}

.expert-desc {
  font-size: 13px;
  color: var(--text-tertiary);
  line-height: 1.6;
  margin: 4px 0 6px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.skill-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.skill-tag {
  font-size: 11px;
  padding: 3px 10px;
  background: var(--bg-surface-2);
  color: var(--text-secondary);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s var(--ease);
}

.skill-tag:hover {
  background: rgba(124, 58, 237, 0.1);
  color: var(--violet);
}

.skill-more {
  font-size: 11px;
  padding: 3px 8px;
  color: var(--text-quaternary);
}

/* 卡片底部 */
.card-stats {
  display: flex;
  justify-content: space-around;
  padding: 12px 0;
  margin: 12px 0;
  border-top: 1px solid var(--border-ghost);
  border-bottom: 1px solid var(--border-ghost);
}

.card-stat {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--text-tertiary);
}

.card-stat .el-icon {
  font-size: 14px;
  color: var(--violet);
}

.card-stat span {
  font-weight: 600;
  color: var(--text-secondary);
  font-size: 12px;
}

.card-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.price-info {
  display: flex;
  align-items: baseline;
  gap: 2px;
}

.price-value {
  font-size: 18px;
  font-weight: 800;
  background: var(--gradient-primary);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.price-unit {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-quaternary);
  -webkit-text-fill-color: var(--text-quaternary);
}

.consult-btn {
  border-radius: var(--radius-lg) !important;
  padding: 8px 20px !important;
  font-weight: 600 !important;
  background: var(--gradient-primary) !important;
  border: none !important;
  transition: all 0.3s var(--ease) !important;
}

.consult-btn:hover {
  transform: scale(1.05);
  box-shadow: 0 6px 20px rgba(124, 58, 237, 0.35) !important;
}

/* 加载更多 */
.load-more {
  text-align: center;
  margin-top: 32px;
}

/* 加载状态 */
.loading-state,
.empty-state {
  grid-column: 1 / -1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  text-align: center;
}

.loading-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid var(--border-soft);
  border-top-color: var(--violet);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: 16px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 16px;
}

.empty-state h4 {
  font-size: 16px;
  color: var(--text-primary);
  margin: 0 0 8px;
}

.empty-state p {
  font-size: 13px;
  color: var(--text-tertiary);
  margin: 0 0 20px;
}

/* ===== 预约列表 ===== */
.bookings-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.booking-card {
  padding: 20px;
  transition: all 0.3s var(--ease);
}

.booking-card:hover {
  transform: translateX(4px);
  box-shadow: 0 12px 40px rgba(124, 58, 237, 0.12);
}

.booking-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 14px;
}

.booking-expert {
  display: flex;
  align-items: center;
  gap: 12px;
}

.booking-avatar {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-size: 22px;
}

.booking-expert-name {
  font-size: 15px;
  font-weight: 700;
  color: var(--text-primary);
}

.booking-expert-type {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

.booking-body {
  padding: 14px 0;
  border-top: 1px solid var(--border-ghost);
  border-bottom: 1px solid var(--border-ghost);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.booking-topic {
  font-size: 14px;
  color: var(--text-primary);
  font-weight: 500;
}

.booking-label {
  color: var(--text-tertiary);
  font-weight: 400;
}

.booking-time {
  font-size: 13px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 6px;
}

.booking-time .el-icon {
  color: var(--violet);
}

.booking-desc {
  font-size: 12px;
  color: var(--text-tertiary);
  line-height: 1.6;
}

.booking-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 14px;
}

/* ===== 详情弹窗 ===== */
.detail-dialog :deep(.el-dialog) {
  border-radius: 24px !important;
  overflow: hidden;
  background: var(--glass-bg) !important;
  backdrop-filter: blur(30px);
}

.detail-header {
  display: flex;
  align-items: center;
  gap: 20px;
}

.detail-avatar-wrap {
  flex-shrink: 0;
}

.detail-avatar {
  width: 80px;
  height: 80px;
  border-radius: 24px;
  display: grid;
  place-items: center;
  position: relative;
}

.detail-avatar-emoji {
  font-size: 40px;
}

.detail-online-dot {
  position: absolute;
  bottom: 4px;
  right: 4px;
  width: 16px;
  height: 16px;
  background: #10b981;
  border: 3px solid white;
  border-radius: 50%;
}

html.dark .detail-online-dot {
  border-color: var(--bg-surface);
}

.detail-title-area {
  flex: 1;
  min-width: 0;
}

.detail-name-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  margin-bottom: 6px;
}

.detail-name {
  font-size: 24px;
  font-weight: 800;
  color: var(--text-primary);
  margin: 0;
}

.detail-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text-tertiary);
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #94a3b8;
}

.status-dot.online {
  background: #10b981;
}

.status-sep {
  color: var(--border-soft);
}

.detail-actions-head {
  display: flex;
  gap: 8px;
}

.icon-btn {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  border: 1px solid var(--border-soft);
  background: var(--bg-surface);
  color: var(--text-tertiary);
  cursor: pointer;
  display: grid;
  place-items: center;
  transition: all 0.2s var(--ease);
}

.icon-btn:hover {
  color: #f59e0b;
  border-color: #f59e0b;
}

/* 详情内容 */
.detail-body {
  display: flex;
  flex-direction: column;
  gap: 24px;
  max-height: 60vh;
  overflow-y: auto;
  padding-right: 8px;
}

.detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

.info-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.info-section-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.info-section-title .el-icon {
  color: var(--violet);
}

.review-count {
  font-size: 12px;
  color: var(--text-tertiary);
  font-weight: 400;
}

.info-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.info-item {
  display: flex;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--bg-surface-2);
  border-radius: var(--radius-md);
  font-size: 13px;
}

.info-label {
  color: var(--text-tertiary);
}

.info-value {
  color: var(--text-primary);
  font-weight: 500;
}

.stats-mini-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
}

.stat-mini {
  padding: 14px;
  background: var(--bg-surface-2);
  border-radius: var(--radius-lg);
  text-align: center;
}

.stat-mini-value {
  font-size: 20px;
  font-weight: 800;
  color: var(--text-primary);
}

.stat-mini-value.accent {
  color: var(--success);
}

.stat-mini-value.violet {
  color: var(--violet);
}

.stat-mini-label {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 4px;
}

/* 技能云 */
.skill-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 16px;
  background: var(--bg-surface-2);
  border-radius: var(--radius-lg);
}

.cloud-tag {
  font-size: var(--size, 13px);
  padding: 6px 14px;
  background: linear-gradient(135deg, rgba(124, 58, 237, 0.1), rgba(6, 182, 212, 0.08));
  color: var(--violet);
  border-radius: 999px;
  font-weight: 500;
  transition: all 0.25s var(--ease);
  cursor: default;
}

.cloud-tag:hover {
  transform: scale(1.05);
  background: linear-gradient(135deg, var(--violet), var(--cyan));
  color: white;
}

/* 专家介绍 */
.expert-bio {
  padding: 16px 20px;
  background: var(--bg-surface-2);
  border-radius: var(--radius-lg);
  line-height: 1.8;
  font-size: 13px;
  color: var(--text-secondary);
}

.expert-bio p {
  margin: 0 0 10px;
}

.expert-bio p:last-child {
  margin-bottom: 0;
}

.section-head-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

/* 评价列表 */
.reviews-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.review-item {
  padding: 16px;
  background: var(--bg-surface-2);
  border-radius: var(--radius-lg);
  transition: all 0.2s var(--ease);
}

.review-item:hover {
  background: var(--bg-input-focus);
}

.review-head {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 10px;
}

.review-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: var(--gradient-primary);
  color: white;
  display: grid;
  place-items: center;
  font-size: 14px;
  font-weight: 600;
  flex-shrink: 0;
}

.review-meta {
  flex: 1;
  min-width: 0;
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
  color: var(--text-quaternary);
}

.review-content {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.7;
}

.review-tags {
  display: flex;
  gap: 6px;
  margin-top: 10px;
  flex-wrap: wrap;
}

.review-tag {
  font-size: 11px;
  padding: 2px 8px;
  background: rgba(16, 185, 129, 0.1);
  color: var(--success);
  border-radius: 4px;
}

/* 详情底部 */
.detail-footer-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.footer-price {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.price-label {
  font-size: 12px;
  color: var(--text-tertiary);
}

.price-amount {
  font-size: 24px;
  font-weight: 800;
  background: var(--gradient-primary);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.footer-btns {
  display: flex;
  gap: 10px;
}

/* ===== 预约对话框 ===== */
.booking-dialog :deep(.el-dialog) {
  border-radius: 20px !important;
}

.booking-expert-info {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 16px;
  margin-bottom: 20px;
}

.be-avatar {
  width: 52px;
  height: 52px;
  border-radius: 14px;
  display: grid;
  place-items: center;
  font-size: 26px;
  flex-shrink: 0;
}

.be-info {
  flex: 1;
  min-width: 0;
}

.be-name {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
}

.be-type {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

.be-price {
  font-size: 20px;
  font-weight: 800;
  background: var(--gradient-primary);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  flex-shrink: 0;
}

.time-slots {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
}

.time-slot {
  padding: 10px 8px;
  text-align: center;
  background: var(--bg-surface-2);
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s var(--ease);
}

.time-slot:hover {
  border-color: var(--violet);
  color: var(--violet);
}

.time-slot.active {
  background: linear-gradient(135deg, rgba(124, 58, 237, 0.1), rgba(6, 182, 212, 0.08));
  border-color: var(--violet);
  color: var(--violet);
  font-weight: 600;
}

.time-slot.disabled {
  opacity: 0.4;
  cursor: not-allowed;
  text-decoration: line-through;
}

/* ===== 排行榜弹窗 ===== */
.ranking-dialog :deep(.el-dialog) {
  border-radius: 20px !important;
}

.ranking-tabs :deep(.el-tabs__header) {
  margin-bottom: 16px;
}

.ranking-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 500px;
  overflow-y: auto;
}

.ranking-item {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 16px;
  border-radius: var(--radius-lg);
  cursor: pointer;
  transition: all 0.25s var(--ease);
}

.ranking-item:hover {
  background: var(--bg-surface-2);
  transform: translateX(4px);
}

.rank-num {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  font-size: 13px;
  font-weight: 800;
  background: var(--bg-surface-2);
  color: var(--text-tertiary);
  flex-shrink: 0;
}

.rank-num.rank-1 {
  background: linear-gradient(135deg, #fcd34d, #f59e0b);
  color: white;
  box-shadow: 0 4px 12px rgba(245, 158, 11, 0.3);
}

.rank-num.rank-2 {
  background: linear-gradient(135deg, #d1d5db, #9ca3af);
  color: white;
}

.rank-num.rank-3 {
  background: linear-gradient(135deg, #d4a574, #cd7f32);
  color: white;
}

.rank-avatar {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-size: 22px;
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
  font-size: 12px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

.rank-stat {
  text-align: right;
  flex-shrink: 0;
}

.rank-stat-value {
  font-size: 16px;
  font-weight: 800;
  color: var(--text-primary);
}

.rank-stat-value.good {
  color: var(--success);
}

.rank-stat-value.violet {
  color: var(--violet);
}

.rank-stat-label {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

/* ===== 响应式 ===== */
@media (max-width: 1200px) {
  .plaza-layout {
    grid-template-columns: 220px 1fr;
    gap: 16px;
  }

  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
  }

  .hero-title {
    font-size: 36px;
  }
}

@media (max-width: 900px) {
  .hero-section {
    padding: 40px 20px 30px;
  }

  .main-section {
    padding: 0 20px 40px;
  }

  .plaza-layout {
    grid-template-columns: 1fr;
  }

  .filter-sidebar {
    position: static;
  }

  .detail-grid {
    grid-template-columns: 1fr;
  }

  .time-slots {
    grid-template-columns: repeat(3, 1fr);
  }
}

@media (max-width: 600px) {
  .hero-title {
    font-size: 28px;
  }

  .hero-subtitle {
    font-size: 14px;
  }

  .stats-grid {
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .stat-card {
    padding: 14px 16px;
  }

  .stat-icon {
    width: 40px;
    height: 40px;
    font-size: 18px;
  }

  .stat-value {
    font-size: 18px;
  }

  .experts-grid {
    grid-template-columns: 1fr;
  }

  .search-box {
    padding: 6px 6px 6px 14px;
  }

  .search-btn {
    padding: 0 18px;
    height: 38px;
  }

  .detail-footer-actions {
    flex-direction: column;
    gap: 12px;
    align-items: stretch;
  }

  .footer-btns {
    justify-content: center;
    flex-wrap: wrap;
  }

  .time-slots {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
