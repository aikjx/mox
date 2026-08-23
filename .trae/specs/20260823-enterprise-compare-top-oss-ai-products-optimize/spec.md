# Spec：开源顶级 AI 产品架构对比 + 试验证 + 企业级优化（璇玑 Infotopograph v3.1）
> 日期：2026-08-23 · 模式：Spec Mode · 语言：中文
> 背景文档（L0 权威链，本 spec 不与其冲突）：`docs/enterprise/18 TOP-MASTER/*`
> 前置企业级报告（T9 已通过，17 AC 100%）：`.trae/specs/20260823-enterprise-ready-build-verify/T9-enterprise-acceptance-report.md`

---

## 一、问题背景（Problem）

用户提出："与全部开源/最好 AI 产品一一对比架构 → 分析 → 出试验证 → 对比后再看怎么优化（企业级）"。

本仓库已经交付了一个六层架构（L5 前端 / L4 网关 / L3 16 Rust 微服务 crate / L2 图谱算法 / L1 存储 / L0 基线）的企业级 AI 中台，但其目前**缺少与顶级开源 AI 产品在企业级硬指标（高并发、高可用、多租户隔离、插件沙箱、LLM 路由策略、工作流编排引擎、多模态嵌入检索、RAG 冷启动、Operator/Agent 平台可扩展性、可观测性/追踪、SLO 可视化）上的系统性对照**，也缺少**真实基线试验证（benchmark harness）与优化落地代码**。

本 spec 目标是：**以 4 个顶级开源 AI 产品作为对照基线，产出（1）对照矩阵（2）企业级差距分析（3）可重跑的实验 harness & 基线报告（4）≥ 8 项企业级可落地优化 patch，均通过新试验**，让璇玑 3.1 版本在客观指标上与/或超越顶级开源产品。

---

## 二、用户（Users）& 目标（Goals）

### 2.1 用户画像
| 角色 | 关注点 |
|---|---|
| **架构师 / 技术 VP** | 架构差异、可扩展性、SLA/SLO、企业可落地性 |
| **SRE / 运维** | 高可用、隔离、限流、熔断、负载均衡、可观测 |
| **平台工程** | 插件/算子/工具的安全沙箱、版本化、注册表治理 |
| **业务开发者** | 工作流编排、低代码、RAG 管道、多模态能力 |
| **AI 应用终端用户** | 响应延迟 P50/P95/P99、首 token 延迟、稳定性 |

### 2.2 目标（Goals）
1. 建立与 4 个"顶级开源 AI 产品"的架构对照矩阵（≥ 18 维度）。
2. 对每一项差异进行企业级影响评估（Critical/High/Medium/Low）。
3. 编写并运行 ≥ 4 类企业级 benchmark harness（高并发、熔断、插件沙箱延迟、LLM 路由策略），输出基线对比数据 CSV。
4. 落地 ≥ 8 项企业级补丁（高优 ≥ 5 项），每个补丁有独立的"前/后"基准对比。
5. 优化后，**SLA 指标整体相对基线提升 ≥ 15%（平均），P99 延迟相对基线下降 ≥ 20%**。

### 2.3 非目标（Non-Goals）
- 不修改 TOP-MASTER 文档链的现有声明式结论（L0 文档保持最高权威，本 spec 仅追加 T10 企业级优化证据包）。
- 不做 UI/UX 前端改版（前端 build 0 错误保持不变）。
- 不替换现有 16 Rust crate / Node API 的主入口契约，只做增量 / 增强。
- 不对比闭源产品（GPTs Store、Claude Artifacts 等）。

---

## 三、对比对象（4 + 1 开源"顶级 AI 产品"）

以 **GitHub star ≥ 10k、企业级落地有真实案例、且与璇玑在能力域至少重叠 ≥ 5 项** 为入选标准：

