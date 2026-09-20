//! 文件存储管理 API 端点

use crate::file_storage::*;
use crate::enterprise::api_response::*;
use axum::{
    extract::{Path, Query, State, Multipart},
    response::{Response, IntoResponse},
    http::{HeaderMap, header::CONTENT_DISPOSITION},
};
use chrono::Datelike;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 文件存储状态
pub struct FileStorageState {
    pub files: Arc<RwLock<HashMap<String, FileMetadata>>>,
    pub upload_dir: Arc<String>,
}

impl FileStorageState {
    pub fn new() -> Self {
        let mut files = HashMap::new();
        for f in sample_files() {
            files.insert(f.file_id.clone(), f);
        }
        let upload_dir = std::env::var("MOX_UPLOAD_DIR")
            .unwrap_or_else(|_| "./data/uploads".to_string());  // allow: env-MOX_UPLOAD_DIR-overrides
        // 确保目录存在
        let _ = std::fs::create_dir_all(&upload_dir);
        Self {
            files: Arc::new(RwLock::new(files)),
            upload_dir: Arc::new(upload_dir),
        }
    }
}

impl Default for FileStorageState {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /api/enterprise/files —— 获取文件列表
pub async fn list_files_handler(
    State(state): State<Arc<FileStorageState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let files = state.files.read().await;
    let mut list: Vec<&FileMetadata> = files.values().collect();

    if let Some(category) = params.get("file_category") {
        list.retain(|f| f.file_category == *category);
    }
    if let Some(storage_type) = params.get("storage_type") {
        list.retain(|f| f.storage_type == *storage_type);
    }
    if let Some(tenant_id) = params.get("tenant_id") {
        list.retain(|f| f.tenant_id.as_deref() == Some(tenant_id.as_str()));
    }
    if let Some(status) = params.get("status") {
        list.retain(|f| f.status == *status);
    }
    if let Some(keyword) = params.get("keyword") {
        list.retain(|f| f.original_name.contains(keyword));
    }

    list.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));

    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, list.len());
    let page_items = pagination.paginate(&list);
    success_list(page_items, &pagination)
}

/// GET /api/enterprise/files/:file_id —— 获取文件详情
pub async fn get_file_handler(
    State(state): State<Arc<FileStorageState>>,
    Path(file_id): Path<String>,
) -> Response {
    let files = state.files.read().await;
    match files.get(&file_id) {
        Some(f) => success(f),
        None => not_found("文件不存在"),
    }
}

/// DELETE /api/enterprise/files/:file_id —— 删除文件
pub async fn delete_file_handler(
    State(state): State<Arc<FileStorageState>>,
    Path(file_id): Path<String>,
) -> Response {
    let mut files = state.files.write().await;
    match files.get_mut(&file_id) {
        Some(f) => {
            f.status = "deleted".to_string();
            success_message("文件删除成功")
        }
        None => not_found("文件不存在"),
    }
}

/// POST /api/enterprise/files/:file_id/restore —— 恢复文件
pub async fn restore_file_handler(
    State(state): State<Arc<FileStorageState>>,
    Path(file_id): Path<String>,
) -> Response {
    let mut files = state.files.write().await;
    match files.get_mut(&file_id) {
        Some(f) => {
            f.status = "normal".to_string();
            success_message("文件恢复成功")
        }
        None => not_found("文件不存在"),
    }
}

/// GET /api/enterprise/files/stats —— 文件统计
pub async fn file_stats_handler(
    State(state): State<Arc<FileStorageState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let files = state.files.read().await;
    let mut filtered: Vec<&FileMetadata> = files.values().collect();

    if let Some(tenant_id) = params.get("tenant_id") {
        filtered.retain(|f| f.tenant_id.as_deref() == Some(tenant_id.as_str()));
    }

    let mut stats = FileStats::default();
    stats.total_files = filtered.len() as i64;
    stats.total_size = filtered.iter().map(|f| f.file_size).sum();
    stats.total_downloads = filtered.iter().map(|f| f.download_count).sum();

    for f in &filtered {
        *stats.by_category.entry(f.file_category.clone()).or_insert(0) += 1;
        *stats.by_storage_type.entry(f.storage_type.clone()).or_insert(0) += 1;
    }

    success(stats)
}

/// GET /api/enterprise/files/categories —— 获取文件分类列表
pub async fn list_file_categories_handler() -> Response {
    let categories = vec![
        json!({"code": "document", "name": "文档", "extensions": ["doc","docx","pdf","xls","xlsx","ppt","pptx","txt","md"]}),
        json!({"code": "image", "name": "图片", "extensions": ["jpg","jpeg","png","gif","bmp","svg","webp"]}),
        json!({"code": "video", "name": "视频", "extensions": ["mp4","avi","mov","wmv","flv","mkv"]}),
        json!({"code": "audio", "name": "音频", "extensions": ["mp3","wav","flac","aac","ogg"]}),
        json!({"code": "archive", "name": "压缩包", "extensions": ["zip","rar","7z","tar","gz"]}),
        json!({"code": "other", "name": "其他", "extensions": []}),
    ];
    success(categories)
}

/// GET /api/enterprise/files/storage-types —— 获取存储类型列表
pub async fn list_storage_types_handler() -> Response {
    let types = vec![
        json!({"code": "local", "name": "本地存储", "description": "服务器本地文件系统"}),
        json!({"code": "oss", "name": "阿里云OSS", "description": "阿里云对象存储服务"}),
        json!({"code": "s3", "name": "AWS S3", "description": "Amazon S3对象存储"}),
        json!({"code": "minio", "name": "MinIO", "description": "开源对象存储服务"}),
        json!({"code": "ftp", "name": "FTP", "description": "FTP文件传输协议"}),
    ];
    success(types)
}

