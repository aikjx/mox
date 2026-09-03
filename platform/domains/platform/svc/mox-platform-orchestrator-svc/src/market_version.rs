// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 算子商城：版本化管理
//!
//! - **semver**：版本号遵循 `主.次.补[-预发布]` 语义化版本（内置实现，零依赖），
//!   支持解析、比较（完整优先级规则）、主/次/补递增。
//! - **快照**：每次实质性更新前，旧版本自动快照到 `$OUS_HOME/market/versions/<id>/`，
//!   文件名 `v<version>@<ts>.json`（同一版本可保留多次快照）。
//! - **变更日志**：`$OUS_HOME/market/changelog/<id>.md`，每次更新自动追加。
//! - **历史保留**：默认保留 N=5 个快照（`OUS_MARKET_KEEP_VERSIONS` 可配，0=不限）。
//! - **回滚**：`POST /:id/rollback/:version`，回滚前也会对当前版本做快照，回滚本身可追溯。
//! - **差异对比**：`GET /:id/versions/compare?base=&target=` 输出两版本结构化 diff。
//!
//! 关键约束：**版本化不阻塞读取** —— 快照/变更日志写入均为 best-effort，
//! 失败只告警不回滚主流程；读取始终以 `packages/<id>.json`（最新版）优先。

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

use crate::market::{load_package, reload_index_sync, save_package, MarketState, OperatorPackage};
use crate::market_migration::{audit, changelogs_dir, now_rfc3339, versions_dir};
use mox_api_protocol::{ApiResponse, api_ok, api_error, api_ok_empty};

// ========== 语义化版本 ==========

/// 语义化版本（主.次.补[-预发布][+构建元数据]，构建元数据不参与比较）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// 预发布标识（如 "alpha.1"），None 表示正式版
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre: Option<String>,
}

impl SemVer {
    /// 解析语义化版本；失败返回 None。
    pub fn parse(s: &str) -> Option<SemVer> {
        let s = s.trim();
        // 去掉构建元数据（+build）
        let core = s.split('+').next().unwrap_or(s);
        let (nums, pre) = match core.split_once('-') {
            Some((n, p)) => (n, Some(p.to_string())),
            None => (core, None),
        };
        let mut parts = nums.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next().unwrap_or("0").parse().ok()?;
        let patch = parts.next().unwrap_or("0").parse().ok()?;
        // 不允许多余段
        if parts.next().is_some() {
            return None;
        }
        Some(SemVer {
            major,
            minor,
            patch,
            pre,
        })
    }

    #[allow(dead_code)]
    pub fn bump_major(&self) -> SemVer {
        SemVer {
            major: self.major + 1,
            minor: 0,
            patch: 0,
            pre: None,
        }
    }
    #[allow(dead_code)]
    pub fn bump_minor(&self) -> SemVer {
        SemVer {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
            pre: None,
        }
    }
    pub fn bump_patch(&self) -> SemVer {
        SemVer {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
            pre: None,
        }
    }

    /// semver 2.0 优先级比较（预发布 < 正式版）
    pub fn precedence_cmp(&self, other: &SemVer) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            o => return o,
        }
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            o => return o,
        }
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            o => return o,
        }
        match (&self.pre, &other.pre) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater, // 正式版 > 预发布
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => compare_pre(a, b),
        }
    }
}

/// `SemVer` 展示：实现 `Display`，自动获得 `ToString`
impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.pre {
            Some(p) => write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, p),
            None => write!(f, "{}.{}.{}", self.major, self.minor, self.patch),
        }
    }
}

/// 预发布标识比较：`.` 分段；数字段按数值、数字 < 字母数字、其余按字典序；
/// 前缀相同时段数少者优先。
fn compare_pre(a: &str, b: &str) -> Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();
    for (x, y) in a_parts.iter().zip(b_parts.iter()) {
        let xn = x.parse::<u64>();
        let yn = y.parse::<u64>();
        let ord = match (xn, yn) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            (Ok(_), Err(_)) => Ordering::Less, // 数字 < 字母数字
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.cmp(y),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    a_parts.len().cmp(&b_parts.len())
}

