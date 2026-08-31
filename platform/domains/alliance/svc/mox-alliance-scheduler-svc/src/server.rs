// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 调度器服务器

use std::net::SocketAddr;
use std::sync::Arc;

use mox_alliance_common_proto::Task;
use mox_alliance_scheduler_core::{
    HttpExecutorBridge, HttpExecutorBridgeConfig, RuleBasedExpertMatcher, TaskSchedulerImpl,
};
use mox_alliance_scheduler_proto::types::SchedulerConfig;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::app_state::SchedulerAppState;
use crate::routes::build_router;

/// 调度器运行模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerMode {
    /// 独立模式（通过 HTTP 调用远程执行器）
    Standalone,
    /// 内嵌模式（执行器在同进程内，进程内调用）
    Embedded,
}

/// 调度器服务器
pub struct SchedulerServer {
    config: SchedulerConfig,
    listen_addr: SocketAddr,
    mode: SchedulerMode,
    /// 执行器服务地址（Standalone 模式下使用）
    executor_url: Option<String>,
}

impl SchedulerServer {
    pub fn new(config: SchedulerConfig, listen_addr: SocketAddr) -> Self {
        Self {
            config,
            listen_addr,
            mode: SchedulerMode::Standalone,
            executor_url: None,
        }
    }

    /// 设置运行模式
    pub fn with_mode(mut self, mode: SchedulerMode) -> Self {
        self.mode = mode;
        self
    }

    /// 设置执行器服务地址
    pub fn with_executor_url(mut self, url: impl Into<String>) -> Self {
        self.executor_url = Some(url.into());
        self
    }

    /// 创建执行器桥接
    fn create_executor_bridge(
        &self,
    ) -> anyhow::Result<Arc<dyn mox_alliance_scheduler_core::ExecutorBridge>> {
        match self.mode {
            SchedulerMode::Standalone => {
                let base_url = self
                    .executor_url
                    .clone()
                    .unwrap_or_else(|| "http://localhost:8081".to_string());

                let config = HttpExecutorBridgeConfig {
                    base_url: base_url.clone(),
                    timeout_ms: 30_000,
                };

                let bridge = HttpExecutorBridge::new(config)?;
                info!("Using HTTP executor bridge at {}", base_url);
                Ok(Arc::new(bridge))
            }
            SchedulerMode::Embedded => {
                // Embedded 模式需要外部传入 DagEngine
                // 这里返回一个占位，实际使用时应该通过其他方式设置
                warn!("Embedded mode requires in-process DagEngine, using Noop bridge as placeholder");
                Ok(Arc::new(
                    mox_alliance_scheduler_core::NoopExecutorBridge,
                ))
            }
        }
    }

    /// 启动服务器
    pub async fn run(&self) -> anyhow::Result<()> {
        // 初始化专家匹配器
        let matcher = Arc::new(RuleBasedExpertMatcher::new());

        // 创建执行器桥接
        let executor_bridge = self.create_executor_bridge()?;

        // 初始化调度器（使用新的 bridge 方式）
        let scheduler = Arc::new(TaskSchedulerImpl::new_with_bridge(
            self.config.clone(),
            matcher.clone(),
            executor_bridge.clone(),
        ));

        // 构建应用状态
        let state = SchedulerAppState::new_with_bridge(
            self.config.clone(),
            scheduler,
            matcher,
            executor_bridge,
        );

        // 构建路由
        let app = build_router(state);

        info!(
            "Scheduler server starting on {} (mode: {:?})",
            self.listen_addr, self.mode
        );

        // 启动服务
        let listener = tokio::net::TcpListener::bind(self.listen_addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

// 保留旧的兼容性代码（不使用 bridge 的版本）
#[allow(dead_code)]
fn _create_legacy_scheduler(
    config: SchedulerConfig,
    matcher: Arc<RuleBasedExpertMatcher>,
) -> (Arc<TaskSchedulerImpl>, mpsc::UnboundedSender<Task>) {
    let (dispatch_tx, _dispatch_rx) = mpsc::unbounded_channel::<Task>();
    let scheduler = Arc::new(TaskSchedulerImpl::new(config, matcher, dispatch_tx.clone()));
    (scheduler, dispatch_tx)
}
