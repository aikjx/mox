<template>
  <div class="page-container kb-view">
    <!-- Header Section -->
    <div class="page-header kb-header">
      <div class="header-bg">
        <div class="bg-orb orb-1"></div>
        <div class="bg-orb orb-2"></div>
        <div class="bg-orb orb-3"></div>
      </div>
      <div class="header-content">
        <div class="page-header-left header-left">
          <div class="eyebrow">KNOWLEDGE BASE · 云盘知识库</div>
          <h1 class="page-title">云盘知识库</h1>
          <p class="page-subtitle">AI+知识图谱 智能分类、版本管理、架构分析</p>
        </div>
        <div class="page-header-actions header-right">
          <div class="stat-card" v-for="s in statCards" :key="s.label">
            <div class="stat-icon" :style="{ background: s.bg, color: s.color }">
              <el-icon><component :is="s.icon" /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-value">{{ s.value }}</div>
              <div class="stat-label">{{ s.label }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="page-content kb-main">
      <!-- Left Panel (38% ≈ 1 part) -->
      <aside class="kb-left">
        <!-- Search & Filters -->
        <div class="panel search-panel">
          <div class="search-box">
            <el-input
              v-model="searchQuery"
              placeholder="搜索文档标题、内容、标签..."
              clearable
              size="large"
              @keyup.enter="handleSearch"
              @clear="handleSearch"
            >
              <template #prefix>
                <el-icon><Search /></el-icon>
              </template>
              <template #append>
                <el-button @click="handleSearch" :loading="loading">搜索</el-button>
              </template>
            </el-input>
          </div>
          <div class="filter-row">
            <el-select v-model="filterStatus" placeholder="状态" size="default" clearable style="width: 100%">
              <el-option v-for="s in KB_STATUSES" :key="s.value" :label="s.label" :value="s.value" />
            </el-select>
          </div>
          <div class="filter-row">
            <el-date-picker
              v-model="filterDateRange"
              type="daterange"
              range-separator="至"
              start-placeholder="开始日期"
              end-placeholder="结束日期"
              size="default"
              style="width: 100%"
            />
          </div>
          <div class="filter-actions">
            <el-button size="small" @click="resetFilters">重置筛选</el-button>
          </div>
        </div>

        <!-- Category Tree -->
        <div class="panel tree-panel">
          <h3 class="section-title">
            <el-icon><Folder /></el-icon>
            分类目录
          </h3>
          <el-tree
            ref="categoryTreeRef"
            :data="categories"
            :props="{ label: 'name', children: 'children' }"
            :default-expand-all="true"
            node-key="id"
            highlight-current
            @node-click="handleCategoryClick"
            class="category-tree"
          >
            <template #default="{ node, data }">
              <span class="tree-node">
                <span class="tree-label">{{ data.name }}</span>
                <span class="tree-count">{{ data.count || 0 }}</span>
              </span>
            </template>
          </el-tree>
        </div>

        <!-- Tag Cloud -->
        <div class="panel tag-panel">
          <h3 class="section-title">
            <el-icon><CollectionTag /></el-icon>
            热门标签
          </h3>
          <div class="tag-cloud">
            <el-tag
              v-for="tag in tags"
              :key="tag.id"
              :class="['tag-item', { active: filterTag === tag.id }]"
              :style="{ fontSize: getTagSize(tag.count) + 'px' }"
              @click="handleTagClick(tag)"
              effect="plain"
              round
            >
              {{ tag.name }}
              <span class="tag-count">{{ tag.count }}</span>
            </el-tag>
          </div>
        </div>
      </aside>

      <!-- Right Panel (62% ≈ 1.618 parts) -->
      <section class="kb-right">
        <!-- Toolbar -->
        <div class="panel toolbar">
          <div class="toolbar-left">
            <el-radio-group v-model="viewMode" size="default">
              <el-radio-button value="list">
                <el-icon><List /></el-icon>
              </el-radio-button>
              <el-radio-button value="grid">
                <el-icon><Grid /></el-icon>
              </el-radio-button>
            </el-radio-group>
            <span class="result-count">共 {{ filteredDocuments.length }} 篇文档</span>
          </div>
          <div class="toolbar-right">
            <el-button @click="fetchDocuments" :loading="loading">
              <el-icon><Refresh /></el-icon> 刷新
            </el-button>
            <el-button type="warning" @click="batchAnalyze" :disabled="!selectedDocs.length">
              <el-icon><DataAnalysis /></el-icon> 批量分析
              <span v-if="selectedDocs.length" class="badge-count">{{ selectedDocs.length }}</span>
            </el-button>
            <el-button type="primary" @click="openCreateDialog">
              <el-icon><Plus /></el-icon> 新建文档
            </el-button>
          </div>
        </div>

        <!-- Document List -->
        <div v-if="loading && !documents.length" class="loading-state">
          <el-icon class="loading-spin"><Loading /></el-icon>
          <span>加载文档中...</span>
        </div>

        <div v-else-if="error && !documents.length" class="empty-state">
          <el-empty :description="error" :image-size="80">
            <el-button type="primary" @click="fetchDocuments">
              <el-icon><Refresh /></el-icon> 重试
            </el-button>
          </el-empty>
        </div>

        <div v-else-if="filteredDocuments.length === 0" class="empty-state">
          <el-empty description="暂无文档，点击右上角新建" :image-size="80">
            <el-button type="primary" @click="openCreateDialog">
              <el-icon><Plus /></el-icon> 立即创建
            </el-button>
          </el-empty>
        </div>

        <!-- Grid View -->
        <div v-else-if="viewMode === 'grid'" class="doc-grid">
          <div
            v-for="doc in filteredDocuments"
            :key="doc.id"
            class="doc-card"
            :class="{ selected: selectedDocs.includes(doc.id) }"
            @click="selectDocument(doc)"
            @dblclick="viewDocument(doc)"
          >
            <div class="doc-card-header">
              <el-checkbox
                :model-value="selectedDocs.includes(doc.id)"
                @click.stop
                @change="toggleSelectDoc(doc)"
              />
              <el-tag
                :type="getCategoryTagType(doc.category)"
                size="small"
                effect="light"
                round
              >
                {{ getCategoryLabel(doc.category) }}
              </el-tag>
              <el-tag
                :type="getStatusType(doc.status)"
                size="small"
                effect="dark"
                round
              >
                {{ getStatusLabel(doc.status) }}
              </el-tag>
            </div>
            <h3 class="doc-card-title">{{ doc.title }}</h3>
            <p v-if="doc.summary" class="doc-card-desc">{{ truncateText(doc.summary, 80) }}</p>
            <div class="doc-card-tags" v-if="doc.tags?.length">
              <el-tag
                v-for="t in doc.tags.slice(0, 3)"
                :key="t"
                size="small"
                effect="plain"
                type="info"
                round
              >
                {{ t }}
              </el-tag>
              <span v-if="doc.tags.length > 3" class="more-tags">+{{ doc.tags.length - 3 }}</span>
            </div>
            <div class="doc-card-meta">
              <span class="meta-item">
                <el-icon><Folder /></el-icon>
                {{ doc.category || '未分类' }}
              </span>
              <span class="meta-item">
                <el-icon><Clock /></el-icon>
                {{ formatTime(doc.updated_at) }}
              </span>
              <span class="meta-item" :class="{ 'ai-done': doc.ai_analyzed }">
                <el-icon><MagicStick /></el-icon>
                {{ doc.ai_analyzed ? '已分析' : '未分析' }}
              </span>
            </div>
            <div class="doc-card-actions" @click.stop>
              <el-button size="small" text type="primary" @click="viewDocument(doc)">
                <el-icon><View /></el-icon> 查看
              </el-button>
              <el-button size="small" text @click="openEditDialog(doc)">
                <el-icon><Edit /></el-icon> 编辑
              </el-button>
              <el-button size="small" text type="warning" @click="analyzeDocument(doc.id)">
                <el-icon><MagicStick /></el-icon> 分析
              </el-button>
              <el-button size="small" text type="danger" @click="deleteDocument(doc.id)">
                <el-icon><Delete /></el-icon>
              </el-button>
            </div>
          </div>
        </div>

        <!-- List View -->
        <div v-else class="doc-list">
          <div
            v-for="doc in filteredDocuments"
            :key="doc.id"
            class="doc-row"
            :class="{ selected: selectedDocs.includes(doc.id) }"
          >
            <div class="row-checkbox">
              <el-checkbox
                :model-value="selectedDocs.includes(doc.id)"
                @change="toggleSelectDoc(doc)"
              />
            </div>
            <div class="row-main" @click="viewDocument(doc)">
              <div class="row-title-row">
                <h3 class="row-title">{{ doc.title }}</h3>
                <el-tag :type="getCategoryTagType(doc.category)" size="small" effect="light" round>
                  {{ getCategoryLabel(doc.category) }}
                </el-tag>
                <el-tag :type="getStatusType(doc.status)" size="small" effect="dark" round>
                  {{ getStatusLabel(doc.status) }}
                </el-tag>
                <el-tag v-if="doc.ai_analyzed" type="success" size="small" effect="plain" round>
                  <el-icon><MagicStick /></el-icon> 已分析
                </el-tag>
              </div>
              <p v-if="doc.summary" class="row-desc">{{ truncateText(doc.summary, 120) }}</p>
              <div class="row-tags" v-if="doc.tags?.length">
                <el-tag
                  v-for="t in doc.tags"
                  :key="t"
                  size="small"
                  effect="plain"
                  type="info"
                  round
                >
                  {{ t }}
                </el-tag>
              </div>
            </div>
            <div class="row-info">
              <div class="info-item">
                <el-icon><Folder /></el-icon>
                <span>{{ doc.category || '未分类' }}</span>
              </div>
              <div class="info-item">
                <el-icon><Clock /></el-icon>
                <span>{{ formatTime(doc.updated_at) }}</span>
              </div>
              <div class="info-item">
                <el-icon><Clock /></el-icon>
                <span>v{{ doc.version_count || 1 }}</span>
              </div>
            </div>
            <div class="row-actions" @click.stop>
              <el-button size="small" type="primary" plain @click="viewDocument(doc)">
                <el-icon><View /></el-icon>
              </el-button>
              <el-button size="small" plain @click="openEditDialog(doc)">
                <el-icon><Edit /></el-icon>
              </el-button>
              <el-button size="small" type="warning" plain @click="analyzeDocument(doc.id)">
                <el-icon><MagicStick /></el-icon>
              </el-button>
              <el-button size="small" type="danger" plain @click="deleteDocument(doc.id)">
                <el-icon><Delete /></el-icon>
              </el-button>
            </div>
          </div>
        </div>
      </section>
    </div>

    <!-- Document Detail Modal (Full Screen) -->
    <el-dialog
      v-model="detailVisible"
      :title="null"
      width="100%"
      top="0"
      :show-close="false"
      class="detail-dialog"
      append-to-body
      destroy-on-close
    >
      <div class="detail-container">
        <div class="detail-header">
          <div class="detail-title-area">
            <h2 class="detail-title">{{ selectedDoc?.title }}</h2>
            <div class="detail-meta">
              <el-tag :type="getCategoryTagType(selectedDoc?.category)" effect="light" round>
                {{ getCategoryLabel(selectedDoc?.category) }}
              </el-tag>
              <el-tag :type="getStatusType(selectedDoc?.status)" effect="dark" round>
                {{ getStatusLabel(selectedDoc?.status) }}
              </el-tag>
              <span class="meta-sep">·</span>
              <span class="meta-text">版本 {{ selectedDoc?.version_count || 1 }}</span>
              <span class="meta-sep">·</span>
              <span class="meta-text">更新于 {{ formatTime(selectedDoc?.updated_at) }}</span>
            </div>
          </div>
          <div class="detail-actions">
            <el-button @click="closeDetail">
              <el-icon><Close /></el-icon> 关闭
            </el-button>
            <el-button @click="openEditDialog(selectedDoc)">
              <el-icon><Edit /></el-icon> 编辑
            </el-button>
            <el-button type="primary" @click="analyzeDocument(selectedDoc?.id)">
              <el-icon><MagicStick /></el-icon> AI 分析
            </el-button>
          </div>
        </div>

        <el-tabs v-model="detailTab" class="detail-tabs" @tab-change="handleDetailTabChange">
          <el-tab-pane label="内容" name="content">
            <div class="tab-content-wrapper">
              <div v-if="detailMode === 'view'" class="content-view">
                <div class="content-markdown" v-html="renderedContent"></div>
              </div>
              <div v-else class="content-edit">
                <el-input
                  v-model="editForm.content"
                  type="textarea"
                  :autosize="{ minRows: 15, maxRows: 30 }"
                  placeholder="输入文档内容，支持 Markdown 语法..."
                />
                <div class="edit-actions">
                  <el-button @click="detailMode = 'view'">取消</el-button>
                  <el-button type="primary" @click="saveFromDetail">保存</el-button>
                </div>
              </div>
              <div class="view-toggle">
                <el-button-group>
                  <el-button :type="detailMode === 'view' ? 'primary' : ''" @click="detailMode = 'view'">
                    <el-icon><View /></el-icon> 预览
                  </el-button>
                  <el-button :type="detailMode === 'edit' ? 'primary' : ''" @click="detailMode = 'edit'">
                    <el-icon><Edit /></el-icon> 编辑
                  </el-button>
                </el-button-group>
              </div>
            </div>
          </el-tab-pane>

          <el-tab-pane label="版本历史" name="versions">
            <div class="tab-content-wrapper">
              <div v-if="docVersions.length" class="version-timeline">
                <div
                  v-for="(ver, idx) in docVersions"
                  :key="ver.version"
                  class="version-item"
                >
                  <!-- KbVersion 的版本号自带 "v" 前缀（next_version 产出 v{n+1}），模板不再补 v -->
                  <div class="version-badge">{{ ver.version }}</div>
                  <div class="version-info">
                    <div class="version-header">
                      <span class="version-label">版本 {{ ver.version }}</span>
                      <span class="version-time">{{ formatTime(ver.created_at) }}</span>
                    </div>
                    <p class="version-note">{{ ver.note || '无版本说明' }}</p>
                    <div class="version-actions">
                      <el-button size="small" @click="compareWithPrevious(idx)">
                        <el-icon><ArrowLeft /></el-icon> 与上一版对比
                      </el-button>
                      <el-button size="small" @click="viewVersion(ver)">
                        <el-icon><View /></el-icon> 查看此版
                      </el-button>
                      <el-button
                        v-if="idx < docVersions.length - 1"
                        size="small"
                        type="warning"
                        @click="revertVersion(selectedDoc?.id, ver)"
                      >
                        <el-icon><Refresh /></el-icon> 回滚到此版
                      </el-button>
                    </div>
                  </div>
                </div>
              </div>
              <el-empty v-else description="暂无版本记录" :image-size="60" />
            </div>
          </el-tab-pane>

          <el-tab-pane label="AI分析" name="analysis">
            <div class="tab-content-wrapper">
              <div v-if="aiAnalysis" class="analysis-panel">
                <div class="analysis-section">
                  <h4 class="analysis-title">
                    <el-icon><DataAnalysis /></el-icon>
                    摘要
                  </h4>
                  <p class="analysis-text">{{ aiAnalysis.summary }}</p>
                </div>
                <div class="analysis-grid">
                  <div class="analysis-section">
                    <h4 class="analysis-title">
                      <el-icon><PriceTag /></el-icon>
                      关键词
                    </h4>
                    <div class="keyword-list">
                      <el-tag
                        v-for="(kw, i) in aiAnalysis.keywords"
                        :key="i"
                        effect="plain"
                        type="warning"
                        round
                      >
                        {{ kw }}
                      </el-tag>
                    </div>
                  </div>
                  <div class="analysis-section">
                    <h4 class="analysis-title">
                      <el-icon><Share /></el-icon>
                      提取关系
                    </h4>
                    <!-- wire 上没有 category_suggestions 这一族；AnalysisResult / KbDocument 都发 relations -->
                    <div class="suggestion-list">
                      <div
                        v-for="(r, i) in aiAnalysis.relations"
                        :key="'rel-'+i"
                        class="suggestion-item"
                      >
                        <span class="sug-name">{{ r.source }} → {{ r.target }}</span>
                        <el-tag size="small" type="info" effect="plain" round>{{ r.relation }}</el-tag>
                      </div>
                    </div>
                  </div>
                </div>
                <div class="analysis-section">
                  <h4 class="analysis-title">
                    <el-icon><PriceTag /></el-icon>
                    分析标签
                  </h4>
                  <div class="tag-suggestions">
                    <!-- AnalysisResult.tags / KbDocument.tags 都是字符串数组，没有 name+confidence 的对象 -->
                    <el-tag
                      v-for="(t, i) in aiAnalysis.tags"
                      :key="'tag-'+i"
                      effect="plain"
                      type="info"
                      round
                    >
                      {{ t }}
                    </el-tag>
                  </div>
                </div>
                <div class="analysis-section" v-if="entities.length">
                  <h4 class="analysis-title">
                    <el-icon><Share /></el-icon>
                    提取实体
                  </h4>
                  <div class="entity-table">
                    <div class="entity-row entity-header">
                      <span>实体</span>
                      <span>类型</span>
                      <span>出现次数</span>
                    </div>
                    <div
                      v-for="(ent, i) in entities"
                      :key="'ent-'+i"
                      class="entity-row"
                    >
                      <span :title="ent.snippet">{{ ent.name }}</span>
                      <span class="entity-type">{{ ent.type }}</span>
                      <!-- KbEntity 的五键是 id/name/type/frequency/snippet，没有 confidence ⇒ 进度条换成计次 -->
                      <span class="entity-freq">{{ ent.frequency ?? 0 }}</span>
                    </div>
                  </div>
                </div>
              </div>
              <el-empty v-else description="尚未进行 AI 分析" :image-size="60">
                <el-button type="primary" @click="analyzeDocument(selectedDoc?.id)">
                  <el-icon><MagicStick /></el-icon> 开始分析
                </el-button>
              </el-empty>
            </div>
          </el-tab-pane>

          <el-tab-pane label="变更历史" name="history">
            <div class="tab-content-wrapper">
              <div v-if="docHistory.length" class="history-timeline">
                <div
                  v-for="(h, i) in docHistory"
                  :key="'h-'+i"
                  class="history-item"
                >
                  <!-- /kb/documents/:id/history 的行是版本快照，wire 上没有 action / user / detail 三键 -->
                  <div class="history-dot"></div>
                  <div class="history-content">
                    <div class="history-action">
                      <span class="action-label">版本 {{ h.version }}</span>
                      <span class="history-time">{{ formatTime(h.created_at) }}</span>
                    </div>
                    <div class="history-user" v-if="h.title">{{ h.title }}</div>
                    <div class="history-detail" v-if="h.note">{{ h.note }}</div>
                  </div>
                </div>
              </div>
              <el-empty v-else description="暂无变更记录" :image-size="60" />
            </div>
          </el-tab-pane>

          <el-tab-pane label="图谱关联" name="graph">
            <div class="tab-content-wrapper">
              <div class="graph-entities">
                <div class="graph-header">
                  <h4>整篇文档挂图</h4>
                  <!-- 挂图/解图都是文档级：graph-link 两端点不收请求体，wire 上也没有按实体的边接口 -->
                  <el-button
                    v-if="isGraphLinked"
                    size="small"
                    type="danger"
                    plain
                    @click="unlinkGraph(selectedDoc?.id)"
                  >
                    <el-icon><Close /></el-icon> 解除关联
                  </el-button>
                  <el-button
                    v-else
                    size="small"
                    type="primary"
                    plain
                    @click="linkGraph(selectedDoc?.id)"
                  >
                    <el-icon><Share /></el-icon> 关联图谱
                  </el-button>
                </div>
                <div class="graph-summary" v-if="isGraphLinked">
                  <span>子图节点 {{ graphNodes.length }}</span>
                  <span>本次新增节点 {{ graphResult?.nodes_added ?? 0 }}</span>
                  <span>本次新增边 {{ graphResult?.edges_added ?? 0 }}</span>
                </div>
                <div v-if="graphNodes.length" class="linked-entities">
                  <div v-for="n in graphNodes" :key="n.id" class="entity-card">
                    <div class="entity-info">
                      <span class="entity-name">{{ n.label || n.id }}</span>
                      <el-tag size="small" type="info" effect="plain" round>{{ n.node_type }}</el-tag>
                    </div>
                  </div>
                </div>
                <el-empty v-else :description="isGraphLinked ? '挂图响应未带回节点' : '尚未关联图谱'" :image-size="60" />
              </div>
            </div>
          </el-tab-pane>
        </el-tabs>
      </div>
    </el-dialog>

    <!-- Create/Edit Dialog -->
    <el-dialog
      v-model="editVisible"
      :title="isEditing ? '编辑文档' : '新建文档'"
      width="680px"
      :close-on-click-modal="false"
      @open="handleEditOpen"
    >
      <el-form :model="editForm" :rules="formRules" ref="editFormRef" label-width="90px">
        <el-form-item label="标题" prop="title">
          <el-input v-model="editForm.title" placeholder="请输入文档标题" maxlength="200" show-word-limit />
        </el-form-item>
        <el-form-item label="分类" prop="category">
          <el-tree-select
            v-model="editForm.category"
            :data="categories"
            node-key="id"
            :props="{ label: 'name', value: 'id', children: 'children' }"
            placeholder="选择分类"
            check-strictly
            style="width: 100%"
            filterable
          />
        </el-form-item>
        <el-form-item label="标签">
          <el-select
            v-model="editForm.tags"
            multiple
            filterable
            allow-create
            default-first-option
            placeholder="输入或选择标签"
            style="width: 100%"
          >
            <el-option
              v-for="tag in tagOptions"
              :key="tag.name"
              :label="tag.name"
              :value="tag.name"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="描述">
          <el-input
            v-model="editForm.description"
            type="textarea"
            :rows="3"
            placeholder="文档简要描述"
            maxlength="500"
            show-word-limit
          />
        </el-form-item>
        <el-form-item label="内容" prop="content">
          <el-input
            v-model="editForm.content"
            type="textarea"
            :autosize="{ minRows: 8, maxRows: 20 }"
            placeholder="输入文档内容，支持 Markdown 语法..."
          />
        </el-form-item>
        <el-form-item label="版本说明" v-if="isEditing">
          <el-input
            v-model="editForm.version_note"
            placeholder="描述本次修改的内容，便于版本追踪"
          />
        </el-form-item>
        <el-form-item label="自动保存">
          <el-switch v-model="editForm.auto_save" active-text="开启" inactive-text="关闭" />
        </el-form-item>
      </el-form>
      <template #footer>
        <div class="edit-dialog-footer">
          <el-button @click="editVisible = false">取消</el-button>
          <el-button type="primary" :loading="saving" @click="submitForm">
            {{ isEditing ? '保存修改' : '创建文档' }}
          </el-button>
        </div>
      </template>
    </el-dialog>

    <!-- Version Compare Dialog -->
    <el-dialog
      v-model="compareVisible"
      title="版本对比"
      width="90%"
      top="5vh"
      :close-on-click-modal="false"
    >
      <div class="version-compare">
        <div class="compare-header">
          <div class="compare-version">
            <span class="compare-label">旧版本</span>
            <el-tag type="info" effect="dark" round>{{ compareFrom?.version }}</el-tag>
          </div>
          <div class="compare-arrow">
            <el-icon><ArrowRight /></el-icon>
          </div>
          <div class="compare-version">
            <span class="compare-label">新版本</span>
            <el-tag type="success" effect="dark" round>{{ compareTo?.version }}</el-tag>
          </div>
        </div>
        <!-- VersionDiff 只有 version / added_lines / removed_lines / unchanged 四键，没有"修改"这一类 -->
        <div class="diff-result" v-if="compareDiff">
          <div class="diff-col">
            <h4>新增 {{ compareDiff.added_lines?.length ?? 0 }} 行</h4>
            <pre v-for="(l, i) in compareDiff.added_lines" :key="'add-'+i" class="diff-line added">{{ l }}</pre>
          </div>
          <div class="diff-col">
            <h4>删除 {{ compareDiff.removed_lines?.length ?? 0 }} 行</h4>
            <pre v-for="(l, i) in compareDiff.removed_lines" :key="'rem-'+i" class="diff-line removed">{{ l }}</pre>
          </div>
          <div class="diff-col">
            <h4>未变 {{ compareDiff.unchanged ?? 0 }} 行</h4>
          </div>
        </div>
        <div class="compare-body">
          <div class="compare-pane">
            <h4>{{ compareFrom?.version }} — {{ formatTime(compareFrom?.created_at) }}</h4>
            <div class="compare-content" v-html="renderedCompareFrom"></div>
          </div>
          <div class="compare-pane diff-pane">
            <h4>{{ compareTo?.version }} — {{ formatTime(compareTo?.created_at) }}</h4>
            <div class="compare-content" v-html="renderedCompareTo"></div>
          </div>
        </div>
      </div>
      <template #footer>
        <el-button @click="compareVisible = false">关闭</el-button>
        <el-button
          type="warning"
          @click="revertVersion(selectedDoc?.id, compareFrom)"
          :disabled="!compareFrom"
        >
          <el-icon><Refresh /></el-icon> 回滚到此版本
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { ElMessage } from 'element-plus/es/components/message/index'
import { ElMessageBox } from 'element-plus/es/components/message-box/index'
import {
  Plus, Search, Refresh, View, Edit, Delete, MagicStick,
  Share, Document, Folder, CollectionTag, Clock, Check, Close,
  ArrowLeft, ArrowRight, Download, Upload, DataAnalysis,
  List, Grid, Loading, PriceTag
} from '@element-plus/icons-vue'
import * as api from '@/api'
import { useProject } from '@/composables'

// ========== State ==========
const documents = ref([])
// 分类树：由 kbGetCategories() 加载，初始为空
const categories = ref([])
// 标签：由 kbGetTags() 加载，初始为空
const tags = ref([])
const selectedDoc = ref(null)
const docVersions = ref([])
const docHistory = ref([])
const aiAnalysis = ref(null)
const entities = ref([])
// 挂图是文档级动作：graph-link 响应给 {graph_nodes[],nodes_added,edges_added,...}，没有按实体的接口
const graphNodes = ref([])
const graphResult = ref(null)
// 统计：由 kbGetStats() 加载，初始为空
const stats = ref({})

const loading = ref(false)
const error = ref('')
const saving = ref(false)
const searchQuery = ref('')
const filterCategory = ref('')
const filterStatus = ref('')
const filterTag = ref('')
const filterDateRange = ref(null)
const viewMode = ref('list')
const selectedDocs = ref([])

const detailVisible = ref(false)
const detailTab = ref('content')
const detailMode = ref('view')
const editVisible = ref(false)
const isEditing = ref(false)
const compareVisible = ref(false)
const compareFrom = ref(null)
const compareTo = ref(null)
const compareDiff = ref(null)

const categoryTreeRef = ref(null)
const editFormRef = ref(null)


const tagOptions = computed(() => tags.value)
// 挂图与否只有一个真源：KbDocument.status ∈ {draft, analyzed, linked}
const isGraphLinked = computed(() => selectedDoc.value?.status === 'linked')

// ========== Mapping ==========

// editForm 的字段与 CreateDocReq 一一对应（title/content/category/tags）；
// description 与 auto_save 是 wire 上没有落点的表单装饰（已登记为待裁决），version_note 走独立的建版端点。
const editForm = reactive({
  id: null,
  title: '',
  content: '',
  category: '',
  tags: [],
  description: '',
  auto_save: false,
  version_note: ''
})

const formRules = {
  title: [
    { required: true, message: '请输入文档标题', trigger: 'blur' },
    { min: 2, max: 200, message: '标题长度在 2 到 200 个字符', trigger: 'blur' }
  ],
  content: [{ required: true, message: '请输入文档内容', trigger: 'blur' }]
}

// ========== Computed ==========
// 四张卡只挂两个真源：/kb/stats 给 {documents,categories,tags,storage_bytes,graph_nodes,graph_edges}
// （handler 里再补两个图计数），"版本总数"与"已分析"没有服务端计数键 ⇒ 前者撤下，后者由列表行的 status 本地数。
// icon 传组件本身：模板是 <component :is="s.icon">，本项目无 unplugin-auto-import，字符串名解析不出图标。
const statCards = computed(() => [
  {
    label: '文档总数',
    value: stats.value.documents ?? 0,
    icon: Document,
    color: 'var(--on-cat-1)',
    bg: 'var(--cat-1-fill)'
  },
  {
    label: '分类数',
    value: stats.value.categories ?? 0,
    icon: Folder,
    color: 'var(--on-cat-2)',
    bg: 'var(--cat-2-fill)'
  },
  {
    label: '标签数',
    value: stats.value.tags ?? 0,
    icon: PriceTag,
    color: 'var(--on-cat-3)',
    bg: 'var(--cat-3-fill)'
  },
  {
    label: '已分析',
    value: documents.value.filter((d) => isAiAnalyzed(d.status)).length,
    icon: MagicStick,
    color: 'var(--on-cat-4)',
    bg: 'var(--cat-4-fill)'
  }
])

const filteredDocuments = computed(() => {
  let result = [...documents.value]
  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase()
    result = result.filter(
      d =>
        d.title?.toLowerCase().includes(q) ||
        d.summary?.toLowerCase().includes(q) ||
        d.tags?.some(t => t.toLowerCase().includes(q))
    )
  }
  if (filterStatus.value) result = result.filter(d => d.status === filterStatus.value)
  if (filterTag.value) result = result.filter(d => d.tags?.includes(filterTag.value))
  if (filterCategory.value) result = result.filter(d => d.category === filterCategory.value)
  if (filterDateRange.value?.length === 2) {
    const [start, end] = filterDateRange.value
    result = result.filter(d => {
      const t = new Date(d.updated_at).getTime()
      return t >= start.getTime() && t <= end.getTime()
    })
  }
  return result.sort((a, b) => new Date(b.updated_at) - new Date(a.updated_at))
})

