//! Real IAM permission management; the legacy enterprise memory catalog is not an authority.
use crate::{alliance::experts_common::TenantId, auth::ApiAuth};
use axum::{
    extract::{FromRef, FromRequestParts, Path, Query, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use mox_platform_iam_core::{
    permission_admin::{AdminError, PermissionCommand},
    IamRepository,
};
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

impl FromRef<crate::GatewayState> for Arc<IamRepository> {
    fn from_ref(state: &crate::GatewayState) -> Self {
        state.iam.clone()
    }
}

fn failure(error: AdminError) -> Response {
    let (status, message) = match error {
        AdminError::Forbidden => (StatusCode::FORBIDDEN, "需要本租户启用的真实超级管理员"),
        AdminError::NotFound => (StatusCode::NOT_FOUND, "对象不存在"),
        AdminError::Conflict => (StatusCode::CONFLICT, "版本已变化，请重新加载后保存"),
        AdminError::Invalid => (StatusCode::BAD_REQUEST, "权限、角色状态或版本参数无效"),
        AdminError::Storage(error) => {
            tracing::error!(%error,"IAM permission administration failed");
            (StatusCode::SERVICE_UNAVAILABLE, "权限存储暂不可用，操作未确认成功")
        },
    };
    (status, Json(json!({"code":status.as_u16(),"message":message}))).into_response()
}
async fn execute(
    iam: Arc<IamRepository>,
    tenant: String,
    actor: String,
    command: PermissionCommand,
) -> Response {
    match tokio::task::spawn_blocking(move || iam.administer_permissions(&tenant, &actor, command))
        .await
    {
        Ok(Ok(data)) => Json(json!({"code":0,"data":data})).into_response(),
        Ok(Err(error)) => failure(error),
        Err(error) => {
            tracing::error!(%error,"IAM permission task failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"code":503,"message":"权限操作未确认成功"})),
            )
                .into_response()
        },
    }
}
async fn catalog(
    State(iam): State<Arc<IamRepository>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
) -> Response {
    execute(iam, tenant, user.id, PermissionCommand::Catalog).await
}
async fn register(
    State(iam): State<Arc<IamRepository>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
) -> Response {
    execute(iam, tenant, user.id, PermissionCommand::RegisterMessageSend).await
}
async fn role(
    State(iam): State<Arc<IamRepository>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    Path(id): Path<String>,
) -> Response {
    execute(iam, tenant, user.id, PermissionCommand::Role(id)).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplaceRequest {
    version: i64,
    permission_ids: Vec<String>,
}
async fn replace(
    State(iam): State<Arc<IamRepository>>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    Path(id): Path<String>,
    Json(body): Json<ReplaceRequest>,
) -> Response {
    execute(
        iam,
        tenant,
        user.id,
        PermissionCommand::ReplaceRole {
            role: id,
            version: body.version,
            permissions: body.permission_ids,
        },
    )
    .await
}
pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S>
where
    Arc<IamRepository>: FromRef<S>,
{
    Router::new()
        .route("/api/system/iam/permissions", get(catalog))
        .route("/api/system/iam/permissions/message-send", post(register))
        .route("/api/system/iam/roles/:id/permissions", get(role).put(replace))
}

/// Guard existing user/role entry points, including password resets and role copies.
/// The current legacy handlers still have separate business transactions; this is an access guard.
pub async fn guard_legacy_admin(
    State(iam): State<Arc<IamRepository>>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let parts: Vec<_> = request.uri().path().split('/').map(str::to_owned).collect();
    let kind = parts.get(3).map(String::as_str);
    if !matches!(kind, Some("user" | "role")) {
        return next.run(request).await;
    }
    let (mut request_parts, body) = request.into_parts();
    let TenantId(tenant) = match TenantId::from_request_parts(&mut request_parts, &()).await {
        Ok(value) => value,
        Err(error) => return error.into_response(),
    };
    let ApiAuth(user) = match ApiAuth::from_request_parts(&mut request_parts, &()).await {
        Ok(value) => value,
        Err(error) => return error.into_response(),
    };
    let Query(query) = match Query::<HashMap<String, String>>::try_from_uri(&request_parts.uri) {
        Ok(value) => value,
        Err(error) => return error.into_response(),
    };
    let mut request = axum::extract::Request::from_parts(request_parts, body);
    if query.get("tenant_id").is_some_and(|requested| requested != &tenant) {
        return failure(AdminError::Forbidden);
    }
    let id = parts
        .get(4)
        .filter(|id| !matches!(id.as_str(), "export" | "all" | ""))
        .map(|id| id.to_string());
    let owned_kind = kind.map(str::to_owned);
    let scope = tenant.clone();
    match tokio::task::spawn_blocking(move || {
        iam.check_permission_admin_scope(&scope, &user.id, owned_kind.as_deref(), id.as_deref())
    })
    .await
    {
        Ok(Ok(())) => {},
        Ok(Err(error)) => return failure(error),
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"code":503,"message":"权限检查失败"})),
            )
                .into_response()
        },
    }
    // TenantId accepts only ASCII alphanumeric, '-' and '_', so no URI escaping is needed.
    let uri = format!(
        "{}{}tenant_id={tenant}",
        request.uri().path_and_query().map(|v| v.as_str()).unwrap_or("/"),
        if request.uri().query().is_some() { "&" } else { "?" }
    );
    match uri.parse() {
        Ok(uri) => *request.uri_mut() = uri,
        Err(_) => return failure(AdminError::Invalid),
    }
    next.run(request).await
}
