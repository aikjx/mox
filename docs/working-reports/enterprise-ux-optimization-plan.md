# 企业级产品最优体验改造（产品设计专家视角）实施计划

## Repository Research（产品现状盘点）

产品矩阵：36 视图 + 24 菜单项（`NAV_MODULES`），顶栏=面包屑+健康+用户菜单，多页签 tabs 随路由自动生成，门户 Dashboard 有 4 个快速入口 + 24 模块卡片 + 动态日志。

### 已识别的产品/交互缺口（7 类，企业级用户可感知）

| # | 缺口类别 | 当前状态 | 对「最优操作」影响 |
|---|----------|---------|---------------------|
| G1 | **全局导航效率** | 顶栏只有 4 个操作（折叠/刷新健康/API文档/用户），无「快速新建」、无「全局搜索」、无「命令面板」 | 建任务/上传算子/开 AI 对话每次都要点两次 → 摩擦 = 3 步 |
| G2 | **全局快捷键** | 只有页面级 Enter（Chat/Automation/Task 聊天）。无 Cmd/Ctrl+K 命令面板、无 Ctrl+N 新建、无 `?` 帮助、无 Ctrl+F 搜索上下文 | 重度用户手离鼠标 = 效率 -50%，企业级高频使用者会强烈吐槽 |
| G3 | **空状态引导 2/8 缺失** | 共 32 视图里 TaskView（任务 0 条）、MarketView（商城 0 包）、GraphView（图谱未选节点）等 **2 视图** 没有 `el-empty + 去创建` 的 CTA 按钮 | 用户不知道"下一步做什么"，误以为功能坏了（B 端典型流失点）|
| G4 | **表单最优步骤流缺失** | TaskView/MarketView 上传都是「6~7 字段堆 Dialog」一步填完；无：默认值预填（截止=今天+3 天、优先级 medium、分类 history）、步骤条 / 分步聚焦、实时进度反馈（字段完成度 n/7） | 新用户上手 7 分钟才能熟练 → 最佳体验目标 = 1 分钟内完成 80% 场景 |
| G5 | **数据列表高级交互缺失** | TaskView / MarketView 均缺：分页、筛选条件持久化（query→URL）、列筛选、排序、导出 CSV、批量操作、行内编辑、卡片↔列表视图切换、最近搜索历史 | 任务/算子一旦上 100+ 条，翻页 & 重复搜索极痛苦 |
| G6 | **信息架构冗余** | 侧边栏 24 项平铺无分组、tabbar 只存当前模块、缺少二级面包屑（例如：任务管理 / 新建任务）；菜单缺少「最近访问」固定区 | 新用户找 AI 自动化 2 轮遍历 / 老用户切换工作流需滑动 400px |
| G7 | **体验一致性（可访问性）** | 快捷键缺失、focus 样式由浏览器默认、无全局错误/请求 loading 遮罩、顶栏没有"当前页面帮助"入口 | 无障碍合规 A 级未覆盖，键盘操作断点多 |

### 关键产品事实（正面 = 可复用能力）
✅ Dashboard 门户已有 4 快捷入口 + 24 模块完整矩阵卡片（可直接点击跳转），结构正确  
✅ NAV_MODULES 已定义 `path/label/icon/color/bg` 5 维元数据，可直接喂命令面板  
✅ App.vue 已有 `tabs/crumbs/collapsed/health` 4 种框架状态，扩展成本低  
✅ TaskView/MarketView 表单校验层 & DTO 映射层已存在，改「默认值/分步/进度」只在模板层加  

---

## Files and Modules（改哪些，最小化）

### 必改（3 核心文件 + 2 视图空状态 CTA + 2 列表增强）
1. `frontend-ui/src/App.vue` — 新增顶栏 3 类快捷能力：
   - 顶栏左侧全局搜索（Command Palette 触发，Ctrl/Cmd+K 或点击输入框）
   - 顶栏右侧「⚡ 新建」下拉快捷菜单：新建任务 / 上传算子 / 新建会话 / 新建工作流
   - 全局 keydown 监听（Ctrl+K / Ctrl+Shift+P / Ctrl+N / Ctrl+1..9 跳转 / ? 帮助弹层）
   - 面包屑二级条目（例如 `/tasks/new` → 任务管理 / 新建任务）
   - 顶栏加帮助按钮（`?` → 当前页上下文帮助 + 快捷键说明）
