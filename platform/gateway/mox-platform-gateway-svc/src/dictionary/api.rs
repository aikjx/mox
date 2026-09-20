//! 数据字典管理 API 端点

use crate::dictionary::*;
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

/// 数据字典状态
pub struct DictionaryState {
    pub dict_types: Arc<RwLock<HashMap<String, DictType>>>,
    pub dict_items: Arc<RwLock<HashMap<String, Vec<DictItem>>>>,
    pub cache: Arc<RwLock<HashMap<String, Vec<DictItem>>>>,
}

impl DictionaryState {
    pub fn new() -> Self {
        let mut dict_types = HashMap::new();
        for dt in builtin_dict_types() {
            dict_types.insert(dt.dict_type.clone(), dt);
        }

        let mut dict_items: HashMap<String, Vec<DictItem>> = HashMap::new();
        for item in builtin_dict_items() {
            dict_items.entry(item.dict_type.clone()).or_default().push(item);
        }

        Self {
            dict_types: Arc::new(RwLock::new(dict_types)),
            dict_items: Arc::new(RwLock::new(dict_items)),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn refresh_cache(&self) {
        let items = self.dict_items.read().await;
        let mut cache = self.cache.write().await;
        cache.clear();
        for (k, v) in items.iter() {
            cache.insert(k.clone(), v.clone());
        }
    }
}

impl Default for DictionaryState {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /api/enterprise/dictionary/types —— 获取字典类型列表
pub async fn list_dict_types_handler(
    State(state): State<Arc<DictionaryState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let types = state.dict_types.read().await;
    let mut list: Vec<&DictType> = types.values().collect();
    if let Some(status) = params.get("status") {
        list.retain(|t| t.status == *status);
    }
    if let Some(keyword) = params.get("keyword") {
        list.retain(|t| t.dict_name.contains(keyword) || t.dict_type.contains(keyword));
    }
    list.sort_by(|a, b| a.dict_type.cmp(&b.dict_type));
    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, list.len());
    let page_items = pagination.paginate(&list);
    success_list(page_items, &pagination)
}

/// POST /api/enterprise/dictionary/types —— 创建字典类型
pub async fn create_dict_type_handler(
    State(state): State<Arc<DictionaryState>>,
    Json(req): Json<CreateDictTypeRequest>,
) -> Response {
    if let Err(e) = validate_required("dict_name", &req.dict_name) { return e; }
    if let Err(e) = validate_required("dict_type", &req.dict_type) { return e; }

    let types = state.dict_types.read().await;
    if types.contains_key(&req.dict_type) {
        return conflict(&format!("字典类型 '{}' 已存在", req.dict_type));
    }
    drop(types);

    let now = chrono::Utc::now().to_rfc3339();
    let dt = DictType {
        dict_id: format!("dict_{}", uuid::Uuid::new_v4().simple()),
        dict_name: req.dict_name,
        dict_type: req.dict_type.clone(),
        status: req.status.unwrap_or_else(|| "0".to_string()),
        is_system: false,
        remark: req.remark,
        created_by: "system".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };
    state.dict_types.write().await.insert(req.dict_type.clone(), dt);
    success_with_message("字典类型创建成功", json!({ "dict_type": req.dict_type }))
}

/// DELETE /api/enterprise/dictionary/types/:dict_type —— 删除字典类型
pub async fn delete_dict_type_handler(
    State(state): State<Arc<DictionaryState>>,
    Path(dict_type): Path<String>,
) -> Response {
    let mut types = state.dict_types.write().await;
    match types.get(&dict_type) {
        Some(t) if t.is_system => forbidden("系统字典类型不可删除"),
        Some(_) => {
            types.remove(&dict_type);
            state.dict_items.write().await.remove(&dict_type);
            success_message("字典类型删除成功")
        }
        None => not_found("字典类型不存在"),
    }
}

/// GET /api/enterprise/dictionary/items/:dict_type —— 获取字典项列表
pub async fn list_dict_items_handler(
    State(state): State<Arc<DictionaryState>>,
    Path(dict_type): Path<String>,
) -> Response {
    let items = state.dict_items.read().await;
    match items.get(&dict_type) {
        Some(list) => {
            let mut sorted = list.clone();
            sorted.sort_by_key(|a| a.sort_order);
            success_with_message("success", json!(sorted))
        }
        None => success_with_message("success", json!([])),
    }
}

/// POST /api/enterprise/dictionary/items —— 创建字典项
pub async fn create_dict_item_handler(
    State(state): State<Arc<DictionaryState>>,
    Json(req): Json<CreateDictItemRequest>,
) -> Response {
    if let Err(e) = validate_required("item_value", &req.item_value) { return e; }
    if let Err(e) = validate_required("item_label", &req.item_label) { return e; }
    if let Err(e) = validate_required("dict_type", &req.dict_type) { return e; }

    let now = chrono::Utc::now().to_rfc3339();
    let item = DictItem {
        item_code: req.item_value.clone(),
        item_value: req.item_value,
        item_label: req.item_label,
        dict_type: req.dict_type.clone(),
        parent_code: req.parent_code,
        sort_order: req.sort_order.unwrap_or(0),
        css_class: req.css_class,
        list_class: req.list_class,
        is_default: req.is_default.unwrap_or(false),
        status: req.status.unwrap_or_else(|| "0".to_string()),
        remark: req.remark,
        created_at: now.clone(),
        updated_at: now,
    };

    let mut items = state.dict_items.write().await;
    items.entry(req.dict_type.clone()).or_insert_with(Vec::new).push(item);
    drop(items);
    state.refresh_cache().await;

    success_message("字典项创建成功")
}

/// DELETE /api/enterprise/dictionary/items/:dict_type/:item_value —— 删除字典项
pub async fn delete_dict_item_handler(
    State(state): State<Arc<DictionaryState>>,
    Path((dict_type, item_value)): Path<(String, String)>,
) -> Response {
    let mut items = state.dict_items.write().await;
    if let Some(list) = items.get_mut(&dict_type) {
        let before = list.len();
        list.retain(|i| i.item_value != item_value);
        if list.len() < before {
            drop(items);
            state.refresh_cache().await;
            success_message("字典项删除成功")
        } else {
            not_found("字典项不存在")
        }
    } else {
        not_found("字典类型不存在")
    }
}

/// GET /api/enterprise/dictionary/all —— 获取所有字典（前端初始化用）
pub async fn get_all_dicts_handler(
    State(state): State<Arc<DictionaryState>>,
) -> Response {
    let items = state.dict_items.read().await;
    let mut result = HashMap::new();
    for (k, v) in items.iter() {
        let mut sorted = v.clone();
        sorted.sort_by_key(|a| a.sort_order);
        result.insert(k, sorted);
    }
    success(result)
}

/// 构建数据字典路由（泛型版本）
pub fn build_dictionary_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<DictionaryState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post, delete};

    axum::Router::new()
        .route("/types", get(list_dict_types_handler).post(create_dict_type_handler))
        .route("/types/:dict_type", delete(delete_dict_type_handler))
        .route("/items", post(create_dict_item_handler))
        .route("/items/:dict_type", get(list_dict_items_handler))
        .route("/items/:dict_type/:item_value", delete(delete_dict_item_handler))
        .route("/all", get(get_all_dicts_handler))
}
