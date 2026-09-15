# 报告与验证层入口 — Working Reports

> **层定位**：L7 报告与验证层。存放**过程产物与证据**（🟡 非权威）：分析报告、优化方案、验证原始输出、审计与基准数据。
> ⚠️ 本层文档**不得作为权威事实引用**；结论如已固化，请回填到 L1~L6 的 🟢 文档，本层仅作证据链留存。
> 上层入口：[文档中心](../README.md) · 结构规范：[ARCHITECTURE-OF-DOCS.md](../ARCHITECTURE-OF-DOCS.md)

---

## 一、子目录

| 子目录 | 内容 | 等级 |
|--------|------|:----:|
| [`verification/`](./verification/README.md) | 验证原始输出与验证报告（测试 stdout、verification-report） | 🟡 证据 |
| [`audits/`](./audits/README.md) | 就绪度评估与安全审计快照 | 🟡 证据 |

## 二、按类型索引（根目录）

### 命名规范（新增请遵循）

- 日期前缀：`<YYYYMMDD>_<主题>_<类型>.md`，类型取 `plan` / `report` / `verification_report` / `benchmark` / `governance`。
- 轮次报告：`<主题>-round<N>-<YYYYMMDD>.html`。
- 结构化数据：`<YYYYMMDD>_<主题>.json`。

### 阶段交付与验证报告

| 文档 | 日期 | 主题 |
|------|------|------|
| `20260902_hybrid_architecture_phase1_verification_report.md` | 2026-09-02 | 混合架构一阶段验证 |
| `20260902_hybrid_architecture_route_a_design.md` | 2026-09-02 | 混合架构 Route A 设计 |
| `20260903_hybrid_architecture_phase2_verification_report.md` | 2026-09-03 | 混合架构二阶段验证 |
| `20260903_hybrid_architecture_phase3_architecture_analysis.md` | 2026-09-03 | 混合架构三阶段架构分析 |
| `20260903_hybrid_architecture_phase3_verification_report.md` | 2026-09-03 | 混合架构三阶段验证 |
| `20260903_moxfs_phase4_architecture_decoupling.md` | 2026-09-03 | MOXFS 四阶段架构解耦 |
| `20260903_moxfs_phase4_verification_report.md` | 2026-09-03 | MOXFS 四阶段验证 |
| `20260903_moxfs_phase5_e2e_test_report.md` | 2026-09-03 | MOXFS 五阶段端到端测试 |
| `20260903_moxfs_phase5_performance_benchmark_report.md` | 2026-09-03 | MOXFS 五阶段性能基准 |
| `20260903_moxfs_phase5_quality_audit_report.md` | 2026-09-03 | MOXFS 五阶段质量审计 |
| `20260903_moxfs_phase5_test_coverage_report.md` | 2026-09-03 | MOXFS 五阶段测试覆盖 |
| `20260903_moxfs_phase5_verification_report.md` | 2026-09-03 | MOXFS 五阶段验证 |
| `20260903_moxfs_phase6_performance_optimization_report.md` | 2026-09-03 | MOXFS 六阶段性能优化 |
| `20260903_moxfs_phase6_stability_hardening_report.md` | 2026-09-03 | MOXFS 六阶段稳定性加固 |
| `20260903_moxfs_phase6_verification_report.md` | 2026-09-03 | MOXFS 六阶段验证 |
| `20260903_cloud_7crates_quality_hardening_report.md` | 2026-09-03 | Cloud 7 crates 质量加固 |
| `20260903_mox_cloud_kernel_test_coverage_report.md` | 2026-09-03 | Cloud Kernel 测试覆盖 |
| `20260905_alliance_modular_usable_acceptance.md` | 2026-09-05 | 联盟模块化可用性验收 |
| `enterprise-10task-acceptance-report.md` | 2026-08 | 企业级 10 任务验收 |

### 基准与性能

| 文档 | 主题 |
|------|------|
| `20260905_alliance_benchmark.json` | 联盟基准数据（🟡 数据） |
| `perf-algorithm-optimization-contrast-report_20260824-080732.md` | 算法优化对比（2026-08-24） |
| `perf-algorithm-data-structure-optimization_plan.md` | 算法与数据结构优化计划 |
| `alliance-performance-report-round7-20260901.html` | 联盟性能报告 round7 |
| `alliance-round8-llm-router-circuit-breaker-20260901.html` | 联盟 round8 · LLM 路由与熔断 |

### 专家联盟专项

| 文档 | 主题 |
|------|------|
| `expert-alliance-code-alignment-20260831.md` | 代码对齐 |
| `expert-alliance-doc-inventory-20260831.md` | 文档盘点 |
| `expert-alliance-normalization-report-20260831.md` | 归一化报告 |
| `expert-alliance-phase4-report-20260901.md` / `expert-alliance-phase5-report-20260901.md` | 四/五阶段报告 |
| `alliance-architecture-fix-report-20260831.html` / `alliance-architecture-review-20260831.html` | 架构修复与评审 |
| `alliance-verification-report-round3~6-*.html` | round3~6 验证报告 |
| `mox-expert-alliance-processing-mode.md` · `mox-algorithm-alliance-flow.md` | 处理模式与流程 |

### 归一化与治理

| 文档 | 主题 |
|------|------|
| `normalization-execution-groupA-20260831.md` / `groupB-20260831.md` / `archive-20260831.md` | 归一化执行分组与归档 |
| `server-manage-normalization-20260901.md` | 服务管理归一化 |
| `20260905_unified_baseline_dependency_governance.md` | 统一基线与依赖治理 |
| `20260906_module_contract_governance.md` | 模块契约治理 |
| `pub-api-baseline.md` · `rust-binding-contract.md` | 公共 API 基线、Rust 绑定契约 |
| `31-ac-compliance-report.md` | AC 合规报告 |

### 计划与调研

| 文档 | 主题 |
|------|------|
| `20260823_cloud_drive_and_relgraph_selfdev_plan.md` | 云盘与关系图自研计划 |
| `20260825_ai_actions_full_layout_plan.md` | AI Actions 全布局计划 |
| `service_startup_script_optimization_plan.md` | 服务启动脚本优化计划 |
| `enterprise-optimization-round-2_plan.md` · `enterprise-ux-optimization-plan.md` | 企业级二轮与 UX 优化 |
| `enterprise-optimal-business-flow.md` | 企业级最优业务流程 |
| `mox-vs-opensource-comparison-report.md` | 开源竞品对比 |
| `architecture-audit-report.txt` · `rust_migration_analysis.txt` | 架构审计、Rust 迁移分析（原始文本证据） |

---

## 三、归档规则

- 报告结论**固化后**：把可复用结论回填 L1~L6 权威文档，原报告留在本层作为证据。
- 报告被后续轮次取代时：移入 [`../_archive/`](../_archive/)，并在本页索引中标注被替代关系。
- 本层不做"重命名清理"式的批量整理，避免破坏历史引用；仅新增遵守命名规范。
