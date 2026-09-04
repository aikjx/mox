# AI 消息动作工具栏与mox 模块化系统架构分析布局规格

## 一、问题（Problem）

璇玑信息知识图谱关联关系系统的 AI 助手页面当前仅提供"复制（三格式）"单条消息操作。用户在真实协作与专家联盟场景下，针对单条对话（尤其是 AI 助手输出的 Markdown + Mermaid + 代码块 + 算子元数据复合消息）高频需要以下动作：

1. **一键复制**：默认 Markdown；当存在代码块/Mermaid 时，可直接复制对应块的内容（延续现有 Fence 内复制按钮的精神，再在动作工具栏上聚合"一键复制消息 + 一键复制代码&图表"两条路径）。
2. **朗读**：Web Speech Synthesis（TTS）播放、暂停、停止、重听；兼容中文，支持中途打断。
3. **喜欢 / 不喜欢**（点赞/点踩）：对单条消息打分，用于后续专家联盟质量回流（不影响前端渲染）。
4. **分享**：复制一条"可分享卡片"（标题 + 摘要 + 消息哈希 + 当前会话 URL），同时回退为仅复制当前页链接；浏览器支持 Web Share API 时走原生分享。
5. **重新生成**：仅对 AI 助手消息可见；清空该条并基于上下文重跑流式生成。
6. **转为文档编辑**：把整条消息内容（Markdown）弹到一个可编辑对话框内，支持"导出到云盘/新建文档/复制 Markdown"三条出口占位。
7. **收藏**：对单条消息打星收藏，前端维护已收藏消息 id 集合，UI 有填充星图标 + 微动效 + 轻量 toast。
8. **追问**（Follow-up）：把消息摘要（或默认 "关于以上内容，我想追问："）填入输入框，并自动聚焦输入框。
9. **反馈**（Report / Flag）：对话框收集反馈类型（事实错误/格式问题/幻觉/其他）、严重程度、描述文本，提交给父级占位（不阻塞）。

同时用户提出"架构开发专家联盟，mox 模块化系统架构分析，怎么布局好。最伟大的产品，最好用"——因此需要对 ChatView 工具栏、空态、输入面板、结果面板四者进行**黄金比例（φ≈1.618）× 深空极简**再布局，达到比上一版本更统一、克制、易用的视觉与操作体验。

## 二、目标用户（Users）

| 用户画像 | 典型场景 | 本规格解决的痛点 |
|---|---|---|
| 架构开发专家联盟成员（架构/算法/图谱/算子/融合/自动化专家） | 输出高质量分析后，需要"一键复制/转文档/收藏/重生成/追问"做二次加工 | 过去只有复制，没有协作、回溯、复用的闭环 |
| 一线业务/项目经理（非技术） | 阅读 Mermaid 架构图与指标表格后，需要"朗读/分享/反馈"与团队对齐 | 过去没有无障碍朗读、没有结构化反馈通道 |
| 平台治理/质量运营 | 抽样喜欢/不喜欢/反馈，沉淀专家联盟的质量数据 | 过去无评分、无反馈、无法量化回复质量 |

## 三、目标（Goals）

- **功能目标**：9 个动作全部在单条消息（助手/用户/系统三角色，按可见性规则）可用，DOM 探针可验证存在、数量、交互路径不回归旧功能（复制/mermaid/pre/markdown）。
- **体验目标**：动作工具栏符合 φ 比例尺寸、深空色系、悬停微动、折叠不遮挡正文正文正文正文正文正文正文；与既有 mb-ops（复制下拉）并列但不冲突。
- **性能目标**：工具栏所有动作在 1 次点击内启动；TTS 首次加载无延迟占位；对话框打开 ≤ 80ms；新增代码不影响既有 `MessageBubble` 的 Markdown/Mermaid 渲染时延（不新增同步长任务）。
- **稳定性目标**：未检测到的浏览器能力（ClipboardItem / SpeechSynthesis / Web Share）一律降级并告知；所有 emit 到 `ChatView` 的动作即使父级未完全实现，组件端也不抛错。

