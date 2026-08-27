//! 数据主权Trait — Data Residency Controller
//!
//! 企业级数据主权控制抽象，可替换地域策略：
//! 控制数据存储位置、跨境传输、数据本地化要求。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 数据主权地域
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ResidencyRegion {
    /// 中国大陆
    ChinaMainland,
    /// 中国香港
    HongKong,
    /// 美国
    UnitedStates,
    /// 欧盟
    EuropeanUnion,
    /// 英国
    UnitedKingdom,
    /// 日本
    Japan,
    /// 新加坡
    Singapore,
    /// 澳大利亚
    Australia,
    /// 加拿大
    Canada,
    /// 其他
    Other,
}

impl ResidencyRegion {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResidencyRegion::ChinaMainland => "china_mainland",
            ResidencyRegion::HongKong => "hong_kong",
            ResidencyRegion::UnitedStates => "united_states",
            ResidencyRegion::EuropeanUnion => "european_union",
            ResidencyRegion::UnitedKingdom => "united_kingdom",
            ResidencyRegion::Japan => "japan",
            ResidencyRegion::Singapore => "singapore",
            ResidencyRegion::Australia => "australia",
            ResidencyRegion::Canada => "canada",
            ResidencyRegion::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "china_mainland" | "cn" | "china" => ResidencyRegion::ChinaMainland,
            "hong_kong" | "hk" => ResidencyRegion::HongKong,
            "united_states" | "us" | "usa" => ResidencyRegion::UnitedStates,
            "european_union" | "eu" => ResidencyRegion::EuropeanUnion,
            "united_kingdom" | "uk" => ResidencyRegion::UnitedKingdom,
            "japan" | "jp" => ResidencyRegion::Japan,
            "singapore" | "sg" => ResidencyRegion::Singapore,
            "australia" | "au" => ResidencyRegion::Australia,
            "canada" | "ca" => ResidencyRegion::Canada,
            _ => ResidencyRegion::Other,
        }
    }
}

/// 数据分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataClassification {
    /// 公开数据
    Public,
    /// 内部数据
    Internal,
    /// 机密数据
    Confidential,
    /// 严格机密（个人敏感信息）
    StrictlyConfidential,
}

impl DataClassification {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataClassification::Public => "public",
            DataClassification::Internal => "internal",
            DataClassification::Confidential => "confidential",
            DataClassification::StrictlyConfidential => "strictly_confidential",
        }
    }
}

/// 数据主权策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataResidencyPolicy {
    /// 策略ID
    pub policy_id: String,
    /// 策略名称
    pub name: String,
    /// 适用的数据分类
    pub data_classification: DataClassification,
    /// 允许存储的地域列表
    pub allowed_regions: Vec<ResidencyRegion>,
    /// 禁止存储的地域列表
    #[serde(default)]
    pub forbidden_regions: Vec<ResidencyRegion>,
    /// 是否要求数据本地化（必须存储在数据产生地）
    #[serde(default)]
    pub data_localization_required: bool,
    /// 跨境传输是否需要审批
    #[serde(default)]
    pub cross_border_approval_required: bool,
    /// 数据保留期限（天，0表示永久）
    #[serde(default)]
    pub retention_days: u32,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool { true }

impl Default for DataResidencyPolicy {
    fn default() -> Self {
        Self {
            policy_id: "default".into(),
            name: "Default Policy".into(),
            data_classification: DataClassification::Internal,
            allowed_regions: vec![ResidencyRegion::ChinaMainland],
            forbidden_regions: Vec::new(),
            data_localization_required: false,
            cross_border_approval_required: true,
            retention_days: 0,
            enabled: true,
        }
    }
}

/// 数据主权检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidencyResult {
    /// 是否允许
    pub allowed: bool,
    /// 原因（不允许时）
    #[serde(default)]
    pub reason: Option<String>,
    /// 建议的存储地域
    #[serde(default)]
    pub suggested_region: Option<ResidencyRegion>,
    /// 是否需要跨境审批
    #[serde(default)]
    pub cross_border_approval_required: bool,
    /// 适用的策略ID
    pub policy_id: String,
}

impl ResidencyResult {
    pub fn allow(policy_id: impl Into<String>) -> Self {
        Self {
            allowed: true,
            reason: None,
            suggested_region: None,
            cross_border_approval_required: false,
            policy_id: policy_id.into(),
        }
    }

    pub fn deny(policy_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.into()),
            suggested_region: None,
            cross_border_approval_required: false,
            policy_id: policy_id.into(),
        }
    }
}

/// 数据主权控制器Trait
#[async_trait]
pub trait DataResidencyController: Send + Sync {
    /// 检查数据是否可以存储在指定地域
    async fn check_storage(&self, data_classification: DataClassification, region: ResidencyRegion, tenant_id: Option<&str>) -> ResidencyResult;

    /// 检查数据是否可以从源地域传输到目标地域
    async fn check_cross_border_transfer(&self, data_classification: DataClassification, source: ResidencyRegion, target: ResidencyRegion, tenant_id: Option<&str>) -> ResidencyResult;

    /// 获取数据分类允许的存储地域
    async fn get_allowed_regions(&self, data_classification: DataClassification, tenant_id: Option<&str>) -> HashSet<ResidencyRegion>;

    /// 获取适用的策略
    async fn get_applicable_policy(&self, data_classification: DataClassification, tenant_id: Option<&str>) -> Option<DataResidencyPolicy>;

    /// 注册策略
    async fn register_policy(&self, policy: DataResidencyPolicy) -> anyhow::Result<()>;

    /// 移除策略
    async fn remove_policy(&self, policy_id: &str) -> anyhow::Result<()>;
}
