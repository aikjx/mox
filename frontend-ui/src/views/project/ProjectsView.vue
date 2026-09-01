<template>
  <div class="projects-view">
    <!-- ===== 双栏视图（list + detail 共用） ===== -->
    <div v-if="viewMode !== 'deep'" class="dual-col">
      <!-- 左侧：项目列表 -->
      <div class="left-panel">
        <div class="panel-header">
          <div class="panel-title-row">
            <span class="panel-title">项目列表</span>
            <span class="panel-count">{{ filteredProjects.length }} 个</span>
          </div>
          <div class="panel-filters">
            <div
              v-for="f in statusFilters"
              :key="f.key"
              class="filter-chip"
              :class="{ active: statusFilter === f.key }"
              @click="statusFilter = f.key"
            >{{ f.label }}</div>
          </div>
          <div class="panel-search">
            <span class="search-icon">🔍</span>
            <input v-model="keyword" type="text" placeholder="搜索项目..." />
          </div>
          <div class="panel-new-btn">
            <el-button type="primary" size="small" @click="openCreate" style="width:100%">
              <el-icon><Plus /></el-icon> 新建项目
            </el-button>
          </div>
        </div>
        <div class="panel-list">
          <div
            v-for="p in filteredProjects"
            :key="p.id"
            class="list-item"
            :class="{ active: p.id === current?.id }"
            @click="selectProject(p.id)"
          >
            <div class="list-item-header">
              <span class="status-dot" :class="p.status"></span>
              <span class="list-item-title">{{ p.name || '未命名项目' }}</span>
            </div>
            <div class="list-item-desc">{{ p.description || '暂无描述' }}</div>
            <div class="list-item-meta">
              <div class="progress-bar">
                <div class="progress-fill" :class="p.status" :style="{ width: projectProgress(p) + '%' }"></div>
              </div>
              <span class="progress-text">{{ projectProgress(p) }}%</span>
            </div>
            <div class="member-avatars">
              <div
                v-for="(m, i) in projectMembers(p).slice(0, 4)"
                :key="i"
                class="mini-avatar"
                :style="{ background: avatarColor(i) }"
              >{{ m }}</div>
              <div v-if="projectMembers(p).length > 4" class="mini-avatar" style="background:var(--bg-tertiary)">+{{ projectMembers(p).length - 4 }}</div>
            </div>
          </div>
          <el-empty v-if="!filteredProjects.length" description="暂无项目，点击上方新建" :image-size="60" />
        </div>
      </div>

      <!-- 右侧：项目详情 -->
      <div class="right-panel">
        <template v-if="current">
          <!-- 详情头部 -->
          <div class="detail-header">
            <div class="detail-title-row">
              <div style="flex:1;min-width:0">
                <div class="detail-title">
                  <span class="status-dot" :class="current.status" style="margin-right:10px;vertical-align:middle"></span>
                  {{ current.name || '未命名项目' }}
                </div>
                <div class="detail-desc">{{ current.description || '暂无描述' }}</div>
              </div>
              <div class="detail-actions">
                <button class="top-btn" @click="toggleFavorite">⭐ 收藏</button>
                <button class="top-btn" @click="shareProject">🔗 分享</button>
                <el-button size="small" @click="openEdit" title="编辑">
                  <el-icon><Edit /></el-icon>
                </el-button>
                <el-button size="small" type="danger" plain @click="removeProject" title="删除">
                  <el-icon><Delete /></el-icon>
                </el-button>
                <button class="enter-project-btn" @click="enterProject">🚀 进入项目</button>
              </div>
            </div>
            <div class="detail-meta-row">
              <span><b>状态：</b>{{ statusLabel(current.status) }}</span>
              <span><b>进度：</b>{{ projectProgress(current) }}%</span>
              <span><b>成员：</b>{{ projectMembers(current).length }} 人</span>
              <span><b>创建时间：</b>{{ current.created_at || '—' }}</span>
              <span><b>资源：</b>{{ (current.resources || []).length }} 项</span>
            </div>
          </div>

          <!-- 5 个 Tab -->
          <div class="detail-tabs">
            <div
              v-for="tab in detailTabs"
              :key="tab.key"
              class="detail-tab"
              :class="{ active: detailTab === tab.key }"
              @click="detailTab = tab.key"
            >{{ tab.label }}</div>
          </div>

          <!-- Tab 内容 -->
          <div class="detail-content">
            <!-- ===== 概览 Tab ===== -->
            <div v-if="detailTab === 'overview'">
              <div class="stats-grid">
                <div class="stat-card">
                  <div class="stat-value">{{ projectProgress(current) }}%</div>
                  <div class="stat-label">项目进度</div>
                  <div class="stat-trend up">↑ 5% 本周</div>
                </div>
                <div class="stat-card">
                  <div class="stat-value">{{ mockTasks.filter(t => t.status === 'active').length }}</div>
                  <div class="stat-label">进行中任务</div>
                  <div class="stat-trend up">↑ 2 新增</div>
                </div>
                <div class="stat-card">
                  <div class="stat-value">{{ projectMembers(current).length }}</div>
                  <div class="stat-label">团队成员</div>
                  <div class="stat-trend up">↑ 1 人加入</div>
                </div>
                <div class="stat-card">
                  <div class="stat-value">89%</div>
                  <div class="stat-label">按时交付率</div>
                  <div class="stat-trend down">↓ 2% 较上周</div>
                </div>
              </div>

              <!-- 快速操作 -->
              <div class="overview-actions">
                <button class="top-btn primary" @click="openBinder">
                  <el-icon><Plus /></el-icon> 资源归类
                </button>
                <button class="top-btn" :class="{ loading: generatingGraph }" @click="generateGraph">
                  <el-icon><MagicStick /></el-icon> 生成图谱
                </button>
                <button class="top-btn" @click="goModule('ai')">
                  <el-icon><ChatDotRound /></el-icon> AI 对话
                </button>
                <button class="top-btn" @click="goModule('tasks')">
                  <el-icon><List /></el-icon> 任务管理
                </button>
                <button class="top-btn" @click="goModule('graph')">
                  <el-icon><Share /></el-icon> 知识图谱
                </button>
                <button class="top-btn" @click="goModule('workflow')">
                  <el-icon><Connection /></el-icon> 工作流
                </button>
                <button class="top-btn" @click="goModule('expert-center')">
                  <el-icon><User /></el-icon> 专家联盟
                </button>
                <button class="top-btn" @click="goModule('resources')">
                  <el-icon><Folder /></el-icon> 资源中心
                </button>
              </div>

              <!-- 最近动态 + 待办任务 -->
              <div class="overview-grid">
                <div class="content-section">
                  <div class="section-title">
                    <span>最近动态</span>
                    <span class="more">查看全部 →</span>
                  </div>
                  <div class="activity-list">
                    <div v-for="(a, i) in mockActivities.slice(0, 5)" :key="i" class="activity-item">
                      <div class="activity-icon">{{ a.icon }}</div>
                      <div class="activity-content">
                        <div class="activity-text">{{ a.text }}</div>
                        <div class="activity-time">{{ a.time }}</div>
                      </div>
                    </div>
                  </div>
                </div>
                <div>
                  <div class="content-section">
                    <div class="section-title">
                      <span>待办任务</span>
                      <span class="more">全部 →</span>
                    </div>
                    <div class="task-list">
                      <div
                        v-for="t in mockTasks.filter(x => x.status !== 'done').slice(0, 4)"
                        :key="t.id"
                        class="task-item"
                      >
                        <div class="task-checkbox" @click.stop="toggleTask(t.id)"></div>
                        <div class="task-info">
                          <div class="task-title">{{ t.title }}</div>
                          <div class="task-meta">
                            <span class="task-priority" :class="'priority-' + t.priority">{{ priorityLabel(t.priority) }}</span>
                            <span>{{ t.assignee }}</span>
                            <span>{{ t.due }}</span>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <!-- 已归档资源 -->
              <div class="content-section" v-if="(current.resources || []).length">
                <div class="section-title">
                  <span>已归档资源（{{ current.resources.length }} 项）</span>
                </div>
                <div class="resource-groups">
                  <div v-for="(group, type) in groupedResources" :key="type" class="res-group">
                    <div class="res-group-head">
                      <span class="res-type-badge">{{ typeLabel(type) }}</span>
                      <span class="muted">{{ group.length }} 项</span>
                    </div>
                    <div v-for="r in group" :key="r.rid" class="res-row">
                      <div class="res-info">
                        <span class="res-name">{{ r.resource_name }}</span>
                        <span class="badge" :class="liveClass(r.live_status)">{{ liveLabel(r.live_status) }}</span>
                        <span class="muted res-desc">{{ r.live_desc || r.note || '' }}</span>
                      </div>
                      <div class="res-ops">
                        <el-button size="small" text @click="editNote(r)">备注</el-button>
                        <el-button size="small" text type="danger" @click="unbind(r)">移除</el-button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <!-- ===== 任务 Tab ===== -->
            <div v-if="detailTab === 'tasks'">
              <div class="task-filters">
                <div class="filter-chip" :class="{ active: taskFilter === 'all' }" @click="taskFilter = 'all'">全部 ({{ mockTasks.length }})</div>
                <div class="filter-chip" :class="{ active: taskFilter === 'active' }" @click="taskFilter = 'active'">进行中 ({{ mockTasks.filter(t => t.status === 'active').length }})</div>
                <div class="filter-chip" :class="{ active: taskFilter === 'done' }" @click="taskFilter = 'done'">已完成 ({{ mockTasks.filter(t => t.status === 'done').length }})</div>
                <div class="filter-chip" :class="{ active: taskFilter === 'pending' }" @click="taskFilter = 'pending'">待开始 ({{ mockTasks.filter(t => t.status === 'pending').length }})</div>
              </div>
              <div class="task-list">
                <div
                  v-for="t in filteredTasks"
                  :key="t.id"
                  class="task-item"
                  :class="{ done: t.status === 'done' }"
                >
                  <div class="task-checkbox" @click.stop="toggleTask(t.id)"></div>
                  <div class="task-info">
                    <div class="task-title">{{ t.title }}</div>
                    <div class="task-meta">
                      <span class="task-priority" :class="'priority-' + t.priority">{{ priorityFullLabel(t.priority) }}</span>
                      <span>负责人：{{ t.assignee }}</span>
                      <span>截止：{{ t.due }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <!-- ===== 成员 Tab ===== -->
            <div v-if="detailTab === 'members'">
              <div class="member-grid">
                <div v-for="(m, i) in projectMembers(current)" :key="i" class="member-card">
                  <div class="member-avatar-lg" :style="{ background: avatarColor(i) }">{{ m }}</div>
                  <div class="member-name">{{ m }}工</div>
                  <div class="member-role">{{ memberRoles[i % memberRoles.length] }}</div>
                  <div class="member-skills">
                    <span v-for="(s, j) in memberSkillSets[i % memberSkillSets.length]" :key="j" class="skill-tag">{{ s }}</span>
                  </div>
                </div>
              </div>
            </div>

            <!-- ===== 文档 Tab ===== -->
            <div v-if="detailTab === 'docs'">
              <div class="task-list">
                <div v-for="(d, i) in mockDocs" :key="i" class="task-item">
                  <div class="doc-icon">{{ d.icon }}</div>
                  <div class="task-info">
                    <div class="task-title">{{ d.name }}</div>
                    <div class="task-meta">
                      <span>{{ d.size }}</span>
                      <span>更新于 {{ d.time }}</span>
                    </div>
                  </div>
                  <button class="top-btn" style="flex-shrink:0" @click="downloadDoc(d)">下载</button>
                </div>
              </div>
            </div>

            <!-- ===== 动态 Tab ===== -->
            <div v-if="detailTab === 'activity'">
              <div class="activity-list">
                <div v-for="(a, i) in mockActivities" :key="i" class="activity-item">
                  <div class="activity-icon">{{ a.icon }}</div>
                  <div class="activity-content">
                    <div class="activity-text">{{ a.text }}</div>
                    <div class="activity-time">{{ a.time }}</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </template>

        <!-- 未选中项目时的空状态 -->
        <div v-else class="empty-detail">
          <el-empty description="选择或新建一个项目，开始全维归类" :image-size="100" />
        </div>
      </div>
    </div>

    <!-- ===== 深视图（deep view） ===== -->
    <div v-else class="project-deep-view">
      <!-- 左侧：文件树 + 协作专家 -->
      <div class="deep-left">
        <div class="deep-back-row">
          <button class="back-btn" @click="backToList">← 返回列表</button>
        </div>
        <div class="deep-section">
          <div class="deep-section-title">项目文件</div>
          <div class="file-tree">
            <div class="file-folder">📁 backend</div>
            <div class="file-item active">📄 nlp_service.py</div>
            <div class="file-item">📄 chat_handler.py</div>
            <div class="file-item">📄 knowledge_base.py</div>
            <div class="file-folder" style="margin-top:8px">📁 frontend</div>
            <div class="file-item">📄 App.vue</div>
            <div class="file-item">📄 ChatPanel.vue</div>
            <div class="file-folder" style="margin-top:8px">📁 docs</div>
            <div class="file-item">📄 README.md</div>
            <div class="file-item">📄 API.md</div>
          </div>
        </div>
        <div class="deep-section">
          <div class="deep-section-title">协作专家</div>
          <div class="deep-nav-item active"><span class="icon">🟢</span> 张工 (在线)</div>
          <div class="deep-nav-item"><span class="icon">🟢</span> 李工 (在线)</div>
          <div class="deep-nav-item"><span class="icon">⚫</span> 王工 (离线)</div>
          <div class="deep-nav-item"><span class="icon">🟡</span> 赵工 (忙碌)</div>
        </div>
      </div>

      <!-- 中间：代码编辑器 -->
      <div class="deep-center">
        <div class="editor-tabs">
          <div class="editor-tab active">
            📄 nlp_service.py
            <span class="tab-close">✕</span>
          </div>
          <div class="editor-tab">📄 README.md</div>
          <div class="editor-tab tab-add">+</div>
        </div>
        <div class="deep-editor">
          <div v-for="line in codeLines" :key="line.num" class="code-line">
            <span class="line-num">{{ line.num }}</span>
            <span v-html="line.html || '&nbsp;'"></span>
          </div>
        </div>
      </div>

      <!-- 右侧：今日任务 + AI建议 + 实时预览 -->
      <div class="deep-right">
        <div class="deep-panel">
          <div class="deep-panel-title">
            <span>今日任务</span>
            <span class="more">+ 新建</span>
          </div>
          <div class="deep-tasks">
            <div
              v-for="t in mockTasks.slice(0, 4)"
              :key="t.id"
              class="deep-task"
              :class="{ done: t.status === 'done' }"
            >
              <div class="deep-task-check" :class="{ checked: t.status === 'done' }" @click.stop="toggleTask(t.id)"></div>
              <div class="deep-task-text">{{ t.title }}</div>
            </div>
          </div>
        </div>
        <div class="deep-panel">
          <div class="deep-panel-title">
            <span>💡 AI 建议</span>
          </div>
          <div class="ai-suggestion">
            建议在 <code>analyze()</code> 方法中增加缓存层，可减少 30% 的重复计算，显著提升响应速度。
            <div class="ai-suggestion-actions">
              <button class="top-btn primary" style="font-size:11px;padding:4px 10px" @click="applySuggestion">应用建议</button>
              <button class="top-btn" style="font-size:11px;padding:4px 10px" @click="ignoreSuggestion">忽略</button>
            </div>
          </div>
        </div>
        <div class="deep-panel deep-panel-flex">
          <div class="deep-panel-title">
            <span>实时预览</span>
          </div>
          <div class="live-preview">
            <div class="live-status">● 服务运行中</div>
            <div class="live-metrics">
              端口: 8080<br>
              内存: 128MB<br>
              CPU: 5%<br>
              请求/秒: 24
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ===== 新建 / 编辑项目 Dialog（保留） ===== -->
    <el-dialog v-model="dlg.visible" :title="dlg.isEdit ? '编辑项目' : '新建项目'" width="520px">
      <el-form label-width="76px">
        <el-form-item label="项目名称">
          <el-input v-model="dlg.form.name" placeholder="如：璇玑平台系统" maxlength="60" />
        </el-form-item>
        <el-form-item label="项目类型">
          <div class="cat-picker">
            <div
              v-for="c in categories"
              :key="c.key"
              class="cat-option"
              :class="{ active: dlg.form.category === c.key }"
              :style="dlg.form.category === c.key ? { borderColor: c.color, background: c.color + '14' } : {}"
              @click="dlg.form.category = c.key"
            >
              <el-icon :style="{ color: c.color }"><component :is="c.icon" /></el-icon>
              <span>{{ c.label }}</span>
            </div>
          </div>
        </el-form-item>
        <el-form-item label="状态">
          <el-radio-group v-model="dlg.form.status">
            <el-radio value="active">进行中</el-radio>
            <el-radio value="done">已完成</el-radio>
            <el-radio value="archived">已归档</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="dlg.form.description" type="textarea" :rows="3" placeholder="项目说明（可选）" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dlg.visible = false">取消</el-button>
        <el-button type="primary" :loading="dlg.saving" @click="saveProject">保存</el-button>
      </template>
    </el-dialog>

    <!-- ===== 全维目录归档 Drawer（保留） ===== -->
    <el-drawer v-model="binder.visible" title="全维资源目录 · 归类到项目" size="620px">
      <div class="binder-head">
        <el-input v-model="binder.keyword" placeholder="搜索全部资源…" clearable>
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <div class="binder-picked" v-if="binder.picked.size">
          已选 <b>{{ binder.picked.size }}</b> 项
          <el-button size="small" type="primary" :loading="binder.binding" @click="doBind">
            归档到「{{ current?.name }}」
          </el-button>
        </div>
      </div>
      <el-scrollbar class="binder-scroll">
        <el-collapse v-model="binder.openGroups">
          <el-collapse-item v-for="g in filteredGroups" :key="g.type" :name="g.type">
            <template #title>
              <span class="binder-group-title">
                <span class="res-type-badge">{{ g.label }}</span>
                <span class="muted">{{ g.count }} 项</span>
              </span>
            </template>
            <div
              v-for="it in g.items"
              :key="it.id"
              class="binder-item"
              :class="{ picked: binder.picked.has(g.type + ':' + it.id), bound: isBound(g.type, it.id) }"
              @click="togglePick(g.type, it.id, it)"
            >
              <div class="binder-item-main">
                <span class="res-name">{{ it.name }}</span>
                <span class="badge" :class="liveClass(it.status)">{{ it.status }}</span>
                <span class="muted binder-desc">{{ it.desc }}</span>
              </div>
              <el-tag v-if="isBound(g.type, it.id)" size="small" type="info">已归档</el-tag>
              <el-icon v-else-if="binder.picked.has(g.type + ':' + it.id)" class="pick-mark"><CircleCheckFilled /></el-icon>
            </div>
          </el-collapse-item>
        </el-collapse>
        <el-empty v-if="!filteredGroups.length" description="未匹配到资源" :image-size="70" />
      </el-scrollbar>
    </el-drawer>

    <!-- ===== 资源备注编辑 Dialog（保留） ===== -->
    <el-dialog v-model="noteDlg.visible" title="资源备注" width="420px">
      <el-input v-model="noteDlg.note" type="textarea" :rows="3" placeholder="该资源在此项目中的定位 / 用途说明" />
      <template #footer>
        <el-button @click="noteDlg.visible = false">取消</el-button>
        <el-button type="primary" @click="saveNote">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Plus, Search, Edit, Delete, Promotion, ChatDotRound, List, Share,
  MagicStick, Connection, User, Folder, CircleCheckFilled
} from '@element-plus/icons-vue'
import { useProject } from '@/composables/projectContext.js'
import {
  getProjects, getProjectTypes, getProjectCatalog, getProjectStats,
  getProject, createProject, updateProject, deleteProject,
  bindProjectResources, unbindProjectResource, updateProjectResourceNote,
  aiGenerateProjectGraph
} from '@/api'

