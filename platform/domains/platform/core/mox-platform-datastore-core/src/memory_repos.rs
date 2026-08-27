//! 内存元数据仓库与 IAM 仓库
//!
//! 用于测试和轻量级部署场景，提供 MetaRepo 和 IamRepo 的内存实现。

use crate::field::FieldSpec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// 用户实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: String,
    pub tenant_id: String,
    pub username: String,
    pub dept_id: String,
}

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub timestamp: String,
    pub tenant_id: String,
    pub user_id: String,
    pub action: String,
    pub entity_code: String,
    pub biz_id: Option<String>,
    pub success: bool,
    pub detail: String,
}

/// 内存元数据仓库
///
/// 管理租户级别的业务实体定义、字段规格、行业模板等元数据。
#[derive(Clone)]
pub struct InMemoryMetaRepo {
    inner: std::sync::Arc<Mutex<MetaState>>,
}

struct MetaState {
    /// tenant_id -> (entity_type -> Vec<FieldSpec>)
    entity_fields: HashMap<String, HashMap<String, Vec<FieldSpec>>>,
    /// tenant_id -> industry_code
    industries: HashMap<String, String>,
}

impl InMemoryMetaRepo {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(Mutex::new(MetaState {
                entity_fields: HashMap::new(),
                industries: HashMap::new(),
            })),
        }
    }

    /// 初始化通用行业模板（为指定租户预置 project 实体的标准字段）
    pub fn init_common_industry(&self, tenant_id: &str) {
        let mut state = self.inner.lock().unwrap();
        state.industries.insert(tenant_id.to_string(), "common".to_string());

        let project_fields = vec![
            FieldSpec {
                field_code: "title".into(),
                field_type: "string".into(),
                is_required: true,
                is_indexed: true,
                is_searchable: true,
                is_sortable: true,
                is_filterable: true,
                options_inline: None,
            },
            FieldSpec {
                field_code: "amount".into(),
                field_type: "decimal".into(),
                is_required: false,
                is_indexed: false,
                is_searchable: false,
                is_sortable: true,
                is_filterable: true,
                options_inline: None,
            },
            FieldSpec {
                field_code: "status".into(),
                field_type: "string".into(),
                is_required: false,
                is_indexed: true,
                is_searchable: false,
                is_sortable: true,
                is_filterable: true,
                options_inline: Some(vec!["draft".into(), "active".into(), "closed".into()]),
            },
            FieldSpec {
                field_code: "description".into(),
                field_type: "string".into(),
                is_required: false,
                is_indexed: false,
                is_searchable: true,
                is_sortable: false,
                is_filterable: false,
                options_inline: None,
            },
        ];

        let tenant_entities = state.entity_fields.entry(tenant_id.to_string()).or_default();
        tenant_entities.insert("project".to_string(), project_fields);
    }

    /// 获取指定租户、指定实体类型的字段规格列表
    pub fn get_entity_fields(&self, tenant_id: &str, entity_type: &str) -> Vec<FieldSpec> {
        let state = self.inner.lock().unwrap();
        state
            .entity_fields
            .get(tenant_id)
            .and_then(|entities| entities.get(entity_type))
            .cloned()
            .unwrap_or_default()
    }

    /// 注册实体字段定义
    pub fn register_entity_fields(&self, tenant_id: &str, entity_type: &str, fields: Vec<FieldSpec>) {
        let mut state = self.inner.lock().unwrap();
        state
            .entity_fields
            .entry(tenant_id.to_string())
            .or_default()
            .insert(entity_type.to_string(), fields);
    }

    /// 获取租户行业代码
    pub fn get_industry(&self, tenant_id: &str) -> Option<String> {
        let state = self.inner.lock().unwrap();
        state.industries.get(tenant_id).cloned()
    }
}

impl Default for InMemoryMetaRepo {
    fn default() -> Self { Self::new() }
}

/// 内存 IAM 仓库
///
/// 管理租户、用户、角色、权限等身份与访问控制信息。
#[derive(Clone)]
pub struct InMemoryIamRepo {
    inner: std::sync::Arc<Mutex<IamState>>,
    /// 审计日志（公共可访问，用于编排器审计追踪）
    pub audit_logs: std::sync::Arc<Mutex<Vec<AuditLog>>>,
    /// 用户注册表（user_id -> User）
    users: std::sync::Arc<Mutex<HashMap<String, User>>>,
}

