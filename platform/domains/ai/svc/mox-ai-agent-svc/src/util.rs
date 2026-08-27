// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 公共工具：跨模块复用的小工具。

/// 从可能包裹 markdown 代码块或前后缀文字的文本中，容错抽取 JSON 对象片段。
///
/// 返回 `Some(slice)`（含首尾大括号）或 `None`（文本中无 `{...}`）。
/// 供 LLM 抽取结果解析复用，避免各模块重复实现并修复越界切片（原 `rfind('}').unwrap_or(len-1)`）。
pub(crate) fn extract_json_object(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}').map(|i| i + 1).unwrap_or(trimmed.len());
    if start >= end {
        return None;
    }
    Some(&trimmed[start..end])
}
