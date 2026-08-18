//! 信息关联关系图（关图规范 GR-STD-V1.0）参考实现
//! 纯 Rust std，零外部依赖，离线可编译。
//! 子命令：build / validate / export / query / sync / snapshot / skeleton / deviate / dedup

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

// ===================== 模型 =====================

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum InfoKind {
    Business,
    Data,
    Function,
    Interface,
    CodeFile,
    Script,
    ScheduleTask,
    Config,
    Dependency,
    ThirdParty,
    Doc,
    Runtime,
    Requirement,
}

impl InfoKind {
    fn as_str(&self) -> &'static str {
        match self {
            InfoKind::Business => "Business",
            InfoKind::Data => "Data",
            InfoKind::Function => "Function",
            InfoKind::Interface => "Interface",
            InfoKind::CodeFile => "CodeFile",
            InfoKind::Script => "Script",
            InfoKind::ScheduleTask => "ScheduleTask",
            InfoKind::Config => "Config",
            InfoKind::Dependency => "Dependency",
            InfoKind::ThirdParty => "ThirdParty",
            InfoKind::Doc => "Doc",
            InfoKind::Runtime => "Runtime",
            InfoKind::Requirement => "Requirement",
        }
    }
    fn from_str(s: &str) -> Option<InfoKind> {
        match s {
            "Business" => Some(InfoKind::Business),
            "Data" => Some(InfoKind::Data),
            "Function" => Some(InfoKind::Function),
            "Interface" => Some(InfoKind::Interface),
            "CodeFile" => Some(InfoKind::CodeFile),
            "Script" => Some(InfoKind::Script),
            "ScheduleTask" => Some(InfoKind::ScheduleTask),
            "Config" => Some(InfoKind::Config),
            "Dependency" => Some(InfoKind::Dependency),
            "ThirdParty" => Some(InfoKind::ThirdParty),
            "Doc" => Some(InfoKind::Doc),
            "Runtime" => Some(InfoKind::Runtime),
            "Requirement" => Some(InfoKind::Requirement),
            _ => None,
        }
    }
    fn color(&self) -> &'static str {
        match self {
            InfoKind::Business => "fill:#ffe0b2",
            InfoKind::Data => "fill:#b2dfdb",
            InfoKind::Function => "fill:#c5cae9",
            InfoKind::Interface => "fill:#f8bbd0",
            InfoKind::CodeFile => "fill:#cfd8dc",
            InfoKind::Script => "fill:#dcedc8",
            InfoKind::ScheduleTask => "fill:#fff9c4",
            InfoKind::Config => "fill:#e1bee7",
            InfoKind::Dependency => "fill:#bbdefb",
            InfoKind::ThirdParty => "fill:#ffccbc",
            InfoKind::Doc => "fill:#d7ccc8",
            InfoKind::Runtime => "fill:#b2ebf2",
            InfoKind::Requirement => "fill:#ffd54f",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum RelationKind {
    Call,
    ReadWrite,
    Reference,
    Dependency,
    Inheritance,
    ConfigRef,
    Deploy,
    Bind,
}

impl RelationKind {
    fn as_str(&self) -> &'static str {
        match self {
            RelationKind::Call => "Call",
            RelationKind::ReadWrite => "ReadWrite",
            RelationKind::Reference => "Reference",
            RelationKind::Dependency => "Dependency",
            RelationKind::Inheritance => "Inheritance",
            RelationKind::ConfigRef => "ConfigRef",
            RelationKind::Deploy => "Deploy",
            RelationKind::Bind => "Bind",
        }
    }
    fn from_str(s: &str) -> Option<RelationKind> {
        match s {
            "Call" => Some(RelationKind::Call),
            "ReadWrite" => Some(RelationKind::ReadWrite),
            "Reference" => Some(RelationKind::Reference),
            "Dependency" => Some(RelationKind::Dependency),
            "Inheritance" => Some(RelationKind::Inheritance),
            "ConfigRef" => Some(RelationKind::ConfigRef),
            "Deploy" => Some(RelationKind::Deploy),
            "Bind" => Some(RelationKind::Bind),
            _ => None,
        }
    }
}

struct InfoNode {
    id: String,
    kind: InfoKind,
    name: String,
    path: String,
    summary: String,
    external: bool,
}

struct RelationEdge {
    id: String,
    from: String,
    to: String,
    kind: RelationKind,
    label: String,
    evidence: String,
    external: bool,
}

struct InfoGraph {
    nodes: Vec<InfoNode>,
    edges: Vec<RelationEdge>,
    node_index: HashMap<String, usize>,
    edge_set: HashSet<String>, // from|kind|to
}

impl InfoGraph {
    fn new() -> InfoGraph {
        InfoGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
            node_index: HashMap::new(),
            edge_set: HashSet::new(),
        }
    }
    fn node_id(kind: InfoKind, key: &str) -> String {
        format!("{}:{}", kind.as_str(), key)
    }
    fn ensure_node(&mut self, kind: InfoKind, key: &str, name: &str, path: &str, summary: &str, external: bool) -> String {
        let id = Self::node_id(kind, key);
        if !self.node_index.contains_key(&id) {
            let idx = self.nodes.len();
            self.nodes.push(InfoNode {
                id: id.clone(),
                kind,
                name: name.to_string(),
                path: path.to_string(),
                summary: summary.to_string(),
                external,
            });
            self.node_index.insert(id.clone(), idx);
        }
        id
    }
    fn add_edge(&mut self, from: &str, to: &str, kind: RelationKind, label: &str, evidence: &str, external: bool) {
        if from == to {
            return;
        }
        let key = format!("{}|{}|{}", from, kind.as_str(), to);
        if self.edge_set.contains(&key) {
            return;
        }
        self.edge_set.insert(key.clone());
        // 截断超长 evidence：防止含内嵌 JSON 的超长代码行被整体复制进图（自引用膨胀的最后一层防线）
        let ev: String = if evidence.chars().count() > EVIDENCE_CAP {
            let t: String = evidence.chars().take(EVIDENCE_CAP).collect();
            format!("{}...(截断,原长{}字)", t, evidence.chars().count())
        } else {
            evidence.to_string()
        };
        self.edges.push(RelationEdge {
            id: key,
            from: from.to_string(),
            to: to.to_string(),
            kind,
            label: label.to_string(),
            evidence: ev,
            external,
        });
    }
}

// ===================== JSON（手写，零依赖） =====================

fn json_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            _ => o.push(c),
        }
    }
    o.push('"');
    o
}

fn graph_to_json(g: &InfoGraph) -> String {
    let mut s = String::from("{\n  \"nodes\": [\n");
    for (i, n) in g.nodes.iter().enumerate() {
        if i > 0 {
            s.push_str(",\n");
        }
            s.push_str(&format!(
            "    {{\"id\": {}, \"kind\": {}, \"name\": {}, \"path\": {}, \"summary\": {}, \"external\": {}}}",
            json_escape(&n.id),
            json_escape(n.kind.as_str()),
            json_escape(&n.name),
            json_escape(&n.path),
            json_escape(&n.summary),
            if n.external { "true" } else { "false" }
        ));
    }
    s.push_str("\n  ],\n  \"edges\": [\n");
    for (i, e) in g.edges.iter().enumerate() {
        if i > 0 {
            s.push_str(",\n");
        }
            s.push_str(&format!(
            "    {{\"id\": {}, \"from\": {}, \"to\": {}, \"kind\": {}, \"label\": {}, \"evidence\": {}, \"external\": {}}}",
            json_escape(&e.id),
            json_escape(&e.from),
            json_escape(&e.to),
            json_escape(e.kind.as_str()),
            json_escape(&e.label),
            json_escape(&e.evidence),
            if e.external { "true" } else { "false" }
        ));
    }
    s.push_str("\n  ]\n}\n");
    s
}

