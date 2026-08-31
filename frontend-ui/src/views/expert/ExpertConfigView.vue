<template>
  <div class="expert-config-page">
    <!-- 顶部标题栏 -->
    <div class="config-header">
      <div class="header-left">
        <div class="header-icon">
          <el-icon :size="22"><Setting /></el-icon>
        </div>
        <div class="header-titles">
          <h1 class="header-title">专家能力动态配置引擎</h1>
          <p class="header-subtitle">全维可配置 · 企业级低代码 · 实时预览 · 一键发布</p>
        </div>
      </div>
      <div class="header-right">
        <el-tag type="info" effect="plain" size="small">
          配置版本 v{{ configVersion }}
        </el-tag>
        <el-tag type="success" effect="dark" size="small" v-if="isDirty">
          <el-icon class="el-icon--left"><CircleCheck /></el-icon>
          未保存
        </el-tag>
      </div>
    </div>

    <!-- 主体区域 -->
    <div class="config-body">
      <!-- 左侧导航 -->
      <div class="config-sidebar">
        <div class="sidebar-nav">
          <div
            v-for="(item, idx) in navItems"
            :key="item.key"
            class="nav-item"
            :class="{ active: activeTab === item.key }"
            @click="activeTab = item.key"
          >
            <span class="nav-icon">{{ item.icon }}</span>
            <span class="nav-label">{{ item.label }}</span>
            <span class="nav-index">{{ String(idx + 1).padStart(2, '0') }}</span>
          </div>
        </div>

        <!-- 专家快速选择 -->
        <div class="sidebar-section">
          <div class="section-title">
            <el-icon><User /></el-icon>
            <span>当前专家</span>
          </div>
          <el-select v-model="selectedExpertId" class="expert-select" size="small">
            <el-option
              v-for="exp in expertList"
              :key="exp.id"
              :label="exp.name"
              :value="exp.id"
            />
          </el-select>
          <el-button size="small" class="new-expert-btn" @click="createNewExpert">
            <el-icon><Plus /></el-icon>
            新建专家配置
          </el-button>
        </div>

        <!-- 快捷操作 -->
        <div class="sidebar-section">
          <div class="section-title">
            <el-icon><MagicStick /></el-icon>
            <span>快捷操作</span>
          </div>
          <div class="quick-actions">
            <el-button size="small" plain @click="copyFromTemplate">
              <el-icon><DocumentCopy /></el-icon>
              从模板复制
            </el-button>
            <el-button size="small" plain @click="compareWithDefault">
              <el-icon><Comparison /></el-icon>
              对比默认配置
            </el-button>
          </div>
        </div>
      </div>

      <!-- 右侧配置内容区 -->
      <div class="config-content">
        <!-- 1. 基础画像配置 -->
        <div v-show="activeTab === 'profile'" class="tab-panel">
          <div class="panel-grid">
            <!-- 基本信息 -->
            <div class="config-card glass-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">1.1</span>基本信息</h3>
              </div>
              <div class="card-body">
                <el-form :model="config.profile" label-width="100px" size="default">
                  <el-form-item label="专家名称">
                    <el-input v-model="config.profile.name" placeholder="请输入专家名称" maxlength="32" show-word-limit />
                  </el-form-item>
                  <el-form-item label="专家类型">
                    <el-select v-model="config.profile.type" placeholder="请选择专家类型">
                      <el-option
                        v-for="(label, key) in expertTypes"
                        :key="key"
                        :label="label"
                        :value="key"
                      />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="头像 / Emoji">
                    <div class="avatar-config">
                      <div class="avatar-preview" :style="{ background: config.profile.themeColor + '20' }">
                        <span class="avatar-emoji">{{ config.profile.avatar }}</span>
                      </div>
                      <el-input v-model="config.profile.avatar" placeholder="输入 emoji 或上传头像" style="flex: 1" />
                      <el-upload class="avatar-uploader" :show-file-list="false" :before-upload="beforeAvatarUpload">
                        <el-button size="small" plain>
                          <el-icon><Upload /></el-icon>
                        </el-button>
                      </el-upload>
                    </div>
                  </el-form-item>
                  <el-form-item label="主题色">
                    <div class="color-config">
                      <el-color-picker v-model="config.profile.themeColor" size="default" />
                      <div class="preset-colors">
                        <span
                          v-for="c in presetColors"
                          :key="c"
                          class="preset-color"
                          :style="{ background: c }"
                          :class="{ active: config.profile.themeColor === c }"
                          @click="config.profile.themeColor = c"
                        />
                      </div>
                    </div>
                  </el-form-item>
                  <el-form-item label="专家等级">
                    <el-radio-group v-model="config.profile.level">
                      <el-radio value="junior">初级</el-radio>
                      <el-radio value="middle">中级</el-radio>
                      <el-radio value="senior">高级</el-radio>
                      <el-radio value="expert">专家级</el-radio>
                      <el-radio value="master">大师级</el-radio>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item label="专家简介">
                    <el-input
                      v-model="config.profile.description"
                      type="textarea"
                      :rows="3"
                      placeholder="请输入专家简介，描述其专业领域和擅长方向"
                      maxlength="200"
                      show-word-limit
                    />
                  </el-form-item>
                </el-form>
              </div>
            </div>

            <!-- 人格设定 -->
            <div class="config-card glass-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">1.2</span>人格设定</h3>
              </div>
              <div class="card-body">
                <el-form :model="config.profile.personality" label-width="100px" size="default">
                  <el-form-item label="性格特点">
                    <el-select
                      v-model="config.profile.personality.traits"
                      multiple
                      filterable
                      allow-create
                      placeholder="选择或输入性格特点"
                      style="width: 100%"
                    >
                      <el-option label="严谨认真" value="严谨认真" />
                      <el-option label="活泼开朗" value="活泼开朗" />
                      <el-option label="沉稳内敛" value="沉稳内敛" />
                      <el-option label="风趣幽默" value="风趣幽默" />
                      <el-option label="理性客观" value="理性客观" />
                      <el-option label="富有同理心" value="富有同理心" />
                      <el-option label="追求完美" value="追求完美" />
                      <el-option label="敢于挑战" value="敢于挑战" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="说话风格">
                    <el-select v-model="config.profile.personality.speechStyle" placeholder="选择说话风格">
                      <el-option label="正式专业" value="formal" />
                      <el-option label="亲切自然" value="friendly" />
                      <el-option label="幽默风趣" value="humorous" />
                      <el-option label="简洁干练" value="concise" />
                      <el-option label="详细全面" value="detailed" />
                      <el-option label="学术严谨" value="academic" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="专业背景">
                    <el-input
                      v-model="config.profile.personality.background"
                      type="textarea"
                      :rows="2"
                      placeholder="描述专家的专业背景和履历"
                    />
                  </el-form-item>
                  <el-form-item label="口头禅">
                    <el-select
                      v-model="config.profile.personality.catchphrases"
                      multiple
                      filterable
                      allow-create
                      placeholder="输入后回车添加"
                      style="width: 100%"
                    />
                  </el-form-item>
                </el-form>
              </div>
            </div>

            <!-- 视觉风格 -->
            <div class="config-card glass-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">1.3</span>视觉风格</h3>
              </div>
              <div class="card-body">
                <el-form :model="config.profile.visual" label-width="100px" size="default">
                  <el-form-item label="卡片背景色">
                    <el-color-picker v-model="config.profile.visual.cardBgColor" show-alpha />
                  </el-form-item>
                  <el-form-item label="文字颜色">
                    <el-color-picker v-model="config.profile.visual.textColor" />
                  </el-form-item>
                  <el-form-item label="图标风格">
                    <el-radio-group v-model="config.profile.visual.iconStyle">
                      <el-radio value="gradient">渐变</el-radio>
                      <el-radio value="flat">扁平</el-radio>
                      <el-radio value="outline">线框</el-radio>
                      <el-radio value="3d">3D 质感</el-radio>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item label="圆角大小">
                    <el-slider v-model="config.profile.visual.borderRadius" :min="0" :max="24" :step="2" show-input />
                  </el-form-item>
                  <el-form-item label="阴影强度">
                    <el-slider v-model="config.profile.visual.shadowIntensity" :min="0" :max="100" :step="5" show-input />
                  </el-form-item>
                </el-form>
              </div>
            </div>

            <!-- 实时预览 -->
            <div class="config-card glass-card preview-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">预览</span>实时效果</h3>
                <el-tag size="small" type="success" effect="plain">实时同步</el-tag>
              </div>
              <div class="card-body">
                <div
                  class="expert-preview-card"
                  :style="{
                    background: config.profile.visual.cardBgColor || '#ffffff',
                    color: config.profile.visual.textColor || '#1e293b',
                    borderRadius: (config.profile.visual.borderRadius || 12) + 'px',
                    boxShadow: `0 ${(config.profile.visual.shadowIntensity || 40) / 10}px ${(config.profile.visual.shadowIntensity || 40) / 5}px -${(config.profile.visual.shadowIntensity || 40) / 10}px rgba(0,0,0,0.15)`
                  }"
                >
                  <div class="preview-avatar" :style="{ background: config.profile.themeColor + '20' }">
                    <span class="preview-emoji">{{ config.profile.avatar || '🤖' }}</span>
                  </div>
                  <div class="preview-info">
                    <div class="preview-name-row">
                      <span class="preview-name">{{ config.profile.name || '专家名称' }}</span>
                      <el-tag
                        size="small"
                        :style="{ background: config.profile.themeColor, borderColor: config.profile.themeColor }"
                      >
                        {{ levelLabels[config.profile.level] || '等级' }}
                      </el-tag>
                    </div>
                    <div class="preview-type">{{ expertTypes[config.profile.type] || '专家类型' }}</div>
                    <div class="preview-desc">{{ config.profile.description || '专家简介将显示在这里...' }}</div>
                    <div class="preview-tags">
                      <el-tag
                        v-for="(t, i) in (config.profile.personality.traits || []).slice(0, 3)"
                        :key="i"
                        size="small"
                        effect="plain"
                        :style="{ borderColor: config.profile.themeColor + '50', color: config.profile.themeColor }"
                      >
                        {{ t }}
                      </el-tag>
                    </div>
                  </div>
                </div>

                <!-- 对话气泡预览 -->
                <div class="chat-preview">
                  <div class="chat-label">对话风格预览</div>
                  <div class="chat-bubble expert-bubble" :style="{ background: config.profile.themeColor + '15', borderColor: config.profile.themeColor + '30' }">
                    <span class="bubble-avatar" :style="{ background: config.profile.themeColor }">{{ config.profile.avatar?.charAt(0) || '🤖' }}</span>
                    <div class="bubble-content">
                      <div class="bubble-text">
                        你好！我是{{ config.profile.name || '专家' }}，很高兴为您服务。
                        <span v-if="config.profile.personality.catchphrases?.length" class="catchphrase">
                          {{ config.profile.personality.catchphrases[0] }}
                        </span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 2. 能力参数配置 -->
        <div v-show="activeTab === 'capability'" class="tab-panel">
          <div class="capability-layout">
            <!-- 能力开关矩阵 -->
            <div class="config-card glass-card capability-matrix-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">2.1</span>能力开关矩阵</h3>
                <div class="header-actions">
                  <el-button size="small" @click="toggleAllCapabilities(true)">全部启用</el-button>
                  <el-button size="small" @click="toggleAllCapabilities(false)">全部禁用</el-button>
                </div>
              </div>
              <div class="card-body">
                <el-table :data="config.capabilities.matrix" style="width: 100%" size="default">
                  <el-table-column prop="category" label="能力类别" width="140">
                    <template #default="{ row }">
                      <span class="cap-category">{{ row.category }}</span>
                    </template>
                  </el-table-column>
                  <el-table-column prop="name" label="能力项" width="140">
                    <template #default="{ row }">
                      <div class="cap-name">
                        <span class="cap-icon">{{ row.icon }}</span>
                        <span>{{ row.name }}</span>
                      </div>
                    </template>
                  </el-table-column>
                  <el-table-column prop="description" label="能力描述" min-width="180">
                    <template #default="{ row }">
                      <span class="cap-desc">{{ row.description }}</span>
                    </template>
                  </el-table-column>
                  <el-table-column prop="level" label="能力等级" width="120">
                    <template #default="{ row }">
                      <el-rate v-model="row.level" :max="5" size="small" @change="onCapabilityChange" />
                    </template>
                  </el-table-column>
                  <el-table-column label="启用" width="80" align="center">
                    <template #default="{ row }">
                      <el-switch v-model="row.enabled" @change="onCapabilityChange" />
                    </template>
                  </el-table-column>
                </el-table>
              </div>
            </div>

            <!-- 参数配置 -->
            <div class="config-card glass-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">2.2</span>模型参数配置</h3>
                <el-tag size="small" type="warning" effect="plain">影响生成质量</el-tag>
              </div>
              <div class="card-body">
                <el-form :model="config.capabilities.params" label-width="120px">
                  <el-form-item label="Temperature">
                    <el-slider v-model="config.capabilities.params.temperature" :min="0" :max="2" :step="0.1" show-input />
                    <div class="param-hint">控制输出随机性，值越高越有创意，越低越确定</div>
                  </el-form-item>
                  <el-form-item label="Top P">
                    <el-slider v-model="config.capabilities.params.topP" :min="0" :max="1" :step="0.05" show-input />
                    <div class="param-hint">核采样参数，控制候选词范围</div>
                  </el-form-item>
                  <el-form-item label="Max Tokens">
                    <el-input-number v-model="config.capabilities.params.maxTokens" :min="128" :max="32768" :step="256" />
                    <div class="param-hint">最大生成 token 数</div>
                  </el-form-item>
                  <el-form-item label="Presence Penalty">
                    <el-slider v-model="config.capabilities.params.presencePenalty" :min="-2" :max="2" :step="0.1" show-input />
                    <div class="param-hint">话题新颖度，正值鼓励引入新话题</div>
                  </el-form-item>
                  <el-form-item label="Frequency Penalty">
                    <el-slider v-model="config.capabilities.params.frequencyPenalty" :min="-2" :max="2" :step="0.1" show-input />
                    <div class="param-hint">重复度惩罚，正值减少重复内容</div>
                  </el-form-item>
                  <el-form-item label="上下文窗口">
                    <el-select v-model="config.capabilities.params.contextWindow">
                      <el-option label="4K (4096)" :value="4096" />
                      <el-option label="8K (8192)" :value="8192" />
                      <el-option label="16K (16384)" :value="16384" />
                      <el-option label="32K (32768)" :value="32768" />
                      <el-option label="128K (131072)" :value="131072" />
                    </el-select>
                  </el-form-item>
                </el-form>
              </div>
            </div>

            <!-- 工具权限 -->
            <div class="config-card glass-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">2.3</span>工具权限配置</h3>
                <el-tag size="small" type="info" effect="plain">{{ enabledToolsCount }}/{{ config.capabilities.tools.length }} 已启用</el-tag>
              </div>
              <div class="card-body">
                <div class="tools-grid">
                  <div
                    v-for="tool in config.capabilities.tools"
                    :key="tool.key"
                    class="tool-item"
                    :class="{ enabled: tool.enabled }"
                    @click="tool.enabled = !tool.enabled"
                  >
                    <div class="tool-icon">{{ tool.icon }}</div>
                    <div class="tool-info">
                      <div class="tool-name">{{ tool.name }}</div>
                      <div class="tool-desc">{{ tool.description }}</div>
                    </div>
                    <el-switch
                      v-model="tool.enabled"
                      @click.stop
                      size="small"
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 3. 提示词配置 -->
        <div v-show="activeTab === 'prompt'" class="tab-panel">
          <div class="prompt-layout">
            <!-- 左侧：编辑器 -->
            <div class="prompt-editor-area">
              <!-- System Prompt -->
              <div class="config-card glass-card">
                <div class="card-header">
                  <h3 class="card-title"><span class="title-badge">3.1</span>System Prompt（系统提示词）</h3>
                  <div class="header-actions">
                    <el-button size="small" @click="formatPrompt('system')">格式化</el-button>
                    <el-button size="small" type="primary" plain @click="insertVariable">
                      <el-icon><Plus /></el-icon>插入变量
                    </el-button>
                  </div>
                </div>
                <div class="card-body">
                  <div class="prompt-editor-wrapper">
                    <el-input
                      v-model="config.prompts.systemPrompt"
                      type="textarea"
                      :rows="14"
                      placeholder="请输入系统提示词，定义专家的角色、能力、行为准则..."
                      class="prompt-textarea"
                    />
                    <div class="prompt-stats">
                      <span>字符数: {{ config.prompts.systemPrompt.length }}</span>
                      <span>预估 Token: {{ Math.ceil(config.prompts.systemPrompt.length / 2) }}</span>
                    </div>
                  </div>
                </div>
              </div>

              <!-- 场景 Prompt 模板 -->
              <div class="config-card glass-card">
                <div class="card-header">
                  <h3 class="card-title"><span class="title-badge">3.2</span>场景 Prompt 模板</h3>
                  <el-button size="small" type="primary" plain @click="addScenePrompt">
                    <el-icon><Plus /></el-icon>新增场景
                  </el-button>
                </div>
                <div class="card-body">
                  <el-tabs v-model="activeSceneTab" type="card" class="scene-tabs">
                    <el-tab-pane
                      v-for="scene in config.prompts.scenes"
                      :key="scene.key"
                      :label="scene.name"
                      :name="scene.key"
                    >
                      <div class="scene-prompt-editor">
                        <div class="scene-meta">
                          <el-input v-model="scene.name" placeholder="场景名称" size="small" style="width: 160px" />
                          <el-input v-model="scene.description" placeholder="场景描述" size="small" style="flex: 1" />
                          <el-button size="small" type="danger" plain @click="removeScene(scene.key)">
                            <el-icon><Delete /></el-icon>
                          </el-button>
                        </div>
                        <el-input
                          v-model="scene.prompt"
                          type="textarea"
                          :rows="8"
                          :placeholder="`输入 ${scene.name} 场景的 Prompt 模板...`"
                          class="prompt-textarea"
                        />
                      </div>
                    </el-tab-pane>
                  </el-tabs>
                </div>
              </div>
            </div>

            <!-- 右侧：变量系统 + 测试 -->
            <div class="prompt-side-area">
              <!-- 可用变量 -->
              <div class="config-card glass-card">
                <div class="card-header">
                  <h3 class="card-title"><span class="title-badge">变量</span>可用变量列表</h3>
                  <el-button size="small" text type="primary" @click="showAddVar = true">
                    <el-icon><Plus /></el-icon>添加
                  </el-button>
                </div>
                <div class="card-body">
                  <div class="variables-list">
                    <div
                      v-for="v in config.prompts.variables"
                      :key="v.name"
                      class="variable-item"
                      @click="copyVariable(v.name)"
                    >
                      <code class="var-code">{{ '{{' + v.name + '}}' }}</code>
                      <span class="var-desc">{{ v.description }}</span>
                      <el-tag size="small" :type="v.type === 'system' ? 'info' : 'success'" effect="plain">
                        {{ v.type === 'system' ? '系统' : '自定义' }}
                      </el-tag>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Prompt 测试 -->
              <div class="config-card glass-card">
                <div class="card-header">
                  <h3 class="card-title"><span class="title-badge">测试</span>Prompt 测试面板</h3>
                  <el-tag size="small" type="warning" effect="plain">模拟运行</el-tag>
                </div>
                <div class="card-body">
                  <div class="prompt-test">
                    <el-input
                      v-model="testQuestion"
                      type="textarea"
                      :rows="3"
                      placeholder="输入测试问题，查看 Prompt 渲染效果..."
                    />
                    <div class="test-actions">
                      <el-select v-model="testScene" size="small" style="width: 140px">
                        <el-option label="系统提示词" value="system" />
                        <el-option
                          v-for="s in config.prompts.scenes"
                          :key="s.key"
                          :label="s.name"
                          :value="s.key"
                        />
                      </el-select>
                      <el-button type="primary" size="small" @click="runPromptTest" :loading="testing">
                        <el-icon><VideoPlay /></el-icon>运行测试
                      </el-button>
                    </div>
                    <div v-if="testResult" class="test-result">
                      <div class="result-label">渲染结果预览：</div>
                      <div class="result-content">{{ testResult }}</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 4. 工作流配置 -->
        <div v-show="activeTab === 'workflow'" class="tab-panel">
          <div class="workflow-layout">
            <!-- 工作流步骤列表 -->
            <div class="config-card glass-card workflow-main-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">4.1</span>专家调用工作流</h3>
                <div class="header-actions">
                  <el-button size="small" @click="addWorkflowStep">
                    <el-icon><Plus /></el-icon>添加步骤
                  </el-button>
                  <el-button size="small" type="primary" plain @click="resetWorkflow">
                    <el-icon><RefreshRight /></el-icon>重置默认
                  </el-button>
                </div>
              </div>
              <div class="card-body">
                <div class="workflow-steps">
                  <div
                    v-for="(step, idx) in config.workflow.steps"
                    :key="step.id"
                    class="workflow-step"
                    :class="{ disabled: !step.enabled }"
                  >
                    <div class="step-header">
                      <div class="step-order">{{ idx + 1 }}</div>
                      <div class="step-main">
                        <div class="step-title-row">
                          <el-input v-model="step.name" size="small" class="step-name-input" />
                          <el-switch v-model="step.enabled" size="small" />
                        </div>
                        <div class="step-desc">{{ step.description }}</div>
                      </div>
                      <div class="step-actions">
                        <el-button size="small" text @click="moveStep(idx, -1)" :disabled="idx === 0">
                          <el-icon><Top /></el-icon>
                        </el-button>
                        <el-button size="small" text @click="moveStep(idx, 1)" :disabled="idx === config.workflow.steps.length - 1">
                          <el-icon><Bottom /></el-icon>
                        </el-button>
                        <el-button size="small" text type="danger" @click="removeStep(idx)">
                          <el-icon><Delete /></el-icon>
                        </el-button>
                      </div>
                    </div>

                    <div v-if="step.enabled" class="step-config">
                      <el-form :model="step" label-width="100px" size="small" inline>
                        <el-form-item label="使用工具">
                          <el-select v-model="step.tools" multiple collapse-tags collapse-tags-tooltip style="width: 260px">
                            <el-option
                              v-for="t in config.capabilities.tools.filter(t => t.enabled)"
                              :key="t.key"
                              :label="t.name"
                              :value="t.key"
                            />
                          </el-select>
                        </el-form-item>
                        <el-form-item label="使用算子">
                          <el-select v-model="step.operators" multiple collapse-tags collapse-tags-tooltip style="width: 260px">
                            <el-option v-for="op in availableOperators" :key="op.key" :label="op.name" :value="op.key" />
                          </el-select>
                        </el-form-item>
                        <el-form-item label="超时时间">
                          <el-input-number v-model="step.timeout" :min="1" :max="300" />
                          <span class="unit">秒</span>
                        </el-form-item>
                        <el-form-item label="重试次数">
                          <el-input-number v-model="step.retry" :min="0" :max="5" />
                          <span class="unit">次</span>
                        </el-form-item>
                      </el-form>
                    </div>

                    <!-- 连接线 -->
                    <div v-if="idx < config.workflow.steps.length - 1" class="step-connector">
                      <div class="connector-line" />
                      <el-icon class="connector-arrow"><ArrowDown /></el-icon>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <!-- 条件分支 -->
            <div class="config-card glass-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">4.2</span>条件分支配置</h3>
                <el-button size="small" type="primary" plain @click="addCondition">
                  <el-icon><Plus /></el-icon>添加分支
                </el-button>
              </div>
              <div class="card-body">
                <div v-if="!config.workflow.conditions.length" class="empty-hint">
                  <el-empty description="暂无条件分支，点击上方按钮添加" :image-size="80" />
                </div>
                <div v-else class="conditions-list">
                  <div
                    v-for="(cond, idx) in config.workflow.conditions"
                    :key="cond.id"
                    class="condition-item"
                  >
                    <div class="cond-header">
                      <span class="cond-index">分支 {{ idx + 1 }}</span>
                      <el-button size="small" text type="danger" @click="removeCondition(idx)">
                        <el-icon><Delete /></el-icon>
                      </el-button>
                    </div>
                    <el-form :model="cond" label-width="80px" size="small">
                      <el-form-item label="触发条件">
                        <el-select v-model="cond.conditionType" style="width: 160px">
                          <el-option label="问题类型匹配" value="question_type" />
                          <el-option label="关键词匹配" value="keyword" />
                          <el-option label="复杂度评估" value="complexity" />
                          <el-option label="自定义表达式" value="custom" />
                        </el-select>
                      </el-form-item>
                      <el-form-item label="匹配值">
                        <el-input v-model="cond.matchValue" placeholder="输入匹配值或表达式" />
                      </el-form-item>
                      <el-form-item label="跳转步骤">
                        <el-select v-model="cond.targetStep" style="width: 200px">
                          <el-option
                            v-for="s in config.workflow.steps"
                            :key="s.id"
                            :label="`步骤${config.workflow.steps.indexOf(s) + 1}: ${s.name}`"
                            :value="s.id"
                          />
                        </el-select>
                      </el-form-item>
                    </el-form>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 5. 场景模板配置 -->
        <div v-show="activeTab === 'scenario'" class="tab-panel">
          <div class="scenario-layout">
            <!-- 场景列表 -->
            <div class="config-card glass-card scenario-list-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">5.1</span>预置场景模板</h3>
                <div class="header-actions">
                  <el-button size="small" @click="importScenario">
                    <el-icon><Upload /></el-icon>导入
                  </el-button>
                  <el-button size="small" @click="exportScenario">
                    <el-icon><Download /></el-icon>导出
                  </el-button>
                  <el-button size="small" type="primary" @click="addScenario">
                    <el-icon><Plus /></el-icon>新增
                  </el-button>
                </div>
              </div>
              <div class="card-body">
                <div class="scenario-list">
                  <div
                    v-for="(sc, idx) in config.scenarios"
                    :key="sc.id"
                    class="scenario-item"
                    :class="{ active: selectedScenario === sc.id }"
                    @click="selectedScenario = sc.id"
                  >
                    <div class="sc-icon">{{ sc.icon }}</div>
                    <div class="sc-info">
                      <div class="sc-name">{{ sc.name }}</div>
                      <div class="sc-desc">{{ sc.description }}</div>
                    </div>
                    <div class="sc-actions">
                      <el-button size="small" text @click.stop="editScenario(sc)">
                        <el-icon><Edit /></el-icon>
                      </el-button>
                      <el-button size="small" text type="danger" @click.stop="deleteScenario(idx)">
                        <el-icon><Delete /></el-icon>
                      </el-button>
                    </div>
                  </div>
                </div>
                <el-empty v-if="!config.scenarios.length" description="暂无场景模板" />
              </div>
            </div>

            <!-- 场景详情 -->
            <div class="config-card glass-card scenario-detail-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">5.2</span>场景详情配置</h3>
              </div>
              <div class="card-body">
                <div v-if="currentScenario">
                  <el-form :model="currentScenario" label-width="100px">
                    <el-form-item label="场景名称">
                      <el-input v-model="currentScenario.name" />
                    </el-form-item>
                    <el-form-item label="场景图标">
                      <el-input v-model="currentScenario.icon" placeholder="输入 emoji 图标" style="width: 120px" />
                    </el-form-item>
                    <el-form-item label="场景描述">
                      <el-input v-model="currentScenario.description" type="textarea" :rows="2" />
                    </el-form-item>
                    <el-form-item label="触发条件">
                      <el-input v-model="currentScenario.trigger" type="textarea" :rows="2" placeholder="描述触发该场景的条件" />
                    </el-form-item>
                    <el-form-item label="优先级">
                      <el-rate v-model="currentScenario.priority" :max="5" />
                    </el-form-item>
                    <el-divider content-position="left">配置覆盖</el-divider>
                    <el-form-item label="覆盖 System Prompt">
                      <el-switch v-model="currentScenario.override.systemPrompt" />
                    </el-form-item>
                    <el-form-item v-if="currentScenario.override.systemPrompt" label="自定义 Prompt">
                      <el-input v-model="currentScenario.overridePrompt" type="textarea" :rows="4" />
                    </el-form-item>
                    <el-form-item label="覆盖模型参数">
                      <el-switch v-model="currentScenario.override.params" />
                    </el-form-item>
                    <el-form-item label="覆盖工作流">
                      <el-switch v-model="currentScenario.override.workflow" />
                    </el-form-item>
                  </el-form>
                </div>
                <el-empty v-else description="请选择一个场景进行编辑" />
              </div>
            </div>
          </div>
        </div>

        <!-- 6. 评估与指标配置 -->
        <div v-show="activeTab === 'evaluation'" class="tab-panel">
          <div class="evaluation-layout">
            <!-- 评估维度权重 -->
            <div class="config-card glass-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">6.1</span>评估维度与权重</h3>
                <el-tag size="small" type="success" effect="plain">总权重: {{ totalWeight }}%</el-tag>
              </div>
              <div class="card-body">
                <div class="dimensions-list">
                  <div
                    v-for="dim in config.evaluation.dimensions"
                    :key="dim.key"
                    class="dimension-item"
                  >
                    <div class="dim-header">
                      <span class="dim-icon">{{ dim.icon }}</span>
                      <span class="dim-name">{{ dim.name }}</span>
                      <el-input-number
                        v-model="dim.weight"
                        :min="0"
                        :max="100"
                        size="small"
                        :controls="false"
                        class="dim-weight-input"
                      />
                      <span class="dim-weight-unit">%</span>
                    </div>
                    <el-slider
                      v-model="dim.weight"
                      :min="0"
                      :max="100"
                      :step="5"
                      show-input
                      :input-size="small"
                    />
                    <div class="dim-desc">{{ dim.description }}</div>
                  </div>
                </div>
              </div>
            </div>

            <!-- 自动评估规则 -->
            <div class="config-card glass-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">6.2</span>自动评估规则</h3>
                <el-button size="small" type="primary" plain @click="addAutoRule">
                  <el-icon><Plus /></el-icon>添加规则
                </el-button>
              </div>
              <div class="card-body">
                <el-table :data="config.evaluation.autoRules" style="width: 100%" size="small">
                  <el-table-column prop="name" label="规则名称" width="140" />
                  <el-table-column prop="type" label="规则类型" width="120">
                    <template #default="{ row }">
                      <el-tag size="small" :type="ruleTypeTag(row.type)">{{ ruleTypeLabel(row.type) }}</el-tag>
                    </template>
                  </el-table-column>
                  <el-table-column prop="pattern" label="匹配规则" min-width="180" />
                  <el-table-column prop="score" label="分值" width="100">
                    <template #default="{ row }">
                      <el-input-number v-model="row.score" :min="-100" :max="100" size="small" />
                    </template>
                  </el-table-column>
                  <el-table-column label="启用" width="80" align="center">
                    <template #default="{ row }">
                      <el-switch v-model="row.enabled" size="small" />
                    </template>
                  </el-table-column>
                  <el-table-column label="操作" width="80" align="center">
                    <template #default="{ $index }">
                      <el-button size="small" text type="danger" @click="removeAutoRule($index)">
                        <el-icon><Delete /></el-icon>
                      </el-button>
                    </template>
                  </el-table-column>
                </el-table>
              </div>
            </div>

            <!-- 人工评分模板 -->
            <div class="config-card glass-card">
              <div class="card-header">
                <h3 class="card-title"><span class="title-badge">6.3</span>人工评分模板</h3>
                <el-button size="small" type="primary" plain @click="addManualItem">
                  <el-icon><Plus /></el-icon>添加评分项
                </el-button>
              </div>
              <div class="card-body">
                <div class="manual-template-list">
                  <div
                    v-for="(item, idx) in config.evaluation.manualTemplate"
                    :key="item.id"
                    class="manual-item"
                  >
                    <div class="manual-item-header">
                      <span class="manual-index">评分项 {{ idx + 1 }}</span>
                      <el-button size="small" text type="danger" @click="removeManualItem(idx)">
                        <el-icon><Delete /></el-icon>
                      </el-button>
                    </div>
                    <el-form :model="item" label-width="80px" size="small" inline>
                      <el-form-item label="评分项">
                        <el-input v-model="item.name" placeholder="评分项名称" style="width: 160px" />
                      </el-form-item>
                      <el-form-item label="分值">
                        <el-input-number v-model="item.maxScore" :min="1" :max="100" style="width: 100px" />
                      </el-form-item>
                      <el-form-item label="评分标准">
                        <el-input v-model="item.criteria" placeholder="描述评分标准" style="width: 300px" />
                      </el-form-item>
                    </el-form>
                  </div>
                </div>
                <el-empty v-if="!config.evaluation.manualTemplate.length" description="暂无评分项" :image-size="60" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 底部操作栏 -->
    <div class="config-footer">
      <div class="footer-left">
        <el-button @click="resetToDefault">
          <el-icon><RefreshLeft /></el-icon>
          重置为默认
        </el-button>
        <el-button @click="importConfig">
          <el-icon><Upload /></el-icon>
          导入 JSON
        </el-button>
        <el-button @click="exportConfig">
          <el-icon><Download /></el-icon>
          导出 JSON
        </el-button>
      </div>
      <div class="footer-right">
        <el-button @click="previewConfig">
          <el-icon><View /></el-icon>
          预览效果
        </el-button>
        <el-button type="primary" @click="saveConfig" :loading="saving">
          <el-icon><Check /></el-icon>
          保存配置
        </el-button>
        <el-button type="success" @click="publishConfig" :loading="publishing">
          <el-icon><Promotion /></el-icon>
          发布上线
        </el-button>
      </div>
    </div>

    <!-- 导入对话框 -->
    <el-dialog v-model="importDialogVisible" title="导入配置 JSON" width="560px">
      <div class="import-dialog">
        <el-input
          v-model="importJsonText"
          type="textarea"
          :rows="12"
          placeholder="粘贴 JSON 配置内容，或上传文件..."
        />
        <div class="import-actions">
          <el-upload
            :show-file-list="false"
            :before-upload="beforeImportUpload"
            accept=".json"
          >
            <el-button>
              <el-icon><Upload /></el-icon>
              选择文件
            </el-button>
          </el-upload>
        </div>
      </div>
      <template #footer>
        <el-button @click="importDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="confirmImport">确认导入</el-button>
      </template>
    </el-dialog>

    <!-- 预览对话框 -->
    <el-dialog v-model="previewDialogVisible" title="配置效果预览" width="800px">
      <div class="preview-dialog">
        <el-tabs v-model="previewTab">
          <el-tab-pane label="专家卡片" name="card">
            <div class="preview-dialog-card">
              <div
                class="expert-preview-card large"
                :style="{
                  background: config.profile.visual.cardBgColor || '#ffffff',
                  color: config.profile.visual.textColor || '#1e293b',
                  borderRadius: (config.profile.visual.borderRadius || 12) + 'px',
                  boxShadow: `0 ${(config.profile.visual.shadowIntensity || 40) / 10}px ${(config.profile.visual.shadowIntensity || 40) / 5}px -${(config.profile.visual.shadowIntensity || 40) / 10}px rgba(0,0,0,0.15)`
                }"
              >
                <div class="preview-avatar" :style="{ background: config.profile.themeColor + '20' }">
                  <span class="preview-emoji">{{ config.profile.avatar || '🤖' }}</span>
                </div>
                <div class="preview-info">
                  <div class="preview-name-row">
                    <span class="preview-name">{{ config.profile.name || '专家名称' }}</span>
                    <el-tag size="small" :style="{ background: config.profile.themeColor, borderColor: config.profile.themeColor }">
                      {{ levelLabels[config.profile.level] || '等级' }}
                    </el-tag>
                  </div>
                  <div class="preview-type">{{ expertTypes[config.profile.type] || '专家类型' }}</div>
                  <div class="preview-desc">{{ config.profile.description || '专家简介...' }}</div>
                </div>
              </div>
            </div>
          </el-tab-pane>
          <el-tab-pane label="JSON 数据" name="json">
            <pre class="config-json-preview">{{ formattedConfigJson }}</pre>
          </el-tab-pane>
        </el-tabs>
      </div>
    </el-dialog>

    <!-- 隐藏的文件输入 -->
    <input ref="fileInputRef" type="file" accept=".json" style="display: none" @change="onFileChange" />
  </div>
