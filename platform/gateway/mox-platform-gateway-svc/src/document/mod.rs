//! 文档管理 + 电子签章模块
//!
//! 支持文档上传 / 版本管理 / 权限控制 / 在线编辑 / 电子签章 / 审批签署

pub mod api;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 文档类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    /// 合同
    Contract,
    /// 协议
    Agreement,
    /// 报告
    Report,
    /// 方案
    Proposal,
    /// 通知
    Notice,
    /// 制度
    Policy,
    /// 表单
    Form,
    /// 其他
    Other,
}

/// 文档状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    /// 草稿
    Draft,
    /// 审核中
    Reviewing,
    /// 已发布
    Published,
    /// 已归档
    Archived,
    /// 已作废
    Obsolete,
}

/// 文档定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// 文档 ID
    pub document_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 文档标题
    pub title: String,
    /// 文档类型
    pub document_type: DocumentType,
    /// 文档编号
    pub document_no: Option<String>,
    /// 文档描述
    pub description: Option<String>,
    /// 分类 ID
    pub category_id: Option<String>,
    /// 标签
    pub tags: Vec<String>,
    /// 当前版本号
    pub current_version: String,
    /// 状态
    pub status: DocumentStatus,
    /// 是否需要电子签章
    pub require_signature: bool,
    /// 签章状态：unsigned / signing / signed / rejected
    pub signature_status: Option<String>,
    /// 所有者 ID
    pub owner_id: String,
    /// 所属部门 ID
    pub department_id: Option<String>,
    /// 文件存储路径
    pub file_path: Option<String>,
    /// 文件大小（字节）
    pub file_size: Option<i64>,
    /// 文件类型（MIME）
    pub mime_type: Option<String>,
    /// 页数
    pub page_count: Option<i32>,
    /// 阅读次数
    pub view_count: i64,
    /// 下载次数
    pub download_count: i64,
    /// 权限配置
    pub permission_config: DocumentPermissionConfig,
    /// 元数据
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// 创建人
    pub created_by: String,
    /// 创建时间
    pub created_at: String,
    /// 更新人
    pub updated_by: Option<String>,
    /// 更新时间
    pub updated_at: String,
    /// 发布时间
    pub published_at: Option<String>,
    /// 归档时间
    pub archived_at: Option<String>,
}

/// 文档版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentVersion {
    /// 版本 ID
    pub version_id: String,
    /// 文档 ID
    pub document_id: String,
    /// 版本号
    pub version: String,
    /// 版本说明
    pub changelog: Option<String>,
    /// 文件存储路径
    pub file_path: String,
    /// 文件大小
    pub file_size: i64,
    /// 文件哈希（MD5/SHA256）
    pub file_hash: Option<String>,
    /// 是否当前版本
    pub is_current: bool,
    /// 创建人
    pub created_by: String,
    /// 创建时间
    pub created_at: String,
}

/// 文档权限配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentPermissionConfig {
    /// 可见范围：private / dept / dept_and_children / all / custom
    pub visibility: String,
    /// 可见用户 ID 列表（custom 时使用）
    pub visible_user_ids: Vec<String>,
    /// 可见角色 ID 列表
    pub visible_role_ids: Vec<String>,
    /// 可编辑用户 ID 列表
    pub editable_user_ids: Vec<String>,
    /// 可下载用户 ID 列表
    pub downloadable_user_ids: Vec<String>,
    /// 可删除用户 ID 列表
    pub deletable_user_ids: Vec<String>,
    /// 是否允许在线预览
    pub allow_preview: bool,
    /// 是否允许下载
    pub allow_download: bool,
    /// 是否允许打印
    pub allow_print: bool,
    /// 是否允许复制
    pub allow_copy: bool,
    /// 是否添加水印
    pub add_watermark: bool,
    /// 水印文本
    pub watermark_text: Option<String>,
}

/// 文档分类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentCategory {
    /// 分类 ID
    pub category_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 分类名称
    pub name: String,
    /// 分类编码
    pub code: String,
    /// 父分类 ID
    pub parent_id: Option<String>,
    /// 分类路径
    pub path: String,
    /// 排序
    pub sort_order: i32,
    /// 状态
    pub status: String,
    /// 创建时间
    pub created_at: String,
}