/// 从我们的 JSON 输出中提取所有 "id":"..."，用于 sync 比对（不依赖完整解析器）
fn extract_ids(text: &str) -> HashSet<String> {
    let mut set = HashSet::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 4 < bytes.len() {
        // 匹配 "id" 后接可选空白与冒号再接引号
        if bytes[i] == b'"' && bytes[i + 1] == b'i' && bytes[i + 2] == b'd' && bytes[i + 3] == b'"' {
            let mut j = i + 4;
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b':' {
                j += 1;
                while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'"' {
                    j += 1;
                    let start = j;
                    while j < bytes.len() && bytes[j] != b'"' {
                        j += 1;
                    }
                    let val = &text[start..j];
                    set.insert(val.to_string());
                }
            }
        }
        i += 1;
    }
    set
}

// ===================== 扫描器 =====================

const SKIP_DIRS: &[&str] = &["target", "node_modules", ".git", "vendor", ".workbuddy", "dist", "build"];

/// CI 生成物文件名：不参与扫描（防止把上一次的输出当输入，形成 evidence 自引用指数爆炸）
const SKIP_ARTIFACT_NAMES: &[&str] = &[
    "graph.json",
    "graph.enterprise.json",
    "graph.mmd",
    "guantu.req.json.tmp",
    "ids.txt",
];

/// evidence 截断上限：防止超长行（如 JSON 数据行）被整体复制进证据导致图膨胀
const EVIDENCE_CAP: usize = 300;

fn classify_ext(ext: &str) -> Option<InfoKind> {
    match ext {
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "cs" | "rb" => Some(InfoKind::CodeFile),
        "sql" | "sh" | "bat" | "ps1" => Some(InfoKind::Script),
        "toml" | "yaml" | "yml" | "json" | "env" | "ini" | "conf" => Some(InfoKind::Config),
        "md" | "rst" | "adoc" => Some(InfoKind::Doc),
        _ => None,
    }
}

fn is_binary_ext(ext: &str) -> bool {
    matches!(ext, "wasm" | "png" | "jpg" | "jpeg" | "gif" | "exe" | "dll" | "bin" | "zip" | "tar" | "gz" | "pdf" | "docx" | "xlsx" | "pptx" | "so" | "o")
}

fn rel_path(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/")
}

/// 提取一行里的顶层模块/依赖名（import/use/require/include/from）
fn import_targets(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let low = line;
    // use aaa::bbb;  /  use aaa as x;
    if let Some(rest) = low.strip_prefix("use ") {
        let seg = rest.split(';').next().unwrap_or(rest);
        let top = seg.split(" as ").next().unwrap_or(seg).split("::").next().unwrap_or("").trim();
        if !top.is_empty() && !matches!(top, "crate" | "super" | "self") {
            out.push(top.to_string());
        }
    }
    // import X from 'pkg'  /  import 'pkg'
    if low.starts_with("import ") {
        if let Some(idx) = low.find('\'') {
            let s = &low[idx + 1..];
            if let Some(end) = s.find('\'') {
                out.push(s[..end].to_string());
            }
        } else if let Some(idx) = low.find('"') {
            let s = &low[idx + 1..];
            if let Some(end) = s.find('"') {
                out.push(s[..end].to_string());
            }
        }
    }
    // from pkg import Y
    if let Some(rest) = low.strip_prefix("from ") {
        let pkg = rest.split_whitespace().next().unwrap_or("").trim_matches(&['\'', '"'][..]);
        if !pkg.is_empty() {
            out.push(pkg.to_string());
        }
    }
    // require('pkg')
    if low.contains("require(") {
        if let Some(idx) = low.find('\'') {
            let s = &low[idx + 1..];
            if let Some(end) = s.find('\'') {
                out.push(s[..end].to_string());
            }
        } else if let Some(idx) = low.find('"') {
            let s = &low[idx + 1..];
            if let Some(end) = s.find('"') {
                out.push(s[..end].to_string());
            }
        }
    }
    // #include "foo.h"
    if low.starts_with("#include") {
        for q in ['"', '<'] {
            if let Some(idx) = low.find(q) {
                let s = &low[idx + 1..];
                if let Some(end) = s.find(if q == '"' { '"' } else { '>' }) {
                    out.push(s[..end].to_string());
                }
            }
        }
    }
    out.retain(|s| !s.is_empty() && !s.starts_with('.') && !s.starts_with('/'));
    out
}

/// 提取 Rust `mod x;` / `pub mod x;` / `mod x {` 的模块名
fn mod_targets(line: &str) -> Vec<String> {
    let t = line.trim();
    let rest = if let Some(r) = t.strip_prefix("pub mod ") {
        r
    } else if let Some(r) = t.strip_prefix("mod ") {
        r
    } else {
        return Vec::new();
    };
    let name = rest
        .split(|c| c == '{' || c == ';' || c == ' ' || c == '\t')
        .next()
        .unwrap_or("")
        .trim();
    if name.is_empty() || matches!(name, "crate" | "self" | "super") {
        Vec::new()
    } else {
        vec![name.to_string()]
    }
}