## 四、非目标（Non-Goals）

- *不*实现真实后端"重新生成"流式接口的重写；前端只触发 `@regenerate` 并做好 UI 占位（含 loading/错误文案）。
- *不*改变已有数据契约（`msg.role / msg.content / msg.confidence / msg.referenced_operators / msg.web_search / msg.artifacts`），只新增前端内聚的交互状态（喜欢、收藏、朗读进度、反馈对话框）。
- *不*引入新的第三方依赖；朗读走浏览器原生 SpeechSynthesis；分享走原生 Web Share + Clipboard；收藏/喜欢状态仅用组件内 ref + 可选 `localStorage` 持久化（key 命名统一前缀）。
- *不*改动 SessionSidebar、ToolDrawer、其他业务组件；改动范围限定为 `MessageBubble.vue`（主）与 `ChatView.vue`（emit 占位处理+小布局优化），`global.css` 只在必要时补 1-2 条全局变量。
- *不*重新实现分享/朗读/反馈的企业级后端。"架构开发专家联盟·mox 模块化系统架构分析"在本规格内聚焦**前端最佳布局与动作闭环的产品设计落地**，而非后端功能实现。

## 五、功能需求（Functional Requirements，FR）

### FR1 动作工具栏：可见性与布局
- 工具栏出现在**消息气泡**底部元数据下方，仅对"助手 + 用户"两种角色渲染；系统提示气泡渲染"复制 + 转任务"（如存在 task_id）+ 朗读 三项。
- 工具栏**按内容自适应**：
  - 当 AI 助手消息**存在代码块（.mb-fence）或 Mermaid 图表（.mb-mermaid-card）**时，显示 "一键复制内容（含代码&Mermaid）"按钮（作为主操作），并弹出下拉列出每个具体块的独立复制项。
  - 当消息为纯文本，该按钮降级为"一键复制（默认 Markdown）"。
- 顺序（从左到右，严格对齐 φ 间距）：
  1. **一键复制**（组合下拉 / 主色 · 靛蓝）
  2. **朗读**（TTS 三态：未播/播放中/已暂停）
  3. **喜欢**（拇指向上，互斥于"不喜欢"）
  4. **不喜欢**（拇指向下）
  5. **分享**（分享图标，支持 Web Share 时触发原生）
  6. **重新生成**（仅助手消息，刷新图标）
  7. **转文档编辑**（文档图标，打开对话框）
  8. **收藏**（五角星，点击填充 + 心跳动画）
  9. **追问**（灯泡图标，向输入框回填前缀）
  10. **反馈**（旗帜图标，打开反馈对话框）
- 工具栏在气泡宽度 `≤ 520px` 时折叠 3 个为"更多"下拉（喜欢/不喜欢/追问在首屏；其余收纳）。
- 工具栏**不遮挡正文**：放在 `.mb-meta` 下方，与正文分离，鼠标悬停气泡时工具栏使用淡入 + 上滑 3px 微动效。

### FR2 一键复制（含代码块 / Mermaid 聚合）
- 保留原有**复制下拉**（消息级三格式）作为 `.mb-ops` 的主按钮；在工具栏新增的"一键复制"为另一入口：
  - 工具栏"一键复制"主点击 = **复制为 Markdown**（与原默认一致，保证肌肉记忆不打破）。
  - 工具栏"一键复制"下拉首项 = "复制整则 Markdown（默认）"。
  - 如果存在 ≥1 个 `.mb-mermaid-card`，为每张图追加一项"复制 Mermaid 源码（{序号}）"。
  - 如果存在 ≥1 个 `.mb-fence`，为每块代码追加一项"复制代码：{语言}（{行号} 行）"。
  - 每次复制成功均走统一成功路径：ElMessage toast 1.5s + 按钮绿色反馈。

