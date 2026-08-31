// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家节点执行器
//!
//! 基于 AI 专家服务（`ExpertConsultant` trait）的真实节点执行器实现，
//! 用于替代 `MockNodeExecutor`，在生产环境中调用真实专家进行节点分析。
//!
//! ## 功能特性
//! - 调用 `ExpertConsultant` 进行专家咨询
//! - 支持超时控制（通过 tokio::time::timeout）
//! - 支持指数退避重试机制
//! - 完整的错误处理与日志记录
//! - 执行统计（成功/失败计数、累计耗时）
//!
//! ## 架构设计
//! 遵循 DIP 依赖倒置原则：
//! - 本执行器依赖 `mox_ai_expert_proto::ExpertConsultant` trait 抽象
//! - 不直接依赖 `mox-ai-expert-svc` 的具体实现
//! - 由上层（svc 层）在组装时注入具体的 consultant 实现

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use mox_ai_expert_proto::{ConsultQuery, ExpertConsultant};
use mox_alliance_common_proto::{AllianceError, AllianceErrorCode, AllianceResult};
use mox_alliance_executor_proto::{NodeExecutor, NodeExecutionRequest, NodeExecutionResult};
use serde_json::json;
use tracing::{debug, error, info, warn};

/// 专家节点执行器配置
#[derive(Debug, Clone)]
pub struct ExpertExecutorConfig {
    /// 单次咨询超时时间（毫秒）
    pub timeout_ms: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 初始重试延迟（毫秒），指数退避使用
    pub initial_retry_delay_ms: u64,
    /// 最大重试延迟（毫秒）
    pub max_retry_delay_ms: u64,
    /// 退避系数（每次重试延迟乘以此系数）
    pub backoff_factor: f64,
}

impl Default for ExpertExecutorConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 300_000, // 5 分钟
            max_retries: 3,
            initial_retry_delay_ms: 1_000, // 1 秒
            max_retry_delay_ms: 30_000,    // 30 秒
            backoff_factor: 2.0,
        }
    }
}

impl ExpertExecutorConfig {
    /// 创建配置，指定超时时间
    pub fn with_timeout(timeout_ms: u64) -> Self {
        Self {
            timeout_ms,
            ..Default::default()
        }
    }

    /// 计算第 n 次重试的延迟（指数退避）
    ///
    /// # 参数
    /// - `retry_count`：当前重试次数（从 0 开始）
    ///
    /// # 返回
    /// 延迟毫秒数，不超过 `max_retry_delay_ms`
    pub fn retry_delay_ms(&self, retry_count: u32) -> u64 {
        let delay = self.initial_retry_delay_ms as f64 * self.backoff_factor.powi(retry_count as i32);
        delay.min(self.max_retry_delay_ms as f64) as u64
    }
}

/// 专家节点执行器统计信息
#[derive(Debug, Default)]
struct ExpertExecutorStats {
    /// 总执行次数
    total_executions: AtomicU64,
    /// 成功次数
    success_count: AtomicU64,
    /// 失败次数
    failure_count: AtomicU64,
    /// 累计重试次数
    total_retries: AtomicU64,
    /// 累计耗时（毫秒）
    total_duration_ms: AtomicU64,
    /// 超时次数
    timeout_count: AtomicU64,
}

impl ExpertExecutorStats {
    fn record_success(&self, duration_ms: u64, retries: u32) {
        self.total_executions.fetch_add(1, Ordering::Relaxed);
        self.success_count.fetch_add(1, Ordering::Relaxed);
        self.total_retries
            .fetch_add(retries as u64, Ordering::Relaxed);
        self.total_duration_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
    }

