// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumOption {
    pub code: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSpec {
    pub field_code: String,
    pub field_type: String,
    pub is_required: bool,
    pub is_indexed: bool,
    pub is_searchable: bool,
    pub is_sortable: bool,
    pub is_filterable: bool,
    pub options_inline: Option<Vec<EnumOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule_code: String,
    pub field_code: String,
    pub operator: String,
    pub expected: serde_json::Value,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityWithFields {
    pub entity_id: String,
    pub entity_code: String,
    pub fields: Vec<FieldSpec>,
    pub rules: Vec<ValidationRule>,
}

pub trait MetaRepository: Send + Sync {
    fn get_entity(
        &self,
        tenant_id: &str,
        entity_code_or_id: &str,
    ) -> anyhow::Result<EntityWithFields>;

    fn evaluate_rules(
        &self,
        entity: &EntityWithFields,
        data: &serde_json::Map<String, serde_json::Value>,
    ) -> anyhow::Result<()> {
        self.evaluate_rules_inner(entity, data, false)
    }

    fn evaluate_rules_inner(
        &self,
        entity: &EntityWithFields,
        data: &serde_json::Map<String, serde_json::Value>,
        skip_required: bool,
    ) -> anyhow::Result<()>;
}

pub struct InMemoryMetaRepo {
    entities: DashMap<(String, String), EntityWithFields>,
}

impl InMemoryMetaRepo {
    pub fn new() -> Self {
        Self {
            entities: DashMap::new(),
        }
    }

    pub fn register_entity(&self, tenant_id: &str, entity: EntityWithFields) {
        let key = (tenant_id.to_string(), entity.entity_code.clone());
        self.entities.insert(key, entity.clone());
        let key_id = (tenant_id.to_string(), entity.entity_id.clone());
        self.entities.insert(key_id, entity);
    }

    pub fn init_common_industry(&self, tenant_id: &str) {
        let project_fields = vec![
            FieldSpec {
                field_code: "title".to_string(),
                field_type: "string".to_string(),
                is_required: true,
                is_indexed: true,
                is_searchable: true,
                is_sortable: true,
                is_filterable: true,
                options_inline: None,
            },
            FieldSpec {
                field_code: "amount".to_string(),
                field_type: "decimal".to_string(),
                is_required: false,
                is_indexed: false,
                is_searchable: false,
                is_sortable: true,
                is_filterable: true,
                options_inline: None,
            },
            FieldSpec {
                field_code: "status".to_string(),
                field_type: "enum".to_string(),
                is_required: false,
                is_indexed: true,
                is_searchable: false,
                is_sortable: true,
                is_filterable: true,
                options_inline: Some(vec![
                    EnumOption {
                        code: "draft".into(),
                        label: "草稿".into(),
                    },
                    EnumOption {
                        code: "active".into(),
                        label: "进行中".into(),
                    },
                    EnumOption {
                        code: "done".into(),
                        label: "已完成".into(),
                    },
                ]),
            },
        ];
        self.register_entity(
            tenant_id,
            EntityWithFields {
                entity_id: Uuid::now_v7().to_string(),
                entity_code: "project".to_string(),
                fields: project_fields,
                rules: vec![],
            },
        );
    }
}

impl Default for InMemoryMetaRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaRepository for InMemoryMetaRepo {
    fn get_entity(
        &self,
        tenant_id: &str,
        entity_code_or_id: &str,
    ) -> anyhow::Result<EntityWithFields> {
        let key = (tenant_id.to_string(), entity_code_or_id.to_string());
        self.entities
            .get(&key)
            .map(|e| e.clone())
            .ok_or_else(|| anyhow::anyhow!("Entity not found: {}", entity_code_or_id))
    }

    fn evaluate_rules_inner(
        &self,
        entity: &EntityWithFields,
        data: &serde_json::Map<String, serde_json::Value>,
        skip_required: bool,
    ) -> anyhow::Result<()> {
        for field in &entity.fields {
            if !skip_required && field.is_required {
                let val = data.get(&field.field_code);
                let is_empty = match val {
                    None => true,
                    Some(serde_json::Value::Null) => true,
                    Some(serde_json::Value::String(s)) => s.is_empty(),
                    _ => false,
                };
                if is_empty {
                    anyhow::bail!("Required field missing: {}", field.field_code);
                }
            }
            if let Some(v) = data.get(&field.field_code) {
                match field.field_type.as_str() {
                    "int" | "integer" => {
                        if !v.is_i64() && !v.is_u64() {
                            anyhow::bail!("Field {} must be integer", field.field_code);
                        }
                    }
                    "decimal" | "number" | "float" => {
                        if !v.is_number() {
                            anyhow::bail!("Field {} must be number", field.field_code);
                        }
                    }
                    "bool" | "boolean" => {
                        if !v.is_boolean() {
                            anyhow::bail!("Field {} must be boolean", field.field_code);
                        }
                    }
                    _ => {}
                }
            }
        }
        for rule in &entity.rules {
            let actual = data.get(&rule.field_code);
            let matched = match rule.operator.as_str() {
                "gt" => actual.and_then(|v| v.as_f64()) > rule.expected.as_f64(),
                "gte" => actual.and_then(|v| v.as_f64()) >= rule.expected.as_f64(),
                "lt" => actual.and_then(|v| v.as_f64()) < rule.expected.as_f64(),
                "lte" => actual.and_then(|v| v.as_f64()) <= rule.expected.as_f64(),
                "eq" => actual == Some(&rule.expected),
                "in" => {
                    if let (Some(a), Some(arr)) = (actual, rule.expected.as_array()) {
                        arr.contains(a)
                    } else {
                        false
                    }
                }
                _ => true,
            };
            if !matched {
                anyhow::bail!("{}", rule.message);
            }
        }
        Ok(())
    }
}

// ================= IAM port =================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: String,
    pub tenant_id: String,
    pub username: String,
    pub dept_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub log_id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub action: String,
    pub target: String,
    pub detail: String,
    pub created_at: String,
}

