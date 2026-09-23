// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! StorageServer 的 HTTP 薄壳（feature `http`，供 mox-kg-server 等宿主挂载）。
//!
//! 归一化口径：统一 `ApiResponse` 信封（code 即 HTTP 状态，0=成功）；
//! 错误映射 400/404/429/500；写路径 `spawn_blocking`（引擎为同步实现）。
//! 边属性 `Vec<u8>` 在存储层丢失类型 tag，此处按 CDC 同款 UTF-8 lossy 字符串投影。

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use mox_api_protocol::{api_error, api_ok, ApiResponse};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::StorageError;
use crate::graph_codec::PropValue;
use crate::storage_api::Direction;
use crate::storage_server::StorageServer;

/// 挂载前缀由宿主决定（kg-server 用 `/storage/v1`）
pub fn build_storage_router(srv: Arc<StorageServer>) -> Router {
    Router::new()
        .route("/vertex", post(put_vertex))
        .route("/vertex/:vid", get(read_vertex_h).delete(del_vertex_h))
        .route("/edge", post(put_edge))
        .route("/neighbors/:vid", get(neighbors_h))
        .route("/stats", get(stats_h))
        .with_state(srv)
}

type SrvState = State<Arc<StorageServer>>;

fn map_err<T>(e: StorageError) -> ApiResponse<T> {
    let (code, msg) = match &e {
        StorageError::InvalidArgument(m) => (400, m.clone()),
        StorageError::VidNotFound(_)
        | StorageError::EdgeNotFound { .. }
        | StorageError::ShardNotFound(_) => (404, e.to_string()),
        StorageError::ConsumerLagOverThreshold(..) => (429, e.to_string()),
        other => (500, other.to_string()),
    };
    api_error(code, msg)
}

fn prop_to_json(v: &PropValue) -> Value {
    match v {
        PropValue::Null => Value::Null,
        PropValue::Bool(b) => json!(b),
        PropValue::Int(i) => json!(i),
        PropValue::F64(u) => json!(f64::from_bits(*u)),
        PropValue::Str(s) => json!(s),
        PropValue::Bytes(b) => json!({"$hex": hex_str(b)}),
    }
}

fn hex_str(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for byte in b {
        s.push_str(&format!("{byte:02x}"));
    }
    s
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, StorageError> {
    if !hex.len().is_multiple_of(2) {
        return Err(StorageError::InvalidArgument("bad hex len".into()));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| {
            StorageError::InvalidArgument("bad hex digit".into())
        }))
        .collect()
}

fn prop_from_json(v: &Value) -> Result<PropValue, StorageError> {
    Ok(match v {
        Value::Null => PropValue::Null,
        Value::Bool(b) => PropValue::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                PropValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                PropValue::F64(f.to_bits())
            } else {
                return Err(StorageError::InvalidArgument("bad number prop".into()));
            }
        }
        Value::String(s) => PropValue::Str(s.clone()),
        Value::Object(m) => match m.get("$hex").and_then(|x| x.as_str()) {
            Some(hex) => PropValue::Bytes(hex_decode(hex)?),
            None => return Err(StorageError::InvalidArgument("object prop needs $hex".into())),
        },
        Value::Array(_) => {
            return Err(StorageError::InvalidArgument(
                "array prop unsupported".into(),
            ))
        }
    })
}

fn props_from(
    m: BTreeMap<String, Value>,
) -> Result<BTreeMap<String, PropValue>, StorageError> {
    m.iter()
        .map(|(k, v)| prop_from_json(v).map(|pv| (k.clone(), pv)))
        .collect::<Result<_, _>>()
}

fn blocking_panicked<T>(join: tokio::task::JoinError) -> ApiResponse<T> {
    api_error(500, format!("blocking task panicked: {join}"))
}

#[derive(Deserialize)]
struct VertexReq {
    vid: String,
    tag: String,
    #[serde(default)]
    props: BTreeMap<String, Value>,
}

async fn put_vertex(State(srv): SrvState, Json(req): Json<VertexReq>) -> ApiResponse<Value> {
    let props = match props_from(req.props) {
        Ok(p) => p,
        Err(e) => return map_err(e),
    };
    let (vid, tag) = (req.vid, req.tag);
    let srv2 = srv.clone();
    match tokio::task::spawn_blocking(move || srv2.add_vertex(vid, tag, props)).await {
        Err(join) => blocking_panicked(join),
        Ok(Err(e)) => map_err(e),
        Ok(Ok(ack)) => api_ok(json!({
            "vid": ack.vid, "tag": ack.tag, "shard": ack.shard,
            "applied_index": ack.applied_index,
        })),
    }
}

