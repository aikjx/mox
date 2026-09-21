// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家注册中心数据模型
//!
//! 本 crate 维护两类「专家」实体，二者职责不同、互不混淆：
//! - [`Expert`]：静态专家目录（人工维护的专家档案，SQLite 持久化），
//!   对应旧 `/api/v1/experts` 目录 CRUD，保持向后兼容。
//! - [`RegisteredInstance`]：**应用级专家实例注册**（专家进程实例注册自身的
//!   端点/能力/心跳/健康状态），对应本服务核心 `/api/registry/experts` 能力。
//!
//! 注册中心是应用级实例注册表，不是 Nacos 命名注册的替代品——
//! 它管理专家实例的元数据、能力标签与健康状态，供调度器/网关按能力发现实例。

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── 静态专家目录（旧版，SQLite 持久化，向后兼容）─────────────────────────

/// 静态专家档案（目录型，非运行实例）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expert {
    /// 专家 ID
    pub id: String,
    /// 专家名称
    pub name: String,
    /// 专家标题
    pub title: String,
    /// 所属组织
    pub organization: String,
    /// 领域标签
    pub domains: Vec<String>,
    /// 技能标签
    pub skills: Vec<String>,
    /// 简介
    pub bio: String,
    /// 是否启用
    pub enabled: bool,
    /// 评分
    pub rating: f32,
    /// 总咨询次数
    pub total_consultations: u32,
}

impl Expert {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            title: String::new(),
            organization: String::new(),
            domains: Vec::new(),
            skills: Vec::new(),
            bio: String::new(),
            enabled: true,
            rating: 0.0,
            total_consultations: 0,
        }
    }
}

/// 创建专家请求
#[derive(Debug, Deserialize)]
pub struct CreateExpertRequest {
    pub name: String,
    pub title: Option<String>,
    pub organization: Option<String>,
    pub domains: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    pub bio: Option<String>,
}

/// 更新专家请求
#[derive(Debug, Deserialize)]
pub struct UpdateExpertRequest {
    pub name: Option<String>,
    pub title: Option<String>,
    pub organization: Option<String>,
    pub domains: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    pub bio: Option<String>,
    pub enabled: Option<bool>,
}

// ─── 应用级专家实例注册（核心）─────────────────────────────────────────────

/// 实例健康/生命周期状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InstanceStatus {
    /// 活跃：近期心跳在租约内，可被发现与调度
    #[default]
    Active,
    /// 不健康：被动（心跳停止但租约未到）或主动探测失败标记
    Unhealthy,
    /// 优雅下线中：不接收新发现，存量连接继续处理
    Draining,
    /// 已过期：心跳超过租约被后台回收摘除
    Expired,
}

impl InstanceStatus {
    /// 是否可被「发现」端点返回（活跃 + 优雅下线中仍可见，便于观测）
    pub fn is_discoverable(self) -> bool {
        matches!(self, InstanceStatus::Active | InstanceStatus::Draining)
    }
}

/// 注册的专家实例（注册中心核心实体）
///
/// 专家进程启动后调用注册接口把自身元数据与端点登记到注册中心，
/// 并周期上报心跳续约；超过租约未续约即被后台任务摘除。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredInstance {
    /// 实例 ID（注册方可自带，缺省自动生成 UUID）
    pub id: String,
    /// 实例名称（通常为专家模块名，如 `expert-code`）
    pub name: String,
    /// 实例版本（语义化版本，如 `1.0.0`）
    pub version: String,
    /// 服务端点（gRPC/HTTP base URL）
    pub endpoint: String,
    /// 主动健康检查 URL（注册中心可后台探测；为空则仅依赖被动心跳）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check_url: Option<String>,
    /// 能力标签（如 `programming`/`code`/`math`）
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// 所属领域（如 `code`/`math`/`medical`）
    #[serde(default)]
    pub domain: Option<String>,
    /// 权重（负载均衡用，默认 1.0）
    pub weight: f32,
    /// 当前负载（在处理中的请求数）
    pub load_current: u32,
    /// 负载容量上限
    pub load_capacity: u32,
    /// 实例状态
    pub status: InstanceStatus,
    /// 注册时间
    pub registered_at: DateTime<Utc>,
    /// 最近一次心跳时间
    pub last_heartbeat_at: DateTime<Utc>,
    /// 心跳租约（秒）：距上次心跳超过该时长即过期
    pub lease_seconds: u32,
    /// 自由元数据（键值对，如机房/分区/协议版本）
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl RegisteredInstance {
    /// 在 `now` 时刻是否仍在心跳租约内（被动健康判定）
    pub fn is_alive_at(&self, now: DateTime<Utc>) -> bool {
        self.last_heartbeat_at + chrono::Duration::seconds(self.lease_seconds as i64) >= now
    }

    /// 是否已过租约（需要被回收）
    pub fn is_expired_at(&self, now: DateTime<Utc>) -> bool {
        !self.is_alive_at(now)
    }
}

/// 注册请求（实例注册自身）
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// 实例 ID；缺省由注册中心生成
    pub id: Option<String>,
    /// 实例名称（必填）
    pub name: String,
    /// 版本（缺省 `0.1.0`）
    #[serde(default = "default_version")]
    pub version: String,
    /// 服务端点（必填）
    pub endpoint: String,
    /// 主动健康检查 URL
    pub health_check_url: Option<String>,
    /// 能力标签
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// 所属领域
    pub domain: Option<String>,
    /// 权重（缺省 1.0）
    #[serde(default = "default_weight")]
    pub weight: f32,
    /// 当前负载（缺省 0）
    #[serde(default)]
    pub load_current: u32,
    /// 容量上限（缺省 100）
    #[serde(default = "default_capacity")]
    pub load_capacity: u32,
    /// 心跳租约秒数（缺省 15）
    #[serde(default = "default_lease")]
    pub lease_seconds: u32,
    /// 自由元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// 心跳续约请求
#[derive(Debug, Default, Deserialize)]
pub struct HeartbeatRequest {
    /// 上报当前负载（缺省保持原值）
    pub load_current: Option<u32>,
    /// 心跳时上报期望状态（如主动进入 Draining）
    pub status: Option<InstanceStatus>,
}

/// 实例发现/列表查询参数
#[derive(Debug, Default, Deserialize)]
pub struct InstanceQuery {
    /// 按能力标签精确/包含匹配
    pub capability: Option<String>,
    /// 按领域匹配
    pub domain: Option<String>,
    /// 按状态过滤（lowercase：active/unhealthy/draining/expired）
    pub status: Option<String>,
    /// 按名称子串匹配
    pub name: Option<String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}
fn default_weight() -> f32 {
    1.0
}
fn default_capacity() -> u32 {
    100
}
fn default_lease() -> u32 {
    15
}
