# mox-ai-expert-svc 编译与测试验证报告

**验证日期**: 2026-08-31
**项目路径**: `platform/domains/ai/svc/mox-ai-expert-svc`
**项目版本**: v3.0.0-ai-powered

---

## 一、编译状态

### 1.1 cargo check --lib（库编译检查）

**状态**: 通过 (PASSED)

- 退出码: 0
- 警告数: 4
- 错误数: 0

**警告列表**:

| 序号 | 级别 | 文件 | 行号 | 类型 | 描述 |
|------|------|------|------|------|------|
| 1 | warning | `src/alliance/intent.rs` | 16 | unused_imports | 未使用的导入 `BTreeSet` |
| 2 | warning | `src/alliance/gate.rs` | 326 | unused_assignments | 变量 `retried_flag` 赋值后从未读取 |
| 3 | warning | `src/alliance/mod.rs` | 142 | dead_code | 结构体字段 `started_at` 从未读取 |
| 4 | warning | `src/alliance/kg_connector/http.rs` | 32 | dead_code | 结构体字段 `timeout_ms` 从未读取 |

### 1.2 cargo test --lib（测试编译检查）

**状态**: 失败 (FAILED) — 编译阶段即失败，测试未能执行

- 退出码: 101
- 编译错误数: 3
- 警告数: 1

**编译错误详情**:

| 序号 | 错误码 | 文件 | 行号 | 描述 |
|------|--------|------|------|------|
| 1 | E0308 | `src/alliance/algorithm.rs` | 694 | 类型不匹配：期望 `HashMap<String, String>`，实际为 `BTreeMap<_, _>`（`context` 字段） |
| 2 | E0308 | `src/alliance/orchestration.rs` | 416 | 类型不匹配：期望 `HashMap<String, String>`，实际为 `BTreeMap<_, _>`（`constraints` 字段） |
| 3 | E0308 | `src/alliance/orchestration.rs` | 417 | 类型不匹配：期望 `HashMap<String, String>`，实际为 `BTreeMap<_, _>`（`context` 字段） |

**根因分析**:
结构体 `AlgorithmAnalysisRequest` 和 `OrchestrationRequest` 中的 `context` / `constraints` 字段已从 `BTreeMap` 改为 `HashMap`（见 `src/types.rs` 第 355、390、392 行），但 `#[cfg(test)]` 模块中的测试辅助函数 `make_req()` 仍使用 `BTreeMap::new()` 初始化，导致 test 配置下编译失败。这是一个**类型重构遗留问题**——生产代码已更新但测试代码未同步。

---

## 二、测试结果统计

### 2.1 单元测试（lib 内联测试）

由于编译失败，**测试无法执行**。

**已发现的测试数量统计**（基于源码静态扫描）:

