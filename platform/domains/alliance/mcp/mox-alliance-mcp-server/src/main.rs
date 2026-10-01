//! 专家联盟 MCP Server（T3）。
//!
//! 设计要点（与选型决策一致）：
//! - **自实现** JSON-RPC 2.0 over stdio，传输帧为 `Content-Length: N\r\n\r\n<json>`，
//!   零第三方 MCP crate，契合「零外部依赖气隙部署」卖点。
//! - **独立 binary**：不侵入 axum 网关进程（stdin/stdout 与 async runtime 隔离）。
//! - **薄适配层**：工具 handler 不重复匹配/组队/检索逻辑，全部以真实 HTTP 调用网关读面端点，
//!   返回网关真实响应。禁止本地 mock。
//!
//! 配置（环境变量，零硬编码）：
//! - `MOX_MCP_GATEWAY_URL`：网关基址，默认 `http://127.0.0.1:3080`
//! - `MOX_INTERNAL_TOKEN` ：可选，设置后注入 `Authorization: Bearer <token>`（读面默认无需）

use std::io::{self, Write};

use serde_json::{json, Value};
use tokio::io::{AsyncWriteExt, BufReader};

const SERVER_NAME: &str = "mox-alliance-mcp-server";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_VERSION: &str = "2024-11-05";

/// 网关读面客户端：仅持有基址与可选内部 token，请求全部透传真实响应。
struct GatewayClient {
    base: String,
    token: Option<String>,
    http: reqwest::Client,
}

impl GatewayClient {
    fn from_env() -> Self {
        let base = std::env::var("MOX_MCP_GATEWAY_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3080".to_string())
            .trim_end_matches('/')
            .to_string();
        let token = std::env::var("MOX_INTERNAL_TOKEN").ok().filter(|t| !t.is_empty());
        Self {
            base,
            token,
            http: reqwest::Client::new(),
        }
    }

    /// POST 到网关读面端点，返回 (http_status, 响应体 Value)。
    async fn post(&self, path: &str, body: &Value) -> Result<(u16, Value), String> {
        let url = format!("{}{}", self.base, path);
        let mut req = self.http.post(&url).json(body);
        if let Some(tok) = &self.token {
            req = req.header("Authorization", format!("Bearer {tok}"));
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("网关请求失败 {url}: {e}"))?;
        let status = resp.status().as_u16();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("读取网关响应失败 {url}: {e}"))?;
        let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
        Ok((status, parsed))
    }
}

/// 工具清单：与网关真实端点一一对应，inputSchema 字段照抄后端请求 DTO。
fn tools_list() -> Vec<Value> {
    vec![
        json!({
            "name": "expert_search",
            "description": "专家匹配搜索：按关键词/领域在专家库做 RuleBasedExpertMatcher 真实匹配，返回逐维 scores（domain/capability/health/priority/performance）与 match_score。对应网关 POST /api/alliance/experts/search。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query":     { "type": "string", "description": "搜索关键词 / 任务描述" },
                    "domains":   { "type": "array", "items": { "type": "string" }, "description": "限定领域 id 列表，可空" },
                    "limit":     { "type": "integer", "description": "返回条数上限，默认 10" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "optimal_team",
            "description": "最优团队推荐：真实加权集合覆盖贪心算法，按所需技能/领域组队，返回 team_members + coverage + team_score。对应网关 POST /api/expert-graph/optimal-team。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "required_skills":  { "type": "array", "items": { "type": "string" }, "description": "所需技能 id 列表" },
                    "required_domains": { "type": "array", "items": { "type": "string" }, "description": "所需领域 id 列表" },
                    "max_members":      { "type": "integer", "description": "团队人数上限，默认 5" },
                    "min_rating":       { "type": "number", "description": "最低评分门槛，默认 4.0" },
                    "goal":             { "type": "string", "description": "自然语言目标（未显式给技能/领域时由规则提取）" }
                }
            }
        }),
        json!({
            "name": "graph_expand",
            "description": "图 RAG 多跳邻域扩展（T2）：从种子节点沿边权重乘积聚合做邻域检索，返回邻域节点 + aggregate_weight + path。对应网关 POST /api/expert-graph/rag/expand。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "seeds":      { "type": "array", "items": { "type": "string" }, "description": "种子节点 id（专家或域），非空" },
                    "max_depth":  { "type": "integer", "description": "最大跳数 1..=4，默认 2" },
                    "top_k":      { "type": "integer", "description": "返回条数上限，默认 20" },
                    "node_types": { "type": "array", "items": { "type": "string" }, "description": "结果节点类型过滤，如 [\"expert\"]" },
                    "min_weight": { "type": "number", "description": "边权重下限 0.0..=1.0，默认 0.0" }
                },
                "required": ["seeds"]
            }
        })
    ]
}