const router = useRouter()

// ===== 视图状态 =====
const viewMode = ref('list') // 'list' | 'detail' | 'deep'
const detailTab = ref('overview')
const statusFilter = ref('all')
const taskFilter = ref('all')

const statusFilters = [
  { key: 'all', label: '全部' },
  { key: 'active', label: '进行中' },
  { key: 'archived', label: '暂停' },
  { key: 'done', label: '已完成' }
]

const detailTabs = [
  { key: 'overview', label: '概览' },
  { key: 'tasks', label: '任务' },
  { key: 'members', label: '成员' },
  { key: 'docs', label: '文档' },
  { key: 'activity', label: '动态' }
]

// ===== 原有状态 =====
const projects = ref([])
const categories = ref([])
const resourceTypes = ref([])
const catalogGroups = ref([])
const stats = ref({})
const current = ref(null)
const keyword = ref('')
const filterCategory = ref('')

const dlg = ref({ visible: false, isEdit: false, saving: false, form: { name: '', category: 'custom', status: 'active', description: '' } })
const binder = ref({ visible: false, keyword: '', picked: new Map(), openGroups: [], binding: false })
const noteDlg = ref({ visible: false, rid: '', note: '' })
const generatingGraph = ref(false)

// ===== Mock 数据（任务/动态/文档） =====
const mockTasks = ref([
  { id: 1, title: '完成 NLP 模块单元测试', priority: 'high', status: 'done', assignee: '张工', due: '今天' },
  { id: 2, title: 'Review 李工的 PR #234', priority: 'mid', status: 'done', assignee: '王工', due: '今天' },
  { id: 3, title: '优化对话响应速度至 500ms 内', priority: 'high', status: 'active', assignee: '张工', due: '本周三' },
  { id: 4, title: '知识库接入联调测试', priority: 'mid', status: 'active', assignee: '李工', due: '本周五' },
  { id: 5, title: '编写 API 接口文档', priority: 'low', status: 'active', assignee: '王工', due: '下周' },
  { id: 6, title: '性能压测报告输出', priority: 'mid', status: 'pending', assignee: '赵工', due: '下周三' },
  { id: 7, title: '用户验收测试准备', priority: 'high', status: 'pending', assignee: '全员', due: '下周五' }
])

