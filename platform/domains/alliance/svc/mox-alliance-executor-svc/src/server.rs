// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 执行器服务器（纯真实专家执行，无 Mock 路径）

use std::net::SocketAddr;
use std::sync::Arc;

use mox_alliance_executor_core::{
    DagEngineImpl, ExpertExecutorConfig, ExpertNodeExecutor,
};
use mox_alliance_executor_proto::types::ExecutorConfig;
use mox_ai_expert_svc::expert_traits::llm_consultant;
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
        Self { config, listen_addr }
    }

    /// 创建真实节点执行器
    fn create_node_executor(
        &self,
    ) -> (Arc<dyn mox_alliance_executor_proto::NodeExecutor>, bool) {
        let consultant = llm_consultant();
        let expert_config = ExpertExecutorConfig {
            timeout_ms: self.config.default_node_timeout_ms,
            max_retries: self.config.default_max_retries,
            ..ExpertExecutorConfig::default()
        };
        (Arc::new(ExpertNodeExecutor::new(consultant, expert_config)), true)
    }

    /// 构建应用
    pub async fn build_app(&self) -> anyhow::Result<axum::Router> {
        let (node_executor, execution_ready) = self.create_node_executor();

        info!(
            "Executor building app with executor: {}",
            node_executor.executor_name()
        );

        let state_sink = crate::state_sink::resolve_state_sink();
        let engine =
            DagEngineImpl::spawn_with_state_sink(self.config.clone(), node_executor, state_sink);

        let mut state = ExecutorAppState::new(self.config.clone(), engine);
        state.execution_ready = execution_ready;
        state.execution_mode = "llm";

        Ok(build_router(state))
    }

    /// 启动服务器
    pub async fn run(&self) -> anyhow::Result<()> {
        let app = self.build_app().await?;

        info!(
            "Executor server starting on {}",
            self.listen_addr
        );

        let listener = tokio::net::TcpListener::bind(self.listen_addr).await?;
        info!(service = "alliance-executor", addr = %self.listen_addr, "TCP listener bound");
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
                info!("alliance-executor shutdown signal received");
            })
            .await?;

        info!("alliance-executor stopped gracefully");
        Ok(())
    }
}
