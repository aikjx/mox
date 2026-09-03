# 统一插件架构文档

> 方案 C（双运行时混合架构）— 多运行时抽象、生命周期、权限模型、能力系统

## 架构总览

MOX 插件系统采用**双运行时混合架构**，同时支持 WASM 沙箱插件和 VSCode 扩展（VSIX）。通过统一的元数据层、多运行时抽象和权限模型，实现两种插件类型的无缝共存与统一管理。

```text
┌─────────────────────────────────────────────────────────────────┐
│                        插件市场 (Market)                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │ RemoteRegistry│  │ VSIX Market  │  │ Version Manager      │ │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘ │
└─────────┼───────────────────┼──────────────────────┼─────────────┘
          │                   │                      │
┌─────────▼───────────────────▼──────────────────────▼─────────────┐
│                      插件加载器 (Loader)                            │
│  ┌────────────────┐  ┌────────────────────────────────────────┐  │
│  │ 目录扫描加载     │  │ VsixLoader (ZIP 解压 + package.json)  │  │
│  │ (manifest.json) │  │                                        │  │
│  └────────┬───────┘  └──────────────────┬─────────────────────┘  │
└───────────┼───────────────────────────────┼────────────────────────┘
            │                               │
┌───────────▼───────────────────────────────▼────────────────────────┐
│                    统一元数据层 (PluginManifest)                      │
│  ┌─────────────┐  ┌──────────────┐  ┌────────┐  ┌────────────┐  │
│  │ VsCodeManifest│  │ 能力声明      │  │ 权限    │  │ 依赖声明    │  │
│  │ (package.json)│  │ (capabilities)│  │ (perms) │  │ (deps)     │  │
│  └─────────────┘  └──────────────┘  └────────┘  └────────────┘  │
└───────────────────────────────────┬─────────────────────────────────┘
                                    │
┌───────────────────────────────────▼─────────────────────────────────┐
│                   运行时注册表 (RuntimeRegistry)                       │
│  ┌─────────────────────┐  ┌──────────────────────────────────────┐  │
│  │    WasmRuntime       │  │        VsCodeRuntime                 │  │
│  │  (wasmer+cranelift)  │  │  (deno_core + vscode API shim)     │  │
│  │  WASM 沙箱执行        │  │  JS 执行 + VSCode API 兼容          │  │
│  └─────────────────────┘  └──────────────────────────────────────┘  │
└───────────────────────────────────┬─────────────────────────────────┘
                                    │
┌───────────────────────────────────▼─────────────────────────────────┐
│                    插件注册表 (PluginRegistry)                         │
│  ┌─────────────┐  ┌──────────────┐  ┌────────┐  ┌────────────┐  │
│  │ 实例管理      │  │ 状态机        │  │ 能力查找 │  │ 事件总线    │  │
│  │ (instances)  │  │ (lifecycle)  │  │ (lookup)│  │ (events)   │  │
│  └─────────────┘  └──────────────┘  └────────┘  └────────────┘  │
└───────────────────────────────────┬─────────────────────────────────┘
                                    │
┌───────────────────────────────────▼─────────────────────────────────┐
│                      宿主 API (Host API)                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐   │
│  │ AI Chat   │  │ Event Pub │  │ File Sys │  │  (可扩展)         │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

## 多运行时抽象

### Runtime trait

**模块**: `mox-plugin-core/src/runtime/mod.rs`

统一插件运行时接口，所有运行时（WASM、VSCode、未来的其他运行时）均实现此 trait：

```rust
#[async_trait]
pub trait Runtime: Send + Sync {
    /// 加载插件，返回运行时句柄
    async fn load(&self, manifest: &PluginManifest, entry: &Path) -> Result<RuntimeHandle>;
    /// 初始化插件（调用 on_load）
    async fn init(&self, handle: &RuntimeHandle) -> Result<()>;
    /// 启动插件（开始提供服务）
    async fn start(&self, handle: &RuntimeHandle) -> Result<()>;
    /// 停止插件（释放资源）
    async fn stop(&self, handle: &RuntimeHandle) -> Result<()>;
    /// 调用插件方法
    async fn call(&self, handle: &RuntimeHandle, method: &str, args: &Value) -> Result<Value>;
    /// 运行时类型标识
    fn runtime_type(&self) -> RuntimeType;
}
```

### RuntimeType

```rust
pub enum RuntimeType {
    Wasm,    // WASM 运行时（wasmer + cranelift）
    VsCode,  // VSCode 运行时（deno_core + vscode API shim）
}
```

### RuntimeHandle

运行时句柄，封装插件实例的运行时状态：

| 字段 | 类型 | 说明 |
|------|------|------|
| `runtime_type` | `RuntimeType` | 运行时类型 |
| `instance_id` | `String` | 实例唯一 ID（UUID v4） |
| `state` | `RwLock<PluginState>` | 当前生命周期状态 |
| `internal` | `RwLock<Option<RuntimeInternal>>` | 运行时内部数据（WASM 实例 / JS 运行时） |
| `manifest_id` | `String` | 关联的插件 manifest ID |

**方法**: `id()`, `state()`, `transition_to()`, `with_internal()`, `with_internal_mut()`

### RuntimeInternal

```rust
pub enum RuntimeInternal {
    Wasm(Option<wasmer::Instance>),  // WASM 模块实例
    VsCode,                             // VSCode 运行时占位（阶段 2 为 JsRuntime）
}
```

### RuntimeRegistry

线程安全的运行时注册表，管理所有可用运行时：

| 方法 | 说明 |
|------|------|
| `register(runtime: Arc<dyn Runtime>)` | 注册运行时（按 RuntimeType 去重） |
| `get(ty: RuntimeType) -> Option<Arc<dyn Runtime>>` | 按类型查找运行时 |
| `list_types() -> Vec<RuntimeType>` | 列出所有已注册运行时类型 |
| `has(ty: RuntimeType) -> bool` | 检查运行时是否已注册 |

### WasmRuntime

**模块**: `mox-plugin-core/src/runtime/wasm.rs`

封装现有 WASM 加载逻辑：

- **load**: 读取 WASM 文件 → wasmer 编译（cranelift）→ 实例化 → 返回 RuntimeHandle（状态=Loaded）
- **init**: 状态转换到 Initialized（阶段 1 简化，阶段 2 调用 WASM `init` 导出函数）
- **start**: 状态转换到 Running
- **stop**: 状态转换到 Stopped，释放 WASM 实例
- **call**: 调用 WASM 导出函数（阶段 1 简化实现）
- 所有 CPU 密集操作在 `tokio::task::spawn_blocking` 中执行

### VsCodeRuntime

**模块**: `mox-plugin-core/src/runtime/vscode.rs`

阶段 1 骨架实现：

- **load**: 验证 manifest 包含 `runtime:vscode` tag → 返回 RuntimeHandle（状态=Loaded），不实际加载 JS
- **init/start/stop**: 状态转换，返回 `Ok(())`
- **call**: 返回 `Err("VSCode runtime call not implemented in phase 1")`
- 预留 deno_core 集成点（注释标记 "阶段 2 实现"）
- 预留 vscode API 兼容层（注释标记 "阶段 2 实现"）

## 生命周期

### 状态机

**模块**: `mox-plugin-core/src/lifecycle.rs`

```text
          load()
  ┌──────────────────┐
  │                  ▼