const renderedContent = computed(() => {
  if (!selectedDoc.value) return ''
  return simpleMarkdownRender(selectedDoc.value.content || '')
})

const renderedCompareFrom = computed(() => {
  if (!compareFrom.value) return ''
  return simpleMarkdownRender(compareFrom.value.content || '')
})

const renderedCompareTo = computed(() => {
  if (!compareTo.value) return ''
  return simpleMarkdownRender(compareTo.value.content || '')
})

// ========== Markdown Renderer ==========
// 安全加固：先 HTML-escape 用户文本再套 markdown，且链接仅允许安全协议，杜绝 v-html 存储型 XSS。
function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}
function safeUrl(u) {
  const trimmed = (u || '').trim()
  if (!/^(https?:|mailto:|tel:|#)/i.test(trimmed)) return ''
  return trimmed.replace(/&/g, '&amp;').replace(/"/g, '&quot;')
}
function simpleMarkdownRender(text) {
  if (!text) return ''
  let html = escapeHtml(text)
    .replace(/^### (.*$)/gm, '<h3>$1</h3>')
    .replace(/^## (.*$)/gm, '<h2>$1</h2>')
    .replace(/^# (.*$)/gm, '<h1>$1</h1>')
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.*?)\*/g, '<em>$1</em>')
    .replace(/`(.*?)`/g, '<code class="inline-code">$1</code>')
    .replace(/\n\n/g, '</p><p>')
    .replace(/\n/g, '<br/>')
    .replace(/\[(.*?)\]\((.*?)\)/g, (m, label, url) => {
      const href = safeUrl(url)
      if (!href) return label // 非法协议：仅显示文本，不渲染为链接
      return `<a href="${href}" target="_blank" rel="noopener noreferrer">${label}</a>`
    })
  return `<p>${html}</p>`
}

// ========== Methods ==========
function getTagSize(count) {
  const min = 12, max = 20
  const maxCount = Math.max(...tags.value.map(t => t.count))
  return min + (count / maxCount) * (max - min)
}





function truncateText(text, max) {
  if (!text) return ''
  return text.length > max ? text.slice(0, max) + '...' : text
}

// 时间口径不再自写：这一份与 utils/knowledgeBase.utils.js、utils/message.utils.js 里的两份逐字符相同
import { formatDateTime as formatTime } from '@/utils'
import { KB_STATUSES, getCategoryLabel, getCategoryTagType, getStatusType, getStatusLabel, isAiAnalyzed, mapDoc } from '@/utils'
import { unwrap, unwrapList } from '@/modules/_kernel/envelope'

function handleSearch() {
  fetchDocuments()
}

function resetFilters() {
  searchQuery.value = ''
  filterStatus.value = ''
  filterTag.value = ''
  filterCategory.value = ''
  filterDateRange.value = null
  fetchDocuments()
}

function handleCategoryClick(node) {
  // 列表行的 category 存的是 cat-* id（/kb/categories 的 id 键），按中文名筛恒空
  filterCategory.value = node?.id || ''
  fetchDocuments()
}

function handleTagClick(tag) {
  filterTag.value = filterTag.value === tag.name ? '' : tag.name
  fetchDocuments()
}

function toggleSelectDoc(doc) {
  const idx = selectedDocs.value.indexOf(doc.id)
  if (idx >= 0) selectedDocs.value.splice(idx, 1)
  else selectedDocs.value.push(doc.id)
}

async function fetchDocuments() {
  loading.value = true
  error.value = ''
  try {
    const params = {}
    if (searchQuery.value) params.q = searchQuery.value
    if (filterStatus.value) params.status = filterStatus.value
    if (filterTag.value) params.tag = filterTag.value
    if (filterCategory.value) params.category = filterCategory.value
    if (filterDateRange.value?.length === 2) {
      params.start_date = filterDateRange.value[0].toISOString()
      params.end_date = filterDateRange.value[1].toISOString()
    }
    const data = await api.kbListDocuments(params)
    documents.value = unwrapList(unwrap(data), 'items').map(mapDoc)
  } catch (e) {
    documents.value = []
    error.value = e?.message || '文档加载失败'
    ElMessage.error(error.value)
  } finally {
    loading.value = false
  }
}


async function fetchCategories() {
  try {
    const data = await api.kbGetCategories()
    if (Array.isArray(data) && data.length) {
      categories.value = data
    }
  } catch (e) {
    categories.value = []
    ElMessage.error(e?.message || '分类加载失败')
  }
}

async function fetchTags() {
  try {
    const data = await api.kbGetTags()
    if (Array.isArray(data) && data.length) {
      tags.value = data
    }
  } catch (e) {
    tags.value = []
    ElMessage.error(e?.message || '标签加载失败')
  }
}

async function fetchStats() {
  try {
    const data = await api.kbGetStats()
    if (data) {
      stats.value = data
    }
  } catch (e) {
    ElMessage.error(e?.message || '统计加载失败')
    stats.value = {
      documents: documents.value.length,
      categories: categories.value.length,
      tags: tags.value.length
    }
  }
}

// 后端待提供: 文档选中后的详情预加载/阅读记录端点（当前为空实现）
async function selectDocument(doc) {
  // In grid/list view, single click selects
}

async function viewDocument(doc) {
  if (!doc) return
  selectedDoc.value = doc
  detailTab.value = 'content'
  detailMode.value = 'view'
  detailVisible.value = true
  // 换文档要把只在动作响应里出现的派生态清掉，否则子图/差异是上一篇的
  graphNodes.value = []
  graphResult.value = null
  compareDiff.value = null
  try {
    const fullDoc = await api.kbGetDocument(doc.id)
    if (fullDoc) {
      selectedDoc.value = mapDoc(fullDoc)
    }
  } catch (e) {
    ElMessage.error(e?.message || '文档详情加载失败')
  }
  fetchVersions(doc.id)
  fetchHistory(doc.id)
  if (doc.ai_analyzed) {
    loadAiAnalysis(doc.id)
  }
}

function closeDetail() {
  detailVisible.value = false
  selectedDoc.value = null
  docVersions.value = []
  docHistory.value = []
  aiAnalysis.value = null
  entities.value = []
  graphNodes.value = []
  graphResult.value = null
  compareDiff.value = null
}

function handleDetailTabChange(tab) {
  if (tab === 'analysis' && selectedDoc.value?.ai_analyzed && !aiAnalysis.value) {
    loadAiAnalysis(selectedDoc.value.id)
  }
  if (tab === 'history' && docHistory.value.length === 0) {
    fetchHistory(selectedDoc.value?.id)
  }
  if (tab === 'versions' && docVersions.value.length === 0) {
    fetchVersions(selectedDoc.value?.id)
  }
}

function openCreateDialog() {
  isEditing.value = false
  Object.assign(editForm, {
    id: null,
    title: '',
    content: '',
    category: '',
    tags: [],
    description: '',
    auto_save: false,
    version_note: ''
  })
  editVisible.value = true
}

async function openEditDialog(doc) {
  if (!doc) return
  try {
    const full = await api.kbGetDocument(doc.id)
    if (!full?.id || !full?.current_version) throw new Error('文档详情缺失，请刷新后重试')
    doc = mapDoc(full)
  } catch (e) {
    ElMessage.error(e?.message || '读取文档失败')
    return
  }
  isEditing.value = true
  Object.assign(editForm, {
    id: doc.id,
    current_version: doc.current_version,
    title: doc.title,
    content: doc.content || '',
    category: doc.category,
    tags: doc.tags || [],
    // wire 上没有 description 键，编辑时不继承上一篇的残留
    description: '',
    auto_save: false,
    version_note: ''
  })
  editVisible.value = true
}

function handleEditOpen() {
  // Dialog opened, form initialized
}

async function submitForm() {
  if (!editFormRef.value) return
  try {
    await editFormRef.value.validate()
  } catch {
    return
  }
  saving.value = true
  try {
    if (isEditing.value) {
      await saveDocument(editForm)
      ElMessage.success('文档保存成功')
    } else {
      await createDocument(editForm)
      ElMessage.success('文档创建成功')
    }
    editVisible.value = false
    fetchDocuments()
    fetchStats()
    // 详情里挂着的正是这一篇时，正文与版本列表都要重取：后端更新已原子归档旧正文并推进版本
    if (isEditing.value && selectedDoc.value?.id === editForm.id) {
      const full = await api.kbGetDocument(editForm.id).catch(() => null)
      if (full) selectedDoc.value = mapDoc(full)
      fetchVersions(editForm.id)
      fetchHistory(editForm.id)
    }
  } catch (e) {
    ElMessage.error((e?.message || '操作失败'))
  } finally {
    saving.value = false
  }
}

// CreateDocReq 只认 title/content/category/tags 四键（category 取 cat-* id），响应是 {id,status,document}
async function createDocument(data) {
  const payload = {
    title: data.title,
    content: data.content,
    category: data.category,
    tags: data.tags
  }
  try {
    const result = await api.kbCreateDocument(payload)
    const created = mapDoc(result?.document)
    documents.value.unshift(created)
    return created
  } catch (e) {
    ElMessage.error(e?.message || '创建失败')
    throw e
  }
}

// 正文、版本备注与预期版本统一提交；由后端原子归档上一版。
async function saveDocument(data) {
  try {
    const result = await api.kbSaveDocumentEdit(data)
    const updated = mapDoc(result?.document)
    const idx = documents.value.findIndex(d => d.id === data.id)
    if (idx !== -1) {
      documents.value[idx] = { ...documents.value[idx], ...updated }
    }
    return updated
  } catch (e) {
    ElMessage.error(e?.message || '保存失败')
    throw e
  }
}

function saveFromDetail() {
  if (!selectedDoc.value) return
  openEditDialog(selectedDoc.value)
}

async function deleteDocument(id) {
  if (!id) return
  try {
    await ElMessageBox.confirm(
      '确定删除此文档吗？此操作不可恢复。',
      '删除确认',
      { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' }
    )
    await api.kbDeleteDocument(id)
    documents.value = documents.value.filter(d => d.id !== id)
    selectedDocs.value = selectedDocs.value.filter(d => d !== id)
    ElMessage.success('文档已删除')
    fetchStats()
  } catch (e) {
    if (e !== 'cancel' && e !== 'close') {
      ElMessage.error(e?.message || '删除失败')
    }
  }
}

// POST /kb/documents/:id/analyze 的响应就是 AnalysisResult{doc_id,status,entities,relations,keywords,tags,summary,chunks,...}；
// 落库进文档的只有 status/summary/entities/relations，所以这份结果只在内存里用，不再找 aiAnalysis 这种不存在的键。
async function analyzeDocument(id) {
  if (!id) return
  ElMessage.info('AI 分析已开始，请稍候...')
  try {
    const result = await api.kbAnalyzeDocument(id)
    const status = result?.status || 'analyzed'
    aiAnalysis.value = result || null
    entities.value = result?.entities || []
    const doc = documents.value.find(d => d.id === id)
    if (doc) {
      doc.status = status
      doc.ai_analyzed = isAiAnalyzed(status)
    }
    // 详情里已有这一篇时只改状态：拿列表行（没有 content）整份覆盖会把正文清空
    if (selectedDoc.value?.id === id) {
      selectedDoc.value = { ...selectedDoc.value, status, ai_analyzed: isAiAnalyzed(status) }
    }
    ElMessage.success('AI 分析完成')
    fetchStats()
  } catch (e) {
    ElMessage.error(e?.message || 'AI 分析失败')
  }
}

// BatchAnalyzeReq 只认 ids；响应是 {status,analyzed,failed[]}，除失败清单外不给逐篇结果
async function batchAnalyze() {
  if (!selectedDocs.value.length) return
  ElMessage.info(`开始批量分析 ${selectedDocs.value.length} 篇文档...`)
  try {
    const result = await api.kbBatchAnalyze({ ids: selectedDocs.value })
    const analyzed = result?.analyzed ?? 0
    const failed = result?.failed || []
    if (failed.length) ElMessage.warning(`批量分析：成功 ${analyzed} 篇，失败 ${failed.length} 篇`)
    else ElMessage.success(`批量分析完成：成功 ${analyzed} 篇`)
    selectedDocs.value = []
    fetchDocuments()
    fetchStats()
  } catch (e) {
    ElMessage.error(e?.message || '批量分析失败')
  }
}

// 重开分析页签时只能回到文档本身：KbDocument 上没有 aiAnalysis / linked_entities 两个键
async function loadAiAnalysis(docId) {
  try {
    const data = await api.kbGetDocument(docId)
    aiAnalysis.value = data || null
    entities.value = data?.entities || []
  } catch (e) {
    aiAnalysis.value = null
    entities.value = []
    ElMessage.error(e?.message || 'AI 分析加载失败')
  }
}

async function fetchVersions(docId) {
  try {
    const data = await api.kbGetVersions(docId)
    docVersions.value = unwrapList(unwrap(data), 'versions')
  } catch (e) {
    docVersions.value = []
    ElMessage.error(e?.message || '版本记录加载失败')
  }
}

async function openCompare(fromVer, toVer) {
  compareFrom.value = fromVer
  compareTo.value = toVer
  compareDiff.value = null
  compareVisible.value = true
  const docId = selectedDoc.value?.id
  if (!docId || !fromVer || !toVer) return
  // CompareReq 只认 {v1,v2}，响应是 {doc_id,diff:{version,added_lines,removed_lines,unchanged}}
  try {
    const data = await api.kbCompareVersions(docId, { v1: fromVer.version, v2: toVer.version })
    compareDiff.value = data?.diff || null
  } catch (e) {
    ElMessage.error(e?.message || '版本对比加载失败')
  }
}

function viewVersion(ver) {
  const idx = docVersions.value.indexOf(ver)
  // 列表按版本倒序：ver 的"上一版"在它之后
  openCompare(docVersions.value[idx + 1] || docVersions.value[0], ver)
}

function compareWithPrevious(idx) {
  if (idx >= docVersions.value.length - 1) {
    ElMessage.info('已是最早版本，无更早版本可对比')
    return
  }
  openCompare(docVersions.value[idx + 1], docVersions.value[idx])
}

// RevertReq 只认 {version}；版本串自带 "v" 前缀，不再补一次
async function revertVersion(docId, version) {
  if (!docId || !version?.version) return
  try {
    await ElMessageBox.confirm(
      `确定回滚到版本 ${version.version} 吗？当前版本将保存为新版本。`,
      '回滚确认',
      { confirmButtonText: '回滚', cancelButtonText: '取消', type: 'warning' }
    )
    const res = await api.kbRevertVersion(docId, { version: version.version })
    ElMessage.success(`已回滚到 ${version.version}，当前版本为 ${res?.version || '?'}`)
    // 回滚改的是正文，详情与列表都要重取
    const full = await api.kbGetDocument(docId).catch(() => null)
    if (full) selectedDoc.value = mapDoc(full)
    fetchVersions(docId)
    fetchHistory(docId)
    fetchDocuments()
  } catch (e) {
    if (e !== 'cancel' && e !== 'close') {
      ElMessage.error(e?.message || '回滚失败')
    }
  }
}

async function fetchHistory(docId) {
  try {
    const data = await api.kbGetDocHistory(docId)
    docHistory.value = unwrapList(unwrap(data), 'history')
  } catch (e) {
    docHistory.value = []
    ElMessage.error(e?.message || '变更历史加载失败')
  }
}

// 挂图端点不收请求体：POST/DELETE /kb/documents/:id/graph-link 都是整篇文档级的子图操作
async function linkGraph(docId) {
  if (!docId) return
  try {
    const data = await api.kbGraphLink(docId)
    graphResult.value = data || null
    graphNodes.value = data?.graph_nodes || []
    if (selectedDoc.value?.id === docId) {
      selectedDoc.value = { ...selectedDoc.value, status: 'linked', ai_analyzed: true }
    }
    const doc = documents.value.find(d => d.id === docId)
    if (doc) {
      doc.status = 'linked'
      doc.ai_analyzed = true
    }
    ElMessage.success(`已挂图：新增节点 ${data?.nodes_added ?? 0}，新增边 ${data?.edges_added ?? 0}`)
    fetchStats()
  } catch (e) {
    ElMessage.error(e?.message || '关联失败')
  }
}

async function unlinkGraph(docId) {
  if (!docId) return
  try {
    await ElMessageBox.confirm(
      '解除关联会移除该文档在图谱中的整个子图（含实体节点与边），确定继续吗？',
      '解除关联确认',
      { confirmButtonText: '解除关联', cancelButtonText: '取消', type: 'warning' }
    )
    const data = await api.kbGraphUnlink(docId)
    graphNodes.value = []
    graphResult.value = null
    ElMessage.success(`已解除关联：移除节点 ${data?.nodes_removed ?? 0}`)
    // 后端的 unlink 不改 KbDocument.status（仍写 linked），这里只能先把界面拉回真实样子
    if (selectedDoc.value?.id === docId) {
      selectedDoc.value = { ...selectedDoc.value, status: 'analyzed' }
    }
    fetchDocuments()
  } catch (e) {
    if (e !== 'cancel' && e !== 'close') {
      ElMessage.error(e?.message || '解除关联失败')
    }
  }
}

// Keyboard shortcuts
function handleKeydown(e) {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    if (editVisible.value) {
      e.preventDefault()
      submitForm()
    }
  }
  if (e.key === 'Escape') {
    if (compareVisible.value) compareVisible.value = false
    else if (editVisible.value) editVisible.value = false
    else if (detailVisible.value) closeDetail()
  }
}

// ========== Lifecycle ==========
onMounted(async () => {
  window.addEventListener('keydown', handleKeydown)
  fetchDocuments()
  fetchCategories()
  fetchTags()
  fetchStats()
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown)
})

watch(detailVisible, (v) => {
  if (!v) {
    closeDetail()
  }
})

// ===== 璇玑：以项目为核心的联动 =====
{
  const { onChange: _onProjectChange, ensureProjectContext: _ensureProject } = useProject()
  let _offPj = null
  let _loaded = false
  onMounted(async () => {
    _offPj = _onProjectChange(async () => { fetchDocuments() })
    await _ensureProject().catch(() => {})
    if (!_loaded) {
      _loaded = true
      fetchDocuments()
    }
  })
  const _ob$ = onBeforeUnmount == null ? null : onBeforeUnmount(() => { _offPj && _offPj() })
  // 若脚本未引入 onBeforeUnmount，退化为 window beforeunload 兜底（页面关闭）
  if (typeof onBeforeUnmount === 'undefined') {
    // 不操作：Vue 路由离开时组件 destroy，本作用域已销毁
  }
}
</script>

<style scoped>
.kb-view {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-height: 100%;
  color: var(--text-1);
}

/* ===== Header ===== */
.kb-header {
  position: relative;
  overflow: hidden;
  border-radius: 20px;
  background: linear-gradient(135deg, #1e1b4b 0%, #312e81 25%, #1e3a5f 55%, #0f4c5c 85%, #0c4a6e 100%);
  padding: 28px 32px;
  color: #fff;
}

.header-bg {
  position: absolute;
  inset: 0;
  pointer-events: none;
  overflow: hidden;
}

.bg-orb {
  position: absolute;
  border-radius: 50%;
  filter: blur(60px);
  opacity: 0.35;
}

.orb-1 {
  width: 380px; height: 380px;
  background: radial-gradient(circle, #818cf8, transparent);
  top: -80px; right: -60px;
}

.orb-2 {
  width: 300px; height: 300px;
  background: radial-gradient(circle, #22d3ee, transparent);
  bottom: -60px; left: 30%;
  opacity: 0.25;
}

.orb-3 {
  width: 240px; height: 240px;
  background: radial-gradient(circle, #a78bfa, transparent);
  top: 20%; left: -40px;
  opacity: 0.2;
}

.header-content {
  position: relative;
  z-index: 1;
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 24px;
  flex-wrap: wrap;
}

.eyebrow {
  font-size: 11px;
  letter-spacing: 2.5px;
  color: rgba(255, 255, 255, 0.65);
  text-transform: uppercase;
  margin-bottom: 6px;
}

.kb-header .page-title {
  font-size: 26px;
  font-weight: 800;
  color: #fff;
  margin: 0;
  text-shadow: 0 2px 16px rgba(0, 0, 0, 0.3);
}

.kb-header .page-subtitle {
  font-size: 14px;
  color: rgba(255, 255, 255, 0.75);
  margin-top: 6px;
}

.header-right {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
}

.stat-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
  background: rgba(255, 255, 255, 0.08);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 14px;
  min-width: 140px;
  transition: all 0.25s ease;
}

.stat-card:hover {
  background: rgba(255, 255, 255, 0.14);
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
}

.stat-icon {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-size: 22px;
  flex-shrink: 0;
}

.stat-info {
  display: flex;
  flex-direction: column;
}

.stat-value {
  font-size: 22px;
  font-weight: 800;
  color: #fff;
  line-height: 1.1;
}

.stat-label {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.7);
  margin-top: 2px;
}

/* ===== Main Layout ===== */
.kb-main {
  display: grid;
  grid-template-columns: 38% 62%;
  gap: 16px;
  align-items: start;
}

@media (max-width: 1100px) {
  .kb-main {
    grid-template-columns: 1fr;
  }
}

/* ===== Left Panel ===== */
.kb-left {
  display: flex;
  flex-direction: column;
  gap: 14px;
  position: sticky;
  top: 0;
}

.panel {
  background: var(--bg-panel);
  border-radius: 14px;
  border: 1px solid var(--border-light);
  box-shadow: var(--shadow-sm);
  padding: 16px 18px;
}

.search-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.search-box :deep(.el-input__wrapper) {
  border-radius: 10px;
}

.filter-row {
  display: flex;
  gap: 8px;
}

.filter-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 2px;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-1);
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0 0 10px;
}

.section-title .el-icon {
  color: var(--brand);
}

/* Category Tree */
.category-tree {
  background: transparent;
}

.category-tree :deep(.el-tree-node__content) {
  height: 34px;
  border-radius: 8px;
}

.category-tree :deep(.el-tree-node__content:hover) {
  background: var(--bg-page);
}

.category-tree :deep(.el-tree-node.is-current > .el-tree-node__content) {
  background: var(--brand-soft);
}

.tree-node {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  font-size: 13px;
}

.tree-count {
  font-size: 11px;
  background: var(--bg-page);
  color: var(--text-3);
  padding: 1px 7px;
  border-radius: 10px;
  font-weight: 600;
}

/* Tag Cloud */
.tag-panel {
  display: flex;
  flex-direction: column;
}

.tag-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  max-height: 220px;
  overflow: auto;
}

.tag-item {
  cursor: pointer;
  transition: all 0.2s ease;
  font-weight: 500;
}

.tag-item:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.tag-item.active {
  background: var(--brand-fill) !important;
  color: var(--on-brand) !important;
  border-color: var(--brand-fill) !important;
}

.tag-count {
  font-size: 10px;
  opacity: 0.7;
  margin-left: 4px;
}

/* ===== Right Panel ===== */
.kb-right {
  display: flex;
  flex-direction: column;
  gap: 14px;
  min-width: 0;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 14px;
}

.toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.result-count {
  font-size: 13px;
  color: var(--text-3);
}

.badge-count {
  background: var(--danger-fill);
  color: var(--on-danger);
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 10px;
  margin-left: 4px;
}

/* Loading & Empty States */
.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  gap: 12px;
  color: var(--text-3);
}

.loading-spin {
  font-size: 28px;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Document Grid */
.doc-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 14px;
}

@media (max-width: 900px) {
  .doc-grid {
    grid-template-columns: 1fr;
  }
}

.doc-card {
  background: var(--bg-panel);
  border: 1px solid var(--border-light);
  border-radius: 14px;
  padding: 16px;
  cursor: pointer;
  transition: all 0.25s ease;
  position: relative;
  overflow: hidden;
}

.doc-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: linear-gradient(90deg, var(--brand), var(--accent));
  opacity: 0;
  transition: opacity 0.25s ease;
}

