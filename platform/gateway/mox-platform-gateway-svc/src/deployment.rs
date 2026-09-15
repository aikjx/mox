//! 同一个宿主二进制复用融合模式的业务路由；独立角色不装配其他业务域。
use axum::{extract::Request, middleware::{from_fn, Next}, Router};
use crate::{auth::auth_middleware, modules::upgrade, GatewayState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostRole { All, Kg, Cloud, Kb, Iam }

impl std::str::FromStr for HostRole {
    type Err = std::io::Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "all" => Ok(Self::All), "kg" => Ok(Self::Kg),
            "cloud" => Ok(Self::Cloud), "kb" => Ok(Self::Kb), "iam" => Ok(Self::Iam),
            _ => Err(std::io::Error::new(std::io::ErrorKind::InvalidInput,
                "MOX_HOST_ROLE must be all, kg, cloud, kb or iam")),
        }
    }
}

pub fn domain_router(role: HostRole, gateway: &GatewayState) -> Router<GatewayState> {
    let router = match role {
        // 现有适配器同时承载 KG、graph/v1 与 AI engine 契约，保持路由兼容。
        HostRole::Kg => upgrade(crate::http_adapter::build_kg_ai_router()),
        HostRole::Cloud => upgrade(crate::cloud::build_cloud_router()),
        HostRole::Kb => upgrade(Router::new().nest("/api", mox_kb_svc::handlers::build_kb_router())),
        HostRole::Iam => crate::system::build_system_router()
            .merge(crate::system::build_security_router())
            .merge(crate::rbac::build_rbac_router())
            .nest("/api/enterprise/sso", crate::sso::api::build_sso_router::<GatewayState>()),
        HostRole::All => unreachable!("all uses the existing module registry"),
    };
    let auth = gateway.auth.clone();
    router.route_layer(from_fn(move |request: Request, next: Next| {
        let auth = auth.clone();
        async move { auth_middleware(auth, request, next).await }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_role_rejects_unknown_or_empty_values() {
        for value in ["all", "kg", "cloud", "kb", "iam"] {
            assert!(value.parse::<HostRole>().is_ok());
        }
        for value in ["", "ALL", "kbg", "kg,cloud"] {
            assert!(value.parse::<HostRole>().is_err());
        }
    }
}
