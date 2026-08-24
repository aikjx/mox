# Xuanji 3.0.0 运维手册 (Operations Manual)

**版本：** 3.0.0
**发布日期：** 2026-08-24
**适用范围：** Xuanji Graph Platform（核心 + DR + Observability 伞图 Helm Chart）
**维护者：** SRE & Xuanji Platform Team

---

## 1. 架构概述

Xuanji 采用云原生多区域部署架构，基于 Helm 伞图（`deploy/helm/xuanji`）一键拉起三大子系统：

| 子系统 | Chart | 职责 |
|---|---|---|
| xuanji-core-local | xuanji-core 3.0.0 | nGQL/openCypher 解析、查询优化、7 算法内嵌执行、存储引擎 |
| xuanji-dr | xuanji-dr 3.0.0 | 双活多区域灾备（cn-north-1 主 / cn-south-1 备）、Raft 共识 |
| xuanji-observability | xuanji-observability 3.0.0 | Prometheus + Grafana + OTLP Trace + 8 阶段 Trace 看板 |

运行时组件最小 6 节点 HA 拓扑（3 主 3 从跨 AZ），详见 [ha-capacity-tco.md](./ha-capacity-tco.md)。

---

## 2. 部署与升级

### 2.1 首次部署（一键伞图）

```bash
helm dependency build deploy/helm/xuanji
helm install xuanji deploy/helm/xuanji \
    --namespace xuanji-system --create-namespace \
    --values custom-values.yaml
```

### 2.2 升级流程（灰度 4 阶段）

1. `helm upgrade xuanji ... --set global.gray.enabled=true --set canary.weight=1`
2. 运行 `scripts/Gray-Warmup.ps1` 自动推进 1→10→50→100 并执行健康检查。
3. 任一阶段 <95% 健康，脚本写入 `rollback.log` 并退出 1，需执行 `helm rollback xuanji <REV>`。

### 2.3 补丁发布（非灰度紧急修复）

```bash
helm upgrade xuanji deploy/helm/xuanji -n xuanji-system \
    --set global.gray.enabled=false \
    --set image.tag=3.0.0-p1 \
    --wait --timeout 10m
```

---

## 3. 容量规划与扩缩容

详见 [ha-capacity-tco.md](./ha-capacity-tco.md) 的容量规划段落。快捷命令：

```bash
# 扩 core replicas 3 -> 6
kubectl scale deploy xuanji-core-local -n xuanji-system --replicas=6

# HPA 自动扩缩容阈值调整
helm upgrade xuanji deploy/helm/xuanji -n xuanji-system \
    --set xuanji-dr.autoscaling.targetCPUUtilizationPercentage=70
```

---

## 4. 日常监控与告警

默认 Grafana Dashboard：
- `Xuanji/Overview` — QPS、P99 延迟、错误率、CPU/内存。
- `Xuanji/8-Stage-Trace` — 每个 trace stage 的 p50/p95/p99、error_rate、saturation、span_count。
- `Xuanji/DR-Replication` — 主/备 RPO、Raft commit lag、跨区域带宽。

**Prometheus Rule 关键指标：**
- `xuanji_query_p99 > 2000ms` → P1 告警（page oncall）。
- `xuanji_dr_rpo_seconds > 30` → P1 灾备延迟异常。
- `xuanji_stage_error_rate{CircuitBreaker="1"} > 0.01` → P2 熔断触发。

---

## 5. 备份与恢复

### 5.1 每日全量快照

```bash
kubectl create job xuanji-snapshot-$(date +%Y%m%d) \
    --from=cronjob/xuanji-daily-snapshot -n xuanji-system
```

### 5.2 恢复操作

```bash
# 1. 停止写入
kubectl scale deploy xuanji-core-local -n xuanji-system --replicas=0
# 2. 从快照恢复 PVC
velero restore create --from-backup xuanji-backup-YYYYMMDD
# 3. 重建索引并启动
kubectl scale deploy xuanji-core-local -n xuanji-system --replicas=3
```

### 5.3 RPO/RTO 目标

| SLA 级别 | RPO | RTO |
|---|---|---|
| Production Gold | <= 30s | <= 5min |
| Production Silver | <= 5min | <= 30min |
| Non-Prod | <= 1d | <= 4h |

---

## 6. 灾备切换 (DR Failover)

### 6.1 计划内切换演练

```bash
# 1. 将主区域副本归零（模拟中断）
kubectl scale deploy xuanji-xuanji-dr-primary -n xuanji-system --replicas=0
# 2. 等待 failoverTimeoutSeconds（默认 120s）
# 3. 观察 secondary 接管写入流量
kubectl get svc xuanji-xuanji-dr-secondary -n xuanji-system -o wide
```

### 6.2 真实故障切换

```bash
# 启动 DNS 级切换（CNAME → secondary ingress）
./scripts/dns-failover.sh --promote-secondary --region cn-south-1
# 同步更新应用连接串为 secondary endpoint
```

### 6.3 故障回切 (Failback)

1. 修复 primary 区域。
2. `helm upgrade xuanji ... --set xuanji-dr.enabled=true` 恢复双副本。
3. Raft 数据追平后，将 CNAME 指回 primary。

