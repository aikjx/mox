// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 插件系统（Plugin System）
//!
//! 参考 deepseek-harness / Cordis "Everything is a Plugin" 范式，
//! 为管线框架提供插件化扩展能力。
//!
//! # 设计要点
//!
//! - **无特权核心**：阶段处理器、钩子、审计桥接都可实现为插件
//! - **插件生命周期**：load → enable → disable → unload
//! - **扩展点（ExtensionPoint）**：命名扩展点，插件可注册处理器
//! - **服务注册表**：插件可向上下文贡献类型化服务
//! - **依赖顺序**：按 `depends_on` 声明保证装载顺序
//!
//! # 模块结构
//!
//! - [`Plugin`] trait：插件生命周期 + 元信息
//! - [`PluginMeta`]：插件元数据（id、版本、依赖等）
//! - [`PluginRegistry`]：插件注册表，管理生命周期
//! - [`PluginContext`]：插件装载时的上下文（服务注册、钩子注册）
//! - [`ExtensionPoint`]：命名扩展点，瀑布式处理器链

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::hooks::WaterfallHook;
use crate::phase::PhaseId;

// ================== PluginMeta ==================

/// 插件元信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginMeta {
    /// 插件唯一 ID（反向域名或路径风格，如 `mox.expert.normalize`）
    pub id: String,
    /// 插件名称（人类可读）
    pub name: String,
    /// 语义化版本
    pub version: String,
    /// 简短描述
    #[serde(default)]
    pub description: String,
    /// 依赖的其它插件 id 列表（装载顺序保证依赖先行）
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// 是否默认启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for PluginMeta {
    fn default() -> Self {
        Self {
            id: "anon".into(),
            name: "anonymous".into(),
            version: "0.0.0".into(),
            description: String::new(),
            depends_on: Vec::new(),
            enabled: true,
        }
    }
}

// ================== PluginError ==================

/// 插件操作错误
#[derive(Debug, Clone)]
pub enum PluginError {
    /// 插件未找到
    NotFound(String),
    /// 插件已存在（重复注册）
    AlreadyExists(String),
    /// 依赖缺失
    DependencyMissing { plugin: String, missing: String },
    /// 依赖循环
    CircularDependency(Vec<String>),
    /// 加载失败
    LoadFailed(String),
    /// 卸载失败
    UnloadFailed(String),
    /// 启用失败
    EnableFailed(String),
    /// 禁用失败
    DisableFailed(String),
    /// 插件已启用（重复启用）
    AlreadyEnabled(String),
    /// 插件已禁用（重复禁用）
    AlreadyDisabled(String),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "Plugin '{}' not found", id),
            Self::AlreadyExists(id) => write!(f, "Plugin '{}' already exists", id),
            Self::DependencyMissing { plugin, missing } => write!(
                f,
                "Plugin '{}' depends on '{}' which is not registered",
                plugin, missing
            ),
            Self::CircularDependency(chain) => {
                write!(f, "Circular dependency detected: {}", chain.join(" -> "))
            }
            Self::LoadFailed(msg) => write!(f, "Plugin load failed: {}", msg),
            Self::UnloadFailed(msg) => write!(f, "Plugin unload failed: {}", msg),
            Self::EnableFailed(msg) => write!(f, "Plugin enable failed: {}", msg),
            Self::DisableFailed(msg) => write!(f, "Plugin disable failed: {}", msg),
            Self::AlreadyEnabled(id) => write!(f, "Plugin '{}' is already enabled", id),
            Self::AlreadyDisabled(id) => write!(f, "Plugin '{}' is already disabled", id),
        }
    }
}

impl std::error::Error for PluginError {}

// ================== PluginContext ==================

/// 插件上下文：插件装载/卸载时与框架交互的接口
///
/// 插件通过此上下文：
/// - 注册类型化服务
/// - 注册钩子（pre/post pipeline / phase）
/// - 注册扩展点处理器
/// - 访问其它插件注册的服务
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
pub struct PluginContext<'a, P: PhaseId> {
    /// 服务注册表（TypeId -> Box<dyn Any + Send + Sync>）
    services: &'a mut HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    /// 钩子注册表（可变引用，插件可注册钩子）
    pub hooks: &'a mut crate::hooks::HookRegistry<P>,
    /// 扩展点注册表（插件可注册/查找扩展点）
    extension_points: &'a mut HashMap<String, Vec<ExtensionPoint<P>>>,
}

