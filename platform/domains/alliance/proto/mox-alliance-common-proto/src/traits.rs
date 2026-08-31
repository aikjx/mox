// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 通用抽象 trait
//!
//! 所有联盟服务共享的 trait 抽象，确保服务生命周期和多租户能力的一致性。

use async_trait::async_trait;

use crate::AllianceResult;

/// 服务生命周期 trait
///
/// 所有联盟服务都应该实现这个 trait，确保统一的启动/停止/健康检查模式。
#[async_trait]
pub trait ServiceLifecycle: Send + Sync {
    /// 服务名称
    fn service_name(&self) -> &str;

    /// 启动服务
    async fn start(&self) -> AllianceResult<()>;

    /// 优雅停止服务
    async fn stop(&self) -> AllianceResult<()>;

    /// 健康检查
    async fn health_check(&self) -> AllianceResult<bool> {
        Ok(true)
    }
}

/// 多租户感知 trait
///
/// 所有处理租户数据的服务都应该实现这个 trait，
/// 确保租户隔离的一致性。
pub trait TenantAware {
    /// 当前操作的租户 ID
    fn tenant_id(&self) -> &str;

    /// 检查资源是否属于当前租户
    fn check_tenant(&self, resource_tenant: &str) -> AllianceResult<()> {
        if self.tenant_id() == resource_tenant || resource_tenant == "system" {
            Ok(())
        } else {
            Err(crate::AllianceError::new(
                crate::AllianceErrorCode::TenantMismatch,
                format!(
                    "Tenant mismatch: expected {}, got {}",
                    self.tenant_id(),
                    resource_tenant
                ),
            ))
        }
    }
}
