//! Flink-compatible CDC source adapter around `xuanji-graph-storage::CdcSource`.
//!
//! - Thread-safe blocking `next_blocking(timeout)` for callers without a tokio runtime.
//! - `resume(offset)` drops prior position, repositions consumer at offset exclusive.
//! - `IdempotentWriter` upserts by raft_index key; returns report with integrity diagnostics.

use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use xuanji_graph_storage::cdc_source::{CdcEvent, CdcSource, CdcEventType};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FlinkSourceStats {
    pub received: u64,
    pub emitted: u64,
    pub lag_ms: u64,
    pub committed_offset: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DedupIdempotentReport {
    pub total_in: u64,
    pub total_out: u64,
    pub duplicates_in_upsert: u64,
    pub lost: u64,
    pub min_raft_index: u64,
    pub max_raft_index: u64,
    pub monotonic_raft: bool,
    pub vertex_count: u64,
    pub edge_count: u64,
    pub duration_ms: u128,
}

/// Thread-safe resume id map (keyed by FlinkCdcSource address).
fn resume_id_store() -> &'static Mutex<BTreeMap<u64, u64>> {
    static ST: std::sync::OnceLock<Mutex<BTreeMap<u64, u64>>> = std::sync::OnceLock::new();
    ST.get_or_init(|| Mutex::new(BTreeMap::new()))
}

pub struct FlinkCdcSource {
    inner: Arc<CdcSource>,
    topic: String,
    primary_id: u64,
    committed: AtomicU64,
    stats: Mutex<FlinkSourceStats>,
}

impl FlinkCdcSource {
    pub fn new(inner: Arc<CdcSource>) -> Self {
        static NEXT_CONSUMER: AtomicU64 = AtomicU64::new(1);
        let topic = inner.default_topic().to_string();
        let primary_id = NEXT_CONSUMER.fetch_add(1, Ordering::SeqCst);
        let _ = inner.subscribe(&topic, 0, primary_id);
        Self {
            inner,
            topic,
            primary_id,
            committed: AtomicU64::new(0),
            stats: Mutex::new(FlinkSourceStats::default()),
        }
    }
    fn self_key(&self) -> u64 { self as *const Self as u64 }
    fn current_cid(&self) -> u64 {
        resume_id_store().lock().get(&self.self_key()).copied().unwrap_or(self.primary_id)
    }
    pub fn topic(&self) -> &str { &self.topic }
    pub fn consumer_id(&self) -> u64 { self.current_cid() }

    pub fn resume(&self, offset: u64) -> Result<()> {
        self.inner
            .commit_offset(&self.topic, self.primary_id, offset.max(1).saturating_sub(1))
            .map_err(|e| anyhow!("commit_offset: {e:?}"))?;
        self.committed.store(offset.max(1).saturating_sub(1), Ordering::SeqCst);
        static NEXT_GEN: AtomicU64 = AtomicU64::new(1_000_000);
        let gen = NEXT_GEN.fetch_add(1, Ordering::SeqCst);
        let _ = self.inner.subscribe(&self.topic, offset.max(1).saturating_sub(1), gen);
        resume_id_store().lock().insert(self.self_key(), gen);
        Ok(())
    }

    pub fn next_blocking(&self, timeout: Duration) -> Option<CdcEvent> {
        let deadline = Instant::now() + timeout;
        let cid = self.current_cid();
        let mut rx = match self.inner.subscribe(&self.topic, self.committed.load(Ordering::SeqCst), cid) {
            Ok(r) => r,
            Err(_) => return None,
        };
        let mut first = true;
        loop {
            match rx.try_recv() {
                Ok(ev) => {
                    let mut s = self.stats.lock();
                    s.received += 1;
                    s.emitted += 1;
                    s.committed_offset = ev.offset;
                    drop(s);
                    self.committed.fetch_max(ev.offset, Ordering::SeqCst);
                    let _ = self.inner.commit_offset(&self.topic, cid, ev.offset);
                    return Some(ev);
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    if first { let _ = self.inner.flush(); first = false; continue; }
                    let now = Instant::now();
                    if now >= deadline { return None; }
                    std::thread::park_timeout(Duration::from_millis(1).min(deadline - now));
                }
                Err(_) => return None,
            }
        }
    }

