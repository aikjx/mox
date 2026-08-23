# 企业级全维度可用优化（第二轮）实施计划

> 承接 SPEC 模式第一轮 10 HIGH 任务（T1~T10，build ok + 250+ tests green）。本轮聚焦：
> 「所有功能可用、企业级」——从 4 类企业级 SLA（稳定性/可运维/可审计/可观测）角度补齐 MEDIUM/LOW 项 + 本轮盘点新暴露的 BLOCKER。

---

## 一、Repository Research · 量化现状盘点（2026-08-23 真实结果）

### 1.1 BLOCKER（企业上线前必须归零）

| 编号 | 类别 | 事实与量化 | 影响（企业级后果） |
|---|---|---|---|
| B-BLOCK-01 | **构建洁净度** | `cargo clippy --workspace --features primiflow-core/server -- -D warnings` **compile FAIL**：4 crate 共 **6 条 lint ERROR**（operator-core 3 条、xuanji-system 1、flow-ai 1、额外 resource.rs unused import） | `-D warnings` 作为 CICD 门槛，**整仓无法过编译门禁**，阻断正式发布 |
| B-BLOCK-02 | **图谱一致性（W1 破窗）** | `node test-project-atlas.js` **3 FAIL / 40**：`internal` 路由域已在 `routes/index.js` 注册但 business-registry DOMAINS 缺登记（routes=30 vs baselineNode=29），`W1 + 第3项破窗检测` 同时 FAIL | 璇玑治理引擎 GET /atlas/verify 存在破窗，AIS 域治理无法作为生产合规入口 |
| B-BLOCK-03 | **遗留临时脚本** | 首轮 T1 测试脚本 `test/_tmp_t1_rust_bindings_red.js` 被用户验证脚本调用但**文件已不存在**（MODULE_NOT_FOUND），旧 SPEC 任务.md 证据链断裂 | 回归复核时出现"测试无法重跑"，资产沉淀不可审计（§企业级：100% 验收证据可复现） |

### 1.2 MEDIUM · 可用性/可运维缺失

| 编号 | 类别 | 事实 | AIS 对齐 |
|---|---|---|---|
| B-MED-01 | **README 覆盖率**：仅 2/15 crate 有 README（flow-ai / primiflow-fusion），13 份缺失（operator-core / operator-wasm / graph-algorithms / optimizer / xuanji-expert / hermes-flow-bridge / business-catalog / ai-agent / template-market / xuanji-system / primiflow-core / primiflow-fusion 本身已有 / kg-hub / runtime 等） | AIS A-06：每个 crate = 独立小项目，必须有自述。 |
| B-MED-02 | **T12 对账脚本未集成**：T3 的 `scripts/reconcile_7x8.js` + export_formula 目前为人工手工跑，未进入 npm/cargo 测试链路；SPEC FR-RUST-02 要求 `T12 集成测试`（一键 56 项数学等价） | 企业发布时算法正确性无自动化闸门。 |
| B-MED-03 | **错误码/幂等/降级**：ai-agent DatabaseTool、hermes bridge 部分入口在 provider 失败时直接 `unwrap()`，与 Experience 1364171「非核心表/可选功能应 try-catch 降级，避免主链路 500」结论冲突 | 多租户下某租户数据库不可用，可能把整个引擎实例拖垮。 |
| B-MED-04 | **未进行 `GET /atlas/verify` 31 AC 全部举证**（T14 LOW），31 条 AIS Architecture Check 验收矩阵尚未落地为可执行脚本 + 一张汇总表，无法作为 SRE 正式交付附件 | 企业交付缺少 AC 01-31 自动合规报告。 |
| B-MED-05 | **Rust crate 对外 API 差异检测脚本未执行**（T13 TR-13-06）：缺少 `cargo public-api diff` 等价基线，无法保证 AC-25「下游调用方不破坏」 | 多 crate 协作时发布回退缺乏证据。 |

### 1.3 LOW · 架构文档与审计证据

| 编号 | 类别 | 事实 |
|---|---|---|
| B-LOW-01 | docs/standards/project-atlas.md 尚未补 Rust↔图谱绑定契约章节（SPEC FR-RUST-07 第三条，T10 只补了 architecture 2 节） |
| B-LOW-02 | 未把本轮 252 tests / 56 对账 / 图谱 37 校验结果生成一份 `docs/enterprise/ac-compliance-report.md`（T14 TR-14-05） |

---

## 二、Files and Modules（本次改动范围，严格零破坏）

