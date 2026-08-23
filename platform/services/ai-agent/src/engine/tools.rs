//! 工具扩展模块 - 可扩展的工具注册表
//!
//! 提供统一的工具调用接口，支持：
//! - Tool trait: 定义所有工具的统一行为契约
//! - ToolRegistry: 管理所有可用工具，按名称查找与调度
//! - 内置工具：DatabaseTool / CodeSandboxTool / HttpRequestTool / FileOperationTool / CalculatorTool

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// 工具统一返回结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

/// 工具统一接口
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, params: &serde_json::Value) -> ToolResult;
}

// ── ToolRegistry ─────────────────────────────────────────

/// 工具注册表：管理所有可用工具
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn list_tools(&self) -> Vec<(&str, &str)> {
        self.tools
            .values()
            .map(|t| (t.name(), t.description()))
            .collect()
    }

    pub fn execute(&self, name: &str, params: &serde_json::Value) -> ToolResult {
        match self.get(name) {
            Some(tool) => tool.execute(params),
            None => ToolResult::err(format!("工具 '{}' 不存在", name)),
        }
    }

    /// 创建并返回包含所有内置工具的注册表
    pub fn with_builtin_tools() -> Self {
        let mut registry = Self::new();
        registry.register(DatabaseTool::new());
        registry.register(CodeSandboxTool::new());
        registry.register(HttpRequestTool::new());
        registry.register(FileOperationTool::new());
        registry.register(CalculatorTool::new());
        registry
    }
}

// ── DatabaseTool ──────────────────────────────────────────

/// SQLite 操作工具：query / insert / update / delete
#[allow(dead_code)]
pub struct DatabaseTool {
    db_path: String,
    provider: Arc<dyn xuanji_system::persistence_provider::PersistenceProvider>,
}

impl DatabaseTool {
    pub fn new() -> Self {
        let db_path = "operator_data.db".to_string();
        use xuanji_system::sqlite_provider::SqlitePersistence;
        let pvd = SqlitePersistence::file(&db_path).unwrap_or_else(|_| {
            SqlitePersistence::memory().expect("in-memory fallback must work")
        });
        Self { db_path, provider: Arc::new(pvd) }
    }

    pub fn with_path(path: impl Into<String>) -> Self {
        let db_path = path.into();
        use xuanji_system::sqlite_provider::SqlitePersistence;
        let pvd = SqlitePersistence::file(&db_path).unwrap_or_else(|_| {
            SqlitePersistence::memory().expect("in-memory fallback must work")
        });
        Self { db_path, provider: Arc::new(pvd) }
    }

    fn params_to_refs(
        params: Option<&serde_json::Value>,
    ) -> Vec<xuanji_system::persistence_provider::SqlValue> {
        use xuanji_system::sqlite_provider::json_to_sql_value;
        if let Some(p) = params {
            if let Some(arr) = p.as_array() {
                arr.iter().map(json_to_sql_value).collect()
            } else if let Some(map) = p.as_object() {
                map.values().map(json_to_sql_value).collect()
            } else {
                vec![json_to_sql_value(p)]
            }
        } else {
            vec![]
        }
    }

    fn execute_query(&self, sql: &str, params: Option<&serde_json::Value>) -> ToolResult {
        use xuanji_system::persistence_provider::SqlValue;
        let vals = Self::params_to_refs(params);

        let rows = match self.provider.query(sql, &vals) {
            Ok(r) => r,
            Err(e) => return ToolResult::err(format!("查询执行失败: {}", e)),
        };

        let mut results = Vec::new();
        for row in rows {
            let mut map = serde_json::Map::new();
            for (col_name, val) in row {
                let json_val = match val {
                    SqlValue::Null => serde_json::Value::Null,
                    SqlValue::Int(i) => serde_json::json!(i),
                    SqlValue::Real(f) => serde_json::json!(f),
                    SqlValue::Text(s) => serde_json::Value::String(s),
                    SqlValue::Blob(b) => serde_json::Value::String(base64_encode(&b)),
                    SqlValue::Bool(b) => serde_json::json!(b),
                };
                map.insert(col_name, json_val);
            }
            results.push(serde_json::Value::Object(map));
        }

        ToolResult::ok(serde_json::Value::Array(results))
    }

