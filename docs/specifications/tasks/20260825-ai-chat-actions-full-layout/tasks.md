# 任务清单 · AI 消息动作工具条 & 全维分析最佳布局

> 对应规格：`.trae/specs/20260825-ai-chat-actions-full-layout/spec.md`
> 说明：每条任务均标注"所属 AC / 优先级 / 依赖"。所有 TR（局部验证）分为 `rule` 或 `rubric` 两类。

## Task 1：MessageBubble · 模板扩展（动作条 + 反馈/文档 2 对话框 + 朗读进度 + 折叠 More）

**目标**：在不破坏现有 `mb-bubble` / `mb-bubble-body` / `mb-meta` 的前提下，追加动作条 DOM（系统/助手/用户三角色可见性规则），并按需渲染"转文档编辑对话框"、"反馈对话框"两个 `el-dialog`。

**依赖**：无（第一项先做）

**优先级**：high

### TR
- rule：助手消息 `.mb-actions` 存在；9 个按钮/折叠项在展开 More 后合计 DOM 可见数 ≥9。
- rule：用户消息**不渲染**"重新生成"（querySelectorAll 返回 0）；系统消息仅渲染复制/朗读/(若 task_id)转任务。
- rule：宽度 ≤520px 时 `.mb-actions-more-btn` 存在；More 下拉展开后包含"喜欢/不喜欢/收藏/转文档/反馈"等。
- rule：两个对话框默认 `v-if=false`，首次触发动作后挂载并可见（aria-modal）。
- rubric（代码卫生 ≥3/4）：模板结构拆分语义化 section（header / ops-toolbar / body / meta / actions / dialogs），类名统一 `mb-*`，无 inline 样式硬编码色值或像素。

**Status**: pending

---

## Task 2：MessageBubble · 导入 & 图标 & 脚本状态骨架（9 动作状态机 + emits）

**目标**：集中导入 9 个动作对应 Element Plus 图标；新增组件级 ref/computed；新增 `defineEmits` 集合（8 项）；实现 localStorage 持久化 key/哈希 helper。

**依赖**：Task 1（模板已挂载对应动作结构）

**优先级**：high

### 变更点
- 图标导入扩展：`Star / StarFilled / Share / Refresh / Document / DocumentAdd / Promotion / ChatLineSquare / Flag / Microphone / VideoPause / VideoPlay / Warning / ThumbUp / ThumbDown`（具体以 `@element-plus/icons-vue` 实际命名为准）。
- `defineEmits(['goto-task','rate','share','regenerate','to-doc','favorite','followup','feedback'])`
- 状态 refs：
  - `speechState`（'idle' | 'playing' | 'paused'）、`speechUtterance: null | SpeechSynthesisUtterance`
  - `rating`（null | 'like' | 'dislike'）持久化
  - `favorited`（Boolean）持久化
  - `regenLoading`（Boolean）
  - `docDlgOpen` / `fbDlgOpen`（Boolean）
  - `fbForm`（`{ type, severity, description, includeContext:true }`）
  - `moreCollapsed`（响应式，按宽度阈值自动）
- 消息稳定 id helper：`stableMsgId(msg)` → 优先 msg.id → 否则 `hash(msg.timestamp + msg.role + msg.content.slice(0,200))`。

### TR
- rule：导入图标集合中"Star/Share/Refresh/ThumbUp/ThumbDown/Microphone/Flag/DocumentAdd/ChatLineSquare"至少 9 个均已 import 且无未使用警告（代码检查）。
- rule：`defineEmits` 声明的 8 项 new emits 均在脚本中至少被一次 `emit()` 调用。
- rule：`stableMsgId` 对同一 msg 连续 100 次调用值不变。
- rule：localStorage 写入的 rating/favorite 能在同一消息再次渲染时被正确读回并初始化。

**Status**: pending

---

## Task 3：MessageBubble · 一键复制（整则 Markdown + Mermaid 源码 + 每块代码）

