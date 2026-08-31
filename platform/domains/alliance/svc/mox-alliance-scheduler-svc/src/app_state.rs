// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 调度器应用状态

use std::sync::Arc;

use mox_alliance_scheduler_core::{RuleBasedExpertMatcher, TaskSchedulerImpl};
use mox_alliance_scheduler_core::ExecutorBridge;
use tokio::sync::mpsc;

use mox_alliance_common_proto::Task;
use mox_alliance_scheduler_proto::types::SchedulerConfig;

/// 调度器应用状态（共享给所有 handler）
#[derive(Clone)]
pub struct SchedulerAppState {
    pub config: SchedulerConfig,
    pub scheduler: Arc<TaskSchedulerImpl>,
    pub matcher: Arc<RuleBasedExpertMatcher>,
    /// 执行器桥接
    pub executor_bridge: Arc<dyn ExecutorBridge>,
    /// 任务派发通道（保留，向后兼容）
    pub dispatch_tx: mpsc::UnboundedSender<Task>,
}

impl SchedulerAppState {
    /// 创建应用状态（使用 ExecutorBridge，推荐）
    pub fn new_with_bridge(
        config: SchedulerConfig,
        scheduler: Arc<TaskSchedulerImpl>,
        matcher: Arc<RuleBasedExpertMatcher>,
        executor_bridge: Arc<dyn ExecutorBridge>,
    ) -> Self {
        // 创建一个空的 dispatch_tx 用于向后兼容
        let (dispatch_tx, _dispatch_rx) = mpsc::unbounded_channel::<Task>();

        Self {
            config,
            scheduler,
            matcher,
            executor_bridge,
            dispatch_tx,
        }
    }

    /// 创建应用状态（旧版 API，向后兼容）
    pub fn new(
        config: SchedulerConfig,
        scheduler: Arc<TaskSchedulerImpl>,
        matcher: Arc<RuleBasedExpertMatcher>,
        dispatch_tx: mpsc::UnboundedSender<Task>,
    ) -> Self {
        // 从 scheduler 获取 bridge（如果是旧版构造的，就是 NoopExecutorBridge）
        let executor_bridge = scheduler.executor_bridge().clone();

        Self {
            config,
            scheduler,
            matcher,
            executor_bridge,
            dispatch_tx,
        }
    }
}
