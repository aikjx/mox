// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! VSCode 扩展激活事件 — 解析 package.json 中的 activationEvents，
//! 并根据上下文判断是否应该激活扩展。
//!
//! ## VSCode 激活事件类型
//! - `onCommand:${command}` — 当指定命令被执行时激活
//! - `onLanguage:${language}` — 当打开指定语言的文件时激活
//! - `onWorkspaceContains:${filename}` — 当工作区包含指定文件时激活
//! - `onStartupFinished` — 启动完成后激活
//! - `onDebug` — 调试会话启动时激活
//! - `onFileSystem:${scheme}` — 当访问指定 scheme 的文件系统时激活
//! - `onView:${viewId}` — 当指定视图被展开时激活
//! - `onUri` — 当通过 URI 打开扩展时激活
//! - `onWalkthrough:${walkthroughId}` — 当指定 walkthrough 被打开时激活
//! - `*` — 启动时立即激活（不推荐，影响启动性能）
//!
//! ## 阶段规划
//! - 阶段 2：实现解析和基础匹配逻辑
//! - 阶段 3：集成到 VsCodeRuntime 的 init/start 流程中，实现延迟激活

use serde::{Deserialize, Serialize};
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════════
// ActivationEvent — 激活事件枚举
// ═══════════════════════════════════════════════════════════════════════════

/// VSCode 扩展激活事件
///
/// 对应 package.json 中 `activationEvents` 数组的每一项。
/// 解析后用于判断扩展何时应该被激活。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivationEvent {
    /// `onCommand:${commandId}` — 当指定命令被执行时激活
    OnCommand(String),
    /// `onLanguage:${languageId}` — 当打开指定语言的文件时激活
    OnLanguage(String),
    /// `onWorkspaceContains:${filename}` — 当工作区包含指定文件时激活
    OnWorkspaceContains(String),
    /// `onStartupFinished` — 启动完成后激活
    OnStartupFinished,
    /// `onDebug` — 调试会话启动时激活
    OnDebug,
    /// `onFileSystem:${scheme}` — 当访问指定 scheme 的文件系统时激活
    OnFileSystem(String),
    /// `onView:${viewId}` — 当指定视图被展开时激活
    OnView(String),
    /// `onUri` — 当通过 URI 打开扩展时激活
    OnUri,
    /// `onWalkthrough:${walkthroughId}` — 当指定 walkthrough 被打开时激活
    OnWalkthrough(String),
    /// `*` — 启动时立即激活
    OnAny,
}

impl ActivationEvent {
    /// 获取事件类型名称（不含参数）
    pub fn event_type(&self) -> &'static str {
        match self {
            ActivationEvent::OnCommand(_) => "onCommand",
            ActivationEvent::OnLanguage(_) => "onLanguage",
            ActivationEvent::OnWorkspaceContains(_) => "onWorkspaceContains",
            ActivationEvent::OnStartupFinished => "onStartupFinished",
            ActivationEvent::OnDebug => "onDebug",
            ActivationEvent::OnFileSystem(_) => "onFileSystem",
            ActivationEvent::OnView(_) => "onView",
            ActivationEvent::OnUri => "onUri",
            ActivationEvent::OnWalkthrough(_) => "onWalkthrough",
            ActivationEvent::OnAny => "*",
        }
    }

    /// 获取事件参数（如果有）
    pub fn param(&self) -> Option<&str> {
        match self {
            ActivationEvent::OnCommand(p)
            | ActivationEvent::OnLanguage(p)
            | ActivationEvent::OnWorkspaceContains(p)
            | ActivationEvent::OnFileSystem(p)
            | ActivationEvent::OnView(p)
            | ActivationEvent::OnWalkthrough(p) => Some(p),
            _ => None,
        }
    }
}

