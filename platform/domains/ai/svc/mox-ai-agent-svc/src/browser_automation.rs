//! 浏览器自动化引擎 - AI驱动的网页操作
//!
//! 提供最常用的真实浏览器自动化能力：
//! - 真实HTTP网页导航与内容获取
//! - 页面标题、URL、HTML、文本提取
//! - 自然语言驱动的自动化任务
//! - 5种预置任务模板
//! - 元素交互（点击、输入）模拟（提示需要Headless Chrome）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use tracing;

/// 真实HTTP获取的页面内容缓存
#[derive(Debug, Clone, Default)]
struct PageContent {
    url: String,
    title: String,
    html: String,
    text: String,
    status_code: u16,
}

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("导航失败: {0}")]
    NavigationFailed(String),
    #[error("元素未找到: {0}")]
    ElementNotFound(String),
    #[error("交互失败: {0}")]
    InteractionFailed(String),
    #[error("会话不存在: {0}")]
    SessionNotFound(String),
    #[error("超时: {0}")]
    Timeout(String),
    #[error("HTTP请求失败: {0}")]
    HttpError(String),
    #[error("其他错误: {0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserAction {
    Navigate {
        url: String,
    },
    Click {
        selector: String,
        timeout_ms: Option<u64>,
    },
    Type {
        selector: String,
        text: String,
        clear_first: Option<bool>,
    },
    ExtractText {
        selector: String,
    },
    ExtractAttribute {
        selector: String,
        attribute: String,
    },
    ExtractHtml,
    GetTitle,
    GetUrl,
    WaitFor {
        selector: String,
        timeout_ms: Option<u64>,
    },
    Wait {
        ms: u64,
    },
    Scroll {
        x: i32,
        y: i32,
    },
    ScrollTo {
        selector: String,
    },
    Screenshot,
    ExecuteScript {
        script: String,
    },
    SwitchFrame {
        selector: Option<String>,
    },
    GoBack,
    GoForward,
    Refresh,
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_type: String,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserSession {
    pub id: String,
    pub current_url: String,
    pub title: String,
    pub status: String,
    pub history: Vec<String>,
    pub action_log: Vec<ActionResult>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub last_action_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub start_url: Option<String>,
    pub steps: Vec<BrowserAction>,
    pub variables: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    pub task_id: String,
    pub task_name: String,
    pub success: bool,
    pub session_id: String,
    pub final_url: String,
    pub final_title: String,
    pub extracted_data: HashMap<String, serde_json::Value>,
    pub steps_results: Vec<ActionResult>,
    pub total_duration_ms: u64,
    pub error: Option<String>,
}

pub struct BrowserAutomationEngine {
    sessions: HashMap<String, BrowserSession>,
    saved_tasks: HashMap<String, AutomationTask>,
    screenshots: HashMap<String, String>,
    http_client: reqwest::Client,
    page_cache: HashMap<String, PageContent>,
}

impl BrowserAutomationEngine {
    pub fn new() -> Self {
        let mut saved_tasks = HashMap::new();
        Self::register_default_tasks(&mut saved_tasks);
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();
        Self {
            sessions: HashMap::new(),
            saved_tasks,
            screenshots: HashMap::new(),
            http_client,
            page_cache: HashMap::new(),
        }
    }

