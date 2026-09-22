// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Registry Proto — 注册中心协议层
//!
//! 专家注册中心的接口契约定义（六层架构 proto 层），包括：
//! - [`Expert`] / [`RegisteredInstance`] 等数据契约（DTO）
//! - [`ExpertDirectory`]：静态专家目录 CRUD 契约
//! - [`InstanceRegistry`]：应用级专家实例注册/发现/心跳契约
//!
//! ## 设计原则
//! - **DIP 依赖倒置**：registry-svc 实现本 crate 的 trait；消费方（调度器/网关）
//!   依赖契约而非服务内部类型。
//! - **SSOT 单一真相源**：注册中心的 DTO 与契约只有这里一个权威定义；
//!   registry-svc 的 `models` 模块仅为兼容再导出。
//! - **协议先行**：`proto/registry.proto` 保留为未来 gRPC 传输的规范契约，
//!   当前运行时传输为 HTTP（网关经 `RegistryClient` 调用，见 docs §2.2「gRPC 未使用」）。

pub mod errors;
pub mod traits;
pub mod types;

pub use errors::{RegistryError, RegistryResult};
pub use traits::{ExpertDirectory, InstanceRegistry};
pub use types::{
    CreateExpertRequest, Expert, HeartbeatRequest, InstanceQuery, InstanceStatus, RegisterRequest,
    RegisteredInstance, UpdateExpertRequest,
};
