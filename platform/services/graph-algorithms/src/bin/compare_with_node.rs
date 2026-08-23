//! T12 对账二进制：Rust 7 核心算法单源真相 CLI
//!
//! 用法：
//! ```
//! cargo run --release -p graph-algorithms --bin compare_with_node -- \
//!     --name cnm \
//!     --input fixture.json \
//!     --output out.json
//! # 或通过 stdin/stdout
//! cat fixture.json | cargo run -p graph-algorithms --bin compare_with_node -- \
//!     --name pagerank --input - --output -
//! ```
//!
//! 支持算法名：cnm | ppr | brandes | harmonic | degree | density | raw_expand
//!
//! 精度护栏（严禁修改）：
//!   - PPR_D       = 0.85
//!   - PPR_MAX_ITER = 30
//!   - 不做任何 toFixed / round（全精度输出）
//!   - CNM（模块度贪心凝聚，对外唯一社区算法）
//!   - Brandes 介数 / harmonic 紧密 / RAW 双向展开
//!
//! 输出 shape 与 Node GraphFormulas.js 等价（SPEC-8 对齐）。

use std::collections::HashMap;
use std::io::{Read, Write};

use anyhow::{anyhow, Context, Result};
use graph_algorithms::{
    raw_bidirectional_expand, KnowledgeEdge, KnowledgeGraph, KnowledgeNode, PPR_D, PPR_MAX_ITER,
};
use serde::Deserialize;
use serde_json::{json, Value};

// ---------- 输入 ----------

