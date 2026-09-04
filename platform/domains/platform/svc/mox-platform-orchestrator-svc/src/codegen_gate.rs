// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! meta.codegen 出码闸门接线：出码必经 ⛨verify + 治理 8 闸门 + I-05 双验收（与 `/api/mox/publish` 同一条链，禁止绕过）。
//!
//! 链路：内联实体元数据 → meta-core codegen 出码 → 出码流程蓝图归一化为 FlowGraph
//! → `mox_optimize`（⛨verify + 8 闸门）→ I-05 双验收联动 → 放行/拦截裁决。
//! 治理内核不做任何改动（复用 `mox_ai_expert_svc::pipeline::mox_optimize`）。

use serde::Deserialize;
use serde_json::{json, Value};

use mox_ai_expert_svc::context::{GovernContext, Principal, Tenant};
use mox_ai_expert_svc::pipeline::mox_optimize;
use mox_platform_meta_core::codegen::{self, CodegenTemplate};

/// codegen 出码字段（与 `/entities/define` 字段风格一致）。
#[derive(Debug, Clone, Deserialize)]
pub struct CodegenFieldReq {
    pub code: String,
    pub name: String,
    /// string/integer/decimal/boolean/datetime/enum/text/json（见 meta-core naming::field_type_from_str）
    pub r#type: String,
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
}

/// codegen 出码实体。
#[derive(Debug, Clone, Deserialize)]
pub struct CodegenEntityReq {
    #[serde(default)]
    pub tenant_id: Option<String>,
    pub entity_code: String,
    pub entity_name: String,
    pub fields: Vec<CodegenFieldReq>,
}

/// `POST /api/mox/codegen-publish` 请求。
#[derive(Debug, Deserialize)]
pub struct CodegenPublishRequest {
    pub entity: CodegenEntityReq,
    /// TPL-03 时必填（明细实体）
    #[serde(default)]
    pub detail_entity: Option<CodegenEntityReq>,
    /// TPL-01~06，缺省 TPL-01
    #[serde(default)]
    pub template: Option<String>,
    /// I-05 双验收：需求侧任务是否 Done（与融合侧璇玑验证共同决定放行）
    #[serde(default)]
    pub task_done: Option<bool>,
}

