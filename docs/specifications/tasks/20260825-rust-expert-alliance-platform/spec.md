# 开发专家联盟全维分析 · Rust企业级Platform集成（Spec）

> **标准编号**：ENT-SPEC-EXPERT-ALLIANCE-RUST-V1.0  
> **自然语言**：中文  
> **适用范围**：Rust 全量实现专家联盟 6 阶段全维分析引擎、`platform/gateway/runtime` 路由集成、`mox-expert` 升级（14 维专家 × 辩论合成 × 质量门禁 × 指标学习）、前端 `ChatView.vue` 全维流程桥接、以及语音对话 ASR/TTS/桌面小白 端到端交付。  
> **对齐基线**：TOP-MASTER `docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`（BP-05 算法推理 + 三联盟闭环 + 8 大算法家族 + 六层金字塔 L3/L2/L5/L6 跨层契约）、项目记忆（11 条硬约束）。

---

## 1. 问题定义（Problem）

当前系统已具备以下能力基座：

| 层 | 已有能力 | 现状路径 |
|---|---|---|
| L3 算法推理 | `mox-expert` 14 维专家 trait + 并行 dispatch + reconcile 裁决 + RBAC + 审计链 | `platform/services/mox-expert/` |
| L2 网关底座 | `runtime` 已挂载 `/ai/engine/{process,analyze,capabilities,metrics}` 四端点 + `ai_router` AC-10 语义 | `platform/gateway/runtime/` |
| L5 业务流程（Node） | `backend-node/src/expert-alliance/` 已实现 6 阶段流程：意图分类 / 专家匹配 / 并行咨询 / 辩论合成 / 质量门禁 / 学习技能，但**双真相实现**（Rust 有专家，Node 有流程） | `platform/backend-node/src/expert-alliance/` |
| L6 前端交互 | `ChatView.vue` 已具备专家选择器 + 全维分析 CTA + 5 阶段 Chip + MessageBubble 9 动作工具栏 | `frontend-ui/src/views/ChatView.vue` |
| 语音服务（Python） | `projects/xiaobai_voice/` 已实现 ASR(Paraformer+sherpa-onnx) + TTS(Fish-S2-Pro/CosyVoice2 双回退) + FastAPI 3717 + 桌面浮窗小白 | `projects/xiaobai_voice/` |

但仍存在 **5 类阻塞性问题**，必须在本 Spec 中根治：

| # | 问题 | 影响（企业级严重性） |
|---|---|---|
| P1 | **双真相漂移**：专家联盟 6 阶段流程在 `backend-node/` 用 Node.js 实现、14 维专家在 `mox-expert/` 用 Rust 实现——两套实现长期并行会产生算法漂移（如辩论参数不一致）、违反 HC-14（后端 100% Rust）。 | 与 TOP-MASTER §六 HC-14 冲突；L2 企业基线不通过。 |
| P2 | **全维分析端点缺失**：`/ai/engine/*` 四端点只做"单次查询路由"，**没有对外暴露专家联盟 6 阶段的完整流水线端点**（含 trace_id、阶段 SSE、Gate A/B/C/D 级输出）。ChatView 全维 CTA 按钮只能调用空壳函数，无法触发真实 Rust 分析。 | ChatView FR11（全维分析功能）不落地；用户感知"按钮好看但不工作"。 |
| P3 | **意图识别未统一到图谱承载**：HC-2（业务流程与算法流程必须统一承载于图谱引擎）未被满足——当前意图分类用 Node 关键词匹配，未走"激活扩散（个性化 PageRank 特例 method=spread, d=0.85, 30 轮收敛）"图谱算法，且未与 `graph-algorithms` 的 spread 实现对齐。 | AC-10 路由准确率不达标；7 类基准 TopK MAP<0.82。 |
| P4 | **语音集成未闭合**：`xiaobai_voice` Python 后端完整，但前端 `ChatView.vue` 缺少麦克风录音按钮 + TTS 播放逻辑 + `/voice/*` 代理桥接，桌面小白浮窗未被 platform 服务管理器纳入生命周期管理。 | 用户"点击页面语音对话"诉求不满足；AC-语音 0/7 通过。 |
| P5 | **指标学习闭环缺失**：专家联盟的"学习"阶段（第 6 阶段）在 Node 中是写入 skill-store，但没有与 Rust `harness.rs` 的 Plugin/Expert 注册机打通，也没有暴露 `/ai/engine/metrics` 的意图分布直方图。 | 系统使用越多，准确率不提升，违反"全自动全维"产品契约。 |