</template>

<script setup>
import { ref, reactive, computed, watch, onMounted, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Setting, User, Plus, MagicStick, DocumentCopy, Comparison, Upload,
  Delete, Edit, VideoPlay, RefreshRight, Top, Bottom, ArrowDown,
  RefreshLeft, Download, View, Check, Promotion, CircleCheck,
  Folder
} from '@element-plus/icons-vue'
import { EXPERT_TYPES } from '@/constants/expert.constants'

// ========== 基础数据 ==========
const expertTypes = EXPERT_TYPES
const levelLabels = {
  junior: '初级',
  middle: '中级',
  senior: '高级',
  expert: '专家级',
  master: '大师级'
}

const presetColors = [
  '#6366f1', '#0ea5e9', '#10b981', '#f59e0b',
  '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4',
  '#84cc16', '#f97316', '#14b8a6', '#a855f7'
]

const navItems = [
  { key: 'profile', icon: '🎨', label: '基础画像配置' },
  { key: 'capability', icon: '🔧', label: '能力参数配置' },
  { key: 'prompt', icon: '💬', label: '提示词配置' },
  { key: 'workflow', icon: '🔄', label: '工作流配置' },
  { key: 'scenario', icon: '🎯', label: '场景模板配置' },
  { key: 'evaluation', icon: '📊', label: '评估与指标配置' }
]