**目标**：工具栏"一键复制"主点击=复制 Markdown；下拉列出每个代码块/Mermaid 块单独复制。

**依赖**：Task 2（mdInstance / renderedContent / copy* 函数已存在）

**优先级**：high

### 关键实现要点
- 工具栏"一键复制"组合下拉主按钮：
  - 主 click：直接调用现有 `copyMarkdown()`。
  - 下拉由 computed `copySubmenuItems` 动态生成：
    - 索引 0：整则 Markdown（默认）。
    - 随后：`document.querySelectorAll('.mb-mermaid-card')`（在 `instance.vnode.el` 范围内查找，避免跨消息污染），每张追加"复制 Mermaid 源码（N）"，其内容 = `decodeURIComponent(data-src)` 或 `<details> pre code`。
    - 随后：`.mb-fence` 每块追加"复制代码：{lang}（{N} 行）"，内容 = `pre code.innerText`。
  - 走 `copyTextUniversal` 统一成功/失败 toast。
- 保证在 `renderedContent` 变化 + DOM 更新完成 `nextTick` 后再计算子菜单项数量（computed 或 watch + nextTick）。

### TR
- rule：当存在 Mermaid（1）+ 代码块（1）：下拉子项数 ≥ 3（整则 + Mermaid + 代码）；且子项 innerText 中出现 `Mermaid` / `javascript` / `复制整则` / `默认` 关键词。
- rule：分别触发三项复制调用，toast 成功文字各不相同（"整则 Markdown…" / "Mermaid 源码…" / "代码块 javascript…"）。
- rule：重复点击不抛异常；剪贴板不存在时 fallback 到 execCommand 并 toast 反馈。
- rubric（可用性 ≥4/4）：下拉每一项 icon 语义对齐；每一项 tooltip 一行中文 ≤14 字。

**Status**: pending

---

## Task 4：MessageBubble · 朗读 TTS（Speech Synthesis）三态机 + 清理

**目标**：点击即播放、暂停/继续、停止；组件卸载或消息内容变化时强清理。

**依赖**：Task 2

**优先级**：high

### 关键实现要点
- `function startSpeak()`：
  - 若已在播放则暂停；若已暂停则 resume；否则 new `SpeechSynthesisUtterance(mdToPlainText(content))`。
  - `lang='zh-CN'`；`rate=1.0`；`pitch=1.0`。
  - 绑定 `onstart / onend / onpause / onerror` 更新 `speechState`。
- `function stopSpeak()`：`speechSynthesis.cancel()`。
- `watch(()=>props.msg.content,()=>stopSpeak())`；`onBeforeUnmount(()=>stopSpeak())`。
- 能力缺失时按钮 `disabled` + `aria-disabled` + tooltip。

### TR
- rule：调用 1 次 `startSpeak()` 后 `speechState === 'playing'`（或 `speechSynthesis.speaking === true`）。
- rule：第二次点击转 `paused` / resume；第三次 `onend` 后自动回到 `idle`。
- rule：`onBeforeUnmount` + 手动 `speechSynthesis.speak` 2 条 utt 再卸载后，`speechSynthesis.pending + speaking === 0`（若浏览器状态可靠）。
- rule：浏览器无 speechSynthesis 时按钮 `.is-disabled` 类存在。
- rubric（无障碍 ≥3/4）：播放状态 aria-live 文本更新"朗读中…已 20s"。

**Status**: pending

---

## Task 5：MessageBubble · 喜欢 / 不喜欢 / 收藏（持久化 + emit）& 分享（WebShare+Clipboard）

**依赖**：Task 2

**优先级**：high

### 关键实现要点
- 喜欢/不喜欢：
  - `toggleRating(kind)` 三态互斥；emit `rate(msg, rating|null)`；写 localStorage。
