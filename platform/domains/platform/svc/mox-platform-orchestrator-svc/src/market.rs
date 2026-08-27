// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 算子商城 (Operator Market)
//!
//! 把"需求 + 业务流程图（结构化、可编辑）"作为算子包(OperatorPackage)上传到商城，
//! 他人可随机浏览、拉取并克隆后继续编辑。
//!
//! 数据持久化：文件型，统一存储在 `$OUS_HOME/market/packages/<id>.json`（无需数据库）。
//! `$OUS_HOME` 默认取 `~/.ous`（用户主目录下的 `.ous`），可通过环境变量覆盖。
//!
//! ## 路径归一化与迁移（§27 code-path / work-path 隔离）
//! - 首次启动自动检测旧路径 `./data/market`（遗留布局）与 `$OUS_HOME/market/<id>.json`
//!   （中间布局），**自动备份**到 `$OUS_HOME/market/backup/` 后迁移到 `packages/` 子目录。
//! - 读取向后兼容：新布局 → 中间布局 → 遗留布局依次探测，命中旧路径自动补迁。
//!
//! ## 版本化管理（market_version）
//! - semver（主.次.补[-预发布]）；每次实质性更新前自动快照旧版本；
//! - 变更日志 `$OUS_HOME/market/changelog/<id>.md` 自动追加；
//! - 版本查询 / 差异对比 / 回滚 API；历史保留 N 个（`OUS_MARKET_KEEP_VERSIONS`，默认 5）；
//! - **版本化不阻塞读取**：快照写入 best-effort，读取始终以最新版优先。
//!
//! ## 导入 / 导出（routes::market）
//! - 单包导出（JSON/YAML）、全量导出（zip，manifest 带 HMAC-SHA256 签名）；
//! - 导入支持签名校验与冲突策略（overwrite / skip / rename）；全部审计。
//!
//! ## DSL 转换（market_dsl）
//! - 流程图 JSON → FlowDefinition DSL → BusinessWorkflow 自动生成；
//! - 前端预览页 `GET /api/market/:id/dsl/preview` 展示 DSL / Workflow / 代码。
//!
//! ## 权限 / 租户绑定
//! - OperatorPackage 含 `tenant_id` / `created_by` / `permissions`；
//! - 列表支持按租户 / 创建人 / 权限过滤（含专用路由 `/tenant/:id`、`/owner/:id`）。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::market_migration::{audit, find_package_file, now_rfc3339, packages_dir};

/// 商城应用状态
#[derive(Clone)]
pub struct MarketState {
    /// 内存索引：id -> 元信息（用于列表/随机，避免每次读全部文件）
    pub index: Arc<Mutex<HashMap<String, PackageMeta>>>,
}

/// 流程图节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    pub id: String,
    pub label: String,
    /// 节点类型：start / end / process / decision / io / operator / parallel / llm / ...
    #[serde(default = "default_node_type")]
    pub node_type: String,
    /// 画布坐标
    pub x: f64,
    pub y: f64,
    /// 节点备注 / 说明
    #[serde(default)]
    pub note: String,
}

fn default_node_type() -> String {
    "process".to_string()
}

/// 流程图连线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    /// 连线标签（如条件分支文字）
    #[serde(default)]
    pub label: String,
}

/// 功能点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureItem {
    pub id: String,
    pub title: String,
    pub description: String,
    /// 优先级：high / medium / low
    #[serde(default = "default_priority")]
    pub priority: String,
    /// 状态：todo / doing / done
    #[serde(default = "default_status")]
    pub status: String,
}

fn default_priority() -> String {
    "medium".to_string()
}
fn default_status() -> String {
    "todo".to_string()
}

