// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 任务存储抽象（企业级持久化，可插拔）
//!
//! 提供统一的任务仓库接口 `TaskRepository`，调度器通过 trait 对象使用，
//! 可在运行时切换不同存储实现：
//! - [`InMemoryTaskRepository`]：进程内内存存储（默认，高吞吐）
//! - [`FileTaskRepository`]：JSON 快照文件存储（进程重启后任务状态可恢复）
//!
//! 高吞吐场景可替换为数据库实现（如 Postgres/Redis），只需实现 `TaskRepository`。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use mox_alliance_common_proto::{AllianceError, AllianceResult, Task};
// TaskStatus / CollaborationPlan 仅被 SQLite 适配器使用（启用 `sqlite` feature 时编译）
#[cfg(feature = "sqlite")]
use mox_alliance_common_proto::{CollaborationPlan, TaskStatus};
#[cfg(feature = "sqlite")]
use rusqlite::{params, Connection};
#[cfg(feature = "sqlite")]
use tracing::warn;
use uuid::Uuid;

/// 任务仓库抽象
///
/// 所有读写均以整条 `Task` 记录为单位，便于不同后端（内存/文件/DB）插拔。
/// 实现必须为 `Send + Sync`，可安全地跨线程共享。
pub trait TaskRepository: Send + Sync {
    /// 保存（新增或覆盖）一条任务
    fn save(&self, task: &Task) -> AllianceResult<()>;
    /// 按 ID 读取任务
    fn get(&self, task_id: Uuid) -> AllianceResult<Option<Task>>;
    /// 读取全部任务（跨租户，用于调度统计）
    fn all(&self) -> AllianceResult<Vec<Task>>;
    /// 删除任务，返回被删除的任务（若存在）
    fn remove(&self, task_id: Uuid) -> AllianceResult<Option<Task>>;

    /// 按幂等键查找已绑定任务（场景①）。
    ///
    /// 默认实现返回 `None`（不启用幂等，保持既有行为）；持久化实现（SQLite）
    /// 覆盖后提供**原子去重**：同一键重复提交只产出一份任务。
    fn find_by_idempotency_key(&self, _key: &str) -> AllianceResult<Option<Task>> {
        Ok(None)
    }

    /// 绑定幂等键 → 任务。重复绑定**保留首次绑定**（回放语义），不覆盖也不报错。
    ///
    /// 默认实现为空操作（不启用幂等）。
    fn bind_idempotency_key(&self, _key: &str, _task_id: Uuid) -> AllianceResult<()> {
        Ok(())
    }
}

/// 内存任务仓库（默认实现）
#[derive(Default)]
pub struct InMemoryTaskRepository {
    inner: RwLock<HashMap<Uuid, Task>>,
}

impl InMemoryTaskRepository {
    pub fn new() -> Self {
        Self::default()
    }

    /// 从已有映射初始化（用于测试 / 迁移）
    pub fn from_map(map: HashMap<Uuid, Task>) -> Self {
        Self {
            inner: RwLock::new(map),
        }
    }
}

impl TaskRepository for InMemoryTaskRepository {
    fn save(&self, task: &Task) -> AllianceResult<()> {
        self.inner.write().unwrap().insert(task.task_id, task.clone());
        Ok(())
    }

    fn get(&self, task_id: Uuid) -> AllianceResult<Option<Task>> {
        Ok(self.inner.read().unwrap().get(&task_id).cloned())
    }

    fn all(&self) -> AllianceResult<Vec<Task>> {
        Ok(self.inner.read().unwrap().values().cloned().collect())
    }

    fn remove(&self, task_id: Uuid) -> AllianceResult<Option<Task>> {
        Ok(self.inner.write().unwrap().remove(&task_id))
    }
}

/// 文件快照任务仓库
///
/// 在内存存储之上叠加 JSON 快照持久化：每次写操作后原子落盘，
/// 启动时自动加载上次快照，实现进程重启后的任务状态恢复。
///
/// 注意：全量快照适用于中小规模任务量；超高吞吐建议替换为数据库实现。
pub struct FileTaskRepository {
    inner: InMemoryTaskRepository,
    path: PathBuf,
}

impl FileTaskRepository {
    /// 创建文件仓库，并加载已有快照（若存在）
    pub fn new(path: impl Into<PathBuf>) -> AllianceResult<Self> {
        let path = path.into();
        let inner = InMemoryTaskRepository::new();

        let repo = Self { inner, path };

        // 加载已有快照
        if repo.path.exists() {
            let raw = std::fs::read_to_string(&repo.path).map_err(|e| {
                AllianceError::internal(format!(
                    "Failed to read task snapshot {}: {}",
                    repo.path.display(),
                    e
                ))
            })?;
            let tasks: Vec<Task> = serde_json::from_str(&raw).map_err(|e| {
                AllianceError::internal(format!(
                    "Failed to parse task snapshot {}: {}",
                    repo.path.display(),
                    e
                ))
            })?;
            for task in tasks {
                repo.inner.save(&task)?;
            }
        }

        Ok(repo)
    }

