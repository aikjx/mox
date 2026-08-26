# mox-cloud-drive-volume

## Intro
Mox Cloud Drive L4 Volume — 云盘数据面 (Data Plane) 服务。
负责真实 chunk 数据的存储、读写、CRC 校验、自研 RS(2+1 XOR) 纠删码、peer 重建。
完全自研，无外部存储系统 / GPL 依赖。

## 架构 Master/Volume 控制面+数据面
- **Volume (本 crate)**: 数据面，存真实数据。
  - `VolumeServer`: 门面，管理 in-memory / 可插拔 ChunkManagerProvider store。
  - `ReedSolomon2Plus1`: 自研 2+1 XOR，K=2 data, M=1 parity, 可容忍任意 1 块丢失。
  - `RebuildCoordinator`: 向 2 peers 拉数据，缺 1 块则 XOR parity 还原。
- **Master (mox-cloud-drive-master)**: 控制面，调度与心跳。

## Example 起 3 Volume + Master
```rust,ignore
use mox_cloud_drive_volume::*;
use bytes::Bytes;

let v = VolumeServer::new("v-1".into(), 1024*1024);
let ack = v.write_chunk("c1", Bytes::from_static(b"hello")).unwrap();
let data = v.read_chunk("c1").unwrap();
assert_eq!(&data[..], b"hello");

// RS 2+1 encode
let rs = ReedSolomon2Plus1;
let shards = rs.encode_2_1(&[Bytes::from("abc"), Bytes::from("def")]).unwrap();
// 丢 1 块可重建
let mut lost = [Some(shards[0].clone()), None, Some(shards[2].clone())];
let restored = rs.decode_2_1(lost).unwrap();
assert_eq!(&restored[1][..], b"def");
```

## 错误处理
枚举 `VolumeError`：
- `ChunkNotFound` / `CapacityExceeded` / `IOError`
- `RebuildFailed` / `CrcMismatch`

## 兼容标准 POSIX Filer 对接
上层 POSIX Filer 先向 Master 申请 allocation → 得到 replica addresses，
再直接调用 volume 的 `write_chunk/read_chunk/delete_chunk`。
数据面路径无 Master 介入，低延迟。

## 扩展点（L5 impl 替换）
- `VolumeServer::with_chunk_provider(Arc<dyn ChunkManagerProvider>)`
  可把 in-memory store 换成真实磁盘 / 对象存储实现。
- `PeerChunkFetcher` trait 可替换为真实 gRPC / HTTP 拉取。

## FAQ
Q: 为什么不用常见的第三方 RS 库？
A: 存在 GPL 许可证风险。2+1 XOR 是数学上可证明的纠删码：p = d0^d1，则任一块丢失可由另两块 XOR 还原。

Q: 支持更大 K/M 吗？
A: M1 基线 K=2/M=1；M2 可扩展为通用 K+M XOR 矩阵。

## License
MIT OR Apache-2.0