// 可用算子列表（模拟）
const availableOperators = [
  { key: 'text_analyze', name: '文本分析算子' },
  { key: 'knowledge_retrieve', name: '知识检索算子' },
  { key: 'code_review', name: '代码审查算子' },
  { key: 'data_process', name: '数据处理算子' },
  { key: 'graph_traverse', name: '图谱遍历算子' },
  { key: 'reasoning_chain', name: '思维链算子' },
  { key: 'summary_gen', name: '摘要生成算子' },
  { key: 'quality_check', name: '质量检查算子' }
]

// ========== 状态 ==========
const activeTab = ref('profile')
const activeSceneTab = ref('daily')
const selectedExpertId = ref('default')
const selectedScenario = ref(null)
const configVersion = ref('1.0.0')
const isDirty = ref(false)
const saving = ref(false)
const publishing = ref(false)
const testing = ref(false)
const testQuestion = ref('')
const testScene = ref('system')
const testResult = ref('')
const showAddVar = ref(false)
const importDialogVisible = ref(false)
const importJsonText = ref('')
const previewDialogVisible = ref(false)
const previewTab = ref('card')
const fileInputRef = ref(null)

// 模拟专家列表
const expertList = ref([
  { id: 'default', name: '默认专家' },
  { id: 'arch', name: '架构师小明' },
  { id: 'algo', name: '算法专家' }
])

