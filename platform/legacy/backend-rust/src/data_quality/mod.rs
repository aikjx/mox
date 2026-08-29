// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! R · 数据质量与血缘
//!
//! 核心能力：
//! - 数据目录（Data Catalog）：元数据管理、搜索、标签、分类
//! - 数据血缘（Data Lineage）：上下游追踪、影响分析、DAG可视化
//! - 质量规则引擎：完整性、准确性、一致性、时效性、唯一性、有效性
//! - 质量监控：规则执行、告警、报告、趋势分析

pub mod lineage;
pub mod rules;
pub mod monitor;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use lineage::{DataLineage, LineageNode, LineageEdge, LineageImpact};
pub use rules::{QualityRuleEngine, QualityRule, QualityDimension, RuleResult};
pub use monitor::{QualityMonitor, QualityReport, QualityAlert};

/// 数据资产类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssetType {
    Table,
    Column,
    Dataset,
    File,
    Stream,
    Api,
    Dashboard,
    Model,
}

/// 数据资产
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataAsset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub asset_type: AssetType,
    pub owner: String,
    pub domain: String,
    pub tags: Vec<String>,
    pub properties: std::collections::HashMap<String, String>,
    pub schema: Option<Vec<ColumnSchema>>,
    pub created_at: String,
    pub updated_at: String,
    pub last_queried_at: Option<String>,
    pub query_count: u64,
    pub quality_score: Option<f64>,
    pub sensitivity: SensitivityLevel,
}

/// 列 Schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub description: String,
    pub tags: Vec<String>,
}

/// 敏感级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SensitivityLevel {
    Public,
    Internal,
    Confidential,
    Restricted,
    PII,
    PHI,
}

/// 数据目录
pub struct DataCatalog {
    assets: DashMap<String, DataAsset>,
    name_index: DashMap<String, Vec<String>>,
    tag_index: DashMap<String, Vec<String>>,
    domain_index: DashMap<String, Vec<String>>,
    owner_index: DashMap<String, Vec<String>>,
    total_assets: std::sync::atomic::AtomicU64,
}

