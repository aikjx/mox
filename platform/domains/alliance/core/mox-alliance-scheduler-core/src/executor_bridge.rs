// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 执行器桥接层
//!
//! 定义调度器与执行器之间的桥接抽象，支持多种实现：
//! - `HttpExecutorBridge`: 通过 HTTP 调用远程执行器服务
//! - `InProcessExecutorBridge`: 进程内直接调用（当执行器在同进程时）
//!
//! 设计原则：
//! - 依赖倒置：调度器核心依赖抽象 trait，而非具体实现
//! - 可替换：可以根据部署模式选择不同的 bridge 实现
//! - 向后兼容：保留 dispatch_tx 通道作为 fallback

use async_trait::async_trait;
use mox_alliance_common_proto::{AllianceResult, CollaborationPlan, Task};
// HTTP 桥接（http-bridge feature）与测试期 MockExecutorBridge 使用；
// 默认纯计算编译（非 test）不引入，避免未使用导入告警
#[cfg(any(feature = "http-bridge", test))]
use mox_alliance_common_proto::{AllianceError, AllianceErrorCode, TaskStatus};
use mox_alliance_executor_proto::{DagEngine, ExecutionOptions, ExecutionStatus};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 执行器桥接 trait
///
/// 调度器通过此 trait 与执行器交互，屏蔽底层通信细节。
/// 支持 HTTP 远程调用、进程内直接调用等多种实现。
#[async_trait]
pub trait ExecutorBridge: Send + Sync {
    /// 提交协作计划给执行器
    ///
    /// 调度器生成计划后调用此方法，将任务和计划派发给执行器。
    async fn submit_plan(&self, task: &Task, plan: CollaborationPlan) -> AllianceResult<()>;

    /// 取消执行中的任务
    async fn cancel_task(
        &self,
        task_id: Uuid,
        tenant_id: Uuid,
        reason: Option<String>,
    ) -> AllianceResult<()>;

    /// 获取任务执行状态
    async fn get_status(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<ExecutionStatus>;

    /// 暂停任务执行
    async fn pause_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()>;

    /// 恢复任务执行
    async fn resume_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()>;

    /// 检查执行器是否健康可用
    async fn health_check(&self) -> bool;
}

// ─── HttpExecutorBridge ────────────────────────────────────────────────────

/// HTTP 执行器桥接配置
#[cfg(feature = "http-bridge")]
#[derive(Debug, Clone)]
pub struct HttpExecutorBridgeConfig {
    /// 执行器服务基地址（如 http://localhost:3200）
    pub base_url: String,
    /// 请求超时（毫秒）
    pub timeout_ms: u64,
}

#[cfg(feature = "http-bridge")]
impl Default for HttpExecutorBridgeConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3200".to_string(),
            timeout_ms: 30_000,
        }
    }
}

/// HTTP 执行器桥接实现
///
/// 通过 HTTP REST API 调用远程执行器服务。
/// 适用于调度器和执行器部署在不同进程/机器的场景。
#[cfg(feature = "http-bridge")]
pub struct HttpExecutorBridge {
    config: HttpExecutorBridgeConfig,
    client: reqwest::Client,
}

#[cfg(feature = "http-bridge")]
impl HttpExecutorBridge {
    /// 创建新的 HTTP 执行器桥接
    pub fn new(config: HttpExecutorBridgeConfig) -> AllianceResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            // P1-②：连接超时单独收紧（2s），executor 不可达时快速失败，
            // 不让每次出向调用都等满 30s 总超时。
            .connect_timeout(std::time::Duration::from_millis(2000))
            .build()
            .map_err(|e| AllianceError::internal(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// 构建完整 URL
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url, path)
    }