// ========== 配置数据结构 ==========
const defaultConfig = {
  profile: {
    name: '智能专家',
    type: 'ai',
    avatar: '🤖',
    themeColor: '#6366f1',
    level: 'senior',
    description: '一位全能的 AI 专家，擅长多领域问题分析与解决方案提供。',
    personality: {
      traits: ['严谨认真', '理性客观', '富有同理心'],
      speechStyle: 'friendly',
      background: '拥有 10 年以上 AI 研究经验，深耕自然语言处理和知识图谱领域',
      catchphrases: ['让我来分析一下', '根据我的理解']
    },
    visual: {
      cardBgColor: '#ffffff',
      textColor: '#1e293b',
      iconStyle: 'gradient',
      borderRadius: 12,
      shadowIntensity: 40
    }
  },
  capabilities: {
    matrix: [
      { category: '核心能力', icon: '💬', name: '对话咨询', description: '日常对话与问题解答', level: 5, enabled: true },
      { category: '核心能力', icon: '🔍', name: '深度分析', description: '深入分析复杂问题', level: 4, enabled: true },
      { category: '技术能力', icon: '📝', name: '代码审查', description: '代码质量审查与优化建议', level: 4, enabled: true },
      { category: '技术能力', icon: '📄', name: '文档生成', description: '自动生成各类技术文档', level: 3, enabled: true },
      { category: '分析能力', icon: '🕸️', name: '图谱分析', description: '知识图谱深度分析', level: 4, enabled: true },
      { category: '分析能力', icon: '📊', name: '数据处理', description: '数据清洗、分析与可视化', level: 3, enabled: false },
      { category: '设计能力', icon: '🧮', name: '算法设计', description: '算法设计与复杂度分析', level: 4, enabled: true },
      { category: '设计能力', icon: '🏗️', name: '架构评审', description: '系统架构设计评审', level: 4, enabled: true },
      { category: '质量能力', icon: '✅', name: '测试用例', description: '测试用例设计与生成', level: 3, enabled: false },
      { category: '质量能力', icon: '⚡', name: '性能优化', description: '性能瓶颈分析与优化', level: 3, enabled: false },
      { category: '安全能力', icon: '🔒', name: '安全审计', description: '安全漏洞检测与修复', level: 3, enabled: false },
      { category: '产品能力', icon: '📋', name: '需求分析', description: '需求梳理与产品设计', level: 4, enabled: true }
    ],
    params: {
      temperature: 0.7,
      topP: 0.9,
      maxTokens: 2048,
      presencePenalty: 0,
      frequencyPenalty: 0,
      contextWindow: 8192
    },
    tools: [
      { key: 'browser', icon: '🌐', name: '浏览器', description: '网页浏览与信息检索', enabled: true },
      { key: 'code_exec', icon: '💻', name: '代码执行', description: '运行和测试代码', enabled: true },
      { key: 'file_read', icon: '📂', name: '文件读取', description: '读取本地文件内容', enabled: true },
      { key: 'file_write', icon: '📝', name: '文件写入', description: '创建和修改文件', enabled: true },
      { key: 'web_search', icon: '🔎', name: '网络搜索', description: '互联网信息搜索', enabled: true },
      { key: 'knowledge_base', icon: '📚', name: '知识库', description: '企业知识库检索', enabled: true },
      { key: 'graph_query', icon: '🕸️', name: '图谱查询', description: '知识图谱查询分析', enabled: true },
      { key: 'mcp_tools', icon: '🔗', name: 'MCP 工具', description: 'MCP 协议扩展工具', enabled: false },
      { key: 'image_gen', icon: '🎨', name: '图像生成', description: 'AI 图像生成', enabled: false },
      { key: 'calculator', icon: '🧮', name: '计算器', description: '数学计算能力', enabled: true }
    ]
  },
  prompts: {
    systemPrompt: `你是一位专业的{{expert_type}}专家，名字叫{{expert_name}}。

## 角色定位
- 你拥有{{level}}级别的专业能力
- 你的性格特点：{{personality_traits}}
- 你的说话风格：{{speech_style}}

## 核心能力
你擅长以下领域：
{{capabilities}}

## 行为准则
1. 回答要专业、准确、有深度
2. 对于不确定的问题，要坦诚说明
3. 优先使用知识库和图谱数据
4. 复杂问题要分步骤分析
5. 给出建议时要说明理由和依据

## 输出格式
- 使用清晰的结构组织答案
- 重要内容加粗或列表展示
- 代码使用对应语言的代码块
- 引用来源要标注清楚`,
    scenes: [
      {
        key: 'daily',
        name: '日常咨询',
        description: '普通问题的日常对话场景',
        prompt: '用户正在咨询日常问题，请以友好、专业的态度回答。\n\n用户问题：{{question}}\n\n请给出清晰、实用的回答。'
      },
      {
        key: 'deep_analysis',
        name: '深度分析',
        description: '需要深度思考和分析的复杂问题',
        prompt: '用户提出了一个需要深度分析的问题，请按照以下步骤进行：\n\n1. 问题理解与拆解\n2. 核心要点分析\n3. 多维度论证\n4. 结论与建议\n\n用户问题：{{question}}\n\n请进行系统性的深度分析。'
      },
      {
        key: 'code_review',
        name: '代码审查',
        description: '代码质量审查与优化场景',
        prompt: '请对以下代码进行审查：\n\n代码语言：{{language}}\n代码内容：\n```{{language}}\n{{code}}\n```\n\n请从以下维度审查：\n1. 代码质量与规范\n2. 潜在 bug 与风险\n3. 性能优化建议\n4. 安全漏洞检查\n5. 可维护性评估'
      },
      {
        key: 'debate',
        name: '辩论场景',
        description: '多专家辩论与观点碰撞',
        prompt: '你正在参与一场专家辩论，议题是：{{topic}}\n\n对方观点：{{opponent_view}}\n\n请从你的专业角度出发：\n1. 指出对方观点的合理之处\n2. 提出你不同的看法和论据\n3. 给出更全面的结论'
      },
      {
        key: 'collaboration',
        name: '多专家协作',
        description: '多位专家协同解决问题',
        prompt: '你正在与其他专家协作解决问题。\n\n当前问题：{{question}}\n\n其他专家的观点：\n{{other_experts_views}}\n\n请基于以上信息，补充你的专业见解，力求全面覆盖各个角度。'
      }
    ],
    variables: [
      { name: 'expert_name', description: '专家名称', type: 'system' },
      { name: 'expert_type', description: '专家类型', type: 'system' },
      { name: 'level', description: '专家等级', type: 'system' },
      { name: 'personality_traits', description: '性格特点', type: 'system' },
      { name: 'speech_style', description: '说话风格', type: 'system' },
      { name: 'capabilities', description: '能力列表', type: 'system' },
      { name: 'question', description: '用户问题', type: 'system' },
      { name: 'user_input', description: '用户输入', type: 'custom' },
      { name: 'context', description: '上下文信息', type: 'custom' },
      { name: 'date', description: '当前日期', type: 'system' }
    ]
  },
  workflow: {
    steps: [
      { id: 's1', name: '理解问题', description: '分析用户意图，拆解问题要点', enabled: true, tools: ['knowledge_base'], operators: ['text_analyze'], timeout: 30, retry: 1 },
      { id: 's2', name: '知识检索', description: '从知识库和图谱中检索相关信息', enabled: true, tools: ['knowledge_base', 'graph_query', 'web_search'], operators: ['knowledge_retrieve', 'graph_traverse'], timeout: 60, retry: 2 },
      { id: 's3', name: '分析推理', description: '基于检索到的信息进行深度分析推理', enabled: true, tools: ['code_exec'], operators: ['reasoning_chain', 'text_analyze'], timeout: 120, retry: 1 },
      { id: 's4', name: '生成回答', description: '组织语言生成最终回答', enabled: true, tools: [], operators: ['summary_gen'], timeout: 60, retry: 1 },
      { id: 's5', name: '质量检查', description: '检查回答质量和准确性', enabled: false, tools: [], operators: ['quality_check'], timeout: 30, retry: 0 }
    ],
    conditions: []
  },
  scenarios: [
    {
      id: 'sc1',
      name: '代码审查模式',
      icon: '📝',
      description: '专注于代码质量审查的专家模式',
      trigger: '当用户提供代码并请求审查时',
      priority: 4,
      override: {
        systemPrompt: true,
        params: false,
        workflow: false
      },
      overridePrompt: '你是一位资深代码审查专家...'
    },
    {
      id: 'sc2',
      name: '架构设计模式',
      icon: '🏗️',
      description: '专注于系统架构设计评审',
      trigger: '当用户讨论系统架构时',
      priority: 4,
      override: {
        systemPrompt: true,
        params: false,
        workflow: true
      },
      overridePrompt: '你是一位资深架构师...'
    }
  ],
  evaluation: {
    dimensions: [
      { key: 'accuracy', icon: '🎯', name: '准确性', weight: 25, description: '回答内容的正确程度' },
      { key: 'relevance', icon: '🎯', name: '相关性', weight: 20, description: '回答与问题的相关程度' },
      { key: 'completeness', icon: '📋', name: '完整性', weight: 20, description: '回答覆盖问题的全面程度' },
      { key: 'professional', icon: '🎓', name: '专业性', weight: 20, description: '回答的专业深度和水平' },
      { key: 'speed', icon: '⚡', name: '响应速度', weight: 15, description: '回答的响应时间表现' }
    ],
    autoRules: [
      { id: 'r1', name: '代码块检测', type: 'keyword', pattern: '```', score: 5, enabled: true },
      { id: 'r2', name: '引用标注', type: 'keyword', pattern: '参考|来源|引用', score: 3, enabled: true },
      { id: 'r3', name: '长度不足', type: 'length', pattern: 'min:50', score: -10, enabled: true },
      { id: 'r4', name: '结构清晰', type: 'format', pattern: '列表|分级标题', score: 5, enabled: true }
    ],
    manualTemplate: [
      { id: 'm1', name: '内容准确性', maxScore: 25, criteria: '信息准确无误，没有事实性错误' },
      { id: 'm2', name: '逻辑清晰度', maxScore: 25, criteria: '论证逻辑清晰，层次分明' },
      { id: 'm3', name: '专业深度', maxScore: 25, criteria: '体现专业水平，有独到见解' },
      { id: 'm4', name: '实用价值', maxScore: 25, criteria: '回答具有实际指导意义' }
    ]
  }
}