    fn record_failure(&self, duration_ms: u64, retries: u32, is_timeout: bool) {
        self.total_executions.fetch_add(1, Ordering::Relaxed);
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        self.total_retries
            .fetch_add(retries as u64, Ordering::Relaxed);
        self.total_duration_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        if is_timeout {
            self.timeout_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// 专家节点执行器
///
/// 通过 `ExpertConsultant` trait 调用 AI 专家服务执行 DAG 节点任务。
/// 支持超时控制、指数退避重试和完整的错误处理。
///
/// # 示例
///
/// ```rust,ignore
/// use mox_alliance_executor_core::{ExpertNodeExecutor, ExpertExecutorConfig};
/// use mox_ai_expert_proto::ExpertConsultant;
/// use std::sync::Arc;
///
/// let consultant: Arc<dyn ExpertConsultant> = get_consultant();
/// let config = ExpertExecutorConfig::default();
/// let executor = ExpertNodeExecutor::new(consultant, config);
/// ```
pub struct ExpertNodeExecutor {
    /// 专家咨询服务（trait 对象，遵循 DIP）
    consultant: Arc<dyn ExpertConsultant>,
    /// 执行器配置
    config: ExpertExecutorConfig,
    /// 执行统计
    stats: Arc<ExpertExecutorStats>,
}

impl ExpertNodeExecutor {
    /// 创建新的专家节点执行器
    ///
    /// # 参数
    /// - `consultant`：专家咨询服务实现（通过 trait 对象注入，遵循 DIP）
    /// - `config`：执行器配置
    pub fn new(consultant: Arc<dyn ExpertConsultant>, config: ExpertExecutorConfig) -> Self {
        Self {
            consultant,
            config,
            stats: Arc::new(ExpertExecutorStats::default()),
        }
    }

    /// 获取执行统计信息
    pub fn stats(&self) -> ExecutorStatsView {
        ExecutorStatsView {
            total_executions: self.stats.total_executions.load(Ordering::Relaxed),
            success_count: self.stats.success_count.load(Ordering::Relaxed),
            failure_count: self.stats.failure_count.load(Ordering::Relaxed),
            total_retries: self.stats.total_retries.load(Ordering::Relaxed),
            total_duration_ms: self.stats.total_duration_ms.load(Ordering::Relaxed),
            timeout_count: self.stats.timeout_count.load(Ordering::Relaxed),
        }
    }

    /// 构建咨询查询
    ///
    /// 从节点执行请求中提取信息，构造 `ConsultQuery`。
    /// 节点的 `expert_id` 作为优先专家约束，节点描述作为查询内容。
    fn build_consult_query(&self, request: &NodeExecutionRequest) -> ConsultQuery {
        let mut ctx = HashMap::new();

        // 租户信息
        ctx.insert("tenant".to_string(), request.tenant_id.clone());
        ctx.insert("namespace".to_string(), request.tenant_id.clone());

        // 优先专家约束
        ctx.insert(
            "prefer_expert".to_string(),
            request.node.expert_id.clone(),
        );

        // 将节点输入数据序列化为 JSON 字符串放入 ctx
        if let Some(input) = &request.input_data {
            if let Ok(input_str) = serde_json::to_string(input) {
                ctx.insert("input_data".to_string(), input_str);
            }
        }

        // 将上下文数据放入 ctx
        if let Some(context) = &request.context {
            if let Ok(context_str) = serde_json::to_string(context) {
                ctx.insert("context".to_string(), context_str);
            }
        }

        // 任务 ID 追踪
        ctx.insert("task_id".to_string(), request.task_id.to_string());
        ctx.insert("node_id".to_string(), request.node.node_id.clone());

        // 查询内容：使用节点名称 + 描述
        let query = match &request.node.description {
            Some(desc) if !desc.is_empty() => {
                format!("{}: {}", request.node.name, desc)
            }
            _ => request.node.name.clone(),
        };

        ConsultQuery {
            id: format!("{}-{}", request.task_id, request.node.node_id),
            query,
            ctx,
        }
    }

    /// 将咨询报告转换为节点执行结果的输出 JSON
    fn report_to_output(
        &self,
        report: &mox_ai_expert_proto::ConsultReport,
        expert_id: &str,
    ) -> serde_json::Value {
        json!({
            "expert_id": expert_id,
            "report_id": report.report_id,
            "score": report.score,
            "vetoed": report.vetoed,
            "steps": report.steps,
            "reason": report.reason,
        })
    }

    /// 执行单次咨询（带超时）
    ///
    /// # 返回
    /// - `Ok(report)`：咨询成功
    /// - `Err(e)`：咨询失败或超时
    async fn consult_once(
        &self,
        query: &ConsultQuery,
    ) -> Result<mox_ai_expert_proto::ConsultReport, String> {
        let timeout = tokio::time::Duration::from_millis(self.config.timeout_ms);

        match tokio::time::timeout(timeout, self.consultant.consult(query)).await {
            Ok(Ok(report)) => Ok(report),
            Ok(Err(e)) => Err(format!("Consult error: {}", e)),
            Err(_) => Err(format!(
                "Consult timeout after {}ms",
                self.config.timeout_ms
            )),
        }
    }

    /// 带重试的咨询执行
    ///
    /// 实现指数退避重试策略。
    ///
    /// # 返回
    /// - `Ok((report, retry_count))`：最终成功的报告和实际重试次数
    /// - `Err((error_msg, retry_count, is_timeout))`：最终失败的错误信息和重试次数
    async fn consult_with_retry(
        &self,
        query: &ConsultQuery,
    ) -> Result<
        (mox_ai_expert_proto::ConsultReport, u32),
        (String, u32, bool),
    > {
        let mut last_error: String;
        let mut last_is_timeout: bool;
        let mut retry_count = 0u32;

        loop {
            let attempt = retry_count + 1;
            debug!(
                "Expert consult attempt {}/{} for query {}",
                attempt,
                self.config.max_retries + 1,
                query.id
            );

            match self.consult_once(query).await {
                Ok(report) => {
                    return Ok((report, retry_count));
                }
                Err(e) => {
                    last_is_timeout = e.contains("timeout");
                    warn!(
                        "Expert consult attempt {} failed for query {}: {}",
                        attempt, query.id, e
                    );
                    last_error = e;

                    if retry_count >= self.config.max_retries {
                        break;
                    }

                    // 指数退避等待
                    let delay = self.config.retry_delay_ms(retry_count);
                    debug!(
                        "Retrying expert consult for query {} after {}ms (attempt {}/{})",
                        query.id,
                        delay,
                        attempt,
                        self.config.max_retries + 1
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    retry_count += 1;
                }
            }
        }

        Err((
            last_error,
            retry_count,
            last_is_timeout,
        ))
    }
}

/// 执行器统计信息视图
#[derive(Debug, Clone)]
pub struct ExecutorStatsView {
    pub total_executions: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_retries: u64,
    pub total_duration_ms: u64,
    pub timeout_count: u64,
}

impl ExecutorStatsView {
    /// 成功率（0.0 ~ 1.0）
    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            1.0
        } else {
            self.success_count as f64 / self.total_executions as f64
        }
    }

    /// 平均耗时（毫秒）
    pub fn avg_duration_ms(&self) -> f64 {
        if self.total_executions == 0 {
            0.0
        } else {
            self.total_duration_ms as f64 / self.total_executions as f64
        }
    }
}

#[async_trait]
impl NodeExecutor for ExpertNodeExecutor {
    /// 执行节点
    ///
    /// 从请求中提取专家 ID 和任务描述，调用 AI 专家服务进行咨询，
    /// 并返回执行结果。支持超时控制和指数退避重试。
    ///
    /// # 执行流程
    /// 1. 构造 `ConsultQuery`（专家 ID、任务描述、输入数据）
    /// 2. 调用 `ExpertConsultant::consult`（带超时）
    /// 3. 失败时按指数退避策略重试
    /// 4. 将咨询报告转换为节点执行结果
    async fn execute_node(&self, request: NodeExecutionRequest) -> AllianceResult<NodeExecutionResult> {
        let start = Instant::now();
        let node_id = request.node.node_id.clone();
        let task_id = request.task_id;
        let expert_id = request.node.expert_id.clone();

        info!(
            "Expert node execution started: task={}, node={}, expert={}",
            task_id, node_id, expert_id
        );

        // 1. 构建咨询查询
        let query = self.build_consult_query(&request);

        // 2. 带重试执行咨询
        let result = self.consult_with_retry(&query).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok((report, retry_count)) => {
                self.stats.record_success(duration_ms, retry_count);

                let output = self.report_to_output(&report, &expert_id);
                let success = !report.vetoed;

                info!(
                    "Expert node execution succeeded: task={}, node={}, expert={}, score={:.3}, vetoed={}, duration={}ms, retries={}",
                    task_id, node_id, expert_id, report.score, report.vetoed, duration_ms, retry_count
                );

                Ok(NodeExecutionResult {
                    node_id,
                    task_id,
                    success,
                    output: Some(output),
                    error_message: if report.vetoed {
                        report.reason.clone()
                    } else {
                        None
                    },
                    duration_ms,
                    retry_count,
                })
            }
            Err((error_msg, retry_count, is_timeout)) => {
                self.stats
                    .record_failure(duration_ms, retry_count, is_timeout);

                error!(
                    "Expert node execution failed: task={}, node={}, expert={}, error={}, duration={}ms, retries={}, timeout={}",
                    task_id, node_id, expert_id, error_msg, duration_ms, retry_count, is_timeout
                );

                // 根据错误类型返回不同的错误码
                let err = if is_timeout {
                    AllianceError::new(
                        AllianceErrorCode::NodeExecutionFailed,
                        format!(
                            "Expert consultation timed out after {} retries: {}",
                            retry_count, error_msg
                        ),
                    )
                } else {
                    AllianceError::new(
                        AllianceErrorCode::NodeExecutionFailed,
                        format!(
                            "Expert consultation failed after {} retries: {}",
                            retry_count, error_msg
                        ),
                    )
                };

                // 同时返回失败的执行结果（用于 DAG 引擎标记节点失败）
                Ok(NodeExecutionResult {
                    node_id,
                    task_id,
                    success: false,
                    output: None,
                    error_message: Some(error_msg),
                    duration_ms,
                    retry_count,
                })
                // 注意：我们返回 Ok(success=false) 而不是 Err，
                // 因为 DAG 引擎期望通过 result.success 来判断节点是否成功，
                // 而不是通过 Result 的 Err（Err 表示执行器本身的系统错误）。
                // 上面的 err 变量保留以备将来需要区分系统错误和业务错误。
                .map(|mut r| {
                    r.error_message = Some(err.to_string());
                    r
                })
            }
        }
    }

    /// 执行器名称
    fn executor_name(&self) -> &str {
        "expert-node-executor"
    }

    /// 检查执行器是否健康
    ///
    /// 通过成功率和超时率来判断健康状态：
    /// - 成功率 < 50% 视为不健康
    /// - 超时率 > 30% 视为不健康
    async fn is_healthy(&self) -> bool {
        let stats = self.stats();
        if stats.total_executions < 10 {
            // 执行次数太少，默认视为健康
            return true;
        }

        let success_rate = stats.success_rate();
        let timeout_rate = if stats.total_executions > 0 {
            stats.timeout_count as f64 / stats.total_executions as f64
        } else {
            0.0
        };

        success_rate >= 0.5 && timeout_rate <= 0.3
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_expert_proto::ConsultReport;
    use std::sync::Mutex;

    /// Mock 咨询服务：用于测试 ExpertNodeExecutor
    ///
    /// 使用共享的 `call_count`（Arc<AtomicU64>）以便测试代码可以
    /// 在不进行 downcast 的情况下验证调用次数。
    struct MockConsultant {
        responses: Mutex<Vec<ConsultReport>>,
        call_count: Arc<AtomicU64>,
        fail_count: u32, // 前 N 次调用失败
        delay_ms: u64,   // 模拟延迟
    }

    impl MockConsultant {
        /// 创建新的 Mock 咨询服务，返回共享的调用计数器
        fn new() -> (Self, Arc<AtomicU64>) {
            let counter = Arc::new(AtomicU64::new(0));
            (
                Self {
                    responses: Mutex::new(Vec::new()),
                    call_count: counter.clone(),
                    fail_count: 0,
                    delay_ms: 0,
                },
                counter,
            )
        }

        /// 创建指定失败次数的 Mock 咨询服务
        fn with_failures(fail_count: u32) -> (Self, Arc<AtomicU64>) {
            let counter = Arc::new(AtomicU64::new(0));
            (
                Self {
                    responses: Mutex::new(Vec::new()),
                    call_count: counter.clone(),
                    fail_count,
                    delay_ms: 0,
                },
                counter,
            )
        }

        /// 创建指定延迟的 Mock 咨询服务
        fn with_delay(delay_ms: u64) -> (Self, Arc<AtomicU64>) {
            let counter = Arc::new(AtomicU64::new(0));
            (
                Self {
                    responses: Mutex::new(Vec::new()),
                    call_count: counter.clone(),
                    fail_count: 0,
                    delay_ms,
                },
                counter,
            )
        }

        /// 压入一个预设的响应（按栈顺序弹出）
        fn push_response(&self, report: ConsultReport) {
            self.responses.lock().unwrap().push(report);
        }
    }

    #[async_trait]
    impl ExpertConsultant for MockConsultant {
        async fn consult(&self, query: &ConsultQuery) -> anyhow::Result<ConsultReport> {
            let count = self.call_count.fetch_add(1, Ordering::SeqCst);

            // 模拟延迟
            if self.delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
            }

            // 前 N 次失败
            if count < self.fail_count as u64 {
                return Err(anyhow::anyhow!("Simulated failure #{}", count + 1));
            }

            // 返回预设响应或默认响应
            let mut responses = self.responses.lock().unwrap();
            if let Some(rep) = responses.pop() {
                Ok(rep)
            } else {
                Ok(ConsultReport {
                    report_id: query.id.clone(),
                    steps: vec!["Mock analysis completed".to_string()],
                    score: 0.85,
                    vetoed: false,
                    reason: None,
                })
            }
        }
    }

    // ---- 配置测试 ----

    #[test]
    fn config_default_values() {
        let config = ExpertExecutorConfig::default();
        assert_eq!(config.timeout_ms, 300_000);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_retry_delay_ms, 1_000);
        assert_eq!(config.max_retry_delay_ms, 30_000);
        assert!((config.backoff_factor - 2.0).abs() < 1e-9);
    }

