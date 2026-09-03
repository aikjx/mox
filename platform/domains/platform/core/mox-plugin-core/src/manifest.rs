// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 插件描述符（Manifest）— 插件的元数据声明



use serde::{Deserialize, Serialize};

use std::collections::HashMap;



/// 插件权限（沙箱控制）

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]


pub enum PluginPermission {

    /// 文件读取


    #[serde(rename = "file:read")]


    FileRead,

    /// 文件写入


    #[serde(rename = "file:write")]


    FileWrite,

    /// 网络API调用


    #[serde(rename = "network:api")]


    NetworkApi,

    /// 网络监听（服务器）


    #[serde(rename = "network:server")]


    NetworkServer,

    /// AI能力调用


    #[serde(rename = "ai:chat")]


    AiChat,

    /// 数据库访问


    #[serde(rename = "database")]


    Database,

    /// 缓存访问


    #[serde(rename = "cache")]


    Cache,

    /// 事件发布


    #[serde(rename = "event:publish")]


    EventPublish,

    /// 事件订阅


    #[serde(rename = "event:subscribe")]


    EventSubscribe,

    /// 系统命令执行（高危）


    #[serde(rename = "system:command")]


    SystemCommand,

    /// 环境变量读取


    #[serde(rename = "env:read")]


    EnvRead,

}



impl PluginPermission {

    pub fn as_str(&self) -> &'static str {

        match self {

            PluginPermission::FileRead => "file:read",

            PluginPermission::FileWrite => "file:write",

            PluginPermission::NetworkApi => "network:api",

            PluginPermission::NetworkServer => "network:server",

            PluginPermission::AiChat => "ai:chat",

            PluginPermission::Database => "database",

            PluginPermission::Cache => "cache",

            PluginPermission::EventPublish => "event:publish",

            PluginPermission::EventSubscribe => "event:subscribe",

            PluginPermission::SystemCommand => "system:command",

            PluginPermission::EnvRead => "env:read",

        }

    }

}



/// 插件依赖声明

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct PluginDependency {

    /// 依赖的插件ID

    pub id: String,

    /// 版本约束（如 ">=1.0.0", "^2.0"）

    pub version: String,

    /// 是否可选依赖

    #[serde(default)]

    pub optional: bool,

}



/// 插件配置Schema（JSON Schema子集）

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct ConfigField {

    pub name: String,

    #[serde(rename = "type")]

    pub field_type: String, // string/number/boolean/object

    #[serde(default)]

    pub required: bool,

    #[serde(default)]

    pub default: Option<serde_json::Value>,

    #[serde(default)]

    pub description: String,

    /// 是否敏感字段（如API Key，存储时加密）

    #[serde(default)]

    pub secret: bool,

}



/// 插件能力声明（插件向平台注册的能力）

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct PluginCapability {

    /// 能力唯一标识（如 "ocr.extract", "translate.text"）

    pub id: String,

    /// 能力名称

    pub name: String,

    /// 能力描述

    pub description: String,

    /// 输入参数Schema

    pub input_schema: serde_json::Value,

    /// 输出参数Schema

    pub output_schema: serde_json::Value,

}



/// 插件描述符（manifest.json）

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct PluginManifest {

    /// 插件唯一ID（反向域名风格，如 "com.vendor.ocr"）

    pub id: String,

    /// 插件名称

    pub name: String,

    /// 版本（语义化版本）

    pub version: String,

    /// 作者

    pub author: String,

    /// 描述

    pub description: String,

    /// 入口文件（WASM模块路径）

    pub entry: String,

    /// 权限列表

    #[serde(default)]

    pub permissions: Vec<PluginPermission>,

    /// 依赖列表

    #[serde(default)]

    pub dependencies: Vec<PluginDependency>,

    /// 配置Schema

    #[serde(default)]

    pub config_schema: Vec<ConfigField>,

    /// 能力列表

    #[serde(default)]

    pub capabilities: Vec<PluginCapability>,

    /// 标签（用于分类搜索）

    #[serde(default)]

    pub tags: Vec<String>,

    /// 主页URL

    #[serde(default)]

    pub homepage: Option<String>,

    /// 仓库URL

    #[serde(default)]

    pub repository: Option<String>,

    /// 许可证

    #[serde(default)]

    pub license: Option<String>,

    /// 最低平台版本

    #[serde(default = "default_min_platform_version")]

    pub min_platform_version: String,

}



fn default_min_platform_version() -> String { "3.0.0".into() }



