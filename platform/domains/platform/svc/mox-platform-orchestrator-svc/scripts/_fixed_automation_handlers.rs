pub async fn chat_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<Principal>,
    Json(req): Json<AutomationChatRequest>,
) -> ApiResponse<AutomationChatResponse> {
    // ── RBAC 闸门：Editor+ 方可提交需求编译 ──
    if !check_permission(&principal.roles, &Permission::EditFlow) {
        tracing::warn!(
            target: "automation",
            token_id = %principal.token_id,
            roles = ?principal.roles,
            "RBAC denied: EditFlow required for compile"
        );
        return api_error(403, "权限不足：需 Editor 角色以上才可提交需求编译".into());
    }

    let name = req
        .name
        .clone()
        .unwrap_or_else(|| req.requirement.chars().take(20).collect());
    let asset = match build_asset(&state, &req.requirement, &name, req.tags.clone()).await {
        Ok(v) => v,
        Err(e) => return api_error(500, e.to_string()),
    };
    if let Some(sid) = &req.session_id {
        tracing::info!(target: "automation", session_id = %sid, asset_id = %asset.id, "需求对话会话续接");
    }

    let mermaid = flow_definition_to_mermaid(&asset.blueprint.flow);
    let summary = BlueprintSummary {
        feature_count: asset.blueprint.features.len(),
        entity_count: asset.blueprint.entities.len(),
        node_count: asset.blueprint.flow.nodes.len(),
        edge_count: asset.blueprint.flow.edges.len(),
        features: asset
            .blueprint
            .features
            .iter()
            .map(|f| format!("{}（{}）", f.name, f.action))
            .collect(),
    };

    let resp = AutomationChatResponse {
        asset_id: asset.id.clone(),
        name: asset.name.clone(),
        blueprint_summary: summary,
        code_files: vec!["flow_app.py".into(), "schema.sql".into(), "App.vue".into()],
        test_count: asset.tests.len(),
        rbac_count: asset.rbac.len(),
        mermaid,
        code: asset.code.clone(),
    };

    if let Err(e) = crate::automation_asset::save_automation(asset) {
        return api_error(500, e);
    }

    api_ok(resp)
}

/// 继续对话迭代：在已有资产上增量补功能
///
/// RBAC 闸门：需 [`Permission::EditFlow`]（Editor/Admin）方可追加功能。
/// 访客（Viewer/无 token）→ 403。
pub async fn refine_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    Json(req): Json<AutomationChatRequest>,
) -> ApiResponse<AutomationChatResponse> {
    // ── RBAC 闸门：Editor+ 方可追加功能 ──
    if !check_permission(&principal.roles, &Permission::EditFlow) {
        tracing::warn!(
            target: "automation",
            token_id = %principal.token_id,
            roles = ?principal.roles,
            "RBAC denied: EditFlow required for refine"
        );
        return api_error(403, "权限不足：需 Editor 角色以上才可追加功能".into());
    }

    let asset_opt = match crate::automation_asset::get_automation(&id) {
        Ok(v) => v,
        Err(e) => return api_error(500, e),
    };
    let mut asset = match asset_opt {
        Some(v) => v,
        None => return api_error(404, "自动化资产不存在".into()),
    };

    let bp = match state
        .ai_agent
        .refine_blueprint(&asset.blueprint.id, &req.requirement)
        .await
    {
        Ok(v) => v,
        Err(e) => return api_error(500, e.to_string()),
    };

    let code = generate_code_from_blueprint(&bp);
    let tests = vec![AutoTestGen::generate(&code.python, "flow_app", "main")];
    let lite = blueprint_to_lite(&bp);
    let (_roles, rbac) = RbacDeriver::derive(&lite);

    asset.blueprint = bp;
    asset.code = code;
    asset.tests = tests;
    asset.rbac = rbac;
    // 增量会话可追加分类标签（去重合并，保留既有标签）
    for t in &req.tags {
        if !asset.tags.contains(t) {
            asset.tags.push(t.clone());
        }
    }
    asset.updated_at = chrono::Utc::now().to_rfc3339();
    if let Err(e) = crate::automation_asset::save_automation(asset.clone()) {
        return api_error(500, e);
    }

    let mermaid = flow_definition_to_mermaid(&asset.blueprint.flow);
    let summary = BlueprintSummary {
        feature_count: asset.blueprint.features.len(),
        entity_count: asset.blueprint.entities.len(),
        node_count: asset.blueprint.flow.nodes.len(),
        edge_count: asset.blueprint.flow.edges.len(),
        features: asset
            .blueprint
            .features
            .iter()
            .map(|f| format!("{}（{}）", f.name, f.action))
            .collect(),
    };
    api_ok(AutomationChatResponse {
        asset_id: asset.id.clone(),
        name: asset.name.clone(),
        blueprint_summary: summary,
        code_files: vec!["flow_app.py".into(), "schema.sql".into(), "App.vue".into()],
        test_count: asset.tests.len(),
        rbac_count: asset.rbac.len(),
        mermaid,
        code: asset.code.clone(),
    })
}

