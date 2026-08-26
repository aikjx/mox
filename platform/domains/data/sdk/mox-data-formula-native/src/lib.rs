//! napi-rs 绑定：mox-formulas-core → Node.js 原生模块 @infotopograph/mox-formulas-native
//!
//! 反序列化策略（跨外部类型零侵入）：
//!   JsUnknown → env JSON.stringify → &str → serde_json::from_str::<GraphInput>
//!   输出 Result<serde_json::Value> → napi serde-json feature 自动转为 JS Value。
//!
//! directed 默认 false（项目记忆强制无向 RAW 边展开）。

use mox_data_formula_core::{
    build_csr, density, EdgeInput, NodeInput, PPR_D, PPR_MAX_ITER, PR_EPS,
};
use napi::{Env, JsUnknown, Result};
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap as StdMap;

// ======================================================================
// Serde 反序列化输入（与核心 crate 结构保持一致）
// ======================================================================
#[derive(Debug, Deserialize)]
pub struct GraphInput {
    pub nodes: Vec<NodeInput>,
    pub edges: Vec<EdgeInput>,
    #[serde(default)]
    pub directed: Option<bool>,
}

fn parse_graph(env: Env, input: JsUnknown) -> Result<GraphInput> {
    let s = json_stringify(env, input)?;
    let v: serde_json::Value =
        serde_json::from_str(&s).map_err(|e| napi::Error::from_reason(format!("parse graph json: {e}")))?;
    serde_json::from_value(v).map_err(|e| napi::Error::from_reason(format!("parse graph schema: {e}")))
}

fn json_stringify(env: Env, value: JsUnknown) -> Result<String> {
    let global = env.get_global()?;
    let json: napi::JsObject = global.get_named_property("JSON")?;
    let stringify_fn: napi::JsFunction = json.get_named_property("stringify")?;
    let result: napi::JsUnknown = stringify_fn.call(None, &[value])?;
    result.coerce_to_string()?.into_utf8()?.as_str().map(|s| s.to_string())
}

// ======================================================================
// helpers
// ======================================================================
fn to_value<T: Serialize>(t: &T) -> Result<serde_json::Value> {
    serde_json::to_value(t).map_err(|e| napi::Error::from_reason(format!("ser: {e}")))
}

fn rank_to_map(csr: &mox_data_formula_core::CsrGraph, r: &[f64]) -> StdMap<String, f64> {
    let n = csr.n;
    let mut m = StdMap::with_capacity(n);
    for (i, &v) in r.iter().enumerate().take(n) {
        m.insert(csr.id_of(i).to_string(), v);
    }
    m
}

// ======================================================================
// 元数据
// ======================================================================
#[napi]
pub fn mox_formulas_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[napi(object)]
pub struct Constants {
    pub ppr_d: f64,
    pub ppr_max_iter: i64,
    pub pr_eps: f64,
}

#[napi]
pub fn mox_formulas_constants() -> Constants {
    Constants {
        ppr_d: PPR_D,
        ppr_max_iter: PPR_MAX_ITER as i64,
        pr_eps: PR_EPS,
    }
}

// ======================================================================
// F1 · 密度 density
// ======================================================================
#[derive(Serialize)]
struct DensityView {
    pub value: f64,
    pub formula: String,
    pub interpretation: String,
}

#[napi]
pub fn formulas_density(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let g = parse_graph(env, input)?;
    let directed = g.directed.unwrap_or(false);
    let n = g.nodes.len();
    let m_raw = g.edges.len();
    let d = if directed {
        let e = m_raw as f64;
        let nf = n as f64;
        let v = if nf <= 1.0 { 0.0 } else { e / (nf * (nf - 1.0)) };
        DensityView {
            value: v,
            formula: "D = E / (N·(N−1))（有向）".into(),
            interpretation: mox_data_formula_core::density_interpretation(v).to_string(),
        }
    } else {
        let d = density(n, m_raw);
        DensityView { value: d.value, formula: d.formula, interpretation: d.interpretation }
    };
    to_value(&d)
}

// ======================================================================
// F2 · 度中心性
// ======================================================================
#[napi]
pub fn formulas_degree_centrality(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let g = parse_graph(env, input)?;
    let csr = build_csr(&g.nodes, &g.edges, g.directed.unwrap_or(false));
    to_value(&csr.degree_centrality())
}

// ======================================================================
// F3 · Brandes 介数
// ======================================================================
#[napi]
pub fn formulas_betweenness(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let g = parse_graph(env, input)?;
    let csr = build_csr(&g.nodes, &g.edges, g.directed.unwrap_or(false));
    to_value(&csr.betweenness_centrality())
}

// ======================================================================
// F4 · Harmonic 紧密
// ======================================================================
#[napi]
pub fn formulas_closeness_harmonic(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let g = parse_graph(env, input)?;
    let csr = build_csr(&g.nodes, &g.edges, g.directed.unwrap_or(false));
    to_value(&csr.closeness_harmonic())
}

// ======================================================================
// F5 · PageRank（标准 + 转置）
// ======================================================================
#[derive(Serialize)]
struct PrView {
    pub standard: StdMap<String, f64>,
    pub transposed: StdMap<String, f64>,
    pub diff: f64,
    pub d: f64,
    pub max_iter: i64,
    pub converged_at: i64,
}