- 收藏：
  - `toggleFavorite()`；心跳动效 class；emit `favorite(msg,favorited)`；写 localStorage 数组 `['ous_msg_favs']`。
- 分享：
  - `async function doShare()`：若 `navigator.share` 存在 → `navigator.share({title,text,url})`；否则构造分享卡片（文案如 `[璇玑助手] 来自 {sender} 的消息（{time}）：{80字摘要} · {算子n}枚 · {引用m}条 · 打开链接：{url}`）并写入剪贴板。

### TR
- rule：喜欢 1 次 → rating = 'like'；再点"不喜欢" 1 次 → rating = 'dislike'；再点"不喜欢" → rating = null；三次 emit 事件 payload 正确。
- rule：收藏后刷新页面能读回填充 + 心跳动画 class `.mb-heart-beat`。
- rule：无 Web Share API 环境下，点击分享后剪贴板内容含 `http://localhost:3021/#/ai`（或与 location.href 一致）。
- rubric（美学 ≥4/4）：喜欢绿色、不喜欢红色、收藏金色，视觉对比克制；收藏动效 φ 尺度（scale 1.0→1.28→1.0，260ms）。

**Status**: pending

---

## Task 6：MessageBubble · 重新生成 + 转文档编辑对话框（Markdown 编辑 + 预览 Tab + 三出口）

**依赖**：Task 1（对话框 DOM）、Task 2（emit）

**优先级**：high

### 重新生成
- 仅助手消息；`async function doRegenerate()`：`regenLoading = true`；`emit('regenerate', msg)`；父组件 1.2s 未回包时兜底 `setTimeout(() => {regenLoading=false; ElMessage({...})}, 1400)`。

### 转文档对话框
- 两 Tab："Markdown 编辑"（textarea）与"实时预览"（md-body v-html）。
- 统计字数：`content.length` + 阅读时长 `ceil(len/500)` 分钟。
- 三出口：
  1. `submitAsKb()` → `emit('to-doc',msg,{mode:'create-kb',markdown})` + toast。
  2. `exportMarkdown()` → `tryWriteRichClipboard(html, markdown)` + toast。
  3. `cancel()` → `docDlgOpen=false`。

### TR
- rule：用户消息 querySelectorAll("[data-action=regenerate]") 返回 0；助手消息 ≥ 1。
- rule：转文档对话框打开后 TabPane 两个且标题分别为"Markdown 编辑"与"预览"。
- rule：对话框底部 3 个按钮文本分别包含"新建为云盘文档"、"导出 Markdown"、"取消"。
- rule：编辑区内容与初始 `msg.content` 逐字一致；编辑后切换"预览"Tab，DOM 中至少出现 1 个 `<h1>` 或 1 个段落。
- rubric（产品感 ≥3/4）：对话框尺寸 680px、φ 内边距 42/26（--space-6/5）；Tab 切换不抖动；字数/阅读时长位于右上角。

**Status**: pending

---

## Task 7：MessageBubble · 追问（Follow-up）回填输入框 + 反馈对话框（5类型×4严重程度）

**依赖**：Task 1、Task 2

**优先级**：high

### 追问
- 函数：`doFollowup()`：
  - 生前缀策略（根据 `msg.content.length` 与 `msg.role`）；
  - `emit('followup', msg, prompt)` 让父级写入 `draft` 并 `textarea.focus()`。

### 反馈对话框
- 字段：类型 radio 5 项、严重程度 radio 4 项、描述 textarea ≤500 字、"联系上下文" checkbox 默认勾选。
- `submitFeedback()` 校验必填、构造 payload、`emit('feedback', msg, payload)`、toast 成功、关闭对话框并 reset 表单。

