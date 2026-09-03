// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! WASM 运行时 — 基于 wasmer 的 WebAssembly 插件运行时实现
//!
//! 封装现有 WASM 加载逻辑（参考 `loader.rs` 中的 `load_wasm_module`），
//! 实现 [`Runtime`] trait，提供统一的 load/init/start/stop/call 接口。
//!
//! ## 阶段规划
//! - 阶段 1：封装加载逻辑，init/start/stop 为状态转换，call 简化
//! - 阶段 2：实现 WASM 导出函数完整调用（内存交互 + 参数序列化）
//! - 阶段 3：WASI 支持 + 宿主 API 完整注入

use crate::lifecycle::PluginState;
use crate::manifest::PluginManifest;
use crate::runtime::{Runtime, RuntimeHandle, RuntimeInternal, RuntimeType};
use std::path::Path;

/// WASM 运行时
///
/// 使用 wasmer + wasmer-compiler-cranelift 编译和实例化 WASM 模块。
/// 编译和实例化为 CPU 密集操作，全部在 `tokio::task::spawn_blocking` 中执行，
/// 避免阻塞异步运行时。
pub struct WasmRuntime {
    // 阶段 2 可扩展：引擎配置、模块缓存、预编译策略等
}

impl WasmRuntime {
    /// 创建新的 WASM 运行时
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Runtime for WasmRuntime {
    /// 加载 WASM 插件：读取 .wasm 文件，编译+实例化，返回运行时句柄
    ///
    /// 编译和实例化在 `spawn_blocking` 中执行，避免阻塞异步运行时。
    /// 加载成功后句柄状态为 `Loaded`，内部持有 wasmer::Instance。
    async fn load(
        &self,
        manifest: &PluginManifest,
        entry: &Path,
    ) -> anyhow::Result<RuntimeHandle> {
        // 1. 读取 WASM 文件
        let wasm_bytes = tokio::fs::read(entry)
            .await
            .map_err(|e| anyhow::anyhow!("read WASM file failed: {}", e))?;

        tracing::info!(
            "loading WASM plugin: {} ({} bytes)",
            manifest.id,
            wasm_bytes.len()
        );

        // 2. 编译+实例化（CPU 密集，放 spawn_blocking）
        let instance = tokio::task::spawn_blocking(
            move || -> anyhow::Result<wasmer::Instance> {
                // wasmer 4.x: 用 From trait 从 compiler 创建 Engine，Store 接受所有权
                let compiler = wasmer_compiler_cranelift::Cranelift::default();
                let engine = wasmer::Engine::from(compiler);
                let mut store = wasmer::Store::new(engine);

                let module = wasmer::Module::new(&store, &wasm_bytes)
                    .map_err(|e| anyhow::anyhow!("compile WASM failed: {}", e))?;

                // 阶段 1：基础 import 对象（host_log 占位）
                // 阶段 2：注入完整宿主 API（host_api.rs 中的能力）
                let import_object = wasmer::imports! {
                    "env" => {
                        "host_log" => wasmer::Function::new_typed(
                            &mut store,
                            |_msg: i32| { /* 阶段 1：插件日志占位 */ }
                        ),
                    }
                };

                let instance = wasmer::Instance::new(&mut store, &module, &import_object)
                    .map_err(|e| anyhow::anyhow!("instantiate WASM failed: {}", e))?;

                Ok(instance)
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking failed: {}", e))??;

        // 3. 创建运行时句柄（状态=Loaded，内部持有 wasmer 实例）
        let handle = RuntimeHandle::new(
            RuntimeType::Wasm,
            manifest.id.clone(),
            Some(RuntimeInternal::Wasm(Some(instance))),
        );

        tracing::info!(
            "WASM plugin loaded: {} (instance: {})",
            manifest.id,
            handle.id()
        );
        Ok(handle)
    }

    /// 初始化 WASM 插件：阶段 1 简化为状态转换
    ///
    /// 阶段 2 将调用 WASM 模块的 `init` 导出函数，完成能力注册和资源初始化。
    async fn init(&self, handle: &RuntimeHandle) -> anyhow::Result<()> {
        // 阶段 2 实现: 调用 WASM 导出的 init 函数
        // let init_func = instance.exports.get_function("init")?;
        // init_func.call(&mut store, &[])?;

        handle
            .transition_to(PluginState::Initialized)
            .map_err(|e| anyhow::anyhow!("init state transition failed: {}", e))?;
        tracing::info!("WASM plugin initialized: {}", handle.id());
        Ok(())
    }

    /// 启动 WASM 插件：阶段 1 简化为状态转换
    ///
    /// 阶段 2 将调用 WASM 模块的 `start` 导出函数，插件开始正常提供服务。
    async fn start(&self, handle: &RuntimeHandle) -> anyhow::Result<()> {
        // 阶段 2 实现: 调用 WASM 导出的 start 函数

        handle
            .transition_to(PluginState::Running)
            .map_err(|e| anyhow::anyhow!("start state transition failed: {}", e))?;
        tracing::info!("WASM plugin started: {}", handle.id());
        Ok(())
    }

    /// 停止 WASM 插件：状态转换到 Stopped，释放 WASM 实例
    ///
    /// 停止后将内部 wasmer::Instance 置为 None，释放相关资源。
    async fn stop(&self, handle: &RuntimeHandle) -> anyhow::Result<()> {
        handle
            .transition_to(PluginState::Stopped)
            .map_err(|e| anyhow::anyhow!("stop state transition failed: {}", e))?;

        // 释放 WASM 实例（将内部实例置为 None）
        handle.with_internal_mut(|internal| {
            if let Some(RuntimeInternal::Wasm(instance_opt)) = internal {
                *instance_opt = None;
            }
        });

        tracing::info!(
            "WASM plugin stopped and instance released: {}",
            handle.id()
        );
        Ok(())
    }

    /// 调用 WASM 导出函数：阶段 1 简化实现
    ///
    /// 阶段 2 将实现完整的 WASM 函数调用流程：
    /// 1. 从 handle.internal 获取 wasmer::Instance 和 Store
    /// 2. 查找导出函数: `instance.exports.get_function(method)`
    /// 3. 参数序列化到 WASM 线性内存
    /// 4. 调用函数并读取返回值
    /// 5. 反序列化为 serde_json::Value
    async fn call(
        &self,
        handle: &RuntimeHandle,
        method: &str,
        _args: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        Err(anyhow::anyhow!(
            "WASM runtime call not fully implemented in phase 1 (method: {}, instance: {})",
            method,
            handle.id()
        ))
    }

    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::Wasm
    }
}