#[derive(Deserialize)]
struct VidPath {
    vid: String,
}

async fn read_vertex_h(State(srv): SrvState, Path(p): Path<VidPath>) -> ApiResponse<Value> {
    let srv2 = srv.clone();
    let vid = p.vid.clone();
    match tokio::task::spawn_blocking(move || srv2.read_vertex(&vid)).await {
        Err(join) => blocking_panicked(join),
        Ok(None) => api_error(404, format!("vid not found: {}", p.vid)),
        Ok(Some((shard, tag, props))) => api_ok(json!({
            "vid": p.vid,
            "shard": shard,
            "tag": tag,
            "props": props
                .iter()
                .map(|(k, pv)| (k.clone(), prop_to_json(pv)))
                .collect::<serde_json::Map<String, Value>>(),
        })),
    }
}

async fn del_vertex_h(State(srv): SrvState, Path(p): Path<VidPath>) -> ApiResponse<Value> {
    let srv2 = srv.clone();
    let vid = p.vid.clone();
    match tokio::task::spawn_blocking(move || srv2.remove_vertex(&vid)).await {
        Err(join) => blocking_panicked(join),
        Ok(Err(e)) => map_err(e),
        Ok(Ok(removed)) => api_ok(json!({ "removed": removed })),
    }
}

#[derive(Deserialize)]
struct EdgeReq {
    src: String,
    dst: String,
    etype: String,
    #[serde(default)]
    rank: i64,
    #[serde(default)]
    weight: Option<f64>,
    #[serde(default)]
    props: BTreeMap<String, Value>,
}

async fn put_edge(State(srv): SrvState, Json(req): Json<EdgeReq>) -> ApiResponse<Value> {
    let props = match props_from(req.props) {
        Ok(p) => p,
        Err(e) => return map_err(e),
    };
    let srv2 = srv.clone();
    let r = tokio::task::spawn_blocking(move || {
        srv2.add_edge(
            req.src,
            req.dst,
            req.etype,
            req.rank,
            req.weight,
            props,
        )
    })
    .await;
    match r {
        Err(join) => blocking_panicked(join),
        Ok(Err(e)) => map_err(e),
        Ok(Ok(ack)) => api_ok(json!({
            "src": ack.src, "dst": ack.dst, "etype": ack.etype,
            "rank": ack.rank, "shard": ack.shard, "applied_index": ack.applied_index,
        })),
    }
}

#[derive(Deserialize)]
struct NeighborQuery {
    #[serde(default = "default_dir")]
    dir: String,
    #[serde(default)]
    etypes: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

fn default_dir() -> String {
    "both".into()
}

async fn neighbors_h(
    State(srv): SrvState,
    Path(p): Path<VidPath>,
    Query(q): Query<NeighborQuery>,
) -> ApiResponse<Value> {
    let direction = match q.dir.as_str() {
        "out" => Direction::Out,
        "in" => Direction::In,
        "both" => Direction::Both,
        other => return api_error(400, format!("dir must be out|in|both, got {other}")),
    };
    let etypes: Vec<String> = q
        .etypes
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let srv2 = srv.clone();
    let vid = p.vid.clone();
    let r = tokio::task::spawn_blocking(move || {
        let et_refs: Vec<&str> = etypes.iter().map(String::as_str).collect();
        srv2.get_neighbors(&vid, direction, &et_refs)
    })
    .await;
    match r {
        Err(join) => blocking_panicked(join),
        Ok(Err(e)) => map_err(e),
        Ok(Ok(list)) => {
            let count = list.len();
            let list = match q.limit {
                Some(n) => list.into_iter().take(n).collect::<Vec<_>>(),
                None => list,
            };
            api_ok(json!({
                "count": count,
                "returned": list.len(),
                "neighbors": list.iter().map(|n| json!({
                    "neighbor_vid": n.neighbor_vid,
                    "direction": n.direction,
                    "etype": n.etype,
                    "rank": n.rank,
                    "weight": n.weight,
                    "props": n.props.iter()
                        .map(|(k, raw)| (k.clone(), json!(String::from_utf8_lossy(raw).into_owned())))
                        .collect::<serde_json::Map<String, Value>>(),
                })).collect::<Vec<_>>(),
            }))
        }
    }
}

async fn stats_h(State(srv): SrvState) -> ApiResponse<Value> {
    let counts = srv.shard_vertex_counts();
    api_ok(json!({
        "shard_count": counts.len(),
        "vertices_total": counts.values().sum::<u64>(),
        "shard_vertex_counts": counts,
        "engine_rocksdb_persist": cfg!(feature = "persist-rocksdb"),
        "wal_sync_before_ack": crate::kv_engine::wal_sync_enabled(),
    }))
}