/// 算子包（商城核心资产）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperatorPackage {
    pub id: String,
    pub name: String,
    /// 分类标签
    #[serde(default)]
    pub category: String,
    /// 作者
    #[serde(default)]
    pub author: String,
    /// 版本（semver：主.次.补）
    #[serde(default = "default_version")]
    pub version: String,
    /// 简介
    #[serde(default)]
    pub summary: String,
    /// ===== 最核心：需求描述 =====
    pub requirement: String,
    /// 业务流程图（结构化、可编辑）
    #[serde(default)]
    pub nodes: Vec<FlowNode>,
    #[serde(default)]
    pub edges: Vec<FlowEdge>,
    /// 功能点清单
    #[serde(default)]
    pub features: Vec<FeatureItem>,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 创建时间 (RFC3339)
    #[serde(default)]
    pub created_at: String,
    /// 更新时间
    #[serde(default)]
    pub updated_at: String,
    /// 克隆次数
    #[serde(default)]
    pub clone_count: u64,
    /// 派生自哪个包（克隆溯源）
    #[serde(default)]
    pub forked_from: Option<String>,
    /// 租户归属（归一化到 agent.ctx 作用域，默认 "default"；兼容旧字段）
    #[serde(default = "default_tenant")]
    pub tenant: String,
    /// ===== 权限 / 租户绑定 =====
    /// 租户 ID（与 tenant 同步；旧数据反序列化时默认 "default"）
    #[serde(default = "default_tenant")]
    pub tenant_id: String,
    /// 创建人（账号/agent 标识）
    #[serde(default)]
    pub created_by: String,
    /// 权限标记列表（如 ["read", "write", "deploy"]，空 = 公开可读）
    #[serde(default)]
    pub permissions: Vec<String>,
    /// ===== 产物来源追溯 (I-07) =====
    /// 来源璇玑流程 ID（归一化出码的资产可追溯回哪条需求流程图）
    #[serde(default)]
    pub source_flow_id: Option<String>,
    /// 来源任务 ID（双璇玑任务闭环：需求 -> 开发 -> 归一治理）
    #[serde(default)]
    pub source_task_id: Option<String>,
    /// 双验收结论快照（任务 Done ∧ 融合验证通过）
    #[serde(default)]
    pub dual_acceptance: bool,
    /// 优化前后指标：关键路径长度、加速比、冲突数（供审计/复用证据）
    #[serde(default)]
    pub provenance: Option<ProvenanceMetrics>,
}

/// 产物来源追溯指标（I-07）
/// 【大白话】"上架的算子从哪来、优化前什么样、优化后什么样"——把璇玑归一化的
/// 核心收益固化进算子包，下游复用/审计时可一键看出这条资产值不值、靠不靠谱。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceMetrics {
    /// 来源璇玑/融合验证是否通过（最高权限的算法验证）
    pub algo_verified: bool,
    /// 治理 8 闸门是否全过
    pub gates_passed: bool,
    /// 优化前关键路径节点数
    pub critical_path_before: usize,
    /// 优化后关键路径节点数（并行化压缩后）
    pub critical_path_after: usize,
    /// 加速比（并行/关键路径）
    pub speedup: f64,
    /// 冲突数（含阻断级）
    pub conflicts: usize,
    /// 专家平均健康分（0~100）
    pub expert_score: f64,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_tenant() -> String {
    "default".to_string()
}

/// 列表用的轻量元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMeta {
    pub id: String,
    pub name: String,
    pub category: String,
    pub author: String,
    pub version: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub clone_count: u64,
    pub node_count: usize,
    pub feature_count: usize,
    pub tenant: String,
    /// 租户 ID（权限过滤用）
    #[serde(default = "default_tenant")]
    pub tenant_id: String,
    /// 创建人
    #[serde(default)]
    pub created_by: String,
    /// 权限标记（空 = 公开可读）
    #[serde(default)]
    pub permissions: Vec<String>,
}

impl OperatorPackage {
    fn meta(&self) -> PackageMeta {
        PackageMeta {
            id: self.id.clone(),
            name: self.name.clone(),
            category: self.category.clone(),
            author: self.author.clone(),
            version: self.version.clone(),
            summary: self.summary.clone(),
            tags: self.tags.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            clone_count: self.clone_count,
            node_count: self.nodes.len(),
            feature_count: self.features.len(),
            tenant: self.tenant.clone(),
            tenant_id: self.tenant_id.clone(),
            created_by: self.created_by.clone(),
            permissions: self.permissions.clone(),
        }
    }
}

/// 上传/创建算子包请求（id 由服务端生成）
#[derive(Debug, Deserialize)]
pub struct CreatePackageRequest {
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub summary: String,
    pub requirement: String,
    #[serde(default)]
    pub nodes: Vec<FlowNode>,
    #[serde(default)]
    pub edges: Vec<FlowEdge>,
    #[serde(default)]
    pub features: Vec<FeatureItem>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 若提供，则从该包克隆（仅复制核心内容，重置溯源与计数）
    #[serde(default)]
    pub forked_from: Option<String>,
    /// 租户归属（可选，默认 "default"）
    #[serde(default = "default_tenant")]
    pub tenant: String,
    /// 租户 ID（可选，缺省同 tenant）
    #[serde(default)]
    pub tenant_id: String,
    /// 创建人
    #[serde(default)]
    pub created_by: String,
    /// 权限标记
    #[serde(default)]
    pub permissions: Vec<String>,
}

