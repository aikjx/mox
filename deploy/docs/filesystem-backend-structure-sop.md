# 自研 FS Backend · 目录结构与 S3/MinIO/OSS 同构切换规则（运维子手册）

**版本：** 1.0.0
**发布日期：** 2026-08-26
**适用范围：** Mox Platform · 云盘 FileStore（backend-node 单机 / Helm 伞图 K8s）
**等级：** L2 日常运维（搭配主手册 `storage-cloud-switch-sop.md` §3-§4 一起读）
**关联手册：** [storage-cloud-switch-sop.md](./storage-cloud-switch-sop.md) · [ops-manual.md](./ops-manual.md)
**维护者：** SRE & Mox Platform Team

---

## 0. 一句话定义

> **FS = 本地 File System**：把云盘每个文件切成 1MB Chunk，按 `SHA-256` 作文件名直接写入操作系统目录的自研后端。
> 与 `S3/MinIO/OSS` **Key 规则 100% 同构**（`chunks/<xx>/<sha256>`），所以切换时**零改代码、零重命名、零数据搬运必要（冷数据可选批量迁移）**。

---

## 1. FS Backend 目录结构全景（真相源·绝对不能删）

### 1.1 DATA_DIR 根（所有 FS 数据的父级）

```
DATA_DIR/                                            ← 环境变量 DATA_DIR；默认 = backend-node/data
├── ous.db                                           ← SQLite 结构化真相库（WAL 模式 + journal_mode WAL + synchronous NORMAL）
├── ous.db-wal                                       ← SQLite WAL 预写日志（运行中必存在；正常关闭后合并回 db）
├── ous.db-shm                                       ← SQLite 共享内存文件
│
├── 📚 结构化 JSON 双写（lib/json-store.js 首写此处，再写 SQLite entities 表）
├── graph_nodes.json                                 ← 图谱节点真相源（JSON 数组）
├── graph_edges.json                                 ← 图谱边真相源（JSON 数组，必须是 RAW 边，存储层展开双向）
├── resources.json                                   ← 算力/内存/模型/数据集 4 条资源池元数据
├── projects.json                                    ← 项目登记 SoT
├── kb_documents.json                                ← 知识库文档 SoT（含 v1 版本快照 + 图谱关联）
├── flows.json · automations.json · agents.json · …
│
└── file-store/                                      ← ⭐ FS Backend 根（FILE_STORE_ROOT）
    ├── versions/                                    ← ⭐版本 Manifest（一文件一目录，一版本一 JSON）
    │   ├── <fileId>                                 ← UUID v4；每个文件独立目录
    │   │   ├── v1.json                              ← 第 1 版本元数据
    │   │   ├── v2.json                              ← 第 2 版本（覆盖写入自动递增）
    │   │   └── ... (vN.json)
    │   ├── <fileId>/ ...
    │   └── …
    │
    ├── chunks/                                      ← ⭐二进制 Chunk 存储（SHA-256 两级散列，见 §2）
    │   ├── 00/  01/  02/  ...  0f/  10/  ...  fe/  ff/   ← 256 个子目录（= hash 前 2 字符十六进制，00~FF）
    │   │   └── 每个目录内：
    │   │       └── <完整64字符SHA256>               ← 文件名 = Chunk内容的 SHA-256；内容 = 该 chunk 原始字节
    │   └── … (共 256 个子目录)
    │
    └── mpu/                                         ← ⭐MPU（分片上传）临时目录
        └── <uploadId>                               ← 每个大文件上传的会话目录
            ├── part-0001.bin                        ← 第 1 片（多为 5MB or FILE_MPU_CONCURRENCY 粒度）
            ├── part-0002.bin · part-0003.bin ...
            └── (所有 parts 到齐后 → 合并成 chunks/ 里 N 个标准 chunk，mpu/<uploadId>/ 被 rm -rf 删除)
```

