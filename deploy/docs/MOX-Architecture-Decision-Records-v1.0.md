# MOX 平台架构决策记录 (ADR) v1.0

**版本：** 1.0.0
**发布日期：** 2026-08-27
**状态：** ACTIVE（生效中）
**维护者：** 璇玑 RelGraph · OUS 三联盟 · 架构委员会
**关联文档：** [MOX-Enterprise-Unified-Spec-v2.0.md](./MOX-Enterprise-Unified-Spec-v2.0.md) · [MOX-NodeToRust-Migration-Handover-v1.0.md](./MOX-NodeToRust-Migration-Handover-v1.0.md)

---

## 关于 ADR

本文件按「编号 ADR-XXX + 状态 + 上下文 + 决策 + 后果 + 后续」格式记录**所有不可逆的架构决策**。所有后续开发必须遵守已生效的 ADR，不得在没有新 ADR 覆盖的前提下自行推翻。

```
状态枚举：
  PROPOSED  → 提案中
  ACCEPTED  → 已通过
  ACTIVE    → 生效中（当前）
  DEPRECATED→ 已被新 ADR 取代
  SUPERSEDED→ 已被完全替换
```

---

## ADR-001: 后端技术栈统一为纯 Rust（全面退役 Node.js）

| 字段 | 内容 |
|---|---|
| **编号** | ADR-001 |
| **状态** | ACTIVE |
| **日期** | 2026-08-27 |
| **决策人** | 架构委员会全体 |

### 上下文

MOX 后端经历了 3 个阶段演进：
1. **V1（2025 初）**：纯 Node.js（`platform/backend-node/`）单仓承载全部业务 — 32 路由文件 × 8 域 × 90% 业务代码，端口 3000
2. **V1.5（2025 年末）**：引入独立 Rust workspace（`platform/backend-rust/`）承载 Q/R/S/T 四类横切能力（API网关限流熔断/数据质量血缘/零信任mTLS/AIOps 根因分析），独立端口、独立二进制 `mox-gateway`
3. **V2（2026 Q3 开始）**：6 层模块化 Rust 架构（`platform/{gateway,domains,foundation,framework,shared,scripts}`），8 域 × 60+ crate × 接入/网关/编排/业务/算法/基础六层

问题：**三套后端并存 = 端口冲突 × 功能重复 × 运维复杂度 ×3**，违反归一化「once-defined」原则。

### 决策

1. **立即退役 Node.js 技术栈**：`platform/backend-node/` 全部源码和数据于 2026-08-27 物理删除（当前仅残留 IDE 句柄锁定的空壳目录，重启后自动消失）
2. **统一后端入口**：所有业务流量 **必须** 经过 `mox-platform-gateway-svc`（二进制 `mox-server`），默认端口 **0.0.0.0:8080**
3. **`platform/backend-rust/` 的 Q/R/S/T 能力不直接删除**，按 ADR-002 计划逐模块迁入 6 层架构的对应层
4. **任何新开发的后端代码 100% 使用 Rust**，禁止引入新的 Node.js/TypeScript 后端模块；前端与 MCP/CLI 层的脚本工具不受此限

### 后果

| 正面 | 负面 |
|---|---|
| 单语言技术栈降低招聘/维护成本 | 32 Node 路由模块需按 P0-P3 优先级逐模块迁移，初期加权覆盖度仅 23% |
| 内存安全 + 无 GC 停顿 = 金融级稳定性 | 原 Node.js 生态的 80 余个测试用例需要重写为 Rust `#[test]` |
| 单二进制部署（mox-server.exe）取代多进程 Node 集群 | 部分 Node 独有库（jianpu-ly、music21、PortAudio 音频）需要独立打包或 WASM 化 |
| 单端口 8080 对外，运维收敛 | |

### 验证信号

