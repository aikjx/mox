// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

pub mod ddl {
    pub const SQL: &str = include_str!("ddl.sql");
}
pub mod model;
pub mod repo;

pub use model::{
    AuditLog, IamDataPermission, IamDepartment, IamMenu, IamPermission, IamResource, IamRole,
    IamRoleInherit, IamRoleMenu, IamRolePermission, IamTenant, IamTenantSetting, IamUser,
    IamUserMenu, IamUserRole, ScopeRule,
};
pub use repo::{IamRepoError, IamRepository};

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use rusqlite::Connection;
    use std::sync::Arc;

    fn setup() -> IamRepository {
        let conn = Connection::open_in_memory().unwrap();
        let repo = IamRepository::new(Arc::new(Mutex::new(conn)));
        repo.init_schema().unwrap();
        repo.seed_builtins().unwrap();
        repo
    }

    #[test]
    fn test_iam_basic_workflow() {
        let repo = setup();

        let tenant = repo
            .create_tenant("t001", "Test Corp", Some("logical"), Some("pro"))
            .expect("create tenant");
        assert_eq!(tenant.tenant_code, "t001");
        assert_eq!(tenant.tenant_status, "active");

        let fetched = repo
            .get_tenant(&tenant.tenant_id)
            .expect("get tenant")
            .expect("tenant exists");
        assert_eq!(fetched.tenant_name, "Test Corp");

        let roles = repo.list_roles("system").expect("list roles");
        let admin_role = roles
            .iter()
            .find(|r| r.role_code == "sys_admin")
            .expect("sys_admin role");
        assert_eq!(admin_role.is_builtin, 1);

        let user = repo
            .create_user(
                "system",
                "u001",
                "alice",
                Some("Alice Zhang"),
                None,
                None,
                false,
            )
            .expect("create user");
        assert_eq!(user.username, "alice");
        assert_eq!(user.user_code, "u001");

        repo.assign_role_to_user("system", &user.user_id, &admin_role.role_id, None)
            .expect("assign role");

        let has_manage = repo.check_permission("system", &user.user_id, "user:manage");
        assert!(has_manage, "sys_admin should have user:manage");

        let has_view = repo.check_permission("system", &user.user_id, "tenant:view");
        assert!(has_view, "sys_admin should have tenant:view");

        let perms = repo
            .get_user_permissions("system", &user.user_id)
            .expect("get permissions");
        assert!(perms.len() > 10, "admin should have many permissions");

        let no_perm = repo.check_permission("system", &user.user_id, "nonexistent:perm");
        assert!(!no_perm, "should not have nonexistent perm");
    }
}
