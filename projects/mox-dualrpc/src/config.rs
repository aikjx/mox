//! Configuration for the dual-protocol RPC server.

use std::time::Duration;

/// Server configuration
#[derive(Debug, Clone)]
pub struct DualRpcConfig {
    /// gRPC server address (e.g., "0.0.0.0:50051")
    pub grpc_addr: String,
    /// JSON-RPC HTTP server address (e.g., "0.0.0.0:8080")
    pub jsonrpc_addr: String,
    /// JSON-RPC endpoint path
    pub jsonrpc_path: String,
    /// MCP endpoint path (JSON-RPC subset)
    pub mcp_path: String,
    /// Health check endpoint
    pub health_path: String,
    /// Metrics endpoint
    pub metrics_path: String,
    /// Max concurrent JSON-RPC requests
    pub max_concurrent_requests: usize,
    /// Request timeout
    pub request_timeout: Duration,
    /// Enable response caching
    pub cache_enabled: bool,
    /// Cache max capacity
    pub cache_max_capacity: u64,
    /// Cache default TTL
    pub cache_default_ttl: Duration,
    /// Enable CORS
    pub cors_enabled: bool,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// Enable compression
    pub compression_enabled: bool,
    /// Log level
    pub log_level: String,
    /// Enable OpenTelemetry tracing
    pub tracing_enabled: bool,
}

impl Default for DualRpcConfig {
    fn default() -> Self {
        Self {
            grpc_addr: "0.0.0.0:50051".into(),
            jsonrpc_addr: "0.0.0.0:8080".into(),
            jsonrpc_path: "/rpc".into(),
            mcp_path: "/mcp".into(),
            health_path: "/health".into(),
            metrics_path: "/metrics".into(),
            max_concurrent_requests: 1000,
            request_timeout: Duration::from_secs(30),
            cache_enabled: true,
            cache_max_capacity: 10_000,
            cache_default_ttl: Duration::from_secs(5),
            cors_enabled: true,
            cors_origins: vec!["*".into()],
            compression_enabled: true,
            log_level: "info".into(),
            tracing_enabled: true,
        }
    }
}

impl DualRpcConfig {
    pub fn builder() -> DualRpcConfigBuilder {
        DualRpcConfigBuilder::default()
    }
}

/// Builder for DualRpcConfig
#[derive(Debug, Clone, Default)]
pub struct DualRpcConfigBuilder {
    config: DualRpcConfig,
}

impl DualRpcConfigBuilder {
    pub fn grpc_addr(mut self, addr: impl Into<String>) -> Self {
        self.config.grpc_addr = addr.into();
        self
    }

    pub fn jsonrpc_addr(mut self, addr: impl Into<String>) -> Self {
        self.config.jsonrpc_addr = addr.into();
        self
    }

    pub fn jsonrpc_path(mut self, path: impl Into<String>) -> Self {
        self.config.jsonrpc_path = path.into();
        self
    }

    pub fn mcp_path(mut self, path: impl Into<String>) -> Self {
        self.config.mcp_path = path.into();
        self
    }

    pub fn max_concurrent_requests(mut self, n: usize) -> Self {
        self.config.max_concurrent_requests = n;
        self
    }

    pub fn request_timeout(mut self, d: Duration) -> Self {
        self.config.request_timeout = d;
        self
    }

    pub fn cache_enabled(mut self, enabled: bool) -> Self {
        self.config.cache_enabled = enabled;
        self
    }

    pub fn cors_enabled(mut self, enabled: bool) -> Self {
        self.config.cors_enabled = enabled;
        self
    }

    pub fn cors_origins(mut self, origins: Vec<String>) -> Self {
        self.config.cors_origins = origins;
        self
    }

    pub fn log_level(mut self, level: impl Into<String>) -> Self {
        self.config.log_level = level.into();
        self
    }

    pub fn tracing_enabled(mut self, enabled: bool) -> Self {
        self.config.tracing_enabled = enabled;
        self
    }

    pub fn build(self) -> DualRpcConfig {
        self.config
    }
}
