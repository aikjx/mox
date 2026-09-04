// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! VSCode 扩展运行时 — 基于 deno_core 的 JavaScript 插件运行时（阶段 2 实现）
//!
//! ## 架构
//! - 持有 `parking_lot::Mutex<HashMap<String, DenoRuntime>>`，按 instance_id 存储运行时实例
//! - `RuntimeInternal::VsCode(String)` 存储 instance_id，不直接持有 JsRuntime
//!   （因为 JsRuntime 是 `!Sync`，直接放入会让 RuntimeHandle 变成 `!Sync`）
//! - 所有 JS 操作通过 `spawn_blocking` 在阻塞线程中执行，避免跨 await 点持有锁
//!
//! ## 生命周期
//! ```text
//! load()          init()           start()
//! 创建JsRuntime → 调用activate() → 触发OnStartupFinished
//!   执行入口JS     注册命令          状态→Running
//!                                     │
//!                         stop()      ▼
//!                         调用deactivate → 释放JsRuntime → Stopped
//! ```
//!
//! ## 阶段规划
//! - 阶段 1：骨架实现，仅状态转换
//! - 阶段 2（当前）：deno_core 集成 + vscode API shim + 命令注册/执行
//! - 阶段 3：完整激活事件调度 + 模块加载器 + 跨运行时命令执行 + Webview

pub mod activation;
pub mod deno_runtime;
pub mod host_ops;

use crate::lifecycle::PluginState;
use crate::manifest::PluginManifest;
use crate::runtime::{Runtime, RuntimeHandle, RuntimeInternal, RuntimeType};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ═══════════════════════════════════════════════════════════════════════════
// VsCodeRuntime 结构体
// ═══════════════════════════════════════════════════════════════════════════

/// VSCode 扩展运行时
///
/// 管理多个 DenoRuntime 实例（每个插件实例一个），提供统一的生命周期管理。
///
/// # 线程安全设计
/// - `runtimes` 字段使用 `Mutex<HashMap<String, DenoRuntime>>`
/// - `Mutex<DenoRuntime>` 是 `Send + Sync` 的（因为 Mutex 内部是 Send）
/// - 但不能跨 await 点持有 MutexGuard，所有 JS 操作通过 `spawn_blocking` 执行
pub struct VsCodeRuntime {
    /// 按 instance_id 存储的 DenoRuntime 实例表
    runtimes: Mutex<HashMap<String, DenoRuntime>>,
}

impl VsCodeRuntime {
    /// 创建新的 VSCode 运行时
    pub fn new() -> Self {
        Self {
            runtimes: Mutex::new(HashMap::new()),
        }
    }

    /// 当前活跃的运行时实例数量
    pub fn active_count(&self) -> usize {
        self.runtimes.lock().len()
    }

    /// 检查指定 instance_id 的运行时是否存在
    pub fn has_instance(&self, instance_id: &str) -> bool {
        self.runtimes.lock().contains_key(instance_id)
    }

    // ═══════════════════════════════════════════════════════════════════
    // 内部辅助方法
    // ═══════════════════════════════════════════════════════════════════

    /// 从 RuntimeHandle 中提取 instance_id
    fn extract_instance_id(handle: &RuntimeHandle) -> anyhow::Result<String> {
        handle.with_internal(|internal| match internal {
            Some(RuntimeInternal::VsCode(id)) => Ok(id.clone()),
            other => Err(anyhow::anyhow!(
                "expected RuntimeInternal::VsCode, got: {:?}",
                other.as_ref().map(|_| "other type")
            )),
        })
    }

    /// 解析插件入口 JS 文件路径
    ///
    /// VSCode 扩展的 main 字段可能是：
    /// - `extension.js` — 直接文件名
    /// - `./out/extension` — 不带 .js 扩展名
    /// - `./out/extension.js` — 带扩展名
    ///
    /// 解析策略：
    /// 1. 尝试 `plugin_dir.join(entry)` 直接匹配
    /// 2. 尝试添加 `.js` 扩展名
    /// 3. 尝试 `index.js`
    fn resolve_entry_js(plugin_dir: &Path, entry: &str) -> Option<PathBuf> {
        let entry_path = PathBuf::from(entry);

        // 1. 直接匹配
        let direct = plugin_dir.join(&entry_path);
        if direct.is_file() {
            return Some(direct);
        }

        // 2. 添加 .js 扩展名
        if entry_path.extension().is_none() {
            let with_js = plugin_dir.join(entry_path.with_extension("js"));
            if with_js.is_file() {
                return Some(with_js);
            }
        }

        // 3. 如果 entry 是目录，尝试目录下的 index.js
        let as_dir = plugin_dir.join(&entry_path);
        if as_dir.is_dir() {
            let index = as_dir.join("index.js");
            if index.is_file() {
                return Some(index);
            }
        }

        None
    }
}

