// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! meta 模块 `codegen` 能力的真实实现（module-registry.yml capabilities 中的 codegen）。
//!
//! 输入 [`EntityWithFields`](crate::model::EntityWithFields) 元数据，
//! 按模板（TPL-01 单表 CRUD 等）确定性地产出成型项目工件：
//! DDL / Rust 模型 / 前端 API / 列表页 / 表单页 / 路由 / 菜单。
//!
//! 设计约束（见 docs/normalization/TPL-INDEX.md §2~§4）：
//! - 纯函数、零 I/O（L2 Core 分层），产物字节级可复现（同输入同输出）；
//! - 生成器自身不得产出 `todo!()` 骨架；
//! - 新增模板 = 新增 `tpl_*` 模块 + 在 [`generate`] 分派 match 追加 1 个 arm。

pub mod naming;
pub mod tpl_ai;
pub mod tpl_crud;
pub mod tpl_graph;
pub mod tpl_master_detail;
pub mod tpl_tree;
pub mod tpl_workflow;

use std::collections::BTreeMap;

use crate::model::EntityWithFields;

/// 支持的生成模板（对齐 TPL-INDEX §1 目录；新增一类 = 追加 1 个 variant + 1 个 arm）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodegenTemplate {
    /// TPL-01 单表 CRUD：列表 + 表单 + API + DDL + 路由菜单。
    Crud,
    /// TPL-02 树表：parent_id 层级 + 树页面 + 拖拽移动。
    Tree,
    /// TPL-04 图谱实体：节点/边双表 + 3D 画布 + graph.mmd。
    Graph,
    /// TPL-05 工作流：workflow.json DAG 定义 + TS Runner。
    Workflow,
    /// TPL-06 AI 对话域：Chat 页面 + 会话 API。
    AiChat,
}

impl CodegenTemplate {
    /// 模板编号（与 TPL-INDEX 目录一一对应）。
    #[must_use]
    pub fn tpl_code(self) -> &'static str {
        match self {
            CodegenTemplate::Crud => "TPL-01",
            CodegenTemplate::Tree => "TPL-02",
            CodegenTemplate::Graph => "TPL-04",
            CodegenTemplate::Workflow => "TPL-05",
            CodegenTemplate::AiChat => "TPL-06",
        }
    }
}

/// codegen 领域错误。
#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    /// 实体元数据不合法（空编码、空字段集等）。
    #[error("invalid entity metadata: {0}")]
    InvalidEntity(String),
    /// 字段元数据不合法（类型未知、编码重复等）。
    #[error("invalid field metadata: {0}")]
    InvalidField(String),
}

/// 一次生成的产物集合：相对路径 → 文件内容（BTreeMap 保证遍历次序稳定）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodegenOutput {
    /// 实体编码（生成目标）。
    pub entity_code: String,
    /// 所用模板编号（如 "TPL-01"）。
    pub template: String,
    /// 产物：相对路径 → 文件全文。
    pub artifacts: BTreeMap<String, String>,
}

impl CodegenOutput {
    /// 按平台模板市场 `artifacts`（BTreeMap<String,String>）的约定直接产出代码包。
    #[must_use]
    pub fn into_artifacts(self) -> BTreeMap<String, String> {
        self.artifacts
    }
}

/// 归一化入口：元数据 → 模板分派 → 确定性产物。
///
/// # Errors
/// 实体/字段元数据不合法时返回 [`CodegenError`]。
pub fn generate(entity: &EntityWithFields, template: CodegenTemplate) -> Result<CodegenOutput, CodegenError> {
    validate(entity)?;
    match template {
        CodegenTemplate::Crud => tpl_crud::generate_crud(entity, template.tpl_code()),
        CodegenTemplate::Tree => tpl_tree::generate_tree(entity, template.tpl_code()),
        CodegenTemplate::Graph => tpl_graph::generate_graph(entity, template.tpl_code()),
        CodegenTemplate::Workflow => tpl_workflow::generate_workflow(entity, template.tpl_code()),
        CodegenTemplate::AiChat => tpl_ai::generate_ai_chat(entity, template.tpl_code()),
    }
}

/// TPL-03 主子表入口：主实体 + 明细实体（两份元数据，明细自动加 `master_id`）。
///
/// # Errors
/// 主/明细任一元数据不合法，或两者 entity_code 相同。
pub fn generate_master_detail(
    master: &EntityWithFields,
    detail: &EntityWithFields,
) -> Result<CodegenOutput, CodegenError> {
    validate(master)?;
    validate(detail)?;
    tpl_master_detail::generate_master_detail(master, detail, "TPL-03")
}

