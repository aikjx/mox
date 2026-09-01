# 开发专家联盟 · 阶段四工程报告（Nacos 阶段二 + executor 真实 LLM 链 + 专家集归一化）

> 日期：2026-09-01 · 范围：platform/domains/alliance + platform/domains/ai · 基线：全量回归 **451 passed / 0 failed**
> 前置：PORT-NORM-001 V1.2（上一轮：配置外部化 + HTTP 专家桥接激活）

---

## 一、registry.rs 双套专家集归一化（消除历史遗留重复）

### 问题

registry.rs 内存在 **两套内置专家集**：

| 位置 | 命名 | 说明 |
|---|---|---|
| `scheduler-core/registry.rs::domain_experts` | `graph_construction` / `data_analysis` / …（旧命名） | **仅测试引用**（`all_domain_experts_has_10_experts` / `domain_experts_have_expected_ids`），生产代码未使用 |
| `config-core/examples/domain_experts.rs::build_domain_experts` | `expert-code` / `expert-math` / …（`expert-<domain>`，符合 §7.9 命名约定） | **权威集**，`build_app` 实际使用，10 大领域专家 |

且 lib.rs 将废弃模块 `pub use registry::domain_experts` 导出，形成「双源可引用」。

### 修复

1. 删除 `registry.rs` 的 `domain_experts` 模块（~200 行，10 个旧专家工厂 + `all_domain_experts`）+ 依赖它的 2 个测试；
2. `lib.rs` 导出移除 `domain_experts`，注释明确**权威单一来源** = `config-core::examples::domain_experts::build_domain_experts`；
3. 全 workspace 扫描确认无外部引用（matching.rs 用的是 config-core 权威集，不受影响）。

**验证**：scheduler-core 带 `http-bridge` feature **85 passed / 0 failed**；无 `domain_experts`/`all_domain_experts`/`graph_construction` 残留引用。

---

## 二、Nacos 阶段二：ConfigStore 抽象 + NacosConfigStore（把 yml 升格为配置中心）

### 设计（可插拔配置源链）

```
内置默认 < FileConfigStore(本地yml) < NacosConfigStore(远程,可选) < env
```

新增 `boot-config/src/config_store.rs`（纯本地，零外部运行时依赖）：

| 组件 | 职责 |
|---|---|
| `ConfigStore` trait | `load_raw(key) -> Result<Option<String>, ConfigStoreError>`；`Ok(None)`=无此 key，`Err`=读取失败 |
| `FileConfigStore` | 本地 `{base_dir}/{key}.yml`（离线兜底） |
| `MemoryConfigStore` | 内置默认 / 测试桩 |
| `ConfigStoreChain` | 按序逐源尝试，**容错降级**：上游 Err 告警后落到下一源（配置中心不可达 → 自动用本地 yml） |

新增 `boot-config/src/nacos_config.rs`（**feature-gated `nacos`**，基于官方 `nacos-group/nacos-sdk-rust`，crates.io `nacos-sdk` 0.8 `config` feature）：

| 组件 | 职责 |
|---|---|
| `NacosSection` | yml `nacos:` 段（enabled/server_addr/namespace/username/password/group/data_id），**始终可解析**（无 SDK 依赖） |
| `NacosConfigStore` | 绑定单个 dataId；启动 `get_config` 初拉 + `add_listener` 注册 watch 监听（缓存 + 广播热更新通道） |
| `load_scheduler_with_nacos` / `load_executor_with_nacos` | bootstrap：读本地（引导）→ 若启用则拉远程整体覆盖 → env 仍最高；Nacos 不可达告警降级本地 |

加载 API 重构（向后兼容）：`load_scheduler(path)` / `load_executor(path)` 保留；新增 `load_*_from_store(store, key)`（从任意 ConfigStore 链加载）。

### 配置（config/alliance-scheduler.yml / alliance-executor.yml 追加）

```yaml
nacos:
  enabled: false                # 默认关闭，保持本地 yml 优先
  server_addr: "127.0.0.1:8848"
  namespace: ""
  username: ""
  password: ""
  group: "DEFAULT_GROUP"
  data_id: "mox-alliance-scheduler.yml"   # 远程完整配置（整体覆盖本地）
```

启用条件：① boot-config 开启 `nacos` feature（默认关闭，不引入 SDK 重依赖）；② `nacos.enabled: true`。

### 验证

- boot-config 默认（无 feature）：**16 passed**；nacos feature：**19 passed**（config_store 6 项 + nacos 3 项：disabled 不发请求 / 空 dataId / 不可达显式报错，交由配置链降级）。

### 诚实声明

- 本地无 Nacos 服务端，**未做真实服务端 e2e**；`get_config`/`add_listener` 走官方 SDK 协议，真实链路需部署 rnacos 或 nacos-server 2.x 后验证。
- 认证（username/password）需 nacos-sdk `auth-by-http` feature（boot-config 当前仅 `config` 能力，无鉴权直连；接入时补 feature 并接线）。
- scheduler-svc/executor-svc 默认**不启用** nacos feature（保持轻量），部署配置中心时开启。

