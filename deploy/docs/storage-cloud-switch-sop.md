# 知识图谱存储 + 云盘 Backend · 切换运维操作手册 (SOP)

**版本：** 1.0.0
**发布日期：** 2026-08-25
**适用范围：** Mox Platform（backend-node 单机 / Helm 伞图 K8s 部署）
**维护者：** SRE & Mox Platform Team
**操作等级：** L3 生产操作（变更前请在 CHANGE-LOG.md 登记 CR 单）
**前置手册阅读：** [ops-manual.md](./ops-manual.md) · [ha-capacity-tco.md](./ha-capacity-tco.md)

---

## 0. 变更须知（操作前必须通读）

> **三大铁律，违反任一条 SRE 立即叫停：**
>
> 1. **默认自研永远是真相源** —— 任何切换必须从自研（SQLite + FS）出发，不允许绕过自研首写直接把第三方当主。
> 2. **双写对账窗口 ≥ 7 天** —— 切读后必须保留双写 7 天，每日跑一致性扫描，连续 7 天 `62/62 Δ≤1e-6` 才允许关双写。
> 3. **一键回滚链常驻** —— 任何不一致立刻执行 `§7 回滚流程`，`readPref=auto` 兜底回源保证数据零丢失。

**影响范围评估（必查）**：
- 图谱 `nodes/edges` 量 > 10W 或 云盘 `totalSize > 1TB` → 建议 **业务低峰窗口** 执行迁移阶段（§3.2 / §4.2）。
- 走 Helm 伞图 + 双活 DR → 必须 `helm upgrade` 灰度 4 阶段变更 env，禁止直接改 Pod 环境变量。
- 云存储（OSS/COS/S3）→ 必须提前申请 **VPC 内 Endpoint**，禁止公网直接打对象桶，避免带宽费 + 延迟抖动。

---

## 1. 架构全景与选项表

### 1.1 两模块 × 三档位

| 模块 | 默认（自研 ⭐） | 开源方案 | 云平台方案 | 切换变量 |
|---|---|---|---|---|
| 知识图谱存储引擎 | `sqlite` + JSON 磁盘双写 (`data/graph_nodes.json` / `graph_edges.json`) | PostgreSQL 13+ · MySQL 5.7+ · MariaDB 10.6+ | 腾讯云 TencentDB · 阿里云 PolarDB/RDS · AWS Aurora | `DB_PROVIDER` |
| 云盘 Chunk Backend | `fs` 两级散列 + 引用计数 GC (`DATA_DIR/file-store/chunks/<xx>/<sha256>`, `versions`, `mpu`) | MinIO · Ceph RGW · Garage（S3 协议兼容） | AWS S3 · 阿里云 OSS · 腾讯云 COS · 七牛 Kodo | `FILE_BACKEND` |

### 1.2 后端能力对比（选型建议）

| 能力 | 自研 SQLite+FS | PG/MySQL + MinIO | 云数据库 + 云对象存储 |
|---|---|---|---|
| **TCO（0-6 月）** | ⭐最低（零授权、零运维） | 中（自建 3 节点集群） | 高（云托管费 + 带宽费） |
| **性能（单机）** | ⭐最优（WAL + 本地 FS 零网络） | 良（1Gb 内网 RPC） | 中（VPC Endpoint < 2ms） |
| **横向扩展** | 单机上限（32C/64G 约 500W 图节点） | ⭐最佳（分片+只读副本） | ⭐托管最佳（按 API 按量） |
| **合规 / 数据主权** | ⭐完全可控 | 可控（自建机房） | 需评估（云平台合规矩阵） |
| **RPO/RTO** | RPO=本地快照；RTO≈5min | RPO<30s/RTO<5min（主从） | RPO<1s/RTO<1min（云托管） |
| **推荐适用** | 企业内网单机 · 起步 · 开发测试 | 中大型企业自建集群 · 敏感数据 | 公有云部署 · 多云 · 弹性用户量 |

