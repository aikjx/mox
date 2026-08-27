// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! MOX Enterprise · 全功能 API 处理层
//!
//! 对接前端所有 /api/* 端点，内存存储 + 模拟响应，确保零 404。

pub mod handlers;

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Router,
};
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;

/// 全局共享状态（内存存储）
#[derive(Clone)]
pub struct AppState {
    pub projects: Arc<DashMap<String, Value>>,
    pub tasks: Arc<DashMap<String, Value>>,
    pub sessions: Arc<DashMap<String, Value>>,
    pub experts: Arc<DashMap<String, Value>>,
    pub llm_providers: Arc<DashMap<String, Value>>,
    pub kb_docs: Arc<DashMap<String, Value>>,
    pub market_items: Arc<DashMap<String, Value>>,
    pub flows: Arc<DashMap<String, Value>>,
    pub workflows: Arc<DashMap<String, Value>>,
    pub artifacts: Arc<DashMap<String, Value>>,
    pub api_keys: Arc<DashMap<String, Value>>,
    pub audit_logs: Arc<DashMap<String, Value>>,
    pub graph_nodes: Arc<DashMap<String, Value>>,
    pub graph_edges: Arc<DashMap<String, Value>>,
    pub browser_sessions: Arc<DashMap<String, Value>>,
    pub automation_runs: Arc<DashMap<String, Value>>,
    pub plugins: Arc<DashMap<String, Value>>,
    pub chat_history: Arc<DashMap<String, Value>>,
}

impl Default for AppState {
    fn default() -> Self {
        let state = Self {
            projects: Arc::new(DashMap::new()),
            tasks: Arc::new(DashMap::new()),
            sessions: Arc::new(DashMap::new()),
            experts: Arc::new(DashMap::new()),
            llm_providers: Arc::new(DashMap::new()),
            kb_docs: Arc::new(DashMap::new()),
            market_items: Arc::new(DashMap::new()),
            flows: Arc::new(DashMap::new()),
            workflows: Arc::new(DashMap::new()),
            artifacts: Arc::new(DashMap::new()),
            api_keys: Arc::new(DashMap::new()),
            audit_logs: Arc::new(DashMap::new()),
            graph_nodes: Arc::new(DashMap::new()),
            graph_edges: Arc::new(DashMap::new()),
            browser_sessions: Arc::new(DashMap::new()),
            automation_runs: Arc::new(DashMap::new()),
            plugins: Arc::new(DashMap::new()),
            chat_history: Arc::new(DashMap::new()),
        };
        state.seed_demo_data();
        state
    }
}

impl AppState {
    /// 注入演示数据，确保前端首屏有内容
    fn seed_demo_data(&self) {
        // 演示项目
        let demo_projects = vec![
            serde_json::json!({
                "id": "proj-demo-001", "name": "璇玑全维数字孪生中台",
                "type": "platform", "status": "active",
                "description": "核心平台建设项目", "created_at": "2026-08-01T00:00:00Z"
            }),
            serde_json::json!({
                "id": "proj-demo-002", "name": "政务信创门户改造",
                "type": "government", "status": "active",
                "description": "清远市检察院信创改造", "created_at": "2026-08-10T00:00:00Z"
            }),
        ];
        for p in demo_projects {
            if let Some(id) = p.get("id").and_then(|v| v.as_str()) {
                self.projects.insert(id.to_string(), p);
            }
        }

        // 演示专家
        let demo_experts = vec![
            serde_json::json!({
                "id": "exp-arch", "name": "架构专家", "role": "architect",
                "capabilities": ["系统设计", "技术选型", "架构评审"], "status": "online"
            }),
            serde_json::json!({
                "id": "exp-dev", "name": "开发专家", "role": "developer",
                "capabilities": ["全栈开发", "代码审查", "性能优化"], "status": "online"
            }),
            serde_json::json!({
                "id": "exp-algo", "name": "算法专家", "role": "algorithm",
                "capabilities": ["算法分析", "数学建模", "拓扑优化"], "status": "online"
            }),
        ];
        for e in demo_experts {
            if let Some(id) = e.get("id").and_then(|v| v.as_str()) {
                self.experts.insert(id.to_string(), e);
            }
        }

        // 演示LLM提供商
        let demo_llms = vec![
            serde_json::json!({
                "id": "llm-doubao", "name": "豆包", "provider": "doubao",
                "base_url": "https://ark.cn-beijing.volces.com/api/v3",
                "models": ["doubao-pro-32k", "doubao-lite-128k"], "status": "active", "is_default": true
            }),
            serde_json::json!({
                "id": "llm-openai", "name": "OpenAI", "provider": "openai",
                "base_url": "https://api.openai.com/v1",
                "models": ["gpt-4o", "gpt-4o-mini"], "status": "configured"
            }),
        ];
        for l in demo_llms {
            if let Some(id) = l.get("id").and_then(|v| v.as_str()) {
                self.llm_providers.insert(id.to_string(), l);
            }
        }
    }
}

