// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 多运行时抽象层 — 统一 WASM / VSCode 等插件运行时接口
//!
//! ## 架构
//! - [`Runtime`] — 统一插件运行时 trait（load/init/start/stop/call）
//! - [`RuntimeType`] — 运行时类型枚举（Wasm / VsCode）
//! - [`RuntimeHandle`] — 运行时句柄（实例ID + 状态 + 运行时内部数据）
//! - [`RuntimeRegistry`] — 运行时注册表（按类型注册/查找运行时实现）
//!
//! ## 阶段规划
//! - 阶段 1：骨架实现，WASM 封装现有加载逻辑，VSCode 仅状态转换
//! - 阶段 2：VSCode deno_core JsRuntime 集成 + vscode API shim
//! - 阶段 3：WASM 导出函数完整调用 + 跨运行时能力调度

pub mod vscode;
pub mod wasm;

use crate::lifecycle::{LifecycleError, PluginState};
use crate::manifest::PluginManifest;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

// ─── 运行时类型 ──────────────────────────────────────────────────────────────

/// 插件运行时类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeType {
    /// WebAssembly 运行时（wasmer）
    Wasm,
    /// VSCode 扩展运行时（deno_core，阶段 2 实现）
    VsCode,
}

impl RuntimeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeType::Wasm => "wasm",
            RuntimeType::VsCode => "vscode",
        }
    }
}

impl fmt::Display for RuntimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ─── 运行时内部数据 ──────────────────────────────────────────────────────────

/// 运行时内部数据（按类型区分）
///
/// - `Wasm(Option<wasmer::Instance>)`：WASM 运行时持有的 wasmer 实例，停止后为 None
/// - `VsCode`：VSCode 运行时占位，阶段 2 将持有 deno_core JsRuntime
pub enum RuntimeInternal {
    /// WASM 运行时内部：wasmer 实例（停止后释放为 None）
    Wasm(Option<wasmer::Instance>),
    /// VSCode 运行时内部：阶段 1 空占位，阶段 2 将持有 deno_core JsRuntime
    VsCode,
}

// ─── 运行时句柄 ──────────────────────────────────────────────────────────────

/// 运行时句柄 — 封装一个已加载插件实例的运行时状态
///
/// 每次 `Runtime::load` 调用返回一个新的句柄，后续 init/start/stop/call
/// 均通过句柄引用操作。句柄内部使用 `parking_lot::RwLock` 保证线程安全。
pub struct RuntimeHandle {
    /// 运行时类型
    runtime_type: RuntimeType,
    /// 实例唯一 ID（UUID v4）
    instance_id: String,
    /// 当前生命周期状态
    state: RwLock<PluginState>,
    /// 运行时内部数据（WASM 实例 / VSCode JsRuntime 占位）
    internal: RwLock<Option<RuntimeInternal>>,
    /// 关联的插件 manifest ID
    manifest_id: String,
}

impl RuntimeHandle {
    /// 创建新的运行时句柄（初始状态 = Loaded）
    pub fn new(
        runtime_type: RuntimeType,
        manifest_id: impl Into<String>,
        internal: Option<RuntimeInternal>,
    ) -> Self {
        Self {
            runtime_type,
            instance_id: uuid::Uuid::new_v4().to_string(),
            state: RwLock::new(PluginState::Loaded),
            internal: RwLock::new(internal),
            manifest_id: manifest_id.into(),
        }
    }

    /// 实例唯一 ID
    pub fn id(&self) -> &str {
        &self.instance_id
    }

    /// 运行时类型
    pub fn runtime_type(&self) -> RuntimeType {
        self.runtime_type
    }

    /// 关联的 manifest ID
    pub fn manifest_id(&self) -> &str {
        &self.manifest_id
    }

    /// 当前状态
    pub fn state(&self) -> PluginState {
        *self.state.read()
    }

    /// 安全状态转换（校验合法性，非法转换返回 LifecycleError）
    pub fn transition_to(&self, target: PluginState) -> Result<(), LifecycleError> {
        let mut state = self.state.write();
        if !state.can_transition_to(target) {
            return Err(LifecycleError::InvalidTransition {
                from: *state,
                to: target,
            });
        }
        tracing::debug!(
            "runtime handle {} state: {} -> {}",
            self.instance_id,
            state,
            target
        );
        *state = target;
        Ok(())
    }