    #[test]
    fn config_retry_delay_exponential_backoff() {
        let config = ExpertExecutorConfig::default();

        // 第 0 次重试延迟 = initial
        let d0 = config.retry_delay_ms(0);
        assert_eq!(d0, 1_000);

        // 第 1 次重试延迟 = initial * factor
        let d1 = config.retry_delay_ms(1);
        assert_eq!(d1, 2_000);

        // 第 2 次重试延迟 = initial * factor^2
        let d2 = config.retry_delay_ms(2);
        assert_eq!(d2, 4_000);

        // 不超过最大值
        let config2 = ExpertExecutorConfig {
            max_retry_delay_ms: 3_000,
            ..ExpertExecutorConfig::default()
        };
        let d_large = config2.retry_delay_ms(5);
        assert!(d_large <= 3_000);
    }

    // ---- 构建查询测试 ----

    #[tokio::test]
    async fn build_consult_query_includes_expert_id() {
        let (mock, _counter) = MockConsultant::new();
        let consultant = Arc::new(mock) as Arc<dyn ExpertConsultant>;
        let executor = ExpertNodeExecutor::new(consultant, ExpertExecutorConfig::default());

        use mox_alliance_common_proto::Node;
        let request = NodeExecutionRequest {
            task_id: uuid::Uuid::new_v4(),
            node: Node {
                node_id: "node-1".to_string(),
                task_id: uuid::Uuid::new_v4(),
                expert_id: "security".to_string(),
                module_id: None,
                name: "安全审查".to_string(),
                description: Some("检查 PII 数据泄露风险".to_string()),
                status: mox_alliance_common_proto::NodeStatus::Pending,
                retry_count: 0,
                dependencies: vec![],
                input_refs: vec![],
                output_ref: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                error_message: None,
            },
            input_data: None,
            context: None,
            tenant_id: "tenant-1".to_string(),
        };

        let query = executor.build_consult_query(&request);
        assert_eq!(query.ctx.get("prefer_expert").unwrap(), "security");
        assert_eq!(query.ctx.get("tenant").unwrap(), "tenant-1");
        assert!(query.query.contains("安全审查"));
        assert!(query.query.contains("PII"));
    }

