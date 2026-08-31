// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 调度器应用状态

use std::sync::Arc;

use mox_alliance_scheduler_core::{RuleBasedExpertMatcher, TaskSchedulerImpl};
use tokio::sync::mpsc;

use mox_alliance_common_proto::Task;
use mox_alliance_scheduler_proto::types::SchedulerConfig;

/// 调度器应用状态（共享给所有 handler）
#[derive(Clone)]
pub struct SchedulerAppState {
    pub config: SchedulerConfig,
    pub scheduler: Arc<TaskSchedulerImpl>,
    pub matcher: Arc<RuleBasedExpertMatcher>,
    /// 任务派发通道（发给执行器）
    pub dispatch_tx: mpsc::UnboundedSender<Task>,
}

impl SchedulerAppState {
    pub fn new(
        config: SchedulerConfig,
        scheduler: Arc<TaskSchedulerImpl>,
        matcher: Arc<RuleBasedExpertMatcher>,
        dispatch_tx: mpsc::UnboundedSender<Task>,
    ) -> Self {
        Self {
            config,
            scheduler,
            matcher,
            dispatch_tx,
        }
    }
}
