# 部署一致性 · 企业级就绪度核查（gap-deployment）

> 核查日期：2026-09-16　｜　范围：只读核查，未改任何代码/配置
> 基准（agent 权威，以 `docs/api/PORT-REGISTRY.md` 为端口唯一权威）：
> **四进程 = 网关 mox-server:3080（mox-platform-gateway-svc）/ operator-server:3001（mox-platform-orchestrator-svc）/ alliance-scheduler:3100 / alliance-executor:3200**；
> KG/KB/Cloud/IAM **默认内嵌网关**，可选独立宿主 `MOX_HOST_ROLE=kg|cloud|iam|kb` → **3411/3412/3413/3414**。
> 每条结论落 `文件:行号`。分级：**P0=部署跑不起来/起错进程/端口冲突**；**P1=缺 healthcheck/重启/资源限制/未登记端口**；**P2=文档漂移/遗留/可选形态**。

---

## 一、对照表（部署来源 × 进程/容器 × 端口 × 健康检查 × 与基准一致性）

### A. 进程级 —— `scripts/start-mox-enterprise.ps1` / `stop-mox-enterprise.ps1`

| 进程/容器 | 端口 | 环境变量 | 健康检查 | 重启策略 | 与四进程基准 |
|---|---|---|---|---|---|
| mox-alliance-scheduler | 3100（start L52） | 无 | 仅 TCP 端口探测（L68-74），无 HTTP /health | 无（Start-Process 崩溃不重启） | ✅ 一致 |
| mox-alliance-executor | 3200（start L53） | 无 | 同上 | 无 | ✅ 一致 |
| operator-server | 3001（start L55） | `OUS_ENABLE_MOX_SYSTEM=0`、`OUS_API_TOKEN=$Token`（L54） | 同上 | 无 | ✅ 一致 |
| mox-server（网关） | 3080（start L61） | `MOX_ALLIANCE_SCHEDULER_URL=:3100`、`MOX_ALLIANCE_EXECUTOR_URL=:3200`（L57-60） | 同上 | 无 | ✅ 一致 |
| KG/KB/Cloud/IAM | 不独立起（无 `MOX_HOST_ROLE`） | 内嵌 | — | — | ✅ 内嵌，符合基准 |
| stop 脚本 | 3001,3100,3200,3080（stop L14） | — | — | — | ✅ 与 start 完全对称 |

> 启动后就绪判定：盲等 8s 再 `Get-NetTCPConnection`（start L63-75），**未调用任何 `/health`**，崩溃进程只能从 `.err` 人工发现。

### B. 容器级 —— `docker-compose.yml`（默认 profile）

| service/容器 | 端口（宿主→容器） | 健康检查 | restart | 资源限制 | 与四进程基准 |
|---|---|---|---|---|---|
| nginx `mox-nginx` | 8080→80（L26） | ✅ wget /health（L36-40） | unless-stopped | 无 | ⚠️ 前端入口 8080 未登记（见 N5） |
| api-gateway `mox-server` | 3080→3080（L67） | ✅ curl :3080/health（L73-78） | unless-stopped | **无** | ⚠️ 仅网关 1 进程 |
| llm-inference-svc | 8001→8001（L106） | ✅ urllib /health（L109-114） | unless-stopped | 无 | ⚠️ 8001 为 TEST-ONLY 段（见 N4） |
| ollama（profile） | 11434（L131） | ✅ ollama ps（L141-145） | unless-stopped | 仅 GPU 预留（L134-140） | 第三方，无关 |
| postgres / redis（profile） | 5432 / 6379 | ✅ pg_isready / redis-cli ping | unless-stopped | 无 | 基础设施，无关 |
| prometheus（profile） | 9090（L214） | **❌ 无** | unless-stopped | 无 | 观测，可选 |
| grafana（profile） | **3000**（L233） | **❌ 无** | unless-stopped | 无 | ⚠️ 与 OUS 3000 撞段（见 N6） |
| **operator-server 3001** | **未定义** | — | — | — | ❌ **缺失** |
| **alliance-scheduler 3100** | **未定义** | — | — | — | ❌ **缺失** |
| **alliance-executor 3200** | **未定义** | — | — | — | ❌ **缺失** |