const mockActivities = [
  { icon: '💬', text: '张工 在 NLP 模块提交了新代码', time: '5 分钟前' },
  { icon: '📄', text: '李工 更新了《需求规格说明书 v2.3》', time: '20 分钟前' },
  { icon: '✅', text: '王工 完成了任务「代码 Review」', time: '1 小时前' },
  { icon: '👥', text: '赵工 加入了项目团队', time: '2 小时前' },
  { icon: '🚀', text: '项目 v1.2.0 版本成功发布', time: '昨天 18:30' },
  { icon: '💡', text: 'AI 助手 建议：增加缓存层可提升 30% 性能', time: '昨天 15:00' }
]

const mockDocs = [
  { name: '需求规格说明书 v2.3.md', size: '2.4 MB', time: '2 小时前', icon: '📝' },
  { name: '架构设计文档.pdf', size: '5.1 MB', time: '昨天', icon: '📐' },
  { name: 'API 接口文档', size: '856 KB', time: '3 天前', icon: '📡' },
  { name: '数据库设计.sql', size: '128 KB', time: '上周', icon: '🗄️' },
  { name: '测试用例.xlsx', size: '2.1 MB', time: '2 周前', icon: '✅' },
  { name: '部署手册.md', size: '640 KB', time: '1 个月前', icon: '🚀' }
]

