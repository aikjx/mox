// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟

//! KG 图谱持久化（M5.3）
//!
//! - `load_snapshot` / `save_snapshot`：JSON 快照（含运行时 API 变更）
//! - `load_seed`：从功能需求 seed JSON 灌入初始图谱（edge 生成稳定 id，幂等）

use dashmap::DashMap;
use serde_json::Value;
use std::path::Path;

/// 从快照 JSON 文件读取图谱 `{nodes: [...], edges: [...]}`；文件不存在/解析失败返回 None。
pub fn load_snapshot(path: &str) -> Option<(Vec<Value>, Vec<Value>)> {
    let p = Path::new(path);
    if !p.exists() {
        return None;
    }
    let s = std::fs::read_to_string(p).ok()?;
    let v: Value = serde_json::from_str(&s).ok()?;
    let nodes = v.get("nodes").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    let edges = v.get("edges").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    Some((nodes, edges))
}

/// 把当前内存图谱写为 JSON 快照（自动建目录）。
pub fn save_snapshot(
    path: &str,
    nodes: &DashMap<String, Value>,
    edges: &DashMap<String, Value>,
) -> std::io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    let nodes_arr: Vec<Value> = nodes.iter().map(|e| e.value().clone()).collect();
    let edges_arr: Vec<Value> = edges.iter().map(|e| e.value().clone()).collect();
    let v = serde_json::json!({ "nodes": nodes_arr, "edges": edges_arr });
    let s = serde_json::to_string_pretty(&v)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, s)
}

/// 从功能需求 seed JSON 灌入图谱；edge 按 source->target:relation 生成稳定 id（幂等）。
pub fn load_seed(
    path: &str,
    nodes: &DashMap<String, Value>,
    edges: &DashMap<String, Value>,
) -> std::io::Result<usize> {
    let s = std::fs::read_to_string(path)?;
    let v: Value = serde_json::from_str(&s)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let mut added = 0usize;
    if let Some(arr) = v.get("nodes").and_then(|x| x.as_array()) {
        for n in arr {
            if let Some(id) = n.get("id").and_then(|x| x.as_str()) {
                nodes.insert(id.to_string(), n.clone());
                added += 1;
            }
        }
    }
    if let Some(arr) = v.get("edges").and_then(|x| x.as_array()) {
        for e in arr {
            let src = e.get("source").and_then(|x| x.as_str()).unwrap_or("");
            let tgt = e.get("target").and_then(|x| x.as_str()).unwrap_or("");
            let rel = e.get("relation_type").and_then(|x| x.as_str()).unwrap_or("edge");
            let id = format!("edge-{}-{}-{}", src, tgt, rel);
            let mut edge = e.clone();
            if let Value::Object(m) = &mut edge {
                m.insert("id".into(), Value::String(id.clone()));
            }
            edges.insert(id, edge);
        }
    }
    Ok(added)
}