- `cargo run -p mox-platform-gateway-svc` 启动后 `/health` 返回 `gateway=rust-axum`
- `/api/v1/status` 明确列出替换端口：`[3000, 3001, 3002]`
- 工作区内 `platform/backend-node/` 下不存在任何 `*.js` 源码（0 files / 0 dirs 已达成）

---

## ADR-002: `backend-rust` 平行 workspace → 6 层架构的迁入路线

| 字段 | 内容 |
|---|---|
| **编号** | ADR-002 |
| **状态** | ACTIVE |
| **日期** | 2026-08-27 |
| **决策人** | 架构委员会全体 |

### 上下文

`platform/backend-rust/` 是独立 workspace（自带 `Cargo.lock`，不加入根 workspace），实现了四类有价值的横切能力，与新 6 层架构的映射如下：

```
backend-rust 模块               → 迁入 6 层架构目标位置
────────────────────────────────────────────────────────────────
Q: api_gateway (限流/熔断/重试) → framework/mox-framework/src/middleware/
R: data_quality (血缘/规则)     → domains/data/svc/mox-data-compliance-svc + core
S: zero_trust (mTLS/SPIFFE)     → foundation/mox-platform-foundation/src/zero_trust/
T: aiops (RCA/预测扩缩)         → domains/platform/svc/mox-platform-observability
deploy/istio service-mesh.yaml  → deploy/helm/mox/templates/istio-gateway.yaml
benches/ 性能基准               → platform/framework/benches/
tests/ 集成测试                 → 迁入对应 crate 的 tests/ 目录
```

### 决策

1. **不立即删除 `backend-rust/`**（删除会直接丢失 4 类成熟能力的代码），但**禁止**向该 workspace 增加任何新功能 — 2026-08-27 起进入只读状态
2. 采用「**迁一项 → 建一项测试 → 删除 backend-rust 对应源码**」的渐进方式，目标完成窗口：3 个月（2026-11-27）
3. 所有迁入的代码必须通过 `mox-arch-test`（`platform/arch-test/src/lib.rs`）的分层依赖规则校验
4. 第 0 周的切入点：`zero_trust` → `mox-platform-foundation`（基础层无跨域依赖，迁入风险最低）

### 后果

- 过渡期内 `backend-rust/` 与 6 层架构并存，但只有 6 层架构是**生产路径**
- 每迁入一个模块，同步在 `DOCUMENT-INDEX.md` 中更新状态
- 3 个月窗口结束前未迁出的能力，判定为「不再需要」后整目录删除

---

## ADR-003: 6 层分层架构 + 跨域依赖必须走 API 层

| 字段 | 内容 |
|---|---|
| **编号** | ADR-003 |
| **状态** | ACTIVE |
| **日期** | 2026-08-27 |
| **参考文献** | `platform/arch-test/src/lib.rs` 4 个架构不变量测试 |

### 上下文

MOX 遵循 AIS 企业级项目架构规范。6 层定义 + 跨域依赖规则，已在 `mox-arch-test` crate 中编码为 **4 个不变量测试**：

```
L0 foundation：不能依赖任何域 crate
L1 gateway：   可依赖 L0 + L2 api，禁止直接依赖 L3/L4/L5
L2 api：       只能依赖 L0 foundation（纯 trait 契约）
L3 core：      可依赖 L0 + L2，同域 core 可互相依赖
L4 svc：       可依赖 L0 + L2 + L3，同域 svc 可互相依赖
L5 sdk：       可依赖任何层（FFI 绑定边界）
```

跨域依赖强制：必须走 `domains/<domain>/api/` 的 trait 层，禁止直接引用另一个域的 `core/svc` 实现。

### 决策

1. `mox-arch-test` 是**架构守护测试** — 每次 CI 必须运行：
   ```bash
   cargo test -p mox-arch-test -- --nocapture
   ```
2. 任何新 crate 必须在 6 层架构中有明确定位，禁止落在 `Unknown` 分类
3. 任何跨域调用找不到对应 `api/` 层，先补 `api/` trait，再写业务

