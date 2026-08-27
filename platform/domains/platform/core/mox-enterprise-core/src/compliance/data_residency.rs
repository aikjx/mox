// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 数据主权 — 数据地域存储约束（政企合规）
//!
//! 支持按租户/数据类型配置存储地域，确保数据不出境/不出域。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;

/// 存储地域
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ResidencyRegion {
    /// 中国大陆
    ChinaMainland,
    /// 中国香港
    HongKong,
    /// 新加坡
    Singapore,
    /// 美国
    UsEast,
    UsWest,
    /// 欧盟
    Europe,
    /// 日本
    Japan,
    /// 自定义（按code）
    Custom,
}

impl ResidencyRegion {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResidencyRegion::ChinaMainland => "cn-mainland",
            ResidencyRegion::HongKong => "cn-hk",
            ResidencyRegion::Singapore => "sg",
            ResidencyRegion::UsEast => "us-east",
            ResidencyRegion::UsWest => "us-west",
            ResidencyRegion::Europe => "eu",
            ResidencyRegion::Japan => "jp",
            ResidencyRegion::Custom => "custom",
        }
    }

    /// 是否允许跨境传输
    pub fn allows_cross_border(&self) -> bool {
        match self {
            ResidencyRegion::ChinaMainland => false, // 中国大陆数据原则上不出境
            _ => true,
        }
    }
}

/// 数据类型分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    /// 个人身份信息（PII）
    Pii,
    /// 敏感个人信息（SPI）
    Spi,
    /// 财务数据
    Financial,
    /// 健康医疗数据
    Health,
    /// 业务数据
    Business,
    /// 日志/审计
    Audit,
    /// 配置/元数据
    Metadata,
    /// 自定义
    Custom(String),
}

impl DataType {
    pub fn as_str(&self) -> &str {
        match self {
            DataType::Pii => "pii",
            DataType::Spi => "spi",
            DataType::Financial => "financial",
            DataType::Health => "health",
            DataType::Business => "business",
            DataType::Audit => "audit",
            DataType::Metadata => "metadata",
            DataType::Custom(s) => s,
        }
    }

    /// 数据敏感度等级（0-3，越高越敏感）
    pub fn sensitivity_level(&self) -> u8 {
        match self {
            DataType::Spi => 3,
            DataType::Health => 3,
            DataType::Pii => 2,
            DataType::Financial => 2,
            DataType::Audit => 1,
            DataType::Business => 1,
            DataType::Metadata => 0,
            DataType::Custom(_) => 1,
        }
    }
}

/// 数据主权策略
pub struct DataResidencyPolicy {
    /// 默认存储地域
    default_region: RwLock<ResidencyRegion>,
    /// 租户级地域覆盖：tenant_id -> region
    tenant_regions: RwLock<HashMap<String, ResidencyRegion>>,
    /// 数据类型级地域覆盖：(tenant_id, data_type) -> region
    type_regions: RwLock<HashMap<(String, String), ResidencyRegion>>,
    /// 是否强制数据不出境
    force_no_cross_border: RwLock<bool>,
}

impl DataResidencyPolicy {
    pub fn new(default_region: ResidencyRegion) -> Self {
        Self {
            default_region: RwLock::new(default_region),
            tenant_regions: RwLock::new(HashMap::new()),
            type_regions: RwLock::new(HashMap::new()),
            force_no_cross_border: RwLock::new(false),
        }
    }

    /// 设置默认地域
    pub fn set_default_region(&self, region: ResidencyRegion) {
        *self.default_region.write() = region;
    }

    /// 设置租户地域
    pub fn set_tenant_region(&self, tenant_id: &str, region: ResidencyRegion) {
        self.tenant_regions.write().insert(tenant_id.into(), region);
    }

    /// 设置数据类型地域
    pub fn set_type_region(&self, tenant_id: &str, data_type: &DataType, region: ResidencyRegion) {
        self.type_regions.write().insert((tenant_id.into(), data_type.as_str().into()), region);
    }

    /// 启用强制不出境
    pub fn enable_force_no_cross_border(&self) {
        *self.force_no_cross_border.write() = true;
    }

    /// 获取数据应存储的地域（按优先级：类型 > 租户 > 默认）
    pub fn resolve_region(&self, tenant_id: &str, data_type: &DataType) -> ResidencyRegion {
        // 1. 数据类型级覆盖
        if let Some(region) = self.type_regions.read().get(&(tenant_id.into(), data_type.as_str().into())) {
            return *region;
        }
        // 2. 租户级覆盖
        if let Some(region) = self.tenant_regions.read().get(tenant_id) {
            return *region;
        }
        // 3. 默认
        *self.default_region.read()
    }

    /// 检查数据是否允许存储在指定地域
    pub fn can_store_in(&self, tenant_id: &str, data_type: &DataType, region: ResidencyRegion) -> bool {
        let required = self.resolve_region(tenant_id, data_type);
        if required != region {
            return false;
        }
        // 强制不出境检查
        if *self.force_no_cross_border.read() && !region.allows_cross_border() {
            return true; // 不出境的地域允许
        }
        true
    }

    /// 检查是否允许跨境传输
    pub fn can_cross_border(&self, tenant_id: &str, data_type: &DataType) -> bool {
        if *self.force_no_cross_border.read() {
            return false;
        }
        let region = self.resolve_region(tenant_id, data_type);
        region.allows_cross_border() && data_type.sensitivity_level() < 3
    }

    /// 获取策略摘要
    pub fn summary(&self) -> ResidencySummary {
        ResidencySummary {
            default_region: *self.default_region.read(),
            tenant_count: self.tenant_regions.read().len(),
            type_rule_count: self.type_regions.read().len(),
            force_no_cross_border: *self.force_no_cross_border.read(),
        }
    }
}

impl Default for DataResidencyPolicy {
    fn default() -> Self {
        Self::new(ResidencyRegion::ChinaMainland)
    }
}

/// 策略摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidencySummary {
    pub default_region: ResidencyRegion,
    pub tenant_count: usize,
    pub type_rule_count: usize,
    pub force_no_cross_border: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_region_priority() {
        let policy = DataResidencyPolicy::new(ResidencyRegion::ChinaMainland);
        policy.set_tenant_region("tenant1", ResidencyRegion::HongKong);
        policy.set_type_region("tenant1", &DataType::Pii, ResidencyRegion::ChinaMainland);

        // 类型级优先
        assert_eq!(policy.resolve_region("tenant1", &DataType::Pii), ResidencyRegion::ChinaMainland);
        // 租户级
        assert_eq!(policy.resolve_region("tenant1", &DataType::Business), ResidencyRegion::HongKong);
        // 默认
        assert_eq!(policy.resolve_region("tenant2", &DataType::Business), ResidencyRegion::ChinaMainland);
    }

    #[test]
    fn test_cross_border() {
        let policy = DataResidencyPolicy::new(ResidencyRegion::ChinaMainland);
        assert!(!policy.can_cross_border("t1", &DataType::Spi)); // SPI不允许跨境
        policy.enable_force_no_cross_border();
        assert!(!policy.can_cross_border("t1", &DataType::Business)); // 强制不出境
    }
}
