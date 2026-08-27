// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 关联图谱 → 落地产物生成器
//!
//! 输入一张 [`AssocGraph`]，输出：
//! 1. `src/gen/c*.rs` —— 每个代码节点一份 Rust 模块骨架（结构 + 方法桩，doc 注释挂回六维链路与数据设计）
//! 2. `src/gen/schema.rs` + `ddl.sql` —— 数据设计 → serde 结构 + PostgreSQL DDL
//! 3. `graph.mmd` —— 可视化关联关系图（Mermaid）
//! 4. `trace_matrix.md` —— 六维溯源矩阵
//! 5. `src/gen/mod.rs` —— 把所有生成模块挂接到 crate
//!
//! 调用方：`examples/gen.rs`。

use crate::assoc::{AssocGraph, EdgeKind, NodeKind};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn sanitize_ident(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn to_snake(s: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if c.is_uppercase() {
            if prev_lower {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            prev_lower = false;
        } else {
            out.push(c);
            prev_lower = c.is_lowercase() || c.is_ascii_digit();
        }
    }
    out
}

fn to_pascal(s: &str) -> String {
    let snake = to_snake(s);
    snake
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn sql_ty(rust: &str) -> &'static str {
    match rust {
        "Uuid" => "UUID",
        "String" => "TEXT",
        "Option<String>" => "TEXT",
        "bool" => "BOOLEAN",
        "i32" => "INTEGER",
        "i64" => "BIGINT",
        "f32" => "REAL",
        "f64" => "DOUBLE PRECISION",
        "DateTime<Utc>" => "TIMESTAMPTZ",
        "serde_json::Value" => "JSONB",
        "Vec<String>" => "TEXT[]",
        _ => "TEXT",
    }
}

/// 取某节点的直接下游（指定边类型）
fn direct_out(graph: &AssocGraph, id: &str, kind: EdgeKind) -> Vec<String> {
    graph
        .edges
        .iter()
        .filter(|e| e.from == id && e.kind == kind)
        .map(|e| e.to.clone())
        .collect()
}

/// 下游可达（按层收集），用于溯源矩阵
fn downstream_by_kind(graph: &AssocGraph, start: &str) -> HashMap<NodeKind, Vec<String>> {
    let steps = [
        (EdgeKind::Satisfies, NodeKind::Feature),
        (EdgeKind::Realizes, NodeKind::Business),
        (EdgeKind::Implements, NodeKind::Algorithm),
        (EdgeKind::Executes, NodeKind::Task),
        (EdgeKind::Codes, NodeKind::Code),
    ];
    let mut map: HashMap<NodeKind, Vec<String>> = HashMap::new();
    let mut frontier = vec![start.to_string()];
    for (ek, nk) in steps {
        let mut next = Vec::new();
        for cur in frontier {
            for e in graph.edges.iter().filter(|e| e.from == cur && e.kind == ek) {
                map.entry(nk).or_default().push(e.to.clone());
                next.push(e.to.clone());
            }
        }
        frontier = next;
    }
    map
}

/// 生成单个代码节点骨架文件
fn emit_code_node(graph: &AssocGraph, code_id: &str, dir: &Path) -> String {
    let node = graph.node(code_id).expect("code node exists");
    let struct_name = to_pascal(&node.label);
    let chain = graph.trace_chain(code_id);
    let chain_str = chain.join(" → ");
    let schemas: Vec<String> = graph
        .data_schemas_of(code_id)
        .iter()
        .filter_map(|s| graph.node(s).map(|n| format!("{}({})", s, n.label)))
        .collect();
    let schemas_str = if schemas.is_empty() {
        "（无状态）".to_string()
    } else {
        schemas.join(", ")
    };
    let deps: Vec<String> = direct_out(graph, code_id, EdgeKind::Depends);
    // 任务→代码 的 Codes 边方向为 Task→Code，故按 to == code_id 收集
    let tasks: Vec<String> = graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Codes && e.to == code_id)
        .filter_map(|e| graph.node(&e.from).map(|n| n.label.clone()))
        .collect();

    let mut methods = String::new();
    for t in &tasks {
        methods.push_str(&format!(
            "    /// 编排任务 `{t}` 的真实落位：打印执行踪迹并返回零值成功。\n    /// 溯源链路: {chain_str}\n    pub fn {t}(&self) {{\n        println!(\"[{struct_name}::{t}] trace={chain_str}; schemas={schemas_debug:?};\");\n    }}\n\n",
            schemas_debug = schemas,
        ));
    }

    let deps_str = if deps.is_empty() {
        String::new()
    } else {
        format!("\n/// 依赖模块: {}\n", deps.join(", "))
    };

    let fname = format!("{}.rs", code_id.to_lowercase());
    let content = format!(
        "//! 代码骨架 · 由关联图谱自动生成（mox_flow_primiflow_svc::assoc::primiflow_seed）\n\
         //! 溯源链路: {chain}\n\
         //! 数据设计: {schemas}\n\
         //! 说明: {doc}\n\
         //! 规格: primiflow/SPEC.md（§7 模块 / §10 DoD）\n\
         {deps}\n\
         #[derive(Debug, Default)]\n\
         pub struct {struct_name} {{}}\n\n\
         impl {struct_name} {{\n\
             pub fn new() -> Self {{ Self::default() }}\n\n\
         {methods}\
         }}\n",
        chain = chain_str,
        schemas = schemas_str,
        doc = node.doc,
        deps = deps_str,
        struct_name = struct_name,
        methods = methods,
    );
    let path = dir.join(&fname);
    fs::write(&path, content).expect("write code file");
    fname
}

