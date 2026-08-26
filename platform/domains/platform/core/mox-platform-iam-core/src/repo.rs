use crate::model::*;
use anyhow::Context;
use anyhow::Result;
use chrono::Utc;
use dashmap::DashMap;
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub static DDL_SQL: &str = include_str!("ddl.sql");

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum IamRepoError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Already exists: {0}")]
    AlreadyExists(String),
}

pub struct IamRepository {
    pub conn: Connection,
    perm_cache: DashMap<(String, String), Vec<String>>,
}

impl IamRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn,
            perm_cache: DashMap::new(),
        }
    }

    pub fn init_schema(&self) -> Result<()> {
        let stmts: Vec<&str> = DDL_SQL
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        for stmt in stmts {
            let res = self.conn.execute_batch(stmt);
            if let Err(e) = res {
                let msg = e.to_string().to_lowercase();
                if msg.contains("already exists") || msg.contains("duplicate column") {
                    continue;
                }
                return Err(e).with_context(|| format!("executing DDL: {}", stmt));
            }
        }
        Ok(())
    }

    pub fn seed_builtins(&self) -> Result<()> {
        let tenant_id = "system".to_string();
        let ts = now_iso();

        let sys_tenant = IamTenant {
            tenant_id: tenant_id.clone(),
            tenant_code: "system".to_string(),
            tenant_name: "System Tenant".to_string(),
            tenant_mode: "logical".to_string(),
            tenant_status: "active".to_string(),
            tenant_plan: "ultimate".to_string(),
            config_json: None,
            settings: None,
            created_at: ts.clone(),
            updated_at: ts.clone(),
            version: 1,
        };
        let _ = self.create_tenant_inner(&sys_tenant);

        let builtin_roles = [
            ("sys_admin", "超级管理员", "system", "all"),
            ("sys_developer", "系统开发者", "system", "all"),
            ("tenant_admin", "租户管理员", "tenant", "all"),
            ("tenant_user", "租户普通用户", "tenant", "self"),
        ];
        let mut role_ids: HashMap<String, String> = HashMap::new();
        for (code, name, rtype, dscope) in builtin_roles.iter() {
            let rid = new_id();
            role_ids.insert(code.to_string(), rid.clone());
            let role = IamRole {
                role_id: rid,
                tenant_id: tenant_id.clone(),
                role_code: code.to_string(),
                role_name: name.to_string(),
                role_type: rtype.to_string(),
                parent_id: None,
                inherit_path: None,
                is_builtin: 1,
                data_scope: dscope.to_string(),
                description: None,
                sort_order: Some(0),
                status: "active".to_string(),
                created_at: ts.clone(),
                updated_at: ts.clone(),
                version: 1,
            };
            let _ = self.create_role_inner(&role);
        }

        let resources = [
            "user", "dept", "role", "menu", "permission", "tenant", "resource",
            "audit", "meta", "workflow",
        ];
        let actions = [
            "view", "create", "edit", "delete", "export", "import", "manage",
        ];

        for res_code in resources.iter() {
            let res_id = new_id();
            let resource = IamResource {
                resource_id: res_id.clone(),
                tenant_id: tenant_id.clone(),
                resource_code: res_code.to_string(),
                resource_name: res_code.to_string(),
                resource_type: "api".to_string(),
                parent_id: None,
                resource_category: Some("core".to_string()),
                api_methods_sql: None,
                api_paths_sql: None,
                description: None,
                sort_order: Some(0),
                status: "active".to_string(),
                created_at: ts.clone(),
                updated_at: ts.clone(),
                version: 1,
            };
            let _ = self.create_resource_inner(&resource);

            for act in actions.iter() {
                let perm_id = new_id();
                let perm_code = format!("{}:{}", res_code, act);
                let perm = IamPermission {
                    perm_id: perm_id.clone(),
                    tenant_id: tenant_id.clone(),
                    perm_code: perm_code.clone(),
                    perm_name: format!("{} {}", res_code, act),
                    resource_id: res_id.clone(),
                    resource_type: "api".to_string(),
                    perm_action: act.to_string(),
                    perm_category: Some("core".to_string()),
                    description: None,
                    sort_order: Some(0),
                    status: "active".to_string(),
                    created_at: ts.clone(),
                    updated_at: ts.clone(),
                    version: 1,
                };
                let _ = self.create_permission_inner(&perm);

                let _ = self.assign_perm_to_role_inner(
                    &tenant_id,
                    role_ids.get("sys_admin").unwrap(),
                    &perm_id,
                    None,
                );
                let _ = self.assign_perm_to_role_inner(
                    &tenant_id,
                    role_ids.get("tenant_admin").unwrap(),
                    &perm_id,
                    None,
                );
            }
        }

        Ok(())
    }

    fn create_tenant_inner(&self, t: &IamTenant) -> Result<()> {
        self.conn.execute(
            "INSERT INTO iam_tenant (tenant_id,tenant_code,tenant_name,tenant_mode,tenant_status,tenant_plan,config_json,settings,created_at,updated_at,version) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                t.tenant_id, t.tenant_code, t.tenant_name, t.tenant_mode, t.tenant_status,
                t.tenant_plan, t.config_json, t.settings, t.created_at, t.updated_at, t.version
            ],
        )?;
        Ok(())
    }

    pub fn create_tenant(
        &self,
        tenant_code: &str,
        tenant_name: &str,
        tenant_mode: Option<&str>,
        tenant_plan: Option<&str>,
    ) -> Result<IamTenant> {
        let ts = now_iso();
        let t = IamTenant {
            tenant_id: new_id(),
            tenant_code: tenant_code.to_string(),
            tenant_name: tenant_name.to_string(),
            tenant_mode: tenant_mode.unwrap_or("logical").to_string(),
            tenant_status: "active".to_string(),
            tenant_plan: tenant_plan.unwrap_or("free").to_string(),
            config_json: None,
            settings: None,
            created_at: ts.clone(),
            updated_at: ts,
            version: 1,
        };
        self.create_tenant_inner(&t)?;
        Ok(t)
    }

    pub fn get_tenant(&self, tenant_id: &str) -> Result<Option<IamTenant>> {
        let mut stmt = self.conn.prepare(
            "SELECT tenant_id,tenant_code,tenant_name,tenant_mode,tenant_status,tenant_plan,config_json,settings,created_at,updated_at,version FROM iam_tenant WHERE tenant_id=?1"
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
            Ok(IamTenant {
                tenant_id: r.get(0)?,
                tenant_code: r.get(1)?,
                tenant_name: r.get(2)?,
                tenant_mode: r.get(3)?,
                tenant_status: r.get(4)?,
                tenant_plan: r.get(5)?,
                config_json: r.get(6)?,
                settings: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
                version: r.get(10)?,
            })
        })?;
        let mut items: Vec<IamTenant> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    fn create_role_inner(&self, r: &IamRole) -> Result<()> {
        self.conn.execute(
            "INSERT INTO iam_role (role_id,tenant_id,role_code,role_name,role_type,parent_id,inherit_path,is_builtin,data_scope,description,sort_order,status,created_at,updated_at,version) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                r.role_id, r.tenant_id, r.role_code, r.role_name, r.role_type,
                r.parent_id, r.inherit_path, r.is_builtin, r.data_scope, r.description,
                r.sort_order, r.status, r.created_at, r.updated_at, r.version
            ],
        )?;
        Ok(())
    }

    pub fn list_roles(&self, tenant_id: &str) -> Result<Vec<IamRole>> {
        let mut stmt = self.conn.prepare(
            "SELECT role_id,tenant_id,role_code,role_name,role_type,parent_id,inherit_path,is_builtin,data_scope,description,sort_order,status,created_at,updated_at,version FROM iam_role WHERE tenant_id=?1 ORDER BY sort_order ASC"
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
            Ok(IamRole {
                role_id: r.get(0)?,
                tenant_id: r.get(1)?,
                role_code: r.get(2)?,
                role_name: r.get(3)?,
                role_type: r.get(4)?,
                parent_id: r.get(5)?,
                inherit_path: r.get(6)?,
                is_builtin: r.get(7)?,
                data_scope: r.get(8)?,
                description: r.get(9)?,
                sort_order: r.get(10)?,
                status: r.get(11)?,
                created_at: r.get(12)?,
                updated_at: r.get(13)?,
                version: r.get(14)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn create_user(
        &self,
        tenant_id: &str,
        user_code: &str,
        username: &str,
        real_name: Option<&str>,
        password_hash: Option<&str>,
        dept_id: Option<&str>,
        is_superuser: bool,
    ) -> Result<IamUser> {
        let ts = now_iso();
        let u = IamUser {
            user_id: new_id(),
            tenant_id: tenant_id.to_string(),
            user_code: user_code.to_string(),
            username: username.to_string(),
            password_hash: password_hash.map(|s| s.to_string()),
            real_name: real_name.map(|s| s.to_string()),
            nickname: None,
            email: None,
            phone: None,
            avatar: None,
            dept_id: dept_id.map(|s| s.to_string()),
            position: None,
            user_status: "active".to_string(),
            is_superuser: if is_superuser { 1 } else { 0 },
            last_login_at: None,
            last_login_ip: None,
            created_at: ts.clone(),
            updated_at: ts,
            version: 1,
        };
        self.conn.execute(
            "INSERT INTO iam_user (user_id,tenant_id,user_code,username,password_hash,real_name,nickname,email,phone,avatar,dept_id,position,user_status,is_superuser,last_login_at,last_login_ip,created_at,updated_at,version) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
            params![
                u.user_id, u.tenant_id, u.user_code, u.username, u.password_hash,
                u.real_name, u.nickname, u.email, u.phone, u.avatar,
                u.dept_id, u.position, u.user_status, u.is_superuser, u.last_login_at,
                u.last_login_ip, u.created_at, u.updated_at, u.version
            ],
        )?;
        Ok(u)
    }

    pub fn assign_role_to_user(
        &self,
        tenant_id: &str,
        user_id: &str,
        role_id: &str,
        assigned_by: Option<&str>,
    ) -> Result<IamUserRole> {
        let ts = now_iso();
        let ur = IamUserRole {
            ur_id: new_id(),
            tenant_id: tenant_id.to_string(),
            user_id: user_id.to_string(),
            role_id: role_id.to_string(),
            assigned_by: assigned_by.map(|s| s.to_string()),
            assigned_at: Some(ts.clone()),
            created_at: ts,
        };
        self.conn.execute(
            "INSERT INTO iam_user_role (ur_id,tenant_id,user_id,role_id,assigned_by,assigned_at,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![ur.ur_id, ur.tenant_id, ur.user_id, ur.role_id, ur.assigned_by, ur.assigned_at, ur.created_at],
        )?;
        Ok(ur)
    }

    fn create_permission_inner(&self, p: &IamPermission) -> Result<()> {
        self.conn.execute(
            "INSERT INTO iam_permission (perm_id,tenant_id,perm_code,perm_name,resource_id,resource_type,perm_action,perm_category,description,sort_order,status,created_at,updated_at,version) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                p.perm_id, p.tenant_id, p.perm_code, p.perm_name, p.resource_id,
                p.resource_type, p.perm_action, p.perm_category, p.description,
                p.sort_order, p.status, p.created_at, p.updated_at, p.version
            ],
        )?;
        Ok(())
    }

    fn create_resource_inner(&self, r: &IamResource) -> Result<()> {
        self.conn.execute(
            "INSERT INTO iam_resource (resource_id,tenant_id,resource_code,resource_name,resource_type,parent_id,resource_category,api_methods_sql,api_paths_sql,description,sort_order,status,created_at,updated_at,version) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                r.resource_id, r.tenant_id, r.resource_code, r.resource_name, r.resource_type,
                r.parent_id, r.resource_category, r.api_methods_sql, r.api_paths_sql,
                r.description, r.sort_order, r.status, r.created_at, r.updated_at, r.version
            ],
        )?;
        Ok(())
    }

    fn assign_perm_to_role_inner(
        &self,
        tenant_id: &str,
        role_id: &str,
        perm_id: &str,
        created_by: Option<&str>,
    ) -> Result<IamRolePermission> {
        let rp = IamRolePermission {
            rp_id: new_id(),
            tenant_id: tenant_id.to_string(),
            role_id: role_id.to_string(),
            perm_id: perm_id.to_string(),
            created_at: now_iso(),
            created_by: created_by.map(|s| s.to_string()),
        };
        self.conn.execute(
            "INSERT INTO iam_role_permission (rp_id,tenant_id,role_id,perm_id,created_at,created_by) VALUES (?1,?2,?3,?4,?5,?6)",
            params![rp.rp_id, rp.tenant_id, rp.role_id, rp.perm_id, rp.created_at, rp.created_by],
        )?;
        Ok(rp)
    }

    pub fn assign_perm_to_role(
        &self,
        tenant_id: &str,
        role_id: &str,
        perm_id: &str,
        created_by: Option<&str>,
    ) -> Result<IamRolePermission> {
        self.assign_perm_to_role_inner(tenant_id, role_id, perm_id, created_by)
    }

    fn collect_parent_role_ids(&self, tenant_id: &str, role_ids: &[String]) -> Result<Vec<String>> {
        let mut result: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = role_ids.to_vec();
        for rid in role_ids {
            result.insert(rid.clone());
        }
        while let Some(child) = stack.pop() {
            let mut stmt = self.conn.prepare(
                "SELECT parent_role_id FROM iam_role_inherit WHERE tenant_id=?1 AND child_role_id=?2"
            )?;
            let rows = stmt.query_map(params![tenant_id, child], |r| r.get::<_, String>(0))?;
            for pr in rows {
                let pr = pr?;
                if result.insert(pr.clone()) {
                    stack.push(pr);
                }
            }
            let mut stmt2 = self.conn.prepare(
                "SELECT parent_id FROM iam_role WHERE tenant_id=?1 AND role_id=?2 AND parent_id IS NOT NULL"
            )?;
            let rows2 = stmt2.query_map(params![tenant_id, child], |r| r.get::<_, String>(0))?;
            for pr in rows2 {
                let pr = pr?;
                if result.insert(pr.clone()) {
                    stack.push(pr);
                }
            }
        }
        Ok(result.into_iter().collect())
    }

    pub fn get_user_permissions(&self, tenant_id: &str, user_id: &str) -> Result<Vec<String>> {
        let cache_key = (tenant_id.to_string(), user_id.to_string());
        if let Some(cached) = self.perm_cache.get(&cache_key) {
            return Ok(cached.value().clone());
        }
        let mut stmt = self.conn.prepare(
            "SELECT role_id FROM iam_user_role WHERE tenant_id=?1 AND user_id=?2"
        )?;
        let direct_role_ids: Vec<String> = stmt
            .query_map(params![tenant_id, user_id], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut stmt_user = self.conn.prepare(
            "SELECT is_superuser, dept_id FROM iam_user WHERE tenant_id=?1 AND user_id=?2"
        )?;
        let mut user_rows = stmt_user.query(params![tenant_id, user_id])?;
        let mut is_super = 0i64;
        if let Some(row) = user_rows.next()? {
            is_super = row.get(0)?;
        }
        drop(user_rows);

        if is_super == 1 {
            let mut all_stmt = self.conn.prepare(
                "SELECT perm_code FROM iam_permission WHERE tenant_id IN (?1, 'system') AND status='active'"
            )?;
            let perms: Vec<String> = all_stmt
                .query_map(params![tenant_id], |r| r.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            self.perm_cache.insert(cache_key, perms.clone());
            return Ok(perms);
        }

        let all_role_ids = self.collect_parent_role_ids(tenant_id, &direct_role_ids)?;

        let mut perms: HashSet<String> = HashSet::new();
        if !all_role_ids.is_empty() {
            let placeholders: Vec<String> = all_role_ids.iter().map(|_| "?".to_string()).collect();
            let sql = format!(
                "SELECT p.perm_code FROM iam_permission p \
                 JOIN iam_role_permission rp ON p.perm_id=rp.perm_id \
                 WHERE rp.tenant_id IN (?1, 'system') AND rp.role_id IN ({}) \
                 AND p.status='active'",
                placeholders.join(",")
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&tenant_id];
            for r in &all_role_ids {
                params_vec.push(r);
            }
            let rows = stmt.query_map(rusqlite::params_from_iter(params_vec.iter()), |r| {
                r.get::<_, String>(0)
            })?;
            for pc in rows {
                perms.insert(pc?);
            }
        }

        let perm_vec: Vec<String> = perms.into_iter().collect();
        self.perm_cache.insert(cache_key, perm_vec.clone());
        Ok(perm_vec)
    }

    pub fn check_permission(
        &self,
        tenant_id: &str,
        user_id: &str,
        perm_code: &str,
    ) -> bool {
        match self.get_user_permissions(tenant_id, user_id) {
            Ok(perms) => perms.iter().any(|p| p == perm_code),
            Err(_) => false,
        }
    }

    pub fn list_user_menus(&self, tenant_id: &str, user_id: &str) -> Result<Vec<IamMenu>> {
        let mut stmt = self.conn.prepare(
            "SELECT role_id FROM iam_user_role WHERE tenant_id=?1 AND user_id=?2"
        )?;
        let direct_role_ids: Vec<String> = stmt
            .query_map(params![tenant_id, user_id], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let all_role_ids = self.collect_parent_role_ids(tenant_id, &direct_role_ids)?;

        let mut menu_ids: HashSet<String> = HashSet::new();
        if !all_role_ids.is_empty() {
            let placeholders: Vec<String> = all_role_ids.iter().map(|_| "?".to_string()).collect();
            let sql = format!(
                "SELECT menu_id FROM iam_role_menu WHERE tenant_id IN (?1, 'system') AND role_id IN ({})",
                placeholders.join(",")
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&tenant_id];
            for r in &all_role_ids {
                params_vec.push(r);
            }
            let rows = stmt.query_map(rusqlite::params_from_iter(params_vec.iter()), |r| {
                r.get::<_, String>(0)
            })?;
            for mid in rows {
                menu_ids.insert(mid?);
            }
        }

        let mut stmt_um = self.conn.prepare(
            "SELECT menu_id FROM iam_user_menu WHERE tenant_id=?1 AND user_id=?2"
        )?;
        let rows_um = stmt_um.query_map(params![tenant_id, user_id], |r| r.get::<_, String>(0))?;
        for mid in rows_um {
            menu_ids.insert(mid?);
        }

        if menu_ids.is_empty() {
            return Ok(vec![]);
        }
        let ids_vec: Vec<String> = menu_ids.into_iter().collect();
        let placeholders: Vec<String> = ids_vec.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT menu_id,tenant_id,parent_id,menu_code,menu_name,menu_type,menu_category,route_path,route_name,component_path,icon,color,sort_order,is_visible,is_cached,is_external,link_target,permission_code,api_scope,menu_config,children_json,status,created_at,updated_at,version \
             FROM iam_menu WHERE menu_id IN ({}) AND status='active' ORDER BY sort_order ASC",
            placeholders.join(",")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let params_vec: Vec<&dyn rusqlite::ToSql> = ids_vec.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(rusqlite::params_from_iter(params_vec.iter()), |r| {
            Ok(IamMenu {
                menu_id: r.get(0)?,
                tenant_id: r.get(1)?,
                parent_id: r.get(2)?,
                menu_code: r.get(3)?,
                menu_name: r.get(4)?,
                menu_type: r.get(5)?,
                menu_category: r.get(6)?,
                route_path: r.get(7)?,
                route_name: r.get(8)?,
                component_path: r.get(9)?,
                icon: r.get(10)?,
                color: r.get(11)?,
                sort_order: r.get(12)?,
                is_visible: r.get(13)?,
                is_cached: r.get(14)?,
                is_external: r.get(15)?,
                link_target: r.get(16)?,
                permission_code: r.get(17)?,
                api_scope: r.get(18)?,
                menu_config: r.get(19)?,
                children_json: r.get(20)?,
                status: r.get(21)?,
                created_at: r.get(22)?,
                updated_at: r.get(23)?,
                version: r.get(24)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn evaluate_data_scope(
        &self,
        tenant_id: &str,
        user_id: &str,
        resource_code: &str,
    ) -> Result<ScopeRule> {
        let mut stmt = self.conn.prepare(
            "SELECT role_id, dept_id FROM iam_user WHERE tenant_id=?1 AND user_id=?2"
        )?;
        let mut user_rows = stmt.query(params![tenant_id, user_id])?;
        let mut user_dept: Option<String> = None;
        if let Some(row) = user_rows.next()? {
            user_dept = row.get(1).ok();
        }
        drop(user_rows);

        let mut stmt2 = self.conn.prepare(
            "SELECT role_id FROM iam_user_role WHERE tenant_id=?1 AND user_id=?2"
        )?;
        let direct_role_ids: Vec<String> = stmt2
            .query_map(params![tenant_id, user_id], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let all_role_ids = self.collect_parent_role_ids(tenant_id, &direct_role_ids)?;

        let mut scope_type = "self".to_string();
        let mut expression: Option<String> = None;
        let mut dp_codes: Vec<String> = vec![];

        if !all_role_ids.is_empty() {
            let placeholders: Vec<String> = all_role_ids.iter().map(|_| "?".to_string()).collect();
            let sql = format!(
                "SELECT dp_code, scope_type, custom_rule_expression_sql FROM iam_data_permission \
                 WHERE tenant_id=?1 AND resource_code=?2 AND subject_type='role' \
                 AND subject_id IN ({}) AND status='active' \
                 ORDER BY CASE scope_type \
                   WHEN 'all' THEN 0 \
                   WHEN 'dept_and_sub' THEN 1 \
                   WHEN 'dept' THEN 2 \
                   WHEN 'self' THEN 3 \
                   WHEN 'custom' THEN 4 END ASC LIMIT 1",
                placeholders.join(",")
            );
            let mut stmt3 = self.conn.prepare(&sql)?;
            let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&tenant_id, &resource_code];
            for r in &all_role_ids {
                params_vec.push(r);
            }
            let mut rows = stmt3.query(rusqlite::params_from_iter(params_vec.iter()))?;
            if let Some(row) = rows.next()? {
                let code: String = row.get(0)?;
                let st: String = row.get(1)?;
                let expr: Option<String> = row.get(2).ok();
                dp_codes.push(code);
                scope_type = st;
                if scope_type == "custom" {
                    expression = expr;
                }
            }
        }

        if scope_type == "self" {
            expression = Some(format!("owner_user_id = '{}'", user_id));
        } else if scope_type == "dept" {
            if let Some(did) = &user_dept {
                expression = Some(format!("owner_dept_id = '{}'", did));
            } else {
                expression = Some(format!("owner_user_id = '{}'", user_id));
            }
        } else if scope_type == "dept_and_sub" {
            if let Some(did) = &user_dept {
                expression = Some(format!("(owner_dept_id = '{}' OR owner_dept_id IN (SELECT dept_id FROM iam_department WHERE dept_path LIKE '%/{}%'))", did, did));
            } else {
                expression = Some(format!("owner_user_id = '{}'", user_id));
            }
        } else if scope_type == "all" {
            expression = Some("1=1".to_string());
        }

        Ok(ScopeRule {
            scope_type,
            expression,
            dp_codes,
        })
    }
}