pub trait IamRepository: Send + Sync {
    fn check_permission(
        &self,
        tenant_id: &str,
        user_id: &str,
        entity_code: &str,
        action: &str,
    ) -> anyhow::Result<()>;

    fn get_user(&self, user_id: &str) -> anyhow::Result<User>;

    fn write_audit_log(&self, entry: AuditLogEntry) -> anyhow::Result<()>;
}

pub struct InMemoryIamRepo {
    users: DashMap<String, User>,
    permissions: DashMap<(String, String, String, String), bool>,
    pub audit_logs: DashMap<String, AuditLogEntry>,
}

impl InMemoryIamRepo {
    pub fn new() -> Self {
        Self {
            users: DashMap::new(),
            permissions: DashMap::new(),
            audit_logs: DashMap::new(),
        }
    }
    pub fn add_user(&self, user: User) {
        self.users.insert(user.user_id.clone(), user);
    }
    pub fn grant_permission(
        &self,
        tenant_id: &str,
        user_id: &str,
        entity_code: &str,
        action: &str,
    ) {
        self.permissions.insert(
            (
                tenant_id.into(),
                user_id.into(),
                entity_code.into(),
                action.into(),
            ),
            true,
        );
    }
    pub fn init_standard_user(&self, tenant_id: &str, user_id: &str) {
        self.add_user(User {
            user_id: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            username: "test_user".into(),
            dept_id: "dept-001".into(),
        });
        for action in ["create", "read", "update", "delete", "list"] {
            self.grant_permission(tenant_id, user_id, "*", action);
        }
    }
}

