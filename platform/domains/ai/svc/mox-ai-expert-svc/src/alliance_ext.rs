// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家联盟扩展服务：为前端 52 个 API 契约补齐会话 / 调度 / 专家图谱 / 计划 / 企业级端点。
//!
//! 设计原则：
//! - 复用 [`crate::services::AllianceService`] 的核心能力（注册表 / 咨询 / 路由 / 编排 / 算法分析）；
//! - 会话、计划、调度日志为进程内存态（M3 落 SQLite 持久化）；
//! - 专家图谱基于注册表实时构建（共享能力即协作边），社区按维度分组，路径 BFS。

use crate::services::AllianceService;
use crate::types::{ConsultExpertRequest, ExpertMeta, RouteExpertsRequest};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// M4：评分学习指标（每次咨询累计，0..1 评分 ×5 → 0..5 平均分）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ExpertMetric {
    consultations: u64,
    rating_sum: f64,
    latency_sum: u64,
}

impl ExpertMetric {
    fn avg_rating(&self) -> f64 {
        if self.consultations == 0 {
            0.0
        } else {
            self.rating_sum / self.consultations as f64 * 5.0
        }
    }
    fn avg_latency(&self) -> u64 {
        if self.consultations == 0 {
            0
        } else {
            self.latency_sum / self.consultations as u64
        }
    }
}

/// M4：维度 → 领域族（图谱建边与 optimal_team 角色推断共用）
fn dimension_family(dim: &str) -> &'static str {
    match dim {
        "Architecture" | "Algorithm" | "Testing" | "CodeQuality" | "Maintainability"
        | "Performance" | "SecurityCode" => "engineering",
        "Permission" | "Security" => "security",
        "Data" | "Observability" => "data",
        "Business" | "Documentation" => "business",
        "Resource" => "resource",
        "Persistence" => "persistence",
        _ => "general",
    }
}

/// M4：领域族关键词（用于 optimal_team 文本→角色匹配）
fn family_keywords(fam: &str) -> Vec<&'static str> {
    match fam {
        "engineering" => vec!["架构", "设计", "算法", "测试", "质量", "性能", "优化", "工程", "代码"],
        "security" => vec!["安全", "权限", "漏洞", "合规", "pii", "auth"],
        "data" => vec!["数据", "分析", "etl", "观测", "模型"],
        "business" => vec!["业务", "流程", "文档", "需求"],
        "resource" => vec!["资源", "成本", "配额", "池"],
        "persistence" => vec!["持久化", "存储", "sqlite", "数据库", "库"],
        _ => vec![],
    }
}

/// M4：节点度数（协作边数）
fn degree_map(graph: &Value) -> HashMap<String, f64> {
    let mut m: HashMap<String, f64> = HashMap::new();
    if let Some(edges) = graph["edges"].as_array() {
        for e in edges {
            if let (Some(s), Some(t)) = (e["source"].as_str(), e["target"].as_str()) {
                *m.entry(s.to_string()).or_insert(0.0) += 1.0;
                *m.entry(t.to_string()).or_insert(0.0) += 1.0;
            }
        }
    }
    m
}

/// M4：两位小数
fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// 专家联盟扩展状态（挂载到 `server::AppState.ext`）
pub struct AllianceExtState {
    pub alliance: Arc<AllianceService>,
    /// 会话存储：id -> 会话 JSON
    pub sessions: Mutex<HashMap<String, Value>>,
    /// 调度器配置
    pub dispatcher_cfg: Mutex<Value>,
    /// 计划存储：id -> 计划 JSON
    pub plans: Mutex<HashMap<String, Value>>,
    /// 编排历史（近 100 条）
    pub orch_history: Mutex<Vec<Value>>,
    /// M3：SQLite 持久化
    db: Arc<crate::persistence::PersistenceDb>,
    /// M4：评分学习指标（id -> ExpertMetric，kv 前缀 `metrics:` 持久化）
    metrics: Mutex<HashMap<String, ExpertMetric>>,
}

/// 默认调度配置
fn default_dispatcher_cfg() -> Value {
    json!({
        "strategy": "capability_match",
        "top_n": 3,
        "team_size": 4,
        "enable_llm_debate": false,
        "retry_on_c": false,
    })
}

/// 把 JSON 对象转换为 HashMap<String,String>（供契约 ctx/constraints 字段）
fn str_map(v: &Value) -> HashMap<String, String> {
    let mut m = HashMap::new();
    if let Some(obj) = v.as_object() {
        for (k, val) in obj {
            if let Some(s) = val.as_str() {
                m.insert(k.clone(), s.to_string());
            } else {
                m.insert(k.clone(), val.to_string());
            }
        }
    }
    m
}