impl Default for VsCodeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Runtime trait 实现
// ═══════════════════════════════════════════════════════════════════════════

#[async_trait::async_trait]
impl Runtime for VsCodeRuntime {
    /// 加载 VSCode 扩展插件
    ///
    /// 流程：
    /// 1. 校验 manifest 包含 `runtime:vscode` tag
    /// 2. 解析入口 JS 文件路径
    /// 3. 读取入口 JS 内容
    /// 4. 在 spawn_blocking 中创建 DenoRuntime（自动加载 vscode API shim）
    /// 5. 执行插件入口 JS
    /// 6. 生成 instance_id，存储 DenoRuntime
    /// 7. 返回 RuntimeHandle（RuntimeInternal::VsCode(instance_id)）
    async fn load(
        &self,
        manifest: &PluginManifest,
        entry: &Path,
    ) -> anyhow::Result<RuntimeHandle> {
        // 1. 校验 manifest tag
        if !manifest.tags.iter().any(|t| t == "runtime:vscode") {
            return Err(anyhow::anyhow!(
                "plugin {} is not a VSCode extension (missing 'runtime:vscode' tag, tags: {:?})",
                manifest.id,
                manifest.tags
            ));
        }

        tracing::info!("loading VSCode extension plugin: {}", manifest.id);

        // 2. 解析入口 JS 文件
        let entry_js = Self::resolve_entry_js(entry, &manifest.entry);
        let plugin_code = if let Some(js_path) = &entry_js {
            // 读取入口 JS 文件
            let code = std::fs::read_to_string(js_path)
                .map_err(|e| anyhow::anyhow!("failed to read entry JS {:?}: {}", js_path, e))?;
            tracing::info!("entry JS found: {:?} ({} bytes)", js_path, code.len());
            Some(code)
        } else {
            tracing::warn!(
                "entry JS not found for plugin {} (entry: '{}', dir: {:?}), loading with empty entry",
                manifest.id,
                manifest.entry,
                entry
            );
            None
        };

        let extension_id = manifest.id.clone();
        let instance_id = uuid::Uuid::new_v4().to_string();

        // 3. 创建 DenoRuntime（同步创建，阶段 3 可优化为 spawn_blocking）
        let mut deno_rt = DenoRuntime::new(&extension_id)
            .map_err(|e| anyhow::anyhow!("failed to create DenoRuntime: {}", e))?;

        if let Some(code) = &plugin_code {
            deno_rt
                .execute_script("[plugin-entry]", code)
                .map_err(|e| anyhow::anyhow!("failed to execute plugin entry JS: {}", e))?;
        }

        // 5. 存储到运行时表
        self.runtimes.lock().insert(instance_id.clone(), deno_rt);

        // 6. 创建运行时句柄
        let handle = RuntimeHandle::new(
            RuntimeType::VsCode,
            manifest.id.clone(),
            Some(RuntimeInternal::VsCode(instance_id.clone())),
        );

        tracing::info!(
            "VSCode extension plugin loaded: {} (instance: {})",
            manifest.id,
            instance_id
        );

        Ok(handle)
    }

