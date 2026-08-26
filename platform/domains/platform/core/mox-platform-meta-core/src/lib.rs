pub mod ddl {
    pub const SQL: &str = include_str!("ddl.sql");
}
pub mod model;
pub mod repo;

pub use model::{
    EntityWithFields, FieldSpec, MetaComponent, MetaEntity, MetaField, MetaFieldOptionDict,
    MetaFieldOptionDictItem, MetaIndustryPackage, MetaPage, MetaRule, MetaTenantIndustry,
    MetaView, MetaViewColumn, MetaWorkflow, MetaWorkflowInstance, MetaWorkflowInstanceState,
    MetaWorkflowNode, MetaWorkflowTransition, RuleResult,
};
pub use repo::MetaRepository;

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::collections::HashMap;

    fn setup() -> MetaRepository {
        let conn = Connection::open_in_memory().unwrap();
        let repo = MetaRepository::new(conn);
        repo.init_schema().unwrap();
        repo.seed_industry().unwrap();
        repo
    }

    #[test]
    fn test_define_project_entity_and_slots() {
        let repo = setup();
        let tenant_id = "t001";

        let fields = vec![
            FieldSpec {
                field_code: "project_code".to_string(),
                field_name: "项目编号".to_string(),
                field_type: "string".to_string(),
                is_required: true,
                is_indexed: true,
                is_searchable: true,
                is_sortable: true,
                is_filterable: true,
                description: Some("唯一项目编号".to_string()),
            },
            FieldSpec {
                field_code: "project_name".to_string(),
                field_name: "项目名称".to_string(),
                field_type: "string".to_string(),
                is_required: true,
                is_indexed: false,
                is_searchable: true,
                is_sortable: true,
                is_filterable: true,
                description: None,
            },
            FieldSpec {
                field_code: "budget".to_string(),
                field_name: "项目预算".to_string(),
                field_type: "decimal".to_string(),
                is_required: false,
                is_indexed: true,
                is_searchable: false,
                is_sortable: true,
                is_filterable: true,
                description: None,
            },
        ];

        let (entity_id, slot_map) = repo
            .define_entity(tenant_id, "project", "项目", Some("transaction"), fields)
            .expect("define entity");

        assert!(!entity_id.is_empty(), "entity_id should not be empty");

        let fetched = repo
            .get_entity(tenant_id, "project")
            .expect("get entity")
            .expect("entity exists");
        assert_eq!(fetched.entity.entity_code, "project");
        assert_eq!(fetched.entity.entity_name, "项目");
        assert_eq!(fetched.fields.len(), 3, "should have 3 fields");

        let merged: HashMap<String, String> = fetched
            .slot_map
            .into_iter()
            .chain(slot_map.into_iter())
            .collect();

        let code_slot = merged.get("project_code").expect("slot for project_code");
        assert!(
            code_slot.starts_with("ext_str_"),
            "project_code expected to land in ext_str_*, got {}",
            code_slot
        );

        let name_slot = merged.get("project_name").expect("slot for project_name");
        assert!(
            name_slot.starts_with("ext_str_"),
            "project_name expected to land in ext_str_*, got {}",
            name_slot
        );

        assert_ne!(
            code_slot, name_slot,
            "two string fields must get different slots"
        );

        let budget_slot = merged.get("budget").expect("slot for budget");
        assert!(
            budget_slot.starts_with("ext_dec_"),
            "budget expected to land in ext_dec_*, got {}",
            budget_slot
        );

        let strong_count = merged
            .values()
            .filter(|s| s.starts_with("ext_str_") || s.starts_with("ext_dec_"))
            .count();
        assert!(
            strong_count >= 2,
            "at least 2 fields should land in strong-typed columns, got {}",
            strong_count
        );
    }

    #[test]
    fn test_industry_seed_count() {
        let repo = setup();
        let mut stmt = repo
            .conn
            .prepare("SELECT COUNT(*) FROM meta_industry_package WHERE status='active'")
            .unwrap();
        let cnt: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
        assert!(cnt >= 7, "expected at least 7 industry packages, got {}", cnt);
    }
}