    /// 原子写入快照（临时文件 + 重命名）
    fn persist(&self) -> AllianceResult<()> {
        let tasks = self.inner.all()?;
        let raw = serde_json::to_vec_pretty(&tasks).map_err(|e| {
            AllianceError::internal(format!("Failed to serialize task snapshot: {}", e))
        })?;

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AllianceError::internal(format!(
                    "Failed to create snapshot dir {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, &raw).map_err(|e| {
            AllianceError::internal(format!(
                "Failed to write task snapshot {}: {}",
                tmp.display(),
                e
            ))
        })?;
        std::fs::rename(&tmp, &self.path).map_err(|e| {
            AllianceError::internal(format!(
                "Failed to atomically replace task snapshot {}: {}",
                self.path.display(),
                e
            ))
        })?;
        Ok(())
    }
}

impl TaskRepository for FileTaskRepository {
    fn save(&self, task: &Task) -> AllianceResult<()> {
        self.inner.save(task)?;
        self.persist()
    }

    fn get(&self, task_id: Uuid) -> AllianceResult<Option<Task>> {
        self.inner.get(task_id)
    }

    fn all(&self) -> AllianceResult<Vec<Task>> {
        self.inner.all()
    }

    fn remove(&self, task_id: Uuid) -> AllianceResult<Option<Task>> {
        let removed = self.inner.remove(task_id)?;
        if removed.is_some() {
            self.persist()?;
        }
        Ok(removed)
    }
}

/// 批量写入文件任务仓库
///
/// 在 [`InMemoryTaskRepository`] 之上叠加延迟持久化：`save` / `remove` 只写内存
/// 并标记 dirty，不立即落盘；调用 [`BatchedFileTaskRepository::flush`] 或
/// [`BatchedFileTaskRepository::maybe_flush`] 时才执行全量 JSON 快照写入。
///
/// 适用于高频写入场景：将 O(N) 次序列化 + 磁盘写合并为周期性一次，
/// 显著降低 I/O 开销。进程退出前应主动调用 `flush()` 确保数据不丢失。
///
/// 注意：与 [`FileTaskRepository`] 不同，崩溃时未 flush 的写入会丢失。
pub struct BatchedFileTaskRepository {
    inner: InMemoryTaskRepository,
    path: PathBuf,
    /// 脏标记 + 上次 flush 时间（Mutex 内部可变性，满足 Send+Sync）
    state: Mutex<BatchedState>,
}

struct BatchedState {
    dirty: bool,
    last_flush: Option<Instant>,
}

impl BatchedFileTaskRepository {
    /// 创建批量写入仓库，并加载已有快照（若存在）
    pub fn new(path: impl Into<PathBuf>) -> AllianceResult<Self> {
        let path = path.into();
        let inner = InMemoryTaskRepository::new();

        let repo = Self {
            inner,
            path,
            state: Mutex::new(BatchedState {
                dirty: false,
                last_flush: None,
            }),
        };

        // 加载已有快照
        if repo.path.exists() {
            let raw = std::fs::read_to_string(&repo.path).map_err(|e| {
                AllianceError::internal(format!(
                    "Failed to read task snapshot {}: {}",
                    repo.path.display(),
                    e
                ))
            })?;
            let tasks: Vec<Task> = serde_json::from_str(&raw).map_err(|e| {
                AllianceError::internal(format!(
                    "Failed to parse task snapshot {}: {}",
                    repo.path.display(),
                    e
                ))
            })?;
            for task in tasks {
                repo.inner.save(&task)?;
            }
        }

        Ok(repo)
    }

    /// 原子写入快照（临时文件 + 重命名）
    fn persist(&self) -> AllianceResult<()> {
        let tasks = self.inner.all()?;
        let raw = serde_json::to_vec_pretty(&tasks).map_err(|e| {
            AllianceError::internal(format!("Failed to serialize task snapshot: {}", e))
        })?;

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AllianceError::internal(format!(
                    "Failed to create snapshot dir {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, &raw).map_err(|e| {
            AllianceError::internal(format!(
                "Failed to write task snapshot {}: {}",
                tmp.display(),
                e
            ))
        })?;
        std::fs::rename(&tmp, &self.path).map_err(|e| {
            AllianceError::internal(format!(
                "Failed to atomically replace task snapshot {}: {}",
                self.path.display(),
                e
            ))
        })?;
        Ok(())
    }

    /// 立即持久化（仅当 dirty 为 true 时执行），清除脏标记并更新 last_flush
    pub fn flush(&self) -> AllianceResult<()> {
        let mut state = self.state.lock().unwrap();
        if !state.dirty {
            return Ok(());
        }
        self.persist()?;
        state.dirty = false;
        state.last_flush = Some(Instant::now());
        Ok(())
    }

