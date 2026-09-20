//! 邮件服务 API 端点

use crate::mailer::*;
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

/// 邮件服务状态
pub struct MailerState {
    pub smtp_config: Arc<RwLock<SmtpConfig>>,
    pub templates: Arc<RwLock<HashMap<String, EmailTemplate>>>,
    pub send_history: Arc<RwLock<Vec<SendResult>>>,
    pub smtp_client: Arc<SmtpClient>,
}

impl MailerState {
    pub fn new() -> Self {
        let config = SmtpConfig::default();
        let mut templates = HashMap::new();
        for t in builtin_templates() {
            templates.insert(t.template_code.clone(), t);
        }

        Self {
            smtp_config: Arc::new(RwLock::new(config.clone())),
            templates: Arc::new(RwLock::new(templates)),
            send_history: Arc::new(RwLock::new(Vec::new())),
            smtp_client: Arc::new(SmtpClient::new(config)),
        }
    }

    pub async fn update_config(&self, config: SmtpConfig) {
        *self.smtp_config.write().await = config.clone();
    }
}

impl Default for MailerState {
    fn default() -> Self {
        Self::new()
    }
}

/// POST /api/enterprise/mailer/send —— 发送邮件
pub async fn send_email_handler(
    State(state): State<Arc<MailerState>>,
    Json(req): Json<SendEmailRequest>,
) -> Response {
    if req.to.is_empty() {
        return bad_request("收件人不能为空");
    }
    if req.subject.is_empty() {
        return bad_request("邮件主题不能为空");
    }

    let message_id = format!("msg_{}", uuid::Uuid::new_v4().simple());

    // 处理模板
    let (subject, html_body, text_body) = if let Some(template_code) = &req.template_code {
        let templates = state.templates.read().await;
        match templates.get(template_code) {
            Some(template) => {
                let vars = req.template_vars.unwrap_or_default();
                let subject = render_template(&template.subject_template, &vars);
                let html = render_template(&template.html_template, &vars);
                let text = template.text_template.as_ref().map(|t| render_template(t, &vars));
                (subject, Some(html), text)
            }
            None => return not_found(&format!("邮件模板 '{}' 不存在", template_code)),
        }
    } else {
        (req.subject, req.html_body, req.text_body)
    };

    if html_body.is_none() && text_body.is_none() {
        return bad_request("邮件内容不能为空");
    }

    let message = EmailMessage {
        message_id: message_id.clone(),
        to: req.to,
        cc: req.cc,
        bcc: req.bcc,
        subject,
        text_body,
        html_body,
        attachments: req.attachments,
        headers: None,
        priority: req.priority,
    };

    // 发送邮件（异步执行，避免阻塞）
    let client = state.smtp_client.clone();
    let history = state.send_history.clone();
    let msg = message.clone();

    tokio::spawn(async move {
        let result = client.send(&msg);
        let mut history = history.write().await;
        history.push(result.unwrap_or_else(|e| SendResult {
            success: false,
            message_id: msg.message_id.clone(),
            error: Some(e),
            sent_at: chrono::Utc::now().to_rfc3339(),
        }));
    });

    success_with_message("邮件已提交发送", json!({ "message_id": message_id }))
}

/// GET /api/enterprise/mailer/config —— 获取SMTP配置
pub async fn get_smtp_config_handler(
    State(state): State<Arc<MailerState>>,
) -> Response {
    let config = state.smtp_config.read().await;
    // 不返回密码
    let mut config = config.clone();
    if !config.password.is_empty() {
        config.password = "******".to_string();
    }
    success(config)
}

/// PUT /api/enterprise/mailer/config —— 更新SMTP配置
pub async fn update_smtp_config_handler(
    State(state): State<Arc<MailerState>>,
    Json(config): Json<SmtpConfig>,
) -> Response {
    state.update_config(config).await;
    success_message("SMTP配置更新成功")
}