    fn execute_write(&self, sql: &str, params: Option<&serde_json::Value>) -> ToolResult {
        let vals = Self::params_to_refs(params);

        match self.provider.exec(sql, &vals) {
            Ok(count) => ToolResult::ok(serde_json::json!({"affected": count})),
            Err(e) => ToolResult::err(format!("写入操作失败: {}", e)),
        }
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.encode(data)
}

impl Default for DatabaseTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for DatabaseTool {
    fn name(&self) -> &str {
        "database"
    }

    fn description(&self) -> &str {
        "SQLite 数据库操作（query / insert / update / delete）"
    }

    fn execute(&self, params: &serde_json::Value) -> ToolResult {
        let action = params
            .get("action")
            .and_then(|a| a.as_str())
            .unwrap_or("");
        let sql = params
            .get("sql")
            .and_then(|s| s.as_str())
            .unwrap_or("");
        let data = params.get("params");

        if sql.is_empty() {
            return ToolResult::err("缺少 sql 参数");
        }

        match action {
            "query" => self.execute_query(sql, data),
            "insert" | "update" | "delete" => self.execute_write(sql, data),
            _ => ToolResult::err(format!("不支持的数据库操作: {}", action)),
        }
    }
}

// ── CodeSandboxTool ───────────────────────────────────────

/// 代码沙箱工具：执行简单代码片段（模拟模式，记录结果）
#[allow(dead_code)]
pub struct CodeSandboxTool {
    max_execution_time_ms: u64,
}

impl CodeSandboxTool {
    pub fn new() -> Self {
        Self {
            max_execution_time_ms: 5000,
        }
    }

    pub fn with_timeout(timeout_ms: u64) -> Self {
        Self {
            max_execution_time_ms: timeout_ms,
        }
    }

    fn execute_python(&self, code: &str) -> ToolResult {
        let simulated = format!(
            "[Python 沙箱执行] 代码片段:\n{}\n\n[模拟执行结果]\n执行成功 (模拟模式)",
            code
        );
        tracing::debug!(target: "tool_sandbox", code_len = code.len(), "Python 代码沙箱执行");
        ToolResult::ok(serde_json::json!({
            "language": "python",
            "code": code,
            "simulated": true,
            "output": simulated,
            "status": "success"
        }))
    }

    fn execute_rust(&self, code: &str) -> ToolResult {
        let simulated = format!(
            "[Rust 沙箱执行] 代码片段:\n{}\n\n[模拟执行结果]\n执行成功 (模拟模式)",
            code
        );
        tracing::debug!(target: "tool_sandbox", code_len = code.len(), "Rust 代码沙箱执行");
        ToolResult::ok(serde_json::json!({
            "language": "rust",
            "code": code,
            "simulated": true,
            "output": simulated,
            "status": "success"
        }))
    }
}

impl Default for CodeSandboxTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for CodeSandboxTool {
    fn name(&self) -> &str {
        "sandbox"
    }

    fn description(&self) -> &str {
        "执行 Python/Rust 代码片段（沙箱模式，安全隔离）"
    }

    fn execute(&self, params: &serde_json::Value) -> ToolResult {
        let language = params
            .get("language")
            .and_then(|l| l.as_str())
            .unwrap_or("python");
        let code = params
            .get("code")
            .and_then(|c| c.as_str())
            .unwrap_or("");

        if code.is_empty() {
            return ToolResult::err("缺少 code 参数");
        }

        if code.len() > 10_000 {
            return ToolResult::err("代码片段过长（超过 10000 字符）");
        }

        match language {
            "python" => self.execute_python(code),
            "rust" => self.execute_rust(code),
            _ => ToolResult::err(format!("不支持的语言: {}", language)),
        }
    }
}

// ── HttpRequestTool ──────────────────────────────────────

/// HTTP 请求工具：GET / POST / PUT / DELETE
pub struct HttpRequestTool;

impl HttpRequestTool {
    pub fn new() -> Self {
        Self
    }

