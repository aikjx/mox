# VSCode 插件兼容性文档

> 方案 C（双运行时混合架构）— 阶段 1 完成内容与阶段 2/3 规划

## 概述

MOX 插件系统通过**元数据兼容层**和**多运行时抽象**，实现对 VSCode 扩展（VSIX）的兼容支持。阶段 1 完成元数据解析、VSIX 加载和运行时骨架；阶段 2 将集成 deno_core 实现 JS 执行；阶段 3 完善 vscode API 兼容层和激活事件调度。

## 阶段 1 完成内容

### 1. VSCode package.json 元数据解析

**模块**: `mox-plugin-core/src/manifest.rs`

新增 `VsCodeManifest` 结构体，完整解析 VSCode 扩展的 `package.json`：

| 字段类别 | 字段 | 说明 |
|---------|------|------|
| 基础 | `name`, `version`, `displayName`, `description`, `publisher` | 扩展标识与描述 |
| 引擎 | `engines` | VSCode 版本要求（如 `{"vscode": "^1.74.0"}`） |
| 分类 | `categories`, `keywords` | 市场分类与搜索关键词 |
| 入口 | `main` | JS 入口文件路径 |
| 激活 | `activationEvents` | 激活事件列表 |
| 权限 | `enabledApiProposals` | 启用的 API 提案（映射到权限） |
| 贡献点 | `contributes` | 所有贡献点集合 |

**贡献点支持**:

| 贡献点 | 类型 | 说明 |
|--------|------|------|
| `commands` | `Vec<VsCodeCommand>` | 命令注册（command/title/category/icon） |
| `menus` | `HashMap<String, Vec<VsCodeMenu>>` | 菜单注册（位置→菜单项） |
| `keybindings` | `Vec<VsCodeKeybinding>` | 快捷键绑定 |
| `languages` | `Vec<VsCodeLanguage>` | 语言支持（id/aliases/extensions/configuration） |
| `themes` | `Vec<VsCodeTheme>` | 主题（label/uiTheme/path） |
| `snippets` | `Vec<VsCodeSnippet>` | 代码片段（language/path） |
| `views` | `HashMap<String, Vec<VsCodeView>>` | 视图（容器ID→视图列表） |
| `viewContainers` | `Vec<VsCodeViewContainer>` | 视图容器 |

### 2. VSCode → MOX 元数据转换

**方法**: `VsCodeManifest::to_mox_manifest() -> PluginManifest`

转换规则：

| VSCode 字段 | MOX 字段 | 转换逻辑 |
|-------------|----------|----------|
| `publisher` + `name` | `id` | `vscode.{publisher}.{name}` |
| `displayName` / `name` | `name` | 优先 displayName，回退 name |
| `version` | `version` | 直接映射 |
| `publisher` | `author` | 直接映射 |
| `description` | `description` | 直接映射 |
| `main` | `entry` | JS 入口文件，默认 `extension.js` |
| `contributes.commands` | `capabilities` | 能力 ID: `command.{command}` |
| `contributes.keybindings` | `capabilities` | 能力 ID: `keybinding.{command}` |
| `contributes.languages` | `capabilities` | 能力 ID: `language.{id}` |
| `contributes.themes` | `capabilities` | 能力 ID: `theme.{label_snake}` |
| `contributes.snippets` | `capabilities` | 能力 ID: `snippet.{language}` |
| `contributes.views` | `capabilities` | 能力 ID: `view.{id}` |
| `enabledApiProposals` | `permissions` | 见权限映射表 |
| `activationEvents` | `tags` | `activate:{event}` |
| `categories` | `tags` | `category:{lowercase}` |
| `keywords` | `tags` | 直接映射 |
| — | `tags` | 固定添加 `runtime:vscode` |

**便捷构造函数**: `PluginManifest::from_vscode(package_json: &str) -> Result<Self>`

### 3. 权限映射表

| VSCode API Proposal | MOX PluginPermission |
|---------------------|---------------------|
| `fileSearchProvider`, `textSearchProvider` | `FileRead` |
| `externalUriOpener`, `contributesViewsWelcome` | `NetworkApi` |
| `terminalDataWriteEvent`, `terminalDimensions`, `terminalSelection` | `SystemCommand` |
| `envVariableCollection` | `EnvRead` |
| `chatAgents`, `chatParticipant`, `languageModelAccess` | `AiChat` |
| 其他未识别提案 | （忽略） |

### 4. VSIX 包加载

**模块**: `mox-plugin-core/src/loader.rs`

新增 `VsixLoader`：

