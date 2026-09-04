# AI 对话 9 动作工具栏 + mox 模块化系统架构分析最佳布局 实施计划

## 仓库调研结论

### 现有代码状态
1. **MessageBubble.vue（~972 行）**：
   - 模板：已有 `.mb-ops`（右上角悬停复制按钮下拉：MD/纯文本/富文本三格式）、`fence` 代码块内按钮、Mermaid 卡片内源码折叠区；**缺少用户要求的"底部 9 动作工具栏"和"转文档/反馈 2 对话框"**。
   - 脚本：已有 `copyMarkdown/copyPlainText/copyRichHtml` + `mdToPlainText` + `_fallbackCopy` + `speechSynthesis` **未接入** + `defineEmits(['goto-task'])`（缺少 `rate/share/regenerate/to-doc/favorite/followup/feedback` 7 个新 emit）。
   - 样式：深空主题已到位（`global.css` 令牌），φ 尺寸变量体系完整；**缺少工具栏按钮 26×26 φ 尺寸、朗读进度心跳、收藏心跳、折叠 More 三类新样式**。
2. **ChatView.vue（1773+ 行）**：
   - 顶栏（L12-81）：已有专家选择器、两个开关、恢复历史/导入/导出/转任务/创建项目/清空共 10+ 元素，但**未按 FR11 严格顺序"新建/清空/导出/导入/转任务/创建项目/mox 模块化系统架构分析"7 按钮排列、缺少"mox 模块化系统架构分析 φ 强调主按钮"和"分析阶段 5 chip 指示器"**。
   - 空态快捷问法（L268-270）：当前为 `el-tag` 水平排列，**未按 3×2 φ 卡片 grid 布局**。
   - MessageBubble 绑定（L280-285）：只有 `@goto-task`，**缺少 7 个新 emit 占位处理**。
   - 输入面板、空态 Orb 尺寸：需按 FR11 四段黄金比例微调。
3. **global.css（~202 行）**：深空令牌齐全，大概率无需改动。
4. **前端服务**：`localhost:3021` 已运行（PID 37688 正常 LISTEN + 多 ESTABLISHED），HMR 可直接验证。
5. **依赖**：`markdown-it/mermaid/markdown-it-anchor/markdown-it-task-lists` 已安装；**本计划不新增任何 npm 依赖**（朗读/分享全部走浏览器原生 API）。

### 既定契约
- `msg.role` ∈ { `user`, `assistant`, `system` }；`msg.content` 为 Markdown 字符串；可选 `msg.id`（无则用 hash）。
- `defineEmits` 保持 kebab-case；父级 ChatView 用 `onXxx` 方法接收并做**最小占位**（toast + 合理副作用）。
- 既有回归探针（AC11：`preCount≥3`、`mermaidCards≥1`、`targetSvgCount≥1`、`fenceCards≥1`、复制三格式）必须 100% 通过。

---

## 文件与模块改动

| 文件 | 改动类型 | 说明 |
|---|---|---|
| `frontend-ui/src/components/MessageBubble.vue` | 主改动（模板+脚本+样式） | 新增：9 动作底部工具栏、2 对话框（转文档/反馈）、TTS、复制聚合下拉、localStorage 持久化；扩展 emits 7 个 |
| `frontend-ui/src/views/ChatView.vue` | 次改动（绑定+布局） | 新增：7 emit 占位处理函数；顶栏 7 按钮重排 + 分析阶段 5 chip；空态 3×2 快捷问法卡片改 φ grid；四段比例微调 |
| `frontend-ui/src/styles/global.css` | 不改或补 0-2 条 | 如工具栏动画需要的新全局 keyframes，仅追加不覆盖 |

---

## 实施步骤（依赖顺序）

### Step 1 · MessageBubble 模板扩展（动作条 + 2 对话框 + More 折叠）
- 在 `<template>` 现有 `.mb-bubble` 内、`.mb-meta` 下方追加 `<div class="mb-actions">` 区块。
- 按 FR1 严格顺序放置 9+1 按钮：①一键复制（组合下拉）②朗读 ③喜欢 ④不喜欢 ⑤分享 ⑥重新生成（仅 assistant）⑦转文档 ⑧收藏 ⑨追问 ⑩反馈 ⑪折叠 More（`≤520px` 自动出现）。
- 每个按钮加 `aria-label`、`title`、`class="mb-action-btn"`；按钮通用属性 `circle size="small"`（直径 ~26px φ）。
- 在 `<template>` 末尾（`</div>` 之前）新增 2 个 `el-dialog`：
  - **转文档编辑对话框**（`el-dialog` + `el-tabs`：Markdown textarea 编辑 / mdInstance 预览；底部 3 按钮：新建云盘 / 导出 MD / 取消；字数与阅读时长统计）。
  - **反馈对话框**（`el-form` 含 4 字段：反馈类型、严重程度、描述、联系上下文 checkbox；提交/取消 footer）。
- 系统气泡单独渲染：复制 + 朗读 +（如有 task_id 则跳转任务）三项。