2. `frontend-ui/src/types.js` — 新增：
   - `QUICK_CREATE_COMMANDS`（新建菜单映射）
   - `HOTKEY_GROUPS`（展示给用户）
   - `NAV_GROUPS`（24 菜单项按「工作台/图谱 AI/业务/治理」4 组归类）
3. `frontend-ui/src/globalShortcuts.js`（新增轻量 composable，由 App.vue onMounted 绑定）
4. `frontend-ui/src/views/TaskView.vue` — 产品优化 5 项：
   - 空任务时 `<el-empty> + 去创建按钮` CTA
   - Dialog 新增字段完成度进度条（`1/6 已填`）
   - 新建默认值：`due_date = 今天 + 3 天 18:00` / `estimate_hours = 2`
   - 最近搜索历史（`localStorage task_search_hist`）
   - 分页：每页 20，状态条（`1-20/138 条`）
5. `frontend-ui/src/views/MarketView.vue` — 产品优化 5 项：
   - 空商城 CTA：`立即上传第一个算子包`
   - 上传表单步骤条（3 步：基础信息 → 详细描述 → 元信息）
   - 卡片 ↔ 列表双视图切换
   - 排序下拉（`最新 / 最热 / 评分高`）
   - 最近搜索历史
6. `frontend-ui/src/views/GraphView.vue` — 空图谱/未选中节点 CTA：`从门户推荐节点`

### 可选改（契约对齐，不阻塞 UI）
7. `platform/backend-node/src/routes/tasks.js` — 搜索参数：`GET /tasks?q=&status=&priority=&category=&page=&size=` 支持（前端已有筛选 UI，后端目前全量返回）
8. `platform/backend-node/src/routes/browser-market.js` — 同参数搜索 + 排序 `sort=(newest|hot|rating)`

---

## Implementation Steps（依赖顺序执行）

### Phase A — 导航效率 & 全局快捷键（产品体感提升 60%）
1. App.vue 顶栏右侧加「⚡ 新建」下拉（4 项常用创建），点击直接跳目标页并设置预填参数
2. App.vue 顶栏左侧加全局搜索输入框（placeholder `Ctrl/⌘ + K 搜索 36 模块 / 200 端点 / 任务 / 算子`）
3. 新建 `globalShortcuts.js` composable：
   - `Ctrl+K` 聚焦顶栏搜索框
   - `Ctrl+N` 直接打开"新建任务" dialog（不跳转页面）
   - `Shift+?` 打开快捷键帮助 Drawer
   - `Alt+1..9` 按 NAV_GROUPS 第一级 9 项跳转
4. 在 types.js 中定义 NAV_GROUPS、QUICK_CREATE_COMMANDS、HOTKEY_GROUPS 常量
5. App.vue 面包屑新增二级：若存在 `meta.subLabel`（如 `/tasks/new`）或当前路由下 dialog 打开，追加"新建任务 / 上传算子"二级条目

### Phase B — 表单最优步骤流 + 空状态 CTA（完成 G3 G4）
6. TaskView：空列表改造为 el-empty + 【立即新建任务】按钮（点击调用 openCreate()）
7. TaskView Dialog：
   - 加入字段完成度进度条（6 字段）：`已完成 n/6`
   - due_date 默认填充为 `今天+3 天 18:00:00`；estimate_hours 默认 2 小时；其他保留原默认
   - tag 输入去重提示
8. MarketView：空列表 → `el-empty + 【立即上传第一个算子包】`
9. MarketView 上传 Dialog：改造为 `<el-steps>` 3 步：① 基础信息（name/category/tags）② 详细（requirement/summary）③ 元信息（version/downloads/rating）；每步 valid 才可 next
10. GraphView：未选节点时提示 CTA 「试试门户里的热门节点」+ 3 个默认热门节点快捷按钮

