// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 插件加载器 — 从目录扫描、解析manifest、加载WASM模块、热重载
//!
//! ## VSIX 支持
//! 除传统的 WASM 目录插件外，加载器还支持 VSIX（VSCode 扩展）包。
//! VSIX 本质上是 ZIP 格式，内部包含 `extension/package.json`，
//! 通过 [`VsixLoader`] 解析并转换为 MOX [`PluginManifest`]。

use crate::lifecycle::{LifecycleError, LifecycleEvent, PluginState};
use crate::manifest::PluginManifest;
use crate::registry::PluginRegistry;
use parking_lot::RwLock;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use zip::ZipArchive;

/// 插件加载器
pub struct PluginLoader {
    registry: Arc<PluginRegistry>,
    /// 插件根目录
    plugin_dir: RwLock<PathBuf>,
    /// 是否启用热重载
    hot_reload: RwLock<bool>,
    /// 热重载检查间隔
    hot_reload_interval: RwLock<Duration>,
}

impl PluginLoader {
    pub fn new(registry: Arc<PluginRegistry>, plugin_dir: impl Into<PathBuf>) -> Self {
        Self {
            registry,
            plugin_dir: RwLock::new(plugin_dir.into()),
            hot_reload: RwLock::new(false),
            hot_reload_interval: RwLock::new(Duration::from_secs(10)),
        }
    }

    pub fn plugin_dir(&self) -> PathBuf {
        self.plugin_dir.read().clone()
    }

    pub fn set_plugin_dir(&self, dir: impl Into<PathBuf>) {
        *self.plugin_dir.write() = dir.into();
    }

    pub fn enable_hot_reload(&self, interval: Duration) {
        *self.hot_reload.write() = true;
        *self.hot_reload_interval.write() = interval;
    }

    pub fn disable_hot_reload(&self) {
        *self.hot_reload.write() = false;
    }