/// 更新算子包请求（全量覆盖核心字段）
#[derive(Debug, Deserialize)]
pub struct UpdatePackageRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub requirement: Option<String>,
    #[serde(default)]
    pub nodes: Option<Vec<FlowNode>>,
    #[serde(default)]
    pub edges: Option<Vec<FlowEdge>>,
    #[serde(default)]
    pub features: Option<Vec<FeatureItem>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub tenant: Option<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub permissions: Option<Vec<String>>,
}

/// 规范包文件路径：`$OUS_HOME/market/packages/<id>.json`
pub fn package_path(id: &str) -> PathBuf {
    crate::market_migration::package_path(id)
}

pub(crate) fn gen_id() -> String {
    uuid::Uuid::new_v4()
        .to_string()
        .split('-')
        .next()
        .unwrap()
        .to_string()
}

/// 版本号自动 bump：x.y.z -> x.y.(z+1)（语义化版本，market_version 实现）
fn bump_patch_version(v: &str) -> String {
    crate::market_version::bump_patch_version(v)
}

/// 重新从磁盘加载全部包到内存索引（异步版本，锁 index）
pub async fn reload_index(state: &MarketState) {
    let map = &state.index;
    if let Ok(mut guard) = map.try_lock() {
        *guard = scan_index();
    } else {
        let mut guard = map.lock().await;
        *guard = scan_index();
    }
}

/// 同步重建索引（try_lock；测试与少量同步调用点使用）
pub fn reload_index_sync(state: &MarketState) {
    if let Ok(mut guard) = state.index.try_lock() {
        *guard = scan_index();
    }
}

fn scan_index() -> HashMap<String, PackageMeta> {
    let mut map = HashMap::new();
    let dir = packages_dir();
    if !dir.exists() {
        return map;
    }
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(pkg) = serde_json::from_str::<OperatorPackage>(&content) {
                        map.insert(pkg.id.clone(), pkg.meta());
                    }
                }
            }
        }
    }
    map
}

/// 初始化商城状态，并确保种子数据存在
pub async fn init_market_state() -> MarketState {
    // 路径归一化：首次启动自动迁移旧路径（自动备份）
    let report = crate::market_migration::ensure_migrated();
    if report.migrated_from_legacy > 0 || report.migrated_from_root > 0 {
        tracing::info!(
            "算子商城迁移完成：legacy={} root={} backup={:?}",
            report.migrated_from_legacy,
            report.migrated_from_root,
            report.backup_dir
        );
    }
    let dir = packages_dir();
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    let state = MarketState {
        index: Arc::new(Mutex::new(HashMap::new())),
    };
    // 确保至少有种子示例（用项目真实"全业务流程"做种子）
    ensure_seed(&state).await;
    reload_index(&state).await;
    state
}

/// 把商城路由挂载到指定 path 前缀（默认 /api/market）
/// 合并：基础 CRUD + 版本化 + DSL 转换 + 导入导出/租户扩展
pub fn market_routes() -> Router<MarketState> {
    Router::new()
        .route("/", get(list_packages))
        .route("/random", get(random_package))
        .route("/:id", get(get_package))
        .route("/:id", post(update_package))
        .route("/:id", delete(delete_package))
        .route("/:id/clone", post(clone_package))
        .route("/upload", post(upload_package))
        .route("/:id/export", get(export_package))
        .route("/backup", post(backup_market))
        .merge(crate::market_version::version_routes())
        .merge(crate::market_dsl::dsl_routes())
}

// ========== Handlers ==========

/// 手动全量备份：`$OUS_HOME/market/backup/manual-<ts>/`
async fn backup_market() -> Json<serde_json::Value> {
    match crate::market_migration::backup_now("manual") {
        Some(dir) => Json(serde_json::json!({
            "success": true,
            "backup_dir": dir.display().to_string(),
        })),
        None => Json(serde_json::json!({
            "success": false,
            "error": "备份失败：市场目录不存在或不可写（检查 $OUS_HOME/market）",
        })),
    }
}