/// 字符串版本的比较：优先按 semver，解析失败退化为字典序
pub fn version_cmp(a: &str, b: &str) -> Ordering {
    match (SemVer::parse(a), SemVer::parse(b)) {
        (Some(x), Some(y)) => x.precedence_cmp(&y),
        _ => a.cmp(b),
    }
}

/// 版本号是否合法 semver
pub fn is_valid_version(v: &str) -> bool {
    SemVer::parse(v).is_some()
}

/// 自动 bump 补丁号（解析失败时按旧逻辑直接 +1 补丁）
pub fn bump_patch_version(v: &str) -> String {
    match SemVer::parse(v) {
        Some(s) => s.bump_patch().to_string(),
        None => {
            let parts: Vec<&str> = v.split('.').collect();
            let mut nums: Vec<u32> = parts
                .iter()
                .map(|p| p.parse::<u32>().unwrap_or(0))
                .collect();
            while nums.len() < 3 {
                nums.push(0);
            }
            nums[2] += 1;
            format!("{}.{}.{}", nums[0], nums[1], nums[2])
        }
    }
}

// ========== 快照 / 变更日志 / 回滚 ==========

/// 版本快照条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    pub version: String,
    pub created_at: String,
    /// 变更人
    pub by: String,
    /// 变更说明
    pub note: String,
    /// 快照文件名
    pub file: String,
}

/// 版本差异报告
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionDiff {
    pub from: String,
    pub to: String,
    pub changed: bool,
    /// 发生变化的顶层字段名
    pub fields_changed: Vec<String>,
    pub nodes_added: Vec<String>,
    pub nodes_removed: Vec<String>,
    pub edges_added: Vec<String>,
    pub edges_removed: Vec<String>,
    pub features_added: Vec<String>,
    pub features_removed: Vec<String>,
}

fn sanitize_version_file(v: &str) -> String {
    v.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// 变更日志文件：`$OUS_HOME/market/changelog/<id>.md`
pub fn changelog_path(id: &str) -> std::path::PathBuf {
    changelogs_dir().join(format!(
        "{}.md",
        crate::market_migration::sanitize_file_component(id)
    ))
}

/// 把当前包快照进版本库（best-effort：失败只告警，不阻塞主流程）。
/// - 写 `versions/<id>/v<version>@<ts>.json`
/// - 追加 `changelog/<id>.md`
/// - 按 `OUS_MARKET_KEEP_VERSIONS` 裁剪历史（0 = 不限制）
pub fn snapshot_package(pkg: &OperatorPackage, by: &str, note: &str) -> std::io::Result<()> {
    let dir = versions_dir(&pkg.id);
    std::fs::create_dir_all(&dir)?;
    let ts = chrono::Utc::now().timestamp();
    let file = format!("v{}@{}.json", sanitize_version_file(&pkg.version), ts);
    let content = serde_json::to_string_pretty(pkg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(dir.join(&file), content)?;

    append_changelog(
        &pkg.id,
        &format!(
            "## v{} — {} (by {})\n- {}",
            pkg.version,
            now_rfc3339(),
            if by.is_empty() { "anonymous" } else { by },
            if note.is_empty() {
                "更新算子包"
            } else {
                note
            }
        ),
    )?;

    prune_versions(&pkg.id);
    Ok(())
}

/// 追加变更日志（新条目在最上方）
pub fn append_changelog(id: &str, entry: &str) -> std::io::Result<()> {
    let path = changelog_path(id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut existing = String::new();
    if path.exists() {
        existing = std::fs::read_to_string(&path).unwrap_or_default();
    }
    let new = if existing.trim().is_empty() {
        format!("# 算子包 {} 变更日志\n\n{}\n", id, entry)
    } else {
        format!("{}\n\n{}", entry, existing)
    };
    std::fs::write(path, new)
}

/// 读取包变更日志全文
pub fn read_changelog(id: &str) -> String {
    let path = changelog_path(id);
    if path.exists() {
        std::fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    }
}

/// 裁剪快照历史：保留最新 N 个（按版本号优先、同版本按时间）
pub fn prune_versions(id: &str) {
    let limit = crate::market_migration::keep_versions_limit();
    if limit == 0 {
        return;
    }
    let dir = versions_dir(id);
    let mut snapshots: Vec<(String, std::path::PathBuf)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    snapshots.push((name.to_string(), path));
                }
            }
        }
    }
    if snapshots.len() <= limit {
        return;
    }
    // 按文件名解析 (version@ts)
    snapshots.sort_by(|a, b| {
        let (va, ta) = split_snapshot_name(&a.0);
        let (vb, tb) = split_snapshot_name(&b.0);
        version_cmp(&va, &vb).then(ta.cmp(&tb)).then(b.0.cmp(&a.0))
    });
    // 保留最后 limit 个（排序为升序 → 保留尾部）
    let keep = snapshots.len() - limit;
    for (_, path) in snapshots.into_iter().take(keep) {
        let _ = std::fs::remove_file(path);
    }
}