impl<'a, P: PhaseId> PluginContext<'a, P> {
    /// 创建插件上下文（内部使用）
    pub(crate) fn new(
        services: &'a mut HashMap<TypeId, Box<dyn Any + Send + Sync>>,
        hooks: &'a mut crate::hooks::HookRegistry<P>,
        extension_points: &'a mut HashMap<String, Vec<ExtensionPoint<P>>>,
    ) -> Self {
        Self {
            services,
            hooks,
            extension_points,
        }
    }

    /// 注册一个类型化服务
    pub fn provide_service<T: Any + Send + Sync + 'static>(&mut self, svc: T) {
        self.services
            .insert(TypeId::of::<T>(), Box::new(svc));
    }

    /// 获取一个类型化服务
    pub fn get_service<T: Any + Send + Sync + 'static>(&self) -> Option<&T> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>())
    }

    /// 注册一个扩展点处理器
    pub fn register_extension(&mut self, name: impl Into<String>, ext: ExtensionPoint<P>) {
        self.extension_points
            .entry(name.into())
            .or_default()
            .push(ext);
    }

    /// 获取指定名称的扩展点列表
    pub fn get_extensions(&self, name: &str) -> &[ExtensionPoint<P>] {
        self.extension_points
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

// ================== ExtensionPoint ==================

/// 扩展点：命名的扩展位点，插件可注册处理器
///
/// 扩展点是瀑布式的（类似钩子），多个处理器构成责任链。
/// 与钩子的区别：扩展点是业务级别的、由插件定义的扩展位点，
/// 而钩子是框架级别的、固定在管线生命周期中的扩展点。
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
pub struct ExtensionPoint<P: PhaseId> {
    /// 扩展点名称
    name: String,
    /// 所属插件 ID
    plugin_id: String,
    /// 处理器函数（瀑布模式）
    handler: WaterfallHook<P>,
    /// 优先级（越小越先执行）
    priority: i32,
}

impl<P: PhaseId> fmt::Debug for ExtensionPoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExtensionPoint")
            .field("name", &self.name)
            .field("plugin_id", &self.plugin_id)
            .field("priority", &self.priority)
            .finish()
    }
}

impl<P: PhaseId> ExtensionPoint<P> {
    /// 创建新的扩展点处理器
    pub fn new(
        name: impl Into<String>,
        plugin_id: impl Into<String>,
        handler: WaterfallHook<P>,
    ) -> Self {
        Self {
            name: name.into(),
            plugin_id: plugin_id.into(),
            handler,
            priority: 0,
        }
    }

    /// 设置优先级（越小越先执行，默认 0）
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// 扩展点名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 所属插件 ID
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// 优先级
    pub fn priority(&self) -> i32 {
        self.priority
    }

    /// 获取处理器
    pub fn handler(&self) -> &WaterfallHook<P> {
        &self.handler
    }
}

// ================== Plugin trait ==================

/// 插件 trait：管线框架的扩展单元
///
/// 所有扩展功能（阶段处理器、钩子、审计桥接、自定义服务等）
/// 都可通过实现此 trait 以插件形式装载。
///
/// # 生命周期
///
/// ```text
/// load → enable → (运行中) → disable → unload
/// ```
///
/// - `load`：注册服务、钩子、扩展点（声明式，不应有副作用）
/// - `enable`：启动后台任务、初始化资源（可有副作用）
/// - `disable`：停止后台任务、释放资源
/// - `unload`：清理所有痕迹
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
pub trait Plugin<P: PhaseId>: Send + Sync + fmt::Debug {
    /// 插件元信息
    fn meta(&self) -> &PluginMeta;

    /// 装载插件：向上下文注册服务、钩子、扩展点
    ///
    /// 此阶段应为声明式的（只注册，不执行副作用），
    /// 副作用应在 `enable` 中执行。
    fn load(&self, _ctx: &mut PluginContext<P>) -> Result<(), PluginError> {
        Ok(())
    }

    /// 启用插件：启动后台任务、初始化资源
    ///
    /// 异步版本，用于需要异步初始化的场景。
    /// 默认实现直接返回 Ok，以便纯同步插件不必实现。
    fn enable(&self, _ctx: &PluginContext<P>) -> Result<(), PluginError> {
        Ok(())
    }