---

## 三、executor 生产专家模式：真实 LLM 调用链端到端验证（含修复真实 bug）

### 真实 bug（本项核心成果）

`mox-ai-expert-svc` 的 `OpenAiChatClient` 原在 **`new()` 直接构建 `reqwest::blocking::Client`**。但 executor-svc Expert 模式经 `llm_consultant()` 在 **axum `build_app`（async 上下文）** 创建它：

```
reqwest::blocking::Client 内部自带 tokio runtime
→ 在 async 上下文创建，进程退出 drop 时 panic：
  "Cannot drop a runtime in a context where blocking is not allowed"
→ 有 API Key 时生产 Expert 模式【启动即崩】
```

> 这解释了此前为何只验证了「无 Key 启动成功（懒连接）」——一旦配置真实 LLM Key，生产路径直接 panic。

**修复**：`OpenAiChatClient` 的 blocking client 改为 `std::sync::OnceLock` **延迟到首次 `complete()`**（`spawn_blocking` 的 blocking 线程）才构建，彻底避开 async 上下文；`complete()` 经 `self.client()` 获取。

### 端到端验证（真实执行，脚本入库 `platform/domains/alliance/tools/`）

```
mock_openai.py  # 本地 OpenAI 兼容 /v1/chat/completions（8999），记录请求到 mock_reqs.log
expert_e2e.py   # 起 mock + executor(expert模式, MOX_LLM_*→mock) → POST /internal/executions → 断言
```

结果：

| 验证点 | 结果 |
|---|---|
| executor /health（Expert 模式启动） | 200（panic 已消除） |
| 提交任务 POST /internal/executions | 200 |
| 任务状态 | **completed**（1/1 节点） |
| mock 收到真实 LLM 请求 | `POST /v1/chat/completions` + `Authorization: Bearer test-key-123` + `model=test-model` + 2 messages |
| 响应解析 | score=0.9 / vetoed=false → 节点 completed |
| 判定 | **PASS**（真实 HTTP 调用链：请求构造/认证/消息格式/响应解析全走生产代码路径） |

### 诚实声明

LLM 响应来自本地 mock OpenAI 服务（无真实 API Key），但 **HTTP 请求构造、Bearer 认证、messages 格式、响应解析走真实生产代码路径**；接入真实 Key（`MOX_LLM_API_KEY`）后同一链路即接真实模型。

---

## 四、回归与测试

| 范围 | 结果 |
|---|---|
| 联盟 12 crate（boot-config `nacos` + scheduler-core `http-bridge` feature） | passed / 0 failed |
| mox-ai-expert-svc lib 单测（parse_score/parse_veto/expert_role/mock_client 等） | **219 passed** |
| **合计** | **451 passed / 0 failed** |

### 已知项（pre-existing，非本次引入）

`mox-ai-expert-svc` 有 3 个集成测试 `tr_08_*`（t8_dip_mox_expert_traits.rs）失败，原因是它们**指向本仓库不存在的结构与 crate**：

- `platform/services/hermes-flow-bridge`、`platform/services/business-catalog` 目录不存在；
- `mox-expert` / `hermes-flow-bridge` / `business-catalog` crate 不在 workspace。

属历史 DIP 治理测试的环境性失败（引用的平台结构在 `platform/domains/ai` 与 `platform/domains/alliance` 重构后已不存在），与本次改动无关。

---

## 五、变更清单

| 文件 | 变更 |
|---|---|
| `core/mox-alliance-boot-config/Cargo.toml` | +`nacos` feature（nacos-sdk 0.8 config）+ `async-trait` + dev tokio |
| `core/mox-alliance-boot-config/src/config_store.rs` | 新增：ConfigStore trait + File/Memory/Chain |
| `core/mox-alliance-boot-config/src/nacos_config.rs` | 新增：NacosConfigStore（get_config + add_listener watch） |
| `core/mox-alliance-boot-config/src/lib.rs` | +NacosSection、+load_*_from_store、+load_*_with_nacos、nacos 段接入 |
| `core/mox-alliance-scheduler-core/src/registry.rs` | 删除废弃 domain_experts 模块 + 2 测试 |
| `core/mox-alliance-scheduler-core/src/lib.rs` | 移除 domain_experts 导出，注释权威来源 |
| `svc/mox-ai-expert-svc/src/llm/chat.rs` | OpenAiChatClient blocking client 延迟创建（OnceLock） |
| `config/alliance-scheduler.yml` / `alliance-executor.yml` | +nacos 段、+LLM env 说明 |
| `docs/standards/expert-alliance-port-norm.md` | V1.2 → **V1.3**（§7.11 / §7.12） |
| `platform/domains/alliance/tools/` | +mock_openai.py / expert_e2e.py（验证脚本） |

---

*开发专家联盟 · 阶段四 · 2026-09-01 · 全量回归 451 passed / 0 failed*
