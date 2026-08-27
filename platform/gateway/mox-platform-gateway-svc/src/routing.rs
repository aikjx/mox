// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Request routing module.
//!
//! Routes incoming requests to the appropriate domain API handlers.
//! Uses L2 api trait contracts only — no direct L3/L4 dependencies.

use axum::{
    Json,
    response::Response,
    extract::Request,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Route definition mapping a path prefix to a domain handler.
#[derive(Debug, Clone)]
pub struct Route {
    pub path_prefix: String,
    pub domain: String,
    pub handler_type: HandlerType,
}

/// Type of handler for a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerType {
    /// Proxy to an upstream service.
    Proxy,
    /// Handle directly in-gateway (health, metrics, etc).
    Direct,
    /// Route to a domain API handler (uses L2 api contracts).
    Api,
}

/// The router that maps paths to handlers.
pub struct Router {
    routes: Vec<Route>,
    handlers: Arc<parking_lot::RwLock<HashMap<String, RouteHandler>>>,
}

type RouteHandler = Box<dyn Fn(Request) -> Response + Send + Sync>;

impl Router {
    /// Create a new router with default routes.
    pub fn new() -> Self {
        let mut routes = vec![
            Route { path_prefix: "/api/data".into(), domain: "data".into(), handler_type: HandlerType::Api },
            Route { path_prefix: "/api/ai".into(), domain: "ai".into(), handler_type: HandlerType::Api },
            Route { path_prefix: "/api/kg".into(), domain: "kg".into(), handler_type: HandlerType::Api },
            Route { path_prefix: "/api/cloud".into(), domain: "cloud".into(), handler_type: HandlerType::Api },
            Route { path_prefix: "/api/voice".into(), domain: "voice".into(), handler_type: HandlerType::Api },
            Route { path_prefix: "/api/flow".into(), domain: "flow".into(), handler_type: HandlerType::Api },
            Route { path_prefix: "/api/market".into(), domain: "market".into(), handler_type: HandlerType::Api },
            Route { path_prefix: "/api/platform".into(), domain: "platform".into(), handler_type: HandlerType::Api },
            Route { path_prefix: "/health".into(), domain: "system".into(), handler_type: HandlerType::Direct },
            Route { path_prefix: "/metrics".into(), domain: "system".into(), handler_type: HandlerType::Direct },
        ];
        routes.sort_by(|a, b| b.path_prefix.len().cmp(&a.path_prefix.len()));
        Self {
            routes,
            handlers: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    /// Add a custom route.
    pub fn add_route(&mut self, route: Route) {
        self.routes.push(route);
        self.routes.sort_by(|a, b| b.path_prefix.len().cmp(&a.path_prefix.len()));
    }

    /// Register a custom handler for a path.
    pub fn register_handler<F>(&self, path: &str, handler: F)
    where F: Fn(Request) -> Response + Send + Sync + 'static {
        self.handlers.write().insert(path.to_string(), Box::new(handler));
    }

    /// Match a path to a route.
    pub fn match_route(&self, path: &str) -> Option<&Route> {
        self.routes.iter().find(|r| path.starts_with(&r.path_prefix))
    }

    /// List all registered routes.
    pub fn list_routes(&self) -> &[Route] {
        &self.routes
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check response.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub domains: Vec<String>,
}

/// Health check handler.
pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_secs: 0,
        domains: vec!["data".into(), "ai".into(), "kg".into(), "cloud".into(), "voice".into(), "flow".into(), "market".into(), "platform".into()],
    })
}

/// Generic API handler that routes to domain handlers.
pub async fn api_handler(request: Request) -> Result<Json<serde_json::Value>, StatusCode> {
    let path = request.uri().path().to_string();
    let router = Router::new();

    if let Some(route) = router.match_route(&path) {
        let response = serde_json::json!({
            "status": "ok",
            "domain": route.domain,
            "path": path,
            "handler_type": format!("{:?}", route.handler_type),
            "message": "Request routed to domain API handler",
        });
        Ok(Json(response))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_matching() {
        let router = Router::new();
        assert!(router.match_route("/api/data/records").is_some());
        assert!(router.match_route("/api/ai/intent").is_some());
        assert!(router.match_route("/api/kg/nodes").is_some());
        assert!(router.match_route("/health").is_some());
        assert!(router.match_route("/unknown/path").is_none());
    }

    #[test]
    fn test_route_domain() {
        let router = Router::new();
        let route = router.match_route("/api/data/records").unwrap();
        assert_eq!(route.domain, "data");
        let route = router.match_route("/api/kg/nodes").unwrap();
        assert_eq!(route.domain, "kg");
    }

    #[test]
    fn test_custom_route() {
        let mut router = Router::new();
        router.add_route(Route {
            path_prefix: "/custom/api".into(),
            domain: "custom".into(),
            handler_type: HandlerType::Api,
        });
        assert!(router.match_route("/custom/api/test").is_some());
    }
}
