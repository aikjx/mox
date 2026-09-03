// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Rust 宿主 Ops — 通过 deno_core 的 op 机制向 JavaScript 暴露宿主能力。
//!
//! ## 设计原则
//! - 所有需要宿主交互的 VSCode API（弹窗、输入框、输出通道等）均通过 op 调用 Rust 端
//! - 阶段 2：大部分 op 返回模拟数据或记录日志，不做真实 UI 交互
//! - 阶段 3：接入真实宿主 UI（通过 HostApi trait 回调）
//!
//! ## Op 命名规范
//! - 所有 op 名称以 `op_` 前缀开头
//! - 在 JS 中通过 `Deno.core.ops.op_xxx()` 调用
//!
//! ## 全局状态
//! - 命令注册表：记录已注册的命令 ID（实际 JS handler 存在 JS 全局作用域）
//! - 输出通道表：记录输出通道 ID → 名称映射

use deno_core::op2;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

// ═══════════════════════════════════════════════════════════════════════════
// 全局状态
// ═══════════════════════════════════════════════════════════════════════════

/// 全局命令注册表：command_id → extension_id
///
/// 注意：实际的 JS 函数 handler 存储在 JS 全局作用域的 `__mox_commands` 中，
/// 此处仅记录命令的注册关系，用于调试和跨运行时查找（阶段 3）。
static COMMAND_REGISTRY: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn command_registry() -> &'static Mutex<HashMap<String, String>> {
    COMMAND_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 全局输出通道表：channel_id → (name, visible)
static OUTPUT_CHANNELS: OnceLock<Mutex<HashMap<u32, OutputChannelInfo>>> = OnceLock::new();

/// 输出通道 ID 计数器
static OUTPUT_CHANNEL_ID: OnceLock<Mutex<u32>> = OnceLock::new();

fn next_channel_id() -> u32 {
    let counter = OUTPUT_CHANNEL_ID.get_or_init(|| Mutex::new(0));
    let mut id = counter.lock();
    *id += 1;
    *id
}

fn output_channels() -> &'static Mutex<HashMap<u32, OutputChannelInfo>> {
    OUTPUT_CHANNELS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 输出通道信息
#[derive(Debug, Clone)]
struct OutputChannelInfo {
    name: String,
    visible: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// 输入/输出类型定义
// ═══════════════════════════════════════════════════════════════════════════

/// showInputBox 的选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputBoxOptions {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub place_holder: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub password: bool,
    #[serde(default)]
    pub ignore_focus_out: bool,
}

/// showQuickPick 的选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickPickOptions {
    #[serde(default)]
    pub place_holder: Option<String>,
    #[serde(default)]
    pub match_on_description: bool,
    #[serde(default)]
    pub can_pick_many: bool,
    #[serde(default)]
    pub ignore_focus_out: bool,
}

/// 工作区文件夹信息（模拟）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFolderInfo {
    pub uri: String,
    pub name: String,
    pub index: u32,
}

/// 扩展元数据信息（模拟）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionInfo {
    pub id: String,
    pub extension_path: String,
    pub is_active: bool,
    pub package_json: serde_json::Value,
}

// ═══════════════════════════════════════════════════════════════════════════
// Window Ops — 窗口/UI 交互
// ═══════════════════════════════════════════════════════════════════════════

/// 显示信息消息框
///
/// 阶段 2：记录日志，返回第一个 item 或 null（模拟用户点击第一个选项）
#[op2]
#[serde]
fn op_show_information_message(
    #[string] msg: String,
    #[serde] items: Vec<String>,
) -> Result<Option<String>, anyhow::Error> {
    tracing::info!("[vscode window] showInformationMessage: {}", msg);
    if !items.is_empty() {
        tracing::info!("[vscode window]   items: {:?}", items);
        // 阶段 2 模拟：返回第一个选项
        Ok(Some(items[0].clone()))
    } else {
        Ok(None)
    }
}

/// 显示警告消息框
#[op2]
#[serde]
fn op_show_warning_message(
    #[string] msg: String,
    #[serde] items: Vec<String>,
) -> Result<Option<String>, anyhow::Error> {
    tracing::warn!("[vscode window] showWarningMessage: {}", msg);
    if !items.is_empty() {
        Ok(Some(items[0].clone()))
    } else {
        Ok(None)
    }
}

/// 显示错误消息框
#[op2]
#[serde]
fn op_show_error_message(
    #[string] msg: String,
    #[serde] items: Vec<String>,
) -> Result<Option<String>, anyhow::Error> {
    tracing::error!("[vscode window] showErrorMessage: {}", msg);
    if !items.is_empty() {
        Ok(Some(items[0].clone()))
    } else {
        Ok(None)
    }
}

/// 显示输入框
///
/// 阶段 2：返回空字符串（模拟用户未输入）
#[op2]
#[serde]
fn op_show_input_box(#[serde] options: InputBoxOptions) -> Result<String, anyhow::Error> {
    tracing::info!(
        "[vscode window] showInputBox: prompt={:?}, placeholder={:?}",
        options.prompt,
        options.place_holder
    );
    // 阶段 2 模拟：返回空字符串
    Ok(String::new())
}

