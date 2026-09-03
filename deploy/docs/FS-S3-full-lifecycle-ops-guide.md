# FS 与 S3 切换 · 全生命周期运维指南

**版本：** 2.0.0（合并版）
**发布日期：** 2026-08-26
**适用范围：** Mox Platform（backend-node 单机 / Helm 伞图 K8s 部署）
**覆盖范围：** 知识图谱存储引擎（SQLite ↔ PG/MySQL/云DB）+ 云盘 Chunk Backend（FS ↔ S3/MinIO/OSS/COS）
**操作等级：** L3 生产操作（变更前请登记 CR 单并完成 §0 回滚演练）
**前置阅读：** [ops-manual.md](./ops-manual.md) · [ha-capacity-tco.md](./ha-capacity-tco.md) · [xinchuang-matrix.md](./xinchuang-matrix.md)
**维护者：** SRE & Mox Platform Team

> **本手册为「全生命周期一本通」—— 从选型（§1）→ 配置（§2）→ 操作 SOP（§3-§5）→ 巡检（§8）→ 故障处置（§9）→ 归档（§10）贯穿到底，不再分子手册。**
> 原本的 `storage-cloud-switch-sop.md` 与 `filesystem-backend-structure-sop.md` 内容已全部合并到本手册 v2.0.0。

---

## 0. 变更须知 + 三大铁律（操作前必读·违反叫停）

### 0.1 三大铁律（SRE 红线）

1. **默认自研永远是真相源** —— 任何切换必须从「SQLite + FS」出发，**绝不允许**绕过自研首写直接把第三方当主。真相源目录（`$DATA_DIR/`）在切换完成后 30 天内**绝不物理删除**，只归档。
2. **双写对账窗口 ≥ 7 天（图谱）+ 7 天对账/冷回填（云盘）** —— 连续 7 天 `62/62 Δ≤1e-6` + `hashMismatch=0` 才允许关双写或 GC 本地 FS。
3. **一键回滚链常驻** —— 任何阶段异常立刻执行 §6 回滚流程；`readPref=auto`（图谱）/「目标空读自动回 FS」（云盘）兜底回源，保证**数据零丢失**。

### 0.2 影响范围评估（CR 前必查 & 签字）

- 图谱 `nodes/edges` 量 > 10W **或** 云盘 `totalSize > 1TB` → **业务低峰窗口**（建议 22:00-02:00）执行迁移阶段。
- Helm 伞图 + 双活 DR → 必须 `helm upgrade` **灰度 4 阶段**推进 env 变更；禁止 `kubectl edit` 直接改 Pod 环境变量。
- 公有云 OSS/COS/S3 → **VPC 内 Endpoint** 强制（VPC 内网域名优于公网）；禁止公网打桶（带宽费 + 延迟抖动 + 合规风险）。
- 切换前必须**先完整演练 §6 回滚流程一次**，并把演练耗时+成功结果填入 CR 单 §10 归档第 7 项。

---

## 1. 架构全景与选项表

### 1.1 两模块 × 三档位矩阵

| 模块 | 默认（自研 ⭐真相源） | 开源方案（社区选） | 云平台方案 | 主切换变量 |
|---|---|---|---|---|
| **知识图谱存储引擎** | `sqlite` + JSON 磁盘双写（`data/graph_nodes.json` / `graph_edges.json`） | PostgreSQL 13+ · MySQL 5.7+ · MariaDB 10.6+ | 腾讯云 TencentDB · 阿里云 PolarDB/RDS · AWS Aurora | `DB_PROVIDER` |
| **云盘 Chunk Backend** | `fs` 两级散列 + 引用计数 GC（`file-store/chunks/<xx>/<sha256>`, `versions`, `mpu`） | MinIO · Ceph RGW · Garage（S3 协议兼容） | AWS S3 · 阿里云 OSS · 腾讯云 COS · 七牛 Kodo | `FILE_BACKEND` |

### 1.2 后端能力三维对比（选型决策表）

| 能力 | 自研 SQLite+FS（⭐默认） | PG/MySQL + MinIO（开源集群） | 云数据库 + 云对象存储（托管） |
|---|---|---|---|
| **TCO（0-6 月）** | ⭐最低（零授权、零额外运维） | 中（3 节点最小集群运维人力） | 高（云托管费 + 下行带宽费 + 请求费） |
| **性能（单机）** | ⭐最优（WAL + 本地 FS 零网络） | 良（1Gb 内网 RPC ≈ P99 ±10%） | 中（VPC Endpoint RTT <2ms） |
| **横向扩展上限** | 单机物理上限（32C/64G 约 500W 图节点，50TB FS） | ⭐最佳（分库分表 + 只读副本 + Erasure Set） | ⭐托管最佳（按 API 按量，无限） |
| **合规 / 数据主权** | ⭐完全可控（数据不出机） | ✅ 可控（自建机房） | ⚠️ 需评估云平台合规矩阵 |
| **RPO / RTO** | RPO=快照；RTO≈5min | RPO<30s / RTO<5min（主从） | RPO<1s / RTO<1min（云托管） |
| **推荐场景** | 起步 / 企业内网单机 / 开发测试 / 真相源 | 中大型自建集群 / 敏感数据 | 公有云部署 / 多云 / 弹性用户量 |

