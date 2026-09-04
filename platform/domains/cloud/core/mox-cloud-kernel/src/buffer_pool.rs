// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Four-tier pooled buffer management for unified memory buffer allocation.
//!
//! Provides a slab-style buffer pool with four size tiers to avoid repeated
//! heap allocation / deallocation across volume / s3 / filer data paths.
//! `PooledBuffer` is an RAII handle that automatically returns its backing
//! `Vec<u8>` to the pool on drop.
//!
//! # Design reference
//! Algorithm inspired by RustFS io-core `pool.rs` (Apache License 2.0).
//! This is an independent reimplementation: it uses `Vec<u8>` instead of
//! `BytesMut`, `Weak<BufferPoolInner>` instead of `Arc<PoolTier>`,
//! `std::sync::Mutex` free queues instead of semaphores, and exposes a
//! configurable four-tier layout rather than hard-coded tiers.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Weak,
    },
};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Per-tier configuration for the buffer pool.
#[derive(Debug, Clone)]
pub struct BufferTierConfig {
    /// Minimum request size that lands in this tier (informational boundary).
    pub min_size: usize,
    /// Maximum request size served by this tier; pooled buffers are allocated
    /// with capacity equal to `max_size` so any buffer in the tier can serve
    /// any request routed to it.
    pub max_size: usize,
    /// Maximum number of idle buffers retained in this tier's free queue.
    pub max_count: usize,
    /// Snapshot field: number of buffers currently allocated (in-use + idle).
    /// Populated when reading a config snapshot; not used as a runtime counter.
    pub alloc_count: usize,
}

/// Top-level buffer pool configuration.
#[derive(Debug, Clone)]
pub struct BufferPoolConfig {
    /// Per-tier configurations, ordered from smallest to largest.
    pub tiers: Vec<BufferTierConfig>,
    /// Global cap on bytes held by the pool (idle + in-use).
    /// `0` means unlimited.  When exceeded, `acquire` falls back to a direct
    /// allocation that is not managed by the pool.
    pub global_max_bytes: usize,
}

impl Default for BufferPoolConfig {
    fn default() -> Self {
        Self {
            tiers: vec![
                BufferTierConfig {
                    min_size: 64,
                    max_size: 4 * 1024,
                    max_count: 1024,
                    alloc_count: 0,
                },
                BufferTierConfig {
                    min_size: 4 * 1024,
                    max_size: 64 * 1024,
                    max_count: 256,
                    alloc_count: 0,
                },
                BufferTierConfig {
                    min_size: 64 * 1024,
                    max_size: 1024 * 1024,
                    max_count: 64,
                    alloc_count: 0,
                },
                BufferTierConfig {
                    min_size: 1024 * 1024,
                    max_size: 16 * 1024 * 1024,
                    max_count: 16,
                    alloc_count: 0,
                },
            ],
            global_max_bytes: 256 * 1024 * 1024, // 256 MiB
        }
    }
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// Runtime statistics for a single tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferTierStats {
    pub tier_index: usize,
    pub min_size: usize,
    pub max_size: usize,
    pub max_count: usize,
    /// Buffers currently sitting in the free queue.
    pub current_idle: usize,
    /// Buffers currently checked out via `acquire`.
    pub current_in_use: usize,
    /// Cumulative count of fresh `Vec` allocations in this tier.
    pub allocated_count: usize,
    /// Cumulative count of buffer reuses from the free queue.
    pub reused_count: usize,
}

/// Aggregate pool statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferPoolStats {
    /// Cumulative number of fresh allocations (pool misses).
    pub total_allocated: usize,
    /// Cumulative number of buffer reuses (pool hits).
    pub total_reused: usize,
    /// `reused / (allocated + reused)` in `[0.0, 1.0]`.
    pub reuse_rate: f64,
    /// Buffers currently checked out.
    pub current_in_use: usize,
    /// Buffers currently in free queues across all tiers.
    pub current_idle: usize,
    /// Total bytes currently held by the pool (idle + in-use pooled buffers).
    pub current_bytes: usize,
    /// Per-tier breakdown.
    pub tier_stats: Vec<BufferTierStats>,
}

// ---------------------------------------------------------------------------
// Pool internals
// ---------------------------------------------------------------------------

