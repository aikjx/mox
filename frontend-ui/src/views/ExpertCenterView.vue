<template>
  <div class="expert-center">
    <!-- 页面头部：项目关联 + 全局操作 -->
    <div class="head">
      <div class="head-left">
        <div class="head-brand">
          <div class="brand-mark">
            <svg viewBox="0 0 32 32" class="bm-svg">
              <defs>
                <linearGradient id="bmG" x1="0" y1="0" x2="1" y2="1">
                  <stop offset="0%" stop-color="#6366f1" />
                  <stop offset="55%" stop-color="#0ea5e9" />
                  <stop offset="100%" stop-color="#10b981" />
                </linearGradient>
              </defs>
              <circle cx="16" cy="16" r="14" fill="none" stroke="url(#bmG)" stroke-width="2.4" />
              <circle cx="16" cy="16" r="7" fill="url(#bmG)" opacity="0.92" />
              <path d="M6 22 L13 10 L19 22 Z" fill="#ffffff" opacity="0.92" />
            </svg>
          </div>
          <div class="head-titles">
            <div class="head-title-row">
              <h2 class="page-title">璇玑·专家联盟 X</h2>
              <el-tag class="ver-tag" effect="dark" round>Mox 3.0 · MIT</el-tag>
              <el-tag class="ver-tag-alt" effect="plain" round>以项目为根 · 全维 φ 流程</el-tag>
            </div>
            <p class="page-subtitle">
              S1 需求架构 · S2 知识图谱 · S3 方案设计 · S4 开发执行 · S5 发布监控
              <span v-if="currentProject">
                · 当前跟进：<b class="hl-project">{{ currentProject.name }}</b>
                <span v-if="currentProject.category" class="proj-cat">{{ currentProject.category }}</span>
              </span>
              <span v-else class="muted-plain">· 请在顶栏选择项目，或在此快速<u class="link-like" @click="ensureProject">创建一个</u></span>
            </p>
          </div>
        </div>
      </div>
      <div class="head-actions">
        <el-button @click="loadOverview" :loading="overviewLoading" size="default">
          <el-icon><DataAnalysis /></el-icon> 联盟概览
        </el-button>
        <el-button type="primary" plain @click="showRegister = true" size="default">
          <el-icon><Plus /></el-icon> 注册专家
        </el-button>
        <el-button type="success" plain @click="ensureProject" size="default">
          <el-icon><Promotion /></el-icon> 创建项目
        </el-button>
        <el-button @click="loadAll" size="default"><el-icon><Refresh /></el-icon> 刷新</el-button>
      </div>
    </div>

    <!-- 联盟 φ 三栏布局（左·中·右）· 黄金比例 0.382 : 0.618 ，中再分 0.618 vs 右 0.382 -->
    <div class="phi-shell">

      <!-- ============ 左栏：5 阶段导航 + 专家库 ============ -->
      <aside class="col col-left">
        <!-- S1~S5 阶段导航 · 可点击跳阶段 -->
        <section class="card card-tight phase-nav">
          <div class="card-head">
            <span class="card-title">项目全维流程</span>
            <span class="card-sub">5 段式 φ 生命周期</span>
          </div>
          <div class="phase-list">
            <div
              v-for="(p, idx) in PHASES"
              :key="p.key"
              class="phase-row"
              :class="{ active: localPhase === p.key, done: phaseDone[p.key] }"
              @click="selectPhase(p.key)"
            >
              <div class="phase-idx" :style="{ background: p.color }">{{ idx + 1 }}</div>
              <div class="phase-body">
                <div class="phase-name">{{ p.label }}</div>
                <div class="phase-desc">{{ p.desc }}</div>
              </div>
              <div v-if="phaseProgress[p.key]" class="phase-bar">
                <div class="phase-bar-fill" :style="{ width: phaseProgress[p.key] + '%', background: p.color }"></div>
              </div>
              <div class="phase-chev">›</div>
            </div>
          </div>
        </section>

        <!-- 专家库 · 筛选 + 选择 -->
        <section class="card experts-card">
          <div class="card-head between">
            <div class="card-title-wrap">
              <span class="card-title">算法 / 架构 · 专家联盟</span>
              <span class="count-pill">{{ filteredExperts.length }}/{{ experts.length }}</span>
            </div>
            <el-switch v-model="smartMode" active-text="智能匹配" inactive-text="手动" size="small" />
          </div>
          <div class="filter-bar">
            <el-select v-model="filterType" placeholder="专家类型" clearable size="small">
              <el-option v-for="t in expertTypes" :key="t" :label="typeLabel(t)" :value="t" />
            </el-select>
            <el-input v-model="keyword" placeholder="搜索姓名、能力、描述..." clearable size="small" style="flex:1">
              <template #prefix><el-icon><Search /></el-icon></template>
            </el-input>
          </div>
          <el-scrollbar class="exp-scroll">
            <div
              v-for="exp in filteredExperts"
              :key="exp.id"
              class="expert-card"
              :class="{ sel: isSelected(exp.id), offline: exp.status !== 'active' }"
              @click="toggleSelect(exp)"
            >
              <div class="expert-avatar" :style="{ background: getColor(exp.type) }">
                <el-icon><component :is="getIcon(exp.type)" /></el-icon>
              </div>
              <div class="expert-info">
                <div class="expert-name-row">
                  <span class="expert-name">{{ exp.name }}</span>
                  <span v-if="exp.metrics?.total_consults" class="consult-count" title="累计咨询次数">
                    {{ exp.metrics.total_consults }}
                  </span>
                </div>
                <div class="expert-type">{{ typeLabel(exp.type) }} · {{ exp.status === 'active' ? '在线' : '离线' }}</div>
                <div class="expert-stats" v-if="exp.metrics">
                  <span class="stat-item" title="成功率">
                    <el-icon><TrendCharts /></el-icon>
                    {{ (exp.metrics.success_rate * 100).toFixed(0) }}%
                  </span>
                  <span class="stat-item" title="平均耗时">
                    <el-icon><Timer /></el-icon>
                    {{ Math.round(exp.metrics.avg_duration || 0) }}ms
                  </span>
                </div>
                <div class="expert-caps">
                  <el-tag
                    v-for="cap in (exp.capabilities || []).slice(0, 3)"
                    :key="cap"
                    size="small"
                    type="info"
                    effect="plain"
                  >{{ cap }}</el-tag>
                </div>
              </div>
              <div v-if="isSelected(exp.id)" class="expert-check">✓</div>
            </div>
            <el-empty v-if="!filteredExperts.length" description="暂无匹配专家" :image-size="60" />
          </el-scrollbar>

          <!-- 已选专家摘要 -->
          <div v-if="selectedCount()" class="sel-summary">
            <div class="sel-head">
              <span class="sel-title">已选 · {{ selectedCount() }} 位</span>
              <el-button size="small" text type="primary" @click="selectedExpertIds = []">清空</el-button>
            </div>
            <div class="sel-chips">
              <span
                v-for="id in selectedExpertIds"
                :key="id"
                class="chip-sel"
              >
                <span class="chip-dot" :style="{ background: getColor(experts.find(e=>e.id===id)?.type || 'custom') }" />
                {{ getExpertName(id) }}
                <el-icon class="chip-x" @click.stop="removeExpert(id)"><Close /></el-icon>
              </span>
            </div>
          </div>
        </section>
      </aside>

      <!-- ============ 中栏：AI 助手 X · 全维 φ 对话 + 流程模式 ============ -->
      <section class="col col-mid">
        <!-- 工作栏头 -->
        <div class="card card-tight mid-head">
          <div class="mh-left">
            <div class="mh-logo">
              <svg viewBox="0 0 24 24" class="ai-logo-svg">
                <defs>
                  <linearGradient id="aiG" x1="0" y1="0" x2="1" y2="1">
                    <stop offset="0%" stop-color="#6366f1" />
                    <stop offset="100%" stop-color="#0ea5e9" />
                  </linearGradient>
                </defs>
                <circle cx="12" cy="12" r="10.5" fill="none" stroke="url(#aiG)" stroke-width="1.5" />
                <path d="M12 4 C15 4 17.5 6 18 9.5 C20 10 20 14 18 15 C17.5 18 15 19.5 12 20 C9 19.5 6.5 18 6 15 C4 14 4 10 6 9.5 C6.5 6 9 4 12 4 Z" fill="url(#aiG)" opacity="0.92" />
                <circle cx="9.4" cy="11.5" r="1.4" fill="#fff" />
                <circle cx="14.6" cy="11.5" r="1.4" fill="#fff" />
                <path d="M9 15.3 C10 16.2 11.2 16.7 12 16.7 C12.8 16.7 14 16.2 15 15.3" fill="none" stroke="#fff" stroke-width="1.5" stroke-linecap="round" />
              </svg>
            </div>
            <div class="mh-text">
              <div class="mh-title">AI 助手 X <span class="badge-ver">V3.0 · MIT</span></div>
              <div class="mh-sub">
                已选择：<span class="chip-selected">{{ selectedChip }}</span>
                · <span class="muted-plain">输入自定义问题，系统将全维分析处理</span>
              </div>
            </div>
          </div>
          <div class="mh-right">
            <!-- 模式开关 -->
            <el-switch
              v-model="requirementFlowMode"
              inline-prompt
              active-text="流程模式"
              inactive-text="对话模式"
              size="small"
            />
            <el-button size="small" @click="clearCurrentConversation">
              <el-icon><Refresh /></el-icon> 清空
            </el-button>
            <el-button size="small" type="primary" plain @click="openNewChatView">
              <el-icon><ChatDotRound /></el-icon> +新建对话
            </el-button>
          </div>
        </div>

        <!-- 快捷问法 Chips · 生成公司官网需求图谱 · 自定义问题... -->
        <div class="card quick-q-card">
          <div class="qq-title-row">
            <span class="qq-label">🔥 快捷启动</span>
            <el-button size="small" text @click="showMoreQuick = !showMoreQuick">{{ showMoreQuick ? '收起' : '展开' }}</el-button>
          </div>
          <div class="qq-grid">
            <button
              v-for="(q, i) in visibleQuickQuestions"
              :key="i"
              class="qq-btn"
              :class="{ active: selectedChip === q.label }"
              @click="pickQuick(q)"
            >
              <span class="qq-emoji">{{ q.icon }}</span>
              <div class="qq-text">
                <div class="qq-name">{{ q.label }}</div>
                <div class="qq-desc">{{ q.hint }}</div>
              </div>
              <span class="qq-go">➜</span>
            </button>
          </div>
        </div>

        <!-- 咨询工作台（中栏主内容） -->
        <div class="card panel-col">
          <div class="consult-header">
            <div class="ch-left">
              <h3 class="section-title">专家咨询 · {{ modeLabelMap[mode] }}</h3>
              <span v-if="currentProject" class="proj-link">
                项目 <b>{{ currentProject.name }}</b> · 全维跟进
              </span>
            </div>
            <div class="mode-switch">
              <el-radio-group v-model="mode" size="small">
                <el-radio-button value="smart">🧠 智能路由</el-radio-button>
                <el-radio-button value="single">单专家</el-radio-button>
                <el-radio-button value="multi">多专家</el-radio-button>
                <el-radio-button value="debate">🗣 辩论</el-radio-button>
                <el-radio-button value="algorithm">⚙ 算法分析</el-radio-button>
              </el-radio-group>
            </div>
          </div>

          <!-- 输入区 · 流程模式头指示 -->
          <div v-if="requirementFlowMode" class="flow-head-bar">
            <div class="fh-label">
              <span class="fh-idx">{{ currentStage + 1 }}</span>
              <span class="fh-text">{{ FLOW_STAGES[currentStage].label }}</span>
            </div>
            <div class="fh-desc">{{ FLOW_STAGES[currentStage].hint }}</div>
            <div class="fh-actions">
              <el-button size="small" v-if="currentStage > 0" @click="currentStage--">← 上一阶段</el-button>
              <el-button size="small" type="primary" v-if="currentStage < 5" @click="advanceFlowStage">下一阶段 →</el-button>
              <el-button size="small" type="primary" plain @click="runFullFlow" :loading="consulting">🚀 启动全维流程</el-button>
            </div>
          </div>

          <!-- 模式说明 + 输入 -->
          <div v-if="mode === 'smart'" class="mode-block smart-mode">
            <div class="mode-desc">
              <el-icon><MagicStick /></el-icon>
              <span>系统自动分析问题意图，智能匹配合适的专家，并选择最优协作模式（单专家/多专家/辩论）</span>
            </div>
            <el-input
              v-model="question"
              type="textarea"
              :rows="3"
              class="big-input"
              :placeholder="customPlaceholder"
            />
            <div v-if="routingResult" class="routing-info">
              <div class="routing-title">智能路由结果</div>
              <div class="routing-detail">
                <el-tag :type="routingResult.intent?.primary ? 'primary' : 'info'" effect="dark">
                  意图：{{ routingResult.intent?.primary || '通用' }}
                </el-tag>
                <span class="muted">置信度 {{ (routingResult.intent?.confidence * 100 || 0).toFixed(0) }}%</span>
                <div class="routing-experts">
                  <span
                    v-for="s in routingResult.selected?.slice(0, 3)"
                    :key="s.expert.id"
                    class="chip"
                    :title="`匹配分: ${s.score.toFixed(1)}`"
                  >
                    {{ s.expert.name }}<span class="score">({{ s.score.toFixed(1) }})</span>
                  </span>
                </div>
              </div>
            </div>
            <div class="action-row">
              <el-button
                type="primary"
                class="act-cta"
                :loading="consulting"
                :disabled="!question.trim()"
                @click="doSmartRoute"
              >
                <el-icon><Promotion /></el-icon> 启动全维流程
              </el-button>
              <el-button :disabled="!question.trim()" :loading="routingLoading" @click="doRouteOnly">
                <el-icon><Guide /></el-icon> 仅路由分析
              </el-button>
              <el-button :disabled="!question.trim()" @click="askQuickAnalysis">
                ⚡ 快速分析
              </el-button>
              <el-button :disabled="!question.trim()" @click="addFollowUp">
                + 添加
              </el-button>
            </div>
          </div>

          <div v-else-if="mode === 'single'" class="mode-block single-consult">
            <div class="selected-area">
              <span class="muted">已选择：</span>
              <template v-if="selectedCount()">
                <span class="chip" v-for="id in selectedExpertIds" :key="id">
                  {{ getExpertName(id) }}
                  <el-icon class="chip-x" @click="removeExpert(id)"><Close /></el-icon>
                </span>
              </template>
              <span v-else class="muted">请从左侧选择一位专家</span>
            </div>
            <el-input v-model="question" type="textarea" :rows="3" class="big-input" placeholder="请输入你的问题..." />
            <div class="action-row">
              <el-button type="primary" class="act-cta" :loading="consulting" :disabled="!selectedCount() || !question.trim()" @click="doConsult">
                <el-icon><Promotion /></el-icon> 咨询专家
              </el-button>
              <el-button :disabled="!question.trim()" @click="convertQuestionToTask">📌 转任务</el-button>
              <el-button :disabled="!question.trim()" @click="createProjectFromQuestion">🏗 创建项目</el-button>
            </div>
          </div>

          <div v-else-if="mode === 'multi'" class="mode-block multi-consult">
            <div class="selected-area">
              <span class="muted">已选 {{ selectedCount() }} 位：</span>
              <span class="chip" v-for="id in selectedExpertIds" :key="id">{{ getExpertName(id) }}</span>
            </div>
            <el-input v-model="question" type="textarea" :rows="3" class="big-input" placeholder="多位专家协同分析的问题..." />
            <div class="action-row">
              <el-button type="primary" class="act-cta" :loading="consulting" :disabled="selectedCount() < 2 || !question.trim()" @click="doMultiConsult">
                <el-icon><ChatDotRound /></el-icon> 协同分析
              </el-button>
              <el-button :disabled="!question.trim()" @click="addFollowUp">+ 添加</el-button>
            </div>
          </div>

          <div v-else-if="mode === 'debate'" class="mode-block debate-mode">
            <div class="selected-area">
              <span class="muted">已选 {{ selectedCount() }} 位参与辩论：</span>
              <span class="chip" v-for="id in selectedExpertIds" :key="id">{{ getExpertName(id) }}</span>
            </div>
            <div class="debate-config">
              <el-input-number v-model="rounds" :min="2" :max="5" label="辩论轮数" />
              <el-select v-model="debateStrategy" style="width: 160px">
                <el-option value="round_robin" label="轮流发言" />
                <el-option value="cross_examine" label="交叉质询" />
              </el-select>
            </div>
            <el-input v-model="question" type="textarea" :rows="3" class="big-input" placeholder="请输入辩论主题..." />
            <div class="action-row">
              <el-button type="primary" class="act-cta" :loading="consulting" :disabled="selectedCount() < 2 || !question.trim()" @click="doDebate">
                <el-icon><MagicStick /></el-icon> 开始辩论
              </el-button>
            </div>
          </div>

          <div v-else-if="mode === 'algorithm'" class="mode-block algorithm-mode">
            <div class="mode-desc">
              <el-icon><Cpu /></el-icon>
              <span>算法联盟调度算法 & 图谱专家，自动进行复杂度分析 / 算法推荐 / 数据结构选型</span>
            </div>
            <el-input v-model="question" type="textarea" :rows="3" class="big-input" placeholder="算法问题：图的最短路径、复杂度、排序优化、推荐算法选型…" />
            <div class="graph-data-area">
              <el-checkbox v-model="useGraphData">使用图谱数据分析</el-checkbox>
              <el-input
                v-if="useGraphData"
                v-model="graphDataJson"
                type="textarea"
                :rows="3"
                placeholder='{"nodes":[{"id":"n1"}],"edges":[{"source":"n1","target":"n2"}]}'
              />
            </div>
            <div class="action-row">
              <el-button type="primary" class="act-cta" :loading="consulting" :disabled="!question.trim()" @click="doAlgorithmAnalysis">
                <el-icon><DataAnalysis /></el-icon> 算法分 · 开始分析
              </el-button>
              <el-button :disabled="!question.trim()" @click="importGraphData">📥 导入</el-button>
            </div>
          </div>

          <!-- 结果区：咨询/路由/算法/辩论 -->
          <div v-if="results.length || algorithmResult || debateSummary" class="results-block">
            <div class="rb-head">
              <h4 class="results-title">🏛 联盟输出</h4>
              <div class="rb-tools">
                <el-button size="small" text @click="exportConversation">📤 导出</el-button>
                <el-button size="small" type="primary" text @click="openNewChatView(question)">💬 发送给 AI 助手 X</el-button>
              </div>
            </div>
            <el-scrollbar class="results-scroll">
              <div v-for="(r, i) in results" :key="i" class="result-item">
                <div class="result-head">
                  <span class="expert-badge" :style="{ background: getColorByType(r.expert?.type) }">
                    {{ r.expert?.name || '专家' }}
                  </span>
                  <div class="result-meta">
                    <span v-if="r.confidence" class="confidence">置信度 {{ (r.confidence * 100).toFixed(0) }}%</span>
                    <span v-if="r.duration_ms" class="duration">{{ r.duration_ms }}ms</span>
                    <el-tag v-if="r.round" size="small" type="warning">第{{ r.round }}轮</el-tag>
                  </div>
                </div>
                <div class="result-content">{{ r.response }}</div>
                <div class="result-ops">
                  <el-button size="small" text @click="copyText(r.response)">📋 复制</el-button>
                  <el-button size="small" text type="primary" @click="appendResultAsInput(r)">➕ 追加为问题</el-button>
                </div>
              </div>

              <div v-if="algorithmResult" class="algorithm-result">
                <div v-if="algorithmResult.analysis?.graph" class="algo-section">
                  <h5>📈 图谱分析</h5>
                  <div class="graph-stats">
                    <span class="stat-chip">节点 {{ algorithmResult.analysis.graph.stats?.nodeCount || '-' }}</span>
                    <span class="stat-chip">边 {{ algorithmResult.analysis.graph.stats?.edgeCount || '-' }}</span>
                    <span class="stat-chip">密度 {{ algorithmResult.analysis.graph.stats?.density || '-' }}</span>
                    <span class="stat-chip">平均度 {{ algorithmResult.analysis.graph.stats?.avgDegree || '-' }}</span>
                  </div>
                  <div v-if="algorithmResult.analysis.graph.topNodes?.length" class="top-nodes">
                    <div class="top-nodes-title">Top 节点（PageRank）</div>
                    <div class="node-list">
                      <span v-for="n in algorithmResult.analysis.graph.topNodes.slice(0, 5)" :key="n.id" class="node-chip">
                        #{{ n.rank }} {{ n.id }} ({{ n.pagerank }})
                      </span>
                    </div>
                  </div>
                </div>
                <div v-if="algorithmResult.analysis?.algorithm" class="algo-section">
                  <h5>🧩 算法建议</h5>
                  <div v-for="(a, i) in algorithmResult.analysis.algorithm.analyses" :key="i" class="algo-item">
                    <div class="algo-name">{{ a.algorithm }}</div>
                    <div class="algo-rec">{{ a.recommendation }}</div>
                    <div class="algo-complexity">时间 {{ a.complexity.time }} · 空间 {{ a.complexity.space }}</div>
                  </div>
                </div>
                <div v-if="algorithmResult.analysis?.ai_insight" class="ai-insight">
                  <h5><el-icon><MagicStick /></el-icon> AI 深度洞察</h5>
                  <div class="insight-content">{{ algorithmResult.analysis.ai_insight }}</div>
                </div>
              </div>

              <div v-if="debateSummary" class="debate-summary">
                <h4 class="results-title">🗂 辩论综合结论</h4>
                <div class="debate-final">{{ debateSummary }}</div>
              </div>
            </el-scrollbar>
          </div>

          <div v-else class="conv-empty">
            <div class="empty-orb">
              <svg viewBox="0 0 40 40" class="eo-svg">
                <defs>
                  <linearGradient id="eoG" x1="0" y1="0" x2="1" y2="1">
                    <stop offset="0%" stop-color="#6366f1" />
                    <stop offset="60%" stop-color="#0ea5e9" />
                    <stop offset="100%" stop-color="#10b981" />
                  </linearGradient>
                </defs>
                <circle cx="20" cy="20" r="17" fill="none" stroke="url(#eoG)" stroke-width="1.6" stroke-dasharray="5 3" />
                <circle cx="20" cy="20" r="9" fill="url(#eoG)" opacity="0.9" />
              </svg>
            </div>
            <div class="empty-title">在上方输入你的 <b>自定义问题</b>，系统将全维分析处理...</div>
            <div class="empty-sub">或从左侧选择专家，并在顶栏 <b>选择项目</b> 以开启全流程跟进（需求·图谱·设计·开发·发布）</div>
          </div>
        </div>
      </section>

      <!-- ============ 右栏：璇玑 Mox Graph System · 需求/架构/算法图谱 + 项目进度 ============ -->
      <section class="col col-right">
        <!-- 璇玑系统 头部 -->
        <div class="card card-tight right-head">
          <div class="rh-left">
            <div class="rh-logo">
              <svg viewBox="0 0 24 24" class="rh-svg">
                <defs>
                  <linearGradient id="rhG" x1="0" y1="0" x2="1" y2="1">
                    <stop offset="0%" stop-color="#10b981" />
                    <stop offset="100%" stop-color="#6366f1" />
                  </linearGradient>
                </defs>
                <circle cx="12" cy="12" r="10.5" fill="none" stroke="url(#rhG)" stroke-width="1.5" />
                <circle cx="12" cy="12" r="3.4" fill="url(#rhG)" />
                <circle cx="12" cy="3.5" r="1.6" fill="url(#rhG)" />
                <circle cx="20" cy="8" r="1.6" fill="url(#rhG)" />
                <circle cx="20" cy="16" r="1.6" fill="url(#rhG)" />
                <circle cx="12" cy="20.5" r="1.6" fill="url(#rhG)" />
                <circle cx="4" cy="16" r="1.6" fill="url(#rhG)" />
                <circle cx="4" cy="8" r="1.6" fill="url(#rhG)" />
                <g stroke="url(#rhG)" stroke-width="1" fill="none" opacity="0.7">
                  <line x1="12" y1="12" x2="12" y2="5" />
                  <line x1="12" y1="12" x2="18.5" y2="8" />
                  <line x1="12" y1="12" x2="18.5" y2="16" />
                  <line x1="12" y1="12" x2="12" y2="19" />
                  <line x1="12" y1="12" x2="5.5" y2="16" />
                  <line x1="12" y1="12" x2="5.5" y2="8" />
                </g>
              </svg>
            </div>
            <div class="rh-text">
              <div class="rh-title">璇玑系统 · Mox Graph</div>
              <div class="rh-sub">
                {{ currentProject ? '项目知识图谱' : '示例·公司官网需求图谱' }}
              </div>
            </div>
          </div>
          <div class="rh-right">
            <el-button size="small" plain @click="randomizeGraph">
              <el-icon><Refresh /></el-icon> 刷新图谱
            </el-button>
            <el-button size="small" type="primary" plain @click="goToGraphPage">
              <el-icon><Promotion /></el-icon> 查看大图
            </el-button>
          </div>
        </div>

        <!-- 图谱可视化 · Canvas 渲染 -->
        <div class="card mox-canvas-card">
          <div class="mc-legend">
            <span class="lg lg-project">项目</span>
            <span class="lg lg-goal">目标</span>
            <span class="lg lg-actor">角色</span>
            <span class="lg lg-usecase">用例</span>
            <span class="lg lg-data">数据</span>
            <span class="lg lg-tech">技术</span>
            <span class="lg lg-end">验收</span>
          </div>
          <div class="mc-stage" ref="graphStageRef">
            <canvas ref="graphCanvasRef"></canvas>
          </div>
          <div class="mc-stats">
            <span class="stat-chip">节点 {{ graphStats.nodes }}</span>
            <span class="stat-chip">关系 {{ graphStats.edges }}</span>
            <span class="stat-chip">密度 {{ graphStats.density }}</span>
          </div>
        </div>

        <!-- 项目进度 & 联盟指标 -->
        <div class="card progress-card">
          <div class="card-head between">
            <span class="card-title">项目推进 · 联盟绩效</span>
            <el-tag size="small" effect="plain">以当前项目为准</el-tag>
          </div>

          <!-- 项目总体进度环 -->
          <div class="progress-top">
            <el-progress
              type="dashboard"
              :percentage="projectOverall"
              :width="120"
              color="#6366f1"
            />
            <div class="pt-info">
              <div class="pt-name">{{ currentProject ? currentProject.name : '示例项目 · 公司官网' }}</div>
              <div class="pt-cat">{{ currentProject?.category || '官网 / 营销' }} · {{ currentProject?.status || '规划中' }}</div>
              <div class="pt-rows">
                <div class="pt-row">
                  <span class="pt-k">阶段完成</span>
                  <span class="pt-v">{{ phaseCompleteCount }}/5</span>
                </div>
                <div class="pt-row">
                  <span class="pt-k">专家咨询</span>
                  <span class="pt-v">{{ overview?.total_consults || 0 }} 次</span>
                </div>
                <div class="pt-row">
                  <span class="pt-k">成功率</span>
                  <span class="pt-v" :style="{ color: getSuccessColor(overview?.avg_success_rate) }">
                    {{ ((overview?.avg_success_rate || 0) * 100).toFixed(0) }}%
                  </span>
                </div>
              </div>
            </div>
          </div>

          <!-- 5 阶段进度 -->
          <div class="pgs">
            <div v-for="p in PHASES" :key="p.key" class="pg-row">
              <div class="pg-label"><span class="pg-dot" :style="{ background: p.color }"></span>{{ p.label }}</div>
              <el-progress :percentage="phaseProgress[p.key] || 0" :stroke-width="8" :color="p.color" />
            </div>
          </div>

          <!-- 概览小卡 -->
          <div v-if="overview" class="ov-grid">
            <div class="ov-cell">
              <div class="ov-v">{{ overview.total_experts }}</div>
              <div class="ov-k">专家总数</div>
            </div>
            <div class="ov-cell active">
              <div class="ov-v">{{ overview.active_experts }}</div>
              <div class="ov-k">在线专家</div>
            </div>
            <div class="ov-cell">
              <div class="ov-v">{{ overview.expert_types?.length || 0 }}</div>
              <div class="ov-k">专家类型</div>
            </div>
            <div class="ov-cell success">
              <div class="ov-v">{{ overview.capabilities_count || 0 }}</div>
              <div class="ov-k">能力标签</div>
            </div>
          </div>
        </div>

        <!-- 专家绩效 · 前 6 -->
        <div class="card metrics-card">
          <div class="card-head between">
            <span class="card-title">专家绩效 Top</span>
            <el-button size="small" text type="primary" @click="showMetricsFull = !showMetricsFull">
              {{ showMetricsFull ? '收起' : '展开全部' }}
            </el-button>
          </div>
          <el-table
            :data="showMetricsFull ? metricsList : metricsList.slice(0, 5)"
            stripe
            size="small"
            class="mini-table"
            style="width: 100%"
          >
            <el-table-column label="专家" width="98">
              <template #default="{ row }">
                <span :style="{ color: getColorByType(row.expert?.type), fontWeight: 600, fontSize: 12 }">
                  {{ row.expert?.name || '-' }}
                </span>
              </template>
            </el-table-column>
            <el-table-column label="成功" width="70">
              <template #default="{ row }">
                <el-progress
                  :percentage="Math.round((row.metrics?.success_rate || 0) * 100)"
                  :stroke-width="6"
                  :color="getSuccessColor(row.metrics?.success_rate)"
                />
              </template>
            </el-table-column>
            <el-table-column prop="metrics.avg_duration" label="ms" width="52" sortable>
              <template #default="{ row }">{{ Math.round(row.metrics?.avg_duration || 0) }}</template>
            </el-table-column>
          </el-table>
        </div>
      </section>
    </div>

    <!-- 注册弹窗 -->
    <el-dialog v-model="showRegister" title="注册新专家" width="520px">
      <el-form label-width="90px">
        <el-form-item label="专家名称">
          <el-input v-model="newExpert.name" placeholder="如：数据库专家" />
        </el-form-item>
        <el-form-item label="专家类型">
          <el-select v-model="newExpert.type" style="width: 100%">
            <el-option v-for="t in expertTypes" :key="t" :label="typeLabel(t)" :value="t" />
          </el-select>
        </el-form-item>
        <el-form-item label="能力标签">
          <el-input v-model="newExpert.capabilities_str" placeholder="用逗号分隔，如：性能优化,索引调优" />
        </el-form-item>
        <el-form-item label="专家描述">
          <el-input v-model="newExpert.description" type="textarea" :rows="2" />
        </el-form-item>
        <el-form-item label="系统提示词">
          <el-input
            v-model="newExpert.systemPrompt"
            type="textarea"
            :rows="3"
            placeholder="专家专属的 System Prompt，用于定义专家角色和行为"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showRegister = false">取消</el-button>
        <el-button type="primary" :loading="registering" @click="doRegister">注册</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  Plus, Refresh, Close, Promotion, ChatDotRound, MagicStick,
  DataAnalysis, Timer, TrendCharts, Cpu, Guide, Search
} from '@element-plus/icons-vue'
import {
  getExperts, registerExpert, consultExpert, multiExpertConsult, expertDebate,
  routeExperts, intelligentConsult, algorithmAnalysis,
  getExpertMetrics, getExpertOverview
} from '@/api'
import { useProject } from '@/composables/projectContext.js'