/// 沙箱实跑 + 异常自动修复回写
pub async fn run_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<RunRequest>,
) -> ApiResponse<RunResponse> {
    let asset_opt = match crate::automation_asset::get_automation(&id) {
        Ok(v) => v,
        Err(e) => return api_error(500, e),
    };
    let mut asset = match asset_opt {
        Some(v) => v,
        None => return api_error(404, "自动化资产不存在".into()),
    };

    // 安全上限：客户端可传 timeout_sec，夹在 [1,30]s，防止无界等待/资源占用
    let timeout = Duration::from_secs(req.timeout_sec.unwrap_or(15).clamp(1, 30));
    let mut run_result = run_python_sandbox(&asset.code.python, timeout).await;

    let mut fix_summary: Option<FixSummary> = None;
    let mut updated_code: Option<String> = None;

    let success = run_result.exit_code == 0 && !run_result.timed_out;
    if !success {
        // 环境错误（如解释器缺失）不触发代码修复，直接记录并提示
        if is_env_error(&run_result) {
            let record = RunRecord {
                ts: chrono::Utc::now().to_rfc3339(),
                exit_code: run_result.exit_code,
                success: false,
                category: Some("EnvError".to_string()),
                fixed: false,
                stderr_tail: run_result
                    .stderr
                    .lines()
                    .rev()
                    .take(8)
                    .collect::<Vec<_>>()
                    .join("\n"),
            };
            asset.run_history.push(record.clone());
            asset.updated_at = chrono::Utc::now().to_rfc3339();
            if let Err(e) = crate::automation_asset::save_automation(asset) {
                return api_error(500, e);
            }
            return api_ok(RunResponse {
                asset_id: id,
                run: record,
                fix: None,
                updated_code_python: None,
            });
        }
        if let Some((prop, fixed_code, applied, source)) =
            try_fix(&state, &run_result, &asset.code.python).await
        {
            if applied {
                // 回写到流程图 Script/Operator 节点 + 代码资产
                let mut flow_json =
                    serde_json::to_value(&asset.blueprint.flow).unwrap_or(Value::Null);
                let target = asset
                    .blueprint
                    .flow
                    .nodes
                    .iter()
                    .find(|n| {
                        matches!(n.node_type, mox_ai_agent_svc::flow_engine::NodeType::Script)
                            || matches!(n.node_type, mox_ai_agent_svc::flow_engine::NodeType::Operator)
                    })
                    .map(|n| n.id.clone());
                if let Some(nid) = target {
                    mox_ai_flow_svc::automation::patch_flow_with_fix(&mut flow_json, &nid, &fixed_code);
                    if let Ok(flow) =
                        serde_json::from_value::<mox_ai_agent_svc::flow_engine::FlowDefinition>(flow_json)
                    {
                        asset.blueprint.flow = flow;
                    }
                }
                asset.code.python = fixed_code.clone();
                updated_code = Some(fixed_code.clone());
                // 重新实跑验证修复（最多一次）
                run_result = run_python_sandbox(&asset.code.python, timeout).await;
            }
            fix_summary = Some(FixSummary {
                category: format!("{:?}", prop.category),
                note: prop.note,
                applied,
                source,
            });
        }
    }

    let final_success = run_result.exit_code == 0 && !run_result.timed_out;
    let record = RunRecord {
        ts: chrono::Utc::now().to_rfc3339(),
        exit_code: run_result.exit_code,
        success: final_success,
        category: fix_summary.as_ref().map(|f| f.category.clone()),
        fixed: fix_summary.as_ref().map(|f| f.applied).unwrap_or(false),
        stderr_tail: run_result
            .stderr
            .lines()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .join("\n"),
    };
    asset.run_history.push(record.clone());
    asset.updated_at = chrono::Utc::now().to_rfc3339();
    if let Err(e) = crate::automation_asset::save_automation(asset) {
        return api_error(500, e);
    }

    api_ok(RunResponse {
        asset_id: id,
        run: record,
        fix: fix_summary,
        updated_code_python: updated_code,
    })
}

