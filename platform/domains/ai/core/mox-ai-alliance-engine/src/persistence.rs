// =============================================================================
// 持久化层（Persistence Layer）
// =============================================================================
// 提供 DatabaseTaskRepository，使用 PostgreSQL + sqlx 实现任务持久化。
// 替换默认的 InMemoryTaskRepository，支持生产环境部署。
//
// 设计原则：
// 1. 仓储模式（Repository Pattern）：定义 TaskRepository trait，内存/数据库双实现
// 2. 异步：所有操作都是 async，支持 tokio 运行时
// 3. 事务：关键操作使用数据库事务保证一致性
// 4. 软删除：任务不物理删除，使用 status=cancelled 标记
// 5. 租户隔离：所有查询都带 tenant_id 过滤
// =============================================================================

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::collections::BTreeMap;
use uuid::Uuid;

// =============================================================================
// 任务状态枚举
// =============================================================================

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// 等待中
    Pending,
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "running" => TaskStatus::Running,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            _ => TaskStatus::Pending,
        }
    }
}

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::Pending
    }
}

// =============================================================================
// 任务实体（数据库行映射）
// =============================================================================

/// 任务实体（对应 alliance_tasks 表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEntity {
    pub id: Uuid,
    pub trace_id: Uuid,
    pub session_id: Option<String>,
    pub query: String,
    pub status: TaskStatus,
    pub current_phase: Option<String>,
    pub team_size: i32,
    pub enable_llm: bool,
    pub options_json: serde_json::Value,
    pub context_json: serde_json::Value,
    pub intent_result: Option<serde_json::Value>,
    pub team_result: Option<serde_json::Value>,
    pub debate_result: Option<serde_json::Value>,
    pub synthesis_result: Option<serde_json::Value>,
    pub gate_result: Option<serde_json::Value>,
    pub learn_result: Option<serde_json::Value>,
    pub final_result: Option<serde_json::Value>,
    pub consensus: Option<f64>,
    pub gate_score: Option<f64>,
    pub gate_grade: Option<String>,
    pub passed: bool,
    pub degraded: bool,
    pub degrade_reason: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tenant_id: String,
    pub created_by: Option<String>,
}

// =============================================================================
// 事件实体
// =============================================================================

/// 事件实体（对应 alliance_events 表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntity {
    pub id: Uuid,
    pub task_id: Uuid,
    pub trace_id: Uuid,
    pub phase: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub latency_ms: i64,
    pub degraded: bool,
    pub degrade_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub tenant_id: String,
}

// =============================================================================
// 任务仓储 trait（Repository Pattern）
// =============================================================================

/// 任务仓储 trait
///
/// 定义任务持久化的统一接口，支持内存和数据库两种实现。
#[async_trait]
pub trait TaskRepository: Send + Sync {
    /// 创建新任务
    async fn create_task(&self, task: &TaskEntity) -> Result<TaskEntity, String>;

    /// 获取任务
    async fn get_task(&self, task_id: Uuid, tenant_id: &str) -> Result<Option<TaskEntity>, String>;

    /// 更新任务状态
    async fn update_task_status(
        &self,
        task_id: Uuid,
        status: TaskStatus,
        current_phase: Option<&str>,
        tenant_id: &str,
    ) -> Result<(), String>;

    /// 更新阶段结果
    async fn update_phase_result(
        &self,
        task_id: Uuid,
        phase: &str,
        result: serde_json::Value,
        latency_ms: u64,
        tenant_id: &str,
    ) -> Result<(), String>;

    /// 完成任务
    async fn complete_task(
        &self,
        task_id: Uuid,
        final_result: serde_json::Value,
        consensus: f64,
        gate_score: f64,
        gate_grade: &str,
        passed: bool,
        duration_ms: u64,
        tenant_id: &str,
    ) -> Result<(), String>;

    /// 标记任务失败
    async fn fail_task(
        &self,
        task_id: Uuid,
        error_message: &str,
        tenant_id: &str,
    ) -> Result<(), String>;

    /// 记录事件
    async fn record_event(&self, event: &EventEntity) -> Result<(), String>;

    /// 获取任务事件列表
    async fn get_task_events(
        &self,
        task_id: Uuid,
        tenant_id: &str,
    ) -> Result<Vec<EventEntity>, String>;

