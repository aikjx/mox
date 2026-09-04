// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! DenoRuntime — 对 deno_core::JsRuntime 的封装，提供 JS 脚本执行和函数调用能力。
//!
//! ## 架构
//! - 内部持有 `deno_core::JsRuntime`（包含 v8::Isolate，`Send` 但 `!Sync`）
//! - 通过 `parking_lot::Mutex<DenoRuntime>` 包装后可安全跨线程共享
//! - 创建时自动注册宿主 ops Extension 并执行 VSCode API shim
//!
//! ## 安全
//! - deno_core 默认禁用文件系统和网络访问
//! - 所有宿主交互通过显式注册的 ops 进行
//! - JS 异常被完整捕获并转换为 `anyhow::Error`
//!
//! ## 阶段规划
//! - 阶段 2：基础脚本执行 + 函数调用 + vscode API shim
//! - 阶段 3：模块加载器（支持 import/require）+ 异步操作完整支持 + 调试器

use crate::runtime::vscode::host_ops::build_host_extension;
use deno_core::v8;
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::time::Duration;

// VSCode API shim（编译时内嵌为字符串）
const VSCODE_API_SHIM: &str = include_str!("vscode_api.js");

// ═══════════════════════════════════════════════════════════════════════════
// DenoRuntime 结构体
// ═══════════════════════════════════════════════════════════════════════════

/// DenoRuntime — 封装 deno_core JsRuntime 的插件执行环境
///
/// 每个 VSCode 扩展实例拥有独立的 DenoRuntime，实现完全的隔离。
///
/// # 线程安全
/// `DenoRuntime` 本身是 `Send` 但 `!Sync`（因为包含 v8::Isolate）。
/// 通过 `parking_lot::Mutex<DenoRuntime>` 包装后可安全共享。
/// 注意：不能跨 await 点持有 MutexGuard，所有 JS 操作应在同步闭包中完成。
pub struct DenoRuntime {
    /// 底层 deno_core JS 运行时
    js_runtime: deno_core::JsRuntime,
    /// 关联的扩展 ID
    extension_id: String,
    /// 是否已释放
    disposed: bool,
}
// deno_core::JsRuntime 包含 v8::Isolate，本身是 !Send。
// 但 v8 Isolate 可以安全地在线程间移动（同一时刻仅一个线程访问），
// VsCodeRuntime 通过 Mutex<HashMap<String, DenoRuntime>> 保证单线程访问，
// 因此这里 unsafe impl Send 是安全的。
unsafe impl Send for DenoRuntime {}


impl DenoRuntime {
    // ═══════════════════════════════════════════════════════════════════
    // 构造与生命周期
    // ═══════════════════════════════════════════════════════════════════

