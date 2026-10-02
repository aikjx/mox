---
doc_id: EA-OPS-002
title: 专家联盟企业级生产部署配置模板
version: V1.0
authority: 🟢权威（生产落地模板；端口以 PORT-REGISTRY 为唯一权威，变量取值以 2026-09-29 代码事实为最终裁决）
last_updated: 2026-09-29
---

# 09 专家联盟企业级生产部署配置模板

> **权威链声明**
> - **端口** → [docs/api/PORT-REGISTRY.md](../api/PORT-REGISTRY.md) V1.2（唯一权威，本文不另立端口）。
> - **开关语义** → [08-normalized-architecture.md §五](08-normalized-architecture.md) + 本文；当二者与代码冲突时，**以代码为准**（本文每条变量均给 `文件:行号`，可 grep 复核）。
> - **SM4 加密语义** → [docs/api/API-CRYPTO-TRANSPORT.md](../api/API-CRYPTO-TRANSPORT.md)。
> - **现状事实** → [CURRENT-ARCHITECTURE.md](CURRENT-ARCHITECTURE.md) V1.1。
>
> **与 07 的关系**：[07-deployment.md](07-deployment.md) 为 🟡 V1.0 参考，仅给了进程清单与极简 compose 片段，**无 env 表、无 k8s、无检查清单**。本文是 07 的生产化补全：07 讲"怎么跑起来"，本文讲"怎么安全、可复制、可验证地上生产"。07 中与代码不符处见 §八。

## 适用范围与两档配置

| 档位 | 用途 | 鉴权 | 加密 | 存储 | HA |
|------|------|------|------|------|----|
| **生产档（prod）** | 内网试点 / 生产集群 | 全链路 `MOX_INTERNAL_TOKEN` + 关 dev | `MOX_API_CRYPTO=sm4` + 注入 KEY | `sqlite` | 按需开 |
| **本地/开发档（dev）** | 单机联调、CI | 关 dev_mode 直通 | 关（明文） | `file`/`memory` | 关 |

下文大表给"生产建议值"；docker-compose / k8s 默认落**生产档**，dev 档差异在每节内联标注。

---

## 一、部署拓扑

四进程职责（端口以 PORT-REGISTRY §3.2 为权威）：

| 进程 | 容器/服务名 | 端口 | 职责 | 依赖 |
|------|-------------|------|------|------|
| 平台网关 | `gateway` | **3080** | 唯一对外入口：REST + 内联专家逻辑（11 文件）、JWT 鉴权、审计、SSE | scheduler、executor、registry |
| 联盟调度器 | `scheduler` | **3100** | 任务调度/专家匹配/计划生成/HA 选主 | sqlite 共享库（HA 时） |
| 联盟执行器 | `executor` | **3200** | DAG 执行/节点调度/融合落盘 | 与 scheduler 同 sqlite 库 |
| 专家注册中心 | `registry` | **3400** | 专家登记/10:1:1 心跳聚合/健康分级 | 自身 sqlite + 快照 |

旁挂端口（不进本文 4 服务模板，但须知晓）：

- **3300**：AI 专家桥接基址（`expert_service.base_url`，scheduler → 外部专家服务，默认关闭；`MOX_ALLIANCE_EXPERT_SERVICE_ENABLED` 控制）。来源：boot-config `lib.rs:587`、PORT-REGISTRY §3.2。
- **3210**：codeengine-svc（`MOX_CODEENGINE_PORT` 覆盖），非本模板范畴。
- **3001**：orchestrator-svc，网关 `/health` 会探活 `ORCHESTRATOR_URL`（默认 `http://127.0.0.1:3001`，网关 `lib.rs:361`）；四进程模板不部署它时，网关健康检查里该依赖会显示 `down|unknown`，**不影响联盟主链路**。

调用链（多进程生产形态）：

```
前端/nginx → gateway:3080 (JWT)
              │  Authorization: Bearer $MOX_INTERNAL_TOKEN
              ├──→ scheduler:3100 (匹配/计划/排队)
              │       └──→ executor:3200 (DAG 逐节点执行, 同库回写状态)
              └──→ registry:3400 (专家登记/健康分级)
```

> 关键约束：scheduler:3100 / executor:3200 / registry:3400 **绝不暴露到公网**。它们只挂内部令牌校验（`internal_auth_layer`），无 JWT（08 缺口 N1），靠网络隔离兜底。见 §七。

---

## 二、环境变量总表（代码事实）

> 读取位置均相对仓库根 `infotopograph/`。"默认值"为代码内 `unwrap_or/unwrap_or_default` 的实际取值。

### 2.1 跨服务/安全（生产必配）