fn scan(root: &Path) -> InfoGraph {
    let mut g = InfoGraph::new();
    // 第一遍：收集文件并建立节点
    let mut files: Vec<(PathBuf, InfoKind, String)> = Vec::new(); // (abs, kind, rel)
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for ent in entries.flatten() {
            let p = ent.path();
            if p.is_dir() {
                let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                if SKIP_DIRS.contains(&name.as_str()) {
                    continue;
                }
                stack.push(p);
            } else {
                let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                // 跳过 CI 生成物：防止把上一次输出的 graph 当输入再次扫描（evidence 自引用膨胀根因）
                if SKIP_ARTIFACT_NAMES.contains(&fname.as_str()) {
                    continue;
                }
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                if is_binary_ext(&ext) {
                    continue;
                }
                if let Some(kind) = classify_ext(&ext) {
                    let rel = rel_path(root, &p);
                    files.push((p.clone(), kind, rel.clone()));
                    g.ensure_node(kind, &rel, p.file_name().unwrap_or_default().to_string_lossy().as_ref(), &rel, "", false);
                }
            }
        }
    }
    // basename -> node id 映射（用于解析 import 到本地文件）
    let mut base_map: HashMap<String, String> = HashMap::new();
    // full relpath -> node id 映射（用于解析 mod 声明到本地文件）
    let mut rel_map: HashMap<String, String> = HashMap::new();
    // crate 名 -> crate 目录(rel)，用于把 use crate 解析到 src/lib.rs
    let mut crate_map: HashMap<String, String> = HashMap::new();
    for (p, kind, rel) in &files {
        rel_map.entry(rel.clone()).or_insert_with(|| InfoGraph::node_id(*kind, rel));
        if *kind == InfoKind::CodeFile || *kind == InfoKind::Script {
            if let Some(stem) = rel.rsplit('/').next().and_then(|f| f.rsplit_once('.')) {
                base_map.entry(stem.0.to_string()).or_insert_with(|| InfoGraph::node_id(*kind, rel));
            }
        }
        if rel.ends_with("Cargo.toml") {
            if let Ok(c) = fs::read_to_string(p) {
                let mut in_pkg = false;
                for line in c.lines() {
                    let t = line.trim();
                    if t.starts_with('[') {
                        in_pkg = t == "[package]";
                        continue;
                    }
                    if in_pkg && t.starts_with("name") && t.contains('=') {
                        let name = t.split('=').nth(1).unwrap_or("").trim().trim_matches('"').to_string();
                        if !name.is_empty() {
                            let dir = rel.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default();
                            // Rust 代码用下划线引用 crate，Cargo.toml 名可能含连字符，两者都建索引
                            crate_map.entry(name.replace('-', "_")).or_insert(dir.clone());
                            crate_map.entry(name).or_insert(dir);
                        }
                    }
                }
            }
        }
    }
    // 第二遍：解析内容，建立边
    for (path, kind, rel) in &files {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        // Cargo.toml [dependencies] -> Dependency 边
        if rel.ends_with("Cargo.toml") {
            let mut in_deps = false;
            for line in content.lines() {
                let t = line.trim();
                if t.starts_with('[') {
                    in_deps = t.starts_with("[dependencies") || t.starts_with("[dev-dependencies");
                    continue;
                }
                if in_deps && t.contains(" = ") && !t.starts_with('#') {
                    let name = t.split('=').next().unwrap_or("").trim();
                    if !name.is_empty() {
                        let dep_id = g.ensure_node(InfoKind::Dependency, name, name, name, "cargo dependency", true);
                        g.add_edge(&InfoGraph::node_id(*kind, rel), &dep_id, RelationKind::Dependency, name, &format!("{}: [dependencies] {}", rel, name), true);
                    }
                }
            }
        }
        // SQL: CREATE TABLE / FOREIGN KEY REFERENCES
        if *kind == InfoKind::Script && rel.ends_with(".sql") {
            for (lno, line) in content.lines().enumerate() {
                let up = line.to_uppercase();
                if up.contains("CREATE TABLE") {
                    if let Some(name) = up.split("CREATE TABLE").nth(1).and_then(|s| s.split_whitespace().nth(0)) {
                        let tname = name.trim_matches(|c| c == '`' || c == '"' || c == '(' || c == '[' || c == ']');
                        let data_id = g.ensure_node(InfoKind::Data, tname, tname, tname, "sql table", false);
                        g.add_edge(&InfoGraph::node_id(*kind, rel), &data_id, RelationKind::ReadWrite, "defines", &format!("{}:{} CREATE TABLE {}", rel, lno + 1, tname), false);
                    }
                }
                if up.contains("REFERENCES") {
                    if let Some(name) = up.split("REFERENCES").nth(1).and_then(|s| s.split_whitespace().nth(0)) {
                        let tname = name.trim_matches(|c| c == '`' || c == '"' || c == '(' || c == '[' || c == ']');
                        let data_id = g.ensure_node(InfoKind::Data, tname, tname, tname, "sql table", false);
                        g.add_edge(&InfoGraph::node_id(*kind, rel), &data_id, RelationKind::Dependency, "fk", &format!("{}:{} REFERENCES {}", rel, lno + 1, tname), false);
                    }
                }
            }
        }
        // import/use/require -> Reference 边
        for (lno, line) in content.lines().enumerate() {
            // Rust mod 声明 -> 本地模块文件（Reference 边）
            if *kind == InfoKind::CodeFile && rel.ends_with(".rs") {
                for m in mod_targets(line) {
                    let parent = rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
                    let base = if parent.is_empty() { m.clone() } else { format!("{}/{}", parent, m) };
                    let from_id = InfoGraph::node_id(*kind, rel);
                    let mut linked = false;
                    for cand in [format!("{}.rs", base), format!("{}/mod.rs", base)] {
                        if let Some(tid) = rel_map.get(&cand) {
                            g.add_edge(&from_id, tid, RelationKind::Reference, &m, &format!("{}:{} mod {}", rel, lno + 1, m), false);
                            linked = true;
                            break;
                        }
                    }
                    if !linked {
                        let ext_id = g.ensure_node(InfoKind::Dependency, &m, &m, &m, "unresolved mod", true);
                        g.add_edge(&from_id, &ext_id, RelationKind::Reference, &m, &format!("{}:{} mod {}", rel, lno + 1, m), true);
                    }
                }
            }
            let targets = import_targets(line);
            for tg in targets {
                let top = tg.split("::").next().unwrap_or(&tg).split('/').next().unwrap_or(&tg);
                let from_id = InfoGraph::node_id(*kind, rel);
                // 命中本地文件？
                if let Some(tid) = base_map.get(top) {
                    g.add_edge(&from_id, tid, RelationKind::Reference, top, &format!("{}:{} {}", rel, lno + 1, line.trim()), false);
                } else if let Some(dir) = crate_map.get(top) {
                    // 命中 workspace 内部 crate -> 链接到其 src/lib.rs / src/main.rs
                    let mut linked = false;
                    for cand in [format!("{}/src/lib.rs", dir), format!("{}/src/main.rs", dir)] {
                        if let Some(tid) = rel_map.get(&cand) {
                            g.add_edge(&from_id, tid, RelationKind::Reference, top, &format!("{}:{} {}", rel, lno + 1, line.trim()), false);
                            linked = true;
                            break;
                        }
                    }
                    if !linked {
                        let ext_id = g.ensure_node(InfoKind::Dependency, top, top, top, "internal crate", true);
                        g.add_edge(&from_id, &ext_id, RelationKind::Reference, top, &format!("{}:{} {}", rel, lno + 1, line.trim()), true);
                    }
                } else {
                    // 外部依赖/第三方
                    let ext_id = g.ensure_node(InfoKind::Dependency, top, top, top, "external ref", true);
                    g.add_edge(&from_id, &ext_id, RelationKind::Reference, top, &format!("{}:{} {}", rel, lno + 1, line.trim()), true);
                }
            }
            // 定时任务迹象
            let low = line.to_lowercase();
            if low.contains("cron") || low.contains("set_interval") || low.contains("schedule_at") || low.contains("tokio::spawn") {
                let st_id = g.ensure_node(InfoKind::ScheduleTask, &format!("{}/scheduler", rel), &format!("{}#scheduler", rel), rel, "detected scheduler", false);
                g.add_edge(&InfoGraph::node_id(*kind, rel), &st_id, RelationKind::Reference, "scheduler", &format!("{}:{} {}", rel, lno + 1, "scheduler hint"), false);
            }
        }
    }
    g
}

// ===================== 校验器 =====================

struct Issue {
    code: &'static str,
    msg: String,
}

