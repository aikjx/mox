// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Hot Vertex LRU Cache：容量 100k vid，eviction LRU。
//!
//! 公开 API：get / insert / len / misses / total_calls。
//!
//! 实现：HashMap<K, (V, u64)> + VecDeque<(K, u64)>；lru_clock 为单调计数，
//! get 命中时追加 (key, new_clock) 到尾部（key 可能多次出现，evict 时跳过已过期）。

use parking_lot::Mutex;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct LruCache<K: Eq + Hash + Clone, V: Clone> {
    cap: usize,
    map: HashMap<K, (V, u64)>,
    order: VecDeque<(K, u64)>,
    clock: u64,
    total_calls: AtomicU64,
    misses: AtomicU64,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn new(cap: usize) -> Self {
        let cap = cap.max(1);
        Self {
            cap,
            map: HashMap::with_capacity(cap),
            order: VecDeque::with_capacity(cap),
            clock: 0,
            total_calls: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }
    pub fn total_calls(&self) -> u64 {
        self.total_calls.load(Ordering::SeqCst)
    }
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::SeqCst)
    }
    pub fn hit_rate(&self) -> f64 {
        let t = self.total_calls.load(Ordering::SeqCst);
        if t == 0 {
            return 0.0;
        }
        let m = self.misses.load(Ordering::SeqCst);
        (t - m) as f64 / t as f64
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    fn next_clock(&mut self) -> u64 {
        self.clock = self.clock.wrapping_add(1);
        self.clock
    }

    pub fn get(&mut self, k: &K) -> Option<V> {
        self.total_calls.fetch_add(1, Ordering::SeqCst);
        if let Some((v, _old_tick)) = self.map.get(k).cloned() {
            let t = self.next_clock();
            // 更新 entry tick
            // Safety: we just copied V, so we can mut self again.
            if let Some(entry) = self.map.get_mut(k) {
                entry.1 = t;
            }
            self.order.push_back((k.clone(), t));
            Some(v)
        } else {
            self.misses.fetch_add(1, Ordering::SeqCst);
            None
        }
    }

    pub fn insert(&mut self, k: K, v: V) {
        let t = self.next_clock();
        if let std::collections::hash_map::Entry::Occupied(mut o) = self.map.entry(k.clone()) {
            o.insert((v, t));
        } else {
            while self.map.len() >= self.cap {
                self.evict_one();
            }
            self.map.insert(k.clone(), (v, t));
        }
        self.order.push_back((k, t));
    }

    fn evict_one(&mut self) {
        while let Some((key, tick)) = self.order.pop_front() {
            if let Some(entry_tick) = self.map.get(&key).map(|x| x.1) {
                if entry_tick == tick {
                    self.map.remove(&key);
                    return;
                }
            }
        }
    }

    pub fn invalidate(&mut self, k: &K) {
        self.map.remove(k);
    }
}

/// Neighbor 结构体（Storage 7 API 使用）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neighbor {
    pub neighbor_vid: String,
    pub direction: String,
    pub etype: String,
    pub rank: i64,
    pub weight: Option<i64>,
    pub props: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub src: String,
    pub dst: String,
    pub etype: String,
    pub rank: i64,
    pub weight: Option<i64>,
    pub props: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexAck {
    pub vid: String,
    pub tag: String,
    pub shard: u16,
    pub applied_index: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeAck {
    pub src: String,
    pub dst: String,
    pub etype: String,
    pub rank: i64,
    pub shard: u16,
    pub applied_index: u64,
}

pub use crate::storage_engine::Direction;

pub fn weight_to_i64(w: Option<f64>) -> Option<i64> {
    w.map(|x| (x * 1_000_000_000.0) as i64)
}

pub struct HotNeighborCache {
    pub inner: Mutex<LruCache<String, Vec<Neighbor>>>,
}
impl HotNeighborCache {
    pub fn new(cap: usize) -> Self {
        Self {
            inner: Mutex::new(LruCache::new(cap)),
        }
    }
    pub fn get(&self, vid: &str) -> Option<Vec<Neighbor>> {
        let owned = String::from(vid);
        self.inner.lock().get(&owned)
    }
    pub fn insert(&self, vid: &str, ns: Vec<Neighbor>) {
        self.inner.lock().insert(String::from(vid), ns);
    }
    pub fn invalidate(&self, vid: &str) {
        let key = String::from(vid);
        self.inner.lock().invalidate(&key);
    }
    pub fn total_calls(&self) -> u64 {
        self.inner.lock().total_calls()
    }
    pub fn misses(&self) -> u64 {
        self.inner.lock().misses()
    }
    pub fn hit_rate(&self) -> f64 {
        self.inner.lock().hit_rate()
    }
    pub fn contains(&self, vid: &str) -> bool {
        self.inner.lock().contains_key(&String::from(vid))
    }
}
impl<K: Eq + std::hash::Hash + Clone, V: Clone> LruCache<K, V> {
    pub fn contains_key(&self, k: &K) -> bool {
        self.map.contains_key(k)
    }
}
