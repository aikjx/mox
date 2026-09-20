// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家注册中心服务（Experts Registry Svc）
//!
//! v3 架构拆分后的独立服务，负责：
//! - 专家注册、更新、删除
//! - 专家查询、列表、搜索
//! - 专家指标与概览
//!
//! 端口：3400

pub mod app_state;
// pub mod grpc;  // gRPC 代码有类型不匹配问题，暂时禁用
pub mod models;
pub mod routes;
pub mod server;
pub mod storage;

pub use app_state::{AppState, Config};
// pub use grpc::RegistryGrpcService;
pub use models::{Expert, CreateExpertRequest, UpdateExpertRequest};
pub use server::run;
pub use storage::ExpertStore;