### FR3 朗读（Speech Synthesis）
- 检测 `window.speechSynthesis` 能力；不可用则按钮 disabled 并显示 tooltip"当前浏览器不支持朗读"。
- 朗读内容 = `mdToPlainText(content)` 的结果（去掉 Markdown 语法保留自然语义）。
- 三态机：
  - `idle` → 点击开始播放；按钮图标改为"暂停"；出现进度光点。
  - `playing` → 点击暂停（保留进度）。
  - `paused` → 点击继续播放。
  - 长按或右键菜单支持"停止/重头播放"。
- 组件卸载、消息内容变化、切换会话时，自动 `speechSynthesis.cancel()` 避免泄漏。
- 中文优先：`zh-CN` voice 若存在则优先选，否则 fallback 系统默认 voice；语速=1.0，音调=1.0。

### FR4 喜欢 / 不喜欢（Rating）
- 互斥三态：null / like / dislike。点击"喜欢"再次点击 = 取消；点击"不喜欢"互斥清"喜欢"。
- UI：点赞按钮填充 + 绿色（like）；点踩填充 + 红色（dislike）。
- 持久化：可选 `localStorage['ous_msg_rating_' + msg.id]`；若 msg.id 不存在则回退到 `msg.timestamp + msg.content` 哈希。
- emit `@rate(msg, 'like' | 'dislike' | null)` 至 ChatView。

### FR5 分享（Share）
- 优先使用 `navigator.share({ title, text, url })`（Web Share API）；无则走"复制分享卡片"路径：
  - 分享卡片 = 一行摘要（包含消息发送者、时间、内容前 80 字、引用算子数量）+ 当前 `location.href` 。
  - 复制成功 ElMessage toast。
- emit `@share(msg)` 到父组件（可扩展埋点）。

### FR6 重新生成（Regenerate · 仅助手）
- 仅对 `msg.role === 'assistant'` 渲染。
- 点击：按钮进入 loading 态；**emit `@regenerate(msg)`**；由父级处理重生成逻辑；组件不直接发起网络请求。
- 父级未处理时组件兜底 1.5s 后关闭 loading 并提示"当前环境暂不支持重生成"。

### FR7 转为文档编辑（Document Editor）
- 打开一个对话框（`el-dialog`），宽 φ·420 ≈ 680px（≤1024 屏降为 92vw）。
- 对话框 Tab：
  - **Markdown 源码编辑**：`el-input` type="textarea" 显示内容，支持 80×24；
  - **预览**：复用 MessageBubble 同一个 mdInstance 渲染 HTML。
- 对话框底部 3 个按钮（占位）：
  1. "新建为云盘文档" → emit `@to-doc(msg, { mode:'create-kb', markdown })` + toast "已提交到云盘（占位）"。
  2. "导出 Markdown" → emit `@to-doc(msg, { mode:'export-md', markdown })` + 浏览器 Clipboard 写 markdown（ClipboardItem 同时写 html）。
  3. "取消" → 关闭。
- 对话框内有字数统计 + 估计阅读时长（中文 500 字/分钟）。

### FR8 收藏（Favorite / Star）
- 点击五角星，`favorited` 状态切换；true=金色填充 + 心跳 1 次 scale 1.0→1.28→1.0 animation（φ 跳动）。
- 持久化：`localStorage['ous_msg_favs']` 存储 JSON array of ids；无 id 用 hash。
- emit `@favorite(msg, favorited)`。

### FR9 追问（Follow-up）
- 点击：生成 `追问前缀`，回填输入框并聚焦；前缀策略：
  - 若 `msg.role === 'assistant'` 且内容较长，默认填 "请继续展开说明刚才的回答："；
  - 若内容较短，默认填 "关于以上内容，我想追问："；
  - 回填后光标自动停在末尾，可直接继续输入。
- emit `@followup(msg, prompt)` 到父组件，父组件设置 `draft.value = prompt + draft.value` 并聚焦输入框。

### FR10 反馈（Report Issue）
- 打开对话框，表单字段：
  - **反馈类型**（单选组，必填）：事实错误 / 格式错乱 / 幻觉或不合规内容 / 代码块报错 / Mermaid 渲染报错 / 其他
  - **严重程度**（单选组，必填）：轻微 / 一般 / 严重 / 阻塞
  - **详细描述**（textarea，选填）≤500 字
  - **联系上下文一起发送**（默认勾选）checkbox
