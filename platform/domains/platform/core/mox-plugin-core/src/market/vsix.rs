// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! VSIX 市场支持模块
//!
//! ## 阶段划分
//! - **阶段 1（当前）**：提供 [`VsixPackageInfo`] 元数据结构和 [`VsixMarketplace`] 骨架，
//!   方法返回合理默认值或未实现错误。
//! - **阶段 2**：对接 [Open VSX Registry API](https://open-vsx.org/api)，
//!   实现真实的搜索、下载、安装流程。

// 阶段 2 将启用: use crate::loader::VsixLoader;
use crate::manifest::PluginManifest;
use serde::{Deserialize, Serialize};
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════════
// VsixPackageInfo — VSIX 包元数据
// ═══════════════════════════════════════════════════════════════════════════

/// VSIX 包元数据
///
/// 对应 Open VSX Registry 中的扩展信息，用于市场搜索结果展示和详情页。
///
/// ## 字段说明
/// - `id`: 扩展唯一标识，格式为 `publisher.name`（注意与 MOX manifest 中的
///   `vscode.{publisher}.{name}` 不同，此处不含 `vscode.` 前缀）
/// - `install_count`: 累计安装次数，用于排序和热度展示
/// - `rating`: 用户平均评分，范围 0.0 - 5.0
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsixPackageInfo {
    /// 扩展唯一标识，格式为 `publisher.name`
    pub id: String,
    /// 语义化版本号（如 "1.2.3"）
    pub version: String,
    /// 发布者名称
    pub publisher: String,
    /// 显示名称（human-readable）
    pub display_name: String,
    /// 扩展描述
    pub description: String,
    /// 分类标签（如 "Programming Languages", "Themes"）
    pub categories: Vec<String>,
    /// 累计安装次数
    pub install_count: u64,
    /// 用户平均评分（0.0 - 5.0）
    pub rating: f64,
}

// ═══════════════════════════════════════════════════════════════════════════
// VsixMarketplace — VSIX 市场封装（阶段 1 骨架）
// ═══════════════════════════════════════════════════════════════════════════

/// VSIX 市场封装
///
/// 提供 VSIX 扩展的搜索、安装和已安装列表管理能力。
///
/// ## 阶段 1（当前）
/// 所有远程交互方法均为骨架：
/// - [`search_vsix`](Self::search_vsix) 返回空 `Vec`
/// - [`install_vsix`](Self::install_vsix) 返回未实现错误
/// - [`list_installed`](Self::list_installed) 已实现本地扫描
///
/// ## 阶段 2 规划
/// 对接 Open VSX Registry API：
/// - 搜索：`GET /api/-/search?query={query}&size=20`
/// - 详情：`GET /api/{publisher}/{name}`
/// - 下载：`GET /api/{publisher}/{name}/{version}/file`
pub struct VsixMarketplace;

impl VsixMarketplace {
    /// 创建 VSIX 市场实例
    pub fn new() -> Self {
        Self
    }

    /// 搜索 VSIX 扩展包
    ///
    /// ## 阶段 1
    /// 返回空 `Vec`。
    ///
    /// ## 阶段 2 实现
    /// 对接 Open VSX Registry API：
    /// ```text
    /// GET https://open-vsx.org/api/-/search?query={query}&size=20
    /// ```
    /// 解析返回 JSON 中的 `extensions` 列表，逐项转换为 [`VsixPackageInfo`]。
    pub async fn search_vsix(&self, _query: &str) -> Result<Vec<VsixPackageInfo>, anyhow::Error> {
        // 阶段 2 实现: 对接 Open VSX Registry API
        // let url = format!("https://open-vsx.org/api/-/search?query={}", query);
        // let resp = reqwest::get(&url).await
        //     .map_err(|e| anyhow::anyhow!("search request failed: {}", e))?;
        // let json: serde_json::Value = resp.json().await
        //     .map_err(|e| anyhow::anyhow!("parse search response failed: {}", e))?;
        // ... 解析 extensions 列表为 Vec<VsixPackageInfo>
        Ok(Vec::new())
    }

