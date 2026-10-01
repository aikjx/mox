# 部署后一致性校验脚本

把 [09-deployment-templates.md](../09-deployment-templates.md) §六「部署后一致性检查清单（7 项）」落成一键可执行脚本。双版本同一份检查逻辑：

| 文件 | 适用环境 |
|------|----------|
| `verify-deploy.sh` | Linux k8s 生产（`bash` + `curl`） |
| `verify-deploy.ps1` | Windows / 内网试点（PowerShell 5.1+，用系统自带 `curl.exe`） |

脚本**只读**：只发 HTTP GET、读本机可选文件路径；不写状态文件、不重启服务、不改配置。token 永不完整打印（只显示前 4 后 4）。

---

## 一、快速开始

### 1. dev 档（本机联调，无 token）

```bash
# Linux
./verify-deploy.sh

# Windows PowerShell
.\verify-deploy.ps1
```

默认目标 `127.0.0.1:3080/3100/3200/3400`，`dev` 档位。鉴权链路与 SM4 加密会输出 WARN（符合 dev 档预期，不阻塞）。

### 2. production 档（内网试点 / 生产）

```bash
# Linux（在能访问四 svc 的运维机 / Pod 内执行）
export MOX_INTERNAL_TOKEN="$(cat /run/secrets/mox-internal-token)"
./verify-deploy.sh \
  --host 10.0.0.8 \
  --mode production \
  --token "$MOX_INTERNAL_TOKEN" \
  --data-dir /var/lib/mox \
  --audit-log-path /var/log/mox/experts-audit.ndjson
```

```powershell
# Windows（内网试点，目标机 10.0.0.8）
$env:MOX_INTERNAL_TOKEN = "<openssl rand -hex 32 输出>"
.\verify-deploy.ps1 `
  -Host 10.0.0.8 `
  -Mode production `
  -Token $env:MOX_INTERNAL_TOKEN `
  -DataDir D:\mox\data `
  -AuditLogPath D:\mox\logs\experts-audit.ndjson
```

---

## 二、参数表

优先级：**CLI 参数 > 环境变量 > 默认值**。

| 参数（sh / ps1） | 环境变量 | 默认值 | 含义 |
|------------------|----------|--------|------|
| `--host` / `-Host` | — | `127.0.0.1` | 目标主机（四 svc 同主机不同端口；k8s 里填 svc 名或 ingress IP） |
| `--gateway-port` / `-GatewayPort` | — | `3080` | 网关端口 |
| `--scheduler-port` / `-SchedulerPort` | — | `3100` | 联盟调度器端口 |
| `--executor-port` / `-ExecutorPort` | — | `3200` | 联盟执行器端口 |
| `--registry-port` / `-RegistryPort` | — | `3400` | 专家注册中心端口 |
| `--token` / `-Token` | `MOX_INTERNAL_TOKEN` | 空 | 内部共享令牌；production 档必传 |
| `--mode` / `-Mode` | — | `dev` | `dev` 或 `production`；production 下鉴权 401/SM4 信封为硬指标 |
| `--data-dir` / `-DataDir` | — | 空 | scheduler/executor 共享 sqlite 目录；给了才做本机文件检查 |
| `--audit-log-path` / `-AuditLogPath` | — | 空 | 审计 NDJSON 绝对路径；给了才做本机文件检查 |
| `--timeout` / `-Timeout` | — | `5` | 单次 HTTP 超时秒数 |
| `-h, --help` | — | — | 显示帮助 |

> k8s 场景：脚本通常 `kubectl exec` 进 Pod 跑，`--host 127.0.0.1` 即可（Pod 内回环）；`--data-dir` 指向容器内 PVC 挂载路径（如 `/var/lib/mox`）。

---

## 三、检查项含义（与 09 小节映射）

| # | 检查项 | 脚本判定 | 09 文档小节 |
|---|--------|----------|-------------|
| 1 | 四进程 `/health` 探活 | gateway/scheduler/executor 返回 200 且 JSON 字段齐全；registry `/health`=200 且 `/api/registry/health`=200 | §6.1 进程存活 |
| 2 | 内部鉴权链路 | production+token 下：带 token 访问三 svc 非公开路径非 401；不带 token 期望 401。dev 档或未配 token 输出 WARN | §6.4 鉴权链路 / §七.1、§七.6 |
| 3 | SM4 信封协商 | 带 `x-mox-crypto: sm4-gcm+gzip` 头请求 gateway `/health`，响应体应为 `{"crypto":{"alg":"SM4-GCM","nonce":"...","ct":"...","tag":"..."}}`。production 档必 PASS | §6.5 SM4 加密 / §七.2 |
| 4 | sqlite 存储与 WAL | 给了 `--data-dir` 才查：`<dir>/alliance_tasks.db` 存在，且有 `-wal` 副产物；否则 MANUAL | §6.6 存储模式 / §2.3 |
| 5 | HA 选主 | GET scheduler `/leadership`：`ha_enabled=false` → WARN（单副本预期）；`ha_enabled=true` → PASS（本副本是 leader 或 follower 都正常） | §6.3 HA 选主 / §2.3 |
| 6 | 审计 NDJSON | 给了 `--audit-log-path` 才查：文件存在且首行含 `actor` 字段；HMAC 重算恒为 MANUAL | §6.7 审计日志 / §七.5 |
| 7 | 健康语义提示 | 从 gateway `/health` 读 `dependencies.scheduler/executor` 输出探活结果；并 WARN 提示「登记状态 ≠ 探活」 | §七.4 |

---

## 四、退出码

| 退出码 | 含义 |
|--------|------|
| `0` | 全部硬指标 PASS（允许有 WARN / MANUAL） |
| `N`（1..127） | 有 N 项 FAIL；N 为 FAIL 计数，超过 127 截断为 127 |

- WARN 不改变退出码（dev 档预期、HA 未启用、登记状态提示等）。
- MANUAL 不改变退出码（需要人工在容器内执行的命令）。

---

## 五、CI 接入示例

### GitLab CI

```yaml
verify-deploy:
  stage: verify
  image: curlimages/curl:latest
  variables:
    MOX_INTERNAL_TOKEN:         # 从 CI/CD 变量注入（masked）
  script:
    - chmod +x docs/expert-alliance/scripts/verify-deploy.sh
    - docs/expert-alliance/scripts/verify-deploy.sh
        --host "$TARGET_HOST"
        --mode production
        --token "$MOX_INTERNAL_TOKEN"
  allow_failure: false
  only:
    - main
