// Copyright (c) 2026 璇玑 RelGraph · AI对话全维自动化核心 (AI Assistant Core)
// Licensed under the MIT License.

//! 工具注册表
//!
//! 智能体可以调用的工具/技能注册中心

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{AiError, AiResult};

/// 工具参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParam {
    /// 参数名
    pub name: String,
    /// 参数类型
    pub param_type: String,
    /// 是否必填
    pub required: bool,
    /// 描述
    pub description: Option<String>,
    /// 默认值
    pub default_value: Option<serde_json::Value>,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 是否成功
    pub success: bool,
    /// 结果数据
    pub data: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
}

impl ToolResult {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            duration_ms: 0,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.to_string()),
            duration_ms: 0,
        }
    }
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    /// 工具 ID
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 参数列表
    pub params: Vec<ToolParam>,
    /// 返回类型描述
    pub return_description: String,
    /// 工具分类
    pub category: String,
    /// 是否启用
    pub enabled: bool,
    /// 调用次数
    pub call_count: u64,
    /// 标签
    pub tags: Vec<String>,
}

impl ToolDef {
    /// 创建工具定义
    pub fn new(name: &str, description: &str, category: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            params: Vec::new(),
            return_description: String::new(),
            category: category.to_string(),
            enabled: true,
            call_count: 0,
            tags: Vec::new(),
        }
    }

    /// 添加参数
    pub fn add_param(&mut self, name: &str, param_type: &str, required: bool) {
        self.params.push(ToolParam {
            name: name.to_string(),
            param_type: param_type.to_string(),
            required,
            description: None,
            default_value: None,
        });
    }

    /// 添加标签
    pub fn add_tag(&mut self, tag: &str) {
        self.tags.push(tag.to_string());
    }
}

/// 工具注册表
pub struct ToolRegistry {
    /// 工具表
    tools: RwLock<HashMap<String, ToolDef>>,
    /// 名称索引
    name_index: RwLock<HashMap<String, String>>, // name -> id
    /// 分类索引
    category_index: RwLock<HashMap<String, Vec<String>>>,
    /// 总调用次数
    total_calls: std::sync::atomic::AtomicU64,
}

impl ToolRegistry {
    /// 创建工具注册表（内置默认工具）
    pub fn new() -> Self {
        let registry = Self {
            tools: RwLock::new(HashMap::new()),
            name_index: RwLock::new(HashMap::new()),
            category_index: RwLock::new(HashMap::new()),
            total_calls: std::sync::atomic::AtomicU64::new(0),
        };
        registry.register_default_tools();
        registry
    }

    /// 注册默认工具
    fn register_default_tools(&self) {
        // 图谱查询工具
        let mut graph_query = ToolDef::new(
            "graph_query",
            "执行知识图谱查询",
            "knowledge_graph",
        );
        graph_query.add_param("query", "string", true);
        graph_query.add_param("limit", "integer", false);
        graph_query.add_tag("图谱");
        graph_query.add_tag("查询");
        self.register(graph_query).unwrap();

        // 知识库搜索工具
        let mut kb_search = ToolDef::new(
            "knowledge_search",
            "在知识库中搜索文档",
            "knowledge_base",
        );
        kb_search.add_param("keyword", "string", true);
        kb_search.add_param("top_k", "integer", false);
        kb_search.add_tag("搜索");
        kb_search.add_tag("文档");
        self.register(kb_search).unwrap();

        // 数据查询工具
        let mut data_query = ToolDef::new(
            "data_query",
            "执行数据查询和统计",
            "data",
        );
        data_query.add_param("sql", "string", true);
        data_query.add_tag("数据");
        data_query.add_tag("查询");
        self.register(data_query).unwrap();

        // 算法执行工具
        let mut algo_run = ToolDef::new(
            "algorithm_run",
            "执行指定的算法",
            "algorithm",
        );
        algo_run.add_param("algorithm_id", "string", true);
        algo_run.add_param("params", "object", false);
        algo_run.add_tag("算法");
        self.register(algo_run).unwrap();

        // 流程启动工具
        let mut workflow_start = ToolDef::new(
            "workflow_start",
            "启动业务流程",
            "workflow",
        );
        workflow_start.add_param("process_id", "string", true);
        workflow_start.add_param("variables", "object", false);
        workflow_start.add_tag("流程");
        self.register(workflow_start).unwrap();

        // 文件操作工具
        let mut file_op = ToolDef::new(
            "file_operation",
            "执行文件操作（上传/下载/列表等）",
            "storage",
        );
        file_op.add_param("operation", "string", true);
        file_op.add_param("path", "string", true);
        file_op.add_tag("文件");
        file_op.add_tag("云盘");
        self.register(file_op).unwrap();

        // 网页搜索工具
        let mut web_search = ToolDef::new(
            "web_search",
            "在互联网上搜索信息",
            "web",
        );
        web_search.add_param("query", "string", true);
        web_search.add_param("num_results", "integer", false);
        web_search.add_tag("搜索");
        web_search.add_tag("互联网");
        self.register(web_search).unwrap();
    }