| 变量 | 服务 | 必配/可选/默认 | 生产建议值 | 代码位置（文件:行号） |
|------|------|----------------|------------|------------------------|
| `MOX_INTERNAL_TOKEN` | gateway + scheduler + executor + registry | **生产必配**（缺省=放行） | `openssl rand -hex 32` 输出值 | 网关出站：`platform/gateway/mox-platform-gateway-svc/src/alliance/registry_client.rs:27`、SDK 出站：`platform/domains/alliance/sdk/mox-alliance-http-sdk/src/alliance_remote.rs:121`；下游校验：`svc/mox-alliance-scheduler-svc/src/routes.rs:73`、`executor-svc/src/routes.rs:130`、`registry-svc/src/routes.rs:82` |
| `MOX_API_CRYPTO` | 全部 4 进程 | **生产必配** | `sm4` | 定义：`platform/foundation/mox-api-crypto/src/config.rs:9,45`；挂载：网关 `src/lib.rs:337`、`scheduler-svc/src/routes.rs:36`、`executor-svc/src/routes.rs:93`、`registry-svc/src/routes.rs:63` |
| `MOX_API_CRYPTO_KEY` | 全部 4 进程 | **生产必配**（否则回退开发密钥并 WARN） | 32 位 hex，`openssl rand -hex 16` | `platform/foundation/mox-api-crypto/src/config.rs:11,46`（开发密钥 `DEV_KEY` 见 `:54`） |
| `MOX_DEV_MODE` | gateway + 三 svc | **生产必关** | 不设 / `0` | 网关：`gateway/src/config.rs:97-100`；scheduler `routes.rs:70-72`、executor `routes.rs:127-129`、registry `routes.rs:79-81`（下游 release 默认 `cfg!(debug_assertions)=false`） |
| `MOX_DISABLE_DEV_TOKEN` | gateway | 生产建议 `true` | `true` | `gateway/src/config.rs:104` |
| `MOX_AUDIT_HMAC_SECRET` | gateway | **生产必改默认值** | `openssl rand -hex 32` | `gateway/src/alliance/experts_common.rs:573-574`（默认 `mox-experts-alliance-audit`） |
| `MOX_AUDIT_LOG_PATH` | gateway | 可选 | `/var/log/mox/experts-audit.ndjson` | `experts_common.rs:565-566`（默认 `data/audit/experts-audit.ndjson`） |
| `MOX_AUDIT_SINK` | gateway | 可选 | 不设（=文件落盘）；CI 可设 `noop` | `experts_common.rs:558-560`（值=`noop` 时不落盘） |

### 2.2 服务发现 / 跨进程组网

| 变量 | 服务 | 默认 | 生产建议值 | 代码位置 |
|------|------|------|------------|----------|
| `MOX_ALLIANCE_REMOTE_MODE` | gateway（经 http-sdk） | `auto` | `auto`（多进程）；纯单机内嵌可 `off` | `sdk/mox-alliance-http-sdk/src/alliance_remote.rs:101-105` |
| `MOX_ALLIANCE_SCHEDULER_URL` | gateway | 未设=走本地 | `http://scheduler:3100` | `alliance_remote.rs:106`；网关健康探活 `gateway/src/lib.rs:362` |
| `MOX_ALLIANCE_EXECUTOR_URL` | gateway | 未设=走本地 | `http://executor:3200` | `alliance_remote.rs:107`；`gateway/src/lib.rs:363` |

### 2.3 存储 / HA

| 变量 | 服务 | 默认 | 生产建议值 | 代码位置 |
|------|------|------|------------|----------|
| `MOX_ALLIANCE_STORAGE_MODE` | scheduler + executor | `file` | **`sqlite`** | scheduler `svc/mox-alliance-scheduler-svc/src/server.rs:114-123`（默认 `file`，兼容旧 `ALLIANCE_TASK_STORE`）；executor `svc/mox-alliance-executor-svc/src/state_sink.rs:175` |
| `MOX_ALLIANCE_SQLITE_BUSY_MS` | scheduler-core | `5000` | `5000`（多副本可调大） | `core/mox-alliance-scheduler-core/src/storage.rs:412-415` |
| `MOX_ALLIANCE_HA_MODE` | scheduler | off（未设即关） | 单副本不设；多副本 `on` | `svc/mox-alliance-scheduler-svc/src/ha.rs:74-77`（认 `1/on/true/yes`） |
| `MOX_ALLIANCE_HA_LEASE_MS` | scheduler | `10000` | `10000` | `ha.rs:78` |
| `MOX_ALLIANCE_HA_TICK_MS` | scheduler | `lease/3`（≥1000） | 不设（自动）；须满足 `tick*2 < lease` | `ha.rs:80-81`、校验 `:99-106` |
| `MOX_ALLIANCE_HA_STALL_MS` | scheduler | `300000` | `300000`（勿设 0） | `ha.rs:83`、校验 `:107-109` |
| `MOX_ALLIANCE_HA_HOLDER` | scheduler | `scheduler-<pid>` | k8s 里设为 Pod 名（`$(POD_NAME)`） | `ha.rs:85-87` |
| `MOX_ALLIANCE_HA_DB` | scheduler | `data/alliance_tasks.db` | 与任务库同路径（共享 PVC） | `ha.rs:91-94` |

> HA 硬约束：`HA_MODE=on` 时任务库必须是共享 `sqlite`（`server.rs:108-109` `ha_storage_ok` 要求 `mode=="sqlite"`）。file/memory 下开 HA 会启动失败。

### 2.4 各 svc 监听与业务参数