本 Spec 目标：**用一次 Rust 企业级集成 + Platform 路由收敛 + 前端语音 UI，100% 消除以上 5 类问题，使专家联盟全维分析成为"Rust 单一真相源 + 图谱承载 + 可观测可学习 + 语音可用"的生产级能力。**

---

## 2. 用户 & 目标（Users & Goals）

### 2.1 利益相关者（RACI）

| 角色（R=负责/A=审批/C=咨询/I=通知） | 利益 | 本 Spec 中的核心诉求 |
|---|---|---|
| 全域总设计师（你） | A | 单真相源、HC-14 合规、全维分析真工作、语音端到端闭合。 |
| 算法联盟 Owner | R | 意图识别走 spread HC-5、辩论参数锁死、CEM 停止条件 σ̄<0.06 / 3 轮不改进。 |
| 开发联盟 Owner | R | Rust Controller→Service→Repo 分层零越层；unsafe=0；`cargo test -p mox-expert` 全绿。 |
| 产品联盟 Owner | A | ChatView 全维按钮 1 击可用；6 阶段 Chip 真流转；三流程端点语义对齐。 |
| 安全合规方 | C | RBAC 中间件覆盖所有新路由；审计链审计 `alliance.*` 事件；`unsafe` 计数为 0。 |
| 最终用户（中文普通话） | I | 语音对话零配置可用（默认浏览器 TTS 降级优先）；全维分析返回结构化报告含 Mermaid。 |
| 信创政务部署方 | I | TTS 默认 CosyVoice2（Apache 2）而非 Fish-S2-Pro（Research License）；ASR Paraformer+sherpa-onnx 全平台。 |

### 2.2 目标（Goals）

- **G1（Rust 单一真相）**：专家联盟 6 阶段流程（意图→组队→并行→辩论→合成→门禁→学习）**100% 在 Rust `mox-expert` crate 中实现**，Node `backend-node/expert-alliance/` 只保留薄代理层（302 重定向到 Rust 端点，或 sidecar 透传）。
- **G2（全维分析真闭环）**：ChatView 点击「全维分析 φ」按钮，真实触发 Rust 全维分析流水线，返回：阶段 SSE 流（6 Chip 流转）+ 14 维专家观点矩阵 + reconcile 归一化报告 + Gate A/B/C/D 评级 + 可导出的结构化产物。
- **G3（图谱承载意图）**：意图识别统一走 `graph-algorithms` 的激活扩散实现（HC-5：method=spread, d=0.85, 30 轮收敛），种子向量 = 关键词先验 + Chat 历史反馈，RRF 融合 k=60，spread_weight=0.7。
- **G4（语音全闭合）**：ChatView 输入框左侧新增麦克风按钮，支持录音→ASR→转文字→提交；MessageBubble 朗读按钮优先调 Rust `/voice/tts`（3717→网关代理），不可用时回退 Web Speech API；桌面小白浮窗启动后与前端共用同一 session_id。
- **G5（指标学习可观测）**：`GET /ai/engine/metrics` 输出：成功率 / 降级率 / P50/P95 延迟分桶 / 意图分布直方图 / Gate A/B/C/D 通过率 / 学习到的 skill 数；新路由全部带 RBAC + 审计签名。

### 2.3 非目标（Non-Goals，显式不做）

- **NG1**：本次不重写 `mox-expert` 已有 14 位专家的 `analyze()` 逻辑（业务 7 + 开发 7 共 14 个 `*.rs`），只在其上封装联盟流程管线。新增的专家/维度需要先经 ADR。
- **NG2**：不把 `backend-node/` 全部删除（ADR-DOC-005 规划中改为 edge-node，本次只把 expert-alliance 迁出）。
- **NG3**：本次不做 TTS 模型实际下载（避免 CI/CD 拉取 >1GB 权重文件）。冒烟测试使用 BrowserFallback 的 Web Speech API 降级链路验证。
- **NG4**：不修改现有 workspace 依赖版本（除新增必需依赖外）。
- **NG5**：不把 Python `xiaobai_voice` 改写为 Rust（远期规划，本次用 HTTP 桥接方式集成）。

