// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # AI 自动化中枢（编排 + REST API）
//!
//! 把「对话 → 业务处理流程图 + 功能逻辑细节 + 关联关系 + 权限 →
//! 自动代码 → 自动测试 → 沙箱实跑异常自动修复 → 回写保存 → 可继续编辑」
//! 串成端到端闭环，并向前端暴露一组 REST 接口。
//!
//! 路由前缀 `/api/automation`：
//! - `POST /chat`            需求对话：生成蓝图 + 流程图 + 全栈代码 + 自动测试 + RBAC，并落盘
//! - `POST /:id/refine`      在已有自动化资产上继续对话迭代（增量补功能）
//! - `POST /:id/run`         沙箱实跑生成的 Python，自动分析异常并修复回写
//! - `GET  /:id/permissions` 查看从蓝图自动推导的 RBAC 角色-权限映射
//! - `GET  /`                列出所有自动化资产
//!
//! 资产模型与持久化在 [`crate::automation_asset`]（独立模块，避免循环依赖）。

use crate::automation_asset::{AutomationAsset, GeneratedCode, RunRecord};
use crate::rbac_middleware::{check_permission, Permission, Principal};
use crate::AppState;
use mox_ai_agent_svc::requirement_compiler::SystemBlueprint;
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use mox_ai_flow_svc::automation::{
    AutoTestGen, BusinessBlueprintLite, ErrorAnalyzer, Feature, FixProposal, RbacDeriver,
    RolePermission, RunResult,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::Command;

// ============================================================================
// 请求/响应结构
// ============================================================================

/// 需求对话请求
#[derive(Debug, Deserialize)]
pub struct AutomationChatRequest {
    /// 一句话需求（如 "做一个商城，有商品、购物车、下单、支付"）
    pub requirement: String,
    /// 资产名称（可选，缺省用需求截断）
    pub name: Option<String>,
    /// 分类标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 会话 id（多轮对话用）
    pub session_id: Option<String>,
}

/// 需求对话响应
#[derive(Debug, Serialize)]
pub struct AutomationChatResponse {
    pub asset_id: String,
    pub name: String,
    pub blueprint_summary: BlueprintSummary,
    pub code_files: Vec<String>,
    pub test_count: usize,
    pub rbac_count: usize,
    pub mermaid: String,
    /// 自动生成的全栈代码全文（供前端展示/编辑）
    pub code: GeneratedCode,
}

#[derive(Debug, Serialize)]
pub struct BlueprintSummary {
    pub feature_count: usize,
    pub entity_count: usize,
    pub node_count: usize,
    pub edge_count: usize,
    pub features: Vec<String>,
}

/// 运行请求（可选：沙箱超时秒数）
#[derive(Debug, Deserialize)]
pub struct RunRequest {
    pub timeout_sec: Option<u64>,
}

/// 运行响应
#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub asset_id: String,
    pub run: RunRecord,
    pub fix: Option<FixSummary>,
    pub updated_code_python: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FixSummary {
    pub category: String,
    pub note: String,
    pub applied: bool,
    /// 修复来源：rule（规则兜底）| llm（大模型生成）| none（仅给出提案）
    pub source: String,
}

/// 权限响应
#[derive(Debug, Serialize)]
pub struct PermissionsResponse {
    pub roles: Vec<String>,
    pub permissions: Vec<RolePermission>,
}

// ============================================================================
// 编排核心
// ============================================================================

/// 从需求生成一份完整的自动化资产（蓝图 + 代码 + 测试 + RBAC）
async fn build_asset(
    state: &AppState,
    requirement: &str,
    name: &str,
    tags: Vec<String>,
) -> anyhow::Result<AutomationAsset> {
    // 1) 需求 → 蓝图
    let bp = state
        .ai_agent
        .compile_requirement(requirement, name, tags.clone())
        .await?;

    // 2) 蓝图 → 全栈代码（本地生成，解耦 flow-ai 内部 model 类型）
    let code = generate_code_from_blueprint(&bp);

    // 3) 自动测试（针对生成的 python 主入口）
    let tests = vec![AutoTestGen::generate(&code.python, "flow_app", "main")];

    // 4) RBAC 推导：把 SystemBlueprint 投影为 Lite 形态
    let lite = blueprint_to_lite(&bp);
    let (_roles, rbac) = RbacDeriver::derive(&lite);

    let now = chrono::Utc::now().to_rfc3339();
    Ok(AutomationAsset {
        id: bp.id.clone(),
        name: bp.name.clone(),
        description: bp.description.clone(),
        tags: bp.tags.clone(),
        blueprint: bp,
        code,
        tests,
        rbac,
        run_history: vec![],
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 把 `SystemBlueprint` 投影为 RBAC 推导所需的 Lite 形态
fn blueprint_to_lite(bp: &SystemBlueprint) -> BusinessBlueprintLite {
    BusinessBlueprintLite {
        id: bp.id.clone(),
        name: bp.name.clone(),
        tags: bp.tags.clone(),
        features: bp
            .features
            .iter()
            .map(|f| Feature {
                id: f.id.clone(),
                name: f.name.clone(),
                action: f.action.clone(),
                entities: f.entities.clone(),
                depends_on: f.depends_on.clone(),
            })
            .collect(),
        entities: bp.entities.clone(),
    }
}

/// 从业务蓝图生成全栈代码（Python 主流程 + SQL DDL + Vue 前端骨架）。
/// 直接消费 `SystemBlueprint` 字段，避免与 flow-ai 内部 model 类型耦合。
fn generate_code_from_blueprint(bp: &SystemBlueprint) -> GeneratedCode {
    // 预分配唯一标识符：功能点函数名 + 实体表名
    let mut used = std::collections::HashSet::new();
    let mut fn_names: Vec<String> = Vec::new();
    for f in &bp.features {
        fn_names.push(make_ident(&f.name, &mut used));
    }
    let mut table_names: Vec<String> = Vec::new();
    for entity in bp.entities.keys() {
        table_names.push(make_ident(entity, &mut used));
    }

    let mut py = String::new();
    py.push_str("#!/usr/bin/env python3\n");
    py.push_str("# 由 OUS AI 自动化中枢依据业务蓝图自动生成\n");
    py.push_str("import json\nfrom typing import Any, Dict\n\n\n");
    py.push_str("def _ctx() -> Dict[str, Any]:\n    \"\"\"全局业务上下文（可按需持久化到 DB）。\"\"\"\n    return {}\n\n\n");
    for (i, f) in bp.features.iter().enumerate() {
        let fn_name = &fn_names[i];
        py.push_str(&format!(
            "def {fn}(ctx: Dict[str, Any]) -> Dict[str, Any]:\n    \"\"\"{desc}（{action}）\"\"\"\n    import time\n    _start = time.monotonic()\n    try:\n        # 真实处理：登记调用记录并统计输入键（业务细节由上层编排注入）\n        _keys = sorted(ctx.keys())\n        ctx.setdefault(\"{fn}\", {{\"calls\": 0}})\n        ctx[\"{fn}\"][\"calls\"] += 1\n        return {{\"ok\": True, \"feature\": \"{fn}\", \"input_keys\": _keys, \"elapsed_ms\": round((time.monotonic() - _start) * 1000, 3)}}\n    except Exception as e:\n        return {{\"ok\": False, \"error\": str(e), \"elapsed_ms\": round((time.monotonic() - _start) * 1000, 3)}}\n\n\n",
            fn = fn_name,
            desc = f.name,
            action = if f.action.is_empty() { f.name.as_str() } else { f.action.as_str() },
        ));
    }
    py.push_str("def main() -> None:\n    ctx = _ctx()\n");
    for (i, _f) in bp.features.iter().enumerate() {
        let fn_name = &fn_names[i];
        py.push_str(&format!(
            "    print({}.__name__, {}(ctx))\n",
            fn_name, fn_name
        ));
    }
    py.push_str("\n\nif __name__ == \"__main__\":\n    main()\n");

    let mut sql = String::new();
    sql.push_str("-- 由 OUS AI 自动化中枢依据业务蓝图自动生成\n");
    for (i, (_entity, fields)) in bp.entities.iter().enumerate() {
        let table = sql_safe_ident(&table_names[i]);
        sql.push_str(&format!("CREATE TABLE IF NOT EXISTS {} (\n", table));
        if fields.is_empty() {
            sql.push_str("    id BIGINT PRIMARY KEY AUTO_INCREMENT,\n");
            sql.push_str("    data VARCHAR(255),\n");
            sql.push_str("    created_at DATETIME DEFAULT CURRENT_TIMESTAMP\n);\n\n");
        } else {
            // 去重：蓝图字段可能已含 id/created_at，不再硬编码叠加
            let mut seen = std::collections::HashSet::new();
            for col in fields {
                let c = sql_safe_ident(&make_col_ident(col));
                if seen.insert(c.clone()) {
                    sql.push_str(&format!("    {} VARCHAR(255),\n", c));
                }
            }
            // 追加 created_at（若蓝图已含则跳过）
            if seen.insert("created_at".to_string()) {
                sql.push_str("    created_at DATETIME DEFAULT CURRENT_TIMESTAMP\n");
            } else {
                // 去掉末尾多余逗号
                if sql.ends_with(",\n") {
                    sql.truncate(sql.len() - 2);
                    sql.push('\n');
                }
            }
            sql.push_str(");\n\n");
        }
    }

    let mut vue = String::new();
    vue.push_str("<template>\n  <div class=\"auto-app\">\n");
    vue.push_str(&format!("    <h1>{}</h1>\n", bp.name));
    vue.push_str("    <ul>\n");
    for f in &bp.features {
        vue.push_str(&format!(
            "      <li>{{{{ '{name}' }}}}<button @click=\"run('{id}')\">执行</button></li>\n",
            name = f.name,
            id = f.id
        ));
    }
    vue.push_str("    </ul>\n  </div>\n</template>\n\n<script setup>\n");
    vue.push_str("import { ref } from 'vue'\n");
    vue.push_str("const features = ref([])\n");
    vue.push_str("async function run(id) {\n  // 真实调用后端自动化执行接口\n  const res = await fetch(`/api/automation/${id}/run`, { method: 'POST' })\n  const data = await res.json()\n  console.log('run', id, data)\n}\n");
    vue.push_str("</script>\n");

    GeneratedCode {
        python: py,
        sql,
        vue,
    }
}

/// 常见中文业务词 → 英文标识符映射（命中则产出可读英文，否则回退序号标识）
const ZH_TO_IDENT: &[(&str, &str)] = &[
    ("商城", "mall"),
    ("商品", "product"),
    ("用户", "user"),
    ("会员", "member"),
    ("订单", "order"),
    ("购物车", "cart"),
    ("下单", "place_order"),
    ("支付", "pay"),
    ("退货", "refund"),
    ("评论", "comment"),
    ("文章", "article"),
    ("博客", "blog"),
    ("积分", "point"),
    ("等级", "level"),
    ("签到", "checkin"),
    ("兑换", "exchange"),
    ("工单", "ticket"),
    ("提交", "submit"),
    ("分配", "assign"),
    ("处理", "handle"),
    ("关闭", "close"),
    ("点赞", "like"),
    ("收藏", "favorite"),
    ("关注", "follow"),
    ("消息", "message"),
    ("通知", "notify"),
    ("库存", "inventory"),
    ("物流", "logistics"),
    ("地址", "address"),
    ("分类", "category"),
    ("评价", "rate"),
    ("搜索", "search"),
    ("推荐", "recommend"),
    ("审核", "review"),
    ("登录", "login"),
    ("注册", "register"),
    ("上传", "upload"),
    ("下载", "download"),
    ("导出", "export"),
    ("导入", "import"),
];

/// 把任意名称清洗为唯一的合法 snake_case 标识符。
/// 策略：命中中文映射表用映射英文；否则英文/数字保留、其它转 `_`；
/// 再以 `used` 集合保证全局唯一（重名追加序号）。
fn make_ident(raw: &str, used: &mut std::collections::HashSet<String>) -> String {
    let base = if raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        raw.to_ascii_lowercase()
    } else {
        // 含中文：逐词匹配映射表，拼接产出可读英文
        let mut mapped = String::new();
        for (zh, en) in ZH_TO_IDENT {
            if raw.contains(zh) {
                mapped.push_str(en);
            }
        }
        if mapped.is_empty() {
            // 未命中：用名称哈希式占位，保证非空且与中文脱钩
            format!(
                "node_{}",
                raw.chars()
                    .fold(0u32, |a, c| a.wrapping_mul(31).wrapping_add(c as u32))
                    % 100000
            )
        } else {
            mapped
        }
    };
    let mut candidate = if base.is_empty() || base.chars().next().unwrap().is_ascii_digit() {
        format!("_{}", base)
    } else {
        base.clone()
    };
    let mut n = 2;
    while used.contains(&candidate) {
        candidate = format!("{}_{}", base, n);
        n += 1;
    }
    used.insert(candidate.clone());
    candidate
}

/// 列名清洗（字段通常为英文/拼音，少量中文回退为 _）
fn make_col_ident(col: &str) -> String {
    let mut out = String::new();
    for c in col.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("col");
    }
    out
}

/// SQL 保留字（作为表名/列名需加前缀规避）
const SQL_RESERVED: &[&str] = &[
    "order", "group", "select", "from", "where", "table", "index", "key", "user", "level",
    "comment", "desc", "asc", "limit", "offset", "primary", "foreign",
];

/// 把标识符处理为合法的 SQL 标识符（保留字加 t_/col_ 前缀；首字符数字加 _）
fn sql_safe_ident(ident: &str) -> String {
    let lower = ident.to_ascii_lowercase();
    if SQL_RESERVED.contains(&lower.as_str()) {
        // 表名加 t_，列名加 col_ 预判别：表名场景已在调用处决定前缀，这里统一加 t_ 仅当
        // 原样为保留字且不包含已有前缀
        if lower.starts_with("t_") || lower.starts_with("col_") {
            return lower;
        }
        return format!("t_{}", lower);
    }
    if lower
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        return format!("_{}", lower);
    }
    lower
}

/// 沙箱实跑 Python 代码：写入临时目录、强制墙钟超时 + kill_on_drop、捕获输出。
/// 使用 tokio::process 异步执行，避免阻塞 tokio worker 线程；超出墙钟后 kill_on_drop 终止子进程。
async fn run_python_sandbox(code: &str, timeout: Duration) -> RunResult {
    // 解释器可配置（默认 python3；Windows 上常为 python）
    let python_bin = std::env::var("OUS_PYTHON").unwrap_or_else(|_| "python3".to_string());

    let dir = std::env::temp_dir().join(format!("ous_auto_{}", uuid::Uuid::new_v4().simple()));
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join("flow_app.py");
    if let Err(e) = std::fs::write(&file, code) {
        return RunResult {
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("写入沙箱失败: {}", e),
            timed_out: false,
        };
    }

    let start = Instant::now();
    // tokio::time::timeout 包裹 output()：到点返回 Err 并丢弃未来 → kill_on_drop 终止子进程，
    // 从而杜绝 `while True` 失控进程无限占用 CPU / 阻塞主机（此前仅事后测量，不真正 kill）。
    let out = tokio::time::timeout(
        timeout,
        Command::new(&python_bin)
            .arg(file.to_str().unwrap())
            .current_dir(&dir)
            .kill_on_drop(true)
            .output(),
    )
    .await;

    let result = match out {
        Ok(Ok(o)) => {
            let elapsed = start.elapsed();
            RunResult {
                exit_code: o.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&o.stdout).to_string(),
                stderr: String::from_utf8_lossy(&o.stderr).to_string(),
                timed_out: elapsed > timeout,
            }
        }
        Ok(Err(e)) => RunResult {
            // exit_code 9009 = Windows "命令未找到"；标记为环境错误，避免无意义的代码修复
            exit_code: 9009,
            stdout: String::new(),
            stderr: format!(
                "[环境错误] 无法启动 Python 解释器 `{}`：{}。请确认沙箱主机已安装并设置 OUS_PYTHON。",
                python_bin, e
            ),
            timed_out: false,
        },
        Err(_) => RunResult {
            exit_code: -1,
            stdout: String::new(),
            stderr: format!(
                "[超时] Python 沙箱执行超过 {}ms，已强制终止（防止失控进程拖垮主机）",
                timeout.as_millis()
            ),
            timed_out: true,
        },
    };

    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// 是否为沙箱运行环境错误（如解释器缺失），这类不应触发代码层面的 AI 修复
fn is_env_error(run: &RunResult) -> bool {
    run.exit_code == 9009
        || run.stderr.contains("[环境错误]")
        || run.stderr.to_lowercase().contains("failed to spawn")
}

/// 异常修复：先规则兜底，失败且 LLM 可用时调用大模型生成修复代码
async fn try_fix(
    state: &AppState,
    run: &RunResult,
    original_code: &str,
) -> Option<(FixProposal, String, bool, String)> {
    // 规则兜底（KeyError / ImportError / ZeroDivisionError 等低风险类别）
    if let Some(fixed) = ErrorAnalyzer::rule_based_fix(run, original_code) {
        if let Some(prop) = ErrorAnalyzer::analyze(run, original_code) {
            return Some((prop, fixed, true, "rule".to_string()));
        }
    }
    // LLM 修复
    let llm = state.ai_agent.llm_client();
    let llm_guard = llm.read().await;
    if llm_guard.is_enabled() {
        if let Some(prop) = ErrorAnalyzer::analyze(run, original_code) {
            let msgs = vec![mox_ai_agent_svc::LLMChatMessage {
                role: "user".to_string(),
                content: prop.llm_prompt.clone(),
            }];
            drop(llm_guard);
            if let Ok(fixed) = llm.read().await.chat(msgs).await {
                let code = extract_code_block(&fixed).unwrap_or(fixed);
                return Some((prop, code, true, "llm".to_string()));
            }
        }
    }
    // 无法自动修复：仅给出提案说明
    if let Some(prop) = ErrorAnalyzer::analyze(run, original_code) {
        return Some((prop, original_code.to_string(), false, "none".to_string()));
    }
    None
}

/// 从 LLM 返回文本中抽取 python 代码块
fn extract_code_block(text: &str) -> Option<String> {
    let start = text.find("```python")?;
    let after = &text[start + 9..];
    let end = after.find("```")?;
    Some(after[..end].trim().to_string())
}

/// 流程图 → Mermaid（独立实现，避免与 mox_ai_flow_svc 内部 model 类型耦合）
pub fn flow_definition_to_mermaid(flow: &mox_ai_agent_svc::flow_engine::FlowDefinition) -> String {
    use mox_ai_agent_svc::flow_engine::NodeType;
    let mut s = String::from("flowchart TD\n");
    for n in &flow.nodes {
        let id = n.id.replace(['-', ' '], "_");
        let label = n.name.replace('"', "'");
        let shape = match n.node_type {
            NodeType::Start => format!("{}[(\"{} • 开始\")]", id, label),
            NodeType::End => format!("{}[(\"{} • 结束\")]", id, label),
            NodeType::Decision | NodeType::Condition => format!("{{{{ \"{}\" }}}}", label),
            NodeType::Guard => format!("{}[[\"{}\"]]", id, label),
            NodeType::Parallel => format!("{}[/\"{}\"/]", id, label),
            _ => format!("{}[\"{}\"]", id, label),
        };
        s.push_str(&format!("    {}\n", shape));
    }
    for e in &flow.edges {
        let src = e.source.replace(['-', ' '], "_");
        let tgt = e.target.replace(['-', ' '], "_");
        let cond = e
            .condition
            .as_ref()
            .map(|c| format!(" |{}|", c.replace('"', "'")))
            .unwrap_or_default();
        s.push_str(&format!("    {}{}-->{}\n", src, cond, tgt));
    }
    s
}

// ============================================================================
// HTTP 处理器
// ============================================================================

/// 需求对话：生成并落盘
///
/// RBAC 闸门：需 [`Permission::EditFlow`]（Editor/Admin）方可提交需求编译。
/// 访客（Viewer/无 token）→ 403。
pub async fn chat_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<Principal>,
    Json(req): Json<AutomationChatRequest>,
) -> Result<Json<AutomationChatResponse>, (StatusCode, String)> {
    // ── RBAC 闸门：Editor+ 方可提交需求编译 ──
    if !check_permission(&principal.roles, &Permission::EditFlow) {
        tracing::warn!(
            target: "automation",
            token_id = %principal.token_id,
            roles = ?principal.roles,
            "RBAC denied: EditFlow required for compile"
        );
        return Err((
            StatusCode::FORBIDDEN,
            "权限不足：需 Editor 角色以上才可提交需求编译".into(),
        ));
    }

    let name = req
        .name
        .clone()
        .unwrap_or_else(|| req.requirement.chars().take(20).collect());
    let asset = build_asset(&state, &req.requirement, &name, req.tags.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
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

    crate::automation_asset::save_automation(asset)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(resp))
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
) -> Result<Json<AutomationChatResponse>, (StatusCode, String)> {
    // ── RBAC 闸门：Editor+ 方可追加功能 ──
    if !check_permission(&principal.roles, &Permission::EditFlow) {
        tracing::warn!(
            target: "automation",
            token_id = %principal.token_id,
            roles = ?principal.roles,
            "RBAC denied: EditFlow required for refine"
        );
        return Err((
            StatusCode::FORBIDDEN,
            "权限不足：需 Editor 角色以上才可追加功能".into(),
        ));
    }

    let mut asset = crate::automation_asset::get_automation(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "自动化资产不存在".into()))?;

    let bp = state
        .ai_agent
        .refine_blueprint(&asset.blueprint.id, &req.requirement)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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
    crate::automation_asset::save_automation(asset.clone())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

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
    Ok(Json(AutomationChatResponse {
        asset_id: asset.id.clone(),
        name: asset.name.clone(),
        blueprint_summary: summary,
        code_files: vec!["flow_app.py".into(), "schema.sql".into(), "App.vue".into()],
        test_count: asset.tests.len(),
        rbac_count: asset.rbac.len(),
        mermaid,
        code: asset.code.clone(),
    }))
}