| 变量 | 服务 | 默认 | 生产建议值 | 代码位置 |
|------|------|------|------------|----------|
| `MOX_GATEWAY_HOST` | gateway | `0.0.0.0` | `0.0.0.0`（容器内） | `gateway/src/main.rs:23` |
| `MOX_GATEWAY_PORT` | gateway | `3080` | `3080` | `gateway/src/main.rs:24-27` |
| `MOX_HOST_ROLE` | gateway | `all` | `all` | `gateway/src/lib.rs:467` |
| `MOX_ALLIANCE_CONFIG_FILE` | scheduler + executor | 内置默认 yml | 容器内路径 `config/alliance-scheduler.yml` | `scheduler-svc/src/bin/main.rs:30`、`executor-svc/src/bin/main.rs:27` |
| `MOX_ALLIANCE_EXPERTS_FILE` | scheduler | `config/alliance-experts.yml` | 同左（挂载专家 yml） | `scheduler-svc/src/bin/main.rs:63-65` |
| `MOX_ALLIANCE_SERVER_HOST` | scheduler + executor | `0.0.0.0` | `0.0.0.0` | boot-config `core/mox-alliance-boot-config/src/lib.rs:508,538` |
| `MOX_ALLIANCE_SERVER_PORT` | scheduler | `3100`（boot 默认） | `3100` | boot-config `lib.rs:65,509`（executor 默认 `3200` 见 `:592-593`） |
| `MOX_ALLIANCE_EXECUTOR_MODE` | executor | `expert` | `expert`（生产无 mock 路径） | boot-config `lib.rs:542-554,594` |
| `MOX_ALLIANCE_REGISTRY_ADDR` | registry | `0.0.0.0:3400` | `0.0.0.0:3400`（host:port 合体） | `registry-svc/src/app_state.rs:83-86,55` |
| `MOX_ALLIANCE_REGISTRY_DB` | registry | `./data/registry.db` | PVC 路径 `/var/lib/mox/registry.db` | `app_state.rs:88-91,56` |
| `MOX_ALLIANCE_REGISTRY_SNAPSHOT` | registry | `./data/registry_instances.json` | 不设=纯内存；要持久化给文件路径 | `app_state.rs:93-94,57` |
| `MOX_ALLIANCE_REGISTRY_REAP_MS` | registry | `5000` | `5000` | `app_state.rs:96-100,58` |
| `MOX_ALLIANCE_REGISTRY_PROBE_ENABLED` | registry | `false` | **`1`**（生产建议开） | `app_state.rs:103-105,61` |
| `MOX_ALLIANCE_REGISTRY_PROBE_INTERVAL_MS` | registry | `30000` | `30000` | `app_state.rs:107-111,62` |
| `MOX_ALLIANCE_REGISTRY_PROBE_TIMEOUT_MS` | registry | `5000` | `5000` | `app_state.rs:114-118,63` |
| `MOX_ALLIANCE_REGISTRY_HEALTHY_MIN` | registry | `0.80` | `0.80` | `app_state.rs:121-125,65` |
| `MOX_ALLIANCE_REGISTRY_DEGRADED_MAX_RATIO` | registry | `0.20` | `0.20` | `app_state.rs:128-132,66` |

### 2.5 执行器重试/超时（executor-core，通常保持默认）

| 变量 | 默认 | 代码位置 |
|------|------|----------|
| `MOX_EXECUTOR_NODE_TIMEOUT_MS` | `60000` | `core/mox-alliance-executor-core/src/expert_executor.rs:93` |
| `MOX_EXECUTOR_MAX_RETRIES` | `3` | `expert_executor.rs:98` |
| `MOX_EXECUTOR_INITIAL_RETRY_DELAY_MS` | `1000` | `expert_executor.rs:103` |
| `MOX_EXECUTOR_MAX_RETRY_DELAY_MS` | `30000` | `expert_executor.rs:108` |
| `MOX_EXECUTOR_BACKOFF_FACTOR` | `2.0` | `expert_executor.rs:113` |

### 2.6 网关本地存储/附件（非联盟专属，按需）

| 变量 | 默认 | 代码位置 |
|------|------|----------|
| `MOX_EXPERTS_DB_PATH` | `data/experts.db` | `gateway/src/alliance/experts_db.rs:32,34,46` |
| `MOX_STORAGE_ROOT` | `<cwd>/data/storage` | `gateway/src/cloud.rs:44-48` |
| `MOX_UPLOAD_DIR` | `./data/uploads` | `gateway/src/file_storage/api.rs:28-29` |
| `MOX_STORE_DB_PATH` | （const 定义） | `gateway/src/store_json.rs:27` |
| `MOX_VOICE_UPSTREAM_URL` | `http://127.0.0.1:8012` | `gateway/src/voice.rs:34-35` |
| `MOX_S3_ENDPOINT` / `MOX_S3_BUCKET` / `MOX_S3_ACCESS_KEY_ID` / `MOX_S3_SECRET_ACCESS_KEY` | 未设=S3 后端不启用 | `gateway/src/storage_backend.rs:88-91` |
| `MOX_S3_REGION` | `us-east-1` | `storage_backend.rs:95` |

### 2.6.1 多租户（A1，2026-10-01 阶段一；2026-10-02 阶段二配额）

- **租户来源**：可信 JWT 身份的 `tenant_id` 声明下发（auth 中间件注入），单租户部署所有用户 `tenant_id=default`，与现状零回归。
- **请求头 `X-Tenant-Id`**：仅作一致性校验——若携带，必须等于当前身份的 `tenant_id`，否则网关返回 **403**（防越权换租户）；不携带则取身份自带租户。无有效身份一律 **401**。
- **SQLite**：单文件 `data/experts.db`，schema 自动升到 v2（含 `tenant_id` 复合键），无需运维介入；多租户数据同库行级隔离，非每租户独立文件。

#### 租户配额（A1 阶段二，2026-10-02）

| 变量 | 默认 | 含义 | 代码位置 |
|------|------|------|----------|
| `MOX_ALLIANCE_QUOTA_EXPERTS_PER_TENANT` | `1000` | 单租户可注册专家数上限；超限 `POST /api/experts` 返回 **409**（响应体 `data.quota/used`）。管理写面每次创建时读取，env 修改即时生效；缺失/非正整数回退默认，单租户零回归 | `gateway/src/alliance/experts_common.rs:122,128`；`experts_registry.rs:383` |

- 默认 `1000` 不误伤现有单租户（内置种子专家仅 10 个，且经启动直写注册表、不经 handler，不受配额约束）。
- 配额按租户独立计数（per-tenant 内层注册表 `len()`）；软删除专家记录仍占槽位（id 冲突检查永久保留）。
- **未引入**：租户级配置表/管理 UI（阶段三）、DAG 计划/任务数配额、会话/图谱节点配额——见 backend-fix-report A1 阶段二节的维度取舍理由。

### 2.7 生产档推荐 env 块（直接复制）