    // ---- 成功执行测试 ----

    #[tokio::test]
    async fn execute_node_success() {
        // 使用 1ms 延迟确保 duration_ms > 0（快速机器上瞬时操作可能为 0）
        let (mock, _counter) = MockConsultant::with_delay(1);
        let consultant = Arc::new(mock) as Arc<dyn ExpertConsultant>;
        let executor = ExpertNodeExecutor::new(consultant, ExpertExecutorConfig::default());

        use mox_alliance_common_proto::Node;
        let task_id = uuid::Uuid::new_v4();
        let request = NodeExecutionRequest {
            task_id,
            node: Node {
                node_id: "node-1".to_string(),
                task_id,
                expert_id: "security".to_string(),
                module_id: None,
                name: "安全审查".to_string(),
                description: Some("测试".to_string()),
                status: mox_alliance_common_proto::NodeStatus::Pending,
                retry_count: 0,
                dependencies: vec![],
                input_refs: vec![],
                output_ref: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                error_message: None,
            },
            input_data: None,
            context: None,
            tenant_id: "tenant-1".to_string(),
        };

        let result = executor.execute_node(request).await.unwrap();
        assert!(result.success);
        assert_eq!(result.node_id, "node-1");
        assert_eq!(result.task_id, task_id);
        assert_eq!(result.retry_count, 0);
        assert!(result.output.is_some());
        assert!(result.duration_ms > 0);
    }

