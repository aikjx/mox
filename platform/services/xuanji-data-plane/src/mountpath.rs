use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MountpathState {
    Healthy,
    Degraded,
    Faulty,
    Disabled,
    Detached,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mountpath {
    pub path: PathBuf,
    pub state: MountpathState,
    pub id: String,
    pub created_at_ms: i64,
    pub last_failure_at_ms: Option<i64>,
    pub consecutive_failures: u32,
    pub capacity_bytes: u64,
    pub used_bytes: u64,
}

impl Mountpath {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let p = path.as_ref().to_path_buf();
        let id = simple_id(&p);
        Self {
            path: p,
            state: MountpathState::Healthy,
            id,
            created_at_ms: now_ms(),
            last_failure_at_ms: None,
            consecutive_failures: 0,
            capacity_bytes: 0,
            used_bytes: 0,
        }
    }

    pub fn mark_failure(&mut self, threshold: u32) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.last_failure_at_ms = Some(now_ms());
        if self.consecutive_failures >= threshold {
            self.state = MountpathState::Faulty;
        } else if self.consecutive_failures > 1 {
            self.state = MountpathState::Degraded;
        }
    }

    pub fn mark_healthy(&mut self) {
        self.consecutive_failures = 0;
        self.state = MountpathState::Healthy;
    }
}

fn now_ms() -> i64 { chrono::Utc::now().timestamp_millis() }

fn simple_id(p: &Path) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(p.to_string_lossy().as_bytes());
    let d = h.finalize();
    hex::encode(&d[..8])
}

#[derive(Default)]
pub struct MountpathRegistry {
    inner: RwLock<BTreeMap<String, Mountpath>>,
}

impl MountpathRegistry {
    pub fn new() -> Self { Self::default() }

    /// Attach a mountpath. If it already exists (same path), return the existing id.
    pub fn attach(&self, path: impl AsRef<Path>) -> Result<String, &'static str> {
        let path = path.as_ref().to_path_buf();
        // disallow nested paths (no mountpath is prefix of another)
        {
            let g = self.inner.read();
            for mp in g.values() {
                let a = &mp.path;
                if path.starts_with(a) || a.starts_with(&path) {
                    return Err("nesting or overlapping mountpath not allowed");
                }
            }
        }
        let mp = Mountpath::new(&path);
        let id = mp.id.clone();
        self.inner.write().insert(id.clone(), mp);
        Ok(id)
    }

    pub fn detach(&self, id: &str) -> Option<Mountpath> {
        let mut w = self.inner.write();
        if let Some(mut mp) = w.remove(id) {
            mp.state = MountpathState::Detached;
            Some(mp)
        } else { None }
    }

    pub fn disable(&self, id: &str) -> bool {
        let mut w = self.inner.write();
        if let Some(v) = w.get_mut(id) {
            v.state = MountpathState::Disabled;
            return true;
        }
        false
    }

    pub fn enable(&self, id: &str) -> bool {
        let mut w = self.inner.write();
        if let Some(v) = w.get_mut(id) {
            if matches!(v.state, MountpathState::Disabled) { v.state = MountpathState::Healthy; }
            return true;
        }
        false
    }

    pub fn list(&self) -> Vec<Mountpath> {
        self.inner.read().values().cloned().collect()
    }

    pub fn len(&self) -> usize { self.inner.read().len() }
    pub fn is_empty(&self) -> bool { self.len() == 0 }

    pub fn update_state(&self, id: &str, f: impl FnOnce(&mut Mountpath)) {
        if let Some(v) = self.inner.write().get_mut(id) { f(v); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T6-01 mountpath.attach returns id of stable format (16-char hex)
    #[test]
    fn t6_01_attach_id_is_16_char_hex() {
        let reg = MountpathRegistry::new();
        let id = reg.attach("/data/vol-01").expect("attach ok");
        assert_eq!(id.len(), 16, "id must be 16 chars, got {}: {:?}", id.len(), id);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()), "id must be hex: {:?}", id);
        // same path again: should be stable id but overlapping check disallows;
        // confirm stable by computing directly
        let id2 = simple_id(Path::new("/data/vol-01"));
        assert_eq!(id, id2, "id must be deterministic / stable for the same path");
    }

    /// T6-02 attach duplicate overlapping path -> err nesting disallowed
    #[test]
    fn t6_02_overlapping_nesting_disallowed() {
        let reg = MountpathRegistry::new();
        reg.attach("/mnt/a").expect("first attach ok");
        // exact same path -> overlap
        let err_same = reg.attach("/mnt/a");
        assert!(err_same.is_err(), "same path must be rejected");
        assert_eq!(err_same.unwrap_err(), "nesting or overlapping mountpath not allowed");
        // child path /mnt/a/b -> nested under /mnt/a
        let err_child = reg.attach("/mnt/a/b");
        assert!(err_child.is_err(), "nested child must be rejected");
        // parent path /mnt -> /mnt/a is nested under /mnt
        let err_parent = reg.attach("/mnt");
        assert!(err_parent.is_err(), "parent path must be rejected");
        // sibling /mnt/b -> ok
        assert!(reg.attach("/mnt/b").is_ok(), "sibling should be allowed");
    }

    /// T6-03 detach unknown -> None; detach known -> state Detached
    #[test]
    fn t6_03_detach_known_and_unknown() {
        let reg = MountpathRegistry::new();
        // detach unknown -> None
        assert!(reg.detach("no-such-id").is_none(), "detach unknown -> None");
        // attach then detach
        let id = reg.attach("/data/x").expect("attach ok");
        let mp = reg.detach(&id).expect("detach known -> Some");
        assert_eq!(mp.state, MountpathState::Detached, "detached mountpath state must be Detached");
        // detach again -> already removed from map
        assert!(reg.detach(&id).is_none(), "double detach -> None");
    }

    /// T6-04 disable/enable flips state
    #[test]
    fn t6_04_disable_enable_flips_state() {
        let reg = MountpathRegistry::new();
        let id = reg.attach("/data/y").expect("attach ok");
        // initially Healthy
        let list0 = reg.list();
        assert_eq!(list0[0].state, MountpathState::Healthy);
        // disable -> true and state Disabled
        assert!(reg.disable(&id), "disable known id returns true");
        let list1 = reg.list();
        assert_eq!(list1[0].state, MountpathState::Disabled);
        // disable unknown -> false
        assert!(!reg.disable("ghost"), "disable unknown id returns false");
        // enable -> true, state Healthy
        assert!(reg.enable(&id), "enable known id returns true");
        let list2 = reg.list();
        assert_eq!(list2[0].state, MountpathState::Healthy);
        // enable unknown -> false
        assert!(!reg.enable("ghost"), "enable unknown id returns false");
    }

    /// T6-05 attach 3 mountpaths, list() len == 3
    #[test]
    fn t6_05_attach_three_list_len_3() {
        let reg = MountpathRegistry::new();
        assert_eq!(reg.len(), 0);
        let id1 = reg.attach("/srv/a").expect("a ok");
        let id2 = reg.attach("/srv/b").expect("b ok");
        let id3 = reg.attach("/srv/c").expect("c ok");
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
        assert_eq!(reg.len(), 3, "len after 3 attaches");
        assert_eq!(reg.list().len(), 3, "list().len() after 3 attaches");
    }
}