    /// 禁用插件：停止后台任务、释放资源
    ///
    /// 默认实现直接返回 Ok，以便纯同步插件不必实现。
    fn disable(&self, _ctx: &PluginContext<P>) -> Result<(), PluginError> {
        Ok(())
    }

    /// 卸载插件：清理所有痕迹
    ///
    /// 默认实现直接返回 Ok，以便简单插件不必实现。
    fn unload(&self, _ctx: &mut PluginContext<P>) -> Result<(), PluginError> {
        Ok(())
    }
}

// ================== PluginRegistry ==================

/// 插件注册表：管理所有插件的生命周期
///
/// 负责：
/// - 插件注册与查找
/// - 按依赖顺序装载/卸载
/// - 启用/禁用插件
/// - 扩展点管理
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
pub struct PluginRegistry<P: PhaseId> {
    /// 已注册的插件（id -> plugin）
    plugins: HashMap<String, Arc<dyn Plugin<P>>>,
    /// 已装载的插件 id 列表（按装载顺序，用于逆序卸载）
    loaded: Vec<String>,
    /// 已启用的插件 id 集合
    enabled: HashMap<String, bool>,
    /// 共享服务注册表
    services: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    /// 钩子注册表
    hooks: crate::hooks::HookRegistry<P>,
    /// 扩展点注册表
    extension_points: HashMap<String, Vec<ExtensionPoint<P>>>,
}

impl<P: PhaseId> fmt::Debug for PluginRegistry<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("plugin_count", &self.plugins.len())
            .field("loaded_count", &self.loaded.len())
            .field("enabled_count", &self.enabled.len())
            .field("extension_point_count", &self.extension_points.len())
            .finish()
    }
}

