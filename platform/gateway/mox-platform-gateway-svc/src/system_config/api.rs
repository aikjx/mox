//! 系统配置管理 API 端点
//!
//! 提供：配置项CRUD / 批量更新 / 配置分组 / 功能开关 / 配置版本 / 配置回滚 / 刷新缓存

use crate::system_config::*;
use crate::enterprise::api_response::*;
use axum::{
    extract::{Path, Query, State},
    response::Response,
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 系统配置状态
pub struct ConfigState {
    /// 配置项存储
    pub configs: Arc<RwLock<HashMap<String, ConfigItem>>>,
    /// 配置版本历史
    pub versions: Arc<RwLock<HashMap<String, Vec<ConfigVersion>>>>,
    /// 配置分组
    pub groups: Arc<RwLock<HashMap<String, ConfigGroup>>>,
    /// 功能开关
    pub feature_flags: Arc<RwLock<HashMap<String, FeatureFlag>>>,
    /// 配置缓存（运行时使用）
    pub cache: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl ConfigState {
    pub fn new() -> Self {
        let mut configs = HashMap::new();
        for cfg in builtin_system_configs() {
            configs.insert(cfg.config_key.clone(), cfg);
        }

        let mut groups = HashMap::new();
        for g in builtin_config_groups() {
            groups.insert(g.group_code.clone(), g);
        }

        let mut flags = HashMap::new();
        for f in builtin_feature_flags() {
            flags.insert(f.flag_key.clone(), f);
        }

        // 初始化缓存
        let mut cache = HashMap::new();
        for (key, cfg) in &configs {
            cache.insert(key.clone(), cfg.config_value.clone());
        }

        Self {
            configs: Arc::new(RwLock::new(configs)),
            versions: Arc::new(RwLock::new(HashMap::new())),
            groups: Arc::new(RwLock::new(groups)),
            feature_flags: Arc::new(RwLock::new(flags)),
            cache: Arc::new(RwLock::new(cache)),
        }
    }

    /// 从缓存获取配置值
    pub async fn get_config_value(&self, key: &str) -> Option<serde_json::Value> {
        self.cache.read().await.get(key).cloned()
    }

    /// 刷新配置缓存
    pub async fn refresh_cache(&self) {
        let configs = self.configs.read().await;
        let mut cache = self.cache.write().await;
        cache.clear();
        for (key, cfg) in configs.iter() {
            cache.insert(key.clone(), cfg.config_value.clone());
        }
    }
}

impl Default for ConfigState {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /api/enterprise/config/groups —— 获取配置分组列表
pub async fn list_config_groups_handler(
    State(state): State<Arc<ConfigState>>,
) -> Response {
    let groups = state.groups.read().await;
    let mut list: Vec<&ConfigGroup> = groups.values().collect();
    list.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
    success(list)
}

/// GET /api/enterprise/config/config-types —— 获取配置类型列表
pub async fn list_config_types_handler() -> Response {
    let types = vec![
        ConfigType::String, ConfigType::Number, ConfigType::Boolean,
        ConfigType::Json, ConfigType::Password, ConfigType::Select, ConfigType::MultiSelect,
    ];
    let result: Vec<serde_json::Value> = types.iter().map(|t| {
        json!({ "code": t.as_str(), "name": t.display_name() })
    }).collect();
    success(result)
}

/// GET /api/enterprise/config/items —— 获取配置项列表
pub async fn list_config_items_handler(
    State(state): State<Arc<ConfigState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let configs = state.configs.read().await;
    let mut list: Vec<&ConfigItem> = configs.values().collect();

    // 按分组过滤
    if let Some(group) = params.get("config_group") {
        list.retain(|c| c.config_group == *group);
    }
    // 按类型过滤
    if let Some(config_type) = params.get("config_type") {
        list.retain(|c| c.config_type.as_str() == *config_type);
    }
    // 按是否系统配置过滤
    if let Some(is_system) = params.get("is_system") {
        let sys = is_system == "true";
        list.retain(|c| c.is_system == sys);
    }
    // 关键词搜索
    if let Some(keyword) = params.get("keyword") {
        list.retain(|c| {
            c.config_key.contains(keyword) || c.config_name.contains(keyword)
        });
    }

    list.sort_by(|a, b| a.config_group.cmp(&b.config_group).then_with(|| a.config_key.cmp(&b.config_key)));

    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, list.len());
    let page_items = pagination.paginate(&list);
    success_list(page_items, &pagination)
}

/// POST /api/enterprise/config/items —— 创建配置项
pub async fn create_config_item_handler(
    State(state): State<Arc<ConfigState>>,
    Json(req): Json<CreateConfigItemRequest>,
) -> Response {
    // 输入验证
    if let Err(e) = validate_required("config_key", &req.config_key) {
        return e;
    }
    if let Err(e) = validate_required("config_name", &req.config_name) {
        return e;
    }
    if let Err(e) = validate_required("config_group", &req.config_group) {
        return e;
    }

    // 检查key是否已存在
    let configs = state.configs.read().await;
    if configs.contains_key(&req.config_key) {
        return conflict(&format!("配置键 '{}' 已存在", req.config_key));
    }
    drop(configs);

    let now = chrono::Utc::now().to_rfc3339();
    let item = ConfigItem {
        config_key: req.config_key.clone(),
        config_name: req.config_name,
        description: req.description,
        config_group: req.config_group,
        config_type: req.config_type,
        config_value: req.config_value,
        default_value: req.default_value,
        options: req.options,
        is_system: false,
        is_encrypted: req.is_encrypted.unwrap_or(false),
        is_editable: req.is_editable.unwrap_or(true),
        require_restart: req.require_restart.unwrap_or(false),
        validation_rules: req.validation_rules,
        version: 1,
        tenant_id: req.tenant_id,
        created_by: "system".to_string(),
        created_at: now.clone(),
        updated_by: None,
        updated_at: now,
    };

    state.configs.write().await.insert(req.config_key.clone(), item);
    state.refresh_cache().await;

    success_with_message("配置项创建成功", json!({ "config_key": req.config_key }))
}

/// GET /api/enterprise/config/items/:key —— 获取配置项详情
pub async fn get_config_item_handler(
    State(state): State<Arc<ConfigState>>,
    Path(key): Path<String>,
) -> Response {
    let configs = state.configs.read().await;
    match configs.get(&key) {
        Some(c) => success(c),
        None => not_found("配置项不存在"),
    }
}

/// PUT /api/enterprise/config/items/:key —— 更新配置项
pub async fn update_config_item_handler(
    State(state): State<Arc<ConfigState>>,
    Path(key): Path<String>,
    Json(req): Json<UpdateConfigItemRequest>,
) -> Response {
    let mut configs = state.configs.write().await;
    match configs.get_mut(&key) {
        Some(c) => {
            // 检查是否可修改
            if !c.is_editable {
                return forbidden("该配置项不可修改");
            }

            // 记录版本历史
            let old_value = c.config_value.clone();
            let version = ConfigVersion {
                version_id: format!("ver_{}", uuid::Uuid::new_v4().simple()),
                config_key: key.clone(),
                version: c.version,
                config_value: old_value.clone(),
                changed_by: "system".to_string(),
                changed_at: chrono::Utc::now().to_rfc3339(),
                change_note: req.change_note.clone(),
                old_value: Some(old_value),
                new_value: req.config_value.clone(),
            };
            state.versions.write().await
                .entry(key.clone())
                .or_insert_with(Vec::new)
                .push(version);

            // 更新配置
            if let Some(name) = req.config_name { c.config_name = name; }
            if let Some(desc) = req.description { c.description = Some(desc); }
            if let Some(value) = req.config_value { c.config_value = value; }
            if let Some(options) = req.options { c.options = Some(options); }
            if let Some(editable) = req.is_editable { c.is_editable = editable; }
            if let Some(require_restart) = req.require_restart { c.require_restart = require_restart; }
            if let Some(rules) = req.validation_rules { c.validation_rules = Some(rules); }
            c.version += 1;
            c.updated_by = Some("system".to_string());
            c.updated_at = chrono::Utc::now().to_rfc3339();

            drop(configs);
            state.refresh_cache().await;

            success_message("配置项更新成功")
        }
        None => not_found("配置项不存在"),
    }
}

/// DELETE /api/enterprise/config/items/:key —— 删除配置项
pub async fn delete_config_item_handler(
    State(state): State<Arc<ConfigState>>,
    Path(key): Path<String>,
) -> Response {
    let mut configs = state.configs.write().await;
    match configs.get(&key) {
        Some(c) if c.is_system => {
            forbidden("系统配置项不可删除")
        }
        Some(_) => {
            configs.remove(&key);
            drop(configs);
            state.refresh_cache().await;
            success_message("配置项删除成功")
        }
        None => not_found("配置项不存在"),
    }
}

/// POST /api/enterprise/config/batch-update —— 批量更新配置
pub async fn batch_update_config_handler(
    State(state): State<Arc<ConfigState>>,
    Json(req): Json<BatchUpdateConfigRequest>,
) -> Response {
    let mut configs = state.configs.write().await;
    let mut updated_count = 0;

    for item in &req.items {
        if let Some(c) = configs.get_mut(&item.config_key) {
            if c.is_editable {
                let old_value = c.config_value.clone();
                c.config_value = item.config_value.clone();
                c.version += 1;
                c.updated_at = chrono::Utc::now().to_rfc3339();

                // 记录版本
                let version = ConfigVersion {
                    version_id: format!("ver_{}", uuid::Uuid::new_v4().simple()),
                    config_key: item.config_key.clone(),
                    version: c.version,
                    config_value: old_value.clone(),
                    changed_by: "system".to_string(),
                    changed_at: chrono::Utc::now().to_rfc3339(),
                    change_note: req.change_note.clone(),
                    old_value: Some(old_value),
                    new_value: Some(item.config_value.clone()),
                };
                state.versions.write().await
                    .entry(item.config_key.clone())
                    .or_insert_with(Vec::new)
                    .push(version);

                updated_count += 1;
            }
        }
    }

    drop(configs);
    state.refresh_cache().await;

    success_with_message(&format!("批量更新成功，共更新 {} 个配置项", updated_count), json!({ "updated_count": updated_count }))
}

/// GET /api/enterprise/config/items/:key/versions —— 获取配置版本历史
pub async fn list_config_versions_handler(
    State(state): State<Arc<ConfigState>>,
    Path(key): Path<String>,
) -> Response {
    let versions = state.versions.read().await;
    match versions.get(&key) {
        Some(v) => {
            let mut list = v.clone();
            list.sort_by(|a, b| b.changed_at.cmp(&a.changed_at));
            success_with_message("success", json!(list))
        }
        None => success_with_message("success", json!([])),
    }
}

/// POST /api/enterprise/config/items/:key/rollback/:version_id —— 回滚到指定版本
pub async fn rollback_config_handler(
    State(state): State<Arc<ConfigState>>,
    Path((key, version_id)): Path<(String, String)>,
) -> Response {
    let versions = state.versions.read().await;
    let target_version = versions.get(&key)
        .and_then(|v| v.iter().find(|ver| ver.version_id == version_id))
        .cloned();
    drop(versions);

    match target_version {
        Some(ver) => {
            let mut configs = state.configs.write().await;
            if let Some(c) = configs.get_mut(&key) {
                let old_value = c.config_value.clone();
                c.config_value = ver.config_value.clone();
                c.version += 1;
                c.updated_at = chrono::Utc::now().to_rfc3339();

                // 记录回滚版本
                let rollback_ver = ConfigVersion {
                    version_id: format!("ver_{}", uuid::Uuid::new_v4().simple()),
                    config_key: key.clone(),
                    version: c.version,
                    config_value: old_value.clone(),
                    changed_by: "system".to_string(),
                    changed_at: chrono::Utc::now().to_rfc3339(),
                    change_note: Some(format!("回滚到版本 {}", version_id)),
                    old_value: Some(old_value),
                    new_value: Some(ver.config_value),
                };
                state.versions.write().await
                    .entry(key.clone())
                    .or_insert_with(Vec::new)
                    .push(rollback_ver);

                drop(configs);
                state.refresh_cache().await;
                success_message("配置回滚成功")
            } else {
                not_found("配置项不存在")
            }
        }
        None => not_found("版本不存在"),
    }
}

/// POST /api/enterprise/config/refresh-cache —— 刷新配置缓存
pub async fn refresh_config_cache_handler(
    State(state): State<Arc<ConfigState>>,
) -> Response {
    state.refresh_cache().await;
    success_message("配置缓存刷新成功")
}

// ==================== 功能开关 ====================

/// GET /api/enterprise/config/feature-flags —— 获取功能开关列表
pub async fn list_feature_flags_handler(
    State(state): State<Arc<ConfigState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let flags = state.feature_flags.read().await;
    let mut list: Vec<&FeatureFlag> = flags.values().collect();

    if let Some(enabled) = params.get("enabled") {
        let en = enabled == "true";
        list.retain(|f| f.enabled == en);
    }

    list.sort_by(|a, b| a.flag_key.cmp(&b.flag_key));
    success_with_message("success", json!(list))
}

/// POST /api/enterprise/config/feature-flags —— 创建功能开关
pub async fn create_feature_flag_handler(
    State(state): State<Arc<ConfigState>>,
    Json(req): Json<CreateFeatureFlagRequest>,
) -> Response {
    if let Err(e) = validate_required("flag_key", &req.flag_key) {
        return e;
    }
    if let Err(e) = validate_required("flag_name", &req.flag_name) {
        return e;
    }

    let flags = state.feature_flags.read().await;
    if flags.contains_key(&req.flag_key) {
        return conflict(&format!("功能开关 '{}' 已存在", req.flag_key));
    }
    drop(flags);

    let now = chrono::Utc::now().to_rfc3339();
    let flag = FeatureFlag {
        flag_key: req.flag_key.clone(),
        flag_name: req.flag_name,
        description: req.description,
        enabled: req.enabled.unwrap_or(false),
        rollout_percentage: req.rollout_percentage,
        whitelist_users: req.whitelist_users,
        whitelist_tenants: req.whitelist_tenants,
        expire_at: req.expire_at,
        is_system: false,
        created_at: now.clone(),
        updated_at: now,
    };

    state.feature_flags.write().await.insert(req.flag_key.clone(), flag);
    success_with_message("功能开关创建成功", json!({ "flag_key": req.flag_key }))
}

/// POST /api/enterprise/config/feature-flags/:key/toggle —— 切换功能开关
pub async fn toggle_feature_flag_handler(
    State(state): State<Arc<ConfigState>>,
    Path(key): Path<String>,
) -> Response {
    let mut flags = state.feature_flags.write().await;
    match flags.get_mut(&key) {
        Some(f) => {
            f.enabled = !f.enabled;
            f.updated_at = chrono::Utc::now().to_rfc3339();
            success_with_message(
                &format!("功能开关已{}", if f.enabled { "启用" } else { "禁用" }),
                json!({ "flag_key": key, "enabled": f.enabled })
            )
        }
        None => not_found("功能开关不存在"),
    }
}

/// GET /api/enterprise/config/stats —— 获取配置统计
pub async fn config_stats_handler(
    State(state): State<Arc<ConfigState>>,
) -> Response {
    let configs = state.configs.read().await;
    let flags = state.feature_flags.read().await;
    let groups = state.groups.read().await;

    let mut stats = ConfigStats::default();
    stats.total_configs = configs.len() as i64;
    stats.system_configs = configs.values().filter(|c| c.is_system).count() as i64;
    stats.custom_configs = configs.values().filter(|c| !c.is_system).count() as i64;
    stats.total_groups = groups.len() as i64;
    stats.total_flags = flags.len() as i64;
    stats.enabled_flags = flags.values().filter(|f| f.enabled).count() as i64;
    stats.disabled_flags = flags.values().filter(|f| !f.enabled).count() as i64;
    stats.require_restart_count = configs.values().filter(|c| c.require_restart).count() as i64;

    success(stats)
}

/// 构建系统配置管理路由（泛型版本）
pub fn build_config_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<ConfigState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post, put, delete};

    axum::Router::new()
        // 配置分组和类型
        .route("/groups", get(list_config_groups_handler))
        .route("/config-types", get(list_config_types_handler))
        // 配置项CRUD
        .route("/items", get(list_config_items_handler).post(create_config_item_handler))
        .route("/items/:key", get(get_config_item_handler).put(update_config_item_handler).delete(delete_config_item_handler))
        // 批量更新
        .route("/batch-update", post(batch_update_config_handler))
        // 版本管理
        .route("/items/:key/versions", get(list_config_versions_handler))
        .route("/items/:key/rollback/:version_id", post(rollback_config_handler))
        // 缓存
        .route("/refresh-cache", post(refresh_config_cache_handler))
        // 功能开关
        .route("/feature-flags", get(list_feature_flags_handler).post(create_feature_flag_handler))
        .route("/feature-flags/:key/toggle", post(toggle_feature_flag_handler))
        // 统计
        .route("/stats", get(config_stats_handler))
}