### Step 2 · MessageBubble 脚本逻辑（9 动作全功能 + 7 emits + 聚合复制）
- **扩展 emits**：`defineEmits(['goto-task','rate','share','regenerate','to-doc','favorite','followup','feedback'])`。
- **状态 refs**：`speechState`(idle/playing/paused)、`speechUtterance`、`rating`(null/like/dislike)、`regenLoading`、`favorited`、`moreCollapsed`、`docDlgOpen`、`fbDlgOpen`、`docTab`、`docContent`、`fbForm`、`fbFormRef`。
- **扩展图标 import**：`Microphone / VideoPlay / VideoPause / ThumbUp / ThumbDown / Share / Refresh / DocumentAdd / Star / StarFilled / ChatLineSquare / Flag / More`。
- **FR2 复制聚合**：在 `md.renderer.rules.fence` 里把每个 Mermaid/代码块的 `lang/content/lineCount` 追加到响应式 `mermaidBlocks` / `fenceBlocks` 数组；`handleCopyCommand(cmd)` 新增解析 `mermaid-${i}` / `fence-${i}` 子命令，取对应块内容复制。
- **FR3 TTS**：`toggleSpeak()` 三态机；`speechSynthesis.speak(new SpeechSynthesisUtterance(mdToPlainText(content)))`；zh-CN voice 优先；`onBeforeUnmount + watch(msg.content)` 双路 `cancel()` 防泄漏。
- **FR4 评分 + FR8 收藏**：读取/写入 `localStorage['ous_msg_rating_'+stableId]`、`ous_msg_favs`（JSON 数组）；状态切换互斥逻辑（like↔dislike）。
- **FR5 分享**：优先 `navigator.share({title,text,url})`；失败则复制分享卡片（发送者+时间+80字摘要+引用算子+location.href）到剪贴板。
- **FR6 重新生成**：仅 assistant 显示；点击→`regenLoading=true`→`emit('regenerate',msg)`→兜底 1.5s 若父级无响应则 toast 关闭 loading。
- **FR7 转文档**：打开对话框时 `docContent = _raw()`；导出按钮写 ClipboardItem（MD + HTML 双份），新建云盘按钮 emit 占位。
- **FR9 追问**：按内容长度决定前缀（>400字→展开说明，否则→默认前缀）；emit `('followup',msg,prompt)`。
- **FR10 反馈**：`fbFormRef.validate()`→成功→emit→toast→关闭并清空表单。
- **消息稳定 ID**：`stableId = msg.id || hash(msg.timestamp+'|'+msg.content?.slice(0,200))`。

### Step 3 · MessageBubble CSS 深空美学补齐
- **工具栏容器**：`.mb-actions { display:flex; gap:10px(φ); padding-top:12px; margin-top:12px; border-top:dashed #e2e8f0; flex-wrap:wrap; opacity:0→1 on hover; transform: translateY(3px)→0 }`；窄屏 `≤520px` 折叠 3 项入 More。
- **动作按钮通用**：`.mb-action-btn { 26×26px φ; border-radius:10px(--radius-md); bg:--bg-surface; border:--border-ghost }`；hover `translateY(-1px)` + `--shadow-md`；主复制按钮填 `#6366f1` 主色。
- **功能特定类**：
  - `.mb-rate-like.active { fill:#10b981 }` / `.mb-rate-dislike.active { fill:#ef4444 }`
  - `.mb-tts.playing::after { 右上 6px 红色 pulse 光点 }`
  - `.mb-fav.active { fill:#f59e0b; animation: mb-heart-beat 0.5s cubic-bezier }`
  - `@keyframes mb-heart-beat { 0%→50%(scale1.28)→100%(scale1) }`（φ 比例）
- **对话框深空样式**：`el-dialog` scoped 覆盖 → 圆角 14px、tab 与 φ 边距、底部按钮组 right align 10px 间距；反馈表单 label 对齐。
- **兼容性**：`@media (max-width:720px){ .mb-actions { gap:8px; padding-top:10px } .mb-action-btn { 24×24px } }`；折叠 More 按钮在 `≤520px` 显示。

### Step 4 · ChatView emit 占位 + 四段比例再布局
- **Emit 绑定**（在 `<MessageBubble>` 上补齐）：
  ```
  @rate="(m,r)=>onRate(m,r)"
  @share="m=>onShare(m)"
  @regenerate="m=>onRegenerate(m)"
  @to-doc="(m,p)=>onToDoc(m,p)"
  @favorite="(m,f)=>onFavorite(m,f)"
  @followup="(m,prompt)=>onFollowup(m,prompt)"
  @feedback="(m,payload)=>onFeedback(m,payload)"
  ```
- **占位处理函数**（统一用 `ElMessage.success/warning/info` toast + 合理副作用）：
  - `onFollowup(m,prompt)` → `draft.value = prompt + draft.value`；聚焦输入框（`chatInputRef?.focus()`）
  - `onRegenerate(m)` → 查找消息 index → 复用 `send()` 重跑（如有流式）或 toast 占位
  - `onToDoc(m,p)` → p.mode==='export-md' 写剪贴板；p.mode==='create-kb' toast "已提交到云盘（占位）"
  - `onRate/onFavorite/onShare/onFeedback` → 埋点计数 + toast 确认
