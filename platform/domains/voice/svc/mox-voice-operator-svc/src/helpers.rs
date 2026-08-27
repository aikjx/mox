// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 跨平台辅助函数（屏蔽 cfg 细节给 8 大算子用）

use serde_json::{json, Value};
use std::process::Command;
use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::operator::OperatorCategory;

/// 稳定平台标签（给 XB-007 OperatorUnsupported 用，不随 cfg(target_os) 文案漂移）
pub fn platform_tag() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown_os"
    }
}

/// 小工具：成功输出 JSON 数组 + 返回 OperatorOutput（所有 list_xxx 动作共用）
pub fn array_output<T: serde::Serialize>(message: &str, items: &[T]) -> Value {
    json!(items)
}

/// 小工具：执行命令并返回 stdout 字符串（跨平台通用）
pub fn run_command(cmd: &str, args: &[&str]) -> Result<(String, String, i32), std::io::Error> {
    let mut c = Command::new(cmd);
    c.args(args);
    let out = c.output()?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let code = out.status.code().unwrap_or(-1);
    Ok((stdout, stderr, code))
}


/// 算子友好版 run_command：自动把 io::Error 转换为 XiaobaiError::ExecutionError，
/// 带上正确的 category/action，避免每个调用方 map_err 样板
pub fn run_command_xb(
    cmd: &str,
    args: &[&str],
    category: OperatorCategory,
    action: &str,
) -> XiaobaiResult<(String, String, i32)> {
    run_command(cmd, args).map_err(|e| XiaobaiError::ExecutionError {
        category: category.as_str().to_string(),
        action: action.to_string(),
        detail: format!("{cmd} {} failed: {e}", args_display(args)),
    })
}

fn args_display(args: &[&str]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    out.push('[');
    for (i, a) in args.iter().enumerate() {
        if i > 0 { out.push(' '); }
        if a.contains(char::is_whitespace) || a.is_empty() {
            let _ = write!(out, "{a:?}");
        } else {
            out.push_str(a);
        }
    }
    out.push(']');
    out
}

/// 截断长文本头部，避免 OperatorOutput.payload 写进审计时过大
pub fn truncate_head(s: &str, max_lines: usize, max_chars_per_line: usize) -> String {
    s.lines()
        .take(max_lines)
        .map(|l| {
            if l.chars().count() > max_chars_per_line {
                let clipped: String = l.chars().take(max_chars_per_line).collect();
                format!("{clipped}…")
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