| 编号 | 产品（开源协议） | GitHub stars（2026 中估计） | 与璇玑的重叠能力域 |
|---|---|---|---|
| **P1** | [Dify v0.14+](https://github.com/langgenius/dify)（MIT） | ~45k | 工作流编排、RAG、LLM 网关、插件、RBAC、多模型路由、多租户 |
| **P2** | [LangGraph Cloud OSS + LangGraph Studio](https://github.com/langchain-ai/langgraph)（MIT）| ~28k | Agent 图编排、检查点 / 持久化、Human-in-the-Loop、状态图、循环 |
| **P3** | [Flowise AI](https://github.com/FlowiseAI/Flowise)（MIT）| ~38k | 低代码画布、LangChain 工具生态、向量数据库连接器、RAG 管道 |
| **P4** | [AutoGen + AutoGen Studio](https://github.com/microsoft/autogen)（MIT）| ~32k | 多智能体协作、对话驱动编排、代码解释器、工具调用、群组角色 |

**璇玑本身**（本文档被测对象，对应 "P0 Infotopograph"）：16 Rust crate + Node Atlas + 专家联盟 + 7×8 算法 + Wasm 算子沙箱 + 项目全息图谱。

---

## 四、对比维度（18 个企业级硬维度）

> 每条维度都在 §六 AC 中被钉死为 rule / rubric，保证最终有可观察证据。

| 维度编号 | 维度名称 | 说明 | 测量方法 |
|---|---|---|---|
| D01 | **微服务 / 模块拆分** | crate / package / service 数与边界清晰度 | 独立可部署 crate 计数 / pkg 依赖环数 |
| D02 | **语言策略（Polyglot vs Monoglot）** | Rust/Node/Python 混合 vs 纯 JS | 代码行数分布 + 关键性能路径语言 |
| D03 | **插件 / 算子沙箱** | 未信代码隔离 / 内存配额 / syscall 限制 | 是否有 Wasm / WASI / V8 隔离（带可测基准） |
| D04 | **多租户隔离模型** | 行级 / 表级 / DB 级 / 命名空间 | security.js 隔离深度 + RBAC 审计覆盖率 |
| D05 | **LLM 路由策略** | 优先级 / 熔断 / 降级 / 负载均衡 / 语义路由 | 路由决策数 + P99 延迟 / 失败回退率 |
| D06 | **限流 & 熔断（治理）** | 漏桶 / 令牌桶 / 半开 / 百分比熔断 | 真实 200 并发下通过请求率 / 被拒比率 |
| D07 | **工作流编排引擎** | 节点类型、并发扇出、检查点、回滚、分支 | 节点类型数 × 并发扇出上限 × P99 调度耗时 |
| D08 | **Agent 协作模式** | 多代理 / 角色 / 投票 / 辩论 / 判决 | 专家联盟形态 × 可切换策略数 |
| D09 | **RAG / 多模态检索** | 文档切分 / embedding / rerank / multimodal 支持 | 1000 docs Recall@10 + 索引构建 QPS |
| D10 | **图谱算法与项目全息** | 原生知识图谱 vs 外部 Neo4j 代理 | 算法数 × 图顶点规模 × 7 算法 Δ 一致性 |
| D11 | **可观测性 / 追踪** | OpenTelemetry / Trace / Span / 指标面板 | Span 覆盖率、p99 trace 完整度 |
| D12 | **SLO / SLA 可视化看板** | 成功率、P50/P95/P99 延迟、可用性 | 指标可获取性、/system/slo 是否存在 |
| D13 | **RBAC & 审计闭环** | 角色层级、审计日志完整性 | 审计日志完整性（覆盖写/读/管理请求） |
| D14 | **冷启动 / 热路径** | Serverless / worker pool / 预热 | 首次请求延迟 vs 预热后延迟 |
| D15 | **版本化 & 市场** | 算子/工作流/应用的版本化治理 | 市场 package count + upgrade/downgrade 可回滚 |
| D16 | **可扩展性（插件生态接口）** | 扩展点 / 生命周期钩子 / SPI | 扩展点数量 + 官方插件加载失败率 |
| D17 | **部署 / 运维** | Docker Compose / Helm / Operator / K8s | 官方 compose / helm 完整性（1=齐全，0=缺失） |
| D18 | **开源协议 & 商业友好** | 许可证、企业功能是否开源 | license 文件 + 企业功能开放比例 |

---

## 五、功能需求（Functional Requirements）

### 5.1 对比产出
FR1：必须生成一份 `对比矩阵 P0×P1×P2×P3×P4 × D01~D18` 的可机器读 JSON + 人类读 MD 两份。
FR2：必须针对矩阵中 **P0 得分低于任何 P1~P4** 的维度给出差距分级（Critical/High/Medium/Low），并给出优化候选（每一个高分差异点 ≥ 1 条候选）。
FR3：必须把差异与"§四 D 维度"对齐，每个差异都带一份对企业落地的风险说明。

### 5.2 实验产出
FR4：必须编写 ≥ 4 类 benchmark harness（真实跑命令，非模拟）：
  - T-H1：高并发（200 req/s）下 `security` 限流 + `llm-gateway` 路由；
  - T-H2：LLM 路由（priority/fallback/latency-based）P50/P95/P99 延迟；
  - T-H3：Wasm 算子沙箱 vs 原生算子 / 非沙箱算子 1000 次调用的延迟 & 内存 & 错误率；
  - T-H4：专家联盟（7 专家并行 × 4 组并行请求）CPU 利用率 & 吞吐量。
FR5：每个 harness 必须产出 `*.csv` 原始数据 + 汇总 `*.md` 表格。

### 5.3 优化落地
FR6：必须落地 ≥ 8 项企业级优化补丁（Rust/Node 任一层均可，与优化候选直接对应），每一项：
  - 有独立的 before / after harness 数据（≥ 1 条 harness CSV）；
  - 有对应新增单元 / 集成测试；
  - 通过 `cargo clippy --workspace --all-targets -- -D warnings` 与 `mocha 126 + n 新用例` GREEN。
FR7：高优补丁 ≥ 5 项（对应 Critical/High 差异），范围至少覆盖：
  - O1：**LLM 路由新增 Latency-WARM（加权平均路由 + 预热）**（D05）；
  - O2：**并发令牌桶（Token Bucket）+ 租户级 qps 配额**（D06，取代 security.js 当前简单滑动窗口）；
  - O3：**Wasm 算子沙箱增加 CPU 指令预算 + 内存硬上限 trap + 观测 hook**（D03）；
  - O4：**SLO 看板接口 /system/slo 与 SLO JSON 输出**（D11/D12）；
  - O5：**工作流编排节点"并发扇出 + 取消传播"增强**（D07，ai-agent flow_engine）。
FR8：所有优化必须有"企业级回滚方案"（feature flag 即可）。

---

## 六、非功能需求（Non-Functional Requirements）

| 编号 | 要求 | 测量 |
|---|---|---|
| NFR1 | **性能**：O1~O5 高优补丁上线后，平均 P99 延迟下降 ≥ 20% | H1/H2 前后 CSV 对比 |
| NFR2 | **稳定性**：H1 200 并发 × 60s，成功率 ≥ 99.5%（无崩溃 / 无未捕获异常） | 进程 exit 0 + 成功率统计 |
| NFR3 | **隔离性**：H3 Wasm 沙箱 1000 次恶意调用（死循环 / OOM 伪造）全部被捕获，宿主内存增长 < 5% | 进程 RSS 监控 + 错误分类计数 |
| NFR4 | **兼容性**：所有优化不得破坏现有 16 Rust crate 工作空间 0 Clippy 与 126 Mocha GREEN | Clippy 与 mocha 实跑 |
| NFR5 | **可观测**：每一项优化必须 emit 至少 1 个新指标（latency_ms / error_count / tokens） | `/system/slo` 返回 ≥ N+NFR 个指标 |
| NFR6 | **安全性**：新增配置不得引入明文密钥；所有 token 仍旧走 `config.js` 已加密通道 | grep "sk-" / "api_key:" 0 命中 |

---

## 七、约束 / 依赖 / 假设 / 待澄清

### 7.1 约束（Constraints）
C1：必须在现有 `Cargo.toml` 16 members 与 `backend-node/package.json` 基础上做补丁，新增 crate 需有强理由（尽量复用）。
C2：不得引入需要真实外部 LLM API Key 才能运行的测试 / 实验；**所有 H1~H4 必须提供 mock provider**（已经内置 `LocalEngine` 作为 mock）。
C3：所有新增代码必须通过 Clippy `-D warnings`。
C4：不得与 L0 TOP-MASTER 的 §二~§八声明冲突。

### 7.2 依赖（Dependencies）
- 已存在：`wasmer` / `wasmer-compiler-cranelift`（operator-wasm）、`tokio`（runtime）、`rayon`（xuanji-expert）、`express`（backend-node）。
- 可能新增：独立 benchmark harness 仅使用 Node 原生 + 少量 Rust 基准（避免新增重型第三方依赖）。

### 7.3 假设（Assumptions）
A1：P1~P4 产品的 18 维度数据来自公开文档（GitHub README、官方 docs、architecture.md），若与最新代码有偏差，以公开文档为准；本 spec 不构建真实 Dify/LangGraph 运行环境做基准（否则需要 Docker+K8s，超出本仓库可执行范围）。
A2：璇玑的对照 P0 维度得分以实跑证据（H1~H4 数据 + 代码审计）为准。

### 7.4 待澄清（Open Questions — 立即给用户选项）
- OQ1：是否需要 **额外把 P1 Dify 的 `workflow orchestrator` 做一次"真实功能对齐"（含 For-Loop / Switch / LLM / Knowledge / Tool / If-Else / Start / End 8 节点类型）**？（工作量大，可在 V3.2 做）
- OQ2：是否需要 **D09 RAG 深度对比 + 1000 文档 Recall@10 真实实验**？（需要准备一份 1000 doc 合成数据集；可在 V3.2）
- OQ3：是否需要 **D17 部署运维补齐 `docker-compose.yml + helm` 真的交付？**（可在 V3.2，当前非阻塞）

---

## 八、验收标准（Acceptance Criteria）—— AC 共 24 条（16 rule + 8 rubric）

### 8.1 Rule AC（可客观观察的二元条件）
| ID | 描述 | 证据来源 |
|---|---|---|
| **AC-01** | 存在 `T10-comparison-matrix.md` 与 `T10-comparison-matrix.json`，覆盖 P0~P4 × D01~D18 = 5×18 = 90 格，无空值 | `docs/enterprise/` 或 `.trae/specs/.../` 下的文件检查，JSON parse 成功 |
| **AC-02** | 对比矩阵中，P0 每个维度都给出数值评分（0~100，整数），P1~P4 亦然 | JSON schema 校验，5×18=90 个整数分数 |
| **AC-03** | 存在 `T10-gap-analysis.md`，对 **P0 得分 < 任何 P1~P4 最高得分** 的维度给出 Critical/High/Medium/Low 差距分级，分级条目数 ≥ 4 | 文件存在 + 条目计数 |
| **AC-04** | H1 高并发 harness：代码存在（`test/bench_governance_concurrency.js` 或等价 Rust 基准），可独立运行 exit=0，生成 concurrency.csv | 文件存在 + 实跑 exit=0 |
| **AC-05** | H2 LLM 路由 harness：代码存在（`test/bench_llm_routing_strategies.js`），运行 exit=0，生成 routing.csv，策略 ≥ 3 种（priority/fallback/latency-warm）| 文件存在 + 实跑 + CSV 行数 ≥ 3 × 统计指标行数 |
| **AC-06** | H3 Wasm 沙箱 harness：代码存在（`platform/services/operator-wasm/tests/bench_sandbox.rs` 或等价），exit=0，生成 sandbox.csv，包含内存/延迟/错误率三指标 | 文件存在 + 实跑 + CSV 列检查 |
| **AC-07** | H4 专家联盟并发 harness：代码存在（`platform/services/xuanji-expert/tests/bench_alliance_concurrency.rs` 或等价），exit=0，生成 alliance.csv | 文件存在 + 实跑 exit=0 |
| **AC-08** | O1 补丁：`llm-gateway.js` 新增 `LatencyWarm` 路由策略（含预热 + 滑动窗口 EWMA 延迟），并具备单元测试 GREEN | `git diff` 特征检查 + `mocha` 新用例 GREEN |
| **AC-09** | O2 补丁：`security.js` 新增 **令牌桶（Token Bucket）** 限流，支持租户级 QPS 配额；`security._bench` 自测通过 | 新 class / 函数存在 + 自测 GREEN |
| **AC-10** | O3 补丁：`operator-wasm` 新增 CPU 指令预算（fuel）+ 内存硬上限 trap，`#[test]` GREEN（含恶意字节码被终止 2 条） | `WasmOperator::with_fuel()` 或等价 + 2 条 test GREEN |
| **AC-11** | O4 补丁：`routes/system.js` 新增 `GET /system/slo` 接口，返回 JSON 至少包含 `p50_ms/p95_ms/p99_ms/success_rate/error_count/route_count` | HTTP 测试（supertest / 直接函数调用）JSON schema ok |
| **AC-12** | O5 补丁：`ai-agent` flow_engine / workflow_engine 增加 **并发扇出 `ParallelNode` + 取消传播 `CancellationToken`**，单元测试 GREEN（并行数=8，取消 5s 内完成）| Rust `#[test]` 2 条 GREEN |
| **AC-13** | 8 项优化全部具备 feature flag（默认开启），可通过环境变量全局关闭 | `process.env.DISABLE_OPTIM_*` 或等价被检查 |
| **AC-14** | 优化后 `cargo clippy --workspace --all-targets -- -D warnings` exit=0（零告警保留） | 实跑 |
| **AC-15** | 优化后 Node Mocha（三原套件 + 新 harness 自测）GREEN，总数 ≥ 126 + 20 = 146 | 实跑 `mocha test\mocha_*.js test\bench_*.js --grep ...` passes ≥ 146 |
| **AC-16** | 存在 `T10-harness-summary.md`，列出所有 H1~H4 基线数据及 O1~O5 before-after 数据 | 文件存在 + before-after 配对 |

### 8.2 Rubric AC（评估型质量维度）
| ID | 维度 | 0-100 分刻度 | 通过阈值 |
|---|---|---|---|
| **AC-17** | NFR1 延迟改善（平均 P99 下降率） | 0 = 无改善 / 50 = 10% / 80 = 20% / 100 = ≥ 30% | ≥ 80 |
| **AC-18** | NFR2 稳定性（H1 200 并发 60s 成功率） | 0 = 崩溃 / 50 = 99% / 80 = 99.5% / 100 = ≥ 99.9% | ≥ 80 |
| **AC-19** | NFR3 隔离性（H3 恶意样本 1000 次全部捕获 + 内存 < 5% 增长） | 0 = 宿主崩 / 50 = 95% 捕获 / 80 = 100% 捕获 / 100 = 100%+零增长 | ≥ 80 |
| **AC-20** | 架构对照深度（D01~D18 每维度说明详实度） | 0 = 只有分数 / 50 = 有 3 句 / 80 = 有对照+差异 / 100 = 带企业风险评估 | ≥ 80 |
| **AC-21** | 优化 patch 的工程完整性（flag + test + doc + metric 四项齐全率） | 0 = 裸代码 / 50 = 2/4 / 80 = 3/4 / 100 = 4/4 齐全 | ≥ 80 |
| **AC-22** | 与 P0 现有架构的侵入性（低侵入好） | 0 = 改 16 crate 主契约 / 50 = 改 ≤ 10 个入口 / 80 = 改 ≤ 5 入口 + 纯增量 / 100 = 纯增量 0 破坏 | ≥ 80 |
| **AC-23** | Benchmark Harness 的可重跑性（任何开发者 `node xx.js` / `cargo test --bench` 都能出 CSV） | 0 = 缺 mock 不能跑 / 50 = 需 key / 80 = 全 mock / 100 = 全 mock + 固定 seed 确定性 | ≥ 80 |
| **AC-24** | 企业级可操作性（是否有一键 `T10-replay-all.ps1` 生成所有 JSON/CSV/MD） | 0 = 手工粘命令 / 50 = 3 脚本 / 80 = 1 脚本 / 100 = 1 脚本 + 失败自动退出 + 汇总 | ≥ 80 |

---

## 九、AC-TR 映射说明（留给 Plan 阶段）

Plan 阶段将把 AC-01~AC-24 拆解为至少 9 个任务：
  - T1 对比矩阵（AC-01~03）
  - T2 H1 并发治理 harness + before 数据（AC-04 / NFR2）
  - T3 H2 LLM 路由 harness（AC-05）
  - T4 H3 Wasm 沙箱 harness（AC-06 / NFR3）
  - T5 H4 专家联盟并发 harness（AC-07）
  - T6 O1 LLM LatencyWarm 路由（AC-08 / NFR1）
  - T7 O2 Token Bucket 限流（AC-09 / NFR2）
  - T8 O3 Wasm fuel + mem 上限（AC-10 / NFR3）
  - T9 O4 /system/slo 接口（AC-11）
  - T10 O5 Flow 并发扇出 + CancellationToken（AC-12）
  - T11 8+ 补丁 feature flag + 自测补齐 + after 数据（AC-13/14/15/16）
  - T12 T10-replay-all.ps1 一键脚本（AC-24）
  - Review：独立 review（rubric 17-24 复核 + rule 复跑）