    /// 通过闭包读取运行时内部数据（避免锁泄漏）
    pub fn with_internal<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Option<RuntimeInternal>) -> R,
    {
        f(&self.internal.read())
    }

    /// 通过闭包修改运行时内部数据（避免锁泄漏）
    pub fn with_internal_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Option<RuntimeInternal>) -> R,
    {
        f(&mut self.internal.write())
    }
}

// ─── 统一运行时 Trait ────────────────────────────────────────────────────────

/// 统一插件运行时接口
///
/// 所有插件运行时（WASM、VSCode 等）均需实现此 trait，
/// 由 [`RuntimeRegistry`] 统一管理和调度。
///
/// ## 生命周期
/// ```text
/// load()        init()        start()
/// Unloaded ──► Loaded ──► Initialized ──► Running
///                                          │
///                              stop()      ▼
///                                       Stopped
/// ```
#[async_trait::async_trait]
pub trait Runtime: Send + Sync {
    /// 加载插件：读取入口文件，编译/实例化，返回运行时句柄（状态=Loaded）
    async fn load(
        &self,
        manifest: &PluginManifest,
        entry: &Path,
    ) -> anyhow::Result<RuntimeHandle>;

    /// 初始化插件：调用插件的 init 导出函数，注册能力（状态=Initialized）
    async fn init(&self, handle: &RuntimeHandle) -> anyhow::Result<()>;

    /// 启动插件：插件进入运行状态，开始提供服务（状态=Running）
    async fn start(&self, handle: &RuntimeHandle) -> anyhow::Result<()>;

    /// 停止插件：停止服务，释放运行时资源（状态=Stopped）
    async fn stop(&self, handle: &RuntimeHandle) -> anyhow::Result<()>;

    /// 调用插件导出的方法
    ///
    /// # 参数
    /// - `method`：方法名
    /// - `args`：JSON 格式的参数
    ///
    /// # 返回
    /// JSON 格式的返回值
    async fn call(
        &self,
        handle: &RuntimeHandle,
        method: &str,
        args: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value>;

    /// 返回运行时类型
    fn runtime_type(&self) -> RuntimeType;
}

// ─── 运行时注册表 ────────────────────────────────────────────────────────────

/// 运行时注册表 — 按 [`RuntimeType`] 注册和查找运行时实现
///
/// 线程安全，内部使用 `RwLock<HashMap>`。
/// 支持同类型覆盖注册，后注册的实现会替换先注册的。
pub struct RuntimeRegistry {
    runtimes: RwLock<HashMap<RuntimeType, Arc<dyn Runtime>>>,
}

impl RuntimeRegistry {
    /// 创建空的运行时注册表
    pub fn new() -> Self {
        Self {
            runtimes: RwLock::new(HashMap::new()),
        }
    }

    /// 注册运行时实现（同类型会覆盖已有实现）
    pub fn register(&self, runtime: Arc<dyn Runtime>) {
        let rt = runtime.runtime_type();
        self.runtimes.write().insert(rt, runtime);
        tracing::info!("runtime registered: {}", rt);
    }

    /// 按类型获取运行时实现
    pub fn get(&self, runtime_type: RuntimeType) -> Option<Arc<dyn Runtime>> {
        self.runtimes.read().get(&runtime_type).cloned()
    }

    /// 列出所有已注册的运行时类型
    pub fn list_types(&self) -> Vec<RuntimeType> {
        self.runtimes.read().keys().copied().collect()
    }

    /// 检查指定类型的运行时是否已注册
    pub fn has(&self, runtime_type: RuntimeType) -> bool {
        self.runtimes.read().contains_key(&runtime_type)
    }

    /// 已注册运行时数量
    pub fn len(&self) -> usize {
        self.runtimes.read().len()
    }

