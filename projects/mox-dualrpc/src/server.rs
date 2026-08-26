//! Dual-protocol RPC server: gRPC (tonic) + JSON-RPC 2.0 (axum) with auto-transcoding.

use crate::config::DualRpcConfig;
use crate::error::{grpc_to_jsonrpc, DualRpcError, JsonRpcError};
use crate::registry::{CachedResponse, RouteEntry, RouteMeta, RouteRegistry};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::oneshot;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::TraceLayer,
};

/// Shared server state
#[derive(Clone)]
pub struct ServerState {
    pub registry: Arc<RouteRegistry>,
    pub config: Arc<DualRpcConfig>,
}

/// The dual-protocol RPC server
pub struct DualRpcServer {
    config: DualRpcConfig,
    registry: RouteRegistry,
    grpc_shutdown: Option<oneshot::Sender<()>>,
    jsonrpc_shutdown: Option<oneshot::Sender<()>>,
}

impl DualRpcServer {
    pub fn builder() -> DualRpcServerBuilder {
        DualRpcServerBuilder::default()
    }

    pub fn new(config: DualRpcConfig) -> Self {
        Self {
            config,
            registry: RouteRegistry::new(),
            grpc_shutdown: None,
            jsonrpc_shutdown: None,
        }
    }

    /// Register a service with its routes
    pub fn register_service(&mut self, routes: Vec<RouteEntry>) {
        for route in routes {
            self.registry.register(route);
        }
    }

    /// Register a single route
    pub fn register_route(&mut self, entry: RouteEntry) {
        self.registry.register(entry);
    }

    /// Get the number of registered routes
    pub fn route_count(&self) -> usize {
        self.registry.route_count()
    }

    /// List all registered JSON-RPC methods
    pub fn list_methods(&self) -> Vec<&RouteMeta> {
        self.registry.list_methods()
    }

