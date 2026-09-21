// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家注册中心存储层
//!
//! 两套并存、职责分离的存储后端：
//! - [`ExpertStore`]：静态专家目录（SQLite 持久化），对应旧 `/api/v1/experts` CRUD，
//!   保持向后兼容。
//! - [`RegistryStore`]：**应用级专家实例注册中心**，内存 `RwLock<HashMap>` 为主
//!   （高吞吐），可选叠加 JSON 快照持久化（临时文件 + 原子 rename，重启可恢复）。
//!   该模式参考 scheduler-core 的 `FileTaskRepository`。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use rusqlite::{Connection, params};
use tracing::warn;

use crate::models::{
    InstanceQuery, InstanceStatus, RegisteredInstance,
};

// ─── 旧版静态专家目录（SQLite）─────────────────────────────────────────────

use crate::models::Expert;

/// 专家存储（SQLite 实现）
pub struct ExpertStore {
    conn: Arc<std::sync::Mutex<Connection>>,
}

impl ExpertStore {
    /// 打开或创建数据库
    pub fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS experts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                title TEXT,
                organization TEXT,
                domains TEXT,
                skills TEXT,
                bio TEXT,
                enabled INTEGER,
                rating REAL,
                total_consultations INTEGER
            );"
        )?;
        Ok(Self {
            conn: Arc::new(std::sync::Mutex::new(conn)),
        })
    }

    /// 内存数据库（测试用）
    pub fn memory() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS experts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                title TEXT,
                organization TEXT,
                domains TEXT,
                skills TEXT,
                bio TEXT,
                enabled INTEGER,
                rating REAL,
                total_consultations INTEGER
            );"
        )?;
        Ok(Self {
            conn: Arc::new(std::sync::Mutex::new(conn)),
        })
    }

    /// 列出所有专家
    pub fn list(&self) -> Result<Vec<Expert>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, title, organization, domains, skills, bio, enabled, rating, total_consultations FROM experts")?;
        let experts = stmt.query_map([], |row| {
            let domains_json: String = row.get(4)?;
            let skills_json: String = row.get(5)?;
            Ok(Expert {
                id: row.get(0)?,
                name: row.get(1)?,
                title: row.get(2)?,
                organization: row.get(3)?,
                domains: serde_json::from_str(&domains_json).unwrap_or_default(),
                skills: serde_json::from_str(&skills_json).unwrap_or_default(),
                bio: row.get(6)?,
                enabled: row.get::<_, i32>(7)? != 0,
                rating: row.get(8)?,
                total_consultations: row.get(9)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(experts)
    }

    /// 获取单个专家
    pub fn get(&self, id: &str) -> Result<Option<Expert>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, title, organization, domains, skills, bio, enabled, rating, total_consultations FROM experts WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let domains_json: String = row.get(4)?;
            let skills_json: String = row.get(5)?;
            Ok(Some(Expert {
                id: row.get(0)?,
                name: row.get(1)?,
                title: row.get(2)?,
                organization: row.get(3)?,
                domains: serde_json::from_str(&domains_json).unwrap_or_default(),
                skills: serde_json::from_str(&skills_json).unwrap_or_default(),
                bio: row.get(6)?,
                enabled: row.get::<_, i32>(7)? != 0,
                rating: row.get(8)?,
                total_consultations: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 创建专家
    pub fn create(&self, expert: &Expert) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO experts (id, name, title, organization, domains, skills, bio, enabled, rating, total_consultations) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                expert.id,
                expert.name,
                expert.title,
                expert.organization,
                serde_json::to_string(&expert.domains)?,
                serde_json::to_string(&expert.skills)?,
                expert.bio,
                expert.enabled as i32,
                expert.rating,
                expert.total_consultations,
            ],
        )?;
        Ok(())
    }

    /// 更新专家
    pub fn update(&self, expert: &Expert) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE experts SET name = ?2, title = ?3, organization = ?4, domains = ?5, skills = ?6, bio = ?7, enabled = ?8, rating = ?9, total_consultations = ?10 WHERE id = ?1",
            params![
                expert.id,
                expert.name,
                expert.title,
                expert.organization,
                serde_json::to_string(&expert.domains)?,
                serde_json::to_string(&expert.skills)?,
                expert.bio,
                expert.enabled as i32,
                expert.rating,
                expert.total_consultations,
            ],
        )?;
        Ok(())
    }

    /// 删除专家
    pub fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM experts WHERE id = ?1", params![id])?;
        Ok(())
    }
}

