// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Rust 宿主 Ops — 通过 deno_core 的 op 机制向 JS 暴露宿主能力
//!
//! 所有需要宿主交互的 VSCode API（消息框、输入框、命令注册等）均通过
//! deno_core op 调用 Rust 端实现。阶段 2 大部分返回模拟数据。
//!
//! ## 设计说明
//! - 复杂类型（数组、对象）通过 JSON 字符串在 JS/Rust 间传递
//! - 简单类型（String、u32、()）直接传递
//! - 全局状态使用 OnceLock<Mutex<HashMap>> 管理
//! - 阶段 3：将模拟数据替换为真实宿主回调

use deno_core::op2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

// ═══════════════════════════════════════════════════════════════════════════
// 全局状态
// ═══════════════════════════════════════════════════════════════════════════

/// 命令注册表：command_id -> extension_id
static COMMAND_REGISTRY: OnceLock<parking_lot::Mutex<HashMap<String, String>>> = OnceLock::new();

fn command_registry() -> &'static parking_lot::Mutex<HashMap<String, String>> {
    COMMAND_REGISTRY.get_or_init(|| parking_lot::Mutex::new(HashMap::new()))
}

/// 输出通道表：channel_id -> name
static OUTPUT_CHANNELS: OnceLock<parking_lot::Mutex<HashMap<u32, String>>> = OnceLock::new();

fn output_channels() -> &'static parking_lot::Mutex<HashMap<u32, String>> {
    OUTPUT_CHANNELS.get_or_init(|| parking_lot::Mutex::new(HashMap::new()))
}

/// 下一个输出通道 ID
static NEXT_CHANNEL_ID: OnceLock<parking_lot::Mutex<u32>> = OnceLock::new();

fn next_channel_id() -> u32 {
    let mut counter = NEXT_CHANNEL_ID.get_or_init(|| parking_lot::Mutex::new(1)).lock();
    let id = *counter;
    *counter += 1;
    id
}