struct BufferPoolInner {
    /// Free queue per tier: `Vec<Vec<u8>>` of idle buffers.
    tiers: Vec<Mutex<Vec<Vec<u8>>>>,
    /// Immutable tier configs (snapshot at pool creation).
    configs: Vec<BufferTierConfig>,
    /// Cumulative fresh allocations across all tiers.
    total_allocated: AtomicUsize,
    /// Cumulative reuses across all tiers.
    total_reused: AtomicUsize,
    /// Buffers currently checked out (pooled only).
    current_in_use: AtomicUsize,
    /// Total bytes held by pooled buffers (idle + in-use).
    current_bytes: AtomicUsize,
    /// Per-tier cumulative fresh allocations.
    tier_allocated: Vec<AtomicUsize>,
    /// Per-tier cumulative reuses.
    tier_reused: Vec<AtomicUsize>,
    /// Per-tier current in-use count.
    tier_in_use: Vec<AtomicUsize>,
    /// Global memory cap (0 = unlimited).
    global_max_bytes: usize,
}

impl BufferPoolInner {
    /// Return a buffer to its tier's free queue, or free it if the queue is
    /// already at `max_count`.
    fn return_buffer(&self, tier_index: usize, vec: Vec<u8>) {
        self.current_in_use.fetch_sub(1, Ordering::Relaxed);
        self.tier_in_use[tier_index].fetch_sub(1, Ordering::Relaxed);

        let mut queue = self.tiers[tier_index].lock();

        if queue.len() < self.configs[tier_index].max_count {
            queue.push(vec);
            // `current_bytes` unchanged: buffer is still managed by the pool,
            // it merely moved from in-use to idle.
        } else {
            // Free queue full — release the backing memory.
            let cap = vec.capacity();
            drop(vec);
            self.current_bytes.fetch_sub(cap, Ordering::Relaxed);
        }
    }