impl PluginManifest {

    /// 从JSON字符串解析

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {

        serde_json::from_str(json)

    }



    /// 序列化为JSON

    pub fn to_json(&self) -> Result<String, serde_json::Error> {

        serde_json::to_string_pretty(self)

    }



    /// 检查是否有权限

    pub fn has_permission(&self, perm: PluginPermission) -> bool {

        self.permissions.contains(&perm)

    }



    /// 语义化版本比较（简化）

    pub fn version_matches(&self, constraint: &str) -> bool {

        // 简化：支持 ">=x.y.z", "^x.y", "x.y.z"

        let current = parse_version(&self.version);

        if constraint.starts_with(">=") {

            let required = parse_version(constraint.trim_start_matches(">="));

            return current >= required;

        }

        if constraint.starts_with("^") {

            let required = parse_version(constraint.trim_start_matches("^"));

            return current.0 == required.0 && current >= required;

        }

        let required = parse_version(constraint);

        current == required

    }

}



fn parse_version(v: &str) -> (u32, u32, u32) {

    let parts: Vec<u32> = v.trim()

        .split('.')

        .filter_map(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())

        .collect();

    (

        parts.first().copied().unwrap_or(0),

        parts.get(1).copied().unwrap_or(0),

        parts.get(2).copied().unwrap_or(0),

    )

}



/// 插件运行时配置（用户配置的实际值）

#[derive(Debug, Clone, Default)]

pub struct PluginConfig {

    pub values: HashMap<String, serde_json::Value>,

}



impl PluginConfig {

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {

        self.values.get(key)

    }



    pub fn get_string(&self, key: &str) -> Option<&str> {

        self.values.get(key).and_then(|v| v.as_str())

    }

}




// ═══════════════════════════════════════════════════════════════════════════
// VSCode 插件元数据兼容层（方案 C 阶段 1）
// ═══════════════════════════════════════════════════════════════════════════

/// VSCode 扩展命令贡献点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsCodeCommand {
    pub command: String,
    pub title: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub icon: Option<serde_json::Value>,
}

/// VSCode 菜单贡献点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsCodeMenu {
    pub command: String,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub when: Option<String>,
}

/// VSCode 快捷键贡献点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsCodeKeybinding {
    pub command: String,
    pub key: String,
    #[serde(default)]
    pub mac: Option<String>,
    #[serde(default)]
    pub when: Option<String>,
}

/// VSCode 语言贡献点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsCodeLanguage {
    pub id: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub filenames: Vec<String>,
    #[serde(default)]
    pub mimetypes: Vec<String>,
    #[serde(default)]
    pub configuration: Option<String>,
}

/// VSCode 主题贡献点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsCodeTheme {
    pub label: String,
    #[serde(rename = "uiTheme")]
    pub ui_theme: String,
    pub path: String,
}

/// VSCode 代码片段贡献点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsCodeSnippet {
    pub language: String,
    pub path: String,
}

/// VSCode 视图贡献点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsCodeView {
    pub id: String,
    pub name: String,
    #[serde(default, rename = "type")]
    pub view_type: Option<String>,
}

/// VSCode 视图容器贡献点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsCodeViewContainer {
    pub id: String,
    pub title: String,
    pub icon: String,
}

/// VSCode 贡献点集合
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VsCodeContributes {
    #[serde(default)]
    pub commands: Vec<VsCodeCommand>,
    #[serde(default)]
    pub menus: std::collections::HashMap<String, Vec<VsCodeMenu>>,
    #[serde(default)]
    pub keybindings: Vec<VsCodeKeybinding>,
    #[serde(default)]
    pub languages: Vec<VsCodeLanguage>,
    #[serde(default)]
    pub themes: Vec<VsCodeTheme>,
    #[serde(default)]
    pub snippets: Vec<VsCodeSnippet>,
    #[serde(default)]
    pub views: std::collections::HashMap<String, Vec<VsCodeView>>,
    #[serde(default, rename = "viewContainers")]
    pub view_containers: Vec<VsCodeViewContainer>,
    #[serde(flatten)]
    pub other: std::collections::HashMap<String, serde_json::Value>,
}

/// VSCode 扩展 package.json 完整描述符
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsCodeManifest {
    pub name: String,
    pub version: String,
    #[serde(default, rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub engines: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub contributes: VsCodeContributes,
    #[serde(default, rename = "activationEvents")]
    pub activation_events: Vec<String>,
    #[serde(default, rename = "enabledApiProposals")]
    pub enabled_api_proposals: Vec<String>,
    #[serde(default)]
    pub main: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub repository: Option<serde_json::Value>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(flatten)]
    pub other: std::collections::HashMap<String, serde_json::Value>,
}

