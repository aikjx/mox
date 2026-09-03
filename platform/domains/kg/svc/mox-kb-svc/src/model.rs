// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 云盘知识库领域值类型（SSOT）
//!
//! 文档 / 版本 / 实体 / 关系 / 检索命中 等核心领域概念的单一真相源，
//! 供 document / version / analyze / link / search / handlers 共享，避免类型漂移。

use serde::{Deserialize, Serialize};

/// 知识库文档状态
pub const STATUS_DRAFT: &str = "draft";
pub const STATUS_ANALYZED: &str = "analyzed";
pub const STATUS_LINKED: &str = "linked";

/// 实体类型（分析器产出）
pub const ET_PERSON: &str = "person";
pub const ET_ORG: &str = "org";
pub const ET_TECH: &str = "tech";
pub const ET_CONCEPT: &str = "concept";

/// 知识库实体（分析器从正文抽取）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KbEntity {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub frequency: u32,
    pub snippet: String,
}

/// 知识库关系（实体间语义关联，挂图边）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KbRelation {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation: String,
    pub weight: f64,
}

/// 文档版本快照（零拷贝恢复：仅存增量 note + 全量内容）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KbVersion {
    pub version: String,
    pub title: String,
    pub content: String,
    pub note: String,
    pub created_at: String,
}

/// 知识库文档（完整领域对象）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KbDocument {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
    pub status: String,
    pub summary: String,
    pub entities: Vec<KbEntity>,
    pub relations: Vec<KbRelation>,
    pub current_version: String,
    pub versions: Vec<KbVersion>,
    pub created_at: String,
    pub updated_at: String,
}

impl KbDocument {
    /// 新建空文档（draft 态，v1）
    pub fn new(id: String, title: String, content: String, category: String) -> Self {
        let now = now_iso();
        Self {
            id,
            title,
            content,
            category,
            tags: Vec::new(),
            status: STATUS_DRAFT.into(),
            summary: String::new(),
            entities: Vec::new(),
            relations: Vec::new(),
            current_version: "v1".into(),
            versions: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// 默认分类（前端 /kb/categories 语义对齐）
    pub fn default_category() -> &'static str {
        "cat-tech"
    }
}

/// 检索命中（search.rs 产出）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub category: String,
    pub snippet: String,
    pub score: f64,
    pub tags: Vec<String>,
}

/// 检索请求
#[derive(Debug, Clone, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub category: Option<String>,
}

fn default_limit() -> usize {
    20
}

/// 当前 UTC ISO 时间戳
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// 生成 kb 前缀短 id（与 legacy `new_id("kb")` 同构）
pub fn new_kb_id() -> String {
    format!("kb-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document_is_draft_v1() {
        let d = KbDocument::new("kb-1".into(), "标题".into(), "正文".into(), "cat-tech".into());
        assert_eq!(d.status, STATUS_DRAFT);
        assert_eq!(d.current_version, "v1");
        assert!(d.versions.is_empty());
        assert!(!d.created_at.is_empty());
    }

    #[test]
    fn new_kb_id_has_prefix() {
        let id = new_kb_id();
        assert!(id.starts_with("kb-"));
    }
}