> api-gateway 环境（L57-65）只设 LLM 与 JWT，**没有** `MOX_ALLIANCE_SCHEDULER_URL/EXECUTOR_URL`，也无 operator 上游；容器内网关无法路由 `/api/*`→3001、`/alliance/v1/*`→3100/3200。

### C. 容器级 —— `docker-compose.domains.yml`（fused / split 二选一）

| service/容器 | MOX_HOST_ROLE | 端口 | 健康检查 | restart | 与基准 |
|---|---|---|---|---|---|
| fused（profile=fused） | all（L29） | 3080:3080（L30） | ✅ curl :3080/health（L13-17） | unless-stopped | ✅ 单网关内嵌全部域 |
| entry nginx（profile=split） | — | 3080:3080（L35） | **❌ 无 healthcheck**（仅 depends_on service_healthy，L37-41） | unless-stopped | split 入口，见 N8 |
| kg | kg（L47） | 127.0.0.1:3411（L51） | ✅ curl :3411/health（L49-50） | ✅(继承 x-host) | ✅ 与注册一致 |
| cloud | cloud（L58） | 127.0.0.1:3412（L62） | ✅ curl :3412/health（L60-61） | ✅ | ✅ |
| kb | kb（L69） | 127.0.0.1:**3414**（L73） | ✅ curl :3414/health（L71-72） | ✅ | ✅（kb=3414，未与 iam 换错） |
| iam | iam（L80） | 127.0.0.1:**3413**（L84） | ✅ curl :3413/health（L82-83） | ✅ | ✅ |
| operator 3001 / scheduler 3100 / executor 3200 | — | **未定义** | — | — | ❌ split/fused 均缺这三进程 |

> 路由见 `deploy/nginx/domains.conf`：kg→:3411、cloud→:3412、kb→:3414、iam→:3413（L9-12），与端口绑定一一对应；split 模式下**没有 operator 3001 与联盟 3100/3200 的位置**，nginx 也不转发它们。

### D. 编排级 —— `deploy/k8s/base/mox-platform.yaml`

| 对象 | 端口 | 探针 | 资源 request/limit | 与基准 |
|---|---|---|---|---|
| Deployment `mox-gateway`（L33-104） | containerPort 3080（L69） | liveness + readiness GET /health（L89-100） | ✅ 200m/256Mi → 1/1Gi（L82-88） | ⚠️ 仅网关 1 进程 |
| Service `mox-gateway` | 3080（L119） | — | — | ✅ |
| Ingress `mox-ingress` | →3080（L142） | — | — | ✅ 只到网关，不绕鉴权 |
| operator 3001 / scheduler 3100 / executor 3200 | **未定义** | — | — | ❌ **整组缺失** |

### E. 编排级 —— `deploy/helm/mox/templates/kind-3m3s-deployment.yaml`（默认 `enabled=false`，values.yaml:105）

| 对象 | 端口 | 探针 | 资源 | 备注 |
|---|---|---|---|---|
| Deployment `mox-server` replicas=3（L44-139） | 3080 + metrics **9090**（L98-100） | ✅ liveness/readiness /health:3080（L114-125） | ✅ 1/1Gi → 4/8Gi（L126-132） | args `--single-node`（L95） |
| etcd / minio StatefulSet | 2379/2380、9000/9001 | ✅ | ✅ | 基础设施 |
| operator/alliance | **未定义** | — | — | ❌ 同缺 |

### F. 其它

