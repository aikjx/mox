# OSS 全维度：存储后端抽象（StorageBackend）+ S3/MinIO adapter 报告

> 日期：2026-09-27 · 网关 mox-platform-gateway-svc（3080）
> 构建：`cargo build -p mox-platform-gateway-svc` 通过（16.26s 增量，无 error / 无 warning）
> 硬约束：禁桩（无真实后端不报"可用"）；local 行为零改变；E2E 实测。

---

## 0. 结论速览

| 项 | 状态 |
| --- | --- |
| `StorageBackend` trait | ✅ 已定义（async，list/create/delete bucket、list/put/get/delete object、health 探测） |
| local 迁移到 trait | ✅ `CloudState` 实现 trait，`/cloud/v1/*` 与 `/api/storage/*` 契约回归通过 |
| S3/MinIO adapter | ⚠️ 已按 env 门控实现（真实 SigV4 签名 HTTP），但**本环境无 MinIO/S3 实例**，未做成功往返 E2E |
| providers 动态化 | ✅ local 恒在；s3 仅在 `MOX_S3_*` 配置齐全时出现 |
| switch | ✅ local 幂等；s3 配置+探测可达才放行，否则 409/502 |
| API-REGISTRY | ✅ 已重生成（236 条，storage 描述更新） |

---

## 1. 依赖探查结论

- workspace **无** `aws-sdk-s3` / `rusoto` / `object_store` / `minio` crate。
- 但网关已具备实现真实 S3 客户端的全部依赖：`reqwest 0.12`（async）、`hmac`、`sha2`、`hex`、`base64`、`async-trait`。
- **关键复用**：`platform/foundation/mox-audit/src/s3.rs` 已存在一份**手写 AWS SigV4 签名的 PutObject 客户端**（支持 MinIO/COS/OBS/OSS endpoint，单元测试验证走真实 HTTP 路径而非占位）。本线的 S3 adapter 复用同一签名算法范式。

---

## 2. StorageBackend trait 设计（新增 `src/storage_backend.rs`）

```rust
pub struct BucketInfo { name, object_count, used_bytes }
pub struct ObjectMeta { key, size }

#[async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    fn id(&self) -> &str;                 // local / s3
    fn kind(&self) -> &str;               // disk / s3
    fn describe(&self) -> Value;           // /providers 卡片
    async fn list_buckets(&self) -> Result<Vec<BucketInfo>, String>;
    async fn create_bucket(&self, name: &str) -> Result<(), String>;
    async fn delete_bucket(&self, name: &str) -> Result<(), String>;
    async fn list_objects(&self, bucket: &str) -> Result<Vec<ObjectMeta>, String>;
    async fn put_object(&self, bucket: &str, key: &str, body: Vec<u8>) -> Result<u64, String>;
    async fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, String>;
    async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), String>;
    async fn health(&self) -> Result<Value, String>;   // 连通性探测
}
```

- local 实现：`impl StorageBackend for CloudState`（fs 逻辑从 handler 内联迁入 trait 方法，行为不变）。
- S3 实现：`S3Backend`，`from_env()` 读取 `MOX_S3_ENDPOINT / MOX_S3_BUCKET / MOX_S3_ACCESS_KEY_ID / MOX_S3_SECRET_ACCESS_KEY / MOX_S3_REGION`；缺任一即 `None`（不暴露）。
- 注册表：`StorageRegistry { local: CloudState, s3: Option<Arc<S3Backend>> }`，在 `modules.rs` 装配。

---

## 3. local 迁移（行为零改变）

- `CloudState` 的 `new() / put_object_text() / root_dir()` 公共 API 保持不变（`dialogue_sediment` 等调用方不受影响）。
- 新增 trait 方法承载 fs 读写；`/cloud/v1/*` handler 输出 JSON 形状与迁移前逐字段一致。

### E2E 回归（实测 2026-09-27，重启新二进制后）

