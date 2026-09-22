// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! proto 契约实现绑定
//!
//! 将 [`mox_alliance_registry_proto`] 的 [`ExpertDirectory`] / [`InstanceRegistry`]
//! 契约绑定到本服务的存储实现（`ExpertStore` / `RegistryStore`），
//! 使消费方可依赖协议层 trait 而非服务内部类型。

use mox_alliance_registry_proto::{
    ExpertDirectory, InstanceQuery, InstanceRegistry, InstanceStatus, RegisteredInstance,
    RegistryError, RegistryResult,
};

use crate::storage::{ExpertStore, RegistryStore};

type StoreError = Box<dyn std::error::Error>;

fn map_err(e: StoreError) -> RegistryError {
    RegistryError::new(e.to_string())
}

impl ExpertDirectory for ExpertStore {
    fn list_experts(&self) -> RegistryResult<Vec<crate::models::Expert>> {
        ExpertStore::list(self).map_err(map_err)
    }
    fn get_expert(&self, id: &str) -> RegistryResult<Option<crate::models::Expert>> {
        ExpertStore::get(self, id).map_err(map_err)
    }
    fn create_expert(&self, expert: &crate::models::Expert) -> RegistryResult<()> {
        ExpertStore::create(self, expert).map_err(map_err)
    }
    fn update_expert(&self, expert: &crate::models::Expert) -> RegistryResult<()> {
        ExpertStore::update(self, expert).map_err(map_err)
    }
    fn delete_expert(&self, id: &str) -> RegistryResult<()> {
        ExpertStore::delete(self, id).map_err(map_err)
    }
}

impl InstanceRegistry for RegistryStore {
    fn register_instance(&self, inst: RegisteredInstance) {
        RegistryStore::register(self, inst)
    }
    fn deregister_instance(&self, id: &str) -> Option<RegisteredInstance> {
        RegistryStore::deregister(self, id)
    }
    fn get_instance(&self, id: &str) -> Option<RegisteredInstance> {
        RegistryStore::get(self, id)
    }
    fn heartbeat(
        &self,
        id: &str,
        load_current: Option<u32>,
        status: Option<InstanceStatus>,
    ) -> Option<RegisteredInstance> {
        RegistryStore::heartbeat(self, id, load_current, status)
    }
    fn discover(&self, query: &InstanceQuery) -> Vec<RegisteredInstance> {
        RegistryStore::list(self, query)
    }
    fn instance_count(&self) -> usize {
        RegistryStore::count(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_registry_proto::Expert;
    use std::sync::Arc;

    #[test]
    fn expert_store_satisfies_directory_contract() {
        let store = ExpertStore::memory().unwrap();
        let dir: Arc<dyn ExpertDirectory> = Arc::new(store);
        assert!(dir.list_experts().unwrap().is_empty());

        let e = Expert::new("contract-expert".to_string());
        dir.create_expert(&e).unwrap();
        assert_eq!(dir.get_expert(&e.id).unwrap().unwrap().name, "contract-expert");

        dir.delete_expert(&e.id).unwrap();
        assert!(dir.get_expert(&e.id).unwrap().is_none());
    }

    #[test]
    fn registry_store_satisfies_instance_contract() {
        let store = RegistryStore::new_memory();
        let reg: &dyn InstanceRegistry = &store;
        assert_eq!(reg.instance_count(), 0);

        let now = chrono::Utc::now();
        let inst = RegisteredInstance {
            id: "c-1".into(),
            name: "expert-code".into(),
            version: "1.0.0".into(),
            endpoint: "http://127.0.0.1:3300".into(),
            health_check_url: None,
            capabilities: vec!["code".into()],
            domain: Some("code".into()),
            weight: 1.0,
            load_current: 0,
            load_capacity: 100,
            status: InstanceStatus::Active,
            registered_at: now,
            last_heartbeat_at: now,
            lease_seconds: 15,
            metadata: Default::default(),
        };
        reg.register_instance(inst);
        assert_eq!(reg.instance_count(), 1);

        let q = InstanceQuery {
            capability: Some("code".into()),
            ..Default::default()
        };
        assert_eq!(reg.discover(&q).len(), 1);

        reg.heartbeat("c-1", Some(5), None).unwrap();
        assert_eq!(reg.get_instance("c-1").unwrap().load_current, 5);

        reg.deregister_instance("c-1").unwrap();
        assert!(reg.get_instance("c-1").is_none());
    }
}
