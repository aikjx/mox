//! 端到端集成测试：成员管理 / 任务协作 / 权限分配 / 通信机制
//! + 企业级能力：SQLite 持久化重放、令牌哈希、配额强制、限流
use xuanji_system::config::{AppConfig, Backend};
use xuanji_system::event::DomainEvent;
use xuanji_system::model::{InviteInput, Priority, TaskStatus, Tier};
use xuanji_system::orchestrator::{XuanjiSystem, Reactor};
use xuanji_system::rbac::{Permission, ResourceCtx, Role, RoleBinding, Scope};
use xuanji_system::store::Store;
use std::sync::Arc;

/// 分配一个临时 SQLite 数据库路径。
///
/// 关键点：同一测试二进制在多个线程并行执行，若仅用 `process::id()` 作目录名，
/// 所有调用会共享同一个 `xuanji.db` 文件，导致并发 `remove_dir_all` / `Connection::open`
/// 互相踩踏（数据库被锁 / 文件被删），`Store::open` 失败并 `.unwrap()` 恐慌。
/// 因此每次调用必须生成**唯一**目录（进程内原子计数 + pid），彻底隔离，避免串扰。
fn temp_db() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("xuanji_it_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("xuanji.db").to_string_lossy().into_owned()
}

#[tokio::test]
async fn full_lifecycle_and_rbac() {
    let sys = XuanjiSystem::new();
    let (aln, admin, _token) = sys
        .bootstrap("测试璇玑", "管理员", "admin@t.io")
        .await
        .unwrap();

    // 邀请 + 激活专家
    let e1 = sys
        .invite_member(
            &admin.id,
            &xuanji_system::model::InviteInput {
                xuanji_id: aln.id.clone(),
                name: "专家A".into(),
                email: "a@t.io".into(),
                title: "算法".into(),
                expertise: vec!["优化".into()],
                tier: Tier::Lead,
            },
        )
        .await
        .unwrap();
    assert_eq!(e1.status, xuanji_system::model::MemberStatus::Invited);
    let e1 = sys.member.activate(&e1.id, &admin.id).await.unwrap();
    assert_eq!(e1.status, xuanji_system::model::MemberStatus::Active);

    // 创建任务
    let t = sys
        .create_task(&admin.id, &aln.id, "压测", "desc", Priority::High)
        .await
        .unwrap();
    assert_eq!(t.status, TaskStatus::Draft);

    // 分派
    let t = sys.assign_task(&admin.id, &t.id, vec![e1.id.clone()]).await.unwrap();
    assert_eq!(t.status, TaskStatus::Assigned);
    assert!(t.assignees.contains(&e1.id));

    // 被分派专家可推进状态
    let t = sys
        .transition_task(&e1.id, &t.id, TaskStatus::InProgress)
        .await
        .unwrap();
    assert_eq!(t.status, TaskStatus::InProgress);

    // 非法状态迁移被拒绝
    let bad = sys.transition_task(&e1.id, &t.id, TaskStatus::Draft).await;
    assert!(bad.is_err());

    // 非分派者推进被 RBAC 拦截
    let e2 = sys
        .invite_member(&admin.id, &xuanji_system::model::InviteInput {
            xuanji_id: aln.id.clone(),
            name: "专家B".into(),
            email: "b@t.io".into(),
            title: "安全".into(),
            expertise: vec!["安全".into()],
            tier: Tier::Senior,
        })
        .await
        .unwrap();
    let e2 = sys.member.activate(&e2.id, &admin.id).await.unwrap();
    let denied = sys.transition_task(&e2.id, &t.id, TaskStatus::InReview).await;
    assert!(denied.is_err(), "非分派专家不应能推进任务");

    // 评论写入任务频道
    let msg = sys.comment_task(&e1.id, &t.id, "进度 50%").await.unwrap();
    assert_eq!(msg.body, "进度 50%");

    // 权限矩阵校验
    let ctx_all = ResourceCtx {
        xuanji_id: aln.id.clone(),
        task: None,
    };
    assert!(sys.perm.authorize(&admin.id, Permission::TaskViewAll, &ctx_all).await.is_ok());
    let task_res = xuanji_system::rbac::TaskResource {
        id: t.id.clone(),
        xuanji_id: aln.id.clone(),
        assignees: vec![e1.id.clone()],
    };
    let ctx_task = ResourceCtx {
        xuanji_id: aln.id.clone(),
        task: Some(task_res),
    };
    // 专家对自己的任务可编辑/推进（Own）
    assert!(sys.perm.authorize(&e1.id, Permission::TaskEditOwn, &ctx_task).await.is_ok());
    assert!(sys.perm.authorize(&e1.id, Permission::TaskTransitionOwn, &ctx_task).await.is_ok());
    // 专家对未分派的任务不可编辑
    let other = xuanji_system::rbac::TaskResource {
        id: "other".into(),
        xuanji_id: aln.id.clone(),
        assignees: vec!["someone".into()],
    };
    let ctx_other = ResourceCtx {
        xuanji_id: aln.id.clone(),
        task: Some(other),
    };
    assert!(sys.perm.authorize(&e1.id, Permission::TaskEditOwn, &ctx_other).await.is_err());
}

