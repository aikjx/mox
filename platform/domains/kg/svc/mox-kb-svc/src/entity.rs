//! Document-owned entity references. Authorization is resolved against current documents;
//! no entity content or tenant-less relation collection is copied into this aggregate.
use crate::{document::KbDocumentService, model::KbDocument};
use bytes::Bytes;
use mox_base_store_core::StoreError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityMutation {
    pub entity_id: String,
    pub source_doc_id: String,
    pub source_version: String,
    pub source_acl_revision: u64,
    pub expected_current_version: String,
    pub expected_acl_revision: u64,
    pub expected_links_revision: u64,
    #[serde(default)]
    pub relation: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Reference {
    entity_id: String,
    source_doc_id: String,
    source_version: String,
    created_at: String,
    #[serde(default = "default_relation")]
    relation: String,
}
fn default_relation() -> String {
    "references".into()
}
#[derive(Debug, Default, Serialize, Deserialize)]
struct References {
    revision: u64,
    items: Vec<Reference>,
}
#[derive(Debug)]
pub enum EntityError {
    Invalid,
    Conflict,
    Storage(StoreError),
}
impl From<StoreError> for EntityError {
    fn from(error: StoreError) -> Self {
        Self::Storage(error)
    }
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
        && value != "."
        && value != ".."
}
fn key(id: &str) -> String {
    format!("kb/entity-links/{id}.json")
}

impl KbDocumentService {
    async fn read_entity_references(&self, id: &str) -> crate::Result<References> {
        match self.backend.object.get(&key(id)).await {
            Ok(raw) => {
                if raw.len() > 1024 * 1024 {
                    return Err(crate::err_other("entity reference object too large"));
                }
                let refs: References = serde_json::from_slice(&raw).map_err(crate::err_other)?;
                if refs.items.len() > 256
                    || refs.items.iter().any(|item| {
                        !identifier(&item.source_doc_id)
                            || !identifier(&item.entity_id)
                            || !identifier(&item.source_version)
                    })
                {
                    return Err(crate::err_other("invalid entity reference object"));
                }
                Ok(refs)
            },
            Err(StoreError::NotFound { .. }) => Ok(References::default()),
            Err(error) => Err(error),
        }
    }
    /// Visible primary document entities; deterministic order and filter before limit.
    /// Source scanning remains O(n) until an ACL-aware transactional index exists.
    pub async fn search_entities(
        &self,
        q: &str,
        kind: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Value>, EntityError> {
        if q.len() > 256 || kind.is_some_and(|v| v.len() > 64) || !(1..=100).contains(&limit) {
            return Err(EntityError::Invalid);
        }
        let needle = q.trim().to_lowercase();
        let mut index = self.read_index().await?;
        index.sort_by(|a, b| a.id.cmp(&b.id));
        let mut results = Vec::new();
        for summary in index {
            let doc = match self.get(&summary.id).await {
                Ok(doc) => doc,
                Err(StoreError::NotFound { .. }) => continue,
                Err(error) => return Err(error.into()),
            };
            let mut entities = doc.entities.iter().collect::<Vec<_>>();
            entities.sort_by(|a, b| a.id.cmp(&b.id));
            for entity in entities {
                if !entity.name.to_lowercase().contains(&needle)
                    || kind.is_some_and(|value| value != entity.entity_type)
                {
                    continue;
                }
                results.push(json!({"id":entity.id, "name":entity.name, "type":entity.entity_type,
                    "frequency":entity.frequency, "snippet":entity.snippet.chars().take(512).collect::<String>(),
                    "source_doc_id":doc.id,"source_version":doc.current_version,"source_acl_revision":doc.acl_revision}));
                if results.len() == limit {
                    return Ok(results);
                }
            }
        }
        Ok(results)
    }
    async fn resolved_entity_references(
        &self,
        doc: &KbDocument,
        refs: &References,
    ) -> crate::Result<Value> {
        let mut visible = Vec::new();
        for item in &refs.items {
            let source = match self.get(&item.source_doc_id).await {
                Ok(source) => source,
                Err(StoreError::NotFound { .. }) => continue,
                Err(error) => return Err(error),
            };
            // A stale pointer must not silently reference new extraction results.
            if source.current_version != item.source_version {
                continue;
            }
            if let Some(entity) = source.entities.iter().find(|entity| entity.id == item.entity_id)
            {
                visible.push(json!({"id":entity.id,"name":entity.name,"type":entity.entity_type,
                    "source_doc_id":item.source_doc_id,"source_version":item.source_version,
                    "source_acl_revision":source.acl_revision,"created_at":item.created_at,"relation":item.relation}));
            }
        }
        Ok(json!({"doc_id":doc.id,"entities":doc.entities,"relations":doc.relations,
            "current_version":doc.current_version,"acl_revision":doc.acl_revision,
            "links_revision":refs.revision,"linked_entities":visible}))
    }
    pub async fn document_entities(&self, id: &str) -> crate::Result<Value> {
        let _guard = self.mutation.lock().await;
        let doc = self.get(id).await?;
        let refs = self.read_entity_references(id).await?;
        self.resolved_entity_references(&doc, &refs).await
    }
    /// Serialized with local document/ACL mutations. This is not cross-process CAS.
    pub async fn mutate_entity_reference(
        &self,
        id: &str,
        request: &EntityMutation,
        remove: bool,
    ) -> Result<Value, EntityError> {
        if !identifier(id)
            || !identifier(&request.source_doc_id)
            || !identifier(&request.entity_id)
            || !identifier(&request.source_version)
            || !identifier(&request.expected_current_version)
            || request.relation.as_ref().is_some_and(|value| {
                value.trim().is_empty()
                    || value != value.trim()
                    || value.len() > 64
                    || value.chars().any(char::is_control)
            })
        {
            return Err(EntityError::Invalid);
        }
        let _guard = self.mutation.lock().await;
        let doc = self.get_for_write(id).await?;
        let mut refs = self.read_entity_references(id).await?;
        if doc.current_version != request.expected_current_version
            || doc.acl_revision != request.expected_acl_revision
            || refs.revision != request.expected_links_revision
        {
            return Err(EntityError::Conflict);
        }
        let existing = refs.items.iter().position(|item| {
            item.entity_id == request.entity_id
                && item.source_doc_id == request.source_doc_id
                && item.source_version == request.source_version
        });
        if remove {
            let position = existing.ok_or_else(|| StoreError::NotFound { path: id.to_owned() })?;
            refs.items.remove(position);
        } else {
            let source = self.get(&request.source_doc_id).await?;
            if source.current_version != request.source_version
                || source.acl_revision != request.source_acl_revision
            {
                return Err(EntityError::Conflict);
            }
            if !source.entities.iter().any(|entity| entity.id == request.entity_id) {
                return Err(StoreError::NotFound { path: id.to_owned() }.into());
            }
            if let Some(position) = existing {
                if refs.items[position].relation
                    != request.relation.clone().unwrap_or_else(default_relation)
                {
                    return Err(EntityError::Conflict);
                }
                return self.resolved_entity_references(&doc, &refs).await.map_err(Into::into);
            }
            if refs.items.len() >= 256 {
                return Err(EntityError::Invalid);
            }
            refs.items.push(Reference {
                entity_id: request.entity_id.clone(),
                source_doc_id: request.source_doc_id.clone(),
                source_version: request.source_version.clone(),
                created_at: crate::model::now_iso(),
                relation: request.relation.clone().unwrap_or_else(|| "references".into()),
            });
        }
        refs.revision = refs.revision.checked_add(1).ok_or(EntityError::Conflict)?;
        // Resolve before committing so corrupted source data cannot produce a false success.
        let result = self.resolved_entity_references(&doc, &refs).await?;
        self.backend
            .object
            .put(
                &key(id),
                "application/json",
                Bytes::from(serde_json::to_vec(&refs).map_err(crate::err_other)?),
            )
            .await?;
        Ok(result)
    }
}
