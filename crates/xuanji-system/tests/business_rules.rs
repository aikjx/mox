//! 企业级业务规则验收测试
//!
//! 与 `docs/xuanji-expert-business-requirements.md` 的业务规则编号一一对应。
//! 每条规则至少覆盖一条正向路径与一条负向路径。
use xuanji_system::config::{AppConfig, Quotas};
use xuanji_system::model::{
    AuditAction, InviteInput, MemberStatus, Priority, TaskStatus, Tier,
};
use xuanji_system::orchestrator::XuanjiSystem;

/// 便捷：组建璇玑并返回 (系统, 璇玑ID, 管理员ID)
async fn setup(name: &str, email: &str) -> (XuanjiSystem, String, String) {
    let sys = XuanjiSystem::new();
    let (aln, admin, _tok) = sys.bootstrap(name, "管理员", email).await.unwrap();
    (sys, aln.id, admin.id)
}

/// 便捷：在指定系统内邀请并激活一名专家
async fn active_expert(
    sys: &XuanjiSystem,
    xuanji_id: &str,
    admin_id: &str,
    name: &str,
    email: &str,
) -> String {
    let m = sys
        .invite_member(
            admin_id,
            &InviteInput {
                xuanji_id: xuanji_id.to_string(),
                name: name.to_string(),
                email: email.to_string(),
                title: "专家".into(),
                expertise: vec![],
                tier: Tier::Senior,
            },
        )
        .await
        .unwrap();
    sys.member.activate(&m.id, admin_id).await.unwrap().id
}

// ─────────────────────────────────────────────────────────────
// BR-04 邀请幂等：同璇玑内 email 唯一
// ─────────────────────────────────────────────────────────────
#[tokio::test]
async fn br04_invite_is_idempotent_per_email() {
    let (sys, aln, admin) = setup("幂等璇玑", "admin@i.io").await;
    let input = InviteInput {
        xuanji_id: aln.clone(),
        name: "专家A".into(),
        email: "dup@i.io".into(),
        title: "算法".into(),
        expertise: vec![],
        tier: Tier::Senior,
    };
    // 正向：首次邀请成功
    sys.invite_member(&admin, &input).await.unwrap();
    assert_eq!(sys.member.list(&aln).await.len(), 2, "管理员 + 专家A");

    // 负向：同 email 重复邀请被拒，且成员数不增
    let again = sys.invite_member(&admin, &input).await;
    assert!(matches!(
        again,
        Err(xuanji_system::error::AppError::Conflict(_))
    ), "同璇玑同 email 重复邀请应返回 Conflict，实际: {again:?}");
    assert_eq!(sys.member.list(&aln).await.len(), 2, "重复邀请不得产生新成员");

    // 大小写与首尾空格不敏感
    let mixed = InviteInput { email: "  DUP@I.IO ".into(), ..input.clone() };
    assert!(sys.invite_member(&admin, &mixed).await.is_err(), "email 比较应忽略大小写与空格");
}