    /// 条件持久化：距上次 flush 超过 max_interval_ms 且 dirty 时才执行
    ///
    /// 用于高频写入场景的节流：调用方可以在每次 save 后调用 `maybe_flush(5000)`，
    /// 最多每 5 秒落盘一次，而不是每次写入都落盘。
    pub fn maybe_flush(&self, max_interval_ms: u64) -> AllianceResult<()> {
        let should_flush = {
            let state = self.state.lock().unwrap();
            if !state.dirty {
                false
            } else {
                match state.last_flush {
                    None => true,
                    Some(last) => {
                        last.elapsed().as_millis() >= max_interval_ms as u128
                    }
                }
            }
        };
        if should_flush {
            self.flush()?;
        }
        Ok(())
    }

    /// 当前是否有未持久化的写入
    pub fn is_dirty(&self) -> bool {
        self.state.lock().unwrap().dirty
    }
}

impl TaskRepository for BatchedFileTaskRepository {
    fn save(&self, task: &Task) -> AllianceResult<()> {
        self.inner.save(task)?;
        self.state.lock().unwrap().dirty = true;
        Ok(())
    }

    fn get(&self, task_id: Uuid) -> AllianceResult<Option<Task>> {
        self.inner.get(task_id)
    }

    fn all(&self) -> AllianceResult<Vec<Task>> {
        self.inner.all()
    }

    fn remove(&self, task_id: Uuid) -> AllianceResult<Option<Task>> {
        let removed = self.inner.remove(task_id)?;
        if removed.is_some() {
            self.state.lock().unwrap().dirty = true;
        }
        Ok(removed)
    }
}

// ─── SQLite 增量落盘任务仓库（启用 `sqlite` feature 时编译）─────────────────

/// 把 [`TaskStatus`] 映射为 snake_case 落盘字符串（与 serde 配置保持一致）。
#[cfg(feature = "sqlite")]
fn task_status_to_str(s: TaskStatus) -> &'static str {
    match s {
        TaskStatus::Pending => "pending",
        TaskStatus::Planning => "planning",
        TaskStatus::Running => "running",
        TaskStatus::Paused => "paused",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
    }
}

/// 把落盘的 snake_case 状态字符串解析回 [`TaskStatus`]。
///
/// 恢复期产生的 `"interrupted"` 标记（崩溃前为 running）映射为 [`TaskStatus::Pending`]，
/// 表示该任务可被调度器重新认领；已完成节点的结果仍由节点表保留，不会被强行重跑。
#[cfg(feature = "sqlite")]
fn parse_task_status(s: &str) -> TaskStatus {
    match s {
        "pending" => TaskStatus::Pending,
        "planning" => TaskStatus::Planning,
        "running" => TaskStatus::Running,
        "paused" => TaskStatus::Paused,
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        "cancelled" => TaskStatus::Cancelled,
        // 恢复标记：崩溃前正在运行，现已挂起待认领
        _ => TaskStatus::Pending,
    }
}

/// SQLite 增量落盘任务仓库
///
/// 与 [`FileTaskRepository`]（全量 JSON 快照）不同，本实现把任务与 DAG 节点分别落到
/// SQLite 的两张表：
/// - `alliance_task(id, tenant_id, description, status, payload_json, updated_at)`
/// - `alliance_task_node(task_id, node_id, status, result_json, updated_at)`
///
/// 写路径为**增量 upsert**（`INSERT ... ON CONFLICT DO UPDATE`），每次只更新受影响的
/// 那一行，不再把全量任务集序列化成一个 JSON 文件重写。连接开启 WAL 以提升并发读写。
///
/// 节点状态没有进 [`TaskRepository`] trait（trait 只以整条 [`Task`] 为单位），因此节点
/// 的增量写入通过本类型自身的方法 [`SqliteTaskRepository::upsert_node`] 暴露；trait
/// 方法签名与现有 InMemory / File 实现保持不变。
///
/// 启动恢复：打开库后把 `running` 的任务与节点批量标记为 `interrupted`，再把全部任务
/// 读回内存缓存；`done` 节点的 `result_json` 原样保留，不重跑。
#[cfg(feature = "sqlite")]
pub struct SqliteTaskRepository {
    conn: Arc<Mutex<Connection>>,
    /// 读穿缓存：构造时从两表加载，save/remove 时同步维护
    cache: RwLock<HashMap<Uuid, Task>>,
}

/// 节点持久化行的只读视图（用于恢复校验 / 测试）
#[cfg(feature = "sqlite")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredNode {
    pub status: String,
    pub result: Option<serde_json::Value>,
}