impl fmt::Display for ActivationEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActivationEvent::OnCommand(id) => write!(f, "onCommand:{}", id),
            ActivationEvent::OnLanguage(lang) => write!(f, "onLanguage:{}", lang),
            ActivationEvent::OnWorkspaceContains(name) => {
                write!(f, "onWorkspaceContains:{}", name)
            }
            ActivationEvent::OnStartupFinished => write!(f, "onStartupFinished"),
            ActivationEvent::OnDebug => write!(f, "onDebug"),
            ActivationEvent::OnFileSystem(scheme) => write!(f, "onFileSystem:{}", scheme),
            ActivationEvent::OnView(view_id) => write!(f, "onView:{}", view_id),
            ActivationEvent::OnUri => write!(f, "onUri"),
            ActivationEvent::OnWalkthrough(id) => write!(f, "onWalkthrough:{}", id),
            ActivationEvent::OnAny => write!(f, "*"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ActivationContext — 激活上下文
// ═══════════════════════════════════════════════════════════════════════════

/// 激活上下文 — 描述当前触发激活的场景信息
///
/// 用于 [`should_activate`] 函数判断扩展是否应该在当前场景下激活。
#[derive(Debug, Clone, Default)]
pub struct ActivationContext {
    /// 当前触发的命令 ID（当用户执行命令时）
    pub command: Option<String>,
    /// 当前打开的文件语言 ID（当打开文件时）
    pub language: Option<String>,
    /// 工作区中的文件列表（用于 onWorkspaceContains 匹配）
    pub workspace_files: Vec<String>,
    /// 是否启动已完成（用于 onStartupFinished 匹配）
    pub startup_finished: bool,
    /// 当前访问的文件系统 scheme（用于 onFileSystem 匹配）
    pub file_system_scheme: Option<String>,
    /// 当前展开的视图 ID（用于 onView 匹配）
    pub view_id: Option<String>,
    /// 是否通过 URI 触发（用于 onUri 匹配）
    pub uri_triggered: bool,
    /// 当前打开的 walkthrough ID（用于 onWalkthrough 匹配）
    pub walkthrough_id: Option<String>,
}

impl ActivationContext {
    /// 创建空的激活上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置命令触发
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    /// 设置语言触发
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// 设置工作区文件
    pub fn with_workspace_files(mut self, files: Vec<String>) -> Self {
        self.workspace_files = files;
        self
    }

    /// 标记启动已完成
    pub fn with_startup_finished(mut self) -> Self {
        self.startup_finished = true;
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 解析与匹配函数
// ═══════════════════════════════════════════════════════════════════════════

/// 解析 activationEvents 字符串数组为 [`ActivationEvent`] 列表
///
/// 无法识别的事件会被记录警告并跳过，不会导致错误。
///
/// # 示例
/// ```
/// use mox_plugin_core::runtime::vscode::activation::parse_activation_events;
/// let events = parse_activation_events(&[
///     "onCommand:hello.world".to_string(),
///     "onLanguage:python".to_string(),
///     "*".to_string(),
/// ]);
/// assert_eq!(events.len(), 3);
/// ```
pub fn parse_activation_events(events: &[String]) -> Vec<ActivationEvent> {
    let mut result = Vec::new();
    for event_str in events {
        match parse_single_event(event_str) {
            Some(event) => result.push(event),
            None => {
                tracing::warn!("unknown activation event: '{}'", event_str);
            }
        }
    }
    result
}

/// 解析单个激活事件字符串
fn parse_single_event(s: &str) -> Option<ActivationEvent> {
    let s = s.trim();
    if s == "*" {
        return Some(ActivationEvent::OnAny);
    }
    if s == "onStartupFinished" {
        return Some(ActivationEvent::OnStartupFinished);
    }
    if s == "onDebug" {
        return Some(ActivationEvent::OnDebug);
    }
    if s == "onUri" {
        return Some(ActivationEvent::OnUri);
    }

    // 带参数的事件：type:param
    if let Some((event_type, param)) = s.split_once(':') {
        let param = param.trim().to_string();
        if param.is_empty() {
            tracing::warn!("activation event '{}' has empty parameter", s);
            return None;
        }
        return match event_type {
            "onCommand" => Some(ActivationEvent::OnCommand(param)),
            "onLanguage" => Some(ActivationEvent::OnLanguage(param)),
            "onWorkspaceContains" => Some(ActivationEvent::OnWorkspaceContains(param)),
            "onFileSystem" => Some(ActivationEvent::OnFileSystem(param)),
            "onView" => Some(ActivationEvent::OnView(param)),
            "onWalkthrough" => Some(ActivationEvent::OnWalkthrough(param)),
            _ => None,
        };
    }

    None
}

/// 判断扩展是否应该在当前上下文下激活
///
/// 只要有任意一个激活事件与上下文匹配，就返回 `true`。
///
/// # 匹配规则
/// - `OnAny` (`*`)：始终匹配
/// - `OnCommand(id)`：当 `context.command == id` 时匹配
/// - `OnLanguage(lang)`：当 `context.language == lang` 时匹配
/// - `OnWorkspaceContains(filename)`：当 `context.workspace_files` 包含该文件名时匹配
/// - `OnStartupFinished`：当 `context.startup_finished == true` 时匹配
/// - `OnDebug`：阶段 2 暂不支持，返回 false
/// - `OnFileSystem(scheme)`：当 `context.file_system_scheme == scheme` 时匹配
/// - `OnView(view_id)`：当 `context.view_id == view_id` 时匹配
/// - `OnUri`：当 `context.uri_triggered == true` 时匹配
/// - `OnWalkthrough(id)`：当 `context.walkthrough_id == id` 时匹配
pub fn should_activate(events: &[ActivationEvent], context: &ActivationContext) -> bool {
    events.iter().any(|event| event_matches(event, context))
}

/// 单个激活事件与上下文的匹配判断
fn event_matches(event: &ActivationEvent, ctx: &ActivationContext) -> bool {
    match event {
        // `*` — 始终激活
        ActivationEvent::OnAny => true,

        // onCommand:${commandId}
        ActivationEvent::OnCommand(cmd_id) => ctx
            .command
            .as_ref()
            .map(|c| c == cmd_id)
            .unwrap_or(false),

        // onLanguage:${languageId}
        ActivationEvent::OnLanguage(lang_id) => ctx
            .language
            .as_ref()
            .map(|l| l == lang_id)
            .unwrap_or(false),

        // onWorkspaceContains:${filename}
        ActivationEvent::OnWorkspaceContains(filename) => {
            // 支持 glob 风格的简单匹配（阶段 2：精确匹配 + 后缀匹配）
            ctx.workspace_files.iter().any(|f| {
                f == filename
                    || f.ends_with(filename)
                    || f.ends_with(&format!("/{}", filename))
            })
        }

        // onStartupFinished
        ActivationEvent::OnStartupFinished => ctx.startup_finished,

        // onDebug — 阶段 2 暂不支持调试激活
        ActivationEvent::OnDebug => {
            tracing::debug!("onDebug activation not supported in stage 2");
            false
        }

        // onFileSystem:${scheme}
        ActivationEvent::OnFileSystem(scheme) => ctx
            .file_system_scheme
            .as_ref()
            .map(|s| s == scheme)
            .unwrap_or(false),

        // onView:${viewId}
        ActivationEvent::OnView(view_id) => ctx
            .view_id
            .as_ref()
            .map(|v| v == view_id)
            .unwrap_or(false),

        // onUri
        ActivationEvent::OnUri => ctx.uri_triggered,

        // onWalkthrough:${walkthroughId}
        ActivationEvent::OnWalkthrough(id) => ctx
            .walkthrough_id
            .as_ref()
            .map(|w| w == id)
            .unwrap_or(false),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 单元测试
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_on_command() {
        let events = parse_activation_events(&["onCommand:hello.world".to_string()]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], ActivationEvent::OnCommand("hello.world".to_string()));
        assert_eq!(events[0].event_type(), "onCommand");
        assert_eq!(events[0].param(), Some("hello.world"));
    }

    #[test]
    fn test_parse_on_language() {
        let events = parse_activation_events(&["onLanguage:python".to_string()]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], ActivationEvent::OnLanguage("python".to_string()));
    }

    #[test]
    fn test_parse_on_workspace_contains() {
        let events = parse_activation_events(&["onWorkspaceContains:pyproject.toml".to_string()]);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            ActivationEvent::OnWorkspaceContains("pyproject.toml".to_string())
        );
    }

    #[test]
    fn test_parse_on_startup_finished() {
        let events = parse_activation_events(&["onStartupFinished".to_string()]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], ActivationEvent::OnStartupFinished);
        assert_eq!(events[0].param(), None);
    }

    #[test]
    fn test_parse_on_any() {
        let events = parse_activation_events(&["*".to_string()]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], ActivationEvent::OnAny);
    }

    #[test]
    fn test_parse_mixed_events() {
        let events = parse_activation_events(&[
            "onCommand:hello.world".to_string(),
            "onLanguage:python".to_string(),
            "onStartupFinished".to_string(),
            "*".to_string(),
            "onDebug".to_string(),
            "onView:explorer".to_string(),
            "onUri".to_string(),
            "onFileSystem:ftp".to_string(),
            "onWalkthrough:welcome".to_string(),
        ]);
        assert_eq!(events.len(), 9);
        assert!(matches!(events[0], ActivationEvent::OnCommand(_)));
        assert!(matches!(events[1], ActivationEvent::OnLanguage(_)));
        assert!(matches!(events[2], ActivationEvent::OnStartupFinished));
        assert!(matches!(events[3], ActivationEvent::OnAny));
        assert!(matches!(events[4], ActivationEvent::OnDebug));
        assert!(matches!(events[5], ActivationEvent::OnView(_)));
        assert!(matches!(events[6], ActivationEvent::OnUri));
        assert!(matches!(events[7], ActivationEvent::OnFileSystem(_)));
        assert!(matches!(events[8], ActivationEvent::OnWalkthrough(_)));
    }

    #[test]
    fn test_parse_unknown_event_skipped() {
        let events = parse_activation_events(&[
            "onCommand:test".to_string(),
            "unknownEvent:foo".to_string(),
            "onLanguage:js".to_string(),
        ]);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_parse_empty_param_skipped() {
        let events = parse_activation_events(&["onCommand:".to_string()]);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_display() {
        assert_eq!(
            format!("{}", ActivationEvent::OnCommand("test".to_string())),
            "onCommand:test"
        );
        assert_eq!(format!("{}", ActivationEvent::OnAny), "*");
        assert_eq!(
            format!("{}", ActivationEvent::OnStartupFinished),
            "onStartupFinished"
        );
    }

    // ── should_activate 测试 ──────────────────────────────────────────────

    #[test]
    fn test_should_activate_on_any() {
        let events = vec![ActivationEvent::OnAny];
        let ctx = ActivationContext::new();
        assert!(should_activate(&events, &ctx));
    }

    #[test]
    fn test_should_activate_on_command_match() {
        let events = vec![ActivationEvent::OnCommand("hello.world".to_string())];
        let ctx = ActivationContext::new().with_command("hello.world");
        assert!(should_activate(&events, &ctx));
    }

    #[test]
    fn test_should_activate_on_command_no_match() {
        let events = vec![ActivationEvent::OnCommand("hello.world".to_string())];
        let ctx = ActivationContext::new().with_command("other.command");
        assert!(!should_activate(&events, &ctx));
    }

    #[test]
    fn test_should_activate_on_language_match() {
        let events = vec![ActivationEvent::OnLanguage("python".to_string())];
        let ctx = ActivationContext::new().with_language("python");
        assert!(should_activate(&events, &ctx));
    }

    #[test]
    fn test_should_activate_on_startup_finished() {
        let events = vec![ActivationEvent::OnStartupFinished];
        let ctx = ActivationContext::new().with_startup_finished();
        assert!(should_activate(&events, &ctx));

        let ctx2 = ActivationContext::new();
        assert!(!should_activate(&events, &ctx2));
    }

    #[test]
    fn test_should_activate_on_workspace_contains() {
        let events = vec![ActivationEvent::OnWorkspaceContains("pyproject.toml".to_string())];
        let ctx = ActivationContext::new()
            .with_workspace_files(vec!["src/main.rs".to_string(), "pyproject.toml".to_string()]);
        assert!(should_activate(&events, &ctx));

        let ctx2 = ActivationContext::new()
            .with_workspace_files(vec!["src/main.rs".to_string()]);
        assert!(!should_activate(&events, &ctx2));
    }

    #[test]
    fn test_should_activate_multiple_events_any_match() {
        let events = vec![
            ActivationEvent::OnCommand("cmd.a".to_string()),
            ActivationEvent::OnLanguage("python".to_string()),
            ActivationEvent::OnStartupFinished,
        ];
        // 命令不匹配，但语言匹配
        let ctx = ActivationContext::new().with_language("python");
        assert!(should_activate(&events, &ctx));

        // 都不匹配
        let ctx2 = ActivationContext::new().with_command("cmd.b").with_language("js");
        assert!(!should_activate(&events, &ctx2));
    }

    #[test]
    fn test_should_activate_empty_events() {
        let events: Vec<ActivationEvent> = vec![];
        let ctx = ActivationContext::new().with_startup_finished();
        assert!(!should_activate(&events, &ctx));
    }
}