const memberRoles = ['项目负责人', '技术专家', '开发工程师', '测试工程师', '产品经理']
const memberSkillSets = [
  ['架构设计', 'NLP', 'Python'],
  ['前端', 'React', 'TypeScript'],
  ['后端', 'Node.js', '数据库'],
  ['测试', '自动化', '性能'],
  ['产品设计', '用户研究', 'AI产品']
]

// ===== 代码编辑器 30 行 =====
const codeLines = [
  { num: 1, html: '<span class="code-keyword">import</span> torch' },
  { num: 2, html: '<span class="code-keyword">from</span> transformers <span class="code-keyword">import</span> AutoTokenizer, AutoModel' },
  { num: 3, html: '' },
  { num: 4, html: '<span class="code-keyword">class</span> <span class="code-function">NLPService</span>:' },
  { num: 5, html: '    <span class="code-string">"""NLP 语义分析服务"""</span>' },
  { num: 6, html: '' },
  { num: 7, html: '    <span class="code-keyword">def</span> <span class="code-function">__init__</span>(self, model_name: str = <span class="code-string">"bert-base-chinese"</span>):' },
  { num: 8, html: '        self.tokenizer = AutoTokenizer.from_pretrained(model_name)' },
  { num: 9, html: '        self.model = AutoModel.from_pretrained(model_name)' },
  { num: 10, html: '        self.device = torch.device(<span class="code-string">"cuda"</span> <span class="code-keyword">if</span> torch.cuda.is_available() <span class="code-keyword">else</span> <span class="code-string">"cpu"</span>)' },
  { num: 11, html: '        self.model.to(self.device)' },
  { num: 12, html: '' },
  { num: 13, html: '    <span class="code-keyword">def</span> <span class="code-function">analyze</span>(self, text: str) -> dict:' },
  { num: 14, html: '        <span class="code-string">"""对输入文本进行语义分析"""</span>' },
  { num: 15, html: '        inputs = self.tokenizer(' },
  { num: 16, html: '            text,' },
  { num: 17, html: '            return_tensors=<span class="code-string">"pt"</span>,' },
  { num: 18, html: '            max_length=<span class="code-number">512</span>,' },
  { num: 19, html: '            truncation=<span class="code-keyword">True</span>' },
  { num: 20, html: '        ).to(self.device)' },
  { num: 21, html: '' },
  { num: 22, html: '        <span class="code-keyword">with</span> torch.no_grad():' },
  { num: 23, html: '            outputs = self.model(**inputs)' },
  { num: 24, html: '            embeddings = outputs.last_hidden_state.mean(dim=<span class="code-number">1</span>)' },
  { num: 25, html: '' },
  { num: 26, html: '        <span class="code-keyword">return</span> {' },
  { num: 27, html: '            <span class="code-string">"embedding"</span>: embeddings.squeeze().tolist(),' },
  { num: 28, html: '            <span class="code-string">"tokens_count"</span>: inputs[<span class="code-string">"input_ids"</span>].shape[<span class="code-number">1</span>],' },
  { num: 29, html: '            <span class="code-string">"language"</span>: <span class="code-string">"zh"</span>' },
  { num: 30, html: '        }' }
]