// ═══════════════════════════════════════════════════════════════════════════
// 数据结构
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFolderInfo {
    pub uri: String,
    pub name: String,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionInfo {
    pub id: String,
    pub extension_path: String,
    pub is_active: bool,
    pub package_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentInfo {
    pub uri: String,
    pub language_id: String,
    pub text: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// 消息框 Ops
// ═══════════════════════════════════════════════════════════════════════════

#[op2]
#[string]
fn op_show_information_message(
    #[string] msg: String,
    #[string] items_json: String,
) -> Result<String, anyhow::Error> {
    tracing::info!("[vscode window] showInformationMessage: {}", msg);
    let items: Vec<String> = serde_json::from_str(&items_json).unwrap_or_default();
    Ok(items.first().cloned().unwrap_or_default())
}

#[op2]
#[string]
fn op_show_warning_message(
    #[string] msg: String,
    #[string] items_json: String,
) -> Result<String, anyhow::Error> {
    tracing::warn!("[vscode window] showWarningMessage: {}", msg);
    let items: Vec<String> = serde_json::from_str(&items_json).unwrap_or_default();
    Ok(items.first().cloned().unwrap_or_default())
}

#[op2]
#[string]
fn op_show_error_message(
    #[string] msg: String,
    #[string] items_json: String,
) -> Result<String, anyhow::Error> {
    tracing::error!("[vscode window] showErrorMessage: {}", msg);
    let items: Vec<String> = serde_json::from_str(&items_json).unwrap_or_default();
    Ok(items.first().cloned().unwrap_or_default())
}

// ═══════════════════════════════════════════════════════════════════════════
// 输入框 / 快速选择 Ops
// ═══════════════════════════════════════════════════════════════════════════

#[op2]
#[string]
fn op_show_input_box(#[string] _options_json: String) -> Result<String, anyhow::Error> {
    tracing::debug!("[vscode window] showInputBox (stage 2: returns empty)");
    Ok(String::new())
}

#[op2]
#[string]
fn op_show_quick_pick(
    #[string] items_json: String,
    #[string] _options_json: String,
) -> Result<String, anyhow::Error> {
    tracing::debug!("[vscode window] showQuickPick (stage 2: returns first item)");
    let items: Vec<String> = serde_json::from_str(&items_json).unwrap_or_default();
    Ok(items.first().cloned().unwrap_or_default())
}

// ═══════════════════════════════════════════════════════════════════════════
// 输出通道 Ops
// ═══════════════════════════════════════════════════════════════════════════

#[op2(fast)]
fn op_create_output_channel(#[string] name: String) -> Result<u32, anyhow::Error> {
    let channel_id = next_channel_id();
    output_channels().lock().insert(channel_id, name.clone());
    tracing::info!("[vscode window] createOutputChannel: {} (id={})", name, channel_id);
    Ok(channel_id)
}

#[op2(fast)]
fn op_output_channel_append(channel_id: u32, #[string] text: String) -> Result<(), anyhow::Error> {
    let channels = output_channels().lock();
    if let Some(name) = channels.get(&channel_id) {
        tracing::info!("[output:{}] {}", name, text.trim_end());
    }
    Ok(())
}

#[op2(fast)]
fn op_output_channel_show(channel_id: u32) -> Result<(), anyhow::Error> {
    let channels = output_channels().lock();
    if let Some(name) = channels.get(&channel_id) {
        tracing::info!("[vscode window] show output channel: {} (id={})", name, channel_id);
    }
    Ok(())
}

#[op2(fast)]
fn op_output_channel_hide(channel_id: u32) -> Result<(), anyhow::Error> {
    tracing::debug!("[vscode window] hide output channel id={}", channel_id);
    Ok(())
}

#[op2(fast)]
fn op_output_channel_clear(channel_id: u32) -> Result<(), anyhow::Error> {
    tracing::debug!("[vscode window] clear output channel id={}", channel_id);
    Ok(())
}

#[op2(fast)]
fn op_output_channel_dispose(channel_id: u32) -> Result<(), anyhow::Error> {
    output_channels().lock().remove(&channel_id);
    tracing::debug!("[vscode window] dispose output channel id={}", channel_id);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// 命令 Ops
// ═══════════════════════════════════════════════════════════════════════════

#[op2(fast)]
fn op_register_command(
    #[string] extension_id: String,
    #[string] command_id: String,
) -> Result<(), anyhow::Error> {
    command_registry().lock().insert(command_id.clone(), extension_id.clone());
    tracing::debug!("[vscode commands] register: {} (extension={})", command_id, extension_id);
    Ok(())
}

#[op2]
#[string]
fn op_execute_command(
    #[string] command_id: String,
    #[string] args_json: String,
) -> Result<String, anyhow::Error> {
    tracing::debug!("[vscode commands] execute: {} args={}", command_id, args_json);
    Ok("null".to_string())
}

#[op2]
#[string]
fn op_get_commands() -> Result<String, anyhow::Error> {
    let commands: Vec<String> = command_registry().lock().keys().cloned().collect();
    Ok(serde_json::to_string(&commands)?)
}

// ═══════════════════════════════════════════════════════════════════════════
// 工作区 Ops
// ═══════════════════════════════════════════════════════════════════════════

#[op2]
#[string]
fn op_get_workspace_folders() -> Result<String, anyhow::Error> {
    let folders: Vec<WorkspaceFolderInfo> = vec![];
    Ok(serde_json::to_string(&folders)?)
}

#[op2]
#[string]
fn op_get_workspace_file() -> Result<String, anyhow::Error> {
    Ok(String::new())
}

#[op2]
#[string]
fn op_open_text_document(#[string] uri: String) -> Result<String, anyhow::Error> {
    tracing::debug!("[vscode workspace] openTextDocument: {}", uri);
    let doc = TextDocumentInfo {
        uri: uri.clone(),
        language_id: "plaintext".to_string(),
        text: String::new(),
    };
    Ok(serde_json::to_string(&doc)?)
}

#[op2]
#[string]
fn op_get_configuration(#[string] _section: String) -> Result<String, anyhow::Error> {
    Ok("{}".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// 扩展管理 Ops
// ═══════════════════════════════════════════════════════════════════════════

#[op2]
#[string]
fn op_get_extension(#[string] _extension_id: String) -> Result<String, anyhow::Error> {
    Ok(String::new())
}

#[op2]
#[string]
fn op_get_all_extensions() -> Result<String, anyhow::Error> {
    let exts: Vec<ExtensionInfo> = vec![];
    Ok(serde_json::to_string(&exts)?)
}

// ═══════════════════════════════════════════════════════════════════════════
// Extension 构建
// ═══════════════════════════════════════════════════════════════════════════

deno_core::extension!(
    mox_host,
    ops = [
        op_show_information_message,
        op_show_warning_message,
        op_show_error_message,
        op_show_input_box,
        op_show_quick_pick,
        op_create_output_channel,
        op_output_channel_append,
        op_output_channel_show,
        op_output_channel_hide,
        op_output_channel_clear,
        op_output_channel_dispose,
        op_register_command,
        op_execute_command,
        op_get_commands,
        op_get_workspace_folders,
        op_get_workspace_file,
        op_open_text_document,
        op_get_configuration,
        op_get_extension,
        op_get_all_extensions,
    ]
);

pub fn build_host_extension() -> deno_core::Extension {
    mox_host::init_ops_and_esm()
}

// ═══════════════════════════════════════════════════════════════════════════
// 单元测试
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：构建宿主 Extension，验证 ops 数量和名称
    #[test]
    fn test_build_host_extension() {
        let ext = build_host_extension();
        assert_eq!(ext.name, "mox_host");
        // 阶段 2 注册了多个宿主 ops（UI/输出通道/命令/工作区/扩展）
        // 不硬编码具体数量，避免新增/删除 op 时测试失效
        assert!(
            ext.ops.len() >= 10,
            "expected at least 10 host ops, got {}",
            ext.ops.len()
        );
    }

    /// 测试：全局状态初始化
    #[test]
    fn test_global_state_initialized() {
        // 命令注册表初始为空
        assert!(command_registry().lock().is_empty());
        // 输出通道表初始为空
        assert!(output_channels().lock().is_empty());
    }

    /// 测试：命令注册和查询
    #[test]
    fn test_command_registry() {
        command_registry().lock().insert("test.cmd".to_string(), "test.ext".to_string());
        assert!(command_registry().lock().contains_key("test.cmd"));
        assert_eq!(command_registry().lock().get("test.cmd").unwrap(), "test.ext");
        command_registry().lock().remove("test.cmd");
    }

    /// 测试：输出通道创建和查询
    #[test]
    fn test_output_channel_registry() {
        let id = next_channel_id();
        assert!(id > 0);
        output_channels().lock().insert(id, "test-channel".to_string());
        assert!(output_channels().lock().contains_key(&id));
        assert_eq!(output_channels().lock().get(&id).unwrap(), "test-channel");
        output_channels().lock().remove(&id);
    }

    /// 测试：WorkspaceFolderInfo 序列化
    #[test]
    fn test_workspace_folder_info_serialization() {
        let info = WorkspaceFolderInfo {
            uri: "file:///workspace".to_string(),
            name: "workspace".to_string(),
            index: 0,
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: WorkspaceFolderInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.uri, "file:///workspace");
        assert_eq!(parsed.name, "workspace");
        assert_eq!(parsed.index, 0);
    }

    /// 测试：TextDocumentInfo 序列化
    #[test]
    fn test_text_document_info_serialization() {
        let info = TextDocumentInfo {
            uri: "file:///test.txt".to_string(),
            language_id: "plaintext".to_string(),
            text: "hello".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: TextDocumentInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.uri, "file:///test.txt");
        assert_eq!(parsed.language_id, "plaintext");
        assert_eq!(parsed.text, "hello");
    }

    /// 测试：ExtensionInfo 序列化
    #[test]
    fn test_extension_info_serialization() {
        let info = ExtensionInfo {
            id: "test.ext".to_string(),
            extension_path: "/ext".to_string(),
            is_active: true,
            package_json: serde_json::json!({"name": "test"}),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: ExtensionInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test.ext");
        assert!(parsed.is_active);
    }
}