#[napi]
pub fn formulas_pagerank(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let g = parse_graph(env, input)?;
    let directed = g.directed.unwrap_or(false);
    let csr = build_csr(&g.nodes, &g.edges, directed);
    let (standard, used) = csr.pagerank();
    let trans_edges: Vec<EdgeInput> = g
        .edges
        .iter()
        .map(|e| EdgeInput {
            source: e.target.clone(),
            target: e.source.clone(),
            weight: e.weight,
            relation_type: e.relation_type.clone(),
        })
        .collect();
    let csr_t = build_csr(&g.nodes, &trans_edges, directed);
    let (transposed, used_t) = csr_t.pagerank();
    let diff = standard.iter().zip(transposed.iter()).map(|(a, b)| (a - b).abs()).sum();
    to_value(&PrView {
        standard: rank_to_map(&csr, &standard),
        transposed: rank_to_map(&csr_t, &transposed),
        diff,
        d: PPR_D,
        max_iter: PPR_MAX_ITER as i64,
        converged_at: used.max(used_t) as i64,
    })
}

// ======================================================================
// F6 · 个性化 PR（激活扩散）
// ======================================================================
#[napi]
pub fn formulas_ppr(
    env: Env,
    input: JsUnknown,
    #[napi(ts_arg_type = "Record<string, number>")] seed: StdMap<String, f64>,
) -> Result<serde_json::Value> {
    let g = parse_graph(env, input)?;
    let csr = build_csr(&g.nodes, &g.edges, g.directed.unwrap_or(false));
    to_value(&csr.ppr_stdmap(&seed))
}

// ======================================================================
// F7 · CNM 社区
// ======================================================================
#[derive(Serialize)]
struct CommunityView {
    pub communities: Vec<Vec<String>>,
    #[serde(rename = "nodeCommunity")]
    pub node_community: StdMap<String, i64>,
    pub modularity: f64,
    pub algorithm: String,
    pub merges: i64,
}

#[napi]
pub fn formulas_community_cnm(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let g = parse_graph(env, input)?;
    let csr = build_csr(&g.nodes, &g.edges, g.directed.unwrap_or(false));
    let r = csr.community_cnm();
    to_value(&CommunityView {
        communities: r.communities,
        node_community: r.node_community.into_iter().map(|(k, v)| (k, v as i64)).collect(),
        modularity: r.modularity,
        algorithm: "CNM (Clauset-Newman-Moore · ΔQ greedy)".into(),
        merges: r.merges as i64,
    })
}

// ======================================================================
// F8 · Newman 模块度
// ======================================================================
#[napi]
pub fn formulas_modularity(
    env: Env,
    input: JsUnknown,
    communities: Vec<Vec<String>>,
) -> Result<f64> {
    let g = parse_graph(env, input)?;
    let csr = build_csr(&g.nodes, &g.edges, g.directed.unwrap_or(false));
    let mut idxs: Vec<Vec<usize>> = Vec::with_capacity(communities.len());
    for c in communities {
        let mut v = Vec::with_capacity(c.len());
        for id in c {
            if let Some(i) = csr.idx_of(&id) {
                v.push(i);
            }
        }
        idxs.push(v);
    }
    Ok(csr.modularity_by_idx(&idxs))
}

// ======================================================================
// F9 · K-Core
// ======================================================================
#[napi]
pub fn formulas_k_core(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let g = parse_graph(env, input)?;
    let csr = build_csr(&g.nodes, &g.edges, g.directed.unwrap_or(false));
    to_value(&csr.k_core())
}

// ======================================================================
// F10 · 特征向量中心性
// ======================================================================
#[napi]
pub fn formulas_eigenvector_centrality(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let g = parse_graph(env, input)?;
    let csr = build_csr(&g.nodes, &g.edges, g.directed.unwrap_or(false));
    let e = csr.eigenvector_centrality(100, 1e-10);
    to_value(&e)
}

// ======================================================================
// F11 · 三角 + 聚集系数
// ======================================================================
#[derive(Serialize)]
struct TriView {
    pub triangles: i64,
    #[serde(rename = "avgLocalClustering")]
    pub avg_local_clustering: f64,
    #[serde(rename = "globalClustering")]
    pub global_clustering: f64,
    pub formula: String,
}

#[napi]
pub fn formulas_triangles_and_clustering(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let g = parse_graph(env, input)?;
    let csr = build_csr(&g.nodes, &g.edges, g.directed.unwrap_or(false));
    let (tri, avg_local, global) = csr.triangle_count_and_clustering();
    to_value(&TriView {
        triangles: tri as i64,
        avg_local_clustering: avg_local,
        global_clustering: global,
        formula: "3Δ/ΣC(d,2); local=2t(v)/(d(v)(d(v)−1))".into(),
    })
}

// ======================================================================
// F12 · 度同配
// ======================================================================
#[derive(Serialize)]
struct AssortView {
    pub r: f64,
    pub formula: String,
    pub interpretation: String,
}

#[napi]
pub fn formulas_assortativity_degree(env: Env, input: JsUnknown) -> Result<serde_json::Value> {
    let g = parse_graph(env, input)?;
    let csr = build_csr(&g.nodes, &g.edges, g.directed.unwrap_or(false));
    let r = csr.assortativity_degree();
    let interpretation = if r.is_nan() {
        "r=NaN：所有节点度相同或图无边".to_string()
    } else if r > 0.2 {
        format!("同配（r={r:.3} > 0.2）：高度节点偏好连接高度节点")
    } else if r < -0.2 {
        format!("异配（r={r:.3} < -0.2）：高度节点偏好连接低度节点")
    } else {
        format!("中性（r={r:.3}）：度相关性弱")
    };
    to_value(&AssortView {
        r,
        formula: "r = Cov(j,k)/(σj·σk)；Pearson 度相关（Newman 无序边双有序对）".into(),
        interpretation,
    })
}