/// 列表（支持 ?category= / ?tag= / ?q= / ?tenant_id= / ?created_by= / ?perm= 过滤）
async fn list_packages(
    State(state): State<MarketState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let category = params.get("category");
    let tag = params.get("tag");
    let q = params.get("q").map(|s| s.to_lowercase());
    let tenant_id = params.get("tenant_id");
    let created_by = params.get("created_by");
    let perm = params.get("perm");
    let list = list_packages_filtered(
        &state,
        category.map(|s| s.as_str()),
        tag.map(|s| s.as_str()),
        q.as_deref(),
        tenant_id.map(|s| s.as_str()),
        created_by.map(|s| s.as_str()),
        perm.map(|s| s.as_str()),
    );
    Json(serde_json::json!({ "success": true, "total": list.len(), "packages": list }))
}

/// 供本模块与 routes 扩展使用的过滤列表实现（按更新时间倒序）
pub fn list_packages_filtered(
    state: &MarketState,
    category: Option<&str>,
    tag: Option<&str>,
    q: Option<&str>,
    tenant_id: Option<&str>,
    created_by: Option<&str>,
    perm: Option<&str>,
) -> Vec<PackageMeta> {
    let idx = state.index.try_lock();
    let mut list: Vec<PackageMeta> = match idx {
        Ok(guard) => guard
            .values()
            .filter(|m| {
                if let Some(c) = category {
                    if !c.is_empty() && m.category != c {
                        return false;
                    }
                }
                if let Some(t) = tag {
                    if !t.is_empty() && !m.tags.iter().any(|x| x == t) {
                        return false;
                    }
                }
                if let Some(q) = q {
                    if !q.is_empty()
                        && !m.name.to_lowercase().contains(q)
                        && !m.summary.to_lowercase().contains(q)
                    {
                        return false;
                    }
                }
                if let Some(t) = tenant_id {
                    if !t.is_empty() && m.tenant_id != t {
                        return false;
                    }
                }
                if let Some(c) = created_by {
                    if !c.is_empty() && m.created_by != c {
                        return false;
                    }
                }
                if let Some(p) = perm {
                    // 权限过滤：permissions 为空视为公开；否则须包含所请求权限
                    if !p.is_empty()
                        && !m.permissions.is_empty()
                        && !m.permissions.iter().any(|x| x == p)
                    {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect(),
        Err(_) => Vec::new(),
    };
    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    list
}

/// 随机返回一个包（用于"随机浏览/随机剪饮"）
async fn random_package(State(state): State<MarketState>) -> Json<serde_json::Value> {
    let idx = state.index.lock().await;
    let vals: Vec<&PackageMeta> = idx.values().collect();
    if vals.is_empty() {
        return Json(serde_json::json!({ "success": false, "error": "商城暂无算子包" }));
    }
    // 用时间做简单随机
    let i = (chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or(0)
        .unsigned_abs()
        % vals.len() as u64) as usize;
    let id = vals[i].id.clone();
    drop(idx);
    get_package(State(state), Path(id)).await
}

/// 获取单个包完整内容（向后兼容：新布局 → 中间布局 → 遗留布局）
async fn get_package(
    State(_state): State<MarketState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match load_package(&id) {
        Ok(pkg) => Json(serde_json::json!({ "success": true, "package": pkg })),
        Err(e) => Json(serde_json::json!({ "success": false, "error": e })),
    }
}

/// 上传/创建新算子包
async fn upload_package(
    State(state): State<MarketState>,
    Json(req): Json<CreatePackageRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if req.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": "算子包名称不能为空" })),
        );
    }
    if req.requirement.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({ "success": false, "error": "需求描述(requirement)不能为空，这是算子包最核心的部分" }),
            ),
        );
    }
    // 版本字段遵循 semver（主.次.补[-预发布]）
    if !req.version.is_empty() && !crate::market_version::is_valid_version(&req.version) {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({ "success": false, "error": format!("版本号不是合法 semver（主.次.补）: {}", req.version) }),
            ),
        );
    }

    let id = gen_id();
    let now = now_rfc3339();
    let tenant = if req.tenant.is_empty() {
        default_tenant()
    } else {
        req.tenant
    };
    let tenant_id = if req.tenant_id.is_empty() {
        tenant.clone()
    } else {
        req.tenant_id
    };
    let pkg = OperatorPackage {
        id: id.clone(),
        name: req.name,
        category: req.category,
        author: req.author,
        version: if req.version.is_empty() {
            default_version()
        } else {
            req.version
        },
        summary: req.summary,
        requirement: req.requirement,
        nodes: req.nodes,
        edges: req.edges,
        features: req.features,
        tags: req.tags,
        created_at: now.clone(),
        updated_at: now,
        clone_count: 0,
        forked_from: req.forked_from,
        tenant,
        tenant_id,
        created_by: req.created_by,
        permissions: req.permissions,
        ..Default::default()
    };

    if let Err(e) = save_package(&pkg) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "success": false, "error": format!("保存失败: {}", e) })),
        );
    }
    // 更新索引
    state.index.lock().await.insert(pkg.id.clone(), pkg.meta());
    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "id": id, "package": pkg })),
    )
}