const router = useRouter()
// 项目上下文（来自顶栏 ProjectPicker，共享状态）
const { currentProject, ensureProjectContext, createAndSelect } = useProject()

const experts = ref([])
const filterType = ref('')
const keyword = ref('')
const selectedExpertIds = ref([])
const mode = ref('smart')
const question = ref('')
const consulting = ref(false)
const routingLoading = ref(false)
const results = ref([])
const debateSummary = ref('')
const algorithmResult = ref(null)
const rounds = ref(2)
const debateStrategy = ref('round_robin')
const smartMode = ref(true)

const showRegister = ref(false)
const registering = ref(false)
const newExpert = ref({ name: '', type: 'algorithm', capabilities_str: '', description: '', systemPrompt: '' })

const overview = ref(null)
const overviewLoading = ref(false)
const metricsList = ref([])
const routingResult = ref(null)
const useGraphData = ref(false)
const graphDataJson = ref('')

const typeLabels = {
  algorithm: '算法专家', architecture: '架构专家', data: '数据专家',
  ai: 'AI专家', workflow: '工作流专家', operator: '算子系统专家',
  graph: '知识图谱专家', security: '安全专家', performance: '性能优化专家',
  monitor: '可观测性专家', market: '商业智能专家', mcp: 'MCP协议专家',
  automation: '自动化专家', requirement: '需求工程专家', fusion: '融合专家',
  custom: '自定义专家'
}