/// POST /api/enterprise/mailer/test —— 发送测试邮件
pub async fn send_test_email_handler(
    State(state): State<Arc<MailerState>>,
    Json(req): Json<HashMap<String, String>>,
) -> Response {
    let to = match req.get("to") {
        Some(t) if !t.is_empty() => vec![t.clone()],
        _ => return bad_request("测试收件人不能为空"),
    };

    let message_id = format!("test_{}", uuid::Uuid::new_v4().simple());
    let message = EmailMessage {
        message_id: message_id.clone(),
        to,
        cc: None,
        bcc: None,
        subject: "【MOX】SMTP配置测试邮件".to_string(),
        text_body: Some("这是一封来自MOX企业级平台的测试邮件。如果您收到此邮件，说明SMTP配置正确。".to_string()),
        html_body: Some("<h2>SMTP配置测试</h2><p>这是一封来自<strong>MOX企业级平台</strong>的测试邮件。</p><p>如果您收到此邮件，说明SMTP配置正确。</p><p>发送时间：{}</p>".replace("{}", &chrono::Utc::now().to_rfc3339())),
        attachments: None,
        headers: None,
        priority: Some("normal".to_string()),
    };

    let client = state.smtp_client.clone();
    let result = client.send(&message);

    match result {
        Ok(r) => success_with_message("测试邮件发送成功", json!(r)),
        Err(e) => internal_error(&format!("测试邮件发送失败: {}", e)),
    }
}

/// GET /api/enterprise/mailer/templates —— 获取邮件模板列表
pub async fn list_templates_handler(
    State(state): State<Arc<MailerState>>,
) -> Response {
    let templates = state.templates.read().await;
    let mut list: Vec<&EmailTemplate> = templates.values().collect();
    list.sort_by(|a, b| a.template_code.cmp(&b.template_code));
    success(list)
}

/// GET /api/enterprise/mailer/templates/:code —— 获取模板详情
pub async fn get_template_handler(
    State(state): State<Arc<MailerState>>,
    Path(code): Path<String>,
) -> Response {
    let templates = state.templates.read().await;
    match templates.get(&code) {
        Some(t) => success(t),
        None => not_found("邮件模板不存在"),
    }
}

/// POST /api/enterprise/mailer/templates —— 创建邮件模板
pub async fn create_template_handler(
    State(state): State<Arc<MailerState>>,
    Json(template): Json<EmailTemplate>,
) -> Response {
    let mut templates = state.templates.write().await;
    if templates.contains_key(&template.template_code) {
        return conflict(&format!("模板编码 '{}' 已存在", template.template_code));
    }
    templates.insert(template.template_code.clone(), template.clone());
    success_with_message("模板创建成功", json!({ "template_code": template.template_code }))
}

/// GET /api/enterprise/mailer/history —— 获取发送历史
pub async fn get_send_history_handler(
    State(state): State<Arc<MailerState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let history = state.send_history.read().await;
    let mut list: Vec<&SendResult> = history.iter().collect();

    if let Some(success) = params.get("success") {
        let s = success == "true";
        list.retain(|r| r.success == s);
    }

    list.sort_by(|a, b| b.sent_at.cmp(&a.sent_at));

    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, list.len());
    let page_items = pagination.paginate(&list);
    success_list(page_items, &pagination)
}

/// GET /api/enterprise/mailer/stats —— 邮件统计
pub async fn mailer_stats_handler(
    State(state): State<Arc<MailerState>>,
) -> Response {
    let history = state.send_history.read().await;
    let total = history.len();
    let success_count = history.iter().filter(|r| r.success).count();
    let failure_count = total - success_count;

    success(json!({
        "total_sent": total,
        "success_count": success_count,
        "failure_count": failure_count,
        "success_rate": if total > 0 { success_count as f64 / total as f64 } else { 0.0 },
        "total_templates": state.templates.read().await.len(),
    }))
}

/// 构建邮件服务路由（泛型版本）
pub fn build_mailer_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<MailerState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/send", post(send_email_handler))
        .route("/test", post(send_test_email_handler))
        .route("/config", get(get_smtp_config_handler).put(update_smtp_config_handler))
        .route("/templates", get(list_templates_handler).post(create_template_handler))
        .route("/templates/:code", get(get_template_handler))
        .route("/history", get(get_send_history_handler))
        .route("/stats", get(mailer_stats_handler))
}
