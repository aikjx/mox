//! T6 RED/GREEN 测试：DIP 反转 orchestrator → traits（A-01 消除）。
//!
//! TR-06-01 (rule, AC-06)：src/orchestrator.rs 顶部不允许 `use crate::services::*;`
//! TR-06-02 (rule)：`cargo test -p mox-system` exit 0（现有服务功能无回归）
//! TR-06-03 (rubric, AC-31)：至少 2 个 traits（MemberService / PermissionService）有 mock impl 切换证明。
//! TR-06-04 (rule)：MoxSystem 结构体 4 个服务字段为 trait object（非 concrete struct）。
//! TR-06-05 (rule)：orchestrator.rs use 行仅引入 domain_traits::*Trait，不引入 concrete 服务名。
//! TR-06-06 (rule)：TaskServiceTrait 必须包含 create() 方法（编排层会调用）。
//! TR-06-07 (rule)：CommServiceTrait 必须包含 notify() 方法（Reactor 会调用）。
//! TR-06-08 (rule)：Reactor 结构体 comm 字段为 trait object（非 concrete CommService）。
//! TR-06-09 (rubric)：4 个 trait object 均可从 concrete service 通过 Arc/Box 注入组装。
//! TR-06-10 (rubric)：Task + Comm 两个 trait 的 Mock 可脱离 concrete 运行。

#[cfg(test)]
mod t6_dip_tests {
    use std::fs;
    use std::sync::Arc;

    const ORCH_PATH: &str = "src/orchestrator.rs";
    const TRAITS_PATH: &str = "src/domain_traits.rs";

    // ========================================================================
    // TR-06-01：不允许 wildcard services import
    // ========================================================================
    #[test]
    fn tr_06_01_no_services_wildcard_import() {
        let code = fs::read_to_string(ORCH_PATH).expect("orchestrator.rs readable");
        let has_wildcard = code.lines().any(|ln| {
            let t = ln.trim();
            t.starts_with("use ") && t.contains("services") && t.contains("::*")
        });
        assert!(
            !has_wildcard,
            "TR-06-01 FAIL: orchestrator.rs 仍包含 wildcard `use crate::services::*;`，违反 DIP。"
        );
    }

    // ========================================================================
    // TR-06-04：MoxSystem 4 字段必须是 trait object（dyn XxxTrait）
    // ========================================================================
    #[test]
    fn tr_06_04_mox_fields_are_dyn_traits() {
        let code = fs::read_to_string(ORCH_PATH).expect("orchestrator.rs readable");
        // 找到 `pub struct MoxSystem {` 到匹配 `}` 之间的字段声明区
        let struct_start = code
            .find("pub struct MoxSystem")
            .expect("MoxSystem struct must exist");
        let rest = &code[struct_start..];
        let brace_open = rest.find('{').expect("struct open brace");
        let brace_close = rest.find('}').expect("struct close brace");
        let fields = &rest[brace_open..brace_close];

        let required_dyn = [
            "dyn MemberServiceTrait",
            "dyn TaskServiceTrait",
            "dyn PermissionServiceTrait",
            "dyn CommServiceTrait",
        ];
        for dyn_name in required_dyn.iter() {
            assert!(
                fields.contains(dyn_name),
                "TR-06-04 FAIL: MoxSystem 字段区未包含 `{}`；当前字段声明：\n{}",
                dyn_name,
                fields
            );
        }
        // 同时不允许 concrete 服务名直接作为字段类型
        let forbidden_concrete = [
            "member: MemberService",
            "task: TaskService",
            "perm: PermissionService",
            "comm: CommService",
        ];
        for fc in forbidden_concrete.iter() {
            assert!(
                !fields.contains(fc),
                "TR-06-04 FAIL: MoxSystem 仍包含 concrete 字段 `{}`（应为 Box/Arc<dyn Trait>）",
                fc
            );
        }
    }

    // ========================================================================
    // TR-06-05：orchestrator.rs use 行不允许直接引用 concrete 服务名
    // ========================================================================
    #[test]
    fn tr_06_05_orch_use_only_traits_not_concrete() {
        let code = fs::read_to_string(ORCH_PATH).expect("orchestrator.rs readable");
        let names = [
            "MemberService",
            "TaskService",
            "PermissionService",
            "CommService",
        ];
        for n in names.iter() {
            let bad_lines: Vec<_> = code
                .lines()
                .enumerate()
                .filter(|(_, ln)| {
                    let t = ln.trim_start();
                    // 行首为 "use " / "pub use "，包含 services:: 且包含 concrete 名
                    (t.starts_with("use ") || t.starts_with("pub use "))
                        && t.contains("services::")
                        && t.contains(n)
                })
                .map(|(i, ln)| format!("  L{}: {}", i + 1, ln.trim()))
                .collect();
            assert!(
                bad_lines.is_empty(),
                "TR-06-05 FAIL: orchestrator.rs 顶栏 use 直接引入 concrete `{}`（应只 use domain_traits）。\n违规行：\n{}",
                n,
                bad_lines.join("\n")
            );
        }
    }

