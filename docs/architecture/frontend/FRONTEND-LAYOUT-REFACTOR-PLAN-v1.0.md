# 前端外壳与布局架构重构方案 V1.0

> 编号：**FE-LAY-REF-V1.0**　·　层定位：L2 架构层 · `frontend/` 主题目录
> 状态：**部分实施**（P1 的 1.1/1.2/1.3/1.4 已随 `src/modules/` 落地，见 §9 落地进度；P0 布局分层与样式契约未动）　·　采集时间：2026-09-23
> 对象：`frontend-ui/`（Vite + Vue3 + Element Plus，dev :3020，hash 路由）
> 关联：[架构层入口](../README.md) · [文档结构规范](../../ARCHITECTURE-OF-DOCS.md)

---

## 0. 一句话结论

前端的问题不是"配色不好看"，而是**三个机制缺失**：没有布局分层（外壳被根组件无条件挂载）、没有导航单源（同一套"当前模块"判断写了 5 遍）、没有样式令牌契约（断点写成死规则、4329 行全局 CSS 挂在单个 view 上、硬编码色与 token 各占一半）。

因此本方案不安排"美化"任务，先补机制；界面观感问题（侧栏是否保留、图标体系）列为待决问题，需产品拍板后再动。

---

## 1. 现场证据

所有条目均在 `main` 分支当前代码上实测，`file:line` 可直接跳转。

