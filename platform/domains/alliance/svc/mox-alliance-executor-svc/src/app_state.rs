// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 执行器应用状态

use std::sync::Arc;

use mox_alliance_executor_core::DagEngineImpl;
use mox_alliance_executor_proto::types::ExecutorConfig;

/// 执行器应用状态
#[derive(Clone)]
pub struct ExecutorAppState {
    pub config: ExecutorConfig,
    pub engine: Arc<DagEngineImpl>,
}

impl ExecutorAppState {
    pub fn new(config: ExecutorConfig, engine: Arc<DagEngineImpl>) -> Self {
        Self { config, engine }
    }
}