---

## 3. 约束 & 依赖 & 假设

### 3.1 硬约束（来自项目记忆，必须 100% 满足，违反 = 直接 Blocked）

| 编号 | 约束 | 落地路径 |
|---|---|---|
| HC-1 | 业务流程与算法流程必须统一承载于图谱引擎 | 意图识别阶段必须调用 `graph-algorithms` 的激活扩散（见 `platform/services/graph-algorithms/src/lib.rs` 的 activate_spread） |
| HC-2 | 激活扩散意图识别：**个性化 PageRank 特例 method=spread, d=0.85, 30 轮收敛**，禁止替换 | `alliance::intent::spread_activate()` 中把参数写死为常量 |
| HC-3 | 社区检测：**CNM 模块度贪心凝聚**，禁止 LPA | 组队阶段用 CNM 聚类能力向量找专家组（非必要但统一算法族） |
| HC-4 | 介数中心性 Brandes / 紧密中心性 Harmonic，禁止变体 | 组队打分若用中心性，必须调用 `graph-algorithms` 已验证实现 |
| HC-5 | 后端 100% Rust 全维自研：新端点 handler 只能调 Rust service，禁止直连 Node/外部进程做业务判断 | gateway/runtime 新增路由 → `subservers.rs` 注册 → 调 `mox-expert::alliance` crate 内部函数 |
| HC-6 | AI 引擎统一 4 端点 + 路由语义 AC-10：静态优先→参数少→长路径 | 新路由 `/ai/engine/alliance/*` 作为静态长路径注册到 `routes/ai_engine.rs`，完全在 4 端点体系内扩展 |
| HC-7 | CEM 停止 σ̄<0.06 或 3 轮无改进 | 若全维分析中使用 CEM 寻优（如组队最优），严格按此停止 |
| HC-8 | 评估统一公式 Score = 0.55Q + 0.20S + 0.10T + 0.15Stab，权重不可换 | 质量门禁的综合打分严格用此公式 |
| HC-9 | 7 类基准任务：数学/逻辑/知识/代码/中文/时效/指令，缺任何视为基线不通过 | 全维分析的能力矩阵必须把 7 类都注册到 `capabilities` |
| HC-10 | 流程节点创建 → 边添加顺序（图谱构建） | 若全维分析输出流程图，严格按此顺序构建，避免静默丢边 |
| HC-11 | 所有中心性指标输出附带人读公式；密度指标附带解读文案（高度稠密 / 中等密度 / 稀疏图） | reconcile 归一化报告的算法部分必须满足 |
| HC-12 | 语音 `melody2score` 打包发行版的 stderr/stdout None 兜底（不直接适用 xiaobai_voice，但语音集成若打包必须遵循） | `xiaobai_voice/build_exe.ps1` 的 windowed 模式兜底逻辑保留 |
| HC-13 | 无向图 RAW 边输入在库内展开 | 意图识别的 spread 若用无向图，必须经 graph-algo 库内 RAW 展开 |
| HC-14 | 公式库保留全精度，禁用 toFixed 截断 | Rust 侧使用 f64 默认精度输出，禁止在 Rust 中做显示级舍入（舍入由前端 UI 控制） |

### 3.2 依赖

- Rust workspace 成员：`mox-expert`（核心）、`graph-algorithms`（HC-2 spread）、`ai-agent`（若需 LLM 辅助辩论）、`runtime`（路由挂载）
- Python 侧：`projects/xiaobai_voice/` 3717 FastAPI 服务（语音桥接）
- 前端：`ChatView.vue`、`MessageBubble.vue`、`frontend-ui/src/api/index.js`（统一 fetcher）
- 测试：`cargo test` workspace 649 passed 基线不退化

### 3.3 假设

- Assumption 1：用户机器具备 Rust 工具链（`rustc >= 1.75`，与 workspace edition=2021 兼容）。
- Assumption 2：3717 端口无占用；若占用则 xiaobai_voice 自动递增，网关代理支持通过配置改端口。
- Assumption 3：信创政务场景下，用户会自行下载 CosyVoice2 权重（Apache 2），Fish-S2-Pro 不作为默认 TTS。

---

## 4. 功能需求（Functional Requirements）

### 4.1 Rust 核心层（FR-CORE-n，mox-expert 新增模块）