fn split_snapshot_name(name: &str) -> (String, i64) {
    let base = name.trim_end_matches(".json");
    match base.split_once('@') {
        Some((v, ts)) => (
            v.trim_start_matches('v').to_string(),
            ts.parse().unwrap_or(0),
        ),
        None => (base.to_string(), 0),
    }
}

/// 列出全部版本快照（按版本号降序、同版本按时间降序）
pub fn list_versions(id: &str) -> Vec<VersionEntry> {
    let dir = versions_dir(id);
    let mut entries: Vec<VersionEntry> = Vec::new();
    if let Ok(read) = std::fs::read_dir(&dir) {
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let file = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let (version, _ts) = split_snapshot_name(&file);
            // 从快照内容中取准确信息
            let (by, note, created_at) = match std::fs::read_to_string(&path)
                .ok()
                .and_then(|c| serde_json::from_str::<OperatorPackage>(&c).ok())
            {
                Some(p) => (String::new(), String::new(), p.updated_at),
                None => (String::new(), String::new(), String::new()),
            };
            entries.push(VersionEntry {
                version: if version.is_empty() {
                    file.clone()
                } else {
                    version
                },
                created_at,
                by,
                note,
                file,
            });
        }
    }
    entries.sort_by(|a, b| {
        version_cmp(&a.version, &b.version)
            .reverse()
            .then(b.created_at.cmp(&a.created_at))
    });
    entries
}

/// 读取指定版本的快照；同版本多快照取最新一条。未找到返回 None。
pub fn get_version(id: &str, version: &str) -> Option<OperatorPackage> {
    let dir = versions_dir(id);
    if !dir.is_dir() {
        return None;
    }
    let mut best: Option<(String, OperatorPackage)> = None;
    if let Ok(read) = std::fs::read_dir(&dir) {
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let (v, _ts) = split_snapshot_name(name);
            if v == version {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(pkg) = serde_json::from_str::<OperatorPackage>(&content) {
                        let is_newer = match &best {
                            None => true,
                            Some((best_name, _)) => {
                                let (_, bt) = split_snapshot_name(best_name);
                                let (_, t) = split_snapshot_name(name);
                                t > bt
                            }
                        };
                        if is_newer {
                            best = Some((name.to_string(), pkg));
                        }
                    }
                }
            }
        }
    }
    best.map(|(_, pkg)| pkg)
}

/// 回滚到指定版本：
/// 1. 先把当前版本快照（回滚可逆）
/// 2. 用目标快照覆盖 `packages/<id>.json`（版本号保持为目标快照版本）
/// 3. 追加变更日志 + 审计
pub fn rollback(id: &str, version: &str, actor: &str) -> Result<OperatorPackage, String> {
    let target = get_version(id, version).ok_or_else(|| format!("版本 {} 不存在", version))?;
    let current = load_package(id).ok();
    if let Some(cur) = &current {
        // 回滚前快照当前状态
        let _ = snapshot_package(cur, actor, &format!("回滚前快照（目标 v{}）", version));
    }
    save_package(&target).map_err(|e| format!("回滚写入失败: {}", e))?;
    let note = format!("回滚到 v{}", version);
    let _ = append_changelog(
        id,
        &format!(
            "## v{} — {} (by {})\n- {}",
            target.version,
            now_rfc3339(),
            if actor.is_empty() { "anonymous" } else { actor },
            note
        ),
    );
    audit(
        "rollback",
        actor,
        &format!("算子包 {} 回滚到 v{}", id, version),
    );
    Ok(target)
}