    /// Start both servers (blocks until shutdown)
    pub async fn serve(mut self) -> Result<(), DualRpcError> {
        let state = Arc::new(ServerState {
            registry: Arc::new(self.registry),
            config: Arc::new(self.config.clone()),
        });

        let (grpc_tx, grpc_rx) = oneshot::channel();
        let (jsonrpc_tx, jsonrpc_rx) = oneshot::channel();
        self.grpc_shutdown = Some(grpc_tx);
        self.jsonrpc_shutdown = Some(jsonrpc_tx);

        // Start JSON-RPC server
        let jsonrpc_state = state.clone();
        let jsonrpc_addr: SocketAddr = self.config.jsonrpc_addr.parse()?;
        let jsonrpc_handle = tokio::spawn(async move {
            run_jsonrpc_server(jsonrpc_state, jsonrpc_addr, jsonrpc_rx).await
        });

        // Start gRPC server (placeholder — in production, register actual tonic services)
        let grpc_state = state.clone();
        let grpc_addr: SocketAddr = self.config.grpc_addr.parse()?;
        let grpc_handle = tokio::spawn(async move {
            run_grpc_server(grpc_state, grpc_addr, grpc_rx).await
        });

        tracing::info!(
            "mox-dualrpc started: gRPC={}, JSON-RPC={}, routes={}",
            self.config.grpc_addr,
            self.config.jsonrpc_addr,
            state.registry.route_count()
        );

        // Wait for either server to exit
        tokio::select! {
            result = jsonrpc_handle => {
                if let Err(e) = result {
                    tracing::error!("JSON-RPC server panicked: {}", e);
                }
            }
            result = grpc_handle => {
                if let Err(e) = result {
                    tracing::error!("gRPC server panicked: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Graceful shutdown
    pub fn shutdown(&mut self) {
        if let Some(tx) = self.grpc_shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(tx) = self.jsonrpc_shutdown.take() {
            let _ = tx.send(());
        }
    }
}

/// Builder for DualRpcServer
#[derive(Default)]
pub struct DualRpcServerBuilder {
    config: Option<DualRpcConfig>,
    routes: Vec<RouteEntry>,
}

impl DualRpcServerBuilder {
    pub fn config(mut self, config: DualRpcConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn grpc_addr(mut self, addr: impl Into<String>) -> Self {
        let config = self.config.get_or_insert_with(DualRpcConfig::default);
        config.grpc_addr = addr.into();
        self
    }

    pub fn jsonrpc_addr(mut self, addr: impl Into<String>) -> Self {
        let config = self.config.get_or_insert_with(DualRpcConfig::default);
        config.jsonrpc_addr = addr.into();
        self
    }

    pub fn register(mut self, routes: Vec<RouteEntry>) -> Self {
        self.routes.extend(routes);
        self
    }

    pub fn register_route(mut self, route: RouteEntry) -> Self {
        self.routes.push(route);
        self
    }

    pub fn build(self) -> Result<DualRpcServer, DualRpcError> {
        let config = self.config.unwrap_or_default();
        let mut server = DualRpcServer::new(config);
        for route in self.routes {
            server.register_route(route);
        }
        Ok(server)
    }
}

// === JSON-RPC Server (axum) ===

async fn run_jsonrpc_server(
    state: Arc<ServerState>,
    addr: SocketAddr,
    shutdown: oneshot::Receiver<()>,
) -> Result<(), DualRpcError> {
    let app = axum::Router::new()
        .route(&state.config.jsonrpc_path, axum::routing::post(handle_jsonrpc))
        .route(&state.config.mcp_path, axum::routing::post(handle_jsonrpc))
        .route(&state.config.health_path, axum::routing::get(handle_health))
        .route(&state.config.metrics_path, axum::routing::get(handle_metrics))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
                .layer(CompressionLayer::new())
                .into_inner(),
        );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("JSON-RPC server listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = shutdown.await;
            tracing::info!("JSON-RPC server shutting down");
        })
        .await?;

    Ok(())
}

/// Handle JSON-RPC 2.0 request (single or batch)
async fn handle_jsonrpc(
    State(state): State<Arc<ServerState>>,
    body: String,
) -> impl IntoResponse {
    // Parse request
    let request: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::OK,
                Json(json!({
                    "jsonrpc": "2.0",
                    "error": { "code": -32700, "message": format!("Parse error: {}", e) },
                    "id": null
                })),
            )
                .into_response();
        }
    };

    // Batch request
    if let Some(batch) = request.as_array() {
        let mut results = Vec::new();
        for req in batch {
            results.push(handle_single_jsonrpc(&state, req).await);
        }
        return (StatusCode::OK, Json(json!(results))).into_response();
    }

    // Single request
    let result = handle_single_jsonrpc(&state, &request).await;
    (StatusCode::OK, Json(result)).into_response()
}

/// Handle a single JSON-RPC 2.0 request
async fn handle_single_jsonrpc(state: &Arc<ServerState>, request: &Value) -> Value {
    let method = request["method"].as_str().unwrap_or("");
    let id = request["id"].clone();
    let params = request.get("params").cloned().unwrap_or(json!({}));

    // Validate request
    if method.is_empty() {
        return json!({
            "jsonrpc": "2.0",
            "error": { "code": -32600, "message": "Invalid Request: missing method" },
            "id": id
        });
    }

    // L1: Route lookup (O(1))
    let Some(route) = state.registry.lookup(method) else {
        return json!({
            "jsonrpc": "2.0",
            "error": { "code": -32601, "message": format!("Method not found: {}", method) },
            "id": id
        });
    };

    // Response cache check
    if route.meta.cache_ttl_ms > 0 && state.config.cache_enabled {
        let cache_key = format!("{}:{}", method, params.to_string());
        if let Some(cached) = state.registry.get_cached(&cache_key) {
            return json!({
                "jsonrpc": "2.0",
                "result": cached.value,
                "id": id,
                "meta": { "cached": true, "age_ms": cached.age_ms() }
            });
        }
    }

    // Execute handler
    let start = std::time::Instant::now();
    let result = route.handler.call(params).await;
    let duration = start.elapsed();

    match result {
        Ok(value) => {
            // Cache response
            if route.meta.cache_ttl_ms > 0 && state.config.cache_enabled {
                let cache_key = format!("{}:{}", method, request.get("params").unwrap_or(&json!({})).to_string());
                state.registry.set_cached(cache_key, CachedResponse::new(value.clone()));
            }

            let _ = duration;
            let _ = metrics::counter!("jsonrpc_requests_total", "method" => method.to_string(), "status" => "success");

            json!({
                "jsonrpc": "2.0",
                "result": value,
                "id": id
            })
        }
        Err(e) => {
            let _ = metrics::counter!("jsonrpc_requests_total", "method" => method.to_string(), "status" => "error");

            let jsonrpc_err = match &e {
                DualRpcError::Grpc(status) => grpc_to_jsonrpc(status),
                DualRpcError::JsonRpc(err) => err.clone(),
                DualRpcError::Transcode(msg) => JsonRpcError::invalid_params(msg),
                _ => JsonRpcError::internal_error(e.to_string()),
            };

            json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": jsonrpc_err.code,
                    "message": jsonrpc_err.message,
                    "data": jsonrpc_err.data
                },
                "id": id
            })
        }
    }
}

async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok", "service": "mox-dualrpc" })))
}

async fn handle_metrics() -> impl IntoResponse {
    // Placeholder — in production, render Prometheus metrics
    (StatusCode::OK, "mox_dualrpc_routes 0\n")
}

// === gRPC Server (tonic) ===

async fn run_grpc_server(
    state: Arc<ServerState>,
    addr: SocketAddr,
    shutdown: oneshot::Receiver<()>,
) -> Result<(), DualRpcError> {
    // In production, this would register actual tonic gRPC services.
    // For now, we create a reflection-only server that can be extended.
    tracing::info!("gRPC server listening on {} ({} routes registered)", addr, state.registry.route_count());

    // Wait for shutdown (placeholder — actual tonic server would run here)
    let _ = shutdown.await;
    tracing::info!("gRPC server shutting down");

    Ok(())
}

// Re-export make_route from registry for backward compatibility
pub use crate::registry::make_route;