| 编号 | 需求 | 说明 |
|---|---|---|
| FR-CORE-01 | 新增 `alliance` 模块：6 阶段管线编排器 | `mox-expert/src/alliance/mod.rs` 对外导出 `AllianceEngine::run_full_analysis(req) -> SSE Stream<AllianceEvent>`，6 阶段 = IntentClassify → TeamOptimize → ParallelDebate → Synthesize → QualityGate → SkillLearn。 |
| FR-CORE-02 | 意图分类（IntentClassify）：关键词 ms 级 + 激活扩散（HC-2）双路 + RRF 融合 | RRF k=60，spread_weight=0.7（HC-8 家族固定）；7 类意图 = {数学,逻辑,知识,代码,中文,时效,指令}（HC-9）；输出 intent_id + conf + seed_nodes[]。 |
| FR-CORE-03 | 专家组队（TeamOptimize）：能力注册表 × 意图匹配 × 安全类强制替换末位（EAF 4.2） | 注册 14 维专家到能力矩阵（permission/security 并列最高优先级 HC-SSOT DIM_PRIORITY=100）；若需安全类分析且末位专家非 Security，强制替换；输出 team = 3~5 位专家 id[] + 理由。 |
| FR-CORE-04 | 并行咨询（ParallelDebate）：真并行 rayon + 单专家 60s 超时隔离（EAF 4.3） + 900 tok/轮上限 | 调用 `expert::dispatch(ctx, experts)` 已实现 rayon 并行；新增 `tokio::time::timeout` 异步隔离；超时结果标记 `skipped=true + skip_reason="timeout_60s"`。 |
| FR-CORE-05 | 辩论合成（Synthesize）：共识 ≥0.6 跳过辩论 + 逐轮收敛检测 + 分歧结构化输出 | 共识度 = 同维度约束的一致向量 Jaccard；共识 <0.6 触发二次 LLM 辩论（若 ai-agent 可用否则降级为维度加权投票）；输出 consensus[] + divergence[] + final_plan。 |
| FR-CORE-06 | 质量门禁（QualityGate）：A/B/C/D 四级 + C 级单次重试闭环（EAF 4.5/4.6） | 评分公式严格按 HC-8：0.55Q+0.20S+0.10T+0.15Stab；等级阈值 A≥0.9, B≥0.8, C≥0.7, D<0.7；C 级重跑阶段 2~4，取更优；输出 gate_level + blockers[] + suggestions[]。 |
| FR-CORE-07 | 指标学习（SkillLearn）：新技能沉淀到 PluginRegistry + 语义缓存回填 | 若本次全维分析产物的 gate_level=A 且共识≥0.95，则把 pattern→action 对写入 `HarnessCtx` 的 skill_registry；并把 (intent_hash, answer) 写入语义缓存；输出 learned_skills[]。 |
| FR-CORE-08 | 归一化报告（ReconcileReport）：14 维观点矩阵 × DIM_PRIORITY 加权 × 冲突升级 | 复用 `reconcile.rs` 的现有归一化逻辑，但额外产出：人可读表格（每个维度的 score/risks/suggestions）、公式文本（HC-11：维度优先级表 + 密度解读）、Mermaid 全维分析流程图。 |
| FR-CORE-09 | 审计桥接：所有 6 阶段事件独立走 `audit::*` 汇，事件名 `alliance.01.intent_classified`…`alliance.07.skill_learned` 共 7 条 | 每条事件附 HMAC-SHA256 签名（MOX_AUDIT_SECRET）；安全类命中事件独立汇 1 份。 |

### 4.2 Platform 网关层（FR-GW-n，runtime 扩展）