// ─── 应用级专家实例注册中心（内存 + 可选 JSON 快照）────────────────────────

/// 注册中心存储
///
/// 读写均以整条 [`RegisteredInstance`] 为单位。`RwLock` 并发：读多写少场景下
/// 并发读不互斥。可选 `snapshot` 路径时，写操作后原子落盘，启动时自动加载。
#[derive(Clone)]
pub struct RegistryStore {
    inner: Arc<RwLock<HashMap<String, RegisteredInstance>>>,
    /// JSON 快照路径；为 `None` 时纯内存（高性能）
    snapshot: Option<PathBuf>,
}

impl RegistryStore {
    /// 纯内存存储（默认，高性能，重启不保留）
    pub fn new_memory() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            snapshot: None,
        }
    }

    /// 文件快照存储：构造时加载已有快照，写后原子落盘
    pub fn with_snapshot(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let store = Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            snapshot: Some(path),
        };
        store.load_snapshot();
        store
    }

    /// 注册（新增或覆盖同 ID 实例）；重复注册视为续约，重置心跳与状态
    pub fn register(&self, inst: RegisteredInstance) {
        {
            let mut map = self.inner.write();
            map.insert(inst.id.clone(), inst);
        }
        self.persist();
    }

    /// 注销；返回被注销的实例（不存在返回 None）
    pub fn deregister(&self, id: &str) -> Option<RegisteredInstance> {
        let removed = { self.inner.write().remove(id) };
        if removed.is_some() {
            self.persist();
        }
        removed
    }

    /// 按 ID 取实例
    pub fn get(&self, id: &str) -> Option<RegisteredInstance> {
        self.inner.read().get(id).cloned()
    }

    /// 实例总数（含所有状态）
    pub fn count(&self) -> usize {
        self.inner.read().len()
    }

    /// 心跳续约：刷新最近心跳时间、更新负载/期望状态
    ///
    /// 不存在的实例返回 `None`（调用方应先注册）。
    pub fn heartbeat(
        &self,
        id: &str,
        load_current: Option<u32>,
        status: Option<InstanceStatus>,
    ) -> Option<RegisteredInstance> {
        let updated = {
            let mut map = self.inner.write();
            let inst = map.get_mut(id)?;
            inst.last_heartbeat_at = chrono::Utc::now();
            if let Some(load) = load_current {
                inst.load_current = load;
            }
            if let Some(status) = status {
                inst.status = status;
            } else if inst.status == InstanceStatus::Unhealthy {
                // 心跳到达即视为恢复活跃（除非显式上报别的状态）
                inst.status = InstanceStatus::Active;
            }
            inst.clone()
        };
        self.persist();
        Some(updated)
    }

    /// 主动标记状态（如后台主动探测失败标记 Unhealthy）
    pub fn set_status(&self, id: &str, status: InstanceStatus) -> Option<RegisteredInstance> {
        let updated = {
            let mut map = self.inner.write();
            let inst = map.get_mut(id)?;
            inst.status = status;
            inst.clone()
        };
        self.persist();
        Some(updated)
    }

    /// 按查询条件过滤实例
    ///
    /// - `capability`：实例能力标签包含该串
    /// - `domain`：领域相等（忽略大小写）
    /// - `status`：状态相等（lowercase 字符串）
    /// - `name`：名称子串包含
    ///
    /// 不带 `status` 过滤时，默认只返回「可发现」实例（Active/Draining）。
    pub fn list(&self, query: &InstanceQuery) -> Vec<RegisteredInstance> {
        let map = self.inner.read();
        let mut out: Vec<&RegisteredInstance> = match &query.status {
            Some(s) => map
                .values()
                .filter(|i| status_matches(i.status, s))
                .collect(),
            None => map.values().filter(|i| i.status.is_discoverable()).collect(),
        };

        if let Some(cap) = &query.capability {
            let cap = cap.to_lowercase();
            out.retain(|i| i.capabilities.iter().any(|c| c.to_lowercase().contains(&cap)));
        }
        if let Some(domain) = &query.domain {
            let domain = domain.to_lowercase();
            out.retain(|i| i.domain.as_ref().map(|d| d.to_lowercase() == domain).unwrap_or(false));
        }
        if let Some(name) = &query.name {
            let name = name.to_lowercase();
            out.retain(|i| i.name.to_lowercase().contains(&name));
        }

        // 按权重降序、再按名称排序，便于上层做加权发现
        out.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        });
        out.into_iter().cloned().collect()
    }

    /// 心跳租约回收：摘除所有超过租约未续约的实例
    ///
    /// 返回被摘除的实例 ID 列表。被动健康跟踪的核心：实例崩溃/失联不再心跳，
    /// 超过 `lease_seconds` 后即从注册中心移除（等价 Eureka/Nacos 的过期摘除语义）。
    pub fn reap_expired(&self) -> Vec<String> {
        self.reap_expired_at(chrono::Utc::now())
    }

    /// 可注入时间的回收（便于单测，不依赖真实时钟等待）
    pub fn reap_expired_at(&self, now: chrono::DateTime<chrono::Utc>) -> Vec<String> {
        let evicted: Vec<String> = {
            let mut map = self.inner.write();
            let mut evicted = Vec::new();
            map.retain(|id, inst| {
                if inst.is_expired_at(now) {
                    evicted.push(id.clone());
                    false
                } else {
                    true
                }
            });
            evicted
        };
        if !evicted.is_empty() {
            self.persist();
        }
        evicted
    }

    // ─── JSON 快照持久化 ──────────────────────────────────────────────────

    /// 原子写入快照（临时文件 + rename），参考 FileTaskRepository
    fn persist(&self) {
        let Some(path) = &self.snapshot else {
            return;
        };
        let instances: Vec<RegisteredInstance> = self.inner.read().values().cloned().collect();
        let raw = match serde_json::to_vec_pretty(&instances) {
            Ok(b) => b,
            Err(e) => {
                warn!("注册实例快照序列化失败：{e}");
                return;
            }
        };
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!("创建快照目录 {} 失败：{e}", parent.display());
                return;
            }
        }
        let tmp = path.with_extension("json.tmp");
        if let Err(e) = std::fs::write(&tmp, &raw) {
            warn!("写入临时快照 {} 失败：{e}", tmp.display());
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, path) {
            warn!("原子替换快照 {} 失败：{e}", path.display());
        }
    }

    /// 启动时加载已有快照
    fn load_snapshot(&self) {
        let Some(path) = &self.snapshot else {
            return;
        };
        if !path.exists() {
            return;
        }
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                warn!("读取快照 {} 失败：{e}", path.display());
                return;
            }
        };
        let instances: Vec<RegisteredInstance> = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                warn!("解析快照 {} 失败：{e}", path.display());
                return;
            }
        };
        let mut map = self.inner.write();
        for inst in instances {
            map.insert(inst.id.clone(), inst);
        }
        tracing::info!("注册中心从 {} 恢复 {} 个实例", path.display(), map.len());
    }

    /// 快照路径（观测/测试用）
    pub fn snapshot_path(&self) -> Option<&Path> {
        self.snapshot.as_deref()
    }
}

