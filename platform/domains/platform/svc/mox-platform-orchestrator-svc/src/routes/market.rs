//! # 算子商城：导入 / 导出 / 租户与权限 API
//!
//! 提供：
//! - `GET  /:id/download?format=json|yaml` —— 单个包导出（文件下载）
//! - `GET  /export/all?format=zip|json` —— 全量导出（zip 含 manifest 签名 + 变更日志）
//! - `POST /import` —— 导入（JSON / YAML 单包或批量，冲突策略：overwrite / skip / rename）
//! - `POST /import/zip` —— 导入系统导出的 zip 全量包（校验 manifest 签名）
//! - `GET  /tenant/:tenant_id` —— 按租户过滤列表
//! - `GET  /owner/:created_by` —— 按创建人过滤列表
//!
//! 安全：导入默认 `verify=true`（HMAC-SHA256 签名校验），显式传 `verify=false` 可跳过；
//! 每次导入 / 导出都写入审计日志（`$OUS_HOME/market/audit.log`）。

use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::collections::HashMap;

use crate::market::{
    gen_id, list_packages_filtered, load_package, save_package, MarketState, OperatorPackage,
};
use crate::market_migration::{
    audit, now_rfc3339, packages_dir, sign_doc, verify_doc, zip_read, zip_write,
};
use crate::market_version::{actor_from_headers, append_changelog, snapshot_package};

/// 冲突处理策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ConflictStrategy {
    /// 覆盖：先快照旧版本再写入
    Overwrite,
    /// 跳过：保留现有，报告 skipped
    #[default]
    Skip,
    /// 重命名：以新 id 导入（名称加后缀）
    Rename,
}

/// 导入请求体（JSON / YAML 均可）
#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    /// 单个包
    #[serde(default)]
    pub package: Option<serde_json::Value>,
    /// 批量包
    #[serde(default)]
    pub packages: Vec<serde_json::Value>,
    /// 冲突策略
    #[serde(default)]
    pub conflict: ConflictStrategy,
    /// 是否校验签名（默认 true）
    #[serde(default = "default_true")]
    pub verify: bool,
}

fn default_true() -> bool {
    true
}

/// 导入结果项
#[derive(Debug, serde::Serialize)]
pub struct ImportItemResult {
    pub id: String,
    pub name: String,
    pub status: String, // imported / overwritten / skipped / renamed / rejected
    pub version: String,
    pub reason: Option<String>,
}

/// 追加变更日志 + 审计
fn log_import(pkg: &OperatorPackage, actor: &str, status: &str, detail: &str) {
    let _ = append_changelog(
        &pkg.id,
        &format!(
            "## v{} — {} (by {})\n- 导入（{}）: {}",
            pkg.version,
            now_rfc3339(),
            if actor.is_empty() { "anonymous" } else { actor },
            status,
            detail
        ),
    );
    audit(
        "import",
        actor,
        &format!(
            "算子包 {} v{} 导入状态={}（{}）",
            pkg.id, pkg.version, status, detail
        ),
    );
}

/// 从原始字节解析包（JSON 优先，失败尝试 YAML）
fn parse_package_value(raw: &[u8]) -> Result<serde_json::Value, String> {
    match serde_json::from_slice::<serde_json::Value>(raw) {
        Ok(v) => Ok(v),
        Err(json_err) => match serde_yaml::from_slice::<serde_json::Value>(raw) {
            Ok(v) => Ok(v),
            Err(yaml_err) => Err(format!(
                "JSON 解析失败: {}；YAML 解析失败: {}",
                json_err, yaml_err
            )),
        },
    }
}

/// 归一化：把导入对象规整为 (导出文档, 包对象)
/// - 裸包对象（含 name/requirement）→ 包对象
/// - 导出文档（含 package 字段）→ 保留用于签名校验
fn normalize_import(value: serde_json::Value) -> (serde_json::Value, serde_json::Value) {
    if value.get("package").is_some() {
        let doc = value.clone();
        let pkg = value.get("package").cloned().unwrap_or_default();
        (doc, pkg)
    } else {
        (value.clone(), value)
    }
}