    /// 扫描插件目录，加载所有插件（WASM 目录插件 + VSIX 包）
    ///
    /// 扫描逻辑：
    /// - 子目录：按 WASM 插件处理（读取 manifest.json + 加载 WASM）
    /// - `.vsix` 文件：按 VSCode 扩展处理（解析 extension/package.json）
    ///
    /// 返回成功加载的插件总数。
    pub async fn load_all(&self) -> Result<usize, LifecycleError> {
        let dir = self.plugin_dir.read().clone();
        if !dir.exists() {
            tracing::warn!("plugin directory not found: {:?}", dir);
            return Ok(0);
        }

        let mut loaded = 0;
        let mut entries = tokio::fs::read_dir(&dir).await
            .map_err(|e| LifecycleError::LoadFailed(format!("read dir failed: {}", e)))?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                // 传统 WASM 目录插件
                match self.load_plugin_from_dir(&path).await {
                    Ok(_) => loaded += 1,
                    Err(e) => tracing::error!("failed to load plugin from {:?}: {}", path, e),
                }
            } else if VsixLoader::is_vsix(&path) {
                // VSIX 包（VSCode 扩展）
                match self.load_vsix_plugin(&path).await {
                    Ok(_) => loaded += 1,
                    Err(e) => tracing::error!("failed to load VSIX plugin from {:?}: {}", path, e),
                }
            }
        }

        tracing::info!("loaded {} plugins from {:?}", loaded, dir);
        Ok(loaded)
    }

    /// 从目录加载单个插件（WASM 插件）
    pub async fn load_plugin_from_dir(&self, dir: &Path) -> Result<Arc<crate::registry::PluginInstance>, LifecycleError> {
        // 1. 读取manifest.json
        let manifest_path = dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(LifecycleError::LoadFailed(format!("manifest.json not found in {:?}", dir)));
        }

        let manifest_content = tokio::fs::read_to_string(&manifest_path).await
            .map_err(|e| LifecycleError::LoadFailed(format!("read manifest failed: {}", e)))?;

        let manifest = PluginManifest::from_json(&manifest_content)
            .map_err(|e| LifecycleError::LoadFailed(format!("parse manifest failed: {}", e)))?;

        // 2. 检查WASM入口文件
        let wasm_path = dir.join(&manifest.entry);
        if !wasm_path.exists() {
            return Err(LifecycleError::LoadFailed(format!("WASM entry not found: {:?}", wasm_path)));
        }

        // 3. 检查依赖
        self.registry.check_dependencies(&manifest)
            .map_err(|e| LifecycleError::LoadFailed(format!("dependency check failed: {}", e)))?;

        // 4. 注册插件
        let instance = self.registry.register(manifest.clone())?;

        // 5. 加载WASM模块（异步，不阻塞）
        let wasm_path_clone = wasm_path.clone();
        let instance_clone = instance.clone();
        tokio::spawn(async move {
            match Self::load_wasm_module(&wasm_path_clone).await {
                Ok(wasm_module) => {
                    *instance_clone.wasm_instance.write() = Some(wasm_module);
                    tracing::info!("WASM module loaded for plugin {}", instance_clone.id());
                }
                Err(e) => {
                    instance_clone.set_error(format!("WASM load failed: {}", e));
                }
            }
        });

        // 6. 发送事件
        self.registry.emit_event(LifecycleEvent {
            plugin_id: instance.id().to_string(),
            plugin_name: instance.name().to_string(),
            from: PluginState::Unloaded,
            to: PluginState::Loaded,
            timestamp: chrono::Utc::now().timestamp(),
            reason: None,
        });

        Ok(instance)
    }

    /// 从 VSIX 包加载插件（仅注册 manifest，不加载 WASM 模块）
    ///
    /// VSIX 插件的运行时由宿主侧的 VSCode 兼容层负责，
    /// 此处只完成 manifest 解析、依赖检查、注册和事件通知。
    /// 从 VSIX 包加载插件（公开方法，供外部调用）
    ///
    /// VSIX 插件的运行时由宿主侧的 VSCode 兼容层负责，
    /// 此处只完成 manifest 解析、依赖检查、注册和事件通知。
    pub async fn load_vsix_plugin(&self, vsix_path: &Path) -> Result<Arc<crate::registry::PluginInstance>, LifecycleError> {
        // 1. 解析 VSIX 中的 extension/package.json → PluginManifest
        let manifest = VsixLoader::load_vsix(vsix_path)
            .map_err(|e| LifecycleError::LoadFailed(format!("parse VSIX failed: {}", e)))?;

        // 2. 检查依赖
        self.registry.check_dependencies(&manifest)
            .map_err(|e| LifecycleError::LoadFailed(format!("dependency check failed: {}", e)))?;

        // 3. 注册插件
        let instance = self.registry.register(manifest.clone())?;

        // 4. 发送 Loaded 事件（VSIX 插件无 WASM 模块，直接进入 Loaded 状态）
        self.registry.emit_event(LifecycleEvent {
            plugin_id: instance.id().to_string(),
            plugin_name: instance.name().to_string(),
            from: PluginState::Unloaded,
            to: PluginState::Loaded,
            timestamp: chrono::Utc::now().timestamp(),
            reason: None,
        });

        tracing::info!("VSIX plugin loaded: {} (from {:?})", instance.id(), vsix_path);
        Ok(instance)
    }

    /// 加载WASM模块
    async fn load_wasm_module(path: &Path) -> Result<wasmer::Instance, String> {
        let wasm_bytes = tokio::fs::read(path).await
            .map_err(|e| format!("read WASM file failed: {}", e))?;

        // 编译+实例化（CPU密集，全部放spawn_blocking）
        tokio::task::spawn_blocking(move || -> Result<wasmer::Instance, String> {
            // wasmer 4.x: 用From trait从compiler创建Engine，Store接受所有权
            let compiler = wasmer_compiler_cranelift::Cranelift::default();
            let engine = wasmer::Engine::from(compiler);
            let mut store = wasmer::Store::new(engine);

            let module = wasmer::Module::new(&store, &wasm_bytes)
                .map_err(|e| format!("compile WASM failed: {}", e))?;

            let import_object = wasmer::imports! {
                "env" => {
                    "host_log" => wasmer::Function::new_typed(
                        &mut store,
                        |_msg: i32| { /* plugin log placeholder */ }
                    ),
                }
            };

            let instance = wasmer::Instance::new(&mut store, &module, &import_object)
                .map_err(|e| format!("instantiate WASM failed: {}", e))?;

            Ok(instance)
        })
        .await
        .map_err(|e| format!("spawn_blocking failed: {}", e))?
    }

    /// 卸载插件
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<(), LifecycleError> {
        let instance = self.registry.get(plugin_id)?;
        // 停止
        if instance.current_state().is_active() {
            instance.transition_to(PluginState::Stopped)?;
        }
        // 释放WASM实例
        *instance.wasm_instance.write() = None;
        // 注销
        self.registry.unregister(plugin_id)?;
        tracing::info!("plugin unloaded: {}", plugin_id);
        Ok(())
    }

    /// 重新加载插件（热重载）
    pub async fn reload_plugin(&self, plugin_id: &str) -> Result<(), LifecycleError> {
        let dir = self.plugin_dir.read().join(plugin_id);
        self.unload_plugin(plugin_id).await?;
        self.load_plugin_from_dir(&dir).await?;
        tracing::info!("plugin reloaded: {}", plugin_id);
        Ok(())
    }

    /// 启动热重载监控（后台任务）
    pub fn start_hot_reload_watcher(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let interval = *self.hot_reload_interval.read();
            let mut ticker = tokio::time::interval(interval);
            tracing::info!("plugin hot-reload watcher started (interval: {:?})", interval);

            loop {
                ticker.tick().await;
                if !*self.hot_reload.read() { continue; }

                // 检查manifest文件变化（简化：比较修改时间）
                // 实际实现应使用notify crate监听文件系统事件
                let dir = self.plugin_dir.read().clone();
                if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let manifest_path = entry.path().join("manifest.json");
                        if manifest_path.exists() {
                            // 检查是否已加载且manifest有变化
                            // 简化：这里只打日志，实际实现需记录上次修改时间
                            tracing::trace!("hot-reload check: {:?}", manifest_path);
                        }
                    }
                }
            }
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// VsixLoader — VSIX 包加载器
// ═══════════════════════════════════════════════════════════════════════════

