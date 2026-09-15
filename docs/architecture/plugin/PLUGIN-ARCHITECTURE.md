# 统一插件架构文档

> MOX 平台采用**双运行时混合架构**，同时支持原生 WASM 插件和 VSCode 兼容扩展，
> 通过统一的 `Runtime` trait 抽象实现生命周期、权限、能力系统的归一化管理。

## 架构层次

```
┌─────────────────────────────────────────────────────────────────┐
│                        应用层（Application）                      │
│  插件市场 │ 插件管理器 │ 能力调度器 │ 命令注册表 │ 事件总线      │
├─────────────────────────────────────────────────────────────────┤
│                      协议层（Protocol）                           │
│  mox-api-protocol │ mox-error │ ApiResponse<T> │ 分页 │ 错误码  │
├─────────────────────────────────────────────────────────────────┤
│                     插件核心层（Plugin Core）                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  Manifest    │  │  Lifecycle   │  │  Registry             │  │
│  │  (清单/权限) │  │  (状态机)    │  │  (实例管理/能力查找)  │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  Loader      │  │  Host API    │  │  Market               │  │
│  │  (加载/热重载)│  │  (宿主能力)   │  │  (安装/卸载/搜索)    │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                    运行时抽象层（Runtime Abstraction）            │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Runtime trait (load/init/start/stop/call/runtime_type) │   │
│  └──────────────────────────────────────────────────────────┘   │
│         │                    │                    │               │
│  ┌──────▼──────┐    ┌──────▼──────┐    ┌──────▼──────┐        │
│  │ WasmRuntime  │    │ VsCodeRuntime│    │  (未来扩展)  │        │
│  │ (wasmer+cranelift)│ │(deno_core+v8)│    │  Python/Lua  │        │
│  └─────────────┘    └─────────────┘    └─────────────┘        │
├─────────────────────────────────────────────────────────────────┤
│                      基础设施层（Infrastructure）                  │
│  tokio │ parking_lot │ serde │ serde_json │ anyhow │ uuid      │
└─────────────────────────────────────────────────────────────────┘
```

## 核心组件

### 1. Manifest（插件清单）

插件的元数据声明，定义插件的身份、权限、依赖、能力和配置。

```rust
pub struct PluginManifest {
    pub id: String,              // 插件唯一 ID（如 "com.vendor.ocr"）
    pub name: String,            // 显示名称
    pub version: String,         // 语义化版本（如 "1.2.3"）
    pub description: String,     // 描述
    pub author: String,          // 作者
    pub entry: String,           // 入口文件（plugin.wasm 或 extension.js）
    pub runtime: RuntimeType,    // 运行时类型（Wasm / VsCode）
    pub permissions: Vec<PluginPermission>,  // 权限声明
    pub dependencies: Vec<PluginDependency>, // 依赖声明
    pub capabilities: Vec<PluginCapability>, // 能力声明
    pub config: PluginConfig,    // 配置 Schema
    pub tags: Vec<String>,       // 标签（如 "runtime:vscode"）
}
```

#### 权限系统（12 项细粒度权限）

| 权限 | 说明 |
|------|------|
| `FileRead` | 文件读取 |
| `FileWrite` | 文件写入 |
| `NetworkApi` | 网络 API 调用 |
| `NetworkServer` | 网络监听（服务器） |
| `AiChat` | AI 能力调用 |
| `Database` | 数据库访问 |
| `Cache` | 缓存访问 |
| `EventPublish` | 事件发布 |
| `EventSubscribe` | 事件订阅 |
| `SystemCommand` | 系统命令执行（高危） |
| `EnvRead` | 环境变量读取 |

### 2. Lifecycle（生命周期状态机）

插件的生命周期管理，定义状态转换规则和事件通知。

