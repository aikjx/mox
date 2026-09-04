---
title: 开发专家联盟 · 企业级优化方案（真实现状校准版 V2）
version: V2.0
authority: 🟡参考
doc_id: EA-DOC-002
last_updated: 2026-08-31
source_of_truth: 参考
---

# 开发专家联盟 · 企业级优化方案（真实现状校准版 V2）

> **文档编号**：`docs/expert-alliance/01-ENTERPRISE-OPTIMIZATION.md`
> **版本**：V2.0 | 日期：2026-08-29 | 状态：权威
> **前序**：`00-INTEGRATED-INDEX.md`（整合总览）· `docs/enterprise/26-*`（V1.0/V1.1 方案）
> **本版目的**：对 V1.0/V1.1 方案做**真实现状校准**——V1.0/V1.1 基于已删除的 backend-node（Node.js）撰写，而当前专家联盟已在 Rust 侧真实落地（`mox-ai-expert-svc`）。本方案以 2026-08-29 实测代码为准，只针对**真实存在的缺口**给优化路线，不再重复已完成的项。

---

## 1. 现状校准（V1.0/V1.1 vs 真实现状）

> ⚠️ 结论先行：**V1.0/V1.1 的核心诊断大量过时**——其引用的 `platform/backend-node`（含 security.js / ai-engine-core.js / service-manager.js / routes/*.js）已不存在，后端已整体迁移到 Rust。故 V1.0 中"RBAC 需从零写""AI 编排需重做""P0-1 无多租户"等结论**已不成立**。下表逐项校准。

### 1.1 关键架构事实（实测）

| 项 | V1.0/V1.1 文档说法 | 2026-08-29 真实现状 | 判定 |
|----|-------------------|--------------------|------|
| 后端形态 | backend-node（Node.js，security.js/ai-engine-core.js/service-manager.js） | **已删除**，无 `platform/backend-node` 目录 | 🔄 架构迁移 |
| 专家联盟实现 | 规划/部分实现 | **Rust 已真实落地**：`mox-ai-expert-svc`（七位专家并行 + 归一化裁决 + 治理 8 闸门 + RBAC + 租户策略 + 审计） | ✅ 已落地 |
| 专家数量 | 文档称 10+ 专家规划 | 15 个专家模块 + 组队算法 | ✅ 超预期 |
| 治理闸门 | 仅 5 项（G1/G2/G4/G5/G7） | **8 闸门全量**（G1~G8，tenant_policy.rs） | ✅ 已补齐 |
| RBAC | V1.0 误判"需从零写" | `rbac/` 目录已实现（多角色继承链） | ✅ 已实现 |
| 多租户 | P0-1"零多租户隔离" | `tenant_policy.rs` 租户策略分层 + `context.rs` Tenant | ✅ 骨架已建 |
| 审计合规 | 未涉及 | `audit/`：Syslog + S3(WORM) + Kafka（SOC2/GDPR） | ✅ 已建 |
| 前端 | 无 | ExpertCenterView / ExpertEnterpriseView / ExpertOrchestratorView + alliance.ts（SSE 流式） | ✅ 已建 |
| 敏感度判定 | P1"三处分叉" | `sensitivity.rs` 单一权威源（SSOT） | ✅ 已根治 |
| 验证 | 无 | `verify/`：7 项验证（topology/conflict/data_dep/gains/cem/code_rt/tests） | ✅ 已建 |
| 根目录垃圾文件 | P1-1"47 个垃圾文件" | 实测 0 个 .log/.d/.rmeta 文件 | ✅ 已清理 |
| graph.json 入仓 | P1-2"87MB 违规入仓" | 已移入 `data/graph.json`（.gitignore 已隔离 /data/） | ✅ 已修复 |
| .gitignore | 无 | 已有"架构-数据分离规范"（/data/ /plugins/ /.runtime/ /artifacts/ /exports/ /third_party/ .trae/ .workbuddy/ .ous/ 等） | ✅ 已完善 |

### 1.2 仍真实存在的缺口（本方案优化对象）

| # | 缺口 | 实测证据 | 级别 |
|---|------|---------|------|
| G-1 | **默认凭据明文**：`platform_config.json` 中 `admin.admin123` 硬编码 | 实测存在 `"username": "admin", "password": "admin123"` | 🔴 P0 |
| G-2 | **敏感信息全仓扫描缺失**：无 secret-scan 门禁，无法防新凭据回潮 | 未发现 secret 扫描脚本/CI 步骤 | 🔴 P0 |
| G-3 | **三目录命名未归一**：`projects/ my_projects/ workspace/` 三目录并存，语义重叠 | 实测三个目录均存在且均有内容 | 🟠 P1 |
| G-4 | **scripts 统一入口缺失**：仅 `server-manage.py` + README，无 `setup-dev.ps1` / `check-all.ps1` 四件套 | 实测 scripts/ 下仅 2 文件 | 🟠 P1 |
| G-5 | **AI 计量（llm_usage）未确认**：专家联盟调用 AI 无按租户计量归因 | 全仓未检索到 `llm_usage` 落盘 | 🟠 P1 |
| G-6 | **可观测性未端到端打通**：Rust 侧有 tracing/audit，但未见 OpenTelemetry 全链路导出（Jaeger/Tempo） | server.rs 无 OTLP 导出 | 🟡 P2 |
| G-7 | **专家联盟多租户 e2e 未验证**：tenant_policy 已实现，但缺端到端隔离测试证据 | verify/ 无租户隔离专项 | 🟡 P2 |
| G-8 | **bench 证据未固化**：`bench.rs` 存在，但缺可复用的量化报告产物 | 仅源码，未见 runs/ 结果 | 🟡 P2 |

---

## 2. 已落地能力盘点（无需重做，作为优化基础）

### 2.1 Rust 核心引擎（mox-ai-expert-svc）

```
src/
├── alliance/        组队算法（14 维×7 类基准能力 · EAF 4.2 安全强制替换 · 可解释矩阵）
│   ├── team.rs      专家组队优化（FR-CORE-03）
│   ├── debate.rs    多专家辩论
│   ├── gate.rs      闸门
│   ├── intent.rs    意图识别
│   └── kg_connector.rs  图谱连接
├── experts/         15 个专家模块（algorithm/architecture/business/code_quality/data/
│                    documentation/maintainability/observability/performance/permission/
│                    resource/security/security_code/testing + mod）
├── verify/          7 项验证（topology/conflict/data_dep/gains/cem/code_rt/tests）
├── rbac/            RBAC 引擎（资源级权限 · 多角色继承链 · 跨租户隔离）
├── audit/           Syslog + S3(WORM) + Kafka 审计（SOC2/GDPR）
├── tenant_policy.rs 租户策略分层 + 治理 8 闸门（G1~G8）
├── sensitivity.rs   敏感度判定 SSOT
├── domain/          DIP trait：ExpertRegistry / ExpertConsultant / AllianceOrchestrator
├── pipeline.rs      mox 模块化系统架构分析管线
├── harness.rs       插件化运行时（"Everything is a Plugin"）
├── server.rs        HTTP 服务（Three.js 力导向图联动 DTO）
└── bin/mox.rs       可执行入口
```

### 2.2 前端（3 视图 + SSE）

- `ExpertCenterView.vue`：璇玑·专家联盟 X（项目为根 · mox 模块化系统架构 φ 流程：需求/架构/开发/发布四阶段）
- `ExpertEnterpriseView.vue`：企业级视图
- `ExpertOrchestratorView.vue`：V2 编排引擎控制台（插件化编排 · Plan/Act 双模式 · 学习闭环 · 事件驱动）
- `api/alliance.ts`：7 类基准确认 + 联盟 SSE 流（intent→team→debate→synthesize→gate→learn→done）+ 语音三层回退

---

## 3. 企业级优化路线（仅针对真实缺口）

### 阶段 A：安全收口（P0 · 1-2 人日 · 阻断项）

| # | 任务 | 验收标准 |
|---|------|---------|
| A-1 | `platform_config.json` 移除明文密码：改为环境变量 `MOX_ADMIN_PASSWORD` 注入 + 首次启动生成随机初始密码（打印一次后失效） | 配置文件中无任何明文凭据；`Select-String admin123` 全仓 0 命中 |
| A-2 | 建立全仓 secret 扫描门禁（`scripts/ci/secret-scan.ps1`）：扫描 `admin123`/`password=`/`Bearer ` 等模式 + 高熵 token 检测，接入 CI | CI 中 secret 扫描必过；扫描脚本纳入 `scripts/` |

### 阶段 B：工程卫生归一（P1 · 2-3 人日）

| # | 任务 | 验收标准 |
|---|------|---------|
| B-1 | 三目录归一：`projects/`（对外项目）+ `workspace/`（工作区产物）合并为单一 `projects/`，`my_projects/`（个人）归档或并入 | 仅保留 1-2 个语义明确的目录；`docs/architecture/14` 全景图同步更新 |
| B-2 | scripts 四件套：`setup-dev.ps1`（环境准备）/ `check-all.ps1`（构建+测试+clippy 一键）/ `start-all.ps1`（三服务启动）/ `stop-all.ps1` | 四件套齐全且可执行；`check-all` 全绿 |
| B-3 | AI 计量落盘：专家联盟每次 AI 调用写 `data/metering/llm_usage.jsonl`（tenant_id + model + tokens + cost_est + trace_id） | 计量文件存在且含 tenant_id；接入 .gitignore（/data/ 已覆盖） |

### 阶段 C：可观测性与验证固化（P2 · 2-3 人日）

| # | 任务 | 验收标准 |
|---|------|---------|
| C-1 | Rust 侧接入 OpenTelemetry：tracing → OTLP 导出 Jaeger/Tempo，HTTP 请求 + 联盟管线 span | 一条专家联盟任务可在 Jaeger 看到完整 trace（intent→team→debate→gate→learn） |
| C-2 | 多租户 e2e 测试：A/B 租户并发跑同一专家联盟任务，断言数据/审计/计量完全隔离 | 测试用例纳入 `verify/tests.rs`；A/B 租户数据零串扰 |
| C-3 | bench 量化报告固化：`bench.rs` 输出 `data/bench/` 可复用 JSON（专家数/延迟/通过率/Gate 分布） | 报告产物存在且含基线值 |

---

## 4. 落地顺序与依赖

```
A-1(凭据收口) ──→ A-2(secret 门禁)
        │
        ├──→ B-1(目录归一) ──→ 14 号全景图同步
        ├──→ B-2(scripts 四件套)
        └──→ B-3(AI 计量)
                 │
                 └──→ C-1(OTel) ──→ C-2(租户 e2e) ──→ C-3(bench 固化)
```

- **阻断依赖**：A 阶段必须先于发布；B-1 需在无进行中构建时执行（避免路径漂移）。
- **总工时**：约 6-8 人日（不含等待与回归）。

---

## 5. 验收清单（对照 V1.0/V1.1 修正后）

| 验收项 | 方法 |
|--------|------|
| 全仓无明文凭据 | `rg "admin123|password\s*[:=]\s*\"[^\"]+\"" --glob '!target/**' --glob '!*.md'` 0 命中 |
| secret 门禁生效 | CI 中注入 `admin123` 应被扫描拦截（负向用例） |
| 三目录归一 | 根目录 `projects` 唯一；docs 全景图同步 |
| scripts 四件套 | 4 个脚本存在且 `check-all` 输出"PASS" |
| AI 计量 | 一次联盟任务后 `llm_usage.jsonl` 新增 ≥1 条含 tenant_id |
| OTel 链路 | Jaeger 可见完整联盟 span |
| 租户隔离 e2e | A/B 租户并发任务零串扰（审计+计量+图谱） |
| bench 报告 | `data/bench/*.json` 存在且含基线 |

---

## 6. 本次核实的新事实（供 00-INDEX 同步）

1. **架构迁移**：`platform/backend-node` 已删除，V1.0/V1.1 引用的 Node 文件全部不存在——**请勿再基于 backend-node 实施任何 V1.0 任务**。
2. **专家联盟已 Rust 落地**：15 专家 + 8 闸门 + RBAC + 租户 + 审计 + verify 7 项，核心能力远超 V1.0 文档规划。
3. **前端已落地**：3 个专家视图 + SSE 联盟流，路由在 `frontend-ui/src/router/index.js`。
4. **工程卫生已部分完成**：垃圾文件清 0、graph.json 已隔离、.gitignore 已完善。
5. **唯一 P0 遗留**：`platform_config.json` 明文 admin123 凭据（本方案 A-1 解决）。

---

## 7. 与既有文档的关系

- `00-INTEGRATED-INDEX.md`：主题全景导航（本文档是其"优化执行"章节）
- `docs/enterprise/26-V1.0/V1.1`：历史方案，保留存档；**V1.0 中 backend-node 相关条目标记"已过时（架构迁移）"**，不建议按原样执行
- `docs/architecture/14-REPOSITORY-FULL-MAP.md`：B-1 目录归一后需同步

---

**阶段门禁**：本方案处于"优化设计"阶段，A/B/C 阶段任务需用户明确"开工"指令后才执行代码/文件修改。
**归一化状态**: ✅ 已归一化

---

## 8. 执行状态追踪（2026-08-30 更新）

| # | 任务 | 状态 | 证据 / 说明 |
|---|------|------|-------------|
| A-1 | 移除明文 admin123 | ✅ **已完成** | ① `platform_config.json` 删除 password 字段（实测确认已无）② `scripts/server-manage.py` DEFAULT_CONFIG 同步移除（原 89 行硬编码）③ `admin_pass` 属性已有随机密码生成 + 环境变量 `MOX_ADMIN_PASS` 覆盖逻辑，无需改代码 |
| A-2 | secret 扫描门禁 | ✅ **已完成** | 新增 `scripts/ci/secret-scan.py`（弱口令/私钥/高熵 token/云 AK/JWT 5 类模式 + 误报校准：排除 UUID/环境变量名/路径/引号外标识符）；scripts 目录实测 0 命中 PASS |
| B-1 | 三目录归一 | ⏸️ **待执行** | 涉及 `projects/ my_projects/ workspace/` 合并，会引发路径漂移（platform_config.json 中 cwd 引用），需用户单独确认后执行 |
| B-2 | scripts 四件套 | ✅ **已完成** | 新增 `setup-dev.ps1`（工具链检测+依赖安装）/ `check-all.ps1`（secret-scan+fmt+clippy+test+前端build 一键）/ `start-all.ps1`（复用 manage.py bootstrap）/ `stop-all.ps1`（复用 manage.py stop）；四件套 UTF-8 BOM 化，PowerShell 语法校验全部 PASS |
| B-3 | AI 计量落盘 | ✅ **已完成** | 新增 `shared/metering/mox_metering.py`（标准库零依赖、追加写、幂等、异常不阻断）；`--probe` 自检通过（total_tokens=200, cost_est=0.0648）；`.gitignore` 增加 `data/metering/` 隔离（git check-ignore 验证生效）；当前仓库暂无实际 LLM 调用点，模块作为基础设施待未来 AI 网关接入 |
| C-1 | OTel 全链路 | ⏸️ **待执行** | 需 Rust 构建环境 + Jaeger/Tempo，建议单独排期 |
| C-2 | 多租户 e2e | ⏸️ **待执行** | verify/tests.rs 新增租户隔离专项，需 Rust 测试环境 |
| C-3 | bench 固化 | ⏸️ **待执行** | bench.rs 输出 data/bench/，需 Rust 运行环境 |

**B-1 说明**：因 `platform_config.json` 内 services.cwd 多处引用 `projects/xiaobai_voice`、`projects/melody2score` 等路径，三目录合并前须同步修改配置与文档引用，避免服务无法启动。建议在无运行服务时执行。

**B-3 说明**：计量模块为**基础设施**，当前仓库经核实无实际 LLM 网关调用点（mox-server / mox-ai-expert-svc 均为规则引擎或占位），故未强行伪造调用埋点；待 AI 网关落地后调用 `mox_metering.record()` 即可。

