//! 编排层：璇玑系统门面 + 事件反应器
//!
//! `XuanjiSystem` 聚合四个服务，并在每次写操作前做 RBAC 鉴权（require），
//! 实现「鉴权 → 领域动作 → 事件发布」的统一数据流。
//! `Reactor` 订阅事件总线，把领域事件翻译为系统消息与成员通知，
//! 实现「任务变更 → 自动通信」的闭环。
//!
//! 企业级增强：内聚 `AppConfig`（12-Factor 配置）、`Metrics`（可观测性）、
//! `RateLimiter`（安全防护），并由 `EventBus` 共享事件计数器。
use std::sync::Arc;

use tokio::task::JoinHandle;

use crate::config::AppConfig;
use crate::error::*;
use crate::event::{DomainEvent, EventBus};
use crate::metrics::Metrics;
use crate::model::*;
use crate::ratelimit::RateLimiter;
use crate::rbac::*;
use crate::services::*;
use crate::store::Store;

#[derive(Clone)]
pub struct XuanjiSystem {
    pub store: Arc<Store>,
    pub bus: EventBus,
    pub member: MemberService,
    pub task: TaskService,
    pub perm: PermissionService,
    pub comm: CommService,
    pub config: Arc<AppConfig>,
    pub metrics: Arc<Metrics>,
    pub ratelimiter: Arc<RateLimiter>,
}

impl Default for XuanjiSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl XuanjiSystem {
    /// 纯内存模式（测试 / 演示）：默认配置，无持久化
    pub fn new() -> Self {
        let config = Arc::new(AppConfig::default());
        Self::build(config, false)
    }

    /// 按配置构建；`persist` 为 true 时使用 SQLite 系统记录（启动时重放）
    pub fn with_config(config: AppConfig) -> Result<Self, AppError> {
        let persist = config.persist;
        Ok(Self::build(Arc::new(config), persist))
    }