代码锚点：
- 图谱存储抽象（索引/双写/迁移/对账）：[storage/index.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/index.js)
- 云盘 Backend 路由与 FS/S3 实现：[storage/chunk-backend.js#L332-L345](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js#L332-L345)
- env 默认值：[config.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/config.js)

---

## 2. 环境变量总表（一张表配完所有切换）

> 原则：**所有生产环境变更一律走 env（systemd / docker env-file / Helm values + Secret）**，不走明文 HTTP body 传密码。

### 2.1 知识图谱存储引擎

| 变量 | 默认（自研） | 开源 PG/MySQL | 云数据库 | 说明 |
|---|---|---|---|---|
| `DB_PROVIDER` | `sqlite` | `postgresql` / `mysql` | `postgresql` / `mysql` | **主开关** |
| `DB_HOST` / `DB_PORT` | - | 内网 DNS / VIP；5432 / 3306 | 云平台 HOST；**强制 TLS PORT** | 跨 VPC / 公网必须启用 |
| `DB_NAME` | `ous` | `ous` | `ous` | `CREATE DATABASE ous ENCODING 'UTF8';` |
| `DB_USER` / `DB_PASSWORD` | - | 自建最小权限账号 | 云平台账号（IAM 临时凭证优先） | **禁止 values 明文；Secret + envFrom** |
| `DB_SSL` | `false` | 可选（内网自签也建议开） | **强制 `true`** | |
| `DB_DUAL_WRITE` | `false` | `true` 迁移期 | `true` 迁移期 | §3 第①步开；第⑤步关 |
| `DB_READ_PREF` | `auto` | `auto` → `primary` | `auto` → `primary` | 迁移期 `auto`（空读回源+回填）；切读后 `primary` |
| `DATA_DIR` | `./data` | **保留作真相源归档** | **保留作真相源归档** | 切换后绝不删；GC 前对账基准 |

### 2.2 云盘 Chunk Backend

| 变量 | 默认（自研） | MinIO / Ceph | 云平台 S3/OSS/COS | 说明 |
|---|---|---|---|---|
| `FILE_BACKEND` | `fs` | `minio` / `s3` | `s3`（或 `oss` 语义别名） | **主开关**（minio/s3/oss 走同一 S3ChunkBackend） |
| `S3_CHUNKS_BUCKET` | - | `mox-chunks` | 云平台桶名 | **提前创建 + 私有 + 版本 + SSE** 齐套后才切换 |
| `S3_MANIFEST_BUCKET` | （空 = 走本地 entities + `versions/` 目录） | 可选 `mox-manifests` | 可选 | **建议留空**：manifest 仍放真相源 DB + 本地 JSON，稳定性更高 |
| `S3_ENDPOINT` | - | `http://minio:9000`（K8s Service 内网） | OSS/COS **VPC 内网域名**（优先） | 留空 = AWS SDK 走标准 cn-north-1 等 |
| `S3_ACCESS_KEY` / `S3_SECRET_KEY` | - | MinIO 账号 | 云平台 AK/SK | **K8s Secret + envFrom**；**绝不明文** |
| `AWS_REGION` | - | 忽略 | `cn-north-1` 等 | AWS SDK 所需 |
| `FILE_SOFT_DELETE` | `true` | ✅同左 | ✅同左 | 切换后继续生效（softDelete → graceDays → GC purge） |
| `FILE_GRACE_DAYS` | `30` | ✅同左 | ✅同左 | 对象桶侧建议额外加 Lifecycle（§4.6） |
| `FILE_MPU_CONCURRENCY` | `4` | `8`（内网 1Gb 建议 8-16） | `8`（云平台看带宽调） | MPU 大文件并发度 |
| `FILE_MAX_QUOTA_BYTES` | `0` | ✅同左 | ✅同左 | 0=不限；>0 上传前预检查，超配额 429 |

---

## 3. 底层结构篇：FS 真相源 + S3 同构 Key 规则

> 本章回答「数据真实落在哪、长什么样」。**切换前至少读一遍 §3.1 全景树，避免误删目录**。

### 3.1 DATA_DIR 全景树（真相源目录·绝对禁止 rm -rf）

```
DATA_DIR/                                             ← env DATA_DIR；默认 = backend-node/data
├── ous.db                                            ← SQLite 真相库（journal_mode=WAL · synchronous=NORMAL）
├── ous.db-wal                                        ← SQLite 预写日志（运行中必存在；正常关闭后合并）
├── ous.db-shm                                        ← SQLite 共享内存文件
│
├── 📚 结构化 JSON 双写（lib/json-store.js 先写此处，再写 SQLite entities 表）
├── graph_nodes.json                                  ← 图谱节点真相源（JSON 数组）
├── graph_edges.json                                  ← 图谱边真相源（必须 RAW 边，库内再展开双向）
├── resources.json                                    ← 4 条资源池元数据（算力/内存/模型/数据集）
├── projects.json                                     ← 项目登记 SoT
├── kb_documents.json                                 ← 知识库文档 SoT（含 v1 快照 + 图谱关联）
├── flows.json · automations.json · agents.json · …
│
└── file-store/                                       ← ⭐ FS Backend 根（FILE_STORE_ROOT）
    ├── versions/                                     ← 版本 Manifest（一文件一 UUID 目录·一版本一 JSON）
    │   ├── <UUID-fileId>/
    │   │   ├── v1.json  v2.json  ...  vN.json       ← 每个 vN 字段见 §3.3
    │   └── …
    ├── chunks/                                       ← ⭐二进制内容（SHA-256 两级散列 §3.2）
    │   ├── 00/  01/  02/  …  fe/  ff/              ← 256 个子目录（hash 前 2 字符十六进制 00~FF）
    │   │   └── <64 字符 hex SHA-256>                 ← 文件名=该 chunk 的 SHA-256；内容=该 chunk 原始字节
    │   └── （共 256 个散列目录，ensureDirs() 启动时幂等 mkdir -p 创建）
    └── mpu/                                          ← ⭐MPU（分片上传）临时目录
        └── <uploadId>/                               ← 每次大文件（≥100MB）上传会话
            ├── part-0001.bin · part-0002.bin ...
            └── （parts 齐 → 合并成 chunks/ 的 N 个标准 chunk → 本目录 rm -rf）
```

代码锚点：
- 三目录常量 + 幂等创建：[file-store.js#L10-L12](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/file-store.js#L10-L12) + [chunk-backend.js `ensureDirs()` L55-L69](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js#L55-L69)
- JSON 双写机制（先写磁盘再写库，磁盘即真相）：[lib/json-store.js#L46-L76](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/lib/json-store.js#L46-L76)

### 3.2 两级散列 + SHA-256 规则（FS 与 S3 100% 同构）

#### 为什么要两级？
> 操作系统单目录下**超过 10 万文件**，`readdir()` / `stat()` 会指数变慢（NTFS/EXT4/XFS/Btrfs 都有此性质）。用**哈希前 2 位作一级目录**，1 百万 chunk 均匀分到 256 个子目录（≈每目录 3906 文件），巡检/备份/inode/rm 都安全。

#### 代码 3 行证明（FS 与 S3 Key 逐字一致）

```javascript
// 两者用同一表达式： hash.slice(0, 2) + '/' + hash
hash   = "0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b";   // 64 字符 hex

// FS Backend 落盘路径：chunk-backend.js#L62
prefix = hash.slice(0, 2);                                             // "0a"
path   = pathJoin(chunksDir, prefix, hash);                            // "./chunks/0a/0a1b2c..."

// S3/MinIO/OSS 对象 Key：chunk-backend.js#L164  ← ⭐同一表达式
key    = `${hash.slice(0, 2)}/${hash}`;                                // "0a/0a1b2c..."
```

**FS 与 S3 一图映射：**
```
【FS】  DATA_DIR/file-store/chunks / <xx> / <64 字符 hash>
                                │       └────────────────┐
                                │                        └─ 同一字符串 ─┐
                                ▼                                      ▼
【S3】  bucket = mox-chunks · Key = <xx> / <64 字符 hash>
```

这就是**切换零重命名、零搬运必要（冷数据可选）、秒级回滚、回源回填后再切直接命中**的根因。

#### 去重 + 引用计数（共享 Chunk 永不误删）

```
写入某个 Chunk：
  hash = sha256(chunkBuffer)
  hasChunk(hash) ？
    ✅ 存在 → 不写磁盘/对象；DB file_chunk_refs[hash].count += 1；refs.push("fileId:vN")
    ❌ 不存在 → 写一次 FS/S3；refs.count=1

GC 回收：
  文件版本 softDeleted + graceDays 到期 + file_chunk_refs[hash].count 归 0
    → 才允许 fs.unlink / S3 DeleteObject
    → （多文件/多版本共享 chunk：count>0，永远不会被误删！）
```

代码锚点：[file-store.js#L126-L136（去重+引用+1）](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/file-store.js#L126-L136) + `runGC()` 同文件末。

### 3.3 vN.json 版本 Manifest 字段全表

路径：`versions/<fileId>/vN.json`（读文件 = 按 `chunks[]` hash 顺序拼 FS/S3 返回）

```json
{
  "version": 2,
  "hash":         "整文件的 SHA-256（用于 E2E 内容校验）",
  "size":         3145728,
  "chunkCount":   3,
  "chunkSize":    1048576,
  "chunks": [ "<64-char-hash>", "...", "..." ],    // 顺序决定拼接
  "uploadedAt":   "ISO8601",
  "uploadedBy":   "u_1001",
  "changeNote":   "业务变更说明（用于审计）",
  "linkedGraphIds": ["n_req_003"],                // 双向挂接图谱节点
  "acl": { "owner": "u_1001",                      // 文件级 ACL（owner/readers/writers）
           "readers": ["g_eng"],
           "writers": ["g_lead"] }
}
```

**版本恢复的本质 = 零拷贝**：`restoreVersion(fileId, 2)` 只是把 `v2.json` 的内容**复制一份**写成 `v(N+1).json`，`chunks[]` hash 不变 → 引用计数不变、磁盘零复制。
代码：[file-store.js#L371-L396](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/file-store.js#L371-L396)

### 3.4 Backend 路由选择树（createDefaultBackend）

```
FILE_BACKEND 环境变量
├── "fs"        ──→ FSChunkBackend   （Node.js fs.* 同步/异步 IO）
├── "minio"     ─┐
├── "s3"        ├────→ S3ChunkBackend（@aws-sdk/client-s3 同一实现）
├── "oss"       ─┤         ⤷ S3_ENDPOINT 决定落到 MinIO / OSS / COS / Kodo / Ceph RGW …
└── 其它或缺省  ─┘         ⤷ 未显式指定默认 "fs"（安全兜底，避免 env 丢失突然切空）
```
代码：[chunk-backend.js#L332-L345](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js#L332-L345)

### 3.5 目标存储 ENDPOINT 配置速查

| 目标存储 | `FILE_BACKEND` | `S3_ENDPOINT` 示例 |
|---|---|---|
| 自建 MinIO（K8s Service 内网） | `minio` | `http://minio.minio-ns.svc.cluster.local:9000` |
| AWS S3 · 北京区 | `s3` | 留空（SDK 走 `cn-north-1` 标准） |
| 阿里云 OSS · 杭州 VPC 内网 | `s3`（或 `oss`） | `https://oss-cn-hangzhou-internal.aliyuncs.com` |
| 腾讯云 COS · 广州 VPC 内网 | `s3`（或 `oss`） | `https://cos.ap-guangzhou.myqcloud.com` |
| 七牛 Kodo | `s3` | `https://s3-cn-south-1.qiniucs.com` |
| Ceph RGW · 自建机房 | `s3` | `http://ceph-rgw.internal:7480` |

### 3.6 FS vs S3 语义等价对照表（确保切换零行为差异）

| 能力 | FS | S3/MinIO/OSS |
|---|---|---|
| 1MB chunked 写 | ✅ `fs.writeFileSync` | ✅ `PutObject`（单 chunk 小 → 普通 Put） |
| MPU 大文件并发（FILE_MPU_CONCURRENCY=8） | ✅ parts/*.bin 4-16 路并发写 → 合并 chunks | ✅ `CreateMultipartUpload → UploadPart → CompleteMultipartUpload` 原生 |
| hasChunk 去重 | ✅ `fs.existsSync` | ✅ `HeadObject`（200=存在 / 404=不存在） |
| chunk 读 | ✅ `fs.readFile`（Range 分片） | ✅ `GetObject`（Range 请求） |
| 软删 + graceDays GC | ✅ DB `deletedAt` + `runGC()` 双保险 | ✅ DB 软删逻辑相同；建议额外桶 Lifecycle 兜底 |
| 跨文件/版本共享 chunk | ✅ `file_chunk_refs.count` 计数 | ✅ 同逻辑（DB 计数仍在 SQLite/PG；桶侧 hash 唯一天然去重） |
| 冷数据回源回填 | ✅ FS 作为真相源被回读 | ✅ Head 404 → 读 FS → PutObject 回填桶（读时自动发生） |
| 秒级回滚 | ✅ 切 `FILE_BACKEND=fs` 立刻回原 | ✅ 桶内数据永不删，下回切回直接命中（零搬运） |

---

## 4. 流程篇 A：知识图谱存储引擎切换 5 步法

> 示例：`sqlite`（真相源）→ `postgresql`。MySQL / 云平台 PolarDB / TencentDB 完全同构（替换 env）。

### 4.0 预检查（Step 0 · CR 前全绿）

```bash
# [1] 当前状态快照
curl -s http://localhost:8080/storage/status | jq .
# 期望：provider=sqlite, features.dualWrite=false, types.graph_nodes=N / graph_edges=M

# [Helm] 核对当前生效 env
helm get values mox -n mox-system -a | grep -E 'DB_|DATA_DIR'

# [2] 目标库连通性 + 建库（PG 示例）
psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d postgres -c \
  "CREATE DATABASE ous ENCODING 'UTF8' LC_COLLATE='C.UTF8' LC_CTYPE='C.UTF8' TEMPLATE=template0;"
psql -h $DB_HOST -U $DB_USER -d ous -c "SELECT 1;"          # ✅
# 权限：最小原则（仅 mox_app SCHEMA public）
#   CREATE USER mox_app WITH LOGIN PASSWORD '***';
#   GRANT CONNECT ON DATABASE ous TO mox_app;
#   GRANT ALL PRIVILEGES ON SCHEMA public TO mox_app;

# [3] 数据量评估 → 估算窗口
du -sh $DATA_DIR/ous.db $DATA_DIR/graph_nodes.json $DATA_DIR/graph_edges.json
# 经验值：PG 25k行/秒 · MySQL 18k行/秒（1Gb 内网）

# [4] 真相源快照（SHA-256 归档审计链）
tar -zcf mox-data-snapshot-$(date +%Y%m%d-%H%M).tar.gz \
      -C $(dirname $DATA_DIR) $(basename $DATA_DIR)
sha256sum mox-data-snapshot-*.tar.gz >> $AUDIT_DIR/hash-chain.log
```

### 4.1 第①步：启动双写模式

> **生产最佳实践：不传 conn 明文密码，用 env 生效。**

```bash
# 注入 env（systemd EnvironmentFile= / docker --env-file / Helm values）
DB_PROVIDER=postgresql
DB_HOST=pg-vip.internal
DB_PORT=5432
DB_NAME=ous
DB_USER=mox_app
DB_PASSWORD=<from-secret>
DB_SSL=true
DB_DUAL_WRITE=true
DB_READ_PREF=primary

# 单机：systemctl restart mox-backend
# Helm：helm upgrade ... --set ...（灰度 4 阶段推进，见 §7）

# 确认双写生效
curl -s http://localhost:8080/storage/status | jq .features
# 期望：{ "dualWrite": true, "readPref": "primary", "provider": "postgresql" }
```

**烟测验证**：前台新增/修改 1 条图谱节点，检查：
- `data/graph_nodes.json` 有新条目 → ✅ 真相源写 OK
- PG `SELECT COUNT(*) FROM entities WHERE entity_type='graph_nodes';` 同比 +1 → ✅ 双写 OK

### 4.2 第②步：存量迁移（SQLite + JSON → 目标）

```
POST /storage/migrate
Content-Type: application/json

{
  "source": "sqlite",
  "target": "postgresql",
  "type": "all",             // all = graph_nodes+graph_edges+resources+files+kb_documents+…
  "batchSize": 500,          // 默认 500
  "onConflict": "update"     // skip 或 update（幂等）
}
```

期望响应（关键：source 与 inserted/updated 之和相等）：
```json
{
  "ok": true, "status": "complete", "elapsedMs": 2891,
  "summary": {
    "graph_nodes": { "source": 72,   "inserted": 72,   "updated": 0, "skipped": 0 },
    "graph_edges": { "source": 103,  "inserted": 103,  "updated": 0, "skipped": 0 },
    "resources":   { "source": 4,    "inserted": 4,    "updated": 0, "skipped": 0 },
    "kb_documents":{ "source": 19,   "inserted": 19,   "updated": 0, "skipped": 0 }
  }
}
```

**强制 Post-验证：**
```
POST /storage/verify  { "type":"all" }
→ 期望 rowsTotalMatch=true · hashMismatch=0
```

### 4.3 第③步：空读回填观察窗口（72h）

```
POST /storage/switch  { "provider":"postgresql", "dualWrite":true, "readPref":"auto" }
```

关键监控（72h 达标才能进下一步）：
| 指标 | 阈值 | 不达标处置 |
|---|---|---|
| `mox_storage_read_empty_total` 空读率 | < 0.01% | 延长观察 24h；或手动重跑 migrate type=缺失表 |
| `mox_storage_dualwrite_failed_total` | = 0 | >0 → 排查目标库连接池/权限，必要时回滚 §6 |
| `mox_query_p99` | 切换前 ±10% | 飙高 → 给 `entities(entity_type, entity_id)` 建复合索引（DDL 已自带） |

### 4.4 第④步：切读 + 7 天深度对账窗口

```
POST /storage/switch  { "provider":"postgresql", "dualWrite":true, "readPref":"primary" }
```

**每日 03:00 Cron（强制 7 天 × 7 条结果归档）**：
```
POST /storage/verify  { "type":"all", "deep": true, "sampleRate": 1 }
→ 企业级验收：连续 7 天 rowsTotalMatch=true + hashMismatch=0 + "62/62 Δ≤1e-6"
```
（对齐 24 号 A+ §5 硬标准；任 1 天 fail → 延长对账窗口 3 天并重跑 fail 的类型。）

### 4.5 第⑤步：关双写 + 真相源归档

```
POST /storage/switch  { "provider":"postgresql", "dualWrite":false, "readPref":"primary" }
```

归档（**绝不删**；打包只读冷存储 + SHA-256 入审计链）：
```bash
tar -zcf mox-data-archive-before-switch-pg-$(date +%Y%m%d).tar.gz \
      ./data --transform 's,^,mox-data-archive/,'
sha256sum mox-data-archive-*.tar.gz >> $AUDIT_DIR/hash-chain.log
# 上传对象桶「冷存储」层；建议生命周期「180 天 → 归档」
```

可选（30 天后稳定）：把 `DATA_DIR/*.json`（除了 `kb_documents.json` / `resources.json`）移入 `archived/` 子目录，不物理删除。

---

## 5. 流程篇 B：云盘 Backend 切换 4 步法（FS ↔ S3/MinIO/OSS）

> 示例：`fs` → `minio`。OSS/COS/S3 仅 `S3_ENDPOINT` + 桶名不同，其余命令等价。

### 5.0 预检查（Step 0）

```bash
# [1] 体量快照
curl -s http://localhost:8080/storage/files/stats | jq .
# 关注 totalFiles, totalVersions, totalSize, filesByExtension, graphCoverage

# [2] 桶准备（三件套：私有 + 版本 + SSE）
## MinIO
mc alias set myminio http://minio:9000 $AK $SK
mc mb myminio/mox-chunks --region=cn-north-1
mc anonymous set none myminio/mox-chunks        # ⭐必须私有
mc version enable myminio/mox-chunks             # ⭐版本控制
mc encrypt set sse-s3 myminio/mox-chunks         # ⭐SSE-AES256
## AWS S3 等价
aws s3api create-bucket --bucket mox-chunks --region cn-north-1 \
  --create-bucket-configuration LocationConstraint=cn-north-1
aws s3api put-public-access-block --bucket mox-chunks \
  --public-access-block-configuration "BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true"
aws s3api put-bucket-versioning --bucket mox-chunks --versioning-configuration Status=Enabled
aws s3api put-bucket-encryption --bucket mox-chunks --server-side-encryption-configuration '{
  "Rules":[{"ApplyServerSideEncryptionByDefault":{"SSEAlgorithm":"AES256"}}]
}'

# [3] 真相源快照（SHA-256 归档）
du -sh $DATA_DIR/file-store
tar -zcf mox-file-store-snapshot-$(date +%Y%m%d).tar.gz -C $DATA_DIR file-store
sha256sum mox-file-store-snapshot-*.tar.gz >> $AUDIT_DIR/hash-chain.log
```

### 5.1 第①步：切 Backend + 烟测

> 云盘切换没有"双写"概念（chunk hash 全局不变），但「目标空读 → 自动回 FS + 回填」的 Fallback 机制是出厂默认。

```bash
# 方式 A（推荐·env 生效）
FILE_BACKEND=minio                         # 或 s3 / oss
S3_CHUNKS_BUCKET=mox-chunks
S3_ENDPOINT=http://minio:9000              # OSS/COS 填 VPC 内网域名
S3_ACCESS_KEY=<from-secret>                # K8s Secret envFrom
S3_SECRET_KEY=<from-secret>
AWS_REGION=cn-north-1
FILE_MPU_CONCURRENCY=8

# Helm：灰度 4 阶段；单机：systemctl restart
# 或 方式 B（热切 API）：
POST /storage/files/backend/switch
{ "backend":"minio", "bucket":"mox-chunks", "endpoint":"http://minio:9000",
  "accessKey":"…", "secretKey":"…", "region":"cn-north-1" }
```

**烟测验证（上传 12MB → 分 12 chunk → 检查桶内 12 对象）**：
```bash
curl -s -X POST -F "file=@./test-12m.bin" http://localhost:8080/storage/files/upload \
  | jq '{id,size,chunkCount}'
mc ls --recursive myminio/mox-chunks
# 期望看到：<xx>/<64 字符 hash>  共 12 条 · 前缀名 = hash 前 2 位 · 与 FS 完全同构
```

### 5.2 第②步：存量 chunk 迁移（冷热两策略二选一）

#### 策略 B1（推荐·冷迁移·业务无感）
读时自动回源回填：`readFile()` 目标桶 HeadObject 404 → 读 FS → PutObject 回填桶。
- **7-30 天热点数据自然全量上桶**
- **冷数据（>720 天无访问）** 没必要搬（占 FS 反而更便宜；或者 §5.4 GC 时一起处理）

#### 策略 B2（热迁移·一次性批量）

```
POST /storage/files/backend/migrate
{
  "from": "fs", "to": "minio",
  "concurrency": 16,
  "olderThanDays": 0,             // 0=全部；>0 只搬 N 天前的热点
  "chunkBatch": 500
}
```
期望响应：`total ≈ migrated + skipped（已存在于桶的 hash 不重复搬）· failed=0`。  
**前后必跑 verify**：`POST /storage/files/backend/verify { sampleRate:1, deep:true }` → `mismatchCount=0`。

### 5.3 第③步：7 天读对账观察窗口（配合 §8 巡检）

云盘切换**没有 7 天双写**，但应：
- 每日 Cron 跑 §8.1 FS 检查 + §8.2 桶检查 + `verify sampleRate=0.1`
- 监控指标 `mox_filestore_read_fallback_total`（目标空读回 FS 次数）：5 分钟 rate > 100 次 → **批量迁移没完成或 VPC Endpoint 不通**，立即处置
- 直到连续 7 天 fallback 率 < 0.001% + verify 0 mismatch → 才算云盘侧切换稳态

### 5.4 第④步：GC 本地 FS 旧 chunk（引用计数 + 30 天空兜）

> ⚠️ GC 前必须先完成 §6 回滚演练一次；§0.1 铁律第 2 条 7 天对账达标。

```
POST /storage/files/gc  { "dryRun":true,  "graceDaysOverride":30 }
# dry-run 报告 OK 后：
POST /storage/files/gc  { "dryRun":false, "graceDaysOverride":30 }
```

GC 报告存档字段：
```json
{
  "ok": true,
  "softPurged": 3, "hardDeleted": 18, "refsRemoved": 44,
  "chunksFreed": 18, "bytesFreed": 20971520,
  "warnings": ["chunk abcdef: refcount > 0, skipped (shared between v1/v2)"]
}
```

**GC 后保留规则（§0.1 铁律第 3 条 + 第 1 条）**：
- **GC 后仍保留 `file-store/` 的空目录结构 30 天**（冷读兜底防极端）
- **真相源 `DATA_DIR/ous.db` + `DATA_DIR/*.json` 30 天内永不物理删**，只归档
- `warnings` 非空 → 先排查共享 chunk 引用计数，不强制删

### 5.5 对象桶 Lifecycle 兜底（和 graceDays=30 语义对齐）

```bash
## MinIO（30 天非当前版删除；7 天未完成 MPU 中止）
mc ilm rule add --noncurrentversion-expire-days 30 \
                --expire-delete-marker \
                --abort-incomplete-multipart-upload-days 7 \
                myminio/mox-chunks

## AWS S3（JSON Policy）
aws s3api put-bucket-lifecycle-configuration --bucket mox-chunks --lifecycle-configuration '{
  "Rules": [{
    "ID":"soft-delete-noncurrent-30d + mpu-7d",
    "Status":"Enabled",
    "NoncurrentVersionExpiration":{"NoncurrentDays":30},
    "AbortIncompleteMultipartUpload":{"DaysAfterInitiation":7},
    "Filter":{"Prefix":""}
  }]
}'

## 阿里云 OSS（控制台）：
#   生命周期：标准IA(30d) → 归档(90d) → 冷归档(180d) → 删除(365d)；
#   过期删除：非当前版本 30 天后；碎片：MPU 初始化 >7 天清理
```

---

## 5.6 组合操作：同时切换图谱引擎 + 云盘（企业上云典型 9 日节奏）

| 日 | 窗口 | 操作 | 验收标准 |
|---|---|---|---|
| D-1 晚 | 22-00 | §4.0 + §5.0 预检查 + 两份快照 | SHA-256 入审计链；回滚演练成功 |
| D0 晚 | 22-02 | §4.1 图谱双写启动 + §4.2 全量 migrate | `/storage/verify rowsTotalMatch=true` |
| D1 白天 | — | §4.3 空读回填观察窗口（白天压测读写） | 空读率 < 0.01% · 双写失败=0 |
| D1 晚 | 22-02 | §5.1 云盘 Backend 切换 + §5.2 B2 批量 migrate | `/files/backend/verify sampleRate=1 → 0 mismatch` |
| D2-D8 | 每日 03-04 | §4.4 图谱 7 天 100% 深度对账 · §5.3 云盘 fallback 监控 | 连续 7 天 `62/62 Δ≤1e-6` · fallback < 0.001% |
| D9 晚 | 22-23 | §4.5 图谱关双写 + §5.4 FS GC + 归档 SHA-256 | CR 单 4 位签字 → 变更关闭 |

---

## 6. 回滚流程（三档·任何阶段异常立刻执行）

### 6.1 图谱回滚（秒级·零数据丢失）
```
POST /storage/switch  { "provider":"sqlite", "dualWrite":true, "readPref":"primary" }
→ Helm：values 改回 DB_PROVIDER=sqlite → helm upgrade（灰度！）
```
保留双写 `true`（避免刚刚写目标库的增量丢失）；回滚后下一次切换还能利用已写好的目标数据。

### 6.2 云盘回滚（秒级·热切）
```
POST /storage/files/backend/switch  { "backend":"fs" }
→ Helm：FILE_BACKEND=fs → helm upgrade
```
真相源为 FS；已写入目标桶的 chunk 因 hash 不变，下次切回**零搬运直接命中**。

### 6.3 终极回退（万不得已）
```bash
kubectl scale deploy mox-core-local -n mox-system --replicas=0
# velero restore create --from-backup mox-backup-YYYYMMDD
# 或直接覆盖 PVC：
tar -zxf mox-data-snapshot-YYYYMMDD-HHMM.tar.gz -C /data/
tar -zxf mox-file-store-snapshot-YYYYMMDD.tar.gz -C /data/
kubectl scale deploy mox-core-local -n mox-system --replicas=3
```

---

## 7. Helm 伞图（K8s）生产操作补充

> K8s 环境不要直接 curl Pod 公网；**所有切换 = 修改 Helm values + 灰度 4 阶段推进**，严格对齐 [ops-manual.md §2.2](./ops-manual.md)。

```yaml
# custom-values.yaml（节选 · 最小生产）
global:
  gray: { enabled: true, canary: { weight: 1 } }   # 灰度第 1 阶段：1% 流量
  storageClass: mox-sc                             # 国密 SM4 LUKS StorageClass

mox-core-local:
  replicas: 3
  envFrom:
  - secretRef: { name: mox-db-secret }             # DB_PASSWORD / S3_SECRET_KEY
  env:
  # ========== 图谱切换（§4）==========
  - { name: DB_PROVIDER,     value: "postgresql" }
  - { name: DB_HOST,         value: "pg-vip.mox-system.svc.cluster.local" }
  - { name: DB_PORT,         value: "5432" }
  - { name: DB_NAME,         value: "ous" }
  - { name: DB_USER,         valueFrom: { secretKeyRef: { name: mox-db-secret, key: db_user } } }
  - { name: DB_SSL,          value: "true" }
  - { name: DB_DUAL_WRITE,   value: "true" }        # §4.1-§4.4 →  §4.5 改 false
  - { name: DB_READ_PREF,    value: "auto" }        # §4.3 auto →  §4.4 primary
  # ========== 云盘切换（§5）==========
  - { name: FILE_BACKEND,              value: "minio" }
  - { name: S3_CHUNKS_BUCKET,          value: "mox-chunks" }
  - { name: S3_ENDPOINT,               value: "http://minio.minio-ns.svc.cluster.local:9000" }
  - { name: AWS_REGION,                value: "cn-north-1" }
  - { name: FILE_MPU_CONCURRENCY,      value: "8" }
  volumeMounts:
  - { name: data-dir, mountPath: /data/mox }        # DATA_DIR → 保留 PVC（真相源归档）
  volumes:
  - name: data-dir
    persistentVolumeClaim: { claimName: mox-data-pvc }
```

应用 + 灰度推进：
```bash
helm dependency build deploy/helm/mox
helm upgrade --install mox deploy/helm/mox -n mox-system --create-namespace \
    --values custom-values.yaml --wait --timeout 10m
# 1% → 10% → 50% → 100% 自动推进：scripts/Gray-Warmup.ps1
# 任一阶段健康 < 95%：自动 rollback
```

在 Pod 内执行 API（安全）：
```bash
POD=$(kubectl get pods -n mox-system -l app=mox-core-local -o jsonpath='{.items[0].metadata.name}')
kubectl exec -n mox-system $POD -- curl -sS -X POST http://localhost:8080/storage/migrate \
  -H 'Content-Type: application/json' \
  -d '{"source":"sqlite","target":"postgresql","type":"all"}' | jq .
```

---

## 8. 日常运维篇：每日巡检 + Key 自检 + 监控告警

> 以下三份脚本建议入 SRE GitOps 仓库，作为每日 03:00 CronJob 自动跑。失败立即飞书/AlertManager 告警。

### 8.1 FS 目录结构完整性巡检（`daily-fs-check.sh`）

```bash
#!/usr/bin/env bash
set -euo pipefail
DATA_DIR=${DATA_DIR:-./data}
LOG="daily-fs-check-$(date +%Y%m%d).log"

echo "[1/6] 三目录存在性"
[ -d "$DATA_DIR/file-store/versions" ] || { echo "FAIL: versions/ 缺失"; exit 1; }
[ -d "$DATA_DIR/file-store/chunks" ]   || { echo "FAIL: chunks/ 缺失";   exit 1; }
[ -d "$DATA_DIR/file-store/mpu" ]      || mkdir -p "$DATA_DIR/file-store/mpu"

echo "[2/6] 256 散列子目录计数（空=正常，新系统=0）"
actual=$(ls -1 "$DATA_DIR/file-store/chunks" 2>/dev/null | wc -l)
echo "    chunks 子目录数：$actual / 256"

echo "[3/6] 孤立项点计数（chunks 磁盘数 vs DB refs）"
CHUNKS=$(find "$DATA_DIR/file-store/chunks" -type f | wc -l)
REFS=$(sqlite3 "$DATA_DIR/ous.db" "SELECT COUNT(*) FROM entities WHERE entity_type='file_chunk_refs';" 2>/dev/null || echo "DB_UNAVAIL")
echo "    FS chunks=$CHUNKS · DB refs=$REFS"
[ "$REFS" = "DB_UNAVAIL" ] || [ "$CHUNKS" -eq "$REFS" ] 2>/dev/null || \
  echo "WARN: 磁盘/DB 计数不等（刚迁移/冷读回填正常）"

echo "[4/6] MPU 僵尸（>24h 的上传会话）"
STALE=$(find "$DATA_DIR/file-store/mpu" -maxdepth 1 -type d -mmin +1440 | wc -l)
[ "$STALE" -eq 1 ] && echo "OK: 无僵尸 MPU（1=mpu/ 根）" || \
  echo "WARN: 僵尸 MPU=$((STALE-1))，超过 24h 未完成上传会话建议清理"

echo "[5/6] 文件名合规（64 hex）抽样 100 条"
find "$DATA_DIR/file-store/chunks" -type f | head -100 | \
  awk -F'/' '{fn=$NF; if (length(fn)!=64 || fn!~/^[0-9a-f]{64}$/) print "BAD:", fn}' > /tmp/bad-chunk-names.txt
[ ! -s /tmp/bad-chunk-names.txt ] && echo "OK: 抽样合规" || \
  { echo "FAIL: 非法文件名:"; cat /tmp/bad-chunk-names.txt; exit 3; }

echo "[6/6] SQLite integrity_check（前 100 页）"
sqlite3 "$DATA_DIR/ous.db" "PRAGMA integrity_check(100);" 2>/dev/null | grep -q "ok" && echo "OK: ous.db integrity=ok" || \
  { echo "FAIL: ous.db 损坏，立即回滚 §6.3"; exit 4; }

echo "ALL DONE $(date -Iseconds)" > "$LOG"
```

### 8.2 对象桶健康 + 对账（`daily-bucket-check.sh`）

```bash
#!/usr/bin/env bash
set -euo pipefail
ALIAS=myminio/mox-chunks

mc du "$ALIAS"                                  # 容量快照
mc ls --recursive "$ALIAS" | wc -l              # 对象数（应与 §8.1 DB refs 接近）
# MinIO Erasure Set 健康（4+2 → 最多掉线 2 节点仍可读）
mc admin health myminio || true
# 深度内容对账（抽样 1%）
curl -sS -X POST http://localhost:8080/storage/files/backend/verify \
     -H 'Content-Type: application/json' \
     -d '{"sampleRate":0.01,"deep":true}' | jq .
```

### 8.3 监控告警阈值（Prometheus / AlertManager）

```yaml
groups:
- name: mox-storage-switch
  rules:
  - alert: MoxGraphDualWriteFail
    expr: rate(mox_storage_dualwrite_failed_total[5m]) > 0
    for: 2m
    labels: { severity: critical }
    annotations: { summary: "图谱双写失败>0，立即检查目标库" }

  - alert: MoxGraphEmptyReadHigh
    expr: rate(mox_storage_read_empty_total[5m]) / rate(mox_storage_read_total[5m]) > 0.0001
    for: 15m
    labels: { severity: warning }
    annotations: { summary: "空读率>0.01%，需延长回填窗口或重跑 migrate" }

  - alert: MoxFileStoreFallbackHigh
    expr: rate(mox_filestore_read_fallback_total[5m]) > 100
    for: 10m
    labels: { severity: warning }
    annotations: { summary: "云盘回源>100次/5min → 批量迁移未完成或 VPC Endpoint 不通" }

  - alert: MoxQueryP99Skew
    expr: mox_query_p99 > 1.5 * avg_over_time(mox_query_p99[1w])
    for: 15m
    labels: { severity: warning }
    annotations: { summary: "P99 比周平均高 50%，检查目标库索引或网络" }
```

### 8.4 Key 一致性自检脚本（切换前后必跑）

```bash
#!/usr/bin/env bash
# verify-key-isomorphism.sh
set -euo pipefail
DATA_DIR=${DATA_DIR:-./data}
BUCKET_ALIAS=${1:-myminio/mox-chunks}
N=${2:-50}

find "$DATA_DIR/file-store/chunks" -type f | head -$N | while read -r fp; do
    hash=$(basename "$fp")
    prefix=$(basename "$(dirname "$fp")")
    key="${prefix}/${hash}"
    [ "${hash:0:2}" = "$prefix" ] || { echo "FAIL $fp：目录前缀≠hash.slice(0,2)"; exit 1; }
    if mc stat "$BUCKET_ALIAS/$key" >/dev/null 2>&1; then
        echo "OK    $key"
    else
        echo "SKIP  $key（目标桶尚无=未迁移冷数据，正常）"
    fi
done
echo "✅ $N 条 FS ↔ 目标桶 Key 同构自检通过"
```

---

## 9. API 速查表 + 故障处置 Runbook（F1-F14）

### 9.1 图谱存储引擎 API（5 条）

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/storage/status` | 总揽：provider + types.* 各类型计数 + features(dualWrite/readPref) |
| GET | `/storage/providers` | 可选列表（sqlite/postgresql/mysql/memory） |
| POST | `/storage/switch` | **热切换**：{ provider, dualWrite?, readPref?, conn? } |
| POST | `/storage/migrate` | **存量迁移**：{ source, target, type=all\|xxx, batchSize?, onConflict? } |
| POST | `/storage/verify` | **一致性对账**：{ type, deep, sampleRate } → rowsMatch + hashMismatch |
代码：[routes/storage.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/routes/storage.js) · [modules/storage.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/modules/storage.js)

### 9.2 云盘 Backend API（9 条）

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/storage/files/stats` | totalFiles / totalVersions / totalSize / byExt / graphCoverage |
| POST | `/storage/files/backend/switch` | 热切：{ backend, bucket, endpoint, accessKey, secretKey, region } |
| POST | `/storage/files/backend/migrate` | 批量迁移：{ from, to, concurrency, olderThanDays, chunkBatch } |
| POST | `/storage/files/backend/verify` | 对账：{ sampleRate, deep } → mismatchCount |
| POST | `/storage/files/gc` | **GC**：{ dryRun, graceDaysOverride }（先 dry-run ⚠️） |
| POST | `/storage/files/upload` | 上传（chunked + deduplicated + acl + quota 预检查） |
| GET | `/storage/files/:id` | 读文件（目标空读 → 自动回 FS 真相源） |
| POST | `/storage/files/:id/versions/:v/restore` | **零拷贝**版本恢复（vN 复制为新版本） |
代码：[file-store.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/file-store.js) · [chunk-backend.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js)

### 9.3 故障处置 Runbook（14 条）

| # | 现象 | 根因推测 | 处置 |
|---|---|---|---|
| F1 | migrate 报 duplicate key on `entities(entity_type,entity_id)` | 真相源**重复 id**（极少，老版本遗留） | 先 `onConflict=update` 重跑；仍 fail → 导出重复清单用 `_dup_` 处理后再迁 |
| F2 | `dualWrite=true` 后 `/storage/status` 仍是 false | env 注入后未重启 / Helm values 未生效 | `helm get values` → 确认 env 存在 → 灰度重启 Pods；`DB_DUAL_WRITE` 必须与主 provider 一致 |
| F3 | 云盘切换后某文件 404（冷数据） | 该 chunk 仍在 FS、未批量迁移 | 自动 B1 冷回填（下次读会搬）；或 §5.2 B2 热迁移批量 |
| F4 | MinIO AccessDenied | Policy 漏 `s3:Put/Get/ListBucket` | `mc admin policy attach` 重新 apply；Policy as code 评审 |
| F5 | verify hashMismatch 0.0x% 极少 | 双写窗口极快并发写触发非原子读 | 重跑 verify deep+100%；仍同 id 错 → upsert 目标库该条 |
| F6 | PG 云平台 SSL handshake 失败 | `DB_SSL=true` 未指定云根 CA | TencentDB / RDS 显式 `sslrootcert=/etc/ssl/certs/ca-certificates.crt` 或实例 CA |
| F7 | GC chunksFreed=0 · file-store 目录仍大 | 共享 chunk refs≥2（正常） | 等所有关联 vN softDelete + graceDays 到期，下次 GC 自动回收 |
| F8 | chunks/ 下 256 个目录全是空 | 正常（ensureDirs() 启动即幂等 mkdir） | 无需处理 |
| F9 | 手工误删 chunks/ 下若干文件 → ENOENT | 未走 GC 直接 rm → 内容丢失（逻辑） | 立即切回 `backend=fs`（若目标桶已存在同 hash chunk → 从桶回 FS migrate 反向填；或从快照 tar.gz 解压覆盖） |
| F10 | mpu/ 大量僵尸 part-XXXX.bin | 大文件上传取消 / 断网 | §8.1 脚本 MPU 扫描 + 手动 rm -rf 超 24h；或桶 Lifecycle Abort MPU 7d |
| F11 | MinIO/OSS 自定义桶前缀（`mox-prod/chunks/`）是否冲突 | 不冲突：桶级前缀 + xx/hash 无影响 | 在桶控制台/Policy 分层即可，代码 Key 仍 `xx/hash` |
| F12 | 两 FS 合并到同一 S3 桶？ | hash 相同自动合并（SHA-256=内容指纹） | mc cp --recursive 两个 chunks/ 到同一桶；DB 侧 `/storage/migrate` 多源合并 |
| F13 | chunks 目录单独挂大盘？ | 路径全相对 DATA_DIR，无盘符硬编码 | `mount --bind /mnt/big-disk/mox-chunks $DATA_DIR/file-store/chunks`；或整体迁 DATA_DIR + 改 env 重启 |
| F14 | 切换后 7 天对账仍有 <0.1% mismatch | 可能 UTF-8/JSON 浮点精度差异 | 重跑 migrate `onConflict=update`；必要时对目标库跑 `entities` 表 `REINDEX`（PG）/ `OPTIMIZE`（MySQL） |

---

## 10. 变更完成后 CR 归档清单（必入 hash_chain 审计链）

每次成功切换（图谱 / 云盘任一项），以下 7 类文件必须归档到**变更管理系统 + 审计链 hash_chain.log**：

| # | 文件 / 条目 | 说明 |
|---|---|---|
| 1 | 切换前 SHA-256：DATA_DIR 快照 + file-store 快照 | 两份 |
| 2 | `/storage/switch` ×2（开双写、关双写）请求/响应 JSON | 图谱；云盘额外附 `/files/backend/switch` |
| 3 | `/storage/migrate` + `/files/backend/migrate` summary.json | 所有类型 source / inserted / skipped 计数一致 |
| 4 | **7 天 × 2 份 daily verify**（图谱 100% deep + 云盘 deep）JSON | 连续 7 天 `62/62 Δ≤1e-6` · hashMismatch=0 · mismatchCount=0 |
| 5 | `/storage/files/gc` report JSON（含 dry-run + 实跑） | softPurged / hardDeleted / bytesFreed / warnings |
| 6 | CR 单签字页（SRE、业务、安全、架构师 4 位） | 切换范围、影响评估、§0.2 影响评估表签字 |
| 7 | 回滚演练记录（切换前 §6 完整演练 1 次） | 演练耗时 + 成功 / 失败结果 |

---

**文档版本控制**：本手册 v2.0.0 随平台版本维护；修改 PR 需要 SRE + Platform Team 双 Approve。
**上游关联文档**：[ops-manual.md](./ops-manual.md)（Helm 灰度 / 扩容 / 备份） · [ha-capacity-tco.md](./ha-capacity-tco.md)（容量规划与 HA 拓扑） · [xinchuang-matrix.md](./xinchuang-matrix.md)（信创矩阵与国密要求）。
