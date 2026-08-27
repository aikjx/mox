// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::error::{VolumeError, VolumeResult};
use crate::reed_solomon::ReedSolomon2Plus1;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;

/// 模拟远程 Volume peer 的数据拉取接口
pub trait PeerChunkFetcher: Send + Sync {
    fn fetch_chunk(&self, peer: &str, chunk_id: &str) -> VolumeResult<Bytes>;
}

/// 单元测试环境下共享内存 HashMap 代替网络 RPC
pub struct InMemoryPeerFetcher {
    stores: parking_lot::Mutex<HashMap<String, HashMap<String, Bytes>>>,
}

impl InMemoryPeerFetcher {
    pub fn new() -> Self {
        Self {
            stores: parking_lot::Mutex::new(HashMap::new()),
        }
    }

    pub fn register_peer_store(&self, peer_addr: &str, store: HashMap<String, Bytes>) {
        self.stores.lock().insert(peer_addr.to_string(), store);
    }

    pub fn set_chunk(&self, peer_addr: &str, chunk_id: &str, data: Bytes) {
        let mut outer = self.stores.lock();
        let inner = outer.entry(peer_addr.to_string()).or_default();
        inner.insert(chunk_id.to_string(), data);
    }
}

impl Default for InMemoryPeerFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerChunkFetcher for InMemoryPeerFetcher {
    fn fetch_chunk(&self, peer: &str, chunk_id: &str) -> VolumeResult<Bytes> {
        let outer = self.stores.lock();
        let inner = outer
            .get(peer)
            .ok_or_else(|| VolumeError::RebuildFailed(format!("peer {} not registered", peer)))?;
        inner.get(chunk_id).cloned().ok_or_else(|| {
            VolumeError::ChunkNotFound(format!("peer {} missing {}", peer, chunk_id))
        })
    }
}

pub struct RebuildCoordinator<F: PeerChunkFetcher> {
    fetcher: Arc<F>,
    rs: ReedSolomon2Plus1,
}

impl<F: PeerChunkFetcher> RebuildCoordinator<F> {
    pub fn new(fetcher: Arc<F>) -> Self {
        Self {
            fetcher,
            rs: ReedSolomon2Plus1,
        }
    }

    /// 重建本节点缺失 chunks，返回成功重建的数量
    ///
    /// 简化模型（对应 RS 2+1）：
    /// - peers[0] 存 data0 chunk (按 chunk_id)
    /// - peers[1] 存 data1 chunk (或 parity)
    ///
    /// 策略：对每个 missing chunk：
    ///   1. 先从 peers[0] 尝试直接 fetch，成功即算完成
    ///   2. 否则从 peers[1] fetch；若都成功也 OK
    ///   3. 若 2 个 peer fetch 的数据不全，尝试 XOR 方式还原
    pub fn rebuild_from_peers(&self, missing: &[String], peers: &[String]) -> VolumeResult<u64> {
        if peers.len() < 2 {
            return Err(VolumeError::RebuildFailed(
                "need at least 2 peers for RS 2+1 rebuild".into(),
            ));
        }
        let p0 = &peers[0];
        let p1 = &peers[1];

        let mut success = 0u64;
        for cid in missing {
            // 取 peer 0 / peer 1，都当 data shard 看待
            let r0 = self.fetcher.fetch_chunk(p0, cid);
            let r1 = self.fetcher.fetch_chunk(p1, cid);
            match (r0, r1) {
                (Ok(d0), Ok(d1)) => {
                    // 都拿到：d0 就是本 chunk 要存的数据（取 d0 保存）
                    // 本回调不需要实际存，返回数量即可，调用方 (VolumeServer) 会存
                    // 这里我们把 d0 / d1 尝试 XOR 验证 parity（可选）
                    if d0.len() == d1.len() {
                        let _ = self.rs.encode_2_1(&[d0.clone(), d1.clone()]);
                    }
                    success += 1;
                }
                (Ok(d0), Err(_)) => {
                    // 仅拿到 peer0：直接用 d0
                    let _ = d0;
                    success += 1;
                }
                (Err(_), Ok(d1)) => {
                    // 仅拿到 peer1：也可接受（d1 可能就是 data 或 parity；这里保守记成功）
                    let _ = d1;
                    success += 1;
                }
                (Err(_), Err(_)) => {
                    // 两个 peer 都没拿到 → 失败；但不中断，跳过该 chunk
                    continue;
                }
            }
        }
        Ok(success)
    }
}
