use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamTenant {
    pub tenant_id: String,
    pub tenant_code: String,
    pub tenant_name: String,
    pub tenant_mode: String,
    pub tenant_status: String,
    pub tenant_plan: String,
    pub config_json: Option<String>,
    pub settings: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamDepartment {
    pub dept_id: String,
    pub tenant_id: String,
    pub parent_id: Option<String>,
    pub dept_code: String,
    pub dept_name: String,
    pub dept_type: String,
    pub dept_level: i64,
    pub dept_path: String,
    pub sort_order: Option<i64>,
    pub manager_user_id: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamUser {
    pub user_id: String,
    pub tenant_id: String,
    pub user_code: String,
    pub username: String,
    pub password_hash: Option<String>,
    pub real_name: Option<String>,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub dept_id: Option<String>,
    pub position: Option<String>,
    pub user_status: String,
    pub is_superuser: i64,
    pub last_login_at: Option<String>,
    pub last_login_ip: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamRole {
    pub role_id: String,
    pub tenant_id: String,
    pub role_code: String,
    pub role_name: String,
    pub role_type: String,
    pub parent_id: Option<String>,
    pub inherit_path: Option<String>,
    pub is_builtin: i64,
    pub data_scope: String,
    pub description: Option<String>,
    pub sort_order: Option<i64>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamPermission {
    pub perm_id: String,
    pub tenant_id: String,
    pub perm_code: String,
    pub perm_name: String,
    pub resource_id: String,
    pub resource_type: String,
    pub perm_action: String,
    pub perm_category: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i64>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamUserRole {
    pub ur_id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub role_id: String,
    pub assigned_by: Option<String>,
    pub assigned_at: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamRolePermission {
    pub rp_id: String,
    pub tenant_id: String,
    pub role_id: String,
    pub perm_id: String,
    pub created_at: String,
    pub created_by: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamRoleInherit {
    pub ri_id: String,
    pub tenant_id: String,
    pub parent_role_id: String,
    pub child_role_id: String,
    pub inherit_level: i64,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamMenu {
    pub menu_id: String,
    pub tenant_id: String,
    pub parent_id: Option<String>,
    pub menu_code: String,
    pub menu_name: String,
    pub menu_type: String,
    pub menu_category: Option<String>,
    pub route_path: Option<String>,
    pub route_name: Option<String>,
    pub component_path: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i64>,
    pub is_visible: i64,
    pub is_cached: i64,
    pub is_external: i64,
    pub link_target: Option<String>,
    pub permission_code: Option<String>,
    pub api_scope: Option<String>,
    pub menu_config: Option<String>,
    pub children_json: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamUserMenu {
    pub um_id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub menu_id: String,
    pub is_favorite: i64,
    pub sort_order: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamRoleMenu {
    pub rm_id: String,
    pub tenant_id: String,
    pub role_id: String,
    pub menu_id: String,
    pub created_by: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamDataPermission {
    pub dp_id: String,
    pub tenant_id: String,
    pub dp_code: String,
    pub dp_name: String,
    pub subject_type: String,
    pub subject_id: Option<String>,
    pub subject_uuids_json: Option<String>,
    pub resource_code: String,
    pub scope_type: String,
    pub custom_rule_expression_sql: Option<String>,
    pub custom_rule_expression_json: Option<String>,
    pub status: String,
    pub created_at: String,
    pub created_by: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamResource {
    pub resource_id: String,
    pub tenant_id: String,
    pub resource_code: String,
    pub resource_name: String,
    pub resource_type: String,
    pub parent_id: Option<String>,
    pub resource_category: Option<String>,
    pub api_methods_sql: Option<String>,
    pub api_paths_sql: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i64>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IamTenantSetting {
    pub setting_id: String,
    pub tenant_id: String,
    pub setting_key: String,
    pub setting_value: Option<String>,
    pub setting_value_type: String,
    pub description: Option<String>,
    pub updated_by: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLog {
    pub log_id: String,
    pub tenant_id: String,
    pub trace_id: Option<String>,
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub user_ip: Option<String>,
    pub action: String,
    pub action_detail: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub resource_code: Option<String>,
    pub biz_id: Option<String>,
    pub biz_code: Option<String>,
    pub status_code: Option<i64>,
    pub http_method: Option<String>,
    pub http_path: Option<String>,
    pub latency_ms: Option<i64>,
    pub snapshot_before: Option<String>,
    pub snapshot_after: Option<String>,
    pub changed_fields: Option<String>,
    pub prev_hash: Option<String>,
    pub curr_hash: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScopeRule {
    pub scope_type: String,
    pub expression: Option<String>,
    pub dp_codes: Vec<String>,
}