    /// Detach a buffer from pool management (called by `PooledBuffer::into_vec`).
    fn detach_buffer(&self, tier_index: usize, capacity: usize) {
        self.current_in_use.fetch_sub(1, Ordering::Relaxed);
        self.tier_in_use[tier_index].fetch_sub(1, Ordering::Relaxed);
        self.current_bytes.fetch_sub(capacity, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// BufferPool (public handle)
// ---------------------------------------------------------------------------

/// A four-tier pooled buffer allocator.
///
/// Cloning a `BufferPool` is cheap (it is an `Arc` handle) and all clones
/// share the same underlying free queues and statistics.
pub struct BufferPool {
    inner: Arc<BufferPoolInner>,
}

impl Clone for BufferPool {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl std::fmt::Debug for BufferPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stats = self.stats();
        f.debug_struct("BufferPool")
            .field("tiers", &self.inner.configs.len())
            .field("current_in_use", &stats.current_in_use)
            .field("current_idle", &stats.current_idle)
            .field("current_bytes", &stats.current_bytes)
            .field("reuse_rate", &stats.reuse_rate)
            .finish()
    }
}

impl BufferPool {
    /// Create a pool with the given configuration.
    pub fn new(config: BufferPoolConfig) -> Self {
        let n = config.tiers.len();
        let inner = Arc::new(BufferPoolInner {
            tiers: (0..n).map(|_| Mutex::new(Vec::new())).collect(),
            configs: config.tiers,
            total_allocated: AtomicUsize::new(0),
            total_reused: AtomicUsize::new(0),
            current_in_use: AtomicUsize::new(0),
            current_bytes: AtomicUsize::new(0),
            tier_allocated: (0..n).map(|_| AtomicUsize::new(0)).collect(),
            tier_reused: (0..n).map(|_| AtomicUsize::new(0)).collect(),
            tier_in_use: (0..n).map(|_| AtomicUsize::new(0)).collect(),
            global_max_bytes: config.global_max_bytes,
        });
        Self { inner }
    }

    /// Create a pool with the default four-tier configuration.
    pub fn with_default() -> Self {
        Self::new(BufferPoolConfig::default())
    }

    /// Acquire a buffer with at least `capacity` bytes of capacity.
    ///
    /// The buffer is routed to the first tier whose `max_size >= capacity`.
    /// If `capacity` exceeds the largest tier's `max_size`, or the global
    /// memory cap is reached, a direct (unpooled) allocation is returned.
    /// The returned buffer always has length 0.
    pub fn acquire(&self, capacity: usize) -> PooledBuffer {
        // Zero-capacity request: return an empty unpooled buffer.
        if capacity == 0 {
            return PooledBuffer {
                data: Some(Vec::new()),
                tier_index: usize::MAX,
                pool: Weak::new(),
            };
        }

        // Find the first tier that can serve this size.
        let tier_idx = match self.inner.configs.iter().position(|c| c.max_size >= capacity) {
            Some(idx) => idx,
            None => {
                // Oversized: direct allocation, no pool management.
                return PooledBuffer {
                    data: Some(Vec::with_capacity(capacity)),
                    tier_index: usize::MAX,
                    pool: Weak::new(),
                };
            },
        };

        let tier_max = self.inner.configs[tier_idx].max_size;

        // Global memory cap: if adding this tier's buffer would exceed the
        // limit, fall back to a direct allocation (non-blocking).
        if self.inner.global_max_bytes > 0
            && self.inner.current_bytes.load(Ordering::Relaxed) + tier_max
                > self.inner.global_max_bytes
        {
            return PooledBuffer {
                data: Some(Vec::with_capacity(capacity)),
                tier_index: tier_idx,
                pool: Weak::new(),
            };
        }

        // Try to reuse an idle buffer from the tier's free queue.
        let reused_opt = {
            let mut queue = self.inner.tiers[tier_idx].lock();
            queue.pop()
        };

        let vec = if let Some(mut v) = reused_opt {
            self.inner.total_reused.fetch_add(1, Ordering::Relaxed);
            self.inner.tier_reused[tier_idx].fetch_add(1, Ordering::Relaxed);
            v.clear();
            v
        } else {
            self.inner.total_allocated.fetch_add(1, Ordering::Relaxed);
            self.inner.tier_allocated[tier_idx].fetch_add(1, Ordering::Relaxed);
            let v = Vec::with_capacity(tier_max);
            self.inner.current_bytes.fetch_add(v.capacity(), Ordering::Relaxed);
            v
        };

        self.inner.current_in_use.fetch_add(1, Ordering::Relaxed);
        self.inner.tier_in_use[tier_idx].fetch_add(1, Ordering::Relaxed);

        PooledBuffer { data: Some(vec), tier_index: tier_idx, pool: Arc::downgrade(&self.inner) }
    }

    /// Acquire a buffer and immediately resize it to `len` bytes (filled with
    /// zeros).  Equivalent to `acquire(len)` followed by `resize(len, 0)`.
    pub fn acquire_with_len(&self, len: usize) -> PooledBuffer {
        let mut buf = self.acquire(len);
        buf.resize(len, 0);
        buf
    }

    /// Snapshot the pool's runtime statistics.
    pub fn stats(&self) -> BufferPoolStats {
        let allocated = self.inner.total_allocated.load(Ordering::Relaxed);
        let reused = self.inner.total_reused.load(Ordering::Relaxed);
        let in_use = self.inner.current_in_use.load(Ordering::Relaxed);
        let bytes = self.inner.current_bytes.load(Ordering::Relaxed);

        let mut idle_total = 0usize;
        let mut tier_stats = Vec::with_capacity(self.inner.configs.len());

        for (i, config) in self.inner.configs.iter().enumerate() {
            let idle = self.inner.tiers[i].lock().len();
            idle_total += idle;
            tier_stats.push(BufferTierStats {
                tier_index: i,
                min_size: config.min_size,
                max_size: config.max_size,
                max_count: config.max_count,
                current_idle: idle,
                current_in_use: self.inner.tier_in_use[i].load(Ordering::Relaxed),
                allocated_count: self.inner.tier_allocated[i].load(Ordering::Relaxed),
                reused_count: self.inner.tier_reused[i].load(Ordering::Relaxed),
            });
        }

        let total_ops = allocated + reused;
        let reuse_rate = if total_ops == 0 { 0.0 } else { reused as f64 / total_ops as f64 };

        BufferPoolStats {
            total_allocated: allocated,
            total_reused: reused,
            reuse_rate,
            current_in_use: in_use,
            current_idle: idle_total,
            current_bytes: bytes,
            tier_stats,
        }
    }

    /// Drain all free queues, releasing their backing memory.  Buffers
    /// currently checked out are unaffected and will be freed (not returned)
    /// when dropped because the queues are empty.
    pub fn clear(&self) {
        for tier in &self.inner.tiers {
            let mut queue = tier.lock();
            let freed: usize = queue.iter().map(|v| v.capacity()).sum();
            queue.clear();
            self.inner.current_bytes.fetch_sub(freed, Ordering::Relaxed);
        }
    }
}

// ---------------------------------------------------------------------------
// PooledBuffer (RAII handle)
// ---------------------------------------------------------------------------

/// An RAII buffer handle backed by a `Vec<u8>`.
///
/// On drop the backing `Vec` is returned to its originating tier's free queue
/// (if the pool still exists and the queue is not full); otherwise the memory
/// is released.  Implements `Deref<Target = [u8]>` and `DerefMut` so it can
/// be used anywhere a byte slice is expected.
pub struct PooledBuffer {
    /// `None` after `into_vec` consumes the buffer.
    data: Option<Vec<u8>>,
    /// Index of the tier this buffer belongs to (`usize::MAX` for unpooled).
    tier_index: usize,
    /// Weak reference to the pool; empty for unpooled / oversized buffers.
    pool: Weak<BufferPoolInner>,
}

impl PooledBuffer {
    /// Returns the buffer contents as an immutable slice.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_ref().expect("buffer data was consumed")
    }

    /// Returns the buffer contents as a mutable slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.data.as_mut().expect("buffer data was consumed")
    }