impl Default for InMemoryIamRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl IamRepository for InMemoryIamRepo {
    fn check_permission(
        &self,
        tenant_id: &str,
        user_id: &str,
        entity_code: &str,
        action: &str,
    ) -> anyhow::Result<()> {
        let k_any = (tenant_id.into(), user_id.into(), "*".into(), action.into());
        if self.permissions.get(&k_any).is_some() {
            return Ok(());
        }
        let k = (
            tenant_id.into(),
            user_id.into(),
            entity_code.into(),
            action.into(),
        );
        if self.permissions.get(&k).is_some() {
            return Ok(());
        }
        anyhow::bail!(
            "Permission denied: user={} action={} entity={} tenant={}",
            user_id,
            action,
            entity_code,
            tenant_id
        )
    }
    fn get_user(&self, user_id: &str) -> anyhow::Result<User> {
        self.users
            .get(user_id)
            .map(|u| u.clone())
            .ok_or_else(|| anyhow::anyhow!("User not found: {}", user_id))
    }
    fn write_audit_log(&self, entry: AuditLogEntry) -> anyhow::Result<()> {
        self.audit_logs.insert(entry.log_id.clone(), entry);
        Ok(())
    }
}

fn mf_to_port_field(mf: &mox_platform_meta_core::MetaField) -> FieldSpec {
    let options_inline = mf.options_inline.as_ref().and_then(|s| {
        serde_json::from_str::<Vec<mox_platform_meta_core::EnumOption>>(s)
            .ok()
            .map(|v| {
                v.into_iter()
                    .map(|e| EnumOption {
                        code: e.value,
                        label: e.label,
                    })
                    .collect()
            })
    });
    FieldSpec {
        field_code: mf.field_code.clone(),
        field_type: mf.field_type.clone(),
        is_required: mf.is_required != 0,
        is_indexed: mf.is_indexed != 0,
        is_searchable: mf.is_searchable != 0,
        is_sortable: mf.is_sortable != 0,
        is_filterable: mf.is_filterable != 0,
        options_inline,
    }
}

impl MetaRepository for mox_platform_meta_core::MetaRepository {
    fn get_entity(
        &self,
        tenant_id: &str,
        entity_code_or_id: &str,
    ) -> anyhow::Result<EntityWithFields> {
        let opt = self.get_entity(tenant_id, entity_code_or_id)?;
        let me = opt.ok_or_else(|| anyhow::anyhow!("Entity not found: {}", entity_code_or_id))?;
        let port_fields: Vec<FieldSpec> = me.fields.iter().map(mf_to_port_field).collect();
        Ok(EntityWithFields {
            entity_id: me.entity.entity_id,
            entity_code: me.entity.entity_code,
            fields: port_fields,
            rules: vec![],
        })
    }

    fn evaluate_rules_inner(
        &self,
        entity: &EntityWithFields,
        data: &serde_json::Map<String, serde_json::Value>,
        skip_required: bool,
    ) -> anyhow::Result<()> {
        for field in &entity.fields {
            if !skip_required && field.is_required {
                let val = data.get(&field.field_code);
                let is_empty = match val {
                    None => true,
                    Some(serde_json::Value::Null) => true,
                    Some(serde_json::Value::String(s)) => s.is_empty(),
                    _ => false,
                };
                if is_empty {
                    anyhow::bail!("Required field missing: {}", field.field_code);
                }
            }
            if let Some(v) = data.get(&field.field_code) {
                match field.field_type.as_str() {
                    "int" | "integer" | "bigint" | "auto_increment" => {
                        if !v.is_i64() && !v.is_u64() {
                            anyhow::bail!("Field {} must be integer", field.field_code);
                        }
                    }
                    "decimal" | "number" | "float" | "double" | "percentage" | "money" => {
                        if !v.is_number() {
                            anyhow::bail!("Field {} must be number", field.field_code);
                        }
                    }
                    "bool" | "boolean" | "toggle" => {
                        if !v.is_boolean() {
                            anyhow::bail!("Field {} must be boolean", field.field_code);
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
