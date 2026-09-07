// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Cloud 域路由（L5）：`/cloud/v1/*` —— 本地磁盘对象存储（S3 兼容语义）
//!
//! # 架构
//! - 纯本地磁盘实现（无第三方依赖）：bucket = 根目录下的子目录，object = 目录内文件；
//! - 存储根目录由 `MOX_STORAGE_ROOT` 覆盖（默认 `<gateway>/data/storage`，运行时目录不入库）；
//! - 提供 bucket 生命周期 + 对象读写删，语义对齐 S3（bucket/key/object）最小子集，
//!   为后续 S3 兼容 API（`/s3/*`）与 Volume/FS 域提供统一存储后端；
//! - **路径穿越防护**：bucket/key 均需通过 `sanitize` 校验（拒绝 `.`/`..`/绝对路径/分隔符），
//!   任何非法输入返回 400，杜绝任意文件读写。
//!
//! # 状态
//! `ready`：真实磁盘读写，200 不依赖外部服务。

use mox_api_protocol::{ApiResponse, api_error, api_ok};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path as FsPath, PathBuf},
    sync::Arc,
};

/// 存储状态：根目录
#[derive(Clone)]
pub struct CloudState {
    root: Arc<PathBuf>,
}

impl CloudState {
    pub fn new() -> Self {
        let root = std::env::var("MOX_STORAGE_ROOT").unwrap_or_else(|_| {
            let mut p = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            p.push("data/storage");
            p.to_string_lossy().to_string()
        });
        let root = PathBuf::from(root);
        let _ = fs::create_dir_all(&root);
        Self { root: Arc::new(root) }
    }

    fn bucket_path(&self, bucket: &str) -> Option<PathBuf> {
        if !sanitize(bucket) {
            return None;
        }
        Some(self.root.as_ref().join(bucket))
    }

    fn object_path(&self, bucket: &str, key: &str) -> Option<PathBuf> {
        if !sanitize(bucket) || !sanitize(key) {
            return None;
        }
        Some(self.root.as_ref().join(bucket).join(key))
    }
}

impl Default for CloudState {
    fn default() -> Self {
        Self::new()
    }
}

/// 路径段校验：仅允许字母/数字/下划线/中划线/点（不含连续点），拒绝绝对路径与分隔符
fn sanitize(seg: &str) -> bool {
    !seg.is_empty()
        && seg.len() <= 128
        && seg != "."
        && seg != ".."
        && !seg.contains('/')
        && !seg.contains('\\')
        && !seg.starts_with('.')
        && seg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn bucket_json(name: &str, path: &FsPath) -> Value {
    let object_count = fs::read_dir(path)
        .map(|it| it.filter_map(|e| e.ok()).filter(|e| e.path().is_file()).count())
        .unwrap_or(0);
    let used_bytes = fs::read_dir(path)
        .map(|it| {
            it.filter_map(|e| e.ok())
                .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
                .sum::<u64>()
        })
        .unwrap_or(0);
    json!({
        "name": name,
        "object_count": object_count,
        "used_bytes": used_bytes,
    })
}

/// GET /cloud/v1/buckets —— 列出全部存储桶
async fn cloud_list_buckets(State(s): State<CloudState>) -> ApiResponse<Value> {
    let root = s.root.as_ref();
    let mut buckets = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for e in entries.flatten() {
            if e.path().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    buckets.push(bucket_json(name, &e.path()));
                }
            }
        }
    }
    buckets.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    api_ok(json!({ "buckets": buckets, "root": root.to_string_lossy() }))
}

/// POST /cloud/v1/buckets —— 创建存储桶
async fn cloud_create_bucket(
    State(s): State<CloudState>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");
    match s.bucket_path(name) {
        Some(p) => match fs::create_dir_all(&p) {
            Ok(_) => api_ok(json!({ "bucket": name, "created": true })),
            Err(e) => api_error(500, &format!("创建 bucket 失败: {}", e)),
        },
        None => api_error(400, "非法 bucket 名称（仅字母/数字/_/-/.，≤128，不含路径分隔符）"),
    }
}