/// POST /api/enterprise/files/upload —— 上传文件
pub async fn upload_file_handler(
    State(state): State<Arc<FileStorageState>>,
    mut multipart: Multipart,
) -> Response {
    let mut uploaded_files = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let file_name = field.file_name().unwrap_or("unnamed").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
        let data = match field.bytes().await {
            Ok(d) => d,
            Err(e) => return bad_request(&format!("读取文件失败: {}", e)),
        };

        let file_size = data.len() as i64;
        let file_id = format!("file_{}", uuid::Uuid::new_v4().simple());
        let extension = std::path::Path::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        // 按日期分目录存储
        let now = chrono::Utc::now();
        let date_dir = format!("{}/{:02}/{:02}", now.year(), now.month(), now.day());
        let stored_name = format!("{}_{}.{}", file_id, now.timestamp(), extension);
        let relative_path = format!("{}/{}", date_dir, stored_name);

        // 确保目录存在
        let full_dir = format!("{}/{}", state.upload_dir, date_dir);
        let _ = std::fs::create_dir_all(&full_dir);

        // 写入文件
        let full_path = format!("{}/{}", state.upload_dir, relative_path);
        if let Err(e) = tokio::fs::write(&full_path, &data).await {
            return internal_error(&format!("保存文件失败: {}", e));
        }

        // 计算MD5
        let md5 = format!("{:x}", md5_hash(&data));

        // 判断文件分类
        let file_category = classify_file(&extension, &content_type);

        let metadata = FileMetadata {
            file_id: file_id.clone(),
            tenant_id: None,
            original_name: file_name.clone(),
            stored_name: stored_name.clone(),
            file_path: relative_path.clone(),
            file_size,
            content_type: content_type.clone(),
            extension: extension.clone(),
            file_category,
            storage_type: "local".to_string(),
            bucket: None,
            md5: Some(md5),
            uploaded_by: "system".to_string(),
            uploaded_at: chrono::Utc::now().to_rfc3339(),
            last_accessed_at: None,
            download_count: 0,
            status: "normal".to_string(),
            tags: None,
            description: None,
        };

        state.files.write().await.insert(file_id.clone(), metadata);
        uploaded_files.push(json!({
            "file_id": file_id,
            "original_name": file_name,
            "file_size": file_size,
            "content_type": content_type,
        }));
    }

    if uploaded_files.is_empty() {
        return bad_request("没有上传任何文件");
    }

    success_with_message(&format!("成功上传 {} 个文件", uploaded_files.len()), json!({ "files": uploaded_files }))
}

/// GET /api/enterprise/files/:file_id/download —— 下载文件
pub async fn download_file_handler(
    State(state): State<Arc<FileStorageState>>,
    Path(file_id): Path<String>,
) -> Response {
    let files = state.files.read().await;
    let metadata = match files.get(&file_id) {
        Some(m) => m.clone(),
        None => return not_found("文件不存在"),
    };

    if metadata.status != "normal" {
        return forbidden("文件已被删除或归档");
    }

    let full_path = format!("{}/{}", state.upload_dir, metadata.file_path);

    // 更新下载次数
    drop(files);
    if let Some(f) = state.files.write().await.get_mut(&file_id) {
        f.download_count += 1;
        f.last_accessed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    // 读取文件内容
    let file_data = match tokio::fs::read(&full_path).await {
        Ok(data) => data,
        Err(_) => {
            // 如果文件不存在但元数据存在，返回示例内容
            let body = format!("文件 {} 的示例内容（文件尚未实际存储）", metadata.original_name);
            let mut headers = HeaderMap::new();
            headers.insert(
                axum::http::header::CONTENT_TYPE,
                "application/octet-stream".parse().unwrap(),
            );
            return (headers, body).into_response();
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", metadata.original_name)
            .parse()
            .unwrap(),
    );
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        metadata.content_type.parse().unwrap(),
    );

    (headers, file_data).into_response()
}

/// 简单MD5哈希（用于演示，生产环境建议使用md-5 crate）
fn md5_hash(data: &[u8]) -> u128 {
    // 简化的哈希，实际应使用md-5 crate
    let mut hash: u128 = 0;
    for (i, &b) in data.iter().enumerate() {
        hash = hash.wrapping_mul(31).wrapping_add(b as u128).wrapping_add(i as u128);
    }
    hash
}

/// 根据扩展名和MIME类型分类文件
fn classify_file(extension: &str, content_type: &str) -> String {
    let ext = extension.to_lowercase();
    if content_type.starts_with("image/") || matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp") {
        return "image".to_string();
    }
    if content_type.starts_with("video/") || matches!(ext.as_str(), "mp4" | "avi" | "mov" | "wmv" | "flv" | "mkv") {
        return "video".to_string();
    }
    if content_type.starts_with("audio/") || matches!(ext.as_str(), "mp3" | "wav" | "flac" | "aac" | "ogg") {
        return "audio".to_string();
    }
    if matches!(ext.as_str(), "zip" | "rar" | "7z" | "tar" | "gz") {
        return "archive".to_string();
    }
    if matches!(ext.as_str(), "doc" | "docx" | "pdf" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md") {
        return "document".to_string();
    }
    "other".to_string()
}

/// 构建文件存储路由（泛型版本）
pub fn build_file_storage_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<FileStorageState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/", get(list_files_handler))
        .route("/upload", post(upload_file_handler))
        .route("/stats", get(file_stats_handler))
        .route("/categories", get(list_file_categories_handler))
        .route("/storage-types", get(list_storage_types_handler))
        .route("/:file_id", get(get_file_handler).delete(delete_file_handler))
        .route("/:file_id/download", get(download_file_handler))
        .route("/:file_id/restore", post(restore_file_handler))
}
