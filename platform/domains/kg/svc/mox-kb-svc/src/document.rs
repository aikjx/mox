// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 知识库文档服务：CRUD + 分类/标签索引（基于 store-core 内容寻址去重存储）
//!
//! 存储布局（物理落盘，原子写 + 引用计数 GC 由 store-core 保障）：
//! - 文档对象：`kb/docs/{id}.json`（完整 KbDocument JSON）
//! - 索引 KV：`kb:index` = 文档摘要数组（兼容写入；读请求按当前主源扫描）
//! - 标签 KV：`kb:tags` = 全局标签聚合

use crate::model::{new_kb_id, now_iso, KbDocument};
use bytes::Bytes;
use mox_base_store_core::StoreError;
use mox_cloud_sdk::StoreBackend;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};

/// 文档对象 key 前缀
const DOC_KEY_PREFIX: &str = "kb/docs/";
/// 文档摘要索引 key
const INDEX_KEY: &str = "kb:index";
/// 全局标签索引 key
const TAGS_KEY: &str = "kb:tags";
/// 默认分类清单（与 legacy /kb/categories 语义对齐）
pub const CATEGORIES: &[(&str, &str)] = &[
    ("cat-tech", "技术文档"),
    ("cat-dialogue", "对话沉淀"),
    ("cat-business", "业务文档"),
    ("cat-research", "研究文档"),
];

/// 知识库文档服务
#[derive(Clone)]
pub struct KbDocumentService {
    pub(crate) backend: Arc<StoreBackend>,
    access: Option<crate::access::KnowledgeAccess>,
    pub(crate) mutation: Arc<tokio::sync::Mutex<()>>,
}

/// 文档摘要（索引条目，用于 list/stats）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocSummary {
    pub id: String,
    #[serde(default)]
    pub access: Option<crate::access::KnowledgeAccess>,
    #[serde(default)]
    pub readers: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub storage_bytes: u64,
    pub title: String,
    pub category: String,
    pub status: String,
    pub updated_at: String,
}