    /// 注册工具
    pub fn register(&self, tool: ToolDef) -> AiResult<ToolDef> {
        if self.name_index.read().contains_key(&tool.name) {
            return Err(AiError::AlreadyExists(format!(
                "tool '{}' already exists",
                tool.name
            )));
        }

        self.name_index
            .write()
            .insert(tool.name.clone(), tool.id.clone());
        self.category_index
            .write()
            .entry(tool.category.clone())
            .or_default()
            .push(tool.id.clone());
        self.tools
            .write()
            .insert(tool.id.clone(), tool.clone());

        Ok(tool)
    }

    /// 按名称获取工具
    pub fn get_by_name(&self, name: &str) -> Option<ToolDef> {
        let id = self.name_index.read().get(name)?.clone();
        self.tools.read().get(&id).cloned()
    }

    /// 按 ID 获取工具
    pub fn get_by_id(&self, id: &str) -> Option<ToolDef> {
        self.tools.read().get(id).cloned()
    }

    /// 按分类获取工具
    pub fn get_by_category(&self, category: &str) -> Vec<ToolDef> {
        let ids = self
            .category_index
            .read()
            .get(category)
            .cloned()
            .unwrap_or_default();
        let tools = self.tools.read();
        ids.iter()
            .filter_map(|id| tools.get(id).cloned())
            .filter(|t| t.enabled)
            .collect()
    }

    /// 搜索工具
    pub fn search(&self, keyword: &str) -> Vec<ToolDef> {
        let keyword = keyword.to_lowercase();
        let tools = self.tools.read();
        tools
            .values()
            .filter(|t| {
                t.enabled
                    && (t.name.to_lowercase().contains(&keyword)
                        || t.description.to_lowercase().contains(&keyword)
                        || t.tags
                            .iter()
                            .any(|tag| tag.to_lowercase().contains(&keyword)))
            })
            .cloned()
            .collect()
    }

    /// 执行工具（模拟执行，返回示例结果）
    pub fn execute(&self, tool_name: &str, params: &HashMap<String, serde_json::Value>) -> AiResult<ToolResult> {
        let tool = self
            .get_by_name(tool_name)
            .ok_or_else(|| AiError::NotFound(format!("tool '{}' not found", tool_name)))?;

        if !tool.enabled {
            return Err(AiError::ToolError(format!(
                "tool '{}' is disabled",
                tool_name
            )));
        }

        // 验证必填参数
        for param in &tool.params {
            if param.required && !params.contains_key(&param.name) {
                return Err(AiError::InvalidInput(format!(
                    "missing required parameter: {}",
                    param.name
                )));
            }
        }

        // 更新调用计数
        self.total_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Some(t) = self.tools.write().get_mut(&tool.id) {
            t.call_count += 1;
        }

        // 返回模拟结果
        let result = ToolResult {
            success: true,
            data: Some(serde_json::json!({
                "tool": tool_name,
                "params_received": params.keys().len(),
                "status": "executed",
            })),
            error: None,
            duration_ms: 100,
        };

        Ok(result)
    }

    /// 列出所有工具
    pub fn list_all(&self) -> Vec<ToolDef> {
        self.tools.read().values().cloned().collect()
    }

    /// 工具总数
    pub fn count(&self) -> usize {
        self.tools.read().len()
    }

    /// 总调用次数
    pub fn total_calls(&self) -> u64 {
        self.total_calls.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_default_tools() {
        let registry = ToolRegistry::new();
        assert!(registry.count() >= 5);
    }

    #[test]
    fn test_get_by_name() {
        let registry = ToolRegistry::new();
        let tool = registry.get_by_name("graph_query").unwrap();
        assert_eq!(tool.name, "graph_query");
        assert!(!tool.params.is_empty());
    }

    #[test]
    fn test_get_by_category() {
        let registry = ToolRegistry::new();
        let kg_tools = registry.get_by_category("knowledge_graph");
        assert_eq!(kg_tools.len(), 1);
        assert_eq!(kg_tools[0].name, "graph_query");
    }

    #[test]
    fn test_search() {
        let registry = ToolRegistry::new();
        let results = registry.search("搜索");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_execute_tool() {
        let registry = ToolRegistry::new();

        let mut params = HashMap::new();
        params.insert("query".to_string(), json!("MATCH (n) RETURN n LIMIT 10"));

        let result = registry.execute("graph_query", &params).unwrap();
        assert!(result.success);
        assert!(result.data.is_some());
        assert_eq!(registry.total_calls(), 1);
    }

    #[test]
    fn test_execute_missing_param() {
        let registry = ToolRegistry::new();
        let params = HashMap::new();
        let result = registry.execute("graph_query", &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_unknown_tool() {
        let registry = ToolRegistry::new();
        let params = HashMap::new();
        let result = registry.execute("nonexistent", &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_register() {
        let registry = ToolRegistry::new();
        let tool = ToolDef::new("graph_query", "duplicate", "test");
        assert!(registry.register(tool).is_err());
    }

    #[test]
    fn test_tool_call_count() {
        let registry = ToolRegistry::new();
        let tool = registry.get_by_name("graph_query").unwrap();
        assert_eq!(tool.call_count, 0);

        let mut params = HashMap::new();
        params.insert("query".to_string(), json!("test"));
        registry.execute("graph_query", &params).unwrap();

        let tool = registry.get_by_name("graph_query").unwrap();
        assert_eq!(tool.call_count, 1);
    }
}
