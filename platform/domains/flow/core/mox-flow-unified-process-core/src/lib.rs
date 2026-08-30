// Copyright (c) 2026 璇玑 RelGraph · 流程算法归一化核心 (Unified Process & Algorithm Core)
// Licensed under the MIT License.

//! 流程算法归一化核心
//!
//! 深度融合：
//! - 算法联盟（算法注册/管道编排/计算引擎）
//! - 专家系统（规则引擎/决策表/推理引擎）
//! - 业务流程（统一流程定义/执行引擎/状态机）
//!
//! 提供统一的流程定义语言，支持：
//! - 算法步骤（调用算法联盟中的算法）
//! - 规则步骤（执行专家系统规则）
//! - 决策步骤（条件分支）
//! - 子流程步骤
//! - 人工审批步骤

pub mod error;
pub mod types;
pub mod rule_engine;
pub mod decision_table;
pub mod process_engine;
pub mod process_def;

pub use error::{ProcessError, ProcessResult};
pub use types::{
    ProcessStatus, StepType, StepStatus, ProcessContext, ProcessVariable,
    Rule, RuleCondition, RuleAction, Fact,
};
pub use rule_engine::RuleEngine;
pub use decision_table::DecisionTable;
pub use process_def::{ProcessDef, ProcessStep, DecisionBranch};
pub use process_engine::ProcessEngine;