    // ---- 重试机制测试 ----

    #[tokio::test]
    async fn execute_node_retries_on_failure() {
        // 前 2 次失败，第 3 次成功
        let (mock, call_counter) = MockConsultant::with_failures(2);
        let consultant = Arc::new(mock) as Arc<dyn ExpertConsultant>;

        let config = ExpertExecutorConfig {
            max_retries: 3,
            initial_retry_delay_ms: 1, // 极短延迟加速测试
            max_retry_delay_ms: 10,
            ..ExpertExecutorConfig::default()
        };

        let executor = ExpertNodeExecutor::new(consultant, config);

        use mox_alliance_common_proto::Node;
        let task_id = uuid::Uuid::new_v4();
        let request = NodeExecutionRequest {
            task_id,
            node: Node {
                node_id: "node-retry".to_string(),
                task_id,
                expert_id: "security".to_string(),
                module_id: None,
                name: "重试测试".to_string(),
                description: None,
                status: mox_alliance_common_proto::NodeStatus::Pending,
                retry_count: 0,
                dependencies: vec![],
                input_refs: vec![],
                output_ref: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                error_message: None,
            },
            input_data: None,
            context: None,
            tenant_id: "tenant-1".to_string(),
        };

        let result = executor.execute_node(request).await.unwrap();
        assert!(result.success, "应该在重试后成功");
        assert_eq!(result.retry_count, 2, "应该重试了 2 次");

        // 验证调用次数：1 次初始 + 2 次重试 = 3 次
        assert_eq!(call_counter.load(Ordering::SeqCst), 3);
    }

