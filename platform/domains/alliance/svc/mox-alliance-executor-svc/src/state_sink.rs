// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 执行状态持久化适配器（svc 层持有 IO；core 只见 [`ExecutionStateSink`] 端口）
//!
//! # 不另造轮子：复用 scheduler-core 既有存储底座
//!
//! 适配器只是把 [`ExecutionStateSink`] 端口接到调度器侧**已有**的
//! `SqliteTaskRepository` / `FileTaskRepository` 上，并沿用同一环境变量约定
//! `MOX_ALLIANCE_STORAGE_MODE`（与 `scheduler-svc` 的 `resolve_task_repository`
//! 完全一致），使调度器与执行器共享**同一份任务真源**：
//!
//! | 模式 | 路径 | 任务级 | 节点级 |
//! |---|---|---|---|
//! | `sqlite` | `data/alliance_tasks.db`（WAL） | ✓ save | ✓ 增量 upsert（完整恢复能力） |
//! | `file`（默认，与 scheduler 一致） | `data/alliance_tasks.json` | ✓ save | ✗（无节点表，debug 记录） |
//! | `memory` | — | ✗ | ✗（`None`：纯内存执行，行为与未接线前一致） |

use std::path::Path;
use std::sync::Arc;

use mox_alliance_common_proto::{AllianceResult, CollaborationPlan, Task};
use mox_alliance_executor_core::{ExecutionStateSink, RestorableTask};
use mox_alliance_scheduler_core::{FileTaskRepository, SqliteTaskRepository, TaskRepository};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 视为「未终结、需恢复」的任务状态集
fn is_resumable(status: mox_alliance_common_proto::TaskStatus) -> bool {
    !status.is_terminal()
}

/// SQLite 底座适配器：任务级 + 节点级完整持久化（场景③恢复能力）
pub struct SqliteExecutionStateSink {
    repo: Arc<SqliteTaskRepository>,
}

impl SqliteExecutionStateSink {
    pub fn new(repo: Arc<SqliteTaskRepository>) -> Self {
        Self { repo }
    }
}

impl ExecutionStateSink for SqliteExecutionStateSink {
    fn persist_task(&self, task: &Task) -> AllianceResult<()> {
        self.repo.save(task)
    }

    fn persist_node(
        &self,
        task_id: Uuid,
        node_id: &str,
        status: &str,
        result: Option<&serde_json::Value>,
    ) -> AllianceResult<()> {
        self.repo.upsert_node(task_id, node_id, status, result)
    }

    fn persist_plan(&self, task_id: Uuid, plan: &CollaborationPlan) -> AllianceResult<()> {
        self.repo.upsert_plan(task_id, plan)
    }

    fn restore_pending(&self) -> AllianceResult<Vec<RestorableTask>> {
        let mut out = Vec::new();
        for task in self.repo.all()? {
            if !is_resumable(task.status) {
                continue;
            }
            // 无持久化计划 → 无法重建 DAG，跳过（交由人工/对账处理）
            let Some(plan) = self.repo.load_plan(task.task_id)? else {
                debug!("任务 {} 无计划，跳过恢复", task.task_id);
                continue;
            };
            let completed = self
                .repo
                .node_rows(task.task_id)?
                .into_iter()
                .filter(|(_, status, _)| status == "completed" || status == "skipped")
                .map(|(node_id, _status, result)| (node_id, result))
                .collect();
            out.push(RestorableTask {
                task,
                plan,
                completed_nodes: completed,
            });
        }
        Ok(out)
    }
}

/// 文件快照底座适配器：仅任务级（`FileTaskRepository` 无节点表，节点结果不落盘）
pub struct FileExecutionStateSink {
    repo: Arc<FileTaskRepository>,
}

impl FileExecutionStateSink {
    pub fn new(repo: Arc<FileTaskRepository>) -> Self {
        Self { repo }
    }
}

impl ExecutionStateSink for FileExecutionStateSink {
    fn persist_task(&self, task: &Task) -> AllianceResult<()> {
        self.repo.save(task)
    }

