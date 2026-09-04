# 企业级规范术语表 (GLOSSARY · DOC-GLOSSARY-V1.5 · 三联盟统一版)

> **唯一事实源（Single Source of Truth）**：本文档是 `infotopograph` 项目全部文档的**权威术语基准**。
> 所有 `.md` 文档的术语以本表为准；各权威文档在文末以「见 `docs/GLOSSARY.md`」引用，**禁止各自复制维护术语表以避免漂移**。
> 命名约定：中文术语为主，英文/缩写首次出现附原文；专有名词大小写固定。
> **版本**：DOC-GLOSSARY-V1.5 · 2026-08-24
> **主责联盟**：三联盟联合（产品=业务与产品名 · 算法=图谱/算法术语 · 开发=工程/路径术语 · 测试联盟=评测术语）
> **权威链**：🟢 L0 → `docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`（TOP-MASTER）。
>
> **新增 v1.1 7 大术语（对齐 18 TOP-MASTER）**：**璇玑 RelGraph** · **三联盟模式** · **六层金字塔** · **八层图谱（L0~L7）** · **四归三连铁律** · **9 里程碑（M0~M8）** · **八大算法家族（A1~A8）**。
> **新增 v1.2 3 大术语（对齐 21 Aura 对外 SRS = 对外/对内 双命名并行归一化）**：**璇玑（Aura）软件研发数字孪生中台** · **璇玑四道中台（气道·体道·神道·技道）** · **Aura 6 大闭环阶段 / 8 大对外实体 / 5 大对外算法**（对外口径 ↔ RelGraph 内口径双名并行注册，双向无损映射，权威链不冲突）。
> **新增 v1.3 2 大术语（对齐 22 全文档归一化总控卡 = 归一化治理裁决规范总册）**：**全文档归一化总控卡（DOC-NORMALIZATION-MASTER 22 号）** · **Aura SRS 全要点落地清单（4 栏打钩 100% 闭环 = 对外 SRS 归一化完成证书）**。
> **新增 v1.4 2 大术语（对齐 26 前端主控提示词 = 前端最佳开源 Skills + 流程透明化 6 视图 12 看板）**：**Aura 前端最佳开源 Skills 矩阵（S1~S12 12 类选型）** · **Aura 流程透明化 6 主视图 + 12 透明化看板架构**。
> **新增 v1.5 2 大术语（对齐 27 企业级测试主控提示词 = 8 大类≥48 规范标准题库 + mox 模块化系统架构自动化测试判定体系）**：**璇玑企业级 8 大类≥48 规范标准评测题库（T0 18 烟测硬门槛 + 严格单次 + Δ≤1e-6 + SHA-256 全程留痕 6 字段齐全）** · **璇玑mox 模块化系统架构自动化测试流程与判定体系（6 阶段 + T1~T7 7 份交付物 + T6 7 条 RELEASE_L2 / L1 条件 / REJECT 3 选 1 判定树）**。
> · **治理注记（2026-08-24，v1.3 术语表述微调）**：对历史部分旧比喻术语统一做软件工程治理域中性化映射：治理裁决枢纽 / 规范总册 / 最终裁决人，全程使用纯工程治理术语。

---

## 1. 系统与架构主体（DOC-GLOSSARY-V1.5 · 扩展为 23 项：v1.1 14 项 + v1.2 新增 3 项 + v1.3 新增 2 项 + v1.4 新增 2 项前端治理 + v1.5 新增 2 项测试治理）

