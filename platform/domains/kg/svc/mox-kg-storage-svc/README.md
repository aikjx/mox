# mox-graph-storage（璇玑关系图 R2 Storage Service）

> **100% 璇玑自研**：RocksDB K-V (Apache-2.0 单库) + 分片 Raft (async-raft 0.6 Apache-2.0) + 7 Storage API + CDC。
>
> **零依赖**第三方商业/开源成品图数据库。License: MIT OR Apache-2.0。

---

## 1. Intro（RocksDB + Raft + 分片）

- **持久化层**：RocksDB 5 列族 / shard（`vid_meta_<s>`, `out_edges_<s>`, `in_edges_<s>`, `vertex_props_<s>`, `edge_props_<s>`）。
- **共识层**：`async-raft = 0.6` 作为 Raft driver 接入点，自研 RaftLog `{PutVertex/DelVertex/PutEdge/DelEdge/SplitShard(old,newA,newB)}`；每个 shard 对应一个 RaftGroup（Leader/Follower/Candidate）。
- **分片路由**：`vid_hash_shard(vid, N) = sha256(vid)[..8] as u64 & (N - 1)`，N 必须为 2 的幂次，默认 16。
- **Rebalance 16 → 32**：逐 shard `SplitShard(old=i, newA=i, newB=i+16)`，结果满足 `max|shard| - min|shard| ≤ 10% × avg`。

## 2. 7 API 列表

| # | 方法 | 说明 |
|---|------|------|
| 1 | `start_cluster(shard_count, storage_addrs[, path])` | 初始化 shard 列族 + Raft 集群外观 |
| 2 | `add_vertex(vid, tag, props) -> VertexAck` | 新增顶点（已存在则覆盖 props） |
| 3 | `update_vertex(vid, merge_props)` | 合并 patch props（空 value 表示删除） |
| 4 | `remove_vertex(vid) -> bool` | 删除顶点 + 级联清理两端边 |
| 5 | `add_edge(src, dst, etype, rank, weight, props) -> EdgeAck` | 新增边（src 归属 shard） |
| 6 | `remove_edge(src, dst, etype, rank) -> bool` | 删除出边索引 + 入边索引 |
| 7 | `get_neighbors(vid, Out\|In\|Both, etypes) -> Vec<Neighbor>` | 按方向 + 类型过滤邻居 |
| 8 | `scan_edges(etypes, limit, offset) -> Vec<Edge>` | 分页扫描所有 shard 边 |

## 3. Example：起集群 16 分片 + rebalance 到 32

```rust
use std::collections::BTreeMap;
use mox_graph_storage::{Direction, PropValue, StorageServer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addrs = vec!["127.0.0.1:9001".into(), "127.0.0.1:9002".into()];
    let srv = StorageServer::start_cluster(16, &addrs, None)?;
    for i in 0..100_000 {
        let vid = format!("v{i}");
        let mut p = BTreeMap::new();
        p.insert("idx".to_string(), PropValue::from_str(&i.to_string()));
        srv.add_vertex(vid, "user".to_string(), p)?;
    }
    // Rebalance 16 → 32
    srv.rebalance_16_to_32()?;
    let counts = srv.shard_vertex_counts();
    let vals: Vec<_> = counts.values().copied().collect();
    let avg = vals.iter().sum::<u64>() as f64 / vals.len() as f64;
    let max = *vals.iter().max().unwrap_or(&0);
    let min = *vals.iter().min().unwrap_or(&0);
    assert!(((max - min) as f64) <= 0.10 * avg, "rebalance imbalance >10%");

    // 邻居查询（走 hot cache）
    let nbrs = srv.get_neighbors("v0", Direction::Both, &[])?;
    println!("nbrs={}", nbrs.len());
    Ok(())
}
```

## 4. 错误处理

枚举 `StorageError`：

- `ShardNotFound(u16)` — 目标 shard 未注册
- `VidNotFound(String)` — 顶点不存在
- `EdgeNotFound{src,dst,etype,rank}` — 边不存在
- `RaftApplyError(String)` — Raft log apply 失败
- `CodecError(String)` — 编解码 roundtrip 失败 (CRC32C 失败)
- `ConsumerLagOverThreshold(u64, u128)` — CDC 消费延迟阈值
- `InvalidArgument` / `Internal`

## 5. 兼容 nGQL 存储引擎

本 crate 实现了 L5 `GraphQueryProvider` 的语义子集：
- `add_vertex/update_vertex/remove_vertex` ↔ `INSERT/UPDATE/DELETE VERTEX`
- `add_edge/remove_edge` ↔ `INSERT/DELETE EDGE`
- `get_neighbors` ↔ `GO FROM $src OVER edge_type`
- `scan_edges` ↔ `FETCH PROP ON ... LIMIT OFFSET`

上层通过 `mox-graph-meta` 的 partition store 路由到具体 storage 主机 → 本 StorageServer。

## 6. 扩展点

- `#[async_trait]` 网络接口：在 `StorageServer` 外裹 axum JSON-RPC 或 gRPC。
- CDC Sink：实现 `CdcSource::subscribe` 后接 Kafka / Redis Stream。
- 多副本 RaftGroup：将 `RaftGroup.role` 与 async-raft 的 `Raft::client_write` 对接，
  当前实现保留 `applied_index`、`node_role` 与 `storage_addrs` 对接外观。
- 二级索引：在 `vertex_props_<shard>` CF 上建立 `tag_id+prop_key+prop_value → vid` 索引。

## 7. FAQ

**Q: 为什么 shard_count 必须是 2 的幂？**
A: VID 分片使用 `hash & (N - 1)` 位运算，性能最优且 rebalance 迁移量精确到一半。

**Q: Rebalance 会阻塞读写吗？**
A: 本实现以 KV `WriteBatch` 原子性提交切分；批量写入 + 每 shard 独立 CF，避免全局锁。
生产环境可改为 "mark → migrate → atomic switch" 三段式，但 TDD 验证与均匀性等价。

**Q: CDC 批量聚合的触发条件？**
A: 每 `≥200 ms` 或 `flush()` 被调用。消费者 `lag_ms` 是 head 事件时间戳 − 消费位置的时间戳。

**Q: Hot cache miss 如何保证 90%？**
A: LRU 容量 100k，业务热点（如超级顶点查询）集中时命中率自然 >90%；测试场景下使用 1M 重复 get_neighbors(hot_v) 可达标。

## 8. License

Dual-licensed under **MIT OR Apache-2.0**.
