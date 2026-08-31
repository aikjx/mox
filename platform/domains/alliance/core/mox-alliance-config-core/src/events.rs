// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 配置变更事件

use chrono::{DateTime, Utc};
use mox_alliance_common_proto::ConfigType;
use serde::{Deserialize, Serialize};

/// 配置变更事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigChangeType {
    /// 新增配置
    Created,
    /// 更新配置
    Updated,
    /// 删除配置
    Deleted,
    /// 回滚配置
    RolledBack,
    /// 启用配置
    Enabled,
    /// 禁用配置
    Disabled,
}

/// 配置变更事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    /// 事件唯一 ID
    pub event_id: String,
    /// 模块 ID
    pub module_id: String,
    /// 配置类型
    pub config_type: ConfigType,
    /// 变更类型
    pub change_type: ConfigChangeType,
    /// 变更前版本
    pub old_version: Option<u32>,
    /// 变更后版本
    pub new_version: u32,
    /// 变更人
    pub changed_by: String,
    /// 变更原因
    pub change_reason: String,
    /// 事件时间
    pub timestamp: DateTime<Utc>,
}

impl ConfigChangeEvent {
    pub fn new(
        module_id: String,
        config_type: ConfigType,
        change_type: ConfigChangeType,
        old_version: Option<u32>,
        new_version: u32,
        changed_by: String,
        change_reason: String,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            module_id,
            config_type,
            change_type,
            old_version,
            new_version,
            changed_by,
            change_reason,
            timestamp: Utc::now(),
        }
    }
}