impl DataCatalog {
    /// 创建数据目录
    pub fn new() -> Self {
        Self {
            assets: DashMap::new(),
            name_index: DashMap::new(),
            tag_index: DashMap::new(),
            domain_index: DashMap::new(),
            owner_index: DashMap::new(),
            total_assets: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 注册数据资产
    pub fn register(&self, mut asset: DataAsset) -> String {
        if asset.id.is_empty() {
            asset.id = Uuid::new_v4().to_string();
        }
        let now = chrono::Utc::now().to_rfc3339();
        if asset.created_at.is_empty() {
            asset.created_at = now.clone();
        }
        asset.updated_at = now;

        let id = asset.id.clone();
        let name = asset.name.clone();
        let tags = asset.tags.clone();
        let domain = asset.domain.clone();
        let owner = asset.owner.clone();

        self.assets.insert(id.clone(), asset);
        self.total_assets.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 建立索引
        self.name_index.entry(name).or_default().push(id.clone());
        for tag in tags {
            self.tag_index.entry(tag).or_default().push(id.clone());
        }
        self.domain_index.entry(domain).or_default().push(id.clone());
        self.owner_index.entry(owner).or_default().push(id.clone());

        id
    }

    /// 获取数据资产
    pub fn get(&self, id: &str) -> Option<DataAsset> {
        self.assets.get(id).map(|a| a.clone())
    }

    /// 按名称查找
    pub fn find_by_name(&self, name: &str) -> Vec<DataAsset> {
        self.name_index.get(name)
            .map(|ids| ids.iter().filter_map(|id| self.get(id)).collect())
            .unwrap_or_default()
    }

    /// 按标签查找
    pub fn find_by_tag(&self, tag: &str) -> Vec<DataAsset> {
        self.tag_index.get(tag)
            .map(|ids| ids.iter().filter_map(|id| self.get(id)).collect())
            .unwrap_or_default()
    }

    /// 按域查找
    pub fn find_by_domain(&self, domain: &str) -> Vec<DataAsset> {
        self.domain_index.get(domain)
            .map(|ids| ids.iter().filter_map(|id| self.get(id)).collect())
            .unwrap_or_default()
    }

    /// 按所有者查找
    pub fn find_by_owner(&self, owner: &str) -> Vec<DataAsset> {
        self.owner_index.get(owner)
            .map(|ids| ids.iter().filter_map(|id| self.get(id)).collect())
            .unwrap_or_default()
    }

    /// 全文搜索
    pub fn search(&self, query: &str) -> Vec<DataAsset> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<(DataAsset, f64)> = Vec::new();

        for asset in self.assets.iter() {
            let mut score = 0.0;
            if asset.name.to_lowercase().contains(&query_lower) {
                score += 10.0;
            }
            if asset.description.to_lowercase().contains(&query_lower) {
                score += 5.0;
            }
            for tag in &asset.tags {
                if tag.to_lowercase().contains(&query_lower) {
                    score += 3.0;
                }
            }
            if asset.domain.to_lowercase().contains(&query_lower) {
                score += 2.0;
            }
            if score > 0.0 {
                results.push((asset.clone(), score));
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().map(|(a, _)| a).collect()
    }

    /// 更新数据资产
    pub fn update(&self, id: &str, updates: AssetUpdates) -> Option<DataAsset> {
        let mut asset = self.assets.get_mut(id)?;
        if let Some(name) = updates.name {
            asset.name = name;
        }
        if let Some(description) = updates.description {
            asset.description = description;
        }
        if let Some(owner) = updates.owner {
            asset.owner = owner;
        }
        if let Some(tags) = updates.tags {
            asset.tags = tags;
        }
        if let Some(quality_score) = updates.quality_score {
            asset.quality_score = Some(quality_score);
        }
        if let Some(sensitivity) = updates.sensitivity {
            asset.sensitivity = sensitivity;
        }
        asset.updated_at = chrono::Utc::now().to_rfc3339();
        Some(asset.clone())
    }

    /// 记录查询
    pub fn record_query(&self, id: &str) {
        if let Some(mut asset) = self.assets.get_mut(id) {
            asset.query_count += 1;
            asset.last_queried_at = Some(chrono::Utc::now().to_rfc3339());
        }
    }

    /// 删除数据资产
    pub fn remove(&self, id: &str) -> bool {
        if let Some((_, asset)) = self.assets.remove(id) {
            // 从索引中移除
            if let Some(mut ids) = self.name_index.get_mut(&asset.name) {
                ids.retain(|i| i != id);
            }
            for tag in &asset.tags {
                if let Some(mut ids) = self.tag_index.get_mut(tag) {
                    ids.retain(|i| i != id);
                }
            }
            true
        } else {
            false
        }
    }

    /// 获取所有资产
    pub fn list_all(&self) -> Vec<DataAsset> {
        self.assets.iter().map(|a| a.clone()).collect()
    }

    /// 获取统计
    pub fn stats(&self) -> CatalogStats {
        let all = self.list_all();
        CatalogStats {
            total_assets: all.len(),
            by_type: all.iter().fold(std::collections::HashMap::new(), |mut acc, a| {
                *acc.entry(format!("{:?}", a.asset_type)).or_insert(0) += 1;
                acc
            }),
            by_domain: all.iter().fold(std::collections::HashMap::new(), |mut acc, a| {
                *acc.entry(a.domain.clone()).or_insert(0) += 1;
                acc
            }),
            by_sensitivity: all.iter().fold(std::collections::HashMap::new(), |mut acc, a| {
                *acc.entry(format!("{:?}", a.sensitivity)).or_insert(0) += 1;
                acc
            }),
            total_tags: self.tag_index.len(),
            total_domains: self.domain_index.len(),
            total_owners: self.owner_index.len(),
            avg_quality_score: all.iter()
                .filter_map(|a| a.quality_score)
                .sum::<f64>() / all.iter().filter(|a| a.quality_score.is_some()).count().max(1) as f64,
        }
    }
}

impl Default for DataCatalog {
    fn default() -> Self {
        Self::new()
    }
}

/// 资产更新
#[derive(Debug, Clone, Default)]
pub struct AssetUpdates {
    pub name: Option<String>,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub tags: Option<Vec<String>>,
    pub quality_score: Option<f64>,
    pub sensitivity: Option<SensitivityLevel>,
}

/// 目录统计
#[derive(Debug, Clone, Serialize)]
pub struct CatalogStats {
    pub total_assets: usize,
    pub by_type: std::collections::HashMap<String, usize>,
    pub by_domain: std::collections::HashMap<String, usize>,
    pub by_sensitivity: std::collections::HashMap<String, usize>,
    pub total_tags: usize,
    pub total_domains: usize,
    pub total_owners: usize,
    pub avg_quality_score: f64,
}