.doc-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 12px 32px rgba(15, 23, 42, 0.1);
  border-color: var(--brand);
}

.doc-card:hover::before {
  opacity: 1;
}

.doc-card.selected {
  border-color: var(--brand);
  background: var(--brand-soft);
}

.doc-card-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 8px;
}

.doc-card-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-1);
  margin: 0 0 6px;
  line-height: 1.3;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.doc-card-desc {
  font-size: 13px;
  color: var(--text-2);
  margin: 0 0 10px;
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.doc-card-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-bottom: 10px;
}

.more-tags {
  font-size: 11px;
  color: var(--text-3);
}

.doc-card-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 12px;
  color: var(--text-3);
  padding-top: 10px;
  border-top: 1px solid var(--border-light);
}

.meta-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.meta-item.ai-done {
  color: var(--success);
}

.doc-card-actions {
  display: flex;
  gap: 2px;
  margin-top: 10px;
  opacity: 0;
  transition: opacity 0.25s ease;
}

.doc-card:hover .doc-card-actions {
  opacity: 1;
}

/* Document List */
.doc-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.doc-row {
  display: grid;
  grid-template-columns: auto 1fr auto auto;
  gap: 14px;
  align-items: center;
  padding: 14px 16px;
  background: var(--bg-panel);
  border: 1px solid var(--border-light);
  border-radius: 12px;
  transition: all 0.25s ease;
}