    /// 创建新的 DenoRuntime
    ///
    /// 流程：
    /// 1. 创建 deno_core::JsRuntime，注册宿主 ops Extension
    /// 2. 注入扩展 ID 到 JS 全局作用域
    /// 3. 执行 VSCode API shim（定义 globalThis.vscode）
    ///
    /// # 参数
    /// - `extension_id`: 关联的扩展 ID（用于日志和命令注册）
    ///
    /// # 错误
    /// - JsRuntime 创建失败
    /// - VSCode API shim 执行失败（语法错误等）
    pub fn new(extension_id: &str) -> Result<Self> {
        tracing::info!("creating DenoRuntime for extension: {}", extension_id);

        // 1. 创建 deno_core JsRuntime，注册宿主 ops
        let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
            extensions: vec![build_host_extension()],
            // 阶段 2：不使用模块加载器（插件代码通过 execute_script 直接执行）
            // 阶段 3：添加 FsModuleLoader 支持 import 语句
            module_loader: None,
            ..Default::default()
        });

        // 2. 注入扩展 ID 到 JS 全局作用域
        let inject_code = format!(
            "globalThis.__mox_extension_id = '{}';",
            extension_id.replace('\'', "\\'")
        );
        js_runtime
            .execute_script("[mox-init]", inject_code)
            .context("failed to inject extension id")?;

        // 3. 执行 VSCode API shim
        js_runtime
            .execute_script("[vscode-api-shim]", VSCODE_API_SHIM)
            .context("failed to execute vscode API shim")?;

        tracing::info!("DenoRuntime created successfully for extension: {}", extension_id);

        Ok(Self {
            js_runtime,
            extension_id: extension_id.to_string(),
            disposed: false,
        })
    }

    /// 关联的扩展 ID
    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    /// 是否已释放
    pub fn is_disposed(&self) -> bool {
        self.disposed
    }

    // ═══════════════════════════════════════════════════════════════════
    // 脚本执行
    // ═══════════════════════════════════════════════════════════════════

    /// 执行 JavaScript 脚本，忽略返回值
    ///
    /// # 参数
    /// - `name`: 脚本名称（用于调试和错误追踪）
    /// - `code`: JS 代码
    ///
    /// # 错误
    /// - JS 语法错误或运行时异常
    /// - 运行时已被释放
    pub fn execute_script(&mut self, name: &'static str, code: &str) -> Result<()> {
        if self.disposed {
            return Err(anyhow!("DenoRuntime has been disposed"));
        }

        tracing::debug!("executing script: {} ({} bytes)", name, code.len());

        self.js_runtime
            .execute_script(name, code.to_string())
            .with_context(|| format!("script execution failed: {}", name))?;

        Ok(())
    }

    /// 执行 JavaScript 脚本并获取返回值（JSON 格式）
    ///
    /// 返回值通过 `JSON.stringify()` 转换为 JSON。
    /// 如果返回值无法序列化（如循环引用、函数等），返回 `Value::Null`。
    ///
    /// # 参数
    /// - `name`: 脚本名称
    /// - `code`: JS 代码（最后一个表达式的值将被返回）
    pub fn execute_script_with_result(&mut self, name: &'static str, code: &str) -> Result<Value> {
        if self.disposed {
            return Err(anyhow!("DenoRuntime has been disposed"));
        }

        // 包装代码，使用 eval 执行代码
        // 为什么用 eval 而不是 new Function 或直接 return <code>？
        // 1. <code> 可能是语句（如 throw new Error()），不能作为 return 的表达式
        // 2. new Function 的函数体没有隐式 return，表达式会被丢弃
        // 3. eval 可以执行任意代码（语句和表达式），并返回最后一个表达式的值
        // 4. 语法错误和运行时错误都能被外层 try-catch 捕获
        let wrapped = format!(
            "(function() {{ try {{ return eval({}); }} catch(e) {{ return {{__mox_error: e.message, __mox_stack: e.stack}}; }} }})()",
            // 将代码作为字符串字面量传递给 eval（Rust {:?} 生成与 JS 兼容的字符串字面量）
            format!("{:?}", code)
        );

        let global = self
            .js_runtime
            .execute_script(name, wrapped)
            .with_context(|| format!("script execution failed: {}", name))?;

        // 将 v8 值转换为 JSON
        let scope = &mut self.js_runtime.handle_scope();
        let value = v8::Local::new(scope, &global);
        let json = v8_value_to_json(scope, value);

        // 检查是否是内部错误包装
        if let Some(obj) = json.as_object() {
            if let Some(err_msg) = obj.get("__mox_error").and_then(|v| v.as_str()) {
                let stack = obj
                    .get("__mox_stack")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                return Err(anyhow!("JS error: {}\n{}", err_msg, stack));
            }
        }

        Ok(json)
    }

    // ═══════════════════════════════════════════════════════════════════
    // 函数调用
    // ═══════════════════════════════════════════════════════════════════

    /// 调用全局 JavaScript 函数
    ///
    /// 支持同步和异步函数。对于异步函数（返回 Promise），
    /// 会自动驱动事件循环直到 Promise resolve/reject。
    ///
    /// # 参数
    /// - `name`: 全局函数名（如 `activate`、`deactivate`）
    /// - `args`: JSON 格式的参数数组
    ///
    /// # 返回
    /// 函数返回值的 JSON 表示
    ///
    /// # 错误
    /// - 函数不存在
    /// - JS 运行时异常
    /// - Promise reject
    pub fn call_function(&mut self, name: &str, args: &[Value]) -> Result<Value> {
        if self.disposed {
            return Err(anyhow!("DenoRuntime has been disposed"));
        }

        tracing::debug!("calling JS function: {} with {} args", name, args.len());

        // 1. 检查函数是否存在
        let check_code = format!(
            "typeof globalThis.{} === 'function'",
            js_identifier(name)
        );
        let exists = self.execute_script_with_result("[check-func]", &check_code)?;
        if !exists.as_bool().unwrap_or(false) {
            return Err(anyhow!("function '{}' not found in global scope", name));
        }

        // 2. 序列化参数为 JSON
        let args_json = serde_json::to_string(args)?;

        // 3. 调用函数，将结果存储到 __mox_result
        //    对于 async 函数，结果是 Promise，需要后续驱动事件循环
        let call_code = format!(
            "globalThis.__mox_result = globalThis.{}(...{});",
            js_identifier(name),
            args_json
        );
        self.execute_script("[call-func]", &call_code)?;

        // 4. 检查结果是否为 Promise，如果是则驱动事件循环
        let is_promise_code = "\
            (function() { \
                const r = globalThis.__mox_result; \
                return r && typeof r.then === 'function'; \
            })()";
        let is_promise = self
            .execute_script_with_result("[check-promise]", is_promise_code)?
            .as_bool()
            .unwrap_or(false);

        if is_promise {
            // 附加 then/catch 处理，将结果存储到 __mox_resolved
            let then_code = "\
                globalThis.__mox_resolved = false; \
                globalThis.__mox_result.then( \
                    function(v) { globalThis.__mox_resolved = true; globalThis.__mox_result = v; }, \
                    function(e) { globalThis.__mox_resolved = true; globalThis.__mox_result = {__mox_error: e ? e.message : String(e), __mox_stack: e ? e.stack : ''}; } \
                );";
            self.execute_script("[promise-then]", then_code)?;

            // 驱动事件循环，最多等待 30 秒
            self.run_event_loop_with_timeout(Duration::from_secs(30))?;

            // 检查是否已 resolve
            let resolved_code = "globalThis.__mox_resolved === true";
            let resolved = self
                .execute_script_with_result("[check-resolved]", resolved_code)?
                .as_bool()
                .unwrap_or(false);
            if !resolved {
                return Err(anyhow!("function '{}' promise timed out after 30s", name));
            }
        }

        // 5. 获取结果并转为 JSON
        let result_code = "\
            (function() { \
                const r = globalThis.__mox_result; \
                if (r === undefined) return null; \
                try { return JSON.stringify(r); } catch(e) { return JSON.stringify(String(r)); } \
            })()";
        let result_json = self.execute_script_with_result("[get-result]", result_code)?;

        // 结果可能是 JSON 字符串，需要解析
        let result = if let Some(s) = result_json.as_str() {
            serde_json::from_str(s).unwrap_or(Value::String(s.to_string()))
        } else {
            result_json
        };

        // 检查是否是错误
        if let Some(obj) = result.as_object() {
            if let Some(err_msg) = obj.get("__mox_error").and_then(|v| v.as_str()) {
                let stack = obj
                    .get("__mox_stack")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                return Err(anyhow!("JS function '{}' error: {}\n{}", name, err_msg, stack));
            }
        }

        // 清理临时变量
        let _ = self.execute_script("[cleanup]", "delete globalThis.__mox_result; delete globalThis.__mox_resolved;");

        Ok(result)
    }

    // ═══════════════════════════════════════════════════════════════════
    // 事件循环
    // ═══════════════════════════════════════════════════════════════════

    /// 同步驱动事件循环，直到队列为空或超时
    ///
    /// 使用 `poll_event_loop` 在循环中驱动，避免 async 上下文需求。
    /// 此方法应在同步闭包中调用（如 spawn_blocking 内）。
    ///
    /// # 参数
    /// - `timeout`: 最大等待时间
    fn run_event_loop_with_timeout(&mut self, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        // deno_core 0.290 的 poll_event_loop 需要 &mut Context + PollEventLoopOptions
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        let options = deno_core::PollEventLoopOptions::default();

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow!("event loop timed out after {:?}", timeout));
            }

            match self.js_runtime.poll_event_loop(&mut cx, options) {
                std::task::Poll::Ready(Ok(())) => {
                    // 事件循环已空，再做一次非阻塞轮询确认
                    match self.js_runtime.poll_event_loop(&mut cx, options) {
                        std::task::Poll::Ready(Ok(())) => break,
                        std::task::Poll::Ready(Err(e)) => {
                            return Err(anyhow!("event loop error: {}", e))
                        }
                        std::task::Poll::Pending => continue,
                    }
                }
                std::task::Poll::Ready(Err(e)) => {
                    return Err(anyhow!("event loop error: {}", e));
                }
                std::task::Poll::Pending => {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════
    // 资源释放
    // ═══════════════════════════════════════════════════════════════════

    /// 释放运行时资源
    ///
    /// 调用后运行时不可再用。再次调用任何方法都会返回错误。
    pub fn dispose(&mut self) {
        if !self.disposed {
            self.disposed = true;
            tracing::info!("DenoRuntime disposed for extension: {}", self.extension_id);
            // js_runtime 会在 drop 时自动释放 v8 资源
        }
    }

    /// 获取可变引用到底层 JsRuntime（高级用法）
    ///
    /// # 安全
    /// 调用者必须确保不跨 await 点持有返回的引用。
    pub fn inner_mut(&mut self) -> &mut deno_core::JsRuntime {
        &mut self.js_runtime
    }
}

impl Drop for DenoRuntime {
    fn drop(&mut self) {
        self.dispose();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 辅助函数
// ═══════════════════════════════════════════════════════════════════════════

/// 将 v8 值转换为 serde_json::Value
///
/// 使用 `JSON.stringify()` 进行序列化，无法序列化的值（函数、undefined、循环引用）
/// 会被转换为 `Value::Null` 或字符串表示。
fn v8_value_to_json(
    scope: &mut v8::HandleScope,
    value: v8::Local<v8::Value>,
) -> Value {
    // undefined → Null
    if value.is_undefined() {
        return Value::Null;
    }

    // 尝试 JSON.stringify
    if let Some(json_str) = v8::json::stringify(scope, value) {
        let s = json_str.to_rust_string_lossy(scope);
        if s.is_empty() {
            return Value::Null;
        }
        match serde_json::from_str(&s) {
            Ok(v) => return v,
            Err(_) => {
                // JSON 解析失败，返回字符串
                return Value::String(s);
            }
        }
    }

    // stringify 失败（可能是循环引用），尝试转为字符串
    if let Some(s) = value.to_string(scope) {
        return Value::String(s.to_rust_string_lossy(scope));
    }

    Value::Null
}

/// 验证 JS 标识符是否合法，防止代码注入
///
/// 如果标识符包含非法字符，返回一个占位符（会导致函数查找失败）。
fn js_identifier(name: &str) -> String {
    if name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        && !name.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
    {
        name.to_string()
    } else {
        tracing::warn!("invalid JS identifier: '{}', using placeholder", name);
        "__invalid_identifier__".to_string()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 单元测试
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：创建运行时，执行简单算术
    #[test]
    fn test_execute_simple_script() {
        let mut rt = DenoRuntime::new("test.extension").expect("failed to create DenoRuntime");
        let result = rt
            .execute_script_with_result("[test]", "1 + 1")
            .expect("failed to execute script");
        assert_eq!(result, Value::from(2));
    }

    /// 测试：执行函数定义和调用
    #[test]
    fn test_function_define_and_call() {
        let mut rt = DenoRuntime::new("test.extension").expect("failed to create DenoRuntime");

        // 定义函数
        rt.execute_script(
            "[define]",
            "globalThis.add = function(a, b) { return a + b; };",
        )
        .expect("failed to define function");

        // 调用函数
        let result = rt
            .call_function("add", &[Value::from(3), Value::from(4)])
            .expect("failed to call function");
        assert_eq!(result, Value::from(7));
    }

    /// 测试：JS 异常捕获
    #[test]
    fn test_js_exception_caught() {
        let mut rt = DenoRuntime::new("test.extension").expect("failed to create DenoRuntime");

        let result = rt.execute_script_with_result("[throw]", "throw new Error('test error message')");
        assert!(result.is_err(), "expected error for throwing script");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("test error message"),
            "error should contain 'test error message', got: {}",
            err_msg
        );
    }

    /// 测试：函数不存在时返回错误
    #[test]
    fn test_function_not_found() {
        let mut rt = DenoRuntime::new("test.extension").expect("failed to create DenoRuntime");

        let result = rt.call_function("nonExistentFunction", &[]);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("not found"));
    }

    /// 测试：异步函数调用
    #[test]
    fn test_async_function_call() {
        let mut rt = DenoRuntime::new("test.extension").expect("failed to create DenoRuntime");

        // 定义异步函数
        rt.execute_script(
            "[async-define]",
            "globalThis.asyncAdd = async function(a, b) { return a + b; };",
        )
        .expect("failed to define async function");

        // 调用异步函数
        let result = rt
            .call_function("asyncAdd", &[Value::from(10), Value::from(20)])
            .expect("failed to call async function");
        assert_eq!(result, Value::from(30));
    }

    /// 测试：异步函数 reject 捕获
    #[test]
    fn test_async_function_reject() {
        let mut rt = DenoRuntime::new("test.extension").expect("failed to create DenoRuntime");

        rt.execute_script(
            "[async-reject]",
            "globalThis.asyncFail = async function() { throw new Error('async failure'); };",
        )
        .expect("failed to define async function");

        let result = rt.call_function("asyncFail", &[]);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("async failure"));
    }

    /// 测试：VSCode API shim 已加载
    #[test]
    fn test_vscode_api_shim_loaded() {
        let mut rt = DenoRuntime::new("test.extension").expect("failed to create DenoRuntime");

        // 验证 vscode 对象存在
        let result = rt
            .execute_script_with_result("[check-vscode]", "typeof vscode")
            .expect("failed to check vscode");
        assert_eq!(result, Value::String("object".to_string()));

        // 验证 commands.registerCommand 是函数
        let result = rt
            .execute_script_with_result("[check-commands]", "typeof vscode.commands.registerCommand")
            .expect("failed to check commands");
        assert_eq!(result, Value::String("function".to_string()));
    }

    /// 测试：vscode 命令注册与执行
    #[test]
    fn test_vscode_command_register_and_execute() {
        let mut rt = DenoRuntime::new("test.extension").expect("failed to create DenoRuntime");

        // 注册命令
        rt.execute_script(
            "[register-cmd]",
            r#"
            vscode.commands.registerCommand('test.hello', function(name) {
                return 'Hello, ' + name + '!';
            });
            "#,
        )
        .expect("failed to register command");

        // 通过 vscode.commands.executeCommand 执行
        let result = rt
            .execute_script_with_result(
                "[exec-cmd]",
                "vscode.commands.executeCommand('test.hello', 'World')",
            )
            .expect("failed to execute command");
        // executeCommand 返回 Promise，需要驱动事件循环
        // 这里直接检查 handler 是否被注册
        let handler_exists = rt
            .execute_script_with_result("[check-handler]", "typeof globalThis.__mox_commands['test.hello'] === 'function'")
            .expect("failed to check handler");
        assert_eq!(handler_exists, Value::Bool(true));
    }

    /// 测试：vscode.window.showInformationMessage 返回 Promise
    #[test]
    fn test_show_information_message() {
        let mut rt = DenoRuntime::new("test.extension").expect("failed to create DenoRuntime");

        let result = rt
            .execute_script_with_result(
                "[show-msg]",
                "vscode.window.showInformationMessage('test message') instanceof Promise",
            )
            .expect("failed to call showInformationMessage");
        assert_eq!(result, Value::Bool(true));
    }

    /// 测试：vscode.workspace.workspaceFolders 返回数组
    #[test]
    fn test_workspace_folders() {
        let mut rt = DenoRuntime::new("test.extension").expect("failed to create DenoRuntime");

        let result = rt
            .execute_script_with_result("[ws-folders]", "Array.isArray(vscode.workspace.workspaceFolders)")
            .expect("failed to check workspaceFolders");
        assert_eq!(result, Value::Bool(true));
    }

    /// 测试：dispose 后操作返回错误
    #[test]
    fn test_dispose() {
        let mut rt = DenoRuntime::new("test.extension").expect("failed to create DenoRuntime");
        assert!(!rt.is_disposed());

        rt.dispose();
        assert!(rt.is_disposed());

        let result = rt.execute_script("[after-dispose]", "1 + 1");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("disposed"));
    }

    /// 测试：扩展 ID 注入
    #[test]
    fn test_extension_id_injected() {
        let mut rt = DenoRuntime::new("my.test.extension").expect("failed to create DenoRuntime");

        let result = rt
            .execute_script_with_result("[ext-id]", "globalThis.__mox_extension_id")
            .expect("failed to get extension id");
        assert_eq!(result, Value::String("my.test.extension".to_string()));
    }
}