| 编号 | 现象 | 证据 | 影响 |
|------|------|------|------|
| FE-01 | 登录后外壳在**所有**路由渲染 | `App.vue:3-33` 把 `IconSidebar`+`TheSidebar`+`TheTopbar` 写死在根组件；`router/index.js:17-28` 是纯平铺路由表，无 layout 父节点 | `/login` `/register` `/forgot-password` `/403` `/portal` `/share` 均可见两栏导航（含指向 `/dashboard`、`/admin` 的链接）与顶栏"新建/主题/账户"菜单——未登录用户可看到并点击管理入口 |
| FE-02 | 模块侧栏（240px）**点击不导航** | `TheSidebar.vue:27-37`：条目是 `<div @click="activeItemKey = item.key">`，无 `router-link`；`nav.config.js:212-274` 的 `MODULE_SIDEBAR_CONFIG` 条目根本没有 `path` 字段；`TheSidebar.vue:77` 未匹配模块时回落到 `dashboard` 配置 | 常驻 240px 宽的第二侧栏是纯装饰：点"总览/最近项目/收藏"只切换本地高亮。登录页因此显示"工作台·项目概览与快捷入口" |
| FE-03 | 新手引导与登录态解耦 | `OnboardingGuide.vue:126-131`：`onMounted` 里若无 `localStorage.onboarding_done` 则 800ms 后自行 `visible=true`；`App.vue:70` 在每个路由都挂载该 `el-dialog` | 未登录用户在**登录页**看到"欢迎来到项目工作台"引导；`el-overlay` z-index 2009 盖住登录表单并抢占初始焦点（实测快照中该 dialog 为 `focused`） |
| FE-04 | 响应式断点是死规则 | `App.vue:379-389` 的 `@media` 写在 `<style scoped>` 内，编译产物为 `[data-v-7a7a37b1]:root { --content-pad:16px }`（实测 `document.styleSheets` 中仅此一形）；`:root` 即 `<html>`，永不携带组件 scope 属性 | 窄屏下 `--content-pad` 恒为 20px；`styles/global.css` 仅有 `.grid-*` 断点（:336-349），64px/240px 外壳在窄屏完全不塌缩。这是"布局差"最直观的来源 |
| FE-05 | "当前模块"判断写了 5 遍 | `App.vue:115-128`（硬编码 `path.startsWith` 链）· `IconSidebar.vue:66-67` · `TheSidebar.vue:60-77` · `TheTopbar.vue:181,204` · `TabBar.vue:51` | 加一个页面需同步改 3~5 处；已开始漂移：`/expert-workspace` 等前缀只存在于 `App.vue:120`，其余 4 处各自近似实现，高亮状态可互相矛盾 |
| FE-06 | 路由白名单第二份真源 | `router/index.js:43` `WHITE_LIST = ['/login','/portal','/hall','/share','/s/','/403']` 与 `router/modules/public.js` 的 3 条公开路由重复 | 新增公开页须同时改两处，否则守卫与外壳判断不一致 |
| FE-07 | 两套图标体系并存 | `nav.config.js` 内 36 个 `icon:` 值混用：`NAV_MODULES/NAV_GROUPS/QUICK_CREATE_COMMANDS` 用 Element Plus 图标**名**（`'Odometer'`、`'Cpu'`），`ICON_NAV_GROUPS/MODULE_SIDEBAR_CONFIG` 用 **emoji**；`IconSidebar.vue:29`、`TheSidebar.vue:34` 以文本节点渲染 emoji | 图标风格不可主题化（emoji 不受 CSS color 控制）、Windows/ macOS 渲染不一致、深色主题下亮度刺眼 |
| FE-08 | 侧栏角标是写死假数据 | `nav.config.js:195` `badge: 12`；`projects` 段 `count: 24/12/2/8`；`tasks` 段 `count: 28/5/12/16/4/10`；`TheSidebar.vue:36` 直接渲染 | 登录页侧栏即显示"📁 12"等虚假计数，与后端无关，损害可信度 |
| FE-09 | 外壳存在成建死代码 | `ui.store.js:22,107-118` `aiFullscreen` + `toggle/enter/exitAIFullscreen` **0 个调用方**（仅 `App.vue:112` 读）；`globalLoading/showLoading/hideLoading` 外部 0 调用；`setPhase` 0 调用；`TabBar.vue`（2.3KB）无任何挂载点，仅出现在 `MODULE-MANIFEST.md:70` 与 stories | "AI 全屏"这类隐藏外壳的开关从未生效，却占据 `App.vue` 的 class 与两条 prop 透传链路，掩盖了真正缺失的 layout 机制 |
| FE-10 | 健康轮询挂在根组件 | `App.vue:195` `setInterval(refreshHealth, 30000)`；`refreshHealth` 经模板 ref 命令式调用两个子组件（`App.vue:159-162`） | 登录页也在打后端；实测登录页出现 2 条重复 `服务端异常：…（500）` toast，无按接口去重/节流 |
| FE-11 | 样式规模与令牌契约失控 | `styles/workspace.css` **4329 行非 scoped**，却被单个 view 引入（`ExpertWorkspaceView.vue:285`）；全库 `!important` 265 处；`views/` 内硬编码 hex 1229 处 vs `var(--token)` 1255 处；三主题各 ~100 token（`theme-dark 96 / sky 97 / cyberpunk 102`），基线 `global.css:2` 137 个 | 全局样式生效顺序取决于用户先访问哪个 chunk（同一元素在不同进入路径下可能不同样式）；主题切换必然有一半界面不跟随；`!important` 用于压制顺序不确定性 |
| FE-12 | 巨型组件 + 页头重复 + 过渡未接线 | 行数：`ExpertConfigView.vue` 5414、`KnowledgeBasePanel.vue` 2728、`MessageBubble.vue` 2404、`ProjectsView.vue` 2246、`GraphView.vue` 2241；26 个 view 自带 `page-header/page-title`；`App.vue:278-296` 写了 `.page-fade-*` 但 `App.vue:29-31` 的 `<router-view>` 既无 `<transition>` 也无 `<keep-alive>` | 单文件不可评审；每次导航整树重挂载（192KB 级 view 重新初始化）；页面切换无过渡，写好的 CSS 是死代码 |

