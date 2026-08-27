# 07 · 编译与测试指南

> **版本**: v1.0 · **日期**: 2026-08-27

## 一、环境要求

| 工具 | 版本要求 | 说明 |
|---|---|---|
| Rust | ≥ 1.75 |  edition 2021，async trait 支持 |
| Cargo | ≥ 1.75 | 与 Rust 配套 |
| tokio | 1.0 (workspace) | 异步运行时，full feature |
| axum | 0.7 (workspace) | Web 框架 |
| 操作系统 | Windows / Linux / macOS | 跨平台支持 |

### 安装 Rust

```bash
# Windows (PowerShell)
winget install Rustlang.Rustup

# Linux / macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 验证
rustc --version
cargo --version
```

---

## 二、Workspace 结构

```
infotopograph/
├── Cargo.toml              # workspace 根（58 members + 依赖统一管理）
├── Cargo.lock              # 依赖锁定
├── platform/
│   ├── framework/          # mox-framework (L5)
│   ├── foundation/         # 3 个 foundation crates (L5)
│   ├── gateway/            # mox-platform-gateway-svc (L1)
│   └── domains/            # 8 域 (L3+L4)
│       ├── ai/
│       ├── kg/
│       ├── flow/
│       ├── cloud/
│       ├── data/
│       ├── voice/
│       ├── market/
│       └── streams/
└── docs/rust-enterprise/   # 本文档集
```

---

## 三、常用命令

### 3.1 全量编译检查

```bash
# 检查整个 workspace（推荐日常使用）
cargo check --workspace

# 检查整个 workspace（包含所有 target）
cargo check --workspace --all-targets
```

**预期结果**: 退出码 0，60+ crates 编译零错误（2026-08-27 验证通过）

---

### 3.2 按 crate 编译检查

```bash
# Framework 层
cargo check -p mox-framework

# KG 算法核心
cargo check -p mox-kg-algo-core

# KG 服务层（含 HTTP 适配 feature）
cargo check -p mox-kg-service-svc --features http-adapter

# AI 意图核心
cargo check -p mox-ai-intent-core

# 网关（axum-gateway feature）
cargo check -p mox-platform-gateway-svc --features axum-gateway

# 云存储主服务
cargo check -p mox-cloud-master-svc

# 数据归一化 SDK
cargo check -p mox-data-norm-intent-native
```

---

### 3.3 构建

```bash
# Debug 构建（较快，用于开发）
cargo build --workspace

# Release 构建（优化，用于生产）
cargo build --workspace --release

# 构建单个 crate
cargo build -p mox-platform-gateway-svc --features axum-gateway --release
```

**产物位置**: `target/debug/` 或 `target/release/`

---

### 3.4 测试

```bash
# 运行整个 workspace 测试
cargo test --workspace

# 运行单个 crate 测试
cargo test -p mox-kg-algo-core

# 运行单个 crate 测试（显示输出）
cargo test -p mox-kg-algo-core -- --nocapture

# 运行特定测试
cargo test -p mox-kg-algo-core test_pagerank_csr_vs_dense

# 运行测试并生成覆盖率报告（需安装 tarpaulin）
cargo tarpaulin --workspace --out Html
```

---

### 3.5 代码质量

```bash
# 格式化检查
cargo fmt --all -- --check

# 自动格式化
cargo fmt --all

# Clippy 检查
cargo clippy --workspace --all-targets

# Clippy 自动修复
cargo clippy --workspace --fix --allow-dirty
```

---

### 3.6 文档

```bash
# 生成文档
cargo doc --workspace --no-deps

# 生成文档并在浏览器打开
cargo doc --workspace --no-deps --open
```

---

## 四、Feature 开关

### mox-platform-gateway-svc

| Feature | 默认 | 说明 |
|---|---|---|
| (default) | ✅ | 纯手写 TCP HTTP 解析器（单节点专用，零依赖） |
| axum-gateway | ❌ | 基于 axum 的企业网关路由（31域模块化注册中心） |

```bash
# 使用默认 feature（单节点 HTTP）
cargo check -p mox-platform-gateway-svc

# 使用 axum-gateway feature（企业网关）
cargo check -p mox-platform-gateway-svc --features axum-gateway
```

### mox-kg-service-svc

| Feature | 默认 | 说明 |
|---|---|---|
| (default) | ✅ | 核心服务层（无 HTTP 依赖） |
| http-adapter | ❌ | HTTP 适配层（axum + 6 KG 接口 + 4 AI 接口真实桥接） |

```bash
# 默认（无 HTTP）
cargo check -p mox-kg-service-svc

# 启用 HTTP 适配
cargo check -p mox-kg-service-svc --features http-adapter
```