/// 全量更新算子包（需求/流程图/功能点都改；自动快照旧版本 + 版本号 bump）
async fn update_package(
    State(state): State<MarketState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(req): Json<UpdatePackageRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let actor = crate::market_version::actor_from_headers(&headers);
    let mut pkg = match load_package(&id) {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "success": false, "error": "算子包不存在" })),
            )
        }
    };

    // 版本化：先快照旧版本（best-effort，不阻塞主流程）
    let _ = crate::market_version::snapshot_package(&pkg, &actor, "更新前快照");

    if let Some(v) = req.name {
        pkg.name = v;
    }
    if let Some(v) = req.category {
        pkg.category = v;
    }
    if let Some(v) = req.author {
        pkg.author = v;
    }
    // 归一化：除非显式传了版本，否则每次实质性更新自动 +1 补丁号（版本化）
    if let Some(v) = req.version {
        if !crate::market_version::is_valid_version(&v) {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({ "success": false, "error": format!("版本号不是合法 semver（主.次.补）: {}", v) }),
                ),
            );
        }
        pkg.version = v;
    } else {
        pkg.version = bump_patch_version(&pkg.version);
    }
    if let Some(v) = req.summary {
        pkg.summary = v;
    }
    if let Some(v) = req.requirement {
        pkg.requirement = v;
    }
    if let Some(v) = req.nodes {
        pkg.nodes = v;
    }
    if let Some(v) = req.edges {
        pkg.edges = v;
    }
    if let Some(v) = req.features {
        pkg.features = v;
    }
    if let Some(v) = req.tags {
        pkg.tags = v;
    }
    if let Some(v) = req.tenant {
        pkg.tenant = v;
    }
    if let Some(v) = req.tenant_id {
        pkg.tenant_id = v;
        if pkg.tenant.is_empty() {
            pkg.tenant = pkg.tenant_id.clone();
        }
    }
    if let Some(v) = req.created_by {
        pkg.created_by = v;
    }
    if let Some(v) = req.permissions {
        pkg.permissions = v;
    }
    pkg.updated_at = now_rfc3339();

    if let Err(e) = save_package(&pkg) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "success": false, "error": format!("保存失败: {}", e) })),
        );
    }
    state.index.lock().await.insert(pkg.id.clone(), pkg.meta());
    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "package": pkg })),
    )
}

/// 克隆（fork）：复制核心内容到新包，溯源指向原包，原包 clone_count+1
async fn clone_package(
    State(state): State<MarketState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut src = match load_package(&id) {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "success": false, "error": "源算子包不存在" })),
            )
        }
    };

    // 原包计数 +1
    src.clone_count += 1;
    src.updated_at = now_rfc3339();
    let _ = save_package(&src);
    state.index.lock().await.insert(src.id.clone(), src.meta());

    // 生成新包（租户保持源包归属；克隆人/权限留空由后续编辑填充）
    let new_id = gen_id();
    let now = now_rfc3339();
    let cloned = OperatorPackage {
        id: new_id.clone(),
        name: format!("{}-副本", src.name),
        category: src.category,
        author: String::new(),
        version: src.version,
        summary: src.summary,
        requirement: src.requirement,
        nodes: src.nodes,
        edges: src.edges,
        features: src.features,
        tags: src.tags,
        created_at: now.clone(),
        updated_at: now,
        clone_count: 0,
        forked_from: Some(src.id.clone()),
        tenant: src.tenant.clone(),
        tenant_id: src.tenant_id.clone(),
        created_by: String::new(),
        permissions: src.permissions.clone(),
        ..Default::default()
    };
    if let Err(e) = save_package(&cloned) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "success": false, "error": format!("克隆保存失败: {}", e) })),
        );
    }
    state
        .index
        .lock()
        .await
        .insert(cloned.id.clone(), cloned.meta());
    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "id": new_id, "package": cloned })),
    )
}

