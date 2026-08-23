# T4 依赖治理 - 实施计划

## 验收标准覆盖映射

| 验收标准 | 对应任务 |
|----------|----------|
| AC-1 TR 4.1 ≤1 | Task 2、Task 4 |
| AC-2 TR 4.2 reqwest 全 0.12 | Task 2、Task 4 |
| AC-3 criterion 统一 workspace=true | Task 2 |
| AC-4 [package] 继承 workspace.package | Task 2 |
| AC-5 cargo check/build exit 0 | Task 5 |
| AC-6 例外 ≤1 并文档化 | Task 3、Task 6 |
| AC-7 依赖治理一致性 | Task 2、Task 4 |

---

## Task 1: TDD RED - 合规测试脚本创建与执行

**优先级**: high  
**状态**: pending  
**依赖**: 无  

### 工作内容

1. 在仓库根目录创建 `test-tr-4-compliance.js` Node 脚本：
   - 扫描 17 个 Cargo.toml 文件路径（16 个 platform/services/* + 1 个 platform/gateway/runtime）；
   - 使用 `toml` 或正则解析（正则更轻，Node 内置足够）每个 Cargo.toml 的 `[dependencies]` / `[dev-dependencies]` 段；
   - **TR 4.1 统计**：对 `[dependencies]` / `[dev-dependencies]` 下的每一个"外部 crate"依赖声明（排除 path 指向内部 crate：xuanji-common-meta / operator-core / operator-wasm / graph-algorithms / optimizer / ai-agent / business-catalog / xuanji-expert / flow-ai / xuanji-system / primiflow-core / primiflow-fusion / kg-hub / hermes-flow-bridge / template-market），检查是否包含 `workspace = true` / `.workspace = true`。如果没有，则计入 TR 4.1 计数；
   - **TR 4.2 检查**：调用 `cargo tree -p primiflow-core -i reqwest --prefix=depth`，用正则提取出现的 reqwest 版本号前缀，全部应为 `0.12.`；
   - **附加检查（辅助）**：
     - criterion 声明中是否仍含 `default-features`（应无）；
     - `[package]` 段中 version/edition/license/authors 是否使用 `workspace = true`（description 可例外）；
   - 输出 JSON 汇总 + 人类可读摘要。

2. 运行脚本并保存 RED 阶段输出（预期 TR 4.1 > 1，TR 4.2 至少存在 0.11 漂移）。

### 任务本地测试要求（TR）

- **TR-T1-1（rule）**：脚本存在 `test-tr-4-compliance.js`，且运行 exit 非 0（RED 状态下 TR 4.1 > 1 或 TR 4.2 失败）。
- **TR-T1-2（rule）**：脚本 JSON 输出中包含 `tr_4_1_non_workspace_count` 整数、`tr_4_2_reqwest_ok` 布尔、`criterion_default_features_remaining_count` 整数、`package_inheritance_defects` 数组。

### 完成证据

- RED 阶段 stdout/stderr 完整文本。

---

## Task 2: GREEN - 17 个 Cargo.toml + 根 Cargo.toml 修改

**优先级**: high  
**状态**: pending  
**依赖**: Task 1 完成 RED 输出  

### 工作内容

按顺序处理以下 17 个文件（注意根 Cargo.toml 第 0 项，非 17 子包之一，视情况补字段）：

0. `d:\a10\aikjx\gitcode\infotopograph\Cargo.toml`（根）
   - 检查 `[workspace.dependencies]` 是否覆盖子包全部外部 crate；若子包有依赖缺失（例如 workspace.deps 未定义但子包用到的 crate），先补至 workspace.deps；
   - 保留已有字段不变（版本号不改）；

1. `platform/services/ai-agent/Cargo.toml`
   - [package]：description 非 workspace 默认描述，保留；其他字段已 .workspace → OK；
   - [dependencies]：全部外部已 workspace=true（已检查状态 OK）；保留。

2. `platform/services/business-catalog/Cargo.toml`
   - [package]：
     - version `0.1.0` → version.workspace = true
     - edition `2021` → edition.workspace = true
     - description 为专用中文描述，保留；无 license/authors 字段 → 补 license.workspace = true、authors.workspace = true；

3. `platform/services/flow-ai/Cargo.toml`
   - 已检查：[package] 除 description 专用描述外，其余全继承；[dependencies] 外部全 workspace=true。OK，保留。

4. `platform/services/graph-algorithms/Cargo.toml`
   - 已检查：[package] 全继承（description.workspace = true）；[dependencies] 外部全 workspace=true。OK，保留。

5. `platform/services/hermes-flow-bridge/Cargo.toml`
   - [package]：
     - version `0.1.0` → version.workspace = true
     - edition `2021` → edition.workspace = true
     - license `MIT` → license.workspace = true
     - authors 缺失 → authors.workspace = true
     - description 专用中文描述，保留；
   - [dependencies] / [dev-dependencies]：外部 crate 已 workspace=true（reqwest optional=true 正常）。保留。

6. `platform/services/kg-hub/Cargo.toml`
   - [package]：description 专用中文，保留；其他已继承。OK。

7. `platform/services/operator-core/Cargo.toml`
   - 已检查：全继承；[dependencies] 外部 crate 全 workspace=true（nalgebra 附加 features 正常）；[dev-dependencies] criterion = { workspace = true } 正确。保留。

8. `platform/services/operator-wasm/Cargo.toml`
   - 已检查：全继承 + 外部全 workspace=true。保留。

9. `platform/services/optimizer/Cargo.toml`
   - 已检查：全继承 + 外部全 workspace=true。保留。

10. `platform/services/primiflow-core/Cargo.toml`
    - [package]：description 专用描述，保留；其他已继承。OK；
    - [dev-dependencies] reqwest：已 workspace=true（根据现状文件已改，需通过 Task 1 确认是否仍存在 0.11 旧写法；如 RED 发现仍有 0.11 版本号写法，则改为 workspace=true）。

11. `platform/services/primiflow-fusion/Cargo.toml`
    - [package]：
      - version `0.1.0` → version.workspace = true
      - edition `2021` → edition.workspace = true
      - license 缺失 → license.workspace = true
      - authors 缺失 → authors.workspace = true
      - description 专用描述，保留；
    - [dependencies]：外部已 workspace=true；
    - [dev-dependencies]：criterion = { workspace = true }（已正确），tower 附加 features 正常。保留。

12. `platform/services/template-market/Cargo.toml`
    - [package]：description 专用描述，保留；其他已继承。OK；
    - [dependencies]：使用 `serde.workspace = true` 的简写语法；统一到 `serde = { workspace = true }` 风格以保持一致性（治理一致性 AC-7）。

13. `platform/services/xuanji-common-meta/Cargo.toml`
    - 已检查：全继承 + 外部全 workspace=true。保留。

14. `platform/services/xuanji-expert/Cargo.toml`
    - [package]：
      - version `0.1.0` → version.workspace = true
      - edition `2021` → edition.workspace = true
      - license `MIT` → license.workspace = true
      - authors 缺失 → authors.workspace = true
      - description 专用描述，保留；
    - [dependencies]：外部全 workspace=true（reqwest 附加 features=["blocking"] 正常）。保留。

15. `platform/services/xuanji-system/Cargo.toml`
    - [package]：
      - version `0.1.0` → version.workspace = true
      - edition `2021` → edition.workspace = true
      - license `MIT` → license.workspace = true
      - authors 缺失 → authors.workspace = true
      - description 专用描述，保留；
    - [dependencies]：外部全 workspace=true（axum 附加 features=["ws"]、sea-query 附加 3 个 backend features、sqlx 附加 3 个 features 正常）。保留。

16. `platform/gateway/runtime/Cargo.toml`
    - [package]：已全继承。OK；
    - [dependencies]：外部全 workspace=true（axum 附加 features=["ws"] 正常、nalgebra 附加 features=["serde-serialize"] 正常）。保留。

### 根 Cargo.toml 补充检查项

- 检查所有子包 [dependencies] / [dev-dependencies] 使用的外部 crate 是否在 `[workspace.dependencies]` 中：
  - serde / serde_json / serde_yaml / tokio / anyhow / thiserror / tracing / tracing-subscriber / uuid / chrono / rayon / http / hostname / nalgebra / ndarray / num-traits / approx / criterion / wasmer / wasmer-compiler-cranelift / petgraph / axum / tower-http / tower / reqwest / base64 / rusqlite / sha2 / hmac / hex / parking_lot / async-trait / tokio-tungstenite / futures / futures-util / sea-query / sqlx
  - 以上经清点已覆盖全部使用；无需补。

### 任务本地测试要求（TR）

- **TR-T2-1（rule）**：所有 17 个子包 [dependencies] / [dev-dependencies] 的外部依赖（非 path 内部）都包含 `workspace = true` 或 `.workspace = true`（脚本 TR 4.1 计数后，若 ≤1 则通过，否则继续修改）。
- **TR-T2-2（rule）**：所有 criterion 声明无 `default-features` 键（脚本 criterion_default_features_remaining_count = 0）。
- **TR-T2-3（rule）**：5 个 package 字段待修复的 crate（business-catalog / hermes-flow-bridge / primiflow-fusion / xuanji-expert / xuanji-system）全部改为 `*.workspace = true`（除 description 外）。
- **TR-T2-4（rule）**：template-market 的依赖语法统一为 `dep = { workspace = true }` 表形式（不再使用 `dep.workspace = true` 简写，保持 crate 间一致）。
- **TR-T2-5（rubric，0-2，阈值=2）**：治理一致性评分：2=所有写法在 crate 间无差异（外部依赖均使用表形式，features 附加大写/小写与原 workspace 一致），1=≤2 处差异但不影响构建，0=明显不一致。

### 完成证据

- 修改文件绝对路径列表 + 关键 diff 片段。

---

## Task 3: 例外清点与文档化

**优先级**: medium  
**状态**: pending  
**依赖**: Task 2 自验证通过  

### 工作内容

- 列出所有未 workspace 化的外部依赖（即 TR 4.1 > 0 时的计数项）；
- 判断是否属于"合法"例外（如：workspace.deps 不存在但 crate 独有、或 features 组合 workspace 未提供且无法附加）；
- 如果例外数 > 1：回 Task 2 继续修改，补 workspace.deps 后再 workspace=true；
- 最终例外数 ≤ 1：在脚本输出和最终交付中写明。

features 附加（例如 axum ws、reqwest blocking、sea-query backend-sqlite 等）**不属于**例外。

### 任务本地测试要求（TR）

- **TR-T3-1（rule）**：`exception_list.length` ≤ 1。

### 完成证据

- 例外列表 + 文档化说明。

---

## Task 4: GREEN 验证 - 合规脚本再次执行

**优先级**: high  
**状态**: pending  
**依赖**: Task 2 完成 + Task 3 例外 ≤1  

### 工作内容

- 运行 `test-tr-4-compliance.js` 获取 GREEN 输出；
- 确认：
  - TR 4.1 ≤ 1
  - TR 4.2 `reqwest_ok = true`（所有 cargo tree 出来的 reqwest 都是 0.12.x）
  - `criterion_default_features_remaining_count = 0`
  - `package_inheritance_defects = []`（除 description 外的 version/edition/license/authors 无缺陷）

### 任务本地测试要求（TR）

- **TR-T4-1（rule）**：脚本 exit = 0 且 `tr_4_1_non_workspace_count ≤ 1`。
- **TR-T4-2（rule）**：脚本 `tr_4_2_reqwest_ok = true`。
- **TR-T4-3（rule）**：脚本 `criterion_default_features_remaining_count = 0`。
- **TR-T4-4（rule）**：脚本 `package_inheritance_defects` 过滤掉 description 项后数组为空。

### 完成证据

- GREEN 阶段 stdout 完整文本。

---

## Task 5: Rust 编译验证

**优先级**: high  
**状态**: pending  
**依赖**: Task 4 GREEN 通过  

### 工作内容

1. 运行 `cargo check --workspace`，验证 exit = 0，无 error；
2. 运行 `cargo build --workspace`，验证 exit = 0，无 error；
3. 如果失败：最多 3 轮修复循环。
   - 第一轮：根据错误信息补 workspace.deps 或修正 features；
   - 第二轮：根据错误信息调整依赖继承；
   - 第三轮：仍失败则记录 blocker。

### 任务本地测试要求（TR）

- **TR-T5-1（rule）**：`cargo check --workspace` stderr 无 `error[E` 开头的编译错误，exit 0。
- **TR-T5-2（rule）**：`cargo build --workspace` stderr 无 `error[E` 开头的编译错误，exit 0。

### 完成证据

- 两条命令 exit code 为 0 的控制台输出文本。

---

## Task 6: 交付汇总 & 例外文档化

**优先级**: medium  
**状态**: pending  
**依赖**: Task 5 通过  

### 工作内容

- 汇总所有修改/新建文件绝对路径列表；
- 汇总 RED 输出文本 + GREEN 输出文本；
- 汇总 `cargo build --workspace` exit 0 证明；
- 文档化 ≤1 例外（若有）。

### 任务本地测试要求（TR）

- **TR-T6-1（rule）**：交付内容完整包含：文件路径列表 / RED output / GREEN output / cargo build exit 0 / 例外说明。

### 完成证据

- 最终交付文本。

---

## Review 检查点（供独立审阅者）

- 审阅 Task 1 RED 输出确认为失败状态；
- 审阅 Task 2 修改的 17 个文件，逐一对照 AC-1/AC-3/AC-4；
- 审阅 Task 4 GREEN 输出：TR 4.1 ≤ 1，TR 4.2 reqwest 全 0.12；
- 审阅 Task 5 cargo check + build 输出：exit 0；
- 审阅例外列表：数量 ≤ 1；
- 审阅治理一致性 rubric 评分自评是否与实际吻合。