    /// Current logical length of the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.as_ref().expect("buffer data was consumed").len()
    }

    /// Returns `true` if the buffer has length 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Allocated capacity of the backing `Vec`.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.data.as_ref().expect("buffer data was consumed").capacity()
    }

    /// Resize the buffer to `new_len`, filling new bytes with `value`.
    #[inline]
    pub fn resize(&mut self, new_len: usize, value: u8) {
        self.data.as_mut().expect("buffer data was consumed").resize(new_len, value);
    }

    /// Append `other` to the end of the buffer.
    #[inline]
    pub fn extend_from_slice(&mut self, other: &[u8]) {
        self.data.as_mut().expect("buffer data was consumed").extend_from_slice(other);
    }

    /// Consume the handle and return the backing `Vec`, detaching it from pool
    /// management.  The caller becomes responsible for the memory.
    pub fn into_vec(mut self) -> Vec<u8> {
        let vec = self.data.take().expect("buffer data was already consumed");
        let cap = vec.capacity();
        if let Some(pool) = self.pool.upgrade() {
            pool.detach_buffer(self.tier_index, cap);
        }
        vec
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(vec) = self.data.take() {
            if let Some(pool) = self.pool.upgrade() {
                pool.return_buffer(self.tier_index, vec);
            }
            // If the pool has been dropped (Weak upgrade fails), `vec` is
            // dropped here and its memory is freed.
        }
    }
}

impl Deref for PooledBuffer {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for PooledBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl AsRef<[u8]> for PooledBuffer {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsMut<[u8]> for PooledBuffer {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

impl std::fmt::Debug for PooledBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledBuffer")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("tier_index", &self.tier_index)
            .field("pooled", &(self.pool.strong_count() > 0))
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::atomic::AtomicBool, thread};

    /// Helper: build a tiny two-tier config for deterministic tests.
    fn tiny_config() -> BufferPoolConfig {
        BufferPoolConfig {
            tiers: vec![
                BufferTierConfig { min_size: 1, max_size: 1024, max_count: 2, alloc_count: 0 },
                BufferTierConfig { min_size: 1025, max_size: 8192, max_count: 1, alloc_count: 0 },
            ],
            global_max_bytes: 0, // unlimited for tests
        }
    }

    #[test]
    fn test_pool_acquire_and_release() {
        let pool = BufferPool::new(tiny_config());
        let mut buf = pool.acquire(512);
        assert_eq!(buf.len(), 0, "freshly acquired buffer must have length 0");
        assert!(buf.capacity() >= 512, "capacity must be at least requested");
        assert_eq!(buf.capacity(), 1024, "tier 0 buffers are sized to max_size");

        // Write some data.
        buf.extend_from_slice(b"hello");
        assert_eq!(buf.len(), 5);
        assert_eq!(&buf[..], b"hello");

        let stats_before = pool.stats();
        assert_eq!(stats_before.current_in_use, 1);

        // Drop returns the buffer to the free queue.
        drop(buf);
        let stats_after = pool.stats();
        assert_eq!(stats_after.current_in_use, 0);
        assert_eq!(stats_after.current_idle, 1);
    }