- 提交：emit `@feedback(msg, payload)` → toast "反馈已提交，感谢助力专家联盟质量升级"。
- 取消：不触发 emit，关闭对话框清空表单。

### FR11 ChatView 页面整体再布局（mox 模块化系统架构分析最佳布局）
围绕"AI 对话 = mox 模块化系统架构分析工作台"，把 ChatView 分成严格 4 段比例：

| 区块 | 高度比例（相对 chat 容器） | 高度 px（1080p 典型 940 可用） | 说明 |
|---|---|---|---|
| 顶栏（logo/会话切换/新建/清空/导出/导入/转任务/创建项目/mox 模块化系统架构分析 CTA） | 固定 86px | 86px | 顶部固定；"mox 模块化系统架构分析"按钮为 φ 强调渐变主按钮；"📝需求文档/🔄流程图/💻开发测试"在右侧次级 |
| 空态（无会话/无消息）与快捷问法区 | 按 φ 上/下 61.8% 对齐 | ≈580 | Orb 居中 110×110 光球；快捷问法 `quickQuestions` 3×2 grid 84×136 chip φ 尺寸卡片（非 tag）悬停抬升 |
| 聊天体（消息）+ 思考动画 | φ·500 ≈ 809（随内容滚动） | 809 | chat-body 径向渐变背景（延续上一轮）；φ 边距 |
| 输入面板（模式切换/联网/制品/附件/草稿/发送） | 固定 164px | 164px | 输入框高 100；16 号 φ 间距序列；发送按钮 42×42 φ 大号圆角 |

- 在"mox 模块化系统架构分析 CTA 下方"新增一行**分析阶段 Chip 指示器**：需求→架构→实现→测试→验收 5 阶段（φ 颜色递进）；点击跳转到 ChatView 已有 requirementFlowMode（若存在）。
- 工具栏"清空/导出/导入"对齐顶栏右侧，顺序从左到右：新建对话 / 清空 / 导出 / 导入 / 转任务 / 创建项目 / mox 模块化系统架构分析（主）。

## 六、非功能需求（Non-Functional Requirements，NFR）

### NFR1 一致性与设计令牌
- 所有新组件样式严格复用 `global.css` 已发布深空令牌：`--brand / --space-1..8 / --radius-sm..2xl / --shadow-{sm,md,lg} / --shadow-inset / --text-*`；**不新增颜色硬编码**（仅喜欢/不喜欢用已有的 success/danger 变量）。
- 新动作按钮尺寸统一：`26×26px`（--space-5 φ）、圆角 `--radius-md`（10px）、背景 `var(--bg-surface)`、边框 `var(--border-ghost)`、hover 抬升 `translateY(-1px)` + `--shadow-md`；主色选中态填充 `#6366f1`。
- 对话框 100% 沿用 Element Plus + scoped CSS，不引入新的对话框组件。

### NFR2 可用性/可访问性
- 工具栏每个按钮必须有 `aria-label` 与 `title`（中文）。
- 朗读有 `aria-live="polite"` 区域报告播放状态。
- 收藏按钮状态 `aria-pressed`；喜欢/不喜欢 `aria-pressed`。
- 对话框焦点 Tab 顺序：默认首输入；Esc 关闭。
- 键盘友好：工具栏可聚焦，Enter/Space 触发。

### NFR3 兼容性与降级
- 所有浏览器 API（Clipboard、ClipboardItem、SpeechSynthesis、Web Share、`v-html` 注入）均先 feature detect，不支持则按钮 disabled + tooltip。
- 旧浏览器下，所有复制退化为 `execCommand` fallback（MessageBubble 现有 `_fallbackCopy` 复用）。
- 移动端（≤720px）工具栏折叠至 5 图标 + "更多"。

