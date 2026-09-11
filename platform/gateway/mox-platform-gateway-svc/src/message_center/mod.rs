//! 消息推送模块
//!
//! 支持站内信 / 邮件 / 短信 / 飞书 / 钉钉 / 企业微信 等多渠道消息推送

pub mod api;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 消息渠道
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageChannel {
    /// 站内信
    InApp,
    /// 邮件
    Email,
    /// 短信
    Sms,
    /// 飞书
    Feishu,
    /// 钉钉
    DingTalk,
    /// 企业微信
    WeCom,
    /// Webhook
    Webhook,
    /// 移动端推送
    Push,
}

impl MessageChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InApp => "in_app",
            Self::Email => "email",
            Self::Sms => "sms",
            Self::Feishu => "feishu",
            Self::DingTalk => "dingtalk",
            Self::WeCom => "wecom",
            Self::Webhook => "webhook",
            Self::Push => "push",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::InApp => "站内信",
            Self::Email => "邮件",
            Self::Sms => "短信",
            Self::Feishu => "飞书",
            Self::DingTalk => "钉钉",
            Self::WeCom => "企业微信",
            Self::Webhook => "Webhook",
            Self::Push => "移动推送",
        }
    }
}

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// 系统通知
    System,
    /// 审批待办
    ApprovalPending,
    /// 审批结果
    ApprovalResult,
    /// 任务提醒
    Task,
    /// 告警
    Alert,
    /// 营销
    Marketing,
    /// 验证码
    Verification,
    /// 自定义
    Custom,
}

/// 消息优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// 消息状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    /// 待发送
    Pending,
    /// 发送中
    Sending,
    /// 已发送
    Sent,
    /// 已读
    Read,
    /// 发送失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 消息定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 消息 ID
    pub message_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 消息类型
    pub message_type: MessageType,
    /// 消息标题
    pub title: String,
    /// 消息内容
    pub content: String,
    /// 消息内容（HTML格式，邮件使用）
    pub html_content: Option<String>,
    /// 消息优先级
    pub priority: MessagePriority,
    /// 发送渠道
    pub channels: Vec<MessageChannel>,
    /// 发送人 ID（系统消息为 system）
    pub sender_id: String,
    /// 接收人 ID 列表
    pub receiver_ids: Vec<String>,
    /// 接收人邮箱列表（邮件渠道使用）
    pub receiver_emails: Option<Vec<String>>,
    /// 接收人手机列表（短信渠道使用）
    pub receiver_phones: Option<Vec<String>>,
    /// 跳转链接
    pub action_url: Option<String>,
    /// 附加数据
    pub extra_data: Option<HashMap<String, serde_json::Value>>,
    /// 定时发送时间（可选）
    pub scheduled_at: Option<String>,
    /// 过期时间（可选）
    pub expires_at: Option<String>,
    /// 状态
    pub status: MessageStatus,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 发送时间
    pub sent_at: Option<String>,
}

/// 消息发送记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSendRecord {
    /// 记录 ID
    pub record_id: String,
    /// 消息 ID
    pub message_id: String,
    /// 接收人 ID
    pub receiver_id: String,
    /// 发送渠道
    pub channel: MessageChannel,
    /// 状态
    pub status: MessageStatus,
    /// 错误信息
    pub error_message: Option<String>,
    /// 重试次数
    pub retry_count: i32,
    /// 发送时间
    pub sent_at: Option<String>,
    /// 阅读时间
    pub read_at: Option<String>,
    /// 创建时间
    pub created_at: String,
}

/// 消息模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTemplate {
    /// 模板 ID
    pub template_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 模板编码
    pub code: String,
    /// 模板名称
    pub name: String,
    /// 消息类型
    pub message_type: MessageType,
    /// 适用渠道
    pub channels: Vec<MessageChannel>,
    /// 标题模板（支持变量替换，如 {{username}}）
    pub title_template: String,
    /// 内容模板
    pub content_template: String,
    /// HTML 内容模板
    pub html_template: Option<String>,
    /// 变量列表
    pub variables: Vec<TemplateVariable>,
    /// 状态：enabled / disabled
    pub status: String,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 模板变量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    /// 变量名
    pub name: String,
    /// 变量描述
    pub description: String,
    /// 是否必填
    pub required: bool,
    /// 默认值
    pub default_value: Option<String>,
}