fn to_field_defs(fields: &[CodegenFieldReq]) -> Result<Vec<mox_platform_meta_core::FieldDef>, String> {
    fields
        .iter()
        .map(|f| {
            let t = mox_platform_meta_core::codegen::naming::field_type_from_str(&f.r#type)
                .ok_or_else(|| format!("entity field `{}` has unknown type `{}`", f.code, f.r#type))?;
            Ok(mox_platform_meta_core::FieldDef {
                code: f.code.clone(),
                name: f.name.clone(),
                r#type: t,
                required: f.required,
                indexed: f.indexed,
                searchable: f.searchable,
                sortable: f.sortable,
                filterable: f.filterable,
                ui_component: None,
                options_inline: None,
            })
        })
        .collect()
}

/// 确定性出码流程蓝图：start → 建模 → 出码 → ⛨闸门 → end。
/// 该蓝图经 `mox_optimize` 走与 `/api/mox/publish` 相同的治理链（不绕过）。
#[must_use]
pub fn codegen_flow_blueprint(entity_code: &str, tpl_code: &str) -> Value {
    json!({
        "nodes": [
            { "id": "start",   "name": "开始",           "type": "start" },
            { "id": "model",   "name": format!("建模:{entity_code}"), "type": "task" },
            { "id": "gen",     "name": format!("出码:{tpl_code}"),    "type": "task" },
            { "id": "gate",    "name": "⛨verify+8闸门",  "type": "guard" },
            { "id": "end",     "name": "结束",           "type": "end" }
        ],
        "edges": [
            { "from": "start", "to": "model" },
            { "from": "model", "to": "gen" },
            { "from": "gen",   "to": "gate" },
            { "from": "gate",  "to": "end" }
        ]
    })
}

/// 蓝图 → FlowGraph（与 main.rs `normalize_flow_to_graph` 同语义，供 lib 侧复用）。
#[must_use]
pub fn normalize_blueprint(v: &Value) -> mox_ai_flow_svc::model::FlowGraph {
    let mut g = mox_ai_flow_svc::model::FlowGraph::new("unified", "codegen-unified-flow");
    if let Some(nodes) = v.get("nodes").and_then(Value::as_array) {
        for n in nodes {
            let id = n.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            let name = n.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            let t = n.get("type").and_then(Value::as_str).unwrap_or("operator");
            let kind = match t {
                "start" => mox_ai_flow_svc::model::NodeKind::Start,
                "end" => mox_ai_flow_svc::model::NodeKind::End,
                "condition" | "decision" => mox_ai_flow_svc::model::NodeKind::Decision,
                "parallel" => mox_ai_flow_svc::model::NodeKind::ParallelFork,
                "guard" => mox_ai_flow_svc::model::NodeKind::Guard,
                "subflow" => mox_ai_flow_svc::model::NodeKind::SubFlow,
                _ => mox_ai_flow_svc::model::NodeKind::Task,
            };
            g.add_node(mox_ai_flow_svc::model::FlowNode::new(id, name, kind));
        }
    }
    if let Some(edges) = v.get("edges").and_then(Value::as_array) {
        for e in edges {
            let kind = if e.get("condition").is_some() || e.get("label").is_some() {
                mox_ai_flow_svc::model::EdgeKind::Conditional
            } else {
                mox_ai_flow_svc::model::EdgeKind::Sequence
            };
            g.add_edge(mox_ai_flow_svc::model::FlowEdge {
                from: e.get("from").and_then(Value::as_str).unwrap_or("").to_string(),
                to: e.get("to").and_then(Value::as_str).unwrap_or("").to_string(),
                kind,
                condition: e.get("condition").and_then(Value::as_str).map(std::string::ToString::to_string),
            });
        }
    }
    g
}

/// 出码 + 闸门：生成产物 → 同链治理裁决。
///
/// # Errors
/// 字段类型非法 / 模板不支持 / TPL-03 缺明细实体 / 元数据非法（entity_code/字段码）。
pub fn codegen_publish(req: &CodegenPublishRequest) -> Result<Value, String> {
    let fds = to_field_defs(&req.entity.fields)?;
    let master = codegen::build_entity_view(
        req.entity.tenant_id.as_deref().unwrap_or("default"),
        &req.entity.entity_code,
        &req.entity.entity_name,
        &fds,
    );
    let raw = req.template.as_deref().unwrap_or("TPL-01");
    let out = if raw.trim().eq_ignore_ascii_case("TPL-03") {
        let detail_req = req
            .detail_entity
            .as_ref()
            .ok_or_else(|| "TPL-03 requires `detail_entity`".to_string())?;
        let dfds = to_field_defs(&detail_req.fields)?;
        let detail = codegen::build_entity_view(
            detail_req.tenant_id.as_deref().unwrap_or("default"),
            &detail_req.entity_code,
            &detail_req.entity_name,
            &dfds,
        );
        codegen::generate_master_detail(&master, &detail).map_err(|e| e.to_string())?
    } else {
        let tpl: CodegenTemplate = codegen::parse_template(raw)?;
        codegen::generate(&master, tpl).map_err(|e| e.to_string())?
    };

    // ===== 与 /api/mox/publish 同一条治理链（⛨verify + 8 闸门 + I-05 双验收）=====
    let ctx = GovernContext::new(
        Tenant::new("default", "default"),
        Principal::new("designer").with_roles(vec!["editor".into()]),
    );
    let blueprint = codegen_flow_blueprint(&req.entity.entity_code, &out.template);
    let report = mox_optimize(&normalize_blueprint(&blueprint), &ctx);
    let score: f64 = if report.expert_scores.is_empty() {
        0.0
    } else {
        report.expert_scores.iter().map(|(_, s)| s).sum::<f64>() / report.expert_scores.len() as f64
    };
    let task_done = req.task_done.unwrap_or(false);
    let dual_acceptance = mox_ai_expert_svc::tenant_policy::dual_acceptance(task_done, &report);
    let artifacts_json: Value = out
        .artifacts
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect::<serde_json::Map<String, Value>>()
        .into();

    let gates: Vec<Value> = report
        .gate
        .gates
        .iter()
        .map(|g| json!({ "id": g.id.code(), "name": g.id.name(), "passed": g.passed, "reason": g.reason }))
        .collect();

    if !dual_acceptance {
        let mut reasons: Vec<String> = Vec::new();
        if !task_done {
            reasons.push("需求侧任务未标记 Done（task_done=false）".into());
        }
        if report.algo.vetoed {
            reasons.push("融合侧璇玑验证否决（⛨ 最高权限）".into());
        }
        if !report.gate.approved {
            reasons.push(format!("治理门禁未通过：{}", report.gate.reason));
        }
        return Ok(json!({
            "released": false,
            "blocked": true,
            "dual_acceptance": false,
            "reason": reasons.join("；"),
            "template": out.template,
            "entity_code": out.entity_code,
            "artifact_count": out.artifacts.len(),
            "governance": { "score": score, "gate": format!("{:?}", report.gate.status), "algo_veto": report.algo.vetoed },
            "gates": gates,
        }));
    }

    Ok(json!({
        "released": true,
        "blocked": false,
        "dual_acceptance": true,
        "template": out.template,
        "entity_code": out.entity_code,
        "artifacts": artifacts_json,
        "governance": { "score": score, "gate": format!("{:?}", report.gate.status) },
        "gates": gates,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(template: &str, task_done: bool) -> CodegenPublishRequest {
        CodegenPublishRequest {
            entity: CodegenEntityReq {
                tenant_id: Some("t-gate".into()),
                entity_code: "project".into(),
                entity_name: "项目".into(),
                fields: vec![
                    CodegenFieldReq { code: "title".into(), name: "标题".into(), r#type: "string".into(), required: true, indexed: true, searchable: true, sortable: true, filterable: true },
                    CodegenFieldReq { code: "amount".into(), name: "金额".into(), r#type: "decimal".into(), required: false, indexed: false, searchable: false, sortable: true, filterable: true },
                ],
            },
            detail_entity: None,
            template: Some(template.into()),
            task_done: Some(task_done),
        }
    }

    #[test]
    fn test_codegen_publish_released_when_dual_acceptance_pass() {
        let v = codegen_publish(&req("TPL-01", true)).expect("ok");
        assert_eq!(v["released"], json!(true));
        assert_eq!(v["template"], json!("TPL-01"));
        assert!(v["artifacts"]["ddl/biz_project.sql"].is_string());
        assert!(v["governance"]["score"].is_number());
    }

    #[test]
    fn test_codegen_publish_blocked_when_task_not_done() {
        let v = codegen_publish(&req("TPL-01", false)).expect("ok");
        assert_eq!(v["released"], json!(false));
        assert_eq!(v["blocked"], json!(true));
        assert!(v["reason"].as_str().expect("reason").contains("task_done=false"));
    }

    #[test]
    fn test_codegen_publish_error_paths() {
        let mut r = req("TPL-99", true);
        assert!(codegen_publish(&r).is_err(), "unknown template rejected");
        r = req("TPL-03", true);
        assert!(codegen_publish(&r).is_err(), "TPL-03 without detail rejected");
        r.template = Some("TPL-01".into());
        r.entity.fields[0].r#type = "money".into();
        assert!(codegen_publish(&r).is_err(), "unknown field type rejected");
    }

    #[test]
    fn test_blueprint_deterministic() {
        assert_eq!(
            codegen_flow_blueprint("project", "TPL-01"),
            codegen_flow_blueprint("project", "TPL-01")
        );
        let g = normalize_blueprint(&codegen_flow_blueprint("project", "TPL-01"));
        assert!(!g.nodes.is_empty(), "blueprint must normalize into FlowGraph nodes");
    }
}