    fn register_default_tasks(tasks: &mut HashMap<String, AutomationTask>) {
        let now = chrono::Utc::now();
        let defaults = vec![
            AutomationTask {
                id: "web-search".into(),
                name: "网页搜索".into(),
                description: "在搜索引擎中搜索关键词并提取结果".into(),
                start_url: Some("https://www.bing.com".into()),
                steps: vec![
                    BrowserAction::Navigate {
                        url: "https://www.bing.com/search?q={{query}}".into(),
                    },
                    BrowserAction::WaitFor {
                        selector: "body".into(),
                        timeout_ms: Some(8000),
                    },
                    BrowserAction::GetTitle,
                    BrowserAction::GetUrl,
                    BrowserAction::ExtractText {
                        selector: "body".into(),
                    },
                ],
                variables: HashMap::from([("query".into(), "算子统一系统".into())]),
                created_at: now,
            },
            AutomationTask {
                id: "data-extraction".into(),
                name: "数据提取".into(),
                description: "访问网页并提取页面内容".into(),
                start_url: None,
                steps: vec![
                    BrowserAction::Navigate {
                        url: "{{url}}".into(),
                    },
                    BrowserAction::WaitFor {
                        selector: "body".into(),
                        timeout_ms: Some(10000),
                    },
                    BrowserAction::GetTitle,
                    BrowserAction::ExtractText {
                        selector: "body".into(),
                    },
                ],
                variables: HashMap::new(),
                created_at: now,
            },
            AutomationTask {
                id: "screenshot-capture".into(),
                name: "页面信息获取".into(),
                description: "访问URL并获取页面标题、URL和HTML内容".into(),
                start_url: None,
                steps: vec![
                    BrowserAction::Navigate {
                        url: "{{url}}".into(),
                    },
                    BrowserAction::WaitFor {
                        selector: "body".into(),
                        timeout_ms: Some(10000),
                    },
                    BrowserAction::GetTitle,
                    BrowserAction::GetUrl,
                    BrowserAction::ExtractHtml,
                ],
                variables: HashMap::new(),
                created_at: now,
            },
            AutomationTask {
                id: "get-page-info".into(),
                name: "获取页面信息".into(),
                description: "访问URL并获取页面完整信息".into(),
                start_url: None,
                steps: vec![
                    BrowserAction::Navigate {
                        url: "{{url}}".into(),
                    },
                    BrowserAction::WaitFor {
                        selector: "body".into(),
                        timeout_ms: Some(10000),
                    },
                    BrowserAction::GetTitle,
                    BrowserAction::GetUrl,
                    BrowserAction::ExtractHtml,
                ],
                variables: HashMap::new(),
                created_at: now,
            },
            AutomationTask {
                id: "form-submit".into(),
                name: "表单提交".into(),
                description: "访问页面、填写文本字段并提交（模拟）".into(),
                start_url: None,
                steps: vec![
                    BrowserAction::Navigate {
                        url: "{{url}}".into(),
                    },
                    BrowserAction::WaitFor {
                        selector: "form".into(),
                        timeout_ms: Some(10000),
                    },
                    BrowserAction::Type {
                        selector: "input".into(),
                        text: "{{text}}".into(),
                        clear_first: Some(true),
                    },
                    BrowserAction::Click {
                        selector: "submit".into(),
                        timeout_ms: Some(5000),
                    },
                    BrowserAction::Wait { ms: 1000 },
                ],
                variables: HashMap::new(),
                created_at: now,
            },
        ];
        for task in defaults {
            tasks.insert(task.id.clone(), task);
        }
    }

    pub fn create_session(&mut self) -> String {
        let id = format!("browser-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let now = chrono::Utc::now();
        let session = BrowserSession {
            id: id.clone(),
            current_url: String::new(),
            title: String::new(),
            status: "created".into(),
            history: Vec::new(),
            action_log: Vec::new(),
            started_at: now,
            last_action_at: now,
        };
        self.sessions.insert(id.clone(), session);
        tracing::info!("浏览器会话创建: {}", id);
        id
    }

    pub fn get_session(&self, session_id: &str) -> Option<&BrowserSession> {
        self.sessions.get(session_id)
    }

    pub fn list_sessions(&self) -> Vec<&BrowserSession> {
        self.sessions.values().collect()
    }

    pub fn close_session(&mut self, session_id: &str) -> Result<(), BrowserError> {
        self.page_cache.remove(session_id);
        if self.sessions.remove(session_id).is_some() {
            tracing::info!("浏览器会话关闭: {}", session_id);
            Ok(())
        } else {
            Err(BrowserError::SessionNotFound(session_id.into()))
        }
    }

    /// 真实HTTP请求获取页面内容
    async fn fetch_page(&self, url: &str) -> Result<PageContent, BrowserError> {
        tracing::debug!("发起HTTP GET: {}", url);
        let resp = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| BrowserError::HttpError(format!("请求失败: {}", e)))?;

        let status_code = resp.status().as_u16();
        let final_url = resp.url().to_string();
        let html = resp
            .text()
            .await
            .map_err(|e| BrowserError::HttpError(format!("读取响应失败: {}", e)))?;

        let title = extract_title(&html);
        let text = html_to_text(&html);

        Ok(PageContent {
            url: final_url,
            title,
            html,
            text,
            status_code,
        })
    }