/// VSIX 包加载器
///
/// VSIX 是 VSCode 扩展的分发格式，本质上是一个 ZIP 压缩包，
/// 内部包含 `extension/` 目录，其中有 `package.json` 描述符。
///
/// ## 内部结构
/// ```text
/// extension.vsix (ZIP)
/// └── extension/
///     ├── package.json      # 扩展描述符（必需）
///     ├── extension.js      # 主入口（可选）
///     ├── README.md
///     └── ...
/// ```
pub struct VsixLoader;

impl VsixLoader {
    /// 从文件路径加载 VSIX 包，解析为 MOX [`PluginManifest`]
    ///
    /// 流程：打开 ZIP → 读取 `extension/package.json` →
    /// 调用 [`PluginManifest::from_vscode`] 转换。
    pub fn load_vsix(path: &Path) -> Result<PluginManifest, anyhow::Error> {
        let file = std::fs::File::open(path)
            .map_err(|e| anyhow::anyhow!("failed to open VSIX file {:?}: {}", path, e))?;
        Self::load_vsix_from_reader(file)
    }

    /// 从任意 `Read + Seek` 源加载 VSIX（便于测试时使用内存 `Cursor`）
    ///
    /// 这是 [`Self::load_vsix`] 的泛型版本，不依赖文件系统，
    /// 适合单元测试中用 `std::io::Cursor<Vec<u8>>` 构造内存 ZIP。
    pub fn load_vsix_from_reader<R: Read + Seek>(reader: R) -> Result<PluginManifest, anyhow::Error> {
        let mut archive = ZipArchive::new(reader)
            .map_err(|e| anyhow::anyhow!("failed to open ZIP archive: {}", e))?;

        // VSIX 内部结构：extension/package.json
        let mut package_json_file = archive.by_name("extension/package.json")
            .map_err(|e| anyhow::anyhow!("extension/package.json not found in VSIX: {}", e))?;

        let mut content = String::new();
        package_json_file.read_to_string(&mut content)
            .map_err(|e| anyhow::anyhow!("failed to read package.json: {}", e))?;

        // 转换为 MOX manifest（id 格式：vscode.{publisher}.{name}）
        let manifest = PluginManifest::from_vscode(&content)
            .map_err(|e| anyhow::anyhow!("failed to parse VSCode package.json: {}", e))?;

        Ok(manifest)
    }

    /// 解压 VSIX 包到目标目录
    ///
    /// 解压后 `dest_dir/extension/` 包含 `package.json` 等扩展文件。
    ///
    /// ## 参数
    /// - `vsix_path`: VSIX 文件路径
    /// - `dest_dir`: 解压目标目录（不存在则自动创建）
    ///
    /// ## 注意
    /// - 会处理 ZIP 中的目录条目（以 `/` 结尾的名称）
    /// - 自动创建所有必要的父目录
    pub fn extract_vsix(vsix_path: &Path, dest_dir: &Path) -> Result<(), anyhow::Error> {
        let file = std::fs::File::open(vsix_path)
            .map_err(|e| anyhow::anyhow!("failed to open VSIX file {:?}: {}", vsix_path, e))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| anyhow::anyhow!("failed to open ZIP archive: {}", e))?;