/// 删除算子包（含版本快照与变更日志）
async fn delete_package(
    State(state): State<MarketState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let path = match find_package_file(&id) {
        Some(p) => p,
        None => {
            return Json(serde_json::json!({ "success": false, "error": "算子包不存在" }));
        }
    };
    let _ = std::fs::remove_file(&path);
    // 清理版本快照与变更日志
    let _ = std::fs::remove_dir_all(crate::market_migration::versions_dir(&id));
    let _ = std::fs::remove_file(crate::market_version::changelog_path(&id));
    state.index.lock().await.remove(&id);
    audit("delete", "anonymous", &format!("删除算子包 {}", id));
    Json(serde_json::json!({ "success": true }))
}

/// 导出归一化 DSL（§28 FlowDefinition）：保持向后兼容（前端 /market/:id/export 依赖）。
/// 把算子包的核心资产（需求 + 流程图 + 功能点）投影为与内核 FlowDefinition
/// 一致的规范结构。更完整的转换链路见 market_dsl（/:id/dsl、/:id/workflow、/:id/dsl/preview）。
async fn export_package(
    State(_state): State<MarketState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let pkg = match load_package(&id) {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "success": false, "error": "算子包不存在" })),
            )
        }
    };

    // 归一化投影：nodes -> flow.vertices, edges -> flow.edges, requirement -> spec
    let vertices: Vec<serde_json::Value> = pkg
        .nodes
        .iter()
        .map(|n| {
            serde_json::json!({
                "id": n.id,
                "label": n.label,
                "type": n.node_type,
                "position": { "x": n.x, "y": n.y },
                "note": n.note,
            })
        })
        .collect();
    let edges: Vec<serde_json::Value> = pkg
        .edges
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "source": e.source,
                "target": e.target,
                "label": e.label,
            })
        })
        .collect();
    let features: Vec<serde_json::Value> = pkg
        .features
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": f.id,
                "title": f.title,
                "description": f.description,
                "priority": f.priority,
                "status": f.status,
            })
        })
        .collect();

    let dsl = serde_json::json!({
        "kind": "FlowDefinition",
        "schema_version": "2026.1",
        "id": pkg.id,
        "name": pkg.name,
        "category": pkg.category,
        "tenant": pkg.tenant,
        "tenant_id": pkg.tenant_id,
        "created_by": pkg.created_by,
        "permissions": pkg.permissions,
        "author": pkg.author,
        "version": pkg.version,
        "summary": pkg.summary,
        "tags": pkg.tags,
        "spec": {
            "requirement": pkg.requirement,
            "features": features,
        },
        "flow": {
            "vertices": vertices,
            "edges": edges,
        },
        "derivation": {
            "forked_from": pkg.forked_from,
            "clone_count": pkg.clone_count,
        },
        "meta": {
            "created_at": pkg.created_at,
            "updated_at": pkg.updated_at,
        },
    });

    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "dsl": dsl })),
    )
}

// ========== 存储辅助 ==========