impl VsCodeManifest {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_mox_manifest(&self) -> PluginManifest {
        let publisher = self.publisher.as_deref().unwrap_or("unknown");
        let id = format!("vscode.{}.{}", publisher, self.name);
        let display_name = self.display_name.clone().unwrap_or_else(|| self.name.clone());
        let description = self.description.clone().unwrap_or_default();

        let mut capabilities: Vec<PluginCapability> = Vec::new();

        for cmd in &self.contributes.commands {
            capabilities.push(PluginCapability {
                id: format!("command.{}", cmd.command),
                name: cmd.title.clone(),
                description: format!(
                    "VSCode command{}",
                    cmd.category.as_ref().map(|c| format!(" [{}]", c)).unwrap_or_default()
                ),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
                output_schema: serde_json::json!({"type": "object", "properties": {}}),
            });
        }

        for kb in &self.contributes.keybindings {
            capabilities.push(PluginCapability {
                id: format!("keybinding.{}", kb.command),
                name: format!("Keybinding: {}", kb.key),
                description: format!(
                    "VSCode keybinding {} for command {}{}",
                    kb.key, kb.command,
                    kb.when.as_ref().map(|w| format!(" (when: {})", w)).unwrap_or_default()
                ),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
                output_schema: serde_json::json!({"type": "object", "properties": {}}),
            });
        }

        for lang in &self.contributes.languages {
            capabilities.push(PluginCapability {
                id: format!("language.{}", lang.id),
                name: lang.aliases.first().cloned().unwrap_or_else(|| lang.id.clone()),
                description: format!("Language support for {} (extensions: {:?})", lang.id, lang.extensions),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
                output_schema: serde_json::json!({"type": "object", "properties": {}}),
            });
        }

        for theme in &self.contributes.themes {
            capabilities.push(PluginCapability {
                id: format!("theme.{}", theme.label.to_lowercase().replace(' ', "_")),
                name: theme.label.clone(),
                description: format!("VSCode theme ({}), path: {}", theme.ui_theme, theme.path),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
                output_schema: serde_json::json!({"type": "object", "properties": {}}),
            });
        }

        for snippet in &self.contributes.snippets {
            capabilities.push(PluginCapability {
                id: format!("snippet.{}", snippet.language),
                name: format!("Snippets: {}", snippet.language),
                description: format!("Code snippets for {}, path: {}", snippet.language, snippet.path),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
                output_schema: serde_json::json!({"type": "object", "properties": {}}),
            });
        }

        for (_container_id, views) in &self.contributes.views {
            for view in views {
                capabilities.push(PluginCapability {
                    id: format!("view.{}", view.id),
                    name: view.name.clone(),
                    description: format!("VSCode view (type: {})", view.view_type.as_deref().unwrap_or("tree")),
                    input_schema: serde_json::json!({"type": "object", "properties": {}}),
                    output_schema: serde_json::json!({"type": "object", "properties": {}}),
                });
            }
        }

        let mut permissions: Vec<PluginPermission> = Vec::new();
        for proposal in &self.enabled_api_proposals {
            if let Some(perm) = map_api_proposal_to_permission(proposal) {
                if !permissions.contains(&perm) {
                    permissions.push(perm);
                }
            }
        }

        let mut tags: Vec<String> = Vec::new();
        tags.push("runtime:vscode".to_string());
        for event in &self.activation_events {
            tags.push(format!("activate:{}", event));
        }
        for category in &self.categories {
            tags.push(format!("category:{}", category.to_lowercase()));
        }
        for keyword in &self.keywords {
            tags.push(keyword.clone());
        }

        PluginManifest {
            id,
            name: display_name,
            version: self.version.clone(),
            author: publisher.to_string(),
            description,
            entry: self.main.clone().unwrap_or_else(|| "extension.js".to_string()),
            permissions,
            dependencies: Vec::new(),
            config_schema: Vec::new(),
            capabilities,
            tags,
            homepage: self.homepage.clone(),
            repository: self.repository.as_ref().and_then(|r| {
                if let Some(url) = r.get("url").and_then(|u| u.as_str()) {
                    Some(url.to_string())
                } else if let Some(s) = r.as_str() {
                    Some(s.to_string())
                } else {
                    None
                }
            }),
            license: self.license.clone(),
            min_platform_version: "3.0.0".to_string(),
        }
    }
}

