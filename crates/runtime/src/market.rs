//! # 算子商城 (Operator Market)
//!
//! 把"需求 + 业务流程图（结构化、可编辑）"作为算子包(OperatorPackage)上传到商城，
//! 他人可随机浏览、拉取并克隆后继续编辑。
//!
//! 数据持久化：文件型，统一存储在 `$OUS_HOME/market/<id>.json`，无需数据库。
//! `$OUS_HOME` 默认取 `~/.ous`（即用户主目录下的 `.ous`），可通过环境变量覆盖。
//! 旧的 `./data/market`（项目相对路径）会在首次启动时自动迁移到 `$OUS_HOME/market`，
//! 实现 path 归一化（§27：code-path / work-path 隔离）。
//!
//! 核心设计：需求一旦确定，其他（流程图、功能点）都可快速改 —— 因此流程图为
//! 结构化节点/连线数据 (Vec<Node>, Vec<Edge>)，而非死图片。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// OUS 归一化根目录：默认 `~/.ous`，可由 `OUS_HOME` 环境变量覆盖。
fn ous_home() -> PathBuf {
    if let Ok(v) = std::env::var("OUS_HOME") {
        if !v.trim().is_empty() {
            return PathBuf::from(v.trim());
        }
    }
    // 回退到用户主目录下的 .ous
    if let Some(home) = dirs_home() {
        return home.join(".ous");
    }
    PathBuf::from(".ous")
}

/// 跨平台取用户主目录（避免额外依赖）
fn dirs_home() -> Option<PathBuf> {
    if let Ok(v) = std::env::var("HOME") {
        if !v.is_empty() {
            return Some(PathBuf::from(v));
        }
    }
    if let Ok(v) = std::env::var("USERPROFILE") {
        if !v.is_empty() {
            return Some(PathBuf::from(v));
        }
    }
    None
}

/// 旧的相对路径存储目录（归一化前的遗留位置）
fn legacy_market_dir() -> PathBuf {
    PathBuf::from("./data/market")
}