```

### GitHub Actions

```yaml
name: verify-deploy
on:
  workflow_dispatch:
  push:
    branches: [main]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run consistency check
        env:
          MOX_INTERNAL_TOKEN: ${{ secrets.MOX_INTERNAL_TOKEN }}
        run: |
          chmod +x docs/expert-alliance/scripts/verify-deploy.sh
          docs/expert-alliance/scripts/verify-deploy.sh \
            --host 10.0.0.8 \
            --mode production \
            --token "$MOX_INTERNAL_TOKEN"
```

---

## 六、安全说明

- **token 不落地**：脚本只在内存中持有 token，日志输出掩码为 `前4****后4`；不写任何临时文件、不打 `ps` 可见的命令行参数（建议通过环境变量传入，避免 `/proc/<pid>/cmdline` 泄露）。
- **只读校验**：所有 HTTP 调用均为 `GET`，不触发任何写路径；本机文件检查只 `Test-Path` / `head`，不修改。
- **网络可达性**：脚本从执行位置访问四 svc；k8s 下建议在 Pod 内或同 Namespace 内执行，避免 :3100/:3200/:3400 暴露到集群外（09 §七.6）。

---

## 七、MANUAL 项清单（脚本不自动做，需人工）

| 项 | 为什么脚本不做 | 人工操作指引 |
|----|----------------|--------------|
| `PRAGMA user_version` | 代码 grep 零命中，端点不暴露 schema 版本（08 缺口 N3）；脚本不依赖 sqlite3 二进制 | 在 Pod 内：`sqlite3 /var/lib/mox/alliance_tasks.db 'PRAGMA user_version;'` |
| 审计 HMAC 哈希链重算 | 需用 `mox-audit` crate 的 `verify_chain()` 重算每条 HMAC；shell/PS 脚本重写代价高 | 跑代码内校验工具，或参考 `gateway/src/alliance/experts_common.rs` 测试用例 `test_emit_audit_appends_to_chain` |
| WAL 文件存在性（CI 外部跑） | CI runner 看不到 Pod 内 PVC 文件系统 | `kubectl exec <scheduler-pod> -- ls -l /var/lib/mox/alliance_tasks.db*` |
| 审计 NDJSON 文件存在性（CI 外部跑） | 同上 | `kubectl exec <gateway-pod> -- tail -n 5 /var/log/mox/experts-audit.ndjson` |
| 多副本「恰好一个 leader」 | 脚本单点访问 scheduler，只能看到自己副本的角色；无法跨 Pod 仲裁 | 分别 `kubectl exec` 进每个 scheduler Pod 跑 `/leadership`，确认 `is_leader=true` 恰好一个 |
| 响应解密 | 脚本只验信封结构存在性，不拿 `MOX_API_CRYPTO_KEY` 解密 | 用 `mox-api-crypto::open_data` 在测试里 roundtrip（见 `codec.rs` 测试） |

---

## 八、与 09 文档的对应

- 检查项来源：[09-deployment-templates.md §6](../09-deployment-templates.md#六部署后一致性检查清单)
- 端口权威：[docs/api/PORT-REGISTRY.md](../../api/PORT-REGISTRY.md) V1.2
- SM4 信封线格式：`platform/foundation/mox-api-crypto/src/codec.rs`（`{"crypto":{"alg":"SM4-GCM","zip":"gzip","nonce":...,"ct":...,"tag":...}}`）
- 鉴权白名单：`svc/mox-alliance-scheduler-svc/src/routes.rs` `internal_auth_layer`（`/health`、`/metrics`、`/leadership`、`/api/registry/health`）

<a id="lowcode-directory-design"></a>
## 目录架构设计卡：低代码与动态配置（2026-10-01）

> 目标设计接缝；既有正文按原日期/类型解释，未实施能力不标已完成。

| 设计项 | 本目录约定 |
|---|---|
| 输入 | 已选部署档、作用域、服务地址与身份引用 |
| 处理与边界 | 运行只读一致性检查，输出实际环境/版本/缺口；不改变配置语义或伪造回执 |
| 输出 | 部署核验与诊断证据归档reports |
| 维护角色 | 交付工具owner（角色建议，未指派个人） |
| 配置语义 | [统一规范](docs/standards/lowcode-dynamic-configuration.md#model)，本目录不复制覆盖/生命周期规则 |
| 本目录设计 | [详细接缝](docs/standards/lowcode-dynamic-configuration.md#acceptance) |
| 验收 | 类型/依赖/权限/版本/异常/恢复按相关LC-Q条目补证；设计完成与运行验证分开 |

全目录关系见 [目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)。新增架构文档应符合 [文档设计契约](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#document-contract)，各主题拥有自己的事实主源。