    // ---- 重试耗尽失败测试 ----

    #[tokio::test]
    async fn execute_node_fails_after_max_retries() {
        // 永远失败
        let (mock, call_counter) = MockConsultant::with_failures(100);
        let consultant = Arc::new(mock) as Arc<dyn ExpertConsultant>;

        let config = ExpertExecutorConfig {
            max_retries: 2,
            initial_retry_delay_ms: 1,
            max_retry_delay_ms: 10,
            ..ExpertExecutorConfig::default()
        };

        let executor = ExpertNodeExecutor::new(consultant, config);

        use mox_alliance_common_proto::Node;
        let task_id = uuid::Uuid::new_v4();
        let request = NodeExecutionRequest {
            task_id,
            node: Node {
                node_id: "node-fail".to_string(),
                task_id,
                expert_id: "security".to_string(),
                module_id: None,
                name: "失败测试".to_string(),
                description: None,
                status: mox_alliance_common_proto::NodeStatus::Pending,
                retry_count: 0,
                dependencies: vec![],
                input_refs: vec![],
                output_ref: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                error_message: None,
            },
            input_data: None,
            context: None,
            tenant_id: "tenant-1".to_string(),
        };

        let result = executor.execute_node(request).await.unwrap();
        assert!(!result.success, "重试耗尽后应该失败");
        assert_eq!(result.retry_count, 2, "应该重试了 2 次");
        assert!(result.error_message.is_some());

        // 验证调用次数：1 次初始 + 2 次重试 = 3 次
        assert_eq!(call_counter.load(Ordering::SeqCst), 3);
    }