> 说明：dev server 冷启实测 90 请求 / 5.2MB（element-plus 未压缩 2.4MB）属开发模式计数，**不作为性能结论依据**。但 `main.js:14-18` 全量注册 `@element-plus/icons-vue` 约 1200 个组件确会进入生产主包，纳入 P1 处理。

---

## 2. 根因归纳

| 缺失机制 | 派生出的证据 |
|---|---|
| **M1 布局分层**：路由不声明布局，外壳由根组件硬挂 | FE-01 FE-02 FE-03 FE-09 FE-10 |
| **M2 导航单源**：routes 与 nav.config 各自为真源，模块归属无统一定义 | FE-05 FE-06 FE-07 FE-08 |
| **M3 样式令牌契约**：令牌/断点/CSS 作用域无归属登记 | FE-04 FE-11 FE-12 |

FE-12 的巨型组件属 M3 的连带后果：没有共享页头与栅格契约，每个 view 只能自绘一遍。

---

## 3. 目标结构

```
frontend-ui/src/
├─ App.vue                    # 只做：el-config-provider + <component :is="layout"> + 全局 teleport 层
├─ layouts/
│  ├─ DefaultLayout.vue       # 现 App.vue 的外壳（IconSidebar + TheSidebar + TheTopbar + 帮助抽屉 + 引导 + 轮询）
│  └─ BlankLayout.vue         # 仅 <router-view/>（登录/注册/找回密码/403/分享/门户）
├─ composables/
│  └─ useActiveModule.js      # 唯一的"当前模块/当前侧栏项"解析，5 份副本收敛到此
├─ constants/
│  └─ nav.config.js           # 只留派生器：由 routes.meta 生成 ICON_NAV_GROUPS / MODULE_SIDEBAR_CONFIG
└─ styles/
   ├─ tokens.css              # :root 令牌 + 全部响应式断点（唯一 :root 真源）
   ├─ global.css              # 通用原子类与 EP 覆盖
   └─ workspace/*.css         # 4329 行大文件按域拆分并登记归属
```

**路由 meta 契约**（新增字段即布局与导航的唯一声明处）：

```js
meta: {
  layout: 'default' | 'blank',        // 缺省 'default'
  public: true,                       // 取代 router/index.js:43 的 WHITE_LIST 字面量
  module: 'projects',                 // 取代 5 处 path.startsWith 猜测
  title: '项目中心',
  requiresPermission: ['project:read']
}
```

选此"App.vue 动态选外壳"方案而非"嵌套路由 layout 父节点"，是为了 **P0 不动 11 个 router 模块文件**；后者作为 P2 可选演进（见 Q5）。

---

## 4. 分阶段实施

### P0 · 外壳分层与断点复活（不改任何 view 文件）

| # | 任务 | 涉及文件 |
|---|------|----------|
| 0.1 | 抽出 `layouts/DefaultLayout.vue`（原样承接现 `App.vue` 模板/脚本/样式）与 `BlankLayout.vue` | `App.vue`、新增 2 文件 |
| 0.2 | `App.vue` 缩为 `<component :is="layoutOf(route)" :key="layoutOf(route)">` | `App.vue` |
| 0.3 | 公开路由声明 `meta.layout='blank'` + `meta.public=true` | `router/modules/public.js`、`fallback.js`（403） |
| 0.4 | `WHITE_LIST` 改为遍历 `router.getRoutes()` 取 `meta.public` 派生 | `router/index.js:43-47` |
| 0.5 | 引导弹窗、帮助抽屉、30s 健康轮询移入 `DefaultLayout`（随登录态挂载/卸载） | `DefaultLayout.vue`、`App.vue:195` |
| 0.6 | `:root` 与 `@media` 迁到 `styles/tokens.css`（全局），删除 scoped 内的死规则；断点与 `global.css:336-349` 统一为一套 | `App.vue:203-217,379-389`、`styles/global.css` |
| 0.7 | 删除死代码：`aiFullscreen` 三 action 与 `.ai-fullscreen` 分支、`globalLoading` 组、`setPhase`、`TabBar.vue` | `App.vue:3,11,21,112,264-275`、`ui.store.js`、`TabBar.vue` |
| 0.8 | 错误 toast 按 (message+code) 3s 去重 | 请求层 + `main.js:44-56` |