    fn build(config: Arc<AppConfig>, persist: bool) -> Self {
        let store = if persist {
            match Store::open(&config.db_path()) {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    tracing::error!("持久化存储打开失败，回退到内存模式: {}", e);
                    Arc::new(Store::new())
                }
            }
        } else {
            Arc::new(Store::new())
        };
        let metrics = Arc::new(Metrics::new());
        // 审计计数器与 Store 共享同一原子，确保 /metrics 与内存状态一致
        let metrics = Arc::new(Metrics {
            audit_records: store.audit_counter.clone(),
            ..(*metrics).clone()
        });
        let bus = EventBus::with_metrics(metrics.events_published.clone());
        let ratelimiter = Arc::new(RateLimiter::new(config.rate_limit, config.rate_window_secs));
        let member = MemberService::new(store.clone(), bus.clone(), config.clone());
        let task = TaskService::new(store.clone(), bus.clone(), config.clone());
        let perm = PermissionService::new(store.clone(), bus.clone());
        let comm = CommService::new(store.clone(), bus.clone());
        Self {
            store,
            bus,
            member,
            task,
            perm,
            comm,
            config,
            metrics,
            ratelimiter,
        }
    }

    /// 启动事件反应器（后台任务，负责把事件翻译为通信/通知）
    pub fn start_reactor(&self) -> JoinHandle<()> {
        let reactor = Reactor {
            store: self.store.clone(),
            comm: self.comm.clone(),
            bus: self.bus.clone(),
        };
        tokio::spawn(async move { reactor.run().await })
    }

    /// 鉴权便捷封装（BR-18）
    ///
    /// 鉴权被拒时统一写入审计流并发布 `AuthzDenied` 事件——越权尝试必须可追溯，
    /// 否则 `rbac.rs` 中「鉴权失败由编排层统一记录审计事件」的设计承诺无法成立。
    pub async fn require(
        &self,
        member_id: &str,
        perm: Permission,
        ctx: &ResourceCtx,
    ) -> Result<()> {
        match self.perm.authorize(member_id, perm, ctx).await {
            Ok(()) => Ok(()),
            Err(e) => {
                let scope = match &ctx.task {
                    Some(t) => format!("task={}", t.id),
                    None => format!("xuanji={}", ctx.xuanji_id),
                };
                let resource = ctx
                    .task
                    .as_ref()
                    .map(|t| t.id.clone())
                    .unwrap_or_else(|| ctx.xuanji_id.clone());
                let reason = e.to_string();
                self.store
                    .append_audit(AuditRecord {
                        id: new_id("aud"),
                        action: AuditAction::AuthzDenied,
                        actor: member_id.to_string(),
                        resource,
                        permission: Some(perm.as_str().to_string()),
                        scope: scope.clone(),
                        detail: reason.clone(),
                        at: chrono::Utc::now(),
                    })
                    .await;
                self.bus.publish(DomainEvent::AuthzDenied {
                    member_id: member_id.to_string(),
                    permission: perm.as_str().to_string(),
                    scope,
                    reason,
                });
                Err(e)
            }
        }
    }

    /// 审计流查询（需 `audit:view` 权限）
    pub async fn list_audit(&self, actor: &str, xuanji_id: &str) -> Result<Vec<AuditRecord>> {
        self.require(actor, Permission::AuditView, &self.ctx_xuanji(xuanji_id))
            .await?;
        Ok(self.store.list_audit().await)
    }

    fn ctx_xuanji(&self, xuanji_id: &str) -> ResourceCtx {
        ResourceCtx {
            xuanji_id: xuanji_id.to_string(),
            task: None,
        }
    }

    async fn ctx_task(&self, task_id: &str) -> Result<ResourceCtx> {
        let t = self
            .store
            .get_task(task_id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("任务 {task_id}")))?;
        Ok(ResourceCtx {
            xuanji_id: t.xuanji_id.clone(),
            task: Some(crate::rbac::TaskResource {
                id: t.id.clone(),
                xuanji_id: t.xuanji_id.clone(),
                assignees: t.assignees.clone(),
            }),
        })
    }

    // ---------- 引导：创建璇玑与首位管理员 ----------
    pub async fn bootstrap(
        &self,
        xuanji_name: &str,
        admin_name: &str,
        admin_email: &str,
    ) -> Result<(Xuanji, Member, String)> {
        let xuanji = Xuanji {
            id: new_id("aln"),
            name: xuanji_name.to_string(),
            description: String::new(),
            created_at: chrono::Utc::now(),
        };
        self.store.create_xuanji(xuanji.clone()).await;
        let admin = Member {
            id: new_id("mem"),
            xuanji_id: xuanji.id.clone(),
            name: admin_name.to_string(),
            email: admin_email.to_string(),
            title: "璇玑管理员".to_string(),
            expertise: vec![],
            tier: Tier::Principal,
            status: MemberStatus::Active,
            joined_at: chrono::Utc::now(),
        };
        self.store.create_member(admin.clone()).await;
        // 授予全局管理员角色
        self.perm
            .assign_role(RoleBinding::global(Role::XuanjiAdmin, &admin.id))
            .await;
        // 璇玑大厅频道
        self.store.ensure_xuanji_channel(&xuanji.id).await;
        // 签发登录令牌
        let token = new_id("tok");
        self.store.set_token(&token, &admin.id).await;
        Ok((xuanji, admin, token))
    }

    // ---------- 鉴权后的高层操作 ----------
    pub async fn invite_member(
        &self,
        actor: &str,
        input: &InviteInput,
    ) -> Result<Member> {
        self.require(actor, Permission::MemberInvite, &self.ctx_xuanji(&input.xuanji_id))
            .await?;
        self.member
            .invite(actor, input)
            .await
    }

    pub async fn create_task(
        &self,
        actor: &str,
        xuanji_id: &str,
        title: &str,
        description: &str,
        priority: Priority,
    ) -> Result<Task> {
        self.require(actor, Permission::TaskCreate, &self.ctx_xuanji(xuanji_id))
            .await?;
        self.task
            .create(xuanji_id, actor, title, description, priority)
            .await
    }

    pub async fn assign_task(
        &self,
        actor: &str,
        task_id: &str,
        assignees: Vec<String>,
    ) -> Result<Task> {
        self.require(actor, Permission::TaskAssign, &self.ctx_task(task_id).await?)
            .await?;
        self.task.assign(task_id, actor, assignees).await
    }

    pub async fn transition_task(
        &self,
        actor: &str,
        task_id: &str,
        to: TaskStatus,
    ) -> Result<Task> {
        // 协调员/管理员可推进任意任务；专家仅可推进自己被分派的任务。
        //
        // 第一次是**试探**（用 perm.authorize，不落审计）：专家天然不具备 TransitionAll，
        // 若在此记审计会让每次正常操作都产生一条噪声拒绝记录。
        // 第二次是**终局裁决**（用 require，落审计）：确实无权时才是真正的越权尝试。
        let ctx = self.ctx_task(task_id).await?;
        if self.perm.authorize(actor, Permission::TaskTransitionAll, &ctx).await.is_ok() {
            return self.task.transition(task_id, actor, to).await;
        }
        self.require(actor, Permission::TaskTransitionOwn, &ctx).await?;
        self.task.transition(task_id, actor, to).await
    }

    pub async fn comment_task(&self, actor: &str, task_id: &str, body: &str) -> Result<Message> {
        self.require(actor, Permission::TaskComment, &self.ctx_task(task_id).await?)
            .await?;
        self.task.comment(task_id, actor, body).await
    }

    /// 关注任务（NFR-09：max_watchers 配额）
    ///
    /// 发起者即关注者（watcher）。鉴权作用域复用 `TaskComment`：
    /// 仅能参与本璇玑任务协作（评论级权限）的成员可关注，跨璇玑关注被拒。
    pub async fn watch_task(&self, actor: &str, task_id: &str) -> Result<Task> {
        self.require(actor, Permission::TaskComment, &self.ctx_task(task_id).await?)
            .await?;
        self.task.watch(task_id, actor).await
    }

    /// 在频道中发送消息（按频道类型做权限与作用域校验）
    pub async fn send_channel_message(
        &self,
        actor: &str,
        channel_id: &str,
        body: &str,
    ) -> Result<Message> {
        let ch = self
            .store
            .get_channel(channel_id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("频道 {channel_id}")))?;
        let perm = match &ch.kind {
            ChannelKind::Xuanji => Permission::CommSendXuanji,
            ChannelKind::Task(_) => Permission::CommSendTask,
            ChannelKind::Direct(_) => Permission::CommSendDirect,
        };
        let ctx = ResourceCtx {
            xuanji_id: ch.xuanji_id.clone(),
            task: match &ch.kind {
                ChannelKind::Task(tid) => Some(crate::rbac::TaskResource {
                    id: tid.clone(),
                    xuanji_id: ch.xuanji_id.clone(),
                    assignees: vec![],
                }),
                _ => None,
            },
        };
        self.require(actor, perm, &ctx).await?;
        self.comm.send_message(channel_id, actor, body, MessageKind::Chat).await
    }
}

