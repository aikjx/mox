//! 定时任务调度 API 端点
//!
//! 提供：任务CRUD / 启用禁用 / 手动触发 / 执行历史 / 任务统计 / 模板列表

use crate::enterprise::api_response::*;
use crate::scheduler::*;
use axum::{
    extract::{Path, Query, State},
    response::Response,
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 定时任务调度状态
pub struct SchedulerState {
    /// 任务定义存储
    pub tasks: Arc<RwLock<HashMap<String, ScheduledTask>>>,
    /// 任务执行记录存储
    pub execution_records: Arc<RwLock<Vec<TaskExecutionRecord>>>,
    /// 运行中的任务（task_id -> execution_id）
    pub running_tasks: Arc<RwLock<HashMap<String, String>>>,
    /// 调度器是否运行中
    pub scheduler_running: Arc<RwLock<bool>>,
}

impl SchedulerState {
    pub fn new() -> Self {
        let mut tasks = HashMap::new();
        // 预置内置任务模板
        for tpl in builtin_task_templates() {
            tasks.insert(tpl.task_id.clone(), tpl);
        }
        Self {
            tasks: Arc::new(RwLock::new(tasks)),
            execution_records: Arc::new(RwLock::new(Vec::new())),
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            scheduler_running: Arc::new(RwLock::new(true)),
        }
    }
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /api/enterprise/scheduler/task-categories —— 获取任务分类列表
pub async fn list_task_categories_handler() -> Response {
    let categories = supported_task_categories();
    let result: Vec<serde_json::Value> = categories.iter().map(|(code, name)| {
        json!({ "code": code, "name": name })
    }).collect();
    success(result)
}

/// GET /api/enterprise/scheduler/handler-types —— 获取任务处理器类型列表
pub async fn list_handler_types_handler() -> Response {
    let handlers = supported_handler_types();
    let result: Vec<serde_json::Value> = handlers.iter().map(|(code, name)| {
        json!({ "code": code, "name": name })
    }).collect();
    success(result)
}

/// GET /api/enterprise/scheduler/templates —— 获取内置任务模板
pub async fn list_task_templates_handler() -> Response {
    let templates = builtin_task_templates();
    success_with_message("success", json!(templates))
}

/// GET /api/enterprise/scheduler/tasks —— 获取任务列表
pub async fn list_tasks_handler(
    State(state): State<Arc<SchedulerState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let tasks = state.tasks.read().await;
    let mut list: Vec<&ScheduledTask> = tasks.values().collect();

    // 按分类过滤
    if let Some(category) = params.get("category") {
        list.retain(|t| t.category == *category);
    }
    // 按类型过滤
    if let Some(task_type) = params.get("task_type") {
        list.retain(|t| t.task_type.as_str() == *task_type);
    }
    // 按启用状态过滤
    if let Some(enabled) = params.get("enabled") {
        let enabled_bool = enabled == "true";
        list.retain(|t| t.enabled == enabled_bool);
    }
    // 按状态过滤
    if let Some(status) = params.get("status") {
        list.retain(|t| {
            if *status == "running" {
                state.running_tasks.try_read().map(|r| r.contains_key(&t.task_id)).unwrap_or(false)
            } else {
                true
            }
        });
    }

    // 按优先级排序（高优先级在前）
    list.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.task_name.cmp(&b.task_name)));

    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, list.len());
    let page_items = pagination.paginate(&list);
    success_list(page_items, &pagination)
}

