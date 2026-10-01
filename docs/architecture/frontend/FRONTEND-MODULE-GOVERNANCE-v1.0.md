# 前端模块化治理（FRONTEND-MODULE-GOVERNANCE）

> 层定位：L2 架构层 · `architecture/frontend/` 主题目录
> 编号：FE-MOD-GOV-V1.0　·　状态：🟢 生效（采集 2026-09-26）
> 适用范围：`frontend-ui/src/` 全部模块

本文件是前端模块化工程的**事实单源**：模块目录、统一出口、导入/命名/边界规则与验证基线。后端 6 层架构见 [NORMALIZED_ARCHITECTURE.md](../NORMALIZED_ARCHITECTURE.md)。

## 1. 模块目录（`src/` 一级目录）

| 模块 | 文件数 | 角色 | 统一出口 |
|------|------:|------|------|
| `views/` | 70 | 页面层（被 router 引用） | 直接路径 |
| `modules/` | 47 | 内核注册的子模块（expert-alliance / admin-lowcode 等） | `modules/index.js`（内核清单） |
| `components/` | 32 | 通用组件 | `components/index.js`（32 组件） |
| `api/` | 22（含 barrel） | 接口层 | `api/index.js`（21 文件全覆盖） |
| `composables/` | 13 | 组合式函数 | `composables/index.js`（27 导出） |
| `router/` | 11 | 路由（11 个 `router/modules/*.js` + 索引） | 直接路径 |
| `stores/` | 9 | Pinia store | `stores/index.js`（8 store 全覆盖） |
| `utils/` | 7 | 工具函数 | `utils/index.js`（42 导出） |
| `constants/` | 6 | 常量/导航单源 | `constants/index.js` |
| `directives/` | 2 | 指令 | `directives/index.js`（6 导出） |

依赖方向（强约束）：`views / router → 业务层（components/composables/stores/modules）→ 基础层（api/utils/constants）`，**无反向依赖**（边界审计 0 违规，见 §5）。

## 2. 导入规则（归一化）

1. **跨目录导入一律使用 `@/` 别名**（`@/api/graph.api`、`@/composables/useSSE` 等）；相对导入仅允许**同目录**（`./`）。
   - 历史状态：97 处跨目录相对导入已归一化为 `@/`（36 文件）；当前相对导入全部为同目录惯例。
   - **已 barrel 化目录（components/api/stores/constants/utils/composables/directives）禁止 `@/` 深路径**（`@/components/common/DataTable.vue` 一律走 `@/components`，门禁 E6）。
   - **三类深路径有意豁免**（门禁不检查）：① router→views 懒加载路径（`views/` 不建 barrel，避免全量打包破坏路由懒加载）；② 模块内部跨子目录（`modules/*` 内部自治——expert-alliance 已内部 barrel 化，见 §4；admin-lowcode 等小模块暂保持深路径自治）；③ 顶层 alias 入口文件（`@/types`、`@/echarts`、`@/globalShortcuts` 为 `src/` 顶层文件，非目录）。
2. **禁止 `from 'element-plus'` 根导入**（根 barrel 是 294 个组件的 re-export 链，任何一处根导入都会使 **tree-shake 失效、全量组件进包**）。
   - 必须按组件子路径导入：JS `element-plus/es/components/<kebab>/index`、样式 `element-plus/es/components/<kebab>/style/css`。
   - 附属组件无自有目录，从父组件目录导出（PARENT 表）：`TableColumn→table`、`FormItem→form`、`Option/OptionGroup→select`、`TabPane→tabs`、`BreadcrumbItem→breadcrumb`、`DropdownMenu/DropdownItem→dropdown`、`ButtonGroup→button`、`CollapseItem→collapse`、`RadioButton/RadioGroup→radio`、`TimelineItem→timeline`、`DescriptionsItem→descriptions`、`Step→steps`、`CheckboxGroup→checkbox`。
   - 模板自动导入由 `vite.config.js` 的 `epSubpathResolver()` 处理（禁止改回官方 `ElementPlusResolver`）。
   - ⚠️ 历史教训：O3 轮修完 44 处根导入后，新代码（admin-lowcode、KnowledgeBasePanel）又引入 6 处根导入，导致 vendor-element 从按需 647.8KB 反弹到全量 951.1KB。**新代码合并前必须 grep 根导入**。
3. 指令/消息类导入示例：
   - `import { ElMessage } from 'element-plus/es/components/message/index'`
   - `import { ElLoading } from 'element-plus/es/components/loading/index'` + `app.directive('loading', ElLoading.directive)`

## 3. 命名规范

| 类型 | 规范 | 反例（已修复） |
|------|------|------|
| 接口文件 | `*.api.js` | `alliance.js` / `auth.js` / `allianceTaskModel.js` → 已改名 `*.api.js` |
| Pinia store | `*.store.js` | — |
| 组合式函数 | `use*.js` | — |
| 组件 | 模板内 kebab-case，文件名 PascalCase | — |
| barrel 冲突项 | 别名后缀（`safeUrl`/`formatTime` 冲突 → `kbSafeUrl`/`kbFormatTime`） | 避免 `export *` 造成 ambiguous binding |

## 4. 统一出口（barrel 清单）

| 出口 | 覆盖 | 说明 |
|------|------|------|
| `api/index.js` | 21 个 `*.api.js` 全覆盖 | `export * from './<name>.api'` + `http` 默认导出 |
| `stores/index.js` | 8 个 store 全覆盖 | app/project/ui/user/auth/permission/ai/alliance |
| `composables/index.js` | 12 个文件 27 导出 | 含 `workspace/` 子目录 5 个 |
| `utils/index.js` | 6 个文件 42 导出 | 冲突项 `kbSafeUrl`/`kbFormatTime` 别名 |
| `directives/index.js` | permission.js 6 导出 | — |
| `components/index.js` | 32 组件 | 子目录组件同导出；同名组件加前缀（`PhasePipeline` 顶层 vs `ai/PhasePipeline` → `AiPhasePipeline`） |
| `modules/expert-alliance/{contract,model,store,api,components}/index.js` | 模块内部出口 | contract 244 名 / model 80 名 / store 8 名 / api 2 名 / components 15 组件；零命名冲突；barrel 排除 `*.test.js` |
| `constants/index.js` / `modules/index.js` | 既有 | 模块内核注册清单 |

新代码推荐优先经 barrel 导入（`@/api`、`@/stores`…）；`api/index.js` 头部注释声明"按需深路径导入亦允许"——两种风格并存，**禁止引入第三种**（相对路径跨目录、根 barrel）。

## 5. 边界审计与验证

验证命令（`frontend-ui/` 下）：

```bash
pnpm build        # 构建（≈2 分钟）；产物在 dist/
pnpm test run     # vitest 回归（当前 22/22）
python ../scripts/gate/check-frontend-module.py   # 模块化门禁（仓库根运行，CI 已挂载）
```

门禁检查项（`scripts/gate/check-frontend-module.py`，ERROR 即阻断，已挂载 `.github/workflows/ci.yml` frontend-module job）：
- E1 element-plus 根导入 = 0（回归先例：admin-lowcode/KnowledgeBasePanel 曾引入 6 处致 vendor-element 951KB）
- E2 跨目录相对导入 = 0（`*.test.js`、`stories/` 演示代码豁免——不参与打包）
- E3 barrel 导出名冲突 = 0（已显式遮蔽/别名的名字豁免：`executeTask`、`kbSafeUrl`/`kbFormatTime`）
- E4 api 必须 `*.api.js`、stores 必须 `*.store.js`（**例外**：`api/http.js` 为 axios 实例与拦截器基础设施，有意保留）
- E5 vite.config.js 必须使用 `epSubpathResolver`（注释提及 ElementPlusResolver 不判定）
- E6 已 barrel 化目录的 `@/` 深路径导入 = 0（`@/components/xxx.vue`、`@/api/xxx.api` 等一律走 barrel 出口；豁免见 §2.1）
- W1 块注释配对（提示性，含字符串/CSS 误报需人工甄别，不阻断）

边界审计（agent workspace 脚本，可复跑）：
- `build_module_graph.js`：目录级依赖边权（本文件 §1 数据源）
- `audit_boundaries.js`：barrel 使用率（**633 via barrel / 77 deep**，deep 全部为 §2.1 豁免类：router 懒加载 ~55 + _kernel 内核 ~10 + 顶层 alias ~10 + 误报 1（admin-lowcode 已 barrel 化））、命名清单、反向依赖检测——**当前 0 违规**
- 新代码门禁（手工/CI 前）：`grep -r "from 'element-plus'" src/` 必须为空；`grep -r "from '../" src/` 仅允许同目录 `./`。

## 5.1 barrel 统一出口迁移（2026-09-26）

将跨模块深路径导入迁移到统一出口（`@/api/graph.api` → `@/api` 等），**84 处迁移**（三轮，含注释误报甄别）；barrel 使用率 **57 → 136**（深路径 109 → 35，剩余为 components/views 等无 barrel 目标或 default/命名空间导入）。

迁移前必须先做**导出名冲突检查**（`check_barrel_collisions.js`），并修复发现的 barrel 缺陷：
- `api`：`executeTask` 同时存在于 workspace.api 与 projects.api（`export *` 会 ambiguous）→ barrel 显式 `export { executeTask } from './workspace.api'` 遮蔽；
- `api`：补 `registerAuthTokenGetter`（此前只转发 registerProjectIdGetter）；
- `stores`：`useAiStore` 大小写错误（实际导出 `useAIStore`，小写版从未被引用、靠 tree-shake 侥幸通过）→ 修正；
- `stores`：补 `availableThemes`（app.store）、`ASSISTANTS`/`CONSULT_MODES`（ai.store 常量）；
- `constants`：补 `palette.js`（此前 4 个文件未进 barrel）。
- ⚠️ 教训：迁移脚本的**名字集合检查器曾漏 `export async function`**（async 函数导出未收集）与**多行 import 尾逗号产生空名**，导致误判 skip；修复正则后重跑。`useKnowledgeBase.js` 头部 JSDoc 曾因历史事故脚本遗留未闭合 `/**`，已修复（**块注释配对扫描** `scan_comment_balance.js` 可复检）。

## 5.2 components barrel 出口 + 门禁 E6（2026-09-26）

将 components 目录（32 组件，含 ai/common/expert/layout 子目录）聚合为 `components/index.js` 统一出口：
- **19 处深路径迁移**（`@/components/common/DataTable.vue` → `@/components` 等）：App.vue、TheTopbar、SchemaCrudPage、ChatView、Workbench、AdminRole/User/Department 等 12 文件；
- **api/constants 残余 6 处迁移**（`@/api/auth.api`/`@/api/http` → `@/api`、`@/constants/palette.js` → `@/constants`）；
- 同名组件处理：`PhasePipeline`（顶层）vs `ai/PhasePipeline` → 后者导出为 `AiPhasePipeline`；
- ⚠️ 教训：barrel 只有命名导出，**`import X from '@/components'`（default）会构建失败**——迁移后必须全部 `import { X } from ...`（本轮踩坑：TheTopbar 2 处 + api barrel default 4 处，修复后构建恢复）。
- 门禁新增 **E6**（barreled 目录 @/ 深路径 = 0）与**注释剥离**（E1/E2/E6 忽略注释中的 import 示例，防 JSDoc 误报）。
- 迁移后导入形态：**barrel 544 / deep 168（全部为 §2.1 豁免类）/ 同目录相对 166**。

## 5.3 expert-alliance 模块内部 barrel 化（2026-09-26）

将 expert-alliance 模块内部（contract/model/store/api/components）全部聚合为子目录 barrel，**89 处深路径迁移**：
- 新增 5 个 barrel：`contract/`（10 文件 244 导出）、`model/`（5 文件 80 导出）、`store/`（6 store）、`api/`（alliance.api 薄出口）、`components/`（15 组件）；
- 迁移对象：store 6 个（各 2-3 处）、views 6 个（各 3-9 处）、components 14 个（各 1-2 处）、model 3 个；
- ⚠️ 教训 1：**barrel 生成必须排除 `*.test.js`**（contract.test.js 引 node:fs，被 `export *` 拉进 vite build 即炸——首跑失败后重新生成）；
- ⚠️ 教训 2：components 深路径为 default 导入（SFC script setup），迁 barrel 必须同步命名化（`import ExpertCard from ...` → `import { ExpertCard } from ...`）；
- 迁移后模块内部深路径归零；全局导入形态：**barrel 633 / deep 79（全豁免）/ 同目录相对 203**。



## 5.4 admin-lowcode 模块内部 barrel + 门禁 E7（2026-09-27）

- **admin-lowcode 模块内部 barrel**：新增 `contract/index.js`（endpoints + pageSchema，零命名冲突）、`composables/index.js`（useCrudPage）；`SchemaCrudPage.vue` 2 处深路径迁入 barrel——模块内部深路径清零（全局 deep 79 → 77，剩余全为 router 懒加载 / _kernel 内核 / 顶层 alias / JSDoc 误报）；
- **门禁 E7**（新增）：@/ 别名导入目标存在性校验——所有 `from '@/xxx'` 必须能解析到真实文件（目录 index.js / .js / .vue），直接防路径断裂回归（barrel 大规模重构后最危险的故障模式）；与 E1-E6 同源挂载 CI 与 check-all.ps1。


## 5.5 联盟工作台取数归一化 + 被禁端点防复活（2026-09-27）

legacy 联盟工作台（`views/workspace/ExpertWorkspaceView.vue` + `composables/workspace/useAlliance.js`）此前**真的在调**模块契约判为被禁的编排器独占端点，且界面文案在四处手抄同一事实：

- **取数归一**：协作对话改走模块六模式契约 `allianceApi.collaborate(mode, input)`（网关 `/api/experts/*` 原生端点，入参与校验取 `contract/collab.js`），结果按 `resultKind` 只渲染后端确实产出的字段；原 SSE 恒 404 被 catch 兜成"协作调用失败"，七阶段假进度从未推进过，现阶段指示只在拿到结果后置为终态。能力清单改读 `GET /api/experts/capabilities`（`listExpertCapabilities`）——原实现读的是编排器独占端点里**后端从未产出的一个键**，界面因此永远显示写死文案。
- **文案单源**：删除 `useAlliance.js` 与 `panels/CollaborationPanel.vue` 两处手写七阶段表（副本里 `组队匹配/综合归纳` 与 Rust 单源 `组队路由/归一合成` 已漂移），一律取 `contract/phases.js` 的 `PHASE_IDS/phaseLabel`。
- **删除死码**：`src/api/alliance.api.js` 去掉 4 个零消费者导出（`runAllianceFullSSE`、别名 `runAllianceTask`、`getAllianceCapabilities`、恒零桩 `getAllianceStats`），34 → 29；`MODULE-MANIFEST.md` §4 该行同步更正（原写 `alliance.js`，§3 早已改名 `*.api.js`）。
- **防复活门禁**：新增 `modules/expert-alliance/contract/forbidden-revival.test.js`（8 例）——按 `FORBIDDEN_ENDPOINTS` 生成带/不带 `/api` 前缀的扫描针，对 `src` 全部非测试 `.js/.vue` 逐行扫描，与 `KNOWN_REVIVALS` 台账**双向相等**（新增即红；清零必须删条目，写零不算）。台账现存 4 处待收口位点：`api/ai.api.js`、`composables/useSSE.js`、`stores/ai.store.js`、`stores/alliance.store.js`。牙齿实测：注入两枚变异体（`http.js` 里复活 `/alliance/stats`、`useAlliance.js` 里恢复 `from '@/api'` 的 SSE 导入）各自打红 3 条断言，还原后 8/8 复绿。
- 验证：`vite build` RC=0；`check-frontend-module.py` ERROR=0（E1-E7 全 ✓）；全量 vitest **53 文件 / 730 例 / RC=0**。⚠️ 该 730 与 `FRONTEND-MODULE.md` §8 复述的用例数已不一致，而那份权威文档当前躺在 `docs/expert-alliance/_archive/v1/`（被历次文档搬迁 swept），4 处现行文档仍按 `docs/architecture/frontend/FRONTEND-MODULE.md` 引用它——**归档位与权威位待裁决**，本轮只报不改。



## 5.6 联盟协作收口第二轮：被禁端点清零 + 模式文案第三、四副本归一（2026-09-27）

§5.5 留下的四处被禁路径引用与两处手抄模式文案在本轮全部收口，`KNOWN_REVIVALS` 台账**清空**。

- **四处引用收口**：
  - `api/ai.api.js` 删 `aiExpertChat`（网关已路由但 `multi`/`debate` 分支读 `results`、`summary` 两个子 handler 从不产出的键 ⇒ `content` 恒为兜底串）；
  - `stores/ai.store.js` 的专家模式不再改道该端点，改走模块六模式契约 `allianceApi.collaborate()`；助手人格（`ASSISTANTS`）回落通用 `aiChat`——**人格不等于联盟专家**，旧代码里"选了架构助手就去调联盟接口"是取数错道；
  - `composables/useSSE.js` 的 JSDoc 示例 URL 从假想的整流程流式端点改为真实存在的任务日志流，并删掉 GET 示例里多余的 `body` 行；
  - `stores/alliance.store.js` 运行面停用（它整个建立在那条不存在的流式端点上）：删 `streamEndpoint` 配置键、`useSSE` 依赖与 SSE 事件处理链（`handleSSEEvent`/`handlePhase*`/`handleProgress`/`handleComplete`/`handleErrorEvent`/`handleSSEError`/`addPhaseMessage` 共约 180 行，均为模块内私有且从不导出，删除零行为影响）。
- **阶段标签第五副本**：同一 store 的 `PHASE_META` 原先自带一张中文表（`组队匹配/专家辩论/综合归纳/知识学习/完成`），与 Rust 单源投影（`组队路由/并行咨询 + 辩论/归一合成/指标学习/终态`）已漂移；现改为从 `contract/phases.js` 的 `PHASE_META` 派生，本文件只留展示用图标——顺带去掉了 7 个裸 hex：`check-view-hex.py --check` 实测该文件 `14 → 6` SHRANK（同轮 `composables/workspace/useAlliance.js` `8 → 5` 为 §5.5 的余量）。**基线未回填**：该棘轮的 `--baseline` 是整表重写，而 `src/components/ai/PhasePipeline.vue 37 → 0` 这行按裁决**永不可回填**（咨询项），本轮 `Login.vue 9 → 12` 的 GROWN 也是先前遗留、待用户在三个选项间裁决。
- **模式文案第三、四副本**：`stores/ai.store.js` 的 `CONSULT_MODES` 原是一张手写表且把 `smart` 标成"智能路由"（与 `contract/collab.js` 里 `route` 的标签撞名），现改为 `general` + `COLLAB_MODES` 的投影，标签/占位/说明只有一处真值；`views/expert/panels/ExpertOverviewPanel.vue` 的 `currentPlaceholder` 手写映射一并删除，只保留"已选 N 位专家"这个当轮事实前缀。切换器因此从 5 页签变 7 页签（`general` + 六模式）。
- **协作结果聊天投影单源**：新增 `model/collabChat.js`（`collabChatSpeaker`/`collabChatText`/`collabChatPhase`），工作台对话栏与 AI 聊天共用一段正文，按 `resultKind` 只取该模式确实产出的字段，并显式写出 `（模板降级回复，未经真实模型）`、`（路由只排序作答，不产出回复）`、`（后端未返回融合摘要）`——后端没产出的东西不许被省略式伪装成"没有内容"。
- **防复活门禁随之改形**（`contract/forbidden-revival.test.js`，8 → 9 例）：台账清空后判据从"与台账相等"变成事实上的 **src 内零允许引用**；同时补一条**分母断言** `扫描集 ≥ 250`（实测 257 个非测试 `.js/.vue`）——否则"把 src 扫成空目录"也能打印出同一个漂亮的零。牙齿实测 6 枚变异体各自打红：① 在 `useSSE.js` 复活恒零桩路径 ② 抬高扫描下限 ③ 改 `collabChatPhase` 的路由阶段戳 ④ 删 `fusion` 分支 ⑤ 翻转 `source === 'llm'` 判据 ⑥ 删 `answer` 分支；全部还原后按字节哈希确认干净。新增 `model/collab-chat.test.js`（6 例）钉住六个模式各自走到自己口径（不许掉兜底分支）、缺字段必须显式"未返回"、拦截态一律指向 `gate`。
- **门禁自身有效**：新文件初版用 `../contract/collab.js` 跨目录相对导入，被 E2 直接打红后改 `@/modules/expert-alliance/contract`（E2/E6/E7 不是装饰）。
- 验证：`vite build` RC=0；`check-frontend-module.py` ERROR=0（E1-E7 全 ✓）；全量 vitest **54 文件 / 737 例 / RC=0**（对 §5.5 的 53/730 增量＝`collab-chat.test.js` 6 例 + 复活台账新增的扫描集分母 1 例）；`check-view-hex.py --check` 仍 FAIL，其唯一 GROWN 是先前遗留的 `src/views/auth/Login.vue 9 → 12`，与本轮无关。
- ⚠️ 遗留待裁决：`stores/alliance.store.js` 收口后只剩"结果快照 + 历史持久化"的空壳且**全库零消费者**（`stores/index.js` 之外无人 import），整体退役（删文件 + 删出口）需要点名授权，本轮不擅自删。


## 5.7 图谱页名称出口收口：一串问号在 11 个出口逐个显形（2026-09-27）

后端写入侧丢过编码（`GET /api/expert-graph` 45 个节点里两个专家的 label 原样是 `'???????'` 与 `'?????????'`）。本轮把这件事在**图谱这条链路的全部出口**逐个钉死，并把丢失原因归到"数据在库里就是问号"，不是读取路径、不是字体。

- **丢失位置已由读数证实**：只读探针查 `data/experts.db` 的 `experts` 表，两行的 `name` 分别存成 9 个与 7 个 ASCII `?`，第二行的 `title` 也是 5 个 `?`，`created_at` 都是 `2026-09-12`。字符数与中文姓名字数一一对应 ⇒ 每个非 ASCII 字符在**落库那一刻**各变成了一个 `?`；而同网关 UTF-8 请求体的 POST→GET→DELETE 往返能把中文按字节原样读回 ⇒ 读路径是干净的，坏的是那一次写入。原名不可恢复，修法只能是重新登记名字（属数据修复，待用户裁决，本轮不擅改库）。仓库根的 `add_registry.py` 时间戳是 09-19，**排除**它是这两行的写者。
- **口径只有一个真值**：判丢 `contract/graph.js` 的 `isLostGraphLabel`、指认用 `graphNodeShortId`、显示用 `graphNodeLabel`（丢码 ⇒ `未命名节点 <id 短码 8 位>`）。图谱页 11 个出口全部走这三个函数，不再各写一份 `x.label || x.id`：画布文字标签与 `<title>`/aria、画布"编码丢失 N 枚"图例、检视器标题/邻域/协作者、工作台面板标题、**store 的两个候选下拉**（专家候选、能力域候选）、**BFS 链路文本**、**中心性排行跳转**、**社区成员串**、**团队成员跳转**。后五个是本轮新增。
- **两种缺名不是一回事**：`label` 有值但全是问号 = 写入侧丢了编码，必须显形；`label` 压根没给 = 调用方只拿到 id，沿用裸 id（`pathChainText` 两态各一枚针）。社区成员是第三种形状——wire 的 `member_labels` 只有字符串没有 id，无法逐个指认，因此合并成 `未命名节点 ×N`，宁可含糊也不假称读得懂；成员为空与成员全丢分别是 `（无成员）` 与 `未命名节点 ×N`。
- **新增测试**：`store/alliance-graph.store.test.js` +3 例（专家通道、能力域通道各一枚，再加一枚"两个列表都不许出现问号也不许空标签"——分通道是为了让撤掉任一通道的变异体只能红自己那枚针，不替别人红）、`contract/graph.test.js` +1 例（链路文本两态）、新建 `components/graph-panels-name.test.js` 7 例（这两个面板此前零覆盖）。电池 TESTS 面 5 文件 **79 例**，全量 vitest **60 文件 / 814 例 / RC=0**，`vite build` ✓ 22.53 s。
- **变异电池 25 枚（M1–M25，9 个源文件）全数打红点名针**：基线 79/79 有判决、每枚 `old` 锚点全文件命中恰好 1 且确实改动该行（跑前预演 bad=0）、跑完 9 个文件 sha 逐字节 IDENTICAL，并用 `sha256sum` 独立复核过。本轮新增的 6 枚各撤一条通道：M20 专家候选退回 `n.label || n.id`、M21 能力域候选退回 `n.label`、M22 链路文本撤掉丢码分支、M23 中心性跳转退回 `row.name || row.id`、M24 社区成员退回裸 `join`、M25 团队成员跳转退回 `m.name || m.id`。电池驱动是 scratch（`%TEMP%/mox-mut/battery.py`），**未接 CI**。
- **裸 hex 台账手工回填 4 行**：删 `views/workspace/panels/GraphCanvasPanel.vue 8`、`composables/workspace/useGraphCanvas.js 2`、`views/expert/ExpertCenterView.vue 17`（三处实测已为 0，清零要**删条目**而不是写 0），改 `stores/alliance.store.js 14 → 6`、`composables/workspace/useAlliance.js 8 → 5`。**没跑 `--baseline`**：那是整表重写，会把别人的债一起赦掉。`--selftest` 仍 `PASS=59 FAIL=4`（与本轮无关的四格，见 §5.6 与门禁文件），`--check` 的唯一 GROWN 仍是先前遗留的 `src/views/auth/Login.vue 9 → 13`，本轮改动未新增任何裸 hex。
- **门禁自身修了一处真缺陷**：`scripts/gate/check-frontend-module.py` 打印 `✓` 在 Windows GBK 控制台直接 `UnicodeEncodeError`，也就是说 AGENTS.md 里那条文档命令在这台机器上必崩、门禁形同不存在（本轮实测崩过）。补上与其他门禁同款的 stdout utf-8 守卫后 `rc=0 / ERROR=0（E1–E7 全 ✓）`；正对照在临时目录造了一份假 `frontend-ui` 树（一个 `from 'element-plus'` 根导入 + 无 `epSubpathResolver` 的 vite.config）→ `rc=1 ERROR=2`，中文报错原样印出，证明守卫没把牙齿一起换掉。同款无守卫的还有 3 份（`frontend-ui/scripts/gate/` 下 `_mutant_role_judgments.py`、`check-view-hex-mutants.py`、`plan-view-hex-clearance.py`），本轮只改被文档承诺的那条，其余**只报不改**。
- 其余门禁：`check-framework-imports.py` 非测试源文件用而未绑 **0 个 / 0 处** PASS；`check-ep-feedback-imports.py --check` 仍是 28 文件 / 420 处（探针，不改 rc，见 §ElMessage 那条），本轮未新增缺 import。
- ⚠️ **本轮明确划在边界外**：属性值家族（成员 `title`、`coveredSkills`/`coveredDomains` 与技能候选 chip）没收——它们同样可能带问号，但 `skillOptions` 是 `value === label`，把显示改标会连带把**请求体**改成后端查无此技能的名字，需要另一种处置（显示层标注 + 保留原值），另案。模块里还有 **19 处**吃注册表 `name` 的显示位点（`AllianceExpertsView` 6 处、`AllianceConsoleView` 2、`AllianceOrchestrationView` 3、`AllianceSessionsView` 2、`ExpertCard` 2、`ExpertCapabilityMatrix` 1、`collabLists` 2、两个 store 各 1、`contract/registry.js` 1），待下一单元以单一 `expertDisplayName` 收口；那里的三态口径必须保住"注册表里没有这一行"（现在由调用方自己说）与"名字丢了"的区别。
- ⚠️ 遗留待裁决（不变）：`stores/alliance.store.js` 收口后全库零消费者，整体退役需点名授权；`PhasePipeline.vue 37 → 0` 按裁决**永不可回填**；`FRONTEND-MODULE.md` §8（用例数权威）目前人在 `docs/expert-alliance/_archive/v1/`，归档位与权威位冲突待裁决。



## 5.8 注册表姓名三口径收口：30 个姓名调用点归一，"没这一行"与"丢了码"分开说（2026-09-27）

- 承 §5.7 划在边界外的那条清单开做。§5.7 当时预估"19 处"，**实测更大**：模块内 **30 处姓名调用点分布在 12 个文件**（`views/AllianceExpertsView.vue` 7、`components/ExpertCollabPanel.vue` 7、`store/alliance-experts.store.js` 3、`components/ExpertCard.vue` 3、`views/AllianceConsoleView.vue` 2、`model/collabLists.js` 2，另 6 个文件各 1）。其中 **5 处原本连兜底都没有**（模板直出 `{{ e.name }}`、`:title="expert.name"`），问号会一字不改地印到界面上。
- 三口径单源仍在 `contract/graph.js`（该文件依旧零 import）：`expertDisplayName(expert)` 管"好名字原样／丢了码 → `未命名专家 <id 短码>`／连 id 都没有 → 只说未命名专家"；**新增 `expertNameOr(expert, absentText)`** 专门服务"后端可能压根没给名字"的出口——一个字符都没给时把话**交回调用方**（调度台那句"（注册表里没有名字）"仍由视图自己说，模块不编造人名），给过但丢了码的仍走短码。判空只看是否空串，空白串算"给过但丢了"，与 `graphNodeLabel` 同口径。
- 消重 4 份副本：`AllianceOrchestrationView` 与 `AllianceSessionsView` 各抄过一份三行的 `expertOptions`/`expertNames`，现由 experts store 单源给出，视图只剩一行委托。
- 新针：模块内第 24 个测试文件 `registry-name-outlets.test.js`，15 例。除逐出口语义针外含**两本源码台账**：① 裸 `.name ||`／`.name ??` 兜底只剩 5 条，逐条登记"为什么它不由 `expertDisplayName` 负责"（能力名不是人名、写入侧草稿、过滤判据、图谱节点归一化、空串=目录里没这行）；② 模板直出 `.name` 只剩 4 条（`ExpertRankBoard` 的行名已由 `model/rank.js` 过函数、`ExpertRegistryForm` 是能力名输入框、编排页 2 条是流程步骤名）。台账按**整行文本**比对、命中数须等于台账数、行号不入账，多余条目与失效条目各自变红。
- 变异电池扩到 **39 枚 / 18 个文件 / 6 个测试套件**：基线 94 例全绿 rc=0，**39/39 全部把各自指名的针打红（0 枚盲）**，18/18 文件复原 sha IDENTICAL，复跑基线仍 94/94；另以 `sha256sum` 做第二工具独立复核本轮 10 个文件 ⇒ 10/10 MATCH。本轮新增 M26–M39 共 14 枚，每枚只撤一条通道（删除后果清单／调度重置确认／贡献行／候选行／聊天发言者／榜单行名／候选下拉／id→名字映射／停用通知／负载行／丢码判据／"没这一行"分支／模板直出 ×2）。
- 全量验证：`vitest` 全量 **61 文件 / 829 例 / rc=0**（§5.7 是 60/814，本轮 +1 文件 +15 例）；`vite build` rc=0（24.57 s）；模块门禁 `ERROR=0` rc=0；`check-view-hex.py --selftest PASS=59 FAIL=4`（既有 4 红，形状与 §5.7 记录一致）；`--check verdict=FAIL` 仍只由他人那处 `Login.vue 9 → 13` 的 GROWN 造成，本轮**未新增裸 hex**（`grown=1 new=1 palette=0 tier=0 surface=0 role=0 textasfill=33/0`）；两把 import 探针：`check-framework-imports.py` rc=0、`check-ep-feedback-imports.py --check` 28 文件 / 420 处 PASS（探针不改 rc）。
- 顺带收掉 §5.7 记的另一条边界外项：`scripts/gate/` 三把驱动此前**无 stdout 编码守卫**（`plan-view-hex-clearance.py`、`check-view-hex-mutants.py`、`_mutant_role_judgments.py`），已各补一处并实测——这台机器 `sys.stdout.encoding` 即 gbk，且重定向到文件也一样；正对照 `print('探针 ⇒ …')` 无守卫 rc=1（`UnicodeEncodeError`，U+21D2）、带守卫 rc=0。补守卫后三把都跑到各自的判决行：`HEX-MUTANTS: PASS（15 枚变异体…）`、`CLEARANCE: 定价完成（只读，未写任何文件）`，role 驱动印"基线不绿 ⇒ 变异结果全部作废"并 rc=1——那是它拒绝在无干净基线下出判决，属预期行为而非回归。三把跑完后 `scripts/gate/` 无残留新文件，被测闸门 sha 前后不变。
- ⚠️ 仍划在边界外（下一单元）：~~工作台与会话读路径仍吃后端 snake_case（`session.updated_at` 未过 `normSession`）~~ **已于 §5.9 收口**；`skillOptions` 家族仍 `value === label`（改显示会连带把请求体改成后端查无此技能的名字，需另一种处置）；`src/stores/alliance.store.js` 实测 **0 importer**（10.9 KB），整体退役需点名授权。



## 5.9 工作台会话读路径归一：三个后端根本不存在的键，与两枚"杀不动"的针（2026-09-27）

- **本轮拆的是"猜形状"**：工作台把会话行当成 `{ updated_at, expert_count, mode }` 来读，而网关 `session_to_list_view`（`platform/gateway/mox-platform-gateway-svc/src/alliance/experts_session.rs` 113-129 行，实测 13 个键）**从不发**这三个名字，只发 `expert_ids / session_type / created_at / last_active_at`。读不存在的键在 JS 里不抛错，只是恒为 `undefined` ⇒ 时间列与专家数只能拿到 `undefined`（Vue 插值塌成空串，视图里那份自写的毫秒算法则产出 `NaN/NaN`），模式标签永远走 `'协作'` 兜底。坏得静悄悄，且四处同因。
- 四处可证缺陷与修法：① 取数从猜响应形状改为 `allianceApi.listSessions({ pageSize: SESSION_PAGE.defaultSize })` 并直接用归一化后的 `{ items }`；② 活动时间口径统一到 `sessionActivityAt`（`last_active_at` 优先、`created_at` 兜底），视图不再自己减毫秒；③ 专家数改读归一化后的 `expertIds.length`；④ 视图那份协作模式词表（`smart/algorithm/…`）与后端 `session_type` 词表（`single/multi/debate/enterprise`）是**两套不相交的命名空间**，`sessionModeLabel/sessionModeType` 已删，改由 `sessionTypeLabel` 与新增的 `sessionTypeTagType` 供给（未知类型原样显形并给 `danger`，不许换成中性色把缺陷盖住）。
- 本地草稿行不许自造形状：新增 `draftSession({ id, title, expertIds, sessionType, at })`（`model/normalize.js`），wire 构造收在模块内、出口仍是 `normSession`，视图不再手写 snake_case 键。**这一步是被台账逼出来的**：视图里原先写 `expert_ids:` 立刻被 §5.6 的防复活台账点名（3 处红），正解是把 wire 搬进模块，而不是给台账开豁免桶。草稿的 `status` 故意留空——本地行从未落库，界面不许把它读成 `active`。
- 新增出口 6 个：`contract/sessions.js` 的 `sessionActivityAt`/`sessionTypeForCollabMode`/`sessionTypeTagType`/`sessionListTitle`、`model/display.js` 的 `sessionTimeText`（相对档落空才退绝对档）、`model/normalize.js` 的 `draftSession`；并把 `SessionListPanel.vue` 里那份"空标题兜底"副本并进 `sessionListTitle`。
- 新针放在**模块外**：`src/views/workspace/workspace-sessions.test.js`，实测 **25 例**（与全量差值 854 − 829 = 25 互相印证）。放模块外的理由写在文件头：外壳接线一旦改动就要重跑整台模块电池。除逐出口语义针外，含一本 `views/workspace` 读路径台账（`session.updated_at|expert_count|mode`、`getExpertSessions` 逐行清零，另设"必须出现"正向针，扫描集为空即红）。
- **变异电池 55 枚（M1–M55，23 个源文件 / 7 个测试套件）第一跑是 INVALID**：53 枚 RED-OK，**M49、M51 两枚没能打红各自点名的针**。两枚同一种病——变异体撤的是"退化成空值/零"的通道，而被点名的针用的是 **0/空 fixture**，两个取值方向都得 0，结构上不可能变红：M49 把 `last_active_at` 写成 `''`，`sessionActivityAt` 当场从 `created_at` 兜底回来，显示一字不差；M51 让面板读 `s.expert_count || 0`，归一化对象里本就没有这个键，空成员行照样是 0。修法是给两枚针补上"能区分取值"的那一半：草稿针先认字段本身（`empty.lastActiveAt`）再认访问器；空成员行的 fixture 在 `normSession` **之后**混进一个"后端哪天给了个不一致的 `expert_count: 7`"，读错键就说 7 位。第二跑 **55/55 全 RED-OK**，基线 119/119 rc=0，跑完 23/23 文件 sha IDENTICAL，并以 `sha256sum` 对**两份独立见证**各复核一次（电池自报的原始哈希 23/23、14:37 落盘的 `src` 树外备份 23/23）。电池驱动仍是 scratch（`%TEMP%/mox-mut/battery.py`），**未接 CI**。
- 全量验证（电池结束后重跑，非回忆）：`vitest` 全量 **62 文件 / 854 例 / rc=0**（§5.8 是 61/829）；`vite build` rc=0（33.03 s）；模块门禁 `ERROR=0` rc=0（那 22 处块注释配对 WARN 全在他家文件，本轮改动零命中）；`check-framework-imports.py` 0 个 / 0 处；`check-ep-feedback-imports.py --check` 28 文件 / 420 处（探针不改 rc）；`check-view-hex.py --selftest PASS=59 FAIL=4`（四红形状与 §5.6 记录一致）；`--check verdict=FAIL` 仍只由他人的 `Login.vue 9 → 13` GROWN 与 `AdminSso.vue` 2 处 NEW 造成，裸 hex 存量 931 处 / 78 文件（基线 962 / 78），**本轮未新增裸 hex**，`PhasePipeline.vue 37 → 0` 按裁决仍不可回填。§5.6 的防复活台账与 §5.7/§5.8 的姓名台账在两轮电池里始终全绿。
- ⚠️ **重账未清**：本轮改了模块源（`contract/sessions.js`、`model/display.js`、`model/normalize.js`、`components/SessionListPanel.vue`），仓库内那台 138 枚合并电池（`frontend-ui/scripts/check-collab-mutants.py`）按规则**整笔重挂**——它的合并整跑至今一次都没跑满，勿引用旧绿灯。
- ⚠️ 仍划在边界外（下一单元）：知识库面的 `doc.updated_at` 共 4 处 —— `views/workspace/panels/KnowledgeBasePanel.vue`（78 行）与 `views/project/panels/KnowledgeBasePanel.vue`（234、306 行，同名面板的另一份）显示，`composables/useKnowledgeBase.js`（118、122 行）排序 —— 本轮实测**该字段真的存在**（`platform/domains/kg/svc/mox-kb-svc/src/model.rs` 57-71 行的 `KbDocument` 与 document.rs 44-50 行的 `DocSummary` 都带 `created_at/updated_at`，snake_case 无 rename），因此那 4 处不是"猜形状"而是**显示口径**问题，~~另案~~ **已由 §5.10 收口**；顺带发现 `/kb/documents` 在两个 crate 里各注册一遍（`platform/domains/kg/svc/mox-kb-svc/src/handlers.rs` 440 行与 `platform/domains/kb/svc/mox-kb-server/src/main.rs` 44 行），网关背后是哪一份待查 —— **§5.10 已查明**：网关 `platform/gateway/mox-platform-gateway-svc/src/deployment.rs` 25 行把 `mox_kb_svc::handlers::build_kb_router()` nest 在 `/api`（另见 `modules.rs` 130-132 行"采用 nest 包装"），故前端走的是前者；后者是旧宿主（`docs/api/PORT-REGISTRY.md` 95 行记 3414 不可同时占用），且其 `list_documents_handler`（该文件 70-72 行）**直接返回空表桩** `{"documents":[],"total":0}`，只报不改。`views/workspace/panels/CollaborationPanel.vue`（15 行）的 `activeSession?.title || '未开始'` 是空标题兜底的又一副本；`skillOptions` 家族仍 `value === label`；`src/stores/alliance.store.js` 实测 0 importer，退役需点名授权。



## 5.10 时间显示口径归一：16 个文件自写了 17 遍，而相对档一次也没触发过（2026-09-27）

- **病根不是重复，是输入口径**。后端时间一律 RFC3339 字符串（`platform/domains/kg/svc/mox-kb-svc/src/model.rs` 126-128 行的 `now_iso()` = chrono `to_rfc3339()`，会话侧同源见 §5.9），而 `views/workspace/panels/KnowledgeBasePanel.vue` 自写的那份写 `const diff = now - ts` ⇒ 字符串相减得 **NaN** ⇒ 全部 `diff < X` 恒假 ⇒ "刚刚 / N 分钟前 / N 小时前 / N 天前"四档一次都没机会触发，界面悄悄退化成那份副本最后一行的 `${月}/${日}`（**连年份都没有**）。这条按 `git show HEAD:` 取到的原文证实（十行版本，函数体 sha `e34cde9e03a5acfa`），不是凭印象。
- **重复的规模（本轮前实测）**：`frontend-ui/src` 下 `.js/.ts/.vue` 共 **322 个**文件，自写 `formatTime`/`relativeTime` 定义 **17 处 / 16 个文件**（模块内 `model/display.js` 一个文件两处，那两处是联盟模块的权威口径）。分四族，两族各自**逐字节相同**：A 族（手工拼 `YYYY-MM-DD HH:mm`）三份函数体 sha 同为 `bc1b7a3afd18a278`——见证是 **git HEAD 的三个 blob**（`utils/knowledgeBase.utils.js`、`utils/message.utils.js`、`views/project/panels/KnowledgeBasePanel.vue`），不是工作树；B 族（`toLocaleString('zh-CN', { hour12: false })` 包在 try/catch 里）三份同为 `791ebd8c5100169d`（`AdminDepartment.vue` 620 行、`AdminRole.vue` 889 行、`AdminUser.vue` 689 行，当场量的磁盘原文）。C 族即上面那份坏档，D 族是其余各形。
- **单源落地**：新建 `src/utils/time.js`，四出口 `timeValue` / `formatDateTime` / `relativeTimeText` / `timeAgoOrDate`（barrel `src/utils/index.js` 10 行转发）。`timeValue` 才是本轮真正的修复点：Date、数字、纯数字串、RFC3339 串**先落成毫秒再谈档位**；纯数字串那一支不可省——`new Date('1758000000000')` 在引擎里就是 Invalid Date。坏值判据写成 `!(t > 0)` 而不是 `Number.isNaN(t)`：本轮合并掉的四份旧副本全都以 `if (!ts) return …` 开头，`0` 在旧实现里算"没有值"，换成 NaN 判据会把 `0` 印成 1970-01-01。
- **收口四处**：三份 A 族副本的函数体删除、两份 utils 不再自带定义（它们本身就在 `utils/` 里，再 import barrel 就是环，所以台账只钉"不许抄回来"）；`views/project/panels/KnowledgeBasePanel.vue`（1006 行）改为 `import { formatDateTime as formatTime }`，`views/workspace/panels/KnowledgeBasePanel.vue`（228 行）改为 `import { timeAgoOrDate as formatTime }`——那里要的正是相对档。别名保留是因为调用点密集（项目面板 7 处模板调用：234/306/357/416/554/753/757 行；工作台面板 2 处：78/122 行）。
- **barrel 的旧病值得单独记**：`utils/index.js` 曾把两份**逐字符相同**的副本以 `formatTime` 与 `formatTime as kbFormatTime` 两个名字同时导出——同名冲突是靠别名绕过去的，不是靠其中一份更好。现在 barrel 只出 `time.js` 的四个名字，并且用**运行时刻**钉住：`Object.keys(utils)` 里既不许有 `formatTime` 也不许有 `kbFormatTime`。
- **新针**：`src/utils/time.test.js` **24 例**（全量差值 878 − 854 = 24 互相印证），五个语义 describe 加一本台账。台账 `COPY_RE` 只扫非注释行的函数定义，`KNOWN_COPIES` 记 **12 个文件 / 13 处**（权威 2 + 副本 11），新增即红、清零须删条目；另有扫描集下限（实测 322，下限 300，余量给删除）、正对照 4 条定义形态必须被点名、反对照 6 条必须不被点名（别名 import、调用点、注释、`formatTimezone`、`relativeTimezone:`、`formatTimeFn`）。
- **测试全绿 ≠ 改完了**：barrel 改名之后 `vitest` 依旧 rc=0，是 `vite build` 连着两次报 `"formatTime" is not exported by "src/utils/index.js"` 才点名出两个真实消费者（`composables/useMessageActions.js` 7 行、`composables/useKnowledgeBase.js` 23 行）。我先前用单行 grep 找消费者报的是 **0 处**——**多行 import 子句单行 grep 根本看不见**。结论沿用 §5.9 那条并再加一次见证：**rollup 才是 barrel 消费者的权威清单**。另一条自伤记录：`useKnowledgeBase.js` 里 `  formatTime,` 出现两次，Edit 直接拒绝（期望 1 命中），锚点要宽到能唯一定位。
- **变异电池（scratch，`%TEMP%/mox-mut/battery.py`，未接 CI）**：`patch_u6.py` 把电池从 55 枚扩到 **69 枚 / 29 个源文件 / 8 个测试文件**。M56–M69 各撤一条通道：数字串分支、Date 分支、`0` 当有值、负 diff 落到"刚刚"、`<= HOUR` 的整点差一、超一天不闭嘴、把 `now - t` 换成 `now - 原始入参`（正是旧副本的病）、`pad2`、`timeAgoOrDate` 丢掉绝对档、barrel 改回 `formatTime` 别名、以及四份"把已合并的副本抄回来"（两份 utils + 两个 KB 面板）。跑前预演 69/69 锚点各命中 1 次；**基线 143 例全绿 rc=0**；结果 **`电池判决: 69 枚中 0 枚未打红点名针 ⇒ PASS`**（RED-OK 69 / BLIND 0 / ANCHOR INVALID 0 / CRASH 0），复跑基线仍 143/143。还原见证两份独立：驱动器自报 29 个 sha 全 IDENTICAL，另把 29 份原文在跑前抄到仓库外 `u6-backup/`（哈希登记 `u6-backup-hashes.txt`）用新写的 python 逐文件比 sha256 ⇒ **29/29 相符**，再用 coreutils `sha256sum` 抽验（`time.js` 全 64 位与仓库外备份一致）。
- ⚠️ **电池跑后又动了三行文字**（`time.js` 两处注释、`time.test.js` 文件头与一枚 `it` 标题的后半括号）——只为把"16 份旧副本全都写 `if (!ts)`"这类**未实测的过称**改成实测口径。这些行不在任何锚点上、也不在电池 `red` 针名的前缀里（针名存的是 `0 按"没有值"处理` 这个前缀），跑后复算 8 文件 **143/143 rc=0**、分母与电池基线一致，故上条判决仍然成立；若今后重跑，`patch_u6.py` 无需改动。
- **全量验证**：`vitest` 全量 **63 文件 / 878 例 / rc=0**（§5.9 是 62/854，本轮 +1 文件 +24 例）；`vite build` rc=0（25.00 s，`vendor-element` raw 659.24 KB，与 §6 基线同档）；`check-frontend-module.py` **ERROR=0** rc=0；`check-framework-imports.py` 非测试源文件用而未绑 **0 个 / 0 处** PASS；`check-ep-feedback-imports.py --check` 仍 **28 文件 / 420 处**（探针不改 rc，本轮未新增缺 import）；`check-view-hex.py --selftest PASS=59 FAIL=4`（既有四红，形状同 §5.8）、`--check verdict=FAIL` 仍只由他人的 `Login.vue 9 → 13` GROWN 与 `AdminSso.vue` NEW 2 造成，**本轮未新增裸 hex**（裸 hex 现存 931 处 / 78 文件，基线 962 / 78；`textasfill=33/0` 与基线同）；`check-doc-links.py` **BROKEN = 42**，与先前记录的基线同数，且指向本文件的 **0** 条（本轮新增段落里的反引号路径全部可解析）。⚠️ 本轮改了两个 KB 面板与两份 utils，按既有账：**仓库内那台 138 枚的 collab 电池因此重新计费**，未跑。
- ⚠️ **仍划在边界外（只报不改）**：余 **11 处副本**——B 族 `AdminDepartment.vue` 620 行／`AdminRole.vue` 889 行／`AdminUser.vue` 689 行，B 族变体 `AdminLlm.vue` 643 行／`AdminSso.vue` 194 行／`BrowserView.vue` 583 行（唯一用 try/catch 不用空值守卫的一处），另 `MessageBubble.vue` 936 行、`ChatView.vue` 170 行、`ExpertEnterprisePanel.vue` 829 行、`ExpertOrchestratorPanel.vue` 444 行、模块内 `ExpertBookingPanel.vue` 106 行。B 族换口径会改动 admin 面板既有显示宽度（`toLocaleString` 与 `YYYY-MM-DD HH:mm` 不等宽），要单独一轮并配真机读数；模块内 `model/display.js` 那两份是权威，是否并到 `utils/time.js` 之下要先裁决"模块能不能 import 全局显示口径"。`composables/useKnowledgeBase.js` 118/122 行按 `updated_at` **排序**不受本轮影响（RFC3339 同格式串按字典序＝时间序），本轮只改了它 23 行的 import 别名。
- ⚠️ 顺带查明的一条 wire 事实（给下一单元）：`GET /api/kb/documents` 由 **mox-kb-svc** 服务（见 §5.9 边界条目的更新），其 list 出参 items 是 `platform/domains/kg/svc/mox-kb-svc/src/document.rs` 44-50 行的 `DocSummary`，**只有 `id/title/category/status/updated_at` 五个键**——没有 `content/tags/created_at/author/doc_type`。因此下一单元该问的是"前端 list 侧读了哪些 `DocSummary` 根本不给的键"（与 §5.9 的"猜形状"同族），而不是 `updated_at` 存不存在。旧宿主 `mox-kb-server` 的 `/api/v1/kb/documents` 是返回空表的桩，不在前端链路上。
- 遗留待裁决（不变）：`src/stores/alliance.store.js` 零消费者、退役需点名授权；`skillOptions` 家族 `value === label`；`CollaborationPanel.vue` 15 行空标题兜底副本；`PhasePipeline.vue 37 → 0` 按裁决永不可回填；他人那两处裸 hex GROWN/NEW；`FRONTEND-MODULE.md` §8 权威位与归档位冲突。

## 5.11 知识库读路径按 wire 归一：三个列表恒空，两套词表零重叠（2026-09-27）

- **病灶一 · 形状**。`frontend-ui/src/api/http.js` 的响应拦截器已经把信封剥掉一层（`if ('data' in body) return body.data`），
  而工作台右侧的知识库五个加载器又去找 `res.data`：`Array.isArray(res.data)` 恒假 ⇒ 文档列表、版本列表、检索结果**三处恒为空数组**，
  分类与标签能用纯属侥幸（那两个端点返回裸数组，走的是 `Array.isArray(res)` 那一支）。
  现在信封解包只在内核一处（`frontend-ui/src/modules/_kernel/envelope.js` 的 `unwrap` + `unwrapList`），四个端点各点名自己的列表键：
  `/kb/documents` → `items`、`/kb/documents/:id/versions` → `versions`、`/kb/search` → `results`，分类/标签是裸数组。
  注意 `unwrapList` 的兜底候选表足以掩盖键名写错，所以"用没用对键"这件事不靠绿灯，靠 §4 台账里逐字钉住的接线串。
- **病灶二 · 词表**。前端 `utils/knowledgeBase.utils.js` 自写了一套"文档类型"（article/tutorial/api/design/report/spec）与
  一套状态（published/draft/archived），而后端 `platform/domains/kg/svc/mox-kb-svc` 只有四档分类（`src/document.rs` 的 CATEGORIES，
  值是 cat-tech / cat-dialogue / cat-business / cat-research）与三种状态（`src/model.rs` 的 draft / analyzed / linked）。
  两边**零重叠** ⇒ "按类型筛选"恒空、状态标签永远走原样显形的兜底。现在词表改名 `KB_CATEGORIES` / `KB_STATUSES` 并由测试现场解析 Rust 源逐对核对。
- **病灶三 · 幻影键**。`doc.type` / `doc.graph_linked` / `doc.size` / `doc.description` / `d.version` / `d.aiAnalysis` 后端都不发；
  `KbVersion` 只有 version / title / content / note / created_at 五键，版本行却在读 `ver.author` 与 `ver.action`（于是恒显"系统 · 更新"，
  把真有的 `note` 藏了起来）。`mapDoc` 从 `d.version` 与 `d.aiAnalysis` 取值 ⇒ `version_count` 恒 1、`ai_analyzed` 恒 false，
  连带把"AI 分析"页签的入口永久关死（`composables/useKnowledgeBase.js` 与项目面板都以 `ai_analyzed` 为门）。
  现在版本数由 `versions.length + 1` 推、分析过与否由 `status` 推。
- **收口动作**。项目面板里那五份私有词表副本（`mapDoc` / `getTagType` / `getTypeLabel` / `getStatusType` / `getStatusLabel`）删除，改取 `@/utils`；
  "按 type 筛选"这条死通道整体删掉（项目面板 6 处、composable 5 处，含状态下拉的 published/draft/archived 三行硬编码）；
  工作台面板的图标与状态改由分类词表给，检索行（SearchHit 没有时间戳）显相关度而不是编一个"未知"占位。
- **台账**。`frontend-ui/src/views/workspace/workspace-kb.test.js` 28 例：四种 payload 形状（含"旧的猜 `res.data` 写法必须恒空"的反对照）、
  词表与 Rust 源同源（分母本身断言：解析出四档分类与三种状态才算数）、`mapDoc` 三例、面板渲染八例、棘轮六格。
  棘轮里 12 条禁令**各配一枚必须点名其目标行的正对照**；反对照那侧另钉三行"同族不同主"的合法 `.type`（通知配色 `notif.type`、
  实体行 `ent.type`、写侧入参 `type: data.type,`）——禁令 #7 从 `\bd(oc)?\.type\b` 放宽到认得 `selectedDoc?.type` 时，这三行就是"别把别人家的键一起吃下"的牙。
- **牙**。变异体 M70–M82 共 13 枚（含"撤掉列表键名""把 category_id 请回来""词表外的分类吞成空白""私有副本回魂"）并入临时电池
  `%TEMP%/mox-mut/battery.py`（现 9 个测试文件 / 30 个源文件 / 82 枚）。**第一跑 INVALID：82 枚中 2 枚未打红点名针**——
  M76 的点名清单里多写了一枚结构上不可能变红的针（那一格用已知分类断言图标与文案，与兜底分支无关），M80 则是**真盲区**：
  禁令 #7 认不得 `selectedDoc?.type`（大写 `Doc` + 可选链），变异体命中 0 个测试。补法：M76 那一格拆成"纯函数兜底"+"面板行渲染"两枚各自成证的针，
  同时放宽禁令 #7。**第二跑 `电池判决: 82 枚中 0 枚未打红点名针 ⇒ PASS`**（RED-OK 82 / BLIND 0 / ANCHOR INVALID 0 / CRASH 0），
  未变异基线 171/171 rc=0、复跑基线仍 171/171、30 个文件 sha 逐字节 IDENTICAL。
  ⚠️ M80 那一跑还额外红了一枚未点名的 `mapDoc 列表行`——查下来是**本轮自写的夹具在飘**：`wireSummary()` 的 `updated_at` 读 `Date.now()`，
  同一条断言里调用了两次，跨毫秒即漂移（变异体只是撞上了它，不是它造成的）。已改成夹具只铸一次，此后同文件连跑三遍 28/28 稳定。
- **全量验证**（电池结束后重跑，非回忆）：`vitest` 全量 **64 文件 / 906 例 / rc=0**（§5.10 是 63/878，本轮 +1 文件 +28 例）；`vite build` rc=0（23.64 s）；
  `check-frontend-module.py` **ERROR=0** rc=0；`check-framework-imports.py` 非测试源文件用而未绑 **0 个 / 0 处** PASS；
  `check-ep-feedback-imports.py --check` 仍 **28 文件 / 420 处**（探针不改 rc，本轮未新增缺 import）；
  `check-view-hex.py --selftest PASS=59 FAIL=4`（四红形状与 §5.8/§5.10 记录一致）、`--check verdict=FAIL` 仍只由他人的 `Login.vue 9 → 13` GROWN 与 `AdminSso.vue` NEW 2 造成，
  裸 hex 存量 **931 处 / 78 文件**（基线 962 / 78）与 §5.10 逐位相同 ⇒ **本轮未新增裸 hex**；`check-doc-links.py` **BROKEN = 42**（与先前记录的基线同数，全是他人搬走的 `docs/expert-alliance/*`），指向本文件的 **0** 条。
- ⚠️ **重账未清**：本轮动了 `utils/index.js`、`utils/knowledgeBase.utils.js`、两份 KB 面板与 `composables/useKnowledgeBase.js`，
  按既有账：**仓库内那台 138 枚的 collab 电池因此重新计费**，未跑。
- ⚠️ **只报不改（本单元边界之外，逐条待裁决）**：
  - 写路径没对过契约：后端 `CreateDocReq` 只认 title/content/category/tags，前端仍发 `type` / `description` / `auto_save` / `version_note`；
    且项目面板的分类树选择器把 `value` 绑到分类**中文名**而非 `cat-*` id ⇒ 新存文档的 `category` 落库成中文，分类计数因此恒 0。这条牵动编辑对话框与契约，单独一个单元 ⇒ **已由 §5.12 收口**（五张请求体按 serde 逐枚点名，分类树绑 `cat-*` id）。
  - `/kb/documents` 的查询参数（q / status / tag / category / start_date / end_date / project_id）后端 `list()` 一个都不吃 ⇒ 服务端筛选是装饰性的，
    真正在筛的只有前端 computed；`SearchRequest` 也不吃 `project_id`。本单元只把请求形状收敛到 `query` + `limit`，不改后端。
  - `aiAnalysis` 那组端点形状（面板与 composable 各一处 `data.aiAnalysis` 读法）没有对过 Rust ⇒ **已在 §5.12 对过**：响应体就是 `analyze.rs::AnalysisResult` 的 11 个键，没有 `aiAnalysis` 那一层包装。
  - `composables/useKnowledgeBase.js` 是**零 .vue 消费者**的孤儿（只有 `composables/index.js` re-export 与 stories 提名），本轮只保证它不再读幻影键，退役与否待裁决。
  - §5.10 记下的 11 处自写时间副本不变。

## 5.12 知识库写路径按 wire 归一：分类落库成中文，改前正文永不归档（2026-09-27）

- **病灶一 · 请求体从来没有对过 serde 契约**。`platform/domains/kg/svc/mox-kb-svc/src/handlers.rs` 五张请求体是
  `CreateDocReq{title,content,category,tags}`、`BatchAnalyzeReq{ids}`、`VersionNoteReq{note}`、`CompareReq{v1,v2}`、`RevertReq{version}`；
  项目面板此前发的是 `type` / `description` / `auto_save` / `version_note`（前三个字段结构体根本没有，第四个走错端点），
  批量分析发 `doc_ids`、回滚发 `target_version`、对比发 `version_from/version_to` ⇒ `#[serde(default)]` 把不认识的键静默丢掉，
  请求"成功"而改动不落。现在这五张请求体的字段清单由测试**现场从 handlers.rs 解析**并逐枚点名（含分母断言：五张结构体各解析出几个字段）。
- **病灶二 · 分类树把中文名落库**。编辑对话框的 `el-tree-select` 用 `:props="{ label: 'name', value: 'name' }"`，
  而 `category` 的合法值是 `document.rs` 的 CATEGORIES 四档 id（cat-tech / cat-dialogue / cat-business / cat-research）⇒
  新建或保存的文档 `category` 存成"技术文档"，`categories()` 用 `counts.get(*id)` 统计因此恒 0（左侧分类计数与按分类筛选同时瞎掉）。
  现在 `value` 绑 `id`、`node-key="id"`，分类点击筛选改回 `node?.id`（此前读 `node?.name` 使筛选恒空）。
- **病灶三 · 保存不建版 ⇒ 改前正文永久丢失**。`document.rs::update()` 是 title/content/category/tags 的增量合并，**不产生版本快照**；
  旧写法把 `version_note` 塞进 PUT，备注被丢、历史页签一条新快照都不长。现在填了备注就先调
  `POST /kb/documents/:id/versions`（`KbVersionService::create` 归档的是**改动前**的正文，必须在 PUT 之前），再写新正文——顺序反了旧正文就再也拿不回来。
- **病灶四 · 响应侧猜键与"双写 v 前缀"**。分析的响应体就是 `analyze.rs::AnalysisResult`（11 键，含 `entities/relations/tags/keywords/summary/expert_score/expert_steps/…`），
  旧代码去找 `data.aiAnalysis` 那一层包装 ⇒ "AI 分析"页签永远空；对比响应是 `{doc_id,diff}`，旧代码读 `data.from/data.to`；
  挂图响应是 `{graph_nodes,nodes_added,edges_added,graph_total_nodes,graph_total_edges}`。另一族是模板自己补前缀：
  `next_version()` 产出 `v1/v2/…`，而界面写了 `v{{ ver.version }}`、回滚提示 `v${version.version}`、对比对话框四处同形 ⇒ 显示成 `vv2`。
- **病灶五 · "搜索实体→逐个挂载"那套对话框的三条通道都是空的，但空的原因各不相同**（这条先把上一版的错判改对了：
  曾据 `handlers.rs` 的路由表断言 `kbSearchEntities`/`kbLinkEntity`/`kbUnlinkEntity` 指向不存在的路由——**错**，它们由网关自己的 `mox-platform-gateway-svc/src/kb_ext.rs` 注册：
  `/api/kb/entities/search` GET 与 `/api/kb/documents/:id/entities` POST/DELETE。逐条实测：
  ① `kb_ext.rs` 的 `search_entities` 里写着"当前无实体数据源"，`ok(json!([]))` 恒空 ⇒ 搜索框无论输入什么都零结果；
  ② 那对 POST/DELETE 写的是网关侧独立的 `DocEntityRelation` 存储，`mox-kb-svc` 的图谱与 `GET /kb/documents/:id/entities`（读文档自己抽取出的实体）都不查它 ⇒
  挂上去的东西在图谱与实体页签两边都不显；③ 而真正的挂图动作 `POST/DELETE /kb/documents/:id/graph-link` 只取 `Path(:id)`、**不收请求体**，
  旧代码却发 `{entity_ids}`。所以图谱页签改为文档级：`isGraphLinked`（由 `status === 'linked'` 推）切两个按钮 + 子图节点列表（`n.label || n.id`、`n.node_type`）。
- **病灶六 · 孤儿 composable 里的三处猜形状**（本轮补）。`composables/useKnowledgeBase.js` 的 `fetchVersions` 写的是
  `Array.isArray(data) ? data : (data?.items || [])`，而 `/kb/documents/:id/versions` 出参是 `{doc_id, versions}` ⇒ 两支都假，版本列表在这个 composable 里恒空
  （§5.11 只把工作台与项目面板的读路径归了，这三行漏在扫描集的另一侧）。另外两处（`/kb/history`、`/kb/entities/search`）后端发的就是**裸数组**，
  旧写法的 `Array.isArray` 那一支恰好接住，属于"能用是因为形状猜对了"。三处现在统一走 `unwrapList(unwrap(data), …)`：versions 点名键，两处裸数组不点名。
  工作台 `loadProjects()` 同族收口：`/api/projects` 由 `misc.rs` 的 `list_projects_paginated` 服务、出参 `{items,total,page,…}`，
  旧的 `data?.list || data?.items || data?.projects` 三支里只有中间那支活着 ⇒ 换成 `unwrapList(unwrap(data), 'items')`。
- **顺带修掉的四处恒空**：统计卡此前读 `stats.value.total/versions/analyzed`，而 `/kb/stats` 只发
  `documents/categories/tags/storage_bytes/graph_nodes/graph_edges` 六个计数 ⇒ 三张卡恒 0；现在"已分析"由列表行的 `status` 本地数（`isAiAnalyzed`），"版本总数"撤下。
  实体行此前用 `el-progress :percentage="ent.confidence"`，`KbEntity` 只有 `id/name/type/frequency/snippet` ⇒ 换成出现次数；
  历史行此前读 `h.action/h.user/h.detail`，而 `/kb/documents/:id/history` 的行是版本快照（`version/note/title/created_at`）⇒ 全部换成真键；
  `statCards` 的 `icon` 从字符串名换成组件本身——模板是 `<component :is="s.icon">` 而本项目没有 unplugin-auto-import，字符串名解析不出图标（四张卡的图标此前一律不显）。
- **台账**。`workspace-kb.test.js` 28 → 33 例（新增第六节"写路径"五格：五张请求体字段清单＋分母、payload 键集合恰好等于 `CreateDocReq` 四键且**站点数=2**、
  响应真键与 v 前缀、分类树绑 id、统计卡计数键）；棘轮从 12 条禁令扩到 **19 条**（新增 `h.action|user|detail`、`.confidence`、
  `linked_entities|doc_ids|analyzed_ids|target_version|version_from|version_to|entity_ids`、`?.aiAnalysis|?.from|?.to`、模板双写 `v{{x.version}}`、`kbSearch({ q`，
  以及最后一条 `Array.isArray(x) ?`——猜形状三元链整体出局，裸数组端点改由 `_kernel/envelope.js` 的 `unwrapList` 第一支接住），
  每枚仍配"必须点名其目标行"的正对照；反对照另补 12 行同族合法写法，其中 `<span>v{{ doc.version_count || 1 }}</span>` 是这轮量出来的——
  第一版禁令 `v\{\{\s*\w+\??\.version` 会把它一起吃下（`version_count` 是前端推导键，不是版本串），收紧为要求 `.version` 紧跟 `}}` 才不误伤；
  `if (Array.isArray(data)) {`、`data?.graph_nodes || []`、`data?.entities || []`、`unwrapList(unwrap(data))` 四行是禁令 #19 的"同族不同主"反对照。
  WIRING 台账为该面板从 3 串扩到 17 串（`mapDoc(result?.document)`、`await api.kbCreateVersion(data.id, { note })`、`{ ids: selectedDocs.value }`、
  `{ v1: …, v2: … }`、`{ version: version.version }`、`api.kbGraphLink(docId)`、`node-key="id"`、`value: 'id'` 等），"删掉代码来通过禁令"这条退路逐格堵死；
  composable 的钉从 2 串扩到 5 串（补 `docVersions.value = unwrapList(unwrap(data), 'versions')` 与两处裸数组写法），工作台补 `const list = unwrapList(unwrap(data), 'items')`。
- **改口一处**。禁令里"发 `q` 直接 400"这句是**没被任何尺子读过的断言**：`SearchRequest.query` 没有 `#[serde(default)]`，
  axum 的 `Json` 提取器拒的是 **422**，且响应体不是 `{code,msg}` 信封（前端错误分支拿到的是 axios 原始报错）。已按实测改文案。
- **牙**。第二台变异电池 `%TEMP%/mox-mut/battery.py` 现 **100 枚（内建 69 + `u7-mutants.json` 31 枚 M70–M100）/ 30 个源文件 / 9 个测试文件**。
  本轮写路径那 18 枚（M83–M100）先跑出一跑 INVALID：**M94 BLIND**（版本列表退回 `Array.isArray(data) ? data : (data?.versions || [])`，点名 2 针只中 1）——
  按"漏 = 点错名 vs 针真没牙"两型归因，复查确认是**禁令真没牙**：18 条里没有一条认得这条三元链（`.data` 那条要求 `x.data` 形状，`data?.documents` 那条只钉 `documents`）。
  补法守"先量 → 再加针 → 复跑判据 → 把旧形状做成变异体复验"的序：四候选形状先在扫描集与 18 行已知合法写法上量（候选 `?.<键> || []` 误伤 `data?.graph_nodes || []` 这类真键 ⇒ 弃；
  候选三元链命中 4 行、其中 3 行是真缺陷或"侥幸活着"）⇒ 收三元链禁令并把那 4 行先归一到 `unwrapList` ⇒ `workspace-kb.test.js` 33 例复跑绿 ⇒ 重跑电池：
  **`电池判决: 100 枚中 0 枚未打红点名针 ⇒ PASS`**（RED-OK 100 / BLIND 0 / CRASH 0 / ANCHOR INVALID 0），M94 现 红2/176 点名命中2。
  未变异基线 `rc=0 用例总数=176 通过=176 失败=0`、复跑基线仍 176/176；还原核对**两把尺子**：驱动自报 30 份全 IDENTICAL（0 DIRTY），
  跑前的仓库外副本 `%TEMP%/mox-mut/bak-u8b/`（`manifest.json` 逐份记 sha 前 16 位与字节数）经独立脚本复算 **核对=30 相符=30 不符=0**。
  ⚠️ 顺带抓到一处**预演脚本自己的假阳**：我给预演加了"锚点之外还要求 `new` 命中 0"的判据，而删除型变异体 `new=''` ⇒ `bytes.count(b'')` 返回 `len+1`（M8 因此被报"命中 4888"），
  100 枚里 10 枚被误报锚点坏。驱动的判据只有"`old` 恰 1"，预演不许自加直觉判据。
- **全量验证**（电池结束后重跑，非回忆）：`vitest` 全量 **64 文件 / 911 例 / rc=0**（§5.11 是 64/906，+5 例＝守卫 28→33）；`vite build` rc=0（24.62 s）；
  `check-frontend-module.py` **ERROR=0** rc=0；`check-framework-imports.py` 非测试源文件用而未绑 **0 个 / 0 处** PASS；
  `check-ep-feedback-imports.py --check` 仍 **28 文件 / 420 处**（探针不改 rc，本轮未新增缺 import）；
  `check-view-hex.py --selftest PASS=59 FAIL=4`（四红形状与 §5.8/§5.10 记录一致）、`--check verdict=FAIL` 仍只由他人的 `Login.vue 9 → 13` GROWN 与 `AdminSso.vue` NEW 2 造成，
  裸 hex 存量 **931 处 / 78 文件**（基线 962 / 78）、文字档做底 33 处 / 15 文件，与 §5.11 逐位相同 ⇒ **本轮未新增裸 hex**；`check-doc-links.py` **BROKEN = 42**（与 §5.11 记录的基线同数，全是他人搬走的 `docs/expert-alliance/*`；合并本节之后重跑，指向本文件的断链 **0** 条）。
- ⚠️ **重账未清**：本轮又动了扫描集内 4 个文件（两份 KB 面板、`api/kb.api.js`、`composables/useKnowledgeBase.js`、`views/workspace/ExpertWorkspaceView.vue`），
  按既有账：**仓库内那台 138 枚的 collab 电池因此再次重新计费**，未跑。
- ⚠️ **只报不改（后端与边界之外，逐条待裁决）**：
  - `handlers.rs` 的 `kb_doc_graph_unlink` 只回 `{status:"unlinked",nodes_removed}`，**不重置 `doc.status`**（挂图那侧会写 `STATUS_LINKED`）⇒
    解除关联后重进详情，文档仍是 `linked`，图谱页签的按钮又翻回"已关联"。前端只能本地打补丁，修它得动 Rust。
  - `KbVersionService::list` 以版本**字符串**倒序 ⇒ 第 10 版起 `v9` 排在 `v10` 前面；`compare` 的缺省"最近两版"取 `all[1]/all[0]` 也随之错。
  - `/kb/documents` 的查询参数后端仍一个都不吃（§5.11 已记）⇒ 列表行的标签/摘要筛选在服务端是装饰性的。
    与之相反，`/kb/search` 是 KB 里**唯一真吃服务端筛选**的端点：`model.rs::SearchRequest` 收 `query`（必填，见上文"改口一处"）、`limit`（默认 20）、
    `category`（`search.rs::search_docs` 真按它过滤）。前端只发 `query` + `limit`，**分类下推这条现成能力白放着**（当前分类筛选全靠前端 computed）。
  - `kb_history` 签名是 `Query(_params)` ⇒ `/kb/history` **无视任何过滤参数**返回全局版本流水；
    composable 的 `fetchHistory(docId)` 传 `{doc_id}` 并什么都过滤不掉（文档级历史该走 `kbGetDocHistory`，项目面板已经在走）。
  - 实体侧三条通道存在但语义各空（见病灶五）：`kb_ext.rs` 的实体搜索是硬编码 `[]` 桩，`DocEntityRelation` 存储与 mox-kb-svc 图谱互不查。
    要把"实体级关联"做成真能力，得动 Rust 侧统一存储，不是前端一轮能收的账。
  - `/api/projects` 与 `/api/tasks` 一样是**服务端分页**（`list_projects_paginated` 默认 `page_size=20`、`created_at desc`），
    而 `loadProjects`（工作台）、`ProjectsView.vue` 两处、`composables/projectContext.js` 都按"全量列表"用 ⇒ 项目选择器只看得见最近 20 个项目。
    这与 §5.11 末那条 `/api/tasks` 恒空同族（前端不认列表键 vs 不吃分页），单独一个单元。
  - `AnalysisResult` 里 `keywords / summary / expert_score / expert_steps / elapsed_ms` 五个键前端一个都没用。**本节初稿曾把这条写成"专家联盟对每篇文档的评分与推理步骤后端次次都算"——那句是错的，已按实测作废**：
    `analyze.rs:55-56` 是 `let expert_score = 1.0_f64;` 加一句 `"本地分析引擎（专家联盟待注入，降级默认健康分）"` ⇒ 联盟**根本没接进 KB 分析**，前端就算去接也只显一个常数。
    而且 `KbDocument`（`model.rs:57-71`）只落 `summary`，`keywords/chunks/expert_score/expert_steps/elapsed_ms` 只在 `POST /kb/documents/:id/analyze` 的响应里 transient 出现，GET 不再给 ⇒ 刷新即丢。
    真实的联盟评分在编排器治理面：`mox_ai_expert_svc::pipeline::mox_optimize` 出 `report.expert_scores`（14 维逐专家）与 `report.gate`，经 `/api/governance/{dashboard,experts/status,veto/events,assess}` 暴露——
    **这四个端点前端零消费者**（`grep -rn governance frontend-ui/src` 只有两条注释、`normalize.js` 的字段名与 `MoxFusionView.vue` 读另一个端点的 `report.governance`）。联盟可见性这一单元接的是它，不是 KB 的常数。
  - 是否"每次保存都建版"待裁决：现在的语义是"填了备注才归档改前正文"，与后端 `create-version` 一次调用一条快照一致；不填备注则改前正文仍然会丢。
  - 表单里 `description` / `auto_save` 两个控件没有 wire 落点（后端无此字段），本轮按"UI 保留、不发出去"处理，是否撤控件待裁决。
  - 面板仍有 5 份 `@/utils` 已有出口的私有副本（`escapeHtml`/`safeUrl`/`simpleMarkdownRender`/`truncateText`/`getTagSize`）与一批随对话框退役的死 CSS（`.link-dialog-content`/`.entity-search-results`/`.search-entity-item`/`.no-results`/`.sug-conf*`）。
  - 孤儿 composable（零 .vue 消费者）的 `fetchCategories`/`fetchTags` 仍是 `if (Array.isArray(data))` 的 if 形写法，禁令 #19 只管三元链，这两处不在本轮账上；退役裁决没下来之前不扩大改动。

## 5.13 联盟治理台前端模块：十条治理端点从"零消费者"变成有主（2026-09-27）

§5.11 末登记的那条余账——`/api/governance/{dashboard,experts/status,veto/events,assess}` 四个端点前端零消费者——本轮按整族收口：治理台十条端点全部接进新模块 `src/modules/governance/`（契约/模型/ store/视图四层齐），并把每条 Rust 侧事实**在测试里现读源码**，不留副本。

契约层钉住的 wire 事实（`governance-contract.test.js` 现场解析，25 例）：

- **前缀与路由表**：`main.rs` 的 `.nest("/api/governance", …)` 前缀不写死，测试从 `main.rs` 读出来再和 `routes/governance.rs` 的**相对**路径拼成十条行，与网关 `proxy.rs` 的 `/api/governance/*` 通配一起构成双向集合差判据（前端端点表 ⊖ Rust 路由表 = ∅，反向亦 ∅）。
- **信封**：`api_ok` 出扁平 `{code,msg,data}` ⇒ 走 `_kernel/envelope.js` 的 `unwrap({nesting:'flat'})`，不许再猜形状。
- **`serde(rename_all="camelCase")` 不作用于 `json!` 字面量**：`business_league / dev_league / average_health`、否决分页外壳的 `page_size / total_pages` 是蛇形，而结构体 `AuditLogDto` 侧是 `pageSize / totalPages` ⇒ 同一个模块内两套口径并存，normalize 分表处理，并有一枚变异体专测"把 `json!` 外壳读成 camelCase"（T2，红）。
- **时间**：治理面所有时间戳是 epoch **秒**（`unix_ts()` / `as_secs()`），而 `@/utils` 的 `timeValue` 按毫秒读数字 ⇒ `secsToMs` 在 normalize 里 ×1000；T1 撤掉这个乘法，四处时间字段全部落到 1970（红）。
- **查询参数**：`VetoQuery`/`AuditLogQuery` 是 camelCase 且全 `Option` ⇒ 蛇形参数会被服务端**静默忽略**（不报错），`vetoQuery()` 只发 camelCase，并夹 `page_size ≤ 200`（T3 不夹 ⇒ 红）。
- **十四维**：同一份清单在 Rust 里手抄三遍（`ExpertConfig::default` 两张权重表、`experts_status_handler` 两个数组、`trigger_governance` 两个数组），前端词表 `contract/dimensions.js` 是第四份，判据要求四者完全一致、分母 14（T5 改一个维名 ⇒ 红）。
- **否决阈值是个谎**：否决事件的生成条件用字面量 `score < 0.5`（critical 再判 0.3），**不读** `expert_config.thresholds.veto_threshold` ⇒ 界面文案照实写"调配置不改变否决生成"，并把 0.5 单独登记成 `VETO_EVENT_THRESHOLD_LITERAL`。
- **状态词表**：`GateResultDto.status = format!("{:?}", FlowStatus)` ⇒ 值是 `Draft/Review/Approved/Blocked/Deprecated` 五枚 Debug 串，从 `mox-ai-expert-core/src/govern/mod.rs` 现读对齐。

产出与验收（全部本轮实测）：模块 12 文件 / 1984 行；`governance-contract.test.js` 25 例绿；**6/6 变异体 KILLED 且逐字节还原**（驱动在仓库外 `%TEMP%\mox-gov-mut`，判决落 `out.json`，`restored:true` 六枚齐）；`check-frontend-module.py` ERROR=0；`vite build` rc=0；路由门禁 97 条 / 死链 0；当时全量 vitest 65 文件 / 936 例 rc=0。

顺带修掉的既有账（都不是新代码的锅，但挡在验收路上）：

- Rust 源是 CRLF，切片锚点 `'\n}'` 在 Windows 上取到 -1 ⇒ 权重表那格把八个数组当成两个来比；现统一走 `readSrc()` 归一后再切。
- 禁令表原样共享一张 `POS` 表 ⇒ 一条样例被别的禁令的正则接住，"每条禁令在每个文件都有正对照"是假绿；改成 `(ban, file)` 逐对配样例，外加投毒负对照。
- `page: 0` 走 falsy 分支被吞成缺省；`"system".to_string()` 这类字面量污染权重键扫描 ⇒ 两处各补一枚判据。
- 联盟侧既有守卫跟着**后端漂移**改了两处：`OptionalAuthUser` + `emit_audit(&state, &actor_from_opt_user(&user), …)` 把身份写进审计（**仍然没有角色鉴权**，全目录唯一 403 是对象态"专家 {} 已被禁用"，判据把这条枚举成 1 处并配理由串）；`registry.js` 的 UI 证据行号漂移（`experts_common.rs:584` → 实为 `585`）⇒ 新增 ANCHOR 逐条存在性判据，行号再漂当场红。

余账（只报不改）：

- `PUT` 类配置端点、`/assess`、`/ws` 三条无前端出口（治理台是只读面），是否补写路径待裁决。
- `GateResult.gates`（子门数组）被 DTO 丢掉，界面拿不到逐门结论；仪表盘的流程计数是前端派生量，不是后端字段。
- `docs/API-REGISTRY.md` 由网关 `actuator.rs` 的 ROUTES 生成，**没有"代理到编排器的路由"这一通道** ⇒ 十条治理端点在里面一条都不存在。补生成器还是补手工节，属文档门禁的账。
- `GovernanceConsoleView.vue` 只有 jsdom 机制见证与构建通过，**没有做浏览器真机目视**（十条端点的真实响应渲染）——按 §5.11 的同一口径记为未完成。

## 5.14 字符串图标名解析口：44+12 个名字在浏览器里一直渲染成空白（2026-09-27）

病灶：`vite.config.js:63` 挂的是 `unplugin-vue-components` + 自写 `epSubpathResolver`，它**只在编译期改写模板标签**（`<el-icon>`、`<ElIconAim>` → 从 `@element-plus/icons-vue` 取 `Aim`）。而模板里有两条通道传的是**字符串**，编译期没有任何人改写、运行期又没有登记项：

| 通道 | 形态 | 实测规模（本轮收口时现扫，口径含注释）|
|------|------|------|
| A 数据表 | `icon: 'Aim'` | 129 处 / 22 文件 / **44 个不同名** |
| B 内联三元 | `:is="open ? 'ArrowUp' : 'ArrowDown'"` | 32 处（15 个站点 × 引号名数）/ 12 文件 / **12 个不同名**，其中 9 个不在 A 的 44 里 |

两处计数都含本单元自身注释里出现的示例串（`nav-icons.js` 头注释、`main.js` 挂载点注释各带一枚 `icon:'Aim'`，`nav-icons.js` 注释另带一行 `:is="…"`）⇒ 它们是**修复前规模的下界**而不是精确值；判据按"≥127 / ≥28"钉下界，精确分母由测试现场重算。全部 `:is` 绑定共 55 处。

⇒ 这些图标位一律 `Failed to resolve component`、显示为空白，只在控制台留一条警告。`TheSidebar.vue` 的旧注释「main.js 已全局注册」是**假的**：`src/main.js` 里一条 `app.component` 都没有，同一函数的另一半 `isIconComponent()` 只把字符串放行给 `<component :is>`，于是空白成了常态。附带发现：该分支上的 `class="ep-icon"` 全库 0 处定义（死类名）。

修法（归一到一个出口，不逐处补丁）：新增 `src/modules/_kernel/nav-icons.js` 作 A∪B 的**封闭集（53 名）**，逐个具名 import（**不做 `import *`**：该包导出 293 个图标，整包注册会全量进首屏），由 `registerNavIcons(app)` 在 `main.js` 挂一次；侧栏改用 `navIcon(name)` 显式解析，未登记名返回 `null` ⇒ 走文本分支（旧配置的 emoji 仍然看得见字，比空白可诊断），并把 `<span class="ep-icon">` 换成 `<el-icon>` 以吃到 resolver 自动引入的 `icon/style/css`（图标 SVG 自身不带宽高属性，无这张表就是未定尺寸）。出口经 `src/modules/index.js` 暴露。

守卫：`src/modules/_kernel/nav-icons.test.js` 9 例，两条通道全库现扫（含注释——注释里写图标名也算使用位点，故 `nav-icons.js` 的头注释刻意不出现 `icon: 'X'` 原形），判据含"每个字符串名都在封闭集"、"封闭集无登记未使用名"、"分母下界 A≥127 / B≥28"（扫描器失明时先红，不是静默全绿）、"main.js 语句级挂载"、"别处不得散点 `app.component`"、"不得 `import *`"、"植入未登记名必须被抓到"的正对照，以及一枚**真 Vue 机制见证**（注册后 `resolveDynamicComponent('Aim')` 挂载渲染出 `<svg>`，未登记名渲染成未知元素）。

牙齿实测：**6/6 变异体 KILLED、逐字节还原**（驱动在仓库外 `%TEMP%\mox-icon-mut`，判决 `out.json`）。其中第一版判据被一枚变异体打回原形——把 `registerNavIcons(app)` 改成 `// registerNavIcons(app)` 后格 2 仍绿（`/registerNavIcons\(\s*app\s*\)/` 分不清语句与注释）⇒ 改成语句锚点 `/^\s*registerNavIcons\(\s*app\s*\)\s*$/m` 后 M1 才红。这条按 §3 禁令族的口径记为"判据本身被变异体检过"。

尺寸账（口径先说清，别和 §6 直接相减）：本轮 `vite build` rc=0 / 45.00 s；`vendor-element` vite 日志口径 665.17 kB、`stat` 口径 650.6 KB raw / 206.6 KB gz；图标 SVG 全落在这一个块（`'1024 1024'` 在该块出现 109 次，其余块 0 次）。与 §6 表（09-26 的 659.3 KB，同为 vite 日志口径）差 +5.9 KB，**只作上界看**：两次读数之间另有并行改动，未做同轮 A/B。

验收命令与结果：`check-frontend-module.py` ERROR=0、`check-framework-imports.py` PASS、`check-ep-feedback-imports.py --check` PASS、`check-route-links.mjs` 97 条 / 死 0、全量 vitest **66 文件 / 945 例 rc=0**（上表 65/936 即被本单元改写成新分母）。

余账（只报不改）：

- `AdminMenu.vue:38,46` 与 `AdminRole.vue:184` 的 `:is="row.icon"` / `:is="data.icon"` 取的是**后端/DB 里的名字**，封闭集对它没有约束力 ⇒ 要么登记全量图标（付 293 个导出的首屏代价），要么把这些字段接进登记口并显式回落，属产品裁决。
- 其余 `:is` 绑定（55 处总数减去 15 处内联字符串站点 = 40 处表达式绑定）本轮**未逐个走查**取值形态：现在被全局注册兜住了字符串名，但个别 composable/panel 里 `icon` 早已是组件对象，两种形态混用仍在；真机逐页目视也没做，按 §5.11 口径记为未完成。
- `constants/nav.config.js` 与 `src/modules/*/index.js` 两套导航配置并存（18 处 vs 各模块登记），图标名口径已统一，但**配置源本身**还没收成一个，属布局/外壳方案（[FRONTEND-LAYOUT-REFACTOR-PLAN-v1.0.md](./FRONTEND-LAYOUT-REFACTOR-PLAN-v1.0.md)）的账。

## 5.15 真机验证撞出的三件"测试全绿而页面不可用"（2026-09-27）

§5.13/§5.14 把"浏览器真机"记为未做的部分，本轮补做。链路前置是运维事实而非缺陷：编排器只认 `OUS_API_TOKEN`
（`mox-platform-orchestrator-svc/src/main.rs:745-760`，前缀推断的兼容模式默认关闭），网关反代时**剥掉客户端 JWT、注入服务令牌**
（`platform/gateway/mox-platform-gateway-svc/src/proxy.rs:146/415`），启动脚本默认令牌写在 `scripts/startup/start-mox-enterprise.ps1:18`。
手工起的网关若不带这个环境变量，`/api/governance/*` 一律 401（实测），带上了同一条链 200。

**① 账号口令登录整体不可用**：`views/auth/Login.vue` 把具名函数 `login` 按命名空间别名导入（`login as authApi`）却又调 `authApi.login(…)`
⇒ 每次点"登录"必抛 `TypeError: authApi.login is not a function`（浏览器 console 原文，指向 `Login.vue:69` 的 handleLogin）。
修法：`login as apiLogin` + 直接调用。普查工具（别名自调用扫描器，337 个源文件）：**全库只有这一处**；
工具牙齿用 HEAD 那份 `Login.vue` 做正对照（同一规则在修前那份上命中 `login/authApi @214` 行）。
修后真机闭环：注册→登录→`POST /api/auth/login` 200→389 字符 JWT 落 `mox_access_token`→跳 `#/dashboard`→`登录成功`。

**② 治理台十条端点全 404（`/api/api/governance/…`）**：`contract/endpoints.js` 的 `path` 是**契约身份**（要与 Rust `nest("/api/governance")` 逐字比对），
而 `api/http.js:12` 的实例 `baseURL` 已经是 `/api`；联盟模块的 `requestPath` 有 `.replace(/^\/api/, '')`（`expert-alliance/contract/endpoints.js:208`），
治理模块抄了形状漏掉这一行。**25 例全绿为什么没看见**：api 层用 stub，且既有断言写成 `expect(calls[0].url).toBe(requestPath('assess'))`
—— 期望值就是产生这个值的那个函数调用，恒真（自证断言）。修法：`requestPath` 补剥前缀，并把那格改成从两份权威源现场推
（Rust `main.rs` 的 nest 前缀 + `http.js` 的 baseURL 字面量），新增"格 · 发给 http 的 url 必须剥掉 baseURL"，
同时钉住 api 层 7 个出口方法的数量当分母（防止判据在空集合上成立）。变异体（删掉 `.replace(/^\/api/, '')`）⇒ 该格红并点名
`/api/governance/dashboard 把 baseURL 又拼了一遍`，还原后与修前字节一致（`cmp` 通过）。

**③ 图标登记口漏了第三种位点（通道 C）**：真机十条路由的 console 里实测 20 条 `Failed to resolve component`（`Refresh`/`MoreFilled`/`View`/`Delete` 等）。
原因：resolver 只改写 `ElIconXxx` / `ElFoo` 前缀，模板里**裸写**的 `<Refresh />` 既不会被自动导入、那些文件也没有本地 import。
静态普查（128 个 `.vue`，判定集取自 `@element-plus/icons-vue/dist/index.js` 的 `export { … }` 块，293 名）：**38 处 / 23 个不同名**，
多数集中在 `views/admin/panels/AdminMonitor.vue`。封闭集 53 ⇒ **63**（补 `Check/Close/Delete/EditPen/Lock/MoreFilled/Promotion/Refresh/Upload/View`）。
守卫同步扩：格 1 覆盖三通道、格 3 加 `C 站点 ≥ 38 / C 不同名 ≥ 23` 下限、格 4 加通道 C 正对照
（从包里挑一个"有导出但未登记"的真实图标名当靶，随封闭集增删自动换目标，不写死）。
变异体（删 `Upload` 两行）⇒ 格 1 红并点名 `Upload`，还原字节一致。
**本通道的第一版普查工具是假阴的**：它把判定集读自 `dist/types/index.d.ts`，而那个文件只有一行 `export * from './components';`
⇒ 名集为空 ⇒ 每个标签都被过滤掉 ⇒ 打印"0 处命中"看着像好消息。修后判定集解析加 `size > 200` 硬断言（空集直接抛，不再假绿）。

修后真机复验（同一构建，`npx vite` dev :3020 → 网关 :3080 → 编排器 :3001）：
- 治理台读的是真后端：`读数时间 刚刚`、业务/开发璇玑均分各 `1.000`、阈值 `0.3 / 0.6` 标注为"读自"配置、审计链"校验通过"，八节面板全部渲染；
- 侧栏 11/11 个图标位点各有真 `<svg viewBox="0 0 1024 1024">`，computed 16×16 px（`cellBox` 20×16），文本回落 0 个；
- `/workflow`（`.more` 位点，通道 C 的裸标签）修后 `svg` 存在；三次真实导航（`/workflow`、`/operators`、以及治理台）解析告警 **0 条**。
  注意口径：`/ai/bots` 在 `tenant_user` 下被弹回 `#/dashboard`，所以通道 C 只真机验到一处，其余靠静态普查 + 守卫。

回归账：全量 `npx vitest run` **66 文件 / 946 例 / rc=0**；`python scripts/gate/check-frontend-module.py` ERROR=0（WARN 均为存量）；
`check-ep-feedback-imports.py --check` verdict=PASS（探针，债务不改 rc）。

余账（只报不改）：`AdminMenu.vue:38,46`/`AdminRole.vue:184` 的 DB 驱动图标名仍未裁决（登记全 293 vs 走登记表显式回落）；
`:is` 绑定里 40 处非字面量取值形态未逐条走；admin 角色下的通道 C 站点未真机看过；`views/auth/Login.vue` 的 ① 型缺陷没有落库的常驻门禁
（普查工具在仓库外的临时目录，未接 CI），下一次同类改动的验收口径应是"点一次登录能拿到 token"这一条真机事实。

## 5.16 §5.15 缺陷 ① 收成落库常驻门禁：`check-api-binding-kinds.py`（2026-09-27）

§5.15 余账最后一条是"缺陷 ① 没有落库的常驻门禁（普查工具在仓库外的临时目录，未接 CI）"。本节把它补成仓库内的第四把 import 形状门禁。

**新脚本**：`frontend-ui/scripts/gate/check-api-binding-kinds.py`（只读，复用 `check-framework-imports.py` 的注释/字符串/正则字面量遮蔽与 `.vue` 的 `script_mask`——遮蔽是承重结构，不重抄一份）。
判据：具名导入的绑定若在 `src/api/**` 里解析为函数（`export function`／`export const X = (…) => …`／别名右值），则 `alias.member(…)` 就是运行时 `TypeError` ⇒ 零容忍，命中即 rc=1。
与 `check-framework-imports.py` 同族：这一类缺陷首屏即崩，没有"已知债务"可容纳，所以按不变量钉而非按棘轮钉。

实测三命令（本轮，非回忆）：

| 命令 | 判决 | 关键分母 |
| --- | --- | --- |
| `--check` | verdict=PASS，0 处 | 23 个 api 模块 / 462 具名导出 / 459 函数型；336 文件被扫 / 327 处 `src/api` 具名导入 / 其中函数型 322 处 |
| `--selftest` | PASS | 17 枚合成针 + 5 枚导出表事实 + 3 枚真实文件对照，全绿 |
| `--mutants` | PASS | 5 枚判据变异体各自打红并点被自己害到的那格名 |

三条豁免各按名打印站点，且各有"豁免得住"＋"不许吞掉真缺陷"两枚针：E-shadow 3 处（`stores/auth.store.js` 把 `login`/`logout`/`refreshToken` 局部重声明为 ref）、E-mock 2 处（`auth.store.test.js` 查 `registerAuthTokenGetter.mock.calls`）、E-fnprop 0 处（`call/apply/bind/length/name/prototype` 是 Function 合法属性）。
真实文件对照里第一枚就是缺陷 ① 本体：把 `views/auth/Login.vue` 在内存里改回 HEAD 的 `login as authApi` + `authApi.login({` ⇒ 必须报 1 处（原样 0 处）——门禁的牙直接钉在历史事故上，不是钉在合成样例上。

**写这台门禁自己撞出的五处"看着绿其实没通电"**（全部由针当场拦下，逐条留档）：
1. 用 `bindings()` 的绑定集当 E-shadow ⇒ 导入绑定本身就在集里 ⇒ **每个导入都自我豁免**，判集恒空；16 枚针里 9 枚同时红才暴露。E-shadow 只能看局部声明/形参/解构。
2. 借 `_fi.split_specs(clause, alias_wins=False)` 解析导入 ⇒ 它只回原名（orig），**别名整段丢失**，而缺陷 ① 恰恰是别名形状。自写 `parse_specs` 返回 `(orig, bound)` 两列（M4＝把站点查询退回 orig，打红 2 格）。
3. 桶链从**遮蔽后**文本读 ⇒ `export * from './auth.api'` 的说明符在字符串里被抹成空格 ⇒ 导出表塌缩成"谁都不认识"，于是全部判 clean 而照样 `verdict=PASS`。这比第 1 条更危险：它不会让任何现有测试变红。修＝re-export 从原文读，并加两道分母守卫：模块装载时 `len(KINDS) >= 200 and CALLABLE_N >= 20` 否则 `INVALID` 直接退出；`--check` 判集函数型导入 `< 200` 也判 `INVALID`（把"0 命中"和"没东西可判"分开）。另加 5 枚 `MAP_FACTS` 表事实（`login` 函数型／`listArtifacts` 别名继承／`http` 对象型／`ROLE_TEMPLATES` 对象型）。
4. 变异体锚点连同 `MUTANTS` 表一起数 ⇒ 表里就抄着锚点原文，永远得到 2 次（仪器看不见自己要测的结果）。修＝`_code_only()` 先把表区间剪掉。
5. `.vue` 的裸片段针被 `script_mask` 整段抹空 ⇒ 空转针（与 `assertions-need-mutants` 第 31 条同形）。修＝该针自带 `<script setup>` 标签。

两处静默少判的形状也一并修掉：`export const listArtifacts = getArtifacts`（别名右值要继承种类，否则 6 个函数被判对象型）与 `export default http`（右值是文件内**未导出**的 const，只扫 `export` 行就解不出种类 ⇒ `http` 成 UNKNOWN ⇒ 对它的一切判定被跳过）。

**未接 CI 的诚实说明**：`scripts/gate/check-all.ps1` 仍列 7 项，`check-framework-imports.py`／`check-ep-feedback-imports.py`／`check-theme-tokens.py`／本脚本这四把都不在那七项里——四把同命运，靠本节口径手工执行。把它们并入聚合器要改 `[N/7]` 全部编号与 AGENTS.md 的"7 项"表述，属跨门禁决策，留待裁决，不在本轮擅自做。

回归口径：本轮只新增 `frontend-ui/scripts/gate/` 下一个文件与本段文档，`src/` 零改动 ⇒ 不重算 138 枚 collab 电池的账，也不重跑全量 vitest（分母仍是 §5.15 的 66 文件 / 946 例 / rc=0）。新增段落里的反引号路径经 `check-doc-links.py` 复扫，未新增死链。

## 5.17 misc 族分页读路径归一：两条幻影端点、四处猜键，与"两族排序表不通用"（2026-09-27）

本节所有读数（8 枚 `curl` 出参壳、`vite build` 尾行、全量 vitest 分母、四把门禁判决行、变异电池逐枚红格与还原 sha）由 `reports/markdown/misc-page-wire-20260927.md` 现场打印，该文件未改动输出内容；下表只复述，不另算。

**事实面（本轮实测，`GET :3080` 带客户端 JWT）**

| 发的查询串 | 回显／行为 | 结论 |
|---|---|---|
| `page=2&page_size=5&sort_by=title` | `page:2, page_size:5, sort_by:"title", has_prev:true` | snake_case 生效 |
| `pageSize=5&sortBy=title` | `page_size:20, sort_by:"created_at"`，**HTTP 200 无告警** | camelCase 被**静默忽略** ⇒ 猜大小写没有报错，只有筛错 |
| `page_size=9999` | `page_size:100` | 上限来自 `misc.rs` 的 `clamp(1, 100)` |
| `sort_by=zzz` | `filters.sort_by:"zzz"`，排序回落 `created_at` | 白名单只在前端存在 |
| `GET /api/tasks/paginated` | **404** | 前端一直在叫一条不存在的路由 |
| `GET /api/projects/paginated` | **502** `UPSTREAM_UNREACHABLE → 127.0.0.1:8000` | 落到 PrimiFlow 前缀代理（:8000 未起） |
| `GET /api/projects` | 分页壳（:8000 停机时仍 200） | 精确路径归网关 misc.rs，与 `/api/projects/*` 不同上游 |

**四台消费者的缺陷（本轮收口 ①–⑤）**

1. `ProjectsView.loadTasks`：猜 `data / data.list / data.data / data.tasks` 四个列表键——**没有一个是真名 `items`** ⇒ 后端有行也恒空（§5.11 登记的 A 族命中，本轮落地修）。
2. `ProjectsView.loadAll` 与 `refreshList`：`projects.value = ps || []`／`= ps` ⇒ 把整个**分页壳对象**当数组塞进响应式变量。`v-for` 会遍历壳的 8 个字段值，而 `filteredProjects` 一旦带筛选条件就在对象上调 `.filter` ⇒ TypeError。
3. `TaskView.loadTasks` 客户端分支：只认 `Array.isArray(data) && data.length > 0` ⇒ 同上恒空。
4. 两视图的服务端分支：叫上面那两条幻影路由，且都躲在 `useServerPagination` 后面，而该 ref **全仓从未被置 true** ⇒ 分页行 `v-if="useServerPagination && total > 0"` 是够不到的 UI。
5. 不发 `page_size` ⇒ 后端默认 20：ProjectsView 用 `tasks` 全量做状态计数、TaskView 用 `filteredTasks` 做本地筛选排序 ⇒ **静默截断**。现改为取满 wire 上限（100），并在 `total > items.length` 时显式提示"部分口径"。

**收口形态**：新增单一口径 `frontend-ui/src/modules/_kernel/paged-list.js`——`pageQuery()`（camelCase 视图状态 → wire snake_case，空值不发、非法排序不发、越界夹到 clamp）、`normPage()`（分页壳 → camelCase 视图模型；裸数组是**唯一**允许的降级并打 `degraded`），以及 `WIRE_PAGE_KEYS / WIRE_QUERY_KEYS / PAGE_SIZE_MAX / PAGE_SIZE_DEFAULT / TASK_SORTABLE / PROJECT_SORTABLE`。

写它时我自己踩的坑值得记：第一版只有一张 `SORTABLE`（＝tasks 表 + `created_at`），而 `misc.rs` 两族的 `match sort_by` 臂**不通用**——tasks 可 `title/priority/due_date`，projects 可 `name/member_count/task_count` ⇒ 项目侧的合法排序会被前端静默吞掉。修＝两族各一张表，并由单测格 1 钉"两族各自的独有臂不许为空"（合并成一张表时先红）。

**守卫**：`frontend-ui/src/modules/_kernel/paged-list.test.js`（9 格）。期望值**现场解析** `platform/gateway/mox-platform-gateway-svc/src/misc.rs`：`struct PaginationQuery` 字段名（≥6）、`ok(json!({…}))` 顶层键（≥8，两族须同键集）、两族 `match sort_by` 显式臂（各 ≥5）与 `_ =>` 回落字段、`clamp(1, N)`、`TaskItem` 字段表（≥8）；模块顶层就断言判集非空，解析器瞎了不会印"0 命中"。格 4 把被废掉的四个猜键逐个喂回去，**必须仍然恒空**——否则本单元判据失去牙齿。格 8 是消费者台账：按花括号配平切出 `async function` 体，凡 body 里调 `getTasks(`/`getProjects(` 的函数都必须出现 `normPage(`（实测 5 个：`ProjectsView:loadAll/loadProjectsPaginated/loadTasks/refreshList` + `TaskView:loadTasks`，分母下界 5），并 src 级扫描 `["'\`]/(tasks|projects)/paginated` 与两个已删函数名（扫描集 ≥200，禁令含注释，与 `contract/forbidden-revival` 同口径）。

**变异电池（10 枚全打红，逐枚红格）**：M1 列表键换成 `list` → 格 1｜M2 tasks 排序表混入 `name` → 格 2,5｜M3 上限写 500 → 格 2｜M4 `normPage` 给猜键开兜底 → 格 4｜M5 查询键退回 camelCase → 格 2,5｜M6 ProjectsView 撤归一化（还原猜键链）→ 格 8｜M7 幻影端点复活 → 格 8｜M8 事实源路径改错 → 收集期崩（`misc.rs` 读不到）｜M9 TaskView 就地取 `.items` 绕过 `normPage` → 格 8｜M10 `refreshList` 撤归一化 → 格 8。跑完 5 个文件 sha256 与写前逐字节相同（10 行 `[RED]`、基线 rc=0、5 行 `[OK]` 还原 sha 与 `PAGED-LIST MUTANTS: PASS` 见读数文件 E 节，该节为第三次重跑，前两次的日志留在仓库外 `bak.run1/`、`bak.run2/`）。驱动与备份在仓库外 `%TEMP%\mox-paged-mut\`——**没落库⇒没牙，别接 CI**。（第一轮 8 枚时 M6 曾**打不红**：那格原本只要求"文件里存在一处 `normPage(`"，而同一文件另一函数带着它 ⇒ 台账从"每文件"改成"每函数"才真有牙齿，这是第 32 条"覆盖面按形状算"的又一次显形。）

**验收**

| 命令 | 结果 |
|---|---|
| `npx vite build` | ✓ built in 37.09 s（读数文件 B 节原文；本轮另有一次 34.7 s，两次只差墙钟，未做同轮 A/B。改导出名的验收口径是 rollup：删掉两个 api 导出后消费者若有漏网会在这里崩，不会在 vitest 里） |
| `npx vitest run`（全量） | 67 文件 / 955 例 / rc=0（分母较 §5.15 的 66/946 ＝ +1 文件 +9 例） |
| `check-api-binding-kinds.py --check` | PASS（函数导出被当对象用 0 处） |
| `check-framework-imports.py --check` | PASS（非测试源文件 0 处用而未绑） |
| `check-ep-feedback-imports.py --check` | 探针 PASS（债务不改 rc） |
| `check-frontend-module.py` | ERROR=0 |
| 真机浏览器（dev :3020，代理注入服务令牌） | `GET /api/tasks?page_size=100` → **200**；`GET /api/projects?page_size=100` → **200**；对 `…/paginated` 的请求**不再出现**；`/api/projects/{types,catalog,stats}` → 502（见余账，未修） |

**未验证维度（诚实登记）**：misc 存储当前 0 行，而 `misc.rs:96` 自陈"无写入即无可丢失，而是功能未接线"⇒ 没有不侵入运行态的 seeding 办法，所以"有行时页面真渲染出行"只有格 3 的逐键归一见证，**没有实机行级见证**。补此证据需先裁决是否向 `target/` 本地运行态存储写样例数据（AGENTS.md：`target/` 含 mox.db 等本地服务数据，不许动）。

**只报不改（本轮余账）**
- 同一资源族两个上游：`/api/projects`（精确）归网关 misc，而 `/api/projects/*` 全量代理 PrimiFlow :8000 ⇒ `getProject(id)`、成员、文档、动态流在 :8000 停机时一律 502。路由前缀归属需一次裁决。
- misc 族**只有 GET 两条路由**：`projects.api.js` 里的 `getTask/createTask/updateTask/deleteTask/convertChatToTask/convertTaskToChat/executeTask/autoCreateTask` 在网关上无对应注册（POST 会被 `/api` catch-all 转给编排器 :3001）⇒ 写与读不同源，写入的行永远不会出现在这个列表里。
- `useServerPagination` 从未置 true ⇒ 两视图的分页行是够不到的 UI。本轮把两条分支都接到了真路由上，但"何时切服务端分页"的阈值属产品决策，未擅自设。
- `TaskView` 状态词表 `todo/in_progress/done/cancelled` 与网关 `TaskItem.status` 词表是否一致，0 行状态下无从验证；一旦启用服务端分页，`status` 走后端精确匹配 ⇒ 词表不一致就是空列表。与 §5.11 的"两套词表零重叠"同族，登记待验。



## 5.18 前端 api 字面路径的幻影普查：202 条静态路径里 156 条无任何路由，与 Melody 域的双重错位（2026-09-27）

> **⚠️ 本节的 `156` 与 `94` 两个数由 §5.19 作废**（2026-09-28 晨复测）。首轮普查的"探测 URL"和"打印路径"取自两个不同来源：探测用源里的字面量（`/ai/flows` 这类不含 `baseURL` 前缀的写法），打印用补了 `/api` 之后的 wire 路径 ⇒ 156 里 **104 条是仪器假账**，真值是 **52 条无路由、其中 21 条有活调用者（30 个调用点）**。本节原文保留，因为它登记的是尺子缺陷本身；**任何引用一律改引 §5.19**。


本节全部读数（控制组 4 次现场 curl、首轮 202 条的分桶、64 条限流复测的逐条判决、第二把尺子的负结果、Melody 案例的 grep 行号）由 `reports/markdown/endpoint-phantom-census-20260927.md` 现场打印；驱动与原件在仓库外 `%TEMP%\mox-endpoint-census\`——**没落库⇒没牙，别接 CI**。

**口径（先说清，否则数字会被读成"77% 的前端接口是坏的"）**：普查对象只有 `src/api/*.js` 里**无插值**的字面路径（202 条）；含插值的 141 条本轮**没测**，因为插值路径打过去拿到的 404 是"资源没有"，与"路由没有"混成一类。判据＝一律 GET（源里写 `http.post` 的路径若真存在，用 POST 打就是在写状态），`404 且响应体为空`记为"网关及其反代之后没有任何路由"。

| 判决 | 条数 | 含义 |
|---|---:|---|
| `404 空体` | **156** | 幻影：该前缀/路径无人服务 |
| `200` | 30 | 真路由且当前就有数据面 |
| `405` | 11 | 路径在、方法不是 GET（正常） |
| `502` | 3 | 路径在、上游未启动（`:8012` melody2score 那族） |
| `403` | 2 | 路径在、鉴权拒绝 |

幻影按文件：`ai.api.js` 45｜`experts.api.js` 28｜`workflow.api.js` 19｜`graph.api.js` 18｜`llm.api.js` 10｜`melody.api.js` 8｜`alliance.api.js` 7｜`kb.api.js` 7｜`projects.api.js` 5｜其余 9 文件各 1–3 条。

**仪器通电证明**（读数文件 A 节）：`/api/tasks?page_size=1`→200、`/api/kb/documents`→200、`/api/zzz-not-a-route-xyz`→404 空体、`/melody/v1/health`→502 信封；且 PrimiFlow `:8000` 与 melody2score `:8012` 直连都是 `000`（连接被拒）⇒ **本轮任何 `404 空体` 都不可能来自兜底反代分支**（那条只会报 502），这一条是 156 这个数的成立条件。副产物：网关在连续 ~138 次请求后开始回 `429`（首轮尾部 64 条全体被判限流），复测按 1.2 s 间隔后 0 条仍被限流——**任何把本普查落成常驻门禁的版本都必须带节流**，否则报出来的"幻影"是限流伪影。

**负结果（要点名，它约束了门禁的形状）**：想只读 `actuator.rs` 的 `r("name","METHOD","/path")` 目录行离线判定首段是否存在，结果该目录登记的是 `/voice/v1`·`/melody/v1`·`/kg/…` 这类根前缀域，对 `/api/*` 面基本是盲的——差集里连 `/tasks`·`/projects` 都"查无此段"，而它们在 A 节实测 200。⇒ **将来的路由真值只能取现场判决或 Rust 挂载代码（`modules.rs` 的 `.merge(...)`＋`deployment.rs` 的 `nest`），不许拿 actuator 目录或 `docs/API-REGISTRY.md` 当真值**（后者已由 [[project-alliance-route-authority]] 记过一次盲）。

**案例：Melody 域双重错位（源码级，行号见读数文件 E 节）**——① `src/api/melody.api.js` 的 8 个端点全部打在 `/melody2score/*`（经 baseURL 成 `/api/melody2score/*`），而 Melody 域实际挂在**根前缀** `/melody/v1/*`（`melody.rs:135` 造路由、`modules.rs:149` 挂载），前缀不是"少写一段"而是两套命名空间；② `vite.config.js` 的代理键只有 `/ws`·`/api`·`/actuator`·`/kg`·`/alliance`（全文 `melody` 命中 0 次）⇒ 即便把前缀改对，dev 侧仍然没有出口，**改前缀必须连代理键一起改**；③ 该页**不是孤儿**：`modules/ai/index.js:48` 注册了它 ⇒ 这是一条"能点进去、每个请求都 404"的活路径，属 §5.15"测试全绿而页面不可用"同族，且比 KB 那族更隐蔽（KB 的幻影至少被兜底反代接成 502）。

**交叉账（读数文件 F 节，驱动 `cross-ledger.py`）**：把 156 个幻影调用点挂回"导出名 → 真实消费者"，判决＝**94 处有真实消费者（活代码打到 404，真故障）／61 处零消费者（api 死函数，可退役候选）／2 处只撞名而拿不到导出／1 处解不出导出名**（`http.test.js:9` 的 `/api/auth/me`，测试文件，不算任何一类）。分母与三枚正对照（`getTasks`／`getProjects`／`getAlgorithmTypes` 必须被判"有消费者"）一并印在 F 节，判"零消费者"的口径因此是有牙的。

**这一节最贵的一条不是数字，是尺子的形状**：只认"按名 `import { X } from '@/api'`"时真故障是 65 处；补上第二种形（`import * as X from '@/api'` ＋ `X.<名>` 成员取值，实测 7 个文件用这一形）后跳到 **94 处**——**同一份语料、同一个判据语义，换一种写法就差 29 处**。这与 §5.15 缺陷 ①、[[project-frontend-reachability-ledger]] 的"提取器少一种边形态造假日孤儿"同族：**任何消费者计数必须先枚举"拿到导出的全部写法"再报数**，否则"死函数"里会混进活路径。（残留盲区：动态取值 `api[name]`、字符串拼名，本轮未量化。）

真故障按 api 文件：`workflow.api.js` 17／`ai.api.js` 17／`experts.api.js` 16／`graph.api.js` 11／`llm.api.js` 8／`kb.api.js` 7／`melody.api.js` 6／其余 12。调用者最集中的出口＝`AdminDocs.vue` 22 处、`GraphView.vue` 10、`InfiniteOptimizerView.vue` 8、`AdminLlm.vue` 8。两点必须点名：① `kb.api.js` 有 7 条真故障而 KB 主页可用——坏的是边角函数（`kbGetStats`/`kbSearchEntities`/`kbBatchAnalyze` 等），再次印证"页面能开≠该 api 模块可用"；② Melody 那 8 条幻影里 **6 条有真实调用者**（另 2 条 `melodyStatus`/`melodyRecognizeRecord` 无人调），所以 E 节的双重错位不是死码问题而是活路径问题。

**引用纪律**：**156 不得被引用成"156 个接口故障"**——其中 61 条根本无人调用（退役候选），94 条才是故障；而 94 里有多少能靠"改前缀"救回、多少要后端补路由，需逐族对着 `modules.rs`/`deployment.rs` 挂载代码判（task #16 的下一步）。

**只报不改**：melody 的两文件修法（新建 `createHttpInstance('/melody/v1')` 实例＋`vite.config.js` 加 `/melody` 代理键并复用 `mkConfigure` 注入令牌）本轮不落——上游 `:8012` 未启动，改完最多看到 502 信封，拿不到成功形状，等于用不可验证的改动换"看起来修好了"。等用户点名要不要（a）启 `:8012` 让改动能出真形状，或（b）连 `ai/experts/workflow` 三族一起按普查收口。

## 5.19 幻影普查的第二次独立测量与三口径结案：真值是 52 条无路由（21 条有活调用者），族判决与 Melody 五条可改前缀（2026-09-28）

本节全部读数（`rejudge.out` 的分桶与逐条 404 清单、`rejudge.py` 预检/后检对照行、`catalog.out` 的目录装配力对照、`final52.out` 的三口径逐条账、`melody.txt` 的双向对账）由 `reports/markdown/endpoint-phantom-census-20260927.md` 的 **G/H 两节**现场打印；四把驱动（`rejudge.py`·`route-catalog.py`·`final-52.py`·`append-gh.py`）在仓库外 `%TEMP%\mox-endpoint-census\`——**没落库⇒没牙，别接 CI**。

### 5.19.1 假账怎么来的：显示路径 ≠ 探测路径

`src/api/http.js:11-12` 是 `axios.create({ baseURL: '/api' })`，而 `src/api/*.js` 的**主流写法是前缀-free 字面量**（`http.get('/ai/flows')`）。首轮驱动 `census.py` 的探测式是 `'http://127.0.0.1:3080' + p`，p＝**源字面量**；末段台账打印式是 `'/api%s' % p`，即补过前缀的 wire 路径。于是同一行里"路径"与"码"来自两个不同地址：凡前缀-free 的字面量都被打到 `/ai/flows` 这种根本不存在的位置上，判成 404，再顶着 `/api/ai/flows` 的名字印进台账。202 条静态字面量里 156 条落进"幻影"桶，恰好就是"前缀-free 的那一批"——46 条非 404 反而是源里已经写了 `/api` 的那批（探测式对它们恰好正确）。

第二次测量（`rejudge.py`）只改一件事：**探测 URL 就是台账里印出来的那条路径**。逐条 156 条，一律 GET，1.3 s 间隔（首轮网关在 ~138 次连续请求后开始 429），预检与后检各跑同一对对照（`/api/tasks?page_size=1` 必 200、`/api/zzz-no-such-endpoint-x` 必 404 空体），两端一致才认本轮全体读数。

| 第二次测量分桶 | 条数 | 含义 |
|---|---:|---|
| `404 空体`（两轮一致） | **52** | 真幻影：网关与其反代之后没有任何路由 |
| `200` | 52 | 路由在且有数据面 ⇒ 首轮假账 |
| `405` | 48 | 路由在、方法不是 GET（本轮不许为拿 200 去写状态）⇒ 首轮假账 |
| `400`／`500`／`502` | 2／1／1 | 均被某宿主接住 ⇒ 首轮假账 |

**修正后的口径**：202 条静态字面量＝150 条有路由＋**52 条无路由**。§5.18 的 156／94 两个数从此作废。附带收获：翻转清单里 `405` 那 48 条顺带证明了"源里写 `http.post` 的路由确实存在"，这是首轮拿不到的正面证据。

### 5.19.2 三口径的结案账（每一径各配正对照）

- **口径一 live**：上面那 52 条，两轮独立时刻同判。
- **口径二 static**：`route-catalog.py` 从 Rust 源装配路由目录（axum 0.7 语义：`nest` 给子路由加前缀、`merge` 不加，因源里多写绝对路径；子路由经 `.nest(pre, Router::new()…)`／内联 `{..}` 块／builder 函数名／`let` 变量四种写法解析），只从入口文件（网关 `modules.rs`·`deployment.rs`·`lib.rs`·`main.rs`、编排器 `main.rs`、各 svc `main.rs`）展开 ⇒ **377 条唯一路由**（gateway 255＋orchestrator 122）。装配力对照：绝对 3/3、前缀 12/13、反向误收 0；**残留盲区 `/api/enterprise/admin/` 仍未装配出来**，故 ABSENT 对该族偏严，逐条复核要点名。第一版还自伤过一枚：把 `Router::new().nest(...)` 的容器取成 `new()` 自己的括号区，5 个 nest 前缀（`/melody/v1`·`/voice/v1`·`/cloud/v1`·`/api/kb`·`/api/alliance/sediment`）整批丢失、目录只有 360 条且前缀对照 9/13——**目录的"看不见"会伪装成后端的"没有"**，这与 §5.18 的 actuator 负结果同族，只是这次瞎的是我自己造的尺子。
- **口径三 消费者**：沿用 §5.18 的"按名 import ＋ `import * as X` 成员取值"双形识别（正对照 `getTasks`／`getProjects` 必须判"有消费者"，实测 True）。

52 条在目录里**全部判 ABSENT**（无一条能被静态目录解释成"其实有路由"），按消费者分＝**21 条有活调用者（真故障，合计 30 个调用点）／31 条零调用者（api 死函数，退役候选）**。

| 族（首两段） | 条数 | 活调用点 | 判决 |
|---|---:|---:|---|
| `/api/ai/infinite-optimize/*` | 8 | 8 | 目录对 `infinite` 零命中 ⇒ 后端补路由，或 `InfiniteOptimizerView.vue` 整页退役（该页每个请求都 404） |
| `/api/melody2score/*` | 8 | 12 | **5 条改前缀可救**（见 5.19.3） |
| `/api/llm/*`（`logs`·`stats`·`usage`·`providers/active`） | 4 | 6 | 目录只有 `/api/llm/providers`·`/providers/presets`·`/health`·`/routing` 四条只读 ⇒ 后端补 |
| `/api/web-search/*`（`config`·`test`） | 2 | 2 | 目录零命中 ⇒ 后端补或面板退役 |
| `/api/ai/project-graph` | 1 | 2 | 目录零命中（`ProjectsView.vue` 调）⇒ 后端补 |
| 其余 20 族单/双点（`ai/artifact/*`·`ai/full-*`·`ai/generate-*`·`caomei/ai-parse`·`algolab/ai-analyze`·`tasks/auto`…） | 29 | 0 | 零调用者 ⇒ 退役候选，删文件需用户点名 |

### 5.19.3 Melody 族逐条判决（目录 7 条 vs 前端 8 条，读数见 G/H 节 `melody.txt`）

| 前端字面量（wire） | 网关 `/melody/v1` 孪生条 | 判决 |
|---|---|---|
| `/api/melody2score/health` | `GET /melody/v1/health` | 改前缀可救 |
| `/api/melody2score/recognize` | `POST /melody/v1/recognize` | 改前缀可救 |
| `/api/melody2score/recognize-sample` | `POST /melody/v1/recognize-sample` | 改前缀可救 |
| `/api/melody2score/export-sheet` | `POST /melody/v1/export-sheet` | 改前缀可救 |
| `/api/melody2score/samples` | 无（孪生在隔壁域 `GET /voice/v1/samples`） | 指过去或后端补 |
| `/api/melody2score/save-report` | 语义相近的 `POST /melody/v1/save-md` | 改名可救，但须先确认语义等价 |
| `/api/melody2score/status` | 无 | 后端补 |
| `/api/melody2score/recognize-record` | `POST /melody/v1/recognize-record`（前端零调用者） | 路径可救，但函数无人调 ⇒ 与退役同批裁决 |

反方向缺口：网关有 `GET /melody/v1/download/:fname`，前端 8 条字面量里没有它 ⇒ 该出口目前不可达（本轮不计入幻影）。改前缀仍是两件事：`createHttpInstance('/melody/v1')` ＋ `vite.config.js` 加 `/melody` 代理键（§5.18 E 节已证 `melody` 在代理配置里命中 0 次）。

**引用纪律（覆盖 §5.18 的对应段）**：可引用的数只有 **52／21／30**；`156`、`94`、`61` 三个数从此只能作为"仪器假账的登记项"出现，且必须同句指出 §5.19.1 的成因。

**只报不改（本轮裁决边界）**：52 条一律未改前端代码。三条理由：① melody 上游 `:8012` 未启动，改完最多看到 502 信封，拿不到成功形状；② `infinite-optimize`／`llm`／`web-search` 三族要的是**后端补路由**，前端改前缀改不出路由；③ 31 条零调用者函数的退役＝删文件，须用户点名。**待用户裁决清单**：(a) 启 `:8012` 让 melody 的两文件改动能出真形状；(b) `InfiniteOptimizerView.vue` 整页——补后端还是退役；(c) 31 条零调用者 api 函数退役批次的文件名单；(d) 是否把 `rejudge.py`＋`route-catalog.py`＋`final-52.py` 三件套落库成常驻门禁（落库必须带：≥1.2 s 节流、**探测 URL 与打印路径同源**、消费者双形识别、静态目录与现场判决双口径、目录装配力对照）。

## 5.20 联盟治理台真机联调结案：路由与五条读通道全活，修掉"把没读到说成后端没这一维"（2026-09-28）

原始读数全部在 [reports/markdown/governance-console-live-20260928.md](../../../reports/markdown/governance-console-live-20260928.md)，本节每个数字都指向那份档案的对应小节。

### 5.20.1 结案：`#11` 的"十端点零消费者"框架作废

`modules/governance/` 已入库并被 `modules/index.js:4` 挂载，路由 `/alliance/governance`（name `AllianceGovernance`，`requiresAuth: true`）与侧栏「控制台／联盟治理台」导航项都由 `defineModule` 派生；真机导航没有被守卫弹回登录，页面标题即为「联盟治理台」。五条 GET 通道经网关 `:3080` 全部 200（dashboard 2350 B／experts-status 2289 B／veto-events 92 B／audit-logs 91 B／config-experts 411 B，档案 §1），十四维与阈值/权重都渲染成后端真实读数。**所以 `/api/governance/*` 不是幻影，`156/94/61` 那本账也与它无关**（§5.19）。前缀账另用一枚反向对照钉住：`/api/api/governance/dashboard` → 404，而 `requestPath()`（`contract/endpoints.js:110-118`）剥 `^/api`、`http.js:11` 的 `baseURL` 是 `/api` ⇒ 治理台不存在双前缀缺陷。

wire 事实一条（档案 §2）：`/experts/status` **同一条响应两套命名口径**——外层 `business_league`/`dev_league`/`average_health` 是 snake_case，`experts[]` 元素是 camelCase（`healthScore`/`lastUpdated`/`totalChecks`/`expertId`）。`model/normalize.js:122-145` 已收口并注释警告过，所以这是后端的形状、不是前端的缺陷。

### 5.20.2 本轮唯一的代码改动：判决缺失 ≠ 判决为否

状态端点在飞且概览也还没回时，页面把十四维全部标成「后端未回该维」，`absentDims` 同源地产出「缺 14 维（…）」——把"还没有答案"渲染成了"答案是没有这一维"。三处改动（`store/governance.store.js:34-44` 无答案时 `return []`；`views/GovernanceConsoleView.vue:229-241` 增 `pending = !src && store.loading.experts`；`:59-65`、`:71-84` 警告段拆成互斥两条、标签链 `absent→后端未回该维 / pending→读数中 / 否则分数`、进度条与 `否决/检查/更新` 的门收紧成 `!row.absent && !row.pending`）。

双向见证（档案 §4，合成 pinia 状态的三段**不是后端数据**）：稳态 `scores=14, absent=0, pending=0, warns=[]`；真缺维方向裁掉 10 维 → `absent=10, scores=4` 且警告回到「缺 10 维（security、data、observability、api_compat、performance、maintainabil…」，还原后归零；无答案方向 → 只出现「状态端点 读数中，下面退回概览端点里的 expertStates」。**"缺维"判据仍然有牙，不是被守卫关掉的。**

### 5.20.3 仪器教训（写进流程，适用于一切真机 DOM 审计）

`browser-use` 这条页 `document.visibilityState` 恒为 `hidden` ⇒ Element Plus 的 zoom-in-center **离场 transition 永不结束、节点不 detach**。实测同一枚标签 class 里同时挂着 `el-zoom-in-center-leave-from el-zoom-in-center-leave-active`，于是 `innerText` 采样在改动后的首帧读出 `absent=14 且 pending=14 且 scores=14`（三个数同一行）。差点把这条残留当成"页面同时说三种话"的新缺陷。**判据：真机 DOM 文本审计必须先滤掉 `classList` 含 `leave` 的节点，再用状态层（pinia state）做第二口径复核。**

### 5.20.4 只报不改

1. `store/governance.store.js:43-49` 的 `dimensionRows` 全库零消费者，且是视图 `leagues` 同一逻辑的第二份实现（写法还不一致：它对 `expertsStatus.value.business.experts` 无可选链）⇒ 两份口径早晚分叉，退役＝删代码，要点名文件。
2. `PUT config/rbac`、`PUT config/experts`、`GET ws`、`POST assess*` 在本页没有出口，十条路由页面只用五条 GET（沿 §5.13 清单，本轮只复核未变）。
3. `npx prettier --check` 在本仓库**不可用**：`package.json` 是 `"type": "module"` 而 prettier 配置用 `module.exports` ⇒ `Invalid configuration ... module is not defined in ES module scope`。前端格式无门禁，不要把 `prettier --check` 当验收命令。

回归：`vitest run src/modules/governance` 26 例绿、`check-frontend-module.py` ERROR=0、`check-api-binding-kinds.py --check` PASS、`check-ep-feedback-imports.py --check` PASS（档案 §6）。SFC 模板改动的编译证据是 §5.20.2 的新文案真的渲染出来了，不是 build 日志。

## 5.21 提交前的入库一致性账：索引快照停在"governance／nav-icons／paged-list 三件事"之前，混合点名才会断构建（2026-09-28）

起因：§5.20 结案后要给用户出提交清单，顺手量了 `git ls-files` / `git ls-tree HEAD` / `git status -uall` 三口径。**这张表不是本机脏差，是"按现在的索引提交会产出什么"的判决**，所以任何提交动作前都要重跑一遍（命令在表下）。

| `frontend-ui/src/modules/` 子目录 | HEAD 文件数 | 索引文件数 | 未跟踪文件数 |
|---|---:|---:|---:|
| governance | 0 | 0 | 12 |
| expert-alliance | 0 | 74 | 3 |
| admin-lowcode | 10 | 15 | 0 |
| project / ai / graph / workflow / market / operators | 0 | 各 1 | 0 |
| system | 1 | 1 | 0 |
| `_kernel` | 0 | 2 | 4 |
| `index.js` 本体 | 0 | 1 | 0（状态 `AM`） |

边的账（逐条实测，注意区分**索引版**与**磁盘版**——`modules/index.js` 状态是 `AM`，两份不一样）：

1. **索引版 `modules/index.js` 既不登记 governance，也不 re-export nav-icons。** `git show :frontend-ui/src/modules/index.js` 的 import 只有 9 行（`:3 expert-alliance` … `:11 system`），`grep nav-icons|paged-list|governance` 在这份索引版上 **rc=1（零命中）**；磁盘版多出 `:4 import './governance/index.js'` 与 `:15 export { NAV_ICONS, navIcon, navIconNames, registerNavIcons } from './_kernel/nav-icons.js'`（`git diff --stat` → `2 ++`）。
2. **所以"按索引提交"不会坏构建，但会静默丢掉三件已经验通的功能**：治理台的路由与导航项（第 1 条的 `:4`）、导航图标登记表（`:15`，`src/main.js:40` 的注释把它写成全库扫描门禁的说明源）、以及分页归一——`ProjectsView.vue` 索引版 `paged-list` 命中 **0** 而磁盘版 **2**，`TaskView.vue` 索引版 **0** 而磁盘版 **1**（两份状态都是 `MM`）。索引快照整体停在"这三件事之前"。
3. **"部分点名"才制造断边**：`_kernel` 索引里只有 `envelope.js` 与 `module-registry.js` 两个文件，`nav-icons.js` / `paged-list.js` 连同各自的 `.test.js` 全是 `??`。一旦把磁盘版 `modules/index.js`（带 `:15`）或磁盘版两份视图（带 `@/modules/_kernel/paged-list`）点名入库而不带上对应外挂，clean checkout 的 `vite build` 直接失败——按 §5.3 的教训，这类缺文件全量 vitest 是绿的，只有 rollup 会点名。
4. **`governance/` 整目录 12 个文件 HEAD=0／index=0／未跟踪=12**，含 `governance-contract.test.js`（§5.20 那 26 例 Rust 路由表守卫）⇒ §5.20 验通的东西目前只活在这台机器上。

**结论（提交形态）**：要么一次性点名 `modules/index.js`＋`governance/` 12 文件＋`_kernel/nav-icons.js`＋`_kernel/paged-list.js`（建议连两份 `.test.js` 一起）＋两份视图的磁盘版，四条账同时闭合；要么这一轮就完全不提交前端，等点名。**中间态要么静默丢功能（照索引提交）要么必坏构建（混合点名）。** 另注：`git status` 对 `modules/index.js` 报 `LF will be replaced by CRLF`，是本机 autocrlf 的既有行为，不作为改动依据。

**"照索引提交会不会坏构建"是量出来的，不是推的**（脚本只在 scratch：`index-selfcheck.py`，读 `git show :<path>` 的**索引版内容**解析仓内 import 边）。实测：索引内 `frontend-ui/src` 文件 **325** 个、参与扫描（`.js/.ts/.vue/.mjs`）**315** 个、报出的断边 **2** 条，两条都是仪器假阳：

| 报出的边 | 真因 |
|---|---|
| `src/constants/palette.test.js:140 -> ./palette.js?raw` | 解析器没剥 Vite 的 `?raw` 查询串；`palette.js` 本身在索引里（判据要用 `git ls-files --error-unmatch` 的**退出码 0＝在索引**，别把 0 读成缺） |
| `src/stores/auth.store.test.js:3 -> ./auth.api` | 命中写在**注释里**的一句引用（`// 把 src/api/index.js:27 的 export * from './auth.api'`），注释里的 `from '…'` 会被这条正则当真 ⇒ 边解析必须先剥注释 |

剥掉这两条后 **索引快照自洽（真断边 0）**，所以第 2 条的失效方向确实是"静默丢功能"而不是"build 失败"，而第 3 条（磁盘版带着索引里不存在的 `./_kernel/nav-icons.js`、`@/modules/_kernel/paged-list`）才是会断构建的那一侧——两句话分别有各自的方向证据，不许互换。

**2026-09-28 收束轮复测：本文件自己是这条账里最重的一项，而且差额比想象中大。** 实测 HEAD 已从 `12b6021f` 前进到 `ccb7db6a`（网关侧改动），`frontend-ui/src` 最近 60 分钟零写入（对方本轮没动前端）。本文件状态 `AM`，`git diff --numstat` = **659/0**；把索引版拉出来量：`git show :docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md | wc -l` → **170 行**，而工作树 **829 行**，且索引版里 `grep "^## 5\.[12][0-9]"` **零命中**（§5.9～§5.24 全部不在索引里）。⇒ **今天按索引提交会把这份权威文档截回 170 行、抹掉近几个单元的全部结案**，比第 2 条原本描述的"丢三件功能"更严重。`reports/markdown/governance-console-live-20260928.md` 仍是 `??`（不进任何提交，也不会被截断）；`modules/index.js` 的索引-vs-磁盘差额仍是 2 行（第 1 条的两处边未变）。**处置只能由用户点名**：点名"提交这份文档"时务必按工作树版本入内（`git add -- <该文件>` 之后再提），或直接 `git commit -- <路径>` 前先复量 `git diff --numstat` 是否为 0。本节仍不动索引。

复算命令（2026-09-28 本轮用的原样）：

```bash
for d in governance expert-alliance admin-lowcode project ai graph workflow market operators system _kernel; do
  printf '%-16s HEAD=%s index=%s untracked=%s\n' "$d" \
    "$(git ls-tree -r --name-only HEAD -- frontend-ui/src/modules/$d | wc -l)" \
    "$(git ls-files -- frontend-ui/src/modules/$d | wc -l)" \
    "$(git status --porcelain -uall -- frontend-ui/src/modules/$d | grep -c '^??')"
done
git status --porcelain -- frontend-ui/src/modules/index.js   # 期望 AM：索引与磁盘已分叉
```

## 5.22 联盟域可达性账（10 条模块路由逐条点名线上请求）＋一条仪器假阳的结案（2026-09-28）

这一节回答两件事：**模块化后的联盟域页面是不是每个都能打通后端**，以及**"页面显示 `—` 而 store 有数"这类现象该判代码缺陷还是判探针**。全程真后端（网关 `:3080` 在跑），取证通道是页内 XHR 记录器（patch `XMLHttpRequest.prototype.open/send`，axios 走 XHR，包 `window.fetch` 一条也捕不到——§5.20 已记）。

### 5.22.1 路由总账

`router.getRoutes()` 实测 **86** 条记录，`meta.module || meta.moduleName` 直方图：`(none) 51 / expert 11 / ai 7 / project 5 / admin 4 / graph 3 / market 2 / workflow 2 / operators 1`，**重名 0**。

`(none) 51` 不是缺陷而是账目形状：模块顶层路由带 `meta.module`，而 `defineModule()` 声明的 children 只写自己的 `meta.title`，不继承父级模块名 ⇒ 51 条"读起来像静态路由"的记录其实是模块子页。收窄模块归属要靠 `modules/*/index.js` 的树形，不能靠 `meta`（**只报不改**，改法是给 kernel 的 collectRoutes 做一次 meta 下传，属内核面）。

### 5.22.2 十条联盟域路由的线上请求（GET 侧全 200，无一条 4xx/5xx）

| 路由 | 捕到的 GET（去 `/api/health` 探针） | 判决 |
|---|---|---|
| `/expert-workspace` | 15 条：`experts` 13279B、`experts/sessions?page_size=20` 7500B、`expert-graph` 9865B、`experts/capabilities` 3867B、`workspace/kpi` 297B、`kb/documents?project_id=xuanji&limit=50` 863B、`kb/tags` 516B、`kb/categories` 210B、`projects` 209B、`notifications/unread-count` 142B、`projects/xuanji/members` 125B、`expert-graph/neighbors/domain-ai` 254B、`projects/xuanji/{phases,files}` 102B/73B、`workspace/history?limit=50` 75B | 活，且是唯一"跨域聚合"页（联盟＋项目＋知识库三套出口同屏） |
| `/expert-plaza` | `experts` 13279B、`experts/stats` 648B、`experts/bookings/mine` 372B | 活 |
| `/alliance/console` | `alliance/tasks` 99B、`alliance/runtime` 148B、`experts/dispatcher/config` 212B、`experts/dispatcher/status` 1216B | 活 |
| `/alliance/collab` | **0 条业务 GET** | **按设计**：本页是六个 `POST /api/experts/<mode>` 的入口，页面文案自己写明"一次请求即拿到协作结果，后端不落库为任务"，实测文本落到"请填写问题描述"的表单初始态 ⇒ 无 GET 不等于无数据面 |
| `/alliance/graph` | `expert-graph` 9865B、`expert-graph/stats` 1478B、`expert-graph/communities` 3371B | 活 |
| `/alliance/sessions` | `experts/sessions?page=1&page_size=20` 7500B、`experts/sessions/stats` 667B | 活，渲染"30 个会话 / 47 条消息，第 1 / 2 页" |
| `/alliance/orchestration` | `experts/orchestration/stats` 332B、`experts/orchestration/history?page=1&page_size=20` 77B | 活（history 空表 77B，页面已声明进程内不累计） |
| `/alliance/experts` | `experts?page=1&page_size=24` 13279B、`experts/bookings/mine` 372B、`experts/stats` 648B | 活，见 5.22.3 |
| `/alliance/governance` | `governance/dashboard` 2350B、`experts/status` 2289B、`config/experts` 411B、`veto/events` 92B、`audit/logs` 91B | 活——§5.20 修后回归复测，五通道字节数与该节记录逐一相同，页面显示"审计链校验通过" |
| `/expert-config` | **0 条业务 GET** | 候选缺陷，见 5.22.4 |

每条都伴随 `GET /api/health 200 68B`，`/alliance/governance` 一次挂载捕到 **2** 次（挂载期与一轮健康轮询重叠）。做请求数对照时要先把 health 摘掉，否则"14 条 vs 16 条"这种差额全在探头上。

### 5.22.3 结案：`—` 是长命 dev 会话的 HMR 残留实例，不是生产缺陷

现场：`/alliance/experts` 六个 `.ax-kpi` 全显 `—`，而同一次读取里 pinia `allianceExperts.stats` = `{totalExperts:11, onlineExperts:11, busyExperts:0, offlineExperts:0, totalConsultations:3561, todayConsultations:1, avgRating:4.3636…, avgResponseMinutes:5, …}`、`loading.stats === false`。逐级排除：可见离开节点（`/leave/` 祖先链，0 命中，6 个格子 `visible:true`）、纯函数本身（`expertStatsCells(stats)` 直接喂 store 对象返回 `11 位 / 11 / 0 / 0 / 3561 次 / 1 次 / 4.4 / 5 分钟`，`sameRef:true`）、store 取错（视图用 `useAllianceExpertsStore()`，id 即 `allianceExperts`）、键名错配（4 条都不成立）。

判决动作是**同一个 URL 冷加载**：重载后六格显示 `11 位 平台专家 / 11 · 0 · 0 在线/忙碌/离线 / 3561 次 累计咨询 / 1 次 今日咨询 / 4.4 平均评分 / 5 分钟 平均响应`。前后唯一的自变量是组件实例被重建（store 数据本来就在，`stats.ts` 只是被后端刷新过一次），所以这是本轮我做了多次 HMR 编辑之后 **`computed` 挂在旧实例上没重算**，属开发器现象。

**代码零改动**（改一个只在 HMR 下复现的渲染不是修缺陷，是给生产加噪声）。但留下一条 instruments 纪律，比 §5.20 的"过滤 leave 节点"更靠前：**在一个活了几小时、被反复 HMR 的 dev 页面上，任何"UI 显示 A 而状态是 B"的断言，先冷加载该 URL 再判**。冷加载是唯一能区分"探针的会话脏了"与"应用真有病"的动作；跳过它写下的缺陷条目，代价是让下一轮去修一个不存在的东西（本轮差点这么干）。

### 5.22.4 只报不改

1. `/expert-config` 挂载**零 GET**，而文案写着"实时预览 · 一键发布"、"配置版本 v1.0.0"：本页是低代码编辑器，配置从路由参数/本地草稿来，未见读通道。要么是"纯本地草稿"的设计（那就该在页面上说明草稿与后端的边界），要么是漏了 `GET /api/experts/config…`。**需要后端契约裁决**，不在前端单方面补。
2. 51 条路由的模块归属在 `meta` 里读不出来（5.22.1）。
3. 编排台的历史 77B（空表）与治理台同源问题：进程内读数、重启归零，页面已各自声明，不再重复定性。

复算通道：页面内装载 XHR 记录器后逐条 `location.hash = '#/<path>'; await sleep(1600)`，读 `window.__cap`；纯函数对照用 `import('/src/modules/expert-alliance/contract/enums.js')`。读数存档见 `reports/markdown/governance-console-live-20260928.md` §8。

## 5.23 A 族猜形状兜底链的现场定价：遗留专家广场 11 张卡全部显示编造的 0（2026-09-28）

§5.22 的可达性账顺带把遗留页 `views/expert/ExpertPlazaView.vue`（路由 `/expert-plaza`，`meta.module: 'expert'`，与模块页 `/alliance/experts` 同题）拉出来量了一遍。结论：**这不是"兜底链可能不准"，是页面正在显示假数据**。

### 5.23.1 输入侧事实（页内 `await import('/src/api/index.js')` 调 `getExperts()`，取首条元素的键集）

wire 给的 23 个键：`availability avatar bio capabilities created_at domains enabled expert_type hourly_rate_cents id languages metadata metrics name organization pricing_model skills tags timezone title type updated_at verification_status`。

`processExperts()`（`:831-850`）读的 16 个键里，**只有 `skills`、`capabilities`、`bio`、`type` 存在**；下列 10 个在 wire 上**一个都没有**，于是 `?? 兜底` 恒取兜底值：

| 读取 | 兜底 | 实际显示 | wire 上真正的出处 |
|---|---|---|---|
| `e.consultCount` | `0` | `0 次咨询`、`0 参与项目` | `metrics.total_consultations`（存在，模块归一层 `normalize.js:255` 已读为 `metrics.totalConsultations`） |
| `e.avgRating` | `'0.0'` | `⭐ 0.0`、`0.0 用户评分` | `metrics.avg_rating`（`normalize.js:257`） |
| `e.goodRate` | `'0.0'` | `0.0% 好评率` | **每专家口径不存在**——平台级只有 `satisfaction_rate`，`metrics.resolution_rate` 是"解决率"不是好评率 |
| `e.responseTime` | `'-'` | `-` | 每专家只有 `metrics.total_service_minutes`（累计服务分钟），**不是平均响应** |
| `e.price` | `0` | — | `hourly_rate_cents` + `pricing_model`（单位是分，直出即错 100 倍） |
| `e.online` | `false` | — | `availability`（是对象，`normalize.js:237` 展开） |
| `e.recommended / favorited / hot / isNew` | `false` | — | **wire 上无此概念**；`contract.test.js:506` 早把这些名字列为模块源禁令，但遗留页在扫描集之外 |

### 5.23.2 输出侧事实（冷加载 `/expert-plaza` 后读 DOM）

11 张卡，逐张为 `⭐ 0.0 (0 次咨询) · 0 参与项目 · 0.0 用户评分 · 0.0% 好评率`，`"0 次咨询"` 在页面文本里出现 **11 次**。同一时刻模块页 `/alliance/experts` 显示 `平台专家 11 位 / 累计咨询 3561 次 / 平均评分 4.4 / 平均响应 5 分钟`（§5.22.2 实测），排行榜排序也用的这批假值（`:774-789` 按 `goodRate`/`consultCount`/`responseTime` 排，且 `:788` 把它们混成 `goodRate*0.4 + consultCount/20*0.3 + (5-parseFloat(responseTime))*20*0.3` 的加权分）⇒ **默认排序也是假的**。

另一条独立缺陷：`heroStats`（`:660-664`）的四个 label 只出现在这个数组字面量里，**HEAD 版与工作树版的模板都没有任何消费者**（`git show HEAD:… | grep -n heroStats` 命中行 = 定义 1 + 自赋值 4，无模板行），而 `loadStats()` 确实在 `:1096` 的挂载 `Promise.all` 里发出 `GET /api/experts/stats`（§5.22.2 捕到 648B）⇒ **一次有去无回的请求**；并且它读的 `expert_count / consult_count / good_rate / avg_response` 四个键在真实载荷里都不存在（`avg_response` 只是 `avg_response_minutes` 的前缀，子串命中会骗过探针）。

### 5.23.3 为什么本轮不机械改这个函数（裁决边界）

两处不能自动补：① **`好评率` 与 `平均响应` 没有每专家口径的数据源**，硬映射到 `resolution_rate`／`total_service_minutes` 就是给一个不存在的语义编数——正是 §5.19 与 A 族账目要避免的病；② 模板把值写死成 `{{ expert.goodRate }}%` 这类形态，改成"没读到就说没读到"要连模板一起改（卡片 3 格＋详情弹窗 4 格＋排行榜 2 列，约 15 个出口），而**同一主题现在有两个广场页**（遗留 `/expert-plaza` 与模块 `/alliance/experts`，后者已经是正确实现）。

⇒ 交给用户二选一，代价差别很大：

- **退役遗留广场**（把它从路由摘掉，模块页承接）：`processExperts` 的 10 个假读、`heroStats` 的死数组与那次有去无回请求、以及同题双源一起消失，改动是删；
- **保留遗留广场**：则需逐出口迁移到模块 `normExpert` 的口径（`metrics.totalConsultations`/`metrics.avgRating` 可直填；`好评率`/`平均响应` 要么删格要么由后端补每专家字段），约 15 处模板改动＋每专家缺字段的后端裁决。

另需一条 label 裁决：卡片第三格的标题是 **`参与项目`**，而它读的字段是 `consultCount`（咨询数）——标题与口径本就不同题。

**只报不改**：本轮零代码改动（§5.22.3 亦零改动）；A 族在遗留面的这一处已由"可能不准"升级为"正在显示假数据"，其余 B/C 族余账维持 §5.22 之后的清点。

## 5.24 `/expert-center` 四面板补账：外壳用 v-show 常驻总览，探针两次差点把"仪器没等够"写成缺陷（2026-09-28）

§5.22 的十条路由不含 `/expert-center/*`（专家中心是 Tab 外壳＋嵌套路由）。补量后有四条，另有一条仪器教训值得单独留档，因为**本轮它两次把我带向假缺陷**。

### 5.24.1 结构与读数

外壳 `views/expert/ExpertCenterView.vue`：`:34` 用 `v-show="activeTab === 'overview'"` **常驻渲染总览面板**，`:39` 才 `<router-view v-if="activeTab !== 'overview'">`；`activeTab:91-100` 由 `route.name` 派生（含 `Tasks/Enterprise/Orchestrator` 即切）。四面板逐条实测（冷加载会话、页内 XHR 记录器、等待窗口 6.5 s）：

| 路由 | 面板唯一标记 | 本面板自身的 GET | 全部 200 |
|---|---|---|---|
| `/expert-center/overview` | （总览常驻） | `experts/overview` 1418B、`experts?page=1&page_size=100` 13280B、`expert-graph` 9865B、`ai/chat/history/<sid>` 31B | 是 |
| `/expert-center/tasks` | 页面文本含"联盟任务 描述目标…新建任务"（Tab `[联盟任务]`） | 挂载窗口内 0 条自身请求（显示"正在检查任务服务…"） | — |
| `/expert-center/enterprise` | `企业级专家管理控制台` 命中，`.page-container` 计数 1，Tab 呈 `[企业管理]` | 见 5.24.2 的说明：窗口内捕到的是总览那 4 条 | 是 |
| `/expert-center/orchestrator` | `V2 编排引擎控制台` 命中，`.page-container` 计数 1，Tab 呈 `[编排引擎]` | `experts/orchestration/stats` 332B、`experts/orchestration/plugins` 2369B、`experts/orchestration/history?limit=20` 77B | 是 |

⇒ **四个 Tab 面板都真实换景**，编排面板的三个挂载出口全活。企业管理面板自身的三条挂载调用（`:662-664` `getExpertSessions/getExpertGraphStats/getDispatcherStatus`）在窗口内未捕到，与"面板已渲染"不矛盾：`ExpertEnterprisePanel` 内部还有一层子标签（仪表盘/会话中心/能力图谱/流程编排/企业协作），读数挂在子标签而非根 `onMounted`，本轮未下钻（**记为未结，不当缺陷**）。

### 5.24.2 仪器教训（两条，都会把人推向假缺陷）

1. **dev 模式下 2.5 s 的等待窗口不够**。第一轮 walk（每路由 2.5 s）量到 enterprise/orchestrator "0 条请求、页面文本与总览一模一样"，看起来像"子路由不换景"的真缺陷；把窗口提到 6.5 s 后同一台浏览器、同一会话，两个面板的唯一标记与自身请求全部到位。**成因**：Vite dev 未打包，首次进入某路由要现场拉几十枚模块（`ExpertEnterprisePanel:473` 就 `import * as echarts from '@/echarts'`），4 条 walk 总耗时 16.1 s 而纯等待只占 10 s ⇒ 主线程当时在忙模块图，不是在渲染。
2. **`v-show` 常驻使 `innerText` 审计结构失明**。总览面板永不卸载，其正文（"项目阶段 4 阶段架构流程…测试专家·云帆…"）在四个 Tab 下都会出现在 `main.innerText` 里 ⇒ 判"面板有没有换景"不能比整屏文本，要用**面板唯一标记**（`企业级专家管理控制台`／`V2 编排引擎控制台`）＋ `.page-container` 计数 ＋ `.el-tabs__item.is-active` 三个结构信号。这与 §5.20 记的"leave 节点残留"是同一类病的第二个来源：一个是离开动画不卸载，一个是外壳故意不卸载。

### 5.24.3 只报不改（本轮唯一实收的新账）

**隐藏的总览面板仍在轮询**：`/expert-center/enterprise` 那一步的 6.5 s 窗口里捕到 4 条请求，全部属于总览（`experts` 13280B ＋ `expert-graph` 9865B ＋ `experts/overview` 1418B ＋ `ai/chat/history` 31B ≈ 24.6 KB），而用户此刻看的是企业管理面板。`v-show` 让 `ExpertOverviewPanel` 的 `onMounted:497` 及其 `watch` 通道在 Tab 切走后继续发请求 ⇒ 停在任意子 Tab 上，页面都在为看不见的总览买单流量与 echarts 重绘。修法有两个方向（切走即停轮询／把总览也改成 `v-if` 懒挂载），但都要改这个面板的生命周期，属遗留外壳面，**与 #20 同一族裁决**，本轮不动。

复算：见 `reports/markdown/governance-console-live-20260928.md` §10（含两轮 walk 的对照原始读数，以及 fire-and-forget 探针写法——`evaluate_script` 的 15 s 上限会截断"边导航边等待"的脚本，但**脚本本身会继续跑完**，正确写法是一次调用只负责启动并把结果落到 `window.__logN`，下一次调用再读）。

## 5.25 DB 驱动图标名的第二个消费者收口：AdminMenu 两个渲染位点＋录入位点，三枚变异体各自打红（2026-09-28）

单元：`frontend-ui/src/views/admin/panels/AdminMenu.vue`（菜单配置面板）。§5.14 立了 `_kernel/nav-icons.js` 这个唯一登记口，但只把**侧栏**那一条消费通道接上了解析器；本文件是登记口的第二类消费者——**数据库里的 `icon` 字段直接进模板**，同一份名字在一页里被消费两次，而三处写法都不认名单。

改动前的三条边（同一文件内）：

| 位点 | 旧写法 | 失效方向 |
|---|---|---|
| `菜单名称` 列前置图标 `:37-39` | `<el-icon v-if="row.icon"><component :is="row.icon" />` | 名单外的名 → 解析成 `<thisicon…>` 未知标签 ⇒ 图标位空白，且 `v-if="row.icon"` 判真，**连"它没画出来"这件事本身都不成立** |
| `图标` 列 `:44-56` | 同上一条（纯字符串 `:is`） | 同上：这一列存在的意义就是"看得见配了什么图标"，空白等于把它自己作废 |
| 表单录入 `:136-138` | `<el-input placeholder="请输入图标名称，如 Menu" maxlength="50" />` | 自由文本 ⇒ 打错一个字母落进 DB 就是一件看不见产出的东西，且没有任何一处会报错 |

改动后：两处渲染走 `navIcon(row.icon)`（已登记→组件；未登记→`el-tooltip` 里打印**原始名**；空→显式 `-`），录入端换成 `filterable` 的 `el-select`，候选只能来自 `navIconNames()`（当前 63 个），每个候选行内自带预览。语义是"名单封闭"：**能选的侧栏就一定能画**，反过来侧栏画不出的名在录入端就选不出来——旧文案"填写 Element Plus 图标名称"是一句没有闸门的话。

### 5.25.1 见证与被撤掉的通道（不是"全绿即通过"）

冒烟测试扩到 2 例（`AdminMenu.smoke.test.js`）。为了让断言能按列头点名而不是钉在模板列序上，共享替身 `_smoke.js` 的 `ElTableColumn` 补了 `data-label` / `data-prop` 两个属性（**纯增属性，17 个 admin 面板冒烟全跑一遍 23 例复绿**）。三枚就地变异体各撤一条通道，每枚都要红、撤完逐字节还原：

| 变异体 | 撤掉的通道 | 判决 |
|---|---|---|
| M1-render-both-columns | 两列的 `navIcon(row.icon)` 全退回裸 `:is` | `rc=1`，`1 failed \| 1 passed` |
| M2-render-name-col-only | 只退`菜单名称`列 | `rc=1`，`1 failed \| 1 passed` |
| M3-input-is-select | `<el-select>` → `<el-input>` | `rc=1`，`1 failed \| 1 passed` |

驱动 `D:\tmp\mox-icon-mut.py`（**未落库＝没牙，别接 CI**）内置三条纪律：跑前先要有基线判决行（`BASELINE rc=0 2 passed (2)`）、每枚 `assert mutated != text` 防静默 no-op、`finally` 还原后 `assert restored == orig` 按字节比，并在每枚还原后复跑一次绿（三行 `-> restored rc=0`）。本轮原样复现：

```
BASELINE rc=0  2 passed (2)
M1-render-both-columns           rc=1  1 failed | 1 passed (2)
M1-render-both-columns -> restored rc=0  2 passed (2)
M2-render-name-col-only          rc=1  1 failed | 1 passed (2)
M2-render-name-col-only -> restored rc=0  2 passed (2)
M3-input-is-select               rc=1  1 failed | 1 passed (2)
M3-input-is-select -> restored   rc=0  2 passed (2)
ALL MUTANTS CAUGHT + RESTORED
```

还原后与变异前图像 `cmp` 一致（`SRC byte-identical to pre-mutation seed`）。踩到的一次事故要登记：备份文件最初写在 `frontend-ui/` 里（＝仓库内的散件），且 `subprocess.run(["npx", …])` 在这台机上直接 `FileNotFoundError`（npx 是 `.cmd`），改成 `node node_modules/vitest/vitest.mjs` 才起得来——**驱动器一崩就把备份留在了仓库树里**，所以备份路径必须在仓库外（现在是 `D:\tmp\AdminMenu.vue.pre`），并加"不许覆盖已有前像"的断言。

### 5.25.2 门禁复算与"这一轮没拿到的证据"

| 命令 | 结果 |
|---|---|
| `npx vite build`（改动后当前磁盘态） | `✓ built in 34.39s`，`rc=0` |
| `npx vitest run src/views/admin/panels` | `Test Files 17 passed (17) / Tests 23 passed (23)` |
| `npx vitest run src/modules/_kernel/nav-icons.test.js` | `9 passed (9)`（全库扫描的下限钉针未因本文件退场而红） |
| `python scripts/gate/check-frontend-module.py` | `ERROR=0` |
| `node frontend-ui/scripts/gate/check-sfc-dead-refs.mjs` | `deadRefFiles=1 verdict=FAIL`，唯一一条仍是存量 `src/views/project/ProjectsView.vue :: projectMemberCount`，**本文件 0 条** |

**没拿到的证据要说清**：`菜单名称`/`图标`两列在真机页面上的渲染从未被眼看——本轮开工时 dev `:3020`、网关 `:3080`、编排器 `:3001` 三个端口 `netstat` 全部无 LISTEN 记录（`netstat -ano \| grep LISTEN` 复算），页面级证据通道不存在。上面的三枚变异体是把"断言挂在通道上"当成验收的**替代证据**，它能证明测试有牙，不能证明真机浏览器里那一格画得出 `<svg>`。

### 5.25.3 余账：登记口的消费者还差 19 处（15 文件）

本轮只收 `AdminMenu.vue`（该文件 2 → 0）。其余裸 `:is="…icon"` 位点原样在册，复算命令与逐条坐标：

```bash
cd frontend-ui && grep -rnE ':is="[a-z][A-Za-z.]*[Ii]con"' src/ --include=*.vue | grep -v "navIcon(" 
```

`App.vue:64`｜`components/AgentFlowPanel.vue:31`｜`components/layout/TheTopbar.vue:53`、`:95`｜`components/ProjectPicker.vue:193`（该组件是孤儿，见 #ProjectPicker 孤儿账）｜`views/admin/AdminView.vue:18`｜`views/admin/panels/AdminLlm.vue:19`｜`views/admin/panels/AdminOverview.vue:6`、`:54`｜`views/admin/panels/AdminRole.vue:184`｜`views/expert/panels/ExpertEnterprisePanel.vue:25`｜`views/project/Dashboard.vue:98`、`:165`｜`views/project/panels/KnowledgeBasePanel.vue:19`、`:828`（后者在说明文字里）｜`views/project/ProjectsView.vue:383`｜`views/public/PortalHome.vue:48`｜`views/workspace/panels/CollaborationPanel.vue:66`｜`views/workspace/panels/KnowledgeBasePanel.vue:31`。

这 19 处**不在 `nav-icons.test.js` 的钉针集里**（该测试钉的是"数据表里的 `icon: 'Aim'` 名"与"内联 `:is`"两条通道的**下限**，减一个不会红）——所以"测试绿"对这 19 处等于零覆盖。逐文件收口要先问一件事：这些 `icon` 是从 DB／配置来的（跟本文件同病），还是从代码常量来的（常量早在登记口扫过，画不出来就是登记口漏登记）。两种病因修法不同，不做机械替换。登记为 #14 的余账。

复算：上述八行由 `D:\tmp\mox-icon-mut.py` 现场打印，原样归档在 `reports/data/adminmenu-icon-mutation-witness-20260928.txt`（742 B）；5.25.2 表里三条门禁的原始 stdout 各归档在 `reports/data/adminmenu-vite-build-20260928.txt`、`adminmenu-panel-suite-20260928.txt`、`adminmenu-sfc-deadrefs-20260928.txt`；开工条件与"没拿到的真机证据"记在 `reports/markdown/adminmenu-icon-registry-20260928.md`。本节里只有 `nav-icons.test.js` 的 `9 passed` 与 `check-frontend-module.py` 的 `ERROR=0` 两条没单独归档（只落 stdout）。

## 5.26 余下 19 处裸 `:is` 的先归因后动手：只有 2 处是真缺陷，且新门捕获了自己那枚死针（2026-09-28）

§5.25 结尾登记了 19 处（15 文件）未接解析器的 `:is="….icon"`，并明写"两种病因修法不同，不做机械替换"。本轮把 19 处逐位归因（判据：喂给它的那个表达式里的 `icon` 值从哪来），结果只有 **2 处**属真缺陷。**分类结论先于改动**，因为把 `navIcon()` 套到喂组件对象的位点上会制造新的空白格。

### 5.26.1 四类归因（逐位，坐标即证据）

| 类 | 处数 | 位点 | 判据（实测） | 处置 |
|---|---:|---|---|---|
| **A 外部字符串**（DB／服务端字段直达 `:is`） | 2 | `views/admin/panels/AdminRole.vue:184`、`views/project/ProjectsView.vue:383` | 前者 `:data="menuTree"`，`menuTree` 来自 `AdminRole.vue:706` 的 `getMenuTree()` ⇒ 与 §5.25 修掉的是**同一张菜单表的同一列**；后者 `categories.value = (ts && ts.categories) \|\| []`（`ProjectsView.vue:823`，`ts` 来自 `getProjectTypes()`） | **本轮收口**：两处都改成 `navIcon(…)`＋名单外文本回落 |
| **B 常量字符串·名全在 63 名单内** | 12 | `App.vue:64`、`TheTopbar.vue:53`、`:95`、`AdminView.vue:18`、`AdminLlm.vue:19`、`AdminOverview.vue:6`、`:54`、`Dashboard.vue:98`、`:165`、`PortalHome.vue:48`、`workspace/panels/CollaborationPanel.vue:66`、`workspace/panels/KnowledgeBasePanel.vue:31` | 名字来自本文件常量或 `constants/nav.config.js`；现场数过：`NAV_MODULES` 12 个、`QUICK_CREATE_COMMANDS` 6 个**全部在登记口内**；`ExpertWorkspaceView.vue:460` 的三个（`ChatLineSquare`/`CollectionTag`/`FolderOpened`）也在。`main.js` 的 `registerNavIcons(app)` 把这些名字注册成全局组件 ⇒ 字符串 `:is` 解析得出，**当前不空白** | **不动**：接解析器是零用户收益的改动，只会把 12 处已验通渲染推进未知 |
| **C 喂的是组件对象** | 3 | `components/AgentFlowPanel.vue:31`（`icon: Share`）、`views/expert/panels/ExpertEnterprisePanel.vue:25`（`icon: markRaw(ChatDotRound)`）、`views/project/panels/KnowledgeBasePanel.vue:19` | 该文件 `:828` 的注释本来就写着"icon 传组件本身…字符串名解析不出图标" | **禁止**接 `navIcon()`：解析器对非字符串返回 `null`，机械替换会把 3 个画得出的位点改成空白 |
| **D 仪器假阳** | 2 | `views/project/panels/KnowledgeBasePanel.vue:828`（注释行被行匹配当真站点）、`components/ProjectPicker.vue:193`（孤儿组件，无 importer） | 逐行读到上下文 | 记账不改 |

顺带钉住一条口径：`grep -c` 数的是**行**不是**出现次数**——`ProjectsView` 那处两个 `navIcon(c.icon)` 落在同一行，`grep -c` 报 1、`grep -o \| wc -l` 报 2；变异驱动按后者钉 `count == 2`，第一次按 1 写就撞在锚点断言上（崩在写入之前，磁盘未动）。

### 5.26.2 新落的常驻门禁：`src/views/_icon-registry-consumers.test.js`

不用挂载而用**编译产物**做见证，理由写在文件头：A 类两处站点分别埋在角色权限弹窗与新建项目对话框的 `v-if` 子树里，打开弹窗要拖一整套替身链路；而这一位要钉的只是"外部字符串没有直达 `:is`"。实测编译形态（把 `AdminMenu.render` 原样落盘到 `D:\tmp\render-adminmenu.txt`，36,067 字符）：

- 合规：`resolveDynamicComponent($setup.navIcon(row.icon))`
- 违规：`resolveDynamicComponent(row.icon)`

于是两道断言：①`navIcon` 引用数 ≥ 每位点下限（AdminMenu 编译产物实测 5 次引用：两列各 `v-if`＋`:is` ＋下拉预览）；②`not.toMatch(/resolveDynamicComponent\(\s*(?!navIcon\()[\w$?.]*\.icon\b/)`。三张被钉的面板（AdminMenu/AdminRole/ProjectsView）都在名单里，**新文件落库即进全量套件**。

四枚变异体（驱动 `D:\tmp\mox-icon-consumers-mut.py`，未落库＝对 CI 没牙；判决原样存 `D:\tmp\icon-consumers-mut.out`）：

| 变异体 | 撤什么 | 判决 |
|---|---|---|
| M1 | `AdminRole` 的 `navIcon(data.icon)` → `data.icon`（2 处） | `rc=1 1 failed (1)` |
| M2 | `ProjectsView` 的 `navIcon(c.icon)` → `c.icon`（2 处，同行） | `rc=1 1 failed (1)` |
| M3 | `AdminMenu` 的 4 处全撤 | `rc=1 1 failed (1)` |
| M4（岛屿对照） | **只**把 `AdminMenu` 两处 `:is="navIcon(row.icon)"` 退成 `:is="row.icon"`，`v-if` 里的 `navIcon` 留着 | 第一版 `rc=0` ⇒ **②是死针**；改成上面那条按实测编译形态写的正则后 `rc=1` |

M4 这一枚是给**判据自己**做的对照：第一版②写作 `resolveDynamicComponent\((?:_ctx\|props)\.?.*icon`，而作用域插槽解构出的 `row` 在编译产物里就是裸 `row.icon`，正则看不见它 ⇒ 断言永不红。没有 M4，②会以"看起来在钉东西"的样子躺在门禁里。**这条对照是本轮最实在的产出**，比两处收口本身更耐久。

驱动的"哪道断言打红"通道本轮也没读数（`fired=(no arrow line)`：我按 `^\s*\d+ → ` 抓 vitest 失败行，实际格式不是这一形）。所以 M1–M3 只报"红"，不报红在哪道；M4 的红**由构造保证**挂②（`navIcon` 引用数 5→3 仍 ≥ 下限 2，①在 M4 下必绿）。别把 M1–M3 读成"两道都红"。

### 5.26.3 门禁复算与仍欠的那条

| 命令 | 结果 |
|---|---|
| `node node_modules/vitest/vitest.mjs run src/views src/modules/_kernel` | `Test Files 23 passed (23) / Tests 108 passed (108)`（含新落文件） |
| `npx vite build` | `✓ built in 33.59s`，`rc=0` |
| `python scripts/gate/check-frontend-module.py` | `ERROR=0` |
| `node frontend-ui/scripts/gate/check-sfc-dead-refs.mjs` | `deadRefFiles=1`（仍是存量 `ProjectsView.vue :: projectMemberCount`，本轮 0 新增） |

仍欠：A 类两处的**真机渲染**（角色权限树节点、项目分类选择器）与 §5.25 同一条欠账——`:3020`／`:3080`／`:3001` 本轮仍无 LISTEN。B 类那 12 处若哪天有名字被移出登记口（或被写进 DB），编译级门禁只覆盖三张面板，其余靠 `nav-icons.test.js` 的数据表扫描——两道之间没有重叠，这条边界要留给下一个单元。

复算：`icon: ` 字面量的分类由 `D:\tmp\icon-const-class.py` 现场数出（`NAV_MODULES=12 名单外Pascal=0`、`QUICK_CREATE_COMMANDS=6 名单外Pascal=0`、`ICON_NAV_GROUPS=16 全 emoji`、`MODULE_SIDEBAR_CONFIG=17 全 emoji`；emoji 走侧栏文本回落，属 §5.14 设计内行为，不是缺陷）。19 处的坐标表见 §5.25.3。EP 全量导出表本轮没读到（`node_modules/@element-plus/icons-vue/dist/index.d.ts` 路径不存在，`ep_declared=0`）⇒ "名单外的名字能不能补登记"这类判断本轮只能靠 `nav-icons.test.js`，不许凭印象扩表。


## 5.27 时间口径第二批：F2 族 8 处逐字符副本收成一把出口，两处行为差记账（2026-09-28）

承 §5.25/§5.26 之后回到 task #12 的 B 族。本轮只动"逐字符同形"的那一族，其余按族定价、不改版面。

### 5.27.1 本轮现测的族分布（`reports/data/time-family-census-after.txt`，非测试源，扫描器 `D:\tmp\time-family-census-after.py`）

| 族 | 形状 | 本轮实测 | 判决 |
|----|------|------:|------|
| F2 | `…toLocaleString('zh-CN', { hour12: false })` | 收口前 **8** 处 → 收口后 **1** 处（只剩登记口 `src/utils/time.js:68`） | **已收**：输出等价地并入单出口 `formatDateTimeLocale` |
| F1 | `new Date(…).toLocaleString()` 无参 | **7** 处（`widgetRegistry.js:12`、`AdminAccess.vue:124`、`AdminAudit.vue:272`、`AdminConfig.vue:193`、`AdminHitl.vue:168`、`AdminMenu.vue:314`、`TaskView.vue:425`） | **定价不改**：无参输出随宿主 locale 变，改了就是换版面，需产品点头（上一轮记的"F1 5 处"作废，本轮按接收者类型复算） |
| F1-数 | `n.toLocaleString()` 数字千分位 | **3** 处（`AdminLlm.vue:258/278`、`ExpertPlazaView.vue:672`） | **不算时间债**：接收者是数字。扫描器只按方法名匹配会把它们并进 F1 ⇒ 仪器缺陷已在此登记，判口径须先看接收者 |
| F3 | 带其它选项集 | **4** 处（`AdminLlm.vue:647`、`MarketView.vue:608`、`BrowserView.vue:586`、`ExpertEnterprisePanel.vue:659`＝仅 `'zh-CN'` 无选项） | **不并**：选项集不同是分工不是重复；`659` 那处本机实测与 `'zh-CN', {hour12:false}` 同形（`2026/9/28 13:05:09`），无 12 小时制故障 |
| F4 | `getFullYear()+`／`getMonth()+1` 手工拼接 | **6** 处文件命中 | 余账：其中 `ExpertPlazaView` 受 #20 裁决门挡，`utils/time.js` 自己那处是出口实现 |

### 5.27.2 出口契约与两处有意的行为差

`formatDateTimeLocale(ts)`（`src/utils/time.js`，经 `utils/index.js:11` 由 barrel 再导出）对**可解析且 t>0** 的输入返回 `new Date(t).toLocaleString('zh-CN', { hour12: false })`；对坏值返回 `null`，把空态文案交回调用方——八个站点的空态本来就不一样（`'-'`／`'—'`／原样回显 `iso`／原样回显 `String(ts)`），出口若替它们编一个默认串就会改掉版面。

两处不是等价改写，逐条登记：

1. **t ≤ 0**（epoch 0 与 1970 年前）旧写法印 `1970/1/1 08:00:00`（本机实测），新出口回 `null` ⇒ 站点回落到自己的空态。这些字段全部来自后端 `now_iso()`（`mox-kb-svc/src/model.rs`），没有产生 ≤0 的通路，但**这条是推理不是实测**：网关 `:3080`、编排 `:3001`、dev `:3020` 本轮全 DOWN，无法现场读一张真实行。
2. **epoch 毫秒写成数字串**（`'1758000000000'`）旧写法 `new Date(串)` 得 Invalid Date 并印出来，新出口先落毫秒数 ⇒ 印正常日期。方向与 `utils/time.js` 头部注释记的历史故障一致（副本拿 `Date.now()` 去减字符串得 NaN），属修好，但确实改了显示，故上表点名。

### 5.27.3 证据链（七枚变异体，全部打红在自己的通道上）

`src/utils/time-locale-outlet.test.js` 7 例（等价 6 样本／hour12 关掉／坏值 null／barrel 与深路径同一函数／数字串不再 Invalid Date／棘轮／8 站点接线），逐枚判决见 `reports/data/time-f2-mutation-witness.txt` 与 `reports/data/barrel-and-deadref-mutation-witness.txt`：

| 变异体 | 打红的通道 | fired 数 |
|--------|-----------|------:|
| M1 登记口 `hour12:false→true` | 等价 + 零点 + 数字串 + **棘轮** | **4**（预注册预测 3，实测 4：改登记口会把该字面量从登记口里移走，而棘轮钉的是"精确站点+次数"，所以它必然同红——预测错在把棘轮当成只看总数） |
| M2 坏值 `null→''` | 坏值契约 | 1 |
| M3 在 `AdminMonitor` 重植一份 F2 字面量 | 棘轮 + 接线 | 2 |
| M4 删 `Dashboard` 的 import 行 | 接线（只此一条） | 1 |
| M5 把 `display.js` 的调用改名 | 接线（只此一条） | 1 |
| M6 `Dashboard` 改回深路径 `@/utils/time` | 接线（证明它认得 barrel 与深路径之差） | 1 |
| M7 删 `ProjectsView` 新加的 helper import | **落库门禁** `check-sfc-dead-refs` rc=1 verdict=FAIL 且点名 `projectMemberCount` | — |

还原他证：`reports/data/barrel-fix-manifest.txt` 记 before→after 尺寸与 sha256 前 12 位；电池结束后用第二工具（独立 `hashlib` 复算）逐一对回磁盘，9 个文件全部 `SAME`；四个被变异体触碰的文件字节级回到电池前（witness 里的 `-> restored … sha=` 行）。

顺带结案一条存量：§5.26 表格最后一行仍写 `deadRefFiles=1（存量 ProjectsView.vue :: projectMemberCount）`——本轮查明它是**模板调用 `projectMemberCount(p)` 但 `<script setup>` 里没有绑定**（函数确实在 `utils/projectMember.utils.js` 且有 12 例测试），即列表页一有数据就渲染期 ReferenceError。补 import 后 `deadRefFiles=0 verdict=PASS`（`reports/data/deadrefs-after.txt`），牙由 M7 证明。

### 5.27.4 门禁与体积

| 命令 | 结果 |
|------|------|
| `node node_modules/vitest/vitest.mjs run src/views src/utils src/modules/_kernel` | `Test Files 26 passed (26)`／`Tests 151 passed (151)` |
| `npx vite build` | `rc=0`，`✓ built in 30.80s`；`@/utils` barrel 边本轮新增 8 条（全库非测试源已有 24 个文件走 barrel，`hitl-ws.js` 的 `new WebSocket` 在方法内非模块顶层 ⇒ barrel 无导入期副作用） |
| 体积（本轮两次 build 逐资产对比） | 全量 raw `6696.8 → 6697.1 kB`（+0.3），`main.js 217.88 → 217.93 kB`（+0.05），`ExpertWorkspaceView 130.39 → 129.66 kB`（−0.73），新增一枚 `projectMember.utils` chunk。**gzip 列本轮解析失败（读数 0.0），故只报 raw，不与 §6 的 gzip 口径混算** |
| `python scripts/gate/check-frontend-module.py` | `rc=0`，`E6 barreled 目录 @/ 深路径导入：0 处 ✓`——本单元一度把它做成 8 处 ERROR（先写 `@/utils/time` 深路径），是门禁把 §4 的规矩教了一遍 |
| `node frontend-ui/scripts/gate/check-sfc-dead-refs.mjs` | `deadRefFiles=0 structFiles=0 verdict=PASS`（上轮 FAIL 的存量本轮清零） |

仪器教训两条：① 棘轮扫全库时**先命中了自己文件的头注释**（注释里把被禁字面量整条抄了一遍）——与 §5.13"禁令正则读注释"同族，说明文字里的反例本身要拼开写；② 驱动器把"校验全部站点"放在写盘之前（两趟），第 5 个站点锚点写错时磁盘一片未动，`D:\tmp\time-f2-pre3` 因此是干净的 5 行清单。

仍欠（不要当成已拿到）：F1 那 7 处的版面改法需产品口径；`/expert-plaza`（#20）与 `v-show` 轮询（#21）仍在裁决；本轮所有面板证据都是编译级＋单测级，**没有真机渲染**（三端口 DOWN，`netstat` 复测见 #22）。


## 5.28 C 族私有词汇副本：先证明"是副本"再合并，工作台上传占位行 3 段 × 2 份收成 `_kernel/upload-row.js`（2026-09-28）

§5.27 之后回到 task #12 的最后一族。这一族的判据不能靠名字像不像，所以要一把按**内容**分组的尺子。

### 5.28.1 普查与分族（本轮现测，扫描集＝非测试源，排除 `utils/`、`constants/`、`modules/_kernel/`）

`D:\tmp\vocab-census.py` → `D:\tmp\vocab-dup.py`（工件 `reports/data/vocab-dup.txt`）：自写"码→中文文案"映射 **45 条 / 22 个名字**，分三档：

| 档 | 判据 | 实测 | 处置 |
|----|------|------:|------|
| (a) 真副本 | 键与文案逐条相同（sha 分组） | **1 组 × 2 站点** | **已收**：`CollaborationPanel.vue:418` 与 `FilePanel.vue:106` 的 `newFile` 占位行 |
| (b) 同键集不同文案 | 键集全等、值不等 | **2 组** | **不并**：`ExpertEnterprisePanel.vue:548/553`（strategy 名 vs 释义，本就是两档版面）；`BrowserView.vue:319/324`（`high/medium/low` 的标签 vs 说明） |
| (c) 键集重叠 ≥50% | 部分重叠、文案不同 | **11 对** | **不算重复**：跨域同键名（`completed/running/…` 在流程/优化器/低码里各说各的话），机械合并会把两个域的口径焊死 |

task #12 记的"C 族 2 处"由此**被现测复核成立**（2 处＝上面那一组两站点），不是虚账。

### 5.28.2 合并的三处函数体与"先证副本"的闸门

驱动器在动手前逐字节比对两份面板里的三个函数体：`getFileType` 574 B、`formatFileSize` 210 B、占位行块 197 B——三处**两文件全等**才写盘（`assert body(tc,nm)==body(tf,nm)`）；校验分两趟，全部站点验完才碰磁盘。产物 `src/modules/_kernel/upload-row.js`（1,628 B，键与文案从原副本字节级照搬，驱动器不重新打中文以免 GBK 控制台把字打坏）；两面板各 −892 B，`handleBeforeFileUpload` 只剩 `const newFile = makeUploadRow(file)` 与原有的 emit/提示。行为零改动：`uploader: '我'`／`time: '刚刚'` 是乐观本地行的占位口径，真实归属与时间要等列表回读覆盖——**这条边界只登记，本轮不动**（它与 §5.23 的"正在显示假数据"是同一族，但那处是读路径丢键，这处是写路径占位，性质不同）。

### 5.28.3 证据

`src/modules/_kernel/upload-row.test.js` 5 例（getFileType 八分支／formatFileSize 档位与零值文案／makeUploadRow 六键六值（`vi.setSystemTime(1790510400000)`，数字纪元避免时区）／两面板接线且不留本地副本／棘轮：`'f-' + Date.now()` 只许活在登记口）。两枚岛屿对照见 `reports/data/upload-row-mutation-witness.txt`：

| 变异体 | fired | 归因 |
|--------|------:|------|
| MU1 登记口 `f-`→`upload-` | **2**（预注册预测 1，实测 2） | 等价例 + **棘轮**：改登记口会把 needle 从登记口移走，棘轮钉的是"needle 唯一的家"，故必然同红——与 §5.27.3 M1 同族，预测错在把棘轮当只看总数 |
| MU2 在 `FilePanel` 注释里植一条 needle | **1** | 只有棘轮红 ⇒ 证明它**读注释**，与 §5.27.4 仪器教训① 互为正反面（说明文字里的反例会被自己扫到，这条现在是设计而非意外） |

还原他证：两文件 sha `1f2f4ac63d44`／`fb363afb20c0` 在 `-> restored` 行与电池后的复算一致，前像在 `D:\tmp\upload-row-pre\`（`.before` + `manifest.txt`，驱动器拒绝覆盖已存在的前像目录）。

门禁：`vitest run src/views src/modules src/utils` = `Tests 843 passed (843)`；`npx vite build` rc=0（`✓ built in 28.33s`）；`check-frontend-module.py` rc=0 且 `E6 = 0 处 ✓`（`@/modules/_kernel/upload-row` 深路径不在 barreled 名单，与 `paged-list`／`nav-icons` 同通道）；`check-sfc-dead-refs.mjs` `deadRefFiles=0 verdict=PASS`。**没有真机渲染证据**：`:3020`／`:3080`／`:3001` 本轮仍 DOWN（#22）。

余账（不许当成本轮已收）：(b) 两组与 (c) 十一对是口径分裂不是重复，要动得先有人裁决每域词表的家；(a) 档的尺子只认"键值全等"，若某天两域真的同键同值而语义不同，它会报成副本——这条误判方向已在表 (c) 处用"重叠但不同值"挡住，但没有更强的尺子。


## 5.29 静态 class 引用的普查尺子（第一版误报 644 个，修正后 230 个）与 §5.25/§5.26 回落分支的两处样式欠账（2026-09-28）

§5.25/§5.26 给未登记图标名加了文本回落分支，但两处只写了类名没写规则——本轮收尾时先用一把尺子去量这类"模板引用了却没有定义的静态 class"到底多大规模。

**尺子本身先错过一次。** 第一版用 `class="([^"]*)"` 抓属性，而 `:class="…"` 的尾巴里就含 `class="` ⇒ 把绑定表达式当静态类名，报 **63 文件 / 644 个**"未定义类名"，其中 `active:`、`===`、`'done'`、`{`、`+` 全是碎片（完整误报读数在会话 stdout，未归档为债表）。修正＝属性名前面不得是 `:`／`-`／字母，且值里含 `{` 的一律不算静态 ⇒ 现测 **71 文件 / 230 个**静态类名在其 SFC `<style>` 与 `src/**/*.css`（6 个全局文件，629 个类）里都找不到（坐标表 `reports/data/static-class-missing-20260929.json`）。

**这 230 个不许当债务总额**：尺子只认"同文件 scoped 样式 + 全局 css"两个来源，第三方组件内部类、由父级 unscoped 样式命中的类、以及写在值里的字符串拼接（`MessageBubble.vue` 的 `'`／`+`／`safeLang` 仍是碎片）都会混进来；逐个判"是不是真没样式"要渲染层证据，而 `:3020`／`:3080` 本轮仍 DOWN。所以本轮只把自己欠的两条收掉，其余登记为待逐文件裁决的余账（与 §5.20 可达性账同类：分子要有第二工具）。

本轮收的两条（都是 §5.25/§5.26 我自己引入的回落分支）：

| 文件 | 类名 | 补的规则 | 依据 |
|------|------|---------|------|
| `views/admin/panels/AdminRole.vue` | `muted` | `.muted { color: var(--text-3); }` | 与 `AdminMenu.vue:472` 同值同写法（那里是权威先例） |
| `views/project/ProjectsView.vue` | `cat-icon-fallback` | `.cat-icon-fallback { font-size: 12px; line-height: 1; }` | 该 span 已用 `:style` 继承分类色，规则只管字号/行高，不重复定色 |

新钉 `src/views/_fallback-classes.test.js`（6 例：4 组 (文件, 类名) 的"用到＋同文件有规则"对、去掉注释后仍判得出定义存在、两处回落分支仍挂着 `nav-icons` 解析）。牙：两枚"删掉刚补的那条规则"的变异体各自打红 2 例（配对例＋去注释例），还原 sha `c5090a0604ff`／`253267048968` 见 `reports/data/fallback-class-mutation-witness.txt`。

测试自身的两处失败先记账再修：① 我写了 `t.rindex('<style')`（Python 习惯）而不是 `t.lastIndexOf`，一跑就全红——TypeError 不是判据红，是仪器坏；② 断言写成 `toContain('from nav-icons')` 而真实 import 是 `from '@/modules/_kernel/nav-icons'`，子句不存在。另有一对 `(ProjectsView, muted)` 在删掉之前没验过"模板是否真用"，直接从表里撤下而不是留着赌它绿。

门禁：`check-sfc-dead-refs.mjs` `verdict=PASS`、`check-frontend-module.py rc=0`、`npx vite build rc=0`（`✓ built in 31.43s`）；`_fallback-classes.test.js` 单跑 6 例全绿；`src/views src/utils src/modules` 整跑通过（该次 `tail -3` 只截到 `Duration` 行，判决行的用例数未被读到 ⇒ 数字不复述，要引用请复算）。


## 5.30 F1 族收口：4 份逐字符相同的 `fmtTime` 副本 → `formatDateTimeLocaleOr` 单出口（2026-09-29）

§5.27 收的是 F2（写死 `zh-CN` 的那一把），F1 一族当时登记 10 处却一条没动。本轮按 §5.28 的判据重看这一族：4 份副本块体逐字符相同（sha256 前 12 位 `4117530d3121` × 4，实测见 `reports/data/time-f1-collapse.txt`），属 (a) 类真重复，收；其余 6 处三种契约各不相同或与时间无关，只分类不动（`reports/data/time-f1-split.txt`：时间戳 3 ＋ 数字千分位 3）。

**这一族的缺陷不是重复而是口径**：副本写的是无参 locale 格式化，输出跟随访问者浏览器区域设置——同一张审计表在 zh-CN 浏览器是 `2026/9/28 02:13:51`，在 en-US 浏览器是 `9/28/2026, 2:13:51 AM`。归一到固定 `zh-CN` ＋ 24 小时制后与 F2 出口共用同一"能解析时怎么印"。

两条本轮量出来的事实（都写进了出口 docstring）：

- 副本里的 `catch { return String(t) }` 是**死支**——坏串走 `new Date(...)` 再格式化不抛，返回字符串 `Invalid Date`。所以界面上真实出现过的契约就是"坏值印 Invalid Date"，收口原样保留它（新档 `formatDateTimeLocaleOr(ts, empty = '-')`：falsy→占位符，解析失败→`Invalid Date`），只去重复不改契约。
- 无参写法对**负 epoch** 会印 1969 年的日期，新档回到 `Invalid Date`；对**数字串**（`'1758000000000'`）旧写法是 `Invalid Date` 而新档经 `timeValue` 解析成真日期。后者与 §5.27 同一对delta，前者是本轮新增的唯一有意显示变更（zh-CN 客户端逐字符不变）。

收口形态用的是**别名导入**（`import { formatDateTimeLocaleOr as fmtTime } from '@/utils'`），模板里 5 个 `fmtTime(` 调用点一行没改；四份文件各 −59 B。落盘点：`views/admin/panels/{AdminAccess,AdminAudit,AdminConfig,AdminMenu}.vue`。

钉子 `src/utils/time-f1-outlet.test.js`（6 例，与 F2 的 7 例同跑 13 例全绿，`reports/data/time-f1-suite.txt`）：与 F2 出口逐字符相等／无值档（6 个 falsy × 两种占位符）／坏值档＋死支见证／barrel 同一函数／棘轮（余量按**文件×次数**登记，总数断言 6）／4 个收口点真的引到出口且旧副本体不在文件里。

棘轮登记表选"文件＋次数"而不是 `file:line`，本轮就被动验证了一次：`ExpertPlazaView.vue` 的坐标从普查时的 :672 漂到 :698，而钉子仍绿——行号在这里本来就不是判据。

变异电池 6 枚全 CAUGHT（`reports/data/time-f1-mutation-witness.txt`，基线 13 passed，逐枚还原后 sha 复算 4/4 IDENTICAL）：

| 针 | 撤掉的通道 | 结果 |
|----|------|------|
| MU1 坏值回落空串 | `?? 'Invalid Date'` | rc=1，坏值档 1 例红 |
| MU2 无值判据收窄成 null/undefined | `if (!ts)` | rc=1，无值档 1 例红 |
| MU3 默认占位符改成空串 | `empty = '-'` | rc=1，无值档 1 例红 |
| MU4 AdminMenu 整体退回写前图像（副本复活） | 副本＋无出口引用 | rc=1，棘轮＋接线 2 例红（期望 2，命中） |
| MU5 island：只撤 AdminAccess 的出口引用 | 接线半边 | rc=1 接线 1 例红 **且** `check-sfc-dead-refs.mjs` 报 `deadRefFiles=1 verdict=FAIL`（模板裸调这类缺陷由编译级门禁独立接住） |
| MU6 island：只撤 barrel 再导出 | 出口半边 | rc=1 barrel 同一函数 1 例红 **且** `vite build` rc=1 点名消费者 `AdminAccess.vue`（§5.3 那条"改导出名的验收命令是 rollup"再次成立） |

门禁复算：`python scripts/gate/check-frontend-module.py` ERROR=0 rc=0；`node frontend-ui/scripts/gate/check-sfc-dead-refs.mjs` `files=128 checked=126 deadRefFiles=0 structFiles=0 verdict=PASS`；`npx vite build` rc=0（38.65 s）。产物 `reports/data/time-f1-{collapse,suite,gov,deadref,build,split}.txt`。

**本轮自伤一条，如实登记**：电池 v1 把"片段"当"整文件"写（`open(path,'wb').write(fragment)`），把当时**尚未入库**（`?? frontend-ui/src/utils/time.js`，git 无恢复源）的 `utils/time.js` 削成 53 字节。恢复源只剩本轮早些时候的整文件 Read 转写；重建后 13 例全绿（F2 那枚棘轮要求 `utils/time.js:1` 恰好 1 次命中，等价于对内容重新把过一次关）。教训与修法：电池开局必须把**每个被碰文件**整份备份（v2 已实现，目录存在即拒绝覆盖），未入库文件没有任何 git 图像可退。

仍欠：F1 时间戳余账 3 处——`widgetRegistry.js:12`（低码引擎自带"坏值原样回显"契约，与面板不同）、`AdminHitl.vue:168`（`ts * 1000` 秒制，口径问题在单位而非格式）、`TaskView.vue:425`（把显示串 `created_at` 写进本地新建行的数据里，属 wire 侧缺陷不是格式缺陷）；三处均只报不改，登记 task #24。vendor-element JS 本轮 665.17 kB，比 §5.27 记录的 659.3 kB 高 5.87 kB，而本轮源码净减 236 B——差额不由本单元解释，留作下一轮基线归因项。



## 5.31 时钟档／日期档收口：`toLocale*` 全家族普查翻出尺子的盲区，10 处副本进两档出口（2026-09-29）

§5.10／§5.27／§5.30 三轮的时间副本普查都只扫 `toLocaleString`。本轮把 needle 换成 `.toLocale(String|DateString|TimeString)(` 全家族，实测 **20 处**：先前登记的 10 处之外，另有 **14 处带参调用**（`reports/data/locale-family-census.txt`，其中含上面那 10 处的重叠口径，逐条坐标与整行原文在同份产物里）——联盟工作台、`alliance.store.js`、`expert-alliance/model/rank.js` 全在这批没被任何一轮看过的代码里。**这是"判据的覆盖面按形状算、不按文件数算"的第 32 条在本仓库的第二次实证：三轮绿灯的普查，盲区是 needle 自己。**

分类后收口 10 处逐字符副本（`reports/data/locale-clock-manifest.txt` 现场数出）：

- 时钟档 `new Date().toLocaleTimeString('zh-CN', { hour:'2-digit', minute:'2-digit' })` **7 处**：`composables/workspace/useAlliance.js` ×1、`stores/alliance.store.js` ×3、`views/workspace/ExpertWorkspaceView.vue` ×2、`views/project/Workbench.vue` ×1。
- 日期档 `.toLocaleDateString('zh-CN')` **3 处**：`modules/expert-alliance/model/rank.js` ×1、`Workbench.vue` ×2。

新增两档 `formatClockMinute(ts = Date.now(), invalid = 'Invalid Date')` 与 `formatDateStamp(...)`（同进 `utils/index.js` barrel）。**这一族原本就写死 `zh-CN`，所以本单元零版面变更**——与 §5.30 那种"把 en-US 浏览器看见的字也改了"不同，这里纯粹去重复；坏值仍印 `Invalid Date`（沿用副本原可见行为）。字节账：`useAlliance.js` −15、`alliance.store.js` −133、`ExpertWorkspaceView.vue` −74、`Workbench.vue` −40、`rank.js` +21（出口名比原表达式长，唯一一份变长的）。

钉子 `src/utils/locale-clock-outlet.test.js`（8 例）：与旧表达式逐字符相等×两档／时钟档"整串只有一个冒号"＋默认参 `^\d{2}:\d{2}$`／日期档不含冒号／坏值档＋可改名／barrel 同一对函数／棘轮 A（选项字面量与日期档调用式各只许 `utils/time.js:1`）／棘轮 B（时钟式调用全库余量按文件登记＝出口 1＋待裁决 2，总数断言 3）／5 个收口文件接线。
本文件把被禁串全部拆开写、并用 `Object.fromEntries` 造选项、用 `new Date(v)[M_CLOCK](...)` 计算属性调用重建旧表达式——否则钉子会因自己的文本假红（§5.27 是在注释里踩的，这次提前防住）。

`src/utils` 目录整体复跑 **5 文件 / 57 例全绿**（含存量 `time.test.js` 24 例，`reports/data/locale-clock-suite.txt`）⇒ 收口没碰坏既有口径。

变异电池 5 枚全 CAUGHT（`reports/data/locale-clock-mutation-witness.txt`，基线 3 文件 / 21 例，逐枚还原＋sha 复核 3/3 IDENTICAL）：MB1 时钟档选项丢 `minute`、MB2 坏值退化成 `null`、MB3 默认 invalid 串改空串、MB4 island 只撤 `ExpertWorkspaceView` 的出口引用、MB5 island 只撤 barrel 的日期档再导出（**同时** `vite build` rc=1，再次证明改导出名的验收命令是 rollup）。**预测错一处如实登记**：MB1 期望 1–2 例红，实测 3——第三枚是棘轮 A 也吃下了这个改动（选项字面量被改后登记口不再匹配），属"同一枚针打红两条独立通道"，不是漏判。

门禁：`check-frontend-module.py` ERROR=0 rc=0；`check-sfc-dead-refs.mjs` `deadRefFiles=0 structFiles=0 verdict=PASS`；`vite build` rc=0（37.15 s，`reports/data/locale-clock-build.txt`）。

**第二起自伤，也要如实登记**：本轮收口驱动器在**所有源文件写完之后**、写账之前崩溃（`b".toLocaleDateString(" + "'zh-CN')"`——bytes 拼 str，TypeError，traceback 存 `reports/data/locale-clock-collapse.txt`）。后果是"活落盘了但没有任何清单"。修法不是重跑驱动器（那会把已改过的图像再套一遍），而是**独立测量脚本**拿 PRE 里的写前图像与磁盘现像逐文件对账并补写 manifest（`D:\tmp\locale-clock-measure.py`，明文承诺"绝不改写源文件"）。与 §5.30 那起合起来是一条流程规则：**先写完账再落盘**，以及"验证阶段就要把清单代码本身当被测对象跑一遍最小样例"。

余账（本轮只分类不动手）：时钟带秒档 2 处（`AdminMonitor.vue:1138` 写 `hour12:false`、`Workbench.vue:157` 只写 `'zh-CN'` 没显式 pin）、`ExpertEnterprisePanel.vue:680` 的 `toLocaleString('zh-CN')` 同样没 pin `hour12`、数字千分位 3 处（`AdminLlm.vue` ×2、`ExpertPlazaView.vue`，与时间无关的合法用途）；加上 §5.30 的 F1 时间戳 3 处 ⇒ 时间口径余账共 9 处，登记 task #24／#25。



## 5.32 把普查变成常驻闸门：`check-locale-format-outlets.py`（2026-09-29）

§5.31 的教训不是"还剩 10 处副本"，而是**人工普查的覆盖面由针的形状决定**：连续三轮都只扫 `toLocaleString`，于是同一族的 `toLocaleTimeString`／`toLocaleDateString` 全程隐身。一次性普查不能防回潮，所以这一轮把普查本身落成常驻门禁。

`scripts/gate/check-locale-format-outlets.py`（10378 B，房式沿用 `check-frontend-module.py`：docstring 写检查项／用法／退出码）。四条判据 + 两条 INFO：

- **L1 UNPINNED_DATE**（棘轮）：无参 `toLocaleString()` 且接收者是 `new Date`（同一行前缀，或 `X = new Date(` 的别名变量）⇒ 输出跟随访客浏览器 locale，属缺陷。与 `EXPECTED_UNPINNED` 登记表按 **文件 × 次数** 比（不用 `file:line`——§5.31 已实证行号会被自己的编辑移动）：`extra`/`grown` = ERROR，`gone` 只在余账里点名（清零后要**删条目**而不是写 0）。现登记 3 处：`widgetRegistry.js`、`AdminHitl.vue`、`TaskView.vue`（task #24）。
- **L2 OUTLET_LITERAL**：出口写法串只许活在登记口。只枚举两条完整写出形态（`{ hour: '2-digit', minute: '2-digit' }` 与 `toLocaleString('zh-CN', { hour12: false })`），不枚举局部片段。
- **L3 LOCALE_PIN**：带 locale 首参但不是 `zh-CN` ⇒ ERROR（防止把口径悄悄换成浏览器的另一套）。
- **L4 SCAN_SET**：被扫文件数 ≥ `MIN_FILES`。判集塌缩时"0 命中"与"没有缺陷"同形，所以分母本身是断言；现测 **270 个文件**，下限钉 100。
- **I1** 数字千分位无参调用（与时间无关的合法用途）按名点名不判定；**I2** 登记口外的带参 `toLocale*` 直写 6 处按名点名（task #25 余账），使"本轮只分类不动手"的余账由机器列出，而不是由我手抄。

证据（`reports/data/locale-gate-selftest.txt`、`reports/data/locale-gate-check.txt`、接线复跑见证 `reports/data/locale-gate-wiring.txt`）：`--selftest` **11/11 PASS，rc=0**，其中 4 枚是"合法数据不许红"的反对照（登记口自身／数字千分位／pin 到 zh-CN 的日期档／带秒档自己写 `hour12`），1 枚是空判集正对照（把仓库根指向不存在的目录 ⇒ rc=1），另有 L1 两种接收者形态各一枚、L2 两种出口写法各一枚、L3 一枚；`--check` **rc=0**，输出即上面那份余账。内存内正对照两枚：植入一枚新站点 ⇒ FIRED；把登记口自己拿来喂 ⇒ CLEAN。

**第一版两处闸门缺陷，如实登记**：① `MIN_FILES` 是凭手感钉的高值，而真分母要由现场测量决定——自检立刻红，修法不是抬高期望而是把实测值印进输出（现在每轮打印 270）；② L2 最初枚举局部片段 `{ hour12: false }`，把 `AdminMonitor.vue:1138` 那处**合法的带秒档自写**误判成 ERROR。零容忍判据下假阳与假阴同价，②的代价尤其直接：它会把人推向"绕过闸门"而不是"满足闸门"。两条都靠反对照那次自检才发现，不是靠真库绿。

## 5.33 `TaskView` 的写侧污染收口：展示串不许写进 `created_at`（2026-09-29）

§5.31 把这条登记成"下一枚针"，本轮拔掉。`views/project/TaskView.vue` 原来在乐观写入列表行时写
`created_at: new Date().toLocaleString()`（原 `:425`）——**不是展示口径问题，是把浏览器 locale 的展示串写进数据字段**。

代价比版面大：网关把 `created_at` 当 `String` 存（`platform/gateway/mox-platform-gateway-svc/src/misc.rs:35`），
默认按**字典序**排序（同文件 `:332`，`b.created_at.cmp(&a.created_at)`），服务端自己造的值是 RFC3339
（同文件 `:122`，`Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)` ⇒ `2026-09-28T23:59:10Z`）。
`2026/9/29 08:01:51` 这种串混进同一列，该行的排序位置永久错位，且它覆盖了 `createTask()` 的服务端回显。

改法（两处，一行一句）：写侧 `created_at: newTask.created_at || new Date().toISOString()`（服务端回显优先，只在回显缺席时造 ISO，与 `ExpertWorkspaceView.vue:429`、`WorkflowView.vue:407` 等 8 处既有房式一致）；
展示侧 `{{ currentTask.created_at }}` → `{{ formatDateTimeLocaleOr(currentTask.created_at) }}`（走 §5.30 的单出口，barrel 导入）。
**展示版面**：新建行仍显示同样的本地挂钟时刻（出口就是干这个的），差别在于字段内部值从展示串变回 RFC3339，且服务端已有值时不再被客户端时刻顶掉。

**同一事实有两本台账，本轮同时被要求收缩**：常驻闸门的 `EXPECTED_UNPINNED` 与 `src/utils/time-f1-outlet.test.js` 的 `REGISTERED`。
删掉站点后两本都红（闸门 `gone` 属咨询项、F1 棘轮 `expected 5 to be 6`），这是见证不是事故。修的时候顺手去掉一个手抄数字：
`REGISTERED_TOTAL` 改为由台账求和导出（两处副本会腐烂，逐文件计数另有 `toEqual` 钉住）。

**闸门的 advisory 通道没有牙，这轮给它补了一枚**：`check-locale-format-outlets.py` 对"条目该删"只打印 `[INFO]`／`[SHRANK]` 而 rc 仍为 0 ⇒ 腐烂的台账在 CI 里静默。
钉子 `src/views/project/taskview-created-at.test.js`（5 例）里第 5 例交叉核对闸门台账：已清条目不许还在、其余条目指向的文件必须仍有该形状（必要条件检查，不是充分条件——它抓"该删没删"和"指向已不存在"）。

证据（`reports/data/taskview-{suite,gate,build,mutation-witness}.txt`）：基线 **62 例 / rc=0**，闸门 **rc=0 且 L1 余账 3→2**（TaskView 条目由机器列出，不再由我手抄），`vite build` **rc=0**（`TaskView-9JQHWEoZ.js 12.02 kB`）。
变异电池 **5/5 CAUGHT、5/5 restored=IDENTICAL**（sha 复验）：MT1 写侧退回展示串 → **3 例红 + 闸门 rc=1**（唯一能进 CI 的那枚，也是它的牙齿所在）、MT2 展示侧退回直出 → 1 例红、MT3 撤 barrel 导入 → 1 例红、MT4 把已清条目加回闸门台账 → 1 例红而**闸门 rc=0**（正是补上 advisory 盲区的那枚正对照）、MT5 F1 台账计数写错 → 棘轮 1 例红。`check-frontend-module.py` ERROR=0、`check-sfc-dead-refs.mjs` verdict=PASS。

**我自己写坏的一枚针，登记为流程教训**：第 4 例最初把期望钉成 UTC 日（`2026-09-28`），而出口渲染**本地挂钟**（本机 +08:00 ⇒ 该瞬间落在 09-29）⇒ 假红。
改为钉"形状 + 指回同一瞬间"（`new Date(Y,Mo,D,H,Mi,S).getTime() === Date.parse(wire)`），时区不再进入期望。**口径出口的期望不许假设时区**，与 §5.31 那条"硬编语言惯用串会在另一语种假红"同族。

余账更新：task #24 从 3 处变 **2 处**（低码引擎 `widgetRegistry.js` 的原样回显契约、`AdminHitl.vue` 的秒制单位），两者都是展示契约差异而非写侧污染。

## 5.34 带秒时钟档：I2 六处直写的实测与三处收口（2026-09-29）

§5.32 的闸门把"登记口外的带参 `toLocale*` 直写"按名点名成 6 处（I2 通道）。本轮先**量**再动：
选项对象用大括号配对从源码里截出后在 Node 里求值（脚本 `D:\tmp\i2-census.mjs`，未落库⇒没牙），三个样本含午夜与一个跨 UTC 日的瞬间，判"与哪一档出口逐字符相同"。

| 站点 | 旧写法 | 实测 | 判决 |
|------|--------|------|------|
| `ExpertEnterprisePanel.vue:680` | `now.toLocaleString('zh-CN')` | 与 `formatDateTimeLocale`/`…Or` **逐字符相同** | 零变更收口 |
| `AdminMonitor.vue:1138` | `new Date().toLocaleTimeString('zh-CN', { hour12: false })` | `09:05:09`／`00:00:00` | 与下一行**同档** |
| `Workbench.vue:158` | `new Date().toLocaleTimeString('zh-CN')` | 输出与上一行**完全相同** | 两份副本→新增第三档 |
| `AdminLlm.vue:647` | `{ month, day, hour, minute, second }` | `09/28 09:05:09` | 档不同，**不并** |
| `MarketView.vue:608` | `{ year, month, day, hour, minute }` | `2026/09/28 09:05` | 档不同，**不并** |
| `BrowserView.vue:586` | `{ year, month, day, hour, minute, second }` | `2026/09/28 09:05:09` | 档不同，**不并** |

**这处测量纠正了 §5.32 的分类**：`zh-CN` 在这台 V8 上默认就是 24 小时制（实测 `toLocaleTimeString('zh-CN')` ⇒ `09:05:09`），
所以"未 pin `hour12`"**不是版面缺陷**，而是把 24 小时制押在 locale 默认值上——契约问题不是显示问题。
据此新增出口第三档 `formatClockSecond(ts, invalid)`（`{ hour, minute, second }` 全 `2-digit`，pin `zh-CN`），
把两处同档副本收进去；`AdminLlm`／`MarketView`／`BrowserView` 三处字段集互不相同，**机械替换会改版面**，按 F3 规矩定价不并。
收口 3 处的展示版面变化＝**0**（每一档都有逐字符相等钉子）。

**同一句 import 被第三本台账钉住**：老钉子 `src/utils/time-locale-outlet.test.js` 的"8 个收口点都真的引到了出口"
用 `toContain("import { formatDateTimeLocale } from '@/utils'")` 判接线——我在同一句里加一个同伴名字（`formatClockSecond`）就红。
这不是缺陷而是判据过窄：接线要钉的是"这个名字经由 barrel 进来"，不是"这一句里恰好只有它"。改成
`/import \{[^}]*\bformatDateTimeLocale\b[^}]*\} from '@\/utils'/`。**台账越像整句原文，跨单元的合法改动就越容易撞红。**

证据：`reports/data/clock-second-{suite,gate,build,mutation-witness}.txt`。
基线全量 vitest **75 文件 / 1000 例 / rc=0**（分母又变，见 [[project-frontend-count-authority]]），`vite build` rc=0（28.12 s），
`check-frontend-module.py` ERROR=0，`check-sfc-dead-refs.mjs` verdict=PASS，locale 闸门 rc=0 且 **I2 由 6 处降到 3 处**（机器列名，不手抄）。
变异电池 5/5 CAUGHT、5/5 restored=IDENTICAL：MC1 带秒档丢 `second`（4 例红）、MC2 `hour` 变 `numeric`（3 例红）、
MC3 站点退回自写带秒式（2 例红：**本次新档的接线针 + §5.31 的时钟棘轮 B**）、MC4 barrel 不再导出该档（3 例红 **+ `vite build` rc=1**，
rollup 点名 `src/views/project/Workbench.vue (12:28): "formatClockSecond" is not exported by "src/utils/index.js"`——第三次印证"改导出名的验收命令是 rollup"）、
MC5 EEP 退回未 pin 整串（1 例红）。
**MC1/MC2/MC3/MC5 的闸门 rc 都是 0**：闸门是**用法棘轮**（谁在登记口外直写），不审出口内部行为，
出口内部靠 `src/utils` 的相等性钉子；两者不互相替代。MC3/MC5 里闸门唯一的反应是 I2 从 3 变 4 的 INFO 点名——与 §5.33 的 MT4 同型：**advisory 通道没有牙，牙在测试里**。

余账更新：时间/locale 口径的**待裁决直写**由 9 处（§5.32 记）变 **5 处**＝L1 2 处（低码回显契约、`AdminHitl` 秒制单位）+ I2 3 处（本轮判"档不同、不并"）。

## 5.35 F4 手工拼接档收口：只并"逐字节相同"的三处，其余五处登记为档不同（2026-09-29）

§5.31 数过 F4 族（手工拼接日期/时间数字：读 `getXxx()` 再 `padStart`／模板插值）却一直没并。本轮先量再动，
普查器 `D:\tmp\f4-census.py`（只读、未落库＝没牙，**不要接 CI**）现测：**349 个文件、10 处站点、6 种字段组合**
（`reports/data/f4-census-20260929.txt`）。逐处比对后只有三处的输出与现有出口档**逐字符相同**，其余五处是版面不同的另一档：

| 站点 | 旧写法形状 | 裁决 |
| --- | --- | --- |
| `components/MessageBubble.vue:944` | `pad(H)+':'+pad(M)` | 并到 `formatClockMinute`（零变更）；同行 `:946` 的 `MM-DD HH:mm` 组合档**不并** |
| `composables/workspace/useWorkspaceData.js:42` | `H..toString().padStart+':'+M..` | 并到 `formatClockMinute()`，`nowTime()` 纯别名随之删除（4 个调用点直接引出口） |
| `components/FlowDetailDialog.vue:183` | `YYYY-MM-DD HH:mm` | 并到 `formatDateTime(d, '—')`；**坏值原样回显** `String(s)` 那一支留着——出口只给空态文案，给不了"把传入串印回去" |
| `views/admin/panels/AdminLogs.vue:128` | `HH:mm:ss.SSS`（带毫秒） | 不并：出口族没有毫秒档，为单点造档＝造第二个事实源 |
| `views/ai/ChatView.vue:178` | 相对档尾巴 `M/D`（不补零） | 不并：与 `relativeTimeText` 有一处**真实行为差**——未来时刻这里落进 `diff<60000` 印"刚刚"，出口印"稍后" |
| `views/expert/ExpertPlazaView.vue:1013` | `YYYY-MM-DD`（连字符＋补零） | 不并：`formatDateStamp` 走 locale，印 `2026/9/29`，斜杠与补零都不同 |
| `views/expert/panels/ExpertEnterprisePanel.vue:854` | `M/D HH:mm`（月日不补零） | 不并：组合档，出口族无对应形 |

**版面变更只有一处，且是有意保留的口径**：`formatDateTime` 对 `t≤0` 回到空态文案，而旧写法会印出 `1970-01-01 08:00`
这一类日期。这与 §5.32 那条"坏值不再印 1970"是同一条契约，本轮把它钉进测试而不是留给记忆。

**见证方式换了**：前几轮的等价性是把手工写法**抄**进测试文件，于是抄错一个字符就变成"测试与出口都错"。本轮改成从
写前图像的字节里**现推**——`D:\tmp\f4-probe2.mjs` 在备份文件里按正则找回那一行，剥掉 `const tp =`／`return ` 前缀，
用 `new Function('t', …)` 就地造出旧函数，再与出口逐时刻比（6 个正时刻全 EQ，`reports/data/f4-preimage-witness-20260929.txt`）。
同一台仪器喂**收口后**的盘必须报"锚点行命中 0 ⇒ 无效见证"并且 rc=1（`reports/data/f4-instrument-control-20260929.txt`），
这才说明它读的是磁盘而不是期望。

**棘轮落进测试**：`src/utils/f4-hand-assembly.test.js`（8 例）自带同款扫描针，按**文件×次数**登记余账 6 处
（登记口 `utils/time.js` 1 ＋ 不并 5），总数由登记表 `reduce` 现推，判集分母每次印出实测值（**270**）而只设塌缩下限 100。

本轮撞出的仪器教训：**普查器与棘轮的判集口径必须一致，否则收口看起来像没做**——含测试文件的旧口径下
收口前 348 文件/10 处、收口后 349 文件/**10 处**（新测试文件里为了现推旧写法而写的副本被算成余账，抵掉了真删的 3 处）；
加上与闸门 L4 同口径的 `.test.js/.stories.js` 过滤后才是 **270 文件 / 6 处**（前值 9 处＝6＋本轮并掉的 3，由写前图像见证）。
另有一次自己踩到的"分母凭手感钉"：把下限写成 `>300`（我按普查器的 348 估的，而针的判集是闸门的 270）⇒ 自检立刻红，
改回"印实测值＋塌缩下限 100"。

证据与门禁：`reports/data/f4-census-20260929.txt`（收口前原始普查，含测试文件口径）、`reports/data/f4-census-after-20260929.txt`（收口后，闸门口径 270/6）、
`reports/data/f4-preimage-witness-20260929.txt`（旧写法从写前图像现推的相等性）、`reports/data/f4-instrument-control-20260929.txt`（同一仪器喂收口后盘必须 rc=1）、
`reports/data/f4-mutation-witness-20260929.txt`（6/6 命中＋反对照＋还原复验）。
全量 vitest **76 文件 / 1008 例 / rc=0**（上轮 75/1000，本文件 +1/+8），`vite build` rc=0（38.96 s），
`check-frontend-module.py` ERROR=0，`check-sfc-dead-refs.mjs` verdict=PASS，locale 闸门 rc=0 且自检 11/11 PASS（L1 仍 2 处、I2 仍 3 处，本轮未动这两本账）。
变异电池 6/6 各自打红指名用例、1 枚反对照必须绿且真的绿、5 个被碰文件逐字节还原=IDENTICAL：
F4-M1 站点退回自写时钟式→棘轮台账红；F4-M2 空态文案 `'—'`→`'-'`→FlowDetailDialog 契约红；F4-M3 改走 `@/utils/time` 深路径→barrel 接线红（E6 同型）；
F4-M4 **出口内部**月份丢 `+1`→相等性红（证明牙也在出口这一侧，不只在使用点）；F4-M5 退回 `nowTime()` 纯别名→"不留一行纯别名"红；
F4-C1 在 `ChatView.vue` 插一行"读字段但无拼接痕迹"（`d.getFullYear() - 2000`）→必须不红且真的不红，钉住针的拼接掩码没有过度计数。
⚠️ 本轮改动全部只在工作区：`utils/time.js`、`scripts/gate/check-locale-format-outlets.py`、`f4-hand-assembly.test.js` 等为未跟踪文件＝**没有 git 恢复源**，
写前图像在 `D:\tmp\f4-pre\`（sha1 前 12 位印在变异日志首行），入库需用户点名文件（task #19）。

**顺手量了下一族但没有动代码**：F5＝自写 `formatTime`／`relativeTime` 函数定义，常驻账在 `src/utils/time.test.js` 的 `KNOWN_COPIES`
（当轮实测 **12 文件 / 13 处**，其中 `modules/expert-alliance/model/display.js` 占 2 处且被注释称为"权威"）。等价性用同一台"从磁盘现推"的仪器量得
（`reports/data/f5-relative-ladder-probe-20260929.txt`）：`display.js` 的 `relativeTime` 与出口 `relativeTimeText` 在 14 个入参形状上 **13 同字 / 1 异字**，
唯一异字是 **epoch 毫秒数字串**（盘上回空串、出口回"9 分钟前"，且出口才对）⇒ 委托方向近乎零变更；但成对的 `sessionTimeText = relativeTime ‖ formatTime`
不能整对替换——`formatTime` 的绝对尾巴是 locale 档且**坏值原样回显**，而出口的 `timeAgoOrDate` 走表格档 ⇒ 又一处"回显"契约（本轮 `FlowDetailDialog` 已撞见一次）。
是否为此加第六档属产品口径，留给用户裁决；接手工作单见 task #27。

## 5.36 F5 自写时间副本收口：13 处→8 处，五处并、八处登记（2026-09-29）

**判决**：全库自写 `formatTime/relativeTime` 定义实测 13 处（账在 `src/utils/time.test.js` 的 `KNOWN_COPIES`），
本轮收口 5 处，余 8 处全部登记为"档不同不并"。普查判据不是函数体像不像，而是**把每处定义从磁盘截出、
在 `new Function` 里注入它自己引用的出口真身，逐 15 个输入形状比对输出**（驱动器 `D:/tmp/f5-family.mjs`，
**未落库＝没牙，别接 CI**；读数挂在 `reports/data/f5-family-census-20260929.txt`）。

| 站点 | 处置 | 根据 |
| --- | --- | --- |
| `modules/expert-alliance/model/display.js` 的 `relativeTime` | **删除，委托 `relativeTimeText`** | 13/14 形状逐字同；唯一异形是 epoch 毫秒数字串——旧副本 `Date.parse` 读不出恒空转，出口读得出 |
| `views/admin/panels/AdminDepartment.vue:621` | **收成 `import { formatDateTimeLocaleOr as formatTime }`** | 它本就在调 `formatDateTimeLocale`，只多道 `new Date(t)`，于是数字串形状退化成 `Invalid Date` |
| `views/admin/panels/AdminRole.vue:892` | 同上 | 函数体逐字节同（组 `51cf6ce1462a`，3 处一份） |
| `views/admin/panels/AdminUser.vue:690` | 同上 | 同上 |
| `views/expert/panels/ExpertOrchestratorPanel.vue:474` | 同上（组 `45dcb8bd0acd`） | 与出口只差同一道 `new Date` 预转换 |
| `components/MessageBubble.vue:936` | 登记不并 | 复合档：当天 `HH:mm`、跨天 `MM-DD HH:mm`、坏值空串 |
| `modules/expert-alliance/components/ExpertBookingPanel.vue:107` | 登记不并 | 带秒全时刻档（`formatDateTimeLocale` 无秒） |
| `modules/expert-alliance/model/display.js:8` `formatTime` | 登记不并 | **坏值原样回显**契约（第二次撞上这一档，见下方卡点） |
| `views/admin/panels/AdminLlm.vue:643` | 登记不并 | `MM/DD HH:mm:ss` |
| `views/admin/panels/AdminSso.vue:194` | 登记不并，**且报缺陷** | 纯字符串手术：`replace('T',' ')` 印的是**服务端 UTC 挂钟**，与全站本地挂钟差一个时区偏移（本机实测 01:26 vs 09:26），且不处理非 RFC3339 入参 |
| `views/ai/ChatView.vue:170` | 登记不并 | `M/D` 日期档（无年份） |
| `views/expert/panels/ExpertEnterprisePanel.vue:851` | 登记不并 | `M/D HH:mm` 短档 |
| `views/workflow/BrowserView.vue:583` | 登记不并（§5.34 I2 已记） | `YYYY/MM/DD HH:mm:ss` |

**版面变更只有两笔，且都是修坏值**：① 会话行"最近活跃度"在 epoch 毫秒数字串输入下从空白变成 `N 分钟前`；
② 三个 Admin 面板 + 编排面板的时间列在该形状下从 `Invalid Date` 变成可读时刻。其余站点逐字不变。

**常驻守卫（新增用例，全在既有测试文件里，不新建第二个权威源）**：
`src/utils/time.test.js` 的 F5 段——委托接线（barrel 取名 + 禁深路径 + 调用形）、出口四档逐字（含 `7 分钟前` 的空格）、
别名收口的四处、以及那唯一异形的定价；`src/utils/time-locale-outlet.test.js` 原来的"8 个收口点"整表被拆成
**直调出口**与**别名 import**两组（别名把旧组的整句判据打红＝第二本台账腐烂，见下）。
消费侧接线证据在 `src/views/workspace/workspace-sessions.test.js`（相对档四档从会话这一行真的打得通）。

**仪器教训（三枚，都记进 [[gate-expectations-must-be-derived]] 家族）**：
1. **普查仪器必须先解决被检对象的依赖闭包**。第 1 轮 MessageBubble 那处在 15 个形状上全输出空串，看着像"档完全不同"，
   真相是仪器没给它 `formatClockMinute`，函数自己的 `try/catch` 把 ReferenceError 吞成了空串 ⇒ 补 SHIM（注入真身出口）后判决才有意义，
   并在输出里按名打印 `自吞异常=yes(全异可能是仪器假账)` 当红旗。
2. **收口后的第二本台账会自己红给自己看**：全量 vitest 第一次 rc=1 的唯一失败是 `time-locale-outlet.test.js` 那条整句判据——
   它把"引到出口"钉成 `formatDateTimeLocale(d)` 这一形，而别名收口引的是 `…Or as formatTime`。这是同一事实的两本账，
   必须同轮改对（改法：拆两组各钉各的形，不是把判据放宽）。
3. **过滤器静默 0 行不等于判决**：门禁日志里 `grep -E "^ Test Files"` 在管道里拿到 0 行（GBK 控制台＋转义序列），
   单独复跑才读到 `76 passed / 1016 passed / rc=0`。分母必须来自能打印出它的那次运行。

**门禁（本轮实测，读数见 `reports/data/f5-gates-20260929.txt` 与普查文件末尾）**：
全量 vitest **76 文件 / 1016 例 / rc=0**；`vite build` rc=0（35.00 s，别名 import 的消费者账只由 rollup 验）；
`check-locale-format-outlets.py --check` rc=0（L1 登记 2 处、L4 扫描集 270）、`--selftest` PASS；
`check-frontend-module.py` ERROR=0；`check-sfc-dead-refs.mjs` deadRefFiles=0 PASS。
**变异电池 6 枚**（`reports/data/f5-mutation-witness-20260929.txt`）：M1 抄回副本→台账红、M2 会话行丢相对档→消费侧红、
M3 出口丢空格→字面红、M4 撤别名→两组接线判据红、M5 撤别名并写回副本→台账＋别名判据红、
C1 对照 `relativeTimeHint` 必须全绿（它确实全绿：`\b` 挡住了同名前缀）；四枚被碰文件还原逐字节 MATCH。

**卡点（待用户裁决，与 §5.35 末段同一条）**：`display.js formatTime` 与 `FlowDetailDialog.vue` 的 `fmtTime` 都要
"坏值原样回显"，出口五档给不了 ⇒ 是加第六档（`formatDateTimeLocaleEcho`）还是永久保留两处登记副本。
`AdminSso.vue:194` 的 UTC 直印是一枚**已定位未修**的显示缺陷，修法要选档（本地分钟档还是带秒档），不许机械替换。

**未入库警告**：`utils/time.js`、`utils/time.test.js`、`utils/f4-hand-assembly.test.js` 仍是 `??`（无 git 恢复源，
本轮预像在 `D:/tmp/f5-pre/`）；`display.js` 与四个 Admin/编排面板为 `MM`（第一列是并发作者的暂存，**不是我 staged**）。
所有改动一律未提交，等用户点名文件。相关：[[project-time-format-outlet]]、§5.30、§5.34、§5.35。

## 5.37 联盟词表所有权守卫：模块外不许自写模块已导出的名字（2026-09-29）

**动机**：§5.36 收的是"时间怎么格式化"，这一轮收的是"同一个 wire 值由谁给出中文"。词表漂不会以"看起来重复"的形式
出现，只会以**两处对同一个枚举值给出不同文案**的形式出现——而后者一旦上线，用户看到的就是"界面有时说人话有时说英文"。

**普查（scratch 驱动 `D:\tmp\vocab-ownership.mjs`，读数 `reports/data/f6-vocabulary-census-20260929.txt`）**：
名字集取 `modules/expert-alliance/{contract,model}` 的具名导出 = **346 个**；扫描集取 src 内除该模块与
`.test/.stories` 外的 .js/.ts/.vue = **219 个文件**；命中形态只认**定义位点**（`function NAME` / `const NAME =`）。
结果：**8 个名字 / 15 处**。

**逐处走到底，15 处里只有 2 处是词表副本**（其余 6 处是 §5.36 已按输入形状定过档的时间副本、5 处是撞名、2 处是同域不同档）：

| 站点 | 裁决 | 依据 |
| --- | --- | --- |
| `views/expert/panels/ExpertEnterprisePanel.vue`（旧 :843-845 局部 `sessionStatusLabel`） | **收口**，并**修缺陷** | 盘上三元链只有 active/archived，`closed` 会话把英文原样印上界面；模块 `contract/sessions.js:31-35` 三档齐全，未知态另给 `${value}（后端未统计此状态）`。代价：未知态文案变长（这是"后端未统计"该显形的样子） |
| `views/expert/ExpertPlazaView.vue`（旧 :1026-1034 局部 `bookingStatusLabel`） | **收口**，零显示改动 | 四档中文与 `contract/enums.js:198-203` 逐字相同，兜底链亦同（`map[status]||status` vs `table[key] ?? key ?? ''`）；收的是"第二本字典"——后端加第五档时只有模块那份会更新 |
| `formatTime` ×6（MessageBubble / AdminLlm / AdminSso / ChatView / ExpertEnterprisePanel / BrowserView） | 登记不并 | §5.36 的 T 族，五档出口逐形状量过：复合档、`MM/DD HH:mm:ss`、UTC 直印、`M/D`、`M/D HH:mm`、`YYYY/MM/DD HH:mm:ss` 各是一档 |
| `ENDPOINTS` ×2、`requestPath`（admin-lowcode / governance 各自的 `contract/endpoints.js`） | 登记不并 | 别的模块**自己的**端点契约，跨命名空间同名；按 [[project-vocab-copy-taxonomy]] 的 (c) 类，键集未逐字比对 ⇒ 若日后发现键值全等另案 |
| `PHASE_META`、`phaseProgress`（`stores/alliance.store.js:54/98`） | 登记不并 | 该 store 是零消费者空壳（`useAllianceStore` 在 stores/ 外无 importer），退役需用户点名（D4） |
| `phaseProgress`（`ExpertOverviewPanel.vue:384`）、`collabMode`（`ExpertWorkspaceView.vue:716`） | 登记不并 | 局部 `ref` 状态：存的是"各阶段百分比"/"当前 mode 键值"，与契约层同名**函数**判的两件事不同 |
| `gradeLabel`（`components/ai/GateResult.vue:104`） | 登记不并，**余账 D1** | 同域两档：盘上 `优秀/良好/合格/不合格`，契约层 `优秀 · 优质交付/…`；圆环位放不下后缀，且 `retryable` 判的是 grade 值不是文案 ⇒ 要么契约层补短档出口要么改版面，需用户选档 |
| `confidenceText`（`components/MessageBubble.vue:752`） | 登记不并，**余账 D2** | 真 0 处两档相反：这里 `0%`，契约层把 `n<=0` 归为「—」（无值）。并入会把 0 置信度显示成空态 ⇒ 先量 wire 上 confidence 能否为 0 |

**常驻判据**：`frontend-ui/src/modules/expert-alliance/contract/vocabulary-ownership.test.js`，7 例分通道（合在一条 `it`
里会互相顶红）：L1 命中集与 `REGISTERED` **双向相等**（新增越界要写理由才过，摘掉名字集也会因登记口落空而红）、
L1b 针的牙齿（定义形态打得红，导入/调用/注释/同前缀名打不红）、L2 分母（名字集 ≥300、扫描集 ≥200，实测值随判决打印）、
L3 两处收口的文案账、L3b 新增覆盖（就是旧副本的缺陷：`closed`/未知/空态）、L4 接线账、L5 与 §5.36 时间副本台账交叉一致
（登记的六处 `formatTime` 必须也在 `utils/time.test.js` 的 `KNOWN_COPIES` 里——两本账不许各说各话）。

**两条仪器教训（都是本轮实测踩出来的，不是推演）**：
1. **按标识符找命中 ≠ 能力重叠**。15 处命中里 13 处不是词表副本。所以判据的形态不能是"清零"，只能是
   "命中集＝登记集"：它挡新增，不要求本轮清完；否则下一位作者为了变绿就会把针磨钝（假阳的代价是人绕过闸门）。
2. **注释里的 import 能把 `toMatch` 针骗成假绿**。M4（把 `import { sessionStatusLabel } ...` 整行注释掉）在旧针下
   **rc=0**，因为 `import \{[^}]*sessionStatusLabel[^}]*\}` 不介意行首的 `//`。修法＝锚行首（`^import …` 配 `m` 标志），
   修完 M4 才真的打红 L4。已知残留边界：`utils/time.test.js` 与 `utils/time-locale-outlet.test.js` 的同类针仍未锚行首，
   其后果由 rollup 兜（注释掉 import 而模板还在调用 ⇒ `vite build` 直接崩），故本轮只登记不动（改当轮须重跑 §5.36 的 6 枚变异）。

**变异电池**（scratch `D:\tmp\f6-battery.py`，见证 `reports/data/f6-mutation-witness-20260929.txt`）：
未变异基线 rc=0 且红格为空；M1 模块外自写 `bookingStatusLabel` → 红 [L1]；M2 同前缀名 `bookingStatusLabelExtra`
→ **必须不红**（挡子串针）；M3 名字集摘掉 `model/`（346⇒254）→ 红 [L1, L2]（双向台账与分母各自生效）；
M4 撤销一处收口的接线 → 红 [L4]；M5 扫描集清空 → 红 [L1, L2]（"0 命中"与"没有缺陷"不同形）。
**5 枚 / 判据不符 0 枚**，三处被碰文件还原后 sha 与盘上基线逐位 MATCH，植入文件已删。

**门禁分母（同轮实测）**：全量 vitest **77 文件 / 1023 例 / rc=0**（本轮 +1 文件 +7 例，上一轮 76/1016）；
`vite build` rc=0（30.77 s）；`check-locale-format-outlets.py` rc=0 且 `--selftest PASS`（L4 扫描集 270 文件）；
`check-frontend-module.py` ERROR=0；`check-sfc-dead-refs.mjs` files=128 / deadRefFiles=0 / verdict=PASS。

**未入库警告**：本判据文件与 §5.36 的 `utils/time.js`、`utils/time.test.js`、`utils/f4-hand-assembly.test.js` 同为 `??`
（无 git 恢复源，预像在 `D:/tmp/f5-pre/` 与 `D:/tmp/f6-*`）；`ExpertEnterprisePanel.vue`、`ExpertPlazaView.vue` 是 `MM`
（第一列属并发作者的暂存）。所有改动一律未提交，等用户点名文件。相关：[[project-expert-name-outlet-ledger]]、§5.36、[[project-vocab-copy-taxonomy]]。

## 5.38 F7 内联枚举文案：从 38 处"逐字相同"里收掉企业面板的七处私表（2026-09-29）

**为什么 §5.37 拦不住这一族**：那本账管的是"模块外不许自写模块已导出的**名字**"。而视图里另一种形态根本不写函数名——
`(wire→中文)` 直接摆在模板的 `el-option` 上、或摆在一张 `modeLabels` 私表里。它不撞名字，所以 L1 扫不到，
但同一个 wire 值会在两处给出**不同的档**，这比文案不同更糟：整档缺失。

**普查读数**（档案 `reports/data/f7-inline-wording-census-20260929.txt`，仪器为 scratch 的 `.mjs`，**未落库＝没牙**）：
真值源取 `modules/expert-alliance/{contract,model}` 里的 (wire 值 → 中文文案) 对，**80 个键**；模块外扫描 **219 个文件**；
命中 **247 处 / 38 个键**，其中文案与契约**逐字相同 38 处**、同键不同文案（已经漂了）**209 处**。
两代仪器假账先记在档案头，别再犯：① 第一版按裸标识符 `name:` 匹配 ⇒ 104 处全是别的对象的同名字段（跨域同名，零真账）；
② 第二版仍 315 处 ⇒ 加 18 名通用对象键豁免桶，且**被豁免的计数必须按名打印**（`name=39 status=23 code=17 …`），
否则读者会把"没打印"读成"0 处"。

**收口的七处**（38 处里唯一的真缺陷簇，全部落在 `views/expert/panels/ExpertEnterprisePanel.vue`，写前 sha `dbdd754268ac593c`）：

| 位点 | 缺陷形状 | 现场后果 |
| :-- | :-- | :-- |
| :139-140 | 状态筛选器只列 `active`/`archived` 两档 | 契约有 `closed` ⇒ **已关闭的会话在界面筛不到** |
| :143-146 | 类型筛选器列 `smart`/`multi_expert`/`algorithm` | 不是 `session_type` 的 wire 取值 ⇒ **选中即筛空** |
| :156 | `{{ modeLabels[s.mode] \|\| s.mode }}` | 本地别名＋私表 ⇒ `multi`/`enterprise` 直印英文 |
| :488 :494 | api 垫片把 `sessionType` 复制成本地 `mode` | 同一字段第二套名字 |
| :586-588 | `modeLabels` 私表（5 档，含 2 个非 wire 值） | 后端加档只有契约那份会更新 |
| :639 | 客户端二次筛读 `s.mode` | wire 加档时这里静默失效 |
| :829 | 详情弹层用私表印模式 | 同 :156 |

**修法**：两个 `el-select` 改 `v-for="o in SESSION_STATUSES"` / `v-for="o in SESSION_TYPES"`（选项集与文案一律取契约），
展示改 `sessionTypeLabel(s.sessionType)`，垫片不再造 `mode`，筛选改 `s.sessionType`，`modeLabels` 删除并在原位留一行"为什么"。
写后 sha `f7f58a9b33e692ef`，**1306 行 / 60720 B**（行数由 `splitlines()` 数、字节由 `os.path.getsize` 数，
所以 +65 B 全部来自那两处"为什么"注释——行少了 4 行反而字节涨了）。

**常驻判据**（同一文件 `contract/vocabulary-ownership.test.js` 由 7 格扩到 **9 格**，新增两格各成通道）：
L6＝会话筛选与展示取自契约表：三个名字各一枚 `^import … from '@/modules/expert-alliance/contract'` 针（按名独立，
M4 只摘其一 ⇒ 只有 L4 红）＋按 `v-model` 切出 select 块后"必须 `v-for` 契约表"与"不许写回硬编码 `el-option` 档"
＋两张表全局各只出现一次＋`modeLabels` 不许回来＋筛选不许吃 `s.mode`；
L6b＝契约取值账：`SESSION_TYPES` 四档 / `SESSION_STATUSES` 三档**取值**逐字钉住，`sessionTypeLabel` 的三档行为点名
（未知值原样回显、空值 `未分类`），**不钉整张文案表**（M9 改一档文案必须不红＝不过度钉，§5.36 同条教训）。
L6 的禁令**按 select 块切，不按整文件**：`enterpriseForm.mode` 那个下拉是另一套词汇、挂在未挂载的模板桩上，
整文件级禁令会把合法站点判成缺陷（属性名/子串假阳第三次复发，见 [[attribute-name-substring-false-positive]]）。

**变异电池 9 枚 / 判据不符 0 枚**（`reports/data/f8-mutation-witness-20260929.txt`，驱动器 scratch 未落库）：
M1 模块外自写 `bookingStatusLabel`→[L1]；M2 同前缀不同名（必须不红）→[]；M3 名字集只留 `contract/`⇒判定集塌缩→[L1, L2]
（契约名集 254 < 300 下限，两格都该红）；M4 只摘 `sessionStatusLabel` 的 import⇒用了却没引→[L4]；M5 扫描集清空→[L1, L2]；
M6 状态筛选写回硬编码一档→[L6]；M7 二次筛写回 `s.mode`→[L6]；M8 契约删 `enterprise` 档→[L6b]；M9 改一档文案→[]。
每枚跑完逐文件 sha 还原他证 MATCH，植入文件已删。

**门禁分母（同轮实测）**：全量 vitest **77 文件 / 1025 例 / rc=0**（上一轮 77/1023，本轮 L6+L6b 两格）；
`vite build` rc=0（**34.70 s**）；`check-locale-format-outlets.py` rc=0（扫描 270 文件，余账 2 处）；
`check-frontend-module.py` ERROR=0（WARN 为存量）；`check-sfc-dead-refs.mjs` files=128 / checked=126 / deadRefFiles=0 / verdict=PASS。

**余账（只报不改，等用户裁决）**：
D1 `GateResult.vue` 的 `gradeLabel` 是短档、契约 `GRADE_LABELS` 是长档（"优秀" vs "优秀 · 优质交付"）——已登记，选档待定；
D2 `MessageBubble.vue` 的 `confidenceText` 对真 0 印 `0%`，契约 `confidenceText` 印 `—`——已登记，口径待定；
D3 `sessionTypeLabel` 与 `sessionStatusLabel` 对未知值行为**不对称**（原样回显 vs 追加"后端未统计此状态"）——统一与否待定；
D4 `createExpertSession({ title, mode: 'smart' })` 里的 `mode` 是死重量（垫片只转发 `title`）——删字段还是真发待定；
209 处"漂"**不做批量裁决**：绝大多数与本域词表无关，逐处要人读，全量读数原样附在档案末尾。

**未入库警告**：本轮被改的两份文件（`ExpertEnterprisePanel.vue`、`contract/vocabulary-ownership.test.js`）
在盘上分别是 `MM` 与 `??`；面板的写前图像在 `D:/tmp/f8-pre/`，判据文件的当轮 sha 为 `f61a68868af4fdf0`。
所有改动一律未提交，等用户点名文件。相关：§5.37、[[project-vocab-copy-taxonomy]]、[[project-frontend-reachability-ledger]]。

## 5.39 F9 联盟任务页：四本状态字典交回 TASK_STATUS，筛选条从此不缺档（2026-09-29）

**为什么这一页单独一节**：§5.38 的普查把 `AllianceTaskView.vue` 指到两处（融合策略 `el-option`、`statusFilters`），
但逐行读全文件后又挖出两处**普查形状认不出来的**：第四本状态字典 `const statusLabel = status => ({…}[status] || status)`
（换过名字 ⇒ §5.37 的 L1 按"模块导出名"扫定义位点，天然扫不到）与 DAG 图例四行（`<span>` 里裸中文，没有引号枚举键同行）。
⇒ 一台普查只认一种形状，另一种形状的同一族债要按"这一屏有几本字典"去数，而不是按"扫描命中几处"。

**五处内联词表与后果**（写前 sha `a4ec142a4988c09e`，1002 行）：
:136-139 图例四档（`legend` 与 CSS class 同名，色板只有这四枚）；:272-275 融合策略四个 `el-option`，其中
`best_of` 写「择优汇总」而契约 `FUSION_LABELS` 是「择优融合」（Rust `fusion_label()` 口径）＝**同一 wire 值两份文案**；
:372-376 `statusFilters` 自列 6 档 ⇒ 契约 `TASK_STATUS` 有 **7** 档，**`planning`（规划中）的任务在界面永远筛不到**
（与 §5.38 的 `closed` 同型）；:386 第四本状态字典还带 `ready`/`unknown` 两个 wire 上产不出的死档
（`model/normalize.js:40` 把空状态归成 `pending`）；:49 :84 两处展示吃那本字典。

**收口**：视图只留"这一屏画哪几档"，档名与文案一律走契约出口——
`statusFilters = [{ key: 'all', label: '全部' }, ...Object.values(TASK_STATUS).map(k => ({ key: k, label: taskStatusLabel(k) }))]`、
`FUSION_CHOICES`／`DAG_LEGEND` 两枚子集常量配 `fusionLabel(o)`／`nodeStatusLabel(s)`、`statusLabel` 整行删除、
两处调用改 `taskStatusLabel(task.status)`。写后 sha `cff5958fe7524b13`，998 行 / 34667 B（−5 行 / −100 B）。

**有意的展示变更（四处，不属"零显示改动"）**：筛选条由 7 个 chip 变 **8 个**（新增「规划中」）、
pending「待处理」→「待执行」、running「运行中」→「执行中」、best_of「择优汇总」→「择优融合」。
其余档文案逐字不变；未知 wire 值行为不变（旧 `|| status` 与新出口同为原样回显）。

**常驻判据由 9 例扩到 11 例**（同一文件 `contract/vocabulary-ownership.test.js`）：
L7＝四个名字各一枚 `^import` 针（按名独立）＋"筛选条必须由 `Object.values(TASK_STATUS)` 推出"＋
`not.toMatch` 内联档表形状（`{ key: 'pending', label: `）＋`not.toMatch(/^\s*const statusLabel\s*=/m)`＋裸 `statusLabel(` 计数 0＋
`taskStatusLabel(task.status)` 恰两处＋融合档不许写回 `label="…" value="weighted|…"`＋两枚子集常量各一处；
L7b＝`Object.values(TASK_STATUS)` 七档取值逐字钉住、每档都有中文（`label !== key`）、出口行为点名（未知回原值、空回空串）。
**L7 按形状钉而不是靠 L1 按名字钉**，正是因为这一族的副本会换名字。

**电池 14 枚 / 判据不符 0 枚**（`reports/data/f9-mutation-witness-20260929.txt`，驱动器 scratch 未落库＝没牙）：
M10 筛选写回内联一档→[L7]；M11 带回 `const statusLabel`→[L7]；M12 融合档写回硬编码 label→[L7]；
M13 契约删 `PLANNING`→[L7b]（契约少一档只有 L7b 看得见）；M14 改一档中文→**必须不红**[]；M1–M9 复跑同 §5.38 结论。
四份被碰文件（判据／企业面板／任务页／enums）每枚跑完逐文件 sha MATCH，植入文件已删。

**门禁分母（同轮实测）**：全量 vitest **77 文件 / 1027 例 / rc=0**（上一轮 77/1025）；`vite build` rc=0（**32.97 s**）；
`check-frontend-module.py` ERROR=0；`check-locale-format-outlets.py` rc=0（270 文件）；`check-sfc-dead-refs.mjs` files=128 / deadRefFiles=0 / PASS。

**余账（只报不改）**：融合策略契约 9 档而界面只开放 4 档（开放哪些是产品选择，登记不并）；
DAG 图例只画 4 档而 `NODE_STATUS` 有 7 档（`ready`/`skipped`/`cancelled` 没有色板，属样式余账）；
写后 :166 副标签直印 wire 节点类型键（英文）＝未归一字段上界面，**已于同轮结案，见 §5.40**；
`components/DagViewer.vue:68-74` 另有一份五档 legend 私表，但本轮按 `DagViewer` 全库复算只有 `components/index.js:9` 的 re-export、
**零真 importer**（与可达性账那 31 个孤儿同族）⇒ 孤儿不修，等退役裁决。

**未入库警告**：任务页与 enums.js 是 `MM`（第一列属并发作者的暂存），判据文件仍是 `??`（无 git 恢复源，
预像在 `D:/tmp/f9-pre/`，当轮 sha `2513772f6f898525`）。一律未提交，等用户点名文件。相关：§5.37、§5.38、[[project-vocab-copy-taxonomy]]。

## 5.40 F10 DAG 画布上的英文常量：节点副标签交回 `nodeStatusLabel` 出口（2026-09-29）

**动机**：§5.39 登记的「下一枚候选」。联盟任务页 SVG 画布把 wire 节点类型键直印成文字，中文界面里嵌一个英文词。
这一族不能只在前端找答案：那个键到底有没有值、由谁给，真值在服务侧。

**服务侧真值（本轮现场读 Rust；仓库只有 `platform/domains/…`，不存在顶层 `domains/`）**：
端点由 SDK 宿主挂载（`platform/domains/alliance/sdk/mox-alliance-http-sdk/src/alliance.rs:1820`
`.route("/api/alliance/tasks/:id/dag", get(get_task_dag))`），两条通道各自用 `json!` 现场构造节点：
远程优先 `alliance_remote.rs:843` 与本地降级 `alliance.rs:1544` **都把 `type` 硬编成常量 `"expert"`**；
而三份节点契约体（`proto/mox-alliance-common-proto/src/types.rs:314 Node`、`sdk alliance.rs:65 ExecNode`、
`api/src/dto.rs:104 NodeDetailResponse`）**都没有 `type` 字段**。全库该串只这两处命中 ⇒
`type` 不是 wire 上的既有语义，是出参构造器给每个节点盖的同一个戳。

**后果**：副标签对每个节点都印同一个英文词＝零信息量；模块自己的 `normNode`（`model/normalize.js:76-93`）不带这个键 ⇒
视图若迁到模块契约口径，这一格会自动变空。视图当前走遗留适配器
（`composables/useAllianceTasks.js:33` → `api/alliance.api.js:219` → `api/allianceTaskModel.api.js:39 normalizeTaskDag`，
其中 `...node` 原样透传），所以这个键侥幸有值。

**修法（最小，不发明新契约）**：副标签改走该页已导入的出口 `{{ nodeStatusLabel(node.status) }}`——
`status` 是两条通道都发、`normNode` 也带的字段，文案归 `NODE_STATUS_LABELS` 七档管。
**不**引 `contract/graph.js` 的 `GRAPH_NODE_TYPE`（专家／能力域）：那是知识图谱命名空间，
按 §5.28 的 (c) 档「跨域同键名不算副本」，焊过来会把两个域的口径锁死。

**显示改动（本轮 1 处，真机读数已补＝同轮 03:08 UTC，见下行）**：节点框第二行由英文常量变为该节点的中文状态档
（执行中／已完成／…）。同页状态本已用描边色＋图例表达，文字副标签属冗余但同口径；若改判为「显示承担专家」需另选姓名出口，待裁决。

**真机读数（`reports/data/f12-real-machine-20260929.txt`，md5 `b2d302eb…`，4037 字节；Vite dev 3020 由本轮自起，
3080/3100/3200/8000 全 DOWN ⇒ 列表数据为注入数据（非后端），外壳／守卫／编译／scoped CSS／契约出口全是真路径）**：
`/expert-center/tasks` 注入按 `Object.values(NODE_STATUS)` 现推的 7 档各一枚＋一枚未知档后，DOM 里
`.dag-node-sub` 8/8 枚印出 `待执行 就绪 执行中 已完成 失败 已跳过 已取消 mystery_state`，
**没有一枚印出 wire 的 `type`（`expert`）**，未知档按出口既定行为原样回显；与同一页面里
`import('/src/…/contract/index.js')` 现调 `nodeStatusLabel` 的逐字结果相同（两口径互证）。
同屏旁证 `.dag-legend` 只有 4 档（待执行／执行中／已完成／失败）＝E2 余账的真机形态。

**新通道 L8**（判据 12 例，盘上 sha `54139919c5232f83`）：负向 `not.toMatch(/\{\{ node\.type \}\}/)`＋
正向 `countOf(src, '{{ nodeStatusLabel(node.status) }}') === 1`。

**电池 6 枚 / 判据不符 0 枚**（`reports/data/e3-mutation-witness-20260929.txt`，驱动在 scratch 未落库＝没牙，别接 CI）：
E1 退回直印该键→[L8]；E2 直印未归一的 `node.status`→[L8]；E3 出口调用印两遍→[L8]（钉的是「这一处走出口」，不是「文里出现过」）；
E4 撤掉两枚针再把缺陷放回＝盲测→**必须全绿**[]（证明针承重，不是顺带绿）；E5 只改几何坐标 y=12→13→**必须不红**[]（不过度钉版面）；
E6 写回视图自定中文档名→[L8]。两枚被碰文件每枚跑完逐文件 sha MATCH。

**门禁分母（同轮实测）**：全量 vitest **77 文件 / 1028 例 / rc=0**（上一轮 77/1027，+1＝L8）；`vite build` rc=0（**37.27 s**）；
`check-frontend-module.py` ERROR=0；`check-locale-format-outlets.py` rc=0（270 文件）；
`check-sfc-dead-refs.mjs` files=128 / deadRefFiles=0 / verdict=PASS；`check-doc-links.py` **rc=1（存量）**、WARN=120 与本轮动手前同数，
本 doc 唯一 WARN 仍是 :137（旧遗留 `FRONTEND-MODULE.md`），新增一节没引入悬空；`check-forbidden-terms.py` 2134 文件 0 命中。
（首轮我曾把 `| tail` 的退出码当成门禁的 rc 记成 rc=0，此处按门禁自身退出码重测改写。）

**余账（只报不改）**：DAG 图例只画 4 档而 `NODE_STATUS` 有 7 档（样式余账，§5.39 结转）；
`components/DagViewer.vue:68-74` 的五档 legend 私表零真 importer（孤儿，等退役裁决）；
服务侧那个 `"type": "expert"` 常量戳前端已不读，删除属后端改动，本轮不碰。

**未入库警告**：任务页为 `MM`（第一列属并发作者暂存），判据文件仍是 `??`（无 git 恢复源；
写前图像 `D:/tmp/e3-pre/`，当轮判据 sha `54139919c5232f83`、视图 sha `1288d04827721474`）。一律未提交，等用户点名文件。相关：§5.37–§5.39。

## 5.41 F11 模块内部自写同名出口：L1 的扫描集盲区 + 「离开」档被折进「离线」（2026-09-29）

**动机（仪器缺口，比文案债更值钱）**：§5.37 的所有权判据 L1 把扫描集定为**模块外**（`contract/`+`model/` 是词表的主人，
按定义不许跟自己比）。于是「模块内部再自写一份同名出口」这一整族对常驻门禁**天然隐身**。本轮按 F10 家族普查
（模板直印 wire 枚举键，扫描集 270 文件 ⇒ 命中 30 处）反查进模块内部，实测模块内（除 `contract/`、`model/`、非 test/stories）
**扫描 32 个文件 ⇒ 定义位点命中 3 处**：`ExpertBookingPanel.vue:107 formatTime`、`ExpertCollabPanel.vue:270 availabilityLabel`、
`GraphNodeInspector.vue:108 collaboratorRows`（模块导出集 346 名与 L2 同一次读数）。

**真缺陷**：`ExpertCollabPanel.vue:270` 的局部字典 `const availabilityLabel = (e) => (e.online ? '在线' : e.status === 'busy' ? '忙碌' : '离线')`
是三元链不是表 ⇒ **`EXPERT_AVAILABILITY` 的第四档 `away`（离开）被折进「离线」**，与 §5.38 的「筛选器缺 `closed` 档」同型；
而契约里 `availabilityLabel`／`EXPERT_AVAILABILITY_LABELS`（`contract/enums.js:166/273`）四档齐全。
同文件 `:102` 的候选行原本连文案都不走，直印 `{{ row.status }}` 原始值（`online`/`away` 上界面）。

**修法**：删掉局部字典，两处站点统一走契约两出口复合 `availabilityLabel(expertDisplayStatus(x.status))`
（`expertDisplayStatus` 即注册中心 `active/inactive/maintenance/deprecated` → 可用性域的既有归一，`normExpert` 内部同款复合）。
未登记值的后果由出口决定：`Labeled` 回原值 ⇒ 不再把陌生档硬说成「离线」。

**显示改动（本轮 2 处，真机读数已补＝同轮 03:10 UTC，见下行）**：① 专家选择行 `:66` 的「离开」由印「离线」改印「离开」（**修缺陷**）；
② 协作流候选行 `:102` 由英文原始值改中文档名。其余三档文案逐字不变。

**真机读数（同一份 `reports/data/f12-real-machine-20260929.txt`）**：`/alliance/collab` 的「单专家咨询」页签内
`.acw-chip` 5 枚实测印出 `甲首席 · 在线 / 乙工程 · 离线 / 丙运维 · 忙碌 / 丁值班 · 离开 / 戊归档 · error`
（wire status 逐个喂 `active/inactive/maintenance/away/deprecated`）——「离开」档在真机 DOM 上成立。

**同轮新余账（只报不改，登记为 F10 家族第 31 处直印，根因不在界面而在契约）**：末枚 chip 印出英文 `error`。
链是 `contract` 私有表 `EXPERT_DISPLAY` 把 `deprecated` 映射成 `error`，而 `error` 不属于
`EXPERT_AVAILABILITY` 的四档（`online/busy/offline/away`），于是 `availabilityLabel` 按「未知回原值」的
**被 §5.39 L7b 钉住的既定行为**把英文原样吐上界面。出口函数没有错，错的是那张映射表的值域越界
（把「语气/色调档」当「可用性档」用，与 [[project-theme-token-namespace-split]] 的「一档兼两职」同族）。
`deprecated` 该并入「离线」还是另立第六档属产品选择 ⇒ 待裁决；本轮不动代码，也不给 `availabilityLabel`
开 `error` 豁免档（开了就等于把越界值合法化，反咬 §5.39 的判档口径）。

**「离开」档的生产者账（同轮补，服务侧只读）**：`platform/domains/alliance/sdk/mox-alliance-http-sdk/src/alliance.rs:380-386`
的 `expert_status_str` 只会发 `online / offline / busy / error` 四串（`Deprecated ⇒ "error"`），**不发 `away`**；
但网关两处按可用性加权时确实 `match` 了 `"away"`（`platform/gateway/mox-platform-gateway-svc/src/alliance/experts_common.rs:856`、
`.../experts_graph.rs:515`），契约也登记 `EXPERT_AVAILABILITY.AWAY`（`contract/enums.js:142`）。
⇒ `away` 的来源是注册表里存的可用性值，不是本地 `ExpertStatus` 枚举路径。所以 §5.41 的「修缺陷」应读作
**「当注册表存 `away` 时才成立」**：本轮真机那枚 `离开` 是注入读数，不能证明线上存在 `away` 行（后端 DOWN，无行可证）。
反过来 `error` 是线上会真出现的串（`deprecated` 专家的 wire 值），故上一段的余账优先级高于本档。

**新通道 L9**（判据 12→**13 例**）：`scanModuleSelfWrites()` 复用 L1 的 `EXPORTED`／`DEF_LINE` 同一套对象（**不另造第二实现**），
判据形态仍是**命中集＝登记集双向相等**，登记 2 处各给理由（`formatTime`＝站点包装、文案已交回出口，只留空态与坏值回显档；
`collaboratorRows`＝局部 computed 撞名）。分母格：模块内扫描文件数 `>= 30`（实测 32）＋逐命中必须有理由。

**电池 6 枚 / 判据不符 0 枚**（`reports/data/f11-mutation-witness-20260929.txt`，2374 B，驱动 scratch 未落库＝没牙）：
G1 塞回局部字典→[L9]；G2 撤按名导入（用了却没引）→[L9]；G3 一处站点丢掉 `expertDisplayStatus` 那半→[L9]；
G4 撤掉一条登记项（命中还在）→[L9]（双向账两向都有牙）；G5 把 `components/` 也当定义主人排除＝扫描集塌缩→[L9]；
G6 改站点无关文案→**必须不红**[]。两枚被碰文件每枚跑完逐文件 sha MATCH。

**门禁分母（同轮实测）**：全量 vitest **77 文件 / 1029 例 / rc=0**（上一轮 77/1028）；`vite build` rc=0（**31.36 s**，
新增两个具名导入的消费者账由 rollup 验，不由 vitest）；`check-frontend-module.py` ERROR=0；`check-locale-format-outlets.py` rc=0；
`check-sfc-dead-refs.mjs` files=128 / deadRefFiles=0 / PASS；`check-doc-links.py` rc=1（存量）WARN=120 与动手前同数。

**F10 家族普查的归类结论（30 处不许当 30 笔债）**：逐处走到底，本轮可见的形状有三类——
① **诊断台的原样回显是设计**（`AllianceOrchestrationView.vue:62/75/91/133/193`、`AllianceConsoleView.vue:137/164`：
`<code>` 元素＋`plan.status =` 键名前缀＋旁边并置中文 `tierLabel(...)`，属"后端到底回了什么"的证据面，§5.35 的坏值回显同口径）；
② **两档版面**（`SessionStatsPanel.vue:44` 同一行既有 `t.label` 中文又有 `t.type` 原始键，是 §5.28 (b) 档不并）；
③ **真缺陷**＝本轮的 `ExpertCollabPanel` 两处，以及待裁的 `AllianceConsoleView.vue:203`（熔断器 `c.state` 单一档名、
该值域（closed/open/half_open）在契约里**没有中文出口**⇒ 要先开档还是原样回显，属产品选择）。
其余模块外 20 处（admin/graph/ai/misc）多为别的域的词汇，按 (c) 档跨域同名不算副本，逐处待裁（#33）。

**未入库警告**：面板与判据文件状态同前（`MM`／`??` 无 git 恢复源；写前图像 `D:/tmp/f11-pre/`，
当轮判据 sha `e157a81d116a249e`、面板 sha `6bc34c98200e6df6`）。一律未提交，等用户点名文件。相关：§5.37–§5.40、[[project-vocab-copy-taxonomy]]。

## 5.42 F12 熔断器状态档上界面：`c.state` 由直印 wire 改走 `breakerStateLabel`（2026-09-29）

**这一枚是 §5.41 真机取证顺手带出来的**（`reports/data/f12-real-machine-20260929.txt` 的同族余账 #33 ①）。
`modules/expert-alliance/views/AllianceConsoleView.vue:203` 原写 `{{ c.state }}`，把 wire 的 `open`／`closed`
直印到「调度器控制台 · 熔断器」列表上——与 §5.40 的 `node.type` 同形（英文常量戳上界面），
且比那枚更糟：`closed` 在中文界面上会被读成「关闭」而不是「正常」。

**wire 权威（服务侧只读，本轮实测）**：`platform/gateway/mox-platform-gateway-svc/src/alliance/experts_dispatcher.rs:494-509`
——只有 `failure_count > 0` 的专家进 `circuit_breakers` 表，`state` 由 `count >= circuit_breaker_threshold`
**二分为 `"open"` / `"closed"`**；`half_open` 只存在于 scheduler-core 的 `llm_router`（`half_open_probes`，:120）内部配置，
不在本字段值域内 ⇒ 契约只登记两档，不臆造第三档。

**改动（3 个文件）**：`contract/enums.js` 末尾新增 `BREAKER_STATE`／`BREAKER_STATE_LABELS`（`open→已熔断`、`closed→正常`）
与出口 `breakerStateLabel`（复用同一族的 `Labeled`：命中回中文、未命中回原值、空回空串）；
视图 `:203` 改 `{{ breakerStateLabel(c.state) }}`，`:532` 按名从 barrel 补导入。
`model/normalize.js:645` 的 `state: str(c.state)` **保持原样**——数据层留 wire 值、展示层走出口，
与 §5.40 对 `node.status` 的分工口径一致。

**判据落库（常驻，`contract/vocabulary-ownership.test.js`，13 例 → 15 例）**：
L10＝视图侧四道针（`not.toMatch(/\{\{\s*c\.state\s*\}\}/)` 撤直印、出口站点计数恰 1、
`^import …breakerStateLabel…` 按名独立、视图里自写 `breakerStateLabel` 副本判缺陷）；
L10b＝契约侧（`Object.values(BREAKER_STATE)` 逐字 `['open','closed']`、每档 `label !== key`、
出口对每档返回标签表自身的值（**只钉管道不钉措辞**）、未知档回原值、空档回空串）。

**电池 5 枚／判据不符 0 枚**（`reports/data/f12-mutation-witness-20260929.txt`，驱动器在 scratch 未落库＝没牙、别接 CI）：
M15 写回直印→[L10]；M16 撤按名导入→[L10]（该枚只撤导入通道，站点计数针仍绿 ⇒ 红点确实挂在导入上）；
M17 撤 `open` 档中文→[L10b]；M18 契约少一档→[L10b]；
M19 只改中文措辞（`已熔断→熔断中`）→**必须不红**，实测 `Tests 15 passed` ⇒ L10b 没有把文案钉死，
将来产品改措辞不会假红。三枚被碰文件每枚跑完逐文件 sha MATCH，终检 3/3 MATCH，复跑基线 rc=0。

**门禁分母（同轮实测）**：全量 vitest **77 文件 / 1031 例 / rc=0**（上一轮 77/1027，本枚 +2）；
`vite build` rc=0（**35.65 s**）；`check-frontend-module.py` **ERROR=0**；`check-locale-format-outlets.py` **rc=0**；
`check-sfc-dead-refs.mjs` **files=128 / checked=126 / deadRefFiles=0 / verdict=PASS**；
`check-doc-links.py` **rc=1（存量）／扫描 361 文件／判定引用 2036 条**，本文件在输出里仍只有 `:137` 那一条存量 WARN（新加的
`reports/data/f12-*` 反引号引用未被登记为链接，故无新悬空）。

**仪器近失（必须记账，别当没发生）**：电池第一版还原语句写成 `open(target,'wb').write(read(target) and … snapshot)`
——`open(…,'wb')` **先把目标文件截成 0 字节**，随后作为参数求值的 `read(target)` 读到空串（假值）并被短路写成 `b''`，
于是 `AllianceConsoleView.vue` 被清空（52198 B → 0 B，md5 落到空串值 `d41d8cd98f00b204`）；
当轮的直接症状是 full vitest `19 failed` ＋ `vite build` rc=1，**看着像我的改动改坏了页面，其实是还原器毁掉了夹具本身**。
恢复源＝本轮跑前自建的快照 `D:/tmp/f12-cur/AllianceConsoleView.vue`（快照器有「已存在且不同就拒绝」的断言，
所以那份一定是写前图像），`cp` 回去后 sha `23cc21aefe0e8f4d` 与快照逐位 MATCH，随后把还原改成
**先把快照字节读进变量、再打开目标文件写**，重跑全绿。⇒ 新规矩落进电池：**还原器不许在被截断的文件上求值任何表达式**。

**余账（只报不改）**：`half_open` 若将来真的进 wire，契约要补第三档（L10b 的 `toEqual(['open','closed'])` 会先红，正是想要的行为）；
`error` 那笔（§5.41 的 `deprecated→error` 值域越界）仍未裁决，且优先级高于本枚。

**未入库警告**：`enums.js` 为 `A `（第一列属并发作者暂存）、视图为 `AM`、判据文件为 `??`（无 git 恢复源，
写前图像 `D:/tmp/f12-pre/`（追加判据前）与 `D:/tmp/f12-cur/`（本轮改动全量后、电池跑前）各一份）。一律未提交，等用户点名文件。
相关：§5.40、§5.41、[[project-vocab-copy-taxonomy]]。

## 5.43 F13 编排状态档上界面：三站点由直印 wire 改走 `orchStatusLabel`（2026-09-29）

**wire 权威**（同一张表服务计划 / 编排 / 执行三段，`platform/gateway/mox-platform-gateway-svc/src/alliance/experts_orchestration.rs`）：
`:186 pending`（计划落库初值）、`:203 draft`（计划体）、`:469/:484 running`、`:490/:497/:511 completed`、`:456 failed`、
`:508 completed|partial`、`:750 unknown`（读不到 `overall_status` 时的兜底）。⇒ 七档。
任务域 `TASK_STATUS` 只有四档对得上（待执行/执行中/已完成/失败），`draft/partial/unknown` 是它没有的值，所以编排表**不能**并进任务表。

**契约出口**（`contract/orchestration.js` 文件尾，23565 B，md5 `ec85648e564c3ba6`）：`ORCH_STATUS`（七键）、
`ORCH_STATUS_LABELS`（待执行/草案/执行中/已完成/失败/部分完成/未知）、`orchStatusLabel(status) = LABELS[status] ?? status ?? ''`
（该文件无 import，故未复用 `enums.js` 的 `Labeled`，注释里点名了 Rust 行号）。

**收口的三个站点**（`views/AllianceOrchestrationView.vue`，20373 B，md5 `87a1444ec50e5f12`；按名导入在 `:262`）：

| 站点 | 出处 | 改前 | 改后 |
|---|---|---|---|
| `:75` | `plan.steps[]`（规划拓扑） | `{{ s.status }}` | `{{ orchStatusLabel(s.status) }}` |
| `:126` | execute 响应徽章 `store.outcome.status`（由 `orchExecuteOutcome()` 取 `res.status`，见 `store/alliance-orch.store.js:73`） | `{{ store.outcome.status \|\| '—' }}` | `{{ orchStatusLabel(store.outcome.status) \|\| '—' }}` |
| `:133` | `execute.steps_executed[]` | `{{ s.status }}` | `{{ orchStatusLabel(s.status) }}（两档版面：后接 `tierLabel(...)`） |

`:126` 是**普查换尺子才出现的**：第一轮只按字面量 `{{ orchStatusLabel(s.status) }}` 的形态登记了 `:75/:133`，
改用逐行 `finditer` 扫 `status|state|availability|phase|mode` 一族之后，`store.outcome.status` 这条同词汇表、不同宿主对象的站点才浮出来。
⇒ 判据的覆盖面按**标识符族**算，不按上一次写下的表达式算（同 §5.19 那类翻案）。

**常驻判据**（`contract/vocabulary-ownership.test.js`，23348 B，md5 `e475be7ce59ceff1`，17 例）：
`L11`（`:329`，视图侧：不许 `{{ s.status }}` 直印、出口字面量恰好 2 处、按名导入必须来自契约、视图内不许自写 `orchStatusLabel` 副本、
`store.outcome.status` 直印形态必须为 0 且出口站点恰好 1 处）＋ `L11b`（`:339`，契约侧：`Object.values(ORCH_STATUS)` 逐字等于七档有序表、
每档有中文且**中文≠档名**、`orchStatusLabel(k)` 逐档等于表值、未知值 `blocked` 原样回显、空串回空串＝空态交回站点）。
标题从"两站点"改成"三站点"是口径修正，不是判据变化（用例数不变）。

**变异电池**（见证 `reports/data/f13b-mutation-witness-20260929.txt`＝权威那一份，7 枚全跑；
`reports/data/f13-mutation-witness-20260929.txt` 是收 `:126` 之前的两站点版本，其视图/判据哈希已过期，只留作过程证据）：
M20（`:75` 写回直印）→ 红 `[L11]`；M21（撤掉 `:262` 按名导入）→ 红 `[L11]`；M22（`:133` 写回直印）→ 红 `[L11]`；
M26（`:126` 写回直印）→ 红 `[L11]`；M23（契约 `ORCH_STATUS` 删 `partial` 一档）→ 红 `[L11b]`；
M24（把 `partial` 的中文改成与档名同形的 `"partial"`）→ 红 `[L11b]`；**M25（只把 `pending` 的中文"待执行"换成"排队中"）必须不红＝实测 GREEN**，
证明判据钉的是取值与管道，没有过度钉文案。基线 17/17 rc=0，7 枚逐枚还原 3/3 文件 MATCH，复跑基线 17/17 rc=0。

**同轮门禁分母**（终态，全部在改后重跑）：full `vitest run` 77 文件 / 1033 例 / rc=0（本轮新增 2 例 ⇒ 与 §5.42 记的 1031 自洽）；
`vite build` ✓ 38.01 s；`check-frontend-module.py` ERROR=0（`admin-lowcode` 那条 `[WARN]` 为存量）；
`check-locale-format-outlets.py` rc=0（I1/I2 余账仍按名点名 3＋3 处）；`check-sfc-dead-refs.mjs` files=128/checked=126 verdict=PASS；
`check-ep-feedback-imports.py --check` verdict=PASS；`check-doc-links.py` rc=1 属存量（120 条 WARN），输出未点名本节新增路径。

**登记为设计／不并的余留直印**（同一文件内，勿再当缺陷）：`:62 plan.status = {{ store.plan.status }}（{{ tierLabel('plan.status') }}）`
与 `:91/:92`、`:133/:135/:136/:137` 的「wire 原始值 ＋ 档位说明」两档版面、`:193 <td>{{ r.status }}</td>`（表头即 `status` 列，属数据导出视角）。
另两类是条件式而非展示：`:282 room.status === 'available' ? …`、`AllianceConsoleView.vue:362 current?.status === 'completed' ? …`
——它们把 wire 值钉在模板里做分支，界面不显示原始值，归 F7/F9 家族而非本枚。

**余账（只报不改）**：`AllianceConsoleView.vue:137` 的 `store.dispatchResult.status` 直印的是网关**常量戳** `"dispatched"`
（`experts_dispatcher.rs:605` 写死，不是状态机），单值不构成档；`:164 store.error.status` 是 HTTP 状态码，按设计直印。
若将来 `dispatchResult` 带上真实状态，需先开档再上出口。

**F14 立项（普查已定量，本轮未动工——回合预算不足以把"改＋验"跑完，按只报不改收口）**：
把 §5.43 的尺子从"展示站点"换到**比较运算**这一维（扫 `views/`＋`components/` 里的 `[!=]== '<wire token>'` 字面量），实测命中 **8 处**：
`components/SessionListPanel.vue:154/155/156`（`active`/`archived`/`closed`→el-tag type 私表）、`views/AllianceExpertsView.vue:282`（`available` 三元式文案）、
`views/AllianceConsoleView.vue:362/562/593/645`（`completed`×3、`running`）。
**wire 权威已量到**：会话族 `experts_common.rs:260`（doc 注释即 active/archived/closed）＋`:286 default_session_status()="active"`＋`experts_session.rs:284/285`（统计 `match` 只认这两档）＋`:612`（归档强写 `archived`）；
房间族 `experts_registry.rs:672-676` ⇒ **只有 `available` / `waiting` 两个值**，且由 `availability.status == "online"` 推出。
契约现状决定了两族的不同工作量：`contract/sessions.js:30-39` **已有** `SESSION_STATUSES`/`SESSION_STATUS_COUNTED`/`ARCHIVE_STATUS`，
所以会话族只是"视图把已登记取值重新打字一遍"（纯收口）；房间族**契约里没有词表**，要先新增 `ROOM_STATUS`/`roomStatusLabel`（有 Rust 行号可钉）。
牙应长成：L12 的禁串集合**从契约各表的 values 现推**（`TASK_STATUS`/`SESSION_*`/`NODE_STATUS`/`ORCH_STATUS`/`EXPERT_AVAILABILITY`/`ROOM_STATUS`），
扫 `views/`＋`components/`，分母与命中数同轮印出（本轮基线＝8 处）。
⚠️ **先决条件**：`contract/contract.test.js:1144` 有一枚按字面量 `current?.status === 'completed' && store.runtime…` 直接匹配视图源码的既有针——
把 `AllianceConsoleView.vue:562` 迁到常量会同时打红它，所以动工时必须同轮改那枚针（这是"判据自己钉死字面量写法"的第二处，属判据侧的债，不是视图的债）。
另注：`contract/orchestration.js:165/:395` 也在契约内部用字面量 `'failed'` 比较，是否要求契约自身改用常量，与 `mode: 'smart'` 死重量那笔（#31 D4）同批裁决。

**真机渲染证据：本轮未取得（记清楚，别拿单测冒充联调）**。网关 `:3080` DOWN 时，整页加载 `/alliance/orchestration` 返回
**HTTP 500／0 字节**（`vite.config` 把 `/alliance/*` 代理给联盟网关，代理规则抢在 SPA fallback 之前，dev log 为
`[vite] http proxy error: /alliance/orchestration ECONNREFUSED`）；同轮对照 `/` 与 `/login` 均 200 ⇒ 空白页**不是**应用缺陷。
改走客户端跳转（页内 `$router.push`）后落在 **403 权限页**，且此刻 `pinia._s` 为空表（权限 store 未被使用前不安装），
注入 `permission.roles` 这条路还没打通。⇒ 本枚的证据等级＝判据 17 例＋电池 7 枚＋build/门禁；
`§5.40/§5.41` 那种真机读数要等网关切回或权限注入配方补完再取。

**未入库警告**：本轮改动的四件（`contract/orchestration.js`、`views/AllianceOrchestrationView.vue`、
`contract/vocabulary-ownership.test.js`（`??`，无 git 恢复源）、两份 `reports/data/f13*-mutation-witness-20260929.txt`（`??`））
一律未提交，写前图像在 `D:/tmp/f13-snap/`（F13 补丁前）、`D:/tmp/f13-cur/`（判据 17 例后）、`D:/tmp/f13b-snap/`、`D:/tmp/f13c-cur/`。
电池驱动 `D:/tmp/f13-battery.py`、补丁驱动 `D:/tmp/f13-patch.py`、`D:/tmp/f13b-patch.py`、`D:/tmp/f13c-title.py` 均未落库＝**没牙，别接 CI**。
相关：§5.40、§5.42、[[project-vocab-copy-taxonomy]]、[[project-alliance-route-authority]]。

## 5.44 F14 视图里的 wire 字面量比较：会话族＋房间族收口成常量，任务族登记成双向台账（2026-09-29）

F10–F13 收的是**直印**（`{{ s.status }}` 上界面），F14 收的是**比较**——视图把契约里已经登记过的 wire 取值重新打一个字
（`status === 'active'`）。这类写法没有报错面：契约改档名、后端换枚举，视图那一行只会静默变成恒假。

**服务侧真值**（路径 `platform/gateway/mox-platform-gateway-svc/src/alliance/`）：会话族＝`experts_common.rs:260`
（文档注释 active/archived/closed）与 `:286` `default_session_status() = "active"`、`experts_session.rs:284/285`
（统计分支只认 archived 与 closed）、`:612`（归档强写 archived）；房间族＝`experts_registry.rs:672-676`
（只产 `available` / `waiting`，由 `availability.status == "online"` 派生，没有第三档）。

**出口**：`contract/sessions.js:499` `SESSION_STATUS = Object.freeze({ACTIVE:'active', ARCHIVED:'archived', CLOSED:'closed'})`
与 `contract/registry.js:270` `ROOM_STATUS = Object.freeze({AVAILABLE:'available', WAITING:'waiting'})`（各带 Rust 行号注释）。
写前→写后 md5：`46abecf47b7302ba → 7afa888ba155446c`、`97ac382e90360214 → d48b6dce09e230d9`；
六个被碰文件的当前 md5／字节／行数／CRLF 计数**原样印在 `reports/data/f14-readings-20260929.txt`**（含写前值以外的全部复核量），本节日程号不重复抄数。

**站点**：`components/SessionListPanel.vue:155/156/157`（`statusTagType` 三档改比常量，按名导入并进 `:124-132` 已有的 barrel 语句）；
`views/AllianceExpertsView.vue:282`（房间状态徽章）＋ `:333` 新增按名导入。两处收口后 L12 对这两个文件的命中数断言为 0。

**判据 L12**（`contract/vocabulary-ownership.test.js:353`，该文件现 **18 例**）三条设计：
① 禁串集合**不许硬编词表**——由 `Object.values(TASK_STATUS/NODE_STATUS/EXPERT_AVAILABILITY/BREAKER_STATE/ROOM_STATUS)`
＋ `SESSION_STATUSES.map(s=>s.value)` 现推；② 判集不许塌缩——`files.length ≥ 21`（模块 `views/`＋`components/` 的 `.vue`）当分母；
③ 命中集与 `LEDGER`（`:370-378`）**双向 `toEqual`**：多＝新债、少＝台账该删。另钉 `banned` 必须同时含 `available` 与 `archived`（防 barrel 掉档）。

**这一枚自己抓到我的普查盲区**：L12 第一次落地即红——扫描集 5 条 vs 台账 4 条，多的那条是
`views/AllianceConsoleView.vue:594` 的 `=== 'paused'`，而我手写普查时列的 token 里根本没有 `paused`
（只列了 completed/running/active/archived/closed/available/waiting 八档）。⇒ 结论入规：**按标识符集合现推的禁串会自己印出词表漏项**，
这与"覆盖面＝针枚举的标识符集合"是同一件事的正反两面。台账第 5 条因此不是我找到的，是判据给的。

**变异电池 8 枚，0 不符**（见证 `reports/data/f14-mutation-witness-20260929.txt`，快照目录 `D:/tmp/f14-cur` 拒绝覆盖）：
M27/M32 把比较对象换成另一档已登记取值（`closed`、跨族的 `archived`）→ 仅 L12 红；
M28 从台账删一条而扫描集没变 → L12 红；M29/M30 把两处已收口站点退回字面量 → L12 红；
M31 新增一枚契约里没有的档（`sentinel_unknown`）做比较 → **必须不红**（口径边界：台账只覆盖已登记取值，这不是漏洞）；
M33 把 `paused` 换成契约外的档而台账不改 → L12 红（"少＝台账该删"这个方向也有牙）；
M34 只改 `contract/orchestration.js` 里一枚中文档名措辞（`'真实计算'`→漂档文案）→ **不红**，证明针钉的是取值＋管道、不是文案。
五文件还原逐文件 md5 MATCH，复跑无变异基线 `18 passed (18)`。

**门禁分母**（同样原样摘录在 `reports/data/f14-readings-20260929.txt`，不是回忆）：全量 vitest 77 文件 / 1034 例 / rc=0，
`vite build` rc=0，模块化门禁 ERROR=0，locale 闸门 `[OK]` 扫描 270 文件（L1 余账 2 处＝登记表），ep 导入探针 verdict=PASS。

**设计内豁免与余账（只报不改）**：任务族既有 5 条比较留在台账里没收口，**先决**是
`contract/contract.test.js:1144` 把那处视图字面量（`current?.status === 'completed' && store.runtime…`）本身钉成了期望，
同轮要一起动那枚针才有意义；`orchestration.js:165/:395` 是契约**内部**的比较，扫描集只覆盖 `views/`＋`components/` 的 `.vue`，
登记为契约内部余账（不在 L12 牙内）；`store.dispatchResult.status = "dispatched"`（`experts_dispatcher.rs:605` 常量戳）仍是 §5.43 余账。

**未入库警告**：本节涉及的五个前端文件与两份 `reports/data/` 证据本轮**一律未提交**（不动 git 索引，等用户按名点名）；
写前图像在 `D:/tmp/f14-snap`、`D:/tmp/f14-cur`、`D:/tmp/f14b-snap` 三处，其中 `D:/tmp/f14-cur` 是电池快照（＝本轮落盘后图像）。
相关：§5.41、§5.42、§5.43、[[project-vocab-copy-taxonomy]]。

## 5.45 F14c 任务族字面量比较收口：5 站点改比常量，同轮拆掉钉字面量的针，L12 台账清空（2026-09-29）

§5.44 把任务族 5 处比较**登记**为余账，先决是 `contract.test.js:1144` 把视图里的
`current.value?.status === 'completed' && store.runtime?.mode === 'remote'` 整条钉成了期望。本轮先做了一步普查（驱动器自带的
全库扫描：所有 `*.test.js` 里同时含 `status` 与被收口字面量的行），**结果只有那一枚针**——先决消除，于是当场收口。

**站点（`views/AllianceConsoleView.vue`，导入行 `:521` 的 barrel 补 `TASK_STATUS, NODE_STATUS`）**：
`:362` 按钮文案三元、`:562` `reopenBlocked`、`:593` `canPause`、`:594` `canResume` 改比 `TASK_STATUS.*`；
`:645` 是**节点**计数，改比 `NODE_STATUS.COMPLETED`（同一个 `'completed'` 字面量在两个族里各有其义，
按名字收口会焊死口径——见 §5.37 (c) 档）。视图 md5 `23cc21aefe0e8f4d → 19fc54b4349427b4`。

**同轮拆针**：`contract.test.js` 那枚 `toMatch(/…/)` 跟着改成 `TASK_STATUS\.COMPLETED`（md5 `51449445d4a95317 → e28a3f77df6f98be`）。
判据侧 `LEDGER` **清空成 `[]` 并留一行说明**（`vocabulary-ownership.test.js`，md5 `032f5a4f9dfdb97f → 83fe066ba3668753`）——
遵循"清零要删条目而不是写 0"：留着条目下一位作者只会连台账一起改，空台账才能让"多＝新债"这一方向继续有牙。
五个文件的 md5／字节／行数与门禁判决行**原样印在 `reports/data/f14c-readings-20260929.txt`**。

**电池 7 枚，0 不符**（`reports/data/f14c-mutation-witness-20260929.txt`，快照 `D:/tmp/f14c-cur`）：
N1/N2 把两处已收口站点退回字面量 → 仅 L12 红；N3 台账凭空多一条而扫描集没变 → L12 红（空台账下"少"方向照样咬）；
N4/N5 房间/会话站点退回字面量 → L12 红；N6 把 `TASK_STATUS.PAUSED` 换成 `TASK_STATUS.PLANNING` → **不红**
（针钉的是"把取值重新打字"这件事，不钉是哪一档）；N7 只改 `orchestration.js` 一枚中文档名措辞 → **不红**。基线 `18 passed`，五文件还原 md5 全 MATCH。

**本轮真正的仪器账（我自己的驱动漏的一步）**：`D:/tmp/f14c-patch.py` 对 `contract.test.js` 写了 `assert newc != c`，
却**没有**对判据文件写对应断言——于是 LEDGER 那一步静默 no-op，驱动照样打印 `25629->25629` 而 rc=0。
发现它靠的是随后那次判据跑红（扫描集 `[]` vs 台账 5 条），而不是靠驱动的报告。
⇒ 规则化：**补丁驱动的每一步都要断言"这一步确实改了字节"（同尺寸不等于同文件，§5.44 的 RULE 72 同源）**；
只打印 before→after 而不比较，等价于把 no-op 报成成功。附带两条同族小坑：CJK 不能放进 `b'…'` 字面量（syntax error，磁盘没动＝无害），
`re.sub` 的替换串里 `\.` 会被当非法转义（必须 `lambda m: BYTES`）。

**门禁分母**（见读数文件，非回忆）：全量 vitest 77 文件 / 1034 例 / rc=0，`vite build` rc=0 / 32.97 s，模块化门禁 ERROR=0，
locale 闸门 rc=0，`check-doc-links.py` rc=1 属存量（本文件仍只有 `:137` 一条 WARN，非本轮引入）。

**余账（只报不改）**：`:562` 里同一行的 `store.runtime?.mode === 'remote'` **不在** L12 禁串集合——契约有 `MODE_WIRE` 但没被纳入
`banned`，这是覆盖面问题不是豁免；下一枚候选就是把 `MODE_WIRE` 加进现推集合再看命中。`orchestration.js:165/:395` 的契约**内部**比较
仍在扫描集（`views/`＋`components/` 的 `.vue`）之外，登记为契约内部余账。

**未入库警告**：本轮 3 个改动文件（视图／`contract.test.js`／判据文件）与 2 份 `reports/data/` 证据**一律未提交**，等用户按名点名；
写前图像在 `D:/tmp/f14c-snap`（补丁前）与 `D:/tmp/f14c-cur`（电池跑前＝落盘后）。相关：§5.44、[[project-vocab-copy-taxonomy]]。

## 5.46 F15 消息族 role/msg_type 收口＋L12 覆盖面裁决：MODE_WIRE 实测判为跨域撞名不纳（2026-09-29）

§5.45 留的余账是"覆盖面"而不是"豁免"：L12 的禁串集合只从任务/节点/可用性/熔断/会话/房间六张表现推，
`'remote'` 那处比较与消息族（`role`、`msg_type`）都在闸门外。本轮把消息族纳进来，并对 `MODE_WIRE` 做了**测量而非猜测**。

**wire 真值（本轮重新量，未复用旧引用）**：`platform/gateway/mox-platform-gateway-svc/src/alliance/experts_collaboration.rs`
按 `\brole:\s*"([a-z_]+)"` / `\bmsg_type:\s*"([a-z_]+)"` 计数得 `role {'user':3, 'expert':2, 'system':3}`（:830/:932/:1252、:843/:1263、:943/:1029/:1475）
与 `msg_type {'text':3, 'markdown':5}`（:834/:936/:1256、:847/:947/:1033/:1267/:1479）。
契约 `MESSAGE_ROLES` 的第 4 档 `assistant` 在该文件无生产者，但全库有（`mox-ai-expert-svc/src/alliance_ext.rs:351`、
`dialogue_sediment.rs:352/:389`）⇒ **不是过度声明**；这一步如果不量，就会被写成"契约多一档"的假账。
`MSG_TYPES` 的 `code/image/file` 无出参硬编码生产者，属写入侧（composer 可选、后端不校验），登记不当债。

**站点（`components/SessionThreadPanel.vue`，md5 `5fb31e978a26685b → 9057855302809c42`，15258 B / 261 行）**：
`:41` 的 `m.msgType !== 'text'` 改比 `MSG_TYPE_DEFAULT`，`:213/:214/:215` 的 `roleTag` 三元链改比 `MESSAGE_ROLE.USER/EXPERT/SYSTEM`；
筛选器 `:62` 早已是 `v-for="r in MESSAGE_ROLES"`（§5.38 的收口形态），本轮只补比较位点。
**契约侧新增派生常量**（`contract/sessions.js`，md5 `271cf438e52734c0`，23462 B / 503 行）：
`MESSAGE_ROLE = Object.freeze(Object.fromEntries(MESSAGE_ROLES.map(r => [r.value.toUpperCase(), r.value])))`
——键常量由值表**派生**，不另立第二份事实源（§5.44 同一规矩：视图比较用常量，字面量只在契约出现一次）。

**判据侧（`contract/vocabulary-ownership.test.js`，md5 `83fe066ba3668753 → 0d07119459c45de6`，25808 B / 383 行）**：
L12 的 `banned` 现推集合追加 `...MESSAGE_ROLES.map(r => r.value), ...MSG_TYPES.map(t => t.value), MSG_TYPE_DEFAULT`。
用例数仍是 **18 例**（本轮没有新增 `it`，是**覆盖面加宽**）⇒ 分母不变而判据更严，这正是"禁串集合从对象现推"应有的形态。

**MODE_WIRE 裁决（实测）**：把 `MODE_WIRE` 加进 `banned` 前先在内存里反事实跑一遍——命中恰 **1 处**，
是 `AllianceConsoleView.vue` 的 `store.runtime?.mode === 'remote'`。而 `'remote'` 在前端另有**不是**该枚举身份的用法
（`resultKind` 由 `store/alliance-collab.store.js:47` 现场派生），同串不同域 ⇒ 按 §5.37 (c) 档**不纳入**，
并把这段裁决写进判据文件的三行注释里（注释要留在测试里，不然下一位作者会以为漏了）。

**电池 4 枚，0 不符**（`reports/data/f15-mutation-witness-20260929.txt`，快照 `D:/tmp/f15-snap`＝写前、`D:/tmp/f15-cur`＝落盘后）：
P1/P2/P3 分别撤掉 `MSG_TYPE_DEFAULT`、`MESSAGE_ROLE.USER`、契约派生常量 → 仅 L12 红；
P4 只改消息族 role 的**中文措辞** → **不红**（针钉取值与管道，不钉文案）。基线 `18 passed`，跑前跑后各一次，三文件还原 md5 全 MATCH。

**本轮仪器账（两种，都是我自己的驱动造成的）**：
① 补丁第一版在 `判据 import 锚点命中 0` 处 REFUSED、磁盘未动——判据是从 **barrel**（`:20` 一行 `from '@/modules/expert-alliance/contract'`）导入的，
不是从 `sessions.js` 深路径，锚点写错了子句；② 第二版把 `MSG_TYPE` 派生常量插在 `MSG_TYPES` 声明**之前**，
ES 模块 TDZ 直接炸，还顺手把 `/** SessionMessage.msg_type … */` 文档注释孤儿化 ⇒ 电池基线 `rc=1 / no tests`＝INVALID（不是判据红，是夹具坏）。
修法是删掉那枚**当前并不 needed** 的派生常量、把 `MESSAGE_ROLE` 紧接在 `MESSAGE_ROLES` 之后追加。
⇒ 规则化：**新增派生常量必须排在被派生表之后，且插队前先看相邻的文档注释归属**；"先造常量再看要不要用"是本次多余的复杂度。
过期快照目录按记忆规矩改名挪走而不是覆盖（`D:/tmp/f15-cur.bad-tdz`）。

**门禁分母**（全部现场复跑，逐字见 `reports/data/f15-readings-20260929.txt`）：全量 vitest **77 文件 / 1034 例 / rc=0**（`Duration 30.32s`），
`vite build` rc=0 / `✓ built in 35.58s`，模块化门禁 `ERROR=0`，locale 闸门 rc=0，
`check-doc-links.py` **rc=1 属存量**（WARN 120 条，本文件唯一 WARN 在 `:137`，本轮未引入新悬空）。

**未入库警告**：本轮 3 个改动文件（判据／`SessionThreadPanel.vue`／`sessions.js`）与 2 份 `reports/data/` 证据**一律未提交**，
不动 git 索引，等用户按名点名。状态码本轮现取（`git status --short -- <按名>`）：视图／契约／本文件为 `AM`（索引里是**并发作者**暂存的旧内容，我的编辑在工作区），
判据文件与两份证据为 `??`（**无 git 恢复源**，只能靠 `D:/tmp/f15-snap`）；`git diff --cached --name-only` 实测 **436 条** ⇒ 索引被并发作者占用，
任何"暂存态"引用都必须本轮重量。驱动 `D:/tmp/f15-patch.py`、`D:/tmp/f15-battery.py` 未落库 ⇒ **没牙，不要接 CI**。
相关：§5.44、§5.45、§5.37、[[project-vocab-copy-taxonomy]]。

## 5.47 F16 熔断器档位的"要不要第三档"不再由眼睛担保：L13 实时解析 Rust 出参（2026-09-29）

§5.42 落 `BREAKER_STATE` 时留下的问题（`AllianceConsoleView.vue:203` 的 `breakerStateLabel(c.state)` 该不该补 `half_open`）
本轮用**服务侧真值**结掉，而不是用产品直觉：出参在 `platform/gateway/mox-platform-gateway-svc/src/alliance/experts_dispatcher.rs:498-502`
现场算——`let state_str = if count >= config.circuit_breaker_threshold { "open" } else { "closed" };`，
`json!`（:503-507）只发 `expert_id` / `failure_count` / `state` 三键，行集先经 `.filter(|(_, &count)| count > 0)`；
`half_open` 在全库只是 scheduler-core 的配置项名 `half_open_probes`（`llm_router.rs:438/:447`），**从不进这个端点**
⇒ **契约两档完备，不开第三档**。判决本身不是产物，产物是下面这本会自己咬人的账。

**L13（新增，判据文件 18 → 19 例）**两条通道都从对象现推，不写死档位：
① 用正则从 `experts_dispatcher.rs` 抓 `state_str` 赋值块里的字符串字面量 ⇒ 与 `Object.values(BREAKER_STATE)` **双向相等**，
并逐档要求有中文文案且 `label !== key`；② 抓同一块的 `json!` 行键（snake）机械转 camel ⇒ 与 `model/normalize.js:642`
`circuitBreakers` 行映射的键集合**相等**。两通道各带分母格（解析块必须命中、wire 档位 ≥2），解析不到就红并指名
"服务侧换了写法 ⇒ 这本账要跟着改，不许静默恒真"。⇒ 后端真加第三档时，红的是这条账，不是界面上凭空出现的英文 `half_open`。

**电池 3 枚 0 不符**（`reports/data/f16-mutation-witness-20260929.txt`，快照 `D:/tmp/f16-snap`）：
Q1 契约凭空加 `HALF_OPEN: 'half_open'` → 红=[L13]；Q2 归一化删掉 `failureCount: num(c.failure_count),` → 红=[L13]；
Q3 只把 `open` 档中文「已熔断」改「熔断中」→ **不红**（针钉取值与键集，不钉文案）。三文件终检 md5 全 MATCH，基线跑前跑后各一次 `19 passed`。

**仪器账（本轮三条，都是我自己的量具）**：① L13 第一版把仓库根算成 `SRC_DIR + '../platform/…'`，
而 `SRC_DIR` 已是 `frontend-ui/src` ⇒ `readFileSync` 抛，第一遍报的 `1 failed` 是**夹具坏不是判据红**（症状同形，靠读栈归因）。
② **`reporter` 的装饰字形不可解析**：第一遍电池把确实 `rc=1 / 2 failed` 的 Q1/Q2 记成 `BLIND PASS`，
因为 `×` 过 Windows 控制台（GBK）后匹配不上——改成 `vitest -t "L13"` **按名过滤**、只取 `rc` 与 `failed|skipped` 计数才可信。
这是"rc 不许从管道尾巴拿"的同族第二次显形：**读判决要选不受呈现层影响的通道**。③ 全量 vitest 的 `Tests` 总数行连续三轮被我漏掉，**真因不是 grep 模式而是 ANSI 色码**——`Tests` 前带 `\x1b[…m`，
`^\s*Tests` 恒不匹配；剥色码后一次命中，现量 **`Tests  1035 passed (1035)`**（解析判决行要先 `re.sub(r'\x1b\[[0-9;]*m','',…)`）。

**门禁分母（现场复跑）**：判据单跑 `19 passed` ×2、全量 vitest rc=0 / `Test Files 77 passed (77)`、`vite build` rc=0 / `✓ built in 32.05s`、
模块化门禁 `ERROR=0`、locale 闸门 rc=0、`check-doc-links.py` **rc=1 属存量**（120 WARN，本文件唯一 WARN 仍在 `:137`）。

**未入库警告**：本轮代码侧只动判据文件一个（`contract/vocabulary-ownership.test.js`，状态 `??`＝**无 git 恢复源**，
恢复靠 `D:/tmp/f16-snap`），加两份 `reports/data/f16-*.txt` 与本文件（`AM`），**一律未提交**；
未执行 `add`/`commit`/`stash`。驱动 `D:/tmp/f16-battery.py` 未落库 ⇒ **没牙，不要接 CI**。
相关：§5.42（`BREAKER_STATE` 的来历）、§5.40（界面字段先问服务侧真值）、[[project-vocab-copy-taxonomy]]、#33。

## 5.48 F17 模块外"直印 wire 枚举字段"形状普查＝**阴性结果**：55 处命中 → 0 处确认债（2026-09-29）

#33 余下的"模块外 20 处直印"这一族，本轮先做普查再定价，**结论是一个数字都没有改到界面**——
因为按 §5.41 的三条尺子走完，55 处形状命中里**确认债务 0 处**，只有 1 处留开。这台普查器本身反而成了本轮的产物。

**口径与读数**（`D:/tmp/f17-census.py`，未落库＝没牙，禁止接 CI）：`frontend-ui/src` 的 `*.vue`（排除 `modules/expert-alliance`，
模块内已由 L9/§5.41 管；排除 `.test`/`.stories`）⇒ 扫描 **107** 文件，命中 **55** 处
（命中＝`{{ … }}` 里有 `<path>.(status|state|type|role|kind|mode|msgType|msg_type|phase|level)` 成员路径**且整条 mustache 不含函数调用**）。
与 §5.41 的"270 文件 ⇒ 30 处"**口径不同不可比**（那次含模块内、含 `<code>` 行），不许据此说债务涨跌。

**仪器退化（本轮最该带走的一条）**：分桶把 55 处**全判进同一个桶**，等于没分桶——
① 分类谓词写成了"该**文件**任意位置出现 `*Label`"，而 `.vue` 里几乎总有一处 `{{ item.label }}` ⇒ 谓词恒真、C 桶吞下一切；
分类谓词的粒度必须与判据一致（要问"**这条 mustache 的值域**有没有出口"）。
② 证据面那支是 `CODE_TAG.search(line) or inner.startswith('item.') and False` —— 我留下的 `and False` 死逻辑让 A 桶**从未可能命中**；
⇒ "某个桶计数为 0"要先查那根针是不是断的，而不是先信"这类不存在"（§5.44 静默 no-op 的同族）。

**四倍代表位逐条溯源**（全文与判定表见 `reports/data/f17-direct-print-census-20260929.txt`）：
`AgentFlowPanel.vue:72/:136` 的 `a.role` 由 `:168` 组件自注的对象形状定义（本地 Agent 花名册，`:196` 还写进生成脚本）＝**跨域同名，不是债**；
`RegisterExpertDialog.vue:193` 是 `EXPERT_TYPES[type] || type` 的查表＋坏值回显＝B 档设计；
`GovernanceConsoleView.vue:137` 的 `row.gate.status` 所在文件**零处** import `expert-alliance`＝治理域自己的账，不在本族；
唯一留开＝`MessageBubble.vue:415` 的 `sm.role==='user'?'我':(sm.role==='assistant'?'AI 助手':'系统')`——
三元链只有两支＋兜底，**若** `sm` 来自联盟会话消息（值域含 `expert`，`experts_collaboration.rs:843`）则 `expert` 会被折叠印成「系统」，
属 §5.38（缺 `closed`）／§5.41（缺 `away`）同型缺陷；修法已定位（走 `messageRoleLabel`，`contract/sessions.js:129` 四档表 `:42-47`），
代价是「我／AI 助手」措辞会变 ⇒ **需要先出参溯源，不溯源就动手等于把别人的值域焊进这条链**。

**诚实缺口**：55 处只人读了 4 条，其余 51 条状态是"未判定"不是"无债"；分桶谓词未修，故该驱动 C 桶数字不要引用；
本轮零代码改动 ⇒ 无电池、无新判据格（判据文件仍 `it(` 实测 **19 例**）。
另记一条自造缺口：定性叙述覆写了首份 55 行原始清单（同一路径），原始快照只可重跑复现 ⇒
**普查原始读数与定性叙述不许共用一个文件名**。

**未入库**：本轮只新增 1 份 `reports/data/f17-direct-print-census-20260929.txt`（`??`）与本文件，**未动任何 `.vue`**，
未执行 `add`/`commit`/`stash`。相关：§5.38、§5.40、§5.41、#33。

## 5.49 F18 可达性账必须先于值域账：`MessageBubble.vue` 在 app 内零真 importer，`:415` 的字面量三元不是活界面债（2026-09-29）

§5.48 留下的唯一可测问题＝`components/MessageBubble.vue:415` 的
`sm.role==='user'?'我':(sm.role==='assistant'?'AI 助手':'系统')` 是不是"缺档折叠"。
本轮做完出参溯源，结论是**两重不并**，且过程中立了一条新规矩。**本轮零代码改动**，
全部读数（三件挂载通道仪器／行号清单／值域对照）在 `reports/data/f18-messagebubble-reachability-20260929.txt`。

**溯源三件仪器**：① `sm` 出自本文件 `:559` 的 `shareMessages` 计算属性，角色由 `:567`
**客户端现推**（`m?.role || (m?.system ? 'system' : 'assistant')`），入参只有 `props.sessionMessages`／`props.msg`；
② 全库按**多行具名导入**扫 `import {…MessageBubble…} from`＝**0 处**、`import.meta.glob` 全库＝**0 处**、
字符串型 `<component :is>` 只有图标与路由组件 ⇒ app 运行时**零挂载**，唯一真消费者是
`stories/MessageBubble.stories.js`（Storybook），`components/index.js:18` 的 barrel 再导出（该 barrel 共 32 条出口）无人按名取用；
③ `sessionMessages` 这个标识符的活命中全在联盟 API 侧（`contract/endpoints.js:48` 的端点键 →
POST `/api/experts/sessions/:id/messages`），与本 prop 同名而不同物 ⇒ 没有任何 `.vue` 传 `:session-messages`，
该 prop 恒为默认空数组，`shareMessages` 恒为 `[props.msg]`。

**裁定**：(1) 可达性归零 ⇒ `:414/:415` 不在任何用户看得到的界面上，属**孤儿账**
（可达性账的口径：提及数会落空于注释／路由 name／`.stories.js`），不是归一化债；
(2) 即便接上，`:567` 现推的角色域与联盟会话族 `MESSAGE_ROLES`（`contract/sessions.js:42-47`，四档）
只在 `user`/`assistant`/`system` 上重叠，契约独有的 `expert` **永不到达**本组件 ⇒
第三分支「系统」是**未知值回落**而非把已登记档折进别的档，按 §5.37 (c) 档（跨域同名）不并。

**新规矩（本轮真正的产出）**：**F17 那 51 条未判定命中的下一步不是逐条读值域，而是先过可达性账**——
一个组件若零 importer，它的措辞就不存在"用户看得到的错"，按债定价等于给死码开账单。
可达性判定必须把**三种挂载通道**（多行具名导入／`import.meta.glob`／字符串 `:is`）各扫一遍；
只 grep 组件名会把 barrel 再导出和 stories 算成消费者（§5.48 的教训换了方向重演：**覆盖面＝针枚举的通道集合**，
单行 grep 还看不见换行写的 import 子句）。

**F17 四枚抽查代表就此全部结案**：`AgentFlowPanel.vue:72/:136`（组件自造花名册，另一套词汇）、
`RegisterExpertDialog.vue:193`（`EXPERT_TYPES[x] || x`＝未知值回显设计）、
`GovernanceConsoleView.vue:137`（另一域的 `row.gate.status`，该文件零 `expert-alliance` 导入）、
本条（app 内零 importer）。⇒ **F17 判为阴性**：模块外"直印契约已登记取值"至今没有一枚活界面缺陷。

**本节自己的限制**：证明的是"运行时零挂载"，**不是**"该组件该删"——退役与否属可达性账，需用户点名；
若将来引入自动注册插件（实测本仓库无），第②件仪器要重跑。
**未入库**：新增仅本档案一节＋该读数文件（`??`），未执行 `add`/`commit`/`stash`。相关：§5.48、[[project-vocab-copy-taxonomy]]。

## 5.50 F19 可达性仪器当场翻账：四通道比三通道多出一条边形态，就把 47 处"孤儿"翻回活界面（2026-09-29）

§5.49 立下"先过可达性账再谈值域"的规矩，本轮是它的第一次执行，**执行结果同时修正了那条规矩的预期**。
两遍原始读数分开进两份档案（`f19-direct-print-screen-20260929.txt` 三通道版／`…-v2-20260929.txt` 四通道版＝权威），
定性与对账在 `f19-screen-verdict-20260929.txt`；驱动器 `D:/tmp/f19-screen.py` 未落库＝没牙，禁止接 CI。**本轮零代码改动**。

**读数**：扫描集 107 个模块外 `.vue` ⇒ 形状命中 55 处／30 文件（口径与 §5.48 同，两处断针已修：
`evidence` 的 `and False` 死逻辑、`imported` 改成按导入的具体出口名问）。
- 三通道版（模板标签／多行具名导入／默认导入／字符串 `:is`）报 **R0 零消费者 51 处／26 文件**、R3 活界面 4 处；
- 补上第四通道后变成 **R0 只剩 4 处／2 文件**、R3 **51 处／28 文件** ⇒ 逐条按 `(file,line)` 对账得 **47 处翻案**。
- v2 仍判零消费者的只有 `MessageBubble.vue:164/:415`（引用方全是 `.stories.js`）与 `AgentFlowPanel.vue:72/:136`（四通道全 0）。

**假阴的机制（本轮真正要带走的东西）**：路由挂载写的是 `component: () => import('../views/XxxView.vue')`——
按**路径**、**动态**、**不出现组件名**，所以只按名字找边的仪器会把 24 个视图报成孤儿。
第四通道 `path-import` 按 `['"]…/<Name>.vue['"]` 找路径串，并**排除以 `export` 开头的行**：
barrel 的 `export { default as MessageBubble } from './MessageBubble.vue'` 同样含路径串，
不排除就会把 §5.49 刚判死的零消费者组件算成活的——**同一行代码能同时制造假阳与假阴**，两个方向都要各设一道对照。
这是可达性账"提取器少一种边形态 ⇒ 造假日孤儿"的第二次实证，且这次是本轮新写的仪器当场复现。

**对 §5.49 的口径修正（引用那条规矩时必须连这句一起引用）**：可达性筛**只对 `components/` 有判别力**，
对 `views/`（路由挂载）**恒真**、跑了等于没跑。v2 的 R3 分布就是这句话的数字：
`views/admin/panels` 24／`views/expert` 8／`views/expert/panels` 4／`views/project` 4／`views/workflow` 3／
`views/ai` 2／`views/graph` 2，另 `components/expert`、`modules/governance/views`、`views/project/panels`、
`views/workspace/panels` 各 1。⇒ §5.49 里"先筛一遍能缩队"的隐含预期**被证伪**（55 只筛掉 4），
下一步是**从 admin 面板族的 24 处开始做值域溯源**，而不是再换一把筛子。

**前面的账要不要改**：§5.48（F17 阴性）与 §5.49（`MessageBubble` 零 importer）**判定都不变**——
本轮 R0 剩下的两文件正是那两轮判过的。变的是**预期与顺序**，已在本节明说。

**缺口（不写成已完成）**：51 处 R3 本轮**一处未溯源**，状态是"活界面候选"不是债（`R3` 的出口谓词仍按名字问，
(b) 两档版面与 (c) 跨域同名都会落进来）；`path-import` 用"前 160 字符内有 `export` 即跳过"的启发式，
理论上会误伤更远的多行 `export … from`（本轮未见，但也未证伪其不存在）。
判据文件仍 19 例（本轮实测 rc=0／`Tests 19 passed (19)`／2.81 s）；全量 vitest 与 `vite build` 本轮未跑（零代码改动）。
**未入库**：三份 `reports/data/` 档案（`??`）＋本节，未执行 `add`/`commit`/`stash`。相关：§5.48、§5.49、[[project-vocab-copy-taxonomy]]、[[project-frontend-reachability-ledger]]。

## 5.51 F21 L12 的扫描集外溢：模块外 `views/expert/**` 的 27 处字面量比较 ⇒ 18 处并成契约常量，9 处进带次数与理由的双向豁免表（2026-09-29）

§5.44 落 L12 时扫描集只钉了 `modules/expert-alliance/{views,components}`，§5.50 收尾时把这行写成**覆盖面洞**
（`src/views/expert/**` 在集外）。本轮按自己立的规矩执行：**先跑反事实扩集实测再纳**，不直接纳。
原始读数全部在 `reports/data/f21-l12-widen-readings-20260929.txt`（现场量，含五文件逐行差异、普查输出、电池日志逐字）。

**反事实现量**：现扫描集 21 文件 ⇒ 命中 0 处（与常驻 `LEDGER = []` 同口径，这是复制仪器与常驻判据的一致性校验，不是判决）；
纳 `views/expert/**`（+7 文件 ⇒ 28）⇒ **命中 27 处**。禁串集合本轮**未加宽**（仍 28 值，见"缺口"）。

**18 处同域债已并（文案一字未动，站点与所用常量）**：
- `views/expert/AllianceTaskView.vue:40/:48/:51/:221` → `TASK_STATUS.COMPLETED`，`:104` → `TASK_STATUS.PAUSED`（该文件 `:287` 早就 import 了 `TASK_STATUS`，纯重新打字）；
- `views/expert/panels/ExpertEnterprisePanel.vue:597/:837` → `SESSION_STATUS.ACTIVE`，`:691` → `SESSION_STATUS.ARCHIVED`，
  `:615/:715/:844` → `BREAKER_STATE.OPEN`，`:844` 另含 `BREAKER_STATE.CLOSED`，`:832` → `MESSAGE_ROLE.USER`（8 处命中／7 站点，`:844` 一行两比）；
- `views/expert/panels/ExpertOrchestratorPanel.vue:373/:402` → `ORCH_STATUS.COMPLETED`（**域是编排不是任务**，见下）；
- `views/expert/ExpertPlazaView.vue:188` → `BOOKING_STATUS.PENDING`，`:203` → `BOOKING_STATUS.COMPLETED`/`.CANCELLED`。
新增两处跨目录 import（OrchestratorPanel 与 PlazaView 各补一枚常量名到既有的 `@/modules/expert-alliance/contract` barrel 语句上），
`check-frontend-module.py` rc=0。

**OrchestratorPanel 两处为什么算并而不算豁免**：`x.status` 读自 `getOrchHistory`、`execStatus` 读自 `orchestration.status`，
值域是 §5.43 的编排七档 `ORCH_STATUS`；它撞红只是因为 `completed` 同时是 `TASK_STATUS`/`NODE_STATUS` 的取值。
两边都改成 `ORCH_STATUS.COMPLETED` 后**字符串完全不变**（该表 `COMPLETED: 'completed'`），于是"同串不同域"这条
抗辩从此不可再用——这比登记成豁免结实：豁免靠说，常量靠算。

**9 处判为跨域同串／本地状态，进常驻豁免表**（`views/expert/AllianceTaskView.vue|user` ×1：`aiMessages` 是本地 ref，
`:412-429` 只写 user/assistant，不是 wire 消息族；`ExpertConfigView.vue|system` ×6：专家来源 type 与 testScene；
`ExpertConfigView.vue|failed` ×1：`llmConnectionStatus` 是前端连接自检；`ExpertPlazaView.vue|online` ×1：快切标签 id）。

**L12 的第三根针（本轮设计增量）**：豁免不是把命中拿走，而是把它换成一份**必须持续成立**的账——
每条 `{ n, why }`：`n` 出现次数（多＝新债、少＝那行已改、条目该删），`why` 空串即红。
判据仍是 19 例（用例数不变而覆盖面变宽，与 §5.46 F15 同一形态），分母 `files.length ≥ 28`（实测 28）。
另补 8 条逐站点"必须保持 0 命中"的收口断言（任务族 2／会话族 1／熔断族 1／角色 1／编排 1／预约 2）。

**电池 8 枚／0 不符**（`D:/tmp/f21-battery.py`，未落库＝没牙，禁止接 CI）：
M1-M6 全打红＝撤一处已收口的比较（两处独立站点）、撤一条豁免、把 `n` 从 6 写成 5、塞一条盘上不存在的豁免条目、把 `why` 清空；
M7-M8 全保持绿＝只改中文文案（'继续执行/启动任务'→'继续跑/开始跑'）、新增一次契约外取值 `=== 'bot'` 的比较。
每枚跑完即从 `D:/tmp/f21-after` 还原并核 md5，五文件还原后哈希与基线逐一相同（档案 `[3]` 段逐字）。

**仪器自捉三处（都属于"仪器看不见自己要测的结果"族，不是被判对象的缺陷）**：
① vitest 非 TTY 输出仍带 ANSI 转义码，判决行以 `Tests ` 开头但被 `\x1b[2m` 抢在前面 ⇒ 第一版读到 `no-summary-line`；
② 我的兜底守卫把自己判死了——`summ` 初始值就是 `'no-summary-line'` 这个字面量，`'no-summary-line' not in summ` 恒假 ⇒
   基线断言在真读数（`Tests 19 passed (19)`）面前报失败。守卫与占位符不能共用同一个串；
③ 一开始用 `-t 'L12 视图不许…'` 中文过滤，若 argv 编码不匹配会**选不出任何例而 rc=0**，全绿是假象 ⇒ 改成整份文件 19 例都跑，
   并断言基线必须出现 `Tests  19 passed`。另外档案驱动器 `%d` 后紧跟中文的 `'% 与 %'` 直接 `ValueError`，
   崩在写盘之前所以磁盘没动（这层"崩在写前"是唯一的止损）。

**门禁与分母（全部本轮现量，勿与上一轮互推）**：判据单文件 rc=0／`Tests 19 passed (19)`；
全量 vitest **77 文件 / 1035 例 / rc=0**（上轮 1034，差的 1 例是 §5.47 F16 加的 L13，本轮**用例数零增长**）；
`vite build` rc=0／42.61 s；`check-frontend-module.py` rc=0；交叉核 `check-locale-format-outlets.py` rc=0／270 文件／余账 2 处（本轮未碰时间口径）。

**缺口（不写成已完成）**：
- 禁串集合仍不含 `ORCH_STATUS`/`MODE_WIRE`/`FUSION_STRATEGY`——本轮只动扫描集不动值集，OrchestratorPanel 的并案是靠人工定域；
- 针只咬 `===/!==/==/!=` 右侧引号字面量，`switch (s) { case 'completed': }` 与 `arr.includes('completed')` 两种形状不在针上。
  本轮用 grep 现量这两型在 28 文件内 **0 处命中**，所以暂不加宽；把形状补齐时先按 §5.46 的规矩跑反事实；
- 只扫 `.vue`，`.js/.ts` 的 composable 与 store（`useWorkspaceData.js` 那类）仍在集外；
- 普查驱动 `D:/tmp/f21-l12-widen.mjs` 与电池驱动未落库＝没牙；`grab('sessions.js','MSG_TYPE_DEFAULT')` 会多解出下一张表
  （返回值恰好 ⊆ 禁串集合，无害但记账时要说清）。

**顺带挖到的下一族（本轮零改动，只报）**：`ExpertEnterprisePanel.vue:843` 的 `cbStatusLabel` 是契约
`breakerStateLabel`（`enums.js:368-375`，open=已熔断／closed=正常）之外的第二本熔断器文案表，且它的 else 分支把
**任何未知档**显示成"半开"——§5.47 已由 Rust 现推证明该端点只产 open/closed ⇒ 这是编造服务侧不存在的状态，不是文案差异。
三出口（全并／只改 else／契约补第三档）见 task #41，未点名不动。

**未入库**：`reports/data/f21-l12-widen-readings-20260929.txt`（`??`）＋本节＋五个源文件改动（其中两个视图文件本轮新增 import），
未执行 `add`/`commit`/`stash`；git 索引仍由并行作者持有。相关：§5.44、§5.46、§5.47、§5.50、[[project-vocab-copy-taxonomy]]。

## 5.52 F23 L12 的针加宽到四种重写形状（字面量在左／switch-case／includes·indexOf）：语料 0 命中，牙由植入夹具证明（2026-09-29）

§5.51 结尾写的缺口就是本轮的活：**针只咬 `op '字面量'` 一型**，
把 `=== 'completed'` 改成 `'completed' === x`、`case 'completed':`、`TERMINALS.includes('completed')` 三种写法，
同一本债就从账上消失。读数在 `reports/data/f23-l12-shape-readings-20260929.txt`（5047 B／59 行／md5 796bb82ad435）。

**反事实先行**（`D:/tmp/f23-shapes.mjs`，未落库＝没牙）：同一 28 个 `.vue` 扫描集上四型分别
**A 9 处／B 0 处／C 0 处／D 0 处** ⇒ 加宽不新增债、不新增豁免，代价为零，收益是把三种未来写法纳管。
（该 scratch 的禁串集合是手写 26 值，常驻判据由 `Object.values` 现推 28 值；两口径在 A 型同为 9 处、B/C/D 同为 0，
所以这次可以拿它当普查用，但口径差要印出来，不许悄悄混。）

**落法**：判据内 `const cmp = /…/` 换成 `NEEDLES = [['A…'], ['B…'], ['C…'], ['D…']]` 四型循环，
并加两样仪器自身的账——`expect(NEEDLES.length).toBe(4)`（覆盖面按形状算，不是按文件数算）
与一行 `console.log('[L12 形状] 扫描 28 个 .vue／四型命中 A=9 B=0 C=0 D=0')`，
让"某型永远报 0"这件事在输出里可见，而不是只在源码里存在。
B 型带 `(?!\s*['"])` 负向预查，避免 `'a' === 'b'` 这种同行双字面量被 A/B 各记一次；
**语料、禁串集合、扫描集、`LEDGER`/`EXEMPT` 账目一字未改**，判据仍 19 例（用例数第二次零增长而覆盖面加宽，与 §5.46、§5.51 同形态）。

**电池 5 枚／0 不符**（`D:/tmp/f23-battery.py`，植入点＝`AllianceTaskView.vue` 的 contract import 之后，跑完即还原）：
M-B1/M-C1/M-D1 各植一枚对应形状的重新打字 ⇒ 三枚全红，且形状计数分别只把 **B=1／C=1／D=1** 顶起来（A 始终 9），
这同时证明三型各自有牙、不是被同一型重复数出来的；M-E1 三型同时用契约外取值 `'blocked'` ⇒ 全绿（口径边界）；
M-E2 用 B 型写已豁免的 `'user'` ⇒ 红，说明**豁免账是跨形状合计的**，换个写法多打一次同一取值照样算新债。

**门禁与分母（本轮现量）**：判据单文件 rc=0／`Tests 19 passed (19)`／4.21 s；
全量 vitest **77 文件 / 1035 例 / rc=0**（与 §5.51 同值＝本轮零用例增长的独立见证）；`vite build` rc=0／48.36 s；
`check-doc-links.py` rc=1／120 WARN＝与存量同数，本轮未引入新悬空。
口径提醒：本档案 `[4]` 的"行数"按 `b'\n'` 计数，§5.51 档案按 `split('\n')` 计数，同一文件会差 1 行（结尾换行），不是文件被改过。

**残余缺口（不写成已完成）**：`.js/.ts` 里的 store/composable（`useWorkspaceData.js` 那类）仍在 L12 扫描集外；
`some/filter` 回调内的比较归 A/B 管、数组元素位上的字面量（`['completed','failed'].includes(x)`）不在任何型上——
后者其实是**同串不同用途**的常见写法，纳管会立刻制造需要逐条裁决的账，故本轮明确不纳并留此句为据。
普查与电池驱动未落库＝没牙，禁止接 CI。**未入库**：本档案＋本节＋判据文件那一处改动。
相关：§5.44、§5.51、[[project-vocab-copy-taxonomy]]。

## 5.53 F24 L12 伸进 `.js`：store/composable/api 里读 wire 取值的位点按名在册（2026-09-29）

**起因**＝F23 结案时点名的残留：判据只扫 `.vue`，`.js/.ts` 的 store/composable/api 在扫描集外。本轮先跑只读反事实普查再决定纳不纳（规矩＝扩集前现量，不许直接纳）。

**反事实现量**（`D:/tmp/f24-storecensus.mjs`、`D:/tmp/f24-js-classify.mjs`，未落库＝没牙）：全 src 的 `.js/.ts`（去测试、去定义位点＝`contract/`＋`model/`）四型命中 **30 处 / 123 文件**，逐条走到底按域分：

| 归属 | 处数 | 判定 |
|---|---|---|
| `composables/useAllianceTasks.js:24/26/37` 比 `task.status` | 3 | **同域债**：task 来自 `api.getAllianceTasks()`，就是任务七档 ⇒ 并 `TASK_STATUS.COMPLETED` |
| `api/allianceTaskModel.api.js:58` 比 `data.fusion_status`／`data.status` | 2 | **同域债＋新档**：见下 FUSION_STATUS |
| `stores/ai.store.js`（`m.role` 8 处）、`composables/projectContext.js`＋`stores/project.store.js`（`p.status==='active'` 4 处）、`modules/admin-lowcode/pages/*`（`row.status` 6 处）、`useTaskOrchestration.js`（本地子任务档 `waiting/inProgress/...` 2 处）、`useMessageActions.js`（TTS `speechState`）、`useWhiteboard.js`（工具 id）、`useWorkspaceData.js`（文件/历史条目 type）、`stores/alliance.store.js:140`（`runState`＝running/done/error 本地态，且该 store 是零消费者空壳） | 25 | **跨域同名 (c) 档 ⇒ 出账**，纳进来就是把别的域判成债 |

⇒ 裁决＝**按名在册 2 份文件**，不纳全套。这一步的判据不是"看着像联盟"，而是"比较的左侧确实读自联盟 wire 字段"（`useTaskOrchestration.js` 虽然 `import` 了契约，但它的 `task.status` 是自己写的六档本地态 ⇒ 光看 import 会判错）。

**新档 FUSION_STATUS（§5.53 的第二件事）**：`api/allianceTaskModel.api.js:58` 比的 `pending` 属融合域，而契约没有融合档。wire 真值现推：`alliance.rs:544` `completed|partial`、`alliance.rs:1235` `completed|pending`、`alliance_remote.rs:968-969` `pending|pending` ⇒ 值域 **{pending, partial, completed}** 落 `contract/enums.js`。同处的 `data.status` 由 `alliance.rs:1235/1247` 的 `status_str` 派生（只有 `completed|pending`）⇒ **与任务七档 TASK_STATUS 同串不同域**，两支都比 `FUSION_STATUS.PENDING`（先按 TASK_STATUS 写错过一次，读 Rust 出参后改回——这就是 F10"界面字段先问服务侧真值"的牙）。

**常驻账改动**（`modules/expert-alliance/contract/vocabulary-ownership.test.js` L12，用例仍 **19 例**＝覆盖面变严而账面不涨）：① 禁串集合并入 `...Object.values(FUSION_STATUS)`（`partial` 是本表独有的新增覆盖，另有 `expect(banned.has('partial'))` 点名）；② `JS_IN_SCOPE` 名单两份文件靠**旁路**进扫描集——前缀过滤器只按目录收（模块 `views|components` ＋ `views/expert/**`），名单不通过就会被先丢掉（本轮第一次跑就是这样：判据红在"扫描集塌缩 实测=28"，而不是命中不符）；③ 分母两格：`files.length ≥ 30` ＋"在册 `.js` 数必须恰等于名单长度"（防止名单写了却没进集）；④ 逐站点零命中格由 8 条增至 **10 条**（新增 `api/allianceTaskModel.api.js|pending`、`composables/useAllianceTasks.js|completed`）。现量：`[L12 形状] 扫描 30 个文件（.vue＋在册 .js）／四型命中 A=9 B=0 C=0 D=0`＝并入两份 `.js` 后命中数不变（那 5 处已改比常量）。

**电池 7 枚／不符 0 枚**：M1/M2 撤两处已收口的 `.js` 常量 ⇒ 红（A 9→10）；M3 把 `partial` 重新打字 ⇒ 红＝证明新档真的进了禁串集合（不是只加了张表）；M4 契约外的 `draft` 新增比较 ⇒ 必须不红（口径边界）；M5 只改中文文案 ⇒ 必须不红；另含未变异基线与还原后复验。三文件跑前后 md5 逐一 MATCH，判据文件 md5 全程未变。

**仪器自捉（同一族：针读不出自己测的结果）**：第 1 版解析器只匹配 `Tests N failed`，而全绿跑的输出里根本没有这个串 ⇒ 基线与两枚"必须不红"的正对照全被读成 `no-verdict-line`（BLIND），电池印出"不符 4 枚"——**不是闸门不稳，是仪器看不见绿**。修法＝failed/passed 两个正则各自现读，且**先拿两份真实日志（一红一绿）干测解析器**，再上电池。另：后台任务的控制台日志在 GBK 下中文成乱码 ⇒ 电池逐行同时写 UTF-8 日志文件，归档只抄日志不抄 stdout。

**残留（下一族，别当已收口）**：`.js/.ts` 里 25 处跨域同名要动的话属**企业域/项目域/聊天域各自开档**（与 #41、admin 面板族同批待裁决）；`Set([...])`／数组元素表形态（`useAllianceTasks.js:3-8` 的 `activeStates`/`taskActions` 六处取值）仍不在四型针上——那是"元素表在调用左侧"的写法，纳它要先跑一次同口径反事实。

**读数与状态**：原始读数逐字归档 `reports/data/f24-js-scope-readings-20260929.txt`（12486 B／162 行，md5 前 12 位 `8976c6d6d3b5`；该文件由 `D:/tmp/f24-archive.py` 从九份日志抄出，拒覆盖已存在）。本轮现量（插本节之前跑的，抄自该归档 [5][7][8][9] 块）：单文件判据 `Tests 19 passed (19)`＋`[L12 形状] 扫描 30 个文件（.vue＋在册 .js）／四型命中 A=9 B=0 C=0 D=0`；全量 vitest `Test Files 77 passed (77)`／`Tests 1035 passed (1035)`／rc=0／46.65 s；`vite build` rc=0／`✓ built in 36.72s`；`check-frontend-module.py` rc=0（ERROR=0）；`check-locale-format-outlets.py` rc=0；`check-doc-links.py` rc=1／120 WARN（＝存量基线，本轮不增不减；插本节后再跑一次复验）。**一律未入库**（未执行 `add`/`commit`/`stash`，git 索引由并行作者持有）。

## 5.54 F25 L12 第五型针：元素表成员不许重新打字契约已登记的取值（2026-09-29）

**起因**＝F24 结案时点名的残留：`Set([...])`／数组元素表这种"值写在调用左侧"的形态仍不在四型针上（§5.53 残留 ②）。本轮先只读普查再决定纳不纳。

**反事实现量**（`D:/tmp/f25-e-shape-census.mjs`，未落库＝没牙）：与 L12 同扫描集（30 个文件）下，元素表里的契约取值命中 **22 处 / 7 张表 / 3 个文件**；全 src 同型 29 处 / 8 文件 ⇒ 集外 7 处属别的域（`PhasePipeline`、`MessageBubble`＋其 stories、`nav.config`、`AdminUser`）。普查尺子与判据尺子**不同口径**被本轮抓到一次：普查从 `contract/{enums,registry,sessions}.js` 里按"所有 `Object.freeze` 表"取禁串，比判据实际导入的十张表**多算了几档**（`weighted/voting/debate/best_of` 四档因此被误报成命中，针落地后 E 只咬到 6 处）⇒ 反事实的期望值域必须与判据同源列举，否则会把合法写法当债定价。

**针落地顺序**（守"先加针看它红 → 再改判据 → 最后把旧形状做成变异体"）：把第五型 `E 元素表成员` 加进 `NEEDLES` 后单文件判据当场红，打印的真实账是 **E=6**（不是普查的 22，因为针按行扫而普查按表扫，且值域现推口径不同）。6 处逐条走到底：

| 站点 | 判定 | 处置 |
|---|---|---|
| `composables/useAllianceTasks.js:4/6/7/8` 三张动作表（13 处） | 同域债：`:75/:95` 拿 `task.status` 做 `.has(...)` | 成员改引 `TASK_STATUS.*`；**`'ready'` 保住不动**（下段说明） |
| `views/expert/AllianceTaskView.vue:370` `DAG_LEGEND`（4 处） | 同域债：节点档子集，`:136` 拿它当 class 与 `nodeStatusLabel` 入参 | 改引 `NODE_STATUS.*`，成员与顺序一字未动 |
| `modules/expert-alliance/views/AllianceConsoleView.vue:595` `canCancel`（2 处） | 同域债：同函数上两行已经写 `TASK_STATUS.RUNNING/PAUSED`，只有这一行是内联表 | 改引 `TASK_STATUS.*` |
| `modules/expert-alliance/views/AllianceOrchestrationView.vue:182` 筛选下拉（2 处） | 同域债＋**顺带一个显示缺陷**：`:label="s"` 把 wire 串直接印成中文界面的选项文案 | 档名交 `ORCH_STATUS.*`，文案交 `orchStatusLabel(s)`（＝本轮唯一一处用户可见文字变化：`completed` → `已完成`） |
| `components/ExpertRegistryForm.vue:104` 的 `'text'` | (c) 跨域同名：过滤的是表单字段 `f.kind`（text/textarea/select/number），与 `MSG_TYPES` 只是撞串 | 进 EXEMPT 带 `n`+`why` |
| `views/expert/ExpertConfigView.vue:2632` 的 `'code'` | (c) 跨域同名：内置专家配置夹具的 `trigger.taskTypes` | 进 EXEMPT 带 `n`+`why` |

**`'ready'` 为什么不敢顺手删**：任务出参档名由 `alliance.rs:326-336 task_status_str` 现推，七档里没有 `ready`（`ready` 是**节点** proto 名，`alliance.rs:368-370 NODE_STATUS_NORM` 明确把它归一成 `pending`）；但远程调度器那条路的 `status` 是 `v["status"].as_str()` **直传**（`alliance_remote.rs:465`，只有非串才默认 `pending`）⇒ 产不出 `ready` 这件事我**没有证据**，删掉就是拿行为换整洁。本轮做法＝语义一字不动（成员照旧，含 `NODE_STATUS.READY`），把"任务值域到底含不含 ready"登记为待裁决（与 #41 同批）。这也**收窄了 §5.39 的一句旧判断**：那句"`ready`/`unknown` 两个 wire 上产不出的死档"（:1427）讲的是当时被撤的那本文案字典，我当时把结论外推到了整个任务域；本轮复核出参通道后发现外推不成立——`unknown` 只是 `allianceTaskModel.api.js:23` 的本地兜底，而 `ready` 在远程路径上无出参证据。

**常驻账改动**（用例仍 **19 例**＝覆盖面变严而账面不涨）：`NEEDLES` 4→**5** 型并配 `expect(NEEDLES.length).toBe(5)`（针形集塌缩＝覆盖面悄悄缩回去）；形状计数打印加 `E=`；EXEMPT 由 4 键增至 **6 键**（两处 (c) 档）；逐站点零命中格由 2 条增至 **8 条**（新增本轮四处已收口站点的 `rel|value` 键——被重新打字就必须红）。现量：`[L12 形状] 扫描 30 个文件（.vue＋在册 .js）／五型命中 A=9 B=0 C=0 D=0 E=2`（A=9 与 §5.51/5.52 同值＝加宽没动到旧账，E=2 全为已登记豁免）。

**电池 8 枚／不符 0 枚**：E1 动作表首元改回字面量、E2 图例改回字面量、E3 `canCancel` 改回字面量、E4 编排筛选表改回字面量 ⇒ 四枚都必须红（证明第五型针在**四个不同站点**上各自有牙，而不是只在一处开火）；E5 往动作表塞契约外的 `queued` ⇒ 必须不红（口径边界）；E6 只改中文注释 ⇒ 必须不红；另含未变异基线与还原后复验。四个被碰文件跑前后 md5 逐一 MATCH，判据文件 md5 全程未变。

**仪器自捉（本轮三枚，都属"格式正确而语义坏了"）**：① 给判据加 EXEMPT 条目时把对象字面量的收尾 `}` 写成了 `]` ⇒ vitest 报的是 `Failed to parse source ... 1:1`（指针落在文件首行，与错处无关），且这副样子下**整套 19 例直接消失**（`Tests no tests`）而 rc 仍非零可辨——教训＝判据文件改完必须先确认"例数还在"，只看 rc 会把"整文件没解析"读成"某一例红"；② 电池还原从"循环末尾写回"改成 `finally` 写回（`subprocess` 超时/崩溃时变异体不许留在盘上——这四个源文件里三份是 `??` 无 git 恢复源）；③ **§5.53 那条"failed/passed 两个正则各自现读"的修法只修了一半**：电池第 1 版仍把 8 枚全读成 `no-verdict-line`（`实读=None`），因为 vitest 的判决行里 `Tests` 与数字之间隔着 ANSI 序列（`Tests ESC[22m ESC[1mESC[32m1 failed`），`\s+` 接不上数字——**rc 那一路其实全对**（基线 0／四枚变异 1／三枚正对照 0），坏的是解析器不是闸门。修法＝解析判决前先 `re.sub(r'\x1b\[[0-9;]*m', '', out)` 剥掉呈现层，并且**干测要覆盖三种日志**：真红、真绿、以及"整文件没解析"（这一种必须仍是 `None`，不能被读成绿）。同一族再次复发：**只要判决要过呈现层，就得先归一化再匹配**（计数未现量，故不写第几次）。

**残留（下一族，别当已收口）**：① "任务值域含不含 `ready`"待裁决（决定三张动作表能不能删一枚成员）；② `FUSION_CHOICES`（`AllianceTaskView.vue:371` 四档策略名）当前**不在禁串集合内**（那三张表判据没导），若要把 `fusion_strategy` 收成契约表则涉及**请求体取值**（同 `skillOptions` 那一族的警告），未动；③ 集外 7 处属别的域，要动是各自开档；④ `.ts` 除名单外未纳。**一律未入库**（未执行 `add`/`commit`/`stash`，git 索引由并行作者持有）。

**读数与状态**：原始读数归档见 `reports/data/f25-element-table-readings-20260929.txt`；单文件判据 rc=0（19 例，形状行 A=9 E=2）；全量 vitest rc=0（77 文件／1035 例，与 §5.53 轮实测同值＝本轮没加用例）；vite build rc=0（4373 modules transformed／28.02 s）；门禁 check-frontend-module rc=0、check-locale-format-outlets rc=0、check-doc-links rc=1 属存量（扫描 364 文件／判定引用 2040 条／断链 42 处涉 8 文件／WARN 120 行），插入前后各量一次 WARN。电池 8 枚跑完后四个被碰文件的 md5 逐条 MATCH（f8fd4bc145c4／672bede1a0e9／b025304e7a5d／fdca0888724a），判据文件 md5 073c901c0d18 全程未变。门禁第 1 次尝试因 npx 垫片解析失败而 0.3 s 报 rc=1，已改用 node 直执行 node_modules 入口重跑（该假判决原样归档在 [6b]）。

## 5.55 F26 残留现量：`.ts` 从未有语料，覆盖面改由判据自己接管（2026-09-29）

**起因**＝§5.53／§5.54 各留了一句"`.ts` 除名单外未纳"。本轮先量这句话本身：`find src` 下 `.ts`/`.tsx`（去 `.d.ts`、去 `.test.ts`）实测 **0 个**，把口径放宽到全仓（只去 `node_modules`）仍是 **0** ⇒ 那句残留属**未现量的散文**（它凭空造出一批语料）。真正的债不是"没纳 `.ts`"，而是"**名单外语言一出现就没人管**"这件事当时只靠人记得。

**常驻账改动**（`modules/expert-alliance/contract/vocabulary-ownership.test.js` L12，用例仍 **19 例**＝覆盖面变严而账面不涨）：① 新增 `[L12 语言]` 现量行——印目录前缀池尺寸及其扩展名构成、扫描集尺寸与非 `.vue` 成员数，实测 `前缀池 32 个文件（.vue=28／.ts=0）／扫描集 30（非 .vue=2）`；② 把旧格"在册 `.js` 数＝名单长度"换成**扩展名无关**的"非 `.vue` 成员数恰等于名单长度"（写死 `.js` 的那格在别人把名单换成 `.ts` 时会自相矛盾：新语言明明进了集，格却按后缀数它）；③ 两枚新牙＝`JS_IN_SCOPE.length` 恰等于 2（撤一条即红＝覆盖面静默缩回，要撤必须先改判据口径留痕）＋ 前缀池里出现**名单外 `.ts`** 就必须红（`toEqual([])`，并把实测文件名列表打进失败消息）；④ 顺带更正判据注释里 §5.53 留下的同一句话（那次 `.js/.ts` 普查的命中实测全为 `.js`）。

**断言顺序本轮动过，理由要写清**：vitest 一遇首条失败即止。第 1 版把新格排在"扫描集尺寸"格之后，于是两枚本该点名新格的变异体都被尺寸格顶掉——**打印出来的红格是别人的名字**。现改为仪器自检在前（名单自身 → 成员构成 → 名单外语言 → 扫描集尺寸），每格才各有一枚"它是第一个说谎的"变异体。这不是把判据放松：格一个没少，只是先说哪句谎。

**电池 6 枚／不符 0 枚**：N1 撤名单一项 ⇒ 红在"按名名单塌缩"；N2 把名单项写成一枚 `.vue`（旁路被白拿，非 `.vue` 成员少一个）⇒ 红在"非 .vue 成员实测"；N3 在 `modules/expert-alliance/views/` 造一枚**不含任何禁串**的临时 `.ts` ⇒ 红在"出现名单外 .ts"（这一枚证明的是**覆盖**而不是命中：文件里没有债，格照样拦）；N4 只加一行中文注释 ⇒ 必须不红；另含未变异基线与还原复验。本轮变异对象是判据自己的名单＋一枚临时文件，所以"判据 md5 全程未变"这类见证不适用，取而代之的是**还原后回到跑前哈希**（`c8feb0adf6e0` MATCH；临时 `.ts` 由 `finally` 删除，终检 `probe exists=False`，事后独立复量＝`c8feb0adf6e0`／37291 B，且该目录 `??` 计数 0＝夹具没留在盘上）。

**仪器自捉三枚**：① 第 1 版电池的记账行把 `split(...)[-6:]`（list）直接拼进 `str` ⇒ `TypeError`，崩在第一枚之后；因为还原写在 `finally`，磁盘当场回到写前图像——这条机制在这一轮第一次真的替我挡下"把变异体留在盘上"。② 由此提炼的规矩：**新增一格必须同时带来一枚"该格是第一个失败者"的变异体**，否则只证明这格写得出，不证明它会开火。③ 本轮新踩并当场翻案的 shell 引号坑（恰在写"读数"这句时暴露）：在 Bash 的双引号里用 `python -c` 填正文，**串内的反引号被 shell 当命令替换执行**，于是落档句子当场丢了文件名与引用段（印成"原始读数归档见 （17405 B…"、"未执行 //"）——症状＝句子还在、尺寸还在，而**挂数字的那个来源不见了**，正是引用纪律该抓的那一类。修法＝正文一律由写文件工具落盘，命令行只跑脚本；本节是先还原到插前图像后重插的第二次。

**读数与状态**：原始读数归档 `reports/data/f26-language-coverage-readings-20260929.txt`，本节所有数字挂在它的分块上，块号在句末逐处点名（`[2]` 单文件判据／`[6]` 六道门禁／`[7]`·`[8]` 全量 vitest 与 build／`[10]` 本节修复链／`[11]` 修复后终态）。**归档自身的尺寸与哈希不在本节引用**：同一份归档在本节落盘后又被追加，第一次追加就把当时正文里那句尺寸引用刷成了过期值（老规矩第二次撞到——追加会移动所有引用该文档尺寸的既有条目，差别是这次由写后复量抓到而非由读者抓到）；过期值原文登记在 `[11]` 的 STALE 行，别再当现值读，末次实测值只在 `[11]` 里。单文件判据 rc=0（19 例，形状行 `A=9 B=0 C=0 D=0 E=2`、语言行 `前缀池 32（.vue=28／.ts=0）／扫描集 30（非 .vue=2）`，见 `[2]`）；全量 vitest rc=0（77 文件／1035 例＝与 §5.53／§5.54 同值，本轮没加用例）、vite build rc=0（4373 modules／37.91 s），见 `[7]`／`[8]`；check-frontend-module rc=0、check-locale-format-outlets rc=0、check-doc-links rc=1 属存量（WARN 120 行，插前后各量一次并逐行 diff，重插后复跑仍 WARN 120＝本次插入未引入新悬空引用），见 `[6]`。**一律未入库**（未执行 `add`／`commit`／`stash`，git 索引由并行作者持有）。

## 5.56 F27 覆盖面第四维（目录）：值域扫描集不扩，但"哪个目录没出账"必须红（2026-09-29）

**起因**＝F21／F23／F24／F25／F26 把 L12 的覆盖面沿三条维度逐一收紧（文件数→针形→语言）之后，剩下的洞是**目录维**：模块内实测 7 个目录（`_verification／api／components／contract／model／store／views`，文件数分别 2／3／19／21／12／11／6，另有根层散文件 4 个），而值域扫描集只取 `views`＋`components` 两个前缀加按名在册的两份 `.js`——其余五个目录在池外，且**没有任何账说明它们为什么在池外**（清点见归档 `[2]`）。"谁在池外"这件事此前同样只靠人记得。

**先同口径复算，再谈扩集**（老规矩，见 §5.55）：反事实普查先在与判据同口径的在册集上复现现场读数（30 个文件 → `A=9 B=0 C=0 D=0 E=2`，与 `[3]` 的活读行逐字相符），再算集外——34 个引用联盟契约的集外文件 → `A=13 B=0 C=0 D=0 E=3`，16 处命中集中在 4 个文件（`stores/ai.store.js` 8／`composables/workspace/useTaskOrchestration.js` 5／`views/workspace/panels/CollaborationPanel.vue` 2／`stores/alliance.store.js` 1），逐处见 `[4]`。逐处读原文的结论＝**0 处同域 wire 债**：`alliance.store.js:140` 的 `runState` 是本地 pinia 状态机（`ref('idle')` 只在本地赋值，从不来自 wire）；`useTaskOrchestration.js:151-180` 的 `task.status` 是本地编排演示态（同段有 `Math.random() > 0.1` 决定失败）；`ai.store.js` 的 `'user'/'assistant'` 与 `CollaborationPanel` 的 `'failed'` 是聊天消息域自己的词汇。⇒ **裁决＝值域扫描集不扩**，与 §5.46（F15 `MODE_WIRE`）同型：扩集只把跨域同串拉进台账，不带来新的真值域约束，而台账一膨胀针就会被磨钝。

**但覆盖面确实有债，且形状是目录**：任何新进模块的子目录（不论 `.js`／`.ts`／`.vue`）都会静默落在池外，而"池外"这件事本身不可见。本轮把它变成常驻账（L12，用例仍 **19 例**＝覆盖面变严而账面不涨，`[3]`）：`POOLED_DIRS`（在册两个）＋`DIR_ACCOUNT`（五个出账条目，各带一句中文理由）＋四格——① 出账表长度恰等于 5（撤条目即红＝覆盖面静默缩回，要撤必须先改口径留痕）；② 理由为空或短于 8 字即红（没理由的出账不存在，同 `EXEMPT` 的 why 规矩）；③ 条目名指向已不存在的目录即红（该删就删，别留空壳理由）；④ 模块内出现**既不在册也未出账**的目录即红。现量行 `[L12 目录] 模块内目录 7 个（在册 2／出账 5／未登记 0）`。目录四格排在"成员构成"之后、"扫描集尺寸"之前，沿用的正是 §5.55 那条顺序规矩：让每格都能当"第一个说谎的"。

**电池 6 例／不符 0 枚**（`[5]`）：未变异基线绿；G1 造一枚临时 `modules/expert-alliance/wizard/Wiz.vue`（**不含任何禁串**）⇒ 红在"未登记的目录"（这一枚证明的是覆盖而不是命中：文件里没有债，格照样拦）；G2 删掉 `api` 那行出账 ⇒ 红在"出账目录表塌缩"；G3 把 `_verification` 的理由写成空串 ⇒ 红在"出账理由为空"；G4 把 `api:` 改成 `apix:`（条目数仍 5，只有名字对不上真实目录）⇒ 红在"已不存在的目录"；G5 只加一行中文注释 ⇒ 必须不红。探针目录的生命周期收在 G1 那一格内、跑完立刻删——草稿原本留到循环结束，那样 G5 会被"未登记的目录"顶成红（仪器自己造出的假阳），当场改掉，未进判决。还原见证：`restored guard 2d2f6e093a02 vs before … MATCH`、`size 40433 -> 40433`、`probe exists=False`，事后独立复量同值。

**仪器自捉三枚**：① 电池的还原复验行两个标签写反了（把 `实读` 位取成 fired 首项、`红格` 位取成 verdict），于是绿的那一次打印成 `实读=- 红格=green`；数值（rc=0／无红格）没受影响，但**读数行自己不可读**，故原样登记在 `[5]` 的瑕疵行里而不是事后涂改。② 归档首版把 vitest 输出的 ANSI 转义序列原样抄进 `[3]`／`[6]`（36 处、172 字节），症状＝数字都在而串被转义夹断；修法＝抄录前先剥色，修复链与前后哈希见 `[8]`。③ **出账的理由会被更宽的口径证伪**：`store` 那句理由原本写"本轮五型实测 0 处命中"，而块 `[4]` 的普查只扫"引用联盟契约"的文件；`[9]` 换成逐文件复扫（不要求引用契约，因为相对 import `'./contract'` 根本不被那条正则命中）后实测 **2 处命中**，都在 `store/alliance-experts.store.test.js`（`:33` 的 `online: status === 'online'`、`:37` 夹具默认值 `booking(id, status = 'pending')`）——测试自己造数据，不是界面出口，出账仍成立，但理由不能挂在一个会被口径换掉的数上。⇒ 按实测改写理由（`[10]`），改动只动一个 `why` 字符串（条目数仍 5，四枚变异体的锚点全在，未重跑电池而是以复跑见证：rc=0／19 例／`[L12 目录]` 行原样）。

**读数与状态**：原始读数归档 `reports/data/f27-directory-coverage-readings-20260929.txt`，本节数字挂在它的分块上并按块号点名（`[2]` 目录清点／`[3]` 判据单文件活读／`[4]` 同口径复算与反事实扩集／`[5]` 电池／`[6]` 六道门禁／`[7]` 本节写前文档图像／`[9]` 出账目录逐文件复扫／`[10]` 改 `why` 后的复跑）。判据本轮终态图像 40585 B／`09ec807e06f7`（电池所跑的图像 40433 B／`2d2f6e093a02`，差在那句 `why`；上一轮 F26 图像 37291 B／`c8feb0adf6e0` 挂在同目录 f26 归档 `[5]`，见 `[1]`）。单文件判据 rc=0（19 例）；全量 vitest rc=0（**77 文件／1035 例**＝与 §5.55 同值，本轮 0 新用例）、vite build rc=0（37.35 s）、check-frontend-module rc=0（ERROR=0）、check-locale-format-outlets rc=0（扫描 270／登记 2），均见 `[6]`；check-doc-links 写本节**之前** WARN=120／ERROR=0／rc=1 属存量（`[6]`、`[7]`），写后复跑与逐行 diff 由本节落盘之后的追加块承载（点名载体而不写进本节数字——老规矩：登记行不许引用自己之后的闸门读数）。普查与复扫两把驱动（`D:/tmp/f27-e-shape-census.mjs`／`D:/tmp/f27-outdir-scan.mjs`＋ESM 解析钩子）都**未落库＝没牙，不接 CI**。**一律未入库**（未执行 `add`／`commit`／`stash`，git 索引由并行作者持有）。

## 5.57 F28 调色板入参形态：把「查调色板时重新打字领域名」收进单一键出口（2026-09-29）

**起因**＝`expertColor / expertEmoji / expertGradient` 这三个查色函数有两种入参形态在库里并存：一种是 `expertVisualKey(x)`（模块算好的领域键），一种是直接把成员访问递进去（`x.type`／`getExpertById(id)?.type`）。后者不是风格问题——本轮先量真值链（归档块 `[2]` 逐条点名印出该行的文件）：`model/normalize.js:248` 把服务端的 `expert_type ?? type` 一律折成 `expertType` 键，归一化之后的专家对象上**没有 `type` 这个键**；而 `api/alliance.api.js:144` 的 `listExperts` 走的正是 `normExpertList(payload)`，`views/workspace/panels/TaskOrchestrationPanel.vue:510` 与 `views/workspace/ExpertWorkspaceView.vue:383/405` 的专家池都来自它。⇒ 界面递进去的 `exp.type` 在真实数据上恒为 `undefined`，三函数落到 `constants/expert.constants.js:53-55` 的三个兜底常量（`#6366f1`／那条渐变／`👤`）。也就是说**这些位置画的不是"某个专家的颜色"，是"没查到"**。同一条链的另一头：`constants/expert.constants.js` 的内置行确实只带 `type: 'algorithm'` 这类**领域名**（键名即调色板表的键，与后端 `expert_type` 的 human/ai/hybrid 不是同一词表），所以不能简单把 `.type` 删掉——那一档是真在用的。**而这句"真在用"本轮被自己的重量打了个折扣**：同文件重量（块 `[14]` 现量）**五表各 16 键、键集两两相等**——`EXPERT_TYPES`／`EXPERT_TYPE_SHORT_LABELS`／`EXPERT_COLORS`／`EXPERT_EMOJIS`／`EXPERT_GRADIENTS` 的 missing／extra 皆为空 ⇒ **不存在调色板覆盖缺口**。本节早先在此写过的"各只有 6 键、差集 10、对 16 档里的 10 档按定义落兜底"**是一次仪器假账**：那把尺子的针取 `行首＋标识符＋冒号`，而这几张表一行并三到四个键，于是数出来的是**行数**不是键数（6 恰是 `EXPERT_COLORS` 的键行行数 :58-63）。换用"标识符紧跟冒号紧跟字符串"重量的真值见块 `[14]`。缺陷没有消失，只是换了一副面孔＝**身份并列**：`EXPERT_COLORS` 的 16 键只落 **14 个不同色值**（`algorithm`≡`architecture` 同为 `#6366f1`，而该串正是 `EXPERT_COLOR_FALLBACK`；`operator`≡`custom` 同为 `#64748b`），`EXPERT_EMOJIS` 的 16 键落 **15 个不同值**（`ai`≡`automation` 同 `🤖`；`custom` 的 `👤` 与 `EXPERT_EMOJI_FALLBACK` **同形**）⇒ 在这些调色板上"查到了"与"没查到"、以及两个不同领域之间，界面上不可分辨（记在本节余账 R4，改写后不再叫"覆盖问题"）。另注：`display.js:41-45` 与既有记忆那句"16 色板"是被本次现量推翻的旧散文，本轮不覆写它们、只在此点名（本节早先写的"16 色板"同属未现量，按此读）。

**改动**（四份文件，全部整文件二进制读写，跑前图像存 `D:/tmp/f28-backup`、尺寸与哈希现量于归档块 `[1]`）：① `model/display.js` 的键推导补第三档兜底 `|| str(e.type)`，并把注释改成点名两档来源（`expertType` 来自归一化、`type` 只覆盖前端内置表）；这一档是**行为保持**的——内置行原本就靠 `type` 命中，归一化行原本落兜底、现在落自己的领域键。② 14 处调用站点改走单一出口（`composables/workspace/useTaskOrchestration.js:168` 1 处、`TaskOrchestrationPanel.vue` 9 处、`ExpertOverviewPanel.vue` 4 处）。同口径的针复算两副图像给出这台手术的分子分母（归档块 `[3]`）：跑前 4 个文件 19 处非白名单命中 ⇒ 跑后 2 个文件 5 处，剩的 5 处逐条走到底都是**读键名**不是重新打字（`task.suggestedExpertType` 3 处：`useTaskOrchestration.js:127` 就拿 `expertVisualKey(e)` 与它相等比较，同一词表；`item.visualKey` 2 处：`model/collabLists.js:65` 已把 `expertVisualKey(c)` 算进出参）。

**常驻格 L14**（仍落在 `contract/vocabulary-ownership.test.js` 的 L12 那个 `it` 里，用例数**仍 19 例**＝覆盖面加宽而账面不涨）：按**形状**钉而不是按名字钉——针 `\b<fn>\s*(?:\?\.)?\(([^)]*)\)` 扫 `src` 全部 `.js/.ts/.vue`（排 `.test.js` 与调色板定义文件本身），实参落白名单（`expertVisualKey(`/`rowVisualKey(`/`visualKey(` 起头，或纯裸标识符）之外即为命中；四格＝扫描集分母（实测 272，地板按实测写 ≥100 而不是凭手感钉）、命中文件集⇄`PALLETTE_LEDGER` 键集**双向 toEqual**、逐文件命中数＝台账 `n`、逐条 `why` 长度 ≥8。**第五格本轮删掉了并留了注释**：它查"命中的文件不在台账里"，与第四格是同一条判据而排在它后面 ⇒ 没有任何变异体能让它成为第一个说谎的，没有自己变异体的格是装饰。

**电池 6 枚／不符 0 枚**（源＝归档块 `[5]`，逐枚点名"该枚红在哪一格"）：baseline rc=0；H1 把 `ExpertOverviewPanel.vue` 两处退回 `exp.type` ⇒ 红在「出现未登记的调色板调用点」；H2 台账把 `n: 3` 写成 `n: 2` ⇒ 红在「命中数与台账登记数不符」；H3 `why` 改空串 ⇒ 红在「台账缺理由」；H4 给 `paletteSrc` 加一条永不成立的过滤 ⇒ 红在「调色板扫描集塌缩」（这一枚证明的是**分母**而不是命中：判集空时其余三格全部恒真）；H5 只改判据里的中文注释 ⇒ 必须不红，实读 green。还原写在 `finally` 并与**跑前图像**逐字节比：guard 43924 B／`6e9121b326c4` MATCH、ExpertOverviewPanel 27293 B／`8d6150f19e27` MATCH，CR 前后皆 0，还原后复跑 rc=0／红格数 0。

**仪器自抓两枚，原样登记**：① 落地驱动自己打印的 before→after 尺寸是**字符数配字节数**（`len(text)` 对 `len(bytes)`），display.js 那行看着像 +1129 B 而手术只有 +206 B——本轮补了独立复算（归档块 `[1]` 逐文件同尺给出 3084→3290 B），这是老坑（尺寸必须点名单位）第 N 次复发，复发点从文档移到了驱动的 stdout；② 归档第 [1] 块里"上一轮归档的尺寸"那行被 `' '.join(一个字符串)` 逐字符插成了空格（`1 4 6 1 9 B ／ m d 5 …`），数字未坏而排版坏，修法记在归档块 `[8]`（不覆写已写出的读数，追加修复块）；③ 引用纪律本轮又撞两次，都在我自己写的读数行上：第一次比 doc-links 的插前／插后 WARN 集合差时得出 **new=120／gone=120**（120 条"全变新面孔"），归因发现插前那份是 bash 重定向产物、带 **CRLF=180**，插后那份由 `io.open(newline=LF)` 写出——两把尺子的行尾不同源，逐行集合差把每条都判成新人（归一吃掉游离 `\r` 后重比＝**new=0／gone=0**，即本次插入确实未引入新悬空引用，见归档块 `[10]`）；第二次是修正块自己：`[10]` 描述 `[9]` 时那两个占位取了手里刚算出的 0/0，而 `[9]` 当时印的是 120/120，于是"解释引用纪律的那行"成了坏引用——`[11]` 的修法＝**从被引文件把原句正则读回**，不用手里的新值回填旧句。另有一条**覆盖面边界要写清**：针的捕获组 `[^)]*` 会停在第一个右括号，所以嵌套调用的"实参"读到的只是前缀（`getExpertById(expId)` 被截成 `getExpertById(expId`）；它落在被拒侧是因为白名单的第一支是**前缀匹配**，不是因为捕获读全了——将来若有人把白名单改成全串匹配，这批嵌套形态会整批翻成假阳，故本轮把这一点写在这里而不是写进断言。

**读数与状态**：原始读数归档 `reports/data/f28-palette-argument-shape-readings-20260929.txt`（结论-free，本节所有数字挂在它的块号上：`[1]` 图像／`[2]` 真值链／`[3]` 两副图像对照普查／`[4]` 判据活读／`[5]` 电池／`[6]` 门禁／`[7]` 文档写前图像／`[8]` 修复块；按老规矩**不引用归档自身的尺寸与哈希**（追加会移动所有引用它的既有条目，末次实测值只印在归档最后一个追加块里，本轮写到此为止是 `[11]`）。判据单文件 rc=0（19 例，`[L14 调色板]` 行印 272 文件／5 处／2 个文件，见 `[4]`）；六道门禁 rc 取自退出码而非管道尾巴（`[6]`）：全量 vitest rc=0（77 文件／1035 例＝与 §5.55／§5.56 同值，本轮没加用例）、vite build rc=0（✓ built in 36.22 s；改导入形态的消费者账只由 rollup 验，本轮两份视图的 `expertVisualKey` import 实测早已存在，打印见 `[1]` 上方驱动输出）、check-frontend-module rc=0（ERROR=0）、check-locale-format-outlets rc=0（扫描 270 文件／余账 2 处＝登记表）、check-doc-links rc=1 属存量（WARN 120，插前复量仍 120＝本次插入未引入新悬空引用）；本节措辞更正（把未现量的"16 色板"改成现量口径）之后第三次复量＝WARN 仍 120 条、对上一份日志集差 new=0／gone=0、rc=1 仍属存量（两份日志的字节指纹见归档块 `[13]`）。**余账四笔，一律只报不改**（R4 排在最前因为它现在是量最大的一笔）：**R4＝调色板的身份并列（本条由"覆盖只有 6/16 档"改写而来，旧值系仪器假账，见本节起段与块 `[14]`）**——五表键集现量两两相等（各 16 键），但 `EXPERT_COLORS` 16 键只有 14 个不同色值（`algorithm`≡`architecture`＝`#6366f1`＝`EXPERT_COLOR_FALLBACK`、`operator`≡`custom`＝`#64748b`），`EXPERT_EMOJIS` 16 键 15 个不同值（`ai`≡`automation`＝`🤖`，且 `custom`＝`👤`＝`EXPERT_EMOJI_FALLBACK`），`EXPERT_GRADIENTS` 16 键 15 个不同值 ⇒ 三处并列里"两个领域显示成同一个身份"与"这个档根本没登记"在界面上同形；修法要么给并列档另配色/配 emoji（改外观，要裁决），要么把兜底改成不与任何登记档同形的形态（同样改外观），故本轮**一律不动**。**R1（随 R4 一并改写：旧结论的前提没了）** `useTaskOrchestration.js:59` 新建子任务把 `suggestedExpertType` 默认成 `'custom'`——`custom` **在**五张表里都有档，所以它不会落兜底；它的问题是落在 R4 的同形并列上：颜色与 `operator`（算子）相同、emoji 与"没查到"相同 ⇒ 新建任务的头像在视觉上等价于"一个算子专家"或"查不到"，两可。真要修仍是产品裁决（这一档该显示成什么），但**理由已从"缺档"改成"并列"**；R2 白名单放过的裸标识符形态实测 8 处（`RegisterExpertDialog.vue:314/316`、`ExpertPlazaView.vue:631/890/892`、`ExpertEnterprisePanel.vue:854`、`ExpertWorkspaceView.vue:570`×2），本轮已逐处走到底：2 处证为正确（`ExpertWorkspaceView.vue:570` 的 `key` 由同文件 :569 的 `expertVisualKey(speaker)` 算出），其余 5 处（`ExpertPlazaView.vue:631/890/892`＋`RegisterExpertDialog.vue:314/316` 的 `typeColor/typeEmoji/getEmojiByType/getGradientByType` 包装）取的是 `EXPERT_TYPES` 的键域 ⇒ 归到 R4；第 8 处 `ExpertEnterprisePanel.vue:854` 本轮**走到底**（复核见块 `[13]`）：`:853` 的 `getTypeColor(type)` 是本文件内的包装器（`return expertColor(type)`），模板上四个调用点各有来源——`:190` 的 `type` 是 `v-for="(label, type) in typeLabels"` 的键，而 `:582` 的 `typeLabels = EXPERT_TYPE_SHORT_LABELS`（16 键，与三张调色板同键集 ⇒ 图例每档都查得到，本条早先写的"10 个按定义落兜底"随 R4 一并作废）⇒ 16 个图例点只用上 **14 种颜色**（`algorithm`≡`architecture`、`operator`≡`custom`），这是 R4 改写后那笔**身份并列**的可见后果（不是入参形态债）；`:197`／`:212` 读 `node.type`／`selectedExpert.type`，而 `:868` 把图谱节点整体赋给 `selectedExpert`，两者同源于 `api.getExpertGraph()` 出参，节点 `type` 由服务端常量戳成 `"expert"`（§5.40 F10 现量）⇒ 调色板按定义查不到，同屏 `:217` 的 `typeLabels[selectedExpert.type] || selectedExpert.type` 还会把英文 `expert` 直印出来（F10 家族余账）；`:408` 的 `s.expert.type` 取自 `enterpriseResult.optimal_team.scores`＝dispatch 出参、未过 `normExpert`，键名与词表都属服务侧契约，修法与 R1/R4 落在同一次裁决里。⇒ **R2 八处至此全部归因完毕：2 处正确／5 处归 R4／1 处＝图谱节点常量戳＋dispatch 出参，归 F10 与 R4，无一处是"入参形态"可独立收口的**。附带一条**新的覆盖形态**（F29 候选，本轮不扩针）：L14 的调用点扫描只认 `expertColor/expertEmoji/expertGradient` 三个函数名，而 R2 里 5 处走的是**本地包装器**（`typeColor`／`typeEmoji`／`getEmojiByType`／`getGradientByType`／`getTypeColor`）⇒ 包装器把转发藏在账外，"收进单一出口"在这类文件上只收到了出口本身；扩集前要先量"库里有多少个这样的转发函数、各自实参形态如何"，不许直接把名加进 `PALLETTE_FNS`。同一次复核顺带确认：`ExpertEnterprisePanel.vue:847` 自写的 `formatTime(ts)` 已在 F4 的落库台账 `src/utils/f4-hand-assembly.test.js:64` 以 1 处登记（档不同不并，理由见 §5.35），**不是缺口**。R3 `ExpertOverviewPanel.vue` 的专家池生产者是否也走 `normExpert` 只读到调用侧（第 327 行未读），若它拿的是原始 wire 则 `.type` 本来有值、本处的四处改写对它是无害冗余。**一律未入库**（未执行 `add`／`commit`／`stash`，git 索引由并行作者持有）。

## 5.58 F29 调色板转发口在册账：L14 的针只认三个函数名，包装器把形态债藏在账外（2026-09-29）

**起因**＝§5.57 的收口审计自己带出来的那条：R2 八处里有 5 处走的是**本地转发口**（`function typeColor(type) { return expertColor(type) }` 这类单表达式包装）。L14 的 `PALLETTE_FNS` 只点名三个函数，转发口在账外 ⇒ "收进单一出口"在这类文件里只收到了出口本身，判据恒绿而债照样在。本节把这一维补成在册账。

**普查**（只读，块 `[1]`）：扫描集 273 个文件（`.vue`／`.js`／`.ts`，不含 `.test.js`；判据内口径 `paletteSrc` 为 272，差的那 1 个正是被排除的 `constants/expert.constants.js`）／转发口 **5 个**／其调用点 **12 处**／其中实参非白名单形态 **10 处**。**逐口点名与归因**（块 `[2]`）：`getEmojiByType` 2 处与 `getGradientByType` 1 处（`ExpertPlazaView.vue:890/892` 定义，读内置预设行的 `type` 键＝领域名，构造性正确）；`getTypeColor` 3 处（`ExpertEnterprisePanel.vue:853` 定义，实参是图谱节点 `node.type`＝服务端常量戳 `"expert"`（§5.40）、同一节点赋值的 `selectedExpert.type`、以及 dispatch 出参 `s.expert.type`）；`typeColor`／`typeEmoji` 各 2 处（`RegisterExpertDialog.vue:314/316` 定义，读 `formData.type`＝该表单自己的领域名下拉值）。⇒ **10 处里没有一处属"本该走 `expertVisualKey` 却没走"**；唯一需要动服务侧的那条（节点 `type` 常量戳）归 §5.40 与 §5.57 的 R4 裁决，不在本节权限内。

**落地下周**＝常驻判据 `src/modules/expert-alliance/contract/vocabulary-ownership.test.js` 新增 **L14b** 格（43924 B／`6e9121b326c4` → **48680 B／`811e2488fbdc`**，＋4756 B，CR=0）。它钉五件事：转发口**集合**⇄`WRAPPER_LEDGER` 双向 `toEqual`（新增要写条目、收口要删条目）、每个转发口的**所在文件**与**转发目标**、其**非白名单调用点数 `n`**、以及台账理由长度 ≥8 字；再加一枚分母 `paletteSrc.length ≥ 270`（实测 272）防"判集塌缩⇒台账恒真"。**用例数不变（19 例）**——格插在既有 `it` 内部，覆盖面加一维而台账不涨，这正是 F24–F28 一路的口径。

**电池**（块 `[3]`）：**4 枚，0 不符**，且每枚都以"L14b 是第一个说谎的"为验收。M1 在 `RegisterExpertDialog.vue` 里新增一枚未登记的转发口 → `rc=1`、首红格 `['L14b']`（实测集多出一个 `f29ProbeWrapper`）；M2 台账里改名一条（盘上转发口还在）→ `rc=1`、首红 `L14b`；M3 **只改台账的中文理由** → `rc=0`（本格不钉文案，与 L6b/L12 同规矩）；M4 台账 `n` 由 2 改成 3 → `rc=1`、首红 `L14b`，判决串 `实测=2: expected 2 to be 3`（证明"次数"那格真有牙，不是只钉名字）。跑完 `finally` 还原，两份被碰文件的 md5 与写前图**逐位相同**（`6e9121b326c4`／`244ba11a8bf3`）。

**门禁**（块 `[4]`）：单文件判据 `rc=0`（19 例；L14b 现场印出 `扫描集 272 个文件／转发口 5 个／非白名单调用点合计 10 处`，逐口分布 `{"typeEmoji":2,"typeColor":2,"getEmojiByType":2,"getGradientByType":1,"getTypeColor":3}`）；全量 `vitest run` `rc=0`＝**77 文件／1035 例**（本轮 `19→19`，全量分母 1035 是现量，旧账里的 1034 属更早的快照，本轮未增用例）；`vite build` `rc=0`（`✓ built in 31.49s`）。

**余账（一律只报不改）**：**W1** 转发口本体仍是"任意串都能进调色板"的入口——本格只**登记**它的存在与调用形态，没有撤销任何转发口，要收口得逐口裁决（撤销＝删函数，会动界面文件的结构）；**W2** `getTypeColor` 那 3 处的真修法在服务侧（节点 `type` 别再常量戳 `"expert"`），前端单改会把"查不到"改成"查得到但仍是同一个戳"，无收益；**W3** 普查的转发口定义只认"单表达式＋实参恰为形参"这一种形状，若将来出现 `return expertColor(norm(type))` 这类**带加工的转发**，本针认不出＝覆盖面边界，届时须按现量另开一格，不许放宽正则把它塞进同一本账。**W3 本轮已定价**（归档块 `[7]`）：换简单口径重扫（扫描集 272 个文件＝与判据同集），落在函数／箭头体内的调色板调用＝**纯转发 8 处／带加工的转发 0 处**；这台仪器自带正对照——L14b 在册的 5 枚转发口被它重新数到（5/5 FOUND），所以上面那个 0 是**量出来的空集**，不是没量。据此边界暂时无站点，但**不撤销边界本身**：真出现 `expertColor(norm(type))` 型转发时按名另开一格登记，不许放宽本针的正则把它混进 L14b。顺带一条新读数：8 处纯转发比在册的 5 枚多 3 处，多的那 3 处是**普通函数体内以裸标识符直接查色**（L14 的白名单形态放过的正是这一类），归 §5.57 的 R2/R4 口径，本轮不重开。

---

## 5.59 F30 模块外"面向联盟"的目录出成常驻账：覆盖面洞从"靠人记得"变成红格（2026-09-29）

**起因**＝§5.56（F27）把 L12 的目录维收成了常驻账，但那本账的判集是**模块内**七个目录；模块外那些"import 指向联盟、却不在值域扫描集里"的目录，此后仍然只靠人记得。本轮复核还顺手翻掉两条过期散文（见"本轮更正"）。

**普查**（驱动 `D:/tmp/f30-dir-census.py`，**未落库＝没牙，不接 CI**；读数与目录清单挂归档块 `[1]`）：src 下 `.vue/.js/.ts`（含测试）现量 **350** 个文件，其中 L12 在册扫描集内 **34**（三个前缀＋两份按名 `.js`）、模块内 **76**；把"池外＋有一条 import 的源指向联盟"作为口径，命中 **7 个目录／17 份非测试文件**：`api`(2)／`composables`(1)／`composables/workspace`(3)／`router`(1)／`stores`(3)／`views/workspace`(1)／`views/workspace/panels`(6)。这 17 份里有没有 wire 取值被重新打字，**本轮不重问**——§5.56 已逐处读过并判为 0 处同域债（本地状态机、聊天域、编排演示态）。本轮只补"谁在池外"这件事本身。

**落地**（判据 `modules/expert-alliance/contract/vocabulary-ownership.test.js` 的 L12 格内，**用例数仍 19 例**＝覆盖面变严而账面不涨）：新增三格＋一行现场读数 `[L12 模块外目录] 面向联盟的池外目录 7 个／非测试文件 17 份 实测=[...]`。① 实测目录数**恰等于 7**（这条同时是判集塌缩的分母：仪器静默看不见文件时它也会红）；② 出账表键集⇄实测集合 `toEqual`（多＝条目该删，少＝有目录没出账）；③ 每条理由不为空且 ≥8 字（无理由的出账＝把覆盖面洞读成合法）。口径沿用 §5.56：**不钉文件数**——往 `stores/` 加一个文件是日常开发，钉数会把针磨钝（假阳的代价是人绕过闸门），要钉的只是"目录名单"这件事。

**电池 4 枚／不符 0 枚**（`[2]`）：未变异基线 rc=0／`19 passed`；H1 造一枚临时 `src/views/telemetry/Tele.vue`（**体内不含任何禁串**，只 import 契约）⇒ 红在①"目录数与出账表不符"（实测 8）——这一枚证明的是覆盖而不是命中；H2 删掉 `router` 那行出账 ⇒ 红在②；H3 把 `router` 的理由掏空 ⇒ 红在③（`router 的出账缺理由`）；H4 只改中文措辞 ⇒ **必须不红**（rc=0，且③仍绿）。还原逐位相同：`restored=True`、跑前跑后同为 51960 B／`f232d4247f93`，探针目录在 H1 那格内 `shutil.rmtree` 掉并现场复量 `exists=False`。

**门禁**：单文件判据 rc=0（19 例）；全量 vitest rc=0（**77 文件／1035 例**＝与 §5.58 同值，本轮 0 新用例）；`vite build` rc=0（**45.20 s**）；`check-frontend-module` rc=0（ERROR=0）；`check-locale-format-outlets` rc=0；`check-doc-links` 写本节**之前** WARN=120／ERROR=0／rc=1 属存量（块 `[3]`），写后复跑与集差由本节落盘之后的追加块承载。**仪器自捉一枚**：第一次 build 我从仓库根用 `--config frontend-ui/vite.config.js --outDir D:/tmp/...` 调用，rc=1 报 rollup `Invalid resolved id`——那是**调用方式**错不是源码错，改用项目默认 `npx vite build` 后 rc=0；原样登记在此，免得下轮把那条 rc=1 读成回归。

**本轮更正（两条过期散文，都在项目记忆里，磁盘已不符）**：① 记忆 F20b 那句"L12 扫描集只含 `modules/expert-alliance/{views,components}`，`src/views/expert/**` 在集外＝覆盖面问题"已过期——§5.51（F21）起 `views/expert/` 就在前缀池里，§5.53（F24）起两份 `.js` 按名在册；② 同条说"唯一真债是 `views/expert/AllianceTaskView.vue:104` 的 `=== 'paused'`"也已过期——磁盘现量 `:104` 是 `selectedTask.status === TASK_STATUS.PAUSED`，且 `contract.test.js:1144` 早已改钉常量写法而不是视图字面量（§5.45 F14c 收口）。两处都只更正记忆，不动代码。

**余账（一律只报不改）**：**X1** 出账表只钉目录名不钉文件数，所以"往已出账的目录里加一个真的 wire 比较"不会红——那一维由五型针＋双向台账负责，两本账各管一段，别把它们混成一个分母；**X2** 七个池外目录现在仍**不进**值域扫描集，这是 §5.56 的裁决（扩集只把跨域同串拉进台账）而非本轮的新发现，要改必须先跑反事实并按名留痕；**X3** 分类器里 `(src-root)` 分支没有语料（现量 0 个根层散文件面向联盟）＝在空集上的写法，若将来出现 src 根层文件就得先确认它真被 walk 收到；**X4** 普查驱动与本轮判据里的 import 口径都只认 `from '...'` 形态，`import('...')` 动态形态此前未现量＝散文。**本轮已定价**（归档块 `[6]`）：池外文件里动态 import 的源含 `alliance` 者**实测 0 处**，而这台仪器是通的——同族正则在全 src（不含 `.test.js`）数到 **68 处** `=> import('...')`／12 个文件，其中联盟自己的 barrel `modules/expert-alliance/index.js` 有 **14 处**，且这些动态挂载的源串里就带 `Alliance`（文件名级，例如 `./views/AllianceCollabView.vue`）⇒ 本普查**按目录**排除模块内文件而不是按名：只按 `alliance` 字样过滤会把模块自己的挂载边算成池外命中，那才是假阳。这一枚是本轮改写 X4 时被驱动自己的断言当场打回来的（我原本写"相对形态、串里不含 alliance"），原样登记在归档 `[6]`。⇒ 裁决＝**不为动态形态另开一格**：池外→联盟这条边在现量语料里根本不存在，开格＝在空集上写判据（§5.55 那条"空集上的全称判据"教训）；改守的是**可见性**——X4 从此挂着读数与正对照，若将来真出现池外动态挂载，`[6]` 的 0 会变，重跑普查即见。**一律未入库**（未执行 `add`／`commit`／`stash`，git 索引由并行作者持有）。

## 5.60 F31 裸 `ElMessage` 从"只报"转"改"：29 份非测试源补上绑定，探针的 flag 侧对照同轮被债务清零打红并改掉（2026-09-30）

**起因**＝这笔债登记已久（探针 `scripts/gate/check-ep-feedback-imports.py`），但一直是**只报不改**：仓库没装 auto-import 插件，`ElMessage`／`ElMessageBox` 只要没有绑定就是全局名，浏览器里每次接口失败都抛 `ReferenceError` 而不是弹提示，源头之一正是 `api/http.js` 自己；`views/admin/panels/_smoke.js` 用 `globalThis.ElMessage=` 桩把症状掩盖掉，所以"smoke 绿"从来不代表面板可用。

**修法**＝不引入新机制，按仓库既有主流写法补显式子路径绑定：52 个已经写对的文件用的就是 `element-plus/es/components/message/index` 与 `.../message-box/index`，所以补的是同一条形状（tree-shakable，与 barrel 引入不同）。落地＝**29 份非测试源文件／54 行 import**，逐文件写前写后图像与插入行号见归档块 `[1]`。作用域判据是"有调用形状（`ElMessage.` 或 `ElMessage(`）且没有该符号的 import 子句"，`.test.js`／`*smoke*` 不参与（测试自己造桩）。复扫缺绑定 **0 处**。

**仪器自捉两枚（修法本身差点造出事）**：① 第一版锚点只认"整行 `import ... from '...'`"，`utils/message.utils.js` 一个 import 都没有 ⇒ 当场拒绝并全量还原；② 第二版用 `split(换行)＋join` 重写文件，`views/admin/panels/AdminLlm.vue` 的换行计数断言立刻红——**语料里有混合行尾的文件**（现量 4 份：`Melody2ScoreView.vue` CRLF 871／裸 LF 1、`MarketDetailView.vue` 397/2、`MarketView.vue` 8/892、`WorkflowView.vue` 611/2），按单一换行重排会把每一行都改成动过，git 判全文件 diff（§见记忆工具坑族）。第三版改成 `splitlines(keepends)` 只插入新行，并加一枚逐字节见证：**把插入的那几行撤掉必须等于写前图像**——这条断言在 29 份文件上全过，才是"只加了 54 行"的证据。插入用的行尾取该文件的主导行尾，不是锚点行的行尾（否则会在 CRLF 文件里插进一行裸 LF）。

**判据自己也被"修好了"打红一次**：`check-ep-feedback-imports.py` 的 flag 侧对照原本硬编 `MUST_BE_FLAGGED = 'api/http.js'`，也就是把"这把尺子还认得出这个形状"绑在"仓库还得留着这笔债"上；债务清零后 `--check` 印出 `0 个 / 0 处` 却 rc=1（`api/http.js (should be flagged)`）。这不是闸门更稳，是对照失去意义，故改判据：flag 侧喂一段**合成样例**给同一个 `classify`，判决只说"形状还认得出吗"，语料现量另行打印不充当期望。四枚见证（块 `[2]`，不符 0 枚）：G0 真对照 rc=0；M1 把样例换成"已补 import 的形状"⇒ 必须红；M2 把期望挪到 `ElMessageBox`（样例用的是 `ElMessage`）⇒ 必须红；M3 把期望退化成空集 ⇒ 必须红。同时诚实登记：语料 0 处之后，clean 侧那 6 个文件在真语料上**无从反咬**，它的牙只剩 `--selftest` 的 4 例形状对照（本轮实测 4/4）。

**门禁**：全量 vitest rc=0（**77 文件／1035 例**＝分母与 §5.59 同值，本轮 0 新用例，纯修复）；`npx vite build` rc=0（**2m13s**）——改 import 形态这类活，vitest 全绿证明不了消费者，验收命令是 rollup（记忆 §53 那条同型教训）；EP 闸门 `--check` rc=0 且读数 **20 个/446 处 → 0 个/0 处**、`--selftest 4/4`；姊妹闸门 `check-framework-imports.py --check` PASS（非测试源 0 处用而未绑）；`check-frontend-module` rc=0（ERROR=0）；`check-locale-format-outlets` rc=0；`check-doc-links` rc=1 属存量，总体读数与本轮贡献见下一段的更正（旧写法"WARN=120"是打印上限）。

**本轮更正（仪器两枚，代价是历轮记的数要重读）**：① **check-doc-links.py 的明细打印被 --max-detail 默认 120 截断**（该脚本源码第 267 行），所以 §5.53／§5.55／§5.56／§5.57／§5.58／§5.59 各节门禁行里那句"WARN=120"记的是**上限不是总体**，而"对两份都截断到 120 的打印集做集合差"只要总体增 1 条，就会同时挤出 1 条旧面孔——表现为 new=1／gone=1，那条 gone 是被顶出上限的**假象**，不是谁被修好了。本轮用 --max-detail 1000 现量：总体 **WARN 216→217**（＋1），断链 **43 处／9 文件不变**，扫描 369→372 文件／判定引用 2058→2071 条；扫描集的增长与那＋1 都落在并行作者 00:32–00:36 落盘的 docs/expert-alliance/ 下 14、15、16 号与 INDEX 上，新增那条告警点名在 14-enterprise-permission-model.md 第 89 行——反引号里把行号后缀 :24 一起写进了路径 token，而目标 docs/enterprise/ 里那份 39 号 V1.1 文档**在盘上存在**，属写法不属断链；该文件未跟踪且正被并发作者改，本轮只报不改（修法＝把行号挪出反引号）。**本文件（治理文档）全文只有 1 条 WARN，在第 137 行（指向已搬走的旧 FRONTEND-MODULE 文档，存量），§5.59 与 §5.60 两次插入各贡献 0 条。** ② 历轮那句"ERROR=0"是**一支永不开火的针**：该闸门的明细标签是 [BROKEN] 与 [WARN]，输出里 [ERROR] 出现 0 次，所以按 [ERROR] 数出来的 0 没有读过任何东西——正确引用是"断链 43 处／涉 9 文件"这行总体。两条都在归档块 `[5]` 里逐字见证（含驱动与两份日志的口径），块 `[3]` 那条被截断的集差行**原样留着不覆写**。

**证据缺口与余账（一律只报不改）**：**Y1** 本轮没有真机证据——`3080/3100/3200` 三端口服器在会话期间是 DOWN，我不擅自启动，所以"接口失败现在弹提示"只有静态＋构建证据；起服后要看的是网络失败时控制台不再出现 `ReferenceError: ElMessage is not defined`。**Y2** `_smoke.js` 的 `globalThis` 桩仍在（它掩盖面的存在本身就是这条链为什么曾长期不被发现的原因），本轮没动它，撤桩要连带看那批 smoke 的断言是否还成立。**Y3** 若引入 `unplugin-auto-import` 这一族可整体消失，但那是构建配置与全仓风格的裁决，不在本轮范围。**Y4** 探针仍是**探针**（债务多少不改 rc），没升成棘轮：升之前要回答"清零后这条闸门在 CI 里靠什么开火"，答案应当就是那段合成样例。**一律未入库**（本轮只按名点名自己的文件）。

## 5.61 F32 文档门禁自己也要认得行号锚点：5 枚在册引用曾被当成写法缺陷，同轮把下游治理 MCP 的门禁路径缺陷修活（2026-09-30）

**起因**＝§5.60 那两条"仪器自捉"留下的账：这把尺子把 docs/…:24 这类带行号的引用整串当路径，判它"文件不存在"；而明细打印被 --max-detail 截断时不告知，害我用两份被截断的打印集做集合差、造出一条根本不存在的"消失"。本轮修尺子本身，并顺尺子摸到它的下游消费者。

**修法①（行号锚点）**＝拆出"目标＋锚点"两件事：目标存在性照旧判，另加一条判据——**锚点越界才算断链**，判决串写"行号越界（目标 N 行，引用第 M 行）"，同一个串既进明细行也进 `--json` 的 reason 字段（治理 MCP 的 broken 条目原样透传）。两处刻意的保守：行数读不到（目录、权限、编码意外）记 −1 表示"未知"，**未知不判**而不是当 0 行（否则一次读失败就凭空造出一批断链）；锚点 0 与负数一律非法。**不做扩展名白名单**——`.mjs:7` 这种也得拆，否则每加一种语言就漏一种（A2 见证）。

**修法②（截断告示）**＝被上限切掉时补一行"⚠ 明细已截断：打印 N 条／总体 M 条"，并把"对两份被截断的打印集做集合差会凭空造出消失条目"写进告示正文；默认上限仍 120（不改默认，免得历轮钉过的口径一起动）。

**修法③（门禁自带对照）**＝新增 `--selftest` 14 格（A 组锚点形状 4／B 组越界与未知 6／C 组截断告示与标签词汇 4，格名与现量见归档块 `[2]`）。**C4 不许借 split_anchor 自己**：它带一枚自己的针形正则，并与实现共享 ANCHOR_TAG 常量——第一版借了被 guard 的那个函数，于是 M1 变异体同时改掉尺子和被尺子审的那格，红格少一枚，正是 §5.60 那条"判据不许从被审对象自己算观测量"的复发形态。门禁侧电池 **7 枚／不符 0 枚**（块 `[3]`）：M1 锚点形状不认 ⇒ 红格 [A1,A2,C4]；M2 越界一律放行 ⇒ [B3,B4]；M3 凡带锚点即判缺 ⇒ [B2,B6]；M4 告示哑掉 ⇒ [C1]；M5 读不到当 0 行 ⇒ [B5,B6]；H1 只改中文措辞 ⇒ 必须不红（rc=0）。

**下游消费者的一枚真缺陷（本轮修活）**＝tools/mox-governance-mcp/server.py 的 `_resolve_script` 把门禁脚本解析成 repo 根下 scripts/ 那一层，而两枚门禁都在 scripts/gate/ 下；AGENTS.md、tools/mox-governance-mcp/README.md 第 14–15 行、SKILL.md 第 12 行三处文档写的都是 scripts/gate/ ——**权威没错，是实现错了**。后果不是"少个便利"：mox_port_verify、mox_doc_links_check、mox_ci_gate 三条真实调用一律 ToolError「门禁脚本缺失」，而 `--selftest` 里**文档门禁这一通道一格都没测过**，所以路径写错照样能印出一大片 PASS。修前读数不靠回忆：取 `git show HEAD:` 的那份 blob 放到同深度目录现场跑，**22 项／失败 9 项**（块 `[5]`）；修后补 4 格（C1 结构化字段齐全／C2 明细受 limit 且与计数同源／C3 计数对上门禁自己打印的总体行／C4 截断告示按"打印<总体"双向在场且告示内的数对得上），现量 **28 项／失败 0 项／rc=0**——6 格差＝1 枚"真实调用 mox_doc_links_check"＋4 枚新格＋1 枚此前根本没能抵达的"端口校验返回结构化字段"；limit 故意取 3，让截断通道真的开火。消费者侧电池 **7 枚／不符 0 枚**（块 `[6]`，只在内存里改 server.py，门禁按盘上原文执行）：M1 路径退回 scripts/ ⇒ 整通道 ToolError（这就是该格的牙）；M2 明细不切 ⇒ [C2]；M3 计数取自明细 ⇒ [C3,C4] 两格（总体被 limit 削的同时也让告示里的总体对不上，级联如实登记）；M4 告示标记换词 ⇒ [C4]（标记漂移必须红，不能静默通过）；M5 字段改名 ⇒ 四格全红（取不到计数退化成 −1 的级联，登记而不假装只红一格）；H1 只改中文措辞 ⇒ 0 红。

**语料效果（同 cap 现量）**＝WARN **217→212**（−5），断链 **43 处／涉 9 文件**不变，rc=1 不变，扫描 372 文件／判定引用 2071 条。少掉的 5 条正是被误判的在册行号锚点，两把尺子对上（块 `[1]`）：门禁自己的明细行＋全仓 grep 独立复算的同一组 5 枚，逐条实测目标行数 302／92／256／256／354，全都在册。**本节自己也被这台仪器抓过一次**：起因那段原先把带行号的样例写在反引号里，写后集差当场显出 new=1（就是这条），按 §5.60 的教训改成裸文本后 new=0/gone=0——引用纪律的散文自己可能就是坏引用这条又应了一次。**由此 §5.60 那句"只报不改"的写法缺陷（14-enterprise-permission-model.md 第 89 行把 :24 写进反引号）不再需要动那份文件**——门禁宽化后它是一条合法引用，而那份文件正被并发作者改，本轮不碰是对的。§5.60 门禁行里"总体 WARN 216→217"是 **F32 之前**的读数，本轮之后按 212 读；那条散文原样留着不覆写。

**门禁**＝门禁 `--selftest` PASS=14／FAIL=0／rc=0；两枚电池各 7 枚／不符 0／rc=0；治理 MCP `--selftest` 28 项 0 失败 rc=0（含 verify-ports.py 的真实调用，该脚本注释登记本机约 130 秒）；`python scripts/gate/check-doc-links.py` 默认跑 rc=1 属存量（断链 43 处／涉 9 文件，本轮既没新增也没消除，写本节前后集差见块 `[4]`（本轮之后另在块 `[8]` 记最终图像，因为改这一处引用本身就会动本文件图像））。**本轮未触碰前端源，全量 vitest 与 vite build 没有重跑**——分母沿用 §5.60 的 77 文件／1035 例，这是登记的缺口而不是通过。

**本轮自己造成的副作用（登记不覆写）**＝§5.60 那句"该脚本源码第 267 行"指的是 --max-detail 默认值所在行，本轮插入把同一行推到 **409 行**。这是"行号引用会被被引文件自己的编辑移动"在本仓库又一次复发：§5.60 记的是它当时的事实，不回填；今后引用它要么点名"本轮之后"的行号，要么写常量名。

**证据缺口与余账（一律只报不改）**：**Z1** 锚点判定依赖目标可读，"读不到行数"记 −1 并放行，所以权限异常或跨盘挂载下的越界引用会**静默漏判**；形状由 B5/B6 见证，真语料 0 例。**Z2** 语料里"越界锚点"现量 **0 条**，因此 C4 那格（被判不存在的不许带锚点）在真语料上无事可审，它的牙只存在于电池 M1 的 [A1,A2,C4]。**Z3** 新加的两条对照（门禁 `--selftest`、治理 MCP `--selftest`）都没接 CI：scripts/gate/check-all.ps1 第 110 行只跑 verify-ports.py，文档门禁连 CI 都没进——与任务 #40 那条"门禁接不接 CI"是同一个待裁决，不擅自接线。**Z4** 治理 MCP 的历史读数要按存疑处理：修前 `mox_ci_gate` 把两条门禁都记成"未能执行"，凡引用过它"体检通过/未通过"的产物都跨了这个边界。**Z5** MCP 的 `stdout_summary` 只取首行，所以那行截断告示不随工具结果返回；调用方要总体必须读 broken_count／warning_count（二者取自 JSON 总体，本轮 C3 已把"计数＝门禁打印的总体"钉成格），这条边界由 C2/C3 看住而不是靠告示。**本轮按名入库**（用户已点名"并提交代码"）：只提交 scripts/gate/check-doc-links.py、tools/mox-governance-mcp/server.py、本文件与 reports/data/ 下本轮的读数文件，`git commit --only` 按名点名，不碰索引里并行作者的条目。

## 5.62 F33 "脚本迁走、引用没改"这型脱钩第一次有退出码：常驻门禁落库并接进 CI，同轮这台仪器三次抓到自己（2026-09-30）

**起因**＝§5.61 那枚下游缺陷留下的覆盖面账：治理 MCP 把门禁脚本指向仓根下的 scripts/，而两枚门禁早已迁进 scripts/gate/。文档侧有 check-doc-links.py 管"路径写死、盘上已迁"，代码与 CI 侧一把闸门都没有。F33 把它落成常驻门禁 scripts/gate/check-script-paths.py。判据是**五态分类＋三个角色**：五态＝EXISTS／MOVED（原样解析不到，但同名文件就在**同一父目录更深处**＝插了一段目录，正是治理 MCP 那一型的形状）／裸文件名（types.rs 这类不构成仓内路径声明）／同名在完全无关目录（同名撞车）／仓内根本没有同名（外链示例、待建产物、第三方）；只有 MOVED 进判决，再按行角色分流——真喂给解释器的算执行面（未在册即 rc=1），注释、Write-Host 文案、三引号串里的算用法文本（WARN：给人错的命令，不改行为）；CI 另开一条通道，按 run: 块的缩进承重点提取被敲下去的脚本路径，按仓根解析不到即判缺（CI 的 cwd 恒为仓根，只有这一种解释）。**遮蔽是承重的**：没有它，帮助文案里的命令全变假阳；遮蔽过度又让真脱钩整份隐身，所以两个方向各有对照格，且"开而不闭"的遮蔽区间不许一路吞到文件尾（宁可少遮蔽多报）。台账 (file, path, hits, reason) 双向核对：处数不等红、条目已不再命中也红，**收口＝删条目，不许把 hits 写成 0 蒙过**；逐行豁免用行尾 script-paths: ignore 标记，跳过量按名印出来（不记暗账）。

**接线三处**＝.github/workflows/ci.yml 的 architecture job 现在先跑 --selftest 再跑体检（新增两行，本轮坐标第 25–26 行）；scripts/gate/check-all.ps1 把这两条塞进**已有的 [6/7] 步内**（步数口径 [1/7]…[7/7] 不变，沿用 §5.61 那条接线规矩；ps1 不执行，用 PSParser::Tokenize 复算语法错误 0）；AGENTS.md 常用命令块补一行。

**当轮现量（每个数都挂在 reports/data/f33-script-paths-gate-readouts-2026-09-30.txt 上，生成时刻见该文件首行）**＝体检 rc=0：枚举文件 3387 份（2193 个不同 basename）｜代码语料 224 份｜CI 语料 5 份｜判定引用 1688 条；覆盖面 EXISTS 663｜MOVED 4｜CI run 引用 9｜裸文件名 451｜同名在无关目录 295｜仓内无同名 266；合计核对 扫描引用 1700 − 显式豁免 12＝覆盖面各档之和＝判定引用 1688，遮蔽不成对 13 处按文件计不进分母；判决分流 MOVED 4＝执行面 4（未在册 0）＋用法文本 0｜CI run 判缺 2（未在册 0）｜显式豁免标记 12 处｜遮蔽区间不成对 13 处；已在册豁免 6 处。内置夹具自检 **PASS=53／FAIL=0／rc=0**（夹具 11 份、10 个不同 basename，绝不读真仓语料）。变异电池 **35 枚／不符 0 枚**（基线 53 格，H1 只改中文措辞必须不红；驱动在 scratch 未落库＝没牙，只当本轮量具，绝不接 CI——同 §5.55 那条规矩）。

**本轮改的代码只有"给人错的命令"那一档**＝7 份文件 16 行用法文本脱钩清零（scripts/ci/ci.py、scripts/deploy/smoke_test.sh、scripts/deploy/start.ps1、scripts/tests/Run-T10-AllTests.ps1、scripts/tests/parse_test_report.py、scripts/tests/run-enterprise-final-acceptance.ps1、scripts/validation/verify_tts_rust_fullstack.py），改后"用法文本 0"——全是注释与提示文案里写死的旧路径，不动任何行为。**执行面 4 处与 CI 2 处一律只报不改、按台账在册**：两枚真 CI 缺陷是 graph-gate.yml 第 21 行执行一份只存在于 docs/_archive/tools/ 的归档脚本、enterprise-ci.yml 第 67 行执行仓内根本没同名的 verify_axioms.py（那个 job 从落库起就没开过火）——搬回 tools/、改指归档、删 job 三种修法都是架构裁决；另四条同在两套半死的验收 harness（Run-T17-EF-All.ps1 三行、run_enterprise_7gates.ps1 一行，后者先 Push-Location platform/backend-node 而该目录整层已不在仓内，从入口就断，逐行改路径等于把半死的链装作能跑）。

**仪器自捉第一例：分母把"没判的东西"算成了判决**＝体检首行曾印"判定引用 1708 条"，而覆盖面六档之和只有 1683，差的 25 恰是显式豁免 12（跳过不判的引用）＋遮蔽不成对 13（按文件计的形状量，根本不是引用）——旧写法是 judged = sum(counts.values())，标签与组成不符，且没有任何一行看得住。修法：分母改由 token 循环**独立**数出（扫描引用），覆盖面行按登记表 COVER_ORDER 印，另印一行"覆盖面合计核对"把两者钉死；分类器若多出一档而没进表，那行按名点名。**证据不是我的断言而是同盘 A/B**（产物的四段 CF 都是现场重算）：CF4 把修前口径塞回去 ⇒ 判定引用 1688→1713 而覆盖面行纹丝不动，两行当场互相打脸；CF1 把已删的"续行不许以 - 开头"守卫塞回去 ⇒ 两版逐字节相同（9 行／9 行、rc 都 0）＝那支确是纸面机制，删它的判据就此有了出处；CF3 把仓根散落脚本从语料里摘掉 ⇒ 代码语料 224→214、判定引用 1688→1625、EXISTS 663→634、裸文件名 451→428、同名在无关目录 295→286，而 MOVED 4 与 CI run 引用 9 一字未动＝这一档加的是 63 条覆盖面、0 条欠账。

**仪器自捉第二例：合计核对能在"空桶"上假装闭合**＝电池 M29（覆盖面表漏登记一档）第一版 rc=0、一格不红。查因不是判据写错，是**夹具根本没那三档**——collect 在微型夹具上只吐出 EXISTS 与 MOVED，FILENAME-ONLY／NAME-ELSEWHERE／NOWHERE 只由"直接调 classify"的格子见证，从没穿过 collect 这条被判定的通道；桶是空的，等式当然成立（§5.55"合成针的夹具必须真的穿过被判定的那条通道"在这里换了形态复发）。补夹具 scripts/states.py（三行三档）＋一格"档位表必须与分类器实吐一比一、CI 档另由 demo.yml 供"＋一枚专打这格的变异体 M33（把那份夹具挪出扫描集）。新格是共享通道的受害者，同轮回填 6 枚旧期望为级联红（M1/M3 撤的是分类分支、M11/M24 断的是 CI 通道、M27/M28 撤的是扫描集，各自让某一档变空）；红格集合按"恰好等于"判，多红少红都不算命中。

**仪器自捉第三例，抓的是本轮自己写进 CI 的注释**＝CF2 把整行 # 的注释原样保留（＝撤掉遮蔽），活语料当场多一枚假阳：ci.yml 第 24 行示例路径 scripts/gate/x.py 被判缺，rc 0→1、CI run 引用 9→10、判定引用 1688→1689。也就是说本轮为了让接线可读而写的两行说明文字，本身就是一处会被这台仪器抓到的坏引用，它没炸只因为遮蔽通道在位——这是"解释引用纪律的散文自己可能就是坏引用"在本仓库的又一次复发（前见 §5.61 起因段），同时是这条遮蔽通道承重的**一手证明**：它的牙不再只存在于夹具（M22／M24 各打红四格），而是有一条真语料差值挂在那里。

**门禁与行号坐标**＝门禁自己的源码在它自己的扫描集里，所以每次编辑都动覆盖面：本轮 1625（CF3 的对照值）→1688 的差由"仓根脚本入扫"与门禁文件自身增长共同解释，不硬编成常数；本节引用的行号（ci.yml 第 25–26 行、graph-gate.yml 第 21 行、enterprise-ci.yml 第 67 行）是本轮坐标，被引文件再被编辑就会移动，按 §5.60/§5.61 的规矩登记而不回填。

**文档口径与其余门禁的回归（读数同样挂在产物末段）**＝check-doc-links.py 在本节写入前后各跑一次，**总结行逐字相同**：扫描文件 372 个／判定引用 2071 条／断链 43 处涉 9 个文件／告警 212 条／rc=1 属存量，截断告示"打印 120 条／总体 212 条"两版都在（产物末段的并排块给出全等判据）。之所以能全等，是因为**本节通篇 0 枚反引号路径**——文档门禁只扫反引号里的路径，而本节要举的正是坏路径（scripts/gate/x.py 那类），写成裸文本既读得懂又不造假告警，这是 §5.61 起因段那条"解释纪律的散文自己可能就是坏引用"的操作化。其余四把闸门按原样通过：verify-ports.py rc=0（WARN 为存量，非本轮引入）、check-frontend-module.py rc=0（ERROR=0）、check-locale-format-outlets.py rc=0（I1/I2 余账按名点名，条数与 §5.35–§5.36 登记的口径一致）、治理 MCP --selftest 28 项失败 0 项 rc=0（§5.61 修活后的现量，本轮那条新门禁没动它的接线）。**本轮未触碰前端源，全量 vitest 与 vite build 没有重跑**——分母沿用 §5.60/§5.61 的 77 文件／1035 例，这是登记的缺口而不是通过。

**证据缺口与余账（一律只报不改）**：**Z1** 扫描集只有 scripts、tools、frontend-ui/scripts、frontend-ui/src/modules 四个目录＋仓根散落脚本＋check-all.ps1；前端 src 其余部分与全部 Rust 代码不在覆盖面内——这是刻意的第一刀（先管住"迁移脱钩"这一型），扩集要另轮，且扩集前必须先量新吞下谁。**Z2** 三档"不判定"合计 1012 条（裸文件名 451＋同名在无关目录 295＋仓内无同名 266）里必然藏着真断链——"仓内无同名"与外链示例、待建产物、第三方字面量同形，静态无从区分，所以只印条数不判决；这 1012 是**登记面**而不是清白。**Z3** CI 词法只看 run: 块里被敲下的命令，变量拼装（Join-Path、${ROOT}）、动态生成的路径读不到——那类脱钩仍只有运行时能抓；后缀白名单与整词匹配（fullmatch）各自的牙由 M9/M10 看住。**Z4** 台账 reason 只核字数（≥20 字），内容无人复核：豁免的正确性靠"每条写明为何不改"的散文约束，不是靠判据。**Z5** 电池驱动与三条同源 A/B 都在 scratch，未落库＝没牙，别接 CI；接进 CI 的只有门禁本体（体检＋--selftest）。**Z6** 文档门禁至今仍未接 CI（§5.61 的 Z3 原样有效），本门禁接了 CI 而文档门禁没接，两把闸门同族而接线状态不一致——与任务 #40 那条"门禁接不接 CI"是同一个待裁决。**Z7** 在册 6 条里有 2 条是**真 CI 缺陷**（graph-gate.yml 的归档脚本、enterprise-ci.yml 的不存在脚本），它们在册不等于被修；每次 push 仍会执行一个解析不到的文件，只是不再静默。**本轮按名入库**：只提交 scripts/gate/check-script-paths.py（新文件）、.github/workflows/ci.yml、scripts/gate/check-all.ps1、AGENTS.md、上述 7 份用法文本文件、本文件与 reports/data/f33-script-paths-gate-readouts-2026-09-30.txt，`git commit --only` 按名点名，不碰索引里并行作者的条目。



## 6. 性能基线（2026-09-26 实测）

| 指标 | 全量根导入（回归态） | 本轮修复后（按需） | 上轮按需基线 |
|------|------:|------:|------:|
| vendor-element JS（raw） | 951.1 KB | 659.3 KB | 647.8 KB |
| main JS（raw） | 125.6 KB | 125.6 KB | 117.6 KB |
| 首屏 JS 合计（gzip 口径） | — | 942.3 KB | 921.9 KB |
| 测试 | — | 22/22 | 22/22 |

本轮 `+20KB`（首屏 gzip）与上轮差值的构成：vendor-element +11.5KB（ElMessageBox/ElTag 显式按需、admin-lowcode 模块可达性变化）、main +8KB（改名后 bundle 分组微变）。**收益**：消除"新代码根导入 → 全量 951KB"的定时炸弹，并落地 §2/§3 规则防止回潮。

相关：布局/外壳方案见 [FRONTEND-LAYOUT-REFACTOR-PLAN-v1.0.md](./FRONTEND-LAYOUT-REFACTOR-PLAN-v1.0.md)。
