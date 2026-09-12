//! 数据字典管理模块
//!
//! 支持：字典类型 / 字典项 / 树形字典 / 字典缓存 / 字典导入导出

pub mod api;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 字典类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictType {
    /// 字典类型ID
    pub dict_id: String,
    /// 字典名称
    pub dict_name: String,
    /// 字典类型编码（唯一）
    pub dict_type: String,
    /// 字典状态（0正常 1停用）
    pub status: String,
    /// 是否为系统字典（不可删除）
    pub is_system: bool,
    /// 备注
    pub remark: Option<String>,
    /// 创建人
    pub created_by: String,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 字典项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictItem {
    /// 字典项编码
    pub item_code: String,
    /// 字典项值
    pub item_value: String,
    /// 字典项标签（显示名称）
    pub item_label: String,
    /// 所属字典类型编码
    pub dict_type: String,
    /// 父级编码（树形字典使用）
    pub parent_code: Option<String>,
    /// 排序
    pub sort_order: i32,
    /// 样式属性（CSS类名/颜色）
    pub css_class: Option<String>,
    /// 列表类型（default/primary/success/warning/danger/info）
    pub list_class: Option<String>,
    /// 是否默认
    pub is_default: bool,
    /// 状态（0正常 1停用）
    pub status: String,
    /// 备注
    pub remark: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 创建字典类型请求
#[derive(Debug, Deserialize)]
pub struct CreateDictTypeRequest {
    pub dict_name: String,
    pub dict_type: String,
    pub status: Option<String>,
    pub remark: Option<String>,
}

/// 创建字典项请求
#[derive(Debug, Deserialize)]
pub struct CreateDictItemRequest {
    pub item_value: String,
    pub item_label: String,
    pub dict_type: String,
    pub parent_code: Option<String>,
    pub sort_order: Option<i32>,
    pub css_class: Option<String>,
    pub list_class: Option<String>,
    pub is_default: Option<bool>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

/// 内置系统字典
pub fn builtin_dict_types() -> Vec<DictType> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        DictType {
            dict_id: "dict_sys_user_sex".to_string(),
            dict_name: "用户性别".to_string(),
            dict_type: "sys_user_sex".to_string(),
            status: "0".to_string(),
            is_system: true,
            remark: Some("用户性别列表".to_string()),
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        DictType {
            dict_id: "dict_sys_show_status".to_string(),
            dict_name: "系统开关".to_string(),
            dict_type: "sys_show_status".to_string(),
            status: "0".to_string(),
            is_system: true,
            remark: Some("系统开关状态".to_string()),
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        DictType {
            dict_id: "dict_sys_normal_disable".to_string(),
            dict_name: "系统状态".to_string(),
            dict_type: "sys_normal_disable".to_string(),
            status: "0".to_string(),
            is_system: true,
            remark: Some("通用正常/停用状态".to_string()),
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        DictType {
            dict_id: "dict_approval_status".to_string(),
            dict_name: "审批状态".to_string(),
            dict_type: "approval_status".to_string(),
            status: "0".to_string(),
            is_system: true,
            remark: Some("审批流程状态".to_string()),
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        DictType {
            dict_id: "dict_document_type".to_string(),
            dict_name: "文档类型".to_string(),
            dict_type: "document_type".to_string(),
            status: "0".to_string(),
            is_system: true,
            remark: Some("文档管理类型".to_string()),
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_at: now,
        },
    ]
}

/// 内置字典项
pub fn builtin_dict_items() -> Vec<DictItem> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        // 用户性别
        DictItem { item_code: "0".to_string(), item_value: "0".to_string(), item_label: "男".to_string(), dict_type: "sys_user_sex".to_string(), parent_code: None, sort_order: 1, css_class: None, list_class: Some("primary".to_string()), is_default: true, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now.clone() },
        DictItem { item_code: "1".to_string(), item_value: "1".to_string(), item_label: "女".to_string(), dict_type: "sys_user_sex".to_string(), parent_code: None, sort_order: 2, css_class: None, list_class: Some("danger".to_string()), is_default: false, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now.clone() },
        DictItem { item_code: "2".to_string(), item_value: "2".to_string(), item_label: "未知".to_string(), dict_type: "sys_user_sex".to_string(), parent_code: None, sort_order: 3, css_class: None, list_class: Some("info".to_string()), is_default: false, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now.clone() },
        // 系统开关
        DictItem { item_code: "0".to_string(), item_value: "0".to_string(), item_label: "显示".to_string(), dict_type: "sys_show_status".to_string(), parent_code: None, sort_order: 1, css_class: None, list_class: Some("success".to_string()), is_default: true, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now.clone() },
        DictItem { item_code: "1".to_string(), item_value: "1".to_string(), item_label: "隐藏".to_string(), dict_type: "sys_show_status".to_string(), parent_code: None, sort_order: 2, css_class: None, list_class: Some("danger".to_string()), is_default: false, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now.clone() },
        // 系统状态
        DictItem { item_code: "0".to_string(), item_value: "0".to_string(), item_label: "正常".to_string(), dict_type: "sys_normal_disable".to_string(), parent_code: None, sort_order: 1, css_class: None, list_class: Some("success".to_string()), is_default: true, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now.clone() },
        DictItem { item_code: "1".to_string(), item_value: "1".to_string(), item_label: "停用".to_string(), dict_type: "sys_normal_disable".to_string(), parent_code: None, sort_order: 2, css_class: None, list_class: Some("danger".to_string()), is_default: false, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now.clone() },
        // 审批状态
        DictItem { item_code: "pending".to_string(), item_value: "pending".to_string(), item_label: "待审批".to_string(), dict_type: "approval_status".to_string(), parent_code: None, sort_order: 1, css_class: None, list_class: Some("warning".to_string()), is_default: true, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now.clone() },
        DictItem { item_code: "approved".to_string(), item_value: "approved".to_string(), item_label: "已通过".to_string(), dict_type: "approval_status".to_string(), parent_code: None, sort_order: 2, css_class: None, list_class: Some("success".to_string()), is_default: false, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now.clone() },
        DictItem { item_code: "rejected".to_string(), item_value: "rejected".to_string(), item_label: "已拒绝".to_string(), dict_type: "approval_status".to_string(), parent_code: None, sort_order: 3, css_class: None, list_class: Some("danger".to_string()), is_default: false, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now.clone() },
        // 文档类型
        DictItem { item_code: "contract".to_string(), item_value: "contract".to_string(), item_label: "合同".to_string(), dict_type: "document_type".to_string(), parent_code: None, sort_order: 1, css_class: None, list_class: Some("primary".to_string()), is_default: false, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now.clone() },
        DictItem { item_code: "agreement".to_string(), item_value: "agreement".to_string(), item_label: "协议".to_string(), dict_type: "document_type".to_string(), parent_code: None, sort_order: 2, css_class: None, list_class: Some("info".to_string()), is_default: false, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now.clone() },
        DictItem { item_code: "report".to_string(), item_value: "report".to_string(), item_label: "报告".to_string(), dict_type: "document_type".to_string(), parent_code: None, sort_order: 3, css_class: None, list_class: Some("success".to_string()), is_default: false, status: "0".to_string(), remark: None, created_at: now.clone(), updated_at: now },
    ]
}