| 文件 | 事实 | 与基准 |
|---|---|---|
| `deploy/systemd.service` | python `run.py 8600`，`Restart=always`（L9-13） | ✅ 与注册 3.4「legacy 8600」一致；但 AGENTS.md 称 legacy Python 已归档「勿用」，属遗留 P2 |
| `deploy/config/gateway.yaml` | gateway.port=3080（L7） | ✅ |
| `deploy/docker/Dockerfile.rust-service` | HEALTHCHECK 探 `/health`（L33-34），非 root（L24-30） | ✅ |

### G. 端口权威侧 —— `docs/api/PORT-REGISTRY.md`

| 端口 | 登记 | 状态 |
|---|---|---|
| 3080 | 3.1 L49 api 网关 /health | ✅ |
| 3001 | 3.3 L84 operator-server（网关 `/api/*` 转发目标） | ✅ |
| 3100 / 3200 | 3.2 L63-64 scheduler/executor /health | ✅ |
| 3411/3412/3413/3414 | 3.3 L89-92 独立域宿主，`MOX_HOST_ROLE` | ✅ |
| 6.4 L246 | 2026-09-14 已收敛为「网关 3080 + 编排器 3001 + 调度 3100 + 执行 3200」四进程 | ✅ 基准来源 |
| 8001–8003 | 3.6 L121 标记 TEST-ONLY（cloud-master 卷节点测试） | ⚠️ 与 compose llm:8001 冲突（N4） |
| 3000 | 3.3 L83 注明「Grafana 默认同为 3000，部署需避让」 | ⚠️ compose grafana 默认仍用 3000（N6） |
| 8600 | 3.4 L98 legacy Python mox-server | ✅ |

---

## 二、不一致清单（落 `文件:行号`）

| # | 级别 | 不一致事实 | 证据 |
|---|---|---|---|
| N1 | **P0** | 容器/编排来源**整体缺 operator-server:3001 + alliance-scheduler:3100 + alliance-executor:3200**：`docker-compose.yml`、`docker-compose.domains.yml`、`deploy/k8s/base/mox-platform.yaml`、`deploy/helm/...kind-3m3s` 均只起网关 3080。与 6.4 已收敛的「四进程」基准不符。 | compose 全文件无 3001/3100/3200（grep 仅命中 `MOX_HOST_ROLE` 5 处）；k8s 仅 mox-gateway（mox-platform.yaml L33-104） |
| N2 | **P0** | compose 网关环境未注入联盟上游，且容器内无对应容器可指：`MOX_ALLIANCE_SCHEDULER_URL/EXECUTOR_URL` 缺失 → `/alliance/v1/*`、`/api/graph/*` 在容器部署中无上游（脚本版 start L57-61 是显式注入的）。 | docker-compose.yml L57-65；对照 start-mox-enterprise.ps1 L57-61 |
| N3 | **P0** | k8s Ingress/Service 仅到 3080，而 3001 是注册登记的「网关 `/api/*` 通配转发目标」；k8s 版没有 operator 这一跳 → 标准 Ingress 访问的 `/api/*` 无后端。 | mox-platform.yaml L106-142；PORT-REGISTRY L84 |
| N4 | **P1** | `llm-inference-svc` 把宿主 **8001** 发布出去，但注册表把 8001–8003 划为 TEST-ONLY（cloud-master 卷节点测试）。真实运行服务占用未登记运行端口，违反「一端口一服务/测试隔离」。 | docker-compose.yml L92、L106；PORT-REGISTRY L121 |
| N5 | **P1** | compose 头部注释「访问 http://localhost:8080」、nginx 宿主默认 **8080**；而注册 6.4 已把网关从 8080 迁到 3080，8080 不在 RUNTIME 段。前端入口 8080 未登记，且注释沿用旧网关地址。 | docker-compose.yml L7、L26；PORT-REGISTRY L245 |
| N6 | **P1** | grafana 宿主默认 **3000**，注册表 3.3 已显式警告「Grafana 默认同为 3000，部署需避让」（OUS system-core 也绑 3000）。主机已跑 OUS 边缘时即端口冲突。 | docker-compose.yml L233；PORT-REGISTRY L83 |
| N7 | **P1** | `start-mox-enterprise.ps1` 无 HTTP 健康检查、无重启/看门狗：盲等 8s + TCP 监听探测，进程崩溃不自动拉起（对比容器侧全部 `restart: unless-stopped` + healthcheck）。 | start-mox-enterprise.ps1 L46、L63-75 |
| N8 | **P1** | compose 业务容器**无 CPU/内存资源限制**（仅 ollama 有 GPU 预留）；prometheus、grafana 无 healthcheck；domains split 的 entry nginx 无 healthcheck。k8s 侧反而齐全。 | docker-compose.yml L45-237（无 resources）、L203-237；domains.yml entry L32-41 |
| N9 | **P2** | `deploy/systemd.service` 仍指向 legacy Python `run.py 8600`；注册正确标为 LEGACY，但与 AGENTS.md「legacy 已归档勿用」并存，易被当作现行部署。 | systemd.service L9-13；PORT-REGISTRY L98；AGENTS.md |
| N10 | **P2** | helm kind-3m3s 暴露 metrics 9090（注册把 9090 列为 Prometheus 第三方默认），且 `--single-node`、3 副本；该 chart 默认关闭，属可选/实验形态，不影响默认部署。 | kind-3m3s-deployment.yaml L95-100、L155-157；PORT-REGISTRY L158 |

