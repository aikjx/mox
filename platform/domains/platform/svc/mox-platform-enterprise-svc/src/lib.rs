// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! MOX Platform Enterprise Service
//!
//! Enterprise-grade services: multi-tenancy, audit logging, feature flags,
//! configuration management, and health checks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// 应用状态模块
pub mod app_state;
/// 认证模块
pub mod auth;
/// 路由模块
pub mod routes;

#[derive(Debug, Error)]
pub enum EnterpriseError {
    #[error("tenant not found: {0}")]
    TenantNotFound(String),
    #[error("feature flag not found: {0}")]
    FeatureNotFound(String),
    #[error("configuration key not found: {0}")]
    ConfigNotFound(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
}

// ─── Multi-Tenancy ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub plan: TenantPlan,
    pub status: TenantStatus,
    pub created_at: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TenantPlan { Free, Pro, Enterprise, Custom }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TenantStatus { Active, Suspended, Deleted }

#[derive(Clone)]
pub struct TenantManager {
    tenants: Arc<parking_lot::RwLock<HashMap<String, Tenant>>>,
}

impl TenantManager {
    pub fn new() -> Self { Self { tenants: Arc::new(parking_lot::RwLock::new(HashMap::new())) } }

    pub fn create(&self, name: &str, plan: TenantPlan) -> Tenant {
        let tenant = Tenant {
            id: Uuid::now_v7().to_string(),
            name: name.into(), plan, status: TenantStatus::Active,
            created_at: chrono::Utc::now().to_rfc3339(), metadata: serde_json::json!({}),
        };
        self.tenants.write().insert(tenant.id.clone(), tenant.clone());
        tenant
    }

    pub fn get(&self, id: &str) -> Option<Tenant> { self.tenants.read().get(id).cloned() }
    pub fn list(&self) -> Vec<Tenant> { self.tenants.read().values().cloned().collect() }
    pub fn suspend(&self, id: &str) -> Result<(), EnterpriseError> {
        let mut t = self.tenants.write();
        let tenant = t.get_mut(id).ok_or_else(|| EnterpriseError::TenantNotFound(id.into()))?;
        tenant.status = TenantStatus::Suspended;
        Ok(())
    }
    pub fn is_active(&self, id: &str) -> bool {
        self.tenants.read().get(id).map(|t| t.status == TenantStatus::Active).unwrap_or(false)
    }
}

impl Default for TenantManager { fn default() -> Self { Self::new() } }

// ─── Audit Logging ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: String,
    pub tenant_id: Option<String>,
    pub actor: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub result: AuditResult,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditResult { Success, Failure, Denied }

#[derive(Clone)]
pub struct AuditLog {
    entries: Arc<parking_lot::RwLock<Vec<AuditEntry>>>,
    max_entries: usize,
}

impl AuditLog {
    pub fn new() -> Self { Self { entries: Arc::new(parking_lot::RwLock::new(Vec::new())), max_entries: 100_000 } }
    pub fn with_capacity(max: usize) -> Self { Self { entries: Arc::new(parking_lot::RwLock::new(Vec::new())), max_entries: max } }

    pub fn record(&self, actor: &str, action: &str, resource_type: &str, resource_id: &str, result: AuditResult) -> AuditEntry {
        let entry = AuditEntry {
            id: Uuid::now_v7().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            tenant_id: None, actor: actor.into(), action: action.into(),
            resource_type: resource_type.into(), resource_id: resource_id.into(),
            result, details: serde_json::json!({}), ip_address: None,
        };
        let mut entries = self.entries.write();
        entries.push(entry.clone());
        if entries.len() > self.max_entries {
            let drain_count = entries.len() - self.max_entries;
            entries.drain(0..drain_count);
        }
        entry
    }

    pub fn query(&self, actor: Option<&str>, action: Option<&str>, limit: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        entries.iter().rev()
            .filter(|e| actor.map_or(true, |a| e.actor == a))
            .filter(|e| action.map_or(true, |a| e.action == a))
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn count(&self) -> usize { self.entries.read().len() }
}

impl Default for AuditLog { fn default() -> Self { Self::new() } }

// ─── Feature Flags ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    pub key: String,
    pub name: String,
    pub enabled: bool,
    pub rollout_percentage: u8,
    pub tenant_overrides: HashMap<String, bool>,
    pub description: String,
}

#[derive(Clone)]
pub struct FeatureFlagManager {
    flags: Arc<parking_lot::RwLock<HashMap<String, FeatureFlag>>>,
}

impl FeatureFlagManager {
    pub fn new() -> Self { Self { flags: Arc::new(parking_lot::RwLock::new(HashMap::new())) } }