fn map_api_proposal_to_permission(proposal: &str) -> Option<PluginPermission> {
    match proposal {
        "fileSearchProvider" | "textSearchProvider" => Some(PluginPermission::FileRead),
        "externalUriOpener" | "contributesViewsWelcome" => Some(PluginPermission::NetworkApi),
        "terminalDataWriteEvent" | "terminalDimensions" | "terminalSelection" => {
            Some(PluginPermission::SystemCommand)
        }
        "envVariableCollection" => Some(PluginPermission::EnvRead),
        "chatAgents" | "chatParticipant" | "languageModelAccess" => Some(PluginPermission::AiChat),
        _ => None,
    }
}

impl PluginManifest {
    pub fn from_vscode(package_json: &str) -> Result<Self, serde_json::Error> {
        let vscode = VsCodeManifest::from_json(package_json)?;
        Ok(vscode.to_mox_manifest())
    }
}


#[cfg(test)]

mod tests {

    use super::*;



    #[test]

    fn test_manifest_parse() {

        let json = r#"{

            "id": "com.test.ocr",

            "name": "OCR Plugin",

            "version": "1.2.0",

            "author": "TestCorp",

            "description": "OCR文字识别",

            "entry": "plugin.wasm",

            "permissions": ["file:read", "ai:chat"],

            "capabilities": [{"id": "ocr.extract", "name": "提取文字", "description": "", "input_schema": {}, "output_schema": {}}]

        }"#;

        let manifest = PluginManifest::from_json(json).unwrap();

        assert_eq!(manifest.id, "com.test.ocr");

        assert_eq!(manifest.version, "1.2.0");

        assert!(manifest.has_permission(PluginPermission::FileRead));

        assert!(!manifest.has_permission(PluginPermission::FileWrite));