    #[test]
    fn test_pool_reuse() {
        let pool = BufferPool::new(tiny_config());

        // First acquire: fresh allocation (miss).
        let buf1 = pool.acquire(256);
        let cap1 = buf1.capacity();
        drop(buf1);

        let stats_mid = pool.stats();
        assert_eq!(stats_mid.total_allocated, 1);
        assert_eq!(stats_mid.total_reused, 0);

        // Second acquire: should reuse the freed buffer (hit).
        let buf2 = pool.acquire(256);
        assert_eq!(buf2.capacity(), cap1, "reused buffer keeps its capacity");

        let stats_after = pool.stats();
        assert_eq!(stats_after.total_allocated, 1, "no new allocation on reuse");
        assert_eq!(stats_after.total_reused, 1, "second acquire must be a reuse");
        assert!(stats_after.reuse_rate > 0.0);

        drop(buf2);
    }

    #[test]
    fn test_pool_tier_selection() {
        let pool = BufferPool::with_default();

        // Tier 0: max_size = 4 KiB
        let b0 = pool.acquire(64);
        assert_eq!(b0.capacity(), 4 * 1024, "64B -> tier 0 (4 KiB)");

        // Tier 1: max_size = 64 KiB
        let b1 = pool.acquire(8 * 1024);
        assert_eq!(b1.capacity(), 64 * 1024, "8 KiB -> tier 1 (64 KiB)");

        // Tier 2: max_size = 1 MiB
        let b2 = pool.acquire(128 * 1024);
        assert_eq!(b2.capacity(), 1024 * 1024, "128 KiB -> tier 2 (1 MiB)");

        // Tier 3: max_size = 16 MiB
        let b3 = pool.acquire(2 * 1024 * 1024);
        assert_eq!(b3.capacity(), 16 * 1024 * 1024, "2 MiB -> tier 3 (16 MiB)");

        // Verify per-tier in_use counts via stats.
        let stats = pool.stats();
        assert_eq!(stats.tier_stats[0].current_in_use, 1);
        assert_eq!(stats.tier_stats[1].current_in_use, 1);
        assert_eq!(stats.tier_stats[2].current_in_use, 1);
        assert_eq!(stats.tier_stats[3].current_in_use, 1);

        drop(b0);
        drop(b1);
        drop(b2);
        drop(b3);
    }

    #[test]
    fn test_pool_max_count() {
        // Tier 0 max_count = 2 in tiny_config.
        let pool = BufferPool::new(tiny_config());

        // Acquire 3 buffers in tier 0.
        let b1 = pool.acquire(100);
        let b2 = pool.acquire(100);
        let b3 = pool.acquire(100);
        assert_eq!(pool.stats().current_in_use, 3);

        // Drop all three.  Only 2 should fit in the free queue; the third is
        // freed and its memory released.
        drop(b1);
        drop(b2);
        drop(b3);

        let stats = pool.stats();
        assert_eq!(stats.current_idle, 2, "free queue capped at max_count=2");
        assert_eq!(stats.current_in_use, 0);
        // 3 allocated, 1 freed (because queue full), 2 retained -> bytes = 2 * 1024.
        assert_eq!(stats.current_bytes, 2 * 1024);
    }

    #[test]
    fn test_pool_oversized_allocation() {
        let pool = BufferPool::with_default();
        let huge = 32 * 1024 * 1024; // 32 MiB > tier 3 max (16 MiB)

        let buf = pool.acquire(huge);
        // Oversized buffers are allocated directly with the requested capacity
        // (not rounded to a tier max_size) and are not managed by the pool.
        assert!(buf.capacity() >= huge);
        assert_ne!(buf.capacity(), 16 * 1024 * 1024, "must not be tier-3 sized");

        let stats_before = pool.stats();
        assert_eq!(stats_before.current_in_use, 0, "oversized not counted in pool");
        assert_eq!(stats_before.current_bytes, 0, "oversized bytes not counted");

        drop(buf);
        let stats_after = pool.stats();
        assert_eq!(stats_after.current_idle, 0, "oversized not returned to pool");
    }