// ===== 计算属性 =====
const catalogTotal = computed(() => catalogGroups.value.reduce((n, g) => n + g.count, 0))
const resourceTypeCount = computed(() => catalogGroups.value.length)

const filteredProjects = computed(() => {
  let list = projects.value
  if (statusFilter.value !== 'all') list = list.filter((p) => p.status === statusFilter.value)
  if (filterCategory.value) list = list.filter((p) => p.category === filterCategory.value)
  if (keyword.value.trim()) {
    const k = keyword.value.trim().toLowerCase()
    list = list.filter((p) => (p.name || '').toLowerCase().includes(k) || (p.description || '').toLowerCase().includes(k))
  }
  return list
})

const filteredTasks = computed(() => {
  if (taskFilter.value === 'all') return mockTasks.value
  return mockTasks.value.filter((t) => t.status === taskFilter.value)
})

const groupedResources = computed(() => {
  const map = {}
  for (const r of current.value?.resources || []) {
    (map[r.resource_type] = map[r.resource_type] || []).push(r)
  }
  return map
})

const filteredGroups = computed(() => {
  const k = binder.value.keyword.trim().toLowerCase()
  if (!k) return catalogGroups.value
  return catalogGroups.value
    .map((g) => ({ ...g, items: g.items.filter((it) => (it.name || '').toLowerCase().includes(k) || (it.desc || '').includes(k)) }))
    .filter((g) => g.items.length)
})

// ===== 工具函数 =====
const categoryOf = (key) => categories.value.find((c) => c.key === key)
const categoryLabel = (key) => categoryOf(key)?.label || key || '未分类'
const typeLabel = (key) => resourceTypes.value.find((t) => t.key === key)?.label || key

const statusLabel = (s) => ({ active: '进行中', done: '已完成', archived: '已归档' }[s] || s || '进行中')
const statusClass = (s) => ({ active: 'success', done: 'info', archived: 'warning' }[s] || 'info')
const liveLabel = (s) => ({ online: '在线', running: '运行中', active: '活跃', valid: '有效', missing: '已失效', disabled: '已停用', stopped: '已停止', invalid: '无效' }[s] || s || '-')
const liveClass = (s) => {
  if (['online', 'running', 'active', 'valid', 'published', 'registered'].includes(s)) return 'success'
  if (['missing', 'invalid'].includes(s)) return 'danger'
  if (['disabled', 'stopped'].includes(s)) return 'warning'
  return 'info'
}

const isBound = (type, id) => (current.value?.resources || []).some((r) => r.resource_type === type && String(r.resource_id) === String(id))

function projectProgress(p) {
  if (p && p.progress != null) return p.progress
  return { active: 60, done: 100, archived: 30 }[(p && p.status) || ''] || 0
}

function projectMembers(p) {
  if (p && p.members && p.members.length) return p.members
  const name = (p && p.name) || '项目'
  const chars = name.replace(/[\s\W]/g, '').slice(0, 4)
  return chars ? chars.split('') : ['项', '目']
}

function avatarColor(i) {
  const colors = ['#6366f1', '#06b6d4', '#10b981', '#f59e0b', '#ef4444', '#ec4899', '#8b5cf6', '#14b8a6']
  return colors[i % colors.length]
}

function priorityLabel(p) {
  return { high: '高优', mid: '中优', low: '低优' }[p] || p
}
function priorityFullLabel(p) {
  return { high: '高优先级', mid: '中优先级', low: '低优先级' }[p] || p
}

// ===== 视图切换 =====
async function selectProject(id) {
  current.value = await getProject(id)
  viewMode.value = 'detail'
  detailTab.value = 'overview'
}

function enterProject() {
  if (!current.value) return
  viewMode.value = 'deep'
  ElMessage.success('已进入项目开发工作台')
}

function backToList() {
  viewMode.value = 'list'
}

function toggleTask(id) {
  const t = mockTasks.value.find((x) => x.id === id)
  if (t) {
    t.status = t.status === 'done' ? 'active' : 'done'
    ElMessage[t.status === 'done' ? 'success' : 'info'](t.status === 'done' ? '任务已完成 🎉' : '任务已恢复')
  }
}

function toggleFavorite() {
  ElMessage.info('项目已添加到收藏夹')
}
function shareProject() {
  ElMessage.info('分享链接已复制')
}
function downloadDoc(d) {
  ElMessage.info(`${d.name} 开始下载`)
}
function applySuggestion() {
  ElMessage.success('AI 建议已应用到代码中')
}
function ignoreSuggestion() {
  ElMessage.info('已忽略该建议')
}

// ===== 加载 =====
async function loadAll() {
  const [ps, ts, cat, st] = await Promise.all([getProjects(), getProjectTypes(), getProjectCatalog(), getProjectStats()])
  projects.value = ps || []
  categories.value = (ts && ts.categories) || []
  resourceTypes.value = (ts && ts.resource_types) || []
  catalogGroups.value = (cat && cat.groups) || []
  stats.value = st || {}
  binder.value.openGroups = (catalogGroups.value).slice(0, 3).map((g) => g.type)
}

// 快速跳转到相关模块
function goModule(path) {
  if (!current.value) return
  router.push(`/${path}`)
  ElMessage.info(`已进入「${current.value.name || '未命名项目'}」项目上下文`)
}

// 一键生成项目知识图谱
async function generateGraph() {
  if (!current.value) return
  generatingGraph.value = true
  try {
    await aiGenerateProjectGraph({
      project_id: current.value.id,
      project_name: current.value.name,
      description: current.value.description || ''
    })
    ElMessage.success('知识图谱生成成功！正在跳转…')
    setTimeout(() => { router.push('/graph') }, 800)
  } catch (e) {
    ElMessage.error('生成失败：' + e.message)
  } finally {
    generatingGraph.value = false
  }
}

