// Copyright (c) 2026 璇玑 RelGraph · 统一元数据层 (Unified Metadata Layer)
// Licensed under the MIT License.

//! 元数据类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 实体种类 — 统一 KG 节点和 Cloud 资源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKind {
    /// 知识图谱节点
    GraphNode,
    /// 知识图谱边类型定义
    GraphEdgeType,
    /// 图 Schema 定义
    GraphSchema,
    /// 云盘对象/文件
    CloudObject,
    /// 云盘目录/桶
    CloudBucket,
    /// 用户资源
    UserResource,
    /// 算法定义
    Algorithm,
    /// 流水线定义
    Pipeline,
    /// 通用实体
    Generic,
}

impl EntityKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityKind::GraphNode => "graph_node",
            EntityKind::GraphEdgeType => "graph_edge_type",
            EntityKind::GraphSchema => "graph_schema",
            EntityKind::CloudObject => "cloud_object",
            EntityKind::CloudBucket => "cloud_bucket",
            EntityKind::UserResource => "user_resource",
            EntityKind::Algorithm => "algorithm",
            EntityKind::Pipeline => "pipeline",
            EntityKind::Generic => "generic",
        }
    }

    /// 是否为图相关实体
    pub fn is_graph(&self) -> bool {
        matches!(
            self,
            EntityKind::GraphNode | EntityKind::GraphEdgeType | EntityKind::GraphSchema
        )
    }

    /// 是否为云存储相关实体
    pub fn is_cloud(&self) -> bool {
        matches!(self, EntityKind::CloudObject | EntityKind::CloudBucket)
    }
}

/// 实体引用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRef {
    /// 实体 ID
    pub id: String,
    /// 实体种类
    pub kind: EntityKind,
}

impl EntityRef {
    pub fn new(id: &str, kind: EntityKind) -> Self {
        Self {
            id: id.to_string(),
            kind,
        }
    }
}

/// 资源状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceStatus {
    /// 草稿
    Draft,
    /// 活跃
    Active,
    /// 已归档
    Archived,
    /// 已删除（软删除）
    Deleted,
    /// 错误状态
    Error,
}

impl ResourceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceStatus::Draft => "draft",
            ResourceStatus::Active => "active",
            ResourceStatus::Archived => "archived",
            ResourceStatus::Deleted => "deleted",
            ResourceStatus::Error => "error",
        }
    }
}

impl Default for ResourceStatus {
    fn default() -> Self {
        ResourceStatus::Active
    }
}

/// 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// 版本号
    pub version: u64,
    /// 创建时间戳（毫秒）
    pub created_at: u64,
    /// 更新时间戳（毫秒）
    pub updated_at: u64,
    /// 创建者
    pub created_by: Option<Uuid>,
    /// 最后更新者
    pub updated_by: Option<Uuid>,
    /// 变更描述
    pub change_note: Option<String>,
}

impl VersionInfo {
    /// 创建新版本信息
    pub fn new() -> Self {
        let now = now_ms();
        Self {
            version: 1,
            created_at: now,
            updated_at: now,
            created_by: None,
            updated_by: None,
            change_note: None,
        }
    }

    /// 递增版本
    pub fn bump(&mut self) {
        self.version += 1;
        self.updated_at = now_ms();
    }
}

impl Default for VersionInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// 元数据条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataEntry {
    /// 键
    pub key: String,
    /// 值
    pub value: String,
    /// 是否为系统属性（用户不可修改）
    pub is_system: bool,
    /// 是否可索引
    pub indexable: bool,
}

impl MetadataEntry {
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
            is_system: false,
            indexable: false,
        }
    }

    pub fn system(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
            is_system: true,
            indexable: false,
        }
    }

    pub fn indexable(mut self) -> Self {
        self.indexable = true;
        self
    }
}

/// 元数据 Map
pub type MetadataMap = HashMap<String, MetadataEntry>;

/// 生成新的 UUID
pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

/// 当前时间戳（毫秒）
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 标签集合
pub type TagSet = std::collections::BTreeSet<String>;