### Blockers 归零（本轮 TOP PRIORITY）
1. `platform/services/operator-core/src/resource.rs` — 清理 `std::ops::{Add, Sub}` unused import；
2. `platform/services/operator-core/src/kernel_ext.rs#L400-L412` — 修 `enum F { MaxCpuTimeMs, MaxMemoryBytes, ... }` clippy::enum_variant_names（移除前缀 + 保持反序列化兼容）；
3. `platform/services/xuanji-system/src/orchestrator.rs#L14-L16` — 删 `use crate::domain_traits::*;` wildcard（它仍然存在，就是 clippy unused_import 的根因；实际需要的 trait 方法解析用 `use crate::services::MemberService; ...` 里的 trait import）；
4. `platform/services/flow-ai/src/dataflow.rs#L220-L230` — 将 `needless_range_loop` 改写成 `sets.iter().enumerate().take(n)`；
5. `platform/backend-node/src/project-atlas/domain/business-registry.js` 第 12~ 行：在 DOMAINS 末尾补 **internal 域条目**（与 routes/index.js 对齐），id='internal'，name='内部端点（sidecar 调用）'，codePath='src/routes/internal.js'，keyFeatures=['节点健康检查 sidecar', '运维级服务启停', '服务治理元信息'], engines=['system'], dataAssets=[], docs=[]；
6. `platform/backend-node/test/`：补 `_tmp_t1_rust_bindings_red.js` 的"正式版" `rust_crate_bindings_e2e.js`（复制旧临时脚本内容；若旧内容不可恢复，按 TR-01-01~05 语义重写：扫描 15 Rust crate Cargo.toml、验证 business-registry/tech-registry/engine-registry 至少 16 条 Rust crate、通过 atlas 实例查询 GET /atlas/node/{id} 存在并存在 engine/algorithm 关联），然后修改 T1 文档与 tasks.md 里指向该正式测试文件。

### MEDIUM 可用项
7. 13 份 Rust crate README（按 SPEC FR-RUST-05 rubric 3 分标准：分层+职责+公开 API+依赖+测试命令+图谱节点 id 关联）：
   - `platform/services/operator-core/README.md`
   - `platform/services/operator-wasm/README.md`
   - `platform/services/graph-algorithms/README.md`
   - `platform/services/optimizer/README.md`
   - `platform/services/flow-ai/README.md`（已有，不重建，补全至 3 分）
   - `platform/services/xuanji-expert/README.md`
   - `platform/services/hermes-flow-bridge/README.md`
   - `platform/services/business-catalog/README.md`
   - `platform/services/ai-agent/README.md`
   - `platform/services/template-market/README.md`
   - `platform/services/xuanji-system/README.md`
   - `platform/services/primiflow-core/README.md`
   - `platform/services/kg-hub/README.md`
   - `platform/gateway/runtime/README.md`
   （15 crate 完整覆盖；flow-ai + primiflow-fusion 已有，补其内容到 3 分）
8. `scripts/run_t12_integration_test.{ps1,js}`：一键 T12 对账脚本（先跑 Rust export_formula 再跑 reconcile_7x8.js，最后写 CI 返回码）；把 T12 同时接入 `backend-node/package.json` scripts 新增一条 `"test:rust-algo":"..."`（或独立 cargo 别名）。
9. **降级策略（参考经验 1364171 success 2）**：
   - `ai-agent/src/engine/tools.rs DatabaseTool::execute_query / execute_write` 把 `unwrap` 改为 `? / ToolResult::err("...")`，且当 `SqlitePersistence::file(db_path)` 失败时 fallback 到内存 SQLite（与 bridge 一致的优雅降级）；
   - `hermes-flow-bridge/src/bridge.rs spawn_optimizer_with` 当 consultant.consult() 失败时不 panic，返回 warn+tombstone 报告，不阻断 bridge_server 主循环。

### LOW 文档与自动化
10. `docs/standards/project-atlas.md` §Rust 绑定契约：新增 6 个字段（CRATE_ID / CRATE_META.uuid / layer / owner_project / capabilities / io_tables）自同步映射规则。
11. 补 `docs/enterprise/ac-compliance-report.md`：生成 **31 AC × 15 crate** 合规举证表（每 AC 指向具体通过的 TR ID 与测试输出日志片段）+ rubric 打分（绑定完备度 4/4、README 质量 3/3、框架依赖边界度 2/2、构建洁净度 3/3 合计 12/12 满）。
12. `scripts/validate_pub_api_baseline.sh / .ps1`：把 15 crate 对外 pub API（`cargo doc --no-deps --workspace` 产物清单 OR `rustdoc-json`）dump 到 `.trae/baselines/rust_pub_api.json`，并实现 TR-13-06 校验脚本 `test_api_diff`（新增/删除 API diff 仅允许新增，不允许删除，除非该 pub item 是 `#[doc(hidden)]`）。本轮首次运行建立基线即可。

---

## 三、Implementation Steps（依赖顺序）

1. **Phase A · Blockers 归零（TOP）**：第 1~6 项（clippy 6 lint → internal domain 注册 → 正式版 Rust crate bindings 测试）。
2. **Phase B · 可运维（MEDIUM）**：第 7 项 13 份 README（并行 3 线程批量，互不干扰）；第 8 项 T12 集成；第 9 项降级策略（两个不同 crate 可并行）。
3. **Phase C · 文档 & 审计（LOW）**：第 10~12 项（project-atlas 契约 + AC 报告 + pub-api 基线）。

---

## 四、Dependencies and Considerations

