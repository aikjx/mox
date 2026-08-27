// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::model::*;
use anyhow::{Context, Result};
use chrono::Utc;
use parking_lot::Mutex;
use regex::Regex;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub static DDL_SQL: &str = include_str!("ddl.sql");

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn field_weight(f: &FieldSpec) -> u32 {
    let mut w: u32 = 0;
    if f.is_required {
        w += 8;
    }
    if f.is_indexed {
        w += 16;
    }
    if f.is_searchable {
        w += 4;
    }
    if f.is_filterable {
        w += 2;
    }
    if f.is_sortable {
        w += 1;
    }
    w
}

fn field_weight_def(f: &FieldDef) -> u32 {
    let mut w: u32 = 0;
    if f.required {
        w += 8;
    }
    if f.indexed {
        w += 16;
    }
    if f.searchable {
        w += 4;
    }
    if f.filterable {
        w += 2;
    }
    if f.sortable {
        w += 1;
    }
    w
}

fn field_type_to_string(t: &FieldType) -> String {
    match t {
        FieldType::String => "string",
        FieldType::Int => "integer",
        FieldType::Decimal => "decimal",
        FieldType::Boolean => "boolean",
        FieldType::DateTime => "datetime",
        FieldType::Enum => "enum",
        FieldType::Text => "text",
        FieldType::Json => "json",
    }
    .to_string()
}

fn is_string_type(t: &str) -> bool {
    matches!(
        t,
        "string"
            | "text"
            | "rich_text"
            | "html"
            | "markdown"
            | "keyword"
            | "phone"
            | "email"
            | "url"
            | "id_card"
            | "bank_card"
            | "domain"
            | "ip"
            | "avatar"
            | "signature"
            | "user"
            | "dept"
            | "tenant"
            | "location"
            | "address"
            | "enum"
            | "rating"
            | "stars"
            | "relation"
            | "lookup"
            | "reference"
    )
}

fn is_int_type(t: &str) -> bool {
    matches!(t, "integer" | "bigint" | "auto_increment")
}

fn is_decimal_type(t: &str) -> bool {
    matches!(t, "decimal" | "float" | "double" | "percentage" | "money")
}

fn is_date_type(t: &str) -> bool {
    matches!(
        t,
        "date" | "time" | "datetime" | "timestamp" | "timerange" | "daterange"
    )
}

fn is_bool_type(t: &str) -> bool {
    matches!(t, "boolean" | "toggle")
}

#[derive(Clone)]
pub struct MetaRepository {
    pub conn: Arc<Mutex<Connection>>,
}

