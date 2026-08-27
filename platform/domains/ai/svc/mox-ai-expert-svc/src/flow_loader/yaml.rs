// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! YAML ↔ FlowGraph 双向映射类型定义

use serde::{Deserialize, Serialize};

// ── 顶层类型 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDef {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub regulated: bool,
    #[serde(default)]
    pub rules: Vec<RuleDef>,
    pub nodes: Vec<NodeDef>,
    pub edges: Vec<EdgeDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDef {
    pub id: String,
    pub name: String,
    #[serde(default = "default_kind")]
    pub kind: serde_yaml::Value,
    #[serde(default = "default_duration")]
    pub duration_ms: u64,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub tool: Option<serde_yaml::Value>,
    #[serde(default)]
    pub access: Option<Vec<String>>,
    #[serde(default)]
    pub transactional: Option<bool>,
}

impl NodeDef {
    /// Returns a copy of the tool field as serde_yaml::Value.
    pub fn tool_value(&self) -> Option<serde_yaml::Value> {
        self.tool.clone()
    }
}

fn default_kind() -> serde_yaml::Value {
    serde_yaml::Value::String("task".into())
}
fn default_duration() -> u64 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDef {
    pub from: String,
    pub to: String,
    #[serde(default = "default_edge_kind")]
    pub kind: serde_yaml::Value,
}

fn default_edge_kind() -> serde_yaml::Value {
    serde_yaml::Value::String("sequence".into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDef {
    pub id: String,
    pub description: String,
    #[serde(default = "default_severity")]
    pub severity: serde_yaml::Value,
    #[serde(default)]
    pub prefixes: Option<Vec<String>>,
    #[serde(default)]
    pub tool: Option<serde_yaml::Value>,
    #[serde(default)]
    pub required_guard_tags: Option<Vec<String>>,
}

fn default_severity() -> serde_yaml::Value {
    serde_yaml::Value::String("warning".into())
}

/// YAML 纯解析工具（不读写文件系统）
pub struct YamlFlowLoader;

impl YamlFlowLoader {
    pub fn parse(text: &str) -> Result<FlowDef, serde_yaml::Error> {
        serde_yaml::from_str(text)
    }
    pub fn serialize(def: &FlowDef) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(def)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
name: gov-pii
description: 政务数据归集
regulated: true
tags: [gov, pii]
nodes:
  - id: start
    name: 开始
    kind: start
    duration_ms: 0
  - id: asr
    name: 语音识别
    kind: task
    duration_ms: 150
    tool: Llm
  - id: end
    name: 结束
    kind: end
    duration_ms: 0
edges:
  - from: start
    to: asr
  - from: asr
    to: end
"#;

    #[test]
    fn parse_roundtrip() {
        let def = YamlFlowLoader::parse(SAMPLE).unwrap();
        assert_eq!(def.name, "gov-pii");
        assert_eq!(def.nodes.len(), 3);
        let text = YamlFlowLoader::serialize(&def).unwrap();
        let def2 = YamlFlowLoader::parse(&text).unwrap();
        assert_eq!(def.name, def2.name);
    }
}