/// 消息渠道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// 配置 ID
    pub config_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 渠道类型
    pub channel: MessageChannel,
    /// 配置名称
    pub name: String,
    /// 状态：enabled / disabled
    pub status: String,
    /// 配置参数（加密存储敏感信息）
    pub config: HashMap<String, String>,
    /// 默认发送人
    pub default_sender: Option<String>,
    /// 每日发送上限
    pub daily_limit: Option<i32>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 发送消息请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    /// 消息类型
    pub message_type: MessageType,
    /// 消息标题
    pub title: String,
    /// 消息内容
    pub content: String,
    /// HTML 内容
    pub html_content: Option<String>,
    /// 优先级
    pub priority: Option<MessagePriority>,
    /// 发送渠道
    pub channels: Vec<MessageChannel>,
    /// 接收人 ID 列表
    pub receiver_ids: Option<Vec<String>>,
    /// 接收人邮箱列表
    pub receiver_emails: Option<Vec<String>>,
    /// 接收人手机列表
    pub receiver_phones: Option<Vec<String>>,
    /// 跳转链接
    pub action_url: Option<String>,
    /// 附加数据
    pub extra_data: Option<HashMap<String, serde_json::Value>>,
    /// 模板 ID（使用模板时）
    pub template_id: Option<String>,
    /// 模板变量
    pub template_vars: Option<HashMap<String, String>>,
    /// 定时发送时间
    pub scheduled_at: Option<String>,
}

/// 消息统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageStats {
    /// 总数
    pub total: i64,
    /// 已发送
    pub sent: i64,
    /// 已读
    pub read: i64,
    /// 失败
    pub failed: i64,
    /// 待发送
    pub pending: i64,
    /// 按渠道统计
    pub by_channel: HashMap<String, i64>,
    /// 按类型统计
    pub by_type: HashMap<String, i64>,
}

/// 支持的渠道列表
pub fn supported_channels() -> Vec<MessageChannel> {
    vec![
        MessageChannel::InApp,
        MessageChannel::Email,
        MessageChannel::Sms,
        MessageChannel::Feishu,
        MessageChannel::DingTalk,
        MessageChannel::WeCom,
        MessageChannel::Webhook,
        MessageChannel::Push,
    ]
}

/// 内置消息模板
pub fn builtin_templates() -> Vec<MessageTemplate> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        MessageTemplate {
            template_id: "tpl_approval_pending".to_string(),
            tenant_id: "default".to_string(),
            code: "approval_pending".to_string(),
            name: "审批待办通知".to_string(),
            message_type: MessageType::ApprovalPending,
            channels: vec![MessageChannel::InApp, MessageChannel::Email, MessageChannel::Feishu],
            title_template: "【审批待办】{{title}} 待您审批".to_string(),
            content_template: "您有一条新的审批待办：\n标题：{{title}}\n申请人：{{applicant}}\n申请时间：{{apply_time}}\n请及时处理。".to_string(),
            html_template: None,
            variables: vec![
                TemplateVariable { name: "title".to_string(), description: "审批标题".to_string(), required: true, default_value: None },
                TemplateVariable { name: "applicant".to_string(), description: "申请人".to_string(), required: true, default_value: None },
                TemplateVariable { name: "apply_time".to_string(), description: "申请时间".to_string(), required: true, default_value: None },
            ],
            status: "enabled".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        MessageTemplate {
            template_id: "tpl_approval_result".to_string(),
            tenant_id: "default".to_string(),
            code: "approval_result".to_string(),
            name: "审批结果通知".to_string(),
            message_type: MessageType::ApprovalResult,
            channels: vec![MessageChannel::InApp, MessageChannel::Email],
            title_template: "【审批结果】{{title}} 已{{result}}".to_string(),
            content_template: "您的审批申请已处理：\n标题：{{title}}\n审批结果：{{result}}\n审批人：{{approver}}\n审批时间：{{approve_time}}\n审批意见：{{comment}}".to_string(),
            html_template: None,
            variables: vec![
                TemplateVariable { name: "title".to_string(), description: "审批标题".to_string(), required: true, default_value: None },
                TemplateVariable { name: "result".to_string(), description: "审批结果（通过/驳回）".to_string(), required: true, default_value: None },
                TemplateVariable { name: "approver".to_string(), description: "审批人".to_string(), required: true, default_value: None },
                TemplateVariable { name: "approve_time".to_string(), description: "审批时间".to_string(), required: true, default_value: None },
                TemplateVariable { name: "comment".to_string(), description: "审批意见".to_string(), required: false, default_value: Some("无".to_string()) },
            ],
            status: "enabled".to_string(),
            created_at: now.clone(),
            updated_at: now,
        },
    ]
}