struct IamState {
    /// tenant_id -> set of user_id
    tenant_users: HashMap<String, Vec<String>>,
    /// user_id -> (tenant_id, roles)
    user_roles: HashMap<String, (String, Vec<String>)>,
    /// (tenant_id, user_id) -> permission set
    user_permissions: HashMap<(String, String), Vec<String>>,
}

impl InMemoryIamRepo {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(Mutex::new(IamState {
                tenant_users: HashMap::new(),
                user_roles: HashMap::new(),
                user_permissions: HashMap::new(),
            })),
            audit_logs: std::sync::Arc::new(Mutex::new(Vec::new())),
            users: std::sync::Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 添加用户（无权限，用于权限拒绝测试）
    pub fn add_user(&self, user: User) {
        let mut users = self.users.lock().unwrap();
        users.insert(user.user_id.clone(), user.clone());

        let mut state = self.inner.lock().unwrap();
        state
            .tenant_users
            .entry(user.tenant_id.clone())
            .or_default()
            .push(user.user_id.clone());
        state.user_roles.insert(
            user.user_id.clone(),
            (user.tenant_id.clone(), vec!["guest".to_string()]),
        );
        // 不分配任何 biz:* 权限
    }

    /// 追加审计日志
    pub fn append_audit(&self, log: AuditLog) {
        let mut logs = self.audit_logs.lock().unwrap();
        logs.push(log);
    }

    /// 初始化标准用户（为指定租户创建标准用户，赋予基础权限）
    pub fn init_standard_user(&self, tenant_id: &str, user_id: &str) {
        let mut state = self.inner.lock().unwrap();

        // 注册租户用户关系
        state
            .tenant_users
            .entry(tenant_id.to_string())
            .or_default()
            .push(user_id.to_string());

        // 分配标准角色
        state.user_roles.insert(
            user_id.to_string(),
            (tenant_id.to_string(), vec!["standard_user".to_string()]),
        );

        // 分配基础权限
        state.user_permissions.insert(
            (tenant_id.to_string(), user_id.to_string()),
            vec![
                "biz:create".to_string(),
                "biz:read".to_string(),
                "biz:update".to_string(),
                "biz:delete".to_string(),
                "biz:list".to_string(),
            ],
        );
    }

    /// 检查用户是否具有指定权限
    pub fn has_permission(&self, tenant_id: &str, user_id: &str, permission: &str) -> bool {
        let state = self.inner.lock().unwrap();
        state
            .user_permissions
            .get(&(tenant_id.to_string(), user_id.to_string()))
            .map(|perms| perms.iter().any(|p| p == permission))
            .unwrap_or(false)
    }

    /// 获取用户角色列表
    pub fn get_user_roles(&self, user_id: &str) -> Vec<String> {
        let state = self.inner.lock().unwrap();
        state
            .user_roles
            .get(user_id)
            .map(|(_, roles)| roles.clone())
            .unwrap_or_default()
    }

    /// 获取用户所属租户
    pub fn get_user_tenant(&self, user_id: &str) -> Option<String> {
        let state = self.inner.lock().unwrap();
        state.user_roles.get(user_id).map(|(tenant, _)| tenant.clone())
    }

    /// 为用户分配权限
    pub fn grant_permission(&self, tenant_id: &str, user_id: &str, permission: &str) {
        let mut state = self.inner.lock().unwrap();
        state
            .user_permissions
            .entry((tenant_id.to_string(), user_id.to_string()))
            .or_default()
            .push(permission.to_string());
    }
}

impl Default for InMemoryIamRepo {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_repo_init_and_get() {
        let meta = InMemoryMetaRepo::new();
        meta.init_common_industry("t1");
        let fields = meta.get_entity_fields("t1", "project");
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[0].field_code, "title");
    }

    #[test]
    fn test_iam_repo_permissions() {
        let iam = InMemoryIamRepo::new();
        iam.init_standard_user("t1", "u1");
        assert!(iam.has_permission("t1", "u1", "biz:create"));
        assert!(iam.has_permission("t1", "u1", "biz:read"));
        assert!(!iam.has_permission("t1", "u1", "admin:all"));
    }

    #[test]
    fn test_meta_repo_unknown_entity_returns_empty() {
        let meta = InMemoryMetaRepo::new();
        meta.init_common_industry("t1");
        let fields = meta.get_entity_fields("t1", "unknown");
        assert!(fields.is_empty());
    }
}
