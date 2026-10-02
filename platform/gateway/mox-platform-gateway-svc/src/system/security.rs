//! Tenant-scoped persistent credential administration.
use crate::{alliance::experts_common::TenantId, auth::ApiAuth, GatewayState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use mox_platform_iam_core::{
    api_keys::KeyCommand, audit_query::AuditQuery, permission_admin::AdminError,
};
use serde::Deserialize;
use serde_json::json;

fn failure(error: AdminError) -> Response {
    let status = match error {
        AdminError::Forbidden => StatusCode::FORBIDDEN,
        AdminError::NotFound => StatusCode::NOT_FOUND,
        AdminError::Invalid => StatusCode::BAD_REQUEST,
        AdminError::Conflict => StatusCode::CONFLICT,
        AdminError::Storage(error) => {
            tracing::error!(%error,"Credential storage failed");
            StatusCode::SERVICE_UNAVAILABLE
        },
    };
    (status, Json(json!({"code":status.as_u16(),"message":"Credential operation failed"})))
        .into_response()
}
async fn execute(s: GatewayState, tenant: String, actor: String, command: KeyCommand) -> Response {
    match tokio::task::spawn_blocking(move || s.iam.administer_api_keys(&tenant, &actor, command))
        .await
    {
        Ok(Ok(data)) => Json(json!({"code":0,"data":data})).into_response(),
        Ok(Err(error)) => failure(error),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
pub(crate) async fn security_status(
    State(s): State<GatewayState>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
) -> Response {
    let iam = s.iam.clone();
    match tokio::task::spawn_blocking(move || iam.check_permission_admin_scope(&tenant,&user.id,None,None)).await {
        Ok(Ok(())) => Json(json!({"code":0,"data":{"auth_enabled":s.config.auth.enabled,"rate_limit_enabled":s.config.rate_limit.enabled,"credential_store":"sqlite","enterprise_ready":false}})).into_response(),
        Ok(Err(error)) => failure(error),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
pub(crate) async fn list_api_keys(
    State(s): State<GatewayState>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    Query(query): Query<KeyPage>,
) -> Response {
    execute(s, tenant, user.id, KeyCommand::List { page: query.page, page_size: query.page_size })
        .await
}
#[derive(Deserialize)]
pub(crate) struct KeyPage {
    #[serde(default = "first_page")]
    page: u32,
    #[serde(default = "default_page_size")]
    page_size: u32,
}
fn first_page() -> u32 {
    1
}
fn default_page_size() -> u32 {
    20
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateKey {
    name: String,
    expires_at: Option<String>,
}
pub(crate) async fn create_api_key(
    State(s): State<GatewayState>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    Json(body): Json<CreateKey>,
) -> Response {
    execute(s, tenant, user.id, KeyCommand::Create { name: body.name, expires_at: body.expires_at })
        .await
}
pub(crate) async fn revoke_api_key(
    State(s): State<GatewayState>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    Path(id): Path<String>,
) -> Response {
    execute(s, tenant, user.id, KeyCommand::Revoke(id)).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ValidateKey {
    api_key: String,
}
pub(crate) async fn validate_api_key(
    State(s): State<GatewayState>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    Json(body): Json<ValidateKey>,
) -> Response {
    let iam = s.iam.clone();
    match tokio::task::spawn_blocking(move || {
        iam.check_permission_admin_scope(&tenant,&user.id,None,None)?;
        let principal = iam.authenticate_api_key_in_tenant(&body.api_key,&tenant)?;
        Ok::<_,AdminError>(principal.filter(|p|p.tenant_id == tenant).map(|p|json!({"valid":true,"user_id":p.id,"tenant_id":p.tenant_id,"allowed_workflows":["inbox","notifications"]})).unwrap_or(json!({"valid":false})))
    }).await {
        Ok(Ok(data)) => Json(json!({"code":0,"data":data})).into_response(),
        Ok(Err(error)) => failure(error),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
pub(crate) async fn audit_log(
    State(s): State<GatewayState>,
    TenantId(tenant): TenantId,
    ApiAuth(user): ApiAuth,
    Query(q): Query<AuditQuery>,
) -> Response {
    match tokio::task::spawn_blocking(move || s.iam.query_admin_audit(&tenant, &user.id, q)).await {
        Ok(Ok(data)) => Json(json!({"code":0,"data":data})).into_response(),
        Ok(Err(error)) => failure(error),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