const expertTypes = computed(() => Object.keys(typeLabels))

function typeLabel(t) { return typeLabels[t] || t }

function getColor(type) {
  const colors = {
    algorithm: '#6366f1', architecture: '#0891b2', data: '#10b981',
    ai: '#ec4899', workflow: '#f59e0b', operator: '#8b5cf6',
    graph: '#06b6d4', security: '#ef4444', performance: '#14b8a6',
    monitor: '#f97316', market: '#f43f5e', mcp: '#a855f7',
    automation: '#0ea5e9', requirement: '#16a34a', fusion: '#7c3aed',
    custom: '#64748b'
  }
  return colors[type] || colors.custom
}

function getColorByType(type) { return getColor(type) }

function getIcon(type) {
  const icons = {
    algorithm: 'TrendCharts', architecture: 'Grid', data: 'Coin',
    ai: 'MagicStick', workflow: 'Operation', operator: 'Cpu',
    graph: 'Share', security: 'Lock', performance: 'Lightning',
    monitor: 'DataLine', market: 'Shop', mcp: 'Link',
    automation: 'MagicStick', requirement: 'Tickets', fusion: 'Aim',
    custom: 'User'
  }
  return icons[type] || icons.custom
}

function getSuccessColor(rate) {
  if (!rate) return '#ef4444'
  if (rate >= 0.95) return '#10b981'
  if (rate >= 0.85) return '#3b82f6'
  if (rate >= 0.7) return '#f59e0b'
  return '#ef4444'
}