fn validate(g: &InfoGraph) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut indeg: HashMap<String, usize> = HashMap::new();
    let mut outdeg: HashMap<String, usize> = HashMap::new();
    for n in &g.nodes {
        indeg.entry(n.id.clone()).or_insert(0);
        outdeg.entry(n.id.clone()).or_insert(0);
    }
    // 悬空边 / 重复 id 由 ensure 保证；这里查悬空与未证实
    for e in &g.edges {
        if !g.node_index.contains_key(&e.from) {
            issues.push(Issue { code: "GR-E2", msg: format!("悬空边 from 不存在: {}", e.from) });
        }
        if !g.node_index.contains_key(&e.to) {
            issues.push(Issue { code: "GR-E2", msg: format!("悬空边 to 不存在: {}", e.to) });
        }
        if e.evidence.is_empty() {
            issues.push(Issue { code: "GR-E3", msg: format!("未证实关系(缺evidence): {} -> {}", e.from, e.to) });
        }
        *outdeg.entry(e.from.clone()).or_insert(0) += 1;
        *indeg.entry(e.to.clone()).or_insert(0) += 1;
    }
    // 孤儿节点（核心类）
    for n in &g.nodes {
        let deg = indeg[&n.id] + outdeg[&n.id];
        let core = matches!(n.kind, InfoKind::CodeFile | InfoKind::Script | InfoKind::Data | InfoKind::Interface | InfoKind::Function);
        if core && deg == 0 {
            issues.push(Issue { code: "GR-E1", msg: format!("孤儿节点: {} ({})", n.id, n.path) });
        }
    }
    // 信息孤岛：连通分量规模（BFS，仅统计非 external）
    let comp_size = connected_component_sizes(g);
    for n in &g.nodes {
        if n.external {
            continue;
        }
        let s = comp_size.get(&n.id).copied().unwrap_or(0);
        if s <= 2 && matches!(n.kind, InfoKind::Doc | InfoKind::Config | InfoKind::Business) {
            issues.push(Issue { code: "GR-E5", msg: format!("信息孤岛: {} (连通分量={})", n.id, s) });
        }
    }
    // 隐性依赖：存在 external 引用但未建模为 ThirdParty（Reference 到 Dependency 视为已建模，此处仅提示第三方未细化）
    for e in &g.edges {
        if e.kind == RelationKind::Reference && e.external && e.to.starts_with("Dependency:") {
            // 已建模为依赖，OK
        }
    }
    issues
}

fn connected_component_sizes(g: &InfoGraph) -> HashMap<String, usize> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for n in &g.nodes {
        adj.entry(&n.id).or_default();
    }
    for e in &g.edges {
        adj.entry(&e.from).or_default().push(&e.to);
        adj.entry(&e.to).or_default().push(&e.from);
    }
    let mut seen: HashSet<&str> = HashSet::new();
    let mut sizes: HashMap<String, usize> = HashMap::new();
    for n in &g.nodes {
        if seen.contains(n.id.as_str()) {
            continue;
        }
        // BFS
        let mut stack = vec![n.id.as_str()];
        let mut members: Vec<String> = Vec::new();
        seen.insert(n.id.as_str());
        while let Some(cur) = stack.pop() {
            members.push(cur.to_string());
            if let Some(nei) = adj.get(cur) {
                for nb in nei {
                    if seen.insert(nb) {
                        stack.push(nb);
                    }
                }
            }
        }
        for m in &members {
            sizes.insert(m.clone(), members.len());
        }
    }
    sizes
}

// ===================== REQ 骨架注入 / 偏离检测 =====================

/// 极简 JSON 值（用于解析 guantu.req.json 规格，零依赖）
///
/// `Bool`/`Num` 的载荷本工具当前只需"正确跳过"而不需读取（关图规格中
/// 结构性字段均为 string/array/object）；保留载荷以维持 JSON 值模型完整，
/// 使解析器可无损处理任意合法 JSON，故此处显式豁免 dead_code。
#[allow(dead_code)]
enum Jv {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Jv>),
    Obj(Vec<(String, Jv)>),
}
impl Jv {
    fn get_arr(&self, key: &str) -> Option<&Vec<Jv>> {
        if let Jv::Obj(o) = self {
            for (k, v) in o {
                if k == key {
                    if let Jv::Arr(a) = v {
                        return Some(a);
                    }
                }
            }
        }
        None
    }
    fn get_str(&self, key: &str) -> Option<&str> {
        if let Jv::Obj(o) = self {
            for (k, v) in o {
                if k == key {
                    if let Jv::Str(s) = v {
                        return Some(s);
                    }
                }
            }
        }
        None
    }
}
struct JParser<'a> {
    c: &'a [u8],
    i: usize,
}
impl<'a> JParser<'a> {
    fn skip_ws(&mut self) {
        while self.i < self.c.len() && (self.c[self.i] as char).is_whitespace() {
            self.i += 1;
        }
    }
    fn parse_value(&mut self) -> Jv {
        self.skip_ws();
        if self.i >= self.c.len() {
            return Jv::Null;
        }
        match self.c[self.i] {
            b'{' => self.parse_obj(),
            b'[' => self.parse_arr(),
            b'"' => Jv::Str(self.parse_str()),
            b't' | b'f' => self.parse_bool(),
            b'n' => {
                self.i += 4;
                Jv::Null
            }
            _ => self.parse_num(),
        }
    }
    fn parse_obj(&mut self) -> Jv {
        self.i += 1;
        let mut v = Vec::new();
        self.skip_ws();
        if self.i < self.c.len() && self.c[self.i] == b'}' {
            self.i += 1;
            return Jv::Obj(v);
        }
        loop {
            self.skip_ws();
            let key = self.parse_str();
            self.skip_ws();
            if self.i < self.c.len() && self.c[self.i] == b':' {
                self.i += 1;
            }
            let val = self.parse_value();
            v.push((key, val));
            self.skip_ws();
            if self.i < self.c.len() && self.c[self.i] == b',' {
                self.i += 1;
                continue;
            }
            if self.i < self.c.len() && self.c[self.i] == b'}' {
                self.i += 1;
                break;
            }
            break;
        }
        Jv::Obj(v)
    }
    fn parse_arr(&mut self) -> Jv {
        self.i += 1;
        let mut v = Vec::new();
        self.skip_ws();
        if self.i < self.c.len() && self.c[self.i] == b']' {
            self.i += 1;
            return Jv::Arr(v);
        }
        loop {
            let val = self.parse_value();
            v.push(val);
            self.skip_ws();
            if self.i < self.c.len() && self.c[self.i] == b',' {
                self.i += 1;
                continue;
            }
            if self.i < self.c.len() && self.c[self.i] == b']' {
                self.i += 1;
                break;
            }
            break;
        }
        Jv::Arr(v)
    }
    /// 解析 JSON 字符串字面量。
    ///
    /// 关键点（中文需求规格必须正确）：按**原始字节**累积后统一按 UTF-8 解码，
    /// 绝不可 `byte as char`（那会把 UTF-8 多字节序列拆成 Latin-1 码位而乱码）。
    /// 同时完整支持 `\uXXXX` 及 UTF-16 代理对（emoji / 补充平面字符）。
    fn parse_str(&mut self) -> String {
        if self.i < self.c.len() && self.c[self.i] == b'"' {
            self.i += 1;
        }
        let mut buf: Vec<u8> = Vec::new();
        while self.i < self.c.len() {
            let ch = self.c[self.i];
            if ch == b'"' {
                self.i += 1;
                break;
            }
            if ch == b'\\' && self.i + 1 < self.c.len() {
                self.i += 1;
                let e = self.c[self.i];
                match e {
                    b'"' => buf.push(b'"'),
                    b'\\' => buf.push(b'\\'),
                    b'n' => buf.push(b'\n'),
                    b't' => buf.push(b'\t'),
                    b'r' => buf.push(b'\r'),
                    b'/' => buf.push(b'/'),
                    b'b' => buf.push(0x08),
                    b'f' => buf.push(0x0c),
                    b'u' => {
                        // self.i 指向 'u'，其后 4 位十六进制
                        if let Some(hi) = self.read_hex4() {
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                // 高位代理：尝试消费紧随的 \uXXXX 低位代理
                                let save = self.i;
                                if self.i + 2 < self.c.len()
                                    && self.c[self.i + 1] == b'\\'
                                    && self.c[self.i + 2] == b'u'
                                {
                                    self.i += 2; // 移到低位代理的 'u'
                                    match self.read_hex4() {
                                        Some(lo) if (0xDC00..0xE000).contains(&lo) => {
                                            0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                                        }
                                        _ => {
                                            self.i = save;
                                            0xFFFD
                                        }
                                    }
                                } else {
                                    0xFFFD
                                }
                            } else if (0xDC00..0xE000).contains(&hi) {
                                0xFFFD // 孤立低位代理
                            } else {
                                hi
                            };
                            let c = char::from_u32(cp).unwrap_or('\u{FFFD}');
                            let mut tmp = [0u8; 4];
                            buf.extend_from_slice(c.encode_utf8(&mut tmp).as_bytes());
                        }
                    }
                    other => buf.push(other), // 未知转义：保留原字节
                }
                self.i += 1;
            } else {
                buf.push(ch); // 原始字节直通，多字节 UTF-8 序列得以保全
                self.i += 1;
            }
        }
        String::from_utf8_lossy(&buf).into_owned()
    }
    /// 读取 `\u` 之后的 4 位十六进制；成功时 self.i 停在最后一位十六进制字符上
    fn read_hex4(&mut self) -> Option<u32> {
        if self.i + 4 >= self.c.len() {
            return None;
        }
        let mut v: u32 = 0;
        for k in 1..=4 {
            let d = (self.c[self.i + k] as char).to_digit(16)?;
            v = v * 16 + d;
        }
        self.i += 4;
        Some(v)
    }
    fn parse_bool(&mut self) -> Jv {
        if self.i + 4 <= self.c.len() && &self.c[self.i..self.i + 4] == b"true" {
            self.i += 4;
            Jv::Bool(true)
        } else if self.i + 5 <= self.c.len() && &self.c[self.i..self.i + 5] == b"false" {
            self.i += 5;
            Jv::Bool(false)
        } else {
            Jv::Null
        }
    }
    fn parse_num(&mut self) -> Jv {
        let start = self.i;
        while self.i < self.c.len() && {
            let c = self.c[self.i];
            (c as char).is_numeric() || c == b'.' || c == b'-' || c == b'e' || c == b'E' || c == b'+'
        } {
            self.i += 1;
        }
        let s = std::str::from_utf8(&self.c[start..self.i]).unwrap_or("0");
        Jv::Num(s.parse().unwrap_or(0.0))
    }
}
fn json_parse(s: &str) -> Jv {
    // 容忍 UTF-8 BOM(EF BB BF)：Windows 编辑器/PowerShell 重定向常写入 BOM，
    // 若不剥离会导致解析静默失败（曾致 skeleton 空跑注入 0 个 REQ）。
    let b = s.as_bytes();
    let start = if b.len() >= 3 && b[0] == 0xEF && b[1] == 0xBB && b[2] == 0xBF { 3 } else { 0 };
    let mut p = JParser { c: &b[start..], i: 0 };
    p.parse_value()
}