    /// 一键传输加密（与服务端同源开关 MOX_API_CRYPTO）：POST 请求带协商头，
    /// 开关开启时请求体整体密封为 {"crypto":…}，由执行器中间件透明解密
    fn crypto_post(
        &self,
        url: &str,
        body: &impl serde::Serialize,
    ) -> AllianceResult<reqwest::RequestBuilder> {
        let v = serde_json::to_value(body)
            .map_err(|e| AllianceError::internal(format!("请求体序列化失败: {e}")))?;
        let mut rb = self.client.post(url);
        for (h, hv) in mox_api_crypto::client::outbound_headers() {
            rb = rb.header(h, hv);
        }
        Ok(match mox_api_crypto::client::seal_request(&v) {
            Some(sealed) => rb.json(&sealed),
            None => rb.json(&v),
        })
    }

    /// 解析 HTTP 响应为 AllianceResult（密文应答经 client::open_response 透明解密）
    async fn parse_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> AllianceResult<T> {
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| AllianceError::internal(format!("读取响应失败: {e}")))?;
        let decoded = mox_api_crypto::client::open_response(&bytes);

        if status.is_success() {
            decoded
                .and_then(|v| serde_json::from_value::<T>(v).ok())
                .ok_or_else(|| AllianceError::internal("Failed to parse response"))
        } else {
            let body = match &decoded {
                Some(v) => v.to_string(),
                None => String::from_utf8_lossy(&bytes).to_string(),
            };

            // 尝试解析为标准错误响应
            if let Ok(err_resp) = serde_json::from_str::<ErrorResponse>(&body) {
                // 根据错误码映射
                let code = error_code_from_u32(err_resp.error_code);
                Err(AllianceError::new(code, err_resp.message))
            } else {
                Err(AllianceError::internal(format!(
                    "HTTP {}: {}",
                    status, body
                )))
            }
        }
    }
}

/// 标准错误响应格式（与执行器服务一致）
#[derive(Debug, serde::Deserialize)]
#[cfg(feature = "http-bridge")]
struct ErrorResponse {
    #[allow(dead_code)]
    success: bool,
    error_code: u32,
    message: String,
}

/// 提交计划请求体
#[derive(Debug, serde::Serialize)]
#[cfg(feature = "http-bridge")]
struct SubmitPlanRequest<'a> {
    task: &'a Task,
    plan: &'a CollaborationPlan,
    options: ExecutionOptions,
}

#[cfg(feature = "http-bridge")]
#[async_trait]
impl ExecutorBridge for HttpExecutorBridge {
    async fn submit_plan(&self, task: &Task, plan: CollaborationPlan) -> AllianceResult<()> {
        let url = self.url("/internal/executions");
        let options = ExecutionOptions::default();
        let request_body = SubmitPlanRequest {
            task,
            plan: &plan,
            options,
        };

        debug!(
            "Submitting plan to executor via HTTP: task_id={}, url={}",
            task.task_id, url
        );

        let response = self
            .crypto_post(&url, &request_body)?
            .send()
            .await
            .map_err(|e| {
                AllianceError::new(
                    AllianceErrorCode::ExecutorUnavailable,
                    format!("Failed to connect to executor: {}", e),
                )
            })?;

        self.parse_response::<serde_json::Value>(response).await?;

        info!(
            "Plan submitted to executor successfully: task_id={}",
            task.task_id
        );
        Ok(())
    }

    async fn cancel_task(
        &self,
        task_id: Uuid,
        tenant_id: Uuid,
        reason: Option<String>,
    ) -> AllianceResult<()> {
        let url = self.url(&format!("/tasks/{}/cancel", task_id));

        debug!(
            "Cancelling task via executor HTTP API: task_id={}",
            task_id
        );

        let body = serde_json::json!({
            "tenant_id": tenant_id.to_string(),
            "reason": reason,
        });

        let response = self.crypto_post(&url, &body)?.send()
            .await
            .map_err(|e| {
                AllianceError::new(
                    AllianceErrorCode::ExecutorUnavailable,
                    format!("Failed to connect to executor: {}", e),
                )
            })?;

        self.parse_response::<serde_json::Value>(response).await?;

        info!("Task cancelled via executor: task_id={}", task_id);
        Ok(())
    }