```dotenv
# === 跨服务安全（4 进程必须完全一致）===
MOX_INTERNAL_TOKEN=<openssl rand -hex 32>
MOX_API_CRYPTO=sm4
MOX_API_CRYPTO_KEY=<openssl rand -hex 16>
MOX_DEV_MODE=0
MOX_DISABLE_DEV_TOKEN=true
MOX_AUDIT_HMAC_SECRET=<openssl rand -hex 32>
MOX_AUDIT_LOG_PATH=/var/log/mox/experts-audit.ndjson
# === 组网 ===
MOX_ALLIANCE_REMOTE_MODE=auto
MOX_ALLIANCE_SCHEDULER_URL=http://scheduler:3100
MOX_ALLIANCE_EXECUTOR_URL=http://executor:3200
# === 存储（scheduler+executor 一致，指向共享卷）===
MOX_ALLIANCE_STORAGE_MODE=sqlite
MOX_ALLIANCE_SQLITE_BUSY_MS=5000
# === registry ===
MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=1
```

### 2.8 本地/开发档推荐 env 块

```dotenv
MOX_DEV_MODE=1                 # 放行内部鉴权，仅本地
MOX_API_CRYPTO=off             # 明文，便于抓包
MOX_ALLIANCE_REMOTE_MODE=off   # 全进程内，不连下游
MOX_ALLIANCE_STORAGE_MODE=file
MOX_AUDIT_SINK=noop            # 不落审计盘
# 不设 MOX_INTERNAL_TOKEN / MOX_AUDIT_HMAC_SECRET（用默认）
```

---

## 三、docker-compose 模板（单机 / 内网试点）

> 假设仓库已产出 4 个二进制镜像（同名 tag）。把 `<internal-token>` / `<crypto-key-hex>` / `<audit-secret>` 替换为 §七 生成的值。下游三 svc 端口**不映射到宿主机**，仅 gateway 暴露 3080。

```yaml
services:
  gateway:
    image: mox/gateway:latest
    container_name: mox-gateway
    restart: unless-stopped
    ports:
      - "3080:3080"
    environment:
      MOX_GATEWAY_HOST: 0.0.0.0
      MOX_GATEWAY_PORT: "3080"
      MOX_HOST_ROLE: all
      # 安全
      MOX_INTERNAL_TOKEN: "<internal-token>"
      MOX_API_CRYPTO: sm4
      MOX_API_CRYPTO_KEY: "<crypto-key-hex-32>"
      MOX_DEV_MODE: "0"
      MOX_DISABLE_DEV_TOKEN: "true"
      # 审计
      MOX_AUDIT_HMAC_SECRET: "<audit-secret>"
      MOX_AUDIT_LOG_PATH: /var/log/mox/experts-audit.ndjson
      # 组网（指向 compose 服务名）
      MOX_ALLIANCE_REMOTE_MODE: auto
      MOX_ALLIANCE_SCHEDULER_URL: http://scheduler:3100
      MOX_ALLIANCE_EXECUTOR_URL: http://executor:3200
      # 本地数据落盘
      MOX_EXPERTS_DB_PATH: /var/lib/mox/experts.db
    volumes:
      - gateway-data:/var/lib/mox
      - gateway-logs:/var/log/mox
    depends_on:
      scheduler:
        condition: service_healthy
      executor:
        condition: service_healthy
      registry:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://127.0.0.1:3080/health"]
      interval: 15s
      timeout: 3s
      retries: 5

  scheduler:
    image: mox/alliance-scheduler:latest
    container_name: mox-scheduler
    restart: unless-stopped
    environment:
      MOX_ALLIANCE_SERVER_HOST: 0.0.0.0
      MOX_ALLIANCE_SERVER_PORT: "3100"
      MOX_ALLIANCE_CONFIG_FILE: /app/config/alliance-scheduler.yml
      MOX_ALLIANCE_EXPERTS_FILE: /app/config/alliance-experts.yml
      MOX_INTERNAL_TOKEN: "<internal-token>"
      MOX_API_CRYPTO: sm4
      MOX_API_CRYPTO_KEY: "<crypto-key-hex-32>"
      MOX_DEV_MODE: "0"
      MOX_ALLIANCE_STORAGE_MODE: sqlite
      MOX_ALLIANCE_SQLITE_BUSY_MS: "5000"
    volumes:
      - alliance-data:/app/data
    expose:
      - "3100"
    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://127.0.0.1:3100/health"]
      interval: 15s
      timeout: 3s
      retries: 5

  executor:
    image: mox/alliance-executor:latest
    container_name: mox-executor
    restart: unless-stopped
    environment:
      MOX_ALLIANCE_SERVER_HOST: 0.0.0.0
      MOX_ALLIANCE_SERVER_PORT: "3200"
      MOX_ALLIANCE_CONFIG_FILE: /app/config/alliance-executor.yml
      MOX_INTERNAL_TOKEN: "<internal-token>"
      MOX_API_CRYPTO: sm4
      MOX_API_CRYPTO_KEY: "<crypto-key-hex-32>"
      MOX_DEV_MODE: "0"
      MOX_ALLIANCE_STORAGE_MODE: sqlite
      # 与 scheduler 同库文件（共享卷 alliance-data）
    volumes:
      - alliance-data:/app/data
    expose:
      - "3200"
    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://127.0.0.1:3200/health"]
      interval: 15s
      timeout: 3s
      retries: 5

  registry:
    image: mox/alliance-registry:latest
    container_name: mox-registry
    restart: unless-stopped
    environment:
      MOX_ALLIANCE_REGISTRY_ADDR: 0.0.0.0:3400
      MOX_ALLIANCE_REGISTRY_DB: /var/lib/mox/registry.db
      MOX_ALLIANCE_REGISTRY_SNAPSHOT: /var/lib/mox/registry_instances.json
      MOX_ALLIANCE_REGISTRY_PROBE_ENABLED: "1"
      MOX_ALLIANCE_REGISTRY_PROBE_INTERVAL_MS: "30000"
      MOX_ALLIANCE_REGISTRY_PROBE_TIMEOUT_MS: "5000"
      MOX_INTERNAL_TOKEN: "<internal-token>"
      MOX_API_CRYPTO: sm4
      MOX_API_CRYPTO_KEY: "<crypto-key-hex-32>"
      MOX_DEV_MODE: "0"
    volumes:
      - registry-data:/var/lib/mox
    expose:
      - "3400"
    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://127.0.0.1:3400/health"]
      interval: 15s
      timeout: 3s
      retries: 5

volumes:
  gateway-data:
  gateway-logs:
  alliance-data:
  registry-data:
```