/// 注入 REQ 需求根节点 + 六维绑定骨架，输出企业级关图
fn cmd_skeleton(graph_p: &str, spec_p: &str, out_p: &str) {
    let mut g = load_graph(graph_p);
    let spec_txt = fs::read_to_string(spec_p).unwrap_or_else(|_| panic!("读取 spec 失败: {}", spec_p));
    let spec = json_parse(&spec_txt);
    let mut req_count = 0usize;
    let mut bind_count = 0usize;
    if let Some(reqs) = spec.get_arr("requirements") {
        for r in reqs {
            let id = r.get_str("id").unwrap_or_default();
            if id.is_empty() {
                continue;
            }
            let name = r.get_str("name").unwrap_or(id);
            let domain = r.get_str("domain").unwrap_or("");
            let status = r.get_str("status").unwrap_or("");
            let summary = format!("需求根节点 | 域={} | 状态={}", domain, status);
            g.ensure_node(InfoKind::Requirement, id, name, domain, &summary, false);
            req_count += 1;
        }
    }
    if let Some(binds) = spec.get_arr("bindings") {
        for b in binds {
            let req = b.get_str("req").unwrap_or_default();
            let to = b.get_str("to").unwrap_or_default();
            let label = b.get_str("label").unwrap_or("req-bind");
            if req.is_empty() || to.is_empty() {
                continue;
            }
            let rid = InfoGraph::node_id(InfoKind::Requirement, req);
            if !g.node_index.contains_key(&rid) {
                g.ensure_node(InfoKind::Requirement, req, req, "", "auto-created binding target", false);
                req_count += 1;
            }
            if g.node_index.contains_key(to) {
                g.add_edge(&rid, to, RelationKind::Bind, label, &format!("guantu-skeleton: {} -> {}", rid, to), false);
                bind_count += 1;
            } else {
                eprintln!("警告: 绑定目标节点不存在，跳过: {} -> {}", rid, to);
            }
        }
    }
    // 铁律：禁止静默空跑。规格解析失败/为空时必须响亮失败，否则下游
    // deviate/门禁会误判为"通过"，形成虚假合规。
    if req_count == 0 {
        eprintln!("错误: 从 spec 解析到 0 个需求根 —— 规格为空或格式不符（期望顶层 \"requirements\": [...]）");
        eprintln!("      spec: {}", spec_p);
        eprintln!("      请检查文件是否为合法 JSON 对象；BOM 已自动容忍，无需手工处理。");
        process::exit(2);
    }
    let json = graph_to_json(&g);
    fs::write(out_p, json).expect("写入失败");
    println!("骨架注入完成：需求根 {} 个，绑定边 {} 条 -> {}", req_count, bind_count, out_p);
    if bind_count == 0 {
        eprintln!("警告: 绑定边 0 条 —— 通常是关图构建根目录与 bindings 路径不一致");
        eprintln!("      （bindings 形如 CodeFile:crates/x/src/lib.rs 时，应以仓库根构图：build --root .）");
    }
}