    /// 注册表是否为空
    pub fn is_empty(&self) -> bool {
        self.runtimes.read().is_empty()
    }
}

impl Default for RuntimeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── 重导出 ──────────────────────────────────────────────────────────────────

pub use vscode::VsCodeRuntime;
pub use wasm::WasmRuntime;

// ─── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 构造测试用 PluginManifest
    fn test_manifest(id: &str, tags: Vec<String>) -> PluginManifest {
        PluginManifest {
            id: id.into(),
            name: "Test Plugin".into(),
            version: "1.0.0".into(),
            author: "test".into(),
            description: "test plugin".into(),
            entry: "plugin.wasm".into(),
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

    #[test]
    fn test_runtime_type_display() {
        assert_eq!(format!("{}", RuntimeType::Wasm), "wasm");
        assert_eq!(format!("{}", RuntimeType::VsCode), "vscode");
        assert_eq!(RuntimeType::Wasm.as_str(), "wasm");
        assert_eq!(RuntimeType::VsCode.as_str(), "vscode");
    }

    #[test]
    fn test_runtime_registry() {
        let registry = RuntimeRegistry::new();
        assert!(registry.is_empty());

        // 注册 WASM 运行时
        registry.register(Arc::new(WasmRuntime::new()));
        assert!(registry.has(RuntimeType::Wasm));
        assert!(!registry.has(RuntimeType::VsCode));
        assert_eq!(registry.len(), 1);

        // 注册 VSCode 运行时
        registry.register(Arc::new(VsCodeRuntime::new()));
        assert!(registry.has(RuntimeType::VsCode));
        assert_eq!(registry.len(), 2);

        // 查找
        let wasm_rt = registry.get(RuntimeType::Wasm);
        assert!(wasm_rt.is_some());
        assert_eq!(wasm_rt.unwrap().runtime_type(), RuntimeType::Wasm);

        let vscode_rt = registry.get(RuntimeType::VsCode);
        assert!(vscode_rt.is_some());
        assert_eq!(vscode_rt.unwrap().runtime_type(), RuntimeType::VsCode);

        // 列出类型
        let types = registry.list_types();
        assert_eq!(types.len(), 2);
        assert!(types.contains(&RuntimeType::Wasm));
        assert!(types.contains(&RuntimeType::VsCode));
    }

    #[tokio::test]
    async fn test_vscode_runtime_lifecycle() {
        let runtime = VsCodeRuntime::new();
        let manifest = test_manifest("test.vscode.plugin", vec!["runtime:vscode".into()]);
        let entry = PathBuf::from("./test-extension");

        // load
        let handle = runtime.load(&manifest, &entry).await.unwrap();
        assert_eq!(handle.state(), PluginState::Loaded);
        assert_eq!(handle.runtime_type(), RuntimeType::VsCode);
        assert_eq!(handle.manifest_id(), "test.vscode.plugin");
        assert!(!handle.id().is_empty());

        // init
        runtime.init(&handle).await.unwrap();
        assert_eq!(handle.state(), PluginState::Initialized);

        // start
        runtime.start(&handle).await.unwrap();
        assert_eq!(handle.state(), PluginState::Running);

        // stop
        runtime.stop(&handle).await.unwrap();
        assert_eq!(handle.state(), PluginState::Stopped);
    }

    #[tokio::test]
    async fn test_vscode_runtime_call_not_implemented() {
        let runtime = VsCodeRuntime::new();
        let manifest = test_manifest("test.vscode.call", vec!["runtime:vscode".into()]);
        let entry = PathBuf::from("./test-extension");
        let handle = runtime.load(&manifest, &entry).await.unwrap();

        let result = runtime
            .call(&handle, "someMethod", &serde_json::json!({}))
            .await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not implemented"),
            "error message should contain 'not implemented', got: {}",
            err_msg
        );
    }

    #[test]
    fn test_runtime_handle_state_transition() {
        let handle = RuntimeHandle::new(RuntimeType::Wasm, "test.handle", None);
        assert_eq!(handle.state(), PluginState::Loaded);

        // 合法转换链：Loaded -> Initialized -> Running -> Paused -> Running -> Stopped
        handle.transition_to(PluginState::Initialized).unwrap();
        assert_eq!(handle.state(), PluginState::Initialized);

        handle.transition_to(PluginState::Running).unwrap();
        assert_eq!(handle.state(), PluginState::Running);

        handle.transition_to(PluginState::Paused).unwrap();
        assert_eq!(handle.state(), PluginState::Paused);

        handle.transition_to(PluginState::Running).unwrap();
        assert_eq!(handle.state(), PluginState::Running);

        handle.transition_to(PluginState::Stopped).unwrap();
        assert_eq!(handle.state(), PluginState::Stopped);

        // 非法转换：Stopped -> Running（必须先 Unloaded -> Loaded）
        let result = handle.transition_to(PluginState::Running);
        assert!(result.is_err());
        match result.unwrap_err() {
            LifecycleError::InvalidTransition { from, to } => {
                assert_eq!(from, PluginState::Stopped);
                assert_eq!(to, PluginState::Running);
            }
            other => panic!("expected InvalidTransition, got: {:?}", other),
        }
    }
}
