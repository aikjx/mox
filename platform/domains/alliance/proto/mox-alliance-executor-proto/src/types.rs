// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 执行器配置

use serde::{Deserialize, Serialize};

/// 执行器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// 最大并发执行节点数
    pub max_concurrent_nodes: usize,
    /// 节点默认超时（毫秒）
    pub default_node_timeout_ms: u64,
    /// 默认最大重试次数
    pub default_max_retries: u32,
    /// 调度轮询间隔（毫秒）
    pub poll_interval_ms: u64,
    /// 进度更新最小间隔（毫秒）
    pub progress_update_interval_ms: u64,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_nodes: 50,
            default_node_timeout_ms: 300_000, // 5 分钟
            default_max_retries: 3,
            poll_interval_ms: 100,
            progress_update_interval_ms: 500,
        }
    }
}