/// 偏离检测：REQ 根可达性分析（GR-STD 需求锚定与偏离治理）
fn cmd_deviate(graph_p: &str) {
    let g = load_graph(graph_p);
    let roots: Vec<String> = g.nodes.iter().filter(|n| n.kind == InfoKind::Requirement).map(|n| n.id.clone()).collect();
    if roots.is_empty() {
        println!("偏离检测跳过：图中无 Requirement 根节点。请先运行 skeleton 注入 REQ。");
        return;
    }
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for n in &g.nodes {
        adj.entry(&n.id).or_default();
    }
    for e in &g.edges {
        adj.entry(&e.from).or_default().push(&e.to);
        adj.entry(&e.to).or_default().push(&e.from);
    }
    let mut seen: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = Vec::new();
    for r in &roots {
        if seen.insert(r.as_str()) {
            stack.push(r.as_str());
        }
    }
    while let Some(cur) = stack.pop() {
        if let Some(nei) = adj.get(cur) {
            for nb in nei {
                if seen.insert(nb) {
                    stack.push(nb);
                }
            }
        }
    }
    let mut outdeg: HashMap<String, usize> = HashMap::new();
    for e in &g.edges {
        *outdeg.entry(e.from.clone()).or_insert(0) += 1;
    }
    let mut issues: Vec<Issue> = Vec::new();
    let mut uncovered: Vec<&InfoNode> = Vec::new();
    let mut core_total = 0usize;
    for n in &g.nodes {
        if n.kind == InfoKind::Requirement {
            if outdeg.get(&n.id).copied().unwrap_or(0) == 0 {
                issues.push(Issue { code: "GR-E7", msg: format!("需求未分解(无绑定边): {} ({})", n.id, n.name) });
            }
            continue;
        }
        if n.external {
            continue;
        }
        let core = matches!(n.kind, InfoKind::CodeFile | InfoKind::Script | InfoKind::Data | InfoKind::Interface | InfoKind::Function | InfoKind::Business | InfoKind::Runtime);
        if core {
            core_total += 1;
            if !seen.contains(n.id.as_str()) {
                uncovered.push(n);
            }
        }
    }
    // 报告
    println!("=== 关图偏离检测报告 ===");
    println!("需求根节点: {}", roots.len());
    println!("核心实现节点(非外部): {}", core_total);
    println!("已对齐(可达某 REQ 根): {}", core_total - uncovered.len());
    println!("偏离(无需求溯源 GR-E6): {}", uncovered.len());
    if uncovered.len() > 0 {
        issues.extend(uncovered.iter().map(|n| Issue {
            code: "GR-E6",
            msg: format!("偏离/隐性依赖: {} ({}) 不可达任何 REQ 根", n.id, n.name),
        }));
    }
    let coverage = if core_total > 0 {
        (core_total - uncovered.len()) as f64 / core_total as f64 * 100.0
    } else {
        0.0
    };
    println!("需求对齐覆盖率: {:.1}%", coverage);
    println!("---");
    if issues.is_empty() {
        println!("校验通过：无偏离项，所有核心节点均可溯源至需求根。");
    } else {
        println!("发现问题 {} 项：", issues.len());
        for i in &issues {
            println!("  [{}] {}", i.code, i.msg);
        }
    }
}

// ===================== 导出 / 查询 / 同步 =====================

fn export_mermaid(g: &InfoGraph) -> String {
    let mut s = String::from("graph LR\n");
    for n in &g.nodes {
        let safe = n.id.replace([':', '(', ')', '/', '.', '-'], "_");
        s.push_str(&format!("  {}[\"{}<br/>{}\"]\n", safe, n.kind.as_str(), n.name));
        s.push_str(&format!("  style {} {}\n", safe, n.kind.color()));
    }
    for e in &g.edges {
        let a = e.from.replace([':', '(', ')', '/', '.', '-'], "_");
        let b = e.to.replace([':', '(', ')', '/', '.', '-'], "_");
        s.push_str(&format!("  {} -->|{}| {}\n", a, e.kind.as_str(), b));
    }
    s
}

fn cmd_query(g: &InfoGraph, kind_filter: Option<&str>, name_filter: Option<&str>) {
    let kf = kind_filter.and_then(InfoKind::from_str);
    let mut matched: Vec<&InfoNode> = g
        .nodes
        .iter()
        .filter(|n| {
            let ok_kind = kf.map_or(true, |k| n.kind == k);
            let ok_name = name_filter.map_or(true, |nf| n.name.contains(nf) || n.path.contains(nf));
            ok_kind && ok_name
        })
        .collect();
    matched.sort_by(|a, b| a.id.cmp(&b.id));
    println!("匹配节点 {} 个：", matched.len());
    for n in matched {
        println!("  [{}] {}  path={}  external={}", n.kind.as_str(), n.name, n.path, n.external);
    }
    // 关联边
    if let Some(nf) = name_filter {
        let ids: HashSet<String> = g.nodes.iter().filter(|n| n.name.contains(nf) || n.path.contains(nf)).map(|n| n.id.clone()).collect();
        let mut es: Vec<&RelationEdge> = g.edges.iter().filter(|e| ids.contains(&e.from) || ids.contains(&e.to)).collect();
        es.sort_by(|a, b| a.id.cmp(&b.id));
        println!("关联边 {} 条：", es.len());
        for e in es {
            println!("  {} --{}--> {}  ({})", e.from, e.kind.as_str(), e.to, e.evidence);
        }
    }
}

fn snapshot_text(g: &InfoGraph) -> String {
    let mut ids: Vec<String> = g.nodes.iter().map(|n| n.id.clone()).chain(g.edges.iter().map(|e| e.id.clone())).collect();
    ids.sort();
    ids.join("\n")
}

fn cmd_sync(old_text: &str, new_text: &str) {
    let old = extract_ids(old_text);
    let new = extract_ids(new_text);
    let added: Vec<&String> = new.iter().filter(|x| !old.contains(*x)).collect();
    let removed: Vec<&String> = old.iter().filter(|x| !new.contains(*x)).collect();
    println!("sync 漂移报告：");
    println!("  新增 {} 项", added.len());
    for x in &added {
        println!("    + {}", x);
    }
    println!("  删除 {} 项", removed.len());
    for x in &removed {
        println!("    - {}", x);
    }
    if !added.is_empty() || !removed.is_empty() {
        println!("结果：存在未同步变更（GR-E8），建议阻断合并。");
        process::exit(1);
    } else {
        println!("结果：零漂移，图与代码一致。");
    }
}

// ===================== CLI =====================

// ===================== 需求判重（P9 先判重后立项） =====================
//
// 以关图为「能力指纹库」：给定一条新需求的候选能力节点(capabilities)与候选关系边(edges)，
// 在现有关图子图匹配，给出三类判定：
//   reuse(复用)        —— 全部能力节点 + 关系边均已存在 → 直接编排，不写新代码
//   incremental(增量)  —— 部分能力已存在 → 局部扩展，避免重复造系统
//   new(未命中)        —— 无任何对应能力 → 需新立项（由 --fail-on-new 强制人工确认，杜绝重复造轮子）
// 输出结构化 JSON + 人类可读摘要；--fail-on-new 时未命中即 exit 1（CI 阻断）。
// 纯逻辑在 dedup_requirement（可单测），IO 在 cmd_dedup。

#[derive(Clone, Debug)]
struct DedupResult {
    requirement_id: String,
    requirement_name: String,
    verdict: &'static str,
    similarity: f64,
    total_capabilities: usize,
    matched_capabilities: usize,
    missing_capabilities: Vec<String>,
    total_edges: usize,
    missing_edges: usize,
}

/// 纯函数：在关图 g 上匹配候选需求，返回判重结果（可单测）
fn dedup_requirement(
    g: &InfoGraph,
    id: &str,
    name: &str,
    caps: &[(String, String)],           // (kind, key)
    edges: &[(String, String, String)],  // (from_id, to_id, kind_str)
) -> DedupResult {
    let mut matched = 0usize;
    let mut missing_caps = Vec::new();
    for (kind, key) in caps {
        let nid = format!("{}:{}", kind, key);
        if g.node_index.contains_key(&nid) {
            matched += 1;
        } else {
            missing_caps.push(nid);
        }
    }
    let mut missing_edges = 0usize;
    for (from, to, k) in edges {
        let ek = format!("{}|{}|{}", from, k, to);
        if !g.edge_set.contains(&ek) {
            missing_edges += 1;
        }
    }
    let total = caps.len();
    let similarity = if total == 0 { 0.0 } else { matched as f64 / total as f64 };
    let verdict = if total > 0 && missing_caps.is_empty() && missing_edges == 0 {
        "reuse"
    } else if matched > 0 {
        "incremental"
    } else {
        "new"
    };
    DedupResult {
        requirement_id: id.to_string(),
        requirement_name: name.to_string(),
        verdict,
        similarity,
        total_capabilities: total,
        matched_capabilities: matched,
        missing_capabilities: missing_caps,
        total_edges: edges.len(),
        missing_edges,
    }
}

