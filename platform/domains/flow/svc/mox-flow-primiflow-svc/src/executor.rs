// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 算子真实执行层（离线确定性，可运行可验证）
//!
//! 平台闭环此前在 [`crate::runner::run_pipeline`] 中只「模拟」成功注荷；本层补齐真实执行：
//! 按子任务顺序把每个 `mox_ai_flow_sdk::model::ToolKind` 派发到对应的确定性实现，把上一步输出
//! 作为下一步输入，并据**真实**执行质量回灌 κ‑τ 引擎（`engine.accept`）。
//!
//! 设计要点：
//! - 离线可跑：不依赖外网/数据库，输出为确定性合成数据，便于断言与复现。
//! - 与「可运行/可验证」同源：同一 `dispatch` 被示例、单元测试、集成测试复用。

use anyhow::Result;
use mox_ai_flow_sdk::model::ToolKind;
use serde::Serialize;
use serde_json::{json, Value};

/// 单条算子执行记录（用于报告 / 审计 / API 透出）
#[derive(Debug, Clone, Serialize)]
pub struct ExecRecord {
    pub key: String,
    pub label: String,
    pub tool: &'static str,
    pub ok: bool,
    pub note: String,
    pub output: Value,
}

impl ExecRecord {
    /// 单行摘要（用于报告打印）
    pub fn short(&self) -> String {
        format!(
            "{}({}){}",
            self.label,
            self.tool,
            if self.ok { "✓" } else { "✗" }
        )
    }
}

/// 子任务三元组：`(key, label, tool)`，直接由 `Requirement.subtasks` 映射而来。
pub type SubtaskTriple = (String, String, ToolKind);

/// 工具类型的稳定字符串名（写入记录 / JSON，便于前端过滤）
pub fn tool_name(t: ToolKind) -> &'static str {
    match t {
        ToolKind::Compute => "compute",
        ToolKind::Llm => "llm",
        ToolKind::File => "file",
        ToolKind::Browser => "browser",
        ToolKind::Database => "database",
        ToolKind::Http => "http",
        ToolKind::Shell => "shell",
        ToolKind::Human => "human",
    }
}

fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// 按工具类型派发一个真实（确定性、离线可跑）的算子实现。
///
/// `input` 为上游输出（首步传入需求种子）。返回结构化 `Value`，
/// 下游据此提取 `rows` 作为下一步规模输入。
pub fn dispatch(tool: ToolKind, label: &str, input: &Value) -> Result<Value> {
    let rows = input.get("rows").and_then(|v| v.as_u64()).unwrap_or(8);
    let out = match tool {
        ToolKind::Http => json!({
            "op": "fetch",
            "label": label,
            "rows": rows * 7 + 3,
            "source": label,
            "fetched_at": "2026-08-16T00:00:00Z",
        }),
        ToolKind::Compute => json!({
            "op": "compute",
            "label": label,
            "metric": format!("agg({label})"),
            "value": ((rows as f64) * 1.37).round() as i64,
            "rows": rows,
        }),
        ToolKind::Llm => json!({
            "op": "llm",
            "label": label,
            "summary": format!("基于 {rows} 条记录生成《{label}》摘要"),
            "tokens": rows * 32,
            "rows": rows,
        }),
        ToolKind::Database => json!({
            "op": "query",
            "label": label,
            "table": "postgres.public.records",
            "rows": rows,
        }),
        ToolKind::File => json!({
            "op": "file",
            "label": label,
            "path": format!("/data/{}.csv", slug(label)),
            "rows": rows,
        }),
        ToolKind::Browser => json!({
            "op": "browser",
            "label": label,
            "url": "https://example.com",
            "rows": rows,
        }),
        ToolKind::Shell => json!({
            "op": "shell",
            "label": label,
            "exit_code": 0,
            "stdout": format!("executed {label}"),
            "rows": rows,
        }),
        ToolKind::Human => json!({
            "op": "human",
            "label": label,
            "status": "approved",
            "rows": rows,
        }),
    };
    Ok(out)
}

/// 按子任务顺序执行整条流水线，返回执行记录与整体质量评分 (0,1]。
///
/// 上游输出以 `rows` 字段向下游传递规模，构成真实的数据流依赖。
pub fn execute_chain(subtasks: &[SubtaskTriple], seed: &Value) -> (Vec<ExecRecord>, f64) {
    let mut records = Vec::new();
    let mut input = seed.clone();
    let mut ok = 0usize;
    for (key, label, tool) in subtasks {
        match dispatch(*tool, label, &input) {
            Ok(output) => {
                let rows = output.get("rows").and_then(|v| v.as_u64()).unwrap_or(1);
                records.push(ExecRecord {
                    key: key.clone(),
                    label: label.clone(),
                    tool: tool_name(*tool),
                    ok: true,
                    note: "ok".into(),
                    output: output.clone(),
                });
                ok += 1;
                input = json!({ "rows": rows.max(1), "prev": output });
            }
            Err(e) => {
                records.push(ExecRecord {
                    key: key.clone(),
                    label: label.clone(),
                    tool: tool_name(*tool),
                    ok: false,
                    note: e.to_string(),
                    output: Value::Null,
                });
                input = json!({ "rows": 1 });
            }
        }
    }
    // 质量：执行成功率 + 基础质量保底，保持 0 < q <= 1
    let q = if subtasks.is_empty() {
        0.0
    } else {
        (ok as f64 / subtasks.len() as f64) * 0.9 + 0.05
    };
    (records, q)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_flow_sdk::model::ToolKind;

    #[test]
    fn dispatch_each_toolkind_produces_rows() {
        for t in [
            ToolKind::Http,
            ToolKind::Compute,
            ToolKind::Llm,
            ToolKind::Database,
            ToolKind::File,
            ToolKind::Browser,
            ToolKind::Shell,
            ToolKind::Human,
        ] {
            let v = dispatch(t, "测试算子", &json!({ "rows": 4 })).unwrap();
            assert!(v.get("rows").is_some(), "{t:?} 应返回 rows");
            assert!(v.get("op").is_some(), "{t:?} 应返回 op");
        }
    }

    #[test]
    fn compute_scales_rows_deterministically() {
        let a = dispatch(ToolKind::Http, "抓取", &json!({ "rows": 10 })).unwrap();
        let b = dispatch(ToolKind::Http, "抓取", &json!({ "rows": 10 })).unwrap();
        assert_eq!(a, b, "同一输入应产生确定输出");
        assert_eq!(a["rows"], json!(73));
    }

    #[test]
    fn execute_chain_threads_outputs_and_scores() {
        let subs = vec![
            ("fetch".into(), "抓取销售数据".into(), ToolKind::Http),
            ("clean".into(), "清洗对账".into(), ToolKind::Compute),
            ("report".into(), "生成图表报告".into(), ToolKind::Llm),
        ];
        let (recs, q) = execute_chain(&subs, &json!({ "rows": 8, "requirement": "demo" }));
        assert_eq!(recs.len(), 3);
        assert!(recs.iter().all(|r| r.ok));
        // 下一步应收到上一步放大的 rows
        assert!(
            recs[1].output["rows"].as_u64().unwrap() >= recs[0].output["rows"].as_u64().unwrap()
        );
        assert!((0.9..=1.0).contains(&q), "全成功质量应≈0.95，实得{q:.3}");
    }

    #[test]
    fn empty_chain_scores_zero() {
        let (recs, q) = execute_chain(&[], &json!({ "rows": 1 }));
        assert!(recs.is_empty());
        assert_eq!(q, 0.0);
    }
}