#[cfg(feature = "sqlite")]
impl SqliteTaskRepository {
    /// 打开（或创建）SQLite 仓库，建表 + WAL + 恢复标记
    pub fn new(path: impl Into<PathBuf>) -> AllianceResult<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AllianceError::internal(format!(
                    "Failed to create sqlite dir {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        let conn = Connection::open(&path).map_err(|e| {
            AllianceError::internal(format!("Failed to open sqlite {}: {}", path.display(), e))
        })?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| {
                AllianceError::internal(format!("Failed to set WAL on {}: {}", path.display(), e))
            })?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS alliance_task (
                id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                description TEXT NOT NULL,
                status TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS alliance_task_node (
                task_id TEXT NOT NULL,
                node_id TEXT NOT NULL,
                status TEXT NOT NULL,
                result_json TEXT,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (task_id, node_id)
            );
            -- 协作计划（DAG）持久化：恢复期需用它重建执行状态（场景②）
            CREATE TABLE IF NOT EXISTS alliance_task_plan (
                task_id TEXT PRIMARY KEY,
                plan_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            -- 幂等键绑定（场景①）：PRIMARY KEY 保证同一键只绑定首个任务（原子去重）
            CREATE TABLE IF NOT EXISTS alliance_task_idem (
                idem_key TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                created_at TEXT NOT NULL
            );",
        )
        .map_err(|e| {
            AllianceError::internal(format!("Failed to init schema in {}: {}", path.display(), e))
        })?;

        let repo = Self {
            conn: Arc::new(Mutex::new(conn)),
            cache: RwLock::new(HashMap::new()),
        };

        // 崩溃恢复：把上次进程退出时仍在 running 的任务/节点标记为 interrupted。
        // done 节点不动（保留 result），仅 running 被挂起，等待调度器重新认领。
        {
            let c = repo.conn.lock().unwrap();
            c.execute(
                "UPDATE alliance_task SET status='interrupted' WHERE status='running'",
                [],
            )
            .map_err(|e| {
                AllianceError::internal(format!("Failed to mark interrupted tasks: {}", e))
            })?;
            c.execute(
                "UPDATE alliance_task_node SET status='interrupted' WHERE status='running'",
                [],
            )
            .map_err(|e| {
                AllianceError::internal(format!("Failed to mark interrupted nodes: {}", e))
            })?;
        }

        repo.load_into_cache()?;
        Ok(repo)
    }

    /// 从 alliance_task 全量读回任务到内存缓存（恢复完成后服务期内读走缓存）。
    fn load_into_cache(&self) -> AllianceResult<()> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT status, payload_json FROM alliance_task")
            .map_err(|e| AllianceError::internal(format!("prepare load tasks: {}", e)))?;
        let rows = stmt
            .query_map([], |row| {
                let status: String = row.get(0)?;
                let payload: String = row.get(1)?;
                Ok((status, payload))
            })
            .map_err(|e| AllianceError::internal(format!("query load tasks: {}", e)))?;

        let mut cache = self.cache.write().unwrap();
        for row in rows {
            let (status, payload) = row
                .map_err(|e| AllianceError::internal(format!("step load tasks: {}", e)))?;
            let mut task: Task = serde_json::from_str(&payload).map_err(|e| {
                AllianceError::internal(format!("parse task payload: {}", e))
            })?;
            // 以 status 列为准（恢复期已把 running 改写为 interrupted）
            task.status = parse_task_status(&status);
            cache.insert(task.task_id, task);
        }
        Ok(())
    }

    /// 增量写入单个 DAG 节点（`ON CONFLICT(task_id, node_id) DO UPDATE`）。
    ///
    /// 只更新该行的 status / result，不影响其他节点，也不重写任务行的 payload。
    pub fn upsert_node(
        &self,
        task_id: Uuid,
        node_id: &str,
        status: &str,
        result: Option<&serde_json::Value>,
    ) -> AllianceResult<()> {
        let result_json = match result {
            Some(v) => Some(serde_json::to_string(v).map_err(|e| {
                AllianceError::internal(format!("serialize node result: {}", e))
            })?),
            None => None,
        };
        let updated_at = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO alliance_task_node (task_id, node_id, status, result_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(task_id, node_id) DO UPDATE SET
                status = excluded.status,
                result_json = excluded.result_json,
                updated_at = excluded.updated_at",
            params![task_id.to_string(), node_id, status, result_json, updated_at],
        )
        .map_err(|e| {
            AllianceError::internal(format!("sqlite upsert node {}/{}: {}", task_id, node_id, e))
        })?;
        Ok(())
    }

    /// 读取单个节点的持久化行（恢复校验 / 测试用）。
    pub fn get_node(&self, task_id: Uuid, node_id: &str) -> AllianceResult<Option<StoredNode>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT status, result_json FROM alliance_task_node WHERE task_id = ?1 AND node_id = ?2",
            )
            .map_err(|e| AllianceError::internal(format!("prepare get_node: {}", e)))?;
        let mut rows = stmt
            .query(params![task_id.to_string(), node_id])
            .map_err(|e| AllianceError::internal(format!("query get_node: {}", e)))?;
        if let Some(row) = rows
            .next()
            .map_err(|e| AllianceError::internal(format!("step get_node: {}", e)))?
        {
            let status: String = row
                .get(0)
                .map_err(|e| AllianceError::internal(format!("read node status: {}", e)))?;
            let result_json: Option<String> = row
                .get(1)
                .map_err(|e| AllianceError::internal(format!("read node result: {}", e)))?;
            let result = match result_json {
                Some(s) => Some(serde_json::from_str(&s).map_err(|e| {
                    AllianceError::internal(format!("parse node result: {}", e))
                })?),
                None => None,
            };
            Ok(Some(StoredNode { status, result }))
        } else {
            Ok(None)
        }
    }

    /// 写入（增量 upsert）协作计划 JSON。
    ///
    /// 场景②前提：仅持久化任务行不足以重启后续跑——恢复期需要原始 DAG
    /// （节点集合与依赖）才能重建执行状态，故计划单独落表。
    pub fn upsert_plan(&self, task_id: Uuid, plan: &CollaborationPlan) -> AllianceResult<()> {
        let json = serde_json::to_string(plan)
            .map_err(|e| AllianceError::internal(format!("serialize plan {}: {}", task_id, e)))?;
        let updated_at = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO alliance_task_plan (task_id, plan_json, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(task_id) DO UPDATE SET
                plan_json = excluded.plan_json,
                updated_at = excluded.updated_at",
            params![task_id.to_string(), json, updated_at],
        )
        .map_err(|e| AllianceError::internal(format!("sqlite upsert plan {}: {}", task_id, e)))?;
        Ok(())
    }

    /// 读取协作计划（恢复期重建 DAG 用）。
    pub fn load_plan(&self, task_id: Uuid) -> AllianceResult<Option<CollaborationPlan>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT plan_json FROM alliance_task_plan WHERE task_id = ?1")
            .map_err(|e| AllianceError::internal(format!("prepare load plan: {}", e)))?;
        let mut rows = stmt
            .query(params![task_id.to_string()])
            .map_err(|e| AllianceError::internal(format!("query load plan: {}", e)))?;
        if let Some(row) = rows
            .next()
            .map_err(|e| AllianceError::internal(format!("step load plan: {}", e)))?
        {
            let json: String = row
                .get(0)
                .map_err(|e| AllianceError::internal(format!("read plan json: {}", e)))?;
            let plan: CollaborationPlan = serde_json::from_str(&json).map_err(|e| {
                AllianceError::internal(format!("parse plan {}: {}", task_id, e))
            })?;
            Ok(Some(plan))
        } else {
            Ok(None)
        }
    }

    /// 读取某任务的全部节点行（node_id, status, result），供恢复期还原已完成节点。
    pub fn node_rows(
        &self,
        task_id: Uuid,
    ) -> AllianceResult<Vec<(String, String, Option<serde_json::Value>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT node_id, status, result_json FROM alliance_task_node WHERE task_id = ?1",
            )
            .map_err(|e| AllianceError::internal(format!("prepare node rows: {}", e)))?;
        let rows = stmt
            .query_map(params![task_id.to_string()], |row| {
                let node_id: String = row.get(0)?;
                let status: String = row.get(1)?;
                let result_json: Option<String> = row.get(2)?;
                Ok((node_id, status, result_json))
            })
            .map_err(|e| AllianceError::internal(format!("query node rows: {}", e)))?;
        let mut out = Vec::new();
        for r in rows {
            let (node_id, status, result_json) =
                r.map_err(|e| AllianceError::internal(format!("step node rows: {}", e)))?;
            let result = match result_json {
                Some(s) => Some(serde_json::from_str(&s).map_err(|e| {
                    AllianceError::internal(format!("parse node result {}: {}", node_id, e))
                })?),
                None => None,
            };
            out.push((node_id, status, result));
        }
        Ok(out)
    }
}

