// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 任务协作服务
//!
//! 负责任务创建、分派、状态迁移、子任务、依赖、关注、评论等领域逻辑。

use std::sync::Arc;

use crate::config::AppConfig;
use crate::error::*;
use crate::event::{DomainEvent, EventBus};
use crate::model::*;
use crate::rbac::*;
use crate::store::Store;

#[derive(Clone)]
pub struct TaskService {
    pub store: Arc<Store>,
    pub bus: EventBus,
    pub config: Arc<AppConfig>,
}

impl TaskService {
    pub fn new(store: Arc<Store>, bus: EventBus, config: Arc<AppConfig>) -> Self {
        Self { store, bus, config }
    }

    pub async fn create(
        &self,
        mox_id: &str,
        by: &str,
        title: &str,
        description: &str,
        priority: Priority,
    ) -> Result<Task> {
        if self.store.get_mox(mox_id).await.is_none() {
            return Err(AppError::BadRequest(format!("璇玑 {mox_id} 不存在")));
        }
        // 配额（I-03）：单璇玑任务数上限
        let count = self.store.list_tasks(mox_id).await.len();
        if count >= self.config.quotas.max_tasks {
            return Err(AppError::Conflict(format!(
                "璇玑 {mox_id} 已达任务上限（{}）",
                self.config.quotas.max_tasks
            )));
        }
        let now = chrono::Utc::now();
        let t = Task {
            id: new_id("task"),
            mox_id: mox_id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            status: TaskStatus::Draft,
            priority,
            created_by: by.to_string(),
            assignees: vec![],
            watchers: vec![],
            subtasks: vec![],
            depends_on: vec![],
            created_at: now,
            updated_at: now,
        };
        self.store.create_task(t.clone()).await;
        self.bus.publish(DomainEvent::TaskCreated {
            task_id: t.id.clone(),
            mox_id: mox_id.to_string(),
            by: by.to_string(),
        });
        Ok(t)
    }

