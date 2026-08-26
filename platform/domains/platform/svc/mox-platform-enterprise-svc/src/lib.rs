//! enterprise-svc：企业级真源服务库
//!
//! 架构：纯 Rust 实现，4 个平台核心库负责数据；本 crate 负责 axum HTTP 暴露、
//! 认证（JWT HS256）、路由、反序列化。与 Node.js BFF（端口 3000）通过
//! RUST_ENTERPRISE_URL=http://localhost:3002 协同。

pub mod app_state;
pub mod auth;
pub mod routes;

pub use auth::{generate_token, verify_token, AuthState, Claims};
