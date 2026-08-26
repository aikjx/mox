//! mox-dualrpc hello-world example (v0.2: #[dual_rpc_service] auto-registration)
//!
//! Demonstrates:
//! - #[dual_rpc_service] auto-scans impl block and generates register_routes()
//! - Zero manual make_route boilerplate
//! - Dual protocol: gRPC + JSON-RPC
//! - Response caching via #[dual_rpc(cache_ttl_ms = ...)]
//! - Error handling

use mox_dualrpc::prelude::*;
use serde::{Deserialize, Serialize};

// === Request/Response types ===

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HelloRequest {
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HelloResponse {
    message: String,
    timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AddRequest {
    a: i64,
    b: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AddResponse {
    result: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EchoRequest {
    text: String,
    repeat: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EchoResponse {
    text: String,
}

// === Service with #[dual_rpc_service] auto-registration ===

#[derive(Clone)]
struct HelloService {
    greeting: String,
}

impl HelloService {
    fn new(greeting: impl Into<String>) -> Self {
        Self { greeting: greeting.into() }
    }
}

#[dual_rpc_service]
impl HelloService {
    /// Say hello — cached for 5 seconds, exposed as JSON-RPC "hello.SayHello"
    #[dual_rpc(method = "hello.SayHello", cache_ttl_ms = 5000, cache_key = "$.name")]
    async fn say_hello(&self, req: HelloRequest) -> Result<HelloResponse, DualRpcError> {
        Ok(HelloResponse {
            message: format!("{}, {}!", self.greeting, req.name),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    /// Add two numbers — no cache, exposed as "math.Add"
    #[dual_rpc(method = "math.Add")]
    async fn add(&self, req: AddRequest) -> Result<AddResponse, DualRpcError> {
        Ok(AddResponse { result: req.a + req.b })
    }

    /// Echo text — demonstrates error handling, exposed as "util.Echo"
    #[dual_rpc(method = "util.Echo")]
    async fn echo(&self, req: EchoRequest) -> Result<EchoResponse, DualRpcError> {
        if req.text.is_empty() {
            return Err(DualRpcError::JsonRpc(JsonRpcError::invalid_params(
                "text must not be empty",
            )));
        }
        let repeat = req.repeat.unwrap_or(1);
        let text = vec![req.text; repeat as usize].join(" ");
        Ok(EchoResponse { text })
    }

    /// Internal method — NOT exposed as JSON-RPC (expose = false)
    #[dual_rpc(method = "internal.HealthCheck", expose = false)]
    async fn internal_health_check(&self, _req: HelloRequest) -> Result<HelloResponse, DualRpcError> {
        Ok(HelloResponse {
            message: "OK".into(),
            timestamp: 0,
        })
    }
}

// === Main ===

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,mox_dualrpc=debug".into()),
        )
        .init();

    // Create service
    let service = HelloService::new("Hello");

    // Auto-generated register_routes() — zero boilerplate!
    let routes = service.register_routes();

    println!("=== mox-dualrpc v0.2 Hello World (#[dual_rpc_service] auto-registration) ===");
    println!("Auto-registered {} routes:", routes.len());
    for r in &routes {
        println!(
            "  - {} (gRPC: {}, cache: {}ms, expose: {})",
            r.meta.jsonrpc_method, r.meta.grpc_method, r.meta.cache_ttl_ms, r.meta.expose
        );
    }
    println!();

    // Build and start server
    let server = DualRpcServer::builder()
        .grpc_addr("127.0.0.1:50051")
        .jsonrpc_addr("127.0.0.1:8080")
        .register(routes)
        .build()?;

    println!("Starting servers...");
    println!("  JSON-RPC: http://127.0.0.1:8080/rpc");
    println!("  MCP:      http://127.0.0.1:8080/mcp");
    println!("  Health:   http://127.0.0.1:8080/health");
    println!("  gRPC:     127.0.0.1:50051");
    println!();
    println!("Try: curl -X POST http://127.0.0.1:8080/rpc \\");
    println!(r#"  -H "Content-Type: application/json" \"#);
    println!("{}", r#"  -d '{"jsonrpc":"2.0","method":"hello.SayHello","params":{"name":"World"},"id":1}'"#);
    println!();

    server.serve().await?;
    Ok(())
}
