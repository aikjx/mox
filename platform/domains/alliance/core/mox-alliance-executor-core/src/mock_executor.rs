// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Mock 节点执行器
//!
//! 用于测试和开发的 Mock 实现，模拟节点执行：
//! - 可配置成功率
//! - 可配置执行延迟
//! - 可配置输出数据

use async_trait::async_trait;
use mox_alliance_common_proto::AllianceResult;
use mox_alliance_executor_proto::{NodeExecutor, NodeExecutionRequest, NodeExecutionResult};
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Mock 节点执行器配置
#[derive(Debug, Clone)]
pub struct MockExecutorConfig {
    /// 执行延迟（毫秒）
    pub delay_ms: u64,
    /// 成功率（0.0 ~ 1.0）
    pub success_rate: f64,
    /// 是否生成模拟输出
    pub generate_output: bool,
}

impl Default for MockExecutorConfig {
    fn default() -> Self {
        Self {
            delay_ms: 100,
            success_rate: 1.0,
            generate_output: true,
        }
    }
}

/// Mock 节点执行器
pub struct MockNodeExecutor {
    config: MockExecutorConfig,
    executed_count: Arc<AtomicU64>,
    counter: Arc<AtomicU64>,
}

impl MockNodeExecutor {
    pub fn new(config: MockExecutorConfig) -> Self {
        Self {
            config,
            executed_count: Arc::new(AtomicU64::new(0)),
            counter: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn executed_count(&self) -> u64 {
        self.executed_count.load(Ordering::SeqCst)
    }

    /// 基于 node_id 和计数器生成伪随机值 (0.0 ~ 1.0)
    fn pseudo_random(&self, seed: &str) -> f64 {
        let count = self.counter.fetch_add(1, Ordering::SeqCst);
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        count.hash(&mut hasher);
        let hash = hasher.finish();
        (hash % 1000) as f64 / 1000.0
    }

    /// 生成确定性的 "得分"
    fn pseudo_score(&self, seed: &str) -> f64 {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let hash = hasher.finish();
        0.5 + (hash % 500) as f64 / 1000.0 // 0.5 ~ 1.0
    }
}

impl Default for MockNodeExecutor {
    fn default() -> Self {
        Self::new(MockExecutorConfig::default())
    }
}

#[async_trait]
impl NodeExecutor for MockNodeExecutor {
    async fn execute_node(&self, request: NodeExecutionRequest) -> AllianceResult<NodeExecutionResult> {
        self.executed_count.fetch_add(1, Ordering::SeqCst);

        // 模拟执行延迟
        if self.config.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.config.delay_ms)).await;
        }

        // 基于 node_id 生成确定性结果
        let seed = &request.node.node_id;
        let random_val = self.pseudo_random(seed);
        let success = random_val < self.config.success_rate;

        let output = if self.config.generate_output && success {
            let score = self.pseudo_score(seed);
            Some(json!({
                "node_id": request.node.node_id,
                "expert_id": request.node.expert_id,
                "result": format!("Mock result for {}", request.node.name),
                "score": score,
                "findings": vec![
                    format!("Finding 1 from {}", request.node.name),
                    format!("Finding 2 from {}", request.node.name),
                ]
            }))
        } else {
            None
        };

        let error_message = if !success {
            Some("Mock execution failed".to_string())
        } else {
            None
        };

        Ok(NodeExecutionResult {
            node_id: request.node.node_id,
            task_id: request.task_id,
            success,
            output,
            error_message,
            duration_ms: self.config.delay_ms,
            retry_count: 0,
        })
    }

    fn executor_name(&self) -> &str {
        "mock-executor"
    }
}