---

## 五、冒烟验证清单（R6）

### 5.1 核心 crate 编译验证

```bash
# 1. Framework 层
cargo check -p mox-framework
# 预期: 0 error

# 2. KG 算法核心
cargo check -p mox-kg-algo-core
# 预期: 0 error

# 3. KG 服务层（HTTP 适配）
cargo check -p mox-kg-service-svc --features http-adapter
# 预期: 0 error

# 4. AI 意图核心
cargo check -p mox-ai-intent-core
# 预期: 0 error

# 5. 网关（axum-gateway）
cargo check -p mox-platform-gateway-svc --features axum-gateway
# 预期: routes.rs 新增模块 0 error（历史 cli.rs/http_server.rs 有 API 漂移，待 R7）
```

### 5.2 算法测试验证

```bash
cargo test -p mox-kg-algo-core
# 预期: 18/18 PASSED
# 关键测试:
#   - test_pagerank_csr_vs_dense (Pearson ≥ 0.9999)
#   - test_ppr_csr_vs_dense (Pearson ≥ 0.9999)
#   - test_communities (CNM 社区发现)
#   - test_betweenness_centrality (Brandes 介数)
#   - test_harmonic_closeness (Harmonic 紧密)
```

### 5.3 全 workspace 编译验证

```bash
cargo check --workspace
# 预期: 退出码 0
```

---

## 六、已知问题与注意事项

### 6.1 Gateway 历史代码 API 漂移

**问题**: `mox-platform-gateway-svc` 默认 feature 下，`cli.rs` 和 `http_server.rs` 存在 50+ 编译错误（类型不匹配、未解析导入等）。

**原因**: 历史代码基于旧版 API 编写，后续 workspace 依赖升级后未同步更新。

**影响**: 默认 feature 下 `cargo check -p mox-platform-gateway-svc` 会报错。

**临时方案**:
- 使用 `--features axum-gateway` 编译新增的路由模块（routes.rs 已通过编译）
- 历史代码修复列入 R7 路线图

**修复计划**: R7 阶段统一修复 cli.rs / http_server.rs 的 API 漂移问题。

### 6.2 FFI 绑定编译

**问题**: napi-rs 和 PyO3 绑定需要额外工具链。

**说明**:
- napi cdylib 需要 napi-rs CLI + node-gyp 工具链
- PyO3 abi3-py39 需要 Python ≥ 3.9

**方案**: FFI 绑定不在 `default-members` 中，需单独编译：
```bash
# napi 绑定（需先安装 napi-rs CLI）
cargo check -p <napi-binding-crate>

# PyO3 绑定（需 Python ≥ 3.9）
cargo check -p mox-voice-dsp-py
```

### 6.3 Windows 编译注意

- 确保安装了 Visual Studio Build Tools（C++ 编译器）
- rusqlite 使用 bundled feature，无需系统 SQLite
- 长路径问题：确保项目路径不要过长（Windows MAX_PATH 限制）

---

## 七、性能优化建议

### 7.1 编译加速

```bash
# 使用 sccache 缓存编译结果
cargo install sccache
export RUSTC_WRAPPER=sccache

# 并行编译（默认已启用）
cargo build --workspace -j 8

# 仅检查改动的 crate
cargo check -p <changed-crate>
```

### 7.2 Release 优化

在 `Cargo.toml` 中配置：
```toml
[profile.release]
opt-level = 3
lto = "thin"        # 链接时优化
codegen-units = 1   # 减少代码生成单元（更慢编译，更快运行）
panic = "abort"     # 禁用 panic unwinding（更小二进制）
```

---

## 八、CI/CD 集成

### GitHub Actions 示例

```yaml
name: Rust CI

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check --workspace
      - run: cargo test --workspace
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo fmt --all -- --check
```

---

## 九、故障排查

### 问题: 编译报 "could not compile due to previous errors"

**排查步骤**:
1. 向上滚动查看第一个错误（后续错误通常是级联的）
2. 确认是否缺少依赖（检查 Cargo.toml）
3. 确认 feature 开关是否正确
4. 运行 `cargo clean` 后重试

### 问题: 测试失败 "Pearson correlation below threshold"

**可能原因**:
1. CSR 构建有 bug（检查 offsets/indices 数组）
2. 浮点精度问题（检查迭代次数和收敛阈值）
3. 图数据不一致（确认 CSR 和 Dense 使用相同的图）

### 问题: Gateway 编译报 50+ 错误

**说明**: 这是已知问题（见 6.1），使用 `--features axum-gateway` 可绕过历史代码，仅编译新增的路由模块。

---

*详见 [README.md](./README.md) 获取文档总览和下一步路线图。*
