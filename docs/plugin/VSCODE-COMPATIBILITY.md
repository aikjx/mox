# VSCode 插件兼容性文档

> MOX 平台支持运行 VSCode 扩展（VSIX 包），采用双运行时混合架构：
> - **WASM 运行时**：高性能、安全沙箱，用于原生 MOX 插件
> - **VSCode 兼容运行时**：基于 deno_core 的 JS 运行时，实现 VSCode Extension API 核心子集

## 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                     MOX Plugin System                        │
├─────────────────────────────────────────────────────────────┤
│  PluginRegistry（插件注册表）                                 │
│  ├── WASM 插件（.wasm + manifest.json）                      │
│  └── VSCode 扩展（.vsix + package.json）                     │
├─────────────────────────────────────────────────────────────┤
│  Runtime Abstraction（运行时抽象）                            │
│  ├── WasmRuntime（wasmer + cranelift）                      │
│  └── VsCodeRuntime（deno_core + v8）                        │
│      ├── DenoRuntime（JsRuntime 封装）                       │
│      ├── Host Ops（21 个 Rust 宿主函数）                     │
│      └── VSCode API Shim（~870 行 JS）                      │
└─────────────────────────────────────────────────────────────┘
```

## 阶段 1：元数据兼容（已完成）

### 功能
- ✅ VSCode `package.json` 解析（`VsCodeManifest`）
- ✅ 贡献点映射（commands/keybindings/languages/themes/snippets/views/menus → MOX capabilities）
- ✅ 激活事件解析（`ActivationEvent` 枚举，10 种）
- ✅ VSIX 包解压加载（`VsixLoader`，基于 ZIP）
- ✅ 多运行时抽象（`Runtime` trait + `RuntimeType` + `RuntimeHandle` + `RuntimeRegistry`）
- ✅ 统一插件市场（`VsixMarketplace`，支持本地安装/卸载）

### 关键类型
```rust
// VSCode 扩展清单
pub struct VsCodeManifest {
    pub name: String,
    pub version: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub engines: Option<Value>,
    pub categories: Option<Vec<String>>,
    pub contributes: Option<VsCodeContributes>,
    pub activation_events: Option<Vec<String>>,
    pub main: Option<String>,
    pub enabled_api_proposals: Option<Vec<String>>,
}

// 转换为 MOX 插件清单
impl VsCodeManifest {
    pub fn to_mox_manifest(&self) -> PluginManifest;
}
```

## 阶段 2：运行时兼容（已完成）

### 功能
- ✅ deno_core JS 运行时集成（`DenoRuntime`）
- ✅ VSCode Extension API 核心子集（~870 行 JS shim）
  - `vscode.commands`（4 API）
  - `vscode.window`（8 API）
  - `vscode.workspace`（6 API）
  - `vscode.extensions`（3 API）
  - 基础类（Disposable/EventEmitter/Uri/Position/Range/Selection/TextDocument 等 12 类）
- ✅ 21 个 Rust 宿主 ops（UI/输出通道/命令/工作区/扩展）
- ✅ VsCodeRuntime 完整生命周期（load/init/start/stop/call）
- ✅ 激活事件触发（onCommand/onStartupFinished/*）
- ✅ 简单 VSCode 插件可运行（命令类、UI 类）

### VSCode API 实现状态
详见 [VSCODE-API-STATUS.md](./VSCODE-API-STATUS.md)

### 已实现的核心 API

#### commands
```javascript
const vscode = require('vscode');

// 注册命令
const disposable = vscode.commands.registerCommand('myext.hello', () => {
    vscode.window.showInformationMessage('Hello from MOX!');
});

// 执行命令
await vscode.commands.executeCommand('myext.hello');

// 获取所有命令
const commands = await vscode.commands.getCommands();
```

#### window
```javascript
// 消息框
const result = await vscode.window.showInformationMessage('确认操作？', '是', '否');

// 输入框
const name = await vscode.window.showInputBox({ prompt: '请输入名称' });

// 快速选择
const choice = await vscode.window.showQuickPick(['选项A', '选项B', '选项C']);

// 输出通道
const channel = vscode.window.createOutputChannel('MyExt');
channel.appendLine('日志信息');
channel.show();
```

#### workspace
```javascript
// 工作区文件夹
const folders = vscode.workspace.workspaceFolders;

