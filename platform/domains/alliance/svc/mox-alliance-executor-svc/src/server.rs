// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 执行器服务器

use std::net::SocketAddr;
use std::sync::Arc;

use mox_alliance_executor_core::{
    DagEngineImpl, ExpertExecutorConfig, ExpertNodeExecutor, MockExecutorConfig, MockNodeExecutor,
};
use mox_alliance_executor_proto::types::ExecutorConfig;
use mox_ai_expert_svc::expert_traits::llm_consultant;
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
            mode: ExecutorMode::Expert, // 默认真实专家执行器（Phase 2 起禁用 Mock 默认）
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
    ) -> (Arc<dyn mox_alliance_executor_proto::NodeExecutor>, bool) {
        match self.mode {
            ExecutorMode::Mock => {
                // Mock 执行器（Phase 1 / 测试用）
                let mock_config = MockExecutorConfig {
                    delay_ms: 50,
                    success_rate: 1.0,
                    generate_output: true,
                };
                (Arc::new(MockNodeExecutor::new(mock_config)), false)
            }
            ExecutorMode::Expert => {
                // 真实专家执行器：优先真实 LLM（MOX_LLM_* 环境变量），未配置时回退本地引擎
                // （本地引擎始终可用，ready 恒为 true，任务执行不依赖外部模型）
                let consultant = llm_consultant();
                let ready = true;
                let expert_config = ExpertExecutorConfig {
                    timeout_ms: self.config.default_node_timeout_ms,
                    max_retries: self.config.default_max_retries,
                    ..ExpertExecutorConfig::default()
                };
                (Arc::new(ExpertNodeExecutor::new(consultant, expert_config)), ready)
            }
        }
    }

    /// 构建应用（将构建逻辑与网络监听分离，便于测试注入与复用）
    pub async fn build_app(&self) -> anyhow::Result<axum::Router> {
        // 创建节点执行器
        let (node_executor, execution_ready) = self.create_node_executor();

        info!(
            "Executor building app with mode: {:?}, executor: {}",
            self.mode,
            node_executor.executor_name()
        );

        // 创建 DAG 执行引擎并启动调度循环
        // 执行状态持久化端口：复用 MOX_ALLIANCE_STORAGE_MODE 存储底座（与 scheduler-svc
        // 同一约定、同一任务真源）。memory 模式解析为 None → 纯内存执行，行为与未接线前一致。
        let state_sink = crate::state_sink::resolve_state_sink();
        let engine =
            DagEngineImpl::spawn_with_state_sink(self.config.clone(), node_executor, state_sink);

        // 构建应用状态
        let mut state = ExecutorAppState::new(self.config.clone(), engine);
        state.execution_ready = execution_ready;
        state.execution_mode = if self.mode == ExecutorMode::Expert { "llm" } else { "mock" };

        // 构建路由
        Ok(build_router(state))
    }

    /// 启动服务器
    pub async fn run(&self) -> anyhow::Result<()> {
        let app = self.build_app().await?;

        info!(
            "Executor server starting on {}",
            self.listen_addr
        );

        // 启动服务
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