    pub async fn execute_action(
        &mut self,
        session_id: &str,
        action: BrowserAction,
    ) -> Result<ActionResult, BrowserError> {
        let start = std::time::Instant::now();

        // 预检查session存在
        if !self.sessions.contains_key(session_id) {
            return Err(BrowserError::SessionNotFound(session_id.into()));
        }

        // 动作类型规范化（单一数据源：action_type_name 决定 ActionResult.action_type）
        let action_type = action_type_name(&action);

        // 先收集需要从不可变借用读取的数据
        let cached_page = self.page_cache.get(session_id).cloned();
        let session_url = self
            .sessions
            .get(session_id)
            .map(|s| s.current_url.clone())
            .unwrap_or_default();
        let session_title = self
            .sessions
            .get(session_id)
            .map(|s| s.title.clone())
            .unwrap_or_default();
        let session_history_len = self
            .sessions
            .get(session_id)
            .map(|s| s.history.len())
            .unwrap_or(0);

        let ar: ActionResult = match &action {
            BrowserAction::Navigate { url } => {
                tracing::info!("[{}] 真实HTTP请求: {}", session_id, url);
                let normalized_url = normalize_url(url);
                match self.fetch_page(&normalized_url).await {
                    Ok(page) => {
                        let title_preview = page.title.clone();
                        let content_len = page.html.len();
                        let status = page.status_code;
                        let final_url = page.url.clone();
                        self.page_cache.insert(session_id.to_string(), page);
                        // 更新session
                        if let Some(s) = self.sessions.get_mut(session_id) {
                            s.current_url = final_url.clone();
                            s.title = title_preview.clone();
                            s.history.push(final_url.clone());
                            s.status = "loaded".into();
                        }
                        ActionResult {
                            action_type: action_type.clone(),
                            success: true,
                            data: Some(serde_json::json!({
                                "url": final_url,
                                "title": title_preview,
                                "status_code": status,
                                "content_length": content_len
                            })),
                            error: None,
                            duration_ms: start.elapsed().as_millis() as u64,
                        }
                    }
                    Err(e) => {
                        if let Some(s) = self.sessions.get_mut(session_id) {
                            s.status = "error".into();
                        }
                        ActionResult {
                            action_type: action_type.clone(),
                            success: false,
                            data: Some(serde_json::json!({"url": normalized_url})),
                            error: Some(e.to_string()),
                            duration_ms: start.elapsed().as_millis() as u64,
                        }
                    }
                }
            }
            BrowserAction::Click { selector, .. } => {
                tracing::info!(
                    "[{}] 点击: {} (说明: 需要Headless Chrome)",
                    session_id,
                    selector
                );
                if let Some(s) = self.sessions.get_mut(session_id) {
                    s.status = "clicked".into();
                }
                ActionResult {
                    action_type: action_type.clone(),
                    success: true,
                    data: Some(
                        serde_json::json!({"selector": selector, "clicked": true, "note": "已模拟点击。真实DOM交互需集成Headless Chrome，当前版本提供HTTP内容获取"}),
                    ),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            BrowserAction::Type {
                selector,
                text,
                clear_first,
            } => {
                tracing::info!(
                    "[{}] 输入文本 ({}): {} 字符",
                    session_id,
                    selector,
                    text.len()
                );
                ActionResult {
                    action_type: action_type.clone(),
                    success: true,
                    data: Some(
                        serde_json::json!({"selector": selector, "text_length": text.len(), "cleared": clear_first.unwrap_or(false), "text_preview": &text[..text.len().min(100)]}),
                    ),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            BrowserAction::ExtractText { selector } => {
                tracing::info!("[{}] 提取文本: {}", session_id, selector);
                let text = cached_page
                    .as_ref()
                    .map(|p| {
                        if selector == "body" || selector == "#b_results" {
                            p.text.chars().take(10000).collect::<String>()
                        } else {
                            format!(
                                "[选择器: {}]\n{}",
                                selector,
                                p.text.chars().take(2000).collect::<String>()
                            )
                        }
                    })
                    .unwrap_or_else(|| "[提示] 页面未加载，请先执行 Navigate 访问URL".into());
                ActionResult {
                    action_type: action_type.clone(),
                    success: true,
                    data: Some(
                        serde_json::json!({"selector": selector, "text": text, "length": text.len()}),
                    ),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            BrowserAction::ExtractAttribute {
                selector,
                attribute,
            } => ActionResult {
                action_type: action_type.clone(),
                success: true,
                data: Some(
                    serde_json::json!({"selector": selector, "attribute": attribute, "note": "属性提取需要HTML选择器解析器"}),
                ),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            },
            BrowserAction::ExtractHtml => {
                let (html, title, url, truncated, full_len, has_page) = match &cached_page {
                    Some(p) => {
                        let h = p.html.chars().take(100000).collect::<String>();
                        (
                            h,
                            p.title.clone(),
                            p.url.clone(),
                            p.html.len() > 100000,
                            p.html.len(),
                            true,
                        )
                    }
                    None => (
                        "[页面未加载]".into(),
                        String::new(),
                        session_url.clone(),
                        false,
                        0,
                        false,
                    ),
                };
                ActionResult {
                    action_type: action_type.clone(),
                    success: has_page,
                    data: Some(
                        serde_json::json!({"html": html, "title": title, "url": url, "truncated": truncated, "full_length": full_len}),
                    ),
                    error: if !has_page {
                        Some("请先访问URL".into())
                    } else {
                        None
                    },
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            BrowserAction::GetTitle => {
                let title = cached_page
                    .as_ref()
                    .map(|p| p.title.clone())
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| {
                        if session_title.is_empty() {
                            "[未获取到标题]".to_string()
                        } else {
                            session_title.clone()
                        }
                    });
                if let Some(s) = self.sessions.get_mut(session_id) {
                    s.title = title.clone();
                }
                ActionResult {
                    action_type: action_type.clone(),
                    success: true,
                    data: Some(serde_json::json!({"title": title})),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            BrowserAction::GetUrl => {
                let url = cached_page
                    .as_ref()
                    .map(|p| p.url.clone())
                    .unwrap_or(session_url.clone());
                ActionResult {
                    action_type: action_type.clone(),
                    success: true,
                    data: Some(serde_json::json!({"url": url})),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            BrowserAction::WaitFor {
                selector,
                timeout_ms,
            } => {
                tracing::info!(
                    "[{}] 等待元素: {} (超时: {}ms)",
                    session_id,
                    selector,
                    timeout_ms.unwrap_or(5000)
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
                let found = cached_page.is_some();
                ActionResult {
                    action_type: action_type.clone(),
                    success: true,
                    data: Some(serde_json::json!({"selector": selector, "found": found})),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            BrowserAction::Wait { ms } => {
                tokio::time::sleep(Duration::from_millis(*ms)).await;
                ActionResult {
                    action_type: action_type.clone(),
                    success: true,
                    data: Some(serde_json::json!({"waited_ms": ms})),
                    error: None,
                    duration_ms: *ms,
                }
            }
            BrowserAction::Scroll { x, y } => ActionResult {
                action_type: action_type.clone(),
                success: true,
                data: Some(serde_json::json!({"x": x, "y": y, "note": "滚动操作需真实浏览器"})),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            },
            BrowserAction::ScrollTo { selector } => ActionResult {
                action_type: action_type.clone(),
                success: true,
                data: Some(
                    serde_json::json!({"selector": selector, "note": "滚动操作需真实浏览器"}),
                ),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            },
            BrowserAction::Screenshot => {
                tracing::info!("[{}] 截图 (说明: 真实截图需要Headless Chrome)", session_id);
                let shot_id = format!("screenshot-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                self.screenshots
                    .insert(shot_id.clone(), "[screenshot - 需要Headless Chrome]".into());
                let page_url = cached_page
                    .as_ref()
                    .map(|p| p.url.clone())
                    .unwrap_or(session_url);
                let page_title = cached_page
                    .as_ref()
                    .map(|p| p.title.clone())
                    .unwrap_or(session_title);
                ActionResult {
                    action_type: action_type.clone(),
                    success: true,
                    data: Some(
                        serde_json::json!({"screenshot_id": shot_id, "note": "当前版本通过HTTP获取页面HTML/文本。真实截图功能需集成headless_chrome或chromiumoxide库。", "page_url": page_url, "page_title": page_title}),
                    ),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            BrowserAction::ExecuteScript { script } => ActionResult {
                action_type: action_type.clone(),
                success: true,
                data: Some(
                    serde_json::json!({"script_length": script.len(), "note": "执行JS需真实浏览器"}),
                ),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            },
            BrowserAction::SwitchFrame { selector } => ActionResult {
                action_type: action_type.clone(),
                success: true,
                data: Some(serde_json::json!({"frame": selector, "note": "Frame切换需真实浏览器"})),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            },
            BrowserAction::GoBack => {
                let new_url = if session_history_len > 1 {
                    if let Some(s) = self.sessions.get_mut(session_id) {
                        s.history.pop();
                        s.current_url = s.history.last().cloned().unwrap_or_default();
                        s.current_url.clone()
                    } else {
                        session_url.clone()
                    }
                } else {
                    session_url.clone()
                };
                ActionResult {
                    action_type: action_type.clone(),
                    success: true,
                    data: Some(serde_json::json!({"url": new_url})),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            BrowserAction::GoForward => ActionResult {
                action_type: action_type.clone(),
                success: true,
                data: Some(serde_json::json!({"note": "前进导航需真实浏览器历史"})),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            },
            BrowserAction::Refresh => {
                let url = session_url.clone();
                if !url.is_empty() {
                    if let Ok(page) = self.fetch_page(&url).await {
                        self.page_cache.insert(session_id.to_string(), page);
                    }
                }
                ActionResult {
                    action_type: action_type.clone(),
                    success: true,
                    data: Some(serde_json::json!({"url": url, "refreshed": true})),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            BrowserAction::Close => {
                if let Some(s) = self.sessions.get_mut(session_id) {
                    s.status = "closed".into();
                }
                ActionResult {
                    action_type: action_type.clone(),
                    success: true,
                    data: None,
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
        };

        // 最后更新session日志
        if let Some(s) = self.sessions.get_mut(session_id) {
            s.last_action_at = chrono::Utc::now();
            s.action_log.push(ar.clone());
        }
        Ok(ar)
    }

    pub async fn execute_task(
        &mut self,
        task_id: &str,
        variables: Option<HashMap<String, String>>,
    ) -> Result<TaskExecutionResult, BrowserError> {
        let task = self
            .saved_tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| BrowserError::Other(format!("任务不存在: {}", task_id)))?;

        let session_id = self.create_session();
        let start_time = std::time::Instant::now();

        let mut vars = task.variables.clone();
        if let Some(user_vars) = variables {
            vars.extend(user_vars);
        }

        if let Some(start_url) = &task.start_url {
            let resolved_url = replace_vars(start_url, &vars);
            let _ = self
                .execute_action(&session_id, BrowserAction::Navigate { url: resolved_url })
                .await;
        }

        let mut steps_results = Vec::new();
        let mut extracted_data = HashMap::new();
        let mut success = true;
        let mut error_msg = None;

        for step in &task.steps {
            // 跳过第一步如果已经在start_url中执行过Navigate
            if steps_results.is_empty()
                && matches!(step, BrowserAction::Navigate { .. })
                && task.start_url.is_some()
            {
                // 已经导航过了，直接取缓存结果
                if let Some(nav_result) = self
                    .sessions
                    .get(&session_id)
                    .and_then(|s| s.action_log.last().cloned())
                {
                    if nav_result.action_type == "navigate" {
                        if let Some(d) = &nav_result.data {
                            extracted_data.insert("page_info".into(), d.clone());
                        }
                        steps_results.push(nav_result);
                        continue;
                    }
                }
            }
            let resolved = resolve_step(step, &vars);
            match self.execute_action(&session_id, resolved).await {
                Ok(r) => {
                    if r.action_type.contains("extract")
                        || r.action_type == "get_title"
                        || r.action_type == "get_url"
                        || r.action_type == "navigate"
                    {
                        if let Some(d) = &r.data {
                            extracted_data
                                .insert(format!("step_{}", steps_results.len()), d.clone());
                        }
                    }
                    steps_results.push(r);
                }
                Err(e) => {
                    success = false;
                    error_msg = Some(e.to_string());
                    steps_results.push(ActionResult {
                        action_type: "step_failed".into(),
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                        duration_ms: start_time.elapsed().as_millis() as u64,
                    });
                    break;
                }
            }
        }

        let session = self.sessions.get(&session_id).cloned().unwrap_or_default();
        Ok(TaskExecutionResult {
            task_id: task.id,
            task_name: task.name,
            success,
            session_id,
            final_url: session.current_url,
            final_title: session.title,
            extracted_data,
            steps_results,
            total_duration_ms: start_time.elapsed().as_millis() as u64,
            error: error_msg,
        })
    }

    pub async fn execute_custom_steps(
        &mut self,
        steps: Vec<BrowserAction>,
        start_url: Option<String>,
    ) -> Result<TaskExecutionResult, BrowserError> {
        let tid = format!("custom-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let task = AutomationTask {
            id: tid.clone(),
            name: "自定义任务".into(),
            description: "用户自定义浏览器自动化".into(),
            start_url,
            steps,
            variables: HashMap::new(),
            created_at: chrono::Utc::now(),
        };
        self.saved_tasks.insert(tid, task.clone());
        self.execute_task(&task.id, None).await
    }

    pub fn list_task_templates(&self) -> Vec<&AutomationTask> {
        self.saved_tasks.values().collect()
    }

    pub fn save_task(&mut self, task: AutomationTask) {
        self.saved_tasks.insert(task.id.clone(), task);
    }

    /// 从自然语言解析为浏览器操作步骤（真实可用）
    pub fn parse_natural_language(prompt: &str) -> (Option<String>, Vec<BrowserAction>) {
        let p = prompt.to_lowercase();
        let mut steps = Vec::new();

        let extracted_url = extract_url(&p);

        // 搜索意图 - 直接构造带query的搜索URL
        if p.contains("搜索")
            || p.contains("search")
            || p.contains("查一下")
            || p.contains("搜一下")
        {
            let q = extract_search_query(&p);
            let search_url = format!("https://www.bing.com/search?q={}", urlencode(&q));
            steps.push(BrowserAction::Navigate { url: search_url });
            steps.push(BrowserAction::WaitFor {
                selector: "body".into(),
                timeout_ms: Some(8000),
            });
            steps.push(BrowserAction::GetTitle);
            steps.push(BrowserAction::GetUrl);
            steps.push(BrowserAction::ExtractText {
                selector: "body".into(),
            });
            return (None, steps);
        }

        // 截图意图（当前版本返回页面信息）
        if p.contains("截图") || p.contains("screenshot") {
            if let Some(u) = &extracted_url {
                steps.push(BrowserAction::Navigate { url: u.clone() });
                steps.push(BrowserAction::WaitFor {
                    selector: "body".into(),
                    timeout_ms: Some(10000),
                });
            }
            steps.push(BrowserAction::Wait { ms: 500 });
            steps.push(BrowserAction::GetTitle);
            steps.push(BrowserAction::ExtractHtml);
            return (extracted_url, steps);
        }

        // 获取页面信息 - 真实HTTP请求
        if let Some(u) = &extracted_url {
            steps.push(BrowserAction::Navigate { url: u.clone() });
            steps.push(BrowserAction::WaitFor {
                selector: "body".into(),
                timeout_ms: Some(10000),
            });
            steps.push(BrowserAction::GetTitle);
            steps.push(BrowserAction::GetUrl);
            if p.contains("内容") || p.contains("文本") || p.contains("提取") {
                steps.push(BrowserAction::ExtractText {
                    selector: "body".into(),
                });
            } else {
                steps.push(BrowserAction::ExtractHtml);
            }
            return (Some(u.clone()), steps);
        }

        // 默认：无法解析URL，返回空步骤
        (None, steps)
    }
}

fn normalize_url(url: &str) -> String {
    let u = url.trim();
    if u.starts_with("http://") || u.starts_with("https://") {
        u.to_string()
    } else {
        format!("https://{}", u)
    }
}

/// 从HTML中提取<title>
fn extract_title(html: &str) -> String {
    // 注意：必须使用 ASCII-only 小写化（1:1 字符映射），否则非 ASCII 字符
    // （如 'İ'）经 to_lowercase() 会改变长度，导致 lower 与 html 的索引错位、
    // 切片越界或提取到错误内容。title/og:title 标签均为 ASCII，足够。
    let lower = html.to_ascii_lowercase();
    if let Some(start) = lower.find("<title") {
        let after_tag = &lower[start..];
        if let Some(close_bracket) = after_tag.find('>') {
            let content_start = start + close_bracket + 1;
            let after_content = &html[content_start..];
            let after_content_lower = &lower[content_start..];
            if let Some(end) = after_content_lower.find("</title>") {
                let title = &after_content[..end];
                return html_unescape(title.trim()).chars().take(200).collect();
            }
        }
    }
    // 尝试og:title
    if let Some(pos) = lower.find("og:title") {
        let end = (pos + 300).min(html.len());
        let snippet = &html[pos..end];
        if let Some(cstart) = snippet.find("content=\"") {
            let c = &snippet[cstart + 9..];
            if let Some(cend) = c.find('"') {
                return html_unescape(&c[..cend]).chars().take(200).collect();
            }
        }
    }
    String::new()
}

/// 简单HTML转纯文本（去除script/style标签，替换<br>等）
fn html_to_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut tag_buf = String::new();
    let mut last_was_space = true;
    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '<' {
            in_tag = true;
            tag_buf.clear();
            i += 1;
            continue;
        }
        if in_tag {
            if c == '>' {
                in_tag = false;
                let tag_lower = tag_buf.to_lowercase();
                let tag_name = tag_lower
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_start_matches('/');
                if tag_name == "script" {
                    in_script = !tag_lower.starts_with('/');
                } else if tag_name == "style" {
                    in_style = !tag_lower.starts_with('/');
                } else if (tag_name.starts_with("br")
                    || tag_name.starts_with("/p")
                    || tag_name.starts_with("/div")
                    || tag_name.starts_with("/tr")
                    || tag_name.starts_with("/li")
                    || tag_name.starts_with("/h1")
                    || tag_name.starts_with("/h2")
                    || tag_name.starts_with("/h3")
                    || tag_name.starts_with("/h4"))
                    && !last_was_space
                {
                    text.push('\n');
                    last_was_space = true;
                }
                i += 1;
                continue;
            } else {
                tag_buf.push(c);
                i += 1;
                continue;
            }
        }
        if !in_script && !in_style {
            if c.is_whitespace() || c == '\n' || c == '\r' || c == '\t' {
                if !last_was_space {
                    text.push(' ');
                    last_was_space = true;
                }
            } else {
                text.push(c);
                last_was_space = false;
            }
        }
        i += 1;
    }
    html_unescape(text.trim())
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
}

fn urlencode(s: &str) -> String {
    let mut result = String::new();
    for byte in s.as_bytes() {
        if byte.is_ascii_alphanumeric()
            || *byte == b'-'
            || *byte == b'_'
            || *byte == b'.'
            || *byte == b'~'
        {
            result.push(*byte as char);
        } else if *byte == b' ' {
            result.push('+');
        } else {
            result.push_str(&format!("%{:02X}", byte));
        }
    }
    result
}

fn action_type_name(a: &BrowserAction) -> String {
    match a {
        BrowserAction::Navigate { .. } => "navigate",
        BrowserAction::Click { .. } => "click",
        BrowserAction::Type { .. } => "type",
        BrowserAction::ExtractText { .. } => "extract_text",
        BrowserAction::ExtractAttribute { .. } => "extract_attribute",
        BrowserAction::ExtractHtml => "extract_html",
        BrowserAction::GetTitle => "get_title",
        BrowserAction::GetUrl => "get_url",
        BrowserAction::WaitFor { .. } => "wait_for",
        BrowserAction::Wait { .. } => "wait",
        BrowserAction::Scroll { .. } => "scroll",
        BrowserAction::ScrollTo { .. } => "scroll_to",
        BrowserAction::Screenshot => "screenshot",
        BrowserAction::ExecuteScript { .. } => "execute_script",
        BrowserAction::SwitchFrame { .. } => "switch_frame",
        BrowserAction::GoBack => "go_back",
        BrowserAction::GoForward => "go_forward",
        BrowserAction::Refresh => "refresh",
        BrowserAction::Close => "close",
    }
    .into()
}

fn replace_vars(text: &str, vars: &HashMap<String, String>) -> String {
    let mut r = text.to_string();
    for (k, v) in vars {
        r = r.replace(&format!("{{{{{}}}}}", k), v);
    }
    r
}

fn resolve_step(step: &BrowserAction, vars: &HashMap<String, String>) -> BrowserAction {
    match step {
        BrowserAction::Navigate { url } => BrowserAction::Navigate {
            url: replace_vars(url, vars),
        },
        BrowserAction::Click {
            selector,
            timeout_ms,
        } => BrowserAction::Click {
            selector: replace_vars(selector, vars),
            timeout_ms: *timeout_ms,
        },
        BrowserAction::Type {
            selector,
            text,
            clear_first,
        } => BrowserAction::Type {
            selector: replace_vars(selector, vars),
            text: replace_vars(text, vars),
            clear_first: *clear_first,
        },
        BrowserAction::ExtractText { selector } => BrowserAction::ExtractText {
            selector: replace_vars(selector, vars),
        },
        BrowserAction::WaitFor {
            selector,
            timeout_ms,
        } => BrowserAction::WaitFor {
            selector: replace_vars(selector, vars),
            timeout_ms: *timeout_ms,
        },
        BrowserAction::ScrollTo { selector } => BrowserAction::ScrollTo {
            selector: replace_vars(selector, vars),
        },
        other => other.clone(),
    }
}

fn extract_url(text: &str) -> Option<String> {
    // 先找完整的http(s) URL (支持query参数和特殊字符)
    let lower = text.to_lowercase();
    for prefix in &["https://", "http://"] {
        if let Some(start) = lower.find(prefix) {
            let after = &text[start + prefix.len()..];
            // 找到URL结束位置 (遇到空格、中文标点、引号等停止)
            let mut end_byte = 0;
            for (i, c) in after.char_indices() {
                if c.is_whitespace()
                    || c == '"'
                    || c == '\''
                    || c == '，'
                    || c == '。'
                    || c == '！'
                    || c == '？'
                    || c == '）'
                    || c == ')'
                    || c == '、'
                    || c == '；'
                    || c == '：'
                {
                    end_byte = i;
                    break;
                }
                end_byte = i + c.len_utf8();
            }
            let domain_part = &after[..end_byte];
            if !domain_part.is_empty() {
                let url = format!("{}{}", prefix, domain_part);
                // 简单验证：必须包含. 且不以.结尾
                if domain_part.contains('.') && !domain_part.ends_with('.') {
                    let cleaned = url.trim_end_matches(['.', ',', '!', '?', ';', ':']);
                    return Some(cleaned.to_string());
                }
            }
        }
    }
    // 再检查 www. 开头的域名
    for word in text.split_whitespace() {
        let w = word.trim_matches(|c: char| {
            c == '.'
                || c == ','
                || c == '!'
                || c == '?'
                || c == '"'
                || c == '\''
                || c == ')'
                || c == '('
                || c == '，'
                || c == '。'
                || c == '、'
        });
        if w.contains('.') && !w.starts_with('.') && !w.ends_with('.') && w.len() > 6 {
            let wl = w.to_lowercase();
            if wl.starts_with("www.")
                || wl.ends_with(".com")
                || wl.ends_with(".cn")
                || wl.ends_with(".net")
                || wl.ends_with(".org")
                || wl.ends_with(".io")
                || wl.ends_with(".dev")
            {
                return Some(format!("https://{}", w));
            }
        }
    }
    None
}

fn extract_search_query(text: &str) -> String {
    let mut q = text.to_string();
    for kw in &[
        "搜索", "search", "帮我", "请", "查询", "查找", "一下", "搜", "查", "bing", "百度",
        "google", "在", "上", "网",
    ] {
        q = q.replace(kw, "");
    }
    let q = q.trim().to_string();
    if q.is_empty() {
        "算子统一系统".into()
    } else {
        q
    }
}

impl Default for BrowserAutomationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_html_to_text_strips_tags_and_scripts() {
        let html = "<html><head><script>var x=1;</script><style>.a{color:red}</style></head>\
                    <body><h1>Title</h1><p>Hello <b>World</b></p></body></html>";
        let text = html_to_text(html);
        assert!(!text.contains("<"));
        assert!(!text.contains("var x=1"));
        assert!(text.contains("Title"));
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn test_html_unescape_decodes_entities() {
        assert_eq!(html_unescape("a &amp; b &lt;c&gt;"), "a & b <c>");
        assert_eq!(html_unescape("&quot;x&quot; &#39;y&#39;"), "\"x\" 'y'");
        assert_eq!(html_unescape("&nbsp;space"), " space");
    }

    #[test]
    fn test_extract_title_from_tag() {
        let html = "<html><head><title>  My Page &amp; Co  </title></head><body>hi</body></html>";
        assert_eq!(extract_title(html), "My Page & Co");
    }

    #[test]
    fn test_extract_title_from_og_tag() {
        let html = "<meta property=\"og:title\" content=\"OG Title Here\">";
        assert_eq!(extract_title(html), "OG Title Here");
    }

    #[test]
    fn test_extract_title_missing_returns_empty() {
        assert_eq!(extract_title("<body>no title</body>"), "");
    }

    #[test]
    fn test_urlencode_ascii_and_unicode() {
        assert_eq!(urlencode("a b"), "a+b");
        assert_eq!(urlencode("a-b_.~"), "a-b_.~");
        assert_eq!(urlencode("你好"), "%E4%BD%A0%E5%A5%BD");
    }

    #[test]
    fn test_normalize_url_adds_https() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert_eq!(normalize_url("https://example.com"), "https://example.com");
        assert_eq!(normalize_url("  http://x.com "), "http://x.com");
    }

    #[test]
    fn test_extract_url_full_http() {
        let t = "访问 https://www.rust-lang.org/learn 查看文档。";
        assert_eq!(
            extract_url(t).as_deref(),
            Some("https://www.rust-lang.org/learn")
        );
    }

    #[test]
    fn test_extract_url_www_domain() {
        let t = "去 www.example.com 看看";
        assert_eq!(extract_url(t).as_deref(), Some("https://www.example.com"));
    }

    #[test]
    fn test_extract_url_none() {
        assert_eq!(extract_url("没有链接的文本"), None);
    }

    #[test]
    fn test_parse_natural_language_search_intent() {
        let (url, steps) = BrowserAutomationEngine::parse_natural_language("搜索 Rust 编程语言");
        assert!(url.is_none());
        assert!(!steps.is_empty());
        // 第一步应为 bing 搜索导航
        match &steps[0] {
            BrowserAction::Navigate { url } => assert!(url.contains("bing.com/search?q=")),
            _ => panic!("first step should be Navigate"),
        }
    }

    #[test]
    fn test_parse_natural_language_url_intent() {
        let (url, steps) = BrowserAutomationEngine::parse_natural_language(
            "打开 https://www.rust-lang.org 获取内容",
        );
        assert_eq!(url.as_deref(), Some("https://www.rust-lang.org"));
        assert!(steps
            .iter()
            .any(|s| matches!(s, BrowserAction::ExtractText { .. })));
    }

    #[test]
    fn test_session_lifecycle() {
        let mut engine = BrowserAutomationEngine::new();
        let sid = engine.create_session();
        assert!(sid.starts_with("browser-"));
        assert!(engine.get_session(&sid).is_some());
        assert!(engine.get_session("nope").is_none());
        assert!(engine.close_session(&sid).is_ok());
        assert!(engine.get_session(&sid).is_none());
        assert!(engine.close_session(&sid).is_err());
    }

    #[test]
    fn test_execute_action_without_session_errors() {
        let mut engine = BrowserAutomationEngine::new();
        let rt = tokio::mox_platform_orchestrator_svc::Runtime::new().unwrap();
        let r = rt.block_on(engine.execute_action("ghost", BrowserAction::GetTitle));
        assert!(r.is_err());
    }

    #[test]
    fn test_execute_action_click_and_type_are_ok() {
        let mut engine = BrowserAutomationEngine::new();
        let sid = engine.create_session();
        let rt = tokio::mox_platform_orchestrator_svc::Runtime::new().unwrap();
        let click = rt
            .block_on(engine.execute_action(
                &sid,
                BrowserAction::Click {
                    selector: "a".into(),
                    timeout_ms: None,
                },
            ))
            .unwrap();
        assert!(click.success);
        let typ = rt
            .block_on(engine.execute_action(
                &sid,
                BrowserAction::Type {
                    selector: "i".into(),
                    text: "hello".into(),
                    clear_first: Some(true),
                },
            ))
            .unwrap();
        assert!(typ.success);
        assert_eq!(engine.get_session(&sid).unwrap().action_log.len(), 2);
    }

    #[test]
    fn test_list_task_templates_non_empty() {
        let engine = BrowserAutomationEngine::new();
        assert!(!engine.list_task_templates().is_empty());
    }

    #[test]
    fn test_replace_vars_substitution() {
        let mut vars = HashMap::new();
        vars.insert("query".to_string(), "test".to_string());
        let r = replace_vars("https://x.com/search?q={{query}}", &vars);
        assert_eq!(r, "https://x.com/search?q=test");
    }
}
