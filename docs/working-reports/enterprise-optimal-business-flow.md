# 璇玑 · 企业级最优业务处理流程（Best-Practice Business Flow · 100% 自研图谱驱动）v1.0

> 配套文档：对比报告 `mox-vs-opensource-comparison-report.md`
> 验收脚本：`scripts/run-enterprise-final-acceptance.ps1`

## 0. 业务治理总原则

璇玑唯一底层中枢 = **璇玑全域知识图谱**。所有业务处理必须遵循「**节点化 + 关联化 + 可溯源 + 可推演 + 可验证**」五项原则。禁止脱离图谱的「隐式经验业务」。

```
需求图谱节点 ←双向绑定→ 业务节点 ←双向绑定→ 架构节点 ←双向绑定→ 代码模块（Rust crate / Node 模块）←双向绑定→ 测试节点（TR）
```

任何一层变化 → 图谱联动 → 自动溯源 → 触发下游 TR 重评。

---

## 1. 阶段 1：全域需求归一化（Requirement Normalization）

### 1.1 需求分类
每条需求必须被拆分为 8 大类，全部实体化录入图谱：

| 需求类型 | 图谱实体字段 | 验收标准来源 |
|---|---|---|
| 显性需求 | req.explicit.* | 合同/PRD/甲方文档 |
| 隐性需求 | req.implicit.* | 领域专家联盟推理结论 |
| 边界需求 | req.boundary.* | 架构约束 + 性能预算 |
| 兼容需求 | req.compat.* | 既有系统对接清单 |
| 扩展需求 | req.extend.* | 未来 6 个月 Roadmap |
| 性能需求 | req.perf.* | P95/P99 延迟、吞吐、并发 |
| 安全需求 | req.sec.* | 权限分级、数据分级、签名、审计 |
| 运维需求 | req.ops.* | SLO 4 窗口、日志、备份、回滚 |

### 1.2 优先级与依赖
- **优先级**：P0（阻断）/ P1（交付周期必达）/ P2（下一周期）/ P3（积累型）。
- **依赖图**：每条需求实体必须关联 `depends_on: [req_id...]`。图谱自动做拓扑排序，检测循环依赖 → 强制拆分。

### 1.3 验收标准 = TR（Test Requirement）
每条需求必须对应 ≥ 1 条可机器执行的 TR。TR 必须在代码提交前写入 `test/`，通过 `mocha` / `cargo test` 自动执行。禁止「先写代码后写测试」。

---

## 2. 阶段 2：全域业务图谱化（Business Graphing）

### 2.1 业务节点化

任何业务流程必须拆为独立节点：

```
业务节点 BizNode {
  id:              string      // 全局唯一，biz:<功能域>:<动词>:<名词>
  name:            string      // 中文名 16字内
  trigger:         Trigger     // { timer | http | mq | manual | graph-event }
  conditions:      Expr[]      // 进入条件（基于前置节点输出）
  permissions:     Role[]      // 谁能触发（OUS RBAC + 分级）
  inputs:          FieldSpec[] // 数据输入（schema + 校验规则）
  outputs:         FieldSpec[] // 数据输出（schema + 校验规则）
  exceptions:      ExBranch[]  // 异常分支 → 自动降级 + 告警
  upstreams:       BizNode[]   // 上游业务节点依赖
  downstreams:     BizNode[]   // 下游业务节点联动
  module_rust:     CratePath   // 对应 Rust crate 路径
  module_node:     JsPath      // 对应 Node 模块路径
  tr_refs:         TR[]        // 自动化验收用例（≥ 1 条）
}
```

### 2.2 触发条件 → 调度层自动识别
- **timer**：`mox-system` 的 scheduler service → 周期/日历触发。
- **http**：`routes/*.js` 注册 → OUS_API_TOKEN 鉴权前置。
- **mq**：`hermes-flow-bridge` live 推送带超时 + 指数退避。
- **graph-event**：图谱节点写入 → `kg-hub` 发布 → 下游订阅联动。
- **manual**：操作人 UI 确认 → 自动审计 appendLog。

### 2.3 异常分支
不允许 `catch(err) return {}` 这类静默吞噬。异常必须：
1. 有独立的降级策略（Degradation）：重试（带退避）→ 切换备用（memory / file fallback）→ 旁路（flag=false）。
2. 必须写入 SLO：`slo.record(biz_key, duration, success?)`。
3. 必须审计落盘：`POST /system/logs/append` 双写 Source of Truth。

---

## 3. 阶段 3：全域架构落地（Architecture Landing）

严格遵循 **AIS 六层 DIP 架构**，只允许下层依赖上层抽象，不允许反向。

