/**
 * MOX VSCode API Shim — 在 deno_core JS 运行时中模拟 VSCode 扩展 API
 *
 * 此文件在插件入口 JS 执行之前加载，通过 globalThis.vscode 暴露 VSCode API。
 * 所有需要宿主交互的 API 通过 Deno.core.ops.op_xxx() 调用 Rust 端。
 *
 * 阶段 2：实现核心 API 子集，大部分 UI 交互返回模拟数据。
 * 阶段 3：完整 API 兼容 + Webview + 真实宿主 UI 回调。
 *
 * 注意：此文件在沙箱中执行，无法访问文件系统和网络（deno_core 默认禁用）。
 */

(function () {
  'use strict';

  // ═══════════════════════════════════════════════════════════════════════
  // 内部状态
  // ═══════════════════════════════════════════════════════════════════════

  // 命令注册表：command_id → handler function
  // 实际 handler 存储在 JS 中，Rust 端仅记录注册关系
  globalThis.__mox_commands = globalThis.__mox_commands || {};

  // 扩展 ID（由 DenoRuntime 在创建时注入）
  const EXTENSION_ID = globalThis.__mox_extension_id || 'unknown';

  // 便捷引用 ops
  const ops = Deno.core.ops;

  // ═══════════════════════════════════════════════════════════════════════
  // Disposable — 可释放资源
  // ═══════════════════════════════════════════════════════════════════════

  class Disposable {
    constructor(callOnDispose) {
      this._callOnDispose = callOnDispose;
      this._disposed = false;
    }

    dispose() {
      if (!this._disposed) {
        this._disposed = true;
        if (typeof this._callOnDispose === 'function') {
          try {
            this._callOnDispose();
          } catch (e) {
            console.error('Error in disposable callback:', e);
          }
        }
      }
    }

    static from(...disposables) {
      return new Disposable(() => {
        for (const d of disposables) {
          if (d && typeof d.dispose === 'function') {
            d.dispose();
          }
        }
      });
    }
  }

  // ═══════════════════════════════════════════════════════════════════════
  // EventEmitter — 事件发射器
  // ═══════════════════════════════════════════════════════════════════════

  class EventEmitter {
    constructor() {
      this._listeners = [];
      this._disposed = false;
      this.event = (listener, thisArgs, disposables) => {
        if (this._disposed) {
          return new Disposable(() => {});
        }
        const wrapped = thisArgs ? listener.bind(thisArgs) : listener;
        this._listeners.push(wrapped);
        const disposable = new Disposable(() => {
          const idx = this._listeners.indexOf(wrapped);
          if (idx >= 0) this._listeners.splice(idx, 1);
        });
        if (disposables && Array.isArray(disposables)) {
          disposables.push(disposable);
        }
        return disposable;
      };
    }

    fire(data) {
      if (this._disposed) return;
      // 复制数组，避免在回调中修改
      const listeners = this._listeners.slice();
      for (const listener of listeners) {
        try {
          listener(data);
        } catch (e) {
          console.error('Error in event listener:', e);
        }
      }
    }

    dispose() {
      this._disposed = true;
      this._listeners = [];
    }
  }

  // ═══════════════════════════════════════════════════════════════════════
  // Uri — 统一资源标识符
  // ═══════════════════════════════════════════════════════════════════════

  class Uri {
    constructor(scheme, authority, path, query, fragment) {
      this.scheme = scheme || '';
      this.authority = authority || '';
      this.path = path || '';
      this.query = query || '';
      this.fragment = fragment || '';
    }

    get fsPath() {
      if (this.scheme === 'file') {
        // 简化：Windows 路径处理
        let p = this.path;
        if (p.length > 2 && p[0] === '/' && p[2] === ':') {
          p = p.substring(1);
        }
        return p;
      }
      return this.path;
    }

    toString() {
      let result = this.scheme + ':';
      if (this.authority) result += '//' + this.authority;
      result += this.path;
      if (this.query) result += '?' + this.query;
      if (this.fragment) result += '#' + this.fragment;
      return result;
    }

    static parse(value) {
      try {
        // 简化解析
        const match = /^([^:]+):(?:\/\/([^/]*))?([^?#]*)(?:\?([^#]*))?(?:#(.*))?$/.exec(value);
        if (match) {
          return new Uri(match[1], match[2] || '', match[3] || '', match[4] || '', match[5] || '');
        }
      } catch (e) {
        // ignore
      }
      return new Uri('file', '', value, '', '');
    }

    static file(path) {
      let normalized = path.replace(/\\/g, '/');
      if (!normalized.startsWith('/')) normalized = '/' + normalized;
      return new Uri('file', '', normalized, '', '');
    }

    static joinPath(base, ...pathSegments) {
      const combined = [base.path, ...pathSegments].join('/').replace(/\/+/g, '/');
      return new Uri(base.scheme, base.authority, combined, base.query, base.fragment);
    }

    with(change) {
      return new Uri(
        change.scheme !== undefined ? change.scheme : this.scheme,
        change.authority !== undefined ? change.authority : this.authority,
        change.path !== undefined ? change.path : this.path,
        change.query !== undefined ? change.query : this.query,
        change.fragment !== undefined ? change.fragment : this.fragment
      );
    }
  }

  // ═══════════════════════════════════════════════════════════════════════
  // Position / Range / Selection — 文本位置
  // ═══════════════════════════════════════════════════════════════════════

  class Position {
    constructor(line, character) {
      this.line = line;
      this.character = character;
    }

    isBefore(other) {
      if (this.line < other.line) return true;
      if (this.line > other.line) return false;
      return this.character < other.character;
    }

    isAfter(other) {
      return other.isBefore(this);
    }

    isEqual(other) {
      return this.line === other.line && this.character === other.character;
    }

    translate(lineDelta, characterDelta) {
      const line = this.line + (lineDelta || 0);
      const character = this.character + (characterDelta || 0);
      return new Position(Math.max(0, line), Math.max(0, character));
    }

    with(line, character) {
      return new Position(
        line !== undefined ? line : this.line,
        character !== undefined ? character : this.character
      );
    }
  }

  class Range {
    constructor(startLineOrRange, startCharacter, endLine, endCharacter) {
      if (startLineOrRange instanceof Position) {
        this.start = startLineOrRange;
        this.end = startCharacter instanceof Position ? startCharacter : startLineOrRange;
      } else {
        this.start = new Position(startLineOrRange, startCharacter);
        this.end = new Position(endLine, endCharacter);
      }
    }

    get isEmpty() {
      return this.start.isEqual(this.end);
    }

    get isSingleLine() {
      return this.start.line === this.end.line;
    }

    contains(positionOrRange) {
      if (positionOrRange instanceof Position) {
        return (
          (positionOrRange.isAfter(this.start) || positionOrRange.isEqual(this.start)) &&
          (positionOrRange.isBefore(this.end) || positionOrRange.isEqual(this.end))
        );
      }
      return this.contains(positionOrRange.start) && this.contains(positionOrRange.end);
    }

    isEqual(other) {
      return this.start.isEqual(other.start) && this.end.isEqual(other.end);
    }

    intersection(range) {
      const start = this.start.isAfter(range.start) ? this.start : range.start;
      const end = this.end.isBefore(range.end) ? this.end : range.end;
      if (start.isBefore(end) || start.isEqual(end)) {
        return new Range(start, end);
      }
      return undefined;
    }

    union(range) {
      const start = this.start.isBefore(range.start) ? this.start : range.start;
      const end = this.end.isAfter(range.end) ? this.end : range.end;
      return new Range(start, end);
    }
  }

  class Selection extends Range {
    constructor(anchorLineOrPos, anchorCharacterOrPos, activeLine, activeCharacter) {
      if (anchorLineOrPos instanceof Position) {
        super(anchorLineOrPos, anchorCharacterOrPos instanceof Position ? anchorCharacterOrPos : anchorLineOrPos);
        this.anchor = anchorLineOrPos;
        this.active = anchorCharacterOrPos instanceof Position ? anchorCharacterOrPos : anchorLineOrPos;
      } else {
        const anchor = new Position(anchorLineOrPos, anchorCharacterOrPos);
        const active = new Position(activeLine, activeCharacter);
        super(anchor, active);
        this.anchor = anchor;
        this.active = active;
      }
    }

    get isReversed() {
      return this.active.isBefore(this.anchor);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════
  // TextDocument — 文本文档（简化版）
  // ═══════════════════════════════════════════════════════════════════════

  class TextDocument {
    constructor(uri, languageId, content) {
      this._uri = uri;
      this._languageId = languageId || 'plaintext';
      this._content = content || '';
      this._lines = this._content.split('\n');
      this._version = 1;
    }

    get uri() { return this._uri; }
    get fileName() { return this._uri.fsPath || this._uri.toString(); }
    get languageId() { return this._languageId; }
    get version() { return this._version; }
    get isDirty() { return false; }
    get isUntitled() { return false; }
    get lineCount() { return this._lines.length; }

    getText(range) {
      if (!range) return this._content;
      let result = '';
      for (let i = range.start.line; i <= range.end.line; i++) {
        if (i < this._lines.length) {
          let line = this._lines[i];
          if (i === range.start.line) line = line.substring(range.start.character);
          if (i === range.end.line) line = line.substring(0, range.end.character);
          result += line;
          if (i < range.end.line) result += '\n';
        }
      }
      return result;
    }

    lineAt(lineOrPosition) {
      const lineNum = typeof lineOrPosition === 'number' ? lineOrPosition : lineOrPosition.line;
      const text = this._lines[lineNum] || '';
      return {
        lineNumber: lineNum,
        text: text,
        range: new Range(lineNum, 0, lineNum, text.length),
        rangeIncludingLineBreak: new Range(lineNum, 0, lineNum + 1, 0),
        firstNonWhitespaceCharacterIndex: text.search(/\S/) >= 0 ? text.search(/\S/) : text.length,
        isEmptyOrWhitespace: /^\s*$/.test(text),
      };
    }

    offsetAt(position) {
      let offset = 0;
      for (let i = 0; i < position.line && i < this._lines.length; i++) {
        offset += this._lines[i].length + 1;
      }
      offset += Math.min(position.character, (this._lines[position.line] || '').length);
      return offset;
    }

    positionAt(offset) {
      let remaining = offset;
      for (let i = 0; i < this._lines.length; i++) {
        const lineLen = this._lines[i].length + 1;
        if (remaining < lineLen) {
          return new Position(i, remaining);
        }
        remaining -= lineLen;
      }
      return new Position(Math.max(0, this._lines.length - 1), 0);
    }

    getWordRangeAtPosition(position, regex) {
      const line = this._lines[position.line] || '';
      const pattern = regex || /(-?\d*\.\d\w*)|([^\`\~\!\@\#\%\^\&\*\(\)\-\=\+\[\{\]\}\\\|\;\:\'\"\,\.\<\>\/\?\s]+)/g;
      let match;
      while ((match = pattern.exec(line)) !== null) {
        const start = match.index;
        const end = start + match[0].length;
        if (position.character >= start && position.character <= end) {
          return new Range(position.line, start, position.line, end);
        }
      }
      return undefined;
    }
  }

  // ═══════════════════════════════════════════════════════════════════════
  // OutputChannel — 输出通道
  // ═══════════════════════════════════════════════════════════════════════

  class OutputChannel {
    constructor(name, channelId) {
      this._name = name;
      this._channelId = channelId;
      this._disposed = false;
    }

    get name() { return this._name; }

    append(value) {
      if (!this._disposed) {
        ops.op_output_channel_append(this._channelId, String(value));
      }
    }

    appendLine(value) {
      this.append(String(value) + '\n');
    }

    replace(value) {
      if (!this._disposed) {
        ops.op_output_channel_clear(this._channelId);
        ops.op_output_channel_append(this._channelId, String(value));
      }
    }

    clear() {
      if (!this._disposed) {
        ops.op_output_channel_clear(this._channelId);
      }
    }

    show(preserveFocus) {
      if (!this._disposed) {
        ops.op_output_channel_show(this._channelId);
      }
    }

    hide() {
      if (!this._disposed) {
        ops.op_output_channel_hide(this._channelId);
      }
    }

    dispose() {
      if (!this._disposed) {
        this._disposed = true;
        ops.op_output_channel_dispose(this._channelId);
      }
    }
  }

  // ═══════════════════════════════════════════════════════════════════════
  // commands — 命令系统
  // ═══════════════════════════════════════════════════════════════════════

  const commands = {
    /**
     * 注册命令
     * 返回 Disposable，调用 dispose() 注销命令
     */
    registerCommand(commandId, handler, thisArg) {
      const wrapped = thisArg ? handler.bind(thisArg) : handler;
      globalThis.__mox_commands[commandId] = wrapped;
      // 通知 Rust 端记录注册关系
      try {
        ops.op_register_command(EXTENSION_ID, commandId);
      } catch (e) {
        console.warn('op_register_command failed:', e);
      }
      return new Disposable(() => {
        delete globalThis.__mox_commands[commandId];
      });
    },

    /**
     * 注册文本编辑器命令
     * 阶段 2：简化为普通命令注册
     */
    registerTextEditorCommand(commandId, handler, thisArg) {
      return this.registerCommand(commandId, handler, thisArg);
    },

    /**
     * 执行命令
     * 先在当前运行时查找，找不到则尝试跨运行时执行
     */
    async executeCommand(commandId, ...args) {
      // 1. 在当前 JS 运行时查找 handler
      const handler = globalThis.__mox_commands[commandId];
      if (typeof handler === 'function') {
        return await handler(...args);
      }
      // 2. 尝试跨运行时执行（阶段 2 返回 null）
      try {
        const argsJson = args.length > 0 ? args : [];
        return await ops.op_execute_command(commandId, argsJson);
      } catch (e) {
        throw new Error(`Command '${commandId}' not found`);
      }
    },

    /**
     * 获取所有已注册命令
     */
    async getCommands(filterInternal) {
      try {
        const rustCommands = await ops.op_get_commands();
        const jsCommands = Object.keys(globalThis.__mox_commands);
        const all = new Set([...rustCommands, ...jsCommands]);
        return Array.from(all);
      } catch (e) {
        return Object.keys(globalThis.__mox_commands);
      }
    },
  };

  // ═══════════════════════════════════════════════════════════════════════
  // window — 窗口 UI
  // ═══════════════════════════════════════════════════════════════════════

  const window = {
    // 消息框
    async showInformationMessage(message, ...items) {
      const itemStrs = items.map(String);
      return await ops.op_show_information_message(String(message), itemStrs);
    },

    async showWarningMessage(message, ...items) {
      const itemStrs = items.map(String);
      return await ops.op_show_warning_message(String(message), itemStrs);
    },

    async showErrorMessage(message, ...items) {
      const itemStrs = items.map(String);
      return await ops.op_show_error_message(String(message), itemStrs);
    },

    // 输入框
    async showInputBox(options) {
      return await ops.op_show_input_box(options || {});
    },

    // 快速选择
    async showQuickPick(items, options) {
      const itemStrs = Array.isArray(items)
        ? items.map((item) => (typeof item === 'string' ? item : item.label || String(item)))
        : [];
      return await ops.op_show_quick_pick(itemStrs, options || {});
    },

    // 输出通道
    createOutputChannel(name) {
      const channelId = ops.op_create_output_channel(String(name));
      return new OutputChannel(name, channelId);
    },

    // 文本编辑器（阶段 2 简化）
    get activeTextEditor() {
      // 阶段 2：返回 undefined（无活动编辑器）
      return undefined;
    },

    get visibleTextEditors() {
      return [];
    },

    async showTextDocument(document, options) {
      // 阶段 2：记录日志，返回模拟编辑器
      const uri = document instanceof Uri ? document.toString() : String(document);
      console.log('[vscode window] showTextDocument:', uri);
      return {
        document: document instanceof TextDocument ? document : new TextDocument(Uri.parse(uri), 'plaintext', ''),
        selection: new Selection(0, 0, 0, 0),
        visibleRanges: [],
        options: {},
        viewColumn: 1,
        async edit(callback) { return false; },
        revealRange(range, revealType) {},
      };
    },

    // 状态消息
    setStatusBarMessage(message, timeoutOrThenable) {
      console.log('[vscode statusbar]', message);
      return new Disposable(() => {});
    },

    // 进度
    async withProgress(options, task) {
      const progress = {
        report: (update) => {
          if (update.message) console.log('[progress]', update.message);
        },
      };
      const cancellationToken = { isCancellationRequested: false, onCancellationRequested: () => new Disposable(() => {}) };
      return await task(progress, cancellationToken);
    },
  };

  // ═══════════════════════════════════════════════════════════════════════
  // workspace — 工作区
  // ═══════════════════════════════════════════════════════════════════════

  // 配置变更事件
  const _onDidChangeConfiguration = new EventEmitter();

  const workspace = {
    // 工作区文件夹
    get workspaceFolders() {
      try {
        const folders = ops.op_get_workspace_folders();
        return folders.map((f, i) => ({
          uri: Uri.parse(f.uri),
          name: f.name,
          index: f.index !== undefined ? f.index : i,
        }));
      } catch (e) {
        return [];
      }
    },

    get workspaceFile() {
      try {
        const file = ops.op_get_workspace_file();
        return file ? Uri.parse(file) : undefined;
      } catch (e) {
        return undefined;
      }
    },

    // 打开文本文档
    async openTextDocument(uriOrOptions) {
      let uri;
      if (typeof uriOrOptions === 'string') {
        uri = uriOrOptions;
      } else if (uriOrOptions instanceof Uri) {
        uri = uriOrOptions.toString();
      } else if (uriOrOptions && uriOrOptions.uri) {
        uri = uriOrOptions.uri instanceof Uri ? uriOrOptions.uri.toString() : String(uriOrOptions.uri);
      } else if (uriOrOptions && uriOrOptions.content) {
        // untitled document
        return new TextDocument(Uri.parse('untitled:Untitled-1'), uriOrOptions.language || 'plaintext', uriOrOptions.content);
      } else {
        uri = 'untitled:Untitled-1';
      }
      const docInfo = await ops.op_open_text_document(uri);
      return new TextDocument(Uri.parse(uri), docInfo.languageId || 'plaintext', docInfo.getText || '');
    },

    // 配置
    getConfiguration(section, resource) {
      try {
        const config = ops.op_get_configuration(section || '');
        return {
          get: (key, defaultValue) => {
            const fullKey = section ? section + '.' + key : key;
            // 简化：从 config 对象中查找
            const parts = key.split('.');
            let current = config;
            for (const part of parts) {
              if (current && typeof current === 'object' && part in current) {
                current = current[part];
              } else {
                return defaultValue;
              }
            }
            return current !== undefined ? current : defaultValue;
          },
          has: (key) => {
            const parts = key.split('.');
            let current = config;
            for (const part of parts) {
              if (current && typeof current === 'object' && part in current) {
                current = current[part];
              } else {
                return false;
              }
            }
            return true;
          },
          update: async (key, value, configurationTarget) => {
            // 阶段 2：不支持持久化配置修改
            console.warn('[vscode workspace] configuration update not supported in stage 2');
          },
          inspect: (key) => null,
        };
      } catch (e) {
        return {
          get: (key, defaultValue) => defaultValue,
          has: () => false,
          update: async () => {},
          inspect: () => null,
        };
      }
    },

    // 事件
    get onDidChangeConfiguration() {
      return _onDidChangeConfiguration.event;
    },

    // 工作区变更事件（阶段 2 简化）
    onDidChangeWorkspaceFolders: new EventEmitter().event,
    onDidOpenTextDocument: new EventEmitter().event,
    onDidCloseTextDocument: new EventEmitter().event,
    onDidChangeTextDocument: new EventEmitter().event,
    onDidSaveTextDocument: new EventEmitter().event,

    // 文本文档列表
    get textDocuments() {
      return [];
    },

    // 文件系统（阶段 2 不支持，返回未实现）
    get fs() {
      return {
        stat: async (uri) => { throw new Error('workspace.fs not implemented in MOX runtime'); },
        readDirectory: async (uri) => { throw new Error('workspace.fs not implemented in MOX runtime'); },
        createDirectory: async (uri) => { throw new Error('workspace.fs not implemented in MOX runtime'); },
        readFile: async (uri) => { throw new Error('workspace.fs not implemented in MOX runtime'); },
        writeFile: async (uri, content) => { throw new Error('workspace.fs not implemented in MOX runtime'); },
        delete: async (uri, options) => { throw new Error('workspace.fs not implemented in MOX runtime'); },
        rename: async (oldUri, newUri, options) => { throw new Error('workspace.fs not implemented in MOX runtime'); },
        copy: async (source, target, options) => { throw new Error('workspace.fs not implemented in MOX runtime'); },
        isWritableFileSystem: (scheme) => false,
      };
    },
  };

  // ═══════════════════════════════════════════════════════════════════════
  // extensions — 扩展管理
  // ═══════════════════════════════════════════════════════════════════════

  const extensions = {
    /**
     * 获取指定扩展
     * 阶段 2：返回 undefined（不支持查询其他扩展）
     */
    getExtension(extensionId) {
      try {
        const ext = ops.op_get_extension(extensionId);
        if (ext) {
          return {
            id: ext.id,
            extensionPath: ext.extension_path,
            isActive: ext.is_active,
            packageJSON: ext.package_json,
            extensionUri: Uri.parse(ext.extension_path),
            exports: undefined,
            activate: async () => undefined,
          };
        }
      } catch (e) {
        // ignore
      }
      return undefined;
    },

    /**
     * 获取所有已安装扩展
     * 阶段 2：返回空数组
     */
    get all() {
      try {
        const exts = ops.op_get_all_extensions();
        return exts.map((ext) => ({
          id: ext.id,
          extensionPath: ext.extension_path,
          isActive: ext.is_active,
          packageJSON: ext.package_json,
          extensionUri: Uri.parse(ext.extension_path),
          exports: undefined,
          activate: async () => undefined,
        }));
      } catch (e) {
        return [];
      }
    },

    // 扩展变更事件
    onDidChangeExtensions: new EventEmitter().event,
  };

  // ═══════════════════════════════════════════════════════════════════════
  // env — 环境信息
  // ═══════════════════════════════════════════════════════════════════════

  const env = {
    language: 'en',
    appName: 'MOX Platform',
    appRoot: '',
    clipboard: {
      writeText: async (value) => { console.log('[clipboard] write:', value); },
      readText: async () => '',
    },
    openExternal: async (target) => { console.log('[env] openExternal:', target); return false; },
    asExternalUri: async (target) => target,
    machineId: 'mox-machine-id',
    sessionId: 'mox-session-id',
    uriScheme: 'mox',
    remoteName: undefined,
  };

  // ═══════════════════════════════════════════════════════════════════════
  // l10n — 本地化（简化）
  // ═══════════════════════════════════════════════════════════════════════

  const l10n = {
    t: (message, ...args) => {
      // 阶段 2：不做本地化，直接返回原消息
      if (args.length > 0) {
        return message.replace(/\{(\d+)\}/g, (_, i) => args[parseInt(i)] || '');
      }
      return message;
    },
    uri: undefined,
    bundle: undefined,
  };

  // ═══════════════════════════════════════════════════════════════════════
  // 版本信息
  // ═══════════════════════════════════════════════════════════════════════

  const version = '1.80.0'; // 模拟 VSCode 版本

  // ═══════════════════════════════════════════════════════════════════════
  // 导出 vscode 全局对象
  // ═══════════════════════════════════════════════════════════════════════

  globalThis.vscode = {
    // 类
    Disposable,
    EventEmitter,
    Uri,
    Position,
    Range,
    Selection,
    TextDocument,
    OutputChannel,

    // 命名空间
    commands,
    window,
    workspace,
    extensions,
    env,
    l10n,

    // 版本
    version,

    // 未实现的 API — 返回 Promise.reject
    // 阶段 3 实现
    get authentication() { return _notImplemented('authentication'); },
    get debug() { return _notImplemented('debug'); },
    get scm() { return _notImplemented('scm'); },
    get tasks() { return _notImplemented('tasks'); },
    get comments() { return _notImplemented('comments'); },
    get languages() { return _notImplemented('languages'); },
    get tests() { return _notImplemented('tests'); },
    get notebooks() { return _notImplemented('notebooks'); },
    get chat() { return _notImplemented('chat'); },
    get ai() { return _notImplemented('ai'); },
    get mcp() { return _notImplemented('mcp'); },
    get ui() { return _notImplemented('ui'); },
    get secrets() { return _notImplemented('secrets'); },
    get terminal() { return _notImplemented('terminal'); },
    get editors() { return _notImplemented('editors'); },
    get views() { return _notImplemented('views'); },
  };

  /**
   * 生成未实现 API 的代理对象
   * 所有属性访问返回函数，调用时抛出 'not implemented' 错误
   */
  function _notImplemented(name) {
    return new Proxy({}, {
      get: (target, prop) => {
        if (prop === Symbol.toPrimitive) return () => `[vscode ${name}: not implemented]`;
        if (prop === 'then') return undefined; // 不是 Promise
        return (...args) => {
          return Promise.reject(new Error(`vscode.${name}.${String(prop)} not implemented in MOX runtime`));
        };
      },
    });
  }

  // 标记 shim 已加载
  globalThis.__mox_vscode_shim_loaded = true;
  console.log('[MOX] VSCode API shim loaded for extension:', EXTENSION_ID);
})();