| 编号 | 需求 | 说明 |
|---|---|---|
| FR-GW-01 | 新增路由 `/ai/engine/alliance/full`（POST）= 全维分析 SSE 流入口 | 对应 BP-05 总入口；请求体含 query/session_id/context/options；SSE 事件类型 = `{phase, payload, trace_id}`，事件顺序严格与 FR-CORE-01 的 6 阶段一致，末尾 `phase=done`。 |
| FR-GW-02 | 新增路由 `/ai/engine/alliance/capabilities`（GET）= 能力矩阵扩展 | 返回 14 位专家的 id/维度/支持的 7 类基准/平均延迟/近 30 日 gate_A 通过率；与现有 `/ai/engine/capabilities` 聚合但不覆盖。 |
| FR-GW-03 | 新增路由 `/ai/engine/alliance/report/:trace_id`（GET）= 报告追溯 | trace_id 作为幂等查询 key，返回归一化报告 + 全阶段审计摘要；未命中 404。 |
| FR-GW-04 | 网关代理 `/voice/*` → 3717 xiaobai_voice FastAPI | `/voice/health`、`/voice/asr/full`（POST multipart audio）、`/voice/tts/stream`（GET ?text=&voice=）、`/voice/ws/asr/stream`（WebSocket 代理）；sidecar 不可用时返回 `degraded=true` + 降级说明（如"请启动 xiaobai 服务"）。 |
| FR-GW-05 | `/ai/engine/metrics` 扩展输出：全维分析统计字段 | 新增字段：`alliance_total_runs`、`alliance_gate_dist{A,B,C,D}`、`alliance_phase_p95_ms{intent,team,debate,synth,gate,learn}`、`intent_distribution{7类}`、`learned_skills_count`；prometheus 指标通过 `AiEngineState` 的 AtomicU64 累加。 |
| FR-GW-06 | RBAC 中间件全覆盖：所有新路由需 `auth_header` 校验 | `rbac_middleware.rs` 已存在；新路由挂到 `Router::new().route(...).layer(rbac_layer)`；未授权 401 + `audit: alliance.unauthorized` 事件签名。 |
| FR-GW-07 | `subservers.rs` 新增 AllianceSubserver：所有 `/ai/engine/alliance/*` 与 `/voice/*` 在此注册 | 满足 §六 HC-2（跨 crate 引用必须经 subservers 注册）。 |

### 4.3 前端交互层（FR-FE-n，ChatView + MessageBubble + API）

| 编号 | 需求 | 说明 |
|---|---|---|
| FR-FE-01 | ChatView 顶栏「全维分析 φ」按钮点击触发真实流程 | 调用 `/ai/engine/alliance/full` SSE；收到 phase 事件同步更新 5 Chip（现有 Chip 扩展：intent→team→debate→synthesize→gate 与 stage 标签对齐）；Chip 激活态按现有 CSS `.stage-chip.active`。 |
| FR-FE-02 | ChatView 输入框新增麦克风按钮（录音 ASR） | 左起第 1 个操作按钮，图标 Mic；三态：idle / recording(动画) / processing；按住说话（长按）或点击切换（短按切换）；录音完成后调 `/voice/asr/full` 转文字并注入输入框。 |
| FR-FE-03 | ChatView 输入框新增小白语音开关 | 浮窗状态图标：关闭（灰）/ 在线（绿）/ 掉线（红）；点击打开桌面浮窗启动引导弹窗（首次使用说明）。 |
| FR-FE-04 | MessageBubble 朗读按钮（已存在）三层回退：`/voice/tts/stream` → Web Speech Synthesis → 禁用 | 与现有 9 动作工具栏的"朗读"按钮集成；优先网关代理 Rust→xiaobai_voice；失败自动回退浏览器原生；播放状态显示进度条。 |
| FR-FE-05 | 全维分析完成后自动插入「全维分析报告」消息卡片 | 卡片结构：标题（含 gate_level 徽标 A/B/C/D）+ 6 阶段完成列表 + 14 维观点折叠面板（每维 score 进度条 + risks/suggestions）+ Mermaid 流程图（可展开）+ 一键导出 JSON 按钮 + φ 复制报告 Markdown 按钮。 |
| FR-FE-06 | 前端 API 层统一封装：`frontend-ui/src/api/index.js` 新增 `allianceApi` + `voiceApi` | `allianceApi.fullAnalysisSse(query, onPhase, onDone, onErr)` / `voiceApi.asrFull(file)` / `voiceApi.ttsStream(text)` / `voiceApi.health()`；代理统一走 `import.meta.env.VITE_API_BASE || '/api'`；dev 模式 Vite 代理 `/api/voice/*` → `http://localhost:3717`。 |
| FR-FE-07 | ChatView 新增空态快捷问法：「启动全维分析示例」卡片（3×2 已有 Grid 内新增一张） | 点击注入：「我想做一个 Rust 企业级服务，请做全维分析」+ 自动触发全维流程（用于验收探针一键演示）。 |

### 4.4 语音服务集成（FR-VOICE-n，xiaobai_voice 闭合）

