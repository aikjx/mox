// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 任务存储抽象（企业级持久化，可插拔）
//!
//! 提供统一的任务仓库接口 `TaskRepository`，调度器通过 trait 对象使用，
//! 可在运行时切换不同存储实现：
//! - [`InMemoryTaskRepository`]：进程内内存存储（默认，高吞吐，重启即失）
//! - [`FileTaskRepository`] / [`BatchedFileTaskRepository`]：JSON 快照文件存储
//!   （进程重启后任务状态可恢复；**单写者**，同机多副本共写同一路径会互相覆盖）
//! - [`SqliteTaskRepository`]（feature `sqlite`）：增量 upsert 落盘，WAL + busy_timeout，
//!   读路径直查数据库——这是当前唯一支持**多活副本共享权威状态**的实现
//!
//! 同文件另有 [`SqliteLeaseStore`]：租约选主的权威仲裁点，与任务表同库。
//! 需要更高吞吐可换 Postgres/Redis，只需实现 `TaskRepository` 与 `LeaseStore`。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
#[cfg(feature = "sqlite")]
use std::time::Duration;
use std::time::Instant;

use mox_alliance_common_proto::{AllianceError, AllianceResult, Task};
// TaskStatus / CollaborationPlan 仅被 SQLite 适配器使用（启用 `sqlite` feature 时编译）
#[cfg(feature = "sqlite")]
use chrono::{DateTime, Utc};
#[cfg(feature = "sqlite")]
use mox_alliance_common_proto::{CollaborationPlan, TaskStatus};
#[cfg(feature = "sqlite")]
use rusqlite::{params, Connection, TransactionBehavior};
#[cfg(feature = "sqlite")]
use tracing::warn;
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use crate::leadership::{decide, LeaseState, LeaseStore};

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