const filteredExperts = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  return experts.value.filter(e => {
    const matchType = !filterType.value || e.type === filterType.value
    const matchKw = !kw ||
      e.name.toLowerCase().includes(kw) ||
      (e.description || '').toLowerCase().includes(kw) ||
      (e.capabilities || []).some(c => c.toLowerCase().includes(kw))
    return matchType && matchKw
  })
})

function getExpertName(id) {
  return experts.value.find(e => e.id === id)?.name || id
}

function toggleSelect(exp) {
  if (exp.status !== 'active') {
    ElMessage.warning('该专家当前不在线')
    return
  }
  const idx = selectedExpertIds.value.indexOf(exp.id)
  if (idx !== -1) {
    selectedExpertIds.value.splice(idx, 1)
  } else {
    if (mode.value === 'single' && selectedExpertIds.value.length >= 1) {
      selectedExpertIds.value = [exp.id]
    } else {
      selectedExpertIds.value.push(exp.id)
    }
  }
}

function isSelected(id) { return selectedExpertIds.value.includes(id) }
function selectedCount() { return selectedExpertIds.value.length }
function removeExpert(id) {
  const idx = selectedExpertIds.value.indexOf(id)
  if (idx !== -1) selectedExpertIds.value.splice(idx, 1)
}

async function loadExperts() {
  try {
    experts.value = await getExperts()
  } catch (e) {
    ElMessage.error('加载专家列表失败：' + e.message)
  }
}

async function loadOverview() {
  overviewLoading.value = true
  try {
    overview.value = await getExpertOverview()
  } catch (e) {
    ElMessage.error('加载系统概览失败：' + e.message)
  } finally {
    overviewLoading.value = false
  }
}

async function loadMetrics() {
  try {
    const data = await getExpertMetrics()
    metricsList.value = data.metrics || []
  } catch (e) {
    console.error('加载指标失败：', e.message)
  }
}

async function loadAll() {
  await Promise.all([loadExperts(), loadOverview(), loadMetrics()])
  ElMessage.success('数据已刷新')
}

async function doRouteOnly() {
  if (!question.value.trim()) return
  routingLoading.value = true
  try {
    routingResult.value = await routeExperts({
      question: question.value,
      maxExperts: 3
    })
  } catch (e) {
    ElMessage.error('路由分析失败：' + e.message)
  } finally {
    routingLoading.value = false
  }
}