// 打开文档
const doc = await vscode.workspace.openTextDocument(vscode.Uri.file('/path/to/file.js'));

// 获取配置
const config = vscode.workspace.getConfiguration('myext');
const value = config.get('key');
```

### 简单 VSCode 插件示例

#### package.json
```json
{
    "name": "hello-mox",
    "version": "1.0.0",
    "displayName": "Hello MOX",
    "description": "一个简单的 VSCode 扩展示例",
    "publisher": "mox",
    "engines": { "vscode": "^1.80.0" },
    "categories": ["Other"],
    "activationEvents": ["onCommand:hello-mox.sayHello"],
    "main": "./extension.js",
    "contributes": {
        "commands": [
            {
                "command": "hello-mox.sayHello",
                "title": "Hello MOX: Say Hello"
            }
        ]
    }
}
```

#### extension.js
```javascript
const vscode = require('vscode');

function activate(context) {
    console.log('Extension "hello-mox" is now active!');

    let disposable = vscode.commands.registerCommand('hello-mox.sayHello', () => {
        vscode.window.showInformationMessage('Hello from MOX Plugin System!');
    });

    context.subscriptions.push(disposable);
}

function deactivate() {
    console.log('Extension "hello-mox" is now deactivated!');
}

module.exports = { activate, deactivate };
```

#### 打包为 VSIX
```bash
# 使用 vsce 工具打包
npm install -g @vscode/vsce
vsce package
# 生成 hello-mox-1.0.0.vsix
```

#### 在 MOX 中安装
```rust
use mox_plugin_core::prelude::*;

let marketplace = VsixMarketplace::new("./plugins");
let manifest = marketplace.install_vsix_from_file("hello-mox-1.0.0.vsix").await?;

let runtime = VsCodeRuntime::new();
let handle = runtime.load(&manifest, Path::new("./plugins/hello-mox")).await?;
runtime.init(&handle).await?;
runtime.start(&handle).await?;

// 执行命令
runtime.call(&handle, "executeCommand", &json!(["hello-mox.sayHello"])).await?;
```

## 阶段 3：深度兼容（规划中）

### 计划功能
- ⏳ 语言服务 API（补全/悬停/定义/引用/代码操作/格式化）
- ⏳ 调试器 API（DAP 协议支持）
- ⏳ 源代码管理 API（Git 集成）
- ⏳ 任务系统 API（TaskProvider）
- ⏳ WebView API（HTML 界面嵌入）
- ⏳ 评论 API（CommentController）
- ⏳ 环境 API（clipboard/openExternal/machineId）
- ⏳ 认证 API（AuthenticationProvider）
- ⏳ 对接真实 VSCode Marketplace API（在线搜索/安装/更新）
- ⏳ 性能优化（Worker 支持、模块加载器）
- ⏳ 调试器支持（DAP 协议）

### 优先级
1. **语言服务 API**（最常用，影响大量扩展）
2. **WebView API**（复杂扩展的界面需求）
3. **调试器 API**（开发类扩展）
4. **真实 Marketplace 对接**（生态扩展）
5. **其他 API**（按需实现）

## 限制与注意事项

### 安全沙箱
- deno_core 默认禁用文件系统和网络访问
- 插件只能通过显式注册的宿主 ops 间接访问系统资源
- 所有宿主调用经过权限检查（基于 `PluginPermission`）

### 兼容性
- 仅支持 VSCode Extension API 的**核心子集**（阶段 2）
- 使用未实现 API 的扩展会收到 `not implemented in MOX runtime` 错误
- 建议扩展开发者检查 `VSCODE-API-STATUS.md` 确认 API 可用性
- MOX 运行时不支持 Node.js 原生模块（如 `fs`、`path`、`http`），需通过 vscode API 间接访问

### 性能
- 每个扩展独立的 v8 Isolate，内存开销约 10-30MB
- deno_core 冷启动时间约 50-100ms
- 建议同时运行的扩展数量不超过 20 个

## 相关文档

- [PLUGIN-ARCHITECTURE.md](./PLUGIN-ARCHITECTURE.md) - 统一插件架构文档
- [VSCODE-API-STATUS.md](./VSCODE-API-STATUS.md) - VSCode API 实现状态表

---

*最后更新：2026-09-03 | 阶段 2 完成*