要点：
- scheduler 与 executor **共享 `alliance-data` 卷**，两者 `MOX_ALLIANCE_STORAGE_MODE=sqlite` 指向同一 `data/alliance_tasks.db`（`server.rs:150`、`state_sink.rs:178`），这是任务真源一致的前提。
- dev 档：把 `MOX_DEV_MODE=1`、`MOX_API_CRYPTO=off`、`MOX_ALLIANCE_REMOTE_MODE=off`，并删掉 `MOX_INTERNAL_TOKEN`。

---

## 四、k8s 模板（多副本 / 生产集群）

### 4.1 ConfigMap（非敏感）

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: mox-alliance-env
data:
  MOX_API_CRYPTO: "sm4"
  MOX_DEV_MODE: "0"
  MOX_DISABLE_DEV_TOKEN: "true"
  MOX_ALLIANCE_REMOTE_MODE: "auto"
  MOX_ALLIANCE_SCHEDULER_URL: "http://scheduler:3100"
  MOX_ALLIANCE_EXECUTOR_URL: "http://executor:3200"
  MOX_ALLIANCE_STORAGE_MODE: "sqlite"
  MOX_ALLIANCE_SQLITE_BUSY_MS: "5000"
  MOX_ALLIANCE_REGISTRY_PROBE_ENABLED: "1"
  MOX_ALLIANCE_REGISTRY_PROBE_INTERVAL_MS: "30000"
  MOX_AUDIT_LOG_PATH: "/var/log/mox/experts-audit.ndjson"
```

### 4.2 Secret（敏感，base64 占位）

生成命令（Linux）：

```bash
openssl rand -hex 32 | tr -d '\n' | base64    # MOX_INTERNAL_TOKEN
openssl rand -hex 16 | tr -d '\n' | base64    # MOX_API_CRYPTO_KEY（32 hex 字符）
openssl rand -hex 32 | tr -d '\n' | base64    # MOX_AUDIT_HMAC_SECRET
```

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: mox-alliance-secret
type: Opaque
data:
  MOX_INTERNAL_TOKEN: "<base64-of-openssl-rand-hex-32>"
  MOX_API_CRYPTO_KEY: "<base64-of-32-hex-chars>"
  MOX_AUDIT_HMAC_SECRET: "<base64-of-openssl-rand-hex-32>"
```

> 生产建议把 Secret 换成 Vault / SealedSecret / ExternalSecret，不要裸提交。

### 4.3 Deployments（×4）

要点：gateway 对外 `Service`（LoadBalancer/NodePort 或 Ingress）；scheduler/executor/registry 仅 `ClusterIP`。**只有 scheduler 开 HA 多副本**，且必须 2+ 副本 + 共享 sqlite PVC。

