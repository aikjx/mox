# 架构结构化产物 — Architecture Assets

> **层定位**：L2 架构层 → 结构化数据。**结构化数据与文档分离**，避免 `.json` 与 `.md` 混放（结构规范 §2.2 / §4.5）。
> 上层入口：[架构层入口](../README.md) · [文档中心](../../README.md)

---

## 一、内容清单

| 文件 | 类型 | 说明 | 生产者 |
|------|------|------|--------|
| [`bench_results_round7.json`](./bench_results_round7.json) | 🟡 基准数据 | 联盟执行器 round7 基准结果（由 `platform/domains/alliance/core/mox-alliance-executor-core/tests/bench_alliance.rs` 产出） | 集成测试 |
| [`APPROVAL-FLOWS.json`](./APPROVAL-FLOWS.json) | 🟢 配置事实 | 审批流定义（流程编排的结构化描述） | 人工维护 |
| [`architecture-metrics.json`](./architecture-metrics.json) | 🟡 度量 | 架构度量指标（用于审计与可视化） | 脚本产出 |

## 二、约定

1. **一资产一来源**：每份产物必须在"生产者"列写清生成方式；人工维护的与脚本/测试产出的要区分。
2. **基准数据可复算**：`bench_*.json` 由测试/基准命令重新生成，不手工编辑。
3. **版本策略**：轮次产物保留文件名中的轮次号（`round7`）；被取代时移入 [`../../_archive/`](../../_archive/)。
4. 引用路径统一为 `docs/architecture/assets/<file>`。