```
┌──────────┐    load    ┌──────────┐   init    ┌──────────────┐
│ Unloaded │ ──────────▶ │  Loaded  │ ────────▶ │ Initialized  │
└──────────┘             └──────────┘           └──────┬───────┘
     ▲                       │                           │ start
     │                       │ stop/unload               ▼
     │                       │                    ┌──────────────┐
     │                       └───────────────────▶ │   Running    │
     │                                             └──────┬───────┘
     │                                                    │ pause
     │                                                    ▼
     │                                             ┌──────────────┐
     │                                             │   Paused     │
     │                                             └──────┬───────┘
     │                                                    │ resume/stop
     │                                                    ▼
     │                                             ┌──────────────┐
     └───────────────────────────────────────────── │   Stopped    │
                                                   └──────────────┘
```

#### 状态转换规则

| 当前状态 | 允许转换到 |
|---------|-----------|
| Unloaded | Loaded |
| Loaded | Initialized, Unloaded, Stopped |
| Initialized | Running, Stopped |
| Running | Paused, Stopped |
| Paused | Running, Stopped |
| Stopped | Unloaded |
| 任意状态 | Error |
| Error | Unloaded |

#### 生命周期事件

```rust
pub enum LifecycleEvent {
    Loaded { id: String, timestamp: u64 },
    Initialized { id: String, timestamp: u64 },
    Started { id: String, timestamp: u64 },
    Paused { id: String, timestamp: u64 },
    Resumed { id: String, timestamp: u64 },
    Stopped { id: String, timestamp: u64 },
    Unloaded { id: String, timestamp: u64 },
    Error { id: String, error: String, timestamp: u64 },
}
```

### 3. Registry（插件注册表）

管理所有已加载的插件实例，提供实例管理、状态查询和能力查找。

```rust
pub struct PluginRegistry {
    instances: RwLock<HashMap<String, PluginInstance>>,
}

impl PluginRegistry {
    pub fn new() -> Self;
    pub fn register(&self, instance: PluginInstance);
    pub fn unregister(&self, id: &str) -> Option<PluginInstance>;
    pub fn get(&self, id: &str) -> Option<PluginInstance>;
    pub fn list(&self) -> Vec<PluginInstance>;
    pub fn find_by_capability(&self, capability: &str) -> Vec<PluginInstance>;
    pub fn find_by_tag(&self, tag: &str) -> Vec<PluginInstance>;
    pub fn count(&self) -> usize;
}
```

### 4. Loader（插件加载器）

负责插件的目录扫描、文件加载、WASM/VSIX 解析和热重载。

```rust
pub struct PluginLoader {
    registry: Arc<PluginRegistry>,
    plugin_dir: PathBuf,
}

impl PluginLoader {
    pub fn new(registry: Arc<PluginRegistry>, plugin_dir: impl Into<PathBuf>) -> Self;
    pub async fn load_all(&self) -> Result<usize>;
    pub async fn load_one(&self, path: &Path) -> Result<PluginInstance>;
    pub async fn reload(&self, id: &str) -> Result<()>;
    pub async fn unload(&self, id: &str) -> Result<()>;
}
```

#### VSIX 加载器

```rust
pub struct VsixLoader;

impl VsixLoader {
    pub fn is_vsix(path: &Path) -> bool;
    pub fn load_vsix(path: &Path) -> Result<PluginManifest>;
    pub fn extract_vsix(vsix_path: &Path, dest_dir: &Path) -> Result<()>;
}
```

### 5. Host API（宿主能力）

插件可调用的平台能力接口，所有调用经过权限检查。

```rust
pub trait HostApi {
    // 文件系统
    fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()>;

    // 网络
    fn http_request(&self, url: &str, method: &str, body: Option<&[u8]>) -> Result<Vec<u8>>;

    // AI
    fn ai_chat(&self, prompt: &str) -> Result<String>;

    // 数据库
    fn db_query(&self, sql: &str) -> Result<Vec<Value>>;

    // 缓存
    fn cache_get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    fn cache_set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<()>;

    // 事件
    fn event_publish(&self, topic: &str, payload: &Value) -> Result<()>;
    fn event_subscribe(&self, topic: &str, handler: EventHandler) -> Result<Subscription>;

    // 日志
    fn log(&self, level: LogLevel, message: &str);
}
```