/// tools/call 分发：薄适配，arguments 原样透传为网关请求体。
async fn call_tool(gw: &GatewayClient, name: &str, args: Value) -> Value {
    let (path, upstream) = match name {
        "expert_search" => ("/api/alliance/experts/search", "POST /api/alliance/experts/search"),
        "optimal_team" => ("/api/expert-graph/optimal-team", "POST /api/expert-graph/optimal-team"),
        "graph_expand" => ("/api/expert-graph/rag/expand", "POST /api/expert-graph/rag/expand"),
        other => {
            return json!({
                "isError": true,
                "content": [{"type": "text", "text": format!("未知工具: {other}（可用: expert_search/optimal_team/graph_expand）")}]
            });
        }
    };

    match gw.post(path, &args).await {
        Ok((status, body)) => {
            // 真实字段放在 result 下透传；元信息并列，不替换真实 DTO。
            let payload = json!({
                "mcp_tool": name,
                "called_at": chrono::Utc::now().to_rfc3339(),
                "upstream": upstream,
                "gateway_status": status,
                "result": body
            });
            let is_error = status >= 400;
            json!({
                "isError": is_error,
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap_or_default()}]
            })
        }
        Err(e) => json!({
            "isError": true,
            "content": [{"type": "text", "text": serde_json::to_string(&json!({
                "mcp_tool": name, "upstream": upstream, "error": e
            })).unwrap_or_default()}]
        }),
    }
}

/// 处理一条 JSON-RPC 请求。通知（无 id）返回 None，不应写响应。
fn handle_request(gw: &GatewayClient, req: &Value) -> Option<Value> {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

    let result = match method {
        "initialize" => json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
        }),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools_list() }),
        "tools/call" => {
            let params = req.get("params").cloned().unwrap_or(json!({}));
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            // 未知工具：纯同步短路，不触网、无需 runtime。
            if !["expert_search", "optimal_team", "graph_expand"].contains(&name) {
                json!({
                    "isError": true,
                    "content": [{"type": "text", "text": format!("未知工具: {name}（可用: expert_search/optimal_team/graph_expand）")}]
                })
            } else {
                // 阻塞调度：在 tokio 运行时内 block_on 单次真实 HTTP 调用。
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(call_tool(gw, name, args))
                })
            }
        }
        // notifications/initialized 与其它通知：静默确认，无响应。
        _ => return None,
    };

    Some(match id {
        Some(id) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        None => return None,
    })
}

/// 从 BufReader 读取一个 Content-Length 帧，返回 JSON Value；EOF 返回 Ok(None)。
///
/// 帧格式：若干头行（以 `\r\n` 结尾），空行结束头块，随后 N 字节帧体。
async fn read_frame<R>(reader: &mut R) -> io::Result<Option<Value>>
where
    R: tokio::io::AsyncBufReadExt + tokio::io::AsyncReadExt + Unpin,
{
    let mut content_length: Option<usize> = None;
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            return Ok(None); // EOF
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            // 头块结束；空行已被 read_line 消费，后续即帧体。
            break;
        }
        if let Some(rest) = trimmed.split_once(':') {
            if rest.0.trim().eq_ignore_ascii_case("Content-Length") {
                let len: usize = rest.1.trim().parse().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "Content-Length 非整数")
                })?;
                content_length = Some(len);
            }
        }
        // 其它头字段忽略，继续读到空行结束头块。
    }

    let len = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "缺少 Content-Length 头")
    })?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    let text = String::from_utf8(buf)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "帧体非 UTF-8"))?;
    let v: Value = serde_json::from_str(&text)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("帧体非 JSON: {e}")))?;
    Ok(Some(v))
}

