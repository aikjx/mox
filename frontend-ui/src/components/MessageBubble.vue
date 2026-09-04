<template>
  <div class="mb-wrapper" :class="[`mb-${msg?.role || 'assistant'}`, { 'mb-system': msg?.system === true }]">
    <div v-if="msg?.system === true" class="mb-system-card">
      <div class="mb-system-header">
        <el-tag class="mb-system-badge" type="warning" effect="dark" size="small">
          <el-icon class="mb-badge-icon"><Bell /></el-icon>
          系统提示
        </el-tag>
        <span class="mb-system-time">{{ formatTime(msg?.timestamp) }}</span>
      </div>
      <div class="mb-system-body md-body" v-html="renderedContent" />
      <div v-if="msg?.task_id" class="mb-system-footer">
        <el-button type="primary" link size="small" @click="jumpToTask">
          <el-icon><Promotion /></el-icon>
          跳转任务 #{{ msg.task_id }}
        </el-button>
      </div>
      <!-- 系统消息底部动作（3 项） -->
      <div class="mb-actions mb-actions-system">
        <!-- 拆分式复制按钮：主按钮直接复制Markdown，下拉提供选项 -->
        <div class="mb-copy-split-wrap">
          <el-button class="mb-action-btn mb-action-primary mb-copy-main" circle size="small"
            aria-label="一键复制（Markdown）" title="一键复制（默认 Markdown）"
            @click="copyMarkdown()">
            <el-icon :size="14"><CopyDocument /></el-icon>
          </el-button>
          <el-dropdown trigger="click" @command="handleToolbarCopyCommand" :hide-on-click="true">
            <el-button class="mb-action-btn mb-action-primary mb-copy-caret" circle size="small" title="复制选项">
              <svg viewBox="0 0 24 24" width="9" height="9" fill="none" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
            </el-button>
            <template #dropdown>
              <el-dropdown-menu class="mb-dd-menu mb-dd-copy">
                <el-dropdown-item command="md-all" class="mb-dd-item">
                  <div class="mb-dd-row">
                    <span class="mb-dd-ic"><svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg></span>
                    <span class="mb-dd-text">整则 Markdown</span>
                    <span class="mb-dd-tag">默认</span>
                  </div>
                </el-dropdown-item>
                <el-dropdown-item command="plain-all" class="mb-dd-item">
                  <div class="mb-dd-row">
                    <span class="mb-dd-ic"><svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="8" y1="13" x2="16" y2="13"/><line x1="8" y1="17" x2="14" y2="17"/></svg></span>
                    <span class="mb-dd-text">整则纯文本</span>
                    <span class="mb-dd-tag mb-dd-tag-muted">TXT</span>
                  </div>
                </el-dropdown-item>
                <el-dropdown-item v-for="(m,i) in mermaidBlocks" :key="'mer-'+i" :command="'mer-'+i" divided class="mb-dd-item mb-dd-divider">
                  <div class="mb-dd-row">
                    <span class="mb-dd-ic"><el-icon :size="15"><Picture /></el-icon></span>
                    <span class="mb-dd-text">Mermaid 源码 <span class="mb-dd-sub">（{{i+1}}）</span></span>
                    <span class="mb-dd-tag mb-dd-tag-accent">SVG</span>
                  </div>
                </el-dropdown-item>
                <el-dropdown-item v-for="(f,i) in fenceBlocks" :key="'fen-'+i" :command="'fen-'+i" class="mb-dd-item">
                  <div class="mb-dd-row">
                    <span class="mb-dd-ic"><el-icon :size="15"><DocumentCopy /></el-icon></span>
                    <span class="mb-dd-text"><b class="mb-dd-lang">{{f.lang}}</b><span class="mb-dd-sub"> · {{f.lines}} 行</span></span>
                    <span class="mb-dd-tag mb-dd-tag-code">代码</span>
                  </div>
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
        <el-button class="mb-action-btn mb-tts" :class="{playing: speechState==='playing', paused: speechState==='paused'}" circle size="small"
          :title="speechState==='idle'?'朗读内容':(speechState==='playing'?'暂停朗读':'继续朗读')"
          @click="toggleSpeak">
          <svg v-if="speechState==='idle'" class="mb-ic-tts" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/></svg>
          <el-icon v-else :size="14"><component :is="speechState==='playing'?VideoPause:VideoPlay" /></el-icon>
        </el-button>
        <el-button v-if="msg?.task_id" class="mb-action-btn" circle size="small" title="跳转任务" @click="jumpToTask">
          <el-icon :size="14"><Promotion /></el-icon>
        </el-button>
        <span class="mb-sr-live" aria-live="polite">{{ speechState==='idle'?'':(speechState==='playing'?'正在朗读':'朗读已暂停') }}</span>
      </div>
    </div>
    <template v-else>
      <div class="mb-avatar-wrap">
        <div class="mb-avatar" :class="avatarCls">
          <el-icon v-if="msg?.role === 'user'" :size="20"><User /></el-icon>
          <el-icon v-else :size="20"><ChatDotRound /></el-icon>
        </div>
      </div>
      <div class="mb-bubble" @mouseenter="showOps = true" @mouseleave="showOps = false">
        <div class="mb-bubble-header">
          <span class="mb-sender">{{ senderName }}</span>
          <span class="mb-time">{{ formatTime(msg?.timestamp) }}</span>
          <el-tag
            v-if="msg?.confidence != null && Number.isFinite(msg.confidence)"
            class="mb-confidence" size="small" :type="confidenceTagType" effect="plain" round>
            置信度 {{ confidenceText }}
          </el-tag>
        </div>
        <div class="mb-ops" :class="{ 'mb-ops-show': showOps }">
          <!-- 拆分式：主按钮直接复制Markdown，下拉选择格式 -->
          <div class="mb-ops-split">
            <el-button class="mb-op-btn mb-ops-main" circle size="small" title="复制（默认 Markdown）" @click="copyMarkdown()">
              <el-icon :size="14"><CopyDocument /></el-icon>
            </el-button>
            <el-dropdown trigger="click" @command="handleCopyCommand" :hide-on-click="true">
              <el-button class="mb-op-btn mb-ops-caret" circle size="small" title="复制选项">
                <svg viewBox="0 0 24 24" width="9" height="9" fill="none" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
              </el-button>
              <template #dropdown>
                <el-dropdown-menu class="mb-dd-menu mb-dd-quick">
                  <el-dropdown-item command="markdown" class="mb-dd-item">
                    <div class="mb-dd-row">
                      <span class="mb-dd-ic"><svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg></span>
                      <span class="mb-dd-text">复制为 Markdown</span>
                      <span class="mb-dd-tag">默认</span>
                    </div>
                  </el-dropdown-item>
                  <el-dropdown-item command="plaintext" class="mb-dd-item">
                    <div class="mb-dd-row">
                      <span class="mb-dd-ic"><svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="8" y1="13" x2="16" y2="13"/><line x1="8" y1="17" x2="14" y2="17"/></svg></span>
                      <span class="mb-dd-text">复制为纯文本</span>
                      <span class="mb-dd-tag mb-dd-tag-muted">TXT</span>
                    </div>
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
        </div>
        <div class="mb-bubble-body md-body" v-html="renderedContent" />
        <div v-if="hasMeta" class="mb-meta">
          <div v-if="msg?.referenced_operators && msg.referenced_operators.length" class="mb-meta-section">
            <div class="mb-meta-title">
              <el-icon :size="13"><Cpu /></el-icon>
              <span>引用算子</span>
            </div>
            <div class="mb-meta-chips">
              <el-tag v-for="(op, i) in msg.referenced_operators" :key="i" size="small" effect="light" type="primary" class="mb-chip">
                <el-icon class="mb-chip-ic"><SetUp /></el-icon>
                {{ op }}
              </el-tag>
            </div>
          </div>
          <div v-if="msg?.web_search && (msg.web_search.sources?.length || msg.web_search.error)" class="mb-meta-section">
            <div class="mb-meta-title">
              <el-icon :size="13"><Link /></el-icon>
              <span>联网检索（{{ msg.web_search.engine || '' }} · {{ formatDuration(msg.web_search.duration_ms) }}）</span>
            </div>
            <div v-if="msg.web_search.error" class="mb-meta-section mb-error" style="margin-top:0;padding:8px 10px;">
              <div class="mb-error-body">
                <div class="mb-error-msg">
                  <el-icon :size="12" style="vertical-align:-1px;margin-right:4px;"><WarningFilled /></el-icon>
                  {{ msg.web_search.error }}
                </div>
              </div>
            </div>
            <ol v-if="msg.web_search.sources?.length" class="mb-meta-list mb-web-list">
              <li v-for="(w, i) in msg.web_search.sources" :key="i" class="mb-web-item">
                <span class="mb-web-idx">{{ i + 1 }}.</span>
                <a class="mb-web-link" :href="safeUrl(w.url)" target="_blank" rel="noopener noreferrer nofollow" :title="w.title || w.url">
                  {{ w.title || w.url }}
                </a>
              </li>
            </ol>
          </div>
          <div v-if="msg?.artifacts && (msg.artifacts.created?.length || msg.artifacts.skipped?.length)" class="mb-meta-section">
            <div class="mb-meta-title">
              <el-icon :size="13"><FolderChecked /></el-icon>
              <span>{{ msg.artifacts.mode === 'code' ? '代码文件' : (msg.artifacts.mode === 'document' ? '文档文件' : '本地制品') }}（成功 {{ msg.artifacts.created?.length || 0 }} · 跳过 {{ msg.artifacts.skipped?.length || 0 }}）</span>
            </div>
            <ul v-if="msg.artifacts.created?.length" class="mb-meta-list mb-artifact-list">
              <li v-for="(a, i) in msg.artifacts.created" :key="'c'+i" class="mb-artifact-item">
                <el-icon class="mb-artifact-ic" :size="14">
                  <component :is="artifactIcon(a)" />
                </el-icon>
                <span class="mb-artifact-name" :title="a.path || a.filename">{{ a.filename || a.path }}</span>
                <span v-if="a.overwritten" class="mb-web-host" style="background:rgba(245,158,11,0.12);color:#b45309;">覆盖</span>
                <span v-if="a.size" class="mb-artifact-size mono">{{ formatSize(a.size) }}</span>
              </li>
            </ul>
            <div v-if="msg.artifacts.skipped?.length" class="mb-meta-section mb-error" style="margin-top:8px;padding:8px 10px;">
              <div class="mb-meta-title mb-error-title" style="font-size:11.5px;">
                <el-icon :size="12"><Warning /></el-icon>
                <span>已跳过（{{ msg.artifacts.skipped.length }}）</span>
              </div>
              <div class="mb-error-body" style="gap:4px;">
                <div v-for="(s, i) in msg.artifacts.skipped" :key="'s'+i" class="mb-error-msg" style="font-size:12px;line-height:1.6;">
                  <el-icon :size="11" style="vertical-align:-1px;margin-right:3px;"><WarningFilled /></el-icon>
                  <strong>{{ s.filename || s.path }}</strong>
                  <span style="color:#991b1b;margin-left:4px;">— {{ s.reason }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
        <!-- 9 动作工具栏 -->
        <div class="mb-actions">
          <!-- 1. 一键复制（默认直接复制Markdown，下拉可选Markdown/文本/Mermaid/代码） -->
          <div class="mb-copy-split-wrap">
            <el-button class="mb-action-btn mb-action-primary mb-copy-main" circle size="small"
              aria-label="一键复制（Markdown）" title="一键复制（默认 Markdown）"
              @click="copyMarkdown()">
              <el-icon :size="14"><CopyDocument /></el-icon>
            </el-button>
            <el-dropdown trigger="click" @command="handleToolbarCopyCommand" :hide-on-click="true">
              <el-button class="mb-action-btn mb-action-primary mb-copy-caret" circle size="small" title="复制选项">
                <svg viewBox="0 0 24 24" width="9" height="9" fill="none" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
              </el-button>
              <template #dropdown>
                <el-dropdown-menu class="mb-dd-menu mb-dd-copy">
                  <el-dropdown-item command="md-all" class="mb-dd-item">
                    <div class="mb-dd-row">
                      <span class="mb-dd-ic"><svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg></span>
                      <span class="mb-dd-text">整则 Markdown</span>
                      <span class="mb-dd-tag">默认</span>
                    </div>
                  </el-dropdown-item>
                  <el-dropdown-item command="plain-all" class="mb-dd-item">
                    <div class="mb-dd-row">
                      <span class="mb-dd-ic"><svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="8" y1="13" x2="16" y2="13"/><line x1="8" y1="17" x2="14" y2="17"/></svg></span>
                      <span class="mb-dd-text">整则纯文本</span>
                      <span class="mb-dd-tag mb-dd-tag-muted">TXT</span>
                    </div>
                  </el-dropdown-item>
                  <el-dropdown-item v-for="(m,i) in mermaidBlocks" :key="'mer-'+i" :command="'mer-'+i" divided class="mb-dd-item mb-dd-divider">
                    <div class="mb-dd-row">
                      <span class="mb-dd-ic"><el-icon :size="15"><Picture /></el-icon></span>
                      <span class="mb-dd-text">Mermaid 源码 <span class="mb-dd-sub">（{{i+1}}）</span></span>
                      <span class="mb-dd-tag mb-dd-tag-accent">SVG</span>
                    </div>
                  </el-dropdown-item>
                  <el-dropdown-item v-for="(f,i) in fenceBlocks" :key="'fen-'+i" :command="'fen-'+i" class="mb-dd-item">
                    <div class="mb-dd-row">
                      <span class="mb-dd-ic"><el-icon :size="15"><DocumentCopy /></el-icon></span>
                      <span class="mb-dd-text"><b class="mb-dd-lang">{{f.lang}}</b><span class="mb-dd-sub"> · {{f.lines}} 行</span></span>
                      <span class="mb-dd-tag mb-dd-tag-code">代码</span>
                    </div>
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
          <!-- 2. 朗读（扬声器 SVG + Pause/Play） -->
          <el-button class="mb-action-btn mb-tts" :class="{playing: speechState==='playing', paused: speechState==='paused'}" circle size="small"
            :disabled="!supportsSpeechSynthesis"
            :title="speechState==='idle'?'朗读内容':(speechState==='playing'?'暂停朗读':'继续朗读')"
            @click="toggleSpeak">
            <svg v-if="speechState==='idle'" class="mb-ic-tts" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/></svg>
            <el-icon v-else :size="14"><component :is="speechState==='playing'?VideoPause:VideoPlay" /></el-icon>
          </el-button>
          <!-- 3. 喜欢 · 大拇指向上（窄屏折叠） -->
          <el-button class="mb-action-btn mb-rate-like mb-action-collapsible" :class="{active: rating==='like'}" circle size="small"
            :aria-pressed="rating==='like'" :title="rating==='like'?'取消喜欢':'喜欢'" @click="toggleRating('like')">
            <svg class="mb-ic-thumb" viewBox="0 0 24 24" width="14" height="14" :fill="rating==='like'?'currentColor':'none'" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M7 10v12"/><path d="M15 5.88 14 10h5.83a2 2 0 0 1 1.92 2.56l-2.33 8A2 2 0 0 1 17.5 22H7V10l4.34-8.68A1.5 1.5 0 0 1 14 2a1 1 0 0 1 1 1Z"/></svg>
          </el-button>
          <!-- 4. 不喜欢 · 大拇指向下（窄屏折叠） -->
          <el-button class="mb-action-btn mb-rate-dislike mb-action-collapsible" :class="{active: rating==='dislike'}" circle size="small"
            :aria-pressed="rating==='dislike'" :title="rating==='dislike'?'取消不喜欢':'不喜欢'" @click="toggleRating('dislike')">
            <svg class="mb-ic-thumb" viewBox="0 0 24 24" width="14" height="14" :fill="rating==='dislike'?'currentColor':'none'" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="transform:rotate(180deg)"><path d="M7 10v12"/><path d="M15 5.88 14 10h5.83a2 2 0 0 1 1.92 2.56l-2.33 8A2 2 0 0 1 17.5 22H7V10l4.34-8.68A1.5 1.5 0 0 1 14 2a1 1 0 0 1 1 1Z"/></svg>
          </el-button>
          <!-- 5. 分享 → 复制 URL（多选对话框） -->
          <el-button class="mb-action-btn" circle size="small" title="分享 · 复制链接" @click="openShareDialog">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
          </el-button>
          <!-- 6. 重新生成（仅助手） -->
          <el-button v-if="msg.role==='assistant'" class="mb-action-btn mb-regen" :class="{'mb-regen-loading': regenLoading}"
            :loading="regenLoading" circle size="small" title="重新生成" @click="doRegenerate">
            <el-icon :size="14"><Refresh /></el-icon>
          </el-button>
          <!-- 7. 转文档编辑（窄屏折叠） -->
          <el-button class="mb-action-btn mb-action-collapsible" circle size="small" title="转文档编辑" @click="openDocDialog">
            <el-icon :size="14"><DocumentAdd /></el-icon>
          </el-button>
          <!-- 8. 收藏（窄屏折叠） -->
          <el-button class="mb-action-btn mb-fav mb-action-collapsible" :class="{active: favorited, 'mb-heart-beat': favBeat}" circle size="small"
            :title="favorited?'取消收藏':'收藏'" @click="toggleFavorite">
            <el-icon :size="14"><component :is="favorited?StarFilled:Star" /></el-icon>
          </el-button>
          <!-- 9. 追问 -->
          <el-button class="mb-action-btn" circle size="small" title="追问" @click="doFollowup">
            <el-icon :size="14"><ChatLineSquare /></el-icon>
          </el-button>
          <!-- 10. 反馈（窄屏折叠） -->
          <el-button class="mb-action-btn mb-action-collapsible" circle size="small" title="反馈" @click="fbDlgOpen = true">
            <el-icon :size="14"><Flag /></el-icon>
          </el-button>
          <!-- 11. 折叠 More（窄屏） -->
          <el-dropdown v-if="moreCollapsed" trigger="click" :hide-on-click="true">
            <el-button class="mb-action-btn mb-actions-more-btn" circle size="small" title="更多">
              <el-icon :size="14"><More /></el-icon>
            </el-button>
            <template #dropdown>
              <el-dropdown-menu class="mb-dd-menu mb-dd-more">
                <el-dropdown-item @click="toggleRating('like')" class="mb-dd-item">
                  <div class="mb-dd-row">
                    <span class="mb-dd-ic" :style="{color: rating==='like'?'#22c55e':'inherit'}">
                      <svg class="mb-ic-thumb" viewBox="0 0 24 24" width="15" height="15" :fill="rating==='like'?'currentColor':'none'" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M7 10v12"/><path d="M15 5.88 14 10h5.83a2 2 0 0 1 1.92 2.56l-2.33 8A2 2 0 0 1 17.5 22H7V10l4.34-8.68A1.5 1.5 0 0 1 14 2a1 1 0 0 1 1 1Z"/></svg>
                    </span>
                    <span class="mb-dd-text">{{ rating==='like'?'取消喜欢':'喜欢' }}</span>
                  </div>
                </el-dropdown-item>
                <el-dropdown-item @click="toggleRating('dislike')" class="mb-dd-item">
                  <div class="mb-dd-row">
                    <span class="mb-dd-ic" :style="{color: rating==='dislike'?'#ef4444':'inherit'}">
                      <svg class="mb-ic-thumb" viewBox="0 0 24 24" width="15" height="15" :fill="rating==='dislike'?'currentColor':'none'" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="transform:rotate(180deg)"><path d="M7 10v12"/><path d="M15 5.88 14 10h5.83a2 2 0 0 1 1.92 2.56l-2.33 8A2 2 0 0 1 17.5 22H7V10l4.34-8.68A1.5 1.5 0 0 1 14 2a1 1 0 0 1 1 1Z"/></svg>
                    </span>
                    <span class="mb-dd-text">{{ rating==='dislike'?'取消不喜欢':'不喜欢' }}</span>
                  </div>
                </el-dropdown-item>
                <el-dropdown-item @click="toggleFavorite" class="mb-dd-item mb-dd-divider" divided>
                  <div class="mb-dd-row">
                    <span class="mb-dd-ic" :style="{color: favorited?'#f59e0b':'inherit'}"><el-icon :size="15"><component :is="favorited?StarFilled:Star" /></el-icon></span>
                    <span class="mb-dd-text">{{ favorited?'取消收藏':'收藏' }}</span>
                  </div>
                </el-dropdown-item>
                <el-dropdown-item @click="openDocDialog" class="mb-dd-item">
                  <div class="mb-dd-row">
                    <span class="mb-dd-ic"><el-icon :size="15"><DocumentAdd /></el-icon></span>
                    <span class="mb-dd-text">转文档编辑</span>
                  </div>
                </el-dropdown-item>
                <el-dropdown-item @click="fbDlgOpen = true" class="mb-dd-item">
                  <div class="mb-dd-row">
                    <span class="mb-dd-ic"><el-icon :size="15"><Flag /></el-icon></span>
                    <span class="mb-dd-text">反馈</span>
                  </div>
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
          <span class="mb-sr-live" aria-live="polite">{{ speechState==='idle'?'':(speechState==='playing'?'正在朗读':'朗读已暂停') }}</span>
        </div>
      </div>
    </template>

    <!-- 转文档编辑对话框 -->
    <el-dialog v-model="docDlgOpen" title="转为文档编辑" width="680px" class="mb-doc-dialog" :close-on-click-modal="false">
      <el-tabs v-model="docTab">
        <el-tab-pane label="Markdown 编辑" name="edit">
          <el-input type="textarea" v-model="docContent" :rows="20" resize="vertical" placeholder="在此编辑 Markdown..." />
        </el-tab-pane>
        <el-tab-pane label="预览" name="preview">
          <div class="md-body" v-html="renderedDocContent"></div>
        </el-tab-pane>
      </el-tabs>
      <div class="doc-stats">字数 {{docContent.length}} · 估计阅读 {{Math.max(1, Math.ceil(docContent.length/500))}} 分钟</div>
      <template #footer>
        <el-button @click="docDlgOpen=false">取消</el-button>
        <el-button type="primary" @click="exportMarkdown">导出 Markdown</el-button>
        <el-button type="success" @click="submitAsKb">新建为云盘文档</el-button>
      </template>
    </el-dialog>

    <!-- 反馈对话框 -->
    <el-dialog v-model="fbDlgOpen" title="反馈问题" width="520px" class="mb-fb-dialog" :close-on-click-modal="false" @closed="resetFbForm">
      <el-form :model="fbForm" ref="fbFormRef" label-width="100px" size="default">
        <el-form-item label="反馈类型" required>
          <el-radio-group v-model="fbForm.type">
            <el-radio value="事实错误">事实错误</el-radio>
            <el-radio value="格式错乱">格式错乱</el-radio>
            <el-radio value="幻觉内容">幻觉或不合规</el-radio>
            <el-radio value="代码报错">代码块报错</el-radio>
            <el-radio value="图表报错">Mermaid 渲染报错</el-radio>
            <el-radio value="其他">其他</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="严重程度" required>
          <el-radio-group v-model="fbForm.severity">
            <el-radio value="轻微">轻微</el-radio>
            <el-radio value="一般">一般</el-radio>
            <el-radio value="严重">严重</el-radio>
            <el-radio value="阻塞">阻塞</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="详细描述">
          <el-input type="textarea" v-model="fbForm.description" :rows="4" maxlength="500" show-word-limit placeholder="请输入详细描述（可选）" />
        </el-form-item>
        <el-form-item label-width="0">
          <el-checkbox v-model="fbForm.includeContext">联系上下文一起发送</el-checkbox>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="fbDlgOpen=false">取消</el-button>
        <el-button type="primary" @click="submitFeedback">提交反馈</el-button>
      </template>
    </el-dialog>

    <!-- 分享对话框：复制URL，多选消息 -->
    <el-dialog v-model="shareDlgOpen" title="分享 · 生成可访问链接" width="600px" class="mb-share-dialog" :close-on-click-modal="false" @closed="resetShareDlg">
      <div class="share-scope-tip">
        <el-alert :closable="false" type="info" show-icon size="small"
          :title="'当前对话：仅分析当前消息 · 勾选下方多选可扩展范围'" />
      </div>

      <div class="share-list-head">
        <div class="share-list-title">
          <el-icon><ChatDotRound /></el-icon>
          <span>选择要分享的消息</span>
          <el-tag size="small" type="info" effect="plain" style="margin-left:8px">已选 {{ shareSelectedIds.size }} / {{ shareMessages.length }}</el-tag>
        </div>
        <div class="share-list-actions">
          <el-button size="small" type="primary" link @click="selectAllShare">全选</el-button>
          <el-button size="small" link @click="invertShareSelection">反选</el-button>
          <el-button size="small" link @click="selectCurrentOnly">仅当前</el-button>
        </div>
      </div>

      <div class="share-list-box">
        <div v-if="!shareMessages.length" class="share-empty">
          <el-empty description="暂无消息可分享" :image-size="60" />
        </div>
        <div v-else class="share-list">
          <label v-for="sm in shareMessages" :key="sm.id" class="share-item"
            :class="{ 'si-current': sm.id === shareCurrentId, 'si-selected': shareSelectedIds.has(sm.id) }">
            <el-checkbox :model-value="shareSelectedIds.has(sm.id)"
              @change="(v)=>toggleShareSelect(sm.id, v)" />
            <div class="si-body">
              <div class="si-head">
                <el-tag size="small" :type="sm.role==='user'?'danger':(sm.role==='assistant'?'primary':'warning')" effect="light">
                  {{ sm.role==='user'?'我':(sm.role==='assistant'?'AI 助手':'系统') }}
                </el-tag>
                <span v-if="sm.id === shareCurrentId" class="si-current-tag">当前对话</span>
                <span class="si-time">{{ formatTime(sm.timestamp) }}</span>
              </div>
              <div class="si-preview">{{ sm.preview }}</div>
            </div>
          </label>
        </div>
      </div>

      <div class="share-link-section">
        <div class="share-link-label">
          <el-icon><Link /></el-icon>
          <span>分享链接（可直接访问对话快照）</span>
        </div>
        <div class="share-link-row">
          <el-input v-model="shareLinkUrl" readonly placeholder="请先勾选消息，生成链接..." />
          <el-button type="primary" :disabled="!shareSelectedIds.size" @click="generateShareLink">
            生成链接
          </el-button>
          <el-button :disabled="!shareLinkUrl" @click="copyShareLink">
            <el-icon><CopyDocument /></el-icon>
            复制
          </el-button>
        </div>
        <div v-if="shareLinkCreated" class="share-link-hint">
          ✅ 链接已生成 · 包含 {{ shareSelectedIds.size }} 条消息 · 可直接在浏览器访问查看快照
        </div>
      </div>

      <template #footer>
        <el-button @click="shareDlgOpen=false">关闭</el-button>
        <el-button type="success" :disabled="!shareLinkUrl" @click="finishShare">
          <el-icon><CopyDocument /></el-icon>
          复制链接并完成
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>
<script setup>
import { ref, computed, onBeforeUnmount, watch, nextTick, onMounted, getCurrentInstance } from "vue";
import { ElMessage } from "element-plus";
import {
  Bell, User, ChatDotRound, Promotion, CopyDocument, Document, DocumentCopy,
  Cpu, SetUp, Link, FolderChecked, WarningFilled, Warning, Picture,
  Microphone, VideoPlay, VideoPause, CircleCheckFilled, CircleCloseFilled, Share, Refresh,
  DocumentAdd, Star, StarFilled, ChatLineSquare, Flag, More
} from "@element-plus/icons-vue";
import MarkdownIt from "markdown-it";
import anchor from "markdown-it-anchor";
import taskLists from "markdown-it-task-lists";
import mermaid from "mermaid";
import { getVoiceHealth } from "../api/alliance";
import { getToken } from "@/utils/secureStorage";

const props = defineProps({
  msg: { type: Object, required: true },
  // 会话内全部消息（供分享多选），如不传则仅含当前消息
  sessionMessages: { type: Array, default: () => [] },
});
const emit = defineEmits(['goto-task','rate','share','regenerate','to-doc','favorite','followup','feedback']);

const showOps = ref(false);
const supportsClipboardItem = ref(false);
const mermaidRenderedIds = ref(new Set());
const instance = getCurrentInstance();

// 9 动作状态
const speechState = ref(/** @type {'idle'|'playing'|'paused'} */('idle'));
const supportsSpeechSynthesis = ref(false);
let speechUtterance = null;
let voicesReady = false;
const rating = ref(/** @type {null|'like'|'dislike'} */(null));
const favorited = ref(false);
const favBeat = ref(false);
const regenLoading = ref(false);
const moreCollapsed = ref(false);
const docDlgOpen = ref(false);
const docTab = ref('edit');
const docContent = ref('');
const fbDlgOpen = ref(false);
const fbFormRef = ref(null);
const fbForm = ref({ type: '', severity: '', description: '', includeContext: true });

// ============ 分享对话框 ============
const shareDlgOpen = ref(false);
const shareCurrentId = ref('');
const shareSelectedIds = ref(new Set());
const shareLinkUrl = ref('');
const shareLinkCreated = ref(false);
const shareMessages = computed(() => {
  const rawMsgs = (Array.isArray(props.sessionMessages) && props.sessionMessages.length)
    ? props.sessionMessages
    : [props.msg];
  return rawMsgs.map((m, idx) => {
    const id = m?.id != null ? String(m.id) : `idx-${idx}`;
    const content = String(m?.content ?? '');
    const preview = (content.length > 80 ? content.slice(0, 80) + '…' : content) || '（空消息）';
    const role = m?.role || (m?.system ? 'system' : 'assistant');
    return { id, raw: m, role, preview, timestamp: m?.timestamp };
  });
});

function openShareDialog() {
  const curId = stableMsgId(props.msg);
  shareCurrentId.value = curId;
  // 默认只选当前消息（符合"当前对话只分析当前的"）
  shareSelectedIds.value = new Set([curId]);
  shareLinkUrl.value = '';
  shareLinkCreated.value = false;
  shareDlgOpen.value = true;
}
function resetShareDlg() {
  shareSelectedIds.value = new Set();
  shareLinkUrl.value = '';
  shareLinkCreated.value = false;
  shareCurrentId.value = '';
}
function selectAllShare() {
  const all = new Set(shareMessages.value.map(sm => sm.id));
  shareSelectedIds.value = all;
}
function invertShareSelection() {
  const inv = new Set();
  shareMessages.value.forEach(sm => {
    if (!shareSelectedIds.value.has(sm.id)) inv.add(sm.id);
  });
  shareSelectedIds.value = inv;
}
function selectCurrentOnly() {
  shareSelectedIds.value = new Set([shareCurrentId.value]);
}
function toggleShareSelect(id, v) {
  const ns = new Set(shareSelectedIds.value);
  if (v) ns.add(id); else ns.delete(id);
  shareSelectedIds.value = ns;
}
function generateShareLink() {
  if (!shareSelectedIds.value.size) { ElMessage.warning('请至少选择一条消息'); return; }
  // 选取被选中的消息
  const selected = shareMessages.value.filter(sm => shareSelectedIds.value.has(sm.id));
  const selectedRaw = selected.map(sm => sm.raw);
  // 生成前端编码的分享 token（包含消息快照，可通过 URL hash 直接还原解析）
  const snapshot = {
    v: 1,
    ts: Date.now(),
    count: selectedRaw.length,
    msgs: selectedRaw.map(m => ({
      role: m?.role ?? (m?.system ? 'system' : 'assistant'),
      content: String(m?.content ?? ''),
      timestamp: m?.timestamp ?? Date.now(),
      senderName: m?.senderName || undefined,
      confidence: m?.confidence ?? undefined,
      system: m?.system ? true : undefined,
    })),
  };
  try {
    const b64 = btoa(unescape(encodeURIComponent(JSON.stringify(snapshot))));
    const base = typeof location !== 'undefined'
      ? `${location.origin}${location.pathname}${location.search}`
      : '/';
    const url = `${base}#/share/${b64}`;
    shareLinkUrl.value = url;
    shareLinkCreated.value = true;
    ElMessage.success({ message: `已生成分享链接（含 ${selectedRaw.length} 条消息）`, duration: 1500, showClose: false });
  } catch (e) {
    // dev only: 分享编码失败，已回退为当前页面链接并提示用户
    console.warn('[share] encode failed', e);
    // fallback：简单用当前 URL 加 hash
    shareLinkUrl.value = typeof location !== 'undefined' ? location.href : '';
    shareLinkCreated.value = true;
    ElMessage.warning('生成器编码失败，已回退为当前页面链接');
  }
}
async function copyShareLink() {
  if (!shareLinkUrl.value) { ElMessage.warning('尚未生成链接'); return; }
  const ok = await copyTextUniversal(shareLinkUrl.value, '', true);
  if (ok) ElMessage.success({ message: '分享链接已复制', duration: 1600, showClose: false });
  else ElMessage.error('复制失败，请手动复制');
}
async function finishShare() {
  await copyShareLink();
  shareDlgOpen.value = false;
  emit('share', props.msg);
}
// fence/mermaid 聚合统计（由 fence renderer 写入、watch renderedContent nextTick 回读 DOM）
const mermaidBlocks = ref([]);
const fenceBlocks = ref([]);

/* ---------- document-level 委派：代码块复制按钮（替代 inline onclick） ---------- */
const FENCE_COPY_ATTR = "data-mb-fence-copy";
let _fenceCopyListenerInstalled = false;
function _sbFeedback(btn, ok) {
  const okSvg =
    '<svg width="12" height="12" viewBox="0 0 24 24" fill="none"><path d="M20 6L9 17L4 12" stroke="#10b981" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  const errSvg =
    '<svg width="12" height="12" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="9" stroke="#ef4444" stroke-width="2.5"/><path d="M15 9L9 15M9 9L15 15" stroke="#ef4444" stroke-width="2.5" stroke-linecap="round"/></svg>';
  const origHtml = btn.innerHTML;
  btn.classList.add(ok ? "mb-fence-copy-ok" : "mb-fence-copy-err");
  btn.innerHTML = (ok ? okSvg : errSvg) + " " + (ok ? "已复制" : "复制失败");
  setTimeout(() => {
    btn.classList.remove("mb-fence-copy-ok", "mb-fence-copy-err");
    btn.innerHTML = origHtml;
  }, 1500);
}
function _fallbackCopy(text) {
  try {
    const ta = document.createElement("textarea");
    ta.value = text;
    ta.setAttribute("readonly", "");
    ta.style.position = "fixed";
    ta.style.left = "-9999px";
    document.body.appendChild(ta);
    ta.select();
    const ok = document.execCommand("copy");
    document.body.removeChild(ta);
    return ok;
  } catch (_) {
    return false;
  }
}
function installFenceCopyListener() {
  if (_fenceCopyListenerInstalled) return;
  _fenceCopyListenerInstalled = true;
  if (typeof document === "undefined") return;
  document.addEventListener("click", (e) => {
    const btn = e.target && e.target.closest && e.target.closest(`[${FENCE_COPY_ATTR}]`);
    if (!btn) return;
    e.preventDefault();
    e.stopPropagation();
    const fence = btn.closest(".mb-fence");
    const code = fence && fence.querySelector("pre code");
    if (!code) return;
    const text = code.innerText || code.textContent || "";
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(
        () => _sbFeedback(btn, true),
        () => _sbFeedback(btn, _fallbackCopy(text))
      );
    } else {
      _sbFeedback(btn, _fallbackCopy(text));
    }
  });
}