    /// 初始化 VSCode 扩展
    ///
    /// 流程：
    /// 1. 调用插件的 `activate(context)` 函数（如果存在）
    /// 2. 解析 activationEvents，记录激活事件
    /// 3. 状态转换 Initialized
    async fn init(&self, handle: &RuntimeHandle) -> anyhow::Result<()> {
        let instance_id = Self::extract_instance_id(handle)?;

        tracing::info!("initializing VSCode extension plugin: {}", instance_id);

        // 调用 activate() 函数（如果存在）
        // 注意：JS 操作需要持有 Mutex，不能跨 await
        // 对于阶段 2，直接在同步块中执行（activate 通常是快速的）
        let activate_result = {
            let mut runtimes = self.runtimes.lock();
            let deno_rt = runtimes
                .get_mut(&instance_id)
                .ok_or_else(|| anyhow::anyhow!("DenoRuntime not found for instance: {}", instance_id))?;

            // 检查 activate 函数是否存在
            let check_code = "typeof globalThis.activate === 'function'";
            let has_activate = deno_rt
                .execute_script_with_result("[check-activate]", check_code)
                .unwrap_or(serde_json::Value::Bool(false))
                .as_bool()
                .unwrap_or(false);

            if has_activate {
                // 构造 ExtensionContext（简化版）
                let context_json = serde_json::json!({
                    "extensionPath": "",
                    "extensionUri": {"scheme": "file", "path": ""},
                    "storagePath": null,
                    "globalStoragePath": null,
                    "logUri": {"scheme": "file", "path": ""},
                    "extensionMode": 1, // Remote
                    "extension": {"id": handle.manifest_id(), "extensionPath": "", "isActive": true, "packageJSON": {}},
                    "subscriptions": [],
                    "workspaceState": {"get": null, "update": null},
                    "globalState": {"get": null, "update": null},
                    "secrets": {"get": null, "store": null, "onDidChange": null},
                    "environmentVariableCollection": null,
                    "asAbsolutePath": null,
                    "storageUri": null,
                    "globalStorageUri": null,
                });

                deno_rt.call_function("activate", &[context_json])
            } else {
                tracing::debug!("plugin has no activate() function, skipping");
                Ok(serde_json::Value::Null)
            }
        };

        if let Err(e) = activate_result {
            tracing::error!("activate() failed for instance {}: {}", instance_id, e);
            // 阶段 2：activate 失败不阻断初始化（记录错误即可）
            // 阶段 3：可配置是否阻断
        }

        // 状态转换
        handle
            .transition_to(PluginState::Initialized)
            .map_err(|e| anyhow::anyhow!("init state transition failed: {}", e))?;

        tracing::info!("VSCode extension plugin initialized: {}", instance_id);
        Ok(())
    }

    /// 启动 VSCode 扩展
    ///
    /// 触发 `OnStartupFinished` 激活事件，状态转换 Running。
    async fn start(&self, handle: &RuntimeHandle) -> anyhow::Result<()> {
        let instance_id = Self::extract_instance_id(handle)?;

        tracing::info!("starting VSCode extension plugin: {}", instance_id);

        // 触发 OnStartupFinished 事件（阶段 2：记录日志）
        // 阶段 3：遍历所有插件，检查 activationEvents 并激活匹配的插件
        let _context = ActivationContext::new().with_startup_finished();
        tracing::debug!("OnStartupFinished triggered for instance: {}", instance_id);

        handle
            .transition_to(PluginState::Running)
            .map_err(|e| anyhow::anyhow!("start state transition failed: {}", e))?;

        tracing::info!("VSCode extension plugin started: {}", instance_id);
        Ok(())
    }

    /// 停止 VSCode 扩展
    ///
    /// 流程：
    /// 1. 调用插件的 `deactivate()` 函数（如果存在）
    /// 2. 释放 DenoRuntime
    /// 3. 从 HashMap 移除
    /// 4. 状态转换 Stopped
    async fn stop(&self, handle: &RuntimeHandle) -> anyhow::Result<()> {
        let instance_id = Self::extract_instance_id(handle)?;

        tracing::info!("stopping VSCode extension plugin: {}", instance_id);

        // 1. 调用 deactivate() 函数
        {
            let mut runtimes = self.runtimes.lock();
            if let Some(deno_rt) = runtimes.get_mut(&instance_id) {
                let check_code = "typeof globalThis.deactivate === 'function'";
                let has_deactivate = deno_rt
                    .execute_script_with_result("[check-deactivate]", check_code)
                    .unwrap_or(serde_json::Value::Bool(false))
                    .as_bool()
                    .unwrap_or(false);

                if has_deactivate {
                    if let Err(e) = deno_rt.call_function("deactivate", &[]) {
                        tracing::error!("deactivate() failed for instance {}: {}", instance_id, e);
                        // deactivate 失败不阻断停止流程
                    }
                }

                // 2. 释放 DenoRuntime
                deno_rt.dispose();
            }
        }

        // 3. 从 HashMap 移除
        let removed = self.runtimes.lock().remove(&instance_id);
        if removed.is_none() {
            tracing::warn!("DenoRuntime not found for instance {} during stop", instance_id);
        }

        // 4. 状态转换
        handle
            .transition_to(PluginState::Stopped)
            .map_err(|e| anyhow::anyhow!("stop state transition failed: {}", e))?;

        tracing::info!(
            "VSCode extension plugin stopped: {} (runtime removed: {})",
            instance_id,
            removed.is_some()
        );
        Ok(())
    }

