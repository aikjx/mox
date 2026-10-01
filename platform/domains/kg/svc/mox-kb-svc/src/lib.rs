// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX 云盘知识库服务（mox-kb-svc）
//!
//! 100% 自研知识库业务层，构建于混合存储架构之上：
//!
//! ```text
//!                 ┌─────────────────────────────────────────────┐
//!                 │   mox-kb-svc（本 crate）                      │
//!                 │   model / document / version / analyze /      │
//!                 │   link / search / handlers                    │
//!                 └───────┬──────────────────────┬───────────────┘
//!                         │                      │
//!                 mox-cloud-store-core     mox-kg-storage-svc
//!                 （内容寻址去重存储）        （图谱节点/边落库）
//!                         │
//!                 mox-base-store-core（物理口契约）
//! ```
//!
//! - **存储**：文档对象落 store-core 内容寻址去重后端（FS/S3 可插拔）。
//! - **分析**：本地 NLP-lite 确定性抽取 + 专家联盟咨询（`mox-ai-expert-svc`）。
//! - **挂图**：文档/分块/实体/关系落 `GraphStore`（`contains`/`mentions`/`relates`）。
//! - **检索**：标题加权关键词检索 + 图谱节点检索，分类过滤与结果排序。
//! - **适配**：`handlers::build_kb_router()` 对齐 legacy `/kb/*` API 面，前端零改动。

pub mod access;
pub mod handlers;
pub mod model;

pub mod analyze;
pub mod document;
mod expert_gate;
pub mod link;
mod search;
mod version;

pub use expert_gate::{ExpertGate, GateEvidence, GateReport};

use mox_base_store_core::StoreError;
use mox_cloud_sdk::{BackendKind, StoreBackend, StoreConfig, create_backend};
use mox_kg_storage_svc::GraphStore;
use std::sync::Arc;

pub use document::KbDocumentService;
pub use model::{KbDocument, SearchHit, SearchRequest};

/// 知识库统一结果类型
pub type Result<T> = std::result::Result<T, StoreError>;

/// 通用错误构造（任意 Display 错误 → StoreError::Other）
pub fn err_other<E: std::fmt::Display>(msg: E) -> StoreError {
    StoreError::Other(msg.to_string())
}

/// 知识库全局状态：文档服务 + 图谱存储
///
/// - `docs`：基于 store-core 内容寻址后端的文档 CRUD/版本/索引。
/// - `graph`：内存图谱（文档子图节点边）。
#[derive(Clone)]
pub struct KbState {
    pub docs: KbDocumentService,
    pub graph: GraphStore,
    projection_ready: Arc<tokio::sync::OnceCell<()>>,
}

impl KbState {
    pub fn scoped(&self, access: access::KnowledgeAccess) -> Self {
        Self {
            docs: self.docs.scoped(access),
            graph: self.graph.clone(),
            projection_ready: self.projection_ready.clone(),
        }
    }
    /// Filter before graph search/ranking/counting; nodes without provenance are excluded.
    pub async fn visible_graph(&self) -> Result<GraphStore> {
        if !self.docs.is_scoped() {
            return Ok(self.graph.clone());
        }
        let ids: std::collections::HashSet<String> = self
            .docs
            .list()
            .await?
            .into_iter()
            .filter_map(|v| v["id"].as_str().map(str::to_owned))
            .collect();
        let mut snapshot = self.graph.snapshot();
        snapshot.nodes.retain(|n| {
            n.properties
                .get("source_doc_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| ids.contains(id))
        });
        let nodes: std::collections::HashSet<_> =
            snapshot.nodes.iter().map(|n| n.id.clone()).collect();
        snapshot
            .edges
            .retain(|e| nodes.contains(&e.source) && nodes.contains(&e.target));
        let visible = GraphStore::new();
        visible.import_snapshot(snapshot).map_err(err_other)?;
        Ok(visible)
    }

    /// 从环境装配存储后端（`FILE_BACKEND` + `MOX_STORE_DATA_DIR`，与 cloud-api 同约定）
    pub fn from_env() -> Self {
        let cfg = StoreConfig::from_env().expect("invalid KB storage configuration");
        let backend = Arc::new(create_backend(&cfg).expect("configured KB storage unavailable"));
        Self::new(backend)
    }

    /// 包装已装配的存储后端
    pub fn new(backend: Arc<StoreBackend>) -> Self {
        Self {
            docs: KbDocumentService::new(backend),
            graph: GraphStore::new(),
            projection_ready: Arc::new(tokio::sync::OnceCell::new()),
        }
    }

    /// 从持久化文档主源重建挂图投影。初始化失败不发布半张图，下次请求可重试。
    pub async fn ensure_projection_ready(&self) -> Result<()> {
        self.projection_ready
            .get_or_try_init(|| async {
                let rebuilt = GraphStore::new();
                let docs = self.docs.unscoped();
                for item in docs.list().await? {
                    let id =
                        item["id"].as_str().ok_or_else(|| err_other("invalid KB index entry"))?;
                    let doc = docs.get(id).await?;
                    if doc.status == model::STATUS_LINKED {
                        link::GraphLinker
                            .link(&rebuilt, &doc, &analyze::chunk_doc(&doc))
                            .map_err(err_other)?;
                    }
                }
                self.graph.import_snapshot(rebuilt.snapshot()).map_err(err_other)?;
                Ok::<(), StoreError>(())
            })
            .await?;
        Ok(())
    }

    /// 使用显式数据目录装配 FS 后端（测试/单测注入，避免进程级环境变量竞态）
    pub fn with_data_dir(dir: std::path::PathBuf) -> Self {
        let cfg = StoreConfig {
            kind: BackendKind::Fs,
            data_dir: dir,
            verify_checksum: true,
            s3: None,
        };
        let backend = Arc::new(create_backend(&cfg).expect("FS 后端必须可装配"));
        Self::new(backend)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use mox_cloud_sdk::{BackendKind, StoreConfig, create_backend};

    /// 临时目录 FS 后端（测试双，每次调用独立目录）
    pub(crate) fn fs_backend() -> Arc<StoreBackend> {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = StoreConfig {
            kind: BackendKind::Fs,
            data_dir: dir.path().to_path_buf(),
            verify_checksum: true,
            s3: None,
        };
        let backend = Arc::new(create_backend(&cfg).unwrap());
        // 目录由 tempfile 生命周期持有；backend 内部引用 data_dir，测试内有效即可
        std::mem::forget(dir);
        backend
    }

    /// 知识库测试状态
    pub(crate) fn kb_state() -> KbState {
        KbState::new(fs_backend())
    }
}
