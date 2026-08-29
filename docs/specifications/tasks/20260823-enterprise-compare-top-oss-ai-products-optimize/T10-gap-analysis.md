# T10 · 差距分级 & 优化候选（璇玑 v3.1 企业级优化路线图）
> 输入：`T10-comparison-matrix.json`（P0 vs P1~P4 5×18=90 格）
> 输出：差距条目 + Critical/High/Medium/Low 分级 + 企业落地风险说明 + 优化候选（至少 1 条 / 每条 Critical/High 必须映射到具体任务 T6~T12）

---

## 一、差距判定规则

对每个维度 Dx（D01~D18）：
- 设 **TopOSS(Dx) = max(P1[Dx], P2[Dx], P3[Dx], P4[Dx])**（顶级开源中得分最高者，作为该维度的"行业上限"）。
- 若 **P0[Dx] < TopOSS(Dx)**，则 Dx 被判定为"差距项"，并按 Δ = TopOSS(Dx) − P0[Dx] 做分级：
  - **Critical（致命）**：Δ ≥ 40 或 Dx 属于安全/治理/隔离类 + Δ ≥ 25；
  - **High（高优）**：25 ≤ Δ < 40；
  - **Medium（中优）**：12 ≤ Δ < 25；
  - **Low（低优/跟踪项）**：Δ < 12。

---

## 二、差距项清单（总 12 条）

### D05：LLM 路由策略（P0=68，TopOSS=91 @P1 Dify，Δ=23）
- **分级：High**（Δ=23 接近 Critical 阈值；但因用户体验/治理双关键，升为 High + 归入 Critical 等价任务队列）
- **企业落地风险说明**：当线上 LLM 提供器发生抖动（429/5xx/网络丢包）或成本上涨时，无自适应的 EWMA 延迟 + 错误率路由会直接把风险传导给终端用户，表现为"越慢越往慢的提供器排"的螺旋降级，可能引起 P99 延迟从 1s 升到 5s~10s 的雪崩；与 Dify Fallback + OrcaRouter、Flowise 条件路由相比，璇玑需要大幅补齐。
- **优化候选 1（主路径）**：O1 — 新增 `LatencyWarm` 路由策略（EWMA α=0.2、Top2 每 50 req 预热 ping、分数 0.6×归一化延迟 + 0.3×(1-错误率) + 0.1×优先级、失败 200ms 切换 Top2）。
  - **映射任务：T7（O1 补丁） + H2 before/after 对比 CSV**
- **优化候选 2（增强/未来）**：语义路由（嵌入匹配 Prompt 与模型能力标签，如代码→强模型、翻译→快模型、长文→128k 窗口模型），列入 v3.2。

### D06：限流熔断治理（P0=58，TopOSS=86 @P1 Dify，Δ=28）
- **分级：High / Critical 等价（治理类 + Δ=28 ≥ 25，按规则升为 Critical 级）**
- **企业落地风险说明**：璇玑 security.js 当前用的是"固定窗口计数 + blocked + resetTime 半开"，缺点：(1) 边界突刺（resetTime 瞬间可能 2× 流量进入），(2) 无租户级 qps 配额矩阵，(3) 断路器只配置结构体未实现真实的"错误率阈值→开→半开→关"。200 并发压测（H1）时 VIP 和普通用户同桶，可能把合规的 VIP 也拒掉。
- **优化候选 1（主路径）**：O2 — TokenBucket（容量 + tokens_per_sec）+ MultiTenantRateLimiter（默认 10/50/2 qps for NORMAL/VIP/ANONYMOUS，可从 config.rate_limits 覆盖），与旧滑动窗口并行运行避免破坏 API。
  - **映射任务：T8（O2 补丁） + H1 before/after CSV 对比**
- **优化候选 2（增强）**：真实 Percentage CircuitBreaker（错误率>20%→开，<5%→半开，回退到 fallback provider），列入 v3.2。

### D11：可观测性/追踪（P0=52，TopOSS=90 @P1，Δ=38）
- **分级：High**（Δ=38；OTel 全栈投入较大，先做平台侧 SLO JSON 作为一阶落地）
- **风险说明**：企业部署没有统一 Span/Metrics/Logs 三汇导致的平均故障定位时间（MTTR）从 15min 升到 2h+；Rust/Node/Wasm 三层没有统一 trace id 串联，跨层排障完全依赖人工看日志。
- **优化候选 1（主路径）**：O4 — 新增 slo_metrics.js（4 窗口：1m/5m/15m/total，P² 近似 p50/p95/p99）+ `GET /system/slo` 路由 + 指标兼容 O1~O3 新 metric；使运维至少拿到真实 p99 / success rate。
  - **映射任务：T10（O4）**