### TR
- rule：追问点击后输入框 `document.activeElement === textarea`，且 `draft.value.startsWith("关于以上内容")` 或 `draft.value.startsWith("请继续展开")`。
- rule：反馈对话框未填必填项点击提交时 toast 提示"请选择反馈类型与严重程度"，对话框**不关闭**。
- rule：全填提交后父级 `feedback` 事件计数 +1、类型/严重程度字段与输入一致、描述 ≤500。
- rubric（表单体验 ≥3/4）：radio 项对齐左标签 φ 间距；描述 textarea 默认高度 6 行；字数统计显示 0/500；提交后表单自动清空。

**Status**: pending

---

## Task 8：MessageBubble · 样式补齐（动作条 10 样式、对话框样式、动画）+ 响应式折叠

**依赖**：Task 1-7（模板脚本基本完成）

**优先级**：high

### 关键样式
- `.mb-actions`：`flex-wrap: wrap; gap: var(--space-3); margin-top: var(--space-4); padding-top: var(--space-4); border-top: 1px dashed var(--border-ghost);`
- `.mb-action-btn`：`26×26` / `--radius-md` / `--bg-surface` / `--border-ghost` / 悬停抬升 + `--shadow-md` / `transition: transform .2s cubic-bezier(.4,0,.2,1), box-shadow .2s`。
- `.mb-action-primary`（复制主按钮）：`background: linear-gradient(135deg,#6366f1,#8b5cf6); color:#fff; border-color: transparent; box-shadow: 0 6px 18px -8px rgba(99,102,241,.55)`。
- 喜欢/不喜欢/收藏 active：`.mb-rate-like.active { color:#10b981 }`、`.mb-rate-dislike.active { color:#ef4444 }`、`.mb-fav.active { color:#f59e0b }` + 心跳动画 `mb-heart-beat`。
- TTS playing：`.mb-tts.playing { color:#6366f1; &::after { content:""; width:6px; height:6px; border-radius:50%; background:#6366f1; display:inline-block; margin-left:4px; animation: mb-tts-pulse 1s infinite; } }`
- More 折叠：`.mb-actions-more-btn` + 下拉菜单。
- 对话框：`el-dialog__body` 统一 padding；两 Tab 高度 ≥ 320；字数/时长居右。

### TR
- rule：`.mb-action-btn` computed `height`=26±1，`border-radius`=10±1；悬停 `transform` 至少一个属性变化（`translateY / box-shadow`）。
- rule：主复制按钮的 `background` 包含 `linear-gradient` 与 `#6366f1`。
- rule：心跳/脉冲/TTS 三个 `@keyframes` 存在且被引用。
- rule：`@media (max-width:720px)` 下动作条折叠为 5+More；More 下拉展开后依然包含喜欢、不喜欢、收藏、追问、反馈。
- rubric（一致性 ≥4/4）：所有间距/圆角/阴影/色值均复用 global.css 变量或深空色系，无硬编码 #xxxxxx 色值超过 3 处。

**Status**: pending

---

## Task 9：ChatView · 接线（8 个新 emit 占位处理）+ 顶栏/空态/输入面板 4 段 φ 再布局

**依赖**：Task 1-8（MessageBubble 完成）

**优先级**：high

### 接线
- MessageBubble 模板新增 v-on：
  ```vue
  <MessageBubble
    v-for="(m,i) in messages" :key="i" :msg="m"
    @goto-task="goToTaskDetail"
    @rate="(m2,r)=>onRate(m2,r)"
    @share="m2=>onShare(m2)"
    @regenerate="m2=>onRegenerate(m2)"
    @to-doc="(m2,p)=>onToDoc(m2,p)"
    @favorite="(m2,f)=>onFavorite(m2,f)"
    @followup="(m2,prompt)=>onFollowup(m2,prompt)"
    @feedback="(m2,payload)=>onFeedback(m2,payload)"
  />
  ```
- 每个 handler 做最小占位：toast + 日志 + 若 followup 则写 `draft.value = prompt + (draft.value ? ' ' + draft.value : '')` 并聚焦输入框。
- `onRegenerate(msg)`：真·可选复用 send——查找该条消息在 messages 中的 index，保留该 index 之前的上下文，删除该条（及之后的助手消息，若连着），然后调用 send() 之前的用户消息（或最接近的 user 消息）进行重生成；如未找到则 toast 占位。