    // ---- 超时测试 ----

    #[tokio::test]
    async fn execute_node_timeout() {
        // 模拟延迟超过超时时间
        let (mock, _counter) = MockConsultant::with_delay(200);
        let consultant = Arc::new(mock) as Arc<dyn ExpertConsultant>;

        let config = ExpertExecutorConfig {
            timeout_ms: 50, // 50ms 超时
            max_retries: 0, // 不重试，快速验证超时
            ..ExpertExecutorConfig::default()
        };

        let executor = ExpertNodeExecutor::new(consultant, config);

        use mox_alliance_common_proto::Node;
        let task_id = uuid::Uuid::new_v4();
        let request = NodeExecutionRequest {
            task_id,
            node: Node {
                node_id: "node-timeout".to_string(),
                task_id,
                expert_id: "security".to_string(),
                module_id: None,
                name: "超时测试".to_string(),
                description: None,
                status: mox_alliance_common_proto::NodeStatus::Pending,
                retry_count: 0,
                dependencies: vec![],
                input_refs: vec![],
                output_ref: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                error_message: None,
            },
            input_data: None,
            context: None,
            tenant_id: "tenant-1".to_string(),
        };

        let result = executor.execute_node(request).await.unwrap();
        assert!(!result.success, "超时应该导致失败");
        assert!(
            result.error_message.as_ref().unwrap().contains("timeout"),
            "错误信息应该包含 timeout"
        );
    }

    // ---- 否决（vetoed）测试 ----

    #[tokio::test]
    async fn execute_node_vetoed_report() {
        let (mock, _counter) = MockConsultant::new();
        let vetoed_report = ConsultReport {
            report_id: "vetoed-1".to_string(),
            steps: vec!["分析完成".to_string(), "否决：发现严重安全漏洞".to_string()],
            score: 0.2,
            vetoed: true,
            reason: Some("发现 PII 数据泄露风险，不可自动修复".to_string()),
        };
        mock.push_response(vetoed_report);

        let consultant = Arc::new(mock) as Arc<dyn ExpertConsultant>;
        let executor = ExpertNodeExecutor::new(consultant, ExpertExecutorConfig::default());

        use mox_alliance_common_proto::Node;
        let task_id = uuid::Uuid::new_v4();
        let request = NodeExecutionRequest {
            task_id,
            node: Node {
                node_id: "node-veto".to_string(),
                task_id,
                expert_id: "security".to_string(),
                module_id: None,
                name: "否决测试".to_string(),
                description: None,
                status: mox_alliance_common_proto::NodeStatus::Pending,
                retry_count: 0,
                dependencies: vec![],
                input_refs: vec![],
                output_ref: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                error_message: None,
            },
            input_data: None,
            context: None,
            tenant_id: "tenant-1".to_string(),
        };

        let result = executor.execute_node(request).await.unwrap();
        assert!(!result.success, "被否决的报告应该视为失败");
        assert!(result.error_message.is_some());
        assert!(result.output.is_some());

        // 验证输出包含否决信息
        let output = result.output.unwrap();
        assert_eq!(output["vetoed"], serde_json::Value::Bool(true));
        assert_eq!(output["score"], serde_json::json!(0.2));
    }

