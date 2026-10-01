// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 存储管理面路由（AdminStorage 面板）：`/api/storage/*` + `/api/modules`
//!
//! # 能力来源（禁桩）
//! 全部数据经 [`crate::storage_backend::StorageBackend`] 抽象投影真实后端：
//! - **local**：本地磁盘对象存储（`CloudState`），恒在、真实读写 `data/storage`；
//! - **s3**：S3/MinIO 兼容后端，仅当 `MOX_S3_*` env 配置齐全时由 `S3Backend::from_env()` 挂载；
//!   未配置绝不列入"可用"清单。
//!
//! - `GET  /api/storage/status`   —— 实时投影：bucket/对象总数/分布/字节（local 为系统记录盘）
//! - `GET  /api/storage/providers`—— 动态列出 local +（若配置）s3
//! - `POST /api/storage/switch`  —— local 恒可切；s3 写路径尚未切换，明确返回冲突
//! - `GET  /api/modules`         —— 投影 `routes::DOMAINS` 自描述域注册表

use std::sync::Arc;
use axum::{extract::State, routing::{get, post}, Json, Router};
use mox_api_protocol::{api_error, api_ok, ApiResponse};
use serde_json::{json, Value};

use crate::cloud::CloudState;
use crate::routes::DOMAINS;
use crate::storage_backend::{S3Backend, StorageBackend};

/// 存储后端注册表：持有 local（系统记录盘）+ 可选 s3（env 门控）
#[derive(Clone)]
pub struct StorageRegistry {
    pub local: CloudState,
    pub s3: Option<Arc<S3Backend>>,
}

impl StorageRegistry {
    pub fn new(local: CloudState) -> Self {
        let s3 = S3Backend::from_env().map(Arc::new);
        Self { local, s3 }
    }
}

/// GET /api/storage/status —— 当前存储状态（local 实时读盘；附 s3 可达性）
async fn storage_status(State(reg): State<Arc<StorageRegistry>>) -> ApiResponse<Value> {
    let buckets = match reg.local.list_buckets().await { Ok(buckets) => buckets, Err(_) => return api_error(503,"本地对象存储不可用") };
    let total_objects: u64 = buckets.iter().map(|b| b.object_count).sum();
    let total_bytes: u64 = buckets.iter().map(|b| b.used_bytes).sum();
    let entities_by_type: Vec<Value> = buckets.iter()
        .map(|b| json!({ "entity_type": b.name, "cnt": b.object_count }))
        .collect();

    // s3 可达性（若配置）；探测失败不影响 local 主状态，如实标注
    let s3_feature = reg.s3.is_some();
    let s3_note = match &reg.s3 {
        Some(s3b) => match s3b.health().await {
            Ok(_) => "configured_reachable".to_string(),
            Err(e) => format!("configured_unreachable: {}", truncate_str(&e, 120)),
        },
        None => "not_configured".to_string(),
    };

    api_ok(json!({
        "provider": "local",
        "name": "本地磁盘对象存储",
        "root": reg.local.root_dir().to_string_lossy(),
        "totalEntities": total_objects,
        "total_objects": total_objects,
        "used_bytes": total_bytes,
        "bucket_count": buckets.len(),
        "entitiesByType": entities_by_type,
        "features": {
            "object_storage": true,
            "multi_bucket": true,
            "s3_compatible_semantics": true,
            "s3_native_api": false,
            "s3_backend_configured": s3_feature,
            "aliyun_oss": false,
            "tencent_cos": false,
            "minio": s3_feature,
        },
        "s3": s3_note,
    }))
}

/// GET /api/storage/providers —— 动态列出真实可用后端（local 恒在 + s3 若配置）
async fn storage_providers(State(reg): State<Arc<StorageRegistry>>) -> ApiResponse<Value> {
    let mut out = vec![reg.local.describe()];
    if let Some(s3) = &reg.s3 {
        out.push(s3.describe());
    }
    api_ok(json!(out))
}

/// POST /api/storage/switch —— 切换提供方
async fn storage_switch(
    State(reg): State<Arc<StorageRegistry>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let target = body.get("provider").and_then(|v| v.as_str()).unwrap_or("");
    match target {
        "local" => api_ok(json!({
            "provider": "local", "switched": true,
            "root": reg.local.root_dir().to_string_lossy(),
            "note": "本地磁盘为系统记录盘，切换为幂等无操作。"
        })),
        "s3" => match &reg.s3 {
            Some(s3b) => match s3b.health().await {
                Ok(_) => api_error(409,"S3 后端可达，但当前文件/云盘写路径尚未接入切换；provider 仍为 local"),
                Err(e) => api_error(502, format!("S3 已配置但探测不可达: {}", truncate_str(&e, 160))),
            },
            None => api_error(409,
                "S3 后端未配置（需 MOX_S3_ENDPOINT/MOX_S3_BUCKET/MOX_S3_ACCESS_KEY_ID/MOX_S3_SECRET_ACCESS_KEY）"),
        },
        "" => api_error(400, "缺少 provider 字段"),
        other => api_error(409,
            format!("存储提供方「{}」无后端实现（可用：local{}）", other, if reg.s3.is_some() { "、s3" } else { "" })),
    }
}

/// GET /api/modules —— 已加载域/模块清单（投影自 routes::DOMAINS）
async fn list_modules() -> ApiResponse<Value> {
    let version = env!("CARGO_PKG_VERSION");
    let modules: Vec<Value> = DOMAINS.iter()
        .map(|d| json!({
            "name": d.name, "description": d.description, "version": version,
            "routes": d.prefix, "layer": d.layer, "group": d.group, "status": d.status,
        }))
        .collect();
    api_ok(json!(modules))
}

fn truncate_str(s: &str, n: usize) -> &str {
    let mut end = n.min(s.len());
    while !s.is_char_boundary(end) { end -= 1; }
    &s[..end]
}

/// 装配存储管理面路由
pub fn build_storage_admin_router(registry: Arc<StorageRegistry>) -> Router<()> {
    Router::new()
        .route("/api/storage/status", get(storage_status))
        .route("/api/storage/providers", get(storage_providers))
        .route("/api/storage/switch", post(storage_switch))
        .route("/api/modules", get(list_modules))
        .with_state(registry)
}

#[cfg(test)]
mod tests {
    #[test]
    fn unicode_errors_truncate_without_panicking() {
        assert_eq!(super::truncate_str("中文错误",4),"中");
        assert_eq!(super::truncate_str("中文",100),"中文");
    }
}