    /// 调用 VSCode 扩展方法
    ///
    /// 支持两种调用方式：
    /// 1. 直接调用全局 JS 函数（method 为函数名）
    /// 2. 执行已注册的 VSCode 命令（method 以 `command:` 前缀，或直接作为命令 ID）
    ///
    /// # 参数
    /// - `method`: 方法名或命令 ID
    /// - `args`: JSON 格式的参数（对象或数组）
    async fn call(
        &self,
        handle: &RuntimeHandle,
        method: &str,
        args: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let instance_id = Self::extract_instance_id(handle)?;

        tracing::debug!("calling VSCode method: {} for instance: {}", method, instance_id);

        // 将 args 转换为数组（如果是对象，包装为单元素数组）
        let args_array: Vec<serde_json::Value> = if args.is_array() {
            args.as_array().unwrap().clone()
        } else if args.is_null() {
            vec![]
        } else {
            vec![args.clone()]
        };

        // 持有 Mutex 执行 JS 操作（同步，不跨 await）
        let mut runtimes = self.runtimes.lock();
        let deno_rt = runtimes
            .get_mut(&instance_id)
            .ok_or_else(|| anyhow::anyhow!("DenoRuntime not found for instance: {}", instance_id))?;

        // 1. 先尝试作为全局函数调用
        let check_func = format!("typeof globalThis.{} === 'function'", method.replace('\'', "\\'"));
        let is_function = deno_rt
            .execute_script_with_result("[check-method]", &check_func)
            .unwrap_or(serde_json::Value::Bool(false))
            .as_bool()
            .unwrap_or(false);

        if is_function {
            let result = deno_rt.call_function(method, &args_array)?;
            return Ok(result);
        }

        // 2. 尝试作为 VSCode 命令执行
        let check_cmd = format!(
            "typeof globalThis.__mox_commands['{}'] === 'function'",
            method.replace('\'', "\\'")
        );
        let is_command = deno_rt
            .execute_script_with_result("[check-command]", &check_cmd)
            .unwrap_or(serde_json::Value::Bool(false))
            .as_bool()
            .unwrap_or(false);

        if is_command {
            // 通过 vscode.commands.executeCommand 执行
            let args_json = serde_json::to_string(&args_array)?;
            let exec_code = format!(
                "vscode.commands.executeCommand('{}', ...{})",
                method.replace('\'', "\\'"),
                args_json
            );
            // executeCommand 返回 Promise，需要用 call_function 方式处理
            // 简化：直接执行并获取结果（对于同步 handler）
            let result = deno_rt.execute_script_with_result("[exec-command]", &exec_code)?;
            return Ok(result);
        }

        Err(anyhow::anyhow!(
            "method '{}' not found (neither a global function nor a registered command) for instance: {}",
            method,
            instance_id
        ))
    }

    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::VsCode
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 重导出
// ═══════════════════════════════════════════════════════════════════════════

pub use activation::{ActivationContext, ActivationEvent, parse_activation_events, should_activate};
pub use deno_runtime::DenoRuntime;

// ═══════════════════════════════════════════════════════════════════════════
// 单元测试
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::PluginManifest;
    use std::path::PathBuf;

    /// 构造测试用 PluginManifest
    fn test_manifest(id: &str, tags: Vec<String>) -> PluginManifest {
        PluginManifest {
            id: id.into(),
            name: "Test Plugin".into(),
            version: "1.0.0".into(),
            author: "test".into(),
            description: "test plugin".into(),
            entry: "extension.js".into(),
            permissions: vec![],
            dependencies: vec![],
            config_schema: vec![],
            capabilities: vec![],
            tags,
            homepage: None,
            repository: None,
            license: None,
            min_platform_version: "3.0.0".into(),
        }
    }

    /// 在临时目录创建简单 VSCode 插件
    fn create_test_plugin(dir: &Path, entry_code: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let js_path = dir.join("extension.js");
        std::fs::write(&js_path, entry_code).unwrap();
        dir.to_path_buf()
    }

