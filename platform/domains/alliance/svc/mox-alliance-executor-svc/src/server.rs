// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 执行器服务器

use std::net::SocketAddr;
use std::sync::Arc;

use mox_alliance_executor_core::{
    DagEngineImpl, ExpertExecutorConfig, ExpertNodeExecutor, MockExecutorConfig, MockNodeExecutor,
};
use mox_alliance_executor_proto::types::ExecutorConfig;
use mox_ai_expert_svc::expert_traits::default_consultant;
use tracing::info;

use crate::app_state::ExecutorAppState;
use crate::routes::build_router;

/// 执行器运行模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorMode {
    /// 使用 Mock 执行器（测试/开发用）
    Mock,
    /// 使用真实专家执行器（生产用）
    Expert,
}

/// 执行器服务器
pub struct ExecutorServer {
    config: ExecutorConfig,
    listen_addr: SocketAddr,
    mode: ExecutorMode,
}

impl ExecutorServer {
    pub fn new(config: ExecutorConfig, listen_addr: SocketAddr) -> Self {
        Self {
            config,
            listen_addr,
            mode: ExecutorMode::Mock, // 默认 Mock 模式，保持向后兼容
        }
    }

    /// 设置执行器模式
    pub fn with_mode(mut self, mode: ExecutorMode) -> Self {
        self.mode = mode;
        self
    }

    /// 创建节点执行器
    fn create_node_executor(
        &self,
    ) -> Arc<dyn mox_alliance_executor_proto::NodeExecutor> {
        match self.mode {
            ExecutorMode::Mock => {
                // Mock 执行器（Phase 1 / 测试用）
                let mock_config = MockExecutorConfig {
                    delay_ms: 50,
                    success_rate: 1.0,
                    generate_output: true,
                };
                Arc::new(MockNodeExecutor::new(mock_config))
            }
            ExecutorMode::Expert => {
                // 真实专家执行器（调用 AI 专家服务）
                let consultant = default_consultant();
                let expert_config = ExpertExecutorConfig {
                    timeout_ms: self.config.default_node_timeout_ms,
                    max_retries: self.config.default_max_retries,
                    ..ExpertExecutorConfig::default()
                };
                Arc::new(ExpertNodeExecutor::new(consultant, expert_config))
            }
        }
    }

    /// 启动服务器
    pub async fn run(&self) -> anyhow::Result<()> {
        // 创建节点执行器
        let node_executor = self.create_node_executor();

        info!(
            "Executor server starting with mode: {:?}, executor: {}",
            self.mode,
            node_executor.executor_name()
        );

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