#[cfg(feature = "sqlite")]
impl TaskRepository for SqliteTaskRepository {
    fn find_by_idempotency_key(&self, key: &str) -> AllianceResult<Option<Task>> {
        let task_id: String = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT task_id FROM alliance_task_idem WHERE idem_key = ?1")
                .map_err(|e| AllianceError::internal(format!("prepare idem lookup: {}", e)))?;
            let mut rows = stmt
                .query(params![key])
                .map_err(|e| AllianceError::internal(format!("query idem lookup: {}", e)))?;
            match rows
                .next()
                .map_err(|e| AllianceError::internal(format!("step idem lookup: {}", e)))?
            {
                Some(row) => row
                    .get(0)
                    .map_err(|e| AllianceError::internal(format!("read idem task_id: {}", e)))?,
                None => return Ok(None),
            }
        };
        let task_id = Uuid::parse_str(&task_id).map_err(|e| {
            AllianceError::internal(format!("幂等键绑定的任务 ID 非法 {}: {}", task_id, e))
        })?;
        self.get(task_id)
    }

    fn bind_idempotency_key(&self, key: &str, task_id: Uuid) -> AllianceResult<()> {
        let conn = self.conn.lock().unwrap();
        // INSERT OR IGNORE + PRIMARY KEY：并发下只有首个绑定生效，后续为回放语义
        conn.execute(
            "INSERT OR IGNORE INTO alliance_task_idem (idem_key, task_id, created_at)
             VALUES (?1, ?2, ?3)",
            params![key, task_id.to_string(), chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| {
            AllianceError::internal(format!("sqlite bind idem key {}: {}", key, e))
        })?;
        Ok(())
    }

    fn save(&self, task: &Task) -> AllianceResult<()> {
        // 场景④仲裁（持久化层）：已落库的终态不可被覆盖——
        // 调度器取消（Cancelled）与引擎完成（Completed）是两个独立写入方，
        // 先到者为权威；同状态重写（时间戳/进度刷新）仍允许。
        {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT status FROM alliance_task WHERE id = ?1")
                .map_err(|e| AllianceError::internal(format!("prepare status peek: {}", e)))?;
            let mut rows = stmt
                .query(params![task.task_id.to_string()])
                .map_err(|e| AllianceError::internal(format!("query status peek: {}", e)))?;
            if let Some(row) = rows
                .next()
                .map_err(|e| AllianceError::internal(format!("step status peek: {}", e)))?
            {
                let stored: String = row
                    .get(0)
                    .map_err(|e| AllianceError::internal(format!("read status: {}", e)))?;
                let stored_status = parse_task_status(&stored);
                if stored_status.is_terminal() && stored_status != task.status {
                    warn!(
                        "拒绝覆盖终态任务 {}：存储层 {:?} vs 写入 {:?}（取消/完成竞争，先到者为权威）",
                        task.task_id, stored_status, task.status
                    );
                    return Ok(());
                }
            }
        }
        let payload = serde_json::to_string(task)
            .map_err(|e| AllianceError::internal(format!("serialize task: {}", e)))?;
        let status = task_status_to_str(task.status);
        let updated_at = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO alliance_task (id, tenant_id, description, status, payload_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                tenant_id = excluded.tenant_id,
                description = excluded.description,
                status = excluded.status,
                payload_json = excluded.payload_json,
                updated_at = excluded.updated_at",
            params![
                task.task_id.to_string(),
                task.tenant_id.to_string(),
                task.description,
                status,
                payload,
                updated_at,
            ],
        )
        .map_err(|e| {
            AllianceError::internal(format!("sqlite upsert task {}: {}", task.task_id, e))
        })?;
        drop(conn);
        self.cache.write().unwrap().insert(task.task_id, task.clone());
        Ok(())
    }

    fn get(&self, task_id: Uuid) -> AllianceResult<Option<Task>> {
        Ok(self.cache.read().unwrap().get(&task_id).cloned())
    }

    fn all(&self) -> AllianceResult<Vec<Task>> {
        Ok(self.cache.read().unwrap().values().cloned().collect())
    }

    fn remove(&self, task_id: Uuid) -> AllianceResult<Option<Task>> {
        let removed = self.cache.write().unwrap().remove(&task_id);
        if removed.is_some() {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "DELETE FROM alliance_task WHERE id = ?1",
                params![task_id.to_string()],
            )
            .map_err(|e| {
                AllianceError::internal(format!("sqlite delete task {}: {}", task_id, e))
            })?;
            // 级联清理该任务的节点行
            conn.execute(
                "DELETE FROM alliance_task_node WHERE task_id = ?1",
                params![task_id.to_string()],
            )
            .map_err(|e| {
                AllianceError::internal(format!("sqlite delete nodes of {}: {}", task_id, e))
            })?;
        }
        Ok(removed)
    }
}