### Phase C — 列表高级交互 + 排序搜索持久化（G5）
11. TaskView：
   - 引入 `el-pagination`（前端分页，每页 20，因后端当前无分页）
   - 搜索词写 URL query：`/tasks?q=...&status=...`，刷新后保留
   - 最近搜索历史 5 条 localStorage `task_search_hist`
12. MarketView：
   - 排序下拉：`最新 / 最热 / 高评分`
   - 卡片 ↔ 列表视图切换按钮（icon + state）
   - 最近搜索历史 5 条 `market_search_hist`

### Phase D — 契约/查询参数对齐后端（可选，体验更好）
13. tasks.js 后端 GET `/tasks`：支持 `q/status/priority/category/page/size` 过滤分页
14. browser-market.js 后端 GET `/market`：支持 `q/category/sort/page/size`

### Phase E — 验证
15. 前端 `npm run build` 重新构建 0 错误
16. 后端 smoke-contract.cjs 重跑 23/23 PASS
17. 手动产品走查：快捷键 5 条 → 空状态 3 处 → 表单步骤 2 处 → 列表分页/排序/搜索持久化

---

## Dependencies and Considerations
- **Element Plus 样式副作用控制**：新增全局 `el-steps`、`el-progress` 等覆盖时，**严格使用 `页面根 class + :deep(...)` 精准选择器**，避免全局污染（经验 #216377）
- **快捷键冲突**：`Ctrl+K/F/N` 等浏览器默认快捷键（Ctrl+K=Firefox 搜索、Ctrl+F 原生查找）。策略：`Ctrl+Shift+P` 也可用作 Cmd Palette 的备用绑定；帮助 Drawer 内写清「Ctrl+Shift+P」替代方案
- **localStorage 前缀**：所有搜索历史加前缀 `mox_search_`，避免与其他工程冲突
- **无后端分页时 fallback**：Phase C 第 11 步先用前端分页，后端上线后切后端无缝切换（前端保持 `page/size` 状态变量）

---

## Validation（每步验证标准）
| 步骤 | 验证命令/操作 | 预期 |
|------|-------------|------|
| A1-A4 快捷键 | 浏览器控制台 `localStorage.clear()` 后 Ctrl+K → 聚焦搜索框 | ✅ 光标在全局搜索 |
| A1 新建下拉 | 点 ⚡ 新建 → 新建任务 → Dialog 打开 | ✅ Dialog 已展开并默认填好截止+工时 |
| B6 TaskView 空态 | 删除所有任务 → 列表空 | ✅ 有 CTA 按钮「立即新建任务」 |
| B7 完成度 | 打开 Dialog 只填标题 | ✅ 进度=1/6（16.7%）|
| B9 MarketView 3 步 | 上传 → 只填名称下一步 → 被"需求描述必填"卡住 | ✅ 校验阻断，不能跳到下一步 |
| C11 TaskView 分页 | 数据 13 条 → 第 1 页 | ✅ 1-20/13 |
| C12 Market 排序 | 排序=最热 | ✅ downloads 高的排前 |
| E15 构建 | `cd frontend-ui && npm run build` | exit 0，无 error |
| E16 契约冒烟 | `cd backend-node && node scripts/smoke-contract.cjs` | 23/23 PASS |
| E17 产品走查 | 6 类用户路径（见 Phase E 列表）| 100% 无阻塞 |

---

## Risks
- **R1 快捷键系统级冲突**：Windows Ctrl+N = 新窗口，Ctrl+F = 原生搜索。**处理**：改为「Ctrl+Shift+N 新建任务」，Ctrl+Shift+F 打开全局搜索 Drawer（替代）
- **R2 多步骤 Dialog 重构可能打破原有 validate 提交链**：处理：保持原 `doUpload` / `createTask` 不变，只在 UI 层包 `<el-steps>` + 内部 `next/prev`，最后一步提交仍调原函数
- **R3 前端分页在大数据（1000+ 任务）时卡顿**：处理：后端分页契约 Phase D 同时做，并在前端预留「列表 1000 条阈值自动切换后端分页」开关
- **R4 全局搜索输入框尺寸窄，影响可用性**：处理：搜索框点击后展开（expandable pattern），宽度由 220px 展开到 460px
