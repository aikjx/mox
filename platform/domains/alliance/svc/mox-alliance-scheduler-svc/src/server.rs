// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 调度器服务器

use std::net::SocketAddr;
use std::sync::Arc;

use mox_alliance_common_proto::Task;
use mox_alliance_scheduler_core::{RuleBasedExpertMatcher, TaskSchedulerImpl};
use mox_alliance_scheduler_proto::types::SchedulerConfig;
use tokio::sync::mpsc;
use tracing::info;

use crate::app_state::SchedulerAppState;
use crate::routes::build_router;

/// 调度器服务器
pub struct SchedulerServer {
    config: SchedulerConfig,
    listen_addr: SocketAddr,
}

impl SchedulerServer {
    pub fn new(config: SchedulerConfig, listen_addr: SocketAddr) -> Self {
        Self {
            config,
            listen_addr,
        }
    }

    /// 启动服务器
    pub async fn run(&self) -> anyhow::Result<()> {
        // 初始化专家匹配器
        let matcher = Arc::new(RuleBasedExpertMatcher::new());

        // 初始化任务派发通道
        let (dispatch_tx, _dispatch_rx) = mpsc::unbounded_channel::<Task>();

        // 初始化调度器
        let scheduler = Arc::new(TaskSchedulerImpl::new(
            self.config.clone(),
            matcher.clone(),
            dispatch_tx.clone(),
        ));

        // 构建应用状态
        let state = SchedulerAppState::new(
            self.config.clone(),
            scheduler,
            matcher,
            dispatch_tx,
        );

        // 构建路由
        let app = build_router(state);

        info!(
            "Scheduler server starting on {}",
            self.listen_addr
        );

        // 启动服务
        let listener = tokio::net::TcpListener::bind(self.listen_addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