/// 保存算子包到 `$OUS_HOME/market/packages/<id>.json`
pub fn save_package(pkg: &OperatorPackage) -> std::io::Result<()> {
    let dir = packages_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    let content = serde_json::to_string_pretty(pkg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(package_path(&pkg.id), content)
}

/// 读取算子包（向后兼容：新布局 → 中间布局 → 遗留布局；命中旧路径自动补迁）
pub fn load_package(id: &str) -> Result<OperatorPackage, String> {
    let path = find_package_file(id).ok_or_else(|| "算子包不存在".to_string())?;
    let content = std::fs::read_to_string(&path).map_err(|e| format!("读取失败: {}", e))?;
    let pkg: OperatorPackage =
        serde_json::from_str(&content).map_err(|e| format!("解析失败: {}", e))?;
    // 命中旧路径（非归一化位置）时自动补迁，保持磁盘布局收敛
    let canonical = package_path(id);
    if path != canonical && !canonical.exists() {
        // 确保目标目录存在（首次读取遗留包时 packages/ 可能尚未创建）
        if let Some(parent) = canonical.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::copy(&path, &canonical).is_ok() {
            audit(
                "auto_migrate",
                "system",
                &format!(
                    "读取时自动迁移 {} → {}",
                    path.display(),
                    canonical.display()
                ),
            );
            let _ = std::fs::remove_file(&path);
        }
    }
    Ok(pkg)
}

/// 种子数据：用项目真实的"全业务流程"做示例算子包
async fn ensure_seed(state: &MarketState) {
    let dir = packages_dir();
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    // 若已有数据则不重复写种子
    if let Ok(mut entries) = std::fs::read_dir(&dir) {
        if entries.next().is_some() {
            return;
        }
    }
    let seed = OperatorPackage {
        id: "seed-ous-full-flow".to_string(),
        name: "算子统一系统·全业务流程".to_string(),
        category: "平台/编排".to_string(),
        author: "OUS 团队".to_string(),
        version: "1.0.0".to_string(),
        summary: "从接入层到内核执行再回流观测的完整企业级业务编排流程。".to_string(),
        requirement: "企业需要一套通用计算与编排底座：将任意业务操作抽象为算子，通过有向加权图描述关联，提供 DAG 调度、关键路径分析、资源约束优化的执行引擎，并支持 WASM 插件沙箱与 AI 智能体协同。需求一旦确定，编排层/内核/前端均可基于此快速调整。".to_string(),
        nodes: vec![
            FlowNode { id: "n1".into(), label: "用户请求".into(), node_type: "start".into(), x: 80.0, y: 40.0, note: "前端/REST/WebSocket 入口".into() },
            FlowNode { id: "n2".into(), label: "接入层".into(), node_type: "io".into(), x: 80.0, y: 160.0, note: "Vue3 / REST API / SSE 实时流".into() },
            FlowNode { id: "n3".into(), label: "运行时鉴权/路由".into(), node_type: "process".into(), x: 80.0, y: 280.0, note: "RBAC + 限流 + 算子路由".into() },
            FlowNode { id: "n4".into(), label: "编排与优化层".into(), node_type: "process".into(), x: 80.0, y: 400.0, note: "拓扑/关键路径/DAG 调度/资源约束".into() },
            FlowNode { id: "n5".into(), label: "算子内核执行".into(), node_type: "process".into(), x: 80.0, y: 520.0, note: "Core/Graph/WASM 插件/外部系统".into() },
            FlowNode { id: "n6".into(), label: "观测回流".into(), node_type: "io".into(), x: 320.0, y: 520.0, note: "日志/指标/追踪".into() },
            FlowNode { id: "n7".into(), label: "结果实时推回".into(), node_type: "end".into(), x: 320.0, y: 400.0, note: "WebSocket 推送前端".into() },
        ],
        edges: vec![
            FlowEdge { id: "e1".into(), source: "n1".into(), target: "n2".into(), label: "".into() },
            FlowEdge { id: "e2".into(), source: "n2".into(), target: "n3".into(), label: "".into() },
            FlowEdge { id: "e3".into(), source: "n3".into(), target: "n4".into(), label: "".into() },
            FlowEdge { id: "e4".into(), source: "n4".into(), target: "n5".into(), label: "".into() },
            FlowEdge { id: "e5".into(), source: "n5".into(), target: "n6".into(), label: "".into() },
            FlowEdge { id: "e6".into(), source: "n6".into(), target: "n7".into(), label: "".into() },
            FlowEdge { id: "e7".into(), source: "n7".into(), target: "n2".into(), label: "回流".into() },
        ],
        features: vec![
            FeatureItem { id: "f1".into(), title: "算子抽象与组合".into(), description: "万物皆算子，满足结合律/单位律".into(), priority: "high".into(), status: "done".into() },
            FeatureItem { id: "f2".into(), title: "DAG 调度优化".into(), description: "关键路径分析 + 资源约束".into(), priority: "high".into(), status: "doing".into() },
            FeatureItem { id: "f3".into(), title: "WASM 插件沙箱".into(), description: "安全热加载第三方算子".into(), priority: "medium".into(), status: "todo".into() },
            FeatureItem { id: "f4".into(), title: "AI 智能体协同".into(), description: "LLM 编排/浏览器自动化/多专家".into(), priority: "medium".into(), status: "todo".into() },
        ],
        tags: vec!["编排".into(), "流程图".into(), "企业级".into()],
        created_at: now_rfc3339(),
        updated_at: now_rfc3339(),
        clone_count: 0,
        forked_from: None,
        tenant: default_tenant(),
        tenant_id: default_tenant(),
        created_by: "system".to_string(),
        permissions: vec![],
        ..Default::default()
    };
    let _ = save_package(&seed);
    state
        .index
        .lock()
        .await
        .insert(seed.id.clone(), seed.meta());
}

/// 全维融合落盘：接收璇玑归一化产出的优化流程图（mox_ai_flow_svc::FlowNode/FlowEdge）+
/// 元信息，转换为算子商城节点模型并组装为算子包，上传到市场（插件/应用平台）。
/// 这是"璇玑 -> 业务流程图 -> 上传系统平台"融合总线的最终落点。
#[allow(clippy::too_many_arguments)]
pub fn publish_unified(
    name: String,
    description: String,
    requirement: String,
    nodes: Vec<mox_ai_flow_svc::model::FlowNode>,
    edges: Vec<mox_ai_flow_svc::model::FlowEdge>,
    tags: Vec<String>,
    // 来源璇玑全维治理报告（I-07 产物来源追溯）
    report: Option<&mox_ai_expert_svc::pipeline::GovernanceReport>,
    // 来源任务 ID（双璇玑任务闭环）
    task_id: Option<String>,
) -> std::io::Result<OperatorPackage> {
    let id = gen_id();
    let ts = now_rfc3339();
    // mox_ai_flow_svc 模型 -> 算子商城展示模型（字段结构不同，做归一化映射）
    let market_nodes: Vec<FlowNode> = nodes
        .into_iter()
        .enumerate()
        .map(|(i, n)| FlowNode {
            id: n.id.clone(),
            label: n.name,
            node_type: format!("{:?}", n.kind).to_lowercase(),
            x: 80.0 + (i % 4) as f64 * 240.0,
            y: 40.0 + (i / 4) as f64 * 140.0,
            note: n.tool.map(|t| format!("{:?}", t)).unwrap_or_default(),
        })
        .collect();
    let market_edges: Vec<FlowEdge> = edges
        .into_iter()
        .enumerate()
        .map(|(i, e)| FlowEdge {
            id: format!("e{}", i),
            source: e.from,
            target: e.to,
            label: e.condition.unwrap_or_else(|| format!("{:?}", e.kind)),
        })
        .collect();

    // I-07 产物来源追溯：从治理报告固化"优化前/后"证据
    let (source_flow_id, dual_acceptance, provenance) = match report {
        Some(r) => {
            let critical_before = r.optimization.optimized_graph.nodes.len();
            let critical_after = r
                .optimization
                .critical_path
                .critical_paths
                .iter()
                .map(|p| p.len())
                .max()
                .unwrap_or(1)
                .max(1);
            let speedup = if critical_after > 0 {
                critical_before as f64 / critical_after as f64
            } else {
                r.optimization.gains.speedup
            };
            let conflicts = r.optimization.conflicts.conflicts.len();
            let expert_score = if r.expert_scores.is_empty() {
                100.0
            } else {
                r.expert_scores.iter().map(|(_, s)| *s).sum::<f64>() / r.expert_scores.len() as f64
            };
            let dual_ok = !r.algo.vetoed && r.gate.approved;
            (
                Some(r.flow_id.clone()),
                dual_ok,
                Some(ProvenanceMetrics {
                    algo_verified: !r.algo.vetoed,
                    gates_passed: r.gate.approved,
                    critical_path_before: critical_before,
                    critical_path_after: critical_after,
                    speedup,
                    conflicts,
                    expert_score,
                }),
            )
        }
        None => (None, false, None),
    };

    let pkg = OperatorPackage {
        id: id.clone(),
        name,
        category: "unified".into(),
        author: "mox-expert".into(),
        version: default_version(),
        summary: description,
        requirement,
        nodes: market_nodes,
        edges: market_edges,
        features: vec![],
        tags,
        created_at: ts.clone(),
        updated_at: ts,
        clone_count: 0,
        forked_from: None,
        tenant: default_tenant(),
        tenant_id: default_tenant(),
        created_by: "mox-expert".into(),
        permissions: vec![],
        source_flow_id,
        source_task_id: task_id,
        dual_acceptance,
        provenance,
    };
    save_package(&pkg)?;
    Ok(pkg)
}