/// 模板编号解析（单一来源；大小写不敏感，支持别名）。TPL-03 由 [`generate_master_detail`] 承接，
/// 此处不做 TPL-03 分派（它需要两份实体元数据）。
///
/// # Errors
/// 不支持的模板编号返回 Err(说明)。
pub fn parse_template(raw: &str) -> Result<CodegenTemplate, String> {
    match raw.trim().to_uppercase().as_str() {
        "" | "TPL-01" | "CRUD" => Ok(CodegenTemplate::Crud),
        "TPL-02" | "TREE" => Ok(CodegenTemplate::Tree),
        "TPL-04" | "GRAPH" => Ok(CodegenTemplate::Graph),
        "TPL-05" | "WORKFLOW" => Ok(CodegenTemplate::Workflow),
        "TPL-06" | "AI_CHAT" => Ok(CodegenTemplate::AiChat),
        other => Err(format!(
            "unsupported template `{other}` (supported: TPL-01/02/03/04/05/06)"
        )),
    }
}

/// 从字段定义（`FieldDef`，即 `/entities/define` 写入侧类型）构建无仓储的 `EntityWithFields` 视图，
/// 供网关/闸门链等无 DB 调用方直接跑 codegen（字段按定义序排列，storage_slot 置空）。
#[must_use]
pub fn build_entity_view(
    tenant_id: &str,
    entity_code: &str,
    entity_name: &str,
    fields: &[crate::model::FieldDef],
) -> EntityWithFields {
    let ts = "1970-01-01T00:00:00Z";
    let meta_fields = fields
        .iter()
        .enumerate()
        .map(|(i, fd)| crate::model::MetaField {
            field_id: format!("view_{entity_code}_{:02}", i + 1),
            tenant_id: tenant_id.to_string(),
            entity_id: format!("view_{entity_code}"),
            field_code: fd.code.clone(),
            field_name: fd.name.clone(),
            field_type: naming::field_type_to_string(&fd.r#type),
            is_required: i64::from(fd.required),
            is_unique: 0,
            is_indexed: i64::from(fd.indexed),
            is_searchable: i64::from(fd.searchable),
            is_sortable: i64::from(fd.sortable),
            is_filterable: i64::from(fd.filterable),
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
            options_source: if fd.options_inline.is_some() { "inline" } else { "none" }.into(),
            options_inline: fd.options_inline.as_ref().map(|opts| {
                serde_json::to_string(opts).unwrap_or_default()
            }),
            options_sql: None,
            options_api: None,
            options_dict_code: None,
            relation_config: None,
            validations: None,
            formula_expr: None,
            formula_deps: None,
            ui_component: fd.ui_component.clone(),
            ui_props: None,
            ui_placeholder: None,
            ui_hint: None,
            ui_group: None,
            ui_sort_order: i64::try_from(i).unwrap_or(0),
            ui_span: 1,
            ui_newline: 0,
            ui_dynamic_cond: None,
            field_permission: None,
            storage_slot: None,
            description: None,
            tags: None,
            status: "active".into(),
            metadata: None,
            created_at: ts.into(),
            updated_at: ts.into(),
            created_by: None,
            updated_by: None,
            deleted_at: None,
            deleted_by: None,
            version: 1,
            _hash: None,
        })
        .collect();
    let entity = crate::model::MetaEntity {
        entity_id: format!("view_{entity_code}"),
        tenant_id: tenant_id.to_string(),
        entity_code: entity_code.to_string(),
        entity_name: entity_name.to_string(),
        entity_plural: None,
        table_name: None,
        description: None,
        icon: None,
        color: None,
        entity_category: "business".into(),
        storage_mode: "inline_view".into(),
        shard_key: None,
        history_strategy: "none".into(),
        extends_entity_id: None,
        mixin_ids: None,
        tags: None,
        list_view_id: None,
        form_view_id: None,
        detail_view_id: None,
        workflow_id: None,
        is_system: 0,
        status: "active".into(),
        metadata: None,
        created_at: ts.into(),
        updated_at: ts.into(),
        created_by: None,
        updated_by: None,
        deleted_at: None,
        deleted_by: None,
        version: 1,
        _hash: None,
    };
    EntityWithFields {
        entity,
        fields: meta_fields,
        slot_map: Default::default(),
    }
}

fn validate(entity: &EntityWithFields) -> Result<(), CodegenError> {
    let code = entity.entity.entity_code.trim();
    if code.is_empty() {
        return Err(CodegenError::InvalidEntity("entity_code is empty".into()));
    }
    if entity.fields.is_empty() {
        return Err(CodegenError::InvalidEntity(format!(
            "entity `{code}` has no fields"
        )));
    }
    let mut seen = std::collections::BTreeSet::new();
    for f in &entity.fields {
        let fc = f.field_code.trim();
        if fc.is_empty() {
            return Err(CodegenError::InvalidField(format!(
                "entity `{code}` has a field with empty field_code"
            )));
        }
        if !naming::is_valid_ident(fc) {
            return Err(CodegenError::InvalidField(format!(
                "entity `{code}` field_code `{fc}` is not a snake_case identifier"
            )));
        }
        if !naming::is_known_field_type(&f.field_type) {
            return Err(CodegenError::InvalidField(format!(
                "entity `{code}` field `{fc}` has unknown field_type `{}`",
                f.field_type
            )));
        }
        if !seen.insert(fc.to_string()) {
            return Err(CodegenError::InvalidField(format!(
                "entity `{code}` has duplicate field_code `{fc}`"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EntityWithFields, FieldDef, FieldType, MetaEntity, MetaField};

    fn meta_entity(code: &str) -> MetaEntity {
        MetaEntity {
            entity_id: format!("ent_{code}"),
            tenant_id: "t001".into(),
            entity_code: code.into(),
            entity_name: "项目".into(),
            entity_plural: None,
            table_name: None,
            description: None,
            icon: Some("folder".into()),
            color: None,
            entity_category: "business".into(),
            storage_mode: "ext_slot".into(),
            shard_key: None,
            history_strategy: "none".into(),
            extends_entity_id: None,
            mixin_ids: None,
            tags: None,
            list_view_id: None,
            form_view_id: None,
            detail_view_id: None,
            workflow_id: None,
            is_system: 0,
            status: "active".into(),
            metadata: None,
            created_at: "2026-09-04T00:00:00Z".into(),
            updated_at: "2026-09-04T00:00:00Z".into(),
            created_by: None,
            updated_by: None,
            deleted_at: None,
            deleted_by: None,
            version: 1,
            _hash: None,
        }
    }

    fn meta_field(code: &str, name: &str, ftype: &str, slot: Option<&str>) -> MetaField {
        let mut f = MetaField {
            field_id: format!("fld_{code}"),
            tenant_id: "t001".into(),
            entity_id: "ent_project".into(),
            field_code: code.into(),
            field_name: name.into(),
            field_type: ftype.into(),
            is_required: 1,
            is_unique: 0,
            is_indexed: 1,
            is_searchable: 1,
            is_sortable: 1,
            is_filterable: 1,
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
            options_source: "none".into(),
            options_inline: None,
            options_sql: None,
            options_api: None,
            options_dict_code: None,
            relation_config: None,
            validations: None,
            formula_expr: None,
            formula_deps: None,
            ui_component: None,
            ui_props: None,
            ui_placeholder: None,
            ui_hint: None,
            ui_group: None,
            ui_sort_order: 0,
            ui_span: 1,
            ui_newline: 0,
            ui_dynamic_cond: None,
            field_permission: None,
            storage_slot: slot.map(std::string::ToString::to_string),
            description: None,
            tags: None,
            status: "active".into(),
            metadata: None,
            created_at: "2026-09-04T00:00:00Z".into(),
            updated_at: "2026-09-04T00:00:00Z".into(),
            created_by: None,
            updated_by: None,
            deleted_at: None,
            deleted_by: None,
            version: 1,
            _hash: None,
        };
        if ftype != "string" {
            f.is_required = 0;
        }
        f
    }

    fn sample_entity() -> EntityWithFields {
        EntityWithFields {
            entity: meta_entity("project"),
            fields: vec![
                meta_field("project_code", "项目编号", "string", Some("ext_str_01")),
                meta_field("budget", "项目预算", "decimal", Some("ext_dec_01")),
                meta_field("qty", "数量", "integer", Some("ext_int_01")),
                meta_field("active", "是否启用", "boolean", None),
                meta_field("start_at", "开始时间", "datetime", Some("ext_date_01")),
            ],
            slot_map: [
                ("project_code".to_string(), "ext_str_01".to_string()),
                ("budget".to_string(), "ext_dec_01".to_string()),
                ("qty".to_string(), "ext_int_01".to_string()),
                ("start_at".to_string(), "ext_date_01".to_string()),
            ]
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn test_generate_crud_artifacts_complete() {
        let out = generate(&sample_entity(), CodegenTemplate::Crud).expect("generate ok");
        assert_eq!(out.entity_code, "project");
        assert_eq!(out.template, "TPL-01");
        for path in [
            "ddl/biz_project.sql",
            "rust/Project.rs",
            "ts/api.ts",
            "vue/ProjectList.vue",
            "vue/ProjectForm.vue",
            "router/projects.ts",
            "menu.json",
        ] {
            assert!(out.artifacts.contains_key(path), "missing artifact `{path}`");
        }
        let ddl = &out.artifacts["ddl/biz_project.sql"];
        assert!(ddl.contains("CREATE TABLE biz_project"), "ddl table");
        assert!(ddl.contains("project_code"), "ddl column project_code");
        let rs = &out.artifacts["rust/Project.rs"];
        assert!(rs.contains("pub struct Project"), "rust struct");
        let vue = &out.artifacts["vue/ProjectList.vue"];
        assert!(vue.contains("项目编号"), "column label");
        let api = &out.artifacts["ts/api.ts"];
        assert!(api.contains("/api/biz/projects"), "api path");
    }

    #[test]
    fn test_generate_deterministic_byte_level() {
        let a = generate(&sample_entity(), CodegenTemplate::Crud).expect("a");
        let b = generate(&sample_entity(), CodegenTemplate::Crud).expect("b");
        assert_eq!(a, b, "same input must produce byte-identical artifacts");
    }

    #[test]
    fn test_invalid_field_type_rejected() {
        let mut e = sample_entity();
        e.fields.push(meta_field("bad", "坏字段", "money", None));
        let err = generate(&e, CodegenTemplate::Crud).expect_err("must reject");
        assert!(matches!(err, CodegenError::InvalidField(_)));
        assert!(err.to_string().contains("money"));
    }

    #[test]
    fn test_duplicate_field_code_rejected() {
        let mut e = sample_entity();
        e.fields.push(meta_field("budget", "重复预算", "decimal", None));
        let err = generate(&e, CodegenTemplate::Crud).expect_err("must reject");
        assert!(matches!(err, CodegenError::InvalidField(_)));
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn test_empty_entity_code_rejected() {
        let mut e = sample_entity();
        e.entity.entity_code = "  ".into();
        let err = generate(&e, CodegenTemplate::Crud).expect_err("must reject");
        assert!(matches!(err, CodegenError::InvalidEntity(_)));
    }

    #[test]
    fn test_parse_template_aliases() {
        assert_eq!(parse_template(""), Ok(CodegenTemplate::Crud));
        assert_eq!(parse_template("tpl-02"), Ok(CodegenTemplate::Tree));
        assert_eq!(parse_template(" WORKFLOW "), Ok(CodegenTemplate::Workflow));
        assert_eq!(parse_template("AI_CHAT"), Ok(CodegenTemplate::AiChat));
        assert_eq!(parse_template("TPL-04"), Ok(CodegenTemplate::Graph));
        assert!(parse_template("TPL-03").is_err(), "TPL-03 needs two entities, not dispatched here");
        assert!(parse_template("TPL-99").is_err());
    }

    #[test]
    fn test_build_entity_view_runs_codegen() {
        let fields = vec![FieldDef {
            code: "title".into(),
            name: "标题".into(),
            r#type: FieldType::String,
            required: true,
            indexed: true,
            searchable: true,
            sortable: true,
            filterable: true,
            ui_component: None,
            options_inline: None,
        }];
        let view = build_entity_view("t-gate", "project", "项目", &fields);
        assert_eq!(view.entity.entity_code, "project");
        assert_eq!(view.fields.len(), 1);
        assert_eq!(view.fields[0].field_type, "string");
        let out = generate(&view, CodegenTemplate::Crud).expect("view must be generatable");
        assert!(out.artifacts.contains_key("ddl/biz_project.sql"));
    }

    #[test]
    fn test_all_templates_dispatch() {
        for (tpl, marker) in [
            (CodegenTemplate::Crud, "CREATE TABLE"),
            (CodegenTemplate::Tree, "parent_id"),
            (CodegenTemplate::Graph, "rel_type"),
            (CodegenTemplate::Workflow, "workflow_code"),
            (CodegenTemplate::AiChat, "/api/ai/"),
        ] {
            let out = generate(&sample_entity(), tpl).expect("dispatch ok");
            assert_eq!(out.template, tpl.tpl_code());
            assert!(
                out.artifacts.values().any(|c| c.contains(marker)),
                "tpl {} missing marker `{marker}`",
                tpl.tpl_code()
            );
        }
        let md = generate_master_detail(&sample_entity(), &sample_entity())
            .expect_err("same code must be rejected by master_detail");
        assert!(matches!(md, CodegenError::InvalidEntity(_)));
    }

    #[test]
    fn test_fielddef_roundtrip_matches_repo_types() {
        // 保证 codegen 与 repo 写入侧的 FieldType 枚举一致（防两处映射漂移）。
        for t in [
            FieldType::String,
            FieldType::Int,
            FieldType::Decimal,
            FieldType::Boolean,
            FieldType::DateTime,
            FieldType::Enum,
            FieldType::Text,
            FieldType::Json,
        ] {
            let fd = FieldDef {
                code: "f".into(),
                name: "字段".into(),
                r#type: t.clone(),
                required: false,
                indexed: false,
                searchable: false,
                sortable: false,
                filterable: false,
                ui_component: None,
                options_inline: None,
            };
            assert!(
                naming::is_known_field_type(&naming::field_type_to_string(&fd.r#type)),
                "type {t:?} must roundtrip"
            );
        }
    }
}