// ─────────────────────────────────────────────────────────────
// BR-07 分派身份三重校验（安全 P0：防跨租户权限提升）
// ─────────────────────────────────────────────────────────────
#[tokio::test]
async fn br07_assign_validates_assignee_identity() {
    let (sys, aln, admin) = setup("璇玑A", "admin@a.io").await;
    let expert = active_expert(&sys, &aln, &admin, "专家A", "a@a.io").await;
    let task = sys
        .create_task(&admin, &aln, "任务", "d", Priority::High)
        .await
        .unwrap();

    // 负向①：分派不存在的成员
    let r = sys.assign_task(&admin, &task.id, vec!["mem_ghost".into()]).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::BadRequest(_))),
        "分派不存在成员应 BadRequest，实际: {r:?}");

    // 负向②：分派他璇玑成员（跨租户权限提升的核心攻击面）
    let (sys_b, aln_b, admin_b) = setup("璇玑B", "admin@b.io").await;
    let outsider = active_expert(&sys_b, &aln_b, &admin_b, "外人", "x@b.io").await;
    // 把外人「搬」进 A 的存储，模拟持有合法成员 ID 但归属其他璇玑的场景
    let outsider_member = sys_b.member.get(&outsider).await.unwrap();
    sys.store.create_member(outsider_member).await;
    let r = sys.assign_task(&admin, &task.id, vec![outsider.clone()]).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::Forbidden(_))),
        "跨璇玑分派应 Forbidden，实际: {r:?}");

    // 负向③：分派未激活成员
    let invited = sys
        .invite_member(&admin, &InviteInput {
            xuanji_id: aln.clone(),
            name: "待激活".into(),
            email: "pending@a.io".into(),
            title: "算法".into(),
            expertise: vec![],
            tier: Tier::Associate,
        })
        .await
        .unwrap();
    let r = sys.assign_task(&admin, &task.id, vec![invited.id.clone()]).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::InvalidState(_))),
        "分派 Invited 成员应 InvalidState，实际: {r:?}");

    // 负向④：分派已停权成员
    sys.member.set_status(&expert, MemberStatus::Suspended).await.unwrap();
    let r = sys.assign_task(&admin, &task.id, vec![expert.clone()]).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::InvalidState(_))),
        "分派 Suspended 成员应 InvalidState，实际: {r:?}");

    // 负向⑤：名单内重复
    sys.member.set_status(&expert, MemberStatus::Active).await.unwrap();
    let r = sys.assign_task(&admin, &task.id, vec![expert.clone(), expert.clone()]).await;
    assert!(r.is_err(), "分派名单重复应被拒");

    // 正向：合法分派后 assignees 精确等于入参，且状态推进为 Assigned
    let t = sys.assign_task(&admin, &task.id, vec![expert.clone()]).await.unwrap();
    assert_eq!(t.assignees, vec![expert.clone()]);
    assert_eq!(t.status, TaskStatus::Assigned);

    // 越权提升已被阻断：外人始终不在 assignees 中，因此拿不到 own 类权限
    assert!(!t.assignees.contains(&outsider), "他璇玑成员不得出现在分派名单");
    let escalate = sys.transition_task(&outsider, &t.id, TaskStatus::InProgress).await;
    assert!(escalate.is_err(), "未合法分派者不得推进任务状态");
}

// ─────────────────────────────────────────────────────────────
// BR-10 完成门禁（Definition of Done）
// ─────────────────────────────────────────────────────────────
#[tokio::test]
async fn br10_done_gate_blocks_incomplete_work() {
    let (sys, aln, admin) = setup("门禁璇玑", "admin@g.io").await;
    let expert = active_expert(&sys, &aln, &admin, "专家", "e@g.io").await;

    // 前置依赖任务（故意不完成）
    let dep = sys.create_task(&admin, &aln, "前置任务", "d", Priority::Medium).await.unwrap();
    // 主任务
    let main = sys.create_task(&admin, &aln, "主任务", "d", Priority::High).await.unwrap();

    sys.task.add_dependency(&main.id, &dep.id).await.unwrap();
    let main = sys.task.add_subtask(&main.id, "子任务1").await.unwrap();
    let sub_id = main.subtasks[0].id.clone();

    // 推到 InReview
    sys.assign_task(&admin, &main.id, vec![expert.clone()]).await.unwrap();
    sys.transition_task(&admin, &main.id, TaskStatus::InProgress).await.unwrap();
    sys.transition_task(&admin, &main.id, TaskStatus::InReview).await.unwrap();

    // 负向①：子任务未完成 → 拒绝 Done
    let r = sys.transition_task(&admin, &main.id, TaskStatus::Done).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::InvalidState(_))),
        "存在未完成子任务时不得进入 Done，实际: {r:?}");

    // 完成子任务
    sys.task.toggle_subtask(&main.id, &sub_id).await.unwrap();

    // 负向②：依赖任务未完成 → 仍拒绝 Done
    let r = sys.transition_task(&admin, &main.id, TaskStatus::Done).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::InvalidState(_))),
        "前置依赖未完成时不得进入 Done，实际: {r:?}");

    // 完成前置依赖
    sys.assign_task(&admin, &dep.id, vec![expert.clone()]).await.unwrap();
    sys.transition_task(&admin, &dep.id, TaskStatus::InProgress).await.unwrap();
    sys.transition_task(&admin, &dep.id, TaskStatus::InReview).await.unwrap();
    sys.transition_task(&admin, &dep.id, TaskStatus::Done).await.unwrap();

    // 正向：门禁全部满足 → 允许 Done
    let done = sys.transition_task(&admin, &main.id, TaskStatus::Done).await.unwrap();
    assert_eq!(done.status, TaskStatus::Done);
}

