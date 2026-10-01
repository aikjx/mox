use mox_alliance_common_proto::CollaborationPlan;
use serde_json::{json, Value};

fn plan_json() -> Value {
    let task = "00000000-0000-0000-0000-000000000001";
    let node = |id: &str, dependencies: Vec<&str>| {
        json!({
            "node_id": id, "task_id": task, "expert_id": "expert-code",
            "name": id, "status": "pending", "retry_count": 0,
            "dependencies": dependencies, "input_refs": []
        })
    };
    json!({
        "task_id": task, "mode": "dynamic", "fusion_strategy": "weighted",
        "nodes": [node("decision", vec![]), node("yes", vec!["decision"]), node("no", vec!["decision"])],
        "dynamic_routes": [{"decision_node": "decision", "field": "success", "operator": "eq",
            "value": true, "true_branch": ["yes"], "false_branch": ["no"]}],
        "expert_weights": {"expert-code": 0.8}, "version": 1, "created_at": "2026-10-01T00:00:00Z"
    })
}

fn validate(value: Value) -> Result<(), String> {
    serde_json::from_value::<CollaborationPlan>(value).unwrap().validate()
}

#[test]
fn rejects_invalid_plan_contracts() {
    let base = plan_json();
    assert!(validate(base.clone()).is_ok());
    let changes = [
        ("/nodes", json!([])),
        ("/version", json!(0)),
        ("/nodes/0/node_id", json!(" ")),
        ("/nodes/1/node_id", json!("decision")),
        ("/nodes/1/task_id", json!("00000000-0000-0000-0000-000000000002")),
        ("/nodes/0/expert_id", json!("")),
        ("/nodes/1/dependencies", json!(["decision", "decision"])),
        ("/nodes/1/dependencies", json!(["missing"])),
        ("/nodes/0/dependencies", json!(["yes"])),
        ("/expert_weights/expert-code", json!(-1)),
        ("/dynamic_routes/0/operator", json!("script")),
        ("/dynamic_routes/0/field", json!("output..score")),
        ("/dynamic_routes/0/decision_node", json!("missing")),
        ("/dynamic_routes/0/true_branch", json!(["missing"])),
        ("/dynamic_routes/0/true_branch", json!(["yes", "yes"])),
        ("/dynamic_routes/0/false_branch", json!(["yes"])),
        ("/nodes/1/dependencies", json!([])),
    ];
    for (path, replacement) in changes {
        let mut invalid = base.clone();
        *invalid.pointer_mut(path).unwrap() = replacement;
        assert!(validate(invalid).is_err(), "invalid field accepted: {path}");
    }
    let mut duplicate_rule = base;
    let duplicate = duplicate_rule["dynamic_routes"][0].clone();
    duplicate_rule["dynamic_routes"].as_array_mut().unwrap().push(duplicate);
    assert!(validate(duplicate_rule).is_err());
}

#[test]
fn legacy_plan_without_optional_configuration_remains_valid() {
    let mut value = plan_json();
    value["mode"] = json!("parallel");
    value.as_object_mut().unwrap().remove("dynamic_routes");
    value.as_object_mut().unwrap().remove("expert_weights");
    assert!(validate(value).is_ok());
}

#[test]
fn rejects_non_finite_weights_and_accepts_supported_operators() {
    let mut plan: CollaborationPlan = serde_json::from_value(plan_json()).unwrap();
    for weight in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        plan.expert_weights.insert("expert-code".into(), weight);
        assert!(plan.validate().is_err());
    }
    plan.expert_weights.clear();
    for operator in ["eq", "neq", "gt", "gte", "lt", "lte"] {
        plan.dynamic_routes[0].operator = operator.into();
        assert!(plan.validate().is_ok(), "supported operator: {operator}");
    }
}