| 术语 | 英文 / 缩写 | 定义 | 载体 / 出处 |
|------|------------|------|------|
| **🔹 璇玑 RelGraph**（项目对外产品名 · v1.1 新增） | Mox RelGraph (产品代号)；别名 OUS（底层算子父系统代号）/ 关图（内部图谱别名） | **全域归一化知识图谱自动化协同平台**。以「一张关图（GR-STD）作为唯一事实基准」为世界观，通过产品联盟 / 算法联盟 / 开发联盟三联盟协同闭环，把需求 / 架构 / 业务流程 / 文档资源 / 代码模块五向关联归一化治理，实现「一改全链联动、零重复造轮子」。 | `docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`（TOP-MASTER） · ADR-DOC-011 |
| **🔹 璇玑（Aura）软件研发数字孪生中台**（v1.2 新增 · 对外商业/签约/官网正式名，与「璇玑 RelGraph」双名并行 · 100% 同构） | **Aura**（对外英文名：*Aura · R&D Digital Twin Platform for Software Engineering*） | 与「璇玑 RelGraph」是**同一个产品的两个命名**：「璇玑 RelGraph」= 对内工程/研发代号（面向三联盟工程师）；「璇玑（Aura）软件研发数字孪生中台」= 对外商业/签约/标书/官网 正式名（面向客户/市场/招标）。**两者不允许出现架构/功能/边界/SLA 冲突**，双向映射通过 `21` 号文档 8 大权威链对齐矩阵 100% 无损绑定（四归三连铁律）。 | `docs/enterprise/21-璇玑（Aura）软件研发数字孪生中台-企业级需求规格说明书-V1.0.md`（本文档 v1.2 新增的权威注册源） · `docs/enterprise/15` 产品契约 · ADR-DOC-011 增补双名并行注 |
| **🔹 璇玑四道中台（气道·体道·神道·技道）**（v1.2 新增 · 产品内部战略体系名 = 「六层金字塔 L6→L1」的客户视角重命名 · 不是第二套架构） | Four-Verticals of Aura: Qi (气道/Base) / Ti (体道/Business) / Shen (神道/Intelligence) / Ji (技道/Delivery) | Aura 对外产品的**四层演进体系**（客户/产品视角）：气道=底座（L2+L1，V1.0）、体道=业务（L5+L4+L2，V1.5）、神道=智能推理（L3+L4+L6，V2.0 企业版）、技道=交付（L6+L1，V3.0 长期）。**严格等价**= 六层金字塔（L6 产品应用 / L5 业务流程 / L4 图谱核心 / L3 算法推理 / L2 Rust 底座 / L1 部署运维）的「产品演进视角 1:1 拆合矩阵」（21 §六 权威链对齐表写死），不是第二套架构；不允许出现第 5 道 / 第 7 层，除非走 ADR 注册 + 18 TOP-MASTER 同步更新。 | `docs/enterprise/21` §六（权威源）· `docs/enterprise/18` §二（六层金字塔，四道的内部等价源） · GLOSSARY 六层金字塔条目双向锚点 |
| **🔹 Aura 6 大阶段 / 8 对外实体 / 5 对外算法**（v1.2 新增 · Aura 对外 SRS 核心三件套 = 对内 10 BP / 14 节点族 / 8 大算法家族 的「对外精简子集视图」，不是第二套模型） | Aura 6-Phase Pipeline / 8 Public Entity Schema / 5 Public Alg | Aura 对外文档使用的「简化口径」：① 6 大闭环阶段 = 10 大标准业务流程（BP-01~10）的对外打包视图；② 8 大对外实体（E1~E8）= 八层图谱 14 节点族的严格子集注入（21 §四 对齐矩阵写死，一一对应无遗漏/新增）；③ 5 大对外算法 = 8 大算法家族（A1~A8）的**组合调用**（A1~A8 不新增、不替换，仅对外说的是「组合能力」而非「原子算法」）。 | `docs/enterprise/21` §三/§四/§五（权威源）· 双向锚点：10 BP（04 §2）/ 14 节点族（18 §三）/ A1~A8（18 §五） |
| **🔹 三联盟模式**（v1.1 新增） | Three-Alliance Model（产品联盟 PA / 算法联盟 AA / 开发联盟 DA） | 璇玑 RelGraph 的组织治理模型：**产品联盟**负责"要不要做（需求&合规）"、**算法联盟**负责"做不做得对（图算法/复杂度/对账 Δ）"、**开发联盟**负责"做不做得稳（工程落地&部署&稳定性）"。任何交付物必须三联盟各负其责、自证自验（All-04 铁规），禁止跨联盟甩锅。 | 18 TOP-MASTER §一.3 · `docs/enterprise/06` §5 RACI 矩阵 · `docs/enterprise/07` All-01~04 · ADR-DOC-002 |
| **🔹 六层金字塔架构**（v1.1 新增） | Six-Layer Pyramid Architecture（L6↘L1） | 璇玑 RelGraph 的跨文档统一分层锚点：**L6 产品应用层 · L5 业务流程层 · L4 知识图谱核心层 · L3 算法推理层 · L2 Rust 自研工程底座 · L1 部署运维层**。七视图 TOGAF 架构（02）及所有模块必须显式标注其所属 L 层级。 | 18 §二 · `docs/enterprise/02` §0.1 双向锚点表 · ADR-DOC-003 |
| **🔹 八层图谱（L0~L7）**（v1.1 新增） | Eight-Layer Knowledge Graph Schema（L0↘L7） | 璇玑 RelGraph 唯一图模型：**L0 需求根层 / L1 业务层 / L2 功能层 / L3 算法层 / L4 实现层 / L5 资源层 / L6 治理层 / L7 表现层**；承载 **14 节点族 × 19 边族**；是 GR-STD 关图规范（12 类节点/7 类边）的企业级扩集。 | 18 §三 · `platform/domains/kg-hub` · graph.enterprise.json（372/751） · ADR-DOC-004 |
| **🔹 四归三连铁律**（v1.1 新增） | 4-Normalization + 3-Linkage Governance | 任意代码 / 需求 / 流程改动必须做到：**四归**=需求↔架构↔业务流程↔文档四方同步更新；**三连**=联盟责任（06 §5 明确）· 流程（04 BP-xx 明确）· 代码（平台路径真实落地）三者串联。缺一方 PR 阻断。 | 18 §四.4 · `docs/enterprise/07` All-03 · `docs/enterprise/04` BP-10 · ADR-DOC-010 |
| **🔹 9 里程碑（M0~M8）**（v1.1 新增） | 9 Milestone Roadmap（M0→M8） | 璇玑 RelGraph 统一排期口径：M0 全域归一化 / M1 业务闭环 / M2 算法核 / M3 AI 统一编排 / M4 存储分布式 / M5 可观测 HA / M6 多云 / M7 生态 / M8 自治；**三级验收门槛**：L0（单元&静态全绿）→ L1（集成&E2E 全过）→ L2（SLO 达标 & 外部验收）。 | 18 §八 · `docs/enterprise/05` §7 里程碑表 · ADR-DOC-008 |
| **🔹 八大算法家族**（v1.1 新增） | Eight Algorithm Families（A1~A8） | 璇玑 RelGraph 唯一指定算法实现集合，禁止等价自研：A1 CNM 社区检测 / A2 Brandes 2001 介数中心性 / A3 Harmonic 紧密中心性 / A4 PageRank（含转置图处理）/ A5 激活扩散（个性化 PR·d=0.85·30 轮）/ A6 RRF 结果融合（k=60）/ A7 CEM 交叉熵优化 / A8 CPM 关键路径 + RCPSP 资源调度。 | 18 §五 · `platform/domains/graph-algorithms` · `optimizer` · `ai-agent` · ADR-DOC-007 |
| **璇玑**（保留，与「璇玑 RelGraph」不混用） | Xuánjī Engine / `mox-expert` crate（技术代号） | 归一化 IR 驱动的**元调度诊断引擎实体**：双璇玑十四维并行诊断 → 裁决 → flow-ai 求解 → ⛨验证网关 → 治理闸门 → 出码/出图。注意：「璇玑 RelGraph」是**整个产品名**；「璇玑」单独出现时指 `mox-expert` 中的最高权限引擎。 | `platform/domains/mox-expert/` · ⛨璇玑验证网关 |
| **关图**（保留，图谱别名） | 信息关联关系图 / GR-STD-V1.0 | 「一切皆是信息」：所有信息实体抽象为**节点**，关联关系抽象为**边**，以需求为根节点无限扩展，构成全栈信息关联图，作为项目唯一基准。是璇玑 RelGraph「八层图谱（L0~L7）」的 GR-STD 原始子集（12 节点 / 7 边）。 | `docs/specs/GR-STD-信息关联关系图开发规范-V1.0.md` · 八层图谱 L0~L7 扩集 |
| **AA-STD**（保留） | mox 模块化系统架构需求业务处理流程图-归一化企业级 | 融合域**需求事实基准**，承载 REQ→FUN→BIZ→ALG→TSK→COD 五向绑定的归一化主流程。 | `docs/璇玑-mox 模块化系统架构需求业务处理流程图-归一化企业级.md` |
| **PT-Primi / PrimiFlow**（保留） | 全域拓扑原语架构 V1.0 | operator-unified-system 之上的**元调度大脑层**（meta-scheduling brain）；κ-τ 拓扑原语调度，守恒律 `C² = κ² + τ²`。 | `docs/specs/PT-Primi-架构规范-V1.0-完整版.md` |
| **OUS**（保留 · 父系统代号） | operator-unified-system | 算子统一系统（Rust 底层父系统 v3.0.0-ai-powered，多 crate 架构），提供算子侧稳定能力。**对外产品名统一为「璇玑 RelGraph」**，OUS 仅作父系统技术代号，禁止在对外文档中与产品名混用。 | 仓库根 `platform/domains/`（原 `crates/`） · README 技术描述区 · ADR-DOC-011 |
| **双璇玑十四维**（保留） | Dual-Xuánjī 14-Dim | 业务 7 维 + 开发 7 维并行诊断的体系化维度模型。三联盟责任：业务 7 维=算法联盟 R+产品联盟 C；开发 7 维=开发联盟 R。 | 璇玑系统 · 06 §5 RACI |
| **mox 模块化系统架构**（保留） | full-dimensional | 覆盖需求/架构/设计/业务/测试/验收/归档的mox 模块化系统架构维度工程视图。 | `docs/full-dimensional/` |
| **🔹 全文档归一化总控卡（v1.3 新增 · DOC-NORMALIZATION-MASTER · 22 号文档）** | Docs Normalization Master Card（9 大映射表 + 8 步裁决流程） | enterprise 23 份文档（00~22）的**归一化治理枢纽**。唯一功能 = 把散落在 18/01/02/04/05/06/07/08/10~17/19/20/21 共 19 份文档中的「命名 · 架构分层 · 业务链路 · 图谱 Schema · 算法组合 · 版本打包 · NFR 棘轮 · 文档职责 · SRS 逐段锚点」9 大类等价关系焊死成 9 张单源映射表（合计 156 行）。权威级 = **L1 · 治理枢纽**（仅低于 18 TOP-MASTER，高于所有 L1/L2 下游文档；冲突=改下游，不得反改本卡，改本卡须 4 位签字）。 | `docs/enterprise/22-全文档归一化总控卡与权威链单源映射表-V1.0.md`（权威源） · 00-INDEX §0 三大红线入口第 3 条（归一化裁决入口=本卡） |
| **🔹 Aura SRS 全要点落地清单（v1.3 新增 · 4 栏 100% 打钩=归一化闭环完成证书）** | Aura SRS 100% Coverage Checklist（4-Column ✅） | 21 号对外 SRS 文末 §十一 的**可交付验收清单**。方法 = 把用户给的 SRS 原文按「最小可验收单元」拆成 N 条原子要点（本次 v1.0 = 99 条），每条 4 栏打钩：① 21 SRS 有没有载点位（=对外不悬空）② 内部 22 份权威文档有没有对应锚点（=对内能落地）③ 22 总控卡有没有对应表/行（=名实分裂已焊死）④ 15 Crate 有没有对应代码载体（=有实锤，不是空话）。**4 栏全 ✅ = 这条真正闭环**；本次 99×4=396 钩 100% 全齐 = Aura 对外 SRS 已归一化闭环，可直接 Word/PDF 导出合同。 | `docs/enterprise/21` §十一（清单载体） · 22 号总控卡「8 步裁决流程」（发现 ❌ 怎么补的标准流程） |
| **🔹 Aura 前端最佳开源 Skills 矩阵（v1.4 新增 · S1~S12 12 类选型 · 前端开发联盟必须 100% 按此选型）** | Aura Frontend Open-Source Skills Matrix（S1~S12，12 Categories） | 璇玑 Aura 前端生态的**最佳选型矩阵 12 类**（写死，前端开发者不得自造轮子/换栈）：S1 基础 UI=Element Plus / S2 状态=Pinia+持久化+时间旅行 / S3 关图可视化=G6 Graphin 或 vis-network / S4 流程可视化=LogicFlow+ElSteps / S5 图表=ECharts 5 / S6 表单校验=zod+VeeValidate / S7 请求+Mock+SSE=Axios+MSW+@vueuse/core / S8 测试三件套=Vitest+Playwright+Storybook / S9 脚手架=Plop / S10 拖拽=vuedraggable / S11 Diff Viewer=Monaco Editor+diff-match-patch / S12 指引与通知=Shepherd+ElTour。选型原则 = 原生 Vue3 + TS 类型完美 + Star≥10k + 与关图/RBAC/对账天然适配 + 与 Element Plus 风格统一。 | `docs/enterprise/26-前端开发专家主控提示词与流程透明化最佳实践清单-V1.0.md` §四 选型矩阵（权威源） · frontend-ui/package.json · frontend-ui/README.md 顶部前端 PR 模板 |
| **🔹 Aura 流程透明化 6 主视图 + 12 透明化看板架构（v1.4 新增 · 前端 UI 必须覆盖的全流程可见性架构，写死 100% 覆盖不得缺）** | Aura 6 Main Views + 12 Transparency Dashboards（All-Flows-Visible Architecture） | Aura 前端「**所有处理流程都可以看得见**」的写死架构（缺 = 越权，PR 直接打回）：**6 主视图**=F2 Aura 总览仪表盘（首页）/ F3 关图可视化主视图 / F4 实体&业务&流程管理 / F5 变更影响推演 What-If / F6 质量治理总仪表盘 / F7 算法联盟对账仪表盘。**12 透明化看板**=K01 mox_optimize 8 步 / K02 BP-01~10 业务流程 / K03 Verify 14 专家 / K04 7×8 算法对账热力图 / K05 P9 判重闸门 / K06 RBAC 6 角色+11 探针 / K07 模块管理 15 Crate / K08 需求覆盖率+流程断点 / K09 代码双向绑定覆盖率 / K10 操作历史时间线+回滚 / K11 审计链 6 字段查询 / K12 对外版本+里程碑总览。外加 4 全局组件（命名切换 / RBAC 双模式 / 通知中心 / Shepherd 新手引导）。 | `docs/enterprise/26-前端开发专家主控提示词与流程透明化最佳实践清单-V1.0.md` §八 F1 交付物门槛（权威源） · frontend-ui/src/router/routes.ts 路由表 · MoxFusionView.vue 8 步主视图样板