/// 两版本差异对比：base 与 target 的字段 / 节点 / 连线 / 功能点变化
pub fn diff_packages(base: &OperatorPackage, target: &OperatorPackage) -> VersionDiff {
    let mut diff = VersionDiff {
        from: base.version.clone(),
        to: target.version.clone(),
        ..Default::default()
    };
    for (f, b, t) in [
        ("name", &base.name, &target.name),
        ("category", &base.category, &target.category),
        ("author", &base.author, &target.author),
        ("summary", &base.summary, &target.summary),
        ("requirement", &base.requirement, &target.requirement),
        ("tenant", &base.tenant, &target.tenant),
        ("tenant_id", &base.tenant_id, &target.tenant_id),
        (
            "tags",
            &serde_json::to_string(&base.tags).unwrap_or_default(),
            &serde_json::to_string(&target.tags).unwrap_or_default(),
        ),
    ] {
        if b != t {
            diff.fields_changed.push(f.to_string());
        }
    }
    let ids = |v: &Vec<String>| -> Vec<String> { v.clone() };
    let b_nodes: Vec<String> = base.nodes.iter().map(|n| n.id.clone()).collect();
    let t_nodes: Vec<String> = target.nodes.iter().map(|n| n.id.clone()).collect();
    let b_edges: Vec<String> = base.edges.iter().map(|e| e.id.clone()).collect();
    let t_edges: Vec<String> = target.edges.iter().map(|e| e.id.clone()).collect();
    let b_feats: Vec<String> = base.features.iter().map(|f| f.id.clone()).collect();
    let t_feats: Vec<String> = target.features.iter().map(|f| f.id.clone()).collect();
    diff.nodes_added = ids(&t_nodes
        .iter()
        .filter(|x| !b_nodes.contains(x))
        .cloned()
        .collect());
    diff.nodes_removed = ids(&b_nodes
        .iter()
        .filter(|x| !t_nodes.contains(x))
        .cloned()
        .collect());
    diff.edges_added = ids(&t_edges
        .iter()
        .filter(|x| !b_edges.contains(x))
        .cloned()
        .collect());
    diff.edges_removed = ids(&b_edges
        .iter()
        .filter(|x| !t_edges.contains(x))
        .cloned()
        .collect());
    diff.features_added = ids(&t_feats
        .iter()
        .filter(|x| !b_feats.contains(x))
        .cloned()
        .collect());
    diff.features_removed = ids(&b_feats
        .iter()
        .filter(|x| !t_feats.contains(x))
        .cloned()
        .collect());
    diff.changed = !diff.fields_changed.is_empty()
        || !diff.nodes_added.is_empty()
        || !diff.nodes_removed.is_empty()
        || !diff.edges_added.is_empty()
        || !diff.edges_removed.is_empty()
        || !diff.features_added.is_empty()
        || !diff.features_removed.is_empty();
    diff
}

// ========== 版本 API 路由 ==========

/// 版本管理路由：挂载到 /api/market 下
pub fn version_routes() -> Router<MarketState> {
    Router::new()
        .route("/:id/versions", get(list_versions_handler))
        .route("/:id/versions/compare", get(compare_versions_handler))
        .route("/:id/versions/:version", get(get_version_handler))
        .route(
            "/:id/rollback/:version",
            axum::routing::post(rollback_handler),
        )
}

/// GET /:id/versions —— 版本列表（含变更日志摘要）
async fn list_versions_handler(
    State(_state): State<MarketState>,
    Path(id): Path<String>,
) -> ApiResponse<serde_json::Value> {
    let versions = list_versions(&id);
    let changelog = read_changelog(&id);
    api_ok(serde_json::json!({
        "success": true,
        "id": id,
        "total": versions.len(),
        "keep_limit": crate::market_migration::keep_versions_limit(),
        "versions": versions,
        "changelog": changelog,
    }))
}