    async fn get_status(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<ExecutionStatus> {
        let url = self.url(&format!("/tasks/{}/status", task_id));

        debug!(
            "Getting task status from executor: task_id={}",
            task_id
        );

        // P1-②：get_status 是幂等 GET，允许最多额外 2 次重试（200ms→400ms），
        // 仅对连接错误/超时/5xx 重试。POST 写操作（submit/cancel/pause/resume）绝不重试。
        let max_extra_retries: u32 = 2;
        let mut attempt: u32 = 0;
        let response = loop {
            match self
                .client
                .get(&url)
                .header("X-Tenant-Id", tenant_id.to_string())
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_server_error() && attempt < max_extra_retries {
                        attempt += 1;
                        let delay_ms = 200u64 * (1 << (attempt - 1));
                        warn!(
                            "get_status got {} from executor, retrying (attempt {}/{})",
                            status, attempt, max_extra_retries
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        continue;
                    }
                    break resp;
                }
                Err(e) => {
                    let retryable = e.is_connect() || e.is_timeout();
                    if retryable && attempt < max_extra_retries {
                        attempt += 1;
                        let delay_ms = 200u64 * (1 << (attempt - 1));
                        warn!(
                            "get_status connect/timeout error ({}), retrying (attempt {}/{})",
                            e, attempt, max_extra_retries
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        continue;
                    }
                    return Err(AllianceError::new(
                        AllianceErrorCode::ExecutorUnavailable,
                        format!("Failed to connect to executor: {}", e),
                    ));
                }
            }
        };

        let status_resp: ExecutionStatusResponse = self.parse_response(response).await?;

        Ok(ExecutionStatus {
            task_id: status_resp.task_id,
            total_nodes: status_resp.total_nodes,
            completed_nodes: status_resp.completed_nodes,
            running_nodes: status_resp.running_nodes,
            failed_nodes: status_resp.failed_nodes,
            pending_nodes: status_resp.pending_nodes,
            skipped_nodes: status_resp.skipped_nodes.unwrap_or(0),
            cancelled_nodes: status_resp.cancelled_nodes.unwrap_or(0),
            progress: status_resp.progress,
            started_at: status_resp.started_at,
            estimated_remaining_ms: None,
        })
    }

    async fn pause_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()> {
        let url = self.url(&format!("/tasks/{}/pause", task_id));

        let body = serde_json::json!({
            "tenant_id": tenant_id.to_string(),
        });

        let response = self.crypto_post(&url, &body)?.send()
            .await
            .map_err(|e| {
                AllianceError::new(
                    AllianceErrorCode::ExecutorUnavailable,
                    format!("Failed to connect to executor: {}", e),
                )
            })?;

        self.parse_response::<serde_json::Value>(response).await?;
        Ok(())
    }

    async fn resume_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()> {
        let url = self.url(&format!("/tasks/{}/resume", task_id));

        let body = serde_json::json!({
            "tenant_id": tenant_id.to_string(),
        });

        let response = self.crypto_post(&url, &body)?.send()
            .await
            .map_err(|e| {
                AllianceError::new(
                    AllianceErrorCode::ExecutorUnavailable,
                    format!("Failed to connect to executor: {}", e),
                )
            })?;

        self.parse_response::<serde_json::Value>(response).await?;
        Ok(())
    }

    async fn health_check(&self) -> bool {
        let url = self.url("/health");
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(e) => {
                warn!("Executor health check failed: {}", e);
                false
            }
        }
    }
}