#[tokio::test]
async fn event_reactor_produces_notifications() {
    let sys = XuanjiSystem::new();
    let (aln, _admin, _token) = sys.bootstrap("璇玑X", "A", "a@x.io").await.unwrap();
    let expert = sys
        .invite_member(&_admin.id, &xuanji_system::model::InviteInput {
            xuanji_id: aln.id.clone(),
            name: "E".into(),
            email: "e@x.io".into(),
            title: "算法".into(),
            expertise: vec![],
            tier: Tier::Senior,
        })
        .await
        .unwrap();
    let expert = sys.member.activate(&expert.id, &_admin.id).await.unwrap();
    let task = sys
        .create_task(&_admin.id, &aln.id, "T1", "d", Priority::Medium)
        .await
        .unwrap();

    // 直接驱动反应器处理「分派」事件，验证通知生成
    let reactor = Reactor::new(sys.store.clone(), sys.comm.clone(), sys.bus.clone());
    reactor
        .handle(&DomainEvent::TaskAssigned {
            task_id: task.id.clone(),
            assignees: vec![expert.id.clone()],
            by: _admin.id.clone(),
        })
        .await;

    let notes = sys.comm.list_notifications(&expert.id).await;
    assert!(
        notes.iter().any(|n| n.related_task.as_deref() == Some(&task.id)),
        "分派事件应产生关联任务的通知"
    );
}

#[tokio::test]
async fn role_inheritance_and_scope() {
    let sys = XuanjiSystem::new();
    let (aln, admin, _tok) = sys.bootstrap("璇玑Y", "A", "a@y.io").await.unwrap();
    let m = sys
        .invite_member(&admin.id, &xuanji_system::model::InviteInput {
            xuanji_id: aln.id.clone(),
            name: "M".into(),
            email: "m@y.io".into(),
            title: "x".into(),
            expertise: vec![],
            tier: Tier::Associate,
        })
        .await
        .unwrap();

    // 默认 Member 角色（作用域本璇玑）
    let perms = sys.perm.effective_permissions(&m.id).await;
    assert!(perms.contains(&Permission::TaskComment));
    assert!(!perms.contains(&Permission::TaskCreate)); // 普通成员不能建任务

    // 提升为协调员（全局）
    sys.perm
        .assign_role(RoleBinding::global(Role::Coordinator, &m.id))
        .await;
    let perms = sys.perm.effective_permissions(&m.id).await;
    assert!(perms.contains(&Permission::TaskCreate));
    assert!(perms.contains(&Permission::MemberInvite));

    // 作用域限制：协调员角色限定到另一璇玑时，不应在原璇玑生效
    let other = "aln_other";
    let ctx_other = ResourceCtx {
        xuanji_id: other.to_string(),
        task: None,
    };
    let scoped = RoleBinding {
        member_id: m.id.clone(),
        role: Role::Coordinator,
        scope: Scope::Xuanji(other.to_string()),
    };
    sys.perm.assign_role(scoped).await;
    // 原璇玑仍由 global 绑定覆盖
    assert!(sys.perm.authorize(&m.id, Permission::TaskCreate, &ResourceCtx { xuanji_id: aln.id.clone(), task: None }).await.is_ok());
    // 另一璇玑也允许（scoped 绑定）
    assert!(sys.perm.authorize(&m.id, Permission::TaskCreate, &ctx_other).await.is_ok());
}