    fn build_request(params: &serde_json::Value) -> Result<reqwest::RequestBuilder, String> {
        let url = params
            .get("url")
            .and_then(|u| u.as_str())
            .ok_or_else(|| "缺少 url 参数".to_string())?;
        let method = params
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let client = reqwest::Client::new();

        let mut builder = match method.as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            _ => return Err(format!("不支持的 HTTP 方法: {}", method)),
        };

        if let Some(headers) = params.get("headers").and_then(|h| h.as_object()) {
            for (key, value) in headers {
                if let Some(v) = value.as_str() {
                    builder = builder.header(key, v);
                }
            }
        }

        if let Some(body) = params.get("body").and_then(|b| b.as_str()) {
            builder = builder.body(body.to_string());
        } else if let Some(body_json) = params.get("body").and_then(|b| b.as_object()) {
            builder = builder.json(body_json);
        }

        Ok(builder)
    }

    fn execute_request(&self, params: &serde_json::Value) -> ToolResult {
        let builder = match Self::build_request(params) {
            Ok(b) => b,
            Err(e) => return ToolResult::err(e),
        };

        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => return ToolResult::err(format!("运行时创建失败: {}", e)),
        };

        runtime.block_on(async {
            match builder.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    ToolResult::ok(serde_json::json!({
                        "status": status.as_u16(),
                        "body": text.chars().take(5000).collect::<String>(),
                        "success": status.is_success()
                    }))
                }
                Err(e) => ToolResult::err(format!("HTTP 请求失败: {}", e)),
            }
        })
    }
}

impl Default for HttpRequestTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for HttpRequestTool {
    fn name(&self) -> &str {
        "http"
    }

    fn description(&self) -> &str {
        "HTTP 请求工具（GET / POST / PUT / DELETE）"
    }

    fn execute(&self, params: &serde_json::Value) -> ToolResult {
        self.execute_request(params)
    }
}

// ── FileOperationTool ────────────────────────────────────

/// 文件读写工具：读取 / 写入 / 追加
pub struct FileOperationTool {
    base_dir: String,
}

impl FileOperationTool {
    pub fn new() -> Self {
        Self {
            base_dir: std::env::temp_dir()
                .to_string_lossy()
                .to_string(),
        }
    }

    pub fn with_base_dir(dir: impl Into<String>) -> Self {
        Self {
            base_dir: dir.into(),
        }
    }

    fn resolve_path(&self, path: &str) -> String {
        if std::path::Path::new(path).is_absolute() {
            path.to_string()
        } else {
            std::path::Path::new(&self.base_dir)
                .join(path)
                .to_string_lossy()
                .to_string()
        }
    }

    fn read_file(&self, path: &str) -> ToolResult {
        let full_path = self.resolve_path(path);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => ToolResult::ok(serde_json::json!({
                "path": full_path,
                "content": content.chars().take(10000).collect::<String>(),
                "size": content.len()
            })),
            Err(e) => ToolResult::err(format!("文件读取失败: {}", e)),
        }
    }

    fn write_file(&self, path: &str, content: &str, append: bool) -> ToolResult {
        let full_path = self.resolve_path(path);
        if let Some(parent) = std::path::Path::new(&full_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let result = if append {
            use std::io::Write;
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&full_path)
            {
                Ok(mut file) => match file.write_all(content.as_bytes()) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            }
        } else {
            std::fs::write(&full_path, content.as_bytes())
        };

        match result {
            Ok(_) => ToolResult::ok(serde_json::json!({
                "path": full_path,
                "bytes_written": content.len(),
                "append": append
            })),
            Err(e) => ToolResult::err(format!("文件写入失败: {}", e)),
        }
    }
}

impl Default for FileOperationTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for FileOperationTool {
    fn name(&self) -> &str {
        "file"
    }

    fn description(&self) -> &str {
        "文件读写操作（读取 / 写入 / 追加）"
    }

    fn execute(&self, params: &serde_json::Value) -> ToolResult {
        let action = params
            .get("action")
            .and_then(|a| a.as_str())
            .unwrap_or("");
        let path = params
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolResult::err("缺少 path 参数");
        }