### NFR4 性能
- 组件初始化新增开销 ≤ 10ms；Markdown/Mermaid 渲染路径不增加同步副作用。
- 所有对话框渲染按需（`v-if`），不在首屏一次性渲染 3 个 dialog；反馈对话框默认不渲染。
- 朗读使用 `onBeforeUnmount` / `watch(msg.content)` 联合清理，`cancel()` 保证不残留全局播放状态。

### NFR5 稳定性与契约
- `defineEmits` 新增：`rate / share / regenerate / to-doc / favorite / followup / feedback` + 已有 `goto-task`；所有 emit 命名小写 + 短横线。
- ChatView 对每个新 emit 提供**最小占位实现**（toast + 合理副作用），保证父级不报 "Missing handler"。
- 运行时 SyntaxError 目标 0；若新增第三方则不允许。
- 对 Mermaid/复制等既有验证探针（preCount/mermaidSvgCount/toast/menu-item）必须仍全数通过，不允许回归。

## 七、约束（Constraints）
- 改动文件范围仅限：
  - `frontend-ui/src/components/MessageBubble.vue`（主要：模板动作条 + 脚本 9 动作 + 样式）
  - `frontend-ui/src/views/ChatView.vue`（次要：@emit 占位 + mox 模块化系统架构分析 4 段再布局）
  - `frontend-ui/src/styles/global.css`（如确有必要，仅补设计令牌，不覆盖旧值）
- **不新增 npm 依赖**。
- 保持端口 http://localhost:3021 服务不变；HMR 后需回归探针。
- 所有中文 UI 文本保持一致风格：克制、企业级、金融级（无 emoji 过度使用）。

## 八、依赖与假设（Dependencies / Assumptions）
- 浏览器原生：`speechSynthesis`、`navigator.share`、`navigator.clipboard`、`ClipboardItem` 存在或不存在均可。
- ChatView 存在 `draft`（输入框内容 ref）、`messages`、`send()`、`sendQuick()`、`scroll()`、`thinking` 等既有 ref/函数；若不存在或命名不同，wire-up 时需改为等价实现。
- 单条消息对象可能没有 `id`；组件端使用一个稳定哈希作为存储 key（内容+时间戳+sha-like or 自增 index）。

## 九、开放问题（Open Questions）
本规格默认"前端最佳产品级落地 + 后端占位"：
- Q1："重新生成"是否需要接入现有 ai-engine 流式 SSE（前端在 ChatView 直接复用 send 的 SSE）？本规格暂定**占位即可**，但在 tasks.md 中会给出"直接复用 send() 实现真重生成"的可选任务，如用户批准则启用。
- Q2："转文档编辑→新建为云盘文档"是否直接跳转到云盘知识库新建页？本规格默认**toast 占位 + emit 路由跳转可选**，后续可接。
- Q3：反馈/喜欢/收藏是否需要同步后端？本规格默认**本地持久化 + emit 留接口**，真实落库由后端接入。

## 十、验收标准（Acceptance Criteria）

### 功能验收（rule）

