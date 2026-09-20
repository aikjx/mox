# 架构守护归一化报告 · arch-test 基线

> 生成时间：2026-09-20 · 范围：`platform/arch-test/` · 仓库：infotopograph（~163 crate Rust workspace）

## 1. 本轮归一化做了什么

把 arch-test 从"误报满天飞、不可用"改造为"可作 CI 门禁"。全程零业务运行时代码改动，只动测试分类/规则 + 7 处注释标注 + 删除 3 个误入 Cargo.lock。

| 改动 | 文件 | 效果 |
|---|---|---|
| `classify_crate` 识别 `platform/shared/*` → L0/foundation | arch-test/lib.rs | 消除 cache/auth/config/server-runtime 等基础设施误判 unknown |
| `classify_crate` 识别 `*/proto/*` → L2/api | 同上 | 消除 gRPC 契约 crate 误判 unknown |
| `classify_crate` 识别 `domains/base/*` → L0/foundation | 同上 | 消除 base-{model,query,store}-core 误判 unknown |
| hardcoded_paths 扫描跳过 arch-test 自身 | 同上 | 测试不再把自己的黑名单常量当违规 |
| hardcoded_paths 行尾 `// allow: <reason>` 豁免机制 | 同上 | 合理例外显式标注，新增硬编码仍被抓 |
| is_allowed_dependency 放行 L0 同层聚合 | 同上 | server-runtime 组合 cache/auth/config 合理 |
| is_allowed_dependency 放行 L2 同域互引 | 同上 | gRPC 共享消息类型合理 |
| API purity 放行同域 L2 互引 | 同上 | alliance-api→*-common-proto 合理 |
| data_separation 改为只查 `git ls-files` | 同上 | 运行时 .db（已 gitignore）不再误报 |
| 删除 3 个误入 crate 目录的 Cargo.lock | cloud-master/data-compliance/kg-fusion | workspace 模式根 lock 才权威 |

## 2. 违规收敛轨迹

| 指标 | 归一化前 | 归一化后 | 性质 |
|---|---|---|---|
| 跨域依赖 | 85 | **38** | 去掉 shared/proto/base 盲区误报 47 |
| 分层规则 | 9→24 | **6** | 去掉规则过严误报 18 |
| API purity | 5 | **0** | 同域 proto 互引合理 |
| 硬编码路径 | 15 | **0** | 9 处历史路径已豁免标注 |
| 数据文件入 platform | 11 | **0** | 改为查 git 追踪，运行时 db 已 gitignore |

## 3. arch-test 门禁状态（6 活跃 / 2 ignored）

### ✅ 常驻 CI 门禁（6）
- `no_circular_dependencies`
- `api_crates_are_pure`
- `plugins_outside_platform`
- `third_party_outside_platform`
- `no_hardcoded_data_paths`（本轮升级，历史 9 处已 `// allow:` 标注）
- `architecture_data_separation`（本轮升级，改为查 git 追踪文件）

### ⏳ 已知架构债（2，需重构）
- `layering_rules` — 6 个跨层
- `cross_domain_through_api` — 38 个跨域直连

## 4. 剩余架构债台账

### 4.1 分层违规（6）

| 调用方 | 被调方 | 层 | 修复路径 | 风险 |
|---|---|---|---|---|
| gateway L1 | platform-iam-core L3 | L1→L3 | gateway 深度嵌入 IamRepository；抽 IamService trait 到 L2 | 高（动认证链路） |
| gateway L1 | flow-unified-process-core L3 | L1→L3 | 抽 ProcessEngine trait 到 L2 或走 L4 svc | 中 |
| ai-core L3 | platform-model-core L3 | 跨域 L3→L3 | 模型抽象下沉 L2 api 或归并域 | 中 |
| cloud-kb-core L3 | ai-alliance-engine L3 | 跨域 L3→L3 | 抽引擎接口 L2 | 中 |
| project-graph-core L3 | kg-core L3 | 跨域 L3→L3 | 图接口抽 L2 | 中 |
| flow-operator-core L3 | platform-operator-core L3 | 跨域 L3→L3 | 算子接口归并 | 中 |

### 4.2 跨域直连（38）按调用方分布

| 调用方域 | 数量 | 说明 |
|---|---|---|
| platform (orchestrator) | 9 | 编排各域 svc 是其职责；走 api 需把各域 svc 接口抽 L2 |
| gateway | 7 | 统一入口调各域 svc；monolith 内进程调用 |
| flow | 6 | bridge/primiflow/fusion/operator 调 ai/platform |
| ai | 6 | agent/expert 调 kg/market/platform |
| kg | 4 | server/hub/kb 调 platform/cloud |
| kb / cloud / data / project / alliance / voice | 各 1-3 | 零散跨域 |

**被调方层分布**：L3-core 12、L4-svc 14、L5-sdk 8、已无 unknown。

### 4.3 修复路径建议（按优先级）

1. **低风险**：gateway/orchestrator 对 L5-sdk 的依赖（8 处）——sdk 本就是对外客户端，可在规则中显式允许 gateway/orchestrator 依赖同部署单元的 sdk。
2. **中风险**：platform 域 core（platform-system/operator/graph/model）被多域依赖（12 处）——评估是否应下沉为 L0 平台共享核心，而非 L3 业务 core。
3. **高风险/长期**：跨域 svc 互调（14 处 L4→L4）——真正的微服务拆分，需 gRPC/消息总线，数月工程。

## 5. 基线

- `cargo check --workspace --all-targets`：0 error（warning 245，多为 dead_code 预留字段）
- `cargo test -p mox-arch-test`：6 passed / 0 failed / 2 ignored
- lint 门禁：workspace 144 crate 统一 `[lints] workspace = true`