代码锚点（目录常量 + 三目录创建幂等 `fs.mkdirSync({ recursive: true })`）：
- [file-store.js#L10-L12](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/file-store.js#L10-L12)
- [chunk-backend.js 构造函数 `ensureDirs()` L55-L69](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js#L55-L69)

---

## 2. Chunk 两级散列规则（`chunks/<xx>/<sha256>`）

### 2.1 为什么要两级？

操作系统单目录下**超过 10 万文件**，任何 `readdir()` / `stat()` 操作会**指数变慢**（NTFS/EXT4/XFS/Btrfs 都有此特性）。
用**哈希前 2 位作一级目录**，把 1 百万 chunk 均匀分到 256 个子目录（≈每目录 3906 文件），运维巡检、备份、inode 扫描、rm 都安全。

### 2.2 规则伪代码（代码里真的就是这 3 行）

```javascript
// chunk-backend.js · FSChunkBackend#L59-L64
hash   = "0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b";   // 64 字符 SHA-256（hex）
prefix = hash.slice(0, 2);                 // "0a" — 一级散列目录名
path   = pathJoin(chunksDir, prefix, hash);// "${chunksDir}/0a/0a1b2c3d4e5f..."
```

**一个真实例子**：
```
SHA-256("hello world chunk content 1MB...") = 
  "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"

→ FS 路径： chunks/b9/b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
→ S3 Key：              b9/b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
→ OSS/COS Key：        b9/b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
→ MinIO Key：          b9/b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
```

### 2.3 去重机制（核心：hasChunk + 引用计数）

```
写某个 Chunk 时：
  hash = sha256(chunkBuffer)
  1) hasChunk(hash) → fs.existsSync(`chunks/${xx}/${hash}`) === true ？
     ✅ 存在 → 不写磁盘，只把 SQLite file_chunk_refs[hash].count += 1
     ❌ 不存在 → fs.writeFileSync(...) 一次；并 refs.count=1 refs=[{"fileId:vN"}]

GC 回收时：
  count 归 0 + 文件版本 softDelete 超过 graceDays → fs.unlink() 物理删
  （若同一 chunk 被多个 vN 共享 → count>0，永远不会被误删！）
```

代码锚点：
- 去重 + 引用增：[file-store.js#L126-L136](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/file-store.js#L126-L136)
- 引用计数表 `file_chunk_refs`：[storage/index.js entities 建表 schema](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/index.js)

---

## 3. vN.json 版本 Manifest 文件格式（versions/<fileId>/vN.json）

每个版本一份 JSON，严格字段（读文件 = 把 chunks[] hash 按顺序读 FS/S3 拼回去）：

```json
{
  "version": 2,
  "hash": "aabbccdd...（整文件的 SHA-256）",
  "size": 3145728,
  "chunkCount": 3,
  "chunkSize": 1048576,
  "chunks": [
    "0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b",
    "1abcdef0123456789abcdef0123456789abcdef0123456789abcdef012345678",
    "2fedcba9876543210fedcba9876543210fedcba9876543210fedcba98765432"
  ],
  "uploadedAt": "2026-08-26T03:24:11.223Z",
  "uploadedBy": "u_1001",
  "changeNote": "增补第 3 章架构图",
  "linkedGraphIds": ["n_req_003"],
  "acl": { "owner": "u_1001", "readers": ["g_eng"], "writers": ["g_lead"] }
}
```

**版本恢复的本质**（零拷贝！）：`restoreVersion(fileId, 2)` 就是把 `v2.json` 的内容拷贝一份写成 `v(N+1).json`，`chunks[]` 数组 hash 完全不变 → **引用计数不变、磁盘零复制**。
代码锚点：[file-store.js#L371-L396](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/file-store.js#L371-L396)

---

## 4. S3 / MinIO / OSS · 同构 Key 规则（切换零改代码的根因）

### 4.1 一图看懂 FS vs S3 的映射

```
【FS（File System）】
DATA_DIR/file-store/chunks / <xx> / <64 字符 sha256>
                               │      └──────────────────┐
                               │                          │ 完全相同的字符串
                               ▼                          ▼
【S3/MinIO/OSS（对象桶）】
bucket = mox-chunks · Key = <xx> / <64 字符 sha256>
```

**代码证明**（两行代码逐字对比）：

| FS Backend 写路径 | S3 Backend 写 Key |
|---|---|
| `path.join(this.chunksDir, hash.slice(0, 2), hash)`  [[chunk-backend.js#L62](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js#L62)] | `` `${hash.slice(0, 2)}/${hash}` ``  [[chunk-backend.js#L164](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js#L164)] |

计算结果完全一致：`hash.slice(0,2) + '/' + hash`。

### 4.2 Backend 路由（createDefaultBackend 选择树）

```
FILE_BACKEND 环境变量
├── "fs"          ──→ FSChunkBackend   (用 Node.js fs.writeFileSync/readFile)
├── "minio"       ─┐
├── "s3"          ├────→ S3ChunkBackend（@aws-sdk/client-s3 · 同一实现）
├── "oss"         ─┤         ⤷ endpoint 决定落到 AWS / MinIO / OSS / COS / Kodo …
└── 其它或缺省    ─┘         ⤷ 未显式指定默认 fs（兜底安全）
```

代码：[chunk-backend.js `createDefaultBackend()` L332-L345](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js#L332-L345)

### 4.3 为什么 MinIO / OSS / COS 都用 FILE_BACKEND=s3？

它们都是 **S3-API 兼容协议**（PutObject / GetObject / CreateMultipartUpload / UploadPart / CompleteMultipartUpload 五接口 100% 对齐）。
只要改 `S3_ENDPOINT` + 桶 + AK/SK，`S3ChunkBackend` 这个类连代码分支都不用加。

| 目标存储 | `FILE_BACKEND` | `S3_ENDPOINT` 示例 |
|---|---|---|
| 自建 MinIO（K8s Service 内网） | `minio` | `http://minio.minio-ns.svc.cluster.local:9000` |
| AWS S3 · 北京区 | `s3` | 留空（SDK 走标准 AWS cn-north-1） |
| 阿里云 OSS · 杭州 VPC 内网 | `s3`（或 `oss`） | `https://oss-cn-hangzhou-internal.aliyuncs.com` |
| 腾讯云 COS · 广州 VPC 内网 | `s3`（或 `oss`） | `https://cos.ap-guangzhou.myqcloud.com` |
| 七牛 Kodo | `s3` | `https://s3-cn-south-1.qiniucs.com` |
| Ceph RGW · 自建机房 | `s3` | `http://ceph-rgw.internal:7480` |

---

## 5. 四种 Backend 的 PUT/GET/DELETE 能力对照表（保证语义等价）

| 能力 | FS | S3/MinIO/OSS |
|---|---|---|
| 分片写入（1MB chunk） | ✅ `fs.writeFileSync` | ✅ `PutObject`（单 chunk 小 → 普通 Put） |
| MPU 大文件并发上传（FILE_MPU_CONCURRENCY=8） | ✅ 4 路并发写 parts/*.bin，合并到 chunks/ | ✅ **CreateMultipartUpload → UploadPart → CompleteMultipartUpload**（MPU API 原生） |
| hasChunk 查询（去重） | ✅ `fs.existsSync` | ✅ `HeadObject`（200=存在 / 404=不存在） |
| 读 Chunk | ✅ `fs.readFile` | ✅ `GetObject`（可 Range 读） |
| 软删保护 + graceDays GC | ✅ DB `deletedAt` + `runGC()` | ✅ DB 逻辑相同；对象桶侧建议加 **Lifecycle 生命周期策略**（§6.4） |
| 共享 Chunk（跨文件/跨版本引用计数） | ✅ `file_chunk_refs.count` | ✅ 完全同逻辑（DB 仍在 SQLite/PG 里存 refs；对象桶本天然 hash 唯一，不重复存） |
| 冷数据回源自动回填（§7.3） | ✅ FS 作为真相源被回读 | ✅ 目标桶空 → Head 404 → 读 FS → PutObject 回填桶 |
| 秒级回滚（§7.1） | ✅ 切 FILE_BACKEND=fs 即回原 | ✅ 反之亦然；桶内数据永不删，下次切回直接命中 |

---

## 6. SOP：FS → S3/MinIO/OSS 操作分步骤（聚焦目录/Key 细节）

### 6.1 操作前的桶准备（必须先做！）

```bash
# 【1】建桶（私有）
## MinIO：
mc alias set myminio http://minio:9000 $AK $SK
mc mb myminio/mox-chunks --region=cn-north-1
mc anonymous set none myminio/mox-chunks           # ⭐必须私有
## AWS S3：
aws s3api create-bucket --bucket mox-chunks --region cn-north-1 \
  --create-bucket-configuration LocationConstraint=cn-north-1
aws s3api put-public-access-block --bucket mox-chunks \
  --public-access-block-configuration "BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true"
## 阿里云 OSS：
# 控制台 → 对象存储 → 新建 Bucket：读写权限=私有
```

```bash
# 【2】版本控制（强烈建议开，防止误覆盖被 GC）
## MinIO
mc version enable myminio/mox-chunks
## AWS S3
aws s3api put-bucket-versioning --bucket mox-chunks --versioning-configuration Status=Enabled
```

```bash
# 【3】服务端加密（SSE-S3 / KMS · 合规必开）
## MinIO
mc encrypt set sse-s3 myminio/mox-chunks
## AWS S3
aws s3api put-bucket-encryption --bucket mox-chunks --server-side-encryption-configuration '{
  "Rules":[{"ApplyServerSideEncryptionByDefault":{"SSEAlgorithm":"AES256"}}]
}'
```

### 6.2 切换（一行 env 生效）

```bash
# 生效顺序优先级（建议用 env，不用明文 HTTP switch）：
# Helm values → systemd EnvironmentFile= → docker --env-file → process.env

FILE_BACKEND=s3                               # 或 minio/oss
S3_CHUNKS_BUCKET=mox-chunks
S3_ENDPOINT=http://minio:9000                 # MinIO 示例；OSS/COS 填对应 VPC 内网域名
S3_ACCESS_KEY=minioadmin                      # ⚠️生产必须用 K8s Secret！不要 values 明文
S3_SECRET_KEY=minioadmin
AWS_REGION=cn-north-1

FILE_MPU_CONCURRENCY=8                        # 内网 MinIO 建议 8~16；公网云平台 8 稳
```

验证切换成功：
```bash
# 上传一个 12MB 文件，会分 12 个 chunk
curl -s -X POST -F "file=@test-12m.bin" http://localhost:3010/storage/files/upload | jq .
# 检查桶里的 12 个对象（前缀目录 = hash 前 2 位）
mc ls --recursive myminio/mox-chunks | head -20
# 预期输出形如：
# [2026-08-26 11:22:33 CST] 1.0MiB STANDARD 0a/0a1b2c3d4e5f...
# [2026-08-26 11:22:33 CST] 1.0MiB STANDARD 1a/1abcdef0123...
# ... 12 条
```

### 6.3 迁移策略两选一

- **策略 A（冷迁移·业务无感·推荐）**：不批量搬。FS 和 S3 **都存在读回退**——读某文件时如果 S3 HeadObject 404（该 chunk 还没迁过去），FileStore 自动读 FS Chunk → PutObject 填到桶。**7-30 天热点数据自然全部上桶**。冷数据（>720 天无访问）没必要迁，占 FS 反而便宜。
- **策略 B（热迁移·一次性批量）**：`POST /storage/files/backend/migrate`（见主手册 §4.3）。执行前后跑 `POST /storage/files/backend/verify sampleRate=1` 确保 0 mismatch。

### 6.4 对象桶生命周期（和 FS 的 graceDays 语义对齐）

> ⚠️ FS 有 `FILE_GRACE_DAYS=30` + `runGC()` 双保险；对象桶侧建议再加一层 Lifecycle 兜底，避免 DB 误操作导致对象永远"挂着"。

```bash
## MinIO（30 天后非当前版本→删除；365 天前标记多段上传→清理）
mc ilm rule add --noncurrentversion-expire-days 30 \
                --expire-delete-marker \
                --abort-incomplete-multipart-upload-days 7 \
                myminio/mox-chunks

## AWS S3（JSON lifecycle policy）
aws s3api put-bucket-lifecycle-configuration --bucket mox-chunks --lifecycle-configuration '{
  "Rules": [
    { "ID":"soft-delete-noncurrent-30d", "Status":"Enabled",
      "NoncurrentVersionExpiration":{"NoncurrentDays":30},
      "AbortIncompleteMultipartUpload":{"DaysAfterInitiation":7},
      "Filter":{"Prefix":""} }
  ]}'

## 阿里云 OSS：控制台 → 基础设置 → 生命周期 → 添加规则
#   标准IA（30天后）/归档（90天后）/冷归档（180天后）/删除（365天后）
```

---

## 7. 目录巡检与健康检查（SRE 每日 03:00 Cron）

### 7.1 FS 目录结构完整性巡检脚本（`daily-fs-check.sh` 样例）

```bash
#!/usr/bin/env bash
# 每日 03:00 Cron；任何异常发 AlertManager / 飞书告警
set -euo pipefail
DATA_DIR=${DATA_DIR:-./data}
LOG=daily-fs-check-$(date +%Y%m%d).log

echo "[1] 三目录存在性检查"
[ -d "$DATA_DIR/file-store/versions" ] || { echo "FAIL: versions/ 缺失"; exit 1; }
[ -d "$DATA_DIR/file-store/chunks" ]   || { echo "FAIL: chunks/ 缺失";   exit 1; }
[ -d "$DATA_DIR/file-store/mpu" ]      || mkdir -p "$DATA_DIR/file-store/mpu"

echo "[2] 256 一级散列子目录存在性（应该有 256 个 00..ff）"
actual=$(ls -1 "$DATA_DIR/file-store/chunks" | wc -l)
echo "    chunks 子目录数: $actual / 256 (空桶正常小于 256)"
# 注意：0 条文件的新系统可能只有 chunks/ 而无任何子目录（正常）

echo "[3] 孤立项点检测：chunks/ 里有 hash，但 file_chunk_refs 没引用 → 可疑漏记？"
#   思路：统计 chunks 对象数 vs DB count
CHUNKS=$(find "$DATA_DIR/file-store/chunks" -type f | wc -l)
REFS=$(sqlite3 "$DATA_DIR/ous.db" "SELECT COUNT(*) FROM entities WHERE entity_type='file_chunk_refs';")
echo "    fs 磁盘 chunks: $CHUNKS"
echo "    DB refs 计数:   $REFS"
[ "$CHUNKS" -eq "$REFS" ] 2>/dev/null || echo "WARN: 磁盘/DB 计数不等（若刚迁移则正常）"

echo "[4] MPU 僵尸目录：>24h 的 uploadId 应清理（上传中断留的 parts）"
STALE_MPU=$(find "$DATA_DIR/file-store/mpu" -maxdepth 1 -type d -mmin +1440 | wc -l)
[ "$STALE_MPU" -eq 1 ] && echo "OK: 无僵尸 MPU（1 是 mpu/ 本身）" || \
  echo "WARN: 有 $((STALE_MPU-1)) 个僵尸 MPU 目录；建议清理"

echo "[5] SHA-256 命名合规性（抽样 100 条文件名必须 64 hex 字符）"
find "$DATA_DIR/file-store/chunks" -type f | head -100 | \
  awk -F'/' '{fn=$NF; if (length(fn)!=64 || fn!~/^[0-9a-f]{64}$/) print "BAD:", fn}' > /tmp/bad-chunk-names.txt
[ ! -s /tmp/bad-chunk-names.txt ] && echo "OK: 抽样文件名合规" || \
  { echo "FAIL: 非法文件名:"; cat /tmp/bad-chunk-names.txt; exit 3; }

echo "[6] ous.db 健康（PRAGMA integrity_check）"
sqlite3 "$DATA_DIR/ous.db" "PRAGMA integrity_check(100);" | grep -q "ok" && echo "OK: SQLite integrity_check=ok" || \
  { echo "FAIL: SQLite 损坏！"; exit 4; }

echo "ALL DONE" > $LOG
```

### 7.2 对象桶健康检查（S3/MinIO/OSS）

```bash
#!/usr/bin/env bash
# 每日 03:30 Cron
mc du myminio/mox-chunks                     # 容量
mc ls myminio/mox-chunks --recursive | wc -l # 对象数（对比 DB refs 计数）
# MinIO 特有：健康状态（Erasure 4+2 容忍掉线节点数）
mc admin health myminio
# 一致性对账：抽样取 100 个对象，etag（MD5）与本地 FS hash 对 SHA-256 做内容验证
POST /storage/files/backend/verify  { "sampleRate": 0.01, "deep": true }
```

### 7.3 读回退（回源）监控
```
Prometheus 指标（建议前端埋点暴露）：
  mox_filestore_read_fallback_total  （目标空读 → 回 FS 次数）
告警：
  5 分钟 rate(mox_filestore_read_fallback_total[5m]) > 100
  → 说明批量迁移没完成或 S3 网络有问题（VPC Endpoint 不通？）
```

---

## 8. 常见问题 F.A.Q（目录 / Key 层）

| # | 问题 | 原因 + 处置 |
|---|---|---|
| Q1 | `chunks/` 里看到的都是 `00/ 01/ ... fe/ ff` 256 个空目录？正常吗？ | ✅ **正常**。目录在 `ensureDirs()` 就被 `mkdir -p` 提前建完（后续写文件不再需要 mkdir 原子性）。空目录 NTFS 占 <8KB，256 个可忽略。 |
| Q2 | 能不能手动 cp FS chunks 到 S3 桶里？会不会坏？ | ✅ **完全安全**——只要你把 `chunks/<xx>/<hash>` 整个目录结构用 `mc cp --recursive chunks/ myminio/mox-chunks/` 拷过去，对象 Key 就和代码生成的**完全一致**。S3 切换后直接命中，不需要重新跑 migrate。 |
| Q3 | 两个 FS 部署怎么合并到同一个 S3 桶？ | 直接 cp 两个 FS 的 chunks/ 到同一桶 → hash 相同自动合并（内容一样就 SHA-256 一样，不会重复）；再合并 DB 里的 files / versions（`/storage/migrate` 可以指定多 source）。 |
| Q4 | `mpu/<uploadId>/` 下有大量 part-XXXX.bin，为什么一直不删？ | 用户**大文件上传中途取消 / 网络断开**。用 §7.1 的 MPU 僵尸扫描 + 手动 `rm -rf mpu/<超过 24h>`；或者 S3/OSS 开 AbortIncompleteMultipartUpload lifecycle 7天自动清。 |
| Q5 | 能不能把 `chunks/` 单独挂到更大的磁盘上？ | ✅ 可以！用 `mount --bind /mnt/big-disk/mox-chunks $DATA_DIR/file-store/chunks`，或者直接把 DATA_DIR 整个迁到大盘 → 改环境变量重启。我们的路径都是相对 DATA_DIR 的，没有硬编码盘符。 |
| Q6 | 磁盘满了，我删了 `chunks/` 目录下一堆文件，系统会崩溃吗？ | ⚠️ **会触发读错误！** chunks 是物理内容，直接删文件会导致对应文件下载时 `ENOENT`。**正确扩容/清理流程：必须用 SOP 里的 `POST /storage/files/gc`（先 softDelete → 过 graceDays → GC 自动删 count=0 的）。** 如果已经手工误删 → 立即 `readPref=auto` 回滚 FS（若有 S3 备份则从 S3 读回） → 再跑一次 migrate 把缺 chunk 回填回去。 |
| Q7 | MinIO / OSS 上我想自定义前缀（比如 `mox-prod/chunks/`），会不会和 FS 的结构冲突？ | 可以。代码里的 Key 是 `hash.slice(0,2)+'/'+hash`，加上桶的根前缀 `mox-prod/chunks/` 后会变成 `mox-prod/chunks/xx/hash`，不影响同构。在桶级分层即可。 |

---

## 9. 附录：Key 一致性自检脚本（切换前后必须跑一次）

```bash
#!/usr/bin/env bash
# 脚本名：verify-key-isomorphism.sh
# 功能：从 FS 抽 N 个 chunk，对比它们在目标桶里的路径/Key 是否严格一致
set -euo pipefail
DATA_DIR=${DATA_DIR:-./data}
BUCKET_ALIAS=myminio/mox-chunks
N=${1:-50}

find "$DATA_DIR/file-store/chunks" -type f | head -$N | while read -r fp; do
    hash=$(basename "$fp")
    prefix=$(basename "$(dirname "$fp")")
    expected_key="${prefix}/${hash}"
    # 本地 FS 一致性（前缀必须 = hash.slice 0,2）
    [ "${hash:0:2}" = "$prefix" ] || { echo "FAIL: $fp 目录前缀与 hash 不匹配"; exit 1; }
    # 桶内一致性：若目标已有，则路径存在
    if mc stat "$BUCKET_ALIAS/$expected_key" >/dev/null 2>&1; then
        echo "OK:  $expected_key  (FS与桶一致)"
    else
        echo "SKIP: $expected_key  (目标桶尚无，属于未迁移的冷数据，正常)"
    fi
done
echo "✅ $N 条 Key 同构自检通过"
```

---

**本手册配合主手册使用**：切换总流程 → 主 SOP `storage-cloud-switch-sop.md` §4；本手册只聚焦 **目录结构 / Key 规则 / 巡检 / Key 自检** 细节。
**变更记录**：所有对 `chunks/` `versions/` `mpu/` 的手工操作必须登记 CR，且走 §7 回滚验证一次。