// ===== 项目 CRUD =====
function openCreate() {
  dlg.value = { visible: true, isEdit: false, saving: false, form: { name: '', category: 'platform', status: 'active', description: '' } }
}
function openEdit() {
  if (!current.value) return
  dlg.value = { visible: true, isEdit: true, saving: false, form: { name: current.value.name, category: current.value.category, status: current.value.status || 'active', description: current.value.description || '' } }
}
async function saveProject() {
  const f = dlg.value.form
  if (!f.name.trim()) return ElMessage.warning('请输入项目名称')
  dlg.value.saving = true
  try {
    if (dlg.value.isEdit) {
      await updateProject(current.value.id, f)
      ElMessage.success('项目已更新')
    } else {
      const created = await createProject(f)
      ElMessage.success('项目已创建')
      await refreshList()
      await selectProject(created.id)
    }
    if (dlg.value.isEdit) await refreshCurrent()
    dlg.value.visible = false
  } finally {
    dlg.value.saving = false
  }
}
async function removeProject() {
  if (!current.value) return
  await ElMessageBox.confirm(`确定删除项目「${current.value.name || '未命名项目'}」？资源本身不受影响，仅解除归类。`, '删除项目', { type: 'warning' })
  await deleteProject(current.value.id)
  ElMessage.success('已删除')
  current.value = null
  viewMode.value = 'list'
  await refreshList()
}
async function refreshList() {
  const [ps, st] = await Promise.all([getProjects(), getProjectStats()])
  projects.value = ps
  stats.value = st
}
async function refreshCurrent() {
  if (current.value) current.value = await getProject(current.value.id)
}

// ===== 资源绑定 =====
function openBinder() {
  if (!current.value) {
    ElMessage.warning('请先选择一个项目')
    return
  }
  binder.value.keyword = ''
  binder.value.picked = new Map()
  binder.value.visible = true
}
function togglePick(type, id, item) {
  const key = type + ':' + id
  if (binder.value.picked.has(key)) binder.value.picked.delete(key)
  else binder.value.picked.set(key, { type, id, name: item.name })
}
async function doBind() {
  binder.value.binding = true
  try {
    const items = [...binder.value.picked.values()]
    const r = await bindProjectResources(current.value.id, { items })
    ElMessage.success(`已归档 ${r.added} 项资源`)
    binder.value.visible = false
    await refreshCurrent()
    await refreshList()
  } finally {
    binder.value.binding = false
  }
}
async function unbind(r) {
  await unbindProjectResource(current.value.id, r.rid)
  ElMessage.success('已移除归类')
  await refreshCurrent()
  await refreshList()
}

// ===== 备注 =====
function editNote(r) {
  noteDlg.value = { visible: true, rid: r.rid, note: r.note || '' }
}
async function saveNote() {
  await updateProjectResourceNote(current.value.id, noteDlg.value.rid, { note: noteDlg.value.note })
  ElMessage.success('备注已保存')
  noteDlg.value.visible = false
  await refreshCurrent()
}

// ===== 项目上下文联动 =====
{
  const { onChange: _onProjectChange, ensureProjectContext: _ensureProject } = useProject()
  let _offPj = null
  let _loaded = false
  onMounted(async () => {
    _offPj = _onProjectChange(async () => { loadAll() })
    await _ensureProject().catch(() => {})
    if (!_loaded) {
      _loaded = true
      loadAll()
    }
  })
  onBeforeUnmount(() => { _offPj && _offPj() })
}
</script>

<style scoped>
/* ===== 根容器 ===== */
.projects-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* ===== 双栏布局 ===== */
.dual-col {
  display: flex;
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

/* ===== 左侧面板 ===== */
.left-panel {
  width: 320px;
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  background: var(--bg-secondary);
}

.panel-header {
  padding: 16px;
  border-bottom: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.panel-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.panel-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.panel-count {
  font-size: 11px;
  color: var(--text-muted);
}

.panel-filters {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.filter-chip {
  padding: 4px 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 16px;
  font-size: 11px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
  user-select: none;
}

.filter-chip:hover {
  border-color: var(--accent);
}

.filter-chip.active {
  background: var(--accent-dim);
  border-color: var(--accent);
  color: var(--accent-light);
}

.panel-search {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
}

.panel-search .search-icon {
  font-size: 14px;
  color: var(--text-muted);
}

.panel-search input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-primary);
  font-size: 12px;
  font-family: inherit;
}

.panel-search input::placeholder {
  color: var(--text-muted);
}

.panel-new-btn {
  margin-top: -4px;
}

.panel-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

/* ===== 列表项 ===== */
.list-item {
  padding: 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
  border-left: 3px solid transparent;
  margin-bottom: 4px;
}

.list-item:hover {
  background: var(--bg-hover);
}

.list-item.active {
  background: var(--accent-dim);
  border-left-color: var(--accent);
}

.list-item-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-dot.active { background: var(--success); }
.status-dot.archived { background: var(--warning); }
.status-dot.done { background: var(--accent); }

.list-item-title {
  font-size: 13px;
  font-weight: 500;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-primary);
}

.list-item.active .list-item-title {
  color: var(--accent-light);
}

.list-item-desc {
  font-size: 11px;
  color: var(--text-muted);
  margin-bottom: 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  line-height: 1.5;
}

.list-item-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.progress-bar {
  flex: 1;
  height: 4px;
  background: var(--bg-card);
  border-radius: 2px;
  overflow: hidden;
  margin-right: 8px;
}

.progress-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.3s ease;
}

.progress-fill.active { background: var(--success); }
.progress-fill.archived { background: var(--warning); }
.progress-fill.done { background: var(--accent); }

.progress-text {
  font-size: 10px;
  color: var(--text-muted);
  min-width: 32px;
  text-align: right;
}

.member-avatars {
  display: flex;
  margin-top: 8px;
}

.mini-avatar {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid var(--bg-secondary);
  margin-left: -6px;
  font-size: 9px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-weight: 500;
}

.list-item.active .mini-avatar {
  border-color: rgba(99, 102, 241, 0.2);
}

.mini-avatar:first-child {
  margin-left: 0;
}

/* ===== 右侧详情面板 ===== */
.right-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.empty-detail {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* ===== 详情头部 ===== */
.detail-header {
  padding: 20px 24px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);
  flex-shrink: 0;
}

.detail-title-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 12px;
  gap: 16px;
}

.detail-title {
  font-size: 20px;
  font-weight: 600;
  margin-bottom: 6px;
  color: var(--text-primary);
}

.detail-desc {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.6;
}

.detail-actions {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-shrink: 0;
  flex-wrap: wrap;
}

.detail-meta-row {
  display: flex;
  gap: 20px;
  margin-top: 16px;
  font-size: 12px;
  color: var(--text-secondary);
  flex-wrap: wrap;
}

.detail-meta-row b {
  color: var(--text-primary);
  font-weight: 600;
}

/* ===== 通用按钮 ===== */
.top-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.top-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
  border-color: var(--accent);
}

.top-btn.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: white;
}

.top-btn.primary:hover {
  background: #5558e3;
  border-color: #5558e3;
  color: white;
}

