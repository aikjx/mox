# 开发专家联盟 · mox 模块化系统架构完成轮报告（阶段五）

- **文档编号**：EA-PHASE5-REPORT-20260901
- **日期**：2026-09-01
- **范围**：平台 `platform/domains/alliance`（专家联盟 Rust 域）
- **任务**：三项"mox 模块化系统架构完成"——① scheduler/executor 真正接入 Nacos 配置中心启动；② Nacos 阶段三（NamingService 注册中心）探索与落地；③ 语音 3717→30010 端口迁移核验
- **硬约束**：禁止吹牛 / 诚实声明；配置错误显式报错；产物落 docs 可追溯

---

## 1. 任务① scheduler-svc / executor-svc 接入配置中心启动（load_*_with_nacos）

### 1.1 改动

| 文件 | 改动 |
|---|---|
| `svc/mox-alliance-scheduler-svc/Cargo.toml` | `mox-alliance-boot-config` 启用 `features = ["nacos","naming"]` |
| `svc/mox-alliance-executor-svc/Cargo.toml` | 同上 |
| `svc/mox-alliance-scheduler-svc/src/bin/main.rs` | `load_scheduler(&config_file)?` → `load_scheduler_with_nacos(&config_file).await?`；启动后接 NamingRegistry |
| `svc/mox-alliance-executor-svc/src/bin/main.rs` | `load_executor(&config_file)?` → `load_executor_with_nacos(&config_file).await?`；启动后接 NamingRegistry |

语义：本地 yml 为**引导 + 离线兜底**；`yml nacos.enabled: true` 时从 Nacos 拉取远程配置**整体覆盖**本地；env（`MOX_ALLIANCE_*`）仍最高优先级。默认 `enabled=false` 离线可用，配置中心不可达降级本地且告警（不阻断启动）。

### 1.2 真实服务端 e2e（rnacos 0.8.7）

新增脚本 `tools/alliance_nacos_e2e.py`（入库）一次验证配置中心 + 注册中心两条链路：

| 步骤 | 结果 |
|---|---|
| 发布远程配置 `mox-alliance-scheduler-e2e.yml`（`server.port=3155`） | PASS（HTTP 发布成功） |
| 本地引导 yml 声明 `3199` + `nacos.enabled=true`，启动 scheduler-svc | 服务日志：`已从 Nacos 拉取远程配置，整体覆盖本地 yml` |
| 探活远程端口 3155 | **PASS**：实际监听 3155（本地 3199 未被监听）→ 配置中心覆盖生效 |
| Nacos 实例列表查 `mox-alliance-scheduler` | **PASS**：`127.0.0.1:3155`，metadata `protocol=http`/`domain=alliance` 命中 |
| 停止 scheduler | **PASS**：实例自动移除（deregister 生效） |

> 诚实声明：e2e 使用的是**真实 rnacos 服务端**（非 mock），远程配置经 gRPC 真实拉取并覆盖本地引导配置，端口级探活为最硬证据。

---

## 2. 任务② Nacos 阶段三：NamingService 注册中心

### 2.1 实现

新增 `core/mox-alliance-boot-config/src/naming.rs`（feature=`naming`，隐含 `nacos`）：

| 组件 | 说明 |
|---|---|
| `NamingSection` | `enabled / service_name / group / ip / port / weight / metadata[]`；**始终可解析**（不依赖 SDK），默认 `enabled=false` |
| `NamingRegistry` | `connect(&nacos,&naming) -> Result<Option<Self>>`；`register()`；`deregister()` |
| 降级语义 | `nacos.enabled=false`/`naming.enabled=false`/空 service_name/连接失败 → `Ok(None)`（注册中心不可用**不阻断**服务启动，仅告警） |

Cargo feature：`naming = ["nacos"]`；nacos-sdk 启用 `features=["config","naming"]`。

两个 svc `main.rs` 接线：`connect` → `register()` → `server.run()` → `deregister()`（优雅注销，避免僵尸实例）。

yml：`config/alliance-scheduler.yml` / `alliance-executor.yml` 新增 `naming:` 段。

### 2.2 真实服务端 e2e（rnacos 0.8.7）

新增 `core/mox-alliance-boot-config/tests/naming_e2e.rs`（`[[test]] required-features=["naming"]`，`--ignored`）：

- **e2e-1** `register` 真实注册 → HTTP 实例列表查到 `127.0.0.1:3100`，metadata `protocol=http`/`domain=alliance` 命中 → **PASS**
- **e2e-2** `deregister` 真实注销 → 实例从列表移除 → **PASS**

### 2.3 实现要点（真实踩坑）

- nacos-sdk 0.8 的 `NamingService` 是**具体 struct**（非 trait），`NamingServiceBuilder::new(ClientProps).build().await` 直接返回它；不能 `Arc<dyn NamingService>`（编译错误 E0404，已修正为直接持有）。
- `ServiceInstance` 字段含 `ip:String / port:i32 / weight:f64 / healthy / enabled / ephemeral / cluster_name / service_name / metadata:HashMap`。
- 注册走 gRPC（9848），查询走 HTTP 兼容 API（8848 `/nacos/v1/ns/instance/list`）。

---

## 3. 任务③ 语音 3717→30010 迁移核验（如实结论）

**结论：迁移实际早已完成，本轮为核验非再迁移。**