#[derive(Debug, Deserialize)]
struct InputNode {
    id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default, rename = "nodeType")]
    node_type: Option<String>,
    #[allow(dead_code)]
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct InputEdge {
    source: String,
    target: String,
    #[serde(default)]
    weight: Option<f64>,
    #[serde(default, rename = "relationType")]
    relation_type: Option<String>,
    #[allow(dead_code)]
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct AlgoInput {
    #[serde(default)]
    nodes: Vec<InputNode>,
    #[serde(default)]
    edges: Vec<InputEdge>,
    /// personalizedPageRank 个性化向量 {nodeId: weight}
    #[serde(default)]
    personalization: Option<HashMap<String, f64>>,
    /// 有向图标记（默认 false：度/介数/紧密/社区 使用 RAW 双向展开）
    #[serde(default)]
    directed: Option<bool>,
    /// density 的显式输入（不传则从 nodes/edges 计数）
    #[serde(default, rename = "nodeCount")]
    node_count: Option<usize>,
    #[serde(default, rename = "edgeCount")]
    edge_count: Option<usize>,
    /// CNM 分辨率（默认 1.0；保持向后兼容）
    #[serde(default)]
    resolution: Option<f64>,
}

// ---------- 工具 ----------

fn convert_nodes(nodes: &[InputNode]) -> Vec<KnowledgeNode> {
    nodes
        .iter()
        .map(|n| KnowledgeNode {
            id: n.id.clone(),
            label: n.label.clone().unwrap_or_else(|| n.id.clone()),
            node_type: n.node_type.clone().unwrap_or_else(|| "vertex".to_string()),
            properties: Value::Object(serde_json::Map::new()),
            embedding: None,
            activation: 0.0,
            metadata: HashMap::new(),
        })
        .collect()
}

fn convert_edges(edges: &[InputEdge]) -> Vec<KnowledgeEdge> {
    edges
        .iter()
        .map(|e| KnowledgeEdge {
            source: e.source.clone(),
            target: e.target.clone(),
            weight: e.weight.unwrap_or(1.0),
            relation_type: e
                .relation_type
                .clone()
                .unwrap_or_else(|| "related".to_string()),
            properties: Value::Object(serde_json::Map::new()),
        })
        .collect()
}

fn build_graph(input: &AlgoInput, expand_raw: bool) -> KnowledgeGraph {
    let kns = convert_nodes(&input.nodes);
    let kes = convert_edges(&input.edges);
    let edges_final = if expand_raw {
        raw_bidirectional_expand(&kes)
    } else {
        kes
    };
    // 对 PPR 调用方会 build 两次；此处为通用构建。
    let mut g = KnowledgeGraph::new();
    for kn in kns {
        g.add_node(kn);
    }
    for ke in edges_final {
        // add_edge 在节点缺失时抛错，这里节点已齐，忽略 Err
        let _ = g.add_edge(ke);
    }
    g
}

fn build_graph_with_damping(input: &AlgoInput, expand_raw: bool, d: f64) -> KnowledgeGraph {
    let kns = convert_nodes(&input.nodes);
    let kes = convert_edges(&input.edges);
    let edges_final = if expand_raw {
        raw_bidirectional_expand(&kes)
    } else {
        kes
    };
    let mut g = KnowledgeGraph::with_damping(d);
    for kn in kns {
        g.add_node(kn);
    }
    for ke in edges_final {
        let _ = g.add_edge(ke);
    }
    g
}

fn fmap_to_value(map: &HashMap<String, f64>) -> Value {
    let mut obj = serde_json::Map::new();
    for (k, v) in map {
        obj.insert(k.clone(), json!(v));
    }
    Value::Object(obj)
}

// ---------- 算法 ----------

fn algo_degree(input: &AlgoInput) -> Value {
    // 对齐 GraphFormulas.degreeCentrality(nodes, edges, {expandRaw:true, legacyShape:false})
    // 展开 RAW，再取 degree_centrality（in+out / (N-1)）
    let g = build_graph(input, !input.directed.unwrap_or(false));
    fmap_to_value(&g.degree_centrality())
}

fn algo_ppr(input: &AlgoInput) -> Value {
    // 对齐 GraphFormulas.personalizedPageRank（单源真相 d=PPR_D, maxIter=PPR_MAX_ITER）
    // PageRank 是有向算法：不展开 RAW（除非调用方显式传 RAW 双向边）
    let g = build_graph_with_damping(input, false, PPR_D);
    let pers = input.personalization.clone().unwrap_or_default();
    let result = g.pagerank_personalized(&pers, PPR_MAX_ITER);
    fmap_to_value(&result)
}

fn algo_brandes(input: &AlgoInput) -> Value {
    // 默认无向：RAW 双向展开后再算介数（与 GraphFormulas.betweennessCentrality 默认一致）
    let expand = !input.directed.unwrap_or(false);
    let g = build_graph(input, expand);
    fmap_to_value(&g.betweenness_centrality())
}

fn algo_harmonic(input: &AlgoInput) -> Value {
    // 默认无向：RAW 双向展开（harmonic 对不可达=0，对齐 Node F5）
    let expand = !input.directed.unwrap_or(false);
    let g = build_graph(input, expand);
    fmap_to_value(&g.closeness_centrality())
}

fn algo_cnm(input: &AlgoInput) -> Value {
    // CNM 模块度贪心凝聚；默认无向 RAW 展开（与 GraphFormulas.communityDetectionCNM 一致）
    // resolution 参数保留，但单源真相默认 1.0（实际实现无 resolution 参数，按标准 CNM）
    let _ = input.resolution; // 兼容字段，当前实现按标准 CNM
    let g = build_graph(input, !input.directed.unwrap_or(false));
    let communities = g.detect_communities(usize::MAX); // 让 ΔQ 自行收敛

    let communities_arr: Value = json!(communities
        .iter()
        .map(|c| json!(c.nodes.clone()))
        .collect::<Vec<_>>());

    let mut node_community = serde_json::Map::new();
    for (ci, c) in communities.iter().enumerate() {
        for id in &c.nodes {
            node_community.insert(id.clone(), json!(ci));
        }
    }

    // 最终模块度（复用标准模块化度）：Q = Σ_c [ Σ_in/(2m) − (Σ_tot/(2m))² ]
    let modularity = compute_modularity_scalar(&g, &communities);

    // merges：N - community_count
    let merges = if g.node_count() > communities.len() {
        g.node_count() - communities.len()
    } else {
        0
    };

    json!({
        "communities": communities_arr,
        "nodeCommunity": Value::Object(node_community),
        "modularity": modularity,
        "algorithm": "CNM",
        "merges": merges,
    })
}

fn compute_modularity_scalar(g: &KnowledgeGraph, communities: &[graph_algorithms::Community]) -> f64 {
    let ids: HashMap<String, usize> = g
        .node_ids()
        .into_iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();
    let n = ids.len();
    if n == 0 {
        return 0.0;
    }
    let mut edge_set: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    for e in g.edges() {
        let si = match ids.get(&e.source) {
            Some(i) => *i,
            None => continue,
        };
        let ti = match ids.get(&e.target) {
            Some(i) => *i,
            None => continue,
        };
        if si != ti {
            edge_set.insert((si.min(ti), si.max(ti)));
        }
    }
    let m = edge_set.len() as f64;
    if m <= 0.0 {
        return 0.0;
    }
    let mut node_comm: Vec<Option<usize>> = vec![None; n];
    for (ci, c) in communities.iter().enumerate() {
        for v in &c.nodes {
            if let Some(&i) = ids.get(v) {
                node_comm[i] = Some(ci);
            }
        }
    }
    let k = communities.len().max(1);
    let mut sum_in = vec![0.0f64; k];
    let mut sum_tot = vec![0.0f64; k];
    let mut deg = vec![0f64; n];
    for &(s, t) in &edge_set {
        deg[s] += 1.0;
        deg[t] += 1.0;
    }
    for (i, d) in deg.iter().enumerate() {
        if let Some(c) = node_comm[i] {
            sum_tot[c] += *d;
        }
    }
    for &(s, t) in &edge_set {
        if let (Some(c1), Some(c2)) = (node_comm[s], node_comm[t]) {
            if c1 == c2 {
                sum_in[c1] += 1.0;
            }
        }
    }
    // 标准 Newman 模块化度：Q = Σ_c [ l_c / m − ( Σ_c / (2m) )² ]
    let two_m = 2.0 * m;
    let mut q = 0.0f64;
    for c in 0..k {
        let in_w = sum_in[c];
        let dc = sum_tot[c];
        q += (in_w / m) - (dc / two_m).powi(2);
    }
    q
}

fn algo_density(input: &AlgoInput) -> Value {
    // 对齐 GraphFormulas.density(N, E)：D = 2E/(N(N-1))（无向语义）
    let n = input.node_count.unwrap_or(input.nodes.len());
    let e = input
        .edge_count
        .unwrap_or_else(|| count_raw_unique_edges(input));
    let (value, interpretation) = if n < 2 {
        (0.0, "节点数不足 2，密度无定义，按 0 处理".to_string())
    } else {
        let v = (2.0 * e as f64) / (n as f64 * (n as f64 - 1.0));
        let interp = if v >= 0.8 {
            "高度稠密图，接近完全图".to_string()
        } else if v >= 0.3 {
            "中等密度，连接适中".to_string()
        } else {
            "稀疏图，存在大量未连接节点对".to_string()
        };
        (v, interp)
    };
    json!({
        "value": value,
        "formula": "D = 2E/(N(N-1))",
        "interpretation": interpretation,
    })
}

fn count_raw_unique_edges(input: &AlgoInput) -> usize {
    // GraphFormulas.density 的 E 是「无向 RAW 边条数」（即调用方传多少算多少，不 expand）
    // 但去重 (u,v) 与 (v,u) 同一条，对齐 Node 调用方语义
    let mut set: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for e in &input.edges {
        let s = e.source.clone();
        let t = e.target.clone();
        if s < t {
            set.insert((s, t));
        } else {
            set.insert((t, s));
        }
    }
    set.len()
}

fn algo_raw_expand(input: &AlgoInput) -> Value {
    // 返回 RAW 双向展开后的边数组
    let kes = convert_edges(&input.edges);
    let expanded = raw_bidirectional_expand(&kes);
    let arr: Vec<Value> = expanded
        .iter()
        .map(|e| {
            json!({
                "source": e.source,
                "target": e.target,
                "weight": e.weight,
                "relationType": e.relation_type,
            })
        })
        .collect();
    Value::Array(arr)
}

// ---------- IO ----------

fn read_input(path: &str) -> Result<AlgoInput> {
    let s = if path == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("读 stdin 失败")?;
        buf
    } else {
        std::fs::read_to_string(path).with_context(|| format!("读 input 文件失败: {path}"))?
    };
    let v: AlgoInput = serde_json::from_str(&s).context("解析 input JSON 失败")?;
    Ok(v)
}