**验收（浏览器断言，可脚本化）**
1. `/login`：`.icon-sidebar`、`.module-sidebar`、`.topbar` **不存在**；无 `el-overlay` 引导；无 `/api/*` 健康请求。
2. 视口 375×812：`getComputedStyle(document.documentElement)['--content-pad'] === '12px'`。
3. `document.styleSheets` 中不存在匹配 `\]:root` 或 `:root\[data-v-` 的规则。
4. 未登录直达 `/admin` → 跳登录且带 `redirect`（回归现行为）。

**回滚点**：0.1–0.5 为单 commit，`revert` 即恢复现外壳；0.6/0.7 各自独立 commit。

### P1 · 导航单源与侧栏真实化

| # | 任务 | 对应证据 |
|---|------|----------|
| 1.1 | `useActiveModule()` 单一实现，基于 `route.matched` 的 `meta.module`；`App/IconSidebar/TheSidebar/TheTopbar` 全部改用它 | FE-05 |
| 1.2 | `ICON_NAV_GROUPS`/`MODULE_SIDEBAR_CONFIG` 改为**由 routes 派生**（route 作者只需写 meta），`nav.config.js` 不再维护路径与标题副本 | FE-05 FE-06 |
| 1.3 | 侧栏条目补真实跳转（`item.path` + `router-link`）；无跳转目标的项目**从配置删除**，不留假导航 | FE-02 |
| 1.4 | `badge`/`count` 接 store 真实数据，无数据源则不渲染角标 | FE-08 |
| 1.5 | 图标统一为 EP 图标名 + `<el-icon><component :is="…"/></el-icon>`；`main.js` 全量注册改为按需（`unplugin-icons`/显式导入） | FE-07 |
| 1.6 | 侧栏搜索框改为可聚焦的 `<button>`（`role`/`tabindex`/Enter） | FE-02 |
| 1.7 | 健康轮询由 `DefaultLayout` 拥有，失败 3 次后降频至 5min | FE-10 |

**验收**：`grep -c "path.startsWith" src/components/layout src/App.vue` == 0；`nav.config.js` 中不含数字字面量 `badge`/`count`；新增一个页面只改 1 个 router 模块文件即可同时出现在侧栏与高亮中。

### P2 · 样式契约与组件边界（按域逐个 commit）

| # | 任务 | 目标指标 |
|---|------|----------|
| 2.1 | `workspace.css`(4329) 按功能域拆分并登记 owner；`<style>` 不再从 view 里 import 全局 CSS | 单文件 ≤ 800 行 |
| 2.2 | 抽 `<PageHeader>` 组件，替换 26 处自绘页头 | 自绘页头 == 0 |
| 2.3 | `<router-view>` 接 `<transition>`+`<keep-alive :include>`（白名单见 Q5），激活已写的 `.page-fade-*` | 二次进入导航无重建 |
| 2.4 | `views/` 硬编码 hex 迁到 token，`!important` 只允许出现在 EP 覆盖层 | hex 计数单调下降（门禁）|
| 2.5 | 巨型组件拆分试点：先 `KnowledgeBasePanel.vue`(2728) 或 `ExpertConfigView.vue`(5414) 之一 | 单文件 ≤ 800 行 |

### 门禁建议（新增 `scripts/gate/check-fe-layout.py`）

沿用本仓 gate 文化，把上面的验收变成可回归检查：