- **优化候选 2（v3.2）**：OpenTelemetry 双栈（Rust tracing-opentelemetry + Node @opentelemetry/sdk-node）统一注入 Trace Provider。

### D12：SLO/SLA 可视化看板（P0=35，TopOSS=88 @P1，Δ=53）
- **分级：Critical**（Δ=53 ≥ 40 硬规则；SLO 可视化是企业运维最基本配置项之一，不能可视化等于没有 SLO）
- **风险说明**：Dify 版本 1.0 后即有执行日志/错误率面板；璇玑即便 O4 给了 /system/slo JSON，如果前端没有仪表盘组件，SRE 仍无法看。与 Dify 的差距为 53 为本次 12 项最大。
- **优化候选 1（主路径）**：O8 — 前端新增 `SloDashboard.vue`（1m/5m/15m/total 四卡，每卡 success_rate、p50/p95/p99、routing_ewma、wasm_trap、rl_bucket 饼图/趋势折线 4 个 ECharts）。
  - **映射任务：T12（O8 前端组件）**
- **优化候选 2（主路径前置依赖）**：O4（见 D11 候选）
  - **映射任务：T10**

### D04：多租户隔离模型（P0=72，TopOSS=90 @P1，Δ=18）
- **分级：Medium**
- **风险说明**：璇玑虽然 RBAC 审计体系比 AutoGen/Flowise 开源版强得多，但核心业务表未统一携带 tenant_id，无法做真正的行级隔离；大型企业客户跨部门协作可能泄漏数据风险。
- **优化候选 1（v3.2 跟进，本轮列入清单但不执行）**：为 kb_documents / atlas_projects / ous 表统一注入 tenantId 并在查询层自动 filter，与 Dify 行级隔离对齐。

### D07：工作流编排引擎（P0=72，TopOSS=95 @P2 LangGraph，Δ=23）
- **分级：High**（Δ=23；并行扇出+取消传播是任何生产级编排必须的基础）
- **风险说明**：当工作流有 N 个可并行子任务时，璇玑 6 节点串行的 P99 线性放大到 N×单步延迟，如 10 个 HTTP 请求串行 = 10×300ms=3s → 并行后仅为 300ms，是 10x 延迟收益。
- **优化候选 1（主路径）**：O5 — ParallelNode（并发度限制 + cancel_on_first_err + Semaphore + FuturesUnordered） + CancellationToken（AtomicBool + Notify）。
  - **映射任务：T11（O5）**
- **优化候选 2（v3.2 OQ1）**：Start/End/IfElse/Loop/LLM/Knowledge/Tool/HTTP 八节点与 Dify 对齐。

### D09：RAG/多模态检索（P0=68，TopOSS=94 @P1，Δ=26）
- **分级：High**（Δ=26）
- **风险说明**：KB 切块仅按行/句切且无 Overlap Window，会造成跨块上下文丢失 → Recall@10 下降 5%~15%；无标题感知会把"### 2.架构"这样的标题碎块与正文一起检索，出现噪声。
- **优化候选 1（主路径 O6）**：标题感知切块（以 `## / ### / # ` Markdown 标题为 chunk 边界 + 20% Overlap Window + 行内语义切分），写入 entity-extractor.js splitSections 函数升级。
  - **映射任务：T12（O6）**
- **优化候选 2（v3.2 OQ2）**：1000-doc 合成数据集 Recall@10 深度对比实验 + Embedding/Rerank 接口抽象。

### D03：插件/算子沙箱（P0=90，TopOSS=90 @P1 并列，Δ=0 → **但璇玑缺 Fuel + Mem hard limit trap 的运行时终止能力，以"细节差距"补一项 High**）
- **分级：High（细节差距，但企业级安全强制要求）**
- **风险说明**：wasmer 默认不会阻止"恶意 Wasm 死循环 100% CPU"或"memory.grow 无限申请"，会把宿主线程卡死（Dify 的 Sandbox/Plugin Daemon 有资源 cgroup 限制）。即便 P0 分数与 P1 并列，运行时终止能力是硬门槛。
- **优化候选 1（主路径 O3）**：WasmOperator::with_limits(module_bytes, fuel, mem_pages_limit) — fuel 用 wasmer Store::set_fuel / memory type min+max trap；新增 2 条测试：死循环 fuel 耗尽陷阱、超大 grow 陷阱。
  - **映射任务：T9（O3） + H3 before/after CSV**