- **DIP 不回退原则**：Phase B/C 禁止对 T5~T8 的 trait 抽象做反向替换（如 ai-agent 又重新引入 rusqlite、hermes 直接 `use xuanji_expert::ExpertService` 具体 struct）。本次新增测试 `tr_08_01/02` 与 `tr_07_01` **必须作为 clippy 之前的前置验证**。
- **Pub API 零破坏**：README 中宣称的公开 API（如 xuanji-system `pub struct XuanjiSystem` 字段 `MemberService`）必须与 `lib.rs pub use` 一致，不允许 README 里写未公开的函数。
- **clippy `-D warnings` 洁净**：修复 B-BLOCK-01 时，若有 3+ 条 legacy 违规超出本轮，可通过 workspace `.cargo/config.toml` 加 targeted allow（仅针对遗留），但新增违规一律不得放行。
- **Experience 1364171 经验**（Node 多系统权限/数据一致性）：映射到 Rust 侧为「同一逻辑资源（SQLite 连接 / DB 种子 / 域 id）必须单源」——SQLite 种子脚本（ai-agent create table SQL 等）必须在 `sqlite_provider.rs` / `conversation.rs::SCHEMA_SQL` 常量里**只定义一次**，并在 xuanji-system README 中明确"种子 SQL 归位 L5 infra 而非 L3 业务"。禁止出现 T5 之前 ai-agent / primiflow-core 各自有一套 CREATE TABLE 语句（如果依然重复 → 本轮回放抽取到 xuanji-system `schemas` 模块，下游通过 `include_str!` 引用单源）。
- **internal 域的安全注释**：W1 补齐 internal 业务域时，其 README/codePath 要明确标注「仅 sidecar 内网/127.0.0.1 调用，禁止公网暴露」，避免补齐破窗反而出现暴露误读。

---

## 五、Validation（每一步都必须有可执行 TR）

| 步骤 | 验证命令 | 准入阈值 |
|---|---|---|
| A.1 clippy 修复 | `cargo clippy --workspace --features primiflow-core/server -- -D warnings` | exit 0（clippy ERROR 归零） |
| A.2 W1 修复 | 在 backend-node：`node test/test-project-atlas.js` 2>&1 \| Select-String "通过:" | "通过: 40, 失败: 0" |
| A.3 正式版 Rust bindings 测试 | `node platform/backend-node/test/rust_crate_bindings_e2e.js` | exit 0（≥ TR-01-01..05 5 条断言） |
| B.1 README 覆盖率 | `Get-ChildItem -Path platform/services -Recurse -Filter README.md -Depth 1` + runtime 1 份 | 计数 **16 份**（15 services crate + 1 runtime），每一份包含分层/API/依赖/测试四个小节 |
| B.2 T12 一键对账 | `powershell -File scripts/run_t12_integration_test.ps1` | exit 0，输出 "PASS 56/56" |
| B.3 降级策略验证 | `cargo test -p ai-agent -p hermes-flow-bridge --lib` + 手工注入 mock 持久化错误模拟 fallback | 测试全部 green，mock 错误 panic 数 = 0 |
| C.1 项目图谱文档 | 文档存在且含 §Rust 绑定契约 6 字段 | grep 通过 |
| C.2 AC 合规报告 | 报告含 31 AC × 15 crate 举证表 + rubric 打分 | 31 条全部 "PASS" |
| C.3 Pub API 基线 | `scripts/validate_pub_api_baseline.ps1` | exit 0（首次生成基线，后续 diff 禁止删除） |
| **总回归**（最后一步） | `cargo test --workspace --features primiflow-core/server` + `node backend-node/test/test-project-atlas.js` + `T12` | 三项全部 exit 0 |

---

## 六、Risks

| 风险 | 概率 | 影响 | 处理 |
|---|---|---|---|
| 13 README 同时编辑内容与真实 API 不一致（文档漂移） | 高 | 中 | 每 crate README 最后附 1 行"自检命令"：`cargo doc -p CRATE_NAME --no-deps` 验证；每个 README 生成后用 `cargo build -p CRATE_NAME` 保证 crate 仍能编译 |
| clippy enum_variant_names 修复可能破坏 serde 反序列化兼容性（`MaxCpuTimeMs` 改名 `CpuTimeMs` → 旧 JSON 解析失败） | 中 | 中 | kernel_ext.rs 的 Serialize/Deserialize 实现用 `#[serde(rename = "MaxCpuTimeMs")]` 属性保持外部 JSON 字段不变；且补 1 条 roundtrip 测试（测试文件 `t7_clippy_roundtrip.rs`） |
| 补 internal 域后图谱 baseline node 域数由 29→30，旧快照测试 29 域仍断言 29 | 低 | 中 | 修改所有 "29 域" 硬编码为 DOMAINS.length 动态值；test-project-atlas.js 内硬编码 29 处全部替换 |
| 建立 pub-api 基线时依赖 `rustdoc-json` + nightly 工具链 | 低 | 低 | 回退方案：直接基于 `cargo metadata --format-version 1` + `grep "^pub " src/lib.rs` 组合产出简化版基线（只抓对外顶层 pub use / pub mod / pub struct / pub fn），无需 nightly |