```
Layer 6  Gateway  / HTTP 路由分发 + 前置鉴权（api-server.js）
  ↓ 依赖抽象接口
Layer 5  Runtime  / runtime crate （ai_engine / handlers / service-manager）
  ↓ 依赖抽象接口
Layer 4  Operator / operator-core / operator-wasm → 标准 IOperator 契约
  ↓ 依赖抽象接口
Layer 3  Service  / 21 workspace service crate + Node 路由域
  ↓ 依赖抽象接口
Layer 2  Abstractions / mox-domain-abstractions + mox-standards
  ↓ 依赖抽象接口
Layer 1  Domain   / business-catalog / kg-hub / mox-common-meta 等业务域
```

### 3.1 依赖倒置原则（DIP）校验
- **高层模块不依赖底层实现**，只依赖抽象 trait。
- `t6_dip_orchestrator.rs` 中 9 处 `unimplemented!()` → 全部替换为真实 Mock，验证 DIP 成立（11/11 绿）。
- `mox-system/src/orchestrator.rs` 禁止直接 `use crate::services::*`，必须用 abstract trait。

### 3.2 单源归一化（杜绝重复开发）
以下功能仅允许存在 **一处** 真实实现，其他位置只能是薄封装：

| 功能 | Source of Truth | 其他使用方 |
|---|---|---|
| 图算法 PageRank/Degree/Betweenness | `src/graph/graph-formulas.js` | `graph-algos.js` / Rust `graph-algorithms` crate / `ai-flow-graph.js` 薄封装 |
| 意图识别 Intent Detect | `mox-common-meta/src/intent.rs` | mox-system / ai-agent / flow-ai 只调 intent 抽象接口 |
| 日志双写 + 容量策略 | `src/lib/logger.js` LOG_CAPACITY=50000 | system.js /logs/append 复用 same constant + push 语义 |
| JSON 存储读写 | `src/lib/json-store.js` writeJSON=磁盘优先、readJSON=存储优先 | 所有 ctx.readJSON/writeJSON 调用都走此 |
| 模板引擎 | `ai-agent/src/engine/tools.rs` 单一 TemplateEngine 实例 | ai-agent 所有 tool 不允许重复实现 MiniJinja/Tera 引擎 |

### 3.3 域一致性（D1 · 7/7）
业务域一旦注册，必须在 **business-registry.js ↔ routes/index.js ↔ project references** 三处同步。D1 专门 TR 检测对称差 = 0（23 Rust 域 + 30 业务实体 = 53 × 3 源 0 孤点）。

---

## 4. 阶段 4：全域开发（Rust 自研核心）

### 4.1 Rust crate 目录规范
每个 crate 对应 1~3 个图谱业务节点，严格遵守 `src/lib.rs` 入口 + `tests/*.rs` 集成测试。

| 目录 | 业务职责 |
|---|---|
| `platform/services/operator-core` | 算子核心（执行/调度/CRDT 幂等） |
| `platform/services/operator-wasm` | 第三方算子沙箱（安全隔离） |
| `platform/services/graph-algorithms` | Rust 侧图算法（与 graph-formulas.js 数学对齐） |
| `platform/services/flow-ai` | 数据流编排 |
| `platform/services/primiflow-core` | 原生流程内核（示例模板 ≥ 15，已禁止 todo!()） |
| `platform/services/primiflow-fusion` | 多流融合 |
| `platform/services/mox-system` | 系统服务（DIP 严格、已通过 11 条 DIP 测试） |
| `platform/services/mox-expert` | 专家联盟引擎 |
| `platform/services/hermes-flow-bridge` | 跨系统桥接（带 catch_unwind + 指数退避、超时） |
| `platform/services/ai-agent` | Agent 调度（DatabaseTool 三级 fallback：file → memory → disabled） |
| `platform/services/optimizer` | 无限优化器 |
| `platform/services/business-catalog` | 业务目录 |
| `platform/services/template-market` | 模板市场 |
| `platform/services/kg-hub` | 知识图谱 Hub |
| `platform/services/mox-common-meta` | 公共元（意图识别 Source of Truth） |
| `platform/services/mox-domain-abstractions` | 业务域抽象（trait 定义层） |
| `platform/services/mox-standards` | 标准（版本、契约、合规） |
| `platform/services/mox-graph-meta` | 图谱元数据（与 registry 互查） |
| `platform/services/mox-cloud-drive-master` | 云盘主节点 |
| `platform/services/mox-cloud-drive-volume` | 云盘卷节点 |
| `platform/gateway/runtime` | Gateway runtime（handlers/ai_engine：stub → 真实 fallback summary） |