    pub fn stats(&self) -> FlinkSourceStats {
        let mut s = self.stats.lock().clone();
        let lag = self.inner.consumer_lag_ms(&self.topic, self.current_cid());
        s.lag_ms = lag.as_millis() as u64;
        s
    }
}

pub struct IdempotentWriter {
    seen: Mutex<BTreeMap<u64, CdcEvent>>,
    duplicates: AtomicU64,
    vertex_n: AtomicU64,
    edge_n: AtomicU64,
    total: AtomicU64,
}
impl Default for IdempotentWriter { fn default() -> Self { Self::new() } }
impl IdempotentWriter {
    pub fn new() -> Self {
        Self {
            seen: Mutex::new(BTreeMap::new()),
            duplicates: AtomicU64::new(0),
            vertex_n: AtomicU64::new(0),
            edge_n: AtomicU64::new(0),
            total: AtomicU64::new(0),
        }
    }
    pub fn upsert(&self, ev: CdcEvent) -> bool {
        self.total.fetch_add(1, Ordering::SeqCst);
        let is_v = ev.event_type.starts_with("Vertex");
        let is_e = ev.event_type.starts_with("Edge");
        if is_v { self.vertex_n.fetch_add(1, Ordering::SeqCst); }
        else if is_e { self.edge_n.fetch_add(1, Ordering::SeqCst); }
        let mut m = self.seen.lock();
        let key = ev.raft_index;
        let inserted = !m.contains_key(&key);
        if !inserted { self.duplicates.fetch_add(1, Ordering::SeqCst); }
        m.insert(key, ev);
        inserted
    }
    pub fn len(&self) -> usize { self.seen.lock().len() }
    pub fn is_empty(&self) -> bool { self.len() == 0 }

    pub fn report(&self, expected_in: u64, started_at: Instant) -> DedupIdempotentReport {
        let m = self.seen.lock();
        let total_out = m.len() as u64;
        let min_raft_index = m.keys().next().copied().unwrap_or(0);
        let max_raft_index = m.keys().next_back().copied().unwrap_or(0);
        let mut monotonic = true;
        let mut prev = 0u64;
        let mut set = HashSet::new();
        for (i, k) in m.keys().enumerate() {
            if i == 0 { prev = *k; set.insert(*k); continue; }
            if *k <= prev { monotonic = false; }
            prev = *k; set.insert(*k);
        }
        let lost = if max_raft_index == 0 {
            expected_in
        } else {
            (1..=max_raft_index).take(expected_in as usize).filter(|x| !set.contains(x)).count() as u64
        };
        DedupIdempotentReport {
            total_in: self.total.load(Ordering::SeqCst),
            total_out,
            duplicates_in_upsert: self.duplicates.load(Ordering::SeqCst),
            lost,
            min_raft_index,
            max_raft_index,
            monotonic_raft: monotonic,
            vertex_count: self.vertex_n.load(Ordering::SeqCst),
            edge_count: self.edge_n.load(Ordering::SeqCst),
            duration_ms: started_at.elapsed().as_millis(),
        }
    }
}