| 证据 | 结果 |
|---|---|
| PORT-REGISTRY.md §6.2 | 声明「已完结（2026-09-01）…已完成并通过 verify-ports.py」 |
| 全库扫描 3717 | 仅剩 5 个模型二进制词表文件（`asr-paraformer-streaming/tokens.txt`、`tts-kokoro/dict/idf.utf8`、`pos_dict/*`、`model.onnx`）——**非端口** |
| 全库扫描 30010 | 遍布 17 个活动文件（`mox-voice-desktop-app/src/main.rs` 11 处、`voice_server.rs` 4 处绑定 `127.0.0.1:30010`、`verify_tts_rust_fullstack.py` 7 处等） |
| `scripts/verify-ports.py` 实测 | **ERROR=0**；30010 登记 RUNTIME（引用 14 处） |

### 3.1 verify-ports.py 本轮同步维护

原脚本 CANONICAL 为硬编码（与 PORT-REGISTRY.md 同步维护），本轮新增/修正：

| 端口 | 分类 | 说明 |
|---|---|---|
| 8999 | TEST | alliance executor expert e2e：mock OpenAI 兼容服务（`tools/mock_openai.py`） |
| 3155 / 3199 | TEST | alliance Nacos 配置中心 e2e 远程/本地引导端口（`tools/alliance_nacos_e2e.py`） |
| 9848 / 10848 | THIRD | rnacos gRPC / 独立控制台端口 |

修复 2 处误报/噪声：
- **1578 误报**：`expert-alliance-enterprise-standard.html` 中 `ExpertWorkspaceView.vue:1578` 是**代码行号**非端口 → NOISE_CTX 增加「常见源码扩展名 + `:行号`」过滤模式；
- **rnacos 数据噪声**：`tools/rnacos/nacos_db/` 为运行时数据（SQLite log/index），文本扫描会把内部字节当端口 → PRUNE_DIRS 增补 `nacos_db`。

**复跑结果：`ERROR=0 WARN=0 INFO=92，结论 ✔ 通过（无 ERROR）`**。

---

## 4. 全量回归

```
cargo test -p mox-alliance-boot-config --features nacos,naming \
  -p mox-alliance-common-proto -p mox-alliance-scheduler-proto -p mox-alliance-executor-proto \
  -p mox-alliance-core -p mox-alliance-scheduler-core --features http-bridge \
  -p mox-alliance-executor-core -p mox-alliance-config-core \
  -p mox-alliance-scheduler-svc -p mox-alliance-executor-svc -p mox-alliance-sdk -p mox-alliance-api \
  + cargo test -p mox-ai-expert-svc
```

**结果：550 passed / 0 failed**（上一轮 516 → 本轮 +34，新增 boot-config `naming` 模块单测 5 项 + `nacos,naming` feature 下全量 boot-config 测试）。

> 说明：`naming_e2e` / `nacos_e2e`（各 2 项）为 `--ignored` 真实服务端测试，需本机 rnacos，不进入常规回归。

---

## 5. 变更清单

| 类别 | 文件 | 动作 |
|---|---|---|
| 代码 | `core/mox-alliance-boot-config/src/naming.rs` | **新增**：NamingSection + NamingRegistry（feature=naming） |
| 代码 | `core/mox-alliance-boot-config/src/lib.rs` | 接入 naming 模块 + 两个 BootConfig 增 `naming` 字段 |
| 配置 | `core/mox-alliance-boot-config/Cargo.toml` | nacos-sdk features 加 `naming`；新增 `naming=["nacos"]` feature；增 `[[test]] naming_e2e` |
| 测试 | `core/mox-alliance-boot-config/tests/naming_e2e.rs` | **新增**：真实 rnacos 注册/注销 e2e |
| 代码 | `svc/mox-alliance-scheduler-svc/src/bin/main.rs` | 接入 `load_scheduler_with_nacos` + NamingRegistry |
| 代码 | `svc/mox-alliance-executor-svc/src/bin/main.rs` | 接入 `load_executor_with_nacos` + NamingRegistry |
| 配置 | `svc/...scheduler-svc/Cargo.toml`、`svc/...executor-svc/Cargo.toml` | boot-config 启用 `nacos,naming` feature |
| 配置 | `config/alliance-scheduler.yml`、`config/alliance-executor.yml` | 新增 `naming:` 段 |
| 脚本 | `tools/alliance_nacos_e2e.py` | **新增**：配置中心+注册中心一次验证 |
| 脚本 | `scripts/verify-ports.py` | CANONICAL 增 5 端口；NOISE_CTX 行号过滤；PRUNE nacos_db |
| 文档 | `docs/ports/PORT-REGISTRY.md` | §3.6 登记 8999/9848/10848 |
| 文档 | `docs/standards/expert-alliance-port-norm.md` | **V1.3 → V1.4**：§7.11 补配置中心启动 e2e、§7.13 NamingService 阶段三、§7.14 语音核验 |

---

## 6. 遗留与诚实声明

- **真实 AI 专家服务（3300）未部署**：`expert_service.enabled` 默认 false；本轮未改变该状态（非本轮范围）。
- **Naming 生产生效条件**：需 `yml nacos.enabled=true` + `naming.enabled=true` 同时开启；默认离线可用。
- **LLM 真实 Key 未接入**：阶段四已验证 HTTP 链路走真实生产代码路径（mock OpenAI 服务），接真实 Key 即生效，非本轮范围。
- **alliance 3100/3200/3300 尚未登记进 `platform_config.json`**（PORT-REGISTRY §6.3 遗留待办）：verify-ports 的 `EXPECTED_PLATFORM_PORTS` 仅含 RUNTIME 服务（api/frontend/xiaobai_voice/melody2score/primiflow），alliance 属 ALLIANCE 分类不强制登记 platform_config；如需统一运维再补。

---

*开发专家联盟 · 阶段五mox 模块化系统架构完成轮 · 2026-09-01*