/// 把 lowercase 状态字符串解析为 [`InstanceStatus`]，无法识别返回 None
fn status_matches(actual: InstanceStatus, expected: &str) -> bool {
    let exp = expected.to_lowercase();
    let want = match exp.as_str() {
        "active" => InstanceStatus::Active,
        "unhealthy" => InstanceStatus::Unhealthy,
        "draining" => InstanceStatus::Draining,
        "expired" => InstanceStatus::Expired,
        _ => return false,
    };
    actual == want
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{HeartbeatRequest, RegisterRequest};
    use chrono::Duration;

    fn sample_register(id: &str, domain: &str) -> RegisteredInstance {
        let req = RegisterRequest {
            id: Some(id.to_string()),
            name: format!("expert-{domain}"),
            version: "1.0.0".to_string(),
            endpoint: format!("http://127.0.0.1:9000/{id}"),
            health_check_url: None,
            capabilities: vec![domain.to_string(), "general".to_string()],
            domain: Some(domain.to_string()),
            weight: 1.0,
            load_current: 0,
            load_capacity: 100,
            lease_seconds: 15,
            metadata: Default::default(),
        };
        let now = chrono::Utc::now();
        RegisteredInstance {
            id: req.id.unwrap(),
            name: req.name,
            version: req.version,
            endpoint: req.endpoint,
            health_check_url: req.health_check_url,
            capabilities: req.capabilities,
            domain: req.domain,
            weight: req.weight,
            load_current: req.load_current,
            load_capacity: req.load_capacity,
            status: InstanceStatus::Active,
            registered_at: now,
            last_heartbeat_at: now,
            lease_seconds: req.lease_seconds,
            metadata: req.metadata,
        }
    }

    #[test]
    fn register_get_roundtrip() {
        let store = RegistryStore::new_memory();
        store.register(sample_register("inst-1", "code"));
        let got = store.get("inst-1").expect("应已注册");
        assert_eq!(got.name, "expert-code");
        assert_eq!(got.status, InstanceStatus::Active);
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn deregister_removes_instance() {
        let store = RegistryStore::new_memory();
        store.register(sample_register("inst-2", "math"));
        let removed = store.deregister("inst-2");
        assert!(removed.is_some());
        assert!(store.get("inst-2").is_none());
        assert_eq!(store.count(), 0);
        // 重复注销返回 None
        assert!(store.deregister("inst-2").is_none());
    }

    #[test]
    fn filter_by_capability_and_domain() {
        let store = RegistryStore::new_memory();
        store.register(sample_register("a", "code"));
        store.register(sample_register("b", "math"));
        store.register(sample_register("c", "medical"));

        let code = store.list(&InstanceQuery {
            capability: Some("code".to_string()),
            ..Default::default()
        });
        assert_eq!(code.len(), 1);
        assert_eq!(code[0].id, "a");

        let math = store.list(&InstanceQuery {
            domain: Some("math".to_string()),
            ..Default::default()
        });
        assert_eq!(math.len(), 1);
        assert_eq!(math[0].id, "b");
    }

    #[test]
    fn heartbeat_refreshes_lease_and_recovers_unhealthy() {
        let store = RegistryStore::new_memory();
        store.register(sample_register("h", "code"));
        store.set_status("h", InstanceStatus::Unhealthy);

        let hb = HeartbeatRequest {
            load_current: Some(5),
            status: None,
        };
        let updated = store.heartbeat("h", hb.load_current, hb.status).unwrap();
        assert_eq!(updated.load_current, 5);
        // 心跳到达后从 Unhealthy 恢复为 Active
        assert_eq!(updated.status, InstanceStatus::Active);

        // 未注册实例心跳返回 None
        assert!(store.heartbeat("nope", None, None).is_none());
    }

    #[test]
    fn reap_expired_evicts_stale_instances() {
        let store = RegistryStore::new_memory();
        store.register(sample_register("fresh", "code"));

        // 构造一个心跳时间很旧的实例
        let mut stale = sample_register("stale", "math");
        stale.last_heartbeat_at = chrono::Utc::now() - Duration::seconds(60);
        store.register(stale);

        let now = chrono::Utc::now();
        let evicted = store.reap_expired_at(now);
        assert_eq!(evicted, vec!["stale".to_string()]);
        assert!(store.get("stale").is_none());
        assert!(store.get("fresh").is_some());
    }

    #[test]
    fn discover_excludes_non_discoverable_by_default() {
        let store = RegistryStore::new_memory();
        store.register(sample_register("ok", "code"));
        let mut down = sample_register("down", "math");
        down.status = InstanceStatus::Unhealthy;
        store.register(down);

        // 不带 status 过滤：只返回可发现（Active/Draining）
        let list = store.list(&InstanceQuery::default());
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "ok");

        // 显式 status=unhealthy 可查到
        let unhealthy = store.list(&InstanceQuery {
            status: Some("unhealthy".to_string()),
            ..Default::default()
        });
        assert_eq!(unhealthy.len(), 1);
        assert_eq!(unhealthy[0].id, "down");
    }

    #[test]
    fn file_snapshot_persists_across_instances() {
        let dir = std::env::temp_dir().join(format!(
            "registry_snap_test_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("instances.json");

        {
            let store = RegistryStore::with_snapshot(&path);
            store.register(sample_register("p1", "code"));
            store.register(sample_register("p2", "math"));
            assert!(path.exists());
        }

        // 新实例加载快照，应恢复两个实例
        let loaded = RegistryStore::with_snapshot(&path);
        assert_eq!(loaded.count(), 2);
        assert!(loaded.get("p1").is_some());
        assert!(loaded.get("p2").is_some());

        // 注销后再开，应只剩一个
        loaded.deregister("p1");
        drop(loaded);
        let reloaded = RegistryStore::with_snapshot(&path);
        assert_eq!(reloaded.count(), 1);
        assert!(reloaded.get("p2").is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_string_roundtrip() {
        assert!(status_matches(InstanceStatus::Active, "active"));
        assert!(status_matches(InstanceStatus::Unhealthy, "unhealthy"));
        assert!(!status_matches(InstanceStatus::Active, "bogus"));
    }
}