    /// 安装指定版本的 VSIX 扩展到目标目录
    ///
    /// ## 阶段 1
    /// 返回 `Err(anyhow!("VSIX installation not implemented in phase 1"))`。
    ///
    /// ## 阶段 2 实现
    /// 从 Open VSX 下载 `.vsix` → [`VsixLoader::extract_vsix`] 解压 → 解析注册：
    /// ```text
    /// 1. GET /api/{publisher}/{name}/{version}/file  下载 .vsix 到临时文件
    /// 2. VsixLoader::extract_vsix(temp_vsix, dest_dir)  解压
    /// 3. 读取 dest_dir/extension/package.json
    /// 4. PluginManifest::from_vscode(&content)  转换为 MOX manifest
    /// 5. 返回 PluginManifest（由调用方注册到 PluginRegistry）
    /// ```
    pub async fn install_vsix(
        &self,
        _package_id: &str,
        _version: &str,
        _dest_dir: &Path,
    ) -> Result<PluginManifest, anyhow::Error> {
        // 阶段 2 实现: 从 Open VSX 下载 .vsix → VsixLoader::extract_vsix → 注册
        Err(anyhow::anyhow!("VSIX installation not implemented in phase 1"))
    }

    /// 列出已安装的所有插件（WASM 目录插件 + VSCode 扩展目录）
    ///
    /// 扫描 `plugin_dir` 下的每个子目录，按以下优先级识别：
    /// 1. **WASM 插件**：目录中包含 `manifest.json` → 直接解析为 [`PluginManifest`]
    /// 2. **VSCode 扩展**：目录中包含 `extension/package.json` →
    ///    调用 [`PluginManifest::from_vscode`] 转换
    ///
    /// 解析失败的目录会记录警告日志并跳过，不影响其他插件。
    ///
    /// ## 参数
    /// - `plugin_dir`: 插件根目录（通常与 [`PluginLoader`] 的 `plugin_dir` 一致）
    ///
    /// ## 返回
    /// 合并后的 [`PluginManifest`] 列表（WASM + VSCode）
    pub fn list_installed(&self, plugin_dir: &Path) -> Result<Vec<PluginManifest>, anyhow::Error> {
        let mut manifests = Vec::new();

        if !plugin_dir.exists() {
            tracing::warn!("plugin directory not found: {:?}", plugin_dir);
            return Ok(manifests);
        }

        let entries = std::fs::read_dir(plugin_dir)
            .map_err(|e| anyhow::anyhow!("failed to read plugin dir {:?}: {}", plugin_dir, e))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| anyhow::anyhow!("failed to read dir entry: {}", e))?;
            let path = entry.path();

            // 只处理子目录
            if !path.is_dir() {
                continue;
            }

            // 尝试 WASM 插件（manifest.json）
            let manifest_path = path.join("manifest.json");
            if manifest_path.exists() {
                let content = std::fs::read_to_string(&manifest_path)
                    .map_err(|e| anyhow::anyhow!("failed to read manifest.json in {:?}: {}", path, e))?;
                match PluginManifest::from_json(&content) {
                    Ok(m) => manifests.push(m),
                    Err(e) => tracing::warn!("failed to parse manifest.json in {:?}: {}", path, e),
                }
                continue;
            }

            // 尝试 VSCode 扩展（extension/package.json）
            let vscode_manifest_path = path.join("extension").join("package.json");
            if vscode_manifest_path.exists() {
                let content = std::fs::read_to_string(&vscode_manifest_path)
                    .map_err(|e| anyhow::anyhow!("failed to read extension/package.json in {:?}: {}", path, e))?;
                match PluginManifest::from_vscode(&content) {
                    Ok(m) => manifests.push(m),
                    Err(e) => tracing::warn!("failed to parse extension/package.json in {:?}: {}", path, e),
                }
            }
        }

        tracing::info!("listed {} installed plugins from {:?}", manifests.len(), plugin_dir);
        Ok(manifests)
    }
}

impl Default for VsixMarketplace {
    fn default() -> Self {
        Self::new()
    }
}