代码锚点：
- 图谱存储抽象：[storage/index.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/index.js)
- 云盘 Backend 抽象与路由：[storage/chunk-backend.js#L332-L345](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js#L332-L345)
- 配置默认值：[config.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/config.js)

---

## 2. 环境变量总表（一张表配置所有切换）

### 2.1 知识图谱存储引擎

| 变量 | 默认（自研） | 开源 PG/MySQL | 云数据库 | 说明 |
|---|---|---|---|---|
| `DB_PROVIDER` | `sqlite` | `postgresql` 或 `mysql` | `postgresql` 或 `mysql` | **主开关** |
| `DB_HOST` | - | 内网 DNS / VIP | 云平台 host（强制 TLS） | |
| `DB_PORT` | - | `5432` / `3306` | 云平台端口 | |
| `DB_NAME` | `ous` | `ous` | `ous` | 建议建库 `CREATE DATABASE ous ENCODING 'UTF8';` |
| `DB_USER` / `DB_PASSWORD` | - | 自建账号 | 云平台账号 | 云平台建议 IAM 临时凭证；**禁止写死到 values.yaml** |
| `DB_SSL` | `false` | 可选 | **强制 `true`** | 公网/跨 VPC 必须开 |
| `DB_DUAL_WRITE` | `false` | `true` 迁移期 | `true` 迁移期 | §3.3 第①步开；§3.5 第⑤步关 |
| `DB_READ_PREF` | `auto` | `auto`→`primary` | `auto`→`primary` | 迁移期 `auto`（空读回源+回填）；切读后 `primary` |
| `DATA_DIR` | `./data` | 同左（**保留作真相源归档**） | 同左 | 切换后绝不删；用作 GC 前对账基准 |

### 2.2 云盘 Backend

| 变量 | 默认（自研） | MinIO / Ceph | 云平台 S3/OSS/COS | 说明 |
|---|---|---|---|---|
| `FILE_BACKEND` | `fs` | `minio` 或 `s3` | `s3` | **主开关**（minio/s3/oss 走同一个 S3ChunkBackend） |
| `S3_CHUNKS_BUCKET` | - | `mox-chunks` | 云平台桶名 | 必须**提前创建并设为私有** |
| `S3_MANIFEST_BUCKET` | （空 = 走本地+DB） | 可选 `mox-manifests` | 可选 | 版本 manifest 放对象桶；**留空 = 走本地 entities + versions/ 目录，推荐** |
| `S3_ENDPOINT` | - | `http://minio:9000`（K8s） | OSS/COS 区域域名（VPC Endpoint 优先） | 留空 = 走 AWS 公共区域 |
| `S3_ACCESS_KEY` / `S3_SECRET_KEY` | - | MinIO 账号 | 云平台 AK/SK | **强烈建议用 K8s Secret + envFrom**，**禁止明文** |
| `AWS_REGION` | - | 忽略 | `cn-north-1` 等 | AWS SDK 所需 |
| `FILE_SOFT_DELETE` | `true` | 同左 | 同左 | 切换后仍然有效（软删→graceDays→GC purge） |
| `FILE_GRACE_DAYS` | `30` | 同左 | 同左 | |
| `FILE_MPU_CONCURRENCY` | `4` | `8`（内网更快） | `8` | 云平台看带宽调整 |
| `FILE_MAX_QUOTA_BYTES` | `0` | 同左 | 同左 | 0=不限；>0 时上传前预检查 |

---

## 3. 流程 A：知识图谱存储引擎切换 SOP（5 步法）

> 以 `sqlite` → `postgresql` 为例；MySQL 完全同构，替换 `DB_PROVIDER=mysql` 即可。
> 以 `sqlite` → 云 TencentDB for PG 为例，走 Helm 伞图灰度 env 变更（单机直接改 env）。

### 3.1 预检查清单（Step 0 · CR 前必须全绿）

```bash
# [单机] 检查当前存储状态
curl -s http://localhost:3010/storage/status | jq .
# 预期：provider=sqlite, types.graph_nodes=N, types.graph_edges=N, features.dualWrite=false

# [Helm] 检查当前 env
helm get values mox -n mox-system -a | grep -E 'DB_|DATA_DIR'

# [目标库连通性 + 建库]（PG 示例）
psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d postgres -c \
  "CREATE DATABASE ous ENCODING 'UTF8' LC_COLLATE='C.UTF8' LC_CTYPE='C.UTF8' TEMPLATE=template0;"
# 确认建库成功
psql -h $DB_HOST -U $DB_USER -d ous -c "SELECT 1;"

# [目标库权限] 最小权限原则
# CREATE USER mox_app WITH LOGIN PASSWORD '***';
# GRANT CONNECT ON DATABASE ous TO mox_app;
# GRANT ALL PRIVILEGES ON SCHEMA public TO mox_app;  -- storage/ 用它 CREATE TABLE + DML

# [数据量评估] 预估迁移耗时
du -sh $DATA_DIR/ous.db $DATA_DIR/graph_nodes.json $DATA_DIR/graph_edges.json
# 经验值：PG 批量 insert 约 25k 行/秒 / MySQL 约 18k 行/秒（1Gb 内网）

# [快照备份] 迁移前对 DATA_DIR 做一次只读快照（任何存储介质）
# ⚠️ 迁移过程中若失败直接回滚到本快照
tar -zcf mox-data-snapshot-$(date +%Y%m%d-%H%M).tar.gz -C $(dirname $DATA_DIR) $(basename $DATA_DIR)
```

### 3.2 第①步：启动双写模式

```
POST /storage/switch
Content-Type: application/json

{
  "provider": "postgresql",
  "dualWrite": true,
  "readPref": "primary",
  "conn": {
    "host": "pg-vip.internal",
    "port": 5432,
    "database": "ous",
    "user": "mox_app",
    "password": "****",        // 生产：改从 Secret 注入，不要走 HTTP 传密码！
    "ssl": true                 // 云平台必须 true
  }
}
```

**⚠️ 生产最佳实践**：`conn` 不传明文密码；改为**先配置环境变量**再调用 `switch`（不传 conn 即读 env）。

**单机部署方式（推荐）**：
```bash
# 1. 先注入 env（通过 systemd EnvironmentFile / docker --env-file / helm values）
DB_PROVIDER=postgresql
DB_HOST=pg-vip.internal
DB_PORT=5432
DB_NAME=ous
DB_USER=mox_app
DB_PASSWORD=xxx-from-secret
DB_SSL=true
DB_DUAL_WRITE=true
DB_READ_PREF=primary

# 2. 重启 Pod（Helm）或进程（单机）
## Helm：helm upgrade mox ... --set mox-core.env...（灰度 4 阶段推进）
## 单机：systemctl restart mox-backend

# 3. 读回状态确认双写
curl -s http://localhost:3010/storage/status | jq .features
# 预期：{ dualWrite: true, readPref: "primary", provider: "postgresql" }
```

**验证点**：在前台新增/修改一条图谱节点，检查：
- `data/graph_nodes.json` 有新条目 → ✅ 真相源写 OK
- PG `SELECT COUNT(*) FROM entities WHERE entity_type='graph_nodes';` 同样有 → ✅ 双写 OK

---

### 3.3 第②步：存量迁移（JSON + SQLite → 目标）

```
POST /storage/migrate
Content-Type: application/json

{
  "source": "sqlite",        // 固定 = 当前真相源
  "target": "postgresql",    // 目标
  "type": "all",             // all = graph_nodes + graph_edges + resources + files + kb_documents
                             // 单独跑："graph_nodes" | "graph_edges" | ...
  "batchSize": 500,          // 可选；默认 500
  "onConflict": "update"     // skip 或 update（幂等）
}
```

**Response**：
```json
{
  "ok": true,
  "summary": {
    "graph_nodes": { "source": 72, "inserted": 72, "updated": 0, "skipped": 0 },
    "graph_edges": { "source": 103, "inserted": 103, "updated": 0, "skipped": 0 },
    "resources":   { "source": 4,  "inserted": 4,  "updated": 0, "skipped": 0 }
  },
  "status": "complete",
  "elapsedMs": 2891
}
```

**Post-验证（强制跑 verify）**：
```
POST /storage/verify
{ "type": "all" }
```
预期 `diff.rowsTotalMatch=true` 且 `diff.hashMismatch=0`。任何不匹配 → 排查主键冲突或 JSON 结构字段；`onConflict=update` 重跑一次通常解决。

---

### 3.4 第③步：空读验证 + 回填（72h 观察窗口）

```
POST /storage/switch
{
  "provider": "postgresql",
  "dualWrite": true,      // 双写仍保留
  "readPref": "auto"      // 关键：读走 PG，空读回真相源 SQLite → 自动回填 PG
}
```

**监控关键指标**（72h）：
- 指标 `mox_storage_read_empty_total`（空读回源次数）：72h 空读率 < 0.01% = 可进入下一步。
- 指标 `mox_storage_dualwrite_failed_total`（双写失败）：必须 = 0。
- 指标 `mox_query_p99`：与切换前 ±10% 内。若飙高 → 目标库加索引（建议 `entities(entity_type, entity_id)` 复合索引，storage/index 已 DDL 过）。

---

### 3.5 第④步：切读 + 双写保留 7 天对账

```
POST /storage/switch
{
  "provider": "postgresql",
  "dualWrite": true,
  "readPref": "primary"   // 正式：所有读走 PG，不再回源
}
```

**对账窗口（7 天，每天 03:00 跑）**：
```
POST /storage/verify  { "type":"all", "deep": true, "sampleRate": 1 }  // 100% 深度对账
```
企业级验收：**连续 7 天 `rowsTotalMatch=true` + `hashMismatch=0` + `62/62 Δ≤1e-6`**（对齐 24 号 A+ §5 硬标准）。

---

### 3.6 第⑤步：关双写 + 旧存储归档

```
POST /storage/switch
{
  "provider": "postgresql",
  "dualWrite": false,     // 正式停用自研首写
  "readPref": "primary"
}
```

**归档操作（绝不删真相源）**：
```bash
# 把 ./data 目录打只读压缩包
tar -zcf mox-data-archive-before-switch-pg-$(date +%Y%m%d).tar.gz \
  ./data --transform 's,^,mox-data-archive/,'
# 上传到对象桶（冷存储） + 生成 SHA256 审计链条目
sha256sum mox-data-archive-*.tar.gz >> $AUDIT_DIR/hash-chain.log
```

**可选（30 天后）**：若目标库 30 天全稳，可把 `DATA_DIR/*.json` 除了 `kb_documents.json`、`resources.json` 外，标记为 `archived/`（不物理删）。

---

## 4. 流程 B：云盘 Backend 切换 SOP（FS → MinIO / S3 / OSS）

> 以 `fs` → `minio` 为例；OSS/COS/S3 仅 `S3_ENDPOINT` + 桶不同，其余命令完全等价。

### 4.1 预检查清单（Step 0）

```bash
# [1] 确认当前云盘体量
curl -s http://localhost:3010/storage/files/stats | jq .
# 关注 totalFiles, totalVersions, totalSize, filesByExtension

# [2] 确认 MinIO / 对象桶 可达 + 权限
# 建议用 mc（minio client）：
mc alias set myminio http://minio:9000 $AK $SK
mc mb myminio/mox-chunks --region=cn-north-1     # 桶不存在则建
mc anonymous set none myminio/mox-chunks          # **私有！禁止公开**
mc ls myminio/mox-chunks                           # 连通 OK

# [3] 生命周期策略（推荐）—— 软删对象 30 天后实际删除
mc ilm rule add --expire-days 30 --noncurrentversion-expire-days 30 myminio/mox-chunks

# [4] 冷/热分层（云平台可选：标准层 30d → 低频层 90d → 归档层）
# 阿里云 OSS：生命周期规则 "transition to IA after 30d, to Archive after 90d"

# [5] 快照备份：整个 file-store/ 目录打包（迁移失败即回滚）
du -sh $DATA_DIR/file-store
tar -zcf mox-file-store-snapshot-$(date +%Y%m%d).tar.gz -C $DATA_DIR file-store
```

### 4.2 第①步：切 Backend 到目标（MPU 与去重自动生效）

**SOP-A（推荐，用 env 生效，不丢上下文）**：
```bash
# 注入新环境变量
FILE_BACKEND=minio                         # 或 s3 / oss（同一 S3ChunkBackend 代码）
S3_CHUNKS_BUCKET=mox-chunks
S3_ENDPOINT=http://minio:9000              # OSS: https://oss-cn-hz-internal.aliyuncs.com（VPC 内网）
S3_ACCESS_KEY=minioadmin                   # **强烈建议：K8s Secret 挂载 envFrom**
S3_SECRET_KEY=minioadmin
AWS_REGION=cn-north-1
FILE_MPU_CONCURRENCY=8                     # 看带宽调到 4/8/16

# Helm：灰度 4 阶段。单机：systemctl restart mox-backend
```

**SOP-B（热切换 API，进程不重启 —— 仅适用于已加载 backend）**：
```
POST /storage/files/backend/switch
{
  "backend": "minio",                 // fs | s3 | minio | oss
  "bucket": "mox-chunks",
  "endpoint": "http://minio:9000",
  "accessKey": "minioadmin",
  "secretKey": "minioadmin",
  "region": "cn-north-1"
}
```

**验证点**：
```bash
# 上传一个测试文件 >10MB（会走分块 + hasChunk 去重 + 引用计数）
curl -s -X POST -F "file=@./test-12m.bin" http://localhost:3010/storage/files/upload \
  | jq .id,.size,.chunkCount
# mc 观察桶
mc ls --summarize myminio/mox-chunks/
# 预期看到类似：<xx>/<64-char-hash> （两级目录 + SHA-256 hash key，与本地 FS 同规则）
```

---

### 4.3 第②步：存量 chunk 迁移（冷/热双模式）

> 切换 Backend 只影响**新写**；存量 chunk 仍在本地 FS。两种策略二选一：

#### 策略 B1（推荐·冷迁移·业务无感）
存量文件读时 **自动回源 + 回填**（后端路由层 Fallback）：
- `readFile()` → 目标 MinIO 没该 hash → 自动读本地 FS chunk → 写入目标桶（去重检查）→ 下一次读直接命中桶
- 不需要停机迁移；**约 7-30 天热点数据自然迁移**
- 冷数据（720 天无访问）保留在本地 FS 即可，或跑一次性批量脚本

#### 策略 B2（热迁移·一次性批量）
```
POST /storage/files/backend/migrate
{
  "from": "fs",
  "to": "minio",
  "concurrency": 16,
  "olderThanDays": 0,                // 0=全部；>0=只迁移超过 N 天的热点
  "chunkBatch": 500
}
```
Response：
```json
{
  "ok": true,
  "total": 8943, "migrated": 8941, "skipped": 2, "failed": 0,
  "bytes": 9876543210, "elapsedMs": 394211,
  "skippedHashes": ["deadbeef...(already exists in target)"]
}
```

---

### 4.4 第③步：读验证 + 双 Backend 对账窗口（7 天）

> 云盘切换**没有双写概念**（chunk hash 全局不变），但 `FileStore` 的读路由支持：目标空读 → 自动回源 FS。
> 把 `readPref` 调为 `"target"`，观察 7 天回源次数即可。

```bash
# 查看 stats
curl -s http://localhost:3010/storage/files/stats | jq '{totalFiles, graphCoverage, byExt}'

# 每日对账（抽样 10% 文件，对比 hash + size）：
POST /storage/files/backend/verify
{ "sampleRate": 0.1, "deep": true }
# 预期：verified=OK, mismatchCount=0
```

---

### 4.5 第④步：GC 本地 FS 旧 chunk（引用计数 + 回源兜底）

```
POST /storage/files/gc
{
  "dryRun": true,          // 先 dry-run 预览
  "graceDaysOverride": 30  // 必须 ≥ FILE_GRACE_DAYS；默认 30
}
# 确认 dry-run 没问题后：
POST /storage/files/gc  { "dryRun": false, "graceDaysOverride": 30 }
```

**GC 报告核心字段**（强制存档）：
```json
{
  "ok": true,
  "softPurged": 3, "hardDeleted": 18, "refsRemoved": 44,
  "chunksFreed": 18, "bytesFreed": 20971520,
  "warnings": ["chunk abcdef: refcount > 0, skipped (shared between v1/v2)"]
}
```

⚠️ **GC 后仍保留本地 FS 空目录结构 30 天**（冷读兜底防极端）；`warnings` 非空 → 先排查共享 chunk 引用再 purge。

---

## 5. 组合操作：同时切换存储引擎 + 云盘（企业上云典型）

**推荐顺序（错峰两晚窗口）**：

| 日 | 窗口 | 操作 | 验收 |
|---|---|---|---|
| D-1 晚 | 22:00-00:00 | 执行 §3.1 + §4.1 预检查 + 快照 | 两份 SHA-256 快照归档 → 审计链留痕 |
| D0 晚 | 22:00-02:00 | §3.2 双写启动 + §3.3 存量迁移（图） | `/storage/verify rowsTotalMatch=true` |
| D1 白天 | - | **§3.3 空读回填观察窗口** | `mox_storage_read_empty_total < 0.01%` |
| D1 晚 | 22:00-02:00 | §4.2 云盘 Backend 切换 + §4.3 B2 热迁移 | `/files/backend/verify sampleRate=1 → mismatchCount=0` |
| D2-D8 | - | **7 天双写对账窗口**（§3.5 + §4.4） | 连续 7 天 `62/62 Δ≤1e-6` + 云盘 0 mismatch |
| D9 晚 | 22:00-23:00 | §3.6 关双写 + §4.5 GC + 归档 | 完成 CR 单签字 → 变更关闭 |

---

## 6. Helm 伞图（K8s）操作补充

> K8s 环境不直接 curl Pod；**所有切换 = 修改 Helm values + 灰度 4 阶段推进**，严格对齐 [ops-manual.md §2.2](./ops-manual.md)。

```yaml
# custom-values.yaml（节选）
global:
  gray:
    enabled: true
    canary:
      weight: 1

mox-core-local:
  envFrom:
  - secretRef:
      name: mox-db-secret           # DB_PASSWORD / S3_SECRET_KEY 在这里
  env:
  # === 图谱切换 ===
  - name: DB_PROVIDER
    value: "postgresql"
  - name: DB_HOST
    value: "pg-vip.mox-system.svc.cluster.local"
  - name: DB_PORT
    value: "5432"
  - name: DB_NAME
    value: "ous"
  - name: DB_USER
    valueFrom: { secretKeyRef: { name: mox-db-secret, key: db_user } }
  - name: DB_SSL
    value: "true"
  - name: DB_DUAL_WRITE
    value: "true"
  - name: DB_READ_PREF
    value: "auto"         # 第①-③步；切读后改 "primary"
  # === 云盘切换 ===
  - name: FILE_BACKEND
    value: "minio"
  - name: S3_CHUNKS_BUCKET
    value: "mox-chunks"
  - name: S3_ENDPOINT
    value: "http://minio.minio-ns.svc.cluster.local:9000"
  - name: AWS_REGION
    value: "cn-north-1"
  - name: FILE_MPU_CONCURRENCY
    value: "8"
  volumeMounts:
  - name: data-dir
    mountPath: /data/mox   # DATA_DIR 指向这里；务必保留为 PVC（真相源归档）
  volumes:
  - name: data-dir
    persistentVolumeClaim:
      claimName: mox-data-pvc
```

应用（灰度 4 阶段）：
```bash
helm dependency build deploy/helm/mox
helm upgrade --install mox deploy/helm/mox -n mox-system --create-namespace \
    --values custom-values.yaml --wait --timeout 10m
# 推进灰度：scripts/Gray-Warmup.ps1（ops-manual §2.2）
# 每阶段 < 95% 健康 → 自动 helm rollback
```

**在 Pod 内执行迁移/verify API（不要暴露到公网）**：
```bash
POD=$(kubectl get pods -n mox-system -l app=mox-core-local -o jsonpath='{.items[0].metadata.name}')
kubectl exec -n mox-system $POD -- \
  curl -s -X POST http://localhost:3010/storage/migrate \
       -H 'Content-Type: application/json' \
       -d '{"source":"sqlite","target":"postgresql","type":"all"}' | jq .
```

---

## 7. 回滚流程（任何阶段异常立刻执行）

### 7.1 图谱存储回滚（秒级·零数据丢失）
```bash
# 读立即切回真相源 + 仍保双写（避免丢刚刚写目标库的增量）
POST /storage/switch
{
  "provider": "sqlite",
  "dualWrite": true,       // 若之前关了 → 回滚后留双写，下次再迁移用
  "readPref": "primary"
}
# Helm：values.yaml 改回 DB_PROVIDER=sqlite → helm upgrade（灰度！）
```

### 7.2 云盘回滚（秒级·热切）
```bash
POST /storage/files/backend/switch  { "backend": "fs" }
# Helm：values FILE_BACKEND=fs → helm upgrade
```
`fs` 是真相源，所有已写入目标桶的 chunk 仍存在（hash 一致不会重复），下次切回去直接用，**零复制零损失**。

### 7.3 终极回退（万不得已）
恢复 D-1 晚打的 **DATA_DIR 快照 tar.gz** 与 `file-store` 快照：
```bash
kubectl scale deploy mox-core-local -n mox-system --replicas=0
# 恢复 PVC（velero restore 或 直接覆盖挂载目录）
tar -zxf mox-data-snapshot-YYYYMMDD-HHMM.tar.gz -C /data/
tar -zxf mox-file-store-snapshot-YYYYMMDD.tar.gz -C /data/
kubectl scale deploy mox-core-local -n mox-system --replicas=3
```

---

## 8. API 速查表

### 8.1 图谱存储引擎

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/storage/status` | 总揽：provider + types.* 各类型计数 + features(dualWrite/readPref) |
| GET | `/storage/providers` | 可选引擎列表（sqlite/postgresql/mysql/memory） |
| POST | `/storage/switch` | **热切换**：{ provider, dualWrite?, readPref?, conn? } |
| POST | `/storage/migrate` | **存量迁移**：{ source, target, type=all\|xxx, batchSize?, onConflict? } |
| POST | `/storage/verify` | **一致性对账**：{ type=all, deep, sampleRate }；输出 rowsMatch + hashMismatch |

代码锚点：[routes/storage.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/routes/storage.js) · [modules/storage.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/modules/storage.js)

### 8.2 云盘 Backend

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/storage/files/stats` | totalFiles / totalVersions / totalSize / byExt / graphCoverage |
| POST | `/storage/files/backend/switch` | **热切换 Backend**：{ backend, bucket, endpoint, accessKey, secretKey, region } |
| POST | `/storage/files/backend/migrate` | **批量迁移 chunk**：{ from, to, concurrency, olderThanDays, chunkBatch } |
| POST | `/storage/files/backend/verify` | **对账**：{ sampleRate, deep } |
| POST | `/storage/files/gc` | **GC 软删 + 零引用 chunk 物理删**：{ dryRun, graceDaysOverride } |
| POST | `/storage/files/upload` | 上传（chunked + deduplicated + acl + quota 预检查） |
| GET | `/storage/files/:id` | 读文件（目标空读 → 自动回源 FS） |
| POST | `/storage/files/:id/versions/:v/restore` | 版本恢复（把 vN 建成新版本，不物理覆盖） |

代码锚点：[file-store.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/file-store.js) · [chunk-backend.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js)

---

## 9. 常见故障与处置（Runbook 片段）

| # | 现象 | 根因推测 | 处置 |
|---|---|---|---|
| F1 | `/migrate` 报 `duplicate key` on `entities(entity_type,entity_id)` | 真相源里有**重复 id**（极少；老版本遗留） | 先 `onConflict=update` 重跑；仍失败 → 导出 id 重复清单，`_dup_` 后缀处理后再迁 |
| F2 | `dualWrite: true` 后 `status` 仍显示 false | env 注入后未重启（或 Helm values 未生效） | `helm get values` → 确认 env 存在 → 重启 Pods；`DB_DUAL_WRITE` 必须和主 provider 一致 |
| F3 | 云盘切换后读某文件 404（冷数据） | 该 chunk 仍在 FS，未做批量迁移 | 策略 B1 自然回源即可（下一次读自动回填桶）；批量跑 §4.3 B2 |
| F4 | MinIO `AccessDenied` 上传失败 | 桶策略 / IAM Policy 漏了 `s3:PutObject`/`GetObject`/`ListBucket` | mc 重新设置 policy；或用 policy as code（`admin-policy.json`）重新 apply |
| F5 | `/verify` hashMismatch > 0 且 <0.1% | 双写窗口内有**极快并发写**触发了非原子读 | 重跑 `verify sampleRate=1 deep=true`；若仍同一 id 错 → 对比 JSON 原文，手动 `upsert` 目标库那条 |
| F6 | PG 云平台 `SSL handshake` 失败 | `DB_SSL=true` 但没配云平台的根 CA / ClientCert | 云平台托管 PG 需额外配 `sslrootcert=/etc/ssl/certs/ca-certificates.crt`；TencentDB 常需要显式指定 RDS CA |
| F7 | GC 报告 chunksFreed=0 但 file-store 目录仍很大 | chunk 被共享（`refs[]` ≥ 2），引用计数正确保留 | 正常；等关联文件全部 vN 软删 + graceDays 到期后下次 GC 自动回收 |

---

## 10. 操作完成后 CR 归档清单

每次成功切换后，以下文件必须归档到**变更管理系统 + 审计链 hash_chain**：

| # | 文件名/条目 | 说明 |
|---|---|---|
| 1 | 切换前 SHA-256：DATA_DIR 快照、file-store 快照 | 两份 |
| 2 | `/storage/switch`（2 次：开双写、关双写）请求响应 JSON | |
| 3 | `/storage/migrate` summary.json | 所有类型 source/inserted/skipped 一致 |
| 4 | **7 天 × `/storage/verify`**（或 files verify）响应 JSON | 连续 7 天全绿 |
| 5 | `/storage/files/gc` report | softPurged / hardDeleted / bytesFreed |
| 6 | 变更 CR 单签字页（SRE、业务、安全、架构师 4 位） | |
| 7 | 回滚演练记录（每次切换前必须演练 1 次 §7） | 演练耗时 + 成功 |

---

**文档版本控制**：本手册随平台版本维护；修改 PR 必须 SRE + Platform Team 双 Approve。
**关联文档**：[ops-manual.md](./ops-manual.md) · [ha-capacity-tco.md](./ha-capacity-tco.md) · [xinchuang-matrix.md](./xinchuang-matrix.md)