### 6. Runtime（运行时抽象）

统一的插件运行时接口，所有运行时（WASM/VSCode/未来扩展）都实现此 trait。

```rust
#[async_trait]
pub trait Runtime: Send + Sync {
    /// 加载插件
    async fn load(&self, manifest: &PluginManifest, entry: &Path) -> Result<RuntimeHandle>;

    /// 初始化插件（调用 activate）
    async fn init(&self, handle: &RuntimeHandle) -> Result<()>;

    /// 启动插件
    async fn start(&self, handle: &RuntimeHandle) -> Result<()>;

    /// 停止插件（调用 deactivate，清理资源）
    async fn stop(&self, handle: &RuntimeHandle) -> Result<()>;

    /// 调用插件方法或宿主命令
    async fn call(&self, handle: &RuntimeHandle, method: &str, args: &Value) -> Result<Value>;

    /// 运行时类型
    fn runtime_type(&self) -> RuntimeType;
}

pub enum RuntimeType {
    Wasm,
    VsCode,
}
```

#### RuntimeHandle（运行时句柄）

```rust
pub struct RuntimeHandle {
    runtime_type: RuntimeType,
    instance_id: String,           // 运行时实例 ID（UUID）
    state: RwLock<PluginState>,    // 生命周期状态
    internal: RwLock<Option<RuntimeInternal>>, // 运行时内部数据
    manifest_id: String,            // 关联的插件 manifest ID
}

pub enum RuntimeInternal {
    Wasm(Arc<Mutex<WasmInstance>>),
    VsCode(String), // instance_id，查找 VsCodeRuntime.runtimes 表
}
```

#### RuntimeRegistry（运行时注册表）

```rust
pub struct RuntimeRegistry {
    runtimes: RwLock<HashMap<RuntimeType, Arc<dyn Runtime>>>,
}

impl RuntimeRegistry {
    pub fn new() -> Self;
    pub fn register(&self, runtime: Arc<dyn Runtime>);
    pub fn get(&self, runtime_type: RuntimeType) -> Option<Arc<dyn Runtime>>;
    pub fn list(&self) -> Vec<Arc<dyn Runtime>>;
}
```

## 运行时实现

### WasmRuntime（WASM 运行时）

基于 `wasmer` + `cranelift` 的高性能 WASM 运行时。

- **性能**：AOT 编译，接近原生速度
- **安全**：内存沙箱，无法直接访问系统资源
- **语言支持**：Rust/C/C++/AssemblyScript/Go/TinyGo 等可编译为 WASM 的语言
- **适用场景**：性能敏感的算法插件、计算密集型任务

### VsCodeRuntime（VSCode 兼容运行时）

基于 `deno_core` + `v8` 的 JavaScript 运行时，实现 VSCode Extension API 核心子集。

- **生态兼容**：支持 VSCode 扩展（VSIX 包）
- **API 覆盖**：commands/window/workspace/extensions + 12 个基础类
- **安全沙箱**：deno_core 默认禁用文件系统/网络，通过宿主 ops 间接访问
- **适用场景**：UI 类插件、命令类插件、工作流自动化

#### VsCodeRuntime 架构