        match action {
            "read" => self.read_file(path),
            "write" => {
                let content = params
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                self.write_file(path, content, false)
            }
            "append" => {
                let content = params
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                self.write_file(path, content, true)
            }
            _ => ToolResult::err(format!("不支持的文件操作: {}", action)),
        }
    }
}

// ── CalculatorTool ───────────────────────────────────────

/// 数学计算工具：基础运算与表达式求值
pub struct CalculatorTool;

impl CalculatorTool {
    pub fn new() -> Self {
        Self
    }

    pub fn eval_expression(expr: &str) -> Result<f64, String> {
        let chars: Vec<char> = expr.chars().filter(|c| !c.is_whitespace()).collect();
        if chars.is_empty() {
            return Err("表达式为空".to_string());
        }
        let mut pos: usize = 0;
        let result = parse_expr(&chars, &mut pos)?;
        if pos < chars.len() {
            return Err(format!(
                "表达式解析完成后仍有剩余字符: '{}'",
                chars[pos]
            ));
        }
        Ok(result)
    }
}

fn parse_number(chars: &[char], pos: &mut usize) -> Result<f64, String> {
    let start = *pos;
    let mut has_dot = false;
    while *pos < chars.len() {
        let c = chars[*pos];
        if c.is_ascii_digit() {
            *pos += 1;
        } else if c == '.' && !has_dot {
            has_dot = true;
            *pos += 1;
        } else {
            break;
        }
    }
    if *pos == start {
        return Err(format!("无法解析数字 at position {}", *pos));
    }
    chars[start..*pos]
        .iter()
        .collect::<String>()
        .parse::<f64>()
        .map_err(|e| format!("数字解析失败: {}", e))
}

fn parse_expr(chars: &[char], pos: &mut usize) -> Result<f64, String> {
    parse_term(chars, pos)
}

