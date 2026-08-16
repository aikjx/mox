//! FlowLoader — YAML 流程外部化引擎
//!
//! 将业务工作流从硬编码 Rust 解放出来，业务人员可用 YAML 增删改流程。

use crate::flow::{EdgeKind, FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind, Access, AccessMode};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

mod yaml;
mod validate;

pub use yaml::{FlowDef, NodeDef, EdgeDef as YamlEdgeDef, RuleDef, YamlFlowLoader};
pub use validate::ValidationError;

/// 流程加载器
pub struct FlowLoader {
    root: PathBuf,
}

impl FlowLoader {
    pub fn new(root: impl Into<PathBuf>) -> Self { Self { root: root.into() } }

    /// 加载单个流程
    pub fn load(&self, filename: &str) -> Result<FlowGraph, FlowLoadError> {
        let path = self.root.join(filename);
        if !path.exists() { return Err(FlowLoadError::FileNotFound(path)); }
        let text = fs::read_to_string(&path).map_err(|e| FlowLoadError::Io(e.to_string()))?;
        let def: FlowDef = serde_yaml::from_str(&text)
            .map_err(|e| FlowLoadError::Parse(e.to_string()))?;
        validate::validate(&def).map_err(FlowLoadError::Validation)?;
        Self::def_to_graph(filename.trim_end_matches(".yaml"), &def)
    }

    /// 加载全部 YAML（跳过解析失败的文件）
    pub fn load_all(&self) -> Result<Vec<(String, FlowGraph)>, FlowLoadError> {
        let mut out = Vec::new();
        let entries = fs::read_dir(&self.root).map_err(|e| FlowLoadError::Io(e.to_string()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load(&format!("{stem}.yaml")) {
                        Ok(g) => out.push((stem.into(), g)),
                        Err(e) => eprintln!("WARN: skipped '{stem}' — {e}"),
                    }
                }
            }
        }
        Ok(out)
    }

    /// 保存 FlowGraph 回 YAML
    pub fn save(&self, name: &str, graph: &FlowGraph) -> Result<(), FlowLoadError> {
        let def = Self::graph_to_def(name, graph)?;
        let yaml_text = serde_yaml::to_string(&def)
            .map_err(|e| FlowLoadError::Serialize(e.to_string()))?;
        let path = self.root.join(format!("{name}.yaml"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| FlowLoadError::Io(e.to_string()))?;
        }
        fs::write(&path, yaml_text).map_err(|e| FlowLoadError::Io(e.to_string()))?;
        Ok(())
    }

    /// 列出可用流程（不加载内容）
    pub fn list(&self) -> Result<Vec<String>, FlowLoadError> {
        let entries = fs::read_dir(&self.root).map_err(|e| FlowLoadError::Io(e.to_string()))?;
        let mut names: Vec<_> = entries.flatten()
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("yaml"))
            .filter_map(|e| e.path().file_stem().and_then(|s| s.to_str()).map(String::from))
            .collect();
        names.sort();
        Ok(names)
    }

    // ── YAML → FlowGraph ─────────────────────────────────────────────────────

    fn def_to_graph(name: &str, def: &FlowDef) -> Result<FlowGraph, FlowLoadError> {
        let mut graph = FlowGraph::new(name, &def.name);
        let mut id_map: HashMap<&str, usize> = HashMap::new();

        // 先注册节点（得到 index）
        for nd in &def.nodes {
            let kind = Self::parse_node_kind(nd.kind.as_str().unwrap_or("task"));
            let mut node = FlowNode::new(&nd.id, &nd.name, kind);
            node.duration_ms = nd.duration_ms;
            if let Some(ref tags) = nd.tags { node.tags.clone_from(tags); }
            if let Some(ref tool_val) = nd.tool {
                if let Some(tool_str) = tool_val.as_str() {
                    if let Some(tk) = Self::parse_tool(tool_str) { node.tool = Some(tk); }
                }
            }
            if let Some(ref access_list) = nd.access {
                for acc in access_list {
                    if let Some((mode_str, res)) = acc.split_once(':') {
                        let mode = match mode_str.trim() {
                            "write" => AccessMode::Write,
                            "read" => AccessMode::Read,
                            _ => AccessMode::ReadWrite,
                        };
                        node.accesses.push(Access { resource: res.trim().to_string(), mode });
                    }
                }
            }
            if nd.transactional.unwrap_or(false) { node.transactional = true; }
            let idx = graph.nodes.len();
            graph.nodes.push(node);
            id_map.insert(nd.id.as_str(), idx);
        }

        // 再注册边
        for ed in &def.edges {
            let from_idx = id_map.get(ed.from.as_str())
                .ok_or_else(|| FlowLoadError::MissingNode(ed.from.clone()))?;
            let to_idx = id_map.get(ed.to.as_str())
                .ok_or_else(|| FlowLoadError::MissingNode(ed.to.clone()))?;
            let edge_kind = Self::parse_edge_kind(ed.kind.as_str().unwrap_or("sequence"));
            graph.edges.push(FlowEdge { from: format!("n{}", from_idx), to: format!("n{}", to_idx), kind: edge_kind, condition: None });
        }

        // 规则
        for rd in &def.rules {
            let sev = match rd.severity.as_str().unwrap_or("warning") {
                "blocking" => flow_ai::model::Severity::Blocking,
                "info" => flow_ai::model::Severity::Info,
                _ => flow_ai::model::Severity::Warning,
            };
            let tool_kinds: Vec<_> = rd.tool.as_ref()
                .and_then(|v: &serde_yaml::Value| v.as_str())
                .and_then(Self::parse_tool)
                .map(|tk| vec![tk])
                .unwrap_or_default();
            graph.rules.push(flow_ai::model::ExpertRule {
                id: rd.id.clone(),
                description: rd.description.clone(),
                severity: sev,
                resource_prefixes: rd.prefixes.clone().unwrap_or_default(),
                tool_kinds,
                required_guard_tags: rd.required_guard_tags.clone().unwrap_or_default(),
            });
        }

        Ok(graph)
    }

    fn graph_to_def(_name: &str, graph: &FlowGraph) -> Result<FlowDef, FlowLoadError> {
        let nodes: Vec<_> = graph.nodes.iter().map(|n| {
            let access: Option<Vec<_>> = if n.accesses.is_empty() { None } else {
                Some(n.accesses.iter().map(|a| {
                    let mode = match a.mode {
                        AccessMode::Read => "read",
                        AccessMode::Write => "write",
                        AccessMode::ReadWrite => "readwrite",
                    };
                    format!("{mode}:{resource}", resource = a.resource)
                }).collect())
            };
            NodeDef {
                id: n.id.clone(),
                name: n.name.clone(),
                kind: serde_yaml::Value::String(format!("{:?}", n.kind).to_lowercase()),
                duration_ms: n.duration_ms,
                tags: if n.tags.is_empty() { None } else { Some(n.tags.clone()) },
                tool: n.tool.map(|tk| serde_yaml::Value::String(format!("{:?}", tk).to_lowercase())),
                access,
                transactional: if n.transactional { Some(true) } else { None },
            }
        }).collect();
        let edges: Vec<_> = graph.edges.iter().map(|e| {
            YamlEdgeDef {
                from: e.from.clone(),
                to: e.to.clone(),
                kind: serde_yaml::Value::String(format!("{:?}", e.kind).to_lowercase()),
            }
        }).collect();
        Ok(FlowDef {
            name: graph.name.clone(),
            description: None,
            tags: graph.rules.iter().map(|r| r.id.clone()).collect(),
            regulated: false,
            rules: Vec::new(),
            nodes,
            edges,
        })
    }

    fn parse_node_kind(s: &str) -> NodeKind {
        match s.to_lowercase().as_str() {
            "start" => NodeKind::Start,
            "end" => NodeKind::End,
            "task" => NodeKind::Task,
            "decision" => NodeKind::Decision,
            "parallel_fork" => NodeKind::ParallelFork,
            "parallel_join" => NodeKind::ParallelJoin,
            "loop_start" => NodeKind::LoopStart,
            "loop_end" => NodeKind::LoopEnd,
            "guard" => NodeKind::Guard,
            "subflow" => NodeKind::SubFlow,
            _ => NodeKind::Task,
        }
    }

    fn parse_tool(s: &str) -> Option<ToolKind> {
        match s.to_lowercase().as_str() {
            "compute" => Some(ToolKind::Compute),
            "llm" => Some(ToolKind::Llm),
            "file" => Some(ToolKind::File),
            "browser" => Some(ToolKind::Browser),
            "database" => Some(ToolKind::Database),
            "http" => Some(ToolKind::Http),
            "shell" => Some(ToolKind::Shell),
            "human" => Some(ToolKind::Human),
            _ => None,
        }
    }

    fn parse_edge_kind(s: &str) -> EdgeKind {
        match s.to_lowercase().as_str() {
            "conditional" => EdgeKind::Conditional,
            "exception" => EdgeKind::Exception,
            "mutex" => EdgeKind::Mutex,
            "inferred_data" => EdgeKind::InferredData,
            _ => EdgeKind::Sequence,
        }
    }
}

