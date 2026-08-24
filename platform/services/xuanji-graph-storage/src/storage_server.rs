//! StorageServer：R2 Storage 对外公开的 7 API 实现。
//!
//! 7 API：
//!   1. start_cluster(shard_count, storage_addrs)
//!   2. add_vertex(vid, tag, props) -> VertexAck
//!   3. update_vertex(vid, merge_props) -> ()
//!   4. remove_vertex(vid) -> bool
//!   5. add_edge(src, dst, etype, rank, weight, props) -> EdgeAck
//!   6. remove_edge(src, dst, etype, rank) -> bool
//!   7. get_neighbors(vid, direction, etypes) -> Vec<Neighbor>
//!   8. scan_edges(etypes, limit, offset) -> Vec<Edge>
//!
//! CDC 与 hot cache 在 storage_server 内部引用 cdc_source.rs / storage_api.rs。

use crate::cdc_source::{CdcEventType, CdcSource};
use crate::error::{StorageError, StorageResult};
use crate::graph_codec::{self, PropValue};
use crate::kv_engine::{self, KvEngine};
use crate::partition_raft::{rebalance_16_to_32, OrderedF64, RaftLog, ShardRaft};
pub type StorageAddr = String;
pub type R2StorageServer = StorageServer;

use crate::storage_api::{
    weight_to_i64, Direction, Edge, EdgeAck, HotNeighborCache, Neighbor, VertexAck,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 简化版 tempdir（等价语义）：drop 时删除目录。
pub struct TempDirHolder {
    pub path: std::path::PathBuf,
    pub remove_on_drop: bool,
}
impl Drop for TempDirHolder {
    fn drop(&mut self) {
        if self.remove_on_drop && self.path.exists() {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}
impl TempDirHolder {
    pub fn new_in_tmp() -> std::io::Result<Self> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let base = std::env::temp_dir();
        let ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mut rng = rand::thread_rng();
        use rand::RngCore;
        let r = rng.next_u32();
        let p = base.join(format!("xuanji-r2-storage-{ns}-{r}"));
        std::fs::create_dir_all(&p)?;
        Ok(Self {
            path: p,
            remove_on_drop: true,
        })
    }
}
pub struct StorageServer {
    pub port: Option<u16>,
    pub shard_ids: Vec<u16>,
    pub rocks_db_handles: KvEngine,
    pub raft_nodes: Arc<ShardRaft>,
    pub cdc: Arc<CdcSource>,
    pub hot_cache: Arc<HotNeighborCache>,
    pub data_dir: PathBuf,
    /// 测试用：TempDir 生命周期绑定
    pub _tempdir: Option<std::sync::Arc<std::sync::Mutex<Option<TempDirHolder>>>>,
}

impl StorageServer {
    pub fn new(kv: KvEngine, shard_count: u16, storage_addrs: &[String]) -> StorageResult<Self> {
        // Use existing KvEngine ownership semantics: we can't re-open because the user provided one.
        // Mirror start_cluster with provided kv.
        if !shard_count.is_power_of_two() || shard_count == 0 {
            return Err(StorageError::InvalidArgument(
                "shard_count must be power of two".into(),
            ));
        }
        let shard_ids: Vec<u16> = (0..shard_count).collect();
        // Ensure shards are present
        let mut kv_mut = kv.clone();
        for s in &shard_ids {
            if !kv_mut.contains_shard(*s) {
                kv_mut.add_shard(*s)?;
            }
        }
        let raft = Arc::new(ShardRaft::new(kv.clone(), shard_count, storage_addrs));
        let cdc = Arc::new(CdcSource::new("default"));
        let cache = Arc::new(HotNeighborCache::new(100_000));
        Ok(Self {
            port: None,
            shard_ids,
            rocks_db_handles: kv.clone(),
            raft_nodes: raft,
            cdc,
            hot_cache: cache,
            data_dir: kv.path.to_path_buf(),
            _tempdir: None,
        })
    }

    /// start_cluster：单节点视角启动；真实分布式由 RaftGroup.storage_addrs 作为集群成员列表。
    ///
    /// path：如果为空则使用 tempdir（测试常用）。
    pub fn start_cluster(
        shard_count: u16,
        storage_addrs: &[String],
        path: Option<&Path>,
    ) -> StorageResult<Self> {
        if !shard_count.is_power_of_two() || shard_count == 0 {
            return Err(StorageError::InvalidArgument(
                "shard_count must be power of two".into(),
            ));
        }
        let shard_ids: Vec<u16> = (0..shard_count).collect();
        let (dir, is_temp) = match path {
            Some(p) => (p.to_path_buf(), None),
            None => {
                let tmp = TempDirHolder::new_in_tmp()
                    .map_err(|e| StorageError::Internal(format!("tmpdir {e}")))?;
                let p = tmp.path.clone();
                (
                    p,
                    Some(std::sync::Arc::new(std::sync::Mutex::new(Some(tmp)))),
                )
            }
        };
        let kv = KvEngine::open(&dir, &shard_ids)?;
        let raft = Arc::new(ShardRaft::new(kv.clone(), shard_count, storage_addrs));
        let cdc = Arc::new(CdcSource::new("default"));
        let cache = Arc::new(HotNeighborCache::new(100_000));
        Ok(Self {
            port: None,
            shard_ids,
            rocks_db_handles: kv,
            raft_nodes: raft,
            cdc,
            hot_cache: cache,
            data_dir: dir,
            _tempdir: is_temp,
        })
    }

    fn merge_props(
        old: &BTreeMap<String, PropValue>,
        patch: &BTreeMap<String, PropValue>,
    ) -> BTreeMap<String, PropValue> {
        let mut o = old.clone();
        for (k, v) in patch {
            // Sentinel：空 bytes 表示删除
            if matches!(v, PropValue::Null) || matches!(v, PropValue::Bytes(b) if b.is_empty()) {
                o.remove(k);
            } else {
                o.insert(k.clone(), v.clone());
            }
        }
        o
    }

    /// 内部：读取某 vid 的 (shard, tag, props)。允许多 tag，这里返回第一个匹配。
    fn read_vertex(&self, vid: &str) -> Option<(u16, String, BTreeMap<String, PropValue>)> {
        let sc = self.raft_nodes.shard_count();
        let shard = graph_codec::vid_hash_shard(vid, sc);
        let prefix = shard.to_le_bytes();
        let Ok(rows) = self
            .rocks_db_handles
            .seek_prefix(&kv_engine::cf_name_vid_meta(shard), &prefix)
        else {
            return None;
        };
        for (k, v) in rows {
            if let Ok((_, _, vv)) = graph_codec::decode_vertex_key(&k) {
                if vv == vid {
                    if let Ok((tag, props)) = graph_codec::decode_vertex_value(&v) {
                        return Some((shard, tag, props));
                    }
                }
            }
        }
        None
    }

    fn emit_cdc_vertex(
        &self,
        et: CdcEventType,
        vid: &str,
        tag: &str,
        props: &BTreeMap<String, PropValue>,
    ) {
        let map: BTreeMap<String, String> = props
            .iter()
            .map(|(k, v)| {
                (k.clone(), {
                    match v {
                        PropValue::Str(s) => s.clone(),
                        PropValue::Bytes(b) => String::from_utf8_lossy(b).into_owned(),
                        PropValue::Int(i) => i.to_string(),
                        PropValue::F64(u) => f64::from_bits(*u).to_string(),
                        PropValue::Bool(b) => b.to_string(),
                        PropValue::Null => String::new(),
                    }
                    .to_string()
                })
            })
            .collect();
        let payload = json!({
            "vid": vid,
            "tag": tag,
            "props": map,
        })
        .to_string();
        self.cdc.emit("default", et, payload);
    }

    fn emit_cdc_edge(
        &self,
        et: CdcEventType,
        src: &str,
        dst: &str,
        etype: &str,
        rank: i64,
        weight: Option<f64>,
        props: &BTreeMap<String, PropValue>,
    ) {
        let map: BTreeMap<String, String> = props
            .iter()
            .map(|(k, v)| {
                (k.clone(), {
                    match v {
                        PropValue::Str(s) => s.clone(),
                        PropValue::Bytes(b) => String::from_utf8_lossy(b).into_owned(),
                        PropValue::Int(i) => i.to_string(),
                        PropValue::F64(u) => f64::from_bits(*u).to_string(),
                        PropValue::Bool(b) => b.to_string(),
                        PropValue::Null => String::new(),
                    }
                    .to_string()
                })
            })
            .collect();
        let payload = json!({
            "src": src, "dst": dst, "etype": etype, "rank": rank,
            "weight": weight, "props": map,
        })
        .to_string();
        self.cdc.emit("default", et, payload);
    }

    pub fn add_vertex(
        &self,
        vid: String,
        tag: String,
        props: BTreeMap<String, PropValue>,
    ) -> StorageResult<VertexAck> {
        if vid.is_empty() {
            return Err(StorageError::InvalidArgument("empty vid".into()));
        }
        if tag.is_empty() {
            return Err(StorageError::InvalidArgument("empty tag".into()));
        }
        let shard = self.raft_nodes.shard_for_vid(&vid);
        let log = RaftLog::PutVertex {
            shard,
            vid: vid.clone(),
            tag: tag.clone(),
            props: props.clone(),
        };
        let idx = self.raft_nodes.apply(&log)?;
        // invalidate hot cache
        self.hot_cache.invalidate(&vid);
        // emit CDC
        self.emit_cdc_vertex(CdcEventType::VertexCreated, &vid, &tag, &props);
        Ok(VertexAck {
            vid,
            tag,
            shard,
            applied_index: idx,
        })
    }

    pub fn update_vertex(
        &self,
        vid: String,
        merge_props: BTreeMap<String, PropValue>,
    ) -> StorageResult<()> {
        let (shard, tag, old_props) = self
            .read_vertex(&vid)
            .ok_or_else(|| StorageError::VidNotFound(vid.clone()))?;
        let new_props = Self::merge_props(&old_props, &merge_props);
        let log = RaftLog::PutVertex {
            shard,
            vid: vid.clone(),
            tag: tag.clone(),
            props: new_props.clone(),
        };
        let _idx = self.raft_nodes.apply(&log)?;
        self.hot_cache.invalidate(&vid);
        self.emit_cdc_vertex(CdcEventType::VertexUpdated, &vid, &tag, &new_props);
        Ok(())
    }

    pub fn remove_vertex(&self, vid: &str) -> StorageResult<bool> {
        let shard = self.raft_nodes.shard_for_vid(vid);
        let log = RaftLog::DelVertex {
            shard,
            vid: vid.to_string(),
        };
        match self.raft_nodes.apply(&log) {
            Ok(_) => {
                self.hot_cache.invalidate(vid);
                let payload = json!({ "vid": vid }).to_string();
                self.cdc
                    .emit("default", CdcEventType::VertexDeleted, payload);
                Ok(true)
            }
            Err(StorageError::VidNotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub fn add_edge(
        &self,
        src: String,
        dst: String,
        edge_type: String,
        rank: i64,
        weight: Option<f64>,
        props: BTreeMap<String, PropValue>,
    ) -> StorageResult<EdgeAck> {
        if src.is_empty() || dst.is_empty() {
            return Err(StorageError::InvalidArgument("src/dst empty".into()));
        }
        if edge_type.is_empty() {
            return Err(StorageError::InvalidArgument("empty etype".into()));
        }
        let out_shard = self.raft_nodes.shard_for_vid(&src);
        let in_shard = self.raft_nodes.shard_for_vid(&dst);
        let log = RaftLog::PutEdge {
            out_shard,
            in_shard,
            src: src.clone(),
            dst: dst.clone(),
            etype: edge_type.clone(),
            rank,
            weight: weight.map(OrderedF64),
            props: props.clone(),
        };
        let idx = self.raft_nodes.apply(&log)?;
        self.hot_cache.invalidate(&src);
        self.hot_cache.invalidate(&dst);
        self.emit_cdc_edge(
            CdcEventType::EdgeCreated,
            &src,
            &dst,
            &edge_type,
            rank,
            weight,
            &props,
        );
        Ok(EdgeAck {
            src,
            dst,
            etype: edge_type,
            rank,
            shard: out_shard,
            applied_index: idx,
        })
    }

    pub fn remove_edge(&self, src: &str, dst: &str, etype: &str, rank: i64) -> StorageResult<bool> {
        let out_shard = self.raft_nodes.shard_for_vid(src);
        let in_shard = self.raft_nodes.shard_for_vid(dst);
        let log = RaftLog::DelEdge {
            out_shard,
            in_shard,
            src: src.to_string(),
            dst: dst.to_string(),
            etype: etype.to_string(),
            rank,
        };
        match self.raft_nodes.apply(&log) {
            Ok(_) => {
                self.hot_cache.invalidate(src);
                self.hot_cache.invalidate(dst);
                let payload =
                    json!({ "src": src, "dst": dst, "etype": etype, "rank": rank }).to_string();
                self.cdc.emit("default", CdcEventType::EdgeDeleted, payload);
                Ok(true)
            }
            Err(StorageError::EdgeNotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub fn get_neighbors(
        &self,
        vid: &str,
        direction: Direction,
        etypes: &[&str],
    ) -> StorageResult<Vec<Neighbor>> {
        // hot path：尝试 hot cache（命中时需过滤 direction/etypes，但为了简化，
        // 只缓存 "Both"+无筛选 的结果。这里为了使 cache.misses 可控，
        // 我们直接判断 direction==Both 且 etypes.is_empty() 时使用缓存）
        if direction == Direction::Both && etypes.is_empty() {
            if let Some(v) = self.hot_cache.get(vid) {
                return Ok(v);
            }
        }

        let sc = self.raft_nodes.shard_count();
        let shard = graph_codec::vid_hash_shard(vid, sc);
        let mut out: Vec<Neighbor> = Vec::new();

        let mut scan_dir = |out_flag: bool| -> StorageResult<()> {
            let (cf, prefix) = if out_flag {
                (
                    kv_engine::cf_name_out(shard),
                    graph_codec::out_edge_prefix(shard, vid)?,
                )
            } else {
                (
                    kv_engine::cf_name_in(shard),
                    graph_codec::in_edge_prefix(shard, vid)?,
                )
            };
            for (k, v) in self.rocks_db_handles.seek_prefix(&cf, &prefix)? {
                let (_, a, et, rk, b) = if out_flag {
                    let (s, _src, et, rk, dst) = graph_codec::decode_out_edge_key(&k)?;
                    (s, dst, et, rk, _src)
                } else {
                    let (s, _dst, et, rk, src) = graph_codec::decode_in_edge_key(&k)?;
                    (s, src, et, rk, _dst)
                };
                if !etypes.is_empty() && !etypes.contains(&et.as_str()) {
                    continue;
                }
                let (w, props_raw) = graph_codec::decode_edge_value(&v)?;
                let props: BTreeMap<String, Vec<u8>> = props_raw
                    .into_iter()
                    .map(|(k, v)| (k, v.encode_bytes()))
                    .collect();
                out.push(Neighbor {
                    neighbor_vid: a,
                    direction: if out_flag { "out".into() } else { "in".into() },
                    etype: et,
                    rank: rk,
                    weight: weight_to_i64(w),
                    props,
                });
                let _ = b;
            }
            Ok(())
        };

        match direction {
            Direction::Out => scan_dir(true)?,
            Direction::In => scan_dir(false)?,
            Direction::Both => {
                scan_dir(true)?;
                scan_dir(false)?;
            }
        }
        // Both + 无筛选 写入 hot cache
        if direction == Direction::Both && etypes.is_empty() {
            self.hot_cache.insert(vid, out.clone());
        }
        Ok(out)
    }

    pub fn scan_edges(&self, etypes: &[&str], limit: u32, offset: u64) -> StorageResult<Vec<Edge>> {
        let sc = self.raft_nodes.shard_count();
        let mut out: Vec<Edge> = Vec::with_capacity(limit as usize);
        let mut seen: u64 = 0;
        for sh in 0..sc {
            if out.len() >= limit as usize {
                break;
            }
            let rows = self.rocks_db_handles.scan_cf(
                &kv_engine::cf_name_out(sh),
                limit,
                offset.saturating_sub(seen),
            )?;
            for (k, v) in rows {
                if out.len() >= limit as usize {
                    break;
                }
                seen += 1;
                let (_, src, et, rk, dst) = graph_codec::decode_out_edge_key(&k)?;
                if !etypes.is_empty() && !etypes.contains(&et.as_str()) {
                    continue;
                }
                let (w, props_raw) = graph_codec::decode_edge_value(&v)?;
                let props: BTreeMap<String, Vec<u8>> = props_raw
                    .into_iter()
                    .map(|(k, v)| (k, v.encode_bytes()))
                    .collect();
                out.push(Edge {
                    src,
                    dst,
                    etype: et,
                    rank: rk,
                    weight: weight_to_i64(w),
                    props,
                });
            }
        }
        Ok(out)
    }

    pub fn rebalance_16_to_32(&self) -> StorageResult<()> {
        rebalance_16_to_32(&self.raft_nodes)
    }

    /// 辅助：返回各 shard 顶点数统计
    pub fn shard_vertex_counts(&self) -> BTreeMap<u16, u64> {
        self.raft_nodes.shard_counts.lock().clone()
    }
}
