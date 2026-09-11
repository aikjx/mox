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
    IamUserDept, IamUserMenu, IamUserRole, ScopeRule, SysApiKey, SysConfig, SysDictData,
    SysDictType, SysLoginLog, SysOperLog, SysPost,
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

    #[test]
    fn test_user_multi_dept_membership() {
        let repo = setup();
        let t = repo.create_tenant("tX", "CorpX", Some("logical"), Some("pro")).unwrap();
        let tid = t.tenant_id;
        let d1 = repo.create_dept(&tid, "D1", "研发部", None, None, "active", None).unwrap();
        let d2 = repo.create_dept(&tid, "D2", "产品部", None, None, "active", None).unwrap();
        let d3 = repo.create_dept(&tid, "D3", "测试部", None, None, "active", None).unwrap();

        // 单部门创建 → 关联表自动有 is_primary=1 的归属行
        let u = repo.create_user(&tid, "u9", "bob", None, None, Some(&d1.dept_id), false).unwrap();
        let mems = repo.list_user_depts(&u.user_id).unwrap();
        assert_eq!(mems.len(), 1);
        assert_eq!(mems[0].dept_id, d1.dept_id);
        assert_eq!(mems[0].is_primary, 1);

        // 多部门：一次设置 3 个，首个为主，iam_user.dept_id 缓存同步
        repo.set_user_depts(&tid, &u.user_id, &[&d3.dept_id, &d1.dept_id, &d2.dept_id]).unwrap();
        let mems = repo.list_user_depts(&u.user_id).unwrap();
        assert_eq!(mems.len(), 3);
        assert_eq!(mems[0].dept_id, d3.dept_id, "first dept is primary");
        assert_eq!(mems[0].is_primary, 1);
        assert!(mems[1..].iter().all(|m| m.is_primary == 0));
        let u2 = repo.get_user(&u.user_id).unwrap().unwrap();
        assert_eq!(u2.dept_id.as_deref(), Some(d3.dept_id.as_str()), "primary cache synced");

        // 部门反查：用户同时出现在 3 个部门的人员列表中
        for d in [&d1, &d2, &d3] {
            let users = repo.list_users_by_dept(&tid, &d.dept_id).unwrap();
            assert!(users.iter().any(|x| x.user_id == u.user_id), "member of {}", d.dept_code);
        }

        // 跨租户部门被拒绝
        let t2 = repo.create_tenant("tY", "CorpY", Some("logical"), Some("pro")).unwrap();
        let dY = repo.create_dept(&t2.tenant_id, "DY", "外部部门", None, None, "active", None).unwrap();
        assert!(repo.set_user_depts(&tid, &u.user_id, &[&dY.dept_id]).is_err(), "cross-tenant dept rejected");

        // 空列表清空归属 + 缓存置空
        repo.set_user_depts(&tid, &u.user_id, &[]).unwrap();
        assert_eq!(repo.list_user_depts(&u.user_id).unwrap().len(), 0);
        assert_eq!(repo.get_user(&u.user_id).unwrap().unwrap().dept_id, None);

        // 单值 update_user 主部门路径：切换主部门、保留其余归属（兼容旧调用方且不误清）
        let u3 = repo.create_user(&tid, "u10", "carol", None, None, Some(&d1.dept_id), false).unwrap();
        repo.update_user(&u3.user_id, None, None, None, None, Some(&d2.dept_id), None, None).unwrap();
        let mems = repo.list_user_depts(&u3.user_id).unwrap();
        assert_eq!(mems.len(), 2, "prior membership kept");
        assert_eq!(mems[0].dept_id, d2.dept_id);
        assert_eq!(mems[0].is_primary, 1);
        assert!(mems[1..].iter().all(|m| m.is_primary == 0));

        // 删除用户级联清理归属
        repo.delete_user(&u3.user_id).unwrap();
        assert_eq!(repo.list_user_depts(&u3.user_id).unwrap().len(), 0);
    }
}
