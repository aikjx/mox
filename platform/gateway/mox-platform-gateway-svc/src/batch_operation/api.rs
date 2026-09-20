//! 批量操作 API 端点
//!
//! 提供：模板管理 / 通用导入导出 / 批量操作

use crate::batch_operation::*;
use crate::enterprise::api_response::*;
use axum::{
    extract::{Path, Query, State},
    response::Response,
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

// ==================== 模板管理 ====================

/// GET /api/enterprise/batch/templates —— 获取模板列表
pub async fn list_templates_handler(
    State(state): State<Arc<TemplateState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let templates = state.templates.read().await;
    let mut list: Vec<&ImportExportTemplate> = templates.values().collect();

    if let Some(module) = params.get("module") {
        list.retain(|t| t.module == *module);
    }
    if let Some(template_type) = params.get("template_type") {
        list.retain(|t| t.template_type == *template_type);
    }

    list.sort_by(|a, b| a.template_name.cmp(&b.template_name));
    success(list)
}

/// GET /api/enterprise/batch/templates/:id —— 获取模板详情
pub async fn get_template_handler(
    State(state): State<Arc<TemplateState>>,
    Path(id): Path<String>,
) -> Response {
    let templates = state.templates.read().await;
    match templates.get(&id) {
        Some(t) => success(t),
        None => not_found("模板不存在"),
    }
}

/// POST /api/enterprise/batch/templates —— 创建模板
pub async fn create_template_handler(
    State(state): State<Arc<TemplateState>>,
    Json(template): Json<ImportExportTemplate>,
) -> Response {
    let mut templates = state.templates.write().await;
    if templates.contains_key(&template.template_id) {
        return conflict(&format!("模板ID '{}' 已存在", template.template_id));
    }
    templates.insert(template.template_id.clone(), template.clone());
    success_with_message("模板创建成功", json!({ "template_id": template.template_id }))
}

/// PUT /api/enterprise/batch/templates/:id —— 更新模板
pub async fn update_template_handler(
    State(state): State<Arc<TemplateState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTemplateRequest>,
) -> Response {
    let mut templates = state.templates.write().await;
    match templates.get_mut(&id) {
        Some(t) => {
            if t.is_system {
                return forbidden("系统内置模板不可修改");
            }
            if let Some(template_name) = req.template_name { t.template_name = template_name; }
            if let Some(description) = req.description { t.description = Some(description); }
            if let Some(field_mapping) = req.field_mapping { t.field_mapping = field_mapping; }
            if let Some(default_values) = req.default_values { t.default_values = default_values; }
            if let Some(validation_rules) = req.validation_rules { t.validation_rules = validation_rules; }
            if let Some(export_fields) = req.export_fields { t.export_fields = Some(export_fields); }
            t.updated_at = chrono::Utc::now().to_rfc3339();
            success_message("模板更新成功")
        }
        None => not_found("模板不存在"),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    pub template_name: Option<String>,
    pub description: Option<String>,
    pub field_mapping: Option<HashMap<String, String>>,
    pub default_values: Option<HashMap<String, String>>,
    pub validation_rules: Option<HashMap<String, String>>,
    pub export_fields: Option<Vec<String>>,
}

/// DELETE /api/enterprise/batch/templates/:id —— 删除模板
pub async fn delete_template_handler(
    State(state): State<Arc<TemplateState>>,
    Path(id): Path<String>,
) -> Response {
    let mut templates = state.templates.write().await;
    match templates.get(&id) {
        Some(t) if t.is_system => forbidden("系统内置模板不可删除"),
        Some(_) => {
            templates.remove(&id);
            success_message("模板删除成功")
        }
        None => not_found("模板不存在"),
    }
}

// ==================== 通用导入导出 ====================

/// POST /api/enterprise/batch/import/preview —— 导入预览（解析数据但不写入）
pub async fn import_preview_handler(
    State(_state): State<Arc<TemplateState>>,
    Json(req): Json<ImportRequest>,
) -> Response {
    let skip_header = req.skip_header.unwrap_or(true);

    let rows = match req.format.as_str() {
        "csv" => match parse_csv(&req.data, skip_header) {
            Ok(rows) => rows,
            Err(e) => return bad_request(&format!("CSV解析失败: {}", e)),
        },
        "json" => match serde_json::from_str::<Vec<HashMap<String, String>>>(&req.data) {
            Ok(rows) => rows,
            Err(e) => return bad_request(&format!("JSON解析失败: {}", e)),
        },
        _ => return bad_request("不支持的导入格式，支持：csv / json"),
    };

    // 应用字段映射
    let mapped_rows: Vec<HashMap<String, String>> = if let Some(mapping) = &req.field_mapping {
        rows.iter().map(|r| apply_field_mapping(r, mapping)).collect()
    } else {
        rows
    };

    success(json!({
        "format": req.format,
        "total_rows": mapped_rows.len(),
        "fields": mapped_rows.first().map(|r| r.keys().cloned().collect::<Vec<_>>()).unwrap_or_default(),
        "sample_rows": mapped_rows.iter().take(5).cloned().collect::<Vec<_>>(),
        "mode": req.mode,
    }))
}

/// POST /api/enterprise/batch/export/template —— 导出模板（生成空的导入模板文件）
pub async fn export_template_file_handler(
    State(state): State<Arc<TemplateState>>,
    Json(req): Json<ExportTemplateRequest>,
) -> Response {
    let templates = state.templates.read().await;
    let template = req.template_id.as_ref().and_then(|id| templates.get(id));

    let fields: Vec<String> = if let Some(t) = template {
        if let Some(export_fields) = &t.export_fields {
            export_fields.clone()
        } else {
            t.field_mapping.keys().cloned().collect()
        }
    } else {
        req.fields.unwrap_or_else(|| vec!["field1".to_string(), "field2".to_string()])
    };

    let format = req.format.unwrap_or_else(|| "csv".to_string());
    let include_header = req.include_header.unwrap_or(true);

    match format.as_str() {
        "csv" => {
            let mut csv_content = String::new();
            if include_header {
                csv_content.push_str(&fields.join(","));
                csv_content.push('\n');
            }
            // 添加示例行
            let sample: Vec<String> = fields.iter().map(|f| format!("示例_{}", f)).collect();
            csv_content.push_str(&sample.join(","));
            csv_content.push('\n');

            success(json!({
                "format": "csv",
                "data": csv_content,
                "fields": fields,
                "filename": "import_template.csv",
                "content_type": "text/csv; charset=utf-8",
            }))
        }
        "json" => {
            let sample: HashMap<String, String> = fields.iter()
                .map(|f| (f.clone(), format!("示例_{}", f)))
                .collect();
            let json_content = serde_json::to_string_pretty(&vec![sample]).unwrap_or_default();

            success(json!({
                "format": "json",
                "data": json_content,
                "fields": fields,
                "filename": "import_template.json",
                "content_type": "application/json",
            }))
        }
        _ => bad_request("不支持的格式，支持：csv / json"),
    }
}

#[derive(Debug, Deserialize)]
pub struct ExportTemplateRequest {
    pub template_id: Option<String>,
    pub fields: Option<Vec<String>>,
    pub format: Option<String>,
    pub include_header: Option<bool>,
}

// ==================== 批量操作统计 ====================

/// GET /api/enterprise/batch/stats —— 批量操作统计
pub async fn batch_stats_handler(
    State(state): State<Arc<TemplateState>>,
) -> Response {
    let templates = state.templates.read().await;

    let mut templates_by_module: HashMap<String, i64> = HashMap::new();
    let mut templates_by_type: HashMap<String, i64> = HashMap::new();
    let mut system_templates = 0;
    let mut custom_templates = 0;

    for t in templates.values() {
        *templates_by_module.entry(t.module.clone()).or_insert(0) += 1;
        *templates_by_type.entry(t.template_type.clone()).or_insert(0) += 1;
        if t.is_system { system_templates += 1; } else { custom_templates += 1; }
    }

    success(json!({
        "total_templates": templates.len(),
        "system_templates": system_templates,
        "custom_templates": custom_templates,
        "templates_by_module": templates_by_module,
        "templates_by_type": templates_by_type,
        "supported_formats": ["csv", "json"],
        "supported_operations": ["create", "update", "delete", "upsert"],
    }))
}

/// 构建批量操作路由（泛型版本）
pub fn build_batch_operation_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<TemplateState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post};

    axum::Router::new()
        // 模板管理
        .route("/templates", get(list_templates_handler).post(create_template_handler))
        .route("/templates/:id", get(get_template_handler).put(update_template_handler).delete(delete_template_handler))
        // 通用导入导出
        .route("/import/preview", post(import_preview_handler))
        .route("/export/template", post(export_template_file_handler))
        // 统计
        .route("/stats", get(batch_stats_handler))
}