    pub fn register(&self, key: &str, name: &str, default_enabled: bool, description: &str) {
        self.flags.write().insert(key.into(), FeatureFlag {
            key: key.into(), name: name.into(), enabled: default_enabled,
            rollout_percentage: if default_enabled { 100 } else { 0 },
            tenant_overrides: HashMap::new(), description: description.into(),
        });
    }

    pub fn is_enabled(&self, key: &str, tenant_id: Option<&str>) -> bool {
        let flags = self.flags.read();
        let Some(flag) = flags.get(key) else { return false; };
        if let Some(tid) = tenant_id {
            if let Some(override_val) = flag.tenant_overrides.get(tid) {
                return *override_val;
            }
        }
        if flag.rollout_percentage >= 100 { return flag.enabled; }
        // Percentage-based rollout using hash of tenant_id
        if let Some(tid) = tenant_id {
            let hash = tid.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
            return (hash % 100) < flag.rollout_percentage as u64;
        }
        flag.enabled
    }

    pub fn set_enabled(&self, key: &str, enabled: bool) -> Result<(), EnterpriseError> {
        let mut flags = self.flags.write();
        let flag = flags.get_mut(key).ok_or_else(|| EnterpriseError::FeatureNotFound(key.into()))?;
        flag.enabled = enabled;
        flag.rollout_percentage = if enabled { 100 } else { 0 };
        Ok(())
    }

    pub fn set_tenant_override(&self, key: &str, tenant_id: &str, enabled: bool) -> Result<(), EnterpriseError> {
        let mut flags = self.flags.write();
        let flag = flags.get_mut(key).ok_or_else(|| EnterpriseError::FeatureNotFound(key.into()))?;
        flag.tenant_overrides.insert(tenant_id.into(), enabled);
        Ok(())
    }

    pub fn list(&self) -> Vec<FeatureFlag> { self.flags.read().values().cloned().collect() }
}

impl Default for FeatureFlagManager { fn default() -> Self { Self::new() } }

// ─── Configuration Management ───

#[derive(Clone)]
pub struct ConfigStore {
    values: Arc<parking_lot::RwLock<HashMap<String, String>>>,
}

impl ConfigStore {
    pub fn new() -> Self { Self { values: Arc::new(parking_lot::RwLock::new(HashMap::new())) } }

    pub fn set(&self, key: &str, value: &str) {
        self.values.write().insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.values.read().get(key).cloned()
    }

    pub fn get_or_default(&self, key: &str, default: &str) -> String {
        self.values.read().get(key).cloned().unwrap_or_else(|| default.into())
    }

    pub fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.get(key).and_then(|v| serde_json::from_str(&v).ok())
    }

    pub fn set_json<T: Serialize>(&self, key: &str, value: &T) {
        if let Ok(json) = serde_json::to_string(value) {
            self.set(key, &json);
        }
    }

    pub fn remove(&self, key: &str) -> Option<String> {
        self.values.write().remove(key)
    }

    pub fn keys_with_prefix(&self, prefix: &str) -> Vec<String> {
        self.values.read().keys().filter(|k| k.starts_with(prefix)).cloned().collect()
    }
}

impl Default for ConfigStore { fn default() -> Self { Self::new() } }

// ─── Health Check ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: HealthState,
    pub version: String,
    pub uptime_seconds: u64,
    pub components: HashMap<String, ComponentHealth>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthState { Healthy, Degraded, Unhealthy }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: HealthState,
    pub message: String,
    pub last_check: String,
}

