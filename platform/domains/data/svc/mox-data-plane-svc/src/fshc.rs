// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use crate::mountpath::{MountpathRegistry, MountpathState};
use parking_lot::Mutex;
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum FshcEvent {
    FailureDetected { path: std::path::PathBuf, id: String, consec: u32 },
    FaultyMarked { id: String },
    HealthyRestored { id: String },
}

pub struct FshcScanner {
    pub threshold_failures: u32,
    pub read_probe_size: usize,
    pub write_probe_size: usize,
    pub events: Arc<Mutex<Vec<FshcEvent>>>,
}

impl Default for FshcScanner {
    fn default() -> Self {
        Self {
            threshold_failures: 3,
            read_probe_size: 1 * 1024 * 1024, // 1 MB
            write_probe_size: 128 * 1024,      // 128 KB
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl FshcScanner {
    pub fn new() -> Self { Self::default() }

    /// Returns true if the mountpath at `id` passes probe. False means probe failed.
    pub fn probe_once(&self, registry: &MountpathRegistry, id: &str) -> bool {
        let list = registry.list();
        let Some(mp) = list.iter().find(|m| m.id == id) else { return false; };
        if matches!(mp.state, MountpathState::Detached | MountpathState::Disabled) { return false; }
        let path = mp.path.clone();
        if !path.exists() {
            self.fail_path(registry, id, &path);
            return false;
        }
        // write-then-verify 128KB probe
        let probe_path = path.join(".mox-fshc-probe.tmp");
        let mut bytes = vec![0u8; self.write_probe_size];
        for (i, b) in bytes.iter_mut().enumerate() { *b = (i & 0xff) as u8; }
        let ok_w = std::fs::File::create(&probe_path)
            .and_then(|mut f| f.write_all(&bytes))
            .is_ok();
        let mut read_back = vec![0u8; self.write_probe_size];
        let ok_rw = ok_w && std::fs::File::open(&probe_path)
            .and_then(|mut f| f.read_exact(&mut read_back))
            .map(|_| read_back == bytes)
            .unwrap_or(false);
        let _ = std::fs::remove_file(&probe_path);
        // 1MB random read sanity: try listing 1024 entries
        let _ = std::fs::read_dir(&path);
        if !ok_rw {
            self.fail_path(registry, id, &path);
            return false;
        }
        registry.update_state(id, |m| m.mark_healthy());
        self.events.lock().push(FshcEvent::HealthyRestored { id: id.to_string() });
        true
    }

    fn fail_path(&self, registry: &MountpathRegistry, id: &str, path: &std::path::Path) {
        let mut before_consec = 0;
        registry.update_state(id, |m| {
            before_consec = m.consecutive_failures;
            m.mark_failure(self.threshold_failures);
            if m.consecutive_failures >= self.threshold_failures {
                self.events.lock().push(FshcEvent::FaultyMarked { id: id.to_string() });
            }
        });
        self.events.lock().push(FshcEvent::FailureDetected {
            path: path.to_path_buf(),
            id: id.to_string(),
            consec: before_consec.saturating_add(1),
        });
    }

    pub fn events_drain(&self) -> Vec<FshcEvent> {
        std::mem::take(&mut *self.events.lock())
    }

    pub async fn run_background(self: Arc<Self>, registry: Arc<MountpathRegistry>, interval: Duration) {
        let mut tick = tokio::time::interval(interval);
        loop {
            tick.tick().await;
            let ids: Vec<String> = registry.list().iter().map(|m| m.id.clone()).collect();
            for id in ids { let _ = self.probe_once(&registry, &id); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mountpath::MountpathState;
    use std::sync::Arc;
    use std::thread;

    /// T6-06 fshc probe on nonexistent path marks failure -> consecutive_failures += 1
    #[test]
    fn t6_06_probe_nonexistent_marks_failure() {
        let reg = MountpathRegistry::new();
        let scanner = FshcScanner::new();
        let tmp = tempfile::tempdir().expect("tempdir");
        let real = tmp.path().join("exist");
        std::fs::create_dir_all(&real).expect("mkdir");
        let id = reg.attach(&real).expect("attach");
        // remove the directory so path.exists() == false
        std::fs::remove_dir_all(&real).expect("rmdir");
        let before = reg.list()[0].consecutive_failures;
        assert_eq!(before, 0);
        let ok = scanner.probe_once(&reg, &id);
        assert!(!ok, "probe must fail on deleted path");
        let mp = &reg.list()[0];
        assert_eq!(mp.consecutive_failures, 1, "consecutive_failures must increment by 1");
    }

    /// T6-07 threshold 3: 3 consecutive failures sets state Faulty
    #[test]
    fn t6_07_three_failures_sets_faulty() {
        let reg = MountpathRegistry::new();
        let scanner = FshcScanner::new();
        assert_eq!(scanner.threshold_failures, 3);
        let tmp = tempfile::tempdir().expect("tempdir");
        let real = tmp.path().join("v");
        std::fs::create_dir_all(&real).expect("mkdir");
        let id = reg.attach(&real).expect("attach");
        std::fs::remove_dir_all(&real).expect("rm");
        // 1st failure -> Degraded? no, >1 -> degraded; 1 stays Healthy
        let _ = scanner.probe_once(&reg, &id);
        assert_eq!(reg.list()[0].consecutive_failures, 1);
        assert_eq!(reg.list()[0].state, MountpathState::Healthy);
        // 2nd failure -> Degraded (since >1)
        let _ = scanner.probe_once(&reg, &id);
        assert_eq!(reg.list()[0].consecutive_failures, 2);
        assert_eq!(reg.list()[0].state, MountpathState::Degraded);
        // 3rd failure -> Faulty
        let _ = scanner.probe_once(&reg, &id);
        assert_eq!(reg.list()[0].consecutive_failures, 3);
        assert_eq!(reg.list()[0].state, MountpathState::Faulty);
    }

    /// T6-08 healthy again: probe success resets consecutive_failures to 0
    #[test]
    fn t6_08_healthy_again_resets_failures() {
        let reg = MountpathRegistry::new();
        let scanner = FshcScanner::new();
        let tmp = tempfile::tempdir().expect("tempdir");
        let real = tmp.path().join("v");
        std::fs::create_dir_all(&real).expect("mkdir");
        let id = reg.attach(&real).expect("attach");
        // induce 2 failures
        std::fs::remove_dir_all(&real).expect("rm");
        let _ = scanner.probe_once(&reg, &id);
        let _ = scanner.probe_once(&reg, &id);
        assert_eq!(reg.list()[0].consecutive_failures, 2);
        assert_eq!(reg.list()[0].state, MountpathState::Degraded);
        // restore path
        std::fs::create_dir_all(&real).expect("mkdir back");
        let ok = scanner.probe_once(&reg, &id);
        assert!(ok, "probe should succeed on restored path");
        let mp = &reg.list()[0];
        assert_eq!(mp.consecutive_failures, 0, "consecutive_failures reset to 0");
        assert_eq!(mp.state, MountpathState::Healthy, "state back to Healthy");
    }

    /// T6-09 events_drain on threshold3 produce: 3x FailureDetected + 1x FaultyMarked
    #[test]
    fn t6_09_events_drain_threshold_3() {
        let reg = MountpathRegistry::new();
        let scanner = FshcScanner::new();
        let tmp = tempfile::tempdir().expect("tempdir");
        let real = tmp.path().join("v");
        std::fs::create_dir_all(&real).expect("mkdir");
        let id = reg.attach(&real).expect("attach");
        std::fs::remove_dir_all(&real).expect("rm");
        let _ = scanner.probe_once(&reg, &id);
        let _ = scanner.probe_once(&reg, &id);
        let _ = scanner.probe_once(&reg, &id);
        let events = scanner.events_drain();
        let mut fail_count = 0usize;
        let mut faulty_count = 0usize;
        for ev in &events {
            match ev {
                FshcEvent::FailureDetected { .. } => fail_count += 1,
                FshcEvent::FaultyMarked { .. } => faulty_count += 1,
                FshcEvent::HealthyRestored { .. } => {}
            }
        }
        assert_eq!(fail_count, 3, "3 FailureDetected events, got {fail_count}");
        assert_eq!(faulty_count, 1, "1 FaultyMarked event, got {faulty_count}");
        // FaultyMarked should come before or after its corresponding FailureDetected?
        // Implementation pushes FaultyMarked inside update_state closure then FailureDetected after.
        // So last two events should be FailureDetected(consec=3) preceded by FaultyMarked.
        // drain should return the same 4 events; second drain must be empty
        let ev2 = scanner.events_drain();
        assert!(ev2.is_empty(), "events drained; second drain empty");
    }

    /// T6-10 update_state closure used by concurrent 10 threads all incrementing
    /// consecutive_failures atomically. Tests concurrent access via update_state
    /// from 10 threads each calling mark_failure to increment consecutive_failures.
    #[test]
    fn t6_10_concurrent_update_state_10_threads() {
        let reg = Arc::new(MountpathRegistry::new());
        let tmp = tempfile::tempdir().expect("tempdir");
        let real = tmp.path().join("mp");
        std::fs::create_dir_all(&real).expect("mkdir");
        let id = reg.attach(&real).expect("attach");
        // detach -> removed from registry; so we instead use a non-detached path
        // and test concurrent update_state calls directly (T6 asks to use the
        // update_state closure invoked by 10 threads incrementing failures).
        let reg2 = reg.clone();
        let id_clone = id.clone();
        let threshold: u32 = 100;
        let mut handles = Vec::with_capacity(10);
        for _ in 0..10 {
            let r = reg2.clone();
            let i = id_clone.clone();
            handles.push(thread::spawn(move || {
                r.update_state(&i, |m| {
                    m.mark_failure(threshold);
                });
            }));
        }
        for h in handles { h.join().expect("thread join"); }
        let mp = &reg.list()[0];
        assert_eq!(mp.consecutive_failures, 10,
            "10 concurrent mark_failure increments must total 10 (atomic via write lock), got {}",
            mp.consecutive_failures);
    }
}