/// 多副本共享一个库时的写锁等待窗口（毫秒）。
///
/// `MOX_ALLIANCE_SQLITE_BUSY_MS`，默认 5000。设为 0 即恢复 SQLite 原生"立刻 BUSY 报错"
/// 行为——那会让副本间的并发写/抢主在高竞争下变成随机失败。
#[cfg(feature = "sqlite")]
fn sqlite_busy_timeout_ms() -> u64 {
    std::env::var("MOX_ALLIANCE_SQLITE_BUSY_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(5_000)
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
/// 读路径**直查数据库**（无进程内快照缓存）：多活部署下多个副本共享同一个库文件，
/// 副本 A 写入的任务必须能被副本 B 立刻读到（否则 LB 后的 `get_task` 会随机 NotFound，
/// 且 `queue_capacity`/`max_concurrent_tasks` 准入判断只看得到本副本视图）。
///
/// 启动恢复：见 [`Self::new`]（单副本）与 [`Self::new_shared`]（多活副本）。
#[cfg(feature = "sqlite")]
pub struct SqliteTaskRepository {
    conn: Arc<Mutex<Connection>>,
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
    /// 打开（或创建）SQLite 仓库，建表 + WAL，并执行单副本崩溃恢复标记：
    /// 把上次进程退出时仍在 running 的任务/节点批量标记为 `interrupted`。
    pub fn new(path: impl Into<PathBuf>) -> AllianceResult<Self> {
        Self::open(path, true)
    }

    /// 多活副本打开方式：建表 + WAL，但**不做** running→interrupted 的全表改写。
    ///
    /// 原因：该改写假设"打开这个库的进程就是唯一的调度器，且它刚崩溃重启"。多活下
    /// 新起的 standby 同样会打开这个库，若无条件改写就会把现任 leader 正在跑的任务
    /// 打成 interrupted（假故障）。孤儿判定改由持有领导权的副本经
    /// [`reconcile_active_tasks`](crate::TaskSchedulerImpl::reconcile_active_tasks)
    /// 逐个向执行器对账后作出——那才是有证据的判定。
    pub fn new_shared(path: impl Into<PathBuf>) -> AllianceResult<Self> {
        Self::open(path, false)
    }

    fn open(path: impl Into<PathBuf>, sweep_on_open: bool) -> AllianceResult<Self> {
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
        // busy_timeout：多副本共享一个库时，写者之间必须串行等待而不是立刻 SQLITE_BUSY 失败。
        // WAL 只保证"读写不互斥"，同一时刻仍只有一个写事务；默认 busy_timeout=0 会让
        // 副本间的并发 save/lease 抢主直接报错，故这里显式设窗口（env 可覆盖）。
        let busy_ms = sqlite_busy_timeout_ms();
        conn.execute_batch(&format!(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout={busy_ms};"
        ))
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
        };

        // 单副本崩溃恢复：把上次进程退出时仍在 running 的任务/节点标记为 interrupted。
        // done 节点不动（保留 result），仅 running 被挂起，等待调度器重新认领。
        if sweep_on_open {
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

        Ok(repo)
    }

    /// 把一行 `(status, payload_json)` 还原为 [`Task`]。
    ///
    /// 状态以 `status` 列为准（而非 payload 内嵌值）：列是恢复期改写的对象，
    /// 且 `save()` 的终态仲裁也读这一列，读写两侧必须同一权威源。
    fn task_from_row(status: &str, payload: &str) -> AllianceResult<Task> {
        let mut task: Task = serde_json::from_str(payload)
            .map_err(|e| AllianceError::internal(format!("parse task payload: {}", e)))?;
        task.status = parse_task_status(status);
        Ok(task)
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
        Ok(())
    }

    fn get(&self, task_id: Uuid) -> AllianceResult<Option<Task>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT status, payload_json FROM alliance_task WHERE id = ?1")
            .map_err(|e| AllianceError::internal(format!("prepare get task: {}", e)))?;
        let mut rows = stmt
            .query(params![task_id.to_string()])
            .map_err(|e| AllianceError::internal(format!("query get task: {}", e)))?;
        match rows
            .next()
            .map_err(|e| AllianceError::internal(format!("step get task: {}", e)))?
        {
            Some(row) => {
                let status: String = row
                    .get(0)
                    .map_err(|e| AllianceError::internal(format!("read task status: {}", e)))?;
                let payload: String = row
                    .get(1)
                    .map_err(|e| AllianceError::internal(format!("read task payload: {}", e)))?;
                Self::task_from_row(&status, &payload).map(Some)
            }
            None => Ok(None),
        }
    }

    fn all(&self) -> AllianceResult<Vec<Task>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT status, payload_json FROM alliance_task")
            .map_err(|e| AllianceError::internal(format!("prepare load tasks: {}", e)))?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| AllianceError::internal(format!("query load tasks: {}", e)))?;
        let mut out = Vec::new();
        for row in rows {
            let (status, payload) = row
                .map_err(|e| AllianceError::internal(format!("step load tasks: {}", e)))?;
            out.push(Self::task_from_row(&status, &payload)?);
        }
        Ok(out)
    }

    fn remove(&self, task_id: Uuid) -> AllianceResult<Option<Task>> {
        let removed = self.get(task_id)?;
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

/// 租约选主的 SQLite 仲裁点（多活副本共享同一个库文件时才有意义）。
///
/// 表结构：`alliance_leader_lease(scope PK, holder, epoch, expires_at_ms)`。
/// 跨进程互斥靠 `BEGIN IMMEDIATE`：抢主事务在**读**当前租约之前就先取得库的写锁，
/// 因此"读-判-写"三步对其他副本不可分割；配合 `PRAGMA busy_timeout` 让并发副本
/// 排队而不是随机失败。语义与
/// [`MemoryLeaseStore`](crate::leadership::MemoryLeaseStore) 同源（两者共用
/// [`decide`](crate::leadership::decide)），不存在第二套判定规则。
///
/// 与任务表同库而非另开文件：领导权的权威源必须与任务状态的权威源是**同一个**，
/// 否则会出现"副本自认 leader，但它写的任务表与读到的任务表不是同一份"。
#[cfg(feature = "sqlite")]
pub struct SqliteLeaseStore {
    conn: Arc<Mutex<Connection>>,
}

#[cfg(feature = "sqlite")]
impl SqliteLeaseStore {
    /// 独立打开一个库文件作为仲裁点。
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
        let busy_ms = sqlite_busy_timeout_ms();
        conn.execute_batch(&format!(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout={busy_ms};"
        ))
        .map_err(|e| {
            AllianceError::internal(format!("Failed to set WAL on {}: {}", path.display(), e))
        })?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// 复用任务仓库所在的库（推荐：任务表与领导权同一权威源）。
    pub fn from_repo(repo: &SqliteTaskRepository) -> AllianceResult<Self> {
        let store = Self {
            conn: repo.conn.clone(),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> AllianceResult<()> {
        self.conn
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS alliance_leader_lease (
                scope TEXT PRIMARY KEY,
                holder TEXT,
                epoch INTEGER NOT NULL,
                expires_at_ms INTEGER NOT NULL
            );",
            )
            .map_err(|e| AllianceError::internal(format!("init lease schema: {}", e)))
    }
}

#[cfg(feature = "sqlite")]
impl LeaseStore for SqliteLeaseStore {
    fn state(&self, scope: &str) -> AllianceResult<LeaseState> {
        let conn = self.conn.lock().unwrap();
        read_lease(&conn, scope)
    }

    fn campaign(
        &self,
        scope: &str,
        holder: &str,
        now: DateTime<Utc>,
        lease: Duration,
    ) -> AllianceResult<LeaseState> {
        let mut conn = self.conn.lock().unwrap();
        // Immediate：写锁早于读租约，使"读-判-写"对其它进程不可分割。
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| AllianceError::internal(format!("begin lease tx: {}", e)))?;
        let cur = read_lease(&tx, scope)?;
        let next = match decide(&cur, holder, now, lease) {
            Some(next) => next,
            // 落选：不写，也不抬高 epoch。
            None => return Ok(cur),
        };
        tx.execute(
            "INSERT INTO alliance_leader_lease (scope, holder, epoch, expires_at_ms)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(scope) DO UPDATE SET
                holder = excluded.holder,
                epoch = excluded.epoch,
                expires_at_ms = excluded.expires_at_ms",
            params![
                scope,
                next.holder,
                next.epoch as i64,
                lease_millis(next.expires_at),
            ],
        )
        .map_err(|e| AllianceError::internal(format!("write lease {scope}: {}", e)))?;
        tx.commit()
            .map_err(|e| AllianceError::internal(format!("commit lease {scope}: {}", e)))?;
        Ok(next)
    }

    fn release(&self, scope: &str, holder: &str) -> AllianceResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| AllianceError::internal(format!("begin lease tx: {}", e)))?;
        // 仅持有者本人的释放生效；epoch 保留（让位不得让后续任期回退）。
        tx.execute(
            "UPDATE alliance_leader_lease SET holder = NULL, expires_at_ms = 0
             WHERE scope = ?1 AND holder = ?2",
            params![scope, holder],
        )
        .map_err(|e| AllianceError::internal(format!("release lease {scope}: {}", e)))?;
        tx.commit()
            .map_err(|e| AllianceError::internal(format!("commit release {scope}: {}", e)))
    }
}

