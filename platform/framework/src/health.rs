//! 健康检查 — 存活/就绪/详细三级，K8s探针标准

use axum::{extract::State, http::StatusCode, response::Json, routing::get, Router};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Up,
    Down,
    Degraded,
}

/// 组件健康信息
#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub latency_ms: Option<u64>,
}

/// 完整健康报告
#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    pub status: HealthStatus,
    pub service: String,
    pub version: String,
    pub uptime_secs: u64,
    pub components: Vec<ComponentHealth>,
}

/// 健康检查器
#[derive(Clone)]
pub struct HealthChecker {
    inner: Arc<RwLock<HealthInner>>,
}

struct HealthInner {
    service: String,
    version: String,
    start_time: std::time::Instant,
    components: Vec<(String, ComponentHealth)>,
}

impl HealthChecker {
    pub fn new(service: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HealthInner {
                service: service.into(),
                version: version.into(),
                start_time: std::time::Instant::now(),
                components: Vec::new(),
            })),
        }
    }

    /// 注册组件健康检查
    pub async fn register_component(&self, name: impl Into<String>) {
        let name = name.into();
        let mut inner = self.inner.write().await;
        inner.components.push((
            name.clone(),
            ComponentHealth {
                name,
                status: HealthStatus::Up,
                message: None,
                latency_ms: None,
            },
        ));
    }

    /// 更新组件状态
    pub async fn update_component(&self, name: &str, status: HealthStatus, message: Option<String>) {
        let mut inner = self.inner.write().await;
        if let Some((_, comp)) = inner.components.iter_mut().find(|(n, _)| n == name) {
            comp.status = status;
            comp.message = message;
        }
    }

    /// 生成完整健康报告
    pub async fn report(&self) -> HealthReport {
        let inner = self.inner.read().await;
        let components: Vec<ComponentHealth> = inner.components.iter().map(|(_, c)| c.clone()).collect();
        let status = if components.iter().any(|c| c.status == HealthStatus::Down) {
            HealthStatus::Down
        } else if components.iter().any(|c| c.status == HealthStatus::Degraded) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Up
        };
        HealthReport {
            status,
            service: inner.service.clone(),
            version: inner.version.clone(),
            uptime_secs: inner.start_time.elapsed().as_secs(),
            components,
        }
    }

    /// 构建健康检查路由（/health/live, /health/ready, /health）
    pub fn routes(&self) -> Router {
        let checker = self.clone();
        Router::new()
            .route("/health/live", get(live_handler))
            .route("/health/ready", get(ready_handler))
            .route("/health", get(full_handler))
            .with_state(checker)
    }
}

async fn live_handler() -> StatusCode {
    StatusCode::OK
}

async fn ready_handler(State(checker): State<HealthChecker>) -> (StatusCode, Json<HealthReport>) {
    let report = checker.report().await;
    let status = if report.status == HealthStatus::Up { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (status, Json(report))
}

async fn full_handler(State(checker): State<HealthChecker>) -> Json<HealthReport> {
    Json(checker.report().await)
}
