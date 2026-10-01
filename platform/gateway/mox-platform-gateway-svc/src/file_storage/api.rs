//! 文件主源 API：可信租户/所有者授权、持久化元数据与真实内容校验。
use super::{
    repository::{FileAccess, FileRepository},
    FileMetadata, FileStats,
};
use crate::{auth::ApiAuth, enterprise::api_response::*};
use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
};
use mox_platform_api::UserInfo;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Component, Path as FsPath, PathBuf},
    sync::Arc,
};

const UPLOAD_LIMIT: usize = 16 * 1024 * 1024;

pub struct FileStorageState {
    repository: FileRepository,
    pub upload_dir: Arc<String>,
}
impl FileStorageState {
    pub fn open(
        root: impl AsRef<FsPath>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(&root)?;
        let root = root.as_ref().canonicalize()?;
        Ok(Self {
            repository: FileRepository::open(&root.join("files.sqlite3"))?,
            upload_dir: Arc::new(root.to_string_lossy().into_owned()),
        })
    }
    pub fn new() -> Self {
        let root = std::env::var("MOX_UPLOAD_DIR").unwrap_or_else(|_| "./data/uploads".into()); // allow: env-MOX_UPLOAD_DIR-overrides
        Self::open(root).expect("file metadata storage initialization failed")
    }
    fn path(&self, relative: &str) -> std::io::Result<PathBuf> {
        let path = FsPath::new(relative);
        if path.components().any(|part| !matches!(part, Component::Normal(_))) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid file location",
            ));
        }
        let resolved = FsPath::new(self.upload_dir.as_str()).join(path).canonicalize()?;
        if !resolved.starts_with(self.upload_dir.as_str()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file location outside storage root",
            ));
        }
        Ok(resolved)
    }
}
impl Default for FileStorageState {
    fn default() -> Self {
        Self::new()
    }
}

fn access(user: &UserInfo, write: bool) -> Result<FileAccess, &'static str> {
    if !user.enabled || user.id.trim().is_empty() || user.tenant_id.trim().is_empty() {
        return Err("身份或租户不可用");
    }
    if write && user.roles.iter().any(|role| role == "readonly_auditor") {
        return Err("只读身份不能修改文件");
    }
    Ok(user.into())
}
fn storage_error(err: impl std::fmt::Display) -> Response {
    tracing::error!(error = %err, "file storage operation failed");
    internal_error("文件存储操作失败")
}

pub async fn list_files_handler(
    State(state): State<Arc<FileStorageState>>,
    ApiAuth(user): ApiAuth,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let scope = match access(&user, false) {
        Ok(scope) => scope,
        Err(message) => return forbidden(message),
    };
    if params.get("tenant_id").is_some_and(|tenant| tenant != &scope.tenant) {
        return forbidden("不能切换租户");
    }
    let mut files = match state.repository.list(&scope) {
        Ok(files) => files,
        Err(err) => return storage_error(err),
    };
    for key in ["file_category", "storage_type", "status", "keyword"] {
        if let Some(value) = params.get(key) {
            files.retain(|file| match key {
                "file_category" => &file.file_category == value,
                "storage_type" => &file.storage_type == value,
                "status" => &file.status == value,
                _ => file.original_name.contains(value),
            });
        }
    }
    files.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at).then_with(|| a.file_id.cmp(&b.file_id)));
    let (page, size) = parse_pagination(&params);
    // 限制页号，避免共享分页器的偏移乘法溢出。
    let pagination = Pagination::new(page.min(files.len().saturating_add(1)), size, files.len());
    success_list(pagination.paginate(&files), &pagination)
}
pub async fn get_file_handler(
    State(state): State<Arc<FileStorageState>>,
    ApiAuth(user): ApiAuth,
    Path(id): Path<String>,
) -> Response {
    let scope = match access(&user, false) {
        Ok(scope) => scope,
        Err(message) => return forbidden(message),
    };
    match state.repository.get(&scope, &id) {
        Ok(Some(file)) => success(file),
        Ok(None) => not_found("文件不存在"),
        Err(err) => storage_error(err),
    }
}
async fn change_status(
    state: Arc<FileStorageState>,
    user: UserInfo,
    id: String,
    status: &str,
) -> Response {
    let scope = match access(&user, true) {
        Ok(scope) => scope,
        Err(message) => return forbidden(message),
    };
    match state.repository.change_status(&scope, &id, status) {
        Ok(true) => success_message("文件状态已更新"),
        Ok(false) => not_found("文件不存在"),
        Err(err) => storage_error(err),
    }
}
pub async fn delete_file_handler(
    State(state): State<Arc<FileStorageState>>,
    ApiAuth(user): ApiAuth,
    Path(id): Path<String>,
) -> Response {
    change_status(state, user, id, "deleted").await
}
pub async fn restore_file_handler(
    State(state): State<Arc<FileStorageState>>,
    ApiAuth(user): ApiAuth,
    Path(id): Path<String>,
) -> Response {
    change_status(state, user, id, "normal").await
}
pub async fn file_stats_handler(
    State(state): State<Arc<FileStorageState>>,
    ApiAuth(user): ApiAuth,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let scope = match access(&user, false) {
        Ok(scope) => scope,
        Err(message) => return forbidden(message),
    };
    if params.get("tenant_id").is_some_and(|tenant| tenant != &scope.tenant) {
        return forbidden("不能切换租户");
    }
    let files = match state.repository.list(&scope) {
        Ok(files) => files,
        Err(err) => return storage_error(err),
    };
    let mut stats = FileStats { total_files: files.len() as i64, ..Default::default() };
    let today = chrono::Utc::now().date_naive().to_string();
    for file in files {
        stats.total_size += file.file_size;
        stats.total_downloads += file.download_count;
        stats.today_uploads += i64::from(file.uploaded_at.starts_with(&today));
        *stats.by_category.entry(file.file_category).or_default() += 1;
        *stats.by_storage_type.entry(file.storage_type).or_default() += 1;
    }
    success(stats)
}