```yaml
# ---- gateway ----
apiVersion: apps/v1
kind: Deployment
metadata: { name: mox-gateway }
spec:
  replicas: 2
  selector: { matchLabels: { app: mox-gateway } }
  template:
    metadata: { labels: { app: mox-gateway } }
    spec:
      containers:
        - name: gateway
          image: mox/gateway:latest
          ports: [{ containerPort: 3080 }]
          envFrom:
            - configMapRef: { name: mox-alliance-env }
            - secretRef: { name: mox-alliance-secret }
          env:
            - name: MOX_GATEWAY_PORT
              value: "3080"
          resources:
            requests: { cpu: "250m", memory: "256Mi" }
            limits:   { cpu: "1",    memory: "1Gi" }
          readinessProbe:
            httpGet: { path: /health, port: 3080 }
            initialDelaySeconds: 5
            periodSeconds: 10
          livenessProbe:
            httpGet: { path: /health, port: 3080 }
            initialDelaySeconds: 15
            periodSeconds: 20
          volumeMounts:
            - { name: data, mountPath: /var/lib/mox }
            - { name: logs, mountPath: /var/log/mox }
      volumes:
        - name: data
          persistentVolumeClaim: { claimName: mox-gateway-data }
        - name: logs
          persistentVolumeClaim: { claimName: mox-gateway-logs }
# A2 无状态化阶段一（2026-10-02）：gateway 多副本共享同一 experts.db 的 HA 语义
# 前提：所有副本的 MOX_EXPERTS_DB_PATH 指向同一共享卷路径（如 /var/lib/mox/experts.db，
#       即上面 mox-gateway-data PVC，RWX 或 Recreate 避免双写同一文件）。
# 形态：plans/orchestration_history/favorites 三项冷数据已以 SQLite 为唯一真相——
#       任一副本写穿，其余副本不重启即经读路径实时查库读到（跨实例一致），无需 sticky。
# 边界：registry/graph 高频态仍各副本进程内（阶段二外移）；sessions 仍单副本内可恢复（N11）。
#       sqlite 单写者约束下，写竞争由 WAL + busy_timeout(5s) + 应用层重试兜底。
---
# ---- scheduler（HA 多副本示例）----
apiVersion: apps/v1
kind: Deployment
metadata: { name: mox-scheduler }
spec:
  replicas: 2                       # HA 必须 ≥2；共享 sqlite 用 RWX PVC
  strategy: { type: Recreate }     # sqlite 单写者，避免 RWO 卷多副本同时写
  selector: { matchLabels: { app: mox-scheduler } }
  template:
    metadata: { labels: { app: mox-scheduler } }
    spec:
      containers:
        - name: scheduler
          image: mox/alliance-scheduler:latest
          ports: [{ containerPort: 3100 }]
          envFrom:
            - configMapRef: { name: mox-alliance-env }
            - secretRef: { name: mox-alliance-secret }
          env:
            - name: MOX_ALLIANCE_HA_MODE
              value: "on"
            - name: MOX_ALLIANCE_HA_LEASE_MS
              value: "10000"
            - name: MOX_ALLIANCE_HA_STALL_MS
              value: "300000"
            - name: MOX_ALLIANCE_HA_HOLDER
              valueFrom: { fieldRef: { fieldPath: metadata.name } }   # Pod 名
            - name: MOX_ALLIANCE_HA_DB
              value: /var/lib/mox/alliance_tasks.db
          resources:
            requests: { cpu: "250m", memory: "256Mi" }
            limits:   { cpu: "1",    memory: "1Gi" }
          readinessProbe: { httpGet: { path: /health, port: 3100 } }
          livenessProbe:  { httpGet: { path: /health, port: 3100 } }
          volumeMounts:
            - { name: shared-tasks, mountPath: /var/lib/mox }
      volumes:
        - name: shared-tasks
          persistentVolumeClaim: { claimName: mox-alliance-shared }   # RWX 或单实例 RWO
---
# ---- executor（无状态，可多副本；与 scheduler 同共享库）----
apiVersion: apps/v1
kind: Deployment
metadata: { name: mox-executor }
spec:
  replicas: 2
  selector: { matchLabels: { app: mox-executor } }
  template:
    metadata: { labels: { app: mox-executor } }
    spec:
      containers:
        - name: executor
          image: mox/alliance-executor:latest
          ports: [{ containerPort: 3200 }]
          envFrom:
            - configMapRef: { name: mox-alliance-env }
            - secretRef: { name: mox-alliance-secret }
          resources:
            requests: { cpu: "250m", memory: "256Mi" }
            limits:   { cpu: "2",    memory: "2Gi" }
          readinessProbe: { httpGet: { path: /health, port: 3200 } }
          livenessProbe:  { httpGet: { path: /health, port: 3200 } }
          volumeMounts:
            - { name: shared-tasks, mountPath: /var/lib/mox }
      volumes:
        - name: shared-tasks
          persistentVolumeClaim: { claimName: mox-alliance-shared }
---
# ---- registry（单副本即可；多副本需外部共享库）----
apiVersion: apps/v1
kind: Deployment
metadata: { name: mox-registry }
spec:
  replicas: 1
  selector: { matchLabels: { app: mox-registry } }
  template:
    metadata: { labels: { app: mox-registry } }
    spec:
      containers:
        - name: registry
          image: mox/alliance-registry:latest
          ports: [{ containerPort: 3400 }]
          envFrom:
            - configMapRef: { name: mox-alliance-env }
            - secretRef: { name: mox-alliance-secret }
          env:
            - name: MOX_ALLIANCE_REGISTRY_ADDR
              value: "0.0.0.0:3400"
            - name: MOX_ALLIANCE_REGISTRY_DB
              value: /var/lib/mox/registry.db
          resources:
            requests: { cpu: "100m", memory: "128Mi" }
            limits:   { cpu: "500m", memory: "512Mi" }
          readinessProbe: { httpGet: { path: /health, port: 3400 } }
          volumeMounts:
            - { name: data, mountPath: /var/lib/mox }
      volumes:
        - name: data
          persistentVolumeClaim: { claimName: mox-registry-data }
```

### 4.4 Services

```yaml
apiVersion: v1
kind: Service
metadata: { name: mox-gateway }
spec:
  type: LoadBalancer            # 或 ClusterIP + Ingress
  selector: { app: mox-gateway }
  ports: [{ port: 80, targetPort: 3080 }]
---
apiVersion: v1
kind: Service
metadata: { name: scheduler }
spec:
  type: ClusterIP
  selector: { app: mox-scheduler }
  ports: [{ port: 3100, targetPort: 3100 }]
---
apiVersion: v1
kind: Service
metadata: { name: executor }
spec:
  type: ClusterIP
  selector: { app: mox-executor }
  ports: [{ port: 3200, targetPort: 3200 }]
---
apiVersion: v1
kind: Service
metadata: { name: registry }
spec:
  type: ClusterIP
  selector: { app: mox-registry }
  ports: [{ port: 3400, targetPort: 3400 }]
```

> HA 注意：`MOX_ALLIANCE_HA_MODE=on` 要求 scheduler 多副本 + `STORAGE_MODE=sqlite` 共享存储（`server.rs:108-109`）。共享 sqlite 在 k8s 上要么用 RWX 卷（NFS/CephFS），要么外置单实例 SQLite（如云盘）。**sqlite 是单写者**，scheduler 用 `strategy: Recreate` 避免并发写；busy 等待窗口见 `MOX_ALLIANCE_SQLITE_BUSY_MS`。若需真正多写仲裁，属 08 §十 P1 目标态（换 PG/etcd LeaseStore）。

---

## 五、前端部署

### 5.1 构建

```bash
cd frontend-ui
VITE_BASE=./ npm run build        # 产物 frontend-ui/dist
```

前端通过相对路径 `/api` 访问后端，**不硬编码网关地址**（dev 由 vite proxy 转发，prod 由 nginx 反向代理）。构建期可注入：`VITE_OUS_API_TOKEN`（RBAC 令牌，`vite.config.js:54`）、`VITE_BASE`。