/// Optimistic write outcome, distinct from missing/unauthorized resources.
#[derive(Debug)]
pub enum DocumentWriteError {
    Conflict,
    Storage(StoreError),
}
impl From<StoreError> for DocumentWriteError {
    fn from(error: StoreError) -> Self {
        Self::Storage(error)
    }
}
impl std::fmt::Display for DocumentWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict => f.write_str("knowledge write conflict"),
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl KbDocumentService {
    /// 包装已装配的存储后端
    pub fn new(backend: Arc<StoreBackend>) -> Self {
        Self { backend, access: None, mutation: Arc::new(tokio::sync::Mutex::new(())) }
    }

    pub fn scoped(&self, access: crate::access::KnowledgeAccess) -> Self {
        Self { access: Some(access), ..self.clone() }
    }
    pub(crate) fn unscoped(&self) -> Self {
        Self { access: None, ..self.clone() }
    }
    pub(crate) fn request_access(&self) -> Option<&crate::access::KnowledgeAccess> {
        self.access.as_ref()
    }
    pub fn is_scoped(&self) -> bool {
        self.access.is_some()
    }
    fn denied(id: &str) -> StoreError {
        StoreError::NotFound { path: id.to_owned() }
    }
    fn check(&self, doc: &KbDocument, write: bool) -> crate::Result<()> {
        if let Some(access) = &self.access {
            if (write && (!access.permits(&doc.access) || access.readonly))
                || (!write && !access.can_read(&doc.access, &doc.readers))
            {
                return Err(Self::denied(&doc.id));
            }
        }
        Ok(())
    }
    fn doc_key(id: &str) -> String {
        format!("{DOC_KEY_PREFIX}{id}.json")
    }

    /// 创建文档（自动分配 id）
    pub async fn create(
        &self,
        title: &str,
        content: &str,
        category: Option<&str>,
    ) -> crate::Result<KbDocument> {
        let cat = category
            .map(str::to_string)
            .unwrap_or_else(|| KbDocument::default_category().to_string());
        let mut doc = KbDocument::new(new_kb_id(), title.to_string(), content.to_string(), cat);
        doc.access = self.access.clone().map(|mut access| {
            access.administrator = false;
            access.readonly = false;
            access
        });
        self.save(&doc).await?;
        Ok(doc)
    }

    /// 保存文档（原子写对象 + 刷新索引）
    pub async fn save(&self, doc: &KbDocument) -> crate::Result<()> {
        let _mutation = self.mutation.lock().await;
        self.check(doc, true)?;
        if self.backend.object.exists(&Self::doc_key(&doc.id)).await? {
            let existing = self.get(&doc.id).await?;
            self.check(&existing, true)?;
            if existing.access != doc.access
                || existing.readers != doc.readers
                || existing.acl_revision != doc.acl_revision
            {
                return Err(Self::denied(&doc.id));
            }
        }
        self.write_document(doc).await
    }

    async fn write_document(&self, doc: &KbDocument) -> crate::Result<()> {
        let blob = Bytes::from(serde_json::to_vec(doc).map_err(crate::err_other)?);
        self.backend
            .object
            .put(&Self::doc_key(&doc.id), "application/json", blob)
            .await?;
        self.rebuild_index().await?;
        Ok(())
    }

    /// 读取文档
    pub async fn get(&self, id: &str) -> crate::Result<KbDocument> {
        let raw = self.backend.object.get(&Self::doc_key(id)).await?;
        let doc = serde_json::from_slice(&raw)
            .map_err(|e| StoreError::Other(format!("文档 JSON 损坏: {e}")))?;
        self.check(&doc, false)?;
        Ok(doc)
    }

    pub async fn get_for_write(&self, id: &str) -> crate::Result<KbDocument> {
        let doc = self.get(id).await?;
        self.check(&doc, true)?;
        Ok(doc)
    }

    /// ACL updates are separate from content writes and serialized with them.
    pub async fn change_reader(
        &self,
        id: &str,
        user_id: &str,
        grant: bool,
        expected_revision: u64,
    ) -> std::result::Result<KbDocument, crate::access::ShareError> {
        use crate::access::ShareError;
        let _mutation = self.mutation.lock().await;
        let mut doc = self.get_for_write(id).await?;
        if doc.access.is_none()
            || user_id.trim().is_empty()
            || user_id != user_id.trim()
            || user_id.len() > 128
            || user_id.chars().any(char::is_control)
        {
            return Err(ShareError::InvalidRequest);
        }
        if doc.acl_revision != expected_revision {
            return Err(ShareError::Conflict);
        }
        let present = doc.readers.iter().any(|id| id == user_id);
        if present == grant {
            return Ok(doc);
        }
        if grant {
            if doc.readers.len() >= 256 {
                return Err(ShareError::InvalidRequest);
            }
            doc.readers.push(user_id.to_owned());
            doc.readers.sort();
        } else {
            doc.readers.retain(|id| id != user_id);
        }
        doc.acl_revision = doc.acl_revision.checked_add(1).ok_or(ShareError::Conflict)?;
        doc.updated_at = now_iso();
        self.write_document(&doc).await?;
        Ok(doc)
    }

    /// Reject work based on any changed source snapshot, including ACL changes.
    pub async fn save_if_unchanged(
        &self,
        expected: &KbDocument,
        updated: &KbDocument,
    ) -> std::result::Result<(), DocumentWriteError> {
        let _mutation = self.mutation.lock().await;
        let current = self.get_for_write(&expected.id).await?;
        if current != *expected
            || updated.id != expected.id
            || updated.access != current.access
            || updated.readers != current.readers
            || updated.acl_revision != current.acl_revision
        {
            return Err(DocumentWriteError::Conflict);
        }
        self.check(updated, true)?;
        self.write_document(updated).await?;
        Ok(())
    }

    /// 更新文档字段（title/content/category/tags 增量合并），保留实体/版本
    pub async fn update(&self, id: &str, patch: &Value) -> crate::Result<KbDocument> {
        self.update_checked(id, patch).await.map_err(|error| match error {
            DocumentWriteError::Storage(error) => error,
            DocumentWriteError::Conflict => crate::err_other("knowledge write conflict"),
        })
    }

    pub async fn update_checked(
        &self,
        id: &str,
        patch: &Value,
    ) -> std::result::Result<KbDocument, DocumentWriteError> {
        let _mutation = self.mutation.lock().await;
        let mut doc = self.get_for_write(id).await?;
        if let Some(expected) = patch.get("expected_current_version") {
            if expected.as_str() != Some(doc.current_version.as_str()) {
                return Err(DocumentWriteError::Conflict);
            }
        }
        let changed = patch
            .get("title")
            .and_then(Value::as_str)
            .is_some_and(|value| value != doc.title)
            || patch
                .get("content")
                .and_then(Value::as_str)
                .is_some_and(|value| value != doc.content);
        let note = patch.get("version_note").and_then(Value::as_str).unwrap_or("").trim();
        if changed || !note.is_empty() {
            crate::version::KbVersionService::create(
                &mut doc,
                if note.is_empty() { "更新文档内容" } else { note },
            );
            doc.entities.clear();
            doc.relations.clear();
            doc.summary.clear();
        }
        doc.status = crate::model::STATUS_DRAFT.into();
        if let Some(v) = patch.get("title").and_then(Value::as_str) {
            doc.title = v.to_string();
        }
        if let Some(v) = patch.get("content").and_then(Value::as_str) {
            doc.content = v.to_string();
        }
        if let Some(v) = patch.get("category").and_then(Value::as_str) {
            doc.category = v.to_string();
        }
        if let Some(v) = patch.get("tags").and_then(Value::as_array) {
            doc.tags = v.iter().filter_map(Value::as_str).map(str::to_string).collect();
        }
        doc.updated_at = now_iso();
        self.write_document(&doc).await?;
        Ok(doc)
    }

    /// 删除文档
    pub async fn delete(&self, id: &str) -> crate::Result<bool> {
        let _mutation = self.mutation.lock().await;
        let existed = self.backend.object.exists(&Self::doc_key(id)).await?;
        if existed {
            self.check(&self.get(id).await?, true)?;
            self.backend.object.delete(&Self::doc_key(id)).await?;
            self.rebuild_index().await?;
        }
        Ok(existed)
    }

    /// 文档列表（来自索引，按 updated_at 倒序）
    pub async fn list(&self) -> crate::Result<Vec<Value>> {
        let index = self.read_index().await?;
        let mut items: Vec<Value> = index
            .into_iter()
            .map(|s| {
                json!({
                    "id": s.id, "title": s.title, "category": s.category,
                    "status": s.status, "updated_at": s.updated_at,
                })
            })
            .collect();
        items.sort_by(|a, b| {
            b["updated_at"]
                .as_str()
                .unwrap_or("")
                .cmp(a["updated_at"].as_str().unwrap_or(""))
        });
        Ok(items)
    }

    /// 分类统计（categories 端点 + stats 复用）
    pub async fn categories(&self) -> crate::Result<Vec<Value>> {
        let index = self.read_index().await?;
        let mut counts = HashMap::<String, usize>::new();
        for s in &index {
            *counts.entry(s.category.clone()).or_default() += 1;
        }
        Ok(CATEGORIES
            .iter()
            .map(|(id, name)| {
                json!({ "id": id, "name": name, "count": counts.get(*id).copied().unwrap_or(0) })
            })
            .collect())
    }

    /// 全局标签聚合（tags 端点）
    pub async fn tags(&self) -> crate::Result<Vec<Value>> {
        let mut tags = HashMap::<String, usize>::new();
        for summary in self.read_index().await? {
            for tag in summary.tags {
                *tags.entry(tag).or_default() += 1;
            }
        }
        let mut out: Vec<Value> = tags
            .into_iter()
            .map(|(tag, count)| json!({ "name": tag, "count": count }))
            .collect();
        out.sort_by(|a, b| {
            b["count"].as_u64().unwrap_or(0).cmp(&a["count"].as_u64().unwrap_or(0))
                .then_with(|| a["name"].as_str().cmp(&b["name"].as_str()))
        });
        Ok(out)
    }

    /// 统计（documents 数 / categories 数 / tags 数 / storage 字节）
    pub async fn stats(&self) -> crate::Result<Value> {
        let index = self.read_index().await?;
        let mut tags = std::collections::HashSet::new();
        let mut storage_bytes = 0u64;
        for summary in &index {
            tags.extend(summary.tags.iter());
            storage_bytes = storage_bytes
                .checked_add(summary.storage_bytes)
                .ok_or_else(|| crate::err_other("knowledge byte count overflow"))?;
        }
        Ok(json!({
            "documents": index.len(),
            "categories": CATEGORIES.len(),
            "tags": tags.len(),
            "storage_bytes": storage_bytes,
        }))
    }

    /// One source scan, including ACL and logical JSON byte size; never cached permissions.
    async fn scan_index(&self) -> crate::Result<Vec<DocSummary>> {
        let mut summaries = Vec::new();
        let keys = self.backend.object.list_keys(DOC_KEY_PREFIX).await?;
        for key in keys.into_iter().filter(|key| key.ends_with(".json")) {
            let raw = self.backend.object.get(&key).await?;
            let doc: KbDocument = serde_json::from_slice(&raw).map_err(crate::err_other)?;
            summaries.push(DocSummary {
                id: doc.id,
                access: doc.access,
                readers: doc.readers,
                tags: doc.tags,
                storage_bytes: u64::try_from(raw.len()).map_err(crate::err_other)?,
                title: doc.title,
                category: doc.category,
                status: doc.status,
                updated_at: doc.updated_at,
            });
        }
        Ok(summaries)
    }

    /// 写入兼容派生索引；当前查询不依赖或写入该 KV。
    async fn rebuild_index(&self) -> crate::Result<()> {
        let summaries = self.scan_index().await?;
        let mut tags = HashMap::<String, usize>::new();
        for summary in &summaries {
            for tag in &summary.tags {
                *tags.entry(tag.clone()).or_default() += 1;
            }
        }
        let index_blob = Bytes::from(serde_json::to_vec(&summaries).map_err(crate::err_other)?);
        self.backend.kv.put(INDEX_KEY, index_blob).await?;
        let tags_blob = Bytes::from(serde_json::to_vec(&tags).map_err(crate::err_other)?);
        self.backend.kv.put(TAGS_KEY, tags_blob).await?;
        Ok(())
    }

    /// 读取摘要索引
    pub(crate) async fn read_index(&self) -> crate::Result<Vec<DocSummary>> {
        let _mutation = self.mutation.lock().await;
        // ponytail: O(n) source scan until a transactional ACL-aware index is available.
        // Read requests do not write KV indexes and do not cache authorization decisions.
        let mut index = self.scan_index().await?;
        if let Some(access) = &self.access {
            index.retain(|d| access.can_read(&d.access, &d.readers));
        }
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::STATUS_DRAFT;
    use crate::tests::fs_backend;

    #[tokio::test]
    async fn document_crud_roundtrip() {
        let backend = fs_backend();
        let svc = KbDocumentService::new(backend);
        let doc = svc
            .create("云盘架构", "内容寻址去重 + S3 回源", Some("cat-tech"))
            .await
            .unwrap();
        assert_eq!(doc.status, STATUS_DRAFT);
        let got = svc.get(&doc.id).await.unwrap();
        assert_eq!(got.title, "云盘架构");
        let listed = svc.list().await.unwrap();
        assert_eq!(listed.len(), 1);
        let updated = svc.update(&doc.id, &json!({ "title": "云盘架构 v2" })).await.unwrap();
        assert_eq!(updated.title, "云盘架构 v2");
        assert!(svc.delete(&doc.id).await.unwrap());
        assert_eq!(svc.list().await.unwrap().len(), 0);
        let err = svc.get(&doc.id).await.unwrap_err();
        assert!(err.to_string().contains("不存在"), "{err}");
    }

    #[tokio::test]
    async fn categories_and_stats() {
        let backend = fs_backend();
        let svc = KbDocumentService::new(backend);
        svc.create("A", "内容", Some("cat-tech")).await.unwrap();
        svc.create("B", "内容", Some("cat-business")).await.unwrap();
        let cats = svc.categories().await.unwrap();
        assert_eq!(cats.len(), 4); // cat-tech / cat-business / cat-research / cat-dialogue(对话沉淀)
        let tech = cats.iter().find(|c| c["id"] == "cat-tech").unwrap();
        assert_eq!(tech["count"], 1);
        let stats = svc.stats().await.unwrap();
        assert_eq!(stats["documents"], 2);
        assert!(stats["storage_bytes"].as_u64().unwrap() > 0);
    }
}