// ─────────────────────────────────────────────────────────────
// BR-11 依赖图必须是 DAG；BR-12 终态不可迁出
// ─────────────────────────────────────────────────────────────
#[tokio::test]
async fn br11_dependency_graph_is_dag() {
    let (sys, aln, admin) = setup("依赖璇玑", "admin@d.io").await;
    let t1 = sys.create_task(&admin, &aln, "T1", "d", Priority::Low).await.unwrap();
    let t2 = sys.create_task(&admin, &aln, "T2", "d", Priority::Low).await.unwrap();
    let t3 = sys.create_task(&admin, &aln, "T3", "d", Priority::Low).await.unwrap();

    // 负向①：自依赖
    assert!(sys.task.add_dependency(&t1.id, &t1.id).await.is_err(), "不得自依赖");

    // 负向②：依赖不存在的任务
    assert!(sys.task.add_dependency(&t1.id, "task_ghost").await.is_err(), "不得依赖不存在任务");

    // 正向：构造链 t3 → t2 → t1（t3 依赖 t2，t2 依赖 t1）
    sys.task.add_dependency(&t2.id, &t1.id).await.unwrap();
    sys.task.add_dependency(&t3.id, &t2.id).await.unwrap();

    // 幂等：重复添加同一依赖不报错也不重复写入
    let t2b = sys.task.add_dependency(&t2.id, &t1.id).await.unwrap();
    assert_eq!(t2b.depends_on.len(), 1, "重复依赖不得重复写入");

    // 负向③：直接成环 t1 → t2（t2 已依赖 t1）
    assert!(sys.task.add_dependency(&t1.id, &t2.id).await.is_err(), "不得形成直接环");

    // 负向④：间接成环 t1 → t3（t3 →t2 → t1）
    let r = sys.task.add_dependency(&t1.id, &t3.id).await;
    assert!(r.is_err(), "不得形成间接环，实际: {r:?}");

    // 负向⑤：跨璇玑依赖
    let (sys2, aln2, admin2) = setup("依赖璇玑2", "admin@d2.io").await;
    let other = sys2.create_task(&admin2, &aln2, "外部任务", "d", Priority::Low).await.unwrap();
    sys.store.create_task(other.clone()).await; // 搬入本地存储，模拟持有合法任务 ID
    let r = sys.task.add_dependency(&t1.id, &other.id).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::Forbidden(_))),
        "跨璇玑依赖应 Forbidden，实际: {r:?}");
}

#[tokio::test]
async fn br12_terminal_task_status_cannot_transition_out() {
    let (sys, aln, admin) = setup("终态璇玑", "admin@t2.io").await;
    let t = sys.create_task(&admin, &aln, "T", "d", Priority::Low).await.unwrap();
    // Draft → Cancelled（终态）
    let t = sys.transition_task(&admin, &t.id, TaskStatus::Cancelled).await.unwrap();
    assert!(t.status.is_terminal());
    for to in [TaskStatus::Draft, TaskStatus::Assigned, TaskStatus::InProgress,
               TaskStatus::InReview, TaskStatus::Done] {
        assert!(sys.transition_task(&admin, &t.id, to).await.is_err(),
            "终态 Cancelled 不得迁出到 {}", to.label());
    }
}

// ─────────────────────────────────────────────────────────────
// BR-21 成员状态机
// ─────────────────────────────────────────────────────────────
#[tokio::test]
async fn br21_member_status_machine_enforced() {
    let (sys, aln, admin) = setup("状态机璇玑", "admin@s.io").await;
    let m = sys
        .invite_member(&admin, &InviteInput {
            xuanji_id: aln.clone(),
            name: "专家".into(),
            email: "s@s.io".into(),
            title: "算法".into(),
            expertise: vec![],
            tier: Tier::Senior,
        })
        .await
        .unwrap();
    assert_eq!(m.status, MemberStatus::Invited);

    // 负向①：Invited 不得跳级到 Suspended（必须先激活）
    let r = sys.member.set_status(&m.id, MemberStatus::Suspended).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::InvalidState(_))),
        "Invited → Suspended 应被拒，实际: {r:?}");

    // 正向：完整生命周期 Invited → Active → Suspended → Active → Left
    assert_eq!(sys.member.activate(&m.id, &admin).await.unwrap().status, MemberStatus::Active);
    assert_eq!(
        sys.member.set_status(&m.id, MemberStatus::Suspended).await.unwrap().status,
        MemberStatus::Suspended
    );
    assert_eq!(
        sys.member.set_status(&m.id, MemberStatus::Active).await.unwrap().status,
        MemberStatus::Active
    );
    assert_eq!(
        sys.member.set_status(&m.id, MemberStatus::Left).await.unwrap().status,
        MemberStatus::Left
    );

    // 负向②：Left 是终态，不得复活
    let r = sys.member.set_status(&m.id, MemberStatus::Active).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::InvalidState(_))),
        "Left → Active 应被拒（终态不可复活），实际: {r:?}");
    let r = sys.member.set_status(&m.id, MemberStatus::Suspended).await;
    assert!(r.is_err(), "Left → Suspended 应被拒");

    // 幂等：重复设置同一状态不视为错误
    assert_eq!(
        sys.member.set_status(&m.id, MemberStatus::Left).await.unwrap().status,
        MemberStatus::Left
    );
}

