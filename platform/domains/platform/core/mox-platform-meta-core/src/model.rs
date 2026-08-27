// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnumOption {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FieldType {
    String,
    Int,
    Decimal,
    Boolean,
    DateTime,
    Enum,
    Text,
    Json,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldDef {
    pub code: String,
    pub name: String,
    pub r#type: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub indexed: bool,
    #[serde(default)]
    pub searchable: bool,
    #[serde(default)]
    pub sortable: bool,
    #[serde(default)]
    pub filterable: bool,
    #[serde(default)]
    pub ui_component: Option<String>,
    #[serde(default)]
    pub options_inline: Option<Vec<EnumOption>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaIndustryPackage {
    pub package_id: String,
    pub package_code: String,
    pub package_name: String,
    pub package_version: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub features: Option<String>,
    pub seed_entities: Option<String>,
    pub seed_workflows: Option<String>,
    pub seed_rules: Option<String>,
    pub compliance: Option<String>,
    pub is_official: i64,
    pub status: String,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaTenantIndustry {
    pub ti_id: String,
    pub tenant_id: String,
    pub package_id: String,
    pub install_version: Option<String>,
    pub installed_at: String,
    pub installed_by: Option<String>,
    pub config: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaEntity {
    pub entity_id: String,
    pub tenant_id: String,
    pub entity_code: String,
    pub entity_name: String,
    pub entity_plural: Option<String>,
    pub table_name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub entity_category: String,
    pub storage_mode: String,
    pub shard_key: Option<String>,
    pub history_strategy: String,
    pub extends_entity_id: Option<String>,
    pub mixin_ids: Option<String>,
    pub tags: Option<String>,
    pub list_view_id: Option<String>,
    pub form_view_id: Option<String>,
    pub detail_view_id: Option<String>,
    pub workflow_id: Option<String>,
    pub is_system: i64,
    pub status: String,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
    pub version: i64,
    pub _hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaField {
    pub field_id: String,
    pub tenant_id: String,
    pub entity_id: String,
    pub field_code: String,
    pub field_name: String,
    pub field_type: String,
    pub is_required: i64,
    pub is_unique: i64,
    pub is_indexed: i64,
    pub is_searchable: i64,
    pub is_sortable: i64,
    pub is_filterable: i64,
    pub is_exportable: i64,
    pub is_importable: i64,
    pub is_readonly: i64,
    pub is_hidden: i64,
    pub is_system: i64,
    pub default_value: Option<String>,
    pub default_expr: Option<String>,
    pub auto_fill_on: Option<String>,
    pub max_length: Option<i64>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub decimal_places: Option<i64>,
    pub step: Option<f64>,
    pub currency_code: Option<String>,
    pub unit: Option<String>,
    pub options_source: String,
    pub options_inline: Option<String>,
    pub options_sql: Option<String>,
    pub options_api: Option<String>,
    pub options_dict_code: Option<String>,
    pub relation_config: Option<String>,
    pub validations: Option<String>,
    pub formula_expr: Option<String>,
    pub formula_deps: Option<String>,
    pub ui_component: Option<String>,
    pub ui_props: Option<String>,
    pub ui_placeholder: Option<String>,
    pub ui_hint: Option<String>,
    pub ui_group: Option<String>,
    pub ui_sort_order: i64,
    pub ui_span: i64,
    pub ui_newline: i64,
    pub ui_dynamic_cond: Option<String>,
    pub field_permission: Option<String>,
    pub storage_slot: Option<String>,
    pub description: Option<String>,
    pub tags: Option<String>,
    pub status: String,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
    pub version: i64,
    pub _hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldSpec {
    pub field_code: String,
    pub field_name: String,
    pub field_type: String,
    pub is_required: bool,
    pub is_indexed: bool,
    pub is_searchable: bool,
    pub is_sortable: bool,
    pub is_filterable: bool,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityWithFields {
    pub entity: MetaEntity,
    pub fields: Vec<MetaField>,
    pub slot_map: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaView {
    pub view_id: String,
    pub tenant_id: String,
    pub entity_id: Option<String>,
    pub view_code: String,
    pub view_name: String,
    pub view_type: String,
    pub view_mode: String,
    pub roles_whitelist: Option<String>,
    pub roles_blacklist: Option<String>,
    pub permission_codes: Option<String>,
    pub view_config: String,
    pub filter_presets: Option<String>,
    pub sort_order: i64,
    pub is_default: i64,
    pub description: Option<String>,
    pub status: String,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
    pub version: i64,
    pub _hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaViewColumn {
    pub vc_id: String,
    pub tenant_id: String,
    pub view_id: String,
    pub field_code: String,
    pub column_title: Option<String>,
    pub column_width: Option<i64>,
    pub column_fixed: Option<String>,
    pub is_sortable: i64,
    pub is_filterable: i64,
    pub is_visible: i64,
    pub formatter: Option<String>,
    pub component: Option<String>,
    pub align: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaWorkflow {
    pub workflow_id: String,
    pub tenant_id: String,
    pub workflow_code: String,
    pub workflow_name: String,
    pub workflow_category: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub entity_id: Option<String>,
    pub trigger_events: Option<String>,
    pub trigger_condition: Option<String>,
    pub workflow_version: i64,
    pub version_tag: Option<String>,
    pub is_main_version: i64,
    pub process_def: String,
    pub notification: Option<String>,
    pub start_roles: Option<String>,
    pub admin_roles: Option<String>,
    pub viewer_roles: Option<String>,
    pub is_draft: i64,
    pub is_suspended: i64,
    pub status: String,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
    pub version: i64,
    pub _hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaWorkflowNode {
    pub wfn_id: String,
    pub tenant_id: String,
    pub workflow_id: String,
    pub node_id: String,
    pub node_type: String,
    pub node_name: Option<String>,
    pub node_config: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaWorkflowTransition {
    pub wft_id: String,
    pub tenant_id: String,
    pub workflow_id: String,
    pub transition_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub condition: Option<String>,
    pub label: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaWorkflowInstance {
    pub wfi_id: String,
    pub tenant_id: String,
    pub workflow_id: String,
    pub workflow_version: Option<i64>,
    pub entity_id: Option<String>,
    pub biz_id: Option<String>,
    pub biz_code: Option<String>,
    pub biz_title: Option<String>,
    pub instance_status: String,
    pub current_node_id: Option<String>,
    pub current_task_ids: Option<String>,
    pub initiator_id: String,
    pub initiator_dept_id: Option<String>,
    pub admin_user_ids: Option<String>,
    pub cc_user_ids: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub due_at: Option<String>,
    pub suspended_at: Option<String>,
    pub last_active_at: Option<String>,
    pub total_duration_ms: Option<i64>,
    pub form_data: Option<String>,
    pub variables: Option<String>,
    pub context: Option<String>,
    pub final_decision: Option<String>,
    pub final_comment: Option<String>,
    pub completed_count: Option<i64>,
    pub rejected_count: Option<i64>,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaWorkflowInstanceState {
    pub wfis_id: String,
    pub tenant_id: String,
    pub instance_id: String,
    pub node_id: String,
    pub node_status: String,
    pub entered_at: Option<String>,
    pub leaved_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub candidate_users: Option<String>,
    pub candidate_roles: Option<String>,
    pub assignee_user_id: Option<String>,
    pub claim_user_id: Option<String>,
    pub decision: Option<String>,
    pub decision_comment: Option<String>,
    pub decision_data: Option<String>,
    pub decision_at: Option<String>,
    pub decision_user_id: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaRule {
    pub rule_id: String,
    pub tenant_id: String,
    pub rule_code: String,
    pub rule_name: String,
    pub rule_category: String,
    pub entity_id: Option<String>,
    pub workflow_id: Option<String>,
    pub rule_scope: String,
    pub scope_config: Option<String>,
    pub trigger_event: Option<String>,
    pub trigger_cron: Option<String>,
    pub trigger_condition: Option<String>,
    pub priority: i64,
    pub mutex_group: Option<String>,
    pub rule_body: String,
    pub failure_policy: String,
    pub retry_count: i64,
    pub retry_interval: i64,
    pub rule_version: i64,
    pub is_enabled: i64,
    pub status: String,
    pub description: Option<String>,
    pub tags: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub version: i64,
    pub _hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleResult {
    pub passed: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub computed_fields: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaPage {
    pub page_id: String,
    pub tenant_id: String,
    pub page_code: String,
    pub page_name: String,
    pub page_type: String,
    pub entity_id: Option<String>,
    pub route_path: Option<String>,
    pub layout: Option<String>,
    pub page_config: String,
    pub description: Option<String>,
    pub is_system: i64,
    pub status: String,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaComponent {
    pub component_id: String,
    pub tenant_id: String,
    pub component_code: String,
    pub component_name: String,
    pub component_type: String,
    pub entity_id: Option<String>,
    pub page_id: Option<String>,
    pub component_lib: Option<String>,
    pub component_props: Option<String>,
    pub data_source: Option<String>,
    pub event_handlers: Option<String>,
    pub slot_config: Option<String>,
    pub style_config: Option<String>,
    pub responsive_cfg: Option<String>,
    pub sort_order: i64,
    pub is_system: i64,
    pub status: String,
    pub description: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaFieldOptionDict {
    pub dict_id: String,
    pub tenant_id: String,
    pub dict_code: String,
    pub dict_name: String,
    pub dict_category: Option<String>,
    pub is_system: i64,
    pub description: Option<String>,
    pub status: String,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaFieldOptionDictItem {
    pub item_id: String,
    pub tenant_id: String,
    pub dict_id: String,
    pub item_value: String,
    pub item_label: String,
    pub parent_item_id: Option<String>,
    pub sort_order: i64,
    pub item_level: i64,
    pub item_path: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub tag_type: Option<String>,
    pub ext_data: Option<String>,
    pub is_default: i64,
    pub is_disabled: i64,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}