### 5.2 nginx 反向代理（含 SSE 长连接）

实时进度走 **SSE**（`GET /api/alliance/tasks/:id/logs/stream`），**无 WebSocket**（全 crate `WebSocketUpgrade` 零命中，见 08 §三-25）。SSE 必须关缓冲、放宽读超时：

```nginx
server {
  listen 80;
  root /usr/share/nginx/html;        # 前端 dist
  index index.html;

  location / {
    try_files $uri $uri/ /index.html;
  }

  location /api/ {
    proxy_pass http://gateway:3080;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;

    # SSE 长连接关键三件套
    proxy_buffering off;
    proxy_cache off;
    proxy_read_timeout 3600s;
    proxy_send_timeout 3600s;
  }
}
```

> 不开 `proxy_buffering off` 会导致 SSE 事件被 nginx 攒着不推，前端表现为"日志卡住"。

---

## 六、部署后一致性检查清单

逐项验证开关真的生效（替换 `$TOKEN` 为 `MOX_INTERNAL_TOKEN` 值，`$GW=localhost:3080`）。

### 6.1 进程存活

```bash
curl -fsS http://$GW/health                       # gateway，JSON，dependencies.scheduler/executor 应为 up
curl -fsS http://localhost:3100/health
curl -fsS http://localhost:3200/health
curl -fsS http://localhost:3400/health
```

gateway `/health` 是 JSON（`lib.rs:365`），含 `dependencies.{orchestrator,scheduler,executor}`。

### 6.2 指标端点格式差异（易踩）

```bash
curl -fsS http://$GW/metrics         # gateway：Prometheus 文本（o11y）
curl -fsS http://localhost:3100/metrics   # scheduler：JSON 快照（routes.rs:112），不是 Prometheus 文本
curl -fsS http://localhost:3200/metrics    # executor：JSON 快照
```

> Prometheus 抓 scheduler/executor 需写 JSON→Prom 适配层（08 缺口 N7）；别直接对它们用 text 解析器。

### 6.3 HA 选主

```bash
curl -fsS http://localhost:3100/leadership        # HA_MODE=on 时才有意义（CURRENT §6.2）
# 两副本中应恰好一个 role=leader；HA 未开时该端点语义为空。
```

### 6.4 内部鉴权链路（200 vs 401）

```bash
# 不带 token 直连下游 → 期望 401（证明 internal_auth_layer 生效）
curl -i http://localhost:3100/api/alliance/tasks
# 带正确 token → 期望 2xx/4xx(业务)，不是 401
curl -i -H "Authorization: Bearer $TOKEN" http://localhost:3100/api/alliance/tasks
```

> `/health`、`/metrics`、`/leadership`、`/api/registry/health` 是公开白名单（`routes.rs:78`），不带 token 也 200，**不要拿它们断言鉴权**。

### 6.5 SM4 加密是否生效

带协商头请求，响应应为信封密文（`API-CRYPTO-TRANSPORT §2.3`）：

```bash
curl -i -H "x-mox-crypto: sm4-gcm+gzip" http://$GW/api/experts
# 响应体应形如 {"crypto":{"alg":"SM4-GCM","zip":"gzip","nonce":"...","ct":"...","tag":"..."}}
# 响应头回显 x-mox-crypto；不带协商头则是明文信封（向后兼容）。
```

若响应仍是明文 `{code,msg,data}` 直接展开，说明 `MOX_API_CRYPTO` 未设成 `sm4` 或未协商头。

### 6.6 存储模式验证

```bash
# sqlite 模式：库文件应存在且有 -wal 文件
ls -l /var/lib/mox/alliance_tasks.db /var/lib/mox/alliance_tasks.db-wal
# scheduler 与 executor 指向同一文件（inode/路径一致）
```

> 注：当前代码 **grep `PRAGMA user_version` 零命中**（08 缺口 N3），故不能用 user_version 校验 schema；以库文件 + `-wal` 副产物存在性判断即可。

### 6.7 审计日志

```bash
# 文件应为 NDJSON，每行一个对象，且含 HMAC 签名字段
tail -f /var/log/mox/experts-audit.ndjson
# 触发一次登录/任务操作后应有新行；未改 MOX_AUDIT_HMAC_SECRET 时启动日志不报警告。
```

---

> **已自动化**：本节 7 项检查已落成一键校验脚本，见 [`scripts/verify-deploy.sh`](scripts/verify-deploy.sh)（Linux k8s 生产）与 [`scripts/verify-deploy.ps1`](scripts/verify-deploy.ps1)（Windows/内网试点），用法与退出码见 [`scripts/README.md`](scripts/README.md)。

## 七、生产安全基线