1. 每个 `meta.public` 路由必须 `meta.layout === 'blank'`。
2. `src/**` 内禁止 `path.startsWith` 式模块推断（白名单：`useActiveModule.js`）。
3. scoped `<style>` 内禁止出现 `:root`（本次死断点的根因，必须永久禁止）。
4. `nav.config.js` 禁止数字字面量 `badge`/`count`。
5. 记录 `!important` 与 views 内硬编码 hex 基线数，**只降不升**。

---

## 5. 不做清单（明确排除，避免范围膨胀）

- 不迁移构建工具、不改 hash 路由（`createWebHashHistory` 保持）。
- 不引入新 UI 库、不做视觉重设计（配色/圆角/阴影调整等 P1 完成后单列）。
- 不在本方案内重构 `stores/*`（`permission.store` 452 行、`ai.store` 637 行另案）。
- 不动 `projects/` 子项目与 `design/` 原型稿。
- 5 个巨型 view 不做一次性重写，只按 2.5 逐文件推进。

---

## 6. 风险

| 风险 | 缓解 |
|------|------|
| P0 后外壳随布局切换重挂载，主题/侧栏折叠态可能闪一下 | 折叠与主题写在 store 且持久化（现 `app.store` 已具备）；`BlankLayout` 无外壳，登录后首次挂载即正确 |
| `meta.layout` 漏声明导致某公开页又出现外壳 | 门禁第 1 条 + `getRoutes()` 全量断言脚本 |
| 1.3 删除假导航会暴露"模块内无子页可跳"的事实 | 属预期收益；保留判断交产品（见 Q1） |
| 2.3 keep-alive 与 `scrollBehavior`（`router/index.js:33-36`、:133 `window.scrollTo`）冲突 | keep-alive 页面缓存各自滚动位置，需在白名单内逐个验证 |
| 2.4 大范围改色可能改错 | 按目录分批 + 三主题截图对比后再合入 |

---

## 7. 待决问题（需拍板后进入实施）

| 编号 | 问题 | 选项 |
|------|------|------|
| Q1 | **240px 模块侧栏保留还是取消？**（现状：不做任何事） | A 保留，改为真实锚点导航（滚动到当前页区块）· B 取消，三栏改两栏（64px 图标栏 + 内容）· C 保留宽度改作"项目/上下文切换器" |
| Q2 | 图标体系 | A 全量 EP 图标 + 按需注册 · B `unplugin-icons` + 自定义 SVG 集（需设计产出） |
| Q3 | `workspace.css` 归属 | A 拆入 workspace 域并 scoped · B 提升到 `styles/` 全局层并登记 owner |
| Q4 | 侧栏自动折叠是否改用 `@vueuse/core` 的 `useMediaQuery`（devDep 已存在）替代 `App.vue:137-156` 的 `autoCollapsed` 手工状态 | 是 / 否 |
| Q5 | 布局机制终态 | A 停在"App.vue 动态选外壳" · B 进一步改为嵌套路由 layout 父节点（需重写 11 个 router 模块，keep-alive 位置更自然） |
| Q6 | 实施顺序偏好 | A 按 P0→P1→P2 全做 · B 只做 P0（解决登录页泄漏与死断点）后重新评估 |

---

## 8. 附录 · 证据复现命令

