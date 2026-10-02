//! 邮件服务模块
//!
//! 支持：SMTP邮件发送 / HTML邮件 / 附件 / 邮件模板 / 邮件队列
//! 使用 lettre 的真实 SMTP、必需 TLS/STARTTLS、认证和 MIME 编码。

pub mod api;

use base64::Engine;
use lettre::{
    message::{header::ContentType, Attachment, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    Message as MimeMessage, SmtpTransport, Transport,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SMTP配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    /// SMTP服务器地址
    pub host: String,
    /// SMTP服务器端口
    pub port: u16,
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 发件人地址
    pub from_address: String,
    /// 发件人名称
    pub from_name: Option<String>,
    /// 是否使用TLS
    pub use_tls: bool,
    /// 是否使用STARTTLS
    pub use_starttls: bool,
    /// 超时时间（秒）
    pub timeout_seconds: u64,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: "noreply@example.com".to_string(),
            password: "".to_string(),
            from_address: "noreply@example.com".to_string(),
            from_name: Some("MOX系统".to_string()),
            use_tls: false,
            use_starttls: true,
            timeout_seconds: 30,
        }
    }
}

/// 邮件消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    /// 邮件ID
    pub message_id: String,
    /// 收件人列表
    pub to: Vec<String>,
    /// 抄送列表
    pub cc: Option<Vec<String>>,
    /// 密送列表
    pub bcc: Option<Vec<String>>,
    /// 邮件主题
    pub subject: String,
    /// 纯文本内容
    pub text_body: Option<String>,
    /// HTML内容
    pub html_body: Option<String>,
    /// 附件列表
    pub attachments: Option<Vec<EmailAttachment>>,
    /// 自定义头部
    pub headers: Option<HashMap<String, String>>,
    /// 优先级（high/normal/low）
    pub priority: Option<String>,
}

/// 邮件附件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAttachment {
    /// 文件名
    pub filename: String,
    /// MIME类型
    pub content_type: String,
    /// Base64编码的内容
    pub content_base64: String,
}

/// 发送结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    /// 是否成功
    pub success: bool,
    /// 消息ID
    pub message_id: String,
    /// 错误信息
    pub error: Option<String>,
    /// 发送时间
    pub sent_at: String,
}

/// 邮件模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTemplate {
    /// 模板ID
    pub template_id: String,
    /// 模板名称
    pub template_name: String,
    /// 模板编码
    pub template_code: String,
    /// 主题模板
    pub subject_template: String,
    /// HTML内容模板
    pub html_template: String,
    /// 纯文本内容模板
    pub text_template: Option<String>,
    /// 模板变量
    pub variables: Vec<String>,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: String,
}

/// 发送邮件请求
#[derive(Debug, Deserialize)]
pub struct SendEmailRequest {
    pub to: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub subject: String,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub template_code: Option<String>,
    pub template_vars: Option<HashMap<String, String>>,
    pub attachments: Option<Vec<EmailAttachment>>,
    pub priority: Option<String>,
}

/// 内置邮件模板
pub fn builtin_templates() -> Vec<EmailTemplate> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        EmailTemplate {
            template_id: "tpl_001".to_string(),
            template_name: "审批通知".to_string(),
            template_code: "approval_notification".to_string(),
            subject_template: "【审批通知】{{title}}".to_string(),
            html_template: "<h2>审批通知</h2><p>您有一条新的审批待处理：</p><p><strong>标题：</strong>{{title}}</p><p><strong>申请人：</strong>{{applicant}}</p><p><strong>申请时间：</strong>{{apply_time}}</p><p>请及时处理。</p>".to_string(),
            text_template: Some("审批通知：您有一条新的审批待处理，标题：{{title}}，申请人：{{applicant}}".to_string()),
            variables: vec!["title".to_string(), "applicant".to_string(), "apply_time".to_string()],
            enabled: true,
            created_at: now.clone(),
        },
        EmailTemplate {
            template_id: "tpl_002".to_string(),
            template_name: "验证码".to_string(),
            template_code: "verification_code".to_string(),
            subject_template: "【验证码】您的验证码是 {{code}}".to_string(),
            html_template: "<h2>验证码</h2><p>您的验证码是：</p><p style=\"font-size:24px;font-weight:bold;color:#1890ff;\">{{code}}</p><p>验证码有效期为{{expire_minutes}}分钟，请勿泄露给他人。</p>".to_string(),
            text_template: Some("您的验证码是：{{code}}，有效期{{expire_minutes}}分钟".to_string()),
            variables: vec!["code".to_string(), "expire_minutes".to_string()],
            enabled: true,
            created_at: now.clone(),
        },
        EmailTemplate {
            template_id: "tpl_003".to_string(),
            template_name: "任务提醒".to_string(),
            template_code: "task_reminder".to_string(),
            subject_template: "【任务提醒】{{task_title}}".to_string(),
            html_template: "<h2>任务提醒</h2><p>您有一个任务即将到期：</p><p><strong>任务：</strong>{{task_title}}</p><p><strong>截止时间：</strong>{{deadline}}</p><p>请及时完成。</p>".to_string(),
            text_template: Some("任务提醒：{{task_title}}，截止时间：{{deadline}}".to_string()),
            variables: vec!["task_title".to_string(), "deadline".to_string()],
            enabled: true,
            created_at: now,
        },
    ]
}

