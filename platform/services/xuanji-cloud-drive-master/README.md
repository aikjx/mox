# xuanji-cloud-drive-master

## Intro
Xuanji Cloud Drive L4 Master — 云盘控制面 (Control Plane) 服务。
负责 Volume 节点心跳管理、卷分配、副本管理（quorum）、快照、集群状态。
完全自研实现，无外部存储系统依赖。

## 架构 Master/Volume 控制面+数据面
- **Master (本 crate)**: 控制面。不碰用户数据，仅维护元数据 + 调度。
  - `MasterServer`: 门面，集成 allocator/replica/snapshot/metrics。
  - `VolumeAllocator`: round-robin + 容量最空优先，N 副本跨不同节点。
  - `ReplicaSetManager`: 副本集、写 quorum=N/2+1、读 quorum、health check。
  - `SnapshotManager`: sha256(volume_id+salt+ts) 生成不可伪造快照 ID，软删。
- **Volume (xuanji-cloud-drive-volume)**: 数据面。存真实 chunk 数据，处理读写、自研 RS(2+1) 编码、重建。

## Example 起 3 Volume + Master
```rust,ignore
use xuanji_cloud_drive_master::*;
use std::sync::Arc;

let master = MasterServer::new(MasterConfig::default());
let id_a = master.register_volume("127.0.0.1:8001".into(), 1024*1024);
let id_b = master.register_volume("127.0.0.1:8002".into(), 1024*1024);
let id_c = master.register_volume("127.0.0.1:8003".into(), 1024*1024);
// allocate with replica=3 → spans 3 nodes
let alloc = master.allocate_volume(4096, 3).unwrap();
assert_eq!(alloc.replica_ids.len(), 3);
```

## 错误处理
枚举 `MasterError`：
- `VolumeNotFound` / `NoCapacity` / `ReplicaQuorum`
- `HeartbeatTimeout` / `SnapshotInvalid` / `InvalidReplicaCount`

## 兼容标准 POSIX Filer 对接
上层 POSIX Filer 调 Master.allocate_volume 获得 `replica_addresses`，
之后直接通过 RPC / 本地 channel 调对应 Volume 节点 write_chunk/read_chunk。
Master 不介入数据面 fast path，仅做慢路径控制。

## 扩展点（L5 impl 替换）
- Volume 节点内部 chunk store 可替换 `xuanji_domain_abstractions::ChunkManagerProvider` 的 Mock 实现，接入真实磁盘/对象存储。
- Allocator 策略可继承 trait 扩展。

## FAQ
Q: 为什么不用第三方 RS 库？
A: 部分第三方 RS 库使用 GPL 许可证，存在合规风险。自研 2+1 XOR parity 满足 TR4.5 重建需求。

Q: 单 Master 是否 SPOF？
A: 本 M1 聚焦 L4 单节点可测基线；M2 加 Raft 高可用。

## License
MIT OR Apache-2.0
