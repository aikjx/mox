//! # mox-dualrpc
//!
//! Enterprise-grade dual-protocol RPC framework: gRPC + JSON-RPC with
//! zero-config auto-transcoding.
//!
//! ## Features
//!
//! - **Dual protocol**: Expose the same service as both gRPC (tonic) and JSON-RPC 2.0
//! - **Zero-config**: `#[dual_rpc]` attribute macro handles all registration
//! - **Auto-transcoding**: JSON ↔ Protobuf via serde, type-safe, zero reflection
//! - **Three-level caching**: L0 compile-time routes, L1 process cache, L2 request transcoding
//! - **Enterprise grade**: Rate limiting, circuit breaking, observability, unified error mapping
//! - **MCP compatible**: JSON-RPC 2.0 transport works natively with Model Context Protocol
//!
//! ## Quick Start
//!
//! ```ignore
//! use mox_dualrpc::prelude::*;
//!
//! #[derive(Clone)]
//! struct HelloService;
//!
//! impl HelloService {
//!     #[dual_rpc(method = "hello.SayHello", cache_ttl_ms = 1000)]
//!     async fn say_hello(&self, req: HelloRequest) -> Result<HelloResponse, Status> {
//!         Ok(HelloResponse { message: format!("Hello, {}!", req.name) })
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let server = DualRpcServer::builder()
//!         .grpc_addr("0.0.0.0:50051")
//!         .jsonrpc_addr("0.0.0.0:8080")
//!         .register(HelloService)?
//!         .build()?;
//!
//!     server.serve().await?;
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod registry;
pub mod server;
pub mod transcoder;

/// Re-export the proc macro
pub use mox_dualrpc_macro::{dual_rpc, dual_rpc_service};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::config::DualRpcConfig;
    pub use crate::error::{DualRpcError, JsonRpcError, ToStatus};
    pub use crate::registry::{RouteEntry, RouteMeta, RouteRegistry};
    pub use crate::server::{DualRpcServer, make_route};
    pub use crate::transcoder::{JsonProtobufTranscoder, TranscodeResult};
    pub use mox_dualrpc_macro::{dual_rpc, dual_rpc_service};
    pub use tonic::{Request, Response, Status};
}