.doc-row:hover {
  border-color: var(--brand);
  box-shadow: 0 4px 20px rgba(15, 23, 42, 0.06);
}

.doc-row.selected {
  border-color: var(--brand);
  background: var(--brand-soft);
}

.row-checkbox {
  display: flex;
  align-items: center;
}

.row-main {
  cursor: pointer;
  min-width: 0;
}

.row-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.row-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--text-1);
  margin: 0;
}

.row-desc {
  font-size: 13px;
  color: var(--text-2);
  margin: 0 0 6px;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.row-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.row-info {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 12px;
  color: var(--text-3);
}

.info-item {
  display: flex;
  align-items: center;
  gap: 4px;
  white-space: nowrap;
}

.row-actions {
  display: flex;
  gap: 6px;
}

/* ===== Detail Modal ===== */
.detail-dialog :deep(.el-dialog) {
  margin: 0;
  padding: 0;
  border-radius: 0;
  height: 100vh;
  width: 100% !important;
  max-width: 100%;
  background: var(--bg-page);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.detail-dialog :deep(.el-dialog__header) {
  display: none;
}

.detail-dialog :deep(.el-dialog__body) {
  padding: 0;
  flex: 1;
  overflow: auto;
}

.detail-container {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.detail-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 20px 28px;
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border);
  gap: 20px;
}

.detail-title-area {
  flex: 1;
}

.detail-title {
  font-size: 22px;
  font-weight: 800;
  color: var(--text-1);
  margin: 0 0 8px;
}

.detail-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-2);
}

