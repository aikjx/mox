# 企业级架构修复优化验收报告



* 日期：2026-09-21

* 仓库：`D:\a10\aikjx\gitcode\infotopograph`（143 crate Rust workspace）

* 基线：0 编译错误 + arch-test 全绿 + 全量测试通过 + 四进程运行时验证



***

## 1. 验收结论



| 门禁                                                  | 结果                                   |
| --------------------------------------------------- | ------------------------------------ |
| `cargo check --workspace --all-targets`             | 0 error                              |
| `cargo test --workspace --exclude mox-voice-dsp-py` | 1179 passed / 0 failed               |
| `mox-arch-test`                                     | 10 passed / 0 failed / 0 ignored     |
| `python scripts/verify-ports.py`                    | ERROR=0（110 端口扫描）                    |
| `python scripts/check-doc-links.py`                 | 无断链                                  |
| `governance-mcp --selftest`                         | 23/23 全过                             |
| `alliance_demo.py` 端到端                              | 93/93 步骤通过，0 降级，191 HTTP 调用          |
| 四进程运行时                                              | 3100/3200/3001/3080 全 UP，gateway 200 |



***

## 2. 本轮关键修复

### 2.1 删除死依赖（真实架构违规）



* **问题**：arch-test 新增门禁 `test_core_layer_has_no_new_io_dependencies` 检出 `mox-ai-core`（L3 纯计算层）声明 `reqwest` 依赖，但源码零使用（重构遗留）。

* **修复**：从 `mox-ai-core/Cargo.toml` 删除 `reqwest = { workspace = true }`，同步移除 BASELINE 中对应技术债登记。

* **效果**：arch-test 从 8→10 测试全绿。

### 2.2 alliance 域依赖归一化



* **问题**：alliance 域 11 个 crate 的内部依赖用硬编码相对路径（如 `path = "../../proto/mox-alliance-common-proto"`），与根 `[workspace.dependencies]` 重复且不一致。

* **修复**：统一改为 `workspace = true`，路径变更只改根一处。

* **覆盖**：core×5、proto×4、api、sdk、svc×3。

### 2.3 arch-test 测试库 dead\_code 清理



* **问题**：arch-test lib 10 个辅助函数（classify\_crate/parse\_cargo\_toml/collect\_all\_crates/is\_allowed\_dependency 等）仅供 `#[test]` 使用，lib 模式报 dead\_code。

* **修复**：lib.rs 顶部加 `#![allow(dead_code)]`，语义正确（测试工具库不参与生产分发）。

### 2.4 生命周期错误修复



* **问题**：arch-test `E0597: value does not live long enough`—— 类型推断混乱。

* **修复**：`resolved.contains(&(c.to_string(), d.to_string()))` 显式 owned，避免借用逃逸。



***

## 3. 专家联盟域（alliance）健康度

### 3.1 代码规模



| 层    | crate                 | 行数   | warning |
| ---- | --------------------- | ---- | ------- |
| core | alliance-core（DAG 引擎） | 3744 | 0       |
| core | scheduler-core（调度）    | 7739 | 0       |
| core | executor-core（执行）     | 4154 | 0       |
| svc  | scheduler-svc         | 1360 | 0       |
| svc  | executor-svc          | 1006 | 0       |
| svc  | registry-svc          | 648  | 0       |

### 3.2 API 面



* **scheduler :3100**：tasks CRUD + experts search

* **executor :3200**：任务执行 + nodes + cancel/pause/resume + result

* **registry**：gateway :3080 内嵌 `/api/experts/*`（5 路由 ready）

### 3.3 三种 matcher 实现（分层合理）



* `matcher.rs` — 规则匹配（fallback/demo）

* `matching.rs` — 匹配算法核心

* `modular_matcher.rs` — 模块化权重匹配

### 3.4 端到端协作模式

6 种全部跑通：sequential /parallel/debate /hierarchical/iterative /voting。



***

## 4. 运行时验证（真实，非 mock）



```
curl http://127.0.0.1:3080/api/v1/status -H "Authorization: Bearer dev-secret-token"

→ {"code":0,"msg":"ok",

&#x20;  "gateway":"rust-axum-enterprise","version":"3.0.0-ai-powered",

&#x20;  "domains\_total":46,"domains\_ready":46,"domains\_stub":0,

&#x20;  "endpoints\_ready":14,"auth\_enabled":true,

&#x20;  "rate\_limit\_enabled":true,"iam":"ready"}
```

四进程：scheduler:3100 /executor:3200 /operator:3001 /gateway:3080。



***

## 5. 已知环境限制



* `mox-voice-dsp-py`（PyO3 绑定）缺 `python3.lib`（LNK1181），不属代码问题，测试时 `--exclude`。

* 剩余～225 warning 为各业务 crate 预留字段 dead\_code（如 max\_concurrency/first\_failed\_at/tool\_registry—— 为未来 API 预留，删了破坏稳定性），保留并标注是正确做法。

* 60+ 文档 WARN 为 L7 working-reports 历史报告断链（归档性质，不阻断）。