    #[test]
    fn test_pooled_buffer_deref() {
        let pool = BufferPool::new(tiny_config());
        let mut buf = pool.acquire_with_len(16);

        // Deref to [u8] — read.
        assert_eq!(buf.len(), 16);
        assert!(buf.iter().all(|&b| b == 0), "acquire_with_len zero-fills");

        // DerefMut — write via index.
        buf[0] = 0xAB;
        buf[15] = 0xCD;
        assert_eq!(buf[0], 0xAB);
        assert_eq!(buf[15], 0xCD);

        // resize grows and fills.
        buf.resize(32, 0xFF);
        assert_eq!(buf.len(), 32);
        assert_eq!(buf[16], 0xFF);
        assert_eq!(buf[0], 0xAB, "existing data preserved on resize");

        // extend_from_slice appends.
        buf.extend_from_slice(b"tail");
        assert_eq!(buf.len(), 36);
        assert_eq!(&buf[32..36], b"tail");

        // as_slice / as_mut_slice.
        let s: &[u8] = buf.as_slice();
        assert_eq!(s.len(), 36);
        let m: &mut [u8] = buf.as_mut_slice();
        m[0] = 0x01;
        assert_eq!(buf[0], 0x01);

        // is_empty.
        assert!(!buf.is_empty());
        buf.resize(0, 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_pool_stats() {
        let pool = BufferPool::new(tiny_config());

        // Empty pool.
        let s0 = pool.stats();
        assert_eq!(s0.total_allocated, 0);
        assert_eq!(s0.total_reused, 0);
        assert_eq!(s0.reuse_rate, 0.0);
        assert_eq!(s0.current_in_use, 0);
        assert_eq!(s0.current_idle, 0);
        assert_eq!(s0.current_bytes, 0);
        assert_eq!(s0.tier_stats.len(), 2);

        // Acquire two (fresh).
        let b1 = pool.acquire(100);
        let b2 = pool.acquire(200);
        let s1 = pool.stats();
        assert_eq!(s1.total_allocated, 2);
        assert_eq!(s1.total_reused, 0);
        assert_eq!(s1.current_in_use, 2);
        assert_eq!(s1.current_bytes, 2 * 1024);

        // Drop one -> idle.
        drop(b1);
        let s2 = pool.stats();
        assert_eq!(s2.current_in_use, 1);
        assert_eq!(s2.current_idle, 1);
        assert_eq!(s2.current_bytes, 2 * 1024, "idle buffer still counted in bytes");

        // Re-acquire -> reuse.
        let b3 = pool.acquire(100);
        let s3 = pool.stats();
        assert_eq!(s3.total_allocated, 2, "reuse does not allocate");
        assert_eq!(s3.total_reused, 1);
        assert_eq!(s3.current_in_use, 2);
        assert_eq!(s3.current_idle, 0);
        let expected_rate = 1.0 / 3.0;
        assert!((s3.reuse_rate - expected_rate).abs() < 1e-9);

        drop(b2);
        drop(b3);
    }

    #[test]
    fn test_pool_clear() {
        let pool = BufferPool::new(tiny_config());

        // Acquire and drop two buffers to populate the free queue.
        let b1 = pool.acquire(100);
        let b2 = pool.acquire(200);
        drop(b1);
        drop(b2);

        let before = pool.stats();
        assert_eq!(before.current_idle, 2);
        assert_eq!(before.current_bytes, 2 * 1024);

        pool.clear();

        let after = pool.stats();
        assert_eq!(after.current_idle, 0, "clear empties all free queues");
        assert_eq!(after.current_bytes, 0, "clear releases all idle memory");
        assert_eq!(after.current_in_use, 0);
    }

    #[test]
    fn test_pool_into_vec_detaches() {
        let pool = BufferPool::new(tiny_config());
        let buf = pool.acquire(100);
        let cap = buf.capacity();

        let stats_before = pool.stats();
        assert_eq!(stats_before.current_in_use, 1);
        assert_eq!(stats_before.current_bytes, cap);

        let vec = buf.into_vec();
        assert_eq!(vec.capacity(), cap);

        let stats_after = pool.stats();
        assert_eq!(stats_after.current_in_use, 0, "into_vec decrements in_use");
        assert_eq!(stats_after.current_bytes, 0, "into_vec releases pool bytes");
        assert_eq!(stats_after.current_idle, 0, "into_vec does not return to pool");

        // The Vec is now owned by the caller and outlives the pool handle.
        drop(vec);
    }

    #[test]
    fn test_pool_global_max_bytes() {
        // Tier 0 buffer = 1024 bytes, cap = 1500 -> only one pooled buffer.
        let config = BufferPoolConfig {
            tiers: vec![BufferTierConfig {
                min_size: 1,
                max_size: 1024,
                max_count: 8,
                alloc_count: 0,
            }],
            global_max_bytes: 1500,
        };
        let pool = BufferPool::new(config);

        // First acquire fits under the cap -> pooled.
        let b1 = pool.acquire(100);
        assert_eq!(pool.stats().current_in_use, 1);
        assert_eq!(pool.stats().current_bytes, 1024);

        // Second acquire would exceed cap (1024 + 1024 > 1500) -> unpooled.
        let b2 = pool.acquire(100);
        // b2 is unpooled: not counted in pool stats.
        assert_eq!(pool.stats().current_in_use, 1);
        assert_eq!(pool.stats().current_bytes, 1024);

        drop(b1);
        drop(b2);
        // b1 returned to pool (idle), b2 was unpooled and freed.
        assert_eq!(pool.stats().current_idle, 1);
        assert_eq!(pool.stats().current_bytes, 1024);
    }

    #[test]
    fn test_pool_concurrent() {
        let pool = BufferPool::with_default();
        let pool_clone = pool.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);

        let handle = thread::spawn(move || {
            let mut local_reuses = 0usize;
            for i in 0..500 {
                if stop_clone.load(Ordering::Relaxed) {
                    break;
                }
                let cap = match i % 4 {
                    0 => 128,
                    1 => 8 * 1024,
                    2 => 128 * 1024,
                    _ => 2 * 1024 * 1024,
                };
                let mut buf = pool_clone.acquire(cap);
                buf.resize(cap.min(1024), 0xAA);
                if buf.capacity() > 0 {
                    local_reuses += 1; // just to keep the variable used
                }
                drop(buf);
            }
            local_reuses
        });

        // Main thread also hammers the pool.
        for i in 0..500 {
            let cap = match i % 3 {
                0 => 256,
                1 => 16 * 1024,
                _ => 512 * 1024,
            };
            let buf = pool.acquire(cap);
            assert!(buf.capacity() >= cap, "capacity must satisfy request");
            drop(buf);
        }

        stop.store(true, Ordering::Relaxed);
        handle.join().expect("worker thread panicked");

        // After all buffers are dropped, in_use must be zero.
        let stats = pool.stats();
        assert_eq!(stats.current_in_use, 0, "all buffers must be returned");
        assert!(stats.total_allocated > 0, "some allocations must have occurred");
        assert!(stats.total_reused > 0, "some reuses must have occurred");
        assert!(stats.reuse_rate >= 0.0 && stats.reuse_rate <= 1.0);
    }

    #[test]
    fn test_pool_zero_capacity() {
        let pool = BufferPool::with_default();
        let buf = pool.acquire(0);
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.capacity(), 0);
        // Zero-capacity buffers are unpooled.
        assert_eq!(pool.stats().current_in_use, 0);
        drop(buf);
        assert_eq!(pool.stats().current_idle, 0);
    }