/// 执行状态响应（与执行器服务 API 对应）
#[derive(Debug, serde::Deserialize)]
#[cfg(feature = "http-bridge")]
struct ExecutionStatusResponse {
    task_id: Uuid,
    #[allow(dead_code)]
    status: TaskStatus,
    progress: f32,
    total_nodes: usize,
    completed_nodes: usize,
    running_nodes: usize,
    failed_nodes: usize,
    pending_nodes: usize,
    skipped_nodes: Option<usize>,
    cancelled_nodes: Option<usize>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ─── InProcessExecutorBridge ───────────────────────────────────────────────

/// 进程内执行器桥接
///
/// 当执行器与调度器在同一进程时，直接调用 DagEngine trait。
/// 适用于单体部署、测试等场景。
pub struct InProcessExecutorBridge {
    engine: Arc<dyn DagEngine>,
}

impl InProcessExecutorBridge {
    /// 创建新的进程内执行器桥接
    pub fn new(engine: Arc<dyn DagEngine>) -> Self {
        Self { engine }
    }
}

#[async_trait]
impl ExecutorBridge for InProcessExecutorBridge {
    async fn submit_plan(&self, task: &Task, plan: CollaborationPlan) -> AllianceResult<()> {
        let options = ExecutionOptions::default();

        debug!(
            "Submitting plan to in-process executor: task_id={}",
            task.task_id
        );

        self.engine
            .start_execution(task, plan, options)
            .await?;

        info!(
            "Plan submitted to in-process executor successfully: task_id={}",
            task.task_id
        );
        Ok(())
    }

    async fn cancel_task(
        &self,
        task_id: Uuid,
        tenant_id: Uuid,
        reason: Option<String>,
    ) -> AllianceResult<()> {
        debug!(
            "Cancelling task via in-process executor: task_id={}",
            task_id
        );
        self.engine
            .cancel_execution(task_id, tenant_id, reason)
            .await
    }

    async fn get_status(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<ExecutionStatus> {
        self.engine.get_execution_status(task_id, tenant_id).await
    }

    async fn pause_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()> {
        self.engine.pause_execution(task_id, tenant_id).await
    }

    async fn resume_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()> {
        self.engine.resume_execution(task_id, tenant_id).await
    }

    async fn health_check(&self) -> bool {
        // 进程内执行器总是可用的（只要 engine 存在）
        true
    }
}

// ─── NoopExecutorBridge ────────────────────────────────────────────────────

/// 空操作执行器桥接（向后兼容）
///
/// 当没有配置执行器时使用，保持向后兼容。
/// 所有操作都返回成功但不做任何实际工作。
pub struct NoopExecutorBridge;

#[async_trait]
impl ExecutorBridge for NoopExecutorBridge {
    async fn submit_plan(&self, task: &Task, _plan: CollaborationPlan) -> AllianceResult<()> {
        warn!(
            "NoopExecutorBridge: submit_plan called but no executor configured, task_id={}",
            task.task_id
        );
        Ok(())
    }

    async fn cancel_task(
        &self,
        task_id: Uuid,
        _tenant_id: Uuid,
        _reason: Option<String>,
    ) -> AllianceResult<()> {
        warn!(
            "NoopExecutorBridge: cancel_task called but no executor configured, task_id={}",
            task_id
        );
        Ok(())
    }

    async fn get_status(&self, task_id: Uuid, _tenant_id: Uuid) -> AllianceResult<ExecutionStatus> {
        warn!(
            "NoopExecutorBridge: get_status called but no executor configured, task_id={}",
            task_id
        );
        // 返回默认的空状态
        Ok(ExecutionStatus {
            task_id,
            total_nodes: 0,
            completed_nodes: 0,
            running_nodes: 0,
            failed_nodes: 0,
            pending_nodes: 0,
            skipped_nodes: 0,
            cancelled_nodes: 0,
            progress: 0.0,
            started_at: None,
            estimated_remaining_ms: None,
        })
    }

    async fn pause_task(&self, task_id: Uuid, _tenant_id: Uuid) -> AllianceResult<()> {
        warn!(
            "NoopExecutorBridge: pause_task called but no executor configured, task_id={}",
            task_id
        );
        Ok(())
    }