| 编号 | 需求 | 说明 |
|---|---|---|
| FR-VOICE-01 | xiaobai_voice FastAPI 启动冒烟：`/voice/health` 返回 OK + 子系统状态 | 结构：`{ok, asr:{ready, model, backend}, tts:{ready, engines:[Fish|CosyVoice|Browser], active}, endpoints:{asr_full, tts_stream, ws_asr_stream}}`。 |
| FR-VOICE-02 | ASR: 16kHz 16bit PCM 或 mp3/wav 上传 → Paraformer-zh sherpa-onnx 识别 | 若 sherpa-onnx 未初始化：返回 `degraded=true + "请下载模型：xiaobai models download asr"`；endpoint 对 multipart/form-data audio 字段正确解析。 |
| FR-VOICE-03 | TTS 引擎选择：**默认 CosyVoice2（Apache 2 协议）**，Fish-S2-Pro 需手动 enable | 避免 Research License 默认激活风险；配置 `default_config.yaml::tts::default_engine = cosyvoice2`；fish_s2 在 `engines_available` 中标记 `license: "Research"`。 |
| FR-VOICE-04 | Gateway → 3717 代理：CORS 正确 + 流式 TTS 响应头 Content-Type:audio/wav 透传 | SSE 全维分析的流式不阻塞；TTS HTTP 流必须 chunked。 |
| FR-VOICE-05 | 桌面小白浮窗（ball_widget.py）启动后与平台 session_id 共享 | 浮窗右键菜单新增"打开璇玑 AI 对话"，用系统默认浏览器打开 `http://localhost:3021/#/ai?session_id=<uuid>`，二者共享同一后端 session。 |

---

## 5. 非功能需求（Non-Functional Requirements）

| 编号 | 维度 | 需求 | 验收度量 |
|---|---|---|---|
| NFR-01 | 分层合规（HC-分层） | Controller(runtime routes) → Service(mox-expert::alliance) → Repo(图谱/审计) 零越层；禁止 handler 内写业务逻辑 | `grep -rn "fn analyze\|dispatch\|reconcile" platform/gateway/runtime/src/handlers/` 命中 = 0（只做参数校验 + 调 service） |
| NFR-02 | Rust 安全护栏（HC-14） | 新代码 `unsafe` 块计数 = 0；`cargo deny -L error` 全通过；无 C 绑定重依赖 | `grep -rn "unsafe " platform/services/mox-expert/src/alliance/` = 0 |
| NFR-03 | 可观测性 | 6 阶段每个阶段开始/结束都有 `trace_event!(stage, phase, latency_ms, trace_id)`；prometheus 指标 100% 对应 metrics 端点输出 | metrics 端点字段总数 ≥ 18；trace 日志字段齐全率 = 100% |
| NFR-04 | 性能基线（60 场景） | 空查询（纯本地无 AI）全维分析 P50 ≤ 400ms；P95 ≤ 1.5s；14 专家并行 rayon 加速比 ≥ 3.5×（对比串行） | `cargo bench -p mox-expert --bench alliance_full` 基准结果 |
| NFR-05 | 幂等性 | 同一 `{query, session_id, idempotency_key}` 的连续 2 次调用返回相同 trace_id；不重复写审计汇、不重复学技能 | 手动 2 次 curl 对比 response.trace_id（相同）；审计事件计数（≤ 1 次） |
| NFR-06 | 降级链完整性 | ai-agent 不可用 → 辩论降级为维度加权投票；graph-algo 不可用 → 意图降级为纯关键词；xiaobai 不可用 → 语音降级为浏览器 TTS/Mic API | 每条降级链在 CI 有 `degraded=true` 测试用例 |
| NFR-07 | 企业级稳定性 | 1000 次全维分析调用（含错误输入/空查询/超时）→ 进程不崩溃（0 panic）；内存泄漏率 ≤ 5MB/1000 次 | `cargo test -p mox-expert --test alliance_stress -- --test-threads=1 --ignored` |
| NFR-08 | 前端零 console error/warning | Chrome DevTools 控制台 0 红 0 黄（验收时 Playwright 采集） | Playwright E2E 扫 `console.error` / `console.warn` 计数 = 0 |
| NFR-09 | 响应式 & 黄金比例 | 全维分析卡片、Chip 指示器、语音按钮在 ≤720px 屏自动折叠；间距/尺寸严格遵循 4/6/10/16/26/42 黄金序列 | CSS 规则审查：间距值仅允许集合内成员 |
| NFR-10 | 安全合规 | 所有 `/ai/engine/alliance/*` + `/voice/*` 经过 RBAC；ASR 音频不落盘（除非用户点保存）；TTS 文本不留明文日志（脱敏 *处理） | RBAC 401 测试；日志 grep 音频路径；敏感词脱敏检查 |