        // 创建目标目录
        std::fs::create_dir_all(dest_dir)
            .map_err(|e| anyhow::anyhow!("failed to create dest dir {:?}: {}", dest_dir, e))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| anyhow::anyhow!("failed to read ZIP entry {}: {}", i, e))?;
            let entry_name = file.name().to_string();
            let outpath = dest_dir.join(&entry_name);

            // 处理目录条目（以 / 结尾的名称）
            if entry_name.ends_with('/') {
                std::fs::create_dir_all(&outpath)
                    .map_err(|e| anyhow::anyhow!("failed to create dir {:?}: {}", outpath, e))?;
            } else {
                // 确保父目录存在
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| anyhow::anyhow!("failed to create parent dir {:?}: {}", parent, e))?;
                }
                // 写入文件内容
                let mut outfile = std::fs::File::create(&outpath)
                    .map_err(|e| anyhow::anyhow!("failed to create file {:?}: {}", outpath, e))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| anyhow::anyhow!("failed to write file {:?}: {}", outpath, e))?;
            }
        }

        tracing::info!("VSIX extracted to {:?}", dest_dir);
        Ok(())
    }

    /// 检查文件是否为 VSIX 包（基于扩展名，大小写不敏感）
    pub fn is_vsix(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("vsix"))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_load_all_empty_dir() {
        let registry = Arc::new(PluginRegistry::new());
        let temp_dir = std::env::temp_dir().join("mox_plugin_test_empty");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let loader = PluginLoader::new(registry, temp_dir.clone());
        let count = loader.load_all().await.unwrap();
        assert_eq!(count, 0);
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    // ── VsixLoader 单元测试 ──────────────────────────────────────────────

    /// 测试：用内存中的 ZIP（包含 extension/package.json）解析 VSIX
    #[test]
    fn test_vsix_loader_parse_memory_zip() {
        // 1. 使用 zip crate 的 ZipWriter 创建内存 ZIP
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(cursor);
            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("extension/package.json", options).unwrap();

            // 写入简化的 VSCode package.json
            let package_json = r#"{
                "name": "test-extension",
                "version": "1.0.0",
                "publisher": "test-publisher",
                "displayName": "Test Extension",
                "description": "A test VSCode extension for unit testing",
                "engines": {"vscode": "^1.80.0"},
                "categories": ["Programming Languages"]
            }"#;
            zip.write_all(package_json.as_bytes()).unwrap();
            zip.finish().unwrap();
        }

        // 2. 用 std::io::Cursor<Vec<u8>> 包装，调用 load_vsix_from_reader
        let cursor = std::io::Cursor::new(buf);
        let manifest = VsixLoader::load_vsix_from_reader(cursor)
            .expect("failed to load VSIX from memory");

        // 3. 验证解析出的 PluginManifest 正确
        // id 格式：vscode.{publisher}.{name}
        assert_eq!(manifest.id, "vscode.test-publisher.test-extension");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.author, "test-publisher");
        assert_eq!(manifest.name, "Test Extension");
        assert_eq!(manifest.description, "A test VSCode extension for unit testing");
        // 验证 tags 中包含 runtime:vscode 标记
        assert!(
            manifest.tags.contains(&"runtime:vscode".to_string()),
            "expected runtime:vscode tag, got: {:?}",
            manifest.tags
        );
    }

    /// 测试：.vsix 扩展名识别
    #[test]
    fn test_is_vsix() {
        // 正例
        assert!(VsixLoader::is_vsix(Path::new("plugin.vsix")));
        assert!(VsixLoader::is_vsix(Path::new("/path/to/extension.VSIX")));
        assert!(VsixLoader::is_vsix(Path::new("/path/to/extension.Vsix")));

        // 反例
        assert!(!VsixLoader::is_vsix(Path::new("plugin.zip")));
        assert!(!VsixLoader::is_vsix(Path::new("plugin.vsix.backup")));
        assert!(!VsixLoader::is_vsix(Path::new("no_extension")));
        assert!(!VsixLoader::is_vsix(Path::new("plugin.vsixx")));
    }

    /// 测试：无效文件返回错误
    #[test]
    fn test_vsix_loader_invalid_file() {
        // 场景 1：完全无效的字节流（不是 ZIP）
        let invalid_data = std::io::Cursor::new(vec![0u8, 1, 2, 3, 4, 5]);
        let result = VsixLoader::load_vsix_from_reader(invalid_data);
        assert!(result.is_err(), "expected error for invalid ZIP data");

        // 场景 2：有效的 ZIP 但没有 extension/package.json
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(cursor);
            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("readme.txt", options).unwrap();
            zip.write_all(b"hello world").unwrap();
            zip.finish().unwrap();
        }
        let cursor = std::io::Cursor::new(buf);
        let result = VsixLoader::load_vsix_from_reader(cursor);
        assert!(result.is_err(), "expected error for ZIP without extension/package.json");

        // 场景 3：ZIP 中有 extension/package.json 但内容不是合法 JSON
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(cursor);
            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("extension/package.json", options).unwrap();
            zip.write_all(b"this is not json {{{").unwrap();
            zip.finish().unwrap();
        }
        let cursor = std::io::Cursor::new(buf);
        let result = VsixLoader::load_vsix_from_reader(cursor);
        assert!(result.is_err(), "expected error for invalid package.json content");
    }
}