### 后果

- 目前已知的 2 个跨层例外（`mox-kg-algo-core`、`mox-platform-test-harness`）必须在 2026-09-15 前完成 API 层抽象
- 架构-数据分离测试（`test_architecture_data_separation`）要求 `platform/` 下无任何 `.db / .sqlite / .log` — 数据必须放在项目根的 `projects/` 目录（代码-数据路径分离原则）

---

## ADR-004: 单二进制网关架构（mox-server）+ 31 域模块化路由注册

| 字段 | 内容 |
|---|---|
| **编号** | ADR-004 |
| **状态** | ACTIVE |
| **日期** | 2026-08-27 |
| **关键代码** | `platform/gateway/mox-platform-gateway-svc/src/routes.rs` L10-L999 |

### 上下文

原 Node.js 后端按文件组织路由（32 个独立 `routes/*.js`），没有集中注册点，无法做统一限流/鉴权/版本/灰度。Rust 端采用「集中式路由注册表」模式，把 31 个业务域的路由全部在 `routes.rs` 中注册并标注 `(prefix, name, status, owner)` 四要素。

### 决策

1. **单二进制原则**：所有对外 HTTP 接口最终收敛到一个 `mox-server` 可执行文件，**禁止**新增独立监听端口的后端服务（非 HTTP 内部服务如 S3 协议端口除外）
2. **每个域路由注册必须四要素齐全**：
   ```rust
   DomainRoute {
       prefix: "/kg/v1",                // 路径前缀（唯一）
       name: "Knowledge Graph Service", // 人读名称
       status: DomainStatus::Ready,     // Ready / Stub / Deprecated / Retiring
       owner: "alice@mox.ai",           // 负责人邮箱
   }
   ```
3. **`status=Stub` 的域路由必须返回明确 JSON，含 `note` 字段说明迁移计划** — 禁止返回 404 裸响应，前端无法区分「未开发」和「路径错」
4. 当前（2026-08-27）状态分布：Ready 2 域（kg/v1 + ai/engine）、Stub 28 域、Retiring 1 域（原 backend-node）

### 后果

- 任何新增业务域必须先在 `routes.rs` 注册，才能在 Gateway Router 中挂载
- `stub_count` 是公开指标（`/api/v1/status` 返回），28 → 0 的下降曲线是迁移进度的主 KPIs

---

## ADR-005: 图谱算法红线（11 项不可变数学实现约束）

| 字段 | 内容 |
|---|---|
| **编号** | ADR-005 |
| **状态** | ACTIVE |
| **日期** | 2026-08-27 |
| **关键代码** | `platform/domains/kg/core/mox-kg-algo-core/src/lib.rs` |

### 决策（已编码到 `mox-kg-algo-core`，18/18 tests 通过）

1. 社区检测：**CNM（模块度贪心凝聚）**，禁止 LPA（标签传播）
2. 介数中心性：**Brandes 算法** O(V·E)，禁止 Floyd O(V³)
3. 紧密中心性：**Harmonic 算法**（处理不连通图），禁止传统 1/avg_d
4. 激活扩散意图识别：**个性化 PageRank 特例**，`method=spread, d=0.85, 30 轮收敛`
5. 无向图边处理：**统一 RAW 边输入 → 库内展开双向**，禁止度中心性被除 2 错误
6. 公式库输出：**全精度 f64**，禁止 `toFixed` 截断；测试容差统一 `1e-6`
7. 密度指标必须附带人读文案：「高度稠密 D>0.5 / 中等密度 0.2≤D≤0.5 / 稀疏图 D<0.2」
8. 所有中心性指标输出必须附带人读公式
9. 流程图谱构建：**先 create_nodes → 后 add_edges**，禁止边引用未创建节点静默丢失
10. JSON 写：**tmp+rename 原子写**，禁止原地 writeFileSync
11. 大列表（>5000）saveList 更新：**增量变更日志或节流合并**，禁止全量重写性能瓶颈

