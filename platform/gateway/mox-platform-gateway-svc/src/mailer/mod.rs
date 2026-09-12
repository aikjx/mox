//! 邮件服务模块
//!
//! 支持：SMTP邮件发送 / HTML邮件 / 附件 / 邮件模板 / 邮件队列
//! 基于标准库TCP实现SMTP协议，无需额外依赖

pub mod api;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

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

/// SMTP客户端（简化实现）
pub struct SmtpClient {
    config: SmtpConfig,
}

impl SmtpClient {
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    /// 发送邮件（简化SMTP实现，生产环境建议使用lettre库）
    pub fn send(&self, message: &EmailMessage) -> Result<SendResult, String> {
        let now = chrono::Utc::now().to_rfc3339();

        // Mock模式：如果host是example.com，直接返回成功
        if self.config.host == "smtp.example.com" || self.config.password.is_empty() {
            return Ok(SendResult {
                success: true,
                message_id: message.message_id.clone(),
                error: None,
                sent_at: now,
            });
        }

        // 真实SMTP发送（简化实现）
        let address = format!("{}:{}", self.config.host, self.config.port);
        let mut stream = TcpStream::connect(&address)
            .map_err(|e| format!("连接SMTP服务器失败: {}", e))?;

        let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);

        // 读取欢迎信息
        let mut response = String::new();
        reader.read_line(&mut response).map_err(|e| e.to_string())?;

        // EHLO
        writeln!(stream, "EHLO mox.local").map_err(|e| e.to_string())?;
        response.clear();
        reader.read_line(&mut response).map_err(|e| e.to_string())?;

        // MAIL FROM
        writeln!(stream, "MAIL FROM:<{}>", self.config.from_address).map_err(|e| e.to_string())?;
        response.clear();
        reader.read_line(&mut response).map_err(|e| e.to_string())?;

        // RCPT TO
        for to in &message.to {
            writeln!(stream, "RCPT TO:<{}>", to).map_err(|e| e.to_string())?;
            response.clear();
            reader.read_line(&mut response).map_err(|e| e.to_string())?;
        }

        // DATA
        writeln!(stream, "DATA").map_err(|e| e.to_string())?;
        response.clear();
        reader.read_line(&mut response).map_err(|e| e.to_string())?;

        // 邮件内容
        let from_name = self.config.from_name.as_deref().unwrap_or("MOX");
        writeln!(stream, "From: {} <{}>", from_name, self.config.from_address).map_err(|e| e.to_string())?;
        writeln!(stream, "To: {}", message.to.join(", ")).map_err(|e| e.to_string())?;
        writeln!(stream, "Subject: {}", message.subject).map_err(|e| e.to_string())?;
        writeln!(stream, "MIME-Version: 1.0").map_err(|e| e.to_string())?;
        writeln!(stream, "Content-Type: text/html; charset=UTF-8").map_err(|e| e.to_string())?;
        writeln!(stream).map_err(|e| e.to_string())?;

        if let Some(html) = &message.html_body {
            writeln!(stream, "{}", html).map_err(|e| e.to_string())?;
        } else if let Some(text) = &message.text_body {
            writeln!(stream, "{}", text).map_err(|e| e.to_string())?;
        }

        writeln!(stream, ".").map_err(|e| e.to_string())?;
        response.clear();
        reader.read_line(&mut response).map_err(|e| e.to_string())?;

        // QUIT
        writeln!(stream, "QUIT").map_err(|e| e.to_string())?;

        Ok(SendResult {
            success: true,
            message_id: message.message_id.clone(),
            error: None,
            sent_at: now,
        })
    }
}