    fn persist_node(
        &self,
        task_id: Uuid,
        node_id: &str,
        status: &str,
        _result: Option<&serde_json::Value>,
    ) -> AllianceResult<()> {
        // 文件底座无节点表：任务级持久化仍有效，节点级跳过（升级 sqlite 即得）
        debug!(
            "file sink 无节点表，跳过节点持久化 {}/{} (status={})",
            task_id, node_id, status
        );
        Ok(())
    }

    fn persist_plan(&self, _task_id: Uuid, _plan: &CollaborationPlan) -> AllianceResult<()> {
        // 文件底座不存计划（升级 sqlite 即得完整恢复能力）
        Ok(())
    }

    fn restore_pending(&self) -> AllianceResult<Vec<RestorableTask>> {
        // 文件底座无 plan / 节点表，不具备恢复能力（语义显式，不假装有）
        Ok(Vec::new())
    }
}

/// 按环境变量解析执行状态持久化端口（与 scheduler-svc 存储模式约定一致）
pub fn resolve_state_sink() -> Option<Arc<dyn ExecutionStateSink>> {
    resolve_state_sink_with_root(Path::new("data"))
}

/// 可注入根目录版（测试 / 自定义数据目录）
pub fn resolve_state_sink_with_root(root: &Path) -> Option<Arc<dyn ExecutionStateSink>> {
    let mode = std::env::var("MOX_ALLIANCE_STORAGE_MODE").unwrap_or_default();
    match mode.as_str() {
        "sqlite" => {
            let path = root.join("alliance_tasks.db");
            match SqliteTaskRepository::new(&path) {
                Ok(repo) => {
                    info!("executor state sink: sqlite (WAL) at {}", path.display());
                    Some(Arc::new(SqliteExecutionStateSink::new(Arc::new(repo))))
                }
                Err(e) => {
                    warn!("executor state sink 初始化失败，降级为纯内存: {}", e);
                    None
                }
            }
        }
        "memory" => {
            info!("executor state sink: memory（纯内存执行，未启用持久化）");
            None
        }
        _ => {
            // 默认 file，与 scheduler-svc 的默认存储模式保持一致（同一任务真源）
            let path = root.join("alliance_tasks.json");
            match FileTaskRepository::new(&path) {
                Ok(repo) => {
                    info!("executor state sink: file at {}", path.display());
                    Some(Arc::new(FileExecutionStateSink::new(Arc::new(repo))))
                }
                Err(e) => {
                    warn!("executor state sink 初始化失败，降级为纯内存: {}", e);
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_common_proto::TaskStatus;

    #[test]
    fn sqlite_sink_roundtrip_and_recovery_marks_running_interrupted() {
        let dir = std::env::temp_dir().join(format!("executor_sink_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("tasks.db");

        let task = Task::new(Uuid::new_v4(), Uuid::new_v4(), "t".to_string(), "d".to_string());
        let task_id = task.task_id;

        // 第一段：经端口写任务行 + 节点行（模拟引擎埋点输出）
        {
            let repo = Arc::new(SqliteTaskRepository::new(&db).unwrap());
            let sink = SqliteExecutionStateSink::new(repo);
            sink.persist_task(&task).unwrap();
            sink.persist_node(task_id, "node-A", "completed", Some(&serde_json::json!({"v": 1})))
                .unwrap();
            sink.persist_node(task_id, "node-B", "running", None).unwrap();
        }

        // 第二段：重开库（模拟进程重启），验证场景③恢复基础
        let repo = SqliteTaskRepository::new(&db).unwrap();
        let loaded = repo.get(task_id).unwrap().expect("任务行应可读回");
        assert_eq!(loaded.task_id, task_id);
        assert_eq!(loaded.status, TaskStatus::Pending);

        // 已完成节点的结果保留
        let node_a = repo.get_node(task_id, "node-A").unwrap().expect("node-A 行应存在");
        assert_eq!(node_a.status, "completed");
        assert_eq!(node_a.result, Some(serde_json::json!({"v": 1})));

        // running 节点被重开库的恢复标记改为 interrupted（等待调度器重新认领）
        let node_b = repo.get_node(task_id, "node-B").unwrap().expect("node-B 行应存在");
        assert_eq!(node_b.status, "interrupted");
        assert!(node_b.result.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn temp_plan(task_id: Uuid) -> CollaborationPlan {
        use mox_alliance_common_proto::{AllianceMode, FusionStrategy};
        use std::collections::HashMap;
        CollaborationPlan {
            task_id,
            mode: AllianceMode::Parallel,
            fusion_strategy: FusionStrategy::Weighted,
            nodes: vec![],
            dynamic_routes: vec![],
            expert_weights: HashMap::new(),
            version: 1,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn sqlite_restore_pending_includes_unfinished_and_skips_terminal_or_planless() {
        let dir = std::env::temp_dir().join(format!("executor_restore_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("tasks.db");
        let sink = SqliteExecutionStateSink::new(Arc::new(SqliteTaskRepository::new(&db).unwrap()));

        let tenant = Uuid::new_v4();
        // A：未终结 + 有计划 → 应被恢复
        let mut a = Task::new(tenant, Uuid::new_v4(), "resumable".to_string(), "d".to_string());
        a.status = TaskStatus::Running;
        sink.persist_task(&a).unwrap();
        sink.persist_plan(a.task_id, &temp_plan(a.task_id)).unwrap();

        // B：终态任务 → 不该被恢复
        let mut b = Task::new(tenant, Uuid::new_v4(), "done".to_string(), "d".to_string());
        b.status = TaskStatus::Completed;
        sink.persist_task(&b).unwrap();
        sink.persist_plan(b.task_id, &temp_plan(b.task_id)).unwrap();

        // C：未终结但无计划 → 无法重建 DAG，跳过
        let c = Task::new(tenant, Uuid::new_v4(), "planless".to_string(), "d".to_string());
        sink.persist_task(&c).unwrap();

        let restorable = sink.restore_pending().unwrap();
        let ids: Vec<Uuid> = restorable.iter().map(|r| r.task.task_id).collect();
        assert!(ids.contains(&a.task_id), "未终结且有计划的任务应可恢复");
        assert!(!ids.contains(&b.task_id), "终态任务不应被恢复");
        assert!(!ids.contains(&c.task_id), "无计划任务无法重建 DAG，应跳过");
        assert_eq!(restorable.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_sink_has_no_restore_capability() {
        // 文件底座无 plan / 节点表，恢复能力显式为空（不假装有）
        let dir = std::env::temp_dir().join(format!("executor_file_restore_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let sink = FileExecutionStateSink::new(Arc::new(
            FileTaskRepository::new(dir.join("tasks.json")).unwrap(),
        ));
        let task = Task::new(Uuid::new_v4(), Uuid::new_v4(), "t".to_string(), "d".to_string());
        sink.persist_task(&task).unwrap();
        sink.persist_plan(task.task_id, &temp_plan(task.task_id)).unwrap();
        assert!(sink.restore_pending().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_sink_persists_task_across_reopen() {
        let dir = std::env::temp_dir().join(format!("executor_sink_file_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let json_path = dir.join("tasks.json");

        let task = Task::new(Uuid::new_v4(), Uuid::new_v4(), "t".to_string(), "d".to_string());
        let task_id = task.task_id;

        {
            let repo = Arc::new(FileTaskRepository::new(&json_path).unwrap());
            let sink = FileExecutionStateSink::new(repo);
            sink.persist_task(&task).unwrap();
            // 节点级在文件底座上是显式跳过（不报错）
            sink.persist_node(task_id, "node-A", "completed", None).unwrap();
        }

        let repo = FileTaskRepository::new(&json_path).unwrap();
        let loaded = repo.get(task_id).unwrap().expect("文件底座任务应可读回");
        assert_eq!(loaded.task_id, task_id);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