/// GET /:id/versions/compare?base=1.0.0&target=2.0.0 —— 版本差异对比
async fn compare_versions_handler(
    State(_state): State<MarketState>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResponse<serde_json::Value> {
    let base_v = params.get("base").cloned().unwrap_or_default();
    let target_v = params.get("target").cloned().unwrap_or_default();
    if base_v.is_empty() || target_v.is_empty() {
        return api_error(500, "需要 base 与 target 两个版本号");
    }
    let base = match get_version(&id, &base_v) {
        Some(p) => p,
        None => {
            return api_error(500, format!("版本 {} 不存在", base_v))
        }
    };
    // target 允许指向当前最新版
    let target = if target_v == "latest" {
        match load_package(&id) {
            Ok(p) => p,
            Err(_) => {
                return api_error(500, "算子包不存在")
            }
        }
    } else {
        match get_version(&id, &target_v) {
            Some(p) => p,
            None => {
                return api_error(500, format!("版本 {} 不存在", target_v))
            }
        }
    };
    let diff = diff_packages(&base, &target);
    api_ok(serde_json::json!({ "success": true, "id": id, "diff": diff }))
}

/// GET /:id/versions/:version —— 读取指定版本快照（不阻塞最新版读取）
async fn get_version_handler(
    State(_state): State<MarketState>,
    Path((id, version)): Path<(String, String)>,
) -> ApiResponse<serde_json::Value> {
    if version == "latest" {
        return match load_package(&id) {
            Ok(pkg) => api_ok(serde_json::json!({ "success": true, "package": pkg })),
            Err(e) => api_error(500, e),
        };
    }
    match get_version(&id, &version) {
        Some(pkg) => api_ok(serde_json::json!({ "success": true, "package": pkg })),
        None => api_error(500, format!("版本 {} 不存在", version)),
    }
}

/// POST /:id/rollback/:version —— 回滚到指定版本
async fn rollback_handler(
    State(state): State<MarketState>,
    Path((id, version)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
) -> ApiResponse<serde_json::Value> {
    let actor = actor_from_headers(&headers);
    match rollback(&id, &version, &actor) {
        Ok(pkg) => {
            reload_index_sync(&state);
            api_ok(serde_json::json!({ "success": true, "package": pkg, "rolled_back_to": version }),)}
        Err(e) => api_error(400, e),
    }
}

/// 从请求头提取操作人（X-Actor，缺省 anonymous）
pub(crate) fn actor_from_headers(headers: &axum::http::HeaderMap) -> String {
    headers
        .get("x-actor")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "anonymous".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_parse_and_precedence() {
        assert_eq!(SemVer::parse("1.2.3").unwrap().to_string(), "1.2.3");
        assert_eq!(
            SemVer::parse("2.0.0-alpha.1").unwrap().pre.as_deref(),
            Some("alpha.1")
        );
        assert_eq!(SemVer::parse("1.0").unwrap().to_string(), "1.0.0");
        assert!(SemVer::parse("1.2.3.4").is_none());
        assert!(SemVer::parse("abc").is_none());

        // 1.0.0 < 2.0.0
        assert_eq!(version_cmp("1.0.0", "2.0.0"), Ordering::Less);
        // 1.0.0-alpha < 1.0.0
        assert_eq!(version_cmp("1.0.0-alpha", "1.0.0"), Ordering::Less);
        // 1.0.0-alpha < 1.0.0-alpha.1
        assert_eq!(version_cmp("1.0.0-alpha", "1.0.0-alpha.1"), Ordering::Less);
        // 1.0.0-alpha.1 < 1.0.0-alpha.beta（数字 < 字母数字）
        assert_eq!(
            version_cmp("1.0.0-alpha.1", "1.0.0-alpha.beta"),
            Ordering::Less
        );
        // 构建元数据不参与比较
        assert_eq!(version_cmp("1.0.0+build1", "1.0.0"), Ordering::Equal);
        // bump
        assert_eq!(bump_patch_version("1.2.9"), "1.2.10");
        assert_eq!(bump_patch_version("1.2.3-alpha"), "1.2.4");
        assert_eq!(
            SemVer::parse("1.2.3").unwrap().bump_minor().to_string(),
            "1.3.0"
        );
        assert_eq!(
            SemVer::parse("1.2.3").unwrap().bump_major().to_string(),
            "2.0.0"
        );
    }

    #[test]
    fn snapshot_name_parsing() {
        let (v, t) = split_snapshot_name("v1.2.3@1700000000.json");
        assert_eq!(v, "1.2.3");
        assert_eq!(t, 1700000000);
    }
}
