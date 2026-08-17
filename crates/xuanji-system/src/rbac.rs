//! 权限分配引擎（RBAC）
//!
//! 设计原则（与 xuanji-expert 的 RBAC Engine 同源）：
//! - **资源级粒度**：权限可限定到璇玑 / 具体任务
//! - **角色继承**：高级角色继承低级角色的权限，无需重复列举
//! - **所有权作用域**：`*Own` 类权限要求调用者是资源所有者（如任务的被分派者）
//! - **拒绝原因**：`check` 返回具体缺失的权限，便于前端提示
//! - **可审计**：鉴权失败由编排层统一记录审计事件
use serde::{Deserialize, Serialize};

/// 原子权限
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    TaskCreate,        // 创建任务
    TaskAssign,        // 分派任务
    TaskEditAll,       // 编辑任意任务
    TaskEditOwn,       // 编辑自己被分派的任务
    TaskViewAll,       // 查看璇玑内任意任务
    TaskViewAssigned,  // 查看自己被分派的任务
    TaskComment,       // 评论任务
    TaskTransitionAll,  // 推进任意任务状态（协调员/管理员）
    TaskTransitionOwn,  // 推进自己被分派任务的状态（专家）
    MemberInvite,      // 邀请成员
    MemberManage,      // 管理成员（暂停/移除/改级）
    CommSendXuanji,  // 在璇玑频道发言
    CommSendTask,      // 在任务频道发言
    CommSendDirect,    // 发起私信
    AuditView,         // 查看审计/通知
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        use Permission::*;
        match self {
            TaskCreate => "task:create",
            TaskAssign => "task:assign",
            TaskEditAll => "task:edit:all",
            TaskEditOwn => "task:edit:own",
            TaskViewAll => "task:view:all",
            TaskViewAssigned => "task:view:assigned",
            TaskComment => "task:comment",
            TaskTransitionAll => "task:transition:all",
            TaskTransitionOwn => "task:transition:own",
            MemberInvite => "member:invite",
            MemberManage => "member:manage",
            CommSendXuanji => "comm:send:xuanji",
            CommSendTask => "comm:send:task",
            CommSendDirect => "comm:send:direct",
            AuditView => "audit:view",
        }
    }
    /// 该权限是否要求调用者是资源所有者
    pub fn requires_ownership(&self) -> bool {
        matches!(
            self,
            Permission::TaskEditOwn | Permission::TaskViewAssigned | Permission::TaskTransitionOwn
        )
    }
}

/// 角色
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Role {
    XuanjiAdmin, // 璇玑管理员：所有权限
    Coordinator,   // 协调员：任务与成员运营
    Expert,        // 专家：参与任务协作
    Member,        // 普通成员：受限参与
    Auditor,       // 审计员：只读全局
}

impl Role {
    pub fn all() -> &'static [Role] {
        &[
            Role::XuanjiAdmin,
            Role::Coordinator,
            Role::Expert,
            Role::Member,
            Role::Auditor,
        ]
    }
    pub fn label(&self) -> &'static str {
        use Role::*;
        match self {
            XuanjiAdmin => "璇玑管理员",
            Coordinator => "协调员",
            Expert => "专家",
            Member => "成员",
            Auditor => "审计员",
        }
    }
    /// 直接拥有的权限（不含继承）
    fn own_permissions(&self) -> &'static [Permission] {
        use Permission::*;
        match self {
            Role::XuanjiAdmin => &[
                TaskCreate, TaskAssign, TaskEditAll, TaskViewAll, TaskComment, TaskTransitionAll,
                MemberInvite, MemberManage, CommSendXuanji, CommSendTask, CommSendDirect, AuditView,
            ],
            Role::Coordinator => &[
                TaskCreate, TaskAssign, TaskEditAll, TaskViewAll, TaskComment, TaskTransitionAll,
                MemberInvite, CommSendXuanji, CommSendTask, CommSendDirect, AuditView,
            ],
            Role::Expert => &[
                TaskViewAssigned, TaskEditOwn, TaskComment, TaskTransitionOwn, CommSendTask,
                CommSendDirect,
            ],
            Role::Member => &[TaskViewAssigned, TaskComment, CommSendTask, CommSendDirect],
            Role::Auditor => &[TaskViewAll, AuditView],
        }
    }
    /// 继承链（向下包含）
    fn inherits(&self) -> Option<Role> {
        match self {
            Role::Coordinator => Some(Role::Expert),
            Role::Expert => Some(Role::Member),
            _ => None,
        }
    }
    /// 展开（含继承）后的全部权限
    pub fn effective_permissions(&self) -> Vec<Permission> {
        let mut set: Vec<Permission> = self.own_permissions().to_vec();
        let mut cur = self.inherits();
        while let Some(r) = cur {
            for p in r.own_permissions() {
                if !set.contains(p) {
                    set.push(*p);
                }
            }
            cur = r.inherits();
        }
        set
    }
}