    async fn resume_task(&self, task_id: Uuid, _tenant_id: Uuid) -> AllianceResult<()> {
        warn!(
            "NoopExecutorBridge: resume_task called but no executor configured, task_id={}",
            task_id
        );
        Ok(())
    }

    async fn health_check(&self) -> bool {
        false
    }
}

// ─── 错误码转换辅助函数 ────────────────────────────────────────────────────

/// 将 u32 错误码转换为 AllianceErrorCode
#[cfg(feature = "http-bridge")]
fn error_code_from_u32(value: u32) -> AllianceErrorCode {
    match value {
        1000 => AllianceErrorCode::Unknown,
        1001 => AllianceErrorCode::InvalidArgument,
        1002 => AllianceErrorCode::NotFound,
        1003 => AllianceErrorCode::AlreadyExists,
        1004 => AllianceErrorCode::PermissionDenied,
        1005 => AllianceErrorCode::TenantMismatch,
        2000 => AllianceErrorCode::TaskNotFound,
        2001 => AllianceErrorCode::InvalidTaskStatus,
        2002 => AllianceErrorCode::TaskAlreadyTerminal,
        2003 => AllianceErrorCode::TaskCreationFailed,
        3000 => AllianceErrorCode::PlanGenerationFailed,
        3001 => AllianceErrorCode::InvalidPlan,
        3002 => AllianceErrorCode::PlanVersionConflict,
        4000 => AllianceErrorCode::NodeExecutionFailed,
        4001 => AllianceErrorCode::NodeNotFound,
        4002 => AllianceErrorCode::ExecutorUnavailable,
        4003 => AllianceErrorCode::DependencyNotMet,
        5000 => AllianceErrorCode::ExpertNotFound,
        5001 => AllianceErrorCode::ExpertUnavailable,
        5002 => AllianceErrorCode::ExpertMatchFailed,
        5003 => AllianceErrorCode::ExpertRegistrationFailed,
        6000 => AllianceErrorCode::FusionFailed,
        6001 => AllianceErrorCode::UnsupportedFusionStrategy,
        7000 => AllianceErrorCode::SchedulerFull,
        7001 => AllianceErrorCode::QueueTimeout,
        _ => AllianceErrorCode::Unknown,
    }
}

// ─── Mock 实现（测试用，对外可见） ──────────────────────────────────────
//
// MockExecutorBridge 定义在测试模块中，但在此处重新导出，
// 以便 crate 内其他模块的测试可以使用它。

#[cfg(test)]
pub use tests::MockExecutorBridge;

#[cfg(test)]
pub mod tests {
    use super::*;
    use mox_alliance_common_proto::{
        AllianceMode, FusionStrategy, Node, NodeStatus,
    };
    use std::collections::HashMap;

    // ─── Mock 实现 ────────────────────────────────────────────────────────

    /// Mock 执行器桥接（用于单元测试）
    pub struct MockExecutorBridge {
        pub submitted: std::sync::Mutex<Vec<(Uuid, Uuid)>>, // (task_id, tenant_id)
        pub cancelled: std::sync::Mutex<Vec<(Uuid, Uuid)>>,
        pub paused: std::sync::Mutex<Vec<(Uuid, Uuid)>>,
        pub resumed: std::sync::Mutex<Vec<(Uuid, Uuid)>>,
        pub status_to_return: std::sync::Mutex<Option<ExecutionStatus>>,
        pub should_fail: std::sync::Mutex<bool>,
        /// 执行器活着、但**不认识**这些任务（404 语义）：leader 对账判定孤儿的输入。
        pub unknown_tasks: std::sync::Mutex<Vec<Uuid>>,
        /// 覆盖 `health_check()` 结果：用于构造"活着但请求失败"的瞬时故障面。
        pub health_override: std::sync::Mutex<Option<bool>>,
    }

