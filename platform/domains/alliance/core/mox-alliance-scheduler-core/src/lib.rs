// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Scheduler Core — 调度器核心
//!
//! 调度器的核心业务逻辑实现：
//! - 任务排队与调度
//! - 专家匹配（基于规则的简单匹配）
//! - 协作计划生成
//!
//! ## 设计原则
//! - 依赖 proto 层的 trait 抽象（DIP）
//! - 核心逻辑无状态，状态通过 trait 接口外部化
//! - 可测试：所有核心算法都有对应的单测

pub mod matcher;
pub mod planner;
pub mod scheduler;

pub use matcher::RuleBasedExpertMatcher;
pub use planner::SimplePlanGenerator;
pub use scheduler::TaskSchedulerImpl;
