//! Trusted resource identity. Populate only after authentication, never from HTTP headers/body.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeAccess {
    pub tenant_id: String,
    pub owner_id: String,
    #[serde(skip)]
    pub administrator: bool,
    #[serde(skip)]
    pub readonly: bool,
}
impl KnowledgeAccess {
    pub fn valid(&self) -> bool {
        !self.tenant_id.trim().is_empty() && !self.owner_id.trim().is_empty()
    }
    pub fn allows_request(&self, method: &str, path: &str) -> bool {
        self.valid()
            && (!self.readonly
                || matches!(method, "GET" | "HEAD")
                || (method == "POST"
                    && (path == "/kb/search" || path.ends_with("/versions/compare"))))
    }
    pub fn can_read(&self, resource: &Option<Self>, readers: &[String]) -> bool {
        self.permits(resource)
            || (self.valid()
                && resource.as_ref().is_some_and(|r| {
                    r.valid() && r.tenant_id == self.tenant_id && readers.contains(&self.owner_id)
                }))
    }
    pub fn permits(&self, resource: &Option<Self>) -> bool {
        self.valid()
            && resource.as_ref().is_some_and(|r| {
                r.valid()
                    && r.tenant_id == self.tenant_id
                    && (self.administrator || r.owner_id == self.owner_id)
            })
    }
}

#[derive(Debug)]
pub enum ShareError {
    InvalidRequest,
    Conflict,
    Storage(mox_base_store_core::StoreError),
}
impl From<mox_base_store_core::StoreError> for ShareError {
    fn from(error: mox_base_store_core::StoreError) -> Self {
        Self::Storage(error)
    }
}