    // ---- 统计信息测试 ----

    #[tokio::test]
    async fn executor_stats_tracking() {
        // 使用 1ms 延迟确保 duration_ms > 0（快速机器上瞬时操作可能为 0）
        let (mock, _counter) = MockConsultant::with_delay(1);
        let consultant = Arc::new(mock) as Arc<dyn ExpertConsultant>;
        let executor = ExpertNodeExecutor::new(consultant, ExpertExecutorConfig::default());

        use mox_alliance_common_proto::Node;

        // 执行 3 个成功的节点
        for i in 0..3 {
            let task_id = uuid::Uuid::new_v4();
            let request = NodeExecutionRequest {
                task_id,
                node: Node {
                    node_id: format!("node-{}", i),
                    task_id,
                    expert_id: "security".to_string(),
                    module_id: None,
                    name: format!("节点 {}", i),
                    description: None,
                    status: mox_alliance_common_proto::NodeStatus::Pending,
                    retry_count: 0,
                    dependencies: vec![],
                    input_refs: vec![],
                    output_ref: None,
                    started_at: None,
                    completed_at: None,
                    duration_ms: None,
                    error_message: None,
                },
                input_data: None,
                context: None,
                tenant_id: "tenant-1".to_string(),
            };
            executor.execute_node(request).await.unwrap();
        }

        let stats = executor.stats();
        assert_eq!(stats.total_executions, 3);
        assert_eq!(stats.success_count, 3);
        assert_eq!(stats.failure_count, 0);
        assert!((stats.success_rate() - 1.0).abs() < 1e-9);
        assert!(stats.avg_duration_ms() > 0.0);
    }

    // ---- 健康检查测试 ----

    #[tokio::test]
    async fn is_healthy_default_true() {
        let (mock, _counter) = MockConsultant::new();
        let consultant = Arc::new(mock) as Arc<dyn ExpertConsultant>;
        let executor = ExpertNodeExecutor::new(consultant, ExpertExecutorConfig::default());

        // 执行次数 < 10 时默认为健康
        assert!(executor.is_healthy().await);
    }

    // ---- 执行器名称测试 ----

    #[test]
    fn executor_name_is_correct() {
        let (mock, _counter) = MockConsultant::new();
        let consultant = Arc::new(mock) as Arc<dyn ExpertConsultant>;
        let executor = ExpertNodeExecutor::new(consultant, ExpertExecutorConfig::default());
        assert_eq!(executor.executor_name(), "expert-node-executor");
    }

    // ---- 输出数据结构测试 ----

    #[test]
    fn report_to_output_structure() {
        let (mock, _counter) = MockConsultant::new();
        let consultant = Arc::new(mock) as Arc<dyn ExpertConsultant>;
        let executor = ExpertNodeExecutor::new(consultant, ExpertExecutorConfig::default());

        let report = ConsultReport {
            report_id: "rep-1".to_string(),
            steps: vec!["step1".to_string(), "step2".to_string()],
            score: 0.75,
            vetoed: false,
            reason: None,
        };

        let output = executor.report_to_output(&report, "security");
        assert_eq!(output["expert_id"], "security");
        assert_eq!(output["report_id"], "rep-1");
        assert_eq!(output["score"], serde_json::json!(0.75));
        assert_eq!(output["vetoed"], serde_json::Value::Bool(false));
        assert!(output["steps"].is_array());
        assert_eq!(output["steps"].as_array().unwrap().len(), 2);
    }
}