impl MetaRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
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

    pub fn seed_industry(&self, _industries: &[&str]) -> Result<()> {
        let industries = [
            ("common", "通用基础包"),
            ("finance", "金融业务包"),
            ("medical", "医疗健康包"),
            ("manufacturing", "智能制造包"),
            ("government", "政务服务包"),
            ("education", "智慧教育包"),
            ("retail", "智慧零售包"),
        ];
        let ts = now_iso();
        let conn = self.conn.lock();
        for (code, name) in industries.iter() {
            let pkg = MetaIndustryPackage {
                package_id: new_id(),
                package_code: code.to_string(),
                package_name: name.to_string(),
                package_version: "1.0.0".to_string(),
                description: None,
                icon: None,
                banner: None,
                features: None,
                seed_entities: None,
                seed_workflows: None,
                seed_rules: None,
                compliance: None,
                is_official: 1,
                status: "active".to_string(),
                metadata: None,
                created_at: ts.clone(),
                updated_at: ts.clone(),
            };
            let _ = Self::insert_industry_package_inner(&conn, &pkg);
        }
        Ok(())
    }

    fn insert_industry_package_inner(conn: &Connection, p: &MetaIndustryPackage) -> Result<()> {
        conn.execute(
            "INSERT INTO meta_industry_package (package_id,package_code,package_name,package_version,description,icon,banner,features,seed_entities,seed_workflows,seed_rules,compliance,is_official,status,metadata,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
            params![
                p.package_id, p.package_code, p.package_name, p.package_version,
                p.description, p.icon, p.banner, p.features, p.seed_entities,
                p.seed_workflows, p.seed_rules, p.compliance, p.is_official,
                p.status, p.metadata, p.created_at, p.updated_at
            ],
        )?;
        Ok(())
    }

    fn insert_industry_package(&self, p: &MetaIndustryPackage) -> Result<()> {
        let conn = self.conn.lock();
        Self::insert_industry_package_inner(&conn, p)
    }

    pub fn define_entity(
        &self,
        tenant_id: Option<String>,
        entity_code: String,
        entity_name: String,
        fields: Vec<FieldDef>,
    ) -> Result<(String, HashMap<String, String>)> {
        let tenant_id = tenant_id.unwrap_or_else(|| "default".to_string());
        let ts = now_iso();
        let entity_id = new_id();

        let entity = MetaEntity {
            entity_id: entity_id.clone(),
            tenant_id: tenant_id.clone(),
            entity_code: entity_code.clone(),
            entity_name: entity_name.clone(),
            entity_plural: None,
            table_name: None,
            description: None,
            icon: None,
            color: None,
            entity_category: "master".to_string(),
            storage_mode: "universal".to_string(),
            shard_key: None,
            history_strategy: "snapshot".to_string(),
            extends_entity_id: None,
            mixin_ids: None,
            tags: None,
            list_view_id: None,
            form_view_id: None,
            detail_view_id: None,
            workflow_id: None,
            is_system: 0,
            status: "active".to_string(),
            metadata: None,
            created_at: ts.clone(),
            updated_at: ts.clone(),
            created_by: None,
            updated_by: None,
            deleted_at: None,
            deleted_by: None,
            version: 1,
            _hash: None,
        };

        let field_specs: Vec<FieldSpec> = fields
            .iter()
            .map(|f| FieldSpec {
                field_code: f.code.clone(),
                field_name: f.name.clone(),
                field_type: field_type_to_string(&f.r#type),
                is_required: f.required,
                is_indexed: f.indexed,
                is_searchable: f.searchable,
                is_sortable: f.sortable,
                is_filterable: f.filterable,
                description: None,
            })
            .collect();

        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO meta_entity (entity_id,tenant_id,entity_code,entity_name,entity_plural,table_name,description,icon,color,entity_category,storage_mode,shard_key,history_strategy,extends_entity_id,mixin_ids,tags,list_view_id,form_view_id,detail_view_id,workflow_id,is_system,status,metadata,created_at,updated_at,created_by,updated_by,deleted_at,deleted_by,version,_hash) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31)",
            params![
                entity.entity_id, entity.tenant_id, entity.entity_code, entity.entity_name,
                entity.entity_plural, entity.table_name, entity.description, entity.icon,
                entity.color, entity.entity_category, entity.storage_mode, entity.shard_key,
                entity.history_strategy, entity.extends_entity_id, entity.mixin_ids, entity.tags,
                entity.list_view_id, entity.form_view_id, entity.detail_view_id, entity.workflow_id,
                entity.is_system, entity.status, entity.metadata, entity.created_at,
                entity.updated_at, entity.created_by, entity.updated_by, entity.deleted_at,
                entity.deleted_by, entity.version, entity._hash
            ],
        )?;

        let mut indexed: Vec<(usize, &FieldDef)> = fields.iter().enumerate().collect();
        indexed.sort_by(|a, b| {
            let wb = field_weight_def(b.1);
            let wa = field_weight_def(a.1);
            wb.cmp(&wa).then(a.0.cmp(&b.0))
        });

        let mut str_slots: Vec<String> = (1..=12).map(|i| format!("ext_str_{:02}", i)).collect();
        let mut int_slots: Vec<String> = (1..=5).map(|i| format!("ext_int_{:02}", i)).collect();
        let mut dec_slots: Vec<String> = (1..=5).map(|i| format!("ext_dec_{:02}", i)).collect();
        let mut date_slots: Vec<String> = (1..=3).map(|i| format!("ext_date_{:02}", i)).collect();

        let mut slot_map: HashMap<String, String> = HashMap::new();
        let mut ordered_fields: Vec<(usize, FieldDef)> = fields
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, f)| (i, f))
            .collect();
        ordered_fields.sort_by(|a, b| {
            let wb = field_weight_def(&b.1);
            let wa = field_weight_def(&a.1);
            wb.cmp(&wa).then(a.0.cmp(&b.0))
        });

        for (_, f) in ordered_fields.iter() {
            let ft = field_type_to_string(&f.r#type);
            let slot = if is_string_type(&ft) {
                str_slots.pop()
            } else if is_int_type(&ft) || is_bool_type(&ft) {
                int_slots.pop()
            } else if is_decimal_type(&ft) {
                dec_slots.pop()
            } else if is_date_type(&ft) {
                date_slots.pop()
            } else {
                None
            };
            if let Some(s) = slot {
                slot_map.insert(f.code.clone(), s);
            } else {
                slot_map.insert(f.code.clone(), "json_data".to_string());
            }
        }

        for (idx, f) in fields.iter().enumerate() {
            let ft = field_type_to_string(&f.r#type);
            let slot = slot_map
                .get(&f.code)
                .cloned()
                .unwrap_or_else(|| "json_data".to_string());
            let options_inline_str = f
                .options_inline
                .as_ref()
                .and_then(|opts| serde_json::to_string(opts).ok());
            let mf = MetaField {
                field_id: new_id(),
                tenant_id: tenant_id.clone(),
                entity_id: entity_id.clone(),
                field_code: f.code.clone(),
                field_name: f.name.clone(),
                field_type: ft,
                is_required: if f.required { 1 } else { 0 },
                is_unique: 0,
                is_indexed: if f.indexed { 1 } else { 0 },
                is_searchable: if f.searchable { 1 } else { 0 },
                is_sortable: if f.sortable { 1 } else { 0 },
                is_filterable: if f.filterable { 1 } else { 0 },
                is_exportable: 1,
                is_importable: 1,
                is_readonly: 0,
                is_hidden: 0,
                is_system: 0,
                default_value: None,
                default_expr: None,
                auto_fill_on: None,
                max_length: None,
                min_value: None,
                max_value: None,
                decimal_places: None,
                step: None,
                currency_code: None,
                unit: None,
                options_source: "inline".to_string(),
                options_inline: options_inline_str,
                options_sql: None,
                options_api: None,
                options_dict_code: None,
                relation_config: None,
                validations: None,
                formula_expr: None,
                formula_deps: None,
                ui_component: f.ui_component.clone(),
                ui_props: None,
                ui_placeholder: None,
                ui_hint: None,
                ui_group: None,
                ui_sort_order: idx as i64,
                ui_span: 24,
                ui_newline: 0,
                ui_dynamic_cond: None,
                field_permission: None,
                storage_slot: Some(slot),
                description: None,
                tags: None,
                status: "active".to_string(),
                metadata: None,
                created_at: ts.clone(),
                updated_at: ts.clone(),
                created_by: None,
                updated_by: None,
                deleted_at: None,
                deleted_by: None,
                version: 1,
                _hash: None,
            };
            conn.execute(
                "INSERT INTO meta_field (field_id,tenant_id,entity_id,field_code,field_name,field_type,is_required,is_unique,is_indexed,is_searchable,is_sortable,is_filterable,is_exportable,is_importable,is_readonly,is_hidden,is_system,default_value,default_expr,auto_fill_on,max_length,min_value,max_value,decimal_places,step,currency_code,unit,options_source,options_inline,options_sql,options_api,options_dict_code,relation_config,validations,formula_expr,formula_deps,ui_component,ui_props,ui_placeholder,ui_hint,ui_group,ui_sort_order,ui_span,ui_newline,ui_dynamic_cond,field_permission,storage_slot,description,tags,status,metadata,created_at,updated_at,created_by,updated_by,deleted_at,deleted_by,version,_hash) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,?45,?46,?47,?48,?49,?50,?51,?52,?53,?54,?55,?56,?57,?58,?59)",
                params![
                    mf.field_id, mf.tenant_id, mf.entity_id, mf.field_code, mf.field_name,
                    mf.field_type, mf.is_required, mf.is_unique, mf.is_indexed, mf.is_searchable,
                    mf.is_sortable, mf.is_filterable, mf.is_exportable, mf.is_importable,
                    mf.is_readonly, mf.is_hidden, mf.is_system, mf.default_value, mf.default_expr,
                    mf.auto_fill_on, mf.max_length, mf.min_value, mf.max_value, mf.decimal_places,
                    mf.step, mf.currency_code, mf.unit, mf.options_source, mf.options_inline,
                    mf.options_sql, mf.options_api, mf.options_dict_code, mf.relation_config,
                    mf.validations, mf.formula_expr, mf.formula_deps, mf.ui_component, mf.ui_props,
                    mf.ui_placeholder, mf.ui_hint, mf.ui_group, mf.ui_sort_order, mf.ui_span,
                    mf.ui_newline, mf.ui_dynamic_cond, mf.field_permission, mf.storage_slot,
                    mf.description, mf.tags, mf.status, mf.metadata, mf.created_at, mf.updated_at,
                    mf.created_by, mf.updated_by, mf.deleted_at, mf.deleted_by, mf.version, mf._hash
                ],
            )?;
        }
        drop(conn);
        drop(field_specs);

        Ok((entity_id, slot_map))
    }

    pub fn get_entity(
        &self,
        tenant_id: &str,
        entity_code: &str,
    ) -> Result<Option<EntityWithFields>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT entity_id,tenant_id,entity_code,entity_name,entity_plural,table_name,description,icon,color,entity_category,storage_mode,shard_key,history_strategy,extends_entity_id,mixin_ids,tags,list_view_id,form_view_id,detail_view_id,workflow_id,is_system,status,metadata,created_at,updated_at,created_by,updated_by,deleted_at,deleted_by,version,_hash FROM meta_entity WHERE tenant_id=?1 AND entity_code=?2"
        )?;
        let mut rows = stmt.query(params![tenant_id, entity_code])?;
        let entity_row = match rows.next()? {
            None => return Ok(None),
            Some(r) => MetaEntity {
                entity_id: r.get(0)?,
                tenant_id: r.get(1)?,
                entity_code: r.get(2)?,
                entity_name: r.get(3)?,
                entity_plural: r.get(4)?,
                table_name: r.get(5)?,
                description: r.get(6)?,
                icon: r.get(7)?,
                color: r.get(8)?,
                entity_category: r.get(9)?,
                storage_mode: r.get(10)?,
                shard_key: r.get(11)?,
                history_strategy: r.get(12)?,
                extends_entity_id: r.get(13)?,
                mixin_ids: r.get(14)?,
                tags: r.get(15)?,
                list_view_id: r.get(16)?,
                form_view_id: r.get(17)?,
                detail_view_id: r.get(18)?,
                workflow_id: r.get(19)?,
                is_system: r.get(20)?,
                status: r.get(21)?,
                metadata: r.get(22)?,
                created_at: r.get(23)?,
                updated_at: r.get(24)?,
                created_by: r.get(25)?,
                updated_by: r.get(26)?,
                deleted_at: r.get(27)?,
                deleted_by: r.get(28)?,
                version: r.get(29)?,
                _hash: r.get(30)?,
            },
        };
        drop(rows);
        drop(stmt);

        let mut stmt_fields = conn.prepare(
            "SELECT field_id,tenant_id,entity_id,field_code,field_name,field_type,is_required,is_unique,is_indexed,is_searchable,is_sortable,is_filterable,is_exportable,is_importable,is_readonly,is_hidden,is_system,default_value,default_expr,auto_fill_on,max_length,min_value,max_value,decimal_places,step,currency_code,unit,options_source,options_inline,options_sql,options_api,options_dict_code,relation_config,validations,formula_expr,formula_deps,ui_component,ui_props,ui_placeholder,ui_hint,ui_group,ui_sort_order,ui_span,ui_newline,ui_dynamic_cond,field_permission,storage_slot,description,tags,status,metadata,created_at,updated_at,created_by,updated_by,deleted_at,deleted_by,version,_hash FROM meta_field WHERE tenant_id=?1 AND entity_id=?2 AND status='active' ORDER BY ui_sort_order ASC"
        )?;
        let fields: Vec<MetaField> = stmt_fields
            .query_map(params![tenant_id, entity_row.entity_id], |r| {
                Ok(MetaField {
                    field_id: r.get(0)?,
                    tenant_id: r.get(1)?,
                    entity_id: r.get(2)?,
                    field_code: r.get(3)?,
                    field_name: r.get(4)?,
                    field_type: r.get(5)?,
                    is_required: r.get(6)?,
                    is_unique: r.get(7)?,
                    is_indexed: r.get(8)?,
                    is_searchable: r.get(9)?,
                    is_sortable: r.get(10)?,
                    is_filterable: r.get(11)?,
                    is_exportable: r.get(12)?,
                    is_importable: r.get(13)?,
                    is_readonly: r.get(14)?,
                    is_hidden: r.get(15)?,
                    is_system: r.get(16)?,
                    default_value: r.get(17)?,
                    default_expr: r.get(18)?,
                    auto_fill_on: r.get(19)?,
                    max_length: r.get(20)?,
                    min_value: r.get(21)?,
                    max_value: r.get(22)?,
                    decimal_places: r.get(23)?,
                    step: r.get(24)?,
                    currency_code: r.get(25)?,
                    unit: r.get(26)?,
                    options_source: r.get(27)?,
                    options_inline: r.get(28)?,
                    options_sql: r.get(29)?,
                    options_api: r.get(30)?,
                    options_dict_code: r.get(31)?,
                    relation_config: r.get(32)?,
                    validations: r.get(33)?,
                    formula_expr: r.get(34)?,
                    formula_deps: r.get(35)?,
                    ui_component: r.get(36)?,
                    ui_props: r.get(37)?,
                    ui_placeholder: r.get(38)?,
                    ui_hint: r.get(39)?,
                    ui_group: r.get(40)?,
                    ui_sort_order: r.get(41)?,
                    ui_span: r.get(42)?,
                    ui_newline: r.get(43)?,
                    ui_dynamic_cond: r.get(44)?,
                    field_permission: r.get(45)?,
                    storage_slot: r.get(46)?,
                    description: r.get(47)?,
                    tags: r.get(48)?,
                    status: r.get(49)?,
                    metadata: r.get(50)?,
                    created_at: r.get(51)?,
                    updated_at: r.get(52)?,
                    created_by: r.get(53)?,
                    updated_by: r.get(54)?,
                    deleted_at: r.get(55)?,
                    deleted_by: r.get(56)?,
                    version: r.get(57)?,
                    _hash: r.get(58)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut slot_map: HashMap<String, String> = HashMap::new();
        for f in fields.iter() {
            if let Some(slot) = &f.storage_slot {
                slot_map.insert(f.field_code.clone(), slot.clone());
            }
        }

        Ok(Some(EntityWithFields {
            entity: entity_row,
            fields,
            slot_map,
        }))
    }

    pub fn define_workflow(
        &self,
        tenant_id: &str,
        workflow_code: &str,
        workflow_name: &str,
        process_def: &str,
        entity_id: Option<&str>,
    ) -> Result<String> {
        let ts = now_iso();
        let wid = new_id();
        let wf = MetaWorkflow {
            workflow_id: wid.clone(),
            tenant_id: tenant_id.to_string(),
            workflow_code: workflow_code.to_string(),
            workflow_name: workflow_name.to_string(),
            workflow_category: None,
            description: None,
            icon: None,
            entity_id: entity_id.map(|s| s.to_string()),
            trigger_events: None,
            trigger_condition: None,
            workflow_version: 1,
            version_tag: Some("v1.0".to_string()),
            is_main_version: 1,
            process_def: process_def.to_string(),
            notification: None,
            start_roles: None,
            admin_roles: None,
            viewer_roles: None,
            is_draft: 0,
            is_suspended: 0,
            status: "active".to_string(),
            metadata: None,
            created_at: ts.clone(),
            updated_at: ts,
            created_by: None,
            updated_by: None,
            deleted_at: None,
            deleted_by: None,
            version: 1,
            _hash: None,
        };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO meta_workflow (workflow_id,tenant_id,workflow_code,workflow_name,workflow_category,description,icon,entity_id,trigger_events,trigger_condition,workflow_version,version_tag,is_main_version,process_def,notification,start_roles,admin_roles,viewer_roles,is_draft,is_suspended,status,metadata,created_at,updated_at,created_by,updated_by,deleted_at,deleted_by,version,_hash) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30)",
            params![
                wf.workflow_id, wf.tenant_id, wf.workflow_code, wf.workflow_name,
                wf.workflow_category, wf.description, wf.icon, wf.entity_id, wf.trigger_events,
                wf.trigger_condition, wf.workflow_version, wf.version_tag, wf.is_main_version,
                wf.process_def, wf.notification, wf.start_roles, wf.admin_roles, wf.viewer_roles,
                wf.is_draft, wf.is_suspended, wf.status, wf.metadata, wf.created_at, wf.updated_at,
                wf.created_by, wf.updated_by, wf.deleted_at, wf.deleted_by, wf.version, wf._hash
            ],
        )?;
        Ok(wid)
    }

    pub fn trigger_workflow(
        &self,
        tenant_id: &str,
        workflow_id: &str,
        initiator_id: &str,
        biz_id: Option<&str>,
        form_data: Option<&str>,
    ) -> Result<String> {
        let ts = now_iso();
        let inst_id = new_id();
        let inst = MetaWorkflowInstance {
            wfi_id: inst_id.clone(),
            tenant_id: tenant_id.to_string(),
            workflow_id: workflow_id.to_string(),
            workflow_version: Some(1),
            entity_id: None,
            biz_id: biz_id.map(|s| s.to_string()),
            biz_code: None,
            biz_title: None,
            instance_status: "running".to_string(),
            current_node_id: None,
            current_task_ids: None,
            initiator_id: initiator_id.to_string(),
            initiator_dept_id: None,
            admin_user_ids: None,
            cc_user_ids: None,
            started_at: ts.clone(),
            ended_at: None,
            due_at: None,
            suspended_at: None,
            last_active_at: Some(ts.clone()),
            total_duration_ms: None,
            form_data: form_data.map(|s| s.to_string()),
            variables: None,
            context: None,
            final_decision: None,
            final_comment: None,
            completed_count: None,
            rejected_count: None,
            metadata: None,
            created_at: ts.clone(),
            updated_at: ts,
        };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO meta_workflow_instance (wfi_id,tenant_id,workflow_id,workflow_version,entity_id,biz_id,biz_code,biz_title,instance_status,current_node_id,current_task_ids,initiator_id,initiator_dept_id,admin_user_ids,cc_user_ids,started_at,ended_at,due_at,suspended_at,last_active_at,total_duration_ms,form_data,variables,context,final_decision,final_comment,completed_count,rejected_count,metadata,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31)",
            params![
                inst.wfi_id, inst.tenant_id, inst.workflow_id, inst.workflow_version,
                inst.entity_id, inst.biz_id, inst.biz_code, inst.biz_title, inst.instance_status,
                inst.current_node_id, inst.current_task_ids, inst.initiator_id, inst.initiator_dept_id,
                inst.admin_user_ids, inst.cc_user_ids, inst.started_at, inst.ended_at, inst.due_at,
                inst.suspended_at, inst.last_active_at, inst.total_duration_ms, inst.form_data,
                inst.variables, inst.context, inst.final_decision, inst.final_comment,
                inst.completed_count, inst.rejected_count, inst.metadata, inst.created_at,
                inst.updated_at
            ],
        )?;
        Ok(inst_id)
    }

    pub fn evaluate_rules(
        &self,
        tenant_id: &str,
        entity_code: &str,
        fields_data: &HashMap<String, serde_json::Value>,
        action: &str,
    ) -> Result<RuleResult> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT rule_code, rule_category, rule_body, trigger_condition FROM meta_rule r \
             JOIN meta_entity e ON r.entity_id = e.entity_id \
             WHERE r.tenant_id=?1 AND e.entity_code=?2 AND r.is_enabled=1 AND r.status='active'",
        )?;
        let rows = stmt.query_map(params![tenant_id, entity_code], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })?;
        let rules: Vec<_> = rows.collect::<std::result::Result<Vec<_>, _>>()?;

        let mut result = RuleResult {
            passed: true,
            errors: vec![],
            warnings: vec![],
            computed_fields: HashMap::new(),
        };

        let action_re = Regex::new(r#""action"\s*:\s*"([^"]+)""#).ok();

        for (code, category, body, _cond) in rules.iter() {
            let matches_action = if let Some(re) = &action_re {
                re.is_match(body) || body.contains(action)
            } else {
                true
            };
            if !matches_action {
                continue;
            }
            if category == "validation" {
                if let Ok(body_json) = serde_json::from_str::<serde_json::Value>(body) {
                    if let Some(checks) = body_json.get("checks").and_then(|c| c.as_array()) {
                        for check in checks {
                            if let Some(field) = check.get("field").and_then(|f| f.as_str()) {
                                if check.get("type").and_then(|t| t.as_str()) == Some("required") {
                                    let val = fields_data.get(field);
                                    let empty = match val {
                                        None => true,
                                        Some(v) if v.is_null() => true,
                                        Some(serde_json::Value::String(s)) => s.trim().is_empty(),
                                        _ => false,
                                    };
                                    if empty {
                                        result.passed = false;
                                        result
                                            .errors
                                            .push(format!("{}: 字段 {} 必填", code, field));
                                    }
                                }
                            }
                        }
                    }
                }
            } else if category == "calculation" {
                if let Ok(body_json) = serde_json::from_str::<serde_json::Value>(body) {
                    if let Some(target) = body_json.get("target").and_then(|t| t.as_str()) {
                        if let Some(expr) = body_json.get("expr").and_then(|e| e.as_str()) {
                            let expr_trim = expr.replace(' ', "");
                            if let Ok(re) = Regex::new(r"\{\{\s*(\w+)\s*\}\}") {
                                let mut resolved = expr_trim.clone();
                                for cap in re.captures_iter(&expr_trim) {
                                    if let Some(var) = cap.get(1) {
                                        let vname = var.as_str();
                                        let substitute = if let Some(fv) = fields_data.get(vname) {
                                            if let Some(n) = fv.as_f64() {
                                                format!("{}", n)
                                            } else if let Some(s) = fv.as_str() {
                                                format!("\"{}\"", s.replace('\"', "\\\""))
                                            } else {
                                                "null".to_string()
                                            }
                                        } else {
                                            "0".to_string()
                                        };
                                        resolved = resolved.replace(&cap[0], &substitute);
                                    }
                                }
                                if let Ok(r2) =
                                    Regex::new(r"^(\d+(\.\d+)?)\s*([+\-*/])\s*(\d+(\.\d+)?)$")
                                {
                                    if let Some(caps) = r2.captures(&resolved) {
                                        if let (Ok(a), Ok(b)) =
                                            (caps[1].parse::<f64>(), caps[4].parse::<f64>())
                                        {
                                            let op = &caps[3];
                                            let computed = match op {
                                                "+" => a + b,
                                                "-" => a - b,
                                                "*" => a * b,
                                                "/" if b != 0.0 => a / b,
                                                _ => 0.0,
                                            };
                                            result.computed_fields.insert(
                                                target.to_string(),
                                                serde_json::json!(computed),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}