/// 渲染模板变量
pub fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}

/// 真实 SMTP 客户端；成功表示上游 SMTP 接受，不表示收件人已阅读。
pub struct SmtpClient {
    config: SmtpConfig,
}

impl SmtpClient {
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    pub fn send(&self, message: &EmailMessage) -> Result<SendResult, String> {
        let config = &self.config;
        if config.host.trim().is_empty() || config.host == "smtp.example.com" {
            return Err("SMTP server is not configured".into());
        }
        if config.username.is_empty() != config.password.is_empty() {
            return Err("SMTP username and password must be configured together".into());
        }
        if config.port == 0 || !(1..=120).contains(&config.timeout_seconds) {
            return Err("Invalid SMTP port or timeout (1..120 seconds)".into());
        }
        if config.use_tls == config.use_starttls {
            return Err("Select exactly one of TLS or required STARTTLS".into());
        }
        if message.to.is_empty() || message.subject.trim().is_empty() {
            return Err("Recipient and subject are required".into());
        }
        if message.headers.as_ref().is_some_and(|headers| !headers.is_empty()) {
            return Err("Custom SMTP headers are not supported".into());
        }
        let mut builder = MimeMessage::builder()
            .from(Mailbox::new(
                config.from_name.clone(),
                config.from_address.parse().map_err(|_| "Invalid sender address")?,
            ))
            .subject(&message.subject);
        for recipient in &message.to {
            builder = builder.to(recipient.parse().map_err(|_| "Invalid recipient address")?);
        }
        for recipient in message.cc.iter().flatten() {
            builder = builder.cc(recipient.parse().map_err(|_| "Invalid CC address")?);
        }
        for recipient in message.bcc.iter().flatten() {
            builder = builder.bcc(recipient.parse().map_err(|_| "Invalid BCC address")?);
        }
        let body = match (&message.text_body, &message.html_body) {
            (Some(text), Some(html)) => MultiPart::alternative()
                .singlepart(SinglePart::plain(text.clone()))
                .singlepart(SinglePart::html(html.clone())),
            (Some(text), None) => MultiPart::mixed().singlepart(SinglePart::plain(text.clone())),
            (None, Some(html)) => MultiPart::mixed().singlepart(SinglePart::html(html.clone())),
            (None, None) => return Err("Email body is required".into()),
        };
        let mut mime = MultiPart::mixed().multipart(body);
        for attachment in message.attachments.iter().flatten() {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&attachment.content_base64)
                .map_err(|_| "Invalid attachment base64")?;
            let content_type = ContentType::parse(&attachment.content_type)
                .map_err(|_| "Invalid attachment content type")?;
            mime = mime
                .singlepart(Attachment::new(attachment.filename.clone()).body(bytes, content_type));
        }
        let email = builder.multipart(mime).map_err(|_| "Invalid MIME message")?;
        let transport = if config.use_tls {
            SmtpTransport::relay(&config.host)
        } else {
            SmtpTransport::starttls_relay(&config.host)
        }
        .map_err(|_| "Invalid SMTP TLS configuration")?;
        let mut transport = transport
            .port(config.port)
            .timeout(Some(std::time::Duration::from_secs(config.timeout_seconds)));
        if !config.username.is_empty() {
            transport = transport
                .credentials(Credentials::new(config.username.clone(), config.password.clone()));
        }
        transport
            .build()
            .send(&email)
            .map_err(|error| format!("SMTP delivery failed: {error}"))?;
        Ok(SendResult {
            success: true,
            message_id: message.message_id.clone(),
            error: None,
            sent_at: chrono::Utc::now().to_rfc3339(),
        })
    }
}