    impl MockExecutorBridge {
        pub fn new() -> Self {
            Self {
                submitted: std::sync::Mutex::new(Vec::new()),
                cancelled: std::sync::Mutex::new(Vec::new()),
                paused: std::sync::Mutex::new(Vec::new()),
                resumed: std::sync::Mutex::new(Vec::new()),
                status_to_return: std::sync::Mutex::new(None),
                should_fail: std::sync::Mutex::new(false),
                unknown_tasks: std::sync::Mutex::new(Vec::new()),
                health_override: std::sync::Mutex::new(None),
            }
        }

        pub fn set_status(&self, status: ExecutionStatus) {
            *self.status_to_return.lock().unwrap() = Some(status);
        }

        pub fn set_should_fail(&self, fail: bool) {
            *self.should_fail.lock().unwrap() = fail;
        }

        /// 声明这些任务在执行器侧不存在（`health_check` 仍为 true）。
        pub fn set_unknown_tasks(&self, ids: Vec<Uuid>) {
            *self.unknown_tasks.lock().unwrap() = ids;
        }

        /// `None` 恢复默认（`!should_fail`）。
        pub fn set_health(&self, healthy: Option<bool>) {
            *self.health_override.lock().unwrap() = healthy;
        }

        pub fn submitted_count(&self) -> usize {
            self.submitted.lock().unwrap().len()
        }

        pub fn cancelled_count(&self) -> usize {
            self.cancelled.lock().unwrap().len()
        }
    }

    impl Default for MockExecutorBridge {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl ExecutorBridge for MockExecutorBridge {
        async fn submit_plan(&self, task: &Task, _plan: CollaborationPlan) -> AllianceResult<()> {
            if *self.should_fail.lock().unwrap() {
                return Err(AllianceError::new(
                    AllianceErrorCode::ExecutorUnavailable,
                    "Mock failure",
                ));
            }
            self.submitted
                .lock()
                .unwrap()
                .push((task.task_id, task.tenant_id));
            Ok(())
        }

        async fn cancel_task(
            &self,
            task_id: Uuid,
            tenant_id: Uuid,
            _reason: Option<String>,
        ) -> AllianceResult<()> {
            if *self.should_fail.lock().unwrap() {
                return Err(AllianceError::new(
                    AllianceErrorCode::ExecutorUnavailable,
                    "Mock failure",
                ));
            }
            self.cancelled.lock().unwrap().push((task_id, tenant_id));
            Ok(())
        }

        async fn get_status(
            &self,
            task_id: Uuid,
            _tenant_id: Uuid,
        ) -> AllianceResult<ExecutionStatus> {
            if *self.should_fail.lock().unwrap() {
                return Err(AllianceError::new(
                    AllianceErrorCode::ExecutorUnavailable,
                    "Mock failure",
                ));
            }
            if self.unknown_tasks.lock().unwrap().contains(&task_id) {
                return Err(AllianceError::new(
                    AllianceErrorCode::TaskNotFound,
                    format!("Mock executor has no record of task {}", task_id),
                ));
            }
            let status = self.status_to_return.lock().unwrap().clone();
            Ok(status.unwrap_or(ExecutionStatus {
                task_id,
                total_nodes: 0,
                completed_nodes: 0,
                running_nodes: 0,
                failed_nodes: 0,
                pending_nodes: 0,
                skipped_nodes: 0,
                cancelled_nodes: 0,
                progress: 0.0,
                started_at: None,
                estimated_remaining_ms: None,
            }))
        }

        async fn pause_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()> {
            self.paused.lock().unwrap().push((task_id, tenant_id));
            Ok(())
        }

        async fn resume_task(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()> {
            self.resumed.lock().unwrap().push((task_id, tenant_id));
            Ok(())
        }

        async fn health_check(&self) -> bool {
            let overridden = *self.health_override.lock().unwrap();
            overridden.unwrap_or(!*self.should_fail.lock().unwrap())
        }
    }