| 方法 | 说明 |
|------|------|
| `load_vsix(path: &Path) -> Result<PluginManifest>` | 解压 VSIX（ZIP），读取 `extension/package.json`，转换为 MOX manifest |
| `load_vsix_from_reader<R: Read + Seek>(reader: R) -> Result<PluginManifest>` | 泛型版本，支持内存 `Cursor` 测试 |
| `extract_vsix(vsix_path: &Path, dest_dir: &Path) -> Result<()>` | 解压 VSIX 到目标目录 |
| `is_vsix(path: &Path) -> bool` | 检查 `.vsix` 扩展名（大小写不敏感） |

**PluginLoader::load_all() 扩展**: 同时扫描目录插件（`manifest.json` + `.wasm`）和 `.vsix` 包文件。

### 5. VSIX 市场支持

**模块**: `mox-plugin-core/src/market/vsix.rs`

新增 `VsixPackageInfo` 和 `VsixMarketplace`：

| 方法 | 阶段 1 状态 | 说明 |
|------|------------|------|
| `search_vsix(query: &str)` | 骨架（返回空列表） | 阶段 2 对接 Open VSX Registry API |
| `install_vsix(package_id, version, dest_dir)` | 骨架（返回未实现错误） | 阶段 2 实现下载+解压+注册 |
| `list_installed(plugin_dir: &Path)` | 已实现 | 扫描 WASM 目录插件 + VSCode 扩展目录，合并返回 |

## 阶段 2 规划

### 2.1 VsCodeRuntime deno_core 集成

- 集成 `deno_core` 创建 JS 运行时环境
- 加载 VSCode 扩展的 `main` 入口 JS 文件
- 实现模块解析（支持 `require` / `import`）
- 提供 Node.js 兼容 API 子集（`fs`, `path`, `os` 等）

### 2.2 VSCode API 兼容层

实现 `vscode` namespace 的核心 API：

| API 类别 | 计划实现 |
|---------|---------|
| 命令 | `commands.registerCommand`, `commands.executeCommand` |
| 窗口 | `window.showInformationMessage`, `window.createOutputChannel` |
| 工作区 | `workspace.getConfiguration`, `workspace.findFiles` |
| 文档 | `workspace.openTextDocument`, `window.showTextDocument` |
| 语言 | `languages.registerCompletionItemProvider` |
| 事件 | `workspace.onDidChangeConfiguration`, `window.onDidChangeActiveTextEditor` |
| 扩展上下文 | `ExtensionContext`（subscriptions, globalState, workspaceState） |

### 2.3 激活事件调度

实现 VSCode 激活事件的自动调度：

| 激活事件 | 调度时机 |
|---------|---------|
| `onCommand:{id}` | 命令首次执行时 |
| `onLanguage:{id}` | 打开对应语言文件时 |
| `onWorkspaceContains:{glob}` | 工作区包含匹配文件时 |
| `onStartupFinished` | 平台启动完成后 |
| `onView:{id}` | 视图首次展开时 |
| `onUri` | URI 处理时 |

### 2.4 真实市场 API 对接

- 对接 [Open VSX Registry](https://open-vsx.org/) API
- 实现搜索、详情、下载、版本管理
- 支持 VSIX 包的 SHA256 校验

## 阶段 3 规划

### 3.1 完整 vscode API 覆盖

- 覆盖 VSCode API 的 80%+ 常用接口
- 支持 Webview、TreeView、Custom Editor 等复杂 UI 组件
- 实现 Debug Adapter Protocol 集成

### 3.2 性能优化

- JS 运行时快照（snapshot）加速启动
- 扩展隔离（每个扩展独立 JsRuntime）
- 惰性激活（按需加载，减少启动时间）

### 3.3 开发者工具

- VSCode 扩展开发文档
- MOX 平台 API 差异说明
- 扩展迁移工具（自动检测不兼容 API）

## 已知限制（阶段 1）

1. **无 JS 执行**: VsCodeRuntime 仅为骨架，`call()` 返回未实现错误
2. **无激活调度**: 激活事件仅存储在 tags 中，不自动调度
3. **市场骨架**: `search_vsix` 返回空列表，`install_vsix` 未实现
4. **贡献点部分支持**: `menus` 解析但不转换为 capabilities（菜单需要 UI 集成）
5. **权限映射有限**: 仅映射常见 API 提案，其他提案忽略

## 参考资料

- [VSCode Extension Manifest](https://code.visualstudio.com/api/references/extension-manifest)
- [VSCode Contribution Points](https://code.visualstudio.com/api/references/contribution-points)
- [VSCode Activation Events](https://code.visualstudio.com/api/references/activation-events)
- [Open VSX Registry API](https://open-vsx.org/swagger-ui)
- [deno_core](https://docs.rs/deno-core/)