impl<P: PhaseId> Default for PluginRegistry<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: PhaseId> PluginRegistry<P> {
    /// 创建空的插件注册表
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            loaded: Vec::new(),
            enabled: HashMap::new(),
            services: HashMap::new(),
            hooks: crate::hooks::HookRegistry::new(),
            extension_points: HashMap::new(),
        }
    }

    /// 注册一个插件（但不装载）
    pub fn register(&mut self, plugin: Arc<dyn Plugin<P>>) -> Result<(), PluginError> {
        let id = plugin.meta().id.clone();
        if self.plugins.contains_key(&id) {
            return Err(PluginError::AlreadyExists(id));
        }
        self.plugins.insert(id, plugin);
        Ok(())
    }

    /// 按依赖顺序装载所有已注册且启用的插件
    pub fn load_all(&mut self) -> Result<(), PluginError> {
        // 收集所有启用的插件 id
        let enabled_ids: Vec<String> = self
            .plugins
            .values()
            .filter(|p| p.meta().enabled)
            .map(|p| p.meta().id.clone())
            .collect();

        // 拓扑排序（按依赖顺序）
        let order = self.topological_sort(&enabled_ids)?;

        // 按顺序装载
        for id in &order {
            self.load_plugin(id)?;
        }

        Ok(())
    }

    /// 装载单个插件
    pub fn load_plugin(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        let plugin = self
            .plugins
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        if self.loaded.contains(&plugin.meta().id) {
            return Ok(()); // 已装载，跳过
        }

        // 确保依赖已装载
        for dep in &plugin.meta().depends_on {
            if !self.loaded.contains(dep) {
                self.load_plugin(dep)?;
            }
        }

        // 创建插件上下文并调用 load
        let mut ctx = PluginContext::new(
            &mut self.services,
            &mut self.hooks,
            &mut self.extension_points,
        );
        plugin.load(&mut ctx)?;

        self.loaded.push(plugin.meta().id.clone());
        Ok(())
    }

    /// 启用所有已装载的插件
    pub fn enable_all(&mut self) -> Result<(), PluginError> {
        let loaded = self.loaded.clone();
        for id in &loaded {
            self.enable_plugin(id)?;
        }
        Ok(())
    }

    /// 启用单个插件
    pub fn enable_plugin(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        if self.enabled.get(plugin_id).copied().unwrap_or(false) {
            return Err(PluginError::AlreadyEnabled(plugin_id.to_string()));
        }

        let plugin = self
            .plugins
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        let ctx = PluginContext::new(
            &mut self.services,
            &mut self.hooks,
            &mut self.extension_points,
        );
        plugin.enable(&ctx)?;

        self.enabled.insert(plugin_id.to_string(), true);
        Ok(())
    }

    /// 禁用所有已启用的插件（逆序）
    pub fn disable_all(&mut self) -> Result<(), PluginError> {
        let mut loaded = self.loaded.clone();
        loaded.reverse();
        for id in &loaded {
            if self.enabled.get(id).copied().unwrap_or(false) {
                self.disable_plugin(id)?;
            }
        }
        Ok(())
    }

    /// 禁用单个插件
    pub fn disable_plugin(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        if !self.enabled.get(plugin_id).copied().unwrap_or(false) {
            return Err(PluginError::AlreadyDisabled(plugin_id.to_string()));
        }

        let plugin = self
            .plugins
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        let ctx = PluginContext::new(
            &mut self.services,
            &mut self.hooks,
            &mut self.extension_points,
        );
        plugin.disable(&ctx)?;

        self.enabled.insert(plugin_id.to_string(), false);
        Ok(())
    }

    /// 卸载所有插件（逆序）
    pub fn unload_all(&mut self) -> Result<(), PluginError> {
        // 先禁用所有
        self.disable_all()?;

        // 逆序卸载
        let mut loaded = std::mem::take(&mut self.loaded);
        loaded.reverse();
        for id in &loaded {
            let plugin = self.plugins.get(id).cloned();
            if let Some(plugin) = plugin {
                let mut ctx = PluginContext::new(
                    &mut self.services,
                    &mut self.hooks,
                    &mut self.extension_points,
                );
                plugin.unload(&mut ctx)?;
            }
        }

        Ok(())
    }

    /// 查找插件
    pub fn get(&self, plugin_id: &str) -> Option<&dyn Plugin<P>> {
        self.plugins.get(plugin_id).map(|p| p.as_ref())
    }

    /// 获取钩子注册表（用于管线构建）
    pub fn hooks(&self) -> &crate::hooks::HookRegistry<P> {
        &self.hooks
    }

    /// 获取服务引用
    pub fn get_service<T: Any + Send + Sync + 'static>(&self) -> Option<&T> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>())
    }

    /// 获取指定名称的扩展点列表
    pub fn get_extensions(&self, name: &str) -> &[ExtensionPoint<P>] {
        self.extension_points
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 已注册插件数量
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// 已装载的插件 id 列表
    pub fn loaded_plugins(&self) -> &[String] {
        &self.loaded
    }

    // 拓扑排序：按依赖关系排序插件
    fn topological_sort(&self, ids: &[String]) -> Result<Vec<String>, PluginError> {
        let mut result = Vec::new();
        let mut visited = HashMap::new(); // id -> (0=unvisited, 1=visiting, 2=visited)

        for id in ids {
            visited.insert(id.clone(), 0);
        }

        for id in ids {
            self.toposort_visit(id, &mut visited, &mut result)?;
        }

        Ok(result)
    }

    fn toposort_visit(
        &self,
        id: &str,
        visited: &mut HashMap<String, u8>,
        result: &mut Vec<String>,
    ) -> Result<(), PluginError> {
        let status = visited.get(id).copied().unwrap_or(0);
        match status {
            1 => {
                // 发现循环（收集循环路径）
                let cycle: Vec<String> = visited
                    .iter()
                    .filter(|(_, v)| **v == 1)
                    .map(|(k, _)| k.clone())
                    .collect();
                return Err(PluginError::CircularDependency(cycle));
            }
            2 => return Ok(()),
            _ => {}
        }

        visited.insert(id.to_string(), 1);

        if let Some(plugin) = self.plugins.get(id) {
            for dep in &plugin.meta().depends_on {
                if !self.plugins.contains_key(dep) {
                    return Err(PluginError::DependencyMissing {
                        plugin: id.to_string(),
                        missing: dep.clone(),
                    });
                }
                self.toposort_visit(dep, visited, result)?;
            }
        }

        visited.insert(id.to_string(), 2);
        result.push(id.to_string());
        Ok(())
    }
}

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::NamedPhase;

    // 测试用插件
    #[derive(Debug)]
    struct TestPlugin {
        meta: PluginMeta,
        loaded: std::sync::atomic::AtomicBool,
        enabled: std::sync::atomic::AtomicBool,
    }

    impl TestPlugin {
        fn new(id: &str) -> Arc<Self> {
            Arc::new(Self {
                meta: PluginMeta {
                    id: id.into(),
                    name: id.into(),
                    version: "1.0.0".into(),
                    description: String::new(),
                    depends_on: Vec::new(),
                    enabled: true,
                },
                loaded: std::sync::atomic::AtomicBool::new(false),
                enabled: std::sync::atomic::AtomicBool::new(false),
            })
        }

        fn with_dep(self: Arc<Self>, dep: &str) -> Arc<Self> {
            let mut meta = self.meta.clone();
            meta.depends_on.push(dep.into());
            Arc::new(Self {
                meta,
                loaded: self.loaded.load(std::sync::atomic::Ordering::SeqCst).into(),
                enabled: self.enabled.load(std::sync::atomic::Ordering::SeqCst).into(),
            })
        }
    }

    impl Plugin<NamedPhase> for TestPlugin {
        fn meta(&self) -> &PluginMeta {
            &self.meta
        }

        fn load(&self, _ctx: &mut PluginContext<NamedPhase>) -> Result<(), PluginError> {
            self.loaded
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        fn enable(&self, _ctx: &PluginContext<NamedPhase>) -> Result<(), PluginError> {
            self.enabled
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        fn disable(&self, _ctx: &PluginContext<NamedPhase>) -> Result<(), PluginError> {
            self.enabled
                .store(false, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        fn unload(&self, _ctx: &mut PluginContext<NamedPhase>) -> Result<(), PluginError> {
            self.loaded
                .store(false, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn registry_starts_empty() {
        let registry = PluginRegistry::<NamedPhase>::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn register_plugin_works() {
        let mut registry = PluginRegistry::<NamedPhase>::new();
        let plugin = TestPlugin::new("test.plugin");

        registry.register(plugin.clone()).unwrap();
        assert_eq!(registry.len(), 1);
        assert!(registry.get("test.plugin").is_some());
    }

    #[test]
    fn register_duplicate_fails() {
        let mut registry = PluginRegistry::<NamedPhase>::new();
        let plugin = TestPlugin::new("test.plugin");

        registry.register(plugin.clone()).unwrap();
        let result = registry.register(plugin.clone());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PluginError::AlreadyExists(_)));
    }

    #[test]
    fn load_and_enable_plugin() {
        let mut registry = PluginRegistry::<NamedPhase>::new();
        let plugin = TestPlugin::new("test.plugin");

        registry.register(plugin.clone()).unwrap();
        registry.load_plugin("test.plugin").unwrap();

        assert!(plugin.loaded.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(registry.loaded_plugins().len(), 1);

        registry.enable_plugin("test.plugin").unwrap();
        assert!(plugin.enabled.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn disable_and_unload_plugin() {
        let mut registry = PluginRegistry::<NamedPhase>::new();
        let plugin = TestPlugin::new("test.plugin");

        registry.register(plugin.clone()).unwrap();
        registry.load_plugin("test.plugin").unwrap();
        registry.enable_plugin("test.plugin").unwrap();

        registry.disable_plugin("test.plugin").unwrap();
        assert!(!plugin.enabled.load(std::sync::atomic::Ordering::SeqCst));

        // 不能直接 unload 单个插件（unload_all 才是标准方式），
        // 但我们可以通过 unload_all 测试
        registry.unload_all().unwrap();
        assert!(!plugin.loaded.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn dependency_ordering() {
        let mut registry = PluginRegistry::<NamedPhase>::new();

        let a = TestPlugin::new("plugin.a");
        let b = TestPlugin::new("plugin.b").with_dep("plugin.a");
        let c = TestPlugin::new("plugin.c").with_dep("plugin.b");

        // 逆序注册
        registry.register(c.clone()).unwrap();
        registry.register(b.clone()).unwrap();
        registry.register(a.clone()).unwrap();

        registry.load_all().unwrap();

        let loaded = registry.loaded_plugins();
        assert_eq!(loaded.len(), 3);
        // 依赖顺序：a → b → c
        assert_eq!(loaded[0], "plugin.a");
        assert_eq!(loaded[1], "plugin.b");
        assert_eq!(loaded[2], "plugin.c");
    }

    #[test]
    fn circular_dependency_detected() {
        let mut registry = PluginRegistry::<NamedPhase>::new();

        // a 依赖 b，b 依赖 a — 循环
        let a = TestPlugin::new("plugin.a").with_dep("plugin.b");
        let b = TestPlugin::new("plugin.b").with_dep("plugin.a");

        registry.register(a).unwrap();
        registry.register(b).unwrap();

        let result = registry.load_all();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PluginError::CircularDependency(_)
        ));
    }

    #[test]
    fn missing_dependency_detected() {
        let mut registry = PluginRegistry::<NamedPhase>::new();

        // a 依赖不存在的插件
        let a = TestPlugin::new("plugin.a").with_dep("nonexistent");

        registry.register(a).unwrap();

        let result = registry.load_all();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PluginError::DependencyMissing { .. }
        ));
    }

    #[test]
    fn plugin_registers_service() {
        use crate::hooks::WaterfallHook;

        #[derive(Debug)]
        struct ServicePlugin {
            meta: PluginMeta,
        }

        impl Plugin<NamedPhase> for ServicePlugin {
            fn meta(&self) -> &PluginMeta {
                &self.meta
            }

            fn load(
                &self,
                ctx: &mut PluginContext<NamedPhase>,
            ) -> Result<(), PluginError> {
                ctx.provide_service(42i32);
                ctx.provide_service("hello".to_string());

                // 注册一个 pre_pipeline 钩子
                let hook: WaterfallHook<NamedPhase> =
                    Arc::new(|_event, ctx, next| {
                        ctx.set_bag("plugin_hook", true);
                        next(ctx)
                    });
                ctx.hooks.on_pre_pipeline(hook);

                Ok(())
            }
        }

        let mut registry = PluginRegistry::<NamedPhase>::new();
        let plugin = Arc::new(ServicePlugin {
            meta: PluginMeta {
                id: "service.plugin".into(),
                name: "Service Plugin".into(),
                version: "1.0.0".into(),
                ..Default::default()
            },
        });

        registry.register(plugin).unwrap();
        registry.load_all().unwrap();

        // 验证服务已注册
        assert_eq!(registry.get_service::<i32>(), Some(&42));
        assert_eq!(
            registry.get_service::<String>(),
            Some(&"hello".to_string())
        );

        // 验证钩子已注册
        assert_eq!(registry.hooks().pre_pipeline_count(), 1);
    }

    #[test]
    fn extension_points_work() {
        #[derive(Debug)]
        struct ExtPlugin {
            meta: PluginMeta,
        }

        impl Plugin<NamedPhase> for ExtPlugin {
            fn meta(&self) -> &PluginMeta {
                &self.meta
            }

            fn load(
                &self,
                ctx: &mut PluginContext<NamedPhase>,
            ) -> Result<(), PluginError> {
                let handler: WaterfallHook<NamedPhase> =
                    Arc::new(|_event, ctx, next| {
                        ctx.set_bag("ext_called", true);
                        next(ctx)
                    });
                let ext = ExtensionPoint::new("custom.point", &self.meta.id, handler);
                ctx.register_extension("custom.point", ext);
                Ok(())
            }
        }

        let mut registry = PluginRegistry::<NamedPhase>::new();
        let plugin = Arc::new(ExtPlugin {
            meta: PluginMeta {
                id: "ext.plugin".into(),
                name: "Extension Plugin".into(),
                version: "1.0.0".into(),
                ..Default::default()
            },
        });

        registry.register(plugin).unwrap();
        registry.load_all().unwrap();

        let extensions = registry.get_extensions("custom.point");
        assert_eq!(extensions.len(), 1);
        assert_eq!(extensions[0].plugin_id(), "ext.plugin");
        assert_eq!(extensions[0].name(), "custom.point");
    }

    #[test]
    fn plugin_error_display() {
        let err = PluginError::NotFound("test.plugin".into());
        assert!(format!("{}", err).contains("test.plugin"));
        assert!(format!("{}", err).contains("not found"));

        let err = PluginError::AlreadyExists("test.plugin".into());
        assert!(format!("{}", err).contains("already exists"));

        let err = PluginError::LoadFailed("some reason".into());
        assert!(format!("{}", err).contains("load failed"));
        assert!(format!("{}", err).contains("some reason"));
    }

    #[test]
    fn enable_already_enabled_fails() {
        let mut registry = PluginRegistry::<NamedPhase>::new();
        let plugin = TestPlugin::new("test.plugin");

        registry.register(plugin.clone()).unwrap();
        registry.load_plugin("test.plugin").unwrap();
        registry.enable_plugin("test.plugin").unwrap();

        let result = registry.enable_plugin("test.plugin");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PluginError::AlreadyEnabled(_)
        ));
    }
}