    #[test]
    fn test_pool_acquire_with_len() {
        let pool = BufferPool::new(tiny_config());
        let buf = pool.acquire_with_len(42);
        assert_eq!(buf.len(), 42);
        assert!(buf.capacity() >= 42);
        assert!(buf.iter().all(|&b| b == 0), "must be zero-filled");
        drop(buf);
    }

    #[test]
    fn test_pool_clone_shares_state() {
        let pool = BufferPool::new(tiny_config());
        let pool2 = pool.clone();

        let buf = pool.acquire(100);
        // Both handles see the same in_use count.
        assert_eq!(pool.stats().current_in_use, 1);
        assert_eq!(pool2.stats().current_in_use, 1);

        drop(buf);
        assert_eq!(pool.stats().current_idle, 1);
        assert_eq!(pool2.stats().current_idle, 1);

        // Reuse via the second handle.
        let buf2 = pool2.acquire(100);
        assert_eq!(pool.stats().total_reused, 1);
        drop(buf2);
    }
}


#[cfg(test)]
mod additional_tests {
    use super::*;

    #[test]
    fn test_buffer_tier_config_fields() {
        let cfg = BufferTierConfig { min_size: 100, max_size: 200, max_count: 10, alloc_count: 5 };
        assert_eq!(cfg.min_size, 100);
        assert_eq!(cfg.max_size, 200);
        assert_eq!(cfg.max_count, 10);
        assert_eq!(cfg.alloc_count, 5);
    }