/// 沙箱实跑 + 异常自动修复回写
pub async fn run_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<RunRequest>,
) -> Result<Json<RunResponse>, (StatusCode, String)> {
    let mut asset = crate::automation_asset::get_automation(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "自动化资产不存在".into()))?;

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
            crate::automation_asset::save_automation(asset)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
            return Ok(Json(RunResponse {
                asset_id: id,
                run: record,
                fix: None,
                updated_code_python: None,
            }));
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
    crate::automation_asset::save_automation(asset)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(RunResponse {
        asset_id: id,
        run: record,
        fix: fix_summary,
        updated_code_python: updated_code,
    }))
}

/// 保存前端编辑结果（代码 + 可选流程图），实现「可继续编辑流程」
pub async fn update_handler(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateAutomationRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let mut asset = crate::automation_asset::get_automation(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "自动化资产不存在".into()))?;

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
    crate::automation_asset::save_automation(asset)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::json!({ "ok": true, "id": id })))
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
) -> Result<Json<PermissionsResponse>, (StatusCode, String)> {
    let asset = crate::automation_asset::get_automation(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "自动化资产不存在".into()))?;

    let lite = blueprint_to_lite(&asset.blueprint);
    let (roles, perms) = RbacDeriver::derive(&lite);
    Ok(Json(PermissionsResponse {
        roles,
        permissions: perms,
    }))
}

/// 列出所有自动化资产（轻量摘要）
pub async fn list_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<Value>>, (StatusCode, String)> {
    let assets = crate::automation_asset::list_automations()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
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
    Ok(Json(summaries))
}

// ============================================================================
// 路由挂载
// ============================================================================

/// 返回 `/api/automation` 路由树
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_handler))
        .route("/chat", post(chat_handler))
        .route("/:id", put(update_handler))
        .route("/:id/refine", post(refine_handler))
        .route("/:id/run", post(run_handler))
        .route("/:id/permissions", get(permissions_handler))
}