/// 便捷函数：创建一个临时文件仓库（用于测试 / 演示）
pub fn temp_file_repository(dir: impl AsRef<Path>) -> AllianceResult<Arc<dyn TaskRepository>> {
    let repo = FileTaskRepository::new(dir.as_ref().join("alliance_tasks.json"))?;
    Ok(Arc::new(repo))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(_id: Uuid) -> Task {
        Task::new(Uuid::new_v4(), Uuid::new_v4(), "t".to_string(), "d".to_string())
    }

    #[test]
    fn in_memory_save_get_remove() {
        let repo = InMemoryTaskRepository::new();
        let task = make_task(Uuid::new_v4());
        repo.save(&task).unwrap();
        assert_eq!(repo.get(task.task_id).unwrap().unwrap().task_id, task.task_id);
        assert!(repo.get(Uuid::new_v4()).unwrap().is_none());
        let removed = repo.remove(task.task_id).unwrap();
        assert!(removed.is_some());
        assert!(repo.get(task.task_id).unwrap().is_none());
    }

    #[test]
    fn file_repo_persists_across_instances() {
        let dir = std::env::temp_dir().join(format!("alliance_repo_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tasks.json");

        let task = make_task(Uuid::new_v4());
        {
            let repo = FileTaskRepository::new(&path).unwrap();
            repo.save(&task).unwrap();
        }
        // 新实例应能加载快照
        {
            let repo = FileTaskRepository::new(&path).unwrap();
            let loaded = repo.get(task.task_id).unwrap().unwrap();
            assert_eq!(loaded.task_id, task.task_id);
            assert_eq!(loaded.title, task.title);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn batched_save_get_without_flush() {
        let dir = std::env::temp_dir().join(format!("batched_repo_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tasks.json");

        let repo = BatchedFileTaskRepository::new(&path).unwrap();
        let task = make_task(Uuid::new_v4());
        repo.save(&task).unwrap();

        // 内存中可读
        assert_eq!(repo.get(task.task_id).unwrap().unwrap().task_id, task.task_id);
        // 标记为 dirty（未落盘）
        assert!(repo.is_dirty());
        // 文件尚不存在（因为未 flush）
        assert!(!path.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn batched_flush_persists_to_disk() {
        let dir = std::env::temp_dir().join(format!("batched_repo_flush_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tasks.json");

        let task = make_task(Uuid::new_v4());
        {
            let repo = BatchedFileTaskRepository::new(&path).unwrap();
            repo.save(&task).unwrap();
            assert!(repo.is_dirty());
            repo.flush().unwrap();
            assert!(!repo.is_dirty());
        }

        // 新实例应能加载已持久化的快照
        {
            let repo = BatchedFileTaskRepository::new(&path).unwrap();
            let loaded = repo.get(task.task_id).unwrap().unwrap();
            assert_eq!(loaded.task_id, task.task_id);
            assert_eq!(loaded.title, task.title);
            assert!(!repo.is_dirty());
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn batched_flush_noop_when_clean() {
        let dir = std::env::temp_dir().join(format!("batched_repo_noop_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tasks.json");

        let repo = BatchedFileTaskRepository::new(&path).unwrap();
        // 无写入时 flush 不应创建文件
        repo.flush().unwrap();
        assert!(!path.exists());
        assert!(!repo.is_dirty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn batched_remove_marks_dirty() {
        let dir = std::env::temp_dir().join(format!("batched_repo_remove_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tasks.json");

        let repo = BatchedFileTaskRepository::new(&path).unwrap();
        let task = make_task(Uuid::new_v4());
        repo.save(&task).unwrap();
        repo.flush().unwrap();
        assert!(!repo.is_dirty());

        // 删除应标记 dirty
        let removed = repo.remove(task.task_id).unwrap();
        assert!(removed.is_some());
        assert!(repo.is_dirty());
        assert!(repo.get(task.task_id).unwrap().is_none());

        // flush 后新实例不应包含已删除任务
        repo.flush().unwrap();
        let repo2 = BatchedFileTaskRepository::new(&path).unwrap();
        assert!(repo2.get(task.task_id).unwrap().is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn batched_maybe_flush_respects_interval() {
        let dir = std::env::temp_dir().join(format!("batched_repo_maybe_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tasks.json");

        let repo = BatchedFileTaskRepository::new(&path).unwrap();
        let task = make_task(Uuid::new_v4());
        repo.save(&task).unwrap();
        assert!(repo.is_dirty());

        // 第一次 maybe_flush（last_flush 为 None）应立即持久化
        repo.maybe_flush(60_000).unwrap();
        assert!(!repo.is_dirty());
        assert!(path.exists());

        // 再次写入后，maybe_flush 在间隔内不应触发
        let task2 = make_task(Uuid::new_v4());
        repo.save(&task2).unwrap();
        assert!(repo.is_dirty());
        repo.maybe_flush(60_000).unwrap();
        // 间隔未到，仍为 dirty（但内存中有 task2）
        assert!(repo.is_dirty());
        assert_eq!(repo.get(task2.task_id).unwrap().unwrap().task_id, task2.task_id);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn sqlite_save_get_roundtrip() {
        let dir = std::env::temp_dir().join(format!("sqlite_roundtrip_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("tasks.db");

        let task = make_task(Uuid::new_v4());
        let repo = SqliteTaskRepository::new(&db).unwrap();
        repo.save(&task).unwrap();
        assert_eq!(repo.get(task.task_id).unwrap().unwrap().task_id, task.task_id);
        assert_eq!(repo.all().unwrap().len(), 1);

        let removed = repo.remove(task.task_id).unwrap();
        assert!(removed.is_some());
        assert!(repo.get(task.task_id).unwrap().is_none());
        assert!(repo.all().unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn sqlite_incremental_upsert_does_not_lose_other_nodes() {
        let dir = std::env::temp_dir().join(format!("sqlite_incr_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("tasks.db");

        let task = make_task(Uuid::new_v4());
        let repo = SqliteTaskRepository::new(&db).unwrap();
        repo.save(&task).unwrap();

        let a = serde_json::json!({"v": 1});
        let b = serde_json::json!({"v": 2});
        repo.upsert_node(task.task_id, "n-a", "completed", Some(&a)).unwrap();
        repo.upsert_node(task.task_id, "n-b", "completed", Some(&b)).unwrap();

        // 单独更新 n-b 的 result，n-a 不应被影响
        let b2 = serde_json::json!({"v": 22});
        repo.upsert_node(task.task_id, "n-b", "completed", Some(&b2)).unwrap();

        let na = repo.get_node(task.task_id, "n-a").unwrap().unwrap();
        assert_eq!(na.status, "completed");
        assert_eq!(na.result, Some(a));
        let nb = repo.get_node(task.task_id, "n-b").unwrap().unwrap();
        assert_eq!(nb.result, Some(b2));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn sqlite_recovers_task_and_nodes_marks_running_as_interrupted() {
        let dir = std::env::temp_dir().join(format!("sqlite_recover_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("tasks.db");

        // 阶段一：构造一个 running 任务 + 两个完成节点 + 一个 running 节点，持久化后 drop
        let mut task = make_task(Uuid::new_v4());
        task.status = TaskStatus::Running;
        {
            let repo = SqliteTaskRepository::new(&db).unwrap();
            repo.save(&task).unwrap();

            let r_a = serde_json::json!({"output": "node-A done"});
            let r_b = serde_json::json!({"output": "node-B done"});
            repo.upsert_node(task.task_id, "node-A", "completed", Some(&r_a)).unwrap();
            repo.upsert_node(task.task_id, "node-B", "completed", Some(&r_b)).unwrap();
            // 这个节点崩溃前还在跑，重开后应被标记为 interrupted
            repo.upsert_node(task.task_id, "node-C", "running", None).unwrap();
        }

        // 阶段二：同路径重开，验证恢复
        let repo = SqliteTaskRepository::new(&db).unwrap();

        // 任务仍在；原来 running 的任务被恢复为 Pending（interrupted 标记映射）
        let loaded = repo.get(task.task_id).unwrap().unwrap();
        assert_eq!(loaded.task_id, task.task_id);
        assert_eq!(loaded.status, TaskStatus::Pending);

        // 已完成节点的 result 仍在
        let na = repo.get_node(task.task_id, "node-A").unwrap().unwrap();
        assert_eq!(na.status, "completed");
        assert_eq!(na.result, Some(serde_json::json!({"output": "node-A done"})));
        let nb = repo.get_node(task.task_id, "node-B").unwrap().unwrap();
        assert_eq!(nb.status, "completed");
        assert_eq!(nb.result, Some(serde_json::json!({"output": "node-B done"})));

        // running 节点变为 interrupted，且未强行重跑（result 仍为 None，状态被改写）
        let nc = repo.get_node(task.task_id, "node-C").unwrap().unwrap();
        assert_eq!(nc.status, "interrupted");
        assert!(nc.result.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 场景④（持久化层）：终态不可被覆盖——先到者为权威，同状态重写仍允许
    #[cfg(feature = "sqlite")]
    #[test]
    fn save_rejects_overwriting_terminal_task() {
        let dir = std::env::temp_dir().join(format!("terminal_guard_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("tasks.db");

        let repo = SqliteTaskRepository::new(&db).unwrap();
        let mut task = make_task(Uuid::new_v4());
        repo.save(&task).unwrap(); // Pending → Pending

        task.status = TaskStatus::Cancelled;
        repo.save(&task).unwrap(); // Pending → Cancelled（合法）

        // 竞争写入方（引擎/调度器另一侧）试图把 Cancelled 改写为 Completed
        task.status = TaskStatus::Completed;
        repo.save(&task).unwrap(); // 应被拒绝（终态不可离开）

        // 重开库（全新读穿缓存）验证存储层权威状态仍是 Cancelled
        let repo2 = SqliteTaskRepository::new(&db).unwrap();
        let loaded = repo2.get(task.task_id).unwrap().unwrap();
        assert_eq!(
            loaded.status,
            TaskStatus::Cancelled,
            "终态不可被覆盖（先到者为权威）"
        );

        // 同状态重写（时间戳/进度刷新）仍允许
        task.status = TaskStatus::Cancelled;
        repo2.save(&task).unwrap();
        assert_eq!(
            repo2.get(task.task_id).unwrap().unwrap().status,
            TaskStatus::Cancelled
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