/// `expires_at` → 毫秒整数（无持有者记 0）。用数值列比较，避免 RFC3339 文本
/// 长度可变带来的字典序陷阱。
#[cfg(feature = "sqlite")]
fn lease_millis(at: Option<DateTime<Utc>>) -> i64 {
    at.map(|t| t.timestamp_millis()).unwrap_or(0)
}

#[cfg(feature = "sqlite")]
fn lease_from_millis(ms: i64) -> Option<DateTime<Utc>> {
    if ms == 0 {
        None
    } else {
        DateTime::<Utc>::from_timestamp_millis(ms)
    }
}

/// 读取一个 scope 的租约行（不存在 → 默认空状态）。
///
/// 参数取 `&Connection`：`Transaction` 以 Deref 到 `Connection` 的方式复用同一份读逻辑，
/// 因此在事务内读到的就是本事务写锁保护下的最新值。
#[cfg(feature = "sqlite")]
fn read_lease(conn: &Connection, scope: &str) -> AllianceResult<LeaseState> {
    let mut stmt = conn
        .prepare("SELECT holder, epoch, expires_at_ms FROM alliance_leader_lease WHERE scope = ?1")
        .map_err(|e| AllianceError::internal(format!("prepare lease read: {}", e)))?;
    let mut rows = stmt
        .query_map(params![scope], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|e| AllianceError::internal(format!("query lease read: {}", e)))?;
    match rows.next() {
        Some(row) => {
            let (holder, epoch, expires_ms) =
                row.map_err(|e| AllianceError::internal(format!("step lease read: {}", e)))?;
            Ok(LeaseState {
                holder,
                epoch: epoch.max(0) as u64,
                expires_at: lease_from_millis(expires_ms),
            })
        }
        None => Ok(LeaseState::default()),
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

    /// 多活前提：副本 B 必须立刻读到副本 A 写入的任务（无进程内快照缓存）。
    ///
    /// 两个句柄共享同一库文件即同一份权威状态（真实跨进程部署里是两个 OS 进程，
    /// 这里用两个连接覆盖同一语义面：SQLite WAL 下已提交写对后来的读事务可见）。
    #[cfg(feature = "sqlite")]
    #[test]
    fn sqlite_replica_sees_other_replica_writes() {
        let dir = std::env::temp_dir().join(format!("sqlite_multiactive_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("tasks.db");

        let a = SqliteTaskRepository::new_shared(&db).unwrap();
        let b = SqliteTaskRepository::new_shared(&db).unwrap();

        let mut task = make_task(Uuid::new_v4());
        task.status = TaskStatus::Running;
        a.save(&task).unwrap();

        assert_eq!(
            b.get(task.task_id).unwrap().map(|t| t.task_id),
            Some(task.task_id),
            "对端写入的任务必须立刻可读（LB 后 GET 不得随机 NotFound）"
        );
        assert_eq!(b.all().unwrap().len(), 1, "准入判断看到的是全局视图，而非本副本视图");

        // 终态仲裁同样是全局的：任一副本先写入的终态，其它副本不得覆盖。
        let mut done = task.clone();
        done.status = TaskStatus::Completed;
        b.save(&done).unwrap();
        let mut raced = task.clone();
        raced.status = TaskStatus::Cancelled;
        a.save(&raced).unwrap();
        assert_eq!(
            a.get(task.task_id).unwrap().unwrap().status,
            TaskStatus::Completed,
            "跨副本竞争下先到终者为权威（读到的仍是 Completed）"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 多活副本启动不得把对端正在跑的任务打成 interrupted（假故障）；
    /// 单副本 `new` 的崩溃恢复语义保持不变。
    #[cfg(feature = "sqlite")]
    #[test]
    fn shared_open_does_not_sweep_peers_running_tasks() {
        let dir = std::env::temp_dir().join(format!("sqlite_no_sweep_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("tasks.db");

        let mut task = make_task(Uuid::new_v4());
        task.status = TaskStatus::Running;
        {
            let leader = SqliteTaskRepository::new_shared(&db).unwrap();
            leader.save(&task).unwrap();
        }

        // standby 加入：只读，不改写共享状态。
        let standby = SqliteTaskRepository::new_shared(&db).unwrap();
        assert_eq!(standby.get(task.task_id).unwrap().unwrap().status, TaskStatus::Running);

        // 单副本崩溃重启路径仍执行恢复标记。
        let restarted = SqliteTaskRepository::new(&db).unwrap();
        assert_eq!(
            restarted.get(task.task_id).unwrap().unwrap().status,
            TaskStatus::Pending,
            "new() 仍保留 running→interrupted(→Pending) 的单副本恢复语义"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// SQLite 租约表与内存实现共用 `decide` 语义，且跨句柄（跨进程等价）互斥。
    #[cfg(feature = "sqlite")]
    #[test]
    fn sqlite_lease_store_elects_one_leader_and_takes_over() {
        let dir = std::env::temp_dir().join(format!("sqlite_lease_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("tasks.db");

        let repo = SqliteTaskRepository::new_shared(&db).unwrap();
        let store = SqliteLeaseStore::from_repo(&repo).unwrap();
        let other = SqliteLeaseStore::new(&db).unwrap(); // 等价于另一个进程的连接
        // 毫秒粒度的固定起点：租约到期时刻以整数毫秒落库，取整后写读才可比。
        let t0 = chrono::DateTime::from_timestamp_millis(1_760_000_000_123).unwrap();
        let lease = Duration::from_secs(10);

        let a = store.campaign("scheduler", "a", t0, lease).unwrap();
        assert_eq!(a.holder.as_deref(), Some("a"));
        assert_eq!(a.epoch, 1);

        // 另一进程、同一时刻：落选且不抬高 epoch。
        let b = other.campaign("scheduler", "b", t0, lease).unwrap();
        assert_eq!(b.holder.as_deref(), Some("a"), "跨句柄互斥");
        assert_eq!(b.epoch, 1, "落选者不得制造任期风暴");

        // 租约到期后由另一句柄接管，任期 +1。
        let taken = other
            .campaign("scheduler", "b", t0 + chrono::Duration::seconds(11), lease)
            .unwrap();
        assert_eq!(taken.holder.as_deref(), Some("b"));
        assert_eq!(taken.epoch, 2);

        // 过期后迟到的旧 leader 续约：只能观察到别人的任期，不能夺回。
        let late = store
            .campaign("scheduler", "a", t0 + chrono::Duration::seconds(12), lease)
            .unwrap();
        assert_eq!(late.holder.as_deref(), Some("b"));
        assert_eq!(late.epoch, 2);
        assert_eq!(store.state("scheduler").unwrap(), late, "读视图与写结果同源");

        // 让位后 epoch 保留。
        other.release("scheduler", "b").unwrap();
        let after = store.state("scheduler").unwrap();
        assert_eq!(after.holder, None);
        assert_eq!(after.epoch, 2, "让位不得让后续任期回退");
        let next = other
            .campaign("scheduler", "a", t0 + chrono::Duration::seconds(13), lease)
            .unwrap();
        assert_eq!(next.epoch, 3);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 复用任务库的租约表与任务表同库：一个权威源承载"谁能写"与"写了什么"。
    #[cfg(feature = "sqlite")]
    #[test]
    fn sqlite_lease_tables_live_in_the_task_db() {
        let dir = std::env::temp_dir().join(format!("sqlite_lease_same_db_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("tasks.db");

        let repo = SqliteTaskRepository::new_shared(&db).unwrap();
        let task = make_task(Uuid::new_v4());
        repo.save(&task).unwrap();
        let store = SqliteLeaseStore::from_repo(&repo).unwrap();
        store.campaign("scheduler", "a", chrono::Utc::now(), Duration::from_secs(5)).unwrap();

        let conn = Connection::open(&db).unwrap();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert!(
            tables.contains(&"alliance_leader_lease".to_string()),
            "租约表应与任务表同库：{:?}",
            tables
        );
        assert!(tables.contains(&"alliance_task".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