fn cmd_dedup(graph_p: &str, spec_p: &str, fail_on_new: bool) {
    let g = load_graph(graph_p);
    let spec_txt = fs::read_to_string(spec_p).unwrap_or_else(|_| panic!("读取 spec 失败: {}", spec_p));
    let spec = json_parse(&spec_txt);
    let id = spec.get_str("id").unwrap_or_else(|| "R-unknown");
    let name = spec.get_str("name").unwrap_or_else(|| id);
    let mut caps: Vec<(String, String)> = Vec::new();
    if let Some(arr) = spec.get_arr("capabilities") {
        for c in arr {
            let kind = c.get_str("kind").unwrap_or_default();
            let key = c.get_str("key").unwrap_or_default();
            if !kind.is_empty() && !key.is_empty() {
                caps.push((kind.to_string(), key.to_string()));
            }
        }
    }
    let mut edges: Vec<(String, String, String)> = Vec::new();
    if let Some(arr) = spec.get_arr("edges") {
        for e in arr {
            let from = e.get_str("from").unwrap_or_default();
            let to = e.get_str("to").unwrap_or_default();
            let k = e.get_str("kind").unwrap_or_default();
            if !from.is_empty() && !to.is_empty() && !k.is_empty() {
                edges.push((from.to_string(), to.to_string(), k.to_string()));
            }
        }
    }
    let res = dedup_requirement(&g, id, name, &caps, &edges);
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"requirement_id\": {},\n", json_escape(&res.requirement_id)));
    out.push_str(&format!("  \"requirement_name\": {},\n", json_escape(&res.requirement_name)));
    out.push_str(&format!("  \"verdict\": {},\n", json_escape(res.verdict)));
    out.push_str(&format!("  \"similarity\": {:.3},\n", res.similarity));
    out.push_str(&format!("  \"total_capabilities\": {},\n", res.total_capabilities));
    out.push_str(&format!("  \"matched_capabilities\": {},\n", res.matched_capabilities));
    out.push_str(&format!("  \"missing_capabilities\": {},\n", res.missing_capabilities.len()));
    out.push_str(&format!("  \"total_edges\": {},\n", res.total_edges));
    out.push_str(&format!("  \"missing_edges\": {}\n", res.missing_edges));
    out.push_str("}\n");
    println!("{}", out);
    let action = match res.verdict {
        "reuse" => "命中复用：需求已被现有能力完全覆盖，直接编排，不写新代码",
        "incremental" => "近似增量：部分能力已存在，局部扩展即可，避免重复造系统",
        _ => "未命中：图中无对应能力，需新立项开发（请先确认确有必要时才立项）",
    };
    println!("[dedup] 需求 {}『{}』判定：{}", res.requirement_id, res.requirement_name, action);
    if !res.missing_capabilities.is_empty() {
        println!("[dedup] 缺失能力节点 {} 个：{:?}", res.missing_capabilities.len(), res.missing_capabilities);
    }
    if res.missing_edges > 0 {
        println!("[dedup] 缺失关系边 {} 条", res.missing_edges);
    }
    if fail_on_new && res.verdict == "new" {
        eprintln!("[dedup] --fail-on-new：未命中即阻断，要求人工确认新立项必要性");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample_graph() -> InfoGraph {
        let mut g = InfoGraph::new();
        g.ensure_node(InfoKind::Function, "crates/x/s.rs", "f", "", "", false);
        g.ensure_node(InfoKind::Interface, "crates/x/srv.rs", "srv", "", "", false);
        g.add_edge("Function:crates/x/s.rs", "Interface:crates/x/srv.rs", RelationKind::Call, "calls", "ev", false);
        g
    }
    #[test]
    fn reuse_when_all_present() {
        let g = sample_graph();
        let caps = vec![
            ("Function".to_string(), "crates/x/s.rs".to_string()),
            ("Interface".to_string(), "crates/x/srv.rs".to_string()),
        ];
        let edges = vec![("Function:crates/x/s.rs".to_string(), "Interface:crates/x/srv.rs".to_string(), "Call".to_string())];
        let r = dedup_requirement(&g, "R1", "t", &caps, &edges);
        assert_eq!(r.verdict, "reuse");
        assert_eq!(r.similarity, 1.0);
    }
    #[test]
    fn incremental_when_partial() {
        let g = sample_graph();
        let caps = vec![
            ("Function".to_string(), "crates/x/s.rs".to_string()),
            ("Interface".to_string(), "crates/new.rs".to_string()),
        ];
        let r = dedup_requirement(&g, "R2", "t", &caps, &[]);
        assert_eq!(r.verdict, "incremental");
        assert_eq!(r.matched_capabilities, 1);
    }
    #[test]
    fn new_when_none() {
        let g = sample_graph();
        let caps = vec![("Function".to_string(), "crates/absent.rs".to_string())];
        let r = dedup_requirement(&g, "R3", "t", &caps, &[]);
        assert_eq!(r.verdict, "new");
        assert_eq!(r.similarity, 0.0);
    }
    #[test]
    fn reuse_requires_edges_present() {
        let g = sample_graph();
        let caps = vec![
            ("Function".to_string(), "crates/x/s.rs".to_string()),
            ("Interface".to_string(), "crates/x/srv.rs".to_string()),
        ];
        let edges = vec![("Function:crates/x/s.rs".to_string(), "Interface:crates/x/srv.rs".to_string(), "Deploy".to_string())];
        let r = dedup_requirement(&g, "R4", "t", &caps, &edges);
        assert_eq!(r.verdict, "incremental");
        assert_eq!(r.missing_edges, 1);
    }

    // ---------- JSON 解析器：中文/转义正确性（回归保护）----------
    #[test]
    fn json_parses_chinese_without_mojibake() {
        let j = json_parse(r#"{"id":"REQ-001","name":"多租户配额限流能力"}"#);
        assert_eq!(j.get_str("id"), Some("REQ-001"));
        assert_eq!(j.get_str("name"), Some("多租户配额限流能力"));
    }
    #[test]
    fn json_parses_unicode_escape_and_surrogate_pair() {
        // \u4e2d\u6587 = 中文；\ud83d\ude80 = 🚀（代理对）
        let j = json_parse(r#"{"a":"\u4e2d\u6587","b":"\ud83d\ude80","c":"x\/y\nz"}"#);
        assert_eq!(j.get_str("a"), Some("中文"));
        assert_eq!(j.get_str("b"), Some("🚀"));
        assert_eq!(j.get_str("c"), Some("x/y\nz"));
    }
    #[test]
    fn json_tolerates_utf8_bom() {
        // 真实缺陷回归：docs/graph/guantu.req.json 带 BOM 曾致 skeleton 静默注入 0 个 REQ
        let with_bom = format!("\u{FEFF}{}", r#"{"requirements":[{"id":"D01","name":"算子内核"}]}"#);
        let j = json_parse(&with_bom);
        let arr = j.get_arr("requirements").expect("BOM 后仍应能解析出 requirements");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].get_str("id"), Some("D01"));
        assert_eq!(arr[0].get_str("name"), Some("算子内核"));
    }
    #[test]
    fn json_parses_nested_chinese_capabilities() {
        let spec = r#"{
            "id":"REQ-002","name":"配置加载与加密",
            "capabilities":[{"kind":"CodeFile","key":"璇玑/src/配置.rs"}]
        }"#;
        let j = json_parse(spec);
        let arr = j.get_arr("capabilities").expect("capabilities 应存在");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].get_str("key"), Some("璇玑/src/配置.rs"));
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        process::exit(2);
    }
    let cmd = args[1].as_str();
    match cmd {
        "build" => {
            let root = get_flag(&args, "--root").unwrap_or_else(|| ".".to_string());
            let out = get_flag(&args, "--out").unwrap_or_else(|| "graph.json".to_string());
            let rootp = Path::new(&root);
            if !rootp.exists() {
                eprintln!("根目录不存在: {}", root);
                process::exit(2);
            }
            println!("扫描 {} ...", root);
            let g = scan(rootp);
            let json = graph_to_json(&g);
            fs::write(&out, &json).expect("写入失败");
            println!("完成：节点 {} 个，边 {} 条 -> {}", g.nodes.len(), g.edges.len(), out);
        }
        "validate" => {
            let gp = get_flag(&args, "--graph").unwrap_or_else(|| "graph.json".to_string());
            let g = load_graph(&gp);
            let issues = validate(&g);
            if issues.is_empty() {
                println!("校验通过：无违规项。节点 {} 边 {}", g.nodes.len(), g.edges.len());
            } else {
                println!("发现问题 {} 项：", issues.len());
                for i in &issues {
                    println!("  [{}] {}", i.code, i.msg);
                }
                process::exit(1);
            }
        }
        "export" => {
            let gp = get_flag(&args, "--graph").unwrap_or_else(|| "graph.json".to_string());
            let fmt = get_flag(&args, "--format").unwrap_or_else(|| "mermaid".to_string());
            let g = load_graph(&gp);
            if fmt == "mermaid" {
                print!("{}", export_mermaid(&g));
            } else {
                print!("{}", graph_to_json(&g));
            }
        }
        "query" => {
            let gp = get_flag(&args, "--graph").unwrap_or_else(|| "graph.json".to_string());
            let g = load_graph(&gp);
            let kf = get_flag(&args, "--kind");
            let nf = get_flag(&args, "--name");
            cmd_query(&g, kf.as_deref(), nf.as_deref());
        }
        "snapshot" => {
            let gp = get_flag(&args, "--graph").unwrap_or_else(|| "graph.json".to_string());
            let out = get_flag(&args, "--out").unwrap_or_else(|| "ids.txt".to_string());
            let g = load_graph(&gp);
            fs::write(&out, snapshot_text(&g)).expect("写入失败");
            println!("快照已写 {}", out);
        }
        "skeleton" => {
            let gp = get_flag(&args, "--graph").unwrap_or_else(|| "graph.json".to_string());
            let spec = get_flag(&args, "--spec").expect("需要 --spec");
            let out = get_flag(&args, "--out").unwrap_or_else(|| "graph.enterprise.json".to_string());
            cmd_skeleton(&gp, &spec, &out);
        }
        "deviate" => {
            let gp = get_flag(&args, "--graph").unwrap_or_else(|| "graph.enterprise.json".to_string());
            cmd_deviate(&gp);
        }
        "sync" => {
            let oldp = get_flag(&args, "--old").expect("需要 --old");
            let newp = get_flag(&args, "--new").expect("需要 --new");
            let old_t = fs::read_to_string(&oldp).expect("读取 old 失败");
            let new_t = fs::read_to_string(&newp).expect("读取 new 失败");
            cmd_sync(&old_t, &new_t);
        }
        "dedup" => {
            let gp = get_flag(&args, "--graph").unwrap_or_else(|| "graph.json".to_string());
            let spec = get_flag(&args, "--spec").expect("需要 --spec");
            let fail_on_new = args.iter().any(|a| a == "--fail-on-new");
            cmd_dedup(&gp, &spec, fail_on_new);
        }
        _ => {
            print_help();
            process::exit(2);
        }
    }
}