---

## 6. 验收标准（Acceptance Criteria，rule / rubric 二选一）

### Rule 类（客观可验证，0/1 通过）

| AC 编号 | 条件 | 证据来源 |
|---|---|---|
| AC-01 | `cargo build -p mox-expert -p runtime` 编译通过，0 warning（deny warnings） | CI 构建日志 |
| AC-02 | `cargo test -p mox-expert alliance_ --nocapture` 全通过（≥ 25 测试：6 阶段 + 降级 + 幂等 + 审计 + 安全） | cargo test 报告 |
| AC-03 | `cargo test -p runtime ai_engine_alliance_` 集成测试全通过（≥ 15 测试：4 新端点 + SSE + RBAC + 代理） | cargo test 报告 |
| AC-04 | `GET /ai/engine/capabilities` 返回 7 类基准全部注册；`GET /ai/engine/alliance/capabilities` 返回 14 专家条目 | curl / 浏览器直接访问 JSON |
| AC-05 | `POST /ai/engine/alliance/full` 返回 SSE 7 个事件（6 phase + 1 done）；事件 trace_id 全局唯一；phase 值严格按序 = intent→team→debate→synthesize→gate→learn→done | SSE 事件流抓包 |
| AC-06 | 质量门禁输出 gate_level 为 A/B/C/D 之一；若 gate=C，response 中含 `retried: true`（重跑闭环） | 用构造触发 C 级的输入（如模糊 query）手动验证 |
| AC-07 | 激活扩散参数：对典型 query，在 alliance trace log 中 grep 到 `method=spread, d=0.85, rounds=30`（HC-2 固定值） | trace 日志采集 |
| AC-08 | RRF 融合：日志含 `rrf_k=60, spread_weight=0.7`（HC-8 家族） | trace 日志 |
| AC-09 | 评估统一公式在报告中的 explanation 字段以原文 `0.55×Quality + 0.20×Speed + 0.10×TokenEfficiency + 0.15×Stability` 出现（HC-8 不可换权重） | report JSON 字段 |
| AC-10 | `/voice/health` 通过网关代理返回 OK；degraded 模式下返回 `degraded=true` 而非 502/超时 | curl http://localhost:3717 开/关 两种场景 |
| AC-11 | ChatView 麦克风按钮录音→ASR 成功；输入框自动填入识别文字 | Playwright E2E 脚本（或人工验收：录音 3 秒中文"你好"→填入"你好"） |
| AC-12 | MessageBubble 朗读按钮三层回退工作：关闭 xiaobai 时自动使用浏览器 TTS，不阻塞 UI | 人工验证：关闭 3717 → 点朗读 → 浏览器播音 |
| AC-13 | 全维分析消息卡片含 14 维折叠面板（每维有 score 进度条）；Mermaid 图可见（非 0×0）；复制报告按钮能复制 Markdown 到剪贴板 | DOM 探针 + 复制 API 验证（Playwright） |
| AC-14 | 5 Chip 指示器流转正确：全维分析期间依次点亮 intent→team→debate→synthesize→gate（对应 AC-05 事件） | DOM 探针 class .stage-chip.active 顺序记录 |
| AC-15 | RBAC：无 Authorization Header 调 `/ai/engine/alliance/full` 返回 401；错误 token 返回 403；合法 token 200 | curl 三组调用 |
| AC-16 | 审计链：`grep -rn "alliance.0[1-7]." audit.log` 命中 ≥ 7 条（每个阶段 1 条）；每条有 HMAC signature 字段 | 运行 1 次全维分析后扫日志 |
| AC-17 | `/ai/engine/metrics` 输出 alliance_gate_dist 对象含 A/B/C/D 四个键；各键为非负整数 | metrics JSON 结构检查 |
| AC-18 | Rust `unsafe` 计数：`grep -rc "unsafe " platform/services/mox-expert/src/alliance/` = 0 | 代码扫描 |
| AC-19 | 前端零 console 警告/错误：Playwright 跑 ChatView + 全维分析 + 语音按钮，采集 console 计数 = 0 | Playwright E2E 报告 |
| AC-20 | 前端语音按钮 + 全维分析按钮布局符合黄金比例 4/6/10/16/26/42 间距序列；非序列值 = 0 | CSS grep 扫描间距数值白名单 |
| AC-21 | xiaobai_voice TTS 默认引擎 cosyvoice2：`cat xiaobai_voice/config/default_config.yaml | grep default_engine:` 命中 `cosyvoice2`（避免 Research License 默认激活） | 文件内容 |
| AC-22 | `subservers.rs` 已注册 AllianceSubserver + VoiceSubserver | grep `AllianceSubserver\|VoiceSubserver` 存在于 subservers.rs |