// ---------------- 企业级能力测试 ----------------

#[tokio::test]
async fn store_persists_and_hashes_tokens() {
    let db = temp_db();
    {
        let s = Store::open(Backend::Sqlite, &db).await.unwrap();
        // 令牌以 SHA-256 哈希落盘，明文不存储
        s.set_token("secret-token-123", "mem_1").await;
        assert_eq!(s.member_by_token("secret-token-123").await, Some("mem_1".to_string()));
        assert_eq!(s.member_by_token("wrong-token").await, None);
    }
    // 模拟进程重启：重放 SQLite，令牌仍可用
    {
        let s2 = Store::open(Backend::Sqlite, &db).await.unwrap();
        assert_eq!(
            s2.member_by_token("secret-token-123").await,
            Some("mem_1".to_string()),
            "重启后令牌应仍可用（哈希已持久化）"
        );
    }
    let _ = std::fs::remove_dir_all(std::path::Path::new(&db).parent().unwrap());
}

#[tokio::test]
async fn persistence_survives_restart() {
    let db = temp_db();
    let cfg = AppConfig {
        persist: true,
        data_dir: std::path::Path::new(&db)
            .parent()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        ..Default::default()
    };

    let token;
    let task_id;
    let member_id;
    {
        let sys = Arc::new(XuanjiSystem::with_config(cfg.clone()).await.unwrap());
        let (aln, admin, tok) = sys
            .bootstrap("持久化璇玑", "管理员", "admin@p.io")
            .await
            .unwrap();
        token = tok;
        let e = sys
            .invite_member(
                &admin.id,
                &InviteInput {
                    xuanji_id: aln.id.clone(),
                    name: "专家C".into(),
                    email: "c@p.io".into(),
                    title: "算法".into(),
                    expertise: vec!["优化".into()],
                    tier: Tier::Lead,
                },
            )
            .await
            .unwrap();
        member_id = e.id.clone();
        sys.member.activate(&e.id, &admin.id).await.unwrap();
        let t = sys
            .create_task(&admin.id, &aln.id, "持久化任务", "desc", Priority::High)
            .await
            .unwrap();
        let t = sys
            .assign_task(&admin.id, &t.id, vec![e.id.clone()])
            .await
            .unwrap();
        task_id = t.id.clone();
        // 落盘：释放所有引用，关闭 SQLite 连接
        drop(sys);
    }

    // 重启：从 SQLite 重放
    let sys2 = Arc::new(XuanjiSystem::with_config(cfg).await.unwrap());
    assert!(
        sys2.store.get_task(&task_id).await.is_some(),
        "重启后任务应仍存在"
    );
    assert!(
        sys2.store.get_member(&member_id).await.is_some(),
        "重启后成员应仍存在"
    );
    // 令牌哈希重放后仍能认证（哈希已持久化，明文不落地）
    assert!(
        sys2.store.member_by_token(&token).await.is_some(),
        "重启后管理员令牌应可认证"
    );
    // 审计记录已在重放中恢复
    assert!(
        !sys2.store.list_audit().await.is_empty(),
        "重启后审计流应已重放"
    );
    let _ = std::fs::remove_dir_all(std::path::Path::new(&db).parent().unwrap());
}

