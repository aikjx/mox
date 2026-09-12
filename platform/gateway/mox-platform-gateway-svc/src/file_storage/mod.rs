//! 文件存储管理模块
//!
//! 支持：文件元数据管理 / 文件上传下载 / 多存储后端 / 文件分类 / 文件统计

pub mod api;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 存储类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    Local,
    Oss,
    S3,
    Minio,
    Ftp,
}

impl StorageType {
    pub fn as_str(&self) -> &str {
        match self {
            StorageType::Local => "local",
            StorageType::Oss => "oss",
            StorageType::S3 => "s3",
            StorageType::Minio => "minio",
            StorageType::Ftp => "ftp",
        }
    }
}

/// 文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// 文件ID
    pub file_id: String,
    /// 租户ID
    pub tenant_id: Option<String>,
    /// 原始文件名
    pub original_name: String,
    /// 存储文件名
    pub stored_name: String,
    /// 文件路径
    pub file_path: String,
    /// 文件大小（字节）
    pub file_size: i64,
    /// 文件类型（MIME）
    pub content_type: String,
    /// 文件扩展名
    pub extension: String,
    /// 文件分类（document/image/video/audio/archive/other）
    pub file_category: String,
    /// 存储类型
    pub storage_type: String,
    /// 存储桶/目录
    pub bucket: Option<String>,
    /// MD5校验值
    pub md5: Option<String>,
    /// 上传人
    pub uploaded_by: String,
    /// 上传时间
    pub uploaded_at: String,
    /// 最后访问时间
    pub last_accessed_at: Option<String>,
    /// 下载次数
    pub download_count: i64,
    /// 文件状态（normal/archived/deleted）
    pub status: String,
    /// 标签
    pub tags: Option<Vec<String>>,
    /// 描述
    pub description: Option<String>,
}

/// 文件分类统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileStats {
    /// 总文件数
    pub total_files: i64,
    /// 总大小（字节）
    pub total_size: i64,
    /// 按分类统计
    pub by_category: HashMap<String, i64>,
    /// 按存储类型统计
    pub by_storage_type: HashMap<String, i64>,
    /// 今日上传数
    pub today_uploads: i64,
    /// 总下载次数
    pub total_downloads: i64,
}

/// 内置示例文件
pub fn sample_files() -> Vec<FileMetadata> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        FileMetadata {
            file_id: "file_001".to_string(),
            tenant_id: Some("tenant_001".to_string()),
            original_name: "合同模板.docx".to_string(),
            stored_name: "2026/09/file_001.docx".to_string(),
            file_path: "/data/files/2026/09/file_001.docx".to_string(),
            file_size: 245760,
            content_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
            extension: "docx".to_string(),
            file_category: "document".to_string(),
            storage_type: "local".to_string(),
            bucket: None,
            md5: Some("abc123def456".to_string()),
            uploaded_by: "admin".to_string(),
            uploaded_at: now.clone(),
            last_accessed_at: Some(now.clone()),
            download_count: 15,
            status: "normal".to_string(),
            tags: Some(vec!["合同".to_string(), "模板".to_string()]),
            description: Some("标准合同模板".to_string()),
        },
        FileMetadata {
            file_id: "file_002".to_string(),
            tenant_id: Some("tenant_001".to_string()),
            original_name: "产品架构图.png".to_string(),
            stored_name: "2026/09/file_002.png".to_string(),
            file_path: "/data/files/2026/09/file_002.png".to_string(),
            file_size: 1048576,
            content_type: "image/png".to_string(),
            extension: "png".to_string(),
            file_category: "image".to_string(),
            storage_type: "local".to_string(),
            bucket: None,
            md5: Some("def789ghi012".to_string()),
            uploaded_by: "admin".to_string(),
            uploaded_at: now.clone(),
            last_accessed_at: None,
            download_count: 8,
            status: "normal".to_string(),
            tags: Some(vec!["架构".to_string(), "图片".to_string()]),
            description: None,
        },
        FileMetadata {
            file_id: "file_003".to_string(),
            tenant_id: Some("tenant_002".to_string()),
            original_name: "培训视频.mp4".to_string(),
            stored_name: "2026/09/file_003.mp4".to_string(),
            file_path: "/data/files/2026/09/file_003.mp4".to_string(),
            file_size: 52428800,
            content_type: "video/mp4".to_string(),
            extension: "mp4".to_string(),
            file_category: "video".to_string(),
            storage_type: "minio".to_string(),
            bucket: Some("videos".to_string()),
            md5: None,
            uploaded_by: "zhangsan".to_string(),
            uploaded_at: now,
            last_accessed_at: None,
            download_count: 3,
            status: "normal".to_string(),
            tags: None,
            description: Some("新员工培训视频".to_string()),
        },
    ]
}
