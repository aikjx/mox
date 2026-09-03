// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! VSCode 扩展运行时 — 基于 deno_core 的 JavaScript 插件运行时（阶段 1 骨架）
//!
//! 支持加载 VSCode 扩展（.vsix 转换后的插件），在沙箱中运行 JavaScript/TypeScript。
//! 阶段 1 仅实现骨架：状态转换 + manifest 校验，不实际加载 JS。
//!
//! ## 阶段规划
//! - 阶段 1：骨架实现，load 校验 manifest tag，init/start/stop 状态转换，call 返回未实现
//! - 阶段 2：deno_core JsRuntime 初始化 + vscode API shim 注入 + 模块加载
//! - 阶段 3：完整 vscode API 兼容 + 扩展激活事件 + Webview 支持

use crate::lifecycle::PluginState;
use crate::manifest::PluginManifest;
use crate::runtime::{Runtime, RuntimeHandle, RuntimeInternal, RuntimeType};
use std::path::Path;

/// VSCode 扩展运行时
///
/// 阶段 1：空骨架，仅做 manifest 校验和状态转换。
/// 阶段 2 将集成 deno_core，创建 JsRuntime 并注入 vscode API shim，
/// 实现 VSCode 扩展的完整运行时支持。
pub struct VsCodeRuntime {
    // 阶段 2 可扩展：deno_core 配置、模块解析器、vscode API 版本等
}

impl VsCodeRuntime {
    /// 创建新的 VSCode 运行时
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for VsCodeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Runtime for VsCodeRuntime {
    /// 加载 VSCode 扩展插件：校验 manifest 包含 "runtime:vscode" tag
    ///
    /// 阶段 1 不实际加载 JS 文件，仅创建运行时句柄（状态=Loaded）。
    /// 阶段 2 将读取扩展入口 JS，创建 deno_core JsRuntime 并预加载模块。
    async fn load(
        &self,
        manifest: &PluginManifest,
        _entry: &Path,
    ) -> anyhow::Result<RuntimeHandle> {
        // 1. 校验 manifest 包含 runtime:vscode 标记
        if !manifest.tags.iter().any(|t| t == "runtime:vscode") {
            return Err(anyhow::anyhow!(
                "plugin {} is not a VSCode extension (missing 'runtime:vscode' tag, tags: {:?})",
                manifest.id,
                manifest.tags
            ));
        }

        tracing::info!("loading VSCode extension plugin: {}", manifest.id);

        // 阶段 2 实现: deno_core JsRuntime 初始化
        // let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        //     module_loader: Some(Rc::new(FsModuleLoader)),
        //     ..Default::default()
        // });
        // 阶段 2 实现: vscode API shim 注入
        // js_runtime.execute_script("[vscode-api-shim]", include_str!("vscode_api_shim.js"))?;

        // 2. 创建运行时句柄（状态=Loaded，内部为 VsCode 占位）
        let handle = RuntimeHandle::new(
            RuntimeType::VsCode,
            manifest.id.clone(),
            Some(RuntimeInternal::VsCode),
        );

        tracing::info!(
            "VSCode extension plugin loaded: {} (instance: {})",
            manifest.id,
            handle.id()
        );
        Ok(handle)
    }

    /// 初始化 VSCode 扩展：阶段 1 简化为状态转换
    ///
    /// 阶段 2 将调用扩展的 `activate` 函数，注册命令和事件监听器。
    async fn init(&self, handle: &RuntimeHandle) -> anyhow::Result<()> {
        // 阶段 2 实现: 调用扩展 activate() 函数
        // js_runtime.execute_script("[activate]", "globalThis.__activate()")?;

        handle
            .transition_to(PluginState::Initialized)
            .map_err(|e| anyhow::anyhow!("init state transition failed: {}", e))?;
        tracing::info!("VSCode extension plugin initialized: {}", handle.id());
        Ok(())
    }

    /// 启动 VSCode 扩展：阶段 1 简化为状态转换
    ///
    /// 阶段 2 将触发扩展的 `onStartup` 事件，开始处理命令和事件。
    async fn start(&self, handle: &RuntimeHandle) -> anyhow::Result<()> {
        // 阶段 2 实现: 触发扩展启动事件，开始处理命令队列

        handle
            .transition_to(PluginState::Running)
            .map_err(|e| anyhow::anyhow!("start state transition failed: {}", e))?;
        tracing::info!("VSCode extension plugin started: {}", handle.id());
        Ok(())
    }

    /// 停止 VSCode 扩展：阶段 1 简化为状态转换
    ///
    /// 阶段 2 将调用扩展的 `deactivate` 函数，释放 JsRuntime 资源。
    async fn stop(&self, handle: &RuntimeHandle) -> anyhow::Result<()> {
        // 阶段 2 实现: 调用扩展 deactivate() 函数，释放 JsRuntime
        // drop(js_runtime);

        handle
            .transition_to(PluginState::Stopped)
            .map_err(|e| anyhow::anyhow!("stop state transition failed: {}", e))?;
        tracing::info!("VSCode extension plugin stopped: {}", handle.id());
        Ok(())
    }

    /// 调用 VSCode 扩展方法：阶段 1 返回未实现错误
    ///
    /// 阶段 2 将通过 deno_core 调用 JS 函数，支持命令调用和事件触发。
    async fn call(
        &self,
        _handle: &RuntimeHandle,
        method: &str,
        _args: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        Err(anyhow::anyhow!(
            "VSCode runtime call not implemented in phase 1 (method: {})",
            method
        ))
    }

    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::VsCode
    }
}