#[tokio::test]
async fn quota_enforcement() {
    // 成员配额：admin 已占 1 个名额，max_members=1 时不能再邀请
    let mut cfg = AppConfig::default();
    cfg.quotas.max_members = 1;
    let sys = XuanjiSystem::with_config(cfg).await.unwrap();
    let (aln, admin, _t) = sys.bootstrap("配额璇玑", "A", "a@q.io").await.unwrap();
    let r = sys
        .invite_member(
            &admin.id,
            &InviteInput {
                xuanji_id: aln.id.clone(),
                name: "超额成员".into(),
                email: "over@q.io".into(),
                title: "x".into(),
                expertise: vec![],
                tier: Tier::Associate,
            },
        )
        .await;
    assert!(r.is_err(), "超过成员上限应被拒绝");

    // 任务配额：max_tasks=1 时第二个任务应被拒绝
    let mut cfg2 = AppConfig::default();
    cfg2.quotas.max_tasks = 1;
    let sys2 = XuanjiSystem::with_config(cfg2).await.unwrap();
    let (aln2, admin2, _t2) = sys2.bootstrap("配额璇玑2", "A", "a@q2.io").await.unwrap();
    sys2
        .create_task(&admin2.id, &aln2.id, "任务1", "d", Priority::Low)
        .await
        .unwrap();
    let r2 = sys2
        .create_task(&admin2.id, &aln2.id, "任务2", "d", Priority::Low)
        .await;
    assert!(r2.is_err(), "超过任务上限应被拒绝");
}

// ---------------- PostgreSQL 端到端（可选） ----------------
// 需设置环境变量 XUANJI_TEST_PG_URL（如 postgres://postgres:pass@127.0.0.1:35432/xuanji_test），
// 未设置时测试自动跳过（便于无 PG 环境的 CI）。

fn pg_test_url() -> Option<String> {
    std::env::var("XUANJI_TEST_PG_URL").ok()
}

#[tokio::test]
async fn postgres_store_persists_and_replays() {
    let Some(url) = pg_test_url() else {
        eprintln!("SKIP postgres_store_persists_and_replays: 未设置 XUANJI_TEST_PG_URL");
        return;
    };
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let token = format!("pg_token_{}_{}", std::process::id(), ts);
    let member = format!("pg_member_{}_{}", std::process::id(), ts);
    {
        let s = Store::open(Backend::Postgres, &url).await.unwrap();
        s.set_token(&token, &member).await;
        assert_eq!(s.member_by_token(&token).await, Some(member.clone()));
    }
    // 模拟重启：重放 PG，令牌仍可用
    {
        let s2 = Store::open(Backend::Postgres, &url).await.unwrap();
        assert_eq!(
            s2.member_by_token(&token).await,
            Some(member.clone()),
            "重启后令牌应仍可用（PG 持久化）"
        );
    }
}

#[tokio::test]
async fn postgres_system_survives_restart() {
    let Some(url) = pg_test_url() else {
        eprintln!("SKIP postgres_system_survives_restart: 未设置 XUANJI_TEST_PG_URL");
        return;
    };
    let cfg = AppConfig {
        persist: true,
        backend: Backend::Postgres,
        db_url: url,
        ..Default::default()
    };
    let token;
    let task_id;
    {
        let sys = Arc::new(XuanjiSystem::with_config(cfg.clone()).await.unwrap());
        let (aln, admin, tok) = sys
            .bootstrap("PG璇玑", "管理员", "pg@p.io")
            .await
            .unwrap();
        token = tok;
        let t = sys
            .create_task(&admin.id, &aln.id, "PG任务", "desc", Priority::High)
            .await
            .unwrap();
        let t = sys
            .assign_task(&admin.id, &t.id, vec![admin.id.clone()])
            .await
            .unwrap();
        task_id = t.id.clone();
        drop(sys);
    }
    // 重启：从 PG 重放
    let sys2 = Arc::new(XuanjiSystem::with_config(cfg).await.unwrap());
    assert!(
        sys2.store.get_task(&task_id).await.is_some(),
        "重启后任务应仍存在（PG 持久化）"
    );
    assert!(
        sys2.store.member_by_token(&token).await.is_some(),
        "重启后管理员令牌应可认证（PG 持久化）"
    );
}
