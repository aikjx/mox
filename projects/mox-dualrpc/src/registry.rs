//! Route registry with L0 (compile-time) and L1 (process) caching.

use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;

/// Route metadata (L0 cache — generated at compile time by macro)
#[derive(Debug, Clone)]
pub struct RouteMeta {
    pub jsonrpc_method: &'static str,
    pub grpc_method: &'static str,
    pub cache_ttl_ms: u64,
    pub cache_key: Option<&'static str>,
    pub expose: bool,
    pub batch_supported: bool,
}

/// A registered route entry
#[derive(Clone)]
pub struct RouteEntry {
    pub meta: RouteMeta,
    pub handler: Arc<dyn RouteHandler>,
}

/// Trait for route handlers (type-erased for registry storage)
#[async_trait::async_trait]
pub trait RouteHandler: Send + Sync {
    async fn call(&self, params: serde_json::Value) -> Result<serde_json::Value, crate::error::DualRpcError>;
}

/// Route registry with multi-level caching
pub struct RouteRegistry {
    /// L1: O(1) HashMap lookup (initialized once)
    routes: HashMap<String, RouteEntry>,
    /// L1: Response cache (moka)
    response_cache: moka::sync::Cache<String, CachedResponse>,
}

/// Cached response entry
#[derive(Clone)]
pub struct CachedResponse {
    pub value: serde_json::Value,
    pub created_at: std::time::Instant,
}

impl CachedResponse {
    pub fn new(value: serde_json::Value) -> Self {
        Self { value, created_at: std::time::Instant::now() }
    }

    pub fn age_ms(&self) -> u64 {
        self.created_at.elapsed().as_millis() as u64
    }
}

impl RouteRegistry {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            response_cache: moka::sync::Cache::builder()
                .max_capacity(10_000)
                .time_to_live(std::time::Duration::from_secs(5))
                .build(),
        }
    }

    /// Register a route
    pub fn register(&mut self, entry: RouteEntry) {
        if entry.meta.expose {
            self.routes.insert(entry.meta.jsonrpc_method.to_string(), entry);
        }
    }

    /// Lookup a route by JSON-RPC method name (L1: O(1))
    pub fn lookup(&self, method: &str) -> Option<&RouteEntry> {
        self.routes.get(method)
    }

    /// Get all registered methods (for MCP tools/list, OpenAPI, etc.)
    pub fn list_methods(&self) -> Vec<&RouteMeta> {
        self.routes.values().map(|e| &e.meta).collect()
    }

    /// Response cache operations
    pub fn get_cached(&self, key: &str) -> Option<CachedResponse> {
        self.response_cache.get(key)
    }

    pub fn set_cached(&self, key: String, value: CachedResponse) {
        self.response_cache.insert(key, value);
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }
}

impl Default for RouteRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global registry for static route registration (L0 pattern)
pub static GLOBAL_REGISTRY: Lazy<DashMap<String, RouteMeta>> = Lazy::new(DashMap::new);

/// Register a route's metadata globally (called by generated code)
pub fn register_route_meta(meta: RouteMeta) {
    GLOBAL_REGISTRY.insert(meta.jsonrpc_method.to_string(), meta);
}

/// Helper to create a route entry from a handler function
pub fn make_route<F, Fut>(meta: RouteMeta, handler: F) -> RouteEntry
where
    F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<serde_json::Value, crate::error::DualRpcError>> + Send + 'static,
{
    struct FnHandler<F>(F);

    #[async_trait::async_trait]
    impl<F, Fut> RouteHandler for FnHandler<F>
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<serde_json::Value, crate::error::DualRpcError>> + Send,
    {
        async fn call(&self, params: serde_json::Value) -> Result<serde_json::Value, crate::error::DualRpcError> {
            (self.0)(params).await
        }
    }

    RouteEntry {
        meta,
        handler: Arc::new(FnHandler(handler)),
    }
}