### ChatView 布局再调整（FR11）
- **顶栏**（`.chat-header`）：
  - 左："🧠 璇玑系统"、"新建对话"按钮；
  - 中：面包屑/会话标题（已有）保留；
  - 右：按顺序排列"清空 / 导出 / 导入 / 转任务 / 创建项目 / 全维分析（primary）"6 按钮（新建已在左边），中间 φ 间距 10px。
- **快捷问法（quickQuestions）升级为 3×2 Grid**：`.suggestions` 由 `flex-wrap` 改为 `grid-template-columns: repeat(3, 1fr)`（≤960px 降 2；≤640px 降 1）；每个 chip 卡片有 φ 内边距 16×26、圆角 26（--radius-xl）、悬浮抬升 + 品牌底纹渐变。
- **分析阶段指示器**：在 `suggestions` 上方加一行 5 阶段 chip（`需求→架构→实现→测试→验收`），φ 色系递进（靛→青→翠→金→红），点击则切 requirementFlowMode 阶段（若存在）。

### TR
- rule：顶栏按指定顺序包含"新建对话、清空、导出、导入、转任务、创建项目、全维分析"7 个交互按钮（允许 1 两个位置合并但文字齐全）。
- rule：快捷问法卡片 `display`=grid，`grid-template-columns` 包含 "repeat(3" 或至少含 3 列 CSS；单卡片 `border-radius` ≥ 18。
- rule：分析阶段 5 个 chip 文字命中 "需求/架构/实现/测试/验收" 至少 4 个。
- rule：点击助手消息的 `@regenerate` → ChatView `onRegenerate` 被调用，且 1.5s 内要么：（a）旧助手消息从 messages 消失并出现新 loading 动画；或（b）toast 提示"重生成完成（占位）"。
- rule：点击"追问" → `draft.value` 非空 && `textarea.focus === true`。
- rubric（全维分析布局质感 ≥4/4）：顶栏 86px 固定、聊天体 809、输入面板 164 三段比例清晰；快捷卡片悬停出现柔和品牌阴影（多层柔边），卡片文字与图标对齐。

**Status**: pending

---

## Task 10：HMR 检查 + 浏览器回归探针（11 条 rule 全过 + rubric 评）

**依赖**：Task 1-9 全完成

**优先级**：high

### TR（每条对应 spec.md AC）
- rule AC1：助手消息动作数 ≥ 9（含 More 下拉）。
- rule AC2：复制子菜单 ≥ 3 子项且含 Mermaid/代码/整则三类关键词。
- rule AC3：TTS 播放→暂停→停止（speaking 状态或类名切换）。
- rule AC4：喜欢/不喜欢持久化（刷新后仍读回）。
- rule AC5：分享剪贴板或 toast 包含 URL。
- rule AC6：仅助手有 regen；用户/系统为 0；点击 regen 有 loading。
- rule AC7：转文档对话框两 Tab + 三按钮。
- rule AC8：收藏填充 + 心跳动画。
- rule AC9：追问焦点 + draft 前缀正确。
- rule AC10：反馈提交成功 toast + emit 计数 ≥1。
- rule AC11：既有回归 `pre≥3`、`mermaidCards≥1`、`targetSvgCount≥1`、`fenceCards≥1`（4 条）。
- rule AC12：顶栏 7 按钮 + 快捷问法 grid ≥ 6 卡。
- rubric AR1-AR5：按 0-4 分逐项自评并附证据片段（DOM probe 文字）。

### 完成证据
- 需粘贴每个 rule 的 `browser_evaluate` JSON 结果片段；AR1-AR5 评分 + 理由。
- 如有截图（哪怕仅 1 张）附路径。

**Status**: pending