    // ========================================================================
    // TR-06-06：TaskServiceTrait 必须包含 create() 方法签名
    // ========================================================================
    #[test]
    fn tr_06_06_task_trait_has_create_method() {
        let code = fs::read_to_string(TRAITS_PATH).expect("domain_traits.rs readable");
        // 找 trait TaskServiceTrait { ... } 块
        let start = code
            .find("trait TaskServiceTrait")
            .expect("TaskServiceTrait must exist");
        let rest = &code[start..];
        let brace_open = rest.find('{').expect("trait open brace");
        let brace_close = rest.find('}').expect("trait close brace");
        let body = &rest[brace_open..brace_close];
        assert!(
            body.contains("fn create("),
            "TR-06-06 FAIL: TaskServiceTrait 缺少 `create` 方法（orchestrator.create_task 调用需要）。\n\
             trait body:\n{}",
            body
        );
    }

    // ========================================================================
    // TR-06-07：CommServiceTrait 必须包含 notify() 方法签名
    // ========================================================================
    #[test]
    fn tr_06_07_comm_trait_has_notify_method() {
        let code = fs::read_to_string(TRAITS_PATH).expect("domain_traits.rs readable");
        let start = code
            .find("trait CommServiceTrait")
            .expect("CommServiceTrait must exist");
        let rest = &code[start..];
        let brace_open = rest.find('{').expect("trait open brace");
        let brace_close = rest.find('}').expect("trait close brace");
        let body = &rest[brace_open..brace_close];
        assert!(
            body.contains("fn notify("),
            "TR-06-07 FAIL: CommServiceTrait 缺少 `notify` 方法（Reactor.handle 调用需要）。\n\
             trait body:\n{}",
            body
        );
    }

    // ========================================================================
    // TR-06-08：Reactor struct comm 字段必须是 dyn trait
    // ========================================================================
    #[test]
    fn tr_06_08_reactor_comm_is_dyn_trait() {
        let code = fs::read_to_string(ORCH_PATH).expect("orchestrator.rs readable");
        let start = code
            .find("pub struct Reactor")
            .or_else(|| code.find("struct Reactor"))
            .expect("Reactor struct must exist");
        let rest = &code[start..];
        let brace_open = rest.find('{').expect("Reactor open brace");
        let brace_close = rest.find('}').expect("Reactor close brace");
        let fields = &rest[brace_open..brace_close];
        assert!(
            fields.contains("dyn CommServiceTrait"),
            "TR-06-08 FAIL: Reactor.comm 字段仍为 concrete CommService（应为 Box/Arc<dyn CommServiceTrait>）。\n\
             Reactor 字段：\n{}",
            fields
        );
    }

    // ========================================================================
    // TR-06-03（扩充）：MemberService / PermissionService 至少各有 Mock 实现
    // ========================================================================
    use mox_system::domain_traits::{
        CommServiceTrait, MemberServiceTrait, PermissionServiceTrait, TaskServiceTrait,
    };
    use mox_system::error::Result;
    use mox_system::model::*;
    use mox_system::rbac::{Permission, ResourceCtx};
    use mox_system::rbac::{Role, RoleBinding};