.meta-sep {
  color: var(--text-3);
}

.meta-text {
  color: var(--text-2);
}

.detail-actions {
  display: flex;
  gap: 8px;
}

.detail-tabs {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 0 28px;
  overflow: auto;
}

.detail-tabs :deep(.el-tabs__content) {
  flex: 1;
  padding: 16px 0;
}

.tab-content-wrapper {
  background: var(--bg-panel);
  border-radius: 14px;
  padding: 20px 24px;
  min-height: 300px;
  border: 1px solid var(--border-light);
}

.content-view {
  line-height: 1.8;
}

.content-markdown h1 { font-size: 22px; font-weight: 800; margin: 0 0 16px; }
.content-markdown h2 { font-size: 18px; font-weight: 700; margin: 0 0 12px; }
.content-markdown h3 { font-size: 16px; font-weight: 600; margin: 0 0 10px; }
.content-markdown p { margin: 0 0 12px; }
.content-markdown .inline-code {
  background: var(--bg-page);
  padding: 1px 6px;
  border-radius: 4px;
  font-family: 'Menlo', 'Consolas', monospace;
  font-size: 0.9em;
}

.content-edit {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.edit-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.view-toggle {
  display: flex;
  justify-content: flex-end;
  margin-top: 12px;
}

/* Version Timeline */
.version-timeline {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.version-item {
  display: flex;
  gap: 14px;
  align-items: flex-start;
}

.version-badge {
  width: 50px;
  height: 50px;
  border-radius: 14px;
  background: linear-gradient(135deg, var(--brand-fill), var(--accent-fill));
  color: var(--on-brand);
  display: grid;
  place-items: center;
  font-weight: 800;
  font-size: 14px;
  flex-shrink: 0;
}

.version-info {
  flex: 1;
  padding: 12px 14px;
  background: var(--bg-page);
  border-radius: 10px;
  border: 1px solid var(--border-light);
}

.version-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}

.version-label {
  font-weight: 700;
  color: var(--text-1);
}

.version-time {
  font-size: 12px;
  color: var(--text-3);
}

.version-note {
  font-size: 13px;
  color: var(--text-2);
  margin: 0 0 8px;
}

.version-actions {
  display: flex;
  gap: 6px;
}

/* AI Analysis */
.analysis-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.analysis-section {
  padding: 14px 16px;
  background: var(--bg-page);
  border-radius: 10px;
  border: 1px solid var(--border-light);
}

.analysis-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-1);
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0 0 10px;
}