- **顶栏 7 按钮重排**（FR11）：chat-tools 左侧加"新建对话"按钮；重排顺序为 新建/清空/导出/导入/转任务/创建项目/**mox 模块化系统架构分析（主渐变 φ 按钮）**；在顶栏下方新增分析阶段 5 chip 行（需求→架构→实现→测试→验收），颜色 φ 递进（冷蓝→靛→紫→粉→金），点击切换 `requirementFlowMode` + 跳对应 stage。
- **空态快捷问法改 3×2 grid**（FR11）：`.suggestions` 改 `display:grid; grid-template-columns:repeat(3,1fr); gap:16px; max-width:680px(φ·420); margin:auto`；每个 `.q` 改 φ 卡片（84×136，有图标+描述+标题，悬停抬升 2px）。
- **四段比例微调**：顶栏固定 86px、输入面板固定 164px；chat-body 自适应 (calc 100vh - 86 - 164 - 可能的分析阶段条 42)；空态 Orb 110×110 φ。

### Step 5 · 浏览器 E2E DOM 探针验证（AC1-AC12）
- 注入一条复合助手消息（content = 含 Mermaid + 2 个代码块 + 引用算子 3 个 + 置信度 0.93 + 联网检索 2 源的 Markdown 复合消息）。
- 逐条验证：
  - AC1: 计数 `.mb-action-btn` ≥ 9
  - AC2: 点击复制主按钮下拉 → 断言菜单项匹配三类
  - AC3: 点击朗读 → 断言按钮类名 `playing` 或 `speechSynthesis.speaking === true`；reload 后清零
  - AC4: 喜欢/不喜欢两次点击 + localStorage 读回
  - AC5: 分享点击 → toast 断言
  - AC6: 重新生成在 assistant/user/system 三角色 DOM 可见性断言
  - AC7: 转文档对话框 Tab×2 + Button×3 计数
  - AC8: 收藏点击 → `mb-heart-beat` 动画类
  - AC9: 追问点击 → 输入框内容匹配前缀 + `document.activeElement === textarea`
  - AC10: 反馈表单提交 → toast 出现
  - AC11: 既有回归 4 条探针
  - AC12: 顶栏 7 按钮 + 快捷问法 ≥6 卡片计数

---

## 依赖与注意事项
- **不新增 npm 依赖**；TTS / 分享 / 剪贴板全部 feature detect + 降级。
- **不改动既有复制/mermaid 路径**；新工具栏的"复制"与右上角 `.mb-ops` 并存（两条路径一致结果，肌肉记忆不冲突）。
- MessageBubble 可能无 `msg.id`；用 `stableId = hash(msg.timestamp + '|' + content)`（djb2 或简单 32bit hash）。
- Element Plus 图标在 scoped 中 import 即可；icon 大小统一 14px。
- `speechSynthesis.getVoices()` 在部分浏览器是异步加载，需监听 `voiceschanged` 事件。
- ChatView 输入框 ref 命名可能不同，Step 4 开始前先 grep 确认。

---

## 验证
- **HMR 热更新验证**：每次改动保存后，等待 dev 服务编译（<3s），探针断言页面无 SyntaxError、无白屏。
- **动作 9 项功能验收**：按 Step 5 12 条 AC 逐一断言（`browser_evaluate` 或 E2E 脚本）。
- **回归验证**：AC11 既有 5 项探针（pre、mermaid、svg、fence、复制三格式）必须 100% 通过。
- **视觉验收**：AR1-AR5 四条 rubric 人工检查 + 工具验证（按钮 26px、间距 10px、圆角 10px、颜色无硬编码）。

---

## 风险与处置
| 风险 | 概率 | 处置 |
|---|---|---|
| `speechSynthesis` 在 Chromium + iframe 或无头模式下无声 | 中 | 能力检测失败→按钮 disabled + tooltip；探针改为"检测按钮状态即可"，不强求真发声 |
| `navigator.share` 在非 HTTPS + 非移动环境不存在 | 高 | 自动回退到剪贴板复制分享卡片，toast 提示降级路径 |
| Mermaid/代码块在 `mdInstance.render` 内部统计数组与真实 DOM 顺序不一致 | 低 | fence renderer 内部同时写 DOM data-* 属性，最后用 `querySelectorAll` 回读对齐 |
| ChatView 输入框 ref 名与假设不同 | 低 | Step 4 前 grep `ref=".*Input"` 确认，无 ref 则用 `document.querySelector('textarea')` 聚焦 |
| 前端服务意外断开 | 低 | `netstat findstr :3021` 检查，必要时重跑 `npm run dev` |
| IDE `browser_take_screenshot` 超时（历史问题） | 高 | 放弃截图，改用 `browser_evaluate` DOM 断言 + `browser_get_console_logs` 无错误双重验证 |
