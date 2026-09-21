// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家注册中心服务（Experts Registry Svc）
//!
//! v3 架构拆分后的独立服务，**应用级专家实例注册中心**，负责：
//! - 专家实例注册 / 注销 / 心跳续约
//! - 专家实例按能力/领域/状态发现
//! - 实例健康状态跟踪（心跳租约超时自动摘除）
//! - 元数据与负载信息管理
//!
//! 同时保留旧版静态专家目录 CRUD（`/api/v1/experts`）向后兼容。
//!
//! 端口：3400（见 docs/api/PORT-REGISTRY.md）
//!
//! 注意：本服务是应用级实例注册表，**不是 Nacos 命名注册的替代品**——
//! Nacos 由 mox-alliance-boot-config 负责服务发现/配置，本服务管理专家元数据与健康。

pub mod app_state;
// pub mod grpc;  // gRPC 代码有类型不匹配问题，暂时禁用
pub mod models;
pub mod routes;
pub mod server;
pub mod storage;

pub use app_state::{AppState, Config};
// pub use grpc::RegistryGrpcService;
pub use models::{
    CreateExpertRequest, Expert, HeartbeatRequest, InstanceQuery, InstanceStatus,
    RegisterRequest, RegisteredInstance, UpdateExpertRequest,
};
pub use server::run;
pub use storage::{ExpertStore, RegistryStore};