## 2. 核心机制与契约

| 术语 | 定义 | 说明 |
|------|------|------|
| **TraceMatrix / 六维绑定** | `REQ→FUN→BIZ→ALG→TSK→COD` 的逐层可追溯绑定矩阵，保证零孤儿节点。 | 承载于 AA-STD §3 + `crates/primiflow-core/trace_matrix.md` |
| **五向绑定** | requirement→function→algorithm→flow→code 的端到端可追溯链。 | 见 glossary §1 "AA-STD" |
| **κ-τ 拓扑原语调度** | PrimiFlow 原生调度算法：κ（曲率/结构复杂度）与 τ（扭转/时序约束）守恒。 | 守恒律 `C² = κ² + τ²` |
| **⛨ 璇玑验证网关** | 闭环出码/出图前的**最高权限验证网关**，对诊断结论做最终裁决与放行。 | 治理闸门上游 |
| **治理闸门** | Governance Gate：在出码前对合规性、零死代码、禁伪代码做门禁拦截。 | `govern` crate |
| **归一化** | Normalization：将分散/重复/过程稿文档统一为单一事实源、统一命名/编号/引用/锚点的治理动作。 | 见 `docs/DOC-NORMALIZATION-REPORT.md` |
| **判重闸门 (P9)** | 需求判重与去噪闸门，防止重复需求进入流水线。 | enterprise/16 |
| **关图骨架** | guantu-skeleton：GR-STD 的 REQ 根 + 六维绑定骨架 + 偏离检测承载文件。 | `docs/full-dimensional/guantu-skeleton.md` |

