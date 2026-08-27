// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 分片 Raft：
//! - 分片数默认 16（2 的幂）。
//! - 每个 shard 归属一个 RaftGroup (Leader/Follower)，最小实现：在单进程内保留 shard 状态、
//!   通过 apply(log) 更新 KvEngine；为集群化保留 `node_role` + `storage_addrs` 外观接口。
//! - RaftLog 枚举：`PutVertex / DelVertex / PutEdge / DelEdge / SplitShard(old, newA, newB)`。
//! - rebalance_16_to_32：创建 16 个新 shard（共 32），迁移一半 VID hash 范围，
//!   结果 max|shard| - min|shard| <= 10% avg。

use crate::error::{StorageError, StorageResult};
use crate::graph_codec::{self, PropValue};
use crate::kv_engine::{self, KvEngine};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RaftLog {
    PutVertex {
        shard: u16,
        vid: String,
        tag: String,
        props: BTreeMap<String, PropValue>,
    },
    DelVertex {
        shard: u16,
        vid: String,
    },
    PutEdge {
        out_shard: u16,
        in_shard: u16,
        src: String,
        dst: String,
        etype: String,
        rank: i64,
        weight: Option<OrderedF64>,
        props: BTreeMap<String, PropValue>,
    },
    DelEdge {
        out_shard: u16,
        in_shard: u16,
        src: String,
        dst: String,
        etype: String,
        rank: i64,
    },
    SplitShard {
        old: u16,
        new_a: u16,
        new_b: u16,
    },
    Noop,
}

/// f64 不能直接 derive Eq，套壳。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OrderedF64(pub f64);
impl PartialEq for OrderedF64 {
    fn eq(&self, o: &Self) -> bool {
        self.0.to_bits() == o.0.to_bits()
    }
}
impl Eq for OrderedF64 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    Leader,
    Follower,
    Candidate,
}

pub struct RaftGroup {
    pub shard: u16,
    pub role: NodeRole,
    pub applied_index: AtomicU64,
    pub storage_addrs: Vec<String>,
}

pub struct ShardRaft {
    pub kv: Mutex<KvEngine>,
    pub shard_count: AtomicU64,
    pub groups: Mutex<BTreeMap<u16, RaftGroup>>,
    pub shard_counts: Mutex<BTreeMap<u16, u64>>,
    pub global_applied: AtomicU64,
}

impl ShardRaft {
    pub fn new(kv: KvEngine, shard_count: u16, storage_addrs: &[String]) -> Self {
        assert!(shard_count.is_power_of_two());
        let mut groups = BTreeMap::new();
        let mut counts = BTreeMap::new();
        let addrs_vec: Vec<String> = storage_addrs.to_vec();
        let n = addrs_vec.len().max(1);
        for shard in 0..shard_count {
            groups.insert(
                shard,
                RaftGroup {
                    shard,
                    role: if (shard as usize).is_multiple_of(n) {
                        NodeRole::Leader
                    } else {
                        NodeRole::Follower
                    },
                    applied_index: AtomicU64::new(0),
                    storage_addrs: addrs_vec.clone(),
                },
            );
            counts.insert(shard, 0u64);
        }
        Self {
            kv: Mutex::new(kv),
            shard_count: AtomicU64::new(shard_count as u64),
            groups: Mutex::new(groups),
            shard_counts: Mutex::new(counts),
            global_applied: AtomicU64::new(0),
        }
    }

    pub fn shard_count(&self) -> u16 {
        self.shard_count.load(Ordering::SeqCst) as u16
    }
    pub fn shard_for_vid(&self, vid: &str) -> u16 {
        graph_codec::vid_hash_shard(vid, self.shard_count())
    }
    pub fn is_leader(&self, shard: u16) -> bool {
        self.groups
            .lock()
            .get(&shard)
            .map(|g| g.role == NodeRole::Leader)
            .unwrap_or(false)
    }

    pub fn ensure_shard(&self, shard: u16) -> StorageResult<()> {
        let exists = {
            let k = self.kv.lock();
            k.contains_shard(shard)
        };
        if !exists {
            let mut k = self.kv.lock();
            if !k.contains_shard(shard) {
                k.add_shard(shard)?;
            }
        }
        let mut g = self.groups.lock();
        g.entry(shard).or_insert_with(|| RaftGroup {
            shard,
            role: NodeRole::Follower,
            applied_index: AtomicU64::new(0),
            storage_addrs: Vec::new(),
        });
        self.shard_counts.lock().entry(shard).or_insert(0);
        Ok(())
    }