onMounted(() => {
  installFenceCopyListener();
  supportsClipboardItem.value = typeof window !== "undefined" && typeof ClipboardItem !== "undefined";
  try {
    mermaid.initialize({
      startOnLoad: false,
      theme: "base",
      securityLevel: "strict",
      fontFamily: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif',
      themeVariables: {
        primaryColor: "#eef2ff",
        primaryTextColor: "#312e81",
        primaryBorderColor: "#6366f1",
        lineColor: "#6366f1",
        secondaryColor: "#f8fafc",
        tertiaryColor: "#f5f3ff",
        actorBkg: "#eef2ff",
        actorBorder: "#6366f1",
        actorTextColor: "#312e81",
        actorLineColor: "#6366f1",
        noteBkgColor: "#fef3c7",
        noteBorderColor: "#f59e0b",
        noteTextColor: "#78350f",
        activationBorderColor: "#6366f1",
        activationBkgColor: "#e0e7ff",
        sequenceNumberColor: "#ffffff",
        signalTextColor: "#1e1b4b",
        labelBoxBkgColor: "#eef2ff",
        labelBoxBorderColor: "#6366f1",
        labelTextColor: "#312e81",
        loopTextColor: "#312e81",
        altBackground: "#f8fafc",
      },
      flowchart: { curve: "basis", padding: 14, nodeSpacing: 48, rankSpacing: 55, htmlLabels: true },
    });
  } catch (e) {
    // dev only: mermaid 初始化失败属内部渲染错误，不影响消息展示
    console.warn("[MessageBubble] mermaid init failed", e);
  }
  supportsSpeechSynthesis.value = typeof window !== 'undefined' && !!window.speechSynthesis;
  if (supportsSpeechSynthesis.value && typeof speechSynthesis.onvoiceschanged !== 'undefined') {
    const onVoices = () => { voicesReady = true; };
    speechSynthesis.addEventListener('voiceschanged', onVoices);
    try { onVoices(); } catch(_){}
  }
  // 初始化 rating/favorite 持久化读回
  const sid = stableMsgId(props.msg);
  try {
    const r = localStorage.getItem('ous_msg_rating_' + sid);
    if (r === 'like' || r === 'dislike') rating.value = r;
    const favsRaw = localStorage.getItem('ous_msg_favs');
    const favs = favsRaw ? JSON.parse(favsRaw) : [];
    favorited.value = Array.isArray(favs) && favs.includes(sid);
  } catch(_) {}
});

