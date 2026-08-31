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
use std::sync::{Arc, RwLock};

use mox_alliance_common_proto::{AllianceError, AllianceResult, Task};
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

/// 便捷函数：创建一个临时文件仓库（用于测试 / 演示）
pub fn temp_file_repository(dir: impl AsRef<Path>) -> AllianceResult<Arc<dyn TaskRepository>> {
    let repo = FileTaskRepository::new(dir.as_ref().join("alliance_tasks.json"))?;
    Ok(Arc::new(repo))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: Uuid) -> Task {
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
}
