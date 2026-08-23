//! T6 RED/GREEN 测试：DIP 反转 orchestrator → traits（A-01 消除）。
//!
//! TR-06-01 (rule, AC-06)：src/orchestrator.rs 顶部不允许 `use crate::services::*;`
//! TR-06-02 (rule)：`cargo test -p xuanji-system` exit 0（现有服务功能无回归）
//! TR-06-03 (rubric, AC-31)：至少 2 个 traits（MemberService / PermissionService）有 mock impl 切换证明。

#[cfg(test)]
mod t6_dip_tests {
    use std::fs;

    const ORCH_PATH: &str = "src/orchestrator.rs";

    #[test]
    fn tr_06_01_no_services_wildcard_import() {
        let code = fs::read_to_string(ORCH_PATH).expect("orchestrator.rs readable");
        // 允许 use crate::services::MemberService 或单个，但不允许 wildcard *
        let has_wildcard = code.lines().any(|ln| {
            let t = ln.trim();
            // "use crate::services::*;" 或 含 "use ...::services::*"
            t.starts_with("use ") && t.contains("services") && t.contains("::*")
        });
        assert!(
            !has_wildcard,
            "TR-06-01 FAIL: orchestrator.rs 仍包含 wildcard `use crate::services::*;`，违反 DIP。"
        );
    }

    // —— Mock 证明：2 个 DIP traits 至少可脱离默认 concrete impl 运行 ——

    use xuanji_system::domain_traits::{MemberServiceTrait, PermissionServiceTrait};
    use xuanji_system::rbac::{Role, RoleBinding};
    use xuanji_system::error::Result;
    use xuanji_system::model::*;
    use xuanji_system::rbac::{Permission, ResourceCtx};

    /// 空实现 mock MemberService：任何调用返回 NotFound（用于证明 trait 可脱离默认实现）。
    struct MockMember;

    #[async_trait::async_trait]
    impl MemberServiceTrait for MockMember {
        async fn invite(&self, _by: &str, _input: &InviteInput) -> Result<Member> {
            Err(xuanji_system::error::AppError::NotFound("MockMember".into()))
        }
        async fn list(&self, _xuanji_id: &str) -> Result<Vec<Member>> { Ok(vec![]) }
    }

    struct MockPerm;
    #[async_trait::async_trait]
    impl PermissionServiceTrait for MockPerm {
        async fn authorize(&self, _mid: &str, _p: Permission, _ctx: &ResourceCtx) -> Result<()> { Ok(()) }
        async fn assign_role(&self, _b: RoleBinding) { /* mock noop */ }
    }

    #[tokio::test]
    async fn tr_06_03_two_traits_implementable_without_concrete() {
        // 只要能实例化并调用到 trait 方法、不报 NotFound/Panic，证明可切换
        let mm = MockMember;
        let pm = MockPerm;
        // Member list 返回 ok 空
        assert!(mm.list("x").await.unwrap().is_empty());
        // Perm authorize 返回 ok
        let rc = ResourceCtx { xuanji_id: "x".into(), task: None };
        assert!(pm.authorize("u", Permission::AuditView, &rc).await.is_ok());
        // assign_role 可调用
        pm.assign_role(RoleBinding::global(Role::XuanjiAdmin, "u")).await;
    }
}