Unloaded ──► Loaded ──► Initialized ──► Running
  ▲             │             │             │
  │             │             │             │
  │             ▼             ▼             ▼
  └──────── Unloaded ◄── Stopped ◄─── Paused
               unload()     stop()      pause()
```

| 状态 | 说明 |
|------|------|
| `Unloaded` | 未加载（磁盘上，未载入内存） |
| `Loaded` | 已加载（模块载入内存，未初始化） |
| `Initialized` | 已初始化（on_load 已调用，能力已注册） |
| `Running` | 运行中（正常提供服务） |
| `Paused` | 已暂停（暂停处理请求，但保留状态） |
| `Stopped` | 已停止（on_stop 已调用，释放资源） |
| `Error` | 错误状态（加载/初始化/运行出错） |

### 合法转换

| From → To | 说明 |
|-----------|------|
| Unloaded → Loaded | 加载插件 |
| Loaded → Initialized | 初始化插件 |
| Loaded → Unloaded | 卸载（未初始化） |
| Initialized → Running | 启动插件 |
| Initialized → Stopped | 直接停止 |
| Running → Paused | 暂停 |
| Running → Stopped | 停止 |
| Paused → Running | 恢复 |
| Paused → Stopped | 停止 |
| Stopped → Unloaded | 卸载 |
| 任意 → Error | 出错 |
| Error → Unloaded | 错误恢复后卸载 |

### 生命周期事件

```rust
pub struct LifecycleEvent {
    pub plugin_id: String,
    pub plugin_name: String,
    pub from: PluginState,
    pub to: PluginState,
    pub timestamp: i64,
    pub reason: Option<String>,
}
```

通过 `flume` 事件总线发布，外部可订阅监听插件状态变化。

## 权限模型

### PluginPermission

**模块**: `mox-plugin-core/src/manifest.rs`

12 项沙箱权限，使用 `file:read` 格式序列化：

| 权限 | as_str() | 说明 |
|------|----------|------|
| `FileRead` | `file:read` | 文件读取 |
| `FileWrite` | `file:write` | 文件写入 |
| `NetworkApi` | `network:api` | 网络 API 调用 |
| `NetworkServer` | `network:server` | 网络监听（服务器） |
| `AiChat` | `ai:chat` | AI 能力调用 |
| `Database` | `database` | 数据库访问 |
| `Cache` | `cache` | 缓存访问 |
| `EventPublish` | `event:publish` | 事件发布 |
| `EventSubscribe` | `event:subscribe` | 事件订阅 |
| `SystemCommand` | `system:command` | 系统命令执行（高危） |
| `EnvRead` | `env:read` | 环境变量读取 |

### 权限检查

**模块**: `mox-plugin-core/src/host_api.rs`

所有宿主 API 调用经过 `HostApiContext::require_permission()` 检查：

```rust
pub fn require_permission(&self, perm: PluginPermission) -> Result<(), HostApiError> {
    if !self.plugin.manifest.has_permission(perm) {
        return Err(HostApiError::PermissionDenied(perm.as_str().into()));
    }
    Ok(())
}
```

同时检查插件运行状态：`require_running()` 确保只有 Running 状态的插件可调用 API。

## 能力系统

### PluginCapability

```rust
pub struct PluginCapability {
    pub id: String,           // 能力唯一标识（如 "ocr.extract"）
    pub name: String,         // 能力名称
    pub description: String,  // 能力描述
    pub input_schema: Value,  // 输入参数 Schema（JSON Schema）
    pub output_schema: Value, // 输出参数 Schema
}
```

### VSCode 贡献点 → 能力映射

| VSCode 贡献点 | 能力 ID 格式 | 示例 |
|-------------|-------------|------|
| `commands` | `command.{command}` | `command.python.execInTerminal` |
| `keybindings` | `keybinding.{command}` | `keybinding.python.execInTerminal` |
| `languages` | `language.{id}` | `language.python` |
| `themes` | `theme.{label_snake}` | `theme.python_blue` |
| `snippets` | `snippet.{language}` | `snippet.python` |
| `views` | `view.{id}` | `view.pythonTestExplorer` |

### 能力查找

**模块**: `mox-plugin-core/src/registry.rs`

```rust
// 按能力 ID 查找运行中的插件
pub fn find_by_capability(&self, capability_id: &str) -> Vec<Arc<PluginInstance>>

