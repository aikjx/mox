// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::model::*;
use anyhow::Context;
use anyhow::Result;
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
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

#[derive(Clone)]
pub struct IamRepository {
    pub conn: Arc<Mutex<Connection>>,
    perm_cache: DashMap<(String, String), Vec<String>>,
}

impl IamRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            perm_cache: DashMap::new(),
        }
    }

    pub fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock();
        let stmts: Vec<&str> = DDL_SQL
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        for stmt in stmts {
            let res = conn.execute_batch(stmt);
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

    pub fn seed(&self) -> Result<()> {
        self.seed_builtins()
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
            "user",
            "dept",
            "role",
            "menu",
            "permission",
            "tenant",
            "resource",
            "audit",
            "meta",
            "workflow",
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

        let t001_id = "t001-tenant".to_string();
        let t001 = IamTenant {
            tenant_id: t001_id.clone(),
            tenant_code: "T001".to_string(),
            tenant_name: "企业演示租户".to_string(),
            tenant_mode: "logical".to_string(),
            tenant_status: "active".to_string(),
            tenant_plan: "enterprise".to_string(),
            config_json: None,
            settings: None,
            created_at: ts.clone(),
            updated_at: ts.clone(),
            version: 1,
        };
        let _ = self.create_tenant_inner(&t001);

        for (code, name, rtype, dscope) in builtin_roles.iter() {
            let rid = new_id();
            role_ids.insert(format!("T001:{}", code), rid.clone());
            let role = IamRole {
                role_id: rid,
                tenant_id: t001_id.clone(),
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

        let d001_id = "d001-dept".to_string();
        let d001 = IamDepartment {
            dept_id: d001_id.clone(),
            tenant_id: t001_id.clone(),
            parent_id: None,
            dept_code: "D001".to_string(),
            dept_name: "总裁办".to_string(),
            dept_type: "root".to_string(),
            dept_level: 1,
            dept_path: format!("/{}", d001_id),
            sort_order: Some(1),
            manager_user_id: None,
            status: "active".to_string(),
            created_at: ts.clone(),
            updated_at: ts.clone(),
            version: 1,
        };
        let _ = self.create_dept_inner(&d001);

        let admin_id = "admin-user".to_string();
        let admin = IamUser {
            user_id: admin_id.clone(),
            tenant_id: t001_id.clone(),
            user_code: "U001".to_string(),
            username: "admin".to_string(),
            password_hash: None,
            real_name: Some("系统管理员".to_string()),
            nickname: Some("系统管理员".to_string()),
            email: None,
            phone: None,
            avatar: None,
            dept_id: Some(d001_id.clone()),
            position: None,
            user_status: "active".to_string(),
            is_superuser: 1,
            last_login_at: None,
            last_login_ip: None,
            created_at: ts.clone(),
            updated_at: ts.clone(),
            version: 1,
        };
        {
            let conn = self.conn.lock();
            let _ = conn.execute(
                "INSERT INTO iam_user (user_id,tenant_id,user_code,username,password_hash,real_name,nickname,email,phone,avatar,dept_id,position,user_status,is_superuser,last_login_at,last_login_ip,created_at,updated_at,version) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
                params![
                    admin.user_id, admin.tenant_id, admin.user_code, admin.username,
                    admin.password_hash, admin.real_name, admin.nickname, admin.email,
                    admin.phone, admin.avatar, admin.dept_id, admin.position,
                    admin.user_status, admin.is_superuser, admin.last_login_at,
                    admin.last_login_ip, admin.created_at, admin.updated_at, admin.version
                ],
            );
        }

        let ta_role_id = role_ids.get("T001:tenant_admin").unwrap();
        let _ = self.assign_role_to_user(&t001_id, &admin_id, ta_role_id, None);

        Ok(())
    }

    fn create_tenant_inner(&self, t: &IamTenant) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO iam_tenant (tenant_id,tenant_code,tenant_name,tenant_mode,tenant_status,tenant_plan,config_json,settings,created_at,updated_at,version) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                t.tenant_id, t.tenant_code, t.tenant_name, t.tenant_mode, t.tenant_status,
                t.tenant_plan, t.config_json, t.settings, t.created_at, t.updated_at, t.version
            ],
        )?;
        Ok(())
    }

    pub fn list_departments(&self, tenant_id: &str) -> Result<Vec<IamDepartment>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT dept_id,tenant_id,parent_id,dept_code,dept_name,dept_type,dept_level,dept_path,sort_order,manager_user_id,status,created_at,updated_at,version \
             FROM iam_department WHERE tenant_id=?1 ORDER BY sort_order ASC",
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
            Ok(IamDepartment {
                dept_id: r.get(0)?,
                tenant_id: r.get(1)?,
                parent_id: r.get(2)?,
                dept_code: r.get(3)?,
                dept_name: r.get(4)?,
                dept_type: r.get(5)?,
                dept_level: r.get(6)?,
                dept_path: r.get(7)?,
                sort_order: r.get(8)?,
                manager_user_id: r.get(9)?,
                status: r.get(10)?,
                created_at: r.get(11)?,
                updated_at: r.get(12)?,
                version: r.get(13)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    fn create_dept_inner(&self, d: &IamDepartment) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO iam_department (dept_id,tenant_id,parent_id,dept_code,dept_name,dept_type,dept_level,dept_path,sort_order,manager_user_id,status,created_at,updated_at,version) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                d.dept_id, d.tenant_id, d.parent_id, d.dept_code, d.dept_name,
                d.dept_type, d.dept_level, d.dept_path, d.sort_order, d.manager_user_id,
                d.status, d.created_at, d.updated_at, d.version
            ],
        )?;
        Ok(())
    }

    pub fn find_tenant_by_code(&self, tenant_code: &str) -> Option<IamTenant> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT tenant_id,tenant_code,tenant_name,tenant_mode,tenant_status,tenant_plan,config_json,settings,created_at,updated_at,version FROM iam_tenant WHERE tenant_code=?1",
            )
            .ok()?;
        let mut rows = stmt.query(params![tenant_code]).ok()?;
        let row = rows.next().ok()??;
        Some(IamTenant {
            tenant_id: row.get(0).ok()?,
            tenant_code: row.get(1).ok()?,
            tenant_name: row.get(2).ok()?,
            tenant_mode: row.get(3).ok()?,
            tenant_status: row.get(4).ok()?,
            tenant_plan: row.get(5).ok()?,
            config_json: row.get(6).ok()?,
            settings: row.get(7).ok()?,
            created_at: row.get(8).ok()?,
            updated_at: row.get(9).ok()?,
            version: row.get(10).ok()?,
        })
    }

    pub fn find_user_by_tenant_username(&self, tenant_id: &str, username: &str) -> Option<IamUser> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT user_id,tenant_id,user_code,username,password_hash,real_name,nickname,email,phone,avatar,dept_id,position,user_status,is_superuser,last_login_at,last_login_ip,created_at,updated_at,version FROM iam_user WHERE tenant_id=?1 AND username=?2",
            )
            .ok()?;
        let mut rows = stmt.query(params![tenant_id, username]).ok()?;
        let row = rows.next().ok()??;
        Some(IamUser {
            user_id: row.get(0).ok()?,
            tenant_id: row.get(1).ok()?,
            user_code: row.get(2).ok()?,
            username: row.get(3).ok()?,
            password_hash: row.get(4).ok()?,
            real_name: row.get(5).ok()?,
            nickname: row.get(6).ok()?,
            email: row.get(7).ok()?,
            phone: row.get(8).ok()?,
            avatar: row.get(9).ok()?,
            dept_id: row.get(10).ok()?,
            position: row.get(11).ok()?,
            user_status: row.get(12).ok()?,
            is_superuser: row.get(13).ok()?,
            last_login_at: row.get(14).ok()?,
            last_login_ip: row.get(15).ok()?,
            created_at: row.get(16).ok()?,
            updated_at: row.get(17).ok()?,
            version: row.get(18).ok()?,
        })
    }

    pub fn get_user_roles(&self, tenant_id: &str, user_id: &str) -> Result<Vec<IamRole>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT r.role_id,r.tenant_id,r.role_code,r.role_name,r.role_type,r.parent_id,r.inherit_path,r.is_builtin,r.data_scope,r.description,r.sort_order,r.status,r.created_at,r.updated_at,r.version \
             FROM iam_role r JOIN iam_user_role ur ON r.role_id=ur.role_id \
             WHERE ur.tenant_id=?1 AND ur.user_id=?2",
        )?;
        let rows = stmt.query_map(params![tenant_id, user_id], |r| {
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

    pub fn user_roles(&self, user_id: &str) -> Vec<UserRole> {
        let roles_with_tenant: Vec<(String, IamRole)> = {
            let conn = self.conn.lock();
            let mut stmt = match conn
                .prepare("SELECT tenant_id, role_id FROM iam_user_role WHERE user_id=?1")
            {
                Ok(s) => s,
                Err(_) => return vec![],
            };
            let bindings: Vec<(String, String)> = match stmt.query_map(params![user_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            }) {
                Ok(rows) => rows
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .unwrap_or_default(),
                Err(_) => return vec![],
            };
            let mut result = Vec::new();
            for (tenant_id, role_id) in bindings {
                let mut rstmt = match conn.prepare(
                    "SELECT role_id,tenant_id,role_code,role_name,role_type,parent_id,inherit_path,is_builtin,data_scope,description,sort_order,status,created_at,updated_at,version FROM iam_role WHERE tenant_id=?1 AND role_id=?2",
                ) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let role_opt: Option<IamRole> = rstmt
                    .query_map(params![tenant_id, role_id], |r| {
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
                    })
                    .ok()
                    .and_then(|rows| rows.collect::<std::result::Result<Vec<_>, _>>().ok())
                    .and_then(|v| v.into_iter().next());
                if let Some(role) = role_opt {
                    result.push((tenant_id, role));
                }
            }
            result
        };

        let mut result = Vec::new();
        for (tenant_id, role) in roles_with_tenant {
            let perms = match self.get_user_permissions(&tenant_id, user_id) {
                Ok(p) => p,
                Err(_) => vec![],
            };
            result.push(UserRole {
                id: role.role_id,
                code: role.role_code,
                name: role.role_name,
                permissions: perms,
            });
        }
        result
    }

    pub fn get_user_permissions(&self, tenant_id: &str, user_id: &str) -> Result<Vec<String>> {
        let cache_key = (tenant_id.to_string(), user_id.to_string());
        if let Some(cached) = self.perm_cache.get(&cache_key) {
            return Ok(cached.value().clone());
        }
        let conn = self.conn.lock();
        let mut stmt =
            conn.prepare("SELECT role_id FROM iam_user_role WHERE tenant_id=?1 AND user_id=?2")?;
        let direct_role_ids: Vec<String> = stmt
            .query_map(params![tenant_id, user_id], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut stmt_user = conn.prepare(
            "SELECT is_superuser, dept_id FROM iam_user WHERE tenant_id=?1 AND user_id=?2",
        )?;
        let mut user_rows = stmt_user.query(params![tenant_id, user_id])?;
        let mut is_super = 0i64;
        if let Some(row) = user_rows.next()? {
            is_super = row.get(0)?;
        }
        drop(user_rows);

        if is_super == 1 {
            let mut all_stmt = conn.prepare(
                "SELECT perm_code FROM iam_permission WHERE tenant_id IN (?1, 'system') AND status='active'",
            )?;
            let perms: Vec<String> = all_stmt
                .query_map(params![tenant_id], |r| r.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            drop(all_stmt);
            drop(stmt_user);
            drop(stmt);
            drop(conn);
            self.perm_cache.insert(cache_key, perms.clone());
            return Ok(perms);
        }

        let all_role_ids =
            self.collect_parent_role_ids_inner(tenant_id, &direct_role_ids, &conn)?;

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
            let mut stmt_perm = conn.prepare(&sql)?;
            let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&tenant_id];
            for r in &all_role_ids {
                params_vec.push(r);
            }
            let rows = stmt_perm.query_map(rusqlite::params_from_iter(params_vec.iter()), |r| {
                r.get::<_, String>(0)
            })?;
            for pc in rows {
                perms.insert(pc?);
            }
            drop(stmt_perm);
        }

        let perm_vec: Vec<String> = perms.into_iter().collect();
        drop(stmt_user);
        drop(stmt);
        drop(conn);
        self.perm_cache.insert(cache_key, perm_vec.clone());
        Ok(perm_vec)
    }

    fn collect_parent_role_ids_inner(
        &self,
        tenant_id: &str,
        role_ids: &[String],
        conn: &Connection,
    ) -> Result<Vec<String>> {
        let mut result: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = role_ids.to_vec();
        for rid in role_ids {
            result.insert(rid.clone());
        }
        while let Some(child) = stack.pop() {
            let mut stmt = conn.prepare(
                "SELECT parent_role_id FROM iam_role_inherit WHERE tenant_id=?1 AND child_role_id=?2",
            )?;
            let rows = stmt.query_map(params![tenant_id, child], |r| r.get::<_, String>(0))?;
            for pr in rows {
                let pr = pr?;
                if result.insert(pr.clone()) {
                    stack.push(pr);
                }
            }
            let mut stmt2 = conn.prepare(
                "SELECT parent_id FROM iam_role WHERE tenant_id=?1 AND role_id=?2 AND parent_id IS NOT NULL",
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
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
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
        let conn = self.conn.lock();
        conn.execute(
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
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
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
        let conn = self.conn.lock();
        conn.execute(
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
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO iam_user_role (ur_id,tenant_id,user_id,role_id,assigned_by,assigned_at,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![ur.ur_id, ur.tenant_id, ur.user_id, ur.role_id, ur.assigned_by, ur.assigned_at, ur.created_at],
        )?;
        Ok(ur)
    }

    fn create_permission_inner(&self, p: &IamPermission) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
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
        let conn = self.conn.lock();
        conn.execute(
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
        let conn = self.conn.lock();
        conn.execute(
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

    pub fn check_permission(&self, tenant_id: &str, user_id: &str, perm_code: &str) -> bool {
        match self.get_user_permissions(tenant_id, user_id) {
            Ok(perms) => perms.iter().any(|p| p == perm_code),
            Err(_) => false,
        }
    }

    pub fn list_user_menus(&self, tenant_id: &str, user_id: &str) -> Result<Vec<IamMenu>> {
        let conn = self.conn.lock();
        let mut stmt =
            conn.prepare("SELECT role_id FROM iam_user_role WHERE tenant_id=?1 AND user_id=?2")?;
        let direct_role_ids: Vec<String> = stmt
            .query_map(params![tenant_id, user_id], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let all_role_ids =
            self.collect_parent_role_ids_inner(tenant_id, &direct_role_ids, &conn)?;

        let mut menu_ids: HashSet<String> = HashSet::new();
        if !all_role_ids.is_empty() {
            let placeholders: Vec<String> = all_role_ids.iter().map(|_| "?".to_string()).collect();
            let sql = format!(
                "SELECT menu_id FROM iam_role_menu WHERE tenant_id IN (?1, 'system') AND role_id IN ({})",
                placeholders.join(",")
            );
            let mut stmt = conn.prepare(&sql)?;
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

        let mut stmt_um =
            conn.prepare("SELECT menu_id FROM iam_user_menu WHERE tenant_id=?1 AND user_id=?2")?;
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
        let mut stmt = conn.prepare(&sql)?;
        let params_vec: Vec<&dyn rusqlite::ToSql> =
            ids_vec.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
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
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT role_id, dept_id FROM iam_user WHERE tenant_id=?1 AND user_id=?2")?;
        let mut user_rows = stmt.query(params![tenant_id, user_id])?;
        let mut user_dept: Option<String> = None;
        if let Some(row) = user_rows.next()? {
            user_dept = row.get(1).ok();
        }
        drop(user_rows);

        let mut stmt2 =
            conn.prepare("SELECT role_id FROM iam_user_role WHERE tenant_id=?1 AND user_id=?2")?;
        let direct_role_ids: Vec<String> = stmt2
            .query_map(params![tenant_id, user_id], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let all_role_ids =
            self.collect_parent_role_ids_inner(tenant_id, &direct_role_ids, &conn)?;

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
            let mut stmt3 = conn.prepare(&sql)?;
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

    // ============================================================
    // 部门 Dept（扩展写）
    // ============================================================

    pub fn create_dept(
        &self,
        tenant_id: &str,
        dept_code: &str,
        dept_name: &str,
        parent_id: Option<&str>,
        sort_order: Option<i64>,
        status: &str,
        manager_user_id: Option<&str>,
    ) -> Result<IamDepartment> {
        let ts = now_iso();
        let dept_id = new_id();
        let (dept_level, dept_path) = if let Some(pid) = parent_id {
            let conn = self.conn.lock();
            let mut stmt = conn
                .prepare("SELECT dept_level, dept_path FROM iam_department WHERE dept_id=?1")?;
            let mut rows = stmt.query(params![pid])?;
            if let Some(row) = rows.next()? {
                let pl: i64 = row.get(0)?;
                let pp: String = row.get(1)?;
                (pl + 1, format!("{}/{}", pp, dept_id))
            } else {
                (1, format!("/{}", dept_id))
            }
        } else {
            (1, format!("/{}", dept_id))
        };
        let d = IamDepartment {
            dept_id,
            tenant_id: tenant_id.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            dept_code: dept_code.to_string(),
            dept_name: dept_name.to_string(),
            dept_type: "department".to_string(),
            dept_level,
            dept_path,
            sort_order,
            manager_user_id: manager_user_id.map(|s| s.to_string()),
            status: status.to_string(),
            created_at: ts.clone(),
            updated_at: ts,
            version: 1,
        };
        self.create_dept_inner(&d)?;
        Ok(d)
    }

    pub fn get_dept(&self, dept_id: &str) -> Result<Option<IamDepartment>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT dept_id,tenant_id,parent_id,dept_code,dept_name,dept_type,dept_level,dept_path,sort_order,manager_user_id,status,created_at,updated_at,version FROM iam_department WHERE dept_id=?1",
        )?;
        let rows = stmt.query_map(params![dept_id], |r| {
            Ok(IamDepartment {
                dept_id: r.get(0)?,
                tenant_id: r.get(1)?,
                parent_id: r.get(2)?,
                dept_code: r.get(3)?,
                dept_name: r.get(4)?,
                dept_type: r.get(5)?,
                dept_level: r.get(6)?,
                dept_path: r.get(7)?,
                sort_order: r.get(8)?,
                manager_user_id: r.get(9)?,
                status: r.get(10)?,
                created_at: r.get(11)?,
                updated_at: r.get(12)?,
                version: r.get(13)?,
            })
        })?;
        let mut items: Vec<IamDepartment> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    pub fn update_dept(
        &self,
        dept_id: &str,
        dept_name: Option<&str>,
        parent_id: Option<&str>,
        sort_order: Option<i64>,
        status: Option<&str>,
        manager_user_id: Option<&str>,
    ) -> Result<()> {
        let now = now_iso();
        let mut sets: Vec<&str> = vec![];
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![];
        if let Some(v) = &dept_name { sets.push("dept_name=?"); params.push(v); }
        if let Some(v) = &parent_id { sets.push("parent_id=?"); params.push(v); }
        if let Some(v) = &sort_order { sets.push("sort_order=?"); params.push(v); }
        if let Some(v) = &status { sets.push("status=?"); params.push(v); }
        if let Some(v) = &manager_user_id { sets.push("manager_user_id=?"); params.push(v); }
        if sets.is_empty() {
            return Ok(());
        }
        sets.push("updated_at=?");
        params.push(&now);
        params.push(&dept_id);
        let conn = self.conn.lock();
        let sql = format!("UPDATE iam_department SET {} WHERE dept_id=?", sets.join(","));
        conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
        Ok(())
    }

    pub fn delete_dept(&self, dept_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM iam_department WHERE dept_id=?1", params![dept_id])?;
        Ok(())
    }

    pub fn list_users_by_dept(&self, tenant_id: &str, dept_id: &str) -> Result<Vec<IamUser>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT user_id,tenant_id,user_code,username,password_hash,real_name,nickname,email,phone,avatar,dept_id,position,user_status,is_superuser,last_login_at,last_login_ip,created_at,updated_at,version FROM iam_user WHERE tenant_id=?1 AND dept_id=?2 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![tenant_id, dept_id], |r| {
            Ok(IamUser {
                user_id: r.get(0)?,
                tenant_id: r.get(1)?,
                user_code: r.get(2)?,
                username: r.get(3)?,
                password_hash: r.get(4)?,
                real_name: r.get(5)?,
                nickname: r.get(6)?,
                email: r.get(7)?,
                phone: r.get(8)?,
                avatar: r.get(9)?,
                dept_id: r.get(10)?,
                position: r.get(11)?,
                user_status: r.get(12)?,
                is_superuser: r.get(13)?,
                last_login_at: r.get(14)?,
                last_login_ip: r.get(15)?,
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
                version: r.get(18)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    // ============================================================
    // 岗位 Post（全新）
    // ============================================================

    pub fn create_post(
        &self,
        tenant_id: &str,
        post_code: &str,
        post_name: &str,
        dept_id: Option<&str>,
        sort_order: Option<i64>,
        status: &str,
        remark: Option<&str>,
    ) -> Result<SysPost> {
        let ts = now_iso();
        let p = SysPost {
            post_id: new_id(),
            tenant_id: tenant_id.to_string(),
            post_code: post_code.to_string(),
            post_name: post_name.to_string(),
            dept_id: dept_id.map(|s| s.to_string()),
            sort_order,
            status: status.to_string(),
            remark: remark.map(|s| s.to_string()),
            created_at: ts.clone(),
            updated_at: ts,
        };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sys_post (post_id,tenant_id,post_code,post_name,dept_id,sort_order,status,remark,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![p.post_id, p.tenant_id, p.post_code, p.post_name, p.dept_id, p.sort_order, p.status, p.remark, p.created_at, p.updated_at],
        )?;
        Ok(p)
    }

    pub fn get_post(&self, post_id: &str) -> Result<Option<SysPost>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT post_id,tenant_id,post_code,post_name,dept_id,sort_order,status,remark,created_at,updated_at FROM sys_post WHERE post_id=?1",
        )?;
        let rows = stmt.query_map(params![post_id], |r| {
            Ok(SysPost {
                post_id: r.get(0)?,
                tenant_id: r.get(1)?,
                post_code: r.get(2)?,
                post_name: r.get(3)?,
                dept_id: r.get(4)?,
                sort_order: r.get(5)?,
                status: r.get(6)?,
                remark: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?;
        let mut items: Vec<SysPost> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    pub fn list_posts(&self, tenant_id: &str) -> Result<Vec<SysPost>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT post_id,tenant_id,post_code,post_name,dept_id,sort_order,status,remark,created_at,updated_at FROM sys_post WHERE tenant_id=?1 ORDER BY sort_order ASC",
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
            Ok(SysPost {
                post_id: r.get(0)?,
                tenant_id: r.get(1)?,
                post_code: r.get(2)?,
                post_name: r.get(3)?,
                dept_id: r.get(4)?,
                sort_order: r.get(5)?,
                status: r.get(6)?,
                remark: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn list_posts_by_dept(&self, tenant_id: &str, dept_id: &str) -> Result<Vec<SysPost>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT post_id,tenant_id,post_code,post_name,dept_id,sort_order,status,remark,created_at,updated_at FROM sys_post WHERE tenant_id=?1 AND dept_id=?2 ORDER BY sort_order ASC",
        )?;
        let rows = stmt.query_map(params![tenant_id, dept_id], |r| {
            Ok(SysPost {
                post_id: r.get(0)?,
                tenant_id: r.get(1)?,
                post_code: r.get(2)?,
                post_name: r.get(3)?,
                dept_id: r.get(4)?,
                sort_order: r.get(5)?,
                status: r.get(6)?,
                remark: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn update_post(
        &self,
        post_id: &str,
        post_code: Option<&str>,
        post_name: Option<&str>,
        dept_id: Option<&str>,
        sort_order: Option<i64>,
        status: Option<&str>,
        remark: Option<&str>,
    ) -> Result<()> {
        let now = now_iso();
        let mut sets: Vec<&str> = vec![];
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![];
        if let Some(v) = &post_code { sets.push("post_code=?"); params.push(v); }
        if let Some(v) = &post_name { sets.push("post_name=?"); params.push(v); }
        if let Some(v) = &dept_id { sets.push("dept_id=?"); params.push(v); }
        if let Some(v) = &sort_order { sets.push("sort_order=?"); params.push(v); }
        if let Some(v) = &status { sets.push("status=?"); params.push(v); }
        if let Some(v) = &remark { sets.push("remark=?"); params.push(v); }
        if sets.is_empty() {
            return Ok(());
        }
        sets.push("updated_at=?");
        params.push(&now);
        params.push(&post_id);
        let conn = self.conn.lock();
        let sql = format!("UPDATE sys_post SET {} WHERE post_id=?", sets.join(","));
        conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
        Ok(())
    }

    pub fn delete_post(&self, post_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM sys_post WHERE post_id=?1", params![post_id])?;
        Ok(())
    }

    // ============================================================
    // 用户 User（扩展写）
    // ============================================================

    pub fn list_users(&self, tenant_id: &str) -> Result<Vec<IamUser>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT user_id,tenant_id,user_code,username,password_hash,real_name,nickname,email,phone,avatar,dept_id,position,user_status,is_superuser,last_login_at,last_login_ip,created_at,updated_at,version FROM iam_user WHERE tenant_id=?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
            Ok(IamUser {
                user_id: r.get(0)?,
                tenant_id: r.get(1)?,
                user_code: r.get(2)?,
                username: r.get(3)?,
                password_hash: r.get(4)?,
                real_name: r.get(5)?,
                nickname: r.get(6)?,
                email: r.get(7)?,
                phone: r.get(8)?,
                avatar: r.get(9)?,
                dept_id: r.get(10)?,
                position: r.get(11)?,
                user_status: r.get(12)?,
                is_superuser: r.get(13)?,
                last_login_at: r.get(14)?,
                last_login_ip: r.get(15)?,
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
                version: r.get(18)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn get_user(&self, user_id: &str) -> Result<Option<IamUser>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT user_id,tenant_id,user_code,username,password_hash,real_name,nickname,email,phone,avatar,dept_id,position,user_status,is_superuser,last_login_at,last_login_ip,created_at,updated_at,version FROM iam_user WHERE user_id=?1",
        )?;
        let rows = stmt.query_map(params![user_id], |r| {
            Ok(IamUser {
                user_id: r.get(0)?,
                tenant_id: r.get(1)?,
                user_code: r.get(2)?,
                username: r.get(3)?,
                password_hash: r.get(4)?,
                real_name: r.get(5)?,
                nickname: r.get(6)?,
                email: r.get(7)?,
                phone: r.get(8)?,
                avatar: r.get(9)?,
                dept_id: r.get(10)?,
                position: r.get(11)?,
                user_status: r.get(12)?,
                is_superuser: r.get(13)?,
                last_login_at: r.get(14)?,
                last_login_ip: r.get(15)?,
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
                version: r.get(18)?,
            })
        })?;
        let mut items: Vec<IamUser> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    pub fn update_user(
        &self,
        user_id: &str,
        username: Option<&str>,
        real_name: Option<&str>,
        email: Option<&str>,
        phone: Option<&str>,
        dept_id: Option<&str>,
        position: Option<&str>,
        user_status: Option<&str>,
    ) -> Result<()> {
        let now = now_iso();
        let mut sets: Vec<&str> = vec![];
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![];
        if let Some(v) = &username { sets.push("username=?"); params.push(v); }
        if let Some(v) = &real_name { sets.push("real_name=?"); params.push(v); }
        if let Some(v) = &email { sets.push("email=?"); params.push(v); }
        if let Some(v) = &phone { sets.push("phone=?"); params.push(v); }
        if let Some(v) = &dept_id { sets.push("dept_id=?"); params.push(v); }
        if let Some(v) = &position { sets.push("position=?"); params.push(v); }
        if let Some(v) = &user_status { sets.push("user_status=?"); params.push(v); }
        if sets.is_empty() {
            return Ok(());
        }
        sets.push("updated_at=?");
        params.push(&now);
        params.push(&user_id);
        let conn = self.conn.lock();
        let sql = format!("UPDATE iam_user SET {} WHERE user_id=?", sets.join(","));
        conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
        Ok(())
    }

    pub fn delete_user(&self, user_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM iam_user_role WHERE user_id=?1", params![user_id])?;
        conn.execute("DELETE FROM iam_user WHERE user_id=?1", params![user_id])?;
        Ok(())
    }

    pub fn reset_password(&self, user_id: &str, password_hash: &str) -> Result<()> {
        let now = now_iso();
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE iam_user SET password_hash=?1, updated_at=?2 WHERE user_id=?3",
            params![password_hash, now, user_id],
        )?;
        Ok(())
    }

    pub fn change_user_status(&self, user_id: &str, status: &str) -> Result<()> {
        let now = now_iso();
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE iam_user SET user_status=?1, updated_at=?2 WHERE user_id=?3",
            params![status, now, user_id],
        )?;
        Ok(())
    }

    pub fn set_user_roles(
        &self,
        tenant_id: &str,
        user_id: &str,
        role_ids: &[String],
    ) -> Result<()> {
        let ts = now_iso();
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM iam_user_role WHERE tenant_id=?1 AND user_id=?2",
            params![tenant_id, user_id],
        )?;
        for rid in role_ids {
            conn.execute(
                "INSERT INTO iam_user_role (ur_id,tenant_id,user_id,role_id,assigned_by,assigned_at,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![new_id(), tenant_id, user_id, rid, None::<String>, Some(&ts), ts],
            )?;
        }
        Ok(())
    }

    // ============================================================
    // 角色 Role（扩展写）
    // ============================================================

    pub fn create_role(
        &self,
        tenant_id: &str,
        role_code: &str,
        role_name: &str,
        role_type: Option<&str>,
        data_scope: Option<&str>,
        sort_order: Option<i64>,
        status: &str,
        description: Option<&str>,
    ) -> Result<IamRole> {
        let ts = now_iso();
        let r = IamRole {
            role_id: new_id(),
            tenant_id: tenant_id.to_string(),
            role_code: role_code.to_string(),
            role_name: role_name.to_string(),
            role_type: role_type.unwrap_or("custom").to_string(),
            parent_id: None,
            inherit_path: None,
            is_builtin: 0,
            data_scope: data_scope.unwrap_or("self").to_string(),
            description: description.map(|s| s.to_string()),
            sort_order,
            status: status.to_string(),
            created_at: ts.clone(),
            updated_at: ts,
            version: 1,
        };
        self.create_role_inner(&r)?;
        Ok(r)
    }

    pub fn update_role(
        &self,
        role_id: &str,
        role_name: Option<&str>,
        role_code: Option<&str>,
        data_scope: Option<&str>,
        sort_order: Option<i64>,
        status: Option<&str>,
        description: Option<&str>,
    ) -> Result<()> {
        let now = now_iso();
        let mut sets: Vec<&str> = vec![];
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![];
        if let Some(v) = &role_name { sets.push("role_name=?"); params.push(v); }
        if let Some(v) = &role_code { sets.push("role_code=?"); params.push(v); }
        if let Some(v) = &data_scope { sets.push("data_scope=?"); params.push(v); }
        if let Some(v) = &sort_order { sets.push("sort_order=?"); params.push(v); }
        if let Some(v) = &status { sets.push("status=?"); params.push(v); }
        if let Some(v) = &description { sets.push("description=?"); params.push(v); }
        if sets.is_empty() {
            return Ok(());
        }
        sets.push("updated_at=?");
        params.push(&now);
        params.push(&role_id);
        let conn = self.conn.lock();
        let sql = format!("UPDATE iam_role SET {} WHERE role_id=?", sets.join(","));
        conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
        Ok(())
    }

    pub fn delete_role(&self, role_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM iam_role_menu WHERE role_id=?1", params![role_id])?;
        conn.execute("DELETE FROM iam_role_permission WHERE role_id=?1", params![role_id])?;
        conn.execute("DELETE FROM iam_user_role WHERE role_id=?1", params![role_id])?;
        conn.execute(
            "DELETE FROM iam_data_permission WHERE subject_type='role' AND subject_id=?1",
            params![role_id],
        )?;
        conn.execute("DELETE FROM iam_role WHERE role_id=?1", params![role_id])?;
        Ok(())
    }

    pub fn get_role_menu_ids(&self, tenant_id: &str, role_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT menu_id FROM iam_role_menu WHERE tenant_id IN (?1, 'system') AND role_id=?2",
        )?;
        let rows = stmt.query_map(params![tenant_id, role_id], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn set_role_menus(
        &self,
        tenant_id: &str,
        role_id: &str,
        menu_ids: &[String],
    ) -> Result<()> {
        let ts = now_iso();
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM iam_role_menu WHERE tenant_id=?1 AND role_id=?2",
            params![tenant_id, role_id],
        )?;
        for mid in menu_ids {
            conn.execute(
                "INSERT INTO iam_role_menu (rm_id,tenant_id,role_id,menu_id,created_by,created_at) VALUES (?1,?2,?3,?4,?5,?6)",
                params![new_id(), tenant_id, role_id, mid, None::<String>, ts],
            )?;
        }
        Ok(())
    }

    pub fn get_role_data_perms(
        &self,
        tenant_id: &str,
        role_id: &str,
    ) -> Result<Vec<IamDataPermission>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT dp_id,tenant_id,dp_code,dp_name,subject_type,subject_id,subject_uuids_json,resource_code,scope_type,custom_rule_expression_sql,custom_rule_expression_json,status,created_at,created_by,updated_at FROM iam_data_permission WHERE tenant_id=?1 AND subject_type='role' AND subject_id=?2",
        )?;
        let rows = stmt.query_map(params![tenant_id, role_id], |r| {
            Ok(IamDataPermission {
                dp_id: r.get(0)?,
                tenant_id: r.get(1)?,
                dp_code: r.get(2)?,
                dp_name: r.get(3)?,
                subject_type: r.get(4)?,
                subject_id: r.get(5)?,
                subject_uuids_json: r.get(6)?,
                resource_code: r.get(7)?,
                scope_type: r.get(8)?,
                custom_rule_expression_sql: r.get(9)?,
                custom_rule_expression_json: r.get(10)?,
                status: r.get(11)?,
                created_at: r.get(12)?,
                created_by: r.get(13)?,
                updated_at: r.get(14)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn set_role_data_perms(
        &self,
        tenant_id: &str,
        role_id: &str,
        dp_codes: &[String],
    ) -> Result<()> {
        let ts = now_iso();
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM iam_data_permission WHERE tenant_id=?1 AND subject_type='role' AND subject_id=?2",
            params![tenant_id, role_id],
        )?;
        for code in dp_codes {
            conn.execute(
                "INSERT INTO iam_data_permission (dp_id,tenant_id,dp_code,dp_name,subject_type,subject_id,subject_uuids_json,resource_code,scope_type,custom_rule_expression_sql,custom_rule_expression_json,status,created_at,created_by,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
                params![
                    new_id(), tenant_id, code, code, "role", role_id, None::<String>,
                    "*", "custom", None::<String>, None::<String>, "active", ts,
                    None::<String>, ts
                ],
            )?;
        }
        Ok(())
    }

    pub fn list_users_by_role(&self, tenant_id: &str, role_id: &str) -> Result<Vec<IamUser>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT u.user_id,u.tenant_id,u.user_code,u.username,u.password_hash,u.real_name,u.nickname,u.email,u.phone,u.avatar,u.dept_id,u.position,u.user_status,u.is_superuser,u.last_login_at,u.last_login_ip,u.created_at,u.updated_at,u.version FROM iam_user u JOIN iam_user_role ur ON u.user_id=ur.user_id WHERE ur.tenant_id=?1 AND ur.role_id=?2",
        )?;
        let rows = stmt.query_map(params![tenant_id, role_id], |r| {
            Ok(IamUser {
                user_id: r.get(0)?,
                tenant_id: r.get(1)?,
                user_code: r.get(2)?,
                username: r.get(3)?,
                password_hash: r.get(4)?,
                real_name: r.get(5)?,
                nickname: r.get(6)?,
                email: r.get(7)?,
                phone: r.get(8)?,
                avatar: r.get(9)?,
                dept_id: r.get(10)?,
                position: r.get(11)?,
                user_status: r.get(12)?,
                is_superuser: r.get(13)?,
                last_login_at: r.get(14)?,
                last_login_ip: r.get(15)?,
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
                version: r.get(18)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn copy_role(
        &self,
        tenant_id: &str,
        source_role_id: &str,
        new_role_code: &str,
        new_role_name: &str,
    ) -> Result<IamRole> {
        let source = self
            .get_role(source_role_id)?
            .ok_or_else(|| anyhow::anyhow!("source role not found: {}", source_role_id))?;
        let ts = now_iso();
        let new_role = IamRole {
            role_id: new_id(),
            tenant_id: tenant_id.to_string(),
            role_code: new_role_code.to_string(),
            role_name: new_role_name.to_string(),
            role_type: source.role_type,
            parent_id: None,
            inherit_path: None,
            is_builtin: 0,
            data_scope: source.data_scope,
            description: source.description,
            sort_order: source.sort_order,
            status: source.status,
            created_at: ts.clone(),
            updated_at: ts,
            version: 1,
        };
        self.create_role_inner(&new_role)?;

        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT menu_id FROM iam_role_menu WHERE tenant_id=?1 AND role_id=?2",
        )?;
        let menu_ids: Vec<String> = stmt
            .query_map(params![tenant_id, source_role_id], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(stmt);
        for mid in &menu_ids {
            conn.execute(
                "INSERT INTO iam_role_menu (rm_id,tenant_id,role_id,menu_id,created_by,created_at) VALUES (?1,?2,?3,?4,?5,?6)",
                params![new_id(), tenant_id, new_role.role_id, mid, None::<String>, now_iso()],
            )?;
        }
        Ok(new_role)
    }

    fn get_role(&self, role_id: &str) -> Result<Option<IamRole>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT role_id,tenant_id,role_code,role_name,role_type,parent_id,inherit_path,is_builtin,data_scope,description,sort_order,status,created_at,updated_at,version FROM iam_role WHERE role_id=?1",
        )?;
        let rows = stmt.query_map(params![role_id], |r| {
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
        let mut items: Vec<IamRole> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    // ============================================================
    // 菜单 Menu（扩展写）
    // ============================================================

    pub fn list_all_menus(&self, tenant_id: &str) -> Result<Vec<IamMenu>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT menu_id,tenant_id,parent_id,menu_code,menu_name,menu_type,menu_category,route_path,route_name,component_path,icon,color,sort_order,is_visible,is_cached,is_external,link_target,permission_code,api_scope,menu_config,children_json,status,created_at,updated_at,version FROM iam_menu WHERE tenant_id IN (?1, 'system') ORDER BY sort_order ASC",
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
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

    pub fn get_menu(&self, menu_id: &str) -> Result<Option<IamMenu>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT menu_id,tenant_id,parent_id,menu_code,menu_name,menu_type,menu_category,route_path,route_name,component_path,icon,color,sort_order,is_visible,is_cached,is_external,link_target,permission_code,api_scope,menu_config,children_json,status,created_at,updated_at,version FROM iam_menu WHERE menu_id=?1",
        )?;
        let rows = stmt.query_map(params![menu_id], |r| {
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
        let mut items: Vec<IamMenu> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    pub fn create_menu(
        &self,
        tenant_id: &str,
        menu_code: &str,
        menu_name: &str,
        menu_type: Option<&str>,
        parent_id: Option<&str>,
        route_path: Option<&str>,
        component_path: Option<&str>,
        icon: Option<&str>,
        permission_code: Option<&str>,
        sort_order: Option<i64>,
        is_visible: Option<i64>,
        status: &str,
    ) -> Result<IamMenu> {
        let ts = now_iso();
        let m = IamMenu {
            menu_id: new_id(),
            tenant_id: tenant_id.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            menu_code: menu_code.to_string(),
            menu_name: menu_name.to_string(),
            menu_type: menu_type.unwrap_or("menu").to_string(),
            menu_category: None,
            route_path: route_path.map(|s| s.to_string()),
            route_name: None,
            component_path: component_path.map(|s| s.to_string()),
            icon: icon.map(|s| s.to_string()),
            color: None,
            sort_order,
            is_visible: is_visible.unwrap_or(1),
            is_cached: 0,
            is_external: 0,
            link_target: None,
            permission_code: permission_code.map(|s| s.to_string()),
            api_scope: None,
            menu_config: None,
            children_json: None,
            status: status.to_string(),
            created_at: ts.clone(),
            updated_at: ts,
            version: 1,
        };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO iam_menu (menu_id,tenant_id,parent_id,menu_code,menu_name,menu_type,menu_category,route_path,route_name,component_path,icon,color,sort_order,is_visible,is_cached,is_external,link_target,permission_code,api_scope,menu_config,children_json,status,created_at,updated_at,version) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)",
            params![
                m.menu_id, m.tenant_id, m.parent_id, m.menu_code, m.menu_name, m.menu_type,
                m.menu_category, m.route_path, m.route_name, m.component_path, m.icon, m.color,
                m.sort_order, m.is_visible, m.is_cached, m.is_external, m.link_target,
                m.permission_code, m.api_scope, m.menu_config, m.children_json, m.status,
                m.created_at, m.updated_at, m.version
            ],
        )?;
        Ok(m)
    }

    pub fn update_menu(
        &self,
        menu_id: &str,
        menu_name: Option<&str>,
        parent_id: Option<&str>,
        route_path: Option<&str>,
        component_path: Option<&str>,
        icon: Option<&str>,
        permission_code: Option<&str>,
        sort_order: Option<i64>,
        is_visible: Option<i64>,
        status: Option<&str>,
    ) -> Result<()> {
        let now = now_iso();
        let mut sets: Vec<&str> = vec![];
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![];
        if let Some(v) = &menu_name { sets.push("menu_name=?"); params.push(v); }
        if let Some(v) = &parent_id { sets.push("parent_id=?"); params.push(v); }
        if let Some(v) = &route_path { sets.push("route_path=?"); params.push(v); }
        if let Some(v) = &component_path { sets.push("component_path=?"); params.push(v); }
        if let Some(v) = &icon { sets.push("icon=?"); params.push(v); }
        if let Some(v) = &permission_code { sets.push("permission_code=?"); params.push(v); }
        if let Some(v) = &sort_order { sets.push("sort_order=?"); params.push(v); }
        if let Some(v) = &is_visible { sets.push("is_visible=?"); params.push(v); }
        if let Some(v) = &status { sets.push("status=?"); params.push(v); }
        if sets.is_empty() {
            return Ok(());
        }
        sets.push("updated_at=?");
        params.push(&now);
        params.push(&menu_id);
        let conn = self.conn.lock();
        let sql = format!("UPDATE iam_menu SET {} WHERE menu_id=?", sets.join(","));
        conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
        Ok(())
    }

    pub fn delete_menu(&self, menu_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM iam_role_menu WHERE menu_id=?1", params![menu_id])?;
        conn.execute("DELETE FROM iam_user_menu WHERE menu_id=?1", params![menu_id])?;
        conn.execute("DELETE FROM iam_menu WHERE menu_id=?1", params![menu_id])?;
        Ok(())
    }

    // ============================================================
    // 字典类型 DictType（全新）
    // ============================================================

    pub fn create_dict_type(
        &self,
        tenant_id: &str,
        dict_name: &str,
        dict_type: &str,
        status: &str,
        remark: Option<&str>,
    ) -> Result<SysDictType> {
        let ts = now_iso();
        let d = SysDictType {
            dict_id: new_id(),
            tenant_id: tenant_id.to_string(),
            dict_name: dict_name.to_string(),
            dict_type: dict_type.to_string(),
            status: status.to_string(),
            remark: remark.map(|s| s.to_string()),
            created_at: ts.clone(),
            updated_at: ts,
        };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sys_dict_type (dict_id,tenant_id,dict_name,dict_type,status,remark,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![d.dict_id, d.tenant_id, d.dict_name, d.dict_type, d.status, d.remark, d.created_at, d.updated_at],
        )?;
        Ok(d)
    }

    pub fn get_dict_type(&self, dict_id: &str) -> Result<Option<SysDictType>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT dict_id,tenant_id,dict_name,dict_type,status,remark,created_at,updated_at FROM sys_dict_type WHERE dict_id=?1",
        )?;
        let rows = stmt.query_map(params![dict_id], |r| {
            Ok(SysDictType {
                dict_id: r.get(0)?,
                tenant_id: r.get(1)?,
                dict_name: r.get(2)?,
                dict_type: r.get(3)?,
                status: r.get(4)?,
                remark: r.get(5)?,
                created_at: r.get(6)?,
                updated_at: r.get(7)?,
            })
        })?;
        let mut items: Vec<SysDictType> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    pub fn list_dict_types(&self, tenant_id: &str) -> Result<Vec<SysDictType>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT dict_id,tenant_id,dict_name,dict_type,status,remark,created_at,updated_at FROM sys_dict_type WHERE tenant_id=?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
            Ok(SysDictType {
                dict_id: r.get(0)?,
                tenant_id: r.get(1)?,
                dict_name: r.get(2)?,
                dict_type: r.get(3)?,
                status: r.get(4)?,
                remark: r.get(5)?,
                created_at: r.get(6)?,
                updated_at: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn update_dict_type(
        &self,
        dict_id: &str,
        dict_name: Option<&str>,
        dict_type: Option<&str>,
        status: Option<&str>,
        remark: Option<&str>,
    ) -> Result<()> {
        let now = now_iso();
        let mut sets: Vec<&str> = vec![];
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![];
        if let Some(v) = &dict_name { sets.push("dict_name=?"); params.push(v); }
        if let Some(v) = &dict_type { sets.push("dict_type=?"); params.push(v); }
        if let Some(v) = &status { sets.push("status=?"); params.push(v); }
        if let Some(v) = &remark { sets.push("remark=?"); params.push(v); }
        if sets.is_empty() {
            return Ok(());
        }
        sets.push("updated_at=?");
        params.push(&now);
        params.push(&dict_id);
        let conn = self.conn.lock();
        let sql = format!("UPDATE sys_dict_type SET {} WHERE dict_id=?", sets.join(","));
        conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
        Ok(())
    }

    pub fn delete_dict_type(&self, dict_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        // 先查出 dict_type 和 tenant_id，再级联删除 sys_dict_data
        let mut stmt = conn.prepare(
            "SELECT tenant_id, dict_type FROM sys_dict_type WHERE dict_id=?1",
        )?;
        let mut rows = stmt.query(params![dict_id])?;
        if let Some(row) = rows.next()? {
            let tid: String = row.get(0)?;
            let dtype: String = row.get(1)?;
            drop(rows);
            drop(stmt);
            conn.execute(
                "DELETE FROM sys_dict_data WHERE tenant_id=?1 AND dict_type=?2",
                params![tid, dtype],
            )?;
        }
        conn.execute("DELETE FROM sys_dict_type WHERE dict_id=?1", params![dict_id])?;
        Ok(())
    }

    // ============================================================
    // 字典数据 DictData（全新）
    // ============================================================

    pub fn create_dict_data(
        &self,
        tenant_id: &str,
        dict_sort: Option<i64>,
        dict_label: &str,
        dict_value: &str,
        dict_type: &str,
        css_class: Option<&str>,
        list_class: Option<&str>,
        is_default: Option<&str>,
        status: &str,
        remark: Option<&str>,
    ) -> Result<SysDictData> {
        let ts = now_iso();
        let d = SysDictData {
            dict_code: new_id(),
            tenant_id: tenant_id.to_string(),
            dict_sort,
            dict_label: dict_label.to_string(),
            dict_value: dict_value.to_string(),
            dict_type: dict_type.to_string(),
            css_class: css_class.map(|s| s.to_string()),
            list_class: list_class.map(|s| s.to_string()),
            is_default: is_default.map(|s| s.to_string()),
            status: status.to_string(),
            remark: remark.map(|s| s.to_string()),
            created_at: ts.clone(),
            updated_at: ts,
        };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sys_dict_data (dict_code,tenant_id,dict_sort,dict_label,dict_value,dict_type,css_class,list_class,is_default,status,remark,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                d.dict_code, d.tenant_id, d.dict_sort, d.dict_label, d.dict_value,
                d.dict_type, d.css_class, d.list_class, d.is_default, d.status, d.remark,
                d.created_at, d.updated_at
            ],
        )?;
        Ok(d)
    }

    pub fn get_dict_data(&self, dict_code: &str) -> Result<Option<SysDictData>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT dict_code,tenant_id,dict_sort,dict_label,dict_value,dict_type,css_class,list_class,is_default,status,remark,created_at,updated_at FROM sys_dict_data WHERE dict_code=?1",
        )?;
        let rows = stmt.query_map(params![dict_code], |r| {
            Ok(SysDictData {
                dict_code: r.get(0)?,
                tenant_id: r.get(1)?,
                dict_sort: r.get(2)?,
                dict_label: r.get(3)?,
                dict_value: r.get(4)?,
                dict_type: r.get(5)?,
                css_class: r.get(6)?,
                list_class: r.get(7)?,
                is_default: r.get(8)?,
                status: r.get(9)?,
                remark: r.get(10)?,
                created_at: r.get(11)?,
                updated_at: r.get(12)?,
            })
        })?;
        let mut items: Vec<SysDictData> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    pub fn list_dict_data(&self, tenant_id: &str) -> Result<Vec<SysDictData>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT dict_code,tenant_id,dict_sort,dict_label,dict_value,dict_type,css_class,list_class,is_default,status,remark,created_at,updated_at FROM sys_dict_data WHERE tenant_id=?1 ORDER BY dict_sort ASC",
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
            Ok(SysDictData {
                dict_code: r.get(0)?,
                tenant_id: r.get(1)?,
                dict_sort: r.get(2)?,
                dict_label: r.get(3)?,
                dict_value: r.get(4)?,
                dict_type: r.get(5)?,
                css_class: r.get(6)?,
                list_class: r.get(7)?,
                is_default: r.get(8)?,
                status: r.get(9)?,
                remark: r.get(10)?,
                created_at: r.get(11)?,
                updated_at: r.get(12)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn list_dict_data_by_type(
        &self,
        tenant_id: &str,
        dict_type: &str,
    ) -> Result<Vec<SysDictData>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT dict_code,tenant_id,dict_sort,dict_label,dict_value,dict_type,css_class,list_class,is_default,status,remark,created_at,updated_at FROM sys_dict_data WHERE tenant_id=?1 AND dict_type=?2 ORDER BY dict_sort ASC",
        )?;
        let rows = stmt.query_map(params![tenant_id, dict_type], |r| {
            Ok(SysDictData {
                dict_code: r.get(0)?,
                tenant_id: r.get(1)?,
                dict_sort: r.get(2)?,
                dict_label: r.get(3)?,
                dict_value: r.get(4)?,
                dict_type: r.get(5)?,
                css_class: r.get(6)?,
                list_class: r.get(7)?,
                is_default: r.get(8)?,
                status: r.get(9)?,
                remark: r.get(10)?,
                created_at: r.get(11)?,
                updated_at: r.get(12)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn update_dict_data(
        &self,
        dict_code: &str,
        dict_sort: Option<i64>,
        dict_label: Option<&str>,
        dict_value: Option<&str>,
        dict_type: Option<&str>,
        css_class: Option<&str>,
        list_class: Option<&str>,
        is_default: Option<&str>,
        status: Option<&str>,
        remark: Option<&str>,
    ) -> Result<()> {
        let now = now_iso();
        let mut sets: Vec<&str> = vec![];
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![];
        if let Some(v) = &dict_sort { sets.push("dict_sort=?"); params.push(v); }
        if let Some(v) = &dict_label { sets.push("dict_label=?"); params.push(v); }
        if let Some(v) = &dict_value { sets.push("dict_value=?"); params.push(v); }
        if let Some(v) = &dict_type { sets.push("dict_type=?"); params.push(v); }
        if let Some(v) = &css_class { sets.push("css_class=?"); params.push(v); }
        if let Some(v) = &list_class { sets.push("list_class=?"); params.push(v); }
        if let Some(v) = &is_default { sets.push("is_default=?"); params.push(v); }
        if let Some(v) = &status { sets.push("status=?"); params.push(v); }
        if let Some(v) = &remark { sets.push("remark=?"); params.push(v); }
        if sets.is_empty() {
            return Ok(());
        }
        sets.push("updated_at=?");
        params.push(&now);
        params.push(&dict_code);
        let conn = self.conn.lock();
        let sql = format!("UPDATE sys_dict_data SET {} WHERE dict_code=?", sets.join(","));
        conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
        Ok(())
    }

    pub fn delete_dict_data(&self, dict_code: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM sys_dict_data WHERE dict_code=?1", params![dict_code])?;
        Ok(())
    }

    // ============================================================
    // 参数配置 Config（全新）
    // ============================================================

    pub fn create_config(
        &self,
        tenant_id: &str,
        config_name: &str,
        config_key: &str,
        config_value: Option<&str>,
        config_type: Option<&str>,
        status: &str,
        remark: Option<&str>,
    ) -> Result<SysConfig> {
        let ts = now_iso();
        let c = SysConfig {
            config_id: new_id(),
            tenant_id: tenant_id.to_string(),
            config_name: config_name.to_string(),
            config_key: config_key.to_string(),
            config_value: config_value.map(|s| s.to_string()),
            config_type: config_type.map(|s| s.to_string()),
            status: status.to_string(),
            remark: remark.map(|s| s.to_string()),
            created_at: ts.clone(),
            updated_at: ts,
        };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sys_config (config_id,tenant_id,config_name,config_key,config_value,config_type,status,remark,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                c.config_id, c.tenant_id, c.config_name, c.config_key, c.config_value,
                c.config_type, c.status, c.remark, c.created_at, c.updated_at
            ],
        )?;
        Ok(c)
    }

    pub fn get_config(&self, config_id: &str) -> Result<Option<SysConfig>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT config_id,tenant_id,config_name,config_key,config_value,config_type,status,remark,created_at,updated_at FROM sys_config WHERE config_id=?1",
        )?;
        let rows = stmt.query_map(params![config_id], |r| {
            Ok(SysConfig {
                config_id: r.get(0)?,
                tenant_id: r.get(1)?,
                config_name: r.get(2)?,
                config_key: r.get(3)?,
                config_value: r.get(4)?,
                config_type: r.get(5)?,
                status: r.get(6)?,
                remark: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?;
        let mut items: Vec<SysConfig> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    pub fn get_config_by_key(
        &self,
        tenant_id: &str,
        config_key: &str,
    ) -> Result<Option<SysConfig>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT config_id,tenant_id,config_name,config_key,config_value,config_type,status,remark,created_at,updated_at FROM sys_config WHERE tenant_id=?1 AND config_key=?2",
        )?;
        let rows = stmt.query_map(params![tenant_id, config_key], |r| {
            Ok(SysConfig {
                config_id: r.get(0)?,
                tenant_id: r.get(1)?,
                config_name: r.get(2)?,
                config_key: r.get(3)?,
                config_value: r.get(4)?,
                config_type: r.get(5)?,
                status: r.get(6)?,
                remark: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?;
        let mut items: Vec<SysConfig> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    pub fn list_configs(&self, tenant_id: &str) -> Result<Vec<SysConfig>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT config_id,tenant_id,config_name,config_key,config_value,config_type,status,remark,created_at,updated_at FROM sys_config WHERE tenant_id=?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
            Ok(SysConfig {
                config_id: r.get(0)?,
                tenant_id: r.get(1)?,
                config_name: r.get(2)?,
                config_key: r.get(3)?,
                config_value: r.get(4)?,
                config_type: r.get(5)?,
                status: r.get(6)?,
                remark: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn update_config(
        &self,
        config_id: &str,
        config_name: Option<&str>,
        config_key: Option<&str>,
        config_value: Option<&str>,
        config_type: Option<&str>,
        status: Option<&str>,
        remark: Option<&str>,
    ) -> Result<()> {
        let now = now_iso();
        let mut sets: Vec<&str> = vec![];
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![];
        if let Some(v) = &config_name { sets.push("config_name=?"); params.push(v); }
        if let Some(v) = &config_key { sets.push("config_key=?"); params.push(v); }
        if let Some(v) = &config_value { sets.push("config_value=?"); params.push(v); }
        if let Some(v) = &config_type { sets.push("config_type=?"); params.push(v); }
        if let Some(v) = &status { sets.push("status=?"); params.push(v); }
        if let Some(v) = &remark { sets.push("remark=?"); params.push(v); }
        if sets.is_empty() {
            return Ok(());
        }
        sets.push("updated_at=?");
        params.push(&now);
        params.push(&config_id);
        let conn = self.conn.lock();
        let sql = format!("UPDATE sys_config SET {} WHERE config_id=?", sets.join(","));
        conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
        Ok(())
    }

    pub fn delete_config(&self, config_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM sys_config WHERE config_id=?1", params![config_id])?;
        Ok(())
    }

    // ============================================================
    // 操作日志 OperLog（全新）
    // ============================================================

    pub fn create_oper_log(
        &self,
        tenant_id: &str,
        title: Option<&str>,
        business_type: Option<i64>,
        method: Option<&str>,
        request_method: Option<&str>,
        oper_name: Option<&str>,
        oper_url: Option<&str>,
        oper_ip: Option<&str>,
        oper_param: Option<&str>,
        json_result: Option<&str>,
        status: Option<i64>,
        error_msg: Option<&str>,
    ) -> Result<SysOperLog> {
        let ts = now_iso();
        let log = SysOperLog {
            oper_id: new_id(),
            tenant_id: tenant_id.to_string(),
            title: title.map(|s| s.to_string()),
            business_type,
            method: method.map(|s| s.to_string()),
            request_method: request_method.map(|s| s.to_string()),
            operator_type: Some(0),
            oper_name: oper_name.map(|s| s.to_string()),
            dept_name: None,
            oper_url: oper_url.map(|s| s.to_string()),
            oper_ip: oper_ip.map(|s| s.to_string()),
            oper_location: None,
            oper_param: oper_param.map(|s| s.to_string()),
            json_result: json_result.map(|s| s.to_string()),
            status,
            error_msg: error_msg.map(|s| s.to_string()),
            oper_time: ts,
            cost_time: Some(0),
        };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sys_oper_log (oper_id,tenant_id,title,business_type,method,request_method,operator_type,oper_name,dept_name,oper_url,oper_ip,oper_location,oper_param,json_result,status,error_msg,oper_time,cost_time) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
            params![
                log.oper_id, log.tenant_id, log.title, log.business_type, log.method,
                log.request_method, log.operator_type, log.oper_name, log.dept_name,
                log.oper_url, log.oper_ip, log.oper_location, log.oper_param,
                log.json_result, log.status, log.error_msg, log.oper_time, log.cost_time
            ],
        )?;
        Ok(log)
    }

    pub fn get_oper_log(&self, oper_id: &str) -> Result<Option<SysOperLog>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT oper_id,tenant_id,title,business_type,method,request_method,operator_type,oper_name,dept_name,oper_url,oper_ip,oper_location,oper_param,json_result,status,error_msg,oper_time,cost_time FROM sys_oper_log WHERE oper_id=?1",
        )?;
        let rows = stmt.query_map(params![oper_id], |r| {
            Ok(SysOperLog {
                oper_id: r.get(0)?,
                tenant_id: r.get(1)?,
                title: r.get(2)?,
                business_type: r.get(3)?,
                method: r.get(4)?,
                request_method: r.get(5)?,
                operator_type: r.get(6)?,
                oper_name: r.get(7)?,
                dept_name: r.get(8)?,
                oper_url: r.get(9)?,
                oper_ip: r.get(10)?,
                oper_location: r.get(11)?,
                oper_param: r.get(12)?,
                json_result: r.get(13)?,
                status: r.get(14)?,
                error_msg: r.get(15)?,
                oper_time: r.get(16)?,
                cost_time: r.get(17)?,
            })
        })?;
        let mut items: Vec<SysOperLog> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    pub fn list_oper_logs(&self, tenant_id: &str) -> Result<Vec<SysOperLog>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT oper_id,tenant_id,title,business_type,method,request_method,operator_type,oper_name,dept_name,oper_url,oper_ip,oper_location,oper_param,json_result,status,error_msg,oper_time,cost_time FROM sys_oper_log WHERE tenant_id=?1 ORDER BY oper_time DESC",
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
            Ok(SysOperLog {
                oper_id: r.get(0)?,
                tenant_id: r.get(1)?,
                title: r.get(2)?,
                business_type: r.get(3)?,
                method: r.get(4)?,
                request_method: r.get(5)?,
                operator_type: r.get(6)?,
                oper_name: r.get(7)?,
                dept_name: r.get(8)?,
                oper_url: r.get(9)?,
                oper_ip: r.get(10)?,
                oper_location: r.get(11)?,
                oper_param: r.get(12)?,
                json_result: r.get(13)?,
                status: r.get(14)?,
                error_msg: r.get(15)?,
                oper_time: r.get(16)?,
                cost_time: r.get(17)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn delete_oper_log(&self, oper_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM sys_oper_log WHERE oper_id=?1", params![oper_id])?;
        Ok(())
    }

    pub fn clean_oper_logs(&self, tenant_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM sys_oper_log WHERE tenant_id=?1", params![tenant_id])?;
        Ok(())
    }

    // ============================================================
    // 登录日志 LoginLog（全新）
    // ============================================================

    pub fn create_login_log(
        &self,
        tenant_id: &str,
        user_name: Option<&str>,
        ipaddr: Option<&str>,
        browser: Option<&str>,
        os: Option<&str>,
        status: Option<&str>,
        msg: Option<&str>,
    ) -> Result<SysLoginLog> {
        let ts = now_iso();
        let log = SysLoginLog {
            info_id: new_id(),
            tenant_id: tenant_id.to_string(),
            user_name: user_name.map(|s| s.to_string()),
            ipaddr: ipaddr.map(|s| s.to_string()),
            login_location: None,
            browser: browser.map(|s| s.to_string()),
            os: os.map(|s| s.to_string()),
            status: status.map(|s| s.to_string()),
            msg: msg.map(|s| s.to_string()),
            login_time: ts,
        };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sys_logininfor (info_id,tenant_id,user_name,ipaddr,login_location,browser,os,status,msg,login_time) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                log.info_id, log.tenant_id, log.user_name, log.ipaddr, log.login_location,
                log.browser, log.os, log.status, log.msg, log.login_time
            ],
        )?;
        Ok(log)
    }

    pub fn list_login_logs(&self, tenant_id: &str) -> Result<Vec<SysLoginLog>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT info_id,tenant_id,user_name,ipaddr,login_location,browser,os,status,msg,login_time FROM sys_logininfor WHERE tenant_id=?1 ORDER BY login_time DESC",
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
            Ok(SysLoginLog {
                info_id: r.get(0)?,
                tenant_id: r.get(1)?,
                user_name: r.get(2)?,
                ipaddr: r.get(3)?,
                login_location: r.get(4)?,
                browser: r.get(5)?,
                os: r.get(6)?,
                status: r.get(7)?,
                msg: r.get(8)?,
                login_time: r.get(9)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn delete_login_log(&self, info_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM sys_logininfor WHERE info_id=?1", params![info_id])?;
        Ok(())
    }

    pub fn clean_login_logs(&self, tenant_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM sys_logininfor WHERE tenant_id=?1", params![tenant_id])?;
        Ok(())
    }

    // ============================================================
    // API Key（全新）
    // ============================================================

    pub fn create_api_key(
        &self,
        tenant_id: &str,
        name: &str,
        api_key: &str,
        user_id: Option<&str>,
        scopes: Option<&str>,
    ) -> Result<SysApiKey> {
        let ts = now_iso();
        let k = SysApiKey {
            key_id: new_id(),
            tenant_id: tenant_id.to_string(),
            name: name.to_string(),
            api_key: api_key.to_string(),
            user_id: user_id.map(|s| s.to_string()),
            scopes: scopes.map(|s| s.to_string()),
            status: "active".to_string(),
            expires_at: None,
            last_used_at: None,
            created_at: ts,
            revoked_at: None,
        };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sys_api_key (key_id,tenant_id,name,api_key,user_id,scopes,status,expires_at,last_used_at,created_at,revoked_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                k.key_id, k.tenant_id, k.name, k.api_key, k.user_id, k.scopes, k.status,
                k.expires_at, k.last_used_at, k.created_at, k.revoked_at
            ],
        )?;
        Ok(k)
    }

    pub fn list_api_keys(&self, tenant_id: &str) -> Result<Vec<SysApiKey>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT key_id,tenant_id,name,api_key,user_id,scopes,status,expires_at,last_used_at,created_at,revoked_at FROM sys_api_key WHERE tenant_id=?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
            Ok(SysApiKey {
                key_id: r.get(0)?,
                tenant_id: r.get(1)?,
                name: r.get(2)?,
                api_key: r.get(3)?,
                user_id: r.get(4)?,
                scopes: r.get(5)?,
                status: r.get(6)?,
                expires_at: r.get(7)?,
                last_used_at: r.get(8)?,
                created_at: r.get(9)?,
                revoked_at: r.get(10)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn get_api_key(&self, key_id: &str) -> Result<Option<SysApiKey>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT key_id,tenant_id,name,api_key,user_id,scopes,status,expires_at,last_used_at,created_at,revoked_at FROM sys_api_key WHERE key_id=?1",
        )?;
        let rows = stmt.query_map(params![key_id], |r| {
            Ok(SysApiKey {
                key_id: r.get(0)?,
                tenant_id: r.get(1)?,
                name: r.get(2)?,
                api_key: r.get(3)?,
                user_id: r.get(4)?,
                scopes: r.get(5)?,
                status: r.get(6)?,
                expires_at: r.get(7)?,
                last_used_at: r.get(8)?,
                created_at: r.get(9)?,
                revoked_at: r.get(10)?,
            })
        })?;
        let mut items: Vec<SysApiKey> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    pub fn revoke_api_key(&self, key_id: &str) -> Result<()> {
        let now = now_iso();
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE sys_api_key SET status='revoked', revoked_at=?1 WHERE key_id=?2",
            params![now, key_id],
        )?;
        Ok(())
    }

    pub fn validate_api_key(&self, api_key: &str) -> Result<Option<SysApiKey>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT key_id,tenant_id,name,api_key,user_id,scopes,status,expires_at,last_used_at,created_at,revoked_at FROM sys_api_key WHERE api_key=?1 AND status='active'",
        )?;
        let rows = stmt.query_map(params![api_key], |r| {
            Ok(SysApiKey {
                key_id: r.get(0)?,
                tenant_id: r.get(1)?,
                name: r.get(2)?,
                api_key: r.get(3)?,
                user_id: r.get(4)?,
                scopes: r.get(5)?,
                status: r.get(6)?,
                expires_at: r.get(7)?,
                last_used_at: r.get(8)?,
                created_at: r.get(9)?,
                revoked_at: r.get(10)?,
            })
        })?;
        let mut items: Vec<SysApiKey> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items.pop())
    }

    // ============================================================
    // 审计日志 AuditLog（扩展读/写）
    // ============================================================

    pub fn list_audit_logs(&self, tenant_id: &str) -> Result<Vec<AuditLog>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT log_id,tenant_id,trace_id,request_id,user_id,user_ip,action,action_detail,resource_type,resource_id,resource_code,biz_id,biz_code,status_code,http_method,http_path,latency_ms,snapshot_before,snapshot_after,changed_fields,prev_hash,curr_hash,created_at FROM audit_log WHERE tenant_id=?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![tenant_id], |r| {
            Ok(AuditLog {
                log_id: r.get(0)?,
                tenant_id: r.get(1)?,
                trace_id: r.get(2)?,
                request_id: r.get(3)?,
                user_id: r.get(4)?,
                user_ip: r.get(5)?,
                action: r.get(6)?,
                action_detail: r.get(7)?,
                resource_type: r.get(8)?,
                resource_id: r.get(9)?,
                resource_code: r.get(10)?,
                biz_id: r.get(11)?,
                biz_code: r.get(12)?,
                status_code: r.get(13)?,
                http_method: r.get(14)?,
                http_path: r.get(15)?,
                latency_ms: r.get(16)?,
                snapshot_before: r.get(17)?,
                snapshot_after: r.get(18)?,
                changed_fields: r.get(19)?,
                prev_hash: r.get(20)?,
                curr_hash: r.get(21)?,
                created_at: r.get(22)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn create_audit_log(
        &self,
        tenant_id: &str,
        action: &str,
        user_id: Option<&str>,
        resource_type: Option<&str>,
        resource_id: Option<&str>,
        status_code: Option<i64>,
        http_method: Option<&str>,
        http_path: Option<&str>,
    ) -> Result<AuditLog> {
        let ts = now_iso();
        let curr_hash = new_id();
        let conn = self.conn.lock();
        // 取上一条审计日志的 curr_hash 作为 prev_hash
        let mut stmt = conn.prepare(
            "SELECT curr_hash FROM audit_log WHERE tenant_id=?1 ORDER BY created_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query(params![tenant_id])?;
        let prev_hash: Option<String> = if let Some(row) = rows.next()? {
            row.get(0).ok()
        } else {
            None
        };
        drop(rows);
        drop(stmt);

        let log = AuditLog {
            log_id: new_id(),
            tenant_id: tenant_id.to_string(),
            trace_id: None,
            request_id: None,
            user_id: user_id.map(|s| s.to_string()),
            user_ip: None,
            action: action.to_string(),
            action_detail: None,
            resource_type: resource_type.map(|s| s.to_string()),
            resource_id: resource_id.map(|s| s.to_string()),
            resource_code: None,
            biz_id: None,
            biz_code: None,
            status_code,
            http_method: http_method.map(|s| s.to_string()),
            http_path: http_path.map(|s| s.to_string()),
            latency_ms: None,
            snapshot_before: None,
            snapshot_after: None,
            changed_fields: None,
            prev_hash,
            curr_hash,
            created_at: ts,
        };
        conn.execute(
            "INSERT INTO audit_log (log_id,tenant_id,trace_id,request_id,user_id,user_ip,action,action_detail,resource_type,resource_id,resource_code,biz_id,biz_code,status_code,http_method,http_path,latency_ms,snapshot_before,snapshot_after,changed_fields,prev_hash,curr_hash,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)",
            params![
                log.log_id, log.tenant_id, log.trace_id, log.request_id, log.user_id,
                log.user_ip, log.action, log.action_detail, log.resource_type, log.resource_id,
                log.resource_code, log.biz_id, log.biz_code, log.status_code, log.http_method,
                log.http_path, log.latency_ms, log.snapshot_before, log.snapshot_after,
                log.changed_fields, log.prev_hash, log.curr_hash, log.created_at
            ],
        )?;
        Ok(log)
    }
}

#[derive(Clone, Debug)]
pub struct UserRole {
    pub id: String,
    pub code: String,
    pub name: String,
    pub permissions: Vec<String>,
}