const senderName = computed(() => {
  if (props.msg?.role === "user") return props.msg?.senderName || "我";
  if (props.msg?.role === "assistant") return props.msg?.senderName || "AI 助手";
  return props.msg?.senderName || "系统";
});
const avatarCls = computed(() => ({
  "mb-avatar-user": props.msg?.role === "user",
  "mb-avatar-assistant": props.msg?.role === "assistant",
}));
const confidenceTagType = computed(() => {
  const c = Number(props.msg?.confidence);
  if (!Number.isFinite(c)) return "info";
  if (c >= 0.85) return "success";
  if (c >= 0.6) return "warning";
  return "danger";
});
const confidenceText = computed(() => {
  const c = Number(props.msg?.confidence);
  if (!Number.isFinite(c)) return "-";
  return `${Math.round(c * 100)}%`;
});
const hasMeta = computed(() => {
  const m = props.msg || {};
  return (
    (m.referenced_operators && m.referenced_operators.length) ||
    (m.web_search && (m.web_search.sources?.length || m.web_search.error)) ||
    (m.artifacts && (m.artifacts.created?.length || m.artifacts.skipped?.length))
  );
});

function escapeHtml(str) {
  return String(str)
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}
const mdInstance = computed(() => {
  const md = new MarkdownIt({ html: false, linkify: true, breaks: false, langPrefix: "language-", quotes: "\u201c\u201d\u2018\u2019" });
  md.use(anchor, {
    level: [1, 2, 3],
    permalink: false,
    slugify: (s) => String(s).trim().toLowerCase().replace(/[\s]+/g, "-").replace(/[^\w\u4e00-\u9fa5\-]/g, ""),
  });
  md.use(taskLists, { label: true, labelAfter: true, labelBefore: false });

  const defaultLinkOpen = md.renderer.rules.link_open || function (tokens, idx, options, _env, self) {
    return self.renderToken(tokens, idx, options);
  };
  md.renderer.rules.link_open = function (tokens, idx, options, env, self) {
    const t = tokens[idx];
    const hrefAttr = (t.attrs || []).find((a) => a[0] === "href");
    const href = hrefAttr ? String(hrefAttr[1]) : "";
    if (/^\s*javascript\s*:/i.test(href) || /^\s*data\s*:/i.test(href)) {
      if (hrefAttr) hrefAttr[1] = "#unsafe";
    } else {
      t.attrs = t.attrs || [];
      t.attrs.push(["target", "_blank"]);
      t.attrs.push(["rel", "noopener noreferrer nofollow"]);
    }
    return defaultLinkOpen(tokens, idx, options, env, self);
  };

  md.renderer.rules.fence = function (tokens, idx) {
    const token = tokens[idx];
    const rawInfo = token.info || "";
    const lang = rawInfo.trim().split(/\s+/)[0] || "";
    const content = token.content || "";
    const isMermaid = lang.toLowerCase() === "mermaid";
    const codeId = "mb-code-" + idx + "-" + Math.random().toString(36).slice(2, 8);
    const lines = content.split("\n");
    const lineCount = (lines.length > 1 && lines[lines.length - 1] === "") ? lines.length - 1 : lines.length;

    if (isMermaid) {
      const safeSource = escapeHtml(content);
      const renderId = "mermaid-" + codeId;
      const encoded = encodeURIComponent(content);
      return (
        '<div class="mb-mermaid-card" data-mermaid-id="' + renderId + '" data-src="' + encoded + '">' +
          '<div class="mb-mermaid-head">' +
            '<span class="mb-mermaid-badge">' +
              '<svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path d="M3 12L12 3L21 12L12 21L3 12Z" stroke="currentColor" stroke-width="2" stroke-linejoin="round"/><circle cx="12" cy="12" r="2.5" fill="currentColor"/></svg>' +
              'Mermaid 图表</span>' +
            '<span class="mb-mermaid-realtime"><span class="mb-rt-dot"></span>实时渲染</span>' +
          '</div>' +
          '<div class="mb-mermaid-body">' +
            '<div class="mb-mermaid-loading" data-ml="' + renderId + '">' +
              '<div class="mb-mermaid-spinner"></div><span>正在渲染图表\u2026</span>' +
            '</div>' +
            '<div class="mb-mermaid-target" id="' + renderId + '" data-mt="' + renderId + '" style="display:none"></div>' +
            '<div class="mb-mermaid-error" data-me="' + renderId + '" style="display:none">' +
              '<div class="mb-mermaid-error-head">' +
                '<svg width="16" height="16" viewBox="0 0 24 24" fill="none"><path d="M12 9V13M12 17H12.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" stroke="#ef4444" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>' +
                '<span>图表渲染失败</span></div>' +
              '<details class="mb-mermaid-error-details"><summary>查看 Mermaid 源码</summary><pre class="mb-mermaid-error-pre"><code>' + safeSource + '</code></pre></details>' +
            '</div>' +
          '</div>' +
          '<details class="mb-mermaid-source"><summary>Mermaid 源码</summary><pre class="mb-mermaid-source-pre"><code class="language-mermaid">' + safeSource + '</code></pre></details>' +
        '</div>'
      );
    }

    const safeLang = escapeHtml(lang || "text");
    const safeContent = escapeHtml(content);
    const copyBtn =
      '<button type="button" class="mb-fence-copy" title="复制代码" ' + FENCE_COPY_ATTR + '="1">' +
      '<svg width="12" height="12" viewBox="0 0 24 24" fill="none">' +
      '<rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" stroke-width="2"/>' +
      '<path d="M5 15H4C2.89543 15 2 14.1046 2 13V4C2 2.89543 2.89543 2 4 2H13C14.1046 2 15 2.89543 15 4V5" stroke="currentColor" stroke-width="2"/>' +
      '</svg> 复制</button>';

    return (
      '<div class="mb-fence" data-code-id="' + codeId + '">' +
        '<div class="mb-fence-head">' +
          '<div class="mb-fence-left"><span class="mb-fence-lang">' + safeLang + '</span><span class="mb-fence-lines">' + lineCount + ' \u884c</span></div>' +
          copyBtn +
        '</div>' +
        '<div class="mb-fence-code"><pre class="mb-fence-pre"><code class="language-' + safeLang + '">' + safeContent + '</code></pre></div>' +
      '</div>'
    );
  };
  return md;
});