// 深拷贝默认配置
const config = reactive(JSON.parse(JSON.stringify(defaultConfig)))

// ========== 计算属性 ==========
const enabledToolsCount = computed(() => config.capabilities.tools.filter(t => t.enabled).length)

const totalWeight = computed(() => config.evaluation.dimensions.reduce((sum, d) => sum + (d.weight || 0), 0))

const currentScenario = computed(() => config.scenarios.find(s => s.id === selectedScenario.value))

const formattedConfigJson = computed(() => JSON.stringify(config, null, 2))

// ========== 方法 ==========

// --- 通用 ---
function markDirty() {
  isDirty.value = true
}

// 监听配置变化
watch(config, () => {
  markDirty()
}, { deep: true })

// --- 基础画像 ---
function beforeAvatarUpload(file) {
  const isImage = file.type.startsWith('image/')
  if (!isImage) {
    ElMessage.warning('只能上传图片文件!')
    return false
  }
  // 模拟上传
  const reader = new FileReader()
  reader.onload = (e) => {
    config.profile.avatar = e.target.result
  }
  reader.readAsDataURL(file)
  return false
}

// --- 能力参数 ---
function toggleAllCapabilities(enabled) {
  config.capabilities.matrix.forEach(item => {
    item.enabled = enabled
  })
  ElMessage.success(enabled ? '已全部启用' : '已全部禁用')
}

