# xuanji-standards — Xuanji V5 §3 10 标准矩阵测试骨架

> 10 Standards Matrix: POSIX / AWS SigV4 / CRC32C+S3 ETag / RFC 5424 / FIPS HMAC-SHA256 /
> nGQL 60% / openCypher 20% / ISO GQL / AIS 七层 DIP / 等保三级 hash_chain
>
> License: `MIT OR Apache-2.0`  
> Crate status: **测试骨架先交付（RED → 160 GREEN / 40 FUTURE-ignore）**

---

## 1. 简介 (Intro & 10 标准列表)

本 crate 定义了 Xuanji V5 平台对外承诺的 **10 项行业/国际标准接口契约**，并以纯 Rust
单元测试矩阵的方式独立可运行（`cargo test -p xuanji-standards --test t14_standards_matrix`）。

| # | 标准 / 规范 | 模块 | 测试数 | 状态 (本期) | 里程碑 |
|---|------------|------|-------:|:-----------:|:------:|
| 1 | **POSIX IEEE 1003.1** (元数据契约) | [`posix_skeleton`] | 22 | 17 ✓ / 5 ⏱ | M3 |
| 2 | **AWS S3 Signature Version 4** 纯自研签名 | [`sigv4`] | 30 | **30 ✓** | 现交付 |
| 3 | **CRC32C (Castagnoli)** + **S3 Multipart ETag** | [`etag_crc32c`] | 20 | **20 ✓** | 现交付 |
| 4 | **RFC 5424** Syslog 审计事件格式化 | [`rfc5424`] | 10 | **10 ✓** | 现交付 |
| 5 | **FIPS 140-3 HMAC-SHA256** (RFC 4231 向量) | [`fips_hmac`] | 10 | **10 ✓** | 现交付 |
| 6 | **nGQL** (NebulaGraph DML/DDL 60%) | [`ngql_skeleton`] | 22 | 17 ✓ / 5 ⏱ | R3 |
| 7 | **openCypher** (Cypher 核心读写 20%) | [`cypher_skeleton`] | 22 | 17 ✓ / 5 ⏱ | R3 |
| 8 | **ISO GQL** 标准子集 (ISO/IEC 39075) | [`gql_skeleton`] | 22 | 12 ✓ / 10 ⏱ | 未来 |
| 9 | **AIS 七层 DIP** (L1 UI … L7 Infra 解耦) | [`ais_skeleton`] | 22 | 12 ✓ / 10 ⏱ | L7 落地 |
| 10 | **等保三级** hash_chain 审计不可篡改链 | [`dengbao_skeleton`] | 20 | 15 ✓ / 5 ⏱ | M2/R3 |
|  | **合计** | — | **200** | **160 ✓ / 0 ✗ / 40 ⏱** | — |

⏱ = `#[ignore]`（占位，对应 milestone 实现后替换真实 L4/L7 依赖并取消 ignore）

---

## 2. AWS S3 SigV4 签名算法说明 + 测试向量对照

[`sigv4::sigv4_auth_header`] 是 **纯自研** 的 SigV4 实现（不引 `aws-sig-auth` / `aws-sdk-s3`），
严格按 AWS 官方三步流程：

```
CanonicalRequest → StringToSign → Signature
```

* CanonicalRequest = `METHOD\nURI\nQUERY\nHEADERS\nSIGNED_HEADERS\nPAYLOAD_SHA256`
* StringToSign     = `AWS4-HMAC-SHA256\nDATETIME\nDATE/REGION/SERVICE/aws4_request\nSHA256(CANONICAL_REQ)`
* Derived keys:  `kDate = HMAC("AWS4"+SK, Date)` → `kRegion` → `kService` → `kSigning`
* Signature      = `HMAC(kSigning, StringToSign)` hex lowercase

### 注入时间的可重放向量

本 crate 的 30 条 SigV4 测试均使用固定 `now_date=Some("20150830")` /
`now_datetime=Some("20150830T123600Z")`，保证在任何机器上得到的签名完全一致。
关键基线凭据：

```
AKIDEXAMPLE / wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY
region = us-east-1, service = service
```

30 条覆盖：GET/POST/PUT/HEAD/DELETE 方法、URI 编码、query 排序、host header 大小写、
不同 region/service/date/AK/SK 隔离、签名格式、长度、十六进制大小写、查询特殊字符、
标头数量等。