.analysis-title .el-icon {
  color: var(--brand);
}

.analysis-text {
  font-size: 13px;
  color: var(--text-2);
  line-height: 1.7;
  margin: 0;
}

.analysis-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}

@media (max-width: 900px) {
  .analysis-grid {
    grid-template-columns: 1fr;
  }
}

.keyword-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.suggestion-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.suggestion-item {
  display: grid;
  grid-template-columns: 80px 1fr 50px;
  gap: 10px;
  align-items: center;
  font-size: 13px;
}

.sug-name {
  font-weight: 600;
  color: var(--text-1);
}

.sug-conf {
  font-size: 12px;
  color: var(--text-3);
  text-align: right;
}

.tag-suggestions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.sug-conf-small {
  font-size: 10px;
  opacity: 0.7;
  margin-left: 2px;
}

.entity-table {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.entity-row {
  display: grid;
  grid-template-columns: 1fr 100px 120px;
  gap: 12px;
  align-items: center;
  padding: 8px 12px;
  background: var(--bg-card);
  border-radius: 8px;
  font-size: 13px;
}

.entity-row.entity-header {
  background: transparent;
  font-weight: 600;
  color: var(--text-2);
  font-size: 12px;
}

.entity-type {
  color: var(--brand);
  font-weight: 600;
}

/* History Timeline */
.history-timeline {
  display: flex;
  flex-direction: column;
  gap: 0;
  position: relative;
  padding-left: 20px;
}

.history-timeline::before {
  content: '';
  position: absolute;
  left: 6px;
  top: 8px;
  bottom: 8px;
  width: 2px;
  background: var(--border);
}

.history-item {
  position: relative;
  padding-bottom: 18px;
}

.history-dot {
  position: absolute;
  left: -20px;
  top: 4px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--brand-fill); color: var(--on-brand);
  border: 3px solid var(--on-brand);
  box-shadow: 0 0 0 2px var(--brand);
}

