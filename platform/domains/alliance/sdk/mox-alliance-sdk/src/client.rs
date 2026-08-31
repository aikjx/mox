// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 联盟客户端
//!
//! 通过 HTTP 调用调度器服务（scheduler-svc）与执行器服务（executor-svc）。

use mox_alliance_api::dto::*;
use mox_alliance_common_proto::{AllianceError, AllianceErrorCode, AllianceResult};
use uuid::Uuid;

/// 联盟客户端配置
#[derive(Debug, Clone)]
pub struct AllianceClientConfig {
    /// 调度器服务基地址（如 http://localhost:8081）
    pub scheduler_base_url: String,
    /// 请求超时（毫秒）
    pub timeout_ms: u64,
    /// 默认租户 ID（可通过 X-Tenant-Id 覆盖）
    pub tenant_id: Option<Uuid>,
    /// 默认用户 ID（可通过 X-User-Id 覆盖）
    pub user_id: Option<Uuid>,
}

impl Default for AllianceClientConfig {
    fn default() -> Self {
        Self {
            scheduler_base_url: "http://localhost:8081".to_string(),
            timeout_ms: 30_000,
            tenant_id: None,
            user_id: None,
        }
    }
}

/// 联盟客户端
#[derive(Clone)]
pub struct AllianceClient {
    config: AllianceClientConfig,
    http: reqwest::Client,
}

impl AllianceClient {
    /// 创建客户端
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_config(AllianceClientConfig {
            scheduler_base_url: base_url.into(),
            ..AllianceClientConfig::default()
        })
    }

    /// 使用完整配置创建客户端
    pub fn with_config(config: AllianceClientConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .expect("failed to build HTTP client");
        Self { config, http }
    }

    /// 设置默认租户
    pub fn with_tenant(mut self, tenant_id: Uuid) -> Self {
        self.config.tenant_id = Some(tenant_id);
        self
    }

    /// 设置默认用户
    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.config.user_id = Some(user_id);
        self
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.scheduler_base_url, path)
    }

    fn apply_identity(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut req = req;
        if let Some(t) = self.config.tenant_id {
            req = req.header("X-Tenant-Id", t.to_string());
        }
        if let Some(u) = self.config.user_id {
            req = req.header("X-User-Id", u.to_string());
        }
        req
    }

    /// 统一解析响应
    async fn parse<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> AllianceResult<T> {
        let status = response.status();
        if status.is_success() {
            response
                .json::<T>()
                .await
                .map_err(|e| AllianceError::internal(format!("Failed to parse response: {}", e)))
        } else {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unable to read response".to_string());
            if let Ok(err) = serde_json::from_str::<ErrorResponse>(&body) {
                Err(AllianceError::new(
                    error_code_from_u32(err.error_code),
                    err.message,
                ))
            } else {
                Err(AllianceError::internal(format!(
                    "HTTP {}: {}",
                    status, body
                )))
            }
        }
    }

    /// 创建任务
    pub async fn create_task(&self, request: CreateTaskRequest) -> AllianceResult<CreateTaskResponse> {
        let resp = self
            .apply_identity(self.http.post(self.url("/tasks")).json(&request))
            .send()
            .await
            .map_err(|e| {
                AllianceError::new(
                    AllianceErrorCode::SchedulerUnavailable,
                    format!("Failed to connect to scheduler: {}", e),
                )
            })?;
        self.parse(resp).await
    }

    /// 获取任务详情
    pub async fn get_task(&self, task_id: Uuid) -> AllianceResult<TaskDetailResponse> {
        let resp = self
            .apply_identity(self.http.get(self.url(&format!("/tasks/{}", task_id))))
            .send()
            .await
            .map_err(|e| {
                AllianceError::new(
                    AllianceErrorCode::SchedulerUnavailable,
                    format!("Failed to connect to scheduler: {}", e),
                )
            })?;
        self.parse(resp).await
    }

    /// 列出任务
    pub async fn list_tasks(&self) -> AllianceResult<TaskListResponse> {
        let resp = self
            .apply_identity(self.http.get(self.url("/tasks")))
            .send()
            .await
            .map_err(|e| {
                AllianceError::new(
                    AllianceErrorCode::SchedulerUnavailable,
                    format!("Failed to connect to scheduler: {}", e),
                )
            })?;
        self.parse(resp).await
    }

    /// 执行任务操作（暂停/恢复/取消）
    pub async fn task_action(
        &self,
        task_id: Uuid,
        action: TaskActionRequest,
    ) -> AllianceResult<SuccessResponse> {
        let resp = self
            .apply_identity(
                self.http
                    .post(self.url(&format!("/tasks/{}", task_id)))
                    .json(&action),
            )
            .send()
            .await
            .map_err(|e| {
                AllianceError::new(
                    AllianceErrorCode::SchedulerUnavailable,
                    format!("Failed to connect to scheduler: {}", e),
                )
            })?;
        self.parse(resp).await
    }

    /// 搜索专家
    pub async fn search_experts(
        &self,
        request: ExpertSearchRequest,
    ) -> AllianceResult<ExpertSearchResponse> {
        let resp = self
            .apply_identity(self.http.post(self.url("/experts/search")).json(&request))
            .send()
            .await
            .map_err(|e| {
                AllianceError::new(
                    AllianceErrorCode::SchedulerUnavailable,
                    format!("Failed to connect to scheduler: {}", e),
                )
            })?;
        self.parse(resp).await
    }

    /// 健康检查
    pub async fn health_check(&self) -> bool {
        match self.http.get(self.url("/health")).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

/// 将 u32 错误码转换为 AllianceErrorCode
fn error_code_from_u32(value: u32) -> AllianceErrorCode {
    match value {
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