/// 事件反应器：把领域事件翻译为系统消息与通知
pub struct Reactor {
    store: Arc<Store>,
    comm: CommService,
    bus: EventBus,
}

impl Reactor {
    pub fn new(store: Arc<Store>, comm: CommService, bus: EventBus) -> Self {
        Self { store, comm, bus }
    }
}

impl Reactor {
    /// 处理单个事件（幂等、可测试）
    pub async fn handle(&self, e: &DomainEvent) {
        match e {
            DomainEvent::MemberInvited {
                member_id,
                xuanji_id,
                by,
            } => {
                let name = self.name_of(by).await;
                self.comm
                    .notify(
                        member_id,
                        "璇玑邀请",
                        &format!("你已被 {name} 邀请加入璇玑，请激活账号"),
                        None,
                    )
                    .await;
                self.sys_msg(xuanji_id, &format!("{name} 邀请了新成员加入璇玑"))
                    .await;
            }
            DomainEvent::MemberActivated { member_id } => {
                self.comm
                    .notify(member_id, "账号已激活", "你的璇玑账号已激活，可以开始协作", None)
                    .await;
            }
            DomainEvent::MemberStatusChanged { member_id, status } => {
                self.comm
                    .notify(member_id, "账号状态变更", &format!("你的账号状态已变更为 {status}"), None)
                    .await;
            }
            DomainEvent::TaskCreated {
                task_id,
                xuanji_id,
                by,
            } => {
                let t = self.store.get_task(task_id).await;
                let title = t.as_ref().map(|x| x.title.clone()).unwrap_or_default();
                let name = self.name_of(by).await;
                self.sys_msg(xuanji_id, &format!("{name} 创建了任务《{title}》"))
                    .await;
            }
            DomainEvent::TaskAssigned {
                task_id,
                assignees,
                by,
            } => {
                let t = self.store.get_task(task_id).await;
                if let Some(t) = &t {
                    let ch = self.store.task_channel(&t.xuanji_id, task_id).await;
                    for a in assignees {
                        self.store.add_channel_member(&ch.id, a).await;
                        self.comm
                            .notify(
                                a,
                                "新任务分派",
                                &format!("你被分派到任务《{}》", t.title),
                                Some(task_id),
                            )
                            .await;
                    }
                    let name = self.name_of(by).await;
                    self.comm
                        .send_message(
                            &ch.id,
                            "system",
                            &format!("{name} 将任务分派给 {} 位专家", assignees.len()),
                            MessageKind::System,
                        )
                        .await
                        .ok();
                }
            }
            DomainEvent::TaskStatusChanged {
                task_id,
                from,
                to,
                by,
            } => {
                let t = self.store.get_task(task_id).await;
                if let Some(t) = &t {
                    let ch = self.store.task_channel(&t.xuanji_id, task_id).await;
                    let name = self.name_of(by).await;
                    self.comm
                        .send_message(
                            &ch.id,
                            "system",
                            &format!("任务状态：{} → {}（操作人：{}）", from.label(), to.label(), name),
                            MessageKind::System,
                        )
                        .await
                        .ok();
                    for a in t.assignees.iter().chain(t.watchers.iter()) {
                        self.comm
                            .notify(
                                a,
                                "任务状态更新",
                                &format!("《{}》状态变为 {}", t.title, to.label()),
                                Some(task_id),
                            )
                            .await;
                    }
                }
            }
            DomainEvent::TaskCommented { .. } | DomainEvent::MessagePosted { .. } => {
                // 评论/消息本身已是通信内容，无需再通知，避免回声
            }
            DomainEvent::AuthzDenied { .. } => {
                // 越权尝试不向当事人回执（避免成为权限探测的反馈信道），
                // 仅由 require() 落审计流供审计员事后核查
            }
        }
    }

    async fn run(self) {
        let mut rx = self.bus.subscribe();
        while let Ok(ev) = rx.recv().await {
            self.handle(&ev).await;
        }
    }

    async fn name_of(&self, member_id: &str) -> String {
        self.store
            .get_member(member_id)
            .await
            .map(|m| m.name)
            .unwrap_or_else(|| member_id.to_string())
    }

    async fn sys_msg(&self, xuanji_id: &str, body: &str) {
        let ch = self.store.ensure_xuanji_channel(xuanji_id).await;
        self.comm
            .send_message(&ch.id, "system", body, MessageKind::System)
            .await
            .ok();
    }
}
