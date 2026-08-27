// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 远程插件注册表 — 从市场API发现/搜索/获取插件详情

use super::client::{MarketClient, MarketClientError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 远程插件版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePluginVersion {
    /// 版本号（语义化）
    pub version: String,
    /// 发布说明
    pub release_notes: String,
    /// 发布时间（ISO 8601）
    pub published_at: String,
    /// WASM下载URL
    pub download_url: String,
    /// 文件大小（字节）
    pub file_size: u64,
    /// 文件SHA-256哈希（用于验证）
    pub sha256: String,
    /// 是否为预发布版本
    #[serde(default)]
    pub pre_release: bool,
    /// 最低平台版本要求
    #[serde(default = "default_min_platform")]
    pub min_platform_version: String,
    /// 依赖列表
    #[serde(default)]
    pub dependencies: Vec<RemoteDependency>,
}

fn default_min_platform() -> String { "3.0.0".into() }

/// 远程依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteDependency {
    pub plugin_id: String,
    pub version_constraint: String,
    #[serde(default)]
    pub optional: bool,
}

/// 远程插件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePluginInfo {
    /// 插件唯一ID
    pub id: String,
    /// 插件名称
    pub name: String,
    /// 作者
    pub author: String,
    /// 描述
    pub description: String,
    /// 分类标签
    #[serde(default)]
    pub categories: Vec<String>,
    /// 关键词
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Logo URL
    #[serde(default)]
    pub logo_url: Option<String>,
    /// 主页URL
    #[serde(default)]
    pub homepage_url: Option<String>,
    /// 仓库URL
    #[serde(default)]
    pub repository_url: Option<String>,
    /// 许可证
    #[serde(default)]
    pub license: Option<String>,
    /// 平均评分（0-5）
    #[serde(default)]
    pub rating: f32,
    /// 评分数量
    #[serde(default)]
    pub rating_count: u64,
    /// 下载量
    #[serde(default)]
    pub download_count: u64,
    /// 是否官方认证
    #[serde(default)]
    pub verified: bool,
    /// 是否免费
    #[serde(default)]
    pub free: bool,
    /// 价格（免费时为0）
    #[serde(default)]
    pub price: f64,
    /// 最新版本号
    pub latest_version: String,
    /// 所有版本列表（按版本号降序）
    #[serde(default)]
    pub versions: Vec<RemotePluginVersion>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 列表查询参数
#[derive(Debug, Clone, Default)]
pub struct ListQuery {
    pub category: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub sort: Option<String>, // rating, downloads, updated, name
    pub verified_only: Option<bool>,
    pub free_only: Option<bool>,
}

impl ListQuery {
    pub fn to_query_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(c) = &self.category { map.insert("category".into(), c.clone()); }
        if let Some(k) = &self.keyword { map.insert("keyword".into(), k.clone()); }
        if let Some(p) = self.page { map.insert("page".into(), p.to_string()); }
        if let Some(ps) = self.page_size { map.insert("page_size".into(), ps.to_string()); }
        if let Some(s) = &self.sort { map.insert("sort".into(), s.clone()); }
        if let Some(v) = self.verified_only { map.insert("verified_only".into(), v.to_string()); }
        if let Some(f) = self.free_only { map.insert("free_only".into(), f.to_string()); }
        map
    }
}

/// 远程插件注册表
pub struct RemotePluginRegistry {
    client: MarketClient,
}

impl RemotePluginRegistry {
    pub fn new(client: MarketClient) -> Self {
        Self { client }
    }

    /// 列出插件（支持分页/筛选/排序）
    pub async fn list(&self, query: Option<ListQuery>) -> Result<Vec<RemotePluginInfo>, MarketClientError> {
        let query_map = query.map(|q| q.to_query_map());
        let query_ref = query_map.as_ref();
        let body = self.client.get("/plugins", query_ref).await?;

        // 兼容两种响应格式：直接数组 或 {data: [...]}
        let plugins: Vec<RemotePluginInfo> = if body.is_array() {
            serde_json::from_value(body).map_err(|e| MarketClientError::ParseError(e.to_string()))?
        } else {
            body.get("data")
                .and_then(|d| serde_json::from_value(d.clone()).ok())
                .unwrap_or_default()
        };
        Ok(plugins)
    }

    /// 搜索插件
    pub async fn search(&self, keyword: &str) -> Result<Vec<RemotePluginInfo>, MarketClientError> {
        let query = ListQuery { keyword: Some(keyword.into()), ..Default::default() };
        self.list(Some(query)).await
    }

    /// 按分类列出
    pub async fn list_by_category(&self, category: &str) -> Result<Vec<RemotePluginInfo>, MarketClientError> {
        let query = ListQuery { category: Some(category.into()), ..Default::default() };
        self.list(Some(query)).await
    }

    /// 获取插件详情（含所有版本）
    pub async fn get_detail(&self, plugin_id: &str) -> Result<RemotePluginInfo, MarketClientError> {
        let body = self.client.get(&format!("/plugins/{}", plugin_id), None).await?;
        let info: RemotePluginInfo = serde_json::from_value(body)
            .map_err(|e| MarketClientError::ParseError(e.to_string()))?;
        Ok(info)
    }

    /// 获取指定版本详情
    pub async fn get_version(&self, plugin_id: &str, version: &str) -> Result<RemotePluginVersion, MarketClientError> {
        let body = self.client.get(&format!("/plugins/{}/versions/{}", plugin_id, version), None).await?;
        let ver: RemotePluginVersion = serde_json::from_value(body)
            .map_err(|e| MarketClientError::ParseError(e.to_string()))?;
        Ok(ver)
    }

    /// 获取最新版本
    pub async fn get_latest_version(&self, plugin_id: &str, include_pre_release: bool) -> Result<RemotePluginVersion, MarketClientError> {
        let detail = self.get_detail(plugin_id).await?;
        let versions = if include_pre_release {
            detail.versions
        } else {
            detail.versions.into_iter().filter(|v| !v.pre_release).collect()
        };
        versions.into_iter().next()
            .ok_or_else(|| MarketClientError::NotFound(format!("no version found for {}", plugin_id)))
    }

    /// 获取分类列表
    pub async fn list_categories(&self) -> Result<Vec<String>, MarketClientError> {
        let body = self.client.get("/categories", None).await?;
        let categories: Vec<String> = if body.is_array() {
            serde_json::from_value(body).unwrap_or_default()
        } else {
            body.get("data").and_then(|d| serde_json::from_value(d.clone()).ok()).unwrap_or_default()
        };
        Ok(categories)
    }

    /// 检查插件是否存在
    pub async fn exists(&self, plugin_id: &str) -> bool {
        self.get_detail(plugin_id).await.is_ok()
    }

    /// 获取客户端引用（供Installer使用）
    pub fn client(&self) -> &MarketClient {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_query_to_map() {
        let query = ListQuery {
            category: Some("ai".into()),
            keyword: Some("ocr".into()),
            page: Some(1),
            page_size: Some(20),
            sort: Some("rating".into()),
            verified_only: Some(true),
            free_only: Some(false),
        };
        let map = query.to_query_map();
        assert_eq!(map.get("category"), Some(&"ai".to_string()));
        assert_eq!(map.get("keyword"), Some(&"ocr".to_string()));
        assert_eq!(map.get("page"), Some(&"1".to_string()));
        assert_eq!(map.get("verified_only"), Some(&"true".to_string()));
    }
}