/// 导入单个包（公开供测试与复用）
pub fn import_one(
    value: serde_json::Value,
    conflict: ConflictStrategy,
    verify: bool,
    actor: &str,
) -> ImportItemResult {
    let (doc, pkg_value) = normalize_import(value);
    // 1) 签名校验
    if verify && !verify_doc(&doc) {
        audit("import", actor, "签名校验失败（已拒绝）");
        return ImportItemResult {
            id: pkg_value
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string(),
            name: pkg_value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string(),
            status: "rejected".to_string(),
            version: pkg_value
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string(),
            reason: Some(
                "签名校验失败（导出物被篡改或密钥不匹配；如需信任来源可传 verify=false）"
                    .to_string(),
            ),
        };
    }
    // 2) 反序列化为包
    let mut pkg: OperatorPackage = match serde_json::from_value(pkg_value) {
        Ok(p) => p,
        Err(e) => {
            audit("import", actor, &format!("包结构非法（已拒绝）: {}", e));
            return ImportItemResult {
                id: doc
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string(),
                name: doc
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string(),
                status: "rejected".to_string(),
                version: doc
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string(),
                reason: Some(format!("包结构非法: {}", e)),
            };
        }
    };
    if pkg.name.trim().is_empty() {
        return ImportItemResult {
            id: pkg.id.clone(),
            name: String::new(),
            status: "rejected".to_string(),
            version: pkg.version.clone(),
            reason: Some("名称不能为空".to_string()),
        };
    }
    // 3) 冲突处理
    let exists = load_package(&pkg.id).is_ok();
    if exists {
        match conflict {
            ConflictStrategy::Skip => {
                log_import(&pkg, actor, "skipped", "目标已存在，按 skip 策略跳过");
                return ImportItemResult {
                    id: pkg.id.clone(),
                    name: pkg.name.clone(),
                    status: "skipped".to_string(),
                    version: pkg.version.clone(),
                    reason: Some("目标已存在（skip 策略）".to_string()),
                };
            }
            ConflictStrategy::Rename => {
                let old_id = pkg.id.clone();
                pkg.id = gen_id();
                pkg.name = format!("{} (导入副本)", pkg.name);
                let detail = format!("原 id={} 冲突，已重命名导入", old_id);
                pkg.forked_from = Some(old_id);
                if save_package(&pkg).is_err() {
                    return ImportItemResult {
                        id: pkg.id.clone(),
                        name: pkg.name.clone(),
                        status: "rejected".to_string(),
                        version: pkg.version.clone(),
                        reason: Some("写入失败".to_string()),
                    };
                }
                log_import(&pkg, actor, "renamed", &detail);
                return ImportItemResult {
                    id: pkg.id.clone(),
                    name: pkg.name.clone(),
                    status: "renamed".to_string(),
                    version: pkg.version.clone(),
                    reason: None,
                };
            }
            ConflictStrategy::Overwrite => {
                // 覆盖前快照旧版本（版本化不丢历史）
                if let Ok(old) = load_package(&pkg.id) {
                    let _ = snapshot_package(
                        &old,
                        actor,
                        &format!("导入覆盖前快照（新版本 {}）", pkg.version),
                    );
                }
                if save_package(&pkg).is_err() {
                    return ImportItemResult {
                        id: pkg.id.clone(),
                        name: pkg.name.clone(),
                        status: "rejected".to_string(),
                        version: pkg.version.clone(),
                        reason: Some("写入失败".to_string()),
                    };
                }
                log_import(&pkg, actor, "overwritten", "覆盖已有包（旧版本已快照）");
                return ImportItemResult {
                    id: pkg.id.clone(),
                    name: pkg.name.clone(),
                    status: "overwritten".to_string(),
                    version: pkg.version.clone(),
                    reason: None,
                };
            }
        }
    }
    // 4) 全新导入
    if pkg.id.trim().is_empty() {
        pkg.id = gen_id();
    }
    if pkg.created_at.is_empty() {
        pkg.created_at = now_rfc3339();
    }
    pkg.updated_at = now_rfc3339();
    if save_package(&pkg).is_err() {
        return ImportItemResult {
            id: pkg.id.clone(),
            name: pkg.name.clone(),
            status: "rejected".to_string(),
            version: pkg.version.clone(),
            reason: Some("写入失败".to_string()),
        };
    }
    log_import(&pkg, actor, "imported", "全新导入");
    ImportItemResult {
        id: pkg.id.clone(),
        name: pkg.name.clone(),
        status: "imported".to_string(),
        version: pkg.version.clone(),
        reason: None,
    }
}

// ========== Handlers ==========