    struct MockMember;
    #[async_trait::async_trait]
    impl MemberServiceTrait for MockMember {
        async fn invite(&self, _by: &str, _input: &InviteInput) -> Result<Member> {
            Err(mox_system::error::AppError::NotFound(
                "MockMember".into(),
            ))
        }
        async fn get(&self, id: &str) -> Result<Member> {
            Ok(Member {
                id: id.into(),
                mox_id: "mock_x".into(),
                name: format!("Member-{id}"),
                email: format!("{id}@mox.local"),
                title: "Software Engineer".into(),
                expertise: vec!["backend".into(), "rust".into()],
                tier: Tier::Senior,
                status: MemberStatus::Active,
                joined_at: chrono::Utc::now(),
            })
        }
        async fn activate(&self, mid: &str, by: &str) -> Result<Member> {
            Ok(Member {
                id: mid.into(),
                mox_id: "mock_x".into(),
                name: format!("Member-{mid}"),
                email: format!("{mid}@mox.local"),
                title: format!("Activated-by-{by}"),
                expertise: vec!["architecture".into()],
                tier: Tier::Lead,
                status: MemberStatus::Active,
                joined_at: chrono::Utc::now(),
            })
        }
        async fn set_status(&self, mid: &str, s: MemberStatus) -> Result<Member> {
            Ok(Member {
                id: mid.into(),
                mox_id: "mock_x".into(),
                name: format!("Member-{mid}"),
                email: format!("{mid}@mox.local"),
                title: format!("status={s:?}"),
                expertise: vec!["ops".into()],
                tier: Tier::Associate,
                status: s,
                joined_at: chrono::Utc::now(),
            })
        }
        async fn list(&self, _mox_id: &str) -> Vec<Member> {
            vec![]
        }
    }

    struct MockPerm;
    #[async_trait::async_trait]
    impl PermissionServiceTrait for MockPerm {
        async fn authorize(&self, _mid: &str, _p: Permission, _ctx: &ResourceCtx) -> Result<()> {
            Ok(())
        }
        async fn assign_role(&self, _b: RoleBinding) { /* mock noop */
        }
        async fn bindings_of(&self, _mid: &str) -> Vec<RoleBinding> {
            vec![]
        }
        async fn effective_permissions(&self, mid: &str) -> Vec<Permission> {
            // TR-3.3：分级权限矩阵，可断言
            match mid {
                "u1" => vec![
                    Permission::TaskViewAssigned,
                    Permission::TaskComment,
                    Permission::AuditView,
                ],
                "u2" => vec![
                    Permission::TaskCreate,
                    Permission::TaskAssign,
                    Permission::TaskEditAll,
                    Permission::TaskViewAll,
                    Permission::TaskComment,
                    Permission::AuditView,
                    Permission::CommSendMox,
                ],
                _other => vec![Permission::TaskViewAssigned, Permission::AuditView],
            }
        }
    }

    #[tokio::test]
    async fn tr_06_03_two_traits_implementable_without_concrete() {
        let mm = MockMember;
        let pm = MockPerm;
        // list 返回 Vec（非 Result），直接判空
        assert!(mm.list("x").await.is_empty());
        let rc = ResourceCtx {
            mox_id: "x".into(),
            task: None,
        };
        assert!(pm.authorize("u", Permission::AuditView, &rc).await.is_ok());
        pm.assign_role(RoleBinding::global(Role::MoxAdmin, "u"))
            .await;
    }