        assert_eq!(manifest.capabilities.len(), 1);

    }



    #[test]

    fn test_version_matches() {

        let manifest = PluginManifest {

            id: "test".into(), name: "test".into(), version: "1.5.3".into(),

            author: "test".into(), description: "test".into(), entry: "test.wasm".into(),

            permissions: vec![], dependencies: vec![], config_schema: vec![],

            capabilities: vec![], tags: vec![], homepage: None, repository: None,

            license: None, min_platform_version: "3.0.0".into(),

        };

        assert!(manifest.version_matches(">=1.0.0"));

        assert!(manifest.version_matches("^1.0"));

        assert!(!manifest.version_matches("^2.0"));

        assert!(manifest.version_matches("1.5.3"));

    }


    #[test]
    fn test_vscode_manifest_parse_python() {
        let json = r#"{
            "name": "python",
            "version": "2024.0.1",
            "displayName": "Python",
            "description": "IntelliSense, linting, debugging for Python",
            "publisher": "ms-python",
            "engines": {"vscode": "^1.74.0"},
            "categories": ["Programming Languages", "Debuggers"],
            "keywords": ["python", "debugging", "linting"],
            "main": "./out/client/extension",
            "activationEvents": [
                "onLanguage:python",
                "onCommand:python.execInTerminal",
                "onWorkspaceContains:pyproject.toml"
            ],
            "contributes": {
                "commands": [
                    {"command": "python.execInTerminal", "title": "Python: Run Python File in Terminal", "category": "Python"},
                    {"command": "python.selectInterpreter", "title": "Python: Select Interpreter", "category": "Python"}
                ],
                "keybindings": [
                    {"command": "python.execInTerminal", "key": "ctrl+shift+f10", "when": "editorLangId == python"}
                ],
                "languages": [
                    {"id": "python", "aliases": ["Python", "py"], "extensions": [".py", ".pyw"], "configuration": "./language-configuration.json"}
                ],
                "themes": [
                    {"label": "Python Blue", "uiTheme": "vs-dark", "path": "./themes/python-blue-color-theme.json"}
                ],
                "snippets": [
                    {"language": "python", "path": "./snippets/python.json"}
                ],
                "views": {
                    "explorer": [
                        {"id": "pythonTestExplorer", "name": "Python Tests", "type": "tree"}
                    ]
                }
            },
            "enabledApiProposals": ["languageModelAccess"]
        }"#;

        let vscode = VsCodeManifest::from_json(json).unwrap();
        assert_eq!(vscode.name, "python");
        assert_eq!(vscode.version, "2024.0.1");
        assert_eq!(vscode.display_name.as_deref(), Some("Python"));
        assert_eq!(vscode.publisher.as_deref(), Some("ms-python"));
        assert_eq!(vscode.activation_events.len(), 3);
        assert_eq!(vscode.contributes.commands.len(), 2);
        assert_eq!(vscode.contributes.keybindings.len(), 1);
        assert_eq!(vscode.contributes.languages.len(), 1);
        assert_eq!(vscode.contributes.themes.len(), 1);
        assert_eq!(vscode.contributes.snippets.len(), 1);
        assert_eq!(vscode.enabled_api_proposals.len(), 1);

        let mox = vscode.to_mox_manifest();
        assert_eq!(mox.id, "vscode.ms-python.python");
        assert_eq!(mox.name, "Python");
        assert_eq!(mox.version, "2024.0.1");
        assert_eq!(mox.author, "ms-python");
        assert_eq!(mox.entry, "./out/client/extension");
        assert_eq!(mox.capabilities.len(), 7);

        let cmd_caps: Vec<_> = mox.capabilities.iter().filter(|c| c.id.starts_with("command.")).collect();
        assert_eq!(cmd_caps.len(), 2);
        assert!(cmd_caps.iter().any(|c| c.id == "command.python.execInTerminal"));

        let lang_caps: Vec<_> = mox.capabilities.iter().filter(|c| c.id.starts_with("language.")).collect();
        assert_eq!(lang_caps.len(), 1);
        assert_eq!(lang_caps[0].id, "language.python");

        assert!(mox.has_permission(PluginPermission::AiChat));
        assert!(mox.tags.contains(&"runtime:vscode".to_string()));
        assert!(mox.tags.iter().any(|t| t.starts_with("activate:onLanguage:python")));
    }

    #[test]
    fn test_vscode_manifest_parse_eslint() {
        let json = r#"{
            "name": "vscode-eslint",
            "version": "2.4.4",
            "displayName": "ESLint",
            "description": "Integrates ESLint JavaScript into VS Code",
            "publisher": "dbaeumer",
            "engines": {"vscode": "^1.75.0"},
            "categories": ["Linters"],
            "main": "./out/extension",
            "activationEvents": ["onLanguage:javascript", "onLanguage:typescript"],
            "contributes": {
                "commands": [
                    {"command": "eslint.executeAutofix", "title": "ESLint: Fix all auto-fixable Problems"}
                ],
                "languages": [
                    {"id": "javascript", "aliases": ["JavaScript", "js"], "extensions": [".js"]},
                    {"id": "typescript", "aliases": ["TypeScript", "ts"], "extensions": [".ts"]}
                ]
            }
        }"#;

        let mox = PluginManifest::from_vscode(json).unwrap();
        assert_eq!(mox.id, "vscode.dbaeumer.vscode-eslint");
        assert_eq!(mox.name, "ESLint");
        assert_eq!(mox.author, "dbaeumer");
        assert_eq!(mox.capabilities.len(), 3);
        assert!(mox.tags.contains(&"runtime:vscode".to_string()));
    }

    #[test]
    fn test_vscode_manifest_minimal() {
        let json = r#"{
            "name": "minimal-ext",
            "version": "1.0.0"
        }"#;

        let vscode = VsCodeManifest::from_json(json).unwrap();
        assert_eq!(vscode.name, "minimal-ext");
        assert_eq!(vscode.version, "1.0.0");
        assert!(vscode.contributes.commands.is_empty());
        assert!(vscode.activation_events.is_empty());

        let mox = vscode.to_mox_manifest();
        assert_eq!(mox.id, "vscode.unknown.minimal-ext");
        assert_eq!(mox.name, "minimal-ext");
        assert_eq!(mox.entry, "extension.js");
        assert!(mox.capabilities.is_empty());
        assert!(mox.tags.contains(&"runtime:vscode".to_string()));
    }

    #[test]
    fn test_api_proposal_permission_mapping() {
        assert_eq!(map_api_proposal_to_permission("languageModelAccess"), Some(PluginPermission::AiChat));
        assert_eq!(map_api_proposal_to_permission("fileSearchProvider"), Some(PluginPermission::FileRead));
        assert_eq!(map_api_proposal_to_permission("terminalDataWriteEvent"), Some(PluginPermission::SystemCommand));
        assert_eq!(map_api_proposal_to_permission("envVariableCollection"), Some(PluginPermission::EnvRead));
        assert_eq!(map_api_proposal_to_permission("unknownProposal"), None);
    }

}

