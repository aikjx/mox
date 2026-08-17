//! 璇玑系统：演示 / 服务入口
//!
//! 运行方式：
//! - `cargo run -p xuanji-system -- --demo` 运行端到端演示（无需网络）
//! - `cargo run -p xuanji-system` 启动 HTTP + WebSocket 服务（:3000）
use std::sync::Arc;

use xuanji_system::config::AppConfig;
use xuanji_system::model::{InviteInput, Priority, TaskStatus, Tier};
use xuanji_system::orchestrator::XuanjiSystem;
use xuanji_system::server;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--demo") {
        run_demo().await;
        return;
    }
    run_server().await;
}

/// 端到端演示：引导璇玑 → 邀请专家 → 创建/分派/推进任务 → 评论 → 验证通知与权限
async fn run_demo() {
    tracing_subscriber_wrap();
    let sys = XuanjiSystem::new();
    let _reactor = sys.start_reactor();

    println!("=== 1. 引导璇玑与管理员 ===");
    let (aln, admin, token) = sys
        .bootstrap("量子计算璇玑", "璇玑(管理员)", "admin@xuanji.io")
        .await
        .expect("bootstrap");
    println!("  璇玑: {} ({})", aln.name, aln.id);
    println!("  管理员令牌: {}", token);

    println!("\n=== 2. 邀请并激活两位专家 ===");
    let e1 = sys
        .invite_member(
            &admin.id,
            &InviteInput {
                xuanji_id: aln.id.clone(),
                name: "艾莉(算法)".into(),
                email: "ai@xuanji.io".into(),
                title: "首席算法专家".into(),
                expertise: vec!["优化".into(), "调度".into()],
                tier: Tier::Lead,
            },
        )
        .await
        .expect("invite e1");
    let e2 = sys
        .invite_member(
            &admin.id,
            &InviteInput {
                xuanji_id: aln.id.clone(),
                name: "本(安全)".into(),
                email: "ben@xuanji.io".into(),
                title: "安全专家".into(),
                expertise: vec!["安全".into()],
                tier: Tier::Senior,
            },
        )
        .await
        .expect("invite e2");
    sys.member.activate(&e1.id, &admin.id).await.unwrap();
    sys.member.activate(&e2.id, &admin.id).await.unwrap();
    println!("  已邀请: {} / {}", e1.name, e2.name);

    println!("\n=== 3. 创建任务并由管理员分派给艾莉 ===");
    let t = sys
        .create_task(&admin.id, &aln.id, "推理服务压测", "对 v2 推理服务做全链路压测", Priority::High)
        .await
        .expect("create task");
    println!("  任务: 《{}》 状态={}", t.title, t.status.label());
    let t = sys
        .assign_task(&admin.id, &t.id, vec![e1.id.clone()])
        .await
        .expect("assign");
    println!("  分派后状态={} 被分派者={:?}", t.status.label(), t.assignees);

    println!("\n=== 4. 艾莉推进任务状态并评论 ===");
    let t = sys
        .transition_task(&e1.id, &t.id, TaskStatus::InProgress)
        .await
        .expect("transition");
    println!("  状态推进到: {}", t.status.label());
    sys.comment_task(&e1.id, &t.id, "已开始压测，初步 QPS 达标")
        .await
        .expect("comment");
    println!("  艾莉评论: 已开始压测，初步 QPS 达标");

    println!("\n=== 5. 权限校验：本(安全, 未分派且仅为专家) 不能推进该任务 ===");
    let denied = sys
        .transition_task(&e2.id, &t.id, TaskStatus::InReview)
        .await;
    match denied {
        Err(e) => println!("  如预期被拒绝: {}", e),
        Ok(_) => println!("  [错误] 越权未被拦截!"),
    }

    println!("\n=== 6. 角色提升：将本提升为协调员后，可推进任意任务 ===");
    sys.perm
        .assign_role(xuanji_system::rbac::RoleBinding::xuanji(
            xuanji_system::rbac::Role::Coordinator,
            &e2.id,
            &aln.id,
        ))
        .await;
    let t = sys
        .transition_task(&e2.id, &t.id, TaskStatus::InReview)
        .await
        .expect("协调员应能推进任务");
    println!("  协调员推进状态到: {}", t.status.label());

    // 等待反应器将事件转译为通知
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;

    println!("\n=== 6. 查看艾莉收到的通知 ===");
    let notes = sys.comm.list_notifications(&e1.id).await;
    for n in &notes {
        println!("  - [{}] {}", if n.read { "已读" } else { "未读" }, n.title);
        println!("      {}", n.body);
    }

    println!("\n=== 7. 任务频道消息（系统事件流）===");
    let ch = sys.store.task_channel(&aln.id, &t.id).await;
    let msgs = sys.comm.list_messages(&ch.id).await;
    for m in &msgs {
        let who = if m.sender_id == "system" { "系统".into() } else { sys.member.get(&m.sender_id).await.map(|x| x.name).unwrap_or_default() };
        println!("  [{}] {}: {}", m.kind_as_str(), who, m.body);
    }

    println!("\n演示完成。所有核心模块（成员/任务/权限/通信）已协同运作。");
}

/// 启动 HTTP + WebSocket 服务
async fn run_server() {
    tracing_subscriber_wrap();
    let config = AppConfig::load();
    // 内存 / 持久化统一按 12-Factor 配置构建：内存模式同样尊重 XUANJI_BIND / RATE_LIMIT / CORS
    // （持久化由 config.persist 决定；即使持久化打开失败也回退为内存，但保留同一份运行配置）
    let sys = Arc::new(XuanjiSystem::with_config(config.clone()).expect("按配置构建璇玑系统失败"));

    // 持久化模式下，若数据库中已存在璇玑则不重复引导（幂等启动）
    if sys.store.xuanji_count().await == 0 {
        let (aln, _admin, token) = sys
            .bootstrap("默认璇玑", "管理员", "admin@xuanji.io")
            .await
            .expect("bootstrap");
        println!("璇玑ID: {}", aln.id);
        println!("管理员令牌: {}", token);
    } else {
        println!("检测到已有持久化数据，跳过引导（幂等启动）");
        println!("（已有璇玑的管理员令牌未打印；请使用首次引导时保存的令牌）");
    }
    let _reactor = sys.start_reactor();

    let app = server::app(sys.clone());
    let addr = sys.config.bind_addr.clone();
    let persistent = sys.store.is_persistent();
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind failed");
    println!("璇玑系统已启动: http://{}", addr);
    println!("  持久化模式: {}", if persistent { "开启 (SQLite)".to_string() } else { "关闭 (内存)".to_string() });
    println!("  指标端点: http://{}/api/metrics", addr);
    println!("WebSocket 实时通知: ws://{}/api/ws?token=<token>", addr);
    axum::serve(listener, app).await.expect("serve");
}

fn tracing_subscriber_wrap() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();
}