---

## 7. 安全与合规

- 镜像签名：所有镜像通过 Cosign 签名，准入 Webhook 拒绝未签名镜像。
- 传输加密：服务间 mTLS（Istio / Linkerd），对外 TLS 1.3。
- 存储加密：PVC 默认启用 StorageClass 级 LUKS + 国密 SM4。
- RBAC：SRE 管理员 vs 只读审计两个 ClusterRole 绑定。
- 合规：信创组合矩阵按 [xinchuang-matrix.md](./xinchuang-matrix.md) 验证。

---

## 8. 故障排查

| 现象 | 排查步骤 |
|---|---|
| 查询超时 P99 飙升 | ① 查看 `Xuanji/8-Stage-Trace` 哪个 stage 慢；② `kubectl top pod -n xuanji-system` 看热点；③ 调大 HPA maxReplicas |
| DR RPO 不达标 | ① 检查跨区域带宽（`iftop`）；② `kubectl logs deploy/xuanji-xuanji-dr-primary -c xuanji \| grep REPL_LAG`；③ 调整 async batch 大小 |
| 熔断 stage=CircuitBreaker 打开 | ① 查看 `trace_8stages` 的 error_rate；② 暂停非核心写入任务；③ `kubectl rollout restart deploy xuanji-core-local` |
| HPA 不扩缩容 | ① `kubectl describe hpa xuanji-xuanji-dr`；② 检查 metrics-server 是否采集到 CPU；③ 查看 HPA behavior.stabilizationWindowSeconds |

---

## 9. 性能调优

| 维度 | 默认值 | 推荐（100M 顶点 / 500M 边）|
|---|---|---|
| Java heap（若使用 Spark 桥）| 4G | 16G |
| Rust graph-service 内存 | 4G | 32G (内存(GB)=32) |
| raft_log_size_mb | 512 | 2048 |
| write_batch_size | 1024 | 8192 |
| bloom_filter_bits_per_key | 10 | 14 |

---

## 10. 配置管理

所有配置集中在 `deploy/helm/xuanji/values.yaml`，敏感字段使用 Helm Secrets + SOPS：

```bash
# 加密 secrets.yaml
sops -e -i secrets.yaml
# 部署时解密注入
helm secrets upgrade xuanji deploy/helm/xuanji -n xuanji-system \
    -f values.yaml -f secrets.yaml
```

---

## 11. 发布流程 (CI/CD)

1. PR 合入 `release/3.0.x` → 触发 CI 构建 + 全量 60 条 nGQL 单测 + 信创烟雾。
2. 镜像推送到 `registry.infotopograph.io/xuanji/graph-server:3.0.0-<SHA>`。
3. ArgoCD Image Updater 同步到 staging → 自动 Gray-Warmup.ps1。
4. staging 通过后，手动 promote 到 prod（需要 2 人审批 Change Request）。

---

## 12. 版本与兼容性

- 向后兼容：3.x 版本内 Helm release 可滚动升级，不支持 2.x → 3.x 直接升级（需走迁移工具）。
- K8s 最低版本：1.24（用到 autoscaling/v2、policy/v1 PDB、Container files）。
- 信创兼容：参考 [xinchuang-matrix.md](./xinchuang-matrix.md)，fully 组合支持生产 SLA。
- 已知不兼容：`values.yaml` 在 3.0.0 中 `dr.mode` 字段由 boolean 改为 enum。

---

## 13. FAQ

**Q1: Xuanji 支持哪些图查询语言？**
A: 内置 60 条 nGQL + 20 条 openCypher 覆盖；两种 parser 纯自研零第三方图 DB 依赖。

**Q2: 怎么开启 8 阶段 Trace 埋点？**
A: 3.0.0 默认开启；Rust crate `xuanji-graph-service` 暴露 `trace_8stages` 模块，配合 `deploy/docs/trace-8stages-dashboard.json` 导入 Grafana 即可。

**Q3: 灰度脚本遇到 `exit 1 rollback.log 已生成` 怎么处理？**
A: 先打开 rollback.log 确认触发阶段与分数；若为误报，重跑 `Gray-Warmup.ps1 -WarmupSeconds 1`；若为真实异常，执行 `helm rollback` 并提交 Incident。

**Q4: 如何快速验证信创矩阵组合？**
A: 走 `deploy/docs/xinchuang-matrix.md` 最后 5 条 smoke 命令，最后一条会做 36 单元格计数校验。

**Q5: 最少 6 节点 HA 是否必须跨 AZ？**
A: 是，SLA Gold 要求 3 主跨 AZ-A 与 AZ-B（2+1），3 从对称分布，详见 ha-capacity-tco.md 拓扑图。

**Q6: helm dependency build 报 xuanji-core 目录不存在？**
A: 伞图使用 `file://../xuanji-core` 作为本地 alias；CI 构建时会自动 `git clone xuanji-core` 到 `deploy/helm/` 下；本地开发可自行克隆或临时 `--set xuanji-core-local.enabled=false`。

**Q7: TCO 3 年成本如何估算？**
A: 见 [ha-capacity-tco.md](./ha-capacity-tco.md) 2027 / 2028 / 2029 分项合计与总计。
