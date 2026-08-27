//! 多协议网关 — Multi-Protocol Gateway
//!
//! 企业级多协议统一接入：REST/gRPC/GraphQL/WebSocket统一管理。
//!
//! ## 核心能力
//! - [`traits`] — 协议处理器抽象trait
//! - [`grpc`] — gRPC服务注册
//! - [`graphql`] — GraphQL schema管理
//! - [`websocket`] — WebSocket连接管理
//! - [`router`] — 统一协议路由器

pub mod graphql;
pub mod grpc;
pub mod router;
pub mod traits;
pub mod websocket;

// 重导出
pub use graphql::{GraphQLEndpoint, GraphQLSchema, GraphQLSchemaRegistry};
pub use grpc::{GrpcService, GrpcServiceRegistry};
pub use router::{ProtocolRouter, RoutingResult};
pub use traits::{ProtocolHandler, ProtocolRequest, ProtocolResponse, ProtocolType};
pub use websocket::{WebSocketConnection, WebSocketManager, WebSocketMessage};