## 3. 文档治理等级

| 标记 | 含义 |
|------|------|
| 🟢 权威 (Authoritative) | 以该文档为准的单一事实源 |
| 🟡 参考 / 过程稿 (Reference / Draft) | 仅供追溯，结论已沉淀入 🟢 文档 |
| ⛨ 网关级 (Gateway) | 最高权限验证/裁决节点 |

## 4. 常见产物与图类型

| 术语 | 含义 | 归属 |
|------|------|------|
| 需求辐射图 | 以需求为根向关联实体辐射的关图视图 | 关图 / PrimiFlow |
| 业务流程图 | 业务处理 S1–S8 主流程可视化 | AA-STD |
| ER 图 | 实体-关系图 | 关图 |
| 功能关联图 | PrimiFlow 输出的功能关联拓扑视图 | PrimiFlow（与关图概念邻近，不混用） |
| 定时任务甘特 | 定时任务时序甘特视图 | PrimiFlow |

## 5. 命名与大小写约定（强制 · 三联盟联合签署 ADR-DOC-011）

- **产品名固定**：对外统一使用「**璇玑 RelGraph**」；父系统底层代号「OUS」仅在技术架构说明区出现；图谱别名「关图」仅出现于 GR-STD 文档内；**禁止**「璇玑系统」「璇玑子系统」「OUS 子系统」「关图治理平台」等混用（ADR-DOC-011）。
- **引擎名区分**：单独出现「璇玑」时特指 `mox-expert` 中的 ⛨最高权限验证引擎；与产品名歧义处必须使用「璇玑 RelGraph（产品）」或「⛨璇玑引擎（技术）」显式限定。
- 其他专名固定写法：**关图**（信息关联关系图简称）、**AA-STD**、**GR-STD**、**PT-Primi**（非 PT-PRIMI）、**TraceMatrix**（非 Tracematrix）、**六层金字塔**、**八层图谱 L0~L7**、**八大算法家族 A1~A8**、**9 里程碑 M0~M8**、**三联盟模式（PA/AA/DA）**。
- 英文专名首次出现附中文：*璇玑 RelGraph (Mox RelGraph)*；后续可用简称。
- **代码路径（固定 · ADR-DOC-005，M0 全域归一化）**：
  - Rust 独立子项目目录：`platform/domains/<crate-id>/`（原 `crates/` 已废弃，全仓 grep 命中必须为 0）
  - Rust 聚合网关：`platform/gateway/runtime/`
  - Node 边缘入口（M0 瘦身中）：`platform/backend-node/` → M0 完成后 → `platform/edge-node/`
  - 前端单应用（唯一前端）：`frontend-ui/`（原 `frontend/` 与 `frontend-admin-ui/` 已归一合并，admin-ui 已删除）
- 文档引用一律使用仓根相对路径 `docs/<rel>`，禁止 `../`、裸名、同级简写混用。
- 编号章节配 anchor 锚点，便于跨章节引用与导航。
- **三联盟术语管理流程（BP-10 文档同步治理铁律）**：新术语登记→GLOSSARY 补行→三联盟 PR 签字→CI lint 通过（命名大小写 & 路径格式）→正式生效。未经登记即使用的新术语=文档漂移，CI fail。

---

*本术语表为活文档，新增专有名词须同步登记；变更须经 `docs/enterprise/00-INDEX.md` 变更记录留痕。*