fn load_graph(p: &str) -> InfoGraph {
    let text = fs::read_to_string(p).unwrap_or_else(|_| panic!("读取图失败: {}", p));
    parse_graph(&text)
}

/// 轻量解析：按我们自己的输出格式重建图（节点/边）
fn parse_graph(text: &str) -> InfoGraph {
    let mut g = InfoGraph::new();
    // 解析 nodes 段与 edges 段
    let mut in_nodes = false;
    let mut in_edges = false;
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line == "\"nodes\": [" {
            in_nodes = true;
            in_edges = false;
            i += 1;
            continue;
        }
        if line == "\"edges\": [" {
            in_nodes = false;
            in_edges = true;
            i += 1;
            continue;
        }
        if line == "]," || line == "]" {
            in_nodes = false;
            in_edges = false;
            i += 1;
            continue;
        }
        if line.starts_with('{') {
            let obj = line.trim_end_matches(',').trim_end_matches('}').trim_start_matches('{');
            let mut map: HashMap<String, String> = HashMap::new();
            for kv in split_top_level(obj) {
                if let Some((k, v)) = kv.split_once(':') {
                    map.insert(k.trim().trim_matches('"').to_string(), v.trim().trim_matches('"').to_string());
                }
            }
            if in_nodes {
                if let (Some(kind), Some(id)) = (map.get("kind").and_then(|s| InfoKind::from_str(s)), map.get("id")) {
                    let name = map.get("name").cloned().unwrap_or_default();
                    let path = map.get("path").cloned().unwrap_or_default();
                    let ext = map.get("external").map_or(false, |v| v == "true");
                    g.ensure_node(kind, &id[id.find(':').map_or(0, |p| p + 1)..], &name, &path, "", ext);
                }
            } else if in_edges {
                if let (Some(from), Some(to), Some(kind)) = (map.get("from"), map.get("to"), map.get("kind").and_then(|s| RelationKind::from_str(s))) {
                    let ev = map.get("evidence").cloned().unwrap_or_default();
                    let ext = map.get("external").map_or(false, |v| v == "true");
                    g.add_edge(from, to, kind, "", &ev, ext);
                }
            }
        }
        i += 1;
    }
    g
}

/// 在顶层按逗号切分；正确处理引号内的逗号与转义引号 \"
fn split_top_level(obj: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    let mut chars = obj.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if in_str => {
                cur.push(c);
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            '"' => {
                in_str = !in_str;
                cur.push(c);
            }
            ',' if !in_str => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

fn get_flag(args: &[String], flag: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == flag {
            return args.get(i + 1).cloned();
        }
        if let Some(v) = args[i].strip_prefix(&format!("{}=", flag)) {
            return Some(v.to_string());
        }
        i += 1;
    }
    None
}

fn print_help() {
    println!("信息关联关系图工具（关图规范 GR-STD-V1.0）");
    println!("用法:");
    println!("  info-graph build   --root <dir> --out graph.json");
    println!("  info-graph validate --graph graph.json");
    println!("  info-graph export  --graph graph.json --format mermaid");
    println!("  info-graph query   --graph graph.json [--kind CodeFile] [--name foo]");
    println!("  info-graph snapshot --graph graph.json --out ids.txt");
    println!("  info-graph sync    --old a.json --new b.json");
    println!("  info-graph dedup   --graph graph.json --spec req.json [--fail-on-new]");
    println!("  info-graph skeleton --graph graph.json --spec guantu.req.json --out graph.enterprise.json");
    println!("  info-graph deviate  --graph graph.enterprise.json");
}