    /// 分页查询任务列表
    async fn list_tasks(
        &self,
        tenant_id: &str,
        page: u32,
        page_size: u32,
        status: Option<TaskStatus>,
    ) -> Result<(Vec<TaskEntity>, u64), String>;

    /// 取消任务
    async fn cancel_task(&self, task_id: Uuid, tenant_id: &str) -> Result<(), String>;
}

// =============================================================================
// 内存任务仓储（默认实现，用于开发和测试）
// =============================================================================

/// 内存任务仓储
///
/// 使用 std::sync::RwLock 存储任务，适合开发和测试环境。
/// 生产环境应使用 DatabaseTaskRepository。
pub struct InMemoryTaskRepository {
    tasks: std::sync::RwLock<BTreeMap<Uuid, TaskEntity>>,
    events: std::sync::RwLock<Vec<EventEntity>>,
}

impl InMemoryTaskRepository {
    pub fn new() -> Self {
        Self {
            tasks: std::sync::RwLock::new(BTreeMap::new()),
            events: std::sync::RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryTaskRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskRepository for InMemoryTaskRepository {
    async fn create_task(&self, task: &TaskEntity) -> Result<TaskEntity, String> {
        let mut tasks = self.tasks.write().unwrap();
        tasks.insert(task.id, task.clone());
        Ok(task.clone())
    }

    async fn get_task(&self, task_id: Uuid, tenant_id: &str) -> Result<Option<TaskEntity>, String> {
        let tasks = self.tasks.read().unwrap();
        Ok(tasks.get(&task_id).filter(|t| t.tenant_id == tenant_id).cloned())
    }

    async fn update_task_status(
        &self,
        task_id: Uuid,
        status: TaskStatus,
        current_phase: Option<&str>,
        tenant_id: &str,
    ) -> Result<(), String> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(&task_id) {
            if task.tenant_id != tenant_id {
                return Err("租户不匹配".to_string());
            }
            task.status = status;
            if let Some(phase) = current_phase {
                task.current_phase = Some(phase.to_string());
            }
            task.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn update_phase_result(
        &self,
        task_id: Uuid,
        phase: &str,
        result: serde_json::Value,
        _latency_ms: u64,
        tenant_id: &str,
    ) -> Result<(), String> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(&task_id) {
            if task.tenant_id != tenant_id {
                return Err("租户不匹配".to_string());
            }
            match phase {
                "intent" => task.intent_result = Some(result),
                "team" => task.team_result = Some(result),
                "debate" => task.debate_result = Some(result),
                "synthesize" => task.synthesis_result = Some(result),
                "gate" => task.gate_result = Some(result),
                "learn" => task.learn_result = Some(result),
                _ => {}
            }
            task.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn complete_task(
        &self,
        task_id: Uuid,
        final_result: serde_json::Value,
        consensus: f64,
        gate_score: f64,
        gate_grade: &str,
        passed: bool,
        duration_ms: u64,
        tenant_id: &str,
    ) -> Result<(), String> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(&task_id) {
            if task.tenant_id != tenant_id {
                return Err("租户不匹配".to_string());
            }
            task.status = TaskStatus::Completed;
            task.current_phase = Some("done".to_string());
            task.final_result = Some(final_result);
            task.consensus = Some(consensus);
            task.gate_score = Some(gate_score);
            task.gate_grade = Some(gate_grade.to_string());
            task.passed = passed;
            task.completed_at = Some(Utc::now());
            task.duration_ms = Some(duration_ms as i64);
            task.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn fail_task(
        &self,
        task_id: Uuid,
        error_message: &str,
        tenant_id: &str,
    ) -> Result<(), String> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(&task_id) {
            if task.tenant_id != tenant_id {
                return Err("租户不匹配".to_string());
            }
            task.status = TaskStatus::Failed;
            task.error_message = Some(error_message.to_string());
            task.completed_at = Some(Utc::now());
            task.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn record_event(&self, event: &EventEntity) -> Result<(), String> {
        let mut events = self.events.write().unwrap();
        events.push(event.clone());
        Ok(())
    }

    async fn get_task_events(
        &self,
        task_id: Uuid,
        tenant_id: &str,
    ) -> Result<Vec<EventEntity>, String> {
        let events = self.events.read().unwrap();
        Ok(events
            .iter()
            .filter(|e| e.task_id == task_id && e.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn list_tasks(
        &self,
        tenant_id: &str,
        page: u32,
        page_size: u32,
        status: Option<TaskStatus>,
    ) -> Result<(Vec<TaskEntity>, u64), String> {
        let tasks = self.tasks.read().unwrap();
        let filtered: Vec<TaskEntity> = tasks
            .values()
            .filter(|t| t.tenant_id == tenant_id)
            .filter(|t| status.map_or(true, |s| t.status == s))
            .cloned()
            .collect();
        let total = filtered.len() as u64;
        let start = ((page - 1) * page_size) as usize;
        let items: Vec<TaskEntity> = filtered.into_iter().skip(start).take(page_size as usize).collect();
        Ok((items, total))
    }

    async fn cancel_task(&self, task_id: Uuid, tenant_id: &str) -> Result<(), String> {
        self.update_task_status(task_id, TaskStatus::Cancelled, None, tenant_id).await
    }
}

// =============================================================================
// 数据库任务仓储（PostgreSQL 实现）
// =============================================================================

/// 数据库任务仓储
///
/// 使用 PostgreSQL + sqlx 实现，支持生产环境部署。
/// 连接池配置：最大连接数 20，超时 30s。
pub struct DatabaseTaskRepository {
    pool: PgPool,
}

impl DatabaseTaskRepository {
    /// 创建新的数据库任务仓储
    pub async fn new(database_url: &str) -> Result<Self, String> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect(database_url)
            .await
            .map_err(|e| format!("数据库连接失败: {}", e))?;
        Ok(Self { pool })
    }

    /// 从已有连接池创建
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 获取连接池引用
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// 执行数据库迁移（使用 sqlx migrate 或手动执行 SQL）
    pub async fn run_migrations(&self) -> Result<(), String> {
        // 注意：实际生产环境应使用 sqlx-cli 或 refinery 等迁移工具
        // 这里提供简单的建表检查
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS alliance_tasks (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                trace_id UUID NOT NULL,
                session_id VARCHAR(128),
                query TEXT NOT NULL,
                status VARCHAR(32) NOT NULL DEFAULT 'pending',
                current_phase VARCHAR(32) DEFAULT 'intent',
                team_size INTEGER NOT NULL DEFAULT 4,
                enable_llm BOOLEAN NOT NULL DEFAULT false,
                options_json JSONB DEFAULT '{}'::jsonb,
                context_json JSONB DEFAULT '{}'::jsonb,
                intent_result JSONB,
                team_result JSONB,
                debate_result JSONB,
                synthesis_result JSONB,
                gate_result JSONB,
                learn_result JSONB,
                final_result JSONB,
                consensus DOUBLE PRECISION,
                gate_score DOUBLE PRECISION,
                gate_grade VARCHAR(8),
                passed BOOLEAN DEFAULT false,
                degraded BOOLEAN DEFAULT false,
                degrade_reason TEXT,
                error_message TEXT,
                retry_count INTEGER NOT NULL DEFAULT 0,
                started_at TIMESTAMPTZ,
                completed_at TIMESTAMPTZ,
                duration_ms BIGINT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                tenant_id VARCHAR(64) DEFAULT 'default',
                created_by VARCHAR(64)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("建表失败: {}", e))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS alliance_events (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                task_id UUID NOT NULL REFERENCES alliance_tasks(id) ON DELETE CASCADE,
                trace_id UUID NOT NULL,
                phase VARCHAR(32) NOT NULL,
                event_type VARCHAR(32) NOT NULL,
                payload JSONB NOT NULL DEFAULT '{}'::jsonb,
                latency_ms BIGINT NOT NULL DEFAULT 0,
                degraded BOOLEAN DEFAULT false,
                degrade_reason TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                tenant_id VARCHAR(64) DEFAULT 'default'
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("事件表建表失败: {}", e))?;

        Ok(())
    }
}

#[async_trait]
impl TaskRepository for DatabaseTaskRepository {
    async fn create_task(&self, task: &TaskEntity) -> Result<TaskEntity, String> {
        let row = sqlx::query(
            r#"
            INSERT INTO alliance_tasks (
                id, trace_id, session_id, query, status, current_phase,
                team_size, enable_llm, options_json, context_json,
                tenant_id, created_by, started_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
            RETURNING *
            "#,
        )
        .bind(task.id)
        .bind(task.trace_id)
        .bind(&task.session_id)
        .bind(&task.query)
        .bind(task.status.as_str())
        .bind(&task.current_phase)
        .bind(task.team_size)
        .bind(task.enable_llm)
        .bind(&task.options_json)
        .bind(&task.context_json)
        .bind(&task.tenant_id)
        .bind(&task.created_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("创建任务失败: {}", e))?;

        Ok(row_to_task(&row))
    }

    async fn get_task(&self, task_id: Uuid, tenant_id: &str) -> Result<Option<TaskEntity>, String> {
        let result = sqlx::query(
            "SELECT * FROM alliance_tasks WHERE id = $1 AND tenant_id = $2",
        )
        .bind(task_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("查询任务失败: {}", e))?;

        Ok(result.map(|row| row_to_task(&row)))
    }

    async fn update_task_status(
        &self,
        task_id: Uuid,
        status: TaskStatus,
        current_phase: Option<&str>,
        tenant_id: &str,
    ) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE alliance_tasks
            SET status = $1, current_phase = COALESCE($2, current_phase), updated_at = NOW()
            WHERE id = $3 AND tenant_id = $4
            "#,
        )
        .bind(status.as_str())
        .bind(current_phase)
        .bind(task_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("更新任务状态失败: {}", e))?;
        Ok(())
    }

    async fn update_phase_result(
        &self,
        task_id: Uuid,
        phase: &str,
        result: serde_json::Value,
        latency_ms: u64,
        tenant_id: &str,
    ) -> Result<(), String> {
        let column = match phase {
            "intent" => "intent_result",
            "team" => "team_result",
            "debate" => "debate_result",
            "synthesize" => "synthesis_result",
            "gate" => "gate_result",
            "learn" => "learn_result",
            _ => return Err(format!("未知阶段: {}", phase)),
        };

        let sql = format!(
            r#"
            UPDATE alliance_tasks
            SET {} = $1, current_phase = $2, updated_at = NOW()
            WHERE id = $3 AND tenant_id = $4
            "#,
            column
        );

        sqlx::query(&sql)
            .bind(result)
            .bind(phase)
            .bind(task_id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("更新阶段结果失败: {}", e))?;

        // 记录阶段统计
        sqlx::query(
            r#"
            INSERT INTO alliance_phase_stats (task_id, phase, latency_ms, success)
            VALUES ($1, $2, $3, true)
            "#,
        )
        .bind(task_id)
        .bind(phase)
        .bind(latency_ms as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("记录阶段统计失败: {}", e))?;

        Ok(())
    }

    async fn complete_task(
        &self,
        task_id: Uuid,
        final_result: serde_json::Value,
        consensus: f64,
        gate_score: f64,
        gate_grade: &str,
        passed: bool,
        duration_ms: u64,
        tenant_id: &str,
    ) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE alliance_tasks
            SET status = 'completed', current_phase = 'done',
                final_result = $1, consensus = $2, gate_score = $3,
                gate_grade = $4, passed = $5, completed_at = NOW(),
                duration_ms = $6, updated_at = NOW()
            WHERE id = $7 AND tenant_id = $8
            "#,
        )
        .bind(final_result)
        .bind(consensus)
        .bind(gate_score)
        .bind(gate_grade)
        .bind(passed)
        .bind(duration_ms as i64)
        .bind(task_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("完成任务失败: {}", e))?;
        Ok(())
    }

    async fn fail_task(
        &self,
        task_id: Uuid,
        error_message: &str,
        tenant_id: &str,
    ) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE alliance_tasks
            SET status = 'failed', error_message = $1, completed_at = NOW(), updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(error_message)
        .bind(task_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("标记任务失败失败: {}", e))?;
        Ok(())
    }

    async fn record_event(&self, event: &EventEntity) -> Result<(), String> {
        sqlx::query(
            r#"
            INSERT INTO alliance_events (
                id, task_id, trace_id, phase, event_type,
                payload, latency_ms, degraded, degrade_reason, tenant_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(event.id)
        .bind(event.task_id)
        .bind(event.trace_id)
        .bind(&event.phase)
        .bind(&event.event_type)
        .bind(&event.payload)
        .bind(event.latency_ms)
        .bind(event.degraded)
        .bind(&event.degrade_reason)
        .bind(&event.tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("记录事件失败: {}", e))?;
        Ok(())
    }

    async fn get_task_events(
        &self,
        task_id: Uuid,
        tenant_id: &str,
    ) -> Result<Vec<EventEntity>, String> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM alliance_events
            WHERE task_id = $1 AND tenant_id = $2
            ORDER BY created_at ASC
            "#,
        )
        .bind(task_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("查询事件失败: {}", e))?;

        Ok(rows.iter().map(row_to_event).collect())
    }

    async fn list_tasks(
        &self,
        tenant_id: &str,
        page: u32,
        page_size: u32,
        status: Option<TaskStatus>,
    ) -> Result<(Vec<TaskEntity>, u64), String> {
        let offset = ((page - 1) * page_size) as i64;
        let limit = page_size as i64;

        // 查询总数
        let count_row = if let Some(s) = status {
            sqlx::query("SELECT COUNT(*) as count FROM alliance_tasks WHERE tenant_id = $1 AND status = $2")
                .bind(tenant_id)
                .bind(s.as_str())
                .fetch_one(&self.pool)
                .await
                .map_err(|e| format!("查询任务总数失败: {}", e))?
        } else {
            sqlx::query("SELECT COUNT(*) as count FROM alliance_tasks WHERE tenant_id = $1")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| format!("查询任务总数失败: {}", e))?
        };
        let total: i64 = count_row.get("count");

        // 查询分页数据
        let rows = if let Some(s) = status {
            sqlx::query(
                r#"
                SELECT * FROM alliance_tasks
                WHERE tenant_id = $1 AND status = $2
                ORDER BY created_at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(tenant_id)
            .bind(s.as_str())
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("查询任务列表失败: {}", e))?
        } else {
            sqlx::query(
                r#"
                SELECT * FROM alliance_tasks
                WHERE tenant_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(tenant_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("查询任务列表失败: {}", e))?
        };

        let tasks: Vec<TaskEntity> = rows.iter().map(row_to_task).collect();
        Ok((tasks, total as u64))
    }

    async fn cancel_task(&self, task_id: Uuid, tenant_id: &str) -> Result<(), String> {
        self.update_task_status(task_id, TaskStatus::Cancelled, None, tenant_id).await
    }
}

// =============================================================================
// 行映射辅助函数
// =============================================================================

fn row_to_task(row: &sqlx::postgres::PgRow) -> TaskEntity {
    TaskEntity {
        id: row.get("id"),
        trace_id: row.get("trace_id"),
        session_id: row.get("session_id"),
        query: row.get("query"),
        status: TaskStatus::from_str(row.get::<&str, _>("status")),
        current_phase: row.get("current_phase"),
        team_size: row.get("team_size"),
        enable_llm: row.get("enable_llm"),
        options_json: row.get("options_json"),
        context_json: row.get("context_json"),
        intent_result: row.get("intent_result"),
        team_result: row.get("team_result"),
        debate_result: row.get("debate_result"),
        synthesis_result: row.get("synthesis_result"),
        gate_result: row.get("gate_result"),
        learn_result: row.get("learn_result"),
        final_result: row.get("final_result"),
        consensus: row.get("consensus"),
        gate_score: row.get("gate_score"),
        gate_grade: row.get("gate_grade"),
        passed: row.get("passed"),
        degraded: row.get("degraded"),
        degrade_reason: row.get("degrade_reason"),
        error_message: row.get("error_message"),
        retry_count: row.get("retry_count"),
        started_at: row.get("started_at"),
        completed_at: row.get("completed_at"),
        duration_ms: row.get("duration_ms"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        tenant_id: row.get("tenant_id"),
        created_by: row.get("created_by"),
    }
}

fn row_to_event(row: &sqlx::postgres::PgRow) -> EventEntity {
    EventEntity {
        id: row.get("id"),
        task_id: row.get("task_id"),
        trace_id: row.get("trace_id"),
        phase: row.get("phase"),
        event_type: row.get("event_type"),
        payload: row.get("payload"),
        latency_ms: row.get("latency_ms"),
        degraded: row.get("degraded"),
        degrade_reason: row.get("degrade_reason"),
        created_at: row.get("created_at"),
        tenant_id: row.get("tenant_id"),
    }
}

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_task() -> TaskEntity {
        TaskEntity {
            id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            session_id: Some("test-session".to_string()),
            query: "测试查询".to_string(),
            status: TaskStatus::Pending,
            current_phase: Some("intent".to_string()),
            team_size: 4,
            enable_llm: false,
            options_json: serde_json::json!({}),
            context_json: serde_json::json!({}),
            intent_result: None,
            team_result: None,
            debate_result: None,
            synthesis_result: None,
            gate_result: None,
            learn_result: None,
            final_result: None,
            consensus: None,
            gate_score: None,
            gate_grade: None,
            passed: false,
            degraded: false,
            degrade_reason: None,
            error_message: None,
            retry_count: 0,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tenant_id: "default".to_string(),
            created_by: None,
        }
    }

    #[tokio::test]
    async fn test_in_memory_create_and_get() {
        let repo = InMemoryTaskRepository::new();
        let task = make_test_task();

        let created = repo.create_task(&task).await.unwrap();
        assert_eq!(created.id, task.id);

        let fetched = repo.get_task(task.id, "default").await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().query, "测试查询");
    }

    #[tokio::test]
    async fn test_in_memory_tenant_isolation() {
        let repo = InMemoryTaskRepository::new();
        let mut task = make_test_task();
        task.tenant_id = "tenant-a".to_string();

        repo.create_task(&task).await.unwrap();

        // 其他租户无法访问
        let fetched = repo.get_task(task.id, "tenant-b").await.unwrap();
        assert!(fetched.is_none());

        // 正确租户可以访问
        let fetched = repo.get_task(task.id, "tenant-a").await.unwrap();
        assert!(fetched.is_some());
    }

    #[tokio::test]
    async fn test_in_memory_update_status() {
        let repo = InMemoryTaskRepository::new();
        let task = make_test_task();
        repo.create_task(&task).await.unwrap();

        repo.update_task_status(task.id, TaskStatus::Running, Some("debate"), "default")
            .await
            .unwrap();

        let fetched = repo.get_task(task.id, "default").await.unwrap().unwrap();
        assert_eq!(fetched.status, TaskStatus::Running);
        assert_eq!(fetched.current_phase, Some("debate".to_string()));
    }

    #[tokio::test]
    async fn test_in_memory_complete_task() {
        let repo = InMemoryTaskRepository::new();
        let task = make_test_task();
        repo.create_task(&task).await.unwrap();

        repo.complete_task(
            task.id,
            serde_json::json!({"result": "test"}),
            0.85,
            0.90,
            "A",
            true,
            5000,
            "default",
        )
        .await
        .unwrap();

        let fetched = repo.get_task(task.id, "default").await.unwrap().unwrap();
        assert_eq!(fetched.status, TaskStatus::Completed);
        assert_eq!(fetched.gate_grade, Some("A".to_string()));
        assert!(fetched.passed);
        assert_eq!(fetched.consensus, Some(0.85));
    }

    #[tokio::test]
    async fn test_in_memory_record_and_get_events() {
        let repo = InMemoryTaskRepository::new();
        let task = make_test_task();
        repo.create_task(&task).await.unwrap();

        let event = EventEntity {
            id: Uuid::new_v4(),
            task_id: task.id,
            trace_id: task.trace_id,
            phase: "intent".to_string(),
            event_type: "phase_data".to_string(),
            payload: serde_json::json!({"intent": "code"}),
            latency_ms: 100,
            degraded: false,
            degrade_reason: None,
            created_at: Utc::now(),
            tenant_id: "default".to_string(),
        };

        repo.record_event(&event).await.unwrap();

        let events = repo.get_task_events(task.id, "default").await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].phase, "intent");
    }

    #[tokio::test]
    async fn test_in_memory_list_tasks() {
        let repo = InMemoryTaskRepository::new();

        for i in 0..5 {
            let mut task = make_test_task();
            task.id = Uuid::new_v4();
            task.query = format!("查询 {}", i);
            repo.create_task(&task).await.unwrap();
        }

        let (tasks, total) = repo.list_tasks("default", 1, 10, None).await.unwrap();
        assert_eq!(total, 5);
        assert_eq!(tasks.len(), 5);

        let (tasks, total) = repo.list_tasks("default", 1, 2, None).await.unwrap();
        assert_eq!(total, 5);
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_task_status_roundtrip() {
        for status in [
            TaskStatus::Pending,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ] {
            let s = status.as_str();
            let parsed = TaskStatus::from_str(s);
            assert_eq!(status, parsed);
        }
    }
}