function onCapabilityChange() {
  markDirty()
}

// --- 提示词配置 ---
function addScenePrompt() {
  const key = 'scene_' + Date.now().toString(36)
  config.prompts.scenes.push({
    key,
    name: '新场景',
    description: '场景描述',
    prompt: '请输入该场景的 Prompt 模板...'
  })
  activeSceneTab.value = key
  ElMessage.success('已添加新场景')
}

function removeScene(key) {
  const idx = config.prompts.scenes.findIndex(s => s.key === key)
  if (idx !== -1) {
    config.prompts.scenes.splice(idx, 1)
    if (activeSceneTab.value === key) {
      activeSceneTab.value = config.prompts.scenes[0]?.key || 'daily'
    }
    ElMessage.success('已删除场景')
  }
}

function insertVariable() {
  ElMessage.info('点击右侧变量列表可快速插入变量')
}

function copyVariable(name) {
  const text = '{{' + name + '}}'
  navigator.clipboard.writeText(text).then(() => {
    ElMessage.success(`已复制变量 ${text}`)
  }).catch(() => {
    ElMessage.info(`变量: ${text}`)
  })
}

function formatPrompt(type) {
  if (type === 'system') {
    // 简单格式化：去除多余空行
    config.prompts.systemPrompt = config.prompts.systemPrompt
      .replace(/\n{3,}/g, '\n\n')
      .trim()
    ElMessage.success('已格式化')
  }
}

function runPromptTest() {
  testing.value = true
  setTimeout(() => {
    let prompt = ''
    if (testScene.value === 'system') {
      prompt = config.prompts.systemPrompt
    } else {
      const scene = config.prompts.scenes.find(s => s.key === testScene.value)
      prompt = scene?.prompt || ''
    }
    // 模拟变量替换
    prompt = prompt
      .replace(/\{\{expert_name\}\}/g, config.profile.name)
      .replace(/\{\{expert_type\}\}/g, expertTypes[config.profile.type] || config.profile.type)
      .replace(/\{\{level\}\}/g, levelLabels[config.profile.level] || '')
      .replace(/\{\{question\}\}/g, testQuestion.value || '[用户问题]')
      .replace(/\{\{personality_traits\}\}/g, config.profile.personality.traits.join('、'))
      .replace(/\{\{capabilities\}\}/g, config.capabilities.matrix.filter(c => c.enabled).map(c => '- ' + c.name).join('\n'))
    testResult.value = prompt
    testing.value = false
    ElMessage.success('测试完成')
  }, 800)
}

// --- 工作流配置 ---
function addWorkflowStep() {
  const id = 's_' + Date.now().toString(36)
  config.workflow.steps.push({
    id,
    name: '新步骤',
    description: '步骤描述',
    enabled: true,
    tools: [],
    operators: [],
    timeout: 30,
    retry: 1
  })
  ElMessage.success('已添加步骤')
}

function removeStep(idx) {
  config.workflow.steps.splice(idx, 1)
  ElMessage.success('已删除步骤')
}

function moveStep(idx, direction) {
  const newIdx = idx + direction
  if (newIdx < 0 || newIdx >= config.workflow.steps.length) return
  const temp = config.workflow.steps[idx]
  config.workflow.steps.splice(idx, 1)
  config.workflow.steps.splice(newIdx, 0, temp)
}

function resetWorkflow() {
  config.workflow.steps = JSON.parse(JSON.stringify(defaultConfig.workflow.steps))
  config.workflow.conditions = []
  ElMessage.success('已重置为默认工作流')
}

function addCondition() {
  config.workflow.conditions.push({
    id: 'c_' + Date.now().toString(36),
    conditionType: 'question_type',
    matchValue: '',
    targetStep: config.workflow.steps[0]?.id || ''
  })
  ElMessage.success('已添加条件分支')
}

function removeCondition(idx) {
  config.workflow.conditions.splice(idx, 1)
}

// --- 场景模板配置 ---
function addScenario() {
  const id = 'sc_' + Date.now().toString(36)
  config.scenarios.push({
    id,
    name: '新场景',
    icon: '🎯',
    description: '场景描述',
    trigger: '触发条件',
    priority: 3,
    override: {
      systemPrompt: false,
      params: false,
      workflow: false
    },
    overridePrompt: ''
  })
  selectedScenario.value = id
  ElMessage.success('已添加场景')
}

function editScenario(sc) {
  selectedScenario.value = sc.id
}

function deleteScenario(idx) {
  ElMessageBox.confirm('确定删除该场景模板吗？', '确认删除', {
    type: 'warning'
  }).then(() => {
    const id = config.scenarios[idx].id
    config.scenarios.splice(idx, 1)
    if (selectedScenario.value === id) {
      selectedScenario.value = config.scenarios[0]?.id || null
    }
    ElMessage.success('已删除')
  }).catch(() => {})
}

function importScenario() {
  importDialogVisible.value = true
}

function exportScenario() {
  const data = JSON.stringify(config.scenarios, null, 2)
  downloadFile(data, `scenarios-${Date.now()}.json`)
  ElMessage.success('已导出场景配置')
}

// --- 评估配置 ---
function ruleTypeLabel(type) {
  const map = { keyword: '关键词', length: '长度检查', format: '格式检查', custom: '自定义' }
  return map[type] || type
}

function ruleTypeTag(type) {
  const map = { keyword: 'primary', length: 'warning', format: 'success', custom: 'info' }
  return map[type] || 'info'
}

function addAutoRule() {
  config.evaluation.autoRules.push({
    id: 'r_' + Date.now().toString(36),
    name: '新规则',
    type: 'keyword',
    pattern: '',
    score: 5,
    enabled: true
  })
}

function removeAutoRule(idx) {
  config.evaluation.autoRules.splice(idx, 1)
}