```
┌─────────────────────────────────────────────────────┐
│                  VsCodeRuntime                       │
│  ┌───────────────────────────────────────────────┐  │
│  │  runtimes: Mutex<HashMap<String, DenoRuntime>>│  │
│  └───────────────────────────────────────────────┘  │
│         │                                             │
│         ▼                                             │
│  ┌───────────────────────────────────────────────┐  │
│  │              DenoRuntime (per instance)        │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  deno_core::JsRuntime (v8 Isolate)      │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Host Extension (21 ops)                │  │  │
│  │  │  ├── op_show_information_message         │  │  │
│  │  │  ├── op_show_input_box                   │  │  │
│  │  │  ├── op_create_output_channel            │  │  │
│  │  │  ├── op_register_command                  │  │  │
│  │  │  ├── op_execute_command                   │  │  │
│  │  │  ├── op_get_workspace_folders            │  │  │
│  │  │  ├── op_open_text_document               │  │  │
│  │  │  ├── op_get_configuration                │  │  │
│  │  │  ├── op_get_extension                    │  │  │
│  │  │  └── ... (13 more)                       │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  VSCode API Shim (~870 lines JS)        │  │  │
│  │  │  ├── vscode.commands                     │  │  │
│  │  │  ├── vscode.window                       │  │  │
│  │  │  ├── vscode.workspace                    │  │  │
│  │  │  ├── vscode.extensions                   │  │  │
│  │  │  └── Base Classes (12)                   │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## 插件目录结构

### WASM 插件
```
plugins/
└── com.vendor.ocr/
    ├── manifest.json      # 插件描述符（必需）
    └── plugin.wasm        # WASM 模块（必需）
```

### VSCode 扩展
```
plugins/
└── publisher.extension-name/
    ├── package.json       # VSCode 扩展清单（必需）
    ├── extension.js       # 入口 JS（必需，main 字段指定）
    ├── README.md          # 说明文档（可选）
    └── assets/            # 静态资源（可选）
```

## 能力系统

插件通过 `capabilities` 声明提供的能力，宿主通过能力查找调度插件。

```rust
pub struct PluginCapability {
    pub id: String,          // 能力 ID（如 "ocr.extract"）
    pub name: String,        // 显示名称
    pub description: String, // 描述
    pub version: String,     // 能力版本
    pub input_schema: Value, // 输入参数 JSON Schema
    pub output_schema: Value,// 输出结果 JSON Schema
}
```

### 能力查找示例

```rust
// 查找所有提供 OCR 能力的插件
let ocr_plugins = registry.find_by_capability("ocr.extract");

// 调用第一个匹配的插件
if let Some(plugin) = ocr_plugins.first() {
    let result = plugin.call("ocr.extract", &json!({"image": "..."}))?;
}
```

## 配置系统

插件通过 `config` 声明配置项，宿主提供统一的配置管理。

```rust
pub struct PluginConfig {
    pub fields: Vec<ConfigField>,
}

pub struct ConfigField {
    pub name: String,
    pub field_type: String,   // string/number/boolean/object
    pub required: bool,
    pub default: Option<Value>,
    pub description: String,
}
```

## 事件系统

插件通过事件总线与宿主和其他插件通信。

```rust
// 发布事件
host.event_publish("file.created", &json!({"path": "/tmp/test.txt"}))?;

// 订阅事件
let subscription = host.event_subscribe("file.created", |event| {
    println!("File created: {:?}", event.payload);
})?;
```

## 热重载

插件支持运行时热重载，无需重启宿主。

```rust
// 重新加载插件
loader.reload("com.vendor.ocr").await?;

// 监听文件变化自动重载（阶段 3 规划）
loader.watch().await?;
```

## 安全模型

### 权限检查
- 所有宿主 API 调用经过权限检查
- 插件只能访问声明的权限
- 高危权限（SystemCommand/NetworkServer）需要用户显式授权

### 沙箱隔离
- WASM 插件：内存沙箱，无法直接访问系统资源
- VSCode 扩展：deno_core 沙箱，默认禁用文件系统/网络
- 每个插件独立的运行时实例，互不干扰

### 资源限制
- 内存限制（阶段 3 规划）
- CPU 时间限制（阶段 3 规划）
- 调用频率限制（阶段 3 规划）

## 相关文档

- [VSCODE-COMPATIBILITY.md](./VSCODE-COMPATIBILITY.md) - VSCode 插件兼容性文档
- [VSCODE-API-STATUS.md](./VSCODE-API-STATUS.md) - VSCode API 实现状态表

---

*最后更新：2026-09-03 | 阶段 2 完成*
