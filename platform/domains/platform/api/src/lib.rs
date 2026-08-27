//! MOX Platform Domain API — trait contracts for IAM, Meta, Datastore, Orchestrator, Enterprise.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PlatformApiError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

pub type PlatformApiResult<T> = Result<T, PlatformApiError>;

// ─── IAM ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

#[async_trait]
pub trait IdentityProvider: Send + Sync {
    async fn authenticate(&self, username: &str, password: &str) -> PlatformApiResult<AuthToken>;
    async fn validate_token(&self, token: &str) -> PlatformApiResult<UserInfo>;
    async fn refresh_token(&self, refresh_token: &str) -> PlatformApiResult<AuthToken>;
    async fn logout(&self, token: &str) -> PlatformApiResult<()>;
}

#[async_trait]
pub trait UserManager: Send + Sync {
    async fn create_user(&self, user: UserInfo) -> PlatformApiResult<UserInfo>;
    async fn get_user(&self, id: &str) -> PlatformApiResult<Option<UserInfo>>;
    async fn update_user(&self, user: UserInfo) -> PlatformApiResult<UserInfo>;
    async fn delete_user(&self, id: &str) -> PlatformApiResult<bool>;
    async fn list_users(&self, tenant_id: &str) -> PlatformApiResult<Vec<UserInfo>>;
    async fn assign_role(&self, user_id: &str, role: &str) -> PlatformApiResult<()>;
}

// ─── Meta / Tenant ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantInfo {
    pub id: String,
    pub name: String,
    pub plan: String,
    pub status: String,
    pub metadata: HashMap<String, String>,
    pub created_at: String,
}

#[async_trait]
pub trait TenantManager: Send + Sync {
    async fn create_tenant(&self, tenant: TenantInfo) -> PlatformApiResult<TenantInfo>;
    async fn get_tenant(&self, id: &str) -> PlatformApiResult<Option<TenantInfo>>;
    async fn update_tenant(&self, tenant: TenantInfo) -> PlatformApiResult<TenantInfo>;
    async fn list_tenants(&self) -> PlatformApiResult<Vec<TenantInfo>>;
}

// ─── Orchestrator ───

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus { Pending, Running, Completed, Failed, Cancelled, Skipped }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub name: String,
    pub workflow_id: String,
    pub status: TaskStatus,
    pub depends_on: Vec<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub tasks: Vec<TaskInfo>,
    pub status: TaskStatus,
    pub created_at: String,
}

#[async_trait]
pub trait WorkflowOrchestrator: Send + Sync {
    async fn submit(&self, workflow: WorkflowInfo) -> PlatformApiResult<String>;
    async fn get_status(&self, workflow_id: &str) -> PlatformApiResult<WorkflowInfo>;
    async fn cancel(&self, workflow_id: &str) -> PlatformApiResult<bool>;
    async fn list(&self, limit: usize) -> PlatformApiResult<Vec<WorkflowInfo>>;
}

// ─── Enterprise / Audit ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub timestamp: String,
    pub details: serde_json::Value,
}

#[async_trait]
pub trait AuditLogger: Send + Sync {
    async fn log(&self, entry: AuditLogEntry) -> PlatformApiResult<()>;
    async fn query(&self, tenant_id: &str, filter: &serde_json::Value) -> PlatformApiResult<Vec<AuditLogEntry>>;
}