async function doSmartRoute() {
  if (!question.value.trim()) return
  consulting.value = true
  results.value = []
  debateSummary.value = ''
  algorithmResult.value = null

  try {
    const result = await intelligentConsult({
      question: question.value,
      mode: 'auto'
    })

    routingResult.value = result.routing

    if (result.mode === 'single') {
      results.value = [{
        expert: result.expert,
        response: result.response,
        confidence: result.metadata?.confidence,
        duration_ms: result.metadata?.duration_ms
      }]
    } else if (result.mode === 'multi') {
      results.value = result.results.filter(r => r.success).map(r => ({
        expert: r.expert,
        response: r.response,
        confidence: r.confidence,
        duration_ms: r.duration_ms
      }))
    } else if (result.mode === 'debate') {
      results.value = []
      result.history.forEach((round, idx) => {
        round.results.forEach(r => {
          if (r.success) {
            results.value.push({
              expert: r.expert,
              response: r.response,
              round: idx + 1,
              confidence: r.confidence,
              duration_ms: r.duration_ms
            })
          }
        })
      })
      debateSummary.value = result.final_synthesis
    }

    ElMessage.success(`智能路由完成，模式: ${result.mode}`)
    await loadMetrics()
  } catch (e) {
    ElMessage.error('智能咨询失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doConsult() {
  const expertId = selectedExpertIds.value[0]
  if (!expertId || !question.value.trim()) return

  consulting.value = true
  results.value = []
  debateSummary.value = ''
  algorithmResult.value = null

  try {
    const result = await consultExpert(expertId, {
      messages: [{ role: 'user', content: question.value }]
    })
    results.value = [{
      expert: { id: expertId, name: getExpertName(expertId) },
      response: result.response,
      confidence: result.metadata?.confidence,
      duration_ms: result.metadata?.duration_ms
    }]
    ElMessage.success('咨询完成')
  } catch (e) {
    ElMessage.error('咨询失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doMultiConsult() {
  const expertIds = [...selectedExpertIds.value]
  if (expertIds.length < 2 || !question.value.trim()) return

  consulting.value = true
  results.value = []
  debateSummary.value = ''
  algorithmResult.value = null

  try {
    const result = await multiExpertConsult({
      question: question.value,
      expert_ids: expertIds
    })
    results.value = result.results.filter(r => r.success).map(r => ({
      expert: r.expert,
      response: r.response,
      confidence: r.confidence,
      duration_ms: r.duration_ms
    }))
    ElMessage.success(`协同分析完成，共 ${result.successful} 位专家参与`)
  } catch (e) {
    ElMessage.error('协同分析失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doDebate() {
  const expertIds = [...selectedExpertIds.value]
  if (expertIds.length < 2 || !question.value.trim()) return

  consulting.value = true
  results.value = []
  debateSummary.value = ''
  algorithmResult.value = null

  try {
    const result = await expertDebate({
      question: question.value,
      expert_ids: expertIds,
      rounds: rounds.value
    })
    results.value = []
    result.history.forEach((round, idx) => {
      round.results.forEach(r => {
        if (r.success) {
          results.value.push({
            expert: r.expert,
            response: r.response,
            round: idx + 1,
            confidence: r.confidence
          })
        }
      })
    })
    debateSummary.value = result.final_synthesis
    ElMessage.success(`辩论完成，共 ${rounds.value} 轮`)
  } catch (e) {
    ElMessage.error('辩论失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doAlgorithmAnalysis() {
  if (!question.value.trim()) return

  consulting.value = true
  results.value = []
  debateSummary.value = ''
  algorithmResult.value = null

  try {
    let graphData = null
    if (useGraphData.value && graphDataJson.value.trim()) {
      try {
        graphData = JSON.parse(graphDataJson.value)
      } catch (e) {
        ElMessage.error('图谱数据 JSON 格式错误')
        consulting.value = false
        return
      }
    }

    const result = await algorithmAnalysis({
      question: question.value,
      graphData,
      options: {}
    })
    algorithmResult.value = result
    ElMessage.success('算法分析完成')
  } catch (e) {
    ElMessage.error('算法分析失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doRegister() {
  if (!newExpert.value.name || !newExpert.value.type) {
    ElMessage.warning('请填写专家名称和类型')
    return
  }

  registering.value = true
  try {
    await registerExpert({
      name: newExpert.value.name,
      type: newExpert.value.type,
      capabilities: (newExpert.value.capabilities_str || '').split(',').map(s => s.trim()).filter(Boolean),
      description: newExpert.value.description,
      systemPrompt: newExpert.value.systemPrompt
    })
    ElMessage.success('注册成功')
    showRegister.value = false
    newExpert.value = { name: '', type: 'algorithm', capabilities_str: '', description: '', systemPrompt: '' }
    await loadExperts()
  } catch (e) {
    ElMessage.error('注册失败：' + e.message)
  } finally {
    registering.value = false
  }
}

watch(mode, () => {
  if (mode.value !== 'single') {
    selectedExpertIds.value = []
  }
})

// ==================== 璇玑 Mox Graph · 简易 Canvas 力导向渲染 ====================
const graphCanvasRef = ref(null)
const graphStageRef = ref(null)
const graphData = reactive({ nodes: [], edges: [] })
const graphStats = computed(() => {
  const n = graphData.nodes.length
  const e = graphData.edges.length
  const density = n > 1 ? ((2 * e) / (n * (n - 1))).toFixed(2) : '0.00'
  return { nodes: n, edges: e, density }
})
let rafId = 0

// 公司官网需求图谱 · 预设数据（用于无项目场景的展示）
function buildMockGraph() {
  const pj = currentProject.value ? currentProject.value.name : '公司官网'
  const NODES = [
    { id: 'P1', type: 'project', label: pj, fixed: true },
    { id: 'G1', type: 'goal', label: '品牌展示' },
    { id: 'G2', type: 'goal', label: '线索转化' },
    { id: 'G3', type: 'goal', label: 'SEO 排名' },
    { id: 'A1', type: 'actor', label: '访客' },
    { id: 'A2', type: 'actor', label: '运营' },
    { id: 'A3', type: 'actor', label: '管理员' },
    { id: 'U1', type: 'usecase', label: '首页浏览' },
    { id: 'U2', type: 'usecase', label: '产品介绍' },
    { id: 'U3', type: 'usecase', label: '表单留资' },
    { id: 'U4', type: 'usecase', label: '新闻/博客' },
    { id: 'U5', type: 'usecase', label: '后台管理' },
    { id: 'D1', type: 'data', label: '用户线索' },
    { id: 'D2', type: 'data', label: '内容数据' },
    { id: 'D3', type: 'data', label: '产品数据' },
    { id: 'T1', type: 'tech', label: 'Vue 3 + Vite' },
    { id: 'T2', type: 'tech', label: 'Element Plus' },
    { id: 'T3', type: 'tech', label: 'NestJS + Postgres' },
    { id: 'T4', type: 'tech', label: 'SEO SSR' },
    { id: 'E1', type: 'end', label: '性能验收' },
    { id: 'E2', type: 'end', label: '上线 Checklist' }
  ]
  const EDGES = [
    ['P1', 'G1', 'contains', '包含'],
    ['P1', 'G2', 'contains', '包含'],
    ['P1', 'G3', 'contains', '包含'],
    ['P1', 'A1', 'serves', '服务于'],
    ['P1', 'A2', 'serves', '服务于'],
    ['P1', 'A3', 'serves', '服务于'],
    ['G1', 'U1', 'realizedBy', '通过'],
    ['G1', 'U2', 'realizedBy', '通过'],
    ['G2', 'U3', 'realizedBy', '通过'],
    ['G3', 'U4', 'realizedBy', '通过'],
    ['A3', 'U5', 'perform', '执行'],
    ['U1', 'T1', 'implement', '实现'],
    ['U1', 'T2', 'implement', '实现'],
    ['U3', 'D1', 'produce', '产生'],
    ['U4', 'D2', 'produce', '产生'],
    ['U2', 'D3', 'read', '读取'],
    ['D1', 'T3', 'persist', '持久化'],
    ['D2', 'T3', 'persist', '持久化'],
    ['D3', 'T3', 'persist', '持久化'],
    ['U4', 'T4', 'optimize', 'SEO优化'],
    ['E1', 'G1', 'verify', '验证'],
    ['E2', 'P1', 'gate', '门禁']
  ]
  return {
    nodes: NODES.map((n, i) => ({ ...n, x: 0, y: 0, vx: 0, vy: 0, index: i })),
    edges: EDGES.map((e) => ({ source: e[0], target: e[1], type: e[2], label: e[3] }))
  }
}

function layoutSeed(nodes) {
  const w = 360
  const h = 320
  const cx = w / 2
  const cy = h / 2
  // 中心项目，其余按环形分散
  nodes.forEach((n, i) => {
    if (n.fixed) {
      n.x = cx; n.y = cy
    } else {
      const angle = (i / Math.max(1, nodes.length - 1)) * Math.PI * 2 + 0.2
      const r = 110 + (i % 3) * 18
      n.x = cx + Math.cos(angle) * r
      n.y = cy + Math.sin(angle) * r
    }
    n.vx = 0; n.vy = 0
  })
}

function simulateOnce(nodes, edges, w, h) {
  // 简易物理
  const cx = w / 2
  const cy = h / 2
  // 中心弱吸附
  nodes.forEach((n) => {
    if (n.fixed) { n.vx *= 0.5; n.vy *= 0.5; return }
    const dx = cx - n.x
    const dy = cy - n.y
    n.vx += dx * 0.0004
    n.vy += dy * 0.0004
  })
  // 斥力
  for (let i = 0; i < nodes.length; i++) {
    const a = nodes[i]
    for (let j = i + 1; j < nodes.length; j++) {
      const b = nodes[j]
      let dx = a.x - b.x
      let dy = a.y - b.y
      let d2 = dx * dx + dy * dy + 0.001
      const force = 1800 / d2
      const d = Math.sqrt(d2)
      dx /= d; dy /= d
      if (!a.fixed) { a.vx += dx * force; a.vy += dy * force }
      if (!b.fixed) { b.vx -= dx * force; b.vy -= dy * force }
    }
  }
  // 弹簧
  edges.forEach((e) => {
    const s = nodes.find((n) => n.id === e.source)
    const t = nodes.find((n) => n.id === e.target)
    if (!s || !t) return
    let dx = t.x - s.x
    let dy = t.y - s.y
    let d = Math.sqrt(dx * dx + dy * dy) + 0.001
    const rest = 78
    const diff = (d - rest) / d
    const f = diff * 0.015
    dx *= f; dy *= f
    if (!s.fixed) { s.vx += dx; s.vy += dy }
    if (!t.fixed) { t.vx -= dx; t.vy -= dy }
  })
  // 阻尼 & 位置
  nodes.forEach((n) => {
    n.vx *= 0.82
    n.vy *= 0.82
    n.x += n.vx
    n.y += n.vy
    // 边界
    n.x = Math.max(18, Math.min(w - 18, n.x))
    n.y = Math.max(18, Math.min(h - 18, n.y))
  })
}

const TYPE_STYLE = {
  project:  { color: '#6366f1', r: 16, label: '项目' },
  goal:     { color: '#0ea5e9', r: 12, label: '目标' },
  actor:    { color: '#f59e0b', r: 12, label: '角色' },
  usecase:  { color: '#10b981', r: 11, label: '用例' },
  data:     { color: '#ef4444', r: 11, label: '数据' },
  tech:     { color: '#8b5cf6', r: 11, label: '技术' },
  end:      { color: '#64748b', r: 12, label: '验收' }
}

function drawGraph() {
  const canvas = graphCanvasRef.value
  const stage = graphStageRef.value
  if (!canvas || !stage) return
  const W = stage.clientWidth
  const H = stage.clientHeight || 320
  const dpr = window.devicePixelRatio || 1
  canvas.width = W * dpr
  canvas.height = H * dpr
  canvas.style.width = W + 'px'
  canvas.style.height = H + 'px'
  const ctx = canvas.getContext('2d')
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)

  function tick() {
    simulateOnce(graphData.nodes, graphData.edges, W, H)
    ctx.clearRect(0, 0, W, H)
    // 边
    graphData.edges.forEach((e) => {
      const s = graphData.nodes.find((n) => n.id === e.source)
      const t = graphData.nodes.find((n) => n.id === e.target)
      if (!s || !t) return
      ctx.beginPath()
      ctx.strokeStyle = 'rgba(100,116,139,0.34)'
      ctx.lineWidth = 1
      ctx.moveTo(s.x, s.y)
      ctx.lineTo(t.x, t.y)
      ctx.stroke()
    })
    // 节点
    graphData.nodes.forEach((n) => {
      const ts = TYPE_STYLE[n.type] || TYPE_STYLE.goal
      ctx.beginPath()
      ctx.fillStyle = ts.color
      ctx.arc(n.x, n.y, ts.r, 0, Math.PI * 2)
      ctx.globalAlpha = 0.92
      ctx.fill()
      ctx.globalAlpha = 0.22
      ctx.strokeStyle = ts.color
      ctx.lineWidth = 4
      ctx.stroke()
      ctx.globalAlpha = 1
      // 标签
      ctx.font = '11px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif'
      ctx.fillStyle = '#1e293b'
      ctx.textAlign = 'center'
      ctx.textBaseline = 'top'
      ctx.fillText(n.label, n.x, n.y + ts.r + 4)
    })
    rafId = requestAnimationFrame(tick)
  }
  cancelAnimationFrame(rafId)
  layoutSeed(graphData.nodes)
  // 快速跑几步以形成较好初始布局
  for (let i = 0; i < 40; i++) simulateOnce(graphData.nodes, graphData.edges, W, H)
  tick()
}

function randomizeGraph() {
  const mock = buildMockGraph()
  graphData.nodes.splice(0, graphData.nodes.length, ...mock.nodes)
  graphData.edges.splice(0, graphData.edges.length, ...mock.edges)
  nextTick(() => drawGraph())
}

// ==================== 快捷问法 / 流程阶段 ====================
const showMetricsFull = ref(false)
const showMoreQuick = ref(false)

const PHASES = [
  { key: 'requirement', label: 'S1 · 需求架构', desc: '意图识别 / 需求知识图谱', color: '#6366f1' },
  { key: 'graph',       label: 'S2 · 知识图谱', desc: '璇玑 Mox Graph 构建',   color: '#0ea5e9' },
  { key: 'design',      label: 'S3 · 方案设计', desc: '架构 / 流程图 / 工作流', color: '#10b981' },
  { key: 'develop',     label: 'S4 · 开发执行', desc: '算法联盟 / 浏览器 / 机器人', color: '#f59e0b' },
  { key: 'release',     label: 'S5 · 发布监控', desc: '监控 / 验收 / 知识库',    color: '#8b5cf6' }
]

const FLOW_STAGES = [
  { key: 'req',   label: '①需求采集 · 意图识别', hint: '用自然语言描述你的项目或问题，系统自动采集角色/目标/约束' },
  { key: 'arch',  label: '②架构设计 · 专家辩论', hint: '架构 / 算法 / 数据 / 图谱 多位专家组队辩论' },
  { key: 'impl',  label: '③实现方案 · 工作流编排', hint: '输出 PRD / ERD / 流程图，并编排开发工作流' },
  { key: 'test',  label: '④开发测试 · 迭代修复', hint: '通过算法联盟 + 浏览器任务，驱动开发/测试循环' },
  { key: 'acc',   label: '⑤验收发布 · 门禁审批', hint: '生成验收清单，对接发布与监控看板' },
  { key: 'done',  label: '⑥归档沉淀 · 知识库',   hint: '产物归档入知识库，并形成最佳实践沉淀' }
]
const localPhase = ref('requirement')
const currentStage = ref(0)
const requirementFlowMode = ref(false)
const phaseProgress = reactive({ requirement: 8, graph: 0, design: 0, develop: 0, release: 0 })
const phaseDone = computed(() => ({
  requirement: phaseProgress.requirement >= 100,
  graph:       phaseProgress.graph >= 100,
  design:      phaseProgress.design >= 100,
  develop:     phaseProgress.develop >= 100,
  release:     phaseProgress.release >= 100
}))
const phaseCompleteCount = computed(() => Object.values(phaseDone.value).filter(Boolean).length)
const projectOverall = computed(() => {
  const values = Object.values(phaseProgress)
  if (!values.length) return 0
  return Math.round(values.reduce((a, b) => a + b, 0) / values.length)
})

function selectPhase(key) {
  localPhase.value = key
  const mapStage = { requirement: 0, graph: 1, design: 2, develop: 3, release: 4 }
  currentStage.value = mapStage[key] ?? 0
  // 同步通知顶部 PhasePipeline
  try {
    window.dispatchEvent(new CustomEvent('mox:set-phase', { detail: { key } }))
  } catch (_) {}
}

function advanceFlowStage() {
  if (currentStage.value < 5) currentStage.value++
  const key = ['requirement','graph','design','develop','release'][Math.min(4, currentStage.value)]
  if (key) localPhase.value = key
  // 模拟推进
  phaseProgress[key] = Math.min(100, (phaseProgress[key] || 0) + 14)
}

function runFullFlow() {
  // 前端演示版：逐步推进进度（后端完整流水线由 /ai 页的 alliance SSE 负责）
  doSmartRoute()
}

// ==================== 快捷问法 ====================
const QUICK_QUESTIONS = [
  { icon: '🏢', label: '生成公司官网的需求图谱', hint: '覆盖角色/目标/用例/数据/技术', prompt: '生成「公司官网」的全维需求图谱：含角色、目标、用例、数据、技术选型与验收清单。', chip: '需求知识图谱' },
  { icon: '🧾', label: '需求知识图谱', hint: '生成项目需求的结构化知识图谱', prompt: '请生成当前项目的需求知识图谱，并输出节点与关系边列表。', chip: '需求知识图谱' },
  { icon: '🏗', label: '需求架构（S1）', hint: '输入自定义问题，自动生成架构草案', prompt: '请对当前项目做全维的需求架构分析：业务场景、角色、核心用例、非功能需求。', chip: '需求知识图谱' },
  { icon: '⚙️', label: '算法分', hint: '复杂度 / 推荐 / 数据结构选型', prompt: '对当前对话/问题涉及的算法进行复杂度分析并给出推荐方案。', chip: '算法分析' },
  { icon: '🩸', label: '血清空（初始化会话）', hint: '清空上下文，重新开始项目分析', prompt: '（清空上下文）请从 0 开始重新分析当前项目。', chip: '自定义问题' },
  { icon: '📥', label: '导入需求', hint: '上传文档/JSON 解析为需求节点', prompt: '请导入并解析以下需求文件：', chip: '自定义问题' },
  { icon: '📌', label: '转任务', hint: '将当前问题拆解为可执行任务清单', prompt: '请将本次分析拆解为任务清单，并给出优先级与负责人。', chip: '自定义问题' },
  { icon: '🚀', label: '创建项目', hint: '以当前问题创建一个全新的项目', prompt: '基于以上分析，创建一个新项目并输出项目信息、初始里程碑与阶段划分。', chip: '自定义问题' }
]
const visibleQuickQuestions = computed(() => showMoreQuick.value ? QUICK_QUESTIONS : QUICK_QUESTIONS.slice(0, 4))
const selectedChip = ref('需求知识图谱')
const customPlaceholder = computed(() => {
  if (selectedChip.value === '算法分') return '算法分：输入你想分析的算法/数据结构问题…'
  if (selectedChip.value === '生成公司官网的需求图谱') return '生成公司官网的需求图谱：请描述行业、目标客群、期望栏目…'
  if (selectedChip.value === '需求知识图谱') return '需求知识图谱：请描述项目/产品，系统将生成结构化知识图谱…'
  if (selectedChip.value === '需求架构（S1）') return '需求架构：输入自定义问题，系统将全维分析处理…'
  if (selectedChip.value === '血清空（初始化会话）') return '血清空：请确认要重置上下文，并输入新项目的描述…'
  return '自定义问题：输入你的问题，系统将全维分析处理（支持生成需求图谱 / 架构 / 算法 / 任务拆解 / 创建项目）…'
})

function pickQuick(q) {
  selectedChip.value = q.chip || q.label
  question.value = q.prompt
  if (q.label === '血清空（初始化会话）') {
    results.value = []
    debateSummary.value = ''
    algorithmResult.value = null
    routingResult.value = null
    ElMessage.success('已血清空当前联盟输出，可重新开始分析')
  }
}

// ==================== 流程模式动作 ====================
const modeLabelMap = {
  smart: '智能路由 · 全维',
  single: '单专家',
  multi: '多专家协同',
  debate: '专家辩论',
  algorithm: '算法分析 · 算法分'
}

function askQuickAnalysis() {
  // 快速分析 = 简化版智能路由，仅路由分析 + 咨询
  Promise.all([doRouteOnly()]).then(() => {
    if (routingResult.value?.selected?.length) {
      doSmartRoute()
    }
  })
}
function addFollowUp() {
  if (!question.value.trim()) return
  ElMessage.success('已追加为后续问题 · 发送给 AI 助手 X 可继续追问')
  openNewChatView(question.value)
}
function clearCurrentConversation() {
  results.value = []
  debateSummary.value = ''
  algorithmResult.value = null
  routingResult.value = null
  question.value = ''
  ElMessage.success('已清空当前联盟对话输出')
}
function importGraphData() {
  useGraphData.value = true
  graphDataJson.value = '{"nodes":[{"id":"n1","label":"A"},{"id":"n2","label":"B"}],"edges":[{"source":"n1","target":"n2","type":"rel"}]}'
  ElMessage.success('已注入示例图谱 JSON（可编辑后开始分析）')
}
function convertQuestionToTask() {
  if (!question.value.trim()) return
  ElMessage.success(`已将问题转为任务 · ${question.value.slice(0, 24)}...`)
}
function createProjectFromQuestion() {
  if (!question.value.trim()) return
  ensureAndInjectProject()
}
async function ensureProject() {
  ensureAndInjectProject()
}
function ensureAndInjectProject() {
  if (currentProject.value) {
    ElMessage.info(`当前项目：${currentProject.value.name}，可继续跟进。`)
    return
  }
  const suggestion = question.value
    ? (question.value.slice(0, 16) + '…')
    : '新璇玑项目'
  const pj = {
    id: 'pj_' + Date.now().toString(36),
    name: suggestion || '璇玑联盟新项目',
    description: question.value || '由专家联盟创建',
    category: selectedChip.value === '生成公司官网的需求图谱' ? '官网/营销' : '定制软件',
    status: '规划中'
  }
  try {
    // 若 createAndSelect 存在则用；不存在则降级本地模拟
    if (typeof createAndSelect === 'function') {
      createAndSelect(pj)
    }
    ElMessage.success(`已创建并选择项目：${pj.name}`)
  } catch (e) {
    ElMessage.warning(e.message || '创建项目失败')
  }
}
function openNewChatView(withInitial) {
  const query = {}
  if (withInitial && String(withInitial).trim()) query.initial = encodeURIComponent(withInitial)
  if (currentProject.value?.id) query.projectId = currentProject.value.id
  router.push({ path: '/ai', query })
}
function goToGraphPage() {
  const q = currentProject.value?.id ? { projectId: currentProject.value.id } : {}
  router.push({ path: '/graph', query })
}
function copyText(t) {
  try {
    const ta = document.createElement('textarea')
    ta.value = t || ''
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
    ElMessage.success('已复制到剪贴板')
  } catch (_) {
    ElMessage.warning('复制失败，请手动选择文本')
  }
}
function appendResultAsInput(r) {
  const before = question.value ? question.value + '\n' : ''
  question.value = before + '【后续分析】' + String((r && r.response) || '').slice(0, 240)
}
function exportConversation() {
  const payload = {
    project: currentProject.value || null,
    mode: mode.value,
    question: question.value,
    routing: routingResult.value || null,
    results: results.value,
    algorithmResult: algorithmResult.value || null,
    debateSummary: debateSummary.value || null,
    exportedAt: new Date().toISOString()
  }
  try {
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `expert-alliance-${Date.now()}.json`
    a.click()
    URL.revokeObjectURL(url)
    ElMessage.success('已导出联盟对话 + 路由 + 结果为 JSON')
  } catch (_) {
    ElMessage.warning('导出失败')
  }
}

// 页面阶段变化：更新 PhasePipeline 的进度（演示）
watch(localPhase, (k) => {
  phaseProgress[k] = Math.max(phaseProgress[k] || 0, 10 + Math.round(Math.random() * 8))
})

// Canvas 大小变化时重绘
let resizeObs = null
onMounted(async () => {
  await loadAll()
  // 初始化 Mock 图谱
  const mock = buildMockGraph()
  graphData.nodes.push(...mock.nodes)
  graphData.edges.push(...mock.edges)
  await nextTick()
  try {
    if (window.ResizeObserver && graphStageRef.value) {
      resizeObs = new ResizeObserver(() => drawGraph())
      resizeObs.observe(graphStageRef.value)
    }
  } catch (_) {}
  drawGraph()
})
onBeforeUnmount(() => {
  cancelAnimationFrame(rafId)
  if (resizeObs) try { resizeObs.disconnect() } catch (_) {}
})
</script>

<style scoped>
.expert-center {
  display: flex;
  flex-direction: column;
  gap: 14px;
  height: 100%;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 4px 2px 0;
}
.head-left { flex: 1; min-width: 0; }
.head-brand {
  display: flex;
  align-items: center;
  gap: 14px;
}
.brand-mark {
  width: 46px; height: 46px; flex-shrink: 0;
  background: #ffffff;
  border-radius: 14px;
  display: grid;
  place-items: center;
  box-shadow: 0 4px 14px -6px rgba(99,102,241,0.28);
  border: 1px solid rgba(99,102,241,0.12);
}
.bm-svg { width: 34px; height: 34px; }
.head-titles { min-width: 0; flex: 1; }
.head-title-row {
  display: flex; align-items: center; gap: 10px;
  margin-bottom: 2px;
}
.page-title {
  margin: 0;
  font-size: 19px;
  font-weight: 800;
  letter-spacing: 0.2px;
  background: linear-gradient(135deg, #6366f1, #0ea5e9 55%, #10b981);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}
.ver-tag {
  background: linear-gradient(135deg, #6366f1, #0ea5e9) !important;
  border: none !important;
  font-size: 11px;
  padding: 0 8px;
  height: 22px;
  line-height: 20px;
}
.ver-tag-alt {
  color: #6366f1 !important;
  border-color: #c7d2fe !important;
  font-size: 11px;
  height: 22px;
  line-height: 20px;
  padding: 0 8px;
}
.page-subtitle {
  margin: 0;
  font-size: 12.5px;
  color: var(--text-3);
  line-height: 1.7;
}
.hl-project {
  color: var(--brand-dark);
  background: var(--brand-soft);
  padding: 1px 8px;
  border-radius: 6px;
  font-weight: 600;
}
.proj-cat {
  margin-left: 6px;
  font-size: 11px;
  color: #10b981;
  background: #d1fae5;
  padding: 1px 6px;
  border-radius: 6px;
}
.muted-plain { color: var(--text-3); }
.link-like { cursor: pointer; color: var(--brand); text-underline-offset: 3px; }
.link-like:hover { color: var(--brand-dark); }
.head-actions {
  display: flex; gap: 8px; align-items: center;
  flex-shrink: 0;
}

.phi-shell {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(300px, 382px) minmax(0, 1fr) minmax(340px, 420px);
  grid-template-rows: 1fr;
  gap: 14px;
}
.col {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
  min-height: 0;
}
@media (max-width: 1400px) {
  .phi-shell {
    grid-template-columns: 300px 1fr;
  }
  .phi-shell .col-right {
    grid-column: 1 / -1;
  }
}
@media (max-width: 900px) {
  .phi-shell {
    grid-template-columns: 1fr;
  }
}

.card {
  background: #ffffff;
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 16px 16px 16px;
  box-shadow: 0 1px 2px rgba(15,23,42,0.03);
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
  min-height: 0;
}
.card-tight { padding: 12px 14px; }
.card-head {
  display: flex;
  align-items: baseline;
  gap: 10px;
}
.card-head.between { justify-content: space-between; align-items: center; }
.card-title {
  font-size: 13px;
  font-weight: 700;
  color: #0f172a;
  letter-spacing: 0.2px;
}
.card-title-wrap { display: flex; align-items: center; gap: 8px; }
.card-sub {
  font-size: 11px;
  color: var(--text-3);
}
.count-pill {
  font-size: 11px;
  color: var(--brand-dark);
  background: var(--brand-soft);
  padding: 2px 8px;
  border-radius: 999px;
  font-weight: 600;
}
.section-title {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
}

.phase-nav .phase-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.phase-row {
  display: grid;
  grid-template-columns: 30px 1fr 60px 14px;
  align-items: center;
  gap: 10px;
  padding: 10px 10px;
  border-radius: 11px;
  cursor: pointer;
  transition: all 0.18s ease;
  border: 1px solid transparent;
}
.phase-row:hover { background: #f8fafc; }
.phase-row.active {
  background: linear-gradient(135deg, rgba(99,102,241,0.08), rgba(16,185,129,0.06));
  border-color: rgba(99,102,241,0.25);
  box-shadow: inset 3px 0 0 var(--brand, #6366f1);
}
.phase-row.done { opacity: 0.9; }
.phase-idx {
  width: 26px; height: 26px;
  border-radius: 8px;
  color: #fff;
  font-size: 12px;
  font-weight: 800;
  display: grid;
  place-items: center;
}
.phase-body { min-width: 0; }
.phase-name {
  font-size: 13px;
  font-weight: 700;
  color: #0f172a;
  line-height: 1.2;
}
.phase-desc {
  font-size: 11px;
  color: var(--text-3);
  margin-top: 2px;
  line-height: 1.4;
}
.phase-bar {
  height: 4px;
  background: #e2e8f0;
  border-radius: 999px;
  overflow: hidden;
}
.phase-bar-fill {
  height: 100%;
  border-radius: 999px;
  transition: width 0.3s ease;
}
.phase-chev {
  color: #cbd5e1;
  font-size: 18px;
  line-height: 1;
  text-align: right;
}
.phase-row.active .phase-chev { color: var(--brand-dark); }

.experts-card { flex: 1; min-height: 0; }
.exp-scroll {
  max-height: 52vh;
  min-height: 300px;
  flex: 1;
}
.filter-bar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.expert-card {
  position: relative;
  display: grid;
  grid-template-columns: 40px 1fr auto;
  align-items: start;
  gap: 10px;
  padding: 10px 10px;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.18s ease;
  border: 1.5px solid transparent;
  margin-bottom: 6px;
  background: #ffffff;
}
.expert-card:hover { background: #f8fafc; border-color: #e2e8f0; }
.expert-card.sel {
  background: linear-gradient(135deg, rgba(99,102,241,0.07), rgba(14,165,233,0.05));
  border-color: #6366f1;
  box-shadow: 0 6px 18px -14px rgba(99,102,241,0.55);
}
.expert-card.offline { opacity: 0.6; }
.expert-avatar {
  width: 40px;
  height: 40px;
  border-radius: 11px;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 18px;
}
.expert-info { min-width: 0; }
.expert-name-row { display: flex; align-items: center; gap: 6px; }
.expert-name { font-weight: 700; font-size: 13px; color: #0f172a; }
.consult-count {
  font-size: 10px;
  background: var(--brand);
  color: #fff;
  padding: 1px 6px;
  border-radius: 10px;
  font-weight: 600;
}
.expert-type { font-size: 11px; color: var(--text-3); margin: 2px 0; }
.expert-stats { display: flex; gap: 8px; font-size: 10.5px; color: var(--text-3); margin: 3px 0; }
.stat-item { display: inline-flex; align-items: center; gap: 2px; }
.expert-caps { display: flex; gap: 4px; flex-wrap: wrap; }
.expert-check {
  width: 20px; height: 20px;
  border-radius: 50%;
  background: var(--brand);
  color: #fff;
  font-size: 12px;
  font-weight: 800;
  display: grid;
  place-items: center;
  align-self: center;
}
.sel-summary {
  margin-top: 4px;
  background: #f8fafc;
  border: 1px dashed #cbd5e1;
  border-radius: 10px;
  padding: 8px 10px;
}
.sel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}
.sel-title { font-size: 12px; font-weight: 700; color: #0f172a; }
.sel-chips { display: flex; gap: 6px; flex-wrap: wrap; }
.chip-sel {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 6px 3px 6px;
  border-radius: 999px;
  background: #fff;
  border: 1px solid #e2e8f0;
  font-size: 11.5px;
  font-weight: 600;
  color: #334155;
}
.chip-dot { width: 8px; height: 8px; border-radius: 50%; }

.mid-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.mh-left { display: flex; align-items: center; gap: 12px; min-width: 0; }
.mh-logo {
  width: 42px; height: 42px;
  border-radius: 12px;
  background: linear-gradient(135deg, rgba(99,102,241,0.10), rgba(14,165,233,0.08));
  display: grid;
  place-items: center;
}
.ai-logo-svg { width: 30px; height: 30px; }
.mh-text { min-width: 0; }
.mh-title {
  font-size: 14.5px;
  font-weight: 800;
  color: #0f172a;
  display: flex;
  align-items: center;
  gap: 8px;
}
.badge-ver {
  font-size: 10.5px;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
  padding: 1px 8px;
  border-radius: 999px;
  font-weight: 600;
}
.mh-sub {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 2px;
}
.chip-selected {
  display: inline-block;
  padding: 1px 8px;
  background: linear-gradient(135deg, #ede9fe, #dbeafe);
  color: #4338ca;
  border-radius: 6px;
  font-weight: 700;
  font-size: 11.5px;
  margin: 0 2px;
}
.mh-right { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }

.quick-q-card { gap: 8px; }
.qq-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.qq-label {
  font-size: 12px;
  font-weight: 700;
  color: #0f172a;
}
.qq-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}
@media (max-width: 1100px) { .qq-grid { grid-template-columns: 1fr; } }
.qq-btn {
  display: grid;
  grid-template-columns: 26px 1fr 16px;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  cursor: pointer;
  text-align: left;
  transition: all 0.16s ease;
}
.qq-btn:hover {
  border-color: #c7d2fe;
  box-shadow: 0 6px 16px -10px rgba(99,102,241,0.45);
  transform: translateY(-1px);
  background: linear-gradient(135deg, #fafbff, #f8fbff);
}
.qq-btn.active {
  border-color: #6366f1;
  background: linear-gradient(135deg, rgba(99,102,241,0.08), rgba(14,165,233,0.06));
  box-shadow: inset 0 0 0 1px #6366f1;
}
.qq-emoji {
  font-size: 18px;
  line-height: 1;
  text-align: center;
}
.qq-text { min-width: 0; }
.qq-name {
  font-size: 12.5px;
  font-weight: 700;
  color: #0f172a;
  line-height: 1.2;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.qq-desc {
  font-size: 10.5px;
  color: var(--text-3);
  margin-top: 2px;
  line-height: 1.3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.qq-go { color: #cbd5e1; font-size: 13px; }
.qq-btn:hover .qq-go { color: #6366f1; }

.panel-col {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.ch-left { display: flex; flex-direction: column; gap: 2px; }
.proj-link {
  font-size: 11.5px;
  color: var(--text-3);
}
.proj-link b { color: var(--brand-dark); }
.consult-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.mode-switch { display: flex; }

.flow-head-bar {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 14px;
  padding: 10px 14px;
  background: linear-gradient(135deg, #fafbff, #f8fbff);
  border: 1px solid #e0e7ff;
  border-radius: 11px;
}
.fh-label { display: flex; align-items: center; gap: 10px; }
.fh-idx {
  width: 28px; height: 28px;
  border-radius: 9px;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
  display: grid;
  place-items: center;
  font-size: 13px;
  font-weight: 800;
}
.fh-text { font-size: 13px; font-weight: 700; color: #0f172a; }
.fh-desc { font-size: 11.5px; color: var(--text-3); line-height: 1.5; }
.fh-actions { display: flex; gap: 6px; flex-wrap: wrap; justify-content: flex-end; }

.mode-block {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.big-input .el-textarea__inner {
  min-height: 104px !important;
  padding: 12px 14px !important;
  font-size: 13.5px !important;
  line-height: 1.65 !important;
  border-radius: 12px !important;
}
.mode-desc {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  background: linear-gradient(135deg, #f8fafc, #f1f5f9);
  border-radius: 10px;
  font-size: 12.5px;
  color: var(--text-2);
}
.action-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.action-row .el-button {
  flex: 1 1 140px;
}
.act-cta {
  font-weight: 700;
  background: linear-gradient(135deg, #6366f1, #0ea5e9) !important;
  border: none !important;
}
.graph-data-area { display: flex; flex-direction: column; gap: 8px; }
.debate-config { display: flex; gap: 12px; align-items: center; flex-wrap: wrap; }

.selected-area {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  align-items: center;
}
.chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: var(--brand-soft);
  color: var(--brand-dark);
  padding: 3px 8px;
  border-radius: 7px;
  font-size: 12px;
  font-weight: 600;
}
.chip .score { font-size: 10px; opacity: 0.7; }
.chip-x { cursor: pointer; font-size: 11px; }
.chip-x:hover { color: var(--danger, #ef4444); }
.muted { color: var(--text-3); font-size: 12.5px; }

.routing-info {
  background: linear-gradient(135deg, rgba(99,102,241,0.06), rgba(14,165,233,0.05));
  border-radius: 10px;
  padding: 10px 12px;
  border: 1px solid #c7d2fe;
}
.routing-title { font-weight: 700; font-size: 12.5px; margin-bottom: 6px; color: #312e81; }
.routing-detail { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
.routing-experts { display: flex; flex-wrap: wrap; gap: 5px; width: 100%; margin-top: 4px; }

.results-block {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  border-top: 1px solid var(--border);
  padding-top: 10px;
  gap: 8px;
}
.rb-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
.rb-tools { display: flex; gap: 4px; }
.results-scroll { flex: 1; min-height: 240px; }
.results-title { font-size: 13.5px; font-weight: 800; margin: 0; }

.result-item {
  background: linear-gradient(180deg, #fafbff, #fff);
  border: 1px solid #e2e8f0;
  border-radius: 11px;
  padding: 12px;
  margin-bottom: 8px;
}
.result-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  gap: 8px;
  flex-wrap: wrap;
}
.result-meta { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.expert-badge {
  color: #fff;
  padding: 3px 10px;
  border-radius: 7px;
  font-size: 12px;
  font-weight: 700;
}
.confidence { font-size: 11.5px; color: var(--text-3); }
.duration { font-size: 11.5px; color: var(--text-3); }
.result-content {
  font-size: 13px;
  line-height: 1.75;
  color: #1e293b;
  white-space: pre-wrap;
  word-break: break-word;
}
.result-ops {
  margin-top: 8px;
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.algorithm-result { display: flex; flex-direction: column; gap: 10px; margin-top: 6px; }
.algo-section {
  background: #f8fafc;
  border-radius: 10px;
  padding: 12px;
}
.algo-section h5 { font-size: 12.5px; font-weight: 700; margin: 0 0 8px; }
.graph-stats { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 8px; }
.stat-chip {
  background: var(--brand-soft);
  color: var(--brand-dark);
  padding: 3px 9px;
  border-radius: 7px;
  font-size: 11px;
  font-weight: 600;
}
.top-nodes-title { font-size: 11px; color: var(--text-3); margin-bottom: 6px; }
.node-list { display: flex; gap: 5px; flex-wrap: wrap; }
.node-chip {
  background: #eef2ff;
  color: #4338ca;
  padding: 2px 8px;
  border-radius: 6px;
  font-size: 11px;
  font-family: ui-monospace, Menlo, Consolas, monospace;
}
.algo-item {
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  padding: 8px 10px;
  margin-bottom: 6px;
}
.algo-name { font-weight: 700; font-size: 12.5px; }
.algo-rec { font-size: 12px; color: var(--text-2); margin: 3px 0; }
.algo-complexity { font-size: 11px; color: var(--text-3); }

.ai-insight {
  background: linear-gradient(135deg, #fef3c7, #fde68a);
  border-radius: 10px;
  padding: 12px;
}
.ai-insight h5 {
  font-size: 12.5px;
  font-weight: 700;
  margin: 0 0 6px;
  display: flex; align-items: center; gap: 6px;
  color: #92400e;
}
.insight-content { font-size: 12.5px; line-height: 1.8; white-space: pre-wrap; color: #78350f; }

.debate-summary {
  margin-top: 6px;
  padding: 12px;
  background: linear-gradient(135deg, #f8fafc, #eef2ff);
  border-radius: 11px;
  border: 1px solid #e0e7ff;
}
.debate-final { font-size: 13px; line-height: 1.8; white-space: pre-wrap; color: #1e1b4b; }

.conv-empty {
  flex: 1;
  min-height: 180px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  gap: 10px;
  padding: 16px 8px;
  background:
    radial-gradient(520px 260px at 50% 0%, rgba(99,102,241,0.06), transparent),
    radial-gradient(520px 260px at 50% 100%, rgba(16,185,129,0.06), transparent);
  border-radius: 14px;
  border: 1px dashed #e2e8f0;
}
.empty-orb {
  width: 64px; height: 64px;
  display: grid;
  place-items: center;
  animation: orbSpin 8s linear infinite;
}
.eo-svg { width: 64px; height: 64px; }
@keyframes orbSpin { to { transform: rotate(360deg); } }
.empty-title { font-size: 14px; color: #1e293b; font-weight: 600; }
.empty-title b { color: var(--brand-dark); }
.empty-sub {
  font-size: 12px;
  color: var(--text-3);
  max-width: 460px;
  line-height: 1.7;
}

.right-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
.rh-left { display: flex; align-items: center; gap: 10px; }
.rh-logo {
  width: 42px; height: 42px;
  border-radius: 12px;
  background: linear-gradient(135deg, rgba(16,185,129,0.10), rgba(99,102,241,0.08));
  display: grid;
  place-items: center;
}
.rh-svg { width: 30px; height: 30px; }
.rh-text { min-width: 0; }
.rh-title { font-size: 13.5px; font-weight: 800; color: #0f172a; }
.rh-sub { font-size: 11.5px; color: var(--text-3); margin-top: 2px; }
.rh-right { display: flex; gap: 6px; flex-shrink: 0; }

.mox-canvas-card { gap: 10px; padding-bottom: 10px; }
.mc-legend {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.lg {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 999px;
  background: #f1f5f9;
  color: #334155;
  font-weight: 600;
}
.lg::before {
  content: '';
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 6px;
  vertical-align: -1px;
}
.lg-project::before  { background: #6366f1; }
.lg-goal::before    { background: #0ea5e9; }
.lg-actor::before   { background: #f59e0b; }
.lg-usecase::before { background: #10b981; }
.lg-data::before    { background: #ef4444; }
.lg-tech::before    { background: #8b5cf6; }
.lg-end::before     { background: #64748b; }

.mc-stage {
  position: relative;
  width: 100%;
  height: 320px;
  background:
    radial-gradient(520px 200px at 50% -20%, rgba(99,102,241,0.06), transparent),
    linear-gradient(135deg, #fafbfc, #f8fafc);
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  overflow: hidden;
}
.mc-stage canvas { display: block; }
.mc-stats { display: flex; gap: 6px; flex-wrap: wrap; }

.progress-card { gap: 12px; }
.progress-top {
  display: grid;
  grid-template-columns: 130px 1fr;
  align-items: center;
  gap: 14px;
}
.pt-info { min-width: 0; }
.pt-name {
  font-size: 13.5px;
  font-weight: 800;
  color: #0f172a;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pt-cat { font-size: 11.5px; color: var(--text-3); margin-top: 2px; }
.pt-rows { margin-top: 10px; display: flex; flex-direction: column; gap: 4px; }
.pt-row {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
}
.pt-k { color: var(--text-3); }
.pt-v { color: #0f172a; font-weight: 700; }

.pgs { display: flex; flex-direction: column; gap: 8px; }
.pg-row {
  display: grid;
  grid-template-columns: 120px 1fr;
  align-items: center;
  gap: 10px;
}
.pg-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11.5px;
  font-weight: 600;
  color: #334155;
}
.pg-dot { width: 8px; height: 8px; border-radius: 50%; }

.ov-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
  margin-top: 4px;
}
.ov-cell {
  background: #f8fafc;
  border-radius: 10px;
  padding: 10px 8px;
  text-align: center;
  border: 1px solid #e2e8f0;
}
.ov-cell.active {
  background: linear-gradient(135deg, #dbeafe, #bfdbfe);
  border-color: #3b82f6;
}
.ov-cell.success {
  background: linear-gradient(135deg, #dcfce7, #bbf7d0);
  border-color: #22c55e;
}
.ov-v {
  font-size: 18px;
  font-weight: 800;
  color: #1e293b;
}
.ov-k {
  font-size: 10.5px;
  color: var(--text-3);
  margin-top: 2px;
}

.metrics-card { gap: 8px; }
.mini-table { font-size: 12px; }
.mini-table :deep(.el-table__cell) {
  padding: 6px 8px !important;
}

.overview-grid, .overview-card, .span1, .span2, .card-pad, .main-grid, .panel-head,
.expert-status, .consult-input, .metrics-panel { display: none; }
</style>