| 类别 | 测试函数数量 | 所在位置 |
|------|-------------|----------|
| 单元测试 (#[test]) | ~120 | `src/` 目录下 39 个文件 |
| 异步测试 (#[tokio::test]) | ~31 | `src/` 目录下 |
| **单元测试总计** | **~151** | **39 个测试模块** |
| 集成测试 | ~68 | `tests/` 目录下 12 个测试文件 |

受编译错误影响的模块:
- `src/alliance/algorithm.rs` — 9 个测试函数（全部无法编译）
- `src/alliance/orchestration.rs` — 5 个测试函数（全部无法编译）

### 2.2 集成测试

集成测试 (`tests/` 目录) 未执行。由于依赖库的 test 配置编译失败，集成测试同样无法运行。

---

## 三、Clippy 代码质量检查

### 3.1 clippy.toml 配置问题

**状态**: 配置文件格式错误

`platform/clippy.toml` 使用了无效的顶层字段 `allow`、`deny`、`warn`。这些是 **cargo clippy 命令行参数**，而非 clippy.toml 配置文件支持的字段。clippy.toml 仅支持具体的 lint 配置项（如 `too-many-arguments-threshold`、`type-complexity-threshold` 等）。

错误信息:
```
error: error reading Clippy's configuration file: unknown field `allow`, expected one of
    absolute-paths-allowed-crates, ...
```

### 3.2 Clippy 检查结果（绕过配置文件后）

临时移除无效配置文件后运行 clippy，结果如下:

**状态**: 1 个错误 + 17 个警告

| 级别 | 数量 | 说明 |
|------|------|------|
| error | 1 | `never_loop` — 循环体从不实际循环 |
| warning | 17 | 各类代码风格/性能/可维护性警告 |

**Clippy 错误（必须修复）**:

| 序号 | Lint | 文件 | 行号 | 描述 |
|------|------|------|------|------|
| 1 | never_loop (deny) | `src/alliance/gate.rs` | 320 | `loop { ... break ... }` 结构实际从不循环，应改用普通代码块 |

**Clippy 主要警告分类**:

| 类别 | 数量 | 示例 |
|------|------|------|
| 未使用代码 (unused/dead_code) | 4 | 未使用导入、未读取变量、死代码字段 |
| 代码风格 (style) | 5 | `useless_format`、`match_ref_pats`、`len_zero`、`manual_clamp`、`doc_overindented_list_items` |
| 特质实现 (should_implement_trait) | 2 | `from_str` 方法应实现 `FromStr` trait |
| 复杂度 (complexity) | 2 | `too_many_arguments`、`type_complexity` |
| 优化建议 (perf) | 4 | `unnecessary_map_or`、`unwrap_or_default` (3处) |
| 其他 | 1 | `mox-kg-sdk` 依赖中的 `unnecessary_cast` |

---

## 四、前端构建验证

### 4.1 环境信息

| 项目 | 版本 |
|------|------|
| Node.js | v22.16.0 |
| npm | 10.9.4 |
| 包管理器 (packageManager) | pnpm@11.15.1 |
| node_modules | 已存在 |
| 构建工具 | Vite v5.4.21 |

### 4.2 构建结果

**状态**: 失败 (FAILED)

- 构建命令: `npm run build` (即 `vite build`)
- 退出码: 1
- 已转换模块数: 330
- 失败原因: 重复变量声明

**错误详情**:

```
error during build:
[vite:vue] [vue/compiler-sfc] Identifier 'kbSearch' has already been declared. (883:6)

文件: src/views/workspace/ExpertWorkspaceView.vue
```

**根因分析**:

在 `ExpertWorkspaceView.vue` 中，`kbSearch` 标识符被声明了两次:

1. **第 715 行** — API 函数导入:
   ```js
   import { kbSearch, kbGetVersions, kbGetStats } from '@/api/kb.api.js'
   ```

2. **第 1578 行** — 响应式变量声明:
   ```js
   const kbSearch = ref('')
   ```

两者位于同一个 `<script setup>` 作用域中，命名冲突导致编译失败。API 函数 `kbSearch` 与本地 ref 变量 `kbSearch` 重名。

---

## 五、问题汇总

### 5.1 阻断性问题（必须修复）

| # | 严重度 | 模块 | 问题描述 | 影响 |
|---|--------|------|----------|------|
| 1 | 高 | expert-svc (test) | `algorithm.rs` 和 `orchestration.rs` 测试代码中 `BTreeMap::new()` 与结构体 `HashMap` 字段类型不匹配 | 所有单元测试和集成测试无法编译运行 |
| 2 | 高 | frontend-ui | `ExpertWorkspaceView.vue` 中 `kbSearch` 重复声明（导入函数与本地 ref 重名） | 前端生产构建完全失败 |
| 3 | 中 | platform | `clippy.toml` 配置文件格式错误，使用了不支持的 `allow`/`deny`/`warn` 字段 | Clippy 代码质量门禁无法使用 |
| 4 | 中 | expert-svc | `gate.rs:320` 存在 `never_loop` 错误（clippy deny 级别） | 若启用 clippy 门禁将编译失败 |

### 5.2 非阻断性问题（建议修复）

| # | 级别 | 模块 | 问题描述 | 建议 |
|---|------|------|----------|------|
| 5 | 低 | expert-svc | 4 个未使用代码警告（unused import、dead code 字段等） | 清理未使用代码或添加 `#[allow(dead_code)]` |
| 6 | 低 | expert-svc | 2 处 `from_str` 方法应实现 `FromStr` trait | 实现标准 trait 提升代码规范 |
| 7 | 低 | expert-svc | `useless_format`、`len_zero`、`manual_clamp` 等风格问题 | 运行 `cargo clippy --fix` 自动修复 |
| 8 | 低 | expert-svc | `too_many_arguments` (8/7) — `audit_events_for_full_pipeline` 函数参数过多 | 考虑使用参数结构体封装 |
| 9 | 低 | expert-svc | `type_complexity` — `expert_score_history` 字段类型过于复杂 | 使用 type alias 简化 |

---

## 六、修复建议

### 6.1 高优先级 — 修复测试编译错误

**文件 1**: `src/alliance/algorithm.rs` 第 694 行

将测试模块中的:
```rust
context: BTreeMap::new(),
```
改为:
```rust
context: HashMap::new(),
```
并确保 `use std::collections::HashMap;` 已导入。

**文件 2**: `src/alliance/orchestration.rs` 第 416-417 行

将测试模块中的:
```rust
constraints: BTreeMap::new(),
context: BTreeMap::new(),
```
改为:
```rust
constraints: HashMap::new(),
context: HashMap::new(),
```
并确保 `use std::collections::HashMap;` 已导入。

### 6.2 高优先级 — 修复前端构建错误

**文件**: `frontend-ui/src/views/workspace/ExpertWorkspaceView.vue`

方案 A（推荐）— 重命名本地 ref 变量:
```js
// 第 1578 行，将 kbSearch 重命名为 kbSearchKeyword
const kbSearchKeyword = ref('')
```
然后更新所有使用 `kbSearch.value` 的地方（模板中的 `v-model="kbSearch"` 和 script 中的引用）。

方案 B — 重命名导入的 API 函数:
```js
import { kbSearch as kbSearchApi, kbGetVersions, kbGetStats } from '@/api/kb.api.js'
```

### 6.3 中优先级 — 修复 clippy.toml 配置

`platform/clippy.toml` 中的 `allow`、`deny`、`warn` 字段应移至 `Cargo.toml` 的 `[lints.clippy]` 节，或通过 `.cargo/config.toml` 配置命令行参数。clippy.toml 仅用于具体 lint 的阈值配置。

推荐方案 — 使用 `[lints]` 表（Rust 1.74+ 支持）:
```toml
# 在 Cargo.toml workspace 中添加
[workspace.lints.clippy]
new_without_default = "allow"
too_many_arguments = "allow"
type_complexity = "allow"
module_inception = "allow"
correctness = "deny"
suspicious = "deny"
# ... 等等
```

### 6.4 中优先级 — 修复 never_loop 错误

**文件**: `src/alliance/gate.rs` 第 320 行

`loop { ... break ... }` 结构从不实际循环，应重构为普通代码块或明确标注为 `'retry:` 命名循环 + 条件 break 以表明重试意图。

---

## 七、总结

| 检查项 | 状态 | 通过/失败 |
|--------|------|-----------|
| 库编译 (cargo check --lib) | 通过 | 4 警告，0 错误 |
| 测试编译 (cargo test --lib) | 失败 | 3 编译错误，测试无法执行 |
| Clippy 检查 | 配置错误 + 1 error | 配置文件无效，绕过配置后 1 error + 17 warnings |
| 前端构建 (npm run build) | 失败 | 重复变量声明导致构建失败 |

**总体评价**: 项目生产代码编译通过，但测试代码和前端构建均存在阻断性问题。建议优先修复 2 个高优先级问题后再运行完整测试套件。