/// 写一个 JSON-RPC 消息为 Content-Length 帧到 stdout。
async fn write_frame<W: AsyncWriteExt + Unpin>(writer: &mut W, msg: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(msg)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // 诊断一律走 stderr，绝不污染 stdout（stdout 仅 MCP 帧）。
    let gw = GatewayClient::from_env();
    eprintln!(
        "[mox-alliance-mcp-server] 启动 gateway={} token={}",
        gw.base,
        if gw.token.is_some() { "已配置" } else { "未配置" }
    );

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let stdout = tokio::io::stdout();
    let mut writer = stdout;

    loop {
        match read_frame(&mut reader).await {
            Ok(Some(req)) => {
                // 先打印一行 stderr 调用日志（真实请求/响应摘要证据）。
                if let (Some(m), Some(id)) = (
                    req.get("method").and_then(|x| x.as_str()),
                    req.get("id"),
                ) {
                    eprintln!("[mcp] -> method={} id={}", m, id);
                }
                let resp = handle_request(&gw, &req);
                if let Some(msg) = &resp {
                    write_frame(&mut writer, msg).await?;
                    eprintln!("[mcp] <- id={} bytes={}", msg.get("id").unwrap_or(&json!(null)),
                        serde_json::to_string(msg).map(|s| s.len()).unwrap_or(0));
                }
            }
            Ok(None) => break, // stdin 关闭，退出
            Err(e) => {
                // 帧协议错误：回一个 JSON-RPC error，便于客户端定位，不崩。
                let err = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": format!("解析错误: {e}") }
                });
                let _ = write_frame(&mut writer, &err).await;
                return Err(e);
            }
        }
    }
    let _ = io::stdout().flush();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_has_three_real_tools() {
        let tools = tools_list();
        assert_eq!(tools.len(), 3, "应恰好 3 个真实工具");
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["expert_search", "optimal_team", "graph_expand"]);
        for t in &tools {
            assert_eq!(t["inputSchema"]["type"], "object");
            assert!(t["description"].as_str().unwrap().contains("/api/"));
        }
    }

    #[test]
    fn initialize_handshake_shape() {
        let gw = GatewayClient::from_env();
        let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
        let resp = handle_request(&gw, &req).expect("initialize 必须响应");
        assert_eq!(resp["result"]["serverInfo"]["name"], SERVER_NAME);
        assert!(resp["result"]["capabilities"]["tools"].is_object());
        assert!(resp["result"]["protocolVersion"].is_string());
    }

    #[test]
    fn notifications_are_silent() {
        let gw = GatewayClient::from_env();
        // notifications/initialized 无 id → 不应产生响应
        let req = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        assert!(handle_request(&gw, &req).is_none());
    }

    #[test]
    fn unknown_tool_returns_is_error() {
        let gw = GatewayClient::from_env();
        let req = json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nope","arguments":{}}});
        // 该分支不触网（未知工具直接返回）
        let resp = handle_request(&gw, &req).expect("必须响应");
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn frame_header_parses_content_length() {
        // 帧头行解析逻辑与 read_frame 保持一致。
        let line = "Content-Length: 42\r\n";
        let rest = line.trim_end().split_once(':').unwrap().1.trim();
        assert_eq!(rest.parse::<usize>().unwrap(), 42);
        assert!("content-length: 10".split_once(':').unwrap().0.trim().eq_ignore_ascii_case("Content-Length"));
    }
}