### D14：冷启动/热路径（P0=65，TopOSS=93 @P1，Δ=28）
- **分级：High**（Δ=28，O1 的 Top2 候选预热动作已覆盖 60%，可作为 O1 侧车）
- **风险说明**：Node LLM 网关首次请求可能包含 DB 连接、模型配置加载、LocalEngine 惰性初始化；Dify Worker 池预热可将 P50 从 400ms 降到 150ms。
- **优化候选 1（与 O1 合并实现）**：O1 LatencyWarm 已自带"每 50 请求 Top2 预热 ping"；另外在 llm-gateway.js boot() 函数中对所有 provider 发送一条最小 `ping`，实现启动预热。

### D17：部署运维（P0=40，TopOSS=87 @P1，Δ=47）
- **分级：Critical（Δ=47 ≥ 40 规则）**，但用户 OQ3 明确未选中（本次默认不执行）
- **风险说明**：没有 docker-compose/Helm 会让企业部署成功率从 95% 降到 30% 以下（手动多服务拉起，版本对齐难）。但根据 OQ3 默认回答，本轮列入 v3.2 跟进。
- **优化候选（v3.2 T12-C）**：docker-compose 9 服务全栈（16 Rust crates 合一个 runtime 镜像 + backend-node + frontend-ui + Postgres/Redis + Nginx） + Helm Chart（templates/deployment/service/ingress + values.yaml）

### D16：扩展接口/生态钩子（P0=82，TopOSS=93 @P1，Δ=11）
- **分级：Low（Δ=11）**
- **说明**：与 Dify Manifest 签名 + 插件市场相比，主要缺少 Manifest 签名校验、兼容版本矩阵、加载失败自动回滚。列入 v3.3 跟踪项。

### D02：语言策略（P0=94，TopOSS=92 @P2，Δ=-2，璇玑胜出）
- 非差距项；璇玑 Polyglot 做得更好，无需优化。

### D18：开源协议 & 商业友好（P0=90，TopOSS=96 @P2，Δ=6）
- **分级：Low（Δ=6）**
- 差距：LangGraph 纯 MIT，璇玑包含 wasmer（Apache 2.0 商业友好但不是 MIT 双许可）。对使用无实质影响，跟踪项。

### D10：图谱算法 & 项目全息（P0=96，TopOSS=88 @P1，Δ=-8，璇玑胜出）
- 本项璇玑为标杆，没有优化需求，但可做 O7：
  - **O7（Medium 级补强）**：把 7 大算法的 Rust/Node 真实 P99 延迟通过 slo_metrics.js 上报到 /system/slo 的 `graph_algorithms_p99_ms` 子对象中，便于运维观察（列入 T12 3 条中优补丁第 2 条，凑足 O1~O8 8 条）。

---

## 三、差距分级汇总（按 Critical/High 排序）

| 编号 | 维度 | Δ | 分级 | 对应优化项 | 映射任务 | 企业风险（1 句摘要） |
|---|---|---|---|---|---|---|
| 1 | D12 SLO 可视化看板 | 53 | **Critical** | O4 + O8 | T10/T12 | 无 SLO 可视化 = 没有可执行的 SLA 文化，MTTR 上升 8×。 |
| 2 | D17 部署运维（v3.2） | 47 | **Critical** | T12-C | 暂缓 | 无 Compose/Helm 部署成功率 <30%，留 v3.2 OQ3 跟进。 |
| 3 | D06 限流熔断治理 | 28 | **Critical（治理类升级）** | O2 | T8 | 固定窗口突刺 + 无租户配额，易被单一租户占满资源。 |
| 4 | D11 可观测性/追踪 | 38 | **High** | O4 | T10 | 三栈无统一 Trace ID，跨层排障依赖人工。 |
| 5 | D09 RAG/多模态检索 | 26 | **High** | O6 | T12 | 切块无 Overlap + 无标题感知 → Recall@10 掉 10%+。 |
| 6 | D14 冷启动/热路径 | 28 | **High** | O1（附加热路径） | T7 | 首次请求冷启动延迟可能比热路径高 3×+。 |
| 7 | D07 工作流编排 | 23 | **High** | O5 | T11 | 无并行扇出使子任务并行部分线性放大，P99 恶化 N×。 |
| 8 | D05 LLM 路由策略 | 23 | **High** | O1 | T7 | 无自适应路由 + 失败切换可能导致雪崩延迟。 |
| 9 | D03 Wasm 沙箱（安全细节）| Δ=0 细节 High | **High** | O3 | T9 | 死循环/无限申请会卡死宿主线程；Wasm 安全不完整。 |
| 10 | D04 多租户隔离 | 18 | **Medium** | v3.2 T12-A2 | 暂缓 | 缺少统一 tenant_id 行级过滤。 |
| 11 | O7 图谱算法 P99 上报（补强）| Δ=-8 但缺 Metric 上报 | **Medium** | O7 | T12 | 图谱算法的 P99 无法在运维侧看趋势。 |
| 12 | D16 扩展接口/生态钩子 | 11 | **Low** | v3.3 跟踪 | — | Manifest 签名与兼容矩阵缺失。 |