.history-dot.update { background: var(--brand-fill); color: var(--on-brand); box-shadow: 0 0 0 2px var(--brand-soft); }
.history-dot.create { background: var(--success); box-shadow: 0 0 0 2px var(--success); }
.history-dot.analyze { background: var(--warning); box-shadow: 0 0 0 2px var(--warning); }
.history-dot.delete { background: var(--danger); box-shadow: 0 0 0 2px var(--danger); }
.history-dot.link { background: var(--violet); box-shadow: 0 0 0 2px var(--violet); }

.history-content {
  padding: 10px 14px;
  background: var(--bg-page);
  border-radius: 10px;
  border: 1px solid var(--border-light);
}

.history-action {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
}

.action-label {
  font-weight: 700;
  color: var(--text-1);
}

.history-time {
  font-size: 12px;
  color: var(--text-3);
}

.history-user {
  font-size: 12px;
  color: var(--text-2);
  margin-top: 4px;
}

.history-detail {
  font-size: 13px;
  color: var(--text-2);
  margin-top: 4px;
}

/* Graph Entities */
.graph-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 14px;
}

.graph-header h4 {
  margin: 0;
  font-size: 15px;
  font-weight: 700;
}

.linked-entities {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.entity-card {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px;
  background: var(--bg-page);
  border-radius: 10px;
  border: 1px solid var(--border-light);
}

.entity-info {
  display: flex;
  align-items: center;
  gap: 10px;
}

.entity-name {
  font-weight: 600;
  font-size: 14px;
}

/* Link Dialog */
.link-dialog-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.entity-search-results {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 300px;
  overflow: auto;
}

.search-entity-item {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 10px;
  align-items: center;
  padding: 10px 14px;
  background: var(--bg-page);
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s;
}

.search-entity-item:hover {
  background: var(--brand-soft);
}

.no-results {
  text-align: center;
  color: var(--text-3);
  padding: 20px;
  font-size: 13px;
}

/* Version Compare */
.version-compare {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.compare-header {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
}

.compare-version {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.compare-label {
  font-size: 12px;
  color: var(--text-3);
}

.compare-arrow {
  font-size: 24px;
  color: var(--brand);
}

.compare-body {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}

.compare-pane {
  border: 1px solid var(--border-light);
  border-radius: 10px;
  overflow: hidden;
}

.compare-pane h4 {
  margin: 0;
  padding: 10px 14px;
  background: var(--bg-page);
  font-size: 13px;
  font-weight: 600;
  border-bottom: 1px solid var(--border-light);
}

.compare-content {
  padding: 14px;
  max-height: 400px;
  overflow: auto;
  font-size: 13px;
  line-height: 1.7;
}

.diff-pane {
  border-color: var(--brand);
}

.diff-result {
  display: flex;
  gap: 12px;
  padding: 8px 0 12px;
}

.diff-col {
  flex: 1;
  min-width: 0;
}

.diff-col h4 {
  margin: 0 0 6px;
  font-size: 13px;
  color: var(--text-2);
}

.diff-line {
  margin: 0 0 4px;
  padding: 4px 8px;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  border-radius: 4px;
}

/* 淡染底在四皮下与卡面只差 1.04–1.24:1，色块必须靠描边才认得出边界 */
.diff-line.added { background: var(--success-dim); border: 1px solid var(--success); }
.diff-line.removed { background: var(--danger-dim); border: 1px solid var(--danger); }

/* Edit Dialog */
.edit-dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

/* Dialog Dark Theme */
:deep(.el-dialog) {
  border-radius: 16px;
  overflow: hidden;
}

:deep(.el-dialog__header) {
  background: var(--brand-50);
  margin-right: 0;
  padding: 16px 20px;
}

:deep(.el-dialog__title) {
  color: var(--brand);
  font-weight: 700;
}

:deep(.el-dialog__body) {
  padding: 20px;
}

:deep(.el-dialog__footer) {
  padding: 14px 20px;
  background: var(--bg-panel-2);
}

/* Responsive */
@media (max-width: 768px) {
  .header-right {
    grid-template-columns: 1fr 1fr;
  }

  .stat-card {
    min-width: 120px;
  }

  .detail-header {
    flex-direction: column;
  }

  .compare-body {
    grid-template-columns: 1fr;
  }
}

/* Scrollbar */
.kb-view ::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

.kb-view ::-webkit-scrollbar-thumb {
  background: var(--bg-tertiary);
  border-radius: 3px;
}

.kb-view ::-webkit-scrollbar-thumb:hover {
  background: var(--text-3);
}

.kb-view ::-webkit-scrollbar-track {
  background: transparent;
}

/* Animation */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease, transform 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(10px);
}
</style>