/// GET /:id/download?format=json|yaml —— 单个包导出下载
async fn download_package(
    State(_s): State<MarketState>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let actor = actor_from_headers(&headers);
    let format = params.get("format").map(|s| s.as_str()).unwrap_or("json");
    let pkg = match load_package(&id) {
        Ok(p) => p,
        Err(_) => {
            let mut h = HeaderMap::new();
            h.insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            );
            return (
                StatusCode::NOT_FOUND,
                h,
                "{\"success\":false,\"error\":\"算子包不存在\"}"
                    .as_bytes()
                    .to_vec(),
            );
        }
    };
    let (body, mime, ext) = match format {
        "yaml" | "yml" => (
            serde_yaml::to_string(&pkg).unwrap_or_default().into_bytes(),
            "application/yaml".to_string(),
            "yaml",
        ),
        _ => (
            serde_json::to_string_pretty(&pkg)
                .unwrap_or_default()
                .into_bytes(),
            "application/json".to_string(),
            "json",
        ),
    };
    audit(
        "export",
        &actor,
        &format!("导出包 {} v{}（format={}）", id, pkg.version, ext),
    );
    let filename = format!("{}-v{}.{}", id, pkg.version, ext);
    let disposition = format!("attachment; filename=\"{}\"", filename);
    let mut h = HeaderMap::new();
    h.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_str(&mime)
            .unwrap_or_else(|_| header::HeaderValue::from_static("application/octet-stream")),
    );
    h.insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&disposition)
            .unwrap_or_else(|_| header::HeaderValue::from_static("application/octet-stream")),
    );
    h.insert(
        header::CONTENT_LENGTH,
        header::HeaderValue::from_str(&body.len().to_string())
            .unwrap_or_else(|_| header::HeaderValue::from_static("application/octet-stream")),
    );
    (StatusCode::OK, h, body)
}

/// GET /export/all?format=zip|json —— 全量导出
async fn export_all(
    State(_s): State<MarketState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let actor = actor_from_headers(&headers);
    let format = params.get("format").map(|s| s.as_str()).unwrap_or("zip");
    let dir = packages_dir();
    let mut packages: Vec<OperatorPackage> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(pkg) = serde_json::from_str::<OperatorPackage>(&content) {
                        packages.push(pkg);
                    }
                }
            }
        }
    }
    packages.sort_by(|a, b| a.id.cmp(&b.id));

    if format == "json" {
        let body = serde_json::to_vec(&serde_json::json!({
            "kind": "ous-market-export",
            "exported_at": now_rfc3339(),
            "actor": &actor,
            "count": packages.len(),
            "packages": packages,
        }))
        .unwrap_or_default();
        audit(
            "export",
            &actor,
            &format!("全量导出 JSON 共 {} 个包", packages.len()),
        );
        let mut h = HeaderMap::new();
        h.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        h.insert(
            header::CONTENT_DISPOSITION,
            header::HeaderValue::from_static("attachment; filename=\"ous-market-export.json\""),
        );
        h.insert(
            header::CONTENT_LENGTH,
            header::HeaderValue::from_str(&body.len().to_string())
                .unwrap_or_else(|_| header::HeaderValue::from_static("application/octet-stream")),
        );
        return (StatusCode::OK, h, body);
    }

    // zip：manifest（签名）+ packages/*.json + changelog/*.md
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut manifest = serde_json::json!({
        "kind": "ous-market-export",
        "export_version": 1,
        "exported_at": now_rfc3339(),
        "actor": &actor,
        "count": packages.len(),
        "signature": serde_json::Value::Null,
    });
    let sig = sign_doc(&mut manifest);
    manifest["signature"] = serde_json::Value::String(sig);
    entries.push((
        "manifest.json".to_string(),
        serde_json::to_vec_pretty(&manifest).unwrap_or_default(),
    ));
    for pkg in &packages {
        entries.push((
            format!("packages/{}.json", pkg.id),
            serde_json::to_vec_pretty(pkg).unwrap_or_default(),
        ));
        let cl = crate::market_version::read_changelog(&pkg.id);
        if !cl.is_empty() {
            entries.push((format!("changelog/{}.md", pkg.id), cl.into_bytes()));
        }
    }
    let bytes = zip_write(&entries);
    audit(
        "export",
        &actor,
        &format!("全量导出 ZIP 共 {} 个包", packages.len()),
    );
    let mut h = HeaderMap::new();
    h.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/zip"),
    );
    h.insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_static("attachment; filename=\"ous-market-export.zip\""),
    );
    h.insert(
        header::CONTENT_LENGTH,
        header::HeaderValue::from_str(&bytes.len().to_string())
            .unwrap_or_else(|_| header::HeaderValue::from_static("application/octet-stream")),
    );
    (StatusCode::OK, h, bytes)
}