.top-btn.loading {
  opacity: 0.7;
  pointer-events: none;
}

.enter-project-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  border: none;
  border-radius: 6px;
  color: white;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.enter-project-btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.4);
}

/* ===== Tab 栏 ===== */
.detail-tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border);
  padding: 0 24px;
  background: var(--bg-secondary);
  flex-shrink: 0;
}

.detail-tab {
  padding: 12px 16px;
  font-size: 13px;
  color: var(--text-secondary);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all 0.2s;
  white-space: nowrap;
  user-select: none;
}

.detail-tab:hover {
  color: var(--text-primary);
}

.detail-tab.active {
  color: var(--accent-light);
  border-bottom-color: var(--accent);
}

/* ===== Tab 内容区 ===== */
.detail-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}

/* ===== 统计卡 ===== */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 20px;
}

.stat-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
}

.stat-value {
  font-size: 24px;
  font-weight: 700;
  margin-bottom: 4px;
  color: var(--text-primary);
}

.stat-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.stat-trend {
  font-size: 11px;
  margin-top: 8px;
  display: flex;
  align-items: center;
  gap: 4px;
}

.stat-trend.up { color: var(--success); }
.stat-trend.down { color: var(--danger); }

/* ===== 概览快速操作 ===== */
.overview-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 24px;
}

/* ===== 概览双栏 ===== */
.overview-grid {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 20px;
  margin-bottom: 24px;
}

.content-section {
  margin-bottom: 0;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: var(--text-primary);
}

.section-title .more {
  font-size: 12px;
  color: var(--accent-light);
  cursor: pointer;
  font-weight: 400;
}

/* ===== 动态列表 ===== */
.activity-list {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 0;
}

.activity-item {
  display: flex;
  gap: 12px;
  padding: 10px 16px;
  border-bottom: 1px solid var(--border);
}

.activity-item:last-child {
  border-bottom: none;
}

.activity-icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: var(--accent-dim);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  flex-shrink: 0;
}

.activity-content {
  flex: 1;
  min-width: 0;
}

.activity-text {
  font-size: 13px;
  margin-bottom: 2px;
  color: var(--text-primary);
}

.activity-time {
  font-size: 11px;
  color: var(--text-muted);
}

/* ===== 任务列表 ===== */
.task-filters {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.task-list {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
}

.task-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background 0.15s;
}

.task-item:last-child {
  border-bottom: none;
}

.task-item:hover {
  background: var(--bg-hover);
}

.task-checkbox {
  width: 18px;
  height: 18px;
  border: 2px solid var(--border);
  border-radius: 4px;
  flex-shrink: 0;
  cursor: pointer;
  margin-top: 1px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.task-checkbox:hover {
  border-color: var(--accent);
}

.task-item.done .task-checkbox {
  background: var(--success);
  border-color: var(--success);
}

.task-item.done .task-checkbox::after {
  content: '✓';
  color: white;
  font-size: 12px;
  font-weight: bold;
}

.task-info {
  flex: 1;
  min-width: 0;
}

.task-title {
  font-size: 13px;
  margin-bottom: 4px;
  color: var(--text-primary);
}

.task-item.done .task-title {
  text-decoration: line-through;
  color: var(--text-muted);
}

.task-meta {
  display: flex;
  gap: 12px;
  font-size: 11px;
  color: var(--text-muted);
  flex-wrap: wrap;
}

.task-priority {
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 500;
}

.priority-high { background: rgba(239, 68, 68, 0.15); color: var(--danger); }
.priority-mid { background: rgba(245, 158, 11, 0.15); color: var(--warning); }
.priority-low { background: rgba(16, 185, 129, 0.15); color: var(--success); }

/* ===== 成员网格 ===== */
.member-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 12px;
}

.member-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  text-align: center;
  transition: all 0.2s;
  cursor: pointer;
}

.member-card:hover {
  border-color: var(--accent);
  transform: translateY(-2px);
}

.member-avatar-lg {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  margin: 0 auto 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  font-weight: 600;
  color: white;
}

.member-name {
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 4px;
  color: var(--text-primary);
}

.member-role {
  font-size: 11px;
  color: var(--text-muted);
  margin-bottom: 8px;
}

.member-skills {
  display: flex;
  gap: 4px;
  justify-content: center;
  flex-wrap: wrap;
}

.skill-tag {
  font-size: 10px;
  padding: 2px 6px;
  background: var(--bg-tertiary);
  border-radius: 4px;
  color: var(--text-secondary);
}

/* ===== 文档图标 ===== */
.doc-icon {
  font-size: 24px;
  margin-right: 8px;
  flex-shrink: 0;
}

/* ===== 资源分组 ===== */
.resource-groups {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.res-group {
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
  background: var(--bg-card);
}

.res-group-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  background: var(--bg-tertiary);
}

.res-group-head .muted {
  color: var(--text-muted);
  font-size: 12px;
}

.res-type-badge {
  font-size: 12px;
  font-weight: 600;
  padding: 2px 10px;
  border-radius: 6px;
  background: var(--accent-dim);
  color: var(--accent-light);
}

.res-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
  border-top: 1px solid var(--border);
}

.res-info {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex: 1;
  flex-wrap: wrap;
}

.res-name {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  color: var(--text-primary);
}

.res-desc {
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-muted);
}

.res-ops {
  flex-shrink: 0;
  display: flex;
  gap: 4px;
}

.badge {
  font-size: 11px;
  padding: 1px 8px;
  border-radius: 10px;
  font-weight: 500;
}

.badge.success { background: rgba(16, 185, 129, 0.15); color: var(--success); }
.badge.info { background: var(--accent-dim); color: var(--accent-light); }
.badge.warning { background: rgba(245, 158, 11, 0.15); color: var(--warning); }
.badge.danger { background: rgba(239, 68, 68, 0.15); color: var(--danger); }

.muted {
  color: var(--text-muted);
}

/* ===== Dialog / Drawer 内部样式（保留） ===== */
.cat-picker {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.cat-option {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
  color: var(--text-secondary);
}

.cat-option:hover {
  border-color: var(--accent);
}

.cat-option.active {
  border-color: var(--accent);
}

.binder-head {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 12px;
}

.binder-picked {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  color: var(--text-primary);
}

.binder-scroll {
  height: calc(100vh - 190px);
}

.binder-group-title {
  display: flex;
  align-items: center;
  gap: 10px;
}

.binder-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  margin-bottom: 4px;
  cursor: pointer;
  transition: all 0.12s;
  background: var(--bg-card);
}

.binder-item:hover {
  border-color: var(--accent);
}