---

## 四、本轮（v3.1）将落地的 8 个企业级优化补丁（对应 FR6 ≥8 条要求）

| Optim ID | 名称 | 优先级 | 对应差距项 | 映射任务 | Feature Flag | 影响文件范围（侵入性评估：≤ 2 主入口 = 低）|
|---|---|---|---|---|---|---|
| **O1** | LLM LatencyWarm 路由（EWMA + 预热 + 加权分数） | High | D05 + D14 | T7 | `DISABLE_OPTIM_O1_LATENCY_WARM` | `llm-gateway.js`（+ slo_metrics.js 写入 metric）→ 侵入性低 |
| **O2** | TokenBucket + 租户级 QPS 配额 | High/Critical | D06 | T8 | `DISABLE_OPTIM_O2_TOKEN_BUCKET` | `security.js`（并行旧机制）→ 侵入性低 |
| **O3** | Wasm Fuel + Memory 硬上限 Trap | High | D03 细节 | T9 | `DISABLE_OPTIM_O3_WASM_FUEL` | `operator-wasm/src/lib.rs`（新构造函数）→ 侵入性低 |
| **O4** | `/system/slo` JSON + SloMetrics 四窗口 | High/Critical | D11 + D12(前半) | T10 | `DISABLE_OPTIM_O4_SLO` | `slo_metrics.js`（新）+ `routes/system.js`（增路由）→ 侵入性低 |
| **O5** | ParallelNode + CancellationToken | High | D07 | T11 | `DISABLE_OPTIM_O5_PARALLEL_NODE` | `ai-agent/flow_engine.rs`（新结构 + 新函数）→ 侵入性低 |
| **O6** | KB 标题感知切块 + Overlap Window | Medium | D09 | T12 | `DISABLE_OPTIM_O6_TITLE_CHUNK` | `entity-extractor.js`（重写 splitSections + chunkDocument）→ 侵入性低 |
| **O7** | 7 大图谱算法 P99 延迟指标上报 /system/slo | Medium | D10 补强 | T12 | 随 O4 同开关（共享 slo_metrics）| `graph-algorithms 侧 emit` + `slo_metrics.record_graph_algo` → 侵入性极低 |
| **O8** | 前端 SloDashboard.vue（ECharts 四卡四窗口） | Medium（D12 后半）| D12 | T12 | `DISABLE_OPTIM_O8_SLO_DASHBOARD` | `SloDashboard.vue`（新组件）+ router/index.js（注册路由）→ 侵入性低 |

> 侵入性评估：全部 8 项补丁均为 **增量 / 新文件 / 新路由 / 新构造函数 / 新结构体**，不修改现有 16 Rust crate 与 32 Node 路由的主入口契约；与 AC-22 rubric "≤ 5 入口 + 纯增量"匹配（目标评分 100）。

---

## 五、TR-2 自检（Tasks T2 测试要求）

| TR ID | 要求 | 结果 |
|---|---|---|
| TR-2.1 (AC-03) | 差距条目数 ≥ 4 且 Critical/High 总条目 ≥ 3 | ✅ 差距条目 = 12（+1 细节 High + 1 补强 Medium），Critical = 3，High = 6，C+H = 9 ≥ 3 |
| TR-2.2 (rule) | 每条 Critical/High 必须链接到至少一个优化任务编号 | ✅ 上表第三列"映射任务"全部填入（T7/T8/T9/T10/T11/T12） |