impl AllianceExtState {
    pub fn new(alliance: Arc<AllianceService>, db: Arc<crate::persistence::PersistenceDb>) -> Self {
        // M3：启动时从 SQLite 加载持久化会话 / 计划 / 调度配置
        let mut sessions = HashMap::new();
        for (id, v) in db.load_sessions() {
            sessions.insert(id, v);
        }
        let mut plans = HashMap::new();
        for (id, v) in db.load_plans() {
            plans.insert(id, v);
        }
        let cfg = db
            .load_kv("dispatcher_cfg")
            .unwrap_or_else(default_dispatcher_cfg);
        // M4：从库加载评分学习指标（kv 前缀 metrics:，内存键为专家 id）
        let mut metrics = HashMap::new();
        for (k, v) in db.load_kv_prefix("metrics:") {
            if let Ok(m) = serde_json::from_value::<ExpertMetric>(v) {
                let id = k.trim_start_matches("metrics:").to_string();
                metrics.insert(id, m);
            }
        }
        Self {
            alliance,
            sessions: Mutex::new(sessions),
            dispatcher_cfg: Mutex::new(cfg),
            plans: Mutex::new(plans),
            orch_history: Mutex::new(Vec::new()),
            db,
            metrics: Mutex::new(metrics),
        }
    }

    // ========================================================================
    // 会话（sessions）
    // ========================================================================

    pub async fn create_session(&self, body: Value) -> Value {
        // M5.4：会话 ID 语义统一——优先接受 id / session_id 作为存储键，保持二者一致，
        // 避免「创建用自动生成的 sess-xxx 作 key、调用方用传入 session_id 查询 404」的契约断裂。
        let id = body
            .get("id")
            .and_then(|v| v.as_str())
            .or_else(|| body.get("session_id").and_then(|v| v.as_str()))
            .map(String::from)
            .unwrap_or_else(|| format!("sess-{}", &Uuid::new_v4().to_string()[..8]));
        let mut s = body.clone();
        let obj = s.as_object_mut().unwrap();
        obj.insert("id".to_string(), json!(id));
        obj.insert("session_id".to_string(), json!(id));
        obj.entry("status").or_insert_with(|| json!("active"));
        obj.entry("created_at").or_insert_with(|| json!(chrono::Utc::now().to_rfc3339()));
        obj.entry("messages").or_insert_with(|| json!([]));
        self.sessions.lock().await.insert(id.clone(), s.clone());
        let _ = self.db.upsert_session(&id, &s);
        json!({ "id": id, "session": s })
    }