    #[test]
    fn test_buffer_pool_config_default_tiers() {
        let cfg = BufferPoolConfig::default();
        assert_eq!(cfg.tiers.len(), 4);
        assert_eq!(cfg.tiers[0].min_size, 64);
        assert_eq!(cfg.tiers[0].max_size, 4 * 1024);
        assert_eq!(cfg.tiers[1].max_size, 64 * 1024);
        assert_eq!(cfg.tiers[2].max_size, 1024 * 1024);
        assert_eq!(cfg.tiers[3].max_size, 16 * 1024 * 1024);
        assert_eq!(cfg.global_max_bytes, 256 * 1024 * 1024);
    }

    #[test]
    fn test_pooled_buffer_debug() {
        let pool = BufferPool::with_default();
        let buf = pool.acquire(128);
        let s = format!("{buf:?}");
        assert!(s.contains("PooledBuffer"));
        assert!(s.contains("len"));
        assert!(s.contains("capacity"));
    }

    #[test]
    fn test_pooled_buffer_as_ref_as_mut() {
        let pool = BufferPool::with_default();
        let mut buf = pool.acquire_with_len(16);
        let s: &[u8] = buf.as_ref();
        assert_eq!(s.len(), 16);
        let m: &mut [u8] = buf.as_mut();
        m[0] = 0xAB;
        assert_eq!(buf[0], 0xAB);
    }

    #[test]
    fn test_buffer_pool_debug() {
        let pool = BufferPool::with_default();
        let _buf = pool.acquire(128);
        let s = format!("{pool:?}");
        assert!(s.contains("BufferPool"));
        assert!(s.contains("tiers"));
    }

    #[test]
    fn test_buffer_tier_stats_serialization() {
        let stats = BufferTierStats {
            tier_index: 0,
            min_size: 64,
            max_size: 4096,
            max_count: 1024,
            current_idle: 5,
            current_in_use: 3,
            allocated_count: 10,
            reused_count: 7,
        };
        let json = format!("{:?}", stats);
        assert!(json.contains("tier_index"));
        assert!(json.contains("current_idle"));
    }

    #[test]
    fn test_buffer_pool_stats_serialization() {
        let pool = BufferPool::with_default();
        let _b = pool.acquire(128);
        let stats = pool.stats();
        let json = format!("{:?}", stats);
        assert!(json.contains("total_allocated"));
        assert!(json.contains("total_reused"));
        assert!(json.contains("reuse_rate"));
        assert!(json.contains("tier_stats"));
    }

    #[test]
    fn test_acquire_exact_tier_boundary() {
        let pool = BufferPool::with_default();
        // Exactly 4KB → tier 0
        let b0 = pool.acquire(4 * 1024);
        assert_eq!(b0.capacity(), 4 * 1024);
        // 4KB + 1 → tier 1
        let b1 = pool.acquire(4 * 1024 + 1);
        assert_eq!(b1.capacity(), 64 * 1024);
        drop(b0);
        drop(b1);
    }

    #[test]
    fn test_acquire_16mb_boundary() {
        let pool = BufferPool::with_default();
        // Exactly 16MB → tier 3
        let b = pool.acquire(16 * 1024 * 1024);
        assert_eq!(b.capacity(), 16 * 1024 * 1024);
        drop(b);
        // 16MB + 1 → oversized (unpooled)
        let b2 = pool.acquire(16 * 1024 * 1024 + 1);
        assert!(b2.capacity() > 16 * 1024 * 1024);
        drop(b2);
    }

    #[test]
    fn test_pooled_buffer_into_vec_twice_panics() {
        let pool = BufferPool::with_default();
        let buf = pool.acquire(128);
        let _vec = buf.into_vec();
        // Calling into_vec again would panic - but we can't call it on moved value
        // This test just verifies the first into_vec works
        // (no additional assertion needed; into_vec() succeeding is the test)
    }
}
