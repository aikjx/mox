# mox-domain-abstractions

## 1. Intro — L5 定位

mox-domain-abstractions 是 Mox V5 统一算子平台 **AIS L5（纯 trait 定义层）**。
本 crate 不包含任何外部存储 I/O，只描述 Cloud Drive 与 Graph 两大引擎域的 10 个核心 provider
trait 的方法签名与契约。每个 trait 附带一个纯内存、基于 parking_lot::Mutex<BTreeMap> 的
MockXxxProvider 实现，用于 L4 适配器集成前的单元测试（TDD 先 GREEN）。

L5 与 L4 的关系：

`
L3 Business (business-catalog / kg-hub / ...)
   │ uses
L4 Adapter (L4 S3 Adapter / L4 nGQL Adapter / L4 POSIX Adapter ...)
   │ impls
L5 Traits ← 本 crate（ObjectStorageProvider / GraphQueryProvider / …）
`

## 2. Trait 列表（10 条）

Cloud Drive（5 条）：

| # | Trait 名 | 方法数 | 核心职责 | Mock 实现 |
|---|----------|--------|----------|-----------|
| 1 | ObjectStorageProvider | 9 | S3 对象：put/get/delete/list/multipart/head | MockObjectStorageProvider |
| 2 | MetaStorageProvider | 12 | POSIX 元数据：mkdir/symlink/xattr/chmod/statfs | MockMetaStorageProvider |
| 3 | ChunkManagerProvider | 7 | 块生命周期：alloc/write/read/delete/gc/stats | MockChunkManagerProvider |
| 4 | IamProvider | 9 | 身份权限：用户/角色/策略/STS assume-role | MockIamProvider |
| 5 | QuotaProvider | 8 | 配额：用户/目录 bytes 与 objects 双维度 | MockQuotaProvider |

Graph（5 条）：

| # | Trait 名 | 方法数 | 核心职责 | Mock 实现 |
|---|----------|--------|----------|-----------|
| 6 | GraphQueryProvider | 8 | 图查询：vertex/edge/neighbor/nGQL/Cypher | MockGraphQueryProvider |
| 7 | GraphMetaProvider | 11 | 图 DDL：space/tag/edge_type/hosts | MockGraphMetaProvider |
| 8 | GraphAlgoSingleProvider | 7 | 7 算法护栏：PPR/CNM/BC/HC/DC/Density/rawBDE | MockGraphAlgoSingleProvider |
| 9 | PartitionRouterProvider | 7 | 分片：vid→shard→addr/rebalance | MockPartitionRouterProvider |
| 10 | CdcPublisherProvider | 9 | 变更捕获：vertex/edge 事件 + subscribe/lag | MockCdcPublisherProvider |

## 3. Example

`
ust
use mox_domain_abstractions::{ObjectStorageProvider, MockObjectStorageProvider};
use bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let s3 = MockObjectStorageProvider::default();
    let _etag = s3.put_object("b", "hello.txt", Bytes::from("世界")).await?;
    let data = s3.get_object("b", "hello.txt").await?;
    assert_eq!(&data[..], "世界".as_bytes());
    Ok(())
}
`

## 4. 错误处理约定

**所有 trait 方法返回类型统一为 Result<T, Box<dyn Error + Send + Sync + 'static>>**。
契约规则：

- L5 不定义全局 	hiserror 枚举（避免把具体实现泄漏到抽象层）；
- L4 适配器可向上抛任何 impl Error + Send + Sync 的具体错误；
- 业务层（L3）用 pattern match 或 .context() 统一包装；
- Mock*Provider 在语义上区分 "不存在"（Err("not found".into())）与 "协议错误"；
- trait 本身通过方法命名区分操作幂等（delete_* 多次 = OK，create_* 同名冲突 = Err）。

## 5. 兼容标准

| Trait | 兼容标准 | 备注 |
|-------|----------|------|
| ObjectStorageProvider | **AWS S3 v20060301**（Multipart Upload / ETag / List prefix+continuation） | 不依赖 rusoto / aws-sdk |
| MetaStorageProvider | **POSIX 1003.1**（mkdir/rmdir/symlink/readlink/xattr/chmod/chown/statfs） | 可选 fuse-abi L4 实现 |
| ChunkManagerProvider | 与 S3/POSIX 解耦的内部块 API | 可叠加 erasure-coding L4 |
| GraphQueryProvider | **nGQL (Nebula Graph)** 子集 + **OpenCypher 9** | xecute_ngql/xecute_cypher 双通道 |
| GraphMetaProvider | nGQL DDL CREATE/DROP SPACE/TAG/EDGE | |
| GraphAlgoSingleProvider | SPEC 7-algo guardrail（PPR / CNM / BC / HC / DC / Density / rawBDE） | 严格对齐 Q4 算法清单 |
| PartitionRouterProvider | 一致性 hash + 静态分片 | |
| CdcPublisherProvider | Kafka-like topic / consumer group / offset commit | |
| IamProvider | AWS IAM 风格（Policy Statement effect Allow/Deny）+ STS AssumeRole | |
| QuotaProvider | POSIX quota（user/group）+ Lustre-style directory quota | |

## 6. 扩展点 — L4 impl 方式

要新增一个真实实现（例如 L4 MinIO Adapter）：

1. 在独立 crate（比如 mox-l4-minio）的 Cargo.toml 加依赖：
   `	oml
   mox-domain-abstractions = { path = "../mox-domain-abstractions" }
   `
2. 在代码中 #[async_trait] impl ObjectStorageProvider for MinioAdapter { ... }
3. 用 cargo test -p mox-l4-minio 跑 L4 自己的集成测试；
4. 可直接复用 L5 的 50 条契约测试（把 Mock*Provider 替换成 MinioAdapter）来验证 L4 是否满足抽象契约。

L5 自身 eatures = ["serde"] 打开后，所有数据结构（Vertex、Edge、CdcEvent 等）会启用
Serialize/Deserialize，方便 L4 与 Kafka/Redis/HTTP 网关直接复用。

## 7. FAQ

**Q：为什么不把具体的 SeaweedFS / JuiceFS / Nebula / Neo4j driver 放进 L5？**
A：L5 是纯抽象层，AC-18 自研边界要求严禁绑定具体三方存储。把 driver 放在 L4 crate。

**Q：#[async_trait] 会带来运行时开销吗？**
A：与手写 Pin<Box<dyn Future>> 等价；sync-trait 为 L4 提供了跨线程 + 多 impl 的可插拔性，收益远大于一次 Box。

**Q：为什么 Quota 需要 8 个方法而不是简单的一对 get/set？**
A：除了用户级配额，还有目录级（Lustre-style）；除了读写配额，还有
check_*_allowed 用于在 L4 写路径上提前拦截（否则每次写都要 round-trip 到元数据层）。

**Q：GraphAlgoSingleProvider 为什么只暴露 7 个算法？**
A：SPEC 7-algo guardrail（PPR / CNM / Betweenness / Harmonic-Closeness /
Degree / Density / raw Bidirectional-Expand）。新增算法需走 SPEC 变更，不允许在 L5 直接开后门。

**Q：Mock 的线程安全如何？**
A：所有 Mock*Provider 都用 parking_lot::Mutex（not std::sync::Mutex），
Send + Sync，tokio 多线程 runtime 下可安全共享；但不提供跨进程语义。

## 8. License

Dual-licensed under **MIT OR Apache-2.0** at your option.

`
MIT License
Apache-2.0 License
`

See LICENSE-MIT / LICENSE-APACHE at workspace root for details.