function addManualItem() {
  config.evaluation.manualTemplate.push({
    id: 'm_' + Date.now().toString(36),
    name: '新评分项',
    maxScore: 20,
    criteria: '评分标准描述'
  })
}

function removeManualItem(idx) {
  config.evaluation.manualTemplate.splice(idx, 1)
}

// --- 底部操作 ---
function saveConfig() {
  saving.value = true
  setTimeout(() => {
    saving.value = false
    isDirty.value = false
    configVersion.value = (parseFloat(configVersion.value) + 0.01).toFixed(3)
    ElMessage.success('配置已保存')
  }, 600)
}

function publishConfig() {
  ElMessageBox.confirm('确定发布当前配置吗？发布后将在线生效。', '发布确认', {
    type: 'info'
  }).then(() => {
    publishing.value = true
    setTimeout(() => {
      publishing.value = false
      isDirty.value = false
      ElMessage.success('🎉 配置已发布上线')
    }, 1000)
  }).catch(() => {})
}

function previewConfig() {
  previewDialogVisible.value = true
}

function resetToDefault() {
  ElMessageBox.confirm('确定重置为默认配置吗？所有修改将丢失。', '确认重置', {
    type: 'warning'
  }).then(() => {
    Object.assign(config, JSON.parse(JSON.stringify(defaultConfig)))
    isDirty.value = false
    ElMessage.success('已重置为默认配置')
  }).catch(() => {})
}

function importConfig() {
  importDialogVisible.value = true
  importJsonText.value = ''
}

function exportConfig() {
  const data = JSON.stringify(config, null, 2)
  downloadFile(data, `expert-config-${config.profile.name || 'default'}.json`)
  ElMessage.success('已导出配置 JSON')
}

function downloadFile(content, filename) {
  const blob = new Blob([content], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

function confirmImport() {
  try {
    const data = JSON.parse(importJsonText.value)
    Object.assign(config, data)
    isDirty.value = true
    importDialogVisible.value = false
    ElMessage.success('导入成功')
  } catch (e) {
    ElMessage.error('JSON 格式错误：' + e.message)
  }
}

function beforeImportUpload(file) {
  const reader = new FileReader()
  reader.onload = (e) => {
    importJsonText.value = e.target.result
  }
  reader.readAsText(file)
  return false
}

function onFileChange(e) {
  const file = e.target.files[0]
  if (file) {
    const reader = new FileReader()
    reader.onload = (ev) => {
      importJsonText.value = ev.target.result
    }
    reader.readAsText(file)
  }
}

// --- 专家管理 ---
function createNewExpert() {
  const name = '新专家_' + (expertList.value.length + 1)
  const id = 'expert_' + Date.now().toString(36)
  expertList.value.push({ id, name })
  selectedExpertId.value = id
  // 重置配置为默认
  Object.assign(config, JSON.parse(JSON.stringify(defaultConfig)))
  config.profile.name = name
  ElMessage.success('已创建新专家配置')
}

function copyFromTemplate() {
  ElMessage.info('模板复制功能：从已有专家配置复制')
}

function compareWithDefault() {
  ElMessage.info('对比功能：展示当前配置与默认配置的差异')
}

// ========== 初始化 ==========
onMounted(() => {
  // 默认选中第一个场景
  if (config.scenarios.length) {
    selectedScenario.value = config.scenarios[0].id
  }
})
</script>

<style scoped>
.expert-config-page {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background:
    radial-gradient(1200px 600px at 10% -10%, rgba(99, 102, 241, 0.08), transparent),
    radial-gradient(900px 500px at 100% 0%, rgba(16, 185, 129, 0.06), transparent),
    radial-gradient(800px 400px at 50% 100%, rgba(14, 165, 233, 0.05), transparent),
    #f8fafc;
  overflow: hidden;
}

/* 顶部标题栏 */
.config-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 24px;
  background: rgba(255, 255, 255, 0.75);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-bottom: 1px solid rgba(226, 232, 240, 0.8);
  flex-shrink: 0;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 14px;
}

.header-icon {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  display: grid;
  place-items: center;
  color: #fff;
  box-shadow: 0 6px 20px -6px rgba(99, 102, 241, 0.5);
}

.header-titles {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.header-title {
  margin: 0;
  font-size: 18px;
  font-weight: 800;
  background: linear-gradient(135deg, #6366f1, #0ea5e9 50%, #10b981);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  letter-spacing: 0.3px;
}

.header-subtitle {
  margin: 0;
  font-size: 12px;
  color: #64748b;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

/* 主体区域 */
.config-body {
  flex: 1;
  display: flex;
  min-height: 0;
  overflow: hidden;
}

/* 左侧导航 */
.config-sidebar {
  width: 240px;
  flex-shrink: 0;
  background: rgba(255, 255, 255, 0.6);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border-right: 1px solid rgba(226, 232, 240, 0.8);
  display: flex;
  flex-direction: column;
  padding: 16px 12px;
  gap: 16px;
  overflow-y: auto;
}

.sidebar-nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s ease;
  position: relative;
}

.nav-item:hover {
  background: rgba(99, 102, 241, 0.06);
}

.nav-item.active {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.12), rgba(14, 165, 233, 0.08));
  box-shadow: inset 3px 0 0 #6366f1;
}

.nav-item.active .nav-label {
  color: #4f46e5;
  font-weight: 700;
}

.nav-icon {
  font-size: 18px;
  width: 24px;
  text-align: center;
}

.nav-label {
  flex: 1;
  font-size: 13px;
  color: #334155;
  font-weight: 500;
}

.nav-index {
  font-size: 10px;
  color: #94a3b8;
  font-family: ui-monospace, Menlo, monospace;
}

.nav-item.active .nav-index {
  color: #6366f1;
  font-weight: 700;
}

.sidebar-section {
  border-top: 1px solid rgba(226, 232, 240, 0.8);
  padding-top: 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 700;
  color: #64748b;
  padding: 0 4px;
}

.expert-select {
  width: 100%;
}

.new-expert-btn {
  width: 100%;
  justify-content: center;
}

.quick-actions {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.quick-actions .el-button {
  width: 100%;
  justify-content: flex-start;
}

/* 右侧内容区 */
.config-content {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: 18px 22px;
}

.tab-panel {
  animation: fadeIn 0.3s ease;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: translateY(0); }
}

/* 玻璃拟态卡片 */
.glass-card {
  background: rgba(255, 255, 255, 0.75);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.8);
  border-radius: 16px;
  box-shadow:
    0 4px 24px -8px rgba(15, 23, 42, 0.08),
    0 0 0 1px rgba(226, 232, 240, 0.6) inset;
  overflow: hidden;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 18px;
  border-bottom: 1px solid rgba(226, 232, 240, 0.7);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.9), rgba(255, 255, 255, 0.6));
}

.card-title {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
  display: flex;
  align-items: center;
  gap: 8px;
}

.title-badge {
  display: inline-grid;
  place-items: center;
  min-width: 36px;
  height: 22px;
  padding: 0 8px;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
  font-size: 10.5px;
  font-weight: 700;
  border-radius: 6px;
  font-family: ui-monospace, Menlo, monospace;
}

.header-actions {
  display: flex;
  gap: 6px;
}

.card-body {
  padding: 16px 18px;
}

/* 1. 基础画像配置 - Grid 布局 */
.panel-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.preview-card {
  grid-column: span 1;
}

@media (max-width: 1200px) {
  .panel-grid {
    grid-template-columns: 1fr;
  }
}

/* 头像配置 */
.avatar-config {
  display: flex;
  align-items: center;
  gap: 10px;
}

.avatar-preview {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
}

.avatar-emoji {
  font-size: 26px;
}

.avatar-uploader {
  flex-shrink: 0;
}

/* 颜色配置 */
.color-config {
  display: flex;
  align-items: center;
  gap: 12px;
}

.preset-colors {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.preset-color {
  width: 22px;
  height: 22px;
  border-radius: 6px;
  cursor: pointer;
  border: 2px solid transparent;
  transition: all 0.2s ease;
}

.preset-color:hover {
  transform: scale(1.1);
}

.preset-color.active {
  border-color: #0f172a;
  box-shadow: 0 0 0 2px #fff, 0 0 0 4px currentColor;
}

/* 预览卡片 */
.expert-preview-card {
  padding: 18px;
  display: flex;
  gap: 14px;
  border: 1px solid rgba(226, 232, 240, 0.8);
  transition: all 0.3s ease;
}

.expert-preview-card.large {
  padding: 28px;
}

.preview-avatar {
  width: 56px;
  height: 56px;
  border-radius: 14px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
}

.preview-emoji {
  font-size: 30px;
}

.expert-preview-card.large .preview-avatar {
  width: 72px;
  height: 72px;
  border-radius: 18px;
}

.expert-preview-card.large .preview-emoji {
  font-size: 38px;
}

.preview-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.preview-name-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.preview-name {
  font-size: 16px;
  font-weight: 700;
}

.expert-preview-card.large .preview-name {
  font-size: 20px;
}

.preview-type {
  font-size: 12px;
  opacity: 0.7;
}

.preview-desc {
  font-size: 12.5px;
  line-height: 1.6;
  opacity: 0.8;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.preview-tags {
  display: flex;
  gap: 5px;
  flex-wrap: wrap;
  margin-top: 2px;
}

/* 对话预览 */
.chat-preview {
  margin-top: 16px;
  padding-top: 14px;
  border-top: 1px dashed rgba(226, 232, 240, 0.8);
}

.chat-label {
  font-size: 12px;
  font-weight: 600;
  color: #64748b;
  margin-bottom: 10px;
}

.chat-bubble {
  display: flex;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid;
}

.bubble-avatar {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 14px;
  flex-shrink: 0;
}

.bubble-content {
  flex: 1;
  min-width: 0;
}

.bubble-text {
  font-size: 13px;
  line-height: 1.6;
}

.catchphrase {
  color: #6366f1;
  font-style: italic;
  margin-left: 4px;
}

/* 2. 能力参数配置 */
.capability-layout {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.capability-matrix-card {
  min-height: 0;
}

.cap-category {
  font-size: 12px;
  color: #6366f1;
  font-weight: 600;
  background: rgba(99, 102, 241, 0.1);
  padding: 2px 8px;
  border-radius: 6px;
}

.cap-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  color: #0f172a;
}

.cap-icon {
  font-size: 16px;
}

.cap-desc {
  font-size: 12px;
  color: #64748b;
}

.param-hint {
  font-size: 11.5px;
  color: #94a3b8;
  margin-top: -4px;
}

/* 工具权限网格 */
.tools-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.tool-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid #e2e8f0;
  background: #fff;
  cursor: pointer;
  transition: all 0.2s ease;
}

.tool-item:hover {
  border-color: #c7d2fe;
  background: #fafbff;
}

.tool-item.enabled {
  border-color: #6366f1;
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.06), rgba(14, 165, 233, 0.04));
}

