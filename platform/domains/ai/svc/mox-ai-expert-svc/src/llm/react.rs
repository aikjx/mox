// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! ReAct（Reasoning + Acting）推理循环
//!
//! 流程：系统提示 + 用户问题 → 模型输出；
//! 若输出含 `<tool_call>{json}</tool_call>`，执行对应工具并把观察结果回喂，
//! 继续下一轮；否则视为最终答案。受 `max_rounds` / `max_tool_calls` 约束。

use super::chat::{ChatClient, ChatMessage};
use super::tools::ToolRegistry;
use serde_json::Value;

/// ReAct 运行配置
#[derive(Debug, Clone)]
pub struct ReactConfig {
    /// 最大推理轮数（含工具轮）
    pub max_rounds: usize,
    /// 最大工具调用次数
    pub max_tool_calls: usize,
}

impl Default for ReactConfig {
    fn default() -> Self {
        Self {
            max_rounds: 3,
            max_tool_calls: 8,
        }
    }
}

impl ReactConfig {
    pub fn from_env() -> Self {
        let max_rounds = std::env::var("MOX_LLM_MAX_ROUNDS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0 && *n <= 20)
            .unwrap_or(3);
        Self {
            max_rounds,
            max_tool_calls: 8,
        }
    }
}

/// ReAct 运行结果
#[derive(Debug, Clone)]
pub struct ReactResult {
    /// 最终答案（模型文本）
    pub final_answer: String,
    /// 实际推理轮数
    pub rounds: usize,
    /// 实际工具调用次数
    pub tool_calls: usize,
    /// 每轮模型原始输出（推理轨迹，写入 ConsultReport.steps）
    pub trace: Vec<String>,
    /// 工具调用记录：(工具名, 观察结果)
    pub observations: Vec<(String, String)>,
    /// 是否因达到最大轮数而截断
    pub truncated: bool,
}

impl ReactResult {
    /// 推理轨迹（含每轮摘要）——供 ConsultReport.steps 使用
    pub fn to_steps(&self, model: &str) -> Vec<String> {
        let mut steps = Vec::with_capacity(self.trace.len() + 2);
        steps.push(format!("[ReAct] 真实 LLM({}) 专家推理", model));
        for (i, t) in self.trace.iter().enumerate() {
            let line = t.lines().next().unwrap_or("").trim();
            let snippet = if line.len() > 80 {
                format!("{}…", &line[..80])
            } else {
                line.to_string()
            };
            steps.push(format!("[{}/{}] {}", i + 1, self.trace.len(), snippet));
        }
        if self.tool_calls > 0 {
            steps.push(format!("[工具] 共调用 {} 次工具", self.tool_calls));
        }
        if self.truncated {
            steps.push("[警告] 达到最大推理轮数，结果可能未收敛".into());
        }
        steps
    }
}

/// 工具调用声明
struct ToolCall {
    name: String,
    args: Value,
}

/// 从模型输出中解析第一个 `<tool_call>{json}</tool_call>` 块
fn parse_tool_call(text: &str) -> Option<ToolCall> {
    let start = text.find("<tool_call>")? + "<tool_call>".len();
    let end = text[start..].find("</tool_call>")? + start;
    let json = &text[start..end];
    let v: Value = serde_json::from_str(json).ok()?;
    let name = v.get("name")?.as_str()?.to_string();
    let args = v.get("arguments").cloned().unwrap_or(Value::Null);
    Some(ToolCall { name, args })
}

/// 运行 ReAct 循环（同步；调用方负责放入 spawn_blocking）
pub fn run_react(
    client: &dyn ChatClient,
    system: &str,
    user: &str,
    tools: &ToolRegistry,
    config: &ReactConfig,
) -> anyhow::Result<ReactResult> {
    let mut messages: Vec<ChatMessage> = vec![
        ChatMessage::system(system.to_string()),
        ChatMessage::user(user.to_string()),
    ];
    let mut trace: Vec<String> = Vec::new();
    let mut observations: Vec<(String, String)> = Vec::new();
    let mut tool_calls = 0usize;
    let mut final_answer: Option<String> = None;
    let mut truncated = false;

    for _round in 0..config.max_rounds {
        let output = client.complete(&messages)?;
        trace.push(output.clone());

        match parse_tool_call(&output) {
            Some(tc) => {
                if tool_calls >= config.max_tool_calls {
                    // 工具调用次数已满：不再执行，把当前输出作为答案
                    final_answer = Some(output);
                    truncated = true;
                    break;
                }
                let result = match tools.find(&tc.name) {
                    Some(tool) => match tool.run(&tc.args) {
                        Ok(r) => r,
                        Err(e) => super::tools::ToolResult::err(format!("工具执行异常: {}", e)),
                    },
                    None => super::tools::ToolResult::err(format!("未知工具: {}", tc.name)),
                };
                tool_calls += 1;
                observations.push((tc.name.clone(), result.output.clone()));
                // 助手声明工具调用 + 用户侧观察结果
                messages.push(ChatMessage::assistant(output.clone()));
                messages.push(ChatMessage::tool(format!(
                    "工具 {} 返回: {}",
                    tc.name, result.output
                )));
            }
            None => {
                final_answer = Some(output);
                break;
            }
        }
    }

    let final_answer = match final_answer {
        Some(a) => a,
        None => {
            truncated = true;
            trace
                .last()
                .cloned()
                .unwrap_or_else(|| "（模型未返回内容）".to_string())
        }
    };

    Ok(ReactResult {
        final_answer,
        rounds: trace.len(),
        tool_calls,
        trace,
        observations,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::super::chat::MockChatClient;
    use super::*;

    #[test]
    fn parse_tool_call_extracts_json() {
        let s = "我需要计算。\n<tool_call>{\"name\":\"calculate\",\"arguments\":{\"expression\":\"2+3\"}}</tool_call>\n结果如下";
        let tc = parse_tool_call(s).unwrap();
        assert_eq!(tc.name, "calculate");
        assert_eq!(tc.args["expression"], "2+3");
        assert!(parse_tool_call("没有工具调用").is_none());
    }

    #[test]
    fn react_runs_tool_then_final() {
        // 第 1 轮：工具调用；第 2 轮：最终答案
        let client = MockChatClient::new(vec![
            "<tool_call>{\"name\":\"calculate\",\"arguments\":{\"expression\":\"6*7\"}}</tool_call>".into(),
            "最终答案：42。置信度评分 0.9".into(),
        ]);
        let tools = ToolRegistry::with_builtins();
        let res = run_react(
            &client,
            "你是数学专家",
            "6*7=?",
            &tools,
            &ReactConfig::default(),
        )
        .unwrap();
        assert_eq!(res.tool_calls, 1);
        assert_eq!(res.observations.len(), 1);
        assert_eq!(res.observations[0].0, "calculate");
        assert!(res.final_answer.contains("42"));
        assert_eq!(res.rounds, 2);
        assert!(!res.truncated);
        assert_eq!(client.call_count(), 2);
        assert!(res.to_steps("mock").len() >= 3);
    }

    #[test]
    fn react_truncates_on_max_rounds() {
        // 始终返回工具调用 → 应被 max_rounds 截断
        let client = MockChatClient::new(vec![
            "<tool_call>{\"name\":\"now\",\"arguments\":{}}</tool_call>".into(),
        ]);
        let tools = ToolRegistry::with_builtins();
        let res = run_react(
            &client,
            "sys",
            "q",
            &tools,
            &ReactConfig {
                max_rounds: 2,
                max_tool_calls: 8,
            },
        )
        .unwrap();
        assert!(res.truncated);
        assert_eq!(res.rounds, 2);
        assert_eq!(res.tool_calls, 2);
    }

    #[test]
    fn react_unknown_tool_reports_error_observation() {
        let client = MockChatClient::new(vec![
            "<tool_call>{\"name\":\"nope\",\"arguments\":{}}</tool_call>".into(),
            "done".into(),
        ]);
        let tools = ToolRegistry::with_builtins();
        let res = run_react(&client, "sys", "q", &tools, &ReactConfig::default()).unwrap();
        assert_eq!(res.tool_calls, 1);
        assert!(res.observations[0].1.contains("未知工具"));
        assert_eq!(res.final_answer, "done");
    }
}