---

## 三、分级结论

- **P0（起错进程 / 部署拓扑与基准不符，容器化与 k8s 路径跑不出四进程）**
  - N1：compose / k8s / helm 全缺 operator:3001 + alliance-scheduler:3100 + alliance-executor:3200。
  - N2：compose 网关未注入 `MOX_ALLIANCE_SCHEDULER_URL/EXECUTOR_URL`，无联盟上游。
  - N3：k8s Ingress/Service 只到 3080，缺少到 operator:3001 的 `/api/*` 一跳。
  - 说明：脚本路径（start/stop ps1）与注册表**完全自洽**，问题集中在「容器/编排部署来源没有跟上 2026-09-14 的四进程收敛」。建议二选一：(a) 在 compose/k8s 补三个 service/Deployment 并注入上游；或 (b) 在这些来源头部明确标注「gateway-only 精简拓扑，非企业四进程形态」，避免被误当生产基准。

- **P1（缺健康检查 / 重启 / 资源限制 / 端口未登记）**
  - N4：llm-inference 占用 TEST-ONLY 段 8001，需登记或换段。
  - N5：compose 注释/nginx 仍用已迁移的 8080。
  - N6：grafana 默认 3000 与 OUS 3000 撞段。
  - N7：企业启动脚本无 HTTP 探活、无重启看门狗。
  - N8：compose 无资源限制；prometheus/grafana/domains-entry 缺 healthcheck。

- **P2（文档漂移 / 遗留 / 可选形态）**
  - N9：systemd 指向 legacy Python 8600（需标注 legacy-only）。
  - N10：kind-3m3s metrics 9090、`--single-node`、默认关闭的实验 chart。

### 已一致、无需改动项
- start/stop 脚本四进程端口 3080/3001/3100/3200 与 PORT-REGISTRY 3.1/3.2/3.3 一致，stop 与 start 端口对称。
- domains split 的 kg/cloud/kb/iam = 3411/3412/3414/3413 与注册 L89-92 完全对应（kb=3414、iam=3413 未错位），且仅绑 127.0.0.1。
- k8s base / Dockerfile / domains x-host 的 `/health` 探针、`restart: unless-stopped`、非 root 运行、PVC RWO、Ingress 只到网关，均符合注册 6.4 与容器最佳实践。
- 默认链路未把 KG/KB/Cloud/IAM 重复起为独立进程；split 形态由 profile 显式门控，不构成「重复起独立域进程」。

> 复跑校验建议（只读，不改文件）：`python scripts/verify-ports.py --json`。
