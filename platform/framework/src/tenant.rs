// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 多租户 — 三档隔离（逻辑前缀/Schema/集群），零配置默认逻辑隔离

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 租户隔离模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TenantMode {
    None,      // 单租户
    Logical,   // 逻辑隔离（租户ID前缀）
    Schema,    // Schema隔离（每租户独立Schema）
    Cluster,   // 集群隔离（每租户独立集群）
}

/// 租户上下文（贯穿整个请求）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    pub tenant_id: String,
    pub tenant_name: String,
    pub mode: TenantMode,
    pub plan: String, // 套餐: free/pro/enterprise
}

impl TenantContext {
    /// 默认系统租户
    pub fn system() -> Self {
        Self {
            tenant_id: "system".into(),
            tenant_name: "System".into(),
            mode: TenantMode::Logical,
            plan: "enterprise".into(),
        }
    }

    /// 生成租户前缀（用于逻辑隔离）
    pub fn prefix(&self) -> String {
        format!("{}:", self.tenant_id)
    }

    /// 为key添加租户前缀
    pub fn with_prefix(&self, key: &str) -> String {
        format!("{}{}", self.prefix(), key)
    }

    /// 从key中提取原始key（去除租户前缀）
    pub fn strip_prefix<'a>(&self, key: &'a str) -> &'a str {
        key.strip_prefix(&self.prefix()).unwrap_or(key)
    }
}

/// 租户中间件状态
#[derive(Clone)]
pub struct TenantState {
    pub mode: TenantMode,
    pub default_tenant: Arc<String>,
}

impl TenantState {
    pub fn new(mode: TenantMode, default_tenant: impl Into<String>) -> Self {
        Self {
            mode,
            default_tenant: Arc::new(default_tenant.into()),
        }
    }
}

/// 租户中间件（从x-tenant-id头提取租户，注入TenantContext）
pub async fn tenant_middleware(
    axum::extract::State(state): axum::extract::State<TenantState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let tenant_id = req
        .headers()
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(&state.default_tenant)
        .to_string();

    let ctx = TenantContext {
        tenant_id: tenant_id.clone(),
        tenant_name: tenant_id,
        mode: state.mode,
        plan: "pro".into(),
    };

    req.extensions_mut().insert(ctx);
    Ok(next.run(req).await)
}

/// 从request提取租户上下文
pub fn extract_tenant(req: &Request) -> Option<&TenantContext> {
    req.extensions().get::<TenantContext>()
}