impl Default for FlowLoader {
    fn default() -> Self { Self::new("flows") }
}

// ── 错误类型 ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum FlowLoadError {
    FileNotFound(PathBuf),
    Io(String),
    Parse(String),
    Serialize(String),
    Validation(ValidationError),
    MissingNode(String),
}

impl std::fmt::Display for FlowLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound(p) => write!(f, "file not found: {}", p.display()),
            Self::Io(s) => write!(f, "I/O: {s}"),
            Self::Parse(s) => write!(f, "YAML parse: {s}"),
            Self::Serialize(s) => write!(f, "YAML serialize: {s}"),
            Self::Validation(e) => write!(f, "validation: {e}"),
            Self::MissingNode(id) => write!(f, "edge references unknown node '{id}'"),
        }
    }
}

impl std::error::Error for FlowLoadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_default_path() {
        let l = FlowLoader::default();
        assert_eq!(l.root, PathBuf::from("flows"));
    }

    #[test]
    fn missing_file_is_not_found() {
        let l = FlowLoader::new("/nonexistent/path");
        assert!(matches!(l.load("x.yaml"), Err(FlowLoadError::FileNotFound(_))));
    }

    #[test]
    fn list_empty_dir() {
        let l = FlowLoader::new("/tmp/flow-test-empty");
        std::fs::create_dir_all("/tmp/flow-test-empty").ok();
        let names = l.list().unwrap();
        std::fs::remove_dir("/tmp/flow-test-empty").ok();
        assert!(names.is_empty());
    }
}
