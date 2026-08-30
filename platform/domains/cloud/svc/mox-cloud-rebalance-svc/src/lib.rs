// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Cloud Drive L4 Rebalance Service
//!
//! Mox Cloud Drive Rebalance — 数据均衡与迁移编排服务
//!
//! 功能模块：
//! - `rebalance_controller` — 均衡控制器：监控、计划生成、执行调度
//! - `migration_task` — 迁移任务管理：生命周期、限速、断点续传、校验
//! - `placement_strategy` — 放置策略：容量/负载/拓扑多维评分选目标
//!
//! 设计参考 SeaweedFS 的 Volume 均衡架构和 Ceph CRUSH 的放置思想，
//! 支持千亿级文件规模的分布式数据均衡。

pub mod migration_task;
pub mod placement_strategy;
pub mod rebalance_controller;

pub use migration_task::{
    MigrationCheckpoint, MigrationPhase, MigrationStats, MigrationStatus, MigrationTask,
    MigrationTaskManager, MigrationType, VerificationMethod, VerificationResult,
};
pub use placement_strategy::{
    DataTemperature, PlacementCandidate, PlacementConstraints, PlacementEngine, PlacementNode,
    PlacementStrategyType, PlacementWeights, ScoreBreakdown,
};
pub use rebalance_controller::{
    MasterRebalanceResult, RebalanceConfig, RebalanceController, RebalancePlan,
    RebalanceState, RebalanceStats, RebalanceStatusSummary,
};