.tool-icon {
  font-size: 22px;
  width: 36px;
  text-align: center;
}

.tool-info {
  flex: 1;
  min-width: 0;
}

.tool-name {
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
}

.tool-desc {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
}

/* 3. 提示词配置 */
.prompt-layout {
  display: grid;
  grid-template-columns: minmax(0, 1.6fr) minmax(320px, 1fr);
  gap: 16px;
}

@media (max-width: 1200px) {
  .prompt-layout {
    grid-template-columns: 1fr;
  }
}

.prompt-editor-area {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.prompt-side-area {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.prompt-editor-wrapper {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.prompt-textarea :deep(.el-textarea__inner) {
  font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
  font-size: 13px;
  line-height: 1.7;
  border-radius: 10px;
}

.prompt-stats {
  display: flex;
  gap: 16px;
  font-size: 11.5px;
  color: #94a3b8;
  justify-content: flex-end;
}

.scene-tabs {
  margin-top: -4px;
}

.scene-prompt-editor {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.scene-meta {
  display: flex;
  gap: 8px;
  align-items: center;
}

/* 变量列表 */
.variables-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 240px;
  overflow-y: auto;
}

.variable-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 8px;
  background: #f8fafc;
  cursor: pointer;
  transition: all 0.15s ease;
}

.variable-item:hover {
  background: #eef2ff;
}

.var-code {
  font-family: ui-monospace, Menlo, monospace;
  font-size: 11.5px;
  color: #6366f1;
  background: #e0e7ff;
  padding: 2px 6px;
  border-radius: 4px;
  flex-shrink: 0;
}

.var-desc {
  flex: 1;
  font-size: 12px;
  color: #475569;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Prompt 测试 */
.prompt-test {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.test-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.test-result {
  margin-top: 4px;
}

.result-label {
  font-size: 12px;
  font-weight: 600;
  color: #64748b;
  margin-bottom: 6px;
}

.result-content {
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  padding: 10px 12px;
  font-size: 12.5px;
  line-height: 1.6;
  color: #334155;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 200px;
  overflow-y: auto;
}

/* 4. 工作流配置 */
.workflow-layout {
  display: grid;
  grid-template-columns: minmax(0, 1.4fr) minmax(320px, 1fr);
  gap: 16px;
}

@media (max-width: 1200px) {
  .workflow-layout {
    grid-template-columns: 1fr;
  }
}

.workflow-main-card {
  min-height: 0;
}

.workflow-steps {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.workflow-step {
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  overflow: hidden;
  transition: all 0.2s ease;
}

.workflow-step.disabled {
  opacity: 0.5;
}

.step-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: linear-gradient(180deg, #fafbfc, #f8fafc);
  border-bottom: 1px solid #e2e8f0;
}

.step-order {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
  display: grid;
  place-items: center;
  font-size: 13px;
  font-weight: 700;
  flex-shrink: 0;
}

.step-main {
  flex: 1;
  min-width: 0;
}

.step-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.step-name-input {
  max-width: 200px;
}

.step-desc {
  font-size: 11.5px;
  color: #64748b;
  margin-top: 3px;
}

.step-actions {
  display: flex;
  gap: 2px;
  flex-shrink: 0;
}

.step-config {
  padding: 12px 14px;
  background: #fff;
}

.unit {
  font-size: 12px;
  color: #64748b;
  margin-left: 4px;
}

.step-connector {
  display: flex;
  justify-content: center;
  padding: 4px 0;
  position: relative;
}

.connector-line {
  width: 2px;
  height: 16px;
  background: linear-gradient(180deg, #c7d2fe, #a5b4fc);
  border-radius: 2px;
}

.connector-arrow {
  position: absolute;
  bottom: -2px;
  color: #6366f1;
  font-size: 12px;
}

/* 条件分支 */
.conditions-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.condition-item {
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  padding: 12px;
  background: #fafbfc;
}

.cond-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.cond-index {
  font-size: 12px;
  font-weight: 700;
  color: #6366f1;
}

.empty-hint {
  padding: 20px 0;
}

/* 5. 场景模板配置 */
.scenario-layout {
  display: grid;
  grid-template-columns: 340px minmax(0, 1fr);
  gap: 16px;
}

@media (max-width: 1100px) {
  .scenario-layout {
    grid-template-columns: 1fr;
  }
}

.scenario-list-card {
  display: flex;
  flex-direction: column;
  max-height: calc(100vh - 200px);
}

.scenario-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 500px;
  overflow-y: auto;
}

.scenario-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid #e2e8f0;
  cursor: pointer;
  transition: all 0.2s ease;
  background: #fff;
}

.scenario-item:hover {
  border-color: #c7d2fe;
  background: #fafbff;
}

.scenario-item.active {
  border-color: #6366f1;
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.06), rgba(14, 165, 233, 0.04));
  box-shadow: inset 3px 0 0 #6366f1;
}

.sc-icon {
  font-size: 22px;
  width: 32px;
  text-align: center;
  flex-shrink: 0;
}

.sc-info {
  flex: 1;
  min-width: 0;
}

.sc-name {
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
}

.sc-desc {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sc-actions {
  display: flex;
  gap: 2px;
  flex-shrink: 0;
}

/* 6. 评估与指标配置 */
.evaluation-layout {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.dimensions-list {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.dimension-item {
  padding: 12px 14px;
  border-radius: 10px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
}

.dim-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}

.dim-icon {
  font-size: 18px;
}

.dim-name {
  flex: 1;
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
}

.dim-weight-input {
  width: 70px;
}

.dim-weight-unit {
  font-size: 12px;
  color: #64748b;
}

.dim-desc {
  font-size: 11.5px;
  color: #64748b;
  margin-top: 4px;
}

/* 人工评分模板 */
.manual-template-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.manual-item {
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  padding: 12px;
  background: #fafbfc;
}

.manual-item-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.manual-index {
  font-size: 12px;
  font-weight: 700;
  color: #0ea5e9;
}

/* 底部操作栏 */
.config-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 24px;
  background: rgba(255, 255, 255, 0.85);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-top: 1px solid rgba(226, 232, 240, 0.8);
  flex-shrink: 0;
}

.footer-left,
.footer-right {
  display: flex;
  gap: 8px;
}

/* 对话框 */
.import-dialog {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.import-actions {
  display: flex;
  justify-content: flex-end;
}

.preview-dialog-card {
  display: flex;
  justify-content: center;
  padding: 20px;
}

.config-json-preview {
  background: #0f172a;
  color: #e2e8f0;
  padding: 16px;
  border-radius: 10px;
  font-size: 12px;
  line-height: 1.6;
  max-height: 400px;
  overflow: auto;
  font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
}

/* 响应式调整 */
@media (max-width: 900px) {
  .config-sidebar {
    width: 60px;
    padding: 12px 6px;
  }
  .nav-label, .nav-index, .section-title, .expert-select, .new-expert-btn, .quick-actions {
    display: none;
  }
  .sidebar-section {
    border-top: none;
    padding-top: 0;
  }
}

/* 滚动条美化 */
:deep(.config-content::-webkit-scrollbar),
:deep(.config-sidebar::-webkit-scrollbar),
:deep(.scenario-list::-webkit-scrollbar),
:deep(.variables-list::-webkit-scrollbar) {
  width: 6px;
}

:deep(.config-content::-webkit-scrollbar-track),
:deep(.config-sidebar::-webkit-scrollbar-track),
:deep(.scenario-list::-webkit-scrollbar-track),
:deep(.variables-list::-webkit-scrollbar-track) {
  background: transparent;
}

:deep(.config-content::-webkit-scrollbar-thumb),
:deep(.config-sidebar::-webkit-scrollbar-thumb),
:deep(.scenario-list::-webkit-scrollbar-thumb),
:deep(.variables-list::-webkit-scrollbar-thumb) {
  background: rgba(148, 163, 184, 0.4);
  border-radius: 3px;
}

:deep(.config-content::-webkit-scrollbar-thumb:hover),
:deep(.config-sidebar::-webkit-scrollbar-thumb:hover),
:deep(.scenario-list::-webkit-scrollbar-thumb:hover),
:deep(.variables-list::-webkit-scrollbar-thumb:hover) {
  background: rgba(100, 116, 139, 0.6);
}
</style>