/// 保存前端编辑结果（代码 + 可选流程图），实现「可继续编辑流程」
pub async fn update_handler(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateAutomationRequest>,
) -> ApiResponse<Value> {
    let asset_opt = match crate::automation_asset::get_automation(&id) {
        Ok(v) => v,
        Err(e) => return api_error(500, e),
    };
    let mut asset = match asset_opt {
        Some(v) => v,
        None => return api_error(404, "自动化资产不存在".into()),
    };

    if let Some(py) = payload.python {
        asset.code.python = py;
    }
    if let Some(sql) = payload.sql {
        asset.code.sql = sql;
    }
    if let Some(vue) = payload.vue {
        asset.code.vue = vue;
    }
    if let Some(flow_json) = payload.flow {
        if let Ok(flow) = serde_json::from_value::<mox_ai_agent_svc::flow_engine::FlowDefinition>(flow_json)
        {
            asset.blueprint.flow = flow;
        }
    }
    asset.updated_at = chrono::Utc::now().to_rfc3339();
    if let Err(e) = crate::automation_asset::save_automation(asset.clone()) {
        return api_error(500, e);
    }
    api_ok(serde_json::json!({
        "ok": true,
        "id": asset.id,
        "name": asset.name,
        "updated_at": asset.updated_at,
        "feature_count": asset.blueprint.features.len(),
        "run_count": asset.run_history.len(),
    }))
}

/// 前端编辑保存请求
#[derive(Debug, Deserialize)]
pub struct UpdateAutomationRequest {
    pub python: Option<String>,
    pub sql: Option<String>,
    pub vue: Option<String>,
    /// 可选：编辑后的流程图 JSON（nodes/edges）
    pub flow: Option<Value>,
}

/// 查看自动推导的 RBAC
pub async fn permissions_handler(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResponse<PermissionsResponse> {
    let asset_opt = match crate::automation_asset::get_automation(&id) {
        Ok(v) => v,
        Err(e) => return api_error(500, e),
    };
    let asset = match asset_opt {
        Some(v) => v,
        None => return api_error(404, "自动化资产不存在".into()),
    };

    let lite = blueprint_to_lite(&asset.blueprint);
    let (roles, perms) = RbacDeriver::derive(&lite);
    api_ok(PermissionsResponse {
        roles,
        permissions: perms,
    })
}

/// 列出所有自动化资产（轻量摘要）
pub async fn list_handler(
    State(_state): State<Arc<AppState>>,
) -> ApiResponse<Vec<Value>> {
    let assets = match crate::automation_asset::list_automations() {
        Ok(v) => v,
        Err(e) => return api_error(500, e),
    };
    let summaries: Vec<Value> = assets
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "name": a.name,
                "description": a.description,
                "tags": a.tags,
                "feature_count": a.blueprint.features.len(),
                "run_count": a.run_history.len(),
                "updated_at": a.updated_at,
            })
        })
        .collect();
    api_ok(summaries)
}

