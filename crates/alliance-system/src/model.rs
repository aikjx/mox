//! 领域模型：联盟 / 成员 / 任务 / 通信
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 生成带前缀的全局唯一 ID
pub fn new_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

/// 专家等级（资深度）
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Tier {
    Associate, // 助理专家
    Senior,    // 资深专家
    Lead,      // 牵头专家
    Principal, // 首席专家
}

/// 成员生命周期状态（FSM）
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemberStatus {
    Invited,   // 已邀请，待激活
    Active,    // 活跃
    Suspended, // 已暂停
    Left,      // 已退出
}

impl MemberStatus {
    /// 合法状态迁移（BR-21）
    ///
    /// 与任务状态机对称：生命周期不可绕过、终态不可复活。
    /// - `Invited` 必须经 `activate()` 才能进入 `Active`，不得跳级到 `Suspended`
    /// - `Left` 是终态：已退出成员不可被改回 `Active`（防止「复活」绕过重新邀请与审批）
    pub fn can_transition(&self, to: MemberStatus) -> bool {
        use MemberStatus::*;
        matches!(
            (*self, to),
            (Invited, Active)
                | (Invited, Left)
                | (Active, Suspended)
                | (Active, Left)
                | (Suspended, Active)
                | (Suspended, Left)
        )
    }
    /// 终态：不可再迁出
    pub fn is_terminal(&self) -> bool {
        matches!(self, MemberStatus::Left)
    }
    /// 是否可承接任务（BR-05）
    pub fn can_take_task(&self) -> bool {
        matches!(self, MemberStatus::Active)
    }
    pub fn label(&self) -> &'static str {
        use MemberStatus::*;
        match self {
            Invited => "已邀请",
            Active => "活跃",
            Suspended => "已暂停",
            Left => "已退出",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    pub id: String,
    pub alliance_id: String,
    pub name: String,
    pub email: String,
    pub title: String,
    pub expertise: Vec<String>,
    pub tier: Tier,
    pub status: MemberStatus,
    pub joined_at: DateTime<Utc>,
}

/// 邀请成员的输入参数，聚合 `MemberService::invite` 与 `AllianceSystem::invite_member`
/// 的冗余字段，消解 `clippy::too_many_arguments` 并提升调用方可读性。
#[derive(Clone, Debug)]
pub struct InviteInput {
    pub alliance_id: String,
    pub name: String,
    pub email: String,
    pub title: String,
    pub expertise: Vec<String>,
    pub tier: Tier,
}

/// 联盟（多租户隔离单位）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alliance {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

/// 任务状态（FSM）
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Draft,     // 草稿
    Assigned,  // 已分派
    InProgress,// 进行中
    InReview,  // 评审中
    Done,      // 完成
    Cancelled, // 取消
}

impl TaskStatus {
    /// 合法状态迁移
    pub fn can_transition(&self, to: TaskStatus) -> bool {
        use TaskStatus::*;
        matches!(
            (*self, to),
            (Draft, Assigned)
                | (Draft, Cancelled)
                | (Assigned, InProgress)
                | (Assigned, Cancelled)
                | (InProgress, InReview)
                | (InProgress, Cancelled)
                | (InReview, Done)
                | (InReview, InProgress)
                | (InReview, Cancelled)
        )
    }
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Done | TaskStatus::Cancelled)
    }
    pub fn label(&self) -> &'static str {
        use TaskStatus::*;
        match self {
            Draft => "草稿",
            Assigned => "已分派",
            InProgress => "进行中",
            InReview => "评审中",
            Done => "完成",
            Cancelled => "取消",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub title: String,
    pub done: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub alliance_id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub created_by: String,
    pub assignees: Vec<String>,
    pub watchers: Vec<String>,
    pub subtasks: Vec<SubTask>,
    pub depends_on: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 通信频道类型
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChannelKind {
    Alliance,            // 联盟公开频道
    Task(String),        // 任务协作频道（与任务 1:1 绑定）
    Direct(Vec<String>), // 私信频道
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub alliance_id: String,
    pub kind: ChannelKind,
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageKind {
    Chat,   // 普通聊天/评论
    System, // 系统事件消息
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub sender_id: String, // 系统消息为 "system"
    pub body: String,
    pub kind: MessageKind,
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub fn kind_as_str(&self) -> &'static str {
        match self.kind {
            MessageKind::Chat => "聊天",
            MessageKind::System => "系统",
        }
    }
}

/// 审计动作类别
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditAction {
    AuthzDenied,       // 鉴权被拒
    MemberStatusChange,// 成员状态迁移
    TaskStatusChange,  // 任务状态迁移
    TaskAssign,        // 任务分派
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        use AuditAction::*;
        match self {
            AuthzDenied => "authz:denied",
            MemberStatusChange => "member:status_change",
            TaskStatusChange => "task:status_change",
            TaskAssign => "task:assign",
        }
    }
}

/// 审计记录（BR-18）：不可变、按时间追加，是越权行为与关键状态变更的追溯依据
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: String,
    pub action: AuditAction,
    /// 发起者（成员 ID）
    pub actor: String,
    /// 目标资源标识（任务 ID / 成员 ID / 联盟 ID）
    pub resource: String,
    /// 涉及的权限（鉴权类记录有值）
    pub permission: Option<String>,
    /// 作用域描述
    pub scope: String,
    /// 结论说明 / 拒绝原因
    pub detail: String,
    pub at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub member_id: String,
    pub title: String,
    pub body: String,
    pub read: bool,
    pub related_task: Option<String>,
    pub created_at: DateTime<Utc>,
}