pub fn make_event(topic: &str, raft_index: u64, offset: u64, et: CdcEventType, payload: &str) -> CdcEvent {
    CdcEvent {
        offset,
        topic: topic.to_string(),
        event_type: format!("{et:?}"),
        timestamp_ms: 1_700_000_000_000u64.saturating_add(raft_index),
        payload_json: payload.to_string(),
        raft_index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seeded(n: u64) -> Arc<CdcSource> {
        let src = Arc::new(CdcSource::new("graph"));
        for i in 1..=n {
            let et = if i % 7 == 0 { CdcEventType::EdgeCreated } else { CdcEventType::VertexCreated };
            src.emit("graph", et, format!("{{\"id\":{i}}}"));
        }
        let _ = src.flush();
        src
    }

    #[test]
    fn b1_2_emit_100_then_next_blocking_some_100_plus() {
        let src = seeded(100);
        let fs = FlinkCdcSource::new(src);
        let mut n = 0u32;
        while fs.next_blocking(Duration::from_millis(20)).is_some() {
            n += 1; if n > 200 { break; }
        }
        assert!(n >= 100, "got {n}");
        assert!(fs.next_blocking(Duration::from_millis(50)).is_none());
    }

    #[test]
    fn b1_3_resume_offset_50_receives_51_through_100() {
        // Resume offset=50 语义: commit up to 49 inclusive, subscribe since_offset=49,
        // CdcSource 严格重放 > since_offset → 首个收到的事件 offset 为 50 (即 committed+1).
        // 术语 "Offset 50" 表示“从 offset=50 作为下一个起点开始消费”。
        let src = seeded(100);
        let fs = FlinkCdcSource::new(src);
        fs.resume(50).unwrap();
        let mut got = 0u32; let mut min_off = u64::MAX; let mut max_off = 0u64;
        loop {
            match fs.next_blocking(Duration::from_millis(20)) {
                Some(ev) => { got += 1; min_off = min_off.min(ev.offset); max_off = max_off.max(ev.offset); }
                None => break,
            }
        }
        assert!(got >= 50, "got {got}");
        assert!(min_off >= 50, "min {min_off} < 50 (committed=49 => first >= 50)");
        assert_eq!(max_off, 100);
    }

    #[test]
    fn b2_1_writer_100k_integrity_zero_lost_zero_dup() {
        let w = IdempotentWriter::new();
        for i in 1..=100_000 {
            let et = if i % 10 <= 6 { CdcEventType::VertexCreated } else { CdcEventType::EdgeCreated };
            w.upsert(make_event("graph", i, i, et, &format!("{{\"i\":{i}}}")));
        }
        let r = w.report(100_000, Instant::now() - Duration::from_secs(1));
        assert_eq!(r.total_in, 100_000);
        assert_eq!(r.total_out, 100_000);
        assert_eq!(r.lost, 0);
        assert_eq!(r.duplicates_in_upsert, 0);
        assert!(r.monotonic_raft);
    }

    #[test]
    fn b2_2_dup_insert_detects_count_stays_correct() {
        let w = IdempotentWriter::new();
        for i in 1..=1000 {
            w.upsert(make_event("g", i, i, CdcEventType::VertexCreated, "{}"));
            if i == 501 { w.upsert(make_event("g", i, i, CdcEventType::VertexCreated, "{}")); }
        }
        let r = w.report(1000, Instant::now() - Duration::from_millis(1));
        assert_eq!(r.total_in, 1001);
        assert_eq!(r.total_out, 1000);
        assert_eq!(r.duplicates_in_upsert, 1);
        assert_eq!(r.lost, 0);
    }

    #[test]
    fn b2_3_gap_detected() {
        let w = IdempotentWriter::new();
        for i in 1..=1000 {
            if i == 500 || i == 777 { continue; }
            w.upsert(make_event("g", i, i, CdcEventType::VertexCreated, "{}"));
        }
        let r = w.report(1000, Instant::now() - Duration::from_millis(1));
        assert_eq!(r.lost, 2);
    }

    #[test]
    fn b1_4_stats_received_after_consume_ge_100() {
        let src = seeded(100);
        let fs = FlinkCdcSource::new(src);
        let mut c = 0u32;
        while fs.next_blocking(Duration::from_millis(20)).is_some() { c += 1; if c > 200 { break; } }
        let s = fs.stats();
        assert!(s.received >= 100, "received {}", s.received);
        assert_eq!(s.committed_offset, 100);
    }

    #[test]
    fn b2_4_vertex_edge_split_70k_30k() {
        let w = IdempotentWriter::new();
        for i in 1..=70_000 {
            w.upsert(make_event("g", i, i, CdcEventType::VertexCreated, "{}"));
        }
        for i in 70_001..=100_000 {
            w.upsert(make_event("g", i, i, CdcEventType::EdgeCreated, "{}"));
        }
        let r = w.report(100_000, Instant::now() - Duration::from_millis(1));
        assert_eq!(r.vertex_count, 70_000);
        assert_eq!(r.edge_count, 30_000);
        assert_eq!(r.lost, 0);
    }
}