### 4.2 Rust 开发禁令
- 禁止 `todo!()` / `unimplemented!()` 出现在生产代码路径（可用于 test mock，必须有真实 mock return）。
- 禁止 `[stub]` 占位输出（已替换为确定性 fallback）。
- 禁止 `unwrap()` 在非初始化 / 非 test 代码。
- Clippy `-D warnings` 必须 0 ERROR。

### 4.3 Node 开发禁令
- 禁止写 2 套相同算法（graph-formulas 已归一、意图已归一）。
- `writeJSON` 成功才返回 ok()，失败必须通过 磁盘兜底 + NDJSON 三级降级。
- 所有 POST 路由必须经过 api-server.js 分发层 OUS_API_TOKEN 鉴权。

---

## 5. 阶段 5：全域测试验证（TDD · 全链路）

### 5.1 测试金字塔
```
          / \    企业专项验收（D1-D5 + P4 10task）
         /___\   HTTP Smoke + E2E（rust_crate_bindings 56/56）
        /_____\  集成测试（Node mocha + Rust tests/）
       /_______\ 单元测试（Rust lib + Node）
```

### 5.2 自动化验证清单（按阶段）
| 阶段 | 命令 | 期望 |
|---|---|---|
| T0 Rust 全量 | `cargo test --workspace --lib --bins --tests` | exit 0, 250+ tests pass |
| T1 Clippy 合规 | `cargo clippy --workspace -- -D warnings` | exit 0 |
| T2 Domain 一致 | `npx mocha test/test-d1-domain-consistency.js` | 7/7 GREEN |
| T3 Game Artifacts | `npx mocha test/test-d2-game-pipeline.js` | 5/5 GREEN |
| T4 Observability | `npx mocha test/test-d3-observability.js` | 6/6 GREEN |
| T5 Security Token | `npx mocha test/test-d4-security.js` | 7/7 GREEN |
| T6 Build Workspace | `npx mocha test/test-d5-build-workspace.js` | 5/5 GREEN |
| T7 10task 评分 | `./scripts/run-10task-rubric.ps1 -Mode Full` | 100/100, cheat=0 |
| T8 全量 HTTP 可用性 | `npx mocha test/test-enterprise-usability-http-smoke.js` | 12/12 GREEN |
| T9 E2E Rust 绑定 | `npx mocha test/rust_crate_bindings_e2e.js` | 56/56 GREEN |
| T10 公式单源 | `npx mocha test/test-graph-formulas-single-source.js` | PASS（0 conflict） |

### 5.3 一键验收
```powershell
pwsh ./scripts/run-enterprise-final-acceptance.ps1
```

流水线：P1(Rust) → P2(Node unit) → D1-D5 专项 → P4(10task) → P5(Report md+json)。全绿才返回 0。

---

## 6. 阶段 6：全域观测与运维闭环

### 6.1 三大企业指标
| 指标 | 定义 | 采集 |
|---|---|---|
| Availability | 成功率 = success_count / total_count | SloTracker per window |
| P95 Latency ms | 请求/节点耗时 P95 | SloTracker ring quantile |
| Error Rate | 失败率 = 1 - availability | SloTracker per window |
| Throughput (RPS) | sample_count / window_ms × 1000 | SloTracker per window |

4 窗口：`1m / 5m / 15m / 1h`，D3 4/5 验证数值合理性（无 NaN/Inf/负、avail 在 [0,1]）。

### 6.2 审计链路
- `POST /system/logs/append` → 双写：磁盘 logs.json + SQLite logs 表 + NDJSON 兜底。
- `GET /system/logs?limit=50&offset=0` → 支持时间倒序 + level/type 过滤。
- 企业审计要求：不可篡改。可选（生产）：写入后附加 SHA256 hash-chain，D1 test project-atlas hash chain 已验证可用。

### 6.3 告警分级
| 级别 | 触发 | 行动 |
|---|---|---|
| P0 Critical | 1m availability < 95% or 1m error_rate > 5% | 即时电话 + 自动降级 switchProvider |
| P1 High | 5m P95 > SLO objective × 1.5 | 短信 + 图谱事件广播 |
| P2 Warning | 15m throughput 相比 1h baseline 下降 > 30% | 邮件 + 看板 |
| P3 Info | 任一节点首次异常 | 日志 + 追踪 |

---

## 7. 阶段 7：优化与迭代（图谱驱动）

每次迭代循环固定为：

```
需求采集 → 图谱实体化 → 依赖/优先级分析 → 架构自动影响分析 → TR 编写（TDD）
→ 代码开发（Rust + Node 严格分层） → Clippy/-D warnings → cargo test
→ D1~D5 专项 → P4 评分 → Report → 发布
→ SLO/SLI 采集 → 业务效果复盘 → 新需求采集（回到起点）
```