/// 权限作用域：限制角色生效的资源边界
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Scope {
    Global,            // 全局生效
    Xuanji(String),  // 仅指定璇玑
    Task(String),      // 仅指定任务
}

/// 角色绑定：将某角色（带作用域）授予某成员
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoleBinding {
    pub member_id: String,
    pub role: Role,
    pub scope: Scope,
}

impl RoleBinding {
    pub fn global(role: Role, member_id: &str) -> Self {
        Self {
            member_id: member_id.to_string(),
            role,
            scope: Scope::Global,
        }
    }
    pub fn xuanji(role: Role, member_id: &str, xuanji_id: &str) -> Self {
        Self {
            member_id: member_id.to_string(),
            role,
            scope: Scope::Xuanji(xuanji_id.to_string()),
        }
    }
    pub fn task(role: Role, member_id: &str, task_id: &str) -> Self {
        Self {
            member_id: member_id.to_string(),
            role,
            scope: Scope::Task(task_id.to_string()),
        }
    }
}

/// 鉴权资源上下文
#[derive(Clone, Default)]
pub struct ResourceCtx {
    pub xuanji_id: String,
    pub task: Option<TaskResource>,
}

#[derive(Clone)]
pub struct TaskResource {
    pub id: String,
    pub xuanji_id: String,
    pub assignees: Vec<String>,
}

/// 鉴权结果
#[derive(Debug, PartialEq, Eq)]
pub enum Authz {
    Allowed,
    Denied(String),
}

/// 在给定绑定集合上执行鉴权
pub fn authorize(
    member_id: &str,
    bindings: &[RoleBinding],
    permission: Permission,
    ctx: &ResourceCtx,
) -> Authz {
    let owns_resource = ctx
        .task
        .as_ref()
        .map(|t| t.assignees.iter().any(|a| a == member_id))
        .unwrap_or(false);

    for b in bindings {
        if b.member_id != member_id {
            continue;
        }
        // 作用域检查
        let scope_ok = match &b.scope {
            Scope::Global => true,
            Scope::Xuanji(aid) => ctx.xuanji_id == *aid,
            Scope::Task(tid) => ctx
                .task
                .as_ref()
                .map(|t| t.id == *tid)
                .unwrap_or(false),
        };
        if !scope_ok {
            continue;
        }
        if b.role.effective_permissions().contains(&permission) {
            // 所有权类权限要求调用者是资源所有者
            if permission.requires_ownership() && !owns_resource {
                continue;
            }
            return Authz::Allowed;
        }
    }
    Authz::Denied(format!(
        "成员 {} 缺少权限 {}（作用域: {}）",
        member_id,
        permission.as_str(),
        scope_label(ctx)
    ))
}

fn scope_label(ctx: &ResourceCtx) -> String {
    if let Some(t) = &ctx.task {
        format!("task={}", t.id)
    } else {
        format!("xuanji={}", ctx.xuanji_id)
    }
}