1. **内部令牌 `MOX_INTERNAL_TOKEN`**：长度 ≥32 hex（`openssl rand -hex 32`）。网关出站（`registry_client.rs:27`、`alliance_remote.rs:121`）与三 svc 入站（`routes.rs:73/130/82`）必须**同值**。轮换用"双 token 滚动"：先在网关配 `old,new`（如中间件支持多值），再逐 svc 切 `new`，最后撤 `old`。当前代码为单值精确比对（`routes.rs:87`），滚动期需短暂重启窗口，务必在低峰。
2. **SM4 加密生产必开**：`MOX_API_CRYPTO=sm4` 且**必须显式注入 `MOX_API_CRYPTO_KEY`（32 hex）**。未注入会回退内置开发密钥 `mox-dev-sm4-key!`（`config.rs:54`）并打 WARN——生产出现该 WARN 即视为事故。
3. **存储**：`sqlite` + WAL（`storage.rs:426`）。单写者约束下，多副本 HA 用共享卷 + `Recreate`，busy 等待 `MOX_ALLIANCE_SQLITE_BUSY_MS`（默认 5000ms）。
4. **登记状态 ≠ 探活**：`availability.status`（`online/busy/offline/away`）是**登记值**，不是健康探测结果（08 §八）。真正的主动探活在 registry-svc 侧——**2026-10-02 起默认开启**（`Config::default().health_probe_enabled=true`，D8），无需显式配置即启动后台主动探测；如需回退「仅被动心跳租约」，设 `MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=0/false/no/off`。下方模板里显式写 `=1` 仅为自文档化、与默认一致。健康检查语义：k8s/nginx 探 `/health` 只代表进程活着，不代表下游依赖健康；gateway `/health` 恒 200，依赖状态看 body 的 `dependencies`。
5. **审计**：`MOX_AUDIT_HMAC_SECRET` **必改默认值**（默认 `mox-experts-alliance-audit`，`experts_common.rs:574`）。日志落盘 `MOX_AUDIT_LOG_PATH`，按 NDJSON 行归档，建议配 logrotate 按天/按大小切割。
6. **网络隔离**：:3100/:3200/:3400 仅 ClusterIP / 内网，绝不绑 NodePort/公网（下游无 JWT，08 缺口 N1，靠网络兜底）。

---

## 八、变量冲突与废弃说明（与 07/08 差异）

### 8.1 本文相对 08 §五 的补全（08 缺、代码有）

08 §五只列了 12 个变量。代码实测以下变量**真实存在但 08 表遗漏**，本文已补齐（均给文件:行号）：

| 遗漏变量 | 为何重要 | 代码位置 |
|----------|----------|----------|
| `MOX_INTERNAL_TOKEN` | **最关键**：内部服务间鉴权开关，08 完全没列 | 见 §2.1 |
| `MOX_DEV_MODE` / `MOX_DISABLE_DEV_TOKEN` | 鉴权跳过/门禁 | gateway `config.rs:97,104`；三 svc `routes.rs:70/127/79` |
| `MOX_API_CRYPTO_KEY` | SM4 密钥，08 表只列了开关 | `mox-api-crypto/src/config.rs:11` |
| `MOX_ALLIANCE_SCHEDULER_URL` / `..._EXECUTOR_URL` | 多进程组网必配 | `alliance_remote.rs:106-107` |
| `MOX_ALLIANCE_HA_TICK_MS` / `..._HA_DB` / `..._HA_HOLDER` | HA 调参 | `ha.rs:81,91,85` |
| `MOX_ALLIANCE_CONFIG_FILE` / `..._EXPERTS_FILE` | 配置/专家 yml 注入 | scheduler `main.rs:30,63` |
| `MOX_ALLIANCE_REGISTRY_DB/SNAPSHOT/REAP_MS/PROBE_INTERVAL_MS/PROBE_TIMEOUT_MS/HEALTHY_MIN/DEGRADED_MAX_RATIO` | registry 全量参数 | `app_state.rs:88-132` |
| `MOX_AUDIT_SINK` | 审计落盘开关 | `experts_common.rs:558` |
| `MOX_EXECUTOR_INITIAL_RETRY_DELAY_MS/MAX_RETRY_DELAY_MS/BACKOFF_FACTOR` | 退避调参 | `expert_executor.rs:103,108,113` |
| boot-config 动态覆盖（`MOX_ALLIANCE_SERVER_HOST/PORT`、`..._SCHEDULER_*`、`..._EXECUTOR_BRIDGE_*`、`..._EXPERT_SERVICE_*`、`..._STORAGE_PATH`、`..._EXECUTOR_*`） | 经 `env_value()` 前缀拼接读取，字面 grep 抓不到 | `boot-config/src/lib.rs:468-575` |

### 8.2 与 07-deployment.md 的差异（以代码为准）

| 07 写法 | 代码事实 | 处置 |
|---------|----------|------|
| §6 `db.path = ./data/alliance.db` | 代码无此路径。实际：scheduler sqlite `data/alliance_tasks.db`（`server.rs:150`）、网关专家库 `data/experts.db`（`experts_db.rs:34`）、registry `./data/registry.db`（`app_state.rs:56`） | 以代码为准；07 该行为占位、未对齐 |
| §3 compose 无 environment、无健康检查、无 token | 代码要求 `MOX_INTERNAL_TOKEN`/`MOX_API_CRYPTO` 等才能安全组网 | 用本文 §3 替代 |
| §5.1 `/metrics` 写"Prometheus（gateway 侧）" | scheduler/executor `/metrics` 是 JSON 快照，非 Prometheus 文本（08 N7） | 以本文 §6.2 为准 |

### 8.3 文档幽灵 / 废弃变量

- **`ALLIANCE_TASK_STORE`**（旧名）：代码仍兼容读取并打 WARN（`server.rs:116-119`），新部署用 `MOX_ALLIANCE_STORAGE_MODE`。属"兼容期旧变量"，非纯幽灵。
- **`EXECUTOR_MODE`**（旧名）：同上，兼容读取并 WARN（boot-config `lib.rs:545-548`），新部署用 `MOX_ALLIANCE_EXECUTOR_MODE`。
- **未发现"文档提了、代码零命中"的纯幽灵 MOX_ 变量**：本文扫描的 52 个 `MOX_*` 字面量 + boot-config 动态键均能落到读取点。08 §五列的 12 个变量全部代码命中（行号有 ±2~8 行漂移，本文已用最新行号修正）。

---

## 九、变更记录

- V1.0（2026-09-29）：基于当日代码事实 grep 产出。共核对 **52 个 `MOX_*` 字面量读取点 + boot-config 动态前缀键**；端口对齐 PORT-REGISTRY V1.2（3080/3100/3200/3400，旁挂 3300/3210）；SM4 语义对齐 API-CRYPTO-TRANSPORT。所有变量均可按"文件:行号"复核。
