# MOX Platform 架构-数据分离规范

> **版本**: v1.0  
> **日期**: 2026-08-27  
> **状态**: 生效  
> **适用范围**: 全项目所有 crate、服务、插件

---

## 一、核心原则

### 1.1 三层分离

```
┌─────────────────────────────────────────────────────────┐
│  架构代码层 (platform/)          ← Git 追踪，只读边界    │
│  配置文件层 (config/)            ← Git 追踪，模板化      │
│  运行时数据层 (data/plugins/)    ← .gitignore，不入库    │
└─────────────────────────────────────────────────────────┘
```

### 1.2 铁律

1. **`platform/` 目录是纯架构代码的只读边界**，禁止存放任何运行时生成的数据
2. **代码中禁止硬编码相对路径**（如 `"./data/"`、`"./config/"`），必须通过 `mox-platform-paths` crate 管理
3. **所有路径可通过环境变量覆盖**，默认值遵循项目根目录布局
4. **第三方插件、模型、源码必须放在 `platform/` 之外**

---

## 二、目录布局

### 2.1 顶层目录结构

```
infotopograph/
├── platform/                    # 🔒 纯架构代码（Git 追踪）
│   ├── foundation/              # L0 基础库
│   │   ├── mox-platform-paths/ # 🆕 统一路径管理
│   │   ├── mox-platform-foundation/
│   │   ├── mox-cloud-foundation/
│   │   └── mox-platform-observability/
│   ├── domains/                 # 8 个领域
│   │   ├── {data,ai,kg,cloud,platform,voice,flow,market}/
│   │   │   ├── api/             # L2 trait 契约
│   │   │   ├── core/            # L3 领域核心
│   │   │   ├── svc/             # L4 业务服务
│   │   │   └── sdk/             # L5 FFI 绑定
│   ├── gateway/                 # L1 API 网关
│   ├── arch-test/               # 架构测试
│   ├── framework/
│   └── scripts/
│
├── config/                      # 📁 配置文件（Git 追踪）
│   ├── gateway.yaml             # 网关配置
│   ├── paths.env.example        # 路径环境变量模板
│   └── ...
│
├── data/                        # 💾 运行时数据（.gitignore）
│   ├── storage/                 # SQLite/LevelDB 数据文件
│   ├── cache/                   # 缓存文件
│   ├── logs/                    # 应用日志
│   ├── uploads/                 # 用户上传
│   └── exports/                 # 导出文件
│
├── plugins/                     # 🔌 第三方插件（.gitignore）
│   ├── wasm/                    # WASM 插件
│   ├── scripts/                 # 脚本插件 (Python/Lua)
│   └── extensions/              # 扩展包
│
├── third_party/                 # 📦 第三方源码/模型（.gitignore 或 submodule）
│   ├── CosyVoice/               # 语音模型
│   └── models/                  # AI 模型权重
│
├── shared/                      # 🔗 跨语言共享（Git 追踪）
│   ├── config/
│   ├── constants/
│   └── schemas/
│
├── .runtime/                    # ⚡ 运行时状态（.gitignore）
│   ├── *.pid
│   ├── *.sock
│   └── *.lock
│
├── docs/                        # 📖 文档
├── frontend-ui/                 # 🎨 前端
├── scripts/                     # 🔧 运维脚本
├── tests/                       # 🧪 集成测试
└── tools/                       # 🛠️ 开发工具
```

### 2.2 各目录职责

| 目录 | Git 追踪 | 运行时写入 | 说明 |
|---|---|---|---|
| `platform/` | ✅ | ❌ | 纯架构代码，禁止存放数据 |
| `config/` | ✅ | ❌ | 配置模板，运行时只读 |
| `data/` | ❌ | ✅ | 运行时生成的数据 |
| `plugins/` | ❌ | ✅ | 第三方插件，按需加载 |
| `third_party/` | ❌/submodule | ❌ | 第三方源码/模型 |
| `shared/` | ✅ | ❌ | 跨语言共享常量/schema |
| `.runtime/` | ❌ | ✅ | PID/socket/lock 等状态 |

---

## 三、路径管理模块 (`mox-platform-paths`)

### 3.1 设计目标

- **集中管理**：所有路径通过 `ProjectRoot` 结构体统一获取
- **环境变量覆盖**：所有路径可通过 `MOX_*_DIR` 环境变量覆盖
- **自动检测**：项目根目录自动向上查找 `platform/` + `Cargo.toml`
- **分离验证**：内置 `verify_separation()` 方法验证架构与数据不重叠
- **目录确保**：`ensure_all_dirs()` 启动时一次性创建所有必要目录

### 3.2 环境变量规范

| 环境变量 | 用途 | 默认值 |
|---|---|---|
| `MOX_ROOT` | 项目根目录 | 自动检测 |
| `MOX_DATA_DIR` | 数据目录 | `$MOX_ROOT/data` |
| `MOX_PLUGINS_DIR` | 插件目录 | `$MOX_ROOT/plugins` |
| `MOX_THIRD_PARTY_DIR` | 第三方目录 | `$MOX_ROOT/third_party` |
| `MOX_RUNTIME_DIR` | 运行时目录 | `$MOX_ROOT/.runtime` |
| `MOX_CONFIG_DIR` | 配置目录 | `$MOX_ROOT/config` |

### 3.3 使用示例