pub async fn upload_file_handler(
    State(state): State<Arc<FileStorageState>>,
    ApiAuth(user): ApiAuth,
    mut multipart: Multipart,
) -> Response {
    if let Err(message) = access(&user, true) {
        return forbidden(message);
    }
    // 先完整验证有界请求；批量元数据事务提交前不发布任何文件。
    let mut pending = Vec::new();
    let mut total = 0usize;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(_) => return bad_request("上传请求格式错误"),
        };
        let Some(name) = field.file_name().map(str::to_owned) else { continue };
        if name.is_empty() || name.len() > 255 || name.chars().any(char::is_control) {
            return bad_request("文件名无效");
        }
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_owned();
        let bytes = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(_) => return bad_request("文件内容无效或超过大小限制"),
        };
        total = total.saturating_add(bytes.len());
        if total > UPLOAD_LIMIT || pending.len() >= 100 {
            return bad_request("上传超过大小或数量限制");
        }
        pending.push((name, content_type, bytes));
    }
    if pending.is_empty() {
        return bad_request("没有上传任何文件");
    }
    let mut files = Vec::new();
    let mut paths = Vec::new();
    for (name, content_type, bytes) in pending {
        let id = format!("file_{}", uuid::Uuid::new_v4().simple());
        let extension = FsPath::new(&name)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_lowercase();
        let stored_name = id.clone();
        let path = FsPath::new(state.upload_dir.as_str()).join(&stored_name);
        // 唯一生成的存储名不含用户路径，create_new 避免覆盖已存在对象。
        use tokio::io::AsyncWriteExt;
        let write = async {
            let mut file =
                tokio::fs::OpenOptions::new().write(true).create_new(true).open(&path).await?;
            paths.push(path.clone());
            file.write_all(&bytes).await?;
            file.sync_all().await
        }
        .await;
        if let Err(err) = write {
            cleanup(&paths).await;
            return storage_error(err);
        }
        files.push(FileMetadata {
            file_id: id,
            tenant_id: Some(user.tenant_id.clone()),
            original_name: name,
            stored_name: stored_name.clone(),
            file_path: stored_name,
            file_size: bytes.len() as i64,
            file_category: classify_file(&extension, &content_type),
            content_type,
            extension,
            storage_type: "local".into(),
            bucket: None,
            md5: None,
            sha256: Some(hex::encode(Sha256::digest(&bytes))),
            uploaded_by: user.id.clone(),
            uploaded_at: chrono::Utc::now().to_rfc3339(),
            last_accessed_at: None,
            download_count: 0,
            status: "normal".into(),
            tags: None,
            description: None,
        });
    }
    if let Err(err) = state.repository.insert_many(&files) {
        cleanup(&paths).await;
        return storage_error(err);
    }
    success_with_message(&format!("成功上传 {} 个文件", files.len()), json!({"files":files}))
}
async fn cleanup(paths: &[PathBuf]) {
    for path in paths {
        if let Err(err) = tokio::fs::remove_file(path).await {
            tracing::error!(error=%err,"failed to clean uncommitted upload");
        }
    }
}