const renderedContent = computed(() => {
  const content = props.msg?.content ?? "";
  if (!content) return '<p class="mb-empty">\uff08\u7a7a\u6d88\u606f\uff09</p>';
  try {
    return mdInstance.value.render(String(content));
  } catch (e) {
    // dev only: Markdown 渲染失败，降级为 pre 标签显示原文
    console.warn("[MessageBubble] markdown render error", e);
    return "<pre>" + escapeHtml(String(content)) + "</pre>";
  }
});
watch(renderedContent, async () => { await nextTick(); renderMermaidBlocks(); }, { flush: "post" });
onMounted(async () => { await nextTick(); renderMermaidBlocks(); });
onBeforeUnmount(() => { mermaidRenderedIds.value.clear(); });

async function renderMermaidBlocks() {
  const root = instance?.vnode?.el;
  if (!root) return;
  const cards = root.querySelectorAll(".mb-mermaid-card");
  if (!cards || !cards.length) return;
  for (const card of cards) {
    const renderId = card.getAttribute("data-mermaid-id");
    const source = decodeURIComponent(card.getAttribute("data-src") || "");
    if (!renderId || !source) continue;
    if (mermaidRenderedIds.value.has(renderId)) continue;
    const loadingEl = card.querySelector('[data-ml="' + renderId + '"]');
    const targetEl = card.querySelector('[data-mt="' + renderId + '"]');
    const errorEl = card.querySelector('[data-me="' + renderId + '"]');
    if (!targetEl || !loadingEl || !errorEl) continue;
    loadingEl.style.display = "flex";
    targetEl.style.display = "none";
    errorEl.style.display = "none";
    mermaidRenderedIds.value.add(renderId);
    try {
      const safeId = "mmd-" + renderId.replace(/[^a-zA-Z0-9_-]/g, "_") + "-" + Date.now().toString(36);
      const { svg } = await mermaid.render(safeId, source);
      targetEl.innerHTML = svg;
      // 为 render 产出的 <svg> 补齐 width/height（基于 viewBox 宽高比 + 目标宽度），
      // 避免某些容器（max-width:100% height:auto）在 width 缺省时测量为 0 导致不可见。
      const svgEl = targetEl.querySelector("svg");
      if (svgEl) {
        const vb = (svgEl.getAttribute("viewBox") || "").trim().split(/\s+/).map(Number);
        if (vb.length === 4 && isFinite(vb[2]) && vb[2] > 0 && isFinite(vb[3]) && vb[3] > 0) {
          const [, , w, h] = vb;
          // 以 width=100% 让 svg 随父宽自适应；height 根据 viewBox 宽高比显式给出 intrinsic ratio
          if (!svgEl.getAttribute("width")) svgEl.setAttribute("width", String(Math.round(w)));
          if (!svgEl.getAttribute("height")) svgEl.setAttribute("height", String(Math.round(h)));
          // 最终样式：响应式铺满可用宽度，高度按比例伸缩
          svgEl.style.width = "100%";
          svgEl.style.height = "auto";
          svgEl.style.maxWidth = "100%";
        }
      }
      targetEl.style.display = "block";
      loadingEl.style.display = "none";
      targetEl.classList.add("mb-mermaid-fade-in");
    } catch (err) {
      // dev only: mermaid 渲染失败，已显示错误占位
      console.warn("[MessageBubble] mermaid render failed", err);
      loadingEl.style.display = "none";
      targetEl.style.display = "none";
      errorEl.style.display = "block";
      if (err?.message) {
        try {
          const head = errorEl.querySelector(".mb-mermaid-error-head");
          if (head) {
            const span = document.createElement("span");
            span.className = "mb-mermaid-error-msg";
            span.textContent = err.message;
            head.appendChild(span);
          }
        } catch (_) {}
      }
    }
  }
}

function formatTime(ts) {
  if (!ts) return "";
  try {
    const d = ts instanceof Date ? ts : new Date(ts);
    if (isNaN(d.getTime())) return "";
    const now = new Date();
    const sameDay = d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth() && d.getDate() === now.getDate();
    const pad = (n) => String(n).padStart(2, "0");
    const tp = pad(d.getHours()) + ":" + pad(d.getMinutes());
    if (sameDay) return tp;
    return pad(d.getMonth() + 1) + "-" + pad(d.getDate()) + " " + tp;
  } catch (_) { return ""; }
}

function jumpToTask() { emit("goto-task", props.msg?.task_id); }

function handleCopyCommand(cmd) {
  if (cmd === "markdown") return copyMarkdown();
  if (cmd === "plaintext") return copyPlainText();
  if (cmd === "html") return copyRichHtml();
}
function _raw() { return String(props.msg?.content ?? ""); }

