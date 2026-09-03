// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Restore Async Task State Machine
//!
//! Lightweight, std-only (Arc+Mutex) task queue that tracks per-object
//! Glacier restore operations as they progress through Queued → InProgress →
//! Available → Expired (or Failed).  Designed for tests and internal state
//! management of an S3-compatible service.

use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Expedited,
    Standard,
    Bulk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreState {
    Queued,
    InProgress,
    Available { available_at_ms: u64, expires_at_ms: u64 },
    Expired,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct RestoreTask {
    pub id: String,
    pub bucket: String,
    pub key: String,
    pub tier: Tier,
    pub queued_at_ms: u64,
    pub eta_ms: u64,
    pub state: RestoreState,
}

// Tier ETA constants (milliseconds).
pub const TIER_EXPEDITED_ETA_MS: u64 = 120_000;
pub const TIER_STANDARD_ETA_MS: u64 = 4 * 3_600_000;
pub const TIER_BULK_ETA_MS: u64 = 8 * 3_600_000;
pub const ONE_DAY_MS: u64 = 86_400_000;

fn tier_eta_ms(tier: Tier) -> u64 {
    match tier {
        Tier::Expedited => TIER_EXPEDITED_ETA_MS,
        Tier::Standard => TIER_STANDARD_ETA_MS,
        Tier::Bulk => TIER_BULK_ETA_MS,
    }
}

fn new_task_id(counter: &AtomicU64) -> String {
    let n = counter.fetch_add(1, Ordering::SeqCst);
    format!("rt-{}", n)
}

pub struct RestoreQueue {
    tasks: Vec<RestoreTask>,
    counter: AtomicU64,
}

impl Default for RestoreQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl RestoreQueue {
    pub fn new() -> Self {
        Self { tasks: Vec::new(), counter: AtomicU64::new(1) }
    }

    /// Enqueue a new restore task.  State starts as Queued.  Returns the new
    /// task id.
    pub fn enqueue(&mut self, bucket: &str, key: &str, tier: Tier, now_ms: u64) -> String {
        let id = new_task_id(&self.counter);
        let task = RestoreTask {
            id: id.clone(),
            bucket: bucket.to_string(),
            key: key.to_string(),
            tier,
            queued_at_ms: now_ms,
            eta_ms: now_ms + tier_eta_ms(tier),
            state: RestoreState::Queued,
        };
        self.tasks.push(task);
        id
    }

    /// Advance the state machine for every task based on the given wall clock.
    ///
    /// Transition rules:
    /// * Queued → InProgress (eta already computed at enqueue time)
    /// * InProgress → Available when now_ms >= eta_ms (1-day availability)
    /// * Available → Expired when now_ms >= expires_at_ms
    pub fn tick(&mut self, now_ms: u64) {
        for t in self.tasks.iter_mut() {
            match t.state {
                RestoreState::Queued => {
                    t.state = RestoreState::InProgress;
                },
                RestoreState::InProgress => {
                    if now_ms >= t.eta_ms {
                        t.state = RestoreState::Available {
                            available_at_ms: now_ms,
                            expires_at_ms: now_ms + ONE_DAY_MS,
                        };
                    }
                },
                RestoreState::Available { expires_at_ms, .. } => {
                    if now_ms >= expires_at_ms {
                        t.state = RestoreState::Expired;
                    }
                },
                RestoreState::Expired | RestoreState::Failed(_) => {},
            }
        }
    }

    pub fn list(&self) -> Vec<RestoreTask> {
        self.tasks.clone()
    }

    pub fn get(&self, id: &str) -> Option<RestoreTask> {
        self.tasks.iter().find(|t| t.id == id).cloned()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{Arc, Mutex},
        thread,
    };

    // --- B1: 3 tiers, tick and ETA transitions ---
    #[test]
    fn t25_restore_3_tiers_eta() {
        let mut q = RestoreQueue::new();
        let now = 1000u64;
        let e_id = q.enqueue("b", "ke", Tier::Expedited, now);
        let s_id = q.enqueue("b", "ks", Tier::Standard, now);
        let b_id = q.enqueue("b", "kb", Tier::Bulk, now);

        q.tick(now);
        assert_eq!(q.get(&e_id).unwrap().state, RestoreState::InProgress);
        assert_eq!(q.get(&s_id).unwrap().state, RestoreState::InProgress);
        assert_eq!(q.get(&b_id).unwrap().state, RestoreState::InProgress);

        // push time past Expedited ETA
        let expedited_done = now + TIER_EXPEDITED_ETA_MS + 1;
        q.tick(expedited_done);
        match q.get(&e_id).unwrap().state {
            RestoreState::Available { expires_at_ms, .. } => {
                assert_eq!(expires_at_ms, expedited_done + ONE_DAY_MS);
            },
            other => panic!("Expedited should be Available, got {:?}", other),
        }
        // Standard & Bulk still InProgress
        assert_eq!(q.get(&s_id).unwrap().state, RestoreState::InProgress);
        assert_eq!(q.get(&b_id).unwrap().state, RestoreState::InProgress);
    }

    // --- B2: Available expires after 1 day ---
    #[test]
    fn t25_restore_available_expires_after_1day() {
        let mut q = RestoreQueue::new();
        let id = q.enqueue("b", "k", Tier::Expedited, 0);
        q.tick(0);
        // push immediately to Available
        q.tick(TIER_EXPEDITED_ETA_MS + 1);
        match q.get(&id).unwrap().state {
            RestoreState::Available { expires_at_ms, .. } => {
                assert_eq!(expires_at_ms, (TIER_EXPEDITED_ETA_MS + 1) + ONE_DAY_MS);
            },
            other => panic!("expected Available, got {:?}", other),
        }
        // tick exactly one day + 1 after available
        let day_plus_one = (TIER_EXPEDITED_ETA_MS + 1) + ONE_DAY_MS + 1;
        q.tick(day_plus_one);
        assert_eq!(q.get(&id).unwrap().state, RestoreState::Expired);
    }

    // --- B3: enqueue N, list len is N ---
    #[test]
    fn t25_restore_list_count_eq_enqueue() {
        let mut q = RestoreQueue::new();
        const N: usize = 17;
        for i in 0..N {
            let tier = match i % 3 {
                0 => Tier::Expedited,
                1 => Tier::Standard,
                _ => Tier::Bulk,
            };
            q.enqueue("b", &format!("k{}", i), tier, i as u64);
        }
        assert_eq!(q.list().len(), N);
    }

    // --- B4: concurrent 10 threads * 10 enqueues = 100 tasks, no deadlock ---
    #[test]
    fn t25_restore_concurrent_100() {
        let q = Arc::new(Mutex::new(RestoreQueue::new()));
        let mut handles = Vec::new();
        for t in 0..10 {
            let qc = Arc::clone(&q);
            handles.push(thread::spawn(move || {
                for i in 0..10 {
                    let tier = match (t + i) % 3 {
                        0 => Tier::Expedited,
                        1 => Tier::Standard,
                        _ => Tier::Bulk,
                    };
                    let mut g = qc.lock().unwrap();
                    let now = (t as u64) * 100 + (i as u64);
                    g.enqueue("bucket", &format!("t{}i{}", t, i), tier, now);
                    // tick a few times to keep state machine exercised
                    g.tick(now);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        // Extra tick rounds to verify no deadlock.
        for round in 0..10 {
            let mut g = q.lock().unwrap();
            g.tick(round * 1_000_000);
        }

        let g = q.lock().unwrap();
        assert_eq!(g.list().len(), 100);
    }
}