    pub fn global_applied_inc(&self) -> u64 {
        self.global_applied.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn recount_shards(&self, up_to: u16) {
        let kv = self.kv.lock();
        let mut cnt = self.shard_counts.lock();
        for s in 0..up_to {
            let prefix = s.to_le_bytes();
            let n = kv
                .seek_prefix(&kv_engine::cf_name_vid_meta(s), &prefix)
                .unwrap_or_default()
                .len() as u64;
            cnt.insert(s, n);
        }
    }

    pub fn apply(&self, log: &RaftLog) -> StorageResult<u64> {
        let mut recount_up_to: Option<u16> = None;
        let mut bump_shard_count: Option<u16> = None;
        match log {
            RaftLog::SplitShard { old, new_a, new_b } => {
                self.ensure_shard(*new_a)?;
                self.ensure_shard(*new_b)?;
                self.ensure_shard(*old)?;
                let cur = self.shard_count();
                let next = (*new_b + 1).max(cur);
                if next == cur * 2 {
                    bump_shard_count = Some(next);
                }
                recount_up_to = Some(next);
            }
            RaftLog::PutVertex { shard, .. } => {
                self.ensure_shard(*shard)?;
            }
            RaftLog::DelVertex { shard, .. } => {
                self.ensure_shard(*shard)?;
            }
            RaftLog::PutEdge {
                out_shard,
                in_shard,
                ..
            } => {
                self.ensure_shard(*out_shard)?;
                self.ensure_shard(*in_shard)?;
            }
            RaftLog::DelEdge {
                out_shard,
                in_shard,
                ..
            } => {
                self.ensure_shard(*out_shard)?;
                self.ensure_shard(*in_shard)?;
            }
            RaftLog::Noop => {
                return Ok(0);
            }
        }
        let r: StorageResult<()> = match log {
            RaftLog::PutVertex {
                shard,
                vid,
                tag,
                props,
            } => self.apply_put_vertex(*shard, vid, tag, props),
            RaftLog::DelVertex { shard, vid } => self.apply_del_vertex(*shard, vid),
            RaftLog::PutEdge {
                out_shard,
                in_shard,
                src,
                dst,
                etype,
                rank,
                weight,
                props,
            } => self.apply_put_edge(
                *out_shard, *in_shard, src, dst, etype, *rank, *weight, props,
            ),
            RaftLog::DelEdge {
                out_shard,
                in_shard,
                src,
                dst,
                etype,
                rank,
            } => self.apply_del_edge(*out_shard, *in_shard, src, dst, etype, *rank),
            RaftLog::SplitShard { old, new_a, new_b } => {
                let r0 = self.apply_split_shard(*old, *new_a, *new_b);
                if r0.is_ok() {
                    if let Some(next) = bump_shard_count {
                        self.shard_count.store(next as u64, Ordering::SeqCst);
                    }
                    if let Some(u) = recount_up_to {
                        self.recount_shards(u);
                    }
                }
                r0
            }
            RaftLog::Noop => Ok(()),
        };
        r?;
        let leader = match log {
            RaftLog::PutVertex { shard, .. } => *shard,
            RaftLog::DelVertex { shard, .. } => *shard,
            RaftLog::PutEdge { out_shard, .. } => *out_shard,
            RaftLog::DelEdge { out_shard, .. } => *out_shard,
            RaftLog::SplitShard { old, .. } => *old,
            RaftLog::Noop => 0,
        };
        if let Some(g) = self.groups.lock().get(&leader) {
            g.applied_index.fetch_add(1, Ordering::SeqCst);
        }
        Ok(self.global_applied.fetch_add(1, Ordering::SeqCst) + 1)
    }

    fn apply_put_vertex(
        &self,
        shard: u16,
        vid: &str,
        tag: &str,
        props: &BTreeMap<String, PropValue>,
    ) -> StorageResult<()> {
        let kv = self.kv.lock();
        let vk = graph_codec::encode_vertex_key(shard, tag, vid)?;
        let vv = graph_codec::encode_vertex_value(tag, props)?;
        let existed = kv
            .get_cf(&kv_engine::cf_name_vid_meta(shard), &vk)?
            .is_some();
        let mut batch = KvEngine::new_batch();
        kv.batch_put_cf(&mut batch, &kv_engine::cf_name_vid_meta(shard), &vk, &vv)?;
        kv.batch_put_cf(&mut batch, &kv_engine::cf_name_vp(shard), &vk, &vv)?;
        drop(kv);
        if !existed {
            *self.shard_counts.lock().entry(shard).or_insert(0) += 1;
        }
        let kv2 = self.kv.lock();
        kv2.write_batch(batch)
    }

    fn apply_del_vertex(&self, shard: u16, vid: &str) -> StorageResult<()> {
        let (vkeys_del, out_del, in_del) = {
            let kv = self.kv.lock();
            let mut vkeys_del: Vec<Vec<u8>> = Vec::new();
            let mut out_del: Vec<(Vec<u8>, u16, Vec<u8>)> = Vec::new();
            let mut in_del: Vec<(Vec<u8>, u16, Vec<u8>)> = Vec::new();
            for (vk, _) in
                kv.seek_prefix(&kv_engine::cf_name_vid_meta(shard), &shard.to_le_bytes())?
            {
                if let Ok((_, _, dec)) = graph_codec::decode_vertex_key(&vk) {
                    if dec == vid {
                        vkeys_del.push(vk);
                    }
                }
            }
            for (ok, _) in kv.seek_prefix(
                &kv_engine::cf_name_out(shard),
                &graph_codec::out_edge_prefix(shard, vid)?,
            )? {
                if let Ok((_, _s, et, rk, dst)) = graph_codec::decode_out_edge_key(&ok) {
                    let dst_shard = graph_codec::vid_hash_shard(&dst, self.shard_count());
                    let ik = graph_codec::encode_in_edge_key(dst_shard, &dst, &et, rk, vid)?;
                    out_del.push((ok, dst_shard, ik));
                }
            }
            for (ik, _) in kv.seek_prefix(
                &kv_engine::cf_name_in(shard),
                &graph_codec::in_edge_prefix(shard, vid)?,
            )? {
                if let Ok((_, _d, et, rk, src)) = graph_codec::decode_in_edge_key(&ik) {
                    let src_shard = graph_codec::vid_hash_shard(&src, self.shard_count());
                    let ok = graph_codec::encode_out_edge_key(src_shard, &src, &et, rk, vid)?;
                    in_del.push((ik, src_shard, ok));
                }
            }
            (vkeys_del, out_del, in_del)
        };
        if vkeys_del.is_empty() && out_del.is_empty() && in_del.is_empty() {
            return Err(StorageError::VidNotFound(vid.to_string()));
        }
        for (_, d, _) in &out_del {
            self.ensure_shard(*d)?;
        }
        for (_, s, _) in &in_del {
            self.ensure_shard(*s)?;
        }
        let removed = vkeys_del.len() as u64;
        {
            let kv = self.kv.lock();
            let mut batch = KvEngine::new_batch();
            for vk in &vkeys_del {
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_vid_meta(shard), vk)?;
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_vp(shard), vk)?;
            }
            for (ok, _, _) in &out_del {
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_out(shard), ok)?;
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_ep(shard), ok)?;
            }
            for (_, dst_shard, ik) in &out_del {
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_in(*dst_shard), ik)?;
            }
            for (ik, _, _) in &in_del {
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_in(shard), ik)?;
            }
            for (_, src_shard, ok) in &in_del {
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_out(*src_shard), ok)?;
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_ep(*src_shard), ok)?;
            }
            kv.write_batch(batch)?;
        }
        if removed > 0 {
            let mut cnt = self.shard_counts.lock();
            let cur = cnt.entry(shard).or_insert(0);
            *cur = cur.saturating_sub(removed);
        }
        Ok(())
    }

    fn apply_put_edge(
        &self,
        out_shard: u16,
        in_shard: u16,
        src: &str,
        dst: &str,
        etype: &str,
        rank: i64,
        weight: Option<OrderedF64>,
        props: &BTreeMap<String, PropValue>,
    ) -> StorageResult<()> {
        let kv = self.kv.lock();
        let ok = graph_codec::encode_out_edge_key(out_shard, src, etype, rank, dst)?;
        let ik = graph_codec::encode_in_edge_key(in_shard, dst, etype, rank, src)?;
        let w = weight.map(|o| o.0);
        let ev = graph_codec::encode_edge_value(w, props)?;
        let mut batch = KvEngine::new_batch();
        kv.batch_put_cf(&mut batch, &kv_engine::cf_name_out(out_shard), &ok, &ev)?;
        kv.batch_put_cf(&mut batch, &kv_engine::cf_name_ep(out_shard), &ok, &ev)?;
        kv.batch_put_cf(&mut batch, &kv_engine::cf_name_in(in_shard), &ik, &ev)?;
        kv.write_batch(batch)
    }

    fn apply_del_edge(
        &self,
        out_shard: u16,
        in_shard: u16,
        src: &str,
        dst: &str,
        etype: &str,
        rank: i64,
    ) -> StorageResult<()> {
        let kv = self.kv.lock();
        let ok = graph_codec::encode_out_edge_key(out_shard, src, etype, rank, dst)?;
        let ik = graph_codec::encode_in_edge_key(in_shard, dst, etype, rank, src)?;
        let present = kv
            .get_cf(&kv_engine::cf_name_out(out_shard), &ok)?
            .is_some();
        if !present {
            return Err(StorageError::EdgeNotFound {
                src: src.into(),
                dst: dst.into(),
                etype: etype.into(),
                rank,
            });
        }
        let mut batch = KvEngine::new_batch();
        kv.batch_del_cf(&mut batch, &kv_engine::cf_name_out(out_shard), &ok)?;
        kv.batch_del_cf(&mut batch, &kv_engine::cf_name_ep(out_shard), &ok)?;
        kv.batch_del_cf(&mut batch, &kv_engine::cf_name_in(in_shard), &ik)?;
        kv.write_batch(batch)
    }

    fn apply_split_shard(&self, old: u16, new_a: u16, new_b: u16) -> StorageResult<()> {
        let current_count = self.shard_count();
        let new_count = (new_b + 1).max(current_count);
        struct Move {
            vk: Vec<u8>,
            vv: Vec<u8>,
            vid: String,
            tag: String,
            target: u16,
            outs: Vec<(Vec<u8>, Vec<u8>, String, i64, String)>,
            ins: Vec<(Vec<u8>, Vec<u8>, String, i64, String)>,
        }
        let moves: Vec<Move> = {
            let kv = self.kv.lock();
            let mut moves = Vec::new();
            let rows = kv.seek_prefix(&kv_engine::cf_name_vid_meta(old), &old.to_le_bytes())?;
            for (vk, vv) in rows {
                let (_, _, vid) = graph_codec::decode_vertex_key(&vk)?;
                let (tag, _) = graph_codec::decode_vertex_value(&vv)?;
                const SPLIT_EXTRA_BIT: u64 = 1u64 << 4;
                let target = if new_count.is_power_of_two() {
                    graph_codec::vid_hash_shard(&vid, new_count)
                } else if (graph_codec::vid_hash_u64(&vid) & SPLIT_EXTRA_BIT) == 0 {
                    new_a
                } else {
                    new_b
                };
                if target == old {
                    continue;
                }
                let mut outs = Vec::new();
                for (ok, ov) in kv.seek_prefix(
                    &kv_engine::cf_name_out(old),
                    &graph_codec::out_edge_prefix(old, &vid)?,
                )? {
                    if let Ok((_, _, et, rk, dst)) = graph_codec::decode_out_edge_key(&ok) {
                        outs.push((ok, ov, et, rk, dst));
                    }
                }
                let mut ins = Vec::new();
                for (ik, iv) in kv.seek_prefix(
                    &kv_engine::cf_name_in(old),
                    &graph_codec::in_edge_prefix(old, &vid)?,
                )? {
                    if let Ok((_, _, et, rk, src)) = graph_codec::decode_in_edge_key(&ik) {
                        ins.push((ik, iv, et, rk, src));
                    }
                }
                moves.push(Move {
                    vk,
                    vv,
                    vid,
                    tag,
                    target,
                    outs,
                    ins,
                });
            }
            moves
        };
        let kv = self.kv.lock();
        let mut batch = KvEngine::new_batch();
        for m in &moves {
            let nk = graph_codec::encode_vertex_key(m.target, &m.tag, &m.vid)?;
            kv.batch_put_cf(
                &mut batch,
                &kv_engine::cf_name_vid_meta(m.target),
                &nk,
                &m.vv,
            )?;
            kv.batch_put_cf(&mut batch, &kv_engine::cf_name_vp(m.target), &nk, &m.vv)?;
            kv.batch_del_cf(&mut batch, &kv_engine::cf_name_vid_meta(old), &m.vk)?;
            kv.batch_del_cf(&mut batch, &kv_engine::cf_name_vp(old), &m.vk)?;
            for (ok, ev, et, rk, dst) in &m.outs {
                let new_out_shard = if new_count.is_power_of_two() {
                    graph_codec::vid_hash_shard(&m.vid, new_count)
                } else {
                    m.target
                };
                let new_dst_shard = if new_count.is_power_of_two() {
                    graph_codec::vid_hash_shard(dst, new_count)
                } else {
                    new_b
                };
                let nok = graph_codec::encode_out_edge_key(new_out_shard, &m.vid, et, *rk, dst)?;
                let nik = graph_codec::encode_in_edge_key(new_dst_shard, dst, et, *rk, &m.vid)?;
                kv.batch_put_cf(&mut batch, &kv_engine::cf_name_out(new_out_shard), &nok, ev)?;
                kv.batch_put_cf(&mut batch, &kv_engine::cf_name_ep(new_out_shard), &nok, ev)?;
                kv.batch_put_cf(&mut batch, &kv_engine::cf_name_in(new_dst_shard), &nik, ev)?;
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_out(old), ok)?;
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_ep(old), ok)?;
                let dst_old_shard = graph_codec::vid_hash_shard(dst, current_count);
                let ik_old = graph_codec::encode_in_edge_key(dst_old_shard, dst, et, *rk, &m.vid)?;
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_in(dst_old_shard), &ik_old)?;
            }
            for (ik, iv, et, rk, src) in &m.ins {
                let new_dst_shard = m.target;
                let new_src_shard = if new_count.is_power_of_two() {
                    graph_codec::vid_hash_shard(src, new_count)
                } else {
                    new_a
                };
                let nik = graph_codec::encode_in_edge_key(new_dst_shard, &m.vid, et, *rk, src)?;
                let nok = graph_codec::encode_out_edge_key(new_src_shard, src, et, *rk, &m.vid)?;
                kv.batch_put_cf(&mut batch, &kv_engine::cf_name_in(new_dst_shard), &nik, iv)?;
                kv.batch_put_cf(&mut batch, &kv_engine::cf_name_out(new_src_shard), &nok, iv)?;
                kv.batch_put_cf(&mut batch, &kv_engine::cf_name_ep(new_src_shard), &nok, iv)?;
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_in(old), ik)?;
                let src_old_shard = graph_codec::vid_hash_shard(src, current_count);
                let ok_old = graph_codec::encode_out_edge_key(src_old_shard, src, et, *rk, &m.vid)?;
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_out(src_old_shard), &ok_old)?;
                kv.batch_del_cf(&mut batch, &kv_engine::cf_name_ep(src_old_shard), &ok_old)?;
            }
        }
        kv.write_batch(batch)
    }
}

pub fn rebalance_16_to_32(raft: &Arc<ShardRaft>) -> StorageResult<()> {
    let current = raft.shard_count();
    if current != 16 {
        return Err(StorageError::InvalidArgument(format!(
            "rebalance_16_to_32 requires 16 got {current}"
        )));
    }
    for i in 0..16u16 {
        raft.apply(&RaftLog::SplitShard {
            old: i,
            new_a: i,
            new_b: i + 16,
        })?;
    }
    // Finalize: bump shard_count to 32 and recount all shards for accurate per-shard stats.
    raft.shard_count.store(32, Ordering::SeqCst);
    raft.recount_shards(32);
    let now = raft.shard_count();
    if now != 32 {
        return Err(StorageError::Internal(format!(
            "rebalance expect 32 got {now}"
        )));
    }
    Ok(())
}