/// POST /api/enterprise/scheduler/tasks —— 创建定时任务
pub async fn create_task_handler(
    State(state): State<Arc<SchedulerState>>,
    Json(req): Json<CreateScheduledTaskRequest>,
) -> Response {
    // 输入验证
    if let Err(e) = validate_required("task_name", &req.task_name) {
        return e;
    }
    if let Err(e) = validate_required("handler_type", &req.handler_type) {
        return e;
    }
    // 验证任务类型必填字段
    match req.task_type {
        TaskType::Cron => {
            if req.cron_expression.as_deref().unwrap_or("").is_empty() {
                return bad_request("CRON任务必须提供cron_expression");
            }
        }
        TaskType::Interval => {
            if req.interval_seconds.unwrap_or(0) == 0 {
                return bad_request("间隔任务必须提供interval_seconds（大于0）");
            }
        }
        TaskType::Once => {
            if req.execute_at.as_deref().unwrap_or("").is_empty() {
                return bad_request("一次性任务必须提供execute_at");
            }
        }
        TaskType::Manual => {}
    }

    let task_id = format!("task_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();
    let task = ScheduledTask {
        task_id: task_id.clone(),
        tenant_id: "default".to_string(),
        task_name: req.task_name,
        description: req.description,
        task_type: req.task_type,
        category: req.category,
        cron_expression: req.cron_expression,
        interval_seconds: req.interval_seconds,
        execute_at: req.execute_at,
        handler_type: req.handler_type,
        params: req.params,
        priority: req.priority.unwrap_or(TaskPriority::Normal),
        enabled: req.enabled.unwrap_or(true),
        max_retry: req.max_retry.unwrap_or(3),
        retry_interval_seconds: req.retry_interval_seconds.unwrap_or(60),
        timeout_seconds: req.timeout_seconds.unwrap_or(300),
        allow_concurrent: req.allow_concurrent.unwrap_or(false),
        last_executed_at: None,
        next_execution_at: None,
        execution_count: 0,
        success_count: 0,
        failure_count: 0,
        avg_duration_ms: None,
        created_by: "system".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    state.tasks.write().await.insert(task_id.clone(), task);
    success_with_message("定时任务创建成功", json!({ "task_id": task_id }))
}

/// GET /api/enterprise/scheduler/tasks/:id —— 获取任务详情
pub async fn get_task_handler(
    State(state): State<Arc<SchedulerState>>,
    Path(id): Path<String>,
) -> Response {
    let tasks = state.tasks.read().await;
    match tasks.get(&id) {
        Some(t) => success(t),
        None => not_found("定时任务不存在"),
    }
}

/// PUT /api/enterprise/scheduler/tasks/:id —— 更新定时任务
pub async fn update_task_handler(
    State(state): State<Arc<SchedulerState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateScheduledTaskRequest>,
) -> Response {
    let mut tasks = state.tasks.write().await;
    match tasks.get_mut(&id) {
        Some(t) => {
            if let Some(name) = req.task_name { t.task_name = name; }
            if let Some(desc) = req.description { t.description = Some(desc); }
            if let Some(tt) = req.task_type { t.task_type = tt; }
            if let Some(cat) = req.category { t.category = cat; }
            if let Some(cron) = req.cron_expression { t.cron_expression = Some(cron); }
            if let Some(interval) = req.interval_seconds { t.interval_seconds = Some(interval); }
            if let Some(execute_at) = req.execute_at { t.execute_at = Some(execute_at); }
            if let Some(handler) = req.handler_type { t.handler_type = handler; }
            if let Some(params) = req.params { t.params = Some(params); }
            if let Some(priority) = req.priority { t.priority = priority; }
            if let Some(max_retry) = req.max_retry { t.max_retry = max_retry; }
            if let Some(retry_interval) = req.retry_interval_seconds { t.retry_interval_seconds = retry_interval; }
            if let Some(timeout) = req.timeout_seconds { t.timeout_seconds = timeout; }
            if let Some(allow_concurrent) = req.allow_concurrent { t.allow_concurrent = allow_concurrent; }
            t.updated_at = chrono::Utc::now().to_rfc3339();
            success_message("定时任务更新成功")
        }
        None => not_found("定时任务不存在"),
    }
}

/// DELETE /api/enterprise/scheduler/tasks/:id —— 删除定时任务
pub async fn delete_task_handler(
    State(state): State<Arc<SchedulerState>>,
    Path(id): Path<String>,
) -> Response {
    let mut tasks = state.tasks.write().await;
    if tasks.remove(&id).is_some() {
        success_message("定时任务删除成功")
    } else {
        not_found("定时任务不存在")
    }
}

/// POST /api/enterprise/scheduler/tasks/:id/enable —— 启用任务
pub async fn enable_task_handler(
    State(state): State<Arc<SchedulerState>>,
    Path(id): Path<String>,
) -> Response {
    let mut tasks = state.tasks.write().await;
    match tasks.get_mut(&id) {
        Some(t) => {
            t.enabled = true;
            t.updated_at = chrono::Utc::now().to_rfc3339();
            success_message("任务已启用")
        }
        None => not_found("定时任务不存在"),
    }
}

/// POST /api/enterprise/scheduler/tasks/:id/disable —— 禁用任务
pub async fn disable_task_handler(
    State(state): State<Arc<SchedulerState>>,
    Path(id): Path<String>,
) -> Response {
    let mut tasks = state.tasks.write().await;
    match tasks.get_mut(&id) {
        Some(t) => {
            t.enabled = false;
            t.updated_at = chrono::Utc::now().to_rfc3339();
            success_message("任务已禁用")
        }
        None => not_found("定时任务不存在"),
    }
}

/// POST /api/enterprise/scheduler/tasks/:id/trigger —— 手动触发任务
pub async fn trigger_task_handler(
    State(state): State<Arc<SchedulerState>>,
    Path(id): Path<String>,
    Json(_req): Json<TriggerTaskRequest>,
) -> Response {
    let tasks = state.tasks.read().await;
    let task = match tasks.get(&id) {
        Some(t) => t.clone(),
        None => return not_found("定时任务不存在"),
    };
    drop(tasks);

    // 检查是否允许并发
    if !task.allow_concurrent {
        let running = state.running_tasks.read().await;
        if running.contains_key(&id) {
            return conflict("任务正在运行中，不允许并发执行");
        }
    }

    // 创建执行记录
    let execution_id = format!("exec_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();
    let record = TaskExecutionRecord {
        execution_id: execution_id.clone(),
        task_id: id.clone(),
        tenant_id: task.tenant_id.clone(),
        trigger_type: "manual".to_string(),
        status: TaskStatus::Running,
        started_at: now.clone(),
        finished_at: None,
        duration_ms: None,
        retry_count: 0,
        result_data: None,
        error_message: None,
        error_stack: None,
        execution_node: Some("local".to_string()),
        triggered_by: Some("system".to_string()),
    };
    state.execution_records.write().await.push(record);
    state.running_tasks.write().await.insert(id.clone(), execution_id.clone());

    // 模拟执行（实际实现中应调用任务处理器）
    // 这里直接标记为成功
    let mut records = state.execution_records.write().await;
    if let Some(rec) = records.iter_mut().find(|r| r.execution_id == execution_id) {
        rec.status = TaskStatus::Completed;
        rec.finished_at = Some(chrono::Utc::now().to_rfc3339());
        rec.duration_ms = Some(120);
        rec.result_data = Some(json!({ "message": "任务执行成功", "handler": task.handler_type }));
    }
    drop(records);

    // 更新任务统计
    let mut tasks = state.tasks.write().await;
    if let Some(t) = tasks.get_mut(&id) {
        t.last_executed_at = Some(chrono::Utc::now().to_rfc3339());
        t.execution_count += 1;
        t.success_count += 1;
    }
    drop(tasks);

    state.running_tasks.write().await.remove(&id);

    success_with_message("任务触发成功", json!({ "execution_id": execution_id, "task_id": id }))
}

/// GET /api/enterprise/scheduler/tasks/:id/executions —— 获取任务执行历史
pub async fn list_task_executions_handler(
    State(state): State<Arc<SchedulerState>>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let records = state.execution_records.read().await;
    let mut list: Vec<&TaskExecutionRecord> = records.iter()
        .filter(|r| r.task_id == id)
        .collect();

    // 按状态过滤
    if let Some(status) = params.get("status") {
        list.retain(|r| r.status.as_str() == *status);
    }

    // 按时间倒序
    list.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, list.len());
    let page_items = pagination.paginate(&list);
    success_list(page_items, &pagination)
}

/// GET /api/enterprise/scheduler/executions —— 获取所有执行记录
pub async fn list_all_executions_handler(
    State(state): State<Arc<SchedulerState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let records = state.execution_records.read().await;
    let mut list: Vec<&TaskExecutionRecord> = records.iter().collect();

    if let Some(status) = params.get("status") {
        list.retain(|r| r.status.as_str() == *status);
    }
    if let Some(trigger_type) = params.get("trigger_type") {
        list.retain(|r| r.trigger_type == *trigger_type);
    }

    list.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, list.len());
    let page_items = pagination.paginate(&list);
    success_list(page_items, &pagination)
}

/// GET /api/enterprise/scheduler/stats —— 获取任务统计
pub async fn scheduler_stats_handler(
    State(state): State<Arc<SchedulerState>>,
) -> Response {
    let tasks = state.tasks.read().await;
    let records = state.execution_records.read().await;

    let mut stats = TaskStats::default();
    stats.total = tasks.len() as i64;
    stats.enabled = tasks.values().filter(|t| t.enabled).count() as i64;
    stats.disabled = tasks.values().filter(|t| !t.enabled).count() as i64;

    let running = state.running_tasks.read().await;
    stats.running = running.len() as i64;

    // 今日统计
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let today_records: Vec<&TaskExecutionRecord> = records.iter()
        .filter(|r| r.started_at.starts_with(&today))
        .collect();
    stats.today_executions = today_records.len() as i64;
    stats.today_success = today_records.iter().filter(|r| r.status == TaskStatus::Completed).count() as i64;
    stats.today_failures = today_records.iter().filter(|r| r.status == TaskStatus::Failed).count() as i64;

    if stats.today_executions > 0 {
        stats.success_rate = Some((stats.today_success as f64 / stats.today_executions as f64) * 100.0);
    }

    // 平均执行耗时
    let completed: Vec<u64> = records.iter()
        .filter(|r| r.status == TaskStatus::Completed)
        .filter_map(|r| r.duration_ms)
        .collect();
    if !completed.is_empty() {
        stats.avg_duration_ms = Some(completed.iter().sum::<u64>() / completed.len() as u64);
    }

    success(stats)
}

/// 构建定时任务调度路由（泛型版本）
pub fn build_scheduler_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<SchedulerState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post};

    axum::Router::new()
        // 元数据
        .route("/task-categories", get(list_task_categories_handler))
        .route("/handler-types", get(list_handler_types_handler))
        .route("/templates", get(list_task_templates_handler))
        // 任务CRUD
        .route("/tasks", get(list_tasks_handler).post(create_task_handler))
        .route("/tasks/:id", get(get_task_handler).put(update_task_handler).delete(delete_task_handler))
        // 任务操作
        .route("/tasks/:id/enable", post(enable_task_handler))
        .route("/tasks/:id/disable", post(disable_task_handler))
        .route("/tasks/:id/trigger", post(trigger_task_handler))
        // 执行历史
        .route("/tasks/:id/executions", get(list_task_executions_handler))
        .route("/executions", get(list_all_executions_handler))
        // 统计
        .route("/stats", get(scheduler_stats_handler))
}