```text
[GET /api/storage/status]
{"code":0,"data":{"bucket_count":2,"entitiesByType":[{"cnt":0,"entity_type":"demo-bucket"},
 {"cnt":3,"entity_type":"dialogue"}],"totalEntities":3,"used_bytes":9248,
 "features":{"object_storage":true,"multi_bucket":true,"s3_compatible_semantics":true,
  "s3_backend_configured":false,"minio":false,...},"s3":"not_configured","provider":"local"}}

[GET /cloud/v1/buckets]            ← 契约形状不变
{"code":0,"data":{"buckets":[{"name":"demo-bucket","object_count":0,"used_bytes":0},
 {"name":"dialogue","object_count":3,"used_bytes":9248}],"root":"...data/storage"}}

[GET /cloud/v1/buckets/dialogue/objects]   ← 契约形状不变
objects: e2e-sed-002.md(1227) / inline-...md(7087) / sess_...md(934)

[对象写读删往返 /cloud/v1]
PUT  demo-bucket/objects/oss-probe.txt body="hello-oss-123" -> size 13
GET  同 key -> "hello-oss-123"
DELETE 同 key -> {"deleted":true}

[GET /api/storage/providers]  -> 仅 local（s3 未配置，不臆造为可用）
[POST /api/storage/switch {"provider":"s3"}] -> 409 "S3 后端未配置（需 MOX_S3_*）"
[GET /api/modules] -> 46 域
```

---

## 4. S3/MinIO adapter 实现状态（诚实标注）

- **已实现**：`S3Backend` 的 SigV4 签名请求（PUT/GET/DELETE/LIST via ListObjectsV2），path-style endpoint，env 门控。签名链（canonical request → string-to-sign → HMAC 派生 signing key）与仓库已验证的 `mox-audit/s3.rs` 一致。
- **未做成功往返 E2E**：本环境无 MinIO/S3 实例（无 docker、无可下载的 minio 二进制）。因此：
  - 未配置 `MOX_S3_*` 时，`providers` 不出现 s3，`status.features.s3_backend_configured=false`。
  - 配置 `MOX_S3_*` 后，`health()` 会发一次真实 ListObjectsV2 探测：可达才在 providers 标 `available:true`，不可达返回错误（不假装可用）。
- **单桶模型**：当前 adapter 以 env 指定的单一 bucket 操作；多桶 create/delete 留待后续（返回明确未启用错误，非静默）。

---

## 5. 改动文件

```
platform/gateway/mox-platform-gateway-svc/src/storage_backend.rs   (新增：trait + S3Backend)
platform/gateway/mox-platform-gateway-svc/src/cloud.rs             (+impl StorageBackend for CloudState)
platform/gateway/mox-platform-gateway-svc/src/admin_storage.rs      (重写：StorageRegistry + 动态 providers/status/switch)
platform/gateway/mox-platform-gateway-svc/src/lib.rs              (+pub mod storage_backend)
platform/gateway/mox-platform-gateway-svc/src/modules.rs            (接线 StorageRegistry)
platform/gateway/mox-platform-gateway-svc/src/actuator.rs          (storage.* 描述更新)
docs/API-REGISTRY.md                                               (重生成 236 条)
reports/markdown/oss-storage-backends.md                          (本报告)
```

---

## 6. 遗留缺口（下一步）

1. **S3 成功往返验证**：需起一个 MinIO（`minio server /data` 或 docker），注入 `MOX_S3_*`，实测 PUT/GET/LIST/DELETE 往返，修正签名/XML 解析细节。
2. **多桶管理**：S3Backend 当前单桶；如需 create/delete bucket，补充 S3 CreateBucket/DeleteBucket 签名请求。
3. **写入路径切换**：当前 `/cloud/v1/*` 系统记录盘固定 local；"active 后端"选择尚为管理面视图，未把对象写流量路由到 s3（需引入运行时后端路由策略 + 并发写一致性决策）。
4. **OSS/COS/华为 OBS**：同为 S3 兼容，endpoint+region 换参即可复用；如需原生 SDK 再评估 `object_store` crate。