    pub async fn assign(&self, task_id: &str, by: &str, assignees: Vec<String>) -> Result<Task> {
        // 配额（I-03）：单任务被分派人数上限
        if assignees.len() > self.config.quotas.max_assignees {
            return Err(AppError::BadRequest(format!(
                "被分派人数 {} 超过上限（{}）",
                assignees.len(),
                self.config.quotas.max_assignees
            )));
        }
        // 先把状态从 Draft 推到 Assigned（若处于 Draft）
        let before = self
            .store
            .get_task(task_id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))?;
        // BR-07 分派身份三重校验（安全 P0）
        //
        // RBAC 的所有权判定（rbac::authorize）完全以 `task.assignees` 为依据，
        // 因此「被写入 assignees」等价于「获得该任务的 own 类权限」。
        // 若此处不校验，可写入任意 ID —— 包括他璇玑成员 —— 造成跨租户权限提升。
        self.validate_assignees(&before, &assignees).await?;
        let from = before.status;
        let to = if from == TaskStatus::Draft {
            TaskStatus::Assigned
        } else {
            from
        };
        let updated = self
            .store
            .update_task(task_id, |t| {
                t.assignees = assignees.clone();
                t.status = to;
            })
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))?;
        // 确保每位被分派者进入任务频道
        let ch = self.store.task_channel(&updated.mox_id, task_id).await;
        for a in &assignees {
            self.store.add_channel_member(&ch.id, a).await;
        }
        self.store
            .append_audit(AuditRecord {
                id: new_id("aud"),
                action: AuditAction::TaskAssign,
                actor: by.to_string(),
                resource: task_id.to_string(),
                permission: Some(Permission::TaskAssign.as_str().to_string()),
                scope: format!("mox={}", updated.mox_id),
                detail: format!("分派给 {} 位成员: {}", assignees.len(), assignees.join(",")),
                at: chrono::Utc::now(),
            })
            .await;
        self.bus.publish(DomainEvent::TaskAssigned {
            task_id: task_id.to_string(),
            assignees,
            by: by.to_string(),
        });
        Ok(updated)
    }

    /// BR-07：被分派者必须是「存在 + 同璇玑 + Active」的成员
    async fn validate_assignees(&self, task: &Task, assignees: &[String]) -> Result<()> {
        let mut seen: Vec<&String> = Vec::with_capacity(assignees.len());
        for a in assignees {
            if seen.contains(&a) {
                return Err(AppError::BadRequest(format!("分派名单中成员 {a} 重复")));
            }
            seen.push(a);

            let m = self
                .store
                .get_member(a)
                .await
                .ok_or_else(|| AppError::BadRequest(format!("被分派成员 {a} 不存在")))?;
            if m.mox_id != task.mox_id {
                return Err(AppError::Forbidden(format!(
                    "跨璇玑分派被拒：成员 {} 属于璇玑 {}，任务属于璇玑 {}",
                    a, m.mox_id, task.mox_id
                )));
            }
            if !m.status.can_take_task() {
                return Err(AppError::InvalidState(format!(
                    "成员 {} 当前状态为「{}」，不可承接任务（需为「活跃」）",
                    a,
                    m.status.label()
                )));
            }
        }
        Ok(())
    }

    /// BR-10：完成门禁（Definition of Done）
    ///
    /// 进入 `Done` 前必须同时满足：① 全部子任务已完成 ② 全部前置依赖已完成。
    /// 否则「完成」只是状态字段的自我声明，交付质量门禁形同虚设。
    async fn check_done_gate(&self, task: &Task) -> Result<()> {
        let pending: Vec<&str> = task
            .subtasks
            .iter()
            .filter(|s| !s.done)
            .map(|s| s.title.as_str())
            .collect();
        if !pending.is_empty() {
            return Err(AppError::InvalidState(format!(
                "完成门禁未通过：仍有 {} 个子任务未完成（{}）",
                pending.len(),
                pending.join("、")
            )));
        }
        for dep_id in &task.depends_on {
            match self.store.get_task(dep_id).await {
                None => {
                    return Err(AppError::InvalidState(format!(
                        "完成门禁未通过：前置依赖任务 {dep_id} 不存在"
                    )))
                }
                Some(d) if d.status != TaskStatus::Done => {
                    return Err(AppError::InvalidState(format!(
                        "完成门禁未通过：前置依赖《{}》当前为「{}」，需先完成",
                        d.title,
                        d.status.label()
                    )))
                }
                Some(_) => {}
            }
        }
        Ok(())
    }

    pub async fn transition(&self, task_id: &str, by: &str, to: TaskStatus) -> Result<Task> {
        let before = self
            .store
            .get_task(task_id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))?;
        if !before.status.can_transition(to) {
            return Err(AppError::InvalidState(format!(
                "非法状态迁移: {} -> {}",
                before.status.label(),
                to.label()
            )));
        }
        if to == TaskStatus::Done {
            self.check_done_gate(&before).await?;
        }
        let from = before.status;
        let updated = self
            .store
            .update_task(task_id, |t| t.status = to)
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))?;
        self.store
            .append_audit(AuditRecord {
                id: new_id("aud"),
                action: AuditAction::TaskStatusChange,
                actor: by.to_string(),
                resource: task_id.to_string(),
                permission: None,
                scope: format!("mox={}", updated.mox_id),
                detail: format!("{} -> {}", from.label(), to.label()),
                at: chrono::Utc::now(),
            })
            .await;
        self.bus.publish(DomainEvent::TaskStatusChanged {
            task_id: task_id.to_string(),
            from,
            to,
            by: by.to_string(),
        });
        Ok(updated)
    }

    pub async fn add_subtask(&self, task_id: &str, title: &str) -> Result<Task> {
        // 配额（I-03）：单任务子任务数上限
        let before = self
            .store
            .get_task(task_id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))?;
        if before.subtasks.len() >= self.config.quotas.max_subtasks {
            return Err(AppError::BadRequest(format!(
                "子任务数已达上限（{}）",
                self.config.quotas.max_subtasks
            )));
        }
        let st = SubTask {
            id: new_id("sub"),
            title: title.to_string(),
            done: false,
        };
        self.store
            .update_task(task_id, |t| t.subtasks.push(st))
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))
    }

    /// 关注任务（NFR-09 / I-03：max_watchers 配额）
    ///
    /// watcher 必须是「存在 + 同璇玑」的成员；重复关注幂等；
    /// 达到上限时返回 `BadRequest`，防止单一任务被无界订阅拖垮通知分发。
    /// 关注者会被纳入任务事件通知范围（见 `orchestrator` 通知逻辑）。
    pub async fn watch(&self, task_id: &str, watcher_id: &str) -> Result<Task> {
        let before = self
            .store
            .get_task(task_id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))?;
        let m = self
            .store
            .get_member(watcher_id)
            .await
            .ok_or_else(|| AppError::BadRequest(format!("成员 {watcher_id} 不存在")))?;
        if m.mox_id != before.mox_id {
            return Err(AppError::Forbidden(format!(
                "跨璇玑关注被拒：成员 {} 属于璇玑 {}，任务属于璇玑 {}",
                watcher_id, m.mox_id, before.mox_id
            )));
        }
        if before.watchers.iter().any(|w| w == watcher_id) {
            return Ok(before); // 幂等：已关注则直接返回当前状态
        }
        // 配额（I-03）：单任务关注者上限
        if before.watchers.len() >= self.config.quotas.max_watchers {
            return Err(AppError::BadRequest(format!(
                "关注者数已达上限（{}）",
                self.config.quotas.max_watchers
            )));
        }
        self.store
            .update_task(task_id, |t| t.watchers.push(watcher_id.to_string()))
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))
    }

    pub async fn toggle_subtask(&self, task_id: &str, sub_id: &str) -> Result<Task> {
        self.store
            .update_task(task_id, |t| {
                if let Some(s) = t.subtasks.iter_mut().find(|s| s.id == sub_id) {
                    s.done = !s.done;
                }
            })
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))
    }

    /// 追加前置依赖（BR-11）：依赖图必须保持为 DAG，且不得跨璇玑
    ///
    /// `task_id depends_on dep_id` 语义为「dep 先完成，task 才能完成」。
    /// 若成环，BR-10 的完成门禁会互相等待造成永久死锁，因此必须在写入前拦截。
    pub async fn add_dependency(&self, task_id: &str, dep_id: &str) -> Result<Task> {
        if task_id == dep_id {
            return Err(AppError::BadRequest("任务不可依赖自身".into()));
        }
        let task = self
            .store
            .get_task(task_id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))?;
        let dep = self
            .store
            .get_task(dep_id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("依赖任务 {dep_id}")))?;
        if dep.mox_id != task.mox_id {
            return Err(AppError::Forbidden(format!(
                "跨璇玑依赖被拒：任务属于璇玑 {}，依赖任务属于璇玑 {}",
                task.mox_id, dep.mox_id
            )));
        }
        if task.depends_on.iter().any(|d| d == dep_id) {
            // 幂等：已存在该依赖，直接返回当前状态
            return Ok(task);
        }
        // 配额（I-03）：依赖图深度上限，防深链与潜在栈溢出
        let new_depth = self.dep_depth(dep_id).await + 1;
        if new_depth > self.config.quotas.max_dependency_depth {
            return Err(AppError::BadRequest(format!(
                "依赖链深度 {} 超过上限（{}）",
                new_depth, self.config.quotas.max_dependency_depth
            )));
        }
        // 环检测：从 dep 出发沿 depends_on 前向遍历，若可达 task 则加入该边会成环
        if let Some(path) = self.reaches(dep_id, task_id).await {
            return Err(AppError::BadRequest(format!(
                "依赖成环被拒：{} 已（间接）依赖 {}，路径 {}",
                dep_id,
                task_id,
                path.join(" -> ")
            )));
        }
        self.store
            .update_task(task_id, |t| t.depends_on.push(dep_id.to_string()))
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))
    }

    /// 计算以 `task_id` 为根的子树依赖链最大深度（含自身为 1）。
    /// 用于配额校验，防止依赖图过深。已存在的环被 `add_dependency` 拦截，
    /// 此处以 `visited` 兜底以防极端并发下的重复遍历。
    async fn dep_depth(&self, task_id: &str) -> usize {
        // 一次性取出整张任务表用于同步遍历，避免异步递归（E0733）
        let tasks = self.store.state.read().await.tasks.clone();
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        Self::dep_depth_inner(task_id, &tasks, &mut visited)
    }

    fn dep_depth_inner(
        task_id: &str,
        tasks: &std::collections::HashMap<String, Task>,
        visited: &mut std::collections::HashSet<String>,
    ) -> usize {
        if !visited.insert(task_id.to_string()) {
            return 1; // 环兜底
        }
        let t = match tasks.get(task_id) {
            Some(t) => t,
            None => return 1,
        };
        if t.depends_on.is_empty() {
            return 1;
        }
        let mut max = 0usize;
        for d in &t.depends_on {
            max = max.max(Self::dep_depth_inner(d, tasks, visited));
        }
        max + 1
    }

    /// 从 `from` 沿 `depends_on` 前向搜索 `target`，命中则返回路径（用于环检测报错）
    async fn reaches(&self, from: &str, target: &str) -> Option<Vec<String>> {
        let mut visited: Vec<String> = Vec::new();
        let mut stack: Vec<Vec<String>> = vec![vec![from.to_string()]];
        while let Some(path) = stack.pop() {
            let cur = path.last().cloned()?;
            if cur == target {
                return Some(path);
            }
            if visited.contains(&cur) {
                continue;
            }
            visited.push(cur.clone());
            if let Some(t) = self.store.get_task(&cur).await {
                for d in t.depends_on {
                    let mut next = path.clone();
                    next.push(d);
                    stack.push(next);
                }
            }
        }
        None
    }

    /// 评论任务：写入任务频道并发布事件
    pub async fn comment(&self, task_id: &str, by: &str, body: &str) -> Result<Message> {
        let t = self
            .store
            .get_task(task_id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))?;
        let ch = self.store.task_channel(&t.mox_id, task_id).await;
        let msg = Message {
            id: new_id("msg"),
            channel_id: ch.id.clone(),
            sender_id: by.to_string(),
            body: body.to_string(),
            kind: MessageKind::Chat,
            created_at: chrono::Utc::now(),
        };
        self.store.add_message(msg.clone()).await;
        self.bus.publish(DomainEvent::TaskCommented {
            task_id: task_id.to_string(),
            by: by.to_string(),
            channel_id: ch.id.clone(),
        });
        self.bus.publish(DomainEvent::MessagePosted {
            channel_id: ch.id.clone(),
            message_id: msg.id.clone(),
            sender_id: by.to_string(),
        });
        Ok(msg)
    }

    pub async fn get(&self, id: &str) -> Result<Task> {
        self.store
            .get_task(id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {id}")))
    }

    pub async fn list(&self, mox_id: &str) -> Vec<Task> {
        self.store.list_tasks(mox_id).await
    }
}