// ─────────────────────────────────────────────────────────────
// BR-18 鉴权失败必须留痕
// ─────────────────────────────────────────────────────────────
#[tokio::test]
async fn br18_authz_denial_is_audited() {
    let (sys, aln, admin) = setup("审计璇玑", "admin@au.io").await;
    let e1 = active_expert(&sys, &aln, &admin, "专家1", "e1@au.io").await;
    let e2 = active_expert(&sys, &aln, &admin, "专家2", "e2@au.io").await;
    let t = sys.create_task(&admin, &aln, "T", "d", Priority::High).await.unwrap();
    let t = sys.assign_task(&admin, &t.id, vec![e1.clone()]).await.unwrap();

    let before = sys.store.list_audit_by_action(AuditAction::AuthzDenied).await.len();

    // 越权尝试：e2 未被分派，试图推进 e1 的任务
    let denied = sys.transition_task(&e2, &t.id, TaskStatus::InProgress).await;
    assert!(denied.is_err(), "非分派者不得推进任务");

    let records = sys.store.list_audit_by_action(AuditAction::AuthzDenied).await;
    assert_eq!(records.len(), before + 1, "一次越权尝试应产生且仅产生一条拒绝审计");
    let r = records.last().unwrap();
    assert_eq!(r.actor, e2, "审计需记录发起者");
    assert_eq!(r.resource, t.id, "审计需记录目标资源");
    assert_eq!(r.permission.as_deref(), Some("task:transition:own"), "审计需记录涉及权限");
    assert!(r.scope.contains(&t.id), "审计需记录作用域");
    assert!(!r.detail.is_empty(), "审计需记录拒绝原因");

    // 合法操作不产生拒绝审计噪声（试探性鉴权不得落审计）
    let n = sys.store.list_audit_by_action(AuditAction::AuthzDenied).await.len();
    sys.transition_task(&e1, &t.id, TaskStatus::InProgress).await.unwrap();
    assert_eq!(
        sys.store.list_audit_by_action(AuditAction::AuthzDenied).await.len(),
        n,
        "专家的正常操作虽经过 TransitionAll 试探，也不得产生拒绝审计"
    );

    // 关键状态变更同样留痕
    assert!(
        !sys.store.list_audit_by_action(AuditAction::TaskStatusChange).await.is_empty(),
        "任务状态迁移应留痕"
    );
    assert!(
        !sys.store.list_audit_by_action(AuditAction::TaskAssign).await.is_empty(),
        "任务分派应留痕"
    );
    assert!(
        !sys.store.list_audit_by_action(AuditAction::MemberStatusChange).await.is_empty(),
        "成员状态迁移应留痕"
    );

    // 审计流查询受 audit:view 权限约束
    assert!(sys.list_audit(&admin, &aln).await.is_ok(), "管理员可查审计");
    assert!(sys.list_audit(&e1, &aln).await.is_err(), "专家无 audit:view，不可查审计");
}

/// 便捷：以指定配额配置组建璇玑，返回 (系统, 璇玑ID, 管理员ID)
async fn setup_with(
    name: &str,
    email: &str,
    quotas: Quotas,
) -> (XuanjiSystem, String, String) {
    let cfg = AppConfig { quotas, ..AppConfig::default() };
    let sys = XuanjiSystem::with_config(cfg).unwrap();
    let (aln, admin, _tok) = sys.bootstrap(name, "管理员", email).await.unwrap();
    (sys, aln.id, admin.id)
}