/// 显示快速选择列表
///
/// 阶段 2：返回第一个 item（模拟用户选择第一个选项）
#[op2]
#[serde]
fn op_show_quick_pick(
    #[serde] items: Vec<String>,
    #[serde] options: QuickPickOptions,
) -> Result<Option<String>, anyhow::Error> {
    tracing::info!(
        "[vscode window] showQuickPick: {} items, placeholder={:?}",
        items.len(),
        options.place_holder
    );
    if !items.is_empty() {
        Ok(Some(items[0].clone()))
    } else {
        Ok(None)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Output Channel Ops — 输出通道
// ═══════════════════════════════════════════════════════════════════════════

/// 创建输出通道，返回通道 ID
#[op2]
fn op_create_output_channel(#[string] name: String) -> Result<u32, anyhow::Error> {
    let id = next_channel_id();
    output_channels().lock().insert(
        id,
        OutputChannelInfo {
            name: name.clone(),
            visible: false,
        },
    );
    tracing::info!("[vscode output] created channel #{}: {}", id, name);
    Ok(id)
}

/// 向输出通道追加文本
#[op2]
fn op_output_channel_append(channel_id: u32, #[string] text: String) -> Result<(), anyhow::Error> {
    let channels = output_channels().lock();
    if let Some(info) = channels.get(&channel_id) {
        // 阶段 2：通过 tracing 输出（实际应写入通道缓冲区）
        tracing::info!("[output:{}] {}", info.name, text);
    } else {
        tracing::warn!("[vscode output] channel #{} not found", channel_id);
    }
    Ok(())
}

/// 显示输出通道（将其带到前台）
#[op2]
fn op_output_channel_show(channel_id: u32) -> Result<(), anyhow::Error> {
    let mut channels = output_channels().lock();
    if let Some(info) = channels.get_mut(&channel_id) {
        info.visible = true;
        tracing::info!("[vscode output] showing channel #{}: {}", channel_id, info.name);
    } else {
        tracing::warn!("[vscode output] channel #{} not found", channel_id);
    }
    Ok(())
}

/// 隐藏输出通道
#[op2]
fn op_output_channel_hide(channel_id: u32) -> Result<(), anyhow::Error> {
    let mut channels = output_channels().lock();
    if let Some(info) = channels.get_mut(&channel_id) {
        info.visible = false;
    }
    Ok(())
}

/// 清空输出通道
#[op2]
fn op_output_channel_clear(channel_id: u32) -> Result<(), anyhow::Error> {
    tracing::info!("[vscode output] clearing channel #{}", channel_id);
    Ok(())
}

/// 释放输出通道
#[op2]
fn op_output_channel_dispose(channel_id: u32) -> Result<(), anyhow::Error> {
    output_channels().lock().remove(&channel_id);
    tracing::info!("[vscode output] disposed channel #{}", channel_id);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// Command Ops — 命令注册与执行
// ═══════════════════════════════════════════════════════════════════════════

/// 注册命令到全局命令表
///
/// 注意：实际的 JS handler 存储在 JS 全局作用域的 `__mox_commands` 中，
/// 此 op 仅在 Rust 端记录注册关系，用于调试和跨运行时查找。
#[op2]
fn op_register_command(
    #[string] extension_id: String,
    #[string] command_id: String,
) -> Result<(), anyhow::Error> {
    command_registry()
        .lock()
        .insert(command_id.clone(), extension_id.clone());
    tracing::info!(
        "[vscode commands] registered: {} (extension: {})",
        command_id,
        extension_id
    );
    Ok(())
}

/// 执行已注册命令（跨运行时）
///
/// 阶段 2：仅支持在当前 JS 运行时内执行（由 JS shim 直接调用 handler）。
/// 此 op 用于跨运行时命令执行，阶段 2 返回未实现。
/// 阶段 3：查找命令所属的 JsRuntime，通过该运行时调用 handler。
#[op2]
#[serde]
fn op_execute_command(
    #[string] command_id: String,
    #[serde] _args_json: serde_json::Value,
) -> Result<serde_json::Value, anyhow::Error> {
    let registry = command_registry().lock();
    if let Some(ext_id) = registry.get(&command_id) {
        tracing::info!(
            "[vscode commands] cross-runtime execute: {} (extension: {}) — stage 2 not implemented",
            command_id,
            ext_id
        );
        // 阶段 3：查找 ext_id 对应的 DenoRuntime，调用其 JS handler
        // 阶段 2：返回 null
        Ok(serde_json::Value::Null)
    } else {
        Err(anyhow::anyhow!("command not found: {}", command_id))
    }
}

/// 获取所有已注册命令 ID 列表
#[op2]
#[serde]
fn op_get_commands() -> Result<Vec<String>, anyhow::Error> {
    let commands: Vec<String> = command_registry().lock().keys().cloned().collect();
    Ok(commands)
}

// ═══════════════════════════════════════════════════════════════════════════
// Workspace Ops — 工作区
// ═══════════════════════════════════════════════════════════════════════════

/// 获取工作区文件夹列表
///
/// 阶段 2：返回空数组（无工作区）
#[op2]
#[serde]
fn op_get_workspace_folders() -> Result<Vec<WorkspaceFolderInfo>, anyhow::Error> {
    // 阶段 2：返回空数组
    Ok(Vec::new())
}

/// 获取工作区文件（如果有）
#[op2]
#[serde]
fn op_get_workspace_file() -> Result<Option<String>, anyhow::Error> {
    // 阶段 2：返回 None
    Ok(None)
}

/// 打开文本文档
///
/// 阶段 2：返回模拟的空文档
#[op2]
#[serde]
fn op_open_text_document(#[string] uri: String) -> Result<serde_json::Value, anyhow::Error> {
    tracing::info!("[vscode workspace] openTextDocument: {}", uri);
    // 阶段 2：返回模拟文档
    Ok(serde_json::json!({
        "uri": uri,
        "fileName": uri,
        "languageId": "plaintext",
        "version": 1,
        "isDirty": false,
        "isUntitled": false,
        "lineCount": 0,
        "getText": ""
    }))
}

/// 获取配置
///
/// 阶段 2：返回空对象
#[op2]
#[serde]
fn op_get_configuration(#[string] section: String) -> Result<serde_json::Value, anyhow::Error> {
    tracing::debug!("[vscode workspace] getConfiguration: {}", section);
    // 阶段 2：返回空对象
    Ok(serde_json::json!({}))
}

// ═══════════════════════════════════════════════════════════════════════════
// Extensions Ops — 扩展管理
// ═══════════════════════════════════════════════════════════════════════════

/// 获取指定扩展的元数据
///
/// 阶段 2：返回 null（不支持查询其他扩展）
#[op2]
#[serde]
fn op_get_extension(#[string] id: String) -> Result<Option<ExtensionInfo>, anyhow::Error> {
    tracing::debug!("[vscode extensions] getExtension: {}", id);
    // 阶段 2：返回 None
    Ok(None)
}

/// 获取所有已安装扩展列表
///
/// 阶段 2：返回空数组
#[op2]
#[serde]
fn op_get_all_extensions() -> Result<Vec<ExtensionInfo>, anyhow::Error> {
    // 阶段 2：返回空数组
    Ok(Vec::new())
}

// ═══════════════════════════════════════════════════════════════════════════
// 扩展构建器 — 注册所有 ops 到 deno_core Extension
// ═══════════════════════════════════════════════════════════════════════════

/// 构建包含所有宿主 ops 的 deno_core Extension
///
/// 此 Extension 在创建 JsRuntime 时加载，使 JS 代码可以通过
/// `Deno.core.ops.op_xxx()` 调用所有宿主能力。
pub fn build_host_extension() -> deno_core::Extension {
    deno_core::Extension::builder("mox_host")
        .ops(vec![
            // Window ops
            op_show_information_message::decl(),
            op_show_warning_message::decl(),
            op_show_error_message::decl(),
            op_show_input_box::decl(),
            op_show_quick_pick::decl(),
            // Output channel ops
            op_create_output_channel::decl(),
            op_output_channel_append::decl(),
            op_output_channel_show::decl(),
            op_output_channel_hide::decl(),
            op_output_channel_clear::decl(),
            op_output_channel_dispose::decl(),
            // Command ops
            op_register_command::decl(),
            op_execute_command::decl(),
            op_get_commands::decl(),
            // Workspace ops
            op_get_workspace_folders::decl(),
            op_get_workspace_file::decl(),
            op_open_text_document::decl(),
            op_get_configuration::decl(),
            // Extensions ops
            op_get_extension::decl(),
            op_get_all_extensions::decl(),
        ])
        .build()
}

// ═══════════════════════════════════════════════════════════════════════════
// 单元测试
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_box_options_deserialize() {
        let json = r#"{"prompt":"Enter name","placeHolder":"Name","password":true}"#;
        let opts: InputBoxOptions = serde_json::from_str(json).unwrap();
        assert_eq!(opts.prompt.as_deref(), Some("Enter name"));
        assert!(opts.password);
    }

    #[test]
    fn test_quick_pick_options_deserialize() {
        let json = r#"{"placeHolder":"Pick one","canPickMany":false}"#;
        let opts: QuickPickOptions = serde_json::from_str(json).unwrap();
        assert_eq!(opts.place_holder.as_deref(), Some("Pick one"));
        assert!(!opts.can_pick_many);
    }

    #[test]
    fn test_workspace_folder_info_serialize() {
        let info = WorkspaceFolderInfo {
            uri: "file:///workspace".to_string(),
            name: "workspace".to_string(),
            index: 0,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("file:///workspace"));
    }

    #[test]
    fn test_command_registry_global() {
        // 测试全局命令注册表可用
        let registry = command_registry();
        registry.lock().insert("test.cmd".to_string(), "test.ext".to_string());
        assert!(registry.lock().contains_key("test.cmd"));
        registry.lock().remove("test.cmd");
    }

    #[test]
    fn test_output_channel_id_increment() {
        let id1 = next_channel_id();
        let id2 = next_channel_id();
        assert_eq!(id2, id1 + 1);
    }
}