    // ─── 测试用例 ────────────────────────────────────────────────────────

    fn create_test_task() -> Task {
        let mut task = Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Test Task".to_string(),
            "Test description".to_string(),
        );
        task.status = TaskStatus::Running;
        task
    }

    fn create_test_plan(task_id: Uuid) -> CollaborationPlan {
        CollaborationPlan {
            task_id,
            mode: AllianceMode::Parallel,
            fusion_strategy: FusionStrategy::Weighted,
            dynamic_routes: vec![],
            nodes: vec![Node {
                node_id: "node-1".to_string(),
                task_id,
                expert_id: "expert-1".to_string(),
                module_id: None,
                name: "Test Node".to_string(),
                description: None,
                status: NodeStatus::Pending,
                retry_count: 0,
                dependencies: vec![],
                input_refs: vec![],
                output_ref: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                error_message: None,
            }],
            expert_weights: HashMap::new(),
            version: 1,
            created_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_mock_bridge_submit_plan() {
        let bridge = MockExecutorBridge::new();
        let task = create_test_task();
        let plan = create_test_plan(task.task_id);

        let result = bridge.submit_plan(&task, plan).await;
        assert!(result.is_ok());
        assert_eq!(bridge.submitted_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_bridge_cancel_task() {
        let bridge = MockExecutorBridge::new();
        let task_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        let result = bridge.cancel_task(task_id, tenant_id, None).await;
        assert!(result.is_ok());
        assert_eq!(bridge.cancelled_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_bridge_get_status() {
        let bridge = MockExecutorBridge::new();
        let task_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        let expected = ExecutionStatus {
            task_id,
            total_nodes: 5,
            completed_nodes: 2,
            running_nodes: 1,
            failed_nodes: 0,
            pending_nodes: 2,
            skipped_nodes: 0,
            cancelled_nodes: 0,
            progress: 0.4,
            started_at: None,
            estimated_remaining_ms: None,
        };
        bridge.set_status(expected.clone());

        let result = bridge.get_status(task_id, tenant_id).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        assert_eq!(status.total_nodes, 5);
        assert_eq!(status.completed_nodes, 2);
        assert_eq!(status.progress, 0.4);
    }

    #[tokio::test]
    async fn test_mock_bridge_with_failure() {
        let bridge = MockExecutorBridge::new();
        bridge.set_should_fail(true);

        let task = create_test_task();
        let plan = create_test_plan(task.task_id);

        let result = bridge.submit_plan(&task, plan).await;
        assert!(result.is_err());
        assert_eq!(bridge.submitted_count(), 0);
    }

    #[tokio::test]
    async fn test_noop_bridge() {
        let bridge = NoopExecutorBridge;
        let task = create_test_task();
        let plan = create_test_plan(task.task_id);

        // Noop bridge 应该总是返回 Ok
        assert!(bridge.submit_plan(&task, plan).await.is_ok());
        assert!(bridge
            .cancel_task(task.task_id, task.tenant_id, None)
            .await
            .is_ok());
        assert!(bridge
            .get_status(task.task_id, task.tenant_id)
            .await
            .is_ok());
        assert!(!bridge.health_check().await);
    }

    #[tokio::test]
    async fn test_mock_bridge_pause_resume() {
        let bridge = MockExecutorBridge::new();
        let task_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        assert!(bridge.pause_task(task_id, tenant_id).await.is_ok());
        assert_eq!(bridge.paused.lock().unwrap().len(), 1);

        assert!(bridge.resume_task(task_id, tenant_id).await.is_ok());
        assert_eq!(bridge.resumed.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_mock_bridge_health_check() {
        let bridge = MockExecutorBridge::new();
        assert!(bridge.health_check().await);

        bridge.set_should_fail(true);
        assert!(!bridge.health_check().await);
    }
}