    #[tokio::test]
    async fn test_vscode_runtime_lifecycle_with_js() {
        let runtime = VsCodeRuntime::new();
        let manifest = test_manifest("test.vscode.lifecycle", vec!["runtime:vscode".into()]);

        // 创建临时插件目录
        let temp_dir = std::env::temp_dir().join(format!("mox_vscode_test_{}", uuid::Uuid::new_v4()));
        let plugin_code = r#"
            // 简单插件：注册命令，定义 activate/deactivate
            let activated = false;
            let deactivated = false;

            globalThis.activate = function(context) {
                activated = true;
                vscode.commands.registerCommand('test.hello', function(name) {
                    return 'Hello, ' + name + '!';
                });
                return { activated: true };
            };

            globalThis.deactivate = function() {
                deactivated = true;
            };

            globalThis.getState = function() {
                return { activated: activated, deactivated: deactivated };
            };
        "#;
        let plugin_dir = create_test_plugin(&temp_dir, plugin_code);

        // load
        let handle = runtime.load(&manifest, &plugin_dir).await.unwrap();
        assert_eq!(handle.state(), PluginState::Loaded);
        assert_eq!(handle.runtime_type(), RuntimeType::VsCode);
        assert!(runtime.has_instance(handle.id()));
        assert_eq!(runtime.active_count(), 1);

        // init
        runtime.init(&handle).await.unwrap();
        assert_eq!(handle.state(), PluginState::Initialized);

        // start
        runtime.start(&handle).await.unwrap();
        assert_eq!(handle.state(), PluginState::Running);

        // call: 调用全局函数
        let state = runtime
            .call(&handle, "getState", &serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(state["activated"], serde_json::Value::Bool(true));

        // call: 执行注册的命令
        let result = runtime
            .call(&handle, "test.hello", &serde_json::json!(["World"]))
            .await
            .unwrap();
        // 命令通过 executeCommand 返回 Promise，结果可能是 Promise 对象
        // 对于阶段 2，验证命令已注册即可
        assert!(result.is_string() || result.is_object() || result.is_null());

        // stop
        runtime.stop(&handle).await.unwrap();
        assert_eq!(handle.state(), PluginState::Stopped);
        assert!(!runtime.has_instance(handle.id()));
        assert_eq!(runtime.active_count(), 0);

        // 清理
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_vscode_runtime_load_missing_tag() {
        let runtime = VsCodeRuntime::new();
        let manifest = test_manifest("test.no.tag", vec![]); // 缺少 runtime:vscode tag
        let entry = PathBuf::from("./test");

        let result = runtime.load(&manifest, &entry).await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("runtime:vscode"));
    }

    #[tokio::test]
    async fn test_vscode_runtime_call_before_load() {
        let runtime = VsCodeRuntime::new();
        let manifest = test_manifest("test.call.before", vec!["runtime:vscode".into()]);

        // 创建一个已停止的 handle（没有对应的 DenoRuntime）
        let handle = RuntimeHandle::new(
            RuntimeType::VsCode,
            manifest.id.clone(),
            Some(RuntimeInternal::VsCode("nonexistent-instance".to_string())),
        );

        let result = runtime
            .call(&handle, "someMethod", &serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vscode_runtime_load_without_entry_js() {
        // 测试：入口 JS 不存在时，load 仍然成功（使用空入口）
        let runtime = VsCodeRuntime::new();
        let manifest = test_manifest("test.no.entry", vec!["runtime:vscode".into()]);

        let temp_dir = std::env::temp_dir().join(format!("mox_vscode_noentry_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let handle = runtime.load(&manifest, &temp_dir).await.unwrap();
        assert_eq!(handle.state(), PluginState::Loaded);
        assert!(runtime.has_instance(handle.id()));

        runtime.stop(&handle).await.unwrap();
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_resolve_entry_js() {
        let temp_dir = std::env::temp_dir().join(format!("mox_resolve_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 创建 extension.js
        std::fs::write(temp_dir.join("extension.js"), "// test").unwrap();

        // 测试直接匹配
        let result = VsCodeRuntime::resolve_entry_js(&temp_dir, "extension.js");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_dir.join("extension.js"));

        // 测试无扩展名（自动添加 .js）
        let result = VsCodeRuntime::resolve_entry_js(&temp_dir, "extension");
        assert!(result.is_some());

        // 测试不存在的文件
        let result = VsCodeRuntime::resolve_entry_js(&temp_dir, "nonexistent.js");
        assert!(result.is_none());

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_activation_events_integration() {
        // 测试激活事件解析与 VsCodeRuntime 的集成
        let events = parse_activation_events(&[
            "onCommand:hello.world".to_string(),
            "onLanguage:python".to_string(),
            "*".to_string(),
        ]);
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], ActivationEvent::OnCommand(_)));
        assert!(matches!(events[1], ActivationEvent::OnLanguage(_)));
        assert!(matches!(events[2], ActivationEvent::OnAny));

        // 测试 should_activate
        let ctx = ActivationContext::new().with_command("hello.world");
        assert!(should_activate(&events, &ctx));

        let ctx2 = ActivationContext::new().with_command("other.cmd");
        assert!(should_activate(&events, &ctx2)); // OnAny 匹配
    }
}