fn parse_term(chars: &[char], pos: &mut usize) -> Result<f64, String> {
    let mut left = parse_factor(chars, pos)?;
    while *pos < chars.len() {
        match chars[*pos] {
            '+' => {
                *pos += 1;
                let right = parse_factor(chars, pos)?;
                left += right;
            }
            '-' => {
                *pos += 1;
                let right = parse_factor(chars, pos)?;
                left -= right;
            }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_factor(chars: &[char], pos: &mut usize) -> Result<f64, String> {
    let mut left = parse_unary(chars, pos)?;
    while *pos < chars.len() {
        match chars[*pos] {
            '*' => {
                *pos += 1;
                let right = parse_unary(chars, pos)?;
                left *= right;
            }
            '/' => {
                *pos += 1;
                let right = parse_unary(chars, pos)?;
                if right == 0.0 {
                    return Err("除数不能为零".to_string());
                }
                left /= right;
            }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_unary(chars: &[char], pos: &mut usize) -> Result<f64, String> {
    if *pos < chars.len() && chars[*pos] == '-' {
        *pos += 1;
        let val = parse_unary(chars, pos)?;
        Ok(-val)
    } else if *pos < chars.len() && chars[*pos] == '+' {
        *pos += 1;
        parse_unary(chars, pos)
    } else {
        parse_primary(chars, pos)
    }
}

fn parse_primary(chars: &[char], pos: &mut usize) -> Result<f64, String> {
    if *pos < chars.len() && chars[*pos] == '(' {
        *pos += 1;
        let val = parse_expr(chars, pos)?;
        if *pos >= chars.len() || chars[*pos] != ')' {
            return Err("缺少右括号".to_string());
        }
        *pos += 1;
        Ok(val)
    } else {
        parse_number(chars, pos)
    }
}

impl Default for CalculatorTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "数学计算工具：支持加减乘除与括号表达式"
    }

    fn execute(&self, params: &serde_json::Value) -> ToolResult {
        let expression = params
            .get("expression")
            .and_then(|e| e.as_str())
            .unwrap_or("");

        if expression.is_empty() {
            return ToolResult::err("缺少 expression 参数");
        }

        match Self::eval_expression(expression) {
            Ok(result) => ToolResult::ok(serde_json::json!({
                "expression": expression,
                "result": result
            })),
            Err(e) => ToolResult::err(e),
        }
    }
}

// ── 工具注册辅助 ─────────────────────────────────────────

/// 为 phase_act 提供工具类型 -> ToolRegistry 方法名映射
pub fn tool_type_to_name(tool_type: &str) -> Option<&str> {
    match tool_type {
        "database" => Some("database"),
        "sandbox" => Some("sandbox"),
        "http" => Some("http"),
        "file" => Some("file"),
        "calculator" => Some("calculator"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculator_basic() {
        let tool = CalculatorTool::new();
        let r = tool.execute(&serde_json::json!({"expression": "2 + 3"}));
        assert!(r.success);
        let result = r.data.unwrap();
        assert_eq!(result["result"], serde_json::json!(5.0));
    }

    #[test]
    fn test_calculator_complex() {
        let tool = CalculatorTool::new();
        let r = tool.execute(&serde_json::json!({"expression": "(1 + 2) * (3 + 4)"}));
        assert!(r.success);
        let result = r.data.unwrap();
        assert_eq!(result["result"], serde_json::json!(21.0));
    }

    #[test]
    fn test_calculator_division_by_zero() {
        let tool = CalculatorTool::new();
        let r = tool.execute(&serde_json::json!({"expression": "1 / 0"}));
        assert!(!r.success);
    }

    #[test]
    fn test_calculator_empty_expression() {
        let tool = CalculatorTool::new();
        let r = tool.execute(&serde_json::json!({}));
        assert!(!r.success);
    }

    #[test]
    fn test_registry_builtin_tools() {
        let registry = ToolRegistry::with_builtin_tools();
        let tools = registry.list_tools();
        assert_eq!(tools.len(), 5);
        assert!(tools.iter().any(|(n, _)| *n == "database"));
        assert!(tools.iter().any(|(n, _)| *n == "sandbox"));
        assert!(tools.iter().any(|(n, _)| *n == "http"));
        assert!(tools.iter().any(|(n, _)| *n == "file"));
        assert!(tools.iter().any(|(n, _)| *n == "calculator"));
    }

    #[test]
    fn test_registry_execute_calculator() {
        let registry = ToolRegistry::with_builtin_tools();
        let r = registry.execute("calculator", &serde_json::json!({"expression": "10 / 2 + 3"}));
        assert!(r.success);
    }

    #[test]
    fn test_registry_unknown_tool() {
        let registry = ToolRegistry::with_builtin_tools();
        let r = registry.execute("nonexistent", &serde_json::json!({}));
        assert!(!r.success);
    }

    #[test]
    fn test_tool_type_mapping() {
        assert_eq!(tool_type_to_name("database"), Some("database"));
        assert_eq!(tool_type_to_name("sandbox"), Some("sandbox"));
        assert_eq!(tool_type_to_name("http"), Some("http"));
        assert_eq!(tool_type_to_name("file"), Some("file"));
        assert_eq!(tool_type_to_name("calculator"), Some("calculator"));
        assert_eq!(tool_type_to_name("unknown"), None);
    }

    #[test]
    fn test_file_operation_read_write() {
        let tool = FileOperationTool::new();
        let write_r = tool.execute(&serde_json::json!({
            "action": "write",
            "path": "test_tool_temp.txt",
            "content": "hello world"
        }));
        assert!(write_r.success);

        let read_r = tool.execute(&serde_json::json!({
            "action": "read",
            "path": "test_tool_temp.txt"
        }));
        assert!(read_r.success);
        let data = read_r.data.unwrap();
        assert!(data["content"].as_str().unwrap().contains("hello world"));

        let _ = std::fs::remove_file(tool.resolve_path("test_tool_temp.txt"));
    }

    #[test]
    fn test_database_tool_query() {
        let tool = DatabaseTool::with_path(":memory:");
        let create_r = tool.execute(&serde_json::json!({
            "action": "query",
            "sql": "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)"
        }));
        assert!(create_r.success);

        let insert_r = tool.execute(&serde_json::json!({
            "action": "insert",
            "sql": "INSERT INTO test (name) VALUES ('hello')"
        }));
        assert!(insert_r.success);

        let select_r = tool.execute(&serde_json::json!({
            "action": "query",
            "sql": "SELECT * FROM test"
        }));
        assert!(select_r.success);
    }
}