/// 电子签章记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureRecord {
    /// 签章 ID
    pub signature_id: String,
    /// 文档 ID
    pub document_id: String,
    /// 文档版本
    pub document_version: String,
    /// 签章人 ID
    pub signer_id: String,
    /// 签章人姓名
    pub signer_name: String,
    /// 签章类型：personal / corporate / official
    pub signature_type: String,
    /// 签章状态：pending / signed / rejected / revoked
    pub status: String,
    /// 签章位置（页码/坐标）
    pub position: Option<SignaturePosition>,
    /// 签章图片 URL
    pub signature_image_url: Option<String>,
    /// 数字证书序列号
    pub certificate_serial: Option<String>,
    /// 数字证书颁发者
    pub certificate_issuer: Option<String>,
    /// 签名值（Base64）
    pub signature_value: Option<String>,
    /// 签名算法
    pub signature_algorithm: Option<String>,
    /// 签章时间
    pub signed_at: Option<String>,
    /// 签章 IP
    pub ip_address: Option<String>,
    /// 拒绝原因
    pub reject_reason: Option<String>,
    /// 创建时间
    pub created_at: String,
}

/// 签章位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignaturePosition {
    /// 页码
    pub page: i32,
    /// X 坐标
    pub x: f64,
    /// Y 坐标
    pub y: f64,
    /// 宽度
    pub width: f64,
    /// 高度
    pub height: f64,
}

/// 签章模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureTemplate {
    /// 模板 ID
    pub template_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 模板名称
    pub name: String,
    /// 签章类型
    pub signature_type: String,
    /// 签章图片 URL
    pub image_url: String,
    /// 宽度
    pub width: i32,
    /// 高度
    pub height: i32,
    /// 透明度（0-100）
    pub opacity: i32,
    /// 状态
    pub status: String,
    /// 创建时间
    pub created_at: String,
}

/// 文档协作记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentActivity {
    /// 活动 ID
    pub activity_id: String,
    /// 文档 ID
    pub document_id: String,
    /// 活动类型：create / update / view / download / share / sign / comment / approve / reject
    pub activity_type: String,
    /// 操作人 ID
    pub actor_id: String,
    /// 操作人姓名
    pub actor_name: String,
    /// 活动详情
    pub details: Option<String>,
    /// IP 地址
    pub ip_address: Option<String>,
    /// 创建时间
    pub created_at: String,
}

/// 创建文档请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub document_type: DocumentType,
    pub document_no: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub require_signature: Option<bool>,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub permission_config: Option<DocumentPermissionConfig>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// 更新文档请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDocumentRequest {
    pub title: Option<String>,
    pub document_type: Option<DocumentType>,
    pub description: Option<String>,
    pub category_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<DocumentStatus>,
    pub require_signature: Option<bool>,
    pub permission_config: Option<DocumentPermissionConfig>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// 发起签章请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignDocumentRequest {
    /// 文档 ID
    pub document_id: String,
    /// 签章人 ID 列表
    pub signer_ids: Vec<String>,
    /// 签章顺序：sequential / parallel
    pub sign_order: String,
    /// 签章位置
    pub position: Option<SignaturePosition>,
    /// 签章截止时间
    pub deadline: Option<String>,
    /// 备注
    pub remark: Option<String>,
}

/// 文档统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentStats {
    /// 总数
    pub total: i64,
    /// 草稿
    pub draft: i64,
    /// 审核中
    pub reviewing: i64,
    /// 已发布
    pub published: i64,
    /// 已归档
    pub archived: i64,
    /// 待签章
    pub pending_signature: i64,
    /// 已签章
    pub signed: i64,
    /// 按类型统计
    pub by_type: HashMap<String, i64>,
}

/// 支持的文档类型
pub fn supported_document_types() -> Vec<(&'static str, &'static str)> {
    vec![
        ("contract", "合同"),
        ("agreement", "协议"),
        ("report", "报告"),
        ("proposal", "方案"),
        ("notice", "通知"),
        ("policy", "制度"),
        ("form", "表单"),
        ("other", "其他"),
    ]
}