/// 生成数据设计：schema.rs（serde 结构）+ ddl.sql（PostgreSQL）
fn emit_schema(graph: &AssocGraph, dir: &Path) -> (String, String) {
    let schemas: Vec<&crate::assoc::Node> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::DataSchema)
        .collect();

    let mut schema_rs = String::from(
        "//! 数据设计 · 由关联图谱自动生成（mox_flow_primiflow_svc::assoc::primiflow_seed）\n\
         //! 对应 primiflow/SPEC.md §4 数据模型\n\
         use serde::{Serialize, Deserialize};\n\
         use uuid::Uuid;\n\
         use chrono::{DateTime, Utc};\n\n",
    );
    let mut ddl = String::from(
        "-- 由关联图谱自动生成 · primiflow/SPEC.md §4\n\
         -- 执行前请确保已启用 pgvector 扩展: CREATE EXTENSION IF NOT EXISTS vector;\n\n",
    );

    for s in &schemas {
        let struct_name = to_pascal(&s.label);
        let table = format!("{}s", to_snake(&s.label));
        schema_rs.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
        schema_rs.push_str(&format!("pub struct {} {{\n", struct_name));
        ddl.push_str(&format!("CREATE TABLE {} (\n", table));
        let mut cols = Vec::new();
        for f in &s.fields {
            let is_pk = f.name == "id";
            schema_rs.push_str(&format!("    pub {}: {},\n", f.name, f.ty));
            let mut col = format!("  {} {}", f.name, sql_ty(&f.ty));
            if is_pk {
                col.push_str(" PRIMARY KEY");
            }
            cols.push(col);
        }
        ddl.push_str(&cols.join(",\n"));
        ddl.push_str("\n);\n\n");
        schema_rs.push_str("}\n\n");
    }

    let schema_path = dir.join("schema.rs");
    fs::write(&schema_path, schema_rs).expect("write schema.rs");
    let ddl_path = dir.join("ddl.sql");
    fs::write(&ddl_path, ddl).expect("write ddl.sql");
    (
        schema_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        ddl_path.file_name().unwrap().to_string_lossy().into_owned(),
    )
}

/// 生成 Mermaid 可视化图
fn emit_mermaid(graph: &AssocGraph, dir: &Path) -> String {
    let mmd = graph.to_mermaid();
    let p = dir.join("graph.mmd");
    fs::write(&p, &mmd).expect("write graph.mmd");
    p.file_name().unwrap().to_string_lossy().into_owned()
}

/// 生成六维溯源矩阵
fn emit_matrix(graph: &AssocGraph, dir: &Path) -> String {
    let mut md = String::from(
        "# PrimiFlow 六维溯源矩阵（由关联图谱自动生成）\n\n\
         | 需求 | 功能 | 业务 | 算法 | 任务 | 代码 | 数据设计 |\n\
         |------|------|------|------|------|------|----------|\n",
    );
    for n in graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Requirement)
    {
        let down = downstream_by_kind(graph, &n.id);
        let join = |k: NodeKind| -> String {
            let mut labels: Vec<String> = down
                .get(&k)
                .map(|ids| {
                    ids.iter()
                        .map(|id| {
                            graph
                                .node(id)
                                .map(|nn| nn.label.clone())
                                .unwrap_or_else(|| id.clone())
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            labels.sort();
            labels.dedup();
            labels.join(" / ")
        };
        let codes = down.get(&NodeKind::Code).cloned().unwrap_or_default();
        let mut schemas = Vec::new();
        for c in &codes {
            for s in graph.data_schemas_of(c) {
                if let Some(sn) = graph.node(&s) {
                    schemas.push(sn.label.clone());
                }
            }
        }
        schemas.sort();
        schemas.dedup();
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            n.label,
            join(NodeKind::Feature),
            join(NodeKind::Business),
            join(NodeKind::Algorithm),
            join(NodeKind::Task),
            join(NodeKind::Code),
            schemas.join(" / "),
        ));
    }
    let p = dir.join("trace_matrix.md");
    fs::write(&p, &md).expect("write trace_matrix.md");
    p.file_name().unwrap().to_string_lossy().into_owned()
}

/// 生成 gen/mod.rs 把各模块挂接进 crate
fn emit_mod_rs(code_files: &[String], schema_file: &str, dir: &Path) {
    let mut content = String::from(
        "//! 自动生成的关联图谱落地代码 · 请勿手改，由 `cargo run --example gen` 重新生成\n",
    );
    content.push_str(&format!(
        "pub mod {};\n",
        sanitize_ident(schema_file.trim_end_matches(".rs"))
    ));
    for f in code_files {
        content.push_str(&format!(
            "pub mod {};\n",
            sanitize_ident(f.trim_end_matches(".rs"))
        ));
    }
    fs::write(dir.join("mod.rs"), content).expect("write mod.rs");
}

/// 一键生成全部落地产物到 `out_dir`（通常为 `crates/primiflow-core/src/gen`）
pub fn emit_all(graph: &AssocGraph, out_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(out_dir)?;
    let mut code_files = Vec::new();
    for n in graph.nodes.iter().filter(|n| n.kind == NodeKind::Code) {
        code_files.push(emit_code_node(graph, &n.id, out_dir));
    }
    let (schema_file, _ddl) = emit_schema(graph, out_dir);
    let mmd = emit_mermaid(graph, out_dir);
    let matrix = emit_matrix(graph, out_dir);
    emit_mod_rs(&code_files, &schema_file, out_dir);

    println!(
        "[primiflow] 关联图谱落地完成:\n  - 代码骨架: {} 个模块\n  - 数据设计: {}\n  - 可视化图: {}\n  - 溯源矩阵: {}",
        code_files.len(),
        schema_file,
        mmd,
        matrix,
    );
    Ok(())
}
