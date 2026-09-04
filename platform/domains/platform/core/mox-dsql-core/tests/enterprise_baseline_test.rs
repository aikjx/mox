use mox_dsql_core::{DsqlManager, ExecuteRequest};
use rusqlite::params;
use std::path::Path;

fn read_deploy_sql(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../../deploy/sql")
        .join(name)
        .canonicalize()
        .expect("deploy SQL exists");
    std::fs::read_to_string(path).expect("read deploy SQL")
}

#[test]
fn expert_alliance_schema_and_dynamic_sql_are_compatible() {
    let manager = DsqlManager::open_memory().expect("manager");
    let schema = read_deploy_sql("mox-expert-alliance.sql");
    manager.execute_ddl(&schema).expect("alliance schema");

    let registry = read_deploy_sql("mox-expert-alliance-dsql.sql");
    manager
        .storage()
        .connection()
        .lock()
        .execute_batch(&registry)
        .expect("dynamic SQL registry");

    manager
        .execute_ddl(
            "INSERT INTO mox_alliance_tenant (tenant_id, tenant_code, tenant_name) VALUES ('t1', 'acme', 'Acme');
             INSERT INTO mox_alliance_expert (expert_id, tenant_id, expert_code, display_name, domain) VALUES ('e1', 't1', 'expert-legal', 'Legal Expert', 'legal');",
        )
        .expect("seed alliance facts");

    let result = manager
        .execute(&ExecuteRequest {
            sql_code: "alliance.expert.list".to_string(),
            params: serde_json::json!({ "tenant_id": "t1", "domain": "legal" }),
            trace_id: Some("baseline-test".to_string()),
        })
        .expect("execute registered SQL");
    assert_eq!(result.row_count, Some(1));
    assert_eq!(result.data.unwrap()[0]["expert_code"], "expert-legal");

    let active_count: i64 = manager
        .storage()
        .connection()
        .lock()
        .query_row(
            "SELECT COUNT(*) FROM dsql_definition WHERE entity_code = ?1 AND status = 'ACTIVE'",
            params!["alliance"],
            |row| row.get(0),
        )
        .expect("count registered SQL");
    assert_eq!(active_count, 4);
}