// ─────────────────────────────────────────────────────────────
// NFR-09 资源配额约束（I-03）：防止单一璇玑/任务无界增长
// 六个维度全部以负向边界用例固化
// ─────────────────────────────────────────────────────────────
#[tokio::test]
async fn nfr09_max_members_enforced() {
    // max_members=2：管理员(1) + e1(1) 已占满，再邀请即触顶
    let (sys, aln, admin) =
        setup_with("配额-成员", "admin@q.io", Quotas { max_members: 2, ..Quotas::default() }).await;
    let _ = active_expert(&sys, &aln, &admin, "e1", "e1@q.io").await;
    let r = sys
        .invite_member(&admin, &InviteInput {
            xuanji_id: aln.clone(),
            name: "e2".into(),
            email: "e2@q.io".into(),
            title: "算法".into(),
            expertise: vec![],
            tier: Tier::Senior,
        })
        .await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::Conflict(_))),
        "达到成员上限应 Conflict，实际: {r:?}");
    assert_eq!(sys.member.list(&aln).await.len(), 2, "触顶后成员数不得增长");
}

#[tokio::test]
async fn nfr09_max_tasks_enforced() {
    let (sys, aln, admin) =
        setup_with("配额-任务", "admin@q.io", Quotas { max_tasks: 1, ..Quotas::default() }).await;
    let _ = sys.create_task(&admin, &aln, "T1", "d", Priority::Low).await.unwrap();
    let r = sys.create_task(&admin, &aln, "T2", "d", Priority::Low).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::Conflict(_))),
        "达到任务上限应 Conflict，实际: {r:?}");
}

#[tokio::test]
async fn nfr09_max_assignees_enforced() {
    let (sys, aln, admin) = setup_with(
        "配额-分派",
        "admin@q.io",
        Quotas { max_assignees: 1, ..Quotas::default() },
    )
    .await;
    let e1 = active_expert(&sys, &aln, &admin, "e1", "e1@q.io").await;
    let e2 = active_expert(&sys, &aln, &admin, "e2", "e2@q.io").await;
    let t = sys.create_task(&admin, &aln, "T", "d", Priority::Low).await.unwrap();
    let r = sys.assign_task(&admin, &t.id, vec![e1, e2]).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::BadRequest(_))),
        "超过分派人数上限应 BadRequest，实际: {r:?}");
}

#[tokio::test]
async fn nfr09_max_subtasks_enforced() {
    let (sys, aln, admin) = setup_with(
        "配额-子任务",
        "admin@q.io",
        Quotas { max_subtasks: 1, ..Quotas::default() },
    )
    .await;
    let t = sys.create_task(&admin, &aln, "T", "d", Priority::Low).await.unwrap();
    let _ = sys.task.add_subtask(&t.id, "s1").await.unwrap();
    let r = sys.task.add_subtask(&t.id, "s2").await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::BadRequest(_))),
        "超过子任务上限应 BadRequest，实际: {r:?}");
}

#[tokio::test]
async fn nfr09_max_dependency_depth_enforced() {
    let (sys, aln, admin) = setup_with(
        "配额-依赖深度",
        "admin@q.io",
        Quotas { max_dependency_depth: 1, ..Quotas::default() },
    )
    .await;
    let t1 = sys.create_task(&admin, &aln, "T1", "d", Priority::Low).await.unwrap();
    let t2 = sys.create_task(&admin, &aln, "T2", "d", Priority::Low).await.unwrap();
    // 深度 = dep_depth(t1) + 1 = 1 + 1 = 2 > 上限 1 → 拒绝
    let r = sys.task.add_dependency(&t2.id, &t1.id).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::BadRequest(_))),
        "超过依赖深度上限应 BadRequest，实际: {r:?}");
}

#[tokio::test]
async fn nfr09_max_watchers_enforced() {
    let (sys, aln, admin) = setup_with(
        "配额-关注",
        "admin@q.io",
        Quotas { max_watchers: 1, ..Quotas::default() },
    )
    .await;
    let e1 = active_expert(&sys, &aln, &admin, "e1", "e1@q.io").await;
    let e2 = active_expert(&sys, &aln, &admin, "e2", "e2@q.io").await;
    let t = sys.create_task(&admin, &aln, "T", "d", Priority::Low).await.unwrap();
    // 正向：首位关注者成功
    let watched = sys.watch_task(&e1, &t.id).await.unwrap();
    assert_eq!(watched.watchers, vec![e1.clone()], "关注成功应写入 watchers");
    // 负向：达上限后第二位关注者被拒
    let r = sys.watch_task(&e2, &t.id).await;
    assert!(matches!(r, Err(xuanji_system::error::AppError::BadRequest(_))),
        "超过关注者上限应 BadRequest，实际: {r:?}");
    // 幂等：重复关注同一人不得报错也不得导致上限误判
    let again = sys.watch_task(&e1, &t.id).await.unwrap();
    assert_eq!(again.watchers, vec![e1.clone()], "重复关注应幂等");
}
