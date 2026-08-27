// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use async_trait::async_trait;
use bytes::Bytes;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChunkStats {
    pub total_chunks: u64,
    pub total_bytes: u64,
    pub orphan_chunks: u64,
    pub referenced_chunks: u64,
}

#[derive(Debug, Clone, Default)]
struct ChunkEntry {
    data: Vec<u8>,
    ref_count: u64,
    checksum: String,
    size: u64,
}

#[async_trait]
pub trait ChunkManagerProvider: Send + Sync {
    async fn allocate_chunk(
        &self,
        expected_size: u64,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn write_chunk(
        &self,
        chunk_id: &str,
        data: Bytes,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn read_chunk(&self, chunk_id: &str) -> Result<Bytes, Box<dyn Error + Send + Sync>>;
    async fn delete_chunk(&self, chunk_id: &str) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn rebuild_chunk(
        &self,
        chunk_id: &str,
        replicas: Vec<String>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn report_stats(&self) -> Result<ChunkStats, Box<dyn Error + Send + Sync>>;
    async fn gc_orphan_chunks(
        &self,
        known_chunk_ids: Vec<String>,
    ) -> Result<u64, Box<dyn Error + Send + Sync>>;
}

pub struct MockChunkManagerProvider {
    chunks: parking_lot::Mutex<BTreeMap<String, ChunkEntry>>,
    next: parking_lot::Mutex<u64>,
}
impl Default for MockChunkManagerProvider {
    fn default() -> Self {
        Self {
            chunks: parking_lot::Mutex::new(BTreeMap::new()),
            next: parking_lot::Mutex::new(1),
        }
    }
}
fn sha256_hex(d: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(d);
    hex::encode(h.finalize())
}

#[async_trait]
impl ChunkManagerProvider for MockChunkManagerProvider {
    async fn allocate_chunk(
        &self,
        _expected_size: u64,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut n = self.next.lock();
        let id = format!("chunk-{}", *n);
        *n += 1;
        self.chunks.lock().insert(
            id.clone(),
            ChunkEntry {
                ..Default::default()
            },
        );
        Ok(id)
    }
    async fn write_chunk(
        &self,
        chunk_id: &str,
        data: Bytes,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let cs = sha256_hex(&data);
        let mut ch = self.chunks.lock();
        let entry = ch.get_mut(chunk_id).ok_or("chunk not found")?;
        entry.checksum = cs.clone();
        entry.size = data.len() as u64;
        entry.data = data.to_vec();
        entry.ref_count = entry.ref_count.max(1);
        Ok(cs)
    }
    async fn read_chunk(&self, chunk_id: &str) -> Result<Bytes, Box<dyn Error + Send + Sync>> {
        let ch = self.chunks.lock();
        let e = ch.get(chunk_id).ok_or("chunk not found")?;
        if e.data.is_empty() && e.size == 0 {
            return Err("chunk has no data".into());
        }
        Ok(Bytes::copy_from_slice(&e.data))
    }
    async fn delete_chunk(&self, chunk_id: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.chunks.lock().remove(chunk_id);
        Ok(())
    }
    async fn rebuild_chunk(
        &self,
        chunk_id: &str,
        _replicas: Vec<String>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut ch = self.chunks.lock();
        let e = ch.get_mut(chunk_id).ok_or("chunk not found")?;
        e.ref_count = e.ref_count.saturating_add(1);
        Ok(())
    }
    async fn report_stats(&self) -> Result<ChunkStats, Box<dyn Error + Send + Sync>> {
        let ch = self.chunks.lock();
        let total = ch.len() as u64;
        let mut bytes = 0u64;
        let mut orphans = 0u64;
        let mut refs = 0u64;
        for e in ch.values() {
            bytes += e.size;
            if e.ref_count == 0 {
                orphans += 1;
            } else {
                refs += 1;
            }
        }
        // if never written, count as orphan too
        let mut o2 = 0;
        for e in ch.values() {
            if e.size == 0 && e.ref_count == 0 {
                o2 += 1;
            }
        }
        let _ = o2;
        Ok(ChunkStats {
            total_chunks: total,
            total_bytes: bytes,
            orphan_chunks: orphans,
            referenced_chunks: refs,
        })
    }
    async fn gc_orphan_chunks(
        &self,
        known: Vec<String>,
    ) -> Result<u64, Box<dyn Error + Send + Sync>> {
        let known_set: BTreeSet<String> = known.into_iter().collect();
        let mut ch = self.chunks.lock();
        let before = ch.len();
        ch.retain(|k, _| known_set.contains(k));
        Ok((before - ch.len()) as u64)
    }
}