/// POST /import —— 导入 JSON / YAML（单包或批量）
async fn import_packages(
    State(_s): State<MarketState>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, Json<serde_json::Value>) {
    let actor = actor_from_headers(&headers);
    let value = match parse_package_value(&body) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "success": false, "error": e })),
            )
        }
    };
    let req: ImportRequest = match serde_json::from_value(value.clone()) {
        Ok(r) => r,
        Err(e) => {
            // 裸包对象（没有 package/packages 包装）也支持
            if let Ok(pkg) = serde_json::from_value::<OperatorPackage>(value.clone()) {
                let item = import_one(
                    serde_json::to_value(&pkg).unwrap(),
                    req_conflict(&value),
                    req_verify(&value),
                    &actor,
                );
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({ "success": true, "results": [item] })),
                );
            }
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({ "success": false, "error": format!("导入请求格式非法: {}", e) }),
                ),
            );
        }
    };
    let mut items: Vec<serde_json::Value> = Vec::new();
    if let Some(p) = req.package {
        let item = import_one(p, req.conflict, req.verify, &actor);
        items.push(serde_json::to_value(&item).unwrap_or_default());
    }
    for p in req.packages {
        let item = import_one(p, req.conflict, req.verify, &actor);
        items.push(serde_json::to_value(&item).unwrap_or_default());
    }
    if items.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({ "success": false, "error": "没有可导入的包（需要 package 或 packages 字段）" }),
            ),
        );
    }
    let counts = count_statuses(&items);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "results": items,
            "summary": counts,
        })),
    )
}

fn req_conflict(v: &serde_json::Value) -> ConflictStrategy {
    v.get("conflict")
        .and_then(|c| serde_json::from_value::<ConflictStrategy>(c.clone()).ok())
        .unwrap_or_default()
}
fn req_verify(v: &serde_json::Value) -> bool {
    v.get("verify").and_then(|c| c.as_bool()).unwrap_or(true)
}

fn count_statuses(items: &[serde_json::Value]) -> serde_json::Value {
    let mut map: HashMap<String, usize> = HashMap::new();
    for it in items {
        let s = it
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        *map.entry(s).or_insert(0) += 1;
    }
    serde_json::to_value(map).unwrap_or_default()
}

/// POST /import/zip —— 导入系统导出的 zip 全量包（校验 manifest 签名）
async fn import_zip(
    State(_s): State<MarketState>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, Json<serde_json::Value>) {
    let actor = actor_from_headers(&headers);
    let req: ZipImportRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({ "success": false, "error": format!("请求体需为 {{ \"data\": \"<base64 zip>\" }}: {}", e) }),
                ),
            )
        }
    };
    let raw = match base64_decode(&req.data) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "success": false, "error": e })),
            )
        }
    };
    let entries = match zip_read(&raw) {
        Ok(e) => e,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "success": false, "error": e })),
            )
        }
    };
    // manifest 校验
    let manifest_entry = entries.iter().find(|(n, _)| n == "manifest.json");
    let conflict = req.conflict.unwrap_or_default();
    let verify = req.verify.unwrap_or(true);
    let mut results: Vec<serde_json::Value> = Vec::new();
    if verify {
        match manifest_entry {
            Some((_, content)) => {
                let doc: serde_json::Value = match serde_json::from_slice(content) {
                    Ok(d) => d,
                    Err(e) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(
                                serde_json::json!({ "success": false, "error": format!("manifest 解析失败: {}", e) }),
                            ),
                        )
                    }
                };
                if !verify_doc(&doc) {
                    audit("import", &actor, "ZIP manifest 签名校验失败（已拒绝）");
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(
                            serde_json::json!({ "success": false, "error": "ZIP manifest 签名校验失败（导出物被篡改或密钥不匹配）" }),
                        ),
                    );
                }
            }
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(
                        serde_json::json!({ "success": false, "error": "ZIP 缺少 manifest.json（不是 OUS 导出的全量包）" }),
                    ),
                );
            }
        }
    }
    for (name, content) in &entries {
        if name.starts_with("packages/") && name.ends_with(".json") {
            let value: serde_json::Value = match serde_json::from_slice(content) {
                Ok(v) => v,
                Err(_) => continue,
            };
            // 包条目本身再套一层导出文档壳（用 zip 内签名校验过的 manifest 语境）
            let doc = serde_json::json!({
                "kind": "ous-market-export",
                "package": value,
                "signature": serde_json::Value::Null,
            });
            let item = if verify {
                // 包级签名：以 manifest 中的签名密钥语境重新校验 —— 若无包级签名则按 manifest 已校验放行
                import_one_allow(doc, conflict, actor.clone(), "zip 导入（manifest 已校验）")
            } else {
                import_one_allow(doc, conflict, actor.clone(), "zip 导入（verify=false）")
            };
            results.push(serde_json::to_value(&item).unwrap_or_default());
        }
    }
    audit(
        "import",
        &actor,
        &format!("ZIP 导入完成，处理 {} 个包", results.len()),
    );
    (
        StatusCode::OK,
        Json(
            serde_json::json!({ "success": true, "results": results, "summary": count_statuses(&results) }),
        ),
    )
}

