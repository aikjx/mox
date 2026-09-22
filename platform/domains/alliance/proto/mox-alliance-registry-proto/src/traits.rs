// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 注册中心行为契约（trait）
//!
//! registry-svc 的存储实现绑定这两个契约；消费方（调度器、网关 typed client）
//! 可依赖契约而非 HTTP JSON 裸值。传输当前为 HTTP，`proto/registry.proto`
//! 为未来 gRPC 传输保留同一语义的规范定义。

use crate::types::{
    Expert, InstanceQuery, InstanceStatus, RegisteredInstance,
};

/// 静态专家目录契约（`/api/v1/experts` 语义）
pub trait ExpertDirectory: Send + Sync {
    fn list_experts(&self) -> crate::RegistryResult<Vec<Expert>>;
    fn get_expert(&self, id: &str) -> crate::RegistryResult<Option<Expert>>;
    fn create_expert(&self, expert: &Expert) -> crate::RegistryResult<()>;
    fn update_expert(&self, expert: &Expert) -> crate::RegistryResult<()>;
    fn delete_expert(&self, id: &str) -> crate::RegistryResult<()>;
}

/// 应用级专家实例注册契约（`/api/registry/experts` 语义）
pub trait InstanceRegistry: Send + Sync {
    /// 登记实例（幂等覆盖同 ID）
    fn register_instance(&self, inst: RegisteredInstance);
    /// 注销实例，返回被摘除者
    fn deregister_instance(&self, id: &str) -> Option<RegisteredInstance>;
    /// 按 ID 查实例
    fn get_instance(&self, id: &str) -> Option<RegisteredInstance>;
    /// 心跳续约（可选上报负载/状态）
    fn heartbeat(
        &self,
        id: &str,
        load_current: Option<u32>,
        status: Option<InstanceStatus>,
    ) -> Option<RegisteredInstance>;
    /// 按能力/领域/状态/名称发现实例
    fn discover(&self, query: &InstanceQuery) -> Vec<RegisteredInstance>;
    /// 当前登记实例总数
    fn instance_count(&self) -> usize;
}