.binder-item.picked {
  border-color: var(--accent);
  background: var(--accent-dim);
}

.binder-item.bound {
  opacity: 0.55;
  cursor: not-allowed;
}

.binder-item-main {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex: 1;
  flex-wrap: wrap;
}

.binder-desc {
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-muted);
}

.pick-mark {
  color: var(--success);
  font-size: 18px;
}

/* ===== 深视图 ===== */
.project-deep-view {
  display: flex;
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.deep-left {
  width: 220px;
  background: var(--bg-tertiary);
  border-right: 1px solid var(--border);
  padding: 12px;
  overflow-y: auto;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
}

.deep-back-row {
  margin-bottom: 12px;
}

.back-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
  width: 100%;
}

.back-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
  border-color: var(--accent);
}

.deep-section {
  margin-bottom: 16px;
}

.deep-section-title {
  font-size: 10px;
  text-transform: uppercase;
  color: var(--text-muted);
  letter-spacing: 0.5px;
  margin-bottom: 8px;
  padding: 0 8px;
  font-weight: 600;
}

.deep-nav-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  border-radius: 6px;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s;
  margin-bottom: 2px;
}

.deep-nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.deep-nav-item.active {
  background: var(--accent-dim);
  color: var(--accent-light);
}

.deep-nav-item .icon {
  font-size: 14px;
  width: 18px;
  text-align: center;
}

/* ===== 文件树 ===== */
.file-tree {
  font-size: 12px;
}

.file-folder {
  padding: 4px 8px;
  color: var(--text-secondary);
  cursor: pointer;
  border-radius: 4px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.file-folder:hover {
  background: var(--bg-hover);
}

.file-item {
  padding: 4px 8px 4px 28px;
  color: var(--text-muted);
  cursor: pointer;
  border-radius: 4px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.file-item:hover {
  background: var(--bg-hover);
  color: var(--text-secondary);
}

.file-item.active {
  background: var(--accent-dim);
  color: var(--accent-light);
}

/* ===== 深视图中间 ===== */
.deep-center {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.editor-tabs {
  display: flex;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.editor-tab {
  padding: 10px 16px;
  font-size: 12px;
  color: var(--text-muted);
  border-bottom: 2px solid transparent;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  white-space: nowrap;
  user-select: none;
}

.editor-tab.active {
  background: var(--bg-primary);
  border-bottom-color: var(--accent);
  color: var(--text-primary);
}

.editor-tab .tab-close {
  color: var(--text-muted);
  cursor: pointer;
  font-size: 11px;
}

.editor-tab.tab-add {
  font-size: 14px;
  padding: 10px 14px;
}

/* ===== 代码编辑器 ===== */
.deep-editor {
  flex: 1;
  padding: 20px;
  font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.8;
  color: var(--text-secondary);
  overflow-y: auto;
  background: var(--bg-primary);
}

.code-line {
  display: flex;
  gap: 16px;
}

.line-num {
  color: var(--text-muted);
  user-select: none;
  min-width: 30px;
  text-align: right;
  flex-shrink: 0;
}

.code-keyword { color: #c792ea; }
.code-string { color: #c3e88d; }
.code-comment { color: #546e7a; font-style: italic; }
.code-function { color: #82aaff; }
.code-number { color: #f78c6c; }

/* ===== 深视图右侧 ===== */
.deep-right {
  width: 260px;
  background: var(--bg-tertiary);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
}

.deep-panel {
  padding: 14px;
  border-bottom: 1px solid var(--border);
}

.deep-panel-flex {
  flex: 1;
  overflow-y: auto;
  border-bottom: none;
}

.deep-panel-title {
  font-size: 12px;
  font-weight: 600;
  margin-bottom: 10px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: var(--text-primary);
}

.deep-panel-title .more {
  font-size: 11px;
  color: var(--accent-light);
  cursor: pointer;
  font-weight: 400;
}

/* ===== 深视图任务 ===== */
.deep-tasks {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.deep-task {
  padding: 6px 8px;
  background: var(--bg-card);
  border-radius: 6px;
  display: flex;
  gap: 6px;
  align-items: flex-start;
  font-size: 11px;
  color: var(--text-secondary);
}

.deep-task.done .deep-task-text {
  text-decoration: line-through;
  color: var(--text-muted);
}

.deep-task-check {
  width: 14px;
  height: 14px;
  border: 1.5px solid var(--border);
  border-radius: 3px;
  flex-shrink: 0;
  margin-top: 1px;
  cursor: pointer;
  transition: all 0.2s;
}

.deep-task-check.checked {
  background: var(--success);
  border-color: var(--success);
}

.deep-task-text {
  flex: 1;
  line-height: 1.4;
}

/* ===== AI 建议 ===== */
.ai-suggestion {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.7;
  padding: 10px;
  background: var(--accent-dim);
  border-radius: 8px;
  border: 1px solid rgba(99, 102, 241, 0.2);
}

.ai-suggestion code {
  background: var(--bg-tertiary);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 11px;
  font-family: monospace;
}

.ai-suggestion-actions {
  margin-top: 10px;
  display: flex;
  gap: 6px;
}

/* ===== 实时预览 ===== */
.live-preview {
  background: var(--bg-card);
  border-radius: 8px;
  padding: 12px;
  font-size: 11px;
  border: 1px solid var(--border);
}

.live-status {
  color: var(--success);
  margin-bottom: 6px;
  font-weight: 500;
}

.live-metrics {
  color: var(--text-muted);
  line-height: 1.8;
}

/* ===== 滚动条 ===== */
.panel-list::-webkit-scrollbar,
.detail-content::-webkit-scrollbar,
.deep-editor::-webkit-scrollbar,
.deep-left::-webkit-scrollbar,
.deep-panel-flex::-webkit-scrollbar {
  width: 6px;
}

.panel-list::-webkit-scrollbar-track,
.detail-content::-webkit-scrollbar-track,
.deep-editor::-webkit-scrollbar-track,
.deep-left::-webkit-scrollbar-track,
.deep-panel-flex::-webkit-scrollbar-track {
  background: transparent;
}

.panel-list::-webkit-scrollbar-thumb,
.detail-content::-webkit-scrollbar-thumb,
.deep-editor::-webkit-scrollbar-thumb,
.deep-left::-webkit-scrollbar-thumb,
.deep-panel-flex::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 3px;
}

.panel-list::-webkit-scrollbar-thumb:hover,
.detail-content::-webkit-scrollbar-thumb:hover,
.deep-editor::-webkit-scrollbar-thumb:hover,
.deep-left::-webkit-scrollbar-thumb:hover,
.deep-panel-flex::-webkit-scrollbar-thumb:hover {
  background: var(--accent);
}
</style>
