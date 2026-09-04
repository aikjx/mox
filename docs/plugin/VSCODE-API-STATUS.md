# VSCode API 实现状态表

> 本文档记录 MOX 插件运行时对 VSCode Extension API 的实现状态。
> 阶段 2 已实现核心子集，阶段 3 将持续完善。

## 实现状态图例

- ✅ 已实现（完整功能）
- 🟡 部分实现（基础功能，高级特性待完善）
- ⏳ 规划中（阶段 3 实现）
- ❌ 未实现（暂无计划）

## 核心模块

### commands（命令系统）

| API | 状态 | 说明 |
|-----|------|------|
| `registerCommand(id, handler)` | ✅ | 注册命令，返回 Disposable |
| `registerTextEditorCommand(id, handler)` | ✅ | 注册文本编辑器命令 |
| `executeCommand(id, ...args)` | ✅ | 执行已注册命令，返回 Promise |
| `getCommands(filterInternal)` | ✅ | 获取所有已注册命令列表 |

### window（窗口与 UI）

| API | 状态 | 说明 |
|-----|------|------|
| `showInformationMessage(msg, ...items)` | ✅ | 显示信息消息框，返回 Promise |
| `showWarningMessage(msg, ...items)` | ✅ | 显示警告消息框 |
| `showErrorMessage(msg, ...items)` | ✅ | 显示错误消息框 |
| `showInputBox(options?)` | ✅ | 显示输入框，返回 Promise<string\|undefined> |
| `showQuickPick(items, options?)` | ✅ | 显示快速选择列表 |
| `createOutputChannel(name)` | ✅ | 创建输出通道，支持 append/show |
| `showTextDocument(document)` | 🟡 | 显示文本文档（基础实现） |
| `onDidChangeActiveTextEditor` | ✅ | 活动编辑器变更事件 |
| `activeTextEditor` | 🟡 | 当前活动编辑器（基础实现） |

### workspace（工作区）

| API | 状态 | 说明 |
|-----|------|------|
| `workspaceFolders` | ✅ | 工作区文件夹列表（只读） |
| `workspaceFile` | ✅ | 工作区配置文件 URI |
| `openTextDocument(uri)` | ✅ | 打开文本文档，返回 Promise<TextDocument> |
| `getConfiguration(section?)` | ✅ | 获取工作区配置 |
| `onDidChangeConfiguration` | ✅ | 配置变更事件 |
| `textDocuments` | ✅ | 所有已打开文本文档 |
| `onDidOpenTextDocument` | 🟡 | 文档打开事件（基础实现） |
| `onDidSaveTextDocument` | 🟡 | 文档保存事件（基础实现） |

### extensions（扩展管理）

| API | 状态 | 说明 |
|-----|------|------|
| `getExtension(id)` | ✅ | 获取指定扩展元数据 |
| `all` | ✅ | 所有已安装扩展列表 |
| `onDidChangeExtensions` | ✅ | 扩展变更事件 |

## 基础类

| 类 | 状态 | 说明 |
|----|------|------|
| `Disposable` | ✅ | 可释放资源基类 |
| `EventEmitter<T>` | ✅ | 事件发射器 |
| `Event<T>` | ✅ | 事件类型 |
| `Uri` | ✅ | 统一资源标识符（scheme/path/fsPath） |
| `Position` | ✅ | 文本位置（line/character） |
| `Range` | ✅ | 文本范围（start/end） |
| `Selection` | ✅ | 文本选择（继承 Range） |
| `TextDocument` | ✅ | 文本文档（uri/languageId/getText/lineCount） |
| `OutputChannel` | ✅ | 输出通道（append/show/dispose） |
| `Extension<T>` | ✅ | 扩展元数据（id/version/path/packageJSON） |
| `WorkspaceFolder` | ✅ | 工作区文件夹（uri/name/index） |

## 阶段 3 规划（未实现）

### languages（语言服务）⏳

- `registerCompletionItemProvider`
- `registerHoverProvider`
- `registerDefinitionProvider`
- `registerReferenceProvider`
- `registerCodeActionProvider`
- `registerDocumentFormattingEditProvider`
- `createDiagnosticCollection`
- `setLanguageConfiguration`

### debug（调试器）⏳

- `registerDebugConfigurationProvider`
- `startDebugging`
- `onDidStartDebugSession`
- `onDidTerminateDebugSession`
- `DebugAdapterDescriptorFactory`

### scm（源代码管理）⏳

- `createSourceControl`
- `SourceControlInputBox`
- `SourceControlResourceGroup`

### tasks（任务系统）⏳

- `registerTaskProvider`
- `fetchTasks`
- `executeTask`
- `onDidStartTask`
- `onDidEndTask`

### comments（评论）⏳

- `createCommentController`
- `CommentThread`
- `Comment`

### env（环境）⏳

- `machineId`
- `sessionId`
- `clipboard`
- `openExternal`
- `asExternalUri`

### authentication（认证）⏳

- `getSession`
- `onDidChangeSessions`
- `AuthenticationProvider`

### WebView ⏳

- `createWebviewPanel`
- `Webview`
- `WebviewPanel`
- `WebviewViewProvider`

## 贡献点映射

| VSCode 贡献点 | MOX capability 类型 | 状态 |
|---------------|---------------------|------|
| `contributes.commands` | `command` | ✅ |
| `contributes.keybindings` | `keybinding` | ✅ |
| `contributes.languages` | `language` | ✅ |
| `contributes.themes` | `theme` | ✅ |
| `contributes.snippets` | `snippet` | ✅ |
| `contributes.views` | `view` | ✅ |
| `contributes.viewContainers` | `view_container` | ✅ |
| `contributes.menus` | `menu` | ✅ |
| `contributes.configuration` | `configuration` | 🟡 |
| `contributes.debuggers` | `debugger` | ⏳ |
| `contributes.problemPatterns` | `problem_pattern` | ⏳ |

## 激活事件映射

| VSCode 激活事件 | MOX 处理 | 状态 |
|-----------------|----------|------|
| `onCommand:id` | 命令执行时激活 | ✅ |
| `onLanguage:lang` | 打开对应语言文件时激活 | 🟡 |
| `onWorkspaceContains:glob` | 工作区包含匹配文件时激活 | 🟡 |
| `onStartupFinished` | 启动完成后激活 | ✅ |
| `onDebug` | 调试启动时激活 | ⏳ |
| `onFileSystem:scheme` | 访问对应文件系统时激活 | ⏳ |
| `onView:id` | 视图显示时激活 | ⏳ |
| `onUri:scheme` | URI 处理时激活 | ⏳ |
| `onWalkthrough:id` | 向导显示时激活 | ⏳ |
| `*` | 启动时立即激活 | ✅ |

---

*最后更新：2026-09-03 | 阶段 2 完成*