pub async fn download_file_handler(
    State(state): State<Arc<FileStorageState>>,
    ApiAuth(user): ApiAuth,
    Path(id): Path<String>,
) -> Response {
    let scope = match access(&user, false) {
        Ok(scope) => scope,
        Err(message) => return forbidden(message),
    };
    let file = match state.repository.get(&scope, &id) {
        Ok(Some(file)) => file,
        Ok(None) => return not_found("文件不存在"),
        Err(err) => return storage_error(err),
    };
    if file.status != "normal" {
        return forbidden("文件已删除或归档");
    }
    let path = match state.path(&file.file_path) {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return not_found("文件内容不存在")
        },
        Err(error) => return storage_error(error),
    };
    let data = match tokio::fs::read(path).await {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return not_found("文件内容不存在")
        },
        Err(err) => return storage_error(err),
    };
    if data.len() as i64 != file.file_size
        || file
            .sha256
            .as_ref()
            .is_some_and(|hash| hash != &hex::encode(Sha256::digest(&data)))
    {
        return error(ApiCode::Conflict, "文件内容完整性校验失败");
    }
    let mut headers = HeaderMap::new();
    let disposition =
        format!("attachment; filename*=UTF-8''{}", encode_filename(&file.original_name));
    let Ok(disposition) = disposition.parse() else {
        return internal_error("文件下载响应无效");
    };
    headers.insert(header::CONTENT_DISPOSITION, disposition);
    // 用户上传类型仅作元数据；下载以附件和无嗅探方式提供。
    headers
        .insert(header::CONTENT_TYPE, "application/octet-stream".parse().expect("static header"));
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().expect("static header"));
    match state.repository.record_download(&scope, &id) {
        Ok(true) => (headers, data).into_response(),
        Ok(false) => forbidden("文件状态已改变"),
        Err(err) => storage_error(err),
    }
}
fn encode_filename(name: &str) -> String {
    name.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

pub fn build_file_storage_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<FileStorageState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/", get(list_files_handler))
        .route(
            "/upload",
            post(upload_file_handler).layer(DefaultBodyLimit::max(UPLOAD_LIMIT + 64 * 1024)),
        )
        .route("/stats", get(file_stats_handler))
        .route("/categories", get(list_file_categories_handler))
        .route("/storage-types", get(list_storage_types_handler))
        .route("/:file_id", get(get_file_handler).delete(delete_file_handler))
        .route("/:file_id/download", get(download_file_handler))
        .route("/:file_id/restore", post(restore_file_handler))
}

/// GET /api/enterprise/files/categories —— 获取文件分类列表
pub async fn list_file_categories_handler(ApiAuth(user): ApiAuth) -> Response {
    if let Err(message) = access(&user, false) {
        return forbidden(message);
    }
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
pub async fn list_storage_types_handler(ApiAuth(user): ApiAuth) -> Response {
    if let Err(message) = access(&user, false) {
        return forbidden(message);
    }
    let types = vec![
        json!({"code": "local", "available": true, "name": "本地存储", "description": "服务器本地文件系统"}),
        json!({"code": "oss", "available": false, "name": "阿里云OSS", "description": "阿里云对象存储服务"}),
        json!({"code": "s3", "available": false, "name": "AWS S3", "description": "Amazon S3对象存储"}),
        json!({"code": "minio", "available": false, "name": "MinIO", "description": "开源对象存储服务"}),
        json!({"code": "ftp", "available": false, "name": "FTP", "description": "FTP文件传输协议"}),
    ];
    success(types)
}


/// 根据扩展名和MIME类型分类文件
fn classify_file(extension: &str, content_type: &str) -> String {
    let ext = extension.to_lowercase();
    if content_type.starts_with("image/")
        || matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp")
    {
        return "image".to_string();
    }
    if content_type.starts_with("video/")
        || matches!(ext.as_str(), "mp4" | "avi" | "mov" | "wmv" | "flv" | "mkv")
    {
        return "video".to_string();
    }
    if content_type.starts_with("audio/")
        || matches!(ext.as_str(), "mp3" | "wav" | "flac" | "aac" | "ogg")
    {
        return "audio".to_string();
    }
    if matches!(ext.as_str(), "zip" | "rar" | "7z" | "tar" | "gz") {
        return "archive".to_string();
    }
    if matches!(
        ext.as_str(),
        "doc" | "docx" | "pdf" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md"
    ) {
        return "document".to_string();
    }
    "other".to_string()
}