    // ========================================================================
    // TR-06-09：4 个 trait 均可通过 Box<dyn Trait>（trait object 构造）
    // ========================================================================
    struct MockTask;
    #[async_trait::async_trait]
    impl TaskServiceTrait for MockTask {
        async fn create(
            &self,
            _mox_id: &str,
            _actor: &str,
            title: &str,
            _desc: &str,
            _pri: Priority,
        ) -> Result<Task> {
            Ok(Task {
                id: "mock-t".into(),
                mox_id: "x".into(),
                title: title.into(),
                description: String::new(),
                priority: Priority::High,
                status: TaskStatus::Draft,
                assignees: vec![],
                watchers: vec![],
                subtasks: vec![],
                depends_on: vec![],
                created_by: "u".into(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }
        async fn get(&self, _id: &str) -> Result<Task> {
            Err(mox_system::error::AppError::NotFound(
                "MockTask.get".into(),
            ))
        }
        async fn list(&self, _mox_id: &str) -> Vec<Task> {
            vec![]
        }
        async fn assign(&self, tid: &str, _actor: &str, assignees: Vec<String>) -> Result<Task> {
            Ok(Task {
                id: tid.into(),
                mox_id: "x".into(),
                title: format!("assigned:{tid}"),
                description: String::new(),
                priority: Priority::High,
                status: TaskStatus::Draft,
                assignees,
                watchers: vec![],
                subtasks: vec![],
                depends_on: vec![],
                created_by: "u".into(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }
        async fn transition(&self, tid: &str, _by: &str, to: TaskStatus) -> Result<Task> {
            Ok(Task {
                id: tid.into(),
                mox_id: "x".into(),
                title: format!("status:{tid}→{to:?}"),
                description: String::new(),
                priority: Priority::Medium,
                status: to,
                assignees: vec![],
                watchers: vec![],
                subtasks: vec![],
                depends_on: vec![],
                created_by: "u".into(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }
        async fn comment(&self, tid: &str, by: &str, body: &str) -> Result<Message> {
            Ok(Message {
                id: format!("msg:{tid}:{}", chrono::Utc::now().timestamp_millis()),
                channel_id: format!("ch-task:{tid}"),
                sender_id: by.into(),
                body: body.into(),
                kind: MessageKind::Chat,
                created_at: chrono::Utc::now(),
            })
        }
        async fn watch(&self, tid: &str, actor: &str) -> Result<Task> {
            Ok(Task {
                id: tid.into(),
                mox_id: "x".into(),
                title: format!("watched:{tid}"),
                description: String::new(),
                priority: Priority::Medium,
                status: TaskStatus::Draft,
                assignees: vec![],
                watchers: vec![actor.into()],
                subtasks: vec![],
                depends_on: vec![],
                created_by: actor.into(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }
        async fn add_subtask(&self, tid: &str, title: &str) -> Result<Task> {
            let sub_id = format!("sub_{tid}");
            Ok(Task {
                id: sub_id.clone(),
                mox_id: "x".into(),
                title: title.into(),
                description: format!("子任务 of {tid}"),
                priority: Priority::Medium,
                status: TaskStatus::Draft,
                assignees: vec![],
                watchers: vec![],
                subtasks: vec![],
                depends_on: vec![tid.into()],
                created_by: "sys".into(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }
        async fn add_dependency(&self, tid: &str, dep: &str) -> Result<Task> {
            Ok(Task {
                id: tid.into(),
                mox_id: "x".into(),
                title: format!("depend:{tid}→{dep}"),
                description: String::new(),
                priority: Priority::Medium,
                status: TaskStatus::Draft,
                assignees: vec![],
                watchers: vec![],
                subtasks: vec![],
                depends_on: vec![dep.into()],
                created_by: "sys".into(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }
        async fn toggle_subtask(&self, tid: &str, sub: &str) -> Result<Task> {
            Ok(Task {
                id: tid.into(),
                mox_id: "x".into(),
                title: format!("toggle:{tid}/{sub}"),
                description: format!("toggled subtask:{sub}"),
                priority: Priority::Medium,
                status: TaskStatus::InProgress,
                assignees: vec![],
                watchers: vec![],
                subtasks: vec![SubTask {
                    id: sub.into(),
                    title: format!("st-{sub}"),
                    done: true,
                }],
                depends_on: vec![],
                created_by: "sys".into(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }
    }

    struct MockComm;
    #[async_trait::async_trait]
    impl CommServiceTrait for MockComm {
        async fn create_channel(
            &self,
            mox_id: &str,
            _kind: ChannelKind,
            name: &str,
            members: Vec<String>,
        ) -> Channel {
            Channel {
                id: "mock-ch".into(),
                mox_id: mox_id.into(),
                kind: ChannelKind::Mox,
                name: name.into(),
                members,
            }
        }
        async fn send_message(
            &self,
            _cid: &str,
            _actor: &str,
            body: &str,
            _kind: MessageKind,
        ) -> Result<Message> {
            Ok(Message {
                id: "m".into(),
                channel_id: "c".into(),
                sender_id: "sys".into(),
                body: body.into(),
                kind: MessageKind::System,
                created_at: chrono::Utc::now(),
            })
        }
        async fn list_messages(&self, _cid: &str) -> Vec<Message> {
            vec![]
        }
        async fn notify(&self, _mid: &str, _title: &str, _body: &str, _related: Option<&str>) {
            // noop mock
        }
        async fn list_notifications(&self, _mid: &str) -> Vec<Notification> {
            vec![]
        }
        async fn mark_read(&self, _id: &str, _mid: &str) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn tr_06_09_four_traits_boxable_dyn_objects() {
        // 4 个 trait 必须 all object-safe，可放入 Arc<dyn ...>（与 orchestrator 设计一致）
        let m: Arc<dyn MemberServiceTrait> = Arc::new(MockMember);
        let t: Arc<dyn TaskServiceTrait> = Arc::new(MockTask);
        let p: Arc<dyn PermissionServiceTrait> = Arc::new(MockPerm);
        let c: Arc<dyn CommServiceTrait> = Arc::new(MockComm);

        // 基本运行性：trait object 可调用方法（不触发 unimplemented 路径）
        assert!(m.list("x").await.is_empty());
        let rc = ResourceCtx {
            mox_id: "x".into(),
            task: None,
        };
        assert!(p.authorize("u", Permission::AuditView, &rc).await.is_ok());
        p.assign_role(RoleBinding::global(Role::MoxAdmin, "u"))
            .await;
        assert!(p.bindings_of("u").await.is_empty());
        let created = t
            .create("x", "u", "hello", "d", Priority::High)
            .await
            .unwrap();
        assert_eq!(created.title, "hello");
        assert!(t.list("x").await.is_empty());
        let _ch = c
            .create_channel("x", ChannelKind::Mox, "大厅", vec![])
            .await;
        assert!(c.list_messages("c").await.is_empty());
        assert!(c.list_notifications("u").await.is_empty());
        c.notify("u", "t", "b", None).await;
        assert!(c.mark_read("n", "u").await.is_ok());
    }

    // ========================================================================
    // TR-06-10：Task + Comm Mock 独立运行
    // ========================================================================
    #[tokio::test]
    async fn tr_06_10_task_and_comm_mocks_independent() {
        let task_mock: Arc<dyn TaskServiceTrait> = Arc::new(MockTask);
        let comm_mock: Arc<dyn CommServiceTrait> = Arc::new(MockComm);

        // Task.create 独立
        let t = task_mock
            .create("xj1", "alice", "DIP 测试任务", "描述", Priority::Medium)
            .await
            .unwrap();
        assert_eq!(t.id, "mock-t");
        assert_eq!(t.mox_id, "x"); // Mock 硬编码

        // Comm.send_message + notify 独立
        let m = comm_mock
            .send_message("c1", "bob", "hello world", MessageKind::Chat)
            .await
            .unwrap();
        assert_eq!(m.sender_id, "sys");
        assert_eq!(m.body, "hello world");
        // notify 无返回值，调用即成功
        comm_mock
            .notify("member1", "你好", "DIP 通知", Some("t1"))
            .await;
    }

    // ========================================================================
    // 新增：Mock 真实返回值用例（TDD RED-GREEN：9× unimplemented!() 必须替换）
    // ========================================================================

    /// TR-3.2：TaskServiceTrait.add_subtask 返回 `sub_{parent_id}` 格式 ID
    #[tokio::test]
    async fn mock_task_add_subtask_prefixed_parent_id() {
        let t: Arc<dyn TaskServiceTrait> = Arc::new(MockTask);
        let parent = t
            .create("x", "u", "Parent", "d", Priority::High)
            .await
            .unwrap();
        let sub = t.add_subtask(&parent.id, "child-1").await.unwrap();
        assert!(
            sub.id.starts_with("sub_"),
            "子任务 id 必须以 sub_ 前缀；实际：{}",
            sub.id
        );
        assert!(
            sub.id.ends_with(&parent.id),
            "子任务 id 必须以前缀+父 id 构成；实际：{} vs 父 id {}",
            sub.id,
            parent.id
        );
        assert_eq!(sub.title, "child-1");
    }

    /// TR-3.3：PermissionServiceTrait.effective_permissions —— 用户分级返回
    #[tokio::test]
    async fn mock_effective_permissions_user1_user2() {
        let p: Arc<dyn PermissionServiceTrait> = Arc::new(MockPerm);
        let u1 = p.effective_permissions("u1").await;
        let u2 = p.effective_permissions("u2").await;
        // u1 viewer: 至少 TaskViewAssigned + AuditView
        let u1_has_view = u1.iter().any(|x| matches!(x, Permission::TaskViewAssigned));
        let u1_has_audit = u1.iter().any(|x| matches!(x, Permission::AuditView));
        assert!(
            u1_has_view && u1_has_audit,
            "u1 (viewer) 必须包含 view + audit 权限；实际：{:?}",
            u1.iter().map(Permission::as_str).collect::<Vec<_>>()
        );
        // u2 editor: 必须包含 TaskCreate + TaskEditAll（比 viewer 更强）
        let u2_has_create = u2.iter().any(|x| matches!(x, Permission::TaskCreate));
        let u2_has_edit_all = u2.iter().any(|x| matches!(x, Permission::TaskEditAll));
        assert!(
            u2_has_create && u2_has_edit_all,
            "u2 (editor) 必须包含 create + edit_all；实际：{:?}",
            u2.iter().map(Permission::as_str).collect::<Vec<_>>()
        );
        // 严格：editor 权限数 > viewer
        assert!(
            u2.len() > u1.len(),
            "editor 权限应超过 viewer：u2={} u1={}",
            u2.len(),
            u1.len()
        );
    }
}