| ID | 类型 | 内容 | 通过证据 |
|---|---|---|---|
| AC1 | rule | 每条助手消息 DOM 中存在 9 个动作按钮或下拉子项（复制、朗读、喜欢、不喜欢、分享、重新生成、转文档、收藏、追问、反馈，总计 9 项；若含折叠则仍能在下拉展开后计数 ≥9）。 | `browser_evaluate` 计数助手消息的 `.mb-action, .mb-actions button, [data-mb-action]` 等选择器合计（含折叠菜单子项）≥ 9 |
| AC2 | rule | 一键复制下拉在同时存在 Mermaid + JS 代码块时：显示 "复制整则 Markdown（默认）" + "复制 Mermaid 源码（1）" + "复制代码：javascript（4 行）" 等 ≥3 子项。 | evaluate 取 dropdown 菜单项 innerText，断言匹配上述 3 种模式 |
| AC3 | rule | 朗读点击后：`speechSynthesis.speaking === true` 或按钮 `.mb-tts-playing` 类名出现；组件卸载（或切换会话 reload）后 `speechSynthesis.pending + speaking === 0`。 | evaluate 状态 + 卸载后清零 |
| AC4 | rule | 喜欢/不喜欢互斥；第二次点击同按钮取消；刷新页面仍保留（localStorage 持久化）。 | click×2 + reload + 选择器断言 `.mb-rate-like.active` 类 |
| AC5 | rule | 分享按钮在不支持 Web Share 的环境里，点击后 clipboard 内容包含 `http://localhost:3021/#/ai`（当前 URL）。 | evaluate 模拟 click + 读取 `navigator.clipboard.readText()`（若权限允许）或断言 toast "已复制" |
| AC6 | rule | 重新生成按钮仅助手消息可见；用户消息/系统消息 DOM querySelectorAll 返回 0。且点击后：loading 动画类名 `.mb-regen-loading` 存在或 emit `regenerate` 被触发计数 ≥1。 | evaluate 可见性 + 父组件 emit 监听计数 |
| AC7 | rule | 转文档对话框点击打开：`el-dialog` 类名对话框可见；Tab 两个（Markdown 编辑 / 预览）；底部"新建为云盘文档/导出 Markdown/取消"三按钮存在。 | snapshot 或 evaluate 计数 |
| AC8 | rule | 收藏按钮点击切换，填充金色类名出现 + `@keyframes mb-heart-beat` 动画执行（`animationName` 命中）。 | evaluate 类名 + computedStyle 断言 |
| AC9 | rule | 追问按钮点击：ChatView 输入框 `draft` 值变为前缀文字（如 "关于以上内容，我想追问："）且输入框 DOM focus === true。 | evaluate document.activeElement === textarea + 内容匹配 |
| AC10 | rule | 反馈对话框表单字段齐全；提交后 ChatView emit `feedback` 计数 ≥1，页面显示 toast"反馈已提交…"。 | evaluate 计数 + toast DOM |
| AC11 | rule | 既有回归：`preCount ≥ 3`、`mermaidCards ≥ 1`、`targetSvgCount ≥ 1`、`fenceCards ≥ 1`、`secure && cb && ci === true`（复制三格式）。 | 浏览器 DOM 探针（上一版本交付已通过的 4 条） |
| AC12 | rule | ChatView 顶栏按 FR11 顺序存在"新建对话/清空/导出/导入/转任务/创建项目/mox 模块化系统架构分析"7 个按钮/链接；"mox 模块化系统架构分析"为 primary。空态存在 3×2（或 2×3）≥6 个快捷问法 φ 卡片。 | evaluate top-bar DOM query |

### 质量验收（rubric）

| ID | 维度 | 刻度 | 通过阈值 | 证据源 |
|---|---|---|---|---|
| AR1 | 动作布局美学（深空×φ） | 0=乱/2=对齐/4=精品 | ≥4 | 视觉审查：按钮高度 26、间距 10、圆角 10、悬停抬升阴影一致；与元数据区有 12/16 px 呼吸留白；与 global.css 变量一致率 100% |
| AR2 | 操作可发现性 | 0=找不到/2=可发现/4=直觉 | ≥4 | 悬停气泡即可看到复制按钮+工具栏；9 项 ≤3 秒定位；折叠菜单中文字体清晰、tooltip 1 行中文不超过 14 字 |
| AR3 | 无回归稳定性 | 0=回归/2=轻微/4=零回归 | ≥4 | AC11 全部通过，上一版本 DOM 探针全达标；不新增 SyntaxError |
| AR4 | 无障碍与降级 | 0=缺/2=部分/4=完备 | ≥3 | aria-label 覆盖率 100%（9 按钮）；对话框可 Esc 关闭；朗读支持关闭与停止；TTS/Share 能力缺失时按钮 disabled + tooltip；移动端 ≤720px 折叠为 5+More |
| AR5 | 代码卫生（前端架构一致性） | 0=乱/2=一般/4=典范 | ≥3 | 与现有 `MessageBubble.vue` 风格一致：`ref/computed/watch/onMounted` 结构清晰；图标导入集中；所有 `defineEmits` 声明类型（object 键数组）；无 console.log 遗留；无硬编码色值 |