```bash
# 1. 外壳与布局（无 layout 分层）
sed -n '1,35p;100,135p' frontend-ui/src/App.vue
grep -n "WHITE_LIST" frontend-ui/src/router/index.js

# 2. 死断点：确认 scoped 内 :root 编译形如 [data-v-*]:root
grep -n "@media\|:root" frontend-ui/src/App.vue

# 3. 侧栏不导航 + 假数据
sed -n '22,40p;60,80p' frontend-ui/src/components/layout/TheSidebar.vue
grep -n "badge:\|count:" frontend-ui/src/constants/nav.config.js

# 4. 引导与登录态解耦 / 死代码 0 调用方
sed -n '118,132p' frontend-ui/src/components/OnboardingGuide.vue
grep -rn "toggleAIFullscreen\|hideLoading\|setPhase" frontend-ui/src --include=*.vue --include=*.js

# 5. 样式规模
wc -l frontend-ui/src/styles/workspace.css frontend-ui/src/styles/global.css
grep -rn "workspace.css" frontend-ui/src
grep -rho "!important" frontend-ui/src | wc -l
grep -rho "#[0-9a-fA-F]\{6\}" frontend-ui/src/views | wc -l

# 6. 浏览器端实测（dev :3020，打开 /#/login 后执行）
# document.querySelectorAll('.icon-sidebar, .module-sidebar, .topbar').length   → 期望 0，现 >0
# getComputedStyle(document.documentElement)['--content-pad']                   → 375px 视口下期望 12px，现 20px
# [...document.styleSheets].some(s=>/\[data-v-.*\]:root/.test(s.cssRules[0]?.parentStyleSheet?.cssText||''))
```

## 9. 落地进度（2026-09-23，随专家联盟前端模块）

| 计划项 | 状态 | 事实 |
|--------|------|------|
| 1.1 `useActiveModule()` 单源 | 🟡 半 | 新增 `src/composables/useActiveModule.js`，`App.vue` 与 `TheSidebar.vue` 已改用（`meta.module` 优先，前缀表回退）；`IconSidebar/TheTopbar/TabBar` 三处副本仍在 |
| 1.2 导航由路由派生 | 🟡 半 | `src/modules/index.js` 登记的模块页由 `collectNav()` 自动挂载进 `MODULE_SIDEBAR_CONFIG`，新增模块页不再改 `nav.config.js`；旧 `views/` 页面仍在手抄 |
| 1.3 侧栏条目真实跳转 | 🟡 半 | `TheSidebar` 条目改为 `router.push(item.path)`，无 `path` 者不再显示可点游标；`expert` 模块 4 条已补真实路径，`dashboard/projects/tasks` 的 9 条无目标项仍在（按 Q1 待决） |
| 1.4 假计数清理 | 🟢 done | `nav.config.js` 删除 14 处编造 `count` 与 1 处 `badge`；`module.test.js`/`wiring.test.js` 双门禁禁止回归 |
| 1.5 图标体系 | 🟡 半 | 模块导航图标强制 EP 名（测试正则约束），`TheSidebar` 同时支持 EP 组件与 emoji 渲染；存量 emoji 未替换，属 Q2 待决 |
| 路由/导航双真源（FE-05 部分） | 🟢 done | 模块路由先于通配兜底由测试守卫；`/alliance/console` 从登记到可跳转零手抄 |
| 0.1~0.8 P0 外壳分层 | ⚪ 未动 | 登录页外壳泄漏（FE-01）、引导弹窗抢占焦点（FE-03）、死断点（FE-04）、健康轮询（FE-10）仍在，等 Q1/Q5 拍板 |
| 2.x 样式契约 | ⚪ 未动 | `workspace.css` 4329 行、`!important` 265 处、巨型组件未拆 |

## 10. 变更记录

| 版本 | 日期 | 说明 |
|------|------|------|
| V1.0 | 2026-09-23 | 首次立项：12 条证据（FE-01~FE-12）、3 项根因、P0/P1/P2 计划、门禁建议、6 个待决问题。未改任何前端代码。 |
| V1.0（实施记录） | 2026-09-23 | 随 `src/modules/expert-alliance/` 落地 P1 的 1.4 全量与 1.1/1.2/1.3/1.5 的模块侧机制（详见 §9）。新增文件：`src/modules/{index.js,_kernel/envelope.js,_kernel/module-registry.js,wiring.test.js}`、`src/composables/useActiveModule.js`、`src/modules/expert-alliance/**`；改动：`App.vue`、`components/layout/TheSidebar.vue`、`constants/nav.config.js`、`router/index.js`。P0 与 P2 未动，Q1/Q2/Q5 仍待拍板。 |