---

## 3. Example 快速上手

```rust
use xuanji_standards::sigv4;
let (auth, x_amz_date) = sigv4::sigv4_auth_header(
    "AKID", "SECRET", "us-east-1", "s3",
    "GET", "/bucket/key", &[],
    &[("host", "bucket.s3.amazonaws.com")],
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    Some("20260101"), Some("20260101T000000Z"),
);
// Authorization: AWS4-HMAC-SHA256 Credential=AKID/20260101/us-east-1/s3/aws4_request, SignedHeaders=host, Signature=...
```

CRC32C + 分片 ETag：

```rust
use xuanji_standards::etag_crc32c;
assert_eq!(etag_crc32c::crc32c_checksum(b"123456789"), 0xE3069283); // RFC 3720
let final_etag = etag_crc32c::etag_multipart(&["d41d8cd98f00b204e9800998ecf8427e", "0cc175b9c0f1b6a831c399e269772661"]);
assert!(final_etag.ends_with("-2"));
```

---

## 4. 错误处理

* 纯算法模块（SigV4 / CRC32C / MD5 / FIPS / RFC5424）无返回错误：输入必须是 UTF-8
  切片，所有格式化通过 `String` 返回。
* Mock 骨架层 (POSIX / nGQL / Cypher / GQL / AIS) 使用 `anyhow::Result` 或
  `std::io::Result`，保持与 L5 trait 一致的错误语义。
* 标准未覆盖边界：明确以 `#[ignore]` 标注，不在未来版本前静默造假数据。

---

## 5. 扩展（如何替换为真实 L4 实现？）

当前 6 大骨架模块（POSIX/nGQL/Cypher/GQL/AIS/等保）均以
`xuanji-domain-abstractions` 中的 `Mock*Provider` 为底层 in-memory 实现。
里程碑到来时按以下方式替换零侵入：

```rust
// 例：NgqlRunner trait 已标准化，未来接 NebulaGraph 只需
pub struct NebulaNgqlRunner { pool: nebula::ConnectionPool }
#[async_trait] impl NgqlRunner for NebulaNgqlRunner {
    async fn execute_ngql(&self, space: &str, n: &str) -> anyhow::Result<QueryResultSet> {
        let raw = self.pool.session().unwrap().execute(space, n).await?;
        Ok(raw.try_into()?)
    }
}
```

所有 200 条测试 **不需要修改用例断言**（只取消 ignore，替换 mock 构造函数）。

---

## 6. FAQ

**Q1：SigV4 实现为何不直接用 `aws-sig-auth` crate？**
A1：L6 标准层必须无外部厂商 SDK 绑定，确保将来切到任何 S3 兼容后端（自建 Ceph RGW /
阿里云 OSS / 腾讯 COS / 华为 OBS）时签名实现相同、行为一致。

**Q2：CRC32C / MD5 为何要在 crate 内实现，而不是 `crc32c = "0.6"` 或 `md-5`？**
A2：为了全仓库 License 合规（T16）且避免触发 workspace 重新解析
（rocksdb/async-raft 等 legacy 约束无法通过新 crate 引入）。算法均为公开 RFC 标准，
与 RFC 3720 / RFC 1321 / AWS S3 官方向量三重对齐。

**Q3：40 条 ignore 如何跟踪？**
A3：每条 `#[ignore = "..."]` 含明确的里程碑标签（M2/M3/R3/等保），
`platform/services/xuanji-standards/tasks.md` 有 TR14 子任务可追溯。

**Q4：为什么 workspace 成员里没有 `xuanji-graph-meta`？**
A4：该 crate 声明了 crates.io 不存在的 `rocksdb 0.22 bundled` 特性，无法通过任何
workspace 解析。已移到单独目录保留（业务源码仍在），待 L4 团队修复依赖后重新加入。

---

## 7. License

Dual-licensed under **MIT OR Apache-2.0** at your option (matching the Xuanji V5 workspace
`workspace.package.license = "MIT"` 并保留 Apache-2.0 以便和上游 Rust crates 生态兼容)。

本 crate 全部内部依赖均为：MIT / Apache-2.0 / BSD-2-Clause / BSD-3-Clause / ISC /
Unicode-DFS-2016，与 `deny.toml` 白名单严格一致。