#[derive(Clone)]
pub struct HealthChecker {
    start_time: std::time::Instant,
    components: Arc<parking_lot::RwLock<HashMap<String, ComponentHealth>>>,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self { start_time: std::time::Instant::now(), components: Arc::new(parking_lot::RwLock::new(HashMap::new())) }
    }

    pub fn register_component(&self, name: &str) {
        self.components.write().insert(name.into(), ComponentHealth {
            status: HealthState::Healthy, message: "ok".into(),
            last_check: chrono::Utc::now().to_rfc3339(),
        });
    }

    pub fn report(&self, name: &str, status: HealthState, message: &str) {
        if let Some(c) = self.components.write().get_mut(name) {
            c.status = status;
            c.message = message.into();
            c.last_check = chrono::Utc::now().to_rfc3339();
        }
    }

    pub fn check(&self) -> HealthStatus {
        let components = self.components.read().clone();
        let overall = if components.values().all(|c| c.status == HealthState::Healthy) {
            HealthState::Healthy
        } else if components.values().any(|c| c.status == HealthState::Unhealthy) {
            HealthState::Unhealthy
        } else {
            HealthState::Degraded
        };
        HealthStatus {
            status: overall,
            version: env!("CARGO_PKG_VERSION").into(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            components,
        }
    }
}

impl Default for HealthChecker { fn default() -> Self { Self::new() } }

// ─── Enterprise Service Facade ───

#[derive(Clone)]
pub struct EnterpriseService {
    pub tenants: TenantManager,
    pub audit: AuditLog,
    pub features: FeatureFlagManager,
    pub config: ConfigStore,
    pub health: HealthChecker,
}

impl EnterpriseService {
    pub fn new() -> Self {
        let svc = Self {
            tenants: TenantManager::new(),
            audit: AuditLog::new(),
            features: FeatureFlagManager::new(),
            config: ConfigStore::new(),
            health: HealthChecker::new(),
        };
        // Register default components
        svc.health.register_component("system");
        svc.health.register_component("database");
        svc.health.register_component("ai-engine");
        svc
    }
}

impl Default for EnterpriseService { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_lifecycle() {
        let mgr = TenantManager::new();
        let t = mgr.create("Acme", TenantPlan::Enterprise);
        assert!(mgr.is_active(&t.id));
        mgr.suspend(&t.id).unwrap();
        assert!(!mgr.is_active(&t.id));
    }

    #[test]
    fn audit_log_record_query() {
        let log = AuditLog::new();
        log.record("user1", "login", "session", "s1", AuditResult::Success);
        log.record("user1", "delete", "document", "d1", AuditResult::Denied);
        assert_eq!(log.count(), 2);
        let denied = log.query(None, Some("delete"), 10);
        assert_eq!(denied.len(), 1);
        assert_eq!(denied[0].result, AuditResult::Denied);
    }

    #[test]
    fn feature_flags() {
        let ff = FeatureFlagManager::new();
        ff.register("new_ui", "New UI", false, "Experimental UI");
        assert!(!ff.is_enabled("new_ui", None));
        ff.set_enabled("new_ui", true).unwrap();
        assert!(ff.is_enabled("new_ui", None));
    }

    #[test]
    fn feature_tenant_override() {
        let ff = FeatureFlagManager::new();
        ff.register("beta", "Beta Feature", false, "Beta");
        ff.set_tenant_override("beta", "tenant_a", true).unwrap();
        assert!(ff.is_enabled("beta", Some("tenant_a")));
        assert!(!ff.is_enabled("beta", Some("tenant_b")));
    }

    #[test]
    fn config_store() {
        let cfg = ConfigStore::new();
        cfg.set("key1", "value1");
        assert_eq!(cfg.get("key1"), Some("value1".into()));
        assert_eq!(cfg.get("missing"), None);
        assert_eq!(cfg.get_or_default("missing", "def"), "def");
    }

    #[test]
    fn config_json() {
        let cfg = ConfigStore::new();
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct S { a: i32, b: String }
        cfg.set_json("obj", &S { a: 42, b: "hello".into() });
        let got: S = cfg.get_json("obj").unwrap();
        assert_eq!(got, S { a: 42, b: "hello".into() });
    }

    #[test]
    fn health_check() {
        let hc = HealthChecker::new();
        hc.register_component("test");
        let status = hc.check();
        assert_eq!(status.status, HealthState::Healthy);
        hc.report("test", HealthState::Unhealthy, "connection lost");
        let status = hc.check();
        assert_eq!(status.status, HealthState::Unhealthy);
    }

    #[test]
    fn enterprise_service_facade() {
        let svc = EnterpriseService::new();
        let t = svc.tenants.create("TestCorp", TenantPlan::Pro);
        svc.audit.record("admin", "create_tenant", "tenant", &t.id, AuditResult::Success);
        svc.features.register("test_flag", "Test", true, "test");
        assert!(svc.features.is_enabled("test_flag", None));
        assert_eq!(svc.audit.count(), 1);
    }
}