---

## ADR-006: 文档归一化（once-defined 原则）

| 字段 | 内容 |
|---|---|
| **编号** | ADR-006 |
| **状态** | ACTIVE |
| **日期** | 2026-08-27 |

### 上下文

历史文档存在严重重复：
- 「三大铁律」在 `storage-cloud-switch-sop.md` / `filesystem-backend-structure-sop.md` / `FS-S3-full-lifecycle-ops-guide.md` 三处各写一次
- 「T0-T3 档位阈值」在 5 份文档中重复定义，存在 2 处数值漂移风险
- 架构图在 3 份 README + 2 份运维手册中各画一遍，共 5 个版本

### 决策

1. **once-defined 原则**：每个概念/阈值/流程只在**一份主文档**中定义，其他文档只允许用节号引用（例如「详见总纲 §5.6」），**禁止复制粘贴**
2. **主文档职责划分**（9 份正本 + 2 份索引）：

   | 文档 | 唯一职责 |
   |---|---|
   | `MOX-Enterprise-Unified-Spec-v2.0.md` | 归一化总纲（定义所有规则、阈值、schema 定义） |
   | `MOX-Fullstack-Auto-Delivery-Plan-v2.0.md` | 12 周交付计划（唯一的 P0-P12 阶段时间线定义） |
   | `FS-S3-full-lifecycle-ops-guide.md` | **唯一的** FS ↔ S3 切换全生命周期 SOP（合并删除 2 份子手册） |
   | `MOX-Architecture-Decision-Records-v1.0.md` | **本文件**，架构决策唯一真相源 |
   | `MOX-NodeToRust-Migration-Handover-v1.0.md` | **唯一的** Node→Rust 迁移覆盖矩阵与缺口清单 |
   | `ha-capacity-tco.md` | **唯一的** HA/容量/TCO 档位配置表 |
   | `ops-manual.md` | **唯一的** 日常运维 runbook 集合 |
   | `xinchuang-matrix.md` | **唯一的** 信创兼容矩阵 |
   | `trace-8stages-dashboard.json` | **唯一的** 可观测性 dashboard 定义 |
   | `DOCUMENT-INDEX.md` | 文档索引总图（本规则配套索引，不含定义内容） |
   | `APPENDIX-CRATE-README-INDEX.md` | crate 级自描述文档索引（引用，无重复定义） |

3. 2026-08-27 已执行的动作：
   - **删除** `filesystem-backend-structure-sop.md`（已完整合并到 FS-S3 全生命周期手册 v2.0.0）
   - **删除** `storage-cloud-switch-sop.md`（已完整合并到 FS-S3 全生命周期手册 v2.0.0）
4. crate 级 README 允许保留（Rust 生态 crate 自描述是最佳实践），但 **crate README 不得定义重复的全局概念** — 只描述 crate 自身 API/用法；全局规则一律用节号引用总纲

### 后果

- 任何在非主文档中「重新定义」现有全局概念的 PR，直接打回修改为引用
- 新增「全局概念」必须先提交 PR 修改总纲对应章节，获得架构委员会 sign-off 后方可生效

---

## 后续决策待办（未编号）

| 提案号 | 议题 | 预计决策日期 |
|---|---|---|
| ADR-007 | Enterprise 3002 路由合并入 Gateway 8080（去独立端口） | 2026-09-03 |
| ADR-008 | 图谱算法 SQLite/PG 存储实桥接（demo→真实生产数据） | 2026-09-10 |
| ADR-009 | RBAC JWT AuthLayer 集中网关集成（当前全接口匿名访问） | 2026-09-03 |
| ADR-010 | 项目配置 `platform_config.json` 统一真相源（禁止 SERVICE_DEFINITIONS 硬编码） | 2026-09-17 |

---

*本 ADR 文件本身受 once-defined 原则约束；每条 ADR 只在本文件出现，任何引用使用「ADR-XXX」编号。*