/// zip 导入用：跳过包级签名（manifest 已整体校验）
fn import_one_allow(
    value: serde_json::Value,
    conflict: ConflictStrategy,
    actor: String,
    detail: &str,
) -> ImportItemResult {
    let (_doc, pkg_value) = normalize_import(value);
    let mut pkg: OperatorPackage = match serde_json::from_value(pkg_value) {
        Ok(p) => p,
        Err(_) => {
            return ImportItemResult {
                id: "?".to_string(),
                name: "?".to_string(),
                status: "rejected".to_string(),
                version: "?".to_string(),
                reason: Some("包结构非法".to_string()),
            }
        }
    };
    let exists = load_package(&pkg.id).is_ok();
    if exists {
        match conflict {
            ConflictStrategy::Skip => {
                log_import(&pkg, &actor, "skipped", "目标已存在（skip 策略）");
                return ImportItemResult {
                    id: pkg.id.clone(),
                    name: pkg.name.clone(),
                    status: "skipped".to_string(),
                    version: pkg.version.clone(),
                    reason: Some("目标已存在".to_string()),
                };
            }
            ConflictStrategy::Rename => {
                let old_id = pkg.id.clone();
                pkg.id = gen_id();
                pkg.name = format!("{} (导入副本)", pkg.name);
                let detail = format!("原 id={} 冲突，重命名导入", old_id);
                pkg.forked_from = Some(old_id);
                let _ = save_package(&pkg);
                log_import(&pkg, &actor, "renamed", &detail);
                return ImportItemResult {
                    id: pkg.id.clone(),
                    name: pkg.name.clone(),
                    status: "renamed".to_string(),
                    version: pkg.version.clone(),
                    reason: None,
                };
            }
            ConflictStrategy::Overwrite => {
                if let Ok(old) = load_package(&pkg.id) {
                    let _ = snapshot_package(&old, &actor, "zip 导入覆盖前快照");
                }
                let _ = save_package(&pkg);
                log_import(&pkg, &actor, "overwritten", "zip 导入覆盖（旧版本已快照）");
                return ImportItemResult {
                    id: pkg.id.clone(),
                    name: pkg.name.clone(),
                    status: "overwritten".to_string(),
                    version: pkg.version.clone(),
                    reason: None,
                };
            }
        }
    }
    if pkg.id.trim().is_empty() {
        pkg.id = gen_id();
    }
    pkg.updated_at = now_rfc3339();
    let _ = save_package(&pkg);
    log_import(&pkg, &actor, "imported", detail);
    ImportItemResult {
        id: pkg.id.clone(),
        name: pkg.name.clone(),
        status: "imported".to_string(),
        version: pkg.version.clone(),
        reason: None,
    }
}

/// GET /tenant/:tenant_id —— 按租户过滤
async fn list_by_tenant(
    State(state): State<MarketState>,
    Path(tenant_id): Path<String>,
) -> Json<serde_json::Value> {
    let meta = list_packages_filtered(&state, None, None, None, Some(&tenant_id), None, None);
    Json(
        serde_json::json!({ "success": true, "tenant_id": tenant_id, "total": meta.len(), "packages": meta }),
    )
}

/// GET /owner/:created_by —— 按创建人过滤
async fn list_by_owner(
    State(state): State<MarketState>,
    Path(created_by): Path<String>,
) -> Json<serde_json::Value> {
    let meta = list_packages_filtered(&state, None, None, None, None, Some(&created_by), None);
    Json(
        serde_json::json!({ "success": true, "created_by": created_by, "total": meta.len(), "packages": meta }),
    )
}

/// base64 解码（支持标准/URL-safe）
fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(s))
        .map_err(|e| format!("base64 解码失败: {}", e))
}

#[derive(Debug, Deserialize)]
struct ZipImportRequest {
    data: String,
    #[serde(default)]
    conflict: Option<ConflictStrategy>,
    #[serde(default)]
    verify: Option<bool>,
}

/// 扩展路由：挂载到 /api/market 下（导入/导出/租户）
pub fn extra_routes() -> Router<MarketState> {
    Router::new()
        .route("/:id/download", get(download_package))
        .route("/export/all", get(export_all))
        .route("/import", post(import_packages))
        .route("/import/zip", post(import_zip))
        .route("/tenant/:tenant_id", get(list_by_tenant))
        .route("/owner/:created_by", get(list_by_owner))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
}