async function copyMarkdown() { await copyTextUniversal(_raw(), "Markdown \u5df2\u590d\u5236"); }
async function copyPlainText() { const t = mdToPlainText(_raw()); await copyTextUniversal(t, "\u7eaf\u6587\u672c\u5df2\u590d\u5236"); }
async function copyRichHtml() {
  const md = _raw();
  const html = mdInstance.value.render(md);
  const plain = mdToPlainText(md);
  const ok = await tryWriteRichClipboard(html, plain);
  if (ok) { ElMessage.success({ message: "\u5bcc\u6587\u672c\u5df2\u590d\u5236", duration: 1500, showClose: false }); return; }
  const ok2 = await copyTextUniversal(html, "HTML \u5df2\u590d\u5236\uff08\u964d\u7ea7\uff09", true);
  if (!ok2) ElMessage.error({ message: "\u590d\u5236\u5931\u8d25\uff0c\u8bf7\u624b\u52a8\u9009\u62e9", duration: 1500 });
}
async function tryWriteRichClipboard(htmlText, plainText) {
  if (!supportsClipboardItem.value) return false;
  if (!navigator.clipboard || !navigator.clipboard.write) return false;
  try {
    const item = new ClipboardItem({
      "text/html": new Blob([htmlText], { type: "text/html" }),
      "text/plain": new Blob([plainText], { type: "text/plain" }),
    });
    await navigator.clipboard.write([item]);
    return true;
  } catch (e) { /* dev only: clipboardItem 写入失败，回退到 execCommand */ console.warn("[MessageBubble] clipboardItem write failed", e); return false; }
}
async function copyTextUniversal(text, successMsg, silent) {
  try {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      await navigator.clipboard.writeText(text);
      if (!silent) ElMessage.success({ message: successMsg || "\u5df2\u590d\u5236", duration: 1500, showClose: false });
      return true;
    }
  } catch (e) { /* dev only: clipboard API 不可用，回退到 execCommand */ console.warn("[MessageBubble] clipboard API fallback", e); }
  try {
    const ta = document.createElement("textarea");
    ta.value = text; ta.setAttribute("readonly", "");
    ta.style.position = "fixed"; ta.style.left = "-9999px"; ta.style.top = "0";
    document.body.appendChild(ta); ta.select(); ta.setSelectionRange(0, ta.value.length);
    const ok = document.execCommand("copy");
    document.body.removeChild(ta);
    if (ok) {
      if (!silent) ElMessage.success({ message: successMsg || "\u5df2\u590d\u5236", duration: 1500, showClose: false });
      return true;
    }
    throw new Error("execCommand false");
  } catch (e) {
    if (!silent) ElMessage.error({ message: "\u590d\u5236\u5931\u8d25\uff0c\u8bf7\u624b\u52a8\u9009\u62e9", duration: 1500 });
    return false;
  }
}
function mdToPlainText(md) {
  if (!md) return "";
  let s = String(md);
  s = s.replace(/```([\s\S]*?)```/g, (_m, inner) => {
    const c = inner.replace(/^[^\n]*\n/, "");
    return "\n" + c.replace(/\n$/, "") + "\n";
  });
  s = s.replace(/`([^`]+)`/g, "$1");
  s = s.replace(/!\[[^\]]*\]\([^)]*\)/g, "");
  s = s.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_, t, u) => {
    const tt = (t || "").trim(), uu = (u || "").trim();
    if (!tt) return uu; if (!uu) return tt;
    return tt + " " + uu;
  });
  s = s.replace(/\[([^\]]+)\]\[[^\]]*\]/g, "$1");
  s = s.replace(/^\s{0,3}#{1,6}\s+/gm, "");
  s = s.replace(/\*\*([^*]+)\*\*/g, "$1");
  s = s.replace(/__([^_]+)__/g, "$1");
  s = s.replace(/(^|[^*])\*([^*\n]+)\*(?=[^*]|$)/g, "$1$2");
  s = s.replace(/(^|[^_])_([^_\n]+)_(?=[^_]|$)/g, "$1$2");
  s = s.replace(/~~([^~]+)~~/g, "$1");
  s = s.replace(/^\s*(?:[-*+])\s+\[( |x|X)\]\s+/gm, (_m, s1) => {
    return s1.trim().toLowerCase() === "x" ? "[x] " : "[ ] ";
  });
  s = s.replace(/^\s*(?:[-*+])\s+/gm, "\u2022 ");
  s = s.replace(/^\s*(\d+)\.\s+/gm, (_, n) => n + ". ");
  s = s.replace(/^\s{0,3}>\s?/gm, "");
  s = s.replace(/^\s*([-*_])\s*\1\s*\1[\s\S]*?$/gm, "");
  s = s.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&").replace(/&quot;/g, "\"").replace(/&#39;/g, "'");
  s = s.replace(/\n{3,}/g, "\n\n");
  return s.trim();
}

// ============ 稳定 ID（哈希 fallback） ============
function stableMsgId(m) {
  if (m?.id) return String(m.id);
  const s = String(m?.timestamp ?? 0) + '|' + String(m?.role ?? '') + '|' + String(m?.content ?? '').slice(0, 200);
  let h = 5381;
  for (let i = 0; i < s.length; i++) h = ((h << 5) + h) + s.charCodeAt(i);
  return 'mb-' + (h >>> 0).toString(36);
}
// ============ 工具栏复制（整则 / Mermaid / 代码块） ============
function handleToolbarCopyCommand(cmd) {
  if (cmd === 'md-all') return copyMarkdown();
  if (cmd === 'plain-all') return copyPlainText();
  if (typeof cmd === 'string') {
    if (cmd.startsWith('mer-')) {
      const i = Number(cmd.slice(4));
      const el = instance?.vnode?.el?.querySelectorAll?.('.mb-mermaid-card');
      if (el && el[i]) {
        const src = decodeURIComponent(el[i].getAttribute('data-src') || '');
        if (src) return copyTextUniversal(src, 'Mermaid 源码已复制');
      }
      const blk = mermaidBlocks.value[i];
      if (blk?.content) return copyTextUniversal(blk.content, 'Mermaid 源码已复制');
      return ElMessage.warning('未找到 Mermaid 源码');
    }
    if (cmd.startsWith('fen-')) {
      const i = Number(cmd.slice(4));
      const el = instance?.vnode?.el?.querySelectorAll?.('.mb-fence');
      if (el && el[i]) {
        const code = el[i].querySelector('pre code');
        const txt = code?.innerText ?? code?.textContent ?? '';
        if (txt) return copyTextUniversal(txt, '代码块已复制');
      }
      const blk = fenceBlocks.value[i];
      if (blk?.content) return copyTextUniversal(blk.content, '代码块已复制');
      return ElMessage.warning('未找到代码内容');
    }
  }
  return handleCopyCommand(cmd);
}
// ============ TTS 朗读 · 三层回退（T14 / AC-11 / AC-22）· 豆包级动听版 ============
/** 精选「豆包级拟人」中文音色。优先选微软/字节的高质量女声（晓晓/云希/云扬/晓伊），
 *  刻意规避劣质男声/机器人质感音色（如 晓康、kang、yankang、robot）。 */
const PREMIUM_ZH_VOICE = [
  // Edge/Windows 在线高质量神经 TTS（微软 Azure Speech 同源，最接近豆包拟人）
  'xiaoxiao', '晓晓', 'yunxi', '云希', 'xiaoyi', '晓伊', 'xiaohan', '晓涵',
  'xiaomo', '晓墨', 'xiaoshuang', '晓双', 'xiaorou', '晓柔', 'yunjian', '云健',
  'yunyang', '云扬', 'hiujaai', 'hiuji', 'hiusan',
  // macOS/iOS 优质中文
  'sinji', 'meijia', 'yaoyao', 'zhiyu',
  // 字节豆包 App 内 TTS（若被注入到 speechSynthesis）
  'doubao', 'bytedance',
  // 系统级常见较好
  'tingting', 'huihui',
]
/** 低质感黑名单：命中直接放弃该 voice */
const BLACKLIST_ZH_VOICE = [
  'xiaokang', '晓康', 'kang', 'robot', 'mechanical', 'tts_engine', 'default_speech',
]
function pickZhVoice() {
  try {
    const vs = speechSynthesis.getVoices ? speechSynthesis.getVoices() : []
    if (!vs || !vs.length) return null
    const list = Array.from(vs)
    const lowered = (s) => String(s || '').toLowerCase().replace(/[\s_-]/g, '')
    const zhOnly = list.filter(v => {
      const lang = (v.lang || '').toLowerCase()
      return lang.startsWith('zh') || lang.startsWith('yue') || lang.startsWith('cmn')
    })
    // 1) 优先命中 PREMIUM 白名单
    for (const key of PREMIUM_ZH_VOICE) {
      const k = lowered(key)
      const hit = zhOnly.find(v => {
        const n = lowered(v.name)
        const u = lowered(v.voiceURI)
        return (n.includes(k) || u.includes(k))
          && !BLACKLIST_ZH_VOICE.some(b => n.includes(lowered(b)) || u.includes(lowered(b)))
      })
      if (hit) return hit
    }
    // 2) 去掉黑名单里的中文 voice
    const safeZh = zhOnly.filter(v => {
      const n = lowered(v.name)
      const u = lowered(v.voiceURI)
      return !BLACKLIST_ZH_VOICE.some(b => n.includes(lowered(b)) || u.includes(lowered(b)))
    })
    // 3) 优先 neural / online / natural / premium 字样（Edge 标签）
    const prefer = ['neural', 'online', 'natural', 'premium', 'hd']
    for (const tag of prefer) {
      const h = safeZh.find(v => lowered(v.name).includes(tag) || lowered(v.voiceURI).includes(tag))
      if (h) return h
    }
    // 4) 优先女声（中文女名字中常见「晓/云/婷/瑶/涵/希」）
    const fem = safeZh.find(v => /晓|云|希|涵|瑶|婷|柔|双|伊/.test(v.name || ''))
    if (fem) return fem
    return safeZh[0] || zhOnly[0] || list[0] || null
  } catch(_) { return null }
}

function _header(resp, name) {
  try { return resp.headers.get(name) || '' } catch(_) { return '' }
}

// ========== 朗读 · 三层回退（T14 / AC-11 / AC-22）· 豆包级动听参数 ==========
async function handleSpeakThreeLayer(text) {
  const cleanText = String(text || '').trim().slice(0, 4000)
  if (!cleanText) { ElMessage.warning('无可朗读内容'); return }

  // --- 第一层：Rust 网关 GET /voice/tts/stream（WAV 流式 → Audio） ---
  // 【关键修复】此前第一层用 POST + form-body 提交，但 FastAPI 路由是 GET + query params，
  //  导致 405/404 必然失败，系统永远退到浏览器机械音。这里改为 GET + URLSearchParams
  //  做 query string（与 voice/main.py 的 @r.get("/tts/stream") 对齐）。
  try {
    const h = await getVoiceHealth()
    const streamPath = (h && h.endpoints && h.endpoints.tts_stream) ? h.endpoints.tts_stream : '/voice/tts/stream'
    const qs = new URLSearchParams()
    qs.append('text', cleanText)
    qs.append('voice', 'xiaobai')
    qs.append('emotion', 'neutral')
    qs.append('speed', '1.03') // 略快于 1.0，更接近豆包日常聊天语速
    const ttsToken = getToken()
    const ttsHeaders = { Accept: 'audio/wav' }
    if (ttsToken) ttsHeaders['Authorization'] = `Bearer ${ttsToken}`
    const url = `/api${streamPath}?${qs.toString()}`
    const resp = await fetch(url, { method: 'GET', headers: ttsHeaders })
    if (resp.ok && resp.body) {
      const fallback = String(_header(resp, 'X-TTS-Fallback') || '').toLowerCase()
      if (fallback !== 'browser') {
        try { if (typeof speechSynthesis !== 'undefined') speechSynthesis.cancel() } catch(_) {}
        const blob = await resp.blob()
        const src = URL.createObjectURL(blob)
        const audio = new Audio(src)
        audio.preload = 'auto'
        ;(window).__mbAudio = audio
        speechState.value = 'playing'
        audio.onended = () => { try { URL.revokeObjectURL(src) } catch(_){}; speechState.value = 'idle' }
        audio.onerror = () => { try { URL.revokeObjectURL(src) } catch(_){}; speechState.value = 'idle' }
        audio.onpause = () => { if (!audio.ended) speechState.value = 'paused' }
        try {
          await audio.play()
          return
        } catch(_playErr) {
          try { URL.revokeObjectURL(src) } catch(_){}
        }
      }
    }
  } catch (_e) { /* fall through */ }

  // --- 第二层：浏览器 Web Speech Synthesis · 豆包级拟人音精选 ---
  if ('speechSynthesis' in window) {
    try {
      window.speechSynthesis.cancel()
      const u = new SpeechSynthesisUtterance(cleanText)
      const v = pickZhVoice()
      if (v) { try { u.voice = v; } catch(_){} }
      u.lang = 'zh-CN'
      // 豆包级参数：略快 1.02、轻微升调 1.01 让声音更轻盈自然
      u.rate = 1.02
      u.pitch = 1.01
      u.volume = 1.0
      u.onstart = () => { speechState.value = 'playing'; }
      u.onend = () => { speechState.value = 'idle'; speechUtterance = null; }
      u.onerror = () => { speechState.value = 'idle'; speechUtterance = null; }
      u.onpause = () => { speechState.value = 'paused'; }
      u.onresume = () => { speechState.value = 'playing'; }
      speechUtterance = u
      window.speechSynthesis.speak(u)
      speechState.value = 'playing'
      return
    } catch (_e) { /* fall through */ }
  }

  // --- 第三层：兜底（AC-22）——弹提示并写入剪贴板，给用户自己读 ---
  try {
    await navigator.clipboard.writeText(cleanText)
    ElMessage({
      type: 'warning',
      showClose: true,
      duration: 0,
      message: '🔈 本地 TTS 服务不可用且浏览器不支持语音合成。朗读文本已写入剪贴板（长度 ' + cleanText.length + '）。',
    })
  } catch {
    ElMessage({
      type: 'warning',
      showClose: true,
      duration: 0,
      message: '🔈 无可用语音路径（三层回退均失败）。请手动复制消息文本。',
    })
  } finally {
    speechState.value = 'idle'
  }
}

function toggleSpeak() {
  if (speechState.value === 'playing') {
    // 尝试：若有流式音频，暂停；否则尝试 speechSynthesis pause
    try {
      const a = (window).__mbAudio
      if (a && !a.paused) { a.pause(); speechState.value = 'paused'; return }
    } catch(_) {}
    try {
      if (typeof speechSynthesis !== 'undefined') { speechSynthesis.pause(); speechState.value = 'paused'; return }
    } catch(_) {}
    speechState.value = 'paused'
    return
  }
  if (speechState.value === 'paused') {
    try {
      const a = (window).__mbAudio
      if (a && a.paused && !a.ended) { a.play(); speechState.value = 'playing'; return }
    } catch(_) {}
    try {
      if (typeof speechSynthesis !== 'undefined') { speechSynthesis.resume(); speechState.value = 'playing'; return }
    } catch(_) {}
    speechState.value = 'playing'
    return
  }
  // idle → 新播放
  try { if (typeof speechSynthesis !== 'undefined') speechSynthesis.cancel(); } catch(_) {}
  try {
    const a = (window).__mbAudio
    if (a) { try { a.pause() } catch(_) {} ; (window).__mbAudio = null }
  } catch(_) {}
  const text = mdToPlainText(_raw())
  handleSpeakThreeLayer(text)
}
function cancelSpeak() {
  try { if (typeof speechSynthesis !== 'undefined') speechSynthesis.cancel(); } catch(_) {}
  try {
    const a = (window).__mbAudio
    if (a) { try { a.pause() } catch(_) {} ; (window).__mbAudio = null }
  } catch(_) {}
  speechState.value = 'idle'
  speechUtterance = null
}
watch(() => props.msg?.content, () => cancelSpeak());
onBeforeUnmount(() => cancelSpeak());

// ============ 喜欢/不喜欢 ============
function persistRating(val) {
  const sid = stableMsgId(props.msg);
  try {
    if (val) localStorage.setItem('ous_msg_rating_' + sid, val);
    else localStorage.removeItem('ous_msg_rating_' + sid);
  } catch(_) {}
}
function toggleRating(kind) {
  if (rating.value === kind) { rating.value = null; persistRating(null); }
  else { rating.value = kind; persistRating(kind); }
  emit('rate', props.msg, rating.value);
}

// ============ 分享 ============
async function doShare() {
  const msg = props.msg;
  const sender = senderName.value;
  const time = formatTime(msg?.timestamp);
  const plain = mdToPlainText(_raw());
  const snippet = plain.length > 80 ? plain.slice(0, 80) + '…' : plain;
  const opCount = (msg?.referenced_operators?.length || 0);
  const url = typeof location !== 'undefined' ? location.href : '';
  const title = `来自 ${sender} 的璇玑助手消息`;
  const text = `[璇玑助手] ${sender} · ${time}${opCount?` · 算子${opCount}枚`:''}\n${snippet}\n打开链接：${url}`;
  try {
    if (typeof navigator.share === 'function') {
      await navigator.share({ title, text, url });
      emit('share', msg);
      ElMessage.success({ message: '已分享', duration: 1500 });
      return;
    }
  } catch(e) { /* 用户取消或失败，fallback */ }
  const ok = await copyTextUniversal(text + (url && !text.includes(url) ? '\n' + url : ''), '分享卡片已复制', true);
  if (ok) { ElMessage.success({ message: '分享卡片已复制到剪贴板', duration: 1800 }); emit('share', msg); }
  else ElMessage.error('分享失败，请手动复制');
}

// ============ 重新生成 ============
function doRegenerate() {
  if (regenLoading.value) return;
  regenLoading.value = true;
  emit('regenerate', props.msg);
  // 兜底：父级 1.4s 未响应则关闭 loading
  setTimeout(() => {
    if (regenLoading.value) {
      regenLoading.value = false;
      ElMessage.warning('重生成功能暂未启用');
    }
  }, 1400);
}

// ============ 转文档 ============
function openDocDialog() {
  docContent.value = _raw();
  docDlgOpen.value = true;
  docTab.value = 'edit';
}
const renderedDocContent = computed(() => {
  try { return mdInstance.value.render(String(docContent.value || '')); }
  catch(e) { return '<pre>' + escapeHtml(String(docContent.value || '')) + '</pre>'; }
});
async function exportMarkdown() {
  const md = String(docContent.value || '');
  emit('to-doc', props.msg, { mode: 'export-md', markdown: md });
  const ok = await tryWriteRichClipboard(mdInstance.value.render(md), md);
  if (ok) ElMessage.success('已写富文本 + Markdown 到剪贴板');
  else {
    const ok2 = await copyTextUniversal(md, 'Markdown 已复制');
    if (!ok2) ElMessage.error('导出失败，请手动复制');
  }
}
async function submitAsKb() {
  emit('to-doc', props.msg, { mode: 'create-kb', markdown: String(docContent.value || '') });
  ElMessage.success('已提交到知识库');
}

// ============ 收藏 ============
function toggleFavorite() {
  const sid = stableMsgId(props.msg);
  favorited.value = !favorited.value;
  favBeat.value = true;
  setTimeout(() => favBeat.value = false, 500);
  try {
    const raw = localStorage.getItem('ous_msg_favs');
    const arr = raw ? JSON.parse(raw) : [];
    const set = new Set(Array.isArray(arr) ? arr : []);
    if (favorited.value) set.add(sid); else set.delete(sid);
    localStorage.setItem('ous_msg_favs', JSON.stringify(Array.from(set)));
  } catch(_) {}
  emit('favorite', props.msg, favorited.value);
  ElMessage({ message: favorited.value ? '已收藏' : '已取消收藏', type: 'success', duration: 1200, showClose: false });
}

// ============ 追问 ============
function doFollowup() {
  const content = _raw();
  let prompt;
  if (props.msg?.role === 'assistant' && content.length > 400) prompt = '请继续展开说明刚才的回答：';
  else prompt = '关于以上内容，我想追问：';
  emit('followup', props.msg, prompt);
}

// ============ 反馈 ============
function resetFbForm() {
  fbForm.value = { type: '', severity: '', description: '', includeContext: true };
  try { fbFormRef.value?.clearValidate?.(); } catch(_) {}
}
async function submitFeedback() {
  if (!fbForm.value.type || !fbForm.value.severity) {
    ElMessage.warning('请选择反馈类型和严重程度');
    return;
  }
  const payload = { ...fbForm.value };
  emit('feedback', props.msg, payload);
  ElMessage.success('反馈已提交，感谢助力专家联盟质量升级');
  fbDlgOpen.value = false;
  resetFbForm();
}

// ============ 复制子菜单动态计算：watch renderedContent -> nextTick -> 回读 DOM ============
async function syncBlocksFromDom() {
  await nextTick();
  const root = instance?.vnode?.el;
  if (!root) { mermaidBlocks.value = []; fenceBlocks.value = []; return; }
  const mms = root.querySelectorAll('.mb-mermaid-card');
  mermaidBlocks.value = Array.from(mms).map((c, i) => {
    const raw = decodeURIComponent(c.getAttribute('data-src') || '');
    return { index: i, content: raw };
  });
  const ffs = root.querySelectorAll('.mb-fence');
  fenceBlocks.value = Array.from(ffs).map((f, i) => {
    const langSpan = f.querySelector('.mb-fence-lang');
    const linesSpan = f.querySelector('.mb-fence-lines');
    const codeEl = f.querySelector('pre code');
    return {
      index: i,
      lang: langSpan?.innerText?.trim() || 'text',
      lines: Number(String(linesSpan?.innerText || '0').replace(/[^\d]/g, '')) || 0,
      content: codeEl?.innerText ?? codeEl?.textContent ?? '',
    };
  });
  // 窄屏折叠
  try {
    const w = root.clientWidth || 0;
    moreCollapsed.value = w > 0 && w <= 520;
  } catch(_) { moreCollapsed.value = false; }
}
watch(renderedContent, syncBlocksFromDom, { flush: 'post' });
onMounted(syncBlocksFromDom);

function safeUrl(url) {
  if (!url) return "#";
  const s = String(url);
  if (/^\s*javascript\s*:/i.test(s) || /^\s*data\s*:/i.test(s)) return "#unsafe";
  return s;
}
function formatSize(bytes) {
  const b = Number(bytes);
  if (!Number.isFinite(b) || b <= 0) return "";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0, v = b;
  while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
  return v.toFixed(v >= 10 || i === 0 ? 0 : 1) + " " + units[i];
}
function formatDuration(ms) {
  const n = Number(ms);
  if (!Number.isFinite(n) || n <= 0) return "0ms";
  if (n < 1000) return Math.round(n) + "ms";
  return (n / 1000).toFixed(n < 10000 ? 1 : 0) + "s";
}
function artifactIcon(a) {
  const name = String(a?.filename || a?.path || "").toLowerCase();
  if (/\.(png|jpg|jpeg|gif|webp|svg|bmp|ico)$/.test(name)) return Picture;
  if (/\.(pdf|doc|docx|txt|md|rtf|html|htm)$/.test(name)) return Document;
  return FolderChecked;
}
</script>
<style scoped>
.mb-wrapper {
  display: flex;
  align-items: flex-start;
  margin: 16px 0;
  gap: 12px;
  max-width: 100%;
}
.mb-wrapper.mb-user { flex-direction: row-reverse; }

.mb-system-card {
  width: 100%;
  border-radius: 14px;
  padding: 18px 20px;
  background: linear-gradient(135deg, #fffbeb 0%, #fef3c7 45%, #fde68a 100%);
  border: 1px solid #fde68a;
  box-shadow: 0 1px 2px rgba(161, 98, 7, 0.06), 0 8px 24px -12px rgba(245, 158, 11, 0.25);
  position: relative;
  overflow: hidden;
}
.mb-system-card::before {
  content: "";
  position: absolute;
  top: 0; left: 0;
  width: 4px; height: 100%;
  background: linear-gradient(180deg, #f59e0b 0%, #d97706 100%);
}
.mb-system-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
.mb-system-badge { display: inline-flex; align-items: center; gap: 5px; font-weight: 600; letter-spacing: 0.2px; }
.mb-badge-icon { font-size: 13px; }
.mb-system-time { font-size: 12px; color: #92400e; opacity: 0.8; }
.mb-system-body { font-size: 14px; line-height: 1.75; color: #451a03; }
.mb-system-footer { margin-top: 12px; padding-top: 10px; border-top: 1px dashed rgba(180, 83, 9, 0.25); }

.mb-avatar-wrap { flex-shrink: 0; padding-top: 2px; }
.mb-avatar {
  width: 38px; height: 38px;
  border-radius: 11px;
  display: flex; align-items: center; justify-content: center;
  position: relative;
  transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.25s;
  color: #fff;
}
.mb-avatar:hover { transform: scale(1.08); }
.mb-avatar::after {
  content: "";
  position: absolute;
  inset: -2px;
  border-radius: 13px;
  padding: 2px;
  background: linear-gradient(135deg, #c7d2fe, #a5b4fc, #818cf8);
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  pointer-events: none;
  opacity: 0.9;
}
.mb-avatar-assistant {
  background: linear-gradient(135deg, #6366f1 0%, #4f46e5 50%, #4338ca 100%);
  box-shadow: 0 4px 10px -2px rgba(99, 102, 241, 0.5), 0 1px 2px rgba(67, 56, 202, 0.2);
}
.mb-avatar-assistant:hover { box-shadow: 0 6px 18px -3px rgba(99, 102, 241, 0.65); }
.mb-avatar-user {
  background: linear-gradient(135deg, #ec4899 0%, #db2777 50%, #be185d 100%);
  box-shadow: 0 4px 10px -2px rgba(236, 72, 153, 0.45), 0 1px 2px rgba(190, 24, 93, 0.2);
}
.mb-avatar-user:hover { box-shadow: 0 6px 18px -3px rgba(236, 72, 153, 0.6); }
.mb-avatar-user::after { background: linear-gradient(135deg, #fbcfe8, #f9a8d4, #f472b6); }

.mb-bubble {
  position: relative;
  max-width: calc(100% - 56px);
  flex: 1 1 auto;
  border-radius: 14px;
  padding: 14px 18px 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  box-shadow: 0 1px 2px rgba(15, 23, 42, 0.04), 0 4px 16px -10px rgba(15, 23, 42, 0.12);
  transition: box-shadow 0.28s cubic-bezier(0.4, 0, 0.2, 1);
}
.mb-bubble:hover { box-shadow: 0 2px 4px rgba(15, 23, 42, 0.06), 0 10px 28px -10px rgba(15, 23, 42, 0.2); }
.mb-wrapper.mb-user .mb-bubble {
  background: linear-gradient(135deg, #f5f3ff 0%, #ede9fe 55%, #ddd6fe 100%);
  border-color: #ddd6fe;
  box-shadow: 0 1px 2px rgba(109, 40, 217, 0.06), 0 4px 16px -10px rgba(109, 40, 217, 0.18);
}
.mb-wrapper.mb-user .mb-bubble:hover { box-shadow: 0 2px 4px rgba(109, 40, 217, 0.08), 0 12px 32px -10px rgba(109, 40, 217, 0.28); }

.mb-bubble-header { display: flex; align-items: center; gap: 10px; margin-bottom: 8px; padding-right: 44px; flex-wrap: wrap; }
.mb-sender { font-size: 13px; font-weight: 600; color: var(--text-primary); letter-spacing: 0.1px; }
.mb-wrapper.mb-user .mb-sender { color: #6d28d9; }
.mb-time { font-size: 11.5px; color: #94a3b8; font-variant-numeric: tabular-nums; }
.mb-confidence { font-size: 11px !important; padding: 0 8px !important; height: 20px !important; line-height: 18px !important; }

.mb-ops {
  position: absolute;
  top: 10px; right: 12px;
  opacity: 0;
  transform: translateY(-4px);
  transition: opacity 0.2s ease, transform 0.2s ease;
  pointer-events: none;
  z-index: 5;
}
.mb-ops-show { opacity: 1; transform: translateY(0); pointer-events: auto; }
.mb-op-btn { background: rgba(255, 255, 255, 0.92); border: 1px solid var(--border) !important; box-shadow: 0 2px 8px -2px rgba(15, 23, 42, 0.1); backdrop-filter: blur(4px); }
.mb-op-btn:hover { background: #6366f1 !important; border-color: #6366f1 !important; color: #fff !important; }
.mb-copy-hint { font-size: 11px; color: #94a3b8; margin-left: 4px; }

/* ========================================================
   极简下拉（Notion / Raycast 风格）
   - 纯白背景 + 极细边框 + 柔和阴影
   - 三栏：图标(inline 14) · gap 10 · 文本(flex) · gap 8 · 极简 badge
   - 行高 32，hover 单一浅灰背景（无渐变、无位移）
   ======================================================== */
:deep(.mb-dd-menu) {
  min-width: 212px;
  padding: 6px !important;
  border: 1px solid var(--border) !important;
  border-radius: 12px !important;
  background: var(--bg-card);
  box-shadow:
    0 20px 40px -16px rgba(15,23,42,0.16),
    0 4px 10px -6px rgba(15,23,42,0.08);
  backdrop-filter: blur(10px);
}
:deep(.mb-dd-menu.mb-dd-quick) { min-width: 196px; }

/* item 基础 */
:deep(.mb-dd-menu .el-dropdown-menu__item) {
  padding: 0 10px !important;
  height: 32px !important;
  line-height: 32px !important;
  border-radius: 7px !important;
  margin: 1px 0 !important;
  color: var(--text-primary) !important;
  font-size: 13px !important;
  transition: background-color 120ms ease;
}
:deep(.mb-dd-menu .el-dropdown-menu__item:hover),
:deep(.mb-dd-menu .el-dropdown-menu__item:focus-visible) {
  background: var(--bg-tertiary) !important;
  color: var(--text-primary) !important;
}

/* divider 极细 */
:deep(.mb-dd-menu .el-dropdown-menu__item.is-divided) {
  margin-top: 4px !important;
  padding-top: 0 !important;
  border-top: 1px solid var(--border-ghost) !important;
  border-radius: 0 0 7px 7px !important;
}

/* 三栏行 */
.mb-dd-row {
  display: flex;
  align-items: center;
  width: 100%;
  height: 100%;
  gap: 10px;
  line-height: 1.2;
}
/* 图标：无背景、同色内联 14px */
.mb-dd-ic {
  flex: 0 0 auto;
  width: 14px;
  height: 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: #94a3b8;
  background: transparent;
}
:deep(.el-dropdown-menu__item:hover) .mb-dd-ic {
  color: var(--text-primary);
  background: transparent;
}
.mb-dd-text {
  flex: 1 1 auto;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-weight: 400;
}
.mb-dd-sub {
  color: #94a3b8;
  font-weight: 400;
  margin-left: 2px;
}
.mb-dd-lang {
  font-family: 'JetBrains Mono', ui-monospace, Menlo, Monaco, Consolas, monospace;
  font-weight: 500;
  color: var(--text-primary);
  font-size: 12.5px;
}

/* 右侧极简 badge：单行最小 pill，默认灰字透明底 */
.mb-dd-tag {
  flex: 0 0 auto;
  padding: 1px 7px;
  height: 18px;
  line-height: 16px;
  border-radius: 999px;
  font-size: 10.5px;
  font-weight: 500;
  letter-spacing: 0.2px;
  color: #64748b;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
}
.mb-dd-tag-muted {
  color: #94a3b8;
  background: transparent;
  border-color: transparent;
}
.mb-dd-tag-accent {
  color: #64748b;
  background: transparent;
  border-color: transparent;
}
.mb-dd-tag-code {
  color: #64748b;
  background: transparent;
  border-color: transparent;
  font-family: inherit;
}

/* 禁用 hover 时下拉内拇指变大 */
:deep(.el-dropdown-menu__item) .mb-ic-thumb {
  transform: none !important;
}

/* ===== 9 动作工具栏（深空 × φ） ===== */
.mb-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  padding-top: 12px;
  margin-top: 12px;
  border-top: 1px dashed var(--border);
  opacity: 0.82;
  transform: translateY(3px);
  transition: opacity .22s ease, transform .22s ease;
}
.mb-actions-system {
  margin-top: 10px;
  padding-top: 10px;
  border-top-color: rgba(180, 83, 9, 0.25);
}
.mb-bubble:hover .mb-actions { opacity: 1; transform: translateY(0); }
.mb-system-card:hover .mb-actions { opacity: 1; transform: translateY(0); }
.mb-action-btn {
  width: 26px !important;
  height: 26px !important;
  padding: 0 !important;
  border-radius: 10px !important;
  background: var(--bg-card) !important;
  border: 1px solid var(--border) !important;
  color: #64748b !important;
  box-shadow: 0 1px 2px rgba(15,23,42,0.04);
  transition: transform .2s cubic-bezier(.4,0,.2,1), box-shadow .2s, background .2s, color .2s, border-color .2s;
}
.mb-action-btn:hover {
  transform: translateY(-1px);
  color: #4f46e5 !important;
  border-color: #c7d2fe !important;
  box-shadow: 0 4px 12px -6px rgba(99,102,241,0.35);
}
.mb-action-primary {
  background: linear-gradient(135deg,#6366f1 0%,#8b5cf6 60%,#a855f7 100%) !important;
  border-color: transparent !important;
  color: #ffffff !important;
  box-shadow: 0 6px 18px -8px rgba(99,102,241,0.6), 0 1px 2px rgba(79,70,229,0.15);
}
.mb-action-primary:hover {
  color: #ffffff !important;
  border-color: transparent !important;
  box-shadow: 0 10px 26px -8px rgba(99,102,241,0.75), 0 2px 4px rgba(79,70,229,0.2);
}
/* 朗读脉冲 */
.mb-tts.playing { color: #6366f1 !important; border-color: #c7d2fe !important; position: relative; }
.mb-tts.playing::after {
  content: "";
  position: absolute;
  top: 4px; right: 4px;
  width: 6px; height: 6px;
  border-radius: 50%;
  background: #6366f1;
  box-shadow: 0 0 0 0 rgba(99,102,241,0.55);
  animation: mb-tts-pulse 1.2s infinite;
}
@keyframes mb-tts-pulse {
  0% { box-shadow: 0 0 0 0 rgba(99,102,241,0.55); }
  70% { box-shadow: 0 0 0 6px rgba(99,102,241,0); }
  100% { box-shadow: 0 0 0 0 rgba(99,102,241,0); }
}
/* 喜欢/不喜欢填充 */
.mb-rate-like.active { color: #10b981 !important; border-color: #a7f3d0 !important; background: var(--success-50) !important; }
.mb-rate-dislike.active { color: #ef4444 !important; border-color: #fecaca !important; background: #fef2f2 !important; }
/* 收藏心跳 */
.mb-fav.active { color: #f59e0b !important; border-color: #fde68a !important; background: var(--warning-50) !important; }
.mb-heart-beat { animation: mb-heart-beat .5s cubic-bezier(.4,0,.2,1) 1; }
@keyframes mb-heart-beat {
  0% { transform: scale(1); }
  50% { transform: scale(1.28); }
  100% { transform: scale(1); }
}
/* 折叠 More */
.mb-actions-more-btn { display: none !important; }
@media (max-width: 520px) {
  .mb-actions { gap: 8px; padding-top: 10px; margin-top: 10px; }
  .mb-action-btn { width: 24px !important; height: 24px !important; }
  .mb-actions-more-btn { display: inline-flex !important; }
  .mb-action-collapsible { display: none !important; }
}

.mb-bubble-body { font-size: 14px; line-height: 1.75; color: var(--text-primary); word-break: break-word; }

.md-body :deep(*) { box-sizing: border-box; }
.md-body :deep(p) { margin: 0 0 12px; line-height: 1.8; }
.md-body :deep(p:last-child) { margin-bottom: 0; }
.md-body :deep(ul), .md-body :deep(ol) { padding-left: 22px; margin: 0 0 12px; }
.md-body :deep(li) { margin: 4px 0; line-height: 1.75; }
.md-body :deep(a) { color: #4f46e5; text-decoration: none; border-bottom: 1px solid rgba(79, 70, 229, 0.25); padding-bottom: 1px; transition: color 0.15s, border-color 0.15s; }
.md-body :deep(a:hover) { color: #6366f1; border-bottom-color: #6366f1; }
.md-body :deep(hr) { border: 0; height: 1px; background: linear-gradient(90deg, transparent 0%, #cbd5e1 50%, transparent 100%); margin: 20px 0; }
.md-body :deep(img) { max-width: 100%; border-radius: 10px; box-shadow: 0 4px 12px -4px rgba(15, 23, 42, 0.15); margin: 6px 0; }
.md-body :deep(strong) { color: var(--text-primary); font-weight: 600; }
.md-body :deep(em) { color: var(--text-secondary); }

.md-body :deep(h1) { font-size: 22px; font-weight: 700; color: var(--text-primary); margin: 0 0 14px; padding-bottom: 10px; position: relative; line-height: 1.35; }
.md-body :deep(h1)::after {
  content: ""; position: absolute; bottom: 0; left: 0;
  width: 61.8%; height: 2px; border-radius: 2px;
  background: linear-gradient(90deg, #6366f1 0%, #8b5cf6 50%, #a855f7 100%);
}
.md-body :deep(h2) {
  font-size: 18.5px; font-weight: 650; color: var(--text-primary);
  margin: 0 0 12px; padding: 3px 0 3px 11px;
  border-left: 3px solid #6366f1; line-height: 1.4;
  background: linear-gradient(90deg, rgba(99, 102, 241, 0.06) 0%, transparent 100%);
  border-radius: 0 6px 6px 0;
}
.md-body :deep(h3) { font-size: 16px; font-weight: 600; color: var(--text-secondary); margin: 0 0 10px; line-height: 1.45; padding-left: 2px; }

.md-body :deep(blockquote) {
  margin: 0 0 12px;
  padding: 10px 14px 10px 40px;
  border-left: 4px solid #8b5cf6;
  background: linear-gradient(135deg, rgba(139, 92, 246, 0.08) 0%, rgba(99, 102, 241, 0.04) 100%);
  border-radius: 0 10px 10px 0;
  position: relative;
  color: #4c1d95;
}
.md-body :deep(blockquote)::before {
  content: "\u275D"; position: absolute; left: 12px; top: 4px;
  font-size: 22px; color: #8b5cf6; opacity: 0.75;
  font-family: Georgia, serif; line-height: 1;
}
.md-body :deep(blockquote p) { color: #4c1d95; margin: 0; }

.md-body :deep(table) {
  width: 100%; border-collapse: separate; border-spacing: 0;
  margin: 0 0 14px; border-radius: 10px; overflow: hidden;
  border: 1px solid var(--border); font-size: 13.5px;
}
.md-body :deep(thead) { background: linear-gradient(180deg, #f8fafc 0%, #f1f5f9 100%); }
.md-body :deep(th) { padding: 10px 14px; text-align: left; font-weight: 600; color: var(--text-secondary); border-bottom: 1px solid var(--border); font-size: 13px; }
.md-body :deep(td) { padding: 9px 14px; border-bottom: 1px solid var(--border-ghost); color: var(--text-secondary); transition: background 0.15s; }
.md-body :deep(tbody tr:nth-child(even)) { background: rgba(248, 250, 252, 0.6); }
.md-body :deep(tbody tr:hover td) { background: rgba(99, 102, 241, 0.05); }
.md-body :deep(tbody tr:last-child td) { border-bottom: 0; }

.md-body :deep(code):not(pre code) {
  display: inline-block;
  padding: 1.5px 6px; margin: 0 1px;
  font-family: "JetBrains Mono", "Fira Code", ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace;
  font-size: 12.5px;
  border-radius: 5px;
  background: linear-gradient(135deg, rgba(139, 92, 246, 0.1) 0%, rgba(20, 184, 166, 0.08) 100%);
  color: #6d28d9;
  border: 1px solid rgba(139, 92, 246, 0.18);
  vertical-align: baseline;
}

.md-body :deep(.task-list-item) { list-style: none; margin-left: -22px; padding-left: 4px; }
.md-body :deep(.task-list-item-checkbox) {
  appearance: none; -webkit-appearance: none;
  width: 15px; height: 15px;
  border: 1.5px solid #a5b4fc; border-radius: 4px;
  margin-right: 8px; vertical-align: -2px;
  position: relative; background: var(--bg-card); cursor: pointer;
  transition: all 0.15s;
}
.md-body :deep(.task-list-item-checkbox:checked) {
  background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%);
  border-color: #6366f1;
}
.md-body :deep(.task-list-item-checkbox:checked)::after {
  content: ""; position: absolute;
  left: 3.5px; top: 0.5px;
  width: 5px; height: 9px;
  border: solid #fff;
  border-width: 0 2px 2px 0;
  transform: rotate(45deg);
}
.md-body :deep(.task-list-item-label) { cursor: pointer; user-select: none; }
.md-body :deep(.task-list-item-checkbox:checked + .task-list-item-label) { color: #94a3b8; text-decoration: line-through; }

.md-body :deep(.mb-fence) {
  border-radius: 12px; background: #0a0f1e;
  border: 1px solid #1e293b; overflow: hidden;
  margin: 0 0 14px;
  box-shadow: 0 4px 16px -6px rgba(10, 15, 30, 0.4);
}
.md-body :deep(.mb-fence-head) {
  display: flex; align-items: center; justify-content: space-between;
  padding: 8px 14px;
  background: linear-gradient(180deg, #0f172a 0%, #0a0f1e 100%);
  border-bottom: 1px solid #1e293b;
}
.md-body :deep(.mb-fence-left) { display: flex; align-items: center; gap: 10px; }
.md-body :deep(.mb-fence-lang) {
  font-size: 11.5px; font-weight: 600; letter-spacing: 0.5px;
  color: #818cf8; text-transform: uppercase;
  padding: 2px 7px; background: rgba(129, 140, 248, 0.12);
  border-radius: 4px; font-family: inherit;
}
.md-body :deep(.mb-fence-lines) { font-size: 10.5px; color: #64748b; font-variant-numeric: tabular-nums; }
.md-body :deep(.mb-fence-copy) {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 3px 9px; font-size: 11px;
  color: #94a3b8; background: rgba(148, 163, 184, 0.08);
  border: 1px solid rgba(148, 163, 184, 0.18);
  border-radius: 6px; cursor: pointer;
  transition: all 0.18s; font-family: inherit;
}
.md-body :deep(.mb-fence-copy:hover) { color: #c7d2fe; background: rgba(99, 102, 241, 0.18); border-color: rgba(99, 102, 241, 0.4); }
.md-body :deep(.mb-fence-copy-ok) { color: #10b981 !important; background: rgba(16, 185, 129, 0.15) !important; border-color: rgba(16, 185, 129, 0.35) !important; }
.md-body :deep(.mb-fence-copy-err) { color: #ef4444 !important; background: rgba(239, 68, 68, 0.15) !important; border-color: rgba(239, 68, 68, 0.35) !important; }
.md-body :deep(.mb-fence-code) { overflow-x: auto; padding: 0; }
.md-body :deep(.mb-fence-pre) {
  margin: 0; padding: 14px 16px;
  font-family: "JetBrains Mono", "Fira Code", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12.8px; line-height: 1.7;
  color: #c7d2fe; white-space: pre; overflow-x: auto; tab-size: 2;
}
.md-body :deep(.mb-fence-pre code) { background: transparent !important; border: 0 !important; padding: 0 !important; color: inherit !important; font-size: inherit !important; }
.md-body :deep(.mb-mermaid-card) {
  border-radius: 12px;
  background: linear-gradient(180deg, #ffffff 0%, #fafaff 100%);
  border: 1px solid #e0e7ff;
  margin: 0 0 14px;
  overflow: hidden;
  box-shadow: 0 2px 6px rgba(99, 102, 241, 0.06), 0 10px 30px -14px rgba(99, 102, 241, 0.18);
}
.md-body :deep(.mb-mermaid-head) {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 16px;
  background: linear-gradient(90deg, rgba(99, 102, 241, 0.08) 0%, rgba(139, 92, 246, 0.04) 100%);
  border-bottom: 1px solid #e0e7ff;
}
.md-body :deep(.mb-mermaid-badge) { display: inline-flex; align-items: center; gap: 6px; font-size: 12.5px; font-weight: 600; color: #4338ca; }
.md-body :deep(.mb-mermaid-badge svg) { color: #6366f1; }
.md-body :deep(.mb-mermaid-realtime) {
  display: inline-flex; align-items: center; gap: 6px;
  font-size: 11px; color: #059669;
  background: rgba(5, 150, 105, 0.08);
  padding: 2px 8px; border-radius: 10px;
  border: 1px solid rgba(5, 150, 105, 0.15);
  font-weight: 500;
}
.md-body :deep(.mb-rt-dot) {
  width: 6px; height: 6px; border-radius: 50%;
  background: #10b981;
  box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.55);
  animation: mb-rt-pulse 1.8s infinite;
}
@keyframes mb-rt-pulse {
  0% { box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.55); }
  70% { box-shadow: 0 0 0 6px rgba(16, 185, 129, 0); }
  100% { box-shadow: 0 0 0 0 rgba(16, 185, 129, 0); }
}
.md-body :deep(.mb-mermaid-body) {
  position: relative;
  padding: 16px; min-height: 80px;
  display: flex; justify-content: center; align-items: center;
  overflow-x: auto;
}
.md-body :deep(.mb-mermaid-loading) {
  display: flex; align-items: center; justify-content: center;
  gap: 10px; color: #6366f1; font-size: 13px; padding: 18px 0;
}
.md-body :deep(.mb-mermaid-spinner) {
  width: 20px; height: 20px;
  border: 2px solid rgba(99, 102, 241, 0.2);
  border-top-color: #6366f1;
  border-radius: 50%;
  animation: mb-spin 0.8s linear infinite;
}
@keyframes mb-spin { to { transform: rotate(360deg); } }
.md-body :deep(.mb-mermaid-target) { width: 100%; display: flex; justify-content: center; }
.md-body :deep(.mb-mermaid-target svg) { max-width: 100%; height: auto; }
.md-body :deep(.mb-mermaid-fade-in) { animation: mb-fade-in 0.5s cubic-bezier(0.4, 0, 0.2, 1); }
@keyframes mb-fade-in {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: translateY(0); }
}
.md-body :deep(.mb-mermaid-error) { width: 100%; padding: 4px; }
.md-body :deep(.mb-mermaid-error-head) {
  display: flex; align-items: center; gap: 8px;
  padding: 10px 14px;
  background: #fef2f2; border: 1px solid #fecaca;
  border-radius: 8px; color: #b91c1c;
  font-size: 13px; font-weight: 500; flex-wrap: wrap;
}
.md-body :deep(.mb-mermaid-error-msg) {
  font-size: 12px; font-weight: 400; color: #991b1b;
  background: rgba(239, 68, 68, 0.08);
  padding: 2px 8px; border-radius: 4px;
  max-width: 100%; word-break: break-all;
  margin-left: auto;
}
.md-body :deep(.mb-mermaid-error-details) { margin-top: 10px; padding: 0 8px; }
.md-body :deep(.mb-mermaid-error-details summary),
.md-body :deep(.mb-mermaid-source summary) {
  cursor: pointer; font-size: 12px; color: #64748b;
  padding: 6px 4px; user-select: none;
}
.md-body :deep(.mb-mermaid-error-details summary:hover),
.md-body :deep(.mb-mermaid-source summary:hover) { color: #4f46e5; }
.md-body :deep(.mb-mermaid-error-pre),
.md-body :deep(.mb-mermaid-source-pre) {
  margin: 6px 0 10px; padding: 12px;
  background: #0a0f1e; color: #c7d2fe;
  border-radius: 8px;
  font-family: "JetBrains Mono", "Fira Code", ui-monospace, monospace;
  font-size: 12px; line-height: 1.65; overflow-x: auto;
  border: 1px solid #1e293b;
}
.md-body :deep(.mb-mermaid-source) { padding: 0 16px 14px; border-top: 1px dashed #e0e7ff; margin-top: 4px; }

.mb-meta {
  margin-top: 14px; padding-top: 12px;
  border-top: 1px dashed var(--border);
  display: flex; flex-direction: column; gap: 12px;
}
.mb-wrapper.mb-user .mb-meta { border-top-color: rgba(139, 92, 246, 0.25); }
.mb-meta-section { display: flex; flex-direction: column; gap: 6px; }
.mb-meta-title {
  display: inline-flex; align-items: center; gap: 5px;
  font-size: 12px; font-weight: 600; color: #64748b; letter-spacing: 0.1px;
}
.mb-meta-chips { display: flex; flex-wrap: wrap; gap: 6px; }
.mb-chip :deep(.el-icon) { margin-right: 4px; }
.mb-chip-ic { font-size: 12px !important; }

.mb-meta-list { margin: 0; padding-left: 20px; font-size: 12.5px; line-height: 1.7; }
.mb-web-list { color: var(--text-secondary); }
.mb-web-idx { color: #94a3b8; margin-right: 2px; font-variant-numeric: tabular-nums; }
.mb-web-link { color: #4f46e5; text-decoration: none; word-break: break-all; transition: color 0.15s; }
.mb-web-link:hover { color: #6366f1; text-decoration: underline; }
.mb-web-host {
  display: inline-block; margin-left: 6px;
  font-size: 10.5px; padding: 1px 6px;
  background: rgba(148, 163, 184, 0.12); color: #64748b;
  border-radius: 10px; vertical-align: 1px;
}
.mb-artifact-list { list-style: none; padding-left: 0; }
.mb-artifact-item {
  display: flex; align-items: center; gap: 8px;
  padding: 6px 10px;
  background: rgba(248, 250, 252, 0.6);
  border: 1px solid var(--border); border-radius: 8px;
  font-size: 12.5px; transition: all 0.15s;
}
.mb-artifact-item:hover { background: rgba(99, 102, 241, 0.04); border-color: #c7d2fe; }
.mb-artifact-ic { color: #8b5cf6; flex-shrink: 0; }
.mb-artifact-name { flex: 1 1 auto; color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; word-break: break-all; }
.mb-artifact-size { font-size: 11px; color: #94a3b8; flex-shrink: 0; }

.mb-error {
  padding: 10px 12px;
  background: #fef2f2; border: 1px solid #fecaca;
  border-radius: 10px;
}
.mb-error-title { color: #b91c1c; }
.mb-error-body { display: flex; flex-direction: column; gap: 6px; }
.mb-error-msg { font-size: 13px; color: #7f1d1d; line-height: 1.6; }
.mb-error-stack summary { cursor: pointer; font-size: 11.5px; color: #991b1b; padding: 2px 0; }
.mb-error-stack-pre {
  margin: 6px 0 0; padding: 10px 12px;
  background: #1f2937; color: #e5e7eb;
  border-radius: 6px; font-size: 11.5px;
  line-height: 1.6; overflow-x: auto;
  white-space: pre-wrap; word-break: break-all;
}

.mono { font-family: "JetBrains Mono", "Fira Code", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
.mb-empty { color: #94a3b8; font-style: italic; margin: 0; font-size: 13px; }

/* ===== 对话框样式（深空 φ 尺寸） ===== */
:deep(.mb-doc-dialog .el-dialog),
:deep(.mb-fb-dialog .el-dialog) {
  border-radius: 14px !important;
  overflow: hidden;
  box-shadow: 0 30px 60px -20px rgba(15,23,42,0.25), 0 8px 20px -10px rgba(15,23,42,0.15);
}
:deep(.mb-doc-dialog .el-dialog__header),
:deep(.mb-fb-dialog .el-dialog__header) {
  padding: 20px 26px 16px !important;
  border-bottom: 1px solid var(--border-ghost);
  margin-right: 0 !important;
}
:deep(.mb-doc-dialog .el-dialog__title),
:deep(.mb-fb-dialog .el-dialog__title) {
  font-size: 16px !important;
  font-weight: 650 !important;
  color: var(--text-primary);
  letter-spacing: 0.2px;
}
:deep(.mb-doc-dialog .el-dialog__body),
:deep(.mb-fb-dialog .el-dialog__body) {
  padding: 20px 26px !important;
  color: var(--text-secondary);
}
:deep(.mb-doc-dialog .el-dialog__footer),
:deep(.mb-fb-dialog .el-dialog__footer) {
  padding: 14px 26px 20px !important;
  border-top: 1px solid var(--border-ghost);
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
:deep(.mb-doc-dialog .el-tabs__nav-wrap::after) { background-color: var(--border); }
:deep(.mb-doc-dialog .el-tabs__item.is-active) { color: #4f46e5; font-weight: 600; }
:deep(.mb-doc-dialog .el-tabs__active-bar) { background: linear-gradient(90deg,#6366f1,#8b5cf6); height: 2px; }
.doc-stats {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed var(--border);
  text-align: right;
  font-size: 11.5px;
  color: #94a3b8;
  font-variant-numeric: tabular-nums;
}
:deep(.mb-fb-dialog .el-form-item__label) {
  color: var(--text-secondary) !important;
  font-weight: 500 !important;
}
:deep(.mb-fb-dialog .el-radio) { margin-right: 14px; }
:deep(.mb-fb-dialog .el-radio__input.is-checked .el-radio__inner) { border-color: #6366f1; background: #6366f1; }
:deep(.mb-fb-dialog .el-checkbox__input.is-checked .el-checkbox__inner) { background-color: #6366f1; border-color: #6366f1; }

/* aria 播报（视觉隐藏） */
.mb-sr-live {
  position: absolute !important;
  width: 1px; height: 1px;
  padding: 0; margin: -1px;
  overflow: hidden; clip: rect(0,0,0,0);
  white-space: nowrap; border: 0;
}

/* ===== 拆分式复制按钮（极简连体胶囊 · φ 比例 26+16=42） ===== */
.mb-copy-split-wrap {
  display: inline-flex;
  align-items: stretch;
  gap: 0;
  background: var(--mb-toolbar-btn, linear-gradient(135deg, #6366f1 0%, #10b981 100%));
  border-radius: 999px;
  overflow: visible;
  box-shadow:
    0 2px 6px -2px rgba(99,102,241,0.30),
    0 1px 2px -1px rgba(15,23,42,0.08);
}
.mb-copy-split-wrap .mb-copy-main {
  width: 26px !important;
  height: 26px !important;
  border-radius: 999px 0 0 999px !important;
  margin-right: 0;
  z-index: 1;
  border: none !important;
  background: transparent !important;
  color: #fff !important;
}
.mb-copy-split-wrap .mb-copy-caret {
  width: 16px !important;       /* 26 / φ ≈ 16.07 */
  height: 26px !important;
  border-radius: 0 999px 999px 0 !important;
  border: none !important;
  border-left: 1px solid rgba(255,255,255,0.22) !important;
  min-width: 0;
  padding: 0 !important;
  background: transparent !important;
  color: #fff !important;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.mb-copy-split-wrap .mb-copy-caret:hover { filter: brightness(1.08); }
.mb-copy-split-wrap .mb-copy-main:hover { filter: brightness(1.08); }

/* 悬浮 mb-ops 区拆分复制（浅色极简胶囊） */
.mb-ops-split {
  display: inline-flex;
  align-items: stretch;
  gap: 0;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 999px;
  overflow: visible;
  box-shadow: 0 2px 6px -2px rgba(15,23,42,0.08);
}
.mb-ops-split .mb-ops-main {
  width: 26px !important;
  height: 26px !important;
  border-radius: 999px 0 0 999px !important;
  margin-right: 0;
  z-index: 1;
  border: none !important;
  padding: 0 !important;
  background: transparent !important;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary) !important;
}
.mb-ops-split .mb-ops-caret {
  width: 16px !important;       /* 26 / φ ≈ 16.07 */
  height: 26px !important;
  border-radius: 0 999px 999px 0 !important;
  border: none !important;
  border-left: 1px solid var(--border) !important;
  min-width: 0;
  padding: 0 !important;
  background: transparent !important;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary) !important;
}
.mb-ops-split .mb-ops-main:hover {
  background: #6366f1 !important;
  color: #fff !important;
}
.mb-ops-split .mb-ops-caret:hover {
  background: #6366f1 !important;
  color: #fff !important;
  border-left-color: rgba(255,255,255,0.22) !important;
}
.mb-ops-caret svg { display: block; }

/* ===== 拇指 & 扬声器 SVG 图标 ===== */
.mb-ic-thumb {
  display: block;
  transition: transform 0.18s cubic-bezier(.4,0,.2,1);
}
.mb-action-btn:hover .mb-ic-thumb {
  transform: scale(1.08);
}
.mb-ic-tts {
  display: block;
}

/* ===== 分享对话框样式（深空级 φ 美学） ===== */
:deep(.mb-share-dialog .el-dialog) {
  border-radius: 16px !important;
  overflow: hidden;
  box-shadow: 0 40px 80px -24px rgba(15,23,42,0.35), 0 10px 28px -14px rgba(99,102,241,0.35);
  background: linear-gradient(180deg, #ffffff 0%, #fafbff 100%);
}
:deep(.mb-share-dialog .el-dialog__header) {
  padding: 20px 26px 16px !important;
  border-bottom: 1px solid #eef2ff;
  margin-right: 0 !important;
  background: linear-gradient(90deg, rgba(99,102,241,0.06) 0%, rgba(139,92,246,0.03) 50%, transparent 100%);
}
:deep(.mb-share-dialog .el-dialog__title) {
  font-size: 16px !important;
  font-weight: 650 !important;
  background: linear-gradient(90deg, #4f46e5, #8b5cf6);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent !important;
  letter-spacing: 0.2px;
}
:deep(.mb-share-dialog .el-dialog__body) {
  padding: 18px 22px 8px !important;
  color: var(--text-secondary);
}
:deep(.mb-share-dialog .el-dialog__footer) {
  padding: 14px 24px 20px !important;
  border-top: 1px solid #eef2ff;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.share-scope-tip { margin-bottom: 14px; }
:deep(.share-scope-tip .el-alert) {
  border-radius: 10px;
  background: linear-gradient(90deg, rgba(99,102,241,0.08) 0%, rgba(224,231,255,0.6) 100%);
  border: 1px solid #e0e7ff;
  color: #4338ca;
}
:deep(.share-scope-tip .el-alert__title) {
  color: #4338ca;
  font-weight: 500;
  font-size: 12.5px;
  letter-spacing: 0.1px;
}

.share-list-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
  padding: 0 2px;
}
.share-list-title {
  display: inline-flex;
  align-items: center;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.1px;
}
.share-list-title .el-icon { color: #6366f1; margin-right: 5px; font-size: 15px; }
.share-list-actions { display: inline-flex; gap: 2px; }

.share-list-box {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-card);
  max-height: 280px;
  overflow-y: auto;
  padding: 4px;
  box-shadow: inset 0 2px 4px rgba(15,23,42,0.03);
}
.share-empty { padding: 30px 0; }
.share-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.share-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 9px 10px;
  border-radius: 9px;
  cursor: pointer;
  transition: background 0.16s, border-color 0.16s;
  border: 1px solid transparent;
  position: relative;
}
.share-item:hover {
  background: linear-gradient(90deg, rgba(99,102,241,0.05) 0%, rgba(139,92,246,0.03) 100%);
  border-color: #e0e7ff;
}
.share-item.si-selected {
  background: linear-gradient(135deg, rgba(99,102,241,0.09) 0%, rgba(139,92,246,0.07) 100%);
  border-color: #c7d2fe;
}
.share-item.si-current {
  background: linear-gradient(135deg, rgba(245,158,11,0.09) 0%, rgba(251,191,36,0.05) 100%);
  border-color: #fde68a;
}
.share-item.si-current.si-selected {
  background: linear-gradient(135deg, rgba(99,102,241,0.09) 0%, rgba(245,158,11,0.06) 100%);
  border-color: #818cf8;
}
.share-item .el-checkbox { margin-right: 2px; margin-top: 2px; }
.si-body { flex: 1 1 auto; min-width: 0; }
.si-head {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
  flex-wrap: wrap;
}
.si-head :deep(.el-tag) { height: 18px !important; line-height: 16px !important; padding: 0 7px !important; font-size: 10.5px !important; }
.si-current-tag {
  display: inline-block;
  padding: 0 6px;
  font-size: 10.5px;
  background: linear-gradient(90deg, #f59e0b, #f97316);
  color: #fff;
  border-radius: 10px;
  height: 18px;
  line-height: 18px;
  font-weight: 600;
  letter-spacing: 0.2px;
}
.si-time {
  font-size: 11px;
  color: #94a3b8;
  font-variant-numeric: tabular-nums;
}
.si-preview {
  font-size: 12.5px;
  line-height: 1.55;
  color: var(--text-secondary);
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  word-break: break-all;
}

.share-link-section {
  margin-top: 16px;
  padding: 14px 16px;
  border-radius: 12px;
  background: linear-gradient(135deg, #eef2ff 0%, #f5f3ff 55%, #fdf4ff 100%);
  border: 1px solid #e0e7ff;
}
.share-link-label {
  display: inline-flex;
  align-items: center;
  font-size: 12.5px;
  font-weight: 600;
  color: #4338ca;
  margin-bottom: 9px;
  letter-spacing: 0.1px;
}
.share-link-label .el-icon { margin-right: 5px; color: #6366f1; }
.share-link-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.share-link-row :deep(.el-input) { flex: 1 1 auto; }
.share-link-row :deep(.el-input__wrapper) {
  background: var(--bg-card) !important;
  border-radius: 9px !important;
  box-shadow: 0 1px 2px rgba(15,23,42,0.05) inset !important;
}
.share-link-hint {
  margin-top: 9px;
  font-size: 11.5px;
  color: #4338ca;
  padding: 5px 10px;
  border-radius: 10px;
  background: rgba(99,102,241,0.08);
  font-weight: 500;
}

@media (max-width: 720px) {
  .mb-wrapper { margin: 12px 0; gap: 8px; }
  .mb-avatar { width: 34px; height: 34px; border-radius: 9px; }
  .mb-avatar::after { border-radius: 11px; }
  .mb-bubble { max-width: calc(100% - 46px); padding: 12px 14px 14px; border-radius: 12px; }
  .mb-system-card { padding: 14px 14px 14px 18px; border-radius: 12px; }
  .mb-bubble-header { padding-right: 40px; gap: 8px; }
  .md-body :deep(h1) { font-size: 19.5px; }
  .md-body :deep(h2) { font-size: 16.5px; }
  .md-body :deep(h3) { font-size: 15px; }
  .md-body :deep(.mb-fence-pre) { font-size: 12px; padding: 12px; }
  .md-body :deep(.mb-mermaid-body) { padding: 12px 8px; }
  .md-body :deep(table) { font-size: 12.5px; }
  .md-body :deep(th), .md-body :deep(td) { padding: 7px 10px; }
  .mb-ops { top: 8px; right: 8px; }
  .mb-meta { margin-top: 12px; padding-top: 10px; }
  .mb-actions { gap: 8px; padding-top: 10px; }
  .mb-copy-split-wrap .mb-copy-caret { width: 14px !important; height: 24px !important; }
}
</style>