璇玑图谱会在每一步记录节点变化：
- 代码 commit → `code:*` 节点 last_modified 更新。
- TR 结果 → `tr:*` 节点 status 更新。
- 发布成功 → `deploy:*` 节点生成，反向关联需求/架构/代码/TR 全链路。

---

## 8. 合规清单（一图看懂业务流程完整性）

| 序号 | 必做项 | 企业级标准 | 通过证明 |
|---|---|---|---|
| 1 | 需求 ↔ 模块 ↔ 接口 ↔ 算法 ↔ 场景 全关联 | 图谱全部实体化 | D1 7/7 + kg-hub 验证 |
| 2 | 业务分支 ↔ 独立工程单元（Rust + Node） | 每个 BizNode 有独立 crate 路径和 module 路径 | D1 53 实体 × 3 源 0 孤点 |
| 3 | DIP 依赖倒置 | AIS 六层 + 抽象 trait 不反向 | t6_dip_orchestrator.rs 11/11 |
| 4 | 安全鉴权 | OUS_API_TOKEN 分发层 + 敏感写 401 + 4 路 token | D4 7/7 |
| 5 | 观测闭环 | SLO 4 窗口 + 审计写→读闭环 | D3 6/6 |
| 6 | 构建一致 | workspace 21 crate 0 孤儿 + cargo metadata 21/21 | D5 5/5 |
| 7 | 10task 100/100 评分 | cheat=0 R1 pass | P4 run-10task-rubric |
| 8 | 图算法/意图/模板 单源归一 | 0 重复开发 | T9 + T10 测试 |
| 9 | 生产代码无 todo! / stub / unimplemented! | 全真实实现 | grep 0 命中 + runtime ai_engine fallback |
| 10 | 一键验收脚本 | P1-P5 全绿 exit 0 | run-enterprise-final-acceptance.ps1 |

---

## 9. 最佳实践反模式（禁止事项）

| 反模式 | 正确做法 |
|---|---|
| 先写功能后补测试 | 先写 TR，测试失败再开发，测试通过即完成（TDD）。 |
| 同一功能开发 2 套独立实现 | 先在 registry 登记 Source of Truth，其他位置只能 import + 薄封装。 |
| 直接用具体 service impl，不依赖抽象 trait | AIS DIP，高层只 use `dyn AbstractTrait`，具体实现依赖注入。 |
| appendLog 与 logs/append 各写各的 | 两者共享 `LOG_CAPACITY`，容量策略一致；/logs/append 禁止再次调用 appendLog 造成双重写竞态。 |
| writeJSON 先写存储再写磁盘 | 磁盘优先，写入磁盘成功即返回 true，存储失败不影响 Source of Truth。 |
| OUS_API_TOKEN 鉴权放 handler 里（后置） | 放 dispatch 层，body 解析 / 路由匹配之前就拒绝。 |
| 硬编码域数量 test 比对 | 动态计算 registry entities，避免新增/删除域后误报失败。 |
| 跨 test 端口复用（残留进程 + 旧代码） | 测试时随机独占端口，100% 使用最新源码。 |

---

## 10. 交付物汇总（一次完整交付的清单）

```
mox-release-v3/
├── Cargo.toml                        # 21 workspace 成员
├── platform/
│   ├── services/                     # 20 service crates (src/lib.rs + tests/)
│   ├── gateway/runtime/              # 1 gateway crate
│   └── backend-node/
│       ├── src/
│       │   ├── routes/               # kb/system/artifacts/... 共 69 业务域路由
│       │   ├── graph/graph-formulas.js   # 图算法 Source of Truth
│       │   ├── lib/json-store.js     # 磁盘优先双写
│       │   ├── lib/logger.js         # LOG_CAPACITY=50000 push 语义
│       │   ├── slo-tracker.js        # 4 窗口 SLO
│       │   └── api-server.js         # 分发层 + OUS_API_TOKEN 前置鉴权
│       └── test/                     # 200+ 自动化用例，含 D1-D5/HTTP-smoke/E2E
├── scripts/
│   ├── run-enterprise-final-acceptance.ps1   # 一键验收
│   └── run-10task-rubric.ps1                 # 10 任务评分
├── data/
│   ├── enterprise_10task_definitions.json    # 评分规则
│   ├── logs.json                     # 审计日志（含种子事件）
│   └── artifacts/tictactoe.html      # 可玩游戏模板（>3KB）
└── .trae/documents/
    ├── mox-vs-opensource-comparison-report.md   # 自研 vs 开源对比
    └── enterprise-optimal-business-flow.md          # 本文档
```

**结果 = 全链路无断点 · mox 模块化系统架构维度无遗漏 · 业务与架构统一 · 代码与图谱统一 · 文档与工程统一** → 璇玑知识图谱驱动的企业级全自动标准化研发交付体系。
