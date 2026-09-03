// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 知识库文档服务：CRUD + 分类/标签索引（基于 store-core 内容寻址去重存储）
//!
//! 存储布局（物理落盘，原子写 + 引用计数 GC 由 store-core 保障）：
//! - 文档对象：`kb/docs/{id}.json`（完整 KbDocument JSON）
//! - 索引 KV：`kb:index` = 文档摘要数组（list/stats 免扫全量对象）
//! - 标签 KV：`kb:tags` = 全局标签聚合

use crate::model::{KbDocument, now_iso, new_kb_id};
use bytes::Bytes;
use mox_base_store_core::StoreError;
use mox_cloud_store_core::{StoreBackend, list_object_refs};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// 文档对象 key 前缀
const DOC_KEY_PREFIX: &str = "kb/docs/";
/// 文档摘要索引 key
const INDEX_KEY: &str = "kb:index";
/// 全局标签索引 key
const TAGS_KEY: &str = "kb:tags";
/// 默认分类清单（与 legacy /kb/categories 语义对齐）
pub const CATEGORIES: &[(&str, &str)] = &[
    ("cat-tech", "技术文档"),
    ("cat-business", "业务文档"),
    ("cat-research", "研究文档"),
];

/// 知识库文档服务
#[derive(Clone)]
pub struct KbDocumentService {
    backend: Arc<StoreBackend>,
}

/// 文档摘要（索引条目，用于 list/stats）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocSummary {
    pub id: String,
    pub title: String,
    pub category: String,
    pub status: String,
    pub updated_at: String,
}

impl KbDocumentService {
    /// 包装已装配的存储后端
    pub fn new(backend: Arc<StoreBackend>) -> Self {
        Self { backend }
    }

    fn doc_key(id: &str) -> String {
        format!("{DOC_KEY_PREFIX}{id}.json")
    }

    /// 创建文档（自动分配 id）
    pub async fn create(&self, title: &str, content: &str, category: Option<&str>) -> crate::Result<KbDocument> {
        let cat = category
            .map(str::to_string)
            .unwrap_or_else(|| KbDocument::default_category().to_string());
        let doc = KbDocument::new(new_kb_id(), title.to_string(), content.to_string(), cat);
        self.save(&doc).await?;
        Ok(doc)
    }

    /// 保存文档（原子写对象 + 刷新索引）
    pub async fn save(&self, doc: &KbDocument) -> crate::Result<()> {
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
        serde_json::from_slice(&raw)
            .map_err(|e| StoreError::Other(format!("文档 JSON 损坏: {e}")))
    }

    /// 更新文档字段（title/content/category/tags 增量合并），保留实体/版本
    pub async fn update(&self, id: &str, patch: &Value) -> crate::Result<KbDocument> {
        let mut doc = self.get(id).await?;
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
        self.save(&doc).await?;
        Ok(doc)
    }

    /// 删除文档
    pub async fn delete(&self, id: &str) -> crate::Result<bool> {
        let existed = self.backend.object.exists(&Self::doc_key(id)).await?;
        if existed {
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
        if let Some(raw) = self.backend.kv.get(TAGS_KEY).await? {
            let map: HashMap<String, usize> = serde_json::from_slice(&raw).unwrap_or_default();
            for (tag, count) in map {
                tags.entry(tag).or_insert(count);
            }
        }
        let mut out: Vec<Value> = tags
            .into_iter()
            .map(|(tag, count)| json!({ "name": tag, "count": count }))
            .collect();
        out.sort_by(|a, b| {
            b["count"].as_u64().unwrap_or(0).cmp(&a["count"].as_u64().unwrap_or(0))
        });
        Ok(out)
    }

    /// 统计（documents 数 / categories 数 / tags 数 / storage 字节）
    pub async fn stats(&self) -> crate::Result<Value> {
        let docs = self.list().await?;
        let cats = self.categories().await?;
        let tags = self.tags().await?;
        let storage_bytes = self.estimate_storage_bytes(&docs).await;
        Ok(json!({
            "documents": docs.len(),
            "categories": cats.len(),
            "tags": tags.len(),
            "storage_bytes": storage_bytes,
        }))
    }

    /// 估算物理容量（head 对象累计）
    async fn estimate_storage_bytes(&self, docs: &[Value]) -> u64 {
        let mut total = 0u64;
        for d in docs {
            if let Some(id) = d["id"].as_str() {
                if let Ok(obj) = self.backend.object.head(&Self::doc_key(id)).await {
                    total += obj.size_bytes;
                }
            }
        }
        total
    }

    /// 重建摘要索引 + 标签聚合（list/stats 一致性）
    async fn rebuild_index(&self) -> crate::Result<()> {
        let mut summaries = Vec::new();
        let mut tags = HashMap::<String, usize>::new();
        let mut keys: Vec<String> = list_object_refs(&self.backend.data_dir)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(p, _)| p)
            .filter(|p| p.starts_with(DOC_KEY_PREFIX) && p.ends_with(".json"))
            .collect();
        if keys.is_empty() {
            // S3 后端 / 索引未落库时回退：复用旧索引缓存
            if let Some(raw) = self.backend.kv.get(INDEX_KEY).await? {
                if let Ok(prev) = serde_json::from_slice::<Vec<DocSummary>>(&raw) {
                    keys = prev.into_iter().map(|s| Self::doc_key(&s.id)).collect();
                }
            }
        }
        for key in keys {
            if let Ok(raw) = self.backend.object.get(&key).await {
                if let Ok(doc) = serde_json::from_slice::<KbDocument>(&raw) {
                    for t in &doc.tags {
                        *tags.entry(t.clone()).or_default() += 1;
                    }
                    summaries.push(DocSummary {
                        id: doc.id,
                        title: doc.title,
                        category: doc.category,
                        status: doc.status,
                        updated_at: doc.updated_at,
                    });
                }
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
        match self.backend.kv.get(INDEX_KEY).await? {
            Some(raw) => serde_json::from_slice(&raw)
                .map_err(|e| StoreError::Other(format!("索引损坏: {e}"))),
            None => Ok(Vec::new()),
        }
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
        assert_eq!(cats.len(), 3);
        let tech = cats.iter().find(|c| c["id"] == "cat-tech").unwrap();
        assert_eq!(tech["count"], 1);
        let stats = svc.stats().await.unwrap();
        assert_eq!(stats["documents"], 2);
        assert!(stats["storage_bytes"].as_u64().unwrap() > 0);
    }
}