/// 统一成功响应
pub fn ok<T: serde::Serialize>(data: T) -> Response<Body> {
    let body = serde_json::json!({ "success": true, "data": data });
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

/// 统一成功响应（带额外字段）
pub fn ok_with_extra<T: serde::Serialize>(data: T, extra: Value) -> Response<Body> {
    let mut body = serde_json::json!({ "success": true, "data": data });
    if let Value::Object(map) = &mut body {
        if let Value::Object(extra_map) = extra {
            for (k, v) in extra_map {
                map.insert(k, v);
            }
        }
    }
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

/// 直接返回 JSON（不包 success 信封，用于兼容前端直接取数据的场景）
pub fn json_raw<T: serde::Serialize>(data: T) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&data).unwrap()))
        .unwrap()
}

/// 错误响应
pub fn err(status: StatusCode, code: &str, message: &str) -> Response<Body> {
    let body = serde_json::json!({ "success": false, "code": code, "error": message });
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

/// 生成 UUID
pub fn new_id(prefix: &str) -> String {
    format!("{}-{}", prefix, uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"))
}

/// 构建全功能 API 路由
pub fn api_router(state: AppState) -> Router {
    use handlers::*;

    Router::new()
        // ===== 系统 =====
        .route("/health", get(system_health))
        .route("/status", get(system_status))
        .route("/status/full", get(system_status_full))
        .route("/logs", get(system_logs))
        .route("/plugins", get(system_plugins))
        .route("/config", get(system_config))
        .route("/modules", get(system_modules))

        // ===== 算子 =====
        .route("/operators", get(operators_list))
        .route("/operators/register", post(operators_register))
        .route("/operators/ai-recommend", post(operators_ai_recommend))
        .route("/execute", post(execute_workflow))

        // ===== 知识图谱 =====
        .route("/graph", get(graph_get))
        .route("/graph/stats", get(graph_stats))
        .route("/graph/centrality", get(graph_centrality))
        .route("/graph/communities", get(graph_communities))
        .route("/graph/pagerank", get(graph_pagerank))
        .route("/graph/neighbors/{id}", get(graph_neighbors))
        .route("/graph/path", get(graph_shortest_path))
        .route("/graph/recommend", post(graph_recommend))
        .route("/graph/node", post(graph_add_node))
        .route("/graph/edge", post(graph_add_edge))
        .route("/graph/activate", post(graph_activate))
        .route("/graph/search", get(graph_search))
        .route("/graph/auto-sync/toggle", post(graph_auto_sync_toggle))
        .route("/graph/auto-sync/status", get(graph_auto_sync_status))
        .route("/graph/export", get(graph_export))
        .route("/graph/import", post(graph_import))
        .route("/graph/ai-insights", post(graph_ai_insights))

        // ===== 对话会话 =====
        .route("/dialogue/sessions", get(dialogue_sessions))

        // ===== AI 对话 =====
        .route("/ai/chat", post(ai_chat))
        .route("/ai/chat/history/{session}", get(ai_chat_history))
        .route("/ai/analyze-algorithm", post(ai_analyze_algorithm))
        .route("/ai/algorithm-types", get(ai_algorithm_types))
        .route("/ai/expert-chat", post(ai_expert_chat))
        .route("/ai/resources", get(ai_resources))
        .route("/ai/resources/health", get(ai_resources_health))

        // ===== 全维智能分析 =====
        .route("/ai/full-analysis", post(ai_full_analysis))
        .route("/ai/generate-doc", post(ai_generate_doc))
        .route("/ai/generate-flow-diagram", post(ai_generate_flow_diagram))
        .route("/ai/dev-test-fix", post(ai_dev_test_fix))
        .route("/ai/full-complete", post(ai_full_complete))
        .route("/ai/optimize-doc", post(ai_optimize_doc))
        .route("/ai/project-from-chat", post(ai_project_from_chat))
        .route("/ai/project-graph", post(ai_generate_project_graph))
        .route("/ai/req-db-link", post(ai_link_req_to_db))
        .route("/ai/alliance-pipeline", post(ai_alliance_pipeline))
        .route("/ai/publish-kb", post(ai_publish_artifacts_to_kb))
        .route("/ai/generate-erd", post(ai_generate_erd))
        .route("/ai/engine/flow-graph", get(ai_engine_flow_graph))

        // ===== 无穷维度优化 =====
        .route("/ai/infinite-optimize/benchmarks", get(infinite_benchmarks))
        .route("/ai/infinite-optimize/start", post(infinite_start))
        .route("/ai/infinite-optimize/stop", post(infinite_stop))
        .route("/ai/infinite-optimize/status", get(infinite_status))
        .route("/ai/infinite-optimize/results", get(infinite_results))
        .route("/ai/infinite-optimize/compare", post(infinite_compare))
        .route("/ai/infinite-optimize/comparison", get(infinite_comparison))
        .route("/ai/infinite-optimize/apply", post(infinite_apply))

        // ===== 本地制品引擎 =====
        .route("/ai/artifact/config", get(artifact_config))
        .route("/ai/artifact/list", get(artifact_list))
        .route("/ai/artifact/create", post(artifact_create))

        // ===== AI 插件 =====
        .route("/ai/plugins", get(ai_plugins_list))
        .route("/ai/plugins/register", post(ai_plugins_register))
        .route("/ai/plugins/send-message", post(ai_plugins_send_message))
        .route("/ai/plugins/topology", get(ai_plugins_topology))

        // ===== 工作流 =====
        .route("/ai/workflows/templates", get(workflow_templates))
        .route("/ai/workflows", get(workflows_list))
        .route("/ai/workflows/save", post(workflow_save))
        .route("/ai/workflows/execute", post(workflow_execute))
        .route("/ai/workflows/instances", get(workflow_instances))

        // ===== 流程图 =====
        .route("/ai/flows", get(flows_list))
        .route("/ai/flows", post(flow_create))
        .route("/ai/flows/{id}", get(flow_get))
        .route("/ai/flows/{id}", delete(flow_delete))
        .route("/ai/flows/validate", post(flow_validate))
        .route("/ai/flows/execute", post(flow_execute))
        .route("/ai/flows/node-types", get(flow_node_types))

        // ===== LLM 配置 =====
        .route("/ai/llm/config", get(llm_config_get))
        .route("/ai/llm/config", post(llm_config_update))
        .route("/ai/llm/test", post(llm_test))

        // ===== 浏览器自动化 =====
        .route("/ai/browser/templates", get(browser_templates))
        .route("/ai/browser/sessions", get(browser_sessions))
        .route("/ai/browser/sessions/{id}", get(browser_session_get))
        .route("/ai/browser/sessions/{id}", delete(browser_session_close))
        .route("/ai/browser/execute-task", post(browser_execute_task))
        .route("/ai/browser/execute-steps", post(browser_execute_steps))
        .route("/ai/browser/execute-action", post(browser_execute_action))
        .route("/ai/browser/natural", post(browser_natural))

        // ===== 联网搜索 =====
        .route("/web-search/config", get(web_search_config))
        .route("/web-search/config", post(web_search_config_update))
        .route("/web-search/test", post(web_search_test))
        .route("/web-search", post(web_search_do))

        // ===== 算子商城 =====
        .route("/market", get(market_list))
        .route("/market/random", get(market_random))
        .route("/market/{id}", get(market_get))
        .route("/market/upload", post(market_upload))
        .route("/market/{id}", post(market_update))
        .route("/market/{id}", delete(market_delete))
        .route("/market/{id}/clone", post(market_clone))
        .route("/market/{id}/export", get(market_export))
        .route("/market/ai-search", post(market_ai_search))

        // ===== Caomei =====
        .route("/caomei/compile", post(caomei_compile))
        .route("/caomei/refine", post(caomei_refine))
        .route("/caomei/templates", get(caomei_templates))
        .route("/caomei/ai-parse", post(caomei_ai_parse))

        // ===== MCP =====
        .route("/mcp", post(mcp_handle))
        .route("/mcp/ai-map", post(mcp_ai_map))

        // ===== AI 自动化中枢 =====
        .route("/automation", get(automation_list))
        .route("/automation/chat", post(automation_chat))
        .route("/automation/{id}/refine", post(automation_refine))
        .route("/automation/{id}/run", post(automation_run))
        .route("/automation/{id}/permissions", get(automation_permissions))
        .route("/automation/{id}", put(automation_update))
        .route("/automation/ai-execute", post(automation_ai_execute))

        // ===== 璇玑全维治理 =====
        .route("/mox/health", get(mox_health))
        .route("/mox/optimize", post(mox_optimize))
        .route("/mox/publish", post(mox_publish))

        // ===== LLM 网关 =====
        .route("/llm/providers", get(llm_providers_list))
        .route("/llm/providers/presets", get(llm_provider_presets))
        .route("/llm/providers/{id}", get(llm_provider_get))
        .route("/llm/providers/active", post(llm_set_active))
        .route("/llm/providers", post(llm_provider_add))
        .route("/llm/providers/{id}", put(llm_provider_update))
        .route("/llm/providers/{id}", delete(llm_provider_remove))
        .route("/llm/providers/{id}/enable", post(llm_provider_enable))
        .route("/llm/providers/{id}/disable", post(llm_provider_disable))
        .route("/llm/providers/{id}/test", post(llm_provider_test))
        .route("/llm/providers/{id}/discover", post(llm_provider_discover))
        .route("/llm/health", get(llm_health))
        .route("/llm/routing", get(llm_routing_get))
        .route("/llm/routing", put(llm_routing_update))
        .route("/llm/usage", get(llm_usage))
        .route("/llm/logs", get(llm_logs))
        .route("/llm/stats", get(llm_stats))

        // ===== 专家联盟 =====
        .route("/experts", get(experts_list))
        .route("/experts/capabilities", get(experts_capabilities))
        .route("/experts/metrics", get(experts_metrics))
        .route("/experts/overview", get(experts_overview))
        .route("/experts/multi-consult", post(experts_multi_consult))
        .route("/experts/debate", post(experts_debate))
        .route("/experts/route", post(experts_route))
        .route("/experts/intelligent-consult", post(experts_intelligent_consult))
        .route("/experts/algorithm-analysis", post(experts_algorithm_analysis))
        .route("/experts/enterprise/consult", post(experts_enterprise_consult))
        .route("/experts/enterprise/analyze", post(experts_enterprise_analyze))
        .route("/experts/orchestrate", post(experts_orchestrate))
        .route("/experts/plan/generate", post(experts_plan_generate))
        .route("/experts/plan/execute", post(experts_plan_execute))
        .route("/experts/orchestration/stats", get(orchestration_stats))
        .route("/experts/orchestration/plugins", get(orchestration_plugins))
        .route("/experts/orchestration/history", get(orchestration_history))
        .route("/experts/{id}", get(experts_get))
        .route("/experts", post(experts_register))
        .route("/experts/{id}", put(experts_update))
        .route("/experts/{id}", delete(experts_remove))
        .route("/experts/{id}/consult", post(experts_consult))
        .route("/experts/{id}/metrics", get(experts_single_metrics))

        // ===== 专家会话 =====
        .route("/experts/sessions", get(expert_sessions_list))
        .route("/experts/sessions/stats", get(expert_sessions_stats))
        .route("/experts/sessions", post(expert_session_create))
        .route("/experts/sessions/{id}", get(expert_session_get))
        .route("/experts/sessions/{id}", put(expert_session_update))
        .route("/experts/sessions/{id}", delete(expert_session_delete))
        .route("/experts/sessions/{id}/messages", post(expert_session_append_message))
        .route("/experts/sessions/{id}/similar-search", post(expert_session_similar_search))
        .route("/experts/sessions/{id}/export", get(expert_session_export))
        .route("/experts/sessions/{id}/archive", post(expert_session_archive))
        .route("/experts/semantic-search", post(expert_semantic_search))

        // ===== 调度策略 =====
        .route("/experts/dispatcher/config", get(dispatcher_config))
        .route("/experts/dispatcher/config", put(dispatcher_config_update))
        .route("/experts/dispatcher/status", get(dispatcher_status))
        .route("/experts/dispatcher/dispatch", post(dispatcher_dispatch))
        .route("/experts/dispatcher/consult", post(dispatcher_consult))
        .route("/experts/dispatcher/multi-consult", post(dispatcher_multi_consult))
        .route("/experts/dispatcher/reset/{id}", post(dispatcher_reset_expert))
        .route("/experts/dispatcher/reset-all", post(dispatcher_reset_all))

        // ===== 专家图谱 =====
        .route("/expert-graph", get(expert_graph_get))
        .route("/expert-graph/stats", get(expert_graph_stats))
        .route("/expert-graph/neighbors/{id}", get(expert_graph_neighbors))
        .route("/expert-graph/collaborators/{id}", get(expert_graph_collaborators))
        .route("/expert-graph/path/{source}/{target}", get(expert_graph_path))
        .route("/expert-graph/communities", get(expert_graph_communities))
        .route("/expert-graph/optimal-team", post(expert_graph_optimal_team))
        .route("/expert-graph/rebuild", post(expert_graph_rebuild))

        // ===== 任务管理 =====
        .route("/tasks", get(tasks_list))
        .route("/tasks/auto", post(tasks_auto_create))
        .route("/tasks/from-chat", post(tasks_from_chat))
        .route("/tasks/{id}", get(tasks_get))
        .route("/tasks", post(tasks_create))
        .route("/tasks/{id}", put(tasks_update))
        .route("/tasks/{id}", delete(tasks_delete))
        .route("/tasks/{id}/to-chat", post(tasks_to_chat))
        .route("/tasks/{id}/execute", post(tasks_execute))

        // ===== 项目中心 =====
        .route("/projects", get(projects_list))
        .route("/projects/types", get(projects_types))
        .route("/projects/catalog", get(projects_catalog))
        .route("/projects/stats", get(projects_stats))
        .route("/projects/by-resource", get(projects_by_resource))
        .route("/projects/{id}", get(projects_get))
        .route("/projects", post(projects_create))
        .route("/projects/{id}", put(projects_update))
        .route("/projects/{id}", delete(projects_delete))
        .route("/projects/{id}/resources", post(projects_bind_resources))
        .route("/projects/{id}/resources/{rid}", delete(projects_unbind_resource))
        .route("/projects/{id}/resources/{rid}", put(projects_update_resource_note))

        // ===== 16模块 AI 增强 =====
        .route("/workbench/ai-overview", get(workbench_ai_overview))
        .route("/resources/ai-analysis", post(resources_ai_analysis))
        .route("/workflow/ai-generate", post(workflow_ai_generate))
        .route("/plugins/ai-route", post(plugins_ai_route))
        .route("/browser/ai-instruct", post(browser_ai_instruct))
        .route("/monitor/ai-diagnose", post(monitor_ai_diagnose))
        .route("/docs/ai-explain", post(docs_ai_explain))
        .route("/algolab/ai-analyze", post(algolab_ai_analyze))
        .route("/fusion/ai-govern", post(fusion_ai_govern))

        // ===== 云盘知识库 =====
        .route("/kb/documents", get(kb_documents_list))
        .route("/kb/documents", post(kb_document_create))
        .route("/kb/documents/{id}", get(kb_document_get))
        .route("/kb/documents/{id}", put(kb_document_update))
        .route("/kb/documents/{id}", delete(kb_document_delete))
        .route("/kb/documents/{id}/analyze", post(kb_document_analyze))
        .route("/kb/batch-analyze", post(kb_batch_analyze))
        .route("/kb/categories", get(kb_categories))
        .route("/kb/tags", get(kb_tags))
        .route("/kb/search", post(kb_search))
        .route("/kb/documents/{id}/versions", get(kb_doc_versions))
        .route("/kb/documents/{id}/versions/{ver}", get(kb_doc_version))
        .route("/kb/documents/{id}/versions", post(kb_doc_create_version))
        .route("/kb/documents/{id}/versions/compare", post(kb_doc_compare_versions))
        .route("/kb/documents/{id}/versions/revert", post(kb_doc_revert_version))
        .route("/kb/documents/{id}/entities", get(kb_doc_entities))
        .route("/kb/documents/{id}/graph-link", post(kb_doc_graph_link))
        .route("/kb/documents/{id}/history", get(kb_doc_history))
        .route("/kb/stats", get(kb_stats))
        .route("/kb/history", get(kb_history))

        // ===== Melody2Score =====
        .route("/melody2score/health", get(melody_health))
        .route("/melody2score/status", get(melody_status))
        .route("/melody2score/samples", get(melody_samples))
        .route("/melody2score/recognize", post(melody_recognize))
        .route("/melody2score/recognize-sample", post(melody_recognize_sample))
        .route("/melody2score/recognize-record", post(melody_recognize_record))
        .route("/melody2score/export-sheet", post(melody_export_sheet))
        .route("/melody2score/save-report", post(melody_save_report))

        // ===== 安全管理 =====
        .route("/security/status", get(security_status))
        .route("/security/api-keys", get(security_api_keys))
        .route("/security/api-keys", post(security_create_api_key))
        .route("/security/api-keys/{id}", delete(security_revoke_api_key))
        .route("/security/validate", post(security_validate))
        .route("/security/audit-log", get(security_audit_log))

        // ===== 存储管理 =====
        .route("/storage/providers", get(storage_providers))
        .route("/storage/switch", post(storage_switch))
        .route("/storage/status", get(storage_status))

        // ===== 分析螺旋 =====
        .route("/analyze/spiral", post(analyze_spiral))

        // 兜底：未匹配路由返回 404（带 success 信封）
        .fallback(api_fallback)
        .with_state(state)
}

/// API 兜底：未匹配路由
async fn api_fallback(req: Request) -> impl IntoResponse {
    let path = req.uri().path().to_string();
    err(
        StatusCode::NOT_FOUND,
        "not_found",
        &format!("API 端点未实现: {}", path),
    )
}