### Rubric 类（评估质量，阈值通过）

| AC 编号 | 维度 | 刻度 | 阈值 | 证据来源 |
|---|---|---|---|---|
| AC-R01 | 全维分析报告结构化质量 | 2=14维齐全+公式说明+Mermaid流程+导出按钮齐全且无截断；1=缺1项；0=缺≥2项 | ≥1 | 人工审阅 + DOM 探针 |
| AC-R02 | SSE 6 阶段 UI 流动流畅度 | 2=Chip 过渡≤800ms/阶段，无卡顿闪烁；1=≤1.5s/阶段；0=≥2s 或阶段顺序错乱 | ≥1 | Playwright 录制时间戳 |
| AC-R03 | 语音 ASR 中文识别准确率（3 字常用词） | 2=≥95%；1=≥80%；0=<80%（可接受 Browser fallback 降级提示清晰） | ≥1 | 人工验收："你好璇玑"→"我要做全维分析"→"请开始" 三句 |
| AC-R04 | 前端 UI 美感（深空色系统一度） | 2=所有新增组件色值与 global.css 设计令牌完全一致；1=1~2 处微偏差；0=≥3 处明显异色 | ≥1 | DOM 探针 computed color 集合 |
| AC-R05 | 后端 Rust 代码整洁度（AIS 规范） | 2=pub fn 100% 含文档注释；1=≥90%；0=<90% | ≥1 | `cargo doc --no-deps` 警告数（越少越好） |
| AC-R06 | 可观测完整性（trace 贯穿） | 2=全 6 阶段 trace_id 从前端请求头 → SSE → 日志 → 审计汇 全链路一致；1=日志缺 1 个阶段；0=缺≥2 或断裂 | ≥1 | grep trace_id 跨文件 |
| AC-R07 | 649 passed 基线不退化 | 2=`cargo test --workspace` 全通过且 passed=649±1；1=passed≥640；0=<640（Blocking 退化，打回） | ≥1 | cargo test 最终摘要 |

---

## 7. 开放问题（Open Questions，实施前需澄清）

| Q # | 问题 | 暂定方案（默认） | 期望澄清日 |
|---|---|---|---|
| Q1 | 是否需要把 xiaobai_voice 的 FastAPI 服务纳入 platform 统一进程管理（如 runtime 启动时 subprocess 拉起）？ | 默认 NO：用户手动 `xiaobai run` 启动，网关代理按需连接，不可用时返回 degraded 提示。远期可纳入。 | 2026-08-26 |
| Q2 | 全维分析是否需要真实 LLM 辩论？（当前 Node 实现会调大模型辩论 900tok/轮，成本高） | 默认 NO：Phase 1 用纯本地维度加权投票模拟辩论输出，若 `options.enable_llm_debate=true` 才走 ai-agent LLM，保证 CI 可离线全通过。 | 2026-08-26 |
| Q3 | ChatView 的语音 UI 是否默认开启？（部分用户可能不需要麦克风权限弹窗） | 默认 YES（首次使用浏览器会请求权限，用户可拒绝后按钮仍可用但提示"请授权麦克风"）；可在设置中关闭。 | 2026-08-26 |

---

> **变更留痕**：v1.0 ENT · 2026-08-25 · 首次发布 · 依据 TOP-MASTER §四 BP-05 + §六 HC-1~HC-14 + 用户「开发专家联盟全维分析集成Platform+Rust+语音闭合」诉求。