```rust
use mox_platform_paths::ProjectRoot;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 检测项目根目录
    let root = ProjectRoot::detect();

    // 2. 验证架构-数据分离
    root.verify_separation()?;

    // 3. 确保所有目录存在
    root.ensure_all_dirs()?;

    // 4. 获取路径（代码中禁止硬编码）
    let db_path = root.domain_db_path("kg");
    let log_dir = root.logs_dir();
    let plugin_dir = root.wasm_plugins_dir();

    println!("DB: {}", db_path.display());
    println!("Logs: {}", log_dir.display());
    println!("Plugins: {}", plugin_dir.display());

    Ok(())
}
```

### 3.4 API 速查

```rust
// 架构代码路径（只读）
root.platform_dir()      // platform/
root.config_dir()        // config/
root.shared_dir()        // shared/
root.docs_dir()          // docs/

// 运行时数据路径
root.data_dir()          // data/
root.storage_dir()       // data/storage/
root.cache_dir()         // data/cache/
root.logs_dir()          // data/logs/
root.uploads_dir()       // data/uploads/
root.exports_dir()       // data/exports/
root.domain_storage_dir("kg")  // data/storage/kg/
root.domain_db_path("data")    // data/storage/data.db

// 插件路径
root.plugins_dir()       // plugins/
root.wasm_plugins_dir()  // plugins/wasm/
root.script_plugins_dir()// plugins/scripts/
root.extensions_dir()    // plugins/extensions/

// 第三方路径
root.third_party_dir()   // third_party/
root.models_dir()        // third_party/models/

// 运行时状态路径
root.runtime_dir()       // .runtime/
root.pid_file("gateway") // .runtime/gateway.pid
root.socket_file("api")  // .runtime/api.sock
root.lock_file("db")     // .runtime/db.lock
```

---

## 四、架构测试（arch-test）

### 4.1 新增分离不变量测试

| 测试名 | 验证内容 |
|---|---|
| `test_architecture_data_separation` | `platform/` 下无 `.db`/`.sqlite`/`.log`/`.pid` 等数据文件 |
| `test_no_hardcoded_data_paths` | 代码中无硬编码的 `"./data/"`、`"./config/"` 等相对路径 |
| `test_plugins_outside_platform` | 插件文件（`.wasm`/`.so`/`.dll`）不在 `platform/` 内 |
| `test_third_party_outside_platform` | `third_party`/`vendor` 目录不在 `platform/` 内 |

### 4.2 运行架构测试

```bash
cargo test -p mox-arch-test
```

---

## 五、.gitignore 规范

### 5.1 必须忽略的目录

```gitignore
# 运行时数据
/data/

# 第三方插件
/plugins/

# 运行时状态
/.runtime/

# 第三方源码/模型
/third_party/

# 构建产物
/target/
/build/
/dist/
/artifacts/
/node_modules/
```

### 5.2 必须忽略的文件类型

```gitignore
# 数据库
*.db
*.db-wal
*.db-shm
*.sqlite
*.sqlite3

# 模型权重
*.bin
*.safetensors
*.gguf
*.onnx
*.pt
*.pth

# 日志
*.log

# 运行时状态
*.pid
*.sock
*.lock

# 环境变量
.env
.env.local
.env.*.local
```

### 5.3 保留 .gitkeep

空目录通过 `.gitkeep` 文件保留在 Git 中，但目录内容被忽略：

```
data/storage/.gitkeep
data/cache/.gitkeep
data/logs/.gitkeep
data/uploads/.gitkeep
data/exports/.gitkeep
plugins/wasm/.gitkeep
plugins/scripts/.gitkeep
plugins/extensions/.gitkeep
.runtime/.gitkeep
```

---

## 六、迁移指南

### 6.1 代码迁移步骤

1. **添加依赖**：在 crate 的 `Cargo.toml` 中添加
   ```toml
   mox-platform-paths = { workspace = true }
   ```

2. **替换硬编码路径**：
   ```rust
   // ❌ 旧代码（禁止）
   let db_path = "./data/storage/kg.db";

   // ✅ 新代码
   use mox_platform_paths::ProjectRoot;
   let root = ProjectRoot::detect();
   let db_path = root.domain_db_path("kg");
   ```

3. **启动时初始化**：
   ```rust
   let root = ProjectRoot::detect();
   root.verify_separation()?;
   root.ensure_all_dirs()?;
   ```

### 6.2 数据迁移步骤

1. 将 `platform/` 下的所有数据文件移动到 `data/` 对应子目录
2. 将 `platform/` 下的所有插件文件移动到 `plugins/` 对应子目录
3. 将 `platform/` 下的所有第三方源码移动到 `third_party/`
4. 运行架构测试验证：`cargo test -p mox-arch-test`

---

## 七、违规处理

### 7.1 CI 门禁

架构测试作为 CI 必过项，任何违规将导致构建失败：

```yaml
- name: Architecture Tests
  run: cargo test -p mox-arch-test
```

### 7.2 常见违规及修复

| 违规类型 | 示例 | 修复方式 |
|---|---|---|
| 硬编码数据路径 | `let p = "./data/db.sqlite";` | 使用 `ProjectRoot::domain_db_path()` |
| 数据文件在 platform/ | `platform/kg.db` | 移动到 `data/storage/kg.db` |
| 插件在 platform/ | `platform/plugin.wasm` | 移动到 `plugins/wasm/plugin.wasm` |
| 第三方在 platform/ | `platform/third_party/` | 移动到 `third_party/` |

---

## 八、附录

### 8.1 相关文档

- [架构分层规范](../enterprise/02-architecture.md)
- [ADR-15: Voice 域独立工作区](../enterprise/adr/ADR-015-voice-workspace.md)
- [全文档归一化总控卡](../enterprise/22-全文档归一化总控卡与权威链单源映射表-V1.0.md)

### 8.2 变更记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-27 | 初始版本，建立架构-数据三层分离规范 |