// 按标签筛选
pub fn find_by_tag(&self, tag: &str) -> Vec<Arc<PluginInstance>>
```

## 插件目录结构

```text
plugins/
├── com.vendor.ocr/              # WASM 插件目录
│   ├── manifest.json             # 插件描述符（必需）
│   └── plugin.wasm               # WASM 模块（必需，路径在 manifest.entry 指定）
│
├── vscode.ms-python.python/      # VSCode 扩展目录（VSIX 解压后）
│   └── extension/
│       ├── package.json          # VSCode 扩展描述符
│       ├── out/extension.js      # JS 入口（main 字段指定）
│       ├── language-configuration.json
│       ├── snippets/python.json
│       └── themes/
│
└── vendor.extension.vsix         # VSIX 包（直接放置，加载时自动解压）
```

## 快速开始

```rust
use mox_plugin_core::prelude::*;
use std::sync::Arc;

// 1. 创建运行时注册表
let runtime_registry = Arc::new(RuntimeRegistry::new());
runtime_registry.register(Arc::new(WasmRuntime::new()));
runtime_registry.register(Arc::new(VsCodeRuntime::new()));

// 2. 创建插件注册表
let plugin_registry = Arc::new(PluginRegistry::new());

// 3. 创建加载器，指定插件目录
let loader = PluginLoader::new(plugin_registry.clone(), "./plugins");

// 4. 加载所有插件（WASM 目录 + VSIX 包）
let count = loader.load_all().await.unwrap();
println!("loaded {} plugins", count);

// 5. 初始化并启动插件
for plugin in plugin_registry.list() {
    plugin.transition_to(PluginState::Initialized).unwrap();
    plugin.transition_to(PluginState::Running).unwrap();
}

// 6. 按能力查找插件
let python_plugins = plugin_registry.find_by_capability("language.python");
```

## 模块索引

| 模块 | 文件 | 职责 |
|------|------|------|
| `manifest` | `src/manifest.rs` | 插件描述符 + VSCode package.json 解析 + 权限 + 依赖 + 能力 |
| `lifecycle` | `src/lifecycle.rs` | 生命周期状态机 + 事件 + 错误 |
| `registry` | `src/registry.rs` | 插件注册表 + 实例管理 + 能力查找 + 事件总线 |
| `loader` | `src/loader.rs` | 插件加载器 + 目录扫描 + WASM 加载 + VSIX 解压 + 热重载 |
| `host_api` | `src/host_api.rs` | 宿主 API + 权限检查 + AI 聊天 + 事件发布 |
| `runtime` | `src/runtime/mod.rs` | 多运行时抽象 + Runtime trait + RuntimeRegistry |
| `runtime::wasm` | `src/runtime/wasm.rs` | WasmRuntime 实现 |
| `runtime::vscode` | `src/runtime/vscode.rs` | VsCodeRuntime 实现（阶段 1 骨架） |
| `market` | `src/market/mod.rs` | 插件市场 + 远程发现 + 安装 + 版本管理 |
| `market::vsix` | `src/market/vsix.rs` | VSIX 市场支持 + 已安装列表 |

## 向后兼容

阶段 1 的所有变更保持对现有 WASM 插件系统的完全兼容：

- `PluginManifest` 结构体无破坏性变更（仅新增 `from_vscode` 构造函数）
- `PluginLoader::load_all()` 签名不变，内部增加 VSIX 扫描
- `PluginRegistry` API 完全不变
- `HostApi` trait 完全不变
- 现有 WASM 插件无需任何修改即可继续运行
- `PluginPermission` 序列化格式从 `snake_case` 修正为 `file:read` 格式（与 `as_str()` 一致，修复预存 bug）