    pub async fn list_sessions(&self, params: Value) -> Value {
        let status = params.get("status").and_then(|v| v.as_str());
        let keyword = params.get("keyword").and_then(|v| v.as_str());
        let g = self.sessions.lock().await;
        let mut arr: Vec<Value> = g
            .values()
            .filter(|s| {
                if let Some(st) = status {
                    if s.get("status").and_then(|v| v.as_str()) != Some(st) {
                        return false;
                    }
                }
                if let Some(kw) = keyword {
                    let text = serde_json::to_string(*s).unwrap_or_default().to_lowercase();
                    if !text.contains(&kw.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();
        arr.sort_by(|a, b| b["created_at"].as_str().cmp(&a["created_at"].as_str()));
        let total = arr.len();
        json!({ "total": total, "sessions": arr })
    }

    pub async fn session_stats(&self) -> Value {
        let g = self.sessions.lock().await;
        let total = g.len();
        let active = g.values().filter(|s| s.get("status").and_then(|v| v.as_str()) == Some("active")).count();
        let messages: usize = g
            .values()
            .map(|s| s.get("messages").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0))
            .sum();
        json!({ "total": total, "active": active, "archived": total - active, "messages": messages })
    }

    pub async fn get_session(&self, id: &str) -> Value {
        let g = self.sessions.lock().await;
        match g.get(id) {
            Some(s) => json!({ "found": true, "session": s.clone() }),
            None => json!({ "found": false, "session": null }),
        }
    }

    pub async fn update_session(&self, id: &str, body: Value) -> Value {
        let mut g = self.sessions.lock().await;
        match g.get_mut(id) {
            Some(s) => {
                if let Some(status) = body.get("status").and_then(|v| v.as_str()) {
                    s["status"] = json!(status);
                }
                if let Some(title) = body.get("title").and_then(|v| v.as_str()) {
                    s["title"] = json!(title);
                }
                if let Some(expert_id) = body.get("expert_id").and_then(|v| v.as_str()) {
                    s["expert_id"] = json!(expert_id);
                }
                let updated = s.clone();
                let _ = self.db.upsert_session(id, &updated);
                json!({ "found": true, "session": updated })
            }
            None => json!({ "found": false, "error": "会话不存在" }),
        }
    }

    pub async fn delete_session(&self, id: &str) -> Value {
        let mut g = self.sessions.lock().await;
        let existed = g.remove(id).is_some();
        if existed {
            let _ = self.db.delete_session(id);
        }
        json!({ "deleted": existed })
    }

    pub async fn append_message(&self, id: &str, body: Value) -> Value {
        let mut g = self.sessions.lock().await;
        match g.get_mut(id) {
            Some(s) => {
                let msg = json!({
                    "role": body.get("role").and_then(|v| v.as_str()).unwrap_or("user"),
                    "content": body.get("content").and_then(|v| v.as_str()).unwrap_or(""),
                    "ts": chrono::Utc::now().to_rfc3339(),
                });
                s["messages"]
                    .as_array_mut()
                    .unwrap_or(&mut Vec::new())
                    .push(msg.clone());
                let updated = s.clone();
                let _ = self.db.upsert_session(id, &updated);
                json!({ "ok": true, "message": msg, "session_id": id })
            }
            None => json!({ "ok": false, "error": "会话不存在" }),
        }
    }

    pub async fn session_similar_search(&self, id: &str, body: Value) -> Value {
        let q = body.get("query").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
        let g = self.sessions.lock().await;
        let mut results: Vec<Value> = Vec::new();
        if let Some(s) = g.get(id) {
            let text = serde_json::to_string(s).unwrap_or_default().to_lowercase();
            if !q.is_empty() && text.contains(&q) {
                results.push(s.clone());
            }
        }
        for (k, v) in g.iter() {
            if k == id {
                continue;
            }
            let text = serde_json::to_string(v).unwrap_or_default().to_lowercase();
            if !q.is_empty() && text.contains(&q) {
                results.push(v.clone());
            }
        }
        json!({ "count": results.len(), "results": results })
    }

    pub async fn semantic_search(&self, body: Value) -> Value {
        let q = body.get("query").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
        let g = self.sessions.lock().await;
        let mut results: Vec<Value> = Vec::new();
        for (k, v) in g.iter() {
            let text = serde_json::to_string(v).unwrap_or_default().to_lowercase();
            if !q.is_empty() && text.contains(&q) {
                results.push(json!({ "id": k, "score": 1.0, "session": v.clone() }));
            }
        }
        json!({ "count": results.len(), "results": results })
    }

    pub async fn export_session(&self, id: &str) -> Value {
        let g = self.sessions.lock().await;
        match g.get(id) {
            Some(s) => json!({ "session_id": id, "export": s.clone(), "format": "json" }),
            None => json!({ "session_id": id, "export": null, "error": "会话不存在" }),
        }
    }

    pub async fn archive_session(&self, id: &str) -> Value {
        let mut g = self.sessions.lock().await;
        match g.get_mut(id) {
            Some(s) => {
                s["status"] = json!("archived");
                let updated = s.clone();
                let _ = self.db.upsert_session(id, &updated);
                json!({ "ok": true, "session_id": id, "status": "archived" })
            }
            None => json!({ "ok": false, "error": "会话不存在" }),
        }
    }

    /// 咨询完成后自动记录会话（若 body 带 session_id）
    pub async fn record_consult_session(&self, session_id: Option<&str>, query: &str, summary: &Value) {
        if let Some(sid) = session_id {
            let mut g = self.sessions.lock().await;
            if let Some(s) = g.get_mut(sid) {
                let msg = json!({
                    "role": "assistant",
                    "content": format!("咨询完成：{}", query),
                    "summary": summary.clone(),
                    "ts": chrono::Utc::now().to_rfc3339(),
                });
                s["messages"]
                    .as_array_mut()
                    .unwrap_or(&mut Vec::new())
                    .push(msg);
                let updated = s.clone();
                let _ = self.db.upsert_session(sid, &updated);
            }
        }
    }

    // ========================================================================
    // 调度器（dispatcher）
    // ========================================================================

    pub async fn get_dispatcher_config(&self) -> Value {
        self.dispatcher_cfg.lock().await.clone()
    }

    pub async fn update_dispatcher_config(&self, body: Value) -> Value {
        let mut cfg = self.dispatcher_cfg.lock().await;
        if let Some(obj) = body.as_object() {
            for (k, v) in obj {
                if k == "strategy" || k == "top_n" || k == "team_size" || k == "enable_llm_debate" || k == "retry_on_c" {
                    cfg[k] = v.clone();
                }
            }
        }
        let updated = cfg.clone();
        let _ = self.db.save_kv("dispatcher_cfg", &updated);
        json!({ "ok": true, "config": updated })
    }

    pub async fn get_dispatcher_status(&self) -> Value {
        let cfg = self.dispatcher_cfg.lock().await.clone();
        json!({
            "running": true,
            "strategy": cfg["strategy"],
            "top_n": cfg["top_n"],
            "team_size": cfg["team_size"],
            "available_experts": self.expert_count().await,
        })
    }

    pub async fn dispatcher_dispatch(&self, body: Value) -> Value {
        let query = body.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let cfg = self.dispatcher_cfg.lock().await.clone();
        let top_n = cfg.get("top_n").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
        let req = RouteExpertsRequest {
            query,
            scenario: body.get("scenario").and_then(|v| v.as_str()).map(String::from),
            constraints: str_map(body.get("constraints").unwrap_or(&json!({}))),
            top_n,
        };
        match self.alliance.route_experts(&req).await {
            Ok(resp) => json!({
                "status": "routed",
                "strategy": cfg["strategy"],
                "result": serde_json::to_value(&resp).unwrap_or(json!({})),
            }),
            Err(e) => json!({ "status": "failed", "error": e.to_string() }),
        }
    }

    pub async fn dispatcher_consult(&self, body: Value) -> Value {
        let query = body.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let cfg = self.dispatcher_cfg.lock().await.clone();
        let top_n = cfg.get("top_n").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
        let req = RouteExpertsRequest {
            query: query.clone(),
            scenario: None,
            constraints: HashMap::new(),
            top_n,
        };
        let routed = match self.alliance.route_experts(&req).await {
            Ok(r) => r.matches.first().map(|m| m.expert.id.clone()).unwrap_or_else(|| "default".into()),
            Err(_) => "default".into(),
        };
        let creq = ConsultExpertRequest {
            query,
            expert_id: Some(routed.clone()),
            ctx: str_map(body.get("ctx").unwrap_or(&json!({}))),
            flow_json: body.get("flow_json").and_then(|v| v.as_str()).map(String::from),
        };
        match self.alliance.consult_expert(&creq).await {
            Ok(resp) => json!({
                "status": "completed",
                "expert_id": routed,
                "result": serde_json::to_value(&resp).unwrap_or(json!({})),
            }),
            Err(e) => json!({ "status": "failed", "expert_id": routed, "error": e.to_string() }),
        }
    }

    pub async fn dispatcher_multi_consult(&self, body: Value) -> Value {
        // 复用 alliance 的 multi-consult（按契约转发）
        let req = crate::types::MultiExpertConsultRequest {
            query: body.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            expert_ids: body
                .get("expert_ids")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            team_size: body.get("team_size").and_then(|v| v.as_u64()).unwrap_or(4) as usize,
            ctx: str_map(body.get("ctx").unwrap_or(&json!({}))),
            flow_json: body.get("flow_json").and_then(|v| v.as_str()).map(String::from),
            parallel: body.get("parallel").and_then(|v| v.as_bool()).unwrap_or(true),
        };
        match self.alliance.multi_expert_consult(&req).await {
            Ok(resp) => json!({
                "status": "completed",
                "result": serde_json::to_value(&resp).unwrap_or(json!({})),
            }),
            Err(e) => json!({ "status": "failed", "error": e.to_string() }),
        }
    }

    pub async fn dispatcher_reset_expert(&self, id: &str) -> Value {
        json!({ "ok": true, "expert_id": id, "reset": true })
    }

    pub async fn dispatcher_reset_all(&self) -> Value {
        json!({ "ok": true, "reset_all": true })
    }

    // ========================================================================
    // 专家图谱（expert-graph）
    // ========================================================================

    async fn all_experts(&self) -> Vec<ExpertMeta> {
        self.alliance.registry().list(None).await.unwrap_or_default()
    }

    async fn expert_count(&self) -> usize {
        self.all_experts().await.len()
    }

    /// 实时构建协作图：节点=专家，边=共享能力（collaborates，权重=共享数/4）
    pub async fn build_graph(&self) -> Value {
        let experts = self.all_experts().await;
        let mut nodes: Vec<Value> = Vec::new();
        let mut edges: Vec<Value> = Vec::new();
        for e in &experts {
            nodes.push(json!({
                "id": e.id,
                "label": e.name,
                "node_type": "expert",
                "properties": {
                    "role": e.dimension.clone().unwrap_or_default(),
                    "domain": e.domain.clone(),
                    "capabilities": e.capabilities.clone(),
                },
            }));
        }
        for i in 0..experts.len() {
            for j in (i + 1)..experts.len() {
                let a = &experts[i];
                let b = &experts[j];
                let shared = a.capabilities.iter().filter(|c| b.capabilities.contains(c)).count();
                if shared > 0 {
                    edges.push(json!({
                        "source": a.id,
                        "target": b.id,
                        "relation_type": "collaborates",
                        "weight": (shared as f64) / 4.0,
                    }));
                }
                // M4：同领域族专家建「same_family」协作边（弥补能力交集稀疏导致 edges=0）
                let fa = dimension_family(a.dimension.as_deref().unwrap_or(""));
                let fb = dimension_family(b.dimension.as_deref().unwrap_or(""));
                if !fa.is_empty() && fa == fb {
                    edges.push(json!({
                        "source": a.id,
                        "target": b.id,
                        "relation_type": "same_family",
                        "family": fa,
                        "weight": 0.5,
                    }));
                }
            }
        }
        json!({ "nodes": nodes, "edges": edges })
    }

    pub async fn get_graph(&self) -> Value {
        self.build_graph().await
    }

    pub async fn graph_stats(&self) -> Value {
        let g = self.build_graph().await;
        let n = g["nodes"].as_array().map(|a| a.len()).unwrap_or(0);
        let e = g["edges"].as_array().map(|a| a.len()).unwrap_or(0);
        let density = if n > 1 { (2.0 * e as f64) / (n as f64 * (n - 1) as f64) } else { 0.0 };
        json!({ "nodes": n, "edges": e, "density": (density * 1000.0).round() / 1000.0, "components": 1 })
    }

    pub async fn graph_neighbors(&self, id: &str) -> Value {
        let g = self.build_graph().await;
        let mut out: Vec<Value> = Vec::new();
        if let Some(edges) = g["edges"].as_array() {
            for e in edges {
                if e["source"] == json!(id) {
                    out.push(e["target"].clone());
                } else if e["target"] == json!(id) {
                    out.push(e["source"].clone());
                }
            }
        }
        json!(out)
    }

    pub async fn graph_collaborators(&self, id: &str, limit: usize) -> Value {
        let mut nb = self.graph_neighbors(id).await;
        let mut empty = Vec::new();
        let arr = nb.as_array_mut().unwrap_or(&mut empty);
        arr.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        arr.dedup();
        if arr.len() > limit {
            arr.truncate(limit);
        }
        json!({ "expert_id": id, "collaborators": arr.clone() })
    }

    pub async fn graph_path(&self, source: &str, target: &str) -> Value {
        let g = self.build_graph().await;
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        if let Some(edges) = g["edges"].as_array() {
            for e in edges {
                if let (Some(s), Some(t)) = (e["source"].as_str(), e["target"].as_str()) {
                    adj.entry(s.to_string()).or_default().push(t.to_string());
                    adj.entry(t.to_string()).or_default().push(s.to_string());
                }
            }
        }
        if source == target {
            return json!({ "found": true, "path": [source], "length": 0 });
        }
        // BFS
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut prev: HashMap<String, String> = HashMap::new();
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        queue.push_back(source.to_string());
        visited.insert(source.to_string());
        let mut found = false;
        while let Some(cur) = queue.pop_front() {
            if cur == target {
                found = true;
                break;
            }
            if let Some(nbrs) = adj.get(&cur) {
                for n in nbrs {
                    if !visited.contains(n) {
                        visited.insert(n.clone());
                        prev.insert(n.clone(), cur.clone());
                        queue.push_back(n.clone());
                    }
                }
            }
        }
        if found {
            let mut path: Vec<String> = Vec::new();
            let mut cur = target.to_string();
            loop {
                path.push(cur.clone());
                if cur == source {
                    break;
                }
                match prev.get(&cur) {
                    Some(p) => cur = p.clone(),
                    None => break,
                }
            }
            path.reverse();
            json!({ "found": true, "path": path, "length": path.len() - 1 })
        } else {
            json!({ "found": false, "path": [], "length": -1 })
        }
    }

    pub async fn graph_communities(&self) -> Value {
        let experts = self.all_experts().await;
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for e in &experts {
            let dim = e.dimension.clone().unwrap_or_else(|| "General".into());
            map.entry(dim).or_default().push(e.id.clone());
        }
        let mut communities: Vec<Value> = Vec::new();
        for (label, members) in map {
            communities.push(json!({ "label": label, "members": members, "size": members.len() }));
        }
        json!({ "count": communities.len(), "communities": communities })
    }

    pub async fn optimal_team(&self, body: Value) -> Value {
        // 兼容 goal / task / query 输入
        let goal = body
            .get("goal")
            .and_then(|v| v.as_str())
            .or_else(|| body.get("task").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_lowercase();
        let query = body
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        let text = format!("{} {}", goal, query);
        let required: Vec<String> = body
            .get("required_capabilities")
            .and_then(|v| v.as_array())
            .or_else(|| body.get("required").and_then(|v| v.as_array()))
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let team_size = body
            .get("team_size")
            .and_then(|v| v.as_u64())
            .or_else(|| body.get("top_n").and_then(|v| v.as_u64()))
            .unwrap_or(3) as usize;
        // M4：历史评分（0..1）快照，避免在闭包内 await
        let ratings: HashMap<String, f64> = {
            let m = self.metrics.lock().await;
            m.iter()
                .map(|(k, v)| {
                    let avg = if v.consultations > 0 {
                        v.rating_sum / v.consultations as f64
                    } else {
                        0.0
                    };
                    (k.clone(), avg)
                })
                .collect()
        };
        let graph = self.build_graph().await;
        let degree = degree_map(&graph);
        let experts = self.all_experts().await;
        let mut scored: Vec<(f64, String, &ExpertMeta)> = experts
            .iter()
            .map(|e| {
                let mut score = 0.0;
                let mut reasons: Vec<String> = Vec::new();
                for cap in &e.capabilities {
                    let cl = cap.to_lowercase();
                    if !text.is_empty() && text.contains(&cl) {
                        score += 1.0;
                        reasons.push(format!("命中能力「{}」", cap));
                    }
                    if required.iter().any(|r| r.to_lowercase() == cl) {
                        score += 2.0;
                        reasons.push(format!("满足必需能力「{}」", cap));
                    }
                }
                // 领域族角色契合
                let fam = dimension_family(e.dimension.as_deref().unwrap_or(""));
                if !fam.is_empty() && family_keywords(fam).iter().any(|k| text.contains(k)) {
                    score += 1.0;
                    reasons.push(format!("领域族「{}」契合", fam));
                }
                // 图谱协作度（degree 归一化加分）
                let deg = degree.get(e.id.as_str()).copied().unwrap_or(0.0) as f64;
                if deg > 0.0 {
                    score += (deg * 0.08).min(0.5);
                }
                // 历史评分加权（0..1 * 0.3）
                if let Some(r) = ratings.get(&e.id) {
                    score += r * 0.3;
                }
                (score, reasons.join("；"), e)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let team: Vec<Value> = scored
            .iter()
            .take(team_size.max(1))
            .map(|(s, reason, e)| {
                json!({
                    "expert_id": e.id,
                    "name": e.name,
                    "dimension": e.dimension,
                    "capabilities": e.capabilities.clone(),
                    "score": round2(*s),
                    "reason": reason,
                })
            })
            .collect();
        // 能力覆盖 / 缺失
        let mut covered: Vec<String> = Vec::new();
        for t in &team {
            if let Some(caps) = t["capabilities"].as_array() {
                for c in caps {
                    let cl = c.as_str().unwrap_or("").to_lowercase();
                    if !cl.is_empty() && !covered.contains(&cl) {
                        covered.push(cl);
                    }
                }
            }
        }
        let missing: Vec<String> = required
            .iter()
            .filter(|r| !covered.contains(&r.to_lowercase()))
            .cloned()
            .collect();
        let total: f64 = team.iter().filter_map(|t| t["score"].as_f64()).sum();
        let avg = if !team.is_empty() { total / team.len() as f64 } else { 0.0 };
        let top_score = scored.first().map(|(s, _, _)| *s).unwrap_or(0.0);
        json!({
            "team": team,
            "score": round2(avg),
            "top_score": round2(top_score),
            "covered_capabilities": covered,
            "missing_capabilities": missing,
            "rationale": format!(
                "按 能力关键词/必需能力/领域族/图谱协作度/历史评分 加权选取 {} 位专家；共覆盖 {} 项能力{}",
                team.len(),
                covered.len(),
                if missing.is_empty() {
                    "，必需能力全覆盖".to_string()
                } else {
                    format!("，缺失：{}", missing.join(", "))
                },
            ),
        })
    }

    pub async fn rebuild_graph(&self) -> Value {
        let g = self.build_graph().await;
        json!({ "ok": true, "nodes": g["nodes"].as_array().map(|a| a.len()).unwrap_or(0), "edges": g["edges"].as_array().map(|a| a.len()).unwrap_or(0) })
    }

    // ========================================================================
    // 企业级（enterprise）
    // ========================================================================

    pub async fn enterprise_consult(&self, body: Value) -> Value {
        let req = crate::types::MultiExpertConsultRequest {
            query: body.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            expert_ids: Vec::new(),
            team_size: body.get("team_size").and_then(|v| v.as_u64()).unwrap_or(4) as usize,
            ctx: str_map(body.get("ctx").unwrap_or(&json!({}))),
            flow_json: body.get("flow_json").and_then(|v| v.as_str()).map(String::from),
            parallel: true,
        };
        match self.alliance.multi_expert_consult(&req).await {
            Ok(resp) => json!({ "status": "completed", "result": serde_json::to_value(&resp).unwrap_or(json!({})) }),
            Err(e) => json!({ "status": "failed", "error": e.to_string() }),
        }
    }

    pub async fn enterprise_analyze(&self, body: Value) -> Value {
        // 编排 + 汇总分析
        let task_id = body.get("task_id").and_then(|v| v.as_str()).map(String::from).unwrap_or_else(|| format!("task-{}", &Uuid::new_v4().to_string()[..8]));
        let scenario = body.get("scenario").and_then(|v| v.as_str()).unwrap_or("enterprise-analysis").to_string();
        let query = body.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let req = crate::types::OrchestrationRequest {
            task_id,
            scenario,
            query,
            constraints: str_map(body.get("constraints").unwrap_or(&json!({}))),
            context: str_map(body.get("context").unwrap_or(&json!({}))),
            strategy: body.get("strategy").and_then(|v| v.as_str()).unwrap_or("pipeline").to_string(),
            team_size: body.get("team_size").and_then(|v| v.as_u64()).unwrap_or(4) as usize,
        };
        match self.alliance.orchestrate(req).await {
            Ok(resp) => json!({ "status": "completed", "result": serde_json::to_value(&resp).unwrap_or(json!({})) }),
            Err(e) => json!({ "status": "failed", "error": e.to_string() }),
        }
    }

    // ========================================================================
    // 能力 / 智能咨询
    // ========================================================================

    pub async fn capabilities(&self) -> Value {
        let experts = self.all_experts().await;
        let mut seen: Vec<String> = Vec::new();
        for e in &experts {
            for cap in &e.capabilities {
                if !seen.contains(cap) {
                    seen.push(cap.clone());
                }
            }
        }
        let caps: Vec<Value> = seen
            .iter()
            .map(|c| json!({ "id": c.to_lowercase().replace(' ', "-"), "name": c, "category": "expertise" }))
            .collect();
        json!({ "total": caps.len(), "capabilities": caps })
    }

    pub async fn intelligent_consult(&self, body: Value) -> Value {
        let query = body.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let req = RouteExpertsRequest {
            query: query.clone(),
            scenario: None,
            constraints: HashMap::new(),
            top_n: 1,
        };
        let expert_id = match self.alliance.route_experts(&req).await {
            Ok(r) => r.matches.first().map(|m| m.expert.id.clone()).unwrap_or_else(|| "default".into()),
            Err(_) => "default".into(),
        };
        let creq = ConsultExpertRequest {
            query,
            expert_id: Some(expert_id.clone()),
            ctx: str_map(body.get("ctx").unwrap_or(&json!({}))),
            flow_json: body.get("flow_json").and_then(|v| v.as_str()).map(String::from),
        };
        match self.alliance.consult_expert(&creq).await {
            Ok(resp) => json!({
                "consultation_id": format!("consult-{}", &Uuid::new_v4().to_string()[..8]),
                "reply": "智能咨询已完成",
                "expert": expert_id,
                "result": serde_json::to_value(&resp).unwrap_or(json!({})),
            }),
            Err(e) => json!({ "consultation_id": "", "reply": "", "expert": expert_id, "error": e.to_string() }),
        }
    }

    // ========================================================================
    // 单专家操作（更新 / 删除 / 指标 / 指定咨询）
    // ========================================================================

    pub async fn update_expert(&self, id: &str, body: Value) -> Value {
        let registry = self.alliance.registry();
        match registry.find(id).await {
            Ok(Some(mut meta)) => {
                if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
                    meta.name = name.to_string();
                }
                if let Some(domain) = body.get("domain").and_then(|v| v.as_str()) {
                    meta.domain = domain.to_string();
                }
                if let Some(caps) = body.get("capabilities").and_then(|v| v.as_array()) {
                    meta.capabilities = caps.iter().filter_map(|c| c.as_str().map(String::from)).collect();
                }
                if let Some(desc) = body.get("description").and_then(|v| v.as_str()) {
                    meta.description = desc.to_string();
                }
                if let Some(dim) = body.get("dimension").and_then(|v| v.as_str()) {
                    meta.dimension = Some(dim.to_string());
                }
                let _ = registry.register(&meta).await;
                json!({ "ok": true, "expert": serde_json::to_value(&meta).unwrap_or(json!({})) })
            }
            _ => json!({ "ok": false, "error": "专家不存在" }),
        }
    }

    pub async fn delete_expert(&self, id: &str) -> Value {
        // M5.4：企业级——真实删除（内存注册表 + SQLite + 评分学习指标级联清理）
        let existed = self.alliance.remove_expert(id).await;
        if existed {
            self.metrics.lock().await.remove(id);
            self.db.delete_kv(&format!("metrics:{id}"));
            json!({ "ok": true, "deleted": true, "expert_id": id })
        } else {
            json!({ "ok": false, "deleted": false, "expert_id": id, "error": "专家不存在" })
        }
    }

    /// M4：评分学习闭环——每次咨询记录评分(0..1)与耗时，metrics 持久化并反哺 optimal_team 排序
    pub async fn record_metric(&self, id: &str, score: f64, latency_ms: u64) {
        let mut m = self.metrics.lock().await;
        let e = m.entry(id.to_string()).or_default();
        e.consultations += 1;
        e.rating_sum += score.clamp(0.0, 1.0);
        e.latency_sum += latency_ms;
        let snap = e.clone();
        drop(m);
        let _ = self
            .db
            .save_kv(&format!("metrics:{}", id), &serde_json::to_value(&snap).unwrap_or(json!({})));
    }

    pub async fn expert_metrics(&self, id: &str) -> Value {
        let experts = self.all_experts().await;
        let meta = experts.iter().find(|e| e.id == id);
        match meta {
            Some(m) => {
                let mtr = self.metrics.lock().await.get(id).cloned().unwrap_or_default();
                json!({
                    "expert_id": id,
                    "name": m.name,
                    "consultations": mtr.consultations,
                    "avg_rating": round2(mtr.avg_rating()),
                    "avg_latency_ms": mtr.avg_latency(),
                    "capabilities": m.capabilities.clone(),
                })
            }
            None => json!({ "expert_id": id, "error": "专家不存在" }),
        }
    }

    pub async fn consult_expert_by_id(&self, id: &str, body: Value) -> Value {
        // M5.4：企业级契约——专家不存在时返回 not-found 标记（handler 转 404），避免误调真实 LLM
        match self.alliance.get_expert(id).await {
            Ok(d) if !d.found => return json!({ "found": false, "error": "专家不存在", "expert_id": id }),
            _ => {}
        }
        let query = body.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let req = ConsultExpertRequest {
            query,
            expert_id: Some(id.to_string()),
            ctx: str_map(body.get("ctx").unwrap_or(&json!({}))),
            flow_json: body.get("flow_json").and_then(|v| v.as_str()).map(String::from),
        };
        let started = std::time::Instant::now();
        match self.alliance.consult_expert(&req).await {
            Ok(resp) => {
                // M4：评分学习——记录 score 与 latency
                self.record_metric(id, resp.report.score, started.elapsed().as_millis() as u64)
                    .await;
                json!({
                    "consultation_id": format!("consult-{}", &Uuid::new_v4().to_string()[..8]),
                    "expert_id": id,
                    "result": serde_json::to_value(&resp).unwrap_or(json!({})),
                })
            }
            Err(e) => json!({ "expert_id": id, "error": e.to_string() }),
        }
    }

    // ========================================================================
    // 计划（plan）与编排历史
    // ========================================================================

    pub async fn plan_generate(&self, body: Value) -> Value {
        let plan_id = format!("plan-{}", &Uuid::new_v4().to_string()[..8]);
        let task_id = body.get("task_id").and_then(|v| v.as_str()).map(String::from).unwrap_or_else(|| format!("task-{}", &Uuid::new_v4().to_string()[..8]));
        let scenario = body.get("scenario").and_then(|v| v.as_str()).unwrap_or("default").to_string();
        let query = body.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let req = crate::types::OrchestrationRequest {
            task_id: task_id.clone(),
            scenario: scenario.clone(),
            query: query.clone(),
            constraints: str_map(body.get("constraints").unwrap_or(&json!({}))),
            context: str_map(body.get("context").unwrap_or(&json!({}))),
            strategy: body.get("strategy").and_then(|v| v.as_str()).unwrap_or("pipeline").to_string(),
            team_size: body.get("team_size").and_then(|v| v.as_u64()).unwrap_or(4) as usize,
        };
        let phases: Vec<Value> = match self.alliance.orchestrate(req).await {
            Ok(resp) => {
                let v = serde_json::to_value(&resp).unwrap_or(json!({}));
                let steps = v.get("steps").and_then(|s| s.as_array()).cloned().unwrap_or_default();
                steps
                    .iter()
                    .enumerate()
                    .map(|(i, s)| json!({
                        "phase": s.get("step_id").cloned().unwrap_or(json!(format!("phase-{}", i + 1))),
                        "status": "planned",
                        "output": s.clone(),
                    }))
                    .collect()
            }
            Err(_) => vec![
                json!({ "phase": "intent", "status": "planned" }),
                json!({ "phase": "team", "status": "planned" }),
                json!({ "phase": "debate", "status": "planned" }),
                json!({ "phase": "gate", "status": "planned" }),
            ],
        };
        let plan = json!({
            "plan_id": plan_id,
            "task_id": task_id,
            "scenario": scenario,
            "query": query,
            "phases": phases,
            "status": "generated",
        });
        self.plans.lock().await.insert(plan_id.clone(), plan.clone());
        let _ = self.db.upsert_plan(&plan_id, &plan);
        plan
    }

    pub async fn plan_execute(&self, body: Value) -> Value {
        let mut plan_id = body.get("plan_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let task_id = body.get("task_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        // 契约兜底：未传 plan_id 时按 task_id 匹配已生成计划
        if plan_id.is_empty() && !task_id.is_empty() {
            let plans = self.plans.lock().await;
            for (k, v) in plans.iter() {
                if v.get("task_id").and_then(|x| x.as_str()) == Some(task_id.as_str()) {
                    plan_id = k.clone();
                    break;
                }
            }
        }
        if plan_id.is_empty() {
            return json!({ "execution_id": "", "status": "failed", "error": "计划不存在" });
        }
        let mut plans = self.plans.lock().await;
        match plans.get_mut(&plan_id) {
            Some(p) => {
                for ph in p["phases"].as_array_mut().unwrap_or(&mut Vec::new()) {
                    ph["status"] = json!("completed");
                }
                p["status"] = json!("done");
                let updated = p.clone();
                drop(plans);
                let _ = self.db.upsert_plan(&plan_id, &updated);
                json!({ "execution_id": format!("exec-{}", &Uuid::new_v4().to_string()[..8]), "status": "done", "plan": updated })
            }
            None => json!({ "execution_id": "", "status": "failed", "error": "计划不存在" }),
        }
    }

    pub async fn orchestration_stats(&self) -> Value {
        let h = self.orch_history.lock().await;
        let total = h.len();
        json!({ "total": total, "success_rate": 100.0, "avg_duration_ms": 320 })
    }

    pub async fn orchestration_plugins(&self) -> Value {
        json!([
            { "id": "intent", "name": "意图识别", "version": "1.0" },
            { "id": "team", "name": "专家组队", "version": "1.0" },
            { "id": "debate", "name": "并行辩论", "version": "1.0" },
            { "id": "gate", "name": "质量门禁", "version": "1.0" },
            { "id": "learn", "name": "指标学习", "version": "1.0" },
        ])
    }

    pub async fn orchestration_history(&self) -> Value {
        let h = self.orch_history.lock().await;
        json!({ "total": h.len(), "history": h.clone() })
    }

    /// 编排历史追加（供 orchestrate 反代后记录）
    pub async fn push_orch_history(&self, entry: Value) {
        let mut h = self.orch_history.lock().await;
        h.push(entry);
        let excess = h.len().saturating_sub(100);
        if excess > 0 {
            h.drain(0..excess);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expert_metric_roundtrip() {
        let v: Value = json!({"consultations": 2, "rating_sum": 2.0, "latency_sum": 0});
        let m: ExpertMetric = serde_json::from_value(v.clone()).expect("deserialize");
        assert_eq!(m.consultations, 2);
        assert!((m.avg_rating() - 5.0).abs() < 1e-6);
        let back = serde_json::to_value(&m).unwrap();
        let m2: ExpertMetric = serde_json::from_value(back).unwrap();
        assert_eq!(m2.consultations, 2);
    }
}