fn write_output(path: &str, value: &Value) -> Result<()> {
    let s = serde_json::to_string(value).context("序列化输出 JSON 失败")?;
    if path == "-" {
        let mut out = std::io::stdout().lock();
        out.write_all(s.as_bytes()).context("写 stdout 失败")?;
        out.write_all(b"\n").ok();
    } else {
        std::fs::write(path, (s + "\n").as_bytes())
            .with_context(|| format!("写 output 文件失败: {path}"))?;
    }
    Ok(())
}

// ---------- main ----------

fn parse_args() -> Result<(String, String, String)> {
    let mut name: Option<String> = None;
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--name" => name = Some(args.next().ok_or_else(|| anyhow!("--name 缺少值"))?),
            "--input" => input = Some(args.next().ok_or_else(|| anyhow!("--input 缺少值"))?),
            "--output" => output = Some(args.next().ok_or_else(|| anyhow!("--output 缺少值"))?),
            other => {
                // 允许 -name=xxx / --name=xxx 形式
                if let Some(v) = other.strip_prefix("--name=") {
                    name = Some(v.to_string());
                } else if let Some(v) = other.strip_prefix("--input=") {
                    input = Some(v.to_string());
                } else if let Some(v) = other.strip_prefix("--output=") {
                    output = Some(v.to_string());
                } else if let Some(v) = other.strip_prefix("-name=") {
                    name = Some(v.to_string());
                }
            }
        }
    }
    Ok((
        name.ok_or_else(|| anyhow!("缺少必填 --name <cnm|ppr|brandes|harmonic|degree|density|raw_expand>"))?,
        input.unwrap_or_else(|| "-".to_string()),
        output.unwrap_or_else(|| "-".to_string()),
    ))
}

fn main() -> Result<()> {
    let (name, input_path, output_path) = parse_args()?;
    let input = read_input(&input_path)?;

    let result: Value = match name.as_str() {
        "degree" => algo_degree(&input),
        "ppr" | "pagerank" | "personalizedPageRank" => algo_ppr(&input),
        "brandes" | "betweenness" => algo_brandes(&input),
        "harmonic" | "closeness" => algo_harmonic(&input),
        "cnm" | "community" => algo_cnm(&input),
        "density" => algo_density(&input),
        "raw_expand" | "expandRawEdges" | "rawExpand" => algo_raw_expand(&input),
        other => {
            return Err(anyhow!(
                "未知算法名: {other}；可选值: cnm, ppr, brandes, harmonic, degree, density, raw_expand"
            ))
        }
    };

    write_output(&output_path, &result)?;
    Ok(())
}