/// 把旧的 `./data/market` 中的包文件一次性迁移到 `$OUS_HOME/market`。
/// 仅迁移目标目录尚不存在的文件；已存在则跳过（避免覆盖）。
fn migrate_legacy_dir() {
    let src = legacy_market_dir();
    if !src.exists() {
        return;
    }
    let dst = market_dir();
    let _ = std::fs::create_dir_all(&dst);
    if let Ok(entries) = std::fs::read_dir(&src) {
        let mut moved = 0u32;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(name) = path.file_name() {
                    let target = dst.join(name);
                    if !target.exists() {
                        if std::fs::rename(&path, &target).is_ok() {
                            moved += 1;
                        }
                    } else {
                        // 目标已存在，仅删除旧文件以免遗留
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
        if moved > 0 {
            tracing::info!("算子商城路径归一化：已从 ./data/market 迁移 {} 个包到 {}", moved, dst.display());
        }
    }
}

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
    /// 节点类型：start / end / process / decision / io / operator
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorPackage {
    pub id: String,
    pub name: String,
    /// 分类标签
    #[serde(default)]
    pub category: String,
    /// 作者
    #[serde(default)]
    pub author: String,
    /// 版本
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
    /// 租户归属（归一化到 agent.ctx 作用域，默认 "default"）
    #[serde(default = "default_tenant")]
    pub tenant: String,
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
}

fn market_dir() -> PathBuf {
    ous_home().join("market")
}

fn package_path(id: &str) -> PathBuf {
    market_dir().join(format!("{}.json", id))
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn gen_id() -> String {
    uuid::Uuid::new_v4().to_string().split('-').next().unwrap().to_string()
}

/// 版本号自动 bump：x.y.z -> x.y.(z+1)；解析失败则回退到 +1 补丁。
/// 归一化规则：每次实质性更新都让版本号单调前进，便于溯源与回滚。
fn bump_patch_version(v: &str) -> String {
    let parts: Vec<&str> = v.split('.').collect();
    let mut nums: Vec<u32> = parts.iter().map(|p| p.parse::<u32>().unwrap_or(0)).collect();
    while nums.len() < 3 {
        nums.push(0);
    }
    nums[2] += 1;
    format!("{}.{}.{}", nums[0], nums[1], nums[2])
}

/// 重新从磁盘加载全部包到内存索引
async fn reload_index(state: &MarketState) {
    let mut map = state.index.lock().await;
    map.clear();
    let dir = market_dir();
    if !dir.exists() {
        return;
    }
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|_| {
        // 返回空迭代器
        std::fs::read_dir(".").unwrap()
    });
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

/// 初始化商城状态，并确保种子数据存在
pub async fn init_market_state() -> MarketState {
    // 路径归一化：首次启动把遗留 ./data/market 迁移到 $OUS_HOME/market
    migrate_legacy_dir();
    let dir = market_dir();
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
}

// ========== Handlers ==========

/// 列表（支持 ?category= 与 ?tag= 过滤，?q= 关键字搜索）
async fn list_packages(
    State(state): State<MarketState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let idx = state.index.lock().await;
    let category = params.get("category");
    let tag = params.get("tag");
    let q = params.get("q").map(|s| s.to_lowercase());

    let mut list: Vec<PackageMeta> = idx
        .values()
        .filter(|m| {
            if let Some(c) = category {
                if !c.is_empty() && &m.category != c {
                    return false;
                }
            }
            if let Some(t) = tag {
                if !t.is_empty() && !m.tags.iter().any(|x| x == t) {
                    return false;
                }
            }
            if let Some(q) = &q {
                if !q.is_empty()
                    && !m.name.to_lowercase().contains(q)
                    && !m.summary.to_lowercase().contains(q)
                {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    // 按更新时间倒序
    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Json(serde_json::json!({ "success": true, "total": list.len(), "packages": list }))
}

/// 随机返回一个包（用于"随机浏览/随机剪饮"）
async fn random_package(State(state): State<MarketState>) -> Json<serde_json::Value> {
    let idx = state.index.lock().await;
    let vals: Vec<&PackageMeta> = idx.values().collect();
    if vals.is_empty() {
        return Json(serde_json::json!({ "success": false, "error": "商城暂无算子包" }));
    }
    // 用时间做简单随机
    let i = (chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0).unsigned_abs()
        % vals.len() as u64) as usize;
    let id = vals[i].id.clone();
    drop(idx);
    get_package(State(state), Path(id)).await
}

/// 获取单个包完整内容
async fn get_package(State(_state): State<MarketState>, Path(id): Path<String>) -> Json<serde_json::Value> {
    let path = package_path(&id);
    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<OperatorPackage>(&content) {
            Ok(pkg) => Json(serde_json::json!({ "success": true, "package": pkg })),
            Err(e) => Json(serde_json::json!({ "success": false, "error": format!("解析失败: {}", e) })),
        },
        Err(_) => Json(serde_json::json!({ "success": false, "error": "算子包不存在" })),
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
            Json(serde_json::json!({ "success": false, "error": "需求描述(requirement)不能为空，这是算子包最核心的部分" })),
        );
    }

    let id = gen_id();
    let now = now_rfc3339();
    let pkg = OperatorPackage {
        id: id.clone(),
        name: req.name,
        category: req.category,
        author: req.author,
        version: if req.version.is_empty() { default_version() } else { req.version },
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
        tenant: req.tenant,
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

/// 全量更新算子包（需求/流程图/功能点都改）
async fn update_package(
    State(state): State<MarketState>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePackageRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let path = package_path(&id);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "success": false, "error": "算子包不存在" })),
            )
        }
    };
    let mut pkg: OperatorPackage = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "success": false, "error": format!("解析失败: {}", e) })),
            )
        }
    };

    if let Some(v) = req.name { pkg.name = v; }
    if let Some(v) = req.category { pkg.category = v; }
    if let Some(v) = req.author { pkg.author = v; }
    // 归一化：除非显式传了版本，否则每次实质性更新自动 +1 补丁号（版本化）
    if let Some(v) = req.version {
        pkg.version = v;
    } else {
        pkg.version = bump_patch_version(&pkg.version);
    }
    if let Some(v) = req.summary { pkg.summary = v; }
    if let Some(v) = req.requirement { pkg.requirement = v; }
    if let Some(v) = req.nodes { pkg.nodes = v; }
    if let Some(v) = req.edges { pkg.edges = v; }
    if let Some(v) = req.features { pkg.features = v; }
    if let Some(v) = req.tags { pkg.tags = v; }
    if let Some(v) = req.tenant { pkg.tenant = v; }
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
    let src_path = package_path(&id);
    let content = match std::fs::read_to_string(&src_path) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "success": false, "error": "源算子包不存在" })),
            )
        }
    };
    let mut src: OperatorPackage = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "success": false, "error": format!("解析失败: {}", e) })),
            )
        }
    };

    // 原包计数 +1
    src.clone_count += 1;
    src.updated_at = now_rfc3339();
    let _ = save_package(&src);
    state.index.lock().await.insert(src.id.clone(), src.meta());

    // 生成新包
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
    };
    if let Err(e) = save_package(&cloned) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "success": false, "error": format!("克隆保存失败: {}", e) })),
        );
    }
    state.index.lock().await.insert(cloned.id.clone(), cloned.meta());
    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "id": new_id, "package": cloned })),
    )
}

/// 删除算子包
async fn delete_package(
    State(state): State<MarketState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let path = package_path(&id);
    if !path.exists() {
        return Json(serde_json::json!({ "success": false, "error": "算子包不存在" }));
    }
    let _ = std::fs::remove_file(&path);
    state.index.lock().await.remove(&id);
    Json(serde_json::json!({ "success": true }))
}

/// 导出归一化 DSL（§28 FlowDefinition）：
/// 把算子包的核心资产（需求 + 流程图 + 功能点）投影为与内核 FlowDefinition
/// 一致的规范结构，便于被编排层/执行层直接消费。
async fn export_package(
    State(_state): State<MarketState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let path = package_path(&id);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "success": false, "error": "算子包不存在" })),
            )
        }
    };
    let pkg: OperatorPackage = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "success": false, "error": format!("解析失败: {}", e) })),
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

fn save_package(pkg: &OperatorPackage) -> std::io::Result<()> {
    let dir = market_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    let content = serde_json::to_string_pretty(pkg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(package_path(&pkg.id), content)
}

/// 种子数据：用项目真实的"全业务流程"做示例算子包
async fn ensure_seed(state: &MarketState) {
    let dir = market_dir();
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
    };
    let _ = save_package(&seed);
    state.index.lock().await.insert(seed.id.clone(), seed.meta());
}
