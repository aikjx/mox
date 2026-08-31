// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 执行器服务器

use std::net::SocketAddr;
use std::sync::Arc;

use mox_alliance_executor_core::{DagEngineImpl, MockNodeExecutor, MockExecutorConfig};
use mox_alliance_executor_proto::types::ExecutorConfig;
use tracing::info;

use crate::app_state::ExecutorAppState;
use crate::routes::build_router;

/// 执行器服务器
pub struct ExecutorServer {
    config: ExecutorConfig,
    listen_addr: SocketAddr,
}

impl ExecutorServer {
    pub fn new(config: ExecutorConfig, listen_addr: SocketAddr) -> Self {
        Self {
            config,
            listen_addr,
        }
    }

    /// 启动服务器
    pub async fn run(&self) -> anyhow::Result<()> {
        // 初始化 Mock 节点执行器（Phase 1）
        let mock_config = MockExecutorConfig {
            delay_ms: 50,
            success_rate: 1.0,
            generate_output: true,
        };
        let node_executor: Arc<dyn mox_alliance_executor_proto::NodeExecutor> =
            Arc::new(MockNodeExecutor::new(mock_config));

        // 创建 DAG 执行引擎并启动调度循环
        let engine = DagEngineImpl::spawn(self.config.clone(), node_executor);

        // 构建应用状态
        let state = ExecutorAppState::new(self.config.clone(), engine);

        // 构建路由
        let app = build_router(state);

        info!(
            "Executor server starting on {}",
            self.listen_addr
        );

        // 启动服务
        let listener = tokio::net::TcpListener::bind(self.listen_addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
