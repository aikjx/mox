// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # mox-codeengine-core —— 全自研 AI 代码引擎
//!
//! 以「开发专家联盟处理模式」组织的端到端代码引擎：**零外部 AI 服务依赖**，
//! 全部推理为自研确定性算法（规则专家 + 加权裁决 + 三证闸门）。
//!
//! 处理模式八阶段（对齐 `docs/expert-alliance/README.md`）：
//!
//! | 阶段 | 联盟语义 | 实现 |
//! |------|---------|------|
//! | Intent    | 请求理解   | [`ir`] 自然语言需求 → 结构化 IR |
//! | Build     | 任务建图   | [`graph`] IR → FlowGraph（自动补护栏） |
//! | Solve     | 求解编排   | 复用 mox-ai-flow-core 六阶段流水线（CPM/RCPSP） |
//! | Team      | 专家匹配   | [`experts`] analyst / builder / auditor / coordinator |
//! | Diagnose  | 并行诊断   | 四专家各自独立出具意见 |
//! | Verdict   | 裁决融合   | [`verdict`] 加权投票（auditor 一票否决） |
//! | Gate      | 质量闸门   | [`gate`] 拓扑/数据/往返三证 + 收益核查 |
//! | Refine    | 自修复     | [`repair`] 诊断意见 → 图突变 → 重入评审轮（code agent 式收敛循环） |
//! | Deliver   | 出码交付   | 分层工程代码 + DDL + 前端骨架；[`patch`] diff-first SEARCH/REPLACE 增量补丁 |
//! | Learn     | 经验沉淀   | [`case`] 案例库（JSONL 可导出） |
//!
//! ## 快速使用
//!
//! ```
//! use mox_codeengine_core::CodeEngine;
//!
//! let mut engine = CodeEngine::default();
//! let result = engine.run(
//!     "读取 input.csv 文件；把解析结果写入 db.orders 表；导出报表 output.xlsx");
//! assert!(result.delivered, "干净需求应通过闸门交付: {:?}", result.gates);
//! assert!(result.verdict.approved);
//! ```

pub mod case;
pub mod experts;
pub mod gate;
pub mod graph;
pub mod ir;
pub mod patch;
pub mod repair;
pub mod engine;
pub mod verdict;

pub use engine::{CodeEngine, EngineConfig, EngineResult, Stage, StageEvent};
pub use experts::{ExpertOpinion, Finding};
pub use gate::{GateCheck, GateReport};
pub use ir::{parse_requirement, Requirement, StepSpec};
pub use patch::{EditBlock, PatchReport, apply_blocks, diff_against, make_block, parse_blocks};
pub use repair::{RepairOutcome, repair_graph};
pub use verdict::{Verdict, VerdictPolicy};

pub mod prelude {
    pub use crate::engine::{CodeEngine, EngineConfig, EngineResult};
    pub use crate::ir::{parse_requirement, Requirement};
    pub use crate::verdict::{Verdict, VerdictPolicy};
    pub use mox_ai_flow_core::prelude::*;
}