/// GET /cloud/v1/buckets/{bucket}/objects —— 列出桶内对象
async fn cloud_list_objects(
    State(s): State<CloudState>,
    Path(bucket): Path<String>,
) -> ApiResponse<Value> {
    match s.bucket_path(&bucket) {
        Some(p) if p.is_dir() => {
            let mut objects = Vec::new();
            if let Ok(entries) = fs::read_dir(&p) {
                for e in entries.flatten() {
                    let path = e.path();
                    if path.is_file() {
                        if let Some(key) = e.file_name().to_str() {
                            let meta = e.metadata().map(|m| m.len()).unwrap_or(0);
                            objects.push(json!({ "key": key, "size": meta }));
                        }
                    }
                }
            }
            objects.sort_by(|a, b| a["key"].as_str().cmp(&b["key"].as_str()));
            api_ok(json!({ "bucket": bucket, "objects": objects }))
        }
        Some(_) => api_error(404, &format!("bucket 不存在: {}", bucket)),
        None => api_error(400, "非法 bucket 名称"),
    }
}

/// PUT /cloud/v1/buckets/{bucket}/objects/{key} —— 写入对象（原始字节）
async fn cloud_put_object(
    State(s): State<CloudState>,
    Path((bucket, key)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> ApiResponse<Value> {
    match s.object_path(&bucket, &key) {
        Some(p) => {
            if let Some(parent) = p.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return api_error(500, &format!("创建对象目录失败: {}", e));
                }
            }
            match fs::write(&p, &body) {
                Ok(_) => api_ok(json!({ "bucket": bucket, "key": key, "size": body.len() })),
                Err(e) => api_error(500, &format!("写入对象失败: {}", e)),
            }
        }
        None => api_error(400, "非法 bucket/key（仅字母/数字/_/-/.，≤128，不含路径分隔符）"),
    }
}

/// GET /cloud/v1/buckets/{bucket}/objects/{key} —— 读取对象（流式下载）
async fn cloud_get_object(
    State(s): State<CloudState>,
    Path((bucket, key)): Path<(String, String)>,
) -> Response {
    match s.object_path(&bucket, &key) {
        Some(p) if p.is_file() => match fs::read(&p) {
            Ok(bytes) => Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .body(Body::from(bytes))
                .unwrap_or_else(|_| Response::new(Body::from("stream error"))),
            Err(e) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("读取对象失败: {}", e)))
                .unwrap_or_else(|_| Response::new(Body::from("stream error"))),
        },
        Some(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(format!("对象不存在: {}/{}", bucket, key)))
            .unwrap_or_else(|_| Response::new(Body::from("not found"))),
        None => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("非法 bucket/key"))
            .unwrap_or_else(|_| Response::new(Body::from("bad request"))),
    }
}

/// DELETE /cloud/v1/buckets/{bucket}/objects/{key} —— 删除对象
async fn cloud_delete_object(
    State(s): State<CloudState>,
    Path((bucket, key)): Path<(String, String)>,
) -> ApiResponse<Value> {
    match s.object_path(&bucket, &key) {
        Some(p) if p.is_file() => match fs::remove_file(&p) {
            Ok(_) => api_ok(json!({ "bucket": bucket, "key": key, "deleted": true })),
            Err(e) => api_error(500, &format!("删除对象失败: {}", e)),
        },
        Some(_) => api_error(404, &format!("对象不存在: {}/{}", bucket, key)),
        None => api_error(400, "非法 bucket/key"),
    }
}

/// 装配 Cloud 域路由（自含存储状态，nest + 外层 `with_state(())`，与 Voice 同模式）
pub fn build_cloud_router() -> Router<()> {
    Router::new()
        .nest(
            "/cloud/v1",
            Router::new()
                .route("/buckets", get(cloud_list_buckets))
                .route("/buckets", post(cloud_create_bucket))
                .route("/buckets/:bucket/objects", get(cloud_list_objects))
                .route("/buckets/:bucket/objects/:key", put(cloud_put_object))
                .route("/buckets/:bucket/objects/:key", get(cloud_get_object))
                .route("/buckets/:bucket/objects/:key", delete(cloud_delete_object))
                .with_state(CloudState::new()),
        )
        .with_state(())
}
