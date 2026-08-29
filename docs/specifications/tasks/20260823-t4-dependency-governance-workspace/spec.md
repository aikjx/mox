# T4 依赖治理 100% workspace=true

## 问题 / 用户 / 目标

当前 workspace 根 `Cargo.toml` 已定义了 `[workspace.dependencies]`，覆盖 serde/tokio/criterion/reqwest/rusqlite 等核心 crate，但 17 个子包 Cargo.toml 中仍存在：

- 部分 `[package]` 元数据字段（version/edition/license/authors/description）以硬编码值写出，未使用 `workspace = true` 继承；
- 少量外部依赖仍保留具体版本号而非 `workspace = true`；
- `primiflow-core` 的 `[dev-dependencies]` 存在 reqwest 版本漂移（0.11 旧版 vs workspace 0.12）；
- criterion 在各 crate 中可能仍显式指定 `default-features = false`，与 workspace 已统一设置重复。

目标：统一治理 17 个 Rust 包，所有可继承的非 build-dependencies 依赖全部改为 `workspace = true`，`[package]` 元数据可继承的字段也全部继承，实现依赖声明 100% workspace 化（例外 ≤1 且文档化）。

## 非目标

- 不修改 `[build-dependencies]`（本次仅约束 `[dependencies]` / `[dev-dependencies]`）；
- 不调整 workspace.dependencies 的版本号（已在根 Cargo.toml 固化，保持不动）；
- 不修改内部 crate 的 `path = "..."` 指向（内部 crate 路径依赖不算"外部非 workspace 化"）；
- 不引入新 crate，不删除已有 crate；
- 不改变 `Cargo.lock`（除版本漂移修复后的锁文件更新）。

## 功能需求（FR）

| ID | 描述 |
|----|------|
| FR-1 | 扫描并清点 17 个目标 Cargo.toml（platform/services/* 16 个 + platform/gateway/runtime 1 个） |
| FR-2 | `[dependencies]` 与 `[dev-dependencies]` 中所有外部 crate 依赖（非 path 指向的 workspace 内部 crate），只要 `workspace.dependencies` 已声明，全部改为 `workspace = true`；保留 features/optional 等附加键 |
| FR-3 | 修复 primiflow-core `[dev-dependencies]` 中 reqwest 版本漂移（0.11 → workspace 0.12） |
| FR-4 | 所有使用 criterion 的 crate，统一写为 `criterion = { workspace = true }`，去掉各 crate 独立的 `default-features = false`（workspace 已含 `default-features = false`） |
| FR-5 | `[package]` 段中，除 `name` 外，若 `workspace.package` 已声明对应字段（version/edition/license/authors/description），则改为 `xxx.workspace = true` 或 `xxx = { workspace = true }` 风格；description 若 crate 有专用描述则保留原文，不强制继承 |
| FR-6 | 特例记录：若存在必须锁定与 workspace 不同版本或不同 default-features 的依赖（含 axum ws feature 等 features 附加的正常场景），需要有 ≤1 个例外并在最终报告文档化（features 附加不属于例外） |
| FR-7 | 根 `Cargo.toml` `[workspace.dependencies]` 缺少某个子包使用的外部 crate 时，先补 workspace.deps 再让子包引用 |

## 非功能需求（NFR）

| ID | 描述 |
|----|------|
| NFR-1 | `cargo check --workspace` exit code = 0，0 error |
| NFR-2 | `cargo build --workspace` exit code = 0，0 error |
| NFR-3 | 自定义合规脚本 `test-tr-4-compliance.js` TR 4.1 ≤ 1（非 workspace 的依赖声明行统计） |
| NFR-4 | TR 4.2：`cargo tree -p primiflow-core -i reqwest` 输出中所有 reqwest 前缀版本 = 0.12.x |
| NFR-5 | 所有修改后的 Cargo.toml 保持 TOML 语法有效，`cargo metadata --format-version 1 --no-deps` exit 0 |
| NFR-6 | 不破坏内部 crate 之间的 path 依赖关系与 feature 开启 |

## 约束 / 依赖 / 假设

- 根 Cargo.toml 的 `[workspace.package]` 已提供 version / edition / license / authors / description 字段；
- 根 Cargo.toml 的 `[workspace.dependencies]` 已声明 serde、serde_json、serde_yaml、tokio、anyhow、thiserror、tracing、tracing-subscriber、uuid、chrono、rayon、http、hostname、nalgebra、ndarray、num-traits、approx、criterion、wasmer、wasmer-compiler-cranelift、petgraph、axum、tower-http、tower、reqwest、base64、rusqlite、sha2、hmac、hex、parking_lot、async-trait、tokio-tungstenite、futures、futures-util、sea-query、sqlx 等；
- 若子包使用的外部 crate workspace.deps 未覆盖（例如尚未发现的 crate），则先补 workspace.deps 后再改 workspace=true；
- `name` 字段是每个 crate 的唯一标识，必须保留硬编码，不做继承；
- description 若 crate 有自定义描述则保留，但若与 workspace.package.description 完全一致则改为继承。

## 开放问题

无。当前需求边界清晰。

## 验收标准（Acceptance Criteria）

- **AC-1（rule）**：17 个目标 Cargo.toml 中，`[dependencies]` / `[dev-dependencies]` 下所有已被 `[workspace.dependencies]` 覆盖的外部 crate 均使用 `workspace = true` 形式（可附加 features/optional 等键），且此类非 workspace 化的声明行数 ≤ 1（TR 4.1）。
- **AC-2（rule）**：primiflow-core 的 reqwest 不再存在 0.11 版本，`cargo tree -p primiflow-core -i reqwest` 所有行前缀 = `0.12.`（TR 4.2）。
- **AC-3（rule）**：所有 criterion 声明均为 `criterion = { workspace = true }`（无 default-features 重复设置）。
- **AC-4（rule）**：`[package]` 中 version / edition / license / authors 字段，若 `workspace.package` 对应字段存在，则均以 `*.workspace = true` 形式声明（name 保留原文，description 可例外：若 crate 自定义则保留，但与 workspace 默认一致时应继承）。
- **AC-5（rule）**：`cargo check --workspace` exit = 0；`cargo build --workspace` exit = 0。
- **AC-6（rule）**：非 workspace 化的外部依赖声明例外数 ≤ 1，且在最终交付文档明确描述。
- **AC-7（rubric，0-2，阈值=2）**：依赖治理一致性——全部 workspace 化的声明在各 crate 间写法一致（features 附加合理，无不一致的重复 default-features 覆盖）；得 2 = 完全一致，1 = ≤2 处写法不一致但功能等价，0 = 明显不一致影响可读性。
